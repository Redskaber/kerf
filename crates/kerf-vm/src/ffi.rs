//! FFI 执行面（r30/48-d——ffi-ownership-model 行为规格的 VM 落地）。
//!
//! **定位**（plan §5b 48-d）：`FfiCall` 的 **VM 执行面**——三原语
//! （CallExternal/AllocExternal/FreeExternal）经操作码
//! （`Op::CallExternal`/`Op::AllocExternal`/`Op::FreeExternal`）驱动；
//! **语言面形式 = Stage 3**（编译臂不发射——IR/VM 面做实即 §21.3
//! 条件 4 兑现，plan §5b 批次 J 排程注 3 如实注记）。
//!
//! **本模块承载**：
//! - `ExternalToken`：线性令牌状态机（ffi-ownership-model §5——
//!   Valid/Invalid 二态；消费经槽级失效标记**全局生效**，一切共享
//!   `Rc` 引用同步失效）；
//! - `ExternSymbolTable`：extern 符号表（§4——扁平符号空间，独立于
//!   用户词法环境，不可被 `define`/`set!` 遮蔽）；
//! - `call_external`：窗口规程执行器（§2.1 步 2-4——装载边界（堆
//!   实参 pin `Φ[v] += 1` + 令牌校验）→ 宿主调用（VM 挂起）→
//!   返回包装 + 全部堆实参 unpin）；
//! - `alloc_external`/`free_external`：外部域分配/释放（外部 malloc
//!   域——**不经过 GC**）。
//!
//! **外部域纪律**（Stage 2 诚实注记）：外部 malloc 域 = Rust 宿主堆
//! `Box<[u8]>` 面（std 默认分配器即系统 malloc）；真 C ABI 链接 =
//! QBE AOT 路径（kerf-backend——不进自举链）。令牌携带
//! `(addr, size)` 对以重建装箱切片（释放路径）。
//!
//! **E 码**（18 §6 码位登记——r18 预留、本轮落位）：
//! E0010 = E9 `FfiTokenInvalid`（令牌失效后使用）；E0011 = E10
//! `FfiOwnershipViolation`（所有权/类型违规）；E0012 = E11
//! `FfiSymbolResolution`（符号解析失败）。消息经
//! `messages.rs` 单源构造（TD-018 纪律）。

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use kerf_runtime::{GcRef, Heap, RuntimeError};
use kerf_span::Span;

use crate::value::Value;
use crate::vm::VmError;

// ---------------------------------------------------------------------------
// 线性令牌状态机（ffi-ownership-model §5）
// ---------------------------------------------------------------------------

/// 令牌种类（`ExternalType::CPointer`/`Opaque` 的运行时消费子集——
/// 所有权归属判据：CPointer = 所有权移交 kerf（可 Free）；
/// Opaque = 外部持有（不可 Free，恒 Valid 直至承载槽被 GC 回收）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    CPointer,
    Opaque,
}

/// 令牌生命周期状态（§5 状态机：Valid → Invalid 一次性消费不可逆；
/// Invalid 是吸收态——任何 FFI 原语操作 = E0010）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenState {
    Valid,
    Invalid,
}

/// FFI 外部令牌（`HeapObj::Foreign` 装箱时的载荷形态之一；直接值
/// 形态 = `Value::External`）。
///
/// **消费全局生效**（§4 主裁定）：`Rc` 共享 + `Cell` 内嵌——
/// 一处 `FreeExternal` 消费，所有共享绑定同步失效（防复制线性的
/// 运行时等价：安全目标 = 防双重释放/防用后释放，不依赖防复制）。
#[derive(Debug)]
pub struct ExternalToken {
    kind: TokenKind,
    /// 外部域地址（`null` = 空令牌哨兵——§6 case 13：Free = 合法
    /// no-op（对齐 `free(NULL)`）；解引用面 = Stage 3）。
    addr: *mut u8,
    /// 外部域分配长度（Box<[u8]> 重建纪律——释放路径的切片长度；
    /// Opaque/NULL 令牌不消费此字段）。
    size: usize,
    state: Cell<TokenState>,
}

// 单线程域（§6 case 10：Stage 2 裁定——GC 与 FFI 边界互斥于 VM
// 挂起点；Rc/Cell 全家非 Send/Sync 即结构性单线程面）。
impl ExternalToken {
    /// 从外部域分配构造 CPointer 令牌（所有权移交 kerf）。
    pub fn from_boxed_slice(buf: Box<[u8]>) -> Rc<Self> {
        let size = buf.len();
        let addr = Box::into_raw(buf) as *mut u8;
        Rc::new(ExternalToken {
            kind: TokenKind::CPointer,
            addr,
            size,
            state: Cell::new(TokenState::Valid),
        })
    }

    /// NULL 哨兵令牌（Valid 空令牌——§6 case 13）。
    pub fn null_sentinel() -> Rc<Self> {
        Rc::new(ExternalToken {
            kind: TokenKind::CPointer,
            addr: std::ptr::null_mut(),
            size: 0,
            state: Cell::new(TokenState::Valid),
        })
    }

    /// Opaque 令牌（外部持有——不存在 kerf 侧消费通路，恒 Valid）。
    pub fn opaque(addr: *mut u8) -> Rc<Self> {
        Rc::new(ExternalToken {
            kind: TokenKind::Opaque,
            addr,
            size: 0,
            state: Cell::new(TokenState::Valid),
        })
    }

    /// 宿主返回 CPointer 构造（strdup 类惯例：C 分配、kerf 持有、
    /// kerf 释放——外部域 Box<[u8]> 纪律）。
    pub fn from_host_ptr(addr: *mut u8, size: usize) -> Rc<Self> {
        Rc::new(ExternalToken {
            kind: TokenKind::CPointer,
            addr,
            size,
            state: Cell::new(TokenState::Valid),
        })
    }

    /// 种类。
    pub fn kind(&self) -> TokenKind {
        self.kind
    }

    /// 外部域地址（Opaque 观测面/测试用）。
    pub fn addr(&self) -> *mut u8 {
        self.addr
    }

    /// 分配长度。
    pub fn size(&self) -> usize {
        self.size
    }

    /// 有效性（Invalid = 吸收态）。
    pub fn is_valid(&self) -> bool {
        self.state.get() == TokenState::Valid
    }

    /// 消费：槽级失效标记（全局生效——共享 Rc 的所有绑定同步失效）。
    pub fn mark_invalid(&self) {
        self.state.set(TokenState::Invalid);
    }

    /// 消费外部内存（归还外部域——Box<[u8]> 重建纪律；调用方保证
    /// kind = CPointer 且 Valid 且 addr 非空）。
    fn drop_external(&self) {
        debug_assert!(!self.addr.is_null(), "空令牌走 no-op 路径");
        // 重建胖指针：Box::into_raw(slice) → (addr, len) → 逆向还原
        let fat = std::ptr::slice_from_raw_parts_mut(self.addr, self.size);
        // SAFETY：addr/size 来自 Box::into_raw 的原对（构造纪律——
        // from_boxed_slice/from_host_ptr 均按同型产生）
        unsafe { drop(Box::from_raw(fat)) };
    }
}

// ---------------------------------------------------------------------------
// extern 符号表（ffi-ownership-model §4——扁平符号空间）
// ---------------------------------------------------------------------------

/// 实参/返回编组形状（`ExternalType` 的 VM 消费子集：CInt 值拷贝 /
/// CPointer 窗口借用或令牌传递 / Opaque 令牌传递。CStruct/CFunction
/// = Stage 2 VM 消费子集外——driver 注册面拒绝（诚实收窄）。
/// `FfiCall.return_type` 等类型字段服务静态面（G2 HM 锚——
/// ffi-ownership-model §8）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternKind {
    CInt,
    CPointer,
    Opaque,
}

/// 借用实参（窗口内——C 侧可见形态；生命周期 = 窗口规程步 2-4）。
#[derive(Debug, Clone, Copy)]
pub enum BorrowedArg<'a> {
    /// CInt 实参（值拷贝出界——无生命周期交互）。
    Int(i64),
    /// CPointer(char*) 实参：Str 堆槽 pin + 数据指针借用出界
    /// （窗口借用——步 2 pin / 步 4 unpin 配对）。
    CharBuf(&'a str),
    /// 令牌实参（借用传递——不消费；CPointer/Opaque 令牌均可）。
    Token(&'a ExternalToken),
}

/// 宿主返回（C → kerf 三分法 + NULL 哨兵——ffi-ownership-model §2.1）。
#[derive(Debug, Clone, Copy)]
pub enum HostRet {
    /// CInt 返回（值拷贝——装箱归 kerf 全权）。
    Int(i64),
    /// NULL 指针哨兵（Valid 空令牌——FreeExternal = 合法 no-op）。
    NullPtr,
    /// CPointer 返回（外部域分配——所有权移交 kerf；strdup 类惯例）。
    CPtr(*mut u8, usize),
    /// Opaque 返回（外部持有——kerf 不可释放）。
    Opaque(*mut u8),
}

/// 宿主外部函数（C ABI 调用的宿主侧形态——write_stdout 类；
/// 窗口内 VM 挂起，宿主不触 kerf 堆（§6 case 6 时序裁定））。
pub type HostFn = fn(&[BorrowedArg<'_>]) -> Result<HostRet, RuntimeError>;

/// extern 符号表条目（符号名 → 签名 + 宿主函数——C 头文件的
/// 运行时对应物：签名在声明处（注册面），调用点仅携带符号名）。
#[derive(Clone)]
pub struct ExternEntry {
    /// 实参编组形状（与调用实参正序对齐）。
    pub params: Vec<ExternKind>,
    /// 返回包装形状。
    pub ret: ExternKind,
    /// 宿主函数。
    pub f: HostFn,
}

impl std::fmt::Debug for ExternEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExternEntry({:?} -> {:?})", self.params, self.ret)
    }
}

/// extern 符号表（**扁平符号空间**——独立于用户词法环境；
/// §4：不可被 define/set! 遮蔽，宏卫生不穿透 FFI 符号空间）。
#[derive(Debug, Clone, Default)]
pub struct ExternSymbolTable {
    entries: HashMap<Rc<str>, ExternEntry>,
}

impl ExternSymbolTable {
    /// 空表（fail-closed 基线——未登记符号 = E0012）。
    pub fn new() -> Self {
        ExternSymbolTable {
            entries: HashMap::new(),
        }
    }

    /// 登记外部函数（签名声明 + 宿主函数）。
    pub fn register(&mut self, name: &str, params: Vec<ExternKind>, ret: ExternKind, f: HostFn) {
        self.entries
            .insert(Rc::from(name), ExternEntry { params, ret, f });
    }

    /// 符号解析（E11 路径——`None` = 未登记，E0012）。
    pub fn resolve(&self, name: &str) -> Option<&ExternEntry> {
        self.entries.get(name)
    }

    /// 已登记符号数（测试/审计观测面）。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ---------------------------------------------------------------------------
// 窗口规程执行器（ffi-ownership-model §2.1 步 2-4——边界包装层）
// ---------------------------------------------------------------------------

/// E0010 构造助手（令牌失效后使用——双释/用后传递/用后使用）。
fn e10(context: &str, span: Span) -> VmError {
    VmError::new_code(10, crate::messages::err_ffi_token_invalid(context), span)
}

/// E0011 构造助手（所有权/类型违规——释放 Opaque/非令牌值/实参
/// 形状不匹配）。
fn e11(detail: &str, span: Span) -> VmError {
    VmError::new_code(11, crate::messages::err_ffi_ownership(detail), span)
}

/// E0012 构造助手（符号解析失败——extern 符号表未登记）。
fn e12(name: &str, span: Span) -> VmError {
    VmError::new_code(12, crate::messages::err_ffi_symbol_resolution(name), span)
}

/// 宿主错误 → VmError（E0004 运行时通用族——06 §3 E7 I/O 通道
/// 失败口径的运行时承载；code None 经 driver `from_vm` 映射）。
fn host_err(e: RuntimeError, span: Span) -> VmError {
    VmError::new(e.message, span)
}

/// pin 下溢 → VmError（E8 口径——结构配对保证不可达；E0004 通道
/// 承载 + 消息携带 E8 注记）。
fn underflow_err(e: RuntimeError, span: Span) -> VmError {
    VmError::new(e.message, span)
}

/// CallExternal 窗口规程（步 2-4；步 1 实参求值由先前指令完成——
/// `args` 正序传入）。
///
/// 步序（§2.1）：
/// 1.（外部完成）实参求值——求值期间对象靠 VM 栈根存活（根集来源 1）；
/// 2. 装载边界：堆实参（Str char* 路径）装箱堆槽逐个 pin（`Φ[v] += 1`）；
///    令牌实参校验有效性（Invalid → E0010）；
/// 3. 宿主调用（VM 挂起——§6 case 6：无安全点轮询、外部 malloc 不进
///    kerf 分配计数）；
/// 4. 返回：**先 unpin 全部堆实参**（`Φ[v] -= 1`——case 6：返回包装
///    不依赖实参堆对象且触发点在 unpin 之后），再按返回形状包装。
pub fn call_external(
    table: &ExternSymbolTable,
    name: &str,
    args: &[Value],
    heap: &mut Heap,
    span: Span,
) -> Result<Value, VmError> {
    // 符号解析（E11——fail-closed：未登记 = E0012）
    let entry = table.resolve(name).ok_or_else(|| e12(name, span))?;
    // 实参数校验（形状违规 → E0011）
    if args.len() != entry.params.len() {
        return Err(e11(
            &format!(
                "外部函数 {} 实参数不匹配（{} ≠ {}）",
                name,
                args.len(),
                entry.params.len()
            ),
            span,
        ));
    }
    // 步 2：装载边界——实参编组 + pin 簿记
    let mut borrowed: Vec<BorrowedArg<'_>> = Vec::with_capacity(args.len());
    let mut pinned: Vec<GcRef> = Vec::new();
    for (i, (kind, arg)) in entry.params.iter().zip(args.iter()).enumerate() {
        match (kind, arg) {
            (ExternKind::CInt, Value::Int(v)) => borrowed.push(BorrowedArg::Int(*v)),
            (ExternKind::CInt, other) => {
                return Err(e11(
                    &format!(
                        "外部函数 {} 第 {} 实参需要 CInt，实际 {}",
                        name,
                        i + 1,
                        other.type_name()
                    ),
                    span,
                ))
            }
            // char* 窗口借用路径（§8 批次 I 行——Str 堆槽 pin + 数据
            // 指针借用出界 + 返回后 unpin）：Str 值装箱堆槽（GC 追踪
            // 形态）→ pin（Φ+1，F-PIN 存活保证）→ 借用 Rc<str> 缓冲
            //（栈根存活 + 堆槽 pin 双保险——防御性协议定位 §3）
            (ExternKind::CPointer, Value::Str(s)) => {
                let slot = heap.alloc_str(Rc::clone(s));
                heap.pin_object(slot);
                pinned.push(slot);
                borrowed.push(BorrowedArg::CharBuf(s));
            }
            // 令牌实参（借用传递——不消费；Invalid 令牌传参 = E9）
            (ExternKind::CPointer, Value::External(t))
            | (ExternKind::Opaque, Value::External(t)) => {
                if !t.is_valid() {
                    return Err(e10("令牌失效后传递（用后传递）", span));
                }
                borrowed.push(BorrowedArg::Token(t));
            }
            (ExternKind::CPointer, other) => {
                return Err(e11(
                    &format!(
                        "外部函数 {} 第 {} 实参需要 CPointer（str 窗口借用或令牌），实际 {}",
                        name,
                        i + 1,
                        other.type_name()
                    ),
                    span,
                ))
            }
            (ExternKind::Opaque, other) => {
                return Err(e11(
                    &format!(
                        "外部函数 {} 第 {} 实参需要 Opaque 令牌，实际 {}",
                        name,
                        i + 1,
                        other.type_name()
                    ),
                    span,
                ))
            }
        }
    }
    // 错误路径的 pin 泄漏：装载边界中途失败（E0010/E0011）时已 pin
    // 槽位不 unpin——按 §6 case 9 容忍口径（泄漏容忍、误回收零容忍；
    // 永久根形态）。
    // 步 3：宿主调用（VM 挂起——借用窗口内宿主只观测借用实参）
    let ret = (entry.f)(&borrowed).map_err(|e| host_err(e, span))?;
    // 步 4a：unpin 全部堆实参（逆序 LIFO——结构配对；窗口关闭）
    for slot in pinned.iter().rev() {
        heap.unpin_object(*slot)
            .map_err(|e| underflow_err(e, span))?;
    }
    // 步 4b：返回包装（不依赖实参堆对象——case 6 时序）
    Ok(match (entry.ret, ret) {
        (ExternKind::CInt, HostRet::Int(v)) => Value::Int(v),
        (ExternKind::CPointer, HostRet::NullPtr) => Value::External(ExternalToken::null_sentinel()),
        (ExternKind::CPointer, HostRet::CPtr(addr, size)) => {
            Value::External(ExternalToken::from_host_ptr(addr, size))
        }
        (ExternKind::Opaque, HostRet::Opaque(addr)) => Value::External(ExternalToken::opaque(addr)),
        // 声明形状与宿主返回形态不匹配 = 注册面契约违规（内部缺陷口径）
        (k, r) => {
            return Err(VmError::new(
                format!(
                    "外部函数 {} 返回形态与声明不匹配（{:?} vs {:?}）——注册面契约违规",
                    name, k, r
                ),
                span,
            ))
        }
    })
}

/// AllocExternal 执行（外部域分配——不经过 GC；产出 CPointer 令牌）。
///
/// `size = 0`：编译期拒绝的**运行期纵深防御**（§6 case 4——操作码
/// 直接构造绕过 lowering 面时的兜底；报错 > 静默，§2.3-4）。
pub fn alloc_external(size: u32, span: Span) -> Result<Value, VmError> {
    if size == 0 {
        return Err(e11(
            "AllocExternal 尺寸为 0（零尺寸缓冲在 kerf 请求面无合法用例）",
            span,
        ));
    }
    let buf = vec![0u8; size as usize].into_boxed_slice();
    Ok(Value::External(ExternalToken::from_boxed_slice(buf)))
}

/// FreeExternal 执行（消费语义——§2.3）。
///
/// 判定序（§5 非法迁移表）：非 Foreign 值 → E0011（迁移 5）；
/// Opaque 令牌 → E0011（迁移 4——外部持有不可释放）；Invalid →
/// E0010（迁移 1——双重释放，诊断吸收非 UB）；Valid CPointer →
/// 消费（槽级失效全局标记）+ 外部内存归还；NULL 哨兵 → 合法 no-op
/// （§6 case 13——对齐 `free(NULL)` 定义良好）。
pub fn free_external(ptr: &Value, span: Span) -> Result<Value, VmError> {
    let token = match ptr {
        Value::External(t) => t,
        other => {
            return Err(e11(
                &format!("FreeExternal 需要 Foreign 令牌，实际 {}", other.type_name()),
                span,
            ))
        }
    };
    if token.kind() == TokenKind::Opaque {
        return Err(e11(
            "Opaque 令牌为外部持有（kerf 侧无释放权——fclose/fopen 类外部 API 约定）",
            span,
        ));
    }
    if !token.is_valid() {
        return Err(e10("双重释放（令牌已消费——诊断吸收，非 UB）", span));
    }
    // 消费：槽级失效标记（全局生效——所有共享绑定同步失效）
    token.mark_invalid();
    if token.addr().is_null() {
        // NULL 哨兵：合法 no-op（§6 case 13）
        return Ok(Value::Unit);
    }
    token.drop_external();
    Ok(Value::Unit)
}
