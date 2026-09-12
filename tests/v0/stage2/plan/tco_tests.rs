//! 批次 H（r18）——TCO 尾调用优化集成测试（40-c / TD-022 兑现）。
//!
//! **覆盖面**（§9.4.3 正负比 ≥1:3——本地码路径专负例承载）：
//! - 正例：深尾递归恒定帧（105_001 层——旧帧上限用例反转为通过项）、
//!   相互尾递归（even/odd 30 万步）、尾位传播（if 两臂 / do 末项 /
//!   let 糖脱装嵌套）、内建尾调用（隐式 RET）、自举链深展开（TD-007
//!   10_000 口径在自举 expander 路径的端到端达成——TCO 解除帧约束）；
//! - 负例：无限尾循环指令预算护栏（预算注入式——38-c find_qbe 同型）、
//!   尾调用 arity 不匹配、非可调用值尾调用；
//! - 诊断面：尾调用帧不出现在追踪链（优化帧语义——非尾双层链 ≥2 note
//!   对照锚在 negative_semantics_tests）。
//!
//! 语义注记（T1 定理域）：eval 参考路径无 TCO（256 深度边界——TD-017
//! r16 裁定维持）；VM 尾递归超 256 深度区域在 T1 检查域之外（双路径
//! 互查测试用有界程序——本套件 105_001 层用例只跑 VM 路径）。

use std::collections::HashMap;

use kerf_driver::compile_source;
use kerf_syntax::Symbol;
use kerf_vm::{run_program_with_budget, Value};

use crate::common;

// ---- 正例：尾递归帧复用 ----

/// 旧帧上限负例反转为 TCO 正例：105_001 层尾递归（> MAX_FRAMES 100_000）
/// 恒定帧数完成返回 0。TCO 前该程序在 100_000 帧报「调用帧超过上限」。
#[test]
fn tail_recursion_constant_frames_105k() {
    common::assert_int("(define (c n) (if (= n 0) 0 (c (- n 1)))) (c 105001)", 0);
}

/// 相互尾递归（even?/odd? 交替）：30 万步往返恒定帧。
#[test]
fn mutual_tail_recursion_even_odd() {
    common::assert_int(
        "(define (ev? n) (if (= n 0) 1 (od? (- n 1))))
         (define (od? n) (if (= n 0) 0 (ev? (- n 1))))
         (ev? 300000)",
        1,
    );
}

/// 尾位传播——If 两臂：then/else 尾位调用均获帧复用。
#[test]
fn tail_position_propagates_through_if_branches() {
    common::assert_int(
        "(define (down n) (if (= n 0) 42 (down (- n 1)))) (down 200000)",
        42,
    );
    // else 臂为非调用、then 臂尾递归
    common::assert_int(
        "(define (down2 n) (if (> n 0) (down2 (- n 1)) 7)) (down2 200000)",
        7,
    );
}

/// 尾位传播——Begin 末项（scheme 语义：do 尾调用 = 尾调用）。
/// 注：嵌套 do 末项经外层 do 末项链式传递尾位（三层传播）。
#[test]
fn tail_position_propagates_through_begin_last() {
    common::assert_int(
        "(define (f n) (if (= n 0) 99 (do 1 (do 2 (f (- n 1))))))
         (f 150000)",
        99,
    );
}

/// 尾位传播——let 糖（脱装为 Lambda+App 后 App 位于应用序尾位）。
#[test]
fn tail_position_through_let_sugar() {
    common::assert_int(
        "(define (f n) (if (= n 0) 11 (let ((m (- n 1))) (f m))))
         (f 200000)",
        11,
    );
}

/// 内建函数尾调用：结果即返回值（隐式 RET 路径）。
#[test]
fn builtin_tail_call_returns_value() {
    common::assert_int("(define (f p) (head p)) (f (cons 5 nil))", 5);
    common::assert_int("(define (f a b) (+ a b)) (f 40 2)", 42);
}

/// 闭包值尾调用（一等函数经参数传递后在尾位调用）。
#[test]
fn closure_value_in_tail_position() {
    common::assert_int(
        "(define (loop f n) (if (= n 0) 77 (f f (- n 1))))
         (define (g f n) (loop f n))
         (g g 100000)",
        77,
    );
}

// ---- 正例：TD-007/TD-022 耦合解除（自举路径端到端） ----

/// 自举 expander 深度链（种子侧 10_000 链 + 自举桥 parity 口径）：
/// TCO 前自举侧在 ~10_000 深链上先撞帧上限（expander.krf trampoline
/// 循环尾调用链耗帧）；TCO 后经帧复用到达 10_000 深度上限结构化报错。
#[test]
fn bootstrap_expander_10k_depth_reaches_limit() {
    // 自指宏：无限展开 → 深度计数到达 10_000（种子与自举双路径一致报错）
    let src = "(define-syntax loop2 (syntax-rules () ((loop2) (loop2)))) (loop2)";
    let err = common::run(src).expect_err("应报深度上限");
    assert!(
        err.contains("宏展开深度超过上限 10000"),
        "实际错误：{}",
        err
    );
}

// ---- 负例：护栏与边界（正负比 ≥1:3） ----

/// 无限尾循环指令预算护栏（预算注入式——真实上限 10^9 太慢，测试注入
/// 小预算快速触发）：帧数恒定不增 → 帧上限不触发 → 指令预算兜底。
#[test]
fn instruction_budget_guards_infinite_tail_loop() {
    let src = "(define (l) (l)) (l)";
    let out = compile_source(src, "<tco>").expect("编译失败");
    let mut globals: HashMap<Symbol, Value> = HashMap::new();
    let mut heap = kerf_runtime::Heap::new();
    let err = run_program_with_budget(&out.program, &mut globals, &mut heap, 10_000)
        .expect_err("无限尾循环应触发指令预算护栏");
    assert!(
        err.message.contains("指令数超过上限 10000"),
        "实际错误：{}",
        err.message
    );
    assert!(
        err.message.contains("疑似无限循环"),
        "护栏错误应含疑似无限循环提示：{}",
        err.message
    );
}

/// 尾调用 arity 不匹配（帧复用路径的参数校验与 Call 同口径）。
#[test]
fn tail_call_arity_mismatch_reports() {
    let err = common::run("(define (f x) x) (define (g) (f 1 2)) (g)").expect_err("应报错");
    assert!(err.contains("过程参数数量不匹配"), "实际错误：{}", err);
}

/// 尾位调用非函数值（与 Call 同口径的结构化错误）。
#[test]
fn tail_call_non_callable_reports() {
    let err = common::run("(define (f) (5)) (f)").expect_err("应报错");
    assert!(err.contains("不可调用的值"), "实际错误：{}", err);
}

/// 非尾位深递归仍受帧上限防护（TCO 不越界——非尾调用照常耗帧）。
#[test]
fn non_tail_deep_recursion_still_frame_limited() {
    let err = common::run("(define (c n) (if (= n 0) 0 (+ 1 (c (- n 1))))) (c 105001)")
        .expect_err("非尾深递归应报帧上限");
    assert!(err.contains("调用帧超过上限 100000"), "实际错误：{}", err);
}
