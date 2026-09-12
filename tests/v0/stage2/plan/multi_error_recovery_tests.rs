//! 批次 G（r17）——TD-013 多错误收集与恢复展开实现测试（38-e）。
//!
//! r7 设计（stage-1/multi-error-recovery-design.md）验收 5 条的
//! 逐项落地 + 双路径（种子/自举桥）同构：
//! 1. 形式级恢复：错误形式跳过 + 后续形式正常展开（产物含之）；
//! 2. 收集上限 128 + 截断提示；
//! 3. 次序 = 源码位置（与 check_program 同口径）；
//! 4. 既有测试零破坏（单错误短路 API `check_source` 行为不变锚）；
//! 5. 本套件 ≥12 case（恢复/跳过/上限/次序/部分产物/短路边界）。
//!
//! 展开错误样本：`()`（E0002「空列表不能作为表达式求值」——
//! read 合法 + expand 报错的真实恢复路径触发器）。

use kerf_driver::{check_source, check_source_recover, Stage};
use kerf_expander::{expand_program_recover, DiagCollector, ExpandCtxt, MAX_DIAGNOSTICS};
use kerf_reader::read_source;
use kerf_syntax::SymbolTable;

const FNAME: &str = "recovery-test.krf";

/// read + 种子恢复展开（expander 层直达——不经过 driver 管线）。
fn seed_recover(src: &str) -> kerf_expander::RecoveredExpansion {
    let mut sm = kerf_span::SourceMap::new();
    let file_id = sm.add_file(FNAME, src);
    let mut table = SymbolTable::new();
    let forms = read_source(src, file_id, &mut table).expect("read 应成功");
    let mut ctx = ExpandCtxt::new(table);
    expand_program_recover(&forms, &mut ctx)
}

// ---------------------------------------------------------------------------
// 种子 expander 层（恢复语义单元）
// ---------------------------------------------------------------------------

#[test]
fn seed_recovery_skips_failing_form_and_keeps_rest() {
    // r7 验收 1 精神版：好/坏/好——坏形式收集、好形式产物保留
    let r = seed_recover("(define x 1)\n()\n(define y 2)\n");
    assert_eq!(r.diags.len(), 1, "空形式应报 E0002");
    assert_eq!(r.core.len(), 2, "两个 define 应正常展开");
    assert!(!r.is_clean());
    // 产物语义：define y 在产物中（r7 验收「编译产物含 y」的展开层等价）
    let has_y = r
        .core
        .iter()
        .any(|e| matches!(e.as_ref(), kerf_core::CoreExpr::Define { .. }));
    assert!(has_y, "define 形式应保留");
}

#[test]
fn seed_recovery_reports_all_errors_across_forms() {
    // 三个坏形式 → 三条诊断（单错误短路的对照面）
    let r = seed_recover("()\n()\n()\n");
    assert_eq!(r.diags.len(), 3);
    assert_eq!(r.core.len(), 0);
}

#[test]
fn seed_recovery_sorted_by_source_position() {
    // r7 验收 3：乱序入收集 → 位置序导出
    let r = seed_recover("(define a 1)\n()\n(define b 2)\n()\n(define c 3)\n");
    assert_eq!(r.diags.len(), 2);
    let sorted = r.diags.clone().into_sorted();
    let starts: Vec<u32> = sorted.iter().map(|e| e.span.start).collect();
    assert_eq!(starts, vec![13, 29], "次序 = 源位置（13/29 实算）");
}

#[test]
fn seed_recovery_cap_128_with_early_termination() {
    // r7 验收 2：>128 错 → 收集恰 128 + 截断标记 + 提前终止（后续形式
    // 不再展开——风暴防护）
    let mut src = String::new();
    for _ in 0..140 {
        src.push_str("()\n");
    }
    src.push_str("(define after 1)\n");
    let r = seed_recover(&src);
    assert_eq!(r.diags.len(), MAX_DIAGNOSTICS);
    assert!(r.diags.truncated(), "截断标记应置位");
    // 提前终止：第 129 个坏形式起不再展开也不再收集——但已恢复的
    // 产物不含 140 个坏形式（core 为空——恢复循环在满后 break）
    assert!(r.core.is_empty(), "满后剩余形式不展开（含 after）");
}

#[test]
fn collector_unit_cap_and_truncation_flag() {
    // 收集器单元：上限 128 + 满 129 个 → 128 存 + truncated
    let mut c = DiagCollector::default();
    for i in 0..MAX_DIAGNOSTICS + 1 {
        c.push(kerf_expander::ExpandError {
            message: format!("e{}", i),
            span: kerf_span::Span::dummy(),
        });
    }
    assert_eq!(c.len(), MAX_DIAGNOSTICS);
    assert!(c.truncated());
    assert!(c.is_full());
}

#[test]
fn clean_program_recovers_nothing() {
    // 零错误程序：恢复路径 = 短路路径（空收集）
    let r = seed_recover("(define x 1)\n(+ x 2)\n");
    assert!(r.is_clean());
    assert_eq!(r.core.len(), 2);
    assert_eq!(r.diags.len(), 0);
}

// ---------------------------------------------------------------------------
// driver 集成层（check_source_recover 全链）
// ---------------------------------------------------------------------------

#[test]
fn driver_recover_report_merges_expand_and_type_diagnostics() {
    // 混合面：expand 错（E0002 空形式）+ typecheck 错（E0005 car 元数）
    // 同报 + 位置序
    let src = "(define x 1)\n()\n(car)\n";
    let report = check_source_recover(src, FNAME).expect("恢复路径应产出报告");
    assert_eq!(report.diagnostics.len(), 2, "E0002 + E0005 各一");
    // 次序：空形式（行 2）在 car（行 3）前
    assert!(report.diagnostics[0].primary_span.start < report.diagnostics[1].primary_span.start);
    // 产物：x 的 define 进入编译（部分产物继续走全管线）
    assert!(report.proto_count >= 1, "部分产物应编译出原型");
}

#[test]
fn driver_recover_partial_product_contains_later_defines() {
    // r7 验收 1 完整版：错误形式后 y 定义照常编译
    let src = "(define x 1)\n()\n(define y 2)\ny\n";
    let report = check_source_recover(src, FNAME).expect("应产出报告");
    assert_eq!(report.diagnostics.len(), 1);
    // 产物含 y：错误形式后的指令数 > 仅 x 的指令数（define y + y 引用
    // 都产指令）——与「y 不在」的对照程序比较
    let control =
        check_source_recover("(define x 1)\n(define y 2)\ny\n", FNAME).expect("对照应产出报告");
    assert!(control.diagnostics.is_empty());
    assert_eq!(
        report.instruction_count, control.instruction_count,
        "恢复产物与无错对照的指令数一致（y 完整进入编译）"
    );
}

#[test]
fn driver_recover_rendered_lines_sorted_and_complete() {
    // 渲染面：逐条 + 位置序（E0002 在 E0005 前——源位置）
    let src = "()\n(define x 1)\n(car)\n";
    let report = check_source_recover(src, FNAME).expect("应产出报告");
    // M2（r40）：rendered 含 error 诊断 + W 级警告（非阻断——(car)
    // 旧名引用触发 W1001 弃用族一条）两通道合计
    assert_eq!(
        report.rendered.len(),
        report.diagnostics.len() + report.warnings.len()
    );
    // 首条 = 空形式（行 1）→ 消息含「空列表」
    assert!(
        report.rendered[0].contains("空列表"),
        "首条：{}",
        report.rendered[0]
    );
    assert!(
        report.rendered[1].contains("car"),
        "次条：{}",
        report.rendered[1]
    );
}

#[test]
fn driver_recover_read_error_still_short_circuits() {
    // 短路边界 1：read 词法错误 → DriverError（非报告——r7 设计 §5）
    let e = check_source_recover("(unclosed", FNAME).expect_err("read 错应短路");
    assert!(matches!(e.stage, Stage::Read));
}

#[test]
fn driver_recover_capability_gate_still_short_circuits() {
    // 短路边界 2：E0006 fail-closed（r8 裁定——恢复面不含能力违规）
    let e = check_source_recover("(print 1)\n()", FNAME).expect_err("E0006 应短路");
    assert!(matches!(e.stage, Stage::Compile));
    assert!(e.rendered.contains("能力权限不足") || e.rendered.contains("E0006"));
}

#[test]
fn legacy_check_source_single_error_semantics_unchanged() {
    // r7 验收 4：既有单错误短路 API 行为不变（恢复只在恢复入口生效）
    // —— expand 错误 → Err（DriverError）而非报告
    let e = check_source("(define x 1)\n()\n", FNAME).expect_err("短路路径应 Err");
    assert!(matches!(e.stage, Stage::Expand));
}

#[test]
fn legacy_typecheck_multi_error_still_works_via_recover() {
    // 既有 typecheck 多错误面（r7 设计 §5 首个消费面）经恢复入口延续：
    // 三个 E0005 全报
    let src = "(car)\n(cdr)\n(+ 1 \"s\")\n";
    let report = check_source_recover(src, FNAME).expect("应产出报告");
    assert_eq!(report.diagnostics.len(), 3, "三 typecheck 错全报");
}

#[test]
fn recover_truncation_note_appended_to_rendered() {
    // 截断提示行（r7 设计 §2：warning 级汇总——rendered 尾注）
    let mut src = String::new();
    for _ in 0..MAX_DIAGNOSTICS + 5 {
        src.push_str("()\n");
    }
    let report = check_source_recover(&src, FNAME).expect("应产出报告");
    assert_eq!(report.diagnostics.len(), MAX_DIAGNOSTICS);
    let last = report.rendered.last().expect("截断提示应在");
    assert!(last.contains("上限"), "截断提示：{}", last);
}

// ---------------------------------------------------------------------------
// 双路径同构（种子恢复 vs 自举桥恢复——parity 纪律）
// ---------------------------------------------------------------------------

#[test]
fn bootstrap_bridge_recovery_matches_seed_path() {
    // 同程序：桥恢复（生产路径）与种子恢复的（诊断数, 产物数）一致
    use kerf_driver::bootstrap_expander;
    let src = "(define x 1)\n()\n(define y 2)\n";
    // 种子
    let seed = seed_recover(src);
    // 桥（需要 file_id 与 table——与 driver 恢复入口同构的最小拼装）
    let mut sm = kerf_span::SourceMap::new();
    let file_id = sm.add_file(FNAME, src);
    let mut table = SymbolTable::new();
    let forms = read_source(src, file_id, &mut table).expect("read 应成功");
    let bridge =
        bootstrap_expander::expand_program_recover(&forms, file_id, &mut table).expect("桥应可用");
    assert_eq!(bridge.diags.len(), seed.diags.len(), "诊断数一致");
    assert_eq!(
        bridge.core.len(),
        seed.core.len(),
        "产物数一致（x/y 两 define）"
    );
    assert_eq!(bridge.diags.len(), 1);
    assert_eq!(bridge.core.len(), 2);
    // 桥错误语义同构（E0002 家族消息）
    let sorted = bridge.diags.into_sorted();
    assert!(
        sorted[0].message.contains("空列表"),
        "桥错误：{}",
        sorted[0].message
    );
}

#[test]
fn run_source_execution_path_still_single_error() {
    // r7 设计 §5：执行路径不恢复（运行期短路维持）——recover 只属编译期
    let e = kerf_driver::run_source("(define x 1)\n()\n", FNAME).expect_err("执行路径应短路");
    assert!(matches!(e.stage, Stage::Expand));
}
