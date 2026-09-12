//! 字节码 VM：switch-dispatch 执行循环（stage0.md §8.12 / §19.5）。
//!
//! **VM 状态**（§8.12）：代码（protos）、数据栈、调用栈、全局环境、常量池、
//! 调试信息表（前四者在本结构；后二者经 `BcProgram` 引用）。
//!
//! **调用帧三扩展槽**（§8.12——格式冻结，Stage 0 可为空但格式锁定）：
//! - `ext1`：continuation / effect handler 预留；
//! - `ext2`：异常处理表预留；
//! - `ext3`：调试帧信息预留。
//!
//! **执行循环骨架**（§19.5）：
//! - fetch-decode（pc 自增；pc 只在 JUMP/RET/CALL 改变——不变式 2）；
//! - 运行时错误经 debug_info_table[pc] 反查 Span 生成堆栈追踪；
//! - 每轮循环末尾的分配配额检查 = GC 安全点（§19.4 陷阱 2）。
//!
//! **迭代式主循环**：kerf 递归以帧栈（堆分配）承载——Rust 栈深度恒定，
//! 深递归不再受宿主栈限制（帧数上限 100_000 显式防护）。

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use kerf_compiler::{BcConst, BcProgram, Op};
use kerf_runtime::{BoxedInput, ForeignBox, GcRef, Heap, RootSet};
use kerf_span::Span;
use kerf_syntax::Symbol;

use crate::value::{BuiltinFn, ClosureValue, GcCell, Value};

/// 运行时上下文（heap 持有方）。
pub struct Rt {
    pub heap: Heap,
}

impl Rt {
    /// 构造（GC 开启——VM 路径语义）。
    pub fn new(heap: Heap) -> Self {
        Rt { heap }
    }
}

/// 效应处理器帧数据（r25/42-f——D6 ext1 具体化：`ContinuationSlot`
/// 预留槽激活，结构零变更——原则 27 接口预留时机兑现）。
///
/// 携带分派键 + handler 原型 + 捕获单元 + 安装时数据栈水位：
/// perform 扫描匹配后据此变形 handler 帧并截回数据栈（外层中间值
/// 保持；挂起帧链/中间值已存入 continuation 快照）。
#[derive(Debug, Clone)]
pub struct HandlerFrame {
    /// 效应族标签（match 单键分派键——符号字面量同型 `Rc<str>`，与
    /// `CoreExpr::Handle.tag` / `Value::Symbol` 同载体）。
    pub tag: Rc<str>,
    /// handler 原型（params = [payload_var, resume_var]）。
    pub handler_proto: u32,
    /// handler 原型捕获单元（共享可变——与 Closure 捕获同机制；
    /// dispatch 变形时作为 H 执行帧的 captures）。
    pub captures: Vec<Rc<GcCell>>,
    /// 安装时刻数据栈深度（dispatch 截回水位——R11-dispatch）。
    pub stack_watermark: usize,
}

/// 结构相等（FrameExt 派生需要）：分派键/原型/水位按值，捕获按单元
/// 指针（同一安装点的双 clone 共享相等；不同安装点同结构不等——
/// 帧身份语义）。
impl PartialEq for HandlerFrame {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag
            && self.handler_proto == other.handler_proto
            && self.stack_watermark == other.stack_watermark
            && self.captures.len() == other.captures.len()
            && self
                .captures
                .iter()
                .zip(other.captures.iter())
                .all(|(a, b)| Rc::ptr_eq(a, b))
    }
}
impl Eq for HandlerFrame {}

/// 预留槽类型（ext2：异常处理表）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HandlerTableSlot;

/// 预留槽类型（ext3：调试帧信息）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DebugFrameSlot;

/// 调用帧扩展（三槽冻结格式；ext1 于 r25/42-f 具体化为效应 handler
/// 帧——D6 预留槽位激活；`Copy` 撤销（ext1 携带 `Rc`））。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameExt {
    /// ext1：效应 handler 帧（None = 未安装）。
    pub ext1: Option<Rc<HandlerFrame>>,
    /// ext2：异常处理表（Stage 0 空）。
    pub ext2: Option<HandlerTableSlot>,
    /// ext3：调试帧信息（Stage 0 空）。
    pub ext3: Option<DebugFrameSlot>,
}

impl FrameExt {
    /// 三槽构造（§19.5 `FrameExt::new()`）。
    pub fn new() -> Self {
        FrameExt::default()
    }
}

/// 调用帧。
#[derive(Debug, Clone)]
pub struct Frame {
    /// 返回原型索引。
    pub ret_proto: usize,
    /// 返回 pc。
    pub ret_pc: u32,
    /// 本帧执行的原型索引。
    pub proto: usize,
    /// 局部槽（参数 + let 绑定）——**共享单元格**（捕获语义：set! 经
    /// `Rc<GcCell>` 传播到闭包，letrec 递归成立；TD-023 堆根性摘要）。
    pub locals: Vec<Rc<GcCell>>,
    /// 闭包捕获槽（共享可变单元）。
    pub captures: Vec<Rc<GcCell>>,
    /// 三扩展槽（格式冻结）。
    pub ext: FrameExt,
    /// 调用点 Span（堆栈追踪）。
    pub call_span: Span,
}

/// 堆栈追踪帧。
#[derive(Debug, Clone)]
pub struct TraceFrame {
    /// 原型名符号。
    pub proto_name: Option<Symbol>,
    /// 位置。
    pub span: Span,
}

/// VM 执行错误（含堆栈追踪——§8.12 运行时错误捕获）。
#[derive(Debug, Clone)]
pub struct VmError {
    pub message: String,
    pub span: Span,
    pub trace: Vec<TraceFrame>,
    /// 结构化诊断码（None = E0004 运行时通用族；r25/42-f 效应族
    /// E0007-E0009 携码——driver 层 `from_vm` 按此映射）。
    pub code: Option<u32>,
}

impl VmError {
    /// 构造（crate 内 + ffi 边界包装层共用——E0004 通用族）。
    pub(crate) fn new(message: impl Into<String>, span: Span) -> Self {
        VmError {
            message: message.into(),
            span,
            trace: Vec::new(),
            code: None,
        }
    }

    /// 携结构化码构造（E0007-E0009 效应族 + E0010-E0012 FFI 族——
    /// messages 单源消息配套）。
    pub(crate) fn new_code(code: u32, message: impl Into<String>, span: Span) -> Self {
        VmError {
            message: message.into(),
            span,
            trace: Vec::new(),
            code: Some(code),
        }
    }

    /// 渲染堆栈追踪（人类可感知输出）。
    pub fn render_trace(&self, resolve: &dyn Fn(Symbol) -> String) -> String {
        let mut out = format!("运行时错误：{}\n  at {}\n", self.message, self.span);
        for t in &self.trace {
            let name = t.proto_name.map(resolve).unwrap_or_else(|| "<main>".into());
            out.push_str(&format!("  called from {} ({})\n", name, t.span));
        }
        out
    }
}

/// 帧数上限（迭代式循环下防护失控递归的显式上限）。
pub const MAX_FRAMES: usize = 100_000;

/// 指令预算上限（TD-022/H2——TCO 尾循环护栏）：帧数不再增长的无限
/// 尾循环（如 `(define (l) (l))`）由指令预算兜底（结构化报错非挂死）。
/// 量级标定：gc_stress ≈ 5×10^5 / 自举 Reader 100KB 源 ≈ 10^7 —— 10^9
/// = 两个数量级裕度（约数十秒量级执行上限，捕获真无限循环）。
pub const MAX_INSTRUCTIONS: u64 = 1_000_000_000;

/// GC 安全点轮询间隔（指令数）。
const GC_POLL_INTERVAL: u64 = 256;

/// GC 冷却倍数：回收收益低（存活率高）时的轮询退避（防止深递归下
/// 根集扫描退化为 O(n²)——根集 O(帧数) × 轮询 O(指令/间隔)）。
const GC_COOLDOWN_MULTIPLIER: u64 = 64;

/// 执行程序入口（§10.1 规则 1 自由函数）。
///
/// `vm::run(&bytecode, &mut runtime)` 形态：全局环境作为参数
/// （driver 注册内置后传入；返回最终值）。
///
/// **FFI 面（r30/48-d）**：本入口的 extern 符号表 = **空表**
/// （fail-closed：任何 FFI 操作码经此入口执行 → E0012 符号解析
/// 失败——生产 run 路径无 FFI 注册面）。带外部函数注册的执行面走
/// [`run_program_with_externs`]；自举模块（Reader/Expander/
/// Compiler）不含 FFI 操作码——`call_closure` 同型维持空表口径。
pub fn run_program(
    program: &BcProgram,
    globals: &mut HashMap<Symbol, Value>,
    heap: &mut Heap,
) -> Result<Value, VmError> {
    let externs = crate::ffi::ExternSymbolTable::new();
    run_program_frames(program, globals, heap, &externs)
}

/// 带 extern 符号表的执行入口（r30/48-d——FFI 注册面：
/// [`crate::ffi::call_external`] 窗口规程的驱动入口）。
///
/// 语义与 [`run_program`] 同构；区别仅在 FFI 操作码的符号解析域
/// （extern 符号表——扁平符号空间，ffi-ownership-model §4）。
/// 未登记符号 = E0012（fail-closed——QBE AOT 链接期解析在 VM
/// 路径的调用期对应物）。
pub fn run_program_with_externs(
    program: &BcProgram,
    globals: &mut HashMap<Symbol, Value>,
    heap: &mut Heap,
    externs: &crate::ffi::ExternSymbolTable,
) -> Result<Value, VmError> {
    run_program_frames(program, globals, heap, externs)
}

/// 执行程序帧初始化 + 追踪快照（run_program 系共体）。
fn run_program_frames(
    program: &BcProgram,
    globals: &mut HashMap<Symbol, Value>,
    heap: &mut Heap,
    externs: &crate::ffi::ExternSymbolTable,
) -> Result<Value, VmError> {
    let mut frames: Vec<Frame> = vec![Frame {
        ret_proto: 0,
        ret_pc: 0,
        proto: program.entry as usize,
        locals: Vec::new(),
        captures: Vec::new(),
        ext: FrameExt::new(),
        call_span: Span::dummy(),
    }];
    let outcome = execute(
        program,
        globals,
        heap,
        &mut frames,
        MAX_INSTRUCTIONS,
        externs,
    );
    outcome.map_err(|mut e| {
        // 调用点追踪（§8.12 运行时错误捕获）：错误发生时的活跃帧链。
        // 跳过主帧；保留最内 16 帧（深递归下外层无信息量，防诊断爆炸）；
        // 帧序 = 调用时间序（外层→内层），与 render_trace 的
        // "called from" 链一致。
        if e.trace.is_empty() && frames.len() > 1 {
            let start = frames.len().saturating_sub(MAX_TRACE_FRAMES).max(1);
            e.trace = frames[start..]
                .iter()
                .map(|f| TraceFrame {
                    proto_name: Some(program.protos[f.proto].name),
                    span: f.call_span,
                })
                .collect();
        }
        e
    })
}

/// 调用点追踪截断上限（深递归诊断防爆——仅保留最内 N 帧）。
const MAX_TRACE_FRAMES: usize = 16;

/// 预算参数化执行入口（TD-022/H2——测试面）：与 [`run_program`] 同构，
/// 但指令预算可注入（无限尾循环负例用小预算快速触发护栏——38-c
/// find_qbe 纯函数注入同型）。生产路径恒用 [`MAX_INSTRUCTIONS`]。
/// FFI 面：空表 fail-closed 口径（同 [`run_program`]——预算测试
/// 不消费 FFI 操作码）。
#[doc(hidden)]
pub fn run_program_with_budget(
    program: &BcProgram,
    globals: &mut HashMap<Symbol, Value>,
    heap: &mut Heap,
    max_instructions: u64,
) -> Result<Value, VmError> {
    let externs = crate::ffi::ExternSymbolTable::new();
    run_program_with_budget_inner(program, globals, heap, max_instructions, &externs)
}

/// 预算入口内体（externs 参数化——测试注入面）。
#[doc(hidden)]
pub fn run_program_with_budget_inner(
    program: &BcProgram,
    globals: &mut HashMap<Symbol, Value>,
    heap: &mut Heap,
    max_instructions: u64,
    externs: &crate::ffi::ExternSymbolTable,
) -> Result<Value, VmError> {
    let mut frames: Vec<Frame> = vec![Frame {
        ret_proto: 0,
        ret_pc: 0,
        proto: program.entry as usize,
        locals: Vec::new(),
        captures: Vec::new(),
        ext: FrameExt::new(),
        call_span: Span::dummy(),
    }];
    execute(
        program,
        globals,
        heap,
        &mut frames,
        max_instructions,
        externs,
    )
}

/// 宿主侧闭包调用入口（自举 Reader 等宿主消费方使用，B3）。
///
/// 语义：以「函数入口帧」进入执行循环——callee 必须是**本 program**
/// 编译产出的字节码闭包；参数注入局部槽；底帧 RET 即本次调用的返回值。
/// 错误路径与 [`run_program`] 同构（活跃帧链快照 → 调用点追踪）。
///
/// 边界（§11 接口隔离）：本入口与 run_program 同级——宿主信任层对已
/// 加载程序的程序化调用；builtin 内部递归 re-entry 仍被禁止（P5 否决
/// 维持）；eval 路径闭包不支持（双路径用于互查，宿主调用走字节码）。
pub fn call_closure(
    program: &BcProgram,
    globals: &mut HashMap<Symbol, Value>,
    heap: &mut Heap,
    callee: &Value,
    args: Vec<Value>,
) -> Result<Value, VmError> {
    let (proto, captures) = match callee {
        Value::Closure(rc) => match rc.as_ref() {
            ClosureValue::Bytecode { proto, captures } => (*proto, captures.clone()),
            // eval 臂理由：双路径互查设计（T1）——eval 闭包属树走查路径，
            // 宿主程序化调用仅支持字节码形态
            ClosureValue::Eval { .. } => {
                return Err(VmError::new(
                    "call_closure 不支持 eval 路径闭包（宿主调用走字节码路径）",
                    Span::dummy(),
                ))
            }
        },
        // other 臂理由：非闭包值（int/str/builtin 等）无原型可进入
        other => {
            return Err(VmError::new(
                format!("call_closure 需要闭包，实际 {}", other.type_name()),
                Span::dummy(),
            ))
        }
    };
    // 原型越界防御：跨程序闭包（原型索引指向别的 program）不可调用——
    // 显式报错而非越界 panic（§2.3-4 报错>静默；P3 否决的运行时面）
    let proto_idx = proto as usize;
    if proto_idx >= program.protos.len() {
        return Err(VmError::new(
            format!(
                "闭包原型索引越界：proto {} / 共 {}（跨程序闭包不可调用）",
                proto,
                program.protos.len()
            ),
            Span::dummy(),
        ));
    }
    let target = &program.protos[proto_idx];
    if target.params.len() != args.len() {
        return Err(VmError::new(
            format!(
                "过程参数数量不匹配：期望 {} 实际 {}",
                target.params.len(),
                args.len()
            ),
            Span::dummy(),
        ));
    }
    // 参数入单元格（捕获共享语义的帧基础——与 CALL 同构；GcCell
    // 承载堆根性摘要标志，TD-023）
    let local_cells: Vec<Rc<GcCell>> = args.into_iter().map(|v| Rc::new(GcCell::new(v))).collect();
    let mut frames: Vec<Frame> = vec![Frame {
        ret_proto: 0,
        ret_pc: 0,
        proto: proto_idx,
        locals: local_cells,
        captures,
        ext: FrameExt::new(),
        call_span: Span::dummy(),
    }];
    let externs = crate::ffi::ExternSymbolTable::new();
    let outcome = execute(
        program,
        globals,
        heap,
        &mut frames,
        MAX_INSTRUCTIONS,
        &externs,
    );
    outcome.map_err(|mut e| {
        // 调用点追踪：与 run_program 同构（内层 16 帧）
        if e.trace.is_empty() && frames.len() > 1 {
            let start = frames.len().saturating_sub(MAX_TRACE_FRAMES).max(1);
            e.trace = frames[start..]
                .iter()
                .map(|f| TraceFrame {
                    proto_name: Some(program.protos[f.proto].name),
                    span: f.call_span,
                })
                .collect();
        }
        e
    })
}

/// 主执行循环（run_program 内层——错误路径的 `return`/`?` 在此传播，
/// 外层从 `frames` 快照附加堆栈追踪）。
fn execute(
    program: &BcProgram,
    globals: &mut HashMap<Symbol, Value>,
    heap: &mut Heap,
    frames: &mut Vec<Frame>,
    max_instructions: u64,
    externs: &crate::ffi::ExternSymbolTable,
) -> Result<Value, VmError> {
    let mut stack: Vec<Value> = Vec::new();
    let mut pc: u32 = 0;
    let mut instructions: u64 = 0;
    let mut gc_cooldown: u64 = 0;
    // TD-023 对症（r24/42-e）：GC 根扫描缓冲跨周期复用（clear 保容量
    // ——根遍历无分配化路径；visited 在闭包密集负载下免每周期重建）
    let mut gc_root_scratch: Vec<GcRef> = Vec::new();
    let mut gc_visited_scratch: HashSet<usize> = HashSet::new();

    // 主循环（迭代式——kerf 递归经帧栈承载，Rust 栈恒定）
    loop {
        let frame_idx = frames.len() - 1;
        let proto_idx = frames[frame_idx].proto;
        let proto = &program.protos[proto_idx];
        let pc_u = pc as usize;
        // 指令边界防御：pc 越界 = 字节码损坏（显式失败，不静默）
        if pc_u >= proto.code.len() {
            return Err(VmError::new(
                format!(
                    "pc 越界：proto {} pc {}（字节码损坏或缺少终止指令）",
                    proto_idx, pc
                ),
                proto.debug_spans.get(pc_u).copied().unwrap_or_default(),
            ));
        }
        let op = proto.code[pc_u].clone();
        let span = proto.debug_spans.get(pc_u).copied().unwrap_or_default();
        // pc 单调性例外（§19.5 不变式 2）：JUMP/RET/CALL 改变 pc；其余 +1
        pc += 1;
        instructions += 1;
        // 指令预算护栏（TD-022/H2）：TCO 后帧数不增的无限尾循环在此兜底
        if instructions > max_instructions {
            return Err(VmError::new(
                format!(
                    "指令数超过上限 {}（疑似无限循环——TCO 尾循环护栏）",
                    max_instructions
                ),
                span,
            ));
        }

        macro_rules! pop {
            () => {
                stack.pop().ok_or_else(|| VmError::new("数据栈下溢（字节码栈不平衡）", span))?
            };
        }
        macro_rules! pop2 {
            () => {{
                let b = stack
                    .pop()
                    .ok_or_else(|| VmError::new("数据栈下溢", span))?;
                let a = stack
                    .pop()
                    .ok_or_else(|| VmError::new("数据栈下溢", span))?;
                (a, b)
            }};
        }
        macro_rules! push {
            ($v:expr) => {
                stack.push($v)
            };
        }
        macro_rules! binop {
            ($f:expr) => {{
                let (a, b) = pop2!();
                let r: Result<Value, String> = $f(&a, &b);
                match r {
                    Ok(v) => push!(v),
                    Err(m) => {
                        return Err(VmError::new(m, span));
                    }
                }
            }};
        }

        match &op {
            Op::PushConst(k) => {
                let c = &program.consts[*k as usize];
                push!(const_to_value(c));
            }
            Op::PushNil => push!(Value::Nil),
            Op::PushTrue => push!(Value::Bool(true)),
            Op::PushFalse => push!(Value::Bool(false)),
            Op::Pop => {
                pop!();
            }
            Op::Dup => {
                let v = stack
                    .last()
                    .cloned()
                    .ok_or_else(|| VmError::new("DUP 栈空", span))?;
                push!(v);
            }
            Op::Swap => {
                let n = stack.len();
                if n < 2 {
                    return Err(VmError::new("SWAP 栈深度不足", span));
                }
                stack.swap(n - 1, n - 2);
            }
            Op::LoadLocal(i) => {
                let f = &frames[frame_idx];
                let cell = Rc::clone(
                    f.locals
                        .get(*i as usize)
                        .ok_or_else(|| VmError::new(format!("局部槽 {} 越界", i), span))?,
                );
                push!(cell.get().clone());
            }
            Op::StoreLocal(i) => {
                let v = pop!();
                let f = &frames[frame_idx];
                let cell = Rc::clone(
                    f.locals
                        .get(*i as usize)
                        .ok_or_else(|| VmError::new(format!("局部槽 {} 越界", i), span))?,
                );
                // GcCell::set：写路径同步维护堆根性摘要（TD-023）
                cell.set(v);
            }
            Op::LoadGlobal(k) => {
                let sym = match &program.consts[*k as usize] {
                    BcConst::Symbol(s) => *s,
                    _ => return Err(VmError::new("LOAD_GLOBAL 操作数不是符号", span)),
                };
                match globals.get(&sym) {
                    Some(v) => push!(v.clone()),
                    None => {
                        return Err(VmError::new(
                            "未绑定的全局变量（卫生回退解析已在 driver 完成）",
                            span,
                        ))
                    }
                }
            }
            Op::StoreGlobal(k) => {
                let sym = match &program.consts[*k as usize] {
                    BcConst::Symbol(s) => *s,
                    _ => return Err(VmError::new("STORE_GLOBAL 操作数不是符号", span)),
                };
                let v = pop!();
                // S1/E3 语义（与 eval 路径 `Env::set` 对齐，T1 定理）：
                // set! 只写已存在的绑定——未绑定报错而非静默创建全局。
                if !globals.contains_key(&sym) {
                    return Err(VmError::new(crate::messages::err_setbang_unbound(), span));
                }
                globals.insert(sym, v);
            }
            Op::DefineGlobal(k) => {
                let sym = match &program.consts[*k as usize] {
                    BcConst::Symbol(s) => *s,
                    _ => return Err(VmError::new("DEFINE_GLOBAL 操作数不是符号", span)),
                };
                let v = pop!();
                // D1/E6 语义（与 eval 路径 `Env::define` 对齐，T1 定理）：
                // define 只新增绑定——同层已存在报错而非静默覆盖。
                if globals.contains_key(&sym) {
                    return Err(VmError::new("重复定义变量", span));
                }
                globals.insert(sym, v);
            }
            Op::LoadCaptured(i) => {
                let f = &frames[frame_idx];
                let cell = f
                    .captures
                    .get(*i as usize)
                    .ok_or_else(|| VmError::new(format!("捕获槽 {} 越界", i), span))?;
                push!(cell.get().clone());
            }
            Op::StoreCaptured(i) => {
                let v = pop!();
                let f = &frames[frame_idx];
                let cell = Rc::clone(
                    f.captures
                        .get(*i as usize)
                        .ok_or_else(|| VmError::new(format!("捕获槽 {} 越界", i), span))?,
                );
                // GcCell::set：写路径同步维护堆根性摘要（TD-023）
                cell.set(v);
            }
            Op::Jump(t) => {
                pc = *t;
            }
            Op::JumpIfFalse(t) => {
                let c = pop!();
                // TD-018：消息经 messages 单源构造（与 eval if 臂同文）
                match c {
                    Value::Bool(b) => {
                        if !b {
                            pc = *t;
                        }
                    }
                    other => {
                        return Err(VmError::new(
                            crate::messages::err_if_cond_bool(other.type_name()),
                            span,
                        ))
                    }
                }
            }
            Op::Closure {
                proto: p,
                n_captures,
            } => {
                // 捕获源描述符直取：与外围帧**共享**单元格（可变捕获语义）
                let target = &program.protos[*p as usize];
                if target.capture_sources.len() != *n_captures as usize {
                    return Err(VmError::new(
                        "CLOSURE 捕获描述符数量不匹配（编译器不变式破坏）",
                        span,
                    ));
                }
                let f = &frames[frame_idx];
                let mut captures: Vec<Rc<GcCell>> = Vec::with_capacity(*n_captures as usize);
                for src in &target.capture_sources {
                    let cell = match src {
                        kerf_compiler::CaptureSource::Local(i) => Rc::clone(
                            f.locals
                                .get(*i as usize)
                                .ok_or_else(|| VmError::new("捕获源局部槽越界", span))?,
                        ),
                        kerf_compiler::CaptureSource::Captured(i) => Rc::clone(
                            f.captures
                                .get(*i as usize)
                                .ok_or_else(|| VmError::new("捕获源捕获槽越界", span))?,
                        ),
                    };
                    captures.push(cell);
                }
                push!(Value::Closure(Rc::new(ClosureValue::Bytecode {
                    proto: *p,
                    captures,
                })));
            }
            Op::Call(n) => {
                let n = *n as usize;
                // 栈布局：[fn, arg0..arg n-1]（06 §2 A1 求值顺序契约：
                // 函数先求值入栈，参数从左到右——与 eval 路径一致）
                if stack.len() < n + 1 {
                    return Err(VmError::new("CALL 栈深度不足", span));
                }
                let mut args: Vec<Value> = Vec::with_capacity(n);
                for _ in 0..n {
                    args.push(stack.pop().expect("上方已检查"));
                }
                args.reverse(); // arg0 在局部槽 0
                let callee = stack.pop().expect("上方已检查");
                match callee {
                    Value::Builtin(b) => match b.call(heap, args) {
                        Ok(v) => push!(v),
                        Err(e) => return Err(VmError::new(e.message, span)),
                    },
                    Value::Closure(rc) if matches!(rc.as_ref(), ClosureValue::Bytecode { .. }) => {
                        let (p, captures) = match rc.as_ref() {
                            ClosureValue::Bytecode { proto, captures } => {
                                (*proto, captures.clone())
                            }
                            _ => unreachable!("上方已窄化"),
                        };
                        let target = &program.protos[p as usize];
                        if target.params.len() != args.len() {
                            return Err(VmError::new(
                                format!(
                                    "过程参数数量不匹配：期望 {} 实际 {}",
                                    target.params.len(),
                                    args.len()
                                ),
                                span,
                            ));
                        }
                        if frames.len() >= MAX_FRAMES {
                            return Err(VmError::new(
                                format!("调用帧超过上限 {}（失控递归）", MAX_FRAMES),
                                span,
                            ));
                        }
                        // 参数入单元格（捕获共享语义的帧基础；GcCell 承载
                        // 堆根性摘要标志，TD-023）
                        let local_cells: Vec<Rc<GcCell>> =
                            args.into_iter().map(|v| Rc::new(GcCell::new(v))).collect();
                        let frame = Frame {
                            ret_proto: proto_idx,
                            ret_pc: pc,
                            proto: p as usize,
                            locals: local_cells,
                            captures,
                            ext: FrameExt::new(),
                            call_span: span,
                        };
                        let _ = &rc;
                        frames.push(frame);
                        pc = 0;
                    }
                    Value::Closure(rc) if matches!(rc.as_ref(), ClosureValue::Eval { .. }) => {
                        return Err(VmError::new(
                            "跨路径闭包调用不支持（eval 闭包不可在 VM 中调用——双路径用于互查）",
                            span,
                        ))
                    }
                    // r25/42-f（D4）：resume = continuation 值的调用形态
                    // ——走 Call 通道（脱糖后与普通调用同一路径分派）。
                    // 控制转移语义：当前帧（H 执行帧）拆
                    // 除——handler 体内 κ 调用之后的代码永不执行（B 恢复
                    // 链自身的返回目标承接返回）
                    Value::Continuation(rc) => {
                        if args.len() != 1 {
                            return Err(VmError::new_code(
                                9,
                                crate::messages::err_resume_arity(args.len()),
                                span,
                            ));
                        }
                        let v = args.into_iter().next().expect("上方已检查");
                        let _h = frames.pop();
                        pc = resume_continuation(&rc, v, program, frames, &mut stack, span)?;
                    }
                    other => {
                        return Err(VmError::new(
                            format!("不可调用的值：{}", other.type_name()),
                            span,
                        ))
                    }
                }
            }
            Op::TailCall(n) => {
                // TD-022/H2——TCO 帧复用：语义同 Call，但当前帧拆除、被调方
                // 返回直达当前帧的调用者（帧数净零——尾递归 O(1) 帧）。
                // 自举 Reader/Expander 的尾调用链主循环（10^5 字符级源）
                // 与用户尾递归程序由此解除帧消耗（TD-022 兑现）。
                let n = *n as usize;
                if stack.len() < n + 1 {
                    return Err(VmError::new("TAIL_CALL 栈深度不足", span));
                }
                let mut args: Vec<Value> = Vec::with_capacity(n);
                for _ in 0..n {
                    args.push(stack.pop().expect("上方已检查"));
                }
                args.reverse(); // arg0 在局部槽 0
                let callee = stack.pop().expect("上方已检查");
                match callee {
                    // 内建函数尾调用 = 计算结果后立即执行隐式 RET（等价语义）
                    Value::Builtin(b) => match b.call(heap, args) {
                        Ok(v) => {
                            if frames.len() <= 1 {
                                return Ok(v);
                            }
                            let frame = frames.pop().expect("上方已检查");
                            pc = frame.ret_pc;
                            push!(v);
                        }
                        Err(e) => return Err(VmError::new(e.message, span)),
                    },
                    Value::Closure(rc) if matches!(rc.as_ref(), ClosureValue::Bytecode { .. }) => {
                        let (p, captures) = match rc.as_ref() {
                            ClosureValue::Bytecode { proto, captures } => {
                                (*proto, captures.clone())
                            }
                            _ => unreachable!("上方已窄化"),
                        };
                        let target = &program.protos[p as usize];
                        if target.params.len() != args.len() {
                            return Err(VmError::new(
                                format!(
                                    "过程参数数量不匹配：期望 {} 实际 {}",
                                    target.params.len(),
                                    args.len()
                                ),
                                span,
                            ));
                        }
                        // 帧替换（净零）：拆除当前帧，继承其返回地址——被调方
                        // RET 直达当前帧的调用者。MAX_FRAMES 不检查（帧数
                        // 恒定）；失控尾循环由指令预算护栏兜底。
                        let local_cells: Vec<Rc<GcCell>> =
                            args.into_iter().map(|v| Rc::new(GcCell::new(v))).collect();
                        let old = frames.pop().expect("尾调用帧存在");
                        let frame = Frame {
                            ret_proto: old.ret_proto,
                            ret_pc: old.ret_pc,
                            proto: p as usize,
                            locals: local_cells,
                            captures,
                            ext: FrameExt::new(),
                            call_span: span,
                        };
                        frames.push(frame);
                        pc = 0;
                    }
                    Value::Closure(rc) if matches!(rc.as_ref(), ClosureValue::Eval { .. }) => {
                        return Err(VmError::new(
                            "跨路径闭包调用不支持（eval 闭包不可在 VM 中调用——双路径用于互查）",
                            span,
                        ))
                    }
                    // r25/42-f：尾位 resume——当前帧拆除（控制已转移：
                    // 恢复的挂起帧链自身携带完整返回链，被拆帧的返回点
                    // 不再被引用——同 TailCall 帧替换语义）
                    Value::Continuation(rc) => {
                        if args.len() != 1 {
                            return Err(VmError::new_code(
                                9,
                                crate::messages::err_resume_arity(args.len()),
                                span,
                            ));
                        }
                        let v = args.into_iter().next().expect("上方已检查");
                        let _old = frames.pop();
                        pc = resume_continuation(&rc, v, program, frames, &mut stack, span)?;
                    }
                    other => {
                        return Err(VmError::new(
                            format!("不可调用的值：{}", other.type_name()),
                            span,
                        ))
                    }
                }
            }
            Op::Ret => {
                let v = pop!();
                if frames.len() <= 1 {
                    // 底帧 RET = 函数入口调用的返回（call_closure 语义）。
                    // run_program 主原型以 Halt 终止（编译器不变式），不经
                    // 此路径——主帧出现 RET 的旧行为是显式报错；本入口引入
                    // 后，底帧 RET 成为合法的程序化返回点。
                    return Ok(v);
                }
                let frame = frames.pop().expect("上方已检查");
                pc = frame.ret_pc;
                push!(v);
            }
            Op::Add => binop!(num_add),
            Op::Sub => binop!(num_sub),
            Op::Mul => binop!(num_mul),
            Op::Div => binop!(num_div),
            Op::Mod => binop!(num_mod),
            Op::NumLt => binop!(num_cmp(|o| o == std::cmp::Ordering::Less)),
            Op::NumGt => binop!(num_cmp(|o| o == std::cmp::Ordering::Greater)),
            Op::NumLe => binop!(num_cmp(|o| o != std::cmp::Ordering::Greater)),
            Op::NumGe => binop!(num_cmp(|o| o != std::cmp::Ordering::Less)),
            Op::NumEq => binop!(num_cmp(|o| o == std::cmp::Ordering::Equal)),
            Op::Eq => {
                let (a, b) = pop2!();
                push!(Value::Bool(a.eq_value(&b)));
            }
            Op::Not => {
                let v = pop!();
                match v {
                    Value::Bool(b) => push!(Value::Bool(!b)),
                    other => {
                        return Err(VmError::new(
                            crate::messages::err_not_bool(other.type_name()),
                            span,
                        ))
                    }
                }
            }
            Op::MakePair => {
                let (a, b) = pop2!();
                let ra = box_value(&a, heap);
                let rb = box_value(&b, heap);
                let r = heap.alloc_pair(ra, rb);
                push!(Value::Pair(r));
            }
            Op::Car => {
                let v = pop!();
                match v {
                    Value::Pair(r) => match heap.get_pair(r) {
                        Some((car, _)) => push!(unbox_slot(car, heap)),
                        None => return Err(VmError::new("car 应用于非序对堆槽", span)),
                    },
                    other => {
                        return Err(VmError::new(
                            crate::messages::err_pair_op("car", other.type_name()),
                            span,
                        ))
                    }
                }
            }
            Op::Cdr => {
                let v = pop!();
                match v {
                    Value::Pair(r) => match heap.get_pair(r) {
                        Some((_, cdr)) => push!(unbox_slot(cdr, heap)),
                        None => return Err(VmError::new("cdr 应用于非序对堆槽", span)),
                    },
                    other => {
                        return Err(VmError::new(
                            crate::messages::err_pair_op("cdr", other.type_name()),
                            span,
                        ))
                    }
                }
            }
            Op::IsNull => {
                let v = pop!();
                push!(Value::Bool(matches!(v, Value::Nil)));
            }
            Op::IsPair => {
                let v = pop!();
                push!(Value::Bool(matches!(v, Value::Pair(_))));
            }
            Op::IsInt => {
                let v = pop!();
                push!(Value::Bool(matches!(v, Value::Int(_))));
            }
            Op::IsBool => {
                let v = pop!();
                push!(Value::Bool(matches!(v, Value::Bool(_))));
            }
            Op::IsProcedure => {
                let v = pop!();
                push!(Value::Bool(matches!(
                    v,
                    Value::Closure(_) | Value::Builtin(_)
                )));
            }
            Op::Halt => {
                let v = stack.pop().unwrap_or(Value::Nil);
                let _ = instructions;
                let _ = frame_idx;
                return Ok(v);
            }
            // ---- 效应（r25/42-f——effect-language-design D6/D8 帧编排）----
            Op::InstallHandler {
                handler: hp,
                body: bp,
                tag: tk,
                trampoline: tp,
            } => {
                let hp = *hp as usize;
                let bp = *bp as usize;
                let tp = *tp as usize;
                if hp >= program.protos.len()
                    || bp >= program.protos.len()
                    || tp >= program.protos.len()
                {
                    return Err(VmError::new(
                        format!(
                            "INSTALL_HANDLER 原型索引越界：handler {} / body {} / trampoline {}（共 {}）",
                            hp,
                            bp,
                            tp,
                            program.protos.len()
                        ),
                        span,
                    ));
                }
                let tag_sym = match &program.consts[*tk as usize] {
                    BcConst::SymLit(s) => Rc::clone(s),
                    _ => return Err(VmError::new("INSTALL_HANDLER 标签操作数须为符号常量", span)),
                };
                if frames.len() + 2 > MAX_FRAMES {
                    return Err(VmError::new(
                        format!("调用帧超过上限 {}（失控递归）", MAX_FRAMES),
                        span,
                    ));
                }
                // 两原型捕获取自当前帧 F（与 Closure 同机制——共享单元格）
                let h_captures = take_frame_captures(
                    &program.protos[hp].capture_sources,
                    frames,
                    frame_idx,
                    span,
                )?;
                let b_captures = take_frame_captures(
                    &program.protos[bp].capture_sources,
                    frames,
                    frame_idx,
                    span,
                )?;
                // ① 压 handler 帧：proto = trampoline（code=[Ret] 哨兵），
                //    ret = (F 的原型, 本指令之后)；ext1 携分派数据
                let handler_data = Rc::new(HandlerFrame {
                    tag: tag_sym,
                    handler_proto: hp as u32,
                    captures: h_captures.clone(),
                    stack_watermark: stack.len(),
                });
                frames.push(Frame {
                    ret_proto: proto_idx,
                    ret_pc: pc,
                    proto: tp,
                    locals: Vec::new(),
                    captures: h_captures,
                    ext: FrameExt {
                        ext1: Some(handler_data),
                        ext2: None,
                        ext3: None,
                    },
                    call_span: span,
                });
                // ② 压 body thunk 帧：ret = (trampoline, 0)——body RET
                //    经 trampoline 弹 handler 帧回到 F 的 handle 后续
                //    （R11-return）；body 内尾调用拆 thunk 帧穿透（D8）
                frames.push(Frame {
                    ret_proto: tp,
                    ret_pc: 0,
                    proto: bp,
                    locals: Vec::new(),
                    captures: b_captures,
                    ext: FrameExt::new(),
                    call_span: span,
                });
                // ③ 转移执行 body（thunk 体 = lambda 体语义——尾位穿线）
                pc = 0;
            }
            Op::Perform => {
                // R10-perform：效应值出栈 → (tag . payload) 解构 → 最近
                // handler 帧扫描 → continuation 快照 → 拆帧到边界 →
                // handler 帧变形为 handler 体执行帧 → 栈截回水位
                let v = pop!();
                let (tag_val, payload) = match &v {
                    Value::Pair(r) => match heap.get_pair(*r) {
                        Some((car_slot, cdr_slot)) => {
                            let tag_v = unbox_slot(car_slot, heap);
                            let payload = unbox_slot(cdr_slot, heap);
                            match tag_v {
                                Value::Symbol(s) => (s, payload),
                                other => {
                                    return Err(VmError::new(
                                        format!(
                                            "perform 效应值 tag 位需要符号，实际 {}",
                                            other.type_name()
                                        ),
                                        span,
                                    ))
                                }
                            }
                        }
                        None => {
                            return Err(VmError::new(
                                "perform 效应值应为 (tag . payload) 点对",
                                span,
                            ))
                        }
                    },
                    other => {
                        return Err(VmError::new(
                            format!(
                                "perform 效应值需要 (tag . payload) 点对，实际 {}",
                                other.type_name()
                            ),
                            span,
                        ))
                    }
                };
                // 从帧栈顶向下扫描最近匹配 tag 的 handler 帧（D6）
                let mut handler_idx: Option<usize> = None;
                for (i, f) in frames.iter().enumerate().rev() {
                    if let Some(hd) = &f.ext.ext1 {
                        if hd.tag == tag_val {
                            handler_idx = Some(i);
                            break;
                        }
                    }
                }
                let hi = match handler_idx {
                    Some(i) => i,
                    None => {
                        return Err(VmError::new_code(
                            7,
                            crate::messages::err_effect_unhandled(&tag_val),
                            span,
                        ))
                    }
                };
                // continuation 快照：**含 handler 帧本身的挂起链**
                // （handler/trampoline 帧 = B 链的返回目标——resume 时
                // 必须恢复，否则 B 链 RET 后的帧序断裂：其返回点
                // (trampoline, 0) 无帧承接）；完整数据栈 + 恢复点。
                // 快照的 handler 帧 ext1 清除——浅处理（D2）：恢复后的
                // perform 不回同一 handler（再匹配需外层嵌套 handle）
                let mut snap = frames[hi..].to_vec();
                snap[0].ext.ext1 = None;
                let cont = Rc::new(crate::value::ContinuationValue {
                    frames: snap,
                    stack: stack.clone(),
                    resume_pc: pc,
                    resume_proto: frames.last().map(|f| f.proto as u32).unwrap_or(0),
                    consumed: std::cell::Cell::new(false),
                    first_resume_span: std::cell::Cell::new(kerf_span::Span::dummy()),
                });
                // handler 帧变形：trampoline → H 执行帧（locals = [payload,
                // κ]；ext1 清除——浅处理一次消费；ret 继承原 handler 帧
                // 的返回点 = F 的 handle 后续）
                let hf = frames[hi].clone();
                let hd = hf.ext.ext1.as_ref().expect("上方已匹配").clone();
                let h_frame = Frame {
                    ret_proto: hf.ret_proto,
                    ret_pc: hf.ret_pc,
                    proto: hd.handler_proto as usize,
                    locals: vec![
                        Rc::new(GcCell::new(payload)),
                        Rc::new(GcCell::new(Value::Continuation(Rc::clone(&cont)))),
                    ],
                    captures: hd.captures.clone(),
                    ext: FrameExt::new(),
                    call_span: hf.call_span,
                };
                frames.truncate(hi + 1);
                frames[hi] = h_frame;
                // 数据栈截回安装水位（挂起链中间值已存 continuation）
                let watermark = hd.stack_watermark;
                if stack.len() > watermark {
                    stack.truncate(watermark);
                }
                pc = 0;
            }
            // ---- FFI（r30/48-d——ffi-ownership-model §2/§8；语言面
            // 形式 Stage 3，编译臂不发射——操作码直接构造/测试面驱动）----
            Op::CallExternal { symbol, n_args } => {
                // 符号按名解析（extern 符号表——SymLit 常量承载符号名；
                // 扁平符号空间，不可被 define/set! 遮蔽）
                let name = match &program.consts[*symbol as usize] {
                    BcConst::SymLit(s) => s.clone(),
                    _ => {
                        return Err(VmError::new(
                            "CALL_EXTERNAL 操作数需要符号字面量（SymLit 常量）",
                            span,
                        ))
                    }
                };
                let n = *n_args as usize;
                if stack.len() < n {
                    return Err(VmError::new(
                        format!("数据栈下溢（FFI 实参不足：需要 {n}，实际 {}）", stack.len()),
                        span,
                    ));
                }
                // 实参正序切出（压栈序 = 正序——栈顶为末实参）
                let args: Vec<Value> = stack.split_off(stack.len() - n);
                // 窗口规程步 2-4（边界包装层——crate::ffi）
                let v = crate::ffi::call_external(externs, &name, &args, heap, span)?;
                push!(v);
            }
            Op::AllocExternal { size } => {
                // 外部域分配（不经过 GC；size=0 运行期纵深防御 = E0011）
                let v = crate::ffi::alloc_external(*size, span)?;
                push!(v);
            }
            Op::FreeExternal => {
                // 消费语义释放（槽级失效全局标记；判定序 = §5 非法迁移表）
                let v = pop!();
                let r = crate::ffi::free_external(&v, span)?;
                push!(r);
            }
        }

        // GC 安全点（§19.4 陷阱 2：只在指令边界触发；每 256 条指令轮询；
        // 低收益回收触发冷却退避——根集扫描成本摊销）
        if instructions.is_multiple_of(GC_POLL_INTERVAL)
            && gc_cooldown == 0
            && heap.should_collect()
        {
            // TD-023：根扫描缓冲复用（take/归还——零 API 变更面）
            let mut roots_buf = std::mem::take(&mut gc_root_scratch);
            let mut visited_buf = std::mem::take(&mut gc_visited_scratch);
            collect_roots_into(&stack, globals, frames, &mut roots_buf, &mut visited_buf);
            let roots = RootSet { refs: roots_buf };
            let stats = kerf_runtime::mark_sweep_cycle(heap, &roots);
            gc_root_scratch = roots.refs;
            gc_visited_scratch = visited_buf;
            // 回收收益 < 25% → 冷却（存活根主导，降低扫描频率）
            if stats.total_freed as usize * 4 < stats.slots {
                gc_cooldown = GC_COOLDOWN_MULTIPLIER;
            }
        } else if gc_cooldown > 0 {
            gc_cooldown = gc_cooldown.saturating_sub(1);
        }
    }
}

/// 常量 → 值。
fn const_to_value(c: &BcConst) -> Value {
    match c {
        BcConst::Int(v) => Value::Int(*v),
        BcConst::Float(v) => Value::Float(*v),
        BcConst::Str(s) => Value::Str(s.clone()),
        BcConst::Bool(b) => Value::Bool(*b),
        BcConst::Nil => Value::Nil,
        BcConst::SymLit(s) => Value::Symbol(s.clone()),
        BcConst::Symbol(_) => Value::Nil, // 符号常量仅作全局名索引（不会压栈）
    }
}

/// 值 → 堆引用（即时值装箱；闭包/内置经 Foreign 形态装箱——TD-010
/// r24 解决：原标记字符串占位 → 真装箱，解箱往返保持 Rc 恒等）。
/// 公共助手：driver 的 cons/list 等内置函数复用。
pub fn box_value(v: &Value, heap: &mut Heap) -> GcRef {
    match v {
        Value::Pair(r) => *r,
        Value::Str(s) => heap.alloc_boxed(BoxedInput::Str(s.clone())),
        Value::Symbol(s) => heap.alloc_symbol(s.clone()),
        Value::Int(i) => heap.alloc_boxed(BoxedInput::Int(*i)),
        Value::Float(f) => heap.alloc_boxed(BoxedInput::Float(*f)),
        Value::Bool(b) => heap.alloc_boxed(BoxedInput::Bool(*b)),
        Value::Nil => heap.alloc_boxed(BoxedInput::Nil),
        Value::Unit => heap.alloc_boxed(BoxedInput::Str(Rc::from("unit"))),
        // TD-010（r24）：闭包/内置装箱——ForeignBox 承载类型擦除的
        // Rc（共享 → eq? 按引用相等的往返恒等性）+ 追踪器（标记阶段
        // 枚举闭包捕获图的 Pair 子引用）
        Value::Closure(rc) => heap.alloc_foreign(ForeignBox {
            any: rc.clone(),
            tracer: trace_foreign_value,
        }),
        Value::Builtin(rc) => heap.alloc_foreign(ForeignBox {
            any: rc.clone(),
            tracer: trace_foreign_value,
        }),
        // r25/42-f（M5）：continuation 装箱——ForeignBox 承载 Rc（解箱
        // 往返恒等 → eq? 按引用）+ 专用追踪器（标记阶段枚举数据栈
        // 快照与帧链槽的堆子引用——同型先例 TD-010）
        Value::Continuation(rc) => heap.alloc_foreign(ForeignBox {
            any: rc.clone(),
            tracer: trace_foreign_continuation,
        }),
        // r30/48-d：FFI 令牌装箱——ForeignBox 承载 Rc<ExternalToken>
        // （解箱往返恒等——同型 TD-010）；追踪器 no-op（令牌载荷
        // 不参与 GC 可达性——ffi-ownership-model §7 裁定）
        Value::External(rc) => heap.alloc_foreign(ForeignBox {
            any: rc.clone(),
            tracer: trace_foreign_noop,
        }),
    }
}

/// FFI 令牌装箱追踪器（r30/48-d——no-op：令牌载荷不参与 GC 可达
/// 性，无堆子引用；ffi-ownership-model §7「Foreign 槽无 GC 子引用」）。
fn trace_foreign_noop(_any: &Rc<dyn Any>, _out: &mut Vec<GcRef>) {}

/// continuation 装箱追踪器（M5——堆内可达性：数据栈快照 + 帧链
/// locals/captures 的堆引用全量入 out；嵌套 κ 递归 trace_value_refs
/// ——visited 防环）。
fn trace_foreign_continuation(any: &Rc<dyn Any>, out: &mut Vec<GcRef>) {
    if let Some(cont) = any
        .as_ref()
        .downcast_ref::<crate::value::ContinuationValue>()
    {
        let mut visited: HashSet<usize> = HashSet::new();
        for sv in &cont.stack {
            trace_value_refs(sv, out, &mut visited);
        }
        for f in &cont.frames {
            for cell in &f.locals {
                if cell.has_heap() {
                    trace_value_refs(&cell.get(), out, &mut visited);
                }
            }
            for cell in &f.captures {
                if cell.has_heap() {
                    trace_value_refs(&cell.get(), out, &mut visited);
                }
            }
        }
    }
}

/// 堆槽 → 值（公共助手：driver 的 car/cdr 等内置函数复用）。
/// TD-010（r24）：Foreign 槽位 downcast 还原闭包/内置——Rc 共享
/// 装箱 → 解箱后 eq? 按引用相等（恒等性保持）。
pub fn unbox_slot(r: GcRef, heap: &Heap) -> Value {
    use kerf_runtime::ValueSlot;
    match heap.unbox(r) {
        Some(ValueSlot::Pair) => Value::Pair(r),
        Some(ValueSlot::Str(s)) => Value::Str(s),
        Some(ValueSlot::Int(i)) => Value::Int(i),
        Some(ValueSlot::Float(f)) => Value::Float(f),
        Some(ValueSlot::Bool(b)) => Value::Bool(b),
        Some(ValueSlot::Symbol(s)) => Value::Symbol(s),
        Some(ValueSlot::Nil) => Value::Nil,
        Some(ValueSlot::Foreign(b)) => {
            // downcast 消耗 Rc：先闭包后内置（装箱方只产生这两种形态）
            if let Ok(c) = b.any.clone().downcast::<ClosureValue>() {
                return Value::Closure(c);
            }
            if let Ok(f) = b.any.clone().downcast::<BuiltinFn>() {
                return Value::Builtin(f);
            }
            // r25/42-f：continuation 解箱（往返恒等——M5 存活验证面）
            if let Ok(k) = b.any.clone().downcast::<crate::value::ContinuationValue>() {
                return Value::Continuation(k);
            }
            // r30/48-d：FFI 令牌解箱（往返恒等——TD-010 同型）
            if let Ok(t) = b.any.clone().downcast::<crate::ffi::ExternalToken>() {
                return Value::External(t);
            }
            // 防御：未知 Foreign 载体（不发生于当前装箱方——上游不变式）
            Value::Nil
        }
        None => Value::Nil,
    }
}

/// 根集枚举（§19.4 根集完备性：VM 栈 + 帧局部 + 全局环境 + 闭包捕获；
/// foreign 根由 Heap 自持）。闭包捕获图经访问集去重（Rc 环防御）。
/// **TD-023 对症（r24/42-e）**：①缓冲由调用方持有跨周期复用
/// （visited/root——无分配化路径 B）；②非堆单元 O(1) 跳过
/// （GcCell::has_heap 摘要——深帧根扫描实测主导成本的对症）。
#[allow(clippy::too_many_arguments)]
fn collect_roots_into(
    stack: &[Value],
    globals: &HashMap<Symbol, Value>,
    frames: &[Frame],
    roots: &mut Vec<GcRef>,
    visited: &mut HashSet<usize>,
) {
    roots.clear();
    visited.clear();
    for v in stack.iter().chain(globals.values()) {
        collect_value_roots(v, roots, visited);
    }
    for f in frames {
        for cell in &f.locals {
            if cell.has_heap() {
                collect_value_roots(&cell.get(), roots, visited);
            }
        }
        for cell in &f.captures {
            if cell.has_heap() {
                collect_value_roots(&cell.get(), roots, visited);
            }
        }
    }
}

fn collect_value_roots(v: &Value, roots: &mut Vec<GcRef>, visited: &mut HashSet<usize>) {
    match v {
        Value::Pair(r) => roots.push(*r),
        Value::Closure(rc) => {
            let key = Rc::as_ptr(rc) as usize;
            if visited.insert(key) {
                if let ClosureValue::Bytecode { captures, .. } = rc.as_ref() {
                    for cell in captures {
                        if cell.has_heap() {
                            collect_value_roots(&cell.get(), roots, visited);
                        }
                    }
                }
            }
        }
        // r25/42-f（M5）：continuation = GC 第六来源（活跃 continuation
        // 帧——effect-language-design D12「GC 五来源扩展为六」）。帧链
        // 的 locals/captures 与数据栈快照均可能持堆引用；visited 防嵌套
        // continuation（κ 存于帧链槽内）与共享环
        Value::Continuation(rc) => {
            let key = Rc::as_ptr(rc) as usize;
            if visited.insert(key) {
                for sv in &rc.stack {
                    collect_value_roots(sv, roots, visited);
                }
                for f in &rc.frames {
                    for cell in &f.locals {
                        if cell.has_heap() {
                            collect_value_roots(&cell.get(), roots, visited);
                        }
                    }
                    for cell in &f.captures {
                        if cell.has_heap() {
                            collect_value_roots(&cell.get(), roots, visited);
                        }
                    }
                }
            }
        }
        // _ 臂理由：即时值（Unit/Nil/Bool/Int/Float/Str/Symbol——TD-002 装箱
        // 叶子 / Builtin）无堆子引用，无需入根集
        _ => {}
    }
}

/// 取原型捕获单元（`Closure`/`InstallHandler` 共用——从当前帧按
/// 捕获源描述符取共享单元格；r25/42-f 抽公用）。
fn take_frame_captures(
    capture_sources: &[kerf_compiler::CaptureSource],
    frames: &[Frame],
    frame_idx: usize,
    span: Span,
) -> Result<Vec<Rc<GcCell>>, VmError> {
    let f = &frames[frame_idx];
    let mut captures: Vec<Rc<GcCell>> = Vec::with_capacity(capture_sources.len());
    for src in capture_sources {
        let cell = match src {
            kerf_compiler::CaptureSource::Local(i) => Rc::clone(
                f.locals
                    .get(*i as usize)
                    .ok_or_else(|| VmError::new("捕获源局部槽越界", span))?,
            ),
            kerf_compiler::CaptureSource::Captured(i) => Rc::clone(
                f.captures
                    .get(*i as usize)
                    .ok_or_else(|| VmError::new("捕获源捕获槽越界", span))?,
            ),
        };
        captures.push(cell);
    }
    Ok(captures)
}

/// 恢复 continuation（R10-resume——`Call`/`TailCall` 的 Continuation
/// 臂共用；返回恢复点 pc）。
///
/// 线性唯一性（D3/E0008）：首次恢复置 `consumed` + 记录首恢位置；
/// 二次恢复报结构化码错误。帧链/数据栈整栈还原后压入实参（= perform
/// 表达式的值）。跨程序 continuation（原型索引越界）显式报错——
/// 与 `call_closure` 同型防御（§2.3-4 报错>静默）。
fn resume_continuation(
    rc: &Rc<crate::value::ContinuationValue>,
    arg: Value,
    program: &BcProgram,
    frames: &mut Vec<Frame>,
    stack: &mut Vec<Value>,
    span: Span,
) -> Result<u32, VmError> {
    if rc.consumed.get() {
        return Err(VmError::new_code(
            8,
            crate::messages::err_continuation_resumed_twice(&format!(
                "{}",
                rc.first_resume_span.get()
            )),
            span,
        ));
    }
    rc.consumed.set(true);
    rc.first_resume_span.set(span);
    // 越界防御：恢复帧链的全部原型索引须在本 program 内
    for f in &rc.frames {
        if f.proto >= program.protos.len() {
            return Err(VmError::new(
                format!(
                    "continuation 恢复帧原型索引越界：proto {} / 共 {}（跨程序 continuation 不可恢复）",
                    f.proto,
                    program.protos.len()
                ),
                span,
            ));
        }
    }
    if rc.resume_proto as usize >= program.protos.len() {
        return Err(VmError::new(
            format!(
                "continuation 恢复点原型越界：proto {} / 共 {}（跨程序 continuation 不可恢复）",
                rc.resume_proto,
                program.protos.len()
            ),
            span,
        ));
    }
    // 帧链恢复（挂起点帧 → handler 边界——按栈序 push）
    frames.extend(rc.frames.iter().cloned());
    // 数据栈整栈还原 + 实参即 perform 表达式的值
    stack.clear();
    stack.extend(rc.stack.iter().cloned());
    stack.push(arg);
    Ok(rc.resume_pc)
}

/// Foreign 装箱值的 GC 追踪器（TD-010）：标记阶段由 Heap::children
/// 调用——闭包捕获图内的 Pair 引用全量入 out（镜像
/// collect_value_roots 的遍历面；Rc 去重防环）；内置函数无 GC 可见
/// 捕获（no-op）。
fn trace_foreign_value(any: &Rc<dyn Any>, out: &mut Vec<GcRef>) {
    if let Some(ClosureValue::Bytecode { captures, .. }) =
        any.as_ref().downcast_ref::<ClosureValue>()
    {
        let mut visited: HashSet<usize> = HashSet::new();
        for cell in captures {
            if cell.has_heap() {
                trace_value_refs(&cell.get(), out, &mut visited);
            }
        }
    }
}

/// 追踪递归体（与 collect_value_roots 同语义：Pair 入列、闭包去重展开）。
fn trace_value_refs(v: &Value, out: &mut Vec<GcRef>, visited: &mut HashSet<usize>) {
    match v {
        Value::Pair(r) => out.push(*r),
        Value::Closure(rc) => {
            let key = Rc::as_ptr(rc) as usize;
            if visited.insert(key) {
                if let ClosureValue::Bytecode { captures, .. } = rc.as_ref() {
                    for cell in captures {
                        if cell.has_heap() {
                            trace_value_refs(&cell.get(), out, visited);
                        }
                    }
                }
            }
        }
        // _ 臂理由：即时值/内置无堆子引用
        _ => {}
    }
}

// ---- 算术与比较（Int/Float 混合提升；除零显式错误） ----

fn num_add(a: &Value, b: &Value) -> Result<Value, String> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x
            .checked_add(*y)
            .map(Value::Int)
            .ok_or_else(|| "整数加法溢出".to_string()),
        _ => Ok(Value::Float(num_f64(a)? + num_f64(b)?)),
    }
}

fn num_sub(a: &Value, b: &Value) -> Result<Value, String> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x
            .checked_sub(*y)
            .map(Value::Int)
            .ok_or_else(|| "整数减法溢出".to_string()),
        _ => Ok(Value::Float(num_f64(a)? - num_f64(b)?)),
    }
}

fn num_mul(a: &Value, b: &Value) -> Result<Value, String> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x
            .checked_mul(*y)
            .map(Value::Int)
            .ok_or_else(|| "整数乘法溢出".to_string()),
        _ => Ok(Value::Float(num_f64(a)? * num_f64(b)?)),
    }
}

fn num_div(a: &Value, b: &Value) -> Result<Value, String> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => {
            if *y == 0 {
                return Err("整数除零".to_string());
            }
            Ok(Value::Int(x / y))
        }
        _ => Ok(Value::Float(num_f64(a)? / num_f64(b)?)),
    }
}

fn num_mod(a: &Value, b: &Value) -> Result<Value, String> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => {
            if *y == 0 {
                return Err("整数取模除零".to_string());
            }
            Ok(Value::Int(x % y))
        }
        _ => Err(format!(
            "mod 需要 int，实际 {} / {}",
            a.type_name(),
            b.type_name()
        )),
    }
}

fn num_cmp(
    pred: fn(std::cmp::Ordering) -> bool,
) -> impl Fn(&Value, &Value) -> Result<Value, String> {
    move |a: &Value, b: &Value| {
        let (x, y) = (num_f64(a)?, num_f64(b)?);
        // 整数精确比较（浮点仅近似——Stage 0 数值塔：Int 与 Float 混合比较
        // 经 f64 提升；纯 Int 路径保持精确）
        let ord = match (a, b) {
            (Value::Int(i), Value::Int(j)) => i.cmp(j),
            // f64 无全序（NaN）——partial_cmp 失配时取 Greater 保底
            //（比较链非短路语义下不产生静默错误；NaN ≠ NaN 语义如实）
            _ => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Greater),
        };
        Ok(Value::Bool(pred(ord)))
    }
}

fn num_f64(v: &Value) -> Result<f64, String> {
    v.as_number()
        .ok_or_else(|| format!("算术操作数需要数值，实际 {}", v.type_name()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kerf_compiler::{compile_module, BcProto};
    use kerf_core::{CoreExpr, LiteralValue};
    use kerf_runtime::RuntimeError;
    use kerf_syntax::ScopeSet;
    use kerf_syntax::Symbol;
    use std::rc::Rc as StdRc;

    fn compile_and_run(exprs: &[StdRc<CoreExpr>]) -> Result<Value, VmError> {
        let program = compile_module(exprs).expect("编译失败");
        let mut globals = HashMap::new();
        let mut heap = Heap::new();
        run_program(&program, &mut globals, &mut heap)
    }

    fn lit(v: i64) -> StdRc<CoreExpr> {
        StdRc::new(CoreExpr::Literal {
            value: LiteralValue::Int(v),
            span: Span::dummy(),
        })
    }

    fn plus_builtin() -> Rc<crate::value::BuiltinFn> {
        crate::value::BuiltinFn::new("+", |_, args| {
            let mut acc = 0i64;
            for a in &args {
                acc += a.as_int().ok_or_else(|| RuntimeError::new("需要 int"))?;
            }
            Ok(Value::Int(acc))
        })
    }

    /// 手写字节码（ISA 级测试——§8.12 操作码分组的行为验证）。
    fn run_handwritten(code: Vec<Op>, consts: Vec<BcConst>) -> Result<Value, VmError> {
        let program = BcProgram {
            protos: vec![BcProto {
                name: Symbol(u32::MAX - 1),
                params: vec![],
                n_locals: 0,
                capture_names: vec![],
                capture_sources: vec![],
                code,
                debug_spans: vec![Span::dummy(); 8],
                free_vars: vec![],
            }],
            consts,
            entry: 0,
            global_refs: vec![],
            module_name: None,
        };
        let mut globals = HashMap::new();
        let mut heap = Heap::new();
        run_program(&program, &mut globals, &mut heap)
    }

    #[test]
    fn call_closure_invokes_function_with_args() {
        // (define (add x y) ...builtin +...) → run_program 加载全局 →
        // call_closure 以参数进入函数入口帧 → 底帧 RET 返回值（B3 宿主调用）
        let body = StdRc::new(CoreExpr::Apply {
            fn_expr: StdRc::new(CoreExpr::Var {
                name: Symbol(100),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            args: vec![
                StdRc::new(CoreExpr::Var {
                    name: Symbol(0),
                    scopes: ScopeSet::new(),
                    span: Span::dummy(),
                }),
                StdRc::new(CoreExpr::Var {
                    name: Symbol(1),
                    scopes: ScopeSet::new(),
                    span: Span::dummy(),
                }),
            ],
            span: Span::dummy(),
        });
        let lam = StdRc::new(CoreExpr::Fn {
            params: vec![Symbol(0), Symbol(1)],
            param_scopes: vec![ScopeSet::new()],
            body,
            span: Span::dummy(),
        });
        let def = StdRc::new(CoreExpr::Define {
            name: Symbol(200),
            value: lam,
            span: Span::dummy(),
        });
        let program = compile_module(&[def]).unwrap();
        let mut globals = HashMap::new();
        globals.insert(Symbol(100), Value::Builtin(plus_builtin()));
        let mut heap = Heap::new();
        run_program(&program, &mut globals, &mut heap).unwrap();
        let add = globals.get(&Symbol(200)).cloned().unwrap();
        // 两种调用形态：多次调用互不影响（帧/堆独立入口）
        let mut heap = Heap::new();
        let r = call_closure(
            &program,
            &mut globals,
            &mut heap,
            &add,
            vec![Value::Int(20), Value::Int(22)],
        )
        .unwrap();
        assert_eq!(r, Value::Int(42));
        let mut heap2 = Heap::new();
        let r2 = call_closure(
            &program,
            &mut globals,
            &mut heap2,
            &add,
            vec![Value::Int(1), Value::Int(2)],
        )
        .unwrap();
        assert_eq!(r2, Value::Int(3));
    }

    #[test]
    fn call_closure_arity_and_type_errors() {
        // 非闭包 → 类型错误；参数数量不匹配 → 元数错误（与 CALL 同口径）
        let program = compile_module(&[lit(1)]).unwrap();
        let mut globals = HashMap::new();
        let mut heap = Heap::new();
        let e =
            call_closure(&program, &mut globals, &mut heap, &Value::Int(1), vec![]).unwrap_err();
        assert!(e.message.contains("需要闭包"));
        let e2 = call_closure(
            &program,
            &mut globals,
            &mut heap,
            &Value::Builtin(plus_builtin()),
            vec![],
        )
        .unwrap_err();
        assert!(e2.message.contains("需要闭包"));
        // 闭包存在但参数数量不匹配
        let lam = StdRc::new(CoreExpr::Fn {
            params: vec![Symbol(0)],
            param_scopes: vec![ScopeSet::new()],
            body: StdRc::new(CoreExpr::Var {
                name: Symbol(0),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            span: Span::dummy(),
        });
        let def = StdRc::new(CoreExpr::Define {
            name: Symbol(200),
            value: lam,
            span: Span::dummy(),
        });
        let p2 = compile_module(&[def]).unwrap();
        let mut g2 = HashMap::new();
        let mut h2 = Heap::new();
        run_program(&p2, &mut g2, &mut h2).unwrap();
        let f = g2.get(&Symbol(200)).cloned().unwrap();
        let e3 = call_closure(
            &p2,
            &mut g2,
            &mut h2,
            &f,
            vec![Value::Int(1), Value::Int(2)],
        )
        .unwrap_err();
        assert!(e3.message.contains("参数数量不匹配"));
    }

    #[test]
    fn call_closure_cross_program_proto_rejected() {
        // 跨程序闭包：原型索引指向别的 program → 显式拒绝（P3 否决的运行时面）
        let lam = StdRc::new(CoreExpr::Fn {
            params: vec![Symbol(0)],
            param_scopes: vec![ScopeSet::new()],
            body: StdRc::new(CoreExpr::Var {
                name: Symbol(0),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            span: Span::dummy(),
        });
        let def = StdRc::new(CoreExpr::Define {
            name: Symbol(200),
            value: lam,
            span: Span::dummy(),
        });
        let program_a = compile_module(&[def]).unwrap();
        let mut globals = HashMap::new();
        let mut heap = Heap::new();
        run_program(&program_a, &mut globals, &mut heap).unwrap();
        let closure = globals.get(&Symbol(200)).cloned().unwrap();
        // program_b：空程序（不同原型表）——closure 的 proto 越界
        let program_b = compile_module(&[lit(0)]).unwrap();
        let e = call_closure(
            &program_b,
            &mut globals,
            &mut heap,
            &closure,
            vec![Value::Int(5)],
        )
        .unwrap_err();
        assert!(e.message.contains("跨程序闭包不可调用"));
    }

    #[test]
    fn call_closure_error_trace_collected() {
        // 被调闭包内部运行时错误 → 错误传播 + 调用帧追踪（与 run_program 同构）
        let body = StdRc::new(CoreExpr::Apply {
            fn_expr: StdRc::new(CoreExpr::Var {
                name: Symbol(0),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            args: vec![],
            span: Span::dummy(),
        });
        let lam = StdRc::new(CoreExpr::Fn {
            params: vec![Symbol(0)],
            param_scopes: vec![ScopeSet::new()],
            body,
            span: Span::dummy(),
        });
        let def = StdRc::new(CoreExpr::Define {
            name: Symbol(200),
            value: lam,
            span: Span::dummy(),
        });
        let program = compile_module(&[def]).unwrap();
        let mut globals = HashMap::new();
        let mut heap = Heap::new();
        run_program(&program, &mut globals, &mut heap).unwrap();
        let f = globals.get(&Symbol(200)).cloned().unwrap();
        // 传入非 builtin 值 → 调用时类型错误
        let e =
            call_closure(&program, &mut globals, &mut heap, &f, vec![Value::Int(7)]).unwrap_err();
        assert!(!e.message.is_empty());
    }

    #[test]
    fn run_literal_program() {
        assert_eq!(compile_and_run(&[lit(42)]).unwrap(), Value::Int(42));
    }

    #[test]
    fn run_if() {
        let e = StdRc::new(CoreExpr::If {
            cond: StdRc::new(CoreExpr::Literal {
                value: LiteralValue::Bool(true),
                span: Span::dummy(),
            }),
            then_branch: lit(1),
            else_branch: lit(2),
            span: Span::dummy(),
        });
        assert_eq!(compile_and_run(&[e]).unwrap(), Value::Int(1));
    }

    #[test]
    fn run_begin_sequence() {
        let e = StdRc::new(CoreExpr::Do {
            body: vec![lit(1), lit(2), lit(3)],
            span: Span::dummy(),
        });
        assert_eq!(compile_and_run(&[e]).unwrap(), Value::Int(3));
    }

    #[test]
    fn define_and_call_closure() {
        // (define f (fn (x) (+ x 1))) (f 41) → 42
        let plus = StdRc::new(CoreExpr::Var {
            name: Symbol(100),
            scopes: ScopeSet::new(),
            span: Span::dummy(),
        });
        let body = StdRc::new(CoreExpr::Apply {
            fn_expr: plus,
            args: vec![
                StdRc::new(CoreExpr::Var {
                    name: Symbol(0),
                    scopes: ScopeSet::new(),
                    span: Span::dummy(),
                }),
                lit(1),
            ],
            span: Span::dummy(),
        });
        let lam = StdRc::new(CoreExpr::Fn {
            params: vec![Symbol(0)],
            param_scopes: vec![ScopeSet::new()],
            body,
            span: Span::dummy(),
        });
        let def = StdRc::new(CoreExpr::Define {
            name: Symbol(200),
            value: lam,
            span: Span::dummy(),
        });
        let call = StdRc::new(CoreExpr::Apply {
            fn_expr: StdRc::new(CoreExpr::Var {
                name: Symbol(200),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            args: vec![lit(41)],
            span: Span::dummy(),
        });
        let program = compile_module(&[def, call]).unwrap();
        let mut globals = HashMap::new();
        globals.insert(Symbol(100), Value::Builtin(plus_builtin()));
        let mut heap = Heap::new();
        assert_eq!(
            run_program(&program, &mut globals, &mut heap).unwrap(),
            Value::Int(42)
        );
    }

    #[test]
    fn closure_captures_shared_mutation() {
        // (define (make-counter) ...)：闭包捕获的 assign 语义——两闭包共享捕获单元
        // 程序：((fn (n) (fn () n)) 5) → 闭包；此处验证捕获值经 LOAD_CAPTURED
        let inner = StdRc::new(CoreExpr::Fn {
            params: vec![],
            param_scopes: vec![ScopeSet::new()],
            body: StdRc::new(CoreExpr::Var {
                name: Symbol(0),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            span: Span::dummy(),
        });
        let outer = StdRc::new(CoreExpr::Fn {
            params: vec![Symbol(0)],
            param_scopes: vec![ScopeSet::new()],
            body: inner,
            span: Span::dummy(),
        });
        let make = StdRc::new(CoreExpr::Apply {
            fn_expr: outer,
            args: vec![lit(5)],
            span: Span::dummy(),
        });
        // ((...) ) 调用内部闭包 → 5
        let call = StdRc::new(CoreExpr::Apply {
            fn_expr: make,
            args: vec![],
            span: Span::dummy(),
        });
        assert_eq!(compile_and_run(&[call]).unwrap(), Value::Int(5));
    }

    #[test]
    fn isa_make_pair_car_cdr() {
        // PUSH 1, PUSH 2, MAKE_PAIR, CAR → 1（§8.12 数据构造组）
        let v = run_handwritten(
            vec![
                Op::PushConst(0),
                Op::PushConst(1),
                Op::MakePair,
                Op::Car,
                Op::Halt,
            ],
            vec![BcConst::Int(1), BcConst::Int(2)],
        )
        .unwrap();
        assert_eq!(v, Value::Int(1));
        // CDR → 2
        let v2 = run_handwritten(
            vec![
                Op::PushConst(0),
                Op::PushConst(1),
                Op::MakePair,
                Op::Cdr,
                Op::Halt,
            ],
            vec![BcConst::Int(1), BcConst::Int(2)],
        )
        .unwrap();
        assert_eq!(v2, Value::Int(2));
    }

    #[test]
    fn isa_predicates() {
        let v = run_handwritten(vec![Op::PushNil, Op::IsNull, Op::Halt], vec![]).unwrap();
        assert_eq!(v, Value::Bool(true));
        let v2 = run_handwritten(
            vec![
                Op::PushConst(0),
                Op::PushConst(1),
                Op::MakePair,
                Op::IsPair,
                Op::Halt,
            ],
            vec![BcConst::Int(1), BcConst::Int(2)],
        )
        .unwrap();
        assert_eq!(v2, Value::Bool(true));
    }

    #[test]
    fn isa_stack_ops() {
        // DUP + EQ：1, 1（DUP 后 EQ 同值）→ true
        let v = run_handwritten(
            vec![Op::PushConst(0), Op::Dup, Op::Eq, Op::Halt],
            vec![BcConst::Int(7)],
        )
        .unwrap();
        assert_eq!(v, Value::Bool(true));
        // SWAP：[2,1] → swap → [1,2]；SUB = a-b = 1-2 = -1
        let v2 = run_handwritten(
            vec![
                Op::PushConst(0),
                Op::PushConst(1),
                Op::Swap,
                Op::Sub,
                Op::Halt,
            ],
            vec![BcConst::Int(2), BcConst::Int(1)],
        )
        .unwrap();
        assert_eq!(v2, Value::Int(-1));
    }

    #[test]
    fn truthy_only_bool() {
        // (if 1 2 3)：非 bool 条件 → 显式类型错误（§19.5 陷阱 3）
        let e = StdRc::new(CoreExpr::If {
            cond: lit(1),
            then_branch: lit(2),
            else_branch: lit(3),
            span: Span::new(0, 0, 12),
        });
        let err = compile_and_run(&[e]).unwrap_err();
        assert!(err.message.contains("truthy"));
        assert_eq!(err.span, Span::new(0, 0, 12));
    }

    #[test]
    fn arity_mismatch_error() {
        // (define f (fn (x) x)) (f 1 2) → 参数数量不匹配
        let lam = StdRc::new(CoreExpr::Fn {
            params: vec![Symbol(0)],
            param_scopes: vec![ScopeSet::new()],
            body: StdRc::new(CoreExpr::Var {
                name: Symbol(0),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            span: Span::dummy(),
        });
        let def = StdRc::new(CoreExpr::Define {
            name: Symbol(200),
            value: lam,
            span: Span::dummy(),
        });
        let call = StdRc::new(CoreExpr::Apply {
            fn_expr: StdRc::new(CoreExpr::Var {
                name: Symbol(200),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            args: vec![lit(1), lit(2)],
            span: Span::dummy(),
        });
        let err = compile_and_run(&[def, call]).unwrap_err();
        assert!(err.message.contains("参数数量不匹配"));
    }

    #[test]
    fn div_by_zero_error() {
        // (define (f) (/ 1 0)) —— 经内置 /
        let div_builtin = crate::value::BuiltinFn::new("/", |_, args| {
            let (a, b) = (
                args[0]
                    .as_int()
                    .ok_or_else(|| RuntimeError::new("需要 int"))?,
                args[1]
                    .as_int()
                    .ok_or_else(|| RuntimeError::new("需要 int"))?,
            );
            if b == 0 {
                return Err(RuntimeError::new("整数除零"));
            }
            Ok(Value::Int(a / b))
        });
        let e = StdRc::new(CoreExpr::Apply {
            fn_expr: StdRc::new(CoreExpr::Var {
                name: Symbol(100),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            args: vec![lit(1), lit(0)],
            span: Span::dummy(),
        });
        let program = compile_module(&[e]).unwrap();
        let mut globals = HashMap::new();
        globals.insert(Symbol(100), Value::Builtin(div_builtin));
        let mut heap = Heap::new();
        let err = run_program(&program, &mut globals, &mut heap).unwrap_err();
        assert!(err.message.contains("除零"));
    }

    #[test]
    fn gc_safepoint_collects_garbage() {
        // 循环构造丢弃的序对：堆应保持有界（回收经指令边界安全点）
        // 程序：(do (quote (1)) (quote (1)) ... ) × 4000 —— 每次分配 3 槽
        let mut body: Vec<StdRc<CoreExpr>> = Vec::new();
        for _ in 0..4000 {
            body.push(StdRc::new(CoreExpr::Literal {
                value: LiteralValue::Pair(
                    StdRc::new(LiteralValue::Int(1)),
                    StdRc::new(LiteralValue::Nil),
                ),
                span: Span::dummy(),
            }));
        }
        let e = StdRc::new(CoreExpr::Do {
            body,
            span: Span::dummy(),
        });
        let program = compile_module(&[e]).unwrap();
        let mut globals = HashMap::new();
        let mut heap = Heap::with_threshold(512);
        run_program(&program, &mut globals, &mut heap).unwrap();
        // 4000 次分配 2 槽/次 = 8000 槽；GC 后（无根）应远小于总量
        assert!(
            heap.slot_count() < 8000,
            "GC 未生效：slots = {}",
            heap.slot_count()
        );
        assert!(heap.stats().collections > 0, "至少发生一次回收");
    }

    #[test]
    fn deep_recursion_bounded_by_frames_not_stack() {
        // 迭代式循环：深递归（1000 层）不溢出 Rust 栈
        // (define (loop n) (if (= n 0) 0 (loop (- n 1)))) (loop 1000)
        fn build(n: u32) -> StdRc<CoreExpr> {
            if n == 0 {
                return StdRc::new(CoreExpr::Literal {
                    value: LiteralValue::Int(0),
                    span: Span::dummy(),
                });
            }
            // (if (= n 0) 0 (loop (- n 1))) 简化为直构递归 Apply
            StdRc::new(CoreExpr::Apply {
                fn_expr: StdRc::new(CoreExpr::Var {
                    name: Symbol(200),
                    scopes: ScopeSet::new(),
                    span: Span::dummy(),
                }),
                args: vec![lit(n as i64 - 1)],
                span: Span::dummy(),
            })
        }
        let _ = build;
        // 直接构造递归：f = (fn (n) (if (= n 0) 0 (f (- n 1))))
        let eq_builtin = crate::value::BuiltinFn::new("=", |_, args| {
            Ok(Value::Bool(
                args.iter().all(|a| a.as_int() == args[0].as_int()),
            ))
        });
        let sub_builtin = crate::value::BuiltinFn::new("-", |_, args| {
            let a = args[0]
                .as_int()
                .ok_or_else(|| RuntimeError::new("需要 int"))?;
            let b = args.get(1).and_then(|v| v.as_int()).unwrap_or(0);
            Ok(Value::Int(a - b))
        });
        let test_expr = StdRc::new(CoreExpr::Apply {
            fn_expr: StdRc::new(CoreExpr::Var {
                name: Symbol(101),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            args: vec![StdRc::new(CoreExpr::Var {
                name: Symbol(0),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            })],
            span: Span::dummy(),
        });
        let recurse = StdRc::new(CoreExpr::Apply {
            fn_expr: StdRc::new(CoreExpr::Var {
                name: Symbol(200),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            args: vec![StdRc::new(CoreExpr::Apply {
                fn_expr: StdRc::new(CoreExpr::Var {
                    name: Symbol(102),
                    scopes: ScopeSet::new(),
                    span: Span::dummy(),
                }),
                args: vec![
                    StdRc::new(CoreExpr::Var {
                        name: Symbol(0),
                        scopes: ScopeSet::new(),
                        span: Span::dummy(),
                    }),
                    lit(1),
                ],
                span: Span::dummy(),
            })],
            span: Span::dummy(),
        });
        let body = StdRc::new(CoreExpr::If {
            cond: test_expr,
            then_branch: lit(0),
            else_branch: recurse,
            span: Span::dummy(),
        });
        let lam = StdRc::new(CoreExpr::Fn {
            params: vec![Symbol(0)],
            param_scopes: vec![ScopeSet::new()],
            body,
            span: Span::dummy(),
        });
        let def = StdRc::new(CoreExpr::Define {
            name: Symbol(200),
            value: lam,
            span: Span::dummy(),
        });
        let call = StdRc::new(CoreExpr::Apply {
            fn_expr: StdRc::new(CoreExpr::Var {
                name: Symbol(200),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            args: vec![lit(1000)],
            span: Span::dummy(),
        });
        let program = compile_module(&[def, call]).unwrap();
        let mut globals = HashMap::new();
        globals.insert(Symbol(101), Value::Builtin(eq_builtin));
        globals.insert(Symbol(102), Value::Builtin(sub_builtin));
        let mut heap = Heap::new();
        let v = run_program(&program, &mut globals, &mut heap).unwrap();
        assert_eq!(v, Value::Int(0));
    }

    #[test]
    fn type_error_carries_span_from_debug_table() {
        let e = StdRc::new(CoreExpr::Apply {
            fn_expr: StdRc::new(CoreExpr::Var {
                name: Symbol(100),
                scopes: ScopeSet::new(),
                span: Span::new(0, 10, 20),
            }),
            args: vec![lit(1)],
            span: Span::new(0, 0, 21),
        });
        let program = compile_module(&[e]).unwrap();
        let mut globals = HashMap::new();
        let bad = crate::value::BuiltinFn::new("bad", |_, _| Err(RuntimeError::new("故意失败")));
        globals.insert(Symbol(100), Value::Builtin(bad));
        let mut heap = Heap::new();
        let err = run_program(&program, &mut globals, &mut heap).unwrap_err();
        assert_eq!(err.message, "故意失败");
        assert_eq!(err.span, Span::new(0, 0, 21));
    }
}
