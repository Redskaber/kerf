# 运行时：最小 I/O 与标记-清除 GC

> **Author**: kerf-doc-agent
> **Date**: 2026-09-12（v6.3：K2/r36 大阶段末深审回写——§14.8 B2-2/B2-5：根集五来源 → 六来源（continuation 帧链 r25/M5 第六源，与 06 §1.2 对齐）+ 类型化分配八入口 → 九入口（alloc_foreign r30 FFI）+ 分代/压缩推迟项口径对齐 TD-008 r24 DEFER Stage 3+；2026-09-11（v6.2：批次 F 深审回写——TD-002 符号值家族对齐：HeapObj 七变体/八入口 + I/O 通道三函数 + gc_stress 回归基线注记；v5.2：HeapObj 六变体对齐 + alloc_boxed 第 7 入口 + 根集五来源 + GC 冷却数值冻结 + 分配算法描述对齐（slot-Vec + 空闲表 + 显式工作栈））
> **Version**: v6.3
> **Status**: Active
> **处理程度**：GC 为 P0（安全关键组件，Stage 0 已实现）；最小 I/O 为 P1（传统全局函数，能力模型 I/O 接口预留）｜ **所属 Stage**：Stage 0 ｜ **推迟项**：分代/增量/并发 GC（TD-008 r24 裁定 DEFER Stage 3+ 条件触发）、能力模型 I/O net/process（Stage 3 触发式，[13-能力矩阵 §3.1.3](./13-capability-matrix.md)）、屏障接口实现（Stage 0 no-op）

> 本文件收录运行时基座的设计与实现：最小 I/O（原 §8.8）、标记-清除 GC 的最小实现（原 §8.11）、内存管理策略（原 §14.2，v5.1 从冻结实现回填分配器/堆/根集契约），以及标记-清除 GC 实现框架（原 §19.4：三阶段骨架 + 核心不变式 + 实现陷阱）。字节码 VM（运行时基座的另一半）见 [04-字节码 VM](./04-bytecode-vm.md)；最小内置库的函数级边界见 [09-标准库](./09-stdlib.md)；12 个能力模型的完整矩阵见 [13-能力矩阵](./13-capability-matrix.md)（其 §2.8/§2.11 为本文件 §1/§2 的规范副本）；高级 GC 的推迟理由见同文件 §3。操作语义侧的 GC 不可观测性引理（L-GC）见 [06-操作语义 §4](./06-operational-semantics.md)。

---

## 1. 最小 I/O（传统）（原 §8.8）

仅 `read_line()` 和 `write_line()`，通过全局函数（非能力模型）。Stage 0 不引入能力模型 I/O，但 VM 栈帧和分配器接口必须预留 `register_foreign_ref` 等接口（Stage 0 可为 no-op），以便 Stage 1+ 升级到能力模型时无需破坏接口。

> **v6.2 通道面注记**：通道层实为**三函数**——r5 标准库批次新增 `write_stdout(s)`（`write-string` 内置的无换行通道载体，io.rs 注记在案；09-stdlib v5.4 已记载该通道名，本文件此前未同步）。升级路径不受影响（三函数同被能力对象分发覆盖）。

**I/O 通道契约（Stage 0 冻结，kerf-runtime/src/io.rs）**：

```rust
/// 运行时错误（错误显式返回，禁止异常——接口契约规则 2）。
pub struct RuntimeError { pub message: String }

/// 全局 stdout 行写入（副作用唯一出口之一）。
pub fn write_line_stdout(s: &str) -> Result<(), RuntimeError>;
/// 全局 stdin 行读取（EOF 返回 None；副作用唯一出口之二）。
pub fn read_line_stdin() -> Result<Option<String>, RuntimeError>;
/// 全局 stdout 原文写入（无换行——`write-string` 的通道层载体，r5 新增；
/// v6.2 补注第 3 通道函数）。
pub fn write_stdout(s: &str) -> Result<(), RuntimeError>;
```

**升级路径（扩展预留接口）**：Stage 2 升级到能力模型 I/O 时，本二函数由 `ReadCapability / WriteCapability` trait（[13-能力矩阵 §3.1.3](./13-capability-matrix.md) 冻结签名）的实例替换——内置函数 δ 表（[09-标准库 §2](./09-stdlib.md)）从直接调用全局函数改为经能力对象分发，调用点接口不变（[17-设计原则 §1 原则 28](./17-principles.md)：渐进替换）。

## 2. 标记-清除 GC（最小实现）（原 §8.11）

**v5.2 算法描述对齐（原「bump-pointer 分配 + 递归标记」摘要失真，以下为实现即事实）**：**slot-Vec 分配 + 空闲表（free 表）+ 显式工作栈标记 + 堆线性遍历清除**。堆为 `Vec<Slot>`（槽位数组，`GcRef = u32` 索引句柄），分配优先复用空闲表中的槽位、耗尽后追加新槽；标记阶段用显式 `work` 栈替代递归（防爆栈）；清除阶段线性遍历堆重建空闲表（不移动对象、不压缩——碎片治理推迟至分代/压缩，TD-008 r24 裁定 DEFER Stage 3+）。根集包括（**六来源**）：VM 数据栈、全局环境、调用帧局部槽、**帧捕获槽（Rc<RefCell> 共享单元格）**、`register_foreign_ref` 登记的外部引用、**活跃 continuation 帧链**（r25/M5 第六源——vm.rs:1368）。Stage 0 用最简单的 mark-sweep，避免分代/增量/并发的复杂度。

**实现指引**：分配、标记、清除三个阶段的完整伪代码与栈溢出对策见本文 §4。

## 3. 内存管理策略（原 §14.2，v5.1 契约回填）

分配器接口协议包含基础分配、屏障接口（Stage 0 为 no-op）、根集管理和 FFI 外部引用追踪。堆对象统一头格式支持从保守 GC 演进到精确/分代/增量 GC。

### 3.1 堆与分配器契约（Stage 0 冻结，kerf-runtime/src/heap.rs）

```rust
/// GC 引用（堆槽句柄——u32 索引，非裸指针：句柄稳定性不依赖内存布局）。
pub struct GcRef(pub u32);

/// 堆对象种类（**七变体**，v6.2 对齐冻结实现：Pair/Str/Int/Float/Bool/Nil/Symbol。
/// Symbol(Rc<str>) 为 r5 批次 B TD-002 符号值的槽位形态（v6.2 前文档停在六变体口径）。
/// 早期版本误写 Boxed(BoxedInput) 变体——实际不存在：闭包/内置函数不可装箱
/// 是显式裁定（序对元素为闭包/内置时以标记字符串占位，TD-010 登记，
/// Stage 2 以 HeapObj::Foreign 承载——目标时机随批次 F 深审改判，见登记册）。
/// **r30/48-d 注记**：`HeapObj::Foreign` 载荷扩展承载 **FFI 外部令牌**
/// （`Rc<ExternalToken>`——ffi-ownership-model §7「令牌载荷不参与 GC
/// 可达性」：追踪器 no-op、children 为空；闭包/内置/continuation/令牌
/// 四形态共用 Foreign 变体——TD-010 追踪器协议的自然扩展）。
pub enum HeapObj { /* Pair(GcRef, GcRef) | Str(Rc<str>) | Int(i64) | Float(f64)
                     | Bool(bool) | Nil | Symbol(Rc<str>) */ }

/// 堆槽（分配粒度单位：对象载荷 + 标记位；空闲表为堆级 `free: Vec<GcRef>`，
/// 不在槽内——清除阶段全量重建）。
pub struct Slot { /* obj: HeapObj; marked: bool */ }

/// 堆（分配器 + 标记-清除回收器 + 外部引用登记簿）。
pub struct Heap {
    // slots: Vec<Slot>; free: Vec<GcRef>; foreign_roots: Vec<GcRef>;
    // pin_counts: Vec<(GcRef, u32)>;  // Φ 计数簿（r30/48-d——foreign_roots
    //                                 // 的计数化泛化：supp(Φ) 并入 mark
    //                                 // 起点；归零摘根——ffi-ownership-model §3）
    // allocs_since_gc: u64; gc_threshold: usize; gc_enabled: bool;
}

impl Heap {
    /// 类型化分配（**九入口**——调用方携带类型信息，非统一 alloc(size)）：
    /// 八个类型化入口 + alloc_boxed（即时值统一描述入口，按 BoxedInput
    /// 分派到 Str/Int/Float/Bool/Nil 五类——符号走 alloc_symbol 直通，
    /// 不在 BoxedInput 内——v6.2 对齐 r5 实现；v6.3 增补：alloc_foreign
    /// 为第 9 入口——r30/48-d FFI ForeignBox 装箱，追踪器协议入标记图）。
    pub fn alloc_pair(&mut self, car: GcRef, cdr: GcRef) -> GcRef;
    pub fn alloc_str(&mut self, s: Rc<str>) -> GcRef;
    pub fn alloc_int(&mut self, v: i64) -> GcRef;
    pub fn alloc_float(&mut self, v: f64) -> GcRef;
    pub fn alloc_bool(&mut self, v: bool) -> GcRef;
    pub fn alloc_nil(&mut self) -> GcRef;
    /// 符号分配（v6.2 补注第 8 入口——TD-002 r5：quote 符号值入堆）。
    pub fn alloc_symbol(&mut self, s: Rc<str>) -> GcRef;
    pub fn alloc_boxed(&mut self, v: BoxedInput) -> GcRef;

    /// 读取访问（不可变查询：unbox/get_pair/get）。
    pub fn unbox(&self, r: GcRef) -> Option<ValueSlot>;
    pub fn get_pair(&self, r: GcRef) -> Option<(GcRef, GcRef)>;

    /// GC 控制（分配计数驱动触发；冷却退避状态在 **VM 侧**（vm.rs 的
    /// gc_cooldown 计数器），不在 Heap——Heap 只暴露 should_collect 与阈值）。
    pub fn set_gc_enabled(&mut self, enabled: bool);
    /// 触发判定：`allocs_since_gc > gc_threshold`（**严格大于**——
    /// 等于间值当轮不触发，与文档早期「≥」写法不同，以实现为准）。
    pub fn should_collect(&self) -> bool;

    /// 外部引用登记（FFI 预留接口——[13-能力矩阵 §4](./13-capability-matrix.md) 决策 3）。
    /// Stage 0 语义：登记的引用作为根保存（非 no-op——正确性优先，
    /// mark_sweep_cycle 起点并入 foreign_roots）；
    /// 屏障接口（write barrier）才是 Stage 0 no-op 项。
    pub fn register_foreign_ref(&mut self, r: GcRef);
    pub fn foreign_roots(&self) -> &[GcRef];

    /// Φ 计数簿（r30/48-d——[ffi-ownership-model §3](../develop/v0/stage-2/ffi-ownership-model.md)
    /// P/U 归约规则的运行形态；`register_foreign_ref` 的**计数化泛化**——
    /// 持久根与窗口计数根并存，supp(Φ) 并入 mark 起点）：
    /// - `pin_object`（P1）：`Φ[v] += 1`（可重复累计——引计数式屏障）；
    /// - `unpin_object`（U1）：`Φ[v] -= 1` 归零摘根（下一轮回收周期
    ///   可回收）；U2 下溢 → `Err`（E8 口径——结构配对保证不可达）；
    /// - `pin_count`/`pinned_refs`：Φ 可观测性（测试/审计面）；
    /// - F-PIN 引理：周期起点 Φ(v) ≥ 1 ⟹ 本轮 sweep 不回收 v。
    pub fn pin_object(&mut self, r: GcRef);
    pub fn unpin_object(&mut self, r: GcRef) -> Result<(), RuntimeError>;
    pub fn pin_count(&self, r: GcRef) -> u32;
    pub fn pinned_refs(&self) -> Vec<GcRef>;

    /// 统计（基准与测试观测点）。
    pub fn stats(&self) -> GcStats;
}
```

**堆对象统一头格式（Slot 结构的语义）**：每个槽携带「对象载荷 + 标记位」。从保守 GC 演进到精确/分代/增量 GC 时：(1) 标记位可直接升级为分代号（低 1-2 位复用）；(2) 堆级空闲表（`free: Vec<GcRef>`）可升级为分代空闲链（v5.2 注：空闲表在堆级而非槽内 `next_free` 指针——以实现为准）；(3) `GcRef` 句柄格式不变——「数据结构的留白是廉价的」（[17-设计原则 §1 原则 20](./17-principles.md)）。

**分配触发协议（v5.2 数值冻结）**：分配计数驱动（`allocs_since_gc > gc_threshold`，**默认阈值 gc_threshold = 1024**，测试可调 `Heap::with_threshold`）+ **GC 冷却退避**（归属 **VM 侧**执行循环：安全点轮询间隔 `GC_POLL_INTERVAL = 256` 条指令；回收收益 < 25%（`total_freed × 4 < slots`）时置冷却计数 `GC_COOLDOWN_MULTIPLIER = 64` 个轮询周期；深递归场景连续触发回收的 O(n²) 消退——worklog Task 4-c 的关键性能修复：122s → 3.4s）。退避仅影响触发时机，不影响语义（[06-操作语义 §4 引理 L-GC](./06-operational-semantics.md)：GC 不可观测）。

> **v6.2 回归基线注记（TD-023）**：批次 F 深审实测 gc_stress（3×10^5 分配 + 30k 深递归）单轮 207~235ms，较 Stage 0 基线 160.4ms 回归 +29~46%（超 §14.6.4 的 10% 阈值），且超线性缩放（spin 15000→59.5ms：2× 分配 → 3.6× 耗时）——冷却四参数未变，候选根因为根集遍历 per-cycle `HashSet<usize>` 去重分配与 Value 枚举宽度增长的缓存效应。登记 TD-023（P2），绑定 Stage 2 批次 I2（与 TD-008 分代/压缩评估同轮——对症路径）；完整口径见 [stage-1/performance-baseline](../develop/v0/stage-1/performance-baseline.md)。

**运行时错误与堆栈追踪（v5.2 补注）**：VM 路径运行时错误在 `run_program` 错误出口统一生成调用点追踪（`VmError.trace`：帧链快照的**最内 16 帧**（`MAX_TRACE_FRAMES = 16`——深递归下外层无信息量，防诊断爆炸），经 debug_info 反查 Span 后渲染为 note 行）；eval 路径错误经 driver 包装为 E0004 诊断——两路径错误事实一致，消息文本存在已存档的分裂面（TD-014 相关，[06-操作语义 §5.3](./06-operational-semantics.md)）。

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

**根集六来源（根集完备性不变式，本文 §4；v6.3 修订——r25/M5 增第六源 continuation 帧链，原 v5.2 增帧捕获槽）**：(1) VM 数据栈；(2) 调用帧局部槽；(3) **帧捕获槽**（`Rc<RefCell<Value>>` 共享单元格——可变捕获的根，VM 侧 `collect_roots` 逐帧枚举 locals 与 captures 两组）；(4) 全局环境；(5) `register_foreign_ref` 登记的外部引用（回收周期起点并入标记工作栈；r30/48-d 起 = 持久根 **∪ supp(Φ)**——pin 计数簿（P/U 规则）为第五来源的计数化泛化，F-PIN 引理维持完备性不变式）；(6) **活跃 continuation 帧链**（r25/M5——`Value::Continuation` 四要素含帧链/数据栈快照，vm.rs:1368 部署跨压力存活锚 effect_tests gc_continuation_boxed）。六类根缺一不可——漏根 = 悬垂指针（P0 级安全缺陷）。

**屏障接口的边界裁定**：Stage 0 的屏障为 no-op（无分代/并发，无屏障需求）；`register_foreign_ref` 在 Stage 0 **不是** no-op（外部持有的 GcRef 必须作为根保存，否则悬垂）——两个接口的成熟度分级不同，见 [13-能力矩阵 §3.1.3](./13-capability-matrix.md)。

## 4. 标记-清除 GC 实现框架（原 §19.4）

> **实现框架系列说明（原 §19 章导言）**：本系列（[02-语法模型 §6](./02-syntax-model.md) / [03-宏系统 §4](./03-macro-system.md) / [04-字节码 VM §2-§3](./04-bytecode-vm.md) / 本文 §4）将 [13-能力矩阵 §2](./13-capability-matrix.md) 定义的 12 个能力模型落实为可直接照抄实现的伪代码框架。所有伪代码采用 Rust 风格语法，但刻意停留在"控制流 + 数据流"层面，不绑定具体内存布局——同一框架可无损翻译为 OCaml（推荐宿主）或 C（VM 基座）。每节按「算法骨架 → 核心不变式 → 实现陷阱」组织：不变式是测试设计的直接依据，陷阱清单来自历史实现（Guix 自举链、rustc、Racket BC）踩过的坑。

slot-Vec 分配 + 显式工作栈标记 + 堆线性遍历清除（能力模型见本文 §2，内存管理策略见本文 §3；v5.2 对齐冻结实现——原 bump-pointer 描述是早期设计草案，非实现事实）。根集六来源：VM 数据栈、调用帧局部槽、帧捕获槽（共享单元格）、全局环境、`register_foreign_ref` 登记的外部引用、活跃 continuation 帧链（r25/M5——本文 §3.2）。

**三阶段骨架**：

```rust
/// 分配：槽位表优先复用空闲表，耗尽则追加新槽（句柄稳定——u32 索引）
fn alloc<T>(heap: &mut Heap, make: impl Fn() -> HeapObj) -> GcRef {
    if let Some(r) = heap.free.pop() {          // 空闲表复用（后进先出）
        heap.slots[r.0 as usize] = Slot { obj: make(), marked: false };
        return r;
    }
    heap.slots.push(Slot { obj: make(), marked: false });  // 追加新槽
    GcRef(heap.slots.len() as u32 - 1)
}

fn gc_cycle(heap: &mut Heap, roots: &RootSet) {
    mark_all(roots, heap);   // 1. 标记：从根集可达的全部对象置位
    sweep(heap);             // 2. 清除：未标记对象归还 free 表（全量重建）
    heap.reset_alloc_counter(); // 3. 复位：分配计数归零，为下一轮摊销窗口
}

fn mark_all(roots: &RootSet, heap: &mut Heap) {
    let mut work: Vec<GcRef> = roots.collect();       // 显式工作栈（防递归爆栈）
    work.extend(heap.foreign_roots());               // 外部登记引用并入起点
    while let Some(obj) = work.pop() {
        if heap.test_and_set_mark(obj) { continue; }  // 已标记则跳过
        for child in heap.children(obj) { work.push(child); }
    }
}

fn sweep(heap: &mut Heap) {
    let mut new_free: Vec<GcRef> = Vec::new();
    for (i, slot) in heap.slots.iter().enumerate() {  // 线性遍历堆
        if !slot.marked {
            new_free.push(GcRef(i as u32));           // 归还 free 表（保守策略：泄漏容忍，误回收零容忍）
        }
    }
    heap.free = new_free;                             // 空闲表全量重建
}
```

**核心不变式**：
1. **根集完备性**：VM 数据栈、调用帧局部槽、**帧捕获槽（共享单元格）**、全局环境、`register_foreign_ref` 登记的外部引用（[13-能力矩阵 §4](./13-capability-matrix.md) FFI 预留）、**活跃 continuation 帧链（r25/M5）**——**六类根**缺一不可，漏根 = 悬垂指针（v5.2 修订：原「四类」漏帧捕获槽；v6.3 修订：r25 增第六源）
2. **标记位复位对称**：清除阶段重建空闲表后，存活槽的标记位与被回收槽的复用状态必须一致复位（新分配槽 `marked: false` 起步），否则下一轮标记会继承脏状态
3. **分配零内卷**：分配路径上不得调用任何可能触发回收的函数（含日志），否则递归触发 GC

**实现陷阱**：
- **递归标记的爆栈风险**：`mark(obj)` 递归深度 = 最长引用链。一个 10^7 元素的嵌套列表会爆 C 栈——因此骨架中用显式工作栈替代递归（这正是 jonesforth 与早期 MIT Lisp 都踩过的坑）
- **VM 执行中途的安全点**：GC 只能在指令边界触发（每 `GC_POLL_INTERVAL = 256` 条指令轮询分配配额），操作数栈中途的状态必须可枚举为根
- **碎片不治理的代价边界**：slot-Vec + 空闲表方案不移动对象（句柄稳定性）；Stage 0 允许「只分配不压缩」，碎片治理推迟到 Stage 2 分代/压缩 GC（TD-008）

## 5. 测试锚点（设计驱动测试，测试验证设计）

| 契约/不变式 | 测试锚点（tests/v0/stage0/ + examples/） | 验证命题 |
|-----------|------------------------------------------|---------|
| 根集完备性（§3.2 六来源） | gc_tests（深递归 + 分配计数驱动触发 + 环/深链/foreign） + examples/usage/gc_stress.krf + effect_tests gc_continuation_boxed（第六源） | 漏根 = 悬垂检测 |
| 标记位复位对称（§4 不变式 2） | 多轮回收周期单测（free 表对账） | 复位无脏状态 |
| GC 不可观测（[06-操作语义 §4](./06-operational-semantics.md) L-GC） | 双执行路径互查（含堆效应程序，集成 164 函数内置） | 归约结果与触发点无关 |
| 冷却退避性能（§3.1） | gc_tests + CLI `kerf bench examples/usage/gc_stress.krf`（122s → 3.4s 修复基线；当前单轮口径 ~0.17s / 3×10^5 分配） | O(n²) 消退 |
| I/O 通道（§1 契约） | examples/usage/io.krf（print/read-line/str-append 回环） + RunOutcome 值存活验证 | 错误显式返回 |
| 外部引用登记（§3.1） | heap 单测：register_foreign_ref 后回收存活 | FFI 预留语义 |
| 堆栈追踪（§3.1 补注） | negative_semantics_tests::message_shape_call_site_trace（note 行帧链断言）+ vm_tests 双路径错误恢复 | 错误携带调用点 |

> 测试矩阵完整定义见 [11-测试基础设施 §3](./11-testing.md)；本表是其运行时侧子集。回收器观测经 `GcStats`（`heap.stats()`）暴露——性能数据是一等公民（[17-设计原则 §1 原则 16](./17-principles.md)）。
