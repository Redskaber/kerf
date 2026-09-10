# Stage 1 测试计划（批次级）

> **Author**: kerf-dev-agent
> **Date**: 2026-09-10（r5）
> **Version**: v0.1.0-r5
> **Status**: Active

## 套件

| 套件 | 文件 | 覆盖 |
|------|------|------|
| expansion_worklist_tests（r4） | tests/v0/stage1/plan/expansion_worklist_tests.rs | TD-007 trampoline：200 层链端到端 / 双路径 / 自指宏报错 / 糖交错 |
| stdlib_tests（r5） | tests/v0/stage1/plan/stdlib_tests.rs | TD-002 符号值消费 + 标准库最小集 24 函数：正例 59 断言 + 负例 172 case（元数/类型/边界矩阵 + 类型全扫描 + Racket 语义边界注记） |

详见各文件内文档注释与 [docs/tests/matrix.md](../../matrix.md) 分套件统计。
