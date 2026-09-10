//! # kerf-syntax
//!
//! Symbol 内部化（interning）、作用域集与语法对象 `SyntaxObject`（stage0.md §3.3）。
//!
//! 语法对象是卫生宏系统的基础——它携带源位置、作用域集和相位信息，
//! 使得展开器可以区分「宏引入的标识符」和「用户代码中的标识符」（§3.3）。
//!
//! 职责边界：
//! - **做什么**：定义 `Symbol`（u32 句柄）与 `Interner`（唯一可信数据源，sop.md §2.3 原则 10）、
//!   `ScopeSet`、`Stx`（语法对象）、`Keyword` 分类。
//! - **不做什么**：不做词法/语法分析（kerf-reader）、不做宏展开（kerf-expander）。

pub mod scope;
pub mod stx;
pub mod symbol;

// 显式 re-export（§10.1 规则 4：禁止 glob re-export）。
// 约定：本 crate 暴露语法域全部公共类型。
pub use scope::{ScopeId, ScopeSet};
pub use stx::{Stx, StxDatum, StxLiteral};
pub use symbol::{Keyword, Symbol, SymbolTable};

/// 相位（§3.3 / §8.9）：Phase 0 = 运行时，Phase 1 = 宏展开时。
/// 两个相位是物理隔离的两个世界（sop.md §2.4.4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Phase 0：运行时世界。
    Runtime,
    /// Phase 1：宏展开（编译时）世界。
    ExpandTime,
}

impl Phase {
    /// 相位数值（§3.3 中 phase: int 的稳定序）。
    pub fn as_u32(self) -> u32 {
        match self {
            Phase::Runtime => 0,
            Phase::ExpandTime => 1,
        }
    }
}
