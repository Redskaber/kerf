//! 自举 Reader（B3：Reader kerf 重写）——kerf 源码 Reader 的加载与值桥。
//!
//! 架构（07-bootstrap §3.2 混合期构成）：
//! - **Reader 逻辑**：`bootstrap/reader.krf`（kerf 源码，经种子 Rust Reader
//!   编译为字节码后在 Stage 0 VM 上运行——`lex-src`/`parse-tokz` 两入口，
//!   与 kerf-reader 的 `lex_source`/`parse_tokens` 接口形状对齐，§11）；
//! - **桥（本模块）**：宿主侧调用（`call_closure`）+ 值树 ↔ `Token`/`Stx`
//!   转换 + 数字文本同源 parse（i64/f64 语义转换是宿主类型边界——
//!   正确舍入的十进制→二进制转换不可在语言算术中可靠重实现）；
//! - **种子（kerf-reader）**：编译 reader.krf 的引导实现 + parity 测试的
//!   oracle（自举种子的经典角色——两实现互为印证，字节级等价为门）。
//!
//! 错误次序契约：数字溢出经 `str-int-valid?` 原语在 VM 词法扫描序内前置
//! 校验；桥侧 parse 仅作同源复核——维持与种子一致的首错位置。
//!
//! 堆契约：Reader 程序的持久堆承载全局数据（KEYWORDS 等序对树）与各次
//! 调用中间产物——GC 以（栈+帧+全局）为根集，跨调用回收安全；两阶段
//! （lex → parse）共用同一堆（parse 消费 lex 产出的原 Token 值）。
//!
//! 并发契约：thread_local 状态（Rc 值非 Sync）——每线程惰性编译加载一次。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kerf_compiler::BcProgram;
use kerf_reader::{operator_of, Delimiter, ReadError, Token, TokenKind};
use kerf_runtime::{BoxedInput, Heap};
use kerf_span::{render_diagnostic, Diagnostic, DiagnosticCode, FileId, Severity, SourceMap, Span};
use kerf_syntax::{Keyword, Phase, ScopeSet, Stx, StxDatum, StxLiteral, Symbol, SymbolTable};
use kerf_vm::{box_value, call_closure, render_value, run_program, unbox_slot, Value, VmError};

use crate::builtins::register_globals;

/// Reader 源码（编译期嵌入——产物自包含，不依赖外部文件）。
const READER_SRC: &str = include_str!("bootstrap/reader.krf");
/// Reader 源文件名（诊断渲染用）。
const READER_FILENAME: &str = "reader.krf";

/// 自举 Reader 状态（每线程一份，惰性初始化）。
struct BootstrapState {
    /// Reader 程序字节码（种子管线编译）。
    program: BcProgram,
    /// 已加载全局（内置 + reader.krf 顶层定义）。
    globals: HashMap<Symbol, Value>,
    /// Reader 自身符号表（入口/自检函数名解析）。
    table: SymbolTable,
    /// `lex-src` 入口符号（Reader 自身符号表口径）。
    lex_sym: Symbol,
    /// `parse-tokz` 入口符号。
    parse_sym: Symbol,
    /// Reader 的源映射（内部错误诊断渲染）。
    source_map: SourceMap,
    /// 持久堆：全局数据 + 各次调用产物（GC 根集含 globals）。
    heap: Heap,
}

thread_local! {
    static BOOTSTRAP: RefCell<Option<BootstrapState>> = const {
        RefCell::new(None)
    };
}

/// 自举读取入口：源文本 → `Vec<Stx>`（与 `kerf_reader::read_source` 同形）。
///
/// 生产管线（`compile_front`）与 `dump_stx` 经此入口消费 kerf Reader。
pub fn read_source(
    source: &str,
    file_id: FileId,
    table: &mut SymbolTable,
) -> Result<Vec<Stx>, ReadError> {
    let blen = source.len() as i64;
    with_bootstrap(|st| {
        // 阶段 1：词法（VM）+ Token 桥（数字在扫描序内验证——错误次序 parity）
        let lex_value = call_entry(
            st,
            st.lex_sym,
            "lex-src",
            vec![Value::Str(Rc::from(source)), Value::Int(blen)],
        )?;
        let _tokens = bridge_tokens(&lex_value, &st.heap, file_id, table)?;
        // 阶段 2：语法（VM——消费同一堆上的原 Token 值）+ Stx 桥
        let parse_value = call_entry(st, st.parse_sym, "parse-tokz", vec![lex_value])?;
        bridge_stx_list(&parse_value, &st.heap, file_id, table)
    })
}

/// 自举词法入口：源文本 → `Vec<Token>`（与 `kerf_reader::lex_source` 同形）。
///
/// 消费方：`dump_tokens`（CLI `tokens` 子命令——§14.9 编译器自调试）。
pub fn lex_source(
    source: &str,
    file_id: FileId,
    table: &mut SymbolTable,
) -> Result<Vec<Token>, ReadError> {
    let blen = source.len() as i64;
    with_bootstrap(|st| {
        let lex_value = call_entry(
            st,
            st.lex_sym,
            "lex-src",
            vec![Value::Str(Rc::from(source)), Value::Int(blen)],
        )?;
        bridge_tokens(&lex_value, &st.heap, file_id, table)
    })
}

/// 以线程局部状态执行（惰性初始化）。
fn with_bootstrap<T>(
    f: impl FnOnce(&mut BootstrapState) -> Result<T, ReadError>,
) -> Result<T, ReadError> {
    BOOTSTRAP.with(|cell| {
        let mut opt = cell.borrow_mut();
        if opt.is_none() {
            match load_bootstrap() {
                Ok(state) => *opt = Some(state),
                Err(e) => return Err(e),
            }
        }
        f(opt.as_mut().expect("上方已初始化"))
    })
}

/// 编译并加载 Reader 程序（种子路径——Rust Reader 编译 reader.krf，
/// 不经自举路径：种子编译自举 Reader，无递归）。
fn load_bootstrap() -> Result<BootstrapState, ReadError> {
    let front = crate::driver::compile_front_seed(READER_SRC, READER_FILENAME).map_err(|e| {
        ReadError::new(
            format!(
                "[bootstrap] 自举 Reader 编译失败（种子管线）：{}",
                e.rendered
            ),
            Span::dummy(),
        )
    })?;
    build_state(front.program, front.table, front.source_map)
}

/// 由前段产物构建 Reader 状态（种子加载与 [`install_state`] 共享——
/// 单一状态构造实现：注册内置 + 卫生回退 + 顶层执行 + 入口 intern）。
fn build_state(
    program: BcProgram,
    mut table: SymbolTable,
    source_map: SourceMap,
) -> Result<BootstrapState, ReadError> {
    // reader.krf 无 I/O 引用（纯词法数据变换）——空授权（R9 已验证
    // 无门控引用；fail-closed：I/O 内置不进自举环境）
    let mut globals = register_globals(&mut table, &crate::capability::IoGrant::none());
    // 卫生回退解析（reader.krf 无宏——空操作；保持与 run_source 管线一致性）
    crate::builtins::resolve_hygiene_fallbacks(&program, &mut table, &mut globals);
    let mut heap = Heap::new();
    let outcome = run_program(&program, &mut globals, &mut heap)
        .map_err(|e| vm_error_to_read(&e, &source_map))?;
    let _ = outcome; // 主原型仅执行顶层定义（值无意义）
    let lex_sym = table.intern("lex-src");
    let parse_sym = table.intern("parse-tokz");
    Ok(BootstrapState {
        program,
        globals,
        table,
        lex_sym,
        parse_sym,
        source_map,
        heap,
    })
}

/// 安装给定产物为当前线程的 Reader 状态（门 B fixpoint——B₂ 轮的
/// 「以 B₁ 为新 bootstrap 程序」语义：自举链产物替换种子加载状态；
/// i1-design §5 门 B + §7.4 隔离运行纪律）。
///
/// 产物须为 reader.krf 源编译所得（入口 `lex-src`/`parse-tokz`
/// 存在性由状态构造实测）。
pub fn install_state(
    program: BcProgram,
    table: SymbolTable,
    source_map: SourceMap,
) -> Result<(), ReadError> {
    let state = build_state(program, table, source_map)?;
    BOOTSTRAP.with(|cell| *cell.borrow_mut() = Some(state));
    Ok(())
}

/// 卸载当前线程 Reader 状态（fixpoint 隔离运行——§7.4 fresh 形态；
/// 后续调用触发惰性种子重载）。
pub fn reset_state() {
    BOOTSTRAP.with(|cell| *cell.borrow_mut() = None);
}

/// 调用 Reader 入口函数（宿主信任层程序化调用——§11 与 run_program 同级）。
///
/// 双形态：kerf 闭包走 [`call_closure`]；内置函数直接调用（自检面消费
/// str->pos-chars 等原语——同信任层，无 VM re-entry）。
fn call_entry(
    st: &mut BootstrapState,
    entry: Symbol,
    entry_name: &str,
    args: Vec<Value>,
) -> Result<Value, ReadError> {
    let callee = st.globals.get(&entry).cloned().ok_or_else(|| {
        ReadError::new(
            format!(
                "[bootstrap] 自举 Reader 内部错误：入口 {} 未定义（reader.krf 损坏）",
                entry_name
            ),
            Span::dummy(),
        )
    })?;
    match &callee {
        Value::Builtin(b) => b.call(&mut st.heap, args).map_err(|e| {
            ReadError::new(
                format!("[bootstrap] 自检调用失败（{}）：{}", entry_name, e.message),
                Span::dummy(),
            )
        }),
        other => call_closure(&st.program, &mut st.globals, &mut st.heap, other, args)
            .map_err(|e| vm_error_to_read(&e, &st.source_map)),
    }
}

/// VM 错误 → ReadError（防御路径：词法/语法错误经 ('err ...) 值返回，
/// 不经此通道——此路径仅覆盖 reader.krf 内部缺陷或帧上限）。
fn vm_error_to_read(e: &VmError, sm: &SourceMap) -> ReadError {
    let mut diag = Diagnostic::error(
        Some(DiagnosticCode(1)),
        format!("自举 Reader 内部错误：{}", e.message),
        e.span,
    );
    for t in &e.trace {
        diag = diag.with_child(Severity::Note, "调用点", t.span);
    }
    // 以 reader.krf 自身源映射渲染（用户源映射对 reader.krf Span 无意义）
    ReadError::new(render_diagnostic(&diag, sm), Span::dummy())
}

// ---- 自检面（§14.9 编译器自调试族——测试与宿主消费方）----

/// 调用 Reader 程序顶层定义的全局函数（与 call_entry 同信任层）。
///
/// 消费方：parity/高阶函数直测（tests/v0/stage1/plan/bootstrap_reader_
/// tests.rs）。列表实参经 [`selfcheck_list`] 构造（`Value::Pair` 需堆
/// 分配）；结果渲染经 [`selfcheck_render`]。
pub fn selfcheck_call(name: &str, args: Vec<Value>) -> Result<Value, ReadError> {
    with_bootstrap(|st| {
        let sym = st.table.intern(name);
        call_entry(st, sym, name, args)
    })
}

/// 在 Reader 持久堆上构造列表值（自检实参构造——与 `list` 内置同构）。
pub fn selfcheck_list(items: Vec<Value>) -> Result<Value, ReadError> {
    with_bootstrap(|st| {
        if items.is_empty() {
            return Ok(Value::Nil);
        }
        let mut acc = st.heap.alloc_boxed(BoxedInput::Nil);
        for v in items.iter().rev() {
            let elem = box_value(v, &mut st.heap);
            acc = st.heap.alloc_pair(elem, acc);
        }
        Ok(Value::Pair(acc))
    })
}

/// 渲染自检结果值（序对需堆走查——与 `print` 同一渲染口径）。
pub fn selfcheck_render(v: &Value) -> Result<String, ReadError> {
    with_bootstrap(|st| Ok(render_value(v, &st.heap)))
}

// ---- 值桥：Token ----

/// Token 值列表 → `Vec<Token>`（含 ('err ...) 检查与数字同源 parse）。
fn bridge_tokens(
    v: &Value,
    heap: &Heap,
    file_id: FileId,
    table: &mut SymbolTable,
) -> Result<Vec<Token>, ReadError> {
    if let Some(e) = as_err_form(v, heap, file_id) {
        return Err(e);
    }
    let mut out = Vec::new();
    let mut cur = v.clone();
    while let Value::Pair(r) = cur {
        let (tok_ref, rest) = heap.get_pair(r).ok_or_else(internal_heap_error)?;
        let tok_val = unbox_slot(tok_ref, heap);
        out.push(convert_token(&tok_val, heap, file_id, table)?);
        cur = unbox_slot(rest, heap);
    }
    Ok(out)
}

/// 单 Token 值（(kind start end payload)）→ `Token`。
///
/// 数字文本在此同源 parse（`text.parse::<i64>()` / `parse::<f64>()`）——
/// 与种子词法器同一 Rust 语义，消息与 Span 逐字节一致；此转换发生在
/// Token 序（= 扫描序）内，维持首错位置 parity。
fn convert_token(
    tok: &Value,
    heap: &Heap,
    file_id: FileId,
    table: &mut SymbolTable,
) -> Result<Token, ReadError> {
    let fields = value_list_fields(tok, heap)?;
    if fields.len() != 4 {
        return Err(internal(&format!(
            "Token 字段数异常：期望 4 实际 {}",
            fields.len()
        )));
    }
    let kind = as_int(&fields[0])?;
    let start = as_int(&fields[1])?;
    let end = as_int(&fields[2])?;
    let payload = fields[3].clone();
    let span = Span::new(file_id, start as u32, end as u32);
    let kind_enum = match kind {
        0 => TokenKind::Eof,
        1 => {
            let text = as_str(&payload)?;
            TokenKind::IntLiteral(
                text.parse::<i64>()
                    .map_err(|_| ReadError::new(format!("整数超出范围 '{}'", text), span))?,
            )
        }
        2 => {
            let text = as_str(&payload)?;
            TokenKind::FloatLiteral(
                text.parse::<f64>()
                    .map_err(|_| ReadError::new(format!("非法浮点字面量 '{}'", text), span))?,
            )
        }
        3 => TokenKind::StringLiteral(Rc::from(as_str(&payload)?)),
        4 => TokenKind::BoolLiteral(true),
        5 => TokenKind::BoolLiteral(false),
        6 => TokenKind::NilLiteral,
        7 => TokenKind::Identifier(table.intern(as_str(&payload)?)),
        8 => {
            let name = as_str(&payload)?;
            TokenKind::Keyword(Keyword::from_name(name).ok_or_else(|| {
                ReadError::new(format!("[bootstrap] 未知关键字 '{}'", name), span)
            })?)
        }
        9 => {
            let name = as_str(&payload)?;
            let op = operator_of(name).ok_or_else(|| {
                ReadError::new(format!("[bootstrap] 未知运算符 '{}'", name), span)
            })?;
            TokenKind::Operator(op, table.intern(name))
        }
        10 => TokenKind::QuoteShorthand,
        11 => TokenKind::Delimiter(Delimiter::OpenParen),
        12 => TokenKind::Delimiter(Delimiter::CloseParen),
        13 => TokenKind::Delimiter(Delimiter::OpenBracket),
        14 => TokenKind::Delimiter(Delimiter::CloseBracket),
        _ => {
            return Err(ReadError::new(
                format!("[bootstrap] 未知 Token 种类 {}", kind),
                span,
            ))
        }
    };
    Ok(Token {
        kind: kind_enum,
        span,
        scopes: ScopeSet::default(),
    })
}

// ---- 值桥：Stx ----

/// datum 节点值列表 → `Vec<Stx>`（含 ('err ...) 检查）。
fn bridge_stx_list(
    v: &Value,
    heap: &Heap,
    file_id: FileId,
    table: &mut SymbolTable,
) -> Result<Vec<Stx>, ReadError> {
    if let Some(e) = as_err_form(v, heap, file_id) {
        return Err(e);
    }
    let mut out = Vec::new();
    let mut cur = v.clone();
    while let Value::Pair(r) = cur {
        let (node_ref, rest) = heap.get_pair(r).ok_or_else(internal_heap_error)?;
        let node_val = unbox_slot(node_ref, heap);
        out.push(convert_node(&node_val, heap, file_id, table)?);
        cur = unbox_slot(rest, heap);
    }
    Ok(out)
}

/// 单 datum 节点 → `Stx`（符号经 `table.intern` 内部化——NFC 归一化与
/// 种子同一入口；关键字名与同名标识符内部化为同一 Symbol，口径一致）。
fn convert_node(
    node: &Value,
    heap: &Heap,
    file_id: FileId,
    table: &mut SymbolTable,
) -> Result<Stx, ReadError> {
    let fields = value_list_fields(node, heap)?;
    if fields.len() < 3 {
        return Err(internal(&format!(
            "datum 节点字段数异常：期望 >=3 实际 {}",
            fields.len()
        )));
    }
    let tag = as_symbol_name(&fields[0])?;
    let start = as_int(&fields[1])?;
    let end = as_int(&fields[2])?;
    let span = Span::new(file_id, start as u32, end as u32);
    let scopes = ScopeSet::default();
    match tag {
        "int" => {
            let text = node_text(&fields, 3)?;
            let v = text.parse::<i64>().map_err(|_| {
                ReadError::new(
                    format!("[bootstrap] 整数复核失败（词法已验证）：'{}'", text),
                    span,
                )
            })?;
            Ok(Stx::literal(StxLiteral::Int(v), span, scopes))
        }
        "float" => {
            let text = node_text(&fields, 3)?;
            let v = text.parse::<f64>().map_err(|_| {
                ReadError::new(
                    format!("[bootstrap] 浮点复核失败（词法已验证）：'{}'", text),
                    span,
                )
            })?;
            Ok(Stx::literal(StxLiteral::Float(v), span, scopes))
        }
        "str" => {
            let content = node_text(&fields, 3)?;
            Ok(Stx::literal(
                StxLiteral::Str(Rc::from(content)),
                span,
                scopes,
            ))
        }
        "true" => Ok(Stx::literal(StxLiteral::Bool(true), span, scopes)),
        "false" => Ok(Stx::literal(StxLiteral::Bool(false), span, scopes)),
        "nil" => Ok(Stx::literal(StxLiteral::Nil, span, scopes)),
        "sym" => {
            let name = node_text(&fields, 3)?;
            let sym = table.intern(name);
            Ok(Stx::symbol(sym, span, scopes))
        }
        "list" => {
            let items = convert_node_items(&fields[3..], heap, file_id, table)?;
            Ok(Stx::list(items, span, scopes))
        }
        "vec" => {
            let items = convert_node_items(&fields[3..], heap, file_id, table)?;
            Ok(Stx {
                datum: StxDatum::Vector(std::rc::Rc::new(items)),
                span,
                scopes,
                phase: Phase::Runtime,
                uniform_tag: None,
            })
        }
        other => Err(ReadError::new(
            format!("[bootstrap] 未知 datum 节点种类 '{}'", other),
            span,
        )),
    }
}

/// 节点子项（fields[3..]，可能嵌套）→ `Vec<Stx>`。
fn convert_node_items(
    items: &[Value],
    heap: &Heap,
    file_id: FileId,
    table: &mut SymbolTable,
) -> Result<Vec<Stx>, ReadError> {
    items
        .iter()
        .map(|item| convert_node(item, heap, file_id, table))
        .collect()
}

// ---- 值访问辅助 ----

/// ('err 消息 起 止) → ReadError（非错误形态返回 None）。
fn as_err_form(v: &Value, heap: &Heap, file_id: FileId) -> Option<ReadError> {
    if let Value::Pair(r) = v {
        if let Some((car, _)) = heap.get_pair(*r) {
            if let Value::Symbol(s) = unbox_slot(car, heap) {
                if &*s == "err" {
                    let fields = value_list_fields(v, heap).ok()?;
                    if fields.len() != 4 {
                        return None;
                    }
                    let msg = as_str(&fields[1]).ok()?;
                    let s = as_int(&fields[2]).ok()?;
                    let e = as_int(&fields[3]).ok()?;
                    // Span 挂用户 file_id（错误位置在用户源文本内）
                    return Some(ReadError::new(
                        msg.to_string(),
                        Span::new(file_id, s as u32, e as u32),
                    ));
                }
            }
        }
    }
    None
}

/// 值列表 → 字段 `Vec<Value>`（走查堆序对脊）。
pub(crate) fn value_list_fields(v: &Value, heap: &Heap) -> Result<Vec<Value>, ReadError> {
    let mut out = Vec::new();
    let mut cur = v.clone();
    while let Value::Pair(r) = cur {
        let (car, cdr) = heap.get_pair(r).ok_or_else(internal_heap_error)?;
        out.push(unbox_slot(car, heap));
        cur = unbox_slot(cdr, heap);
    }
    Ok(out)
}

pub(crate) fn as_int(v: &Value) -> Result<i64, ReadError> {
    v.as_int()
        .ok_or_else(|| internal(&format!("期望 int 字段，实际 {}", v.type_name())))
}

pub(crate) fn as_str(v: &Value) -> Result<&str, ReadError> {
    match v {
        Value::Str(s) => Ok(&**s),
        other => Err(internal(&format!(
            "期望 str 字段，实际 {}",
            other.type_name()
        ))),
    }
}

pub(crate) fn as_symbol_name(v: &Value) -> Result<&str, ReadError> {
    match v {
        Value::Symbol(s) => Ok(&**s),
        other => Err(internal(&format!(
            "期望 symbol 字段，实际 {}",
            other.type_name()
        ))),
    }
}

/// 节点文本字段（fields[idx]，str）。
fn node_text(fields: &[Value], idx: usize) -> Result<&str, ReadError> {
    fields
        .get(idx)
        .ok_or_else(|| internal("datum 节点缺少文本字段"))
        .and_then(as_str)
}

pub(crate) fn internal_heap_error() -> ReadError {
    internal("堆序对读取失败")
}

pub(crate) fn internal(msg: &str) -> ReadError {
    ReadError::new(
        format!("[bootstrap] 自举 Reader 内部错误：{}", msg),
        Span::dummy(),
    )
}
