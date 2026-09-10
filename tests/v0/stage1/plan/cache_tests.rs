//! 编译缓存集成测试（tests/v0/stage1/plan/cache_tests.rs ↔
//! kerf-driver/src/cache.rs + hash.rs ↔ 13-能力矩阵 §3.1.4 三方法规格）。
//!
//! 批次 C（25-b）：内容寻址键 / trait 行为规格 1-3 / 管线复用等价性
//! （§21.3 Stage 1 条件 4「增量编译基础设施可用」的验收面）。
//!
//! 线程隔离：缓存为 thread_local——每个 #[test] 独立线程天然隔离
//! （跨测试零污染；同测试内多调用共享会话缓存）。

use kerf_driver::{
    cache_enabled, cache_entry_count, cache_invalidate_all, cache_key, cache_reset, cache_stats,
    check_source, compile_source, eval_source, run_source, run_source_rendered, set_cache_enabled,
    InMemoryCompilationCache,
};
use kerf_driver::{sha256_hex, CacheKey, CachedResult, CompilationCache};

// ---------------------------------------------------------------------------
// SHA-256（内容寻址的正确性根基——公共 API 复验）
// ---------------------------------------------------------------------------

/// NIST 标准向量（跨 crate 公共 API 面——与 hash.rs 单测互为镜像锚）。
#[test]
fn sha256_public_api_vectors() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

// ---------------------------------------------------------------------------
// 冻结 trait 行为规格（13 §3.1.4 条款 1/2/3——独立实例直测）
// ---------------------------------------------------------------------------

/// 条款 1+2：store → get 命中；幂等覆盖；同键产物等价。
#[test]
fn trait_spec_store_get_idempotent() {
    let mut c = InMemoryCompilationCache::new();
    let key = cache_key("(+ 1 2)", "spec.krf");
    assert!(c.get_cached(&key).is_none(), "空缓存不应命中");
    // 经真实管线产物写入（trait 面）
    let out = compile_source("(+ 1 2)", "spec.krf").expect("编译失败");
    c.store(
        key.clone(),
        CachedResult {
            program: out.program.clone(),
            core: out.core.clone(),
            code_value: None,
        },
    );
    let got = c.get_cached(&key).expect("store 后应命中");
    assert_eq!(got.program, out.program, "产物级命中必须逐字段等价");
    assert_eq!(got.core.len(), out.core.len());
    // 条款 2：幂等——同键覆盖不增条目
    c.store(
        key.clone(),
        CachedResult {
            program: out.program,
            core: out.core,
            code_value: None,
        },
    );
    assert_eq!(c.entry_count(), 1);
}

/// 条款 3：invalidate 显式失效（单键）。
#[test]
fn trait_spec_invalidate() {
    let mut c = InMemoryCompilationCache::new();
    let key = cache_key("(if true 1 2)", "inv.krf");
    let out = compile_source("(if true 1 2)", "inv.krf").expect("编译失败");
    c.store(
        key.clone(),
        CachedResult {
            program: out.program,
            core: out.core,
            code_value: None,
        },
    );
    assert!(c.get_cached(&key).is_some());
    c.invalidate(&key);
    assert!(c.get_cached(&key).is_none(), "失效后应未中");
}

/// 键语义：源变更 → 键变（规格「源变更由键哈希自然区分」）。
#[test]
fn key_discriminates_source_and_location() {
    let k1 = cache_key("(+ 1 2)", "a.krf");
    let k1b = cache_key("(+ 1 2)", "a.krf");
    let k2 = cache_key("(+ 1 3)", "a.krf");
    let k3 = cache_key("(+ 1 2)", "b.krf");
    assert_eq!(k1, k1b, "同源同位置 → 同键（内容寻址）");
    assert_ne!(k1, k2, "源变更 → 键变");
    assert_ne!(k1, k3, "文件名参与配置指纹（SourceMap 位置一致性）");
}

// ---------------------------------------------------------------------------
// 管线集成（run/eval/check 消费面——会话 thread_local 实例）
// ---------------------------------------------------------------------------

/// 同源二次执行：第二次命中缓存 + 结果逐字节等价（复用安全性）。
#[test]
fn pipeline_run_twice_hits_and_equivalent() {
    cache_reset();
    let src = "(define (f x) (* x 2)) (f 21)";
    let first = run_source_rendered(src, "cache-a.krf").expect("首次执行失败");
    assert_eq!(
        cache_stats().misses,
        1,
        "首次应未中（本测试线程从空缓存起步）"
    );
    let second = run_source_rendered(src, "cache-a.krf").expect("二次执行失败");
    assert_eq!(cache_stats().hits, 1, "二次应命中");
    assert_eq!(first, second, "命中执行结果必须与首次逐字节等价");
    assert_eq!(second, "42");
}

/// 源变更 → 新键未中；原条目不受影响（内容寻址并存）。
#[test]
fn pipeline_source_change_misses() {
    cache_reset();
    let _ = run_source_rendered("(+ 1 2)", "cache-b.krf");
    let _ = run_source_rendered("(+ 1 3)", "cache-b.krf");
    assert_eq!(cache_stats().misses, 2, "异源两次均应未中");
    assert_eq!(cache_stats().hits, 0);
    assert_eq!(cache_entry_count(), 2, "两条目并存");
}

/// eval 路径共享缓存（T1 双路径产物同源——run 存 eval 取）。
#[test]
fn pipeline_eval_shares_cache() {
    cache_reset();
    let src = "(define (g x) (+ x 1)) (g 41)";
    let _ = run_source(src, "cache-c.krf").expect("run 失败");
    let outcome = eval_source(src, "cache-c.krf").expect("eval 失败");
    assert_eq!(cache_stats().hits, 1, "eval 应命中 run 存入的条目");
    assert_eq!(kerf_vm::render_value(&outcome.value, &outcome.heap), "42");
}

/// 停用缓存：直编译（基准对照路径）——统计不动。
#[test]
fn pipeline_disabled_bypasses_cache() {
    cache_reset();
    set_cache_enabled(false);
    assert!(!cache_enabled());
    let _ = run_source_rendered("(+ 2 2)", "cache-d.krf").expect("执行失败");
    assert_eq!(cache_stats().misses, 0, "停用时管线路径不查缓存");
    assert_eq!(cache_entry_count(), 0, "停用时不写缓存");
    set_cache_enabled(true);
}

/// 编译期错误不缓存：同源两次均未中（错误不落条目——诊断可能随版本
/// 演化；运行期错误的前段产物正常缓存——编译成功即等价产物）。
#[test]
fn pipeline_compile_errors_not_cached() {
    cache_reset();
    let src = "(car"; // Read 阶段错误（括号未闭合）
    assert!(run_source(src, "cache-e.krf").is_err());
    assert!(run_source(src, "cache-e.krf").is_err());
    assert_eq!(cache_stats().misses, 2, "编译错误两次均应未中");
    assert_eq!(cache_entry_count(), 0, "编译错误不落缓存");
    // 运行期错误的前段正常缓存（编译成功——内容寻址产物等价）
    let rt_src = "(+ 1 \"a\")";
    assert!(run_source(rt_src, "cache-e2.krf").is_err());
    assert!(run_source(rt_src, "cache-e2.krf").is_err());
    assert_eq!(cache_stats().hits, 1, "运行期错误的编译前段应命中");
}

/// 编译产物等价性（确定性编译证明）：缓存命中产物 == 新编译产物。
#[test]
fn cached_program_equals_fresh_compile() {
    cache_reset();
    let src = "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 10)";
    let _ = run_source(src, "cache-f.krf").expect("执行失败");
    // 命中路径的完整产物
    let cached = compile_source(src, "cache-f.krf").expect("命中编译失败");
    // 停用缓存 → 新编译产物
    set_cache_enabled(false);
    let fresh = compile_source(src, "cache-f.krf").expect("新编译失败");
    set_cache_enabled(true);
    assert_eq!(
        cached.program, fresh.program,
        "缓存命中产物必须与新编译产物逐字节一致（确定性编译）"
    );
    // IR 面等价（消费方产物同源——节点数对齐）
    assert_eq!(cached.ir.len(), fresh.ir.len());
}

/// check_source 缓存观测面：首次未中、二次命中（CheckReport.cache_hit）。
#[test]
fn check_source_cache_hit_flag() {
    cache_reset();
    let src = "(+ 1 2)";
    let r1 = check_source(src, "cache-g.krf").expect("首次检查失败");
    assert!(!r1.cache_hit, "首次应未中");
    let r2 = check_source(src, "cache-g.krf").expect("二次检查失败");
    assert!(r2.cache_hit, "二次应命中");
    // 两次诊断面等价（0 诊断）
    assert!(r1.diagnostics.is_empty() && r2.diagnostics.is_empty());
}

/// 全量失效：条目清零、管线路径回到未中（编译器升级场景——条款 3 全量形态）。
#[test]
fn invalidate_all_returns_to_miss() {
    cache_reset();
    let src = "(+ 5 5)";
    let _ = run_source_rendered(src, "cache-h.krf").expect("执行失败");
    assert_eq!(cache_entry_count(), 1);
    cache_invalidate_all();
    assert_eq!(cache_entry_count(), 0);
    let _ = run_source_rendered(src, "cache-h.krf").expect("失效后再执行失败");
    assert_eq!(cache_stats().misses, 2, "失效后应重新未中");
}

/// 缓存键直接构造（公共 API 形状——消费方可预计算键做外部索引）。
#[test]
fn cache_key_public_shape() {
    let k = CacheKey {
        source_hash: 42,
        config_fingerprint: 7,
    };
    assert_eq!(k.source_hash, 42);
    assert_eq!(k.config_fingerprint, 7);
    let k2 = cache_key("x", "y.krf");
    assert!(k2.source_hash != 0 || k2.config_fingerprint != 0);
}
