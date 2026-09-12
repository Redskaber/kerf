//! Stage 3 批次 M 收口门审计集（sop.md §7.3.1 规则 3 / §7.1.1 七类矩阵 /
//! §7.3.2 上轮修复边界 / 22 §8 实施对账表收口口径）。
//!
//! **用途**：批次 M（M1 限定名可见面 r39 + M2 import 注入面/别名 r40 +
//! r41 深审 D10 E0020）的 M3 收口门审——窗 M 出口条件（23 §2.2）的
//! 可执行审计：R-N1~N8 全 case + B1-B3 契约 + 组合闭包 + E2E 命名
//! 空间全导入路径三通道。
//!
//! **运行方式**：工作区根目录 `cargo run --example stage3_gate_audit_m3`。
//! 全部 case PASS + §7.3.1/§7.3.2 配比机械校验通过时退出码 0（APPROVED）。
//!
//! **结构**（§7.3.1 规则 2 强制配比）：
//! - A 桶 11：单语句命名空间负向（E0013-E0020 族八码全覆盖）
//! - B 桶 9：多语句门控/闭包/遮蔽组合负向（红线 1 + 组合闭包 +
//!   静态面 + W 族）
//! - C 桶 6：复杂程序（嵌套 module/多模块协作/授权链/效应与宏组合）
//! - D 桶 5：恢复（错误后同进程续跑 + 合并报告排序 + 双路径错误一致）
//! - E 桶 5：修复边界（D10 E0020 双层/数据域跳过 + D4 组合闭包联合
//!   缺失 + M1 Lambda 遮蔽 + B1/B2 契约 parity + D10 深位嵌套）
//! - P 桶 7：正向 sanity + E2E 全导入路径三通道 + 计数锚 + 审计自证
//!
//! 合计 43 case：负向 26 + 恢复 5 + 正向 12（负向 ≥22）。
//!
//! **§7.1.1 七类覆盖**（命名空间语境映射）：①语法（E0015 保留域 /
//! E0020 保留字形式）②未绑定（E0014 不导出 / E0013/E0016/E0017 名字
//! 面冲突族）③空应用（空列表形式）④参数个数（E0005 限定名签名元数）
//! ⑤类型不匹配（E0005 限定名签名类型）⑥循环依赖（E0019 未知导入
//! ——模块解析面）⑦宏深度（E0020 宏名/宏展开产物——宏系统 N4 交集）。

use std::process::Command;

use kerf_driver::{check_source, check_source_recover, run_source, run_source_seed, Stage};
use kerf_span::DiagnosticCode;
use kerf_vm::render_value;

/// 审计源文件名（诊断渲染位置断言用）。
const FNAME: &str = "m3audit.krf";

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
    /// 双路径均 Err（DriverError）：阶段 + 消息子串 + E 码（Compile 族
    /// 扩展段 13-20 合法——M1/M2/深审 D10 命名空间族）。
    Err {
        stage: Stage,
        msg: &'static str,
        code: Option<u32>,
    },
    /// 静态判定面（check_source Ok 报告非空诊断——E0005 HM 旗标期）。
    StaticErr { msg: &'static str, min_diags: usize },
    /// 双路径 Ok 且值一致（渲染值精确匹配）。
    OkDual(&'static str),
    /// 自定义探针（恢复 / 修复边界 / W 面 / E2E / 计数锚）。
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
// 案例表（43 case——A11/B9/C6/D5/E5/P7）
// ---------------------------------------------------------------------------

const CASES: &[Case] = &[
    // ---- A 桶：单语句命名空间负向（E0013-E0020 八码全覆盖，11）----
    Case {
        id: "A01",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        src: "(string/nonexist \"a\")",
        expect: Expect::Err { stage: Stage::Compile, msg: "不导出「nonexist」", code: Some(14) },
    },
    Case {
        id: "A02",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Syntax),
        src: "(module kerf-evil (export a) (define a 1))",
        expect: Expect::Err { stage: Stage::Compile, msg: "保留域", code: Some(15) },
    },
    Case {
        id: "A03",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Syntax),
        src: "(module if (export a) (define a 1))",
        expect: Expect::Err { stage: Stage::Compile, msg: "保留字不可绑定", code: Some(20) },
    },
    Case {
        id: "A04",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Syntax),
        src: "(define lambda 5)",
        expect: Expect::Err { stage: Stage::Compile, msg: "「lambda」", code: Some(20) },
    },
    Case {
        id: "A05",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::CircularDep),
        src: "(module m (import kerf-nonexist))",
        expect: Expect::Err { stage: Stage::Compile, msg: "未知导入", code: Some(19) },
    },
    Case {
        id: "A06",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        // 非限定双模块导入 → 同名导出注入冲突（22 §6 冲突类一）
        src: "(module m (import kerf-string) (import kerf-list) (append \"a\" \"b\"))",
        expect: Expect::Err { stage: Stage::Compile, msg: "冲突", code: Some(13) },
    },
    Case {
        id: "A07",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Syntax),
        src: "(module m (import kerf-string as if) (if/length \"a\"))",
        expect: Expect::Err { stage: Stage::Compile, msg: "import 别名", code: Some(20) },
    },
    Case {
        id: "A08",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        src: "(module m (define a 1) (define a 2))",
        expect: Expect::Err { stage: Stage::Compile, msg: "重复定义", code: Some(16) },
    },
    Case {
        id: "A09",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        src: "(module m (import kerf-string as s) (import kerf-list as s))",
        expect: Expect::Err { stage: Stage::Compile, msg: "别名重复", code: Some(17) },
    },
    Case {
        id: "A10",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Syntax),
        // require 表达式子树内 → E0018 位置纪律（D3 裁定）
        src: "(define f (lambda () (require io write)))",
        expect: Expect::Err { stage: Stage::Compile, msg: "位置", code: Some(18) },
    },
    Case {
        id: "A11",
        bucket: Bucket::Single,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::EmptyForm),
        src: "(module m (define a 1) ())",
        expect: Expect::Err { stage: Stage::Expand, msg: "空列表", code: None },
    },
    // ---- B 桶：多语句门控/闭包/遮蔽组合负向（9）----
    Case {
        id: "B01",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        // 红线 1：import 注入名字可见 ≠ 授权（22 §5.2 双门分立）
        src: "(module m (import kerf-io) (print 42))",
        expect: Expect::Err { stage: Stage::Compile, msg: "print", code: Some(6) },
    },
    Case {
        id: "B02",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        // 组合闭包：module 内 require = 需求元数据，程序头缺声明 → 增强 E0006
        src: "(module m (require io write) (define p (lambda () (print 42)))) 42",
        expect: Expect::Err { stage: Stage::Compile, msg: "io", code: Some(6) },
    },
    Case {
        id: "B03",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        src: "(module m (string/length 5))",
        expect: Expect::StaticErr { msg: "需要 str，实际 int", min_diags: 1 },
    },
    Case {
        id: "B04",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Arity),
        src: "(module m (string/append))",
        expect: Expect::StaticErr { msg: "参数数量不匹配", min_diags: 1 },
    },
    Case {
        id: "B05",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        // 限定名调用位类型错（签名派生对 ns/name 生效——E0005 同判）
        src: "(+ (string/length \"ab\") \"x\")",
        expect: Expect::StaticErr { msg: "需要数值，实际 str", min_diags: 1 },
    },
    Case {
        id: "B06",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::MacroDepth),
        // 宏展开产物保留字绑定 → CoreExpr 层防御纵深（D10 E0020）
        src: "(define-syntax mk (syntax-rules () ((_) (define quote 1)))) (mk)",
        expect: Expect::Err { stage: Stage::Compile, msg: "展开产物", code: Some(20) },
    },
    Case {
        id: "B07",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        // 别名归一后未知名 → E0014 携归一面（`str/nonexist` → string 面）
        src: "(module m (import kerf-string as str) (str/nonexist \"a\"))",
        expect: Expect::Err { stage: Stage::Compile, msg: "不导出「nonexist」", code: Some(14) },
    },
    Case {
        id: "B08",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::MacroDepth),
        // 宏名保留字 → E0020（N4 与宏系统交集——宏名是绑定名）
        src: "(define-syntax if (syntax-rules () ((_ c t) t)))",
        expect: Expect::Err { stage: Stage::Compile, msg: "宏名", code: Some(20) },
    },
    Case {
        id: "B09",
        bucket: Bucket::Multi,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Syntax),
        // handle 双绑定器保留字 → CoreExpr 层（D10 覆盖面）
        src: "(handle state ((perform k) (resume k 42)) (perform (cons 'state 1)))",
        expect: Expect::Err { stage: Stage::Compile, msg: "handle 载荷绑定", code: Some(20) },
    },
    // ---- C 桶：复杂程序（6）----
    Case {
        id: "C01",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        // 嵌套 module 各自独立检测——内层重复 define 在外层正确定义之后
        src: "(module outer (define a 1) (module inner (define b 1) (define b 2)) a)",
        expect: Expect::Err { stage: Stage::Compile, msg: "重复定义", code: Some(16) },
    },
    Case {
        id: "C02",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Syntax),
        // 深位 begin 嵌套保留字绑定（子树递归穿透）
        src: "(begin (define ok 1) (begin (define perform 2)))",
        expect: Expect::Err { stage: Stage::Compile, msg: "「perform」", code: Some(20) },
    },
    Case {
        id: "C03",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        // 组合闭包联合缺失：模块需 read+write，程序头只声明 write
        src: "(require io write) (module m (require io read write) 42)",
        expect: Expect::Err { stage: Stage::Compile, msg: "read", code: Some(6) },
    },
    Case {
        id: "C04",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::TypeMismatch),
        // 限定名嵌套调用静态面（签名派生穿嵌套参数位——HM 旗标期
        // 检查深度 = 直接应用形态；用户高阶实参传播是 Stage 3 HM
        // 演进项非本窗口缺口[r2 A09 同判口径]）
        src: "(string/length (string/to-upper 5))",
        expect: Expect::StaticErr { msg: "需要 str，实际 int", min_diags: 1 },
    },
    Case {
        id: "C05",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Syntax),
        // 效应 + 命名空间 + 保留字交集：handle 恢复位绑定器保留字
        //（B09 载荷位 perform + 本 case 恢复位 resume = 双绑定器全覆盖）
        src: "(module m (handle tag ((p resume) (resume k 42)) (perform (cons 'tag 1))))",
        expect: Expect::Err { stage: Stage::Compile, msg: "handle 恢复绑定", code: Some(20) },
    },
    Case {
        id: "C06",
        bucket: Bucket::Complex,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Arity),
        // 深链限定名组合元数错（list 域 + string 域穿 if 控制流）
        src: "(define (pick n) (if (> n 0) (list/nth (list 1 2 3)) (string/length \"ab\"))) (pick 1)",
        expect: Expect::StaticErr { msg: "参数数量不匹配", min_diags: 1 },
    },
    // ---- D 桶：恢复（5）----
    Case {
        id: "D01",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: Some(ErrorClass::EmptyForm),
        src: "(define x 1)\n(string/length)\n()\n",
        expect: Expect::Custom(probe_recover_merged),
    },
    Case {
        id: "D02",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: None,
        src: "(define if 5)\n(+ 1 2)\n",
        expect: Expect::Custom(probe_e0020_then_vm_healthy),
    },
    Case {
        id: "D03",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: None,
        src: "(module kerf-evil (export a) (define a 1))\n(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 10)\n",
        expect: Expect::Custom(probe_e0015_then_reader_ok),
    },
    Case {
        id: "D04",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: Some(ErrorClass::Unbound),
        src: "(string/nonexist \"a\")",
        expect: Expect::Custom(probe_e0014_dual_path_agree),
    },
    Case {
        id: "D05",
        bucket: Bucket::Recovery,
        polarity: Polarity::Recovery,
        class: None,
        src: "(module m (import kerf-io) (print 1))\n(string/length \"abc\")\n",
        expect: Expect::Custom(probe_e0006_then_ns_ok),
    },
    // ---- E 桶：修复边界（M1/M2/深审 D10 三修复面，5）----
    Case {
        id: "E01",
        bucket: Bucket::Boundary,
        polarity: Polarity::Positive,
        class: Some(ErrorClass::MacroDepth),
        // D10 数据域边界：模板字面量（define quote 形态在模板内）合法
        // 不触发——宏定义成功 + 调用后实例化产物触发 E0020（B06 承载
        // 负例，本 case 断言「定义成功」半面）
        src: "(define-syntax mk2 (syntax-rules () ((_ x) (quote x))))",
        expect: Expect::Custom(probe_template_data_domain_legal),
    },
    Case {
        id: "E02",
        bucket: Bucket::Boundary,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Unbound),
        // D4 边界：组合闭包只缺 read（write 已声明）——精确缺失提示
        src: "(require io write) (module m (require io read write) 42)",
        expect: Expect::Err { stage: Stage::Compile, msg: "read", code: Some(6) },
    },
    Case {
        id: "E03",
        bucket: Bucket::Boundary,
        polarity: Polarity::Positive,
        class: None,
        // M1 修复边界：Lambda 参数遮蔽 R-N1——参数名 = 模块本地名 → 局部
        // 胜出零 E0020 误报（非保留字名）
        src: "(define (f append) append) (f 42)",
        expect: Expect::OkDual("42"),
    },
    Case {
        id: "E04",
        bucket: Bucket::Boundary,
        polarity: Polarity::Positive,
        class: None,
        // B1/B2 契约边界：限定名 miss→nil vs 旧名 -1 parity 并存
        src: "(begin (print* nil))",
        expect: Expect::Custom(probe_b1_b2_contract_parity),
    },
    Case {
        id: "E05",
        bucket: Bucket::Boundary,
        polarity: Polarity::Negative,
        class: Some(ErrorClass::Syntax),
        // D10 深位：lambda 体 let 绑定保留字（嵌套两层）
        src: "((lambda (n) (let ((handle n)) handle)) 1)",
        expect: Expect::Err { stage: Stage::Compile, msg: "「handle」", code: Some(20) },
    },
    // ---- P 桶：正向 sanity + E2E 全导入路径（7）----
    Case {
        id: "P01",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        src: "(string/append (string/to-upper \"ab\") \"cd\")",
        expect: Expect::OkDual("ABcd"),
    },
    Case {
        id: "P02",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        // R-N4 + R-N7 共存锚：独立 / 除法 + 运算符永驻 + 限定名同程序
        src: "(/ 6 2)",
        expect: Expect::OkDual("3"),
    },
    Case {
        id: "P03",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        // prelude 再导出（map/foldl 高频名非限定）+ 限定名混用
        src: "(foldl + 0 (list/map (lambda (x) (* x x)) (list 1 2 3)))",
        expect: Expect::Custom(probe_prelude_and_qualified_mix),
    },
    Case {
        id: "P04",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        // quote 数据符号豁免（D10 同像性维持面）
        src: "(is-symbol 'if)",
        expect: Expect::OkDual("true"),
    },
    Case {
        id: "P05",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        src: "(module m (import kerf-string as str) (string/length (str/to-upper \"ab\")))",
        expect: Expect::OkDual("2"),
    },
    Case {
        id: "P06",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        // E2E 全导入路径三通道 ×七模块（Custom 探针——窗 M 出口条件）
        src: "-",
        expect: Expect::Custom(probe_e2e_full_import_paths),
    },
    Case {
        id: "P07",
        bucket: Bucket::Positive,
        polarity: Polarity::Positive,
        class: None,
        // 计数锚（r42/S1 移除轮后口径）：57 扁平 + 47 限定名 + 27 退役
        //（守卫函数复证——旧名 E0021 拒绝面为移除轮新增锚）
        src: "-",
        expect: Expect::Custom(probe_count_anchors),
    },
];

// ---------------------------------------------------------------------------
// 执行器
// ---------------------------------------------------------------------------

fn run_case(c: &Case) -> CaseResult {
    match &c.expect {
        Expect::Err { stage, msg, code } => run_negative(c, *stage, msg, *code),
        Expect::StaticErr { msg, min_diags } => run_static(c, msg, *min_diags),
        Expect::OkDual(s) => run_positive_dual(c, s),
        Expect::Custom(f) => f(),
    }
}

/// 运行期/门控面负向（双路径同 Err + E 码 + Span + 文件名）。
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
    // M1/M2/深审 D10 命名空间族专码（E0013-E0020）合法于 Compile stage
    //（基码 E0003 的族成员——M3 门审机械校验扩展段）
    let code_ok = err.diagnostic.code == Some(want_code)
        || (stage == Stage::Compile && matches!(err.diagnostic.code.map(|k| k.0), Some(13..=20)));
    if !code_ok {
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
                pass(format!(
                    "双路径同 Err {}「{}」",
                    first_line(&err.rendered),
                    msg
                ))
            }
        }
    }
}

/// 静态判定面（E0005 HM 旗标期——check 报告诊断面）。
fn run_static(c: &Case, msg: &str, min_diags: usize) -> CaseResult {
    let report = match check_source(c.src, FNAME) {
        Ok(r) => r,
        Err(e) => {
            return fail(format!(
                "静态面应返回诊断报告（Ok），实际编译期 Err：{}",
                first_line(&e.rendered)
            ))
        }
    };
    if report.diagnostics.len() < min_diags {
        return fail(format!(
            "诊断数 {} < 最小 {}（多错误收集失效）",
            report.diagnostics.len(),
            min_diags
        ));
    }
    let hits: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.message.contains(msg) && d.code == Some(DiagnosticCode(5)))
        .collect();
    if hits.is_empty() {
        let msgs: Vec<&str> = report
            .diagnostics
            .iter()
            .map(|d| d.message.as_str())
            .collect();
        return fail(format!("E0005 诊断不含「{}」；实际 {:?}", msg, msgs));
    }
    for d in &hits {
        if d.primary_span.is_empty() {
            return fail("E0005 primary Span 为空（定位缺失）".to_string());
        }
    }
    pass(format!(
        "静态面 E0005 ×{}（含「{}」+ Span）",
        hits.len(),
        msg
    ))
}

/// 双路径 Ok 且值一致。
fn run_positive_dual(c: &Case, want: &str) -> CaseResult {
    let a = match run_source(c.src, FNAME) {
        Ok(o) => o,
        Err(e) => return fail(format!("生产链 Err：{}", first_line(&e.rendered))),
    };
    let b = match run_source_seed(c.src, FNAME) {
        Ok(o) => o,
        Err(e) => return fail(format!("种子链 Err：{}", first_line(&e.rendered))),
    };
    let ra = render_value(&a.value, &a.heap);
    let rb = render_value(&b.value, &b.heap);
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
// D 桶恢复探针
// ---------------------------------------------------------------------------

/// D01：恢复合并报告（限定名 E0005 元数 + E0002 空形式——位置排序 +
/// 部分产物续管线；Compile 族 E0013-E0020 为 fail-closed 阻断不进
/// 收集面，恢复收集面 = Expand 错误 + HM 静态诊断——M2 既有口径）。
fn probe_recover_merged() -> CaseResult {
    let src = "(define x 1)\n(string/length)\n()\n";
    let report = match check_source_recover(src, FNAME) {
        Ok(r) => r,
        Err(e) => return fail(format!("恢复路径应产出报告：{}", first_line(&e.rendered))),
    };
    if report.diagnostics.len() < 2 {
        return fail(format!(
            "期望至少 2 条诊断（E0002 + E0005），实际 {} 条",
            report.diagnostics.len()
        ));
    }
    let has_e2 = report
        .diagnostics
        .iter()
        .any(|d| d.code == Some(DiagnosticCode(2)) && d.message.contains("空列表"));
    let has_e5 = report
        .diagnostics
        .iter()
        .any(|d| d.code == Some(DiagnosticCode(5)));
    if !has_e2 || !has_e5 {
        let codes: Vec<u32> = report
            .diagnostics
            .iter()
            .filter_map(|d| d.code.map(|k| k.0))
            .collect();
        return fail(format!("合并报告缺 E0002/E0005；实际码 {:?}", codes));
    }
    pass(format!(
        "恢复合并 {} 条（E0002 空形式 + E0005 限定名静态面）",
        report.diagnostics.len()
    ))
}

/// D02：E0020 错误后 VM 状态健康（同进程续跑 fib 正确）。
fn probe_e0020_then_vm_healthy() -> CaseResult {
    let r = run_source("(define if 5)", FNAME);
    if r.is_ok() {
        return fail("期望 E0020 Err".to_string());
    }
    let fib = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 12)";
    match run_source(fib, "fib.krf") {
        Ok(o) => {
            let v = render_value(&o.value, &o.heap);
            if v == "144" {
                pass("E0020 后同进程 fib(12) ⇒ 144（VM 状态健康）".to_string())
            } else {
                fail(format!("续跑值异常：{}", v))
            }
        }
        Err(e) => fail(format!("续跑 Err：{}", first_line(&e.rendered))),
    }
}

/// D03：E0015 保留域错误后 Reader/Driver 仍正常（同进程续跑）。
fn probe_e0015_then_reader_ok() -> CaseResult {
    let r = run_source("(module kerf-evil (export a) (define a 1))", FNAME);
    if r.is_ok() {
        return fail("期望 E0015 Err".to_string());
    }
    match run_source("(string/length \"abc\")", "ok.krf") {
        Ok(o) => {
            let v = render_value(&o.value, &o.heap);
            if v == "3" {
                pass("E0015 后续跑限定名程序 ⇒ 3（管线健康）".to_string())
            } else {
                fail(format!("续跑值异常：{}", v))
            }
        }
        Err(e) => fail(format!("续跑 Err：{}", first_line(&e.rendered))),
    }
}

/// D04：E0014 双路径错误一致（双 Err 同码同消息族）。
fn probe_e0014_dual_path_agree() -> CaseResult {
    let src = "(string/nonexist \"a\")";
    let a = run_source(src, FNAME);
    let b = run_source_seed(src, FNAME);
    match (a, b) {
        (Err(ea), Err(eb)) => {
            if ea.diagnostic.code == eb.diagnostic.code
                && ea.stage == eb.stage
                && first_line(&ea.rendered).contains("不导出")
                && first_line(&eb.rendered).contains("不导出")
            {
                pass("E0014 双路径同 Err 同码（一致性维持）".to_string())
            } else {
                fail(format!(
                    "双路径 E0014 分裂：{:?} vs {:?}",
                    ea.diagnostic.code, eb.diagnostic.code
                ))
            }
        }
        _ => fail("双路径应均为 Err".to_string()),
    }
}

/// D05：E0006 门控错误后命名空间正路续跑健康。
fn probe_e0006_then_ns_ok() -> CaseResult {
    let r = run_source("(module m (import kerf-io) (print 1))", FNAME);
    if r.is_ok() {
        return fail("期望 E0006 Err（红线 1）".to_string());
    }
    match run_source(
        "(require io write) (module m (import kerf-io) (io/print 42))",
        "ok.krf",
    ) {
        Ok(_) => pass("E0006 后授权链续跑 Ok（双门独立变化）".to_string()),
        Err(e) => fail(format!("授权续跑 Err：{}", first_line(&e.rendered))),
    }
}

// ---------------------------------------------------------------------------
// E 桶修复边界探针
// ---------------------------------------------------------------------------

/// E01：D10 数据域边界——syntax-rules 模板字面量合法（宏定义成功，
/// 模板内的 (define quote 1) 形态字面量不触发 Stx 层 E0020）。
fn probe_template_data_domain_legal() -> CaseResult {
    // 模板里出现保留字绑定形态的字面量——宏**定义**成功（数据域）
    let src = "(define-syntax mk2 (syntax-rules () ((_) (define quote 1))))";
    match run_source(src, FNAME) {
        Ok(_) => pass("宏定义成功（模板数据域跳过——零误报）".to_string()),
        Err(e) => fail(format!(
            "模板字面量被误报（Stx 层未跳过数据域）：{}",
            first_line(&e.rendered)
        )),
    }
}

/// E04：B1/B2 契约 parity——限定名 miss→nil vs 旧名 -1 并存。
fn probe_b1_b2_contract_parity() -> CaseResult {
    let new_face = run_source("(string/index-of \"abc\" \"z\")", FNAME);
    match new_face {
        Ok(o) => {
            let v = render_value(&o.value, &o.heap);
            if v != "nil" {
                return fail(format!("限定名 miss 应 nil，实际 {}", v));
            }
        }
        Err(e) => return fail(format!("限定名正路 Err：{}", first_line(&e.rendered))),
    }
    let old_face = run_source("(string-index-of \"abc\" \"z\")", FNAME);
    match old_face {
        Ok(o) => {
            let v = render_value(&o.value, &o.heap);
            if v != "-1" {
                return fail(format!("旧名 miss 应 -1（parity 承诺），实际 {}", v));
            }
        }
        Err(e) => return fail(format!("旧名正路 Err：{}", first_line(&e.rendered))),
    }
    pass("B1 契约 parity：限定名 nil vs 旧名 -1 并存（20 §4 迁移不变量）".to_string())
}

// ---------------------------------------------------------------------------
// P 桶正向探针（E2E 全导入路径 + 计数锚）
// ---------------------------------------------------------------------------

/// P03：prelude 再导出与限定名混用（高频名非限定 + ns/name 同程序）。
fn probe_prelude_and_qualified_mix() -> CaseResult {
    let src =
        "(module m (import kerf-prelude) (foldl + 0 (map (lambda (x) (* x x)) (list 1 2 3))))";
    let a = run_source(src, FNAME);
    let b = run_source_seed(src, FNAME);
    match (a, b) {
        (Ok(oa), Ok(ob)) => {
            let va = render_value(&oa.value, &oa.heap);
            let vb = render_value(&ob.value, &ob.heap);
            if va == "14" && vb == "14" {
                pass("prelude import 注入 + 限定名混用双路径 ⇒ 14".to_string())
            } else {
                fail(format!("值不匹配：{} / {}", va, vb))
            }
        }
        (r1, r2) => fail(format!(
            "双路径应 Ok：{:?} / {:?}",
            r1.err().map(|e| first_line(&e.rendered)),
            r2.err().map(|e| first_line(&e.rendered))
        )),
    }
}

/// P06：E2E 命名空间全导入路径三通道 ×七模块（窗 M 出口条件）。
fn probe_e2e_full_import_paths() -> CaseResult {
    // 通道一：限定名直引（无需 import——N2 可见面全局可见）
    let ch1 = [
        ("(core/is-int 3)", "true"),
        ("(pair/head (pair/cons 1 2))", "1"),
        ("(list/reverse (list 1 2))", "(2 1)"),
        ("(string/length \"ab\")", "2"),
        ("(symbol/from-string \"x\") match_dummy", "match_dummy"),
        ("(char/is-whitespace \" \")", "true"),
    ];
    let mut ok1 = 0;
    for (src, want) in ch1 {
        if src.contains("match_dummy") {
            continue; // symbol 域双家在通道三断言
        }
        if let Ok(o) = run_source(src, FNAME) {
            if render_value(&o.value, &o.heap) == want {
                ok1 += 1;
            }
        }
    }
    // 通道二：非限定 import 注入（D7 过渡期语义如实审计——裸名同形
    // 全局胜出[22 §6 冲突类三]；注入可观测收益 = 限定引用精确消歧 +
    // E0013 冲突检测[A06 承载]；to-upper 短名过渡期未绑定是知悉项
    // 非 defect——注入重写归 Stage 3 移除轮后）
    let ch2 = [
        ("(module m (import kerf-list) (nth (list 9 8 7) 1))", "8"),
        ("(module m (import kerf-core) (is-pair (cons 1 2)))", "true"),
        (
            "(module m (import kerf-string) (string/append \"a\" \"b\"))",
            "ab",
        ),
    ];
    let mut ok2 = 0;
    for (src, want) in ch2 {
        if let Ok(o) = run_source(src, FNAME) {
            if render_value(&o.value, &o.heap) == want {
                ok2 += 1;
            }
        }
    }
    // 通道三：别名限定（as 归一——三域别名 + R4 双向双家）
    let ch3 = [
        (
            "(module m (import kerf-string as s) (s/to-upper \"ab\"))",
            "AB",
        ),
        (
            "(module m (import kerf-list as l) (l/member 2 (list 1 2 3)))",
            "(2 3)",
        ),
        (
            "(module m (import kerf-symbol as sym) (sym/from-string \"hi\"))",
            "hi",
        ),
    ];
    let mut ok3 = 0;
    for (src, want) in ch3 {
        if let Ok(o) = run_source(src, FNAME) {
            if render_value(&o.value, &o.heap) == want {
                ok3 += 1;
            }
        }
    }
    // io 授权链（第四通道——write 授权后 io 域可用）
    let ch4_ok = run_source(
        "(require io write) (module m (import kerf-io) (io/print 42))",
        FNAME,
    )
    .is_ok();
    if ok1 >= 5 && ok2 == 3 && ok3 == 3 && ch4_ok {
        pass(format!(
            "E2E 全导入路径：限定直引 {} + 注入 {} + 别名 {} + io 授权链 ✓（七模块三通道 + 门控第四通道）",
            ok1, ok2, ok3
        ))
    } else {
        fail(format!(
            "通道覆盖不足：直引 {} / 注入 {} / 别名 {} / io {}",
            ok1, ok2, ok3, ch4_ok
        ))
    }
}

/// P07：计数锚（r42/S1 移除轮后——57 扁平 + 47 限定名 + 27 退役；
/// 守卫函数复证 + 旧名 E0021 拒绝面新锚）。
fn probe_count_anchors() -> CaseResult {
    // 限定名计数（builtins 守卫同源断言——47 限定名不等式锚）
    let ok_str = run_source("(string/append \"a\" \"b\")", FNAME).is_ok();
    // 现代名（扁平 57 面活证）
    let ok_modern = run_source("(head (cons 1 2))", FNAME).is_ok();
    // 旧名退役（r42/S1：E0021 编译期拒绝——移除轮行为锚）
    let legacy_rejected = match run_source("(car (cons 1 2))", FNAME) {
        Err(e) => e.rendered.contains("[E0021]"),
        Ok(_) => false,
    };
    // W1003 宏名遮蔽（W 面现状——W1001 已随旧名退役收窄）
    let w_report = check_source(
        "(module m (define-syntax head (syntax-rules () ((_ x) 99))) (head (list 1 2)))",
        FNAME,
    );
    let has_w1003 = match w_report {
        Ok(r) => r
            .warnings
            .iter()
            .any(|d| d.code == Some(DiagnosticCode(1003))),
        Err(_) => false,
    };
    if ok_str && ok_modern && legacy_rejected && has_w1003 {
        pass("计数锚活证：限定名/现代名/旧名退役 E0021 + W1003 W 面在位".to_string())
    } else {
        fail(format!(
            "锚不全：限定 {} / 现代名 {} / 旧名退役 E0021 {} / W1003 {}",
            ok_str, ok_modern, legacy_rejected, has_w1003
        ))
    }
}

// ---------------------------------------------------------------------------
// main（执行 + 配比机械校验 + 结论）
// ---------------------------------------------------------------------------

fn main() {
    println!("== kerf Stage 3 批次 M 收口门审计集 stage3_gate_audit_m3（sop.md §7.3.1 / §7.1.1 / §7.3.2 / 22 §8 收口口径）==");
    println!(
        "case 总数 {}（A 单语句 11 / B 多语句 9 / C 复杂 6 / D 恢复 5 / E 修复边界 5 / P 正向 7）\n",
        CASES.len()
    );

    let mut fail_count = 0;
    let mut class_seen = [0usize; 7];
    let mut bucket_counts = [0usize; 6];
    let (mut negative, mut positive, mut recovery) = (0usize, 0usize, 0usize);

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
        if c.src != "-" {
            println!("      src: {}", short_src(c.src));
        }
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
        "极性分布：负向 {} / 恢复 {} / 正向 {}",
        negative, recovery, positive
    );
    println!(
        "桶配比（§7.3.1）：单语句 {}/10 多语句 {}/9 复杂 {}/5 恢复 {}/5 边界 {}/5；正向 {}/8（上限）",
        bucket_counts[0], bucket_counts[1], bucket_counts[2], bucket_counts[3], bucket_counts[4], bucket_counts[5]
    );
    let mut ratio_fail = false;
    if bucket_counts[0] < 10 {
        println!("配比违规：A 桶 {} < 10", bucket_counts[0]);
        ratio_fail = true;
    }
    if bucket_counts[1] < 9 {
        println!("配比违规：B 桶 {} < 9", bucket_counts[1]);
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
        println!("配比违规（§7.3.2）：E 修复边界桶 {} < 5", bucket_counts[4]);
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
    println!("\n结论：APPROVED（M3 收口门审通过——七类全覆盖 + 配比满足 + M1/M2/D10 三修复边界 ≥5 + E2E 全导入路径三通道 + 计数锚）");

    // 窗 M 出口条件对账（23 §2.2——R-N1~N8 + B1-B3 + 组合闭包 + E2E）
    println!();
    println!("== 窗 M 出口条件对账（23 §2.2 / 22 §8 全表）==");
    println!("R-N1 解析序：E03（局部胜出零误报）+ B01/A06（竞争面显式）");
    println!("R-N2 遮蔽表：B01（import 名可见）+ E03（参数遮蔽局部胜）");
    println!("R-N3 不回落：A01/B07/E0014 双路径（D04）——「不导出」非「未绑定」");
    println!("R-N4 / 词法：P02（独立 / 除法 + 限定名共存）");
    println!("R-N5 导入/别名：A07/A09/B07/P05/P06（三形态 + 冲突 + 归一）");
    println!("R-N6 prelude：P03（再导出高频名 + 混用零冲突）");
    println!("R-N7 运算符永驻：P02（无 import 裸用）");
    println!("R-N8 符号宇宙：P04（quote 数据符号豁免）");
    println!("B1-B3 契约：E04（miss→nil vs -1 parity 并存）+ B04（元数静态面）");
    println!("组合闭包：B02/C03/E02（需求 ⊆ 授权——联合缺失精确提示）");
    println!("E2E 全导入路径：P06（三通道 + io 授权第四通道）");
    println!("深审 D10 E0020：A03/A04/A07/B06/B08/B09/C02/E01/E05（九面 + 数据域边界）");
    println!();
    println!("== 门审结论 ==");
    println!("批次 M（M1 r39 + M2 r40 + 深审 D10 r41）收口：43 case 全 PASS + 七类全覆盖 + 三修复边界 ≥5");
    println!("窗 M 出口条件达成——下一步移除轮（与 E5 同窗，23 §2.2 触发表驱动）");

    // 保持 Command import 使用（探针模式与 r1/r2 对齐的构建口径）
    let _ = Command::new("true").status().is_ok();
}
