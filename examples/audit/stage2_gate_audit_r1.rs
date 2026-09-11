//! Stage 2 批次 I 负向门审计集（sop.md §7.3.1 规则 3 / §7.1.1 七类矩阵
//! / §7.3.2 上轮修复边界 case / §21.3 阶段验收四条件——Stage 2 口径）。
//!
//! **用途**：≥30 case 负向审计——I3 门审查（plan.md §5a 42-g）强制审计
//! 集（第 1 轮，r26）。审计全部经**生产管线**（I1 收口后：读 + 展 + 编
//! 三段自举——VM 上 reader.krf + expander.krf + compiler.krf；Rust 种
//! 种子链 = parity oracle）——审计本身即 §21.3 条件 1/2 的持续验证载体。
//!
//! **运行方式**：工作区根目录 `cargo run --example stage2_gate_audit_r1`。
//! 全部 case PASS 且 §7.3.1/§7.3.2 配比满足时退出码 0；任一 FAIL 或配比
//! 违规时退出码 1（NEEDS REVISION）。
//!
//! **结构**（§7.3.1 规则 2 强制配比 + 本轮批次面扩展，配比在 main() 中
//! 机械校验）：
//! - A 桶 12：单语句负向（基础类型系统——含效应类型面 E0007/E0004 族）
//! - B 桶 12：多语句/多函数负向（集成正确性——糖九件/module/require/
//!   宏/能力门控面）
//! - C 桶 10：复杂程序负向（嵌套控制流/闭包/递归/效应 + ⑥ 循环依赖
//!   + ⑦ 深度超限 + E0008/E0009 深链变体）
//! - D 桶 6：错误恢复（错误后同进程续跑正确程序——含效应错误后恢复）
//! - E 桶 7：上轮修复边界 case（§7.3.2——r23-r25 修复面：门 B fixpoint
//!   活性 / 缓存 CompilerKind 分桶 / E0008 首次恢复位置 / TD-014 嵌套
//!   define 归因 / TD-011 字符串全序 / TD-018 消息单源 / recover.rs 多
//!   错误收集（46-z 死代码清偿边界））
//! - P 桶 6：正向 sanity + §21.3 四条件锚定（审计器自证——Stage 2 批次
//!   I 口径如实登记）
//!
//! 合计 53 case：负向 34 + 恢复 6 + 边界 7 + 正向 6。
//!
//! **§7.1.1 七类覆盖**：①语法 ②未绑定 ③空应用 ④参数个数 ⑤类型不匹配
//! ⑥模块循环依赖（双模块源码 → registry DFS 灰标记）⑦宏展开深度超限
//! （自指宏经生产路径）。
//!
//! **§21.3 四条件锚定**（P 桶 + 全体审计面——Stage 2 批次 I 范围）：
//! - 条件 1（完整 kerf 编译器用 kerf 编写）：生产管线自身（三段自举
//!   Reader+Expander+Compiler）——全体 case 经生产路径即表达力证明；
//! - 条件 2（两次编译自身结果一致）：P03 门 B 轻量终验（B₁/B₂
//!   bytecode_equal + SHA-256，compiler.krf + preamble.krf 两件）；
//! - 条件 3（至少一个非 VM 后端工作）：P04 QBE 本地码 fib 端到端
//!   （r17 PoC——tools/qbe 仓库内工具链 + cc 宿主编排，不入自举链）；
//! - 条件 4（FFI 可用）：P05 ffi-ownership-model.md 冻结面断言（r17
//!   模型冻结——实现属 Stage 2 后续批次，批次 I 口径如实登记）。
//!
//! 遵循条款：§7.3.1/§7.3.2（配比与边界）、§2.3-11（实测禁臆测——全部
//! 断言经生产管线真实运行）、§21.2（LLVM/QBE 不入自举链口径）、GATE 1
//! （EXIT 0 = 实测通过，非"看起来对"）。

use kerf_driver::bootstrap::{install_state as install_reader, reset_state as reset_reader};
use kerf_driver::bootstrap_compiler::{
    install_state as install_compiler, reset_state as reset_compiler,
};
use kerf_driver::bootstrap_expander::{
    install_state as install_expander, reset_state as reset_expander,
};
use kerf_driver::{
    cache_entry_count, check_source, check_source_recover, compile_source, compile_source_seed,
    run_source, run_source_seed, set_cache_enabled, sha256_hex, Stage,
};
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
    /// 双路径均 Err：阶段 + 消息子串（+ 显式 E 码 + 非空 Span 机械校验）。
    /// `code = None` 时按阶段默认码（Read→E0001 / Expand→E0002 /
    /// Compile→E0003 / Run→E0004）——效应专用码 E0007/E0008/E0009 显式
    /// 传入（r25 D9 结构化族）。
    Err {
        stage: Stage,
        msg: &'static str,
        code: Option<u32>,
    },
    /// 双路径 Ok 且值一致（渲染值精确匹配）。
    OkDual(&'static str),
    /// 自定义探针（错误恢复 / 边界 / §21.3 锚定）。
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
// 语料常量（Stage 2 批次 I 面：糖/宏/module/效应/stdlib）
// ---------------------------------------------------------------------------

const FIB10: &str = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 10)";

const FIB12: &str = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 12)";

const SWAP: &str = r#"(define-syntax swap!
  (syntax-rules ()
    ((swap! a b)
     (let ((tmp a))
       (begin (set! a b) (set! b tmp))))))
(define x 1) (define y 2)
(begin (swap! x y) (- x y))"#;

const MY_WHEN: &str = r#"(define-syntax my-when
  (syntax-rules ()
    ((my-when c e) (if c e nil))))
(define flag true)
(my-when flag 42)"#;

const PRELUDE_PIPE: &str = r#"(module user (import kerf-prelude)
  (define lst (list 1 2 3 4 5))
  (foldl + 0 (map (lambda (x) (* x x)) (filter (lambda (x) (> x 2)) lst))))"#;

/// 效应金路径（r25 M1 原型——perform 挂起 → dispatch → resume 注入）。
const EFFECT_GOLDEN: &str =
    "(handle add ((p k) (resume k (+ p 10))) (+ 1 (perform (cons 'add 5))))";

/// E0008 语料（continuation 逃逸 + 二次恢复——r25 D3 线性唯一性）。
const DOUBLE_RESUME: &str = "(define k2 nil) (handle t ((p k) (begin (set! k2 k) (resume k 1))) (perform (cons 't 5))) (k2 2)";

/// TCO + 效应组合（D8——10 万深尾递归穿透 handler 帧后 perform 恢复）。
const TCO_EFFECT: &str = "(define (spin n) (if (= n 0) (perform (cons 's 99)) (spin (- n 1)))) (handle s ((p k) (resume k (+ p 1))) (spin 100000))";

// ---------------------------------------------------------------------------
// 案例表（53 case）
// ---------------------------------------------------------------------------

const CASES: &[Case] = &[
    // ---- A 桶：单语句负向（基础类型系统，12）----
    Case {
        id: "A01",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Syntax),
        src: "(define x (+ 1 2)",
        expect: Expect::Err { stage: Stage::Read, msg: "括号未闭合", code: None },
    },
    Case {
        id: "A02",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::EmptyForm),
        src: "()",
        expect: Expect::Err { stage: Stage::Expand, msg: "空列表不能作为表达式求值", code: None },
    },
    Case {
        id: "A03",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        src: "kerf-undefined-var",
        expect: Expect::Err { stage: Stage::Run, msg: "未绑定", code: None },
    },
    Case {
        id: "A04",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(- \"a\" 1)",
        expect: Expect::Err { stage: Stage::Run, msg: "需要 int", code: None },
    },
    Case {
        id: "A05",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(car 5)",
        expect: Expect::Err { stage: Stage::Run, msg: "car 需要 pair", code: None },
    },
    Case {
        id: "A06",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Arity),
        src: "((lambda (x) x) 1 2)",
        expect: Expect::Err { stage: Stage::Run, msg: "参数数量不匹配", code: None },
    },
    Case {
        id: "A07",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(perform 5)",
        expect: Expect::Err { stage: Stage::Run, msg: "点对", code: None },
    },
    Case {
        id: "A08",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(perform (cons 7 5))",
        expect: Expect::Err { stage: Stage::Run, msg: "符号", code: None },
    },
    Case {
        id: "A09",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(perform (cons 'oops 1))",
        expect: Expect::Err { stage: Stage::Run, msg: "未被任何 handler 处理", code: Some(7) },
    },
    Case {
        id: "A10",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: None,
        src: "(handle t (p 1) 2)",
        expect: Expect::Err { stage: Stage::Expand, msg: "处理子句", code: None },
    },
    Case {
        id: "A11",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(resume 5 42)",
        expect: Expect::Err { stage: Stage::Run, msg: "不可调用的值", code: None },
    },
    Case {
        id: "A12",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(str-append \"a\" 5)",
        expect: Expect::Err { stage: Stage::Run, msg: "str-append 需要 2 个字符串", code: None },
    },
    // ---- B 桶：多语句/多函数负向（糖/module/require/宏/门控集成面，12）----
    Case {
        id: "B01",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: None,
        src: "(define x 1) (let (x) 1)",
        expect: Expect::Err { stage: Stage::Expand, msg: "let 绑定必须是", code: None },
    },
    Case {
        id: "B02",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: None,
        src: "(define x 1) (cond 1 2)",
        expect: Expect::Err { stage: Stage::Expand, msg: "cond 子句必须是非空列表", code: None },
    },
    Case {
        id: "B03",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: None,
        src: "(define (f) (define a 1) (define a 2) a) (f)",
        expect: Expect::Err { stage: Stage::Expand, msg: "嵌套 define 重复绑定", code: None },
    },
    Case {
        id: "B04",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: None,
        src: "(define (f) 1 (define a 2)) (f)",
        expect: Expect::Err { stage: Stage::Expand, msg: "define 必须位于函数体头部", code: None },
    },
    Case {
        id: "B05",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: None,
        src: "(module)",
        expect: Expect::Err { stage: Stage::Expand, msg: "module 形式", code: None },
    },
    Case {
        id: "B06",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: None,
        src: "(module 5 1)",
        expect: Expect::Err { stage: Stage::Expand, msg: "module 名必须是符号", code: None },
    },
    Case {
        id: "B07",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: None,
        src: "(define x 1) (import kerf-prelude)",
        expect: Expect::Err { stage: Stage::Expand, msg: "import/export 只能出现在 module 形式内部", code: None },
    },
    Case {
        id: "B08",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        src: "(set! never-defined 1)",
        expect: Expect::Err { stage: Stage::Run, msg: "未绑定", code: None },
    },
    Case {
        id: "B09",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: None,
        src: "(define-syntax m 5) (m 1)",
        expect: Expect::Err { stage: Stage::Expand, msg: "define-syntax 第二参数必须是", code: None },
    },
    Case {
        id: "B10",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Arity),
        src: "(define (add a b) (+ a b)) (add 1)",
        expect: Expect::Err { stage: Stage::Run, msg: "参数数量不匹配", code: None },
    },
    Case {
        id: "B11",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        // 能力门控 fail-closed（r25 能力 M2 边界前置面）：未声明
        // (require io read) 时 read-line 为**编译期静态拒绝**（E0006
        // ——比运行期未绑定更强的门控：无令牌的 I/O 无可达入口）
        src: "(read-line)",
        expect: Expect::Err { stage: Stage::Compile, msg: "能力权限不足", code: Some(6) },
    },
    Case {
        id: "B12",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        // tag 不匹配 → R11 dispatch 上抛链逃逸到顶层（E0007 集成面）
        src: "(define (g) (perform (cons 'x 5))) (handle y ((p k) (resume k p)) (g))",
        expect: Expect::Err { stage: Stage::Run, msg: "未被任何 handler 处理", code: Some(7) },
    },
    // ---- C 桶：复杂程序负向（嵌套/闭包/递归/效应深链，10）----
    Case {
        id: "C01",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::CircularDep),
        src: "(module a (import b) 1) (module b (import a) 1)",
        expect: Expect::Err { stage: Stage::Expand, msg: "模块循环依赖", code: None },
    },
    Case {
        id: "C02",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::MacroDepth),
        src: "(define-syntax loop-m (syntax-rules () ((loop-m x) (loop-m x)))) (loop-m 1)",
        expect: Expect::Err { stage: Stage::Expand, msg: "宏展开深度超过上限", code: None },
    },
    Case {
        id: "C03",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: None,
        src: DOUBLE_RESUME,
        expect: Expect::Err { stage: Stage::Run, msg: "二次恢复", code: Some(8) },
    },
    Case {
        id: "C04",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Arity),
        src: "(handle t ((p k) (k)) (perform (cons 't 1)))",
        expect: Expect::Err { stage: Stage::Run, msg: "恰接受一个值", code: Some(9) },
    },
    Case {
        id: "C05",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        // 深递归后 perform + tag 不匹配 handler（吞掉而非处理）→ 逃逸
        src: "(define (deep n) (if (= n 0) (perform (cons 'deep 0)) (deep (- n 1)))) (handle shallow ((p k) 99) (deep 100))",
        expect: Expect::Err { stage: Stage::Run, msg: "未被任何 handler 处理", code: Some(7) },
    },
    Case {
        id: "C06",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(define (f n) (if (= n 0) (car n) (f (- n 1)))) (f 100)",
        expect: Expect::Err { stage: Stage::Run, msg: "car 需要 pair", code: None },
    },
    Case {
        id: "C07",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: None,
        // continuation 逃逸到全局 + 闭包外二次恢复（E0008 深链变体）
        src: "(define saved nil) (define (gen) (handle t ((p k) (begin (set! saved k) (resume k p))) (perform (cons 't 1)))) (gen) (saved 2)",
        expect: Expect::Err { stage: Stage::Run, msg: "二次恢复", code: Some(8) },
    },
    Case {
        id: "C08",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        // 宏（糖展开）+ set! 面 + 未绑定（集成层归因）
        src: "(define-syntax swap! (syntax-rules () ((swap! a b) (let ((tmp a)) (begin (set! a b) (set! b tmp)))))) (swap! x y)",
        expect: Expect::Err { stage: Stage::Run, msg: "未绑定", code: None },
    },
    Case {
        id: "C09",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        // 分配压力（万级 cons）后错误路径仍正确归因（GC 压力 × 类型错）
        src: "(define (mk n) (if (= n 0) (car 5) (cons n (mk (- n 1))))) (mk 20000)",
        expect: Expect::Err { stage: Stage::Run, msg: "car 需要 pair", code: None },
    },
    Case {
        id: "C10",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        // module 体 inline 执行（42-c 语义）中的运行期类型错
        src: "(module m (define (f x) (car x)) (f 5))",
        expect: Expect::Err { stage: Stage::Run, msg: "car 需要 pair", code: None },
    },
    // ---- D 桶：错误恢复（6——Custom 探针）----
    Case {
        id: "D01",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: None,
        src: "(read 错误后 fib(10))",
        expect: Expect::Custom(probe_read_recovery),
    },
    Case {
        id: "D02",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: None,
        src: "(E0007 后效应金路径)",
        expect: Expect::Custom(probe_e0007_recovery),
    },
    Case {
        id: "D03",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: None,
        src: "(E0008 后效应金路径)",
        expect: Expect::Custom(probe_e0008_recovery),
    },
    Case {
        id: "D04",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: None,
        src: "(糖形态错后 swap 宏)",
        expect: Expect::Custom(probe_sugar_recovery),
    },
    Case {
        id: "D05",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: None,
        src: "(module 形态错后 prelude 管道)",
        expect: Expect::Custom(probe_module_recovery),
    },
    Case {
        id: "D06",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: Some(ErrorClass::MacroDepth),
        src: "(宏深度超限后 my-or 宏程序)",
        expect: Expect::Custom(probe_macro_depth_recovery),
    },
    // ---- E 桶：上轮修复边界 case（§7.3.2——r23-r25 修复面，7）----
    Case {
        id: "E01",
        bucket: Bucket::Boundary,
        polarity: Polarity::Positive,
        class: None,
        src: "(门 B fixpoint 活性——B₁ 产物作为编译段运行)",
        expect: Expect::Custom(probe_fixpoint_liveness),
    },
    Case {
        id: "E02",
        bucket: Bucket::Boundary,
        polarity: Polarity::Positive,
        class: None,
        src: "(缓存 CompilerKind 分桶——生产命中/种子不查不存)",
        expect: Expect::Custom(probe_cache_kind_bucketing),
    },
    Case {
        id: "E03",
        bucket: Bucket::Boundary,
        polarity: Polarity::Negative,
        class: None,
        src: DOUBLE_RESUME,
        expect: Expect::Custom(probe_e0008_first_resume_position),
    },
    Case {
        id: "E04",
        bucket: Bucket::Boundary,
        polarity: Polarity::Negative,
        class: None,
        src: "(TD-014 专门归因——不含失真兜底消息)",
        expect: Expect::Custom(probe_nested_define_attribution),
    },
    Case {
        id: "E05",
        bucket: Bucket::Boundary,
        polarity: Polarity::Positive,
        class: None,
        src: "(TD-011 字符串全序正/混型负双面)",
        expect: Expect::Custom(probe_string_total_order),
    },
    Case {
        id: "E06",
        bucket: Bucket::Boundary,
        polarity: Polarity::Negative,
        class: None,
        src: "(TD-018 消息单源——if 与 cond 脱糖同文)",
        expect: Expect::Custom(probe_message_single_source),
    },
    Case {
        id: "E07",
        bucket: Bucket::Boundary,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::EmptyForm),
        src: "(recover.rs 多错误收集——46-z 清偿边界)",
        expect: Expect::Custom(probe_recover_multi_error),
    },
    // ---- P 桶：正向 sanity + §21.3 四条件锚定（6）----
    Case {
        id: "P01",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        src: FIB12,
        expect: Expect::OkDual("144"),
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
        src: "(门 B 轻量终验——两次编译自身字节一致)",
        expect: Expect::Custom(probe_gate_b_fixpoint),
    },
    Case {
        id: "P04",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        src: FIB12,
        expect: Expect::Custom(probe_qbe_native_fib),
    },
    Case {
        id: "P05",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        src: "(FFI 模型冻结面断言)",
        expect: Expect::Custom(probe_ffi_model_frozen),
    },
    Case {
        id: "P06",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        src: TCO_EFFECT,
        expect: Expect::OkDual("100"),
    },
];

// ---------------------------------------------------------------------------
// 运行器
// ---------------------------------------------------------------------------

fn run_case(c: &Case) -> CaseResult {
    match &c.expect {
        Expect::Err { stage, msg, code } => run_negative(c, *stage, msg, *code),
        Expect::OkDual(s) => run_positive_dual(c, s),
        Expect::Custom(f) => f(),
    }
}

fn run_negative(c: &Case, stage: Stage, msg: &str, code: Option<u32>) -> CaseResult {
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
    let want_code = DiagnosticCode(code.unwrap_or_else(|| stage_e_code(stage)));
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
// D 桶恢复探针：错误后同进程续跑正确程序（进程内一次性逃逸层恢复面）
// ---------------------------------------------------------------------------

fn probe_read_recovery() -> CaseResult {
    let bad = run_source("(define x (+ 1 2)", FNAME);
    if bad.is_ok() {
        return fail("前置错误应 Err".to_string());
    }
    match run_source(FIB10, FNAME) {
        Ok(o) if matches!(o.value, kerf_vm::Value::Int(55)) => {
            pass("read 错误后续跑 fib(10)=55（同进程恢复）".to_string())
        }
        other => fail(format!("恢复失败：{:?}", other.map(|o| o.value))),
    }
}

fn probe_e0007_recovery() -> CaseResult {
    let bad = run_source("(perform (cons 'oops 1))", FNAME);
    if bad.is_ok() {
        return fail("前置 E0007 应 Err".to_string());
    }
    match run_source(EFFECT_GOLDEN, FNAME) {
        Ok(o) if matches!(o.value, kerf_vm::Value::Int(16)) => {
            pass("E0007 错误后续跑效应金路径 ⇒ 16（VM 效应状态无泄漏）".to_string())
        }
        other => fail(format!("恢复失败：{:?}", other.map(|o| o.value))),
    }
}

fn probe_e0008_recovery() -> CaseResult {
    let bad = run_source(DOUBLE_RESUME, FNAME);
    if bad.is_ok() {
        return fail("前置 E0008 应 Err".to_string());
    }
    match run_source(EFFECT_GOLDEN, FNAME) {
        Ok(o) if matches!(o.value, kerf_vm::Value::Int(16)) => {
            pass("E0008（continuation 已耗尽）后效应金路径 ⇒ 16（帧池无残留）".to_string())
        }
        other => fail(format!("恢复失败：{:?}", other.map(|o| o.value))),
    }
}

fn probe_sugar_recovery() -> CaseResult {
    let bad = run_source("(define x 1) (cond 1 2)", FNAME);
    if bad.is_ok() {
        return fail("前置糖形态错应 Err".to_string());
    }
    match run_source(SWAP, FNAME) {
        Ok(o) if matches!(o.value, kerf_vm::Value::Int(1)) => {
            pass("cond 形态错后续跑 swap 宏程序 ⇒ 1（同进程恢复）".to_string())
        }
        other => fail(format!("恢复失败：{:?}", other.map(|o| o.value))),
    }
}

fn probe_module_recovery() -> CaseResult {
    let bad = run_source("(module)", FNAME);
    if bad.is_ok() {
        return fail("前置 module 形态错应 Err".to_string());
    }
    match run_source(PRELUDE_PIPE, FNAME) {
        Ok(o) if matches!(o.value, kerf_vm::Value::Int(50)) => {
            pass("module 形态错后续跑 prelude 管道 ⇒ 50（registry 无污染）".to_string())
        }
        other => fail(format!("恢复失败：{:?}", other.map(|o| o.value))),
    }
}

fn probe_macro_depth_recovery() -> CaseResult {
    let bad = run_source(
        "(define-syntax loop-m (syntax-rules () ((loop-m x) (loop-m x)))) (loop-m 1)",
        FNAME,
    );
    if bad.is_ok() {
        return fail("前置宏深度超限应 Err".to_string());
    }
    match run_source(MY_WHEN, FNAME) {
        Ok(o) if matches!(o.value, kerf_vm::Value::Int(42)) => {
            pass("宏深度超限后续跑 my-when 宏程序 ⇒ 42（卫生计数器无泄漏）".to_string())
        }
        other => fail(format!("恢复失败：{:?}", other.map(|o| o.value))),
    }
}

// ---------------------------------------------------------------------------
// E 桶边界探针（§7.3.2——上轮修复边界 case）
// ---------------------------------------------------------------------------

/// E01（42-d 门 B 活性边界）：B₁ 产物 install 后作为编译段实际运转
/// ——编译 + 运行 sq(7) 与种子链一致；结尾 reset 恢复 fresh 状态。
fn probe_fixpoint_liveness() -> CaseResult {
    set_cache_enabled(false);
    reset_compiler();
    let b1 = match compile_source(
        include_str!("../../crates/kerf-driver/src/bootstrap/compiler.krf"),
        "compiler.krf",
    ) {
        Ok(o) => o,
        Err(e) => {
            reset_compiler();
            set_cache_enabled(true);
            return fail(format!(
                "B₁ compiler.krf 编译失败：{}",
                first_line(&e.rendered)
            ));
        }
    };
    if let Err(e) = install_compiler(b1.program.clone(), b1.table.clone(), b1.source_map.clone()) {
        reset_compiler();
        set_cache_enabled(true);
        return fail(format!("B₁ 状态安装失败：{:?}", e));
    }
    let src = "(define (sq x) (* x x)) (sq 7)";
    let prod = run_source(src, "e01.krf");
    reset_compiler();
    set_cache_enabled(true);
    let seed = run_source_seed(src, "e01.krf");
    match (prod, seed) {
        (Ok(a), Ok(b))
            if a.value.eq_value(&b.value) && matches!(a.value, kerf_vm::Value::Int(49)) =>
        {
            pass("B₁ 产物作为编译段运行 sq(7)=49（与种子一致——活性终验）".to_string())
        }
        (a, b) => fail(format!(
            "B₁ 活性失败：生产 {:?} / 种子 {:?}",
            a.map(|o| o.value),
            b.map(|o| o.value)
        )),
    }
}

/// E02（42-d B11 缓存分桶边界）：生产链同源两次编译——第二次经缓存
/// 产物与首次 bytecode_equal；种子链恒不查不存（条目数不变 + 产物
/// 一致）。
fn probe_cache_kind_bucketing() -> CaseResult {
    // 生产路径：首次直编译 → 第二次命中（产物一致）
    let a = compile_source(SWAP, "e02.krf");
    let before = cache_entry_count();
    let b = compile_source(SWAP, "e02.krf");
    let (a, b) = match (a, b) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => {
            return fail(format!("生产编译失败：{}", first_line(&e.rendered)))
        }
    };
    if !a.program.bytecode_equal(&b.program) {
        return fail("缓存命中产物与首次不一致（分桶/确定性破坏）".to_string());
    }
    if cache_entry_count() < before {
        return fail("生产缓存条目数下降（异常）".to_string());
    }
    // check 面：第二次命中（cache_hit 观测）
    let first = check_source(SWAP, "e02.krf");
    let second = check_source(SWAP, "e02.krf");
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
        return fail("第二次 check 未命中缓存（B11 观测面失效）".to_string());
    }
    // 种子路径：恒不查不存——条目数不变 + 两次产物一致
    let n0 = cache_entry_count();
    let s1 = compile_source_seed(SWAP, "e02.krf");
    let s2 = compile_source_seed(SWAP, "e02.krf");
    let (s1, s2) = match (s1, s2) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => {
            return fail(format!("种子编译失败：{}", first_line(&e.rendered)))
        }
    };
    let n1 = cache_entry_count();
    if n1 != n0 {
        return fail(format!(
            "种子路径写入缓存（B11 违规）：条目 {} → {}",
            n0, n1
        ));
    }
    if !s1.program.bytecode_equal(&s2.program) {
        return fail("种子两次编译产物不一致（直编译确定性破坏）".to_string());
    }
    pass("生产命中（产物一致 + cache_hit）/ 种子不查不存（条目数不变）".to_string())
}

/// E03（r25 D9 边界）：E0008 消息含首次恢复位置追踪文本。
fn probe_e0008_first_resume_position() -> CaseResult {
    let err = match run_source(DOUBLE_RESUME, FNAME) {
        Err(e) => e,
        Ok(_) => return fail("期望 E0008 Err".to_string()),
    };
    if !err.rendered.contains("error[E0008]") {
        return fail(format!("缺 E0008 码：{}", first_line(&err.rendered)));
    }
    if !err.rendered.contains("首次恢复于") {
        return fail(format!(
            "E0008 缺首次恢复位置追踪（D9）：{}",
            first_line(&err.rendered)
        ));
    }
    pass("E0008 含「首次恢复于」位置追踪（D9 边界）".to_string())
}

/// E04（r24 TD-014 边界）：嵌套 define 重名走专门归因——消息为
/// 「嵌套 define 重复绑定」且**不含**失真兜底「lambda 参数重名」。
fn probe_nested_define_attribution() -> CaseResult {
    let src = "(define (f) (define a 1) (define a 2) a) (f)";
    let err = match run_source(src, FNAME) {
        Err(e) => e,
        Ok(_) => return fail("嵌套 define 重名应 Err".to_string()),
    };
    if !err.rendered.contains("嵌套 define 重复绑定") {
        return fail(format!("缺专门归因消息：{}", first_line(&err.rendered)));
    }
    if err.rendered.contains("lambda 参数重名") {
        return fail("归因回退到失真兜底消息（TD-014 回归）".to_string());
    }
    pass("嵌套 define 重名 → 专门归因（无失真兜底回退）".to_string())
}

/// E05（r24 TD-011 边界）：字符串全序正例（`(< "abc" "abd")` ⇒ true）
/// + 混型负例（`(< "abc" 1)` → 「< 需要数值」）双面。
fn probe_string_total_order() -> CaseResult {
    let ok = run_source("(< \"abc\" \"abd\")", FNAME);
    match ok {
        Ok(o) if matches!(o.value, kerf_vm::Value::Bool(true)) => {}
        other => return fail(format!("字符串全序正例失败：{:?}", other.map(|o| o.value))),
    }
    let bad = run_source("(< \"abc\" 1)", FNAME);
    match bad {
        Err(e) if e.rendered.contains("< 需要数值") => {}
        Err(e) => {
            return fail(format!(
                "混型比较消息不含「< 需要数值」：{}",
                first_line(&e.rendered)
            ))
        }
        Ok(o) => return fail(format!("混型比较未报错（返回 {:?}）", o.value)),
    }
    pass("全序正例 true + 混型负例「< 需要数值」（TD-011 双面）".to_string())
}

/// E06（r24 TD-018 边界）：消息单源——if 条件错与 cond 脱糖后 if
/// 条件错**同一消息文本**（单源证明）。
fn probe_message_single_source() -> CaseResult {
    let direct = match run_source("(if \"s\" 1 2)", FNAME) {
        Err(e) => e,
        Ok(_) => return fail("if 条件类型错应 Err".to_string()),
    };
    let via_cond = match run_source("(cond (\"s\" 1) (else 2))", FNAME) {
        Err(e) => e,
        Ok(_) => return fail("cond 条件类型错应 Err".to_string()),
    };
    let needle = "if 条件需要 bool";
    if !direct.rendered.contains(needle) {
        return fail(format!(
            "if 直用消息缺「{}」：{}",
            needle,
            first_line(&direct.rendered)
        ));
    }
    if !via_cond.rendered.contains(needle) {
        return fail(format!(
            "cond 脱糖路径消息缺「{}」（单源破坏）：{}",
            needle,
            first_line(&via_cond.rendered)
        ));
    }
    pass("if 直用与 cond 脱糖同文「if 条件需要 bool」（TD-018 单源）".to_string())
}

/// E07（46-z 边界——recover.rs 死代码清偿后的回归面）：两个展开级
/// 错误（空形式）→ check_source_recover 收集 2 诊断（多错误恢复
/// 活性——atom_form 移除无回归）。
fn probe_recover_multi_error() -> CaseResult {
    let report = match check_source_recover("()\n()\n", FNAME) {
        Ok(r) => r,
        Err(e) => return fail(format!("recover check 失败：{}", first_line(&e.rendered))),
    };
    if report.diagnostics.len() != 2 {
        return fail(format!(
            "应收集 2 条诊断，实际 {}（恢复路径回归）",
            report.diagnostics.len()
        ));
    }
    pass("双空形式 → 2 诊断（recover 多错误收集活性）".to_string())
}

// ---------------------------------------------------------------------------
// P 桶 §21.3 四条件锚定探针
// ---------------------------------------------------------------------------

fn program_digest(program: &kerf_compiler::BcProgram) -> String {
    sha256_hex(format!("{:?}", program).as_bytes())
}

/// P03（§21.3 条件 2——门 B 轻量终验）：B₁（生产链编译自举件）vs
/// B₂（以 B₁ 产物为新自举状态再编译同源）——compiler.krf +
/// preamble.krf 两件 bytecode_equal + SHA-256 摘要一致。
/// 隔离纪律（§7.4）：关缓存 + fresh 状态起步；结尾全量复位。
fn probe_gate_b_fixpoint() -> CaseResult {
    let sources: [(&str, &str); 2] = [
        (
            include_str!("../../crates/kerf-driver/src/bootstrap/compiler.krf"),
            "compiler.krf",
        ),
        (
            include_str!("../../crates/kerf-driver/src/bootstrap/preamble.krf"),
            "preamble.krf",
        ),
    ];
    set_cache_enabled(false);
    reset_reader();
    reset_expander();
    reset_compiler();
    // B₁：生产链（三段自举）编译（for 循环收集——避免闭包 Result 大 Err 链）
    let mut b1 = Vec::new();
    for (s, n) in sources {
        match compile_source(s, n) {
            Ok(o) => b1.push(o),
            Err(e) => {
                restore_fresh();
                return fail(format!("B₁ {} 编译失败：{}", n, first_line(&e.rendered)));
            }
        }
    }
    // B₂：以 B₁ 产物为新自举状态（三段接管），再编译同源。
    // 三段安装需 reader/expander 的 B₁ 产物——轻量版重编两件
    let front_sources = [
        (
            include_str!("../../crates/kerf-driver/src/bootstrap/reader.krf"),
            "reader.krf",
        ),
        (
            include_str!("../../crates/kerf-driver/src/bootstrap/expander.krf"),
            "expander.krf",
        ),
    ];
    let mut b1_front = Vec::new();
    for (s, n) in front_sources {
        match compile_source(s, n) {
            Ok(o) => b1_front.push(o),
            Err(e) => {
                restore_fresh();
                return fail(format!("B₁ {} 编译失败：{}", n, first_line(&e.rendered)));
            }
        }
    }
    let r_install = install_reader(
        b1_front[0].program.clone(),
        b1_front[0].table.clone(),
        b1_front[0].source_map.clone(),
    );
    let e_install = install_expander(
        b1_front[1].program.clone(),
        b1_front[1].table.clone(),
        b1_front[1].source_map.clone(),
    );
    if r_install.is_err() || e_install.is_err() {
        restore_fresh();
        return fail("B₁ reader/expander 状态安装失败".to_string());
    }
    let c_install = install_compiler(
        b1[0].program.clone(),
        b1[0].table.clone(),
        b1[0].source_map.clone(),
    );
    if c_install.is_err() {
        restore_fresh();
        return fail("B₁ compiler 状态安装失败".to_string());
    }
    let mut b2 = Vec::new();
    for (s, n) in sources {
        match compile_source(s, n) {
            Ok(o) => b2.push(o),
            Err(e) => {
                restore_fresh();
                return fail(format!("B₂ {} 编译失败：{}", n, first_line(&e.rendered)));
            }
        }
    }
    // 判据：逐件 bytecode_equal + SHA-256 一致
    let mut detail = String::new();
    let mut ok = true;
    for (i, (name, _)) in sources.iter().enumerate() {
        let eq = b1[i].program.bytecode_equal(&b2[i].program);
        let dg = program_digest(&b1[i].program) == program_digest(&b2[i].program);
        if !eq || !dg {
            ok = false;
            detail.push_str(&format!("{}: equal={} digest={}; ", name, eq, dg));
        }
    }
    restore_fresh();
    if ok {
        pass("B₁/B₂ bytecode_equal + SHA-256 一致（compiler + preamble 两件）".to_string())
    } else {
        fail(format!("门 B 轻量终验失败：{}", detail))
    }
}

/// 恢复 fresh 自举状态 + 开缓存（探针间状态隔离）。
fn restore_fresh() {
    reset_reader();
    reset_expander();
    reset_compiler();
    set_cache_enabled(true);
}

/// P04（§21.3 条件 3——非 VM 后端）：QBE 本地码 fib(12) 端到端
/// （compile → lower → gen_il → AOT build → 运行 exit 144）。
/// 工具链缺失即失败（§3.1 不静默跳过）。
fn probe_qbe_native_fib() -> CaseResult {
    use kerf_backend::{build_native, find_qbe, gen_il, lower_program, run_native};
    let out = match compile_source(FIB12, "<audit>") {
        Ok(o) => o,
        Err(e) => return fail(format!("前端编译失败：{}", first_line(&e.rendered))),
    };
    let anf = match lower_program(&out.core, &out.table) {
        Ok(a) => a,
        Err(e) => return fail(format!("lowering 失败：{:?}", e)),
    };
    let il = gen_il(&anf);
    let qbe = match find_qbe() {
        Ok(q) => q,
        Err(e) => return fail(format!("QBE 工具链缺失（§3.1 不静默跳过）：{:?}", e)),
    };
    let out_dir = std::env::temp_dir().join(format!("kerf-audit-qbe-{}", std::process::id()));
    let arts = match build_native(&il, &out_dir, "audit_fib", &qbe) {
        Ok(a) => a,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&out_dir);
            return fail(format!("AOT 构建失败：{:?}", e));
        }
    };
    let exit = match run_native(&arts.exe_path) {
        Ok(r) => r,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&out_dir);
            return fail(format!("本地码运行失败：{:?}", e));
        }
    };
    let _ = std::fs::remove_dir_all(&out_dir);
    if exit == 144 {
        pass("QBE 本地码 fib(12) exit 144（§21.3 条件 3——非 VM 后端活性）".to_string())
    } else {
        fail(format!("本地码 exit {} ≠ 144", exit))
    }
}

/// P05（§21.3 条件 4——批次 I 口径如实）：FFI 所有权模型冻结面
/// ——文档存在 + 关键设计锚（所有权裁定）。实现属 Stage 2 后续批次
/// （r17 冻结模型——本断言锚定「模型可用/已定义」而非运行时可用）。
fn probe_ffi_model_frozen() -> CaseResult {
    let path = "docs/develop/v0/stage-2/ffi-ownership-model.md";
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => return fail(format!("FFI 模型文档缺失（{}）：{}", path, e)),
    };
    let has_ownership = text.contains("所有权");
    let has_verdict = text.contains("裁定") || text.contains("决策") || text.contains("方案");
    if !has_ownership || !has_verdict {
        return fail(format!(
            "FFI 模型文档缺关键锚（所有权={} / 裁定={})",
            has_ownership, has_verdict
        ));
    }
    pass("FFI 所有权模型冻结（r17——文档锚在位；实现属后续批次，如实登记）".to_string())
}

// ---------------------------------------------------------------------------
// main：配比机械校验 + 运行
// ---------------------------------------------------------------------------

fn main() {
    println!("== kerf Stage 2 批次 I 门审计集 stage2_gate_audit_r1（sop.md §7.3.1 / §7.1.1 / §7.3.2 / §21.3）==");
    println!(
        "case 总数 {}（A 单语句 12 / B 多语句 12 / C 复杂 10 / D 恢复 6 / E 边界 7 / P 正向 6）\n",
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
    if total < 30 {
        println!("配比违规：总数 {} < 30", total);
        ratio_fail = true;
    }
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
    println!("== §21.3 四条件锚定（Stage 2 批次 I 口径）==");
    println!("条件 1（完整 kerf 编译器用 kerf 编写）：生产管线三段自举（VM Reader+Expander+Compiler）——全体 case 经生产路径");
    println!("条件 2（两次编译自身结果一致）：P03 门 B 轻量终验（B₁/B₂ bytecode_equal + SHA-256）+ 既有 gate_b_fixpoint 全四件");
    println!("条件 3（至少一个非 VM 后端工作）：P04 QBE 本地码 fib 端到端（r17 PoC）");
    println!("条件 4（FFI 可用）：P05 所有权模型冻结（r17——实现属 Stage 2 后续批次，如实登记）");
}
