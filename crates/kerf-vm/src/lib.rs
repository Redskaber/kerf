//! # kerf-vm
//!
//! **五正交轴定位（lang-design 15 §5.3，v6.0）**：轴 3/4（效应/能力）的运行时消费面——字节码执行 + 双路径 eval 互查（语义验证，§21.8 Phase 1）。
//!
//! 双执行路径（stage0.md §21.8 Phase 1：eval 与 VM 互查验证 9 原语语义）：
//! - **元循环求值器**（`eval`）：`CoreExpr` 树走查 + `Rc` 环境链
//!   （§8.4，参考语义——Stage 2 起被编译器替换，Stage 3+ 移除）；
//! - **字节码 VM**（`vm`）：switch-dispatch 执行循环 + 三扩展槽帧
//!   （§8.12 / §19.5，生产路径）。
//!
//! **共享值模型**：`Value`（§8.4/§8.5）——即时值（Nil/Bool/Int/Float/Str）
//! + 堆引用（Pair，经 kerf-runtime GC 管理）+ 闭包 + 内置函数。
//!
//! **truthy 语义显式定义**（§19.5 陷阱 3）：仅 `Bool` 参与条件跳转，
//! 其余类型报 TypeMismatch（避免 JavaScript 式隐式转换的语义泥潭）。

pub mod eval;
pub mod value;
pub mod vm;

// 显式 re-export（§10.1 规则 4：禁止 glob re-export）。
// 约定：本 crate 暴露执行层全部公共类型与入口。
pub use eval::{apply_value, eval_expr, eval_program, Env, EvalError};
pub use value::{render_value, BuiltinFn, ClosureValue, Value};
pub use vm::{box_value, unbox_slot};
pub use vm::{
    call_closure, run_program, run_program_with_budget, Frame, FrameExt, Rt, TraceFrame, VmError,
};
