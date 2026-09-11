# 8 原语迁移五项语义层评估（H1——§13.2 切换期重构评估轮）

> **Author**: Super Z（ARCH-A 主导 + PM-A/DEV-A/QA-A/ALG-C 五角色投票——L3 多角色会话）
> **Date**: 2026-09-11（批次 H r18）
> **Version**: v1.0
> **Status**: Active（裁定落档；五项投票全票通过——详见 §6）
> **输入**: sop.md §13.2（切换期重构）/§13.4（J1-J6 判据）/§6.3（投票）；[01-core-forms §7.3/§8.3](../../lang-design/01-core-forms.md)；[stage0 §6.12.6](../../stage0.md)（收敛裁定 + 语义等价映射表）；[12-roadmap §2.4/§2.5.1](../../lang-design/12-roadmap.md)（演进矩阵 Stage 2 承诺面）；[stage-2/plan.md](./plan.md) H1 节；r17 代码实况（kerf-core expr.rs / kerf-compiler compile.rs / kerf-backend anf.rs）
> **上游**: 批次 G r17（605:0:0 基线 + AnnotatedANF 实化 + HM 设计 GO 有条件）

---

## 0. 评估轮定位声明（§1.2「重构」+「进入新阶段」联合路由）

本文件是 Stage 2 批次 H 的**语义演进评估交付物**——回答「8 原语形态（next2 推荐的
`Fn/Let/Apply/Const/Var/Branch/Perform/Handle`）的五项迁移主题，在当前时点
（r17，批次 G 交付后、批次 I 自举迁移前）各自应当：**现在做 / 绑定条件后做 /
登记 Stage 3 候选 / 拒绝**。评估不是实施——每项裁定均经 §13.4 J1-J6 判据检查 +
§6.3 加权投票落档；**Stage 2 内原语集零变更**是本评估的默认保守基线（核心冻结
原则 9 精确化表述：语义原语集冻结 + 声明变体可追加），任何例外须逐项投票。

**评估时点的三个关键实况**（r17 代码锚，非文档记忆）：

| # | 实况 | 代码锚 | 对评估的含义 |
|---|------|--------|-------------|
| A1 | **词法寻址已实现**：编译器将命名 VarRef 解析为槽索引（`VarSource::Local(u32)/Global/Captured` → `Op::LoadLocal(u32)` 等操作数） | compile.rs L42-51 + opcode.rs L37-51 | de Bruijn 的**运行期收益**（O(1) 变量访问）已既得——剩余议题仅在 IR 形态层 |
| A2 | **块式 ANF 已实化**：kerf-backend anf.rs（ABlock/APhi/LowerCtxt，r17 38-b/c）在后端边界完成 ANF 转换 | anf.rs L66-77 | `Let` 原语的 ANF 锚定点已存在于 **IR 层**，不必然要求 Core 层变更 |
| A3 | **自举 parity 链在飞**：读+展开两段全自举（expander.krf 生产路径）+ 双审计集 91 case + 605 基线；批次 I = 编译器本体 kerf ~80% 迁移 | 07-bootstrap §3.3 + stage-2/plan I1 | mid-Stage-2 引入新核心形式 = 在最坏时机打断 parity 链（expander.krf 重锚 + 双审计 + 全量基线重验） |

## 1. 评估框架（§13.2 × §13.4 联合）

- **时点规则**（§13.2 目的）：切换期重构 = 阶段完全结束（gate 全过）与新阶段开始之间。
  当前处于 Stage 2 **中途**（批次 G ✅ / H 进行中 / I 待执行）——**不是切换期**；
  Stage 2 内唯一合法的语义层变更窗口 = 批次边界 + §6.3 投票 + §13.2.3 全六强制规则。
- **判据规则**（§13.4.1）：每个迁移候选过 J1-J6；任一「否」= 该重构不合规。
- **投票规则**（§6.3）：ARCH-A 2 票（一票否决权）+ DEV-A 1.5 + QA-A 1 + ALG-C 1
  = 5.5 总权；≥95% 通过（实践 = 全员 APPROVED / APPROVED WITH MINOR CONCERNS）。

---

## 2. 逐项评估（五项 × 三段式 + J1-J6 + 裁定候选）

### E1 `Let` 原语（ANF 绑定——Begin → Let 链脱糖）

**现状实锚**：CoreExpr 无 Let（`Begin` 承担顺序 + `Lambda+App` 模拟绑定——
01 §7.3「begin a b → ((lambda (x) b) a」）；ANF 转换在后端边界由 LowerCtxt 完成
（实况 A2）。next2 论证完备性：Begin 可模拟 Let 但**无 ANF 优化能力**。

**三段式评估**：
- **收益**：① Core 层 ANF 化使类型检查与优化锚定在统一 IR（HM 推断在 ANF 上
  更规整——let-多态边界显式）；② 脱糖 Begin→Let 链后求值器无「丢弃中间值」
  语义特判；③ 与 8 原语形态的目标结构对齐。
- **成本**：CoreExpr 新变体 → 展开（含 **expander.krf 自举 parity 重锚**）、
  编译、VM 操作码、typecheck、QBE lowering、双审计集、605 基线**全链重验**；
  估算触及 ≥7 crate / ≥15 测试文件（实况 A3——最坏时机）。
- **风险**：ANF 化 Core 与「编译器本体 kerf ~80% 迁移」（I1）竞争同一批
  工程带宽；mid-flight 语义变更违反 §13.2 时点规则（当前非切换期）。

**J1-J6**：J1 ✅（01 §8.3 映射表对齐）；J2 ✅；J3 ✅；J4 ✅；J5 ✅；
**J6 ⚠️ 粒度合法但时机非法**（§13.2 时点——非切换期）。

**裁定候选**：**DEFER-TO-STAGE3**（条件绑定：I1 自举迁移完成 + 两次编译字节
一致验证通过后的切换期；届时 Let 与 E3/E5 同轮评估一次性迁移）。Stage 2 内
ANF 优化能力已由 IR 层（anf.rs）承载——Core 层 Let 的增益不足以对冲 parity
链重验成本。

### E2 `Perform`/`Handle` 效应原语化（副作用 → 效应）

**现状实锚**：效应在元层做实（driver effects.rs + 能力门控 I/O——13 §3.1.1
预留 → r8 做实 require/R9/E0006/IoGrant）；12-roadmap §2.5.1 Stage 2 行 =
**做实引入（语言级）——Stage 2 承诺项**（§2.4 五能力演进图同口径）。06-
operational-semantics §3/§4 已有 Perform/Handle 归约对照与 continuation 三要素。

**三段式评估**：
- **收益**：语言级效应表达力（异步/错误恢复/自定义控制流统一机制——
  12 §2.2.1「演进空间极高」）；`set! → Perform(State)`、`begin → Let 链`
  展开规则由 next2 论证完备；OCaml 5 实践数据可用（Forester 6.0）。
- **成本**：语义层最大单项变更——VM 栈处理（handler 帧 + resumption）、
  编译器新操作码族、typecheck 效应推断（或保守推迟）、stdlib 适配；
  实现估算远超批次剩余带宽。
- **风险**：与 TD-022 TCO 决策耦合（Perform 捕获栈 = TCO 交互面）；Stage 2
  门（§21.3 四条件）不含效应语言级——**承诺面在 12-roadmap，验收面在
  §21.3，二者口径差须显式调和**。

**J1-J6**：J1 ✅（06 + 13 §3.1.1 规格）；J2 ✅；J3 ✅；J4 ✅；J5 ✅；J6 ✅。

**裁定候选**：**GO（设计先行）**——本批次 H3 交付语言级 Effect 设计文档
（Perform/Handle 原语化对照 + 语义规格 + 迁移路径）；**实现落位由 H3 设计
的迁移路径裁定**（初排批次 I 后段或 Stage 2 末环——不与本评估冲突：设计
承诺即刻兑现，实现按 §21.5 信号走）。E2 是五项中唯一 Stage 2 roadmap
承诺项——「现在做」的部分是**设计**，不是实现。

### E3 de Bruijn 索引（消除变量名）

**现状实锚**：展开层 = 命名符号 + ScopeSet 集合作用域（TD-004 r13 收口：
`(name, scopes ⊆)` 匹配——**卫生机制的载体**）；编译层 = 槽索引词法寻址
（实况 A1——`VarSource::Local(u32)`）；运行期已是 O(1) 槽访问。

**三段式评估**：
- **收益**：① alpha-free IR（替换无需改名——MLton 式闭包转换与优化器
  变换简化）；② 消除 SymbolTable 运行期依赖（编译产物自包含）；
  ③ IR 尺寸缩减（索引 vs 字符串）。
- **成本**：**展开层不可索引化**——scope-set 解析即卫生（r13 收口成果，
  60 测试锚）；若 Core 层改 de Bruijn，expander.krf parity、TD-004 全套
  断言、E0002/E0005 诊断的名称渲染全部重锚。运行期收益（A1）**已既得**。
- **风险**：诊断质量回归（名字消失 → 运行期错误只剩索引——需旁路名称表，
  净复杂度上升）；与 HM 推断的 letrec/泛化锚（38-d 设计 D5/D6）耦合。

**J1-J6**：J1 ✅（stage0 §6.12.6 映射）；J2 ✅；J3 ✅；J4 ⚠️→✅（若仅在
  IR 层）；J5 ✅；J6 ✅。

**裁定候选**：**分层裁定**——① **展开层命名制永久保持**（卫生载体，不可
  牺牲）；② **Core/IR 层 de Bruijn = Stage 3 优化器引入时机的伴生评估**
  （优化器变换才需要 alpha-free；当前两个后端（VM 槽索引 + QBE 局部临时
  名）均不受益）；③ 与 H2 eval 重写评估**信息互通**（eval 深树递归的根治
  方案空间包含「编译期解析前置」——若 H2 裁定 eval 重写走 ANF 输入，则
  E3 的 IR 层形态自然并入）。

### E4 continuation 类型安全（resumption 三要素 + 线性唯一性）

**现状实锚**：无 continuation 原语（VM 调用栈无捕获）；06 §4 continuation
三要素规格已登记；OCaml 5 已知缺陷（不静态确保效应被处理）的规避方案 =
类型安全 Handle（next2 裁定：resumption 类型 + 线性唯一性）。

**三段式评估**：
- **收益**：效应系统 soundness 的类型面保证（「未被处理的效应不可逃逸」
  静态成立）；Stage 3 异步生态的语义地基。
- **成本**：类型级线性性（use-once 消费语义）超出当前 TcType 表达力
  （38-d 十一构造子不含线性性——设计明示「防复制性留类型级线性性
  Stage 3 锚」）；依赖 E2 落地后才有宿主。
- **风险**：提前实现 = 无宿主抽象（违反 §2.3-11 能力边界不清不做）。

**J1-J6**：J1 ✅（06 §4 规格）；J2-J6 ✅（作为**设计规格登记**而非实施）。

**裁定候选**：**SPEC-ANCHOR**——三要素 + 线性唯一性作为**规格约束**进入
H3 设计文档（Effect 语言级设计消费此锚）；**实现 = Stage 3**（依赖 E2 +
类型级线性性）。38-d HM 设计 Stage 3 锚点六项已含线性性——口径一致。

### E5 语义化命名迁移（Lambda→Fn / App→Apply / If→Branch / …）

**现状实锚**：01 §7.2 命名精确性对照表 + §8.3 十二行迁移映射；§8.2 裁定
「命名在冻结期内不变」+ 三原则合规（当前架构形态全绿）；表面/内部语法严格
分离（§8.4——CoreExpr 变体名是编译器私有，用户不可见）。

**三段式评估**：
- **收益**：与设计文档的 2026 形态对齐（阅读一致性）。
- **成本**：纯机械 churn——10 crate + 605 测试锚全量重命名，**零语义增益**
  （§8.4：用户可见面是 S 表达式皮肤，变体名不影响任何外部行为）。
- **风险**：**名实不符**——8 形态命名的前提是结构就位（`Fn(params: usize,
  de Bruijn)` ≠ `Lambda(params: Vec<Symbol>, param_scopes)`；`Perform/Handle`
  不存在时把 `SetBang` 改名 `Perform(State)` = 谎言命名）。违反 §2.3-1
  「显式 > 隐式」精神（名字描述不存在的行为）。

**J1-J6**：J1 ✅（§8.3 映射）；J2-J5 ✅；J6 ⚠️（纯 churn 无职责变化——
  「粒度由职责决定」反例形态）。

**裁定候选**：**REJECT-STANDALONE**（拒绝独立重命名）——命名迁移绑定
E1-E4 全部落地后的 **Stage 3 切换期一次性迁移**（§8.3 映射表为路线图；
届时结构变更与命名变更同轮，名实同步）。§7.2 裁定维持：核心 ADT 命名
冻结期内不变。

---

## 3. §6.3 委员会逐项投票（五角色，2026-09-11，r18 批次 H）

> 投票口径：每项一票（APPROVED / APPROVED WITH MINOR CONCERNS / NEEDS
> REVISION）；加权 ≥95% = 通过（5.5 权全票）。NEEDS REVISION 触发二次内循环。

| 项 | ARCH-A（2） | DEV-A（1.5） | QA-A（1） | ALG-C（1） | 加权结果 |
|---|---|---|---|---|---|
| E1 Let | APPROVED（DEFER-TO-STAGE3——parity 链保护为一票否决级理由） | APPROVED（IR 层 ANF 已承载——Core 层增益/成本比 <1） | APPROVED（605 基线 + 双审计零重验义务） | APPROVED（语义等价映射不失效——形式糖等价） | **5.5/5.5 ✅ DEFER** |
| E2 Perform/Handle | APPROVED（GO 设计先行——roadmap 承诺 + H3 本批次交付设计） | APPROVED WITH MINOR CONCERNS（实现落位须在 H3 设计后回锚 I 段带宽） | APPROVED（设计轮无测试面影响） | APPROVED（06 语义锚完备——三要素规格进设计） | **5.5/5.5 ✅ GO-DESIGN** |
| E3 de Bruijn | APPROVED（分层裁定——展开层命名制永久保持写入 01 §7.3 注记） | APPROVED（A1 实况——运行期收益已既得，剩余纯 IR 层议题） | APPROVED（诊断名称渲染质量不回归） | APPROVED（卫生 = scope-set 载体，r13 收口成果保护） | **5.5/5.5 ✅ 分层** |
| E4 continuation | APPROVED（SPEC-ANCHOR——规格进 H3，实现 Stage 3） | APPROVED（无实现义务——纯登记） | APPROVED（无测试面） | APPROVED（线性唯一性 = OCaml5 缺陷规避正解） | **5.5/5.5 ✅ SPEC** |
| E5 语义化命名 | APPROVED（REJECT-STANDALONE——名实不符风险 + 零语义增益） | APPROVED（churn 成本全在 DEV 侧——10 crate 机械重命名） | APPROVED（605 锚点重写零回报） | APPROVED（名字描述不存在行为 = 语义谎言） | **5.5/5.5 ✅ REJECT** |

**投票结论**：五项全票通过（5.5/5.5 × 5）；零 NEEDS REVISION——无需二次
内循环。DEV-A 对 E2 的 MINOR CONCERNS 为 P3 级（实现落位回锚义务登记于
H3 设计文档的回写义务节）。

## 4. 裁定汇总与绑定落位

| 项 | 裁定 | 落位 | 绑定条件 |
|---|---|---|---|
| E1 Let | DEFER-TO-STAGE3 | Stage 3 切换期候选 | I1 自举迁移 + 两次编译字节一致后 |
| E2 Perform/Handle | GO-DESIGN（本批次） | **H3 设计文档（40-d）**；实现 = 批次 I 后段或 Stage 2 末环（H3 迁移路径裁定） | H3 设计交付；TD-022 TCO 裁定耦合声明 |
| E3 de Bruijn | 分层（展开层命名永久 / IR 层 Stage 3） | 展开层注记即刻回写 01 §7.3；IR 层 = Stage 3 优化器伴生评估 | H2 eval 重写信息互通 |
| E4 continuation | SPEC-ANCHOR | H3 设计消费（三要素 + 线性唯一性约束节） | E2 实现落地 |
| E5 语义化命名 | REJECT-STANDALONE | Stage 3 切换期一次性迁移（§8.3 路线图） | E1-E4 全落地 |

**总裁定**：**Stage 2 内原语集零变更**（核心冻结原则 9 维持）；唯一语义层
推进 = E2 的**设计承诺即刻兑现**（H3）——与 12-roadmap §2.5.1 Stage 2
「做实引入（语言级）」的口径调和：**设计做实在 Stage 2（本批次），实现
窗口由 H3 迁移路径显式裁定并回写 roadmap**。8 原语形态整体迁移 = Stage 3
切换期重构候选（§13.2 流程登记——届时 J1-J6 重走）。

## 5. 回写义务清单（本评估触发）

| # | 文档 | 回写内容 | 落位轮 |
|---|---|---|---|
| W1 | 01-core-forms §7.3 | 批次 H 评估裁定注记（五项裁定 + 展开层命名制永久保持） | 40-f 收尾 |
| W2 | 12-roadmap §2.5.1 | Effect 语言级「设计做实 Stage 2 / 实现窗口 H3 裁定」口径注记 | 40-f 收尾（H3 交付后） |
| W3 | stage-2/plan.md | H1 行执行注记（本文件 + 投票结果） | 40-f 收尾 |
| W4 | tech-debt-register | 无新 TD（五项裁定均不产生实现债务——DEFER/REJECT 项登记为 Stage 3 候选而非债务） | — |

## 6. 量化验收对账

- 五项评估 × 三段式（收益/成本/风险）= 15 段全落 ✅
- J1-J6 判据 × 5 项 = 30 检查点全落（E1 J6 ⚠️ / E5 J6 ⚠️ 如实标注并计
  入裁定理由）✅
- §6.3 投票 5 项 × 4 角色 = 20 票全记录（加权 5.5/5.5 × 5）✅
- 代码实况锚 3 项（A1 词法寻址 / A2 块式 ANF / A3 parity 链在飞）✅
- Stage 2 原语集变更数 = **0**（保守基线维持）✅

---

*遵循条款：§13.2（切换期重构——时点规则 + §13.2.3 强制规则）、§13.4.1（J1-J6 六大判据逐项检查）、§6.3（五角色加权投票 + 二次内循环触发规则——本次零触发）、§2.2 原则 9（核心冻结精确化）、§2.3-1（显式 > 隐式——E5 名实相符裁定）、§2.3-11（能力边界不清不做——E4 提前实现否决）、§8.4.5（决策附条款号 + 代码实况锚）、§12（最优 > 最小——E1 IR 层已承载即当前最优解）、§21.5（切换信号口径——§21.3 验收面与 roadmap 承诺面的调和显式化）。*
