# G2 设计轮：HM 类型推断升级评估（保守 R1-R8 → 推断演进裁定）

> **Author**: kerf-doc-agent（ALG-A 类型系统理论 / ARCH-A 架构联合——Task 38-d）
> **Date**: 2026-09-12
> **Version**: v0.1.0（G2 设计轮唯一交付物——评估 + 设计，不含实现）
> **Status**: Proposed（裁定供批次 G 收口采纳；实现排期建议见 §7）
> **输入**: kerf-compiler/src/typecheck.rs（669 行实况全文）、tests/v0/stage1/plan/typecheck_tests.rs、kerf-driver/src/builtins.rs（BUILTIN_SIGS L1143 起）、kerf-driver/src/driver.rs（check_source L477-488）；[16-参照分析 §2.2](../../lang-design/16-reference-analysis.md)、[06-操作语义](../../lang-design/06-operational-semantics.md)、[01-核心形式](../../lang-design/01-core-forms.md)、[13-能力矩阵（类型检查器行）](../../lang-design/13-capability-matrix.md)、[stage-2/plan §1/§5 批次 G 行](./plan.md)、[TD 登记册](../../tech-debt-register.md)（TD-002/004/011/013/016/017/022）、[stage-1/multi-error-recovery-design](../stage-1/multi-error-recovery-design.md)（同批协同 + 风格基准）
> **上游**: Stage 1 r15（553:0:0 全绿）+ r16 批次 F 深审环（TD-013 改判绑定 G2）

---

## 1. 定位声明

G2 = plan §5 批次 G 行「保守 R1-R8 → HM 升级评估 + 16 参照复核」。本文件是
**评估 + 架构设计**文档：

- **裁定**升级路径（算法 / 值限制 / 泛化时机 / 类型域映射 / 诊断 / 过渡策略）；
- **不裁定**实现细节与排期（PoC 面建议见 §7，落地 MUV 由后续批次规划采纳）；
- 现状盘点以 typecheck.rs **实况代码**为准（逐项带行号锚），不猜测既有行为。

Stage 2 核心目标已含 HM 推断（plan §1 §21.1 / 12-roadmap §2.5 Stage 2 行 /
13-能力矩阵「保守静态检查（R1-R8 确定性规则……Hindley-Milner 完全体
Stage 2+）」）——本文件回答的是**怎么进、以多大步幅进**，不是要不要进。

## 2. R1-R8 现状盘点（typecheck.rs 实况）

### 2.1 架构事实（推断升级的直接约束面）

| # | 事实 | 代码锚 |
|---|------|--------|
| A1 | 类型域 `TcType` 10 变体：Unknown/Int/Float/Num/Bool/Str/Nil/Symbol/Pair/Callable{min,max}——**无类型变量、无函数类型构造**（Callable 只携带元数区间，无参/返回类型） | L53-71 |
| A2 | 合一替代物 = **单态环境** `env: HashMap<Symbol, TcType>`：lambda 参数装订 Unknown（L279-282）、define 顺序插入（L252-260）、遮蔽经 insert/restore 模拟（L565-574） | L230-236 |
| A3 | 分支汇合 = `join`：Int/Float/Num 互 join → Num；具体类型互异 → Unknown（if 两支 L312-314、begin 末值） | L95-108 |
| A4 | 逃逸面三级回退：env 未命中 → builtins Callable → **Unknown**（卫生符号 name$hyg$N / 递归自引用 / 未绑定引用——L548-562 注释口径） | L548-562 |
| A5 | `set!` 返回被赋值的类型，**不改变绑定已有类型** | L332-335 |
| A6 | 深度预算 `MAX_CHECK_DEPTH = 512`：超限子树保守 Unknown、零诊断（宏展开可产生超深结构——A3 展开上限 10_000） | L47 / L269-273 |
| A7 | 诊断码 E0005（L39）；全量收集按 `(file_id, start, end)` 排序——TD-013 多错误收集的首个消费面（multi-error-recovery-design §5） | L220-227 |
| A8 | 内置签名 BUILTIN_SIGS 49 项（kerf-driver/builtins.rs L1143 起）与 register_globals **同文件维护**（防漂移）；结果类型供上层消费（谓词 → Bool 使 R1 命中 if 条件）；check_source 经 front 管线注入（driver.rs L483-488） | builtins.rs / driver.rs |

测试面锚（typecheck_tests.rs）：examples 六件套零诊断 + 动态边界零误报
（参数值/递归自引用/卫生符号/数值塔/内置遮蔽/set!-begin-if 混合）+ R1-R8
负例矩阵（每规则 ≥3 case）+ 双向锚 `static_error_is_runtime_error`
（静态报错程序实跑必报 Run 阶段错——防静态误报与漏报失真）+ E0005 码断言。

### 2.2 八条规则逐条盘点

| 规则 | 保守行为（代码实况） | 拒绝什么 | 接受面（仍放过什么） |
|---|---|---|---|
| R1 | if 条件类型 ∉ {Unknown, Bool} 报错 | 静态已知非 bool 条件（运行时 E1/I3） | 条件 Unknown——参数值路径（`dynamic_programs_clean` case 1：`(if x 1 2)` 零诊断） |
| R2 | 算术族签名 TcParam::Num 逐参检查（check_param_rule） | 操作数静态已知非数值（E5 家族） | Int/Float 混合（Num 并集域——数值塔）；参数 Unknown |
| R3 | Ordering/NumOrAllStr 特例：`=` 族全数值或全字符串、排序族全字符串报 TD-011 边界消息（L452-503） | 非数值比较、混串（TD-016 全操作数口径） | Unknown 操作数；`(< 1 2.0)` 数值塔零误报锚 |
| R4 | not 参数签名 Bool 规则 | 静态已知非 bool（E1 家族） | 参数 Unknown |
| R5 | car/cdr 签名 Pair 规则 | 静态已知非 pair（E5） | quote 字面量 → Pair 命中（`(car '(1 2 3))` 零诊断锚）；参数 Unknown；**元素类型不检查**（car 结果恒 Unknown——builtins.rs L1161-1162） |
| R6 | fn_ty 具体非 Callable 报「不可调用」（L406-414） | 静态已知不可调用（E4 家族） | fn_ty Unknown（一切动态分派）；**递归调用**（自引用回退 Unknown） |
| R7 | Callable 元数区间检查（lambda L388-405 / 内置 L428-440，元数先检与运行时口径一致） | 元数不匹配（E2/E3 家族） | **递归函数的元数不检查**（fib 体内自引用 Unknown）；高阶参数 |
| R8 | Str/Symbol 参数规则（str-append / string->symbol / symbol->string 等） | 串/符号族类型不符（E5） | 参数 Unknown |

**升级价值缺口**（R1-R8 静默放过、HM 能捕获的典型）：
① 用户 lambda 实参类型错（`((lambda (x) (+ x 1)) "foo")`——参数 Unknown，
运行期 E5，静态零诊断）；② car/cdr 元素类型误用（car 结果恒 Unknown）；
③ 分支汇合类型分歧被 join 吞为 Unknown 后的下游误用；④ 递归函数体内的
元数/域错（`(fib #t)` 静默——A4 回退）。HM 的论证面正是这四类。

### 2.3 保守性契约（升级必须显式重定义的对象）

typecheck.rs 头注（L8-12）：**误报 = P1 缺陷**——只报「运行期**必然**以同类
错误失败」的程序；判据 = 全部既有测试与检查器零冲突。HM 推断本质是
**类型一致性强于运行期安全性**的判定——典型：`(lambda (x) (x x))` 自应用
在运行期可存活（传过程即合法求值），HM occurs check 拒绝。升级 = 契约从
「零误报（误报 = P1）」收窄为「类型不一致即报（含运行期可存活的类型不一致）」。
**此政策变更必须显式登记并重锚测试断言语义**（§6 P0-2）——否则 553 套件的
保守性验收口径静默漂移。

## 3. 核心问题逐项裁定

### 3.1 算法选择：约束生成 + 求解三段式

| 候选 | 形态 | 与 kerf 约束的适配 |
|---|---|---|
| Algorithm W | 自顶向下、边遍历边合一、替换传播 | 单错误短路式——中途失败即中止，**与 TD-013 多错误恢复（r16 改判绑定 G2 同批）冲突**；替换链深递归在 2 MiB 栈面紧张；合一失败点 ≠ 源过错点，定位弱 |
| Algorithm J | 破坏性 union-find 合一 | 高效，但错误后合一状态残缺——多错误恢复需快照回滚，复杂度失控；破坏性可变状态与「错误是数据」收集原则摩擦 |
| **约束三段式（裁定）** | ①遍历生成带 Span 的约束集 → ②worklist 求解（union-find + occurs check）→ ③zonk + 诊断收集 | 生成/求解分离：多错误 = 求解失败逐条收集（对齐 TD-013 形式级恢复与 (file_id,start,end) 排序——multi-error-recovery-design §2/§3）；深度预算只作用于生成期（512 语义不变，超限子树零约束零诊断）；求解器迭代式 = 栈安全 + **自举友好**（纯数据结构算法、无深递归——规避 TD-022/TCO 未决依赖，I1 批次推断器本体须 kerf 化）；rustc 同构（推断变量 + 义务约束——16 §2.2 rustc 谱系背书） |

**裁定 D1**：约束生成 + 求解三段式。W/J 否决主因：多错误恢复协同与栈安全
（后者是自举迁移的硬约束，非偏好）。

### 3.2 let 多态与值限制（含 set! 交互与递归绑定）

kerf 绑定语义实况：顶层/模块体 define 保留原语（06 §2 R6 两位置语义）；
函数体内部 define 经展开器改写为 **nil 预绑定 + set!**（letrec 推导——01
§2 糖推导表：`letrec f = e → let f = nil in (set! f e) in body`）；任意
已有绑定可 set!（06 §2 R5——词法链定位，闭包捕获经共享单元格 L3 引理）；
表面 `let` = App-of-Lambda 糖（01 §2）。四个子裁定：

- **D2（值限制：采用）**：泛化仅限**语法值**绑定（lambda 字面量 / 标量与
  点对字面量 / 变量引用）；set! 目标、App 结果、letrec 预绑定 → **弱单态**
  （类型变量不泛化，逐用点共享同一实例）。理由：kerf 可变绑定无处不在
  （set! + 共享捕获单元格）——无值限制的经典 unsoundness 反例（可变单元格
  持有多态类型后异型写入逃逸）在 kerf 全域成立；OCaml 对照见 §4。
- **D3（set! 类型语义：join 而非合一）**：`set! x e` 后 x 的类型 =
  join(旧, 新)——**复用 A3 的 join 格**：数值塔内无损（Int|Float→Num，与
  运行时数值塔提升一致）；具体异型 → Unknown（保守降级）。关键效果：
  **保守契约在赋值面保持零误报**（`(define x 1) (set! x "foo")
  (str-append x "!")` 运行期合法——join 降 Unknown 后静态仍零诊断；若用
  合一则误报，违反 §2.3 原契约）。set! 过的绑定永久弱单态（D2）。
- **D4（递归绑定预置：新裁定）**：两形状统一机制——检查递归绑定 RHS 时
  将绑定名**预置为 fresh 单态变量**（不取运行时初始值的类型）：
  - 顶层 `(define f (lambda …f 自引用…))`：现状自引用回退 Unknown（A4——
    typecheck_tests 动态边界 case 2 实锚）；HM 下预置 α_f，体内自引用与
    α_f 合一——**递归元数/域错从「静默」变「可检出」**（§2.2 缺口 ④ 兑现）；
  - letrec 展开形（`App[Lambda[f, Begin[SetBang[f, rhs], body]], Literal nil]`
    一类）：若按字面取 f 初值类型则 Nil ~ (α→β) 合一失败——**P0 冲突**
    （§5 C2）——约束生成器识别该形状置 α_f（01 §2 推导表为形准；推导表
    v5.2 自注「非穷举——以展开器实现为准」，实现前须以实际产物核对）。
  递归体内 = 单态递归（ML 标准口径）；RHS 为 lambda 字面量（语法值）→
  检查完成后**泛化**（SML letrec-of-lambda 同款）→ `(define (id x) x)`
  顶层 Int/Bool 双用点均合法。**多态递归不做**（ML 同口径——无注解面下
  不可判定）。
- **D3b（值域完整性注记）**：点对字面量内容归约后不可变（06 §2 R4 注），
  是语法值；quote 产物（含 Sym——TD-002 r5）同口径。

### 3.3 occurs check 与无限类型诊断

- **D5（采用，错误级）**：合一 α ~ …α… 时 occurs check 必备；失败诊断
  「无法构造无限类型：α 与 τ」，渲染两端类型片段（Haskell/GHC 诊断形态，
  §4）。Span = 引入该约束的用点（约束生成期携带）。
- 误报面显式承认：自应用类程序运行期可存活——HM 拒绝 = 契约收窄（§2.3）。
  **逃生舱 = A4 的 Unknown 回退路径原样保留**（未绑定引用/卫生符号 →
  Dynamic，不进约束集）——动态风格代码不被推断面强制（推断只作用于
  可约束子图——渐进语义边界）。

### 3.4 泛化/实例化时机

- **D6（双点泛化，序处理）**：泛化点 = ①每个顶层/模块体形式检查完成时
  （define 经值限制过滤后——与 check_form 现序同构 L252-260，M1 累积语义
  对齐）；②let 形状（**App-of-Lambda 模式识别**——表面 let 糖在 CoreExpr
  层无 let 节点，推断器按形状还原；用户手写 `((lambda (x) …) e)` 同形同
  待遇，语义等价故无害）。实例化 = VarRef 逐引用 fresh 化（标准）。
- 模块顶层序（Racket 式顺序 define）+ ML 式 let 泛化的**混合**：与 kerf
  「顶层 define 顺序生效 + 函数体无 define 原语」的展开产物结构一一对应
  ——零注解面维持（不为推断引入新表面语法）。
- Stage 2 若引入 `Let` ANF 原语（01 §7.3/§8.3 演进项——批次 H1 主题），
  泛化载体可从形状识别迁至结构节点——本设计不依赖该迁移（形状识别对
  现有产物即完备）。

### 3.5 TcType 类型域 → HM 类型构造子映射

```text
τ ::= α（类型变量） | Int | Float | Num | Bool | Str | Nil | Sym
    | Pair(τ, τ) | τ → τ | Dynamic
```

| TcType 现状 | HM 映射 | 裁定说明 |
|---|---|---|
| Unknown | **Dynamic（逃逸原子）**：不进约束、不泛化、永不报告 | A4 三级回退原样保留——渐进逃生舱（§3.3） |
| Int / Float | 原子 Int、Float | 运行时 type_name/int? 区分——不折叠为单原子 |
| Num | 原子 Num + **格扩展合一**：Int⊔Float = Num、Int~Num = Num | **非教科书 Robinson 合一——文档化偏差**（继承 A3 join 语义；数值塔并集域是运行时事实）。算术/比较内置约束按「参数 ⊑ Num」求解 |
| Bool / Str / Nil / Symbol | 原子 | Symbol 域按 TD-002 r5；Str 与 Sym 分立（运行时区分） |
| Pair | **Pair(τ₁,τ₂) 构造子**：cons : α→β→Pair(α,β)；car : Pair(α,β)→α；cdr : Pair(α,β)→β | 元素类型推断兑现（§2.2 缺口 ②） |
| Callable{min,max} | 结构化 τ₁…τₙ→τ，元数由构造子携带 | R7 从区间检查变结构约束 |
| （无对应——运行时 unit/closure/builtin） | closure/builtin → Arrow（lookup 二级）；**Unit 无静态面**（现状签名表无 Unit 结果者——维持，避免引入分歧行；P3 注记） | 06 §1.2 十变体与检查域的名实对照 |

BUILTIN_SIGS 49 项 → scheme 重解释：TcParam::Any → α（逐调用实例化）；
Num → ⊑Num 格约束；List → Pair(α,β) ∪ Nil 联合域约束（length/reverse
接受空表——保守保留联合域而非递归 list 类型）；NumOrAllStr/Ordering
特例约束形态保留（TD-011/016 语义对齐）。**签名表数据不动、解释层换**
——A8 同源维护防漂移结构保持。

### 3.6 诊断集成（TD-013 协同）

- **D7**：诊断码维持 E0005 家族（新消息族：类型不一致 / 元数结构 / 无限
  类型）；**约束生成期逐约束携带 Span**（求解失败回溯到引入用点——W/J
  的「失败点 ≠ 过错点」问题消解）；多错误 = 求解失败全量入 DiagCollector
  （上限 128 + 截断提示——multi-error-recovery-design §3 复用）；排序口径
  `(file_id, start, end)` 不变（A7 对齐——check 与 expand 两收集面同序
  渲染）；深度预算 = 生成期 512 语义不变（超限子树零约束零诊断——与
  「未检查」等价，零误报零噪音）。

### 3.7 推断与 typecheck 双轨过渡策略

- **D8（不做运行期双轨回退；一刀切分三阶段演进）**：「推断失败回退
  R1-R8」会产生两套诊断口径——TD-018（双路径消息分裂，P3）是既有教训，
  不再开新双路径面。过渡 = **演进轨道**：
  1. **PoC（离线）**：新推断器与 R1-R8 并行跑测试面；门 = **超集门**
     （R1-R8 全部负例矩阵仍检出）+ **零误报门**（examples 六件套 +
     动态边界 case 全维持零诊断）；
  2. **旗标期**：`kerf check` 切换 HM 判定面；R1-R8 退为回归基线断言；
  3. **默认期**：R1-R8 语义**内化**进约束集——R1 = cond 约束 Bool；R2/R3/
     R4/R8 = 内置 scheme 约束；R5 = Pair(α,β) 约束；R6 = fn_expr 约束
     Arrow；R7 = 元数结构化。规则不被删除而是被结构约束**蕴含**。
- **solver 崩溃防御**（E8 口径——编译器缺陷而非用户错误）：内部异常 →
  回退 R1-R8 报告 + P0 缺陷单（唯一保留的回退路径）。
- 契约重定义随旗标期显式落地（§2.3——保守性断言更新为「零类型不一致
  误报」口径；双向锚 `static_error_is_runtime_error` 范围重锚：超集门内
  维持，occurs 面豁免并文档化为政策行为）。

## 4. 16 参照对照表（复核结论 + 借鉴裁定）

**复核结论（诚实登记）**：16-reference-analysis v5.0 的 HM 相关内容仅一行
——§2.2 OCaml 评定「Hindley-Milner 类型推断捕获大量编译期错误」+ rustc
OCaml→Rust 谱系佐证；**无系统的 rustc/Swift/Koka/Haskell 推断章节**（该
文件主体 = C 深度参照 + 实现语言选择，源自 stage0.md Part IV 拆分）。下表
以六语言公开事实为基（本文件自足），供 G2 裁定消费；若批次 H 需 Koka 行
多态深对照（H3 Effect 设计输入），建议补参照分析专章——非本文件义务。

| 参照 | 推断形态 | kerf 借什么 | kerf 不借什么 |
|---|---|---|---|
| rustc | 局部推断 + 推断变量/义务约束（infcx）+ trait 求解 | 约束三段式架构、推断变量表示、错误回溯定位（D7） | trait 义务求解 / 生命周期 region（kerf 无 trait 无借用面） |
| Swift | 注解趋近（decl-site 双向传播，泛型显式约束） | 「注解锚点」诊断形态（未来若引入可选注解语法的措辞/定位设计参照） | 声明驱动双向推断（kerf 零注解面——推断必须全免费，D6） |
| Koka | 行多态（effect rows）+ 完整推断 | 行类型概念为 H3 Effect 语言级设计预留对照 | 行多态本体（无记录类型；G2 范围外） |
| Haskell (GHC) | 全 HM + 类型类 + 单态限制 | 泛化/实例化证据规则、occurs check 诊断形态（「无限类型」措辞——D5） | 类型类（数值塔用格合一替代——§3.5 Num 行）；单态限制（值限制已覆盖 kerf 需求） |
| OCaml | HM + 值限制（relaxed 形态） | **值限制本体**（D2）+ letrec-of-lambda 特判（D4） | 多态变体 / 对象行系统（无对应语义面） |
| Racket | 动态 + 契约（boundary 检查） | 分层哲学：静态层 + 运行期边界（kerf 现状同构——Dynamic 逃逸 → 运行期 E1-E7 吸收语义，06 §3） | typed/untyped 渐进边界与契约延迟检查（G2 不做渐进类型面） |

## 5. 与核心语义的冲突清单

| # | 冲突 | 等级 | 处置 |
|---|---|---|---|
| C1 | **lambda 的 ScopeSet 元数据**（TD-004 已解：VarRef/SetBang 引用作用域集 + Lambda param_scopes——01 §2 v6.2 注记）：推断环境绑定解析现状 = 纯名 Symbol（A2）——Symbol 已含卫生后缀故现状成立；多模块实例化隔离（Stage 1+ 演进项）或表面改名时须换 (name, scopes) 口径 | P2 | PoC 维持纯名（与 typecheck.rs 同构）；迁移绑定 TD-004 体内复用 |
| C2 | **letrec 的 nil 预绑定编码**（01 §2 推导表 + 06 §2 R6 函数体路径）：HM 字面解释触发 Nil ~ (α→β) 合一失败 → 函数体内递归定义全误报 | **P0** | 形状识别 + fresh 预置（D4）；实现前以展开器实际产物核对形状（推导表非穷录注记） |
| C3 | **顶层递归 define 自引用**：现状 Unknown 回退（A4）——不预置则递归函数推断面为零（fib 无法成为 PoC 验收程序） | **P0** | D4 预置 α_f；行为变更是超集方向（体内错可检出），须负例矩阵重锚 |
| C4 | **宏展开后的类型交互**：展开产物无注解面；宏引入符号走 Dynamic 回退（A4 注释口径）；宏合成代码的诊断 Span 指向展开器回填位——归因失真与 TD-014 同族 | P1 | Dynamic 逃逸保持零强制；Span 归因质量随 TD-014/018 消息批（批次 I2）同轮 |
| C5 | **set! 与类型变量**（含闭包共享捕获单元格 L3——多闭包读写同一变量）：统一式合一破坏保守契约（§3.2 D3 反例）；共享单元格使赋值效果跨闭包可见 | **P0（政策面）** | D3 join 语义——保守契约在赋值面保持；弱单态标记防泛化逃逸 |
| C6 | **quote 产物的 Pair/Symbol 域**：点对字面量递归结构 → Pair(τ,τ) 递归推断（`'(1 2 3)` = Pair(Int, Pair(Int, Pair(Int, Nil)))）；超深 quote 字面量受生成期深度预算约束（超限保守零约束——与 A6 现状同构） | P2 | 维持预算语义；向量值类型（TD-002 开放项）届时扩 Vec(τ) 原子——预留注记非承诺 |
| C7 | **begin 内 define 全局语义**（03c/03d 统一行为——typecheck.rs L316-331 注释）与函数体 set! 改写（06 §2 R6）两路径并存：泛化点在两路径必须同构，否则同一程序局部/顶层行为漂移 | P1 | 两路径均按「形式完成时泛化」（D6 ①）——begin 内 define 与顶层 define 同点（现状 check_form 与 Begin 分支的遍历序已同构，推断器沿用） |

## 6. 迁移风险评估（P0-P3 分级——P0/P1 实现前必须解决）

| 级 | 风险 | 缓解 |
|---|---|---|
| **P0-1** | letrec/递归形状识别错漏 → 递归程序全误报（fib/fact/closures/higher_order examples 直接红——examples_all_clean 六件套回归锚） | D4 双形状特判 + PoC 门含 examples 全零诊断 |
| **P0-2** | 保守契约重定义未显式化 → 「误报 = P1」断言语义漂移，553 套件保守性验收口径失效 | §2.3 契约变更随旗标期显式登记（断言更新 + occurs/自应用误报面文档化为接受行为；双向锚范围重锚） |
| **P0-3** | 数值塔格合一实现错（Int~Float 处置不当）→ 数值塔合法程序误报（R2 数值塔边界锚红） | §3.5 Num 格合一文档化为偏差 + 专项矩阵（Int/Float/Num 全组合 ~ 关系 ≥9 case） |
| **P1-1** | 49 项签名 scheme 重解释遗漏（variadic Any 实例化 / List 联合域 / NumOrAllStr 特例） | 签名表数据不动、逐项重解释 + 每内置 ≥1 正例推断断言 |
| **P1-2** | Dynamic 变量逃逸污染泛化（未绑定引用的 fresh var 被错误捕获进 scheme） | 未绑定引用固定 Dynamic 原子（不进约束集——「不约束、不泛化」二元纪律） |
| **P1-3** | 求解失败后约束图残缺 → 多错误后续失败定位失真 | 形式级恢复复用（TD-013 §2 裁定）：每顶层形式独立约束桶，跨形式仅经全局 scheme 交互——失败形式按粒度隔离 |
| **P2-1** | 生成期递归栈深（超预算宏展开产物） | 512 预算语义原样迁移 + typecheck.rs 深度单测两枚转锚 |
| **P2-2** | 纯名解析口径 vs TD-004 演进（C1） | PoC 纯名；迁移绑定 TD-004 |
| **P2-3** | 超集门维护成本（R1-R8 负例矩阵与 HM 输出对齐） | 旗标期 R1-R8 退为回归基线断言（D8 阶段 2） |
| **P3** | Unit 原子缺位 / 向量类型（TD-002 开放项）/ 效应行（H3 对照）/ 多值扩展 | 预留注记；非 G2 义务 |

## 7. 建议与 GO/NO-GO 裁定

**裁定：GO（有条件）**。HM 升级方向成立（§2.2 四类缺口是实锚 + Stage 2
核心目标既定——plan §1/§21.1）。但**实施排期建议 = 批次 H 新增 MUV（H4
推断 PoC）而非批次 G 内塞实现**：批次 G 已含 G1 QBE PoC + G3 FFI 所有权
定义 + G2 本设计 + TD-013 实现（plan §5 G 行）——容量约束下 PoC 与语义
演进评估轮同批更优（H1 的 Let/ANF 评估与 D6 泛化载体、H3 效应设计与
Koka 行对照互为输入）。**I1 自举迁移（编译器本体 kerf 化）前必须达成
默认期**——类型检查器是 kerf 化本体的一部分（D1 的自举友好选型即为此
预锚）。

**算法**：约束生成 + 求解三段式（D1）。

**最小 PoC 面**（验收 = §9 门 + §6 P0 三项全绿）：
1. 类型域 §3.5 十一构造子 + 格合一（Int/Float/Num）；
2. 遍历面：字面量/VarRef/Lambda/App/If/SetBang/Begin/Define/Module
   （Require → Dynamic——零运行时语义无类型约束，typecheck.rs L348-350
   同口径）；
3. D2 值限制 + D4 双形状递归预置 + D3 set! join + D6 let 形状泛化；
4. BUILTIN_SIGS 49 项 scheme 重解释；
5. **验收程序**：`(define (fib n) (if (< n 2) n (+ (fib (- n 1))
   (fib (- n 2)))))` 与 examples 六件套**零标注零诊断，且 fib 推断为
   Int→Int（非 Dynamic）**；
6. 超集门：R1-R8 负例矩阵（typecheck_tests 每规则 ≥3 case）全检出；
7. 多错误：≥3 错程序全量收集按 Span 次序（TD-013 消费面续接）；
   occurs check 负例 ≥3（含自应用）+ 值限制负例 ≥2（set! 目标不泛化）。

## 8. 决策记录

| # | 决策 | 裁定 | 依据 |
|---|---|---|---|
| D1 | 推断算法 | 约束生成 + 求解三段式 | §3.1：TD-013 同批协同（r16 改判 G2）+ 栈安全/自举友好（TD-022/TCO 未决 + I1 kerf 化）+ 16 §2.2 rustc 谱系。**新裁定** |
| D2 | 值限制 | 采用（语法值才泛化；set!/letrec/App 弱单态） | §3.2：set! 全域可变（typecheck.rs L332-335 + 06 §2 R5/L3）+ OCaml 对照 §4。**新裁定** |
| D3 | set! 类型语义 | join(旧,新) 非合一——保守契约赋值面保零误报 | §3.2：A3 join 格复用（typecheck.rs L95-108）。**新裁定** |
| D4 | 递归绑定预置 | 顶层自引用 + letrec 展开形双特判，fresh 单态预置；lambda-RHS 检查后泛化；多态递归不做 | §3.2/§5 C2-C3：A4 回退实况（typecheck.rs L548-562）+ 01 §2 推导表 + SML/OCaml 口径 §4。**新裁定** |
| D5 | occurs check | 必备、错误级、双端类型渲染 + 约束用点 Span | §3.3。**新裁定** |
| D6 | 泛化/实例化 | 顶层 define 序 + let 形状双点泛化；VarRef 实例化；零注解面维持 | §3.4：check_form 顺序实况（L252-260）+ 01 §2 糖推导 + M1 语义。**新裁定** |
| D7 | 诊断集成 | E0005 族 + 约束携带 Span + DiagCollector 128 上限 + (file_id,start,end) 序 + 生成期 512 预算 | §3.6：multi-error-recovery-design §2/§3 + typecheck.rs L220-227/L47。**新裁定** |
| D8 | 双轨过渡 | 演进轨道三阶段（PoC→旗标→默认）；无运行期回退（solver 崩溃防御除外，E8 口径） | §3.7：TD-018 双路径分裂教训 + TD-013 登记册行（G2 绑定）。**新裁定** |
| D9 | 类型域映射 | 十一构造子 + Dynamic 逃逸原子 + Num 格扩展合一（文档化偏差） | §3.5：TcType L53-71 实况 + BUILTIN_SIGS（builtins.rs L1143 起）。**新裁定** |
| D10 | 参照复核 | 16 参照 HM 覆盖仅一行（§2.2）——对照表以公开事实自足 | §4 复核结论：16-reference-analysis v5.0 实况（诚实登记，不虚构参照章节）。**新裁定** |
| D11 | 实施排期 | GO 有条件——PoC 排批次 H 新增 MUV；默认期须早于 I1 自举迁移 | §7：plan §5 G/H 行容量约束 + §21.3 条件 1。**新裁定** |

## 9. 验收标准（本设计文档的量化验收）

1. **R1-R8 盘点完整**：8 条规则逐条（§2.2 表 8 行全覆盖）+ 架构事实 8 项
   （§2.1）+ 契约分析（§2.3）——每项带 typecheck.rs 行号锚。✅
2. **裁定 ≥7 项**：D1-D11 共 **11 项**（含 5 项 P0 相关裁定）。✅
3. **参照对照表 ≥6 行**：rustc/Swift/Koka/Haskell/OCaml/Racket 恰 6 行
   + 复核结论诚实登记（参照输入缺口不静默带过）。✅
4. **风险分级含缓解**：P0×3 + P1×3 + P2×3 + P3×1，每项附缓解列。✅
5. **GO/NO-GO 明确**：GO（有条件）+ 排期裁定（H4 PoC）+ 最小 PoC 面
   七条 + 默认期时限锚（早于 I1）。✅
6. **交付物唯一性**：仅本文件，零代码修改。✅

---

*遵循条款：§21.5（批次 G 逐批细化——G2 MUV 唯一交付物为设计文档）、
§8.4.5（决策附依据）、§2.3-4（显式失败——参照复核缺口不静默带过）、
§12（最优>最小——算法选型以多错误恢复与自举安全为硬约束而非实现最简）、
TD-013 登记册行（r16 改判 G2 绑定——同批协同设计）、TD-002/004/011/016/
022 关联（§5/§6 处置列）。*
