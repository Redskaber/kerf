//! 编译缓存接口预留（stage0.md §9.1.4 / lang-design 13 §3.1.4——P2：
//! 类型 + 完整行为规格；r7 批次 C 起消费面 `crate::cache`（内存内容
//! 寻址 + 三方法做实））。
//!
//! **为什么预留**：增量编译/查询式缓存的地基（rustc/salsa 验证）。
//!
//! **拆分注记（r18 / 40-g）**：自 mod.rs 拆出（§13.4 J1——13 §3.1.4
//! 分节对齐）；签名与行为规格零变化（原则 27 冻结契约）。实现位于
//! `crate::cache`（r7 做实——契约与实现的模块分离形态）。
//!
//! **完整行为规格**（P2——Stage 1 可直接按规格实现）：
//! 1. `get_cached`：键命中返回产物（内容寻址——源不变 + 配置不变 →
//!    产物必然等价，可直接复用，无需重新编译）；
//! 2. `store`：写入缓存（幂等——同键覆盖）；
//! 3. `invalidate`：显式失效（源变更由键哈希自然区分；此方法用于
//!    编译器升级等全量失效场景）；
//! 4. Stage 2 深化：查询式增量编译（salsa 风格依赖图），
//!    本三方法保持签名不变（渐进替换原则 §28）。

use std::rc::Rc;

use kerf_compiler::BcProgram;
use kerf_core::{CodeValue, CoreExpr};

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
    pub program: BcProgram,
    /// 核心表达式（诊断与 CodeValue 检查用）。
    pub core: Vec<Rc<CoreExpr>>,
    /// 产物对应的代码值（结构化检查）。
    pub code_value: Option<CodeValue>,
}

/// 编译缓存（§9.1.4 预留接口——Stage 1 做实「查询式」）。
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

    /// P2 冻结性测试：签名以「测试实现体编译通过」证明契约冻结。
    struct ProbeCache;
    impl CompilationCache for ProbeCache {
        fn get_cached(&self, _key: &CacheKey) -> Option<CachedResult> {
            None
        }
        fn store(&mut self, _key: CacheKey, _result: CachedResult) {}
        fn invalidate(&mut self, _key: &CacheKey) {}
    }

    fn empty_program() -> BcProgram {
        BcProgram {
            protos: vec![],
            consts: vec![],
            entry: 0,
            global_refs: vec![],
            module_name: None,
        }
    }

    #[test]
    fn compilation_cache_signature_is_frozen() {
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
}
