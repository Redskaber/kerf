//! 批次 G（r17）——QBE 后端 PoC 集成测试（38-c）。
//!
//! **覆盖面**（§9.4.3 正负比 ≥1:3——本地码路径专负例承载）：
//! - 正例：fib 递归端到端（IL 生成结构 + 本地码运行 144）、算术、
//!   值上下文 if（块参数 merge/phi 路径）、尾位 if、do、VM/native
//!   双路径一致性；
//! - 负例：fn 值位置（闭包边界）、define 非 fn、字符串/浮点/
//!   点对字面量、assign、module、未定义函数、arity 不匹配、自由变量、
//!   函数值一等传递；
//! - 工具链：qbe 查找链 + AOT 进程错误。
//!
//! 端到端测试依赖仓库内 `tools/qbe/bin/qbe`（38-b 落位）与系统 `cc`
//! ——缺失即失败（§3.1 不静默跳过口径）。

use std::rc::Rc;

use kerf_backend::{build_native, find_qbe, gen_il, lower_program, run_native};
use kerf_driver::compile_source;

/// 编译源码到 ANF（前端管线 + lowering）。
fn lower_src(src: &str) -> Result<kerf_backend::AnnotatedANF, kerf_backend::LowerError> {
    let out = compile_source(src, "<test>").expect("前端编译失败（测试源应为良构）");
    lower_program(&out.core, &out.table)
}

/// 编译源码 → QBE IL 文本。
fn il_of(src: &str) -> String {
    gen_il(&lower_src(src).expect("lowering 应成功"))
}

/// 端到端产物目录唯一化（并行测试互删勘误：共享 pid 目录 +
/// remove_dir_all 会互删他人产物——38-c 实测；原子计数器隔离）。
static TEST_DIR_SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// 完整 AOT 端到端：源码 → 本地可执行运行结果（exit code）。
fn native_exit_of(src: &str, stem: &str) -> i64 {
    let il = il_of(src);
    let qbe = find_qbe().expect("仓库内 QBE 应就位（sh scripts/qbe/setup.sh）");
    let seq = TEST_DIR_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let out_dir =
        std::env::temp_dir().join(format!("kerf-qbe-test-{}-{}", std::process::id(), seq));
    let arts = build_native(&il, &out_dir, stem, &qbe).expect("AOT 构建应成功");
    let r = run_native(&arts.exe_path).expect("运行应成功");
    let _ = std::fs::remove_dir_all(&out_dir); // 仅删自身唯一目录
    r
}

// ---------------------------------------------------------------------------
// 正例（≥5——端到端 + 结构断言）
// ---------------------------------------------------------------------------

#[test]
fn fib_end_to_end_native_code_matches_vm() {
    // §21.3 条件 3 锚点：首个非 VM 后端——fib(12) 递归本地码 = 144
    let src = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2)))))\n(fib 12)\n";
    // VM 侧
    let vm = kerf_driver::run_source(src, "<test>").expect("VM 路径应成功");
    let vm_val = match vm.value {
        kerf_vm::Value::Int(i) => i,
        ref other => panic!("VM 产物应为 Int，实为 {:?}", other),
    };
    assert_eq!(vm_val, 144);
    // native 侧（exit code 完整可见：144 < 256）
    assert_eq!(native_exit_of(src, "t_fib"), 144);
}

#[test]
fn fib_il_structure_contains_expected_ops() {
    // IL 结构断言（无进程依赖）：csltl 比较 + 递归 call + sub/add
    let il = il_of("(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2)))))\n(fib 12)\n");
    assert!(
        il.contains("function l $fib(l %t0)"),
        "函数签名（带返回类型）：\n{}",
        il
    );
    assert!(il.contains("csltl"), "比较指令：\n{}", il);
    assert!(il.contains("call $fib"), "递归调用：\n{}", il);
    assert!(il.contains("sub %t0, 1"), "递减实参：\n{}", il);
    assert!(il.contains("add"), "求和：\n{}", il);
    assert!(
        il.contains("export function l $main()"),
        "main 导出：\n{}",
        il
    );
    // 块标签系：entry @l0 + 分支 @l1/@l2
    assert!(il.contains("@l0"));
    assert!(il.contains("jnz"), "条件跳转：\n{}", il);
}

#[test]
fn arithmetic_end_to_end() {
    let src = "(* (+ 2 3) (- 10 4))\n";
    // 5 * 6 = 30
    assert_eq!(native_exit_of(src, "t_arith"), 30);
}

#[test]
fn value_context_if_merges_via_block_params() {
    // 值上下文 if：块参数 merge（QBE phi 等价物）——(- (if (< 1 2) 10 20) 5) = 5
    let src = "(- (if (< 1 2) 10 20) 5)\n";
    let il = il_of(src);
    // phi 形态：`%t3 =l phi @l1 %t1, @l2 %t2`（双来源同一指令）
    assert!(il.contains("phi @l1"), "phi 绑定应出现：\n{}", il);
    assert!(il.contains("@l2 %t"), "phi 第二来源应出现：\n{}", il);
    assert_eq!(native_exit_of(src, "t_ifval"), 5);
}

#[test]
fn begin_and_multiple_top_level_forms() {
    // do 序列 + 多顶层语句（中间值丢弃，尾值返回）
    let src = "(do (+ 1 2) (* 3 4))\n(+ 100 44)\n";
    assert_eq!(native_exit_of(src, "t_begin"), 144);
}

#[test]
fn not_and_eq_prim_arithmetic_domain() {
    // not（ceql x 0 翻译）+ eq 整数域 + 嵌套
    let src = "(if (not (eq 3 4)) 77 88)\n";
    assert_eq!(native_exit_of(src, "t_not"), 77);
}

#[test]
fn vm_native_consistency_on_multiple_programs() {
    // 双路径一致性采样（VM run vs native exit——整数值域）
    let cases: Vec<(&str, i64)> = vec![
        ("(+ 20 22)\n", 42),
        ("(if (>= 5 5) 1 2)\n", 1),
        ("(mod 100 7)\n", 2), // 100 mod 7 = 2
        ("(/ 100 5)\n", 20),
    ];
    for (src, expect) in cases {
        let vm = kerf_driver::run_source(src, "<t>").expect("VM 成功");
        let vm_val = match vm.value {
            kerf_vm::Value::Int(i) => i,
            ref other => panic!("VM 应产 Int：{:?}", other),
        };
        assert_eq!(vm_val, expect, "VM 值：{}", src);
        assert_eq!(
            native_exit_of(src, "t_consist"),
            expect,
            "native 值：{}",
            src
        );
    }
}

// ---------------------------------------------------------------------------
// 负例（≥7——PoC 边界与静态校验，显式错误非静默）
// ---------------------------------------------------------------------------

#[test]
fn negative_lambda_value_position_rejected() {
    let src = "((fn (x) x) 5)\n";
    let e = lower_src(src).expect_err("fn 值位置（被调者）应拒绝");
    assert!(e.message.contains("被调者"), "消息：{}", e.message);
}

#[test]
fn negative_define_non_lambda_rejected() {
    let src = "(define x 5)\n(x)\n";
    let e = lower_src(src).expect_err("define 非 fn 应拒绝");
    assert!(e.message.contains("define"), "消息：{}", e.message);
}

#[test]
fn negative_string_literal_rejected() {
    let src = "(+ 1 2)\n\"hello\"\n";
    let e = lower_src(src).expect_err("字符串字面量应拒绝");
    assert!(e.message.contains("字面量"), "消息：{}", e.message);
}

#[test]
fn negative_float_literal_rejected() {
    let src = "(+ 1 2.5)\n";
    let e = lower_src(src).expect_err("浮点字面量应拒绝");
    assert!(e.message.contains("字面量"), "消息：{}", e.message);
}

#[test]
fn negative_setbang_rejected() {
    let src = "(define (f x) (assign x 5) x)\n(f 1)\n";
    let e = lower_src(src).expect_err("assign 应拒绝");
    assert!(e.message.contains("assign"), "消息：{}", e.message);
}

#[test]
fn negative_module_form_rejected() {
    let src = "(module m (import) (export) (+ 1 2))\n";
    let e = lower_src(src).expect_err("module 应拒绝");
    assert!(e.message.contains("module"), "消息：{}", e.message);
}

#[test]
fn negative_undefined_call_rejected() {
    let src = "(undefined-fn 1 2)\n";
    let e = lower_src(src).expect_err("未定义调用应拒绝");
    assert!(e.message.contains("不是已定义函数"), "消息：{}", e.message);
}

#[test]
fn negative_arity_mismatch_rejected() {
    let src = "(define (f x y) (+ x y))\n(f 1)\n";
    let e = lower_src(src).expect_err("arity 不匹配应拒绝");
    assert!(e.message.contains("参数数"), "消息：{}", e.message);
}

#[test]
fn negative_free_variable_rejected() {
    let src = "(define (f x) (+ x y))\n(f 1)\n";
    let e = lower_src(src).expect_err("自由变量应拒绝");
    assert!(e.message.contains("自由变量"), "消息：{}", e.message);
}

#[test]
fn negative_builtin_io_out_of_poc() {
    // print/IO 类内置未进本地码 PoC（B1 登记——批次 I FFI 面承接）。
    // 注意：require 前缀使前端过能力门（E0006）——降级到 lowering 拒
    let src = "(require io write)\n(print 5)\n";
    let e = lower_src(src).expect_err("print 应拒绝");
    assert!(e.message.contains("print"), "消息：{}", e.message);
}

#[test]
fn negative_function_value_passing_rejected() {
    // 函数名作值传递（一等函数）未支持
    let src = "(define (f x) x)\n(f f)\n";
    let e = lower_src(src).expect_err("函数值传递应拒绝");
    assert!(
        e.message.contains("函数值") || e.message.contains("参数数"),
        "消息：{}",
        e.message
    );
}

// ---------------------------------------------------------------------------
// 结构/契约面（anf 摘要 + require 跳过 + 空程序）
// ---------------------------------------------------------------------------

#[test]
fn require_forms_are_skipped_in_lowering() {
    // 零运行时语义（与 VM 字节码口径一致——require 不产指令）
    let ir = lower_src("(require io write)\n(+ 1 2)\n").expect("require 应跳过");
    assert_eq!(ir.funcs.len(), 1); // 仅 main
                                   // main 体：单 call/add 语句 + Ret
    assert!(!ir.funcs[0].body.blocks.is_empty());
}

#[test]
fn empty_program_main_returns_zero() {
    let ir = lower_src("").expect("空程序应合法");
    assert_eq!(ir.funcs.len(), 1);
    assert!(ir.funcs[0].name == "main");
    let il = gen_il(&ir);
    assert!(il.contains("ret 0"), "空 main 返回 0：\n{}", il);
    assert_eq!(native_exit_of("", "t_empty"), 0);
}

#[test]
fn anf_fingerprint_is_content_addressed() {
    // 同程序同指纹；异程序异指纹（内容寻址口径——契约字段语义）
    let a = lower_src("(+ 1 2)\n").unwrap();
    let b = lower_src("(+ 1 2)\n").unwrap();
    let c = lower_src("(+ 1 3)\n").unwrap();
    assert_eq!(a.fingerprint, b.fingerprint);
    assert_ne!(a.fingerprint, c.fingerprint);
}

#[test]
fn nested_if_and_calls_compose() {
    // 嵌套 if（外层值上下文 + 内层尾位）+ 多函数组合
    let src =
        "(define (classify n)\n  (if (< n 0) (if (< n -10) 111 112) (+ n 100)))\n(classify -25)\n";
    assert_eq!(native_exit_of(src, "t_nested"), 111);
    let src2 =
        "(define (classify n)\n  (if (< n 0) (if (< n -10) 111 112) (+ n 100)))\n(classify 44)\n";
    assert_eq!(native_exit_of(src2, "t_nested2"), 144);
}

// ---------------------------------------------------------------------------
// 既有程序回归面（VM 全绿样本的 native 侧采样——fib 之外的组合）
// ---------------------------------------------------------------------------

#[test]
fn existing_examples_in_poc_domain_run_native() {
    // examples/usage/fib.krf 含 print（边界外）——PoC 域内改写为纯 fib
    let fib = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2)))))\n(fib 10)\n";
    assert_eq!(native_exit_of(fib, "t_fib10"), 55);
}

#[test]
fn lower_program_accepts_core_directly() {
    // API 形状：core + table 直接消费（与 compile_module 同源输入——
    // 前端产物即后端输入，目标中立原则的类型级延续）
    let out = compile_source("(+ 40 2)", "<t>").unwrap();
    let core: Vec<Rc<kerf_core::CoreExpr>> = out.core.clone();
    let ir = lower_program(&core, &out.table).unwrap();
    assert_eq!(ir.funcs.len(), 1);
    let il = gen_il(&ir);
    assert!(il.contains("add"), "原语算术应内联（非 call）：\n{}", il);
}
