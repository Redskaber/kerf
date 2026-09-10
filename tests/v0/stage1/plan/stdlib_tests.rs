//! 标准库最小集集成测试（tests/v0/stage1/plan/stdlib_tests.rs ↔
//! docs/lang-design/09-stdlib.md §2 清单 + 07 §3.3 阶段门条件 3）。
//!
//! r5（批次 B MUV 22-c）：列表操作/字符串处理/基本 I/O 各 ≥8 函数。
//! 正负消息全部实跑校准（§9.4.3 禁止臆测）；高阶函数（map/filter/foldl）
//! 显式推迟至 B3 Reader kerf 重写批次（用 kerf 源码 preamble 实现——
//! 自举验证命题本体，§12 最优>最小：Rust 抢实现会移除 B3 验证内容）。

#[path = "../../../common/mod.rs"]
mod common;

use kerf_driver::{run_source, Stage};

/// 通用断言：src 失败于 Run 阶段（E0004），消息含子串。
fn expect_run_err(src: &str, msg: &str) {
    let err = match run_source(src, "stdlib-neg.krf") {
        Ok(_) => panic!("期望报错，实际 Ok：{}", src),
        Err(e) => e,
    };
    assert_eq!(err.stage, Stage::Run, "阶段不符：{}\n{}", src, err.rendered);
    assert!(
        err.rendered.contains(msg),
        "消息缺少「{}」：{}\n完整错误：{}",
        msg,
        src,
        err.rendered
    );
}

// ---------------------------------------------------------------------------
// 列表操作（8 函数：length/append/reverse/list-ref/list-tail/member/assoc/last-pair）
// ---------------------------------------------------------------------------

/// 列表正向语义（正例 ≥8 case）。
#[test]
fn list_ops_positive_semantics() {
    assert_eq!(common::run_rendered("(length '())"), "0");
    assert_eq!(common::run_rendered("(length '(1 2 3))"), "3");
    assert_eq!(common::run_rendered("(append '(1 2) '(3 4))"), "(1 2 3 4)");
    assert_eq!(common::run_rendered("(append)"), "nil");
    assert_eq!(common::run_rendered("(append '(1) 5)"), "(1 . 5)"); // 末参原样（improper 允许）
    assert_eq!(common::run_rendered("(reverse '(1 2 3))"), "(3 2 1)");
    assert_eq!(common::run_rendered("(reverse '())"), "nil"); // 空表 = nil 值形态（与 '() 一致）
    assert_eq!(common::run_rendered("(list-ref '(a b c) 0)"), "a");
    assert_eq!(common::run_rendered("(list-ref '(a b c) 2)"), "c");
    assert_eq!(common::run_rendered("(list-tail '(a b c d) 2)"), "(c d)");
    assert_eq!(common::run_rendered("(list-tail '(a b) 0)"), "(a b)");
    assert_eq!(common::run_rendered("(member 2 '(1 2 3))"), "(2 3)");
    assert_eq!(common::run_rendered("(member 9 '(1 2 3))"), "false");
    assert_eq!(common::run_rendered("(assoc 'b '((a 1) (b 2)))"), "(b 2)");
    assert_eq!(common::run_rendered("(assoc 'z '((a 1)))"), "false");
    assert_eq!(common::run_rendered("(last-pair '(1 2 3))"), "(3)");
}

/// 列表负向语义（元数/类型/索引边界，≥12 case）。
#[test]
fn list_ops_negative_semantics() {
    expect_run_err("(length 5)", "length 需要 list");
    expect_run_err("(length)", "length 需要 1 个参数");
    expect_run_err("(length (cons 1 2))", "length 需要 list"); // improper（运行时构造——reader 无点对语法）
    expect_run_err("(reverse 'a)", "reverse 需要 list，实际 symbol");
    expect_run_err("(reverse 42)", "reverse 需要 list，实际 int");
    expect_run_err("(list-ref '(a b) 5)", "索引 5 超出列表范围");
    expect_run_err("(list-ref '(a b) -1)", "索引需要非负整数，实际 -1");
    expect_run_err("(list-ref '(a b) 'x)", "list-ref 索引需要 int，实际 symbol");
    expect_run_err("(list-tail '(a) 3)", "索引 3 超出列表范围");
    expect_run_err("(member 'a 5)", "member 第二参需要 list，实际 int");
    expect_run_err("(assoc 'a '(1 2))", "assoc 表项需要 pair，实际 int");
    expect_run_err("(last-pair nil)", "last-pair 需要非空 list");
    expect_run_err(
        "(append '(1 2) 5 '(3))",
        "append 第 2 参需要 list，实际 int",
    );
    expect_run_err("(append 'a 'b)", "append 第 1 参需要 list，实际 symbol");
}

// ---------------------------------------------------------------------------
// 字符串处理（10 函数：str-length/substring/index-of/contains?/prefix?/
// suffix?/upcase/downcase/string->symbol/symbol->string + str-append 基线）
// ---------------------------------------------------------------------------

/// 字符串正向语义（Unicode 语义：字符索引非字节，正例 ≥10 case）。
#[test]
fn string_ops_positive_semantics() {
    assert_eq!(common::run_rendered("(str-length \"\")"), "0");
    assert_eq!(common::run_rendered("(str-length \"héllo\")"), "5"); // Unicode 字符数
    assert_eq!(common::run_rendered("(str-substring \"héllo\" 1 3)"), "él");
    assert_eq!(common::run_rendered("(str-substring \"abc\" 0 3)"), "abc");
    assert_eq!(
        common::run_rendered("(str-index-of \"héllo wörld\" \"w\")"),
        "6"
    ); // 字符位
    assert_eq!(common::run_rendered("(str-index-of \"abc\" \"z\")"), "-1");
    assert_eq!(
        common::run_rendered("(str-contains? \"abc\" \"bc\")"),
        "true"
    );
    assert_eq!(
        common::run_rendered("(str-contains? \"abc\" \"z\")"),
        "false"
    );
    assert_eq!(common::run_rendered("(str-prefix? \"abc\" \"ab\")"), "true");
    assert_eq!(
        common::run_rendered("(str-prefix? \"abc\" \"bc\")"),
        "false"
    );
    assert_eq!(common::run_rendered("(str-suffix? \"abc\" \"bc\")"), "true");
    assert_eq!(common::run_rendered("(str-upcase \"aé\")"), "AÉ");
    assert_eq!(common::run_rendered("(str-downcase \"AÉ\")"), "aé");
    assert_eq!(common::run_rendered("(str-append \"a\" \"b\")"), "ab");
    // TD-002 联动互转（符号值 ↔ 字符串）
    assert_eq!(common::run_rendered("(string->symbol \"foo\")"), "foo");
    assert_eq!(common::run_rendered("(symbol->string 'foo)"), "foo");
    assert_eq!(
        common::run_rendered("(eq? (string->symbol \"a\") 'a)"),
        "true"
    );
}

/// 字符串负向语义（类型/索引边界，≥10 case）。
#[test]
fn string_ops_negative_semantics() {
    expect_run_err("(str-length 'a)", "str-length 需要 str，实际 symbol");
    expect_run_err("(str-length 5)", "str-length 需要 str，实际 int");
    expect_run_err("(str-substring \"abc\" 0 9)", "索引越界：0..9（长度 3）");
    expect_run_err("(str-substring \"abc\" 2 1)", "索引越界：2..1（长度 3）");
    expect_run_err("(str-substring \"abc\" -1 2)", "索引越界：-1..2");
    expect_run_err(
        "(str-substring \"abc\" 'a 'b)",
        "str-substring 索引需要 int",
    );
    expect_run_err("(str-index-of \"a\" 1)", "两参都需要 str，实际 str 与 int");
    expect_run_err("(str-contains? 1 \"a\")", "两参都需要 str，实际 int 与 str");
    expect_run_err("(str-prefix? 1 2)", "两参都需要 str");
    expect_run_err("(string->symbol 5)", "string->symbol 需要 str，实际 int");
    expect_run_err(
        "(symbol->string \"s\")",
        "symbol->string 需要 symbol，实际 str",
    );
}

// ---------------------------------------------------------------------------
// 基本 I/O（6 函数：newline/write-string/read-int/read-num/error/assert-eq?）
// ---------------------------------------------------------------------------

/// I/O 正向语义（非交互函数；read-int/read-num 的 EOF 与解析行为在
/// 双路径测试内经 stdin 重定向锚定不可行——CLI 层验证，正例 ≥6 case）。
#[test]
fn io_ops_positive_semantics() {
    assert_eq!(common::run_rendered("(newline)"), "nil");
    assert_eq!(common::run_rendered("(write-string \"x\")"), "nil");
    assert_eq!(common::run_rendered("(assert-eq? 'a 'a)"), "true");
    assert_eq!(common::run_rendered("(assert-eq? 1 1)"), "true");
    assert_eq!(common::run_rendered("(assert-eq? \"s\" \"s\")"), "true");
    // error 消息透传（用户结构化报错）
    let err = run_source("(error \"custom msg\")", "io.krf").unwrap_err();
    assert_eq!(err.stage, Stage::Run);
    assert!(err.rendered.contains("custom msg"));
}

/// I/O 负向语义（元数/类型，≥6 case）。
#[test]
fn io_ops_negative_semantics() {
    expect_run_err("(newline 1)", "newline 需要 0 个参数");
    expect_run_err("(write-string 5)", "write-string 需要 str，实际 int");
    expect_run_err("(read-int 1)", "read-int 需要 0 个参数");
    expect_run_err("(read-num 1)", "read-num 需要 0 个参数");
    expect_run_err("(error)", "error 需要 ≥1 个参数");
    expect_run_err("(assert-eq? 1 2)", "assert-eq? 断言失败：1 ≠ 2");
    expect_run_err("(assert-eq? 1)", "assert-eq? 需要 2 个参数");
}

// ---------------------------------------------------------------------------
// 双路径一致（T1：VM 与 eval 渲染等价——高密度经 dual_path_agrees）
// ---------------------------------------------------------------------------

/// 标准库双路径一致（纯函数子集——I/O 副作用函数除外）。
#[test]
fn stdlib_dual_path_agreement() {
    assert!(common::dual_path_agrees("(length '(1 2 3))"));
    assert!(common::dual_path_agrees("(append '(1 2) '(3))"));
    assert!(common::dual_path_agrees("(reverse '(1 2 3))"));
    assert!(common::dual_path_agrees("(list-ref '(a b c) 1)"));
    assert!(common::dual_path_agrees("(list-tail '(a b c) 1)"));
    assert!(common::dual_path_agrees("(member 2 '(1 2 3))"));
    assert!(common::dual_path_agrees("(assoc 'a '((a 1)))"));
    assert!(common::dual_path_agrees("(last-pair '(1 2))"));
    assert!(common::dual_path_agrees("(str-length \"héllo\")"));
    assert!(common::dual_path_agrees("(str-substring \"héllo\" 1 3)"));
    assert!(common::dual_path_agrees("(string->symbol \"x\")"));
    assert!(common::dual_path_agrees("(symbol->string 'x)"));
    assert!(common::dual_path_agrees("(str-upcase \"aé\")"));
    assert!(common::dual_path_agrees("(str-index-of \"abc\" \"b\")"));
}

// ---------------------------------------------------------------------------
// 负例矩阵扩张（§9.4.3 全局 ≥1:3 维持：24 新函数 × 元数/类型/边界）
// 消息全部实跑校准（2026-09-10 探针批量）
// ---------------------------------------------------------------------------

/// 元数矩阵（24 函数逐一：0 参/少参/多参形态按各自契约）。
#[test]
fn stdlib_negative_arity_matrix() {
    let cases: &[(&str, &str)] = &[
        ("(length)", "length 需要 1 个参数，实际 0"),
        ("(length 1 2)", "length 需要 1 个参数，实际 2"),
        ("(reverse)", "reverse 需要 1 个参数，实际 0"),
        ("(reverse 1 2)", "reverse 需要 1 个参数，实际 2"),
        ("(list-ref)", "list-ref 需要 2 个参数，实际 0"),
        ("(list-tail)", "list-tail 需要 2 个参数，实际 0"),
        ("(member)", "member 需要 2 个参数，实际 0"),
        ("(member 1)", "member 需要 2 个参数，实际 1"),
        ("(assoc)", "assoc 需要 2 个参数，实际 0"),
        ("(assoc 'a)", "assoc 需要 2 个参数，实际 1"),
        ("(last-pair)", "last-pair 需要 1 个参数，实际 0"),
        ("(str-length)", "str-length 需要 1 个参数，实际 0"),
        ("(str-substring \"a\")", "str-substring 需要 3 个参数"),
        ("(str-index-of \"a\")", "str-index-of 需要 2 个参数，实际 1"),
        (
            "(str-contains? \"a\")",
            "str-contains? 需要 2 个参数，实际 1",
        ),
        ("(str-prefix? \"a\")", "str-prefix? 需要 2 个参数，实际 1"),
        ("(str-upcase)", "str-upcase 需要 1 个参数，实际 0"),
        ("(string->symbol)", "string->symbol 需要 1 个参数，实际 0"),
        ("(symbol->string)", "symbol->string 需要 1 个参数，实际 0"),
        ("(newline 1 2)", "newline 需要 0 个参数，实际 2"),
        ("(write-string)", "write-string 需要 1 个参数，实际 0"),
        ("(read-int 1 2)", "read-int 需要 0 个参数，实际 2"),
        ("(read-num 'a)", "read-num 需要 0 个参数，实际 1"),
        ("(error)", "error 需要 ≥1 个参数"),
        ("(assert-eq?)", "assert-eq? 需要 2 个参数，实际 0"),
    ];
    for (src, msg) in cases {
        expect_run_err(src, msg);
    }
}

/// 类型矩阵（参数类型误用——含两参类型组合报告）。
#[test]
fn stdlib_negative_type_matrix() {
    let cases: &[(&str, &str)] = &[
        (
            "(length \"s\")",
            "length 需要 list（nil 终结的序对链），实际 str",
        ),
        ("(append 1 2)", "append 第 1 参需要 list，实际 int"),
        ("(append \"a\" '())", "append 第 1 参需要 list，实际 str"),
        ("(reverse \"s\")", "reverse 需要 list，实际 str"),
        ("(list-tail 5 0)", "list-tail 需要 list，实际 int"),
        ("(member 2 (cons 1 2))", "member 第二参需要 list，实际 int"),
        ("(assoc 'a 5)", "assoc 第二参需要 list，实际 int"),
        ("(assoc 'a '(1))", "assoc 表项需要 pair，实际 int"),
        ("(last-pair 5)", "last-pair 需要 list，实际 int"),
        ("(last-pair (cons 1 2))", "last-pair 需要 list，实际 int"),
        ("(str-length true)", "str-length 需要 str，实际 bool"),
        ("(str-substring 1 0 1)", "str-substring 需要 str，实际 int"),
        (
            "(str-substring \"abc\" 'a 'b)",
            "str-substring 索引需要 int，实际 symbol",
        ),
        (
            "(str-index-of 1 2)",
            "str-index-of 两参都需要 str，实际 int 与 int",
        ),
        (
            "(str-contains? 'a 'b)",
            "str-contains? 两参都需要 str，实际 symbol 与 symbol",
        ),
        (
            "(str-prefix? nil nil)",
            "str-prefix? 两参都需要 str，实际 nil 与 nil",
        ),
        ("(str-upcase 'a)", "str-upcase 需要 str，实际 symbol"),
        (
            "(string->symbol 'a)",
            "string->symbol 需要 str，实际 symbol",
        ),
        ("(symbol->string 5)", "symbol->string 需要 symbol，实际 int"),
        ("(write-string 'a)", "write-string 需要 str，实际 symbol"),
        ("(str-substring \"abc\" 2 1)", "索引越界：2..1（长度 3）"),
    ];
    for (src, msg) in cases {
        expect_run_err(src, msg);
    }
}

/// 边界矩阵（索引/空结构/improper 边界 + 结构化报错内容）。
#[test]
fn stdlib_negative_boundary_matrix() {
    let cases: &[(&str, &str)] = &[
        ("(list-ref '(a) 1)", "list-ref 索引 1 超出列表范围"),
        ("(list-ref 'a 0)", "list-ref 索引 0 超出列表范围"),
        ("(list-tail '(a) 3)", "list-tail 索引 3 超出列表范围"),
        ("(list-tail '(a b) 5)", "list-tail 索引 5 超出列表范围"),
        ("(last-pair nil)", "last-pair 需要非空 list"),
        ("(error 'sym)", "symbol"), // 非字符串部件以类型名渲染（结构化消息部件语义）
        ("(assert-eq? 'a 1)", "assert-eq? 断言失败：a ≠ 1"),
        ("(assert-eq? \"a\" \"b\")", "assert-eq? 断言失败：a ≠ b"),
        ("(member 9 (cons 1 2))", "member 第二参需要 list，实际 int"),
    ];
    for (src, msg) in cases {
        expect_run_err(src, msg);
    }
}

/// Racket 语义边界注记（实测确认的正例——负例矩阵的反向归档）：
/// - `(append 1)` → 1（单参恒等——末参原样，Racket 同）；
/// - `(member 1 (cons 1 2))` → (1 . 2)（improper 首元素命中即返回）。
#[test]
fn stdlib_racket_semantics_boundary_notes() {
    assert_eq!(common::run_rendered("(append 1)"), "1");
    assert_eq!(common::run_rendered("(member 1 (cons 1 2))"), "(1 . 2)");
    // list-tail k=0 恒等（list 输入）
    assert_eq!(common::run_rendered("(list-tail '(a b) 0)"), "(a b)");
    assert_eq!(common::run_rendered("(list-tail '() 0)"), "nil");
}

// ---------------------------------------------------------------------------
// 类型矩阵扩张（§9.4.3 全局 ≥1:3 维持——类型名消息模板经多样本实测：
// str 函数 × 非 str 输入 / list 函数 × 非 list 输入全类型扫描）
// ---------------------------------------------------------------------------

/// str 函数第一参类型全扫描（消息模板「X 需要 str，实际 TYPE」实测稳定）。
#[test]
fn stdlib_negative_str_type_scan() {
    // 非 str 类型代表值：int / float / bool / nil / symbol / pair
    let type_repr: &[(&str, &str)] = &[
        ("5", "int"),
        ("1.5", "float"),
        ("true", "bool"),
        ("nil", "nil"),
        ("'sym", "symbol"),
        ("'(1)", "pair"),
    ];
    let single_arg_fns: &[&str] = &[
        "str-length",
        "str-upcase",
        "str-downcase",
        "string->symbol",
        "write-string",
    ];
    for fn_name in single_arg_fns {
        for (val, ty) in type_repr {
            let src = format!("({} {})", fn_name, val);
            let msg = format!("{} 需要 str，实际 {}", fn_name, ty);
            expect_run_err(&src, &msg);
        }
    }
    // symbol->string 反向扫描（需要 symbol，实际非 symbol）
    for (val, ty) in type_repr {
        if *ty != "symbol" {
            let src = format!("(symbol->string {})", val);
            let msg = format!("symbol->string 需要 symbol，实际 {}", ty);
            expect_run_err(&src, &msg);
        }
    }
}

/// list 函数输入类型全扫描（消息模板「X 需要 list…，实际 TYPE」实测稳定）。
#[test]
fn stdlib_negative_list_type_scan() {
    let type_repr: &[(&str, &str)] = &[
        ("5", "int"),
        ("1.5", "float"),
        ("\"s\"", "str"),
        ("true", "bool"),
        ("'sym", "symbol"),
    ];
    // length/reverse/last-pair：单参 list 契约
    for fn_name in &["length", "reverse", "last-pair"] {
        for (val, ty) in type_repr {
            let src = format!("({} {})", fn_name, val);
            let prefix = if *fn_name == "length" {
                "list（nil 终结的序对链）"
            } else {
                "list"
            };
            let msg = format!("{} 需要 {}，实际 {}", fn_name, prefix, ty);
            expect_run_err(&src, &msg);
        }
    }
    // list-tail：第二参 list（k=0 也拒绝非 list——每步形态校验）
    for (val, ty) in type_repr {
        let src = format!("(list-tail {} 0)", val);
        let msg = format!("list-tail 需要 list，实际 {}", ty);
        expect_run_err(&src, &msg);
    }
}

/// assert-eq? 堆值引用语义边界（eq? 序对按引用——两同构表不等）+
/// 数值塔不隐式（Int ≠ Float——eq? 无数值塔）。
#[test]
fn stdlib_eq_semantics_boundaries() {
    // eq? 堆值按引用：同构不同对象 → 断言失败（结构等价用 equal?——未实现，注记）
    expect_run_err("(assert-eq? (list 1) (list 1))", "断言失败");
    expect_run_err("(assert-eq? '(1) (list 1))", "断言失败");
    // eq? 无数值塔：1 ≠ 1.0
    expect_run_err("(assert-eq? 1 1.0)", "断言失败：1 ≠ 1.0");
    // 字符串按内容（D2 修复语义）——正例
    assert_eq!(common::run_rendered("(assert-eq? \"a\" \"a\")"), "true");
    // 符号按名（TD-002）——正例
    assert_eq!(
        common::run_rendered("(assert-eq? (string->symbol \"a\") 'a)"),
        "true"
    );
}

/// str 双参函数第二参类型扫描（第一参合法 str，第二参逐类型误用——
/// 「两参都需要 str，实际 str 与 TYPE」模板实测稳定）。
#[test]
fn stdlib_negative_str_second_arg_scan() {
    let bad: &[(&str, &str)] = &[
        ("5", "int"),
        ("1.5", "float"),
        ("true", "bool"),
        ("nil", "nil"),
        ("'sym", "symbol"),
    ];
    for fn_name in &[
        "str-index-of",
        "str-contains?",
        "str-prefix?",
        "str-suffix?",
    ] {
        for (val, ty) in bad {
            let src = format!("({} \"a\" {})", fn_name, val);
            let msg = format!("{} 两参都需要 str，实际 str 与 {}", fn_name, ty);
            expect_run_err(&src, &msg);
        }
    }
    // str-append 两参契约（第二参类型错）
    for (val, _ty) in bad {
        let src = format!("(str-append \"a\" {})", val);
        expect_run_err(&src, "str-append 需要 2 个字符串");
    }
    // member/assoc 第一参任意（eq? 万物可比）——第二参已扫描；
    // 补 append 中间参类型（非末参位置——末参原样允许 improper）
    expect_run_err("(append '() 5 '())", "append 第 2 参需要 list，实际 int");
    expect_run_err(
        "(append '() '() 5 '())",
        "append 第 3 参需要 list，实际 int",
    );
    expect_run_err(
        "(append '() \"s\" '())",
        "append 第 2 参需要 list，实际 str",
    );
}
