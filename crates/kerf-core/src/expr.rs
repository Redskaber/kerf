//! `CoreExpr`：9 个正交核心原语（stage0.md §3.2 最终定义）。
//!
//! ```ocaml
//! type core_expr =
//!   | Lambda of { params : string list; body : core_expr }
//!   | App of { fn : core_expr; args : core_expr list }
//!   | If of { cond; then_branch; else_branch }
//!   | VarRef of string
//!   | Literal of literal_value
//!   | SetBang of { name : string; value : core_expr }
//!   | Define of { name : string; value : core_expr }
//!   | Begin of core_expr list
//!   | Module of { name; imports; exports; body }
//!   | Require of { caps : capability list }   (* r8：能力声明，零运行时语义 *)
//! ```
//!
//! 每个 AST 节点携带 Span（§8.6：Span 全管线传播，编译期段）。

use std::rc::Rc;

use kerf_span::Span;
use kerf_syntax::Symbol;

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
/// **Stage 2 ADT 演进目标（lang-design 01 §8.3 迁移映射——冻结期不
/// 实施，登记为 Stage 2「目标语言完整化」门审查评估清单）**：
/// Lambda→`Fn`（de Bruijn）/ App→`Apply` / If→`Branch` / VarRef→`Var` /
/// Literal→`Const` / SetBang→`Perform(State)` / Define→脱糖消除 /
/// Begin→`Let` 链 / Module→模块系统层 / 新增 `Let`+`Perform`/`Handle` /
/// Require→保留（声明面）。迁移须经 §13.2 切换期重构流程 + 委员会投票。
#[derive(Debug, Clone, PartialEq)]
pub enum CoreExpr {
    /// `(lambda (params...) body)`
    Lambda {
        params: Vec<Symbol>,
        body: Rc<CoreExpr>,
        span: Span,
    },
    /// `(fn-expr arg1 arg2 ...)`
    App {
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
    /// `name`（自由或绑定引用）
    VarRef { name: Symbol, span: Span },
    /// 字面量（含 quote 产物的点对结构）
    Literal { value: LiteralValue, span: Span },
    /// `(set! name value)`
    SetBang {
        name: Symbol,
        value: Rc<CoreExpr>,
        span: Span,
    },
    /// `(define name value)`（模块顶层；函数体内部由展开器改写，§19.2 陷阱 3）
    Define {
        name: Symbol,
        value: Rc<CoreExpr>,
        span: Span,
    },
    /// `(begin e1 e2 ...)`（返回最后一个表达式的值）
    Begin { body: Vec<Rc<CoreExpr>>, span: Span },
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
            CoreExpr::Lambda { span, .. }
            | CoreExpr::App { span, .. }
            | CoreExpr::If { span, .. }
            | CoreExpr::VarRef { span, .. }
            | CoreExpr::Literal { span, .. }
            | CoreExpr::SetBang { span, .. }
            | CoreExpr::Define { span, .. }
            | CoreExpr::Begin { span, .. }
            | CoreExpr::Module { span, .. }
            | CoreExpr::Require { span, .. } => *span,
        }
    }

    /// 原语名（诊断与 dump 用）。
    pub fn kind_name(&self) -> &'static str {
        match self {
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
    }

    /// S 表达式形式渲染（需符号表解析函数；测试与 dump 用）。
    pub fn render(&self, resolve: &dyn Fn(Symbol) -> String) -> String {
        match self {
            CoreExpr::Lambda { params, body, .. } => {
                let ps: Vec<String> = params.iter().map(|p| resolve(*p)).collect();
                format!("(lambda ({}) {})", ps.join(" "), body.render(resolve))
            }
            CoreExpr::App { fn_expr, args, .. } => {
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
            CoreExpr::VarRef { name, .. } => resolve(*name),
            CoreExpr::Literal { value, .. } => value.render(),
            CoreExpr::SetBang { name, value, .. } => {
                format!("(set! {} {})", resolve(*name), value.render(resolve))
            }
            CoreExpr::Define { name, value, .. } => {
                format!("(define {} {})", resolve(*name), value.render(resolve))
            }
            CoreExpr::Begin { body, .. } => {
                let parts: Vec<String> = body.iter().map(|e| e.render(resolve)).collect();
                format!("(begin {})", parts.join(" "))
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
        }
    }

    /// 自由变量计算（多重主体按序遍历；绑定屏蔽）。
    /// CodeValue 与编译器闭包捕获分析共用此语义（唯一可信数据源）。
    pub fn free_variables(&self, bound: &mut Vec<Symbol>, out: &mut Vec<Symbol>) {
        match self {
            CoreExpr::VarRef { name, .. } => {
                if !bound.contains(name) && !out.contains(name) {
                    out.push(*name);
                }
            }
            CoreExpr::Literal { .. } => {}
            CoreExpr::Lambda { params, body, .. } => {
                let shadowed = params.len();
                for p in params {
                    if !bound.contains(p) {
                        bound.push(*p);
                    }
                }
                body.free_variables(bound, out);
                for _ in 0..shadowed {
                    bound.pop();
                }
            }
            CoreExpr::App { fn_expr, args, .. } => {
                fn_expr.free_variables(bound, out);
                for a in args {
                    a.free_variables(bound, out);
                }
            }
            CoreExpr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                cond.free_variables(bound, out);
                then_branch.free_variables(bound, out);
                else_branch.free_variables(bound, out);
            }
            CoreExpr::SetBang { name, value, .. } => {
                if !bound.contains(name) && !out.contains(name) {
                    out.push(*name);
                }
                value.free_variables(bound, out);
            }
            CoreExpr::Define { name, value, .. } => {
                // 定义引入新绑定：先求值再绑定（letrec* 序）
                value.free_variables(bound, out);
                if !bound.contains(name) {
                    bound.push(*name);
                }
            }
            CoreExpr::Begin { body, .. } => {
                for e in body {
                    e.free_variables(bound, out);
                }
            }
            CoreExpr::Module { body, .. } => {
                for e in body {
                    e.free_variables(bound, out);
                }
            }
            // _ 臂理由：require 不含变量引用（零运行时语义——能力声明
            // 不进入作用域分析）
            CoreExpr::Require { .. } => {}
        }
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
        Rc::new(CoreExpr::VarRef {
            name: sym(n),
            span: Span::dummy(),
        })
    }

    #[test]
    fn free_variables_lambda_shadows() {
        // (lambda (a) (f a b)) → 自由变量 {f, b}（a 被参数屏蔽）
        let body = Rc::new(CoreExpr::App {
            fn_expr: var(6),
            args: vec![var(0), var(1)],
            span: Span::dummy(),
        });
        let lam = CoreExpr::Lambda {
            params: vec![sym(0)],
            body,
            span: Span::dummy(),
        };
        let fv = lam.free_variable_list();
        assert_eq!(fv, vec![sym(6), sym(1)]);
    }

    #[test]
    fn free_variables_nested() {
        // (lambda (x) (lambda (y) (x y z))) → 自由变量 {z}
        let inner = CoreExpr::Lambda {
            params: vec![sym(1)],
            body: Rc::new(CoreExpr::App {
                fn_expr: var(0),
                args: vec![var(1), var(2)],
                span: Span::dummy(),
            }),
            span: Span::dummy(),
        };
        let outer = CoreExpr::Lambda {
            params: vec![sym(0)],
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
