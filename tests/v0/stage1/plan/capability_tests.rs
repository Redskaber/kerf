//! 能力模型 I/O 基础传递集成测试（r8 批次 D——D2：13 §3.1.3 规格四
//! 条款逐条端到端锁定）。
//!
//! 覆盖面（§9.4.3 正负比 ≥1:3——本套件负向为主）：
//! - 条款 3（编译期权限验证）：E0006 在 run/eval/check/compile 四路径
//!   全量生效（front 管线单一验证点——「任意流程节点」的管线面证明）；
//! - 声明面 `(require io read|write)`：正向语义 + 形状负例（未知主体/
//!   未知能力/缺参/非符号）；
//! - 令牌门控：未声明程序 I/O 内置不注册（fail-closed）+ 授权面一致；
//! - 卫生回退边界：宏模板内 I/O 引用的基名判定；
//! - 用户接管豁免：define 同名不误报（零误报纪律）；
//! - examples/usage/io.krf 实跑（require 前缀形态）。

use kerf_driver::{check_source, run_source, run_source_seed, Stage};
use kerf_vm::Value;

const FNAME: &str = "cap.krf";

#[allow(clippy::result_large_err)] // 测试助手——错误路径体积可接受（driver 入口同款约定）
fn run(src: &str) -> Result<kerf_driver::RunOutcome, kerf_driver::DriverError> {
    run_source(src, FNAME)
}

// ---------------------------------------------------------------------------
// 正向：声明后可用（条款 1/2/4 的授权链）
// ---------------------------------------------------------------------------

/// require + print：写入路径全通（令牌捕获 + StdCapabilityIO）。
#[test]
fn declared_write_io_runs() {
    let o = run("(require io write) (print 42)").unwrap();
    assert!(matches!(o.value, Value::Nil));
}

/// require + newline：零参写内置可用。
#[test]
fn declared_newline_runs() {
    let o = run("(require io write) (newline)").unwrap();
    assert!(matches!(o.value, Value::Nil));
}

/// require 读写双声明：写路径全通（读路径见 EOF 子进程探针——
/// 进程内直调 `(read-line)` 会阻塞真实 stdin，非确定性，禁止）。
#[test]
fn declared_read_write_both() {
    let o = run("(require io read write) (print 1)").unwrap();
    assert!(matches!(o.value, Value::Nil));
}

/// read-line EOF 约定路径（确定性子进程探针：程序经临时文件传入、
/// stdin = null 设备 → read-line 立即见 EOF → nil；不依赖测试运行器
/// 的 stdin 形态——§2.3 确定性边界先行）。
#[test]
fn read_line_eof_returns_nil_via_subprocess() {
    use std::process::{Command, Stdio};
    let tmp = std::env::temp_dir().join("kerf-cap-eof-probe.krf");
    std::fs::write(&tmp, "(require io read) (read-line)").expect("写入探针程序");
    let out = Command::new(env!("CARGO_BIN_EXE_kerf"))
        .args(["run", tmp.to_str().unwrap()])
        .stdin(Stdio::null())
        .output()
        .expect("应能启动 kerf 子进程");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "EOF 读应成功，stderr：{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("nil"), "EOF 读应得 nil，实际：{}", stdout);
    let _ = std::fs::remove_file(&tmp);
}

/// require 求值恒 nil + 双路径一致（T1 新口径 42-d：生产链与种子链
/// 同口径——require 零运行时语义）。
#[test]
fn require_form_evals_nil_both_paths() {
    let a = run("(require io write)").unwrap();
    let b = run_source_seed("(require io write)", FNAME).unwrap();
    assert!(matches!(a.value, Value::Nil));
    assert!(matches!(b.value, Value::Nil));
}

/// 声明幂等（重复 require 集合语义）。
#[test]
fn duplicate_require_is_idempotent() {
    let o = run("(require io write) (require io write) (print 9)").unwrap();
    assert!(matches!(o.value, Value::Nil));
}

/// 顺序无关：require 出现在引用之后仍满足（编译期集合判定，非顺序）。
#[test]
fn require_after_use_still_passes() {
    let o = run("(print 3) (require io write)").unwrap();
    assert!(matches!(o.value, Value::Nil));
}

/// check 路径：require 程序零诊断 + 报告含声明观测。
#[test]
fn check_report_includes_io_requirements() {
    let r = check_source("(require io write) (define x 1) x", FNAME).unwrap();
    assert!(r.diagnostics.is_empty());
    assert!(r.io_write, "CheckReport 应观测 write 声明");
    assert!(!r.io_read);
}

/// examples/usage/io.krf 实跑（require 前缀形态的活体样例；相对路径
/// 以 workspace 根为 cwd 口径——与 typecheck_tests 的示例锚同则）。
#[test]
fn example_io_krf_runs() {
    let src = std::fs::read_to_string("examples/usage/io.krf").unwrap();
    let o = run(&src).unwrap();
    assert!(matches!(o.value, Value::Int(42)));
}

// ---------------------------------------------------------------------------
// 负向：E0006 编译期权限验证（条款 3——四路径 + 边界）
// ---------------------------------------------------------------------------

/// 未声明 print → E0006（run 路径，编译期阶段而非运行时未绑定）。
#[test]
fn undeclared_print_is_compile_error() {
    let e = run("(print 42)").unwrap_err();
    assert_eq!(e.stage, Stage::Compile);
    assert_eq!(e.diagnostic.code, Some(kerf_span::DiagnosticCode(6)));
    assert!(e.rendered.contains("error[E0006]"));
    assert!(e.rendered.contains("(require io write)"));
}

/// 未声明 read 族 → E0006（read 能力粒度）。
#[test]
fn undeclared_read_is_compile_error() {
    let e = run("(read-line)").unwrap_err();
    assert_eq!(e.stage, Stage::Compile);
    assert!(e.rendered.contains("(require io read)"));
}

/// 声明 read 不开放 write（最小权限——粒度互斥）。
#[test]
fn read_declaration_does_not_open_write() {
    let e = run("(require io read) (print 1)").unwrap_err();
    assert_eq!(e.stage, Stage::Compile);
    assert!(e.rendered.contains("write"));
}

/// 种子链同门控（front 单一验证点——管线面任意节点；42-d 口径迁移）。
#[test]
fn seed_path_gated_identically() {
    let e = run_source_seed("(print 1)", FNAME).unwrap_err();
    assert_eq!(e.stage, Stage::Compile);
    assert!(e.rendered.contains("error[E0006]"));
}

/// check 路径同门控（诊断链前后不交叉：编译期错误先行）。
#[test]
fn check_path_gated_identically() {
    let e = check_source("(newline)", FNAME).unwrap_err();
    assert_eq!(e.stage, Stage::Compile);
    assert!(e.rendered.contains("error[E0006]"));
}

/// 嵌套深处的引用（lambda 体内 + 条件分支）仍被静态捕获。
#[test]
fn nested_reference_gated() {
    let src = "(define (f) (lambda () (if true (print 1) 2)))";
    let e = run(src).unwrap_err();
    assert_eq!(e.stage, Stage::Compile);
    assert!(e.rendered.contains("print"));
}

/// 宏模板内的 I/O 引用：卫生基名回退判定（与运行时回退同则）。
#[test]
fn macro_template_io_reference_gated() {
    let src = r#"
        (define-syntax shout
          (syntax-rules ()
            ((shout v) (print v))))
        (shout 1)
    "#;
    let e = run(src).unwrap_err();
    assert_eq!(e.stage, Stage::Compile);
    assert!(e.rendered.contains("print"), "宏模板内引用须按基名门控");
}

// ---------------------------------------------------------------------------
// 负向：require 形状（展开期 E2 家族）
// ---------------------------------------------------------------------------

/// 缺参：`(require io)` → 展开错误。
#[test]
fn require_missing_capability_args() {
    let e = run("(require io)").unwrap_err();
    assert_eq!(e.stage, Stage::Expand);
    assert!(e.rendered.contains("require 形式"));
}

/// 未知主体：`(require net ...)` → 未知主体错误（Stage 2 扩展面提示）。
#[test]
fn require_unknown_subject() {
    let e = run("(require net read)").unwrap_err();
    assert_eq!(e.stage, Stage::Expand);
    assert!(e.rendered.contains("未知能力主体"));
}

/// 未知能力项：`(require io fly)` → 未知能力错误。
#[test]
fn require_unknown_capability() {
    let e = run("(require io fly)").unwrap_err();
    assert_eq!(e.stage, Stage::Expand);
    assert!(e.rendered.contains("未知能力项"));
}

/// 非符号能力项：`(require io "read")` → 符号形态错误。
#[test]
fn require_non_symbol_capability() {
    let e = run(r#"(require io "read")"#).unwrap_err();
    assert_eq!(e.stage, Stage::Expand);
    assert!(e.rendered.contains("符号"));
}

// ---------------------------------------------------------------------------
// 边界：接管豁免与授权面一致性（零误报纪律）
// ---------------------------------------------------------------------------

/// 用户接管：`(define print 5)` 后引用不标记（用户全局）。
#[test]
fn user_takeover_exempt() {
    let o = run("(define print 5) print").unwrap();
    assert!(matches!(o.value, Value::Int(5)));
}

/// 别名定义值侧引用内置：仍需声明（非接管）。
#[test]
fn alias_value_reference_gated() {
    let e = run("(define my-print print)").unwrap_err();
    assert_eq!(e.stage, Stage::Compile);
    assert!(e.rendered.contains("print"));
}

/// 令牌外部不可构造（不可伪造性的边界证明——外部 crate 仅可引用
/// 类型不可实例化，与 reserved.rs 单元测试镜像）。
#[test]
fn capability_tokens_unforgeable_from_outside() {
    fn takes_read(_cap: &kerf_driver::ReadCapability) {}
    fn takes_write(_cap: &kerf_driver::WriteCapability) {}
    let _ = (
        takes_read as fn(&kerf_driver::ReadCapability),
        takes_write as fn(&kerf_driver::WriteCapability),
    );
}

/// IoRequirements 公共观测面（from_core 的外部消费形态）。
#[test]
fn io_requirements_public_observation() {
    let req = kerf_driver::IoRequirements::default();
    assert!(req.is_empty());
    assert_eq!(req.render(), "（未声明）");
}
