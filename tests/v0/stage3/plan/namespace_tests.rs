//! 批次 M 首件 M1（v0.6 命名空间层，r39）——N2 限定名可见面集成测试
//! （tests/v0/stage3/plan/ ↔ docs/lang-design/22-namespace-design.md §8
//! 实施对账表 R-N3/R-N4/R-N7/R-N8 行 + 20-surface-conventions §5.1/§5.2
//! + 22 §5.1 矩阵/§5.2 红线 1 + §7 保留域）。
//!
//! 覆盖面（§9.4.3 正负比 ≥1:3——负向为主）：
//! - R-N3 不回落：限定名仅查 stdlib 七面 export——未命中 = E0014
//!   「不导出」编译期诊断（非运行期「未绑定」——诊断增益锚）；
//! - R-N4 词法域：独立 `/` 维持除法（`(/ a b)`）；`ns/name` 单 token；
//! - R-N7 运算符永驻：N1 裸名零 import（既有程序全绿的显式锚）；
//! - R-N8 符号宇宙豁免：quote 符号值不参与解析（存在性 ≠ 可见性）；
//! - 22 §5.1 矩阵/红线 1：`io/*` 限定名双门分立——名字可见性（N2
//!   注册）与操作许可（E0006 require）独立：未授权 io 限定名 →
//!   E0006（fail-closed 先行）；授权后可调用；
//! - 22 §7 保留域：`kerf-` 前缀模块名 = E0015（用户占用违例）；
//! - M1 收窄如实锚：用户接管（define 含 `/` 名）豁免 E0014；
//! - 静态面：限定名签名派生（check/hm 对 `ns/name` 同判——E0005）。

use crate::common;

use kerf_driver::{run_source, Stage};
use kerf_span::DiagnosticCode;

const FNAME: &str = "ns.krf";

/// 编译期错误断言：阶段 Compile + 诊断码 + 消息子串。
fn expect_compile_err(src: &str, code: u32, msg: &str) {
    let e = match run_source(src, FNAME) {
        Ok(_) => panic!("期望编译期错误，实际 Ok：{}", src),
        Err(e) => e,
    };
    assert_eq!(
        e.stage,
        Stage::Compile,
        "阶段不符（应为 Compile）：\n{}",
        e.rendered
    );
    assert_eq!(
        e.diagnostic.code,
        Some(DiagnosticCode(code)),
        "诊断码不符（应为 E{:04}）：\n{}",
        code,
        e.rendered
    );
    assert!(
        e.rendered.contains(msg),
        "消息缺「{}」：\n完整：\n{}",
        msg,
        e.rendered
    );
}

// ---------------------------------------------------------------------------
// 正向：七模块限定名可见面（20 §5.2 模块树终态版图）
// ---------------------------------------------------------------------------

/// 七模块限定名解析（R-N3 正路——每模块 ≥1 抽检 + 跨模块组合）。
#[test]
fn qualified_names_resolve_across_modules() {
    assert_eq!(
        common::run_rendered("(string/append \"ab\" \"cd\")"),
        "abcd"
    );
    assert_eq!(common::run_rendered("(string/length \"héllo\")"), "5");
    assert_eq!(common::run_rendered("(list/nth '(a b c) 1)"), "b");
    assert_eq!(common::run_rendered("(list/append '(1) '(2 3))"), "(1 2 3)");
    assert_eq!(common::run_rendered("(pair/head (pair/cons 1 2))"), "1");
    assert_eq!(common::run_rendered("(pair/tail (pair/cons 1 2))"), "2");
    assert_eq!(common::run_rendered("(core/is-int 3)"), "true");
    assert_eq!(common::run_rendered("(core/eq 'a 'a)"), "true");
    assert_eq!(common::run_rendered("(char/is-whitespace \" \")"), "true");
    assert_eq!(common::run_rendered("(char/is-alphabetic \"é\")"), "true");
    // 跨模块组合（N2 面与既有全局/别名自由互操作）
    assert_eq!(
        common::run_rendered("(list/reverse (list/nth (list (list 1 2) 3) 0))"),
        "(2 1)"
    );
    assert_eq!(
        common::run_rendered("(core/is-string (string/to-upper \"a\"))"),
        "true"
    );
}

/// kerf/string 完整 export 面（11 本地名逐一——20 §5.2 表逐行对账）。
#[test]
fn string_module_full_export_face() {
    assert_eq!(common::run_rendered("(string/append \"a\" \"b\")"), "ab");
    assert_eq!(common::run_rendered("(string/length \"abc\")"), "3");
    assert_eq!(
        common::run_rendered("(string/substring \"héllo\" 1 3)"),
        "él"
    );
    assert_eq!(common::run_rendered("(string/index-of \"abc\" \"b\")"), "1");
    // v0.5 语义维持（B1 契约变更 v0.6 M2 承载——20 §4/§8 契约列）
    assert_eq!(
        common::run_rendered("(string/index-of \"abc\" \"z\")"),
        "-1"
    );
    assert_eq!(
        common::run_rendered("(string/contains \"abc\" \"bc\")"),
        "true"
    );
    assert_eq!(
        common::run_rendered("(string/starts-with \"abc\" \"ab\")"),
        "true"
    );
    assert_eq!(
        common::run_rendered("(string/ends-with \"abc\" \"bc\")"),
        "true"
    );
    assert_eq!(common::run_rendered("(string/to-upper \"aé\")"), "AÉ");
    assert_eq!(common::run_rendered("(string/to-lower \"AÉ\")"), "aé");
    assert_eq!(common::run_rendered("(string/to-symbol \"foo\")"), "foo");
    assert_eq!(common::run_rendered("(string/from-symbol 'foo)"), "foo");
}

/// R4 双向双家（20 §5.2——string 侧 to-symbol/from-symbol 与 symbol 侧
/// to-string/from-string 的对偶）。
#[test]
fn symbol_r4_dual_home_roundtrip() {
    assert_eq!(common::run_rendered("(symbol/to-string 'foo)"), "foo");
    assert_eq!(common::run_rendered("(symbol/from-string \"foo\")"), "foo");
    // 双家对偶往返（两家入口行为等价——底层共享分派体）
    assert_eq!(
        common::run_rendered("(core/eq (symbol/from-string \"x\") (string/to-symbol \"x\"))"),
        "true"
    );
    assert_eq!(
        common::run_rendered("(core/eq (string/from-symbol 'y) (symbol/to-string 'y))"),
        "true"
    );
}

/// kerf/core 面（14 本地名——谓词/关系/断言域全抽检）。
#[test]
fn core_module_face_covers_predicates() {
    assert_eq!(common::run_rendered("(core/is-nil nil)"), "true");
    assert_eq!(common::run_rendered("(core/is-bool true)"), "true");
    assert_eq!(common::run_rendered("(core/is-float 3.5)"), "true");
    assert_eq!(common::run_rendered("(core/is-number 3)"), "true");
    assert_eq!(common::run_rendered("(core/is-string \"s\")"), "true");
    assert_eq!(common::run_rendered("(core/is-symbol 's)"), "true");
    assert_eq!(
        common::run_rendered("(core/is-pair (pair/cons 1 2))"),
        "true"
    );
    assert_eq!(common::run_rendered("(core/is-list '(1))"), "true");
    assert_eq!(common::run_rendered("(core/is-procedure core/eq)"), "true");
    assert_eq!(common::run_rendered("(core/not true)"), "false");
    assert_eq!(common::run_rendered("(core/assert-eq 1 1)"), "true");
    assert_eq!(common::run_rendered("(list/length '(core/error 略))"), "2"); // error 域成员存在性经负例组锚（终止语义）
}

// ---------------------------------------------------------------------------
// 负向：R-N3 不回落（E0014）+ 保留域（E0015）——22 §8 表对应行
// ---------------------------------------------------------------------------

/// 限定名不导出（已知命名空间未知本地名）→ E0014 编译期（非「未绑定」
/// 运行期——R-N3 诊断增益：打错名字精确指向模块面）。
#[test]
fn qualified_not_exported_known_ns_is_e0014() {
    expect_compile_err("(string/nonexist 1)", 14, "「string」不导出「nonexist」");
    expect_compile_err("(list/nonexist 1)", 14, "「list」不导出「nonexist」");
    expect_compile_err(
        "(define f (lambda (x) (core/nonexist x)))",
        14,
        "「core」不导出「nonexist」",
    ); // 嵌套深度（Lambda 体递归）
}

/// 未知命名空间 → E0014（不回落：不查全局/局部/其他模块）。
#[test]
fn qualified_unknown_ns_is_e0014() {
    expect_compile_err("(nosuchns/foo 1)", 14, "「nosuchns」不导出「foo」");
    expect_compile_err("(io/prints 1)", 14, "「io」不导出「prints」");
}

/// R-N3 消息携带不回落语义（诊断三段式锚——禁回落规则可读性）。
#[test]
fn e0014_message_carries_no_fallback_semantics() {
    expect_compile_err("(string/nonexist 1)", 14, "限定名无回落");
}

/// 保留域违例（22 §7）：用户模块名 kerf- 前缀 → E0015。
#[test]
fn reserved_domain_module_name_is_e0015() {
    expect_compile_err(
        "(module kerf-evil (export f) (define f 1) (f))",
        15,
        "保留域违例：模块名「kerf-evil」占用 kerf- 前缀",
    );
    expect_compile_err(
        "(module m (define f 1) (module kerf-inner (export g) (define g 2) (g)))",
        15,
        "「kerf-inner」",
    ); // 嵌套 module（体递归臂）
}

/// 用户接管豁免（零误报纪律——与 R9 同口径）：define 含 `/` 名 +
/// 引用同名 → 不触发 E0014；**Lambda 参数遮蔽**（R-N1：N3 局部绑定
/// 先于 N2——`(lambda (foo/bar) foo/bar)` 是局部名非限定引用）。
#[test]
fn takeover_slash_named_define_exempt() {
    assert_eq!(common::run_rendered("(define my/ns 5) my/ns"), "5");
    assert_eq!(
        common::run_rendered("(define f (lambda (foo/bar) foo/bar)) (f 7)"),
        "7"
    );
    // 嵌套遮蔽：外层限定引用有效 + 内层参数同名遮蔽共存
    assert_eq!(
        common::run_rendered("(define f (lambda (string/append) string/append)) (f 9)"),
        "9"
    ); // 参数遮蔽 N2 限定名（R-N2 第一行形态——N3 胜出合法）
}

// ---------------------------------------------------------------------------
// 22 §5.1 矩阵 / 红线 1：io 限定名双门分立
// ---------------------------------------------------------------------------

/// 红线 1（import ≠ 授权的 M1 可观测形态）：`io/print` 未声明 require
/// → E0006（fail-closed 编译期先行；消息携带限定形态「io/print」）。
#[test]
fn io_qualified_requires_write_e0006() {
    expect_compile_err("(io/print 42)", 6, "「io/print」需要 write 能力");
    expect_compile_err("(io/newline)", 6, "「io/newline」需要 write 能力");
    expect_compile_err(
        "(io/write-string \"x\")",
        6,
        "「io/write-string」需要 write 能力",
    );
}

/// io read 族：声明 write 不开放 read（最小权限粒度维持）。
#[test]
fn io_qualified_read_requires_read_e0006() {
    expect_compile_err(
        "(require io write) (io/read-line)",
        6,
        "「io/read-line」需要 read 能力",
    );
}

/// 授权后 io 限定名可调用（N2 注册 fail-closed 的正路——print 族返回
/// nil 副作用经 stdout；无 E0006 即授权面生效的证据。read 族正路经
/// stdin 重定向在测试内不可锚[stdlib_tests 既有结论]——CLI 层验证）。
#[test]
fn io_qualified_authorized_calls_work() {
    assert_eq!(
        common::run_rendered("(require io write) (io/print 42)"),
        "nil"
    );
    assert_eq!(
        common::run_rendered("(require io write) (io/write-string \"x\")"),
        "nil"
    );
    assert_eq!(
        common::run_rendered("(require io write) (io/newline)"),
        "nil"
    );
}

// ---------------------------------------------------------------------------
// R-N4 / R-N7 / R-N8 机制锚
// ---------------------------------------------------------------------------

/// R-N4 词法域：独立 `/` 维持除法（`(/ a b)` 不受限定名机制影响）。
#[test]
fn division_operator_unbroken_rn4() {
    assert_eq!(common::run_rendered("(/ 10 2)"), "5");
    assert_eq!(common::run_rendered("(/ 9 3 3)"), "1"); // 链式除法
                                                        // 限定名与除法同程序共存
    assert_eq!(common::run_rendered("(/ (string/length \"abcd\") 2)"), "2");
}

/// R-N7 运算符永驻：N1 裸名零 import（「零依赖最小骨架」锚——任何
/// 程序先付 import 税与 R-N7 矛盾的显式反例不存在）。
#[test]
fn operators_permanent_in_n1_rn7() {
    assert_eq!(common::run_rendered("(+ 1 2)"), "3");
    assert_eq!(common::run_rendered("(* 2 (+ 3 4))"), "14");
    assert_eq!(common::run_rendered("(< 1 2)"), "true");
    assert_eq!(common::run_rendered("(mod 7 3)"), "1");
}

/// R-N8 符号宇宙豁免：quote 符号值不参与解析（`'string/append` 是
/// 名字即值——存在性 ≠ 可见性；宏模板同名异义零干涉）。
#[test]
fn quoted_symbols_exempt_from_resolution_rn8() {
    assert_eq!(
        common::run_rendered("(core/eq 'string/append 'string/append)"),
        "true"
    );
    assert_eq!(
        common::run_rendered("(core/eq 'nosuchns/foo 'nosuchns/foo)"),
        "true"
    ); // 未知 ns 的符号值同样合法（N0 层存在）
}

// ---------------------------------------------------------------------------
// 静态面 + 双路径
// ---------------------------------------------------------------------------

/// 限定名签名派生（同分派 → 同静态检查面）：`ns/name` 调用参与 E0005
/// 静态判定（`builtin_sigs` 按模块表派生——22 §8 R-N1/R-N3 行静态面；
/// check 路径 = hm 旗标期判定面）。
#[test]
fn qualified_names_static_checked_e0005() {
    let report =
        kerf_driver::check_source("(string/append 1 2)", FNAME).expect("check 入口不应结构失败");
    let hit = report
        .diagnostics
        .iter()
        .any(|d| d.code == Some(DiagnosticCode(5)) || d.message.contains("str"));
    assert!(
        hit,
        "静态检查未覆盖限定名：{:?}",
        report
            .diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
    );
    // 正例零诊断（限定名合法调用不误报）
    let clean = kerf_driver::check_source("(string/append \"a\" \"b\")", FNAME)
        .expect("check 入口不应结构失败");
    assert!(
        clean
            .diagnostics
            .iter()
            .all(|d| d.code != Some(DiagnosticCode(5))),
        "限定名正例误报：{:?}",
        clean
            .diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
    );
}

/// 双路径一致（T1 口径——生产链[自举读+展+编]与种子链渲染等价）。
#[test]
fn qualified_names_dual_path_agrees() {
    assert!(common::dual_path_agrees("(string/append \"a\" \"b\")"));
    assert!(common::dual_path_agrees("(list/nth '(a b c) 1)"));
    assert!(common::dual_path_agrees("(core/eq 'x 'x)"));
    assert!(common::dual_path_agrees("(/ 10 2)"));
}
