//! 展开器主控（stage0.md §19.2 展开循环骨架的落地实现；TD-012 拆分后的
//! 模块布局）。
//!
//! `Stx` → `CoreExpr`：入口分派（宏 → 糖 → 核心形式 → 函数应用）。
//!
//! **模块布局**（TD-012，2026-09-10 拆分）：
//! - 本文件（主控）：公共类型（`ExpandCtxt`/`ExpandError`）+ 入口分派 +
//!   宏调用展开 + 公共构造辅助（关键词句柄/字面量/set! 构造）；
//! - `core_forms`：9 核心形式展开 + 函数体 define 提升路径；
//! - `sugar`：内置语法糖变换器（§3.2 推导表——`TransformerKind::Builtin`
//!   载体）；
//! - `macro_sys`：变换器机制（syntax-rules/卫生上下文）；
//! - `phase`：相位驱动（模块 declare/visit/instantiate 簿记）。
//!
//! **展开循环骨架**（§19.2）：
//! 1. 核心形式：不展开自身，只递归展开子节点（`core_forms`）；
//! 2. 宏调用：Phase 1 执行 transformer，再递归展开产物（直到核心形式）；
//! 3. 标识符：保留符号引用（slot 解析按 §19.3 由编译器执行）。
//!
//! **三个核心不变式**（§19.2）：
//! 1. 展开终止性：展开深度超上限报错而非栈溢出；
//! 2. 卫生性保持：宏引入标识符经 α 重命名，永不与用户标识符串扰；
//! 3. 相位封闭性：Phase 1 transformer 只产生 Phase 0 语法对象。

use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;

use kerf_core::{CoreExpr, LiteralValue};
use kerf_syntax::{Keyword, ScopeId, ScopeSet, Stx, StxDatum, StxLiteral, Symbol, SymbolTable};

use crate::core_forms::expand_core_form;
use crate::macro_sys::{BuiltinTransformers, Transformer, TransformerKind};
use crate::sugar::{
    desugar_and, desugar_cond, desugar_let, desugar_let_star, desugar_letrec, desugar_or,
    desugar_unless, desugar_when, desugar_while,
};

/// 展开器错误（最小共享形态 `{ message, span }`）。
#[derive(Debug, Clone, PartialEq)]
pub struct ExpandError {
    pub message: String,
    pub span: kerf_span::Span,
}

impl ExpandError {
    pub(crate) fn new(message: impl Into<String>, span: kerf_span::Span) -> Self {
        ExpandError {
            message: message.into(),
            span,
        }
    }
}

/// 宏展开深度上限（§19.2 不变式 1：超限报错而非栈溢出）。
///
/// **TD-007 部分解除**（Stage 1 批次 A3）：宏展开链经 trampoline 工作表
/// 迭代化（[`expand_form`] 顶层循环——宏产物头部仍是宏调用时不递归、
/// 循环继续），展开控制流栈深与链长解耦。上限从 128（rustc 递归对齐
/// 校准）提升至 **500**。
///
/// **标定依据**（TD-017 同型实测法，探针 example 实测）：
/// - 2MiB 测试线程：1_000 层通过 / 2_000 层溢出；
/// - 8MiB 主线程（CLI 生产）：4_000 层通过 / 5_000 层溢出；
/// - 残余栈约束来自 Stx 值语义深树的 clone/drop 递归（数据结构层
///   约束，非展开控制流）；
/// - 500 = 最严格交付环境（测试线程 2MiB）实测通过值 1_000 的 2×
///   裕度（TD-017 先例 690→256 同型 2.7×）——不变式 1（超限报错
///   而非栈溢出）须在环境波动下成立，故上限必须落在实测边界内侧。
///
/// 完整文档口径 10_000 依赖 Stx `Rc` 化（Stage 1 前端重写批次 B 范围）
/// ——残留边界登记于 TD-007 注记。
/// 展开深度上限（TD-007 完整口径，批次 H2）：trampoline 工作表（r4）+
/// Stx Rc 共享化（H2——clone O(1)、旧树经共享免深 drop）双解除后，
/// 10_000 层链实测恒定栈深（正例锚点 deep_macro_chain_expands_
/// iteratively）。深度计数语义不变：当前展开路径上的宏展开总数。
pub const MAX_EXPANSION_DEPTH: u32 = 10_000;

/// 展开上下文（§10.1 规则 2：`Ctxt` 后缀）。
pub struct ExpandCtxt {
    /// 共享符号表（唯一可信数据源）。
    pub table: SymbolTable,
    /// 变换器注册表（Phase 1 世界）。
    transformers: HashMap<Symbol, Transformer>,
    /// 当前宏展开深度。
    depth: u32,
    /// 作用域分配器（TD-004/r13：绑定形式 fresh scope 唯一发放处——
    /// 全局单调递增，保证 ScopeId 在一次展开内不重号）。
    next_scope: ScopeId,
}

impl ExpandCtxt {
    /// 构造（含标记符号初始化）。
    pub fn new(mut table: SymbolTable) -> Self {
        BuiltinTransformers::init_markers(&mut table);
        init_keywords(&mut table);
        ExpandCtxt {
            table,
            transformers: HashMap::new(),
            depth: 0,
            next_scope: 1,
        }
    }

    /// 分配 fresh scope（TD-004：绑定形式注入用；0 保留给「无作用域」语义——
    /// 空集 = 全局/内置兑底，不占用）。
    pub fn fresh_scope(&mut self) -> ScopeId {
        let s = self.next_scope;
        self.next_scope += 1;
        s
    }

    /// 登记变换器（define-syntax 的 Phase 1 效果）。
    pub fn register_transformer(&mut self, name: Symbol, transformer: Transformer) {
        self.transformers.insert(name, transformer);
    }

    /// 查询变换器（Stage 0：按名查找；作用域集解析见 TD-004）。
    pub fn lookup_transformer(&self, name: Symbol) -> Option<&Transformer> {
        self.transformers.get(&name)
    }

    /// 语法糖内置变换器判定与注册（惰性：查询时构造）。
    fn builtin_transformer(&mut self, head: Symbol) -> Option<Transformer> {
        let name = self.table.name(head).to_string();
        let f: fn(&[Stx], &mut crate::macro_sys::HygieneCtx) -> Result<Stx, String> =
            match name.as_str() {
                "let" => desugar_let,
                "letrec" => desugar_letrec,
                "let*" => desugar_let_star,
                "cond" => desugar_cond,
                "and" => desugar_and,
                "or" => desugar_or,
                "when" => desugar_when,
                "unless" => desugar_unless,
                "while" => desugar_while,
                _ => return None,
            };
        Some(Transformer {
            kind: TransformerKind::Builtin(f),
            def_scopes: ScopeSet::new(),
        })
    }
}

/// 展开程序（顶层形式序列 → CoreExpr 序列）。
/// 顶层 = 模块体语义：define 合法、define-syntax 合法。
pub fn expand_program(
    forms: &[Stx],
    ctx: &mut ExpandCtxt,
) -> Result<Vec<Rc<CoreExpr>>, ExpandError> {
    let mut out = Vec::new();
    for form in forms {
        out.push(expand_form(form, ctx)?);
    }
    Ok(out)
}

/// 展开单个形式（入口函数，§10.1 规则 1）。
///
/// **宏展开 trampoline**（TD-007 工作表化）：列表头命中变换器（用户宏
/// /内置糖）时展开产物**头部仍是宏调用**则在市层循环继续，不递归——
/// 宏链长度与栈深解耦（10_000 层链恒定栈深）；产物非宏头时进入常规
/// 展开（核心形式/应用——子项递归，深度受 reader 嵌套上限保护）。
/// `ctx.depth` 语义保持「当前展开路径上的宏展开总数」（兄弟不累计：
/// 入口快照、成功出口回滚）。
pub fn expand_form(stx: &Stx, ctx: &mut ExpandCtxt) -> Result<Rc<CoreExpr>, ExpandError> {
    let entry_depth = ctx.depth;
    let mut current: Cow<'_, Stx> = Cow::Borrowed(stx);
    while let Some(expanded) = try_macro_step(&current, ctx)? {
        current = Cow::Owned(expanded);
    }
    let result = match &current.datum {
        StxDatum::Literal(l) => Ok(Rc::new(CoreExpr::Literal {
            value: literal_from_stx(l),
            span: current.span,
        })),
        StxDatum::Symbol(name) => Ok(Rc::new(CoreExpr::Var {
            name: *name,
            scopes: current.scopes.clone(),
            span: current.span,
        })),
        StxDatum::List(items) => expand_list(&current, items, ctx),
        StxDatum::Vector(_) => Err(ExpandError::new(
            "向量不能出现在表达式位置（仅用于绑定组/模式）",
            current.span,
        )),
    };
    // 路径深度回滚：trampoline 链计数不跨兄弟累计（与旧递归版语义等价）
    ctx.depth = entry_depth;
    result
}

/// 宏展开工作表单步（TD-007）：若 `stx` 是宏调用（列表头命中变换器
/// 注册表——含卫生基名回退；或命中内置糖触发集），执行变换器并返回
/// 产物；否则返回 `None`（进入常规展开）。
///
/// 优先级与旧 `expand_list` 一致：用户宏（含糖已注册项）→ 未注册糖名
/// 惰性注册 → 核心形式/应用交回常规展开。深度计数在此累计（链长）。
fn try_macro_step(stx: &Stx, ctx: &mut ExpandCtxt) -> Result<Option<Stx>, ExpandError> {
    let items = match &stx.datum {
        StxDatum::List(items) if !items.is_empty() => items,
        _ => return Ok(None),
    };
    let head = match items[0].datum.as_symbol() {
        Some(s) => s,
        None => return Ok(None),
    };
    // ---- 1. 用户宏（define-syntax 注册的变换器；可覆盖内置语法糖名） ----
    // 卫生回退（D6 修复）：宏模板引入的变换器引用被 α 重命名
    // （`name$hyg$N`）——精确符号未命中变换器表时按**基名**回退查表
    // （03 §2.4 卫生保证(2)「自由标识符穿透」的 Stage 0 近似，与
    // driver 的全局卫生回退同则；修复前宏调宏两路径均未绑定）。
    let resolved = if ctx.lookup_transformer(head).is_some() {
        Some(head)
    } else {
        hygienic_base_symbol(head, &mut ctx.table)
            .filter(|base| ctx.lookup_transformer(*base).is_some())
    };
    let transformer_sym = match resolved {
        Some(sym) => sym,
        // ---- 2. 语法糖内置变换器（惰性注册） ----
        None => {
            if !is_sugar_symbol(head, &ctx.table) {
                return Ok(None);
            }
            let t = match ctx.builtin_transformer(head) {
                Some(t) => t,
                None => return Ok(None),
            };
            ctx.register_transformer(head, t);
            head
        }
    };
    // ---- 深度计数（链长语义；超限报错非栈溢出——§19.2 不变式 1） ----
    ctx.depth += 1;
    if ctx.depth > MAX_EXPANSION_DEPTH {
        ctx.depth -= 1;
        return Err(ExpandError::new(
            format!(
                "宏展开深度超过上限 {}（疑似无限递归展开）",
                MAX_EXPANSION_DEPTH
            ),
            stx.span,
        ));
    }
    let transformer = ctx
        .transformers
        .get(&transformer_sym)
        .cloned()
        .expect("调用方已校验变换器存在");
    let args: &[Stx] = &items[1..];
    let use_scopes = stx.scopes.clone();
    let expanded = transformer
        .apply_named(transformer_sym, args, &mut ctx.table, &use_scopes)
        .map_err(|e| {
            ExpandError::new(
                format!("宏「{}」展开失败：{}", ctx.table.name(transformer_sym), e),
                stx.span,
            )
        })?;
    Ok(Some(expanded))
}

/// 字面量 Stx → CoreExpr 字面量（quote datum 路径复用）。
pub(crate) fn literal_from_stx(l: &StxLiteral) -> LiteralValue {
    match l {
        StxLiteral::Int(v) => LiteralValue::Int(*v),
        StxLiteral::Float(v) => LiteralValue::Float(*v),
        StxLiteral::Str(s) => LiteralValue::Str(s.clone()),
        StxLiteral::Bool(b) => LiteralValue::Bool(*b),
        StxLiteral::Nil => LiteralValue::Nil,
    }
}

fn expand_list(
    stx: &Stx,
    items: &[Stx],
    ctx: &mut ExpandCtxt,
) -> Result<Rc<CoreExpr>, ExpandError> {
    if items.is_empty() {
        return Err(ExpandError::new("空列表不能作为表达式求值", stx.span));
    }
    // 宏头与糖头已由 expand_form 的 trampoline 前置处理（TD-007）——
    // 到达此处的列表必为：核心形式或函数应用。
    // ---- 3. 核心形式（关键字查表，纯结构分派） ----
    if let Some(sym) = items[0].datum.as_symbol() {
        let name = ctx.table.name(sym);
        if let Some(kw) = Keyword::from_name(name) {
            return expand_core_form(kw, stx, items, ctx);
        }
    }
    // ---- 4. 函数应用 ----
    let fn_expr = expand_form(&items[0], ctx)?;
    let mut args = Vec::with_capacity(items.len() - 1);
    for a in &items[1..] {
        args.push(expand_form(a, ctx)?);
    }
    Ok(Rc::new(CoreExpr::Apply {
        fn_expr,
        args,
        span: stx.span,
    }))
}

/// 卫生基名解析：`name$hyg$N` → 基名 Symbol（后缀须为纯数字；
/// 与 driver `resolve_hygiene_fallbacks` 的保守判定同则）。
fn hygienic_base_symbol(sym: Symbol, table: &mut kerf_syntax::SymbolTable) -> Option<Symbol> {
    let name = table.name(sym).to_string();
    let (base, suffix) = name.rsplit_once("$hyg$")?;
    if suffix.is_empty() || !suffix.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(table.intern(base))
}

// ---- 关键词句柄（thread_local 缓存 intern 结果——sugar/core_forms 共用） ----

thread_local! {
    pub(crate) static WORD_LAMBDA: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
    pub(crate) static WORD_SETBANG: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
    pub(crate) static WORD_IF: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
    pub(crate) static WORD_BEGIN: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
    pub(crate) static WORD_LET: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
    pub(crate) static WORD_LETREC: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
    pub(crate) static WORD_ELSE: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
    pub(crate) static WORD_LETSTAR: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
}

/// 初始化核心形式关键词句柄（`ExpandCtxt::new` 自动调用）。
pub(crate) fn init_keywords(table: &mut SymbolTable) {
    let words: Vec<(
        Keyword,
        &'static std::thread::LocalKey<std::cell::RefCell<Symbol>>,
    )> = vec![
        (Keyword::Fn, &WORD_LAMBDA),
        (Keyword::Assign, &WORD_SETBANG),
        (Keyword::If, &WORD_IF),
        (Keyword::Do, &WORD_BEGIN),
        (Keyword::Let, &WORD_LET),
        (Keyword::LetRec, &WORD_LETREC),
        (Keyword::Else, &WORD_ELSE),
        (Keyword::LetStar, &WORD_LETSTAR),
    ];
    for (kw, slot) in words {
        let sym = table.keyword_symbol(kw);
        slot.with(|w| *w.borrow_mut() = sym);
    }
}

pub(crate) fn kw_symbol(
    word: &'static std::thread::LocalKey<std::cell::RefCell<Symbol>>,
) -> Symbol {
    word.with(|w| *w.borrow())
}

// ---- 构造辅助（core_forms/sugar 共用的 Stx 构造底座） ----

pub(crate) fn nil_stx(span: kerf_span::Span) -> Stx {
    Stx::literal(StxLiteral::Nil, span, ScopeSet::new())
}

pub(crate) fn true_stx(span: kerf_span::Span) -> Stx {
    Stx::literal(StxLiteral::Bool(true), span, ScopeSet::new())
}

pub(crate) fn false_stx(span: kerf_span::Span) -> Stx {
    Stx::literal(StxLiteral::Bool(false), span, ScopeSet::new())
}

pub(crate) fn keyword_stx(ctx: &ExpandCtxt, kw: Keyword) -> Stx {
    let sym = ctx.table.keyword_symbol(kw);
    Stx::symbol(sym, kerf_span::Span::dummy(), ScopeSet::new())
}

pub(crate) fn make_setbang(name: Symbol, value: Stx, span: kerf_span::Span) -> Stx {
    // (set! name value) 三元素列表（内部 define 提升路径）
    Stx::list(
        vec![
            Stx::symbol(kw_symbol(&WORD_SETBANG), span, ScopeSet::new()),
            Stx::symbol(name, span, ScopeSet::new()),
            value,
        ],
        span,
        ScopeSet::new(),
    )
}

pub(crate) fn is_head_keyword(stx: &Stx, ctx: &ExpandCtxt, kw: Keyword) -> bool {
    stx.datum
        .as_list()
        .and_then(|l| l.first())
        .and_then(|h| h.datum.as_symbol())
        .map(|s| ctx.table.is_keyword(s, kw))
        .unwrap_or(false)
}

/// 语法糖符号判定（内置变换器通道的触发集合——与 builtin_transformer 的名字集一致）。
fn is_sugar_symbol(sym: Symbol, table: &SymbolTable) -> bool {
    matches!(
        table.name(sym),
        "let" | "letrec" | "let*" | "cond" | "and" | "or" | "when" | "unless" | "while"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kerf_reader::read_source;

    fn ctx() -> ExpandCtxt {
        ExpandCtxt::new(SymbolTable::new())
    }

    fn expand_src(src: &str, c: &mut ExpandCtxt) -> Result<Vec<Rc<CoreExpr>>, ExpandError> {
        let forms = read_source(src, 0, &mut c.table).expect("读取失败");
        expand_program(&forms, c)
    }

    fn render(exprs: &[Rc<CoreExpr>], c: &ExpandCtxt) -> Vec<String> {
        exprs
            .iter()
            .map(|e| e.render(&|s| c.table.name(s).to_string()))
            .collect()
    }

    #[test]
    fn expand_lambda_and_app() {
        let mut c = ctx();
        let out = expand_src("((fn (x) (+ x 1)) 41)", &mut c).unwrap();
        assert_eq!(render(&out, &c), vec!["((fn (x) (+ x 1)) 41)"]);
    }

    #[test]
    fn expand_if_with_implicit_else() {
        let mut c = ctx();
        let out = expand_src("(if true 1)", &mut c).unwrap();
        assert_eq!(render(&out, &c)[0], "(if true 1 nil)");
    }

    #[test]
    fn expand_define_function_sugar() {
        let mut c = ctx();
        let out = expand_src("(define (f x) (* x x))", &mut c).unwrap();
        assert_eq!(render(&out, &c)[0], "(define f (fn (x) (* x x)))");
    }

    #[test]
    fn expand_let_derives_to_lambda_app() {
        let mut c = ctx();
        let out = expand_src("(let ((x 1) (y 2)) (+ x y))", &mut c).unwrap();
        assert_eq!(render(&out, &c)[0], "((fn (x y) (+ x y)) 1 2)");
    }

    #[test]
    fn expand_letrec_derives() {
        let mut c = ctx();
        let out = expand_src("(letrec ((f (fn () 1))) (f))", &mut c).unwrap();
        let r = render(&out, &c)[0].clone();
        assert!(r.starts_with("((fn (f)"), "letrec 推导：{}", r);
        assert!(r.contains("assign f"));
    }

    #[test]
    fn expand_cond_chain() {
        let mut c = ctx();
        let out = expand_src("(cond (false 1) (true 2) (else 3))", &mut c).unwrap();
        let r = render(&out, &c)[0].clone();
        assert!(
            r.starts_with("(if false 1 (if true 2 3))"),
            "cond 推导：{}",
            r
        );
    }

    #[test]
    fn expand_and_or() {
        let mut c = ctx();
        let out = expand_src("(and a b c)", &mut c).unwrap();
        assert_eq!(render(&out, &c)[0], "(if a (if b c false) false)");
        let out2 = expand_src("(or a b)", &mut c).unwrap();
        assert_eq!(render(&out2, &c)[0], "(if a true b)");
    }

    #[test]
    fn expand_while_uses_fresh_loop() {
        let mut c = ctx();
        let out = expand_src("(while (< i 10) (assign i (+ i 1)))", &mut c).unwrap();
        let r = render(&out, &c)[0].clone();
        assert!(r.contains("loop$hyg$"), "while 的 loop 必须唯一化：{}", r);
    }

    #[test]
    fn quote_list_becomes_pairs() {
        let mut c = ctx();
        let out = expand_src("(quote (1 2 3))", &mut c).unwrap();
        assert_eq!(render(&out, &c)[0], "(1 2 3)");
    }

    #[test]
    fn quote_symbol_becomes_symbol_literal() {
        // TD-002 解除：符号 datum → 符号字面量（卫生后缀剥离）
        let mut c = ctx();
        let out = expand_src("(quote sym)", &mut c).unwrap();
        assert_eq!(render(&out, &c)[0], "sym");
        let out2 = expand_src("'sym", &mut c).unwrap();
        assert_eq!(render(&out2, &c)[0], "sym");
    }

    #[test]
    fn inner_define_hoisted() {
        let mut c = ctx();
        let out = expand_src("(fn (x) (define y 2) (+ x y))", &mut c).unwrap();
        let r = render(&out, &c)[0].clone();
        // (fn (x) ((fn (y) (do (assign y 2) (+ x y))) nil))
        assert!(r.contains("assign y 2"), "内部 define 提升：{}", r);
    }

    #[test]
    fn inner_define_after_expr_is_error() {
        let mut c = ctx();
        let err = expand_src("(fn (x) (+ x 1) (define y 2))", &mut c).unwrap_err();
        assert!(err.message.contains("define 必须位于函数体头部"));
    }

    #[test]
    fn expansion_depth_limit() {
        let mut c = ctx();
        // 自指宏：无限展开（模板中的自引用经保留集不重命名）
        let src = "(define-syntax loop2 (syntax-rules () ((loop2) (loop2)))) (loop2)";
        let forms = read_source(src, 0, &mut c.table).unwrap();
        let result = expand_program(&forms, &mut c);
        let err = result.unwrap_err();
        assert!(
            err.message.contains("展开深度"),
            "实际错误：{}",
            err.message
        );
    }

    #[test]
    fn deep_macro_chain_expands_iteratively() {
        // TD-007 完整口径正例（H2）：10_000 层透传宏链 = 文档示例口径。
        // r4 trampoline 解除控制流递归后，残留栈约束为 Stx 值语义深树
        // clone/drop（实测 8MiB 主线程 4_000 通过/5_000 溢出）——H2
        // Rc 共享化（List/Vector → Rc<Vec<Stx>>，clone O(1) + 旧树共享
        // 免深 drop）解除后本测试在 2MiB 测试线程通过（恒定栈深）。
        // 构造链而非源码嵌套（reader 嵌套上限 256 不适用于 Stx 构造）。
        let src = "(define-syntax m (syntax-rules () ((m x) x)))";
        let mut c = ctx();
        let forms = read_source(src, 0, &mut c.table).unwrap();
        expand_program(&forms, &mut c).unwrap(); // 注册透传宏 m
        let m_sym = c.table.intern("m");
        let span = kerf_span::Span::dummy();
        let mut cur = Stx::literal(StxLiteral::Int(42), span, ScopeSet::new());
        for _ in 0..10_000 {
            cur = Stx::list(
                vec![Stx::symbol(m_sym, span, ScopeSet::new()), cur],
                span,
                ScopeSet::new(),
            );
        }
        let out = expand_form(&cur, &mut c).unwrap();
        match out.as_ref() {
            CoreExpr::Literal {
                value: LiteralValue::Int(42),
                ..
            } => {}
            other => panic!("透传 10_000 层链应归约到字面量 42，实际 {:?}", other),
        }
    }

    #[test]
    fn macro_chain_beyond_limit_reports_error() {
        // TD-007 负例（H2 口径）：10_001 层链超上限 → 结构化报错
        // （非栈溢出）。上限语义 = 展开路径上的宏展开总数（10_000 层
        // 链恰好通过、10_001 层报错——边界含头含尾验证）。
        let src = "(define-syntax m (syntax-rules () ((m x) x)))";
        let mut c = ctx();
        let forms = read_source(src, 0, &mut c.table).unwrap();
        expand_program(&forms, &mut c).unwrap();
        let m_sym = c.table.intern("m");
        let span = kerf_span::Span::dummy();
        let mut cur = Stx::literal(StxLiteral::Int(1), span, ScopeSet::new());
        for _ in 0..10_001 {
            cur = Stx::list(
                vec![Stx::symbol(m_sym, span, ScopeSet::new()), cur],
                span,
                ScopeSet::new(),
            );
        }
        let err = expand_form(&cur, &mut c).unwrap_err();
        assert!(
            err.message.contains("超过上限 10000"),
            "实际错误：{}",
            err.message
        );
    }

    #[test]
    fn user_macro_with_hygiene() {
        let mut c = ctx();
        let src = r#"
            (define-syntax swap!
              (syntax-rules ()
                ((swap! a b)
                 (do
                   (assign tmp (quote (1)))
                   (assign a b)))))
            (swap! x y)
        "#;
        let out = expand_src(src, &mut c).unwrap();
        let r = render(&out, &c).join(" ");
        // 引入标识符 tmp 必须被重命名（卫生保证）
        assert!(r.contains("tmp$hyg$"), "引入标识符重命名：{}", r);
    }

    #[test]
    fn module_form_expands() {
        let mut c = ctx();
        let out = expand_src("(module m (import a) (export f) (define f 1) (f))", &mut c).unwrap();
        let r = render(&out, &c)[0].clone();
        assert!(
            r.starts_with("(module m (import a) (export f)"),
            "module：{}",
            r
        );
    }

    #[test]
    fn errors_carry_span() {
        let mut c = ctx();
        let err = expand_src("(if)", &mut c).unwrap_err();
        assert!(!err.span.is_empty() || err.span.start == 0);
    }
}
