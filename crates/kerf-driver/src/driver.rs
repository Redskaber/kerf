//! 驱动层：全管线编排（read → expand → lower → compile → run）。
//!
//! 入口（§10.1 规则 1 自由函数）：
//! - `compile_source`：源文本 → `CompileOutput`（完整管线含图 IR——
//!   dump/检查/CodeValue 消费方入口，TD-015 分流的完整出口）；
//! - `run_source`：编译前段 + VM 执行（生产路径，不构造图 IR——
//!   前段三段（读/展开/编译）均自举实现：I1 生产切换 42-d）；
//! - `run_source_seed`/`compile_source_seed`：种子管线（Rust 三段）
//!   参考路径——T1 双路径互查的 oracle 面（eval 元循环求值器已
//!   退役：42-d P5，自举编译器 + VM 为唯一生产路径）。
//!
//! 错误统一为 `DriverError`：阶段标识 + 已渲染诊断（含位置摘录，
//! §2.2 原则 16：人类可感知输出是一等公民）。

use std::rc::Rc;

use kerf_compiler::{disassemble_program, BcProgram};
use kerf_core::{lower_program, CoreExpr, IrGraph};
use kerf_expander::{expand_program, phase::ModuleRegistry, ExpandCtxt, ExpandError};
use kerf_reader::{read_source, ReadError, TokenKind};
use kerf_runtime::Heap;
use kerf_span::{render_diagnostic, Diagnostic, DiagnosticCode, FileId, Severity, SourceMap, Span};
use kerf_syntax::{Keyword, Stx, Symbol, SymbolTable};
use kerf_vm::{run_program, Value, VmError};

use crate::builtins::{builtin_sigs, register_globals, resolve_hygiene_fallbacks, STDLIB_MODULES};
use crate::cache::{cache_enabled, cache_key, with_cache};
use crate::capability::{
    collect_takeover, hygienic_base, verify_io_capabilities, IoGrant, IoRequirements,
};

/// 管线阶段标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Read,
    Expand,
    Compile,
    Run,
}

impl Stage {
    fn label(self) -> &'static str {
        match self {
            Stage::Read => "read",
            Stage::Expand => "expand",
            Stage::Compile => "compile",
            Stage::Run => "run",
        }
    }
}

/// 驱动层错误（阶段 + 结构化诊断 + 已渲染文本）。
#[derive(Debug, Clone)]
pub struct DriverError {
    pub stage: Stage,
    pub diagnostic: Diagnostic,
    pub rendered: String,
}

impl std::fmt::Display for DriverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.stage.label(), self.rendered)
    }
}

impl DriverError {
    fn from_read(e: &ReadError, sm: &SourceMap) -> Self {
        let diag = Diagnostic::error(Some(DiagnosticCode(1)), e.message.clone(), e.span);
        DriverError {
            stage: Stage::Read,
            rendered: render_diagnostic(&diag, sm),
            diagnostic: diag,
        }
    }

    fn from_expand(e: &ExpandError, sm: &SourceMap) -> Self {
        let diag = Diagnostic::error(Some(DiagnosticCode(2)), e.message.clone(), e.span);
        DriverError {
            stage: Stage::Expand,
            rendered: render_diagnostic(&diag, sm),
            diagnostic: diag,
        }
    }

    fn from_compile(e: &kerf_compiler::CompileError, sm: &SourceMap) -> Self {
        let diag = Diagnostic::error(Some(DiagnosticCode(3)), e.message.clone(), e.span);
        DriverError {
            stage: Stage::Compile,
            rendered: render_diagnostic(&diag, sm),
            diagnostic: diag,
        }
    }

    fn from_vm(e: &VmError, sm: &SourceMap, table: &SymbolTable) -> Self {
        // E 码映射：VM 携码（r25/42-f 效应族 E0007-E0009）直用；
        // None = E0004 运行时通用族（既有口径）
        let mut diag = Diagnostic::error(
            Some(DiagnosticCode(e.code.unwrap_or(4))),
            e.message.clone(),
            e.span,
        );
        for t in &e.trace {
            diag = diag.with_child(Severity::Note, "调用点", t.span);
        }
        let _ = table;
        DriverError {
            stage: Stage::Run,
            rendered: render_diagnostic(&diag, sm),
            diagnostic: diag,
        }
    }
}

/// 编译产物（`compile_source` 输出——供 CLI dump 与 CodeValue 检查）。
pub struct CompileOutput {
    /// 展开后的核心表达式序列。
    pub core: Vec<Rc<CoreExpr>>,
    /// 图 IR（结构化形式）。
    pub ir: IrGraph,
    /// 字节码程序。
    pub program: BcProgram,
    /// 源映射（诊断渲染）。
    pub source_map: SourceMap,
    /// 符号表（渲染与调试）。
    pub table: SymbolTable,
    /// 模块注册簿记（declare/visit/instantiate）。
    pub registry: ModuleRegistry,
    /// W 级诊断（W1002 遮蔽族/W1003 宏名遮蔽族；非阻断观测面——r42/S1
    /// 后现状，W1001 已随旧名退役）。
    pub warnings: Vec<Diagnostic>,
}

/// 符号渲染（处理编译器哨兵：`u32::MAX-1` = main、`u32::MAX-2` = 匿名原型）。
fn resolve_symbol(s: Symbol, table: &SymbolTable) -> String {
    match s.0 {
        x if x == u32::MAX - 1 => "<main>".to_string(),
        x if x == u32::MAX - 2 => "<lambda>".to_string(),
        _ => table.name(s).to_string(),
    }
}

impl CompileOutput {
    /// 反汇编渲染。
    pub fn disassemble(&self) -> String {
        disassemble_program(&self.program, &|s: Symbol| resolve_symbol(s, &self.table))
    }

    /// 核心表达式渲染（每形式一行）。
    pub fn render_core(&self) -> String {
        self.core
            .iter()
            .map(|e| e.render(&|s: Symbol| resolve_symbol(s, &self.table)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 源文本（入口文件）。
    pub fn source(&self) -> &str {
        self.source_map.file(0).map(|f| &*f.src).unwrap_or("")
    }
}

/// 编译前段产物（read → expand → 相位簿记 → 字节码；**不含图 IR lowering**）。
///
/// TD-015 偿还：run 生产路径消费本形态——`IrGraph` 仅在消费方
/// （`kerf ir` 子命令 / CodeValue 检查 / 完整 dump）经 [`compile_source`]
/// 按需计算，生产执行路径不再旁路构造后丢弃（恒定开销消除）。
///
/// pub(crate)（B3）：自举 Reader 加载（bootstrap 模块）消费 program /
/// table / source_map 三字段——种子管线的产物形状。
///
/// Clone（批次 C）：编译缓存命中返回快照克隆（cache.rs 复用安全性
/// 前提 3——运行期状态不在缓存条目上变异）。
#[derive(Clone)]
pub(crate) struct FrontOutput {
    /// 展开后的核心表达式序列。
    pub(crate) core: Vec<Rc<CoreExpr>>,
    /// W 级诊断（W1002 遮蔽族/W1003 宏名遮蔽族；非阻断，编译成功
    /// 路径收集，CLI/CheckReport 观测面渲染）。
    pub(crate) warnings: Vec<Diagnostic>,
    /// 字节码程序。
    pub(crate) program: BcProgram,
    /// 源映射（诊断渲染）。
    pub(crate) source_map: SourceMap,
    /// 符号表（渲染与调试）。
    pub(crate) table: SymbolTable,
    /// 模块注册簿记（declare/visit 已完成）。
    pub(crate) registry: ModuleRegistry,
}

/// prelude 模块源码（编译期嵌入——产物自包含，与 reader/expander 同
/// 口径；TD-021：hofs 用户面注入载体）。
const PREAMBLE_SRC: &str = include_str!("bootstrap/preamble.krf");
/// prelude 源文件名（Span 诊断归属——独立 file，无源码拼接污染）。
const PREAMBLE_FILENAME: &str = "preamble.krf";
/// prelude 模块名（用户程序 (import kerf-prelude) 声明名——标识符字符
/// 集不含 '.'，取连字符形态）。
const PRELUDE_MODULE: &str = "kerf-prelude";

/// Stx 层 module 头 import 扫描（TD-021 注入触发判定：任一顶层 module
/// 形式声明 (import kerf-prelude)）。
fn imports_prelude(f: &Stx, table: &SymbolTable) -> bool {
    let items = match f.datum.as_list() {
        Some(l) if !l.is_empty() => l,
        // _ 臂理由：非列表/空列表不参与 import 扫描
        _ => return false,
    };
    let head = match items[0].datum.as_symbol() {
        Some(s) => s,
        None => return false,
    };
    if table.name(head) != "module" {
        return false;
    }
    // 可选头部区（import/export 依序、体前终止——镜像 expand_module 头部游走）
    for item in items.iter().skip(2) {
        let sub = match item.datum.as_list() {
            Some(l) if !l.is_empty() => l,
            // _ 臂理由：非列表头部项 = 体形式——头部区终止
            _ => break,
        };
        let h = match sub[0].datum.as_symbol() {
            Some(s) => s,
            None => break,
        };
        match table.name(h) {
            "import" => {
                for name in &sub[1..] {
                    if let Some(sym) = name.datum.as_symbol() {
                        if table.name(sym) == PRELUDE_MODULE {
                            return true;
                        }
                    }
                }
                continue;
            }
            "export" => continue,
            // _ 臂理由：头部区终止（体形式）
            _ => break,
        }
    }
    false
}

// ---------------------------------------------------------------------------
// 批次 M 首件 M1（v0.6 命名空间层，r39）——限定名编译期验证
// （22 §3.3 R-N3 不回落 E0014 + §7 保留域 E0015；front 全路径同 R9）
// ---------------------------------------------------------------------------

/// E0014：限定名不导出（R-N3 不回落——仅查模块 export 面）。
const QUALIFIED_NOT_EXPORTED_CODE: DiagnosticCode = DiagnosticCode(14);
/// E0015：保留域违例（kerf- 前缀用户占用——22 §7）。
const RESERVED_DOMAIN_CODE: DiagnosticCode = DiagnosticCode(15);
/// 保留域前缀（模块标识形——`kerf-`；22 §7 保留域）。
const RESERVED_PREFIX: &str = "kerf-";
/// 保留域许可名单（preamble 注入件 + 七模块标识形——M2 import 直用）。
const RESERVED_ALLOW: &[&str] = &[
    "kerf-prelude",
    "kerf-core",
    "kerf-pair",
    "kerf-list",
    "kerf-string",
    "kerf-symbol",
    "kerf-io",
    "kerf-char",
];

// ---------------------------------------------------------------------------
// 批次 M 次件 M2（v0.6 命名空间层，r40）——import 注入面/别名 +
// 组合闭包 + 位置纪律 + E0016 + W 弃用/遮蔽族（22 §8 M2 行；
// 20 §7 批次 M 行；21 §4.4；深审 D1-D9 裁定见 22 v1.3）
// ---------------------------------------------------------------------------

/// E0013：非限定 import 冲突（两模块同名导出——22 §6 冲突类一）。
const IMPORT_CONFLICT_CODE: DiagnosticCode = DiagnosticCode(13);
/// E0016：module 体内同名 define 重复定义（22 §3.2 R-N2 第四行）。
const MODULE_DUPLICATE_CODE: DiagnosticCode = DiagnosticCode(16);
/// E0017：别名重复（两 `as` 同名——22 §3.5 R-N5 冲突列）。
const ALIAS_DUPLICATE_CODE: DiagnosticCode = DiagnosticCode(17);
/// E0018：require 位置违例（合法位 = 顶层/module 体直接元素——
/// 深审 D3 裁定：fail-closed 位置纪律）。
const REQUIRE_POSITION_CODE: DiagnosticCode = DiagnosticCode(18);
/// E0019：import 未知模块（在册名单外——深审 D2 裁定：Clojure
/// require 同型，编译期拒绝）。
const UNKNOWN_IMPORT_CODE: DiagnosticCode = DiagnosticCode(19);
/// W1001（r40 引入——r42/S1 退役）：旧名弃用警告族。旧名 27 件已在
/// v0.9 移除（20 §7 移除轮行），旧名引用升 E0021 编译期错误——弃用
/// W 面消失合法（23 §3.4 生命周期四阶段：引入 v0.5 → 默认 v0.6 →
/// 弃用 v0.7 → 移除 v0.9 完整走完）。
/// W1002：N3 遮蔽 N2 注入名警告（22 §3.2 R-N2 第二行——可恢复但
/// 值得提示）。
const SHADOW_IMPORT_CODE: DiagnosticCode = DiagnosticCode(1002);
/// W1003：宏名遮蔽内置名警告（r42 / S1——22 §11 D11 排期移除轮
/// 同窗兑现：宏胜出是宏系统的本质能力[用户重定义语义合法场景]，
/// 但遮蔽**内置名**值得知会；意图不可判定[故意 vs 意外] → W 级
/// [知会非阻断]是唯一可判定位）。
const MACRO_SHADOW_CODE: DiagnosticCode = DiagnosticCode(1003);

// ---------------------------------------------------------------------------
// r41 / 62-a 语言形式深审轮——D10 裁定落地：E0020 保留字绑定禁令
//（22 §2.1 N4 层不变量「不是值、不可引用、不可遮蔽」的编译期执行）
// ---------------------------------------------------------------------------

/// E0020：N4 保留字不可绑定（25 关键字全域禁作绑定名——深审 D10：
/// 修复前 `(define if 5)` 合法且值位可读（N4 不变量被违反），操作位
/// 恒关键字形式——**用户绑定静默失效** = 「显式失败优于静默遮蔽」的
/// 反面形态（kerf-prelude/E0014/E0006 同族先例全反）。裁定 = 严格
/// 保留字（2026 跨范式共识：Rust/Swift 关键字全域禁作标识符 +
/// Rhombus v1.0 conventional syntax 同型）——绑定面（define/set!/
/// lambda 参数/let 系/define-syntax 宏名/module 名/import as 别名/
/// handle 双绑定器）编译期显式拒绝；quote 位 `'if` 数据符号不受
/// 影响（同像性维持——N0 符号宇宙层非 N4 语法层）。
const RESERVED_BINDING_CODE: DiagnosticCode = DiagnosticCode(20);

/// E0021：已移除旧名引用（r42 / S1 移除轮——20 §7「旧名引用 =
/// E00xx 错误」兑现）：v0.9 起旧名 27 件退役（REMOVED_BUILTIN_NAMES
/// 单源——旧名→现代名指引）。诊断携现代名指引（比裸 E0004 未绑定
/// 更 actionable：迁移期 DX 最优形态）；遮蔽/接管豁免与 E0014 同
/// 口径（N3 局部绑定胜出 + 用户接管合法——旧名退役的是**内置注册**
/// 面，非符号宇宙层）。
const REMOVED_NAME_CODE: DiagnosticCode = DiagnosticCode(21);

/// 保留字判定（单源 = kerf-syntax `Keyword::from_name` 25 名查表——
/// 与 Reader/expander 消费同一表，零名单漂移面）。
fn is_reserved_word(name: &str) -> bool {
    Keyword::from_name(name).is_some()
}

/// E0020 诊断构造（Stx/CoreExpr 两层共用文案——D10 裁定依据随诊断
/// 携带：22 §2.1 不变量 + 静默失效陷阱说明）。
fn reserved_binding_diag(name: &str, span: Span, kind: &str) -> Diagnostic {
    Diagnostic::error(
        Some(RESERVED_BINDING_CODE),
        format!(
            "保留字不可绑定：「{}」是 N4 关键字（{}）——关键字不是值、不可引用、不可遮蔽（22 §2.1）；\
             同名绑定在操作位将被关键字形式静默覆盖，故编译期显式拒绝（E0020，深审 D10）",
            name, kind
        ),
        span,
    )
}

/// Stx 层保留字绑定检查（**源码面**——expand 前拦截，覆盖全部绑定
/// 形式 + define-syntax 宏名[展开后无 CoreExpr 承载] + import as
/// 别名[别名归一在 rewrite 消费]；递归穿透 begin/module 体/嵌套
/// lambda/let 等全部子树）。
#[allow(clippy::result_large_err)]
fn verify_reserved_bindings_stx(
    forms: &[Stx],
    table: &SymbolTable,
    sm: &SourceMap,
) -> Result<(), DriverError> {
    for f in forms {
        if let Some(diag) = reserved_stx_diag(f, table) {
            return Err(face_err(diag, sm));
        }
    }
    Ok(())
}

/// Stx 单节点绑定面检查（列表头分派 + 子树递归；非列表原子无绑定面）。
fn reserved_stx_diag(f: &Stx, table: &SymbolTable) -> Option<Diagnostic> {
    let items = match f.datum.as_list() {
        Some(l) if !l.is_empty() => l,
        // _ 臂理由：原子/空列表无绑定形式
        _ => return None,
    };
    let head = match items[0].datum.as_symbol() {
        Some(s) => table.name(s),
        None => "",
    };
    // 单名绑定族：define / set! / define-syntax / module
    match head {
        "define" | "set!" | "module" => {
            if items.len() >= 2 {
                if let Some(n) = items[1].datum.as_symbol() {
                    let name = table.name(n);
                    if is_reserved_word(name) {
                        let kind = match head {
                            "module" => "模块名",
                            "set!" => "赋值目标",
                            _ => "绑定名",
                        };
                        return Some(reserved_binding_diag(name, items[1].span, kind));
                    }
                }
            }
        }
        // define-syntax：只查宏名（items[1]）——**模板子树是数据域**
        //（syntax-rules 的模式/模板是宏的字面量数据非程序绑定——Stx
        // 结构游走不区分模板/代码，递归进入会误报[模板里合法出现
        // `(define quote …)` 形态字面量]；真正实例化的展开产物由
        // CoreExpr 层 E0020 拦截——防御纵深两层各管其域）
        "define-syntax" => {
            if items.len() >= 2 {
                if let Some(n) = items[1].datum.as_symbol() {
                    let name = table.name(n);
                    if is_reserved_word(name) {
                        return Some(reserved_binding_diag(name, items[1].span, "宏名"));
                    }
                }
            }
            return None;
        }
        // lambda 参数表（各参数名均为绑定器）
        "lambda" => {
            if items.len() >= 2 {
                if let Some(params) = items[1].datum.as_list() {
                    for p in params {
                        if let Some(n) = p.datum.as_symbol() {
                            let name = table.name(n);
                            if is_reserved_word(name) {
                                return Some(reserved_binding_diag(name, p.span, "lambda 参数"));
                            }
                        }
                    }
                }
            }
        }
        // let 系绑定对（对首为绑定名）
        "let" | "letrec" | "let*" => {
            if items.len() >= 2 {
                if let Some(bindings) = items[1].datum.as_list() {
                    for b in bindings {
                        if let Some(pair) = b.datum.as_list() {
                            if !pair.is_empty() {
                                if let Some(n) = pair[0].datum.as_symbol() {
                                    let name = table.name(n);
                                    if is_reserved_word(name) {
                                        return Some(reserved_binding_diag(
                                            name,
                                            pair[0].span,
                                            "let 绑定名",
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // import as 别名（contextual 标记后的符号是局部绑定——
        // 22 §3.5 R-N5；别名归一在 rewrite_alias_refs 消费，无 CoreExpr
        // 承载，故 Stx 层拦截）
        "import" => {
            let mut i = 1;
            while i < items.len() {
                if let Some(s) = items[i].datum.as_symbol() {
                    if table.name(s) == "as" && i + 1 < items.len() {
                        if let Some(a) = items[i + 1].datum.as_symbol() {
                            let name = table.name(a);
                            if is_reserved_word(name) {
                                return Some(reserved_binding_diag(
                                    name,
                                    items[i + 1].span,
                                    "import 别名",
                                ));
                            }
                        }
                    }
                }
                i += 1;
            }
        }
        _ => {}
    }
    // 子树递归（begin 体 / module 体 / lambda 体 / let 体 / if 分支 /
    // 被绑值子树等——全部穿透）
    for item in items {
        if let Some(d) = reserved_stx_diag(item, table) {
            return Some(d);
        }
    }
    None
}

/// CoreExpr 层保留字绑定检查（**宏展开产物面**——源码层 Stx 检查的
/// 防御纵深：宏模板生成的绑定名同样过 E0020；let 系脱糖为 Lambda+
/// App 后绑定名收敛于 Lambda.params；handle 双绑定器在展开产物
/// 结构化承载）。
#[allow(clippy::result_large_err)]
fn verify_reserved_bindings_core(
    core: &[Rc<CoreExpr>],
    table: &SymbolTable,
    sm: &SourceMap,
) -> Result<(), DriverError> {
    for e in core {
        if let Some(diag) = reserved_core_diag(e, table) {
            return Err(face_err(diag, sm));
        }
    }
    Ok(())
}

/// CoreExpr 单节点绑定面检查（Define/Lambda/SetBang/Module/Handle
/// 五绑定器变体 + 全子树递归）。
fn reserved_core_diag(e: &CoreExpr, table: &SymbolTable) -> Option<Diagnostic> {
    match e {
        CoreExpr::Define { name, span, .. } => {
            let n = table.name(*name);
            if is_reserved_word(n) {
                return Some(reserved_binding_diag(n, *span, "绑定名（展开产物）"));
            }
            None
        }
        CoreExpr::Lambda {
            params, body, span, ..
        } => {
            for p in params {
                let n = table.name(*p);
                if is_reserved_word(n) {
                    return Some(reserved_binding_diag(n, *span, "lambda 参数（展开产物）"));
                }
            }
            reserved_core_diag(body, table)
        }
        CoreExpr::SetBang { name, span, .. } => {
            let n = table.name(*name);
            if is_reserved_word(n) {
                return Some(reserved_binding_diag(n, *span, "赋值目标（展开产物）"));
            }
            None
        }
        CoreExpr::Module {
            name, body, span, ..
        } => {
            let n = table.name(*name);
            if is_reserved_word(n) {
                return Some(reserved_binding_diag(n, *span, "模块名（展开产物）"));
            }
            body.iter().find_map(|i| reserved_core_diag(i, table))
        }
        CoreExpr::Handle {
            payload_var,
            resume_var,
            handler_body,
            body,
            span,
            ..
        } => {
            for (n, kind) in [
                (table.name(*payload_var), "handle 载荷绑定"),
                (table.name(*resume_var), "handle 恢复绑定"),
            ] {
                if is_reserved_word(n) {
                    return Some(reserved_binding_diag(n, *span, kind));
                }
            }
            reserved_core_diag(handler_body, table).or_else(|| reserved_core_diag(body, table))
        }
        CoreExpr::Begin { body, .. } => body.iter().find_map(|i| reserved_core_diag(i, table)),
        CoreExpr::App { fn_expr, args, .. } => reserved_core_diag(fn_expr, table)
            .or_else(|| args.iter().find_map(|a| reserved_core_diag(a, table))),
        CoreExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => reserved_core_diag(cond, table)
            .or_else(|| reserved_core_diag(then_branch, table))
            .or_else(|| reserved_core_diag(else_branch, table)),
        CoreExpr::Perform { effect, .. } => reserved_core_diag(effect, table),
        // VarRef/Literal/Require 无绑定面（引用与声明非绑定——值位引用
        // 保留字在绑定禁令生效后恒走未绑定路径，fail-closed 天然维持）
        _ => None,
    }
}

/// import 面收集结果（Stx 层——与 imports_prelude 同型头部区游走；
/// 22 §3.5 R-N5 三形态：非限定导入/限定别名/隐式门控注入）。
#[derive(Debug, Default)]
pub(crate) struct ImportFace {
    /// 非限定导入模块的 ns 段集（`kerf-string` → `string`——注入
    /// export 面；过渡期全局扁平名仍在，注入可观测性 = 冲突/
    /// 遮蔽/闭包三面——深审 D7 设计知悉项）。
    pub(crate) unqualified: Vec<String>,
    /// 限定别名映射（别名 → ns 段——`(import kerf-string as str)` →
    /// `str` → `string`；别名是局部绑定 N3 语义位——22 §3.5）。
    pub(crate) aliases: Vec<(String, String)>,
    /// 非限定注入面（各导入模块 export 本地名的并集——E0013 冲突
    /// 检测与 W1002 遮蔽检测的数据面）。
    pub(crate) injected_names: Vec<String>,
}

/// Stx 层 import 面收集（module 头部区游走——镜像 expand_module
/// 头部解析；`as` 为 contextual 标记非 N4 关键字——深审 D1 裁定：
/// 02 词法架构零改动，Rust `use as`/Python `import as` 2026 惯例同型）。
///
/// 形态：`(import kerf-string)`（非限定）｜`(import kerf-string as str)`
///（限定别名）｜混列 `(import a as x b)`（别名对 + 非限定并存）。
/// `as` 在 import 列表内恒为别名标记（contextual 语义的边界代价——
/// 用户模块名不可在 import 列表内叫 `as`，限定名引用不受影响）。
/// r42 / S1 移除轮——W1003 宏名收集（22 §11 D11：define-syntax 宏名
/// 与内置函数同名 → W 级知会）。Stx 层唯一承载面：宏展开后名字从
/// CoreExpr 消失，此面是 W1003 的唯一数据源。递归列表臂覆盖顶层 +
/// module 体（嵌套列表中模板字面量 `(define-syntax …)` 假阳性可接受
/// ——W 级非阻断 + 引导语料零碰撞[reader/expander 自举件不经本管线]）。
fn collect_macro_names(forms: &[Stx], table: &SymbolTable) -> Vec<(String, Span)> {
    let mut out: Vec<(String, Span)> = Vec::new();
    for f in forms {
        if let Some(items) = f.datum.as_list() {
            if items.len() >= 2 {
                let is_defsyntax = items[0]
                    .datum
                    .as_symbol()
                    .map(|s| table.name(s) == "define-syntax")
                    .unwrap_or(false);
                if is_defsyntax {
                    if let Some(name_sym) = items[1].datum.as_symbol() {
                        out.push((table.name(name_sym).to_string(), f.span));
                    }
                }
            }
            out.extend(collect_macro_names(items, table));
        }
    }
    out
}

fn collect_import_face(forms: &[Stx], table: &SymbolTable) -> ImportFace {
    let mut face = ImportFace::default();
    for f in forms {
        let items = match f.datum.as_list() {
            Some(l) if !l.is_empty() => l,
            // _ 臂理由：非列表/空列表不参与 import 扫描
            _ => continue,
        };
        let head = match items[0].datum.as_symbol() {
            Some(s) => s,
            None => continue,
        };
        if table.name(head) != "module" {
            continue;
        }
        // 头部区游走（import/export 依序、体前终止）
        for item in items.iter().skip(2) {
            let sub = match item.datum.as_list() {
                Some(l) if !l.is_empty() => l,
                // _ 臂理由：非列表头部项 = 体形式——头部区终止
                _ => break,
            };
            let h = match sub[0].datum.as_symbol() {
                Some(s) => s,
                None => break,
            };
            if table.name(h) != "import" {
                // export 头与体形式：头部区游走终止条件（import 面
                // 只在连续 import/export 头内收集）
                if table.name(h) == "export" {
                    continue;
                }
                break;
            }
            // import 项序列解析（前瞻 as 形态感知——`(模块名 as 别名)`
            // 是别名限定导入：只登记别名对，**不注入非限定名**[22 §3.5
            // 别名是局部绑定的限定引用形态；as 形态模块入 unqualified
            // 会与同 export 名的另一非限定导入误报 E0013——前瞻解析
            // 从源头分立两形态]）
            let mut i = 1;
            while i < sub.len() {
                let sym = match sub[i].datum.as_symbol() {
                    Some(s) => s,
                    // _ 臂理由：非符号项交给 E0019 名单校验阶段报错
                    None => {
                        i += 1;
                        continue;
                    }
                };
                let name = table.name(sym).to_string();
                // 前瞻两符号：次位 as + 三位别名 → 别名对登记跳三位
                if i + 2 < sub.len() {
                    let next_is_as = sub[i + 1]
                        .datum
                        .as_symbol()
                        .map(|s| table.name(s) == "as")
                        .unwrap_or(false);
                    if next_is_as {
                        if let Some(alias_sym) = sub[i + 2].datum.as_symbol() {
                            face.aliases
                                .push((table.name(alias_sym).to_string(), import_target_ns(&name)));
                        }
                        i += 3;
                        continue;
                    }
                }
                // 非限定导入：注入 export 面（E0013 冲突检测面）
                face.unqualified.push(import_target_ns(&name));
                i += 1;
            }
        }
    }
    // 注入面派生：各导入模块 export 本地名并集（STDLIB_MODULES 单源）
    for ns in &face.unqualified {
        for &(m, local, _) in STDLIB_MODULES {
            if m == ns {
                face.injected_names.push(local.to_string());
            }
        }
    }
    face
}

/// import 目标名 → ns 段归一（`kerf-string` → `string`；`kerf-prelude`
/// 无导出面——零注入，仅闭包/名单核对）。
fn import_target_ns(name: &str) -> String {
    name.strip_prefix("kerf-").unwrap_or(name).to_string()
}

/// import 面验证（E0013 冲突 / E0017 别名重复 / E0019 未知模块——
/// 22 §5.4/§6 + 深审 D2 裁定；fail-closed 首违阻断，与 R9 同口径）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同入口约定
fn verify_import_face(
    face: &ImportFace,
    table: &SymbolTable,
    sm: &SourceMap,
    import_spans: &[(String, Span)],
) -> Result<(), DriverError> {
    // E0019 未知模块：全部 import 目标 ∈ 在册名单（七模块 + prelude）
    for (name, span) in import_spans {
        if !RESERVED_ALLOW.contains(&name.as_str()) {
            let diag = Diagnostic::error(
                Some(UNKNOWN_IMPORT_CODE),
                format!(
                    "未知导入模块「{}」（在册名单：{}——import 仅接标准库模块，用户模块间导入属 Stage 3 编译单元窗口）",
                    name,
                    RESERVED_ALLOW.join(" / ")
                ),
                *span,
            );
            return Err(face_err(diag, sm));
        }
    }
    // E0013 非限定冲突：两导入模块 export 面同名 → 显式错误
    // （22 §6 冲突类一：逃生阀 = 限定导入 as 别名）
    for ia in 0..face.unqualified.len() {
        for ib in (ia + 1)..face.unqualified.len() {
            let (ma, mb) = (&face.unqualified[ia], &face.unqualified[ib]);
            if ma == mb || ma == "prelude" || mb == "prelude" {
                continue;
            }
            let mut clashes: Vec<&str> = Vec::new();
            for &(nsa, la, _) in STDLIB_MODULES {
                if nsa != ma {
                    continue;
                }
                for &(nsb, lb, _) in STDLIB_MODULES {
                    if nsb == mb && la == lb {
                        clashes.push(la);
                    }
                }
            }
            if let Some(local) = clashes.first() {
                let diag = Diagnostic::error(
                    Some(IMPORT_CONFLICT_CODE),
                    format!(
                        "import 冲突：「{}」来自 kerf-{} 与 kerf-{}（两模块同名导出——显式错误非静默遮蔽；逃生阀 = 限定导入 (import kerf-{} as 别名)）",
                        local, ma, mb, ma
                    ),
                    // 定位：import 面首个 span（头部区）——精确度受
                    // collect 时机限制，诊断消息已携带双方模块名
                    import_spans
                        .first()
                        .map(|(_, s)| *s)
                        .unwrap_or_else(Span::dummy),
                );
                return Err(face_err(diag, sm));
            }
        }
    }
    // E0017 别名重复：两 as 同名别名（22 §3.5：别名重复定义错误）
    let mut seen: std::collections::HashSet<&String> = std::collections::HashSet::new();
    for (alias, _) in &face.aliases {
        if !seen.insert(alias) {
            let diag = Diagnostic::error(
                Some(ALIAS_DUPLICATE_CODE),
                format!(
                    "别名重复：「{}」被多次绑定（别名是模块体内局部绑定——同名重复属重复定义错误；22 §3.5 R-N5）",
                    alias
                ),
                import_spans
                    .first()
                    .map(|(_, s)| *s)
                    .unwrap_or_else(Span::dummy),
            );
            return Err(face_err(diag, sm));
        }
    }
    let _ = (table, sm); // （本函数当前仅消费 spans/名单——参数面对齐保留）
    Ok(())
}

/// import 面错误包装（DriverError 构造助手）。
fn face_err(diag: Diagnostic, sm: &SourceMap) -> DriverError {
    // Compile stage（与 M1 verify_qualified_refs 先例一致：expand 后的
    // front 管线编译期验证族——语义归属验证非展开）
    DriverError {
        stage: Stage::Compile,
        rendered: render_diagnostic(&diag, sm),
        diagnostic: diag,
    }
}

/// import 目标收集（名 + Span——E0019 定位用；与 collect_import_face
/// 同游走但保留原始目标名——kerf- 形）。
fn collect_import_targets(forms: &[Stx], table: &SymbolTable) -> Vec<(String, Span)> {
    let mut out = Vec::new();
    for f in forms {
        let items = match f.datum.as_list() {
            Some(l) if !l.is_empty() => l,
            _ => continue,
        };
        let head = match items[0].datum.as_symbol() {
            Some(s) => s,
            None => continue,
        };
        if table.name(head) != "module" {
            continue;
        }
        for item in items.iter().skip(2) {
            let sub = match item.datum.as_list() {
                Some(l) if !l.is_empty() => l,
                _ => break,
            };
            let h = match sub[0].datum.as_symbol() {
                Some(s) => s,
                None => break,
            };
            match table.name(h) {
                "import" => {
                    let mut i = 1;
                    while i < sub.len() {
                        if let Some(sym) = sub[i].datum.as_symbol() {
                            let name = table.name(sym).to_string();
                            if name == "as" {
                                i += 2;
                                continue;
                            }
                            out.push((name, sub[i].span));
                        }
                        i += 1;
                    }
                    continue;
                }
                "export" => continue,
                _ => break,
            }
        }
    }
    out
}

/// E0016：module 体内同名 define 重复定义检测（22 §3.2 R-N2 第四行
/// 及 09 v6.2 既有裁定「显式报重复定义」的 M2 落位：每 module 直接
/// `body` 的 Define 名面查重，嵌套 module 各自独立检测）。
#[allow(clippy::result_large_err)]
fn verify_module_define_duplicates(
    core: &[Rc<CoreExpr>],
    table: &SymbolTable,
    sm: &SourceMap,
) -> Result<(), DriverError> {
    for e in core {
        if let Some(diag) = module_dup_diag(e, table) {
            return Err(face_err(diag, sm));
        }
    }
    Ok(())
}

fn module_dup_diag(e: &CoreExpr, table: &SymbolTable) -> Option<Diagnostic> {
    match e {
        CoreExpr::Module {
            name, body, span, ..
        } => {
            let module_name = table.name(*name).to_string();
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            for item in body {
                if let CoreExpr::Define {
                    name: d,
                    span: dspan,
                    ..
                } = item.as_ref()
                {
                    let dn = table.name(*d).to_string();
                    if !seen.insert(dn.clone()) {
                        return Some(Diagnostic::error(
                            Some(MODULE_DUPLICATE_CODE),
                            format!(
                                "重复定义：模块「{}」内 define「{}」重复（09 v6.2 既有裁定——显式报重复定义非静默遮蔽）",
                                module_name, dn
                            ),
                            *dspan,
                        ));
                    }
                }
                if let Some(d) = module_dup_diag(item, table) {
                    return Some(d);
                }
            }
            let _ = span;
            None
        }
        CoreExpr::Begin { body, .. } => body.iter().find_map(|i| module_dup_diag(i, table)),
        _ => None,
    }
}

/// E0018：require 位置纪律（深审 D3 裁定：合法位 = 程序顶层序列直接
/// 元素 + module 体直接元素；表达式子树内（lambda/let 脱糖后 Lambda/
/// If/App/Begin 嵌套层）出现 = 位置错误——fail-closed 位置纪律，
/// Clojure require 限定 ns 顶层同型）。
#[allow(clippy::result_large_err)]
fn verify_require_positions(
    core: &[Rc<CoreExpr>],
    _table: &SymbolTable,
    sm: &SourceMap,
) -> Result<(), DriverError> {
    for e in core {
        if let Some(diag) = require_pos_diag(e, true) {
            return Err(face_err(diag, sm));
        }
    }
    Ok(())
}

/// require 位置递归（top 位 = 序列直接元素合法；容器子树内非法）。
fn require_pos_diag(e: &CoreExpr, top: bool) -> Option<Diagnostic> {
    match e {
        CoreExpr::Require { span, .. } => {
            if top {
                None
            } else {
                Some(Diagnostic::error(
                    Some(REQUIRE_POSITION_CODE),
                    "require 位置违例：require 合法位 = 程序顶层或 module 体直接元素（当前位置为表达式子树内——require 是声明非表达式；深审 D3 裁定".to_string(),
                    *span,
                ))
            }
        }
        CoreExpr::Module { body, .. } => body.iter().find_map(|item| require_pos_diag(item, true)),
        CoreExpr::Lambda { body, .. } => require_pos_diag(body, false),
        CoreExpr::App { fn_expr, args, .. } => require_pos_diag(fn_expr, false)
            .or_else(|| args.iter().find_map(|a| require_pos_diag(a, false))),
        CoreExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => require_pos_diag(cond, false)
            .or_else(|| require_pos_diag(then_branch, false))
            .or_else(|| require_pos_diag(else_branch, false)),
        CoreExpr::SetBang { value, .. } | CoreExpr::Define { value, .. } => {
            require_pos_diag(value, false)
        }
        CoreExpr::Begin { body, .. } => {
            if top {
                // 顶层/module 体 Begin：序列形态内的 require 视同合法
                // （begin 是序形式非表达式嵌套——零误报优先裁定）
                body.iter().find_map(|i| require_pos_diag(i, true))
            } else {
                body.iter().find_map(|i| require_pos_diag(i, false))
            }
        }
        CoreExpr::Perform { effect, .. } => require_pos_diag(effect, false),
        CoreExpr::Handle {
            handler_body, body, ..
        } => require_pos_diag(handler_body, false).or_else(|| require_pos_diag(body, false)),
        CoreExpr::VarRef { .. } | CoreExpr::Literal { .. } => None,
    }
}

/// W 警告收集（W1002 遮蔽注入名族 + W1003 宏名遮蔽内置名族——非阻断；
/// 22 §3.2 R-N2 第二行 + 22 §11 D11）。每名去重首现一条（诊断聚合）。
/// r42/S1：W1001 弃用族退役（旧名升 E0021 编译期阻断——W 面收窄）。
fn collect_warnings(
    core: &[Rc<CoreExpr>],
    table: &SymbolTable,
    face: &ImportFace,
    macro_defs: &[(String, Span)],
) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();
    let mut shadow_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    // r42 / S1 移除轮：W1001 弃用族退役（旧名 27 件升 E0021 编译期
    // 错误——W 级面消失合法；W1002 遮蔽族维持）。W1003 宏名遮蔽
    // 落地（22 §11 D11 排期本窗兑现）。
    let mut macro_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (name, span) in macro_defs {
        if !macro_seen.insert(name.clone()) {
            continue; // 每名一条去重（诊断聚合）
        }
        if crate::builtins::builtin_name_exists(name) {
            out.push(Diagnostic::warning(
                Some(MACRO_SHADOW_CODE),
                format!(
                    "宏名「{}」遮蔽了内置名（W1003——宏胜出是本质能力，此处仅知会；若非故意请改名）",
                    name
                ),
                *span,
            ));
        }
    }
    for e in core {
        warn_diag(e, table, face, &mut shadow_seen, &mut out);
    }
    out
}

fn warn_diag(
    e: &CoreExpr,
    table: &SymbolTable,
    face: &ImportFace,
    shadow_seen: &mut std::collections::HashSet<String>,
    out: &mut Vec<Diagnostic>,
) {
    match e {
        CoreExpr::VarRef { .. } => {
            // r42/S1：W1001 弃用族退役——VarRef 无 W 职责（旧名已升
            // E0021 编译期阻断于 verify_qualified_refs，fail-closed）
        }
        CoreExpr::Module { name, body, .. } => {
            let module_name = table.name(*name).to_string();
            // preamble 结构性豁免（20 §6.5 引导语料纪律：prelude 模块
            // 体内部不属用户 W 面——展开后 Span 的 file 传递经自举桥重建
            // 不可依赖，按模块名结构豁免防 W1002 注入遮蔽误报；r42/S1
            // 后 preamble 已迁移现代名，豁免保持为零误报保险）
            if module_name == PRELUDE_MODULE {
                return;
            }
            // W1002 define 遮蔽注入名（R-N2 第二行：N3 遮蔽 N2 注入名
            // = 合法 + W 级警告——多半是命名事故，可恢复但值得提示）
            for item in body {
                if let CoreExpr::Define { name: d, span, .. } = item.as_ref() {
                    let dn = hygienic_base(table.name(*d));
                    if face.injected_names.contains(&dn.to_string())
                        && shadow_seen.insert(dn.to_string())
                    {
                        out.push(Diagnostic::warning(
                            Some(SHADOW_IMPORT_CODE),
                            format!(
                                "模块「{}」define「{}」遮蔽了 import 注入名（R-N2：合法但多半是命名事故——改名或改用限定名可消除歧义）",
                                module_name, dn
                            ),
                            *span,
                        ));
                    }
                }
                warn_diag(item, table, face, shadow_seen, out);
            }
        }
        CoreExpr::Lambda { params, body, .. } => {
            // 参数遮蔽注入名：R-N2 第二行同型警告（内层胜合法）
            for p in params {
                let pn = hygienic_base(table.name(*p));
                if face.injected_names.contains(&pn.to_string())
                    && shadow_seen.insert(pn.to_string())
                {
                    out.push(Diagnostic::warning(
                        Some(SHADOW_IMPORT_CODE),
                        format!(
                            "lambda 参数「{}」遮蔽了 import 注入名（R-N2：合法但多半是命名事故）",
                            pn
                        ),
                        e.span(),
                    ));
                }
            }
            warn_diag(body, table, face, shadow_seen, out);
        }
        CoreExpr::App { fn_expr, args, .. } => {
            warn_diag(fn_expr, table, face, shadow_seen, out);
            for a in args {
                warn_diag(a, table, face, shadow_seen, out);
            }
        }
        CoreExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            warn_diag(cond, table, face, shadow_seen, out);
            warn_diag(then_branch, table, face, shadow_seen, out);
            warn_diag(else_branch, table, face, shadow_seen, out);
        }
        CoreExpr::SetBang { value, .. } | CoreExpr::Define { value, .. } => {
            warn_diag(value, table, face, shadow_seen, out)
        }
        CoreExpr::Begin { body, .. } => {
            for item in body {
                warn_diag(item, table, face, shadow_seen, out);
            }
        }
        CoreExpr::Perform { effect, .. } => warn_diag(effect, table, face, shadow_seen, out),
        CoreExpr::Handle {
            handler_body, body, ..
        } => {
            warn_diag(handler_body, table, face, shadow_seen, out);
            warn_diag(body, table, face, shadow_seen, out);
        }
        CoreExpr::Require { .. } | CoreExpr::Literal { .. } => {}
    }
}

/// M2 别名限定名编译期归一（22 §3.5 R-N5「别名是局部绑定」的机制面
/// 实现：`(import kerf-string as str)` 后 `str/append` 的 VarRef 重写为
/// `string/append`——HM 签名/字节码编译/运行面全链消费归一名，零运行
/// 时别名感知；T1 双路径共享 front_from_core 段天然 parity）。
/// 别名集为空时原样返回（零重写开销——无别名程序零成本）。
fn rewrite_alias_refs(
    core: Vec<Rc<CoreExpr>>,
    face: &ImportFace,
    table: &mut SymbolTable,
) -> Vec<Rc<CoreExpr>> {
    if face.aliases.is_empty() {
        return core;
    }
    core.into_iter()
        .map(|e| rewrite_alias_expr(e, face, table))
        .collect()
}

fn rewrite_alias_expr(e: Rc<CoreExpr>, face: &ImportFace, table: &mut SymbolTable) -> Rc<CoreExpr> {
    match e.as_ref() {
        CoreExpr::VarRef { name, scopes, span } => {
            let raw = table.name(*name);
            let base = hygienic_base(raw);
            if let Some((ns, local)) = base.split_once('/') {
                if let Some((_, module)) = face.aliases.iter().find(|(a, _)| a == ns) {
                    let new_name = format!("{}/{}", module, local);
                    return Rc::new(CoreExpr::VarRef {
                        name: table.intern(&new_name),
                        scopes: scopes.clone(),
                        span: *span,
                    });
                }
            }
            Rc::new(e.as_ref().clone())
        }
        CoreExpr::Lambda {
            params,
            param_scopes,
            body,
            span,
        } => Rc::new(CoreExpr::Lambda {
            params: params.clone(),
            param_scopes: param_scopes.clone(),
            body: rewrite_alias_expr(Rc::clone(body), face, table),
            span: *span,
        }),
        CoreExpr::App {
            fn_expr,
            args,
            span,
        } => Rc::new(CoreExpr::App {
            fn_expr: rewrite_alias_expr(Rc::clone(fn_expr), face, table),
            args: args
                .iter()
                .map(|a| rewrite_alias_expr(Rc::clone(a), face, table))
                .collect(),
            span: *span,
        }),
        CoreExpr::If {
            cond,
            then_branch,
            else_branch,
            span,
        } => Rc::new(CoreExpr::If {
            cond: rewrite_alias_expr(Rc::clone(cond), face, table),
            then_branch: rewrite_alias_expr(Rc::clone(then_branch), face, table),
            else_branch: rewrite_alias_expr(Rc::clone(else_branch), face, table),
            span: *span,
        }),
        CoreExpr::SetBang {
            name,
            scopes,
            value,
            span,
        } => Rc::new(CoreExpr::SetBang {
            name: *name,
            scopes: scopes.clone(),
            value: rewrite_alias_expr(Rc::clone(value), face, table),
            span: *span,
        }),
        CoreExpr::Define { name, value, span } => Rc::new(CoreExpr::Define {
            name: *name,
            value: rewrite_alias_expr(Rc::clone(value), face, table),
            span: *span,
        }),
        CoreExpr::Begin { body, span } => Rc::new(CoreExpr::Begin {
            body: body
                .iter()
                .map(|i| rewrite_alias_expr(Rc::clone(i), face, table))
                .collect(),
            span: *span,
        }),
        CoreExpr::Module {
            name,
            imports,
            exports,
            body,
            span,
        } => Rc::new(CoreExpr::Module {
            name: *name,
            imports: imports.clone(),
            exports: exports.clone(),
            body: body
                .iter()
                .map(|i| rewrite_alias_expr(Rc::clone(i), face, table))
                .collect(),
            span: *span,
        }),
        CoreExpr::Require { .. } | CoreExpr::Literal { .. } => Rc::clone(&e),
        CoreExpr::Perform { effect, span } => Rc::new(CoreExpr::Perform {
            effect: rewrite_alias_expr(Rc::clone(effect), face, table),
            span: *span,
        }),
        CoreExpr::Handle {
            tag,
            payload_var,
            payload_scopes,
            resume_var,
            resume_scopes,
            handler_body,
            body,
            span,
        } => Rc::new(CoreExpr::Handle {
            tag: Rc::clone(tag),
            payload_var: *payload_var,
            payload_scopes: payload_scopes.clone(),
            resume_var: *resume_var,
            resume_scopes: resume_scopes.clone(),
            handler_body: rewrite_alias_expr(Rc::clone(handler_body), face, table),
            body: rewrite_alias_expr(Rc::clone(body), face, table),
            span: *span,
        }),
    }
}

/// 限定名引用验证（R-N3 不回落——22 §8 表 R-N3 行）：含 `/` 的
/// VarRef 名仅查 stdlib 七模块 export 面 + 程序接管豁免（define/set!
/// 同名——零误报纪律与 R9 同口径）；未命中 → E0014「不导出」而非
/// 「未绑定」（诊断增益：打错模块名精确指向模块面）。R-N8 豁免天然
/// 成立（本验证只走 VarRef 引用位——quote 符号值是 Literal，不参与）。
/// 保留域（E0015）：module 名 kerf- 前缀占用（用户模块去前缀）。
/// M1 收窄如实注记：用户模块 export 限定名引用归 M2 import 面承载。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同入口约定
fn verify_qualified_refs(
    core: &[Rc<CoreExpr>],
    face: &ImportFace,
    table: &SymbolTable,
    sm: &SourceMap,
) -> Result<(), DriverError> {
    let mut takeover: std::collections::HashSet<String> = std::collections::HashSet::new();
    for e in core {
        collect_takeover(e, table, &mut takeover);
    }
    let shadowed: std::collections::HashSet<String> = std::collections::HashSet::new();
    for e in core {
        if let Some(diag) = qualified_ref_diag(e, &takeover, &shadowed, face, table) {
            return Err(DriverError {
                stage: Stage::Compile,
                rendered: render_diagnostic(&diag, sm),
                diagnostic: diag,
            });
        }
    }
    Ok(())
}

/// 限定名/保留域诊断遍历（首个违例即返——fail-closed 单条阻断，
/// 多错误收集属 E0005 家族职责，同 R9 口径）。
///
/// R-N1 解析序（22 §3.1）：N3 局部绑定先于 N2——**Lambda 参数遮蔽
/// 限定名检查**（`(lambda (foo/bar) foo/bar)` 是局部名非限定引用）；
/// 遮蔽集随递归传入（Lambda 入并集——不可变传播，词法嵌套正确性）。
fn qualified_ref_diag(
    e: &CoreExpr,
    takeover: &std::collections::HashSet<String>,
    shadowed: &std::collections::HashSet<String>,
    face: &ImportFace,
    table: &SymbolTable,
) -> Option<Diagnostic> {
    match e {
        CoreExpr::VarRef { name, span, .. } => {
            let raw = table.name(*name);
            let base = hygienic_base(raw);
            if shadowed.contains(base) || shadowed.contains(raw) {
                return None; // N3 局部绑定胜出（R-N1——参数遮蔽合法）
            }
            // r42 / S1 移除轮——E0021 已移除旧名（值位/操作位全域；与
            // E0014 同 traversal——遮蔽已豁免[上]，接管豁免[下]同口径：
            // 用户 define/set! 旧名 = 用户全局合法[退役的是内置注册面]）
            if !takeover.contains(raw) && !takeover.contains(base) {
                if let Some(&(_, modern)) = crate::builtins::REMOVED_BUILTIN_NAMES
                    .iter()
                    .find(|&&(old, _)| old == base)
                {
                    return Some(Diagnostic::error(
                        Some(REMOVED_NAME_CODE),
                        format!(
                            "「{}」已于 v0.9 移除——现代名「{}」（20 §7 移除轮；限定名形 kerf 域亦可用）",
                            base, modern
                        ),
                        *span,
                    ));
                }
            }
            // 独立 `/`（除法运算符）与不含 `/` 的名字不参与（R-N4 词法
            // 域限定：`/` 仅标识符内部分隔——裸名走既有未绑定路径）
            if base.len() < 2 || !base.contains('/') {
                return None;
            }
            if takeover.contains(raw) || takeover.contains(base) {
                return None;
            }
            let (ns, local) = base.split_once('/')?;
            // M2 别名归一（22 §3.5 R-N5：`str/append` 的 ns 段先查别名
            // 映射——别名是局部绑定的限定引用形态，归一后按 R-N3 查
            // 模块 export 面；非别名 ns 原样查询）
            let alias_of = face
                .aliases
                .iter()
                .find(|(a, _)| a == ns)
                .map(|(_, m)| m.clone());
            let resolved_ns = alias_of.clone().unwrap_or_else(|| ns.to_string());
            // R-N3 不回落：仅查 N2 export 面（stdlib 七模块；用户接管
            // 已豁免——用户模块 export 限定名 = M2 import 面）
            let known = STDLIB_MODULES
                .iter()
                .any(|&(module, local_name, _)| module == resolved_ns && local_name == local);
            if known {
                return None;
            }
            Some(Diagnostic::error(
                Some(QUALIFIED_NOT_EXPORTED_CODE),
                format!(
                    "「{}」不导出「{}」（限定名无回落——R-N3：仅查模块 export 面，不查全局/局部/其他模块{}）",
                    resolved_ns, local,
                    alias_of.map(|m| format!("（别名 {} → kerf-{}）", ns, m)).unwrap_or_default()
                ),
                *span,
            ))
        }
        CoreExpr::Module {
            name, span, body, ..
        } => {
            // E0015 保留域（22 §7）：用户模块名占用 kerf- 前缀
            let module_name = table.name(*name);
            if module_name.starts_with(RESERVED_PREFIX) && !RESERVED_ALLOW.contains(&module_name) {
                return Some(Diagnostic::error(
                    Some(RESERVED_DOMAIN_CODE),
                    format!(
                        "保留域违例：模块名「{}」占用 kerf- 前缀（标准库保留域——22 §7；用户模块名请去前缀）",
                        module_name
                    ),
                    *span,
                ));
            }
            body.iter()
                .find_map(|item| qualified_ref_diag(item, takeover, shadowed, face, table))
        }
        CoreExpr::Lambda { params, body, .. } => {
            // R-N1：参数是 N3 局部绑定——并入遮蔽集再递归体（let 系已
            // 脱糖为 Lambda+App，CoreExpr 层 N3 绑定面 = Lambda 参数）
            let mut inner = shadowed.clone();
            for p in params {
                inner.insert(table.name(*p).to_string());
            }
            qualified_ref_diag(body, takeover, &inner, face, table)
        }
        CoreExpr::App { fn_expr, args, .. } => {
            qualified_ref_diag(fn_expr, takeover, shadowed, face, table).or_else(|| {
                args.iter()
                    .find_map(|a| qualified_ref_diag(a, takeover, shadowed, face, table))
            })
        }
        CoreExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => qualified_ref_diag(cond, takeover, shadowed, face, table)
            .or_else(|| qualified_ref_diag(then_branch, takeover, shadowed, face, table))
            .or_else(|| qualified_ref_diag(else_branch, takeover, shadowed, face, table)),
        CoreExpr::SetBang { value, .. } => {
            qualified_ref_diag(value, takeover, shadowed, face, table)
        }
        CoreExpr::Define { value, .. } => {
            qualified_ref_diag(value, takeover, shadowed, face, table)
        }
        CoreExpr::Begin { body, .. } => qualified_ref_seq(body, takeover, shadowed, face, table),
        CoreExpr::Require { .. } | CoreExpr::Literal { .. } => None,
        CoreExpr::Perform { effect, .. } => {
            qualified_ref_diag(effect, takeover, shadowed, face, table)
        }
        CoreExpr::Handle {
            handler_body, body, ..
        } => qualified_ref_diag(handler_body, takeover, shadowed, face, table)
            .or_else(|| qualified_ref_diag(body, takeover, shadowed, face, table)),
    }
}

/// 序列臂助手（Begin/Module 体——同 verify_refs 形态；遮蔽集透传）。
fn qualified_ref_seq(
    body: &[Rc<CoreExpr>],
    takeover: &std::collections::HashSet<String>,
    shadowed: &std::collections::HashSet<String>,
    face: &ImportFace,
    table: &SymbolTable,
) -> Option<Diagnostic> {
    body.iter()
        .find_map(|item| qualified_ref_diag(item, takeover, shadowed, face, table))
}

/// prelude 注入（TD-021）：声明 import 的程序 → 读入 preamble 形式并
/// 前置合并（种子 Reader——固定库工件与 expander.krf 同口径；本文件
/// 自身错误不可能（构造性正确），Span 归属 preamble.krf 独立 file）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同 front 入口约定，错误体积可接受
fn resolve_prelude_imports(
    forms: Vec<Stx>,
    table: &mut SymbolTable,
    sm: &mut SourceMap,
) -> Result<Vec<Stx>, DriverError> {
    let needed = forms.iter().any(|f| imports_prelude(f, table));
    if !needed {
        return Ok(forms);
    }
    let file_id = sm.add_file(PREAMBLE_FILENAME, PREAMBLE_SRC);
    let mut prelude =
        read_source(PREAMBLE_SRC, file_id, table).map_err(|e| DriverError::from_read(&e, sm))?;
    prelude.extend(forms);
    Ok(prelude)
}

/// 编译前段（read → expand → 相位簿记 → 字节码，不含 lower）。
///
/// **读阶段经自举 Reader**（B3：kerf 源码 Reader 在 VM 上运行，
/// crate::bootstrap 桥接）——生产管线入口。自举 Reader 本身的编译走
/// 种子路径 [`compile_front_seed`]（无递归：种子编译自举实现）。
///
/// [`compile_source`]（完整管线，含图 IR）与 [`run_source`]（生产执行
/// 路径，无需 IR）共用本前段——单一编译逻辑，两分流出口（字节码产物
/// 一致性由 `fast_path_bytecode_matches_full_compile` 守护）。
///
/// **E1-β 生产切换**：读 **与展开** 均经自举实现（VM 上 reader.krf +
/// expander.krf）；**I1 生产切换（42-d）**：编译段同样经自举实现（VM
/// 上 compiler.krf）——「语言能表达自身前端 + 编译器」的完整生产命题；
/// Rust 种子保留双角色：自举引导（三 krf + preamble 的编译）+ parity
/// oracle（测试对照）。切换守护：`production_expander_is_bootstrap` +
/// `production_compiler_is_bootstrap`（独立线程活性探针双信号，实测
/// 判别）；自举一致性（两次编译自身字节一致）由门 B fixpoint 测试
/// 守护（bootstrap_compiler_tests——§21.3 条件 2）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——非性能热路径，错误体积可接受
fn compile_front(source: &str, filename: &str) -> Result<FrontOutput, DriverError> {
    let mut sm = SourceMap::new();
    let file_id = sm.add_file(filename, source);
    let mut table = SymbolTable::new();
    // 默认模块名（无 module 形式时使用）——在任何 read/expand 之前 intern
    let main_sym = table.intern("main");

    // 1. Reader（自举）：源 → Stx
    let forms = crate::bootstrap::read_source(source, file_id, &mut table)
        .map_err(|e| DriverError::from_read(&e, &sm))?;

    front_from_forms(
        forms,
        table,
        sm,
        main_sym,
        file_id,
        ExpanderKind::Bootstrap,
        CompilerKind::Bootstrap,
    )
}

/// 种子编译前段（Rust Reader + Rust Expander + Rust Compiler——自举
/// 实现的引导编译与 parity oracle）。
///
/// 消费方：bootstrap 加载（编译 reader.krf/expander.krf/compiler.krf/
/// preamble.krf）+ parity 测试对照基准（[`run_source_seed`]/
/// [`compile_source_seed`]）。逻辑与 [`compile_front`] 完全同构——
/// read/expand/compile 三入口均为种子实现（无递归：种子编译自举
/// 实现——bootstrap 加载恒种子路径，P1 硬约定）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同上入口约定
pub(crate) fn compile_front_seed(source: &str, filename: &str) -> Result<FrontOutput, DriverError> {
    let mut sm = SourceMap::new();
    let file_id = sm.add_file(filename, source);
    let mut table = SymbolTable::new();
    let main_sym = table.intern("main");

    // 1. Reader（种子）：源 → Stx
    let forms =
        read_source(source, file_id, &mut table).map_err(|e| DriverError::from_read(&e, &sm))?;

    front_from_forms(
        forms,
        table,
        sm,
        main_sym,
        file_id,
        ExpanderKind::Seed,
        CompilerKind::Seed,
    )
}

/// 前段展开器选择（E1-β 生产切换：生产 = 自举 Expander；种子路径 =
/// Rust Expander——bootstrap 加载与 parity oracle）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum ExpanderKind {
    /// 自举 Expander（expander.krf 在 VM 上运行——生产路径）。
    Bootstrap,
    /// Rust 种子 Expander（引导编译 + parity oracle）。
    Seed,
}

/// 前段编译器选择（I1 生产切换 42-d——i1-design §6 P1：分派位在
/// [`front_from_core`] 第 4 步，镜像 `ExpanderKind`）。
///
/// 生产 = 自举 Compiler（compiler.krf 在 VM 上运行）；种子路径 =
/// Rust Compiler（bootstrap 加载引导 + parity oracle——「自举种子
/// 经典角色」07 §3.2）。缓存口径（B11/P4）：键含本维度——种子与
/// 自举产物不得混享缓存条目。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CompilerKind {
    /// 自举 Compiler（VM 上 compiler.krf——生产路径）。
    Bootstrap,
    /// Rust 种子 Compiler（引导编译 + parity oracle）。
    Seed,
}

/// 前段公共部分（read 之后：expand → 相位簿记 → 字节码）。
///
/// 两入口（自举/种子）共享——保证除 read 与 expand 外的管线行为单一
/// 实现。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同上入口约定
fn front_from_forms(
    forms: Vec<Stx>,
    mut table: SymbolTable,
    mut sm: SourceMap,
    main_sym: Symbol,
    file_id: FileId,
    expander: ExpanderKind,
    compiler: CompilerKind,
) -> Result<FrontOutput, DriverError> {
    // 1.5 TD-021 prelude 模块注入（import 解析——read 后、expand 前：
    // forms 级合并，单一编译单元；Span 指向 preamble.krf（独立 file））
    let forms = resolve_prelude_imports(forms, &mut table, &mut sm)?;

    // 1.6 批次 M 次件 M2（r40）import 面收集（Stx 层头部区游走——
    // expand 前的声明面：模块名/别名对/注入名；expand 后 CoreExpr 不
    // 携带别名信息，此面是 R-N5 别名归一与 E0013/E0019 的唯一数据源）
    let face = collect_import_face(&forms, &table);
    let import_targets = collect_import_targets(&forms, &table);
    // 1.58 r42 / S1 移除轮：W1003 宏名收集（Stx 层——expand 前的
    // 声明面；宏名与内置注册面同名 → W 级知会[22 §11 D11]）
    let macro_defs = collect_macro_names(&forms, &table);

    // 1.65 r41 / 62-a 深审 D10：E0020 保留字绑定禁令（Stx 源码面——
    // expand 前拦截；宏名/别名唯一承载层；run/check/compile 全路径）
    verify_reserved_bindings_stx(&forms, &table, &sm)?;

    // 2. Expander：Stx → CoreExpr（visit = 变换器注册完成）
    let core = match expander {
        ExpanderKind::Bootstrap => {
            let mut t = table;
            let core = crate::bootstrap_expander::expand_program(&forms, file_id, &mut t)
                .map_err(|e| DriverError::from_expand(&e, &sm))?;
            table = t;
            core
        }
        ExpanderKind::Seed => {
            let mut ectx = ExpandCtxt::new(table);
            let core =
                expand_program(&forms, &mut ectx).map_err(|e| DriverError::from_expand(&e, &sm))?;
            table = ectx.table;
            core
        }
    };

    front_from_core(
        core,
        face,
        import_targets,
        macro_defs,
        table,
        sm,
        main_sym,
        file_id,
        compiler,
    )
}

/// 前段核心段（expand 之后：R9 验证 → 相位簿记 → 字节码——
/// [`check_source_recover`]（TD-013 恢复路径）与 [`front_from_forms`]
/// 共享：部分产物（恢复跳过错误形式后的 core）与完整产物走同一
/// 后续管线，行为单一实现（§12 最优>最小）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同上入口约定
#[allow(clippy::too_many_arguments)] // front 管线共享段（两入口汇聚——参数面即管线段清单）
fn front_from_core(
    core: Vec<Rc<CoreExpr>>,
    face: ImportFace,
    import_targets: Vec<(String, Span)>,
    macro_defs: Vec<(String, Span)>,
    mut table: SymbolTable,
    sm: SourceMap,
    main_sym: Symbol,
    file_id: FileId,
    compiler: CompilerKind,
) -> Result<FrontOutput, DriverError> {
    // 2.45 r41 / 62-a 深审 D10：E0020 保留字绑定禁令（CoreExpr 展开
    // 产物面——防御纵深：宏模板生成的绑定名同样过禁令；front 全路径）
    verify_reserved_bindings_core(&core, &table, &sm)?;

    // 2.5 批次 M 次件 M2（r40）——组合闭包**先行**（21 §4.4：需求 ⊆
    // 授权——E0006 增强形态先于 R9 基础形态：模块需求违规先报[携带
    // 模块归属与上移指引]，纯程序级缺声明才走基础形态）+
    // import 面验证（E0013 冲突/E0017 别名重复/E0019 未知模块）+
    // E0016 模块内重复 define + E0018 require 位置纪律（fail-closed
    // 全链阻断——与 R9 同口径；front 管线全路径生效）
    crate::capability::verify_capability_closure(&core, &table, &sm)?;
    verify_import_face(&face, &table, &sm, &import_targets)?;
    verify_module_define_duplicates(&core, &table, &sm)?;
    verify_require_positions(&core, &table, &sm)?;

    // 2.55 R9 能力权限验证（r8——13 §3.1.3 条款 3「编译期错误」：
    // front 管线全路径生效 run/check/compile；首个违规即阻断
    // fail-closed；E0006 家族与 E0005 静态检查分离）
    verify_io_capabilities(&core, &table, &sm)?;

    // 2.6 批次 M 首件 M1（r39）限定名验证（22 §3.3 R-N3 不回落 +
    // §7 保留域——E0014/E0015；front 管线全路径同 R9 生效）+
    // M2 别名归一验证（face.aliases——诊断携原始名+归一面）
    verify_qualified_refs(&core, &face, &table, &sm)?;

    // 2.7 M2 别名限定名编译期归一（verify 后重写——E0014 诊断用原始
    // 名；重写后 registry/compile/HM/运行全链消费归一限定名）
    let core = rewrite_alias_refs(core, &face, &mut table);

    // 3. 相位簿记：declare + visit（§8.9——TD-021 多模块：全部 module
    // 形式按出现序 declare（prelude 注入在前、用户模块在后），visit 主
    // 模块（最后一个 module 形式——import 边传递依赖 visit；无 module
    // 程序行为与单模块时代一致：find_map 兜底 main）
    let mut registry = ModuleRegistry::new();
    // M2 stdlib 预 declare（import 边合法化——stdlib 七模块是 Rust
    // 内置模块（M1 形态：限定名直接注册），非程序内 module 形式；程序
    // 模块 `(import kerf-string)` 的 visit 传递依赖检查需要它们在
    // registry 在场。预置 visited = true（stdlib 无 import 边——visit
    // 叶子；语义 = N2 标准库面在相位簿记中恒在场）。**kerf-prelude
    // 除外**：preamble 注入时自带 `(module kerf-prelude ...)` 形式
    // （正常 declare 路径——预声明会与之重复冲突）
    for stdlib_name in RESERVED_ALLOW {
        if *stdlib_name == "kerf-prelude" {
            continue;
        }
        let sym = table.intern(stdlib_name);
        let _ = registry.declare(sym, vec![], vec![]);
        let _ = registry.visit(sym);
    }
    let mut main_module_span = Span::dummy();
    for e in core.iter() {
        if let CoreExpr::Module {
            name,
            imports,
            exports,
            span,
            ..
        } = e.as_ref()
        {
            let owned_imports = imports.clone();
            let owned_exports = exports.clone();
            registry
                .declare(*name, owned_imports, owned_exports)
                .map_err(|m| {
                    // 声明错误挂该 module 形式 Span（诊断定位——门审计
                    // 发现项修复：此前 dummy 无定位 + 无渲染）
                    let diag = Diagnostic::error(Some(DiagnosticCode(2)), m.clone(), *span);
                    DriverError {
                        stage: Stage::Expand,
                        rendered: render_diagnostic(&diag, &sm),
                        diagnostic: diag,
                    }
                })?;
            main_module_span = *span;
        }
    }
    // 无 module 形式的程序：main 兜底声明（镜像旧单模块行为——
    // visit 前提是已 declare）
    if !core
        .iter()
        .any(|e| matches!(e.as_ref(), CoreExpr::Module { .. }))
    {
        registry.declare(main_sym, vec![], vec![]).map_err(|m| {
            let diag = Diagnostic::error(Some(DiagnosticCode(2)), m.clone(), Span::dummy());
            DriverError {
                stage: Stage::Expand,
                rendered: render_diagnostic(&diag, &sm),
                diagnostic: diag,
            }
        })?;
    }
    let module_name = core
        .iter()
        .rev()
        .find_map(|e| match e.as_ref() {
            CoreExpr::Module { name, .. } => Some(*name),
            // _ 臂理由：非 module 顶形式不参与查找（rev 取最后一个——
            // 主模块 = 用户模块（prelude 注入序在前））
            _ => None,
        })
        .unwrap_or(main_sym);
    registry.visit(module_name).map_err(|m| {
        // visit 错误挂主模块形式 Span（循环依赖/未声明导入的定位——
        // 门审计发现项修复：此前 dummy 无定位 + 无渲染）
        let diag = Diagnostic::error(Some(DiagnosticCode(2)), m.clone(), main_module_span);
        DriverError {
            stage: Stage::Expand,
            rendered: render_diagnostic(&diag, &sm),
            diagnostic: diag,
        }
    })?;

    // 4. Compile：CoreExpr 树 → 字节码（I1 生产切换 42-d：分派
    // CompilerKind——镜像 ExpanderKind（i1-design §6 P1）；lower 仅
    // 完整入口执行——见下）
    let program = match compiler {
        // 种子臂理由：Rust 编译器 = 引导 + parity oracle（bootstrap
        // 加载恒种子路径——无递归，B5/P1 硬约定）
        CompilerKind::Seed => kerf_compiler::compile_module(&core),
        // 自举臂理由：compiler.krf 在 VM 上运行（compile_front 生产
        // 路径——桥经 bootstrap_compiler）
        CompilerKind::Bootstrap => {
            crate::bootstrap_compiler::compile_module(&core, file_id, &mut table)
        }
    }
    .map_err(|e| DriverError::from_compile(&e, &sm))?;

    // 5. M2 W 警告收集（编译成功路径——非阻断；Stx 层 import 面 +
    // core 树双数据源）
    let warnings = collect_warnings(&core, &table, &face, &macro_defs);

    Ok(FrontOutput {
        core,
        warnings,
        program,
        source_map: sm,
        table,
        registry,
    })
}

/// 缓存感知的编译前段（批次 C——[13-能力矩阵 §3.1.4] 做实的管线消费面）。
///
/// 键命中：返回 FrontOutput **克隆**（快照语义——符号表/源映射随产物
/// 一致复用）；未中：编译并写入缓存。错误路径不缓存。缓存停用时直
/// 编译（基准对照）。返回值第二项 = 是否命中（观测/报告用）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同上入口约定
pub(crate) fn compile_front_cached(
    source: &str,
    filename: &str,
) -> Result<(FrontOutput, bool), DriverError> {
    if !cache_enabled() {
        return Ok((compile_front(source, filename)?, false));
    }
    // 键含 CompilerKind 维度（B11/P4——i1-design §6：种子与自举产物
    // 不混享缓存条目；本入口恒生产（Bootstrap）；种子参考路径
    // （compile_front_seed）不经缓存）
    let key = cache_key(source, filename, CompilerKind::Bootstrap);
    if let Some(front) = with_cache(|c| c.lookup_front(&key)) {
        return Ok((front, true));
    }
    let out = compile_front(source, filename)?;
    with_cache(|c| c.store_front(key, out.clone()));
    Ok((out, false))
}

/// 静态检查报告（`kerf check` 子命令与外部消费方的数据面——多错误
/// 收集，TD-013 设计的首个消费面）。
#[derive(Debug, Clone)]
pub struct CheckReport {
    /// 全部静态诊断（Span 次序；E0005）。
    pub diagnostics: Vec<Diagnostic>,
    /// 逐条渲染后的诊断文本（error[E0005] + `-->` 位置 + 源摘录）。
    pub rendered: Vec<String>,
    /// 编译前段是否命中缓存（增量编译基础设施的观测面）。
    pub cache_hit: bool,
    /// 程序是否声明读能力（(require io read)——r8 能力观测面）。
    pub io_read: bool,
    /// 程序是否声明写能力（(require io write)——同上）。
    pub io_write: bool,
    /// 原型数。
    pub proto_count: usize,
    /// 常量池大小。
    pub const_count: usize,
    /// 全局引用数。
    pub global_ref_count: usize,
    /// 指令总数。
    pub instruction_count: usize,
    /// W 级诊断（M2——非阻断；渲染面与 error 诊断同通道）。
    pub warnings: Vec<Diagnostic>,
}

/// 静态检查源文本：read → expand → compile（缓存路径）→ HM 推断
/// 判定（[kerf_compiler::hm::hm_check_program]——约束三段式；多错误全量
/// 收集，非短路）。
///
/// **旗标期（D8 阶段 2——r29/50-a 切换）**：`kerf check` 判定面 =
/// HM 推断（hm-inference-design §3.7 演进轨道阶段 2）；R1-R8
/// （[kerf_compiler::check_program]）退为**回归基线断言**（测试面
/// 保留——超集门参照侧，非生产判定面）。保守性契约随旗标期重定义：
/// 「零类型不一致误报」口径 + occurs/自应用面豁免为接受行为
/// （hm-inference-design §2.3 契约变更显式登记——P0-2 兑现）。
///
/// 编译期错误（Read/Expand/Compile 阶段）仍以 `DriverError` 返回——
/// 静态检查只对编译通过的程序进行（诊断链前后不交叉）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同上入口约定
pub fn check_source(source: &str, filename: &str) -> Result<CheckReport, DriverError> {
    let (front, cache_hit) = compile_front_cached(source, filename)?;
    let io_req = IoRequirements::from_core(&front.core);
    let mut table = front.table;
    let sigs = builtin_sigs(&mut table);
    let diagnostics = kerf_compiler::hm::hm_check_program(&front.core, &sigs, &table).diags;
    let rendered: Vec<String> = diagnostics
        .iter()
        .map(|d| render_diagnostic(d, &front.source_map))
        .collect();
    let warnings = front.warnings;
    let warnings_rendered: Vec<String> = warnings
        .iter()
        .map(|d| render_diagnostic(d, &front.source_map))
        .collect();
    let mut rendered = rendered;
    rendered.extend(warnings_rendered);
    Ok(CheckReport {
        proto_count: front.program.proto_count(),
        const_count: front.program.consts.len(),
        global_ref_count: front.program.global_refs.len(),
        instruction_count: front.program.total_instructions(),
        io_read: io_req.read,
        io_write: io_req.write,
        diagnostics,
        rendered,
        warnings,
        cache_hit,
    })
}

/// 静态检查源文本（**恢复模式**——TD-013 实现，批次 G / 38-e）。
///
/// 与 [`check_source`]（单错误短路）并列：expand 阶段形式级恢复——
/// 单形式展开失败 → 诊断收集 + 跳过 + 继续后续形式（r7 设计 §2）；
/// 部分产物照常走 R9 验证 / 相位簿记 / 字节码 / 类型检查。诊断合并
/// 面 = 展开诊断（E0002）+ 类型诊断（E0005），按 `(file_id, start,
/// end)` 排序——输出次序与源码位置对齐（与 hm_inference 同口径）。
///
/// **旗标期（D8 阶段 2——r29/50-a 切换）**：判定面 = HM 推断
/// （[kerf_compiler::hm::hm_check_program]——与 [`check_source`] 同判定面；
/// R1-R8 退为回归基线断言，契约重定义同 §2.3）。
///
/// **短路边界**：read 错误（词法级）与 R9 能力违规（E0006 fail-closed，
/// r8 裁定「首个违规即阻断」）仍以 `DriverError` 返回——恢复面仅覆盖
/// expand 阶段错误（r7 设计 §5 消费面表格的 Read/Expand/Compile 中
/// Expand 段；compile 错误在部分产物上属全程序性，短路）。
///
/// **缓存裁定**：不走 [`compile_front_cached`]——部分产物入缓存会污染
/// 正常路径的完整产物命中（缓存键 = 源内容寻址，无法区分恢复模式）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同上入口约定
pub fn check_source_recover(source: &str, filename: &str) -> Result<CheckReport, DriverError> {
    // 1. Reader（自举——错误短路：词法级不可形式级恢复）
    let mut sm = SourceMap::new();
    let file_id = sm.add_file(filename, source);
    let mut table = SymbolTable::new();
    let main_sym = table.intern("main");
    let forms = crate::bootstrap::read_source(source, file_id, &mut table)
        .map_err(|e| DriverError::from_read(&e, &sm))?;
    // 1.5 prelude 注入（与 front_from_forms 同序：read 后、expand 前）
    let forms = resolve_prelude_imports(forms, &mut table, &mut sm)?;
    // 1.6 M2 import 面收集（与 front_from_forms 同序）
    let face = collect_import_face(&forms, &table);
    let import_targets = collect_import_targets(&forms, &table);
    // 1.58 r42 / S1：W1003 宏名收集（与 front_from_forms 同序）
    let macro_defs = collect_macro_names(&forms, &table);
    // 1.65 r41 / 62-a 深审 D10：E0020 保留字绑定禁令（Stx 源码面——
    // 与 front_from_forms 同序；恢复路径同样 fail-closed 拦截）
    verify_reserved_bindings_stx(&forms, &table, &sm)?;
    // 2. 恢复展开（自举桥——逐形式；with_expander 加载失败仍致命 Err）
    let recovered = crate::bootstrap_expander::expand_program_recover(&forms, file_id, &mut table)
        .map_err(|e| DriverError::from_expand(&e, &sm))?;
    let expand_count = recovered.diags.len();
    let core = recovered.core;
    // 3. 部分产物 → 后续管线（R9 fail-closed + 簿记 + 字节码——共享
    // front_from_core：短路边界如上文档；编译段 = 生产口径（自举
    // Compiler——check 为生产子命令面））
    let front = front_from_core(
        core,
        face,
        import_targets,
        macro_defs,
        table,
        sm,
        main_sym,
        file_id,
        CompilerKind::Bootstrap,
    )?;
    // 4. 类型检查（部分产物）+ 诊断合并 + 排序
    let io_req = IoRequirements::from_core(&front.core);
    let mut table = front.table;
    let sigs = builtin_sigs(&mut table);
    let mut diagnostics: Vec<Diagnostic> = recovered
        .diags
        .into_sorted()
        .into_iter()
        .map(|e| Diagnostic::error(Some(DiagnosticCode(2)), e.message, e.span))
        .collect();
    let mut type_diags = kerf_compiler::hm::hm_check_program(&front.core, &sigs, &table).diags;
    diagnostics.append(&mut type_diags);
    diagnostics.sort_by(|a, b| {
        a.primary_span
            .file_id
            .cmp(&b.primary_span.file_id)
            .then(a.primary_span.start.cmp(&b.primary_span.start))
            .then(a.primary_span.end.cmp(&b.primary_span.end))
    });
    let rendered: Vec<String> = diagnostics
        .iter()
        .map(|d| render_diagnostic(d, &front.source_map))
        .collect();
    let warnings = front.warnings;
    let warnings_rendered: Vec<String> = warnings
        .iter()
        .map(|d| render_diagnostic(d, &front.source_map))
        .collect();
    let mut rendered = rendered;
    rendered.extend(warnings_rendered);
    Ok(CheckReport {
        proto_count: front.program.proto_count(),
        const_count: front.program.consts.len(),
        global_ref_count: front.program.global_refs.len(),
        instruction_count: front.program.total_instructions(),
        io_read: io_req.read,
        io_write: io_req.write,
        diagnostics,
        rendered,
        warnings,
        cache_hit: false, // 恢复路径不缓存（裁定见函数文档）
    })
    .map(|mut r| {
        // 截断提示（r7 设计 §2：上限后附 warning 级汇总行——复用
        // rendered 尾注，diagnostics 数量以收集器为准不重复入列）
        if expand_count >= EXPAND_DIAG_CAP {
            r.rendered.push(format!(
                "note: 展开诊断达到上限 {}（截断——诊断风暴防护）",
                EXPAND_DIAG_CAP
            ));
        }
        r
    })
}

/// 展开诊断上限（kerf-expander 常量直引——driver 侧零魔数）。
const EXPAND_DIAG_CAP: usize = kerf_expander::MAX_DIAGNOSTICS;

/// 编译源文本（全管线，不执行）。
///
/// 管线：read → expand（含相位 declare/visit 簿记）→ lower（图 IR）
/// → compile（字节码）。instantiate 簿记在执行入口（run）完成。
///
/// 消费方：`kerf ir`/`code`/`bc`/`check` 等 dump 与检查子命令、测试、
/// CodeValue 检查——需要图 IR 或完整产物的场景。纯执行路径（run）
/// 走 [`compile_front`] 快路径（TD-015 分流）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——非性能热路径，错误体积可接受
pub fn compile_source(source: &str, filename: &str) -> Result<CompileOutput, DriverError> {
    let (front, _hit) = compile_front_cached(source, filename)?;
    // Lower：CoreExpr → 图 IR（结构化形式 + 共享节点）——仅消费方按需计算
    let ir = lower_program(&front.core);
    Ok(CompileOutput {
        core: front.core,
        ir,
        program: front.program,
        source_map: front.source_map,
        table: front.table,
        registry: front.registry,
        warnings: front.warnings,
    })
}

/// 种子管线完整编译（参考路径：Rust 三段（读/展开/编译）——门 B
/// fixpoint 的 B₀ 基准与字节码 parity oracle（42-d）；与
/// [`compile_source`] 同构但**不经缓存**（B11/P4：种子产物不入生产
/// 缓存——种子与自举产物不混享条目）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同上入口约定
pub fn compile_source_seed(source: &str, filename: &str) -> Result<CompileOutput, DriverError> {
    let front = compile_front_seed(source, filename)?;
    let ir = lower_program(&front.core);
    Ok(CompileOutput {
        core: front.core,
        ir,
        program: front.program,
        source_map: front.source_map,
        table: front.table,
        registry: front.registry,
        warnings: front.warnings,
    })
}

/// 运行产物：最终值 + 存活堆（值中的 `GcRef` 引用该堆——渲染需要堆存活）。
pub struct RunOutcome {
    /// 最终值。
    pub value: Value,
    /// 执行后的堆（GC 统计与值渲染使用）。
    pub heap: Heap,
    /// W 级诊断（W1002 遮蔽族/W1003 宏名遮蔽族；CLI/测试观测面）。
    pub warnings: Vec<Diagnostic>,
}

impl std::fmt::Debug for RunOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunOutcome")
            .field("value", &self.value.type_name())
            .field("heap_slots", &self.heap.slot_count())
            .field("warnings", &self.warnings.len())
            .finish()
    }
}

/// 运行源文本（生产路径：编译前段 + VM 执行）。
///
/// 走 [`compile_front_cached`] 快路径——不构造图 IR（TD-015：执行路径
/// 无 IR 消费，旁路计算为恒定开销浪费）；同源重复执行命中编译缓存
/// （批次 C——内容寻址键，产物等价性由确定性编译保证）；前段三段
/// 均自举（I1 生产切换 42-d）——执行段单一实现（[`run_front`]，与
/// 种子参考路径 [`run_source_seed`] 共享）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——非性能热路径，错误体积可接受
pub fn run_source(source: &str, filename: &str) -> Result<RunOutcome, DriverError> {
    let (out, _cache_hit) = compile_front_cached(source, filename)?;
    run_front(out)
}

/// 种子管线执行（参考路径：Rust 三段（读/展开/编译）+ VM）。
///
/// T1 双路径互查的 oracle 面（42-d P5——eval 元循环求值器退役后的
/// 替代对拍口径：生产链 [`run_source`]（自举三段）vs 种子链本入口，
/// 两独立实现同源同果）。**不经编译缓存**（B11/P4：种子产物不入
/// 生产缓存——种子与自举产物不混享条目）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同上入口约定
pub fn run_source_seed(source: &str, filename: &str) -> Result<RunOutcome, DriverError> {
    let front = compile_front_seed(source, filename)?;
    run_front(front)
}

/// 前段产物执行核心（值语义——run_source 与 run_source_seed 共享：
/// 两入口仅前段（自举/种子）不同，执行段单一实现）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同上入口约定
fn run_front(front: FrontOutput) -> Result<RunOutcome, DriverError> {
    let mut table = front.table;
    let warnings = front.warnings;
    let grant = IoGrant::from_requirements(IoRequirements::from_core(&front.core));
    let mut globals = register_globals(&mut table, &grant);
    let program = front.program;
    let source_map = front.source_map;
    // 卫生回退解析（$hyg$N 后缀剥离）
    resolve_hygiene_fallbacks(&program, &mut table, &mut globals);
    let mut heap = Heap::new();
    // instantiate 簿记（Phase 0 执行）
    instantiate_registry(front.registry);
    run_program(&program, &mut globals, &mut heap)
        .map(|value| RunOutcome {
            value,
            heap,
            warnings,
        })
        .map_err(|e| DriverError::from_vm(&e, &source_map, &table))
}

fn instantiate_registry(mut registry: ModuleRegistry) {
    for entry in registry.entries().to_vec() {
        let _ = registry.instantiate(entry.name);
    }
}

/// 运行源文本并渲染结果（CLI/Playground 便捷形态）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——非性能热路径，错误体积可接受
pub fn run_source_rendered(source: &str, filename: &str) -> Result<String, DriverError> {
    let outcome = run_source(source, filename)?;
    Ok(kerf_vm::render_value(&outcome.value, &outcome.heap))
}

// ---------------------------------------------------------------------------
// kerf test：用例运行器（r8——内部效应系统的消费面：测试短路 + 错误恢复）
// ---------------------------------------------------------------------------

/// 单用例结果。
#[derive(Debug, Clone)]
pub struct TestCaseOutcome {
    /// 用例序号（1 起）。
    pub index: usize,
    /// 用例形态（核心表达式渲染——截断形态）。
    pub name: String,
    /// 是否通过（求值无错且值非 #f）。
    pub pass: bool,
    /// 失败详情（效应载荷——用例内任意深度捕获）。
    pub detail: String,
}

/// 测试报告。
#[derive(Debug, Clone)]
pub struct TestReport {
    /// 逐用例结果（源序）。
    pub cases: Vec<TestCaseOutcome>,
    /// 通过数。
    pub passed: usize,
    /// 失败数。
    pub failed: usize,
}

impl TestReport {
    /// 全部通过。
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }
}

/// 测试用例失败载荷（效应系统类型化逃逸——`perform_escape` 从用例体内
/// 任意深度触发，`handle_escape` 边界捕获：**测试短路**（case 内首错即
/// 停止该 case）+ **错误恢复**（边界捕获后下一 case 续跑——同进程状态
/// 隔离：每 case 全新全局环境与堆）。
struct TestFailure {
    detail: String,
}

/// 用例内任意深度失败上报（嵌套助手层零签名污染——D-任意节点消费面）。
fn fail_test(detail: impl Into<String>) -> ! {
    crate::effects::perform_escape(TestFailure {
        detail: detail.into(),
    })
}

/// 判定核心形式是否为前置（prelude）形态（define/set!/module/require——
/// 状态与声明，非用例）。
fn is_prelude_form(e: &CoreExpr) -> bool {
    matches!(
        e,
        CoreExpr::Define { .. }
            | CoreExpr::SetBang { .. }
            | CoreExpr::Module { .. }
            | CoreExpr::Require { .. }
    )
}

/// 运行测试源文本（`kerf test` 的驱动入口）。
///
/// **约定**（零新语法——测试面即语言面）：
/// - 前置形式（define/set!/module/require）= 公共前置（每个用例独立
///   重放——用例间状态隔离，v0 口径：用例是前置的纯函数）；
/// - 其余顶层表达式形式 = 用例：求值无错且值非 `#f` = PASS（`(= a b)`
///   形谓词即断言），运行时错误或 `#f` = FAIL；
/// - 执行路径 = 生产 VM 路径（§11 无平行语义；编译缓存内容寻址复用）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——driver 入口既有约定
pub fn test_source(source: &str, filename: &str) -> Result<TestReport, DriverError> {
    // 效应系统 hook 安装（幂等——进程一次；效应逃逸零噪声）
    crate::effects::ensure_hook_installed();
    let (front, _hit) = compile_front_cached(source, filename)?;
    let table = front.table;
    let sm = front.source_map;
    let grant = IoGrant::from_requirements(IoRequirements::from_core(&front.core));

    // 前置/用例切分（源序保持）
    let mut prelude: Vec<Rc<CoreExpr>> = Vec::new();
    let mut cases: Vec<Rc<CoreExpr>> = Vec::new();
    for e in &front.core {
        if is_prelude_form(e) {
            prelude.push(e.clone());
        } else {
            cases.push(e.clone());
        }
    }

    let mut outcomes: Vec<TestCaseOutcome> = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        let index = i + 1;
        let name = truncate_form_name(&case.render(&|s: Symbol| resolve_symbol(s, &table)));
        // 用例边界（效应处理器）：短路（case 内任意深度 fail_test 即停）
        // + 恢复（捕获后下一用例续跑）
        let outcome = crate::effects::handle_escape::<(), TestFailure>(|| {
            run_case(&prelude, case, &table, &sm, &grant)
        });
        let (pass, detail) = match outcome {
            Ok(()) => (true, String::new()),
            Err(f) => (false, f.detail),
        };
        outcomes.push(TestCaseOutcome {
            index,
            name,
            pass,
            detail,
        });
    }

    let passed = outcomes.iter().filter(|c| c.pass).count();
    Ok(TestReport {
        failed: outcomes.len() - passed,
        passed,
        cases: outcomes,
    })
}

/// 单用例执行（嵌套助手层——失败点任意深度 `fail_test` 直达边界）。
fn run_case(
    prelude: &[Rc<CoreExpr>],
    case: &Rc<CoreExpr>,
    table: &SymbolTable,
    sm: &SourceMap,
    grant: &IoGrant,
) {
    // 层 1：编译切片（前置 ++ 用例）
    let mut slice = prelude.to_vec();
    slice.push(case.clone());
    let program = match kerf_compiler::compile_module(&slice) {
        Ok(p) => p,
        Err(e) => {
            let diag = kerf_span::Diagnostic::error(
                Some(kerf_span::DiagnosticCode(3)),
                e.message.clone(),
                e.span,
            );
            fail_test(format!(
                "用例编译失败：{}",
                first_line(&kerf_span::render_diagnostic(&diag, sm))
            ))
        }
    };
    // 层 2：全新环境执行（状态隔离）
    let mut run_table = table.clone();
    let mut globals = register_globals(&mut run_table, grant);
    let mut heap = Heap::new();
    // 卫生回退解析（与 run_source 同则——宏引入全局引用可达）
    resolve_hygiene_fallbacks(&program, &mut run_table, &mut globals);
    let value = match kerf_vm::run_program(&program, &mut globals, &mut heap) {
        Ok(v) => v,
        Err(e) => {
            let derr = DriverError::from_vm(&e, sm, table);
            fail_test(first_line(&derr.rendered).to_string())
        }
    };
    // 层 3：断言检查（嵌套最深点：#f 即失败，其余值通过）
    assert_case_value(&value, &heap);
}

/// 值断言（层 3——嵌套最深点：#f 即失败，其余值通过）。
fn assert_case_value(value: &Value, heap: &Heap) {
    if matches!(value, Value::Bool(false)) {
        fail_test(format!(
            "断言返回 #f（期望非 #f 值）——求值结果 {}",
            kerf_vm::render_value(value, heap)
        ));
    }
}

/// 用例名截断（渲染形态 ≤ 64 字符）。
fn truncate_form_name(rendered: &str) -> String {
    const MAX: usize = 64;
    if rendered.chars().count() <= MAX {
        rendered.to_string()
    } else {
        let truncated: String = rendered.chars().take(MAX).collect();
        format!("{}…", truncated)
    }
}

/// 首行提取（诊断消息压缩）。
fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or("")
}

/// Token 流 dump（§16 人类可感知输出 / §14.9 编译器自调试）。
///
/// reader 是 driver 唯一合法调用者（§11 接口隔离 §14.7.2 B4）——
/// 根 CLI 的 `tokens` 子命令经本入口转发，不直连 kerf-reader。
/// B3：读阶段经自举 Reader（kerf 源码在 VM 上运行）——Token 种类
/// 分类/数字语义与种子同源（桥侧同源 parse）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同上方入口约定，错误体积可接受
pub fn dump_tokens(source: &str, filename: &str) -> Result<String, DriverError> {
    let mut sm = SourceMap::new();
    let file_id = sm.add_file(filename, source);
    let mut table = SymbolTable::new();
    let toks = crate::bootstrap::lex_source(source, file_id, &mut table)
        .map_err(|e| DriverError::from_read(&e, &sm))?;
    let mut out = String::new();
    for t in &toks {
        let name = match &t.kind {
            TokenKind::Identifier(s) => table.name(*s).to_string(),
            TokenKind::Keyword(k) => k.as_str().to_string(),
            TokenKind::Operator(o, s) => format!("{:?}({})", o, table.name(*s)),
            other => format!("{:?}", other),
        };
        out.push_str(&format!(
            "{:>4}..{:<4} {}\n",
            t.span.start, t.span.end, name
        ));
    }
    Ok(out)
}

/// 语法对象 dump（Stx 渲染——§14.9 编译器自调试）。
///
/// 同 [`dump_tokens`]：CLI `stx` 子命令经 driver 转发调用 reader（B3：
/// 自举 Reader）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同上方入口约定，错误体积可接受
pub fn dump_stx(source: &str, filename: &str) -> Result<String, DriverError> {
    let mut sm = SourceMap::new();
    let file_id = sm.add_file(filename, source);
    let mut table = SymbolTable::new();
    let forms = crate::bootstrap::read_source(source, file_id, &mut table)
        .map_err(|e| DriverError::from_read(&e, &sm))?;
    let mut out = String::new();
    for f in &forms {
        out.push_str(&format!(
            "{}\n",
            f.render(&|s: Symbol| table.name(s).to_string())
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_expander_is_bootstrap() {
        // E1-β 生产切换守护（§2.3-11 实测判别，非推断）：独立线程内
        // （thread_local 状态零残留）——编译前自举 Expander 未加载，
        // 编译后已加载 ⇒ compile_front 的展开段确实经 bootstrap_
        // expander（VM 上运行）；辅以宏产物展开代次标记（≥1——retag
        // 统一 +1）双信号确认。
        let ok = std::thread::spawn(|| {
            let before = crate::bootstrap_expander::is_loaded();
            let src = "(define-syntax m (syntax-rules () ((m x) (begin x)))) (m 42)";
            let (front, _hit) = compile_front_cached(src, "t.krf").expect("编译失败");
            let after = crate::bootstrap_expander::is_loaded();
            let has_expansion_mark = front.core.iter().any(|e| match e.as_ref() {
                CoreExpr::Begin { span, .. } => span.expansion_id >= 1,
                _ => false,
            });
            (!before, after, has_expansion_mark)
        })
        .join()
        .expect("探针线程失败");
        assert!(ok.0, "独立线程起始应未加载自举 Expander");
        assert!(ok.1, "生产编译后自举 Expander 应已加载（VM 路径活性）");
        assert!(ok.2, "宏产物应携带展开代次标记（retag +1）");
    }

    #[test]
    fn run_simple_program() {
        let o = run_source("42", "t.krf").unwrap();
        assert!(matches!(o.value, Value::Int(42)));
    }

    #[test]
    fn run_arith() {
        let o = run_source("(+ 1 2 3)", "t.krf").unwrap();
        assert!(matches!(o.value, Value::Int(6)));
    }

    #[test]
    fn run_define_and_call() {
        let src = "(define (f x) (+ x 1)) (f 41)";
        let o = run_source(src, "t.krf").unwrap();
        assert!(matches!(o.value, Value::Int(42)));
    }

    #[test]
    fn production_seed_paths_agree() {
        // T1 双路径互查新口径（42-d P5——eval 退役）：生产链（自举
        // 三段 + VM）vs 种子链（Rust 三段 + VM）同源同果——两独立
        // 实现互查（§21.8 Phase 1 核心验证的 I1 后形态）
        let src = "(define (f x) (+ x 1)) (f 41)";
        let a = run_source(src, "t.krf").unwrap();
        let b = run_source_seed(src, "t.krf").unwrap();
        assert!(a.value.eq_value(&b.value));
    }

    #[test]
    fn production_compiler_is_bootstrap() {
        // I1 生产切换守护（§2.3-11 实测判别，非推断——镜像
        // production_expander_is_bootstrap）：独立线程内（thread_local
        // 状态零残留）——编译前自举 Compiler 未加载，生产编译后已
        // 加载 ⇒ compile_front 的编译段确实经 bootstrap_compiler
        // （VM 上运行 compiler.krf）；第二信号：种子路径
        // （compile_front_seed）不加载自举 Compiler（bootstrap 加载恒
        // 种子路径——P1 硬约定的实测面）。
        let prod = std::thread::spawn(|| {
            let before = crate::bootstrap_compiler::is_loaded();
            let src = "(define (seed-guard-42) 41) (seed-guard-42)";
            let (front, _hit) = compile_front_cached(src, "guard.krf").expect("编译失败");
            let after = crate::bootstrap_compiler::is_loaded();
            (before, after, front.program.total_instructions() > 0)
        })
        .join()
        .expect("探针线程失败");
        assert!(!prod.0, "独立线程起始应未加载自举 Compiler");
        assert!(prod.1, "生产编译后自举 Compiler 应已加载（VM 路径活性）");
        assert!(prod.2, "生产产物应为非空字节码");
        let seed = std::thread::spawn(|| {
            let before = crate::bootstrap_compiler::is_loaded();
            let front = compile_front_seed("(+ 40 2)", "seed-guard.krf");
            let after = crate::bootstrap_compiler::is_loaded();
            (before, after, front.is_ok())
        })
        .join()
        .expect("种子探针线程失败");
        assert!(!seed.0, "种子探针线程起始亦应未加载（隔离前提）");
        assert!(!seed.1, "种子路径不得加载自举 Compiler（引导恒种子）");
        assert!(seed.2, "种子编译应成功");
    }

    #[test]
    fn read_error_is_rendered() {
        let err = run_source("(+ 1", "bad.krf").unwrap_err();
        assert_eq!(err.stage, Stage::Read);
        assert!(err.rendered.contains("未闭合"));
        assert!(err.rendered.contains("bad.krf"));
    }

    #[test]
    fn expand_error_is_rendered() {
        let err = run_source("(lambda (x))", "bad.krf").unwrap_err();
        assert_eq!(err.stage, Stage::Expand);
        assert!(err.to_string().contains("[expand]"));
    }

    #[test]
    fn compile_output_dumps() {
        let out = compile_source("(+ 1 2)", "t.krf").unwrap();
        assert!(out.disassemble().contains("CALL"));
        assert_eq!(out.render_core(), "(+ 1 2)");
        assert!(out.ir.len() >= 3);
    }

    #[test]
    fn fast_path_bytecode_matches_full_compile() {
        // TD-015 分流守护：run 快路径（compile_front）与完整编译
        // （compile_source）对同一源必须产出一致字节码——防止两出口
        // 编译逻辑漂移（孤立正确 → 集成失败的 §7.2 防崩检查 Q2）。
        let src = "(define (f x) (+ x 1)) (f 2)";
        let full = compile_source(src, "t.krf").unwrap();
        let front = compile_front(src, "t.krf").unwrap();
        let full_text = full.disassemble();
        let front_text =
            disassemble_program(&front.program, &|s: Symbol| resolve_symbol(s, &front.table));
        assert_eq!(
            full_text, front_text,
            "快路径与完整编译的字节码必须逐指令一致"
        );
    }

    #[test]
    fn hygiene_fallback_in_real_pipeline() {
        // 宏模板中引用全局 +（经卫生重命名后回退解析）
        let src = r#"
            (define-syntax inc!
              (syntax-rules ()
                ((inc! v) (set! v (+ v 1)))))
            (define x 41)
            (inc! x)
            x
        "#;
        let o = run_source(src, "t.krf").unwrap();
        assert!(
            matches!(o.value, Value::Int(42)),
            "卫生回退应使宏内全局引用可达"
        );
    }
}
