//! 流水线 E2E 集成测试（tests/v0/stage0/plan/pipeline_tests.rs ↔ docs/tests/v0/stage0/plan/pipeline.md）。
//!
//! 覆盖：全管线数据流（Reader → Expander → Compiler → VM → Runtime）/
//! Span 全管线传播（§21.3 验收项 4）/ CodeValue 检查 / 接口预留冻结 /
//! 诊断渲染 / 性能基准基线。

use crate::common;

use kerf_core::CodeValue;
use kerf_driver::{compile_source, eval_source, run_source, Stage};

/// 全管线快照：编译 → 执行 → 渲染。
#[test]
fn pipeline_end_to_end_snapshot() {
    let src = "(define (square x) (* x x)) (square 12)";
    assert_eq!(common::run_rendered(src), "144");
}

/// Span 全管线传播抽检（§21.3 Stage 0 验收项 4）：
/// 词法 Span → 语法对象 → CoreExpr → 字节码 debug_info。
#[test]
fn span_propagates_through_all_stages() {
    let src = "(define x 1) (+ x 2)";
    let out = compile_source(src, "span.krf").unwrap();
    // 1. 词法 → 语法对象：define 形式覆盖源区间
    let core = &out.core[0];
    assert_eq!(core.span().start, 0);
    // 2. CoreExpr → 图 IR：元信息携带 Span
    assert!(!out.ir.get_metadata(out.ir.roots()[0]).span.is_empty());
    // 3. 字节码：每条指令有 debug_span
    for proto in &out.program.protos {
        assert_eq!(proto.code.len(), proto.debug_spans.len());
    }
    // 4. 运行时错误反查（错误路径的 Span 链路）
    let err = run_source("(+ 1 (car 2))", "err.krf").unwrap_err();
    assert_eq!(err.stage, Stage::Run);
    assert!(err.rendered.contains("err.krf"));
}

/// CodeValue 检查（§22.3 里程碑：CodeValue 良构）。
#[test]
fn code_value_well_formed_check() {
    let out = compile_source("(define (f x) (+ x 1)) (f 2)", "cv.krf").unwrap();
    for e in &out.core {
        let cv = CodeValue::from_expr(e);
        assert!(cv.is_well_formed(), "CodeValue 必须良构");
    }
}

/// 四项接口预留冻结（§22.3 里程碑 + §21.3 验收项 6）。
#[test]
fn four_reserved_interfaces_frozen() {
    // 编译缓存（P2）：键内容寻址
    let k1 = kerf_driver::CacheKey {
        source_hash: 1,
        config_fingerprint: 2,
    };
    let k2 = k1.clone();
    assert_eq!(k1, k2);
    // 能力模型令牌（P2）：类型可引用、不可伪造（私有构造）
    fn assert_read(_c: &kerf_driver::ReadCapability) {}
    fn assert_write(_c: &kerf_driver::WriteCapability) {}
    let _ = (
        assert_read as fn(&kerf_driver::ReadCapability),
        assert_write as fn(&kerf_driver::WriteCapability),
    );
    // EffectSystem / MultiStage（P3）：trait 可命名（冻结形状由
    // kerf-driver::reserved 单元测试的 Probe 实现证明）
}

/// 同结果测试：同程序两次编译 + 两次执行结果一致（§21.3 验收项 3 的
/// Stage 0 形态——Rust 宿主下的管线确定性）。
#[test]
fn convergent_compilation_and_execution() {
    let src = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 15)";
    let out1 = compile_source(src, "same.krf").unwrap();
    let out2 = compile_source(src, "same.krf").unwrap();
    assert!(out1.program.bytecode_equal(&out2.program));
    let r1 = run_source(src, "same.krf").unwrap();
    let r2 = run_source(src, "same.krf").unwrap();
    assert!(r1.value.eq_value(&r2.value));
}

/// 诊断渲染：每阶段错误的完整人类可读输出（§2.2 原则 16）。
#[test]
fn diagnostics_render_per_stage() {
    // Read 阶段
    let e1 = run_source("(+ 1", "d1.krf").unwrap_err();
    assert!(e1.rendered.contains("error[E0001]"));
    assert!(e1.rendered.contains("d1.krf:1:1"));
    // Expand 阶段
    let e2 = run_source("(if)", "d2.krf").unwrap_err();
    assert_eq!(e2.stage, Stage::Expand);
    // Run 阶段
    let e3 = run_source("(car 3)", "d3.krf").unwrap_err();
    assert_eq!(e3.stage, Stage::Run);
    assert!(e3.rendered.contains("d3.krf"));
}

/// eval 路径可用（参考语义路径完整）。
#[test]
fn eval_path_complete() {
    let o = eval_source("(let ((x 3)) (* x x))", "e.krf").unwrap();
    assert!(matches!(o.value, kerf_vm::Value::Int(9)));
}

/// 内置函数全集合（§9 stdlib：核心零内置，driver 注入）。
#[test]
fn builtin_library_registered() {
    let cases = [
        ("(+ 1 2)", "3"),
        ("(- 10 4 1)", "5"),
        ("(* 2 3 4)", "24"),
        ("(/ 100 5 2)", "10"),
        ("(mod 17 5)", "2"),
        ("(= 3 3 3)", "true"),
        ("(< 1 2 3)", "true"),
        ("(> 3 1)", "true"),
        ("(<= 2 2)", "true"),
        ("(>= 1 2)", "false"),
        ("(cons 1 2)", "(1 . 2)"),
        ("(car (quote (9 8)))", "9"),
        ("(cdr (quote (9 8)))", "(8)"),
        ("(list 1 2 3)", "(1 2 3)"),
        ("(null? nil)", "true"),
        ("(pair? (cons 1 2))", "true"),
        ("(int? 5)", "true"),
        ("(bool? true)", "true"),
        ("(procedure? car)", "true"),
        ("(eq? 1 1)", "true"),
        ("(not false)", "true"),
        ("(str-append \"foo\" \"bar\")", "foobar"),
    ];
    for (src, expected) in cases {
        assert_eq!(common::run_rendered(src), expected, "内置 {}", src);
    }
}

/// 字符串与浮点程序。
#[test]
fn strings_and_floats() {
    assert_eq!(common::run_rendered("\"hello\""), "hello");
    assert_eq!(common::run_rendered("(+ 1.5 2.5)"), "4.0");
    assert_eq!(common::run_rendered("(* 2 1.5)"), "3.0");
}

/// 宏系统用户程序（卫生 + 省略号组合）。
#[test]
fn user_macro_programs() {
    let src = r#"
        (define-syntax swap!
          (syntax-rules ()
            ((swap! a b)
             (let ((tmp a))
               (set! a b)
               (set! b tmp)))))
        (define x 1)
        (define y 2)
        (swap! x y)
        (list x y)
    "#;
    assert_eq!(common::run_rendered(src), "(2 1)");
}

/// 结构化错误：DriverError 可 Display。
#[test]
fn driver_error_display_format() {
    let err = run_source("(lambda (x))", "f.krf").unwrap_err();
    let text = format!("{}", err);
    assert!(text.starts_with("[expand]"));
}
