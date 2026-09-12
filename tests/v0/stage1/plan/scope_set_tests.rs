//! Stage 1 批次 E（TD-004，r13）集成测试：作用域集解析（Racket 式
//! `(name, scopes ⊆)` 匹配 + max-cardinality）的端到端语义锚点。
//!
//! 覆盖口径（A4 原验收：Racket 式语义锚点测试 ≥6，含 ≥3 负例）：
//! - **正例**：嵌套 shadowing（内层胜出）/ 闭包捕获（捕获绑定携带
//!   绑定作用域集）/ assign 命中词法绑定而非全局——均双路径（VM + eval）
//!   一致（T1 定理在 scope-set 解析下的保持）；
//! - **负例**：引用作用域集**不含**绑定作用域集时按 Racket 语义判
//!   未绑定（VM → 全局兜底后报「未绑定的全局变量」；eval → 「未绑定
//!   变量」）——这是与旧名称基解析的行为差异点（名称匹配但作用域
//!   不匹配 ≠ 解析命中）；assign 同口径；宏引入绑定不捕获用户同名。
//!
//! 源码级正/负例走 driver 双入口；作用域不匹配负例经手工构造
//! CoreExpr（展开器注入不变式保证真实源码不产生该形态，故手工构造
//! 是该语义的唯一可观测载体——与编译器/eval 单元测试同源）。

use std::rc::Rc;

use kerf_compiler::compile_module;
use kerf_core::CoreExpr;
use kerf_driver::{run_source, run_source_seed};
use kerf_runtime::Heap;
use kerf_span::Span;
use kerf_syntax::{ScopeId, ScopeSet, SymbolTable};
use kerf_vm::{eval_program, run_program, Env, Value};

fn int_of(v: &Value) -> i64 {
    match v {
        Value::Int(i) => *i,
        other => panic!("期望 Int，实际 {:?}", other),
    }
}

/// 构造「绑定作用域集 = {binder}，引用作用域集 = {ref}」的
/// `((fn (x) x) 42)`——`binder ⊄ ref_scopes` 时引用不得命中绑定。
fn mismatched_lambda(binder: ScopeId, ref_scope: ScopeId) -> Vec<Rc<CoreExpr>> {
    let mut table = SymbolTable::new();
    let x = table.intern("x");
    let binder_scopes = ScopeSet::from_iter_scopes([binder]);
    let ref_scopes = ScopeSet::from_iter_scopes([ref_scope]);
    let body = Rc::new(CoreExpr::VarRef {
        name: x,
        scopes: ref_scopes,
        span: Span::dummy(),
    });
    let lam = Rc::new(CoreExpr::Lambda {
        params: vec![x],
        param_scopes: vec![binder_scopes],
        body,
        span: Span::dummy(),
    });
    let arg = Rc::new(CoreExpr::Literal {
        value: kerf_core::LiteralValue::Int(42),
        span: Span::dummy(),
    });
    vec![Rc::new(CoreExpr::App {
        fn_expr: lam,
        args: vec![arg],
        span: Span::dummy(),
    })]
}

// ---- 正例（双路径一致） ----

/// 正例 1：shadowing——内层形参遮蔽全局同名（max-cardinality =
/// 内层绑定作用域集基数严格更大）。双路径。
#[test]
fn shadowing_resolves_innermost_dual_path() {
    let src = "(define x 10) ((fn (x) x) 20)";
    let a = run_source(src, "s1.krf").unwrap();
    let b = run_source_seed(src, "s1.krf").unwrap();
    assert_eq!(int_of(&a.value), 20, "VM 路径内层绑定胜出");
    assert_eq!(int_of(&b.value), 20, "eval 路径内层绑定胜出");
}

/// 正例 2：双层同名嵌套 shadowing——`((fn (x) ((fn (x) x) 2)) 1)`
/// → 2（最内层绑定；scope 注入不变式：内层绑定作用域集严格包含外层）。
/// 双路径。
#[test]
fn nested_shadowing_innermost_wins_dual_path() {
    let src = "(define x 0) ((fn (x) ((fn (x) x) 2)) 1)";
    let a = run_source(src, "s2.krf").unwrap();
    let b = run_source_seed(src, "s2.krf").unwrap();
    assert_eq!(int_of(&a.value), 2);
    assert_eq!(int_of(&b.value), 2);
}

/// 正例 3：闭包捕获——捕获绑定携带原绑定作用域集，内层原型体内引用
/// 经同一子集匹配命中捕获槽。`((fn (x) ((fn (y) (+ x y)) 2)) 1)`
/// → 3。双路径。
#[test]
fn closure_capture_by_scope_set_dual_path() {
    let src = "((fn (x) ((fn (y) (+ x y)) 2)) 1)";
    let a = run_source(src, "s3.krf").unwrap();
    let b = run_source_seed(src, "s3.krf").unwrap();
    assert_eq!(int_of(&a.value), 3, "VM 捕获经子集匹配");
    assert_eq!(int_of(&b.value), 3, "eval 捕获经子集匹配");
}

/// 正例 4：assign 目标解析——命中词法形参绑定而非全局（赋值后全局
/// 不变）。`(define g 0) (do ((fn (x) (assign x 5) x) 9) g)` → 0。
/// 双路径（与旧名称基同解，验证切换零回归）。
#[test]
fn setbang_hits_lexical_not_global_dual_path() {
    let src = "(define g 0) (do ((fn (x) (assign x 5) x) 9) g)";
    let a = run_source(src, "s4.krf").unwrap();
    let b = run_source_seed(src, "s4.krf").unwrap();
    assert_eq!(int_of(&a.value), 0, "VM：assign 命中形参，g 不变");
    assert_eq!(int_of(&b.value), 0, "eval：assign 命中形参，g 不变");
}

// ---- 负例（Racket 语义差异点：作用域不匹配 ≠ 名称匹配） ----

/// 负例 1：引用作用域集不含绑定作用域集 → 不命中词法绑定（VM 路径：
/// 解析为全局 → 运行时报「未绑定的全局变量」）。旧名称基实现会命中
/// 形参槽（错误捕获），本测试锚定新语义的判别点。
#[test]
fn scope_mismatch_reference_unbound_vm() {
    let exprs = mismatched_lambda(7, 9); // 绑定 {7} ⊄ 引用 {9}
    let program = compile_module(&exprs).unwrap();
    let mut globals = std::collections::HashMap::new();
    let mut heap = Heap::new();
    let err = run_program(&program, &mut globals, &mut heap).unwrap_err();
    assert!(
        err.message.contains("未绑定的全局变量"),
        "VM 实际错误：{}",
        err.message
    );
}

/// 负例 2：同上（eval 路径）——`Env::lookup` 子集匹配不命中 →
/// 「未绑定变量」。双路径同判（T1）。
#[test]
fn scope_mismatch_reference_unbound_eval() {
    let exprs = mismatched_lambda(7, 9);
    let root = Env::new();
    let mut heap = Heap::new();
    let err = eval_program(&exprs, &root, &mut heap).unwrap_err();
    assert!(
        err.message.contains("未绑定变量"),
        "eval 实际错误：{}",
        err.message
    );
}

/// 负例 3：assign 目标作用域集不含绑定作用域集 → 不命中词法绑定
/// （VM：StoreGlobal → 「assign 未绑定变量」；eval：env.set 无命中 →
/// 同报）。构造 `(fn (x) (assign x 1) x)` 形态：绑定 {7}，目标引用 {9}。
#[test]
fn scope_mismatch_setbang_unbound_dual_path() {
    let mut table = SymbolTable::new();
    let x = table.intern("x");
    let binder = ScopeSet::from_iter_scopes([7]);
    let mismatch = ScopeSet::from_iter_scopes([9]);
    let one = Rc::new(CoreExpr::Literal {
        value: kerf_core::LiteralValue::Int(1),
        span: Span::dummy(),
    });
    let setb = Rc::new(CoreExpr::SetBang {
        name: x,
        scopes: mismatch,
        value: one,
        span: Span::dummy(),
    });
    let body = Rc::new(CoreExpr::Begin {
        body: vec![setb],
        span: Span::dummy(),
    });
    let lam = Rc::new(CoreExpr::Lambda {
        params: vec![x],
        param_scopes: vec![binder],
        body,
        span: Span::dummy(),
    });
    // 应用闭包使体执行：((fn (x) (do (assign x 1))) 2)
    let two = Rc::new(CoreExpr::Literal {
        value: kerf_core::LiteralValue::Int(2),
        span: Span::dummy(),
    });
    let app = Rc::new(CoreExpr::App {
        fn_expr: lam.clone(),
        args: vec![two],
        span: Span::dummy(),
    });
    // VM 路径
    let program = compile_module(std::slice::from_ref(&app)).unwrap();
    let mut globals = std::collections::HashMap::new();
    let mut heap = Heap::new();
    let err_vm = run_program(&program, &mut globals, &mut heap).unwrap_err();
    assert!(
        err_vm.message.contains("assign 未绑定变量"),
        "VM 实际错误：{}",
        err_vm.message
    );
    // eval 路径
    let root = Env::new();
    let mut heap2 = Heap::new();
    let err_ev = eval_program(&[app], &root, &mut heap2).unwrap_err();
    assert!(
        err_ev.message.contains("assign 未绑定变量"),
        "eval 实际错误：{}",
        err_ev.message
    );
}

/// 负例 4：宏引入绑定不捕获用户同名词法绑定（卫生 + 作用域集双保险）。
/// `(define-syntax self7 ... (let ((t 7)) t))`；用户形参 `t` = 99；
/// 宏展开引用自身的 t（α 重命名 + binder 作用域集含宏引入 fresh 与
/// use-site 并集）→ 结果 7。若捕获破坏（用户 t 泄入宏体）→ 99。
/// 双路径。
#[test]
fn macro_introduced_binding_does_not_capture_user_binding() {
    let src = r#"
        (define-syntax self7 (syntax-rules () ((self7) (let ((t 7)) t))))
        ((fn (t) (self7)) 99)
    "#;
    let a = run_source(src, "s5.krf").unwrap();
    let b = run_source_seed(src, "s5.krf").unwrap();
    assert_eq!(int_of(&a.value), 7, "VM：宏引入 t 不被用户 t 捕获");
    assert_eq!(int_of(&b.value), 7, "eval：宏引入 t 不被用户 t 捕获");
}

/// 对照例（正例 5）：匹配的引用作用域集（含绑定作用域集）命中——
/// 同一手工构造骨架下 `binder ⊆ ref`（{7} ⊆ {7, 9}）解析命中形参。
/// 与负例 1/2 构成同骨架正负对照，证明判别差异确因子集关系。
#[test]
fn scope_subset_match_resolves_to_param() {
    let mut table = SymbolTable::new();
    let x = table.intern("x");
    let binder = ScopeSet::from_iter_scopes([7]);
    let sup = ScopeSet::from_iter_scopes([7, 9]);
    let body = Rc::new(CoreExpr::VarRef {
        name: x,
        scopes: sup,
        span: Span::dummy(),
    });
    let lam = Rc::new(CoreExpr::Lambda {
        params: vec![x],
        param_scopes: vec![binder],
        body,
        span: Span::dummy(),
    });
    let arg = Rc::new(CoreExpr::Literal {
        value: kerf_core::LiteralValue::Int(42),
        span: Span::dummy(),
    });
    let app = Rc::new(CoreExpr::App {
        fn_expr: lam,
        args: vec![arg],
        span: Span::dummy(),
    });
    // VM 路径：命中形参 → 42
    let program = compile_module(std::slice::from_ref(&app)).unwrap();
    let mut globals = std::collections::HashMap::new();
    let mut heap = Heap::new();
    let r = run_program(&program, &mut globals, &mut heap).unwrap();
    assert!(matches!(r, Value::Int(42)), "VM 子集匹配命中：{:?}", r);
    // eval 路径
    let root = Env::new();
    let mut heap2 = Heap::new();
    let r2 = eval_program(&[app], &root, &mut heap2).unwrap();
    assert!(matches!(r2, Value::Int(42)), "eval 子集匹配命中：{:?}", r2);
}

/// 负例 5（TD-018 r24 回归）：if 条件非 bool 的**消息文本双路径同文**
/// ——VM（`JumpIfFalse`）与 eval（if 臂）均经 `messages::err_if_cond_bool`
/// 单源构造（此前 VM 侧「条件位置需要 bool」/ eval 侧「if 条件需要 bool」
/// 文本分裂）。同骨架手构 `(if 1 2 3)` 双跑对拍断言。
#[test]
fn td018_if_cond_message_unified_dual_path() {
    let one = Rc::new(CoreExpr::Literal {
        value: kerf_core::LiteralValue::Int(1),
        span: Span::dummy(),
    });
    let two = Rc::new(CoreExpr::Literal {
        value: kerf_core::LiteralValue::Int(2),
        span: Span::dummy(),
    });
    let three = Rc::new(CoreExpr::Literal {
        value: kerf_core::LiteralValue::Int(3),
        span: Span::dummy(),
    });
    let if_expr = Rc::new(CoreExpr::If {
        cond: one,
        then_branch: two,
        else_branch: three,
        span: Span::dummy(),
    });
    // VM 路径
    let program = compile_module(std::slice::from_ref(&if_expr)).unwrap();
    let mut globals = std::collections::HashMap::new();
    let mut heap = Heap::new();
    let err_vm = run_program(&program, &mut globals, &mut heap).unwrap_err();
    // eval 路径
    let root = Env::new();
    let mut heap2 = Heap::new();
    let err_ev = eval_program(&[if_expr], &root, &mut heap2).unwrap_err();
    assert!(
        err_vm.message.contains("if 条件需要 bool，实际 int"),
        "VM 消息应含统一前缀：{}",
        err_vm.message
    );
    assert!(
        err_ev.message.contains("if 条件需要 bool，实际 int"),
        "eval 消息应与 VM 同文：{}",
        err_ev.message
    );
    // TD-018 核心判据：两路径消息文本一致（分裂面消除的回归锚）
    assert_eq!(
        err_vm.message, err_ev.message,
        "TD-018：if 条件消息必须双路径同文"
    );
}
