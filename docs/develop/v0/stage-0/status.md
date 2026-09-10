# Stage 0 阶段状态报告

> **Author**: kerf-dev-agent（QA-A 角色）
> **Date**: 2026-09-09
> **Version**: v0.1.0
> **Status**: Active

## 1. 交付概览

| 指标 | 值 |
|------|-----|
| 代码规模（Rust，不含测试） | ~5200 行 |
| 测试总数 | **204 全绿**（单元 127 + 集成 77；v5.1 语义分裂修复 +4） |
| crates | 9 成员 + 1 根 crate（零外部依赖） |
| 操作码 | 39 个（§8.12 分组全覆盖） |
| 验收 | §3.2 全绿（build/check/test/fmt/clippy） |

## 2. 12（10+2）能力模型交付状态

| # | 能力 | P 级 | 状态 | 载体 |
|---|------|------|------|------|
| 1 | 类型化 Token 流 Reader | P0 | ✅ | kerf-reader（Token/TokenKind/词法器/语法器） |
| 2 | 图 IR（共享节点） | P0 | ✅ | kerf-core::ir（Arena + node_map 字面量共享） |
| 3 | 结构化 CodeValue | P0 | ✅ | kerf-core::code_value（良构/自由变量/α重命名/组合预留） |
| 4 | 元循环求值器 | P0 | ✅ | kerf-vm::eval（Rc 环境链） |
| 5 | 基础闭包 | P0 | ✅ | 共享单元格捕获（letrec 递归语义正确） |
| 6 | Span 全管线传播 | P0 | ✅ | Token→Stx→CoreExpr→IR→debug_spans |
| 7 | 结构化诊断 | P0 | ✅ | kerf-span::diagnostic（E0001-E0004 + 摘录渲染） |
| 8 | 最小 I/O | P1 | ✅ | read_line/write_line + print/read-line 内置 |
| 9 | 相位分离 | P0 | ✅ | declare/visit/instantiate 生命周期簿记 |
| 10 | 基础宏系统 | P1 | ✅ | syntax-rules + 内置糖 + 一致性卫生重命名 |
| 11 | 标记-清除 GC | P0 | ✅ | 分配驱动触发 + 冷却退避 + 显式工作栈 |
| 12 | 字节码 VM | P0 | ✅ | switch-dispatch + 三扩展槽帧 + 堆栈追踪 |

## 3. 四项接口预留（P2/P3 冻结）

Effect Handlers（P3）/ 多阶段编程（P3）/ 能力模型 I/O（P2）/ 编译缓存（P2）——
签名见 kerf-driver/src/reserved.rs，冻结性经 Probe 实现测试证明。

## 4. 性能基线（§14.8）

- fib(25)（含编译）：**84.7 ms/轮**（release，5 轮均值）
- GC 压力：3×10^5 临时分配 0.72s（堆有界；回收 >1 次）

## 5. 已知边界（TD 登记）

见 docs/develop/v0/tech-debt-register.md（TD-002~TD-011，全部 P2/P3 级，
无 P0/P1 遗留——阶段切换信号「技术债清零」条件满足）。
