//! 负向 Reader 测试（tests/v0/stage0/plan/negative_reader_tests.rs ↔
//! docs/tests/v0/stage0/plan/negative-tests.md）。
//!
//! §9.4.3 负向测试优先原则的定向扩张：本文件覆盖 **Read 阶段（E0001）**
//! 全部错误族——词法（字符串/数字/非法字符/转义/注释）与语法
//! （括号三态：未闭合/多余/不匹配）。
//!
//! 每行 = 1 case（表格驱动）。所有期望消息/Span 均经实测校准
//! （预期 Err 实际 Ok 的候选已剔除并记录于文档）。
//!
//! 语义边界（实测确认，非负例）：
//! - `$` 是合法标识符起始字符（`$x` 经 Reader、在 Run 阶段报未绑定）；
//! - `1.0e999` 浮点字面量溢出到 inf 被**静默接受**（对照：整数溢出报错）；
//! - 字符串可跨行（未闭合才报错）。
//!
//! 已知边界存档（实测确认）：
//! - FS-1 已修复：MAX_NESTING_DEPTH 10_000 → 256（栈安全裕度 >8×）
//!   ——见 `deep_nesting_guard_structured_error`（含 255 层边界正例）。

use kerf_driver::{run_source, Stage};

/// 通用断言：src 失败于指定阶段，渲染错误含消息子串（返回错误供深断言）。
fn expect_err(src: &str, stage: Stage, msg: &str) -> kerf_driver::DriverError {
    let err = match run_source(src, "neg.krf") {
        Ok(_) => panic!("期望报错，实际 Ok：{}", src),
        Err(e) => e,
    };
    assert_eq!(err.stage, stage, "阶段不符：{}", src);
    assert!(
        err.rendered.contains(msg),
        "消息缺少「{}」：{}\n完整错误：{}",
        msg,
        src,
        err.rendered
    );
    err
}

/// 断言错误 Span 精确指向（字节偏移主键，§19.1 陷阱 1）。
fn expect_span(err: &kerf_driver::DriverError, src: &str, start: u32, end: u32) {
    assert_eq!(
        err.diagnostic.primary_span.start, start,
        "Span.start 不符：{}（实际 {}）",
        src, start
    );
    assert_eq!(
        err.diagnostic.primary_span.end, end,
        "Span.end 不符：{}（实际 {}）",
        src, end
    );
}

/// Read 阶段负例（4 case）：未闭合字符串（含转义中 EOF / 跨行后 EOF）。
#[test]
fn unclosed_string_literals() {
    let cases: &[(&str, &str, u32, u32)] = &[
        ("\"abc", "字符串未闭合", 0, 4),
        ("\"abc\\", "字符串未闭合", 0, 5),
        ("\"abc\ndef", "字符串未闭合", 0, 8),
        ("(+ (quote \"abc) 1)", "字符串未闭合", 10, 18),
    ];
    for (src, msg, s, e) in cases {
        let err = expect_err(src, Stage::Read, msg);
        expect_span(&err, src, *s, *e);
    }
}

/// 非法转义序列（3 case）：未支持的转义字符。
#[test]
fn illegal_escape_sequences() {
    let cases: &[(&str, &str)] = &[
        ("\"a\\q1\"", "非法转义字符"),
        ("\"\\z\"", "非法转义字符"),
        ("\"a\\u12\"", "非法转义字符"),
    ];
    for (src, msg) in cases {
        expect_err(src, Stage::Read, msg);
    }
}

/// 贪婪数字拒绝（§19.1 陷阱 4）（5 case）：
/// 数字后紧跟标识符字符 / 多点浮点。
#[test]
fn greedy_number_rejections() {
    let cases: &[(&str, &str)] = &[
        ("123abc", "数字 '123' 后紧跟标识符字符 'a'"),
        ("1.5x", "数字 '1.5' 后紧跟标识符字符 'x'"),
        ("12e5x", "数字 '12e5' 后紧跟标识符字符 'x'"),
        ("-4ab", "数字 '-4' 后紧跟标识符字符 'a'"),
        ("1.2.3", "非法字符 '.'"),
    ];
    for (src, msg) in cases {
        let err = expect_err(src, Stage::Read, msg);
        assert!(
            err.rendered.contains("error[E0001]"),
            "应携带 E0001：{}",
            src
        );
    }
}

/// 非法字符（14 case）：不在标识符/数字/分隔符字符集内的字符。
/// 注意 `$` 合法（见文件头注）；`#` 仅在 `#|` 块注释开头合法。
#[test]
fn illegal_characters() {
    let cases: &[(&str, &str)] = &[
        ("@", "非法字符 '@'"),
        ("(+ @ 1)", "非法字符 '@'"),
        ("(+ ~ 1)", "非法字符 '~'"),
        ("(+ # 1)", "非法字符 '#'"),
        ("#", "非法字符 '#'"),
        ("(+ 1, 2)", "非法字符 ','"),
        ("(+ ^ 1)", "非法字符 '^'"),
        ("`x", "非法字符 '`'"),
        ("(+ § 1)", "非法字符 '§'"),
        ("(+ ÷ 1)", "非法字符 '÷'"),
        ("(+ © 1)", "非法字符 '©'"),
        ("(+ ¿ 1)", "非法字符 '¿'"),
        ("(+ £ 1)", "非法字符 '£'"),
        ("(+ ± 1)", "非法字符 '±'"),
    ];
    for (src, msg) in cases {
        expect_err(src, Stage::Read, msg);
    }
}

/// 引号简写 `'` 后跟 EOF（3 case）：期望表达式却遇文件结束。
#[test]
fn quote_shorthand_at_eof() {
    let cases: &[(&str, u32)] = &[("'", 1), ("(+ 1 '", 6), ("(quote '", 8)];
    for (src, span_start) in cases {
        let err = expect_err(src, Stage::Read, "意外的文件结束");
        expect_span(&err, src, *span_start, *span_start);
    }
}

/// 多余的关闭括号（5 case）：闭侧括号无对应开侧。
#[test]
fn stray_closing_delimiters() {
    let cases: &[(&str, &str, u32, u32)] = &[
        (")", "多余的关闭括号：')'", 0, 1),
        ("]", "多余的关闭括号：']'", 0, 1),
        ("(+ 1 2) )", "多余的关闭括号：')'", 8, 9),
        ("[1 2] ]", "多余的关闭括号：']'", 6, 7),
        ("(+ 1 2)\n)", "多余的关闭括号：')'", 8, 9),
    ];
    for (src, msg, s, e) in cases {
        let err = expect_err(src, Stage::Read, msg);
        expect_span(&err, src, *s, *e);
    }
}

/// 未闭合括号（7 case）：开侧括号未在 EOF 前关闭（Span 指向开侧）。
#[test]
fn unclosed_delimiters() {
    let cases: &[(&str, &str, u32, u32)] = &[
        ("(+ 1 2", "括号未闭合：缺少 ')'", 0, 1),
        ("[1 2", "括号未闭合：缺少 ']'", 0, 1),
        ("[1 2 3", "括号未闭合：缺少 ']'", 0, 1),
        ("(define x 1", "括号未闭合：缺少 ')'", 0, 1),
        ("((((", "括号未闭合：缺少 ')'", 3, 4),
        ("(let ((x 1)", "括号未闭合：缺少 ')'", 5, 6),
        ("[\n", "括号未闭合：缺少 ']'", 0, 1),
    ];
    for (src, msg, s, e) in cases {
        let err = expect_err(src, Stage::Read, msg);
        expect_span(&err, src, *s, *e);
    }
}

/// 括号不匹配（4 case）：开闭类型交叉（( 用 ] 关、[ 用 ) 关）。
#[test]
fn mismatched_delimiters() {
    let cases: &[(&str, &str)] = &[
        ("(+ 1 2]", "括号不匹配：'(' 应以 ')' 关闭，实际 ']'"),
        ("[1 2)", "括号不匹配：'[' 应以 ']' 关闭，实际 ')'"),
        ("([)])", "括号不匹配：'[' 应以 ']' 关闭，实际 ')'"),
        ("(+ [1 2)", "括号不匹配：'[' 应以 ']' 关闭，实际 ')'"),
    ];
    for (src, msg) in cases {
        expect_err(src, Stage::Read, msg);
    }
}

/// 指数缺数字（4 case）：科学计数法指数后必须跟数字。
#[test]
fn bad_exponent_literals() {
    let cases: &[(&str, &str)] = &[
        ("1e", "指数缺少数字"),
        ("1e+", "指数缺少数字"),
        ("1e-", "指数缺少数字"),
        ("2.5e", "指数缺少数字"),
    ];
    for (src, msg) in cases {
        expect_err(src, Stage::Read, msg);
    }
}

/// 整数字面量超出 i64（2 case）。
#[test]
fn integer_literal_overflow() {
    for src in ["99999999999999999999999", "-99999999999999999999999"] {
        expect_err(src, Stage::Read, "整数超出范围");
    }
}

/// 块注释未闭合（3 case，含嵌套）。
#[test]
fn unclosed_block_comments() {
    let cases: &[(&str, &str)] = &[
        ("#| abc", "块注释 #|...|# 未闭合"),
        ("#|x", "块注释 #|...|# 未闭合"),
        ("#| #| nested", "块注释 #|...|# 未闭合"),
    ];
    for (src, msg) in cases {
        expect_err(src, Stage::Read, msg);
    }
}

/// 渲染形状完整断言（4 case）：error[E0001] + `-->` 位置行 + 源摘录行
/// （§2.2 原则 16：人类可感知输出是一等公民）。
#[test]
fn reader_error_rendering_shape() {
    // 单行：摘录含 `1 |` 行号栏 + 脱字符 ^ 指向
    let err = run_source("(+ # 1)", "neg.krf").unwrap_err();
    assert!(err.rendered.contains("error[E0001]: 非法字符 '#'"));
    assert!(err.rendered.contains("--> neg.krf:1:4"));
    assert!(err.rendered.contains("1 | (+ # 1)"));
    assert!(err.rendered.contains("^"));
    // 双行源：位置行号随错误所在行递增
    let err2 = run_source("(+ 1 2)\n)", "neg.krf").unwrap_err();
    assert!(
        err2.rendered.contains("--> neg.krf:2:1"),
        "实际：{}",
        err2.rendered
    );
    // 注释后未闭合：行号指向第二行
    let err3 = run_source("; comment\n(+ 1 2\n", "neg.krf").unwrap_err();
    assert!(
        err3.rendered.contains("--> neg.krf:2:1"),
        "实际：{}",
        err3.rendered
    );
    // Display 前缀携带阶段标识 [read]
    let err4 = run_source("(+ 1", "neg.krf").unwrap_err();
    assert!(err4.to_string().starts_with("[read] error[E0001]"));
}

/// E0001 码结构化断言（1 case）：DiagnosticCode(1)（诊断是数据，§8.7）。
#[test]
fn reader_error_code_is_e0001() {
    let err = run_source("(+ # 1)", "neg.krf").unwrap_err();
    assert_eq!(
        err.diagnostic.code,
        Some(kerf_span::DiagnosticCode(1)),
        "Reader 错误必须编号为 E0001"
    );
    assert_eq!(err.diagnostic.primary_span.file_id, 0);
    assert!(err.diagnostic.is_error());
}

/// FS-1 已修复（MAX_NESTING_DEPTH 10_000→256，留 >8× 栈裕度）：
/// 超深嵌套报结构化错误而非栈溢出（守卫先于实际溢出触发）。
#[test]
fn deep_nesting_guard_structured_error() {
    let src = format!("{}1{}", "(".repeat(10_001), ")".repeat(10_001));
    let mut t = kerf_syntax::SymbolTable::new();
    let err = kerf_reader::read_source(&src, 0, &mut t).unwrap_err();
    assert!(
        err.message.contains("嵌套深度超过上限 256"),
        "应报结构化深度上限错误：{}",
        err.message
    );
    // 边界正例：255 层合法（上限内不误拦）
    let ok_src = format!("{}1{}", "(".repeat(255), ")".repeat(255));
    let mut t2 = kerf_syntax::SymbolTable::new();
    assert!(
        kerf_reader::read_source(&ok_src, 0, &mut t2).is_ok(),
        "255 层嵌套应在上限内"
    );
}
