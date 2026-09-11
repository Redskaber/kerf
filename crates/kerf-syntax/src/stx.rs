//! 语法对象 `Stx`（SyntaxObject）：Reader 的输出、Expander 的输入（stage0.md §3.3）。
//!
//! 结构：
//! ```text
//! type stx_obj = {
//!   expr   : stx_expr;        // 本文件中的 StxDatum
//!   span   : span;            // 源码位置——必须字段，不可为空（§3.3）
//!   scopes : scope_set;       // 作用域集合
//!   phase  : int;             // 所属相位（§3.3）
//! }
//! ```

use std::rc::Rc;

use kerf_span::Span;

use crate::scope::ScopeSet;
use crate::symbol::Symbol;
use crate::Phase;

/// 语法字面量（Reader 产出、宏模式可匹配的最小数据单元）。
#[derive(Debug, Clone, PartialEq)]
pub enum StxLiteral {
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    Bool(bool),
    Nil,
}

impl StxLiteral {
    /// 数据化渲染（datum 打印）。
    pub fn render(&self) -> String {
        match self {
            StxLiteral::Int(i) => i.to_string(),
            StxLiteral::Float(f) => {
                if f.fract() == 0.0 && f.is_finite() {
                    format!("{:.1}", f)
                } else {
                    f.to_string()
                }
            }
            StxLiteral::Str(s) => format!("{:?}", s),
            StxLiteral::Bool(b) => b.to_string(),
            StxLiteral::Nil => "nil".to_string(),
        }
    }
}

/// 语法数据（datum）：符号 / 字面量 / 列表 / 向量。
///
/// **Rc 共享化（TD-007 / 批次 H2）**：列表/向量子项以 `Rc<Vec<Stx>>`
/// 承载——`Clone` 变为 O(1) 浅拷贝（子树共享），旧实现值语义深树的
/// clone/drop 递归约束解除（实测 8MiB 主线程 4_000 通过/5_000 溢出 →
/// Rc 化后展开链 10_000 恒定栈深）。结构相等（`PartialEq`）语义不变
/// （`Rc<Vec>` 按内容比较——与旧行为逐元素等价）。
#[derive(Debug, Clone, PartialEq)]
pub enum StxDatum {
    /// 标识符出现。
    Symbol(Symbol),
    /// 字面量出现。
    Literal(StxLiteral),
    /// 圆括号列表（核心形式的载体）。Rc 共享（见类型注记）。
    List(Rc<Vec<Stx>>),
    /// 方括号向量（语法糖数据，Stage 0 主要用于 let 绑定组与宏字面量组）。
    /// Rc 共享（同上）。
    Vector(Rc<Vec<Stx>>),
}

impl StxDatum {
    /// 是否为符号数据。
    pub fn as_symbol(&self) -> Option<Symbol> {
        match self {
            StxDatum::Symbol(s) => Some(*s),
            _ => None,
        }
    }

    /// 是否为列表数据。
    pub fn as_list(&self) -> Option<&[Stx]> {
        match self {
            StxDatum::List(items) => Some(items),
            _ => None,
        }
    }

    /// 列表首符号（核心形式头判定）。
    pub fn list_head_symbol(&self) -> Option<Symbol> {
        self.as_list()
            .and_then(|items| items.first())
            .and_then(|stx| stx.datum.as_symbol())
    }
}

/// 语法对象：datum + Span + ScopeSet + Phase（§3.3 全字段）。
///
/// **结构相等**：手写 `PartialEq`——只比较 datum/span/scopes/phase
/// 四个语义字段，`uniform_tag`（内部性能标记，见字段注记）不参与。
#[derive(Debug, Clone)]
pub struct Stx {
    pub datum: StxDatum,
    /// 源码位置——必须字段（§3.3：不可为空；Reader 保证）。
    pub span: Span,
    /// 作用域集合（卫生宏基础）。
    pub scopes: ScopeSet,
    /// 所属相位（Reader 产出时为 Runtime；宏产物为 ExpandTime，§8.9）。
    pub phase: Phase,
    /// **均匀作用域标记（内部性能字段，TD-007/H2）**：`Some(T)` 当且仅当
    /// 本节点与全部子孙的 `scopes` 均为 `T`（由 `retag_scopes` 重建时设置
    /// ——供其 O(1) 共享快路径）。外部构造恒置 `None`；`add_scope_to_all`
    /// 注入时清除（均匀性破坏）。不参与结构相等与任何语义判定。
    pub uniform_tag: Option<ScopeSet>,
}

impl PartialEq for Stx {
    fn eq(&self, other: &Self) -> bool {
        self.datum == other.datum
            && self.span == other.span
            && self.scopes == other.scopes
            && self.phase == other.phase
    }
}
impl Eq for Stx {}

impl Stx {
    /// 构造符号语法对象。
    pub fn symbol(sym: Symbol, span: Span, scopes: ScopeSet) -> Self {
        Stx {
            datum: StxDatum::Symbol(sym),
            span,
            scopes,
            phase: Phase::Runtime,
            uniform_tag: None,
        }
    }

    /// 构造列表语法对象（Span 由调用方合并提供）。子项 Rc 共享承载。
    pub fn list(items: Vec<Stx>, span: Span, scopes: ScopeSet) -> Self {
        Stx {
            datum: StxDatum::List(Rc::new(items)),
            span,
            scopes,
            phase: Phase::Runtime,
            uniform_tag: None,
        }
    }

    /// 构造字面量语法对象。
    pub fn literal(value: StxLiteral, span: Span, scopes: ScopeSet) -> Self {
        Stx {
            datum: StxDatum::Literal(value),
            span,
            scopes,
            phase: Phase::Runtime,
            uniform_tag: None,
        }
    }

    /// 提升为宏产物：相位 → ExpandTime + 展开代次 + 1 + 作用域并入宏定义处作用域
    /// （§19.2 卫生性关键：展开产物自动携带「宏定义处作用域 ∪ 使用处作用域」的并集）。
    pub fn as_macro_expansion(&self, macro_def_scopes: &ScopeSet) -> Stx {
        Stx {
            datum: self.datum.clone(),
            span: self.span.bumped_expansion(),
            scopes: self.scopes.union(macro_def_scopes),
            phase: Phase::ExpandTime,
            uniform_tag: None,
        }
    }

    /// 深注入作用域（TD-004/r13，Racket 集合作用域模型）：本语法对象与
    /// 全部子项的作用域集并入 `scope`。绑定形式（lambda 等）在展开时
    /// 对绑定器与全体体形式执行本注入——体内引用因此「看见」绑定作用域，
    /// 供 `(name, scopes ⊆)` 子集匹配解析。
    ///
    /// Rc 共享（H2）：共享子树经 `Rc::make_mut` 写时复制（本层 Vec 浅
    /// 克隆，子项仍共享）——注入语义与旧值语义版完全一致。**均匀标记
    ///清除**（TD-007 H2 健全性）：注入破坏子树作用域均匀性——标记置
    /// `None`，防 `retag_scopes` 快路径误共享。
    pub fn add_scope_to_all(&mut self, scope: crate::scope::ScopeId) {
        self.scopes.add(scope);
        self.uniform_tag = None;
        match &mut self.datum {
            StxDatum::List(items) | StxDatum::Vector(items) => {
                let items = Rc::make_mut(items);
                for item in items.iter_mut() {
                    item.add_scope_to_all(scope);
                }
            }
            StxDatum::Symbol(_) | StxDatum::Literal(_) => {}
        }
    }

    /// 数据化渲染（调试/dump：`(if x 1 2)` 形式，符号名由调用方传入解析表）。
    pub fn render(&self, resolve: &dyn Fn(Symbol) -> String) -> String {
        match &self.datum {
            StxDatum::Symbol(s) => resolve(*s),
            StxDatum::Literal(l) => l.render(),
            StxDatum::List(items) => {
                let inner: Vec<String> = items.iter().map(|x| x.render(resolve)).collect();
                format!("({})", inner.join(" "))
            }
            StxDatum::Vector(items) => {
                let inner: Vec<String> = items.iter().map(|x| x.render(resolve)).collect();
                format!("[{}]", inner.join(" "))
            }
        }
    }

    /// 整体 Span（含全部子项，构造父节点 Span 用）。
    pub fn total_span(&self) -> Span {
        match &self.datum {
            StxDatum::List(items) | StxDatum::Vector(items) => {
                let mut span = self.span;
                for item in items.iter() {
                    span = span.merge(item.span);
                }
                span
            }
            _ => self.span,
        }
    }
}

/// **扁平式深链拆除（TD-007/H2）**：`Stx` 的自定义 `Drop`——唯一持有的
/// 列表/向量脊柱（`Rc` 引用计数 = 1）迭代式拆解入工作表，子项逐个出栈
/// 处理；共享子树（计数 > 1）仅引用计数递减（天然无递归）。旧值语义深
/// 树的 drop 递归约束由此解除（10_000 层链解体栈深恒定）。
impl Drop for Stx {
    fn drop(&mut self) {
        // 快路径：非容器 datum（符号/字面量）——默认拆除即可（零递归）。
        if !matches!(self.datum, StxDatum::List(_) | StxDatum::Vector(_)) {
            return;
        }
        let mut work: Vec<Stx> = Vec::new();
        if let Some(items) = take_owned_children(&mut self.datum) {
            work.extend(items);
        }
        while let Some(mut node) = work.pop() {
            if let Some(items) = take_owned_children(&mut node.datum) {
                work.extend(items);
            }
        }
    }
}

/// 取出 datum 内唯一持有的子项集；共享（或弱引用持有）时返回 `None`
/// 并让取出的 `Rc` 递减（本节点即将析构，无需放回）。
fn take_owned_children(datum: &mut StxDatum) -> Option<Vec<Stx>> {
    if !matches!(datum, StxDatum::List(_) | StxDatum::Vector(_)) {
        return None;
    }
    // 哑值占位（本 Stx 正在析构——占位值不会外泄）
    let taken = std::mem::replace(datum, StxDatum::Symbol(Symbol(u32::MAX)));
    match taken {
        StxDatum::List(rc) | StxDatum::Vector(rc) => {
            // 共享持有（Err 臂）：Rc 丢弃 = 计数递减，无递归拆解
            Rc::try_unwrap(rc).ok()
        }
        _ => unreachable!("上方已窄化为容器变体"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scopes(ids: &[ScopeId]) -> ScopeSet {
        ScopeSet::from_iter_scopes(ids.iter().copied())
    }

    type ScopeId = u32;

    #[test]
    fn render_list_nested() {
        let s = Stx::list(
            vec![
                Stx::symbol(Symbol(1), Span::dummy(), scopes(&[])),
                Stx::literal(StxLiteral::Int(2), Span::dummy(), scopes(&[])),
            ],
            Span::dummy(),
            scopes(&[]),
        );
        assert_eq!(s.render(&|sym: Symbol| format!("#{}", sym.0)), "(#1 2)");
    }

    #[test]
    fn macro_expansion_bumps_phase_and_scopes() {
        let base = Stx::symbol(Symbol(3), Span::new(0, 1, 5), scopes(&[1]));
        let def_scopes = scopes(&[9]);
        let expanded = base.as_macro_expansion(&def_scopes);
        assert_eq!(expanded.phase, Phase::ExpandTime);
        assert_eq!(expanded.span.expansion_id, 1);
        assert!(expanded.scopes.contains(1) && expanded.scopes.contains(9));
    }

    #[test]
    fn list_head_symbol_extraction() {
        let s = Stx::list(
            vec![
                Stx::symbol(Symbol(7), Span::dummy(), scopes(&[])),
                Stx::symbol(Symbol(8), Span::dummy(), scopes(&[])),
            ],
            Span::dummy(),
            scopes(&[]),
        );
        assert_eq!(s.datum.list_head_symbol(), Some(Symbol(7)));
        assert_eq!(
            Stx::symbol(Symbol(7), Span::dummy(), scopes(&[]))
                .datum
                .list_head_symbol(),
            None
        );
    }

    #[test]
    fn literal_render() {
        assert_eq!(StxLiteral::Int(-5).render(), "-5");
        assert_eq!(StxLiteral::Bool(true).render(), "true");
        assert_eq!(StxLiteral::Nil.render(), "nil");
        assert_eq!(StxLiteral::Float(1.5).render(), "1.5");
    }
}
