//! 结构化 CodeValue：代码的一等值表示（stage0.md §8.3 / §6.3）。
//!
//! ```text
//! pub struct CodeValue {
//!     pub ir: GraphIR,             // 完整的图 IR
//!     pub span: Span,              // 源位置
//!     pub scope: ScopeSet,         // 作用域
//!     pub type_info: Option<TypeInfo>,
//!     pub free_vars: Vec<Symbol>,  // 自由变量列表
//!     pub stage: Stage,            // 阶段信息（为多阶段编程预留）
//! }
//! ```
//!
//! **职责边界**（§8.3）：仅作为代码的值表示，**不执行隐式编译**。
//! 不可变性（§2.2 原则 23）：变换（substitute / alpha_rename）产生新值。
//!
//! Stage 0 用途：`kerf code` 子命令的检查载体、多阶段编程预留接口
//! `MultiStage::Code` 的类型定义（kerf-driver::reserved）、以及
//! 图 IR 消费方（编译器/未来工具链）的代码打包形式。

use kerf_span::Span;
use kerf_syntax::{ScopeSet, Symbol};

use crate::expr::CoreExpr;
use crate::ir::{lower_program, IrGraph, NodeId};

/// 多阶段编程的阶段标记（§8.3 `stage: Stage`；Stage 2 多阶段引入前的预留枚举）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StageLevel {
    /// 当前阶段（now）——本阶段即执行。
    Now,
    /// 未来阶段（later）——代码值为下一阶段生成（`compose_with` 组合）。
    Later(u32),
}

/// 结构化代码值（§8.3 能力模型；trait 形态见设计文档，Stage 0 以具体类型交付，
/// 原因：单一实现下 inherent API 即满足可替换性——类型在 kerf-core 定义，
/// 消费方经 crate 公共 API 访问；Stage 2 出现第二表示时提取 trait）。
#[derive(Debug, Clone)]
pub struct CodeValue {
    /// 完整图 IR（单根：`root` 为根节点）。
    pub ir: IrGraph,
    /// 根节点。
    pub root: NodeId,
    /// 源位置。
    pub span: Span,
    /// 作用域集合。
    pub scopes: ScopeSet,
    /// 自由变量列表（构造时计算，随变换同步维护）。
    pub free_vars: Vec<Symbol>,
    /// 阶段信息（多阶段编程预留，§8.3）。
    pub stage: StageLevel,
}

impl CodeValue {
    /// 从单棵 `CoreExpr` 构造代码值。
    pub fn from_expr(expr: &CoreExpr) -> CodeValue {
        let ir = lower_program(&[std::rc::Rc::new(expr.clone())]);
        let root = ir.roots()[0];
        CodeValue {
            span: expr.span(),
            free_vars: expr.free_variable_list(),
            root,
            ir,
            scopes: ScopeSet::new(),
            stage: StageLevel::Now,
        }
    }

    /// 良构性检查：根节点存在、自由变量声明与图内**绑定感知**自由引用一致。
    pub fn is_well_formed(&self) -> bool {
        if (self.root as usize) >= self.ir.len() {
            return false;
        }
        // 图内自由引用（尊重 Lambda/Define 绑定屏蔽）必须与 free_vars 声明一致
        let refs = free_refs_of_graph(&self.ir, self.root);
        refs.len() == self.free_vars.len() && refs.iter().all(|r| self.free_vars.contains(r))
    }

    /// 自由变量列表（§8.3 接口契约）。
    pub fn free_variables(&self) -> Vec<Symbol> {
        self.free_vars.clone()
    }

    /// 不可变换换：`var` 的引用替换为 `other` 的根（返回新 CodeValue）。
    pub fn substitute(&self, var: Symbol, other: &CodeValue) -> CodeValue {
        let ir = self.ir.substitute(var, other.root);
        let free_vars = recompute_free(&ir, self.root);
        CodeValue {
            ir,
            root: self.root,
            span: self.span,
            scopes: self.scopes.clone(),
            free_vars,
            stage: self.stage,
        }
    }

    /// 不可变 α 重命名：`from` → `to`（返回新 CodeValue）。
    pub fn alpha_rename(&self, from: Symbol, to: Symbol) -> CodeValue {
        let ir = self.ir.map_nodes(|_id, node| match node {
            crate::ir::IrNode::VarRef(name) if *name == from => crate::ir::IrNode::VarRef(to),
            other => other.clone(),
        });
        let free_vars = self
            .free_vars
            .iter()
            .map(|v| if *v == from { to } else { *v })
            .collect();
        CodeValue {
            ir,
            root: self.root,
            span: self.span,
            scopes: self.scopes.clone(),
            free_vars,
            stage: self.stage,
        }
    }

    /// 多阶段组合（**接口预留**，§8.3：Stage 2 实现；Stage 0 显式返回不支持错误，
    /// 不静默，§2.3 原则 4）。
    pub fn compose_with(&self, _other: &CodeValue) -> Result<CodeValue, CompositionError> {
        Err(CompositionError::UnsupportedInStage0)
    }
}

/// 组合错误（§6.3 接口契约）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositionError {
    /// Stage 0 未实现多阶段组合（接口预留冻结，Stage 2 落地）。
    UnsupportedInStage0,
}

fn collect_var_refs(ir: &IrGraph, root: NodeId, out: &mut Vec<Symbol>) {
    use crate::ir::IrNode;
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        match ir.get_node(id) {
            IrNode::VarRef(s) => {
                if !out.contains(s) {
                    out.push(*s);
                }
            }
            IrNode::Literal(_) => {}
            IrNode::Lambda { body, .. } => stack.push(*body),
            IrNode::App { callee, args } => {
                stack.push(*callee);
                stack.extend(args.iter().copied());
            }
            IrNode::If {
                cond,
                then_branch,
                else_branch,
            } => {
                stack.push(*cond);
                stack.push(*then_branch);
                stack.push(*else_branch);
            }
            IrNode::SetBang { value, .. } | IrNode::Define { value, .. } => stack.push(*value),
            IrNode::Begin { body } | IrNode::Module { body, .. } => {
                stack.extend(body.iter().copied())
            }
            IrNode::Perform { effect } => stack.push(*effect),
            IrNode::Handle {
                handler_body, body, ..
            } => {
                stack.push(*handler_body);
                stack.push(*body);
            }
        }
    }
}

fn recompute_free(ir: &IrGraph, root: NodeId) -> Vec<Symbol> {
    let mut refs = Vec::new();
    collect_var_refs(ir, root, &mut refs);
    refs
}

/// 绑定感知的自由引用计算（Lambda 参数屏蔽；Define 引入绑定）。
fn free_refs_of_graph(ir: &IrGraph, root: NodeId) -> Vec<Symbol> {
    use crate::ir::IrNode;
    let mut out: Vec<Symbol> = Vec::new();
    let mut bound: Vec<Symbol> = Vec::new();
    fn walk(ir: &IrGraph, id: NodeId, bound: &mut Vec<Symbol>, out: &mut Vec<Symbol>) {
        match ir.get_node(id) {
            IrNode::VarRef(s) => {
                if !bound.contains(s) && !out.contains(s) {
                    out.push(*s);
                }
            }
            IrNode::Literal(_) => {}
            IrNode::Lambda { params, body } => {
                let pushed = params.iter().filter(|p| !bound.contains(p)).count();
                bound.extend(params.iter().copied());
                walk(ir, *body, bound, out);
                for _ in 0..pushed {
                    bound.pop();
                }
            }
            IrNode::App { callee, args } => {
                walk(ir, *callee, bound, out);
                for a in args {
                    walk(ir, *a, bound, out);
                }
            }
            IrNode::If {
                cond,
                then_branch,
                else_branch,
            } => {
                walk(ir, *cond, bound, out);
                walk(ir, *then_branch, bound, out);
                walk(ir, *else_branch, bound, out);
            }
            IrNode::SetBang { name, value } | IrNode::Define { name, value } => {
                walk(ir, *value, bound, out);
                if !bound.contains(name) {
                    bound.push(*name);
                }
            }
            IrNode::Begin { body } | IrNode::Module { body, .. } => {
                for b in body {
                    walk(ir, *b, bound, out);
                }
            }
            IrNode::Perform { effect } => walk(ir, *effect, bound, out),
            IrNode::Handle {
                payload_var,
                resume_var,
                handler_body,
                body,
                ..
            } => {
                // 两绑定器屏蔽（与 Lambda 同型）
                let pushed = [payload_var, resume_var]
                    .iter()
                    .filter(|p| !bound.contains(p))
                    .count();
                bound.push(*payload_var);
                bound.push(*resume_var);
                walk(ir, *handler_body, bound, out);
                walk(ir, *body, bound, out);
                for _ in 0..pushed {
                    bound.pop();
                }
            }
        }
    }
    walk(ir, root, &mut bound, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::LiteralValue;
    use std::rc::Rc;

    fn var(n: u32) -> CoreExpr {
        CoreExpr::VarRef {
            name: Symbol(n),
            scopes: ScopeSet::new(),
            span: Span::dummy(),
        }
    }

    fn lam(param: u32, body: CoreExpr) -> CoreExpr {
        CoreExpr::Lambda {
            params: vec![Symbol(param)],
            param_scopes: vec![ScopeSet::new()],
            body: Rc::new(body),
            span: Span::dummy(),
        }
    }

    fn app(f: CoreExpr, arg: CoreExpr) -> CoreExpr {
        CoreExpr::App {
            fn_expr: Rc::new(f),
            args: vec![Rc::new(arg)],
            span: Span::dummy(),
        }
    }

    #[test]
    fn well_formed_and_free_vars() {
        // (lambda (x) (x y)) → 自由 {y}
        let e = lam(0, app(var(0), var(1)));
        let cv = CodeValue::from_expr(&e);
        assert!(cv.is_well_formed());
        assert_eq!(cv.free_variables(), vec![Symbol(1)]);
    }

    #[test]
    fn alpha_rename_rewrites() {
        let e = lam(0, app(var(0), var(1)));
        let cv = CodeValue::from_expr(&e);
        let renamed = cv.alpha_rename(Symbol(1), Symbol(9));
        assert_eq!(renamed.free_variables(), vec![Symbol(9)]);
        assert!(renamed.is_well_formed());
        // 不可变：原值不变（§2.2 原则 23）
        assert_eq!(cv.free_variables(), vec![Symbol(1)]);
    }

    #[test]
    fn compose_with_reserved_unsupported() {
        let e = CoreExpr::Literal {
            value: LiteralValue::Int(1),
            span: Span::dummy(),
        };
        let a = CodeValue::from_expr(&e);
        assert!(matches!(
            a.compose_with(&a),
            Err(CompositionError::UnsupportedInStage0)
        ));
    }
}
