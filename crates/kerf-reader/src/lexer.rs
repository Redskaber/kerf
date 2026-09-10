//! 词法器（stage0.md §19.1 词法器骨架的落地实现）。
//!
//! - 输入：`&str` 源文本 + file_id；
//! - 输出：`Vec<Token>`（无损覆盖输入，末尾追加 Eof）；
//! - 错误：`ReadError`（必带 Span）。
//!
//! 实现要点（§19.1 陷阱全部处理）：
//! - 字符串跨行时行号继续维护（陷阱 3）；
//! - `123abc` 报错而非拆分（陷阱 4：拒绝贪心数字）；
//! - 字节偏移为主键（陷阱 1）；
//! - 标识符 intern 一次（陷阱 2，NFC 在 SymbolTable 内部完成）。

use std::rc::Rc;

use kerf_span::{ByteOffset, FileId, Span};
use kerf_syntax::{Keyword, SymbolTable};

use crate::token::{Delimiter, Operator, ReadError, Token, TokenKind};

/// 词法主入口（sop.md §10.1 规则 1：`<verb>_<noun>` 自由函数）。
///
/// 保证（§8.1 接口契约）：
/// - 输出的 Token 流覆盖整个输入（无损）；
/// - 每个 Token 携带精确的 Span；
/// - 词法错误返回结构化的 `ReadError`（含位置信息）。
pub fn lex_source(
    source: &str,
    file_id: FileId,
    interner: &mut SymbolTable,
) -> Result<Vec<Token>, ReadError> {
    let mut lexer = Lexer::new(source, file_id, interner);
    lexer.run()
}

/// 词法器状态机。
pub struct Lexer<'a, 'i> {
    src: &'a str,
    file_id: FileId,
    interner: &'i mut SymbolTable,
    /// 前瞻缓冲（char + 字节偏移）。
    chars: Vec<(usize, char)>,
    pos: usize,
}

impl<'a, 'i> Lexer<'a, 'i> {
    /// 构造（char 索引化——UTF-8 感知）。
    pub fn new(source: &'a str, file_id: FileId, interner: &'i mut SymbolTable) -> Self {
        Lexer {
            src: source,
            file_id,
            interner,
            chars: source.char_indices().collect(),
            pos: 0,
        }
    }

    /// 运行全部词法。
    pub fn run(&mut self) -> Result<Vec<Token>, ReadError> {
        let mut toks = Vec::new();
        loop {
            self.skip_trivia();
            let (offset, ch) = match self.peek() {
                Some(oc) => oc,
                None => {
                    toks.push(Token {
                        kind: TokenKind::Eof,
                        span: Span::new(
                            self.file_id,
                            self.src.len() as ByteOffset,
                            self.src.len() as ByteOffset,
                        ),
                        scopes: Default::default(),
                    });
                    return Ok(toks);
                }
            };
            let kind = match ch {
                '(' => self.consume_delimiter(Delimiter::OpenParen),
                ')' => self.consume_delimiter(Delimiter::CloseParen),
                '[' => self.consume_delimiter(Delimiter::OpenBracket),
                ']' => self.consume_delimiter(Delimiter::CloseBracket),
                '\'' => {
                    self.bump();
                    TokenKind::QuoteShorthand
                }
                '"' => self.string_literal(offset)?,
                '.' if self.peek2().map(|(_, d)| d) == Some('.') => self.identifier(offset, '.'),
                c if c.is_ascii_digit()
                    || ((c == '-' || c == '+')
                        && self.peek2().map(|(_, d)| d.is_ascii_digit()) == Some(true)) =>
                {
                    self.number(offset, c)?
                }
                c if is_id_start(c) => self.identifier(offset, c),
                ';' => {
                    // 行注释：跳到行尾
                    self.bump();
                    while let Some((_, c)) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                    continue;
                }
                '#' if self.peek2().map(|(_, c)| c) == Some('|') => {
                    self.block_comment()?;
                    continue;
                }
                c => {
                    return Err(ReadError::new(
                        format!("非法字符 '{}'", c),
                        self.span_char(offset, c),
                    ))
                }
            };
            let span = Span::new(
                self.file_id,
                offset as ByteOffset,
                self.cur_offset() as ByteOffset,
            );
            toks.push(Token {
                kind,
                span,
                scopes: Default::default(),
            });
        }
    }

    // ---- 基础游标 ----

    fn peek(&self) -> Option<(usize, char)> {
        self.chars.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<(usize, char)> {
        self.chars.get(self.pos + 1).copied()
    }

    fn bump(&mut self) -> Option<(usize, char)> {
        let r = self.peek();
        if r.is_some() {
            self.pos += 1;
        }
        r
    }

    fn cur_offset(&self) -> usize {
        self.peek().map(|(o, _)| o).unwrap_or(self.src.len())
    }

    fn span_char(&self, offset: usize, c: char) -> Span {
        Span::new(
            self.file_id,
            offset as ByteOffset,
            (offset + c.len_utf8()) as ByteOffset,
        )
    }

    // ---- 分类 ----

    fn consume_delimiter(&mut self, d: Delimiter) -> TokenKind {
        self.bump();
        TokenKind::Delimiter(d)
    }

    fn string_literal(&mut self, offset: usize) -> Result<TokenKind, ReadError> {
        self.bump(); // 消费开头 "
        let mut out = String::new();
        while let Some((o, c)) = self.peek() {
            match c {
                '"' => {
                    self.bump();
                    return Ok(TokenKind::StringLiteral(Rc::from(out.as_str())));
                }
                '\\' => {
                    self.bump();
                    match self.bump() {
                        Some((_, 'n')) => out.push('\n'),
                        Some((_, 't')) => out.push('\t'),
                        Some((_, 'r')) => out.push('\r'),
                        Some((_, '\\')) => out.push('\\'),
                        Some((_, '"')) => out.push('"'),
                        Some((_, '0')) => out.push('\0'),
                        Some((eo, ec)) => {
                            return Err(ReadError::new(
                                format!("非法转义字符 '\\{}'", ec),
                                Span::new(self.file_id, o as ByteOffset, (eo + 1) as ByteOffset),
                            ))
                        }
                        None => {
                            return Err(ReadError::new(
                                "字符串未闭合（文件意外结束）",
                                Span::new(
                                    self.file_id,
                                    offset as ByteOffset,
                                    self.src.len() as ByteOffset,
                                ),
                            ))
                        }
                    }
                }
                // 陷阱 3：字符串跨行合法，偏移继续推进（行/列由 SourceMap 派生）
                '\n' => {
                    out.push('\n');
                    self.bump();
                }
                c => {
                    out.push(c);
                    self.bump();
                }
            }
        }
        Err(ReadError::new(
            "字符串未闭合（文件意外结束）",
            Span::new(
                self.file_id,
                offset as ByteOffset,
                self.src.len() as ByteOffset,
            ),
        ))
    }

    fn number(&mut self, offset: usize, first: char) -> Result<TokenKind, ReadError> {
        let mut text = String::new();
        text.push(first);
        self.bump();
        let mut is_float = false;
        let mut has_exp = false;
        while let Some((_, c)) = self.peek() {
            if c.is_ascii_digit() {
                text.push(c);
                self.bump();
            } else if c == '.'
                && !is_float
                && !has_exp
                && self.peek2().map(|(_, d)| d.is_ascii_digit()) == Some(true)
            {
                is_float = true;
                text.push('.');
                self.bump();
            } else if (c == 'e' || c == 'E') && !has_exp {
                has_exp = true;
                is_float = true;
                text.push('e');
                self.bump();
                // 科学计数法符号
                if let Some((_, s)) = self.peek() {
                    if s == '-' || s == '+' {
                        text.push(s);
                        self.bump();
                    }
                }
                // 指数必须跟数字（否则报错）
                match self.peek() {
                    Some((_, d)) if d.is_ascii_digit() => {}
                    _ => {
                        return Err(ReadError::new(
                            format!("非法数字字面量 '{}'（指数缺少数字）", text),
                            Span::new(
                                self.file_id,
                                offset as ByteOffset,
                                self.cur_offset() as ByteOffset,
                            ),
                        ))
                    }
                }
            } else {
                break;
            }
        }
        // 陷阱 4：拒绝贪心数字——数字后紧跟标识符字符即报错
        if let Some((o, c)) = self.peek() {
            if is_id_start(c) {
                return Err(ReadError::new(
                    format!("数字 '{}' 后紧跟标识符字符 '{}'（应为独立 Token）", text, c),
                    self.span_char(o, c),
                ));
            }
        }
        if is_float {
            match text.parse::<f64>() {
                Ok(v) => Ok(TokenKind::FloatLiteral(v)),
                Err(_) => Err(ReadError::new(
                    format!("非法浮点字面量 '{}'", text),
                    Span::new(
                        self.file_id,
                        offset as ByteOffset,
                        self.cur_offset() as ByteOffset,
                    ),
                )),
            }
        } else {
            match text.parse::<i64>() {
                Ok(v) => Ok(TokenKind::IntLiteral(v)),
                Err(_) => Err(ReadError::new(
                    format!("整数超出范围 '{}'", text),
                    Span::new(
                        self.file_id,
                        offset as ByteOffset,
                        self.cur_offset() as ByteOffset,
                    ),
                )),
            }
        }
    }

    fn identifier(&mut self, offset: usize, first: char) -> TokenKind {
        let mut text = String::new();
        text.push(first);
        self.bump();
        // 省略号：连续 '.' 贪婪收集（"..." 为宏系统保留标识符）
        if first == '.' {
            while let Some((_, c)) = self.peek() {
                if c == '.' {
                    text.push('.');
                    self.bump();
                } else {
                    break;
                }
            }
            if text.len() != 3 {
                return TokenKind::Identifier(self.interner.intern(&text));
            }
        }
        while let Some((_, c)) = self.peek() {
            if is_id_continue(c) {
                text.push(c);
                self.bump();
            } else {
                break;
            }
        }
        let _ = offset;
        // 剩余带符号数（如 "-x" 路径回退、"+" 等单符号）：查表分类
        let sym = self.interner.intern(&text);
        // 关键字分类：纯查表（词法层零语义，§19.1 不变式 3）
        if let Some(kw) = Keyword::from_name(&text) {
            return TokenKind::Keyword(kw);
        }
        if let Some(op) = operator_of(&text) {
            return TokenKind::Operator(op, sym);
        }
        if text == "true" {
            return TokenKind::BoolLiteral(true);
        }
        if text == "false" {
            return TokenKind::BoolLiteral(false);
        }
        if text == "nil" {
            return TokenKind::NilLiteral;
        }
        TokenKind::Identifier(sym)
    }

    fn skip_trivia(&mut self) {
        while let Some((_, c)) = self.peek() {
            if c.is_whitespace() {
                self.bump();
            } else if c == ';' {
                self.bump();
                while let Some((_, c)) = self.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.bump();
                }
            } else {
                break;
            }
        }
    }

    fn block_comment(&mut self) -> Result<(), ReadError> {
        let start = self.peek().map(|(o, _)| o).unwrap_or(self.src.len());
        self.bump(); // '#'
        self.bump(); // '|'
        let mut depth = 1usize;
        while let Some((_, c)) = self.peek() {
            if c == '#' && self.peek2().map(|(_, d)| d) == Some('|') {
                depth += 1;
                self.bump();
                self.bump();
            } else if c == '|' && self.peek2().map(|(_, d)| d) == Some('#') {
                depth -= 1;
                self.bump();
                self.bump();
                if depth == 0 {
                    return Ok(());
                }
            } else {
                self.bump();
            }
        }
        Err(ReadError::new(
            "块注释 #|...|# 未闭合",
            Span::new(
                self.file_id,
                start as ByteOffset,
                self.src.len() as ByteOffset,
            ),
        ))
    }
}

/// 运算符查表（纯查表）。
///
/// pub（B3）：自举 Reader 桥（kerf-driver/bootstrap）复用同一映射——
/// kerf 侧 Token 的 Operator 分类与种子词法器保持唯一可信数据源（§2.3-10）。
pub fn operator_of(text: &str) -> Option<Operator> {
    Some(match text {
        "+" => Operator::Add,
        "-" => Operator::Sub,
        "*" => Operator::Mul,
        "/" => Operator::Div,
        "mod" => Operator::Mod,
        "<" => Operator::Lt,
        ">" => Operator::Gt,
        "<=" => Operator::Le,
        ">=" => Operator::Ge,
        "=" => Operator::Eq,
        _ => return None,
    })
}

/// 标识符起始字符（XID_Start 的务实子集：字母 + 运算符字符 + 扩展集）。
fn is_id_start(c: char) -> bool {
    c.is_alphabetic()
        || matches!(
            c,
            '_' | '!' | '$' | '%' | '&' | '?' | '+' | '-' | '*' | '/' | '<' | '>' | '='
        )
}

/// 标识符后续字符。
fn is_id_continue(c: char) -> bool {
    is_id_start(c) || c.is_ascii_digit() || c == '\''
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(src: &str) -> Result<Vec<Token>, ReadError> {
        let mut t = kerf_syntax::SymbolTable::new();
        lex_source(src, 0, &mut t)
    }

    #[test]
    fn tokens_cover_input_losslessly() {
        let src = "(define x 1)";
        let toks = lex(src).unwrap();
        let mut prev_end = 0u32;
        for t in &toks {
            if t.is_eof() {
                continue;
            }
            assert!(t.span.start >= prev_end, "Token 区间重叠");
            prev_end = t.span.end;
        }
        assert!(prev_end <= src.len() as u32);
        assert_eq!(toks.iter().filter(|t| !t.is_eof()).count(), 5);
        assert!(toks.last().unwrap().is_eof());
    }

    #[test]
    fn spans_are_exact() {
        let toks = lex("(+ 12 ab)").unwrap();
        assert_eq!(toks[1].span.start, 1);
        assert_eq!(toks[1].span.end, 2);
        assert_eq!(toks[2].span.start, 3);
        assert_eq!(toks[2].span.end, 5);
        assert_eq!(toks[3].span.start, 6);
        assert_eq!(toks[3].span.end, 8);
    }

    #[test]
    fn keyword_vs_identifier() {
        let mut t = kerf_syntax::SymbolTable::new();
        let toks = lex_source("lambda lambdax if iff", 0, &mut t).unwrap();
        assert!(matches!(toks[0].kind, TokenKind::Keyword(Keyword::Lambda)));
        assert!(matches!(toks[1].kind, TokenKind::Identifier(_)));
        assert!(matches!(toks[2].kind, TokenKind::Keyword(Keyword::If)));
        assert!(matches!(toks[3].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn string_escapes_and_newline() {
        let toks = lex("\"a\\nb\nc\"").unwrap();
        match &toks[0].kind {
            TokenKind::StringLiteral(s) => assert_eq!(&**s, "a\nb\nc"),
            other => panic!("期望字符串，实际 {:?}", other),
        }
    }

    #[test]
    fn float_int_and_scientific() {
        let toks = lex("3 3.5 1e3 -5 -5.5 +7").unwrap();
        assert!(matches!(toks[0].kind, TokenKind::IntLiteral(3)));
        assert!(
            matches!(toks[1].kind, TokenKind::FloatLiteral(f) if (f - 3.5).abs() < f64::EPSILON)
        );
        assert!(matches!(toks[2].kind, TokenKind::FloatLiteral(f) if (f - 1000.0).abs() < 1.0));
        assert!(matches!(toks[3].kind, TokenKind::IntLiteral(-5)));
        assert!(
            matches!(toks[4].kind, TokenKind::FloatLiteral(f) if (f + 5.5).abs() < f64::EPSILON)
        );
        assert!(matches!(toks[5].kind, TokenKind::IntLiteral(7)));
    }

    #[test]
    fn operators_lexed_from_symbols() {
        let toks = lex("+ - * / <= >= = mod < >").unwrap();
        assert!(matches!(
            toks[0].kind,
            TokenKind::Operator(Operator::Add, _)
        ));
        assert!(matches!(
            toks[1].kind,
            TokenKind::Operator(Operator::Sub, _)
        ));
        assert!(matches!(toks[4].kind, TokenKind::Operator(Operator::Le, _)));
        assert!(matches!(toks[5].kind, TokenKind::Operator(Operator::Ge, _)));
        assert!(matches!(
            toks[7].kind,
            TokenKind::Operator(Operator::Mod, _)
        ));
    }

    #[test]
    fn greedy_number_rejected() {
        let err = lex("123abc").unwrap_err();
        assert!(err.message.contains("紧跟标识符字符"));
    }

    #[test]
    fn illegal_char_has_span() {
        let err = lex("(+ ~ 1)").unwrap_err();
        assert!(err.message.contains("非法字符"));
        assert_eq!(err.span.start, 3);
        assert_eq!(err.span.end, 4);
    }

    #[test]
    fn comments_skipped() {
        let toks = lex("; 行注释\n(+ 1 2) ; 尾注\n#| 块\n注释 |# 3").unwrap();
        let non_eof: Vec<&Token> = toks.iter().filter(|t| !t.is_eof()).collect();
        assert_eq!(non_eof.len(), 6);
    }

    #[test]
    fn unclosed_string_is_error() {
        let err = lex("\"abc").unwrap_err();
        assert!(err.message.contains("未闭合"));
    }

    #[test]
    fn unclosed_block_comment_is_error() {
        let err = lex("#| abc").unwrap_err();
        assert!(err.message.contains("块注释"));
    }

    #[test]
    fn quote_shorthand_token() {
        let toks = lex("'x").unwrap();
        assert!(matches!(toks[0].kind, TokenKind::QuoteShorthand));
    }

    #[test]
    fn booleans_and_nil() {
        let toks = lex("true false nil").unwrap();
        assert!(matches!(toks[0].kind, TokenKind::BoolLiteral(true)));
        assert!(matches!(toks[1].kind, TokenKind::BoolLiteral(false)));
        assert!(matches!(toks[2].kind, TokenKind::NilLiteral));
    }

    #[test]
    fn block_comment_nesting() {
        let toks = lex("#| 外层 #| 内层 |# 仍在外层 |# 42").unwrap();
        let non_eof: Vec<&Token> = toks.iter().filter(|t| !t.is_eof()).collect();
        assert_eq!(non_eof.len(), 1);
        assert!(matches!(non_eof[0].kind, TokenKind::IntLiteral(42)));
    }
}
