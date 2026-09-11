//! 负向 Expander 测试（tests/v0/stage0/plan/negative_expander_tests.rs ↔
//! docs/tests/v0/stage0/plan/negative-tests.md）。
//!
//! §9.4.3 定向扩张：本文件覆盖 **Expand 阶段（E0002）** 全部错误族——
//! 空应用（§7.1.1 类 3，T14-b 发现的零测试分类）、21 个关键字的误用
//! 矩阵、宏系统负例（模式不匹配/展开深度超限）、相位簿记违规
//! （declare/visit/instantiate + 模块循环依赖存档）。
//!
//! 每行 = 1 case（表格驱动）。所有期望消息/Span 均经实测校准。
//!
//! 语义边界（实测确认，非负例，按实际行为归类）：
//! - `(define)`/`(lambda)`/`(if)` 等**结构**错误在 Expand 阶段报 E0002
//!   （非 Reader——Reader 只查括号三态与字符合法性）；
//! - `(begin)`/`(and)`/`(or)` 零参数合法（空 begin → nil；and → true；or → false）；
//! - `(if c t)` 双分支形态合法（缺省 else → nil）；
//! - 宏模板中未绑定于模式的标识符**不报错**——经卫生重命名成为引入
//!   标识符，错误推迟到 Run 阶段（未绑定全局）——见语义发现 FS-3。
//!
//! 已知缺陷存档（实测确认，#[ignore]，不计 case 数）：
//! - 深递归 eval 路径 Rust 栈溢出 abort（VM 路径有 MAX_FRAMES 结构化
//!   错误）——`deep_recursion_eval_path_ignored`（归入 VM 文件同款）。
//! - 模块循环依赖已修复（phase.rs DFS 灰标记 → 结构化 Err
//!   「模块循环依赖：Symbol(N) → …」）——见 `module_cycle_dependency_errors`。

use kerf_driver::{run_source, Stage};
use kerf_expander::phase::ModuleRegistry;
use kerf_syntax::Symbol;

/// 通用断言：src 失败于 Expand 阶段，渲染错误含消息子串。
fn expect_expand_err(src: &str, msg: &str) -> kerf_driver::DriverError {
    let err = match run_source(src, "neg.krf") {
        Ok(_) => panic!("期望报错，实际 Ok：{}", src),
        Err(e) => e,
    };
    assert_eq!(
        err.stage,
        Stage::Expand,
        "阶段不符：{}\n{}",
        src,
        err.rendered
    );
    assert!(
        err.rendered.contains(msg),
        "消息缺少「{}」：{}\n完整错误：{}",
        msg,
        src,
        err.rendered
    );
    err
}

/// 断言错误 Span 精确指向。
fn expect_span(err: &kerf_driver::DriverError, start: u32, end: u32) {
    assert_eq!(err.diagnostic.primary_span.start, start, "Span.start 不符");
    assert_eq!(err.diagnostic.primary_span.end, end, "Span.end 不符");
}

/// 空应用 `()`（§7.1.1 类 3——T14-b 零测试分类补齐）（11 case）：
/// 多形态——顶层 / begin 体 / 调用位 / 参数位 / let 绑定值 / if 条件 /
/// define 值 / set! 值 / 三层嵌套内层 / 多个 / begin 尾部。
#[test]
fn empty_application_family() {
    let cases: &[(&str, u32, u32)] = &[
        ("()", 0, 2),
        ("(begin ())", 7, 9),
        ("(() 1)", 1, 3),
        ("(+ ())", 3, 5),
        ("() ()", 0, 2),
        ("(let ((x ())) 1)", 9, 11),
        ("(if () 1 2)", 4, 6),
        ("(define x ())", 10, 12),
        ("(define x 1) (set! x ())", 21, 23),
        ("((()))", 2, 4),
        ("(begin 1 ())", 9, 11),
    ];
    for (src, s, e) in cases {
        let err = expect_expand_err(src, "空列表不能作为表达式求值");
        expect_span(&err, *s, *e);
        assert!(
            err.rendered.contains("error[E0002]"),
            "应携带 E0002：{}",
            src
        );
    }
}

/// lambda 误用（8 case）：元数/参数表结构/参数类型/重名/嵌套重名/
/// 体部重复 define（提升路径映射为参数重名）。
#[test]
fn lambda_misuse() {
    let cases: &[(&str, &str)] = &[
        ("(lambda)", "lambda 形式：(lambda (参数...) 体...)"),
        ("(lambda (x))", "lambda 形式：(lambda (参数...) 体...)"),
        ("(lambda x 1)", "lambda 参数必须是符号列表"),
        ("(lambda (1) 1)", "lambda 参数必须是符号（不支持解构参数）"),
        (
            "(lambda (x \"s\") x)",
            "lambda 参数必须是符号（不支持解构参数）",
        ),
        ("(lambda (x x) x)", "lambda 参数重名"),
        ("((lambda (x) (lambda (x x) x)) 1)", "lambda 参数重名"),
        ("(lambda () (define x 1) (define x 2))", "lambda 参数重名"),
    ];
    for (src, msg) in cases {
        expect_expand_err(src, msg);
    }
}

/// if 误用（3 case）：元数 0/1/5。
#[test]
fn if_misuse() {
    for (src, span) in [
        ("(if)", (0u32, 4u32)),
        ("(if true)", (0, 9)),
        ("(if true 1 2 3)", (0, 15)),
    ] {
        let err = expect_expand_err(src, "if 形式：(if 条件 真分支 [假分支])");
        expect_span(&err, span.0, span.1);
    }
}

/// set! 误用（4 case）：元数 2/4、目标非符号。
#[test]
fn setbang_misuse() {
    let cases: &[(&str, &str, u32, u32)] = &[
        ("(set! x)", "set! 形式：(set! 名 值)", 0, 8),
        ("(set! x 1 2)", "set! 形式：(set! 名 值)", 0, 12),
        ("(set! 1 2)", "set! 目标必须是符号", 6, 7),
        ("(set! \"s\" 1)", "set! 目标必须是符号", 6, 9),
    ];
    for (src, msg, s, e) in cases {
        let err = expect_expand_err(src, msg);
        expect_span(&err, *s, *e);
    }
}

/// define 误用（7 case）：元数/目标非符号/多值表达式/函数糖头非符号/
/// 函数糖无体。
#[test]
fn define_misuse() {
    let cases: &[(&str, &str)] = &[
        (
            "(define)",
            "define 形式：(define 名 值) 或 (define (名 参数...) 体...)",
        ),
        (
            "(define x)",
            "define 形式：(define 名 值) 或 (define (名 参数...) 体...)",
        ),
        ("(define 1 2)", "define 目标必须是符号"),
        ("(define x 1 2)", "define 值形式必须是单个表达式"),
        ("(define (1 x) x)", "define 函数糖首元素必须是符号"),
        ("(define (\"s\" x) x)", "define 函数糖首元素必须是符号"),
        (
            "(define (f))",
            "define 形式：(define 名 值) 或 (define (名 参数...) 体...)",
        ),
    ];
    for (src, msg) in cases {
        expect_expand_err(src, msg);
    }
}

/// module 误用（4 case）：缺名/名非符号/import/export 项非符号。
#[test]
fn module_misuse() {
    let cases: &[(&str, &str)] = &[
        (
            "(module)",
            "module 形式：(module 名 [(import ...)] [(export ...)] 体...)",
        ),
        ("(module 1)", "module 名必须是符号"),
        ("(module m (import 1))", "import 项必须是符号"),
        ("(module m (export 1))", "export 项必须是符号"),
    ];
    for (src, msg) in cases {
        expect_expand_err(src, msg);
    }
}

/// import/export 误用（4 case）：module 外使用（顶层 + lambda 体）。
#[test]
fn import_export_misuse() {
    for src in [
        "(import x)",
        "(export x)",
        "(lambda () (import x))",
        "(lambda () (export x))",
    ] {
        expect_expand_err(src, "import/export 只能出现在 module 形式内部");
    }
}

/// quote 误用（4 case）：元数/向量值（Vector 推迟）。
/// TD-002 解除后符号 datum 为正例——符号值的**语义**负例（算术/条件
/// 位置误用）在 negative_vm_tests / negative_semantics_tests 层锚定。
#[test]
fn quote_misuse() {
    let cases: &[(&str, &str)] = &[
        ("(quote)", "quote 形式：(quote 数据)"),
        ("(quote x y)", "quote 形式：(quote 数据)"),
        ("'[1 2]", "quote 向量暂不支持"),
        ("'[1 [2 a]]", "quote 向量暂不支持"),
    ];
    for (src, msg) in cases {
        expect_expand_err(src, msg);
    }
}

/// let 误用（6 case）：元数/绑定组非列表/绑定非二元/绑定名非符号。
#[test]
fn let_misuse() {
    let cases: &[(&str, &str)] = &[
        ("(let)", "宏「let」展开失败：let 形式"),
        ("(let ((x 1)))", "宏「let」展开失败：let 形式"),
        ("(let 1 2)", "宏「let」展开失败：let 绑定组必须是列表"),
        (
            "(let ((x)) 1)",
            "宏「let」展开失败：let 绑定必须是 (名 值) 二元列表",
        ),
        (
            "(let ((x 1 2)) 1)",
            "宏「let」展开失败：let 绑定必须是 (名 值) 二元列表",
        ),
        ("(let ((1 2)) 1)", "宏「let」展开失败：let 绑定名必须是符号"),
    ];
    for (src, msg) in cases {
        expect_expand_err(src, msg);
    }
}

/// letrec 误用（5 case）。
#[test]
fn letrec_misuse() {
    let cases: &[(&str, &str)] = &[
        ("(letrec)", "宏「letrec」展开失败：letrec 形式"),
        (
            "(letrec ((x)) 1)",
            "宏「letrec」展开失败：letrec 绑定必须是 (名 值) 二元列表",
        ),
        (
            "(letrec ((x 1 2)) 1)",
            "宏「letrec」展开失败：letrec 绑定必须是 (名 值) 二元列表",
        ),
        (
            "(letrec ((1 2)) 1)",
            "宏「letrec」展开失败：letrec 绑定名必须是符号",
        ),
        (
            "(letrec 1 1)",
            "宏「letrec」展开失败：letrec 绑定组必须是列表",
        ),
    ];
    for (src, msg) in cases {
        expect_expand_err(src, msg);
    }
}

/// let* 误用（4 case）——注意降级链：`(let* ((x)) 1)` 报的是内层
/// let 的绑定错误（推导表展开路径）。
#[test]
fn letstar_misuse() {
    let cases: &[(&str, &str)] = &[
        ("(let*)", "宏「let*」展开失败：let* 形式"),
        (
            "(let* ((x)) 1)",
            "宏「let」展开失败：let 绑定必须是 (名 值) 二元列表",
        ),
        (
            "(let* ((x 1 2)) 1)",
            "宏「let」展开失败：let 绑定必须是 (名 值) 二元列表",
        ),
        ("(let* 1 1)", "宏「let*」展开失败：let* 绑定组必须是列表"),
    ];
    for (src, msg) in cases {
        expect_expand_err(src, msg);
    }
}

/// cond 误用（4 case）：零子句/子句非列表/空子句/测试位空列表。
#[test]
fn cond_misuse() {
    let cases: &[(&str, &str)] = &[
        ("(cond)", "宏「cond」展开失败：cond 需要至少一个子句"),
        ("(cond 1)", "宏「cond」展开失败：cond 子句必须是非空列表"),
        ("(cond ())", "宏「cond」展开失败：cond 子句必须是非空列表"),
        ("(cond (()))", "空列表不能作为表达式求值"),
    ];
    for (src, msg) in cases {
        expect_expand_err(src, msg);
    }
}

/// else 误用（1 case）：非 cond 测试位置。
#[test]
fn else_misuse() {
    let err = expect_expand_err("(else 1)", "else 只能出现在 cond 子句的测试位置");
    expect_span(&err, 0, 8);
}

/// when/while/unless 误用（3 case）：零参数。
#[test]
fn derived_form_misuse() {
    let cases: &[(&str, &str)] = &[
        ("(when)", "宏「when」展开失败：when 形式"),
        ("(while)", "宏「while」展开失败：while 形式"),
        ("(unless)", "宏「unless」展开失败：unless 形式"),
    ];
    for (src, msg) in cases {
        expect_expand_err(src, msg);
    }
}

/// define-syntax 误用（8 case）：元数/名非符号/第二参数非 syntax-rules/
/// syntax-rules 内部结构（缺参/字面量非符号/子句非二元/子句三元）。
#[test]
fn define_syntax_misuse() {
    let cases: &[(&str, &str)] = &[
        (
            "(define-syntax)",
            "define-syntax 形式：(define-syntax 名 (syntax-rules ...))",
        ),
        (
            "(define-syntax m)",
            "define-syntax 形式：(define-syntax 名 (syntax-rules ...))",
        ),
        (
            "(define-syntax 1 (syntax-rules () (_ 1)))",
            "define-syntax 名必须是符号",
        ),
        (
            "(define-syntax m 1)",
            "define-syntax 第二参数必须是 (syntax-rules ...) 形式",
        ),
        (
            "(define-syntax m (syntax-rules))",
            "syntax-rules 解析失败：syntax-rules 缺少参数",
        ),
        (
            "(define-syntax m (syntax-rules (1) (_ 1)))",
            "syntax-rules 解析失败：syntax-rules 字面量必须是符号",
        ),
        (
            "(define-syntax m (syntax-rules () 1))",
            "syntax-rules 解析失败：syntax-rules 子句必须是 (pattern template) 二元列表",
        ),
        (
            "(define-syntax m (syntax-rules () (_ 1 2)))",
            "syntax-rules 解析失败：syntax-rules 子句必须是 (pattern template) 二元列表",
        ),
    ];
    for (src, msg) in cases {
        expect_expand_err(src, msg);
    }
}

/// syntax-rules 裸用（2 case）：define-syntax 外出现。
#[test]
fn syntax_rules_misuse() {
    for src in [
        "(syntax-rules () (_ 1))",
        "(lambda () (syntax-rules () (_ 1)))",
    ] {
        expect_expand_err(src, "syntax-rules 只能出现在 define-syntax 内部");
    }
}

/// 向量出现在表达式位置（1 case）。
#[test]
fn vector_in_expression_position() {
    let err = expect_expand_err("[1 2 3]", "向量不能出现在表达式位置");
    expect_span(&err, 0, 7);
}

/// define 不在函数体头部（1 case）——§19.2 陷阱 3 的反面。
#[test]
fn define_not_at_body_head() {
    let err = expect_expand_err("(lambda () 1 (define x 2))", "define 必须位于函数体头部");
    expect_span(&err, 13, 25);
}

/// 宏展开负例（6 case）：调用与模式不匹配 ×3（多余实参/裸符号模式/
/// 模板变量未入模式）、自引用展开深度超限（§7.1.1 类 7）、
/// 自引用乘性扩展（不匹配路径）。
#[test]
fn macro_expansion_failures() {
    let cases: &[(&str, &str)] = &[
        (
            "(define-syntax m (syntax-rules () (_ 1))) (m 1 2)",
            "宏「m」展开失败：宏调用与全部子句模式均不匹配",
        ),
        (
            "(define-syntax m (syntax-rules () (x 1))) (m 1)",
            "宏「m」展开失败：宏调用与全部子句模式均不匹配",
        ),
        (
            "(define-syntax m (syntax-rules () (_ x))) (m 1)",
            "宏「m」展开失败：宏调用与全部子句模式均不匹配",
        ),
        (
            "(define-syntax lo (syntax-rules () ((lo) (lo)))) (lo)",
            "宏展开深度超过上限 10000（疑似无限递归展开）",
        ),
        (
            "(define-syntax lo (syntax-rules () ((lo) (lo lo)))) (lo)",
            "宏「lo」展开失败：宏调用与全部子句模式均不匹配",
        ),
        (
            "(define-syntax m (syntax-rules () ((_ 1) 1) ((_ x) 2))) (m)",
            "宏「m」展开失败：宏调用与全部子句模式均不匹配",
        ),
    ];
    for (src, msg) in cases {
        expect_expand_err(src, msg);
    }
}

/// 相位簿记（4 case，ModuleRegistry 公共 API 直测）：
/// 重复 declare / 未声明即 visit / 未 visit 即 instantiate / visit 传递
/// 依赖未声明。
#[test]
fn module_registry_phase_violations() {
    // 重复声明（显式失败，不静默覆盖）
    let mut reg = ModuleRegistry::new();
    reg.declare(Symbol(1), vec![], vec![]).unwrap();
    let err = reg.declare(Symbol(1), vec![], vec![]).unwrap_err();
    assert!(err.contains("模块重复声明"), "实际：{}", err);
    // 未声明即 visit
    let mut reg2 = ModuleRegistry::new();
    let err2 = reg2.visit(Symbol(9)).unwrap_err();
    assert!(err2.contains("未声明的模块"), "实际：{}", err2);
    // 未 visit 即 instantiate（相位违规）
    let mut reg3 = ModuleRegistry::new();
    reg3.declare(Symbol(1), vec![], vec![]).unwrap();
    let err3 = reg3.instantiate(Symbol(1)).unwrap_err();
    assert!(err3.contains("未 visit 即 instantiate"), "实际：{}", err3);
    // 传递依赖破坏：visit(m2) 需要先 visit 未声明的 m1
    let mut reg4 = ModuleRegistry::new();
    reg4.declare(Symbol(2), vec![Symbol(1)], vec![]).unwrap();
    let err4 = reg4.visit(Symbol(2)).unwrap_err();
    assert!(err4.contains("未声明的模块"), "实际：{}", err4);
}

/// 从源码可达的相位错误（1 case）：模块 import 未声明模块。
/// 注：driver 对 registry 错误走特殊渲染路径——无 error[E0002] 前缀，
/// 仅 `[expand] 未声明的模块` Display 形态（如实断言当前行为）。
#[test]
fn module_import_undeclared_from_source() {
    let err = match run_source("(module m (import x) 1)", "neg.krf") {
        Ok(_) => panic!("期望报错，实际 Ok"),
        Err(e) => e,
    };
    assert_eq!(err.stage, Stage::Expand);
    assert!(err.to_string().contains("[expand]"), "实际：{}", err);
    assert!(
        err.rendered.contains("未声明的模块"),
        "实际：{}",
        err.rendered
    );
}

/// E0002 渲染形状完整断言（3 case）：error[E0002] + `-->` + 源摘录。
#[test]
fn expander_error_rendering_shape() {
    let err = expect_expand_err("(lambda (x))", "lambda 形式");
    assert!(err.rendered.contains("error[E0002]: lambda 形式"));
    assert!(err.rendered.contains("--> neg.krf:1:1"));
    assert!(err.rendered.contains("1 | (lambda (x))"));
    let err2 = expect_expand_err("(set! 1 2)", "set! 目标必须是符号");
    assert!(err2.rendered.contains("--> neg.krf:1:7"));
    let err3 = expect_expand_err("(lambda (x x) x)", "lambda 参数重名");
    assert!(err3.rendered.contains("--> neg.krf:1:12"));
    assert!(
        err3.diagnostic.code == Some(kerf_span::DiagnosticCode(2)),
        "Expand 错误必须编号为 E0002"
    );
}

/// 模块循环依赖（§7.1.1 类 6——原零测试分类）：
/// A imports B + B imports A → `visit` 报结构化 Err「模块循环依赖」
/// （修复前：无限递归栈溢出 abort——phase.rs DFS 灰标记修复后激活）。
#[test]
fn module_cycle_dependency_errors() {
    let mut reg = ModuleRegistry::new();
    let a = Symbol(1);
    let b = Symbol(2);
    reg.declare(a, vec![b], vec![]).unwrap();
    reg.declare(b, vec![a], vec![]).unwrap();
    let err = reg.visit(a).unwrap_err();
    assert!(err.contains("模块循环依赖"), "应报循环依赖：{}", err);
}
