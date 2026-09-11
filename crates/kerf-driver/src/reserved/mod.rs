//! 接口预留层（stage0.md §9 / lang-design 13 §3.1 + §3.3-§3.5；v6.1 完整性
//! 审查后 14 项）。
//!
//! **冻结契约，不写实现**——七个子模块按能力族分文件组织（r18 / 40-g
//! 拆分：J1 对齐 13 §3.1 分节结构；本文件仅承载模块声明与 re-export）：
//!
//! | 子模块 | 等级 | 内容 |
//! |---|---|---|
//! | [`effect_handlers`] | P3 | Effect Handlers（`EffectFamily`/`Effect`/`EffectSystem`——仅类型形状） |
//! | [`multistage`] | P3 | 多阶段编程（`MultiStage`——quote/splice/run） |
//! | [`capability_io`] | P2 | 能力模型 I/O（`ReadCapability`/`WriteCapability`/`CapabilityIO` + 完整行为规格） |
//! | [`compilation_cache`] | P2 | 编译缓存（`CacheKey`/`CachedResult`/`CompilationCache` + 完整行为规格） |
//! | [`toolchain`] | P0/P1/P2 | 工具链生态（LSP/调试/增量查询 + 服务化 + 包管理/AI） |
//! | [`ffi`] | P1 | FFI 边界（`ExternalType`/`FfiCall`/`FfiBoundary`——GC pin/unpin 协议） |
//! | [`codegen`] | P1 | 多目标后端（`CodegenBackend`/`WasmBackend`——LLVM 永不入自举链） |
//!
//! **契约与实现的模块分离形态**（40-g 标准化注记）：P2 两项的做实
//! 消费面 = `crate::capability`（r8 能力门控）与 `crate::cache`（r7
//! 查询式缓存）；P3 效应的消费面 = `crate::effects`（r8 金属层逃逸）。
//! 实现深化按「渐进替换原则」在各自实现模块演进，契约签名不变。
//!
//! **效应扩展与能力委托**（13 §3.3.10 裁定）：不新增数据结构位置——分布式
//! 效应经 `Effect` 关联类型的自定义效应族扩展（Stage 3+）；能力委托/组合
//! 以库形态叠加于本模块令牌类型之上（Stage 1+），签名不变。
//!
//! **接口稳定性原则**（§2.2 原则 27）：本模块的全部签名在整个生命周期内
//! 保持向后兼容——演进只替换实现，不破坏契约（re-export 路径恒有效）。
//!
//! **渐进替换原则**（§2.2 原则 28）：元循环求值器 → 编译器、传统 I/O →
//! 能力模型、传统闭包 → Effect Handlers——每条替换路径的挂点在此冻结。
//!
//! **预留留白原则**（§2.2 原则 32，v6.1）：预留的本质是「为未来留出空间」
//! 而非「提前实现」——要求数据结构与类型定义的兼容性，而非功能的完整性。
//! 全部预留以 Probe 实现体编译通过证明契约冻结（测试锚点：各子模块
//! `*_signature_is_frozen` 系）。

mod capability_io;
mod codegen;
mod compilation_cache;
mod effect_handlers;
mod ffi;
mod multistage;
mod toolchain;

pub use capability_io::{CapabilityIO, IOError, ReadCapability, WriteCapability};
// 令牌铸造仅 crate 内可达（driver 组合根流出——构造面控制，
// r8 批次 D 裁定维持）
pub(crate) use capability_io::{mint_read_token, mint_write_token};
pub use codegen::{
    AnnotatedANF, CodegenBackend, CodegenError, CompiledModule, PassDescriptor, TargetTriple,
    WasmBackend, WasmComponent,
};
pub use compilation_cache::{CacheKey, CachedResult, CompilationCache};
pub use effect_handlers::{Effect, EffectFamily, EffectSystem};
pub use ffi::{CIntSize, ExternalType, FfiBoundary, FfiCall, PointeeType};
pub use multistage::MultiStage;
pub use toolchain::{
    Address, AiAssistant, AstNodeId, CodeAction, CompileJobId, CompileRequest, CompileResult,
    CompileStatus, CompileTarget, CompilerService, CompletionItem, DebugInfoGenerator,
    DebugInfoRequest, DebugTraceable, DependencyGraph, DeserializeError, DwarfSections, EditError,
    ExternalModule, FetchError, FunctionSignature, IncrementalAst, LanguageService, Location,
    LockFile, ModuleSource, OptLevel, PackageArtifact, PackageManager, PackageManifest, Position,
    Query, QueryDescriptor, QueryResult, QuerySystem, Refactoring, RenameError, ResolveError,
    ReusePlan, SemanticSummary, Serializable, SourceMapBlob, SyntaxNodeRef, TextEdit,
    TypeCheckResult, TypeError, TypeInfo, VarId, WorkspaceEdit,
};
