# gc 测试计划（开发轮）

> **Author**: kerf-dev-agent
> **Date**: 2026-09-09
> **Version**: v0.1.0
> **Status**: Active

## 1. 测试目标

对应代码文件 [`tests/v0/stage0/plan/gc_tests.rs`](../../../../../tests/v0/stage0/plan/gc_tests.rs)
的双向印证文档（sop.md §9.2 规则 1）。

## 2. 覆盖场景

见代码文件内每个 `#[test]` 的文档注释（目标/不变式/验收条款引用）。
核心场景：正向语义 + 快照黄金输出 + 负向错误路径（§9.4.3 负向优先）。

## 3. 测试统计

实际数量见 [docs/tests/matrix.md](../../../matrix.md) 分套件统计表。

## 4. 依赖

- 上游：kerf-driver 全管线（run_source / compile_source）
- 共享：tests/common/mod.rs（run / run_rendered / dual_path_agrees）
