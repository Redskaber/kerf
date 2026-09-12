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
//! - **终止**：HALT；
//! - **效应（r25/42-f）**：INSTALL_HANDLER / PERFORM；
//! - **FFI（r30/48-d）**：CALL_EXTERNAL / ALLOC_EXTERNAL / FREE_EXTERNAL
//!   （IR/VM 面做实——语言面形式 Stage 3，编译臂不发射；ffi-ownership-model
//!   §2/§8 窗口规程与三原语的字节码面）。

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
    /// **尾调用（TD-022/H2——TCO）**：语义同 Call，但帧复用——被调方
    /// 返回直达当前帧的调用者（当前帧拆除，帧数净零）。编译器仅在
    /// 函数体尾位（Lambda 体 / If 两臂 / Begin 末项）发射。
    TailCall(u32),
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

    // ---- 效应（r25/42-f——effect-language-design D6/D8）----
    /// 安装 handler 并调用被保护体（handle 表达式全部帧编排）。
    ///
    /// 从当前帧取 handler/body 两原型的捕获单元 → ① 压 handler 帧
    /// （`trampoline` 哨兵原型——code=[Ret]；ext1 = HandlerFrame{
    /// tag, handler 原型, 捕获, 数据栈水位}；ret = (当前原型, 本指令
    /// 之后)）→ ② 压 body thunk 帧（ret = (trampoline, 0)）→ ③ 转移
    /// 执行 body。body RET 经 trampoline 弹 handler 帧回到本指令之后
    /// （R11-return）；body 内尾调用拆 thunk 帧穿透（D8：TCO 与浅
    /// 处理正交——handler 帧保留在下方）；perform 上抛扫描 ext1
    /// 匹配分派（R11-dispatch，D6）。
    InstallHandler {
        handler: u32,
        body: u32,
        /// 分派标签（常量池 SymLit 索引——符号值）。
        tag: u32,
        trampoline: u32,
    },
    /// 效应上抛（R10-perform）：弹出效应值（须为 `(tag . payload)`
    /// 点对——tag = 符号值）→ 从帧栈顶向下扫描匹配 ext1 的最近
    /// handler 帧 → 快照挂起 continuation（挂起帧链 + 数据栈 +
    /// 恢复点）→ 拆帧到 handler 边界 → handler 帧变形为 handler 体
    /// 执行帧（locals = [payload, κ]，ext1 清除——浅处理一次）→
    /// 数据栈截回安装水位。未匹配 = E0007（逃逸到顶层）。
    Perform,

    // ---- FFI（r30/48-d——ffi-ownership-model §2/§8；IR/VM 面做实，
    // 语言面形式 Stage 3（plan §5b 批次 J 排程注 3））----
    /// 调用外部函数（符号经 extern 符号表按名解析——E0012 fail-closed；
    /// `symbol` = 常量池 SymLit 索引（extern 符号名——扁平符号空间，
    /// 独立于用户词法环境，不可被 define/set! 遮蔽）。
    /// 窗口规程（ffi-ownership-model §2.1 步 2-4，边界包装层执行）：
    /// 实参已按序压栈（步 1 由先前指令完成）→ 装载边界（堆实参 pin
    /// Φ[v]+=1 + 令牌实参校验有效性，Invalid = E0010）→ 宿主调用
    /// （VM 挂起）→ 返回包装 + 全部堆实参 unpin（Φ[v]-=1）。
    CallExternal {
        symbol: u32,
        n_args: u32,
    },
    /// 分配外部内存（外部 malloc 域——**不经过 GC**；产出 CPointer
    /// 令牌）。`size = 0` 编译期拒绝（lowering 面）+ 运行期防御
    /// 拒绝（E0011）——防御纵深（§6 case 4）。
    AllocExternal {
        size: u32,
    },
    /// 释放外部内存（消费语义：槽级失效标记全局生效；Invalid 再释放
    /// = E0010 双重释放；Opaque 令牌 = E0011；非令牌值 = E0011）。
    FreeExternal,
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
            Op::TailCall(_) => "TAIL_CALL",
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
            Op::InstallHandler { .. } => "INSTALL_HANDLER",
            Op::Perform => "PERFORM",
            Op::CallExternal { .. } => "CALL_EXTERNAL",
            Op::AllocExternal { .. } => "ALLOC_EXTERNAL",
            Op::FreeExternal => "FREE_EXTERNAL",
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
            Op::InstallHandler {
                handler,
                body,
                tag,
                trampoline,
            } => {
                format!(
                    " handler={} body={} tag={} trampoline={}",
                    handler, body, tag, trampoline
                )
            }
            Op::CallExternal { symbol, n_args } => {
                format!(" symbol={} n_args={}", symbol, n_args)
            }
            Op::AllocExternal { size } => format!(" size={}", size),
            // _ 臂理由：无操作数指令（PushNil/Pop/Dup/Ret/算术/比较/谓词/Car/Cdr/Halt/Perform/FreeExternal 等）——无操作数后缀
            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_count_matches_spec() {
        // 冻结契约：46 个操作码（04-bytecode-vm §1 全枚举对齐）。
        // 逐一列举以保证计数稳定——新增/删除任何变体都必须同步
        // 04 文档与本清单（双向冻结：enum ↔ 测试 ↔ 文档三方一致）。
        // r21/42-b 修正：TailCall（r18/40-c TCO 引入）此前漏列于本
        // 清单——三方冻结漂移按 R4（代码为准 + 本次修正文档）补齐。
        // r30/48-d 扩：FFI 三指令（43→46，第十组——ffi-ownership-model
        // §2/§8 的字节码面；语言面形式 Stage 3——plan §5b 排程注 3）。
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
            // 函数操作（4：含 TailCall——TD-022/H2 TCO，r18）
            Op::Closure {
                proto: 0,
                n_captures: 0,
            },
            Op::Call(0),
            Op::TailCall(0),
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
            // 效应（2：r25/42-f——effect-language-design D6，原语集 9→11
            // 对应字节码面两指令；04-bytecode-vm 同步冻结）
            Op::InstallHandler {
                handler: 0,
                body: 0,
                tag: 0,
                trampoline: 0,
            },
            Op::Perform,
            // FFI（3：r30/48-d——ffi-ownership-model §2/§8；IR/VM 面
            // 做实，语言面形式 Stage 3；04 文档与 compiler.krf OP-* 表
            // 三方同步）
            Op::CallExternal {
                symbol: 0,
                n_args: 0,
            },
            Op::AllocExternal { size: 0 },
            Op::FreeExternal,
        ];
        assert_eq!(
            ops.len(),
            46,
            "操作码总数（r30/48-d FFI 面 43→46——04 文档与 compiler.krf OP-* 表三方同步）"
        );
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
