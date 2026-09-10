//! 工具链生态接口预留（stage0.md §9.3.2/§9.3.3/§9.3.5/§9.3.6/§9.3.8/§9.3.9，
//! lang-design 13 §3.3.2/§3.3.3/§3.3.5/§3.3.6/§3.3.8/§3.3.9——next4 第八轮吸收，
//! r12 批次冻结）。
//!
//! **P0 三项**（数据结构位置必须 Stage 0 冻结——不预留 = 数月级破坏性重构）：
//! - LSP/IDE 查询：`LanguageService`（语法/语义查询——原则 14「工具即
//!   编译器」）+ `IncrementalAst`（AST 增量更新）；
//! - 调试信息：`DebugInfoGenerator`（IR↔源码映射、变量位置、DWARF/源映射）
//!   + `DebugTraceable`（IR 节点反向链接 + 调试名）；
//! - 增量编译查询：`QuerySystem` + `Query`（salsa 风格——纯函数查询 +
//!   依赖完整声明 + 按需失效）。
//!
//! **P1**：`CompilerService`（编译器即服务）+ `Serializable`（可序列化边界）。
//! **P2**：`PackageManager` + `ExternalModule`（包管理）；`AiAssistant`
//! （AI 辅助语义 API——复用 LanguageService 查询基建）。
//!
//! **既有数据结构位置对账（P0 前提条件）**：
//! - 每节点可追溯 Span ✓（CoreExpr/NodeMetadata/`debug_spans` 全管线）；
//! - 绑定可查询作用域 ✓（`IrGraph::NodeMetadata.scopes: ScopeSet`）；
//! - 变量稳定 ID ✓（`kerf_core::ir::NodeId`；字节码侧 proto 索引 + 槽位）；
//! - 函数边界唯一标识 ✓（`BcProgram::protos` 索引 = 原型稳定标识）；
//! - 调试帧槽 ✓（VM `ext3: Option<DebugFrameSlot>`，Stage 0 空形状）。
//!
//! 集成验证见 tests/v0/stage1/plan/reserved_ext_tests.rs。

use kerf_compiler::BcProgram;
use kerf_core::ir::NodeId;
use kerf_span::{Diagnostic, FileId, Span};
use kerf_syntax::Symbol;

// ---------------------------------------------------------------------------
// 共享形状类型（P3 冻结——Stage 1+ 可细化为富类型，签名兼容优先）
// ---------------------------------------------------------------------------

/// 源码位置（行/列，1 起算——LSP 协议口径）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

/// 源码位置区间（文件 + Span）——LSP `Location` 与调试回查共用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub file: FileId,
    pub span: Span,
}

/// 文本编辑（增量更新单元——LSP didChange → IncrementalAst::apply_edit）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub file: FileId,
    pub span: Span,
    pub new_text: String,
}

/// 语法树节点引用（P3 形状：NodeId 承载；Stage 1 细化为红绿树游标）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxNodeRef {
    pub node: NodeId,
}

/// 补全项（P3 形状：标签 + 插入文本）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub insert_text: String,
}

/// 类型信息（P3 形状：结构化描述串——Stage 1 接类型检查器富类型）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeInfo {
    pub description: String,
}

/// 代码动作（快速修复——P3 形状：标题 + 文本编辑）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAction {
    pub title: String,
    pub edit: TextEdit,
}

/// 工作区编辑（重命名结果——跨文件多编辑）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceEdit {
    pub edits: Vec<TextEdit>,
}

/// 重命名错误（P3 形状：原因串）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameError {
    pub reason: String,
}

/// 编辑应用错误（IncrementalAst::apply_edit 的失败面）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditError {
    pub reason: String,
}

/// 复用计划（增量重析的未变子树复用清单——P3 形状）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReusePlan {
    pub reusable_nodes: Vec<NodeId>,
}

/// 变量稳定 ID（调试信息用——区别于 de Bruijn 索引的稳定标识）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId(pub u32);

/// AST 节点稳定 ID（IR → AST 反向链接的 P3 形状）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AstNodeId(pub u32);

/// 目标码地址（字节码 pc / 未来本地码偏移——P3 形状）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Address(pub u32);

/// DWARF 调试段（P3 形状：字节信封——Stage 2 按 DWARF 标准具体化）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DwarfSections {
    pub bytes: Vec<u8>,
}

/// 源映射（P3 形状：字节信封——与 kerf-span SourceMap 的序列化形态衔接）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceMapBlob {
    pub bytes: Vec<u8>,
}

// ---------------------------------------------------------------------------
// 1. LSP/IDE 查询接口（P0，13 §3.3.2）
// ---------------------------------------------------------------------------

/// 编译器作为语言服务器的接口（Stage 2 实现基础 LSP 服务器）。
///
/// **预留契约**（不实现）：Stage 0 仅冻结查询形状——实现侧（rust-analyzer
/// 模式）消费与编译器相同的语法树/符号表/诊断流（原则 13/14）。
pub trait LanguageService {
    /// 语法查询：定位位置处的语法树节点。
    fn syntax_tree_at(&self, file: FileId, pos: Position) -> Option<SyntaxNodeRef>;

    /// 语法查询：补全项。
    fn completion_items(&self, file: FileId, pos: Position) -> Vec<CompletionItem>;

    /// 语法查询：文档符号（顶层绑定清单）。
    fn document_symbols(&self, file: FileId) -> Vec<Symbol>;

    /// 语义查询：定义位置（goto definition）。
    fn definition_of(&self, file: FileId, pos: Position) -> Option<Location>;

    /// 语义查询：引用清单（find references）。
    fn references_to(&self, file: FileId, pos: Position) -> Vec<Location>;

    /// 语义查询：位置处类型（hover——Stage 1 接类型检查器）。
    fn type_at(&self, file: FileId, pos: Position) -> Option<TypeInfo>;

    /// 诊断：当前诊断流。
    fn diagnostics(&self, file: FileId) -> Vec<Diagnostic>;

    /// 诊断：快速修复建议。
    fn quick_fixes(&self, file: FileId, pos: Position) -> Vec<CodeAction>;

    /// 重构：符号重命名（跨文件工作区编辑）。
    fn rename_symbol(
        &mut self,
        file: FileId,
        pos: Position,
        new_name: String,
    ) -> Result<WorkspaceEdit, RenameError>;
}

/// AST 增量更新接口（P0——LSP 按需重析与增量编译的 AST 侧挂点）。
pub trait IncrementalAst {
    /// 应用文本编辑（增量重析的入口）。
    fn apply_edit(&mut self, edit: &TextEdit) -> Result<(), EditError>;

    /// 失效区间（编辑影响范围标注）。
    fn invalidate_range(&mut self, range: Span);

    /// 复用计划：与另一版本比较，返回可复用的未变子树。
    fn reuse_unchanged(&self, other: &Self) -> ReusePlan;
}

// ---------------------------------------------------------------------------
// 2. 调试信息生成接口（P0，13 §3.3.3）
// ---------------------------------------------------------------------------

/// 调试信息生成器（Stage 2 生成源映射，Stage 2+ 生成 DWARF）。
///
/// **既有位置对账**：VM 侧 `debug_spans` + `debug_info_table`（字节码 pc →
/// Span 反查）已做实（堆栈追踪消费）；本 trait 把同一能力升格为标准接口。
pub trait DebugInfoGenerator {
    /// 将 IR 节点映射到源码位置。
    fn source_location_of(&self, ir_node: NodeId) -> Option<Span>;

    /// 将目标码地址映射回 IR 节点。
    fn ir_node_at(&self, code_addr: Address) -> Option<NodeId>;

    /// 变量的位置信息（在哪个寄存器/栈槽——调试器变量查看）。
    fn variable_location(&self, var: VarId, at_pc: Address) -> Option<Location>;

    /// 生成 DWARF 调试段。
    fn generate_dwarf(&self) -> DwarfSections;

    /// 生成源映射。
    fn generate_source_map(&self) -> SourceMapBlob;
}

/// IR 节点的调试可回溯性（每个 IR 节点实现）。
///
/// **预留契约**：AST↔IR 反向链接以 `AstNodeId` 稳定标识承载（P3 形状）；
/// 调试名区别于 de Bruijn 索引（`Option<Symbol>`——捕获/参数名）。
pub trait DebugTraceable {
    /// AST → IR 反向链接（从 IR 回溯到源码语法树）。
    fn ast_node_id(&self) -> AstNodeId;

    /// 变量的调试名称（而非仅 de Bruijn 索引）。
    fn debug_name(&self) -> Option<Symbol>;
}

// ---------------------------------------------------------------------------
// 3. 增量编译查询接口（P0，13 §3.3.5——与编译缓存构成数据面/架构面双预留）
// ---------------------------------------------------------------------------

/// 查询描述符（P3 形状：名称 + 输入指纹——salsa QueryDescriptor 的最小形状）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryDescriptor {
    /// 查询名（pass 名——Reader/Expand/Compile/Typecheck...）。
    pub name: &'static str,
    /// 输入指纹（内容寻址——与 CacheKey 口径一致）。
    pub input_fingerprint: u64,
}

/// 查询结果信封（P3 形状：指纹 + 缓存命中标记；Stage 1 具体化为类型化查询表）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryResult {
    /// 输出指纹（红绿测试的绿侧）。
    pub output_fingerprint: u64,
    /// 是否命中缓存（增量编译有效性度量）。
    pub cached: bool,
}

/// 查询系统：所有编译操作通过查询执行（rustc/Salsa 风格）。
///
/// **预留契约**：编译器的每个 pass 必须建模为查询；查询之间必须能声明
/// 依赖；查询结果必须可缓存/可失效（§13.1/15 §3.1 正文展开）。
pub trait QuerySystem {
    /// 输入：源文件（的内容寻址键——与 CacheKey 口径一致）。
    type Input;

    /// 执行查询（带缓存 + 依赖追踪）。
    fn query(&self, descriptor: &QueryDescriptor) -> QueryResult;

    /// 输入变化时失效相关缓存（依赖图反向传播）。
    fn invalidate(&mut self, input: Self::Input);

    /// 依赖追踪：查询 Q 依赖哪些其他查询。
    fn dependencies_of(&self, descriptor: &QueryDescriptor) -> Vec<QueryDescriptor>;
}

/// 单个查询（编译器的每个 pass 必须建模为查询）。
pub trait Query {
    type Input;
    type Output;

    /// 此查询依赖的其他查询（依赖必须完整声明）。
    fn dependencies(&self) -> Vec<QueryDescriptor>;

    /// 执行查询（纯函数——同输入必同输出）。
    ///
    /// 查询系统输入键与查询输入同型（内容寻址口径一致）。
    fn execute(&self, db: &dyn QuerySystem<Input = Self::Input>) -> Self::Output;
}

// ---------------------------------------------------------------------------
// 4. 编译器即服务接口（P1，13 §3.3.6）
// ---------------------------------------------------------------------------

/// 编译任务 ID（服务化提交凭证）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompileJobId(pub u64);

/// 编译状态（服务化生命周期）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// 编译目标（多目标口径——与 codegen::TargetTriple 衔接）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileTarget {
    /// 字节码（Stage 0 唯一执行后端）。
    Bytecode,
    /// 本地码（Stage 2 QBE/Cranelift——不入自举链）。
    Native,
    /// WebAssembly（Stage 2+ 组件模型）。
    Wasm,
}

/// 优化级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    None,
    Default,
    Aggressive,
}

/// 调试信息需求。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugInfoRequest {
    None,
    Spans,
    Full,
}

/// 编译请求（支持增量）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileRequest {
    /// 完整源码（P3 形状：String；Stage 2 增量形态 = 基线 + 编辑流）。
    pub source: String,
    /// 编译目标。
    pub target: CompileTarget,
    /// 优化级别。
    pub opt_level: OptLevel,
    /// 调试信息需求。
    pub debug_info: DebugInfoRequest,
}

/// 编译结果（P3 形状：字节码产物 + 诊断流——服务化返回面）。
#[derive(Debug, Clone)]
pub struct CompileResult {
    /// 字节码产物（Bytecode 目标时存在）。
    pub program: Option<BcProgram>,
    /// 编译诊断流（结构化——可流式输出）。
    pub diagnostics: Vec<Diagnostic>,
}

/// 编译器作为可远程调用的服务（P1——web Playground 即宿主侧雏形）。
pub trait CompilerService {
    /// 提交编译请求（异步——返回任务凭证）。
    fn submit(&mut self, request: CompileRequest) -> CompileJobId;

    /// 查询编译状态。
    fn status(&self, job: CompileJobId) -> Option<CompileStatus>;

    /// 获取编译结果（完成后）。
    fn result(&self, job: CompileJobId) -> Option<CompileResult>;

    /// 流式获取诊断信息（P3 形状：Vec 承载；Stage 2 细化为流）。
    fn diagnostics_stream(&self, job: CompileJobId) -> Vec<Diagnostic>;

    /// 取消编译（可中断性预留）。
    fn cancel(&mut self, job: CompileJobId);
}

/// 可序列化边界（编译器状态可传输的前提——P1）。
pub trait Serializable {
    fn serialize(&self) -> Vec<u8>;

    fn deserialize(data: &[u8]) -> Result<Self, DeserializeError>
    where
        Self: Sized;
}

/// 反序列化错误（P3 形状）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeserializeError {
    pub reason: String,
}

// ---------------------------------------------------------------------------
// 5. 包管理/依赖解析接口（P2，13 §3.3.8）
// ---------------------------------------------------------------------------

/// 包清单（P3 形状：名称 + 版本 + 依赖说明串）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub dependencies: Vec<String>,
}

/// 依赖图（P3 形状：拓扑序包名清单）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DependencyGraph {
    pub packages: Vec<String>,
}

/// 解出的包产物（P3 形状：可能预编译）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageArtifact {
    pub name: String,
    pub version: String,
    /// 预编译标记（外部模块消费路径分叉）。
    pub precompiled: bool,
}

/// 锁文件（P3 形状：文本承载——与 Cargo/Cargo.lock 同语义）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LockFile {
    pub content: String,
}

/// 依赖解析错误（P3 形状）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveError {
    pub reason: String,
}

/// 获取包失败（P3 形状）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchError {
    pub reason: String,
}

/// 模块来源（包名 + 路径——导入路径表示包依赖的预留位置）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleSource {
    pub package: String,
    pub path: String,
}

/// 包管理器与编译器的接口（P2——Stage 1 基础包解析）。
pub trait PackageManager {
    /// 解析依赖图。
    fn resolve_dependencies(
        &self,
        manifest: &PackageManifest,
    ) -> Result<DependencyGraph, ResolveError>;

    /// 获取包的编译产物。
    fn get_package(&self, name: &str, version: &str) -> Result<PackageArtifact, FetchError>;

    /// 锁定依赖版本。
    fn lock(&self, graph: &DependencyGraph) -> LockFile;
}

/// 外部模块（模块系统的包边界预留——P2）。
pub trait ExternalModule {
    /// 外部模块的来源（包名+路径）。
    fn source(&self) -> ModuleSource;

    /// 外部模块的编译产物（可能是预编译的）。
    fn compiled(&self) -> Option<BcProgram>;
}

// ---------------------------------------------------------------------------
// 6. AI 辅助编程接口（P2，13 §3.3.9——复用 LanguageService 查询基建）
// ---------------------------------------------------------------------------

/// 语义摘要（AI 上下文理解用——P3 形状：结构化文本）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticSummary {
    pub summary: String,
}

/// 函数签名（补全建议用——P3 形状：入参/出参/效应描述）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FunctionSignature {
    pub signature: String,
}

/// 快速类型检查结果（AI 生成代码验证——P3 形状）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TypeCheckResult {
    pub passed: bool,
    pub notes: Vec<String>,
}

/// 类型检查失败（P3 形状）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeError {
    pub reason: String,
}

/// 重构建议（AI 辅助重构——P3 形状）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refactoring {
    pub description: String,
}

/// 为 AI 编程助手提供的语义 API（P2）。
///
/// **预留契约**：复用 [`LanguageService`] 的查询基建（符号表/类型表/诊断流）
/// ——语义摘要、签名查询、快速类型检查、重构建议、文档注释生成。
pub trait AiAssistant {
    /// 获取代码的语义摘要（用于上下文理解）。
    fn semantic_summary(&self, file: FileId) -> SemanticSummary;

    /// 获取函数的输入/输出类型和效应（用于补全建议）。
    fn function_signature(&self, symbol: &Symbol) -> FunctionSignature;

    /// 验证 AI 生成的代码是否类型安全（快速检查）。
    fn quick_typecheck(&self, snippet: &str) -> Result<TypeCheckResult, TypeError>;

    /// 提供重构建议（AI 辅助重构）。
    fn refactoring_suggestions(&self, selection: &Span) -> Vec<Refactoring>;

    /// 生成代码的文档注释。
    fn generate_doc_comment(&self, item: &Symbol) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P0-P2 冻结性测试：全部预留 trait 以「测试实现体编译通过」证明契约
    /// 冻结（§27 接口稳定性；先例：reserved_signatures_are_frozen）。
    struct ProbeLangService;
    impl LanguageService for ProbeLangService {
        fn syntax_tree_at(&self, _file: FileId, _pos: Position) -> Option<SyntaxNodeRef> {
            None
        }
        fn completion_items(&self, _file: FileId, _pos: Position) -> Vec<CompletionItem> {
            vec![]
        }
        fn document_symbols(&self, _file: FileId) -> Vec<Symbol> {
            vec![]
        }
        fn definition_of(&self, _file: FileId, _pos: Position) -> Option<Location> {
            None
        }
        fn references_to(&self, _file: FileId, _pos: Position) -> Vec<Location> {
            vec![]
        }
        fn type_at(&self, _file: FileId, _pos: Position) -> Option<TypeInfo> {
            None
        }
        fn diagnostics(&self, _file: FileId) -> Vec<Diagnostic> {
            vec![]
        }
        fn quick_fixes(&self, _file: FileId, _pos: Position) -> Vec<CodeAction> {
            vec![]
        }
        fn rename_symbol(
            &mut self,
            _file: FileId,
            _pos: Position,
            _new_name: String,
        ) -> Result<WorkspaceEdit, RenameError> {
            Err(RenameError {
                reason: "P0 预留：rename Stage 2 实现".into(),
            })
        }
    }

    struct ProbeIncrementalAst {
        version: u32,
    }
    impl IncrementalAst for ProbeIncrementalAst {
        fn apply_edit(&mut self, _edit: &TextEdit) -> Result<(), EditError> {
            self.version += 1;
            Ok(())
        }
        fn invalidate_range(&mut self, _range: Span) {}
        fn reuse_unchanged(&self, _other: &Self) -> ReusePlan {
            ReusePlan::default()
        }
    }

    struct ProbeDebugGen;
    impl DebugInfoGenerator for ProbeDebugGen {
        fn source_location_of(&self, _ir_node: NodeId) -> Option<Span> {
            None
        }
        fn ir_node_at(&self, _code_addr: Address) -> Option<NodeId> {
            None
        }
        fn variable_location(&self, _var: VarId, _at_pc: Address) -> Option<Location> {
            None
        }
        fn generate_dwarf(&self) -> DwarfSections {
            DwarfSections::default()
        }
        fn generate_source_map(&self) -> SourceMapBlob {
            SourceMapBlob::default()
        }
    }

    struct ProbeIrNode;
    impl DebugTraceable for ProbeIrNode {
        fn ast_node_id(&self) -> AstNodeId {
            AstNodeId(0)
        }
        fn debug_name(&self) -> Option<Symbol> {
            None
        }
    }

    struct ProbeQuerySystem {
        inputs_invalidated: u32,
    }
    impl QuerySystem for ProbeQuerySystem {
        type Input = u64;
        fn query(&self, _descriptor: &QueryDescriptor) -> QueryResult {
            QueryResult {
                output_fingerprint: 0,
                cached: false,
            }
        }
        fn invalidate(&mut self, _input: Self::Input) {
            self.inputs_invalidated += 1;
        }
        fn dependencies_of(&self, _descriptor: &QueryDescriptor) -> Vec<QueryDescriptor> {
            vec![]
        }
    }

    struct ProbePass;
    impl Query for ProbePass {
        type Input = String;
        type Output = u64;
        fn dependencies(&self) -> Vec<QueryDescriptor> {
            vec![]
        }
        fn execute(&self, _db: &dyn QuerySystem<Input = String>) -> Self::Output {
            0
        }
    }

    struct ProbeService;
    impl CompilerService for ProbeService {
        fn submit(&mut self, _request: CompileRequest) -> CompileJobId {
            CompileJobId(1)
        }
        fn status(&self, _job: CompileJobId) -> Option<CompileStatus> {
            Some(CompileStatus::Queued)
        }
        fn result(&self, _job: CompileJobId) -> Option<CompileResult> {
            None
        }
        fn diagnostics_stream(&self, _job: CompileJobId) -> Vec<Diagnostic> {
            vec![]
        }
        fn cancel(&mut self, _job: CompileJobId) {}
    }

    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    struct ProbeState {
        n: u32,
    }
    impl Serializable for ProbeState {
        fn serialize(&self) -> Vec<u8> {
            self.n.to_le_bytes().to_vec()
        }
        fn deserialize(data: &[u8]) -> Result<Self, DeserializeError> {
            let bytes: [u8; 4] = data.try_into().map_err(|_| DeserializeError {
                reason: "长度".into(),
            })?;
            Ok(ProbeState {
                n: u32::from_le_bytes(bytes),
            })
        }
    }

    struct ProbePackageManager;
    impl PackageManager for ProbePackageManager {
        fn resolve_dependencies(
            &self,
            manifest: &PackageManifest,
        ) -> Result<DependencyGraph, ResolveError> {
            Ok(DependencyGraph {
                packages: vec![manifest.name.clone()],
            })
        }
        fn get_package(&self, name: &str, version: &str) -> Result<PackageArtifact, FetchError> {
            Ok(PackageArtifact {
                name: name.into(),
                version: version.into(),
                precompiled: false,
            })
        }
        fn lock(&self, _graph: &DependencyGraph) -> LockFile {
            LockFile::default()
        }
    }

    struct ProbeExternalModule;
    impl ExternalModule for ProbeExternalModule {
        fn source(&self) -> ModuleSource {
            ModuleSource {
                package: "kerf".into(),
                path: "lib".into(),
            }
        }
        fn compiled(&self) -> Option<BcProgram> {
            None
        }
    }

    struct ProbeAi;
    impl AiAssistant for ProbeAi {
        fn semantic_summary(&self, _file: FileId) -> SemanticSummary {
            SemanticSummary::default()
        }
        fn function_signature(&self, _symbol: &Symbol) -> FunctionSignature {
            FunctionSignature::default()
        }
        fn quick_typecheck(&self, snippet: &str) -> Result<TypeCheckResult, TypeError> {
            if snippet.is_empty() {
                Err(TypeError {
                    reason: "空片段".into(),
                })
            } else {
                Ok(TypeCheckResult::default())
            }
        }
        fn refactoring_suggestions(&self, _selection: &Span) -> Vec<Refactoring> {
            vec![]
        }
        fn generate_doc_comment(&self, _item: &Symbol) -> String {
            String::new()
        }
    }

    #[test]
    fn toolchain_reserved_signatures_are_frozen() {
        // 全部工具链预留 trait 可按冻结签名实现（编译通过 = 契约冻结）
        let mut ls = ProbeLangService;
        assert!(ls
            .rename_symbol(0, Position { line: 1, column: 1 }, "x".into())
            .is_err());
        let mut ia = ProbeIncrementalAst { version: 0 };
        ia.apply_edit(&TextEdit {
            file: 0,
            span: Span::dummy(),
            new_text: String::new(),
        })
        .unwrap();
        assert_eq!(ia.version, 1);
        let dg = ProbeDebugGen;
        assert_eq!(dg.generate_dwarf(), DwarfSections::default());
        let ir = ProbeIrNode;
        assert_eq!(ir.ast_node_id(), AstNodeId(0));
        let mut qs = ProbeQuerySystem {
            inputs_invalidated: 0,
        };
        qs.invalidate(42u64);
        assert_eq!(qs.inputs_invalidated, 1);
        let pass = ProbePass;
        assert_eq!(pass.dependencies().len(), 0);
        let mut svc = ProbeService;
        assert_eq!(
            svc.submit(CompileRequest {
                source: String::new(),
                target: CompileTarget::Bytecode,
                opt_level: OptLevel::None,
                debug_info: DebugInfoRequest::Spans,
            }),
            CompileJobId(1)
        );
        assert_eq!(ProbeState::deserialize(&3u32.to_le_bytes()).unwrap().n, 3);
        let pm = ProbePackageManager;
        assert!(pm.get_package("kerf", "0.1.0").is_ok());
        let em = ProbeExternalModule;
        assert!(em.compiled().is_none());
        let ai = ProbeAi;
        assert!(ai.quick_typecheck("").is_err());
    }

    #[test]
    fn query_descriptor_is_hashable_and_fingerprinted() {
        // 内容寻址口径：同输入指纹的描述符等价（依赖图节点身份）
        let d1 = QueryDescriptor {
            name: "expand",
            input_fingerprint: 42,
        };
        let d2 = QueryDescriptor {
            name: "expand",
            input_fingerprint: 42,
        };
        assert_eq!(d1, d2);
        let d3 = QueryDescriptor {
            name: "expand",
            input_fingerprint: 43,
        };
        assert_ne!(d1, d3);
        let d4 = QueryDescriptor {
            name: "compile",
            input_fingerprint: 42,
        };
        assert_ne!(d1, d4);
    }

    #[test]
    fn compile_status_lifecycle_shape_is_total() {
        // 服务化生命周期五态全可达（枚举穷尽 = 形状完备）
        let all = [
            CompileStatus::Queued,
            CompileStatus::Running,
            CompileStatus::Completed,
            CompileStatus::Failed,
            CompileStatus::Cancelled,
        ];
        assert_eq!(all.len(), 5);
        assert!(all.contains(&CompileStatus::Queued));
    }
}
