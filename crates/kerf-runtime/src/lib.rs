//! # kerf-runtime
//!
//! **五正交轴定位（lang-design 15 §5.3，v6.0）**：轴 3/4 的通道层——最小 I/O 与 GC 基座（Layer 0 无依赖；能力门控的运行时通道面）。
//!
//! 运行时基座（Layer 0，**无 crate 依赖**——sop.md §2.4.5 层级依赖规则）：
//! GC 堆（标记-清除）+ 最小 I/O + foreign ref 登记。
//!
//! **能力模型**（§8.11 + §19.4）：
//! - bump 风格槽位分配（`alloc_pair`）；
//! - 显式工作栈标记（防递归爆栈，§19.4 陷阱 1）；
//! - 线性堆遍历清除（free list）；
//! - 根集：`RootSet`（由 VM 在安全点收集——§19.4 陷阱 2：GC 只在指令边界触发）。
//!
//! **核心不变式**（§19.4）：
//! 1. 根集完备性：四类根（VM 栈/调用栈局部/全局环境/foreign ref）缺一不可；
//! 2. 标记位复位对称：`clear_marks` 遍历全部对象（含刚回收的）；
//! 3. 分配零内卷：分配路径不触发任何可能再分配的操作。
//!
//! **Stage 0 堆语义**（文档化决策）：GC 管理序对图（`HeapObj::Pair`）；
//! 字符串经 `Rc<str>` 引用计数（无内部堆引用，无需跟踪）；
//! 分代/压缩推迟到 Stage 2（TD-008）。

pub mod gc;
pub mod heap;
pub mod io;

// 显式 re-export（§10.1 规则 4：禁止 glob re-export）。
// 约定：本 crate 暴露运行时基座全部公共类型与入口。
pub use gc::{mark_sweep_cycle, RootSet};
pub use heap::GcStats;
pub use heap::{BoxedInput, GcRef, Heap, HeapObj, ValueSlot};
pub use io::{read_line_stdin, write_line_stdout, write_stdout, RuntimeError};
