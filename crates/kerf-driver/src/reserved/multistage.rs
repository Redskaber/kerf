//! 多阶段编程接口预留（stage0.md §9.1.2 / lang-design 13 §3.1.2——P3，
//! 仅类型形状）。
//!
//! **为什么预留**：编译期计算与代码生成（comptime）的表达力地基
//! ——MetaOCaml 理论完备（良构/良类型/良作用域），实现启发式。
//!
//! **拆分注记（r18 / 40-g）**：自 mod.rs 拆出（§13.4 J1——13 §3.1.2
//! 分节对齐）；签名与行为规格零变化（原则 27 冻结契约）。
//!
//! **行为规格**（MetaOCaml 语义，stage0.md §6.1）：
//! - `quote`：将表达式提升为代码值（良构/良类型/良作用域保证）；
//! - `splice`：将代码值拼接进当前阶段的语法；
//! - `run`：执行未来阶段代码（Stage 2 做实时为编译期计算）。
//!
//! Stage 0 裁定：元循环求值器（§21.10 决策点 1）；多阶段于 Stage 2+ 替换。

use kerf_core::CoreExpr;
use kerf_runtime::RuntimeError;
use kerf_vm::Value;

/// 多阶段编程（§9.1.2 预留接口——Stage 2 实现）。
///
/// Stage 0 裁定：元循环求值器（§21.10 决策点 1）；多阶段于 Stage 2+ 替换。
pub trait MultiStage {
    /// 代码值类型（Stage 2 具体化为 `CodeValue`）。
    type Code;

    /// 引号：表达式 → 代码值（Stage 2 实现）。
    fn quote(&self, expr: &CoreExpr) -> Self::Code;

    /// 拼接：代码值 → 当前阶段代码（Stage 2 实现）。
    fn splice(&self, code: &Self::Code) -> Result<CoreExpr, RuntimeError>;

    /// 执行代码值（Stage 2 实现）。
    fn run(&self, code: &Self::Code) -> Result<Value, RuntimeError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use kerf_core::CodeValue;

    /// P3 冻结性测试：签名以「测试实现体编译通过」证明契约冻结
    /// （§27 接口稳定性——后续阶段按此签名直接实现）。
    struct ProbeMultiStage;
    impl MultiStage for ProbeMultiStage {
        type Code = CodeValue;
        fn quote(&self, expr: &CoreExpr) -> CodeValue {
            CodeValue::from_expr(expr)
        }
        fn splice(&self, _code: &CodeValue) -> Result<CoreExpr, RuntimeError> {
            Err(RuntimeError::new("P3 预留：splice Stage 2 实现"))
        }
        fn run(&self, _code: &CodeValue) -> Result<Value, RuntimeError> {
            Err(RuntimeError::new("P3 预留：run Stage 2 实现"))
        }
    }

    #[test]
    fn multistage_signature_is_frozen() {
        let m = ProbeMultiStage;
        let expr = CoreExpr::Literal {
            value: kerf_core::LiteralValue::Int(1),
            span: kerf_span::Span::dummy(),
        };
        let cv = m.quote(&expr);
        assert!(cv.is_well_formed());
    }
}
