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
    // M2（r40）B1 契约落位（20 §4）：限定名 miss → nil（nil-as-absence
    // 与语言值域一致——read-int EOF→nil 先例同型；旧名/扁平新名维持
    // -1 旧契约——「新名新契约、旧名旧契约并存至移除」迁移不变量）
    assert_eq!(
        common::run_rendered("(string/index-of \"abc\" \"z\")"),
        "nil"
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
        "(define f (fn (x) (core/nonexist x)))",
        14,
        "「core」不导出「nonexist」",
    ); // 嵌套深度（Fn 体递归）
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
/// 引用同名 → 不触发 E0014；**Fn 参数遮蔽**（R-N1：N3 局部绑定
/// 先于 N2——`(fn (foo/bar) foo/bar)` 是局部名非限定引用）。
#[test]
fn takeover_slash_named_define_exempt() {
    assert_eq!(common::run_rendered("(define my/ns 5) my/ns"), "5");
    assert_eq!(
        common::run_rendered("(define f (fn (foo/bar) foo/bar)) (f 7)"),
        "7"
    );
    // 嵌套遮蔽：外层限定引用有效 + 内层参数同名遮蔽共存
    assert_eq!(
        common::run_rendered("(define f (fn (string/append) string/append)) (f 9)"),
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

// ---------------------------------------------------------------------------
// 批次 M 次件 M2（v0.6 命名空间层，r40）——import 注入面/别名 +
// 组合闭包 + 位置纪律 + E0016 + W 弃用/遮蔽族 + B1/B2/B3 契约
// （22 §8 实施对账表 M2 行 + 20 §4/§7 + 21 §4.4 + 深审 D1-D9 裁定）
// ---------------------------------------------------------------------------

/// R-N5 非限定导入 + 22 §6 冲突类三过渡期语义锚：`(import kerf-string)`
/// 后裸 `append` 仍解析到 N1 全局 list-append（不同语义位——非限定
/// 引用走 R-N1 查到 N1；限定引用走 R-N3 查 N2；N1 旧名移除[Stage 3]
/// 后竞争自消——22 §6 第三行既裁行为，v0.6 过渡期如实锚定）。
#[test]
fn m2_unqualified_import_injects_export_face() {
    // 裸 append = 全局 list-append（22 §6 冲突类三——过渡期语义）
    assert_eq!(
        common::run_rendered("(module m (import kerf-string) (append '(1) '(2)))"),
        "(1 2)"
    );
    // 裸 nth = 全局 list 族（同上——注入面与全局面同名时全局胜出）
    assert_eq!(
        common::run_rendered("(module m (import kerf-list) (nth '(a b c) 1))"),
        "b"
    );
    // 限定引用 = string 家 append（R-N3 精确消歧——注入声明的正面收益）
    assert_eq!(
        common::run_rendered("(module m (import kerf-string) (string/append \"a\" \"b\"))"),
        "ab"
    );
}

/// R-N5 限定别名：`(import kerf-string as str)` → `str/append` 别名
/// 限定引用（编译期归一——22 §3.5「别名是局部绑定」机制面）。
#[test]
fn m2_qualified_alias_import_resolves() {
    assert_eq!(
        common::run_rendered("(module m (import kerf-string as str) (str/append \"a\" \"b\"))"),
        "ab"
    );
    assert_eq!(
        common::run_rendered("(module m (import kerf-list as l) (l/nth '(a b c) 1))"),
        "b"
    );
    // 别名 + 非限定混合导入（22 §3.5 三形态并存）
    assert_eq!(
        common::run_rendered(
            "(module m (import kerf-list as l) (import kerf-core) (l/nth (list 1 2) 0))"
        ),
        "1"
    );
}

/// E0013 非限定 import 冲突（22 §6 冲突类一：两模块同名导出——显式
/// 错误非静默遮蔽；消息携带双方模块名 + 逃生阀指引）。
#[test]
fn m2_import_conflict_e0013() {
    expect_compile_err(
        "(module m (import kerf-list) (import kerf-string) 42)",
        13,
        "import 冲突",
    );
    // 逃生阀：as 别名限定导入后不注入非限定名——冲突消除（22 §6）
    assert_eq!(
        common::run_rendered(
            "(module m (import kerf-list as l) (import kerf-string) (string/append \"a\" (string/append \"b\" \"c\")))"
        ),
        "abc"
    );
}

/// E0017 别名重复（22 §3.5 R-N5 冲突列：别名重复定义错误）。
#[test]
fn m2_alias_duplicate_e0017() {
    expect_compile_err(
        "(module m (import kerf-string as s) (import kerf-symbol as s) 42)",
        17,
        "别名重复",
    );
}

/// E0019 未知导入模块（深审 D2 裁定：Clojure require 同型编译期拒绝；
/// 消息携带在册名单 + Stage 3 窗口指引）。
#[test]
fn m2_unknown_import_e0019() {
    expect_compile_err(
        "(module m (import kerf-nonexist) 42)",
        19,
        "未知导入模块「kerf-nonexist」",
    );
}

/// E0016 module 体内同名 define 重复（09 v6.2 既有裁定的 M2 落位）。
#[test]
fn m2_module_define_duplicate_e0016() {
    expect_compile_err(
        "(module m (define x 1) (define x 2) x)",
        16,
        "重复定义：模块「m」内 define「x」重复",
    );
}

/// E0018 require 位置纪律（深审 D3 裁定：合法位 = 顶层/module 体直接
/// 元素；表达式子树内 = 位置错误——fail-closed）。
#[test]
fn m2_require_position_e0018() {
    // fn 体内 require = 位置违例（授权声明非表达式）
    expect_compile_err(
        "(require io write) (require io read) (module m ((fn (x) (require io read) x) 1))",
        18,
        "require 位置违例",
    );
    // module 体直接元素合法（需求元数据——组合闭包收集面）
    let r = run_source(
        "(require io write) (module m (require io write) (define f (print 1)) 42)",
        FNAME,
    );
    assert!(
        r.is_ok(),
        "module 体直接位 require 合法：\n{:?}",
        r.err().map(|e| e.rendered)
    );
}

/// 组合闭包（21 §4.4）：模块需求 ⊆ 入口授权——模块体内 require 是
/// 需求元数据非授权获得；缺失 → E0006 增强形态（携带模块归属）。
#[test]
fn m2_capability_closure_e0006_enhanced() {
    expect_compile_err(
        "(module m (require io write) (define f (print 1)) 42)",
        6,
        "模块「m」声明需要 io write",
    );
    // 上移入口后通过（授权获得唯一路径 = 顶层声明）
    let r = run_source(
        "(require io write) (module m (require io write) (define f (print 1)) 42)",
        FNAME,
    );
    assert!(
        r.is_ok(),
        "require 上移入口后应通过：\n{:?}",
        r.err().map(|e| e.rendered)
    );
}

/// 组合闭包双分项（read 与 write 独立核对——需求并集语义）。
#[test]
fn m2_capability_closure_covers_read_write_union() {
    // 程序声明 write、模块需求 read+write → read 缺失（并集不满足）
    expect_compile_err(
        "(require io write) (module m (require io read) (require io write) (define f (print 1)) 42)",
        6,
        "io read",
    );
}

/// B1 契约（20 §4）：限定名 `string/index-of` miss → nil；旧名
/// `string-index-of` 与扁平新名 `string-index-of` 维持 -1（迁移不变量：
/// 「新名新契约、旧名旧契约并存至移除」——r38 别名层 parity 不动）。
#[test]
fn m2_b1_contract_qualified_nil_vs_legacy_minus_one() {
    // 限定名（新契约）
    assert_eq!(
        common::run_rendered("(string/index-of \"abc\" \"z\")"),
        "nil"
    );
    assert_eq!(common::run_rendered("(string/index-of \"abc\" \"b\")"), "1");
    // 旧名（旧契约维持）
    assert_eq!(
        common::run_rendered("(string-index-of \"abc\" \"z\")"),
        "-1"
    );
    // 扁平新名（v0.5 别名层 parity 维持——改名与改行为永不混步）
    assert_eq!(
        common::run_rendered("(string-index-of \"abc\" \"z\")"),
        "-1"
    );
}

/// B2 契约（20 §4）：`list/member`/`list/assoc` miss → nil（返回类型
/// 单一化：表-or-nil）；命中形态不变；旧名 false 维持。
#[test]
fn m2_b2_contract_member_assoc_nil() {
    assert_eq!(common::run_rendered("(list/member 9 (list 1 2 3))"), "nil");
    assert_eq!(
        common::run_rendered("(list/member 2 (list 1 2 3))"),
        "(2 3)"
    );
    assert_eq!(common::run_rendered("(list/assoc 'z '((a 1)))"), "nil");
    assert_eq!(common::run_rendered("(list/assoc 'a '((a 1)))"), "(a 1)");
    // 旧名（旧契约维持）
    assert_eq!(common::run_rendered("(member 9 (list 1 2 3))"), "false");
    assert_eq!(common::run_rendered("(assoc 'z '((a 1)))"), "false");
}

/// B3 契约（20 §4）：`io/read-line` 严格 0 参（多余实参 = 运行时错误
/// ——底层已对齐[FS-4 修复面，20 §4 B3 行 R4 修正]；本 case 为测试锚）。
#[test]
fn m2_b3_contract_read_line_strict_arity() {
    let e = run_source("(require io read) (module m (io/read-line 1))", FNAME)
        .expect_err("多余实参应报运行时错误");
    assert!(
        e.rendered.contains("read-line 需要 0 个参数"),
        "实际：{}",
        e.rendered
    );
}

/// r42 / S1 移除轮——W1001 弃用族退役 + W1003 宏名遮蔽警告（22 §11
/// D11 排期本窗兑现）。旧名 27 件升 E0021 编译期错误（本组由
/// stdlib_tests removed_rejects_* 27 case 承载）；本组锚定：
/// ①现代名零警告（W 面收窄实证）②限定名零警告 ③W1003 正例（宏名
/// 遮蔽内置名 → 恰一条知会）④W1003 非内置名零误报 ⑤W1003 限定名
/// 遮蔽同知会 ⑥宏胜出行为不变（宏体执行非内置）。
#[test]
fn s1_w1003_macro_shadow_builtin_warning() {
    // ① 现代名零警告（W1001 退役后 W 面收窄）
    let o = run_source("(module m (head (list 1 2)))", FNAME).expect("现代名可用");
    assert!(
        o.warnings.is_empty(),
        "现代名不应有任何 W 警告：{:?}",
        o.warnings
            .iter()
            .map(|w| w.message.clone())
            .collect::<Vec<_>>()
    );
    // ② 限定名零警告
    let o2 = run_source("(module m (pair/head (pair/cons 1 2)))", FNAME).expect("限定名可用");
    assert!(o2.warnings.is_empty(), "限定名不应警告");
    // ③ W1003 正例：宏名遮蔽内置名（故意重定义——宏胜出是本质能力，
    // W 级知会非阻断）
    let o3 = run_source(
        "(module m (define-syntax head (syntax-rules () ((_ x) 99))) (head (list 1 2)))",
        FNAME,
    )
    .expect("宏遮蔽内置应运行（W 级非阻断）");
    assert_eq!(o3.warnings.len(), 1, "恰一条 W1003 宏名遮蔽警告");
    assert!(
        o3.warnings[0].message.contains("宏名「head」遮蔽了内置名")
            && o3.warnings[0].message.contains("W1003"),
        "消息：{}",
        o3.warnings[0].message
    );
    // ⑥ 宏胜出行为不变（宏体 99 执行，非内置 head）
    assert_eq!(
        common::run_rendered(
            "(module m (define-syntax head (syntax-rules () ((_ x) 99))) (head (list 1 2)))"
        ),
        "99",
        "宏应胜出（本质能力——W 仅知会）"
    );
}

/// W1003 非内置名零误报 + 限定名遮蔽同知会（22 §11 D11：宏名 ∈
/// 内置注册面[57 扁平 + 47 限定]→ W；非内置名（用户名/模块名）零 W）。
#[test]
fn s1_w1003_scoping_and_qualified() {
    // 非内置宏名：零 W
    let o = run_source(
        "(module m (define-syntax my-macro (syntax-rules () ((_) 1))) (my-macro))",
        FNAME,
    )
    .expect("用户宏名应运行零警告");
    assert!(
        o.warnings.is_empty(),
        "非内置宏名不应 W1003：{:?}",
        o.warnings
            .iter()
            .map(|w| w.message.clone())
            .collect::<Vec<_>>()
    );
    // 限定名遮蔽：宏名取限定形态 string/append → 同知会（注册面含 47 限定名）
    let o2 = run_source(
        "(module m (define-syntax string/append (syntax-rules () ((_ a b) 7))) (string/append \"a\" \"b\"))",
        FNAME,
    )
    .expect("限定名宏遮蔽应运行");
    assert!(
        o2.warnings
            .iter()
            .any(|w| w.message.contains("宏名「string/append」遮蔽了内置名")),
        "限定名遮蔽应 W1003：{:?}",
        o2.warnings
            .iter()
            .map(|w| w.message.clone())
            .collect::<Vec<_>>()
    );
}

/// r42 / S1 移除轮——preamble 注入零 W 面维持（W1001 退役后：preamble
/// 已迁移现代名，结构豁免守 W1002 注入遮蔽误报——20 §6.5 纪律不变）。
#[test]
fn s1_prelude_injection_zero_warnings() {
    let o = run_source(
        "(module user (import kerf-prelude) (define lst (list 1 2 3)) (map (fn (x) x) lst))",
        FNAME,
    )
    .expect("prelude 注入程序应运行");
    assert!(
        o.warnings.is_empty(),
        "preamble 注入不应产生用户面警告：{:?}",
        o.warnings
            .iter()
            .map(|w| w.message.clone())
            .collect::<Vec<_>>()
    );
}

/// r42 / S1 移除轮——E0021 限定形旧名引用（值位/操作位全域 + 恢复
/// 健康：E0021 后管线继续可编译合法程序）。
#[test]
fn s1_e0021_recovery_and_positions() {
    // 操作位 + 值位（嵌套表达式 + define 初始化位）
    let err = run_source("(module m (define x (car (list 1 2))))", FNAME)
        .expect_err("旧名 define 初始化位应 E0021");
    assert!(
        err.rendered.contains("[E0021]") && err.rendered.contains("现代名「head」"),
        "E0021 携现代名指引：{}",
        err.rendered
    );
    // assign 值位
    let err2 = run_source("(module m (define x 1) (assign x (cdr (list 1 2))))", FNAME)
        .expect_err("旧名 assign 值位应 E0021");
    assert!(
        err2.rendered.contains("[E0021]"),
        "assign 值位：{}",
        err2.rendered
    );
    // E0021 后管线健康（fail-closed 单条阻断——修正后正常编译）
    let ok = run_source("(module m (define x (head (list 1 2))) nil)", FNAME);
    assert!(
        ok.is_ok(),
        "修正后应正常编译：{:?}",
        ok.err().map(|e| e.rendered)
    );
    assert_eq!(
        common::run_rendered("(module m (define x (head (list 1 2))) nil)"),
        "nil"
    );
}

/// W1002 遮蔽注入名警告（22 §3.2 R-N2 第二行：N3 遮蔽 N2 注入名 =
/// 合法 + W 级警告——可恢复但值得提示）。
#[test]
fn m2_w1002_shadow_import_warning() {
    let o = run_source(
        "(module m (import kerf-list) ((fn (length) length) 1))",
        FNAME,
    )
    .expect("参数遮蔽注入名合法（内层胜——R-N2）");
    assert_eq!(o.warnings.len(), 1);
    assert!(
        o.warnings[0]
            .message
            .contains("fn 参数「length」遮蔽了 import 注入名"),
        "消息：{}",
        o.warnings[0].message
    );
}

/// R-N1 解析全序锚（22 §3.1：N3 → N2 → N1——内层绑定胜出全局名）。
#[test]
fn m2_r_n1_resolution_order_local_wins() {
    // fn 参数遮蔽全局名：局部绑定胜出（N3 > N1——词法 60 年共识）
    assert_eq!(
        common::run_rendered("(module m (import kerf-list) ((fn (nth) nth) 99))"),
        "99"
    );
}

/// 红线 1 双门分立（22 §5.2：import 不传播授权——import kerf-io 后
/// 名字可见但未 require 调用仍 E0006）。
#[test]
fn m2_redline1_import_does_not_propagate_authority() {
    expect_compile_err("(module m (import kerf-io) (print 42))", 6, "print");
    // 授权后通过（双门独立变化——红线 2）
    let r = run_source(
        "(require io write) (module m (import kerf-io) (print 42))",
        FNAME,
    );
    assert!(
        r.is_ok(),
        "require 授权后 import 面可调用：\n{:?}",
        r.err().map(|e| e.rendered)
    );
}

/// 别名归一 E0014 形态（`str/nonexist` → 诊断携归一模块面信息）。
#[test]
fn m2_alias_qualified_e0014_with_normalization() {
    expect_compile_err(
        "(module m (import kerf-string as str) (str/nonexist \"a\"))",
        14,
        "不导出「nonexist」",
    );
}

/// 别名限定名双路径一致（T1 口径——编译期归一在共享 front 段完成）。
#[test]
fn m2_alias_dual_path_agrees() {
    assert!(common::dual_path_agrees(
        "(module m (import kerf-string as str) (str/append \"a\" \"b\"))"
    ));
    assert!(common::dual_path_agrees(
        "(module m (import kerf-list as l) (l/member 2 (list 1 2 3)))"
    ));
}

// ---------------------------------------------------------------------------
// r41 / 62-a 语言形式深审轮——D10 裁定测试组：E0020 保留字绑定禁令
//（22 §2.1 N4 层不变量「不是值、不可引用、不可遮蔽」的编译期执行；
// 修复前形态 = 值位可绑定 + 操作位静默失效的 Lisp 双轨陷阱）
// ---------------------------------------------------------------------------

/// 正例：非保留字绑定全绿（E0020 零误报——绑定面白名单形态）。
#[test]
fn d10_non_reserved_bindings_all_legal() {
    // define/lambda/let 系正常名（含现代名/限定名符号/下划线风格）
    assert_eq!(
        common::run_rendered(
            "(define my-x 5) (define (lambda-pair a b) (pair/cons a b)) \
             (let ((lo 1) (hi 2)) (+ lo hi)) \
             ((fn (first-arg second-arg) (* first-arg second-arg)) 3 4)"
        ),
        "12"
    );
    // letrec/let* 正常名
    assert_eq!(
        common::run_rendered("(letrec ((f (fn (n) (if (= n 0) 1 n)))) (f 3))"),
        "3"
    );
    // handle 绑定器正常名（CoreExpr 层零误报——payload/resume 位合法名）
    assert_eq!(
        common::run_rendered(
            "(handle state ((payload resume-k) (resume-k 99)) (perform (cons 'state 1)))"
        ),
        "99"
    );
}

/// 正例：quote 保留字数据符号合法（N0 符号宇宙层——同像性维持；
/// 存在性 ≠ 可见性，E0020 只禁绑定与值位引用，不禁数据符号）。
#[test]
fn d10_quote_reserved_word_data_symbols_legal() {
    assert_eq!(common::run_rendered("(is-symbol 'if)"), "true");
    assert_eq!(common::run_rendered("(is-symbol (quote fn))"), "true");
    assert_eq!(common::run_rendered("(core/eq 'else 'else)"), "true");
}

/// 正例：宏展开产物正常绑定（CoreExpr 层防御纵深零误报——宏模板
/// 展开生成 define/fn 绑定不触发 E0020）。
#[test]
fn d10_macro_expansion_bindings_legal() {
    assert_eq!(
        common::run_rendered(
            "(define-syntax swap-pair (syntax-rules () ((_ a b) (pair/cons b a)))) \
             (swap-pair 1 2)"
        ),
        "(2 . 1)"
    );
}

/// 负例：define 绑定保留字（深审 D10 修复前实测形态——`(define if 5)`
/// 合法且值位可读 = N4 不变量违反 + 操作位静默失效）。
#[test]
fn d10_define_reserved_word_e0020() {
    expect_compile_err("(define if 5)", 20, "保留字不可绑定");
    expect_compile_err("(define fn 5)", 20, "「fn」");
    expect_compile_err("(define assign 7)", 20, "「assign」");
}

/// 负例：fn 参数表保留字（修复前 `(fn (fn) fn)` = 42）。
#[test]
fn d10_lambda_param_reserved_word_e0020() {
    expect_compile_err("((fn (if) if) 1)", 20, "fn 参数");
    expect_compile_err("((fn (x else) x) 1 2)", 20, "「else」");
    expect_compile_err("((fn (do) do) 1)", 20, "「do」");
}

/// 负例：let 系绑定名保留字（三变体同判）。
#[test]
fn d10_let_bindings_reserved_word_e0020() {
    expect_compile_err("(let ((if 5)) if)", 20, "let 绑定名");
    expect_compile_err("(let* ((do 1)) do)", 20, "「do」");
    expect_compile_err("(letrec ((module 1)) module)", 20, "「module」");
}

/// 负例：assign 赋值目标保留字。
#[test]
fn d10_setbang_target_reserved_word_e0020() {
    expect_compile_err("(define x 1) (assign do 2)", 20, "赋值目标");
}

/// 负例：module 名 / import as 别名 / define-syntax 宏名保留字。
#[test]
fn d10_module_alias_macro_name_reserved_word_e0020() {
    expect_compile_err("(module if (export a) (define a 1))", 20, "模块名");
    expect_compile_err(
        "(module m (import kerf-string as if) (if/length \"a\"))",
        20,
        "import 别名",
    );
    expect_compile_err(
        "(define-syntax if (syntax-rules () ((_ c t) t)))",
        20,
        "宏名",
    );
}

/// 负例：begin/嵌套子树递归穿透（深位置绑定同样拦截）。
#[test]
fn d10_nested_begin_reserved_word_e0020() {
    expect_compile_err(
        "(do (define ok 1) (do (define perform 2)))",
        20,
        "「perform」",
    );
    // fn 体深嵌套
    expect_compile_err("((fn (n) (let ((handle n)) handle)) 1)", 20, "「handle」");
}

/// 负例：宏展开产物绑定保留字（CoreExpr 层防御纵深——宏模板生成
/// E0020 违例同样编译期拦截）。
#[test]
fn d10_macro_expansion_reserved_binding_e0020() {
    expect_compile_err(
        "(define-syntax m-kw (syntax-rules () ((_) (define quote 1)))) (m-kw)",
        20,
        "绑定名（展开产物）",
    );
}

/// 负例：handle 双绑定器保留字（payload/resume 绑定位——CoreExpr 层
/// 结构化承载）。
#[test]
fn d10_handle_binders_reserved_word_e0020() {
    expect_compile_err(
        "(handle state ((perform k) (resume k 42)) (perform (cons 'state 1)))",
        20,
        "handle 载荷绑定",
    );
}

/// 值位引用保留字 → 既有未绑定路径（fail-closed 维持——绑定禁令后
/// 值位恒无绑定，天然走 E0002 未绑定；诊断面按既有口径）。
#[test]
fn d10_value_position_reserved_word_unbound() {
    let e = match run_source("(print if)", FNAME) {
        Ok(_) => panic!("值位引用保留字应报错（未绑定路径）"),
        Err(e) => e,
    };
    assert!(
        e.rendered.contains("if"),
        "诊断应含保留字名：\n{}",
        e.rendered
    );
}

// ---------------------------------------------------------------------------
// r43 / E5 S2 关键字腿（22 §12 D19-D33 + e5-plan §3.2 S2 断言③④）
// ---------------------------------------------------------------------------

/// S2 断言③：关键字旧形（lambda/set!/begin）引用 → E0021 携新形指引
/// （操作位调用形态 + 值位引用；消息分源——关键字文案「关键字已切换」）。
#[test]
fn s2_e0021_keyword_old_forms_rejected() {
    for (src, modern) in [
        ("(lambda (x) x)", "fn"),
        ("(set! x 1)", "assign"),
        ("(begin 1 2)", "do"),
        ("(require io write) (define x 1) (print lambda)", "fn"),
    ] {
        let err = run_source(src, FNAME).expect_err("关键字旧形应 E0021");
        assert!(
            err.rendered.contains("[E0021]"),
            "E0021 码位（{}）：\n{}",
            src,
            err.rendered
        );
        assert!(
            err.rendered.contains(&format!("现代形式「{}」", modern)),
            "E0021 携新形指引（{} → {}）：\n{}",
            src,
            modern,
            err.rendered
        );
    }
}

/// S2 断言③接管豁免：用户 define 旧形 = 合法用户变量（退役的是 N4
/// 关键字面非 N0 符号宇宙层——与 S1 内置旧名接管豁免同口径）。
#[test]
fn s2_e0021_takeover_exempt() {
    let ok = run_source("(require io write) (define lambda 5) (print lambda)", FNAME)
        .expect("用户接管旧形名合法（N0 符号宇宙层自由）");
    let _ = ok;
    assert_eq!(common::run_rendered("(define lambda 5) lambda"), "5");
}

/// S2 断言④：新名 fn/assign/do 即刻入 E0020 禁绑面（25 名计数不变
/// ——三名出三名入；d10 组既有 12 case 已随语料迁移覆盖新名）。
#[test]
fn s2_e0020_new_reserved_names() {
    for (src, kind) in [
        ("(define fn 5)", "绑定名"),
        ("(define do 5)", "绑定名"),
        ("((fn (assign) assign) 1)", "fn 参数"),
        ("((fn (do) do) 1)", "fn 参数"),
    ] {
        expect_compile_err(src, 20, kind);
    }
}

/// S2 门 A（CoreExpr parity——e5-plan §3.2 S2 行断言①的集成形态）：
/// 新形源码与旧形等价程序产相同行为（表面/内部分离原则 31 的行为面
/// 验证——同映 CoreExpr::Fn/Assign/Do）。
#[test]
fn s2_core_expr_parity_new_forms() {
    assert_eq!(common::run_rendered("((fn (x) (* x x)) 6)"), "36");
    assert_eq!(
        common::run_rendered("(define x 1) (assign x (+ x 1)) x"),
        "2"
    );
    assert_eq!(common::run_rendered("(do 1 2 3)"), "3");
}
