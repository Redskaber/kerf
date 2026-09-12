//! `CoreExpr`：9 个正交核心原语（stage0.md §3.2 最终定义——名面 r44/
//! E5 S3 M-R 臂同词根化：变体名 = 表面关键字词根，语义零变更）。
//!
//! ```ocaml
//! type core_expr =
//!   | Fn of { params : string list; body : core_expr }        (* 表面 fn *)
//!   | Apply of { fn_expr : core_expr; args : core_expr list } (* 调用——无关键字 *)
//!   | If of { cond; then_branch; else_branch }
//!   | Var of string                                          (* 引用——无关键字 *)
//!   | Literal of literal_value                               (* 字面——无关键字 *)
//!   | Assign of { name : string; value : core_expr }          (* 表面 assign *)
//!   | Define of { name : string; value : core_expr }
//!   | Do of core_expr list                                    (* 表面 do *)
//!   | Module of { name; imports; exports; body }
//!   | Require of { caps : capability list }   (* r8：能力声明，零运行时语义 *)
//! ```
//!
//! 每个 AST 节点携带 Span（§8.6：Span 全管线传播，编译期段）。

use std::rc::Rc;

use kerf_span::Span;
use kerf_syntax::{ScopeSet, Symbol};

/// 核心层字面量值（§3.2 `literal_value`）。
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    Bool(bool),
    Nil,
    /// 符号值（quote 符号 datum——TD-002；存剥离卫生后缀的基名）。
    Symbol(Rc<str>),
    Pair(Rc<LiteralValue>, Rc<LiteralValue>),
}

impl LiteralValue {
    /// 数据化渲染（quote 产物的打印形式）。
    pub fn render(&self) -> String {
        match self {
            LiteralValue::Int(i) => i.to_string(),
            LiteralValue::Float(f) => {
                if f.fract() == 0.0 && f.is_finite() {
                    format!("{:.1}", f)
                } else {
                    f.to_string()
                }
            }
            LiteralValue::Str(s) => format!("{:?}", s),
            LiteralValue::Bool(b) => b.to_string(),
            LiteralValue::Nil => "nil".to_string(),
            LiteralValue::Symbol(s) => s.to_string(),
            LiteralValue::Pair(a, b) => {
                // 经典 Scheme 列表打印：非平凡尾列表用点对表示
                let mut parts = vec![render_elem(a)];
                let mut tail = b;
                loop {
                    match tail.as_ref() {
                        LiteralValue::Pair(hd, tl) => {
                            parts.push(render_elem(hd));
                            tail = tl;
                        }
                        LiteralValue::Nil => return format!("({})", parts.join(" ")),
                        other => {
                            parts.push(".".to_string());
                            parts.push(render_elem(&Rc::new(other.clone())));
                            return format!("({})", parts.join(" "));
                        }
                    }
                }
            }
        }
    }
}

fn render_elem(v: &Rc<LiteralValue>) -> String {
    v.render()
}

/// 9 个正交核心原语（核心冻结——签名在整个生命周期不变，§2.2 原则 9）。
///
/// **冻结边界精确化（lang-design 01 §6，v5.4）**：冻结的是语义原语集
/// （9 原语正交完备 + `Require` 声明变体——零运行时语义的元数据节点）；
/// 声明变体可追加。
///
/// **内部语法三原则合规（lang-design 01 §8.2，v6.0——sop §2.2 原则
/// 29-31）**：本 enum 是编译器私有 ADT（Rust 私有构造语义——用户代码
/// 无法构造，安全性由类型系统保证而非命名约定）；表面 S 表达式经
/// kerf-reader → kerf-expander 桥接到本类型（无旁路）；Span 独立携带
/// 元数据（`kind_name` 仅诊断渲染用）。
///
/// **E5 S3 M-R 名面臂（r44——22 §13 D34-D40）**：变体名与表面关键字
/// 同词根化已落地——`Fn`/`Assign`/`Do`/`Apply`/`Var` 五件 M-R 重命名
/// （零语义载荷，经 E5 窗 S3 腿通道）；`If→Branch`/`Literal→Const` 两件
/// NO-GO（一致性判据否决——22 §13 D35）；结构臂（Define 脱糖/Do→Let
/// 链/Module 迁移/de Bruijn IR）归 r45+ 承载（D39 分臂裁定）。
///
/// **Stage 2 ADT 演进目标（lang-design 01 §8.3 迁移映射——冻结维持：
/// K1 终门审 r35 + K2 深审 r36 复核维持冻结，迁移窗口随 Stage 3「目标
/// 语言完整化」重评）**：
/// M-R 名面臂 ✅ r44（Fn/Apply/Assign/Do/Var）；If/Literal 维持（D35
/// NO-GO 修正 01 §8.3 原候选 Branch/Const）；Define→脱糖消除 /
/// Do→`Let` 链 / Module→模块系统层 / de Bruijn = r45+ 结构臂（D39）；
/// 新增 `Let`+`Perform`/`Handle` 已在位（r25）。迁移须经 §13.2 切换期
/// 重构流程 + 委员会投票。
#[derive(Debug, Clone, PartialEq)]
pub enum CoreExpr {
    /// `(fn (params...) body)`——`param_scopes[i]` = 第 i 个形参的
    /// **绑定作用域集**（绑定器符号的作用域集，TD-004；与 `params`
    /// 平行等长，由展开器在绑定形式 fresh scope 注入后捕获）。
    Fn {
        params: Vec<Symbol>,
        param_scopes: Vec<ScopeSet>,
        body: Rc<CoreExpr>,
        span: Span,
    },
    /// `(fn-expr arg1 arg2 ...)`（调用——无表面关键字，位置形式）
    Apply {
        fn_expr: Rc<CoreExpr>,
        args: Vec<Rc<CoreExpr>>,
        span: Span,
    },
    /// `(if cond then else)`
    If {
        cond: Rc<CoreExpr>,
        then_branch: Rc<CoreExpr>,
        else_branch: Rc<CoreExpr>,
        span: Span,
    },
    /// `name`（自由或绑定引用）。
    ///
    /// `scopes` = 引用处作用域集（TD-004，r13）：编译器/eval 按
    /// `(name, scopes ⊆)` 匹配绑定（Racket 式集合作用域解析——空集
    /// 绑定 ⊆ 任意引用集，故全局/内置天然充当名称基兜底路径）。
    Var {
        name: Symbol,
        scopes: ScopeSet,
        span: Span,
    },
    /// 字面量（含 quote 产物的点对结构）
    Literal { value: LiteralValue, span: Span },
    /// `(assign name value)`——`scopes` = 赋值目标符号的引用处作用域集
    /// （与 Var 同一解析口径，TD-004）。
    Assign {
        name: Symbol,
        scopes: ScopeSet,
        value: Rc<CoreExpr>,
        span: Span,
    },
    /// `(define name value)`（模块顶层；函数体内部由展开器改写，§19.2 陷阱 3）
    Define {
        name: Symbol,
        value: Rc<CoreExpr>,
        span: Span,
    },
    /// `(do e1 e2 ...)`（返回最后一个表达式的值）
    Do { body: Vec<Rc<CoreExpr>>, span: Span },
    /// `(module name (import ...) (export ...) body...)`
    Module {
        name: Symbol,
        imports: Vec<Symbol>,
        exports: Vec<Symbol>,
        body: Vec<Rc<CoreExpr>>,
        span: Span,
    },
    /// `(require io read|write ...)`（r8 能力声明——13 §3.1.3 基础传递的
    /// 程序侧声明面；**零运行时语义**：不产字节码、不求值出 nil，仅供
    /// 编译期权限验证（R9/E0006）与 driver 令牌铸造消费）。
    Require { caps: Vec<Capability>, span: Span },
    /// `(perform ⟨effect-value⟩)`（r25/42-f——effect-language-design
    /// §2.1/R10：效应上抛。`effect` 先求值（求值顺序契约 Apply 序不变），
    /// 结果须为 `(tag . payload)` 点对（tag = Symbol 分派键；非此形态报
    /// 运行时类型错）。控制转移非值归约——resume 后值由恢复点注入，
    /// 未恢复则本表达式求值永不完成（body 剩余部分被 dispatch 丢弃）。
    Perform { effect: Rc<CoreExpr>, span: Span },
    /// `(handle ⟨tag⟩ ((⟨payload-var⟩ ⟨resume-var⟩) ⟨result-expr⟩) ⟨body⟩)`
    /// （R11——浅处理，D2：handler 处理一层效应；嵌套 handle = 用户侧
    /// 组合深处理）。`payload_var`/`resume_var` 为绑定器（绑定作用域集
    /// 平行字段与 Fn.param_scopes 同口径，TD-004）；`resume` 非独立
    /// 原语——continuation 值的调用形态（D4，展开期脱糖为 Apply）。
    Handle {
        /// 效应族标签（match 单键分派——D1：与 syntax-rules 字面量集合同型；
        /// 符号字面量同型载体 `Rc<str>`——与 LiteralValue::Symbol 一致，
        /// 编译侧无需符号表）。
        tag: Rc<str>,
        payload_var: Symbol,
        /// payload 绑定器的绑定作用域集（fresh scope 注入，展开器携带）。
        payload_scopes: ScopeSet,
        resume_var: Symbol,
        /// resume 绑定器的绑定作用域集。
        resume_scopes: ScopeSet,
        /// handler 体（单子句——D3 线性唯一性的语法承载）。
        handler_body: Rc<CoreExpr>,
        /// 被保护计算（效应触发时挂起于此）。体尾位 = 尾位穿线
        /// （body thunk 闭包体语义——TCO 穿透 handler 帧，D8）。
        body: Rc<CoreExpr>,
        span: Span,
    },
}

/// 能力令牌种类（`(require ...)` 声明项——Stage 1 仅 I/O 两类；
/// Stage 2 扩展 net/process 等（数据驱动扩展面，任意流程节点裁定
/// F2 修复））。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// stdin 读能力（read-line/read-int/read-num 门控）。
    IoRead,
    /// stdout 写能力（print/newline/write-string 门控）。
    IoWrite,
}

impl Capability {
    /// 声明名（require 形式与诊断渲染用）。
    pub fn as_str(self) -> &'static str {
        match self {
            Capability::IoRead => "read",
            Capability::IoWrite => "write",
        }
    }
}

impl CoreExpr {
    /// 节点 Span。
    pub fn span(&self) -> Span {
        match self {
            CoreExpr::Fn { span, .. }
            | CoreExpr::Apply { span, .. }
            | CoreExpr::If { span, .. }
            | CoreExpr::Var { span, .. }
            | CoreExpr::Literal { span, .. }
            | CoreExpr::Assign { span, .. }
            | CoreExpr::Define { span, .. }
            | CoreExpr::Do { span, .. }
            | CoreExpr::Module { span, .. }
            | CoreExpr::Require { span, .. }
            | CoreExpr::Perform { span, .. }
            | CoreExpr::Handle { span, .. } => *span,
        }
    }

    /// 原语名（诊断与 dump 用——22 §13 D37 五面同词根：与表面关键字/
    /// 桥 tag/渲染面同词根，12 名表 r44 落地）。
    pub fn kind_name(&self) -> &'static str {
        match self {
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
    }

    /// S 表达式形式渲染（需符号表解析函数；测试与 dump 用）。
    /// 渲染面/诊断面/tag 面五面同词根（r43 表面切换 + r44 内部名面
    /// 闭合——22 §13 D34：一个语义一个名）。
    pub fn render(&self, resolve: &dyn Fn(Symbol) -> String) -> String {
        match self {
            CoreExpr::Fn { params, body, .. } => {
                let ps: Vec<String> = params.iter().map(|p| resolve(*p)).collect();
                format!("(fn ({}) {})", ps.join(" "), body.render(resolve))
            }
            CoreExpr::Apply { fn_expr, args, .. } => {
                let mut parts = vec![fn_expr.render(resolve)];
                parts.extend(args.iter().map(|a| a.render(resolve)));
                format!("({})", parts.join(" "))
            }
            CoreExpr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => format!(
                "(if {} {} {})",
                cond.render(resolve),
                then_branch.render(resolve),
                else_branch.render(resolve)
            ),
            CoreExpr::Var { name, .. } => resolve(*name),
            CoreExpr::Literal { value, .. } => value.render(),
            CoreExpr::Assign { name, value, .. } => {
                format!("(assign {} {})", resolve(*name), value.render(resolve))
            }
            CoreExpr::Define { name, value, .. } => {
                format!("(define {} {})", resolve(*name), value.render(resolve))
            }
            CoreExpr::Do { body, .. } => {
                let parts: Vec<String> = body.iter().map(|e| e.render(resolve)).collect();
                format!("(do {})", parts.join(" "))
            }
            CoreExpr::Module {
                name,
                imports,
                exports,
                body,
                ..
            } => {
                let mut out = format!("(module {}", resolve(*name));
                if !imports.is_empty() {
                    let is: Vec<String> = imports.iter().map(|i| resolve(*i)).collect();
                    out.push_str(&format!(" (import {})", is.join(" ")));
                }
                if !exports.is_empty() {
                    let es: Vec<String> = exports.iter().map(|e| resolve(*e)).collect();
                    out.push_str(&format!(" (export {})", es.join(" ")));
                }
                for e in body {
                    out.push(' ');
                    out.push_str(&e.render(resolve));
                }
                out.push(')');
                out
            }
            CoreExpr::Require { caps, .. } => {
                let cs: Vec<&str> = caps.iter().map(|c| c.as_str()).collect();
                format!("(require io {})", cs.join(" "))
            }
            CoreExpr::Perform { effect, .. } => {
                format!("(perform {})", effect.render(resolve))
            }
            CoreExpr::Handle {
                tag,
                payload_var,
                resume_var,
                handler_body,
                body,
                ..
            } => {
                format!(
                    "(handle {} (({} {}) {}) {})",
                    tag,
                    resolve(*payload_var),
                    resolve(*resume_var),
                    handler_body.render(resolve),
                    body.render(resolve)
                )
            }
        }
    }

    /// 自由变量计算（多重主体按序遍历；绑定屏蔽）。
    /// CodeValue 与编译器闭包捕获分析共用此语义（唯一可信数据源）。
    /// 名称投影自 [`CoreExpr::free_var_occurrences`]（同一遍历的单一定义）。
    pub fn free_variables(&self, bound: &mut Vec<Symbol>, out: &mut Vec<Symbol>) {
        let mut occ: Vec<(Symbol, ScopeSet)> = Vec::new();
        self.free_var_occurrences(bound, &mut occ);
        out.extend(occ.iter().map(|(s, _)| *s));
    }

    /// 自由变量出现（含首次出现的作用域集——TD-004/r13：编译器捕获
    /// 分析需要对每个自由名判定 `(name, scopes ⊆)` 归属，故携带首次
    /// 出现的引用作用域集作为该名的代表；同名多次出现经展开器注入
    /// 不变式保证归同一绑定，Stage 0 规模下无歧义）。
    ///
    /// 遍历语义与名称版完全一致（绑定屏蔽、letrec* 序）；去重按名
    /// 首现优先。
    pub fn free_var_occurrences(&self, bound: &mut Vec<Symbol>, out: &mut Vec<(Symbol, ScopeSet)>) {
        match self {
            CoreExpr::Var { name, scopes, .. } => {
                if !bound.contains(name) && !out.iter().any(|(s, _)| s == name) {
                    out.push((*name, scopes.clone()));
                }
            }
            CoreExpr::Literal { .. } => {}
            CoreExpr::Fn { params, body, .. } => {
                let shadowed = params.len();
                for p in params {
                    if !bound.contains(p) {
                        bound.push(*p);
                    }
                }
                body.free_var_occurrences(bound, out);
                for _ in 0..shadowed {
                    bound.pop();
                }
            }
            CoreExpr::Apply { fn_expr, args, .. } => {
                fn_expr.free_var_occurrences(bound, out);
                for a in args {
                    a.free_var_occurrences(bound, out);
                }
            }
            CoreExpr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                cond.free_var_occurrences(bound, out);
                then_branch.free_var_occurrences(bound, out);
                else_branch.free_var_occurrences(bound, out);
            }
            CoreExpr::Assign {
                name,
                scopes,
                value,
                ..
            } => {
                if !bound.contains(name) && !out.iter().any(|(s, _)| s == name) {
                    out.push((*name, scopes.clone()));
                }
                value.free_var_occurrences(bound, out);
            }
            CoreExpr::Define { name, value, .. } => {
                // 定义引入新绑定：先求值再绑定（letrec* 序）
                value.free_var_occurrences(bound, out);
                if !bound.contains(name) {
                    bound.push(*name);
                }
            }
            CoreExpr::Do { body, .. } => {
                for e in body {
                    e.free_var_occurrences(bound, out);
                }
            }
            CoreExpr::Module { body, .. } => {
                for e in body {
                    e.free_var_occurrences(bound, out);
                }
            }
            // _ 臂理由：require 不含变量引用（零运行时语义——能力声明
            // 不进入作用域分析）
            CoreExpr::Require { .. } => {}
            CoreExpr::Perform { effect, .. } => {
                effect.free_var_occurrences(bound, out);
            }
            CoreExpr::Handle {
                payload_var,
                resume_var,
                handler_body,
                body,
                ..
            } => {
                // 绑定屏蔽（与 Fn 同型——两绑定器先入 bound，体遍历
                // 后弹出；tag 为符号常量非变量引用）
                let shadowed = 2;
                for p in [payload_var, resume_var] {
                    if !bound.contains(p) {
                        bound.push(*p);
                    }
                }
                handler_body.free_var_occurrences(bound, out);
                body.free_var_occurrences(bound, out);
                for _ in 0..shadowed {
                    bound.pop();
                }
            }
        }
    }

    /// 便捷封装：本表达式的自由变量出现（名 + 首现作用域集）。
    pub fn free_var_occurrence_list(&self) -> Vec<(Symbol, ScopeSet)> {
        let mut out = Vec::new();
        let mut bound = Vec::new();
        self.free_var_occurrences(&mut bound, &mut out);
        out
    }

    /// 便捷封装：本表达式的自由变量列表。
    pub fn free_variable_list(&self) -> Vec<Symbol> {
        let mut out = Vec::new();
        let mut bound = Vec::new();
        self.free_variables(&mut bound, &mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(n: u32) -> Symbol {
        Symbol(n)
    }

    fn var(n: u32) -> Rc<CoreExpr> {
        Rc::new(CoreExpr::Var {
            name: sym(n),
            scopes: ScopeSet::new(),
            span: Span::dummy(),
        })
    }

    #[test]
    fn free_variables_lambda_shadows() {
        // (fn (a) (f a b)) → 自由变量 {f, b}（a 被参数屏蔽）
        let body = Rc::new(CoreExpr::Apply {
            fn_expr: var(6),
            args: vec![var(0), var(1)],
            span: Span::dummy(),
        });
        let lam = CoreExpr::Fn {
            params: vec![sym(0)],
            param_scopes: vec![ScopeSet::new()],
            body,
            span: Span::dummy(),
        };
        let fv = lam.free_variable_list();
        assert_eq!(fv, vec![sym(6), sym(1)]);
    }

    #[test]
    fn free_variables_nested() {
        // (fn (x) (fn (y) (x y z))) → 自由变量 {z}
        let inner = CoreExpr::Fn {
            params: vec![sym(1)],
            param_scopes: vec![ScopeSet::new()],
            body: Rc::new(CoreExpr::Apply {
                fn_expr: var(0),
                args: vec![var(1), var(2)],
                span: Span::dummy(),
            }),
            span: Span::dummy(),
        };
        let outer = CoreExpr::Fn {
            params: vec![sym(0)],
            param_scopes: vec![ScopeSet::new()],
            body: Rc::new(inner),
            span: Span::dummy(),
        };
        assert_eq!(outer.free_variable_list(), vec![sym(2)]);
    }

    #[test]
    fn literal_render_pair_list() {
        // (1 2 3) as Pair chain
        let three = Rc::new(LiteralValue::Int(3));
        let two = Rc::new(LiteralValue::Int(2));
        let one = Rc::new(LiteralValue::Int(1));
        let list = LiteralValue::Pair(
            one.clone(),
            Rc::new(LiteralValue::Pair(
                two.clone(),
                Rc::new(LiteralValue::Pair(
                    three.clone(),
                    Rc::new(LiteralValue::Nil),
                )),
            )),
        );
        assert_eq!(list.render(), "(1 2 3)");
        // 点对：(1 . 2)
        let dotted = LiteralValue::Pair(one, two);
        assert_eq!(dotted.render(), "(1 . 2)");
    }

    #[test]
    fn render_core_forms() {
        let resolve = |s: Symbol| {
            ((b'a' + s.0 as u8) as char)
                .to_string()
                .chars()
                .take(1)
                .collect::<String>()
        };
        let if_expr = CoreExpr::If {
            cond: var(0),
            then_branch: var(1),
            else_branch: Rc::new(CoreExpr::Literal {
                value: LiteralValue::Nil,
                span: Span::dummy(),
            }),
            span: Span::dummy(),
        };
        assert_eq!(if_expr.render(&resolve), "(if a b nil)");
        assert_eq!(if_expr.kind_name(), "if");
    }
}
