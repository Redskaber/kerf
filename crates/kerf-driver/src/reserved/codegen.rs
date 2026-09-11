//! 多目标后端接口预留（stage0.md §9.3.7 / lang-design 13 §3.3.7——P1）。
//!
//! **做实迁移注记（38-b，批次 G）**：本模块的契约类型与 trait 已迁入
//! 正式家 `kerf-backend/src/codegen.rs`（AnnotatedANF 实化 + QbeBackend
//! 首个做实实现）。此处保留薄 re-export——依据 §2.2 原则 27「签名
//! 向后兼容」：r12 冻结期的历史引用路径（`kerf_driver::reserved::*`）
//! 继续可用（`reserved_ext_tests` 契约锁存测试不变即迁移兼容的实证）。
//!
//! **自举合规注记**（08-backend-evolution / §21）：LLVM 永不进入自举
//! 链；QBE 是宿主侧工具链（kerf-backend 为 Rust ~20% 基建部分）。

pub use kerf_backend::codegen::{
    CodegenBackend, CodegenError, CompiledModule, PassDescriptor, TargetTriple, WasmBackend,
    WasmComponent,
};

// AnnotatedANF 实化后携带函数定义集（kerf-backend::anf::AFuncDef）——
// 本模块不 re-export 实化类型（预留层语义止于契约形状；完整实化面经
// kerf-backend 公共路径消费），fingerprint 字段语义不变。
pub use kerf_backend::anf::AnnotatedANF;

#[cfg(test)]
mod tests {
    use super::*;

    /// 迁移兼容锁存：预留层路径 → 正式家类型（原则 27——演进替换实现
    /// 不破坏契约：旧路径可见性 + 签名一致性）。
    #[test]
    fn reserved_path_reexports_formal_contract() {
        struct ProbeWasm;
        impl WasmBackend for ProbeWasm {
            fn compile_component(&self, _ir: &AnnotatedANF) -> Result<WasmComponent, CodegenError> {
                Ok(WasmComponent::default())
            }
        }
        let t = TargetTriple("bytecode-vm".into());
        let e = CodegenError::new("探针");
        assert_eq!(e.reason, "探针");
        let m = CompiledModule {
            target: t.clone(),
            bytes: vec![],
        };
        assert_eq!(m.target, t);
        let ir = AnnotatedANF::default();
        assert_eq!(ir.fingerprint, 0);
        // trait 可达性（类型级）
        fn assert_backend<B: CodegenBackend>(_: &B) {}
        assert_backend(&kerf_backend::QbeBackend);
        // WasmBackend 契约形状（占位随迁不变——泛型函数类型级锚）
        fn assert_wasm<W: WasmBackend>(_: &W) {}
        let _ = assert_wasm::<ProbeWasm>;
        let _ = PassDescriptor { name: "probe" };
        let _ = WasmComponent::default();
    }
}
