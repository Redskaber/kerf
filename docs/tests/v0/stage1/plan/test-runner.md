# test-runner 测试计划（r8 批次 D）

> **Author**: kerf-dev-agent
> **Date**: 2026-09-10
> **Version**: v0.1.0-r8
> **Status**: Active

## 1. 测试目标

对应 [`tests/v0/stage1/plan/test_runner_tests.rs`](../../../../../../tests/v0/stage1/plan/test_runner_tests.rs)
的双向印证文档（sop.md §9.2 规则 1）。覆盖 `kerf test` 用例运行器
（`test_source` API）的语义契约：前置/用例切分、PASS 判定（值非 #f）、
效应短路 + 错误恢复 + 状态隔离、front 错误面（整体 DriverError）。

## 2. 覆盖场景

见代码文件内每个 `#[test]` 的文档注释。核心维度：
- 约定面正向：单/多用例 PASS + 源序 + 非 bool 真值（42/nil）+
  前置 define 逐用例重放 + require/set! 为前置 + 空报告（6 case）；
- 断言失败：`(= 1 2)` / `false` 字面量 → FAIL 含 detail（2 case）；
- **短路**：case 内首错即停（后续表达式不执行——副作用不外泄）；
- **恢复**：首 case 失败后下一 case 续跑（效应消费面核心）；
- **深位失败**：三层嵌套函数深层错误直达边界（零签名污染）；
- **状态隔离**：case 内 set! 不泄漏进后续 case（前置独立重放）；
- 报告面：全败计数 / 用例名渲染 / all_passed 语义；
- front 错误面：read（未闭合）/ expand（require 形状）/ R9（E0006）
  三类整体 DriverError（非用例 FAIL——与运行器边界清晰）。

## 3. 测试统计

见 [docs/tests/matrix.md](../../../matrix.md)（18 函数：正向 6 +
负向/边界 12——套件内正负 1:2，全局维持 ≈1:3.15）。

口径注记：`run_case` 的「用例编译失败」分支为防御路径（front 阶段
read/expand 已拦截形状错误）——该分支正确性由类型系统穷尽匹配保证，
不人工构造（记于文件头注释）。

## 4. 依赖

- 上游：kerf-driver::test_source（前置切分 + 效应边界）+ effects.rs
  （handle_escape/perform_escape——一次性逃逸层）
- 设计锚：[11-测试 §4](../../../../lang-design/11-testing.md)
  （用例运行器约定）+ [13-能力矩阵 §3.1.1](../../../../lang-design/13-capability-matrix.md)
  r8 注记（效应系统消费面）
- 生产路径复用：编译缓存内容寻址（§11 无平行语义）
