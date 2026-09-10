//! 共享测试辅助（sop.md §9.1 规则 4：tests/common/ 放共享辅助）。
//!
//! 注：不同测试目标消费不同子集——未用助手按公共库语义允许 dead_code。

use kerf_driver::{run_source, run_source_rendered, RunOutcome};
use kerf_vm::Value;

/// 编译 + VM 执行（返回最终值）。
#[allow(dead_code)]
pub fn run(src: &str) -> Result<Value, String> {
    match run_source(src, "test.krf") {
        Ok(o) => Ok(o.value),
        Err(e) => Err(e.to_string()),
    }
}

/// 编译 + VM 执行（返回渲染值 + 执行后堆——GC 断言用）。
#[allow(dead_code)]
pub fn run_with_heap(src: &str) -> Result<RunOutcome, String> {
    run_source(src, "test.krf").map_err(|e| e.to_string())
}

/// 编译 + 执行（渲染字符串——快照测试的黄金输出）。
#[allow(dead_code)]
pub fn run_rendered(src: &str) -> String {
    run_source_rendered(src, "test.krf").unwrap_or_else(|e| format!("<error:{}>", e))
}

/// 双路径互查：VM 与 eval 的渲染结果必须一致（§21.8 Phase 1 核心验证）。
#[allow(dead_code)]
pub fn dual_path_agrees(src: &str) -> bool {
    let vm = match run_source_rendered(src, "test.krf") {
        Ok(s) => s,
        Err(e) => format!("<error:{}>", e),
    };
    let ev = kerf_driver::eval_source(src, "test.krf")
        .map(|o| kerf_vm::render_value(&o.value, &o.heap))
        .unwrap_or_else(|e| format!("<error:{}>", e));
    vm == ev
}

/// 断言运行结果为指定整数。
#[allow(dead_code)]
pub fn assert_int(src: &str, expected: i64) {
    match run(src) {
        Ok(Value::Int(v)) if v == expected => {}
        other => panic!("期望 {}，实际 {:?}", expected, other),
    }
}
