# stdlib 测试计划（r5 批次 B）

> **Author**: kerf-dev-agent
> **Date**: 2026-09-10
> **Version**: v0.1.0-r5
> **Status**: Active

## 1. 测试目标

对应 [`tests/v0/stage1/plan/stdlib_tests.rs`](../../../../../../tests/v0/stage1/plan/stdlib_tests.rs)
的双向印证文档（sop.md §9.2 规则 1）。覆盖标准库最小集 24 新函数
（列表 8 / 字符串 10 / I/O 6）+ TD-002 符号值消费端语义。

## 2. 覆盖场景

见代码文件内每个 `#[test]` 的文档注释。核心维度：
- 正向语义（16+17+6 断言 + Racket 语义边界注记 4）
- 负向矩阵（元数 25 / 类型 86 含全扫描 / 边界 12 / 断言语义 3+）
- 双路径一致（T1：14 断言纯函数子集）
- `eq?` 语义边界（堆值按引用 / 无数值塔 / 字符串按内容 / 符号按名）

## 3. 测试统计

见 [docs/tests/matrix.md](../../../matrix.md)（15 函数 / 172 负 case）。

## 4. 依赖

- 上游：kerf-driver 全管线（run_source）+ builtins.rs（r5 扩展）
- 共享：tests/common/mod.rs（run_rendered / dual_path_agrees）
- 通道层：kerf-runtime::write_stdout（r5 新增）
