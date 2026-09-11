//! 能力模型 I/O 基础传递（r8 批次 D——D2：13 §3.1.3 P2 规格做实，
//! 12-roadmap §2.4.5 Stage 1「基础能力（读写需授权）」）。
//!
//! **三层分工**（interface-contract-review F2 修复裁定）：
//! - `reserved.rs`：冻结契约（令牌类型 + `CapabilityIO` trait——签名
//!   权威）；
//! - 本模块：**铸造 + 实现 + 验证**——`IoRequirements`（声明提取）、
//!   `IoGrant`（令牌铸造，pub(crate) 构造面）、`StdCapabilityIO`
//!   （冻结 trait 的 stdio 实现）、`verify_io_capabilities`（R9 编译期
//!   权限验证，E0006）；
//! - `builtins.rs`：**消费**——I/O 内置函数捕获令牌（能力参数化形态，
//!   规格条款 4「driver 注册的内置函数改为能力参数化」）。
//!
//! **行为规格逐条锚**（13 §3.1.3）：
//! 1. `read_line` 仅在持有 `ReadCapability` 时可调用——令牌经线性传递
//!    （`RefCell` 借用期互斥 = 线性近似，F5），不可复制、不可伪造
//!    （私有构造 + pub(crate) 铸造）；
//! 2. `write_line` 同理；
//! 3. 无令牌的 I/O 调用为**编译期错误**（R9/E0006——front 管线全路径
//!    验证，非运行时检查）；
//! 4. 渐进替换：kerf 程序面的 I/O 全部经令牌；`kerf_runtime::io` 全局
//!    函数降为 OS 边界层（StdCapabilityIO 之下），Stage 2 语言级令牌
//!    值形态时完整退役。
//!
//! **R9 规则**（保守静态——零误报契约，与 R1-R8 同纪律）：
//! 程序中任何对门控内置名（read-line/read-int/read-num/read；print/
//! newline/write-string/write）的**引用**（VarRef 任意位置）都要求对应
//! `(require io ...)` 声明；**用户接管豁免**：程序自身 define/set! 该
//! 名字时视为用户全局（非内置使用，不标记——确定性边界，防误报）。
//! 卫生回退符号（`name$hyg$N`）按基名判定（与 driver 运行时回退同则）。

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use kerf_core::{Capability, CoreExpr};
use kerf_span::{render_diagnostic, Diagnostic, DiagnosticCode, SourceMap, Span};
use kerf_syntax::SymbolTable;

use crate::driver::{DriverError, Stage};
use crate::reserved::{
    mint_read_token, mint_write_token, CapabilityIO, IOError, ReadCapability, WriteCapability,
};

/// E0006 诊断码（结构化阶段码序列 E1-E4/E0005 之后——能力权限族）。
pub(crate) const IO_PERMISSION_CODE: DiagnosticCode = DiagnosticCode(6);

/// 读门控内置名（R9 数据驱动表——read 能力覆盖面）。
pub(crate) const READ_GATED: &[&str] = &["read-line", "read-int", "read-num"];

/// 写门控内置名（R9 数据驱动表——write 能力覆盖面）。
pub(crate) const WRITE_GATED: &[&str] = &["print", "newline", "write-string"];

/// 程序声明的 I/O 能力需求集合（从 `CoreExpr::Require` 提取）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IoRequirements {
    /// stdin 读需求（(require io read)）。
    pub read: bool,
    /// stdout 写需求（(require io write)）。
    pub write: bool,
}

impl IoRequirements {
    /// 从核心表达式序列提取（Require 形式全量收集——幂等集合语义）。
    pub fn from_core(core: &[Rc<CoreExpr>]) -> Self {
        let mut req = IoRequirements::default();
        for e in core {
            collect_requirements(e, &mut req);
        }
        req
    }

    /// 是否为空声明。
    pub fn is_empty(self) -> bool {
        !self.read && !self.write
    }

    /// 渲染（CLI/CheckReport 观测面）。
    pub fn render(self) -> String {
        let mut parts = Vec::new();
        if self.read {
            parts.push("io:read");
        }
        if self.write {
            parts.push("io:write");
        }
        if parts.is_empty() {
            "（未声明）".to_string()
        } else {
            parts.join(" ")
        }
    }
}

/// 表达式树遍历收集 Require 声明。
fn collect_requirements(e: &CoreExpr, req: &mut IoRequirements) {
    match e {
        CoreExpr::Require { caps, .. } => {
            for c in caps {
                match c {
                    Capability::IoRead => req.read = true,
                    Capability::IoWrite => req.write = true,
                }
            }
        }
        CoreExpr::Lambda { body, .. } => collect_requirements(body, req),
        CoreExpr::App { fn_expr, args, .. } => {
            collect_requirements(fn_expr, req);
            for a in args {
                collect_requirements(a, req);
            }
        }
        CoreExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_requirements(cond, req);
            collect_requirements(then_branch, req);
            collect_requirements(else_branch, req);
        }
        CoreExpr::SetBang { value, .. } => collect_requirements(value, req),
        CoreExpr::Define { value, .. } => collect_requirements(value, req),
        CoreExpr::Begin { body, .. } | CoreExpr::Module { body, .. } => {
            for item in body {
                collect_requirements(item, req);
            }
        }
        CoreExpr::VarRef { .. } | CoreExpr::Literal { .. } => {}
    }
}

/// 已授权令牌集（driver 组合根铸造——按程序声明的需求）。
///
/// 令牌的线性语义：`Rc<RefCell<令牌>>` 承载（F5 裁定——`Rc<dyn Fn>`
/// 共享捕获下的线性近似：构造不可伪造严格保持；`borrow_mut` 互斥 =
/// 调用期独占）。Stage 2 令牌值化时随语言线性类型消除本近似。
pub struct IoGrant {
    read: Option<Rc<RefCell<ReadCapability>>>,
    write: Option<Rc<RefCell<WriteCapability>>>,
}

impl IoGrant {
    /// 按需求铸造（需求 → 令牌一一对应——声明面即授权面，编译期 R9
    /// 已保证一致）。
    pub(crate) fn from_requirements(req: IoRequirements) -> Self {
        IoGrant {
            read: req.read.then(|| Rc::new(RefCell::new(mint_read_token()))),
            write: req.write.then(|| Rc::new(RefCell::new(mint_write_token()))),
        }
    }

    /// 空授权（无声明程序——I/O 内置不注册，fail-closed）。
    ///
    /// 公共构造（r14）：宿主/测试侧构造「无能力」全局环境的合法入口
    /// （E1-α 自举 Expander 行为面测试消费；§11 接口最小放宽——仅此
    /// 一个构造器，令牌铸造面保持 crate 私有）。
    pub fn none() -> Self {
        IoGrant {
            read: None,
            write: None,
        }
    }

    /// 读令牌句柄（注册面消费——None = 未授权不注册读内置）。
    pub(crate) fn read_handle(&self) -> Option<Rc<RefCell<ReadCapability>>> {
        self.read.clone()
    }

    /// 写令牌句柄（同上）。
    pub(crate) fn write_handle(&self) -> Option<Rc<RefCell<WriteCapability>>> {
        self.write.clone()
    }

    /// 授权面回读（观测/测试）。
    pub fn requirements(&self) -> IoRequirements {
        IoRequirements {
            read: self.read.is_some(),
            write: self.write.is_some(),
        }
    }
}

/// stdio 能力实现（冻结契约 `CapabilityIO` 的 Stage 1 基础做实）。
///
/// EOF 约定：`read_line` 于 stdin EOF 返回 `Err(IOError{message:"EOF"})`
/// （错误结构最小形态——冻结签名只有 message；内置适配层按本约定映射
/// nil，与 Stage 0 read-line 行为一致）。
pub struct StdCapabilityIO;

impl CapabilityIO for StdCapabilityIO {
    fn read_line(_cap: &mut ReadCapability) -> Result<String, IOError> {
        // 令牌在签名中线性占用——权限已由构造面保证（编译期 R9 + 铸造）
        match kerf_runtime::read_line_stdin() {
            Ok(Some(s)) => Ok(s),
            Ok(None) => Err(IOError {
                message: "EOF".to_string(),
            }),
            Err(e) => Err(IOError { message: e.message }),
        }
    }

    fn write_line(_cap: &mut WriteCapability, s: &str) -> Result<(), IOError> {
        kerf_runtime::write_line_stdout(s).map_err(|e| IOError { message: e.message })
    }
}

// ---------------------------------------------------------------------------
// R9：编译期权限验证（E0006——front 管线全路径）
// ---------------------------------------------------------------------------

/// 门控内置名 → 所需能力（数据驱动表——「任意流程节点」扩展面：
/// Stage 2 新能力族增行即可）。
fn required_capability(builtin_name: &str) -> Option<Capability> {
    if READ_GATED.contains(&builtin_name) {
        Some(Capability::IoRead)
    } else if WRITE_GATED.contains(&builtin_name) {
        Some(Capability::IoWrite)
    } else {
        None
    }
}

/// 卫生基名解析（`name$hyg$N` → `name`；与 driver 运行时回退同则——
/// 保守判定：后缀须非空纯数字）。
fn hygienic_base(name: &str) -> &str {
    match name.rsplit_once("$hyg$") {
        Some((base, suffix))
            if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) =>
        {
            base
        }
        _ => name,
    }
}

/// R9 权限验证：程序中所有门控内置引用都有对应声明（13 §3.1.3 条款 3
/// ——编译期错误，非运行时检查）。
///
/// **用户接管豁免**：程序 define/set! 同名符号 → 视为用户全局（非内置
/// 使用）——零误报纪律（与 typecheck 保守性契约一致）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——driver 入口既有约定
pub(crate) fn verify_io_capabilities(
    core: &[Rc<CoreExpr>],
    table: &SymbolTable,
    sm: &SourceMap,
) -> Result<(), DriverError> {
    let declared = IoRequirements::from_core(core);
    // 用户接管集合（define/set! 的名字——遍历口径与声明收集一致）
    let mut takeover: HashSet<String> = HashSet::new();
    for e in core {
        collect_takeover(e, table, &mut takeover);
    }
    // 门控引用检查（首个违规即报——多错误收集属 E0005 家族职责，
    // 权限违规单条即阻断：fail-closed）
    for e in core {
        if let Some(diag) = verify_refs(e, &declared, &takeover, table) {
            return Err(DriverError {
                stage: Stage::Compile,
                rendered: render_diagnostic(&diag, sm),
                diagnostic: diag,
            });
        }
    }
    Ok(())
}

/// 引用遍历验证（返回首个违规诊断）。
fn verify_refs(
    e: &CoreExpr,
    declared: &IoRequirements,
    takeover: &HashSet<String>,
    table: &SymbolTable,
) -> Option<Diagnostic> {
    match e {
        CoreExpr::VarRef { name, span, .. } => {
            let raw = table.name(*name).to_string();
            let base = hygienic_base(&raw).to_string();
            if takeover.contains(&raw) || takeover.contains(&base) {
                return None;
            }
            match required_capability(&base) {
                Some(Capability::IoRead) if !declared.read => {
                    Some(permission_diag(&base, "read", "(require io read)", *span))
                }
                Some(Capability::IoWrite) if !declared.write => {
                    Some(permission_diag(&base, "write", "(require io write)", *span))
                }
                _ => None,
            }
        }
        CoreExpr::Lambda { body, .. } => verify_refs(body, declared, takeover, table),
        CoreExpr::App { fn_expr, args, .. } => verify_refs(fn_expr, declared, takeover, table)
            .or_else(|| {
                args.iter()
                    .find_map(|a| verify_refs(a, declared, takeover, table))
            }),
        CoreExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => verify_refs(cond, declared, takeover, table)
            .or_else(|| verify_refs(then_branch, declared, takeover, table))
            .or_else(|| verify_refs(else_branch, declared, takeover, table)),
        CoreExpr::SetBang {
            name, value, span, ..
        } => {
            // set! 目标：若名字是门控内置且未被接管 → 内置状态写入（无此
            // 形态——门控内置无 set! 语义）；此处按引用口径检查值侧
            let _ = (name, span);
            verify_refs(value, declared, takeover, table)
        }
        CoreExpr::Define { name, value, span } => {
            // define 的名字在接管豁免集（takeover 收集阶段）；值侧检查
            let _ = span;
            let _ = name;
            verify_refs(value, declared, takeover, table)
        }
        CoreExpr::Begin { body, .. } | CoreExpr::Module { body, .. } => body
            .iter()
            .find_map(|item| verify_refs(item, declared, takeover, table)),
        CoreExpr::Require { .. } | CoreExpr::Literal { .. } => None,
    }
}

/// 权限诊断构造。
fn permission_diag(builtin: &str, cap: &str, declare_hint: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        Some(IO_PERMISSION_CODE),
        format!(
            "能力权限不足：「{}」需要 {} 能力——程序未声明 {}（无令牌的 I/O 调用为编译期错误）",
            builtin, cap, declare_hint
        ),
        span,
    )
}

/// 用户接管名收集（define/set! 的名字——含卫生基名两形）。
fn collect_takeover(e: &CoreExpr, table: &SymbolTable, out: &mut HashSet<String>) {
    match e {
        CoreExpr::Define { name, value, .. } => {
            out.insert(table.name(*name).to_string());
            collect_takeover(value, table, out);
        }
        CoreExpr::SetBang { name, value, .. } => {
            out.insert(table.name(*name).to_string());
            collect_takeover(value, table, out);
        }
        CoreExpr::Lambda { body, .. } => collect_takeover(body, table, out),
        CoreExpr::App { fn_expr, args, .. } => {
            collect_takeover(fn_expr, table, out);
            for a in args {
                collect_takeover(a, table, out);
            }
        }
        CoreExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_takeover(cond, table, out);
            collect_takeover(then_branch, table, out);
            collect_takeover(else_branch, table, out);
        }
        CoreExpr::Begin { body, .. } | CoreExpr::Module { body, .. } => {
            for item in body {
                collect_takeover(item, table, out);
            }
        }
        CoreExpr::Require { .. } | CoreExpr::VarRef { .. } | CoreExpr::Literal { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_with(src: &str) -> SymbolTable {
        let mut t = SymbolTable::new();
        t.intern(src);
        t
    }

    #[test]
    fn requirements_extracted_from_require_forms() {
        let req_sym = "read";
        let mut t = SymbolTable::new();
        let r = t.intern(req_sym);
        let core = vec![Rc::new(CoreExpr::Require {
            caps: vec![Capability::IoRead],
            span: Span::dummy(),
        })];
        let req = IoRequirements::from_core(&core);
        assert!(req.read && !req.write);
        let _ = r;
    }

    #[test]
    fn requirements_nested_in_module() {
        let core = vec![Rc::new(CoreExpr::Module {
            name: SymbolTable::new().intern("m"),
            imports: vec![],
            exports: vec![],
            body: vec![Rc::new(CoreExpr::Require {
                caps: vec![Capability::IoRead, Capability::IoWrite],
                span: Span::dummy(),
            })],
            span: Span::dummy(),
        })];
        let req = IoRequirements::from_core(&core);
        assert!(req.read && req.write);
    }

    #[test]
    fn undeclared_print_reference_is_e0006() {
        let src = "(print 42)";
        let mut sm = SourceMap::new();
        let fid = sm.add_file("t.krf", src);
        let mut t = SymbolTable::new();
        let print = t.intern("print");
        let core = vec![Rc::new(CoreExpr::VarRef {
            name: print,
            scopes: kerf_syntax::ScopeSet::new(),
            span: Span::new(fid, 1, 6),
        })];
        let err = verify_io_capabilities(&core, &t, &sm).unwrap_err();
        assert_eq!(err.stage, Stage::Compile);
        assert_eq!(err.diagnostic.code, Some(DiagnosticCode(6)));
        assert!(err.rendered.contains("print"));
        assert!(err.rendered.contains("(require io write)"));
        assert!(err.rendered.contains("error[E0006]"));
    }

    #[test]
    fn declared_print_reference_passes() {
        let mut t = SymbolTable::new();
        let print = t.intern("print");
        let core = vec![
            Rc::new(CoreExpr::Require {
                caps: vec![Capability::IoWrite],
                span: Span::dummy(),
            }),
            Rc::new(CoreExpr::VarRef {
                name: print,
                scopes: kerf_syntax::ScopeSet::new(),
                span: Span::dummy(),
            }),
        ];
        let sm = SourceMap::new();
        assert!(verify_io_capabilities(&core, &t, &sm).is_ok());
    }

    #[test]
    fn read_builtin_needs_read_not_write() {
        let mut t = SymbolTable::new();
        let rl = t.intern("read-line");
        let core = vec![
            Rc::new(CoreExpr::Require {
                caps: vec![Capability::IoWrite],
                span: Span::dummy(),
            }),
            Rc::new(CoreExpr::VarRef {
                name: rl,
                scopes: kerf_syntax::ScopeSet::new(),
                span: Span::dummy(),
            }),
        ];
        let sm = SourceMap::new();
        let err = verify_io_capabilities(&core, &t, &sm).unwrap_err();
        assert!(err.rendered.contains("(require io read)"));
    }

    #[test]
    fn user_takeover_exempts_shadowed_builtin() {
        let mut t = SymbolTable::new();
        let print = t.intern("print");
        // (define print 5) + print 引用——用户接管豁免（零误报纪律）
        let core = vec![
            Rc::new(CoreExpr::Define {
                name: print,
                value: Rc::new(CoreExpr::Literal {
                    value: kerf_core::LiteralValue::Int(5),
                    span: Span::dummy(),
                }),
                span: Span::dummy(),
            }),
            Rc::new(CoreExpr::VarRef {
                name: print,
                scopes: kerf_syntax::ScopeSet::new(),
                span: Span::dummy(),
            }),
        ];
        let sm = SourceMap::new();
        assert!(verify_io_capabilities(&core, &t, &sm).is_ok());
    }

    #[test]
    fn alias_of_builtin_still_gated() {
        let mut t = SymbolTable::new();
        let print = t.intern("print");
        let my = t.intern("my-print");
        // (define my-print print)——值侧 VarRef print 未被接管（接管的是
        // my-print）→ 正确标记（别名确实使用内置）
        let core = vec![Rc::new(CoreExpr::Define {
            name: my,
            value: Rc::new(CoreExpr::VarRef {
                name: print,
                scopes: kerf_syntax::ScopeSet::new(),
                span: Span::dummy(),
            }),
            span: Span::dummy(),
        })];
        let sm = SourceMap::new();
        let err = verify_io_capabilities(&core, &t, &sm).unwrap_err();
        assert!(err.rendered.contains("print"));
    }

    #[test]
    fn hygienic_base_resolves_gated_reference() {
        let mut t = SymbolTable::new();
        let print_hyg = t.intern("print$hyg$3");
        let core = vec![Rc::new(CoreExpr::VarRef {
            name: print_hyg,
            scopes: kerf_syntax::ScopeSet::new(),
            span: Span::dummy(),
        })];
        let sm = SourceMap::new();
        let err = verify_io_capabilities(&core, &t, &sm).unwrap_err();
        assert!(err.rendered.contains("print"));
        // 非数字后缀不剥离（保守判定——与运行时回退同则）
        let mut t2 = SymbolTable::new();
        let not_hyg = t2.intern("print$hyg$x");
        let core2 = vec![Rc::new(CoreExpr::VarRef {
            name: not_hyg,
            scopes: kerf_syntax::ScopeSet::new(),
            span: Span::dummy(),
        })];
        let sm2 = SourceMap::new();
        assert!(verify_io_capabilities(&core2, &t2, &sm2).is_ok());
    }

    #[test]
    fn hygienic_base_rule() {
        assert_eq!(hygienic_base("plain"), "plain");
        assert_eq!(hygienic_base("print$hyg$3"), "print");
        assert_eq!(hygienic_base("print$hyg$x"), "print$hyg$x");
        assert_eq!(hygienic_base("print$hyg$"), "print$hyg$");
    }

    #[test]
    fn requirements_render() {
        assert_eq!(
            IoRequirements {
                read: true,
                write: false
            }
            .render(),
            "io:read"
        );
        assert_eq!(IoRequirements::default().render(), "（未声明）");
    }

    #[test]
    fn grant_mints_only_declared() {
        let g = IoGrant::from_requirements(IoRequirements {
            read: true,
            write: false,
        });
        assert!(g.read_handle().is_some());
        assert!(g.write_handle().is_none());
        assert_eq!(g.requirements().render(), "io:read");
        let none = IoGrant::none();
        assert!(none.read_handle().is_none() && none.write_handle().is_none());
    }

    #[test]
    fn std_io_write_line_roundtrip() {
        let mut cap = mint_write_token();
        assert!(<StdCapabilityIO as CapabilityIO>::write_line(&mut cap, "").is_ok());
    }

    #[test]
    fn gated_tables_are_disjoint() {
        for r in READ_GATED {
            assert!(!WRITE_GATED.contains(r));
            assert_eq!(required_capability(r), Some(Capability::IoRead));
        }
        for w in WRITE_GATED {
            assert_eq!(required_capability(w), Some(Capability::IoWrite));
        }
        assert_eq!(required_capability("cons"), None);
        let _ = table_with("x");
    }
}
