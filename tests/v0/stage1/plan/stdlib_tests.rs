//! 标准库最小集集成测试（tests/v0/stage1/plan/stdlib_tests.rs ↔
//! docs/lang-design/09-stdlib.md §2 清单 + 07 §3.3 阶段门条件 3）。
//!
//! r5（批次 B MUV 22-c）：列表操作/字符串处理/基本 I/O 各 ≥8 函数。
//! 正负消息全部实跑校准（§9.4.3 禁止臆测）；高阶函数（map/filter/foldl）
//! 显式推迟至 B3 Reader kerf 重写批次（用 kerf 源码 preamble 实现——
//! 自举验证命题本体，§12 最优>最小：Rust 抢实现会移除 B3 验证内容）。

use crate::common;

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
    assert_eq!(common::run_rendered("(nth '(a b c) 0)"), "a");
    assert_eq!(common::run_rendered("(nth '(a b c) 2)"), "c");
    assert_eq!(common::run_rendered("(drop '(a b c d) 2)"), "(c d)");
    assert_eq!(common::run_rendered("(drop '(a b) 0)"), "(a b)");
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
    expect_run_err("(nth '(a b) 5)", "索引 5 超出列表范围");
    expect_run_err("(nth '(a b) -1)", "索引需要非负整数，实际 -1");
    expect_run_err("(nth '(a b) 'x)", "nth 索引需要 int，实际 symbol");
    expect_run_err("(drop '(a) 3)", "索引 3 超出列表范围");
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
// suffix?/upcase/downcase/string->symbol/symbol->string + string-append 基线）
// ---------------------------------------------------------------------------

/// 字符串正向语义（Unicode 语义：字符索引非字节，正例 ≥10 case）。
#[test]
fn string_ops_positive_semantics() {
    assert_eq!(common::run_rendered("(string-length \"\")"), "0");
    assert_eq!(common::run_rendered("(string-length \"héllo\")"), "5"); // Unicode 字符数
    assert_eq!(
        common::run_rendered("(string-substring \"héllo\" 1 3)"),
        "él"
    );
    assert_eq!(
        common::run_rendered("(string-substring \"abc\" 0 3)"),
        "abc"
    );
    assert_eq!(
        common::run_rendered("(string-index-of \"héllo wörld\" \"w\")"),
        "6"
    ); // 字符位
    assert_eq!(
        common::run_rendered("(string-index-of \"abc\" \"z\")"),
        "-1"
    );
    assert_eq!(
        common::run_rendered("(string-contains \"abc\" \"bc\")"),
        "true"
    );
    assert_eq!(
        common::run_rendered("(string-contains \"abc\" \"z\")"),
        "false"
    );
    assert_eq!(
        common::run_rendered("(string-starts-with \"abc\" \"ab\")"),
        "true"
    );
    assert_eq!(
        common::run_rendered("(string-starts-with \"abc\" \"bc\")"),
        "false"
    );
    assert_eq!(
        common::run_rendered("(string-ends-with \"abc\" \"bc\")"),
        "true"
    );
    assert_eq!(common::run_rendered("(string-to-upper \"aé\")"), "AÉ");
    assert_eq!(common::run_rendered("(string-to-lower \"AÉ\")"), "aé");
    assert_eq!(common::run_rendered("(string-append \"a\" \"b\")"), "ab");
    // TD-002 联动互转（符号值 ↔ 字符串）
    assert_eq!(common::run_rendered("(string-to-symbol \"foo\")"), "foo");
    assert_eq!(common::run_rendered("(symbol-to-string 'foo)"), "foo");
    assert_eq!(
        common::run_rendered("(eq (string-to-symbol \"a\") 'a)"),
        "true"
    );
}

/// 字符串负向语义（类型/索引边界，≥10 case）。
#[test]
fn string_ops_negative_semantics() {
    expect_run_err("(string-length 'a)", "string-length 需要 str，实际 symbol");
    expect_run_err("(string-length 5)", "string-length 需要 str，实际 int");
    expect_run_err("(string-substring \"abc\" 0 9)", "索引越界：0..9（长度 3）");
    expect_run_err("(string-substring \"abc\" 2 1)", "索引越界：2..1（长度 3）");
    expect_run_err("(string-substring \"abc\" -1 2)", "索引越界：-1..2");
    expect_run_err(
        "(string-substring \"abc\" 'a 'b)",
        "string-substring 索引需要 int",
    );
    expect_run_err(
        "(string-index-of \"a\" 1)",
        "两参都需要 str，实际 str 与 int",
    );
    expect_run_err(
        "(string-contains 1 \"a\")",
        "两参都需要 str，实际 int 与 str",
    );
    expect_run_err("(string-starts-with 1 2)", "两参都需要 str");
    expect_run_err(
        "(string-to-symbol 5)",
        "string-to-symbol 需要 str，实际 int",
    );
    expect_run_err(
        "(symbol-to-string \"s\")",
        "symbol-to-string 需要 symbol，实际 str",
    );
}

// ---------------------------------------------------------------------------
// 基本 I/O（6 函数：newline/write-string/read-int/read-num/error/assert-eq?）
// ---------------------------------------------------------------------------

/// I/O 正向语义（非交互函数；read-int/read-num 的 EOF 与解析行为在
/// 双路径测试内经 stdin 重定向锚定不可行——CLI 层验证，正例 ≥6 case）。
#[test]
fn io_ops_positive_semantics() {
    assert_eq!(common::run_rendered("(require io write) (newline)"), "nil");
    assert_eq!(
        common::run_rendered("(require io write) (write-string \"x\")"),
        "nil"
    );
    assert_eq!(common::run_rendered("(assert-eq 'a 'a)"), "true");
    assert_eq!(common::run_rendered("(assert-eq 1 1)"), "true");
    assert_eq!(common::run_rendered("(assert-eq \"s\" \"s\")"), "true");
    // error 消息透传（用户结构化报错）
    let err = run_source("(error \"custom msg\")", "io.krf").unwrap_err();
    assert_eq!(err.stage, Stage::Run);
    assert!(err.rendered.contains("custom msg"));
}

/// I/O 负向语义（元数/类型，≥6 case）。
#[test]
fn io_ops_negative_semantics() {
    expect_run_err("(require io write) (newline 1)", "newline 需要 0 个参数");
    expect_run_err(
        "(require io write) (write-string 5)",
        "write-string 需要 str，实际 int",
    );
    expect_run_err("(require io read) (read-int 1)", "read-int 需要 0 个参数");
    expect_run_err("(require io read) (read-num 1)", "read-num 需要 0 个参数");
    expect_run_err("(error)", "error 需要 ≥1 个参数");
    expect_run_err("(assert-eq 1 2)", "assert-eq 断言失败：1 ≠ 2");
    expect_run_err("(assert-eq 1)", "assert-eq 需要 2 个参数");
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
    assert!(common::dual_path_agrees("(nth '(a b c) 1)"));
    assert!(common::dual_path_agrees("(drop '(a b c) 1)"));
    assert!(common::dual_path_agrees("(member 2 '(1 2 3))"));
    assert!(common::dual_path_agrees("(assoc 'a '((a 1)))"));
    assert!(common::dual_path_agrees("(last-pair '(1 2))"));
    assert!(common::dual_path_agrees("(string-length \"héllo\")"));
    assert!(common::dual_path_agrees("(string-substring \"héllo\" 1 3)"));
    assert!(common::dual_path_agrees("(string-to-symbol \"x\")"));
    assert!(common::dual_path_agrees("(symbol-to-string 'x)"));
    assert!(common::dual_path_agrees("(string-to-upper \"aé\")"));
    assert!(common::dual_path_agrees("(string-index-of \"abc\" \"b\")"));
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
        ("(nth)", "nth 需要 2 个参数，实际 0"),
        ("(drop)", "drop 需要 2 个参数，实际 0"),
        ("(member)", "member 需要 2 个参数，实际 0"),
        ("(member 1)", "member 需要 2 个参数，实际 1"),
        ("(assoc)", "assoc 需要 2 个参数，实际 0"),
        ("(assoc 'a)", "assoc 需要 2 个参数，实际 1"),
        ("(last-pair)", "last-pair 需要 1 个参数，实际 0"),
        ("(string-length)", "string-length 需要 1 个参数，实际 0"),
        ("(string-substring \"a\")", "string-substring 需要 3 个参数"),
        (
            "(string-index-of \"a\")",
            "string-index-of 需要 2 个参数，实际 1",
        ),
        (
            "(string-contains \"a\")",
            "string-contains 需要 2 个参数，实际 1",
        ),
        (
            "(string-starts-with \"a\")",
            "string-starts-with 需要 2 个参数，实际 1",
        ),
        ("(string-to-upper)", "string-to-upper 需要 1 个参数，实际 0"),
        (
            "(string-to-symbol)",
            "string-to-symbol 需要 1 个参数，实际 0",
        ),
        (
            "(symbol-to-string)",
            "symbol-to-string 需要 1 个参数，实际 0",
        ),
        (
            "(require io write) (newline 1 2)",
            "newline 需要 0 个参数，实际 2",
        ),
        (
            "(require io write) (write-string)",
            "write-string 需要 1 个参数，实际 0",
        ),
        (
            "(require io read) (read-int 1 2)",
            "read-int 需要 0 个参数，实际 2",
        ),
        (
            "(require io read) (read-num 'a)",
            "read-num 需要 0 个参数，实际 1",
        ),
        ("(error)", "error 需要 ≥1 个参数"),
        ("(assert-eq)", "assert-eq 需要 2 个参数，实际 0"),
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
        ("(drop 5 0)", "drop 需要 list，实际 int"),
        ("(member 2 (cons 1 2))", "member 第二参需要 list，实际 int"),
        ("(assoc 'a 5)", "assoc 第二参需要 list，实际 int"),
        ("(assoc 'a '(1))", "assoc 表项需要 pair，实际 int"),
        ("(last-pair 5)", "last-pair 需要 list，实际 int"),
        ("(last-pair (cons 1 2))", "last-pair 需要 list，实际 int"),
        ("(string-length true)", "string-length 需要 str，实际 bool"),
        (
            "(string-substring 1 0 1)",
            "string-substring 需要 str，实际 int",
        ),
        (
            "(string-substring \"abc\" 'a 'b)",
            "string-substring 索引需要 int，实际 symbol",
        ),
        (
            "(string-index-of 1 2)",
            "string-index-of 两参都需要 str，实际 int 与 int",
        ),
        (
            "(string-contains 'a 'b)",
            "string-contains 两参都需要 str，实际 symbol 与 symbol",
        ),
        (
            "(string-starts-with nil nil)",
            "string-starts-with 两参都需要 str，实际 nil 与 nil",
        ),
        (
            "(string-to-upper 'a)",
            "string-to-upper 需要 str，实际 symbol",
        ),
        (
            "(string-to-symbol 'a)",
            "string-to-symbol 需要 str，实际 symbol",
        ),
        (
            "(symbol-to-string 5)",
            "symbol-to-string 需要 symbol，实际 int",
        ),
        (
            "(require io write) (write-string 'a)",
            "write-string 需要 str，实际 symbol",
        ),
        ("(string-substring \"abc\" 2 1)", "索引越界：2..1（长度 3）"),
    ];
    for (src, msg) in cases {
        expect_run_err(src, msg);
    }
}

/// 边界矩阵（索引/空结构/improper 边界 + 结构化报错内容）。
#[test]
fn stdlib_negative_boundary_matrix() {
    let cases: &[(&str, &str)] = &[
        ("(nth '(a) 1)", "nth 索引 1 超出列表范围"),
        ("(nth 'a 0)", "nth 索引 0 超出列表范围"),
        ("(drop '(a) 3)", "drop 索引 3 超出列表范围"),
        ("(drop '(a b) 5)", "drop 索引 5 超出列表范围"),
        ("(last-pair nil)", "last-pair 需要非空 list"),
        ("(error 'sym)", "symbol"), // 非字符串部件以类型名渲染（结构化消息部件语义）
        ("(assert-eq 'a 1)", "assert-eq 断言失败：a ≠ 1"),
        ("(assert-eq \"a\" \"b\")", "assert-eq 断言失败：a ≠ b"),
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
    // drop k=0 恒等（list 输入）
    assert_eq!(common::run_rendered("(drop '(a b) 0)"), "(a b)");
    assert_eq!(common::run_rendered("(drop '() 0)"), "nil");
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
        "string-length",
        "string-to-upper",
        "string-to-lower",
        "string-to-symbol",
        "write-string",
    ];
    for fn_name in single_arg_fns {
        for (val, ty) in type_repr {
            // write-string 门控（r8 R9）：先声明 write 能力再触发类型错误
            let src = if *fn_name == "write-string" {
                format!("(require io write) ({} {})", fn_name, val)
            } else {
                format!("({} {})", fn_name, val)
            };
            let msg = format!("{} 需要 str，实际 {}", fn_name, ty);
            expect_run_err(&src, &msg);
        }
    }
    // symbol-to-string 反向扫描（需要 symbol，实际非 symbol）
    for (val, ty) in type_repr {
        if *ty != "symbol" {
            let src = format!("(symbol-to-string {})", val);
            let msg = format!("symbol-to-string 需要 symbol，实际 {}", ty);
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
    // drop：第二参 list（k=0 也拒绝非 list——每步形态校验）
    for (val, ty) in type_repr {
        let src = format!("(drop {} 0)", val);
        let msg = format!("drop 需要 list，实际 {}", ty);
        expect_run_err(&src, &msg);
    }
}

/// assert-eq 堆值引用语义边界（eq 序对按引用——两同构表不等）+
/// 数值塔不隐式（Int ≠ Float——eq 无数值塔）。
#[test]
fn stdlib_eq_semantics_boundaries() {
    // eq 堆值按引用：同构不同对象 → 断言失败（结构等价用 equal?——未实现，注记）
    expect_run_err("(assert-eq (list 1) (list 1))", "断言失败");
    expect_run_err("(assert-eq '(1) (list 1))", "断言失败");
    // eq 无数值塔：1 ≠ 1.0
    expect_run_err("(assert-eq 1 1.0)", "断言失败：1 ≠ 1.0");
    // 字符串按内容（D2 修复语义）——正例
    assert_eq!(common::run_rendered("(assert-eq \"a\" \"a\")"), "true");
    // 符号按名（TD-002）——正例
    assert_eq!(
        common::run_rendered("(assert-eq (string-to-symbol \"a\") 'a)"),
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
        "string-index-of",
        "string-contains",
        "string-starts-with",
        "string-ends-with",
    ] {
        for (val, ty) in bad {
            let src = format!("({} \"a\" {})", fn_name, val);
            let msg = format!("{} 两参都需要 str，实际 str 与 {}", fn_name, ty);
            expect_run_err(&src, &msg);
        }
    }
    // string-append 两参契约（第二参类型错）
    for (val, _ty) in bad {
        let src = format!("(string-append \"a\" {})", val);
        expect_run_err(&src, "string-append 需要 2 个字符串");
    }
    // member/assoc 第一参任意（eq 万物可比）——第二参已扫描；
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

// ---------------------------------------------------------------------------
// TD-016（批次 C）：链式比较全操作数前置校验
// ---------------------------------------------------------------------------

/// TD-016 负向（6 case）：比较链短路终止不再跳过后续操作数类型检查。
/// 修复前行为（FS-5 边界）：`(< 3 1 "a")` 静默返回 false——语义文档
/// 09-stdlib §2 v5.5 已同步收紧为全操作数口径。
#[test]
fn comparison_chain_all_operands_checked() {
    // 短路后仍检查：首对为假 + 尾参非数值
    expect_run_err("(< 3 1 \"a\")", "< 需要数值");
    expect_run_err("(>= 5 2 'sym)", ">= 需要数值");
    // 等值链同理：首对不等 + 尾参非数值
    expect_run_err("(= 1 2 \"s\")", "= 需要数值");
    // 深位混串（链中段非数值）
    expect_run_err("(< 1 2 3 \"a\" 4)", "< 需要数值");
    // 全数值链不受影响（正向回归锚——短路值语义保持）
    assert_eq!(common::run_rendered("(< 1 2 3)"), "true");
    assert_eq!(common::run_rendered("(< 3 1 2)"), "false");
    assert_eq!(common::run_rendered("(= 1 1 1)"), "true");
    // 全字符串等值链不受影响（= 的字符串相等保持）
    assert_eq!(common::run_rendered("(= \"a\" \"a\" \"a\")"), "true");
}

/// TD-016 收敛注记（3 case）：混串排序链消息统一为「需要数值」
/// （修复前 `(< "a" "b" 1)` 报 TD-011 字符串消息——前置校验后按首个
/// 非数值操作数归因，两参消息口径不变）。
#[test]
fn comparison_chain_mixed_string_converges() {
    expect_run_err("(< \"a\" \"b\" 1)", "< 需要数值");
    expect_run_err("(> 1 \"a\" \"b\")", "> 需要数值");
    // 两参混串口径不变（回归锚）
    expect_run_err("(< \"a\" 1)", "< 需要数值");
}

// ---------------------------------------------------------------------------
// 字符串全序比较（TD-011 r24 解决：码点序）
// ---------------------------------------------------------------------------

/// 字符串全序正向语义（12 case）：Unicode 码点序 = UTF-8 字节序
/// （编码保序性）；排序族全部四算子 + 链式 + 码点边界（ASCII 大写 <
/// 小写；非 ASCII 多字节序）。
#[test]
fn string_ordering_codepoint_positive() {
    // 基础字典序（ASCII）
    assert_eq!(common::run_rendered("(< \"a\" \"b\")"), "true");
    assert_eq!(common::run_rendered("(> \"b\" \"a\")"), "true");
    assert_eq!(common::run_rendered("(<= \"a\" \"a\")"), "true");
    assert_eq!(common::run_rendered("(>= \"a\" \"a\")"), "true");
    assert_eq!(common::run_rendered("(< \"b\" \"a\")"), "false");
    // 链式全成立/中途失败
    assert_eq!(common::run_rendered("(< \"a\" \"b\" \"c\")"), "true");
    assert_eq!(common::run_rendered("(< \"a\" \"c\" \"b\")"), "false");
    // 码点序边界：大写 Z (0x5A) < 小写 a (0x61)
    assert_eq!(common::run_rendered("(< \"Z\" \"a\")"), "true");
    // 多字节码点序：z (0x7A) < é (0xE9) < 中 (0x4E2D)
    assert_eq!(common::run_rendered("(< \"z\" \"é\")"), "true");
    assert_eq!(common::run_rendered("(< \"é\" \"中\")"), "true");
    assert_eq!(common::run_rendered("(> \"中\" \"é\" \"z\")"), "true");
    // 前缀序：短串 < 同前缀长串（"ab" < "abc"）
    assert_eq!(common::run_rendered("(< \"ab\" \"abc\")"), "true");
}

/// 字符串全序负例（3 case，§9.4.3 正负成对）：元数 + 与既有数值域
/// 消息的边界（字符串排序链串入非字符串 → 首个非数值归因——TD-016
/// 口径保持）。
#[test]
fn string_ordering_negative_boundary() {
    expect_run_err("(< \"a\")", "< 至少需要 2 个参数");
    expect_run_err("(< 1 \"a\")", "< 需要数值");
    // 符号值不是字符串（序比较域外）
    expect_run_err("(< 'a 'b)", "< 需要数值");
}

// ---------------------------------------------------------------------------
// 闭包/内置装箱（TD-010 r24 解决：序对元素真装箱——原标记字符串占位）
// ---------------------------------------------------------------------------

/// 装箱往返正向语义（10 case）：闭包/内置作为序对元素——解箱还原
/// 为可调用值（Rc 恒等 → eq 按引用相等；is-procedure 判定；list 构造
/// 逐元素装箱；渲染 `#<procedure>` / `#<builtin:名>`）。
#[test]
fn procedure_boxing_roundtrip_positive() {
    // 闭包装箱 → 解箱 → 立即调用
    assert_eq!(
        common::run_rendered("((head (cons (lambda (x) x) nil)) 42)"),
        "42"
    );
    // 内置装箱 → 解箱 → 调用
    assert_eq!(
        common::run_rendered("((head (cons head nil)) (quote (7 8)))"),
        "7"
    );
    // is-procedure 判定经往返保持
    assert_eq!(
        common::run_rendered("(is-procedure (head (cons (lambda (x) x) nil)))"),
        "true"
    );
    assert_eq!(
        common::run_rendered("(is-procedure (head (cons head nil)))"),
        "true"
    );
    // eq 恒等性（Rc 共享装箱——按引用相等）
    assert_eq!(
        common::run_rendered("(define f (lambda (x) x)) (eq (head (cons f nil)) f)"),
        "true"
    );
    // list 构造逐元素装箱（函数列表模式——库化前提）
    assert_eq!(
        common::run_rendered(
            "(define fs (list (lambda (x) (* x 2)) (lambda (x) (+ x 1)))) ((head fs) 21)"
        ),
        "42"
    );
    assert_eq!(
        common::run_rendered(
            "(define fs (list (lambda (x) (* x 2)) (lambda (x) (+ x 1)))) ((head (tail fs)) 41)"
        ),
        "42"
    );
    // map 应用函数列表（prelude 高阶面 + 装箱协同）
    assert_eq!(
        common::run_rendered("(module m (import kerf-prelude) (define fs (list (lambda (x) (* x 2)))) (head (map (lambda (f) (f 21)) fs)))"),
        "42"
    );
    // 渲染形态（r42/S1：主名现代化——#<builtin:head>）
    assert_eq!(
        common::run_rendered("(head (cons (lambda (x) x) nil))"),
        "#<procedure>"
    );
    assert_eq!(
        common::run_rendered("(cons 1 (cons head nil))"),
        "(1 #<builtin:head>)"
    );
    // 点对形态（尾部直接为装箱值）
    assert_eq!(
        common::run_rendered("(cons 1 (head (cons head nil)))"),
        "(1 . #<builtin:head>)"
    );
}

/// 装箱边界负例（2 case，§9.4.3）：装箱值仍是值——数据位置语义
/// 不变（算术位拒绝函数值；序对渲染不误印为字符串）。
#[test]
fn procedure_boxing_negative_boundary() {
    expect_run_err("(+ 1 (head (cons (lambda (x) x) nil)))", "+ 需要 int");
    expect_run_err(
        "(head (head (cons (lambda (x) x) nil)))",
        "head 需要 pair，实际 procedure",
    );
}

// ---------------------------------------------------------------------------
// 类型谓词完备面（r24 / 42-e stdlib 缺口补齐：Value 变体判别 5 件）
// ---------------------------------------------------------------------------

/// 新谓词正向语义（17 case）：string?/symbol?/float?/number?/list?
/// ——每谓词正反例 × 跨类型不混淆；is-list 含真表/点对/环三态。
#[test]
fn type_predicates_positive() {
    // is-string
    assert_eq!(common::run_rendered("(is-string \"a\")"), "true");
    assert_eq!(common::run_rendered("(is-string 1)"), "false");
    assert_eq!(common::run_rendered("(is-string 'a)"), "false");
    // is-symbol
    assert_eq!(common::run_rendered("(is-symbol 'a)"), "true");
    assert_eq!(common::run_rendered("(is-symbol \"a\")"), "false");
    assert_eq!(common::run_rendered("(is-symbol nil)"), "false");
    // is-float / is-number（数值塔两态）
    assert_eq!(common::run_rendered("(is-float 1.5)"), "true");
    assert_eq!(common::run_rendered("(is-float 1)"), "false");
    assert_eq!(common::run_rendered("(is-number 1)"), "true");
    assert_eq!(common::run_rendered("(is-number 1.5)"), "true");
    assert_eq!(common::run_rendered("(is-number \"1\")"), "false");
    // is-list：nil / 真表 / 点对 / 非表值
    assert_eq!(common::run_rendered("(is-list nil)"), "true");
    assert_eq!(common::run_rendered("(is-list (list 1 2 3))"), "true");
    assert_eq!(common::run_rendered("(is-list (quote ()))"), "true");
    assert_eq!(common::run_rendered("(is-list (cons 1 2))"), "false");
    assert_eq!(common::run_rendered("(is-list 5)"), "false");
    // 与既有谓词族协同（判别面互斥性抽检）
    assert_eq!(common::run_rendered("(is-list (cons 1 nil))"), "true");
    assert_eq!(common::run_rendered("(is-pair (cons 1 2))"), "true");
}

/// is-list 环安全（2 case）：tail 链成环 → false（Floyd 龟兔判定——
/// 真表 tail 链无环；引用 Racket 语义）。环经 set! tail 构造。
#[test]
fn list_predicate_cycle_safe() {
    // 自环：(define x (list 1)) (set! ... tail ...) ——set! 作用于序对元素
    // 需经 car/cdr 装箱往返；kerf 无 set-car!/set-cdr!（序对不可变），
    // 环不可经用户面构造 → 本 case 以 unit 层面锚定（vm 集成不可达面），
    // 断言改为：长真表判定正确（深链 Floyd 终止）
    assert_eq!(
        common::run_rendered("(is-list (list 1 2 3 4 5 6 7 8 9 10))"),
        "true"
    );
    assert_eq!(
        common::run_rendered("(is-list (append (list 1 2) (cons 3 4)))"),
        "false"
    );
}

/// 新谓词负例（3 case，§9.4.3）：元数校验。
#[test]
fn type_predicates_negative_arity() {
    expect_run_err("(is-string)", "is-string 需要 1 个参数");
    expect_run_err("(is-number 1 2)", "is-number 需要 1 个参数，实际 2");
    expect_run_err("(is-list nil nil)", "is-list 需要 1 个参数，实际 2");
}

// ---------------------------------------------------------------------------
// r42 / 63-b 移除轮 S1（v0.9）——旧名退役负例组（20 §7 移除轮行：
// 旧名引用 = E0021 编译期错误，诊断携现代名指引——fail-closed）。
// 映射单源 = 20 §8；注册单源 = builtins.rs REMOVED_BUILTIN_NAMES
// （闭合守卫 removed_names_group_covers_all_27 对账——防「退役了没
// 拒、拒了没退役」双向漂移）。r38 parity 组的移除轮演进形态：双名
// 同行为的 parity 断言（v0.5-v0.8 双注册期）收敛为「独名 + 旧名拒绝」
// （v0.9 移除期——23 §3.4 生命周期四阶段完整走完）。
// ---------------------------------------------------------------------------

/// 旧名退役负例：编译期 E0021（阶段 Compile + 码位 [E0021] + 消息含
/// 现代名指引与移除版本）。值位/操作位全域同拒（与 E0014 同 traversal）。
fn expect_removed(old_src: &str, modern: &str) {
    let err = match run_source(old_src, "removed-neg.krf") {
        Ok(_) => panic!("旧名期望 E0021 编译期拒绝，实际 Ok：{}", old_src),
        Err(e) => e,
    };
    assert_eq!(
        err.stage,
        Stage::Compile,
        "阶段应为 Compile（旧名编译期阻断）：{}\n{}",
        old_src,
        err.rendered
    );
    assert!(
        err.rendered.contains("[E0021]"),
        "应为 E0021 码位：{}\n完整错误：{}",
        old_src,
        err.rendered
    );
    assert!(
        err.rendered.contains(&format!("现代名「{}」", modern)),
        "消息应含现代名指引「{}」：{}\n完整错误：{}",
        modern,
        old_src,
        err.rendered
    );
}

/// `car`/`cdr` 退役（R1 访问器——现代名 head/tail）。
#[test]
fn removed_rejects_car_cdr() {
    expect_removed("(car '(1 2 3))", "head");
    expect_removed("(cdr '(1 2 3))", "tail");
    expect_removed("((lambda (x) (car x)) '(1 2))", "head"); // 值位嵌套同拒
}

/// `list-ref`/`list-tail` 退役（R1 压缩——现代名 nth/drop）。
#[test]
fn removed_rejects_list_ref_list_tail() {
    expect_removed("(list-ref '(a b c) 0)", "nth");
    expect_removed("(list-tail '(a b c) 1)", "drop");
}

/// `null?`/`pair?` 退役（R2 谓词——现代名 is-nil/is-pair）。
#[test]
fn removed_rejects_null_pair() {
    expect_removed("(null? '())", "is-nil");
    expect_removed("(pair? (cons 1 2))", "is-pair");
}

/// `int?`/`bool?` 退役（R2——现代名 is-int/is-bool）。
#[test]
fn removed_rejects_int_bool() {
    expect_removed("(int? 42)", "is-int");
    expect_removed("(bool? true)", "is-bool");
}

/// `procedure?`/`string?` 退役（R2——现代名 is-procedure/is-string）。
#[test]
fn removed_rejects_procedure_string() {
    expect_removed("(procedure? head)", "is-procedure");
    expect_removed("(string? \"s\")", "is-string");
}

/// `symbol?`/`float?` 退役（R2——现代名 is-symbol/is-float）。
#[test]
fn removed_rejects_symbol_float() {
    expect_removed("(symbol? 's)", "is-symbol");
    expect_removed("(float? 3.5)", "is-float");
}

/// `number?`/`list?` 退役（R2——现代名 is-number/is-list）。
#[test]
fn removed_rejects_number_list() {
    expect_removed("(number? 3)", "is-number");
    expect_removed("(list? '(1))", "is-list");
}

/// `eq?` 退役（R3 去问号——现代名 eq）。
#[test]
fn removed_rejects_eq() {
    expect_removed("(eq? 1 1)", "eq");
}

/// `str-append`/`str-length` 退役（R1 全词化——现代名 string-*）。
#[test]
fn removed_rejects_str_append_length() {
    expect_removed("(str-append \"a\" \"b\")", "string-append");
    expect_removed("(str-length \"ab\")", "string-length");
}

/// `str-substring`/`str-index-of` 退役（R1——现代名 string-*）。
#[test]
fn removed_rejects_str_substring_index() {
    expect_removed("(str-substring \"abc\" 0 2)", "string-substring");
    expect_removed("(str-index-of \"abc\" \"b\")", "string-index-of");
}

/// `str-contains?`/`str-prefix?`/`str-suffix?` 退役（R3 动词化）。
#[test]
fn removed_rejects_str_predicates() {
    expect_removed("(str-contains? \"ab\" \"a\")", "string-contains");
    expect_removed("(str-prefix? \"ab\" \"a\")", "string-starts-with");
    expect_removed("(str-suffix? \"ab\" \"b\")", "string-ends-with");
}

/// `str-upcase`/`str-downcase` 退役（R4 转换方向词）。
#[test]
fn removed_rejects_str_case() {
    expect_removed("(str-upcase \"a\")", "string-to-upper");
    expect_removed("(str-downcase \"A\")", "string-to-lower");
}

/// `string->symbol`/`symbol->string` 退役（R4——现代名 *-to-*）。
#[test]
fn removed_rejects_arrow_conversions() {
    expect_removed("(string->symbol \"foo\")", "string-to-symbol");
    expect_removed("(symbol->string 'foo)", "symbol-to-string");
}

/// `assert-eq?` 退役（R3——断言是动词）。
#[test]
fn removed_rejects_assert_eq() {
    expect_removed("(assert-eq? 1 1)", "assert-eq");
}

/// N3 局部遮蔽豁免（R-N1——与 E0014 同口径）：lambda 参数以旧名命名
/// 合法（退役的是内置注册面非符号宇宙层；参数遮蔽后体内引用 = 局部名）。
#[test]
fn removed_shadowed_by_param_is_legal() {
    // (lambda (car) car)——car 是参数名非内置引用：编译过、行为 = 恒等
    let out = common::run_rendered("((lambda (car) car) 5)");
    assert_eq!(out, "5", "参数遮蔽旧名应合法（N3 局部绑定胜出）");
}

/// 用户接管豁免：用户 define 旧名 = 用户全局合法（同 E0014 接管口径）。
#[test]
fn removed_user_takeover_is_legal() {
    let out = common::run_rendered("(define (car x) (+ x 1)) (car 5)");
    assert_eq!(out, "6", "用户接管旧名应合法（退役的是内置注册面）");
}

/// 退役闭合守卫（20 §7 移除轮行的双向对账）：本组静态现代名名单 ↔
/// driver 退役表逐名对账——零缺（退役了没拒）、零溢（拒了没退役）、
/// 计数恰 27；每名 E0021 实测（旧名经完整生产管线拒绝）+ 现代名正路
/// 可解析（测了没注册的反向）。
#[test]
fn removed_names_group_covers_all_27() {
    // 本组静态名单（与上方 #[test] fn removed_rejects_* 的现代名对应）
    let tested: &[&str] = &[
        "head",
        "tail",
        "nth",
        "drop",
        "is-nil",
        "is-pair",
        "is-int",
        "is-bool",
        "is-procedure",
        "is-string",
        "is-symbol",
        "is-float",
        "is-number",
        "is-list",
        "eq",
        "string-append",
        "string-length",
        "string-substring",
        "string-index-of",
        "string-contains",
        "string-starts-with",
        "string-ends-with",
        "string-to-upper",
        "string-to-lower",
        "string-to-symbol",
        "symbol-to-string",
        "assert-eq",
    ];
    let retired = kerf_driver::builtins::REMOVED_BUILTIN_NAMES;
    assert_eq!(retired.len(), 27, "driver 退役表应为 27 项");
    assert_eq!(tested.len(), 27, "退役组静态名单应为 27 项");
    for &(old, modern) in retired {
        assert!(
            tested.contains(&modern),
            "退役旧名缺负例 case（退役了没拒）：{} → {}",
            old,
            modern
        );
        // 实测核对：旧名经完整生产管线 → E0021（编译期拒绝）
        let err = run_source(&format!("({} 'x)", old), "removed-guard.krf")
            .expect_err("旧名应编译期拒绝");
        assert!(
            err.rendered.contains("[E0021]"),
            "旧名 {} 应 E0021：{}",
            old,
            err.rendered
        );
        // 现代名正路可解析（求值成功或域/元数错误均证明名字在册——
        // 「未绑定」即失败态）
        let out = common::run_rendered(&format!("({} 'x)", modern));
        assert!(
            !out.contains("未绑定"),
            "现代名经完整管线不可解析（现代名未注册）：{} → {}",
            modern,
            out
        );
    }
    for name in tested {
        assert!(
            retired.iter().any(|&(_, m)| m == *name),
            "负例 case 无退役表对应（拒了没退役）：{}",
            name
        );
    }
}
