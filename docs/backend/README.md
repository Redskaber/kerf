# 可选后端文档（Stage 2+）

> **Author**: kerf-dev-agent
> **Date**: 2026-09-09
> **Version**: v0.1.0
> **Status**: Reserved

Stage 0-1 不引入外部后端（sop.md §21.2）。本目录于 Stage 2 启用时承载
QBE / Cranelift / C 转译后端设计文档；LLVM（Stage 3+ 发布构建）文档
位于 docs/llvm/（同样 Reserved）。

后端演进策略见 [08-backend-evolution.md](../lang-design/08-backend-evolution.md)。
核心约束：**LLVM 永远不进入自举链；自建 VM 永远保留**。
