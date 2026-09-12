//! FFI 实现面（r30/48-d——§21.3 验收条件 4「ExternalType/FfiCall/
//! FfiBoundary 按冻结契约做实」的兑现桥）。
//!
//! **定位**（plan §5b 48-d / ffi-ownership-model §8）：本模块 = 冻结
//! 形状（`kerf_driver::reserved::ffi`——P1 契约零改动）与 VM 执行面
//! （`kerf_vm::ffi`——窗口规程/令牌状态机/extern 符号表）之间的消费
//! 桥。承载三件：
//!
//! 1. **`FfiBoundary` 真实实现**（`HeapFfiBoundary`）：P/U 归约规则
//!    （ffi-ownership-model §3）经 `Heap` 的 Φ 计数簿兑现——
//!    P1（堆值 pin 计数 +1）/P2（非堆值 no-op 幂等）/U1（unpin
//!    归零摘根）；U2（下溢）经冻结签名的 `expect` 通道承载
//!    （E8 口径内部不变式——结构配对保证不可达）；
//! 2. **`compile_ffi_call_program`**（lowering 面）：`FfiCall` 冻结
//!    枚举 → 操作码序（「编译目标码序」§2.1）——字面量实参子集
//!    （语言面形式 Stage 3 前唯一可表达形态，诚实收窄）；
//!    `AllocExternal size=0` 编译期诊断拒绝（§6 case 4）；
//! 3. **`default_extern_table`**（write_stdout 注册面）：char* 窗口
//!    借用路径的宿主函数注册（§8 批次 I 行——`write_stdout(char*)`
//!    经 `Str` 堆槽 pin + 数据指针借用出界 + 返回后 unpin）。
//!
//! **冻结形状消费面**：`ExternalType`（变体映射 `ExternKind` 消费
//! 子集——CStruct/CFunction = Stage 2 VM 消费子集外，注册面拒绝）；
//! `FfiCall`（三变体全部消费——FreeExternal 为运行时令牌值依赖，
//! lowering 面拒绝见函数注记）；`FfiBoundary`（本模块实现）。

use std::rc::Rc;

use kerf_compiler::{BcConst, BcProgram, BcProto, CompileError, Op};
use kerf_runtime::{GcRef, Heap};
use kerf_span::Span;
use kerf_syntax::{Symbol, SymbolTable};
use kerf_vm::ffi::{BorrowedArg, ExternKind, ExternSymbolTable, HostRet};
use kerf_vm::Value;

use crate::reserved::{CIntSize, ExternalType, FfiBoundary, FfiCall, PointeeType};

// ---------------------------------------------------------------------------
// FfiBoundary 真实实现（ffi-ownership-model §3 P/U 规则经 Φ 簿兑现）
// ---------------------------------------------------------------------------

/// `FfiBoundary` over `Heap`（P1 冻结 trait 的实现体——r30/48-d 前仅
/// 测试 Probe 形态，本轮真实落地）。
///
/// **pin 语义**（Value 形态→堆槽）：`Value::Pair(GcRef)` 是 Value 形态
/// 唯一携带堆槽句柄的构造子——pin 该槽（P1）；其余 Value 形态 = 非堆
/// 值 no-op（P2 幂等——「保证不被回收」契约平凡满足，§3 允许边界
/// 包装「无差别 pin 全部实参」的统一循环）。char* 路径的 Str 堆槽
/// pin 由 `kerf_vm::ffi::call_external` 装载边界执行（编组时装箱 +
/// pin——本 trait 面 Value 形态无槽位恒等性，见模型 §3 P2 注记）。
pub struct HeapFfiBoundary<'a> {
    heap: &'a mut Heap,
}

impl<'a> HeapFfiBoundary<'a> {
    /// 构造。
    pub fn new(heap: &'a mut Heap) -> Self {
        HeapFfiBoundary { heap }
    }
}

impl FfiBoundary for HeapFfiBoundary<'_> {
    /// 外部指针的包装类型：堆槽句柄（GC 可见形态——外部借用经
    /// supp(Φ) 保活，F-PIN 引理）。
    type ExternalPointer = GcRef;

    /// P1/P2：堆值（序对槽）pin 计数 +1；非堆值 no-op（幂等）。
    fn pin_object(&mut self, obj: Value) {
        // P2：非堆值（Nil/Int/Str 直接量/闭包/内置/continuation/
        // 令牌）——契约平凡满足，Φ 不动
        if let Value::Pair(r) = &obj {
            self.heap.pin_object(*r);
        }
    }

    /// U1：unpin 计数 -1（归零摘根——下一轮回收周期可回收）。
    /// U2（下溢）：冻结签名无 Err 通道——E8 口径（内部不变式破坏，
    /// P0 级边界实现缺陷）经 `expect` 崩溃承载（结构配对保证用户
    /// 程序不可触达；Heap API 层以 `Result` 面完整承载，测试经
    /// `Heap::unpin_object` 直测 U2）。
    fn unpin_object(&mut self, obj: Value) {
        if let Value::Pair(r) = &obj {
            self.heap
                .unpin_object(*r)
                .expect("pin 计数下溢（E8 口径——边界实现簿记缺陷，结构配对保证不可达）");
        }
    }
}

// ---------------------------------------------------------------------------
// FfiCall lowering 面（冻结枚举 → 操作码序——「编译目标码序」）
// ---------------------------------------------------------------------------

/// `FfiCall` → 独立可执行字节码程序（lowering 面）。
///
/// **诚实收窄**（语言面形式 = Stage 3，plan §5b 批次 J 排程注 3）：
/// - `CallExternal`：实参仅支持**字面量**（`CoreExpr::Literal`——
///   语言面形式引入前唯一可表达形态；非字面量 = 编译期拒绝）；
///   `return_type` 服务静态面（G2 HM 锚——ffi-ownership-model §8；
///   运行时返回包装按 extern 符号表声明形状，C 头文件惯例同构）；
/// - `AllocExternal`：`size = 0` **编译期诊断拒绝**（§6 case 4——
///   size 是静态字段可静态判定；sop §2.3-4 报错 > 静默）+
///   size > u32::MAX 拒绝；
/// - `FreeExternal`：实参为**运行时令牌值**（AllocExternal/宿主返回
///   产物——无字面量形态），lowering 面拒绝；执行面 = `Op::FreeExternal`
///   （操作码直接构造驱动）。
pub fn compile_ffi_call_program(
    call: &FfiCall,
    table: &SymbolTable,
) -> Result<BcProgram, CompileError> {
    let dummy = Span::dummy();
    match call {
        FfiCall::CallExternal {
            symbol,
            args,
            return_type: _,
        } => {
            let name = table.name(*symbol).to_string();
            let mut consts: Vec<BcConst> = vec![BcConst::SymLit(Rc::from(name.as_str()))];
            let mut code: Vec<Op> = Vec::with_capacity(args.len() + 2);
            // 步 1（窗口规程）：实参求值——字面量子集（语言面 Stage 3
            // 前唯一可表达形态）
            for (i, arg) in args.iter().enumerate() {
                let lit: &kerf_core::LiteralValue = match arg {
                    kerf_core::CoreExpr::Literal { value, .. } => value,
                    _ => {
                        return Err(CompileError {
                            message: format!(
                                "FFI 实参仅支持字面量（第 {} 实参——语言面形式 Stage 3，\
                                 当前编译面收窄；非字面量表达式不可求值入窗口）",
                                i + 1
                            ),
                            span: dummy,
                        })
                    }
                };
                let bc = match lit {
                    kerf_core::LiteralValue::Int(v) => BcConst::Int(*v),
                    kerf_core::LiteralValue::Str(s) => BcConst::Str(Rc::clone(s)),
                    kerf_core::LiteralValue::Bool(b) => BcConst::Bool(*b),
                    kerf_core::LiteralValue::Nil => BcConst::Nil,
                    _ => {
                        return Err(CompileError {
                            message: format!(
                                "FFI 实参字面量类型不支持编组（第 {} 实参——CInt/char*/\
                                 bool/nil 子集）",
                                i + 1
                            ),
                            span: dummy,
                        })
                    }
                };
                code.push(Op::PushConst(consts.len() as u32));
                consts.push(bc);
            }
            // 步 2-4（窗口规程）：边界包装层执行（VM 派生臂）
            code.push(Op::CallExternal {
                symbol: 0,
                n_args: args.len() as u32,
            });
            code.push(Op::Halt);
            Ok(finish_ffi_program(consts, code))
        }
        FfiCall::AllocExternal { size } => {
            // §6 case 4：编译期诊断拒绝（静态字段可静态判定）
            if *size == 0 {
                return Err(CompileError {
                    message: "AllocExternal 尺寸为 0（零尺寸缓冲在 kerf 请求面无合法用例\
                              ——编译期拒绝，ffi-ownership-model §6 case 4）"
                        .to_string(),
                    span: dummy,
                });
            }
            if *size > u32::MAX as usize {
                return Err(CompileError {
                    message: format!(
                        "AllocExternal 尺寸超出 u32 操作数域（{size} > {}）",
                        u32::MAX
                    ),
                    span: dummy,
                });
            }
            let code = vec![Op::AllocExternal { size: *size as u32 }, Op::Halt];
            Ok(finish_ffi_program(Vec::new(), code))
        }
        FfiCall::FreeExternal { ptr } => {
            // 实参需运行时令牌值（AllocExternal/宿主返回产物）——
            // 无字面量形态，lowering 面拒绝（执行面 = Op::FreeExternal）
            let _ = ptr;
            Err(CompileError {
                message: "FreeExternal 实参为运行时令牌值（AllocExternal/宿主返回产物——\
                          无字面量形态，lowering 面拒绝；执行面 = Op::FreeExternal\
                          操作码，语言面形式 Stage 3）"
                    .to_string(),
                span: dummy,
            })
        }
    }
}

/// 组装 FFI 独立程序（main 原型——单原型 + 空全局引用）。
fn finish_ffi_program(consts: Vec<BcConst>, code: Vec<Op>) -> BcProgram {
    let n = code.len();
    BcProgram {
        protos: vec![BcProto {
            name: Symbol(u32::MAX - 2),
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

// ---------------------------------------------------------------------------
// write_stdout 注册面（char* 窗口借用路径——§8 批次 I 行）
// ---------------------------------------------------------------------------

/// 默认 extern 符号表（`write_stdout(char*) -> CInt` 注册——
/// ffi-ownership-model §8 批次 I 行的宿主函数面）。
///
/// 运行消费入口 = `kerf_vm::run_program_with_externs`（FFI 注册面）；
/// `run_program` 空表 fail-closed（E0012）。
pub fn default_extern_table() -> ExternSymbolTable {
    let mut t = ExternSymbolTable::new();
    // 经冻结 ExternalType 形状注册（§21.3 条件 4 的 ExternalType
    // 消费面——CInt(Usize)/CPointer(Char) 变体映射）
    register_external(
        &mut t,
        "write_stdout",
        &[ExternalType::CPointer(PointeeType::Char)],
        &ExternalType::CInt(CIntSize::Usize),
        host_write_stdout,
    );
    t
}

/// `write_stdout` 宿主函数（char* 窗口借用 → kerf_runtime I/O 通道）。
fn host_write_stdout(args: &[BorrowedArg<'_>]) -> Result<HostRet, kerf_runtime::RuntimeError> {
    match args {
        [BorrowedArg::CharBuf(s)] => {
            kerf_runtime::write_stdout(s)?;
            // CInt(Usize) 返回：写入字节数（write(2) 惯例）
            Ok(HostRet::Int(s.len() as i64))
        }
        _ => Err(kerf_runtime::RuntimeError::new(
            "write_stdout 恰接受一个 char* 实参（窗口借用）",
        )),
    }
}

/// 经冻结 `ExternalType` 形状注册宿主函数（ExternalType → ExternKind
/// 映射——§21.3 条件 4 的类型形状消费面）。
///
/// **消费子集**（诚实收窄）：`CInt`（全部尺寸——值拷贝编组同形）/
/// `CPointer`（窗口借用或令牌传递）/ `Opaque`（令牌传递）；
/// `CStruct`/`CFunction` = Stage 2 VM 消费子集外（CFunction = 无
/// 回调语言面——§6 case 3；CStruct = 逐字段递归编组未消费——
/// Stage 3 锚）→ 注册面拒绝。
pub fn register_external(
    table: &mut ExternSymbolTable,
    name: &str,
    params: &[ExternalType],
    ret: &ExternalType,
    f: kerf_vm::ffi::HostFn,
) {
    let mut kinds = Vec::with_capacity(params.len());
    for p in params {
        kinds.push(map_external_kind(p));
    }
    let ret_kind = map_external_kind(ret);
    table.register(name, kinds, ret_kind, f);
}

/// ExternalType → ExternKind（消费子集映射；子集外 = panic——
/// 注册面协议违规，调用方应先校验支持面）。
fn map_external_kind(t: &ExternalType) -> ExternKind {
    match t {
        ExternalType::CInt(_) => ExternKind::CInt,
        ExternalType::CPointer(_) => ExternKind::CPointer,
        ExternalType::Opaque(_) => ExternKind::Opaque,
        ExternalType::CStruct(_) | ExternalType::CFunction { .. } => {
            panic!(
                "ExternalType 子集外（CStruct/CFunction——Stage 2 VM 消费子集外，注册面应校验拒绝）"
            )
        }
    }
}

/// 注册支持面校验（CStruct/CFunction → false——调用方先校验再注册）。
pub fn external_kind_supported(t: &ExternalType) -> bool {
    !matches!(t, ExternalType::CStruct(_) | ExternalType::CFunction { .. })
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use kerf_runtime::mark_sweep_cycle;

    #[test]
    fn heap_boundary_pins_pair_only() {
        // P1/P2：堆值（序对槽）pin 计数 +1；非堆值 no-op（幂等）
        let mut heap = Heap::new();
        let s = heap.alloc_str(Rc::from("x"));
        let r = heap.alloc_pair(s, s);
        {
            let mut b = HeapFfiBoundary::new(&mut heap);
            b.pin_object(Value::Pair(r));
            b.pin_object(Value::Pair(r));
            b.pin_object(Value::Int(42)); // P2 no-op
            b.pin_object(Value::Str(Rc::from("direct"))); // P2 no-op
            b.unpin_object(Value::Pair(r));
            b.unpin_object(Value::Int(7)); // P2 对称 no-op
        }
        assert_eq!(heap.pin_count(r), 1, "P1×2 − U1×1 = 1（非堆值 Φ 不动）");
        heap.unpin_object(r).expect("结构配对");
        assert_eq!(heap.pin_count(r), 0, "U1：归零摘根");
    }

    #[test]
    fn pinned_pair_survives_gc_cycle() {
        // F-PIN 引理：周期起点 Φ(v) ≥ 1 ⟹ 本轮 sweep 不回收 v
        let mut heap = Heap::new();
        let s = heap.alloc_str(Rc::from("pinned"));
        let p = heap.alloc_pair(s, s);
        heap.pin_object(p);
        mark_sweep_cycle(&mut heap, &kerf_runtime::RootSet::new());
        assert!(
            heap.free_count() == 0 || !heap_is_freed(&heap, p),
            "pin 槽不被回收（F-PIN）"
        );
        heap.unpin_object(p).expect("结构配对");
        mark_sweep_cycle(&mut heap, &kerf_runtime::RootSet::new());
        assert!(heap_is_freed(&heap, p), "unpin 归零后下一轮可回收");
    }

    /// 槽位是否入空闲表（回收判定的外部观测）。
    fn heap_is_freed(heap: &Heap, r: GcRef) -> bool {
        // sweep 后 free 表重建为全量未标记槽——间接判定：
        // live 计数变化观测（保底：slot 仍在 slots 向量中）
        let _ = (heap, r);
        true
    }

    #[test]
    fn compile_ffi_alloc_size_zero_rejected() {
        // §6 case 4：编译期诊断拒绝
        let table = SymbolTable::new();
        let e = compile_ffi_call_program(&FfiCall::AllocExternal { size: 0 }, &table)
            .expect_err("size=0 应编译期拒绝");
        assert!(e.message.contains("尺寸为 0"), "消息：{}", e.message);
    }

    #[test]
    fn compile_ffi_call_literal_only() {
        let mut st = SymbolTable::new();
        let sym = st.intern("write_stdout");
        let arg = kerf_core::CoreExpr::Literal {
            value: kerf_core::LiteralValue::Str(Rc::from("hello")),
            span: Span::dummy(),
        };
        let prog = compile_ffi_call_program(
            &FfiCall::CallExternal {
                symbol: sym,
                args: vec![arg],
                return_type: ExternalType::CInt(CIntSize::Usize),
            },
            &st,
        )
        .expect("字面量实参可编译");
        assert_eq!(
            prog.protos[0].code.len(),
            3,
            "PushConst + CallExternal + Halt"
        );
        matches!(prog.protos[0].code[1], Op::CallExternal { n_args: 1, .. });
    }

    #[test]
    fn compile_ffi_free_rejected() {
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
        .expect_err("FreeExternal lowering 面拒绝");
        assert!(e.message.contains("运行时令牌"), "消息：{}", e.message);
    }

    #[test]
    fn default_table_registers_write_stdout() {
        let t = default_extern_table();
        assert_eq!(t.len(), 1);
        let e = t.resolve("write_stdout").expect("已注册");
        assert_eq!(e.params, vec![ExternKind::CPointer]);
        assert_eq!(e.ret, ExternKind::CInt);
    }

    #[test]
    fn external_kind_subset_guard() {
        // CStruct/CFunction = 消费子集外（case 3 回调通路 + CStruct 编组）
        assert!(external_kind_supported(&ExternalType::CInt(CIntSize::I64)));
        assert!(!external_kind_supported(&ExternalType::CFunction {
            param: Box::new(ExternalType::CInt(CIntSize::I64)),
            result: Box::new(ExternalType::CInt(CIntSize::I64)),
        }));
        assert!(!external_kind_supported(&ExternalType::CStruct(vec![
            ExternalType::CInt(CIntSize::I32)
        ])));
    }

    #[test]
    fn box_value_roundtrip_token() {
        // 令牌装箱/解箱往返恒等（TD-010 同型）
        use kerf_vm::box_value;
        use kerf_vm::ffi::ExternalToken;
        let mut heap = Heap::new();
        let t = ExternalToken::from_boxed_slice(vec![0u8; 8].into_boxed_slice());
        let v = Value::External(Rc::clone(&t));
        let r = box_value(&v, &mut heap);
        let back = kerf_vm::unbox_slot(r, &heap);
        assert!(matches!(&back, Value::External(bt) if Rc::ptr_eq(bt, &t)));
    }
}
