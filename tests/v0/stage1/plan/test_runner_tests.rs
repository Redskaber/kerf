//! 用例运行器集成测试（r8 批次 D——D1 消费面：`test_source` 约定锁定）。
//!
//! 覆盖面（§9.4.3——正负比负向为主；全局面 1089/346 ≈ 1:3.15 维持）：
//! - **约定面**（零新语法——测试面即语言面）：前置形式
//!   （define/set!/module/require）= 公共前置逐用例重放；其余顶层
//!   表达式 = 用例（求值无错且值非 #f = PASS）；
//! - **效应消费面**（effects.rs 的端到端证明）：短路（case 内任意深度
//!   失败即停该 case）+ 恢复（边界捕获后下一 case 续跑）+ 状态隔离
//!   （每 case 全新全局环境与堆——失败 case 不毒化后续 case）；
//! - **报告面**：逐用例结果/计数/名称渲染/源序保持；
//! - **front 错误面**：源级语法错误 → 整体 DriverError（非用例 FAIL）。
//!
//! 注：`run_case` 的「用例编译失败」分支为防御路径（front 阶段
//! read/expand 已拦截形状错误，compile_module 对合法核心表达式不产
//! 结构性错误）——该分支的正确性由类型系统穷尽匹配保证，本套件不
//! 人工构造（记于 docs/tests/v0/stage1/plan/test-runner.md 口径节）。

use kerf_driver::{test_source, Stage};

const FNAME: &str = "runner.krf";

fn report(src: &str) -> kerf_driver::TestReport {
    test_source(src, FNAME).expect("测试源应可编译执行")
}

// ---------------------------------------------------------------------------
// 正向：约定面（PASS 判定 + 前置切分 + 报告计数）
// ---------------------------------------------------------------------------

/// 单用例断言真 → PASS（`(= a b)` 谓词即断言）。
#[test]
fn single_assertion_case_passes() {
    let r = report("(= 1 1)");
    assert_eq!(r.cases.len(), 1);
    assert!(r.cases[0].pass);
    assert_eq!(r.passed, 1);
    assert_eq!(r.failed, 0);
    assert!(r.all_passed());
}

/// 多用例全过（源序保持——index 1..n）。
#[test]
fn multiple_cases_all_pass_in_order() {
    let r = report("(= 1 1) (= 2 2) (= 3 3)");
    assert_eq!(r.cases.len(), 3);
    for (i, c) in r.cases.iter().enumerate() {
        assert!(c.pass, "用例 {} 应通过", i + 1);
        assert_eq!(c.index, i + 1, "用例序号应源序 1 起");
    }
    assert_eq!(r.passed, 3);
}

/// 非 bool 真值（42 / nil）→ PASS（约定 = 值非 #f 即过）。
#[test]
fn non_false_truthy_values_pass() {
    let r = report("42 nil");
    assert_eq!(r.cases.len(), 2);
    assert!(r.cases[0].pass);
    assert!(r.cases[1].pass);
}

/// 前置 define 逐用例重放：用例是前置的纯函数（状态隔离正向面）。
#[test]
fn prelude_define_shared_across_cases() {
    let src = "(define x 10) (= x 10) (= (+ x 1) 11)";
    let r = report(src);
    assert_eq!(r.cases.len(), 2, "define 为前置，不作用例");
    assert!(r.all_passed());
}

/// require/assign 为前置形态（不作用例计数）。
#[test]
fn require_and_setbang_are_prelude_forms() {
    let src = "(define x 1) (assign x 2) (require io write) (= x 2)";
    let r = report(src);
    assert_eq!(r.cases.len(), 1, "仅最后一个 = 形式为用例");
    assert!(r.all_passed());
}

/// 空（纯前置）源：零用例 → all_passed（空集平凡通过）。
#[test]
fn prelude_only_source_has_zero_cases() {
    let r = report("(define a 1) (define b 2)");
    assert!(r.cases.is_empty());
    assert_eq!((r.passed, r.failed), (0, 0));
    assert!(r.all_passed());
}

// ---------------------------------------------------------------------------
// 负向：断言失败与运行时错误（FAIL 判定 + 短路 + 恢复）
// ---------------------------------------------------------------------------

/// 断言返回 #f → FAIL，detail 含断言语义。
#[test]
fn false_assertion_fails_with_detail() {
    let r = report("(= 1 2)");
    assert!(!r.cases[0].pass);
    assert_eq!(r.failed, 1);
    assert!(
        r.cases[0].detail.contains("#f"),
        "失败详情应含 #f 语义，实际：{}",
        r.cases[0].detail
    );
}

/// #f 字面量用例 → FAIL（非 bool 真值约定的对偶面）。
#[test]
fn literal_false_case_fails() {
    let r = report("false");
    assert!(!r.cases[0].pass);
}

/// 运行时错误（未绑定变量）→ FAIL 且 detail 含错误首行。
#[test]
fn runtime_error_case_fails_with_error_detail() {
    let r = report("undefined-variable");
    assert!(!r.cases[0].pass);
    assert!(
        !r.cases[0].detail.is_empty(),
        "运行时错误用例应携带失败详情"
    );
}

/// **错误恢复（效应消费面核心）**：首 case 失败后下一 case 续跑。
#[test]
fn failure_recovers_next_case_continues() {
    let src = "(= 1 2) (= 2 2) (= 3 3)";
    let r = report(src);
    assert_eq!(r.cases.len(), 3);
    assert!(!r.cases[0].pass);
    assert!(r.cases[1].pass, "失败 case 后续 case 应续跑（恢复）");
    assert!(r.cases[2].pass);
    assert_eq!((r.passed, r.failed), (2, 1));
}

/// **短路**：case 内首错即停——后续表达式不执行（副作用不外泄）。
#[test]
fn first_error_short_circuits_within_case() {
    // case = (do undefined-x (= 1 2))：首表达式错误即停，
    // 整 case 单条 FAIL（detail 为首错，非 #f 断言）
    let src = "(define x 1) (do undefined-x (= 1 2))";
    let r = report(src);
    assert_eq!(r.cases.len(), 1);
    assert!(!r.cases[0].pass);
    assert!(
        !r.cases[0].detail.contains("#f"),
        "短路语义：首错（未绑定）即停，不应到达 #f 断言，实际：{}",
        r.cases[0].detail
    );
}

/// **深位失败零签名污染**：嵌套函数深层错误直达边界（任意深度）。
#[test]
fn deep_nested_failure_reaches_boundary() {
    let src = r#"
        (define (level3) undefined-deep)
        (define (level2) (level3))
        (define (level1) (level2))
        (level1)
        (= 1 1)
    "#;
    let r = report(src);
    assert_eq!(r.cases.len(), 2, "define 全为前置");
    assert!(!r.cases[0].pass, "深层未绑定应判 FAIL");
    assert!(r.cases[1].pass, "深位失败后下一用例恢复");
}

/// **状态隔离**：失败 case 的脏状态不毒化后续 case（每 case 全新
/// 环境与堆——前置逐用例重放：case 1 内的 assign 不泄漏进 case 2）。
#[test]
fn failing_case_does_not_poison_following_cases() {
    let src = "(define x 5) (do (assign x 6) (= 1 2)) (= x 5)";
    let r = report(src);
    assert_eq!(r.cases.len(), 2, "define 为前置，两个 do/= 为用例");
    assert!(!r.cases[0].pass, "首用例 #f 断言应失败");
    assert!(
        r.cases[1].pass,
        "次用例独立重放前置（x=5）——首用例的 assign 不泄漏（隔离）"
    );
}

/// 全败报告：0 通过 / N 失败 / all_passed 为假。
#[test]
fn all_fail_report_counts() {
    let r = report("(= 1 2) false (= 3 4)");
    assert_eq!(r.cases.len(), 3);
    assert!((r.passed, r.failed) == (0, 3));
    assert!(!r.all_passed());
}

/// 用例失败 detail 非空 + 用例名含形式渲染。
#[test]
fn case_name_and_detail_rendered() {
    let r = report("(= 99 100)");
    assert!(
        r.cases[0].name.contains("="),
        "用例名应为形式渲染，实际：{}",
        r.cases[0].name
    );
    assert!(!r.cases[0].detail.is_empty());
}

// ---------------------------------------------------------------------------
// 负向：front 错误面（整体 DriverError——非用例 FAIL）
// ---------------------------------------------------------------------------

/// 源级语法错误（未闭合括号）→ 整体 Err（Stage::Read），报告不产出。
#[test]
fn syntax_error_returns_whole_file_error() {
    let e = test_source("(define x 1", FNAME).unwrap_err();
    assert_eq!(e.stage, Stage::Read);
}

/// require 形状错误 → 整体 Err（Stage::Expand——front 拦截先于用例切分）。
#[test]
fn malformed_require_returns_expand_error() {
    let e = test_source("(require io) (= 1 1)", FNAME).unwrap_err();
    assert_eq!(e.stage, Stage::Expand);
}

/// 未声明 I/O 的用例 → 整体 E0006（R9 front 单一验证点先于运行器）。
#[test]
fn undeclared_io_gates_whole_test_source() {
    let e = test_source("(print 1) (= 1 1)", FNAME).unwrap_err();
    assert_eq!(e.stage, Stage::Compile);
    assert!(e.rendered.contains("error[E0006]"));
}
