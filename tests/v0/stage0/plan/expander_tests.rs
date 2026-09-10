//! Expander 集成测试（tests/v0/stage0/plan/expander_tests.rs ↔ docs/tests/v0/stage0/plan/expander.md）。
//!
//! 覆盖：9 核心形式展开 / 语法糖推导（§3.2 表）/ 卫生宏 / 相位分离 /
//! 内部 define 提升 / 展开深度上限。

#[path = "../../../common/mod.rs"]
mod common;

use kerf_expander::{expand_program, phase::ModuleRegistry, ExpandCtxt};
use kerf_reader::read_source;
use kerf_syntax::SymbolTable;

fn expand(src: &str) -> Result<String, String> {
    let mut table = SymbolTable::new();
    let forms = read_source(src, 0, &mut table).map_err(|e| e.message)?;
    let mut ctx = ExpandCtxt::new(table);
    let core = expand_program(&forms, &mut ctx).map_err(|e| e.message)?;
    Ok(core
        .iter()
        .map(|e| e.render(&|s| ctx.table.name(s).to_string()))
        .collect::<Vec<_>>()
        .join("\n"))
}

/// 核心形式快照：let → lambda 推导（§3.2 表第 1 行）。
#[test]
fn let_derives_to_lambda_app() {
    assert_eq!(
        expand("(let ((x 1) (y 2)) (+ x y))").unwrap(),
        "((lambda (x y) (+ x y)) 1 2)"
    );
}

/// letrec 推导（§3.2 表第 2 行）。
#[test]
fn letrec_derives_with_setbang() {
    let out = expand("(letrec ((f (lambda () 1))) (f))").unwrap();
    assert!(out.starts_with("((lambda (f)"));
    assert!(out.contains("set! f (lambda () 1)"));
}

/// cond 推导（§3.2 表第 3 行）。
#[test]
fn cond_derives_to_nested_if() {
    assert_eq!(
        expand("(cond (a 1) (b 2) (else 3))").unwrap(),
        "(if a 1 (if b 2 3))"
    );
}

/// and/or 推导（§3.2 表第 4-5 行）。
#[test]
fn and_or_derive() {
    assert_eq!(expand("(and a b)").unwrap(), "(if a b false)");
    assert_eq!(expand("(or a b)").unwrap(), "(if a true b)");
    assert_eq!(expand("(and)").unwrap(), "true");
    assert_eq!(expand("(or)").unwrap(), "false");
    assert_eq!(
        expand("(and a b c)").unwrap(),
        "(if a (if b c false) false)"
    );
}

/// while 推导（§3.2 表第 6 行）——loop 卫生唯一化。
#[test]
fn while_derives_with_fresh_loop() {
    let out = expand("(while c body)").unwrap();
    assert!(out.contains("loop$hyg$"), "loop 必须唯一化：{}", out);
    assert!(out.contains("letrec") || out.starts_with("((lambda"));
    assert!(out.contains("(set! loop$hyg$"));
}

/// 函数糖：(define (f x) body) → (define f (lambda (x) body))。
#[test]
fn define_function_sugar() {
    assert_eq!(
        expand("(define (f x) x)").unwrap(),
        "(define f (lambda (x) x))"
    );
}

/// 内部 define 提升（§19.2 陷阱 3）。
#[test]
fn inner_define_hoisted_to_letrec() {
    let out = expand("(lambda (x) (define y 2) (+ x y))").unwrap();
    assert!(out.contains("set! y 2"), "提升结果：{}", out);
    assert!(out.starts_with("(lambda (x) ((lambda (y)"));
}

/// 内部 define 非头部 → 显式错误。
#[test]
fn inner_define_after_expression_is_error() {
    let err = expand("(lambda (x) (+ x 1) (define y 2))").unwrap_err();
    assert!(err.contains("define 必须位于函数体头部"));
}

/// 卫生宏：宏内外同名不串扰（§20.1 Week 2 验收项）。
#[test]
fn hygiene_renames_introduced_identifiers() {
    let out =
        expand(r#"(define-syntax m (syntax-rules () ((m a) (let ((tmp 1)) (+ a tmp))))) (m x)"#)
            .unwrap();
    assert!(out.contains("tmp$hyg$"), "引入 tmp 必须重命名：{}", out);
    // 用户标识符 x 保持原名（未被重命名）
    assert!(!out.contains("x$hyg$"), "用户标识符不得重命名：{}", out);
    assert!(out.contains(" x "), "用户标识符 x 保留：{}", out);
}

/// 宏自引用不被重命名（递归宏正确性）。
#[test]
fn macro_self_reference_preserved() {
    let out = expand(
        r#"(define-syntax loop2 (syntax-rules () ((loop2) (loop2))))"#
            .to_string()
            .as_str(),
    )
    .unwrap();
    // define-syntax 本身展开为 nil；模板中的 loop2 不重命名（保留集）
    assert_eq!(out, "nil");
}

/// 展开深度上限（§19.2 不变式 1）。
#[test]
fn expansion_depth_limit_errors() {
    let mut table = SymbolTable::new();
    let src = "(define-syntax l2 (syntax-rules () ((l2) (l2)))) (l2)";
    let forms = read_source(src, 0, &mut table).unwrap();
    let mut ctx = ExpandCtxt::new(table);
    let err = expand_program(&forms, &mut ctx).unwrap_err();
    assert!(err.message.contains("展开深度"), "实际：{}", err.message);
}

/// 省略号宏：零或多匹配。
#[test]
fn ellipsis_macro_expands() {
    let out = expand(
        r#"(define-syntax my-list (syntax-rules () ((my-list x ...) (list x ...))))
           (my-list 1 2 3)"#,
    )
    .unwrap();
    let last = out.lines().last().unwrap_or("");
    assert!(last.contains("list"), "展开：{}", last);
    assert!(last.contains("1") && last.contains("3"));
}

/// 相位生命周期：declare → visit → instantiate（§8.9）。
#[test]
fn module_phase_lifecycle() {
    let mut reg = ModuleRegistry::new();
    let mut table = SymbolTable::new();
    let m = table.intern("m");
    reg.declare(m, vec![], vec![]).unwrap();
    // 未 visit 即 instantiate → 相位违规
    assert!(reg.instantiate(m).is_err());
    reg.visit(m).unwrap();
    reg.instantiate(m).unwrap();
}

/// quote 数据化（点对列表）。
#[test]
fn quote_list_becomes_pairs() {
    assert_eq!(expand("(quote (1 2 3))").unwrap(), "(1 2 3)");
}

/// quote 符号显式错误（TD-002）。
#[test]
fn quote_symbol_is_explicit_error() {
    let err = expand("(quote sym)").unwrap_err();
    assert!(err.contains("TD-002"));
}

/// 模块形式展开。
#[test]
fn module_form_snapshot() {
    let out = expand("(module m (import a) (export f) (define f 1))").unwrap();
    assert!(out.starts_with("(module m (import a) (export f)"));
}

/// 糖推导正向锚点（01 §5 锚点表补齐——T15-b 差距项）：
/// let*/when/unless 是核心形式之上的纯语法糖，推导目标可静态断言。
#[test]
fn derived_forms_sugar_expansion_anchors() {
    // let* → 嵌套 lambda 应用（let 本身是 lambda 糖——01 §2 推导）
    let out = expand("(let* ((a 1) (b 2)) (+ a b))").unwrap();
    assert!(
        out.contains("(lambda (a)"),
        "let* 外层应展开为 lambda 应用：{}",
        out
    );
    assert!(out.contains("(lambda (b)"), "let* 内层嵌套：{}", out);
    // when → (if x (begin body…) nil)
    let when_out = expand("(when x 1 2)").unwrap();
    assert_eq!(when_out, "(if x (begin 1 2) nil)");
    // unless → (if x nil (begin body…))
    let unless_out = expand("(unless x 1)").unwrap();
    assert_eq!(unless_out, "(if x nil (begin 1))");
}
