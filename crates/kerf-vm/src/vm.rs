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

/// 预留槽类型（§8.12 三扩展槽——Stage 0 空实现，格式冻结）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ContinuationSlot;

/// 预留槽类型（ext2：异常处理表）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HandlerTableSlot;

/// 预留槽类型（ext3：调试帧信息）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DebugFrameSlot;

/// 调用帧扩展（三槽冻结格式）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameExt {
    /// ext1：continuation / effect handler（Stage 0 空）。
    pub ext1: Option<ContinuationSlot>,
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
}

impl VmError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        VmError {
            message: message.into(),
            span,
            trace: Vec::new(),
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
pub fn run_program(
    program: &BcProgram,
    globals: &mut HashMap<Symbol, Value>,
    heap: &mut Heap,
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
    let outcome = execute(program, globals, heap, &mut frames, MAX_INSTRUCTIONS);
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
#[doc(hidden)]
pub fn run_program_with_budget(
    program: &BcProgram,
    globals: &mut HashMap<Symbol, Value>,
    heap: &mut Heap,
    max_instructions: u64,
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
    execute(program, globals, heap, &mut frames, max_instructions)
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
    let outcome = execute(program, globals, heap, &mut frames, MAX_INSTRUCTIONS);
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
        // _ 臂理由：即时值（Unit/Nil/Bool/Int/Float/Str/Builtin）无堆子引用，无需入根集
        _ => {}
    }
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
        let body = StdRc::new(CoreExpr::App {
            fn_expr: StdRc::new(CoreExpr::VarRef {
                name: Symbol(100),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            args: vec![
                StdRc::new(CoreExpr::VarRef {
                    name: Symbol(0),
                    scopes: ScopeSet::new(),
                    span: Span::dummy(),
                }),
                StdRc::new(CoreExpr::VarRef {
                    name: Symbol(1),
                    scopes: ScopeSet::new(),
                    span: Span::dummy(),
                }),
            ],
            span: Span::dummy(),
        });
        let lam = StdRc::new(CoreExpr::Lambda {
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
        let lam = StdRc::new(CoreExpr::Lambda {
            params: vec![Symbol(0)],
            param_scopes: vec![ScopeSet::new()],
            body: StdRc::new(CoreExpr::VarRef {
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
        let lam = StdRc::new(CoreExpr::Lambda {
            params: vec![Symbol(0)],
            param_scopes: vec![ScopeSet::new()],
            body: StdRc::new(CoreExpr::VarRef {
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
        let body = StdRc::new(CoreExpr::App {
            fn_expr: StdRc::new(CoreExpr::VarRef {
                name: Symbol(0),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            args: vec![],
            span: Span::dummy(),
        });
        let lam = StdRc::new(CoreExpr::Lambda {
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
        let e = StdRc::new(CoreExpr::Begin {
            body: vec![lit(1), lit(2), lit(3)],
            span: Span::dummy(),
        });
        assert_eq!(compile_and_run(&[e]).unwrap(), Value::Int(3));
    }

    #[test]
    fn define_and_call_closure() {
        // (define f (lambda (x) (+ x 1))) (f 41) → 42
        let plus = StdRc::new(CoreExpr::VarRef {
            name: Symbol(100),
            scopes: ScopeSet::new(),
            span: Span::dummy(),
        });
        let body = StdRc::new(CoreExpr::App {
            fn_expr: plus,
            args: vec![
                StdRc::new(CoreExpr::VarRef {
                    name: Symbol(0),
                    scopes: ScopeSet::new(),
                    span: Span::dummy(),
                }),
                lit(1),
            ],
            span: Span::dummy(),
        });
        let lam = StdRc::new(CoreExpr::Lambda {
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
        let call = StdRc::new(CoreExpr::App {
            fn_expr: StdRc::new(CoreExpr::VarRef {
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
        // (define (make-counter) ...)：闭包捕获的 set! 语义——两闭包共享捕获单元
        // 程序：((lambda (n) (lambda () n)) 5) → 闭包；此处验证捕获值经 LOAD_CAPTURED
        let inner = StdRc::new(CoreExpr::Lambda {
            params: vec![],
            param_scopes: vec![ScopeSet::new()],
            body: StdRc::new(CoreExpr::VarRef {
                name: Symbol(0),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            span: Span::dummy(),
        });
        let outer = StdRc::new(CoreExpr::Lambda {
            params: vec![Symbol(0)],
            param_scopes: vec![ScopeSet::new()],
            body: inner,
            span: Span::dummy(),
        });
        let make = StdRc::new(CoreExpr::App {
            fn_expr: outer,
            args: vec![lit(5)],
            span: Span::dummy(),
        });
        // ((...) ) 调用内部闭包 → 5
        let call = StdRc::new(CoreExpr::App {
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
        // (define f (lambda (x) x)) (f 1 2) → 参数数量不匹配
        let lam = StdRc::new(CoreExpr::Lambda {
            params: vec![Symbol(0)],
            param_scopes: vec![ScopeSet::new()],
            body: StdRc::new(CoreExpr::VarRef {
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
        let call = StdRc::new(CoreExpr::App {
            fn_expr: StdRc::new(CoreExpr::VarRef {
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
        let e = StdRc::new(CoreExpr::App {
            fn_expr: StdRc::new(CoreExpr::VarRef {
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
        // 程序：(begin (quote (1)) (quote (1)) ... ) × 4000 —— 每次分配 3 槽
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
        let e = StdRc::new(CoreExpr::Begin {
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
            // (if (= n 0) 0 (loop (- n 1))) 简化为直构递归 App
            StdRc::new(CoreExpr::App {
                fn_expr: StdRc::new(CoreExpr::VarRef {
                    name: Symbol(200),
                    scopes: ScopeSet::new(),
                    span: Span::dummy(),
                }),
                args: vec![lit(n as i64 - 1)],
                span: Span::dummy(),
            })
        }
        let _ = build;
        // 直接构造递归：f = (lambda (n) (if (= n 0) 0 (f (- n 1))))
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
        let test_expr = StdRc::new(CoreExpr::App {
            fn_expr: StdRc::new(CoreExpr::VarRef {
                name: Symbol(101),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            args: vec![StdRc::new(CoreExpr::VarRef {
                name: Symbol(0),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            })],
            span: Span::dummy(),
        });
        let recurse = StdRc::new(CoreExpr::App {
            fn_expr: StdRc::new(CoreExpr::VarRef {
                name: Symbol(200),
                scopes: ScopeSet::new(),
                span: Span::dummy(),
            }),
            args: vec![StdRc::new(CoreExpr::App {
                fn_expr: StdRc::new(CoreExpr::VarRef {
                    name: Symbol(102),
                    scopes: ScopeSet::new(),
                    span: Span::dummy(),
                }),
                args: vec![
                    StdRc::new(CoreExpr::VarRef {
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
        let lam = StdRc::new(CoreExpr::Lambda {
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
        let call = StdRc::new(CoreExpr::App {
            fn_expr: StdRc::new(CoreExpr::VarRef {
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
        let e = StdRc::new(CoreExpr::App {
            fn_expr: StdRc::new(CoreExpr::VarRef {
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
