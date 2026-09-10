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
use kerf_syntax::Symbol;

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
    /// 全局（按名）。
    Global,
}

/// 词法作用域栈帧（编译期）。
#[derive(Debug, Default)]
struct ScopeFrame {
    locals: Vec<Symbol>,
    captures: Vec<Symbol>,
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
        }
    }

    /// 解析变量来源（栈自顶向下——内层绑定优先）。
    fn resolve_var(&self, name: Symbol) -> VarSource {
        for (fi, frame) in self.scopes.iter().enumerate().rev() {
            if fi == 0 {
                // main 原型帧：局部槽存在但不构成闭包语义——仅局部
                if let Some(i) = frame.locals.iter().position(|l| *l == name) {
                    return VarSource::Local(i as u32);
                }
                continue;
            }
            if let Some(i) = frame.locals.iter().position(|l| *l == name) {
                return VarSource::Local(i as u32);
            }
            if let Some(i) = frame.captures.iter().position(|c| *c == name) {
                return VarSource::Captured(i as u32);
            }
        }
        VarSource::Global
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
        compile_expr(&mut ctx, e)?;
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
fn compile_expr(ctx: &mut CompileCtxt, e: &CoreExpr) -> Result<(), CompileError> {
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
        CoreExpr::VarRef { name, .. } => {
            match ctx.resolve_var(*name) {
                VarSource::Local(i) => ctx.emit(Op::LoadLocal(i), span),
                VarSource::Captured(i) => ctx.emit(Op::LoadCaptured(i), span),
                VarSource::Global => {
                    let gi = ctx.intern_global(*name);
                    ctx.emit(Op::LoadGlobal(gi), span);
                }
            }
            Ok(())
        }
        CoreExpr::Lambda { params, body, .. } => compile_lambda(ctx, params, body, span),
        CoreExpr::App { fn_expr, args, .. } => {
            // 求值顺序契约（06 §1.3/§2 A1，与 eval 路径一致——T1 定理归纳基础）：
            // 被调函数先求值，参数从左到右
            compile_expr(ctx, fn_expr)?;
            for a in args {
                compile_expr(ctx, a)?;
            }
            ctx.emit(Op::Call(args.len() as u32), span);
            Ok(())
        }
        CoreExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            compile_expr(ctx, cond)?;
            let j_false = ctx.emit_jump(Op::JumpIfFalse, span);
            compile_expr(ctx, then_branch)?;
            let j_end = ctx.emit_jump(Op::Jump, span);
            ctx.patch_jump(j_false);
            compile_expr(ctx, else_branch)?;
            ctx.patch_jump(j_end);
            Ok(())
        }
        CoreExpr::SetBang { name, value, .. } => {
            // 栈平衡（§19.3 不变式 1）：set! 表达式的值为被赋值——
            // 值入栈后 DUP 再存储，栈净 +1
            compile_expr(ctx, value)?;
            ctx.emit(Op::Dup, span);
            match ctx.resolve_var(*name) {
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
                    "define 仅允许出现在顶层或函数体头部（begin/表达式位置包裹的 define 不合法——嵌套 define 请置于体头部）",
                    span,
                ));
            }
            compile_expr(ctx, value)?;
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
                compile_expr(ctx, item)?;
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
                compile_expr(ctx, item)?;
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
        )
        .map(|_| ()),
    }
}

/// Lambda 编译：闭包捕获转换（§8.5 基础闭包 + §19.3 CLOSURE 指令）。
///
/// 步骤：
/// 1. 计算 Lambda 的自由变量（CoreExpr::free_variable_list）；
/// 2. 对每个自由变量解析当前作用域：Local/Captured → 捕获槽；Global → 原型内全局引用；
/// 3. 压入捕获值（与捕获表顺序一致），发射 CLOSURE；
/// 4. 压入新帧作用域，编译原型体（参数 = 局部槽 0..n）。
fn compile_lambda(
    ctx: &mut CompileCtxt,
    params: &[Symbol],
    body: &Rc<CoreExpr>,
    span: Span,
) -> Result<(), CompileError> {
    // 自由变量（排除参数屏蔽）
    let mut bound: Vec<Symbol> = params.to_vec();
    let mut free: Vec<Symbol> = Vec::new();
    body.free_variables(&mut bound, &mut free);

    // 捕获解析：可解析为局部/捕获的自由变量（编译期定型捕获描述符）
    let mut capture_names: Vec<Symbol> = Vec::new();
    let mut capture_srcs: Vec<CaptureSource> = Vec::new();
    let mut free_vars_all: Vec<Symbol> = Vec::new();
    for f in &free {
        free_vars_all.push(*f);
        match ctx.resolve_var(*f) {
            VarSource::Local(i) => {
                capture_names.push(*f);
                capture_srcs.push(CaptureSource::Local(i));
            }
            VarSource::Captured(i) => {
                capture_names.push(*f);
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

    // 新作用域帧
    ctx.scopes.push(ScopeFrame {
        locals: params.to_vec(),
        captures: capture_names.clone(),
    });

    // 体编译（尾部 RET 保证返回语义）——切入新原型
    let enclosing = ctx.current;
    ctx.current = proto_idx as usize;
    compile_expr(ctx, body)?;
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
        // (begin 1 1 1) → 常量池只含一个 Int(1)（§19.3 陷阱）
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
        // (lambda (x) (f x))——f 自由 → 全局引用（Stage 0 单帧无捕获源）
        let body = app(var(5), vec![var(0)]);
        let lam = Rc::new(CoreExpr::Lambda {
            params: vec![Symbol(0)],
            body,
            span: Span::dummy(),
        });
        let p = compile_ok(&[lam]);
        assert_eq!(p.protos.len(), 2);
        let closure_proto = &p.protos[1];
        assert_eq!(closure_proto.arity(), 1);
        assert!(closure_proto.capture_names.is_empty());
        // 原型体：LOAD_GLOBAL f, LOAD_LOCAL 0, CALL 1, RET
        // （06 §2 A1：函数先求值，参数从左到右）
        assert_eq!(
            closure_proto.code,
            vec![Op::LoadGlobal(0), Op::LoadLocal(0), Op::Call(1), Op::Ret]
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
        // (lambda (x) (lambda (y) (x y)))——内层捕获 x
        let inner = Rc::new(CoreExpr::Lambda {
            params: vec![Symbol(1)],
            body: app(var(0), vec![var(1)]),
            span: Span::dummy(),
        });
        let outer = Rc::new(CoreExpr::Lambda {
            params: vec![Symbol(0)],
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
        // 原型 2：LOAD_CAPTURED 0(x), LOAD_LOCAL 0(y), CALL 1, RET
        // （06 §2 A1 求值顺序：被调者先于参数）
        assert_eq!(
            p.protos[2].code,
            vec![Op::LoadCaptured(0), Op::LoadLocal(0), Op::Call(1), Op::Ret]
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
