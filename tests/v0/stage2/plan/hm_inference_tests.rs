//! 批次 H（r18）——H4 HM 推断 PoC 集成测试（40-e / 38-d 设计 §7 落地）。
//!
//! **门（D8 PoC 阶段 = 离线并行验证）**：
//! - **超集门**：R1-R8 负例矩阵全检出（每程序：check_program 报 →
//!   hm_check_program 亦报）；
//! - **零误报门**：examples 六件套 + 动态边界语料（typecheck_tests
//!   dynamic_programs_clean 同源）→ 0 诊断；
//! - **HM 增值面**（R1-R8 静默放过、38-d §2.2 四类缺口的检出证明）：
//!   ①用户 lambda 实参类型错；②car/cdr 元素类型（Pair(τ,τ) 构造子）；
//!   ③分支分歧；④递归函数体内元数/域错；
//! - **裁定负例**：occurs check（含自应用）≥3 + 值限制（set! 目标 /
//!   App 结果不泛化）≥2 + 多错误 Span 序。
//!
//! **旗标期口径（D8 阶段 2——r29 / 50-a / MUV 48-c）**：`kerf check`
//! 生产判定面已切换为 `hm_check_program`（driver.rs 两入口）——本套件
//! 的离线入口（`hm(src)`）与生产入口同源同判定；`r18(src)`（R1-R8）
//! 保留为**回归基线断言**（超集门参照侧——双面检出纪律）。生产入口的
//! 旗标期门（超集门 + 零误报门 + 契约重定义断言锚）见
//! typecheck_tests.rs 尾部旗标期判定面组。
//!
//! 实现注记（验收口径对齐——R4 代码为准）：38-d 设计 §7 验收行预期
//! fib : Int→Int；实现实测 **fib : (num → num)**——n 的全部用点均为
//! 数值域约束（`<`/`-`/递归传参），格合一最小解 = Num（Int 无锚点）。
//! 关键验收属性维持：**非 Dynamic**（递归形状经 α_f 合一成功推断——
//! 缺口④兑现）。设计文档随本测试同步回写。

use kerf_compiler::check_program;
use kerf_compiler::hm::{hm_check_program, HmReport};
use kerf_driver::builtins::builtin_sigs;
use kerf_driver::compile_source;

/// 前端编译 + HM 推断（PoC 离线入口）。
fn hm(src: &str) -> HmReport {
    let out = compile_source(src, "<hm-test>").expect("前端编译失败（测试源应为良构）");
    let mut table = out.table;
    let sigs = builtin_sigs(&mut table);
    hm_check_program(&out.core, &sigs, &table)
}

/// R1-R8 基线（超集门的参照侧）。
fn r18(src: &str) -> usize {
    let out = compile_source(src, "<hm-superset>").expect("前端编译失败");
    let mut table = out.table;
    let sigs = builtin_sigs(&mut table);
    check_program(&out.core, &sigs, &table).len()
}

fn assert_clean(src: &str) {
    let r = hm(src);
    assert!(
        r.diags.is_empty(),
        "预期 0 诊断，实际 {} 条：{:?}\n程序：{}",
        r.diags.len(),
        r.diags
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>(),
        src
    );
}

fn assert_diag(src: &str, msg: &str) {
    let r = hm(src);
    assert!(
        r.diags.iter().any(|d| d.message.contains(msg)),
        "未检出「{}」；实际 {} 条：{:?}\n程序：{}",
        msg,
        r.diags.len(),
        r.diags
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>(),
        src
    );
}

// ---------------------------------------------------------------------------
// 零误报门（examples 六件套 + 动态边界语料）
// ---------------------------------------------------------------------------

#[test]
fn examples_six_piece_zero_diag() {
    for f in [
        "fib.krf",
        "closures.krf",
        "higher_order.krf",
        "macros.krf",
        "gc_stress.krf",
        "io.krf",
    ] {
        let src = std::fs::read_to_string(format!("examples/usage/{}", f))
            .unwrap_or_else(|e| panic!("读取示例 {} 失败：{}", f, e));
        let r = hm(&src);
        assert!(
            r.diags.is_empty(),
            "示例 {} 预期 0 诊断，实际 {} 条：{:?}",
            f,
            r.diags.len(),
            r.diags
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn dynamic_boundary_corpus_clean() {
    // typecheck_tests::dynamic_programs_clean 同源语料（零误报门）
    assert_clean("(define (f x) (if x 1 2)) (f true)");
    assert_clean("(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 5)");
    assert_clean("(require io write) (define x 42) (print x)");
    assert_clean("(+ 1 2.5)");
    assert_clean("(< 1 2.0)");
    assert_clean("(head '(1 2 3))");
    assert_clean("(define (head x) 42) (head 5)");
    assert_clean(
        "(require io write) (define x 1) (set! x (+ x 1)) (begin (print x) (if (is-nil nil) x 2))",
    );
    assert_clean("(if (is-nil nil) 1 2)");
    assert_clean("(if (eq 'a 'a) 1 2)");
    assert_clean("(string-append \"a\" \"b\")");
    assert_clean("(string-to-symbol \"foo\")");
    assert_clean("(symbol-to-string 'foo)");
    assert_clean("(= \"a\" \"b\")");
}

#[test]
fn numeric_tower_and_set_join_clean() {
    // 数值塔格合一（Int/Float/Num 互匹——P0-3 专项矩阵子集）
    assert_clean("(+ 1 2.5 3)");
    assert_clean("(- 2.5 1)");
    assert_clean("(* 1.5 2.0 3)");
    // D3 join 保守契约：异型赋值 → Dynamic 降级零诊断
    assert_clean("(define x 1) (set! x \"foo\") (string-append x \"!\")");
    assert_clean("(define x 1) (set! x 2.5) (+ x 1)");
}

// ---------------------------------------------------------------------------
// 验收面：fib 推断为 (num → num)（非 Dynamic——缺口④递归推断兑现）
// ---------------------------------------------------------------------------

#[test]
fn fib_infers_num_to_num_not_dynamic() {
    let src = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 12)";
    let r = hm(src);
    assert!(
        r.diags.is_empty(),
        "零诊断：{:?}",
        r.diags
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
    );
    let out = compile_source(src, "<hm-fib>").expect("编译失败");
    let mut table = out.table;
    let fib_sym = table.intern("fib");
    let ty = r
        .globals
        .get(&fib_sym)
        .unwrap_or_else(|| panic!("fib 应入全局绑定表"));
    assert_eq!(
        ty.render(),
        "(num → num)",
        "fib 推断类型（非 Dynamic = 递归 α_f 合一成功）"
    );
}

// ---------------------------------------------------------------------------
// 超集门：R1-R8 负例矩阵全检出（双检查器并行对照）
// ---------------------------------------------------------------------------

#[test]
fn superset_gate_r1_to_r8() {
    let corpus: &[&str] = &[
        // R1（if 条件非 bool）
        "(if 1 2 3)",
        "(if 1.5 2 3)",
        "(if \"s\" 2 3)",
        "(if nil 2 3)",
        "(if 'sym 2 3)",
        "(if true 1 (if nil 2 3))",
        // R2（算术非数值）
        "(+ 1 \"a\")",
        "(- 1 \"a\")",
        "(* \"a\" 2)",
        "(/ nil 2)",
        "(mod true 2)",
        "(+ 1 2 \"a\" 4)",
        // R3（比较域）
        "(< 1 \"a\")",
        "(> 1 \"a\")",
        "(= 1 2 \"s\")",
        // TD-011 r24：全字符串排序链已合法（码点序）——换混串链锚 R3
        "(< \"a\" 1)",
        "(< 3 1 \"a\")",
        // R4（not 非 bool）
        "(not 1)",
        "(not \"s\")",
        // R5（car/cdr 非 pair）
        "(head 5)",
        "(tail \"s\")",
        // R6（不可调用）
        "(1 2 3)",
        "(\"s\" 1)",
        // R7（元数）
        "((lambda (x) x) 1 2)",
        "((lambda (x y) x) 1)",
        "(head 1 2)",
        // R8（串/符号族）
        "(string-append 1 \"a\")",
        "(string-to-symbol 5)",
        "(symbol-to-string 7)",
    ];
    for src in corpus {
        let r_count = r18(src);
        let hm_count = hm(src).diags.len();
        assert!(r_count >= 1, "参照侧（R1-R8）未检出——语料失效：{}", src);
        assert!(
            hm_count >= 1,
            "超集门失败：R1-R8 检出 {} 条而 HM 0 条\n程序：{}",
            r_count,
            src
        );
    }
}

// ---------------------------------------------------------------------------
// HM 增值面（38-d §2.2 四类缺口的检出证明——R1-R8 静默放过）
// ---------------------------------------------------------------------------

#[test]
fn gap1_user_lambda_arg_type_error_detected() {
    // 缺口①：用户 lambda 实参类型错（保守检查：参数 Unknown 放过）
    assert_diag("((lambda (x) (+ x 1)) \"foo\")", "需要数值");
    // 非数值域参数（bool 传入算术位）
    assert_diag("((lambda (x) (+ x 1)) true)", "需要数值");
}

#[test]
fn gap2_car_element_type_inferred() {
    // 缺口②：head 元素类型（Pair(τ,τ) 构造子推断——元素 Bool 入算术位）
    let src = "(define x (head (cons true nil))) (+ x 1)";
    assert_diag(src, "需要数值");
    // 正例：数值元素参与算术零诊断
    assert_clean("(define x (head (cons 1 nil))) (+ x 1)");
}

#[test]
fn gap3_branch_disagreement_detected() {
    // 缺口③：分支类型分歧（变元参与 → 约束合一）
    // f 返回 if 分支两支（变元路径）：一支数值域约束，一支字符串
    let src = "(define (f c) (if c (+ 1 c) (string-append c \"\"))) (f 1)";
    assert_diag(src, "类型不一致");
}

#[test]
fn gap4_recursive_arity_and_domain_detected() {
    // 缺口④：递归函数体内元数错（自引用经 α_f 合一——非 Unknown）
    assert_diag(
        "(define (fib n) (if (< n 2) n (+ (fib (- n 1) 1) (fib (- n 2)))))",
        "参数数量不匹配",
    );
    // 递归域错：自引用参数传非数值（n 经 < 数值锚 ~Num 与 string-append
    // Str 域约束冲突 → 类型不一致）
    assert_diag(
        "(define (g n) (if (< n 2) n (g (string-append n \"x\"))))",
        "类型不一致",
    );
}

// ---------------------------------------------------------------------------
// 裁定负例：occurs check（D5）≥3（含自应用）
// ---------------------------------------------------------------------------

#[test]
fn occurs_check_negatives() {
    // ① 自应用（参数变元参与自身约束——经典 (λx. x x)）
    assert_diag("(define (f x) (x x))", "无法构造无限类型");
    // ② 自引用传播（f 自身在参数位）
    assert_diag("(define (f x) (f f))", "无法构造无限类型");
    // ③ 变元中介的嵌套自应用（(x x) 结果再应用——α 经两重箭头自含）
    assert_diag("(define (f x) ((x x) 1))", "无法构造无限类型");
}

// ---------------------------------------------------------------------------
// 裁定负例：值限制（D2）≥2
// ---------------------------------------------------------------------------

#[test]
fn value_restriction_negatives() {
    // ① App 结果不泛化：head 的元素类型 Mono 共享——两用点异型报错
    assert_diag(
        "(define f (head (cons (lambda (x) x) nil))) (f 1) (f true)",
        "类型不一致",
    );
    // ② set! 目标弱单态：Poly 绑定经 set! 后永久单态——两用点异型报错
    assert_diag(
        "(define id (lambda (x) x)) (set! id id) (id 1) (id true)",
        "类型不一致",
    );
    // 正例对照：未经 set! 的 lambda 绑定泛化（两域用点均合法）
    assert_clean("(define id (lambda (x) x)) (id 1) (id true)");
}

// ---------------------------------------------------------------------------
// D6 泛化载体：let 形状（App-of-Lambda 识别——用户手写同形同待遇）
// ---------------------------------------------------------------------------

#[test]
fn let_shape_generalization() {
    // 表面 let 糖（脱装为 App-of-Lambda）与用户手写同待遇
    assert_clean("(let ((id (lambda (x) x))) (+ (id 1) (id 2)))");
    assert_clean("((lambda (id) (+ (id 1) (id 2))) (lambda (x) x))");
    // 泛化实例化：let 绑定的 id 在 Int/Bool 两域用点均合法
    assert_clean("(let ((id (lambda (x) x))) (begin (id 1) (id true)))");
    // letrec 形状（D4）：lambda-RHS 泛化（体内两域用点合法）
    assert_clean("(letrec ((id (lambda (x) x))) (begin (id 1) (id true)))");
    // 递归 letrec：fact 自引用推断（非 Dynamic）
    let src = "(letrec ((fact (lambda (n) (if (= n 0) 1 (* n (fact (- n 1))))))) (fact 5))";
    assert_clean(src);
}

// ---------------------------------------------------------------------------
// D7 多错误：全量收集 + Span 序
// ---------------------------------------------------------------------------

#[test]
fn multi_error_collection_span_order() {
    let src = "(+ 1 \"a\") (if 2 1 3) (head 5) (string-append 1 \"b\")";
    let r = hm(src);
    assert!(
        r.diags.len() >= 4,
        "应 ≥4 条诊断，实际 {}：{:?}",
        r.diags.len(),
        r.diags
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
    );
    let starts: Vec<u32> = r.diags.iter().map(|d| d.primary_span.start).collect();
    let mut sorted = starts.clone();
    sorted.sort();
    assert_eq!(starts, sorted, "诊断应按 Span 起点升序");
    // 诊断码族：E0005
    for d in &r.diags {
        assert_eq!(d.code.map(|c| c.render()), Some("E0005".to_string()));
    }
}

// ---------------------------------------------------------------------------
// 保守边界维持（A4 渐进逃生舱 + A6 深度预算）
// ---------------------------------------------------------------------------

#[test]
fn dynamic_escape_not_forced() {
    // 未绑定引用（卫生符号/动态风格）不被推断面强制——零诊断
    assert_clean("(define (f x) (head x))");
    assert_clean("(print-unregistered-thing 1 2 3)");
    // 混型 set! 后降级 Dynamic：下游零约束
    assert_clean("(define x 1) (set! x true) (if x 1 2)");
}

#[test]
fn deep_nesting_within_budget_clean() {
    // 512 生成期预算内深嵌套零诊断（A6 语义原样迁移）
    let mut src = String::from("1");
    for _ in 0..250 {
        src = format!("(if true {} 2)", src);
    }
    assert_clean(&src);
}
