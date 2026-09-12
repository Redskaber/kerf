# 工具链基础设施：LSP API、自调试与性能基准

> **Author**: kerf-doc-agent
> **Date**: 2026-09-11（v6.3：批次 G 增量——§2 CLI 11→13 子命令（anf/native——QBE 后端 PoC 面）+ check 恢复模式注记；v6.2：批次 F 深审回写——§2 补 CLI 11 子命令表面清单（B4 灰区收口））
> **Version**: v6.4（r34 / 55-a：LSP/格式化 × 命名空间三交互点注记（F10——22 机制设计的工具面接线）；v6.3：r17 CLI 13 子命令）
> **Status**: Active

> 本文件收录编译器工具链相关的三块外围能力：工具链基础设施（原 §14.4：编译器暴露内部 API 供 LSP/格式化器/linter 消费）、编译器自调试工具（原 §14.9：四层调试）、性能基准框架（原 §14.8：标准基准测试集）。三者均属外围能力层与操作基础层的"最小骨架"版本（处理程度分级见 [12-路线图 §2.1](./12-roadmap.md)）。测试基础设施见 [11-测试](./11-testing.md)；架构分层总览见 [15-架构分层](./15-architecture-layers.md)。

---

## 1. 工具链基础设施（原 §14.4）

编译器必须暴露内部 API 供 LSP 服务器、格式化器和 linter 消费。

**架构含义**（结合 [17-设计原则 §1 原则 13/14](./17-principles.md)）：
- **目标中立性（原则 13）**：前端与目标无关，工具链消费的语法树与编译器内部的表示完全一致；
- **工具即编译器（原则 14）**：LSP 消费与编译器相同的语法树——rust-analyzer 是该模式的参考实现（[19-参考文献 §3.6 工具链](./19-references.md)）；
- LSP 集成按 [13-能力矩阵 §3.2](./13-capability-matrix.md) 推迟到 Stage 2 实现，但 v6.1 完整性审查将其自「完全推迟」**重分类为接口预留**（[13-能力矩阵 §3.3.2](./13-capability-matrix.md)：`LanguageService` 语法/语义查询 trait + `IncrementalAst` AST 增量接口，P0 级）——Stage 0 必须在 AST 数据结构中预留绑定查询与增量更新位置（Span 追溯已实现 ✓），否则后期引入需破坏性重构；
- **LSP/格式化 × 命名空间交互（v6.4 注记——r34 / 55-a F10 落位）**：v0.6 命名空间机制（[22-命名空间设计](./22-namespace-design.md)——限定名 `string/append` + 五层 N0-N4）与工具链接口预留存在三个已识别交互点，实现期（Stage 2+ LSP 落地时）必须逐点核对：①**补全**（completion 须感知 N2 模块面——候选含限定名形态与 import 注入名，而非仅 N1 全局名）；②**重命名**（rename 重构必须尊重 N2 边界——跨模块重命名牵涉 export 面与 import 面，非局部操作）；③**格式化**（formatter 须区分运算符 `/` 与标识符内分隔符 `/`——[22 §3.4 R-N4](./22-namespace-design.md) 词法域规则的工具面镜像）。数据结构影响：AST 增量接口的绑定查询键从纯 `Symbol` 扩展为「Symbol × 命名层位」——`IncrementalAst` 预留位不破坏（键空间扩展非结构变更），但实现期需 v0.6 后口径；治理 owner = [23-演进治理 §6](./23-evolution-governance.md)（实施对账表）。
- API 的暴露方式（编译即 API，原则 10）在 Stage 0 的接口设计中预留（原 v5.0 裁定不变）。

## 2. 编译器自调试工具（原 §14.9）

四层调试：IR dump 接口、阶段跟踪、编译器内省 API、交互式调试器。

**CLI 表面清单（v6.2 补写 B4 灰区收口；v6.3 r17 增量——批次 G +2）**：根 CLI（src/main.rs）实际暴露 **13 个子命令**——`run`（编译+执行）/ `eval`（eval 参考路径）/ `check`（静态检查——r7 类型检查器消费面；**r17 起恢复模式**：TD-013 形式级恢复，E0002 展开 + E0005 类型全量合并报告）/ `test`（用例运行器——r8 批次 D 交付，见 [11-测试 §5](./11-testing.md)）/ `tokens` / `stx`（Reader 两层 dump——四层调试的第 1 层）/ `core` / `ir` / `bc` / `code`（Expander/Compiler 四层 dump——第 2 层，经 driver 转发不越界 §11）/ `bench`（性能基准——本文 §3 的 CLI 载体）/ **`anf`（AnnotatedANF IR 摘要 dump——后端 lowering 面）/ `native`（QBE AOT 编译+运行——首个非 VM 后端，[13-能力矩阵 §3.3.7](./13-capability-matrix.md) 做实）**（r17 批次 G / 38-c 新增）。第 3/4 层（内省 API/交互式调试器）预留未实现（属 13 §3.3 LSP/DebugInfoGenerator 做实范围——Stage 2+）。

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
