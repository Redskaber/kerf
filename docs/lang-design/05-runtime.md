# 运行时：最小 I/O 与标记-清除 GC

> **Author**: kerf-doc-agent
> **Date**: 2026-09-09（v5.1 增补：分配器/堆契约从 Stage 0 冻结实现回填）
> **Version**: v5.1
> **Status**: Active
> **处理程度**：GC 为 P0（安全关键组件，Stage 0 已实现）；最小 I/O 为 P1（传统全局函数，能力模型 I/O 接口预留）｜ **所属 Stage**：Stage 0 ｜ **推迟项**：分代/增量/并发 GC（Stage 2）、能力模型 I/O（Stage 2，[13-能力矩阵 §3.1.3](./13-capability-matrix.md)）、屏障接口实现（Stage 0 no-op）

> 本文件收录运行时基座的设计与实现：最小 I/O（原 §8.8）、标记-清除 GC 的最小实现（原 §8.11）、内存管理策略（原 §14.2，v5.1 从冻结实现回填分配器/堆/根集契约），以及标记-清除 GC 实现框架（原 §19.4：三阶段骨架 + 核心不变式 + 实现陷阱）。字节码 VM（运行时基座的另一半）见 [04-字节码 VM](./04-bytecode-vm.md)；最小内置库的函数级边界见 [09-标准库](./09-stdlib.md)；12 个能力模型的完整矩阵见 [13-能力矩阵](./13-capability-matrix.md)（其 §2.8/§2.11 为本文件 §1/§2 的规范副本）；高级 GC 的推迟理由见同文件 §3。操作语义侧的 GC 不可观测性引理（L-GC）见 [06-操作语义 §4](./06-operational-semantics.md)。

---

## 1. 最小 I/O（传统）（原 §8.8）

仅 `read_line()` 和 `write_line()`，通过全局函数（非能力模型）。Stage 0 不引入能力模型 I/O，但 VM 栈帧和分配器接口必须预留 `register_foreign_ref` 等接口（Stage 0 可为 no-op），以便 Stage 1+ 升级到能力模型时无需破坏接口。

**I/O 通道契约（Stage 0 冻结，kerf-runtime/src/io.rs）**：

```rust
/// 运行时错误（错误显式返回，禁止异常——接口契约规则 2）。
pub struct RuntimeError { pub message: String }

/// 全局 stdout 行写入（副作用唯一出口之一）。
pub fn write_line_stdout(s: &str) -> Result<(), RuntimeError>;
/// 全局 stdin 行读取（EOF 返回 None；副作用唯一出口之二）。
pub fn read_line_stdin() -> Result<Option<String>, RuntimeError>;
```

**升级路径（扩展预留接口）**：Stage 2 升级到能力模型 I/O 时，本二函数由 `ReadCapability / WriteCapability` trait（[13-能力矩阵 §3.1.3](./13-capability-matrix.md) 冻结签名）的实例替换——内置函数 δ 表（[09-标准库 §2](./09-stdlib.md)）从直接调用全局函数改为经能力对象分发，调用点接口不变（[17-设计原则 §1 原则 28](./17-principles.md)：渐进替换）。

## 2. 标记-清除 GC（最小实现）（原 §8.11）

Bump-pointer 分配 + 递归标记 + 堆遍历清除。根集包括 VM 栈、调用栈局部变量、全局环境和 C 栈显式根。Stage 0 用最简单的 mark-sweep，避免分代/增量/并发的复杂度。

**实现指引**：分配、标记、清除三个阶段的完整伪代码与栈溢出对策见本文 §4。

## 3. 内存管理策略（原 §14.2，v5.1 契约回填）

分配器接口协议包含基础分配、屏障接口（Stage 0 为 no-op）、根集管理和 FFI 外部引用追踪。堆对象统一头格式支持从保守 GC 演进到精确/分代/增量 GC。

### 3.1 堆与分配器契约（Stage 0 冻结，kerf-runtime/src/heap.rs）

```rust
/// GC 引用（堆槽句柄——u32 索引，非裸指针：句柄稳定性不依赖内存布局）。
pub struct GcRef(pub u32);

/// 堆对象（全类型装箱：Pair/Str/Int/Float/Bool/Nil/Boxed）。
pub enum HeapObj { /* Pair(GcRef, GcRef) | Str(Rc<str>) | Int(i64) | Float(f64)
                     | Bool(bool) | Nil | Boxed(BoxedInput) */ }

/// 堆槽（分配粒度单位：对象 + 标记位 + 空闲链指针）。
pub struct Slot { /* obj: HeapObj; marked: bool; next_free: Option<u32> */ }

/// 堆（分配器 + 标记-清除回收器 + 外部引用登记簿）。
pub struct Heap {
    // slots: Vec<Slot>; free_list: Option<u32>; free_count: usize;
    // allocs_since_gc: usize; gc_threshold: usize; gc_enabled: bool;
    // foreign_refs: Vec<GcRef>;
}

impl Heap {
    /// 类型化分配（六入口——调用方携带类型信息，非统一 alloc(size)）。
    pub fn alloc_pair(&mut self, car: GcRef, cdr: GcRef) -> GcRef;
    pub fn alloc_str(&mut self, s: Rc<str>) -> GcRef;
    pub fn alloc_int(&mut self, v: i64) -> GcRef;
    pub fn alloc_float(&mut self, v: f64) -> GcRef;
    pub fn alloc_bool(&mut self, v: bool) -> GcRef;
    pub fn alloc_nil(&mut self) -> GcRef;

    /// 读取访问（不可变查询：unbox/get_pair/get）。
    pub fn unbox(&self, r: GcRef) -> Option<ValueSlot>;
    pub fn get_pair(&self, r: GcRef) -> Option<(GcRef, GcRef)>;

    /// GC 控制（分配计数驱动触发 + 冷却退避）。
    pub fn set_gc_enabled(&mut self, enabled: bool);
    pub fn should_collect(&self) -> bool;

    /// 外部引用登记（FFI 预留接口——[13-能力矩阵 §4](./13-capability-matrix.md) 决策 3）。
    /// Stage 0 语义：登记的引用作为根保存（非 no-op——正确性优先）；
    /// 屏障接口（write barrier）才是 Stage 0 no-op 项。
    pub fn register_foreign_ref(&mut self, r: GcRef);
    pub fn foreign_roots(&self) -> &[GcRef];

    /// 统计（基准与测试观测点）。
    pub fn stats(&self) -> GcStats;
}
```

**堆对象统一头格式（Slot 结构的语义）**：每个槽携带「对象载荷 + 标记位 + 空闲链指针」。从保守 GC 演进到精确/分代/增量 GC 时：(1) 标记位可直接升级为分代号（低 1-2 位复用）；(2) `next_free` 链可升级为分代空闲链；(3) `GcRef` 句柄格式不变——「数据结构的留白是廉价的」（[17-设计原则 §1 原则 20](./17-principles.md)）。

**分配触发协议**：分配计数驱动（`allocs_since_gc ≥ gc_threshold` 时下一安全点触发回收）+ **GC 冷却退避**（深递归场景连续触发回收的 O(n²) 消退——worklog Task 4-c 的关键性能修复：122s → 3.4s）。退避仅影响触发时机，不影响语义（[06-操作语义 §4 引理 L-GC](./06-operational-semantics.md)：GC 不可观测）。

### 3.2 根集契约（kerf-runtime/src/gc.rs）

```rust
/// 根集：回收周期的可达性起点集合。
pub struct RootSet { /* refs: Vec<GcRef> */ }

impl RootSet {
    pub fn new() -> Self;
    pub fn from_refs(refs: impl IntoIterator<Item = GcRef>) -> Self;
    pub fn push(&mut self, r: GcRef);
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}

/// 完整回收周期（标记 → 清除 → 复位，返回统计）。
pub fn mark_sweep_cycle(heap: &mut Heap, roots: &RootSet) -> GcStats;
```

**根集四来源（根集完备性不变式，本文 §4）**：(1) VM 数据栈；(2) 调用帧局部槽；(3) 全局环境；(4) `register_foreign_ref` 登记的外部引用。四类根缺一不可——漏根 = 悬垂指针（P0 级安全缺陷）。

**屏障接口的边界裁定**：Stage 0 的屏障为 no-op（无分代/并发，无屏障需求）；`register_foreign_ref` 在 Stage 0 **不是** no-op（外部持有的 GcRef 必须作为根保存，否则悬垂）——两个接口的成熟度分级不同，见 [13-能力矩阵 §3.1.3](./13-capability-matrix.md)。

## 4. 标记-清除 GC 实现框架（原 §19.4）

> **实现框架系列说明（原 §19 章导言）**：本系列（[02-语法模型 §6](./02-syntax-model.md) / [03-宏系统 §4](./03-macro-system.md) / [04-字节码 VM §2-§3](./04-bytecode-vm.md) / 本文 §4）将 [13-能力矩阵 §2](./13-capability-matrix.md) 定义的 12 个能力模型落实为可直接照抄实现的伪代码框架。所有伪代码采用 Rust 风格语法，但刻意停留在"控制流 + 数据流"层面，不绑定具体内存布局——同一框架可无损翻译为 OCaml（推荐宿主）或 C（VM 基座）。每节按「算法骨架 → 核心不变式 → 实现陷阱」组织：不变式是测试设计的直接依据，陷阱清单来自历史实现（Guix 自举链、rustc、Racket BC）踩过的坑。

Bump-pointer 分配 + 递归标记 + 堆遍历清除（能力模型见本文 §2，内存管理策略见本文 §3）。根集包括 VM 栈、调用栈局部变量、全局环境和 C 栈显式根。

**三阶段骨架**：

```rust
/// 分配：bump-pointer，分配失败时触发回收
fn alloc(heap: &mut Heap, size: usize, roots: &RootSet) -> *mut ObjHeader {
    if let Some(p) = heap.bump_alloc(size) { return p; }
    gc_cycle(heap, roots);                    // 回收一轮
    if let Some(p) = heap.bump_alloc(size) { return p; }
    heap.grow(size);                          // 仍不足则扩堆
    heap.bump_alloc(size).expect("OOM")
}

fn gc_cycle(heap: &mut Heap, roots: &RootSet) {
    mark_all(roots, heap);   // 1. 标记：从根集可达的全部对象置位
    sweep(heap);             // 2. 清除：未标记对象归还 free list
    heap.clear_marks();      // 3. 复位：为下一轮准备
}

fn mark_all(roots: &RootSet, heap: &mut Heap) {
    let mut work: Vec<GcRef> = roots.collect();       // 显式工作栈（防递归爆栈）
    while let Some(obj) = work.pop() {
        if heap.test_and_set_mark(obj) { continue; }  // 已标记则跳过
        for child in heap.children(obj) { work.push(child); }
    }
}

fn sweep(heap: &mut Heap) {
    for obj in heap.iter_objects() {                  // 线性遍历堆
        if !heap.is_marked(obj) { heap.free(obj); }   // 归还 free list（保守策略：泄漏容忍，误回收零容忍）
    }
}
```

**核心不变式**：
1. **根集完备性**：VM 数据栈、调用栈局部、全局环境、`register_foreign_ref` 登记的外部引用（[13-能力矩阵 §4](./13-capability-matrix.md) FFI 预留）——四类根缺一不可，漏根 = 悬垂指针
2. **标记位复位对称**：`clear_marks` 必须遍历全部对象（含刚回收的），否则下一轮标记会继承脏状态
3. **分配零内卷**：bump 分配路径上不得调用任何可能触发分配的函数（含日志），否则递归触发 GC

**实现陷阱**：
- **递归标记的爆栈风险**：`mark(obj)` 递归深度 = 最长引用链。一个 10^7 元素的嵌套列表会爆 C 栈——因此骨架中用显式工作栈替代递归（这正是 jonesforth 与早期 MIT Lisp 都踩过的坑）
- **VM 执行中途的安全点**：GC 只能在指令边界触发（CALL / JUMP 等分派点检查分配配额），操作数栈中途的状态必须可枚举为根
- **bump 分配器与 free list 的共存**：sweep 归还的空间先记入 free list；Stage 0 允许"只分配不压缩"，碎片治理推迟到 Stage 2 分代 GC

## 5. 测试锚点（设计驱动测试，测试验证设计）

| 契约/不变式 | 测试锚点（tests/v0/stage0/ + examples/） | 验证命题 |
|-----------|------------------------------------------|---------|
| 根集完备性（§3.2 四来源） | gc_stress（深递归 + 分配计数驱动触发） + IO 外部引用存活 | 漏根 = 悬垂检测 |
| 标记位复位对称（§4 不变式 2） | 多轮回收周期单测（free_count 对账） | 复位无脏状态 |
| GC 不可观测（[06-操作语义 §4](./06-operational-semantics.md) L-GC） | 双执行路径互查（含堆效应程序） | 归约结果与触发点无关 |
| 冷却退避性能（§3.1） | benchmarks：gc_stress 运行时对账（122s → 3.4s 修复基线） | O(n²) 消退 |
| I/O 通道（§1 契约） | examples/io（read_line/write_line 回环） + RunOutcome 值存活验证 | 错误显式返回 |
| 外部引用登记（§3.1） | heap 单测：register_foreign_ref 后回收存活 | FFI 预留语义 |

> 测试矩阵完整定义见 [11-测试基础设施 §3](./11-testing.md)；本表是其运行时侧子集。回收器观测经 `GcStats`（`heap.stats()`）暴露——性能数据是一等公民（[17-设计原则 §1 原则 16](./17-principles.md)）。
