# Stage 1 架构设计审查（§14.6.1.2）

> **Author**: Super Z（ARCH-A 主导，QA-A 复核，PM-A 归档）
> **Date**: 2026-09-11
> **Version**: v0.3.0-arch-r16
> **Status**: Active
> **基线**: 553:0:0（本会话 clean 全量复验）/ lang-design v6.2（36-b 回写后）/ 读+展开两段全自举（生产切换 + parity oracle）/ 9 crate + 根 CLI / 双审计集 91 case
> **审查方法**: 按编译管道 7 阶段 × 5 维度（完整性/设计对齐/结构清晰性/效率/扩展性）打分；全部结论附 file:line 或实测证据；引用 deep-review-round1.md D1/D5/D6 结论；对照 Stage 0 架构审查（v0.1.0-arch-r1）的 ⚠️ 项收敛情况

---

## 1. 总体结论

**架构判定：✅ 通过（含 3 项 ⚠️ 观察项，0 项 ❌）**。9 crate + 根 CLI 依赖 DAG 无环、零外部依赖、单向数据流；7 阶段 35 维度评分 32 ✅ / 3 ⚠️ / 0 ❌。**Stage 0 的 4 项 ⚠️ 中 2 项已收敛**（TD-012 expander 拆分——批次 B 落地；扩展点冻结代价——Probe 冻结测试 + 位置断言就位）、1 项转化（IR 旁路 → TD-015 维持登记）、1 项新增观察（driver 内聚自举桥的 Stage 2 拆分候选）。批次 F 深审（36-a）未发现任何跨阶段耦合新增或回流反模式。

### 1.1 架构级实证（全管线横切，本会话实测）

| 项 | 结论 | 证据 |
|---|---|---|
| 依赖 DAG 无环 | ✅ | Cargo.toml 逐 crate 实测拓扑序 `{span, runtime} ← syntax ← {core, reader} ← {expander, compiler, vm} ← driver ← 根`；**driver 为唯一全依赖成员（组合根）**；vm→compiler 四处引用全为数据类型（vm.rs:23 use + 482/487 CaptureSource 模式 + 866 cfg(test)） |
| 零外部依赖 | ✅ | 全部 10 个 Cargo.toml `[dependencies]` 仅含 kerf-* 成员（自举信任根显式化——含 1,948 行自举 kerf 源码也零第三方） |
| §14.7.2 清单 | ✅ | B1 compiler→reader 0 / B2 vm→compiler 内部 0 / B3 compiler→driver 0 / B4 根 CLI 直调 reader 0（Stage 0 A4 修复保持）/ B5 glob re-export 0——五项 grep 实测全 PASS |
| 模块文档头 | ✅ | 66 个生产 .rs 文件 66/66 以 `//!` 开头（head -1 全量核对，36-c C4） |
| catch-all 注释纪律 | ✅ | 真 `_ => {}` 通配臂全库仅 1 处且带臂级理由（vm.rs:779 前置行）；其余 14 处空臂为显式变体臂（语义自明） |
| 生产区 unwrap 纪律 | ✅ | 生产区 `.expect()` 4 处全部为不变式说明型（expander.rs:247「调用方已校验」/ phase.rs:110/116/133「上方已检查」）；测试区 unwrap 不在约束内 |
| 测试基线 | ✅ | 553:0:0（clean 复验 + 36-c 零断言等价复跑双确认）；正负比 ≈1:3.15 |
| 自举架构 | ✅ | 种子（Rust）→ VM 上 kerf（reader.krf/expander.krf/preamble.krf）两层；生产 compile_front 走自举、引导与 parity 走 compile_front_seed——**单一编译逻辑两分流出口**（fast_path_bytecode_matches_full_compile 守护字节码一致性） |

### 1.2 LOC 分布（实测，2026-09-11）

| crate | LOC（文件数） | 主要文件 | 备注 |
|---|---|---|---|
| kerf-span | 567（4） | diagnostic.rs | 持平 Stage 0 |
| kerf-syntax | 701（4） | symbol.rs / stx.rs / scope.rs | +24（Stx 代次字段 v2 等） |
| kerf-core | 1,197（4） | expr.rs / ir.rs / code_value.rs | +106（作用域集 + LiteralValue::Symbol） |
| kerf-runtime | 595（4） | heap.rs 436 | +20（alloc_symbol 等） |
| kerf-reader | 1,091（4） | lexer.rs / parser.rs | +9 |
| kerf-expander | 2,663（6） | macro_sys.rs 715 / expander.rs 632 / core_forms.rs 604 | **+328 且拆为六文件**——Stage 0 ⚠️ TD-012 收敛（原单文件 1,351） |
| kerf-compiler | 2,007（5） | compile.rs 770 / typecheck.rs 669 | +801（**typecheck.rs 新增**——批次 C 类型检查器 R1-R8） |
| kerf-vm | 2,202（4） | vm.rs 1,526 / eval.rs | +456（Value::Symbol / 闭包共享捕获 / eval 卫生接线） |
| kerf-driver | **5,168（13）** | builtins.rs 1,473 / driver.rs 1,053 / toolchain.rs 803 / capability.rs 597 | **+3,936（最大增长）**：builtins 24→52、capability 门控、effects、cache、reserved/ 14 项、bootstrap 桥 ×2 |
| 根（src/main.rs） | 362（1） | CLI 11 子命令 | +65（test 子命令等） |
| **Rust 合计** | **18,056（66）** | | vs Stage 0 10,806（+67%） |
| 自举 kerf | 1,948（3） | expander.krf ~1,180 / reader.krf 471 / preamble.krf 297 | **全新**（r6-r15 交付） |
| 测试 | 7,568（25） | tests/ 阶段树 + runner 单入口 | +~4,000 |

> ⚠️ **观察项 O-1**：kerf-driver 5,168 LOC / 13 文件占全库 28.6%——职责仍单一（管线编排 + 内置注册 + 自举桥 + 能力门控），但 Stage 2 批次 I1（编译器本体迁移）时**自举桥三模块（bootstrap.rs / bootstrap_expander.rs / bootstrap/*.krf 载荷）应随编译器 kerf 化自然外迁**——登记为 Stage 2 结构候选，非本阶段拆分项（拆分无行为收益且引入 churn）。

---

## 2. 逐阶段架构评分（7 阶段 × 5 维度）

### 2.1 reader（kerf-reader 1,091 + 自举 reader.krf 471）

| 维度 | 评分 | 依据 |
|---|---|---|
| 完整性 | ✅ | 词法/语法 Stage 0 全量保持 + 自举 Reader（r6 B3：全 kerf 源码，词法 + 语法 + hofs 序章）经生产管线执行 553 套件 |
| 设计对齐 | ✅ | 02-syntax-model 口径全对（36-a 无偏差确认清单：TokenKind 12 / 叶级 45 / Keyword 22 / Stx 四字段 + 代次 +1） |
| 结构清晰性 | ✅ | token/lexer/parser 三文件；自举侧单文件 reader.krf 含序章（自描述标准库片段——设计使然） |
| 效率 | ⚠️ | **TD-022**：自举 Reader 帧消耗 O(源字符数)（无 TCO）——10^5 字符上限；种子路径无此限制。性能基线 §5 实测 69~388× 比值的组成部分 |
| 扩展性 | ✅ | 多语法可插拔（02 §1——Stage 2 评估项有量化背书）；TokenKind 扩展点 Probe 冻结（r12 位置断言） |

### 2.2 expander（kerf-expander 2,663 六文件 + 自举 expander.krf ~1,180）

| 维度 | 评分 | 依据 |
|---|---|---|
| 完整性 | ✅ | 9 核心形式 + 10 糖 + syntax-rules 全模式面（单层边界）+ 相位簿记 + 模块/导入（r15 注册簿 + 传递依赖）；E1-β 宏收口 + 生产切换 |
| 设计对齐 | ✅ | 03-macro-system v6.2 回写后（ExpandCtxt 四字段 + 全模式面限定词——36-b 落位）；07 §3.2/3.3 交付面全对（parity 36 测试） |
| 结构清晰性 | ✅ | **TD-012 收敛**：expander/core_forms/macro_sys/sugar/phase 五模块 + lib——同层聚块；自举侧 expander.krf 镜像宏系统 |
| 效率 | ⚠️ | 自举 Expander 同 TD-022 帧消耗模式（388× 比值实测——H2 TCO 前置裁定验证）；种子路径无热点 |
| 扩展性 | ✅ | 变换器注册表单表（用户宏覆盖糖名）；变换器 trait 扩展点 Probe 冻结 |

### 2.3 compiler（kerf-compiler 2,007 五文件）

| 维度 | 评分 | 依据 |
|---|---|---|
| 完整性 | ✅ | 40 操作码冻结（守护测试逐项）+ 编译确定性 + **typecheck.rs（批次 C：R1-R8 保守静态 + E0005 多错误收集 + Span 次序）** |
| 设计对齐 | ✅ | 04-bytecode-vm 全篇无偏差（36-a 亲验：操作码/帧/常量池/编译序）；R1-R8 与 09/13 口径一致 |
| 结构清晰性 | ✅ | compile/typecheck/bytecode/opcode 四文件单一职责；依赖仅 span/syntax/core（无 reader——B1 合规） |
| 效率 | ✅ | 编译缓存（cache.rs r7：内容寻址 + 三方法 trait + front 复用）——第二次 check 命中可观测（测试锚点） |
| 扩展性 | ✅ | 操作码扩展 = 枚举 + 守护测试同步（冻结核对）；AnnotatedANF 扩展位为 Stage 2 G1 预留（codegen.rs Probe） |

### 2.4 vm（kerf-vm 2,202 四文件）

| 维度 | 评分 | 依据 |
|---|---|---|
| 完整性 | ✅ | 帧栈协议 + 16 帧追踪 + 双路径（VM 生产 / eval 参考）；闭包共享单元格捕获（r5+）；Value 十变体 |
| 设计对齐 | ✅ | 06 v6.2 回写后值域十变体对齐；R1-R9 归约双侧对应（无偏差确认） |
| 结构清晰性 | ✅ | vm/eval/value 三文件；MAX_FRAMES=10^5 结构化上限（深递归防护） |
| 效率 | ⚠️ | **TD-023**：gc_stress 深递归 GC 根扫描回归（+29~46% 超线性——per-cycle HashSet 分配 + Value 宽度候选根因）；批次 I2 对症 |
| 扩展性 | ✅ | 调试帧槽三扩展位（04 §1）；debug_info_table → 反查 Span（诊断质量锚） |

### 2.5 runtime（kerf-runtime 595 四文件）

| 维度 | 评分 | 依据 |
|---|---|---|
| 完整性 | ✅ | slot-Vec 堆 + mark-sweep（显式工作栈）+ 三函数 I/O 通道 + foreign ref 登记 |
| 设计对齐 | ✅ | 05 v6.2 回写后（七变体/八入口/五来源根/三通道——36-b 落位） |
| 结构清晰性 | ✅ | heap/gc/io 三模块；零依赖 crate（信任根） |
| 效率 | ✅ | 冷却退避四参数冻结；分配 O(1)（空闲表复用） |
| 扩展性 | ✅ | 标记位→分代号升级路径（05 §3.1 留白设计）；ValueSlot 往返对称 |

### 2.6 driver（kerf-driver 5,168 十三文件）

| 维度 | 评分 | 依据 |
|---|---|---|
| 完整性 | ✅ | 管线编排 + 52 内置（能力门控 6 项）+ effects 双层 + cache + reserved 14 项 + **自举桥 ×2（读 + 展开）+ prelude 注入（TD-021）** |
| 设计对齐 | ✅ | 13-capability-matrix §3.1/§3.3-§3.5 逐条对齐（36-a 无偏差确认）；07 自举交付面全对 |
| 结构清晰性 | ⚠️ | O-1：5,168 LOC / 13 文件为全库最大 crate——职责单一但内聚度高；Stage 2 I1 时自举桥外迁候选（见 §1.2 注记） |
| 效率 | ✅ | 冷启动 16.1ms（自举装载——每进程一次）；warm 前段 µs 级（trivial 0.015ms） |
| 扩展性 | ✅ | reserved/ 14 项 Probe 冻结（「测试实现体编译通过 = 契约冻结」先例）；能力门控 fail-closed |

### 2.7 根 CLI（src/main.rs 362）

| 维度 | 评分 | 依据 |
|---|---|---|
| 完整性 | ✅ | 11 子命令（run/eval/check/test/tokens/stx/core/ir/bc/code/bench）——10-toolchain v6.2 已补清单（36-b B5） |
| 设计对齐 | ✅ | 全部经 driver 转发（B4 合规）；bench 载体与 10 §3 对齐 |
| 结构清晰性 | ✅ | 单文件 + cmd_* 函数分派 |
| 效率 | ✅ | —（CLI 壳层） |
| 扩展性 | ✅ | 新子命令 = 分派行 + cmd 函数（12 项匹配表） |

---

## 3. 与 Stage 0 架构审查的对照（⚠️ 项收敛追踪）

| Stage 0 ⚠️ 项 | Stage 1 状态 | 证据 |
|---|---|---|
| TD-012 expander.rs 单文件 1,351 拆分候选 | **✅ 收敛** | 批次 B 前端重写落地六文件（expander.rs 632）；登记册 r16 标记 resolved |
| J6 LOC 观察全库增长 | **✅ 转常态** | LOC 10,806→18,056（+67%）与功能交付同步（类型检查器/能力 I/O/自举 1,948 行 kerf）；无失控单文件（最大 vm.rs 1,526 < 1,600 线） |
| 冻结契约扩展点固有代价 | **✅ 收敛** | reserved/ Probe 冻结测试 8 + 位置断言 12（r12）——扩展点成本显式化 |
| IR 旁路计算（O-1/O-2） | **⚠️ 维持登记** | TD-015（run/eval 路径 IrGraph 旁路丢弃）——O(n) 非热点，Stage 2 切换期重构候选 |
| （新增）driver 内聚自举桥 | **⚠️ O-1 新观察** | Stage 2 I1 外迁候选（非本阶段项） |

## 4. 结论

**✅ 架构通过**——32/35 维度优秀、3 项 ⚠️ 均有登记与节点绑定（TD-022→H2 / TD-023→I2 / O-1→I1 候选）。阶段切换架构侧无阻塞。自举架构（两层 + 双角色种子 + parity oracle + 切换守护）是 Stage 1 的**新增架构资产**——其「单一编译逻辑两分流出口」设计使切换风险被结构性约束（字节码一致性守护 + 553 零回归实证）。

*遵循：§14.6.1.2（架构设计审查五维度）、§14.7.2（合规清单实测）、§11（接口隔离）、§2.4（核心架构原则——混合务实 + 单向/层级依赖）、§2.3-11（实测取证）。*
