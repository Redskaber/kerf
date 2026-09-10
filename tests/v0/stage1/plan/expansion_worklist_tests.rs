//! Stage 1 批次 A3（TD-007）集成测试：宏展开 trampoline 工作表的
//! 端到端验证——深宏链经展开→编译→VM 全管线。
//!
//! 单元级深链测试（2_000 层构造链）见 kerf-expander 内联；本文件
//! 覆盖真实源码路径（reader 嵌套上限 256 内的最大链深）。

use kerf_driver::{eval_source, run_source, Stage};
use kerf_vm::Value;

/// 正例：200 层嵌套透传宏链（源码嵌套 ≤ 256 reader 上限内）端到端
/// 展开（trampoline 迭代）→ 编译 → VM 执行 → 归约到字面量。
#[test]
fn deep_macro_chain_end_to_end_vm() {
    let depth = 200;
    let src = format!(
        "(define-syntax m (syntax-rules () ((m x) x))) (m {}42{})",
        "(m ".repeat(depth),
        ")".repeat(depth)
    );
    let o = run_source(&src, "chain.krf").unwrap();
    assert!(matches!(o.value, Value::Int(42)), "200 层宏链应归约到 42");
}

/// 双路径一致性：深宏链在 VM 与元循环求值器两条执行路径上结果一致
/// （T1 定理在 trampoline 展开下的保持）。
#[test]
fn deep_macro_chain_dual_path_agrees() {
    let depth = 100;
    let src = format!(
        "(define-syntax m (syntax-rules () ((m x) x))) (m {}42{})",
        "(m ".repeat(depth),
        ")".repeat(depth)
    );
    let a = run_source(&src, "chain.krf").unwrap();
    let b = eval_source(&src, "chain.krf").unwrap();
    assert!(a.value.eq_value(&b.value));
}

/// 负例：无限自指宏经完整管线报结构化错误（展开阶段 E2——超限
/// 报错而非栈溢出；错误阶段标识 = expand）。
#[test]
fn infinite_macro_reports_expand_stage_error() {
    let src = "(define-syntax lo (syntax-rules () ((lo) (lo)))) (lo)";
    let err = run_source(src, "inf.krf").unwrap_err();
    assert_eq!(err.stage, Stage::Expand);
    assert!(err.to_string().contains("展开深度"), "实际：{}", err);
}

/// 糖链混合：宏链与语法糖（let*/cond/while）交错展开——验证
/// trampoline 与糖推导通道的协作（宏产物进入糖推导、糖产物回到
/// trampoline 的双向流转）。
#[test]
fn macro_chain_interleaved_with_sugar() {
    let src = r#"
        (define-syntax wrap
          (syntax-rules ()
            ((wrap x) (let ((v x)) (+ v 0)))))
        (define (f n)
          (cond ((= n 0) 0)
                (else (+ n (f (- n 1))))))
        (wrap (f 10))
    "#;
    let o = run_source(src, "mix.krf").unwrap();
    assert!(matches!(o.value, Value::Int(55)), "1..10 求和 = 55");
}
