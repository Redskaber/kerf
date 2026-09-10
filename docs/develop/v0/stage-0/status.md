# Stage 0 阶段状态报告

> **Author**: kerf-dev-agent（QA-A 角色）
> **Date**: 2026-09-10（r3 修复轮 + 负测扩张 + 审计集就位后对账）
> **Version**: v0.1.0-r3
> **Status**: Active

## 1. 交付概览

| 指标 | 值 |
|------|-----|
| 代码规模（Rust 生产代码，crates + src，不含 tests/examples） | 见 RELEASE_NOTES（r3 末实测口径 `wc -l`） |
| 测试总数 | **297 全绿**（单元 130 + 集成 168 函数 = 298，其中 1 个 #[ignore] 文档化存档；0 失败） |
| 负向测试 | 四文件 86 函数 / **483 case** + 审计集 41 case（负 32）——正负比 **≈1:3.2**（§9.4.3 门限达标，r1 时 1:0.24） |
| 门审计集 | examples/audit/stage0_gate_audit_r1.rs：41 case（§7.3.1 配比全满足） |
| crates | 9 成员 + 1 根 crate（零外部依赖，DAG 无环） |
| 操作码 | **40 个**（八组显式枚举：栈 7/变量访问 7/控制流 2/函数 3/算术比较 12/数据 3/谓词 5/终止 1；守护测试逐项列举） |
| 内置函数 | 24 项（语言层 read-line/print；通道层 read_line_stdin/write_line_stdout） |
| 验收 | §3.2 全绿（build/check/test/fmt/clippy） |

## 2. 12（10+2）能力模型交付状态

| # | 能力 | P 级 | 状态 | 载体 |
|---|------|------|------|------|
| 1 | 类型化 Token 流 Reader | P0 | ✅ | kerf-reader（TokenKind 12 变体/叶级 44/词法器/语法器） |
| 2 | 图 IR（共享节点） | P0 | ✅ | kerf-core::ir（Arena + node_map 字面量共享） |
| 3 | 结构化 CodeValue | P0 | ✅ | kerf-core::code_value（良构/自由变量/α重命名/组合预留） |
| 4 | 元循环求值器 | P0 | ✅ | kerf-vm::eval（Rc 环境链 + 卫生回退解析接线） |
| 5 | 基础闭包 | P0 | ✅ | 共享单元格捕获（letrec 递归语义正确） |
| 6 | Span 全管线传播 | P0 | ✅ | Token→Stx→CoreExpr→IR→debug_spans |
| 7 | 结构化诊断 | P0 | ✅ | kerf-span::diagnostic（E0001-E0004 + 摘录渲染 + 调用点追踪 note） |
| 8 | 最小 I/O | P1 | ✅ | 通道 read_line_stdin/write_line_stdout + 语言层 print/read-line |
| 9 | 相位分离 | P0 | ✅ | declare/visit/instantiate 生命周期簿记 + **循环依赖检测（DFS 灰标记）** |
| 10 | 基础宏系统 | P1 | ✅ | syntax-rules + 内置糖 + 一致性卫生重命名 |
| 11 | 标记-清除 GC | P0 | ✅ | 分配驱动触发 + 冷却退避 + 显式工作栈（根集**五来源**） |
| 12 | 字节码 VM | P0 | ✅ | switch-dispatch 40 操作码 + 三扩展槽帧 + **堆栈追踪（最内 16 帧）** |

## 3. 四项接口预留（P2/P3 冻结）

Effect Handlers（P3）/ 多阶段编程（P3）/ **能力模型 I/O（P2——reserved.rs 附 4 条完整行为规格）**
/ 编译缓存（P2）——冻结签名权威为 kerf-driver/src/reserved.rs（可编译签名 + Probe 实现测试证明），
文档副本见 lang-design/13 §3.1（v5.2 回填）。

## 4. 性能基线（§14.8，release 实测 2026-09-10）

- fib(25)（含编译）：**84.4 ms/轮**（release ×5 均值 84.443，复现 r1 声称 84.7 ✅；
  载体：CLI `kerf bench examples/usage/fib.krf`）
- GC 压力：3×10^5 临时分配**单轮 ~0.17 s**（堆有界；回收 >1 次——r1 审查更正
  0.72s 的陈旧口径，方向保守夸大 4.2×；示例载体 examples/usage/gc_stress.krf，
  验收权威口径为 gc_tests 10^6 有界测试）
- clean release 构建 5.75 s；二进制 1.01 MiB

## 5. r3 修复轮清单（deep-review Round 1 → Task 16）

- **App 求值顺序统一函数先**（P1-④，偏差 #13）：compile.rs/vm.rs 双侧对齐 06 §1.3/§2 A1
  契约；回归测试 app_evaluates_fn_then_args + app_evaluation_order_fn_first_dual_path
- **opcode 冻结守护测试重写**（P1-③，偏差 #7）：40 项显式枚举（八组），模块头分组注释补全
- **模块循环依赖检测**（偏差/§7.1.1 类 6）：phase.rs DFS 灰标记 → 结构化 Err
  「模块循环依赖：Symbol(N) → …」；菱形依赖合法
- **VM 堆栈追踪**（审计发现 C03）：run_program 错误路径附加最内 16 帧调用点 note；
  VmError.trace 有生产者
- **eval 路径卫生回退接线**（审计发现 C08，T1）：driver.rs resolve_eval_hygiene_fallbacks
  ——与 VM 路径 resolve_hygiene_fallbacks 语义镜像
- **driver 公共调试 API**：dump_tokens/dump_stx（根 CLI tokens/stx 子命令经 driver 转发，
  §14.7.2 B4 合规）
- **负测扩张**：四文件 483 case（r1 为 37）+ 审计集 41 case；正负比 1:0.24 → **1:3.2**
- **examples 重组**：6 个 .krf → examples/usage/；审计集 → examples/audit/（§9.6 结构 + README 索引）

## 6. 已知边界（TD 登记）

见 docs/develop/v0/tech-debt-register.md（TD-002~TD-014，全部 P2/P3 级，无 P0/P1 遗留
——阶段切换信号「技术债清零」条件满足）。新登记：TD-012（expander.rs 拆分候选）、
TD-013（多错误收集/Expander 恢复展开未实现——单错误短路，P2）、TD-014（嵌套 define
重复展开期消息归因失真，P3）。
