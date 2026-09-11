//! 自举 Compiler（I1 前段：compile 段 kerf 化——42-b parity 影子路径）。
//!
//! 架构（07-bootstrap §3.2 混合期构成；r6/r14 三件套第三实例——
//! i1-incision-migration-design §2 INC1）：
//! - **编译逻辑**：`bootstrap/compiler.krf`（kerf 源码，经种子管线编译
//!   为字节码后在 Stage 0 VM 上运行——`lexc-compile-program` 入口，
//!   与 kerf-compiler 的 `compile_module` 接口形状对齐，§11）；
//! - **桥（本模块）**：宿主侧调用（`call_closure`）+ 值树双向转换
//!   （`CoreExpr` → core 节点（含 span/作用域集/标记化字面量）→
//!   VM 节点 → `('prog ...)` → `BcProgram` 类型重建）；
//! - **种子（kerf-compiler）**：自举引导（compiler.krf 的编译）+ parity
//!   测试的 oracle（自举种子经典角色——两实现互为印证）。
//!
//! **42-b 边界（显式登记，不静默——§2.3-4）**：本模块是 parity 影子
//! 路径——生产管线（`compile_front`）仍走 Rust `compile_module`
//! （CompilerKind 切换属 42-d，i1-incision-design §6 P1）；module/
//! require 两臂属 42-c（compiler.krf 显式边界错误）。parity 判据 =
//! `BcProgram::bytecode_equal`（INC5——全结构含 debug_spans）。
//!
//! 堆契约：Compiler 程序的持久堆承载全局状态（CC-* 编译上下文——
//! 入口复位纪律使其不含跨调用影响输出的状态）与各次调用中间产物——
//! GC 以（栈+帧+全局）为根集，跨调用回收安全；输入节点树在持久堆上
//! 构造（一次性，调用后可回收）。
//!
//! 并发契约：thread_local 状态（Rc 值非 Sync）——每线程惰性编译加载一次。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kerf_compiler::{BcConst, BcProgram, BcProto, CaptureSource, CompileError, Op};
use kerf_core::{CoreExpr, LiteralValue};
use kerf_runtime::{BoxedInput, Heap};
use kerf_span::{FileId, SourceMap, Span};
use kerf_syntax::{Symbol, SymbolTable};
use kerf_vm::{box_value, call_closure, run_program, unbox_slot, Value, VmError};

use crate::bootstrap::{as_int, as_str, as_symbol_name, value_list_fields};
use crate::builtins::register_globals;

/// Compiler 源码（编译期嵌入——产物自包含，不依赖外部文件）。
const COMPILER_SRC: &str = include_str!("bootstrap/compiler.krf");
/// Compiler 源文件名（诊断渲染用）。
const COMPILER_FILENAME: &str = "compiler.krf";
/// 入口名（compiler.krf 顶层定义）。
const ENTRY_NAME: &str = "lexc-compile-program";

/// main 原型名（渲染由调用方替换——种子 compile.rs 同款魔法符号）。
const MAIN_PROTO_NAME: Symbol = Symbol(u32::MAX - 1);
/// 无参 Lambda 原型名。
const ANON_PROTO_NAME: Symbol = Symbol(u32::MAX - 2);

/// 自举 Compiler 状态（每线程一份，惰性初始化）。
struct BootstrapCompilerState {
    /// Compiler 程序字节码（种子管线编译）。
    program: BcProgram,
    /// 已加载全局（内置 + compiler.krf 顶层定义）。
    globals: HashMap<Symbol, Value>,
    /// `lexc-compile-program` 入口符号。
    entry_sym: Symbol,
    /// Compiler 的源映射（内部错误诊断渲染）。
    source_map: SourceMap,
    /// 持久堆：全局数据 + 各次调用产物（GC 根集含 globals）。
    heap: Heap,
}

thread_local! {
    static BOOTSTRAP_CMP: RefCell<Option<BootstrapCompilerState>> = const {
        RefCell::new(None)
    };
}

/// 自举编译入口：`Vec<Rc<CoreExpr>>` → `BcProgram`（与
/// `kerf_compiler::compile_module` 同形——parity oracle 对照面）。
///
/// **42-b 边界**：parity 影子路径（生产未切换——`compile_front` 仍走
/// 种子 `compile_module`；切换点设计见 i1-incision-design §6 P1）。
pub fn compile_module(
    exprs: &[Rc<CoreExpr>],
    file_id: FileId,
    table: &mut SymbolTable,
) -> Result<BcProgram, CompileError> {
    with_compiler(|st| {
        // 阶段 1：CoreExpr → VM core 节点（span/作用域集/符号名 str 携带；
        // 字面量消歧标记化——Str/Symbol/Float 在 VM 值面无谓词区分，
        // 桥侧定型传递）
        let nodes: Vec<Value> = exprs
            .iter()
            .map(|e| core_to_node(e, table, &mut st.heap))
            .collect();
        let input = heap_list(&mut st.heap, nodes);
        // 阶段 2：入口调用（VM 上运行 compiler.krf）
        let output = call_entry(st, input)?;
        // 阶段 3：错误检查 + prog 节点桥（BcProgram 类型重建）
        if let Some(e) = as_compile_err(&output, &st.heap, file_id) {
            return Err(e);
        }
        prog_from_value(&output, &st.heap, file_id, table)
    })
}

/// 以线程局部状态执行（惰性初始化）。
fn with_compiler<T>(
    f: impl FnOnce(&mut BootstrapCompilerState) -> Result<T, CompileError>,
) -> Result<T, CompileError> {
    BOOTSTRAP_CMP.with(|cell| {
        let mut opt = cell.borrow_mut();
        if opt.is_none() {
            match load_compiler() {
                Ok(state) => *opt = Some(state),
                Err(e) => return Err(e),
            }
        }
        f(opt.as_mut().expect("上方已初始化"))
    })
}

/// 编译并加载 Compiler 程序（种子路径——Rust Reader/Expander/Compiler
/// 编译 compiler.krf，不经自举路径：种子编译自举 Compiler，无递归）。
fn load_compiler() -> Result<BootstrapCompilerState, CompileError> {
    let front =
        crate::driver::compile_front_seed(COMPILER_SRC, COMPILER_FILENAME).map_err(|e| {
            compile_internal(&format!(
                "[bootstrap-cmp] 自举 Compiler 编译失败（种子管线）：{}",
                e.rendered
            ))
        })?;
    let mut table = front.table;
    // compiler.krf 无 I/O 引用（纯数据变换）——空授权（R9 fail-closed）
    let mut globals = register_globals(&mut table, &crate::capability::IoGrant::none());
    let program = front.program;
    crate::builtins::resolve_hygiene_fallbacks(&program, &mut table, &mut globals);
    let mut heap = Heap::new();
    let outcome = run_program(&program, &mut globals, &mut heap)
        .map_err(|e| vm_error_to_compile(&e, &front.source_map))?;
    let _ = outcome; // 主原型仅执行顶层定义（值无意义）
    let entry_sym = table.intern(ENTRY_NAME);
    Ok(BootstrapCompilerState {
        program,
        globals,
        entry_sym,
        source_map: front.source_map,
        heap,
    })
}

/// 调用 Compiler 入口（宿主信任层程序化调用——§11 与 run_program 同级）。
fn call_entry(st: &mut BootstrapCompilerState, input: Value) -> Result<Value, CompileError> {
    let callee = st.globals.get(&st.entry_sym).cloned().ok_or_else(|| {
        compile_internal(&format!(
            "[bootstrap-cmp] 入口 {} 未定义（compiler.krf 损坏）",
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
    .map_err(|e| vm_error_to_compile(&e, &st.source_map))
}

/// VM 错误 → CompileError（防御路径：编译错误经 ('err ...) 值返回，
/// 不经此通道——此路径仅覆盖 compiler.krf 内部缺陷或帧上限）。
fn vm_error_to_compile(e: &VmError, sm: &SourceMap) -> CompileError {
    use kerf_span::{render_diagnostic, Diagnostic, DiagnosticCode, Severity};
    let mut diag = Diagnostic::error(
        Some(DiagnosticCode(2)),
        format!("自举 Compiler 内部错误：{}", e.message),
        e.span,
    );
    for t in &e.trace {
        diag = diag.with_child(Severity::Note, "调用点", t.span);
    }
    CompileError {
        message: render_diagnostic(&diag, sm),
        span: Span::dummy(),
    }
}

// ---- 反向值桥：CoreExpr → VM core 节点 ----

/// `CoreExpr` → core 节点值 `(tag s e exp ...)`（符号名以 str 携带——
/// 桥回 intern；作用域集 = int 列表；字面量消歧标记形态）。
fn core_to_node(e: &CoreExpr, table: &SymbolTable, heap: &mut Heap) -> Value {
    let s = Value::Int(e.span().start as i64);
    let en = Value::Int(e.span().end as i64);
    let exp = Value::Int(e.span().expansion_id as i64);
    let tag = |name: &str| Value::Symbol(Rc::from(name));
    let items: Vec<Value> = match e {
        CoreExpr::Literal { value, .. } => {
            vec![tag("lit"), s, en, exp, litval_to_value(value, heap)]
        }
        CoreExpr::VarRef { name, scopes, .. } => vec![
            tag("var"),
            s,
            en,
            exp,
            Value::Str(Rc::from(table.name(*name))),
            scope_list_value(scopes, heap),
        ],
        CoreExpr::App { fn_expr, args, .. } => {
            let f = core_to_node(fn_expr, table, heap);
            let arg_nodes: Vec<Value> = args.iter().map(|a| core_to_node(a, table, heap)).collect();
            let mut v = vec![tag("app"), s, en, exp, f];
            v.extend(arg_nodes);
            v
        }
        CoreExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            let c = core_to_node(cond, table, heap);
            let t = core_to_node(then_branch, table, heap);
            let el = core_to_node(else_branch, table, heap);
            vec![tag("if"), s, en, exp, c, t, el]
        }
        CoreExpr::Lambda {
            params,
            param_scopes,
            body,
            ..
        } => {
            let names: Vec<Value> = params
                .iter()
                .map(|p| Value::Str(Rc::from(table.name(*p))))
                .collect();
            let scopes: Vec<Value> = param_scopes
                .iter()
                .map(|sc| scope_list_value(sc, heap))
                .collect();
            let params_v = heap_list(heap, names);
            let scopes_v = heap_list(heap, scopes);
            let body_v = core_to_node(body, table, heap);
            vec![tag("lambda"), s, en, exp, params_v, scopes_v, body_v]
        }
        CoreExpr::SetBang {
            name,
            scopes,
            value,
            ..
        } => {
            let sc = scope_list_value(scopes, heap);
            let val = core_to_node(value, table, heap);
            vec![
                tag("set"),
                s,
                en,
                exp,
                Value::Str(Rc::from(table.name(*name))),
                sc,
                val,
            ]
        }
        CoreExpr::Define { name, value, .. } => {
            let val = core_to_node(value, table, heap);
            vec![
                tag("define"),
                s,
                en,
                exp,
                Value::Str(Rc::from(table.name(*name))),
                val,
            ]
        }
        CoreExpr::Begin { body, .. } => {
            let item_nodes: Vec<Value> =
                body.iter().map(|b| core_to_node(b, table, heap)).collect();
            let mut v = vec![tag("begin"), s, en, exp];
            v.extend(item_nodes);
            v
        }
        CoreExpr::Module {
            name,
            imports,
            exports,
            body,
            ..
        } => {
            let strs = |syms: &[Symbol]| -> Vec<Value> {
                syms.iter()
                    .map(|x| Value::Str(Rc::from(table.name(*x))))
                    .collect()
            };
            let imports_v = heap_list(heap, strs(imports));
            let exports_v = heap_list(heap, strs(exports));
            let item_nodes: Vec<Value> =
                body.iter().map(|b| core_to_node(b, table, heap)).collect();
            let mut v = vec![
                tag("module"),
                s,
                en,
                exp,
                Value::Str(Rc::from(table.name(*name))),
                imports_v,
                exports_v,
            ];
            v.extend(item_nodes);
            v
        }
        CoreExpr::Require { caps, .. } => {
            let mut v = vec![tag("require"), s, en, exp];
            v.extend(caps.iter().map(|c| Value::Str(Rc::from(c.as_str()))));
            v
        }
    };
    heap_list(heap, items)
}

/// `LiteralValue` → 消歧标记值（Str/Symbol/Float 在 VM 值面无谓词
/// 区分——桥侧定型标记；符号以名 str 携带，§7.3 符号 str 纪律）。
fn litval_to_value(v: &LiteralValue, heap: &mut Heap) -> Value {
    let tag = |name: &str| Value::Symbol(Rc::from(name));
    match v {
        LiteralValue::Int(i) => heap_list(heap, vec![tag("int"), Value::Int(*i)]),
        LiteralValue::Float(f) => heap_list(heap, vec![tag("float"), Value::Float(*f)]),
        LiteralValue::Str(s) => heap_list(heap, vec![tag("str"), Value::Str(s.clone())]),
        LiteralValue::Bool(b) => heap_list(heap, vec![tag(if *b { "true" } else { "false" })]),
        LiteralValue::Nil => heap_list(heap, vec![tag("nil")]),
        LiteralValue::Symbol(s) => heap_list(heap, vec![tag("sym"), Value::Str(s.clone())]),
        LiteralValue::Pair(a, b) => {
            let car = litval_to_value(a, heap);
            let cdr = litval_to_value(b, heap);
            heap_list(heap, vec![tag("pair"), car, cdr])
        }
    }
}

/// 作用域集 → int 列表值（升序迭代）。
fn scope_list_value(scopes: &kerf_syntax::ScopeSet, heap: &mut Heap) -> Value {
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

// ---- 正向值桥：VM prog 节点 → BcProgram ----

/// ('err 消息 起 止) → CompileError（非错误形态返回 None；Span 挂
/// 用户 file_id——错误位置在用户源文本内）。
fn as_compile_err(v: &Value, heap: &Heap, file_id: FileId) -> Option<CompileError> {
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
                    return Some(CompileError {
                        message: msg,
                        span: Span::new(file_id, s as u32, e as u32),
                    });
                }
            }
        }
    }
    None
}

/// `('prog (proto...) (const...) (glob...))` → `BcProgram`。
fn prog_from_value(
    v: &Value,
    heap: &Heap,
    file_id: FileId,
    table: &mut SymbolTable,
) -> Result<BcProgram, CompileError> {
    let fields = value_list_fields(v, heap).map_err(read_to_compile)?;
    if fields.len() != 4 {
        return Err(internal_x("prog 节点字段数异常"));
    }
    // 原型表
    let proto_nodes = value_list_fields(&fields[1], heap).map_err(read_to_compile)?;
    let mut protos = Vec::with_capacity(proto_nodes.len());
    for pn in &proto_nodes {
        protos.push(proto_from_value(pn, heap, file_id, table)?);
    }
    // 常量池
    let const_nodes = value_list_fields(&fields[2], heap).map_err(read_to_compile)?;
    let mut consts = Vec::with_capacity(const_nodes.len());
    for cn in &const_nodes {
        consts.push(const_from_value(cn, heap, table)?);
    }
    // 全局引用（首插入序）
    let glob_nodes = value_list_fields(&fields[3], heap).map_err(read_to_compile)?;
    let mut global_refs = Vec::with_capacity(glob_nodes.len());
    for g in &glob_nodes {
        let name = as_str(g).map_err(read_to_compile)?;
        global_refs.push(table.intern(name));
    }
    Ok(BcProgram {
        protos,
        consts,
        entry: 0,
        global_refs,
        module_name: None,
    })
}

/// `('proto 名形 (参数) nlocals (捕获名) (捕获源) (指令) (span) 自由变量)`
/// → `BcProto`。
fn proto_from_value(
    pn: &Value,
    heap: &Heap,
    file_id: FileId,
    table: &mut SymbolTable,
) -> Result<BcProto, CompileError> {
    let f = value_list_fields(pn, heap).map_err(read_to_compile)?;
    if f.len() != 9 {
        return Err(internal_x("proto 节点字段数异常"));
    }
    let name = proto_name_from_value(&f[1], heap, table)?;
    let params = symbol_vec_field(&f[2], heap, table)?;
    let n_locals = as_int(&f[3]).map_err(read_to_compile)? as u32;
    let capture_names = symbol_vec_field(&f[4], heap, table)?;
    let src_nodes = value_list_fields(&f[5], heap).map_err(read_to_compile)?;
    let mut capture_sources = Vec::with_capacity(src_nodes.len());
    for sn in &src_nodes {
        capture_sources.push(capture_source_from_value(sn, heap)?);
    }
    let op_nodes = value_list_fields(&f[6], heap).map_err(read_to_compile)?;
    let mut code = Vec::with_capacity(op_nodes.len());
    for on in &op_nodes {
        code.push(op_from_value(on, heap)?);
    }
    let span_nodes = value_list_fields(&f[7], heap).map_err(read_to_compile)?;
    let mut debug_spans = Vec::with_capacity(span_nodes.len());
    for sn in &span_nodes {
        let sp = value_list_fields(sn, heap).map_err(read_to_compile)?;
        if sp.len() != 3 {
            return Err(internal_x("span 三元组字段数异常"));
        }
        let s = as_int(&sp[0]).map_err(read_to_compile)?;
        let e = as_int(&sp[1]).map_err(read_to_compile)?;
        let x = as_int(&sp[2]).map_err(read_to_compile)?;
        debug_spans.push(Span {
            file_id,
            start: s as u32,
            end: e as u32,
            expansion_id: x as u32,
        });
    }
    let free_vars = symbol_vec_field(&f[8], heap, table)?;
    Ok(BcProto {
        name,
        params,
        n_locals,
        capture_names,
        capture_sources,
        code,
        debug_spans,
        free_vars,
    })
}

/// 原型名形：('main)（魔法符号）/ ('anon)（匿名）/ ('name 名字str)
/// （首个形参名——经用户表 intern）。
fn proto_name_from_value(
    v: &Value,
    heap: &Heap,
    table: &mut SymbolTable,
) -> Result<Symbol, CompileError> {
    let f = value_list_fields(v, heap).map_err(read_to_compile)?;
    if f.is_empty() {
        return Err(internal_x("原型名形字段缺失"));
    }
    let kind = as_symbol_name(&f[0]).map_err(read_to_compile)?;
    match kind {
        "main" => Ok(MAIN_PROTO_NAME),
        "anon" => Ok(ANON_PROTO_NAME),
        "name" => {
            if f.len() != 2 {
                return Err(internal_x("名形字段数异常"));
            }
            let name = as_str(&f[1]).map_err(read_to_compile)?;
            Ok(table.intern(name))
        }
        other => Err(internal_x(&format!("未知原型名形「{}」", other))),
    }
}

/// 捕获源节点：('local i) / ('captured i)。
fn capture_source_from_value(v: &Value, heap: &Heap) -> Result<CaptureSource, CompileError> {
    let f = value_list_fields(v, heap).map_err(read_to_compile)?;
    if f.len() != 2 {
        return Err(internal_x("捕获源字段数异常"));
    }
    let kind = as_symbol_name(&f[0]).map_err(read_to_compile)?;
    let i = as_int(&f[1]).map_err(read_to_compile)? as u32;
    match kind {
        "local" => Ok(CaptureSource::Local(i)),
        "captured" => Ok(CaptureSource::Captured(i)),
        other => Err(internal_x(&format!("未知捕获源「{}」", other))),
    }
}

/// 指令节点：('op 码int 操作数...) → `Op`（码表 = opcode.rs 声明序
/// 0..=40——与 compiler.krf OP-* 常量同表）。
fn op_from_value(v: &Value, heap: &Heap) -> Result<Op, CompileError> {
    let f = value_list_fields(v, heap).map_err(read_to_compile)?;
    if f.len() < 2 {
        return Err(internal_x("指令节点字段数异常"));
    }
    let code = as_int(&f[1]).map_err(read_to_compile)?;
    let operands: Vec<i64> = f[2..]
        .iter()
        .map(|x| as_int(x).map_err(read_to_compile))
        .collect::<Result<_, _>>()?;
    let need = |n: usize| -> Result<(), CompileError> {
        if operands.len() != n {
            Err(internal_x(&format!(
                "操作码 {} 操作数数异常（{} ≠ {}）",
                code,
                operands.len(),
                n
            )))
        } else {
            Ok(())
        }
    };
    Ok(match code {
        0 => {
            need(1)?;
            Op::PushConst(operands[0] as u32)
        }
        1 => {
            need(0)?;
            Op::PushNil
        }
        2 => {
            need(0)?;
            Op::PushTrue
        }
        3 => {
            need(0)?;
            Op::PushFalse
        }
        4 => {
            need(0)?;
            Op::Pop
        }
        5 => {
            need(0)?;
            Op::Dup
        }
        6 => {
            need(0)?;
            Op::Swap
        }
        7 => {
            need(1)?;
            Op::LoadLocal(operands[0] as u32)
        }
        8 => {
            need(1)?;
            Op::StoreLocal(operands[0] as u32)
        }
        9 => {
            need(1)?;
            Op::LoadGlobal(operands[0] as u32)
        }
        10 => {
            need(1)?;
            Op::StoreGlobal(operands[0] as u32)
        }
        11 => {
            need(1)?;
            Op::DefineGlobal(operands[0] as u32)
        }
        12 => {
            need(1)?;
            Op::LoadCaptured(operands[0] as u32)
        }
        13 => {
            need(1)?;
            Op::StoreCaptured(operands[0] as u32)
        }
        14 => {
            need(1)?;
            Op::Jump(operands[0] as u32)
        }
        15 => {
            need(1)?;
            Op::JumpIfFalse(operands[0] as u32)
        }
        16 => {
            need(2)?;
            Op::Closure {
                proto: operands[0] as u32,
                n_captures: operands[1] as u32,
            }
        }
        17 => {
            need(1)?;
            Op::Call(operands[0] as u32)
        }
        18 => {
            need(1)?;
            Op::TailCall(operands[0] as u32)
        }
        19 => {
            need(0)?;
            Op::Ret
        }
        20 => {
            need(0)?;
            Op::Add
        }
        21 => {
            need(0)?;
            Op::Sub
        }
        22 => {
            need(0)?;
            Op::Mul
        }
        23 => {
            need(0)?;
            Op::Div
        }
        24 => {
            need(0)?;
            Op::Mod
        }
        25 => {
            need(0)?;
            Op::NumLt
        }
        26 => {
            need(0)?;
            Op::NumGt
        }
        27 => {
            need(0)?;
            Op::NumLe
        }
        28 => {
            need(0)?;
            Op::NumGe
        }
        29 => {
            need(0)?;
            Op::NumEq
        }
        30 => {
            need(0)?;
            Op::Eq
        }
        31 => {
            need(0)?;
            Op::Not
        }
        32 => {
            need(0)?;
            Op::MakePair
        }
        33 => {
            need(0)?;
            Op::Car
        }
        34 => {
            need(0)?;
            Op::Cdr
        }
        35 => {
            need(0)?;
            Op::IsNull
        }
        36 => {
            need(0)?;
            Op::IsPair
        }
        37 => {
            need(0)?;
            Op::IsInt
        }
        38 => {
            need(0)?;
            Op::IsBool
        }
        39 => {
            need(0)?;
            Op::IsProcedure
        }
        40 => {
            need(0)?;
            Op::Halt
        }
        other => {
            return Err(internal_x(&format!("未知操作码 {}", other)));
        }
    })
}

/// 常量节点：('const tag 值) → `BcConst`（sym 经用户表 intern；
/// symlit/str 按内容——与种子 intern 的内容等价口径）。
fn const_from_value(
    v: &Value,
    heap: &Heap,
    table: &mut SymbolTable,
) -> Result<BcConst, CompileError> {
    let f = value_list_fields(v, heap).map_err(read_to_compile)?;
    if f.len() != 3 {
        return Err(internal_x("const 节点字段数异常"));
    }
    let tag = as_symbol_name(&f[1]).map_err(read_to_compile)?;
    match tag {
        "int" => Ok(BcConst::Int(as_int(&f[2]).map_err(read_to_compile)?)),
        "float" => match &f[2] {
            Value::Float(x) => Ok(BcConst::Float(*x)),
            other => Err(internal_x(&format!(
                "float 常量值类型异常 {}",
                other.type_name()
            ))),
        },
        "str" => Ok(BcConst::Str(Rc::from(
            as_str(&f[2]).map_err(read_to_compile)?,
        ))),
        "bool" => match &f[2] {
            Value::Bool(b) => Ok(BcConst::Bool(*b)),
            other => Err(internal_x(&format!(
                "bool 常量值类型异常 {}",
                other.type_name()
            ))),
        },
        "nil" => Ok(BcConst::Nil),
        "sym" => {
            let name = as_str(&f[2]).map_err(read_to_compile)?;
            Ok(BcConst::Symbol(table.intern(name)))
        }
        "symlit" => Ok(BcConst::SymLit(Rc::from(
            as_str(&f[2]).map_err(read_to_compile)?,
        ))),
        other => Err(internal_x(&format!("未知常量类型「{}」", other))),
    }
}

// ---- 字段访问辅助 ----

/// str 列表值 → Symbol 向量（经用户表 intern）。
fn symbol_vec_field(
    v: &Value,
    heap: &Heap,
    table: &mut SymbolTable,
) -> Result<Vec<Symbol>, CompileError> {
    let fields = value_list_fields(v, heap).map_err(read_to_compile)?;
    let mut out = Vec::with_capacity(fields.len());
    for f in &fields {
        let name = as_str(f).map_err(read_to_compile)?;
        out.push(table.intern(name));
    }
    Ok(out)
}

/// ReadError → CompileError（共享走查辅助的错误面转换——消息与 Span
/// 保真）。
fn read_to_compile(e: kerf_reader::ReadError) -> CompileError {
    CompileError {
        message: e.message,
        span: e.span,
    }
}

fn internal_x(msg: &str) -> CompileError {
    compile_internal(msg)
}

fn compile_internal(msg: &str) -> CompileError {
    CompileError {
        message: format!("[bootstrap-cmp] 自举 Compiler 内部错误：{}", msg),
        span: Span::dummy(),
    }
}
