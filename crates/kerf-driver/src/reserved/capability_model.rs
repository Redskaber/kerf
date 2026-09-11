//! 能力模型骨架预留（模型层——r19 / 41-a；develop/v0/stage-2/
//! capability-model-design.md D3/D4 裁定：模型层与族层分离）。
//!
//! **为什么预留**：能力安全模型（权限的颁发/传递/削弱/撤销/验证）
//! 的通用形状。IO 是第一实例（[`capability_io`]——r8 做实基础传递）、
//! FFI 线性令牌（`ExternalType::CPointer`——ffi-ownership-model.md
//! 设计冻结）是第二实例；Stage 2 net/process 族引入前须有模型层
//! 锚位，否则每族新增都是「五点手术」（设计 D11——手术面收敛为
//! 三点加法：枚举变体 + 门控表行 + 族文件）。
//!
//! **层次注记（r19 / 41-a）**：本文件是 13 §3.1.3 的**模型层骨架**
//! ——不计入预留项计数（14 项不变）；族层契约（capability_io.rs）
//! 与授权管线（crate::capability）签名零变化（原则 27）。**模型层
//! 公开面不含任何族专属名**（Io 前缀禁入——防层次再耦合）。
//!
//! **演算位预留**（设计 D12 窗口裁定）：
//! - mint（铸造）：各族内部 pub(crate) 构造面（已做实——不占位）；
//! - delegate（委托）：令牌值传递即委托（隐式承载——不占位）；
//! - attenuate（削弱）：派生只减不增权限的子令牌（Stage 2 中段）；
//! - revoke（撤销）：令牌失效（Stage 3——效应恢复统一语义窗口）。
//!
//! **化学反应锚**（设计 §5——正交可组合不合并）：能力 × 效应
//! （perform 需令牌 + handler = 权限作用域）/ 能力 × 多阶段
//! （代码值携带能力集合——run 验证）/ 能力 × 缓存（CacheKey 加
//! 能力面）/ 能力 × HM（令牌类型禁泛化）。
//!
//! Stage 0 裁定：能力模型于 Stage 1 基础（✅ r8 兑现）/ Stage 2
//! 完整（net/process 族 + 演算做实——12 §2.4.5）。

/// 能力族形状（模型层——族实例的类型归属锚，P3）。
///
/// 族实例（io/ffi/net/process/…）各自冻结契约（一族一文件——
/// 40-g 惯例）；本 trait 只声明「族」的统一形状：令牌类型 + 族
/// 声明名（`(require <family> <member> ...)` 的族段）。
pub trait CapabilityModelFamily {
    /// 族令牌类型（不可伪造——各族内部私有构造 + pub(crate) 铸造面）。
    type Token;

    /// 族声明名（`(require <family> ...)` 的族段——如 io/ffi/net）。
    fn family_name() -> &'static str;
}

/// 令牌演算形状（模型层——Stage 2/3 显式演算位，P3）。
///
/// mint/delegate 已由各族构造面与令牌值传递承载（不占位）；本
/// trait 预留两个显式演算位。**演算方法无默认体**——任何实现必须
/// 显式提供（P3 位不可被实现体静默污染）。
pub trait TokenCalculus {
    /// 令牌类型。
    type Token;

    /// 削弱：派生只减不增权限的子令牌（attenuation——Stage 2 中段；
    /// net 族引入时做实）。
    fn attenuate(source: &Self::Token) -> Self::Token;

    /// 撤销：令牌失效（revocation——Stage 3；效应恢复统一语义窗口）。
    fn revoke(token: &mut Self::Token);
}

#[cfg(test)]
mod tests {
    use super::*;
    // 族实例类型锚（模型-实例归属证明——测试面引用族层类型；
    // 生产面子模块保持零依赖 §13.4 J3）
    use crate::reserved::{ExternalType, ReadCapability, WriteCapability};

    /// P3 冻结性测试：签名以「测试实现体编译通过」证明契约冻结
    /// （原则 27——后续阶段按此签名直接实现演算位）。
    struct ProbeNetFamily;
    struct ProbeNetToken {
        granted: bool,
    }
    impl CapabilityModelFamily for ProbeNetFamily {
        type Token = ProbeNetToken;
        fn family_name() -> &'static str {
            "net"
        }
    }
    impl TokenCalculus for ProbeNetFamily {
        type Token = ProbeNetToken;
        fn attenuate(source: &ProbeNetToken) -> ProbeNetToken {
            // 削弱语义形状：只减不增（Probe 演示——granted 保持，
            // 实际实现可派生权限子集）
            ProbeNetToken {
                granted: source.granted,
            }
        }
        fn revoke(token: &mut ProbeNetToken) {
            token.granted = false;
        }
    }

    #[test]
    fn capability_model_skeleton_is_frozen() {
        // 编译通过 = 契约冻结（multistage.rs Probe 同型）
        assert_eq!(ProbeNetFamily::family_name(), "net");
        let mut t = ProbeNetToken { granted: true };
        let attenuated = <ProbeNetFamily as TokenCalculus>::attenuate(&t);
        assert!(attenuated.granted);
        <ProbeNetFamily as TokenCalculus>::revoke(&mut t);
        assert!(!t.granted);
    }

    /// 模型-实例归属证明（io 族）：IO 族两令牌均满足模型族形状
    /// （设计 D3 层次分离的机器验证——io ⊂ 能力模型：用户判断
    /// 「IO 只是能力模型子集」的代码面证明）。
    struct IoReadShape;
    impl CapabilityModelFamily for IoReadShape {
        type Token = ReadCapability;
        fn family_name() -> &'static str {
            "io"
        }
    }
    struct IoWriteShape;
    impl CapabilityModelFamily for IoWriteShape {
        type Token = WriteCapability;
        fn family_name() -> &'static str {
            "io"
        }
    }

    #[test]
    fn io_family_tokens_satisfy_model_shape() {
        assert_eq!(IoReadShape::family_name(), "io");
        assert_eq!(IoWriteShape::family_name(), "io");
        // 令牌类型作为 Token 关联类型编译通过 = 归属证明（形状断言：
        // 关联类型与族实例令牌类型一致）
        fn token_of<F: CapabilityModelFamily>() -> fn(&F::Token) {
            fn ptr<T>(_: &T) {}
            ptr::<F::Token>
        }
        let read_token_fn: fn(&ReadCapability) = token_of::<IoReadShape>();
        let write_token_fn: fn(&WriteCapability) = token_of::<IoWriteShape>();
        let _ = (read_token_fn, write_token_fn);
    }

    /// FFI 线性令牌归位证明（设计 D8）：CPointer 载体类型满足族
    /// 形状（ExternalType::CPointer——ffi-ownership-model.md 冻结
    /// 设计的模型层归属）。
    struct FfiTokenShape;
    impl CapabilityModelFamily for FfiTokenShape {
        type Token = ExternalType;
        fn family_name() -> &'static str {
            "ffi"
        }
    }

    #[test]
    fn ffi_family_token_satisfies_model_shape() {
        assert_eq!(FfiTokenShape::family_name(), "ffi");
        fn token_of<F: CapabilityModelFamily>() -> fn(&F::Token) {
            fn ptr<T>(_: &T) {}
            ptr::<F::Token>
        }
        let ffi_token_fn: fn(&ExternalType) = token_of::<FfiTokenShape>();
        let _ = ffi_token_fn;
    }

    /// 演算位签名证明：attenuate/revoke 以函数指针形态可取——
    /// 签名冻结（`&Token -> Token` / `&mut Token -> ()`），且无
    /// 默认实现体（实现方必须显式提供——Probe 即证）。
    #[test]
    fn calculus_slots_are_stage_positions() {
        let attenuate_fn: fn(&ProbeNetToken) -> ProbeNetToken =
            <ProbeNetFamily as TokenCalculus>::attenuate;
        let revoke_fn: fn(&mut ProbeNetToken) = <ProbeNetFamily as TokenCalculus>::revoke;
        let mut t = ProbeNetToken { granted: true };
        assert!(attenuate_fn(&t).granted);
        revoke_fn(&mut t);
        assert!(!t.granted);
    }
}
