//! 类型化 Token（stage0.md §8.1：2026 成熟方案——Rust proc-macro 风格的类型化 Token）。

use std::rc::Rc;

use kerf_span::Span;
use kerf_syntax::{Keyword, ScopeSet, Symbol};

/// Token 结构（§8.1 能力模型）：
/// ```text
/// pub struct Token {
///     pub kind: TokenKind,      // 类型化的 Token 种类
///     pub span: Span,           // 源位置
///     pub scope_id: ScopeSet,   // 作用域信息（Reader 产出时为空集，展开器补标记）
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    /// 作用域集合：Reader 产出时为空集（Racket 语义：read 不加作用域，
    /// 展开阶段加 mark）；字段保留以满足 §8.1 接口形状。
    pub scopes: ScopeSet,
}

impl Token {
    /// 是否为 EOF。
    pub fn is_eof(&self) -> bool {
        matches!(self.kind, TokenKind::Eof)
    }

    /// 是否为指定关键字。
    pub fn is_keyword(&self, kw: Keyword) -> bool {
        matches!(&self.kind, TokenKind::Keyword(k) if *k == kw)
    }
}

/// 类型化 Token 种类（§8.1）。
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // 字面量
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(Rc<str>),
    BoolLiteral(bool),
    NilLiteral,

    // 标识符
    Identifier(Symbol),
    /// 引号简写 `'x`（解析为 `(quote x)` 的简写标记）。
    QuoteShorthand,

    // 关键字
    Keyword(Keyword),

    // 运算符（§8.1：携带种类信息 + Symbol 句柄——语法器据此符号化）
    Operator(Operator, Symbol),

    // 分隔符
    Delimiter(Delimiter),

    /// 宏扩展点标记（§8.1 接口保留位：`#name` 直调语法 Stage 1 评估；
    /// Stage 0 宏调用经由列表头符号在展开器解析）。
    MacroInvocation(Symbol),

    Eof,
}

/// 运算符（S 表达式中缀符号作为标识符处理，此处保留分类能力）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
}

/// 分隔符（§8.1：(, ), {, }, [, ]；kerf 表面语法使用 ( ) 与 [ ]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Delimiter {
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
}

impl Delimiter {
    /// 分隔符字符。
    pub fn as_char(self) -> char {
        match self {
            Delimiter::OpenParen => '(',
            Delimiter::CloseParen => ')',
            Delimiter::OpenBracket => '[',
            Delimiter::CloseBracket => ']',
        }
    }

    /// 匹配关闭侧（括号平衡校验）。
    pub fn closing_of(open: Delimiter) -> Delimiter {
        match open {
            Delimiter::OpenParen => Delimiter::CloseParen,
            Delimiter::OpenBracket => Delimiter::CloseBracket,
            _ => Delimiter::CloseParen,
        }
    }
}

/// Reader 域错误的最小共享形态（sop.md §10.1 规则 8：`{ message, span }`）。
///
/// 结构化：诊断化由调用方（driver）将 `ReadError` 转入 `Diagnostic`。
#[derive(Debug, Clone, PartialEq)]
pub struct ReadError {
    pub message: String,
    pub span: Span,
}

impl ReadError {
    /// 构造词法/语法错误（必带 Span，§19.1 不变式 2）。
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        ReadError {
            message: message.into(),
            span,
        }
    }

    /// 意外的 Token（语法器）。
    pub fn unexpected_token(token: &Token) -> Self {
        ReadError {
            message: format!("意外的 Token：{:?}", token.kind),
            span: token.span,
        }
    }

    /// 括号不平衡（开侧未闭合）。
    pub fn unclosed_delimiter(open: Delimiter, span: Span) -> Self {
        ReadError {
            message: format!(
                "括号未闭合：缺少 '{}'",
                Delimiter::closing_of(open).as_char()
            ),
            span,
        }
    }

    /// 括号不平衡（闭侧多余）。
    pub fn stray_closing_delimiter(close: Delimiter, span: Span) -> Self {
        ReadError {
            message: format!("多余的关闭括号：'{}'", close.as_char()),
            span,
        }
    }
}
