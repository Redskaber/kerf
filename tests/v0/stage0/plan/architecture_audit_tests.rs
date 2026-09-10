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
use kerf_syntax::{Stx, Symbol, SymbolTable};

// ---------------------------------------------------------------------------
// 原则 30：私有 ADT 冻结 + Span 元数据独立（01 §8.5 锚点 1）
// ---------------------------------------------------------------------------

/// 十变体穷尽 match（编译期证明变体集冻结）+ 逐变体字段形状锚定
/// （01 §5 既有锚点的集成面补强）+ `span()`/`kind_name()` 全变体全通
/// （元数据与命名分离——Span 独立携带，`kind_name` 仅诊断渲染）。
#[test]
fn core_expr_variant_set_frozen_exhaustive() {
    let s0 = Symbol(0);
    let sp = kerf_span::Span::dummy();

    // 十变体各构造一例——构造本身即字段形状锚定（§3.2 冻结定义 +
    // Require 声明变体；无任何类型/效应/能力标注字段）。
    let lit = CoreExpr::Literal {
        value: LiteralValue::Int(7),
        span: sp,
    };
    let instances: Vec<CoreExpr> = vec![
        CoreExpr::Lambda {
            params: vec![s0],
            param_scopes: vec![kerf_syntax::ScopeSet::new()],
            body: Rc::new(lit.clone()),
            span: sp,
        },
        CoreExpr::App {
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
        CoreExpr::VarRef {
            name: s0,
            scopes: kerf_syntax::ScopeSet::new(),
            span: sp,
        },
        lit,
        CoreExpr::SetBang {
            name: s0,
            scopes: kerf_syntax::ScopeSet::new(),
            value: Rc::new(CoreExpr::VarRef {
                name: s0,
                scopes: kerf_syntax::ScopeSet::new(),
                span: sp,
            }),
            span: sp,
        },
        CoreExpr::Define {
            name: s0,
            value: Rc::new(CoreExpr::VarRef {
                name: s0,
                scopes: kerf_syntax::ScopeSet::new(),
                span: sp,
            }),
            span: sp,
        },
        CoreExpr::Begin {
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
    ];

    // 穷尽 match（编译期证明）：本 match 无通配臂——新增/删除/重命名变体
    // 都会使本函数编译失败（冻结的机器证明；字段解构同时锚定字段形状，
    // 01 §5 逐字段对照的集成面）。
    let prove_frozen = |e: &CoreExpr| -> &'static str {
        match e {
            CoreExpr::Lambda { .. } => "lambda",
            CoreExpr::App { .. } => "app",
            CoreExpr::If { .. } => "if",
            CoreExpr::VarRef { .. } => "var-ref",
            CoreExpr::Literal { .. } => "literal",
            CoreExpr::SetBang { .. } => "set!",
            CoreExpr::Define { .. } => "define",
            CoreExpr::Begin { .. } => "begin",
            CoreExpr::Module { .. } => "module",
            CoreExpr::Require { .. } => "require",
        }
    };
    let names: Vec<&'static str> = instances.iter().map(prove_frozen).collect();

    // kind_name 全变体互异（诊断渲染面的全映射 + 零冲突）。
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "kind_name 全变体互异");
    assert_eq!(names.len(), 10, "冻结变体集 = 9 原语 + Require 声明变体");
    for (e, n) in instances.iter().zip(&names) {
        assert_eq!(e.kind_name(), *n);
        // Span 独立携带：全变体可取且与构造值一致（元数据不依赖命名约定——
        // 原则 30 的「Span 系统独立携带元数据」面）。
        assert_eq!(e.span(), sp);
    }
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
    let forms: Result<Vec<Stx>, _> = read_source("(begin 1 2)", 0, &mut interner);
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
    let src =
        "(module m (export main)\n(require io write)\n(define (main) (begin (print 1) 42)))\n";
    let a = compile_source(src, "a.krf").expect("compile a");
    let b = compile_source(src, "b.krf").expect("compile b");
    assert_eq!(
        a.render_core(),
        b.render_core(),
        "同一表面语法 → 相同内部核心形式（确定性桥接）"
    );
}
