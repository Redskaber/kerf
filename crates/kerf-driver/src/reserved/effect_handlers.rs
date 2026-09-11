//! Effect Handlers 接口预留（stage0.md §9.1.1 / lang-design 13 §3.1.1
//! ——P3，仅类型形状；r8 批次 D 起消费面 `crate::effects` 编译器内部做实）。
//!
//! **为什么预留**：异步/错误恢复/自定义控制流的统一机制——OCaml 5
//! 实践（Forester 6.0）证明「早期实践」档成熟度。
//!
//! **拆分注记（r18 / 40-g）**：自 mod.rs 拆出（§13.4 J1——13 §3.1.1
//! 分节对齐）；签名与行为规格零变化（原则 27 冻结契约）。
//!
//! **行为规格**（做实时的完整语义见 stage0.md §6.4 + r18 效应语言级
//! 设计 `stage-2/effect-language-design.md`——设计冻结，实现窗口
//! D12 = 批次 I 后段）：
//! - `perform`：挂起当前计算，向上查找匹配的 handler；
//! - `handle`：安装 handler 并执行计算——效应触发时以 continuation 恢复。
//!
//! Stage 0 裁定：传统闭包（§21.10 决策点 2）；Effects 于 Stage 2
//! 语言级引入。

use kerf_vm::Value;

/// 效应族（P3 预留）：效应操作与结果类型关联。
pub trait EffectFamily {
    /// 执行该效应后的结果值（P3 形状：以 Value 具体化——Stage 2 做实时
    /// 可细化为泛型关联）。
    type Result;
}

/// 效应（P3 预留）：效应操作描述。
pub trait Effect {
    /// 所属效应族。
    type Family: EffectFamily<Result = Value>;
}

/// Effect Handlers（§9.1.1 预留接口——Stage 2 实现）。
///
/// Stage 0 裁定：传统闭包（§21.10 决策点 2）；Effects 于 Stage 2 语言级引入。
pub trait EffectSystem {
    type Effect: Effect;
    type Handler;
    type Continuation;

    /// 执行效应：挂起并向上传递（Stage 2 实现；P3 形状：结果经 Value）。
    fn perform(&self, effect: Self::Effect) -> Value;

    /// 安装效应处理器并执行计算（Stage 2 实现）。
    fn handle(&self, handler: Self::Handler, computation: impl FnOnce() -> Value) -> Value;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P3 冻结性测试：签名以「测试实现体编译通过」证明契约冻结
    /// （§27 接口稳定性——后续阶段按此签名直接实现）。
    struct ProbeEffects;
    impl EffectFamily for u8 {
        type Result = Value;
    }
    impl Effect for u8 {
        type Family = u8;
    }
    impl EffectSystem for ProbeEffects {
        type Effect = u8;
        type Handler = ();
        type Continuation = ();
        fn perform(&self, _effect: Self::Effect) -> Value {
            Value::Nil
        }
        fn handle(&self, _handler: Self::Handler, computation: impl FnOnce() -> Value) -> Value {
            computation()
        }
    }

    #[test]
    fn effect_handlers_signature_is_frozen() {
        let e = ProbeEffects;
        assert!(matches!(e.perform(0u8), Value::Nil));
    }
}
