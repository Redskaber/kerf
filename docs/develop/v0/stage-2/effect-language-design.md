# Effect 语言级引入设计（H3——Perform/Handle 原语化对照）

> **Author**: Super Z（ARCH-A 主导 + ALG-C 语义审查——L3 多角色会话）
> **Date**: 2026-09-11（批次 H r18）
> **Version**: v1.0
> **Status**: Active（设计冻结——实现窗口裁定见 D12；H1 E2 GO-DESIGN 兑现）
> **输入**: sop.md §21.5/§13.2；[01-core-forms §7.3](../../lang-design/01-core-forms.md)（next2 效应映射论证）；[06-operational-semantics §1-§3](../../lang-design/06-operational-semantics.md)（归约规则体系与错误吸收）；[13-capability-matrix §3.1.1](../../lang-design/13-capability-matrix.md)（P3 冻结契约 + r8 编译器内部做实注记）；[stage0 §6.4](../../stage0.md)（OCaml 5 方案四完整论证）；[primitive-migration-evaluation.md](./primitive-migration-evaluation.md)（40-b E2/E4 裁定）；[ffi-ownership-model.md](./ffi-ownership-model.md)（线性令牌先例 + 诊断码族先例）；r18 代码实况（effects.rs / vm.rs ext1 / TCO 40-c）
> **上游**: 批次 G r17（605 基线）+ 批次 H r18（TCO 落地——帧语义与效应栈交互面就绪）

---

## 0. 设计轮定位声明（§1.2「写文档」路由）

本文件是 Stage 2 效应语言级引入的**设计冻结文档**——回答「kerf 的
`perform` / `handle` 表面语法、核心归约、continuation 语义、VM 实现
形态、诊断面」的完整规格。依据 40-b E2 裁定（GO-DESIGN）：**设计做实
于 Stage 2（本批次），实现窗口由本文 D12 裁定并回写 roadmap**。
语言级效应是 Stage 2 唯一批准的原语集变更（§8.3 映射表的
Perform/Handle 两行）——其余映射行维持 Stage 3 候选（E1/E3/E5 裁定）。

## 1. 现状基线（代码实锚——非文档记忆）

| # | 实况 | 代码锚 | 对设计的含义 |
|---|------|--------|-------------|
| A1 | **元层效应已做实（一次性逃逸）**：`handle_escape<R,T>` / `perform_escape<T>`（类型化载荷 + 线程局部深度计数 + catch_unwind/resume_unwind + panic hook 过滤） | effects.rs L74/L98 | 「任意流程节点能力」的机械实现已验证；**one-shot 浅处理**语义与目标语言面同构——VM 化是将其从宿主 Rust 栈迁移到 VM 帧栈 |
| A2 | **冻结契约层**：`EffectFamily/Effect/EffectSystem` 三 trait（reserved.rs 冻结签名 + `InternalEffectSystem` 真实现 + Probe 契约双证） | 13 §3.1.1 + effects.rs L145 | 语言面实现须**复用该契约形状**（P3→P0 升级路径——非推翻重设计） |
| A3 | **VM 帧三槽已预留**：`FrameExt { ext1: Option<ContinuationSlot>, ext2, ext3 }`（Stage 0 全空） | vm.rs L56-63 | `ext1` = continuation/handler 槽位——**设计既定落点，结构零变更** |
| A4 | **效应消费面在线**：`kerf test` 用例短路 + 错误恢复（11-测试 §4） | test_runner | 语言面落地后该消费面**零迁移**（元层路径与语言面并存——D7） |
| A5 | **TCO 已落地（r18/40-c）**：尾调用帧复用（TailCall 拆帧承返回地址） | vm.rs Op::TailCall | 效应栈与尾调用的交互面必须显式裁定（D8）——**设计前置依赖已就绪** |
| A6 | **能力门控 I/O 在线**：`require io read/write` + IoGrant 令牌（r8） | driver R9/E0006 | 效应系统与能力模型的职责分界须显式裁定（D10） |

## 2. 目标语义（表面语法 + 归约规则）

### 2.1 表面语法（S 表达式皮肤）

```scheme
;; 效应执行（核心原语）
(perform ⟨effect-value⟩)                ; 挂起当前计算，效应值向上传递

;; 效应处理（核心原语——浅处理）
(handle ⟨effect-tag⟩ ⟨handler-clauses⟩ ⟨body⟩)
;;   effect-tag    : 符号（效应族标签——match 分派键）
;;   handler-clauses: ((⟨payload-var⟩ ⟨resume-var⟩) ⟨result-expr⟩) 单子句
;;                    （单效应族处理——D3 线性唯一性的语法承载）
;;   body          : 被保护计算（效应触发时挂起于此帧）

;; 恢复（continuation 使用——唯一一次）
(resume ⟨resume-var⟩ ⟨value⟩)
```

**两形式进 CoreExpr**：`Perform { effect, span }` / `Handle { tag, payload_var,
resume_var, handler_body, body, span }`——原语集 9(+Require) → **11(+Require)**（§8.3
映射表的（新增）Perform/Handle 两行兑现；`resume` 不是原语——是 continuation
值的调用形态（D4））。

### 2.2 归约规则（R10/R11——06 体系扩展）

```text
(R10-perform)   ⟨(perform v), ρ, σ⟩ → ⤴ v                （效应上抛：控制转移，
                                                          不是值归约——⤴ 记号）
(R10-resume)    ⟨(resume κ v), ρ, σ⟩ → ⟨e_κ, ρ_κ[x ↦ v], σ⟩
                （κ = 挂起 continuation：捕获帧链 + 挂起点环境）
(R11-handle)    ⟨(handle tag H B), ρ, σ⟩
                  → ⟨B, ρ, σ ⊕ handler帧[tag ↦ (H, ρ)]⟩   （安装：帧栈压入）
(R11-dispatch)  挂起点 perform v 且 v.tag = 帧.tag
                  → handler 体求值（payload/resume 绑定注入）
                  ——非匹配 tag 沿帧栈继续上抛（逃逸到顶层 = E0007）
(R11-return)    B 归约到值 v 且无挂起 → handle 表达式值 = v（帧弹出）
```

**求值顺序契约维持**（06 §1.3 A1）：`perform` 的效应值先求值（App 序
不变）；`handle` 体按 Begin 序。**T1 定理扩展**：双路径（VM 帧 /
eval 逃逸映射）对效应可观测行为（恢复次数/顺序/逃逸深度）一致——
D7 给出 eval 侧映射。

### 2.3 continuation 三要素与线性唯一性（E4 规格锚兑现）

`resume-var` 绑定的 continuation 值携带三要素：

| 要素 | 形状 | 语义 |
|------|------|------|
| ① 捕获帧链 | VM 帧栈快照（挂起点 → handler 帧边界） | 恢复时重建执行位置 |
| ② 挂起点环境 | 局部槽/捕获单元格（共享 Rc） | 恢复后变量可见性与挂起时一致 |
| ③ 唯一性状态 | `Fresh \| Resumed`（帧级标记） | **线性唯一**：第二次 resume = E0008（诊断，非 UB） |

线性唯一性 = OCaml 5 已知缺陷（不静态确保效应被处理 + 多次恢复
continuation 的语义裂缝）的显式规避（next2 裁定 + 40-b E4 SPEC-ANCHOR
兑现）。**类型级线性性（编译期防复制）留 Stage 3**（与 38-d HM 设计
的线性性锚同位）——运行时唯一性标记 = Stage 2 的动态防线（D3）。

## 3. 设计裁定（D1-D12——每项附依据）

- **D1 表面语法 = perform/handle 二形式（非 effect 类型层）**：
  依据 §8.4 表面/内部分离——S 表达式皮肤先落两核心形式；效应行
  （effect rows）/行多态属类型系统层（Stage 3，Koka 路线——16-
  reference §2.2 对照）；效应族标签 = 符号（match 单键分派——与
  syntax-rules 字面量集合同型）。
- **D2 浅处理（shallow）先行**：handler 处理**一层**效应——恢复后的
  再 perform 不回同一 handler（OCaml 5 Shallow 同型；深度处理 = 嵌套
  handle 表达——用户侧组合）。依据 stage0 §6.4.5 Deep/Shallow 对照
  + A1 元层 one-shot 同构——深处理需多 shots continuation（D11 否决）。
- **D3 continuation 线性唯一（动态防线）**：`Fresh → Resumed` 帧级
  单次消费；违者 E0008。类型级线性性 Stage 3（38-d 锚同位）。依据
  E4 规格 + ffi-ownership-model 线性令牌先例（一次性消费语义在
  kerf 已有成熟裁定形态——`CallExternal` 令牌消费同构）。
- **D4 resume 非独立原语**：continuation 是值（`Value::Continuation`），
  调用形态走 `Call` 通道——`resume` 表面关键字脱糖为 `(κ v)` 调用
  （App 复用）。依据 §2.2 原则 4（核心极小——复用调用机制）+ Racket
  continuation 调用惯例。
- **D5 `set! → Perform(State)` 映射 = 语义等价证明，非实现义务**：
  next2 论证完备性登记（01 §7.3）；Stage 2 set! 保留原语（E1/H1
  裁定——原语集变更最小面）。依据 40-b E1 DEFER 裁定。
- **D6 VM 实现 = ext1 槽位激活 + 帧栈效应表**：`FrameExt.ext1:
  Option<ContinuationSlot>` 具体化为 `Option<Rc<HandlerFrame>>`
  （HandlerFrame = { tag, payload_var, resume_var, handler_body 求值
  闭包环境, consumed: Cell<bool> }）；perform = 从帧栈顶向下扫描
  匹配 tag 的最近 handler 帧 → 拆帧到该边界 → handler 体在**该帧
  位置**求值（挂起点帧链存入 continuation）。依据 A3 预留槽位
  （结构零变更——原则 27 接口预留时机兑现）+ 06 §1.2 运行时域扩展
  最小面。
- **D7 eval 参考路径 = 逃逸映射（Rust 栈 → continuation 语义）**：
  eval 侧 handle 用 `handle_escape` 承载（payload = 效应值，resume
  经 `Box<dyn FnOnce>` 包装挂起点续体）；**双路径一致性域 = 浅处理
  单次恢复程序**（多次恢复/深处理 = eval 域外——T1 注记扩展，同
  TD-017 256 深度域口径）。依据 A1（元层已验证）+ A4（消费面零迁移）。
- **D8 TCO 交互 = 尾调用穿透 handler 帧（帧数语义不变）**：`TailCall`
  的帧替换在 handler 帧之上（内层）发生时照常拆帧——效应帧边界不变
  （尾调用 = 返回语义，返回路径不携带效应——只有 perform 上抛穿越
  handler 帧）。**裁定：TCO 与浅处理正交**（深度处理才需要帧复制
  ——D11 否决连带解除冲突）。依据 40-c TCO 语义（帧替换承返回地址
  ——效应上抛是独立通道）+ 06 §3 错误吸收同构（错误上抛同样穿透）。
- **D9 诊断码族 = E0007-E0009（运行时效应面）**：E0007 = 未处理
  效应逃逸到顶层（「效应 tag 未被任何 handler 处理」+ tag 名 + 挂起
  点 Span）；E0008 = continuation 二次恢复（线性唯一性违反——含首
  次恢复位置追踪）；E0009 = resume 于非 continuation 值（类型面）。
  **码位登记**：FFI 实现族（ffi-ownership-model E9/E10/E11 暂名）落
  位 E0010-E0012（避免运行时效应族冲突——回写义务 W3）。依据 §2.3
  + E0004 运行时错误先例（VmError + Span + 调用点追踪链复用）。
- **D10 效应系统与能力模型 I/O 正交并存**：能力门控（require io/
  IoGrant）= **通道权限**（可否引用 I/O 内置——编译期 fail-closed）；
  效应系统 = **控制流抽象**（挂起/恢复语义——运行时）。语言级 I/O
  效应化（`(perform (Write s))` 替代 print）= Stage 3 评估项（两者
  语义可叠加：效应 I/O 在能力门内）。依据 A6 + 13 §3.1.3（能力
  模型与效应分属两行）——**不合并**（职责分离，§11 接口隔离）。
- **D11 多次恢复（multi-shot）不实现**：帧复制语义（深复制 VM 帧链
  + 局部槽）成本与 GC 交互复杂度不成比例（Stage 2 门不含）；浅处理
  + 单次恢复覆盖错误恢复/生成器/短路全部 Stage 2 消费面。依据 r8
  D1 裁定维持（13 §3.1.1 注记）+ D2。
- **D12 实现窗口 = 批次 I 后段（I2 之后、I3 门审之前）**：理由——
  ① I1 编译器 kerf ~80% 迁移是更大的语义变更（先做大后做小——切换
  期窗口复用）；② HM PoC（H4）先行给出效应行的类型面基线；③ TD-008
  分代 GC 评估（I2）与效应帧的生命周期管理同轮（挂起帧链的根集
  扫描——GC 五来源扩展为六：+ 活跃 continuation 帧）。**回写义务
  W2**：12-roadmap §2.5.1 Stage 2 行「做实引入（语言级）」注记
  「设计冻结 r18 / 实现批次 I 后段（D12）」。依据 §21.5 切换信号
  （I1 完成后 = 阶段内稳定窗口）+ 40-b E2 裁定的口径调和义务。

## 4. 迁移路径（实现阶段拆分——批次 I 后段）

| 阶段 | 内容 | 验收 |
|------|------|------|
| M1 | CoreExpr + Reader + Expander（两形式展开 + 核心形式 11 面断言更新） | 展开锚 + parity（种子/自举双路径——expander.krf 效应形式两行） |
| M2 | 编译器（Perform → `Op::Perform(tag)`；Handle → 帧安装指令族；resume → Call 通道闭包化）+ VM（ext1 具体化 + 帧扫描 + continuation 值） | 端到端 fib 级 + 嵌套 handle + 逃逸 E0007 |
| M3 | eval 路径映射（handle_escape 承载）+ T1 域测试（浅处理单次恢复一致） | 双路径一致 ≥8 case |
| M4 | 诊断三码（E0007/8/9）+ 线性唯一性 + 调用点追踪 | 负例 ≥6 case（正负比 1:3） |
| M5 | GC 六来源根集（活跃 continuation 帧）+ gc_stress 效应变体 | 跨 GC 存活验证 |

## 5. 测试锚点计划（§9.4.3 正负比 ≥1:3）

- 正例：单 handler 单恢复 / 嵌套 handle（内层非匹配 tag 逃逸到外层）/
  handle 内纯计算零效应 / 恢复后闭包环境一致性 / effect 值先求值序
  / TCO 尾调用穿透 handler 帧（帧数断言）。
- 负例：未处理逃逸（E0007）/ 二次恢复（E0008）/ 非值 resume（E0009）/
  handler 子句形态错（展开期 E0002 族）/ perform 非效应值位置 /
  恢复值跨 GC 逃逸（M5）。

## 6. 风险与缓解

| 风险 | 等级 | 缓解 |
|------|------|------|
| VM 帧扫描 per-perform O(帧深) | P2 | 浅处理 = 最近 handler 帧索引缓存（帧栈侧栏——与 TD-023 根集遍历同型优化路径） |
| continuation 帧链 GC 根集扩展引入扫描回归 | P2 | M5 专项 + TD-023 重定型基准（非尾形）同轮测量 |
| 自举 parity（expander.krf 效应形式两行） | P2 | M1 双路径 parity 锚先行——E1-β 宏 parity 框架直接复用 |
| eval/VM 效应可观测分歧 | P3 | T1 域显式收窄（浅处理单次恢复——D7）+ 域外程序双路径测试回避 |
| 与 HM 推断的效应行交互 | P3 | 类型面 Stage 3（效应行/行多态——16 §2.2）；H4 PoC 不受影响 |

## 7. 回写义务清单（实现落地时触发——批次 I 后段）

| # | 文档 | 内容 |
|---|---|---|
| W1 | 01-core-forms §2/§8.3 | 原语集 9→11 注记（Perform/Handle 两行「已引入」状态翻转） |
| W2 | 12-roadmap §2.5.1 | Stage 2 Effect 行「设计冻结 r18 / 实现 D12 批次 I 后段」注记（**本批次即回写**——40-f 执行） |
| W3 | 诊断码位登记（09-stdlib §2 或 18-terminology） | E0007-E0009 效应族 + FFI 族 E0010-E0012 预留 |
| W4 | 06-operational-semantics | R10/R11 归约规则节 + 运行时域 extension（continuation 值） |
| W5 | 04-bytecode-vm | Op::Perform 指令族 + ext1 具体化注记 |

## 8. 量化验收对账（本设计轮）

- 代码实况锚 6 项（A1-A6）✅；设计裁定 12 项（每项附条款/实锚依据）✅
- 归约规则 6 条（R10 × 2 + R11 × 4）✅；E4 三要素兑现表 ✅
- 诊断码族 3 + 码位冲突预防登记 1 ✅；迁移路径 5 阶段 + 测试锚点
  正 6 负 6（≥1:3）✅；风险 5 项全附缓解 ✅
- **实现量 = 0（设计轮交付边界——D12 裁定实现窗口）**；回写义务
  5 处（W2 本批次执行）✅

---

*遵循条款：§21.5（切换信号——D12 窗口裁定）、§13.2（切换期重构流程——语言级效应是 40-b E2 批准的批次内语义变更）、§8.4.5（决策附条款号 + 代码实锚 6 项）、§11（接口隔离——D6 ext1 激活 / D10 能力-效应正交）、原则 27（接口预留时机——ext1 兑现）、§2.3-2（显式失败——E0007-E0009 诊断而非 UB）、§2.3-4（缺口显式——T1 域收窄注记）、§9.4.3（正负比 1:3 计划）、GATE 3（全部裁定附依据）。*
