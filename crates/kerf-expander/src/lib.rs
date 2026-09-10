//! # kerf-expander
//!
//! **五正交轴定位（lang-design 15 §5.3，v6.0）**：轴 1 桥接的执行面——`Stx → CoreExpr` 的唯一翻译器（宏展开 + 糖推导；翻译发生而非直通，审计测试锚定）。
//!
//! 展开器骨架 + 卫生宏系统 + 相位分离（stage0.md §8.9 / §8.10 / §12.3 / §19.2）。
//!
//! 管线位置（sop.md §2.4.4）：`Stx`（kerf-reader 产出）→ **本 crate** →
//! `CoreExpr`（kerf-core）。
//!
//! **模块布局**（TD-012 拆分，2026-09-10：核心形式/糖推导/相位驱动
//! 三职责分置；主控 expander 保留入口分派与公共构造辅助）：
//! - `expander`：入口分派 + 公共类型（`ExpandCtxt`/`ExpandError`）+
//!   宏调用展开 + 构造底座；
//! - `core_forms`：9 核心形式展开 + 函数体 define 提升路径（crate 私有）；
//! - `sugar`：语法糖内置变换器（§3.2 推导表，crate 私有）；
//! - `macro_sys`：变换器机制（syntax-rules/卫生上下文）；
//! - `phase`：相位驱动（模块 declare/visit/instantiate 簿记）。
//!
//! **能力清单**：
//! - 9 个核心形式的展开（lambda/if/set!/define/begin/module/varref/literal/app）；
//! - 语法糖内置变换器（let/letrec/let*/cond/and/or/when/unless/while——§3.2 推导表）；
//! - 用户卫生宏 `define-syntax` + `syntax-rules`（模式/模板/省略号）；
//! - 相位分离（Phase 0/1 物理隔离 + declare/instantiate/visit 模块生命周期）。
//!
//! **卫生模型**（Stage 0，P1 级）：
//! - 语法对象全程携带 `ScopeSet`（绑定形式加 mark，§19.2）；
//! - 引入标识符（宏模板中的非模式变量）经 **α 重命名** 为唯一符号——
//!   「宏内外同名不串扰」的卫生保证（完整，P1 要求）；
//! - Racket 式作用域集解析推迟到 Stage 1（技术债 TD-004）。
//!
//! **核心不变式**（§19.2）：
//! 1. 展开终止性：展开代次超上限报错而非栈溢出（当前 128，TD-007
//!    迭代式工作表化后解除至文档口径 10_000）；
//! 2. 卫生性保持：宏引入标识符与用户标识符永不混淆；
//! 3. 相位封闭性：Phase 1 transformer 只产生 Phase 0 语法对象。

mod core_forms;
pub mod expander;
pub mod macro_sys;
pub mod phase;
mod sugar;

// 显式 re-export（§10.1 规则 4：禁止 glob re-export）。
// 约定：本 crate 暴露展开域全部公共类型与入口；core_forms/sugar 为
// 内部实现模块（§11 接口隔离——外部仅见 expander 入口），不导出。
pub use expander::{expand_form, expand_program, ExpandCtxt, ExpandError};
pub use macro_sys::{SyntaxRules, Transformer, TransformerKind};
pub use phase::{ModuleEntry, ModuleRegistry, PhaseLevel};
