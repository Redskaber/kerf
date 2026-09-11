//! GC 集成测试（tests/v0/stage0/plan/gc_tests.rs ↔ docs/tests/v0/stage0/plan/gc.md）。
//!
//! 覆盖：标记-清除回收（§21.3 Stage 0 验收）/ 10^6 临时对象堆稳定 /
//! 存活数据保护 / foreign ref 根 / 统计可观测。
//! （环引用与深链防爆栈由 kerf-runtime 单元测试覆盖——见 crate 内联测试。）

use crate::common;

use kerf_runtime::{mark_sweep_cycle, GcRef, Heap, RootSet};

/// 分配 10^6 量级临时对象堆稳定（§21.3 Stage 0 验收项——循环丢弃分配）。
#[test]
fn million_temp_objects_heap_stays_bounded() {
    // 每层 20 次序对分配（3 槽/次）× 3×10^4 层 ≈ 1.8×10^6 临时分配
    // （递归深度 3×10^4 < 帧上限 10^5；深度与分配量解耦）
    let src = r#"
        (define (spin n)
          (if (= n 0)
              0
              (begin
                (cons 1 2) (cons 3 4) (cons 5 6) (cons 7 8) (cons 9 10)
                (cons 11 12) (cons 13 14) (cons 15 16) (cons 17 18) (cons 19 20)
                (cons 21 22) (cons 23 24) (cons 25 26) (cons 27 28) (cons 29 30)
                (cons 31 32) (cons 33 34) (cons 35 36) (cons 37 38) (cons 39 40)
                (spin (- n 1)))))
        (spin 30000)
    "#;
    let outcome = common::run_with_heap(src).unwrap_or_else(|e| panic!("执行失败：{}", e));
    let stats = outcome.heap.stats();
    assert!(stats.collections > 0, "至少发生一次 GC");
    // 堆槽位应远低于总分配量（≈1.5M 槽 → 回收后有界）
    assert!(
        stats.slots < 200_000,
        "堆未保持有界：slots = {}（collections = {}）",
        stats.slots,
        stats.collections
    );
}

/// 存活数据不被误回收（保守策略：泄漏容忍、误回收零容忍）。
#[test]
fn live_data_survives_collections() {
    let src = r#"
        (define keep (quote (1 2 3 4 5)))
        (define (spin n)
          (if (= n 0) 0 (begin (cons 9 9) (cons 9 9) (cons 9 9) (cons 9 9) (cons 9 9) (spin (- n 1)))))
        (spin 50000)
        (car (cdr (cdr (cdr (cdr keep)))))
    "#;
    common::assert_int(src, 5);
}

/// 闭包捕获的数据跨 GC 存活（根集完备性：帧捕获单元）。
#[test]
fn closure_captured_data_survives_gc() {
    let src = r#"
        (define (make-holder v) (lambda () v))
        (define h (make-holder (quote (7 8))))
        (define (spin n)
          (if (= n 0) 0 (begin (cons 1 1) (cons 1 1) (cons 1 1) (cons 1 1) (cons 1 1) (spin (- n 1)))))
        (spin 50000)
        (car (h))
    "#;
    common::assert_int(src, 7);
}

/// foreign ref 登记的根存活（§8.8 预留接口）。
#[test]
fn foreign_refs_survive_cycle() {
    let mut heap = Heap::new();
    let r = heap.alloc_int(42);
    let garbage = heap.alloc_pair(r, r); // 制造不可达伴随垃圾
    let _ = garbage;
    heap.register_foreign_ref(r);
    let stats = mark_sweep_cycle(&mut heap, &RootSet::from_refs([]));
    assert_eq!(stats.live, 1, "foreign 根存活、垃圾回收");
    assert_eq!(stats.free, 1);
}

/// 可达对象链存活、不可达回收。
#[test]
fn reachability_partition() {
    let mut heap = Heap::new();
    // 链：(1 . (2 . nil))——从 head 可达 3 槽
    let nil = heap.alloc_nil();
    let one = heap.alloc_int(1);
    let two = heap.alloc_int(2);
    let inner = heap.alloc_pair(two, nil);
    let head = heap.alloc_pair(one, inner);
    // 孤儿
    let orphan = heap.alloc_int(99);
    let _ = orphan;
    let stats = mark_sweep_cycle(&mut heap, &RootSet::from_refs([head]));
    assert_eq!(stats.live, 5, "链 5 槽存活（head/inner/one/two/nil）");
    assert_eq!(stats.free, 1, "孤儿回收");
}

/// GC 统计可观测（性能基准数据源）。
#[test]
fn gc_stats_observable() {
    let src = "(define keep 1) (cons 1 2) (cons 3 4)";
    let outcome = common::run_with_heap(src).unwrap();
    let stats = outcome.heap.stats();
    assert!(stats.total_allocs >= 4);
    assert_eq!(stats.slots, stats.live + stats.free);
    let _ = GcRef(0); // 句柄可构造（仅库内使用——公共类型形状）
}

/// TD-010（r24）：装箱闭包（Foreign 槽位）的捕获数据跨 GC 存活——
/// 标记阶段经追踪器枚举闭包捕获图（HeapObj::Foreign children）。
/// 闭包装箱进序对后，其捕获的 Pair 仅经 Foreign 槽位可达——追踪器
/// 缺失将导致误回收（use-after-free 级缺陷）。
#[test]
fn boxed_closure_captures_survive_gc() {
    let src = r#"
        (define (make-box) (let ((cell (cons 42 nil))) (lambda () (car cell))))
        (define holder (cons (make-box) nil))
        (define (spin n)
          (if (= n 0) 0 (begin (cons 1 1) (cons 1 1) (cons 1 1) (cons 1 1) (cons 1 1) (spin (- n 1)))))
        (spin 50000)
        ((car holder))
    "#;
    let outcome = common::run_with_heap(src).unwrap_or_else(|e| panic!("执行失败：{}", e));
    let stats = outcome.heap.stats();
    assert!(stats.collections > 0, "压力期发生 GC（追踪器被实际行使）");
}

/// TD-010（r24）：装箱闭包捕获链的多级传递可达性——holder 全局序对
/// 根 → Foreign 槽 → 追踪器 → 捕获 Pair 链（cons 40 (cons 2 nil)，
/// 标记阶段经 Pair children 续展）。
#[test]
fn boxed_closure_trace_transitive_pairs() {
    let src = r#"
        (define (make-get) (let ((p (cons 40 (cons 2 nil)))) (lambda (x) (+ x (car p)))))
        (define holder (list (make-get)))
        (define (spin n)
          (if (= n 0) 0 (begin (cons 2 2) (cons 2 2) (cons 2 2) (cons 2 2) (cons 2 2) (spin (- n 1)))))
        (spin 50000)
        ((car holder) 0)
    "#;
    common::assert_int(src, 40);
}

/// TD-023（r24）：GcCell 堆根性摘要的**写路径 sound 不变式**——set!
/// 将 Pair 写入捕获单元后标志必须翻转（否则该 Pair 仅经闭包捕获链
/// 可达，根扫描按标志跳过 → 误回收 → 悬垂读）。box 初值 nil（flag
/// false）→ init 写 Pair（flag true）→ GC 压力 → 读回 42。
#[test]
fn gc_cell_flag_flips_on_pair_write() {
    let src = r#"
        (define (make)
          (let ((box nil))
            (lambda (cmd)
              (if (eq? cmd (quote init))
                  (set! box (list 42))
                  (car box)))))
        (define f (make))
        (f (quote init))
        (define (spin n)
          (if (= n 0) 0 (begin (cons 1 2) (cons 3 4) (spin (- n 1)))))
        (spin 50000)
        (f (quote read))
    "#;
    common::assert_int(src, 42);
}
