//! I1 前段自举 Compiler parity 套件（Stage 2 批次 I / Task 42-b）。
//!
//! 验收门（plan §5a 42-b 行 + i1-incision-migration-design §4 S1 / §5
//! 门 A 基础组）：
//! - **结构 parity（门 A）**：自举 Compiler（compiler.krf，VM 上运行）
//!   与种子 Compiler（kerf-compiler/compile.rs，Rust）在同一 CoreExpr
//!   输入上——`BcProgram::bytecode_equal` 全结构一致（INC5：含
//!   protos/consts/global_refs/debug_spans——逐字节无灰区）；
//! - **错误 parity**：负例语料（函数体内 define 位置违规——D1 全局
//!   泄漏防护）消息 + Span(start,end) 逐字一致（E1 同口径边界：
//!   expansion_id 不参与错误判据——桥侧 ('err msg s e) 三字段契约，
//!   bootstrap_expander 同型先例）；
//! - **行为面**：自举编译段产物经 VM 执行 = 生产管线（自举前端 + 种子
//!   编译段）运行结果——编译段可执行性的端到端证明（语料镜像
//!   examples/usage 基础件 fib/closures/higher_order 核心语义——print/
//!   require 属 42-c 与 I/O 边界外，qbe_backend_tests 同型先例）；
//! - **确定性纪律**（§7/B8）：同输入两次自举编译字节一致；42-b 边界
//!   = parity 影子路径（生产未切换——module/require 臂属 42-c，显式
//!   边界断言）。
//!
//! 遵循条款：§9.4.3（正负例成对）、§7.1（集成验证 ≥3）、§2.3-11
//! （先实测禁臆测——全部断言经双实现实跑比对）、§21.3（门 A 为
//! 条件 2 的段级前置判据）。

use std::rc::Rc;

use kerf_compiler::compile_module as seed_compile;
use kerf_core::CoreExpr;
use kerf_driver::bootstrap::read_source as bootstrap_read;
use kerf_driver::bootstrap_compiler::compile_module as bootstrap_compile;
use kerf_driver::bootstrap_expander::expand_program as bootstrap_expand;
use kerf_driver::builtins::register_globals;
use kerf_driver::capability::IoGrant;
use kerf_driver::run_source;
use kerf_expander::{expand_program as seed_expand, ExpandCtxt};
use kerf_runtime::Heap;
use kerf_syntax::SymbolTable;
use kerf_vm::run_program;

// ---- parity 走查 ----

/// 前段种子管线：源 → Stx → CoreExpr（两编译路径共享同一输入——
/// 编译段是唯一被测变量；file_id = 0 统一（span 全等判据的表参数化））。
fn seed_front(src: &str) -> (Vec<Rc<CoreExpr>>, SymbolTable) {
    let mut table = SymbolTable::new();
    let forms = bootstrap_read(src, 0, &mut table).expect("读取失败");
    let mut ectx = ExpandCtxt::new(table);
    let core = seed_expand(&forms, &mut ectx).expect("展开失败（语料应为正例）");
    (core, ectx.table)
}

/// 门 A parity 断言：同一 CoreExpr 输入，种子 vs 自举编译——
/// `bytecode_equal` 全结构一致（INC5）。
fn parity(src: &str) {
    let (core, mut table) = seed_front(src);
    let a = seed_compile(&core).expect("种子编译失败（语料应为正例）");
    let b = bootstrap_compile(&core, 0, &mut table)
        .unwrap_or_else(|e| panic!("自举编译失败（src: {:?}）：{}", src, e.message));
    assert!(
        a.bytecode_equal(&b),
        "parity 失败（src: {:?}）\n== 种子 ==\n{}\n== 自举 ==\n{}",
        src,
        kerf_compiler::disassemble_program(&a, &|s| table.name(s).to_string()),
        kerf_compiler::disassemble_program(&b, &|s| table.name(s).to_string()),
    );
}

/// 负例 parity 断言：双路径均报错且消息 + Span(start,end) 逐字一致
/// （E1 同口径边界：expansion_id 不参与错误判据）。
fn parity_err(src: &str) {
    let (core, mut table) = seed_front(src);
    let ea = seed_compile(&core).expect_err("种子应报错（语料应为负例）");
    let eb = bootstrap_compile(&core, 0, &mut table).expect_err("自举应报错（语料应为负例）");
    assert_eq!(ea.message, eb.message, "错误消息不一致（src: {:?}）", src);
    assert_eq!(
        (ea.span.start, ea.span.end),
        (eb.span.start, eb.span.end),
        "错误 Span 不一致（src: {:?}）：{}..{} vs {}..{}",
        src,
        ea.span.start,
        ea.span.end,
        eb.span.start,
        eb.span.end
    );
}

/// 行为面断言：自举前端（读+展开）+ 自举编译段产物经 VM 执行，与
/// run_source（生产管线 = 自举前端 + 种子编译段）结果一致——编译段
/// 是唯一被测变量。
fn behavior(src: &str) {
    let expected = run_source(src, "behavior.krf").expect("生产管线失败");
    let mut t = SymbolTable::new();
    let forms = bootstrap_read(src, 0, &mut t).expect("读取失败");
    let core = bootstrap_expand(&forms, 0, &mut t).expect("自举展开失败");
    let program = bootstrap_compile(&core, 0, &mut t).expect("自举编译失败");
    // 行为语料无 I/O（纯算术/控制流/闭包）——空授权（R9 fail-closed）
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

// ---- 正例 parity：字面量与常量池 ----

#[test]
fn parity_literals() {
    for src in [
        "42", "-7", "3.5", "-0.5", "\"hi\"", "\"\"", "true", "false", "nil",
    ] {
        parity(src);
    }
}

#[test]
fn parity_quote_symbols_and_pairs() {
    // 引号点对递归（设计 §4 S1 特化 case：先 car 后 cdr 对齐 MAKE_PAIR；
    // 注：Reader 不支持点对字面量语法——quote 产物 = 尾 nil 的序对链，
    // 嵌套点对经嵌套列表 datum 构造）
    for src in [
        "'sym",
        "(quote sym)",
        "'()",
        "(quote (1 2 3))",
        "'(a (b c) 2.5)",
        "'(1 (2 3) (4 (5 6)))",
        "'((1 2) (3 (4 5)))",
        "'(sym \"str\" 1 2.5 true nil)",
        "''x",
        "'(quote (x y))",
    ] {
        parity(src);
    }
}

#[test]
fn parity_const_pool_dedup() {
    // 常量池去重（§19.3 陷阱：(begin 1 1 1) 不膨胀——关联列表首插序镜像）
    parity("(begin 1 1 1)");
    parity("(begin \"a\" \"a\" 1 \"a\")");
    parity("(list 1 1.5 1 1.5)");
    parity("(begin 'x 'x 'y)");
}

// ---- 正例 parity：变量 / define / set! ----

#[test]
fn parity_vars_define_globals() {
    parity("x");
    parity("(define x 5)");
    parity("(define x 5) x");
    parity("(define x 5) (define y x) y");
    // 全局符号卫生回退基名（B10——编译期按原名发射 LoadGlobal）
    parity("(define x$hyg$3 7) x$hyg$3");
}

#[test]
fn parity_if_jumps() {
    parity("(if true 1 2)");
    parity("(if x 1 2)");
    parity("(if x 1)");
    parity("(if (= 1 2) 1 (if true 2 3))");
    parity("(if x (if y 1 2) (if z 3 4))");
    parity("(define (f n) (if (< n 2) n (+ (f (- n 1)) (f (- n 2)))) )");
}

#[test]
fn parity_begin() {
    parity("(begin)");
    parity("(begin 1)");
    parity("(begin 1 2 3)");
    parity("(begin 1 (begin 2 3) 4)");
    parity("(begin (if x 1 2) (define y 2) y)");
}

// ---- 正例 parity：Lambda / 闭包捕获（B3 最难段特化语料）----

#[test]
fn parity_lambda_basic() {
    parity("(lambda (x) x)");
    parity("((lambda (x) x) 5)");
    parity("(define (f x y) (+ x y)) (f 1 2)");
    parity("(lambda () 1)");
    parity("(lambda (a b c) (list a b c))");
}

#[test]
fn parity_lambda_nested_captures() {
    // 嵌套闭包捕获（设计 §4 S1 特化 case）
    parity("(define (f x) (lambda (y) (x y))) (f 3)");
    // 三层捕获链：最内层引用最外层形参（传递共享 Captured 源）
    parity(
        "(define (a x)
            (define (b y)
              (define (c z) (+ x (+ y z)))
              (c 1))
            (b 2))
         (a 3)",
    );
    // 同层双捕获（捕获序 = 自由变量首现序）
    parity("(define (f x y) (lambda () (list x y))) (f 1 2)");
    // 形参遮蔽（shadow-pop 口径逐位镜像）
    parity("((lambda (x) ((lambda (x) x) 3)) 4)");
    // 捕获与全局引用并存（Global 零捕获 + free_vars 并集口径）
    parity("(define (f x) (lambda (y) (+ x g))) (define g 1) (f 2)");
}

#[test]
fn parity_lambda_set_bang_paths() {
    // set! 三路径：局部 / 捕获 / 全局（解析与 VarRef 同一口径）
    parity("(define (f) (let ((x 1)) (set! x 5) x)) (f)");
    // 捕获写传播（共享可变单元——词法闭包语义）
    parity(
        "(define (make-counter)
            (let ((n 0))
              (lambda () (set! n (+ n 1)) n)))
         (define c (make-counter)) (c) (c) (c)",
    );
    // 全局 set!
    parity("(define x 1) (set! x 9) x");
    // set! 值先入栈 DUP（栈平衡不变式 1）
    parity("(define (f x) (+ (set! x 3) x)) (f 1)");
}

// ---- 正例 parity：App / 尾位穿线（B9 TCO 必含面）----

#[test]
fn parity_app_eval_order() {
    // 求值顺序契约（§19.5 不变式 3）：被调函数先，参数从左到右
    parity("(f a b c)");
    parity("(f (g 1) (h 2))");
    parity("(+ (* 2 3) (- 10 4))");
}

#[test]
fn parity_tail_position_threading() {
    // 深尾递归（TailCall 位——B9；编译期 parity，不执行）
    parity("(define (c n) (if (= n 0) 0 (c (- n 1)))) (c 5)");
    // begin 末项继承尾位（begin 尾调用 = 尾调用——scheme 语义）
    parity("(define (f n) (begin 1 (f (- n 1))))");
    // 非尾位（体中表达式位置 → Call）
    parity("(define (g n) (+ 1 (g (- n 1))))");
    // if 两臂继承尾位 + let 糖脱装嵌套（tco_tests 语料同型）
    parity("(define (h n acc) (if (= n 0) acc (h (- n 1) (+ acc 1)))) (let ((r (h 10 0))) r)");
    // 相互尾递归
    parity("(define (ev? n) (if (= n 0) 1 (od? (- n 1)))) (define (od? n) (if (= n 0) 0 (ev? (- n 1)))) (ev? 6)");
}

#[test]
fn parity_deep_nesting_recursion() {
    // 深嵌套表达式（krf 侧编译递归深度压力——100 层算术嵌套）
    let mut src = String::from("(define (f x) ");
    for _ in 0..100 {
        src.push_str("(+ 1 ");
    }
    src.push('x');
    for _ in 0..100 {
        src.push(')');
    }
    src.push_str(") (f 1)");
    parity(&src);
}

#[test]
fn parity_deterministic_double_compile() {
    // 确定性纪律（§7/B8）：同输入两次自举编译字节一致
    let (core, mut table) = seed_front("(define (f x) (if (= n 0) 0 (f (- n 1)))) (f 5)");
    let a = bootstrap_compile(&core, 0, &mut table).expect("第一次自举编译失败");
    let b = bootstrap_compile(&core, 0, &mut table).expect("第二次自举编译失败");
    assert!(
        a.bytecode_equal(&b),
        "两次自举编译字节不一致（入口复位纪律破坏）"
    );
    // 空程序（main_span 哑元路径）
    parity("");
    parity("(begin)");
}

// ---- 负例 parity（消息 + Span 逐字一致——D1 全局泄漏防护）----

#[test]
fn parity_err_define_position() {
    // 函数体内表达式位置的 define（展开器仅提升体头部 define——
    // begin/if 表达式位漏入者由编译段结构化拒绝，两路径同报 E0003 口径）
    parity_err("(define (f x) (begin (define y 2) x))");
    parity_err("(lambda (x) (begin 1 (define y 2)))");
    parity_err("(lambda (x) (if true (define y 2) x))");
    parity_err("(define (f x) (let ((y 1)) (begin (define z 2) y)))");
}

#[test]
fn parity_err_module_require_boundary() {
    // 42-c 边界显式断言：module/require 臂未迁移——自举侧报边界错误
    // （种子侧正常编译——边界不对称是 42-b 声明的影子路径边界，非缺陷）
    let (core, mut table) = seed_front("(require io read)");
    let seed_ok = seed_compile(&core).is_ok();
    let boot = bootstrap_compile(&core, 0, &mut table);
    assert!(seed_ok, "种子应正常编译 require（零字节码臂）");
    assert!(
        boot.is_err(),
        "自举应报 42-c 边界错误（require 臂未迁移——显式失败不静默）"
    );
    let (core_m, mut table_m) = seed_front("(module m (define x 1) x)");
    let boot_m = bootstrap_compile(&core_m, 0, &mut table_m);
    assert!(
        boot_m.is_err(),
        "自举应报 42-c 边界错误（module 臂未迁移——显式失败不静默）"
    );
}

// ---- 行为面（examples/usage 基础件核心语义——设计 §4 S1 集成验证）----

#[test]
fn behavior_fib() {
    // fib.krf 核心语义（print/require 属 I/O 边界外——qbe_backend_tests
    // 同型先例；fib 12 = 144 与 CLI 冒烟同口径）
    behavior("(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 12)");
}

#[test]
fn behavior_closures() {
    // closures.krf 核心语义：计数器共享单元独立性 + 加法器捕获
    behavior(
        "(define (make-counter)
            (let ((n 0))
              (lambda () (set! n (+ n 1)) n)))
         (define a (make-counter))
         (define b (make-counter))
         (a) (a) (a)
         (b)
         (list (a) (b))",
    );
    behavior("(define (make-adder n) (lambda (x) (+ x n))) (define add5 (make-adder 5)) (add5 37)");
}

#[test]
fn behavior_higher_order() {
    // higher_order.krf 核心语义：map/filter 经显式递归 + 闭包实参
    behavior(
        "(define (map f lst)
            (if (null? lst)
                nil
                (cons (f (car lst)) (map f (cdr lst)))))
         (define (filter pred lst)
            (cond ((null? lst) nil)
                  ((pred (car lst)) (cons (car lst) (filter pred (cdr lst))))
                  (else (filter pred (cdr lst)))))
         (define (even? n) (= (mod n 2) 0))
         (map (lambda (x) (* x x)) (quote (1 2 3 4 5)))",
    );
    behavior("(define (map f lst) (if (null? lst) nil (cons (f (car lst)) (map f (cdr lst))))) (map (lambda (x) (+ x 1)) '(1 2 3))");
}

#[test]
fn behavior_deep_tail_recursion() {
    // 深尾递归端到端（TailCall 帧复用——自举编译段产物在 VM 上执行；
    // 100_000 层非 TCO 将触帧上限——B9 必含面的行为级证明）
    behavior("(define (c n) (if (= n 0) 'done (c (- n 1)))) (c 100000)");
}
