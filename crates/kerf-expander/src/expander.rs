//! 展开器（stage0.md §19.2 展开循环骨架的落地实现）。
//!
//! `Stx` → `CoreExpr`：递归展开 9 个核心形式 + 宏调用（变换器）
//! + 语法糖内置变换器（§3.2 推导表）。
//!
//! **展开循环骨架**（§19.2）：
//! 1. 核心形式：不展开自身，只递归展开子节点；
//! 2. 宏调用：Phase 1 执行 transformer，再递归展开产物（直到核心形式）；
//! 3. 标识符：保留符号引用（slot 解析按 §19.3 由编译器执行）。
//!
//! **三个核心不变式**（§19.2）：
//! 1. 展开终止性：展开深度超上限（10_000）报错而非栈溢出；
//! 2. 卫生性保持：宏引入标识符经 α 重命名，永不与用户标识符串扰；
//! 3. 相位封闭性：Phase 1 transformer 只产生 Phase 0 语法对象。

use std::collections::HashMap;
use std::rc::Rc;

use kerf_core::{CoreExpr, LiteralValue};
use kerf_span::Span;
use kerf_syntax::{Keyword, ScopeSet, Stx, StxDatum, StxLiteral, Symbol, SymbolTable};

use crate::macro_sys::{BuiltinTransformers, SyntaxRules, Transformer, TransformerKind};

/// 展开器错误（最小共享形态 `{ message, span }`）。
#[derive(Debug, Clone, PartialEq)]
pub struct ExpandError {
    pub message: String,
    pub span: Span,
}

impl ExpandError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        ExpandError {
            message: message.into(),
            span,
        }
    }
}

/// 宏展开深度上限（§19.2 不变式 1：超限报错而非栈溢出）。
///
/// 取值 128 与 rustc 默认递归上限（recursion_limit）一致——真实宏嵌套深度
/// 远低于此；设计文档示例值 10_000 需要迭代式工作表展开（Stage 1 计划，
/// TD-007：当前递归实现在受限栈环境（测试线程 2MiB）下的安全校准）。
pub const MAX_EXPANSION_DEPTH: u32 = 128;

/// 展开上下文（§10.1 规则 2：`Ctxt` 后缀）。
pub struct ExpandCtxt {
    /// 共享符号表（唯一可信数据源）。
    pub table: SymbolTable,
    /// 变换器注册表（Phase 1 世界）。
    transformers: HashMap<Symbol, Transformer>,
    /// 当前宏展开深度。
    depth: u32,
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
        }
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
pub fn expand_form(stx: &Stx, ctx: &mut ExpandCtxt) -> Result<Rc<CoreExpr>, ExpandError> {
    match &stx.datum {
        StxDatum::Literal(l) => Ok(Rc::new(CoreExpr::Literal {
            value: literal_from_stx(l),
            span: stx.span,
        })),
        StxDatum::Symbol(name) => Ok(Rc::new(CoreExpr::VarRef {
            name: *name,
            span: stx.span,
        })),
        StxDatum::List(items) => expand_list(stx, items, ctx),
        StxDatum::Vector(_) => Err(ExpandError::new(
            "向量不能出现在表达式位置（仅用于绑定组/模式）",
            stx.span,
        )),
    }
}

fn literal_from_stx(l: &StxLiteral) -> LiteralValue {
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
    let head = items[0].datum.as_symbol();
    // ---- 1. 用户宏（define-syntax 注册的变换器；可覆盖内置语法糖名） ----
    if let Some(sym) = head {
        if ctx.lookup_transformer(sym).is_some() {
            return expand_macro_call(stx, items, ctx);
        }
    }
    // ---- 2. 语法糖内置变换器（let/letrec/cond/and/or/when/unless/while/let*） ----
    if let Some(sym) = head {
        if is_sugar_symbol(sym, &ctx.table) {
            if let Some(t) = ctx.builtin_transformer(sym) {
                ctx.register_transformer(sym, t);
                return expand_macro_call(stx, items, ctx);
            }
        }
    }
    // ---- 3. 核心形式（关键字查表，纯结构分派） ----
    if let Some(sym) = head {
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
    Ok(Rc::new(CoreExpr::App {
        fn_expr,
        args,
        span: stx.span,
    }))
}

fn expand_macro_call(
    stx: &Stx,
    items: &[Stx],
    ctx: &mut ExpandCtxt,
) -> Result<Rc<CoreExpr>, ExpandError> {
    let name = items[0].datum.as_symbol().expect("调用方已校验头为符号");
    // 展开终止性（§19.2 不变式 1）
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
        .get(&name)
        .cloned()
        .expect("调用方已校验变换器存在");
    let args: Vec<Stx> = items[1..].to_vec();
    let use_scopes = stx.scopes.clone();
    let expanded = transformer
        .apply_named(name, &args, &mut ctx.table, &use_scopes)
        .map_err(|e| {
            ExpandError::new(
                format!("宏「{}」展开失败：{}", ctx.table.name(name), e),
                stx.span,
            )
        })?;
    let result = expand_form(&expanded, ctx);
    ctx.depth -= 1;
    result
}

fn expand_core_form(
    kw: Keyword,
    stx: &Stx,
    items: &[Stx],
    ctx: &mut ExpandCtxt,
) -> Result<Rc<CoreExpr>, ExpandError> {
    match kw {
        Keyword::Lambda => expand_lambda(stx, items, ctx),
        Keyword::If => expand_if(stx, items, ctx),
        Keyword::SetBang => expand_setbang(stx, items, ctx),
        Keyword::Define => expand_define(stx, items, ctx),
        Keyword::Begin => expand_begin(stx, items, ctx, true),
        Keyword::Module => expand_module(stx, items, ctx),
        Keyword::Quote => expand_quote(stx, items),
        Keyword::Import | Keyword::Export => Err(ExpandError::new(
            "import/export 只能出现在 module 形式内部",
            stx.span,
        )),
        Keyword::Let
        | Keyword::LetRec
        | Keyword::LetStar
        | Keyword::Cond
        | Keyword::And
        | Keyword::Or
        | Keyword::When
        | Keyword::Unless
        | Keyword::While => {
            // 由 builtin_transformer 通道处理（此分支不可达——防御性显式失败）
            Err(ExpandError::new(
                "语法糖形式应经内置变换器处理（内部不变式破坏）",
                stx.span,
            ))
        }
        Keyword::Else => Err(ExpandError::new(
            "else 只能出现在 cond 子句的测试位置",
            stx.span,
        )),
        Keyword::DefineSyntax => expand_define_syntax(stx, items, ctx),
        Keyword::SyntaxRules => Err(ExpandError::new(
            "syntax-rules 只能出现在 define-syntax 内部",
            stx.span,
        )),
    }
}

// ---- 核心形式实现 ----

fn expand_lambda(
    stx: &Stx,
    items: &[Stx],
    ctx: &mut ExpandCtxt,
) -> Result<Rc<CoreExpr>, ExpandError> {
    if items.len() < 3 {
        return Err(ExpandError::new(
            "lambda 形式：(lambda (参数...) 体...)",
            stx.span,
        ));
    }
    let params = parse_params(&items[1])?;
    let body = expand_body(&items[2..], ctx)?;
    Ok(Rc::new(CoreExpr::Lambda {
        params,
        body,
        span: stx.span,
    }))
}

fn parse_params(param_stx: &Stx) -> Result<Vec<Symbol>, ExpandError> {
    let list = param_stx
        .datum
        .as_list()
        .ok_or_else(|| ExpandError::new("lambda 参数必须是符号列表", param_stx.span))?;
    let mut params = Vec::with_capacity(list.len());
    for p in list {
        match p.datum.as_symbol() {
            Some(s) => {
                // A3 卫式（[06-操作语义 §2]）：同名形参在同层只允许出现一次。
                // 展开期检查是两执行路径（eval/VM）的共同上游——单点防御。
                if params.contains(&s) {
                    return Err(ExpandError::new(
                        "lambda 参数重名（同名形参只允许出现一次）",
                        p.span,
                    ));
                }
                params.push(s);
            }
            None => {
                return Err(ExpandError::new(
                    "lambda 参数必须是符号（不支持解构参数）",
                    p.span,
                ))
            }
        }
    }
    Ok(params)
}

/// 函数体展开：内部 define 提升（§19.2 陷阱 3）。
/// `(define x e)... expr...` → `((lambda (x...) (begin (set! x e)... expr...)) nil...)`
fn expand_body(forms: &[Stx], ctx: &mut ExpandCtxt) -> Result<Rc<CoreExpr>, ExpandError> {
    // 切分头部 defines
    let mut defines: Vec<(Symbol, &Stx)> = Vec::new();
    let mut rest_idx = forms.len();
    for (i, f) in forms.iter().enumerate() {
        if is_head_keyword(f, ctx, Keyword::Define) {
            defines.push((define_name(f, ctx)?, f));
        } else {
            rest_idx = i;
            break;
        }
    }
    // define 必须位于头部（任何非头部 define 即报错，显式失败——含无头部 define 的序列）
    for f in &forms[rest_idx..] {
        if is_head_keyword(f, ctx, Keyword::Define) {
            return Err(ExpandError::new(
                "define 必须位于函数体头部（后续 define 不合法）",
                f.span,
            ));
        }
    }
    if defines.is_empty() {
        // 无 define：表达式序列
        let exprs: Result<Vec<Rc<CoreExpr>>, _> =
            forms.iter().map(|f| expand_form(f, ctx)).collect();
        let exprs = exprs?;
        return Ok(wrap_body(exprs, forms));
    }
    // 构造提升式
    let span = forms.first().map(|f| f.span).unwrap_or_default();
    let names: Vec<Symbol> = defines.iter().map(|(n, _)| *n).collect();
    let mut body_forms: Vec<Stx> = Vec::new();
    for (name, def) in &defines {
        // (set! name value)
        let value = define_value_stx(def, ctx)?;
        body_forms.push(make_setbang(*name, value, def.span));
    }
    body_forms.extend(forms[rest_idx..].iter().cloned());
    // ((lambda (names...) body...) nil...)——体形式直接铺平为 lambda 尾部
    let mut lambda_items: Vec<Stx> = vec![
        keyword_stx(ctx, Keyword::Lambda),
        Stx::list(
            names
                .iter()
                .map(|n| Stx::symbol(*n, span, ScopeSet::new()))
                .collect(),
            span,
            ScopeSet::new(),
        ),
    ];
    lambda_items.extend(body_forms);
    let lambda_form = Stx::list(lambda_items, span, ScopeSet::new());
    // app = (lambda nil nil ...)——nil 实参直接铺平
    let mut app_items: Vec<Stx> = vec![lambda_form];
    for _ in names {
        app_items.push(nil_stx(span));
    }
    let app_form = Stx::list(app_items, span, ScopeSet::new());
    expand_form(&app_form, ctx)
}

fn wrap_body(exprs: Vec<Rc<CoreExpr>>, forms: &[Stx]) -> Rc<CoreExpr> {
    let span = forms.first().map(|f| f.span).unwrap_or_default();
    if exprs.len() == 1 {
        return exprs.into_iter().next().expect("单个元素");
    }
    Rc::new(CoreExpr::Begin { body: exprs, span })
}

fn expand_if(stx: &Stx, items: &[Stx], ctx: &mut ExpandCtxt) -> Result<Rc<CoreExpr>, ExpandError> {
    if items.len() != 3 && items.len() != 4 {
        return Err(ExpandError::new(
            "if 形式：(if 条件 真分支 [假分支])",
            stx.span,
        ));
    }
    let cond = expand_form(&items[1], ctx)?;
    let then_branch = expand_form(&items[2], ctx)?;
    let else_branch = if items.len() == 4 {
        expand_form(&items[3], ctx)?
    } else {
        Rc::new(CoreExpr::Literal {
            value: LiteralValue::Nil,
            span: stx.span,
        })
    };
    Ok(Rc::new(CoreExpr::If {
        cond,
        then_branch,
        else_branch,
        span: stx.span,
    }))
}

fn expand_setbang(
    stx: &Stx,
    items: &[Stx],
    ctx: &mut ExpandCtxt,
) -> Result<Rc<CoreExpr>, ExpandError> {
    if items.len() != 3 {
        return Err(ExpandError::new("set! 形式：(set! 名 值)", stx.span));
    }
    let name = items[1]
        .datum
        .as_symbol()
        .ok_or_else(|| ExpandError::new("set! 目标必须是符号", items[1].span))?;
    let value = expand_form(&items[2], ctx)?;
    Ok(Rc::new(CoreExpr::SetBang {
        name,
        value,
        span: stx.span,
    }))
}

fn expand_define(
    stx: &Stx,
    items: &[Stx],
    ctx: &mut ExpandCtxt,
) -> Result<Rc<CoreExpr>, ExpandError> {
    if items.len() < 3 {
        return Err(ExpandError::new(
            "define 形式：(define 名 值) 或 (define (名 参数...) 体...)",
            stx.span,
        ));
    }
    // 函数糖：(define (f x y) body...) → (define f (lambda (x y) body...))——体形式铺平
    if let Some(flist) = items[1].datum.as_list() {
        if let Some(fname) = flist.first().and_then(|f| f.datum.as_symbol()) {
            let mut lambda_items: Vec<Stx> = vec![
                keyword_stx(ctx, Keyword::Lambda),
                Stx::list(flist[1..].to_vec(), stx.span, ScopeSet::new()),
            ];
            lambda_items.extend(items[2..].iter().cloned());
            let lambda_form = Stx::list(lambda_items, stx.span, ScopeSet::new());
            let define_form = Stx::list(
                vec![
                    keyword_stx(ctx, Keyword::Define),
                    Stx::symbol(fname, items[1].span, ScopeSet::new()),
                    lambda_form,
                ],
                stx.span,
                ScopeSet::new(),
            );
            return expand_form(&define_form, ctx);
        }
        return Err(ExpandError::new(
            "define 函数糖首元素必须是符号",
            items[1].span,
        ));
    }
    let name = items[1]
        .datum
        .as_symbol()
        .ok_or_else(|| ExpandError::new("define 目标必须是符号", items[1].span))?;
    if items.len() != 3 {
        return Err(ExpandError::new(
            "define 值形式必须是单个表达式（函数糖请用 (define (名 参数...) 体...)）",
            stx.span,
        ));
    }
    let value = expand_form(&items[2], ctx)?;
    Ok(Rc::new(CoreExpr::Define {
        name,
        value,
        span: stx.span,
    }))
}

fn expand_begin(
    stx: &Stx,
    items: &[Stx],
    ctx: &mut ExpandCtxt,
    _toplevel: bool,
) -> Result<Rc<CoreExpr>, ExpandError> {
    if items.len() < 2 {
        return Ok(Rc::new(CoreExpr::Begin {
            body: vec![],
            span: stx.span,
        }));
    }
    let body: Result<Vec<Rc<CoreExpr>>, _> =
        items[1..].iter().map(|f| expand_form(f, ctx)).collect();
    Ok(Rc::new(CoreExpr::Begin {
        body: body?,
        span: stx.span,
    }))
}

fn expand_module(
    stx: &Stx,
    items: &[Stx],
    ctx: &mut ExpandCtxt,
) -> Result<Rc<CoreExpr>, ExpandError> {
    if items.len() < 2 {
        return Err(ExpandError::new(
            "module 形式：(module 名 [(import ...)] [(export ...)] 体...)",
            stx.span,
        ));
    }
    let name = items[1]
        .datum
        .as_symbol()
        .ok_or_else(|| ExpandError::new("module 名必须是符号", items[1].span))?;
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    let mut body_start = 2;
    // 可选 (import ...) 与 (export ...)
    while body_start < items.len() {
        if let Some(head) = items[body_start]
            .datum
            .as_list()
            .and_then(|l| l.first())
            .and_then(|h| h.datum.as_symbol())
        {
            let kw = Keyword::from_name(ctx.table.name(head));
            match kw {
                Some(Keyword::Import) => {
                    let list = items[body_start].datum.as_list().expect("已检查为列表");
                    for s in &list[1..] {
                        match s.datum.as_symbol() {
                            Some(sym) => imports.push(sym),
                            None => return Err(ExpandError::new("import 项必须是符号", s.span)),
                        }
                    }
                    body_start += 1;
                    continue;
                }
                Some(Keyword::Export) => {
                    let list = items[body_start].datum.as_list().expect("已检查为列表");
                    for s in &list[1..] {
                        match s.datum.as_symbol() {
                            Some(sym) => exports.push(sym),
                            None => return Err(ExpandError::new("export 项必须是符号", s.span)),
                        }
                    }
                    body_start += 1;
                    continue;
                }
                _ => break,
            }
        }
        break;
    }
    let body = expand_program(&items[body_start..], ctx)?;
    Ok(Rc::new(CoreExpr::Module {
        name,
        imports,
        exports,
        body,
        span: stx.span,
    }))
}

fn expand_quote(stx: &Stx, items: &[Stx]) -> Result<Rc<CoreExpr>, ExpandError> {
    if items.len() != 2 {
        return Err(ExpandError::new("quote 形式：(quote 数据)", stx.span));
    }
    let value = datum_to_value(&items[1])?;
    Ok(Rc::new(CoreExpr::Literal {
        value,
        span: stx.span,
    }))
}

/// datum → 字面量值（quote 语义；符号 datum 显式报错——Symbol 值推迟，TD-002）。
fn datum_to_value(stx: &Stx) -> Result<LiteralValue, ExpandError> {
    match &stx.datum {
        StxDatum::Literal(l) => Ok(literal_from_stx(l)),
        StxDatum::List(items) => {
            // 列表 → 右折叠点对
            let mut acc = LiteralValue::Nil;
            for item in items.iter().rev() {
                acc = LiteralValue::Pair(Rc::new(datum_to_value(item)?), Rc::new(acc));
            }
            Ok(acc)
        }
        StxDatum::Symbol(_) => Err(ExpandError::new(
            "quote 符号暂不支持（Symbol 值类型推迟到 Stage 1，TD-002）",
            stx.span,
        )),
        StxDatum::Vector(_) => Err(ExpandError::new(
            "quote 向量暂不支持（Vector 值类型推迟，TD-002）",
            stx.span,
        )),
    }
}

fn expand_define_syntax(
    stx: &Stx,
    items: &[Stx],
    ctx: &mut ExpandCtxt,
) -> Result<Rc<CoreExpr>, ExpandError> {
    if items.len() != 3 {
        return Err(ExpandError::new(
            "define-syntax 形式：(define-syntax 名 (syntax-rules ...))",
            stx.span,
        ));
    }
    let name = items[1]
        .datum
        .as_symbol()
        .ok_or_else(|| ExpandError::new("define-syntax 名必须是符号", items[1].span))?;
    let rules_stx = &items[2];
    // 头必须是 syntax-rules
    let head_ok = rules_stx
        .datum
        .as_list()
        .and_then(|l| l.first())
        .and_then(|h| h.datum.as_symbol())
        .map(|s| ctx.table.is_keyword(s, Keyword::SyntaxRules))
        .unwrap_or(false);
    if !head_ok {
        return Err(ExpandError::new(
            "define-syntax 第二参数必须是 (syntax-rules ...) 形式",
            rules_stx.span,
        ));
    }
    let rules = SyntaxRules::parse(rules_stx)
        .map_err(|e| ExpandError::new(format!("syntax-rules 解析失败：{}", e), rules_stx.span))?;
    ctx.register_transformer(
        name,
        Transformer {
            kind: TransformerKind::Rules(rules),
            def_scopes: stx.scopes.clone(),
        },
    );
    // Phase 1 形式：运行期为无操作（nil 字面量）
    Ok(Rc::new(CoreExpr::Literal {
        value: LiteralValue::Nil,
        span: stx.span,
    }))
}

// ---- 内置语法糖变换器（§3.2 推导表；输出 Stx 后经展开器再展开） ----
//
// 约定：这些函数是 TransformerKind::Builtin 的载体（签名一致）。
// 引入标识符（loop/tmp 等）在此直接构造为普通符号——真正需要卫生的
// 场景由 while 的 loop 与 cond 的 gensym 路径处理（经 intro 通道唯一化）。

fn desugar_let(args: &[Stx], _h: &mut crate::macro_sys::HygieneCtx) -> Result<Stx, String> {
    // (let ((x e)...) body...) → ((lambda (x...) body...) e...)
    if args.len() < 2 {
        return Err("let 形式：(let ((名 值)...) 体...)".to_string());
    }
    let bindings = args[0].datum.as_list().ok_or("let 绑定组必须是列表")?;
    let span = args[0].span;
    let mut names = Vec::new();
    let mut values = Vec::new();
    for b in bindings {
        let pair = b
            .datum
            .as_list()
            .filter(|p| p.len() == 2)
            .ok_or("let 绑定必须是 (名 值) 二元列表")?;
        let name = pair[0].datum.as_symbol().ok_or("let 绑定名必须是符号")?;
        names.push(Stx::symbol(name, pair[0].span, pair[0].scopes.clone()));
        values.push(pair[1].clone());
    }
    let scopes = args[0].scopes.clone();
    let param_list = Stx::list(names, span, scopes.clone());
    let mut lambda_items: Vec<Stx> = vec![
        Stx::symbol(kw_symbol(&WORD_LAMBDA), span, scopes.clone()),
        param_list,
    ];
    lambda_items.extend(args[1..].iter().cloned());
    let lambda = Stx::list(lambda_items, span, scopes);
    let app = Stx::list(
        {
            let mut v = vec![lambda];
            v.extend(values);
            v
        },
        span,
        ScopeSet::new(),
    );
    Ok(app)
}

thread_local! {
    static WORD_LAMBDA: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
    static WORD_SETBANG: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
    static WORD_IF: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
    static WORD_BEGIN: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
    static WORD_LET: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
    static WORD_LETREC: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
    static WORD_ELSE: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
    static WORD_LETSTAR: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
}

/// 初始化核心形式关键词句柄（`ExpandCtxt::new` 自动调用）。
pub(crate) fn init_keywords(table: &mut SymbolTable) {
    let words: Vec<(
        Keyword,
        &'static std::thread::LocalKey<std::cell::RefCell<Symbol>>,
    )> = vec![
        (Keyword::Lambda, &WORD_LAMBDA),
        (Keyword::SetBang, &WORD_SETBANG),
        (Keyword::If, &WORD_IF),
        (Keyword::Begin, &WORD_BEGIN),
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

fn kw_symbol(word: &'static std::thread::LocalKey<std::cell::RefCell<Symbol>>) -> Symbol {
    word.with(|w| *w.borrow())
}

fn desugar_letrec(args: &[Stx], _h: &mut crate::macro_sys::HygieneCtx) -> Result<Stx, String> {
    // (letrec ((f e)...) body...) → ((lambda (f...) (begin (set! f e)... body...)) nil...)
    if args.len() < 2 {
        return Err("letrec 形式：(letrec ((名 值)...) 体...)".to_string());
    }
    let bindings = args[0].datum.as_list().ok_or("letrec 绑定组必须是列表")?;
    let span = args[0].span;
    let scopes = args[0].scopes.clone();
    let mut names = Vec::new();
    let mut sets = Vec::new();
    for b in bindings {
        let pair = b
            .datum
            .as_list()
            .filter(|p| p.len() == 2)
            .ok_or("letrec 绑定必须是 (名 值) 二元列表")?;
        let name = pair[0].datum.as_symbol().ok_or("letrec 绑定名必须是符号")?;
        names.push(name);
        sets.push(Stx::list(
            vec![
                Stx::symbol(kw_symbol(&WORD_SETBANG), span, scopes.clone()),
                Stx::symbol(name, pair[0].span, scopes.clone()),
                pair[1].clone(),
            ],
            pair[0].span,
            scopes.clone(),
        ));
    }
    let mut inner = vec![Stx::symbol(kw_symbol(&WORD_BEGIN), span, scopes.clone())];
    inner.extend(sets);
    inner.extend(args[1..].iter().cloned());
    let body = Stx::list(inner, span, scopes.clone());
    let param_list = Stx::list(
        names
            .iter()
            .map(|n| Stx::symbol(*n, span, scopes.clone()))
            .collect(),
        span,
        scopes.clone(),
    );
    let lambda = Stx::list(
        vec![
            Stx::symbol(kw_symbol(&WORD_LAMBDA), span, scopes.clone()),
            param_list,
            body,
        ],
        span,
        scopes.clone(),
    );
    // app = (lambda nil nil ...)——nil 实参直接铺平（不包列表！）
    let mut app_items: Vec<Stx> = vec![lambda];
    for _ in names {
        app_items.push(nil_stx(span));
    }
    Ok(Stx::list(app_items, span, scopes))
}

fn desugar_let_star(args: &[Stx], h: &mut crate::macro_sys::HygieneCtx) -> Result<Stx, String> {
    // (let* ((x e) rest...) body...) → (let ((x e)) (let* (rest...) body...))
    // (let* () body...) → (let () body...)
    if args.len() < 2 {
        return Err("let* 形式：(let* ((名 值)...) 体...)".to_string());
    }
    let bindings = args[0]
        .datum
        .as_list()
        .ok_or("let* 绑定组必须是列表")?
        .to_vec();
    let span = args[0].span;
    let scopes = args[0].scopes.clone();
    if bindings.is_empty() {
        return desugar_let(args, h);
    }
    let first = bindings[0].clone();
    let rest = bindings[1..].to_vec();
    let rest_list = Stx::list(rest, span, scopes.clone());
    let inner = Stx::list(
        {
            let mut v = vec![
                Stx::symbol(kw_symbol(&WORD_LETSTAR), span, scopes.clone()),
                rest_list,
            ];
            v.extend(args[1..].iter().cloned());
            v
        },
        span,
        scopes.clone(),
    );
    Ok(Stx::list(
        vec![
            Stx::symbol(kw_symbol(&WORD_LET), span, scopes.clone()),
            Stx::list(vec![first], span, scopes.clone()),
            inner,
        ],
        span,
        scopes,
    ))
}

fn desugar_cond(args: &[Stx], _h: &mut crate::macro_sys::HygieneCtx) -> Result<Stx, String> {
    // (cond (c1 e1) (c2 e2) (else e3)) → 嵌套 if；子句 (test) → test
    if args.is_empty() {
        return Err("cond 需要至少一个子句".to_string());
    }
    let scopes = args[0].scopes.clone();
    let mut chain: Option<Stx> = None; // 从右向左构造
    for clause in args.iter().rev() {
        let pair = clause
            .datum
            .as_list()
            .filter(|p| !p.is_empty())
            .ok_or("cond 子句必须是非空列表")?;
        let is_else = pair[0]
            .datum
            .as_symbol()
            .map(is_else_symbol)
            .unwrap_or(false);
        let body = if pair.len() == 1 {
            // (test) → 返回测试值本身
            pair[0].clone()
        } else if pair.len() == 2 {
            // 单表达式子句：直接使用（与经典推导一致）
            pair[1].clone()
        } else {
            Stx::list(
                {
                    let mut v = vec![Stx::symbol(
                        kw_symbol(&WORD_BEGIN),
                        pair[0].span,
                        scopes.clone(),
                    )];
                    v.extend(pair[1..].iter().cloned());
                    v
                },
                pair[0].span,
                scopes.clone(),
            )
        };
        let node = if is_else {
            body
        } else {
            // 检查链尾是否需要 else 分支（无 else 的 cond 尾部为 nil）
            match &chain {
                None => Stx::list(
                    vec![
                        Stx::symbol(kw_symbol(&WORD_IF), pair[0].span, scopes.clone()),
                        pair[0].clone(),
                        body,
                        nil_stx(pair[0].span),
                    ],
                    pair[0].span,
                    scopes.clone(),
                ),
                Some(tail) => Stx::list(
                    vec![
                        Stx::symbol(kw_symbol(&WORD_IF), pair[0].span, scopes.clone()),
                        pair[0].clone(),
                        body,
                        tail.clone(),
                    ],
                    pair[0].span,
                    scopes.clone(),
                ),
            }
        };
        chain = Some(node);
    }
    Ok(chain.expect("args 非空已保证"))
}

fn is_else_symbol(s: Symbol) -> bool {
    WORD_ELSE.with(|w| *w.borrow() == s)
}

fn desugar_and(args: &[Stx], _h: &mut crate::macro_sys::HygieneCtx) -> Result<Stx, String> {
    // (and) → true；(and a) → a；(and a b...) → (if a (and b...) false)
    let span = args.first().map(|a| a.span).unwrap_or_default();
    let scopes = args.first().map(|a| a.scopes.clone()).unwrap_or_default();
    if args.is_empty() {
        return Ok(true_stx(span));
    }
    if args.len() == 1 {
        return Ok(args[0].clone());
    }
    let rest = desugar_and(&args[1..], _h)?;
    Ok(Stx::list(
        vec![
            Stx::symbol(kw_symbol(&WORD_IF), span, scopes.clone()),
            args[0].clone(),
            rest,
            false_stx(span),
        ],
        span,
        scopes,
    ))
}

fn desugar_or(args: &[Stx], _h: &mut crate::macro_sys::HygieneCtx) -> Result<Stx, String> {
    // (or) → false；(or a) → a；(or a b...) → (if a true (or b...))（§3.2 推导规范）
    let span = args.first().map(|a| a.span).unwrap_or_default();
    let scopes = args.first().map(|a| a.scopes.clone()).unwrap_or_default();
    if args.is_empty() {
        return Ok(false_stx(span));
    }
    if args.len() == 1 {
        return Ok(args[0].clone());
    }
    let rest = desugar_or(&args[1..], _h)?;
    Ok(Stx::list(
        vec![
            Stx::symbol(kw_symbol(&WORD_IF), span, scopes.clone()),
            args[0].clone(),
            true_stx(span),
            rest,
        ],
        span,
        scopes,
    ))
}

fn desugar_when(args: &[Stx], _h: &mut crate::macro_sys::HygieneCtx) -> Result<Stx, String> {
    // (when c body...) → (if c (begin body...) nil)
    if args.is_empty() {
        return Err("when 形式：(when 条件 体...)".to_string());
    }
    let span = args[0].span;
    let scopes = args[0].scopes.clone();
    let body = Stx::list(
        {
            let mut v = vec![Stx::symbol(kw_symbol(&WORD_BEGIN), span, scopes.clone())];
            v.extend(args[1..].iter().cloned());
            v
        },
        span,
        scopes.clone(),
    );
    Ok(Stx::list(
        vec![
            Stx::symbol(kw_symbol(&WORD_IF), span, scopes.clone()),
            args[0].clone(),
            body,
            nil_stx(span),
        ],
        span,
        scopes,
    ))
}

fn desugar_unless(args: &[Stx], _h: &mut crate::macro_sys::HygieneCtx) -> Result<Stx, String> {
    // (unless c body...) → (if c nil (begin body...))
    if args.is_empty() {
        return Err("unless 形式：(unless 条件 体...)".to_string());
    }
    let span = args[0].span;
    let scopes = args[0].scopes.clone();
    let body = Stx::list(
        {
            let mut v = vec![Stx::symbol(kw_symbol(&WORD_BEGIN), span, scopes.clone())];
            v.extend(args[1..].iter().cloned());
            v
        },
        span,
        scopes.clone(),
    );
    Ok(Stx::list(
        vec![
            Stx::symbol(kw_symbol(&WORD_IF), span, scopes.clone()),
            args[0].clone(),
            nil_stx(span),
            body,
        ],
        span,
        scopes,
    ))
}

fn desugar_while(args: &[Stx], h: &mut crate::macro_sys::HygieneCtx) -> Result<Stx, String> {
    // (while c body...) → (letrec ((loop (lambda () (if c (begin body... (loop)) nil)))) (loop))
    if args.is_empty() {
        return Err("while 形式：(while 条件 体...)".to_string());
    }
    let span = args[0].span;
    let scopes = args[0].scopes.clone();
    // loop 为引入标识符 → 卫生重命名（唯一化符号）
    let loop_base = h.table().intern("loop");
    let loop_sym = h.fresh_symbol(loop_base);
    let loop_call = Stx::list(
        vec![Stx::symbol(loop_sym, span, scopes.clone())],
        span,
        scopes.clone(),
    );
    let mut body_items: Vec<Stx> = vec![Stx::symbol(kw_symbol(&WORD_BEGIN), span, scopes.clone())];
    body_items.extend(args[1..].iter().cloned());
    body_items.push(loop_call);
    let body = Stx::list(body_items, span, scopes.clone());
    let if_form = Stx::list(
        vec![
            Stx::symbol(kw_symbol(&WORD_IF), span, scopes.clone()),
            args[0].clone(),
            body,
            nil_stx(span),
        ],
        span,
        scopes.clone(),
    );
    let lambda = Stx::list(
        vec![
            Stx::symbol(kw_symbol(&WORD_LAMBDA), span, scopes.clone()),
            Stx::list(vec![], span, scopes.clone()),
            if_form,
        ],
        span,
        scopes.clone(),
    );
    let binding = Stx::list(
        vec![Stx::list(
            vec![Stx::symbol(loop_sym, span, scopes.clone()), lambda],
            span,
            scopes.clone(),
        )],
        span,
        scopes.clone(),
    );
    let letrec_call = Stx::list(
        vec![
            Stx::symbol(kw_symbol(&WORD_LETREC), span, scopes.clone()),
            binding,
            Stx::list(
                vec![Stx::symbol(loop_sym, span, scopes.clone())],
                span,
                scopes.clone(),
            ),
        ],
        span,
        scopes,
    );
    Ok(letrec_call)
}

// ---- 小工具 ----

fn nil_stx(span: Span) -> Stx {
    Stx::literal(StxLiteral::Nil, span, ScopeSet::new())
}

fn true_stx(span: Span) -> Stx {
    Stx::literal(StxLiteral::Bool(true), span, ScopeSet::new())
}

fn false_stx(span: Span) -> Stx {
    Stx::literal(StxLiteral::Bool(false), span, ScopeSet::new())
}

fn keyword_stx(ctx: &ExpandCtxt, kw: Keyword) -> Stx {
    let sym = ctx.table.keyword_symbol(kw);
    Stx::symbol(sym, Span::dummy(), ScopeSet::new())
}

fn make_setbang(name: Symbol, value: Stx, span: Span) -> Stx {
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

fn is_head_keyword(stx: &Stx, ctx: &ExpandCtxt, kw: Keyword) -> bool {
    stx.datum
        .as_list()
        .and_then(|l| l.first())
        .and_then(|h| h.datum.as_symbol())
        .map(|s| ctx.table.is_keyword(s, kw))
        .unwrap_or(false)
}

fn define_name(def: &Stx, _ctx: &ExpandCtxt) -> Result<Symbol, ExpandError> {
    let items = def
        .datum
        .as_list()
        .ok_or_else(|| ExpandError::new("define 必须是列表", def.span))?;
    if items.len() < 2 {
        return Err(ExpandError::new("define 形式不完整", def.span));
    }
    // 函数糖：(define (f x) ...)
    if let Some(flist) = items[1].datum.as_list() {
        return flist
            .first()
            .and_then(|f| f.datum.as_symbol())
            .ok_or_else(|| ExpandError::new("define 函数糖首元素必须是符号", items[1].span));
    }
    items[1]
        .datum
        .as_symbol()
        .ok_or_else(|| ExpandError::new("define 目标必须是符号", items[1].span))
}

fn define_value_stx(def: &Stx, ctx: &ExpandCtxt) -> Result<Stx, ExpandError> {
    // 返回 define 的「值」部分；函数糖规范化为 (define name (lambda ...))
    let items = def
        .datum
        .as_list()
        .ok_or_else(|| ExpandError::new("define 必须是列表", def.span))?;
    if items.len() < 2 {
        return Err(ExpandError::new("define 形式不完整", def.span));
    }
    // 函数糖：(define (f x) body...) → 构造 (define f (lambda (x) body...))
    if let Some(flist) = items[1].datum.as_list() {
        if flist.first().and_then(|f| f.datum.as_symbol()).is_none() {
            return Err(ExpandError::new(
                "define 函数糖首元素必须是符号",
                items[1].span,
            ));
        }
        let mut lambda_items: Vec<Stx> = vec![
            keyword_stx(ctx, Keyword::Lambda),
            Stx::list(flist[1..].to_vec(), items[1].span, ScopeSet::new()),
        ];
        lambda_items.extend(items[2..].iter().cloned());
        let lambda_form = Stx::list(lambda_items, items[1].span, ScopeSet::new());
        // 返回等价 define 的值（lambda 语法形式——由提升路径统一再展开）
        return Ok(lambda_form);
    }
    if items.len() == 3 {
        Ok(items[2].clone())
    } else {
        Err(ExpandError::new("define 值形式必须是单个表达式", def.span))
    }
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
        let out = expand_src("((lambda (x) (+ x 1)) 41)", &mut c).unwrap();
        assert_eq!(render(&out, &c), vec!["((lambda (x) (+ x 1)) 41)"]);
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
        assert_eq!(render(&out, &c)[0], "(define f (lambda (x) (* x x)))");
    }

    #[test]
    fn expand_let_derives_to_lambda_app() {
        let mut c = ctx();
        let out = expand_src("(let ((x 1) (y 2)) (+ x y))", &mut c).unwrap();
        assert_eq!(render(&out, &c)[0], "((lambda (x y) (+ x y)) 1 2)");
    }

    #[test]
    fn expand_letrec_derives() {
        let mut c = ctx();
        let out = expand_src("(letrec ((f (lambda () 1))) (f))", &mut c).unwrap();
        let r = render(&out, &c)[0].clone();
        assert!(r.starts_with("((lambda (f)"), "letrec 推导：{}", r);
        assert!(r.contains("set! f"));
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
        let out = expand_src("(while (< i 10) (set! i (+ i 1)))", &mut c).unwrap();
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
    fn quote_symbol_is_explicit_error() {
        let mut c = ctx();
        let err = expand_src("(quote sym)", &mut c).unwrap_err();
        assert!(err.message.contains("TD-002"));
    }

    #[test]
    fn inner_define_hoisted() {
        let mut c = ctx();
        let out = expand_src("(lambda (x) (define y 2) (+ x y))", &mut c).unwrap();
        let r = render(&out, &c)[0].clone();
        // (lambda (x) ((lambda (y) (begin (set! y 2) (+ x y))) nil))
        assert!(r.contains("set! y 2"), "内部 define 提升：{}", r);
    }

    #[test]
    fn inner_define_after_expr_is_error() {
        let mut c = ctx();
        let err = expand_src("(lambda (x) (+ x 1) (define y 2))", &mut c).unwrap_err();
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
    fn user_macro_with_hygiene() {
        let mut c = ctx();
        let src = r#"
            (define-syntax swap!
              (syntax-rules ()
                ((swap! a b)
                 (begin
                   (set! tmp (quote (1)))
                   (set! a b)))))
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

/// 语法糖符号判定（内置变换器通道的触发集合——与 builtin_transformer 的名字集一致）。
fn is_sugar_symbol(sym: Symbol, table: &SymbolTable) -> bool {
    matches!(
        table.name(sym),
        "let" | "letrec" | "let*" | "cond" | "and" | "or" | "when" | "unless" | "while"
    )
}
