//! kerf-backend：本地码后端（批次 G / 38-b-c——G1 QBE PoC 交付）。
//!
//! **分层定位**（15-架构分层 / 13 §3.3.7）：后端层——消费前端产物
//! （`CoreExpr`），产出目标码（QBE IL → 汇编 → 本地可执行）。
//!
//! **模块图**：
//! - [`codegen`]：多目标后端契约（自 `kerf-driver/reserved/codegen` 迁入
//!   的正式家——签名冻结不变，原则 27）；
//! - [`anf`]：AnnotatedANF IR 实化 + `CoreExpr` lowering（块式 ANF——
//!   值上下文 if 经块参数 merge）；
//! - [`qbe`]：IL 生成 + [`qbe::QbeBackend`]（CodegenBackend 首个做实实现）；
//! - [`aot`]：外部进程编排（qbe/cc 子进程链——永不进入自举链 §21.2）。
//!
//! **PoC 边界（B1 登记）**：整数域原语/比较/if/直接调用/递归；闭包、
//! Float/Str/Pair、set!/module、print/IO、函数值一等均边界外（显式
//! 错误——批次 H/I 按 38-a FFI 所有权模型与 GC-后端协同承接）。

pub mod anf;
pub mod aot;
pub mod codegen;
pub mod qbe;

pub use anf::{
    lower_program, AAtom, ABlock, ABody, ACtrl, AFuncDef, ALabel, AOp, APhi, APrim, AStmt, ATemp,
    AnnotatedANF, LowerError,
};
pub use aot::{build_native, find_qbe, run_native, NativeArtifacts};
pub use codegen::{
    CodegenBackend, CodegenError, CompiledModule, PassDescriptor, TargetTriple, WasmBackend,
    WasmComponent,
};
pub use qbe::{gen_il, QbeBackend};
