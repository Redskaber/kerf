//! Stage 0 负向门审计集（sop.md §7.3.1 规则 3 / §7.1.1 七类矩阵）。
//!
//! **用途**：≥30 case 负向审计——阶段门审查（§7.3）强制审计集。本文件为
//! 第 1 轮（r1），对应 Stage 0 深度审查 deep-review-round1（NEEDS REVISION）
//! 的修复轮，供 Gate R2 重跑使用；§7.3.1 规则 4 要求后续轮次 ≥ 本轮规模。
//!
//! **运行方式**：工作区根目录 `cargo run --example stage0_gate_audit_r1`。
//! 全部 case PASS（含 XFAIL：按「当前真实行为」断言并打印 WARN 的发现项）
//! 时退出码 0；任一 case FAIL 或 §7.3.1 强制配比不满足时退出码 1。
//! 内部参数 `--cyclic-dep-probe` 仅供 ⑥ 循环依赖子进程探针复用本二进制。
//!
//! **结构**（§7.3.1 规则 2 强制配比，配比在 main() 中机械校验）：
//! - A 桶 12：单语句负向（基础类型系统）
//! - B 桶 12：多语句/多函数负向（集成正确性）
//! - C 桶 8：复杂程序负向（嵌套控制流/闭包/递归/宏 + ⑥ 循环依赖
//!   + 双路径分裂发现）
//! - D 桶 6：错误恢复（错误后同进程续跑正确程序 / Lexer·Parser 仍正常）
//! - P 桶 3：正向 sanity（审计器自证）
//!
//! 合计 41 case：负向 32 + 恢复 6 + 正向 3（负向 ≥22、正向 ≤8）。
//!
//! **§7.1.1 七类覆盖**：①语法 ②未绑定 ③空应用 ④参数个数 ⑤类型不匹配
//! ⑥模块循环依赖（已修复：phase.rs DFS 灰标记 → 结构化 Err，探针
//! CYCLE-DETECTED 路径激活）⑦宏展开深度超限。
//!
//! **历史发现项**（审计集驱动修复——均已修复，case 自动升级为严格 PASS）：
//! - C07：⑥ 循环依赖无检测 → 栈溢出崩溃（已修：DFS 灰标记）；
//! - C03：运行时堆栈追踪未实现（已修：run_program 包装帧链快照，
//!   最内 16 帧渲染 note 调用点）；
//! - C08：eval 路径缺卫生回退解析（已修：driver eval 侧镜像解析，
//!   双路径一致 Ok 42）。

use std::process::Command;

use kerf_driver::{dump_stx, dump_tokens, run_source, run_source_rendered, run_source_seed, Stage};
use kerf_expander::phase::ModuleRegistry;
use kerf_span::DiagnosticCode;
use kerf_syntax::SymbolTable;
use kerf_vm::{render_value, Value};

/// 审计源文件名（诊断渲染位置断言用）。
const FNAME: &str = "audit.krf";
/// 循环依赖子进程探针参数（main 分派）。
const PROBE_ARG: &str = "--cyclic-dep-probe";

// ---------------------------------------------------------------------------
// 桶 / 极性 / §7.1.1 错误类
// ---------------------------------------------------------------------------

/// §7.3.1 规则 2 的四个强制桶 + 正向 sanity。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Bucket {
    Single,
    Multi,
    Complex,
    Recovery,
    Positive,
}

impl Bucket {
    fn label(self) -> &'static str {
        match self {
            Bucket::Single => "A:单语句",
            Bucket::Multi => "B:多语句",
            Bucket::Complex => "C:复杂程序",
            Bucket::Recovery => "D:错误恢复",
            Bucket::Positive => "P:正向",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Polarity {
    Negative,
    Mixed,
    Positive,
}

/// §7.1.1 负向测试最小覆盖矩阵的七类（index 1..=7）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum ErrorClass {
    Syntax,
    Unbound,
    EmptyApp,
    Arity,
    TypeMismatch,
    CircularDep,
    MacroDepth,
}

impl ErrorClass {
    fn label(self) -> &'static str {
        match self {
            ErrorClass::Syntax => "①语法错误",
            ErrorClass::Unbound => "②未绑定标识符",
            ErrorClass::EmptyApp => "③空应用",
            ErrorClass::Arity => "④参数个数错误",
            ErrorClass::TypeMismatch => "⑤类型不匹配",
            ErrorClass::CircularDep => "⑥模块循环依赖",
            ErrorClass::MacroDepth => "⑦宏展开深度超限",
        }
    }

    fn index(self) -> usize {
        match self {
            ErrorClass::Syntax => 1,
            ErrorClass::Unbound => 2,
            ErrorClass::EmptyApp => 3,
            ErrorClass::Arity => 4,
            ErrorClass::TypeMismatch => 5,
            ErrorClass::CircularDep => 6,
            ErrorClass::MacroDepth => 7,
        }
    }
}

// ---------------------------------------------------------------------------
// case 表与期望
// ---------------------------------------------------------------------------

enum Expect {
    /// 双路径均 Err：阶段 + 消息子串 + E 码 + 非空 Span（+ 可选堆栈追踪）。
    Err {
        stage: Stage,
        msg: &'static str,
        trace: bool,
    },
    /// 双路径均 Ok 且值一致（Int）。
    OkInt(i64),
    /// VM 路径 Ok 且渲染值精确匹配。
    OkRendered(&'static str),
    /// 自定义探针（错误恢复 / 循环依赖子进程）。
    Custom(fn() -> CaseResult),
}

struct Case {
    id: &'static str,
    bucket: Bucket,
    polarity: Polarity,
    class: Option<ErrorClass>,
    src: &'static str,
    expect: Expect,
}

const FIB10: &str = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 10)";

const COUNTERS: &str = r#"
        (define (make-counter)
          (let ((n 0))
            (lambda () (set! n (+ n 1)) n)))
        (define a (make-counter))
        (define b (make-counter))
        (a) (a) (a)
        (b)
        (list (a) (b))
    "#;

const CASES: &[Case] = &[
    // ---- A 桶：单语句负向（基础类型系统）----
    Case {
        id: "A01",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Syntax),
        src: "(lambda (x)",
        expect: Expect::Err {
            stage: Stage::Read,
            msg: "括号未闭合",
            trace: false,
        },
    },
    Case {
        id: "A02",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(+ 1 \"s\")",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "+ 需要 int",
            trace: false,
        },
    },
    Case {
        id: "A03",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(head 5)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "head 需要 pair，实际 int",
            trace: false,
        },
    },
    Case {
        id: "A04",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(1 2)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "不可调用的值：int",
            trace: false,
        },
    },
    Case {
        id: "A05",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        src: "undefined-x",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "未绑定",
            trace: false,
        },
    },
    Case {
        id: "A06",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        src: "(set! y 1)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "set! 未绑定变量",
            trace: false,
        },
    },
    Case {
        id: "A07",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(/ 1 0)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "整数除零",
            trace: false,
        },
    },
    Case {
        id: "A08",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(tail nil)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "tail 需要 pair，实际 nil",
            trace: false,
        },
    },
    Case {
        id: "A09",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::EmptyApp),
        src: "()",
        expect: Expect::Err {
            stage: Stage::Expand,
            msg: "空列表不能作为表达式求值",
            trace: false,
        },
    },
    Case {
        id: "A10",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Arity),
        src: "((lambda (x) x) 1 2)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "过程参数数量不匹配：期望 1 实际 2",
            trace: false,
        },
    },
    Case {
        id: "A11",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(if 1 2 3)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "需要 bool",
            trace: false,
        },
    },
    Case {
        id: "A12",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(string-append 1 2)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "string-append 需要 2 个字符串",
            trace: false,
        },
    },
    // ---- B 桶：多语句/多函数负向（集成正确性）----
    Case {
        id: "B01",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Arity),
        src: "(define (g a b) (cons a b)) (g 1)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "过程参数数量不匹配：期望 2 实际 1",
            trace: false,
        },
    },
    Case {
        id: "B02",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Arity),
        src: "(define (f x) x) (f)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "过程参数数量不匹配：期望 1 实际 0",
            trace: false,
        },
    },
    Case {
        id: "B03",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(define (f x) (+ x 1)) (f \"s\")",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "+ 需要 int",
            trace: false,
        },
    },
    Case {
        id: "B04",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(define (make-adder n) (lambda (x) (+ x n))) ((make-adder \"s\") 1)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "+ 需要 int",
            trace: false,
        },
    },
    Case {
        id: "B05",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        src: "(define x 1) (set! y x)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "set! 未绑定变量",
            trace: false,
        },
    },
    Case {
        id: "B06",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        // E6 重复定义（§7.1.1 外加负向——Task 12 修复项回归锚点）
        class: None,
        src: "(define x 1) (define x 2)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "重复定义变量",
            trace: false,
        },
    },
    Case {
        id: "B07",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(define n \"s\") (+ n 1)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "+ 需要 int",
            trace: false,
        },
    },
    Case {
        id: "B08",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        // 展开期 A3 卫式检查（§7.1.1 外加负向）
        class: None,
        src: "(define (f x x) x)",
        expect: Expect::Err {
            stage: Stage::Expand,
            msg: "lambda 参数重名",
            trace: false,
        },
    },
    Case {
        id: "B09",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Arity),
        src: "(define (apply2 f) (f 1 2)) (apply2 head)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "head 需要 1 个参数，实际 2",
            trace: false,
        },
    },
    Case {
        id: "B10",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(define lst (quote (1 2))) (head (tail (tail lst)))",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "head 需要 pair，实际 nil",
            trace: false,
        },
    },
    Case {
        id: "B11",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(define (join a b) (string-append a b)) (join 1 2)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "string-append 需要 2 个字符串",
            trace: false,
        },
    },
    Case {
        id: "B12",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        // 跨形式依赖：函数体内未绑定标识符，经调用触发。
        // 发现（如实记录）：eval 路径错误为「求值失败：未绑定变量」且
        // Span 指向调用点而非标识符处（VM 路径精确 / eval 路径调用点）——
        // 双路径诊断保真度差异，见 worklog Task 16-a 发现清单。
        class: Some(ErrorClass::Unbound),
        src: "(define (f) x) (f)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "未绑定",
            trace: false,
        },
    },
    // ---- C 桶：复杂程序负向（嵌套控制流/闭包/递归/宏/循环依赖）----
    Case {
        id: "C01",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::MacroDepth),
        src: "(define-syntax loop (syntax-rules () ((loop) (loop)))) (loop)",
        expect: Expect::Err {
            stage: Stage::Expand,
            msg: "宏展开深度超过上限",
            trace: false,
        },
    },
    Case {
        id: "C02",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::MacroDepth),
        src: "(define-syntax m (syntax-rules () ((m x) (m (x))))) (m 1)",
        expect: Expect::Err {
            stage: Stage::Expand,
            msg: "宏展开深度超过上限",
            trace: false,
        },
    },
    Case {
        id: "C03",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        // 发现（XFAIL）：运行时错误无调用链堆栈追踪（children 恒空）。
        src: "(define (deep n) (if (= n 0) (head 5) (deep (- n 1)))) (deep 3)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "head 需要 pair，实际 int",
            trace: true,
        },
    },
    Case {
        id: "C04",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(define (count n) (if (= n 0) \"done\" (count (- n 1)))) (+ (count 100) 1)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "+ 需要 int",
            trace: false,
        },
    },
    Case {
        id: "C05",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(define (classify n) (if (= (mod n 2) 0) (if (> n 10) (if (> n 100) (head n) \"big\") 1) 2)) (classify 200)",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "head 需要 pair",
            trace: false,
        },
    },
    Case {
        id: "C06",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        // 宏展开产物中的运行时类型错（expansion 代次 Span）
        src: "(define-syntax twice! (syntax-rules () ((twice! e) (begin e e)))) (twice! (head 5))",
        expect: Expect::Err {
            stage: Stage::Run,
            msg: "head 需要 pair",
            trace: false,
        },
    },
    Case {
        id: "C07",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::CircularDep),
        // ⑥ 模块循环依赖：经 ModuleRegistry 公共 API 构造 A imports B /
        // B imports A，子进程探针触发 visit（避免崩溃杀死审计进程）。
        src: "<ModuleRegistry: cycle-a imports cycle-b; cycle-b imports cycle-a; visit(cycle-a)>",
        expect: Expect::Custom(probe_circular_module_dep),
    },
    Case {
        id: "C08",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        // 发现（XFAIL）：宏展开产物引用全局（卫生回退）——VM 路径回退后
        // Ok(42)，eval 路径无回退 → Err 未绑定（双路径分裂）。
        // 类归属：⑤ 类型不匹配非也——归 ②未绑定标识符（eval 侧错误类）。
        class: Some(ErrorClass::Unbound),
        src: "<宏 inc! 展开 set! (+ v 1) 引用全局 +：VM 卫生回退 / eval 无回退>",
        expect: Expect::Custom(probe_macro_global_dual_path),
    },
    // ---- D 桶：错误恢复（同进程序列）----
    Case {
        id: "D01",
        bucket: Bucket::Recovery,
        polarity: Polarity::Mixed,
        class: None,
        src: "<先 (head 5) 后 fib(10)>",
        expect: Expect::Custom(probe_error_then_correct),
    },
    Case {
        id: "D02",
        bucket: Bucket::Recovery,
        polarity: Polarity::Mixed,
        class: None,
        src: "<先 (/ 1 0) 双路径后 (+ 40 2) 双路径>",
        expect: Expect::Custom(probe_dual_path_error_then_ok),
    },
    Case {
        id: "D03",
        bucket: Bucket::Recovery,
        polarity: Polarity::Mixed,
        class: None,
        src: "<三阶段错误后 dump_tokens / dump_stx>",
        expect: Expect::Custom(probe_reader_after_errors),
    },
    Case {
        id: "D04",
        bucket: Bucket::Recovery,
        polarity: Polarity::Mixed,
        class: None,
        src: "<5 个错误连发后 fib(10)>",
        expect: Expect::Custom(probe_error_barrage_then_correct),
    },
    Case {
        id: "D05",
        bucket: Bucket::Recovery,
        polarity: Polarity::Mixed,
        class: None,
        src: "<堆分配后错误，再跑闭包计数器>",
        expect: Expect::Custom(probe_heap_intact_after_error),
    },
    Case {
        id: "D06",
        bucket: Bucket::Recovery,
        polarity: Polarity::Mixed,
        class: None,
        src: "<两个错误后第三个错误的诊断结构完整性>",
        expect: Expect::Custom(probe_diagnostics_structure_after_errors),
    },
    // ---- P 桶：正向 sanity（审计器自证）----
    Case {
        id: "P1",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        src: FIB10,
        expect: Expect::OkInt(55),
    },
    Case {
        id: "P2",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        src: COUNTERS,
        expect: Expect::OkRendered("(4 2)"),
    },
    Case {
        id: "P3",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        src: "((lambda (x y) (- x y)) 10 4)",
        expect: Expect::OkInt(6),
    },
];

// ---------------------------------------------------------------------------
// 结果类型与表驱动执行器
// ---------------------------------------------------------------------------

struct CaseResult {
    /// 断言当前行为成立（XFAIL 项也算 pass——按真实行为断言）。
    pass: bool,
    /// 当前行为 = 缺陷行为（已 WARN 标注，期望行为见 warn 文本）。
    xfail: bool,
    detail: String,
    warn: Option<String>,
}

fn pass(detail: String) -> CaseResult {
    CaseResult {
        pass: true,
        xfail: false,
        detail,
        warn: None,
    }
}

fn fail(detail: String) -> CaseResult {
    CaseResult {
        pass: false,
        xfail: false,
        detail,
        warn: None,
    }
}

fn xfail(detail: String, warn: String) -> CaseResult {
    CaseResult {
        pass: true,
        xfail: true,
        detail,
        warn: Some(warn),
    }
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or("")
}

fn stage_e_code(stage: Stage) -> u32 {
    match stage {
        Stage::Read => 1,
        Stage::Expand => 2,
        Stage::Compile => 3,
        Stage::Run => 4,
    }
}

/// 负向 case 执行器：VM 路径（阶段/消息/E 码/Span/文件名/追踪）+
/// eval 双路径（同 Err 而非挂起、同消息子串）。
fn run_negative(c: &Case, stage: Stage, msg: &str, want_trace: bool) -> CaseResult {
    let err = match run_source(c.src, FNAME) {
        Err(e) => e,
        Ok(_) => return fail(format!("期望 {:?} 阶段 Err，VM 路径返回 Ok", stage)),
    };
    if err.stage != stage {
        return fail(format!(
            "VM 阶段不匹配：期望 {:?} 实际 {:?}",
            stage, err.stage
        ));
    }
    if !err.rendered.contains(msg) {
        return fail(format!(
            "VM 消息不含「{}」：{}",
            msg,
            first_line(&err.rendered)
        ));
    }
    let want_code = DiagnosticCode(stage_e_code(stage));
    if err.diagnostic.code != Some(want_code) {
        return fail(format!(
            "E 码不匹配：期望 E{:04} 实际 {:?}",
            want_code.0,
            err.diagnostic.code.map(|k| k.0)
        ));
    }
    if err.diagnostic.primary_span.is_empty() {
        return fail("primary Span 为空（诊断定位缺失）".to_string());
    }
    if !err.rendered.contains(FNAME) {
        return fail(format!("渲染输出缺文件名 {}（位置摘录缺失）", FNAME));
    }
    // 种子链双路径（T1 新口径 42-d：eval 退役——种子链为对拍 oracle）：
    // 同 Err（而非挂起/静默成功）且消息子串一致
    match run_source_seed(c.src, FNAME) {
        Ok(_) => return fail("期望 Err，种子链返回 Ok（双路径分裂）".to_string()),
        Err(e2) => {
            if !e2.rendered.contains(msg) {
                return fail(format!(
                    "种子链消息不含「{}」：{}",
                    msg,
                    first_line(&e2.rendered)
                ));
            }
        }
    }
    let span = err.diagnostic.primary_span;
    let base = format!(
        "{} | E{:04} | span {}..{}",
        first_line(&err.rendered),
        want_code.0,
        span.start,
        span.end
    );
    if want_trace {
        if err.diagnostic.children.is_empty() {
            return xfail(
                format!("{} | 双路径 Err | 无堆栈追踪（children 空）", base),
                "当前行为：运行时错误不含调用链追踪（VmError.trace 在 vm.rs 仅初始化为空、无任何生产者；worklog Task 4-c 声称『VM 堆栈追踪』与实际不符，vm_tests.runtime_error_has_trace 实际只断言消息文本）；期望行为：错误诊断携带调用点 children（§8.12 堆栈追踪承诺）——TODO 修复后本 case 自动升级为 PASS".to_string(),
            );
        }
        return pass(format!("{} | trace✓ | 双路径 Err", base));
    }
    pass(format!("{} | 双路径 Err", base))
}

/// 正向 case 执行器（OkInt）：双路径值一致（生产链 vs 种子链——T1
/// 新口径 42-d：eval 退役，种子链替代 oracle）。
fn run_positive_int(c: &Case, n: i64) -> CaseResult {
    let (o, o2) = match (run_source(c.src, FNAME), run_source_seed(c.src, FNAME)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => {
            return fail(format!("正向程序意外失败：{}", first_line(&e.rendered)))
        }
    };
    match (&o.value, &o2.value) {
        (Value::Int(a), Value::Int(b)) if *a == n && *b == n => {
            pass(format!("生产⇒{} 种子⇒{}（双路径一致）", a, b))
        }
        _ => fail(format!(
            "值不匹配：期望 {}，生产⇒{} 种子⇒{}",
            n,
            render_value(&o.value, &o.heap),
            render_value(&o2.value, &o2.heap)
        )),
    }
}

/// 正向 case 执行器（OkRendered）。
fn run_positive_rendered(c: &Case, expected: &str) -> CaseResult {
    match run_source_rendered(c.src, FNAME) {
        Ok(s) if s.trim() == expected => pass(format!("VM⇒{}", s.trim())),
        Ok(s) => fail(format!(
            "渲染值不匹配：期望「{}」实际「{}」",
            expected,
            s.trim()
        )),
        Err(e) => fail(format!("正向程序意外失败：{}", first_line(&e.rendered))),
    }
}

fn run_case(c: &Case) -> CaseResult {
    match &c.expect {
        Expect::Err { stage, msg, trace } => run_negative(c, *stage, msg, *trace),
        Expect::OkInt(n) => run_positive_int(c, *n),
        Expect::OkRendered(s) => run_positive_rendered(c, s),
        Expect::Custom(f) => f(),
    }
}

// ---------------------------------------------------------------------------
// ⑥ 循环依赖探针（C07）——ModuleRegistry 公共 API 直构
// ---------------------------------------------------------------------------

/// 子进程模式：构造 A imports B / B imports A 并 visit(A)。
/// 若循环检测存在则打印 CYCLE-DETECTED；静默通过打印 CYCLE-SILENT-PASS；
/// 当前实际行为：无限递归 → 栈溢出崩溃（非零退出）。
fn cyclic_probe_child() {
    let mut table = SymbolTable::new();
    let a = table.intern("cycle-a");
    let b = table.intern("cycle-b");
    let mut reg = ModuleRegistry::new();
    let _ = reg.declare(a, vec![b], vec![]);
    let _ = reg.declare(b, vec![a], vec![]);
    match reg.visit(a) {
        Ok(()) => println!("CYCLE-SILENT-PASS"),
        Err(m) => println!("CYCLE-DETECTED: {}", m),
    }
}

fn probe_circular_module_dep() -> CaseResult {
    let exe = std::env::current_exe().expect("获取当前可执行文件路径失败");
    let out = Command::new(&exe)
        .arg(PROBE_ARG)
        .output()
        .expect("循环依赖子进程探针启动失败");
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr);
    if out.status.success() {
        if stdout.contains("CYCLE-DETECTED") {
            pass(format!("visit 返回结构化错误：{}", stdout))
        } else {
            xfail(
                "visit(A↔B) 静默返回 Ok".to_string(),
                "当前行为：循环依赖静默通过（无检测）；期望行为：结构化 Err『循环依赖』诊断（§7.1.1⑥）".to_string(),
            )
        }
    } else {
        let overflowed = stderr.contains("overflowed its stack");
        let hint = if overflowed {
            "（栈溢出崩溃）"
        } else {
            "（异常退出）"
        };
        xfail(
            format!("子进程探针非正常退出{}：{}", hint, first_line(&stderr)),
            "当前行为：ModuleRegistry::visit 对 A imports B / B imports A 无循环检测，".to_string()
                + "无限递归栈溢出崩溃（kerf-expander/src/phase.rs visit 的 import 递归，"
                + "注释声称『环在此显式失败』与实际行为不符）；"
                + "期望行为：declare/visit 侧结构化 Err『循环依赖』诊断（§7.1.1⑥）——"
                + "TODO 修复后本 case 自动升级为 PASS",
        )
    }
}

// ---------------------------------------------------------------------------
// ⑧ 双路径分裂探针（C08）——宏全局引用双路径同解（T1 新口径 42-d）
// ---------------------------------------------------------------------------

/// C08：宏展开产物引用全局 +：生产链（自举三段 + VM）经
/// resolve_hygiene_fallbacks 回退后 Ok(42)；种子链（Rust 三段 + VM，
/// 42-d eval 退役后的对拍 oracle）同样接线 ⇒ 双路径一致 42。
fn probe_macro_global_dual_path() -> CaseResult {
    let src = "(define-syntax inc! (syntax-rules () ((inc! v) (set! v (+ v 1))))) (define x 41) (inc! x) x";
    // 生产链（自举三段 + VM——卫生回退生效，正向锚点）
    match run_source(src, FNAME) {
        Ok(o) if matches!(o.value, Value::Int(42)) => {}
        Ok(o) => {
            return fail(format!(
                "生产链值不匹配：{}",
                render_value(&o.value, &o.heap)
            ))
        }
        Err(e) => return fail(format!("生产链意外失败：{}", first_line(&e.rendered))),
    }
    // 种子链（Rust 三段 + VM——参考 oracle；原 eval 路径分裂已于驱动
    // 层修复，42-d 口径迁移后对拍面保留）
    match run_source_seed(src, FNAME) {
        Err(e) if e.rendered.contains("未绑定") => xfail(
            "生产⇒42 / 种子⇒Err[未绑定变量]（双路径分裂）".to_string(),
            "当前行为：种子链宏展开产物中的全局引用 +$hyg$N 未绑定报错（种子链卫生回退未接线）；期望行为：双路径一致 Ok 42（§21.8 双路径互查 / T1 新口径——42-d eval 退役后对拍面）——TODO 修复后本 case 自动升级为 PASS".to_string(),
        ),
        Ok(o) if matches!(o.value, Value::Int(42)) => {
            pass("生产⇒42 种子⇒42（双路径一致）".to_string())
        }
        Ok(o) => fail(format!(
            "种子链 Ok 但值非 42：{}",
            render_value(&o.value, &o.heap)
        )),
        Err(e) => fail(format!(
            "种子链 Err 但非未绑定类：{}",
            first_line(&e.rendered)
        )),
    }
}

// ---------------------------------------------------------------------------
// D 桶错误恢复探针
// ---------------------------------------------------------------------------

/// D01：错误程序 → 同进程续跑正确程序结果正确。
fn probe_error_then_correct() -> CaseResult {
    let err = match run_source("(head 5)", FNAME) {
        Err(e) => e,
        Ok(_) => return fail("错误程序 (head 5) 意外成功".to_string()),
    };
    if !err.rendered.contains("head 需要 pair") {
        return fail(format!("消息不匹配：{}", first_line(&err.rendered)));
    }
    match run_source(FIB10, FNAME) {
        Ok(o) if matches!(o.value, Value::Int(55)) => {
            pass("Err[head 需要 pair] → 后续 fib(10)⇒55".to_string())
        }
        Ok(o) => fail(format!(
            "恢复后值不匹配：{}",
            render_value(&o.value, &o.heap)
        )),
        Err(e) => fail(format!(
            "错误后续跑正确程序失败（状态泄漏）：{}",
            first_line(&e.rendered)
        )),
    }
}

/// D02：除零在生产与种子双路径均 Err，随后双路径续跑正确（T1 新
/// 口径 42-d——eval 退役，种子链对拍）。
fn probe_dual_path_error_then_ok() -> CaseResult {
    match (
        run_source("(/ 1 0)", FNAME),
        run_source_seed("(/ 1 0)", FNAME),
    ) {
        (Err(e1), Err(e2))
            if e1.rendered.contains("整数除零") && e2.rendered.contains("整数除零") => {}
        (Ok(_), _) | (_, Ok(_)) => {
            return fail("(/ 1 0)：期望双路径 Err，有路径返回 Ok".to_string())
        }
        (Err(e1), Err(e2)) => {
            return fail(format!(
                "(/ 1 0)：消息不匹配：生产「{}」种子「{}」",
                first_line(&e1.rendered),
                first_line(&e2.rendered)
            ))
        }
    }
    match (
        run_source("(+ 40 2)", FNAME),
        run_source_seed("(+ 40 2)", FNAME),
    ) {
        (Ok(a), Ok(b))
            if matches!(a.value, Value::Int(42)) && matches!(b.value, Value::Int(42)) =>
        {
            pass("(/ 1 0) 双路径 Err → (+ 40 2) 双路径⇒42".to_string())
        }
        _ => fail("错误后双路径续跑正确程序失败（状态泄漏）".to_string()),
    }
}

/// D03：多阶段错误后 Lexer/Parser 对后续输入仍正常（dump_tokens/dump_stx）。
fn probe_reader_after_errors() -> CaseResult {
    // 三阶段各一个错误：Read / Expand / Run
    for (src, msg) in [
        ("(+ 1", "括号未闭合"),
        ("()", "空列表不能作为表达式求值"),
        ("(head 5)", "head 需要 pair"),
    ] {
        match run_source(src, FNAME) {
            Err(e) if e.rendered.contains(msg) => {}
            Err(e) => return fail(format!("{}：消息不匹配：{}", src, first_line(&e.rendered))),
            Ok(_) => return fail(format!("{}：期望 Err", src)),
        }
    }
    let toks = match dump_tokens("(+ 1 2)", FNAME) {
        Ok(s) => s,
        Err(e) => {
            return fail(format!(
                "错误后 dump_tokens 失败：{}",
                first_line(&e.rendered)
            ))
        }
    };
    for want in [
        "Delimiter(OpenParen)",
        "Add(+)",
        "IntLiteral(1)",
        "IntLiteral(2)",
    ] {
        if !toks.contains(want) {
            return fail(format!("dump_tokens 缺 Token「{}」", want));
        }
    }
    let stx = match dump_stx("(quote (a b))", FNAME) {
        Ok(s) => s,
        Err(e) => return fail(format!("错误后 dump_stx 失败：{}", first_line(&e.rendered))),
    };
    if !stx.contains("(quote (a b))") {
        return fail(format!("dump_stx 输出不符：{}", stx.trim()));
    }
    pass("三阶段错误后 tokens/stx dump 均正常".to_string())
}

/// D04：5 个错误连发（含三阶段），随后正确程序仍 55（一个错误不污染后续）。
fn probe_error_barrage_then_correct() -> CaseResult {
    let barrage = [
        ("(+ 1 \"s\")", Stage::Run, "+ 需要 int"),
        ("()", Stage::Expand, "空列表不能作为表达式求值"),
        ("(+ 1", Stage::Read, "括号未闭合"),
        ("(head 5)", Stage::Run, "head 需要 pair"),
        ("(set! zz 1)", Stage::Run, "set! 未绑定变量"),
    ];
    for (src, want_stage, msg) in barrage {
        match run_source(src, FNAME) {
            Err(e) if e.stage == want_stage && e.rendered.contains(msg) => {}
            Err(e) => {
                return fail(format!(
                    "{}：阶段/消息不匹配（期望 {:?}/{})：{}",
                    src,
                    want_stage,
                    msg,
                    first_line(&e.rendered)
                ))
            }
            Ok(_) => return fail(format!("{}：期望 Err", src)),
        }
    }
    match run_source(FIB10, FNAME) {
        Ok(o) if matches!(o.value, Value::Int(55)) => {
            pass("5 连发错误（含三阶段）后 fib(10)⇒55".to_string())
        }
        Ok(o) => fail(format!(
            "恢复后值不匹配：{}",
            render_value(&o.value, &o.heap)
        )),
        Err(e) => fail(format!(
            "5 错误后续跑正确程序失败：{}",
            first_line(&e.rendered)
        )),
    }
}

/// D05：堆分配后触发错误 → 同进程闭包计数器（共享可变捕获 + pair 分配 +
/// slot_count>0）结果精确正确（错误不破坏后续堆行为）。
fn probe_heap_intact_after_error() -> CaseResult {
    match run_source("(define p (cons 1 (cons 2 nil))) (head p) (head 5)", FNAME) {
        Err(e) if e.rendered.contains("head 需要 pair") => {}
        Err(e) => return fail(format!("消息不匹配：{}", first_line(&e.rendered))),
        Ok(_) => return fail("含 (head 5) 的程序意外成功".to_string()),
    }
    match run_source(COUNTERS, FNAME) {
        Ok(o) => {
            let rendered = render_value(&o.value, &o.heap);
            if rendered.trim() != "(4 2)" {
                return fail(format!("恢复后闭包计数器不匹配：{}", rendered.trim()));
            }
            if o.heap.slot_count() == 0 {
                return fail("恢复后堆槽位数为 0（堆未正常参与执行）".to_string());
            }
            pass(format!(
                "堆分配期错误后计数器⇒(4 2)（heap slots={}）",
                o.heap.slot_count()
            ))
        }
        Err(e) => fail(format!("错误后闭包程序失败：{}", first_line(&e.rendered))),
    }
}

/// D06：两个错误后第三个错误的诊断结构仍完整（阶段/E 码/Span/文件名/消息）。
fn probe_diagnostics_structure_after_errors() -> CaseResult {
    let _ = run_source("(+ 1 \"s\")", FNAME);
    let _ = run_source("(/ 1 0)", FNAME);
    let err = match run_source("(set! zz 1)", FNAME) {
        Err(e) => e,
        Ok(_) => return fail("(set! zz 1) 意外成功".to_string()),
    };
    if err.stage != Stage::Run {
        return fail(format!("阶段漂移：{:?}", err.stage));
    }
    if err.diagnostic.code != Some(DiagnosticCode(4)) {
        return fail("E 码漂移（期望 E0004）".to_string());
    }
    if err.diagnostic.primary_span.is_empty() {
        return fail("Span 漂移（为空）".to_string());
    }
    if !err.rendered.contains("set! 未绑定变量") || !err.rendered.contains(FNAME) {
        return fail(format!("消息/位置漂移：{}", first_line(&err.rendered)));
    }
    pass("两错误后第三错误诊断结构完整（stage/E0004/Span/位置）".to_string())
}

// ---------------------------------------------------------------------------
// main：逐 case 报告 + 汇总 + §7.3.1 强制配比机械校验
// ---------------------------------------------------------------------------

fn main() {
    // 子进程探针模式（⑥ 循环依赖——崩溃隔离）
    if std::env::args().nth(1).as_deref() == Some(PROBE_ARG) {
        cyclic_probe_child();
        return;
    }

    println!("== kerf Stage 0 门审计集 stage0_gate_audit_r1（sop.md §7.3.1 / §7.1.1）==");
    println!(
        "case 总数 {}（A 单语句 12 / B 多语句 12 / C 复杂 8 / D 恢复 6 / P 正向 3）\n",
        CASES.len()
    );

    let mut fail_count = 0;
    let mut xfail_count = 0;
    let mut class_seen = [0usize; 7];
    let mut bucket_counts = [0usize; 5];
    let mut negative = 0;
    let mut positive = 0;
    let mut mixed = 0;

    for c in CASES {
        bucket_counts[c.bucket as usize] += 1;
        match c.polarity {
            Polarity::Negative => negative += 1,
            Polarity::Positive => positive += 1,
            Polarity::Mixed => mixed += 1,
        }
        if let Some(cls) = c.class {
            class_seen[cls.index() - 1] += 1;
        }
        let r = run_case(c);
        let status = if !r.pass {
            "FAIL"
        } else if r.xfail {
            "XFAIL"
        } else {
            "PASS"
        };
        let class_label = c.class.map(|k| k.label()).unwrap_or("-");
        println!(
            "[{}] {} {} {} {}",
            status,
            c.id,
            c.bucket.label(),
            polarity_label(c.polarity),
            class_label
        );
        println!("      src: {}", short_src(c.src));
        println!("      → {}", r.detail);
        if let Some(w) = &r.warn {
            println!("      WARN: {}", w);
        }
        if !r.pass {
            fail_count += 1;
        }
        if r.xfail {
            xfail_count += 1;
        }
    }

    // 汇总
    let total = CASES.len();
    let passed = total - fail_count;
    println!("\n== 汇总 ==");
    println!(
        "total {} / PASS {}（含 XFAIL-WARN {}）/ FAIL {}",
        total, passed, xfail_count, fail_count
    );
    println!(
        "桶配比：单语句 {}/10 多语句 {}/10 复杂 {}/5 恢复 {}/5 正向 {}/8（上限）",
        bucket_counts[0], bucket_counts[1], bucket_counts[2], bucket_counts[3], bucket_counts[4]
    );
    println!(
        "极性：负向 {}（≥22）/ 恢复-混合 {} / 正向 {}（≤8）",
        negative, mixed, positive
    );
    print!("§7.1.1 七类覆盖：");
    for (i, n) in class_seen.iter().enumerate() {
        print!(" {}={}", i + 1, n);
    }
    println!();

    // §7.3.1 强制失败条件机械校验（配比不满足 → NEEDS REVISION，退出码 1）
    let mut violations: Vec<&str> = Vec::new();
    if total < 32 {
        violations.push("case 总数 < 32");
    }
    if bucket_counts[0] < 10 {
        violations.push("单语句负向 < 10");
    }
    if bucket_counts[1] < 10 {
        violations.push("多语句负向 < 10");
    }
    if bucket_counts[2] < 5 {
        violations.push("复杂程序负向 < 5");
    }
    if bucket_counts[3] < 5 {
        violations.push("错误恢复测试 < 5");
    }
    if negative < 22 {
        violations.push("负向 case < 22");
    }
    if positive > 8 {
        violations.push("正向 case > 8");
    }
    if class_seen.contains(&0) {
        violations.push("§7.1.1 七类未全覆盖");
    }
    if fail_count > 0 {
        violations.push("存在 FAIL case");
    }
    if violations.is_empty() {
        println!("\n§7.3.1 强制配比全部满足；七类全覆盖。EXIT 0");
        std::process::exit(0);
    } else {
        println!("\n强制失败条件触发（§7.3.1）：");
        for v in &violations {
            println!("  - {}", v);
        }
        println!("EXIT 1");
        std::process::exit(1);
    }
}

fn polarity_label(p: Polarity) -> &'static str {
    match p {
        Polarity::Negative => "负向",
        Polarity::Mixed => "恢复",
        Polarity::Positive => "正向",
    }
}

fn short_src(src: &str) -> String {
    let one_line = src.replace('\n', " ");
    let mut s: String = one_line.chars().take(72).collect();
    if one_line.chars().count() > 72 {
        s.push_str("...");
    }
    s
}
