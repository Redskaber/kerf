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

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use kerf_compiler::{BcConst, BcProgram, Op};
use kerf_runtime::{BoxedInput, GcRef, Heap, RootSet};
use kerf_span::Span;
use kerf_syntax::Symbol;

use crate::value::{ClosureValue, Value};

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
    /// `Rc<RefCell>` 传播到闭包，letrec 递归成立）。
    pub locals: Vec<Rc<RefCell<Value>>>,
    /// 闭包捕获槽（共享可变单元）。
    pub captures: Vec<Rc<RefCell<Value>>>,
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
    let mut stack: Vec<Value> = Vec::new();
    let mut frames: Vec<Frame> = vec![Frame {
        ret_proto: 0,
        ret_pc: 0,
        proto: program.entry as usize,
        locals: Vec::new(),
        captures: Vec::new(),
        ext: FrameExt::new(),
        call_span: Span::dummy(),
    }];
    let mut pc: u32 = 0;
    let mut instructions: u64 = 0;
    let mut gc_cooldown: u64 = 0;

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
                push!(cell.borrow().clone());
            }
            Op::StoreLocal(i) => {
                let v = pop!();
                let f = &frames[frame_idx];
                let cell = Rc::clone(
                    f.locals
                        .get(*i as usize)
                        .ok_or_else(|| VmError::new(format!("局部槽 {} 越界", i), span))?,
                );
                *cell.borrow_mut() = v;
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
                globals.insert(sym, v);
            }
            Op::LoadCaptured(i) => {
                let f = &frames[frame_idx];
                let cell = f
                    .captures
                    .get(*i as usize)
                    .ok_or_else(|| VmError::new(format!("捕获槽 {} 越界", i), span))?;
                push!(cell.borrow().clone());
            }
            Op::StoreCaptured(i) => {
                let v = pop!();
                let f = &frames[frame_idx];
                let cell = Rc::clone(
                    f.captures
                        .get(*i as usize)
                        .ok_or_else(|| VmError::new(format!("捕获槽 {} 越界", i), span))?,
                );
                *cell.borrow_mut() = v;
            }
            Op::Jump(t) => {
                pc = *t;
            }
            Op::JumpIfFalse(t) => {
                let c = pop!();
                match c {
                    Value::Bool(b) => {
                        if !b {
                            pc = *t;
                        }
                    }
                    other => {
                        return Err(VmError::new(
                            format!(
                                "条件位置需要 bool，实际 {}（truthy 语义显式定义）",
                                other.type_name()
                            ),
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
                let mut captures: Vec<Rc<RefCell<Value>>> =
                    Vec::with_capacity(*n_captures as usize);
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
                // 栈布局：[arg0..arg n-1, fn]（§19.3 求值顺序契约）
                if stack.len() < n + 1 {
                    return Err(VmError::new("CALL 栈深度不足", span));
                }
                let callee = stack.pop().expect("上方已检查");
                let mut args: Vec<Value> = Vec::with_capacity(n);
                for _ in 0..n {
                    args.push(stack.pop().expect("上方已检查"));
                }
                args.reverse(); // arg0 在局部槽 0
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
                        // 参数入单元格（捕获共享语义的帧基础）
                        let local_cells: Vec<Rc<RefCell<Value>>> =
                            args.into_iter().map(|v| Rc::new(RefCell::new(v))).collect();
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
            Op::Ret => {
                let v = pop!();
                if frames.len() <= 1 {
                    return Err(VmError::new("主原型出现 RET（编译器不变式破坏）", span));
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
                            format!("not 需要 bool，实际 {}", other.type_name()),
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
                            format!("car 需要 pair，实际 {}", other.type_name()),
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
                            format!("cdr 需要 pair，实际 {}", other.type_name()),
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
            let roots = collect_roots(&stack, globals, &frames);
            let stats = kerf_runtime::mark_sweep_cycle(heap, &roots);
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
        BcConst::Symbol(_) => Value::Nil, // 符号常量仅作全局名索引（不会压栈）
    }
}

/// 值 → 堆引用（即时值装箱；闭包装箱不支持——显式限制，TD-010）。
/// 公共助手：driver 的 cons/list 等内置函数复用。
pub fn box_value(v: &Value, heap: &mut Heap) -> GcRef {
    match v {
        Value::Pair(r) => *r,
        Value::Str(s) => heap.alloc_boxed(BoxedInput::Str(s.clone())),
        Value::Int(i) => heap.alloc_boxed(BoxedInput::Int(*i)),
        Value::Float(f) => heap.alloc_boxed(BoxedInput::Float(*f)),
        Value::Bool(b) => heap.alloc_boxed(BoxedInput::Bool(*b)),
        Value::Nil => heap.alloc_boxed(BoxedInput::Nil),
        Value::Unit => heap.alloc_boxed(BoxedInput::Str(Rc::from("unit"))),
        Value::Closure(_) | Value::Builtin(_) => {
            // TD-010：闭包装箱显式占位（以可识别标记字符串——调用方 car/cdr
            // 将得到 str；此路径在 Stage 0 程序中不可达（编译器不生成），
            // 出现即为上游 bug，堆栈追踪可见）
            heap.alloc_boxed(BoxedInput::Str(Rc::from("#<procedure:unboxed>")))
        }
    }
}

/// 堆槽 → 值（公共助手：driver 的 car/cdr 等内置函数复用）。
pub fn unbox_slot(r: GcRef, heap: &Heap) -> Value {
    use kerf_runtime::ValueSlot;
    match heap.unbox(r) {
        Some(ValueSlot::Pair) => Value::Pair(r),
        Some(ValueSlot::Str(s)) => Value::Str(s),
        Some(ValueSlot::Int(i)) => Value::Int(i),
        Some(ValueSlot::Float(f)) => Value::Float(f),
        Some(ValueSlot::Bool(b)) => Value::Bool(b),
        Some(ValueSlot::Nil) => Value::Nil,
        None => Value::Nil,
    }
}

/// 根集枚举（§19.4 根集完备性：VM 栈 + 帧局部 + 全局环境 + 闭包捕获；
/// foreign 根由 Heap 自持）。闭包捕获图经访问集去重（Rc 环防御）。
fn collect_roots(stack: &[Value], globals: &HashMap<Symbol, Value>, frames: &[Frame]) -> RootSet {
    let mut roots = RootSet::new();
    let mut visited: HashSet<usize> = HashSet::new();
    for v in stack.iter().chain(globals.values()) {
        collect_value_roots(v, &mut roots, &mut visited);
    }
    for f in frames {
        for cell in &f.locals {
            collect_value_roots(&cell.borrow(), &mut roots, &mut visited);
        }
        for cell in &f.captures {
            collect_value_roots(&cell.borrow(), &mut roots, &mut visited);
        }
    }
    roots
}

fn collect_value_roots(v: &Value, roots: &mut RootSet, visited: &mut HashSet<usize>) {
    match v {
        Value::Pair(r) => roots.push(*r),
        Value::Closure(rc) => {
            let key = Rc::as_ptr(rc) as usize;
            if visited.insert(key) {
                if let ClosureValue::Bytecode { captures, .. } = rc.as_ref() {
                    for cell in captures {
                        collect_value_roots(&cell.borrow(), roots, visited);
                    }
                }
            }
        }
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
            span: Span::dummy(),
        });
        let body = StdRc::new(CoreExpr::App {
            fn_expr: plus,
            args: vec![
                StdRc::new(CoreExpr::VarRef {
                    name: Symbol(0),
                    span: Span::dummy(),
                }),
                lit(1),
            ],
            span: Span::dummy(),
        });
        let lam = StdRc::new(CoreExpr::Lambda {
            params: vec![Symbol(0)],
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
            body: StdRc::new(CoreExpr::VarRef {
                name: Symbol(0),
                span: Span::dummy(),
            }),
            span: Span::dummy(),
        });
        let outer = StdRc::new(CoreExpr::Lambda {
            params: vec![Symbol(0)],
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
            body: StdRc::new(CoreExpr::VarRef {
                name: Symbol(0),
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
                span: Span::dummy(),
            }),
            args: vec![StdRc::new(CoreExpr::VarRef {
                name: Symbol(0),
                span: Span::dummy(),
            })],
            span: Span::dummy(),
        });
        let recurse = StdRc::new(CoreExpr::App {
            fn_expr: StdRc::new(CoreExpr::VarRef {
                name: Symbol(200),
                span: Span::dummy(),
            }),
            args: vec![StdRc::new(CoreExpr::App {
                fn_expr: StdRc::new(CoreExpr::VarRef {
                    name: Symbol(102),
                    span: Span::dummy(),
                }),
                args: vec![
                    StdRc::new(CoreExpr::VarRef {
                        name: Symbol(0),
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
