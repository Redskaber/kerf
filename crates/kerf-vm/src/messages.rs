//! 运行时错误消息单源构造器（TD-018 偿还：双路径文本分裂统一）。
//!
//! **背景**：同一语义错误（if 条件位非 bool 等）的文本常量曾分散于
//! `vm.rs` / `eval.rs` / `builtins.rs` 三处且互有出入（「条件位置需要
//! bool」vs「if 条件需要 bool」）——E 码/阶段/Span 一致而文本分裂。
//!
//! **本模块**：kerf-vm 公共层（TD-018 登记偿还方案的既定位置）——
//! 同族消息在此单源构造，三个消费面（VM 条件跳转 / eval 参考实现 /
//! driver 内置函数）同源引用，此后文本演进只改一处。
//!
//! **归因口径**：`if 条件` 前缀与静态面（typecheck R1 诊断）一致——
//! `and`/`or`/`when`/`unless` 等条件糖均脱糖为 `if`，归因到 `if`
//! 形式是唯一稳定口径（§7.3 名字唯一稳定口径的运行时镜像）。

/// if 条件位类型错误（VM `JumpIfFalse` / eval if 臂共用）。
pub fn err_if_cond_bool(actual: &str) -> String {
    format!("if 条件需要 bool，实际 {actual}（truthy 语义显式定义）")
}

/// `not` 操作数类型错误（VM `Not` 操作码 / driver `not` 内置共用）。
pub fn err_not_bool(actual: &str) -> String {
    format!("not 需要 bool，实际 {actual}")
}

/// `set!` 未绑定（VM `SetGlobal` / eval set! 臂共用——两路径既有同文，单源化维持）。
pub fn err_setbang_unbound() -> String {
    "assign 未绑定变量".to_string()
}

/// `car`/`cdr` 非序对操作数（VM 操作码 / driver 内置共用——两路径既有同文）。
pub fn err_pair_op(op: &str, actual: &str) -> String {
    format!("{op} 需要 pair，实际 {actual}")
}

/// E0007：效应未处理逃逸到顶层（r25/42-f——effect-language-design D9；
/// VM `Perform` 扫描无匹配 handler 帧时构造）。
pub fn err_effect_unhandled(tag: &str) -> String {
    format!("效应 '{tag}' 未被任何 handler 处理（逃逸到顶层）")
}

/// E0008：continuation 二次恢复（线性唯一性违反——D3/D9；含首次
/// 恢复位置追踪，VM `Call`/`TailCall` 的 Continuation 臂构造）。
pub fn err_continuation_resumed_twice(first: &str) -> String {
    format!("continuation 二次恢复（线性唯一性——首次恢复于 {first}）")
}

/// E0009：resume 调用元数错（continuation 调用恰一实参——D4 调用
/// 形态的元数面；非 continuation 值被调用的类型面归 E0004 通用族，
/// 见 effect-language-design v1.1 执行注记）。
pub fn err_resume_arity(n: usize) -> String {
    format!("resume 恰接受一个值（continuation 调用实参数 {n}）")
}

/// E0010：FFI 令牌失效后使用（r30/48-d——ffi-ownership-model E9：
/// 双重释放/用后传递/用后使用——消费全局生效后的吸收态）。
pub fn err_ffi_token_invalid(context: &str) -> String {
    format!("FFI 令牌已失效（{context}）")
}

/// E0011：FFI 所有权/类型违规（ffi-ownership-model E10——释放
/// Opaque 令牌/释放非令牌值/实参形状不匹配/零尺寸防御）。
pub fn err_ffi_ownership(detail: &str) -> String {
    format!("FFI 所有权违规（{detail}）")
}

/// E0012：FFI 符号解析失败（ffi-ownership-model E11——extern 符号
/// 表未登记；QBE AOT 路径的链接期符号解析在 VM 路径的调用期对应
/// 物——双路径一致 fail-closed）。
pub fn err_ffi_symbol_resolution(name: &str) -> String {
    format!("外部符号 '{name}' 未在 extern 符号表登记（符号解析失败）")
}
