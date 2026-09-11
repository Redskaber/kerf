//! 跨 crate 集成测试**唯一总入口**（sop.md §8.4.6 测试入口架构意图
//! v11.2——Cargo.toml 干净精要原则）。
//!
//! cargo 对 `tests/` 顶层 `.rs` 自动发现本文件为单一测试目标，**零
//! `[[test]]` 配置**；v0/stage-N 全部测试文件经 `#[path]` mod 树挂载
//! ——阶段/plan/gate 目录语义不变（物理位置即语义），仅入口收敛。
//!
//! 新增测试文件的标准动作：在对应 stage 分组追加一行 `#[path]` mod
//! 声明 + 新建文件（禁止回到 `[[test]]` 逐文件声明——sop.md §8.4.6
//! 强制规则 8）。
//!
//! 选择性运行：`cargo test --test runner <module>::<case>`（模块路径
//! 即过滤器，如 `cargo test --test runner capability_tests::`）。
//! 测试文件内部的相对 `#[path]`（如 `mod common`）以文件自身目录为
//! 基准解析——挂载方式不改变其解析基准。

// ---------------------------------------------------------------------------
// 共享测试辅助（单实例共享——各测试模块经 `use crate::common` 消费，
// 替代原先每文件 `mod common` 的重复加载形态）
// ---------------------------------------------------------------------------

#[path = "common/mod.rs"]
mod common;

// ---------------------------------------------------------------------------
// Stage 0（语义验证）：plan 套件
// ---------------------------------------------------------------------------

#[path = "v0/stage0/plan/reader_tests.rs"]
mod reader_tests;

#[path = "v0/stage0/plan/expander_tests.rs"]
mod expander_tests;

#[path = "v0/stage0/plan/compiler_tests.rs"]
mod compiler_tests;

#[path = "v0/stage0/plan/vm_tests.rs"]
mod vm_tests;

#[path = "v0/stage0/plan/gc_tests.rs"]
mod gc_tests;

#[path = "v0/stage0/plan/pipeline_tests.rs"]
mod pipeline_tests;

// 架构合规审计（v6.0——原则 29-31 形态审计：私有 ADT 冻结/Span 独立/
// 表面-内部分离；01 §8.5 + 02 §8.1 锚点落地）

#[path = "v0/stage0/plan/architecture_audit_tests.rs"]
mod architecture_audit_tests;

// 负向测试四文件（§9.4.3 正负比 ≥1:3 的主承载）

#[path = "v0/stage0/plan/negative_reader_tests.rs"]
mod negative_reader_tests;

#[path = "v0/stage0/plan/negative_expander_tests.rs"]
mod negative_expander_tests;

#[path = "v0/stage0/plan/negative_vm_tests.rs"]
mod negative_vm_tests;

#[path = "v0/stage0/plan/negative_semantics_tests.rs"]
mod negative_semantics_tests;

// ---------------------------------------------------------------------------
// Stage 0：gate 门审计（§7.3）
// ---------------------------------------------------------------------------

#[path = "v0/stage0/gate/gate_review_r1.rs"]
mod gate_review_r1;

// ---------------------------------------------------------------------------
// Stage 1（自举验证）：plan 套件（批次 A → D）
// ---------------------------------------------------------------------------

// 批次 A（r4：TD-007 trampoline 工作表化）

#[path = "v0/stage1/plan/expansion_worklist_tests.rs"]
mod expansion_worklist_tests;

// 批次 B（r5/r6：标准库最小集 + B3 自举 Reader）

#[path = "v0/stage1/plan/stdlib_tests.rs"]
mod stdlib_tests;

#[path = "v0/stage1/plan/bootstrap_reader_tests.rs"]
mod bootstrap_reader_tests;

// 批次 C（r7：类型检查器 + 编译缓存）

#[path = "v0/stage1/plan/typecheck_tests.rs"]
mod typecheck_tests;

#[path = "v0/stage1/plan/cache_tests.rs"]
mod cache_tests;

// 批次 D（r8：能力 I/O 基础传递 + 内部效应/测试运行器）

#[path = "v0/stage1/plan/capability_tests.rs"]
mod capability_tests;

#[path = "v0/stage1/plan/test_runner_tests.rs"]
mod test_runner_tests;

// r12 批次（worklog Task 31-d）：接口预留扩展层（v6.1 完整性审查——
// next4 第八轮；P0 位置断言 + 14 项预留 API 跨 crate 可达性）

#[path = "v0/stage1/plan/reserved_ext_tests.rs"]
mod reserved_ext_tests;

// 批次 E（r13：TD-004 作用域集解析收口——Racket 式 (name, scopes ⊆)
// 匹配的双路径语义锚点 + 作用域不匹配负例）

#[path = "v0/stage1/plan/scope_set_tests.rs"]
mod scope_set_tests;

// 批次 E（r14：E1-α 自举 Expander——核心形式 + 九糖影子路径 + parity）

#[path = "v0/stage1/plan/bootstrap_expander_tests.rs"]
mod bootstrap_expander_tests;

// 批次 E（r15：E1-β 宏收口 + 生产切换 + TD-021 prelude 模块注入）

#[path = "v0/stage1/plan/prelude_tests.rs"]
mod prelude_tests;

// ---------------------------------------------------------------------------
// Stage 2（完整语言）：plan 套件（批次 G 起）
// ---------------------------------------------------------------------------

// 批次 G（r17：QBE 后端 PoC——AnnotatedANF 实化 + IL 生成 + AOT 端到端
// + PoC 边界负例；38-b/38-c 承载）

#[path = "v0/stage2/plan/qbe_backend_tests.rs"]
mod qbe_backend_tests;

// 批次 H（r18：TD-022 TCO 尾调用优化——帧复用 + 尾位传播 + 指令预算
// 护栏 + TD-007/TD-022 耦合解除自举端到端；40-c 承载）

#[path = "v0/stage2/plan/tco_tests.rs"]
mod tco_tests;

// 批次 H（r18：H4 HM 推断 PoC——约束三段式 + 值限制 + 递归预置 +
// 超集/零误报双门 + 四类缺口检出证明；40-e 承载——38-d 设计落地）

#[path = "v0/stage2/plan/hm_inference_tests.rs"]
mod hm_inference_tests;

// 批次 G（r17：TD-013 多错误收集与恢复展开——种子/桥双路径 +
// driver check_source_recover 消费面；38-e 承载）

#[path = "v0/stage2/plan/multi_error_recovery_tests.rs"]
mod multi_error_recovery_tests;

// 批次 I（r21：I1 前段自举 Compiler parity——八臂 + 作用域机 + 捕获 +
// 回填 + 常量池 + TCO 尾位；bytecode_equal 全结构判据；42-b 承载——
// i1-incision-migration-design §5 门 A 基础组）

#[path = "v0/stage2/plan/bootstrap_compiler_tests.rs"]
mod bootstrap_compiler_tests;

// 批次 I 后段（r25：42-f Effect M1-M5——语言级效应面 + GC 六来源 +
// 双路径一致；effect-language-design §5 测试锚正 6 负 7 + M3 ≥8）

#[path = "v0/stage2/plan/effect_tests.rs"]
mod effect_tests;
