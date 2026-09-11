//! TD-021 prelude 模块注入套件（Stage 1 批次 E / Task 34-c）。
//!
//! 验收门（tech-debt-register TD-021 偿还——「hofs 用户面注入缺载体」）：
//! - **用户面可见**：用户程序 `(module 名 (import kerf-prelude) ...)`
//!   声明导入后，map/filter/foldl/for-each 以普通全局函数可调用（模块
//!   系统承载——forms 级合并注入，非源码拼接/跨程序合并/builtin 调
//!   闭包三否决路径）；
//! - **行为面**：注入的 hofs 与用户程序在同一 VM 程序内执行——双执行
//!   路径（run/eval）一致（T1 口径）；
//! - **显式失败**：同名 define → 「重复定义变量」结构化报错（不静默
//!   遮蔽——§2.3 原则 4）；
//! - **opt-in 语义**：无 import 声明 → hofs 保持未绑定（无隐式注入）。
//!
//! 遵循条款：§9.4.3（正负例成对）、§7.1（集成验证 ≥3）、§2.3-11
//! （先实测禁臆测——全部断言经真实管线运行）。

use kerf_driver::{eval_source, run_source, RunOutcome};

fn value_of(o: &RunOutcome) -> String {
    kerf_vm::render_value(&o.value, &o.heap)
}

#[test]
fn prelude_import_hofs_user_face() {
    // 用户面可见性：import 声明后 map/filter 可直接调用
    let src = r#"(module user (import kerf-prelude)
                  (define lst (list 1 2 3 4 5))
                  (map (lambda (x) (* x 2)) lst))"#;
    let o = run_source(src, "prelude.krf").expect("prelude 注入后应可运行");
    assert_eq!(value_of(&o), "(2 4 6 8 10)");
}

#[test]
fn prelude_composition_pipeline() {
    // 组合管道：filter → map → foldl（hofs 协同）
    let src = r#"(module user (import kerf-prelude)
                  (define lst (list 1 2 3 4 5))
                  (foldl + 0 (map (lambda (x) (* x x))
                                  (filter (lambda (x) (> x 2)) lst))))"#;
    let o = run_source(src, "prelude.krf").expect("组合管道应可运行");
    assert_eq!(value_of(&o), "50");
}

#[test]
fn prelude_for_each_side_effect() {
    // for-each 副作用序（遍历序求和——set! 观测）
    let src = r#"(module user (import kerf-prelude)
                  (define count 0)
                  (for-each (lambda (x) (set! count (+ count x)))
                            (list 1 2 3))
                  count)"#;
    let o = run_source(src, "prelude.krf").expect("for-each 应可运行");
    assert_eq!(value_of(&o), "6");
}

#[test]
fn prelude_dual_execution_paths_agree() {
    // T1 双路径一致：注入产物在 VM 与 eval 路径同果
    let src = r#"(module user (import kerf-prelude)
                  (foldl (lambda (acc x) (+ acc x)) 0 (list 1 2 3 4)))"#;
    let a = run_source(src, "prelude.krf").expect("VM 路径失败");
    let b = eval_source(src, "prelude.krf").expect("eval 路径失败");
    assert!(a.value.eq_value(&b.value), "双路径应一致");
    assert_eq!(value_of(&a), "10");
}

#[test]
fn prelude_import_without_module_stays_unbound() {
    // opt-in 语义：顶层直接引用（无 import 声明）→ 未绑定（无隐式注入）
    let err = run_source("(map (lambda (x) x) (list 1))", "no-import.krf")
        .expect_err("未 import 时 map 应未绑定");
    assert!(err.rendered.contains("未绑定"), "实际：{}", err.rendered);
}

#[test]
fn prelude_name_capture_fails_explicitly() {
    // 同名 define → 「重复定义变量」结构化报错（显式失败，不静默遮蔽）
    let src = r#"(module user (import kerf-prelude)
                  (define (map f lst) lst)
                  (map (lambda (x) x) (list 1)))"#;
    let err = run_source(src, "capture.krf").expect_err("同名 define 应显式报错");
    assert!(err.rendered.contains("重复定义"), "实际：{}", err.rendered);
}

#[test]
fn prelude_import_unknown_module_errors() {
    // 未知 import → 相位簿记显式报错（未声明的模块——registry.visit）
    let src = "(module user (import nonexistent-module) 1)";
    let err = run_source(src, "unknown.krf").expect_err("未知导入应报错");
    assert!(
        err.rendered.contains("未声明的模块"),
        "实际：{}",
        err.rendered
    );
}
