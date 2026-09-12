//! 效应语言面集成测试（r25/42-f——批次 I 后段 M1-M5 验收）。
//!
//! **验收合同**（plan.md §5a 42-f 行 + effect-language-design §5 测试锚）：
//! - 正例 ≥6（设计锚逐条）；负例 ≥6（E0007/E0008/E0009 + 展开期形态
//!   E0002 族 + 运行时类型面）；正负比 ≥1:3 纪律（§9.4.3）；
//! - M3 双路径 ≥8（42-d 新口径：生产链（自举三段 + VM）vs 种子链
//!   （Rust 三段 + VM）——eval 退役后 resume 一致性的 T1 承载面）；
//! - M5 GC 六来源（活跃 continuation 帧——跨分配压力存活验证）。

use crate::common::{assert_int, dual_path_agrees, run, run_rendered};
use kerf_vm::Value;

// ---------------------------------------------------------------------------
// 正例（设计 §5 锚 1-6）
// ---------------------------------------------------------------------------

/// 锚 1：单 handler 单恢复——perform 挂起 → dispatch → resume 注入值。
#[test]
fn effect_single_handler_single_resume() {
    assert_int(
        "(handle add ((p k) (resume k (+ p 10))) (+ 1 (perform (cons 'add 5))))",
        16,
    );
}

/// 锚 2：嵌套 handle——内层非匹配 tag 逃逸到外层（R11-dispatch 上抛链）。
#[test]
fn effect_nested_handle_escape_to_outer() {
    assert_int(
        "(handle outer ((p k) (resume k p)) (handle inner ((q j) 42) (perform (cons 'outer 7))))",
        7,
    );
}

/// 锚 3：handle 内纯计算零效应——体完成值 = handle 表达式值（R11-return）。
#[test]
fn effect_pure_body_no_effect() {
    assert_int("(handle t ((p k) 0) (+ 40 2))", 42);
}

/// 锚 4：恢复后闭包环境一致性——挂起点局部变量经共享单元格跨 resume
/// 可见（continuation 三要素②：挂起点环境）。
#[test]
fn effect_resume_env_consistency() {
    // 挂起前 set! x=1；handler 恢复后 set! x=100——B 帧局部槽共享
    //（快照 clone 的是 Rc<GcCell>——同一单元格）：恢复体读 x=100
    assert_int(
        "(define (f) (define x 1) (+ (handle t ((p k) (resume k 0)) (begin (set! x 100) (perform (cons 't 0)) x)) x)) (f)",
        200,
    );
}

/// 锚 5：效应值先求值序（App 序契约 06 §1.3 A1——效应值表达式在 PERFORM
/// 之前完整求值；序经计数器副作用可观测）。
#[test]
fn effect_value_eval_order() {
    assert_int(
        "(define n 0) (define (bump) (begin (set! n (+ n 1)) n)) (handle t ((p k) (resume k (+ p n))) (+ 0 (perform (cons 't (bump)))))",
        2,
    );
    // n 在 perform 前已 bump 为 1；handler p=1（payload=(t . 1)）→
    // resume k (+ 1 1)=2；效应值先求值序使 handler 观察到副作用 ✓
}

/// 锚 6：TCO 尾调用穿透 handler 帧（D8——帧数语义不变；10 万深尾递归
/// 后 perform 恢复正常——帧复用与效应帧边界正交）。
#[test]
fn effect_tco_through_handler_frame() {
    assert_int(
        "(define (spin n) (if (= n 0) (perform (cons 's 99)) (spin (- n 1)))) (handle s ((p k) (resume k (+ p 1))) (spin 100000))",
        100,
    );
}

// ---------------------------------------------------------------------------
// M3：双路径一致（T1 新口径——42-d eval 退役后：生产链 vs 种子链，≥8）
// ---------------------------------------------------------------------------

/// 双路径组（种子/自举两编译链的效应行为一致——bytecode_equal 之外的
/// 行为面互查；含恢复/嵌套/逃逸负域/闭包捕获/多重效应串联形态）。
#[test]
fn effect_dual_path_behaviors() {
    let cases = [
        // 基础恢复
        "(handle add ((p k) (resume k (+ p 10))) (+ 1 (perform (cons 'add 5))))",
        // 嵌套逃逸
        "(handle outer ((p k) (resume k p)) (handle inner ((q j) 42) (perform (cons 'outer 7))))",
        // 纯体
        "(handle t ((p k) 0) (+ 40 2))",
        // 恢复值 = 闭包（continuation 恢复后调用闭包）
        "(define (mk a) (lambda (b) (+ a b))) (handle t ((p k) (resume k (mk 40))) ((perform (cons 't 0)) 2))",
        // handler 不恢复（吞效应给替代值）
        "(handle t ((p k) 99) (+ 1 (perform (cons 't 0))))",
        // 多重效应串联（同 handler 两次 perform——浅处理：第二次经
        // 嵌套 handle 结构承接）
        "(define (twice) (+ (handle t ((p k) (resume k 20)) (perform (cons 't 1))) (handle t2 ((p k) (resume k 21)) (perform (cons 't2 1))))) (twice)",
        // 捕获共享：handler 体引用外围闭包变量
        "(define base 40) (handle t ((p k) (resume k (+ p base))) (+ 1 (perform (cons 't 1))))",
        // 恢复点在深层调用链中（帧链快照 > 1 帧）
        "(define (g x) (+ x (perform (cons 't 5)))) (define (f y) (g y)) (handle t ((p k) (resume k (* p 2))) (f 3))",
    ];
    for (i, src) in cases.iter().enumerate() {
        assert!(
            dual_path_agrees(src),
            "双路径不一致（case {}：{:?}）",
            i,
            src
        );
    }
}

/// eval 域（树走路径——D7 逃逸映射）：无 resume 的 dispatch/逃逸程序
/// 在 eval 域与生产链值一致（M3 域内面——resume 哨兵域限见负例组）。
#[test]
fn effect_eval_domain_dispatch_agrees() {
    use kerf_driver::builtins::register_globals;
    use kerf_driver::capability::IoGrant;
    use kerf_expander::{expand_program, ExpandCtxt};
    use kerf_reader::read_source;
    use kerf_runtime::Heap;
    use kerf_syntax::SymbolTable;
    use kerf_vm::{eval_program, Env};

    let cases = [
        "(handle t ((p k) (+ p 1)) (perform (cons 't 41)))",
        "(handle t ((p k) 99) (+ 1 (perform (cons 't 0))))",
        "(handle outer ((p k) (+ p 2)) (handle inner ((q j) 0) (perform (cons 'outer 7))))",
    ];
    for (i, src) in cases.iter().enumerate() {
        // 生产链
        let vm = match run(src) {
            Ok(v) => v,
            other => panic!("生产链失败（case {}）：{:?}", i, other),
        };
        // eval 域（树走）
        let mut table = SymbolTable::new();
        let stx = read_source(src, 0, &mut table).expect("read");
        let core = {
            let mut ctx = ExpandCtxt::new(table.clone());
            expand_program(&stx, &mut ctx).expect("expand")
        };
        // eval 域全局环境：driver 内置注册（无 I/O 授权——fail-closed
        // 同生产口径；效应程序无需 io）
        let globals = register_globals(&mut table, &IoGrant::none());
        let env = Env::new();
        for (sym, val) in globals {
            env.define(sym, val);
        }
        let mut heap = Heap::new();
        let ev = eval_program(&core, &env, &mut heap)
            .map_err(|e| format!("{:?}（effect={:?}）", e.message, e.effect.is_some()))
            .unwrap_or_else(|e| panic!("eval 域失败（case {}）：{}", i, e));
        match (&vm, &ev) {
            (Value::Int(a), Value::Int(b)) => assert_eq!(a, b, "eval 域不一致（case {}）", i),
            other => panic!("值形态异常（case {}）：{:?}", i, other),
        }
    }
}

// ---------------------------------------------------------------------------
// 负例（设计 §5 锚 1-6 负域——E0007/E0008/E0009 + E0002 形态 + 类型面）
// ---------------------------------------------------------------------------

/// 断言运行失败且渲染含指定子串（结构化码文本锚）。
fn expect_run_err(src: &str, needle: &str) {
    let out = run_rendered(src);
    assert!(
        out.starts_with("<error:") && out.contains(needle),
        "期望错误含「{}」，实际：{}",
        needle,
        out
    );
}

/// E0007：未处理效应逃逸到顶层（D9——含 tag 名）。
#[test]
fn negative_unhandled_escape_e0007() {
    expect_run_err("(perform (cons 'oops 1))", "error[E0007]");
    expect_run_err("(perform (cons 'oops 1))", "未被任何 handler 处理");
}

/// E0008：continuation 二次恢复（D3 线性唯一性——含首次恢复位置追踪）。
#[test]
fn negative_double_resume_e0008() {
    expect_run_err(
        "(define k2 nil) (handle t ((p k) (begin (set! k2 k) (resume k 1))) (perform (cons 't 5))) (k2 2)",
        "error[E0008]",
    );
    expect_run_err(
        "(define k2 nil) (handle t ((p k) (begin (set! k2 k) (resume k 1))) (perform (cons 't 5))) (k2 2)",
        "二次恢复",
    );
}

/// E0009：resume 元数面（continuation 调用恰一实参——D4 调用形态约束）。
#[test]
fn negative_resume_arity_e0009() {
    expect_run_err(
        "(handle t ((p k) (k)) (perform (cons 't 1)))",
        "error[E0009]",
    );
}

/// 非 continuation 值被 resume（(resume 5 42) → 脱糖后 (5 42)——
/// 不可调用类型面归 E0004 通用族；效应语境可见于错误消息）。
#[test]
fn negative_resume_non_continuation() {
    expect_run_err("(resume 5 42)", "不可调用的值");
}

/// 展开期 E0002 族：handler 子句形态错（设计负例 4——((p r) result)
/// 形态校验）。
#[test]
fn negative_handler_clause_shape_e0002() {
    expect_run_err("(handle t (p 1) 2)", "error[E0002]");
    expect_run_err("(handle t ((p r)) 2)", "处理子句");
}

/// perform 非效应值（类型面——(tag . payload) 判据）。
#[test]
fn negative_perform_non_pair_payload() {
    expect_run_err("(handle t ((p k) (resume k p)) (perform 5))", "点对");
}

/// perform tag 位非符号（类型面）。
#[test]
fn negative_perform_non_symbol_tag() {
    expect_run_err(
        "(handle t ((p k) (resume k p)) (perform (cons 7 5)))",
        "符号",
    );
}

// ---------------------------------------------------------------------------
// M5：GC 六来源（活跃 continuation 帧——跨分配压力存活）
// ---------------------------------------------------------------------------

/// 挂起 payload（Pair）在 handler 体内大量分配（多轮 GC）后存活——
/// dispatch 帧 locals 的根扫描（第六来源）。
#[test]
fn gc_payload_survives_alloc_pressure() {
    assert_int(
        "(define (mk n) (if (= n 0) nil (cons n (mk (- n 1))))) (handle t ((p k) (begin (mk 50000) (resume k (+ (car p) 1)))) (+ 0 (perform (cons 't (cons 41 0)))))",
        42,
    );
}

/// continuation 装箱（Pair 的 ForeignBox 形态）跨 GC 压力存活——解箱
/// 恢复 + 恢复后行为正确（M5 装箱-解箱-存活全链）。
#[test]
fn gc_continuation_boxed_survives_alloc_pressure() {
    assert_int(
        "(define (mk n) (if (= n 0) nil (cons n (mk (- n 1))))) (define box nil) (handle t ((p k) (begin (set! box (cons k 41)) (mk 60000) (resume (car box) (+ (cdr box) 1)))) (+ 0 (perform (cons 't 0))))",
        42,
    );
}

// ---------------------------------------------------------------------------
// 48-b（r28/49-a）：效应静态收敛——typecheck.rs/hm.rs 子表达式遍历
//
// 深审 D3/D8「Unknown 放宽面」覆盖缺口闭环：Perform 效应值 / Handle
// 体与 handler 体入 R1-R8 与约束集检出域（结果类型维持 Unknown/Dynamic
// ——行多态属 Stage 3 边界如实）；绑定器 payload/resume 装订动态值
// （零误报保守契约维持）。
// ---------------------------------------------------------------------------

use kerf_compiler::check_program;
use kerf_compiler::hm::hm_check_program;
use kerf_driver::builtins::builtin_sigs;
use kerf_driver::compile_source;

/// 保守检查面（R1-R8 生产面——typecheck.rs 经 check_program）。
fn tc_msgs(src: &str) -> Vec<String> {
    let out =
        compile_source(src, "<effect-static>").unwrap_or_else(|e| panic!("前端编译失败：{}", e));
    let mut table = out.table;
    let sigs = builtin_sigs(&mut table);
    check_program(&out.core, &sigs, &table)
        .into_iter()
        .map(|d| d.message)
        .collect()
}

/// HM PoC 面（hm.rs——r18 离线入口同源）。
fn hm_msgs(src: &str) -> Vec<String> {
    let out =
        compile_source(src, "<effect-static-hm>").unwrap_or_else(|e| panic!("前端编译失败：{}", e));
    let mut table = out.table;
    let sigs = builtin_sigs(&mut table);
    hm_check_program(&out.core, &sigs, &table)
        .diags
        .into_iter()
        .map(|d| d.message)
        .collect()
}

/// 双面同检（超集纪律——r18 门 A 口径延续：tc 面报 → hm 面亦报）。
fn both_detect(src: &str, needle: &str) {
    let tc = tc_msgs(src);
    assert!(
        tc.iter().any(|m| m.contains(needle)),
        "tc 面未检出「{}」；实际 {:?}\n程序：{}",
        needle,
        tc,
        src
    );
    let hm = hm_msgs(src);
    assert!(
        hm.iter().any(|m| m.contains(needle)),
        "hm 面未检出「{}」；实际 {:?}\n程序：{}",
        needle,
        hm,
        src
    );
}

/// Perform 效应值静态违例（R2/R1——效应值表达式入检出域）。
#[test]
fn static_perform_effect_value_violations() {
    both_detect("(perform (+ 1 \"a\"))", "需要数值");
    both_detect("(perform (if \"x\" 1 2))", "if 条件需要 bool");
}

/// Handle handler 体静态违例（R5——子句体入检出域）。
#[test]
fn static_handler_body_violations() {
    both_detect("(handle t ((p k) (car 42)) 1)", "car 需要 pair");
    both_detect("(handle t ((p k) (cdr \"s\")) 1)", "cdr 需要 pair");
}

/// Handle 体静态违例（R1——被保护计算入检出域）+ 双体多错误收集。
#[test]
fn static_handle_body_violations() {
    both_detect("(handle t ((p k) k) (if 1 2 3))", "if 条件需要 bool");
    // handler 体（R2）+ handle 体（R1）各一处 → 双面多错误收集
    let src = "(handle t ((p k) (+ \"b\" 2)) (if 1 2 3))";
    let tc = tc_msgs(src);
    assert!(
        tc.iter().any(|m| m.contains("需要数值"))
            && tc.iter().any(|m| m.contains("if 条件需要 bool")),
        "tc 面期望双违例收集，实际 {:?}",
        tc
    );
    let hm = hm_msgs(src);
    assert!(hm.len() >= 2, "hm 面期望 ≥2 诊断，实际 {:?}", hm);
}

/// 零误报对照（保守契约维持——r25 六正例 + 绑定器动态用点 + M5 语料）。
#[test]
fn static_effect_zero_false_positive() {
    let corpus = [
        // 设计 §5 六正例（r25 同源语料——静态面零诊断）
        "(handle add ((p k) (resume k (+ p 10))) (+ 1 (perform (cons 'add 5))))",
        "(handle outer ((p k) (resume k p)) (handle inner ((q j) 42) (perform (cons 'outer 7))))",
        "(handle t ((p k) 0) (+ 40 2))",
        "(define (f) (define x 1) (+ (handle t ((p k) (resume k 0)) (begin (set! x 100) (perform (cons 't 0)) x)) x)) (f)",
        "(define n 0) (define (bump) (begin (set! n (+ n 1)) n)) (handle t ((p k) (resume k (+ p n))) (+ 0 (perform (cons 't (bump)))))",
        "(define (spin n) (if (= n 0) (perform (cons 's 99)) (spin (- n 1)))) (handle s ((p k) (resume k (+ p 1))) (spin 100000))",
        // 绑定器动态用点（payload Unknown/Dynamic 的算术与点对——零误报）
        "(handle t ((p k) (resume k (cons (car p) (cdr p)))) (perform (cons 't 1)))",
        "(handle t ((p k) (+ p 0)) (perform (cons 't 1)))",
    ];
    for (i, src) in corpus.iter().enumerate() {
        let tc = tc_msgs(src);
        assert!(
            tc.is_empty(),
            "tc 面误报（case {}）：{:?}\n程序：{}",
            i,
            tc,
            src
        );
        let hm = hm_msgs(src);
        assert!(
            hm.is_empty(),
            "hm 面误报（case {}）：{:?}\n程序：{}",
            i,
            hm,
            src
        );
    }
    // M5 效应语料（effect_stress.krf——深层递归 + 绑定器动态用点）
    let stress = std::fs::read_to_string("examples/usage/effect_stress.krf")
        .expect("读取 effect_stress.krf 失败");
    let tc = tc_msgs(&stress);
    assert!(tc.is_empty(), "tc 面对 effect_stress 误报：{:?}", tc);
    let hm = hm_msgs(&stress);
    assert!(hm.is_empty(), "hm 面对 effect_stress 误报：{:?}", hm);
}
