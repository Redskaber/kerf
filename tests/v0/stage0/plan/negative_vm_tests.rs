//! 负向 VM 测试（tests/v0/stage0/plan/negative_vm_tests.rs ↔
//! docs/tests/v0/stage0/plan/negative-tests.md）。
//!
//! §9.4.3 定向扩张：本文件覆盖 **Run 阶段（E0004）** 的 24 内置函数
//! 系统负测表（元数 × 类型规则 × 左右位置）+ 调用/求值错误 +
//! 双路径一致负例。每行/每次迭代 = 1 case（表格与矩阵驱动）。
//!
//! 全部期望消息经 builtins.rs 源码核对 + 实跑校准（禁止臆测）。
//!
//! 语义边界（实测确认，非负例）：
//! - 数值塔混合 `( < 1 2.0)` 合法（Int/Float 经 as_number 比较）；
//! - `(= 1 1.0)` 合法（数值塔相等）；`(= "a" "b")` 合法（字符串仅支持 =）；
//! - `(/ 1 0.0)` 合法（IEEE754 → inf；仅整数除零报 E5）；
//! - `(list)` 可变参（零参 → 空表，无元数错误）；
//! - 链式比较全操作数前置校验（TD-016 批次 C 收紧后语义，09-stdlib
//!   §2 v5.5）：比较链的**全部**操作数先做类型检查再逐对比较——`(< 3 1 "a")`
//!   报错（修复前 FS-5 边界为静默 false）；顺带收敛：`(< "a" "b" 1)`
//!   报「< 需要数值」（修复前报 TD-011 字符串消息）。
//!   静态检查面（kerf check）同步收敛——typecheck_tests R3。
//! - `read-line` 不校验元数（任意实参被忽略——见 FS-4）。
//!
//! 双路径消息分裂面（VM ≠ eval，仅断言 VM 侧，分裂记录于文档）：
//! - 未绑定变量：VM「未绑定的全局变量（…）」vs eval「未绑定变量」；
//! - if 非布尔：VM「条件位置需要 bool」vs eval「if 条件需要 bool」；
//! - 提升表达式错误：eval 多包一层「求值失败：」前缀。
//!
//! 已修复缺陷（原存档 #[ignore]，D3/D4 修复后激活）：
//! - `(mod -9223372036854775808 -1)`：checked_rem 结构化报错
//!   （`mod_i64_min_structured_error`）；
//! - 深递归 eval 路径：MAX_EVAL_DEPTH=4096 结构化报错
//!   （`deep_recursion_eval_path_structured_error`）。
//!
//! 已知边界（实测确认，#[ignore]，均不计数）：
//! - negative_vm_tests::read_line_arity_ignored（read-line 元数不校验
//!   ——语义发现 FS-4）。

use crate::common;

use kerf_driver::{run_source, Stage};
use kerf_vm::Value;

/// 通用断言：src 失败于 Run 阶段，渲染错误含消息子串。
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

// ---------------------------------------------------------------------------
// 算术（+ - * / mod）：类型规则 × 元数 × 除零 × 溢出
// ---------------------------------------------------------------------------

/// 非数值操作数 ×5 算子 × 左右位置 ×6 类型（60 case，整数路径）。
/// 消息形状：`{op} 需要 int`（builtins.rs 整数路径逐实参 as_int 校验）。
#[test]
fn arithmetic_non_numeric_operands() {
    let ops = ["+", "-", "*", "/", "mod"];
    let bad = [
        "\"s\"",
        "true",
        "nil",
        "(cons 1 2)",
        "(lambda (x) x)",
        "car",
    ];
    for op in ops {
        for b in bad {
            for b_on_right in [true, false] {
                let src = if b_on_right {
                    format!("({} 1 {})", op, b)
                } else {
                    format!("({} {} 1)", op, b)
                };
                expect_run_err(&src, &format!("{} 需要 int", op));
            }
        }
    }
}

/// 非数值操作数 × 浮点路径（20 case）：任一 Float → 数值塔路径，
/// 消息形状：`{op} 需要数值`（builtins.rs as_number 校验）。
#[test]
fn arithmetic_float_path_non_numeric() {
    let ops = ["+", "-", "*", "/", "mod"];
    let bad = ["\"s\"", "nil"];
    for op in ops {
        for b in bad {
            // 浮点在左
            expect_run_err(&format!("({} 1.5 {})", op, b), &format!("{} 需要数值", op));
            // 浮点在右
            expect_run_err(&format!("({} {} 1.5)", op, b), &format!("{} 需要数值", op));
        }
    }
}

/// mod 拒绝浮点操作数（2 case）——数值塔例外（整除语义约束）。
#[test]
fn modulo_rejects_float_operands() {
    expect_run_err("(mod 5.0 2)", "mod 不接受浮点操作数");
    expect_run_err("(mod 5 2.0)", "mod 不接受浮点操作数");
}

/// 算术元数与单位元边界（6 case，D5/D9 修复后语义）：
/// (+)→0、(*)→1（单位元，Ok）；- 空参报错、一元取负；/ 与 mod 至少 2。
#[test]
fn arithmetic_arity_floor() {
    expect_run_err("(-)", "- 至少需要 1 个参数");
    expect_run_err("(/)", "/ 至少需要 2 个参数");
    expect_run_err("(/ 5)", "/ 至少需要 2 个参数");
    expect_run_err("(mod)", "mod 至少需要 2 个参数");
    expect_run_err("(mod 5)", "mod 至少需要 2 个参数");
    // 单位元与一元取负（D5/D9 正向锚）
    let unit = common::run("(*)").expect("(*) 单位元");
    assert!(matches!(unit, kerf_vm::Value::Int(1)));
    let zero = common::run("(+)").expect("(+) 单位元");
    assert!(matches!(zero, kerf_vm::Value::Int(0)));
    let neg = common::run("(- 5)").expect("一元取负");
    assert!(matches!(neg, kerf_vm::Value::Int(-5)));
    let negf = common::run("(- 2.5)").expect("一元取负（浮点）");
    assert!(matches!(negf, kerf_vm::Value::Float(f) if f == -2.5));
}

/// 整数除零 / 取模零（3 case，E5 载体）。注：`(/ 1 0.0)` 合法（inf）。
#[test]
fn division_and_modulo_by_zero() {
    expect_run_err("(/ 1 0)", "整数除零");
    expect_run_err("(/ 0 0)", "整数除零");
    expect_run_err("(mod 5 0)", "整数取模除零");
}

/// 整数溢出（3 case，E5 载体，checked_* 路径）。
#[test]
fn integer_overflow_detected() {
    expect_run_err("(+ 9223372036854775807 1)", "整数加法溢出");
    expect_run_err("(- -9223372036854775808 1)", "整数减法溢出");
    expect_run_err("(* 9223372036854775807 2)", "整数乘法溢出");
}

// ---------------------------------------------------------------------------
// 比较（= < > <= >=）：类型规则 × 元数
// ---------------------------------------------------------------------------

/// 字符串序比较（4 case）：仅支持 =（Stage 0 边界，TD-011）。
#[test]
fn comparison_string_ordering_rejected() {
    let msg = "字符串仅支持 = 比较（Stage 0 边界，TD-011）";
    expect_run_err("(< \"a\" \"b\")", msg);
    expect_run_err("(> \"a\" \"b\")", msg);
    expect_run_err("(<= \"a\" \"b\")", msg);
    expect_run_err("(>= \"a\" \"b\")", msg);
}

/// 比较类型不匹配 ×5 算子 ×6 变体（30 case）：消息 `{op} 需要数值`。
/// 混合数值塔（(< 1 2.0)）合法，不在此列。
#[test]
fn comparison_type_mismatch() {
    let ops = ["=", "<", ">", "<=", ">="];
    let variants: &[(&str, &str)] = &[
        ("1", "\"s\""),      // int/str
        ("\"s\"", "1"),      // str/int
        ("nil", "1"),        // nil/int
        ("1", "nil"),        // int/nil
        ("true", "false"),   // bool/bool
        ("(cons 1 2)", "3"), // pair/int
    ];
    for op in ops {
        for (l, r) in variants {
            expect_run_err(
                &format!("({} {} {})", op, l, r),
                &format!("{} 需要数值", op),
            );
        }
    }
}

/// 比较元数（5 case）：链式比较至少 2 参。
#[test]
fn comparison_arity_floor() {
    expect_run_err("(< 1)", "< 至少需要 2 个参数");
    expect_run_err("(> 1)", "> 至少需要 2 个参数");
    expect_run_err("(<= 1)", "<= 至少需要 2 个参数");
    expect_run_err("(>= 1)", ">= 至少需要 2 个参数");
    expect_run_err("(= 1)", "= 至少需要 2 个参数");
}

// ---------------------------------------------------------------------------
// 序对（cons car cdr）
// ---------------------------------------------------------------------------

/// cons 元数（3 case）：恰好 2 参。
#[test]
fn cons_arity() {
    expect_run_err("(cons)", "cons 需要 2 个参数，实际 0");
    expect_run_err("(cons 1)", "cons 需要 2 个参数，实际 1");
    expect_run_err("(cons 1 2 3)", "cons 需要 2 个参数，实际 3");
}

/// car/cdr 元数（4 case）：恰好 1 参（元数检查先于类型检查）。
#[test]
fn car_cdr_arity() {
    expect_run_err("(car)", "car 需要 1 个参数，实际 0");
    expect_run_err("(cdr)", "cdr 需要 1 个参数，实际 0");
    expect_run_err("(car 5 6)", "car 需要 1 个参数，实际 2");
    expect_run_err("(cdr 5 6)", "cdr 需要 1 个参数，实际 2");
}

/// car/cdr 非序对 ×7 类型 ×2 算子（14 case）：消息含 type_name。
#[test]
fn car_cdr_non_pair_operands() {
    let bad = [
        ("5", "int"),
        ("1.5", "float"),
        ("\"s\"", "str"),
        ("true", "bool"),
        ("nil", "nil"),
        ("(lambda (x) x)", "procedure"),
        ("car", "builtin-procedure"),
    ];
    for op in ["car", "cdr"] {
        for (operand, type_name) in bad {
            let src = format!("({} {})", op, operand);
            expect_run_err(&src, &format!("{} 需要 pair，实际 {}", op, type_name));
        }
    }
}

// ---------------------------------------------------------------------------
// 谓词（null? pair? int? bool? procedure? eq?）
// ---------------------------------------------------------------------------

/// 一元谓词元数（10 case）：0 参 / 2 参。
#[test]
fn unary_predicate_arity() {
    let preds = ["null?", "pair?", "int?", "bool?", "procedure?"];
    for p in preds {
        expect_run_err(&format!("({})", p), &format!("{} 需要 1 个参数，实际 0", p));
        expect_run_err(
            &format!("({} 1 2)", p),
            &format!("{} 需要 1 个参数，实际 2", p),
        );
    }
}

/// eq? 元数（3 case）：恰好 2 参（对照链式 = ——eq? 不可链）。
#[test]
fn eq_arity() {
    expect_run_err("(eq?)", "eq? 需要 2 个参数，实际 0");
    expect_run_err("(eq? 1)", "eq? 需要 2 个参数，实际 1");
    expect_run_err("(eq? 1 2 3)", "eq? 需要 2 个参数，实际 3");
}

// ---------------------------------------------------------------------------
// 逻辑（not）与 I/O（print）
// ---------------------------------------------------------------------------

/// not 非布尔 ×7 类型（7 case，E1 载体）。
#[test]
fn not_requires_bool() {
    let bad = [
        ("5", "int"),
        ("1.5", "float"),
        ("\"s\"", "str"),
        ("nil", "nil"),
        ("(cons 1 2)", "pair"),
        ("(lambda (x) x)", "procedure"),
        ("car", "builtin-procedure"),
    ];
    for (operand, type_name) in bad {
        expect_run_err(
            &format!("(not {})", operand),
            &format!("not 需要 bool，实际 {}", type_name),
        );
    }
}

/// not/print 元数（4 case）。
#[test]
fn not_and_print_arity() {
    expect_run_err("(not)", "not 需要 1 个参数，实际 0");
    expect_run_err("(not true false)", "not 需要 1 个参数，实际 2");
    expect_run_err("(require io write) (print)", "print 需要 1 个参数，实际 0");
    expect_run_err(
        "(require io write) (print 1 2)",
        "print 需要 1 个参数，实际 2",
    );
}

/// read-line 元数（FS-4 修复——r8 能力参数化重写时补齐校验，启用
/// 存档断言：带参与 print 族一致报元数错误，不再静默读 stdin）。
#[test]
fn read_line_arity() {
    expect_run_err("(require io read) (read-line 1)", "read-line 需要 0 个参数");
}

// ---------------------------------------------------------------------------
// 字符串（str-append）
// ---------------------------------------------------------------------------

/// str-append 非字符串 ×7 类型 ×2 位置（14 case）。
#[test]
fn str_append_requires_strings() {
    let bad = [
        "1",
        "1.5",
        "true",
        "nil",
        "(cons 1 2)",
        "(lambda (x) x)",
        "car",
    ];
    for b in bad {
        expect_run_err(
            &format!("(str-append {} \"b\")", b),
            "str-append 需要 2 个字符串",
        );
        expect_run_err(
            &format!("(str-append \"a\" {})", b),
            "str-append 需要 2 个字符串",
        );
    }
}

/// str-append 元数（2 case）：恰好 2 参。
#[test]
fn str_append_arity() {
    expect_run_err("(str-append \"a\")", "str-append 需要 2 个参数");
    expect_run_err("(str-append \"a\" \"b\" \"c\")", "str-append 需要 2 个参数");
}

// ---------------------------------------------------------------------------
// 调用位（E4 NotCallable）与 if 条件（E1）
// ---------------------------------------------------------------------------

/// 不可调用值 ×7 类型在调用位（7 case）：字面量/序对/过程以外的值。
#[test]
fn not_callable_values() {
    let bad = [
        ("(1 2)", "int"),
        ("(1.5 1)", "float"),
        ("(\"s\" 1)", "str"),
        ("(true 1)", "bool"),
        ("(false 1)", "bool"),
        ("(nil 1)", "nil"),
        ("((cons 1 2) 1)", "pair"),
    ];
    for (src, type_name) in bad {
        expect_run_err(src, &format!("不可调用的值：{}", type_name));
    }
}

/// if 条件非布尔 ×6 类型（6 case，VM 消息形态）。注：eval 路径消息
/// 前缀不同（「if 条件需要 bool」）——双路径分裂面，仅断言 VM 侧。
#[test]
fn if_condition_requires_bool() {
    let bad = [
        ("1", "int"),
        ("1.5", "float"),
        ("\"s\"", "str"),
        ("nil", "nil"),
        ("(cons 1 2)", "pair"),
        ("(lambda (x) x)", "procedure"),
    ];
    for (operand, type_name) in bad {
        let src = format!("(if {} 1 2)", operand);
        expect_run_err(&src, &format!("条件位置需要 bool，实际 {}", type_name));
    }
}

// ---------------------------------------------------------------------------
// 用户过程元数（E2 ArityMismatch）
// ---------------------------------------------------------------------------

/// lambda/函数糖元数不匹配（5 case，含多语句程序）。
#[test]
fn lambda_arity_mismatch() {
    expect_run_err("((lambda (x) x) 1 2)", "过程参数数量不匹配：期望 1 实际 2");
    expect_run_err("((lambda (x y) x) 1)", "过程参数数量不匹配：期望 2 实际 1");
    expect_run_err(
        "((lambda (a b c) a) 1 2)",
        "过程参数数量不匹配：期望 3 实际 2",
    );
    expect_run_err("((lambda () 1) 1)", "过程参数数量不匹配：期望 0 实际 1");
    expect_run_err(
        "(define (f x) x) (f 1 2)",
        "过程参数数量不匹配：期望 1 实际 2",
    );
}

/// 未绑定变量（2 case，VM 消息形态）——消息与 eval 路径分裂（如实
/// 断言 VM 侧；eval 侧为「未绑定变量」）。
#[test]
fn unbound_variable_vm_message() {
    let msg = "未绑定的全局变量（卫生回退解析已在 driver 完成）";
    expect_run_err("undefined-x", msg);
    expect_run_err("(undefined-fn 1)", msg);
}

// ---------------------------------------------------------------------------
// E0004 码与渲染形状
// ---------------------------------------------------------------------------

/// E0004 渲染形状完整断言（3 case）：error[E0004] + `-->` + 源摘录 +
/// 诊断码结构（Diagnostic 是数据，§8.7）。
#[test]
fn vm_error_code_is_e0004() {
    let err = expect_run_err("(+ \"s\" 1)", "+ 需要 int");
    assert!(err.rendered.contains("error[E0004]: + 需要 int"));
    assert!(err.rendered.contains("--> neg.krf:1:1"));
    assert!(err.rendered.contains("1 | (+ \"s\" 1)"));
    assert!(err.to_string().starts_with("[run] error[E0004]"));
    assert_eq!(
        err.diagnostic.code,
        Some(kerf_span::DiagnosticCode(4)),
        "Run 错误必须编号为 E0004"
    );
    let err2 = expect_run_err("(car nil)", "car 需要 pair，实际 nil");
    // Call 指令的调试 Span = 整个应用形式（1:1）——错误定位到调用位
    assert!(err2.rendered.contains("--> neg.krf:1:1"));
    let err3 = expect_run_err(
        "(define (f x) x) (f 1 2)",
        "过程参数数量不匹配：期望 1 实际 2",
    );
    assert!(err3.rendered.contains("error[E0004]"));
    assert_eq!(err3.diagnostic.primary_span.start, 17);
}

/// 调用帧上限（1 case）：**非尾位**失控递归在 VM 路径报结构化错误
/// （MAX_FRAMES = 100_000；迭代式循环——§8.12）。
/// H2/TCO 注记：尾递归（旧用例 `(c (- n 1))` 尾位）经帧复用恒定帧数
/// 不再触发本上限——尾递归正例移驻 tco_tests（batch H2 交付物）；
/// 此处改用非尾形态（`(+ 1 …)` 消费结果 → 每层实增一帧）保持
/// 「帧上限结构化报错」验证意图。eval 号径（递归求值器）在同深度
/// Rust 栈溢出 abort——分裂存档于 `deep_recursion_eval_path_ignored`。
#[test]
fn frame_limit_deep_recursion_vm_path() {
    let src = "(define (c n) (if (= n 0) 0 (+ 1 (c (- n 1))))) (c 105001)";
    expect_run_err(src, "调用帧超过上限 100000（失控递归）");
}

/// 深尾递归生产路径（42-d 口径重写——TD-017 终验：eval 参考路径
/// 退役，其 256 深度上限域随路径注销；生产 VM 路径经 TCO 帧复用
/// 无深度上限——105_000 层尾递归正确终止）。
#[test]
fn deep_tail_recursion_production_path_tco() {
    let src = "(define (c n) (if (= n 0) 0 (c (- n 1)))) (c 105000)";
    let o = kerf_driver::run_source(src, "neg.krf").expect("生产路径应经 TCO 成功");
    assert!(matches!(o.value, kerf_vm::Value::Int(0)));
    // 双路径一致正例（种子链同果——fib(10)）
    assert!(common::dual_path_agrees(
        "(define (f n) (if (< n 2) n (+ (f (- n 1)) (f (- n 2))))) (f 10)"
    ));
}

/// mod i64::MIN -1 溢出结构化错误（D4 修复：checked_rem）+ 除法对偶。
#[test]
fn mod_i64_min_structured_error() {
    expect_run_err("(mod -9223372036854775808 -1)", "整数取模溢出");
    expect_run_err("(/ -9223372036854775808 -1)", "整数除法溢出");
    // 种子链共享内置（两路径同报——T1 新口径 42-d）
    let ev = kerf_driver::run_source_seed("(mod -9223372036854775808 -1)", "neg.krf").unwrap_err();
    assert!(ev.rendered.contains("整数取模溢出"));
}

// ---------------------------------------------------------------------------
// 双路径一致负例（仅已对齐语义：E6/set!-E3/参数重名/内置错误）
// ---------------------------------------------------------------------------

/// 双路径一致错误程序（12 case）：VM 与 eval 渲染逐字节一致
/// （§21.8 Phase 1）。仅纳入**已对齐**的消息形态——未绑/if-非布尔
/// 等分裂面见文件头注与 negative_semantics_tests.rs。
#[test]
fn dual_path_agrees_on_error_programs() {
    let programs = [
        // E6：同层重复定义（Task 12 修复面）
        "(define x 1) (define x 2)",
        "(define (f x) x) (define (f y) y)",
        "(begin (define x 1) (define x 2))",
        // E3：set! 未绑定（Task 12 修复面）
        "(set! y 1)",
        // 参数重名（展开期单点防御）
        "(lambda (x x) x)",
        // E5：内置运行时错误
        "(/ 1 0)",
        "(mod 5 0)",
        "(+ 9223372036854775807 1)",
        // E1：内置类型错误
        "(car 5)",
        "(+ \"s\" 1)",
        "(mod 5 2.0)",
        "(not 5)",
    ];
    for src in programs {
        assert!(
            common::dual_path_agrees(src),
            "双路径错误形态不一致：{}",
            src
        );
    }
}

/// 帧上限错误路径的辅助验证（1 case）：VM 报错后 RunOutcome 不产出
/// （错误即数据——Result 通道承载）。
#[test]
fn run_error_yields_no_value() {
    let r = run_source("(+ 1 0) (+ \"s\" 1)", "neg.krf");
    match r {
        Ok(o) => panic!("不应产出值：{:?}", o.value.type_name()),
        Err(e) => assert!(matches!(e.diagnostic.severity, kerf_span::Severity::Error)),
    }
    let _ = Value::Nil; // 值类型可达性锚（kerf-vm 依赖在位）
}

// ---------------------------------------------------------------------------
// TD-002 符号值语义负例（符号 datum 解禁后的值消费端边界）
// 消息经实跑校准（kerf run 探针，2026-09-10）
// ---------------------------------------------------------------------------

/// 符号值误用（6 case）：算术 / 条件 / 序对操作 / 数值比较 / 链式比较。
#[test]
fn symbol_value_misuse() {
    let cases: &[(&str, &str)] = &[
        ("(+ 1 'a)", "+ 需要 int"),
        ("(- 'a 'b)", "- 需要 int"),
        ("(* 'a 2)", "* 需要 int"),
        ("(if 'a 1 2)", "条件位置需要 bool，实际 symbol"),
        ("(car 'a)", "car 需要 pair，实际 symbol"),
        ("(= 'a 1)", "= 需要数值"),
    ];
    for (src, msg) in cases {
        expect_run_err(src, msg);
    }
}
