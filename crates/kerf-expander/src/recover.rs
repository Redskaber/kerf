//! 多错误收集与形式级恢复展开（TD-013 实现——批次 G / 38-e）。
//!
//! 依据 r7 设计（stage-1/multi-error-recovery-design.md，Accepted）：
//! - **恢复粒度 = 顶层形式**：一个形式展开失败 → 诊断入收集器 → 跳过
//!   该形式 → 继续后续形式（kerf 顶层形式无隐式数据流——形式间天然
//!   独立）；表达式级不恢复（半展开 Stx 树状态重建成本不成比例）；
//! - **收集上限 128**：诊断风暴防护（宏模板级连坐）+ 截断标记；
//! - **导出排序**：`(file_id, start, end)` 稳定排序——多错误输出次序
//!   与源码位置对齐（与 `check_program` 同口径）；
//! - **编译期控制流**（非 effect——§11 接口隔离）；run/eval 执行路径
//!   不恢复（运行期状态不可回滚）。
//!
//! 与 [`crate::expander::expand_program`]（单错误短路——生产 run/eval
//! 路径不变）并列的恢复入口；双 expander（种子/自举桥）同构实现，
//! parity 纪律对齐。

use std::cmp::Ordering;
use std::rc::Rc;

use kerf_core::CoreExpr;
use kerf_syntax::Stx;

use crate::expander::{expand_form, ExpandCtxt, ExpandError};

/// 诊断收集上限（r7 设计 §2——诊断风暴防护）。
pub const MAX_DIAGNOSTICS: usize = 128;

/// 多错误收集器（r7 设计 §3 契约——Diagnostic 是数据，§8.7）。
#[derive(Debug, Clone, Default)]
pub struct DiagCollector {
    diags: Vec<ExpandError>,
    truncated: bool,
}

impl DiagCollector {
    /// 追加诊断（超上限 → 置 truncated 不存）。
    pub fn push(&mut self, e: ExpandError) {
        if self.diags.len() >= MAX_DIAGNOSTICS {
            self.truncated = true;
            return;
        }
        self.diags.push(e);
    }

    /// 恢复循环的提前终止信号（已满）。
    pub fn is_full(&self) -> bool {
        self.diags.len() >= MAX_DIAGNOSTICS
    }

    /// 显式置截断标记（恢复循环在上限时提前终止——剩余形式被丢弃
    /// 即截断发生；38-e 实测勘误：break 路径不走 push 分支）。
    pub fn mark_truncated(&mut self) {
        self.truncated = true;
    }

    /// 已收集数。
    pub fn len(&self) -> usize {
        self.diags.len()
    }

    /// 是否零收集。
    pub fn is_empty(&self) -> bool {
        self.diags.is_empty()
    }

    /// 是否发生截断（达到上限后仍有错误被丢弃）。
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    /// 按 `(file_id, start, end)` 稳定排序导出（源码位置对齐——
    /// 与 `check_program` 排序口径一致）。
    pub fn into_sorted(mut self) -> Vec<ExpandError> {
        self.diags.sort_by(|a, b| span_order(&a.span, &b.span));
        self.diags
    }
}

/// Span 次序键（file_id → start → end）。
fn span_order(a: &kerf_span::Span, b: &kerf_span::Span) -> Ordering {
    a.file_id
        .cmp(&b.file_id)
        .then(a.start.cmp(&b.start))
        .then(a.end.cmp(&b.end))
}

/// 恢复展开产物：部分 CoreExpr + 诊断收集器（错误与产物共存——
/// r7 设计 §5 消费面表格的既定形态）。
#[derive(Debug, Clone, Default)]
pub struct RecoveredExpansion {
    /// 成功形式的展开产物（错误形式被跳过——不含于产物）。
    pub core: Vec<Rc<CoreExpr>>,
    /// 失败形式的诊断（按推入序；导出时经 [`DiagCollector::into_sorted`]）。
    pub diags: DiagCollector,
}

impl RecoveredExpansion {
    /// 恢复是否全程无错（空收集器 = 与短路路径等价）。
    pub fn is_clean(&self) -> bool {
        self.diags.is_empty()
    }
}

/// 形式级恢复展开（种子 Expander 路径）。
///
/// 逐形式调用 [`expand_form`]：单形式失败 → 收集 + 跳过 + 继续；
/// 收集器满 → 提前终止（剩余形式不展开——诊断风暴防护）。
/// ExpandCtxt 的相位/变换器簿记在成功形式间持续（形式级跳过不回滚
/// 已完成注册——跳过形式对后续形式的影响仅在「引用被跳过绑定时」
/// 产生新错误，属恢复语义的正确代价）。
pub fn expand_program_recover(forms: &[Stx], ctx: &mut ExpandCtxt) -> RecoveredExpansion {
    let mut out = RecoveredExpansion::default();
    for form in forms {
        if out.diags.is_full() {
            // 上限防护：提前终止——剩余形式被丢弃即截断（显式标记：
            // break 路径不经过 push 的截断分支，38-e 勘误）
            out.diags.mark_truncated();
            break;
        }
        match expand_form(form, ctx) {
            Ok(core) => out.core.push(core),
            Err(e) => out.diags.push(e),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expander::ExpandCtxt;

    /// 构造测试 Stx（原子符号形式——经 SymbolTable intern）。
    fn atom_form(name: &str, table: &mut kerf_syntax::SymbolTable) -> Stx {
        let sym = table.intern(name);
        Stx::symbol(sym, kerf_span::Span::dummy(), kerf_syntax::ScopeSet::new())
    }

    #[test]
    fn recover_skips_failing_forms_and_continues() {
        // 好形式（define）/坏形式（空列表——展开错误）/好形式：
        // 中间错被收集、前后 define 正常展开（r7 验收 1 骨架）
        let mut table = kerf_syntax::SymbolTable::new();
        let forms = vec![
            list_form(&["define", "x", "1"], &mut table), // 合法定义
            Stx::list(
                Vec::new(),
                kerf_span::Span::dummy(),
                kerf_syntax::ScopeSet::new(),
            ), // 空——expand 错
            list_form(&["define", "y", "2"], &mut table), // 合法定义
        ];
        let mut ctx = ExpandCtxt::new(table);
        let r = expand_program_recover(&forms, &mut ctx);
        assert_eq!(r.diags.len(), 1, "中间形式应报错");
        assert_eq!(r.core.len(), 2, "前后定义应正常展开");
        assert!(!r.is_clean());
    }

    /// 构造列表形式（define 糖——符号 intern）。
    fn list_form(names: &[&str], table: &mut kerf_syntax::SymbolTable) -> Stx {
        let items: Vec<Stx> = names
            .iter()
            .map(|n| {
                let sym = table.intern(n);
                Stx::symbol(sym, kerf_span::Span::dummy(), kerf_syntax::ScopeSet::new())
            })
            .collect();
        Stx::list(
            items,
            kerf_span::Span::dummy(),
            kerf_syntax::ScopeSet::new(),
        )
    }

    #[test]
    fn collector_cap_at_128_with_truncation_flag() {
        let mut c = DiagCollector::default();
        for i in 0..MAX_DIAGNOSTICS + 10 {
            c.push(ExpandError {
                message: format!("err-{}", i),
                span: kerf_span::Span::dummy(),
            });
        }
        assert_eq!(c.len(), MAX_DIAGNOSTICS);
        assert!(c.truncated());
        assert!(c.is_full());
    }

    #[test]
    fn collector_sorted_by_span_position() {
        let mut c = DiagCollector::default();
        let mk = |start: u32| kerf_span::Span {
            file_id: 0,
            start,
            end: start + 1,
            expansion_id: 0,
        };
        c.push(ExpandError {
            message: "late".into(),
            span: mk(90),
        });
        c.push(ExpandError {
            message: "early".into(),
            span: mk(10),
        });
        c.push(ExpandError {
            message: "mid".into(),
            span: mk(50),
        });
        let sorted = c.into_sorted();
        let starts: Vec<u32> = sorted.iter().map(|e| e.span.start).collect();
        assert_eq!(starts, vec![10, 50, 90]);
    }

    #[test]
    fn collector_empty_and_not_full_defaults() {
        let c = DiagCollector::default();
        assert!(c.is_empty());
        assert!(!c.is_full());
        assert!(!c.truncated());
        assert_eq!(c.len(), 0);
    }
}
