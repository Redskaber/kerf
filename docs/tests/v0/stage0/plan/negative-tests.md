# negative 负向测试计划（开发轮 r3——四文件）

> **Author**: kerf-dev-agent（QA-A 角色）
> **Date**: 2026-09-10
> **Version**: v0.1.0-r3
> **Status**: Active

## 1. 测试目标

对应四个负向测试文件（sop.md §9.2 规则 1 双向印证；§9.4.3 负向优先原则的定向扩张——
deep-review Round 1 行动计划 B 的落地）：

- `tests/v0/stage0/plan/negative_reader_tests.rs`——**Read 阶段（E0001）** 全部错误族
- `tests/v0/stage0/plan/negative_expander_tests.rs`——**Expand 阶段（E0002）** 全部错误族
- `tests/v0/stage0/plan/negative_vm_tests.rs`——**Run 阶段（VM 路径）** 24 内置 × 类型/元数系统表 + E0004 断言
- `tests/v0/stage0/plan/negative_semantics_tests.rs`——**06-操作语义锚定**：E0–E8 错误码矩阵、T1 双路径回归、错误消息形状

设计原则：**表格驱动**（每行 = 1 个独立 case，含 CATEGORY/POLARITY/EXPECTED 断言）；
全部期望消息/Span 经实测校准（预期 Err 实际 Ok 的候选已剔除）；「语义边界」（实测确认
的合法行为，非负例）与「已知缺陷存档」（#[ignore]，不计数）在文件头注逐一记录。

## 2. 覆盖场景

见各代码文件内每个 `#[test]` 的文档注释（目标/不变式/验收条款引用）。核心场景：

- **Reader**：未闭合字符串/非法转义/贪心数字/非法字符/块注释/括号三态/引号 EOF/整数字面量溢出
  + E0001 渲染形状（error[E0001] + `-->` + 源摘录）与 DiagnosticCode 结构化断言
- **Expander**：空应用（§7.1.1 类 3——r1 零测试分类）/21 关键字误用矩阵/syntax-rules 误用/
  宏展开失败（模式不匹配、深度超限）/相位簿记违规（declare/visit/instantiate + **模块循环依赖**）/
  E0002 渲染形状
- **VM**：算术（非数值 ×5 算子 ×左右位置 ×6 类型 = 60 case 整数路径 + 20 浮点路径 + mod 拒浮点）/
  比较类型不匹配 ×5 算子 ×6 变体 = 30 case/元数下限/除零取模零/整数溢出/序对谓词元数与类型/
  eq? 不可链/not 非布尔/str-append/不可调用值/if 非布尔/帧上限（MAX_FRAMES）+ E0004 断言
- **语义**：E1 truthy（7 类型 + 嵌套位置）/E2 元数（含双路径）/E3 未绑定（10 上下文 + 宏引入）/E4
  不可调用（7 类型 + 计算被调者）/E5 内置错误族/E6 重复定义矩阵（9 形态 + 双路径）/
  消息形状（阶段码 + `-->` + 摘录 + 调用点追踪 note 帧）/T1 回归三件套（全局存储/App 顺序/
  卫生回退）/错误恢复（单错误短路——TD-013 存档；不 panic 不挂起）

## 3. 测试统计

实际函数数见 [matrix.md](../../../matrix.md) 分套件统计表（cargo test 实测口径）；
case 数按各文件内注释汇总：

| 文件 | 函数数（通过+忽略） | case 数 |
|------|--------------------|---------|
| negative_reader_tests.rs | 13 + 1 | 59 |
| negative_expander_tests.rs | 23 + 0 | 96 |
| negative_vm_tests.rs | 26 + 3 | 230 |
| negative_semantics_tests.rs | 20 + 0 | 98 |
| **合计** | **82 + 4** | **483** |

- 4 个 `#[ignore]` 均为文档化存档（守卫失效/元数不校验/eval 栈溢出分裂/i64::MIN mod），
  不计 case 数。
- 正负比对账：483 + 审计集负向 32 = 515 负向 case vs 正向 ≈160 → **≈1:3.2（≥1:3 达标）**。
- 不可触发码的文档化存档（negative_semantics_tests 头注）：E7（I/O 错误——read-line EOF → nil
  为正常语义）/E8（内部不变式）/E0003（编译期内部防御）——非用户程序可构造，§9.4.3 规则 2。

## 4. 依赖

- 上游：kerf-driver 全管线（run_source / eval_source / Stage）；common/mod.rs（dual_path_agrees——
  双路径 Err 事实断言）
- 共享：kerf_expander::phase::ModuleRegistry（循环依赖公共 API 直测）、kerf_vm::Value
- 关联：审计集 examples/audit/stage0_gate_audit_r1.rs（§7.3.1 门审计——与 negative_expander
  的循环依赖负例互为补充）；[pipeline-test-coverage.md](../../../pipeline-test-coverage.md) §3 路径矩阵
