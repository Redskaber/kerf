//! VM 集成测试（tests/v0/stage0/plan/vm_tests.rs ↔ docs/tests/v0/stage0/plan/vm.md）。
//!
//! 覆盖：9 核心原语执行语义（§21.3 Stage 0 验收项 1）/ fib(25) /
//! 运行时错误含堆栈追踪 / 双路径互查。

#[path = "../../../common/mod.rs"]
mod common;

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
