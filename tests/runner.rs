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
