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
#[derive(Debug, Clone, PartialEq)]
pub enum StxDatum {
    /// 标识符出现。
    Symbol(Symbol),
    /// 字面量出现。
    Literal(StxLiteral),
    /// 圆括号列表（核心形式的载体）。
    List(Vec<Stx>),
    /// 方括号向量（语法糖数据，Stage 0 主要用于 let 绑定组与宏字面量组）。
    Vector(Vec<Stx>),
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
#[derive(Debug, Clone, PartialEq)]
pub struct Stx {
    pub datum: StxDatum,
    /// 源码位置——必须字段（§3.3：不可为空；Reader 保证）。
    pub span: Span,
    /// 作用域集合（卫生宏基础）。
    pub scopes: ScopeSet,
    /// 所属相位（Reader 产出时为 Runtime；宏产物为 ExpandTime，§8.9）。
    pub phase: Phase,
}

impl Stx {
    /// 构造符号语法对象。
    pub fn symbol(sym: Symbol, span: Span, scopes: ScopeSet) -> Self {
        Stx {
            datum: StxDatum::Symbol(sym),
            span,
            scopes,
            phase: Phase::Runtime,
        }
    }

    /// 构造列表语法对象（Span 由调用方合并提供）。
    pub fn list(items: Vec<Stx>, span: Span, scopes: ScopeSet) -> Self {
        Stx {
            datum: StxDatum::List(items),
            span,
            scopes,
            phase: Phase::Runtime,
        }
    }

    /// 构造字面量语法对象。
    pub fn literal(value: StxLiteral, span: Span, scopes: ScopeSet) -> Self {
        Stx {
            datum: StxDatum::Literal(value),
            span,
            scopes,
            phase: Phase::Runtime,
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
        }
    }

    /// 深注入作用域（TD-004/r13，Racket 集合作用域模型）：本语法对象与
    /// 全部子项的作用域集并入 `scope`。绑定形式（lambda 等）在展开时
    /// 对绑定器与全体体形式执行本注入——体内引用因此「看见」绑定作用域，
    /// 供 `(name, scopes ⊆)` 子集匹配解析。
    pub fn add_scope_to_all(&mut self, scope: crate::scope::ScopeId) {
        self.scopes.add(scope);
        match &mut self.datum {
            StxDatum::List(items) | StxDatum::Vector(items) => {
                for item in items {
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
                for item in items {
                    span = span.merge(item.span);
                }
                span
            }
            _ => self.span,
        }
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
