//! # kerf-span
//!
//! **五正交轴定位（lang-design 15 §5.3，v6.0）**：横切元数据基座——五轴共享的 Span/Diagnostic 层（原则 30：元数据独立于命名，`kind_name` 仅诊断渲染）。
//!
//! Span 源位置追踪系统与结构化诊断框架（stage0.md §8.6 / §8.7 / §12.1 / §12.2，Layer 1）。
//!
//! 职责边界（stage0.md §8.6）：
//! - **做什么**：提供不可变的 `Span`（file_id + 字节偏移 + 展开代次）、`SourceMap`
//!   （file_id → 源文本/行号表）与 rustc 风格的 `Diagnostic` 结构化诊断。
//! - **不做什么**：不做任何 I/O 副作用（渲染为字符串返回，由调用方输出）；
//!   不依赖任何其它 kerf crate（Layer 1 基础设施）。
//!
//! 设计约束（sop.md §2.4.4）：
//! - Span 是跨阶段共享的 ID 类型（无前缀，§10.1 规则 3）；
//! - 错误是数据而非异常——`Diagnostic` 可组合、可渲染、可序列化为文本。

pub mod diagnostic;
pub mod source_map;
pub mod span;

// 显式 re-export（§10.1 规则 4：禁止 glob re-export）。
// 约定：本 crate 对外暴露 Span 域的全部公共类型。
pub use diagnostic::{
    render_diagnostic, Diagnostic, DiagnosticCode, Severity, SubDiagnostic, Suggestion,
};
pub use source_map::{SourceFile, SourceMap};
pub use span::{ByteOffset, ExpansionId, FileId, Span};
