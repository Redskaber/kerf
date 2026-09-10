//! 驱动层：全管线编排（read → expand → lower → compile → run）。
//!
//! 入口（§10.1 规则 1 自由函数）：
//! - `compile_source`：源文本 → `CompileOutput`（完整管线含图 IR——
//!   dump/检查/CodeValue 消费方入口，TD-015 分流的完整出口）；
//! - `run_source`：编译前段 + VM 执行（生产路径，不构造图 IR）；
//! - `eval_source`：编译前段 + 元循环求值器执行（参考路径，同上）。
//!
//! 错误统一为 `DriverError`：阶段标识 + 已渲染诊断（含位置摘录，
//! §2.2 原则 16：人类可感知输出是一等公民）。

use std::rc::Rc;

use kerf_compiler::{compile_module, disassemble_program, BcProgram};
use kerf_core::{lower_program, CoreExpr, IrGraph};
use kerf_expander::{expand_program, phase::ModuleRegistry, ExpandCtxt, ExpandError};
use kerf_reader::{read_source, ReadError, TokenKind};
use kerf_runtime::Heap;
use kerf_span::{render_diagnostic, Diagnostic, DiagnosticCode, Severity, SourceMap, Span};
use kerf_syntax::{Stx, Symbol, SymbolTable};
use kerf_vm::{run_program, Env, Value, VmError};

use crate::builtins::{builtin_sigs, register_globals, resolve_hygiene_fallbacks};
use crate::cache::{cache_enabled, cache_key, with_cache};
use crate::capability::{verify_io_capabilities, IoGrant, IoRequirements};
use std::collections::HashMap;

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
        let mut diag = Diagnostic::error(Some(DiagnosticCode(4)), e.message.clone(), e.span);
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
/// TD-015 偿还：run/eval 生产路径消费本形态——`IrGraph` 仅在消费方
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
    /// 字节码程序。
    pub(crate) program: BcProgram,
    /// 源映射（诊断渲染）。
    pub(crate) source_map: SourceMap,
    /// 符号表（渲染与调试）。
    pub(crate) table: SymbolTable,
    /// 模块注册簿记（declare/visit 已完成）。
    pub(crate) registry: ModuleRegistry,
}

/// 编译前段（read → expand → 相位簿记 → 字节码，不含 lower）。
///
/// **读阶段经自举 Reader**（B3：kerf 源码 Reader 在 VM 上运行，
/// crate::bootstrap 桥接）——生产管线入口。自举 Reader 本身的编译走
/// 种子路径 [`compile_front_seed`]（无递归：种子编译自举实现）。
///
/// [`compile_source`]（完整管线，含图 IR）与 [`run_source`]/[`eval_source`]
/// （生产执行路径，无需 IR）共用本前段——单一编译逻辑，两分流出口
/// （字节码产物一致性由 `fast_path_bytecode_matches_full_compile` 守护）。
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

    front_from_forms(forms, table, sm, main_sym)
}

/// 种子编译前段（Rust Reader——自举 Reader 的引导实现与 parity oracle）。
///
/// 消费方：bootstrap 加载（编译 reader.krf）+ parity 测试对照基准。
/// 逻辑与 [`compile_front`] 完全同构——仅 read 入口为种子实现。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同上入口约定
pub(crate) fn compile_front_seed(source: &str, filename: &str) -> Result<FrontOutput, DriverError> {
    let mut sm = SourceMap::new();
    let file_id = sm.add_file(filename, source);
    let mut table = SymbolTable::new();
    let main_sym = table.intern("main");

    // 1. Reader（种子）：源 → Stx
    let forms =
        read_source(source, file_id, &mut table).map_err(|e| DriverError::from_read(&e, &sm))?;

    front_from_forms(forms, table, sm, main_sym)
}

/// 前段公共部分（read 之后：expand → 相位簿记 → 字节码）。
///
/// 两入口（自举/种子）共享——保证除 read 外的管线行为单一实现。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同上入口约定
fn front_from_forms(
    forms: Vec<Stx>,
    table: SymbolTable,
    sm: SourceMap,
    main_sym: Symbol,
) -> Result<FrontOutput, DriverError> {
    // 2. Expander：Stx → CoreExpr（visit = 变换器注册完成）
    let mut ectx = ExpandCtxt::new(table);
    let core = expand_program(&forms, &mut ectx).map_err(|e| DriverError::from_expand(&e, &sm))?;
    let table = ectx.table;

    // 2.5 R9 能力权限验证（r8——13 §3.1.3 条款 3「编译期错误」：
    // front 管线全路径生效 run/eval/check/compile；首个违规即阻断
    // fail-closed；E0006 家族与 E0005 静态检查分离）
    verify_io_capabilities(&core, &table, &sm)?;

    // 3. 相位簿记：declare + visit（§8.9 单模块生命周期）
    let mut registry = ModuleRegistry::new();
    let module_name = core
        .iter()
        .find_map(|e| match e.as_ref() {
            CoreExpr::Module { name, .. } => Some(*name),
            // _ 臂理由：非 module 顶形式不参与查找（find_map 取首个 module 形式）
            _ => None,
        })
        .unwrap_or(main_sym);
    let (imports, exports) = core
        .iter()
        .find_map(|e| match e.as_ref() {
            CoreExpr::Module {
                imports, exports, ..
            } => Some((imports.clone(), exports.clone())),
            // _ 臂理由：非 module 顶形式不参与查找（find_map 取首个 module 形式）
            _ => None,
        })
        .unwrap_or((vec![], vec![]));
    registry
        .declare(module_name, imports, exports)
        .map_err(|m| {
            let diag = Diagnostic::error(Some(DiagnosticCode(2)), m.clone(), Span::dummy());
            DriverError {
                stage: Stage::Expand,
                rendered: format!("[expand] {}", m),
                diagnostic: diag,
            }
        })?;
    registry.visit(module_name).map_err(|m| {
        let diag = Diagnostic::error(Some(DiagnosticCode(2)), m.clone(), Span::dummy());
        DriverError {
            stage: Stage::Expand,
            rendered: format!("[expand] {}", m),
            diagnostic: diag,
        }
    })?;

    // 4. Compile：CoreExpr 树 → 字节码（lower 仅完整入口执行——见下）
    let program = compile_module(&core).map_err(|e| DriverError::from_compile(&e, &sm))?;

    Ok(FrontOutput {
        core,
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
    let key = cache_key(source, filename);
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
}

/// 静态检查源文本：read → expand → compile（缓存路径）→ 保守类型
/// 检查（[kerf_compiler::check_program]——R1-R8 规则集；多错误全量
/// 收集，非短路）。
///
/// 编译期错误（Read/Expand/Compile 阶段）仍以 `DriverError` 返回——
/// 静态检查只对编译通过的程序进行（诊断链前后不交叉）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——同上入口约定
pub fn check_source(source: &str, filename: &str) -> Result<CheckReport, DriverError> {
    let (front, cache_hit) = compile_front_cached(source, filename)?;
    let io_req = IoRequirements::from_core(&front.core);
    let mut table = front.table;
    let sigs = builtin_sigs(&mut table);
    let diagnostics = kerf_compiler::check_program(&front.core, &sigs, &table);
    let rendered = diagnostics
        .iter()
        .map(|d| render_diagnostic(d, &front.source_map))
        .collect();
    Ok(CheckReport {
        proto_count: front.program.proto_count(),
        const_count: front.program.consts.len(),
        global_ref_count: front.program.global_refs.len(),
        instruction_count: front.program.total_instructions(),
        io_read: io_req.read,
        io_write: io_req.write,
        diagnostics,
        rendered,
        cache_hit,
    })
}

/// 编译源文本（全管线，不执行）。
///
/// 管线：read → expand（含相位 declare/visit 簿记）→ lower（图 IR）
/// → compile（字节码）。instantiate 簿记在执行入口（run/eval）完成。
///
/// 消费方：`kerf ir`/`code`/`bc`/`check` 等 dump 与检查子命令、测试、
/// CodeValue 检查——需要图 IR 或完整产物的场景。纯执行路径（run/eval）
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
    })
}

/// 运行产物：最终值 + 存活堆（值中的 `GcRef` 引用该堆——渲染需要堆存活）。
pub struct RunOutcome {
    /// 最终值。
    pub value: Value,
    /// 执行后的堆（GC 统计与值渲染使用）。
    pub heap: Heap,
}

impl std::fmt::Debug for RunOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunOutcome")
            .field("value", &self.value.type_name())
            .field("heap_slots", &self.heap.slot_count())
            .finish()
    }
}

/// 运行源文本（生产路径：编译前段 + VM 执行）。
///
/// 走 [`compile_front_cached`] 快路径——不构造图 IR（TD-015：执行路径
/// 无 IR 消费，旁路计算为恒定开销浪费）；同源重复执行命中编译缓存
/// （批次 C——内容寻址键，产物等价性由确定性编译保证）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——非性能热路径，错误体积可接受
pub fn run_source(source: &str, filename: &str) -> Result<RunOutcome, DriverError> {
    let (out, _cache_hit) = compile_front_cached(source, filename)?;
    let mut table = out.table;
    let grant = IoGrant::from_requirements(IoRequirements::from_core(&out.core));
    let mut globals = register_globals(&mut table, &grant);
    // 卫生回退解析（$hyg$N 后缀剥离）
    let program = out.program;
    let source_map = out.source_map;
    let registry = out.registry;
    resolve_hygiene_fallbacks(&program, &mut table, &mut globals);
    let mut heap = Heap::new();
    // instantiate 簿记（Phase 0 执行）
    instantiate_registry(registry);
    run_program(&program, &mut globals, &mut heap)
        .map(|value| RunOutcome { value, heap })
        .map_err(|e| DriverError::from_vm(&e, &source_map, &table))
}

/// 求值源文本（参考路径：编译前段 + 元循环求值器）。
///
/// 同 [`run_source`]：走 [`compile_front_cached`] 快路径，不构造图 IR
/// （TD-015）；与 VM 路径共享同一编译缓存（T1 双路径的产物同源）。
#[allow(clippy::result_large_err)] // 错误路径（含完整诊断结构）——非性能热路径，错误体积可接受
pub fn eval_source(source: &str, filename: &str) -> Result<RunOutcome, DriverError> {
    let (out, _cache_hit) = compile_front_cached(source, filename)?;
    let mut table = out.table;
    let grant = IoGrant::from_requirements(IoRequirements::from_core(&out.core));
    let globals = register_globals(&mut table, &grant);
    // eval 路径全局经根环境注入（宿主信任代码：内置名互不重复，
    // define 返回值在此无需检查——E6 只约束用户程序的同层重复定义）
    let root = Env::new();
    for (sym, v) in &globals {
        let _ = root.define(*sym, v.clone());
    }
    // 卫生回退解析（eval 路径与 VM 路径一致——T1 定理：宏引入的
    // 全局引用双路径必须同解；见 resolve_eval_hygiene_fallbacks）
    resolve_eval_hygiene_fallbacks(&out.core, &mut table, &globals, &root);
    let mut heap = Heap::new();
    heap.set_gc_enabled(false); // eval 路径不触发回收（根集不完整，TD-009）
    let registry = out.registry;
    instantiate_registry(registry);
    let core = out.core;
    let source_map = out.source_map;
    kerf_vm::eval_program(&core, &root, &mut heap)
        .map(|value| RunOutcome { value, heap })
        .map_err(|e| {
            DriverError::from_vm(
                &VmError {
                    message: e.message,
                    span: e.span,
                    trace: vec![],
                },
                &source_map,
                &table,
            )
        })
}

fn instantiate_registry(mut registry: ModuleRegistry) {
    for entry in registry.entries().to_vec() {
        let _ = registry.instantiate(entry.name);
    }
}

/// eval 路径的卫生回退解析（与 VM 路径的 `resolve_hygiene_fallbacks`
/// 语义对齐——T1 定理：宏引入的全局引用双路径一致解析）。
///
/// VM 路径在字节码 `global_refs` 上解析（run_source）；eval 路径在核心
/// 表达式树上解析：收集 VarRef/SetBang/Define 中的 `name$hyg$N` 符号
/// （与编译器 intern_global 口径一致），基名命中全局（内置）时在根
/// 环境预定义别名（宿主信任注入，不触发 E6——与 VM 路径静默别名
/// 行为一致）。
fn resolve_eval_hygiene_fallbacks(
    core: &[Rc<CoreExpr>],
    table: &mut SymbolTable,
    globals: &HashMap<Symbol, Value>,
    root: &Rc<Env>,
) {
    let mut refs: Vec<Symbol> = Vec::new();
    for e in core {
        collect_global_refs(e, &mut refs);
    }
    for sym in refs {
        if globals.contains_key(&sym) {
            continue;
        }
        let owned = table.name(sym).to_string();
        if let Some((base, suffix)) = owned.rsplit_once("$hyg$") {
            // 截取剩余后缀必须是数字（保守判定——与 VM 侧一致）
            if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
                let base_sym = table.intern(base);
                if let Some(v) = globals.get(&base_sym).cloned() {
                    let _ = root.define(sym, v);
                }
            }
        }
    }
}

/// 收集表达式树中的全局引用符号（VarRef + SetBang + Define 的名字，
/// 与编译器 intern_global 的收集口径一致——两路径回退语义互为镜像）。
fn collect_global_refs(e: &CoreExpr, out: &mut Vec<Symbol>) {
    match e {
        CoreExpr::VarRef { name, .. } => out.push(*name),
        CoreExpr::Lambda { body, .. } => collect_global_refs(body, out),
        CoreExpr::App { fn_expr, args, .. } => {
            collect_global_refs(fn_expr, out);
            for a in args {
                collect_global_refs(a, out);
            }
        }
        CoreExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_global_refs(cond, out);
            collect_global_refs(then_branch, out);
            collect_global_refs(else_branch, out);
        }
        CoreExpr::SetBang { name, value, .. } => {
            out.push(*name);
            collect_global_refs(value, out);
        }
        CoreExpr::Define { name, value, .. } => {
            out.push(*name);
            collect_global_refs(value, out);
        }
        CoreExpr::Begin { body, .. } => {
            for item in body {
                collect_global_refs(item, out);
            }
        }
        CoreExpr::Module { body, .. } => {
            for item in body {
                collect_global_refs(item, out);
            }
        }
        // _ 臂理由：require 零运行时语义（无全局引用——R9 静态门控）
        CoreExpr::Require { .. } | CoreExpr::Literal { .. } => {}
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
    fn eval_path_agrees_with_vm() {
        // 双执行路径互查（§21.8 Phase 1 核心验证）
        let src = "(define (f x) (+ x 1)) (f 41)";
        let a = run_source(src, "t.krf").unwrap();
        let b = eval_source(src, "t.krf").unwrap();
        assert!(a.value.eq_value(&b.value));
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
        // TD-015 分流守护：run/eval 快路径（compile_front）与完整编译
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
