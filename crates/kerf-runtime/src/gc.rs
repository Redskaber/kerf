//! 标记-清除回收入口（stage0.md §19.4 `gc_cycle` 三阶段）。
//!
//! ```text
//! fn gc_cycle(heap, roots) {
//!     mark_all(roots, heap);   // 1. 标记：从根集可达的全部对象置位
//!     sweep(heap);             // 2. 清除：未标记对象归还 free list
//!     heap.clear_marks();      // 3. 复位：为下一轮准备
//! }
//! ```

use crate::heap::{GcRef, GcStats, Heap};

/// 根集（§19.4：VM 栈 + 调用栈局部 + 全局环境 + foreign ref——
/// 前三类由 VM 在安全点收集为 `GcRef` 列表传入，foreign 由 Heap 自持）。
#[derive(Debug, Clone, Default)]
pub struct RootSet {
    /// 可达堆引用（由调用方枚举：值栈/帧局部/全局环境/闭包捕获）。
    pub refs: Vec<GcRef>,
}

impl RootSet {
    pub fn new() -> Self {
        RootSet { refs: Vec::new() }
    }

    /// 从引用列表构造。
    pub fn from_refs(refs: impl IntoIterator<Item = GcRef>) -> Self {
        RootSet {
            refs: refs.into_iter().collect(),
        }
    }

    /// 追加根。
    pub fn push(&mut self, r: GcRef) {
        self.refs.push(r);
    }

    /// 根数量。
    pub fn len(&self) -> usize {
        self.refs.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.refs.is_empty()
    }
}

/// 执行一轮标记-清除（§19.4 `gc_cycle`）。
pub fn mark_sweep_cycle(heap: &mut Heap, roots: &RootSet) -> GcStats {
    heap.collect(roots)
}
