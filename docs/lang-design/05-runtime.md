# 运行时：最小 I/O 与标记-清除 GC

> **Author**: kerf-doc-agent
> **Date**: 2026-09-09
> **Version**: v5.0（源自 stage0.md v5.0 拆分）
> **Status**: Active

> 本文件收录运行时基座的设计与实现：最小 I/O（原 §8.8）、标记-清除 GC 的最小实现（原 §8.11）、内存管理策略（原 §14.2：分配器接口协议、屏障接口与 FFI 外部引用追踪），以及标记-清除 GC 实现框架（原 §19.4：三阶段骨架 + 核心不变式 + 实现陷阱）。字节码 VM（运行时基座的另一半）见 [04-字节码 VM](./04-bytecode-vm.md)；最小内置库的函数级边界见 [09-标准库](./09-stdlib.md)；12 个能力模型的完整矩阵见 [13-能力矩阵](./13-capability-matrix.md)；高级 GC 的推迟理由见同文件 §3。

---

## 1. 最小 I/O（传统）（原 §8.8）

仅 `read_line()` 和 `write_line()`，通过全局函数（非能力模型）。Stage 0 不引入能力模型 I/O，但 VM 栈帧和分配器接口必须预留 `register_foreign_ref` 等接口（Stage 0 可为 no-op），以便 Stage 1+ 升级到能力模型时无需破坏接口。

## 2. 标记-清除 GC（最小实现）（原 §8.11）

Bump-pointer 分配 + 递归标记 + 堆遍历清除。根集包括 VM 栈、调用栈局部变量、全局环境和 C 栈显式根。Stage 0 用最简单的 mark-sweep，避免分代/增量/并发的复杂度。

**实现指引**：分配、标记、清除三个阶段的完整伪代码与栈溢出对策见本文 §4。

## 3. 内存管理策略（原 §14.2）

分配器接口协议包含基础分配、屏障接口（Stage 0 为 no-op）、根集管理和 FFI 外部引用追踪。堆对象统一头格式支持从保守 GC 演进到精确/分代/增量 GC。

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
