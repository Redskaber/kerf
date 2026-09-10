# 接口契约与可替换性深审报告（批次 D 前置设计对齐）

> **Author**: Super Z（ARCH-A/REV-A 角色）
> **Date**: 2026-09-10（r8，Stage 1 批次 D 前置）
> **Version**: v0.1.0-r8
> **Status**: Accepted（发现清单驱动批次 D 范围——26-b/26-c 按本报告裁定实施）
> **输入**: [13-能力矩阵 §3](../../lang-design/13-capability-matrix.md)（四项预留契约权威）；`kerf-driver/src/reserved.rs`（冻结签名实现权威）；[12-路线图 §2.4/§2.5](../../lang-design/12-roadmap.md)（四级做实主题）；[15-架构分层](../../lang-design/15-architecture-layers.md)；[17-设计原则 §26/§27/§28](../../lang-design/17-principles.md)（成熟度匹配/接口稳定性/渐进替换）；[multi-error-recovery-design §4](./multi-error-recovery-design.md)（效应联动裁定）；Stage 1 plan §5 批次 D 节点

---

## 1. 审查范围与方法

**审查对象**：① 四项预留契约（EffectSystem / MultiStage / CapabilityIO / CompilationCache）；② 12 个能力模块的接口契约边界；③ 五条渐进替换路径（§28）的挂点真实性；④ 前置架构约束（13 §4 三项）的落地状态。

**四维标尺**（本报告统一口径，逐契约/逐模块打分并附条款锚）：

| 维度 | 定义 | 判据 |
|------|------|------|
| **D-强度** | 契约形状能否承载其目标语义（含错误路径） | 签名可直接实现目标行为？错误形态结构化？ |
| **D-任意节点** | 机制能否在**管线/执行流的任意深度**挂接，而不污染途经签名 | 无签名线程化即可深位触发？挂接点枚举完整？ |
| **D-边界** | 做什么/不做什么 + 签名无歧义（可测试可审计） | lang-design 三段式与代码签名零漂移？消费方误用会被编译器/测试拦截？ |
| **D-可替换** | §28 渐进替换路径的挂点**真实存在**（非纸面承诺） | 替换 seam 有活体证明（测试/parity/双路径）？ |

**证据来源**：代码实读（reserved.rs 289 行 / builtins.rs 1397 行 / vm.rs 1495 行 / heap.rs / io.rs / driver.rs 685 行）+ 既有验证工件（408 测试 / parity 87+307 / 审计集 41 case / T1 双路径断言）。

---

## 2. 四项预留契约逐项深审

### 2.1 EffectSystem（P3 → r8 做实「编译器内部」）

**契约形状**（reserved.rs 冻结）：`EffectFamily::Result` / `Effect::Family: EffectFamily<Result = Value>` / `EffectSystem { Effect, Handler, Continuation; perform(&self, Effect) -> Value; handle(&self, Handler, impl FnOnce() -> Value) -> Value }`。

| 维度 | 评定 | 分析 |
|------|------|------|
| D-强度 | ✅ 足够（一次性逃逸）/ ⚠️ 不足（多次恢复） | one-shot escape（挂起→向上→处理器接管）形状完备；**resumption 需要 Continuation 可具体化**——当前关联类型未约束，P3 留白属设计意图（行为语义留给引入时实践数据，12 §2.1.2）。Stage 2 语言级引入时须补 continuation 调用形状。 |
| D-任意节点 | ✅ 成立（有代价） | `perform(&self, ...)` **无上下文参数**——唯一可行实现是线程局部处理器栈：任意 Rust 调用深度可触发，零签名污染（这正是 D-任意节点的机械定义）。代价：① 逃逸机制须用 unwind（`catch_unwind`/`resume_unwind`，std-only 合规）；② payload 为 `Box<dyn Any + Send>`，downcast 定向；③ **panic hook 噪声须过滤**（payload 类型判别——私有类型保证仅效应逃逸被抑制）；④ 构建约束：unwind profile（全仓无 `panic = "abort"`，已核验）。 |
| D-边界 | ⚠️ **发现 F1**（P2） | 冻结形状把效应载荷耦合到 `Value`（P3 注记「以 Value 具体化」）——**编译器内部使用**（错误恢复/测试短路）需要原生 Rust 类型载荷，直接实现须把 String/结构体编组进 Value。裁定：双层 API——`Escape<T>` 类型化逃逸（内部机械）+ `InternalEffectSystem` 冻结 trait 适配器（Value 编组证明形状可实现）。Stage 2 细化为泛型关联时消除（reserved.rs 注记已留）。 |
| D-可替换 | ✅ 成立 | 内部机械（处理器栈 + unwind）与契约（trait）分离——Stage 2 语言级引入可换实现不破契约（§27 接口稳定性：Probe 实现测试 + r8 真实现同签名双证）。 |

**联动裁定核验**（multi-error-recovery-design §4）：编译期恢复 ≠ effect——**成立且继续有效**。r8 的内部效应用于**运行期/工具链**的跨嵌套逃逸（测试短路、错误后同进程续跑），不触碰展开器编译期恢复（其形式级恢复仍为控制流，批次 E 落地）。两者共用「错误是数据」（Diagnostic/载荷结构体）但机制分离——§11 接口隔离维持。

### 2.2 MultiStage（P3 保持——Stage 2 做实）

| 维度 | 评定 | 分析 |
|------|------|------|
| D-强度 | ✅ | quote/splice/run 三方法 + `Result<_, RuntimeError>` 错误路径结构化；`Code` 关联类型可具体化为 `CodeValue`（`from_expr` 已存在，Probe 实证）。 |
| D-任意节点 | ✅（P3 口径） | quote 可在任何产出 `&CoreExpr` 的节点调用；splice 语义「代码值→当前阶段代码」在任何展开/合成节点可用。 |
| D-边界 | ✅ | 职责边界明确（良构/良作用域保证 + 不做隐式编译）；Stage 1 无消费面——**不做实**（依赖链：类型系统稳定，12 §2.4.4）。 |
| D-可替换 | ✅ | CodeValue 的 `compose_with` 预留位仍在（13 §2.3）；无实现即无替换风险。 |

**结论**：维持 P3，零行动（「预留长期不做实是允许的——接口预留的成本只有类型维护」12 §2.5.1）。

### 2.3 CapabilityIO（P2 → r8 做实「基础传递」）

**契约形状**：`ReadCapability/WriteCapability { _private: () }`（不可伪造）+ `CapabilityIO::read_line(cap: &mut ReadCapability) -> Result<String, IOError>` / `write_line(cap: &mut WriteCapability, s: &str)` + 4 条行为规格（13 §3.1.3）。

| 维度 | 评定 | 分析 |
|------|------|------|
| D-强度 | ✅ 足够 | 令牌不可伪造（私有字段）+ 线性传递（`&mut` 独占）+ `IOError` 结构化；规格条款 1-4 完整可直接实现（P2 判据）。 |
| D-任意节点 | ❌→✅ **发现 F2**（P2，r8 修复） | **当前失败**：I/O 消费面硬接线——`builtins.rs` 的 print/read-line/newline/write-string/read-int/read-num 直调 `kerf_runtime` 全局函数（io.rs），CapabilityIO 契约**零消费面**，令牌无法流经任何管线节点；「编译期错误（条款 3）」无验证点（typecheck R1-R8 无权限规则）。→ r8 修复：① front 管线权限验证（R9，全路径 run/eval/check/compile）；② 注册 seam 能力参数化（grant → 令牌捕获）；③ 声明面 `(require io read|write)`。修复后能力可门控**任意被声明的 I/O 消费节点**（数据驱动表→Stage 2 net/process 无缝扩展）。 |
| D-边界 | ✅ | 三层分工无歧义：reserved.rs（契约）/ capability.rs（铸造+实现）/ builtins（消费）。⚠️ **发现 F5**（P3）：`BuiltinImpl = Rc<dyn Fn>` 共享捕获 vs 线性令牌——`RefCell` 内部可变性承载（构造不可伪造保持 + `borrow_mut` 互斥 = 借用期线性），记入实现注记。 |
| D-可替换 | ✅（r8 后） | §28 路径「传统 I/O → 能力模型」：替换 seam = `register_globals`（r8 参数化）；全局函数退役轨迹 = kerf 程序面退役（r8）/ OS 边界层保留（StdCapabilityIO 之下）/ Stage 2 语言级令牌值形态。 |

### 2.4 CompilationCache（P2 → r7 已做实）

| 维度 | 评定 | 分析 |
|------|------|------|
| D-强度 | ✅ | 三方法（get/store/invalidate）+ 内容寻址键（SHA-256 截断 + 配置指纹）+ 幂等/显式失效语义逐条测试锁定（cache_tests 13 case）。 |
| D-任意节点 | ✅（源级口径） | 挂接点 = `compile_front_cached`（front 管线单一入口，run/eval/check/compile 四路径共用）；语句级/依赖级失效 = salsa Stage 2（规格条款 4 预告，签名冻结）。 |
| D-边界 | ✅ | `InMemoryCompilationCache` 实现 trait + 管线富入口（lookup_front/store_front）职责分离明确。 |
| D-可替换 | ✅ | trait 在位，salsa 替换不破三方法（渐进替换样板工程——CapabilityIO r8 照此模式做实）。 |

---

## 3. 12 能力模块边界清晰度矩阵

逐模块「职责边界（lang-design 三段式）↔ 代码 seam ↔ 替换证明」对账（✅ = 有活体证据；◇ = 计划中；△ = 契约文档层）：

| # | 能力模块 | 做什么/不做什么锚 | 实现 seam | 替换证明 |
|---|---------|------------------|-----------|---------|
| 1 | 类型化 Token Reader | 02 §6（词法零语义） | `read_source`/`lex_source` 双实现（种子 Rust + 自举 kerf） | ✅ **B3 换活体证明**：driver bootstrap 桥切换生产读路径，parity 87 正 + 307 负 |
| 2 | 图 IR（含共享） | 13 §2.2（不执行不求值） | `kerf_core::lower_program`（front 按需计算，TD-015 分流） | ✅ TD-015 分流守护测试（快路径字节码一致断言） |
| 3 | 结构化 CodeValue | 13 §2.3（不隐式编译） | `CodeValue::from_expr` + `is_well_formed` | △ P3（compose_with 预留位） |
| 4 | 元循环求值器 | 13 §2.4（传统 eval/apply） | `kerf_vm::eval_program`（参考路径） | ✅ **T1 双路径互查**（408 套件 + 审计集双路径断言）；Stage 2 被编译器替换（唯一退役路径，12 §2.5.1） |
| 5 | 基础闭包 | 13 §2.5（纯词法） | VM CLOSURE/编译器闭包捕获 | ✅ closures.krf + 计数器测试 |
| 6 | Span 全管线 | 13 §2.6（不可变随行） | `Span` 字段贯通 Token/Stx/CoreExpr/IR/指令 | ✅ 诊断渲染位置断言（全负向 case） |
| 7 | 结构化诊断 | 13 §2.7（错误是数据） | `Diagnostic` + `render_diagnostic` + E 码族 | ✅ E0005 多错误次序断言 |
| 8 | 最小 I/O | 13 §2.8（传统全局函数） | `kerf_runtime::io` + builtins 注册 | ✅→r8 能力参数化（F2 修复后 seam 升级） |
| 9 | 相位分离 | 13 §2.9（declare/visit/instantiate） | `ModuleRegistry` 三操作 | ✅ 模块生命周期测试 |
| 10 | 基础宏系统 | 13 §2.10（卫生骨架） | `expand_program` + 变换器注册表 | ◇ 批次 E kerf 重写（seam = expand_program API + parity 待建） |
| 11 | 标记-清除 GC | 13 §2.11（最简 mark-sweep） | `Heap`（alloc/mark/sweep + `register_foreign_ref` ✅ 13 §4 FFI 前置约束已落地） | △ Stage 2+ 分代（分配器接口稳定） |
| 12 | 字节码 VM | 13 §2.12（switch-dispatch） | `run_program` + `FrameExt` 三槽（ext1 continuation/effect、ext2 异常表、ext3 调试——**静态休眠**） | ✅ T1 双路径；ext1 激活属 Stage 2（发现 F3 记录） |

**结论**：12 模块边界**全部有 lang-design 三段式锚 + 代码 seam**，其中 8 项有活体替换/等价证明。漂移项零（v5.2-v5.3 回写闭环生效）。

## 4. 五条渐进替换路径可替换性审计（§28）

| 替换路径 | seam（挂点） | 活体证明 | 残余风险 | 触发时机 |
|---------|-------------|---------|---------|---------|
| ① 元循环求值器 → 编译器 | run/eval 双入口共享 `compile_front_cached` | ✅ T1 双路径 408 全绿 | eval 路径 trace 为空（from_vm trace: vec![]）——已知差异，Stage 2 eval 退役时消除 | Stage 2 完全自举 |
| ② 传统 I/O → 能力模型 | `register_globals`（r8 参数化 grant） | r8 交付（E0006 + 令牌流） | 语言级令牌值形态（Stage 2 线性类型） | Stage 2 完整模型 |
| ③ 传统闭包 → Effect Handlers | 冻结契约（r8 内部机械做实） | r8 交付（InternalEffectSystem） | resumption 语义（Continuation 具体化） | Stage 2 语言级 |
| ④ Rust Reader → kerf Reader | driver bootstrap 桥（种子/自举双实现） | ✅ B3 parity 87+307 + 生产路径切换 | 词表演进须双侧同步（r8 require 关键字顺带实证该义务） | 已完成（持续 parity） |
| ⑤ Rust Expander → kerf Expander | `expand_program` API | ◇ 批次 E | parity 工具待建（Reader parity 模式复用） | Stage 1 批次 E |

## 5. 「任意流程节点能力」专项裁定

**总则**：一个机制的 D-任意节点能力 = 它能以**零签名污染**方式挂接到流程的任意深度节点。逐机制裁定：

| 机制 | 任意节点能力 | 挂接点枚举 | 边界（非任意处） |
|------|------------|-----------|----------------|
| 内部效应（r8） | ✅ Rust 调用树任意深度 | 线程局部处理器栈：perform 可在任意嵌套闭包/循环内触发，向上展开至最近处理器 | FFI 边界、unwind-abort 构建配置（文档化约束） |
| 能力门控（r8） | ✅ 编译期任意被声明消费点（数据驱动表） | front 管线验证（E0006）+ 注册 seam 消费（fail-closed 双层） | 语句级动态门控（Stage 2 salsa 依赖图协同） |
| 编译缓存（r7） | ✅ 源级任意入口 | `compile_front_cached` 单一入口四路径共用 | 语句级失效（Stage 2） |
| VM 帧三槽（ext1-3） | ◇ Stage 2 | 帧结构常量格式已冻结（13 §2.12），激活语言级效应/异常表 | 当前静态休眠（F3 边界记录） |
| 阶段模块 seam | ✅ 管线四阶段（Read/Expand/Compile/Run）各自可换实现 | Reader（已证）/Expander（批次 E）/Compiler（稳定）/VM（双路径） | 跨阶段契约 = FrontOutput/DriverError 数据形态（已冻结） |

## 6. 发现清单与分级处置

| ID | 级别 | 发现 | 处置 |
|----|------|------|------|
| **F1** | P2 | Effect 冻结形状 Value 耦合——编译器内部使用需类型化适配 | r8 修复：`Escape<T>` 类型化逃逸 + `InternalEffectSystem` Value 适配器（冻结 trait 双证：Probe + 真实现）；Stage 2 泛型关联细化（reserved.rs 注记） |
| **F2** | P2 | I/O 消费面硬接线——CapabilityIO 零消费面，条款 3（编译期错误）无验证点，D-任意节点失败 | r8 修复：R9/E0006（front 全路径）+ grant 参数化注册 + `(require io read\|write)` 声明面 |
| **F3** | P3 | ext1 槽位休眠无书面裁定——「内部效应不激活帧槽」的边界未记录，存在倒挂误读风险 | r8 记录：一次性逃逸在 Rust 层不需要帧槽；ext1 激活 = Stage 2 语言级（本报告 §5 + 04 文档注记） |
| **F4** | P3 | require 声明面引入需关键词——无先例锚风险 | r8 顺带：走 module/import/export 先例（Keyword 枚举 + reader.krf KEYWORDS 双侧 parity 义务实证） |
| **F5** | P3 | `Rc<dyn Fn>` 共享捕获 vs 线性令牌 | r8 记录：RefCell 借用期互斥 = 线性近似（构造不可伪造严格保持）；Stage 2 令牌值化时随语言线性类型消除 |
| **F6** | 观察 | `register_foreign_ref` 已落地（13 §4 FFI 前置约束） | 无需行动——前置约束合规样例（正面确认） |

## 7. 结论与批次 D 范围裁定

**总体结论**：四项预留契约形状**均足以承载目标语义**（D-强度全过）；12 模块边界清晰度**零漂移**；可替换性 **8/12 有活体证明**。两处 P2 缺陷（F1 Value 耦合、F2 I/O 硬接线）**恰好是批次 D 的做实目标本身**——审查与实现收敛于同一点：**做实即修复**。

**批次 D 范围裁定**（驱动 26-b/26-c）：
1. **D1**（26-b）：`effects.rs` 内部效应系统（处理器栈 + 一次性逃逸 + 冻结契约真实现）+ `kerf test` 子命令（测试短路/错误恢复双场景消费面）——语言面零暴露（12 §2.4.3「编译器内部使用属实现策略」）；
2. **D2**（26-c）：`capability.rs`（pub(crate) 铸造 + StdCapabilityIO）+ `(require io read|write)`（module 先例）+ R9/E0006（front 全路径）+ builtins 能力参数化；
3. **不做**：语言级效应/resumption（Stage 2）、语句级失效（salsa）、令牌值形态（线性类型）、ext1 激活（F3 边界记录）——全部防投机钩子（§21.7）。

*遵循条款：§13.1（批次 D 前设计对齐——本报告即对齐记录）、§6.1（P2/P3 分级）、§11（接口隔离——内部效应不进语言面）、§12（最优>最小——审查与做实收敛）、§2.2 原则 27/28（接口稳定性/渐进替换）、§21.7（不做实不预留投机钩子）。*
