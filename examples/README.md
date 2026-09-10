# kerf 示例（examples/）

> **Author**: kerf-dev-agent
> **Date**: 2026-09-10
> **Version**: v0.1.0
> **Status**: Active

演示与审计脚本（sop.md §9.6 目录规范：usage/ 长期保留 + audit/ 阶段审查归档）。

## usage/ —— 语言演示（`*.krf`，cargo CLI 运行）

| 文件 | 演示内容 | 运行 |
|------|---------|------|
| `fib.krf` | 递归语义基准（fib(25) = 75025） | `kerf run examples/usage/fib.krf` |
| `closures.krf` | 词法闭包 + 可变捕获独立性 | `kerf run examples/usage/closures.krf` |
| `macros.krf` | 卫生宏（省略号 / swap / 引入重命名） | `kerf run examples/usage/macros.krf` |
| `higher_order.krf` | map/filter 高阶函数 | `kerf run examples/usage/higher_order.krf` |
| `gc_stress.krf` | 10^6 量级分配的堆稳定性 | `kerf run examples/usage/gc_stress.krf` |
| `io.krf` | print / str-append 最小 I/O | `kerf run examples/usage/io.krf` |

## audit/ —— 阶段审查脚本（`*.rs`，cargo example 运行，历史归档）

| 文件 | 审计内容 | 运行 |
|------|---------|------|
| `stage0_gate_audit_r1.rs` | §7.3.1 门审计集（41 case：负向 32 + 恢复 6 + 正向 3；§7.1.1 七类全覆盖） | `cargo run --example stage0_gate_audit_r1` |

命名规范（§9.6.3）：语言演示 `<主题>.krf`（小写 + 下划线）；阶段审查
`stage<N>_gate_audit_r<R>.rs`。维护策略：usage/ 随新能力同步新增（API
变更必须同步更新）；audit/ 为轮次归档，阶段闭合后不再扩展（§9.6.5）。
