//! 作用域集（ScopeSet）：卫生宏的语法标记基础（stage0.md §3.3 / §19.2）。
//!
//! 每个语法对象携带一个作用域集合。绑定形式引入新作用域（mark），
//! 宏展开产物携带「宏定义处作用域 ∪ 使用处作用域」（§19.2 卫生性关键）。
//!
//! 不变式（§19.2 不变式 2）：宏引入的标识符作用域集 ≠ 用户代码作用域集，
//! 二者在 SyntaxObject 中**永不合并为一个集合**。

/// 作用域标识（全局唯一分配的句柄）。
pub type ScopeId = u32;

/// 有序作用域集合（去重、升序）。成员关系与包含关系为 O(n)，
/// Stage 0 规模下（每个标识符 ≤ 数十个作用域）足够。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ScopeSet(Vec<ScopeId>);

impl ScopeSet {
    /// 空集。
    pub fn new() -> Self {
        ScopeSet(Vec::new())
    }

    /// 从迭代器构造（内部去重排序）。
    pub fn from_iter_scopes(scopes: impl IntoIterator<Item = ScopeId>) -> Self {
        let mut v: Vec<ScopeId> = scopes.into_iter().collect();
        v.sort_unstable();
        v.dedup();
        ScopeSet(v)
    }

    /// 添加作用域（保持有序去重）。
    pub fn add(&mut self, scope: ScopeId) {
        match self.0.binary_search(&scope) {
            Ok(_) => {}
            Err(pos) => self.0.insert(pos, scope),
        }
    }

    /// 是否包含某作用域。
    pub fn contains(&self, scope: ScopeId) -> bool {
        self.0.binary_search(&scope).is_ok()
    }

    /// 并集（卫生展开：定义处 ∪ 使用处）。
    pub fn union(&self, other: &ScopeSet) -> ScopeSet {
        ScopeSet::from_iter_scopes(self.0.iter().copied().chain(other.0.iter().copied()))
    }

    /// 子集判定：`self ⊆ other`（绑定匹配：绑定引入的作用域必须是引用处作用域的子集）。
    pub fn is_subset_of(&self, other: &ScopeSet) -> bool {
        self.0.iter().all(|s| other.contains(*s))
    }

    /// 移除作用域（下降遍历恢复栈时使用）。
    pub fn remove(&mut self, scope: ScopeId) {
        if let Ok(pos) = self.0.binary_search(&scope) {
            self.0.remove(pos);
        }
    }

    /// 成员数量。
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 迭代作用域（升序）。
    pub fn iter(&self) -> impl Iterator<Item = ScopeId> + '_ {
        self.0.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_keeps_sorted_dedup() {
        let mut s = ScopeSet::new();
        s.add(3);
        s.add(1);
        s.add(3);
        s.add(2);
        assert_eq!(s.iter().collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn union_and_subset() {
        let a = ScopeSet::from_iter_scopes([1, 2]);
        let b = ScopeSet::from_iter_scopes([2, 3]);
        let u = a.union(&b);
        assert_eq!(u.iter().collect::<Vec<_>>(), vec![1, 2, 3]);
        assert!(a.is_subset_of(&u));
        assert!(!u.is_subset_of(&a));
    }

    #[test]
    fn hygiene_invariant_two_scope_sets_never_merge_to_single() {
        // §19.2 不变式 2：宏引入作用域与用户作用域并集保留二者（非折叠为单元素）
        let macro_scopes = ScopeSet::from_iter_scopes([10]);
        let user_scopes = ScopeSet::from_iter_scopes([1, 2]);
        let expanded = macro_scopes.union(&user_scopes);
        assert_eq!(expanded.len(), 3);
        assert!(expanded.contains(10) && expanded.contains(1) && expanded.contains(2));
    }
}
