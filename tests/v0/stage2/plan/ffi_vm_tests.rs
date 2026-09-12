//! FFI VM 面集成测试（r30/48-d——批次 J 批 48-d J3 验收）。
//!
//! **验收合同**（plan.md §5b 48-d 行 + ffi-ownership-model §10）：
//! - §6 边界 **13 case 全判定落地**（覆盖映射表见下）；
//! - 正例端到端 ≥1（write_stdout char* 路径经冻结 FfiCall →
//!   lowering → 操作码 → 窗口规程 → 宿主真实 I/O）；
//! - 负例组正负比 ≥1:3（§9.4.3——功能点粒度：窗口规程/分配释放/
//!   令牌传递/符号解析/诊断族各 ≥1 正 + ≥3 负）；
//! - E0010/E0011/E0012 诊断族落位（18 §6 码位登记 r18 预留兑现）。
//!
//! **13 case 覆盖映射**（ffi-ownership-model §6）：
//!
//! | case | 判定 | 测试锚 |
//! |------|------|--------|
//! | 1 pin 后 GC 运行 | F-PIN 存活保证 | `f_pin_case1_survives_gc` |
//! | 2 pin 期间 set! | 允许（不冻结内容） | `pin_window_allows_binding_mutation_case2` |
//! | 3 外部回调越过窗口 | Stage 2 无回调通路 | `negative_cstruct_cfunction_registration_out_of_subset`（CFunction 形状拒绝）|
//! | 4 AllocExternal size 0 | 编译期拒绝 + 运行期纵深 | `negative_alloc_size_zero_compile_rejected` + `negative_alloc_size_zero_vm_defense` |
//! | 5 双重释放 | E0010 诊断吸收非 UB | `negative_double_free_e10` |
//! | 6 CallExternal 期间 GC | 不触发（外部零 kerf 分配） | `positive_window_protocol_balanced`（total_allocs Δ=1 仅装箱）|
//! | 7 返回 Opaque | Foreign 槽承载/不可 Free | `negative_free_opaque_e11` |
//! | 8 深浅 pin | 伪问题（mark 传播） | `shallow_pin_case8_deep_reachability` |
//! | 9 错误路径 pin/令牌泄漏 | 容忍（误回收零容忍） | `negative_error_path_pin_leak_tolerated_case9` + `negative_panic_host_pin_leak_tolerated_case9` + `token_slot_gc_reclaim_leak_tolerated_case9` |
//! | 10 多线程 | Stage 2 单线程域裁定 | 结构性注记（Rc/Cell 全家 !Send——ffi.rs 头注；无行为面）|
//! | 11 令牌复制一处 Free | 消费全局生效 | `negative_token_shared_invalidation_e10` |
//! | 12 令牌/GcRef 混淆 | E0010/E0011 | `negative_free_non_token_value_e11` |
//! | 13 C 返回 NULL | Valid 空令牌 no-op | `positive_null_sentinel_free_noop` |

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kerf_compiler::{BcConst, BcProgram, Op};
use kerf_driver::ffi::{compile_ffi_call_program, default_extern_table, HeapFfiBoundary};
use kerf_driver::reserved::{CIntSize, ExternalType, FfiBoundary, FfiCall, PointeeType};
use kerf_runtime::{mark_sweep_cycle, Heap, RootSet, RuntimeError};
use kerf_span::Span;
use kerf_syntax::{Symbol, SymbolTable};
use kerf_vm::ffi::{
    call_external, free_external, BorrowedArg, ExternKind, ExternSymbolTable, ExternalToken,
    HostRet,
};
use kerf_vm::{run_program, run_program_with_externs, Value, VmError};

// ---------------------------------------------------------------------------
// 测试辅助
// ---------------------------------------------------------------------------

/// 手工字节码程序（FFI 操作码驱动面——语言面 Stage 3 前的执行形态）。
fn ffi_program(code: Vec<Op>, consts: Vec<BcConst>) -> BcProgram {
    let n = code.len();
    BcProgram {
        protos: vec![kerf_compiler::BcProto {
            name: Symbol(0),
            params: Vec::new(),
            n_locals: 0,
            capture_names: Vec::new(),
            capture_sources: Vec::new(),
            code,
            debug_spans: vec![Span::dummy(); n],
            free_vars: Vec::new(),
        }],
        consts,
        entry: 0,
        global_refs: Vec::new(),
        module_name: None,
    }
}

/// 执行（带 extern 表——FFI 注册面）。
fn run_ffi(prog: &BcProgram, table: &ExternSymbolTable) -> Result<Value, VmError> {
    let mut heap = Heap::new();
    let mut globals = HashMap::new();
    run_program_with_externs(prog, &mut globals, &mut heap, table)
}

/// SymLit 常量构造（extern 符号名——扁平空间）。
fn sym_const(name: &str) -> BcConst {
    BcConst::SymLit(Rc::from(name))
}

/// 单实参 CallExternal 程序（Str 实参——char* 窗口借用路径）。
fn call_str_program(fn_name: &str, s: &str) -> BcProgram {
    ffi_program(
        vec![
            Op::PushConst(1),
            Op::CallExternal {
                symbol: 0,
                n_args: 1,
            },
            Op::Halt,
        ],
        vec![sym_const(fn_name), BcConst::Str(Rc::from(s))],
    )
}

// 窗口观测捕获（宿主侧——thread_local 单线程域，case 10 裁定；
// 普通注释：doc 注释不适用于宏展开产物）。
thread_local! {
    static LAST_WINDOW: RefCell<String> = const { RefCell::new(String::new()) };
}

/// 捕获型宿主函数（char* 借用入 → CInt 返回）——记录窗口内可见实参形态。
fn capture_host(args: &[BorrowedArg<'_>]) -> Result<HostRet, RuntimeError> {
    let observed = format!(
        "[{}]",
        args.iter()
            .map(|a| match a {
                BorrowedArg::CharBuf(s) => format!("CharBuf({s:?})"),
                BorrowedArg::Int(v) => format!("Int({v})"),
                BorrowedArg::Token(t) =>
                    format!("Token(kind={:?},valid={})", t.kind(), t.is_valid()),
            })
            .collect::<Vec<_>>()
            .join(",")
    );
    LAST_WINDOW.with(|w| *w.borrow_mut() = observed);
    Ok(HostRet::Int(0))
}

/// 探针宿主函数（CPointer 令牌借用传递——不消费）。
fn probe_token_host(args: &[BorrowedArg<'_>]) -> Result<HostRet, RuntimeError> {
    match args {
        [BorrowedArg::Token(_)] => Ok(HostRet::Int(1)),
        _ => Err(RuntimeError::new("probe 恰接受一个令牌实参")),
    }
}

/// NULL 哨兵宿主函数（CPointer 返回——NULL 形态）。
fn make_null_host(_args: &[BorrowedArg<'_>]) -> Result<HostRet, RuntimeError> {
    Ok(HostRet::NullPtr)
}

/// Opaque 哨兵宿主函数（外部持有返回——非 kerf 释放权）。
fn make_opaque_host(_args: &[BorrowedArg<'_>]) -> Result<HostRet, RuntimeError> {
    // 静态哨兵地址（外部 API 拥有——kerf 永不释放）
    Ok(HostRet::Opaque(0x1234_0000usize as *mut u8))
}

/// 失败宿主函数（E0004 运行时通用族传播面——E7 I/O 口径）。
fn failing_host(_args: &[BorrowedArg<'_>]) -> Result<HostRet, RuntimeError> {
    Err(RuntimeError::new("外部调用失败（宿主 I/O 通道错误）"))
}

/// 恐慌宿主函数（case 9——C 侧非局部跳转同型：unwind 路径 pin 泄漏）。
fn panic_host(_args: &[BorrowedArg<'_>]) -> Result<HostRet, RuntimeError> {
    panic!("宿主外部函数非局部跳转（case 9 泄漏容忍观测）");
}

/// CInt 倍增宿主函数。
fn double_host(args: &[BorrowedArg<'_>]) -> Result<HostRet, RuntimeError> {
    match args {
        [BorrowedArg::Int(v)] => Ok(HostRet::Int(v * 2)),
        _ => Err(RuntimeError::new("double 恰接受一个 CInt 实参")),
    }
}

/// 测试表（捕获/探针/NULL/Opaque/失败/恐慌/倍增 + arity 双参形态）。
fn test_table() -> ExternSymbolTable {
    let mut t = ExternSymbolTable::new();
    t.register(
        "capture",
        vec![ExternKind::CPointer],
        ExternKind::CInt,
        capture_host,
    );
    t.register(
        "probe_token",
        vec![ExternKind::CPointer],
        ExternKind::CInt,
        probe_token_host,
    );
    t.register(
        "make_null",
        Vec::new(),
        ExternKind::CPointer,
        make_null_host,
    );
    t.register(
        "make_opaque",
        Vec::new(),
        ExternKind::Opaque,
        make_opaque_host,
    );
    t.register("failing", Vec::new(), ExternKind::CInt, failing_host);
    t.register(
        "panic_fn",
        vec![ExternKind::CPointer],
        ExternKind::CInt,
        panic_host,
    );
    t.register(
        "double",
        vec![ExternKind::CInt],
        ExternKind::CInt,
        double_host,
    );
    t.register(
        "two_params",
        vec![ExternKind::CInt, ExternKind::CInt],
        ExternKind::CInt,
        |args| match args {
            [BorrowedArg::Int(a), BorrowedArg::Int(b)] => Ok(HostRet::Int(a + b)),
            _ => Err(RuntimeError::new("two_params 恰接受两个 CInt")),
        },
    );
    t
}

// ---------------------------------------------------------------------------
// 正例（功能点粒度——窗口规程/分配释放/令牌传递/符号解析各锚）
// ---------------------------------------------------------------------------

/// **正例端到端**（验收锚：write_stdout char* 路径）——冻结
/// `FfiCall::CallExternal`（字面量实参 + return_type 静态面）→
/// `compile_ffi_call_program`（编译目标码序）→ `Op::CallExternal`
/// → 窗口规程步 2-4（Str 堆槽 pin + 数据指针借用出界 + unpin）→
/// 宿主真实 I/O（kerf_runtime::write_stdout）→ CInt 返回 = 字节数。
#[test]
fn positive_write_stdout_e2e_real() {
    let mut st = SymbolTable::new();
    let sym = st.intern("write_stdout");
    let prog = compile_ffi_call_program(
        &FfiCall::CallExternal {
            symbol: sym,
            args: vec![kerf_core::CoreExpr::Literal {
                value: kerf_core::LiteralValue::Str(Rc::from("hello ffi")),
                span: Span::dummy(),
            }],
            return_type: ExternalType::CInt(CIntSize::Usize),
        },
        &st,
    )
    .expect("字面量 FfiCall 可编译");
    let mut heap = Heap::new();
    let mut globals = HashMap::new();
    let out = run_program_with_externs(&prog, &mut globals, &mut heap, &default_extern_table())
        .expect("write_stdout 端到端执行");
    match out {
        Value::Int(n) => assert_eq!(n, 9, "CInt(Usize) 返回 = 写入字节数"),
        other => panic!("期望 Int，实际 {:?}", other),
    }
}

/// 窗口规程步 2-4 协议不变量：宿主观测 CharBuf 借用 + Φ 簿配平
/// （窗口关闭后 pinned_refs 空）+ total_allocs Δ=1（char* 装箱——
/// **case 6**：宿主调用与外部 malloc 零 kerf 分配贡献）。
#[test]
fn positive_window_protocol_balanced() {
    let table = test_table();
    let mut heap = Heap::new();
    let before = heap.stats().total_allocs;
    let v = call_external(
        &table,
        "capture",
        &[Value::Str(Rc::from("hello"))],
        &mut heap,
        Span::dummy(),
    )
    .expect("窗口规程执行");
    assert!(matches!(v, Value::Int(0)));
    let after = heap.stats().total_allocs;
    assert_eq!(
        after - before,
        1,
        "case 6：唯一 kerf 分配 = char* 装箱（宿主调用零贡献）"
    );
    assert!(
        heap.pinned_refs().is_empty(),
        "窗口关闭：Φ 簿配平（步 4 unpin 归零摘根）"
    );
    let observed = LAST_WINDOW.with(|w| w.borrow().clone());
    assert_eq!(
        observed, "[CharBuf(\"hello\")]",
        "宿主侧数据指针借用出界形态"
    );
}

/// 重复窗口配对（多次调用的 Φ 簿重复平衡——引计数式屏障无残留）。
#[test]
fn positive_multiple_calls_repeated_window() {
    let table = test_table();
    let mut heap = Heap::new();
    for _i in 0..8 {
        call_external(
            &table,
            "capture",
            &[Value::Str(Rc::from("x"))],
            &mut heap,
            Span::dummy(),
        )
        .expect("第 {i} 次窗口执行");
    }
    assert!(heap.pinned_refs().is_empty(), "8 次窗口后 Φ 簿零残留");
    assert_eq!(heap.stats().total_allocs, 8, "每调用恰一次装箱分配");
}

/// AllocExternal → FreeExternal 操作码级往返（外部域分配/释放 +
/// 消费语义）。
#[test]
fn positive_alloc_free_roundtrip() {
    let table = test_table();
    let prog = ffi_program(
        vec![Op::AllocExternal { size: 16 }, Op::FreeExternal, Op::Halt],
        Vec::new(),
    );
    let out = run_ffi(&prog, &table).expect("分配-释放往返");
    assert!(matches!(out, Value::Unit), "FreeExternal 返回 unit");
}

/// 令牌借用传递（自环迁移——不消费）：probe 观测 Token 形态 +
/// 调用后令牌仍 Valid。
#[test]
fn positive_token_borrow_pass_not_consumed() {
    let table = test_table();
    let token = ExternalToken::from_boxed_slice(vec![0u8; 8].into_boxed_slice());
    let v = call_external(
        &table,
        "probe_token",
        &[Value::External(Rc::clone(&token))],
        &mut Heap::new(),
        Span::dummy(),
    )
    .expect("令牌借用传递");
    assert!(matches!(v, Value::Int(1)));
    assert!(token.is_valid(), "借用传递不消费（状态机自环）");
}

/// **case 13**：NULL 哨兵——宿主返回 NULL → Valid 空令牌；
/// FreeExternal = 合法 no-op（对齐 free(NULL) 定义良好）。
#[test]
fn positive_null_sentinel_free_noop() {
    let table = test_table();
    let prog = ffi_program(
        vec![
            Op::CallExternal {
                symbol: 0,
                n_args: 0,
            },
            Op::FreeExternal,
            Op::Halt,
        ],
        vec![sym_const("make_null")],
    );
    let out = run_ffi(&prog, &table).expect("NULL 哨兵释放 = no-op");
    assert!(matches!(out, Value::Unit));
}

/// CInt 值拷贝路径（实参值出界 → 返回值拷贝入界）。
#[test]
fn positive_cint_roundtrip() {
    let table = test_table();
    let prog = ffi_program(
        vec![
            Op::PushConst(1),
            Op::CallExternal {
                symbol: 0,
                n_args: 1,
            },
            Op::Halt,
        ],
        vec![sym_const("double"), BcConst::Int(21)],
    );
    match run_ffi(&prog, &table).expect("CInt 往返") {
        Value::Int(v) => assert_eq!(v, 42),
        other => panic!("期望 Int，实际 {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Φ 计数簿 / P-U 归约规则（ffi-ownership-model §3——机制验证面）
// ---------------------------------------------------------------------------

/// P1：可重复 pin 计数累计（3 次 pin → 计数 3）。
#[test]
fn phi_p1_repeated_pin_counts() {
    let mut heap = Heap::new();
    let s = heap.alloc_str(Rc::from("x"));
    let p = heap.alloc_pair(s, s);
    heap.pin_object(p);
    heap.pin_object(p);
    heap.pin_object(p);
    assert_eq!(heap.pin_count(p), 3, "P1：引计数式屏障（计数累计）");
    heap.unpin_object(p).expect("U1");
    assert_eq!(heap.pin_count(p), 2);
}

/// P2：非堆值 pin no-op（幂等——经 FfiBoundary 真实实现面）。
#[test]
fn phi_p2_nonheap_noop() {
    let mut heap = Heap::new();
    {
        let mut b = HeapFfiBoundary::new(&mut heap);
        b.pin_object(Value::Int(42));
        b.pin_object(Value::Str(Rc::from("direct")));
        b.pin_object(Value::Nil);
    }
    assert!(heap.pinned_refs().is_empty(), "P2：非堆值 Φ 不动（幂等）");
}

/// U1：归零移除出 supp(Φ)（下一轮回收周期可回收——非立即回收）。
#[test]
fn phi_u1_unpin_zero_removes_from_support() {
    let mut heap = Heap::new();
    let s = heap.alloc_str(Rc::from("x"));
    heap.pin_object(s);
    assert_eq!(heap.pinned_refs(), vec![s], "supp(Φ) 含 pinned");
    heap.unpin_object(s).expect("U1");
    assert!(heap.pinned_refs().is_empty(), "归零摘根");
}

/// **case 1**（F-PIN 引理）：pin 后 GC 周期 → 对象存活；
/// unpin 归零 → 下一轮周期回收。
#[test]
fn f_pin_case1_survives_gc() {
    let mut heap = Heap::new();
    let s = heap.alloc_str(Rc::from("pinned"));
    let live_before = heap.stats().live;
    heap.pin_object(s);
    mark_sweep_cycle(&mut heap, &RootSet::new());
    assert_eq!(heap.stats().live, live_before, "F-PIN：Φ ≥ 1 ⟹ 本轮不回收");
    heap.unpin_object(s).expect("结构配对");
    mark_sweep_cycle(&mut heap, &RootSet::new());
    assert!(
        heap.stats().live < live_before,
        "unpin 摘根后下一轮可回收（非立即）"
    );
}

/// **case 8**：浅 pin 语义等价深 pin（mark 传播覆盖全可达子图——
/// 伪问题消解的运行时证明）。
#[test]
fn shallow_pin_case8_deep_reachability() {
    let mut heap = Heap::new();
    // 嵌套图：pair(a) → (pair(b) → (str_inner . nil)) . str_leaf
    let str_inner = heap.alloc_str(Rc::from("inner"));
    let nil = heap.alloc_nil();
    let pb = heap.alloc_pair(str_inner, nil);
    let str_leaf = heap.alloc_str(Rc::from("leaf"));
    let pa = heap.alloc_pair(pb, str_leaf);
    let live_before = heap.stats().live;
    heap.pin_object(pa); // 浅 pin：仅根单点
    mark_sweep_cycle(&mut heap, &RootSet::new());
    assert_eq!(
        heap.stats().live,
        live_before,
        "浅 pin：mark 传播覆盖全可达子图（等价深 pin）"
    );
}

/// **case 2**：pin 期间 set!（绑定变更）——pin 只约束回收资格，
/// 不冻结内容/绑定流。
#[test]
fn pin_window_allows_binding_mutation_case2() {
    use kerf_vm::GcCell;
    let mut heap = Heap::new();
    let s = heap.alloc_str(Rc::from("v1"));
    let p = heap.alloc_pair(s, s);
    heap.pin_object(p);
    let cell = GcCell::new(Value::Pair(p));
    // set!：绑定改指新值（绑定流不冻结）
    cell.set(Value::Int(99));
    assert_eq!(heap.pin_count(p), 1, "pin 簿不受绑定变更影响");
    mark_sweep_cycle(&mut heap, &RootSet::new());
    assert_eq!(heap.pin_count(p), 1, "回收资格不冻结——pin 维持");
    assert!(
        matches!(&*cell.get(), Value::Int(99)),
        "内容/绑定变更生效（互斥域内无数据竞争）"
    );
}

/// **case 9**（Foreign 槽回收令牌泄漏——容忍口径）：令牌装箱入序对
/// 后全部引用丢弃 → GC 回收承载槽；令牌真相在 Rc（非 GC 槽）→
/// 后续消费仍语义完好（无 UB）。
#[test]
fn token_slot_gc_reclaim_leak_tolerated_case9() {
    let mut heap = Heap::new();
    let token = ExternalToken::from_boxed_slice(vec![0u8; 8].into_boxed_slice());
    let v = Value::External(Rc::clone(&token));
    let r = kerf_vm::box_value(&v, &mut heap);
    let nil_ref = heap.alloc_nil();
    let _carrier = heap.alloc_pair(r, nil_ref);
    let live_before = heap.stats().live;
    mark_sweep_cycle(&mut heap, &RootSet::new());
    assert!(
        heap.stats().live < live_before,
        "承载槽回收（外部内存泄漏——容忍，finalizer = Stage 3 锚）"
    );
    // 令牌 Rc 仍有效：消费路径语义完好
    assert!(token.is_valid());
    let out = free_external(&Value::External(Rc::clone(&token)), Span::dummy())
        .expect("槽回收不影响令牌真相（Rc 承载）");
    assert!(matches!(out, Value::Unit));
    assert!(!token.is_valid(), "消费全局生效");
}

// ---------------------------------------------------------------------------
// 负例组（E0010/E0011/E0012 + 防御面——正负比 ≥1:3 主承载）
// ---------------------------------------------------------------------------

/// **case 5**：双重释放 → E0010（诊断吸收非 UB——C double-free 的
/// UB 语义升级）。
/// 码序：[Alloc, Dup, Free, Pop, Free, Halt]——Dup 留共享 Rc 份在栈，
/// 第一次 Free 消费（全局失效），Pop 弃 Unit 返回，第二次 Free 触达
/// Invalid 令牌（同一令牌的第二引用——case 11 的程序化形态）。
#[test]
fn negative_double_free_e10() {
    let table = test_table();
    let prog = ffi_program(
        vec![
            Op::AllocExternal { size: 16 },
            Op::Dup,
            Op::FreeExternal,
            Op::Pop,
            Op::FreeExternal,
            Op::Halt,
        ],
        Vec::new(),
    );
    let e = run_ffi(&prog, &table).expect_err("双重释放必须检出");
    assert_eq!(e.code, Some(10), "E0010 码");
    assert!(
        e.message.contains("令牌已失效") || e.message.contains("双重释放"),
        "消息：{}",
        e.message
    );
}

/// 非法迁移 2：用后传递（Invalid 令牌作 CallExternal 实参）→ E0010。
/// 码序：[Alloc, Dup, Free, Pop, CallExternal probe 1, Halt]——Dup 后
/// 栈上两份共享 Rc，Free 消费其一（全局失效），残余份传参即触发。
#[test]
fn negative_use_after_free_arg_pass_e10() {
    let table = test_table();
    let prog = ffi_program(
        vec![
            Op::AllocExternal { size: 8 },
            Op::Dup,
            Op::FreeExternal,
            Op::Pop,
            Op::CallExternal {
                symbol: 0,
                n_args: 1,
            },
            Op::Halt,
        ],
        vec![sym_const("probe_token")],
    );
    let e = run_ffi(&prog, &table).expect_err("用后传递必须检出");
    assert_eq!(e.code, Some(10), "E0010 码（令牌失效后传递）");
}

/// **case 11**：令牌复制后一处 Free → 消费全局生效（槽级失效标记）——
/// 另一共享绑定任何后续使用 = E0010（Rc 共享 Cell 的全局可见性）。
#[test]
fn negative_token_shared_invalidation_e10() {
    let table = test_table();
    let token = ExternalToken::from_boxed_slice(vec![0u8; 4].into_boxed_slice());
    let a = Value::External(Rc::clone(&token));
    let b = Value::External(Rc::clone(&token));
    free_external(&a, Span::dummy()).expect("一处消费");
    assert!(!token.is_valid(), "消费全局生效（共享 Cell）");
    let e = call_external(&table, "probe_token", &[b], &mut Heap::new(), Span::dummy())
        .expect_err("共享引用同步失效");
    assert_eq!(e.code, Some(10), "E0010（用后使用）");
}

/// **case 4**（编译期）：AllocExternal size=0 → lowering 诊断拒绝。
#[test]
fn negative_alloc_size_zero_compile_rejected() {
    let table = SymbolTable::new();
    let e = compile_ffi_call_program(&FfiCall::AllocExternal { size: 0 }, &table)
        .expect_err("size=0 编译期拒绝");
    assert!(e.message.contains("尺寸为 0"), "消息：{}", e.message);
}

/// **case 4**（运行期纵深防御）：操作码直接构造 size=0 → E0011。
#[test]
fn negative_alloc_size_zero_vm_defense() {
    let table = test_table();
    let prog = ffi_program(vec![Op::AllocExternal { size: 0 }, Op::Halt], Vec::new());
    let e = run_ffi(&prog, &table).expect_err("运行期防御拒绝");
    assert_eq!(e.code, Some(11), "E0011（纵深防御）");
}

/// 非法迁移 4：释放 Opaque 令牌 → E0011（外部持有，kerf 无释放权）。
#[test]
fn negative_free_opaque_e11() {
    let table = test_table();
    // make_opaque（Opaque 返回）→ Free（应拒绝）
    let prog = ffi_program(
        vec![
            Op::CallExternal {
                symbol: 0,
                n_args: 0,
            },
            Op::FreeExternal,
            Op::Halt,
        ],
        vec![sym_const("make_opaque")],
    );
    let e = run_ffi(&prog, &table).expect_err("Opaque 释放必须拒绝");
    assert_eq!(e.code, Some(11));
    assert!(
        e.message.contains("外部持有") || e.message.contains("Opaque"),
        "消息：{}",
        e.message
    );
}

/// **case 12**：非 Foreign 值释放（Int）→ E0011（类型违规）。
#[test]
fn negative_free_non_token_value_e11() {
    let e = free_external(&Value::Int(42), Span::dummy()).expect_err("非令牌释放必须拒绝");
    assert_eq!(e.code, Some(11));
    assert!(e.message.contains("Foreign 令牌"), "消息：{}", e.message);
}

/// case 12 泛化（第二形态）：Str 值释放 → E0011。
#[test]
fn negative_free_str_value_e11() {
    let e = free_external(&Value::Str(Rc::from("s")), Span::dummy()).expect_err("Str 释放拒绝");
    assert_eq!(e.code, Some(11));
}

/// 非法迁移 6：符号未解析 → E0012（fail-closed）。
#[test]
fn negative_symbol_unresolved_e12() {
    let table = test_table();
    let prog = call_str_program("no_such_fn", "x");
    let e = run_ffi(&prog, &table).expect_err("未登记符号必须失败");
    assert_eq!(e.code, Some(12), "E0012 码");
    assert!(
        e.message.contains("no_such_fn") && e.message.contains("符号解析"),
        "消息：{}",
        e.message
    );
}

/// 默认入口 fail-closed：run_program（空表）下 FFI 操作码 → E0012
/// （生产 run 路径无 FFI 注册面——注册面显式性）。
#[test]
fn negative_default_entry_fail_closed_e12() {
    let prog = call_str_program("write_stdout", "x");
    let mut heap = Heap::new();
    let mut globals = HashMap::new();
    let e = run_program(&prog, &mut globals, &mut heap).expect_err("空表 fail-closed");
    assert_eq!(e.code, Some(12), "默认入口 E0012");
}

/// 实参数不匹配（2 实参 vs 2 形参的正例对照 + 1 实参错配负例）：
/// 1 实参传 two_params（2 形参）→ E0011。
#[test]
fn negative_arity_mismatch_e11() {
    let table = test_table();
    let e = call_external(
        &table,
        "two_params",
        &[Value::Int(1)],
        &mut Heap::new(),
        Span::dummy(),
    )
    .expect_err("元数错配必须检出");
    assert_eq!(e.code, Some(11));
    assert!(e.message.contains("实参数不匹配"), "消息：{}", e.message);
}

/// 零实参元数错配（capture 恰 1 形参）→ E0011。
#[test]
fn negative_zero_args_arity_mismatch_e11() {
    let table = test_table();
    let e = call_external(&table, "capture", &[], &mut Heap::new(), Span::dummy())
        .expect_err("零实参元数错配");
    assert_eq!(e.code, Some(11));
}

/// CInt 形参 ← Str 实参 → E0011（编组类型违规）。
#[test]
fn negative_cint_arg_type_mismatch_e11() {
    let table = test_table();
    let e = call_external(
        &table,
        "double",
        &[Value::Str(Rc::from("s"))],
        &mut Heap::new(),
        Span::dummy(),
    )
    .expect_err("CInt 形参类型违规");
    assert_eq!(e.code, Some(11));
    assert!(e.message.contains("CInt"), "消息：{}", e.message);
}

/// CPointer 形参 ← Int 实参 → E0011（编组类型违规——case 12 泛化）。
#[test]
fn negative_cpointer_arg_type_mismatch_e11() {
    let table = test_table();
    let e = call_external(
        &table,
        "capture",
        &[Value::Int(7)],
        &mut Heap::new(),
        Span::dummy(),
    )
    .expect_err("CPointer 形参类型违规");
    assert_eq!(e.code, Some(11));
    assert!(e.message.contains("CPointer"), "消息：{}", e.message);
}

/// 宿主失败传播：RuntimeError → VmError（E0004 运行时通用族——
/// E7 I/O 通道失败口径）。
#[test]
fn negative_host_error_propagates_e0004() {
    let table = test_table();
    let prog = ffi_program(
        vec![
            Op::CallExternal {
                symbol: 0,
                n_args: 0,
            },
            Op::Halt,
        ],
        vec![sym_const("failing")],
    );
    let e = run_ffi(&prog, &table).expect_err("宿主失败必须传播");
    assert_eq!(e.code, None, "E0004 运行时通用族（code None）");
    assert!(e.message.contains("外部调用失败"), "消息：{}", e.message);
}

/// 码面防御：CALL_EXTERNAL 操作数非 SymLit 常量 → VmError（防御）。
#[test]
fn negative_symbol_operand_not_symlit() {
    let table = test_table();
    let prog = ffi_program(
        vec![
            Op::CallExternal {
                symbol: 0,
                n_args: 0,
            },
            Op::Halt,
        ],
        vec![BcConst::Int(5)],
    );
    let e = run_ffi(&prog, &table).expect_err("符号操作数防御");
    assert!(e.message.contains("SymLit"), "消息：{}", e.message);
}

/// 栈防御：n_args 超栈深 → VmError（数据栈下溢防御）。
#[test]
fn negative_stack_underflow_ffi_args() {
    let table = test_table();
    let prog = ffi_program(
        vec![
            Op::CallExternal {
                symbol: 0,
                n_args: 2,
            },
            Op::Halt,
        ],
        vec![sym_const("capture")],
    );
    let e = run_ffi(&prog, &table).expect_err("实参不足防御");
    assert!(e.message.contains("下溢"), "消息：{}", e.message);
}

/// lowering 面：非字面量实参（VarRef）→ 编译期拒绝（语言面 Stage 3
/// 收窄的显式化）。
#[test]
fn negative_nonliteral_arg_rejected() {
    let mut st = SymbolTable::new();
    let sym = st.intern("write_stdout");
    let var = kerf_core::CoreExpr::VarRef {
        name: st.intern("buf"),
        scopes: kerf_syntax::ScopeSet::new(),
        span: Span::dummy(),
    };
    let e = compile_ffi_call_program(
        &FfiCall::CallExternal {
            symbol: sym,
            args: vec![var],
            return_type: ExternalType::CInt(CIntSize::Usize),
        },
        &st,
    )
    .expect_err("非字面量实参拒绝");
    assert!(e.message.contains("字面量"), "消息：{}", e.message);
}

/// lowering 面：FreeExternal（实参为运行时令牌值）→ 拒绝（执行面 =
/// Op::FreeExternal 操作码——诚实收窄注记）。
#[test]
fn negative_free_lowering_rejected() {
    let table = SymbolTable::new();
    let e = compile_ffi_call_program(
        &FfiCall::FreeExternal {
            ptr: kerf_core::CoreExpr::Literal {
                value: kerf_core::LiteralValue::Nil,
                span: Span::dummy(),
            },
        },
        &table,
    )
    .expect_err("FreeExternal lowering 拒绝");
    assert!(e.message.contains("运行时令牌"), "消息：{}", e.message);
}

/// **case 3** 锚（CFunction = 形状非回调注册面）+ CStruct：Stage 2
/// VM 消费子集外 → 注册面拒绝（诚实收窄）。
#[test]
fn negative_cstruct_cfunction_registration_out_of_subset() {
    use kerf_driver::ffi::external_kind_supported;
    assert!(!external_kind_supported(&ExternalType::CFunction {
        param: Box::new(ExternalType::CInt(CIntSize::I64)),
        result: Box::new(ExternalType::CInt(CIntSize::I64)),
    }));
    assert!(!external_kind_supported(&ExternalType::CStruct(vec![
        ExternalType::CPointer(PointeeType::U8)
    ])));
    assert!(external_kind_supported(&ExternalType::CPointer(
        PointeeType::Char
    )));
}

/// **case 9**（错误传播路径 pin 泄漏——容忍）：装载边界中途失败
/// （第 2 实参 Invalid）→ 已 pin 的第 1 实参槽泄漏（永久根形态；
/// 误回收零容忍的对称面）。
#[test]
fn negative_error_path_pin_leak_tolerated_case9() {
    let mut table = test_table();
    table.register(
        "two_mixed",
        vec![ExternKind::CPointer, ExternKind::CPointer],
        ExternKind::CInt,
        |args| match args {
            [BorrowedArg::CharBuf(_), BorrowedArg::Token(_)] => Ok(HostRet::Int(0)),
            _ => Err(RuntimeError::new("two_mixed 形态不符")),
        },
    );
    let mut heap = Heap::new();
    let token = ExternalToken::from_boxed_slice(vec![0u8; 4].into_boxed_slice());
    free_external(&Value::External(Rc::clone(&token)), Span::dummy()).expect("先消费");
    let e = call_external(
        &table,
        "two_mixed",
        &[Value::Str(Rc::from("first")), Value::External(token)],
        &mut heap,
        Span::dummy(),
    )
    .expect_err("第 2 实参 Invalid → E0010");
    assert_eq!(e.code, Some(10));
    assert_eq!(
        heap.pinned_refs().len(),
        1,
        "case 9：装载中途失败——已 pin 槽泄漏容忍（永久根形态）"
    );
}

/// **case 9**（C 侧非局部跳转同型——panic unwind 路径）：宿主恐慌
/// → 窗口未闭合 → pin 泄漏容忍（unwind 不破坏堆不变式）。
#[test]
fn negative_panic_host_pin_leak_tolerated_case9() {
    let table = test_table();
    let mut heap = Heap::new();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = call_external(
            &table,
            "panic_fn",
            &[Value::Str(Rc::from("x"))],
            &mut heap,
            Span::dummy(),
        );
    }));
    assert!(result.is_err(), "宿主恐慌向上传播（非局部跳转同型）");
    assert_eq!(
        heap.pinned_refs().len(),
        1,
        "case 9：unwind 路径 pin 泄漏容忍（C longjmp 同型——永久根）"
    );
    // 堆不变式未破坏：后续 unpin 配对可完成（泄漏为容忍态而非腐坏态）
    let r = heap.pinned_refs()[0];
    heap.unpin_object(r)
        .expect("泄漏槽可显式回收配对（非腐坏）");
}

// ---------------------------------------------------------------------------
// U2 下溢（E8 口径——Heap API 的 Result 面直测）
// ---------------------------------------------------------------------------

/// U2：未登记 unpin → Err（E8 口径——边界实现簿记缺陷；结构配对
/// 保证用户程序不可触达）。
#[test]
fn phi_u2_underflow_internal_error() {
    let mut heap = Heap::new();
    let s = heap.alloc_str(Rc::from("x"));
    let e = heap.unpin_object(s).expect_err("下溢必须检出");
    assert!(
        e.message.contains("下溢") || e.message.contains("E8"),
        "消息：{}",
        e.message
    );
}
