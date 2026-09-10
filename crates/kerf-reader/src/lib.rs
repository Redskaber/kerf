//! # kerf-reader
//!
//! 类型化 Token 流 Reader（stage0.md §8.1 / §19.1，Layer 2）。
//!
//! **能力模型**：将字符流转为类型化 Token 流，再经递归下降语法器产出
//! 语法对象 `Stx`。每个 Token 携带精确 Span；词法/语法错误返回
//! 结构化错误（含位置信息）。
//!
//! **职责边界**（§8.1 接口契约）：
//! - **做什么**：字符流 → Token 流 → 语法对象；无损覆盖输入；Span 精确。
//! - **不做什么**：不做宏展开；不判断类型正确性；不做语义分析。
//!
//! **核心不变式**（§19.1）：
//! 1. 无损性：Token 流的 Span 并集精确覆盖输入字节区间（无间隙、无重叠）；
//! 2. 位置完备性：任何错误路径都构造携带完整 Span 的错误；
//! 3. 词法层零语义：关键字分类是纯查表；`MacroInvocation` Token 只是扩展点标记。

pub mod lexer;
pub mod parser;
pub mod token;

// 显式 re-export（§10.1 规则 4：禁止 glob re-export）。
// 约定：本 crate 暴露 Reader 域全部公共类型与入口。
pub use lexer::{lex_source, Lexer};
pub use parser::{parse_tokens, read_program, read_source, Parser};
pub use token::{Delimiter, Operator, ReadError, Token, TokenKind};
