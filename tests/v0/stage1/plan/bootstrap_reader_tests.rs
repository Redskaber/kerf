//! B3 自举 Reader parity 套件（Stage 1 批次 B / MUV 24-c）。
//!
//! 验收门（plan §5 批次 B「B3 Reader kerf 重写」）：
//! - **字节级 parity**：自举 Reader（kerf 源码，VM 上运行）与种子 Reader
//!   （Rust）在正/负/奇异语料上——Stx 树（datum + Span 递归比对，符号
//!   按名等价）与错误（消息 + Span）逐字节一致；
//! - **错误次序 parity**：双错误源（数字溢出 + 后置词法错误）首错位置
//!   与种子一致（str-int-valid? 前置校验的契约验证）；
//! - **高阶函数直测**：map/filter/foldl/for-each（kerf 源码实现）经
//!   自检面驱动（builtin 作 f——跨程序安全的函数值）；
//! - **Reader 原语**：str->pos-chars / char-whitespace? / char-alphabetic? /
//!   str-int-valid?（含误用负例）；
//! - **管线集成**：run/eval 双路径经自举 Reader 端到端。
//!
//! 遵循条款：§9.4.3（正负例成对）、§7.1（集成验证 ≥3）、§2.3-11
//! （先实测禁臆测——全部断言经双实现实跑比对）。

use kerf_driver::bootstrap::{
    lex_source, read_source, selfcheck_call, selfcheck_list, selfcheck_render,
};
use kerf_reader::{lex_source as seed_lex, read_source as seed_read, TokenKind};
use kerf_runtime::RuntimeError;
use kerf_syntax::{Stx, StxDatum, StxLiteral, SymbolTable};
use kerf_vm::{BuiltinFn, Value};

// ---- parity 走查 ----

/// Stx 树等价（datum + Span 递归；符号按名——两实现各自 intern 次序
/// 不保证一致，名字是唯一稳定口径）。
fn stx_equiv(a: &Stx, ta: &SymbolTable, b: &Stx, tb: &SymbolTable) -> Result<(), String> {
    if a.span.start != b.span.start || a.span.end != b.span.end {
        return Err(format!(
            "Span 不一致：({},{}) vs ({},{})",
            a.span.start, a.span.end, b.span.start, b.span.end
        ));
    }
    match (&a.datum, &b.datum) {
        (StxDatum::Symbol(x), StxDatum::Symbol(y)) => {
            if ta.name(*x) != tb.name(*y) {
                return Err(format!("符号名不一致：{} vs {}", ta.name(*x), tb.name(*y)));
            }
            Ok(())
        }
        (StxDatum::Literal(x), StxDatum::Literal(y)) => match (x, y) {
            (StxLiteral::Int(p), StxLiteral::Int(q)) => (p == q)
                .then_some(())
                .ok_or_else(|| format!("int 不一致：{} vs {}", p, q)),
            (StxLiteral::Float(p), StxLiteral::Float(q)) => {
                // 同源 parse（十进制→二进制正确舍入）→ 位一致；inf==inf 成立
                (p == q || (p.is_nan() && q.is_nan()))
                    .then_some(())
                    .ok_or_else(|| format!("float 不一致：{} vs {}", p, q))
            }
            (StxLiteral::Str(p), StxLiteral::Str(q)) => (p == q)
                .then_some(())
                .ok_or_else(|| format!("str 不一致：{:?} vs {:?}", p, q)),
            (StxLiteral::Bool(p), StxLiteral::Bool(q)) => (p == q)
                .then_some(())
                .ok_or_else(|| format!("bool 不一致：{} vs {}", p, q)),
            (StxLiteral::Nil, StxLiteral::Nil) => Ok(()),
            _ => Err(format!("字面量种类不一致：{:?} vs {:?}", x, y)),
        },
        (StxDatum::List(xs), StxDatum::List(ys)) | (StxDatum::Vector(xs), StxDatum::Vector(ys)) => {
            if xs.len() != ys.len() {
                return Err(format!("元素数不一致：{} vs {}", xs.len(), ys.len()));
            }
            for (i, (x, y)) in xs.iter().zip(ys.iter()).enumerate() {
                stx_equiv(x, ta, y, tb).map_err(|e| format!("[{}]: {}", i, e))?;
            }
            Ok(())
        }
        _ => Err(format!(
            "datum 种类不一致：{} vs {}",
            a.datum.kind_name(),
            b.datum.kind_name()
        )),
    }
}

/// 单源 parity 断言：读路径（正例树等价 / 负例消息+Span 等价 / 形态一致）。
fn assert_read_parity(source: &str) {
    let mut ta = SymbolTable::new();
    let mut tb = SymbolTable::new();
    let ra = seed_read(source, 0, &mut ta);
    let rb = read_source(source, 0, &mut tb);
    match (&ra, &rb) {
        (Ok(fa), Ok(fb)) => {
            assert_eq!(
                fa.len(),
                fb.len(),
                "顶层形式数不一致：{}（源 {:?}）",
                source,
                source
            );
            for (i, (x, y)) in fa.iter().zip(fb.iter()).enumerate() {
                stx_equiv(x, &ta, y, &tb).unwrap_or_else(|e| {
                    panic!(
                        "parity 失败（第 {} 个形式）：{}\n源：{:?}",
                        i + 1,
                        e,
                        source
                    )
                });
            }
        }
        (Err(ea), Err(eb)) => {
            assert_eq!(
                ea.message, eb.message,
                "错误消息不一致\n源：{:?}\n种子：{:?}\n自举：{:?}",
                source, ea.message, eb.message
            );
            assert_eq!(
                (ea.span.start, ea.span.end),
                (eb.span.start, eb.span.end),
                "错误 Span 不一致\n源：{:?}",
                source
            );
        }
        _ => panic!(
            "结果形态不一致（一方 Ok 一方 Err）\n源：{:?}\n种子：{:?}\n自举：{:?}",
            source,
            ra.map(|_| "Ok"),
            rb.map(|_| "Ok")
        ),
    }
}

/// 负例 parity 断言：种子确实报错（防语料误收正例）+ 双实现一致。
fn assert_negative_parity(source: &str) {
    let mut t = SymbolTable::new();
    assert!(
        seed_read(source, 0, &mut t).is_err(),
        "负例语料误收（种子读成功）：{:?}",
        source
    );
    assert_read_parity(source);
}

// ---- 正例语料（forms parity）----

/// 语料来源：种子 reader 单元/集成用例全集 + 奇异边界扩展。
#[test]
fn parity_forms_positive() {
    let corpus = [
        "42",
        "(+ 1 2)",
        "(define x 1)",
        "(+ 1 (* 2 3))",
        "(define (f x) (+ x 1)) (f 41)",
        "'x",
        "'(1 2 3)",
        "(quote x)",
        "(quote (a b (c 1 \"s\" true nil)))",
        "'(quote x)",
        "[1 2 3]",
        "lambda",
        "lambdax",
        "true false nil",
        "\"a\\nb\\nc\"",
        "3 3.5 1e3 -5 -5.5 +7",
        "+ - * / <= >= = mod < >",
        "; 行注释\n(+ 1 2) ; 尾注\n#| 块 |# 3",
        "#| 外层 #| 内层 |# 仍在外层 |# 42",
        "...",
        "..",
        "....",
        "...abc",
        "(a . b)",
        "\"héllo wörld\"",
        "(define é 1)",
        "é λ λx _x x' a? b! c$ d% e& f!o",
        "1e-3 1E+3 0.5",
        "-0",
        "9223372036854775807",
        "-9223372036854775808",
        "+9223372036854775807",
        "00009223372036854775807",
        "1e999",
        "",
        "   \n; 仅注释\n",
        "(begin 1)",
        "(define (make-adder n) (lambda (x) (+ x n)))\n(define add5 (make-adder 5))\n(list (add5 10) (if (null? nil) 1 2))",
        "'é",
        "'λx",
    ];
    for src in corpus {
        assert_read_parity(src);
    }
}

// ---- 负例语料（错误消息 + Span parity）----

#[test]
fn parity_reader_negative() {
    let corpus = [
        "(+ 1",
        "(+ 1 2]",
        "[1 2 3)",
        ")",
        "]",
        "123abc",
        "3-4",
        "(+ ~ 1)",
        "\"abc",
        "\"a\\q\"",
        "#| abc",
        "#| a #| b |# c",
        "1e",
        "1e+",
        "9223372036854775808",
        "-9223372036854775809",
        "99999999999999999999",
        "0009223372036854775808",
        "'",
        "' ",
        "(quote",
        ".",
        ". 5",
        "3.",
        "1.2.3",
        "(a b))",
        "(lambda (x) x))",
        "1e3.5",
    ];
    for src in corpus {
        assert_negative_parity(src);
    }
}

// ---- 双错误次序 parity（首错位置契约）----

#[test]
fn parity_dual_error_ordering() {
    // 数字溢出 + 后置词法错误：种子在扫描序内先报溢出——自举经
    // str-int-valid? 前置校验维持同一次序
    let corpus = [
        "(+ 99999999999999999999 ~)",
        "(* 9223372036854775808",
        "(+ 1 99999999999999999999abc)",
        "(+ 99999999999999999999",
        "(+ 99999999999999999999 1]",
    ];
    for src in corpus {
        assert_negative_parity(src);
    }
}

// ---- Unicode / NFC parity ----

#[test]
fn parity_unicode_and_nfc() {
    let corpus = [
        // 组合字符 vs 预组字符：两实现各自 intern 归一化后同名
        "(define e\u{0301} 1) (define \u{00E9} 2)",
        "\u{00E9}",
        "e\u{0301}",
        // 多字节标识符的 Span（字节偏移）
        "(é λ 你好)",
        // 字符串内容的多字节保持
        "\"éλ你\"",
        // NNBSP 等异形空白 = Unicode 空白（trivia）
        "(+ 1\u{00A0}2)",
    ];
    for src in corpus {
        assert_read_parity(src);
    }
}

// ---- Token 流 parity（词法层：种类 + Span）----

#[test]
fn parity_token_stream_kinds_and_spans() {
    let corpus = [
        "(define x 1)",
        "(+ 12 ab)",
        "lambda lambdax if iff",
        "3 3.5 1e3 -5 -5.5 +7",
        "+ - * / <= >= = mod < >",
        "\"a\\nb\nc\"",
        "true false nil",
        "'x",
        "[1 2 3]",
        "... ..",
        "é λ _x",
    ];
    for src in corpus {
        let mut ta = SymbolTable::new();
        let mut tb = SymbolTable::new();
        let seed_toks = seed_lex(src, 0, &mut ta).expect("种子词法失败");
        let boot_toks = lex_source(src, 0, &mut tb).expect("自举词法失败");
        assert_eq!(
            seed_toks.len(),
            boot_toks.len(),
            "Token 数不一致\n源：{:?}",
            src
        );
        for (i, (x, y)) in seed_toks.iter().zip(boot_toks.iter()).enumerate() {
            assert_eq!(
                (x.span.start, x.span.end),
                (y.span.start, y.span.end),
                "Token[{}] Span 不一致\n源：{:?}",
                i,
                src
            );
            token_kind_equiv(&x.kind, &ta, &y.kind, &tb)
                .unwrap_or_else(|e| panic!("Token[{}] 种类不一致：{}\n源：{:?}", i, e, src));
        }
    }
}

/// TokenKind 等价（符号按名）。
fn token_kind_equiv(
    a: &TokenKind,
    ta: &SymbolTable,
    b: &TokenKind,
    tb: &SymbolTable,
) -> Result<(), String> {
    use kerf_reader::Delimiter;
    match (a, b) {
        (TokenKind::Eof, TokenKind::Eof) => Ok(()),
        (TokenKind::IntLiteral(p), TokenKind::IntLiteral(q)) => (p == q)
            .then_some(())
            .ok_or_else(|| format!("int {} vs {}", p, q)),
        (TokenKind::FloatLiteral(p), TokenKind::FloatLiteral(q)) => (p == q)
            .then_some(())
            .ok_or_else(|| format!("float {} vs {}", p, q)),
        (TokenKind::StringLiteral(p), TokenKind::StringLiteral(q)) => (p == q)
            .then_some(())
            .ok_or_else(|| format!("str {:?} vs {:?}", p, q)),
        (TokenKind::BoolLiteral(p), TokenKind::BoolLiteral(q)) => (p == q)
            .then_some(())
            .ok_or_else(|| format!("bool {} vs {}", p, q)),
        (TokenKind::NilLiteral, TokenKind::NilLiteral) => Ok(()),
        (TokenKind::Identifier(p), TokenKind::Identifier(q)) => (ta.name(*p) == tb.name(*q))
            .then_some(())
            .ok_or_else(|| format!("ident {} vs {}", ta.name(*p), tb.name(*q))),
        (TokenKind::Keyword(p), TokenKind::Keyword(q)) => (p == q)
            .then_some(())
            .ok_or_else(|| format!("keyword {:?} vs {:?}", p, q)),
        (TokenKind::Operator(p, s), TokenKind::Operator(q, t)) => {
            if p == q && ta.name(*s) == tb.name(*t) {
                Ok(())
            } else {
                Err(format!(
                    "operator {:?}/{} vs {:?}/{}",
                    p,
                    ta.name(*s),
                    q,
                    tb.name(*t)
                ))
            }
        }
        (TokenKind::QuoteShorthand, TokenKind::QuoteShorthand) => Ok(()),
        (TokenKind::Delimiter(p), TokenKind::Delimiter(q)) => {
            let of = |d: &Delimiter| d.as_char();
            (of(p) == of(q))
                .then_some(())
                .ok_or_else(|| format!("delimiter {} vs {}", of(p), of(q)))
        }
        _ => Err(format!(
            "种类分支不一致：{} vs {}",
            a.kind_name(),
            b.kind_name()
        )),
    }
}

// ---- dump 格式 parity（种子格式循环镜像）----

#[test]
fn parity_dump_tokens_format() {
    // 自举 dump 与「种子 Token + dump_tokens 同一格式循环」逐字节一致
    let sources = ["(define x 1)", "(+ 12 ab) 'q", "\"s\\n\" [1 2] ; c\n"];
    for src in sources {
        let mut ta = SymbolTable::new();
        let seed_toks = seed_lex(src, 0, &mut ta).unwrap();
        let mut expected = String::new();
        for t in &seed_toks {
            let name = match &t.kind {
                TokenKind::Identifier(s) => ta.name(*s).to_string(),
                TokenKind::Keyword(k) => k.as_str().to_string(),
                TokenKind::Operator(o, s) => format!("{:?}({})", o, ta.name(*s)),
                other => format!("{:?}", other),
            };
            expected.push_str(&format!(
                "{:>4}..{:<4} {}\n",
                t.span.start, t.span.end, name
            ));
        }
        let actual = kerf_driver::dump_tokens(src, "t.krf").unwrap();
        assert_eq!(actual, expected, "dump 不一致\n源：{:?}", src);
    }
}

// ---- 嵌套深度边界 ----

#[test]
fn parity_deep_nesting_boundary() {
    // 256 层合法 / 257 层报错（防御性上限——与种子一致）
    let ok_src = format!("{}1{}", "(".repeat(256), ")".repeat(256));
    assert_read_parity(&ok_src);
    let err_src = format!("{}1{}", "(".repeat(257), ")".repeat(257));
    assert_negative_parity(&err_src);
    // 混合括号深度
    let mix_err = format!("{}1]{}", "(".repeat(300), "");
    assert_negative_parity(&mix_err);
}

// ---- 管线集成（端到端）----

#[test]
fn bootstrap_pipeline_run_end_to_end() {
    // fib 经自举 Reader 读 → 展开 → 编译 → VM 执行（生产管线全链）
    let src = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 10)";
    let out = kerf_driver::run_source(src, "t.krf").unwrap();
    assert!(matches!(out.value, Value::Int(55)));
}

#[test]
fn bootstrap_pipeline_read_error_rendered() {
    // 读错误经自举 Reader → DriverError 渲染（阶段 + 文件名 + 摘录）
    let err = kerf_driver::run_source("(+ 1", "bad.krf").unwrap_err();
    assert_eq!(err.stage, kerf_driver::Stage::Read);
    assert!(err.rendered.contains("未闭合"));
    assert!(err.rendered.contains("bad.krf"));
}

#[test]
fn bootstrap_pipeline_eval_path_agrees() {
    // T1 双路径：eval 路径同样经自举 Reader 读——两路径结果一致
    let src = "(define (f x) (+ x 1)) (f 41)";
    let a = kerf_driver::run_source(src, "t.krf").unwrap();
    let b = kerf_driver::eval_source(src, "t.krf").unwrap();
    assert!(a.value.eq_value(&b.value));
}

#[test]
fn bootstrap_dump_stx_end_to_end() {
    let out = kerf_driver::dump_stx("(quote (1 2 3)) 'x", "t.krf").unwrap();
    assert!(out.contains("(quote (1 2 3))"));
    assert!(out.contains("(quote x)"));
}

// ---- 高阶函数直测（kerf 源码实现经自检面驱动；builtin 作 f——跨程序安全）----

#[test]
fn hof_map_with_builtin_function() {
    let inc = BuiltinFn::new("inc", |_, args| {
        let v = args
            .first()
            .and_then(|v| v.as_int())
            .ok_or_else(|| RuntimeError::new("需要 int"))?;
        Ok(Value::Int(v + 1))
    });
    let lst = selfcheck_list(vec![Value::Int(1), Value::Int(2), Value::Int(3)]).unwrap();
    let r = selfcheck_call("map", vec![Value::Builtin(inc), lst]).unwrap();
    assert_eq!(selfcheck_render(&r).unwrap(), "(2 3 4)");
}

#[test]
fn hof_filter_with_builtin_predicate() {
    let is_bool = BuiltinFn::new("is-bool", |_, args| {
        Ok(Value::Bool(matches!(args.first(), Some(Value::Bool(_)))))
    });
    let lst = selfcheck_list(vec![Value::Bool(true), Value::Int(1), Value::Bool(false)]).unwrap();
    let r = selfcheck_call("filter", vec![Value::Builtin(is_bool), lst]).unwrap();
    assert_eq!(selfcheck_render(&r).unwrap(), "(true false)");
}

#[test]
fn hof_foldl_with_builtin_function() {
    let add2 = BuiltinFn::new("add2", |_, args| {
        let a = args.first().and_then(|v| v.as_int()).unwrap_or(0);
        let b = args.get(1).and_then(|v| v.as_int()).unwrap_or(0);
        Ok(Value::Int(a + b))
    });
    let lst = selfcheck_list(vec![Value::Int(1), Value::Int(2), Value::Int(3)]).unwrap();
    let r = selfcheck_call("foldl", vec![Value::Builtin(add2), Value::Int(0), lst]).unwrap();
    assert_eq!(selfcheck_render(&r).unwrap(), "6");
}

#[test]
fn hof_for_each_returns_nil() {
    let nop = BuiltinFn::new("nop", |_, _| Ok(Value::Nil));
    let nop2 = nop.clone();
    let lst = selfcheck_list(vec![Value::Int(1), Value::Int(2)]).unwrap();
    let r = selfcheck_call("for-each", vec![Value::Builtin(nop), lst]).unwrap();
    assert!(matches!(r, Value::Nil));
    // 空表恒等
    let empty = selfcheck_list(vec![]).unwrap();
    let r2 = selfcheck_call("map", vec![Value::Builtin(nop2), empty]).unwrap();
    assert!(matches!(r2, Value::Nil), "map 空表应返回 nil");
}

#[test]
fn hof_map_empty_list_identity() {
    let id = BuiltinFn::new("id", |_, args| {
        Ok(args.first().cloned().unwrap_or(Value::Nil))
    });
    let empty = selfcheck_list(vec![]).unwrap();
    let r = selfcheck_call("map", vec![Value::Builtin(id), empty]).unwrap();
    assert!(matches!(r, Value::Nil));
}

// ---- Reader 原语（含误用负例）----

#[test]
fn reader_primitive_str_pos_chars() {
    // 偏移 + 单字符 + 空串
    let lst = selfcheck_list(vec![]).unwrap(); // 仅为初始化状态
    let _ = lst;
    let r = selfcheck_call("str->pos-chars", vec![strv("aéb")]).unwrap();
    let rendered = selfcheck_render(&r).unwrap();
    // 渲染为 ((0 . a) (1 . é) (3 . b))——偏移按字节（Str 渲染不带引号）
    assert!(rendered.contains("(0 . a)"), "渲染：{}", rendered);
    assert!(rendered.contains("(1 . é)"), "渲染：{}", rendered);
    assert!(rendered.contains("(3 . b)"), "渲染：{}", rendered);
    // 空串 → nil
    let r2 = selfcheck_call("str->pos-chars", vec![strv("")]).unwrap();
    assert!(matches!(r2, Value::Nil));
}

#[test]
fn reader_primitive_str_pos_chars_negative() {
    // 非字符串参数 → 结构化错误（报错>静默）
    let e = selfcheck_call("str->pos-chars", vec![Value::Int(1)]).unwrap_err();
    assert!(e.message.contains("需要 str"), "消息：{}", e.message);
    // 元数错误
    let e2 = selfcheck_call("str->pos-chars", vec![strv("a"), strv("b")]).unwrap_err();
    assert!(
        e2.message.contains("1 个参数") || e2.message.contains("参数"),
        "消息：{}",
        e2.message
    );
}

#[test]
fn reader_primitive_char_predicates() {
    // 谓词原语：正常路径
    let ws = selfcheck_call("char-whitespace?", vec![strv(" ")]).unwrap();
    assert!(matches!(ws, Value::Bool(true)));
    let ws2 = selfcheck_call("char-whitespace?", vec![strv("\u{00A0}")]).unwrap();
    assert!(matches!(ws2, Value::Bool(true)), "NNBSP 是 Unicode 空白");
    let ws3 = selfcheck_call("char-whitespace?", vec![strv("a")]).unwrap();
    assert!(matches!(ws3, Value::Bool(false)));
    let al = selfcheck_call("char-alphabetic?", vec![strv("λ")]).unwrap();
    assert!(matches!(al, Value::Bool(true)), "λ 是字母");
    let al2 = selfcheck_call("char-alphabetic?", vec![strv("1")]).unwrap();
    assert!(matches!(al2, Value::Bool(false)));
}

#[test]
fn reader_primitive_char_predicates_negative() {
    // 多字符 → 显式拒绝；非字符串 → 类型错误
    let e = selfcheck_call("char-whitespace?", vec![strv("ab")]).unwrap_err();
    assert!(e.message.contains("1 字符"), "消息：{}", e.message);
    let e2 = selfcheck_call("char-alphabetic?", vec![Value::Int(1)]).unwrap_err();
    assert!(e2.message.contains("需要 str"), "消息：{}", e2.message);
    let e3 = selfcheck_call("char-whitespace?", vec![Value::Bool(true)]).unwrap_err();
    assert!(e3.message.contains("需要 str"), "消息：{}", e3.message);
}

#[test]
fn reader_primitive_str_int_valid() {
    // i64 域判定 = Rust parse 语义原样（宿主类型边界）
    let cases = [
        ("9223372036854775807", true),
        ("9223372036854775808", false),
        ("-9223372036854775808", true),
        ("-9223372036854775809", false),
        ("+7", true),
        ("000005", true),
        ("", false),
    ];
    for (text, expect) in cases {
        let r = selfcheck_call("str-int-valid?", vec![strv(text)]).unwrap();
        assert!(
            matches!(r, Value::Bool(b) if b == expect),
            "str-int-valid?({:?}) 期望 {}",
            text,
            expect
        );
    }
}

#[test]
fn reader_primitive_str_int_valid_negative() {
    let e = selfcheck_call("str-int-valid?", vec![Value::Int(3)]).unwrap_err();
    assert!(e.message.contains("需要 str"), "消息：{}", e.message);
}

// ---- 辅助：测试本地 Value 构造 ----

/// 字符串值构造（Rc<str>）。
fn strv(s: &str) -> Value {
    Value::Str(std::rc::Rc::from(s))
}

/// StxDatum 种类名（诊断用）。
trait DatumKindName {
    fn kind_name(&self) -> &'static str;
}

impl DatumKindName for StxDatum {
    fn kind_name(&self) -> &'static str {
        match self {
            StxDatum::Symbol(_) => "symbol",
            StxDatum::Literal(_) => "literal",
            StxDatum::List(_) => "list",
            StxDatum::Vector(_) => "vector",
        }
    }
}

trait TokenKindBridge {
    fn kind_name(&self) -> &'static str;
}

impl TokenKindBridge for TokenKind {
    fn kind_name(&self) -> &'static str {
        match self {
            TokenKind::Eof => "eof",
            TokenKind::IntLiteral(_) => "int",
            TokenKind::FloatLiteral(_) => "float",
            TokenKind::StringLiteral(_) => "str",
            TokenKind::BoolLiteral(_) => "bool",
            TokenKind::NilLiteral => "nil",
            TokenKind::Identifier(_) => "identifier",
            TokenKind::Keyword(_) => "keyword",
            TokenKind::Operator(..) => "operator",
            TokenKind::QuoteShorthand => "quote",
            TokenKind::Delimiter(_) => "delimiter",
            TokenKind::MacroInvocation(_) => "macro",
        }
    }
}

// ---- 系统化负例矩阵（§9.4.3 全局正负比 ≥1:3 维持；每个 case 在两实现
// ---- 上锁字节级错误 parity + 种子确实报错断言）----

#[test]
fn parity_negative_battery_brackets() {
    // 未闭合深度扫描 1..30（开括号 + 内容 + EOF）
    for k in 1..=30 {
        assert_negative_parity(&format!("{}1", "(".repeat(k)));
    }
    // 未闭合（带内容变体）
    for src in [
        "(+ 1",
        "(+ 1 2",
        "(define (x",
        "(\"s\"",
        "(quote \"s",
        "[quote 1",
        "(lambda (x",
        "(begin (+ 1",
        "[let [x",
        "(1 (2",
        "((",
        "[[",
        "([",
        "[(",
    ] {
        assert_negative_parity(src);
    }
    // 括号不匹配矩阵（开放 × 关闭全组合中的错配）+ 嵌套错配
    for src in [
        "(1]", "[1)", "((1]]", "[[1))", "([1]", "[(1)", "(+ 1 2]", "[+ 1 2)", "(a b c]", "[a b c)",
        "((a] b)",
    ] {
        assert_negative_parity(src);
    }
    // 多余关闭括号（独立与尾部）
    for src in [
        ")", "]", "))", ")]", ")))", "(1))", "((1)))", "1)", "x]", "(+ 1 2))",
    ] {
        assert_negative_parity(src);
    }
}

#[test]
fn parity_negative_battery_strings() {
    // 非法转义字符（不在 n/t/r/\\//"/0 六集合内）
    for c in [
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "o", "p", "q", "s", "u",
        "v", "w", "x", "y", "z", "A", "Z", "!", "?", "_", "1", "é", "λ", " ",
    ] {
        assert_negative_parity(&format!("\"\\{}\"", c));
    }
    // 带内容的非法转义 + 多字节转义（Span 逐字节口径）+ 裸反斜杠
    for src in [
        "\"ab\\q\"",
        "\"a\\db\"",
        "\"\\é\"",
        "\"x\\λ\"",
        "before \\q after",
    ] {
        assert_negative_parity(src);
    }
    // 反斜杠后 EOF / 未闭合与转义组合
    for src in ["\"abc\\", "\"a\\n\\", "\"\\", "(\"]\\", "'\"\\"] {
        assert_negative_parity(src);
    }
}

#[test]
fn parity_negative_battery_numbers() {
    // 指数缺数字（e/eE/符号后无数字）
    for src in [
        "1e", "1E", "1e+", "1e-", "1E+", "1E-", "1e+x", "12e", "1.5e", "0.5e-", "1.2.3", "3.",
        "1e3.5", "3.5.5", "00.5.5",
    ] {
        assert_negative_parity(src);
    }
    // 贪婪数字拒绝：数字 × 标识符字符矩阵（10 数字 × 5 字符类）
    for d in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] {
        for c in ["a", "_", "é", "λ", "+"] {
            assert_negative_parity(&format!("{}{}", d, c));
        }
    }
    // 贪婪拒绝（带上下文）
    for src in ["123abc", "(+ 123abc 1)", "1x", "-1z", "+9a", "0b1", "1_2"] {
        assert_negative_parity(src);
    }
    // 溢出扫描：9 的串长 19..26 × 无符号/+/-
    for len in 19..=26 {
        let nines = "9".repeat(len);
        assert_negative_parity(&nines);
        assert_negative_parity(&format!("+{}", nines));
        assert_negative_parity(&format!("-{}", nines));
    }
    // 边界外一格（max+1 / min-1）
    for src in [
        "9223372036854775808",
        "-9223372036854775809",
        "+9223372036854775808",
        "09223372036854775808",
        "00092233720368547758080",
    ] {
        assert_negative_parity(src);
    }
}

#[test]
fn parity_negative_battery_illegal_chars() {
    // 非法字符（不在任何 Token 起始集）：独立与上下文
    for ch in ["~", "@", ",", ":", "^", "`", "\\", "|"] {
        assert_negative_parity(ch);
        assert_negative_parity(&format!("(+ {} 1)", ch));
        assert_negative_parity(&format!("x{}", ch));
    }
    // 单点（. 不跟 .）与孤立 #
    for src in [".", ". 5", "5 .", "(+ . 1)", "1 . 2", "#", "#x", "a # b"] {
        assert_negative_parity(src);
    }
}

#[test]
fn parity_negative_battery_dual_errors() {
    // 溢出 + 后置错误（首错 = 溢出——扫描序 parity）
    let big = "9".repeat(22);
    for tail in [" ~)", " ~", "]", ")", " z", " \"s"] {
        assert_negative_parity(&format!("(+ {}{}", big, tail));
    }
    // 前置词法错误 + 后置溢出（首错 = 前置错误）
    assert_negative_parity(&format!("(~ (+ {}))", big));
    assert_negative_parity(&format!("\"unclosed (+ {})", big));
    // 溢出 + 未闭合
    assert_negative_parity(&format!("(define x {}", big));
}

#[test]
fn parity_negative_battery_comments_and_eof() {
    // 块注释未闭合（嵌套深度 1..5）
    for depth in 1..=5 {
        let mut src = String::new();
        for _ in 0..depth {
            src.push_str("#| ");
        }
        assert_negative_parity(&src);
    }
    // 部分闭合的嵌套
    assert_negative_parity("#| 外层 #| 内层 |# 仍在外层");
    assert_negative_parity("#| a #| b |# c #| d");
    // 注释吞噬开括号后未闭合
    assert_negative_parity("(#| a");
    assert_negative_parity("(+ 1 #| 2");
    // 注释后多余关闭
    assert_negative_parity("#| c |# )");
    assert_negative_parity("#|c|# ]");
    // 引号后 EOF
    assert_negative_parity("'");
    assert_negative_parity("'");
    assert_negative_parity("' ");
    assert_negative_parity("(quote ");
    assert_negative_parity("(define x '");
}
