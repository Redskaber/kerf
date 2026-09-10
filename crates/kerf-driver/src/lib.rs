//! # kerf-driver
//!
//! 编译器入口（编排层）：`read → expand → lower → compile → run` 全管线。
//!
//! **管线**（sop.md §2.4.4 单向流动）：
//! ```text
//! 源文本 → [kerf-reader] Token/Stx → [kerf-expander] CoreExpr
//!        → [kerf-core] IrGraph/CodeValue → [kerf-compiler] BcProgram
//!        → [kerf-vm] 执行（eval / VM 双路径） → [kerf-runtime] 堆与 I/O
//! ```
//!
//! **相位分离接线**（§8.9）：declare → visit（变换器注册）→ instantiate
//! （编译产物消费）——单模块的完整生命周期经 `ModuleRegistry` 簿记。
//!
//! **全局环境**：内置函数注册（§9.stdlib：语言核心零内置，全部经 driver 注入）
//! + 卫生回退解析（`$hyg$N` 后缀剥离——Stage 0 名称基解析的既定近似）。
//!
//! **接口预留层**（§9.1，P2/P3 级）：Effect Handlers / 多阶段编程 /
//! 能力模型 I/O / 编译缓存——仅类型签名与行为规格，冻结契约。

//! **自举 Reader**（B3，Stage 1 批次 B）：`bootstrap` 模块——kerf 源码
//! Reader（`bootstrap/reader.krf`）经种子管线编译后在 VM 上运行；生产
//! 读路径（`compile_front`）经自举 Reader，种子（kerf-reader）保留为引导
//! 实现与 parity oracle（07-bootstrap §3.2 混合期构成）。

pub mod bootstrap;
pub mod builtins;
pub mod cache;
pub mod capability;
pub mod driver;
pub mod effects;
pub mod hash;
pub mod reserved;

// 显式 re-export（§10.1 规则 4：禁止 glob re-export）。
// 约定：本 crate 暴露驱动层全部公共类型与入口。
pub use cache::{
    cache_enabled, cache_entry_count, cache_invalidate_all, cache_key, cache_reset, cache_stats,
    set_cache_enabled, CacheStats, InMemoryCompilationCache, COMPILE_CONFIG_SEED,
};
pub use capability::{IoGrant, IoRequirements, StdCapabilityIO};
pub use driver::{
    check_source, compile_source, dump_stx, dump_tokens, eval_source, run_source,
    run_source_rendered, test_source, CheckReport, CompileOutput, DriverError, RunOutcome, Stage,
    TestCaseOutcome, TestReport,
};
pub use effects::{
    handle_escape, perform_escape, InternalEffectSystem, ValueEffect, ValueEffectFamily,
    ValueHandler,
};
pub use hash::{content_hash64, sha256, sha256_hex};
pub use reserved::{
    CacheKey, CachedResult, CapabilityIO, CompilationCache, Effect, EffectFamily, EffectSystem,
    IOError, MultiStage, ReadCapability, WriteCapability,
};
