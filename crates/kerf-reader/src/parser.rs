//! 递归下降语法器（stage0.md §19.1 语法器骨架的落地实现）。
//!
//! Token 流 → 语法对象 `Stx` 树（datum）。每个非终结符一函数；
//! `'x` 简写展开为 `(quote x)`。
//!
//! 入口（sop.md §10.1 规则 1）：
//! - `read_source`：字符串 → `Vec<Stx>`（词法 + 语法一站式入口，driver 使用）；
//! - `parse_tokens`：已有 Token 流 → `Vec<Stx>`。

use kerf_syntax::{Stx, StxDatum, StxLiteral, SymbolTable};

use crate::lexer::lex_source;
use crate::token::{Delimiter, ReadError, Token, TokenKind};

/// 一站式读取入口：源文本 → 语法对象序列（程序 = 顶层形式序列）。
pub fn read_source(
    source: &str,
    file_id: u32,
    interner: &mut SymbolTable,
) -> Result<Vec<Stx>, ReadError> {
    let tokens = lex_source(source, file_id, interner)?;
    parse_tokens(&tokens, interner)
}

/// 读取程序（`read_source` 的别名入口，语义同上；见 §10.1 入口函数模式）。
pub fn read_program(
    source: &str,
    file_id: u32,
    interner: &mut SymbolTable,
) -> Result<Vec<Stx>, ReadError> {
    read_source(source, file_id, interner)
}

/// Token 流 → 语法对象序列。
pub fn parse_tokens(tokens: &[Token], interner: &mut SymbolTable) -> Result<Vec<Stx>, ReadError> {
    let mut parser = Parser::new(tokens, interner);
    parser.parse_program()
}

/// 递归下降语法器（§10.1 规则 2：`-er` 后缀上下文类型）。
pub struct Parser<'t, 'i> {
    tokens: &'t [Token],
    interner: &'i mut SymbolTable,
    pos: usize,
    /// 括号深度（防御性上限：拒绝超深嵌套导致的潜在栈溢出）。
    depth: usize,
}

/// 最大嵌套深度（语法器栈保护；超深嵌套报结构化错误而非崩溃）。
///
/// 值域依据（FS-1 修复）：递归下降语法器在默认 2 MiB 测试线程栈上
/// 约 600 层即溢出（实测）——取 256 留 >8× 裕度，同时覆盖下游
/// expander/compile/eval 对同构树的递归（各栈帧同量级）。
const MAX_NESTING_DEPTH: usize = 256;

impl<'t, 'i> Parser<'t, 'i> {
    /// 构造。
    pub fn new(tokens: &'t [Token], interner: &'i mut SymbolTable) -> Self {
        Parser {
            tokens,
            interner,
            pos: 0,
            depth: 0,
        }
    }

    /// 程序 = 顶层形式* EOF。
    pub fn parse_program(&mut self) -> Result<Vec<Stx>, ReadError> {
        let mut forms = Vec::new();
        loop {
            if self.peek().is_eof() {
                return Ok(forms);
            }
            forms.push(self.parse_datum()?);
        }
    }

    /// 单个 datum：字面量 / 符号 / 列表 / 向量 / quote 简写。
    pub fn parse_datum(&mut self) -> Result<Stx, ReadError> {
        let token = self.peek().clone();
        match &token.kind {
            TokenKind::IntLiteral(v) => {
                self.bump();
                Ok(self.literal(StxLiteral::Int(*v), &token))
            }
            TokenKind::FloatLiteral(v) => {
                self.bump();
                Ok(self.literal(StxLiteral::Float(*v), &token))
            }
            TokenKind::StringLiteral(s) => {
                self.bump();
                Ok(self.literal(StxLiteral::Str(s.clone()), &token))
            }
            TokenKind::BoolLiteral(b) => {
                self.bump();
                Ok(self.literal(StxLiteral::Bool(*b), &token))
            }
            TokenKind::NilLiteral => {
                self.bump();
                Ok(self.literal(StxLiteral::Nil, &token))
            }
            TokenKind::Identifier(sym) => {
                self.bump();
                let s = *sym;
                Ok(Stx::symbol(s, token.span, token.scopes.clone()))
            }
            TokenKind::Keyword(kw) => {
                // 关键字作为符号出现（非头位置时由展开器裁决合法性）
                self.bump();
                let kw = *kw;
                let sym = self.interner.keyword_symbol(kw);
                Ok(Stx::symbol(sym, token.span, token.scopes.clone()))
            }
            TokenKind::Operator(_op, sym) => {
                // 运算符 = 标识符（携带 Symbol 句柄，语法层直接符号化）
                self.bump();
                let s = *sym;
                Ok(Stx::symbol(s, token.span, token.scopes.clone()))
            }
            TokenKind::QuoteShorthand => {
                self.bump();
                let quoted = self.parse_datum()?;
                let quote_sym = self.interner.keyword_symbol(kerf_syntax::Keyword::Quote);
                let span = token.span.merge(quoted.span);
                let list = Stx::list(
                    vec![
                        Stx::symbol(quote_sym, token.span, token.scopes.clone()),
                        quoted,
                    ],
                    span,
                    token.scopes.clone(),
                );
                Ok(list)
            }
            TokenKind::Delimiter(Delimiter::OpenParen | Delimiter::OpenBracket) => {
                self.parse_list_like()
            }
            TokenKind::Delimiter(Delimiter::CloseParen | Delimiter::CloseBracket) => {
                let close = match token.kind {
                    TokenKind::Delimiter(d) => d,
                    // _ 臂理由：不可达——外层模式已窄化为 Delimiter，此臂仅满足穷尽性
                    _ => Delimiter::CloseParen,
                };
                Err(ReadError::stray_closing_delimiter(close, token.span))
            }
            TokenKind::Eof => Err(ReadError::new("意外的文件结束（期望表达式）", token.span)),
            TokenKind::MacroInvocation(_) => Err(ReadError::new(
                "宏直调语法 '#name' Stage 0 未启用（接口预留）",
                token.span,
            )),
        }
    }

    /// 列表 / 向量：`(a b c)` 或 `[a b c]`。
    fn parse_list_like(&mut self) -> Result<Stx, ReadError> {
        let open = self.peek().clone();
        let (open_delim, is_vector) = match &open.kind {
            TokenKind::Delimiter(Delimiter::OpenParen) => (Delimiter::OpenParen, false),
            TokenKind::Delimiter(Delimiter::OpenBracket) => (Delimiter::OpenBracket, true),
            // _ 臂理由：不可达——唯一调用点（parse_datum）已窄化为开放分隔符，按圆括号兜底
            _ => (Delimiter::OpenParen, false),
        };
        self.bump();
        self.depth += 1;
        if self.depth > MAX_NESTING_DEPTH {
            self.depth -= 1;
            return Err(ReadError::new(
                format!("嵌套深度超过上限 {}", MAX_NESTING_DEPTH),
                open.span,
            ));
        }
        let mut items = Vec::new();
        let result = loop {
            let t = self.peek();
            match &t.kind {
                TokenKind::Eof => {
                    break Err(ReadError::unclosed_delimiter(open_delim, open.span));
                }
                TokenKind::Delimiter(Delimiter::CloseParen | Delimiter::CloseBracket) => {
                    let close_kind = match &t.kind {
                        TokenKind::Delimiter(d) => *d,
                        // _ 臂理由：不可达——外层模式已窄化为 Delimiter，此臂仅满足穷尽性
                        _ => Delimiter::CloseParen,
                    };
                    let close_span = t.span;
                    self.bump();
                    let expected = Delimiter::closing_of(open_delim);
                    if close_kind != expected {
                        break Err(ReadError::new(
                            format!(
                                "括号不匹配：'{}' 应以 '{}' 关闭，实际 '{}'",
                                open_delim.as_char(),
                                expected.as_char(),
                                close_kind.as_char(),
                            ),
                            close_span,
                        ));
                    }
                    let span = open.span.merge(close_span);
                    let scopes = open.scopes.clone();
                    let stx = if is_vector {
                        Stx {
                            datum: StxDatum::Vector(std::rc::Rc::new(items)),
                            span,
                            scopes,
                            phase: kerf_syntax::Phase::Runtime,
                            uniform_tag: None,
                        }
                    } else {
                        Stx::list(items, span, scopes)
                    };
                    break Ok(stx);
                }
                // _ 臂理由：非 EOF/关闭分隔符的 token（原子与嵌套开放分隔符）即列表元素——递归解析
                _ => match self.parse_datum() {
                    Ok(d) => items.push(d),
                    Err(e) => break Err(e),
                },
            }
        };
        self.depth -= 1;
        result
    }

    // ---- 基础游标 ----

    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn bump(&mut self) -> Option<&Token> {
        let r = self.tokens.get(self.pos);
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        r
    }

    fn literal(&self, value: StxLiteral, token: &Token) -> Stx {
        Stx::literal(value, token.span, token.scopes.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(src: &str) -> Result<Vec<Stx>, ReadError> {
        let mut t = SymbolTable::new();
        read_source(src, 0, &mut t)
    }

    #[test]
    fn parse_nested_list() {
        let mut t = SymbolTable::new();
        let forms = read_source("(+ 1 (* 2 3))", 0, &mut t).unwrap();
        let rendered: Vec<String> = forms
            .iter()
            .map(|s| s.render(&|sym| t.name(sym).to_string()))
            .collect();
        assert_eq!(rendered, vec!["(+ 1 (* 2 3))"]);
    }

    #[test]
    fn parse_multiple_toplevel_forms() {
        let forms = read("(define x 1)\n(+ x 2)\n42").unwrap();
        assert_eq!(forms.len(), 3);
    }

    #[test]
    fn quote_shorthand_expands() {
        let mut t = SymbolTable::new();
        let forms = read_source("'(1 2 3)", 0, &mut t).unwrap();
        // 'x → (quote x)
        assert!(matches!(forms[0].datum, StxDatum::List(_)));
        assert_eq!(
            forms[0].render(&|s| t.name(s).to_string()),
            "(quote (1 2 3))"
        );
    }

    #[test]
    fn unclosed_paren_error_has_span() {
        let err = read("(define x 1").unwrap_err();
        assert!(err.message.contains("未闭合"));
        assert_eq!(err.span.start, 0);
    }

    #[test]
    fn mismatched_brackets_error() {
        let err = read("(+ 1 2]").unwrap_err();
        assert!(err.message.contains("括号不匹配"));
    }

    #[test]
    fn stray_close_error() {
        let err = read(")").unwrap_err();
        assert!(err.message.contains("多余的关闭括号"));
    }

    #[test]
    fn vector_datum() {
        let forms = read("[1 2 3]").unwrap();
        assert!(matches!(forms[0].datum, StxDatum::Vector(ref v) if v.len() == 3));
    }

    #[test]
    fn spans_merge_over_list() {
        let src = "(do 1)";
        let mut t = SymbolTable::new();
        let forms = read_source(src, 0, &mut t).unwrap();
        let stx = &forms[0];
        assert_eq!(stx.span.start, 0);
        assert_eq!(stx.span.end, src.len() as u32);
    }

    #[test]
    fn keyword_as_symbol_datum() {
        let forms = read("fn").unwrap();
        assert!(matches!(forms[0].datum, StxDatum::Symbol(_)));
    }
}
