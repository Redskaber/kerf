//! Stage 1 负向门审计集（sop.md §7.3.1 规则 3 / §7.1.1 七类矩阵 /
//! §7.3.2 上轮修复边界 case / §21.3 阶段验收四条件）。
//!
//! **用途**：≥30 case 负向审计——Stage 1 阶段门审查（§7.3）强制审计集
//! （第 1 轮，r1）。审计全部经**生产管线**（E1-β 生产切换后：读 + 展开
//! 均自举——VM 上 reader.krf + expander.krf；Rust 种子 = parity oracle）
//! ——审计本身即 §21.3 条件 1/2 的持续验证载体。
//!
//! **运行方式**：工作区根目录 `cargo run --example stage1_gate_audit_r1`。
//! 全部 case PASS 时退出码 0；任一 FAIL 或 §7.3.1 强制配比不满足时退
//! 出码 1。
//!
//! **结构**（§7.3.1 规则 2 强制配比，配比在 main() 中机械校验）：
//! - A 桶 12：单语句负向（基础类型系统）
//! - B 桶 12：多语句/多函数负向（集成正确性——宏/模块/prelude 面）
//! - C 桶 10：复杂程序负向（嵌套控制流/闭包/递归/宏 + ⑥ 循环依赖
//!   + ⑦ 深度超限）
//! - D 桶 6：错误恢复（错误后同进程续跑正确程序）
//! - E 桶 6：上轮修复边界 case（§7.3.2——本批次修复面：Span 并集
//!   代次守卫 / require·set·define-syntax 代次 / 前导注入 / 生产切换
//!   活性 / 卫生计数器嵌套 / 深度上限生产路径）
//! - P 桶 4：正向 sanity + §21.3 四条件锚定（审计器自证）
//!
//! 合计 50 case：负向 34 + 恢复 6 + 边界 6 + 正向 4。
//!
//! **§7.1.1 七类覆盖**：①语法 ②未绑定 ③空应用 ④参数个数 ⑤类型不匹配
//! ⑥模块循环依赖（双模块源码 → registry DFS 灰标记）⑦宏展开深度超限
//! （自指宏经生产路径）。
//!
//! **§21.3 四条件锚定**（P 桶 + 全体审计面）：
//! - 条件 1（子集表达编译器前端）：生产管线本身（自举 Reader+Expander）
//!   + 宏正负例全过 = 表达力证明；
//! - 条件 2（Stage 0 VM 上正确运行）：自举 Expander 在 VM 上承载全部
//!   生产展开（每 case 隐式验证）；
//! - 条件 3（标准库最小集可用）：P02 prelude 管道（列表 hofs）+ 既有
//!   stdlib 套件（列表/字符串/I/O ≥8 函数各面，tests/v0/stage1/plan/
//!   stdlib_tests.rs）；
//! - 条件 4（增量编译基础设施可用）：P04 编译缓存确定性（重复编译同
//!   源一致）+ cache_tests.rs 内容寻址/失效面。
//!
//! 遵循条款：§7.3.1/§7.3.2（配比与边界）、§2.3-11（实测禁臆测——
//! 全部断言经生产管线真实运行）。

use kerf_driver::{check_source, compile_source, run_source, run_source_seed, Stage};
use kerf_span::DiagnosticCode;

/// 审计源文件名（诊断渲染位置断言用）。
const FNAME: &str = "audit.krf";

// ---------------------------------------------------------------------------
// 桶 / 极性 / §7.1.1 错误类
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Bucket {
    Single,
    Multi,
    Complex,
    Recovery,
    Boundary,
    Positive,
}

impl Bucket {
    fn label(self) -> &'static str {
        match self {
            Bucket::Single => "A单语句",
            Bucket::Multi => "B多语句",
            Bucket::Complex => "C复杂",
            Bucket::Recovery => "D恢复",
            Bucket::Boundary => "E边界",
            Bucket::Positive => "P正向",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Polarity {
    Negative,
    Positive,
    Recovery,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ErrorClass {
    Syntax,
    Unbound,
    EmptyForm,
    Arity,
    TypeMismatch,
    CircularDep,
    MacroDepth,
}

impl ErrorClass {
    fn index(self) -> usize {
        match self {
            ErrorClass::Syntax => 1,
            ErrorClass::Unbound => 2,
            ErrorClass::EmptyForm => 3,
            ErrorClass::Arity => 4,
            ErrorClass::TypeMismatch => 5,
            ErrorClass::CircularDep => 6,
            ErrorClass::MacroDepth => 7,
        }
    }
    fn label(self) -> &'static str {
        match self {
            ErrorClass::Syntax => "①语法",
            ErrorClass::Unbound => "②未绑定",
            ErrorClass::EmptyForm => "③空应用",
            ErrorClass::Arity => "④参数个数",
            ErrorClass::TypeMismatch => "⑤类型不匹配",
            ErrorClass::CircularDep => "⑥循环依赖",
            ErrorClass::MacroDepth => "⑦宏深度",
        }
    }
}

// ---------------------------------------------------------------------------
// 期望与结果
// ---------------------------------------------------------------------------

enum Expect {
    /// 双路径均 Err：阶段 + 消息子串（+ E 码 + 非空 Span 机械校验）。
    Err { stage: Stage, msg: &'static str },
    /// 双路径 Ok 且一致。
    OkDual(&'static str),
    /// 自定义探针（错误恢复 / §21.3 锚定）。
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

struct CaseResult {
    pass: bool,
    detail: String,
}

fn pass(detail: String) -> CaseResult {
    CaseResult { pass: true, detail }
}

fn fail(detail: String) -> CaseResult {
    CaseResult {
        pass: false,
        detail,
    }
}

fn short_src(src: &str) -> String {
    let one = src.replace('\n', " ⏎ ");
    if one.len() > 88 {
        format!("{}…", &one[..84])
    } else {
        one
    }
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").to_string()
}

// ---------------------------------------------------------------------------
// 语料常量
// ---------------------------------------------------------------------------

const FIB10: &str = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 10)";

const SWAP: &str = r#"(define-syntax swap!
  (syntax-rules ()
    ((swap! a b)
     (let ((tmp a))
       (begin (set! a b) (set! b tmp))))))
(define x 1) (define y 2)
(begin (swap! x y) (- x y))"#;

const MY_OR: &str = r#"(define-syntax my-or
  (syntax-rules ()
    ((my-or) false)
    ((my-or a rest ...) (if a a (my-or rest ...)))))"#;

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

const PRELUDE_PIPE: &str = r#"(module user (import kerf-prelude)
  (define lst (list 1 2 3 4 5))
  (foldl + 0 (map (lambda (x) (* x x)) (filter (lambda (x) (> x 2)) lst))))"#;

// ---------------------------------------------------------------------------
// 案例表（50 case）
// ---------------------------------------------------------------------------

const CASES: &[Case] = &[
    // ---- A 桶：单语句负向（基础类型系统，12）----
    Case {
        id: "A01",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Syntax),
        src: "(define-syntax m (syntax-rules () ((m x) x)",
        expect: Expect::Err { stage: Stage::Read, msg: "括号未闭合" },
    },
    Case {
        id: "A02",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::EmptyForm),
        src: "()",
        expect: Expect::Err { stage: Stage::Expand, msg: "空列表不能作为表达式求值" },
    },
    Case {
        id: "A03",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        src: "undefined_var",
        expect: Expect::Err { stage: Stage::Run, msg: "未绑定" },
    },
    Case {
        id: "A04",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(+ 1 \"string\")",
        expect: Expect::Err { stage: Stage::Run, msg: "需要 int" },
    },
    Case {
        id: "A05",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Syntax),
        src: "(syntax-rules () ((m x) x))",
        expect: Expect::Err { stage: Stage::Expand, msg: "syntax-rules 只能出现在 define-syntax 内部" },
    },
    Case {
        id: "A06",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: None,
        src: "(define-syntax)",
        expect: Expect::Err { stage: Stage::Expand, msg: "define-syntax 形式" },
    },
    Case {
        id: "A07",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: None,
        src: "(define-syntax m (syntax-rules ()))",
        expect: Expect::Err { stage: Stage::Expand, msg: "syntax-rules 至少需要一个子句" },
    },
    Case {
        id: "A08",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(if 1 2 3)",
        expect: Expect::Err { stage: Stage::Run, msg: "需要 bool" },
    },
    Case {
        id: "A09",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: None,
        src: "(quote [1 2])",
        expect: Expect::Err { stage: Stage::Expand, msg: "quote 向量暂不支持" },
    },
    Case {
        id: "A10",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: None,
        src: "(set! 1 2)",
        expect: Expect::Err { stage: Stage::Expand, msg: "set! 目标必须是符号" },
    },
    Case {
        id: "A11",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        src: "(define-syntax m (syntax-rules () ((m) undefined_thing))) (m)",
        expect: Expect::Err { stage: Stage::Run, msg: "未绑定" },
    },
    Case {
        id: "A12",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Arity),
        src: "((lambda (x) x) 1 2)",
        expect: Expect::Err { stage: Stage::Run, msg: "参数数量不匹配" },
    },
    // ---- B 桶：多语句/多函数负向（集成正确性，12）----
    Case {
        id: "B01",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: None,
        src: "(define-syntax m (syntax-rules () ((m x) x))) (m)",
        expect: Expect::Err { stage: Stage::Expand, msg: "宏调用与全部子句模式均不匹配" },
    },
    Case {
        id: "B02",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: None,
        src: "(define-syntax m (syntax-rules () ((m x y) x))) (m 1)",
        expect: Expect::Err { stage: Stage::Expand, msg: "宏「m」展开失败" },
    },
    Case {
        id: "B03",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Syntax),
        src: "(define-syntax m (syntax-rules (1) ((m x) x))) (m 1)",
        expect: Expect::Err { stage: Stage::Expand, msg: "syntax-rules 字面量必须是符号" },
    },
    Case {
        id: "B04",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: None,
        src: "(define-syntax m (syntax-rules () ((m x) x))) (m 1) (m)",
        expect: Expect::Err { stage: Stage::Expand, msg: "均不匹配" },
    },
    Case {
        id: "B05",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        src: "(define (f) (set! y 1)) (f)",
        expect: Expect::Err { stage: Stage::Run, msg: "未绑定" },
    },
    Case {
        id: "B06",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Arity),
        src: "(define (f a b) (+ a b)) (f 1)",
        expect: Expect::Err { stage: Stage::Run, msg: "参数数量不匹配" },
    },
    Case {
        id: "B07",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: None,
        src: "(module user (import nonexistent-module) 1)",
        expect: Expect::Err { stage: Stage::Expand, msg: "未声明的模块" },
    },
    Case {
        id: "B08",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: None,
        src: "(module user (import kerf-prelude) (define (map f l) l) (map car (list 1)))",
        expect: Expect::Err { stage: Stage::Run, msg: "重复定义" },
    },
    Case {
        id: "B09",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(define (f x) (+ x 1)) (f \"str\")",
        expect: Expect::Err { stage: Stage::Run, msg: "需要 int" },
    },
    Case {
        id: "B10",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: None,
        src: "(lambda (x) (define y 1) (define z 2) (+ x y) (define w 3))",
        expect: Expect::Err { stage: Stage::Expand, msg: "define 必须位于函数体头部" },
    },
    Case {
        id: "B11",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: None,
        src: "(define-syntax m (syntax-rules () ((m (a)) a))) (m 1)",
        expect: Expect::Err { stage: Stage::Expand, msg: "宏「m」展开失败" },
    },
    Case {
        id: "B12",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::EmptyForm),
        src: "(define x ()) x",
        expect: Expect::Err { stage: Stage::Expand, msg: "空列表不能作为表达式求值" },
    },
    // ---- C 桶：复杂程序负向（嵌套控制流/闭包/递归/宏，10）----
    Case {
        id: "C01",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::CircularDep),
        src: "(module a (import b) 1) (module b (import a) 1)",
        expect: Expect::Err { stage: Stage::Expand, msg: "模块循环依赖" },
    },
    Case {
        id: "C02",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::MacroDepth),
        src: "(define-syntax loop2 (syntax-rules () ((loop2) (loop2)))) (loop2)",
        expect: Expect::Err { stage: Stage::Expand, msg: "宏展开深度超过上限 10000" },
    },
    Case {
        id: "C03",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        src: "(define (deep n) (if (= n 0) missing-base (+ (deep (- n 1)) 1))) (deep 10)",
        expect: Expect::Err { stage: Stage::Run, msg: "未绑定" },
    },
    Case {
        id: "C04",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Arity),
        src: "(define (ack m n) (if (= m 0) (+ n 1) (if (= n 0) (ack (- m 1) 1) (ack (- m 1) (ack m (- n 1)))))) (ack 1)",
        expect: Expect::Err { stage: Stage::Run, msg: "参数数量不匹配" },
    },
    Case {
        id: "C05",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(define (f n acc) (if (= n 0) acc (f (- n 1) (+ acc 1)))) (f 10 \"seed\")",
        expect: Expect::Err { stage: Stage::Run, msg: "需要 int" },
    },
    Case {
        id: "C06",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: None,
        src: "(let ((f (lambda (x) (g x)))) (f 1))",
        expect: Expect::Err { stage: Stage::Run, msg: "未绑定" },
    },
    Case {
        id: "C07",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: None,
        src: "(define-syntax m (syntax-rules () ((m x) (m (m x))))) (m 1)",
        expect: Expect::Err { stage: Stage::Expand, msg: "宏展开深度超过上限 10000" },
    },
    Case {
        id: "C08",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: None,
        src: "(module user (import kerf-prelude) (foldl + 0))",
        expect: Expect::Err { stage: Stage::Run, msg: "参数数量不匹配" },
    },
    Case {
        id: "C09",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::EmptyForm),
        src: "(define (f) (let ((x ())) x)) (f)",
        expect: Expect::Err { stage: Stage::Expand, msg: "空列表不能作为表达式求值" },
    },
    Case {
        id: "C10",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: None,
        src: "(while true (set! x (+ x 1)))",
        expect: Expect::Err { stage: Stage::Run, msg: "未绑定" },
    },
    // ---- D 桶：错误恢复（错误后同进程续跑，6）----
    Case {
        id: "D01",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: None,
        src: "(+ 1",
        expect: Expect::Custom(probe_read_recovery),
    },
    Case {
        id: "D02",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: None,
        src: "(define-syntax m (syntax-rules (1) ((m x) x))) (m 1)",
        expect: Expect::Custom(probe_expand_recovery),
    },
    Case {
        id: "D03",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: None,
        src: "(module a (import b) 1) (module b (import a) 1)",
        expect: Expect::Custom(probe_cycle_recovery),
    },
    Case {
        id: "D04",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: None,
        src: "(define-syntax loop2 (syntax-rules () ((loop2) (loop2)))) (loop2)",
        expect: Expect::Custom(probe_depth_recovery),
    },
    Case {
        id: "D05",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: None,
        src: "(undefined-one)",
        expect: Expect::Custom(probe_run_recovery),
    },
    Case {
        id: "D06",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: None,
        src: "((lambda (x) x) 1 2)",
        expect: Expect::Custom(probe_arity_recovery),
    },
    // ---- E 桶：上轮修复边界 case（§7.3.2——本批次修复面，6）----
    Case {
        id: "E01",
        bucket: Bucket::Boundary,
        polarity: Polarity::Positive,
        class: None,
        src: SWAP,
        expect: Expect::OkDual("1"),
    },
    Case {
        id: "E02",
        bucket: Bucket::Boundary,
        polarity: Polarity::Positive,
        class: None,
        src: "(require io) (quote 1)",
        expect: Expect::Err { stage: Stage::Expand, msg: "require 形式" },
    },
    Case {
        id: "E03",
        bucket: Bucket::Boundary,
        polarity: Polarity::Positive,
        class: None,
        src: "(define x 1) (set! x 2) x",
        expect: Expect::OkDual("2"),
    },
    Case {
        id: "E04",
        bucket: Bucket::Boundary,
        polarity: Polarity::Positive,
        class: None,
        src: PRELUDE_PIPE,
        expect: Expect::OkDual("50"),
    },
    Case {
        id: "E05",
        bucket: Bucket::Boundary,
        polarity: Polarity::Positive,
        class: None,
        src: "(module m (define-syntax f (syntax-rules () ((f x) x))) (f 1))",
        expect: Expect::OkDual("1"),
    },
    Case {
        id: "E06",
        bucket: Bucket::Boundary,
        polarity: Polarity::Positive,
        class: None,
        src: "(define-syntax swap!
  (syntax-rules ()
    ((swap! a b) (let ((tmp a)) (begin (set! a b) (set! b tmp))))))
(let ((tmp 10) (u 3) (v 4)) (begin (swap! u v) (+ u v tmp)))",
        expect: Expect::Custom(probe_macro_in_sugar_span_boundary),
    },
    // ---- P 桶：正向 sanity + §21.3 锚定（4）----
    Case {
        id: "P01",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        src: FIB10,
        expect: Expect::OkDual("55"),
    },
    Case {
        id: "P02",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        src: PRELUDE_PIPE,
        expect: Expect::OkDual("50"),
    },
    Case {
        id: "P03",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        src: MY_OR,
        expect: Expect::Custom(probe_macro_or_positive),
    },
    Case {
        id: "P04",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        src: FIB10,
        expect: Expect::Custom(probe_incremental_compile_determinism),
    },
];

// ---------------------------------------------------------------------------
// 运行器
// ---------------------------------------------------------------------------

fn run_case(c: &Case) -> CaseResult {
    match &c.expect {
        Expect::Err { stage, msg } => run_negative(c, *stage, msg),
        Expect::OkDual(s) => run_positive_dual(c, s),
        Expect::Custom(f) => f(),
    }
}

fn run_negative(c: &Case, stage: Stage, msg: &str) -> CaseResult {
    let err = match run_source(c.src, FNAME) {
        Err(e) => e,
        Ok(_) => return fail(format!("期望 {:?} 阶段 Err，VM 路径返回 Ok", stage)),
    };
    if err.stage != stage {
        return fail(format!(
            "VM 阶段不匹配：期望 {:?} 实际 {:?}（{}）",
            stage,
            err.stage,
            first_line(&err.rendered)
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
    // 同 Err（而非静默成功/挂起）且阶段 + 消息子串一致
    match run_source_seed(c.src, FNAME) {
        Ok(_) => fail("期望 Err，种子链返回 Ok（双路径分裂）".to_string()),
        Err(e2) => {
            if e2.stage != stage {
                fail(format!(
                    "种子链阶段不匹配：期望 {:?} 实际 {:?}",
                    stage, e2.stage
                ))
            } else if !e2.rendered.contains(msg) {
                fail(format!(
                    "种子链消息不含「{}」：{}",
                    msg,
                    first_line(&e2.rendered)
                ))
            } else {
                pass(format!("双路径同 Err（E{:04} + Span 非空）", want_code.0))
            }
        }
    }
}

fn run_positive_dual(c: &Case, want: &str) -> CaseResult {
    let a = match run_source(c.src, FNAME) {
        Ok(o) => o,
        Err(e) => return fail(format!("生产链 Err：{}", first_line(&e.rendered))),
    };
    let b = match run_source_seed(c.src, FNAME) {
        Ok(o) => o,
        Err(e) => return fail(format!("种子链 Err：{}", first_line(&e.rendered))),
    };
    let ra = kerf_vm::render_value(&a.value, &a.heap);
    let rb = kerf_vm::render_value(&b.value, &b.heap);
    if ra != want || rb != want {
        return fail(format!(
            "值不匹配：期望 {}，生产 {} / 种子 {}",
            want, ra, rb
        ));
    }
    if !a.value.eq_value(&b.value) {
        return fail(format!("双路径值不一致：生产 {} vs 种子 {}", ra, rb));
    }
    pass(format!("双路径同值 ⇒ {}", ra))
}

fn stage_e_code(stage: Stage) -> u32 {
    match stage {
        Stage::Read => 1,
        Stage::Expand => 2,
        Stage::Compile => 3,
        Stage::Run => 4,
    }
}

// ---------------------------------------------------------------------------
// D 桶恢复探针：错误后同进程续跑正确程序（进程内 Effect 短路恢复面）
// ---------------------------------------------------------------------------

fn probe_read_recovery() -> CaseResult {
    let bad = run_source("(+ 1", FNAME);
    if bad.is_ok() {
        return fail("前置错误应 Err".to_string());
    }
    match run_source(FIB10, FNAME) {
        Ok(o) if matches!(o.value, kerf_vm::Value::Int(55)) => {
            pass("read 错误后续跑 fib(10)=55（同进程恢复——Effect 一次性逃逸层）".to_string())
        }
        other => fail(format!("恢复失败：{:?}", other.map(|o| o.value))),
    }
}

fn probe_expand_recovery() -> CaseResult {
    let bad = run_source(
        "(define-syntax m (syntax-rules (1) ((m x) x))) (m 1)",
        FNAME,
    );
    if bad.is_ok() {
        return fail("前置错误应 Err".to_string());
    }
    match run_source(SWAP, FNAME) {
        Ok(o) if matches!(o.value, kerf_vm::Value::Int(1)) => {
            pass("宏解析错误后续跑 swap 宏程序（同进程恢复）".to_string())
        }
        other => fail(format!("恢复失败：{:?}", other.map(|o| o.value))),
    }
}

fn probe_cycle_recovery() -> CaseResult {
    let bad = run_source("(module a (import b) 1) (module b (import a) 1)", FNAME);
    if bad.is_ok() {
        return fail("前置错误应 Err".to_string());
    }
    match run_source("(module m (export f) (define (f x) (* x 2))) 1", FNAME) {
        Ok(o) if matches!(o.value, kerf_vm::Value::Int(1)) => {
            pass("循环依赖错误后续跑模块程序（同进程恢复——DFS 灰标记无残留）".to_string())
        }
        other => fail(format!("恢复失败：{:?}", other.map(|o| o.value))),
    }
}

fn probe_depth_recovery() -> CaseResult {
    let bad = run_source(
        "(define-syntax loop2 (syntax-rules () ((loop2) (loop2)))) (loop2)",
        FNAME,
    );
    if bad.is_ok() {
        return fail("前置错误应 Err".to_string());
    }
    match run_source(
        "(define-syntax m (syntax-rules () ((m x) x))) (begin (m 1) (m 2) (m 3))",
        FNAME,
    ) {
        Ok(o) if matches!(o.value, kerf_vm::Value::Int(3)) => {
            pass("深度超限错误后续跑宏程序（同进程恢复——thread_local 状态零残留）".to_string())
        }
        other => fail(format!("恢复失败：{:?}", other.map(|o| o.value))),
    }
}

fn probe_run_recovery() -> CaseResult {
    let bad = run_source("(undefined-one)", FNAME);
    if bad.is_ok() {
        return fail("前置错误应 Err".to_string());
    }
    match run_source("(+ 20 22)", FNAME) {
        Ok(o) if matches!(o.value, kerf_vm::Value::Int(42)) => {
            pass("运行期未绑定后续跑算术（同进程恢复）".to_string())
        }
        other => fail(format!("恢复失败：{:?}", other.map(|o| o.value))),
    }
}

fn probe_arity_recovery() -> CaseResult {
    let bad = run_source("((lambda (x) x) 1 2)", FNAME);
    if bad.is_ok() {
        return fail("前置错误应 Err".to_string());
    }
    match run_source(COUNTERS, FNAME) {
        Ok(o) => {
            let got = kerf_vm::render_value(&o.value, &o.heap);
            if got == "(4 2)" {
                pass("arity 错误后续跑闭包计数器（a 第 4 次 / b 第 2 次）".to_string())
            } else {
                fail(format!("恢复值不匹配：(4 2) vs {}", got))
            }
        }
        Err(e) => fail(format!("恢复失败：{}", first_line(&e.rendered))),
    }
}

// ---------------------------------------------------------------------------
// E 桶边界探针（§7.3.2——本批次修复面的边界）
// ---------------------------------------------------------------------------

/// 宏产物 Span 并集代次守卫的边界：宏在糖产物内（use-site 代次 ≥1）——
/// 产物 Span 落模板范围（用户跨距按代次守卫丢弃）且**行为正确**（+ 值）。
fn probe_macro_in_sugar_span_boundary() -> CaseResult {
    let s = "(define-syntax swap!
  (syntax-rules ()
    ((swap! a b) (let ((tmp a)) (begin (set! a b) (set! b tmp))))))
(let ((tmp 10) (u 3) (v 4)) (begin (swap! u v) (+ u v tmp)))";
    match run_source(s, FNAME) {
        Ok(o) => {
            let got = kerf_vm::render_value(&o.value, &o.heap);
            // u=4, v=3, tmp=10 → 17
            if got == "17" {
                pass("糖内宏调用行为正确（u/v 交换 + 用户 tmp 词法可见=10）→ 17".to_string())
            } else {
                fail(format!("边界行为不匹配：17 vs {}", got))
            }
        }
        Err(e) => fail(format!("边界用例 Err：{}", first_line(&e.rendered))),
    }
}

// ---------------------------------------------------------------------------
// P 桶 §21.3 锚定探针
// ---------------------------------------------------------------------------

fn probe_macro_or_positive() -> CaseResult {
    let src = format!("{} (my-or false false true)", MY_OR);
    match run_source(&src, FNAME) {
        Ok(o) => {
            let got = kerf_vm::render_value(&o.value, &o.heap);
            if got == "true" {
                pass("省略号递归宏短路（末参 true）⇒ true".to_string())
            } else {
                fail(format!("期望 true，实际 {}", got))
            }
        }
        Err(e) => fail(format!("期望 Ok：{}", first_line(&e.rendered))),
    }
}

/// §21.3 条件 4：增量编译基础设施——同源重复编译产物一致（内容寻址
/// 缓存）+ 第二次 check 命中缓存（cache_hit 观测面）+ 静态检查 0 错。
fn probe_incremental_compile_determinism() -> CaseResult {
    let a = compile_source(FIB10, FNAME);
    let b = compile_source(FIB10, FNAME);
    let (a, b) = match (a, b) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => return fail(format!("编译失败：{}", first_line(&e.rendered))),
    };
    if a.render_core() != b.render_core() {
        return fail("重复编译核心表达式不一致（缓存确定性破坏）".to_string());
    }
    // 静态检查面（类型检查器消费同一前段）+ 缓存命中观测（第二次）
    let first = check_source(FIB10, FNAME);
    let second = check_source(FIB10, FNAME);
    let (first, second) = match (first, second) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => {
            return fail(format!("check 失败：{}", first_line(&e.rendered)))
        }
    };
    if !first.diagnostics.is_empty() {
        return fail(format!("静态检查报错 {} 项", first.diagnostics.len()));
    }
    if !second.cache_hit {
        return fail("第二次 check 未命中缓存（增量编译基础设施失效）".to_string());
    }
    pass("重复编译 core 一致 + 第二次 check 命中缓存 + 静态检查 0 错".to_string())
}

// ---------------------------------------------------------------------------
// main：配比机械校验 + 运行
// ---------------------------------------------------------------------------

fn main() {
    println!("== kerf Stage 1 门审计集 stage1_gate_audit_r1（sop.md §7.3.1 / §7.1.1 / §7.3.2 / §21.3）==");
    println!(
        "case 总数 {}（A 单语句 12 / B 多语句 12 / C 复杂 10 / D 恢复 6 / E 边界 6 / P 正向 4）\n",
        CASES.len()
    );

    let mut fail_count = 0;
    let mut class_seen = [0usize; 7];
    let mut bucket_counts = [0usize; 6];
    let mut negative = 0;
    let mut positive = 0;
    let mut recovery = 0;

    for c in CASES {
        bucket_counts[c.bucket as usize] += 1;
        match c.polarity {
            Polarity::Negative => negative += 1,
            Polarity::Positive => positive += 1,
            Polarity::Recovery => recovery += 1,
        }
        if let Some(cls) = c.class {
            class_seen[cls.index() - 1] += 1;
        }
        let r = run_case(c);
        let status = if r.pass { "PASS" } else { "FAIL" };
        let class_label = c.class.map(|k| k.label()).unwrap_or("-");
        println!(
            "[{}] {} {} {} {}",
            status,
            c.id,
            c.bucket.label(),
            match c.polarity {
                Polarity::Negative => "负向",
                Polarity::Positive => "正向",
                Polarity::Recovery => "恢复",
            },
            class_label
        );
        println!("      src: {}", short_src(c.src));
        println!("      → {}", r.detail);
        if !r.pass {
            fail_count += 1;
        }
    }

    // 汇总 + §7.3.1/§7.3.2 配比机械校验
    let total = CASES.len();
    let passed = total - fail_count;
    println!("\n== 汇总 ==");
    println!("total {} / PASS {} / FAIL {}", total, passed, fail_count);
    println!(
        "桶配比（§7.3.1）：单语句 {}/10 多语句 {}/10 复杂 {}/5 恢复 {}/5；边界 {}/5（§7.3.2）；正向 {}/8（上限）",
        bucket_counts[0], bucket_counts[1], bucket_counts[2], bucket_counts[3], bucket_counts[4], bucket_counts[5]
    );
    println!(
        "极性：负向 {}（≥22）/ 恢复 {} / 正向 {}",
        negative, recovery, positive
    );
    print!("§7.1.1 七类覆盖：");
    for (i, n) in class_seen.iter().enumerate() {
        print!(" {}={}", i + 1, n);
    }
    println!();

    let mut ratio_fail = false;
    if bucket_counts[0] < 10 {
        println!("配比违规：A 桶 {} < 10", bucket_counts[0]);
        ratio_fail = true;
    }
    if bucket_counts[1] < 10 {
        println!("配比违规：B 桶 {} < 10", bucket_counts[1]);
        ratio_fail = true;
    }
    if bucket_counts[2] < 5 {
        println!("配比违规：C 桶 {} < 5", bucket_counts[2]);
        ratio_fail = true;
    }
    if bucket_counts[3] < 5 {
        println!("配比违规：D 桶 {} < 5", bucket_counts[3]);
        ratio_fail = true;
    }
    if bucket_counts[4] < 5 {
        println!("配比违规（§7.3.2）：E 边界桶 {} < 5", bucket_counts[4]);
        ratio_fail = true;
    }
    if negative < 22 {
        println!("配比违规：负向 {} < 22", negative);
        ratio_fail = true;
    }
    for (i, n) in class_seen.iter().enumerate() {
        if *n == 0 {
            println!("配比违规：§7.1.1 第 {} 类零覆盖", i + 1);
            ratio_fail = true;
        }
    }

    if fail_count > 0 || ratio_fail {
        println!(
            "\n结论：NEEDS REVISION（FAIL {} / 配比违规 {}）",
            fail_count, ratio_fail
        );
        std::process::exit(1);
    }
    println!("\n结论：APPROVED（审计通过——七类全覆盖 + 配比满足 + 边界 ≥5 + 零发现）");
    println!();
    println!("== §21.3 四条件锚定 ==");
    println!("条件 1（子集表达前端）：生产管线自举（VM Reader + VM Expander 含宏）——本审计全体 case 经生产路径");
    println!("条件 2（Stage 0 VM 正确运行）：自举 Expander 于 VM 上承载全部生产展开（含宏/模块/prelude）");
    println!("条件 3（标准库最小集）：P02 prelude 管道（列表 hofs 用户面）+ stdlib_tests（列表/字符串/I/O）");
    println!(
        "条件 4（增量编译基础设施）：P04 缓存确定性 + 静态检查 0 错（cache_tests 内容寻址/失效面）"
    );
}
