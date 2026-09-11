//! 多目标后端契约（lang-design 13 §3.3.7——P1，正式家）。
//!
//! **迁移注记（38-b，批次 G 做实落位）**：本模块自
//! `kerf-driver/src/reserved/codegen.rs`（r12 冻结的预留形状）迁移而来
//! ——依据 §2.2 原则 27「签名在整个生命周期保持向后兼容」：全部契约
//! 类型与方法签名零变化，`AnnotatedANF` 从占位形状（仅指纹字段）实化
//! 为携带函数定义集的完整 IR（`fingerprint` 字段保留——内容寻址口径
//! 不变）。`kerf-driver/reserved/codegen` 改为薄 re-export（历史引用
//! 路径继续可用——契约演进替换实现而非破坏契约的实证）。
//!
//! **自举合规注记**（08-backend-evolution / §21.2）：QBE 后端是宿主侧
//! 工具链（Rust 编写的 IL 生成 + 外部 qbe/cc 子进程编排），**永不进入
//! 自举链**；`kerf-backend` 属于 Stage 2 实现 语言 ~20% Rust 部分的
//! 「构建引导 + 后端」基座（12-roadmap §2 同口径）。

use crate::anf::AnnotatedANF;

// ---------------------------------------------------------------------------
// 目标与产物形状（P3 冻结——迁移不变）
// ---------------------------------------------------------------------------

/// 目标三元组（字符串承载——`x86_64-unknown-linux-gnu` 口径）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TargetTriple(pub String);

/// 编译产物（字节信封——字节码/本地码/WASM/QBE IL 统一承载）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompiledModule {
    pub target: TargetTriple,
    pub bytes: Vec<u8>,
}

/// 代码生成错误（形状迁移不变；`span` 扩展为可选——AOT 编排层的
/// 进程错误无源码位置）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenError {
    pub reason: String,
}

impl CodegenError {
    /// 构造辅助（§10.1 规则 1 命名：`new` 惯用构造器）。
    pub fn new(reason: impl Into<String>) -> Self {
        CodegenError {
            reason: reason.into(),
        }
    }
}

/// 优化 pass 描述符（名称 + 目标无关标记）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PassDescriptor {
    pub name: &'static str,
}

// ---------------------------------------------------------------------------
// 后端 trait（P1——契约迁移不变；Stage 2 QBE 后端按契约实现）
// ---------------------------------------------------------------------------

/// 目标描述与代码生成后端。
///
/// **完整行为规格**（P1——迁移自 reserved，语义不变）：
/// 1. `supported_targets`：后端支持的目标清单（能力声明）；
/// 2. `compile`：将 AnnotatedANF 编译为目标码（后端只消费 IR——原则 13
///    目标中立；实化后 AnnotatedANF 携带函数定义集，`compile` 产出
///    后端目标产物，如 QBE IL 文本）；
/// 3. `backend_optimizations`：后端特有优化清单（与优化器 pass 区分——
///    优化器 Stage 3 引入，本清单仅声明后端内 pass）。
pub trait CodegenBackend {
    /// 后端支持的目标。
    fn supported_targets(&self) -> Vec<TargetTriple>;

    /// 将 IR 编译为目标码。
    fn compile(
        &self,
        ir: &AnnotatedANF,
        target: &TargetTriple,
    ) -> Result<CompiledModule, CodegenError>;

    /// 后端特有的优化。
    fn backend_optimizations(&self) -> Vec<PassDescriptor>;
}

/// WebAssembly 后端（2026 年的关键需求：组件模型 + 接口类型 + WASM GC）。
///
/// **预留契约**（P3 占位随迁——不变）：`compile_component` 编译到 WASM
/// 组件模型，Stage 2+ 按组件模型标准具体化。
pub trait WasmBackend {
    /// 编译到 WASM 组件模型。
    fn compile_component(&self, ir: &AnnotatedANF) -> Result<WasmComponent, CodegenError>;
}

/// WASM 组件（P3 形状：字节信封 + 组件模型版本标记）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WasmComponent {
    pub bytes: Vec<u8>,
}

// ---------------------------------------------------------------------------
// 契约锁存测试（自 reserved/codegen.rs 随迁——形状断言不变 + 实化新面）
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anf::AFuncDef;

    /// P1 冻结性测试（随迁）：后端 trait 以「测试实现体编译通过」证明契约冻结。
    struct ProbeBackend;
    impl CodegenBackend for ProbeBackend {
        fn supported_targets(&self) -> Vec<TargetTriple> {
            vec![TargetTriple("bytecode-vm".into())]
        }
        fn compile(
            &self,
            ir: &AnnotatedANF,
            target: &TargetTriple,
        ) -> Result<CompiledModule, CodegenError> {
            if target.0.is_empty() {
                Err(CodegenError::new("空目标"))
            } else {
                Ok(CompiledModule {
                    target: target.clone(),
                    bytes: ir.fingerprint.to_le_bytes().to_vec(),
                })
            }
        }
        fn backend_optimizations(&self) -> Vec<PassDescriptor> {
            vec![PassDescriptor { name: "none" }]
        }
    }

    struct ProbeWasm;
    impl WasmBackend for ProbeWasm {
        fn compile_component(&self, ir: &AnnotatedANF) -> Result<WasmComponent, CodegenError> {
            Ok(WasmComponent {
                bytes: ir.fingerprint.to_le_bytes().to_vec(),
            })
        }
    }

    #[test]
    fn codegen_reserved_signatures_are_frozen() {
        // 迁移不变项：trait 三方法签名 + 目标中立负向 + 指纹透传
        let b = ProbeBackend;
        assert_eq!(b.supported_targets().len(), 1);
        assert!(b
            .compile(&AnnotatedANF::default(), &TargetTriple(String::new()))
            .is_err());
        let ir = AnnotatedANF {
            funcs: Vec::new(),
            fingerprint: 42,
        };
        let m = b.compile(&ir, &TargetTriple("bytecode-vm".into())).unwrap();
        assert_eq!(m.bytes, 42u64.to_le_bytes().to_vec());
        assert_eq!(b.backend_optimizations().len(), 1);
        let w = ProbeWasm;
        let c = w.compile_component(&ir).unwrap();
        assert_eq!(c.bytes, 42u64.to_le_bytes().to_vec());
    }

    #[test]
    fn backend_consumes_ir_not_core_expr() {
        // 原则 13「目标中立」的类型级证明（随迁）：compile 入参是
        // AnnotatedANF（IR 层）而非 CoreExpr——依赖方向在签名层面冻结
        fn assert_input_is_ir(_: &AnnotatedANF) {}
        assert_input_is_ir(&AnnotatedANF::default());
        // 指纹字段保留（实化不破坏内容寻址契约）
        let a = AnnotatedANF {
            funcs: Vec::new(),
            fingerprint: 1,
        };
        let b = AnnotatedANF {
            funcs: Vec::new(),
            fingerprint: 1,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn annotated_anf_realization_keeps_default_and_fingerprint() {
        // 实化新面（38-b）：Default 兼容（既有 reserved_ext_tests 的
        // `AnnotatedANF::default()` 调用路径继续成立）+ funcs 字段携带
        // 函数定义集 + Debug 渲染可用
        let ir = AnnotatedANF::default();
        assert!(ir.funcs.is_empty());
        assert_eq!(ir.fingerprint, 0);
        let ir2 = AnnotatedANF {
            funcs: vec![AFuncDef {
                name: "main".into(),
                params: Vec::new(),
                body: Default::default(),
            }],
            fingerprint: 7,
        };
        assert_eq!(ir2.funcs.len(), 1);
        assert_eq!(ir2.funcs[0].name, "main");
        assert_ne!(ir, ir2);
        let s = format!("{:?}", ir2);
        assert!(s.contains("main"));
    }
}
