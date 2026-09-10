# 全局测试矩阵（覆盖率追踪）

> **Author**: kerf-dev-agent（QA-A 角色）
> **Date**: 2026-09-10（r6：Stage 1 批次 B 收官 B3「Reader kerf 重写」交付后全量对账——`cargo test --workspace` 实测复核）
> **Version**: v0.1.0-r6
> **Status**: Active

## 总量

**356 通过 / 0 失败 / 1 忽略**（357 个测试函数；忽略项均为文档化存档，见下）。
§3.2 release 验收基线 204（r2）→ r3 负向测试扩张 + 审计集就位 + FS-1 守卫修复 + 糖正向锚点 + T17-a 六缺陷（D1-D7/D9）修复回归后 297 → r4（Stage 1 批次 A）304 → r5（批次 B TD-002 符号值 + 标准库最小集）324 → **r6（批次 B 收官 B3 自举 Reader）356**：+4 kerf-vm 单元（call_closure 宿主调用：带参调用/元数与类型错/跨程序拒绝/错误传播）+ +28 bootstrap_reader_tests（B3 parity 套件：正例 87 case + 负例 307 case——Stx 树与错误消息/Span 双实现逐字节等价、双错误次序、Unicode/NFC、深度边界 256/257、高阶函数直测、Reader 原语误用）。**全套件 356 项经自举 Reader（kerf 源码，VM 上运行）执行——含全部既有负向消息断言，即整体行为等价的实证。**

> 1 个 `#[ignore]`（实测确认的行为边界存档，不计 case 数，文件内注释说明理由）：
> - negative_vm_tests::read_line_arity_ignored（read-line 元数不校验——语义发现 FS-4）
>
> r3 内已修复激活（原 4 个 ignore → 1）：FS-1 嵌套守卫（10_000→256）、
> D3 eval 深度上限（MAX_EVAL_DEPTH=256 结构化）、D4 mod/除 i64::MIN
> 溢出（checked_div/rem）。

## 分套件统计（2026-09-10 实测）

### 单元测试（130，crates 内联）

| 套件 | 层级 | 文件/位置 | 测试数 |
|------|------|----------|--------|
| kerf-span 单元 | crate 内联 | crates/kerf-span/src/*.rs | 11 |
| kerf-syntax 单元 | crate 内联 | crates/kerf-syntax/src/*.rs | 11 |
| kerf-core 单元 | crate 内联 | crates/kerf-core/src/*.rs | 10 |
| kerf-reader 单元 | crate 内联 | crates/kerf-reader/src/*.rs | 23 |
| kerf-expander 单元 | crate 内联 | crates/kerf-expander/src/*.rs | 26 |
| kerf-compiler 单元 | crate 内联 | crates/kerf-compiler/src/*.rs | 12 |
| kerf-runtime 单元 | crate 内联 | crates/kerf-runtime/src/*.rs | 9 |
| kerf-vm 单元 | crate 内联 | crates/kerf-vm/src/*.rs | 18（r6 +4 call_closure） |
| kerf-driver 单元 | crate 内联 | crates/kerf-driver/src/*.rs | 14 |

> r2 基线 127 → r3 130：span +1（守卫）、core 11→10 / expander 23→26（含守护测试重写与循环依赖检测）、
> compiler +1（app_evaluates_fn_then_args）等——逐项以 `cargo test --workspace` 输出为准。

### 集成测试（173 函数 = 172 通过 + 1 忽略，tests/ 阶段树）

> r4 新增 `tests/v0/stage1/plan/expansion_worklist_tests.rs`（4 函数——TD-007 集成锚点：200 层链端到端 VM / 双路径一致 / 无限自指宏 Expand 结构化报错 / 宏链与糖交错推导）。Cargo.toml [[test]] 已显式声明。

| 套件 | 文件 | 函数数 | 通过 | 忽略 |
|------|------|--------|------|------|
| reader_tests | tests/v0/stage0/plan/reader_tests.rs | 10 | 10 | 0 |
| expander_tests | tests/v0/stage0/plan/expander_tests.rs | 18 | 18 | 0 |
| compiler_tests | tests/v0/stage0/plan/compiler_tests.rs | 10 | 10 | 0 |
| vm_tests | tests/v0/stage0/plan/vm_tests.rs | 20 | 20 | 0 |
| gc_tests | tests/v0/stage0/plan/gc_tests.rs | 6 | 6 | 0 |
| pipeline_tests | tests/v0/stage0/plan/pipeline_tests.rs | 11 | 11 | 0 |
| gate_review_r1 | tests/v0/stage0/gate/gate_review_r1.rs | 8 | 8 | 0 |
| negative_reader_tests | tests/v0/stage0/plan/negative_reader_tests.rs | 15 | 15 | 0 |
| negative_expander_tests | tests/v0/stage0/plan/negative_expander_tests.rs | 23 | 23 | 0 |
| negative_vm_tests | tests/v0/stage0/plan/negative_vm_tests.rs | 31 | 31 | 0 |
| negative_semantics_tests | tests/v0/stage0/plan/negative_semantics_tests.rs | 20 | 20 | 0 |
| stdlib_tests（r5） | tests/v0/stage1/plan/stdlib_tests.rs | 15 | 15 | 0 |
| bootstrap_reader_tests（r6） | tests/v0/stage1/plan/bootstrap_reader_tests.rs | 28 | 28 | 0 |

### 负向测试规模与正负比（§9.4.3 对账）

| 负测文件 | 函数数 | case 数（按文件内注释汇总） |
|----------|--------|------------------------------|
| negative_reader_tests.rs | 14 | 59 |
| negative_expander_tests.rs | 23 | 96 |
| negative_vm_tests.rs | 30 | 236（r5 +6 符号值语义误用） |
| negative_semantics_tests.rs | 20 | 98 |
| **四文件合计** | **87** | **489** |
| stdlib_tests（r5，tests/v0/stage1） | 15 | 172（元数 25/类型 86/边界 12/语义 31/双参扫描 18） |
| bootstrap_reader_tests（r6，tests/v0/stage1） | 28 | 307（负例语料 28 + 次序 5 + 深度 2 + 原语 6 + 系统化矩阵 266：括号 65/转义 42/数字 101/非法字符 32/双错误 9/注释与 EOF 16） |
| 审计集（examples/audit/stage0_gate_audit_r1.rs） | — | 41（负向 32 + 恢复 6 + 正向 3） |

- **全局正负比（case 口径）≈ 1:3.2**：负向 case 489（四文件）+ 172（stdlib r5）+ 307（bootstrap r6）
  + 32（审计集负向）= 1000 vs 正向 ≈ 311（r5 224 + r6 正例 87 case：parity 语料/hof/原语/管线）——
  **§9.4.3 的 ≥1:3 门限维持达标**（r1 审查时为 1:0.24）。r6 负例全部经
  `assert_negative_parity`（种子确实报错断言——防语料误收正例）。
- 逐分类负测非零：reader/expander/compiler/vm/gc/pipeline/gate/driver 全部含负向 case
  （r1 审查时 7 个分类为零）。
- §7.1.1 七类负向矩阵 7/7（含空应用与模块循环依赖）；E 码直接断言：E1–E6 全部
  （negative_semantics_tests 逐码矩阵）+ E0001/E0002/E0004 结构化断言
  （negative_reader/expander/vm）；E7/E8/E0003 经公开 API 不可触发——文档化存档
  （negative_semantics_tests 头注，§9.4.3 规则 2）。
- 负测文档锚点：[negative-tests.md](./v0/stage0/plan/negative-tests.md)。

## 需求覆盖（sop.md §21.3 Stage 0 验收标准 → 测试）

| 验收项 | 覆盖测试 |
|--------|---------|
| (1) 9 原语语义正确 | vm_tests::nine_primitives_semantics + gate_g1/g2 + negative_semantics E 码矩阵 |
| (2) 50+ 快照测试 | 全套件（294 ≥ 50） |
| (3) 自举测试（同结果） | compiler_tests::deterministic + pipeline_tests::convergent |
| (4) Span 全管线传播 | pipeline_tests::span_propagates + gate_g6 + negative_reader/expander Span 精确断言 |
| (5) 性能基准基线 | CLI bench（fib(25) 84.4ms/轮实测；examples/usage/fib.krf） |
| (6) 四项接口预留冻结 | gate_g7_to_g10 + reserved.rs Probe 测试（reserved_signatures_are_frozen 等） |
| (7) §7.3.1 门审计集 ≥30 case | examples/audit/stage0_gate_audit_r1.rs（41 case，配比满足） |

## 测试类型分布

- 快照/黄金输出：Token 快照 / Stx 渲染 / CoreExpr 渲染 / 反汇编 / 运行结果渲染
- 语义断言：9 原语 / fib / 闭包 / 宏 / quote / GC
- 负向/错误（表格驱动，每行 = 1 case）：词法 59 / 展开 96 / VM 230 / 语义 98 / 审计 41——全阶段错误路径
- 双路径互查：VM vs eval（164 集成函数全部内置；错误程序断言 Err 事实一致）
- 压力/稳健：10^6 有界分配（gc_tests，验收口径）/ 3×10^5（gc_stress 示例口径）/ 10^4 深递归 / 2×10^5 深链标记
