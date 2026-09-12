//! 内存编译缓存（Stage 1 批次 C——[13-能力矩阵 §3.1.4](../../docs/lang-design/13-capability-matrix.md)
//! 三方法规格的做实落地；§21.3 Stage 1 条件 4「增量编译基础设施可用」的基座）。
//!
//! **两层入口**（§11 接口隔离——冻结契约与管线服务分离）：
//! - 冻结 trait [`CompilationCache`]（reserved.rs v5.2 冻结签名）：产物级
//!   查询/写入/失效——`get_cached`/`store`/`invalidate` 行为规格逐条落地；
//! - 管线富入口（inherent）：`lookup_front`/`store_front` 携带完整前端
//!   输出（符号表/源映射/模块簿记）——`run`/`eval`/`check` 路径的复用面。
//!
//! **键语义**（内容寻址，规格条款 1）：`CacheKey` 冻结字段为
//! `source_hash`（SHA-256 截断 u64——hash.rs）+ `config_fingerprint`
//! （编译配置指纹 = 阶段版本种子 [`COMPILE_CONFIG_SEED`] + 源文件名。
//! 文件名入指纹的理由：产物内嵌 SourceMap（Span → 文件名/行号渲染），
//! 同文本不同文件名的产物**不等价**——规格「源不变 + 配置不变 → 产物
//! 必然等价」在含位置信息产物下要求位置一致）。
//!
//! **复用安全性**（三个前提全部成立，任一破坏即失守）：
//! 1. 编译确定性：同源同表 → 逐字节相同产物（global/const 均为首次
//!    遇见序分配，无哈希迭代序输入）；
//! 2. 符号表幂等：`intern` 同名 → 同 Symbol——命中返回的表快照与后续
//!    `register_globals` 的内置注册互不冲突；
//! 3. 快照语义：命中返回 FrontOutput **克隆**——运行期状态（globals/
//!    heap/相位标记）永不在缓存条目上变异。
//!
//! 错误路径不缓存：`compile_front` Err 不落缓存（重编译错误路径——
//! 诊断可能随版本演化；错误路径无性能压力）。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kerf_compiler::BcProgram;
use kerf_core::CoreExpr;

use crate::driver::{CompilerKind, FrontOutput};
use crate::hash::content_hash64;
use crate::reserved::{CacheKey, CachedResult, CompilationCache};

/// 编译配置指纹的阶段版本种子（编译器升级 → 改此常量 → 全量自然失效，
/// 13 §3.1.4 规格条款 3 的键路径实现；Stage 2 查询式深化时保持不变）。
pub const COMPILE_CONFIG_SEED: &str = "kerf-stage1-batchC-r7";

/// 缓存条目（两级：管线富条目 / trait 产物级条目）。
enum CacheEntry {
    /// 管线富条目（store_front 写入——run/eval/check 可复用）。
    Front(FrontOutput),
    /// 产物级条目（trait `store` 写入——只服务 `get_cached`；无符号表/
    /// 源映射，管线路径按未中处理）。
    Artifacts(BcProgram, Vec<Rc<CoreExpr>>),
}

/// 缓存观测统计（数据驱动原则 §2.1——命中率的可度量面）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheStats {
    /// 命中次数（含 trait 查询与管线查询）。
    pub hits: usize,
    /// 未中次数。
    pub misses: usize,
    /// 写入次数（store_front + trait store）。
    pub stores: usize,
    /// 失效次数（invalidate 单键 + 全量失效累计条目数）。
    pub invalidations: usize,
}

/// 内存编译缓存（单线程会话缓存——thread_local 实例见下）。
pub struct InMemoryCompilationCache {
    entries: HashMap<CacheKey, CacheEntry>,
    enabled: bool,
    stats: CacheStats,
}

impl InMemoryCompilationCache {
    /// 空缓存（默认启用）。
    pub fn new() -> Self {
        InMemoryCompilationCache {
            entries: HashMap::new(),
            enabled: true,
            stats: CacheStats::default(),
        }
    }

    /// 是否启用。
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// 启用/停用（停用时管线路径直编译——基准对照/调试用）。
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 统计快照。
    pub fn stats(&self) -> CacheStats {
        self.stats
    }

    /// 条目数。
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// 管线富查询：命中返回 FrontOutput 克隆（快照语义）。
    /// pub(crate)：FrontOutput 为 crate 内形态；外部消费方经
    /// [`crate::driver`] 入口（run/eval/check）间接消费缓存。
    pub(crate) fn lookup_front(&mut self, key: &CacheKey) -> Option<FrontOutput> {
        let hit = matches!(self.entries.get(key), Some(CacheEntry::Front(_)));
        if hit {
            self.stats.hits += 1;
            // unwrap 安全性：上方 matches! 确认 Front 变体
            match self.entries.get(key) {
                Some(CacheEntry::Front(front)) => Some(front.clone()),
                _ => unreachable!("已确认为 Front 条目"),
            }
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// 管线富写入（同键覆盖——幂等，与 trait store 同口径）。pub(crate)
    /// ——同 [`lookup_front`] 口径。
    pub(crate) fn store_front(&mut self, key: CacheKey, front: FrontOutput) {
        self.entries.insert(key, CacheEntry::Front(front));
        self.stats.stores += 1;
    }

    /// 全量失效（编译器升级场景——条目清空；统计保留并累计失效数）。
    pub fn invalidate_all(&mut self) {
        self.stats.invalidations += self.entries.len();
        self.entries.clear();
    }
}

impl Default for InMemoryCompilationCache {
    fn default() -> Self {
        Self::new()
    }
}

// 冻结 trait 实现（reserved.rs §3.1.4 签名——行为规格 1/2/3 逐条落地）。
impl CompilationCache for InMemoryCompilationCache {
    /// 规格条款 1：键命中返回产物（内容寻址——可直接复用）。
    fn get_cached(&self, key: &CacheKey) -> Option<CachedResult> {
        match self.entries.get(key) {
            Some(CacheEntry::Front(front)) => Some(CachedResult {
                program: front.program.clone(),
                core: front.core.clone(),
                // code_value：按需计算面（查询式语义——缓存条目不强制
                // 携带；消费方经 CodeValue::from_expr 现算）
                code_value: None,
            }),
            Some(CacheEntry::Artifacts(program, core)) => Some(CachedResult {
                program: program.clone(),
                core: core.clone(),
                code_value: None,
            }),
            None => None,
        }
    }

    /// 规格条款 2：写入缓存（幂等——同键覆盖）。
    fn store(&mut self, key: CacheKey, result: CachedResult) {
        self.entries
            .insert(key, CacheEntry::Artifacts(result.program, result.core));
        self.stats.stores += 1;
    }

    /// 规格条款 3：显式失效（源变更由键哈希自然区分；此处用于单键失效）。
    fn invalidate(&mut self, key: &CacheKey) {
        if self.entries.remove(key).is_some() {
            self.stats.invalidations += 1;
        }
    }
}

thread_local! {
    /// 会话级缓存实例（每线程独立——测试线程天然隔离，无跨测试污染）。
    static CACHE: RefCell<InMemoryCompilationCache> =
        RefCell::new(InMemoryCompilationCache::new());
}

/// 访问会话缓存（thread_local 借用封装）。
pub fn with_cache<R>(f: impl FnOnce(&mut InMemoryCompilationCache) -> R) -> R {
    CACHE.with(|c| f(&mut c.borrow_mut()))
}

/// 构造缓存键（源文本 + 文件名 + 编译器种类 → 内容寻址键）。
///
/// **CompilerKind 维度（B11/P4——i1-design §6 切换点）**：种子与自举
/// 产物不得混享缓存条目——维度入 `config_fingerprint` 构成层（
/// `CacheKey` 冻结字段结构不动——v5.2 接口预留纪律；生产入口恒
/// Bootstrap，种子参考路径不经缓存，键区分是防御性分桶实测面）。
pub fn cache_key(source: &str, filename: &str, kind: CompilerKind) -> CacheKey {
    let kind_tag = match kind {
        CompilerKind::Bootstrap => "bootstrap",
        CompilerKind::Seed => "seed",
    };
    CacheKey {
        source_hash: content_hash64(source.as_bytes()),
        config_fingerprint: content_hash64(
            format!("{}|{}|{}", COMPILE_CONFIG_SEED, filename, kind_tag).as_bytes(),
        ),
    }
}

/// 会话缓存统计快照。
pub fn cache_stats() -> CacheStats {
    with_cache(|c| c.stats())
}

/// 会话缓存条目数。
pub fn cache_entry_count() -> usize {
    with_cache(|c| c.entry_count())
}

/// 启用/停用会话缓存（停用后管线直编译——基准对照用）。
pub fn set_cache_enabled(enabled: bool) {
    with_cache(|c| c.set_enabled(enabled));
}

/// 会话缓存是否启用。
pub fn cache_enabled() -> bool {
    with_cache(|c| c.enabled())
}

/// 重置会话缓存（条目 + 统计清零——测试隔离用）。
pub fn cache_reset() {
    with_cache(|c| {
        *c = InMemoryCompilationCache::new();
    });
}

/// 全量失效（编译器升级场景——规格条款 3 的全量形态；统计保留）。
pub fn cache_invalidate_all() {
    with_cache(|c| c.invalidate_all());
}

/// 测试用空程序（BcProgram 无 Default——字段全空 + 入口 0）。
#[cfg(test)]
fn empty_program() -> BcProgram {
    BcProgram {
        protos: Vec::new(),
        consts: Vec::new(),
        entry: 0,
        global_refs: Vec::new(),
        module_name: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kerf_expander::phase::ModuleRegistry;
    use kerf_span::SourceMap;
    use kerf_syntax::SymbolTable;

    /// 规格条款 1/2：store → get_cached 命中；产物逐字段等价。
    #[test]
    fn trait_store_then_get_hits() {
        let mut c = InMemoryCompilationCache::new();
        let key = CacheKey {
            source_hash: 1,
            config_fingerprint: 2,
        };
        assert!(c.get_cached(&key).is_none());
        c.store(
            key.clone(),
            CachedResult {
                program: empty_program(),
                core: Vec::new(),
                code_value: None,
            },
        );
        let got = c.get_cached(&key).expect("store 后应命中");
        assert_eq!(got.core.len(), 0);
        assert!(got.code_value.is_none());
    }

    /// 规格条款 2：幂等——同键覆盖不报错、条目数不增。
    #[test]
    fn trait_store_idempotent_overwrite() {
        let mut c = InMemoryCompilationCache::new();
        let key = CacheKey {
            source_hash: 7,
            config_fingerprint: 8,
        };
        let result = CachedResult {
            program: empty_program(),
            core: Vec::new(),
            code_value: None,
        };
        c.store(key.clone(), result.clone());
        c.store(key.clone(), result);
        assert_eq!(c.entry_count(), 1);
    }

    /// 规格条款 3：invalidate 单键失效。
    #[test]
    fn trait_invalidate_misses() {
        let mut c = InMemoryCompilationCache::new();
        let key = CacheKey {
            source_hash: 9,
            config_fingerprint: 10,
        };
        c.store(
            key.clone(),
            CachedResult {
                program: empty_program(),
                core: Vec::new(),
                code_value: None,
            },
        );
        c.invalidate(&key);
        assert!(c.get_cached(&key).is_none());
        assert_eq!(c.stats().invalidations, 1);
        // 未命中键的失效不计数（幂等失效）
        c.invalidate(&key);
        assert_eq!(c.stats().invalidations, 1);
    }

    /// 键性质：源文本区分 + 文件名区分 + 配置种子区分。
    #[test]
    fn cache_key_discriminates() {
        let a = cache_key("(+ 1 2)", "a.krf", CompilerKind::Bootstrap);
        let a2 = cache_key("(+ 1 2)", "a.krf", CompilerKind::Bootstrap);
        let b = cache_key("(+ 1 3)", "a.krf", CompilerKind::Bootstrap);
        let f = cache_key("(+ 1 2)", "b.krf", CompilerKind::Bootstrap);
        assert_eq!(a, a2);
        assert_ne!(a, b);
        assert_ne!(a, f);
    }

    /// B11/P4：CompilerKind 分桶——同源同名异 kind 键不同（种子与
    /// 自举产物不混享条目——键层防御实测；i1-design §6 P4 + 风险表
    /// 「缓存混享」缓解项的显式测试）。
    #[test]
    fn cache_key_buckets_by_compiler_kind() {
        let prod = cache_key("(+ 1 2)", "a.krf", CompilerKind::Bootstrap);
        let seed = cache_key("(+ 1 2)", "a.krf", CompilerKind::Seed);
        assert_ne!(
            prod, seed,
            "同源同名异 CompilerKind 键必须不同（B11 分桶——混享即种子产物污染生产缓存）"
        );
    }

    /// B11/P4 集成面：种子键条目不被生产查询命中（lookup_front 跨
    /// kind 未中——即使同源同名）。
    #[test]
    fn seed_bucket_entry_not_served_to_production_lookup() {
        let mut c = InMemoryCompilationCache::new();
        let seed_key = cache_key("(+ 1 2)", "k.krf", CompilerKind::Seed);
        let prod_key = cache_key("(+ 1 2)", "k.krf", CompilerKind::Bootstrap);
        let front = FrontOutput {
            warnings: Vec::new(),
            core: Vec::new(),
            program: empty_program(),
            source_map: SourceMap::new(),
            table: SymbolTable::new(),
            registry: ModuleRegistry::new(),
        };
        c.store_front(seed_key, front);
        assert!(
            c.lookup_front(&prod_key).is_none(),
            "生产查询不得返回种子桶条目（B11：种子命中后自举路径不得返回种子产物）"
        );
        // 同键命中（快照语义自检——种子桶自身可命中）
        assert!(c
            .lookup_front(&cache_key("(+ 1 2)", "k.krf", CompilerKind::Seed))
            .is_some());
    }

    /// trait 产物级条目不服务管线富查询（Front 缺位按未中处理）。
    #[test]
    fn trait_store_degrades_for_pipeline() {
        let mut c = InMemoryCompilationCache::new();
        let key = CacheKey {
            source_hash: 11,
            config_fingerprint: 12,
        };
        c.store(
            key.clone(),
            CachedResult {
                program: empty_program(),
                core: Vec::new(),
                code_value: None,
            },
        );
        assert!(c.lookup_front(&key).is_none());
        assert!(c.get_cached(&key).is_some());
    }

    /// 全量失效：条目清零 + 统计累计。
    #[test]
    fn invalidate_all_clears_entries() {
        let mut c = InMemoryCompilationCache::new();
        let key = CacheKey {
            source_hash: 13,
            config_fingerprint: 14,
        };
        c.store(
            key,
            CachedResult {
                program: empty_program(),
                core: Vec::new(),
                code_value: None,
            },
        );
        c.invalidate_all();
        assert_eq!(c.entry_count(), 0);
        assert_eq!(c.stats().invalidations, 1);
        assert_eq!(c.stats().stores, 1);
    }
}
