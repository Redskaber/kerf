# Stage 1 测试计划（批次级）

> **Author**: kerf-dev-agent
> **Date**: 2026-09-10（r6）
> **Version**: v0.1.0-r6
> **Status**: Active

## 套件

| 套件 | 文件 | 覆盖 |
|------|------|------|
| expansion_worklist_tests（r4） | tests/v0/stage1/plan/expansion_worklist_tests.rs | TD-007 trampoline：200 层链端到端 / 双路径 / 自指宏报错 / 糖交错 |
| stdlib_tests（r5） | tests/v0/stage1/plan/stdlib_tests.rs | TD-002 符号值消费 + 标准库最小集 24 函数：正例 59 断言 + 负例 172 case（元数/类型/边界矩阵 + 类型全扫描 + Racket 语义边界注记） |
| bootstrap_reader_tests（r6） | tests/v0/stage1/plan/bootstrap_reader_tests.rs | **B3 自举 Reader parity**：正例 87 case（Stx 树含 Span 递归等价、符号按名）+ 负例 307 case（错误消息+Span 逐字节等价、`assert_negative_parity` 防误收）+ 双错误次序（首错位置契约）+ Unicode/NFC + 深度边界 256/257 + Token 流/dump 格式 parity + 高阶函数直测（map/filter/foldl/for-each，builtin 作 f）+ Reader 原语（str->pos-chars/char-whitespace?/char-alphabetic?/str-int-valid? 含误用）+ 管线集成（run/eval 双路径 + 读错误渲染） |

详见各文件内文档注释与 [docs/tests/matrix.md](../../matrix.md) 分套件统计。
