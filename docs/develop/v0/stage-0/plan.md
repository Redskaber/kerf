# Stage 0 阶段计划

> **Author**: kerf-dev-agent（ARCH-A 角色）
> **Date**: 2026-09-09
> **Version**: v0.1.0
> **Status**: Active

## 1. 阶段目标（stage0.md §21.1 / sop.md §21.1）

**语义验证**：Rust（100%）实现 12（10+2）个能力模型，自建字节码 VM 后端。

- P0：Token Reader、图 IR、Span、诊断、GC、VM、结构化 CodeValue、元循环求值器、基础闭包
- P1：宏系统（卫生保证完整）、最小 I/O
- P2：编译缓存（接口预留 + 行为规格）
- P3：Effect Handlers / 多阶段编程 / 能力模型 I/O（仅类型定义）

## 2. MUV 拆分（sop.md §4）

| MUV | 输出物 | 验收标准 | Task ID |
|-----|--------|---------|---------|
| 环境与脚手架 | Cargo Workspace 9 crates + 根 crate | cargo build 成功；结构符合 §8.4.6 | 1, 3 |
| kerf-span | Span/SourceMap/Diagnostic | 单元测试全绿；Span merge/行号渲染正确 | 4-a |
| kerf-syntax | Symbol/Interner/ScopeSet/Stx | NFC 归一化一次；作用域集运算正确 | 4-a |
| kerf-core | CoreExpr 9 原语/图 IR/CodeValue | 共享字面量去重；自由变量绑定感知 | 4-a |
| kerf-reader | 词法器/语法器 | Token 无损覆盖；全部错误带 Span | 4-b |
| kerf-expander | 展开器/卫生宏/相位分离 | §3.2 推导表全量；卫生重命名一致 | 4-b |
| kerf-compiler | 字节码/闭包捕获/回填 | 回填零残留；常量池去重；栈平衡 | 4-c |
| kerf-runtime | GC 堆/最小 I/O | 显式工作栈；sweep free-list | 4-c |
| kerf-vm | 双执行路径 | fib(25)；帧三扩展槽；堆栈追踪 | 4-c |
| kerf-driver | 管线编排/内置/预留 | 四项预留冻结；卫生回退解析 | 4-c |
| 根 crate + 集成测试 | CLI + tests/v0/stage0 树 | ≥50 快照测试；§22.3 清单可执行审计 | 4-d |
| 验收与打包 | 验收记录 + tar.gz | §3.2 全绿；§19 打包 | 7, 9 |

## 3. 验收标准（sop.md §21.3 Stage 0）

1. 9 原语语义正确（`tests/v0/stage0/plan/vm_tests.rs::nine_primitives_semantics`）
2. 50+ 快照测试通过（实际 200 项——见 docs/tests/matrix.md）
3. 自举测试：管线确定性（两次编译字节码一致）——Rust 宿主下的 Stage 0 形态
4. Span 全管线传播（`pipeline_tests.rs::span_propagates_through_all_stages`）
5. 性能基准基线建立（fib(25)：84.7ms/轮 release）
6. 四项接口预留冻结（`gate_review_r1.rs::gate_g7_to_g10_reserved_interfaces`）

## 4. 依赖（§18.5 DAG）

kerf-span（基座）→ kerf-syntax → {kerf-reader, kerf-core}；kerf-core → kerf-expander；
kerf-core → kerf-compiler（Op/字节码定义）；kerf-runtime（Layer 0 基座）← kerf-vm；
kerf-vm 依赖 compiler+runtime；kerf-driver 编排全部。

> §18.5 图中 L6→L7（vm→runtime）边按 §2.4.5「Layer 0 最小 Runtime 无依赖」
> 裁定为 kerf-vm 依赖 kerf-runtime——运行时为 VM 提供对象模型与 GC 基座。
> 详细论证见 docs/graph/pipeline/dependency-graph.md。
