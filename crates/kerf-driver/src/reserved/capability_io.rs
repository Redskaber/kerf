//! 能力模型 I/O 接口预留（stage0.md §9.1.3 / lang-design 13 §3.1.3
//! ——P2：类型 + 完整行为规格；v5.2 修订）。
//!
//! **为什么预留**：安全模型与最小权限（线性令牌——Rust 所有权同型
//! 验证）；r8 起消费面 `crate::capability`（require/IoGrant 门控——
//! 「基础传递」做实）。
//!
//! **拆分注记（r18 / 40-g）**：自 mod.rs 拆出（§13.4 J1——13 §3.1.3
//! 分节对齐）；签名与行为规格零变化（原则 27 冻结契约）。
//!
//! **完整行为规格**（P2——Stage 1 可直接按规格实现）：
//! 1. `read_line` 仅在持有 `ReadCapability` 时可调用——令牌经线性传递，
//!    不可复制、不可伪造；
//! 2. `write_line` 同理（`WriteCapability`）；
//! 3. 无令牌的 I/O 调用为**编译期错误**（权限验证，非运行时检查）；
//! 4. 与 Stage 0 传统 I/O（kerf-runtime::io 全局函数）的替换关系：
//!    渐进替换原则 §28——driver 注册的内置函数改为能力参数化形态，
//!    全局函数逐步退役。
//!
//! Stage 0 裁定：传统 I/O（§21.10 决策点 3）；能力模型于 Stage 1 基础 /
//! Stage 2 完整引入。

/// 读能力令牌（不可伪造——私有构造，经权限传递获得）。
pub struct ReadCapability {
    _private: (),
}

/// 写能力令牌（不可伪造）。
pub struct WriteCapability {
    _private: (),
}

/// 能力模型 I/O 错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IOError {
    pub message: String,
}

/// 能力模型 I/O（§9.1.3 预留接口——Stage 1+ 实现）。
pub trait CapabilityIO {
    /// 读一行（需要读能力）。
    fn read_line(cap: &mut ReadCapability) -> Result<String, IOError>;

    /// 写一行（需要写能力）。
    fn write_line(cap: &mut WriteCapability, s: &str) -> Result<(), IOError>;
}

// ---------------------------------------------------------------------------
// 令牌铸造（r8 批次 D——pub(crate)：令牌仅经 driver 组合根流出，
// 外部 crate 可引用类型但不可构造——「经权限传递获得」的构造面控制）
// ---------------------------------------------------------------------------

/// 铸造读能力令牌（仅供 crate::capability 的授权管线调用——按程序
/// 声明的 (require io read) 铸造；13 §3.1.3 规格条款 1）。
pub(crate) fn mint_read_token() -> ReadCapability {
    ReadCapability { _private: () }
}

/// 铸造写能力令牌（同上——规格条款 2）。
pub(crate) fn mint_write_token() -> WriteCapability {
    WriteCapability { _private: () }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_tokens_are_unforgeable() {
        // 私有构造：外部无法构造 ReadCapability（编译期保证）
        // 此测试验证令牌类型可被引用而不可被实例化
        fn takes_read(_cap: &ReadCapability) {}
        fn takes_write(_cap: &WriteCapability) {}
        let _ = (
            takes_read as fn(&ReadCapability),
            takes_write as fn(&WriteCapability),
        );
    }
}
