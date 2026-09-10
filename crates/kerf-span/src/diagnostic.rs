//! 结构化诊断框架（stage0.md §8.7 / §12.2）。
//!
//! 借鉴 rustc 的 Diagnostic 结构：
//! - **错误是数据而非异常**（§8.7 架构原则）；
//! - 支持错误恢复策略（诊断可收集后统一渲染）；
//! - 多阶段错误关联（children 子诊断 + suggestions 修复建议）。
//!
//! 所有错误类型的最小共享形态为 `{ message: String, span: Span }`
//! （sop.md §10.1 规则 8）。

use crate::source_map::SourceMap;
use crate::span::Span;

/// 诊断严重级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

impl Severity {
    /// 渲染标签（固定宽度，便于多诊断对齐）。
    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
            Severity::Help => "help",
        }
    }
}

/// 诊断编号（如 E0001 = 词法错误）。`None` 表示未编号。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiagnosticCode(pub u32);

impl DiagnosticCode {
    /// 渲染为 "E0001" 形式。
    pub fn render(self) -> String {
        format!("E{:04}", self.0)
    }
}

/// 次要诊断（关联说明，如「宏在此展开」）。
#[derive(Debug, Clone)]
pub struct SubDiagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

/// 修复建议。
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub message: String,
    pub span: Span,
    pub replacement: String,
}

/// rustc 风格结构化诊断。
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: Option<DiagnosticCode>,
    pub message: String,
    pub primary_span: Span,
    pub children: Vec<SubDiagnostic>,
    pub suggestions: Vec<Suggestion>,
}

impl Diagnostic {
    /// 构造 Error 级诊断。
    pub fn error(code: Option<DiagnosticCode>, message: impl Into<String>, span: Span) -> Self {
        Diagnostic {
            severity: Severity::Error,
            code,
            message: message.into(),
            primary_span: span,
            children: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// 构造 Warning 级诊断。
    pub fn warning(code: Option<DiagnosticCode>, message: impl Into<String>, span: Span) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            code,
            message: message.into(),
            primary_span: span,
            children: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// 附加次要诊断（链式）。
    pub fn with_child(
        mut self,
        severity: Severity,
        message: impl Into<String>,
        span: Span,
    ) -> Self {
        self.children.push(SubDiagnostic {
            severity,
            message: message.into(),
            span,
        });
        self
    }

    /// 附加修复建议（链式）。
    pub fn with_suggestion(
        mut self,
        message: impl Into<String>,
        span: Span,
        replacement: impl Into<String>,
    ) -> Self {
        self.suggestions.push(Suggestion {
            message: message.into(),
            span,
            replacement: replacement.into(),
        });
        self
    }

    /// 判定是否为阻断性错误。
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// 渲染诊断为人类可读文本（§2.2 原则 16：人类可感知输出是一等公民）。
///
/// 格式：
/// ```text
/// error[E0001]: 非法字符 '#'
///   --> main.krf:1:5
///    |
///  1 | (+# 1)
///    |      ^
/// ```
pub fn render_diagnostic(diag: &Diagnostic, source_map: &SourceMap) -> String {
    let mut out = String::new();
    let code = diag
        .code
        .as_ref()
        .map(|c| format!("[{}]", c.render()))
        .unwrap_or_default();
    out.push_str(&format!(
        "{}{}: {}\n",
        diag.severity.label(),
        code,
        diag.message
    ));
    let span = diag.primary_span;
    out.push_str(&format!(
        "  --> {}\n",
        source_map.render_location(span.file_id, span.start, span.expansion_id)
    ));
    if !span.is_empty() || source_map.file(span.file_id).is_some() {
        let excerpt = source_map.excerpt(span.file_id, span.start);
        if !excerpt.is_empty() {
            out.push_str("   |\n");
            for line in excerpt.lines() {
                out.push_str(&format!("{}\n", line));
            }
        }
    }
    for child in &diag.children {
        out.push_str(&format!(
            "  {}: {} ({})\n",
            child.severity.label(),
            child.message,
            source_map.render_location(
                child.span.file_id,
                child.span.start,
                child.span.expansion_id
            )
        ));
    }
    for sugg in &diag.suggestions {
        out.push_str(&format!(
            "  help: {} → 「{}」\n",
            sugg.message, sugg.replacement
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_render_full_shape() {
        let mut sm = SourceMap::new();
        let id = sm.add_file("t.krf", "(+ # 1)");
        let diag = Diagnostic::error(Some(DiagnosticCode(1)), "非法字符 '#'", Span::new(id, 3, 4))
            .with_child(Severity::Note, "此处需要操作数", Span::new(id, 0, 7))
            .with_suggestion("是否想输入 1", Span::new(id, 3, 4), "1");
        let text = render_diagnostic(&diag, &sm);
        assert!(text.contains("error[E0001]: 非法字符 '#'"));
        assert!(text.contains("--> t.krf:1:4"));
        assert!(text.contains("note: 此处需要操作数"));
        assert!(text.contains("help: 是否想输入 1"));
    }

    #[test]
    fn severity_labels() {
        assert_eq!(Severity::Error.label(), "error");
        assert_eq!(Severity::Warning.label(), "warning");
    }

    #[test]
    fn code_render() {
        assert_eq!(DiagnosticCode(1).render(), "E0001");
        assert_eq!(DiagnosticCode(331).render(), "E0331");
    }
}
