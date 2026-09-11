//! E1-α 自举 Expander parity 套件（Stage 1 批次 E / Task 33-a）。
//!
//! 验收门（plan §5 批次 E「E1 Expander kerf 重写」α 阶段）：
//! - **结构 parity**：自举 Expander（expander.krf，VM 上运行）与种子
//!   Expander（kerf-expander/*.rs，Rust）在正例语料上——CoreExpr 树
//!   （原语形态 + Span(start,end) + 作用域集 + param_scopes）递归一致
//!   （符号按名——两实现各自 intern 序不保证一致，名字是唯一稳定口径；
//!   expansion_id 不参与判据——E1-α 边界，r6 Reader parity 同口径）；
//! - **错误 parity**：负例语料——消息 + Span(start,end) 逐字一致；
//! - **E1-α 边界**：define-syntax 显式报错（宏/define-syntax 属 E1-β）；
//! - **行为面**：自举展开产物经 compile + VM 执行，与 run_source（种子
//!   全管线）结果一致——展开产物是可执行 CoreExpr 的端到端证明。
//!
//! 遵循条款：§9.4.3（正负例成对）、§7.1（集成验证 ≥3）、§2.3-11
//! （先实测禁臆测——全部断言经双实现实跑比对）。

use std::rc::Rc;

use kerf_compiler::compile_module;
use kerf_core::{CoreExpr, LiteralValue};
use kerf_driver::bootstrap::read_source as bootstrap_read;
use kerf_driver::bootstrap_expander::expand_program;
use kerf_driver::builtins::register_globals;
use kerf_driver::capability::IoGrant;
use kerf_driver::run_source;
use kerf_expander::{expand_program as seed_expand, ExpandCtxt, ExpandError};
use kerf_runtime::Heap;
use kerf_syntax::{ScopeSet, Symbol, SymbolTable};
use kerf_vm::run_program;

// ---- parity 走查 ----

/// CoreExpr 树等价（原语形态 + Span(start,end) + 作用域集 + 结构递归；
/// 符号按名——两表各自 intern，名字是唯一稳定口径；expansion_id 不
/// 参与判据——E1-α 边界（自举侧糖产物恒 0，种子侧随代次递增）。
fn core_equiv(
    a: &CoreExpr,
    ta: &SymbolTable,
    b: &CoreExpr,
    tb: &SymbolTable,
) -> Result<(), String> {
    if a.span().start != b.span().start || a.span().end != b.span().end {
        return Err(format!(
            "Span 不一致：({},{}) vs ({},{})",
            a.span().start,
            a.span().end,
            b.span().start,
            b.span().end
        ));
    }
    match (a, b) {
        (CoreExpr::Literal { value: x, .. }, CoreExpr::Literal { value: y, .. }) => {
            if x != y {
                Err(format!("字面量不一致：{:?} vs {:?}", x, y))
            } else {
                Ok(())
            }
        }
        (
            CoreExpr::VarRef {
                name: x,
                scopes: sx,
                ..
            },
            CoreExpr::VarRef {
                name: y,
                scopes: sy,
                ..
            },
        ) => {
            if ta.name(*x) != tb.name(*y) {
                return Err(format!("var 名不一致：{} vs {}", ta.name(*x), tb.name(*y)));
            }
            if sx != sy {
                return Err(format!(
                    "var「{}」作用域集不一致：{:?} vs {:?}",
                    ta.name(*x),
                    sx.iter().collect::<Vec<_>>(),
                    sy.iter().collect::<Vec<_>>()
                ));
            }
            Ok(())
        }
        (
            CoreExpr::Lambda {
                params: px,
                param_scopes: sx,
                body: bx,
                ..
            },
            CoreExpr::Lambda {
                params: py,
                param_scopes: sy,
                body: by,
                ..
            },
        ) => {
            if px.len() != py.len() {
                return Err(format!("形参数不一致：{} vs {}", px.len(), py.len()));
            }
            for (i, (x, y)) in px.iter().zip(py.iter()).enumerate() {
                if ta.name(*x) != tb.name(*y) {
                    return Err(format!(
                        "形参名不一致[{}]：{} vs {}",
                        i,
                        ta.name(*x),
                        tb.name(*y)
                    ));
                }
            }
            if sx != sy {
                return Err(format!(
                    "param_scopes 不一致：{:?} vs {:?}",
                    scope_debug(sx),
                    scope_debug(sy)
                ));
            }
            core_equiv(bx, ta, by, tb).map_err(|e| format!("[body]: {}", e))
        }
        (
            CoreExpr::App {
                fn_expr: fx,
                args: ax,
                ..
            },
            CoreExpr::App {
                fn_expr: fy,
                args: ay,
                ..
            },
        ) => {
            core_equiv(fx, ta, fy, tb).map_err(|e| format!("[fn]: {}", e))?;
            if ax.len() != ay.len() {
                return Err(format!("实参数不一致：{} vs {}", ax.len(), ay.len()));
            }
            for (i, (x, y)) in ax.iter().zip(ay.iter()).enumerate() {
                core_equiv(x, ta, y, tb).map_err(|e| format!("[arg{}]: {}", i, e))?;
            }
            Ok(())
        }
        (
            CoreExpr::If {
                cond: cx,
                then_branch: tx,
                else_branch: ex,
                ..
            },
            CoreExpr::If {
                cond: cy,
                then_branch: ty,
                else_branch: ey,
                ..
            },
        ) => {
            core_equiv(cx, ta, cy, tb).map_err(|e| format!("[cond]: {}", e))?;
            core_equiv(tx, ta, ty, tb).map_err(|e| format!("[then]: {}", e))?;
            core_equiv(ex, ta, ey, tb).map_err(|e| format!("[else]: {}", e))
        }
        (
            CoreExpr::SetBang {
                name: x,
                scopes: sx,
                value: vx,
                ..
            },
            CoreExpr::SetBang {
                name: y,
                scopes: sy,
                value: vy,
                ..
            },
        ) => {
            if ta.name(*x) != tb.name(*y) {
                return Err(format!("set! 名不一致：{} vs {}", ta.name(*x), tb.name(*y)));
            }
            if sx != sy {
                return Err(format!(
                    "set!「{}」作用域集不一致：{:?} vs {:?}",
                    ta.name(*x),
                    sx.iter().collect::<Vec<_>>(),
                    sy.iter().collect::<Vec<_>>()
                ));
            }
            core_equiv(vx, ta, vy, tb).map_err(|e| format!("[value]: {}", e))
        }
        (
            CoreExpr::Define {
                name: x, value: vx, ..
            },
            CoreExpr::Define {
                name: y, value: vy, ..
            },
        ) => {
            if ta.name(*x) != tb.name(*y) {
                return Err(format!(
                    "define 名不一致：{} vs {}",
                    ta.name(*x),
                    tb.name(*y)
                ));
            }
            core_equiv(vx, ta, vy, tb).map_err(|e| format!("[value]: {}", e))
        }
        (CoreExpr::Begin { body: x, .. }, CoreExpr::Begin { body: y, .. }) => {
            seq_equiv(x, ta, y, tb, "begin")
        }
        (
            CoreExpr::Module {
                name: nx,
                imports: ix,
                exports: ex,
                body: bx,
                ..
            },
            CoreExpr::Module {
                name: ny,
                imports: iy,
                exports: ey,
                body: by,
                ..
            },
        ) => {
            if ta.name(*nx) != tb.name(*ny) {
                return Err(format!(
                    "module 名不一致：{} vs {}",
                    ta.name(*nx),
                    tb.name(*ny)
                ));
            }
            for (dir, x, y) in [("import", ix, iy), ("export", ex, ey)] {
                if x.len() != y.len() {
                    return Err(format!("{} 数不一致", dir));
                }
                for (i, (a2, b2)) in x.iter().zip(y.iter()).enumerate() {
                    if ta.name(*a2) != tb.name(*b2) {
                        return Err(format!("{}[{}] 不一致", dir, i));
                    }
                }
            }
            seq_equiv(bx, ta, by, tb, "module-body")
        }
        (CoreExpr::Require { caps: x, .. }, CoreExpr::Require { caps: y, .. }) => {
            if x == y {
                Ok(())
            } else {
                Err(format!("require 能力不一致：{:?} vs {:?}", x, y))
            }
        }
        _ => Err(format!(
            "原语形态不一致：{} vs {}",
            a.kind_name(),
            b.kind_name()
        )),
    }
}

fn seq_equiv(
    x: &[Rc<CoreExpr>],
    ta: &SymbolTable,
    y: &[Rc<CoreExpr>],
    tb: &SymbolTable,
    what: &str,
) -> Result<(), String> {
    if x.len() != y.len() {
        return Err(format!("{} 元素数不一致：{} vs {}", what, x.len(), y.len()));
    }
    for (i, (a, b)) in x.iter().zip(y.iter()).enumerate() {
        core_equiv(a, ta, b, tb).map_err(|e| format!("[{}[{}]]: {}", what, i, e))?;
    }
    Ok(())
}

fn scope_debug(sets: &[ScopeSet]) -> Vec<Vec<u32>> {
    sets.iter().map(|s| s.iter().collect()).collect()
}

/// 正例 parity 断言：双读（各自表）→ 双展开 → 结构逐字比对。
fn parity(src: &str) {
    let mut ta = SymbolTable::new();
    let mut tb = SymbolTable::new();
    let fa = bootstrap_read(src, 0, &mut ta).expect("oracle 读取失败");
    let fb = bootstrap_read(src, 0, &mut tb).expect("自举 读取失败");
    let mut ctx = ExpandCtxt::new(ta);
    let oracle = seed_expand(&fa, &mut ctx).unwrap_or_else(|e| {
        panic!(
            "种子展开失败（语料应为正例）：{} @ {}",
            e.message, e.span.start
        )
    });
    let kerf = expand_program(&fb, 0, &mut tb).unwrap_or_else(|e| {
        panic!(
            "自举展开失败（语料应为正例）：{} @ {}",
            e.message, e.span.start
        )
    });
    assert_eq!(
        oracle.len(),
        kerf.len(),
        "顶层形式数不一致（src: {:?}）",
        src
    );
    for (i, (a, b)) in oracle.iter().zip(kerf.iter()).enumerate() {
        core_equiv(a, &ctx.table, b, &tb)
            .unwrap_or_else(|e| panic!("parity 失败[{}]（src: {:?}）：{}", i, src, e));
    }
}

/// 负例 parity 断言：双路径均报错且消息 + Span(start,end) 逐字一致。
fn parity_err(src: &str) {
    let mut ta = SymbolTable::new();
    let mut tb = SymbolTable::new();
    let fa = bootstrap_read(src, 0, &mut ta).expect("oracle 读取失败");
    let fb = bootstrap_read(src, 0, &mut tb).expect("自举 读取失败");
    let mut ctx = ExpandCtxt::new(ta);
    let ea: ExpandError = seed_expand(&fa, &mut ctx).expect_err("种子应报错（语料应为负例）");
    let eb = expand_program(&fb, 0, &mut tb).expect_err("自举应报错（语料应为负例）");
    assert_eq!(ea.message, eb.message, "错误消息不一致（src: {:?}）", src);
    assert_eq!(
        (ea.span.start, ea.span.end),
        (eb.span.start, eb.span.end),
        "错误 Span 不一致（src: {:?}）：{}",
        src,
        ea.message
    );
}

/// 行为面断言：自举展开产物经 compile + VM 执行，与 run_source（种子
/// 全管线）结果一致——展开产物是可执行 CoreExpr 的端到端证明。
fn behavior(src: &str) {
    let expected = run_source(src, "behavior.krf").expect("种子全管线失败");
    let mut t = SymbolTable::new();
    let forms = bootstrap_read(src, 0, &mut t).expect("读取失败");
    let core = expand_program(&forms, 0, &mut t).expect("自举展开失败");
    let program = compile_module(&core).expect("自举产物编译失败");
    // 行为语料无 I/O（纯算术/控制流）——空授权（R9 fail-closed）
    let mut globals = register_globals(&mut t, &IoGrant::none());
    let mut heap = Heap::new();
    let actual = run_program(&program, &mut globals, &mut heap).expect("自举产物执行失败");
    assert!(
        actual.eq_value(&expected.value),
        "行为不一致（src: {:?}）：{:?} vs {:?}",
        src,
        kerf_vm::render_value(&actual, &heap),
        kerf_vm::render_value(&expected.value, &expected.heap)
    );
}

#[allow(dead_code)]
fn _unused(_: &Symbol, _: &LiteralValue) {}

// ---- 正例 parity：核心形式 ----

#[test]
fn parity_literals() {
    for src in ["42", "3.5", "\"hi\"", "true", "false", "nil"] {
        parity(src);
    }
}

#[test]
fn parity_symbol_ref_empty_scopes() {
    // 顶层自由引用：作用域集 = ∅（r13 语义锚——空集 = 全局兜底）
    parity("x");
    parity("(+ x 1)");
}

#[test]
fn parity_lambda_nested_scopes() {
    // r13 fresh-scope 注入 parity：嵌套 lambda 的 param_scopes 与体内
    // VarRef 作用域集逐集一致
    parity("((lambda (x) (+ x 1)) 41)");
    parity("(lambda (x) (lambda (y) (+ x y)))");
    parity("(lambda (x) (lambda (x) x))");
}

#[test]
fn parity_if_setbang_define_begin() {
    parity("(if x 1 2)");
    parity("(if x 1)");
    parity("(set! x 5)");
    parity("(define x 5)");
    parity("(define (f x) (* x x))");
    parity("(begin 1 2 3)");
    parity("(begin)");
}

#[test]
fn parity_quote() {
    parity("(quote sym)");
    parity("'sym");
    parity("(quote (1 2 3))");
    parity("(quote (a (b c) 2.5))");
    parity("'()");
}

// ---- 正例 parity：九糖 ----

#[test]
fn parity_sugar_let_family() {
    parity("(let ((x 1) (y 2)) (+ x y))");
    parity("(letrec ((f (lambda () 1))) (f))");
    parity("(let* ((x 1) (y (+ x 1))) (+ x y))");
    parity("(let* () 7)");
}

#[test]
fn parity_sugar_cond_and_or() {
    parity("(cond (false 1) (true 2) (else 3))");
    parity("(cond ((= 1 2)))");
    parity("(cond (a 1 2 3) (else 4))");
    parity("(and a b c)");
    parity("(or a b)");
    parity("(and)");
    parity("(or a)");
    parity("(and x)");
}

#[test]
fn parity_sugar_when_unless_while() {
    parity("(when c 1 2)");
    parity("(unless c 1)");
    // while：loop$hyg$1 唯一化名（HygieneCtx 每次实例化计数重置的镜像）
    parity("(while (< i 10) (set! i (+ i 1)))");
}

// ---- 正例 parity：module/require/内部 define 提升 ----

#[test]
fn parity_module_require() {
    parity("(module m (import a) (export f) (define f 1) (f))");
    parity("(module m (define f 1))");
    parity("(require io read write)");
    parity("(require io read)");
}

#[test]
fn parity_inner_define_hoisting() {
    // 提升路径：构造节点 ∅ 作用域 + hoisted fresh scope（r13 语义）
    parity("(lambda (x) (define y 2) (+ x y))");
    parity("(lambda (x) (define y 2) (define z 3) (+ x (+ y z)))");
    parity("(define (f x) (define (g y) (+ x y)) (g 1))");
}

// ---- 负例 parity（消息 + Span 逐字一致） ----

#[test]
fn parity_err_core_forms() {
    parity_err("()");
    parity_err("(if)");
    parity_err("(if 1 2 3 4)");
    parity_err("(set! 1 2)");
    parity_err("(set! x)");
    parity_err("(lambda x 1)");
    parity_err("(lambda (1) 1)");
    parity_err("(lambda (x x) 1)");
    parity_err("(define)");
    parity_err("(define (1 x) 1)");
    parity_err("(define x 1 2)");
    parity_err("(lambda (x) (+ x 1) (define y 2))");
}

#[test]
fn parity_err_hoisting_paths() {
    // 提升路径错误（切分期名校验 / 值校验——提升路径短消息口径）
    parity_err("(lambda (x) (define) 1)");
    parity_err("(lambda (x) (define x 1 2) x)");
    parity_err("(lambda (x) (define 1 2) x)");
}

#[test]
fn parity_err_quote_vector() {
    parity_err("[1 2]");
    parity_err("(quote [1])");
}

#[test]
fn parity_err_sugar() {
    parity_err("(let)");
    parity_err("(let (x) 1)");
    parity_err("(let ((x)) 1)");
    parity_err("(let ((1 2)) 1)");
    parity_err("(letrec)");
    parity_err("(letrec ((x)) 1)");
    parity_err("(let*)");
    parity_err("(cond)");
    parity_err("(cond 1)");
    parity_err("(while)");
    parity_err("(when)");
    parity_err("(unless)");
}

#[test]
fn parity_err_module_require_keyword() {
    parity_err("(require net)");
    parity_err("(require io net)");
    parity_err("(require io)");
    parity_err("(module)");
    parity_err("(module 1 1)");
    parity_err("(module m (import 1) 1)");
    parity_err("(module m (export x 2) 1)");
    parity_err("(else 1)");
    parity_err("(import a)");
    parity_err("(syntax-rules () _)");
}

// ---- E1-α 边界（显式报错，不静默——§2.3 原则 4） ----

#[test]
fn boundary_define_syntax_reports_explicitly() {
    let mut t = SymbolTable::new();
    let forms = bootstrap_read("(define-syntax m (syntax-rules () ((m x) x)))", 0, &mut t)
        .expect("读取失败");
    let err = expand_program(&forms, 0, &mut t).expect_err("define-syntax 应显式报 E1-α 边界错误");
    assert!(err.message.contains("E1-α"), "实际错误：{}", err.message);
}

// ---- 行为面（自举产物端到端可执行） ----

#[test]
fn behavior_arith_and_closures() {
    behavior("((lambda (x) (+ x 1)) 41)");
    behavior("(let ((x 1) (y 2)) (+ x y))");
    behavior("(letrec ((f (lambda () 1))) (f))");
    behavior("(let* ((x 1) (y (+ x 1))) (+ x y))");
}

#[test]
fn behavior_cond_while_fib() {
    behavior("(cond (false 1) (true 2) (else 3))");
    behavior("(define i 0) (while (< i 10) (set! i (+ i 1))) i");
    behavior("(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 10)");
}

#[test]
fn behavior_inner_define_and_shadowing() {
    behavior("(define (f x) (define (g y) (+ x y)) (g 1)) (f 5)");
    behavior("(define x 10) ((lambda (x) x) 20)");
}
