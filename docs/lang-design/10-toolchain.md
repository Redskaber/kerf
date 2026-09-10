# 工具链基础设施：LSP API、自调试与性能基准

> **Author**: kerf-doc-agent
> **Date**: 2026-09-09
> **Version**: v5.0（源自 stage0.md v5.0 拆分）
> **Status**: Active

> 本文件收录编译器工具链相关的三块外围能力：工具链基础设施（原 §14.4：编译器暴露内部 API 供 LSP/格式化器/linter 消费）、编译器自调试工具（原 §14.9：四层调试）、性能基准框架（原 §14.8：标准基准测试集）。三者均属外围能力层与操作基础层的"最小骨架"版本（处理程度分级见 [12-路线图 §2.1](./12-roadmap.md)）。测试基础设施见 [11-测试](./11-testing.md)；架构分层总览见 [15-架构分层](./15-architecture-layers.md)。

---

## 1. 工具链基础设施（原 §14.4）

编译器必须暴露内部 API 供 LSP 服务器、格式化器和 linter 消费。

**架构含义**（结合 [17-设计原则 §1 原则 13/14](./17-principles.md)）：
- **目标中立性（原则 13）**：前端与目标无关，工具链消费的语法树与编译器内部的表示完全一致；
- **工具即编译器（原则 14）**：LSP 消费与编译器相同的语法树——rust-analyzer 是该模式的参考实现（[19-参考文献 §2 工具链](./19-references.md)）；
- LSP 集成本身按 [13-能力矩阵 §3.2](./13-capability-matrix.md) 完全推迟到 Stage 2（非核心功能，需要稳定的编译器 API），但 API 的暴露方式（编译即 API，原则 10）必须在 Stage 0 的接口设计中预留。

## 2. 编译器自调试工具（原 §14.9）

四层调试：IR dump 接口、阶段跟踪、编译器内省 API、交互式调试器。

**四层与既有设计的关系**：
- **IR dump 接口**：对应各阶段数据结构（Token 流 / Graph IR / CodeValue / 字节码）的序列化输出，Span 全管线传播（[02-语法模型 §2](./02-syntax-model.md)）保证 dump 可反查源码；
- **阶段跟踪**：Reader → Expander → Compiler → VM 的阶段边界计时（配合本文 §3 的编译速度基准）；
- **编译器内省 API**：编译即 API（[17-设计原则 §1 原则 10](./17-principles.md)：所有阶段可独立调用）的自然延伸；
- **交互式调试器**：依赖 VM 的 debug_info_table（[15-架构分层 §4.7](./15-architecture-layers.md)：字节码地址 → 源码位置映射）与三扩展槽中的调试帧槽（[04-字节码 VM §1](./04-bytecode-vm.md) ext3）。

## 3. 性能基准框架（原 §14.8）

标准基准测试集：编译速度（lex/parse/expand/compile）、执行速度（fib/loop/alloc）、自举基准。

**三类基准的度量对象**：

| 基准类别 | 度量对象 | 示例 |
|---------|---------|------|
| 编译速度 | 管线各阶段吞吐 | lex / parse / expand / compile |
| 执行速度 | VM 与运行时性能 | fib / loop / alloc |
| 自举基准 | 编译器编译自身的端到端成本 | 编译器源码全集 |

基准数据是后续阶段决策的输入：Stage 3 优化器的引入需要性能基准数据（[13-能力矩阵 §3.2](./13-capability-matrix.md)），JIT 依赖 profiling 数据；基准基线的建立是 Stage 0 Week 4 的交付物之一（[12-路线图 §1 周级任务表](./12-roadmap.md)）。相关参考数据（Cone 前后端耗时占比、PyPy 元追踪、QBE/Cranelift）见 [08-后端演化](./08-backend-evolution.md) 与 [19-参考文献 §1 参考案例](./19-references.md)。
