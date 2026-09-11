# Stage 1 性能基线（§14.6.4）

> **Author**: Super Z（QA-A 主导，ARCH-A 复核，PM-A 归档）
> **Date**: 2026-09-11
> **Version**: v0.3.0-perf-baseline-r24
> **Status**: Active
> **测量环境**: release profile（opt-level 3）；单机单会话顺序执行；二进制 `target/release/kerf`（r15 代码态 + 批次 F 注释级整理——零语义变化，553:0:0 等价复跑）
> **基线状态**: 批次 F（Stage 1 深审收尾环）正式基线；对比对象 = Stage 0 基线（Task 16 后：fib 88.8ms / gc_stress 160.4ms / clean 构建 5.96s / 二进制 1.021 MiB / 测试 290:0:4）
> **本基线的独特使命**: 补录 §21.5 信号 4 缺口——**自举管线性能口径**（自举 vs 种子前段开销 + 自举切换对用户面的零代价实证 + TD-023 回归登记）

---

## 1. 编译时间（clean 全量构建）

```bash
cd /home/z/my-project/kerf
cargo clean && cargo build --release
```

| 指标 | 本次实测（r16 会话） | Stage 0 基线 | Δ | 判定 |
|---|---|---|---|---|
| clean 全量 release 构建 wall time | **9.86s** | 5.96s | +65% | 与代码量同步（Rust 10,806→**18,056** LOC = +67%；+ 自举 kerf 1,948 LOC + 测试 7,568 LOC）✓ |
| 零警告 | ✅ | ✅ | 持平 | ✓ |

## 2. 二进制体积

| 指标 | 本次实测 | Stage 0 基线 | Δ |
|---|---|---|---|
| `target/release/kerf` | **1.48 MiB** | 1.021 MiB | +45%（< LOC 增幅 67%） |

## 3. 运行时基准（fib(25)——自举切换零代价实证）

```bash
target/release/kerf bench examples/usage/fib.krf 10
```

| 口径 | Stage 1 实测（r16） | Stage 0 基线 | Δ | 判定 |
|---|---|---|---|---|
| bench 内置每轮（= 自举前段 + 编译 + VM fib(25) 执行） | **89.4 / 89.6 ms**（两轮组） | 88.8ms（3×5 均值） | **+0.8%** | **噪声带（±5% 内）——自举切换零性能代价** ✓ |
| 外部单程（进程级，date 差分） | 语义示例 4 项 16~20ms | ~89ms（fib 含执行） | — | 冷进程口径见 §5 |

> **结论（r15 生产切换的核心质量证据）**：读+展开两段切换到 VM 上自举实现后，fib 端到端
> 耗时不变——因为 fib(25) 的 VM 执行（~87ms）主导，自举前段 warm 成本（~2.2ms）仅占
> 2.5%，落在噪声带内。**「语言能表达自身前端」以零用户可感知代价达成。**

## 4. GC 压力基准（TD-023 回归——登记与追踪）

```bash
target/release/kerf bench examples/usage/gc_stress.krf 5
```

| 指标 | 本次实测（3 组） | Stage 0 基线 | Δ | 判定 |
|---|---|---|---|---|
| gc_stress 每轮（3×10^5 分配 + 30k 深递归） | **207 / 214 / 235 ms** | 160.4ms | **+29%~46%** | **超 10% 回归阈值 → P2，登记 TD-023（绑定批次 I2）** |
| 存活数据正确性 | ✅ ⇒ 5 | ✅ | 持平 | 语义不受影响（L-GC） |
| 缩放实验（spin 15000 = 减半） | 59.5ms | — | 2× 分配 → 3.6× 耗时 | **超线性**——深递归根扫描模式 |

> **候选根因**（冷却四参数 256/64/25%/1024 未变，Stage 1 新增变量）：① 根集遍历
> per-cycle `HashSet<usize>` 去重分配（vm.rs `collect_value_roots`）；② `Value` 枚举
> 宽度两轮增长（r5 Symbol + 闭包共享捕获）的栈/帧布局与缓存效应。**处置**：TD-023
> P2 → 批次 I2（TD-008 分代 GC 同轮——分代缩小年轻代扫描集 = 对症）；每次合入后按
> §8 协议复测 gc_stress 5 轮追踪。**不阻塞阶段切换**（§14.5.3 D6 规则：性能瓶颈记录
> 为 Stage N+2 优化项——正确性无影响）。

### 4.1 r24 / 42-e 复测（TD-023 基准重定型 + 对症交付）

**新基准**：`examples/usage/gc_stress_nontail.krf`（非尾形深递归——`(+
(grow (- n 1)) 1)` 参数位 ⇒ 峰值 3×10^4 存活帧 + 10 cons/层；尾形
gc_stress 经 TCO 后帧深 O(1)，非尾形为根扫描压力的真实锚定负载）。

| 口径 | 改前（r23 二进制，stash 重建实测） | 改后（r24） | Δ | 判定 |
|---|---|---|---|---|
| gc_stress（尾形）每轮 | 39.684ms | 38.565ms | **-2.8%** | 噪声带 ✓（验收 ≤5%） |
| gc_stress_nontail（非尾形）每轮 | 144.057ms | **105.147ms** | **-27.1%** | **对症交付** |
| fib(25) bench 每轮 | 86.965ms | 84.222ms | -3.1% | 噪声带 ✓（GcCell 热路径零代价） |
| 非尾形缩放（15k/30k/60k） | 45.07 / 144.06 / 528.33（2×→3.2~3.7×） | 30.89 / 105.15 / 394.14（2×→3.4×） | 常数改善 | 残留超线性如实归因（见下） |

**对症双件**：① `GcCell` 堆根性摘要（`Rc<RefCell<Value>>` → `Rc<GcCell>`
——has_heap 标志由写路径维护；根集枚举对非堆单元 O(1) 跳过——深帧根扫描
的实测主导成本）；② 根扫描缓冲跨周期复用（root Vec + visited HashSet
跨周期 take/归还——无分配化）。**残留如实归因**：超线性（2×→3.4×）由
帧栈内存 churn（每帧 2 Vec 分配）+ 每周期固定成本构成——精确 mark-sweep
栈根扫描的结构性成本（非分配模式缺陷——已治愈）；分代/压缩对该残留对症
有限（TD-008 裁定 DEFER 依据②，见登记册）。

## 5. 自举 vs 种子前段开销（§21.5 信号 4 补录——本基线核心新口径）

**测量方法**：同进程 warm 计时（各 20 次取均值），`compile_front`（生产 = VM 上
reader.krf + expander.krf）vs `compile_front_seed`（Rust 种子 = parity oracle）。临时
探针单元测试采集（采集后移除——探针源码见附录 A，可随时重建复测）。语料 4 份：

| 语料 | 种子前段 | 自举前段 | **比值** |
|---|---|---|---|
| fib.krf（8 行，1 define + 1 递归函数） | 31µs/次 | 2,152µs/次 | **69×** |
| macros.krf（宏示例，define-syntax） | 30µs/次 | 4,696µs/次 | **157×** |
| gc_stress.krf（11 行） | 38µs/次 | 4,394µs/次 | **116×** |
| 合成 300-defines（300 个函数定义） | 1,168µs/次 | 452,640µs/次 | **388×** |
| **冷启动**（首次 compile_front，含自举装载 = 种子编译 reader/expander/preamble.krf + VM 装载） | — | **16.1ms（一次性）** | — |

**三条结论**：

1. **冷启动 16.1ms**：与外部进程口径互洽（语义示例 4 项外测 16~20ms = 进程启动 + 16ms
   自举装载 + 前段 + 执行）——每进程一次性成本，CLI 场景可接受。
2. **warm 自举前段 = 种子的 69~388×，且随程序复杂度放大**：简单程序 69×（2ms 级——
   被 VM 执行掩盖，用户不可感知）；宏程序 157×；**程序规模增长时比值恶化（300 defines
   → 388×，1.5ms/define）**——这是 TD-022（自举 Reader/Expander 帧消耗 O(源字符数)，
   无 TCO）在编译前段的直接体现：种子 Rust 展开器按形式近 O(1)，VM 上 kerf 展开器
   递归消耗帧与指令。
3. **对 Stage 2 的就绪度含义**（§14.6.1.4 复杂度增长评估输入）：批次 I1（编译器本体
   kerf ~80% 化）的编译路径将承担此税——**H2（TCO 决策 + TD-007 Rc 化 + TD-022
   帧消耗裁定）是 I1 的实质前置**，stage-2/plan DAG 的 H → I 次序被本实测验证为正确
   （非仅拓扑偏好）。若 TCO 推迟，I1 的编译器自编译吞吐将按 388× 量级劣化——
   已在 hidden-problems-assessment.md 登记为 Stage 2 排程约束。

## 6. 其余示例（`kerf run`，外部 wall time）

| 示例 | wall time | 输出 |
|---|---|---|
| closures.krf | 19ms | `(4 2)` / `⇒ 42` |
| higher_order.krf | 20ms | `⇒ (1 4 9 16 25)` |
| io.krf | 16ms | 两行输出 + `⇒ 42` |
| macros.krf | 19ms | `(2 1)` / `⇒ 42` |

> 全部 16~20ms = 进程启动 + 16.1ms 自举装载 + warm 前段（µs 级）+ 执行（µs 级）——
> **冷启动主导**，功能锚点正常。trivial bench（`(define x 42) x`）warm 0.015ms/轮。

## 7. 测试套件耗时

| 指标 | 本次实测 | Stage 0 基线 | Δ |
|---|---|---|---|
| 结果 | **553:0:0**（零断言修改等价复跑） | 290:0:4 | +263（测试函数 290→553，忽略清零） |
| 全量 wall（clean 后含测试目标编译） | **25s** | 13.6s | +84%（测试 LOC 3,5xx→7,568 = +114%） |
| 纯执行 wall（测试二进制缓存后） | ~11s | 1.33s | 套件翻倍 + parity 双实现执行 |

## 8. 口径说明（防跨轮数字误比）

1. **bench 内置计时**：先 1 验证轮再 N 计时轮；每轮 = 完整 run_source = **自举前段
   （VM 上 reader.krf + expander.krf）+ 编译 + VM 执行**，不含进程启动。
2. **§5 前段口径**：warm = 同进程 bootstrap 状态已装载；冷 = 首调（含装载）。探针
   在 kerf-driver crate 内（可访问 pub(crate) 双路径），非公共 API 变更。
3. **gc_stress 与 fib 的回归归因隔离**：两者 bench 每轮均含自举前段——fib 前段
   ~2.2ms（2.5%）而 gc_stress 前段 ~4.4ms（1.9%），**前段差异不足以解释 gc_stress
   的 +75ms** → 回归根因在 VM 执行/GC 侧（TD-023 候因成立）。
4. **环境声明**：单机顺序执行、同日同会话；跨会话比较按 ±5% 环境噪声预留。

## 9. 性能回归检测协议（§14.6.4 执行协议 3）

| 规则 | 内容 |
|---|---|
| 触发时机 | 每次代码变更合入后重跑 §3/§4（fib 5 轮 + gc_stress 5 轮，约 2s）；涉及构建配置/依赖/前段变更时追加 §1/§2/§5（探针重建——附录 A 源码 30 行） |
| 回归判定 | 单指标劣化 >10% 判回归（P2 起）；+5%~10% 观察项；±5% 噪声带 |
| **当前回归/观察项（r24 后 1 项）** | ① ~~TD-023~~ **resolved（r24/42-e）**——非尾形基准重定型 + 对症（-27.1%）+ 尾形/fib 噪声带（§4.1）；② 自举前段 69~388× 比值（**非回归**——r15 切换的既定架构成本 + 随复杂度放大特性登记为 Stage 2 排程约束，见 §5 结论 3） |
| 记录义务 | 每次重跑在 §10 复测记录追加一行；基线表数字只在判定为新基线时整体更新 |
| 同步义务 | 本文档数字同步至 docs/tests/pipeline-test-coverage.md 性能基线小节（36-d 同批重写该文件） |

### 9.1 复测记录

| 日期 | 触发变更 | fib(25) | gc_stress | 判定 |
|---|---|---|---|---|
| 2026-09-11 | 批次 F 建立本基线（r15 代码态） | 89.4~89.6ms | 207~235ms | fib 噪声带 ✓；gc_stress TD-023 回归登记 |
| 2026-09-11 | r18 TCO（40-c） | — | 61~62ms | TD-023 证据基础失效（尾形帧压消失）——降级 P3 + 残留转非尾形锚定 |
| 2026-09-11 | **r24 / 42-e TD-023 对症**（GcCell 摘要 + 缓冲复用；stash 重建 r23 二进制同会话对拍） | 86.97→84.22ms | 39.68→38.57ms | 双双噪声带 ✓；**非尾形基准 144.06→105.15ms（-27.1%）**；残留超线性归因帧栈结构性成本（§4.1）——TD-023 resolved |

## 10. 性能热点识别（§14.6.4 执行协议 4）

| 热点/候选 | 状态 | 证据/处置 |
|---|---|---|
| gc_stress 深递归根扫描（per-cycle HashSet + Value 宽度） | **resolved（r24/42-e）** | TD-023 对症双件（GcCell 摘要 + 缓冲复用）——§4.1 复测：非尾形 -27.1% / 尾形 + fib 噪声带 |
| 自举前段帧消耗 O(源字符数)（无 TCO） | **架构成本 + 排程约束** | 388× 比值实测（§5）；TD-022 → 批次 H2（**I1 实质前置**——DAG 次序实测验证） |
| IR 旁路计算（O(n) 单遍） | 观察候选（非热点） | TD-015 维持登记（run/eval 路径 IrGraph 旁路丢弃） |
| CALL 弹栈布局（App 顺序对调） | Stage 0 观察项已收敛 | Stage 0 基线 +5.1% → 本轮 +0.8% 噪声带——判定为顺序对调真实成本稳定，观察项关闭 |

## 11. 基线数字总表

| # | 指标 | 本次实测（2026-09-11） | Stage 0 基线 | Δ |
|---|---|---|---|---|
| 1 | clean release 构建 | 9.86s | 5.96s | +65%（同步 LOC） |
| 2 | 二进制体积 | 1.48 MiB | 1.021 MiB | +45% |
| 3 | fib(25) bench 每轮 | 89.4~89.6ms | 88.8ms | +0.8% 噪声带（**自举零代价**） |
| 4 | gc_stress 每轮 | 207~235ms | 160.4ms | **+29~46% TD-023 回归** |
| 5 | 自举前段 warm（fib 语料） | 2,152µs | — | 新口径（69× 种子） |
| 6 | 自举前段比值区间 | **69~388×** | — | 新口径（随复杂度放大） |
| 7 | 冷启动（含自举装载） | 16.1ms | — | 新口径（每进程一次性） |
| 8 | 语义示例外部 | 16~20ms | 各 1ms | 冷启动主导（可接受） |
| 9 | 测试全量 | 553:0:0 / 25s | 290:0:4 / 13.6s | +263 用例 |

**总评**：9 项指标中 5 项同步增长正常（构建/体积/测试——与代码量同步）、1 项零代价
实证（fib 自举切换）、1 项冷启动新口径（16ms 可接受）、1 项架构成本登记（前段比值——
Stage 2 排程约束）、**1 项判回归（gc_stress TD-023 → 批次 I2 带显式处置路径）**。
**性能侧不阻塞阶段切换**（回归项正确性无影响且有绑定偿还节点）。

---

## 附录 A：§5 前段探针源码（临时部署于 kerf-driver/src/driver.rs tests 模块，采集后移除——重建即复测）

```rust
#[test]
fn front_perf_probe() {
    let fib = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../examples/usage/fib.krf")).unwrap();
    let macros = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../examples/usage/macros.krf")).unwrap();
    let big: String = (0..300)
        .map(|i| format!("(define f{i} (lambda (x) (+ x {i})))\n")).collect();
    let corpus = vec![("fib.krf", fib), ("macros.krf", macros), ("gc_stress.krf", gc),
        ("synthetic-300-defines", big)];
    let t0 = std::time::Instant::now();
    let _ = compile_front(corpus[0].1, corpus[0].0).unwrap();   // 冷：含自举装载
    println!("PERF-PROBE cold = {}µs", t0.elapsed().as_micros());
    for (name, src) in corpus {
        let _ = compile_front_seed(&src, name).unwrap();        // warm
        let n = 20u32;
        let t1 = std::time::Instant::now();
        for _ in 0..n { let _ = compile_front_seed(&src, name).unwrap(); }
        let seed_us = t1.elapsed().as_micros() / n as u128;
        let t2 = std::time::Instant::now();
        for _ in 0..n { let _ = compile_front(&src, name).unwrap(); }
        let boot_us = t2.elapsed().as_micros() / n as u128;
        println!("PERF-PROBE {} seed={}µs bootstrap={}µs ratio={:.2}x",
            name, seed_us, boot_us, boot_us as f64 / seed_us as f64);
    }
}
```

> 运行：`cargo test -p kerf-driver --release front_perf_probe -- --nocapture`
> （探针为临时测量载体非交付物——553 交付口径不含它；§14.6.4 协议 3 的复测
> 重建成本 = 追加 30 行 + 单命令运行）。
