//! # kerf-compiler
//!
//! **五正交轴定位（lang-design 15 §5.3，v6.0）**：轴 2（语法/类型）桥接点——typecheck 独立推导不回流 `CoreExpr`（15 §5.1 Layer 2 正交性）+ 字节码生成。
//!
//! CoreExpr → 字节码的直译式编译（stage0.md §8.12 / §19.3）。
//!
//! **编译骨架**（§19.3）：
//! - 跳转回填：`emit_jump` 占位 → `patch_jump` 回填（`finish` 断言占位清空）；
//! - 常量池：`intern_const` 按值哈希去重（陷阱：常量池去重）；
//! - 闭包捕获转换：Lambda 的自由变量解析为捕获向量（LOAD/STORE_CAPTURED）；
//! - debug_info_table：每条指令 (pc, Span) 映射——VM 错误反查源码（§8.12）。
//!
//! **核心不变式**（§19.3）：
//! 1. 栈平衡：任何 CoreExpr 的编译产物执行前后，操作数栈净变化恰好 +1；
//! 2. 回填完备：CodeBuf 完成时占位队列必须为空（断言）；
//! 3. 调试信息全覆盖：每条指令都有 Span，无裸指令。
//!
//! **操作码集**（§8.12，约 39 个）：栈操作 / 函数 / 控制流 / 数据构造 /
//! 变量访问 / 算术与比较 / 谓词 / 终止。

pub mod bytecode;
pub mod opcode;

// 显式 re-export（§10.1 规则 4：禁止 glob re-export）。
// 约定：本 crate 暴露字节码（Bc*）域全部公共类型。
pub use bytecode::{
    disassemble_program, BcConst, BcProgram, BcProto, CaptureSource, DebugInfoTable,
};
pub use opcode::Op;

pub mod compile;

// 编译入口（§10.1 规则 1）单独导出便于路径导入。
pub use compile::{compile_module, CompileCtxt, CompileError};

pub mod typecheck;

// 静态检查域（Tc 前缀——§10.1 域前缀约定）公共类型显式导出。
pub use typecheck::{check_program, BuiltinSig, TcParam, TcParams, TcType, CHECK_DIAG_CODE};
