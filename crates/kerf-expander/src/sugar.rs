//! 内置语法糖变换器（stage0.md §3.2 推导表；TD-012 拆分自 expander.rs）。
//!
//! 职责边界（sop.md §11 接口隔离）：
//! - **本模块**：`TransformerKind::Builtin` 的载体——let/letrec/let*/
//!   cond/and/or/when/unless/while 九种糖的 Stx → Stx 推导（输出经
//!   展开器主控再展开到核心形式）；
//! - `expander`（主控）：入口分派与公共构造辅助（`kw_symbol`/字面量
//!   构造/`WORD_*` 关键词句柄）；
//! - `core_forms`：9 核心形式展开（糖的推导目标）。
//!
//! 约定：这些函数是 `TransformerKind::Builtin` 的载体（签名一致）。
//! 引入标识符（loop/tmp 等）在此直接构造为普通符号——真正需要卫生的
//! 场景由 while 的 loop 与 cond 的 gensym 路径处理（经 intro 通道唯一化）。

use kerf_syntax::{Stx, Symbol};

use crate::expander::{
    false_stx, kw_symbol, nil_stx, true_stx, WORD_BEGIN, WORD_ELSE, WORD_IF, WORD_LAMBDA, WORD_LET,
    WORD_LETREC, WORD_LETSTAR, WORD_SETBANG,
};
use kerf_syntax::ScopeSet;

pub(crate) fn desugar_let(
    args: &[Stx],
    _h: &mut crate::macro_sys::HygieneCtx,
) -> Result<Stx, String> {
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

pub(crate) fn desugar_letrec(
    args: &[Stx],
    _h: &mut crate::macro_sys::HygieneCtx,
) -> Result<Stx, String> {
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

pub(crate) fn desugar_let_star(
    args: &[Stx],
    h: &mut crate::macro_sys::HygieneCtx,
) -> Result<Stx, String> {
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

pub(crate) fn desugar_cond(
    args: &[Stx],
    _h: &mut crate::macro_sys::HygieneCtx,
) -> Result<Stx, String> {
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

pub(crate) fn desugar_and(
    args: &[Stx],
    _h: &mut crate::macro_sys::HygieneCtx,
) -> Result<Stx, String> {
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

pub(crate) fn desugar_or(
    args: &[Stx],
    _h: &mut crate::macro_sys::HygieneCtx,
) -> Result<Stx, String> {
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

pub(crate) fn desugar_when(
    args: &[Stx],
    _h: &mut crate::macro_sys::HygieneCtx,
) -> Result<Stx, String> {
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

pub(crate) fn desugar_unless(
    args: &[Stx],
    _h: &mut crate::macro_sys::HygieneCtx,
) -> Result<Stx, String> {
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

pub(crate) fn desugar_while(
    args: &[Stx],
    h: &mut crate::macro_sys::HygieneCtx,
) -> Result<Stx, String> {
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
