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
    "set! 未绑定变量".to_string()
}

/// `car`/`cdr` 非序对操作数（VM 操作码 / driver 内置共用——两路径既有同文）。
pub fn err_pair_op(op: &str, actual: &str) -> String {
    format!("{op} 需要 pair，实际 {actual}")
}
