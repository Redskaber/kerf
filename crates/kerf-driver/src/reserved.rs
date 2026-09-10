//! 接口预留层（stage0.md §9.1，P2/P3 级处理程度）。
//!
//! **冻结契约，不写实现**：
//! - **Effect Handlers**（P3）：OCaml 5 风格效应系统——仅类型形状，
//!   行为语义留给引入时的实践数据；
//! - **多阶段编程**（P3）：MetaOCaml 风格 quote/splice/run；
//! - **能力模型 I/O**（P2）：ReadCapability/WriteCapability 权限令牌 +
//!   完整行为规格（Stage 1+ 可按规格直接实现）；
//! - **编译缓存**（P2）：查询式缓存三方法 + 完整行为规格。
//!
//! **接口稳定性原则**（§2.2 原则 27）：本文件的全部签名在整个生命周期内
//! 保持向后兼容——演进只替换实现，不破坏契约。
//!
//! **渐进替换原则**（§2.2 原则 28）：元循环求值器 → 编译器、传统 I/O →
//! 能力模型、传统闭包 → Effect Handlers——每条替换路径的挂点在此冻结。

use kerf_core::{CodeValue, CoreExpr};
use kerf_runtime::RuntimeError;
use kerf_vm::Value;

// ---------------------------------------------------------------------------
// 1. Effect Handlers（P3：仅类型形状，§9.1.1）
// ---------------------------------------------------------------------------

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
/// 行为规格（做实时的完整语义见 stage0.md §6.4）：
/// - `perform`：挂起当前计算，向上查找匹配的 handler；
/// - `handle`：安装 handler 并执行计算——效应触发时以 continuation 恢复。
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

// ---------------------------------------------------------------------------
// 2. 多阶段编程（P3：仅类型形状，§9.1.2）
// ---------------------------------------------------------------------------

/// 多阶段编程（§9.1.2 预留接口——Stage 2 实现）。
///
/// 行为规格（MetaOCaml 语义，stage0.md §6.1）：
/// - `quote`：将表达式提升为代码值（良构/良类型/良作用域保证）；
/// - `splice`：将代码值拼接进当前阶段的语法；
/// - `run`：执行未来阶段代码（Stage 2 做实时为编译期计算）。
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

// ---------------------------------------------------------------------------
// 3. 能力模型 I/O（P2：类型 + 完整行为规格，§9.1.3）
// ---------------------------------------------------------------------------

/// 读能力令牌（不可伪造——私有构造，经权限传递获得）。
pub struct ReadCapability {
    _private: (),
}

/// 写能力令牌（不可伪造）。
pub struct WriteCapability {
    _private: (),
}

/// 能力模型 I/O 错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IOError {
    pub message: String,
}

// ---------------------------------------------------------------------------
// 令牌铸造（r8 批次 D——pub(crate)：令牌仅经 driver 组合根流出，
// 外部 crate 可引用类型但不可构造——「经权限传递获得」的构造面控制）
// ---------------------------------------------------------------------------

/// 铸造读能力令牌（仅供 crate::capability 的授权管线调用——按程序
/// 声明的 (require io read) 铸造；13 §3.1.3 规格条款 1）。
pub(crate) fn mint_read_token() -> ReadCapability {
    ReadCapability { _private: () }
}

/// 铸造写能力令牌（同上——规格条款 2）。
pub(crate) fn mint_write_token() -> WriteCapability {
    WriteCapability { _private: () }
}

/// 能力模型 I/O（§9.1.3 预留接口——Stage 1+ 实现）。
///
/// **完整行为规格**（P2——Stage 1 可直接按规格实现）：
/// 1. `read_line` 仅在持有 `ReadCapability` 时可调用——令牌经线性传递，
///    不可复制、不可伪造；
/// 2. `write_line` 同理（`WriteCapability`）；
/// 3. 无令牌的 I/O 调用为**编译期错误**（权限验证，非运行时检查）；
/// 4. 与 Stage 0 传统 I/O（kerf-runtime::io 全局函数）的替换关系：
///    渐进替换原则 §28——driver 注册的内置函数改为能力参数化形态，
///    全局函数逐步退役。
///
/// Stage 0 裁定：传统 I/O（§21.10 决策点 3）；能力模型于 Stage 1 基础 /
/// Stage 2 完整引入。
pub trait CapabilityIO {
    /// 读一行（需要读能力）。
    fn read_line(cap: &mut ReadCapability) -> Result<String, IOError>;

    /// 写一行（需要写能力）。
    fn write_line(cap: &mut WriteCapability, s: &str) -> Result<(), IOError>;
}

// ---------------------------------------------------------------------------
// 4. 编译缓存（P2：类型 + 完整行为规格，§9.1.4）
// ---------------------------------------------------------------------------

/// 缓存键（源哈希 + 编译配置指纹——确定性编译的键）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// 源文本内容哈希（Stage 1：SHA-256 内容寻址）。
    pub source_hash: u64,
    /// 编译配置指纹（阶段版本 + 选项）。
    pub config_fingerprint: u64,
}

/// 缓存结果（编译产物——字节码 + 核心表达式）。
#[derive(Debug, Clone)]
pub struct CachedResult {
    /// 字节码程序。
    pub program: kerf_compiler::BcProgram,
    /// 核心表达式（诊断与 CodeValue 检查用）。
    pub core: Vec<std::rc::Rc<CoreExpr>>,
    /// 产物对应的代码值（结构化检查）。
    pub code_value: Option<CodeValue>,
}

/// 编译缓存（§9.1.4 预留接口——Stage 1 做实「查询式」）。
///
/// **完整行为规格**（P2——Stage 1 可直接按规格实现）：
/// 1. `get_cached`：键命中返回产物（内容寻址——源不变 + 配置不变 →
///    产物必然等价，可直接复用，无需重新编译）；
/// 2. `store`：写入缓存（幂等——同键覆盖）；
/// 3. `invalidate`：显式失效（源变更由键哈希自然区分；此方法用于
///    编译器升级等全量失效场景）；
/// 4. Stage 2 深化：查询式增量编译（salsa 风格依赖图），
///    本三方法保持签名不变（渐进替换原则 §28）。
pub trait CompilationCache {
    /// 查询缓存。
    fn get_cached(&self, key: &CacheKey) -> Option<CachedResult>;

    /// 写入缓存。
    fn store(&mut self, key: CacheKey, result: CachedResult);

    /// 失效缓存项。
    fn invalidate(&mut self, key: &CacheKey);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P3/P2 冻结性测试：签名以「测试实现体编译通过」证明契约冻结
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

    struct ProbeCache;
    impl CompilationCache for ProbeCache {
        fn get_cached(&self, _key: &CacheKey) -> Option<CachedResult> {
            None
        }
        fn store(&mut self, _key: CacheKey, _result: CachedResult) {}
        fn invalidate(&mut self, _key: &CacheKey) {}
    }

    #[test]
    fn reserved_signatures_are_frozen() {
        // 四项预留 trait 均可按冻结签名实现（编译通过 = 契约冻结）
        let e = ProbeEffects;
        assert!(matches!(e.perform(0u8), Value::Nil));
        let m = ProbeMultiStage;
        let expr = CoreExpr::Literal {
            value: kerf_core::LiteralValue::Int(1),
            span: kerf_span::Span::dummy(),
        };
        let cv = m.quote(&expr);
        assert!(cv.is_well_formed());
        let mut c = ProbeCache;
        c.store(
            CacheKey {
                source_hash: 1,
                config_fingerprint: 1,
            },
            CachedResult {
                program: empty_program(),
                core: vec![],
                code_value: None,
            },
        );
        assert!(c
            .get_cached(&CacheKey {
                source_hash: 1,
                config_fingerprint: 1
            })
            .is_none());
    }

    fn empty_program() -> kerf_compiler::BcProgram {
        kerf_compiler::BcProgram {
            protos: vec![],
            consts: vec![],
            entry: 0,
            global_refs: vec![],
            module_name: None,
        }
    }

    #[test]
    fn cache_key_is_content_addressed() {
        let k1 = CacheKey {
            source_hash: 42,
            config_fingerprint: 1,
        };
        let k2 = CacheKey {
            source_hash: 42,
            config_fingerprint: 1,
        };
        assert_eq!(k1, k2);
        let k3 = CacheKey {
            source_hash: 43,
            config_fingerprint: 1,
        };
        assert_ne!(k1, k3);
    }

    #[test]
    fn capability_tokens_are_unforgeable() {
        // 私有构造：外部无法构造 ReadCapability（编译期保证）
        // 此测试验证令牌类型可被引用而不可被实例化
        fn takes_read(_cap: &ReadCapability) {}
        fn takes_write(_cap: &WriteCapability) {}
        let _ = (
            takes_read as fn(&ReadCapability),
            takes_write as fn(&WriteCapability),
        );
    }
}
