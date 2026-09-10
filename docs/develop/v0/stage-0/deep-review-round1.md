# Stage 0 深度审查报告（Round 1）

> **Author**: Super Z（ARCH-A 主导，QA-A/REV-A/PM-A 会签）
> **Date**: 2026-09-10
> **Version**: v0.1.0-deep-r1
> **Status**: Active
> **基线**: lang-design v5.1 / 204 测试全绿（release 复验）/ cargo tree 零外部依赖
> **审查方法**: §14.5 D1-D8 八维度 + §14.8 设计偏差对照；事实采集由 4 路独立子代理（T14-a 架构 / T14-b 测试 / T14-c 偏差 / T14-d 性能与文档）实测提供，全部结论附 file:line 证据

---

## 1. 执行摘要

Stage 0 的**功能正确性基线良好**（204 测试 release 全绿复验、fib(25) 84.4ms 复现、DAG 无环、§11 合规 5/6、生产代码零 unwrap/零 TODO/37 文件全 `//!` 头），12 能力模型与四项接口预留全部交付。但深度审查发现 **4 项 P1 级缺陷**，其中两项触发 SOP 的**自动 NEEDS REVISION 条款**（§9.4.3 正负比例、§7.3.1 审计集缺位）：

- **P1×4**：① 全局正负测试比 1:0.24（154:37），7 个测试分类负测为零（§9.5.2 规则 6 逐分类判 P1）；② §7.3.1 要求的 ≥30 case 门审计集不存在，**此前 gate-review R1 的 PASS 判定因此无效**（强制失败条件："审计集 < 30 case……自动触发 NEEDS REVISION"）；③ opcode.rs 冻结计数守护测试失效（断言 39、枚举数组漏 DefineGlobal、注释写"41 个"）；④ App 求值顺序双路径分裂（eval 先函数、编译先参数——T1 定理未被测试覆盖的反例面）。
- **P2×8**：文档计数/表面失真集（matrix.md 表列合计 200 vs 头 204、README 200/39、status.md 39 操作码、09-stdlib "20 项"实为 24、GC 基线 0.72s 陈旧夸大、web hero "集成 73"实为 77）；§7.1.1 七类负向矩阵缺 2 类（空应用/模块循环依赖，5/7 < 6/7 门限）；E4/E7/E8/E0002-E0004 零直接断言；TD 登记册自身缺陷（TD-001/006 去向不明、详情 6/9）；19 处 catch-all 无臂级注释（含 vm.rs:648 生产 GC 路径静默空臂）；根 CLI `tokens`/`stx` 子命令绕过 driver 直调 reader（§14.7.2 清单偏差）；examples/ 根目录平铺 6 个 .krf 违反 §9.6.2 规则 1 且无 README 索引；pipeline-test-coverage.md 结论夸大（"16 类均有正负向覆盖"不实）且缺 §14.6.1.1 完整性审查小节。
- **P3×5**：docs/scripts 与 docs/tools 的 setup.md 字节级重复；03 instantiate 传递依赖契约超述；syntax-rules Stage 0 单层省略号边界未注记；NFC 保守子集裁剪未入设计；GC 压力基准口径漂移。

**建议行动：NO-GO → NEEDS REVISION**。按 §14.5.3（阻塞项必须本阶段修复），进入修复内循环（Task 16），修复后以 Round 2 复审。**预计修复量级：新增 ≥420 个负向测试 case（表格驱动）+ 30-case 审计集 + 4 处代码修复 + 文档批量对账。**

---

## 2. 八维度审查结论

### D1. 架构健康度（§11 接口隔离）

- **现状**：9 crate 依赖图实测为 **DAG 无环**（24 条成员边 + 根 9 条 + 1 条 dev-dep），拓扑序存在：`{span, runtime} → syntax → {core, reader} → {expander, compiler} → vm → driver → 根`；零外部依赖；lib.rs 显式 re-export 29 条/9 crate（零 glob，B5/B6 检查 PASS）。§14.7.2 清单适配结果：B1/B2/B3/B5/B6 全零匹配 PASS；B4"reader 仅 driver 调用"在 9 成员 crate 范围内 PASS（expander 3 处命中全在 `#[cfg(test)]` 且 reader 为其 dev-dependency）。
- **风险**：① 根 CLI `src/main.rs:15/140/168` 的 `tokens`/`stx` 自调试子命令绕过 driver 直调 reader——严格按 §14.7.2 属偏差（根 crate 非 9 层成员，需裁定：属调试工具豁免或暴露 driver 转发）；② kerf-expander 2276 LOC 占 crates 22.2%（expander.rs 单文件 1350 行）——职责仍单一但已是下一阶段拆分候选；③ L6→L7 定向裁定（vm→runtime）已在 data-flow.md 文档化但 **15-architecture-layers.md 未回写**（见 §6 偏差 #19）。
- **建议**：driver 增加 `dump_tokens`/`dump_stx` 公共转发（≤15 LOC），根 CLI 改走 driver；expander 拆分推迟至 §13.2 切换期（记录为 TD-012 候选）。

### D2. 技术债清单

- **现状**：TD-002~TD-011 共 9 条全部开放（P2×2：TD-004 scope-set 解析、TD-007 迭代式展开；P3×7），目标阶段 Stage 1×4 / Stage 2×5；9/9 有代码注释锚（可追溯性良好）。**登记册自身缺陷**：TD-001、TD-006 编号缺席且无"已解决"留痕（§6.2.1 要求）；详情小节仅 6/9（TD-005/008/011 只有索引行）。
- **风险**：编号断档使"已解决项确实消失"的大阶段末全审（§6.2 规则 2）无法闭环；三条无详情项的偿还计划不可考。
- **建议**：补 TD-001/TD-006 去向记录（推断：已随早期轮次解决或合并——须查 worklog 落实，禁止臆测）；补 3 条详情小节；新增 TD-012（expander 拆分候选，P3）。

### D3. 测试覆盖深度（正负比例——§14.5.1 完成标准 #8）

- **现状**：204/204 全绿（release 复验 16 套件逐一核对：单元 127 + 集成 77）。**正负实测（三口径）**：严格错误路径标记 154 正 : **37 负 = 1:0.24**；含弱信号 1:0.32；按函数名语义词 1:0.19。**远低于 §9.4.3 强制的 1:3**。负测分布严重失衡：reader 7 / expander 8 / vm 12 / driver 2 / span 1 / pipeline 3，而 **compiler、core、runtime、syntax、gate、gc_tests、compiler_tests 七个分类负测为 0**（§9.5.2 规则 6：不达标分类记 P1）。§7.1.1 七类矩阵覆盖 5/7（缺：空应用 `()`——expander.rs:151 已实现报错但零测试；模块循环依赖——零测试）；5/7 < 6/7 门限，按规则触发 NEEDS REVISION。语义码 E4（NotCallable）/E7（I/O）/E8（内部不变式）与结构码 E0002/E0003/E0004 **零直接断言**（仅经消息/Stage 前缀间接覆盖）。
- **风险**：T14-c 发现的 App 求值顺序双路径分裂（D5/偏差 #13）恰是无负测覆盖的语义面——负测缺位已实际放过一个 T1 反例面；"编译器的核心价值是拒绝错误程序"（§9.4.3），当前证据链不足以支撑该论断。
- **建议**：见 §4 行动计划 A1（≥420 case 表格驱动负测扩张 + 7 类矩阵补全 + E 码直接断言矩阵）。

### D4. 下一阶段（Stage 1）就绪度

| Stage 1 需求（v0.5-roadmap / 12-roadmap） | 当前状态 | 差距 | 解阻计划 |
|---|---|---|---|
| 语义基座：9 原语 + 双路径一致求值 | ✅ R1-R9/E0-E8 齐备 | App 求值顺序分裂（P1-④） | 修复后加双路径负例 |
| Stage 0 VM 作为 Stage 1 后端 | ✅ 40 操作码 switch-dispatch + 调试信息 | 冻结计数守护测试失效 | 修复守护测试 |
| 卫生宏展开器（Stage 1 前端重写基础） | ✅ MAX_EXPANSION_DEPTH=128 + 一致性重命名 | TD-004/TD-007（P2）按计划 Stage 1 偿还 | 已入册 |
| GC 堆（Stage 1 程序分配基座） | ✅ 五来源根集 + 冷却退避 | 05 文档未冻结第五来源与数值 | 本轮回写（偏差 #24/#25） |
| 接口冻结（Effects/多阶段/能力 IO/缓存） | ✅ Probe 测试证明冻结 | 13 §3.1 文档伪签名与实现漂移 | 本轮回写（偏差 #16） |
| 类型检查器宿主扩展点 | ⚠️ reserved/ 无类型槽位 | Stage 1 规划职责（§21），非本阶段缺陷 | 纳入 Stage 1 规划输入 |

- **风险**：文档失真（39 vs 40 操作码等）若带入 Stage 1 规划，会以错误基线开新阶段。
- **建议**：本报告 §6 偏差清单全部回写后再进入 §21 Stage 1 规划。

### D5. 设计合理性

- **现状**：9 原语 ADT 逐字段与 01-core-forms 一致（9/9）；R1-R9/E0-E8 双侧齐备且可构造；DefineGlobal 双路径语义与 06 v5.1 完全一致（value;DUP;DefineGlobal / Env::define bool / E6）；CapabilityIO 冻结签名逐字一致；MAX_EXPANSION_DEPTH/ModuleRegistry 七方法/HygieneCtx 全一致——**核心语义设计↔实现契合度高**（26 项偏差中仅 2 项触及语义行为）。
- **风险**：① **App 求值顺序**：06 §1.3/§2 A1 承诺"函数先求值、双路径一致（T1 归纳基础）"，eval.rs:134 先函数、compile.rs:256-263 先参数（自注"被调者最后"）——**04 与 06 文档内部亦冲突**；错误排序场景（`((undefined-a) undefined-b)`）双路径可观测分歧，构成 T1 定理的反例面；② 操作码冻结契约出现"文档-测试-代码"三方漂移（enum 40 / 守护测试 39 / 注释 41）；③ Value::Unit 存在于代码但不在 06 §1.2 值域（"一一对应"声明为假）；④ 内置函数表面 24 项 vs 文档 20 项，且 read-line/print vs read_line/write_line、null? vs nil? 表面错位。
- **建议**：App 顺序按 06 契约修复编译侧（fn 先入栈，CALL 弹参后弹 fn）并加双路径错误排序负例；守护测试重写为 40 项显式枚举；Unit/内置表面按"实现即事实"回写。

### D6. 性能与可扩展性

- **现状**（release 实测）：clean 构建 5.75s；二进制 1.01 MiB；**fib(25) bench 口径 84.443 ms/轮（声称 84.7 复现 ✅，外部 time 含启动 ~87ms）**；GC 压力 3×10^5 单轮 ~0.17s（status.md 声称 0.72s——陈旧/口径混合，方向保守夸大 4.2×）；gc_tests 10^6 分配有界测试通过（0.66s）。
- **风险**：无 O(n²) 已知热点；expander.rs 1350 行单文件在 Stage 1 前端重写时是认知负担（非性能）；GC 冷却参数（256/64/25%/1024）未冻结进 05 文档，调优无基准锚。
- **建议**：performance-baseline.md（§14.6.4）以本轮实测值建立正式基线（含口径声明）；status.md GC 数字同步更新。

### D7. 文档与知识传承

- **现状**：docs/ 47 个 .md / 9365 行；元数据四要素 45/47（缺：stage-committee-process.md——自有头格式、worklog.md——协议声明型）；00-overview v5.1 修订记录存在 ✅；worklog 镜像 diff=0 ✅；data-flow 两图覆盖全管线 ✅。
- **风险**：① 计数失真三处（matrix 表列合计 200 vs 头 204、README 200/39、web hero 73）会让"新 Agent 仅凭文档理解架构"（D7 目标）产生错误基线；② docs/scripts/rust/setup.md 与 docs/tools/rust/setup.md 字节级重复（md5 同）；③ pipeline-test-coverage.md 缺 §14.6.1.1 完整性审查小节——本报告 §6 偏差 #12 的载体缺失。
- **建议**：Task 16 统一对账；重复 setup.md 合并（§3.3 合并>新增）；补完整性小节。

### D8. 测试路径覆盖与流水线印证

- **现状**：pipeline-test-coverage.md 仅 29 行、16 行路径表；实核 6 类零负测（图 IR lower、compile 回填/常量池/闭包捕获、GC、reserved 冻结）；其"16 类均有正负向覆盖"结论**与事实不符**（夸大）；"vm 栈下溢 ✅"无对应测试；§9.5.1 三层结构（Tier1 阶段内/Tier2 阶段间/Tier3 全流程）未按格式记录（缺测试 ID/名称/覆盖阶段/预期输出/状态四要素与正负比例统计）。
- **风险**：D8 是外循环投票的数据源——失真结论会直接污染 §6.3 投票证据链。
- **建议**：重写 pipeline-test-coverage.md 为 §9.5.1 记录格式 + §14.6.1.1 完整性小节（本报告已有 catch-all 42 处/静默 1 处/unwrap 0 的实测数据可直接入册）。

---

## 3. 委员会投票（Round 1——§14.3 流程第 6 步）

| 角色 | 投票 | 理由（引用条款） |
|------|------|------|
| ARCH-A | **NO-GO（NEEDS REVISION）** | P1-④ App 求值顺序分裂违反 T1 定理前提（§6.1 P1"语义错误"）；15/13 文档漂移将污染 Stage 1 规划基线 |
| DEV-A | **NO-GO** | P1-③ 冻结守护测试失效使 40 操作码契约无守卫（§2.3-9 正确>妥协）；修复量可控（本轮内完成） |
| QA-A | **NO-GO** | §9.4.3 比例 1:0.24 + §7.3.1 审计集缺位 = 两条自动 NEEDS REVISION 条款同时命中；gate R1 PASS 判定依 §7.3.1 强制失败条件无效 |
| ALG-C | **NO-GO** | 空应用/循环依赖 5/7 < 6/7（§7.1.1 审查规则）；E4/E7 零断言使语义层证明链不完整 |
| SKL-A | **APPROVED WITH CONCERNS**（不参与加权） | CLI/示例/文档工程面良好；仅 examples/ 布局违规（§9.6.2）待整理 |

**加权票：0/5.5 通过（0%）< 95% → NEEDS REVISION 确认**。触发条件：P1 未清零（§5.3 硬条件 1）。

---

## 4. 行动计划（修复内循环 = Task 16 范围）

### A. 代码修复（P1，阻塞）

| # | 修复项 | 载体 | 验证 |
|---|--------|------|------|
| A1 | App 求值顺序统一为"函数先"（06 §2 A1 契约）：compile.rs App 编译改为 fn 先求值入栈 + vm.rs CALL 弹参/弹 fn 顺序对调 + 双路径错误排序负例 | kerf-compiler/src/compile.rs、kerf-vm/src/vm.rs | 新增负例：`((undefined-a) undefined-b)` 双路径同报 undefined-a |
| A2 | opcode.rs 冻结计数守护测试重写：显式枚举全部 40 项（含 DefineGlobal、谓词组、Mod/Not）+ 模块头分组注释补全（8 组） | kerf-compiler/src/opcode.rs | 守护测试断言 40 并逐项列举 |
| A3 | ≥30 case 门审计集：`examples/audit/stage0_gate_audit_r1.rs`（10 单语句 + 10 多语句 + 5 复杂 + 5 错误恢复 + 2 备份，覆盖 §7.1.1 全 7 类含空应用与模块循环依赖）+ Cargo.toml `[[example]]` 声明 | kerf/examples/audit/ | `cargo run --example stage0_gate_audit_r1` 退出码 0，逐 case 报告 |
| A4 | 根 CLI reader 直调改走 driver：driver 增加 `dump_tokens`/`dump_stx` 转发，main.rs 改调用 | kerf-driver、src/main.rs | grep 复验 §14.7.2 B4 全 PASS |

### B. 负向测试扩张（P1-①，阻塞）

- 目标：全局正负比 ≥1:3（负 ≥462）且逐分类非零。手段：**表格驱动** case（每行 = 一个独立测试 case，含 CATEGORY/POLARITY/EXPECTED 断言），新增 4 个测试文件：
  - `negative_reader_tests.rs`（~45 case：词法/语法全错误类、E0001/E0002 直接断言、多错误收集）
  - `negative_expander_tests.rs`（~55 case：空应用、关键字误用矩阵、E0003 直接断言、相位 API 负例含循环依赖）
  - `negative_vm_tests.rs`（~130 case：24 内置 × 类型/元数错误系统表、E1/E2/E4/E5 直接断言、div-zero、not-callable）
  - `negative_semantics_tests.rs`（~90 case：E6/E3 双路径、App 顺序、set!/define 分裂、深递归、错误恢复、消息形状（码+Span+Stage））
- 同时补齐 7 类矩阵剩余 2 类 + E0004/E7/E8 直接断言（E8 经公开 API 不可触发者记 DEFERRED+理由）。

### C. 文档对账与回写（P2，与 §14.8 联动）

| 文档 | 动作 |
|------|------|
| 04-bytecode-vm.md | 操作码全 40 项显式枚举（8 组含谓词组）、分组数统一、漂移注记改为真 |
| 05-runtime.md | HeapObj 六变体（删 Boxed 契约错写）、alloc_boxed 第 7 入口注记、根集五来源、GC 数值冻结（256/64/25%/1024）、算法描述对齐 slot-Vec+显式工作栈、`>` 语义 |
| 06-operational-semantics.md | 值域补 Unit；App 顺序契约确认（修复后双侧函数先） |
| 09-stdlib.md | 24 内置准确清单（null?/read-line/print/mod/<=/>=/list/not/eq?/str-append）+ I/O 表面说明 |
| 13-capability-matrix.md | §3.1 以 reserved.rs 冻结签名回填（三 trait）+ 能力模型 I/O P3→P2 |
| 15-architecture-layers.md | §1.5 附 7 层→9 crate 映射表 + vm→runtime 定向裁定 + Runtime C→Rust 语义验证口径注记 |
| 03-macro-system.md | instantiate Stage 0 简化注记 + syntax-rules 单层省略号边界 |
| 02-syntax-model.md | Token 叶级 44 计数更正 + NFC 保守子集注记 |
| 01-core-forms.md | Runtime 行 C→Rust 口径注记（与 15 联动） |
| 12-roadmap.md | GC 验收口径 10^6（以 gc_tests 10^6 为准）+ benchmarks 锚点改指 CLI bench |
| docs/tests/matrix.md | 分套件表逐行对账（204） |
| docs/tests/pipeline-test-coverage.md | 重写：§9.5.1 三层记录格式 + §14.6.1.1 完整性小节（catch-all 42/静默 1/unwrap 0）+ 真实正负比例 |
| README.md / status.md / RELEASE_NOTES | 204/40/24/LOC 10573/GC 0.17s 对账 |
| tech-debt-register.md | TD-001/006 去向 + 3 条详情 + TD-012 新增 |
| 06/05/03 的测试锚点表 | 随新增负测同步 |

### D. 工程整理（P2/P3）

- examples/ 重组：6 个 .krf → `examples/usage/`，新增 `examples/README.md` 索引（§9.6.2）；引用路径同步（CLI bench 默认路径、gate 测试）。
- 19 处无注释 catch-all 补臂级理由（C1 卫生）；vm.rs:648 GC 静默空臂补注释。
- docs/scripts 与 docs/tools 重复 setup.md 合并（§3.3 合并>新增）。

### 修复后门审查（Gate R2）

审计集就位后按 §7.3 重跑门审查（Round 2），≥30 case 全绿 + 正负比例复测，方可进入 §6.3 外循环投票。

---

## 5. 结论（Round 1）

**NEEDS REVISION**（P1×4 未清零；§9.4.3/§7.3.1 自动触发条款命中）。修复行动计划见 §4；修复完成 + §3.2 全绿后出 Round 2 复审。

---

## 6. 设计偏差清单（§14.8——T14-c 10 检查点实测，去重 26 项）

> 分类：B1=实现<设计；B2=实现>设计；B3=实现≠设计（判优后回写）；B4=设计灰区（实现即事实，补写设计）。逐项证据见 T14-c 报告（worklog Task 14-c）。

| # | 类型 | 设计文档章节 | 偏差描述 | 最优判断 | 重构判断 | 回写动作 |
|---|------|-------------|---------|---------|---------|---------|
| 1 | B1 | 03 §2.3 | syntax-rules 点对/尾省略号/`(… tpl)`/嵌套省略号未实现（03 文法超述） | 实现合理（Stage 0 骨架） | N/A（Stage 2 TD-005） | 03 补 Stage 0 裁定注记 |
| 2 | B1 | 12:29 | GC 验收"10^6"实为 3×10^5 示例（gc_tests 有 10^6） | 文档为真（测试在） | N/A | status 口径注明双轨 |
| 3 | B2 | 05 §3.1 | 第 7 分配入口 alloc_boxed | 实现即事实 | N/A | 05 补注记 |
| 4 | B2 | 03 §2.1 | 旧接口 Transformer::apply 未列文档 | 保留（内部使用） | N/A | 03 标注 |
| 5 | B2 | 01 | when/unless/let* 糖超推导示例 | 灰区填满 | N/A | 01 注"非穷举" |
| 6 | B2 | 12 | CLI 10 子命令/web/204 测试超额 | 良性 | N/A | 保持 |
| 7 | **B3** | **04/opcode.rs** | **冻结计数测试断言 39、数组漏 DefineGlobal、注释 41、模块头漏列** | 代码 enum=40 为真 | **本阶段修复（P1-③）** | 04 全枚举回写 |
| 8 | B3 | 04:24 | Ne 文档有代码无；谓词组×5+Mod+Not 代码有文档无；分组数 7/5/5/8 混乱 | 代码为真 | N/A | 04 重写分组清单 |
| 9 | B3 | 03:51-53 | instantiate 无传递依赖递归（单模块 Stage 0 无可观测差异） | 文档超述 | 切换期 | 03 契约改述 Stage 0 简化 |
| 10 | B3 | 05:48-49 | HeapObj 无 Boxed 变体（05 契约注记失真，代码有 TD-010 裁定） | 代码为真 | N/A | 05 改写 |
| 11 | B3 | 05 §2/§3.1 | bump-pointer+递归标记摘要失真（实现 slot-Vec+free-list+显式工作栈） | 实现更优 | N/A | 05 对齐 |
| 12 | B3 | 05:91 | 冷却状态在 vm.rs 非 Heap；`>` vs `≥` | 实现为真 | N/A | 05 补归属+语义 |
| 13 | **B3** | **06 §1.3/§2 A1 ↔ compile.rs:256-263** | **App 求值顺序双路径分裂（fn先 vs 参数先）** | **设计（fn 先）为优**（Racket 惯例、T1 一致性） | **本阶段修复（P1-④）** | 修复后 04/06 确认 |
| 14 | B4 | 06 §1.2 | Value::Unit 不在值域文法（"一一对应"为假） | 补写 | N/A | 06 值域加 Unit |
| 15 | B3 | 09:31-42 | 内置 24≠20；read-line/print≠read_line/write_line；null?≠nil? | 代码为真 | N/A | 09 重写清单 |
| 16 | B3 | 13 §3.1 | 三 trait 伪签名不可编译（reserved.rs 为冻结权威） | 代码为真 | N/A | 13 以 reserved.rs 回填 |
| 17 | B3 | 02:56/58 | Token "41 种类"不可验证（enum 12/叶级 44） | 重算钉测 | N/A | 02 更正+守护 |
| 18 | B4 | 02 §6 | NFC 保守子集（零依赖裁定）未入设计 | 补写 | N/A | 02 注边界 |
| 19 | B3 | 15 §1.5 / 12:440-480 | 三依赖边 crate 不成立；syntax 无槽位；7 层 vs 9 crate；Runtime C→Rust；reserved/ 4 文件→单文件 | 代码为真（workspace 裁定） | N/A | 15 附映射表 |
| 20 | B3 | status.md:15 | 仍写 39 操作码 | 计数对账 | N/A | 本轮更正 |
| 21 | B3 | 12:77/268 vs status/reserved.rs | 能力模型 I/O P3 vs P2 | P2 自洽（有完整行为规格） | N/A | 12 回改 P2 |
| 22 | B3 | 04:173/12:476 | benchmarks/ 目录不存在（CLI bench 替代） | 载体等价 | N/A | 锚点改指 |
| 23 | B4 | 04:24 | 谓词组/Mod/Not 无分组归属 | 并入 #8 | N/A | 并入 #8 |
| 24 | B4 | 05 §3.1 | GC 冷却数值契约缺失（256/64/25%/1024） | 补写 | N/A | 05 数值冻结 |
| 25 | B4 | 05:111/gc.rs:13 | 根集第五来源（帧捕获槽）未入四来源不变式 | 实现更完备 | N/A | 05/06 补第五来源 |
| 26 | B4 | 07 vs 13 | 四预留契约位置在 13 §3.1 非 07（导览前提错位） | 引用修正 | N/A | 07 加指引 |

**无偏差确认清单**（§14.8.3 强制要求"即使全一致也须明确记录"）：9 原语 ADT 逐字段（CP1）；操作码总数契约 40=enum 实测（CP2）；MAX_EXPANSION_DEPTH=128、ModuleRegistry 七方法+ModuleEntry 五字段、HygieneCtx 全契约（CP3）；io 双函数逐字、六分配入口、register_foreign_ref 非 no-op（CP4）；R1-R9/E0-E8 全集、DefineGlobal 双路径（CP5）；CapabilityIO 逐字（CP7）；Span 四字段三别名（CP8）；单向无环/L0 无依赖/driver 全依赖（CP9）；§22.3 十二项里程碑（CP10）——共 10 大项确认一致。

---

## 7. 本轮遵循原则

§2.3-9（正确>妥协：App 顺序按设计修编译侧而非改设计）；§2.3-8（设计驱动测试：E 码直接断言矩阵）；§9.4.3/§7.3.1/§7.1.1（负向三条强制条款实测裁决）；§14.5.3（P1 本阶段修复）；§8.4.5（全部结论先查文档+实测取证）；§6.1（P1 定级依据"语义错误"与"测试覆盖严重不足"）。
