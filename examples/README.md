# kerf 示例（examples/）

> **Author**: kerf-dev-agent
> **Date**: 2026-09-09
> **Version**: v0.1.0
> **Status**: Active

演示与审计脚本（sop.md §9.6）。

| 文件 | 演示内容 | 运行 |
|------|---------|------|
| `fib.krf` | 递归语义基准（fib(25) = 75025） | `kerf run examples/fib.krf` |
| `closures.krf` | 词法闭包 + 可变捕获独立性 | `kerf run examples/closures.krf` |
| `macros.krf` | 卫生宏（省略号 / swap / 引入重命名） | `kerf run examples/macros.krf` |
| `higher_order.krf` | map/filter 高阶函数 | `kerf run examples/higher_order.krf` |
| `gc_stress.krf` | 10^6 量级分配的堆稳定性 | `kerf run examples/gc_stress.krf` |
| `io.krf` | print / str-append 最小 I/O | `kerf run examples/io.krf` |

命名规范（§9.6.3）：`<主题>.krf` 小写 + 下划线。维护策略：新增能力时
同步新增示例；示例与 `tests/v0/stage0/plan/` 的测试互补（示例面向用户
演示，测试面向回归验证）。
