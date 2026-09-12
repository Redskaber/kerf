//! 编译器实现（stage0.md §19.3 代码生成骨架的落地实现）。
//!
//! `CoreExpr` → `BcProgram`：
//! - 顶层形式序列编译进 main 原型（entry = 0）；
//! - Lambda 编译为独立原型 + 闭包捕获转换；
//! - If 经跳转回填（emit_jump / patch_jump）；
//! - 求值顺序契约（§19.5 不变式 3）：App 的参数从左到右、被调者最后压栈。
//!
//! **全局符号解析**（Stage 0 名称基解析，TD-004 的既定近似）：
//! LOAD_GLOBAL 未命中时按 `$hyg$N` 后缀剥离回退——使卫生重命名的引入
//! 引用仍能解析到同名全局（捕获保护与全局可达性同时成立）。

use std::collections::HashMap;
use std::rc::Rc;

use kerf_core::{CoreExpr, LiteralValue};
use kerf_span::Span;
use kerf_syntax::{ScopeSet, Symbol};

use crate::bytecode::{BcConst, BcProgram, BcProto, CaptureSource, ConstPool};
use crate::opcode::Op;

/// 编译错误（`{ message, span }` 最小形态）。
#[derive(Debug, Clone, PartialEq)]
pub struct CompileError {
    pub message: String,
    pub span: Span,
}

impl CompileError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        CompileError {
            message: message.into(),
            span,
        }
    }
}

/// 变量的解析结果（唯一可信数据源：编译器的作用域栈）。
#[derive(Debug, Clone, Copy, PartialEq)]
enum VarSource {
    /// 当前帧局部槽。
    Local(u32),
    /// 当前闭包捕获槽。
    Captured(u32),
    /// 全局（按名——空作用域全局绑定 ⊆ 任意引用集，由 VM/eval 全局层承接）。
    Global,
}

/// 完整解析结果（含命中绑定的绑定作用域集——捕获描述符需要它
/// 注入新帧的 captures，使内层原型的体内引用经同一子集匹配命中）。
#[derive(Debug, Clone)]
struct ResolvedVar {
    source: VarSource,
    binder_scopes: ScopeSet,
}

/// 词法绑定项（编译期）：名 + 绑定作用域集（TD-004/r13）。
#[derive(Debug, Clone)]
struct Binding {
    name: Symbol,
    scopes: ScopeSet,
}

/// 帧内子集匹配解析：同名且 `binder.scopes ⊆ ref_scopes` 的绑定中取
/// max-cardinality（并列先注册优先——同层同名经展开器/define_scoped
/// 已拒绝，并列仅理论可能）；返回（槽位，绑定作用域集）。
fn match_bindings(
    bindings: &[Binding],
    name: Symbol,
    ref_scopes: &ScopeSet,
) -> Option<(u32, ScopeSet)> {
    let mut best: Option<(u32, ScopeSet)> = None;
    let mut best_rank = 0usize;
    for (i, b) in bindings.iter().enumerate() {
        if b.name != name || !b.scopes.is_subset_of(ref_scopes) {
            continue;
        }
        let rank = b.scopes.len();
        if best.is_none() || rank > best_rank {
            best = Some((i as u32, b.scopes.clone()));
            best_rank = rank;
        }
    }
    best
}

/// 词法作用域栈帧（编译期）。
#[derive(Debug, Default)]
struct ScopeFrame {
    locals: Vec<Binding>,
    captures: Vec<Binding>,
}

/// 编译上下文（§10.1 规则 2：`Ctxt` 后缀）。
pub struct CompileCtxt {
    /// 作用域栈（帧 0 = main 原型）。
    scopes: Vec<ScopeFrame>,
    /// 全部原型。
    protos: Vec<BcProto>,
    /// 当前正在发射的原型索引（嵌套 Lambda 的切换点）。
    current: usize,
    /// 常量池。
    pool: ConstPool,
    /// 全局引用登记（去重）。
    global_refs: Vec<Symbol>,
    /// 全局名 → 常量池索引（快速查）。
    global_index: HashMap<Symbol, u32>,
    /// 跳转回填占位队列（§19.3 不变式 2：完成时必须清空）。
    pending_patches: Vec<(usize, usize)>, // (proto_idx, pc)
    /// trampoline 哨兵原型索引（r25/42-f——程序级惰性单例；None = 未
    /// 生成。首个 Handle 时创建：code=[Ret]，与 krf 侧 CC-TRAMPOLINE
    /// 同序镜像——parity 确定性）。
    trampoline: Option<u32>,
}

impl CompileCtxt {
    fn new() -> Self {
        CompileCtxt {
            scopes: vec![ScopeFrame::default()],
            protos: Vec::new(),
            current: 0,
            pool: ConstPool::new(),
            global_refs: Vec::new(),
            global_index: HashMap::new(),
            pending_patches: Vec::new(),
            trampoline: None,
        }
    }

    /// 解析变量来源（TD-004/r13：Racket 集合作用域）。
    ///
    /// 规则：帧栈自顶（内层）向下；帧内 locals 先于 captures，同名绑定
    /// 中 `binder.scopes ⊆ ref_scopes` 者取 max-cardinality；首个含
    /// 子集匹配的帧胜出（链序）。无任何子集候选 → Global（空作用域
    /// 全局/内置承接名称基解析；`$hyg$` 基名回退 = 已降级的回退路径，
    /// 与 eval 路径 `Env::lookup` 同一口径——T1 双路径一致）。
    /// 良构程序上（注入不变式：内层绑定作用域集严格包含外层）与旧
    /// 名称基栈序同解；差异仅在引用作用域集不含绑定作用域集时——
    /// 旧实现误命中同名绑定，新实现按 Racket 语义判为未绑定→全局。
    fn resolve_var(&self, name: Symbol, ref_scopes: &ScopeSet) -> ResolvedVar {
        for (fi, frame) in self.scopes.iter().enumerate().rev() {
            if fi == 0 {
                // main 原型帧：局部槽存在但不构成闭包语义——仅局部
                if let Some((i, bs)) = match_bindings(&frame.locals, name, ref_scopes) {
                    return ResolvedVar {
                        source: VarSource::Local(i),
                        binder_scopes: bs,
                    };
                }
                continue;
            }
            if let Some((i, bs)) = match_bindings(&frame.locals, name, ref_scopes) {
                return ResolvedVar {
                    source: VarSource::Local(i),
                    binder_scopes: bs,
                };
            }
            if let Some((i, bs)) = match_bindings(&frame.captures, name, ref_scopes) {
                return ResolvedVar {
                    source: VarSource::Captured(i),
                    binder_scopes: bs,
                };
            }
        }
        ResolvedVar {
            source: VarSource::Global,
            binder_scopes: ScopeSet::new(),
        }
    }

    /// 当前帧引用计数（原型索引）。
    fn current_proto_mut(&mut self) -> &mut BcProto {
        let idx = self.current;
        &mut self.protos[idx]
    }

    fn emit(&mut self, op: Op, span: Span) {
        self.current_proto_mut().code.push(op);
        self.current_proto_mut().debug_spans.push(span);
    }

    fn here(&self) -> u32 {
        let idx = self.current;
        self.protos[idx].code.len() as u32
    }

    /// 发射占位跳转（回填完备性：登记占位，§19.3 不变式 2）。
    fn emit_jump(&mut self, op_factory: fn(u32) -> Op, span: Span) -> u32 {
        let pc = self.here();
        self.emit(op_factory(u32::MAX), span);
        let proto_idx = self.current;
        self.pending_patches.push((proto_idx, pc as usize));
        pc
    }

    /// 回填跳转目标。
    fn patch_jump(&mut self, pc: u32) {
        let proto_idx = self.current;
        let target = self.here();
        let proto = &mut self.protos[proto_idx];
        match &mut proto.code[pc as usize] {
            Op::Jump(t) => *t = target,
            Op::JumpIfFalse(t) => *t = target,
            other => {
                let msg = format!("回填目标不是跳转指令：{:?}", other);
                panic!("编译器内部不变式破坏：{}", msg);
            }
        }
        // 从占位队列移除（matched by pc）
        self.pending_patches
            .retain(|(p, c)| !(*p == proto_idx && *c == pc as usize));
    }

    fn intern_const(&mut self, c: BcConst) -> u32 {
        self.pool.intern(c)
    }

    fn intern_global(&mut self, name: Symbol) -> u32 {
        if let Some(&i) = self.global_index.get(&name) {
            return i;
        }
        let i = self.intern_const(BcConst::Symbol(name));
        self.global_refs.push(name);
        self.global_index.insert(name, i);
        i
    }
}

/// 编译模块入口（§10.1 规则 1：`<verb>_<noun>` 自由函数）。
///
/// 输入：顶层 CoreExpr 序列（expander 产物）。
/// 输出：`BcProgram`（entry = 0 = main 原型）。
pub fn compile_module(exprs: &[Rc<CoreExpr>]) -> Result<BcProgram, CompileError> {
    let mut ctx = CompileCtxt::new();
    let main_span = exprs.first().map(|e| e.span()).unwrap_or_default();

    // main 原型（占位——在编译过程中填充；current = 0）
    ctx.protos.push(BcProto {
        name: Symbol(u32::MAX - 1), // main 名（渲染由调用方替换）
        params: vec![],
        n_locals: 0,
        capture_names: vec![],
        capture_sources: vec![],
        code: vec![],
        debug_spans: vec![],
        free_vars: vec![],
    });
    ctx.current = 0;

    if exprs.is_empty() {
        ctx.emit(Op::PushNil, main_span);
    }
    for (i, e) in exprs.iter().enumerate() {
        // 顶层形式非尾位（主原型 Halt 终止——TCO 不参与顶层）
        compile_expr(&mut ctx, e, false)?;
        if i + 1 < exprs.len() {
            ctx.emit(Op::Pop, e.span()); // 栈平衡：中间值弹出
        }
    }
    ctx.emit(Op::Halt, main_span);

    // 回填完备性断言（§19.3 不变式 2）
    if !ctx.pending_patches.is_empty() {
        let (p, c) = ctx.pending_patches[0];
        return Err(CompileError::new(
            format!("跳转回填不完备：proto {} pc {} 未回填", p, c),
            ctx.protos[p].debug_spans[c],
        ));
    }

    let consts = ctx.pool.finish();
    let global_refs = std::mem::take(&mut ctx.global_refs);
    Ok(BcProgram {
        protos: ctx.protos,
        consts,
        entry: 0,
        global_refs,
        module_name: None,
    })
}

/// 编译单条表达式（栈平衡：产物执行后栈净 +1，§19.3 不变式 1）。
///
/// **尾位穿线（TD-022/H2——VM TCO）**：`tail = true` 表示本表达式位于
/// 函数体尾位（Lambda 体 / If 两臂 / Begin 末项的传递闭包）——其中的
/// `App` 编译为 `Op::TailCall`（帧复用：被调方返回直达当前调用者，跳过
/// 当前帧）。顶层形式恒 `tail = false`（主原型以 Halt 终止——不参与）。
fn compile_expr(ctx: &mut CompileCtxt, e: &CoreExpr, tail: bool) -> Result<(), CompileError> {
    let span = e.span();
    match e {
        CoreExpr::Literal { value, .. } => {
            let idx = match value {
                LiteralValue::Int(v) => ctx.intern_const(BcConst::Int(*v)),
                LiteralValue::Float(v) => ctx.intern_const(BcConst::Float(*v)),
                LiteralValue::Str(s) => ctx.intern_const(BcConst::Str(s.clone())),
                LiteralValue::Symbol(s) => ctx.intern_const(BcConst::SymLit(s.clone())),
                LiteralValue::Bool(b) => {
                    ctx.emit(if *b { Op::PushTrue } else { Op::PushFalse }, span);
                    return Ok(());
                }
                LiteralValue::Nil => {
                    ctx.emit(Op::PushNil, span);
                    return Ok(());
                }
                // 引号点对结构：递归构造（与 MAKE_PAIR 对齐——先 car 后 cdr 压栈）
                LiteralValue::Pair(car, cdr) => {
                    compile_literal_pair(ctx, car, cdr, span)?;
                    return Ok(());
                }
            };
            ctx.emit(Op::PushConst(idx), span);
            Ok(())
        }
        CoreExpr::VarRef { name, scopes, .. } => {
            match ctx.resolve_var(*name, scopes).source {
                VarSource::Local(i) => ctx.emit(Op::LoadLocal(i), span),
                VarSource::Captured(i) => ctx.emit(Op::LoadCaptured(i), span),
                VarSource::Global => {
                    let gi = ctx.intern_global(*name);
                    ctx.emit(Op::LoadGlobal(gi), span);
                }
            }
            Ok(())
        }
        CoreExpr::Lambda {
            params,
            param_scopes,
            body,
            ..
        } => compile_lambda(ctx, params, param_scopes, body, span),
        CoreExpr::App { fn_expr, args, .. } => {
            // 求值顺序契约（06 §1.3/§2 A1，与 eval 路径一致——T1 定理归纳基础）：
            // 被调函数先求值，参数从左到右
            compile_expr(ctx, fn_expr, false)?;
            for a in args {
                compile_expr(ctx, a, false)?;
            }
            // 尾位 App → TailCall（帧复用）；其余 → Call（帧增长 + MAX_FRAMES 守卫）
            let op = if tail {
                Op::TailCall(args.len() as u32)
            } else {
                Op::Call(args.len() as u32)
            };
            ctx.emit(op, span);
            Ok(())
        }
        CoreExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            compile_expr(ctx, cond, false)?;
            let j_false = ctx.emit_jump(Op::JumpIfFalse, span);
            compile_expr(ctx, then_branch, tail)?;
            let j_end = ctx.emit_jump(Op::Jump, span);
            ctx.patch_jump(j_false);
            compile_expr(ctx, else_branch, tail)?;
            ctx.patch_jump(j_end);
            Ok(())
        }
        CoreExpr::SetBang {
            name,
            scopes,
            value,
            ..
        } => {
            // 栈平衡（§19.3 不变式 1）：set! 表达式的值为被赋值——
            // 值入栈后 DUP 再存储，栈净 +1（目标解析与 VarRef 同一口径，TD-004）
            compile_expr(ctx, value, false)?;
            ctx.emit(Op::Dup, span);
            match ctx.resolve_var(*name, scopes).source {
                VarSource::Local(i) => ctx.emit(Op::StoreLocal(i), span),
                VarSource::Captured(i) => ctx.emit(Op::StoreCaptured(i), span),
                VarSource::Global => {
                    let gi = ctx.intern_global(*name);
                    ctx.emit(Op::StoreGlobal(gi), span);
                }
            }
            Ok(())
        }
        CoreExpr::Define { name, value, .. } => {
            // 顶层定义：求值 → DUP → 定义全局（Define 返回 v——R6/D1，
            // 与 eval 路径 `Env::define` 后返回 v 一致，T1 定理）。
            // DUP 留存返回值，DefineGlobal 弹出另一份写入全局（栈净 +1）。
            //
            // D1 修复（全局泄漏防护）：Define 仅允许出现在入口原型
            // （顶层/模块体 inline）。函数体内经 begin/when 等表达式位置
            // 漏入的 define（展开器体提升只匹配直接头部 define）在此
            // 结构化拒绝——否则 DefineGlobal 会把局部名泄漏为全局，
            // 与 eval 路径的词法 Env::define 分裂（违反 T1）。两路径
            // 共享 compile_source，故双侧同报 E0003。
            if ctx.current != 0 {
                return Err(CompileError::new(
                    "define 仅允许出现在顶层或函数体头部（do/表达式位置包裹的 define 不合法——嵌套 define 请置于体头部）",
                    span,
                ));
            }
            compile_expr(ctx, value, false)?;
            ctx.emit(Op::Dup, span);
            let gi = ctx.intern_global(*name);
            ctx.emit(Op::DefineGlobal(gi), span);
            Ok(())
        }
        CoreExpr::Begin { body, .. } => {
            if body.is_empty() {
                ctx.emit(Op::PushNil, span);
                return Ok(());
            }
            for (i, item) in body.iter().enumerate() {
                // 末项继承尾位（begin 尾调用 = 尾调用——scheme 语义）
                let item_tail = tail && i + 1 == body.len();
                compile_expr(ctx, item, item_tail)?;
                if i + 1 < body.len() {
                    ctx.emit(Op::Pop, item.span());
                }
            }
            Ok(())
        }
        CoreExpr::Module { body, name, .. } => {
            // Stage 0 单模块：体 inline 编译；模块名记入元信息
            if body.is_empty() {
                ctx.emit(Op::PushNil, span);
            }
            for (i, item) in body.iter().enumerate() {
                compile_expr(ctx, item, false)?;
                if i + 1 < body.len() {
                    ctx.emit(Op::Pop, item.span());
                }
            }
            if let Some(pm) = ctx.protos.first_mut() {
                let _ = name;
                pm.name = Symbol(u32::MAX - 1);
            }
            Ok(())
        }
        CoreExpr::Require { span, .. } => {
            // r8 能力声明：求值恒为 nil（零副作用）——产 PushNil 保持
            // 顶层形式「每形式一值」栈不变式（与 eval 路径 Ok(Nil) 一致，
            // T1 双路径口径；权限验证已在 driver front 管线 R9/E0006 完成）
            ctx.emit(Op::PushNil, *span);
            Ok(())
        }
        CoreExpr::Perform { effect, span } => {
            // r25/42-f（R10-perform）：效应值先求值（求值顺序契约 App
            // 序不变——效应值表达式求值后入栈）→ PERFORM（解构 (tag .
            // payload) + 扫描分派 + continuation 快照 + 栈截回水位）。
            // 恒非尾位：控制转移后未恢复则本表达式永不完成（body 剩余
            // 部分被 dispatch 丢弃——resume 后值由恢复点注入）
            compile_expr(ctx, effect, false)?;
            ctx.emit(Op::Perform, *span);
            Ok(())
        }
        CoreExpr::Handle {
            tag,
            payload_var,
            payload_scopes,
            resume_var,
            resume_scopes,
            handler_body,
            body,
            span,
        } => compile_handle(
            ctx,
            tag,
            payload_var,
            payload_scopes,
            resume_var,
            resume_scopes,
            handler_body,
            body,
            *span,
        ),
    }
}

/// 引号点对递归编译：先构造点（car 构造 → cdr 构造 → MAKE_PAIR）。
fn compile_literal_pair(
    ctx: &mut CompileCtxt,
    car: &Rc<LiteralValue>,
    cdr: &Rc<LiteralValue>,
    span: Span,
) -> Result<(), CompileError> {
    compile_literal_value(ctx, car, span)?;
    compile_literal_value(ctx, cdr, span)?;
    ctx.emit(Op::MakePair, span);
    Ok(())
}

fn compile_literal_value(
    ctx: &mut CompileCtxt,
    v: &LiteralValue,
    span: Span,
) -> Result<(), CompileError> {
    match v {
        LiteralValue::Pair(a, b) => compile_literal_pair(ctx, a, b, span),
        LiteralValue::Symbol(s) => {
            let idx = ctx.intern_const(BcConst::SymLit(s.clone()));
            ctx.emit(Op::PushConst(idx), span);
            Ok(())
        }
        other => compile_expr(
            ctx,
            &CoreExpr::Literal {
                value: other.clone(),
                span,
            },
            false,
        )
        .map(|_| ()),
    }
}

/// Lambda 编译：闭包捕获转换（§8.5 基础闭包 + §19.3 CLOSURE 指令）。
///
/// 步骤：
/// 1. 计算 Lambda 的自由变量出现（名 + 首现引用作用域集，TD-004）；
/// 2. 对每个自由变量按 `(name, scopes ⊆)` 解析当前作用域栈：
///    Local/Captured → 捕获槽（携带命中绑定的绑定作用域集）；Global → 原型内全局引用；
/// 3. 压入捕获值（与捕获表顺序一致），发射 CLOSURE；
/// 4. 压入新帧作用域（形参绑定携带绑定作用域集；捕获绑定携带原绑定
///    作用域集——体内引用经同一子集匹配命中），编译原型体（参数 = 局部槽 0..n）。
fn compile_lambda(
    ctx: &mut CompileCtxt,
    params: &[Symbol],
    param_scopes: &[ScopeSet],
    body: &Rc<CoreExpr>,
    span: Span,
) -> Result<(), CompileError> {
    // 自由变量出现（排除参数屏蔽；携带首现引用作用域集——捕获判定数据源）
    let mut bound: Vec<Symbol> = params.to_vec();
    let mut free: Vec<(Symbol, ScopeSet)> = Vec::new();
    body.free_var_occurrences(&mut bound, &mut free);

    // 捕获解析：可解析为局部/捕获的自由变量（编译期定型捕获描述符）
    let mut capture_names: Vec<Symbol> = Vec::new();
    let mut capture_bindings: Vec<Binding> = Vec::new();
    let mut capture_srcs: Vec<CaptureSource> = Vec::new();
    let mut free_vars_all: Vec<Symbol> = Vec::new();
    for (f, f_scopes) in &free {
        free_vars_all.push(*f);
        let r = ctx.resolve_var(*f, f_scopes);
        match r.source {
            VarSource::Local(i) => {
                capture_names.push(*f);
                capture_bindings.push(Binding {
                    name: *f,
                    scopes: r.binder_scopes,
                });
                capture_srcs.push(CaptureSource::Local(i));
            }
            VarSource::Captured(i) => {
                capture_names.push(*f);
                capture_bindings.push(Binding {
                    name: *f,
                    scopes: r.binder_scopes,
                });
                capture_srcs.push(CaptureSource::Captured(i));
            }
            VarSource::Global => {} // 原型内按全局引用
        }
    }

    // 新原型（捕获描述符随原型冻结——CLOSURE 时 VM 直取共享单元格）
    let proto_idx = ctx.protos.len() as u32;
    let proto_name = params.first().copied().unwrap_or(Symbol(u32::MAX - 2));
    ctx.protos.push(BcProto {
        name: proto_name,
        params: params.to_vec(),
        n_locals: params.len() as u32,
        capture_names: capture_names.clone(),
        capture_sources: capture_srcs,
        code: vec![],
        debug_spans: vec![],
        free_vars: free_vars_all,
    });

    // 新作用域帧（形参绑定携带绑定作用域集；缺失时容错为空集——
    // 手工构造的 CoreExpr/旧序列化路径不携带作用域即名称基语义）
    let local_bindings: Vec<Binding> = params
        .iter()
        .enumerate()
        .map(|(i, n)| Binding {
            name: *n,
            scopes: param_scopes.get(i).cloned().unwrap_or_default(),
        })
        .collect();
    ctx.scopes.push(ScopeFrame {
        locals: local_bindings,
        captures: capture_bindings,
    });

    // 体编译（尾部 RET 保证返回语义）——切入新原型。
    // 函数体 = 尾位（TCO）：体尾 App → TailCall（帧复用直达调用者）
    let enclosing = ctx.current;
    ctx.current = proto_idx as usize;
    compile_expr(ctx, body, true)?;
    ctx.emit(Op::Ret, body.span());

    // 弹出作用域帧；切回外围原型发射 CLOSURE
    // （捕获源描述符在原型内——VM 据此共享外围单元格，无需栈上预压）
    ctx.scopes.pop();
    ctx.current = enclosing;
    ctx.emit(
        Op::Closure {
            proto: proto_idx,
            n_captures: capture_names.len() as u32,
        },
        span,
    );
    Ok(())
}

/// Handle 表达式编译（r25/42-f——effect-language-design D6 帧编排，
/// 三原型方案）。
///
/// 原型（确定性序：T 惰性 → H → B）：
/// - **T**（trampoline，程序级惰性单例）：`code = [Ret]` 哨兵原型——
///   handler 帧的执行原型，body RET 经它弹 handler 帧回到 handle
///   后续（R11-return）；
/// - **H**（handler 原型）：params = [payload_var, resume_var]，体 =
///   handler_body（尾部 Ret）——dispatch 变形时挂入 H 执行帧；
/// - **B**（body thunk 原型）：params = []，体 = handle body（尾部
///   Ret；体尾位穿线 = lambda 体语义——body 内尾调用拆 thunk 帧
///   穿透 handler 帧，D8）。
///
/// 发射：`InstallHandler { handler: H, body: B, tag: k, trampoline: T }`
/// （VM 从当前帧取两原型捕获 → 压 handler 帧 + thunk 帧 → 转移执行
/// body——编译器不发射 Closure/Call：帧编排由指令原子完成）。捕获
/// 源描述符随原型冻结（与 Lambda 同机制）。 tag 入常量池（SymLit——
/// 符号字面量同型）。
#[allow(clippy::too_many_arguments)]
fn compile_handle(
    ctx: &mut CompileCtxt,
    tag: &Rc<str>,
    payload_var: &Symbol,
    payload_scopes: &ScopeSet,
    resume_var: &Symbol,
    resume_scopes: &ScopeSet,
    handler_body: &Rc<CoreExpr>,
    body: &Rc<CoreExpr>,
    span: Span,
) -> Result<(), CompileError> {
    // ① trampoline 原型（程序级惰性单例——code=[Ret]；span 恒 dummy
    //    与 krf 侧 (0 0 0) 三元组对齐——parity 判据含 debug_spans）
    let tp = ensure_trampoline(ctx);
    // ② H 原型（handler：params = [payload_var, resume_var]）
    let hp = compile_inner_proto(
        ctx,
        &[*payload_var, *resume_var],
        &[payload_scopes.clone(), resume_scopes.clone()],
        handler_body,
    )?;
    // ③ B 原型（body thunk：params = []）
    let bp = compile_inner_proto(ctx, &[], &[], body)?;
    // ④ tag 常量（SymLit——符号字面量同型，按内容 intern）
    let tk = ctx.intern_const(BcConst::SymLit(tag.clone()));
    // ⑤ 原子帧编排指令（捕获由 VM 从当前帧按两原型描述符取）
    ctx.emit(
        Op::InstallHandler {
            handler: hp,
            body: bp,
            tag: tk,
            trampoline: tp,
        },
        span,
    );
    Ok(())
}

/// trampoline 惰性生成（程序级单例——首个 Handle 时创建；确定性：
/// 同源程序原型序恒一致）。`debug_spans` 恒 `[dummy]`（哨兵帧的
/// RET 无源位置——与 krf 侧 (0 0 0) 对齐）。
fn ensure_trampoline(ctx: &mut CompileCtxt) -> u32 {
    if let Some(idx) = ctx.trampoline {
        return idx;
    }
    let idx = ctx.protos.len() as u32;
    ctx.protos.push(BcProto {
        name: Symbol(u32::MAX - 2),
        params: Vec::new(),
        n_locals: 0,
        capture_names: Vec::new(),
        capture_sources: Vec::new(),
        code: vec![Op::Ret],
        debug_spans: vec![Span::dummy()],
        free_vars: Vec::new(),
    });
    ctx.trampoline = Some(idx);
    idx
}

/// 内层原型构造（`compile_lambda` 的原型段——无 CLOSURE 发射；
/// r25/42-f 供 Handle 的 H/B 原型复用：自由变量分析 + 捕获解析 +
/// 新原型 + 作用域帧 + 体编译（尾位 = true——lambda 体语义）+ 尾部
/// RET + 弹帧还原窗口）。
fn compile_inner_proto(
    ctx: &mut CompileCtxt,
    params: &[Symbol],
    param_scopes: &[ScopeSet],
    body: &Rc<CoreExpr>,
) -> Result<u32, CompileError> {
    let mut bound: Vec<Symbol> = params.to_vec();
    let mut free: Vec<(Symbol, ScopeSet)> = Vec::new();
    body.free_var_occurrences(&mut bound, &mut free);

    let mut capture_names: Vec<Symbol> = Vec::new();
    let mut capture_bindings: Vec<Binding> = Vec::new();
    let mut capture_srcs: Vec<CaptureSource> = Vec::new();
    let mut free_vars_all: Vec<Symbol> = Vec::new();
    for (f, f_scopes) in &free {
        free_vars_all.push(*f);
        let r = ctx.resolve_var(*f, f_scopes);
        match r.source {
            VarSource::Local(i) => {
                capture_names.push(*f);
                capture_bindings.push(Binding {
                    name: *f,
                    scopes: r.binder_scopes,
                });
                capture_srcs.push(CaptureSource::Local(i));
            }
            VarSource::Captured(i) => {
                capture_names.push(*f);
                capture_bindings.push(Binding {
                    name: *f,
                    scopes: r.binder_scopes,
                });
                capture_srcs.push(CaptureSource::Captured(i));
            }
            VarSource::Global => {} // 原型内按全局引用
        }
    }

    let proto_idx = ctx.protos.len() as u32;
    let proto_name = params.first().copied().unwrap_or(Symbol(u32::MAX - 2));
    ctx.protos.push(BcProto {
        name: proto_name,
        params: params.to_vec(),
        n_locals: params.len() as u32,
        capture_names: capture_names.clone(),
        capture_sources: capture_srcs,
        code: vec![],
        debug_spans: vec![],
        free_vars: free_vars_all,
    });

    let local_bindings: Vec<Binding> = params
        .iter()
        .enumerate()
        .map(|(i, n)| Binding {
            name: *n,
            scopes: param_scopes.get(i).cloned().unwrap_or_default(),
        })
        .collect();
    ctx.scopes.push(ScopeFrame {
        locals: local_bindings,
        captures: capture_bindings,
    });

    let enclosing = ctx.current;
    ctx.current = proto_idx as usize;
    compile_expr(ctx, body, true)?;
    ctx.emit(Op::Ret, body.span());
    ctx.scopes.pop();
    ctx.current = enclosing;
    Ok(proto_idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kerf_span::Span;

    fn lit(v: i64) -> Rc<CoreExpr> {
        Rc::new(CoreExpr::Literal {
            value: LiteralValue::Int(v),
            span: Span::dummy(),
        })
    }

    fn var(n: u32) -> Rc<CoreExpr> {
        Rc::new(CoreExpr::VarRef {
            name: Symbol(n),
            scopes: ScopeSet::new(),
            span: Span::dummy(),
        })
    }

    fn app(f: Rc<CoreExpr>, args: Vec<Rc<CoreExpr>>) -> Rc<CoreExpr> {
        Rc::new(CoreExpr::App {
            fn_expr: f,
            args,
            span: Span::dummy(),
        })
    }

    fn compile_ok(exprs: &[Rc<CoreExpr>]) -> BcProgram {
        compile_module(exprs).expect("编译失败")
    }

    #[test]
    fn compile_const_and_halt() {
        let p = compile_ok(&[lit(42)]);
        let main = p.entry_proto();
        assert_eq!(main.code, vec![Op::PushConst(0), Op::Halt]);
        assert_eq!(p.consts, vec![BcConst::Int(42)]);
        // 调试信息全覆盖（§19.3 不变式 3）
        assert_eq!(main.debug_spans.len(), main.code.len());
    }

    #[test]
    fn const_pool_dedup_in_program() {
        // (do 1 1 1) → 常量池只含一个 Int(1)（§19.3 陷阱）
        let exprs = vec![Rc::new(CoreExpr::Begin {
            body: vec![lit(1), lit(1), lit(1)],
            span: Span::dummy(),
        })];
        let p = compile_ok(&exprs);
        assert_eq!(p.consts, vec![BcConst::Int(1)]);
        // 栈平衡：PUSH POP PUSH POP PUSH HALT
        let main = p.entry_proto();
        assert_eq!(
            main.code,
            vec![
                Op::PushConst(0),
                Op::Pop,
                Op::PushConst(0),
                Op::Pop,
                Op::PushConst(0),
                Op::Halt
            ]
        );
    }

    #[test]
    fn if_backpatch_complete() {
        // (if cond 1 2)
        let expr = Rc::new(CoreExpr::If {
            cond: var(0),
            then_branch: lit(1),
            else_branch: lit(2),
            span: Span::dummy(),
        });
        let p = compile_ok(&[expr]);
        let main = p.entry_proto();
        // LOAD_GLOBAL, JUMP_IF_FALSE t, PUSH 1, JUMP e, PUSH 2(else), HALT
        assert!(matches!(main.code[1], Op::JumpIfFalse(t) if t != u32::MAX));
        assert!(matches!(main.code[3], Op::Jump(t) if t != u32::MAX));
        match (&main.code[1], &main.code[3]) {
            (Op::JumpIfFalse(tf), Op::Jump(te)) => {
                // else 入口 = PUSH 2 的位置（4）；汇合点 = HALT（5）
                assert_eq!(*tf, 4);
                assert_eq!(*te, 5);
            }
            _ => panic!("跳转结构错误"),
        }
    }

    #[test]
    fn lambda_closure_conversion() {
        // (fn (x) (f x))——f 自由 → 全局引用（Stage 0 单帧无捕获源）
        let body = app(var(5), vec![var(0)]);
        let lam = Rc::new(CoreExpr::Lambda {
            params: vec![Symbol(0)],
            param_scopes: vec![ScopeSet::new()],
            body,
            span: Span::dummy(),
        });
        let p = compile_ok(&[lam]);
        assert_eq!(p.protos.len(), 2);
        let closure_proto = &p.protos[1];
        assert_eq!(closure_proto.arity(), 1);
        assert!(closure_proto.capture_names.is_empty());
        // 原型体：LOAD_GLOBAL f, LOAD_LOCAL 0, TAIL_CALL 1, RET
        // （06 §2 A1：函数先求值，参数从左到右；体尾 App = TailCall——
        // TD-022/H2 TCO 尾位发射，帧复用语义）
        assert_eq!(
            closure_proto.code,
            vec![
                Op::LoadGlobal(0),
                Op::LoadLocal(0),
                Op::TailCall(1),
                Op::Ret
            ]
        );
        // main：CLOSURE proto=1 captures=0, HALT
        assert_eq!(
            p.entry_proto().code,
            vec![
                Op::Closure {
                    proto: 1,
                    n_captures: 0
                },
                Op::Halt
            ]
        );
    }

    #[test]
    fn nested_lambda_captures() {
        // (fn (x) (fn (y) (x y)))——内层捕获 x
        let inner = Rc::new(CoreExpr::Lambda {
            params: vec![Symbol(1)],
            param_scopes: vec![ScopeSet::new()],
            body: app(var(0), vec![var(1)]),
            span: Span::dummy(),
        });
        let outer = Rc::new(CoreExpr::Lambda {
            params: vec![Symbol(0)],
            param_scopes: vec![ScopeSet::new()],
            body: inner,
            span: Span::dummy(),
        });
        let p = compile_ok(&[outer]);
        assert_eq!(p.protos.len(), 3);
        // 原型 2（内层）捕获 x（源 = 原型 1 的局部槽 0）
        assert_eq!(p.protos[2].capture_names, vec![Symbol(0)]);
        assert_eq!(p.protos[2].capture_sources, vec![CaptureSource::Local(0)]);
        // main → CLOSURE(1, 0)；原型 1：CLOSURE(2, 1), RET（捕获描述符直取）
        assert_eq!(
            p.protos[1].code,
            vec![
                Op::Closure {
                    proto: 2,
                    n_captures: 1
                },
                Op::Ret
            ]
        );
        // 原型 2：LOAD_CAPTURED 0(x), LOAD_LOCAL 0(y), TAIL_CALL 1, RET
        // （06 §2 A1 求值顺序：被调者先于参数；体尾 App = TailCall——
        // TD-022/H2 TCO 尾位发射）
        assert_eq!(
            p.protos[2].code,
            vec![
                Op::LoadCaptured(0),
                Op::LoadLocal(0),
                Op::TailCall(1),
                Op::Ret
            ]
        );
    }

    #[test]
    fn deterministic_compilation() {
        // 同结果测试基础：同一输入两次编译 → 字节码一致
        let exprs = vec![
            Rc::new(CoreExpr::Define {
                name: Symbol(9),
                value: lit(7),
                span: Span::dummy(),
            }),
            app(var(9), vec![lit(1)]),
        ];
        let a = compile_ok(&exprs);
        let b = compile_ok(&exprs);
        assert!(a.bytecode_equal(&b));
    }

    #[test]
    fn define_emits_global_store() {
        let exprs = vec![Rc::new(CoreExpr::Define {
            name: Symbol(3),
            value: lit(5),
            span: Span::dummy(),
        })];
        let p = compile_ok(&exprs);
        assert_eq!(
            p.entry_proto().code,
            vec![Op::PushConst(0), Op::Dup, Op::DefineGlobal(1), Op::Halt]
        );
        assert_eq!(p.global_refs, vec![Symbol(3)]);
    }

    #[test]
    fn app_evaluates_fn_then_args_left_to_right() {
        // (f a b) → f, a, b, CALL 2（06 §2 A1：函数先，参数从左到右）
        let e = app(var(9), vec![var(1), var(2)]);
        let p = compile_ok(&[e]);
        let code = &p.entry_proto().code;
        // LOAD_GLOBAL f, LOAD_GLOBAL a(1), LOAD_GLOBAL b(2), CALL 2, HALT
        assert!(matches!(code[0], Op::LoadGlobal(_))); // f
        assert!(matches!(code[1], Op::LoadGlobal(_))); // a
        assert!(matches!(code[2], Op::LoadGlobal(_))); // b
        assert_eq!(code[3], Op::Call(2));
    }
}
