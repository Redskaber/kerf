//! 类型检查器集成测试（tests/v0/stage1/plan/typecheck_tests.rs ↔
//! kerf-compiler/src/typecheck.rs ↔ 12-roadmap §2.5.2 Stage 1 L 节点）。
//!
//! 批次 C（25-a/25-c）：保守静态类型检查 R1-R8 规则集 + 多错误收集
//! （TD-013 消费面）+ check_source 管线集成。
//!
//! 验收锚（§9.4.3）：
//! - **保守性零误报**：examples/ 全部程序 + 全部既有语义边界程序 →
//!   0 诊断（静态检查只报告「运行期必然失败」的程序——动态信息不足
//!   一律 Unknown 跳过）；
//! - 每规则负例 ≥3（表驱动矩阵）；
//! - 静态诊断与运行时行为**一致性**抽样：静态报错程序实跑确实报
//!   Run 阶段错（双向锚定——防「静态误报」与「静态漏报失真」）；
//! - 多错误收集：单程序 N 错全量按 Span 次序返回。

#[path = "../../../common/mod.rs"]
mod common;

use kerf_driver::{check_source, run_source, Stage};

/// 断言源文本静态检查 = 0 诊断（保守性锚）。
fn expect_clean(src: &str) {
    let report = check_source(src, "typecheck-pos.krf").expect("编译失败");
    assert!(
        report.diagnostics.is_empty(),
        "预期 0 诊断，实际 {} 条：{}\n程序：{}",
        report.diagnostics.len(),
        report.rendered.join("\n"),
        src
    );
}

/// 断言源文本产生含指定消息子串的静态诊断（E0005）。
fn expect_diag(src: &str, msg: &str) {
    let report = check_source(src, "typecheck-neg.krf").expect("编译失败");
    assert!(
        report.diagnostics.iter().any(|d| d.message.contains(msg)),
        "未检出诊断「{}」；实际 {} 条：{}\n程序：{}",
        msg,
        report.diagnostics.len(),
        report.rendered.join("\n"),
        src
    );
    for d in &report.diagnostics {
        assert_eq!(
            d.code.map(|c| c.render()),
            Some("E0005".to_string()),
            "诊断码必须为 E0005"
        );
    }
}

/// 断言源文本静态诊断条数 = N（多错误收集锚）。
fn expect_diag_count(src: &str, n: usize) {
    let report = check_source(src, "typecheck-multi.krf").expect("编译失败");
    assert_eq!(
        report.diagnostics.len(),
        n,
        "诊断数不符：{}\n程序：{}",
        report.rendered.join("\n"),
        src
    );
}

/// 断言静态报错程序在运行期同样失败（保守性反向锚——静态确定性验证）。
fn static_error_is_runtime_error(src: &str) {
    match run_source(src, "typecheck-rt.krf") {
        Ok(_) => panic!("静态报错程序运行期通过（静态误报嫌疑）：{}", src),
        Err(e) => assert_eq!(e.stage, Stage::Run, "运行期错误阶段：{}", e),
    }
}

// ---------------------------------------------------------------------------
// 保守性（零误报）——R1-R8 的「不误报」面
// ---------------------------------------------------------------------------

/// examples/ 全部示例程序零诊断（6 case——真实程序回归锚）。
#[test]
fn examples_all_clean() {
    for name in [
        "fib.krf",
        "closures.krf",
        "macros.krf",
        "higher_order.krf",
        "gc_stress.krf",
        "io.krf",
    ] {
        let path = format!("examples/usage/{}", name);
        let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}", e));
        expect_clean(&src);
    }
}

/// 动态语义边界程序零诊断（参数化值路径不触发静态断言）。
#[test]
fn dynamic_programs_clean() {
    // 参数传递（值类型不可知）——保守 Unknown
    expect_clean("(define (f x) (if x 1 2)) (f true)");
    // 递归定义自引用（定义前引用 → Unknown）
    expect_clean("(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 5)");
    // 宏引入的卫生符号（$hyg$N 后缀——运行期卫生回退解析）
    expect_clean("(define x 42) (print x)");
    // 数值塔混合（Int/Float 并集 = Num 合法域）
    expect_clean("(+ 1 2.5)");
    expect_clean("(< 1 2.0)");
    // 引用列表的 car（quote 字面量 → Pair）
    expect_clean("(car '(1 2 3))");
    // 内置遮蔽（用户定义遮蔽内置——按用户类型走）
    expect_clean("(define (car x) 42) (car 5)");
    // set!/begin/if 分支混合
    expect_clean("(define x 1) (set! x (+ x 1)) (begin (print x) (if (null? nil) x 2))");
    // 谓词结果作 if 条件（结果类型 Bool 推断）
    expect_clean("(if (null? nil) 1 2)");
    expect_clean("(if (eq? 'a 'a) 1 2)");
    // 字符串族正确用法
    expect_clean("(str-append \"a\" \"b\")");
    expect_clean("(string->symbol \"foo\")");
    expect_clean("(symbol->string 'foo)");
    // 全字符串等值比较（= 的字符串相等）
    expect_clean("(= \"a\" \"b\")");
}

/// 深嵌套（Reader 限内最大深度）零诊断 + 栈安全（深度预算防 />
/// MAX_CHECK_DEPTH 的程序化构造面由 typecheck.rs 单测覆盖——Reader
/// 上限 256 使源文本不可达预算阈值，此处验证限内深嵌套零误报）。
#[test]
fn deep_nesting_within_reader_limit_clean() {
    let mut src = String::from("1");
    for _ in 0..250 {
        src = format!("(if true {} 2)", src);
    }
    expect_clean(&src);
}

// ---------------------------------------------------------------------------
// R1：if 条件静态已知非 bool
// ---------------------------------------------------------------------------

/// R1 负向矩阵（8 case：int/float/str/nil/symbol/pair/quote-pair/procedure）。
#[test]
fn r1_if_condition_non_bool() {
    expect_diag("(if 1 2 3)", "if 条件需要 bool，实际 int");
    expect_diag("(if 1.5 2 3)", "if 条件需要 bool，实际 float");
    expect_diag("(if \"s\" 2 3)", "if 条件需要 bool，实际 str");
    expect_diag("(if nil 2 3)", "if 条件需要 bool，实际 nil");
    expect_diag("(if 'sym 2 3)", "if 条件需要 bool，实际 symbol");
    expect_diag("(if (cons 1 2) 2 3)", "if 条件需要 bool，实际 pair");
    expect_diag("(if '(1) 2 3)", "if 条件需要 bool，实际 pair");
    expect_diag(
        "(if (lambda (x) x) 2 3)",
        "if 条件需要 bool，实际 procedure",
    );
    // 静态确定性反向锚：运行期确实失败
    static_error_is_runtime_error("(if 1 2 3)");
    static_error_is_runtime_error("(if \"s\" 2 3)");
}

/// R1 嵌套位置（嵌套 if 的条件同样检查——非仅顶层）。
#[test]
fn r1_nested_if_condition() {
    expect_diag("(if true 1 (if nil 2 3))", "if 条件需要 bool，实际 nil");
}

// ---------------------------------------------------------------------------
// R2：算术族操作数静态已知非数值
// ---------------------------------------------------------------------------

/// R2 负向矩阵（5 算子 × 非数值变体）。
#[test]
fn r2_arithmetic_non_numeric() {
    let ops = ["+", "-", "*", "/", "mod"];
    for op in ops {
        expect_diag(
            &format!("({} 1 \"a\")", op),
            &format!("{} 需要数值，实际 str", op),
        );
    }
    // 左位置非数值
    expect_diag("(+ \"a\" 1)", "+ 需要数值，实际 str");
    expect_diag("(* 'sym 2)", "* 需要数值，实际 symbol");
    expect_diag("(/ nil 2)", "/ 需要数值，实际 nil");
    expect_diag("(mod true 2)", "mod 需要数值，实际 bool");
    expect_diag("(+ (cons 1 2) 5)", "+ 需要数值，实际 pair");
    // 深位（链中段）
    expect_diag("(+ 1 2 \"a\" 4)", "+ 需要数值，实际 str");
    // 静态确定性反向锚
    static_error_is_runtime_error("(+ 1 \"a\")");
}

/// R2 数值塔边界：Int/Float 混合不误报（Num 并集域）。
#[test]
fn r2_numeric_tower_clean() {
    expect_clean("(+ 1 2.5 3)");
    expect_clean("(- 2.5 1)");
    expect_clean("(* 1.5 2.0 3)");
}

// ---------------------------------------------------------------------------
// R3：比较族（= 与排序）——TD-016 全操作数静态口径
// ---------------------------------------------------------------------------

/// R3 排序族负向（4 算子 × 非数值）。
#[test]
fn r3_ordering_non_numeric() {
    for op in ["<", ">", "<=", ">="] {
        expect_diag(
            &format!("({} 1 \"a\")", op),
            &format!("{} 需要数值，实际 str", op),
        );
    }
    // TD-016 静态面：短路后操作数同样检出（< 3 1 "a" 运行期已收紧）
    expect_diag("(< 3 1 \"a\")", "< 需要数值，实际 str");
    expect_diag("(= 1 2 \"s\")", "= 需要数值，实际 str");
    // 全字符串排序 → TD-011 边界消息（静态对齐）
    expect_diag(
        "(< \"a\" \"b\")",
        "字符串仅支持 = 比较（Stage 0 边界，TD-011）",
    );
    // 静态确定性反向锚
    static_error_is_runtime_error("(< 3 1 \"a\")");
    static_error_is_runtime_error("(< \"a\" \"b\")");
}

/// R3 等值族全字符串合法（零误报锚）+ 混串检出。
#[test]
fn r3_equality_string_domain() {
    expect_clean("(= \"a\" \"b\" \"c\")");
    expect_diag("(= \"a\" 1)", "= 需要数值，实际 str");
    expect_diag("(= 1 2 'sym)", "= 需要数值，实际 symbol");
}

// ---------------------------------------------------------------------------
// R4/R5：not 与 car/cdr
// ---------------------------------------------------------------------------

/// R4 not 非布尔（3 case + 反向锚）。
#[test]
fn r4_not_non_bool() {
    expect_diag("(not 1)", "not 需要 bool，实际 int");
    expect_diag("(not \"s\")", "not 需要 bool，实际 str");
    expect_diag("(not nil)", "not 需要 bool，实际 nil");
    static_error_is_runtime_error("(not 1)");
}

/// R5 car/cdr 非序对（6 case + 反向锚）。
#[test]
fn r5_car_cdr_non_pair() {
    expect_diag("(car 5)", "car 需要 pair，实际 int");
    expect_diag("(cdr 5)", "cdr 需要 pair，实际 int");
    expect_diag("(car \"s\")", "car 需要 pair，实际 str");
    expect_diag("(car 'sym)", "car 需要 pair，实际 symbol");
    expect_diag("(cdr true)", "cdr 需要 pair，实际 bool");
    expect_diag("(car nil)", "car 需要 pair，实际 nil");
    // 引用点对字面量合法（零误报锚）
    expect_clean("(car '(1 2))");
    expect_clean("(cdr '(1 2))");
    static_error_is_runtime_error("(car 5)");
}

// ---------------------------------------------------------------------------
// R6：被调表达式静态已知不可调用
// ---------------------------------------------------------------------------

/// R6 不可调用（4 case + 反向锚）。
#[test]
fn r6_not_callable() {
    expect_diag("(1 2)", "不可调用的值：int");
    expect_diag("(\"s\" 1)", "不可调用的值：str");
    expect_diag("((cons 1 2) 3)", "不可调用的值：pair");
    expect_diag("(true 1)", "不可调用的值：bool");
    // 静态确定性反向锚
    static_error_is_runtime_error("(1 2)");
}

// ---------------------------------------------------------------------------
// R7：元数不匹配（字面量 lambda / 内置签名）
// ---------------------------------------------------------------------------

/// R7 字面量 lambda 元数（3 case）。
#[test]
fn r7_lambda_arity() {
    expect_diag("((lambda (x) x) 1 2)", "过程参数数量不匹配：期望 1 实际 2");
    expect_diag("((lambda (x y) x) 1)", "过程参数数量不匹配：期望 2 实际 1");
    expect_diag("((lambda () 1) 5)", "过程参数数量不匹配：期望 0 实际 1");
    static_error_is_runtime_error("((lambda (x) x) 1 2)");
}

/// R7 内置元数（5 case——与运行时元数守卫口径一致）。
#[test]
fn r7_builtin_arity() {
    expect_diag("(car 1 2)", "car 需要 1 个参数，实际 2");
    expect_diag("(cons 1)", "cons 需要 2 个参数，实际 1");
    expect_diag("(not 1 2)", "not 需要 1 个参数，实际 2");
    expect_diag("(str-append \"a\")", "str-append 需要 2 个参数，实际 1");
    expect_diag("(< 1)", "< 至少需要 2 个参数，实际 1");
    static_error_is_runtime_error("(car 1 2)");
}

/// R7 用户定义闭包元数（define → Callable 推断）。
#[test]
fn r7_defined_closure_arity() {
    expect_diag(
        "(define (f x) x) (f 1 2)",
        "过程参数数量不匹配：期望 1 实际 2",
    );
    // 参数化调用不误报（f 经参数传递 → Unknown callable）
    expect_clean("(define (f x) x) (define (g f) (f 1 2)) (g f)");
}

// ---------------------------------------------------------------------------
// R8：字符串/符号族操作数类型不符
// ---------------------------------------------------------------------------

/// R8 字符串族（4 case + 反向锚）。
#[test]
fn r8_string_family_type() {
    expect_diag("(str-length 5)", "str-length 需要 str，实际 int");
    expect_diag("(str-upcase 'sym)", "str-upcase 需要 str，实际 symbol");
    expect_diag(
        "(str-contains? 1 \"a\")",
        "str-contains? 需要 str，实际 int",
    );
    expect_diag(
        "(string->symbol true)",
        "string->symbol 需要 str，实际 bool",
    );
    static_error_is_runtime_error("(str-length 5)");
}

/// R8 符号转换（2 case）。
#[test]
fn r8_symbol_conversion() {
    expect_diag("(symbol->string 5)", "symbol->string 需要 symbol，实际 int");
    expect_diag(
        "(symbol->string \"s\")",
        "symbol->string 需要 symbol，实际 str",
    );
    static_error_is_runtime_error("(symbol->string 5)");
}

// ---------------------------------------------------------------------------
// 多错误收集（TD-013 消费面）
// ---------------------------------------------------------------------------

/// 单程序多错误全量收集 + Span 次序（TD-013 设计的实证消费）。
#[test]
fn multi_error_collection_span_order() {
    // 三个独立错误（if/算术/不可调用）全量返回
    let report =
        check_source("(if 1 2 3) (+ 1 \"a\") (5 6)", "typecheck-multi.krf").expect("编译失败");
    assert_eq!(
        report.diagnostics.len(),
        3,
        "{}",
        report.rendered.join("\n")
    );
    // Span 次序（源码先后）
    let starts: Vec<u32> = report
        .diagnostics
        .iter()
        .map(|d| d.primary_span.start)
        .collect();
    let mut sorted = starts.clone();
    sorted.sort();
    assert_eq!(starts, sorted, "诊断必须按 Span 起始偏移排序");
    // 渲染含位置与 E0005
    assert!(report.rendered[0].contains("error[E0005]"));
    assert!(report.rendered[0].contains("-->"));
}

/// 同一表达式内的多错误（错误后继续检查后续参数——非短路）。
#[test]
fn multi_error_continues_after_error() {
    // (+ "a" "b")：两个非数值参数均检出（首个错误后不终止）
    expect_diag_count("(+ \"a\" \"b\")", 2);
    // if 分支中的错误照常收集
    expect_diag_count("(if true (+ 1 \"a\") (+ 2 \"b\"))", 2);
}

/// 多形式程序：逐形式收集（define 后续形式仍检查）。
#[test]
fn multi_error_across_forms() {
    expect_diag_count("(define x 1) (if nil 2 3) (+ x \"a\") (car 5)", 3);
}

// ---------------------------------------------------------------------------
// 管线集成（check_source 与执行路径）
// ---------------------------------------------------------------------------

/// check_source 编译错误透传（Read/Expand 阶段错误仍以 DriverError 返回）。
#[test]
fn check_source_compile_error_passthrough() {
    let err = check_source("(+ 1", "typecheck-err.krf").unwrap_err();
    assert_eq!(err.stage, Stage::Read);
    let err = check_source("(if)", "typecheck-err.krf").unwrap_err();
    assert_eq!(err.stage, Stage::Expand);
}

/// check_source 编译统计面（原型/指令计数——check 子命令输出数据源）。
#[test]
fn check_report_stats_present() {
    let report = check_source("(+ 1 2)", "typecheck-stats.krf").expect("编译失败");
    assert!(report.proto_count >= 1);
    assert!(report.instruction_count >= 3);
    assert!(report.rendered.is_empty());
}

/// 双路径一致性：静态诊断不改变 run/eval 行为（检查是旁路报告——
/// 编译产物不受检查影响）。
#[test]
fn typecheck_does_not_affect_execution() {
    let src = "(define (f x) (+ x 1)) (f 41)";
    expect_clean(src);
    assert!(common::dual_path_agrees(src));
    assert_eq!(common::run_rendered(src), "42");
}

/// 深嵌套预算下的栈安全（Reader 限内 250 层——TD-017 2 MiB 栈口径）。
#[test]
fn deep_nesting_stack_safe() {
    let mut src = String::from("1");
    for _ in 0..250 {
        src = format!("(if true {} 2)", src);
    }
    // 不崩溃不诊断
    let _ = check_source(&src, "typecheck-deep.krf");
}
