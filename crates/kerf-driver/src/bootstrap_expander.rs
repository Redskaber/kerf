//! 自举 Expander（E1-β：Expander kerf 重写生产切换——含宏收口）。
//!
//! 架构（07-bootstrap §3.2 混合期构成；r6 自举 Reader 同型三件套）：
//! - **展开逻辑**：`bootstrap/expander.krf`（kerf 源码，经种子管线编译
//!   为字节码后在 Stage 0 VM 上运行——`lexp-expand-program` 入口，
//!   与 kerf-expander 的 `expand_program` 接口形状对齐，§11）；
//! - **桥（本模块）**：宿主侧调用（`call_closure`）+ 值树双向转换
//!   （`Stx` → VM datum 节点含作用域集与展开代次 → VM core 节点 →
//!   `CoreExpr` 作用域集/代次重建）；
//! - **种子（kerf-expander）**：自举引导（expander.krf 的编译）+ parity
//!   测试的 oracle（自举种子经典角色——两实现互为印证）。
//!
//! **E1-β 生产切换**：`compile_front`（生产管线）的展开段经本模块——
//! 读 + 展开两段均自举（07 §3.3「语言能表达自身前端」的完整生产命题）。
//! 切换守护：driver `production_expander_is_bootstrap`（宏产物
//! CoreExpr Span 展开代次判别——本桥产出保留源节点代次）。
//!
//! 堆契约：Expander 程序的持久堆承载全局数据（KEYWORDS/SCOPE-NEXT/
//! TRANSFORMERS 等）与各次调用中间产物——GC 以（栈+帧+全局）为根集，
//! 跨调用回收安全；输入节点树在持久堆上构造（一次性，调用后可回收）。
//!
//! 并发契约：thread_local 状态（Rc 值非 Sync）——每线程惰性编译加载一次。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kerf_compiler::BcProgram;
use kerf_core::{Capability, CoreExpr, LiteralValue};
use kerf_expander::ExpandError;
use kerf_runtime::{BoxedInput, Heap};
use kerf_span::{render_diagnostic, Diagnostic, DiagnosticCode, FileId, Severity, SourceMap, Span};
use kerf_syntax::{ScopeSet, Stx, StxDatum, StxLiteral, Symbol, SymbolTable};
use kerf_vm::{box_value, call_closure, run_program, unbox_slot, Value, VmError};

use crate::bootstrap::{as_int, as_str, as_symbol_name, value_list_fields};
use crate::builtins::register_globals;

/// Expander 源码（编译期嵌入——产物自包含，不依赖外部文件）。
const EXPANDER_SRC: &str = include_str!("bootstrap/expander.krf");
/// Expander 源文件名（诊断渲染用）。
const EXPANDER_FILENAME: &str = "expander.krf";
/// 入口名（expander.krf 顶层定义）。
const ENTRY_NAME: &str = "lexp-expand-program";

/// 自举 Expander 状态（每线程一份，惰性初始化）。
struct BootstrapExpanderState {
    /// Expander 程序字节码（种子管线编译）。
    program: BcProgram,
    /// 已加载全局（内置 + expander.krf 顶层定义）。
    globals: HashMap<Symbol, Value>,
    /// `lexp-expand-program` 入口符号。
    entry_sym: Symbol,
    /// Expander 的源映射（内部错误诊断渲染）。
    source_map: SourceMap,
    /// 持久堆：全局数据 + 各次调用产物（GC 根集含 globals）。
    heap: Heap,
}

thread_local! {
    static BOOTSTRAP_EXP: RefCell<Option<BootstrapExpanderState>> = const {
        RefCell::new(None)
    };
}

/// 自举展开入口：`Vec<Stx>` → `Vec<Rc<CoreExpr>>`（与
/// `kerf_expander::expand_program` 同形——parity oracle 对照面）。
///
/// **E1-β 生产路径**：`compile_front` 消费（含宏展开）；种子路径
/// [`crate::driver::compile_front_seed`]（bootstrap 加载与 parity
/// oracle）仍走 Rust `kerf_expander::expand_program`。
pub fn expand_program(
    forms: &[Stx],
    file_id: FileId,
    table: &mut SymbolTable,
) -> Result<Vec<Rc<CoreExpr>>, ExpandError> {
    with_expander(|st| {
        // 阶段 1：Stx → VM datum 节点（含作用域集 int 列表；符号经
        // 用户表取名——节点名字以 str 携带，正向桥回 intern）
        let nodes: Vec<Value> = forms
            .iter()
            .map(|f| stx_to_node(f, table, &mut st.heap))
            .collect();
        let input = heap_list(&mut st.heap, nodes);
        // 阶段 2：入口调用（VM 上运行 expander.krf）
        let output = call_entry(st, input)?;
        // 阶段 3：错误检查 + core 节点桥（CoreExpr 重建）
        if let Some(e) = as_expand_err(&output, &st.heap, file_id) {
            return Err(e);
        }
        let mut out = Vec::new();
        let mut cur = output;
        while let Value::Pair(r) = cur {
            let (node_ref, rest) = st.heap.get_pair(r).ok_or_else(internal_heap_x)?;
            let node_val = unbox_slot(node_ref, &st.heap);
            out.push(core_from_value(&node_val, &st.heap, file_id, table)?);
            cur = unbox_slot(rest, &st.heap);
        }
        Ok(out)
    })
}

/// 自举 Expander 状态活性探针（E1-β 生产切换守护：生产编译路径
/// 是否真的经 VM Expander——`loaded` 后 `compile_front` 产出的任何
/// 程序都应使本探针为真）。
#[cfg(test)]
pub(crate) fn is_loaded() -> bool {
    BOOTSTRAP_EXP.with(|cell| cell.borrow().is_some())
}

/// 以线程局部状态执行（惰性初始化）。
fn with_expander<T>(
    f: impl FnOnce(&mut BootstrapExpanderState) -> Result<T, ExpandError>,
) -> Result<T, ExpandError> {
    BOOTSTRAP_EXP.with(|cell| {
        let mut opt = cell.borrow_mut();
        if opt.is_none() {
            match load_expander() {
                Ok(state) => *opt = Some(state),
                Err(e) => return Err(e),
            }
        }
        f(opt.as_mut().expect("上方已初始化"))
    })
}

/// 编译并加载 Expander 程序（种子路径——Rust Reader/Expander 编译
/// expander.krf，不经自举路径：种子编译自举 Expander，无递归）。
fn load_expander() -> Result<BootstrapExpanderState, ExpandError> {
    let front =
        crate::driver::compile_front_seed(EXPANDER_SRC, EXPANDER_FILENAME).map_err(|e| {
            expand_internal(&format!(
                "[bootstrap-exp] 自举 Expander 编译失败（种子管线）：{}",
                e.rendered
            ))
        })?;
    let mut table = front.table;
    // expander.krf 无 I/O 引用（纯数据变换）——空授权（R9 fail-closed）
    let mut globals = register_globals(&mut table, &crate::capability::IoGrant::none());
    let program = front.program;
    crate::builtins::resolve_hygiene_fallbacks(&program, &mut table, &mut globals);
    let mut heap = Heap::new();
    let outcome = run_program(&program, &mut globals, &mut heap)
        .map_err(|e| vm_error_to_expand(&e, &front.source_map))?;
    let _ = outcome; // 主原型仅执行顶层定义（值无意义）
    let entry_sym = table.intern(ENTRY_NAME);
    Ok(BootstrapExpanderState {
        program,
        globals,
        entry_sym,
        source_map: front.source_map,
        heap,
    })
}

/// 调用 Expander 入口（宿主信任层程序化调用——§11 与 run_program 同级）。
fn call_entry(st: &mut BootstrapExpanderState, input: Value) -> Result<Value, ExpandError> {
    let callee = st.globals.get(&st.entry_sym).cloned().ok_or_else(|| {
        expand_internal(&format!(
            "[bootstrap-exp] 入口 {} 未定义（expander.krf 损坏）",
            ENTRY_NAME
        ))
    })?;
    call_closure(
        &st.program,
        &mut st.globals,
        &mut st.heap,
        &callee,
        vec![input],
    )
    .map_err(|e| vm_error_to_expand(&e, &st.source_map))
}

/// VM 错误 → ExpandError（防御路径：展开错误经 ('err ...) 值返回，
/// 不经此通道——此路径仅覆盖 expander.krf 内部缺陷或帧上限）。
fn vm_error_to_expand(e: &VmError, sm: &SourceMap) -> ExpandError {
    let mut diag = Diagnostic::error(
        Some(DiagnosticCode(2)),
        format!("自举 Expander 内部错误：{}", e.message),
        e.span,
    );
    for t in &e.trace {
        diag = diag.with_child(Severity::Note, "调用点", t.span);
    }
    ExpandError {
        message: render_diagnostic(&diag, sm),
        span: Span::dummy(),
    }
}

// ---- 反向值桥：Stx → VM datum 节点 ----

/// `Stx` → datum 节点值 `(tag s e exp scopes ...)`（符号名以 str 携带；
/// 作用域集 = int 列表——正向桥按 ScopeSet 排序去重语义重建；exp =
/// 展开代次（Span.expansion_id——E1-β 宏 parity：instantiate 的 Span
// 并集守卫依赖代次判等，桥侧输入恒源码代次 0）。
fn stx_to_node(stx: &Stx, table: &SymbolTable, heap: &mut Heap) -> Value {
    let s = Value::Int(stx.span.start as i64);
    let e = Value::Int(stx.span.end as i64);
    let exp = Value::Int(stx.span.expansion_id as i64);
    let scopes = scope_list_value(&stx.scopes, heap);
    let tag = |name: &str| Value::Symbol(Rc::from(name));
    let items: Vec<Value> = match &stx.datum {
        StxDatum::Symbol(sym) => vec![
            tag("sym"),
            s,
            e,
            exp,
            scopes,
            Value::Str(Rc::from(table.name(*sym))),
        ],
        StxDatum::Literal(l) => match l {
            StxLiteral::Int(v) => vec![tag("int"), s, e, exp, scopes, Value::Int(*v)],
            StxLiteral::Float(v) => {
                vec![tag("float"), s, e, exp, scopes, Value::Float(*v)]
            }
            StxLiteral::Str(v) => {
                vec![tag("str"), s, e, exp, scopes, Value::Str(v.clone())]
            }
            StxLiteral::Bool(b) => {
                vec![tag(if *b { "true" } else { "false" }), s, e, exp, scopes]
            }
            StxLiteral::Nil => vec![tag("nil"), s, e, exp, scopes],
        },
        StxDatum::List(children) => {
            let mut v = vec![tag("list"), s, e, exp, scopes];
            v.extend(children.iter().map(|c| stx_to_node(c, table, heap)));
            v
        }
        StxDatum::Vector(children) => {
            let mut v = vec![tag("vec"), s, e, exp, scopes];
            v.extend(children.iter().map(|c| stx_to_node(c, table, heap)));
            v
        }
    };
    heap_list(heap, items)
}

/// 作用域集 → int 列表值（升序迭代）。
fn scope_list_value(scopes: &ScopeSet, heap: &mut Heap) -> Value {
    let items: Vec<Value> = scopes.iter().map(|i| Value::Int(i as i64)).collect();
    heap_list(heap, items)
}

/// 在指定堆上构造列表值（与 `list` 内置同构——桥自检面同款）。
fn heap_list(heap: &mut Heap, items: Vec<Value>) -> Value {
    if items.is_empty() {
        return Value::Nil;
    }
    let mut acc = heap.alloc_boxed(BoxedInput::Nil);
    for v in items.into_iter().rev() {
        let elem = box_value(&v, heap);
        acc = heap.alloc_pair(elem, acc);
    }
    Value::Pair(acc)
}

// ---- 正向值桥：VM core 节点 → CoreExpr ----

/// ('err 消息 起 止) → ExpandError（非错误形态返回 None；Span 挂用户
/// file_id——错误位置在用户源文本内）。
fn as_expand_err(v: &Value, heap: &Heap, file_id: FileId) -> Option<ExpandError> {
    if let Value::Pair(r) = v {
        if let Some((car, _)) = heap.get_pair(*r) {
            if let Value::Symbol(s) = unbox_slot(car, heap) {
                if &*s == "err" {
                    let fields = value_list_fields(v, heap).ok()?;
                    if fields.len() != 4 {
                        return None;
                    }
                    let msg = as_str(&fields[1]).ok()?.to_string();
                    let s = as_int(&fields[2]).ok()?;
                    let e = as_int(&fields[3]).ok()?;
                    return Some(ExpandError {
                        message: msg,
                        span: Span::new(file_id, s as u32, e as u32),
                    });
                }
            }
        }
    }
    None
}

/// core 节点值 `(tag s e exp ...)` → `CoreExpr`（符号名经用户表 intern——
/// 与种子同一 intern 入口；作用域集经 ScopeSet 排序去重重建；exp =
/// 展开代次——E1-β 生产切换后保留（宏相位可观测性诊断「expansion N」
/// 标记的产出通道，§19.1）。
fn core_from_value(
    v: &Value,
    heap: &Heap,
    file_id: FileId,
    table: &mut SymbolTable,
) -> Result<Rc<CoreExpr>, ExpandError> {
    let fields = value_list_fields(v, heap).map_err(read_to_expand)?;
    if fields.len() < 4 {
        return Err(internal_x("core 节点字段数异常"));
    }
    let tag = as_symbol_name(&fields[0]).map_err(read_to_expand)?;
    let s = as_int(&fields[1]).map_err(read_to_expand)?;
    let e = as_int(&fields[2]).map_err(read_to_expand)?;
    let exp = as_int(&fields[3]).map_err(read_to_expand)?;
    let span = Span {
        file_id,
        start: s as u32,
        end: e as u32,
        expansion_id: exp as u32,
    };
    let rest = &fields[4..];
    match tag {
        "lit" => Ok(Rc::new(CoreExpr::Literal {
            value: literal_from_value(rest.first().ok_or_else(|| internal_x("lit 缺值"))?, heap)?,
            span,
        })),
        "var" => {
            let name = str_field(rest, 0)?;
            let scopes = scope_set_field(rest, 1, heap)?;
            Ok(Rc::new(CoreExpr::VarRef {
                name: table.intern(name),
                scopes,
                span,
            }))
        }
        "app" => {
            let fn_expr = core_from_value(field(rest, 0)?, heap, file_id, table)?;
            let mut args = Vec::with_capacity(rest.len().saturating_sub(1));
            for a in &rest[1..] {
                args.push(core_from_value(a, heap, file_id, table)?);
            }
            Ok(Rc::new(CoreExpr::App {
                fn_expr,
                args,
                span,
            }))
        }
        "if" => {
            if rest.len() != 3 {
                return Err(internal_x("if 节点字段数异常"));
            }
            Ok(Rc::new(CoreExpr::If {
                cond: core_from_value(&rest[0], heap, file_id, table)?,
                then_branch: core_from_value(&rest[1], heap, file_id, table)?,
                else_branch: core_from_value(&rest[2], heap, file_id, table)?,
                span,
            }))
        }
        "lambda" => {
            if rest.len() != 3 {
                return Err(internal_x("lambda 节点字段数异常"));
            }
            let name_fields = value_list_fields(&rest[0], heap).map_err(read_to_expand)?;
            let names: Vec<Symbol> = name_fields
                .iter()
                .map(|n| {
                    let name = as_str(n).map_err(read_to_expand)?;
                    Ok(table.intern(name))
                })
                .collect::<Result<Vec<Symbol>, ExpandError>>()?;
            let param_scopes: Vec<ScopeSet> = value_list_fields(&rest[1], heap)
                .map_err(read_to_expand)?
                .iter()
                .map(|sc| scope_set_from_value(sc, heap))
                .collect::<Result<Vec<_>, _>>()?;
            let body = core_from_value(&rest[2], heap, file_id, table)?;
            Ok(Rc::new(CoreExpr::Lambda {
                params: names,
                param_scopes,
                body,
                span,
            }))
        }
        "set" => {
            if rest.len() != 3 {
                return Err(internal_x("set 节点字段数异常"));
            }
            let name = table.intern(as_str(&rest[0]).map_err(read_to_expand)?);
            let scopes = scope_set_from_value(&rest[1], heap)?;
            let value = core_from_value(&rest[2], heap, file_id, table)?;
            Ok(Rc::new(CoreExpr::SetBang {
                name,
                scopes,
                value,
                span,
            }))
        }
        "define" => {
            if rest.len() != 2 {
                return Err(internal_x("define 节点字段数异常"));
            }
            let name = table.intern(as_str(&rest[0]).map_err(read_to_expand)?);
            let value = core_from_value(&rest[1], heap, file_id, table)?;
            Ok(Rc::new(CoreExpr::Define { name, value, span }))
        }
        "begin" => {
            let mut body = Vec::with_capacity(rest.len());
            for b in rest {
                body.push(core_from_value(b, heap, file_id, table)?);
            }
            Ok(Rc::new(CoreExpr::Begin { body, span }))
        }
        "module" => {
            if rest.len() < 3 {
                return Err(internal_x("module 节点字段数异常"));
            }
            let name = table.intern(as_str(&rest[0]).map_err(read_to_expand)?);
            let imports = symbol_vec_field(&rest[1], heap, table)?;
            let exports = symbol_vec_field(&rest[2], heap, table)?;
            let mut body = Vec::with_capacity(rest.len() - 3);
            for b in &rest[3..] {
                body.push(core_from_value(b, heap, file_id, table)?);
            }
            Ok(Rc::new(CoreExpr::Module {
                name,
                imports,
                exports,
                body,
                span,
            }))
        }
        "require" => {
            let mut caps = Vec::with_capacity(rest.len());
            for c in rest {
                let cap_name = as_str(c).map_err(read_to_expand)?;
                let cap = match cap_name {
                    "read" => Capability::IoRead,
                    "write" => Capability::IoWrite,
                    other => {
                        return Err(internal_x(&format!("未知能力项「{}」", other)));
                    }
                };
                caps.push(cap);
            }
            Ok(Rc::new(CoreExpr::Require { caps, span }))
        }
        other => Err(internal_x(&format!("未知 core 节点种类 '{}'", other))),
    }
}

/// VM 值 → 字面量值（quote 产物：序对树经堆走查递归重建）。
fn literal_from_value(v: &Value, heap: &Heap) -> Result<LiteralValue, ExpandError> {
    match v {
        Value::Int(i) => Ok(LiteralValue::Int(*i)),
        Value::Float(f) => Ok(LiteralValue::Float(*f)),
        Value::Str(s) => Ok(LiteralValue::Str(s.clone())),
        Value::Bool(b) => Ok(LiteralValue::Bool(*b)),
        Value::Nil => Ok(LiteralValue::Nil),
        Value::Symbol(s) => Ok(LiteralValue::Symbol(s.clone())),
        Value::Pair(r) => {
            let (car, cdr) = heap.get_pair(*r).ok_or_else(internal_heap_x)?;
            let car_l = literal_from_value(&unbox_slot(car, heap), heap)?;
            let cdr_l = literal_from_value(&unbox_slot(cdr, heap), heap)?;
            Ok(LiteralValue::Pair(Rc::new(car_l), Rc::new(cdr_l)))
        }
        other => Err(internal_x(&format!(
            "不支持的字面量值 {}",
            other.type_name()
        ))),
    }
}

// ---- 字段访问辅助 ----

fn field(rest: &[Value], idx: usize) -> Result<&Value, ExpandError> {
    rest.get(idx).ok_or_else(|| internal_x("core 节点字段缺失"))
}

fn str_field(rest: &[Value], idx: usize) -> Result<&str, ExpandError> {
    as_str(field(rest, idx)?).map_err(read_to_expand)
}

/// int 列表值 → ScopeSet（排序去重——与 ScopeSet::from_iter_scopes 同一语义）。
fn scope_set_from_value(v: &Value, heap: &Heap) -> Result<ScopeSet, ExpandError> {
    let fields = value_list_fields(v, heap).map_err(read_to_expand)?;
    let ids = fields
        .iter()
        .map(|f| as_int(f).map_err(read_to_expand))
        .collect::<Result<Vec<i64>, _>>()?;
    Ok(ScopeSet::from_iter_scopes(
        ids.into_iter().map(|i| i as u32),
    ))
}

fn scope_set_field(rest: &[Value], idx: usize, heap: &Heap) -> Result<ScopeSet, ExpandError> {
    scope_set_from_value(field(rest, idx)?, heap)
}

/// str 列表值 → Symbol 向量（经用户表 intern）。
fn symbol_vec_field(
    v: &Value,
    heap: &Heap,
    table: &mut SymbolTable,
) -> Result<Vec<Symbol>, ExpandError> {
    let fields = value_list_fields(v, heap).map_err(read_to_expand)?;
    let mut out = Vec::with_capacity(fields.len());
    for f in &fields {
        out.push(table.intern(as_str(f).map_err(read_to_expand)?));
    }
    Ok(out)
}

/// ReadError → ExpandError（共享走查辅助的错误面转换——消息与 Span 保真）。
fn read_to_expand(e: kerf_reader::ReadError) -> ExpandError {
    ExpandError {
        message: e.message,
        span: e.span,
    }
}

fn internal_heap_x() -> ExpandError {
    expand_internal("堆序对读取失败")
}

fn internal_x(msg: &str) -> ExpandError {
    expand_internal(msg)
}

fn expand_internal(msg: &str) -> ExpandError {
    ExpandError {
        message: format!("[bootstrap-exp] 自举 Expander 内部错误：{}", msg),
        span: Span::dummy(),
    }
}
