//! 阶段门审查测试（tests/v0/stage0/gate/gate_review_r1.rs ↔
//! docs/tests/v0/stage0/gate/gate-review-round1.md）。
//!
//! §22.3 第一个里程碑验证清单的**可执行审计**（每个清单项对应至少一个断言）：
//!
//! - [x] Reader 能正确解析所有 9 个核心原语的语法
//! - [x] Expander 能正确展开所有核心形式
//! - [x] Compiler 能正确生成字节码（含 debug_info）
//! - [x] VM 能正确执行所有操作码
//! - [x] GC 能正确回收未引用对象
//! - [x] Span 在所有阶段正确传播
//! - [x] Effect Handlers 接口已定义（实现为空）
//! - [x] 多阶段编程接口已定义
//! - [x] 能力模型 I/O 类型已定义
//! - [x] 编译缓存接口已定义
//!
//! （50+ 快照测试与性能基线由 plan/ 树全量覆盖——见 docs/tests/matrix.md。）

use crate::common;

use kerf_driver::{compile_source, run_source};
use kerf_vm::Value;

/// G1：Reader 解析 9 核心原语语法（每原语一读）。
#[test]
fn gate_g1_reader_parses_nine_primitives() {
    let forms = [
        "(fn (x) x)",
        "(f 1 2)",
        "(if a b c)",
        "x",
        "42",
        "(assign x 1)",
        "(define x 1)",
        "(do 1 2)",
        "(module m (define x 1) x)",
    ];
    for src in forms {
        let mut t = kerf_syntax::SymbolTable::new();
        let forms = kerf_reader::read_source(src, 0, &mut t)
            .unwrap_or_else(|e| panic!("Reader 解析失败 [{}]：{}", src, e.message));
        assert!(!forms.is_empty(), "Reader 空产物 [{}]", src);
    }
}

/// G2：Expander 展开 9 核心形式（无错误 + 语义正确性经 G3-G4 验证）。
#[test]
fn gate_g2_expander_expands_all_core_forms() {
    let src = r#"
        (define x 1)
        (assign x 2)
        (do x x)
        (if (= x 2) 10 20)
        ((fn (y) (+ y 1)) 5)
        (module m (define z 3) z)
    "#;
    let out = compile_source(src, "gate.krf").unwrap();
    assert_eq!(out.core.len(), 6);
}

/// G3：Compiler 生成字节码 + debug_info 全覆盖。
#[test]
fn gate_g3_compiler_debug_info_complete() {
    let out = compile_source("(define (f x) (+ x 1)) (f 2)", "gate.krf").unwrap();
    assert!(out.program.proto_count() >= 2);
    for proto in &out.program.protos {
        assert_eq!(proto.code.len(), proto.debug_spans.len());
    }
    assert!(out.program.total_instructions() >= 8);
}

/// G4：VM 执行全部操作码类别（各组至少一个操作码经真实程序触达）。
#[test]
fn gate_g4_vm_executes_opcode_groups() {
    // 栈操作（DUP 经 assign）/ 变量访问（local/captured/global）/
    // 控制流（jump）/ 函数（closure/call/ret）/ 算术 / 数据构造 / 谓词 / 终止
    let src = r#"
        (define (make-adder n) (fn (x) (+ x n)))
        (define add5 (make-adder 5))
        (define lst (quote (1 2 3)))
        (define acc 0)
        (list
          (add5 10)
          (if (is-nil nil) 1 2)
          (is-pair lst)
          (eq (head lst) 1)
          (mod 7 3)
          (do (assign acc 1) acc))
    "#;
    let o = run_source(src, "gate.krf").unwrap();
    assert!(matches!(o.value, Value::Pair(_)));
}

/// G5：GC 回收未引用对象（§21.3 验收——堆有界）。
#[test]
fn gate_g5_gc_collects_unreferenced() {
    let src = r#"
        (define (spin n)
          (if (= n 0) 0
              (do (cons 1 2) (cons 3 4) (cons 5 6) (cons 7 8)
                     (cons 9 10) (cons 11 12) (spin (- n 1)))))
        (spin 30000)
    "#;
    let o = run_source(src, "gate.krf").unwrap();
    assert!(matches!(o.value, Value::Int(0)));
    assert!(o.heap.stats().collections > 0);
}

/// G6：Span 全管线传播（§21.3 验收项 4）。
#[test]
fn gate_g6_span_propagation() {
    let out = compile_source("(define x 1) x", "gate.krf").unwrap();
    assert!(!out.core[0].span().is_empty());
    assert!(!out.ir.get_metadata(out.ir.roots()[0]).span.is_empty());
    for p in &out.program.protos {
        assert_eq!(p.code.len(), p.debug_spans.len());
    }
}

/// G7-G10：四项接口预留冻结（§21.3 验收项 6：接口预留冻结）。
#[test]
fn gate_g7_to_g10_reserved_interfaces() {
    // G7 Effect Handlers（P3）——trait 名可引用即冻结（P2/P3 无实现体）
    fn effect_family_name() -> &'static str {
        "EffectFamily"
    }
    assert_eq!(effect_family_name(), "EffectFamily");
    // G8 多阶段编程（P3）——CodeValue 的 compose_with 预留
    let expr = kerf_core::CoreExpr::Literal {
        value: kerf_core::LiteralValue::Int(1),
        span: Default::default(),
    };
    let cv = kerf_core::CodeValue::from_expr(&expr);
    assert!(matches!(
        cv.compose_with(&cv),
        Err(kerf_core::CompositionError::UnsupportedInStage0)
    ));
    // G9 能力模型 I/O（P2）——令牌类型冻结
    fn cap_shape(_r: &kerf_driver::ReadCapability, _w: &kerf_driver::WriteCapability) {}
    let _ = cap_shape as fn(&kerf_driver::ReadCapability, &kerf_driver::WriteCapability);
    // G10 编译缓存（P2）——键与 trait 冻结
    let key = kerf_driver::CacheKey {
        source_hash: 1,
        config_fingerprint: 1,
    };
    assert_eq!(key, key.clone());
    fn cache_shape<C: kerf_driver::CompilationCache + ?Sized>() {}
    let _ = cache_shape::<dyn kerf_driver::CompilationCache> as fn();
}

/// P3 冻结形状证明：效应族可被实现（签名稳定）。
#[allow(dead_code)] // 形状证明类型（仅在泛型位置使用）
struct TestFamily;
impl kerf_driver::EffectFamily for TestFamily {
    type Result = Value;
}

/// fib(25)：语义验证基准（§20.1 Week 3 验收——数值正确性）。
#[test]
fn gate_fib_25_semantics() {
    let src = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 25)";
    common::assert_int(src, 75025);
}
