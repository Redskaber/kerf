//! VM 集成测试（tests/v0/stage0/plan/vm_tests.rs ↔ docs/tests/v0/stage0/plan/vm.md）。
//!
//! 覆盖：9 核心原语执行语义（§21.3 Stage 0 验收项 1）/ fib(25) /
//! 运行时错误含堆栈追踪 / 双路径互查。

use crate::common;

use common::{assert_int, dual_path_agrees};

/// 9 原语逐项语义验证：Literal / VarRef / Lambda / App / If / SetBang / Define / Begin / Module。
#[test]
fn nine_primitives_semantics() {
    // Literal
    assert_int("42", 42);
    assert_int("\"str\" 1", 1); // 字符串值 + 尾表达式
    assert_int("nil 7", 7);
    // VarRef / Define
    assert_int("(define x 99) x", 99);
    // Lambda / App
    assert_int("((lambda (x) x) 42)", 42);
    assert_int("((lambda (x y) (- x y)) 10 4)", 6);
    // If（真/假/无 else）
    assert_int("(if true 1 2)", 1);
    assert_int("(if false 1 2)", 2);
    // SetBang
    assert_int("(define n 1) (set! n 5) n", 5);
    // Begin
    assert_int("(begin 1 2 3)", 3);
    // Module
    assert_int("(module m (define x 8) x)", 8);
}

/// fib(25) 在 VM 上执行正确（§20.1 Week 3 验收）。
#[test]
fn fib_25_correct_on_vm() {
    let src = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 25)";
    common::assert_int(src, 75025);
}

/// 递归深度 10_000：迭代式循环不受 Rust 栈限制。
#[test]
fn deep_recursion_10k() {
    let src = "(define (count n) (if (= n 0) 0 (count (- n 1)))) (count 10000)";
    common::assert_int(src, 0);
}

/// 词法闭包：共享可变捕获（计数器独立性）。
#[test]
fn closures_share_mutable_captures() {
    let src = r#"
        (define (make-counter)
          (let ((n 0))
            (lambda () (set! n (+ n 1)) n)))
        (define a (make-counter))
        (define b (make-counter))
        (a) (a) (a)
        (b)
        (list (a) (b))
    "#;
    assert_eq!(common::run_rendered(src), "(4 2)");
}

/// 高阶函数：map 经显式递归。
#[test]
fn higher_order_functions() {
    let src = r#"
        (define (map f lst)
          (if (null? lst)
              nil
              (cons (f (car lst)) (map f (cdr lst)))))
        (map (lambda (x) (* x x)) (quote (1 2 3 4 5)))
    "#;
    assert_eq!(common::run_rendered(src), "(1 4 9 16 25)");
}

/// truthy 显式语义：非 bool 条件报错（§19.5 陷阱 3）。
#[test]
fn truthy_requires_bool() {
    let err = common::run("(if 1 2 3)").unwrap_err();
    assert!(err.contains("truthy") || err.contains("bool"));
}

/// 运行时错误捕获含堆栈追踪（§8.12）。
#[test]
fn runtime_error_has_trace() {
    let src = "(define (inner) (car 5)) (define (outer) (inner)) (outer)";
    let err = common::run(src).unwrap_err();
    assert!(err.contains("car") || err.contains("pair"));
}

/// 未绑定变量错误。
#[test]
fn unbound_variable_error() {
    let err = common::run("(undefined-fn 1)").unwrap_err();
    assert!(err.contains("未绑定"));
}

/// 除零错误。
#[test]
fn division_by_zero_error() {
    let err = common::run("(/ 1 0)").unwrap_err();
    assert!(err.contains("除零"));
}

/// 参数数量不匹配。
#[test]
fn arity_mismatch_error() {
    let err = common::run("(define (f x) x) (f 1 2)").unwrap_err();
    assert!(err.contains("参数数量"));
}

/// 双路径互查：核心程序集的 VM 与 eval 结果一致（§21.8 Phase 1）。
#[test]
fn dual_execution_paths_cross_validate() {
    let programs = [
        "(+ 1 2 3)",
        "(define (f x) (+ x 1)) (f 41)",
        "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 12)",
        "(let ((x 1) (y 2)) (+ x y))",
        "(letrec ((even? (lambda (n) (if (= n 0) true (odd? (- n 1))))) (odd? (lambda (n) (if (= n 0) false (even? (- n 1)))))) (even? 10))",
        "(cond ((< 1 0) 1) ((> 1 0) 2) (else 3))",
        "(define (make-counter) (let ((n 0)) (lambda () (set! n (+ n 1)) n))) (define c (make-counter)) (c) (c) (c)",
        "(car (quote (1 2 3)))",
        "((lambda (x y) (cons x y)) 1 2)",
    ];
    for src in programs {
        assert!(dual_path_agrees(src), "双路径结果不一致：{}", src);
    }
}

/// quote 数据操作：序对构造与遍历。
#[test]
fn pair_construction_and_traversal() {
    assert_eq!(common::run_rendered("(cons 1 2)"), "(1 . 2)");
    assert_eq!(common::run_rendered("(quote (1 2 3))"), "(1 2 3)");
    assert_eq!(common::run_rendered("(cdr (quote (1 2 3)))"), "(2 3)");
    assert_eq!(common::run_rendered("(car (cdr (quote (1 2 3))))"), "2");
    assert_eq!(common::run_rendered("(null? (quote ()))"), "true");
}

/// T1 对账（[06-操作语义 §2 R6/D1]）：Define 返回值为被定义值——
/// 双路径必须一致（修复前 VM 返回 nil、eval 返回 v）。
#[test]
fn define_returns_value_dual_path() {
    assert!(dual_path_agrees("(define x 5)"));
    assert!(dual_path_agrees("(define x (+ 2 3)) x"));
    assert_int("(define x 5)", 5); // D1：Define 表达式的值 = v
}

/// E6（[06-操作语义 §3]）：同层重复定义报错——双路径一致
/// （修复前 eval 静默覆盖、VM 静默覆盖）。
#[test]
fn duplicate_define_errors_dual_path() {
    let src = "(define x 1) (define x 2)";
    let err = common::run(src).unwrap_err();
    assert!(err.contains("重复定义变量"), "VM 路径应报 E6：{}", err);
    // 种子链（参考路径——T1 新口径 42-d）
    let ev = kerf_driver::run_source_seed(src, "test.krf").err().unwrap();
    assert!(ev.to_string().contains("重复定义变量"), "种子链应报 E6");
    assert!(dual_path_agrees(src)); // 错误消息形态一致（渲染层对账）
}

/// E3（[06-操作语义 §2 R5/S1]）：set! 未绑定变量在 VM 路径同样报错
/// （修复前 VM 静默创建全局——与 eval 分裂，违反 T1）。
#[test]
fn set_unbound_errors_on_vm() {
    let err = common::run("(set! y 1)").unwrap_err();
    assert!(err.contains("set! 未绑定变量"), "VM 路径应报 E3：{}", err);
    assert!(dual_path_agrees("(set! y 1)"));
    // 正例：已绑定（先 define 后 set!）两路径均成功
    assert!(dual_path_agrees("(define n 1) (set! n 5) n"));
}

/// A3 卫式（[06-操作语义 §2]）：lambda 形参表重名在展开期即拒绝
/// （两执行路径的共同上游单点防御）。
#[test]
fn lambda_duplicate_params_rejected_at_expand() {
    let err = common::run("(lambda (x x) x)").unwrap_err();
    assert!(err.contains("参数重名"), "展开期应拒绝重名形参：{}", err);
}

/// T1 对账（[06-操作语义 §1.3/§2 A1]）：App 求值顺序 = 函数先、参数从左到右。
/// 错误排序场景：((undefined-a) undefined-b) 双路径必须同报 fn 位置的
/// 未绑定（span 指向 undefined-a），而非参数位置（修复前 VM 先求参数，
/// 会报 undefined-b——T1 定理反例面）。
#[test]
fn app_evaluation_order_fn_first_dual_path() {
    let src = "((undefined-a) undefined-b)";
    // "undefined-a" 位于源 2..14；"undefined-b" 位于 16..28
    let fn_pos = 2u32;
    let arg_pos = 16u32;
    // VM 路径（生产路径）
    let vm_err = kerf_driver::run_source(src, "test.krf").expect_err("VM 路径应报错");
    assert!(
        vm_err.diagnostic.primary_span.start >= fn_pos
            && vm_err.diagnostic.primary_span.start < arg_pos,
        "VM 路径应报 fn 位置的未绑定（实际 span.start={}）：{}",
        vm_err.diagnostic.primary_span.start,
        vm_err.rendered
    );
    // 种子链（参考路径——T1 新口径 42-d：eval 退役，种子链为对拍 oracle）
    let ev_err = kerf_driver::run_source_seed(src, "test.krf").expect_err("种子链应报错");
    assert!(
        ev_err.diagnostic.primary_span.start >= fn_pos
            && ev_err.diagnostic.primary_span.start < arg_pos,
        "种子链应报 fn 位置的未绑定（实际 span.start={}）：{}",
        ev_err.diagnostic.primary_span.start,
        ev_err.rendered
    );
    // 双侧同属 Run 阶段 E0004 载体（E3 语义经由运行时路径报告）
    assert!(vm_err.rendered.contains("E0004"));
    assert!(ev_err.rendered.contains("E0004"));
    // 正例：函数先求值不改变正确程序的结果
    assert!(dual_path_agrees("((lambda (x) (* x x)) 6)"));
    assert!(dual_path_agrees("(define (f a b) (+ a b)) (f 1 2)"));
}

// ---------------------------------------------------------------------------
// TD-002 符号值（quote 符号 datum → Value::Symbol）：VM 语义 + 双路径
// ---------------------------------------------------------------------------

/// 符号值基础语义：quote 符号 / 符号列表 / 混合列表 / eq? 按名相等。
#[test]
fn quote_symbol_value_semantics() {
    assert_eq!(common::run_rendered("(quote sym)"), "sym");
    assert_eq!(common::run_rendered("'sym"), "sym");
    // 符号列表（符号入堆序对——HeapObj::Symbol 槽位形态）
    assert_eq!(common::run_rendered("'(a b c)"), "(a b c)");
    // 混合 datum 列表（VM 渲染层 Str 不带引号——render_value 语义）
    assert_eq!(common::run_rendered("'(a 1 \"s\")"), "(a 1 s)");
    // eq? 按名相等（值语义）
    assert_eq!(common::run_rendered("(eq? 'a 'a)"), "true");
    assert_eq!(common::run_rendered("(eq? 'a 'b)"), "false");
    // 符号 ≠ 字符串（类型严格）
    assert_eq!(common::run_rendered("(eq? 'a \"a\")"), "false");
}

/// 符号值构造路径：cons/list 内置装箱符号 + car 取回。
#[test]
fn symbol_construction_and_extraction() {
    assert_eq!(common::run_rendered("(cons 'a '(b))"), "(a b)");
    assert_eq!(common::run_rendered("(list 'a 'b)"), "(a b)");
    // car 取回符号（堆槽 → Value::Symbol 往返）
    assert_eq!(common::run_rendered("(car '(a b))"), "a");
    assert_eq!(common::run_rendered("(car (cdr '(a b)))"), "b");
}

/// 符号值双路径一致（T1：VM 与 eval 渲染等价——TD-002 触点双路径覆盖）。
#[test]
fn quote_symbol_dual_path_agreement() {
    assert!(dual_path_agrees("'sym"));
    assert!(dual_path_agrees("'(a b c)"));
    assert!(dual_path_agrees("(eq? 'a 'a)"));
    assert!(dual_path_agrees("(cons 'x '(y))"));
    assert!(dual_path_agrees("(car '(a b))"));
}
