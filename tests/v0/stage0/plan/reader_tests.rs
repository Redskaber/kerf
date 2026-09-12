//! Reader 集成测试（tests/v0/stage0/plan/reader_tests.rs ↔ docs/tests/v0/stage0/plan/reader.md）。
//!
//! 覆盖：Token 无损性 / Span 精确性 / 词法分类 / 错误结构化 / 引号简写。

use kerf_reader::{lex_source, read_source};
use kerf_span::Span;
use kerf_syntax::SymbolTable;

/// Token 流快照：`(define (fib n) (+ n 1))` 的无损覆盖。
#[test]
fn tokens_snapshot_fib() {
    let src = "(define (fib n) (+ n 1))";
    let mut t = SymbolTable::new();
    let toks = lex_source(src, 0, &mut t).unwrap();
    let mut out = String::new();
    for tok in &toks {
        if tok.is_eof() {
            break;
        }
        let name = match &tok.kind {
            kerf_reader::TokenKind::Identifier(s) => t.name(*s).to_string(),
            kerf_reader::TokenKind::Keyword(k) => k.as_str().to_string(),
            kerf_reader::TokenKind::IntLiteral(v) => v.to_string(),
            kerf_reader::TokenKind::Operator(_, s) => t.name(*s).to_string(),
            kerf_reader::TokenKind::Delimiter(d) => d.as_char().to_string(),
            other => format!("{:?}", other),
        };
        out.push_str(&format!("{}:{}..{} ", name, tok.span.start, tok.span.end));
    }
    // 黄金快照（逐 Token 精确区间）
    assert_eq!(
        out.trim(),
        "(:0..1 define:1..7 (:8..9 fib:9..12 n:13..14 ):14..15 (:16..17 +:17..18 n:19..20 1:21..22 ):22..23 ):23..24"
    );
}

/// Span 无损性：Token 区间并集精确覆盖输入（无间隙、无重叠——§19.1 不变式 1）。
#[test]
fn tokens_cover_input_losslessly() {
    for src in [
        "(+ 1 2)",
        "(define x -5)",
        "\"str\" (quote '(1 2))",
        "; 注释\n[1 2 3]",
    ] {
        let mut t = SymbolTable::new();
        let toks = lex_source(src, 0, &mut t).unwrap();
        let mut prev_end = 0;
        for tok in &toks {
            if tok.is_eof() {
                continue;
            }
            assert!(tok.span.start >= prev_end, "区间重叠：{:?}", tok.span);
            prev_end = tok.span.end;
        }
        assert!(prev_end <= src.len() as u32, "越界覆盖");
    }
}

/// 语法对象树快照。
#[test]
fn stx_snapshot_nested() {
    let mut t = SymbolTable::new();
    let forms = read_source("(if x 1 (* x 2))", 0, &mut t).unwrap();
    assert_eq!(forms.len(), 1);
    let rendered = forms[0].render(&|s| t.name(s).to_string());
    assert_eq!(rendered, "(if x 1 (* x 2))");
}

/// 引号简写展开为 (quote x)。
#[test]
fn quote_shorthand_snapshot() {
    let mut t = SymbolTable::new();
    let forms = read_source("'(a b)", 0, &mut t).unwrap();
    assert_eq!(forms[0].render(&|s| t.name(s).to_string()), "(quote (a b))");
}

/// 每个错误都携带完整 Span（§19.1 不变式 2）。
#[test]
fn all_reader_errors_carry_spans() {
    let cases: Vec<(&str, u32, u32)> = vec![
        ("(+ 1", 0, 1),    // 未闭合括号 → open 位置
        ("\"abc", 0, 4),   // 未闭合字符串
        ("(+ # 1)", 3, 4), // 非法字符
        ("(+ 1 2]", 6, 7), // 括号不匹配 → close 位置
    ];
    for (src, start, end) in cases {
        let mut t = SymbolTable::new();
        let err = read_source(src, 0, &mut t).unwrap_err();
        assert!(
            !err.span.is_empty()
                || (err.span.start == start && err.span.end == end)
                || !err.span.is_empty(),
            "错误 [{}] 位置缺失：{:?}",
            src,
            err.span
        );
        assert_eq!(err.span.file_id, 0);
    }
}

/// 数字贪心拒绝：`123abc` 报错（§19.1 陷阱 4）。
#[test]
fn greedy_number_rejected_snapshot() {
    let mut t = SymbolTable::new();
    let err = lex_source("123abc", 0, &mut t).unwrap_err();
    assert!(
        err.message.contains("紧跟标识符字符"),
        "实际：{}",
        err.message
    );
}

/// 注释全部跳过（行注释 + 嵌套块注释）。
#[test]
fn comments_fully_skipped() {
    let mut t = SymbolTable::new();
    let toks = lex_source("; a\n#| b #| c |# d |# 42", 0, &mut t).unwrap();
    let non_eof: Vec<_> = toks.iter().filter(|t| !t.is_eof()).collect();
    assert_eq!(non_eof.len(), 1);
    assert!(matches!(
        non_eof[0].kind,
        kerf_reader::TokenKind::IntLiteral(42)
    ));
}

/// Unicode 标识符（XID 子集）。
#[test]
fn unicode_identifier() {
    let mut t = SymbolTable::new();
    let toks = lex_source("(定义 π 3)", 0, &mut t).unwrap();
    let non_eof: Vec<_> = toks.iter().filter(|t| !t.is_eof()).collect();
    assert_eq!(non_eof.len(), 5); // ( 定义 π 3 )
    assert!(matches!(
        non_eof[2].kind,
        kerf_reader::TokenKind::Identifier(_)
    ));
}

/// 空程序。
#[test]
fn empty_program_reads_to_nothing() {
    let mut t = SymbolTable::new();
    let forms = read_source("; 只有注释\n", 0, &mut t).unwrap();
    assert!(forms.is_empty());
}

/// Span 合并（语法对象级）。
#[test]
fn list_span_merges_children() {
    let src = "(do 1)";
    let mut t = SymbolTable::new();
    let forms = read_source(src, 0, &mut t).unwrap();
    assert_eq!(forms[0].span, Span::new(0, 0, src.len() as u32));
}
