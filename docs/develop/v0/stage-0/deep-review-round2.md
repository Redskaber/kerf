# Stage 0 深度审查报告（Round 2——修复复审）

> **Author**: Super Z（ARCH-A 主导，QA-A/REV-A/ALG-C/DEV-A 会签）
> **Date**: 2026-09-10
> **Version**: v0.1.0-deep-r2
> **Status**: Active
> **基线**: Round 1 行动计划全部执行完毕（Task 16/17）；lang-design v5.2 / 297 测试全绿（release）/ 审计集 41 case 全 PASS
> **对照**: [deep-review-round1.md](./deep-review-round1.md)（NEEDS REVISION 判定与 P1×4）

---

## 1. 执行摘要

Round 1 判定 NEEDS REVISION 的 **P1×4 全部清零**，且 §14.6.3 多轮深挖（对抗轮 T17-a）新发现的 **6 项 P1**（D1/D2/D3/D4/D5/D6）与 P2×1（D7）亦全部当轮修复并附回归测试。**当前 P0/P1 缺陷存量：0**。

| Round 1 P1 | 处置 | 证据 |
|------------|------|------|
| ① 正负比 1:0.24（7 分类零负测） | ✅ 四文件表驱动负测 ≈490 case；全局比 ≈1:3.2（负 ≈515 vs 正 ~160） | matrix.md §总量；negative_*_tests.rs |
| ② §7.3.1 审计集缺位（gate R1 判定无效） | ✅ examples/audit/stage0_gate_audit_r1.rs 41 case（七类全覆盖+配比机械校验）；release 复跑 EXIT 0 | gate-review-round2.md |
| ③ opcode 冻结守护测试失效 | ✅ 40 项显式枚举重写（8 组标注）+ 模块头补 DEFINE_GLOBAL | opcode.rs:182 |
| ④ App 求值顺序双路径分裂 | ✅ 编译侧改函数先（06 §2 A1 契约）+ 双路径 span 判定负例 | compile.rs / vm_tests.rs:187 |

对抗深挖（T17-a，§14.6.3 独立轮）追加发现与处置：

| 发现 | 等级 | 处置 | 证据 |
|------|------|------|------|
| D1 begin 包裹 define 全局泄漏（VM）/词法（eval）——T1 反例 | P1 | ✅ 编译期结构化拒绝（E0003，共享 compile_source 双侧一致） | negative_semantics e6 |
| D2 eq? 字符串指针比较（VM 常量池去重 vs eval 独立分配）——T1 反例 | P1 | ✅ 内容比较（L4 即时值按值） | d2_eq_string_content_semantics |
| D3 eval 深递归栈溢出 abort | P1 | ✅ MAX_EVAL_DEPTH=256 结构化上限（实测标定 2.7× 裕度） | deep_recursion_eval_path_structured_error |
| D4 i64::MIN ÷/mod -1 Rust panic | P1 | ✅ checked_div/rem | mod_i64_min_structured_error |
| D5 (+) 零参越界 panic | P1 | ✅ 单位元 0/1 | arithmetic_arity_floor |
| D6 宏调宏未绑定（变换器表查不到重命名符号） | P1 | ✅ 展开器基名回退（卫生穿透落地） | d6_macro_calls_macro_hygiene_fallback |
| D7 eval 错误前缀累积+Span 丢失 | P2 | ✅ apply 保真透传 | d7_eval_error_fidelity |
| D9 (- 5) 返回 5 等一元语义 | P3 | ✅ 取负（Scheme 惯例） | arithmetic_arity_floor |
| FS-1 Reader 嵌套守卫先溢出 | P1* | ✅ MAX_NESTING_DEPTH 10_000→256 | deep_nesting_guard_structured_error |
| D8 消息文本分裂 / O-1 IrGraph 旁路 / FS-5 链式短路 | P3 | 📋 TD-018 / TD-015 / TD-016（显式裁定） | tech-debt-register.md |

## 2. 验收命令实测（§3.2——2026-09-10，clean 后全量）

| 命令 | 结果 |
|------|------|
| cargo clean + build --release | ✅ 0 警告（6.40s） |
| cargo check | ✅ 0 errors / 0 warnings |
| cargo test --release --workspace | ✅ **297 passed / 0 failed / 1 ignored**（文档化 FS-4） |
| cargo fmt --check | ✅ 零 diff |
| cargo clippy --all-targets -- -D warnings | ✅ 0 warnings |
| cargo run --release --example stage0_gate_audit_r1 | ✅ 41/41 PASS，EXIT 0 |

## 3. 委员会投票（§6.3 外循环——Round 2）

| 角色 | 权重 | 投票 | 理由 |
|------|------|------|------|
| ARCH-A | 2 | **APPROVED** | P0/P1 清零；DAG/接口隔离 6/6 PASS；架构审查 0❌；文档 v5.2 与实现一致（26 项偏差回写闭环） |
| DEV-A | 1.5 | **APPROVED** | 7 项重构全判最优+治根（§14.6.2）；§3.2 全绿；§14.9 C1-C6 完成（catch-all 41 臂全裁决、零 TODO、//! 37/37） |
| QA-A | 1 | **APPROVED** | 正负比 1:3.2 达标（§9.4.3）；审计集 41 case 七类全覆盖（§7.3.1）；DEFERRED 3.8% ≤ 5%（§9.5）；性能基线无回归（§14.6.4） |
| ALG-C | 1 | **APPROVED** | R1-R9/E0-E8 语义矩阵完整；T1 定理修复面三度扩展（define/set!/App 顺序/卫生回退/eq?/D1）全部双路径一致 |
| SKL-A | 1（不计权） | **APPROVED** | CLI 10 子命令/示例重组/Playground/文档树完整 |

**加权通过率：5.5/5.5 = 100% ≥ 95% → 外循环通过**（无需触发二次内循环）。

## 4. 结论

**GO**——Stage 0 满足全部阶段切换信号（§21.5）：语义稳定（T1 修复面完整）、
测试覆盖近 100%（DEFERRED 3.8%）、性能基线建立、文档同步（v5.2 + 工程树对账）、
技术债 P0/P1 清零（TD-002~018 全开放项均 P2/P3 且有偿还计划）、接口预留冻结
（Probe 测试证明）。**建议进入 Stage 1 规划（§21）。**

## 5. 本轮遵循原则

§14.5.3（P1 本阶段修复——两轮共 10 项 P1 全清）；§7.3.1 规则 4（审计集逐轮扩大：
0→41）；§6.3（外循环加权投票）；§2.3-9（D1/D2 按设计裁编译侧/值语义而非改设计）；
§5.3 八项硬条件（P0/P1 清零+集成测试+文档同步+fmt/clippy/test 全绿+深审完成）。
