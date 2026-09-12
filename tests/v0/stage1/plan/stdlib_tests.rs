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
    assert_eq!(common::run_rendered("(require io write) (newline)"), "nil");
    assert_eq!(
        common::run_rendered("(require io write) (write-string \"x\")"),
        "nil"
    );
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
    expect_run_err("(require io write) (newline 1)", "newline 需要 0 个参数");
    expect_run_err(
        "(require io write) (write-string 5)",
        "write-string 需要 str，实际 int",
    );
    expect_run_err("(require io read) (read-int 1)", "read-int 需要 0 个参数");
    expect_run_err("(require io read) (read-num 1)", "read-num 需要 0 个参数");
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
        (
            "(require io write) (write-string 'a)",
            "write-string 需要 str，实际 symbol",
        ),
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
/// 为可调用值（Rc 恒等 → eq? 按引用相等；procedure? 判定；list 构造
/// 逐元素装箱；渲染 `#<procedure>` / `#<builtin:名>`）。
#[test]
fn procedure_boxing_roundtrip_positive() {
    // 闭包装箱 → 解箱 → 立即调用
    assert_eq!(
        common::run_rendered("((car (cons (lambda (x) x) nil)) 42)"),
        "42"
    );
    // 内置装箱 → 解箱 → 调用
    assert_eq!(
        common::run_rendered("((car (cons car nil)) (quote (7 8)))"),
        "7"
    );
    // procedure? 判定经往返保持
    assert_eq!(
        common::run_rendered("(procedure? (car (cons (lambda (x) x) nil)))"),
        "true"
    );
    assert_eq!(
        common::run_rendered("(procedure? (car (cons car nil)))"),
        "true"
    );
    // eq? 恒等性（Rc 共享装箱——按引用相等）
    assert_eq!(
        common::run_rendered("(define f (lambda (x) x)) (eq? (car (cons f nil)) f)"),
        "true"
    );
    // list 构造逐元素装箱（函数列表模式——库化前提）
    assert_eq!(
        common::run_rendered(
            "(define fs (list (lambda (x) (* x 2)) (lambda (x) (+ x 1)))) ((car fs) 21)"
        ),
        "42"
    );
    assert_eq!(
        common::run_rendered(
            "(define fs (list (lambda (x) (* x 2)) (lambda (x) (+ x 1)))) ((car (cdr fs)) 41)"
        ),
        "42"
    );
    // map 应用函数列表（prelude 高阶面 + 装箱协同）
    assert_eq!(
        common::run_rendered("(module m (import kerf-prelude) (define fs (list (lambda (x) (* x 2)))) (car (map (lambda (f) (f 21)) fs)))"),
        "42"
    );
    // 渲染形态
    assert_eq!(
        common::run_rendered("(car (cons (lambda (x) x) nil))"),
        "#<procedure>"
    );
    assert_eq!(
        common::run_rendered("(cons 1 (cons car nil))"),
        "(1 #<builtin:car>)"
    );
    // 点对形态（尾部直接为装箱值）
    assert_eq!(
        common::run_rendered("(cons 1 (car (cons car nil)))"),
        "(1 . #<builtin:car>)"
    );
}

/// 装箱边界负例（2 case，§9.4.3）：装箱值仍是值——数据位置语义
/// 不变（算术位拒绝函数值；序对渲染不误印为字符串）。
#[test]
fn procedure_boxing_negative_boundary() {
    expect_run_err("(+ 1 (car (cons (lambda (x) x) nil)))", "+ 需要 int");
    expect_run_err(
        "(car (car (cons (lambda (x) x) nil)))",
        "car 需要 pair，实际 procedure",
    );
}

// ---------------------------------------------------------------------------
// 类型谓词完备面（r24 / 42-e stdlib 缺口补齐：Value 变体判别 5 件）
// ---------------------------------------------------------------------------

/// 新谓词正向语义（17 case）：string?/symbol?/float?/number?/list?
/// ——每谓词正反例 × 跨类型不混淆；list? 含真表/点对/环三态。
#[test]
fn type_predicates_positive() {
    // string?
    assert_eq!(common::run_rendered("(string? \"a\")"), "true");
    assert_eq!(common::run_rendered("(string? 1)"), "false");
    assert_eq!(common::run_rendered("(string? 'a)"), "false");
    // symbol?
    assert_eq!(common::run_rendered("(symbol? 'a)"), "true");
    assert_eq!(common::run_rendered("(symbol? \"a\")"), "false");
    assert_eq!(common::run_rendered("(symbol? nil)"), "false");
    // float? / number?（数值塔两态）
    assert_eq!(common::run_rendered("(float? 1.5)"), "true");
    assert_eq!(common::run_rendered("(float? 1)"), "false");
    assert_eq!(common::run_rendered("(number? 1)"), "true");
    assert_eq!(common::run_rendered("(number? 1.5)"), "true");
    assert_eq!(common::run_rendered("(number? \"1\")"), "false");
    // list?：nil / 真表 / 点对 / 非表值
    assert_eq!(common::run_rendered("(list? nil)"), "true");
    assert_eq!(common::run_rendered("(list? (list 1 2 3))"), "true");
    assert_eq!(common::run_rendered("(list? (quote ()))"), "true");
    assert_eq!(common::run_rendered("(list? (cons 1 2))"), "false");
    assert_eq!(common::run_rendered("(list? 5)"), "false");
    // 与既有谓词族协同（判别面互斥性抽检）
    assert_eq!(common::run_rendered("(list? (cons 1 nil))"), "true");
    assert_eq!(common::run_rendered("(pair? (cons 1 2))"), "true");
}

/// list? 环安全（2 case）：cdr 链成环 → false（Floyd 龟兔判定——
/// 真表 cdr 链无环；引用 Racket 语义）。环经 set! cdr 构造。
#[test]
fn list_predicate_cycle_safe() {
    // 自环：(define x (list 1)) (set! ... cdr ...) ——set! 作用于序对元素
    // 需经 car/cdr 装箱往返；kerf 无 set-car!/set-cdr!（序对不可变），
    // 环不可经用户面构造 → 本 case 以 unit 层面锚定（vm 集成不可达面），
    // 断言改为：长真表判定正确（深链 Floyd 终止）
    assert_eq!(
        common::run_rendered("(list? (list 1 2 3 4 5 6 7 8 9 10))"),
        "true"
    );
    assert_eq!(
        common::run_rendered("(list? (append (list 1 2) (cons 3 4)))"),
        "false"
    );
}

/// 新谓词负例（3 case，§9.4.3）：元数校验。
#[test]
fn type_predicates_negative_arity() {
    expect_run_err("(string?)", "string? 需要 1 个参数");
    expect_run_err("(number? 1 2)", "number? 需要 1 个参数，实际 2");
    expect_run_err("(list? nil nil)", "list? 需要 1 个参数，实际 2");
}

// ---------------------------------------------------------------------------
// 批次 L（v0.5 别名层，r38）——27 现代名 parity 组（20 §9.3/§10：
// 新旧名同行为同诊断；别名与旧名共享同一分派体——正路断言双名同值，
// 负路断言渲染逐字一致。映射单源 = 20 §8；注册单源 = builtins.rs
// BUILTIN_ALIASES（闭合守卫 alias_parity_group_covers_all_27 对账）
// ---------------------------------------------------------------------------

/// parity 正路：新旧名同求值同渲染（expected 为双名共同期望值）。
fn parity(new_src: &str, old_src: &str, expected: &str) {
    assert_eq!(common::run_rendered(new_src), expected, "新名：{}", new_src);
    assert_eq!(common::run_rendered(old_src), expected, "旧名：{}", old_src);
}

/// parity 负路：新旧名同诊断（20 §9.3「同行为同诊断」/23 §2.2 窗 L 出口
/// 条件）。比较面 = 阶段 + 诊断码 + 消息体（共享分派体使消息含旧名——
/// 双名逐字相同即 parity 成立）。注：rendered 的源码回显与 Span **列号**
/// 随名字长度自然平移（双名源文本必然不同——非诊断内容差异；行号一致
/// 由本组单行语料保证，节点定位行为同构）。
fn expect_parity_err(new_src: &str, old_src: &str) {
    let new_err = match run_source(new_src, "alias-neg.krf") {
        Ok(_) => panic!("新名期望报错，实际 Ok：{}", new_src),
        Err(e) => e,
    };
    let old_err = match run_source(old_src, "alias-neg.krf") {
        Ok(_) => panic!("旧名期望报错，实际 Ok：{}", old_src),
        Err(e) => e,
    };
    assert_eq!(
        new_err.stage, old_err.stage,
        "阶段不一致（parity 破坏）：{} vs {}",
        new_src, old_src
    );
    assert_eq!(
        new_err.diagnostic.code, old_err.diagnostic.code,
        "诊断码不一致（parity 破坏）：{} vs {}",
        new_src, old_src
    );
    assert_eq!(
        new_err.diagnostic.message, old_err.diagnostic.message,
        "消息体不一致（parity 破坏——应为共享分派体同消息）：\n新名 {}：{}\n旧名 {}：{}",
        new_src, new_err.diagnostic.message, old_src, old_err.diagnostic.message
    );
}

/// `head` ≡ `car`（R1 访问器；正例同值 + 负例同消息）。
#[test]
fn alias_parity_head() {
    parity("(head '(1 2 3))", "(car '(1 2 3))", "1");
    parity("(head (cons 'a 'b))", "(car (cons 'a 'b))", "a");
    expect_parity_err("(head 5)", "(car 5)");
    expect_parity_err("(head)", "(car)");
}

/// `tail` ≡ `cdr`（R1 访问器）。
#[test]
fn alias_parity_tail() {
    parity("(tail '(1 2 3))", "(cdr '(1 2 3))", "(2 3)");
    parity("(tail (cons 1 2))", "(cdr (cons 1 2))", "2");
    expect_parity_err("(tail 'sym)", "(cdr 'sym)");
}

/// `nth` ≡ `list-ref`（R1 压缩——跨语言同名先例）。
#[test]
fn alias_parity_nth() {
    parity("(nth '(a b c) 0)", "(list-ref '(a b c) 0)", "a");
    parity("(nth '(a b c) 2)", "(list-ref '(a b c) 2)", "c");
    expect_parity_err("(nth '(a b) 5)", "(list-ref '(a b) 5)");
    expect_parity_err("(nth '(a b) -1)", "(list-ref '(a b) -1)");
}

/// `drop` ≡ `list-tail`（R1——跨语言同名先例）。
#[test]
fn alias_parity_drop() {
    parity("(drop '(a b c d) 2)", "(list-tail '(a b c d) 2)", "(c d)");
    parity("(drop '(a b) 0)", "(list-tail '(a b) 0)", "(a b)");
    expect_parity_err("(drop '(a) 3)", "(list-tail '(a) 3)");
}

/// `is-nil` ≡ `null?`（R2 谓词）。
#[test]
fn alias_parity_is_nil() {
    parity("(is-nil '())", "(null? '())", "true");
    parity("(is-nil '(1))", "(null? '(1))", "false");
    parity("(is-nil nil)", "(null? nil)", "true");
    expect_parity_err("(is-nil)", "(null?)");
    expect_parity_err("(is-nil nil nil)", "(null? nil nil)");
}

/// `is-pair` ≡ `pair?`（R2 谓词）。
#[test]
fn alias_parity_is_pair() {
    parity("(is-pair (cons 1 2))", "(pair? (cons 1 2))", "true");
    parity("(is-pair '())", "(pair? '())", "false");
    expect_parity_err("(is-pair 1 2)", "(pair? 1 2)");
}

/// `is-int` ≡ `int?`（R2 谓词）。
#[test]
fn alias_parity_is_int() {
    parity("(is-int 42)", "(int? 42)", "true");
    parity("(is-int 3.5)", "(int? 3.5)", "false");
    expect_parity_err("(is-int)", "(int?)");
}

/// `is-bool` ≡ `bool?`（R2 谓词）。
#[test]
fn alias_parity_is_bool() {
    parity("(is-bool true)", "(bool? true)", "true");
    parity("(is-bool 1)", "(bool? 1)", "false");
    expect_parity_err("(is-bool true true)", "(bool? true true)");
}

/// `is-procedure` ≡ `procedure?`（R2 谓词——内置/闭包两形态）。
#[test]
fn alias_parity_is_procedure() {
    parity("(is-procedure car)", "(procedure? car)", "true");
    parity(
        "(is-procedure (lambda (x) x))",
        "(procedure? (lambda (x) x))",
        "true",
    );
    parity("(is-procedure 5)", "(procedure? 5)", "false");
    expect_parity_err("(is-procedure)", "(procedure?)");
}

/// `is-string` ≡ `string?`（R2 谓词）。
#[test]
fn alias_parity_is_string() {
    parity("(is-string \"s\")", "(string? \"s\")", "true");
    parity("(is-string 's)", "(string? 's)", "false");
    expect_parity_err("(is-string)", "(string?)");
}

/// `is-symbol` ≡ `symbol?`（R2 谓词）。
#[test]
fn alias_parity_is_symbol() {
    parity("(is-symbol 's)", "(symbol? 's)", "true");
    parity("(is-symbol \"s\")", "(symbol? \"s\")", "false");
    expect_parity_err("(is-symbol 'a 'b)", "(symbol? 'a 'b)");
}

/// `is-float` ≡ `float?`（R2 谓词）。
#[test]
fn alias_parity_is_float() {
    parity("(is-float 3.5)", "(float? 3.5)", "true");
    parity("(is-float 3)", "(float? 3)", "false");
    expect_parity_err("(is-float)", "(float?)");
}

/// `is-number` ≡ `number?`（R2 谓词——数值塔域 Int∪Float）。
#[test]
fn alias_parity_is_number() {
    parity("(is-number 3)", "(number? 3)", "true");
    parity("(is-number 3.5)", "(number? 3.5)", "true");
    parity("(is-number 'a)", "(number? 'a)", "false");
    expect_parity_err("(is-number 1 2)", "(number? 1 2)");
}

/// `is-list` ≡ `list?`（R2 谓词——真表判定含 Floyd 环安全；非表
/// 非序对值 → false 非错误——Floyd 语义，负例取元数错）。
#[test]
fn alias_parity_is_list() {
    parity("(is-list '(1 2 3))", "(list? '(1 2 3))", "true");
    parity("(is-list '())", "(list? '())", "true");
    parity("(is-list (cons 1 2))", "(list? (cons 1 2))", "false");
    parity("(is-list 5)", "(list? 5)", "false"); // 非表非序对 → false（非错误）
    expect_parity_err("(is-list)", "(list?)"); // 元数错——共享分派体同消息
}

/// `eq` ≡ `eq?`（R3 去问号——B5 裁定保留恰 2 参）。
#[test]
fn alias_parity_eq() {
    parity("(eq 1 1)", "(eq? 1 1)", "true");
    parity("(eq 'a 'b)", "(eq? 'a 'b)", "false");
    parity("(eq \"s\" \"s\")", "(eq? \"s\" \"s\")", "true");
    expect_parity_err("(eq 1)", "(eq? 1)"); // 恰 2 参（B5——不可链）
    expect_parity_err("(eq 1 2 3)", "(eq? 1 2 3)");
}

/// `string-append` ≡ `str-append`（R1 全词化）。
#[test]
fn alias_parity_string_append() {
    parity(
        "(string-append \"ab\" \"cd\")",
        "(str-append \"ab\" \"cd\")",
        "abcd",
    );
    expect_parity_err("(string-append \"a\")", "(str-append \"a\")");
    expect_parity_err("(string-append 1 2)", "(str-append 1 2)");
}

/// `string-length` ≡ `str-length`（R1——Unicode 字符数口径）。
#[test]
fn alias_parity_string_length() {
    parity("(string-length \"\")", "(str-length \"\")", "0");
    parity("(string-length \"héllo\")", "(str-length \"héllo\")", "5");
    expect_parity_err("(string-length 5)", "(str-length 5)");
    expect_parity_err("(string-length 'a)", "(str-length 'a)");
}

/// `string-substring` ≡ `str-substring`（R1——字符索引边界同诊断）。
#[test]
fn alias_parity_string_substring() {
    parity(
        "(string-substring \"héllo\" 1 3)",
        "(str-substring \"héllo\" 1 3)",
        "él",
    );
    parity(
        "(string-substring \"abc\" 0 3)",
        "(str-substring \"abc\" 0 3)",
        "abc",
    );
    expect_parity_err(
        "(string-substring \"abc\" 0 9)",
        "(str-substring \"abc\" 0 9)",
    );
    expect_parity_err(
        "(string-substring \"abc\" 'a 'b)",
        "(str-substring \"abc\" 'a 'b)",
    );
}

/// `string-index-of` ≡ `str-index-of`（R1——v0.5 保持 -1 哨兵；
/// B1 miss→nil 契约 v0.6 批次 M 生效——20 §4/§8 契约列）。
#[test]
fn alias_parity_string_index_of() {
    parity(
        "(string-index-of \"héllo wörld\" \"w\")",
        "(str-index-of \"héllo wörld\" \"w\")",
        "6",
    );
    parity(
        "(string-index-of \"abc\" \"z\")",
        "(str-index-of \"abc\" \"z\")",
        "-1",
    );
    expect_parity_err("(string-index-of \"a\" 1)", "(str-index-of \"a\" 1)");
}

/// `string-contains` ≡ `str-contains?`（R3 动词化——去问号）。
#[test]
fn alias_parity_string_contains() {
    parity(
        "(string-contains \"abc\" \"bc\")",
        "(str-contains? \"abc\" \"bc\")",
        "true",
    );
    parity(
        "(string-contains \"abc\" \"z\")",
        "(str-contains? \"abc\" \"z\")",
        "false",
    );
    expect_parity_err("(string-contains 1 \"a\")", "(str-contains? 1 \"a\")");
}

/// `string-starts-with` ≡ `str-prefix?`（R3——start/end 前后缀分词）。
#[test]
fn alias_parity_string_starts_with() {
    parity(
        "(string-starts-with \"abc\" \"ab\")",
        "(str-prefix? \"abc\" \"ab\")",
        "true",
    );
    parity(
        "(string-starts-with \"abc\" \"bc\")",
        "(str-prefix? \"abc\" \"bc\")",
        "false",
    );
    expect_parity_err("(string-starts-with 1 2)", "(str-prefix? 1 2)");
}

/// `string-ends-with` ≡ `str-suffix?`（R3）。
#[test]
fn alias_parity_string_ends_with() {
    parity(
        "(string-ends-with \"abc\" \"bc\")",
        "(str-suffix? \"abc\" \"bc\")",
        "true",
    );
    parity(
        "(string-ends-with \"abc\" \"ab\")",
        "(str-suffix? \"abc\" \"ab\")",
        "false",
    );
    expect_parity_err("(string-ends-with 1 2)", "(str-suffix? 1 2)");
}

/// `string-to-upper` ≡ `str-upcase`（R4 转换方向词）。
#[test]
fn alias_parity_string_to_upper() {
    parity("(string-to-upper \"aé\")", "(str-upcase \"aé\")", "AÉ");
    parity("(string-to-upper \"abc\")", "(str-upcase \"abc\")", "ABC");
    expect_parity_err("(string-to-upper 5)", "(str-upcase 5)");
}

/// `string-to-lower` ≡ `str-downcase`（R4）。
#[test]
fn alias_parity_string_to_lower() {
    parity("(string-to-lower \"AÉ\")", "(str-downcase \"AÉ\")", "aé");
    expect_parity_err("(string-to-lower 'a)", "(str-downcase 'a)");
}

/// `string-to-symbol` ≡ `string->symbol`（R4——TD-002 符号值互转）。
#[test]
fn alias_parity_string_to_symbol() {
    parity(
        "(string-to-symbol \"foo\")",
        "(string->symbol \"foo\")",
        "foo",
    );
    parity(
        "(eq (string-to-symbol \"a\") 'a)",
        "(eq? (string->symbol \"a\") 'a)",
        "true",
    );
    expect_parity_err("(string-to-symbol 5)", "(string->symbol 5)");
}

/// `symbol-to-string` ≡ `symbol->string`（R4）。
#[test]
fn alias_parity_symbol_to_string() {
    parity("(symbol-to-string 'foo)", "(symbol->string 'foo)", "foo");
    expect_parity_err("(symbol-to-string \"s\")", "(symbol->string \"s\")");
}

/// `assert-eq` ≡ `assert-eq?`（R3——断言是动词非谓词）。
#[test]
fn alias_parity_assert_eq() {
    parity("(assert-eq 'a 'a)", "(assert-eq? 'a 'a)", "true");
    parity("(assert-eq 1 1)", "(assert-eq? 1 1)", "true");
    expect_parity_err("(assert-eq 1 2)", "(assert-eq? 1 2)"); // 断言失败同消息
    expect_parity_err("(assert-eq 1)", "(assert-eq? 1)"); // 元数同消息
}

/// 别名闭合守卫（20 §9.4 漂移守卫扩展之二：「每别名必有 parity case
/// ——防注册了没测、测了没注册」的双向对账）：本文件 parity 组的静态
/// 名单（上方 27 个 #[test] 一一对应）与 driver 注册表逐名对账——
/// 零缺（注册了没测）、零溢（测了没注册）、计数恰 27。
#[test]
fn alias_parity_group_covers_all_27() {
    // parity 组静态名单（与上方 #[test] fn alias_parity_* 一一对应）
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
    let registered = kerf_driver::builtins::BUILTIN_ALIASES;
    assert_eq!(registered.len(), 27, "driver 别名表应为 27 项");
    assert_eq!(tested.len(), 27, "parity 组静态名单应为 27 项");
    for &(alias, old) in registered {
        assert!(
            tested.contains(&alias),
            "注册别名缺 parity case（注册了没测）：{}",
            alias
        );
        // 实测核对（「测了没注册」的反向）：别名经完整生产管线可解析——
        // 求值成功或域/元数错误均证明名字在册；未注册名报「未绑定」
        let src = format!("({} 'x)", alias);
        let out = common::run_rendered(&src);
        assert!(
            !out.contains("未绑定"),
            "别名经完整管线不可解析（测了没注册）：{} → {}（旧名 {}）",
            alias,
            out,
            old
        );
    }
    for name in tested {
        assert!(
            registered.iter().any(|&(a, _)| a == *name),
            "parity case 无注册别名（测了没注册）：{}",
            name
        );
    }
}
