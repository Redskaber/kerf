//! Compiler 集成测试（tests/v0/stage0/plan/compiler_tests.rs ↔ docs/tests/v0/stage0/plan/compiler.md）。
//!
//! 覆盖：跳转回填零占位残留（§20.1 Week 3 验收）/ 常量池去重 /
//! 闭包捕获描述符 / 栈平衡 / 调试信息全覆盖 / 同结果（确定性）编译。

use kerf_compiler::{compile_module, Op};
use kerf_core::CoreExpr;
use std::rc::Rc;

fn compile_via_driver(src: &str) -> kerf_driver::CompileOutput {
    kerf_driver::compile_source(src, "test.krf").unwrap()
}

/// 跳转回填完备：全部 JUMP/JUMP_IF_FALSE 目标非占位（§19.3 不变式 2）。
#[test]
fn backpatch_leaves_no_placeholders() {
    let out =
        compile_via_driver("(define (f n) (if (< n 2) n (+ (f (- n 1)) (f (- n 2))))) (f 10)");
    for proto in &out.program.protos {
        for (pc, op) in proto.code.iter().enumerate() {
            match op {
                Op::Jump(t) | Op::JumpIfFalse(t) => {
                    assert_ne!(*t, u32::MAX, "占位残留：proto pc {}", pc);
                    assert!((*t as usize) < proto.code.len(), "回填目标越界");
                }
                _ => {}
            }
        }
    }
}

/// 常量池去重（§19.3 陷阱：(do 1 1 1) 不膨胀）。
#[test]
fn const_pool_dedup() {
    let out = compile_via_driver("(do 1 1 1 1 1)");
    let int_consts = out
        .program
        .consts
        .iter()
        .filter(|c| matches!(c, kerf_compiler::BcConst::Int(1)))
        .count();
    assert_eq!(int_consts, 1);
}

/// 调试信息全覆盖：每条指令都有 (pc, Span)（§19.3 不变式 3 + §22.3）。
#[test]
fn debug_info_full_coverage() {
    let out = compile_via_driver("(define (f x) (+ x 1)) (f 41)");
    for (i, proto) in out.program.protos.iter().enumerate() {
        assert_eq!(
            proto.code.len(),
            proto.debug_spans.len(),
            "proto {} 调试信息不全",
            i
        );
    }
}

/// 闭包捕获描述符：嵌套 fn 的捕获源正确。
#[test]
fn nested_closure_capture_descriptors() {
    // (fn (x) (fn (y) (x y)))——内层捕获 x
    let x = kerf_syntax::Symbol(1);
    let y = kerf_syntax::Symbol(2);
    let inner = Rc::new(CoreExpr::Fn {
        params: vec![y],
        param_scopes: vec![kerf_syntax::ScopeSet::new()],
        body: Rc::new(CoreExpr::Apply {
            fn_expr: Rc::new(CoreExpr::Var {
                name: x,
                scopes: kerf_syntax::ScopeSet::new(),
                span: Default::default(),
            }),
            args: vec![Rc::new(CoreExpr::Var {
                name: y,
                scopes: kerf_syntax::ScopeSet::new(),
                span: Default::default(),
            })],
            span: Default::default(),
        }),
        span: Default::default(),
    });
    let outer = Rc::new(CoreExpr::Fn {
        params: vec![x],
        param_scopes: vec![kerf_syntax::ScopeSet::new()],
        body: inner,
        span: Default::default(),
    });
    let program = compile_module(&[outer]).unwrap();
    assert_eq!(program.protos.len(), 3);
    assert_eq!(program.protos[2].capture_names, vec![x]);
}

/// 求值顺序契约：Apply 参数从左到右、被调者最后（§19.3/§19.5 不变式 3）。
#[test]
fn app_evaluation_order() {
    let out = compile_via_driver("(f a b)");
    let main = out.program.entry_proto();
    // LOAD_GLOBAL × 3（a, b, f）然后 CALL 2
    assert!(matches!(main.code[0], Op::LoadGlobal(_)));
    assert!(matches!(main.code[1], Op::LoadGlobal(_)));
    assert!(matches!(main.code[2], Op::LoadGlobal(_)));
    assert_eq!(main.code[3], Op::Call(2));
}

/// 同结果测试（确定性编译——两次编译字节码一致，§21.3 Stage 0 验收项）。
#[test]
fn deterministic_compilation_two_passes_identical() {
    let src = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 12)";
    let a = compile_via_driver(src);
    let b = compile_via_driver(src);
    assert!(
        a.program.bytecode_equal(&b.program),
        "两次编译输出必须字节一致（同结果测试）"
    );
}

/// main 原型以 HALT 终止。
#[test]
fn main_proto_terminates_with_halt() {
    let out = compile_via_driver("(+ 1 2)");
    let main = out.program.entry_proto();
    assert!(matches!(main.code.last(), Some(Op::Halt)));
}

/// define 产生 DEFINE_GLOBAL（D1/E6 语义）+ DUP 返回值 + 值登记。
#[test]
fn define_emits_global_store() {
    let out = compile_via_driver("(define x 5) x");
    assert!(!out.program.global_refs.is_empty());
    let main = out.program.entry_proto();
    // T1 对齐：Define 返回值 = v（DUP 留存），经 DefineGlobal 写入
    assert!(main.code.iter().any(|op| matches!(op, Op::DefineGlobal(_))));
    assert!(main.code.iter().any(|op| matches!(op, Op::Dup)));
}

/// 反汇编渲染（人类可感知输出）。
#[test]
fn disassembly_is_renderable() {
    let out = compile_via_driver("(define (f x) (* x x)) (f 7)");
    let text = out.disassemble();
    assert!(text.contains("CLOSURE"));
    assert!(text.contains("CALL"));
    assert!(text.contains("RET"));
    assert!(text.contains("HALT"));
    assert!(text.contains("proto"));
}

/// 图 IR：字面量共享（公共子表达式）。
#[test]
fn ir_shares_literal_nodes() {
    let out = compile_via_driver("(+ 1 1)");
    // 两个字面量 1 → 共享节点（node_map 命中）
    assert!(
        out.ir.len() < 5,
        "图 IR 节点数（共享生效）：{}",
        out.ir.len()
    );
}
