//! GC 堆：槽位分配 + 标记-清除回收（stage0.md §8.11 / §19.4 三阶段骨架）。
//!
//! 结构：`slots` 为对象槽位向量（`GcRef` = 槽位索引）；sweep 将未标记槽
//! 归还 `free` 空闲表（Stage 0「只分配不压缩」策略，§19.4 陷阱 3）。

use std::any::Any;
use std::rc::Rc;

use crate::gc::RootSet;
use crate::io::RuntimeError;

/// 堆对象引用（槽位索引）。构造仅经 `Heap`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GcRef(pub u32);

/// 堆对象种类（Layer 0 自包含对象模型）。
/// 闭包/内置函数的序对元素形态 = `Foreign`（TD-010 r24 解决：
/// 原标记字符串占位 → 真装箱，往返恒等性保持）。
#[derive(Clone)]
pub enum HeapObj {
    /// 序对 (car . cdr)。
    Pair(GcRef, GcRef),
    /// 装箱字符串。
    Str(Rc<str>),
    /// 装箱整数（序对元素为即时值时的槽位形态）。
    Int(i64),
    /// 装箱浮点。
    Float(f64),
    /// 装箱布尔。
    Bool(bool),
    /// 装箱符号（quote 符号值的槽位形态——TD-002）。
    Symbol(Rc<str>),
    /// 装箱 nil。
    Nil,
    /// 装箱 Foreign 值（闭包/内置函数——TD-010）：`any` 承载装箱方
    /// 类型（kerf-vm 的函数值）；`tracer` 由装箱方提供，标记阶段
    /// 枚举其 GC 可见子引用（闭包捕获图）——kerf-runtime 不依赖
    /// kerf-vm 类型（§11 接口隔离：追踪协议而非类型耦合）。
    Foreign(Rc<ForeignBox>),
}

/// Foreign 追踪器：枚举装箱值内 GC 可见子引用（闭包捕获图内的
/// Pair 引用全量入 out——Rc 去重防环由实现方承担）。
pub type ForeignTracer = fn(&Rc<dyn Any>, &mut Vec<GcRef>);

/// Foreign 装箱值载体（TD-010）：值本体 + 追踪器。
#[derive(Clone)]
pub struct ForeignBox {
    /// 装箱的函数值（kerf-vm Value::Closure/Builtin 的类型擦除形态；
    /// Rc 共享 → 解箱往返保持恒等性（eq? 按引用相等））。
    pub any: Rc<dyn Any>,
    /// 标记阶段子引用追踪器（装箱方注入）。
    pub tracer: ForeignTracer,
}

// `Rc<dyn Any>` 不可派生 Debug/PartialEq——手工实现（载体语义：
// 值恒等 = Rc 同一分配；追踪器是装箱位点的伴生属性，不参与恒等判定）。
impl std::fmt::Debug for ForeignBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ForeignBox(..)")
    }
}

impl PartialEq for ForeignBox {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.any, &other.any)
    }
}

impl std::fmt::Debug for HeapObj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeapObj::Pair(a, b) => write!(f, "Pair({:?}, {:?})", a, b),
            HeapObj::Str(s) => write!(f, "Str({:?})", s),
            HeapObj::Int(i) => write!(f, "Int({})", i),
            HeapObj::Float(v) => write!(f, "Float({})", v),
            HeapObj::Bool(b) => write!(f, "Bool({})", b),
            HeapObj::Symbol(s) => write!(f, "Symbol({:?})", s),
            HeapObj::Nil => write!(f, "Nil"),
            HeapObj::Foreign(_) => write!(f, "Foreign(..)"),
        }
    }
}

impl PartialEq for HeapObj {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (HeapObj::Pair(a1, b1), HeapObj::Pair(a2, b2)) => a1 == a2 && b1 == b2,
            (HeapObj::Str(a), HeapObj::Str(b)) => a == b,
            (HeapObj::Int(a), HeapObj::Int(b)) => a == b,
            (HeapObj::Float(a), HeapObj::Float(b)) => a == b,
            (HeapObj::Bool(a), HeapObj::Bool(b)) => a == b,
            (HeapObj::Symbol(a), HeapObj::Symbol(b)) => a == b,
            (HeapObj::Nil, HeapObj::Nil) => true,
            (HeapObj::Foreign(a), HeapObj::Foreign(b)) => Rc::ptr_eq(&a.any, &b.any),
            // _ 臂理由：不同构造子的组合（跨类型装箱值）不相等——类型严格相等
            _ => false,
        }
    }
}

/// 单个堆槽位。
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    pub obj: HeapObj,
    pub marked: bool,
}

/// GC 堆（标记-清除）。
#[derive(Debug, Clone)]
pub struct Heap {
    slots: Vec<Slot>,
    free: Vec<GcRef>,
    /// GC 触发阈值（存活对象数超过即回收——由 VM 安全点检查）。
    gc_threshold: usize,
    /// GC 开关（eval 路径关闭：无完整根集时禁用回收，见 crate 文档）。
    gc_enabled: bool,
    /// foreign ref 持久根（§8.8/§15 预留：FFI 根登记，Stage 0 可为空集）。
    foreign_roots: Vec<GcRef>,
    /// pin 计数簿 Φ（r30/48-d——ffi-ownership-model §3：`foreign_roots`
    /// 的**计数化泛化**。`Vec<(GcRef, u32)>` 有序对（supp(Φ) 插入序；
    /// 计数归零即移除——「unpin 到 0 → 从 supp(Φ) 摘除，下一轮回收
    /// 周期可回收」的运行形态）。与持久根并存：持久根 = 零计数前身
    /// （登记即根保存，永不摘除）；Φ = 窗口借用配对计数。
    pin_counts: Vec<(GcRef, u32)>,
    /// 统计：总分配数。
    total_allocs: u64,
    /// 距上次回收的分配数（分配计数驱动的触发器——回收成本按分配量摊销）。
    allocs_since_gc: u64,
    /// 统计：回收次数。
    collections: u64,
    /// 统计：累计回收对象数。
    total_freed: u64,
}

impl Default for Heap {
    fn default() -> Self {
        Heap::new()
    }
}

impl Heap {
    /// 构造（默认阈值：分配 1024 次触发回收——测试可调）。
    pub fn new() -> Self {
        Heap::with_threshold(1024)
    }

    /// 指定阈值构造。
    pub fn with_threshold(gc_threshold: usize) -> Self {
        Heap {
            slots: Vec::new(),
            free: Vec::new(),
            gc_threshold,
            gc_enabled: true,
            foreign_roots: Vec::new(),
            pin_counts: Vec::new(),
            total_allocs: 0,
            allocs_since_gc: 0,
            collections: 0,
            total_freed: 0,
        }
    }

    /// 分配序对（bump 风格：优先空闲表复用，否则追加新槽）。
    /// 分配零内卷（§19.4 不变式 3）：此路径不触发回收、不触发任何分配性调用。
    pub fn alloc_pair(&mut self, car: GcRef, cdr: GcRef) -> GcRef {
        let obj = HeapObj::Pair(car, cdr);
        self.alloc_obj(obj)
    }

    /// 分配装箱字符串。
    pub fn alloc_str(&mut self, s: Rc<str>) -> GcRef {
        self.alloc_obj(HeapObj::Str(s))
    }

    /// 分配装箱整数。
    pub fn alloc_int(&mut self, v: i64) -> GcRef {
        self.alloc_obj(HeapObj::Int(v))
    }

    /// 分配装箱浮点。
    pub fn alloc_float(&mut self, v: f64) -> GcRef {
        self.alloc_obj(HeapObj::Float(v))
    }

    /// 分配装箱布尔。
    pub fn alloc_bool(&mut self, v: bool) -> GcRef {
        self.alloc_obj(HeapObj::Bool(v))
    }

    /// 分配装箱 nil。
    pub fn alloc_nil(&mut self) -> GcRef {
        self.alloc_obj(HeapObj::Nil)
    }

    /// 分配装箱符号（TD-002：quote 符号值入序对）。
    pub fn alloc_symbol(&mut self, s: Rc<str>) -> GcRef {
        self.alloc_obj(HeapObj::Symbol(s))
    }

    /// 分配装箱 Foreign 值（TD-010：闭包/内置函数的序对元素形态）。
    pub fn alloc_foreign(&mut self, b: ForeignBox) -> GcRef {
        self.alloc_obj(HeapObj::Foreign(Rc::new(b)))
    }

    /// 装箱即时值（TD-010 r24：闭包/内置经 Foreign 形态可装箱）。
    pub fn alloc_boxed(&mut self, v: BoxedInput) -> GcRef {
        match v {
            BoxedInput::Str(s) => self.alloc_obj(HeapObj::Str(s)),
            BoxedInput::Int(i) => self.alloc_obj(HeapObj::Int(i)),
            BoxedInput::Float(f) => self.alloc_obj(HeapObj::Float(f)),
            BoxedInput::Bool(b) => self.alloc_obj(HeapObj::Bool(b)),
            BoxedInput::Nil => self.alloc_obj(HeapObj::Nil),
            BoxedInput::Foreign(b) => self.alloc_obj(HeapObj::Foreign(Rc::new(b))),
        }
    }

    /// 解箱为值槽描述（kerf-vm 转换为 Value；序对槽返回 Pair 标记，
    /// 调用方以自身引用包装为 Value::Pair）。
    pub fn unbox(&self, r: GcRef) -> Option<ValueSlot> {
        self.get(r).map(|s| match &s.obj {
            HeapObj::Pair(..) => ValueSlot::Pair,
            HeapObj::Str(v) => ValueSlot::Str(v.clone()),
            HeapObj::Int(v) => ValueSlot::Int(*v),
            HeapObj::Float(v) => ValueSlot::Float(*v),
            HeapObj::Bool(v) => ValueSlot::Bool(*v),
            HeapObj::Symbol(v) => ValueSlot::Symbol(v.clone()),
            HeapObj::Nil => ValueSlot::Nil,
            HeapObj::Foreign(b) => ValueSlot::Foreign(b.clone()),
        })
    }

    fn alloc_obj(&mut self, obj: HeapObj) -> GcRef {
        self.total_allocs += 1;
        self.allocs_since_gc += 1;
        if let Some(r) = self.free.pop() {
            self.slots[r.0 as usize].obj = obj;
            self.slots[r.0 as usize].marked = false;
            return r;
        }
        let r = GcRef(self.slots.len() as u32);
        self.slots.push(Slot { obj, marked: false });
        r
    }

    /// 读取序对（类型不匹配返回 None——显式失败）。
    pub fn get_pair(&self, r: GcRef) -> Option<(GcRef, GcRef)> {
        match &self.get(r)?.obj {
            HeapObj::Pair(a, b) => Some((*a, *b)),
            _ => None,
        }
    }

    /// 读取对象。
    pub fn get(&self, r: GcRef) -> Option<&Slot> {
        self.slots.get(r.0 as usize)
    }

    /// 存活对象数（含未回收的垃圾——真实存活需在 mark 后统计）。
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// 空闲槽数。
    pub fn free_count(&self) -> usize {
        self.free.len()
    }

    /// GC 开关（eval 路径：无完整根集时禁用）。
    pub fn set_gc_enabled(&mut self, enabled: bool) {
        self.gc_enabled = enabled;
    }

    /// 是否到达回收阈值（分配计数驱动——回收成本按分配量摊销；
    /// VM 指令边界安全点轮询，§19.4 陷阱 2）。
    pub fn should_collect(&self) -> bool {
        self.gc_enabled && self.allocs_since_gc > self.gc_threshold as u64
    }

    /// 登记 foreign ref（§8.8：FFI 根预留；Stage 0 持久根集合，可为 no-op 使用）。
    pub fn register_foreign_ref(&mut self, r: GcRef) {
        if !self.foreign_roots.contains(&r) {
            self.foreign_roots.push(r);
        }
    }

    /// foreign 根集合（GC 周期并入根集）。
    pub fn foreign_roots(&self) -> &[GcRef] {
        &self.foreign_roots
    }

    /// pin 对象（P1 规则：`Φ[v] += 1`；未登记则插入计数 1）。
    /// 可重复 pin 同一对象——计数累计（引计数式屏障）。
    pub fn pin_object(&mut self, r: GcRef) {
        if let Some(entry) = self.pin_counts.iter_mut().find(|(g, _)| *g == r) {
            entry.1 += 1;
        } else {
            self.pin_counts.push((r, 1));
        }
    }

    /// unpin 对象（U1 规则：`Φ[v] -= 1`，归零移除出 supp(Φ)——
    /// 对象**下一轮**回收周期才可回收，非立即回收）。
    /// U2 规则：未登记（`Φ(v) = 0`）→ `Err`（06-操作语义 §3 E8
    /// 口径——内部不变式破坏 = 边界实现簿记缺陷，P0 级，非用户错误；
    /// 窗口规程结构性配对保证用户程序不可触达）。
    pub fn unpin_object(&mut self, r: GcRef) -> Result<(), RuntimeError> {
        let idx = self.pin_counts.iter().position(|(g, _)| *g == r);
        match idx {
            Some(i) => {
                self.pin_counts[i].1 -= 1;
                if self.pin_counts[i].1 == 0 {
                    self.pin_counts.remove(i);
                }
                Ok(())
            }
            None => Err(RuntimeError::new(
                "pin 计数下溢（E8 口径——边界实现簿记缺陷，结构配对保证不可达）",
            )),
        }
    }

    /// Φ 计数查询（P1/U1 可观测性——测试与协议审计面）。
    pub fn pin_count(&self, r: GcRef) -> u32 {
        self.pin_counts
            .iter()
            .find(|(g, _)| *g == r)
            .map(|(_, c)| *c)
            .unwrap_or(0)
    }

    /// supp(Φ) 引用集（计数 > 0 的全部引用——mark 起点并入；
    /// GC 周期 `collect` 消费，外部观测供测试）。
    pub fn pinned_refs(&self) -> Vec<GcRef> {
        self.pin_counts.iter().map(|(g, _)| *g).collect()
    }

    /// 子引用枚举（标记阶段的图遍历边）。
    fn children(&self, r: GcRef) -> Vec<GcRef> {
        match &self.get(r).map(|s| &s.obj) {
            Some(HeapObj::Pair(a, b)) => vec![*a, *b],
            // TD-010：Foreign 槽位经装箱方注入的追踪器枚举闭包捕获图
            // （Pair 引用全量——内建值无子引用，追踪器为 no-op）。
            Some(HeapObj::Foreign(b)) => {
                let mut out = Vec::new();
                (b.tracer)(&b.any, &mut out);
                out
            }
            // _ 臂理由：非序对堆对象（Str/Int/Float/Bool/Nil）与越界引用均无子引用——标记图遍历无出边
            _ => Vec::new(),
        }
    }

    /// 标记复位（§19.4 不变式 2：遍历全部对象，含刚回收的）。
    fn clear_marks(&mut self) {
        for s in &mut self.slots {
            s.marked = false;
        }
    }

    /// 统计信息。
    pub fn stats(&self) -> GcStats {
        GcStats {
            slots: self.slots.len(),
            free: self.free.len(),
            live: self.slots.len() - self.free.len(),
            total_allocs: self.total_allocs,
            collections: self.collections,
            total_freed: self.total_freed,
        }
    }

    /// 内部：执行标记-清除（供 gc::mark_sweep_cycle 调用）。
    pub(crate) fn collect(&mut self, roots: &RootSet) -> GcStats {
        // 分配计数复位（成本摊销窗口重新计）
        self.allocs_since_gc = 0;
        // 1. 标记：显式工作栈（§19.4——防递归爆栈）
        let mut work: Vec<GcRef> = roots.refs.clone();
        work.extend_from_slice(&self.foreign_roots);
        // supp(Φ) 并入 mark 起点（ffi-ownership-model §3——Φ 独立于
        // 标记位持久：计数簿是程序态，标记位是周期态；F-PIN 引理：
        // 周期起点 Φ(v) ≥ 1 ⟹ 本轮 sweep 不回收 v）
        for (r, _) in &self.pin_counts {
            work.push(*r);
        }
        while let Some(r) = work.pop() {
            if let Some(slot) = self.slots.get_mut(r.0 as usize) {
                if slot.marked {
                    continue;
                }
                slot.marked = true;
            } else {
                continue;
            }
            for child in self.children(r) {
                work.push(child);
            }
        }

        // 2. 清除：线性遍历堆（保守策略：泄漏容忍，误回收零容忍）
        let mut freed = 0usize;
        let mut new_free: Vec<GcRef> = Vec::new();
        for (i, slot) in self.slots.iter().enumerate() {
            if !slot.marked {
                new_free.push(GcRef(i as u32));
                freed += 1;
            }
        }
        // free 表重建（旧 free 槽位的 obj 已被复用逻辑覆盖——重置为全量未标记槽）
        self.free = new_free;
        self.total_freed += freed as u64;
        self.collections += 1;

        // 3. 复位（为下一轮准备）
        self.clear_marks();

        self.stats()
    }
}

/// 装箱输入（Layer 0 即时值描述；TD-010 r24：闭包/内置经 Foreign
/// 形态可装箱）。
#[derive(Debug, Clone, PartialEq)]
pub enum BoxedInput {
    Str(Rc<str>),
    Int(i64),
    Float(f64),
    Bool(bool),
    Nil,
    Foreign(ForeignBox),
}

/// 解箱输出（`Pair` 标记序对槽——调用方以槽位引用包装；`Foreign`
/// 携带装箱载体——调用方（kerf-vm）自行 downcast 还原函数值）。
#[derive(Debug, Clone, PartialEq)]
pub enum ValueSlot {
    Pair,
    Str(Rc<str>),
    Int(i64),
    Float(f64),
    Bool(bool),
    Symbol(Rc<str>),
    Nil,
    Foreign(Rc<ForeignBox>),
}

/// GC 统计快照。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GcStats {
    /// 当前槽位总数。
    pub slots: usize,
    /// 空闲槽数。
    pub free: usize,
    /// 存活（未入空闲表）槽数。
    pub live: usize,
    /// 累计分配数。
    pub total_allocs: u64,
    /// 回收次数。
    pub collections: u64,
    /// 累计回收对象数。
    pub total_freed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_reuses_free_slots() {
        let mut heap = Heap::new();
        let a = heap.alloc_pair(GcRef(0), GcRef(0));
        let b = heap.alloc_pair(GcRef(0), GcRef(0));
        assert_ne!(a, b);
        assert_eq!(heap.slot_count(), 2);
    }

    #[test]
    fn collect_frees_unreachable() {
        let mut heap = Heap::new();
        // 孤儿链：(1 . (2 . nil)) 无根
        let nil = heap.alloc_str(Rc::from(""));
        let two = heap.alloc_str(Rc::from("2"));
        let one = heap.alloc_str(Rc::from("1"));
        let _inner = heap.alloc_pair(two, nil);
        let outer = heap.alloc_pair(one, two);

        let roots = RootSet::from_refs([outer]);
        let stats = heap.collect(&roots);
        // 5 个对象：outer 可达 → outer/one/two 存活 3；nil/inner 回收 2
        assert_eq!(
            stats.free, 2,
            "5 个对象中可达 3（outer+one+two），回收 2（nil+inner）"
        );
        assert_eq!(stats.live, 3);
        assert_eq!(heap.stats().collections, 1);
        // 复用：分配命中空闲表
        let x = heap.alloc_pair(GcRef(0), GcRef(0));
        assert!(x.0 < 5);
        assert_eq!(heap.free_count(), 1);
    }

    #[test]
    fn collect_keeps_cycle_from_roots() {
        // 自环：a = (a . a)——从根可达的自环不被回收
        let mut heap = Heap::new();
        let a = heap.alloc_pair(GcRef(0), GcRef(0));
        // 写入自环（直接操纵槽位）
        heap.slots[a.0 as usize].obj = HeapObj::Pair(a, a);
        let roots = RootSet::from_refs([a]);
        let stats = heap.collect(&roots);
        assert_eq!(stats.live, 1);
        assert_eq!(stats.free, 0);
    }

    #[test]
    fn unrooted_cycle_is_collected() {
        // 无根自环：不可达 → 回收（标记-清除天然处理环）
        let mut heap = Heap::new();
        let a = heap.alloc_pair(GcRef(0), GcRef(0));
        heap.slots[a.0 as usize].obj = HeapObj::Pair(a, a);
        let roots = RootSet::from_refs([]);
        let stats = heap.collect(&roots);
        assert_eq!(stats.free, 1);
    }

    #[test]
    fn deep_chain_no_stack_overflow() {
        // 10^5 深度链：显式工作栈标记（§19.4 陷阱 1 的回归测试）
        let mut heap = Heap::with_threshold(usize::MAX);
        let nil = heap.alloc_str(Rc::from("nil"));
        let mut head = nil;
        for _ in 0..100_000 {
            head = heap.alloc_pair(nil, head);
        }
        let roots = RootSet::from_refs([head]);
        let stats = heap.collect(&roots);
        assert_eq!(stats.live, 100_001, "全部链节点 + nil 存活");
        assert_eq!(stats.free, 0);
    }

    #[test]
    fn foreign_roots_survive() {
        let mut heap = Heap::new();
        let s = heap.alloc_str(Rc::from("foreign"));
        heap.register_foreign_ref(s);
        let roots = RootSet::from_refs([]);
        let stats = heap.collect(&roots);
        assert_eq!(stats.live, 1, "foreign root 存活");
    }

    #[test]
    fn gc_disabled_stays_put() {
        let mut heap = Heap::new();
        heap.set_gc_enabled(false);
        let _ = heap.alloc_str(Rc::from("x"));
        let roots = RootSet::from_refs([]);
        let stats = heap.collect(&roots);
        // gc_enabled=false 时 collect 仍可被显式调用（eval 路径约定为不调用）
        assert_eq!(stats.free, 1);
        // should_collect 恒 false
        assert!(!heap.should_collect());
    }
}
