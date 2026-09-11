//! 负向语义测试（tests/v0/stage0/plan/negative_semantics_tests.rs ↔
//! docs/tests/v0/stage0/plan/negative-tests.md）。
//!
//! §9.4.3 定向扩张：本文件锚定 **06-operational-semantics** 的错误语义
//! 层——E0-E8 错误码矩阵、R 规则错误路径、T1 定理双路径一致性回归
//! （Task 12/16 修复面）、错误消息形状（码+Span+摘录+调用点追踪）。
//! 每行/每次迭代 = 1 case（表格与矩阵驱动）。
//!
//! 全部期望消息经实测校准（2026-09-10，修复后基线）。
//!
//! 语义边界（实测确认）：
//! - E7（I/O 错误）经公开 API 不可触发（read-line EOF → nil 为正常
//!   语义；write 失败需关闭 stdout，CLI 场景不可构造）——文档说明，
//!   不计 case；
//! - E8（内部不变式：栈下溢/pc 越界/字节码损坏）非用户程序可触发
//!   （编译器保证不变式）——文档说明，不计 case；
//! - E0003（Compile 阶段）同样非用户程序可触发（编译错误全部为
//!   内部不变式防御）——文档说明，不计 case；
//! - 嵌套 define 重复在**展开期**被体内部提升机制拒绝（TD-014 r24
//!   修正归因：专门消息「嵌套 define 重复绑定」+ Span 指向第二次出现
//!   处——原「lambda 参数重名」归因失真已消除），语义上等价 E6 的
//!   提前防御（§2.3-4 报错>静默）；
//! - `(define car 5)` 影子化内置全局 → E6（内置名占据全局层，
//!   同层重复定义约束对用户/内置统一生效）。
//!
//! 双路径消息面（TD-018 r24 统一后）：
//! - 未绑定变量：VM「未绑定的全局变量（…）」vs eval「未绑定变量」
//!   ——保留（阶段信息差异：VM 侧携带全局兜底解析完成度）；
//! - if 非布尔：VM 与 eval 同文「if 条件需要 bool」（messages.rs 单源）。

use crate::common;

use common::dual_path_agrees;
use kerf_driver::{run_source, run_source_seed, Stage};
use kerf_vm::Value;

/// 通用断言：src 失败于 Run 阶段（E0004 载体），含消息子串。
fn expect_run_err(src: &str, msg: &str) -> kerf_driver::DriverError {
    let err = match run_source(src, "neg.krf") {
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
    err
}

/// 双路径 Err 事实断言：生产链与种子链都必须失败（消息文本允许分裂，
/// 但失败事实与阶段必须一致——T1 的错误吸收形态；42-d 新口径：
/// 种子链替代 eval 为对拍 oracle）。
fn expect_dual_err(src: &str) {
    assert!(run_source(src, "neg.krf").is_err(), "生产链应报错：{}", src);
    assert!(
        run_source_seed(src, "neg.krf").is_err(),
        "种子链应报错：{}",
        src
    );
}

// ---------------------------------------------------------------------------
// E1 TypeMismatch：truthy 语义（§19.5 陷阱 3：仅 Bool 参与）
// ---------------------------------------------------------------------------

/// E1（06 §3）：if 条件位置仅接受 Bool——7 种非布尔类型（7 case）。
#[test]
fn e1_truthy_requires_bool_all_types() {
    let bad = [
        "1",
        "2.5",
        "\"s\"",
        "nil",
        "(cons 1 2)",
        "(lambda (x) x)",
        "(quote (1))",
    ];
    for b in bad {
        expect_run_err(&format!("(if {} 1 2)", b), "需要 bool");
    }
}

/// E1：truthy 严格性在嵌套位置同样生效（3 case：then/else 分支内的
/// if 条件、begin 内 if、lambda 体内 if）。
#[test]
fn e1_truthy_nested_positions() {
    expect_run_err("(if true (if 1 2 3) 4)", "需要 bool");
    expect_run_err("(begin 1 (if \"s\" 2 3))", "需要 bool");
    expect_run_err("((lambda () (if nil 1 2)))", "需要 bool");
}

// ---------------------------------------------------------------------------
// E2 ArityMismatch：过程元数（R8/apply 协议）
// ---------------------------------------------------------------------------

/// E2：lambda/define 糖/闭包捕获调用元数错误（6 case）。
#[test]
fn e2_arity_mismatch_matrix() {
    expect_run_err("((lambda (x) x) 1 2)", "过程参数数量不匹配：期望 1 实际 2");
    expect_run_err("((lambda (x y) x) 1)", "过程参数数量不匹配：期望 2 实际 1");
    expect_run_err("((lambda () 1) 9)", "过程参数数量不匹配：期望 0 实际 1");
    expect_run_err(
        "(define (f a b) a) (f 1)",
        "过程参数数量不匹配：期望 2 实际 1",
    );
    expect_run_err(
        "(define (make) (lambda (x) x)) ((make) 1 2 3)",
        "过程参数数量不匹配：期望 1 实际 3",
    );
    expect_run_err(
        "(define (outer x) (lambda (y) x)) ((outer 1) 2 3)",
        "过程参数数量不匹配：期望 1 实际 2",
    );
}

/// E2 双路径：元数错误双路径一致报错（Err 事实，4 case）。
#[test]
fn e2_arity_dual_path_errors() {
    expect_dual_err("((lambda (x) x) 1 2)");
    expect_dual_err("((lambda (x y) x) 1)");
    expect_dual_err("(define (f) 1) (f 2)");
    expect_dual_err("(define (f a b) (+ a b)) (f 1)");
}

// ---------------------------------------------------------------------------
// E3 UnboundVariable：未绑定标识符（R5/S1）
// ---------------------------------------------------------------------------

/// E3：未绑定在全部求值上下文（10 case）。
#[test]
fn e3_unbound_all_contexts() {
    expect_run_err("undefined_x", "未绑定");
    expect_run_err("(define (f) undefined_x) (f)", "未绑定");
    expect_run_err("(define x undefined_y)", "未绑定");
    expect_run_err("(if undefined_c 1 2)", "未绑定");
    expect_run_err("(begin undefined_b 2)", "未绑定");
    expect_run_err("(set! undefined_s 1)", "set! 未绑定变量");
    expect_run_err("(define (g) (set! undefined_h 1)) (g)", "set! 未绑定变量");
    expect_run_err("(+ 1 undefined_n)", "未绑定");
    expect_run_err("(car undefined_p)", "未绑定");
    expect_run_err("((lambda (x) undefined_q) 1)", "未绑定");
}

/// E3：宏引入标识符引用未定义全局——卫生回退只覆盖内置基名，
/// 未定义名两路径均报错（2 case；宏展开 Span 携带 expansion 标记）。
#[test]
fn e3_macro_introduced_unbound() {
    let src = "(define-syntax m (syntax-rules () ((m) undefined_thing))) (m)";
    expect_run_err(src, "未绑定");
    expect_dual_err(src);
}

/// E3 消息形状：错误携带宏展开 Span（`expansion` 标记——Span 系统的
/// 相位可观测性，§19.1）（1 case）。
#[test]
fn e3_macro_error_carries_expansion_mark() {
    let src = "(define-syntax m (syntax-rules () ((m) undefined_thing))) (m)";
    let err = expect_run_err(src, "未绑定");
    assert!(
        err.rendered.contains("expansion 1"),
        "宏引入引用的未绑定错误应携带展开相位标记：\n{}",
        err.rendered
    );
}

// ---------------------------------------------------------------------------
// E4 NotCallable：不可调用的值（A2）
// ---------------------------------------------------------------------------

/// E4：7 种非过程值占据调用位（7 case）。
#[test]
fn e4_not_callable_all_types() {
    let bad = [
        "1",
        "2.5",
        "\"s\"",
        "true",
        "nil",
        "(cons 1 2)",
        "(quote (1 2))",
    ];
    for b in bad {
        expect_run_err(&format!("({} 1)", b), "不可调用");
    }
}

/// E4：调用结果（非过程）再调用 + 嵌套调用位（3 case）。
#[test]
fn e4_not_callable_computed_callee() {
    expect_run_err("((if true 1 2) 3)", "不可调用");
    expect_run_err("((car (cons 5 6)) 7)", "不可调用");
    expect_run_err("(define (f) 5) ((f) 1)", "不可调用");
}

// ---------------------------------------------------------------------------
// E5 BuiltinError：内置函数错误
// ---------------------------------------------------------------------------

/// E5：内置错误族（5 case：整数除零/取模零/car/cdr 非序对/str-append）。
#[test]
fn e5_builtin_error_matrix() {
    expect_run_err("(/ 1 0)", "整数除零");
    expect_run_err("(mod 5 0)", "整数取模除零");
    expect_run_err("(car 5)", "car 需要 pair");
    expect_run_err("(cdr \"s\")", "cdr 需要 pair");
    expect_run_err("(str-append 1 \"a\")", "str-append 需要");
}

// ---------------------------------------------------------------------------
// E6 DuplicateDefine：同层重复定义（D1——Task 12 修复面）
// ---------------------------------------------------------------------------

/// E6：重复定义矩阵（9 case：顶层/糖形式/内置影子/nested 提前防御/
/// begin 内/set! 后重复/不同值形态重复/define 后再糖定义）。
#[test]
fn e6_duplicate_define_matrix() {
    expect_run_err("(define x 1) (define x 2)", "重复定义变量");
    expect_run_err("(define x 1) (define x \"s\")", "重复定义变量");
    expect_run_err("(define (f) 1) (define f 2)", "重复定义变量");
    expect_run_err("(define car 5)", "重复定义变量"); // 内置名占全局层
    expect_run_err("(define cdr 5)", "重复定义变量");
    expect_run_err("(define n 1) (set! n 5) (define n 2)", "重复定义变量");
    expect_run_err("(begin (define b 1) (define b 2))", "重复定义变量");
    // 嵌套重复：直接体 define 提升后展开期拒绝（TD-014 r24：专门消息
    // 归因到「嵌套 define 重复绑定」——原「lambda 参数重名」失真已消除）；
    // begin 包裹的 define：D1 修复后编译期结构化拒绝（修复前 VM 全局
    // 泄漏 vs eval 词法定义——T1 反例；两路径共享 compile_source 同报 E0003）
    let lifted =
        run_source("(define (f) (define x 1) (define x 2) x) (f)", "neg.krf").expect_err("应报错");
    assert_eq!(
        lifted.stage,
        Stage::Expand,
        "直接体 define 重复在展开期拒绝"
    );
    assert!(lifted.rendered.contains("嵌套 define 重复绑定"));
    let begin_wrapped = run_source(
        "(define (f) (begin (define y 1) (define y 2))) (f)",
        "neg.krf",
    )
    .expect_err("begin 包裹 define 应被编译期拒绝（D1）");
    assert_eq!(begin_wrapped.stage, Stage::Compile, "D1：编译期拒绝");
    assert!(
        begin_wrapped
            .rendered
            .contains("define 仅允许出现在顶层或函数体头部"),
        "D1 消息：\n{}",
        begin_wrapped.rendered
    );
    assert!(
        begin_wrapped.rendered.contains("error[E0003]"),
        "D1 产生首个用户可触发 E0003：\n{}",
        begin_wrapped.rendered
    );
}

/// E6 双路径回归（Task 12 修复面）：重复定义双路径一致报错（2 case）。
#[test]
fn e6_duplicate_define_dual_path() {
    expect_dual_err("(define x 1) (define x 2)");
    expect_dual_err("(define (f) 1) (define f 2)");
}

// ---------------------------------------------------------------------------
// E 码载体与消息形状（§8.7 结构化诊断：码 + Span + 摘录 + 调用点）
// ---------------------------------------------------------------------------

/// 消息形状：阶段码 E0001/E0002/E0004 + `-->` Span 行 + 源摘录栏（8 case）。
#[test]
fn message_shape_stage_codes_and_span() {
    let cases = [
        ("\"unclosed", Stage::Read, "E0001"), // 词法：未闭合字符串
        ("(define", Stage::Read, "E0001"),    // 语法：括号未闭合（Reader）
        ("(set! )", Stage::Expand, "E0002"),  // 展开：set! 空形式
        ("(lambda (x x) x)", Stage::Expand, "E0002"), // 形参重名（A3）
        ("(undefined_x)", Stage::Run, "E0004"), // 运行：未绑定
        ("(define x 1) (define x 2)", Stage::Run, "E0004"), // E6
        ("(+ 1 \"s\")", Stage::Run, "E0004"), // E5/E1 载体
        ("(1 2)", Stage::Run, "E0004"),       // E4
    ];
    for (src, stage, code) in cases {
        let err = run_source(src, "shape.krf")
            .err()
            .unwrap_or_else(|| panic!("期望报错，实际 Ok：{}", src));
        assert_eq!(err.stage, stage, "阶段不符：{}", src);
        assert!(
            err.rendered.contains(&format!("error[{}]", code)),
            "[{}] 错误应渲染码 {}：\n{}",
            src,
            code,
            err.rendered
        );
        assert!(err.rendered.contains("-->"), "应含 Span 指示行：{}", src);
        assert!(err.rendered.contains("|"), "应含源摘录栏：{}", src);
        assert!(err.rendered.contains("^"), "应含下划标记：{}", src);
    }
}

/// 消息形状：调用点追踪（§8.12——帧链渲染为 note 行；6 case）。
#[test]
fn message_shape_call_site_trace() {
    // 顶层错误（无调用帧）→ 无 note
    let top = run_source("(+ 1 \"s\")", "t.krf").err().unwrap();
    assert!(
        !top.rendered.contains("调用点"),
        "顶层错误不应有调用点 note：\n{}",
        top.rendered
    );
    // 单层调用：错误在 f 体内 → 1 个调用点 note
    let one = run_source("(define (f n) (+ n \"s\")) (f 1)", "t.krf")
        .err()
        .unwrap();
    assert!(
        one.rendered.contains("note: 调用点"),
        "函数内错误应含调用点 note：\n{}",
        one.rendered
    );
    // 双层调用链：g→f，错误在 f 体内 → 2 个调用点 note。
    // （H2/TCO 注记：`(+ 0 (f 5))` 使调用非尾位——尾调用经帧复用会从
    // 追踪链消失（优化帧不出栈迹——GCC/clang -O2 同行为）；本测试
    // 验证的是多帧追踪渲染面，故走非尾形态。）
    let two = run_source(
        "(define (f n) (car n)) (define (g) (+ 0 (f 5))) (g)",
        "t.krf",
    )
    .err()
    .unwrap();
    let notes = two.rendered.matches("note: 调用点").count();
    assert!(
        notes >= 2,
        "双层调用链应含 ≥2 调用点 note（实际 {}）：\n{}",
        notes,
        two.rendered
    );
    // 深递归（**非尾位**）：帧上限错误 + 追踪截断（≤16 帧渲染，防诊断爆炸）。
    // H2/TCO 注记：旧用例 `(f n)` 自尾调无限循环经 TCO 帧复用不再耗帧——
    // 改由指令预算护栏接管（tco_tests 的 instruction_budget 负例锚定）；
    // 本块保持「帧上限 + 截断渲染」验证意图，走非尾位形态。
    let deep = run_source("(define (f n) (+ 1 (f n))) (f 1)", "t.krf")
        .err()
        .unwrap();
    assert!(
        deep.rendered.contains("调用帧超过上限 100000"),
        "失控递归应报帧上限：\n{}",
        deep.rendered
    );
    let deep_notes = deep.rendered.matches("note: 调用点").count();
    assert!(
        deep_notes <= 16,
        "深递归追踪应截断至 ≤16 帧（实际 {}）",
        deep_notes
    );
    assert!(deep_notes > 0, "帧上限错误应至少含 1 个调用点");
    // set! 未绑定 + 调用点
    let setb = run_source("(define (g) (set! y 1)) (g)", "t.krf")
        .err()
        .unwrap();
    assert!(setb.rendered.contains("note: 调用点"));
    // 宏引入错误：expansion 相位标记
    let hyg = run_source(
        "(define-syntax m (syntax-rules () ((m) undefined_thing))) (m)",
        "t.krf",
    )
    .err()
    .unwrap();
    assert!(hyg.rendered.contains("expansion 1"));
}

// ---------------------------------------------------------------------------
// T1 定理双路径一致性回归（Task 12 + Task 16 修复面）
// ---------------------------------------------------------------------------

/// T1 回归：全局存储语义修复面（4 case——修复前 VM/eval 分裂）。
#[test]
fn t1_regression_global_storage_semantics() {
    // Define 返回值（D1：修复前 VM nil / eval v）
    assert!(dual_path_agrees("(define x 5)"));
    assert!(dual_path_agrees("(define x (+ 2 3)) x"));
    // 重复定义 E6（修复前双路径均静默覆盖）
    assert!(dual_path_agrees("(define x 1) (define x 2)"));
    // set! 未绑定 E3（修复前 VM 静默创建全局）
    assert!(dual_path_agrees("(set! y 1)"));
}

/// T1 回归：App 求值顺序（06 §2 A1 函数先——Task 16 修复面，5 case）。
#[test]
fn t1_regression_app_evaluation_order() {
    // fn 位未绑定先于参数位报错（span 判定见 vm_tests 同名测试）
    let src = "((undefined-a) undefined-b)";
    let vm = run_source(src, "o.krf").expect_err("生产链应报错");
    let ev = run_source_seed(src, "o.krf").expect_err("种子链应报错");
    assert!(vm.diagnostic.primary_span.start < 16, "生产链应报 fn 位");
    assert!(ev.diagnostic.primary_span.start < 16, "种子链应报 fn 位");
    // fn 位类型错误先于参数位（fn 表达式自身报错）
    expect_dual_err("((car 1) undefined-b)");
    let vm2 = run_source("((car 1) undefined-b)", "o.krf").err().unwrap();
    assert!(
        vm2.rendered.contains("car 需要 pair"),
        "fn 位错误应先报：\n{}",
        vm2.rendered
    );
    // fn 合法 + 参数错误（顺序不影响结果，但两路径同报参数错）
    expect_dual_err("(car undefined-arg)");
    // fn 位不可调用（E4）先于参数求值
    expect_dual_err("((if true 5 6) undefined-x)");
}

/// T1 回归：卫生回退解析（eval 路径接线修复——Task 16 修复面，3 case）。
#[test]
fn t1_regression_hygiene_fallback_dual_path() {
    // 宏引入内置引用：基名命中 → 双路径同解（修复前 eval 报未绑定）
    let src = "(define-syntax m (syntax-rules () ((m) (+ 1 41)))) (m)";
    assert!(dual_path_agrees(src), "宏→内置引用双路径应一致解析为 42");
    // 宏引入未定义名：两路径同报未绑定（回退只覆盖内置基名；
    // 消息文本两侧不同——VM「未绑定的全局变量…」vs eval「未绑定变量」，
    // 故断言 Err 事实而非渲染一致）
    let src2 = "(define-syntax m (syntax-rules () ((m) undefined_thing))) (m)";
    expect_dual_err(src2);
    // 卫生宏主体：宏内外同名不串扰（swap 语义正例——负例文件中的
    // 唯一正向锚，保证卫生回退未破坏正常宏语义）
    assert!(dual_path_agrees(
        "(define-syntax swap! (syntax-rules () ((swap! x y) (let ((tmp x)) (set! x y) (set! y tmp)))))
         (define p 1) (define q 2) (swap! p q) (- p q)"
    ));
}

// ---------------------------------------------------------------------------
// 作用域与卫生负例（R7/R9 环境链）
// ---------------------------------------------------------------------------

/// 作用域负例：词法作用域/闭包捕获错误形态（5 case）。
#[test]
fn scope_closure_negatives() {
    // lambda 内 set! 未捕获的外层未定义变量
    expect_run_err("(define (g) (set! outer 1)) (g)", "set! 未绑定变量");
    // 闭包体内引用未绑定（捕获链查不到）
    expect_run_err("((lambda () nope))", "未绑定");
    // 形参遮蔽后引用外层同名的未绑定（词法作用域边界）
    expect_run_err("(define (f x) (+ x y)) (f 1)", "未绑定");
    // begin 内局部定义后引用另一未定义
    expect_run_err("(begin (define a 1) b)", "未绑定");
    // TD-002 解除后符号 datum 为合法符号值——负向锚点转为值消费端：
    // 符号值参与算术在两路径均报 E0004（双路径一致负例）
    let vm_err = run_source("(+ 'x 1)", "q.krf").expect_err("符号算术应报错（VM）");
    assert_eq!(vm_err.stage, Stage::Run);
    assert!(vm_err.rendered.contains("+ 需要 int"));
    let ev_err = run_source_seed("(+ 'x 1)", "q.krf").expect_err("符号算术应报错（种子链）");
    assert!(ev_err.rendered.contains("int"));
}

// ---------------------------------------------------------------------------
// 错误恢复语义（§7.3.1 D 桶——单错误短路语义的诚实归档）
// ---------------------------------------------------------------------------

/// 错误恢复：Stage 0 为单错误短路语义（首个错误终止管线；后续形式
/// 不再求值）——本组锚定该事实行为（4 case，多错误收集为 stage0.md
/// §8.7 的 Stage 1 演进项，TD-013）。
#[test]
fn error_recovery_single_error_semantics() {
    // begin 中段错误：后续形式不求值（错误即吸收——E0）
    let err = run_source("(begin 1 (car 5) 2)", "r.krf").err().unwrap();
    assert_eq!(err.stage, Stage::Run);
    assert!(err.rendered.contains("car 需要 pair"));
    // 首个错误优先（前错遮蔽后错）
    let first = run_source("(car 1) (cdr 2)", "r.krf").err().unwrap();
    assert!(
        first.rendered.contains("car 需要 pair"),
        "首错应先报：\n{}",
        first.rendered
    );
    // 展开期错误阻断编译（后续好形式不出现在诊断）
    let exp = run_source("(lambda (x x) x) (define good 1)", "r.krf")
        .err()
        .unwrap();
    assert_eq!(exp.stage, Stage::Expand);
    // 错误后独立程序不受影响（进程级恢复——审计 D 桶同款语义）
    let ok = run_source("(+ 40 2)", "r2.krf");
    assert!(matches!(ok, Ok(ref o) if matches!(o.value, Value::Int(42))));
}

/// 错误恢复：错误不 panic/不挂起——全部 E 码路径返回结构化 Err（4 case）。
#[test]
fn error_recovery_no_panics_structured() {
    for src in [
        "(car nil)",
        "(/ 1 0)",
        "(define x 1) (define x 1)",
        "((lambda (x) x) 1 2 3 4 5)",
    ] {
        // 返回 Err（而非 panic/abort）即为结构化恢复
        let r = run_source(src, "s.krf");
        assert!(r.is_err(), "应结构化报错：{}", src);
    }
}

// ---------------------------------------------------------------------------
// T17-a 对抗深挖回归（D2/D7——双路径健全性修复面）
// ---------------------------------------------------------------------------

/// D2 回归：eq? 字符串按内容比较（修复前 VM 常量池去重 → true /
/// eval 独立分配 → false——指针比较分裂，违反 T1 与「即时值按值」）。
#[test]
fn d2_eq_string_content_semantics() {
    // 同内容字面量：双路径均 true
    let src = "(eq? \"a\" \"a\")";
    let vm = common::run(src).expect("VM 应 Ok");
    assert!(matches!(vm, Value::Bool(true)), "VM 同内容应 true");
    let ev = kerf_driver::run_source_seed(src, "d2.krf").expect("种子链应 Ok");
    assert!(matches!(ev.value, Value::Bool(true)), "种子链同内容应 true");
    // 构造字符串 vs 字面量：内容相等 → true（双路径一致）
    let src2 = "(eq? \"ab\" (str-append \"a\" \"b\"))";
    assert!(
        common::dual_path_agrees(src2),
        "构造字符串内容比较双路径一致"
    );
    let v = common::run(src2).expect("应 Ok");
    assert!(matches!(v, Value::Bool(true)), "内容相等应 true");
    // 不同内容 → false（负例）
    let v2 = common::run("(eq? \"a\" \"b\")").expect("应 Ok");
    assert!(matches!(v2, Value::Bool(false)));
    // 堆值按引用（序对）语义不受影响
    let v3 = common::run("(eq? (cons 1 2) (cons 1 2))").expect("应 Ok");
    assert!(matches!(v3, Value::Bool(false)), "堆值（序对）仍按引用");
}

/// D7 回归：参考链错误保真（修复前 eval 逐层累积「求值失败：」前缀且
/// Span 落最外调用点；修复后消息原样、Span 定位最内错误位——42-d
/// 口径迁移：种子链对拍同判据）。
#[test]
fn d7_eval_error_fidelity() {
    // f→g→+ 类型错：种子链消息无「求值失败：」前缀（与生产链消息形态一致）
    let src = "(define (f n) (+ n \"s\")) (define (g) (f 1)) (g)";
    let ev = run_source_seed(src, "d7.krf").expect_err("种子链应报错");
    assert!(
        !ev.rendered.contains("求值失败"),
        "eval 错误不应累积包装前缀：\n{}",
        ev.rendered
    );
    assert!(
        ev.rendered.contains("+ 需要 int"),
        "错误消息应原样透传：\n{}",
        ev.rendered
    );
    // Span 保真：定位到最内错误位（+ 调用），而非最外 (g) 调用点
    // （源中 `(+ n \"s\")` 位于 1:15-26 区间，`(g)` 位于 1:43）
    assert!(
        ev.diagnostic.primary_span.start < 40,
        "eval 错误 Span 应定位最内错误位（实际 start={}）：\n{}",
        ev.diagnostic.primary_span.start,
        ev.rendered
    );
    // 双路径错误消息一致（D8 消息分裂面外的形态对齐验证）
    let vm = run_source(src, "d7.krf").expect_err("VM 应报错");
    assert!(
        vm.rendered.contains("+ 需要 int"),
        "VM 错误消息一致：\n{}",
        vm.rendered
    );
}

// D3/D5/D9 回归见 negative_vm_tests（深度上限结构化/单位元/一元取负）；
// D1/D6 回归见本文件 t1 回归段（编译器/展开器修复面）。

/// D6 回归：宏调宏（模板引入另一变换器引用经卫生 α 重命名后按基名
/// 回退解析——03 §2.4 卫生保证(2)「自由标识符穿透」的落地）。
/// 修复前：`inc$hyg$N` 两路径均未绑定报错（变换器表查不到重命名符号）。
#[test]
fn d6_macro_calls_macro_hygiene_fallback() {
    let src = "(define-syntax inc (syntax-rules () ((inc v) (set! v (+ v 1)))))
         (define-syntax twice! (syntax-rules () ((twice! e) (begin (inc e) (inc e)))))
         (define x 40) (twice! x) x";
    // 两路径一致 ⇒ 42
    let vm = common::run(src).expect("VM 路径应 Ok");
    assert!(matches!(vm, Value::Int(42)), "VM 宏调宏应解析：{:?}", vm);
    let ev = kerf_driver::run_source_seed(src, "d6.krf").expect("种子链应 Ok");
    assert!(
        matches!(ev.value, Value::Int(42)),
        "种子链宏调宏应解析（基名回退）：{:?}",
        ev.value
    );
    assert!(common::dual_path_agrees(src));
    // 负例：未定义宏名仍报错（回退只覆盖已注册变换器基名）
    expect_dual_err("(define-syntax m (syntax-rules () ((m) (no-such-macro)))) (m)");
}
