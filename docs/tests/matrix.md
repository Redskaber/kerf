# 全局测试矩阵（覆盖率追踪）

> **Author**: kerf-dev-agent（QA-A 角色）
> **Date**: 2026-09-09
> **Version**: v0.1.0
> **Status**: Active

## 总量

**204 通过 / 0 失败 / 0 忽略**（§3.2 release 验收实测；v5.1 语义分裂修复新增 4 项双路径对账测试）

## 分套件统计

| 套件 | 层级 | 文件/位置 | 测试数 |
|------|------|----------|--------|
| kerf-span 单元 | crate 内联 | crates/kerf-span/src/*.rs | 10 |
| kerf-syntax 单元 | crate 内联 | crates/kerf-syntax/src/*.rs | 11 |
| kerf-core 单元 | crate 内联 | crates/kerf-core/src/*.rs | 11 |
| kerf-reader 单元 | crate 内联 | crates/kerf-reader/src/*.rs | 23 |
| kerf-expander 单元 | crate 内联 | crates/kerf-expander/src/*.rs | 23 |
| kerf-compiler 单元 | crate 内联 | crates/kerf-compiler/src/*.rs | 12 |
| kerf-runtime 单元 | crate 内联 | crates/kerf-runtime/src/*.rs | 9 |
| kerf-vm 单元 | crate 内联 | crates/kerf-vm/src/*.rs | 14 |
| kerf-driver 单元 | crate 内联 | crates/kerf-driver/src/*.rs | 14 |
| reader_tests | 根项目集成 | tests/v0/stage0/plan/reader_tests.rs | 10 |
| expander_tests | 根项目集成 | tests/v0/stage0/plan/expander_tests.rs | 16 |
| compiler_tests | 根项目集成 | tests/v0/stage0/plan/compiler_tests.rs | 8 |
| vm_tests | 根项目集成 | tests/v0/stage0/plan/vm_tests.rs | 10 |
| gc_tests | 根项目集成 | tests/v0/stage0/plan/gc_tests.rs | 6 |
| pipeline_tests | 根项目集成 | tests/v0/stage0/plan/pipeline_tests.rs | 11 |
| gate_review_r1 | 阶段门审计 | tests/v0/stage0/gate/gate_review_r1.rs | 12 |

## 需求覆盖（sop.md §21.3 Stage 0 验收标准 → 测试）

| 验收项 | 覆盖测试 |
|--------|---------|
| (1) 9 原语语义正确 | vm_tests::nine_primitives_semantics + gate_g1/g2 |
| (2) 50+ 快照测试 | 全套件（204 ≥ 50） |
| (3) 自举测试（同结果） | compiler_tests::deterministic + pipeline_tests::convergent |
| (4) Span 全管线传播 | pipeline_tests::span_propagates + gate_g6 |
| (5) 性能基准基线 | bench CLI（fib(25) 84.7ms/轮） |
| (6) 四项接口预留冻结 | gate_g7_to_g10 + reserved.rs Probe 测试 |

## 测试类型分布

- 快照/黄金输出：Token 快照 / Stx 渲染 / CoreExpr 渲染 / 反汇编 / 运行结果渲染
- 语义断言：9 原语 / fib / 闭包 / 宏 / quote / GC
- 负向/错误：词法 / 语法 / 展开 / 编译 / 运行时 全阶段错误路径
- 双路径互查：VM vs eval（9 程序逐字节一致）
- 压力/稳健：3×10^5 分配 GC / 10^4 深递归 / 2×10^5 深链标记
