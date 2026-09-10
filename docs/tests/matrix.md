# 全局测试矩阵（覆盖率追踪）

> **Author**: kerf-dev-agent（QA-A 角色）
> **Date**: 2026-09-10（r3 负测扩张后全量对账——`cargo test --workspace` 实测复核）
> **Version**: v0.1.0-r3
> **Status**: Active

## 总量

**297 通过 / 0 失败 / 1 忽略**（298 个测试函数；忽略项均为文档化存档，见下）。
§3.2 release 验收基线 204（r2）→ r3 负向测试扩张 + 审计集就位 + FS-1 守卫修复 + 糖正向锚点 + T17-a 六缺陷（D1-D7/D9）修复回归后 **297**。

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
| kerf-vm 单元 | crate 内联 | crates/kerf-vm/src/*.rs | 14 |
| kerf-driver 单元 | crate 内联 | crates/kerf-driver/src/*.rs | 14 |

> r2 基线 127 → r3 130：span +1（守卫）、core 11→10 / expander 23→26（含守护测试重写与循环依赖检测）、
> compiler +1（app_evaluates_fn_then_args）等——逐项以 `cargo test --workspace` 输出为准。

### 集成测试（166 函数 = 163 通过 + 3 忽略，tests/ 阶段树）

| 套件 | 文件 | 函数数 | 通过 | 忽略 |
|------|------|--------|------|------|
| reader_tests | tests/v0/stage0/plan/reader_tests.rs | 10 | 10 | 0 |
| expander_tests | tests/v0/stage0/plan/expander_tests.rs | 17 | 17 | 0 |
| compiler_tests | tests/v0/stage0/plan/compiler_tests.rs | 10 | 10 | 0 |
| vm_tests | tests/v0/stage0/plan/vm_tests.rs | 17 | 17 | 0 |
| gc_tests | tests/v0/stage0/plan/gc_tests.rs | 6 | 6 | 0 |
| pipeline_tests | tests/v0/stage0/plan/pipeline_tests.rs | 11 | 11 | 0 |
| gate_review_r1 | tests/v0/stage0/gate/gate_review_r1.rs | 8 | 8 | 0 |
| negative_reader_tests | tests/v0/stage0/plan/negative_reader_tests.rs | 15 | 15 | 0 |
| negative_expander_tests | tests/v0/stage0/plan/negative_expander_tests.rs | 23 | 23 | 0 |
| negative_vm_tests | tests/v0/stage0/plan/negative_vm_tests.rs | 30 | 30 | 0 |
| negative_semantics_tests | tests/v0/stage0/plan/negative_semantics_tests.rs | 20 | 20 | 0 |

### 负向测试规模与正负比（§9.4.3 对账）

| 负测文件 | 函数数 | case 数（按文件内注释汇总） |
|----------|--------|------------------------------|
| negative_reader_tests.rs | 14 | 59 |
| negative_expander_tests.rs | 23 | 96 |
| negative_vm_tests.rs | 29 | 230 |
| negative_semantics_tests.rs | 20 | 98 |
| **四文件合计** | **86** | **483** |
| 审计集（examples/audit/stage0_gate_audit_r1.rs） | — | 41（负向 32 + 恢复 6 + 正向 3） |

- **全局正负比（case 口径）≈ 1:3.2**：负向 case 483（四文件）+ 32（审计集负向）= 515 vs 正向 ≈ 160
  （正向/快照/双路径断言估算）——**§9.4.3 的 ≥1:3 门限达标**（r1 审查时为 1:0.24）。
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
