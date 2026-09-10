//! Span：源码位置的不可变表示（stage0.md §8.6）。
//!
//! rustc 风格，每个 Token / AST 节点 / IR 节点 / 字节码指令都携带 Span：
//! - 任何错误都能精确定位到源码；
//! - 调试器可以反查字节码 → IR → 源码；
//! - 增量编译可以基于 Span 进行细粒度失效。
//!
//! 列号语义（§19.1 实现陷阱 1）：**字节偏移为主键，行/列仅用于诊断渲染**——
//! Span 只存字节偏移，行/列在渲染时由 `SourceMap` 派生。

use core::fmt;

/// 源文件标识（`SourceMap` 中的索引）。
pub type FileId = u32;

/// 字节偏移（主键，见模块文档）。
pub type ByteOffset = u32;

/// 宏展开代次（§19.2 不变式 1：每次宏调用产生的语法对象携带 expansion_id + 1）。
pub type ExpansionId = u32;

/// 源码位置的不可变表示。
///
/// 不变式：`start <= end`；同一 Span 的 file_id 与 expansion_id 不可变。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    /// 所属源文件（`SourceMap` 索引）。
    pub file_id: FileId,
    /// 起始字节偏移（含）。
    pub start: ByteOffset,
    /// 结束字节偏移（不含）。
    pub end: ByteOffset,
    /// 宏展开代次：0 = 用户源码，N = 第 N 代宏展开产物。
    pub expansion_id: ExpansionId,
}

impl Span {
    /// 构造 Span。
    pub fn new(file_id: FileId, start: ByteOffset, end: ByteOffset) -> Self {
        Span {
            file_id,
            start,
            end,
            expansion_id: 0,
        }
    }

    /// 合成 Span（无真实源位置时的占位，例如宏展开产物）。
    ///
    /// 注意：合成产物应尽量改用宏定义处/调用处 Span 拼接（§19.2 陷阱 1），
    /// 「空 Span 会让后续诊断失效」。
    pub fn dummy() -> Self {
        Span::new(0, 0, 0)
    }

    /// 提升展开代次：宏产物携带「展开代次 + 1」（§19.2 不变式 1）。
    pub fn bumped_expansion(self) -> Self {
        Span {
            expansion_id: self.expansion_id.saturating_add(1),
            ..self
        }
    }

    /// 合并两个 Span（取并集；文件或代次不一致时退回 `self`——保守策略，
    /// 保证诊断渲染始终有可用位置）。
    pub fn merge(self, other: Span) -> Span {
        if self.file_id != other.file_id || self.expansion_id != other.expansion_id {
            return self;
        }
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            ..self
        }
    }

    /// Span 是否覆盖给定字节偏移（诊断关联查询用）。
    pub fn contains(self, offset: ByteOffset) -> bool {
        offset >= self.start && offset < self.end
    }

    /// 字节长度。
    pub fn len(self) -> ByteOffset {
        self.end.saturating_sub(self.start)
    }

    /// 是否为空区间（dummy 或零长）。
    pub fn is_empty(self) -> bool {
        self.end <= self.start
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Span(file {}, {}..{}{}",
            self.file_id,
            self.start,
            self.end,
            if self.expansion_id > 0 {
                format!(", exp {})", self.expansion_id)
            } else {
                ")".to_string()
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_merge_union() {
        let a = Span::new(0, 5, 10);
        let b = Span::new(0, 8, 20);
        assert_eq!(a.merge(b), Span::new(0, 5, 20));
        // 反向合并结果一致（并集）
        assert_eq!(b.merge(a), Span::new(0, 5, 20));
    }

    #[test]
    fn span_merge_different_files_falls_back() {
        let a = Span::new(0, 5, 10);
        let b = Span::new(1, 0, 3);
        assert_eq!(a.merge(b), a);
    }

    #[test]
    fn span_expansion_bump() {
        let a = Span::new(0, 1, 2);
        assert_eq!(a.bumped_expansion().expansion_id, 1);
        assert_eq!(a.bumped_expansion().bumped_expansion().expansion_id, 2);
    }

    #[test]
    fn span_contains_and_len() {
        let a = Span::new(0, 10, 20);
        assert!(a.contains(10) && a.contains(19));
        assert!(!a.contains(20) && !a.contains(9));
        assert_eq!(a.len(), 10);
        assert!(!a.is_empty());
        assert!(Span::dummy().is_empty());
    }
}
