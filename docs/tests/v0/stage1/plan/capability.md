# capability 测试计划（r8 批次 D）

> **Author**: kerf-dev-agent
> **Date**: 2026-09-10
> **Version**: v0.1.0-r8
> **Status**: Active

## 1. 测试目标

对应 [`tests/v0/stage1/plan/capability_tests.rs`](../../../../../../tests/v0/stage1/plan/capability_tests.rs)
的双向印证文档（sop.md §9.2 规则 1）。覆盖能力模型 I/O 基础传递
（13-capability-matrix §3.1.3 P2 规格四条款）的端到端锁定：
`(require io read|write)` 声明形式 + R9/E0006 编译期权限验证 +
令牌授权面（IoGrant 门控注册）。

## 2. 覆盖场景

见代码文件内每个 `#[test]` 的文档注释。核心维度：
- 条款 3（编译期验证）：E0006 于 run/eval/check 路径一致门控
  （front 管线单一验证点——「任意流程节点」的管线面证明）；
- 声明面正向：require 后 I/O 可用（print/newline/读写双声明）+
  求值恒 nil 双路径一致（T1）+ 幂等 + 顺序无关 + CheckReport 观测；
- 门控边界：嵌套深引用 / 宏模板卫生基名判定 / 粒度互斥
  （read 声明不开放 write——最小权限）；
- 豁免纪律：用户 define 同名接管不误报 / 别名值侧引用仍门控（零误报）；
- 形状负例：缺参 / 未知主体 / 未知能力项 / 非符号（E2 家族）；
- 不可伪造性：令牌类型外部仅可引用不可构造（与 reserved.rs 单测镜像）；
- read-line EOF 约定路径：**确定性子进程探针**（CARGO_BIN_EXE_kerf
  + Stdio::null()——进程内直调会阻塞真实 stdin，非确定性，禁止）；
- examples/usage/io.krf 活体样例（require 前缀形态）。

## 3. 测试统计

见 [docs/tests/matrix.md](../../../matrix.md)（24 函数 / 负例 15+：
门控 8 + 形状 4 + 边界 3）。

## 4. 依赖

- 上游：kerf-driver 全管线（run/eval/check 三入口）+ capability.rs
  （R9 验证）+ builtins.rs（能力参数化注册）
- 冻结契约：reserved.rs（CapabilityIO trait + 令牌铸造 pub(crate)）
- 设计锚：[13-能力矩阵 §3.1.3](../../../../lang-design/13-capability-matrix.md)
  r8 交付注记（三层分工）
