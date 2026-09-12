//! 架构合规审计测试（v6.0——lang-design [01 §8.5](../../../docs/lang-design/01-core-forms.md) /
//! [02 §8.1](../../../docs/lang-design/02-syntax-model.md) / [15 §5.3](../../../docs/lang-design/15-architecture-layers.md)
//! 锚点的测试落地；sop §2.2 原则 29-31 的架构形态审计）。
//!
//! 审计对象（三原则的「架构形态」面——命名本身冻结不变，见 01 §8.2 裁定）：
//! - **原则 30（类型安全优于命名安全）**：`CoreExpr` 是编译器私有 ADT——
//!   变体集冻结（穷尽 match = 编译期证明恰好十变体，新增变体即编译失败），
//!   且每变体 Span 独立携带（元数据与命名分离：`kind_name` 仅诊断渲染），
//!   字段形状经构造逐字段锚定（无类型/效应/能力标注字段——四层正交的
//!   语法层纯净性，15 §5.1 Layer 1）。
//! - **原则 31（表面-内部语法严格分离）**：Reader 产出 `Stx` 不产出
//!   `CoreExpr`（类型级断言）；Expander 是唯一 `Stx → CoreExpr` 桥
//!   （翻译发生证明——表面语法细节不旁路进入核心形式）；同一源码
//!   双次编译产出逐字节相同核心形式（可替换性基线——Stage 2 目标语法
//!   Reader 引入时的「两语法同核验证」前置，02 §8.1）。

use std::rc::Rc;

use kerf_core::{Capability, CoreExpr, LiteralValue};
use kerf_driver::compile_source;
use kerf_expander::{expand_program, ExpandCtxt};
use kerf_reader::read_source;
use kerf_syntax::ScopeSet;
use kerf_syntax::{Keyword, Stx, Symbol, SymbolTable};

// ---------------------------------------------------------------------------
// 原则 30：私有 ADT 冻结 + Span 元数据独立（01 §8.5 锚点 1）
// ---------------------------------------------------------------------------

/// 十变体穷尽 match（编译期证明变体集冻结）+ 逐变体字段形状锚定
/// （01 §5 既有锚点的集成面补强）+ `span()`/`kind_name()` 全变体全通
/// （元数据与命名分离——Span 独立携带，`kind_name` 仅诊断渲染）。
/// 十二变体各构造一例（共享构造面：冻结测试与五面同词根测试共用）。
fn frozen_instances(sp: kerf_span::Span) -> Vec<CoreExpr> {
    let s0 = Symbol(0);
    let lit = CoreExpr::Literal {
        value: LiteralValue::Int(7),
        span: sp,
    };
    vec![
        CoreExpr::Fn {
            params: vec![s0],
            param_scopes: vec![kerf_syntax::ScopeSet::new()],
            body: Rc::new(lit.clone()),
            span: sp,
        },
        CoreExpr::Apply {
            fn_expr: Rc::new(lit.clone()),
            args: vec![],
            span: sp,
        },
        CoreExpr::If {
            cond: Rc::new(lit.clone()),
            then_branch: Rc::new(lit.clone()),
            else_branch: Rc::new(lit.clone()),
            span: sp,
        },
        CoreExpr::Var {
            name: s0,
            scopes: kerf_syntax::ScopeSet::new(),
            span: sp,
        },
        lit,
        CoreExpr::Assign {
            name: s0,
            scopes: kerf_syntax::ScopeSet::new(),
            value: Rc::new(CoreExpr::Var {
                name: s0,
                scopes: kerf_syntax::ScopeSet::new(),
                span: sp,
            }),
            span: sp,
        },
        CoreExpr::Define {
            name: s0,
            value: Rc::new(CoreExpr::Var {
                name: s0,
                scopes: kerf_syntax::ScopeSet::new(),
                span: sp,
            }),
            span: sp,
        },
        CoreExpr::Do {
            body: vec![],
            span: sp,
        },
        CoreExpr::Module {
            name: s0,
            imports: vec![],
            exports: vec![],
            body: vec![],
            span: sp,
        },
        CoreExpr::Require {
            caps: vec![Capability::IoRead],
            span: sp,
        },
        // r25/42-f：效应两原语（原语集 9→11——effect-language-design
        // §2.1/W1 回写；与 Perform/Handle 两变体同步冻结）
        CoreExpr::Perform {
            effect: Rc::new(CoreExpr::Literal {
                value: LiteralValue::Int(1),
                span: sp,
            }),
            span: sp,
        },
        CoreExpr::Handle {
            tag: Rc::from("tag"),
            payload_var: Symbol(1),
            payload_scopes: ScopeSet::new(),
            resume_var: Symbol(2),
            resume_scopes: ScopeSet::new(),
            handler_body: Rc::new(CoreExpr::Literal {
                value: LiteralValue::Nil,
                span: sp,
            }),
            body: Rc::new(CoreExpr::Literal {
                value: LiteralValue::Nil,
                span: sp,
            }),
            span: sp,
        },
    ]
}

/// 十二变体穷尽 match（编译期证明变体集冻结）+ 逐变体字段形状锚定
/// （01 §5 既有锚点的集成面补强）+ `span()`/`kind_name()` 全变体全通
/// （元数据与命名分离——Span 独立携带，`kind_name` 仅诊断渲染）。
#[test]
fn core_expr_variant_set_frozen_exhaustive() {
    let sp = kerf_span::Span::dummy();
    let instances = frozen_instances(sp);

    let prove_frozen = |e: &CoreExpr| -> &'static str {
        match e {
            CoreExpr::Fn { .. } => "fn",
            CoreExpr::Apply { .. } => "apply",
            CoreExpr::If { .. } => "if",
            CoreExpr::Var { .. } => "var",
            CoreExpr::Literal { .. } => "literal",
            CoreExpr::Assign { .. } => "assign",
            CoreExpr::Define { .. } => "define",
            CoreExpr::Do { .. } => "do",
            CoreExpr::Module { .. } => "module",
            CoreExpr::Require { .. } => "require",
            CoreExpr::Perform { .. } => "perform",
            CoreExpr::Handle { .. } => "handle",
        }
    };
    let names: Vec<&'static str> = instances.iter().map(prove_frozen).collect();

    // kind_name 全变体互异（诊断渲染面的全映射 + 零冲突）。
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "kind_name 全变体互异");
    assert_eq!(
        names.len(),
        12,
        "冻结变体集 = 11 原语 + Require 声明变体（r25/42-f 效应两原语入集）"
    );
    for (e, n) in instances.iter().zip(&names) {
        assert_eq!(e.kind_name(), *n);
        // Span 独立携带：全变体可取且与构造值一致（元数据不依赖命名约定——
        // 原则 30 的「Span 系统独立携带元数据」面）。
        assert_eq!(e.span(), sp);
    }
}

// ---------------------------------------------------------------------------
// r44 / E5 S3 M-R 名面臂：五面同词根（22 §13 D34/D37——诊断面 ↔ 关键字
// 注册面穿透断言）
// ---------------------------------------------------------------------------

/// 五面同词根穿透（22 §13 D34「一个语义一个名」的机器断言）：
/// - 原语级关键字变体（9 名）：`kind_name` 必须是**注册关键字**且与
///   `Keyword::as_str` 同名（诊断面 = 关键字面 = 表面 = 桥 tag = 渲染）；
/// - 内部名变体（apply/var/literal 3 名）：`kind_name` **不得**是关键字
///   （无表面关键字的语义——内部名独立域，不侵蚀 E0020 全域排他面）。
#[test]
fn s3mr_kind_name_keyword_face_penetration() {
    let instances = frozen_instances(kerf_span::Span::dummy());
    // 原语级关键字面（9 名）：kind_name ∈ Keyword 注册面（同名穿透）。
    let primitive_keywords = [
        "fn", "if", "assign", "define", "do", "module", "require", "perform", "handle",
    ];
    // 内部名面（3 名）：无表面关键字对应物——三面（桥/ADT/诊断）同名的
    // 独立内部名域。
    let internal_names = ["apply", "var", "literal"];
    let mut seen = 0;
    for e in &instances {
        let n = e.kind_name();
        if primitive_keywords.contains(&n) {
            assert!(
                Keyword::from_name(n).is_some(),
                "原语级 kind_name「{}」必须是注册关键字（五面同词根）",
                n
            );
            seen += 1;
        } else if internal_names.contains(&n) {
            assert!(
                Keyword::from_name(n).is_none(),
                "内部名「{}」不得占用关键字面（内部名独立域）",
                n
            );
        } else {
            panic!("未归类的 kind_name「{}」（12 名表失效）", n);
        }
    }
    assert_eq!(seen, 9, "原语级关键字变体恰 9 名（9+3 = 12 名表）");
    assert_eq!(instances.len(), 12, "12 名表计数锚");
}

// ---------------------------------------------------------------------------
// 原则 31：表面/内部语法严格分离（02 §8.1 锚点 1/3）
// ---------------------------------------------------------------------------

/// Reader 产出 `Stx`（类型级断言）——表面语法解析止步于语法对象层，
/// `CoreExpr` 只能经 Expander 桥接产生（无旁路）。
#[test]
fn reader_emits_stx_not_core_expr() {
    let mut interner = SymbolTable::new();
    // 类型即断言：read_source 的输出类型是 Vec<Stx>——本绑定若与内部
    // AST 类型不符即编译失败（Reader 与 CoreExpr 的类型级隔离）。
    let forms: Result<Vec<Stx>, _> = read_source("(do 1 2)", 0, &mut interner);
    let forms = forms.expect("Reader 产出语法对象");
    assert!(!forms.is_empty(), "S 表达式表面语法经 Reader 归约为 Stx");
}

/// Expander 是唯一 `Stx → CoreExpr` 桥且**翻译确实发生**：字面量 `42`
/// 经展开后是 `CoreExpr::Literal`（内部形态），不是语法对象的原样直通
/// （表面语法细节零泄漏——脱糖近恒等映射的「映射」面）。
#[test]
fn expander_is_sole_stx_to_core_bridge() {
    let mut interner = SymbolTable::new();
    let forms = read_source("42", 0, &mut interner).expect("read");
    let mut ctx = ExpandCtxt::new(interner);
    let core = expand_program(&forms, &mut ctx).expect("expand");
    // 断言翻译产物是内部 ADT 形态（而非 Stx 直通）。
    let first = core.first().expect("单形式程序产出单核心形式");
    assert!(
        matches!(**first, CoreExpr::Literal { .. }),
        "42 经 Expander 翻译为 CoreExpr::Literal——翻译发生而非直通"
    );
}

/// 同一源码双次编译产出逐字节相同核心形式（`render_core` 全等）——
/// 表面语法可替换性的确定性基线：Stage 2 目标语法 Reader 引入时的
/// 「两语法同核验证」（02 §8.1 锚点 2 的 Stage 0 前置条件）。
#[test]
fn same_surface_source_compiles_to_identical_core() {
    // M2（r40）组合闭包（21 §4.4）：require 上移程序顶层——模块体内
    // require = 需求元数据非授权获得（授权唯一来源 = 入口顶层声明）
    let src = "(require io write)\n(module m (export main)\n(define (main) (do (print 1) 42)))\n";
    let a = compile_source(src, "a.krf").expect("compile a");
    let b = compile_source(src, "b.krf").expect("compile b");
    assert_eq!(
        a.render_core(),
        b.render_core(),
        "同一表面语法 → 相同内部核心形式（确定性桥接）"
    );
}
