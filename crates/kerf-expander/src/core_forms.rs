//! 9 核心形式展开（stage0.md §19.2 核心形式分派；TD-012 拆分自 expander.rs）。
//!
//! 职责边界（sop.md §11 接口隔离）：
//! - **本模块**：关键字查表后的纯结构分派与 9 核心形式实现
//!   （lambda/if/set!/define/begin/module/quote/define-syntax + import/export
//!   位置校验）+ 函数体 define 提升路径；
//! - `expander`（主控）：入口分派（宏 → 糖 → 核心形式 → 函数应用）与
//!   公共类型（`ExpandCtxt`/`ExpandError`）；
//! - `sugar`：语法糖内置变换器（§3.2 推导表）；
//! - `macro_sys`/`phase`：变换器机制与相位驱动（不变）。
//!
//! 核心形式展开不展开自身，只递归展开子节点（§19.2 骨架规则 1）。

use std::rc::Rc;

use kerf_core::{Capability, CoreExpr, LiteralValue};
use kerf_syntax::{Keyword, ScopeSet, Stx, StxDatum, SymbolTable};

use crate::expander::{
    expand_form, expand_program, is_head_keyword, keyword_stx, make_setbang, nil_stx, ExpandCtxt,
    ExpandError,
};
use crate::macro_sys::{SyntaxRules, Transformer, TransformerKind};

/// 核心形式分派（关键字 → 对应展开器；纯结构分派，无宏语义）。
pub(crate) fn expand_core_form(
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
        Keyword::Require => expand_require(stx, items, ctx),
        Keyword::Quote => expand_quote(stx, items, ctx),
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

fn parse_params(param_stx: &Stx) -> Result<Vec<kerf_syntax::Symbol>, ExpandError> {
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
    let mut defines: Vec<(kerf_syntax::Symbol, &Stx)> = Vec::new();
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
    let names: Vec<kerf_syntax::Symbol> = defines.iter().map(|(n, _)| *n).collect();
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
                // _ 臂理由：头部非 import/export（define 等体形式或非关键字符号头）——可选头部区结束，余下为模块体
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

/// `(require io read|write ...)` 展开（r8 能力声明——13 §3.1.3 基础传递
/// 的程序侧声明面）。
///
/// 形状契约：`(require <主体> <能力>...)`；主体当前仅 `io`（Stage 2 扩
/// 展 net/process——数据驱动扩展面）；能力项 `read`/`write`。声明为幂
/// 等集合语义（重复项去重）；产出 `CoreExpr::Require`（零运行时语义，
/// 供 driver R9 权限验证与令牌铸造消费）。
fn expand_require(
    stx: &Stx,
    items: &[Stx],
    ctx: &mut ExpandCtxt,
) -> Result<Rc<CoreExpr>, ExpandError> {
    if items.len() < 3 {
        return Err(ExpandError::new(
            "require 形式：(require io read|write)——主体与能力项不可缺省",
            stx.span,
        ));
    }
    let subject = items[1]
        .datum
        .as_symbol()
        .ok_or_else(|| ExpandError::new("require 主体必须是符号（当前仅 io）", items[1].span))?;
    let subject_name = ctx.table.name(subject).to_string();
    if subject_name != "io" {
        return Err(ExpandError::new(
            format!(
                "未知能力主体「{}」（当前仅支持 io；net/process 属 Stage 2）",
                subject_name
            ),
            items[1].span,
        ));
    }
    let mut caps: Vec<Capability> = Vec::new();
    for item in &items[2..] {
        let cap_sym = item
            .datum
            .as_symbol()
            .ok_or_else(|| ExpandError::new("require 能力项必须是符号（read/write）", item.span))?;
        let cap_name = ctx.table.name(cap_sym).to_string();
        let cap = match cap_name.as_str() {
            "read" => Capability::IoRead,
            "write" => Capability::IoWrite,
            other => {
                return Err(ExpandError::new(
                    format!("未知能力项「{}」（当前仅 read/write）", other),
                    item.span,
                ))
            }
        };
        if !caps.contains(&cap) {
            caps.push(cap);
        }
    }
    Ok(Rc::new(CoreExpr::Require {
        caps,
        span: stx.span,
    }))
}

fn expand_quote(stx: &Stx, items: &[Stx], ctx: &ExpandCtxt) -> Result<Rc<CoreExpr>, ExpandError> {
    if items.len() != 2 {
        return Err(ExpandError::new("quote 形式：(quote 数据)", stx.span));
    }
    let value = datum_to_value(&items[1], &ctx.table)?;
    Ok(Rc::new(CoreExpr::Literal {
        value,
        span: stx.span,
    }))
}

/// datum → 字面量值（quote 语义；符号 datum → 符号值（基名剥离卫生后缀）
/// ——TD-002；向量 datum 仍显式报错）。
fn datum_to_value(stx: &Stx, table: &SymbolTable) -> Result<LiteralValue, ExpandError> {
    match &stx.datum {
        StxDatum::Literal(l) => Ok(crate::expander::literal_from_stx(l)),
        StxDatum::List(items) => {
            // 列表 → 右折叠点对
            let mut acc = LiteralValue::Nil;
            for item in items.iter().rev() {
                acc = LiteralValue::Pair(Rc::new(datum_to_value(item, table)?), Rc::new(acc));
            }
            Ok(acc)
        }
        StxDatum::Symbol(sym) => {
            // 符号值的名字 = 用户可见名：卫生重命名（$hyg$N 后缀）是实现细节，
            // 剥离后给出 Racket 语义近似（scope-set 解析落地前的显式近似，
            // 与 driver::resolve_hygiene_fallbacks 同一剥离口径）。
            let name = sym.as_str(table);
            let base = strip_hygiene_suffix(name);
            Ok(LiteralValue::Symbol(Rc::from(base)))
        }
        StxDatum::Vector(_) => Err(ExpandError::new(
            "quote 向量暂不支持（Vector 值类型推迟，后续阶段）",
            stx.span,
        )),
    }
}

/// 卫生后缀剥离（`name$hyg$N` → `name`；无后缀原样返回）。
/// 与 driver 的全局解析回退同一口径（TD-004 的名称基显式近似）。
fn strip_hygiene_suffix(name: &str) -> &str {
    match name.rsplit_once("$hyg$") {
        Some((base, _)) => base,
        None => name,
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

fn define_name(def: &Stx, _ctx: &ExpandCtxt) -> Result<kerf_syntax::Symbol, ExpandError> {
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
