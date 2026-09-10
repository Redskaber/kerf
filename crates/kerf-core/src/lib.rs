//! # kerf-core
//!
//! 核心语言定义层：`CoreExpr`（9 个正交原语）、图 IR（Arena + 共享节点）与
//! 结构化 `CodeValue`（stage0.md §3.2 / §8.2 / §8.3）。
//!
//! 设计约束（§3.2）：
//! - **正交**：9 个原语互相不可推导（`let` 等语法糖必须由宏展开推导，不进核心）；
//! - **完备**：语言全部语义可由 9 原语表达；
//! - **稳定**：核心冻结——一旦定义，在整个语言生命周期内不变（§2.2 原则 9）。
//!
//! 管线数据流（sop.md §2.4.4，编译时数据体系）：
//! `Stx`（kerf-syntax）→ `CoreExpr`（本 crate，展开产物）
//! → `IrGraph`（本 crate，结构化形式）→ `BcProgram`（kerf-compiler）。

pub mod code_value;
pub mod expr;
pub mod ir;

// 显式 re-export（§10.1 规则 4：禁止 glob re-export）。
// 约定：本 crate 暴露核心语言（expr）、图 IR 与代码值全部公共类型。
pub use code_value::{CodeValue, CompositionError, StageLevel};
pub use expr::{Capability, CoreExpr, LiteralValue};
pub use ir::{lower_program, IrGraph, IrNode, NodeId, NodeMetadata};
