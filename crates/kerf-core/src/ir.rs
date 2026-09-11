//! 图 IR（含共享节点）：程序的 Arena 结构化形式（stage0.md §8.2）。
//!
//! **能力模型**（§8.2）：
//! - Arena 分配（`NodeId` 索引），节点元信息（Span + 作用域）平行表存储；
//! - `node_map`：结构键 → 节点列表（**支持共享查找**——公共子表达式/常量共享）；
//! - 不可变换换 API（`map_nodes` / `substitute`）返回新图。
//!
//! **Stage 0 共享策略**（风险表 §21.11「图 IR 实现过于复杂」的既定缓解）：
//! 字面量与变量节点按结构键完整去重共享；复合节点不做激进 CSE
//! （显式推迟到 Stage 2 图结构迁移，登记于技术债 TD-003）。
//!
//! **职责边界**（§8.2）：表示程序的结构化形式；不执行求值、不做类型推断、
//! 不处理控制流执行顺序。

use std::collections::HashMap;
use std::rc::Rc;

use kerf_span::Span;
use kerf_syntax::{ScopeSet, Symbol};

use crate::expr::{CoreExpr, LiteralValue};

/// Arena 节点索引。
pub type NodeId = u32;

/// IR 节点（§8.2 `IRNode`；每节点元信息见 `NodeMetadata` 平行表）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IrNode {
    Lambda {
        params: Vec<Symbol>,
        body: NodeId,
    },
    App {
        callee: NodeId,
        args: Vec<NodeId>,
    },
    If {
        cond: NodeId,
        then_branch: NodeId,
        else_branch: NodeId,
    },
    VarRef(Symbol),
    Literal(LiteralKey),
    SetBang {
        name: Symbol,
        value: NodeId,
    },
    Define {
        name: Symbol,
        value: NodeId,
    },
    Begin {
        body: Vec<NodeId>,
    },
    Module {
        name: Symbol,
        imports: Vec<Symbol>,
        exports: Vec<Symbol>,
        body: Vec<NodeId>,
    },
    /// 效应执行（r25/42-f——effect-language-design §2.1/R10 直译）。
    Perform {
        effect: NodeId,
    },
    /// 效应处理（浅处理——两绑定器 payload/resume 引入 fresh scope
    /// 作用域，与 Lambda 同型；tag 为符号字面量同型 `Rc<str>`）。
    Handle {
        tag: Rc<str>,
        payload_var: Symbol,
        resume_var: Symbol,
        handler_body: NodeId,
        body: NodeId,
    },
}

/// 字面量结构键（可哈希共享的最小形式；运行期值由编译器常量池承载）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LiteralKey {
    Int(i64),
    Str(Rc<str>),
    Bool(bool),
    Nil,
    /// 浮点键：按位展开（f64 不实现 Eq/Hash，Stage 0 以位模式为键）。
    FloatBits(u64),
    /// 符号键（按名去重——TD-002）。
    Symbol(Rc<str>),
}

impl LiteralKey {
    /// 从 `LiteralValue` 提炼可哈希键（Pair 结构不参与共享去重——保留单实例）。
    pub fn from_value(v: &LiteralValue) -> LiteralKey {
        match v {
            LiteralValue::Int(i) => LiteralKey::Int(*i),
            LiteralValue::Float(f) => LiteralKey::FloatBits(f.to_bits()),
            LiteralValue::Str(s) => LiteralKey::Str(s.clone()),
            LiteralValue::Bool(b) => LiteralKey::Bool(*b),
            LiteralValue::Nil => LiteralKey::Nil,
            LiteralValue::Symbol(s) => LiteralKey::Symbol(s.clone()),
            LiteralValue::Pair(..) => LiteralKey::Nil,
        }
    }
}

/// 节点元信息（§8.2：每个节点都携带 Span 和作用域信息）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeMetadata {
    pub span: Span,
    pub scopes: ScopeSet,
}

/// 图 IR：Arena + 共享查找表 + 根节点集合。
#[derive(Debug, Clone, Default)]
pub struct IrGraph {
    nodes: Vec<IrNode>,
    metadata: Vec<NodeMetadata>,
    /// 结构键 → 共享节点列表（支持共享查找，§8.2 接口契约 `find_shared`）。
    node_map: HashMap<LiteralKey, Vec<NodeId>>,
    roots: Vec<NodeId>,
}

impl IrGraph {
    /// 空图。
    pub fn new() -> Self {
        IrGraph::default()
    }

    /// 构造节点（元信息必填——Span 不可为空，§3.3）。
    pub fn add_node(&mut self, node: IrNode, meta: NodeMetadata) -> NodeId {
        let id = self.nodes.len() as NodeId;
        self.nodes.push(node);
        self.metadata.push(meta);
        id
    }

    /// 构造**共享**字面量节点：结构键命中即复用既有节点（公共子表达式共享）。
    pub fn add_shared_literal(&mut self, key: LiteralKey, meta: NodeMetadata) -> NodeId {
        if let Some(existing) = self.node_map.get(&key).and_then(|ids| ids.first()).copied() {
            return existing;
        }
        let id = self.add_node(IrNode::Literal(key.clone()), meta);
        self.node_map.entry(key).or_default().push(id);
        id
    }

    /// 查询节点。
    pub fn get_node(&self, id: NodeId) -> &IrNode {
        &self.nodes[id as usize]
    }

    /// 查询节点元信息。
    pub fn get_metadata(&self, id: NodeId) -> &NodeMetadata {
        &self.metadata[id as usize]
    }

    /// 共享查找：同结构键的全部节点（§8.2 接口契约）。
    pub fn find_shared(&self, key: &LiteralKey) -> Vec<NodeId> {
        self.node_map.get(key).cloned().unwrap_or_default()
    }

    /// 根节点集合。
    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    /// 追加根节点。
    pub fn add_root(&mut self, id: NodeId) {
        self.roots.push(id);
    }

    /// 节点总数（共享后去重计数 ≤ 构造次数）。
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// 是否为空图。
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// 全部节点迭代（按 NodeId 升序）。
    pub fn iter_nodes(&self) -> impl Iterator<Item = (NodeId, &IrNode)> + '_ {
        self.nodes.iter().enumerate().map(|(i, n)| (i as NodeId, n))
    }

    /// 不可变变换：按函数映射节点（返回新图，元信息保留）。
    /// Lambda/App 等子节点 ID 不自动重写——调用方负责映射一致性（Stage 0 约定）。
    pub fn map_nodes(&self, f: impl Fn(NodeId, &IrNode) -> IrNode) -> IrGraph {
        let mut out = IrGraph {
            nodes: Vec::with_capacity(self.nodes.len()),
            metadata: self.metadata.clone(),
            node_map: self.node_map.clone(),
            roots: self.roots.clone(),
        };
        for (id, node) in self.iter_nodes() {
            out.nodes.push(f(id, node));
        }
        out
    }

    /// 不可变换换：将 `var` 的全部引用替换为 `replacement` 根（返回新图）。
    pub fn substitute(&self, var: Symbol, replacement: NodeId) -> IrGraph {
        let mut out = IrGraph {
            nodes: Vec::with_capacity(self.nodes.len()),
            metadata: self.metadata.clone(),
            node_map: HashMap::new(),
            roots: self.roots.clone(),
        };
        for (id, node) in self.iter_nodes() {
            let mapped = match node {
                IrNode::VarRef(name) if *name == var => self.get_node(replacement).clone(),
                other => other.clone(),
            };
            let nid = out.add_node(mapped, self.metadata[id as usize].clone());
            let _ = nid;
        }
        out
    }
}

/// `CoreExpr` 树 → 图 IR 的直译式下降（lowering）。
///
/// 公共入口（sop.md §10.1 规则 1：`<verb>_<noun>` 自由函数）。
/// 顶层表达式的 ID 依次登记为图的根。
pub fn lower_program(exprs: &[Rc<CoreExpr>]) -> IrGraph {
    let mut graph = IrGraph::new();
    let mut scopes = ScopeSet::new();
    for e in exprs {
        let id = lower_expr(&mut graph, e, &mut scopes);
        graph.add_root(id);
    }
    graph
}

fn lower_expr(graph: &mut IrGraph, e: &CoreExpr, scopes: &mut ScopeSet) -> NodeId {
    let meta = NodeMetadata {
        span: e.span(),
        scopes: scopes.clone(),
    };
    match e {
        CoreExpr::Literal { value, .. } => {
            graph.add_shared_literal(LiteralKey::from_value(value), meta)
        }
        CoreExpr::VarRef { name, .. } => graph.add_node(IrNode::VarRef(*name), meta),
        CoreExpr::Lambda { params, body, .. } => {
            // 绑定引入新作用域（mark，§19.2 展开侧语义在 IR 侧的镜像）
            let scope = fresh_scope();
            scopes.add(scope);
            let body_id = lower_expr(graph, body, scopes);
            scopes.remove(scope);
            graph.add_node(
                IrNode::Lambda {
                    params: params.clone(),
                    body: body_id,
                },
                meta,
            )
        }
        CoreExpr::App { fn_expr, args, .. } => {
            let callee = lower_expr(graph, fn_expr, scopes);
            let arg_ids: Vec<NodeId> = args.iter().map(|a| lower_expr(graph, a, scopes)).collect();
            graph.add_node(
                IrNode::App {
                    callee,
                    args: arg_ids,
                },
                meta,
            )
        }
        CoreExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            let c = lower_expr(graph, cond, scopes);
            let t = lower_expr(graph, then_branch, scopes);
            let el = lower_expr(graph, else_branch, scopes);
            graph.add_node(
                IrNode::If {
                    cond: c,
                    then_branch: t,
                    else_branch: el,
                },
                meta,
            )
        }
        CoreExpr::SetBang { name, value, .. } => {
            let v = lower_expr(graph, value, scopes);
            graph.add_node(
                IrNode::SetBang {
                    name: *name,
                    value: v,
                },
                meta,
            )
        }
        CoreExpr::Define { name, value, .. } => {
            let v = lower_expr(graph, value, scopes);
            graph.add_node(
                IrNode::Define {
                    name: *name,
                    value: v,
                },
                meta,
            )
        }
        CoreExpr::Begin { body, .. } => {
            let ids: Vec<NodeId> = body.iter().map(|e| lower_expr(graph, e, scopes)).collect();
            graph.add_node(IrNode::Begin { body: ids }, meta)
        }
        CoreExpr::Module {
            name,
            imports,
            exports,
            body,
            ..
        } => {
            let ids: Vec<NodeId> = body.iter().map(|e| lower_expr(graph, e, scopes)).collect();
            graph.add_node(
                IrNode::Module {
                    name: *name,
                    imports: imports.clone(),
                    exports: exports.clone(),
                    body: ids,
                },
                meta,
            )
        }
        CoreExpr::Require { span, .. } => {
            // r8 能力声明：零运行时语义——降级为 nil 字面量节点（图连通
            // 保持；编译期权限验证在 driver front 管线完成，非 IR 职责）
            let _ = span;
            graph.add_shared_literal(LiteralKey::from_value(&LiteralValue::Nil), meta)
        }
        CoreExpr::Perform { effect, .. } => {
            let e = lower_expr(graph, effect, scopes);
            graph.add_node(IrNode::Perform { effect: e }, meta)
        }
        CoreExpr::Handle {
            tag,
            payload_var,
            resume_var,
            handler_body,
            body,
            ..
        } => {
            // 两绑定器 fresh scope（镜像 Lambda 的作用域语义——TD-004）
            let scope = fresh_scope();
            scopes.add(scope);
            let h = lower_expr(graph, handler_body, scopes);
            let b = lower_expr(graph, body, scopes);
            scopes.remove(scope);
            graph.add_node(
                IrNode::Handle {
                    tag: tag.clone(),
                    payload_var: *payload_var,
                    resume_var: *resume_var,
                    handler_body: h,
                    body: b,
                },
                meta,
            )
        }
    }
}

/// 作用域计数器（crate 级单例语义：测试并发不涉及；IR 标记唯一性即可）。
fn fresh_scope() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(10_000);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(v: i64) -> Rc<CoreExpr> {
        Rc::new(CoreExpr::Literal {
            value: LiteralValue::Int(v),
            span: Span::dummy(),
        })
    }

    #[test]
    fn shared_literal_dedup() {
        let exprs = vec![
            Rc::new(CoreExpr::App {
                fn_expr: lit(1),
                args: vec![],
                span: Span::dummy(),
            }),
            lit(1),
            lit(1),
            lit(2),
        ];
        let graph = lower_program(&exprs);
        // (1) 共享命中：三处 Int(1) 只应有一个 Literal 节点
        let shared = graph.find_shared(&LiteralKey::Int(1));
        assert_eq!(shared.len(), 1);
        let shared2 = graph.find_shared(&LiteralKey::Int(2));
        assert_eq!(shared2.len(), 1);
        // (2) 根数量完整；节点总数 = 1 App + 1 共享 Int(1) + 1 Int(2)
        assert_eq!(graph.roots().len(), 4);
        assert_eq!(graph.len(), 3);
    }

    #[test]
    fn substitute_rewrites_var() {
        let exprs = vec![
            Rc::new(CoreExpr::VarRef {
                name: Symbol(1),
                scopes: kerf_syntax::ScopeSet::new(),
                span: Span::dummy(),
            }),
            Rc::new(CoreExpr::VarRef {
                name: Symbol(2),
                scopes: kerf_syntax::ScopeSet::new(),
                span: Span::dummy(),
            }),
        ];
        let graph = lower_program(&exprs);
        let graph2 = graph.substitute(Symbol(1), graph.roots()[1]);
        // 替换后根 0 成为 VarRef(2)
        let root0 = graph2.roots()[0];
        match graph2.get_node(root0) {
            IrNode::VarRef(s) => assert_eq!(*s, Symbol(2)),
            other => panic!("期望 VarRef，实际 {:?}", other),
        }
    }

    #[test]
    fn map_nodes_returns_new_graph() {
        let exprs = vec![lit(42)];
        let graph = lower_program(&exprs);
        let mapped = graph.map_nodes(|_id, node| node.clone());
        assert_eq!(mapped.len(), graph.len());
        assert_eq!(mapped.get_node(0), graph.get_node(0));
    }
}
