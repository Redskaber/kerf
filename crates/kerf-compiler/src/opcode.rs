//! 字节码操作码（stage0.md §8.12：约 35+ 个操作码，按类分组）。
//!
//! 分组（§8.12）：
//! - **栈操作**：PUSH_CONST / PUSH_NIL / PUSH_TRUE / PUSH_FALSE / POP / DUP / SWAP；
//! - **函数操作**：CLOSURE / CALL / RET；
//! - **控制流**：JUMP / JUMP_IF_FALSE；
//! - **数据构造**：MAKE_PAIR / CAR / CDR；
//! - **变量访问**：LOAD_LOCAL / STORE_LOCAL / LOAD_GLOBAL / STORE_GLOBAL /
//!   DEFINE_GLOBAL / LOAD_CAPTURED / STORE_CAPTURED（闭包捕获转换，
//!   Stage 0 扩展；DEFINE_GLOBAL 为 D1/E6 语义的冻结契约）；
//! - **算术与比较**：ADD / SUB / MUL / DIV / MOD / NUM_LT / NUM_GT / NUM_LE /
//!   NUM_GE / NUM_EQ / EQ / NOT；
//! - **谓词**：IS_NULL / IS_PAIR / IS_INT / IS_BOOL / IS_PROCEDURE；
//! - **终止**：HALT。

/// 操作码（Debug 渲染 + PartialEq 供同结果测试比对）。
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    // ---- 栈操作 ----
    /// 压入常量池 `k` 中的常量。
    PushConst(u32),
    /// 压入 nil。
    PushNil,
    /// 压入 true。
    PushTrue,
    /// 压入 false。
    PushFalse,
    /// 弹出并丢弃栈顶。
    Pop,
    /// 复制栈顶。
    Dup,
    /// 交换栈顶两项。
    Swap,

    // ---- 变量访问 ----
    /// 读取当前帧局部槽 `i`。
    LoadLocal(u32),
    /// 写入当前帧局部槽 `i`。
    StoreLocal(u32),
    /// 按常量池符号索引读取全局。
    LoadGlobal(u32),
    /// 按常量池符号索引写入全局。
    StoreGlobal(u32),
    /// 按常量池符号索引定义全局（D1/E6 语义：不存在才写入；
    /// 已存在报「重复定义变量」——与 eval 路径 `Env::define` 对齐，
    /// T1 定理要求两条路径的全局存储语义一致）。
    DefineGlobal(u32),
    /// 读取当前闭包捕获槽 `i`。
    LoadCaptured(u32),
    /// 写入当前闭包捕获槽 `i`（共享可变单元——词法闭包语义）。
    StoreCaptured(u32),

    // ---- 控制流 ----
    /// 无条件跳转到 `t`。
    Jump(u32),
    /// 栈顶为假值时跳转到 `t`（truthy 语义显式定义：仅 Bool 参与，
    /// 其余类型报 TypeMismatch——§19.5 陷阱 3）。
    JumpIfFalse(u32),

    // ---- 函数操作 ----
    /// 从原型 `proto` 构造闭包，弹出 `n_captures` 个捕获值。
    Closure {
        proto: u32,
        n_captures: u32,
    },
    /// 调用栈顶函数，`n` 个实参已按序压栈（被调者在栈顶第 n 项）。
    Call(u32),
    /// 返回：弹出返回值，恢复调用帧。
    Ret,

    // ---- 算术与比较 ----
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    NumLt,
    NumGt,
    NumLe,
    NumGe,
    NumEq,
    /// 通用相等（即时值按值、堆值按引用）。
    Eq,
    /// 逻辑非（仅 Bool）。
    Not,

    // ---- 数据构造 ----
    /// 弹出 (b, a) 构造点对 (a . b)（堆分配）。
    MakePair,
    /// 取点对左部。
    Car,
    /// 取点对右部。
    Cdr,

    // ---- 谓词 ----
    IsNull,
    IsPair,
    IsInt,
    IsBool,
    IsProcedure,

    // ---- 终止 ----
    /// 终止：弹出最终结果值。
    Halt,
}

impl Op {
    /// 指令名称（反汇编渲染）。
    pub fn name(&self) -> &'static str {
        match self {
            Op::PushConst(_) => "PUSH_CONST",
            Op::PushNil => "PUSH_NIL",
            Op::PushTrue => "PUSH_TRUE",
            Op::PushFalse => "PUSH_FALSE",
            Op::Pop => "POP",
            Op::Dup => "DUP",
            Op::Swap => "SWAP",
            Op::LoadLocal(_) => "LOAD_LOCAL",
            Op::StoreLocal(_) => "STORE_LOCAL",
            Op::LoadGlobal(_) => "LOAD_GLOBAL",
            Op::StoreGlobal(_) => "STORE_GLOBAL",
            Op::DefineGlobal(_) => "DEFINE_GLOBAL",
            Op::LoadCaptured(_) => "LOAD_CAPTURED",
            Op::StoreCaptured(_) => "STORE_CAPTURED",
            Op::Jump(_) => "JUMP",
            Op::JumpIfFalse(_) => "JUMP_IF_FALSE",
            Op::Closure { .. } => "CLOSURE",
            Op::Call(_) => "CALL",
            Op::Ret => "RET",
            Op::Add => "ADD",
            Op::Sub => "SUB",
            Op::Mul => "MUL",
            Op::Div => "DIV",
            Op::Mod => "MOD",
            Op::NumLt => "NUM_LT",
            Op::NumGt => "NUM_GT",
            Op::NumLe => "NUM_LE",
            Op::NumGe => "NUM_GE",
            Op::NumEq => "NUM_EQ",
            Op::Eq => "EQ",
            Op::Not => "NOT",
            Op::MakePair => "MAKE_PAIR",
            Op::Car => "CAR",
            Op::Cdr => "CDR",
            Op::IsNull => "IS_NULL",
            Op::IsPair => "IS_PAIR",
            Op::IsInt => "IS_INT",
            Op::IsBool => "IS_BOOL",
            Op::IsProcedure => "IS_PROCEDURE",
            Op::Halt => "HALT",
        }
    }

    /// 操作数渲染（反汇编）。
    pub fn render_operands(&self) -> String {
        match self {
            Op::PushConst(k)
            | Op::LoadLocal(k)
            | Op::StoreLocal(k)
            | Op::LoadGlobal(k)
            | Op::StoreGlobal(k)
            | Op::DefineGlobal(k)
            | Op::LoadCaptured(k)
            | Op::StoreCaptured(k)
            | Op::Jump(k)
            | Op::JumpIfFalse(k)
            | Op::Call(k) => format!(" {}", k),
            Op::Closure { proto, n_captures } => {
                format!(" proto={} captures={}", proto, n_captures)
            }
            // _ 臂理由：无操作数指令（PushNil/Pop/Dup/Ret/算术/比较/谓词/Car/Cdr/Halt 等）——无操作数后缀
            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_count_matches_spec() {
        // 冻结契约：40 个操作码（04-bytecode-vm §1 全枚举对齐）。
        // 逐一列举以保证计数稳定——新增/删除任何变体都必须同步
        // 04 文档与本清单（双向冻结：enum ↔ 测试 ↔ 文档三方一致）。
        let ops = [
            // 栈操作（7）
            Op::PushConst(0),
            Op::PushNil,
            Op::PushTrue,
            Op::PushFalse,
            Op::Pop,
            Op::Dup,
            Op::Swap,
            // 变量访问（7）
            Op::LoadLocal(0),
            Op::StoreLocal(0),
            Op::LoadGlobal(0),
            Op::StoreGlobal(0),
            Op::DefineGlobal(0),
            Op::LoadCaptured(0),
            Op::StoreCaptured(0),
            // 控制流（2）
            Op::Jump(0),
            Op::JumpIfFalse(0),
            // 函数操作（3）
            Op::Closure {
                proto: 0,
                n_captures: 0,
            },
            Op::Call(0),
            Op::Ret,
            // 算术与比较（12）
            Op::Add,
            Op::Sub,
            Op::Mul,
            Op::Div,
            Op::Mod,
            Op::NumLt,
            Op::NumGt,
            Op::NumLe,
            Op::NumGe,
            Op::NumEq,
            Op::Eq,
            Op::Not,
            // 数据构造（3）
            Op::MakePair,
            Op::Car,
            Op::Cdr,
            // 谓词（5）
            Op::IsNull,
            Op::IsPair,
            Op::IsInt,
            Op::IsBool,
            Op::IsProcedure,
            // 终止（1）
            Op::Halt,
        ];
        assert_eq!(ops.len(), 40, "操作码总数（Stage 0 冻结契约）");
    }

    #[test]
    fn render_shapes() {
        assert_eq!(Op::Call(3).name(), "CALL");
        assert_eq!(Op::Call(3).render_operands(), " 3");
        assert_eq!(
            Op::Closure {
                proto: 1,
                n_captures: 2
            }
            .render_operands(),
            " proto=1 captures=2"
        );
        assert_eq!(Op::Add.render_operands(), "");
    }
}
