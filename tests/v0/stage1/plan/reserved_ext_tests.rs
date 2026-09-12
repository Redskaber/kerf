//! 接口预留扩展层集成测试（r12 批次 / worklog Task 31-d）。
//!
//! **设计锚点**：lang-design 13-capability-matrix §3.3-§3.5（v6.1——
//! next4.md 第八轮「2026 接口预留完整性审查」吸收）+ §3.3 测试锚点注记。
//!
//! **验证面**（P0 数据结构位置的跨 crate 断言 + 预留 API 的外部可达性）：
//! 1. P0 LSP 前提：每节点可追溯 Span（CoreExpr::span()）+ 绑定可查询作用域
//!    （NodeMetadata.scopes）；
//! 2. P0 调试前提：变量稳定 ID（ir::NodeId）+ IR→源码回查（NodeMetadata.span）
//!    + 函数边界标识（BcProto/debug_spans）+ VM 调试帧槽（ext3）；
//! 3. P0 增量查询前提：内容寻址口径一致（QueryDescriptor ↔ CacheKey）；
//! 4. 预留层 14 项 API 自 driver 外部可达（路径 + 形状行为）；
//! 5. FFI 自举合规：预留形状不触碰自举链（无 Reader/Expander 依赖——
//!    以「FFI 类型仅存在于 reserved 模块」的路径断言近似）。

use std::rc::Rc;

use kerf_compiler::BcProgram;
use kerf_core::ir::NodeId;
use kerf_core::{CoreExpr, LiteralValue};
use kerf_driver::reserved::{
    AiAssistant, AnnotatedANF, CodegenBackend, CompileRequest, CompileStatus, CompileTarget,
    CompilerService, DebugInfoGenerator, DebugInfoRequest, DebugTraceable, ExternalType,
    FfiBoundary, FfiCall, IncrementalAst, LanguageService, OptLevel, PackageManager, Position,
    QueryDescriptor, QueryResult, QuerySystem, TargetTriple, TextEdit, TypeCheckResult, VarId,
};
use kerf_span::Span;
use kerf_syntax::Symbol;
use kerf_vm::Value;

// ---------------------------------------------------------------------------
// P0 数据结构位置断言（不预留 = 后期破坏性重构——13 §3.3.2/§3.3.3）
// ---------------------------------------------------------------------------

#[test]
fn p0_lsp_position_every_core_node_carries_span() {
    // 前提 1：每个节点必须可追溯 Span（LanguageService 的定位查询地基）
    let lit = CoreExpr::Literal {
        value: LiteralValue::Int(1),
        span: Span::dummy(),
    };
    let vr = CoreExpr::Var {
        name: Symbol(0),
        scopes: kerf_syntax::ScopeSet::new(),
        span: Span::dummy(),
    };
    let app = CoreExpr::Apply {
        fn_expr: Rc::new(lit.clone()),
        args: vec![Rc::new(vr)],
        span: Span::dummy(),
    };
    for e in [&lit, &app] {
        assert_eq!(e.span(), Span::dummy());
    }
}

#[test]
fn p0_debug_position_stable_node_id_and_scoped_metadata() {
    // 前提 2：变量稳定 ID（ir::NodeId）+ 绑定可查询作用域
    // （NodeMetadata.scopes: ScopeSet——LSP 绑定查询 + 调试回查共用）
    let id: NodeId = 7;
    assert_eq!(id, 7);
    // NodeMetadata 形状：span + scopes 公共可达（13 §3.3.2 前提 2 的
    // 「需预留」在 r5-r8 已做实——此处冻结其公共形状）
    let md = kerf_core::NodeMetadata {
        span: Span::dummy(),
        scopes: kerf_syntax::ScopeSet::new(),
    };
    assert!(md.scopes.is_empty());
}

#[test]
fn p0_debug_position_bytecode_debug_surface() {
    // 前提 3：函数边界唯一标识（BcProgram::protos 索引）+ 字节码 pc →
    // Span 反查表（debug_spans）+ VM 调试帧槽（ext3）
    let program = BcProgram {
        protos: vec![],
        consts: vec![],
        entry: 0,
        global_refs: vec![],
        module_name: None,
    };
    assert!(program.protos.is_empty());
    // ext3 调试帧槽（kerf-vm）：Stage 0 空形状、位置冻结
    let ext = kerf_vm::FrameExt::default();
    assert!(ext.ext3.is_none());
}

#[test]
fn p0_query_position_content_addressing_unified() {
    // 前提 4：增量查询与编译缓存共用内容寻址口径（数据面/架构面双预留
    // 一致性——13 §3.3.5 与 §3.1.4 的键口径相同：u64 指纹）
    let qd = QueryDescriptor {
        name: "compile",
        input_fingerprint: 42,
    };
    let ck = kerf_driver::CacheKey {
        source_hash: 42,
        config_fingerprint: 1,
    };
    assert_eq!(qd.input_fingerprint, ck.source_hash);
}

// ---------------------------------------------------------------------------
// 预留层外部可达性 + 形状行为（跨 crate 消费面）
// ---------------------------------------------------------------------------

#[test]
fn reserved_layer_api_reachable_from_outside() {
    // 14 项预留的 API 自 driver 外部经 reserved:: 路径可达（pub 模块面）
    let et = ExternalType::CInt(kerf_driver::reserved::CIntSize::I64);
    assert_eq!(et, ExternalType::CInt(kerf_driver::reserved::CIntSize::I64));
    let call = FfiCall::AllocExternal { size: 16 };
    assert_eq!(call, FfiCall::AllocExternal { size: 16 });
    let triple = TargetTriple("wasm32-wasip2".into());
    assert_eq!(triple.0, "wasm32-wasip2");
    let ir = AnnotatedANF::default();
    assert_eq!(ir.fingerprint, 0);
}

#[test]
fn language_service_shape_returns_option_and_vec() {
    // LSP 查询形状：未实现层返回 None/空（P0 形状——Stage 2 实现侧
    // 按 rust-analyzer 模式填充）
    struct Noop;
    impl LanguageService for Noop {
        fn syntax_tree_at(
            &self,
            _file: u32,
            _pos: Position,
        ) -> Option<kerf_driver::reserved::SyntaxNodeRef> {
            None
        }
        fn completion_items(
            &self,
            _file: u32,
            _pos: Position,
        ) -> Vec<kerf_driver::reserved::CompletionItem> {
            vec![]
        }
        fn document_symbols(&self, _file: u32) -> Vec<Symbol> {
            vec![Symbol(1)]
        }
        fn definition_of(
            &self,
            _file: u32,
            _pos: Position,
        ) -> Option<kerf_driver::reserved::Location> {
            None
        }
        fn references_to(
            &self,
            _file: u32,
            _pos: Position,
        ) -> Vec<kerf_driver::reserved::Location> {
            vec![]
        }
        fn type_at(&self, _file: u32, _pos: Position) -> Option<kerf_driver::reserved::TypeInfo> {
            None
        }
        fn diagnostics(&self, _file: u32) -> Vec<kerf_span::Diagnostic> {
            vec![]
        }
        fn quick_fixes(
            &self,
            _file: u32,
            _pos: Position,
        ) -> Vec<kerf_driver::reserved::CodeAction> {
            vec![]
        }
        fn rename_symbol(
            &mut self,
            _file: u32,
            _pos: Position,
            _new_name: String,
        ) -> Result<kerf_driver::reserved::WorkspaceEdit, kerf_driver::reserved::RenameError>
        {
            Err(kerf_driver::reserved::RenameError {
                reason: "Stage 2 实现".into(),
            })
        }
    }
    let mut ls = Noop;
    assert!(ls
        .definition_of(0, Position { line: 1, column: 1 })
        .is_none());
    assert_eq!(ls.document_symbols(0), vec![Symbol(1)]);
    assert!(ls
        .rename_symbol(0, Position { line: 1, column: 1 }, "x".into())
        .is_err());
}

#[test]
fn incremental_ast_and_debug_shapes() {
    struct Ast {
        rev: u32,
    }
    impl IncrementalAst for Ast {
        fn apply_edit(&mut self, _edit: &TextEdit) -> Result<(), kerf_driver::reserved::EditError> {
            self.rev += 1;
            Ok(())
        }
        fn invalidate_range(&mut self, _range: Span) {}
        fn reuse_unchanged(&self, _other: &Self) -> kerf_driver::reserved::ReusePlan {
            kerf_driver::reserved::ReusePlan::default()
        }
    }
    let mut ast = Ast { rev: 0 };
    ast.apply_edit(&TextEdit {
        file: 0,
        span: Span::dummy(),
        new_text: String::new(),
    })
    .unwrap();
    assert_eq!(ast.rev, 1);

    struct Dbg;
    impl DebugInfoGenerator for Dbg {
        fn source_location_of(&self, _ir_node: NodeId) -> Option<Span> {
            None
        }
        fn ir_node_at(&self, _code_addr: kerf_driver::reserved::Address) -> Option<NodeId> {
            None
        }
        fn variable_location(
            &self,
            _var: VarId,
            _at_pc: kerf_driver::reserved::Address,
        ) -> Option<kerf_driver::reserved::Location> {
            None
        }
        fn generate_dwarf(&self) -> kerf_driver::reserved::DwarfSections {
            kerf_driver::reserved::DwarfSections::default()
        }
        fn generate_source_map(&self) -> kerf_driver::reserved::SourceMapBlob {
            kerf_driver::reserved::SourceMapBlob::default()
        }
    }
    struct Node;
    impl DebugTraceable for Node {
        fn ast_node_id(&self) -> kerf_driver::reserved::AstNodeId {
            kerf_driver::reserved::AstNodeId(3)
        }
        fn debug_name(&self) -> Option<Symbol> {
            Some(Symbol(5))
        }
    }
    let dbg = Dbg;
    assert!(dbg.source_location_of(0).is_none());
    let n = Node;
    assert_eq!(n.ast_node_id(), kerf_driver::reserved::AstNodeId(3));
    assert_eq!(n.debug_name(), Some(Symbol(5)));
}

#[test]
fn query_system_shape_with_dependency_tracking() {
    struct Db {
        invalidated: Vec<u64>,
    }
    impl QuerySystem for Db {
        type Input = u64;
        fn query(&self, _descriptor: &QueryDescriptor) -> QueryResult {
            QueryResult {
                output_fingerprint: 0,
                cached: false,
            }
        }
        fn invalidate(&mut self, input: Self::Input) {
            self.invalidated.push(input);
        }
        fn dependencies_of(&self, _descriptor: &QueryDescriptor) -> Vec<QueryDescriptor> {
            vec![QueryDescriptor {
                name: "parse",
                input_fingerprint: 1,
            }]
        }
    }
    let mut db = Db {
        invalidated: vec![],
    };
    db.invalidate(9);
    assert_eq!(db.invalidated, vec![9]);
    let deps = db.dependencies_of(&QueryDescriptor {
        name: "expand",
        input_fingerprint: 1,
    });
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].name, "parse");
}

#[test]
fn compiler_service_shape_lifecycle() {
    struct Svc {
        cancelled: Vec<kerf_driver::reserved::CompileJobId>,
    }
    impl CompilerService for Svc {
        fn submit(&mut self, _request: CompileRequest) -> kerf_driver::reserved::CompileJobId {
            kerf_driver::reserved::CompileJobId(1)
        }
        fn status(&self, _job: kerf_driver::reserved::CompileJobId) -> Option<CompileStatus> {
            Some(CompileStatus::Running)
        }
        fn result(
            &self,
            _job: kerf_driver::reserved::CompileJobId,
        ) -> Option<kerf_driver::reserved::CompileResult> {
            None
        }
        fn diagnostics_stream(
            &self,
            _job: kerf_driver::reserved::CompileJobId,
        ) -> Vec<kerf_span::Diagnostic> {
            vec![]
        }
        fn cancel(&mut self, job: kerf_driver::reserved::CompileJobId) {
            self.cancelled.push(job);
        }
    }
    let mut svc = Svc { cancelled: vec![] };
    let job = svc.submit(CompileRequest {
        source: "(print 1)".into(),
        target: CompileTarget::Bytecode,
        opt_level: OptLevel::Default,
        debug_info: DebugInfoRequest::Spans,
    });
    assert_eq!(job, kerf_driver::reserved::CompileJobId(1));
    assert_eq!(svc.status(job), Some(CompileStatus::Running));
    assert!(svc.result(job).is_none());
    svc.cancel(job);
    assert_eq!(svc.cancelled, vec![job]);
}

#[test]
fn package_and_ai_shapes() {
    struct Pm;
    impl PackageManager for Pm {
        fn resolve_dependencies(
            &self,
            manifest: &kerf_driver::reserved::PackageManifest,
        ) -> Result<kerf_driver::reserved::DependencyGraph, kerf_driver::reserved::ResolveError>
        {
            Ok(kerf_driver::reserved::DependencyGraph {
                packages: vec![manifest.name.clone()],
            })
        }
        fn get_package(
            &self,
            name: &str,
            version: &str,
        ) -> Result<kerf_driver::reserved::PackageArtifact, kerf_driver::reserved::FetchError>
        {
            Ok(kerf_driver::reserved::PackageArtifact {
                name: name.into(),
                version: version.into(),
                precompiled: true,
            })
        }
        fn lock(
            &self,
            _graph: &kerf_driver::reserved::DependencyGraph,
        ) -> kerf_driver::reserved::LockFile {
            kerf_driver::reserved::LockFile {
                content: String::new(),
            }
        }
    }
    let pm = Pm;
    let graph = pm
        .resolve_dependencies(&kerf_driver::reserved::PackageManifest {
            name: "kerf".into(),
            version: "0.1.0".into(),
            dependencies: vec![],
        })
        .unwrap();
    assert_eq!(graph.packages, vec!["kerf".to_string()]);

    struct Ai;
    impl AiAssistant for Ai {
        fn semantic_summary(&self, _file: u32) -> kerf_driver::reserved::SemanticSummary {
            kerf_driver::reserved::SemanticSummary {
                summary: String::new(),
            }
        }
        fn function_signature(&self, _symbol: &Symbol) -> kerf_driver::reserved::FunctionSignature {
            kerf_driver::reserved::FunctionSignature {
                signature: String::new(),
            }
        }
        fn quick_typecheck(
            &self,
            snippet: &str,
        ) -> Result<TypeCheckResult, kerf_driver::reserved::TypeError> {
            if snippet.is_empty() {
                Err(kerf_driver::reserved::TypeError {
                    reason: "空".into(),
                })
            } else {
                Ok(TypeCheckResult {
                    passed: true,
                    notes: vec![],
                })
            }
        }
        fn refactoring_suggestions(
            &self,
            _selection: &Span,
        ) -> Vec<kerf_driver::reserved::Refactoring> {
            vec![]
        }
        fn generate_doc_comment(&self, _item: &Symbol) -> String {
            String::new()
        }
    }
    let ai = Ai;
    // 负向形状：空片段类型检查失败（错误面可承载）
    assert!(ai.quick_typecheck("").is_err());
    assert!(ai.quick_typecheck("(print 1)").unwrap().passed);
}

#[test]
fn codegen_backend_shape_target_neutral() {
    struct B;
    impl CodegenBackend for B {
        fn supported_targets(&self) -> Vec<TargetTriple> {
            vec![
                TargetTriple("bytecode-vm".into()),
                TargetTriple("wasm32-wasip2".into()),
            ]
        }
        fn compile(
            &self,
            _ir: &AnnotatedANF,
            target: &TargetTriple,
        ) -> Result<kerf_driver::reserved::CompiledModule, kerf_driver::reserved::CodegenError>
        {
            if target.0 == "bytecode-vm" {
                Ok(kerf_driver::reserved::CompiledModule {
                    target: target.clone(),
                    bytes: vec![],
                })
            } else {
                Err(kerf_driver::reserved::CodegenError {
                    reason: "Stage 2+ 后端未实现（P1 预留）".into(),
                })
            }
        }
        fn backend_optimizations(&self) -> Vec<kerf_driver::reserved::PassDescriptor> {
            vec![]
        }
    }
    let b = B;
    assert_eq!(b.supported_targets().len(), 2);
    // 正向：字节码目标（Stage 0 唯一执行后端）
    assert!(b
        .compile(
            &AnnotatedANF::default(),
            &TargetTriple("bytecode-vm".into())
        )
        .is_ok());
    // 负向：未实现目标被拒绝（错误面承载 P1 预留语义）
    assert!(b
        .compile(
            &AnnotatedANF::default(),
            &TargetTriple("wasm32-wasip2".into())
        )
        .is_err());
}

#[test]
fn ffi_boundary_shape_gc_isolation() {
    struct Boundary {
        pinned: u32,
    }
    impl FfiBoundary for Boundary {
        type ExternalPointer = *const u8;
        fn pin_object(&mut self, _obj: Value) {
            self.pinned += 1;
        }
        fn unpin_object(&mut self, _obj: Value) {
            self.pinned = self.pinned.saturating_sub(1);
        }
    }
    let mut b = Boundary { pinned: 0 };
    b.pin_object(Value::Nil);
    b.pin_object(Value::Bool(true));
    assert_eq!(b.pinned, 2);
    b.unpin_object(Value::Nil);
    assert_eq!(b.pinned, 1);
    // pin/unpin 对称（GC 隔离协议双向）
}
