//! 多目标后端接口预留（stage0.md §9.3.7 / lang-design 13 §3.3.7——P1，
//! next4 第八轮吸收，r12 批次冻结）。
//!
//! **为什么预留**：后端可插拔（IR 层级缺失需求之一）——目标中立性原则
//! （§2.2 原则 13）的接口化落地；WebAssembly 是 2026 年的关键目标。
//!
//! **与后端策略的关系**（08-backend-evolution / §21）：Stage 0 不引入
//! LLVM、字节码 VM 为唯一执行后端——本 trait 预留的是**未来后端的插槽**
//! （Stage 2 QBE → Stage 2+ WASM/LLVM 可选），**LLVM 永不进入自举链**
//! 的裁定不变。

// ---------------------------------------------------------------------------
// 目标与产物形状（P3 冻结）
// ---------------------------------------------------------------------------

/// 目标三元组（P3 形状：字符串承载——`x86_64-unknown-linux-gnu` 口径）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TargetTriple(pub String);

/// 注解 ANF IR（P3 形状：Stage 2 后端输入 IR 的占位——六 IR 层级中
/// AnnotatedANF 层的形状插槽；Stage 2 具体化，本类型仅冻结依赖方向）。
///
/// **预留契约**：后端只消费 IR 不消费 CoreExpr——`compile` 以本类型为入参
/// 即「前端与目标无关」的类型级声明。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnnotatedANF {
    /// IR 指纹（内容寻址——与 CacheKey/QueryDescriptor 口径一致）。
    pub fingerprint: u64,
}

/// 编译产物（P3 形状：字节信封——字节码/本地码/WASM 统一承载）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompiledModule {
    pub target: TargetTriple,
    pub bytes: Vec<u8>,
}

/// 代码生成错误（P3 形状）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenError {
    pub reason: String,
}

/// 优化 pass 描述符（P3 形状：名称 + 目标无关标记）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PassDescriptor {
    pub name: &'static str,
}

// ---------------------------------------------------------------------------
// 后端 trait（P1——Stage 0 预留 trait 定义，Stage 2 QBE 后端按契约实现）
// ---------------------------------------------------------------------------

/// 目标描述与代码生成后端。
///
/// **完整行为规格**（P1——Stage 2 可直接按规格实现）：
/// 1. `supported_targets`：后端支持的目标清单（能力声明）；
/// 2. `compile`：将 AnnotatedANF 编译为目标码（后端只消费 IR——原则 13
///    目标中立）；
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
/// **预留契约**：`compile_component` 编译到 WASM 组件模型（P3 形状——
/// Stage 2+ 按组件模型标准具体化）。
pub trait WasmBackend {
    /// 编译到 WASM 组件模型。
    fn compile_component(&self, ir: &AnnotatedANF) -> Result<WasmComponent, CodegenError>;
}

/// WASM 组件（P3 形状：字节信封 + 组件模型版本标记）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WasmComponent {
    pub bytes: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P1 冻结性测试：后端 trait 以「测试实现体编译通过」证明契约冻结。
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
                Err(CodegenError {
                    reason: "空目标".into(),
                })
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
        let b = ProbeBackend;
        assert_eq!(b.supported_targets().len(), 1);
        // 负向形状：空目标被拒绝（错误面可承载）
        assert!(b
            .compile(&AnnotatedANF::default(), &TargetTriple(String::new()))
            .is_err());
        // 正向形状：指纹透传（内容寻址口径一致性）
        let ir = AnnotatedANF { fingerprint: 42 };
        let m = b.compile(&ir, &TargetTriple("bytecode-vm".into())).unwrap();
        assert_eq!(m.bytes, 42u64.to_le_bytes().to_vec());
        assert_eq!(b.backend_optimizations().len(), 1);
        let w = ProbeWasm;
        let c = w.compile_component(&ir).unwrap();
        assert_eq!(c.bytes, 42u64.to_le_bytes().to_vec());
    }

    #[test]
    fn backend_consumes_ir_not_core_expr() {
        // 原则 13「目标中立」的类型级证明：compile 入参是 AnnotatedANF
        // （IR 层）而非 CoreExpr——依赖方向在签名层面冻结
        fn assert_input_is_ir(_: &AnnotatedANF) {}
        assert_input_is_ir(&AnnotatedANF::default());
        // 指纹字段存在且可内容寻址
        let a = AnnotatedANF { fingerprint: 1 };
        let b = AnnotatedANF { fingerprint: 1 };
        assert_eq!(a, b);
    }
}
