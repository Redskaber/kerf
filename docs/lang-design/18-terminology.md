# 术语表与术语源流考

> **Author**: kerf-doc-agent
> **Date**: 2026-09-15（**v6.7：r42 / 63-b 移除轮 S1——E0021 旧名移除错误 + W1003 宏名遮蔽警告码位落位**（20 §7 移除轮行「旧名引用 = E00xx 错误」兑现 + 22 §11 D11 排期兑现——W1001 弃用族随之退役：W 面收窄至 W1002/W1003，23 §3.4 生命周期四阶段完整走完[引入 v0.5 → 默认 v0.6 → 弃用 v0.7 → 移除 v0.9]）；2026-09-14（**v6.4：r34 / 55-a 设计缺陷深度审计收敛轮——§5c 增补术语 13 条**（三轴坐标系/双轴归属/授权组合闭包/编译期权威三判据/诊断终止域/横切泛函域/演进六窗/时间治理四红线/变更通道矩阵/受控债务四步/生命周期四阶段/十二审计轴/判据先于先例）+ **§6 批次 M 码位预登记**（E0013-E0019 预留段——F9 前置兑现）；2026-09-13（**v6.3：r33 / 54-a 能力架构深度设计轮——§5b 增补术语 14 条**（「能力」三义/能力四要素/L0-L3 四层/正交判据 J1-J4/授权三态/六类迁移性质/新原语准入两判据/五层命名层级 N0-N4/解析优先序/遮蔽许可表/import 不传播授权/kerf 保留域/原则 34）+ §5a 对齐术语精确化（v1.1 口径）；v6.2：r32——§6 增补术语 6 条（is- 前缀谓词/方向词转换/命名空间限定名/别名层/能力-命名空间对齐/表面债）；v6.0：next3 讨论六术语增补；v5.5：next2 讨论八术语增补；v5.0：源自 stage0.md v5.0 拆分）
> **Version**: v6.7（**r42 / 63-b 移除轮 S1——E0021 + W1003 码位落位登记**：§6 增移除名错误行 + W1001 退役注记（W 面收窄至 W1002/W1003）；v6.6：r41 E0020；v6.5：r40）
> **Status**: Active

> 本文件收录 stage0.md 附录 A（关键术语表，42 条术语——与源文档逐条一致）与附录 D（术语源流考详细版：五大核心能力的命名起源、理论根基与历史演化路径）。附录 D 是对正文 [14-替代设计 §2（术语起源与历史脉络）](./14-design-alternatives.md) 的深化补充；参考文献与外部链接见 [19-参考文献](./19-references.md)。术语表中括注的文档引用已改写为指向本目录对应设计文件。

---

## 1. 关键术语表（原附录 A）

| 术语 | 定义 |
|------|------|
| 自举（Bootstrap） | 从外部种子到自描述核心的渐进过程 |
| 自托管（Self-hosting） | 编译器能编译自身源代码的状态 |
| 元循环求值器（Metacircular Evaluator） | 用被解释语言本身编写的解释器 |
| 语法对象（Syntax Object） | 携带源位置、作用域集的不可变数据结构 |
| 核心形式（Core Forms） | 语言语义的最小正交集（9 个原语） |
| 相位分离（Phase Separation） | 编译时和运行时代码的物理隔离 |
| 卫生宏（Hygienic Macro） | 保证宏展开不引入意外绑定的宏系统 |
| 同结果测试（Rounds-testing） | 两次编译的输出必须字节一致的验证 |
| DDC（Diverse Double Compilation） | 多样化双重编译，对抗 Trusting Trust 攻击 |
| Span | 源码位置的不可变表示 |
| 查询系统（Query System） | 将编译建模为纯函数查询图的架构 |
| 信任信任攻击（Trusting Trust Attack） | Ken Thompson 揭示的编译器后门注入 |
| hex0 | Guix 全源自举的 357 字节种子 |
| 操作语义（Operational Semantics） | 用归约规则形式化定义程序行为 |
| 标记-清除（Mark-Sweep） | 最简单的垃圾回收算法 |
| bump-pointer | 线性分配器，指针只增不减 |
| 同像性（Homoiconicity） | 程序的 AST 可用语言本身的数据结构表示 |
| 引号（Quote） | 阻止求值、将代码作为数据引用的机制 |
| 闭包（Closure） | 函数与其捕获环境的组合 |
| Effect Handlers | OCaml 5 引入的代数效应处理机制 |
| 多阶段编程（Multi-stage Programming） | MetaOCaml 风格的编译期代码生成 |
| 能力模型（Capability Model） | 通过不可复制的 token 控制权限的模型 |
| 线性类型（Linear Type） | 每个值只能被使用一次的类型 |
| 结构化代码值（Structured Code Value） | 携带类型/Span/Scope 的代码值 |
| 接口预留（Interface Reserved） | 仅定义类型签名不实现，为未来留位 |
| Token 流（Token Stream） | 类型化、携带 Span/Scope 的词法单元序列（[02-语法模型](./02-syntax-model.md)） |
| 图 IR（Graph IR） | 以节点与边表示程序的结构化中间表示，支持公共子表达式共享（[13-能力矩阵](./13-capability-matrix.md)） |
| Arena 分配 | 预分配大块内存、以索引引用对象的分配方式（[13-能力矩阵](./13-capability-matrix.md)） |
| 跳转回填（Backpatching） | 先生成占位跳转、确定目标地址后再修补的编译技术（[04-字节码 VM](./04-bytecode-vm.md)） |
| 操作码（Opcode） | VM 指令的操作编码，Stage 0 约 35 个（[04-字节码 VM](./04-bytecode-vm.md)） |
| 递归下降（Recursive Descent） | 自顶向下、每个非终结符对应一个函数的解析技术（[02-语法模型](./02-syntax-model.md)） |
| 自举链（Bootstrap Chain） | 从种子到完整工具链的逐级构建序列（[00-总览](./00-overview.md)） |
| 同结果性（Reproducibility） | 相同输入必然产出字节一致输出的构建属性（[07-自举策略](./07-bootstrap-strategy.md)） |
| 元追踪 JIT（Meta-tracing JIT） | 通过追踪解释器热路径生成机器码的 JIT 技术（[08-后端演化](./08-backend-evolution.md)） |
| 保守 GC（Conservative GC） | 不精确识别指针、宁泄漏不误回收的 GC 策略（[05-运行时](./05-runtime.md)） |
| 安全点（Safepoint） | VM 指令边界上允许触发 GC 的位置（[05-运行时](./05-runtime.md)） |
| 诊断（Diagnostic） | 结构化错误数据：严重性/码/消息/Span/建议（[02-语法模型](./02-syntax-model.md)） |
| 处理程度（Processing Depth） | P0 完整实现 → P4 完全推迟的五级标度，度量能力在给定阶段的实现深度 |
| 成熟度分级（Maturity Tier） | 生产就绪 / 早期实践 / 研究前沿三档生态评估，引入时机的第一判据 |
| 演进矩阵（Evolution Matrix） | 能力 × 阶段的处理程度总表，回答"何时引入、引入到什么程度" |
| 三级成熟度匹配原则 | 处理程度必须匹配生态成熟度的引入决策规则（[17-设计原则](./17-principles.md) 原则 26） |
| 渐进替换原则（Progressive Replacement） | 实现可从简单演进到复杂而接口契约保持不变的演进规则（[17-设计原则](./17-principles.md) 原则 28） |

---

## 2. 术语源流考（详细版）（原附录 D）

> **本附录详细考据五大核心能力的命名起源、理论根基与历史演化路径，作为对正文 [14-替代设计 §2](./14-design-alternatives.md) 的深化补充。** 术语并非中立标签——它们携带了发明时的范式偏向与时代局限，理解术语的源流有助于在使用时保持批判性自觉。

### 2.1 元循环求值器（Metacircular Evaluator）（原 D.1）

#### 2.1.1 命名起源与论文（原 D.1.1）

术语由 John McCarthy 在 1960 年的开创性论文中隐式引入：

> McCarthy, J. (1960). *Recursive Functions of Symbolic Expressions and Their Computation by Machine, Part I*. Communications of the ACM, 3(4), 184-195.

这篇论文是 LISP 的原始论文，首次描述了用 LISP 自身实现的 `eval` 函数。该函数后来被 Sussman 和 Steele 在 Scheme Manual（1975）中称为 "metacircular interpreter"。

#### 2.1.2 词源学分解（原 D.1.2）

- **Meta-**（希腊语 μετα-）：表示"超越、关于、之后"。在数学和计算机科学中常用于表示"对自身的描述"——metadata（关于数据的数据）、metaprogramming（编写编写程序的程序）
- **Circular**（拉丁语 circularis）：圆形的、循环的。指求值器与被求值语言之间的循环依赖关系

**组合含义**：求值器在"元层"（meta-level）操作"对象层"（object-level）的代码，而这两层使用相同的结构，形成循环。

#### 2.1.3 历史演化时间线（原 D.1.3）

- **1960**：McCarthy 在 LISP 论文中描述 `eval` 函数
- **1975**：Sussman & Steele 在 Scheme Manual 中正式使用 "metacircular" 术语
- **1985**：SICP（Abelson & Sussman）用元循环求值器作为教学核心
- **2007**：Racket 的 `racket/kernel` 将元循环核心工程化
- **2020+**：MetaOCaml 将元循环思想升级为类型安全的多阶段编程

#### 2.1.4 术语的局限（原 D.1.4）

"元循环求值器"这一术语暗示了"循环求值"的必要性，但实际上：
- **循环性不是必要的**：多阶段编程可以打破循环（编译期生成代码而非运行时解释）
- **求值器不是必要的**：编译器也可以描述语言语义
- **更准确的现代术语**：**类型安全的多阶段计算**（Type-safe Multi-stage Computation）

### 2.2 S 表达式的诞生与 M 表达式的遗弃（原 D.2）

#### 2.2.1 McCarthy 的原始设计（原 D.2.1）

McCarthy 在 1960 年的论文中提出了两种语法：

- **S 表达式（Symbolic Expressions）**：`(A B C)` 形式的嵌套列表，用于"数据表示"
- **M 表达式（Meta Expressions）**：`[A; B; C]` 形式，类似传统数学符号，用于"程序编写"

McCarthy 原本计划让程序员用 M 表达式编程，S 表达式仅作为"中间表示"（IR）——程序员写 M 表达式，编译器将其转为 S 表达式供机器处理。

#### 2.2.2 历史转折（原 D.2.2）

但实际发展出现了意外：
- **M 表达式从未被实现**：McCarthy 团队从未为其编写解析器
- **S 表达式被直接使用**：由于 S 表达式已被 LISP 解释器支持，程序员发现直接用 S 表达式编程更方便
- **历史偶然**：S 表达式成为 Lisp 的"语法"是一个**意外**，而非深思熟虑的设计选择

McCarthy 在后来的访谈中承认："S 表达式之所以胜出，仅仅是因为我们从未实现 M 表达式的解析器。"

#### 2.2.3 对本设计文档的启示（原 D.2.3）

S 表达式作为同像性的"标准实现"是一个历史偶然：
- 它不是 McCarthy 的"理想设计"
- 它之所以流行，是因为它"已经存在"
- 这意味着**任何对 S 表达式的"必然性"辩护都是历史错觉**

2026 年的设计应该超越这一历史包袱，认真考虑替代方案（如 Token 流 + 图 IR），而非默认接受 S 表达式。

### 2.3 同像性（Homoiconicity）（原 D.3）

#### 2.3.1 Mooers 与 TRAC 语言（原 D.3.1）

术语最早出现在 Calvin Mooers 开发的 TRAC 语言的语境中：

> Mooers, C. N. (1966). *TRAC, a procedure-describing language for the reactive typewriter*.

Mooers 的设计目标："TRAC 的输入脚本（用户键入的内容）应该与指导内部动作的文本相同"——这是一种程序与数据统一的追求。

#### 2.3.2 词源学（原 D.3.2）

- **Homo-**（希腊语 ὁμός）：相同的
- **Icon**（希腊语 εἰκών）：图像、表示、形式

**字面含义**："相同的形式"——程序的"图像"与数据的"图像"是同一个。

#### 2.3.3 术语的误解史（原 D.3.3）

同像性是一个**被广泛误解**的术语：
- **常见误解**："代码看起来像数据"（基于表面语法）
- **正确含义**："程序的 AST 可以用语言本身的数据结构表示"（基于内部结构）

这种误解的根源在于：Lisp 程序员看到 `(A B C)` 既可以是代码（函数调用）也可以是数据（列表），就误以为同像性是"表面语法的统一"。但实际上，真正的关键是"AST 与数据结构的统一"——Rust 的 TokenStream 也是同像的（按此定义），尽管 Rust 的语法看起来不像"数据"。

#### 2.3.4 2026 年的重新审视（原 D.3.4）

2026 年的同像性研究承认：
- 同像性是**一个谱系**（spectrum），而非二元属性
- Lisp 并不处于"最同像"的位置——它只是最著名的同像性实现
- 现代语言（Rust proc-macro、MetaOCaml、PEG）提供了不同的同像性实现，各有优劣

### 2.4 引号（Quote）与 Lambda 演算（原 D.4）

#### 2.4.1 Church 的 Lambda 演算（原 D.4.1）

引号操作符可追溯至 Alonzo Church 的 Lambda 演算：

> Church, A. (1936). *An unsolvable problem of elementary number theory*. American Journal of Mathematics, 58(2), 345-363.

Church 使用希腊字母 λ（lambda）作为"绑定运算符"——它将变量名与表达式绑定。在 λ 演算中，所有表达式都会被求值（β-归约）。

#### 2.4.2 引号的语义功能（原 D.4.2）

但要操作"代码本身"（如元编程、自指），需要一种方式"引用"代码而**不求值**它——这就是引号的语义功能：
- 在 Lisp 中：`(quote (A B C))` 或简写 `'(A B C)`
- 在 MetaOCaml 中：`.<expr>.`
- 在 Rust 中：`quote! { ... }`

**引号的本质**：阻止求值，将代码作为数据引用。

#### 2.4.3 Q-引用与准引用（原 D.4.3）

Quine（1940）在《Mathematical Logic》中引入了"准引用"（quasiquotation）——允许在引号内部嵌入求值的表达式。这一概念后来被 Lisp 的 `` ` `` 语法（backquote）和 Rust 的 `#var` 语法采用。

### 2.5 闭包（Closure）与 SECD 机器（原 D.5）

#### 2.5.1 Landin 的贡献（原 D.5.1）

术语由 Peter Landin 在 1964 年定义：

> Landin, P. J. (1964). *The mechanical evaluation of expressions*. Computer Journal, 6(4), 308-320.

Landin 的 SECD 机器中，闭包被定义为"包含环境部分和控制部分"的结构。SECD 是 Lisp 编译器的第一个抽象机器模型。

#### 2.5.2 数学闭包的借用（原 D.5.2）

数学中的"闭包"（closure）指：
- 一个集合 S 在某个操作 op 下"闭合"——即对 S 中任意元素 a, b，`op(a, b)` 仍在 S 内
- 例：自然数集在加法下闭合（1+1=2 仍是自然数），但在减法下不闭合（1-2=-1 不是自然数）

Landin 借用这个概念：**函数加上其捕获的环境，形成了一个"闭合"的计算单元**——它携带了求值所需的全部信息，可以独立传递和调用。

#### 2.5.3 历史演化（原 D.5.3）

- **1964**：Landin 在 SECD 机器中形式化闭包
- **1970s**：Scheme 将闭包作为一等公民
- **1975**：Sussman & Steele 提出"闭包即对象"（continuation pattern）
- **1980s**：ML 系列语言引入类型化闭包
- **2020+**：OCaml 5 的 Effect Handlers 提供了"可挂起闭包"——闭包的演化形态

### 2.6 最小 I/O 副作用通道（原 D.6）

#### 2.6.1 不是"发明"，而是"发现"（原 D.6.1）

最小 I/O 副作用通道没有特定的"发明者"——它是自举理论的**自然产物**：
- **图灵机**：每个图灵机需要一个"读写头"作为 I/O 通道
- **λ 演算**：纯粹的计算是封闭的，但任何实际实现需要"输入"和"输出"
- **Guix hex0**：357 字节的种子必须能"读"和"写"机器码

#### 2.6.2 从汇编到能力模型（原 D.6.2）

最小 I/O 的实现方式随技术演进而升级：
- **机器码时代**：直接读写内存地址
- **汇编时代**：syscall 指令
- **C 时代**：`printf` / `scanf` 等标准库函数
- **Haskell 时代**：IO Monad（将副作用封装为类型）
- **2026 年**：能力模型 + 线性类型（编译期验证 I/O 权限）

#### 2.6.3 命名的"非命名性"（原 D.6.3）

这个术语的"非命名性"反映了它的根本性——它是自举的"边界条件"，不需要被发明，只需要被识别。

## 3. next2 讨论增补术语（v5.5 吸收）

| 术语 | 定义 | 出处语境 |
|------|------|---------|
| ANF（A-范式） | 将所有中间计算命名的 IR 形式（`Let` 绑定链）——简化优化与代码生成的现代编译器标准 | next2 批判审查（缺 Let 即缺 ANF） |
| CPS（延续传递风格） | 以显式 continuation 参数传递控制流的编程风格——可消除 `If` 但效率极低（每分支建闭包） | next2 方案 B（不推荐） |
| de Bruijn 索引 | 以词法位置（整数）替代变量名的绑定表示——"允许替换而不做 lambda 转换" | next2 方案 A（Stage 2 评估主题） |
| 行多态（Row Polymorphism） | 按结构（非名义）泛化的多态性——效应追踪与可扩展记录的类型基础 | next2 前沿全景（效应安全基础） |
| 效应安全（Effect Safety） | 类型系统静态确保所有效应都被处理（OCaml 5 缺失；Koka/Eff/Unison 具备） | next2 continuation 类型批判 |
| 闭包转换（Closure Conversion） | 将 `Fn` 显式化为携带环境的 `Closure` 的编译 pass——MLton 性能路径关键组件 | next2 修正四（MLton 风格） |
| comptime（编译期求值） | 用语言自身编写编译期元编程逻辑（Zig 模式）——多阶段编程的务实前身 | next2 前沿全景（生产就绪档） |
| continuation（延续） | 被挂起计算的恢复点（env + stack + resumption 类型三要素）——线性唯一性由类型系统保证 | next2 最终修正（类型安全续体） |

## 4. next3 讨论增补术语（v6.0 吸收——next3.md 第七轮「内部语法的根本性重构」）

| 术语 | 定义 |
|------|------|
| 派生关键词（Derived Keyword） | Racket `#%` 前缀式编译器内部形式（用户不可 shadow 的逃生舱）——`#%` 前缀是语法与 AST 同构（同像性）语境的历史妥协，本设计否决的旧方案（[01 §8](./01-core-forms.md)） |
| 表面语法（Surface Syntax） | 用户可见、可替换的语法层（皮肤）：S 表达式/中缀/DSL 经 Reader 桥接到核心形式（[01 §8.4](./01-core-forms.md)、[12 §2.4.1](./12-roadmap.md)） |
| 内部语法（Internal Syntax） | 编译器私有、不变的 AST 表示（骨架）：类型安全 ADT，用户不可构造（[01 §8](./01-core-forms.md)） |
| 命名行为导向原则 | 命名精确描述行为而非语法历史（`Branch`/`Apply` 而非 `if`/`App`）——原则 29（[17 §1.1](./17-principles.md)） |
| 类型安全优于命名安全原则 | AST 安全性由类型系统（ADT 私有构造子）保证而非命名约定（`#%` 前缀）——原则 30（[17 §1.1](./17-principles.md)） |
| 表面-内部语法严格分离原则 | 皮肤可替换、骨架不变，任何表面语法编译产物为相同核心形式——原则 31（[17 §1.1](./17-principles.md)） |

---

## 5. next4 讨论增补术语（v6.1 吸收——第八轮「2026 接口预留完整性审查」）

| 术语 | 定义 |
|------|------|
| 接口预留完整性审查 | 对预留层覆盖度的系统评估：既有六类（效应/能力/多阶段/缓存/类型/IR）覆盖 ~70%，补齐工具链生态 30% 缺口（[13 §3.3](./13-capability-matrix.md)） |
| 完全推迟（精确语义） | 尚未承诺采用的能力（保留选择权）；区别于接口预留的「已承诺采用、实现推迟」（期票语义，[13 §3.2](./13-capability-matrix.md) 边界调和） |
| LanguageService | 编译器作为语言服务器的查询接口 trait：语法树/补全/文档符号 + 定义/引用/类型 + 诊断/快速修复 + 重命名（P0，[13 §3.3.2](./13-capability-matrix.md)） |
| IncrementalAst | AST 增量更新接口：apply_edit / invalidate_range / reuse_unchanged——LSP 按需重析与增量编译的 AST 侧挂点（P0） |
| DebugInfoGenerator / DebugTraceable | 调试信息生成接口：IR 节点 ↔ 源码位置映射、变量位置查询、DWARF/源映射生成；IR 节点反向链接 AST + 调试名（P0，[13 §3.3.3](./13-capability-matrix.md)） |
| ExternalType / FfiBoundary | FFI 边界类型表示（CInt/CPointer/CStruct/CFunction/Opaque）与 GC 隔离协议（pin/unpin——外部引用不可回收）（P1，[13 §3.3.4](./13-capability-matrix.md)） |
| QuerySystem / Query | 查询式增量编译接口：纯函数查询 + 依赖完整声明 + 按需失效传播（salsa 风格，与编译缓存构成数据面/架构面双预留）（P0，[13 §3.3.5](./13-capability-matrix.md)） |
| CompilerService | 编译器即服务接口：提交/状态/结果/流式诊断/取消——可序列化状态 + 可中断恢复（P1，[13 §3.3.6](./13-capability-matrix.md)） |
| CodegenBackend / WasmBackend | 多目标后端 trait：supported_targets + compile + 后端特有优化；WASM 组件模型目标（P1，[13 §3.3.7](./13-capability-matrix.md)） |
| PackageManager / ExternalModule | 包管理接口：依赖图解析 + 产物获取 + 版本锁定；模块系统的包边界（P2，[13 §3.3.8](./13-capability-matrix.md)） |
| AiAssistant | AI 辅助语义 API：语义摘要/签名查询/快速类型检查/重构建议/文档注释生成——复用 LanguageService 查询基建（P2，[13 §3.3.9](./13-capability-matrix.md)） |
| P0-P3 预留优先级 | 按「不预留的破坏性代价」排序：P0 = 必须在 Stage 0 数据结构中预留位置（LSP/调试/增量）；P1 = 强烈建议 Stage 0 预留 trait（FFI/后端/服务化）；P2 = Stage 1 预留（包管理/AI）；P3 = Stage 3+ 可加（分布式效应）（[13 §3.5.1](./13-capability-matrix.md)） |
| 预留留白原则 | 原则 32：接口预留的本质是「为未来留出空间」而非「提前实现」——要求数据结构与类型定义的兼容性，而非功能的完整性（[17 §1](./17-principles.md)） |

## 5a. r32 表面现代化增补术语（v6.2——[20-表面规范](./20-surface-conventions.md) 载体）

| 术语 | 定义 |
|------|------|
| is- 前缀谓词 | 状态/类型谓词的现代命名形（`is-nil`/`is-pair`——2026 跨范式共识：Swift `is`/Kotlin `isX`/Rust `is_`）；取代 Lisp 家族 `?` 后缀（R2） |
| 方向词转换 | 转换函数的 `to`/`from` 命名（`string/to-symbol` + `symbol/from-string` 双向双家——Rust `to_`/`from_` 方向语义学）；取代 `->` 中缀（R4） |
| 命名空间限定名 | `模块/本地名` 形态的引用（`string/append`——`/` 为标识符内部分隔符，独立 `/` 维持除法；Clojure 同型词法裁定）；取代扁平前缀补丁（`str-`） |
| 别名层 | 零破坏迁移第一段：现代名与旧名双注册（行为 parity 逐字节锚定——别名层零语义变更不变量）；v0.5 批次 L |
| 能力-命名空间对齐 | 命名空间族边界与授权族边界重合（不变量 A）+ 机制分工两门（可见性门 import / 许可门 require——v1.1 精确化：import 不传播授权，[21 §4.3](./21-capability-architecture.md)） |
| 表面债 | 用户可见表面（命名/行为契约/组织）沿用历史家族形态累积的迁移成本——随库生长线性累积（原则 33 的量化对象） |
| 表面现代化动态演进原则 | 原则 33：每个引入新表面的设计时点对照当期前沿审视而非默认继承历史家族形态；标点后缀约定 = 前类型系统时代的补偿机制（[17 §1](./17-principles.md)） |

## 5b. r33 能力架构增补术语（v6.3——[21-能力架构](./21-capability-architecture.md) + [22-命名空间设计](./22-namespace-design.md) 载体）

| 术语 | 定义 |
|------|------|
| 工程能力 / 语言能力 / 授权（三义） | 「能力」三义定锚：工程能力 = 编译器构建技术（[13](./13-capability-matrix.md) 三层分类 owner）；语言能力 = 运行时能力域（[21 §2](./21-capability-architecture.md) L0-L3 owner）；授权 = 安全学术语 ocap（require 门控/FFI 令牌——[21 §4](./21-capability-architecture.md)）——禁互换 |
| 能力四要素 | 定位（解决什么问题）/边界（不解决什么——负空间）/职责（拥有的决策与不变量）/正交（与邻层的接口契约）——[21 §2.2](./21-capability-architecture.md) 每层一张卡；库能力域同构（[21 §3.1](./21-capability-architecture.md) 八域表） |
| 语言能力四层（L0-L3） | L0 值域（数据是什么）/L1 控制（计算怎么组合）/L2 效应（与外界怎么交互）/L3 授权（交互被谁允许）——[21 §2](./21-capability-architecture.md)；判层规则两问（唯一归属禁止跨层登记） |
| 正交判据（J1-J4） | J1 值-控交换律 / J2 效应-语义独立 / J3 许可-语义独立 / J4 效应-授权对偶——正交的操作化定义：任一层语义变更不引发另一层语义变更；各配既有实测锚（[21 §2.3](./21-capability-architecture.md)） |
| 授权三态 | 纯度态（函数级——静态纪律）/声明态（程序级——require 门控）/令牌态（值级——FFI External 线性持有）——粒度递进收紧（[21 §4.1](./21-capability-architecture.md)） |
| 六类迁移性质（M-R/D/I/L/E/A） | 原语级结构重构分类学：重命名/脱糖/索引化/层级迁移/效应化/新增——「原语级迁移 ≠ 重命名」（[21 §5.1](./21-capability-architecture.md)） |
| 不可归约性 / 单层归属（新原语准入两判据） | 提案必须附四要素卡 + ①不能由既有原语组合等价表达 ②J 判据过（跨层按受控债务登记——set! 先例）（[21 §5.6](./21-capability-architecture.md)） |
| 五层命名层级（N0-N4） | N0 符号宇宙（存在性）/N1 全局注册层（默认可见面）/N2 模块层（组织与授予）/N3 局部绑定层（作用域）/N4 保留字层（语法标记）——[22 §2](./22-namespace-design.md) |
| 解析优先序（R-N1） | 非限定名：N3 内向外 → N2 import 注入面 → N1 全局 → 未绑定错误；限定名：仅查该模块 export 面，不回落（R-N3）——[22 §3](./22-namespace-design.md) |
| 遮蔽许可表（R-N2） | N3 遮蔽 N1 合法；N3 遮蔽 N2 合法 + W 警告；同层 define 重复 = 错误；N4 不可遮蔽——[22 §3.2](./22-namespace-design.md) |
| import 不传播授权（红线 1） | import 授予名字可见性，require 授予操作许可——双门分立；import kerf-io 后未 require 调 print 仍 E0006（[22 §5.2](./22-namespace-design.md)） |
| kerf/ 保留域 | `kerf/` 前缀 = 标准库专属（用户模块不得使用——最小保留原则，Clojure `clojure.*` 同型）（[22 §7](./22-namespace-design.md)） |
| 能力正交与授权分层原则 | 原则 34（v6.4）：L0-L3 四层正交 + J1-J4 可审计判据 + 授权三态 fail-closed + 原语准入两判据（[17 §1](./17-principles.md)） |


## 5c. r34 演进治理增补术语（v6.4——[21-能力架构 v1.1](./21-capability-architecture.md) + [22-命名空间设计 v1.1](./22-namespace-design.md) + [23-演进治理](./23-evolution-governance.md) 载体）

| 术语 | 定义 |
|------|------|
| 三轴坐标系（L×N×P） | 语义轴 L0-L3 × 命名轴 N0-N4 × 相位轴 P0/P1（+ 实现轴 crate——工程轴非语义轴）；「L3′」记号已退役（命名机制升格独立轴非 L3 旁支）；判层三问 + L×N 交互矩阵（[21 §2.5](./21-capability-architecture.md)——owner） |
| 双轴归属 | 同一单元在两个轴上的独立登记（`=`：语义属关系域[链式值相等]，名形属运算符族[R6 符号冻结]——主从声明，非跨层违规）（[21 §2.5](./21-capability-architecture.md)） |
| 授权组合闭包（传递闭包显式化） | 程序头 require = 导入链全部模块授权需求的传递闭包显式声明；模块 require 声明 = 需求元数据非授权获得（编译期核对 `授权面(程序) ⊇ ⋃ 授权需求(导入闭包)`——WASI 无环境权威同构）（[21 §4.4](./21-capability-architecture.md)） |
| 编译期权威三判据 | 确定性（可复现构建）/零外部 I/O 或编译期令牌/展开可终止（预算制）——Phase 1 零授权面裁定的提案门（syntax-parse 类）（[21 §4.5](./21-capability-architecture.md)） |
| 诊断终止域 | `error`/`assert-eq?` 的域归属：终止性诊断发起（Err 吸收态 E5 δ 族）——**终止 ≠ 可拦截**（与 Perform 可拦截通道分立是设计裁定：终止不是副作用，不入门控表）（[21 §3.1](./21-capability-architecture.md) 十一域表第 10 卡） |
| 横切泛函域 | prelude 自有五件（map/filter/foldl/foldr/for-each）的域归属：任意可迭代域的高阶变换服务层——不持数据结构职责（[21 §3.1](./21-capability-architecture.md) 十一域表第 11 卡） |
| 演进六窗（K/L/M/E5/T3+） | 演进总时间线的窗口划分：批次 K 终批 → v0.5 批次 L 别名层 → v0.6 批次 M 命名空间层 → Stage 3 E5 一次性窗 → Stage 3+ 触发式族；每窗入口信号全满足才开窗（时间治理四红线）（[23 §2](./23-evolution-governance.md)——owner） |
| 时间治理四红线 | 禁止时间驱动切换/禁止处理程度倒挂/禁止无信号开窗/禁止窗口合并（改名与改行为永不混步）（[23 §2.3](./23-evolution-governance.md)） |
| 变更通道矩阵 | 每类变更对象的唯一通道与前置判据对账表（语义原语→E5 窗/关键字→N4 唯一通道/库表面名→L 别名渐进/效应族→J2 增族窗/授权族→J3J4/值类型→J1/码位→先占位后落位/模块→四表联动）（[23 §3.1](./23-evolution-governance.md)） |
| 受控债务通道（四步） | 跨层冲突的唯一合法处置：①登记（TD/裁定文档）②语义等价证明义务③清偿窗口声明④每轮深审复核——D5 set! 先例的流程化（[23 §3.3](./23-evolution-governance.md)） |
| 生命周期四阶段 | 引入（双注册/接口预留）→ 默认（新语料新规范）→ 弃用（W 警告）→ 移除（E 错误 + 负例组）——引入与移除永不同窗同面；弃用期最小一个稳定版本（[23 §3.4](./23-evolution-governance.md)） |
| 十二审计轴（A1-A12） | 设计缺陷审计的轴系：用户六轴（定位/权限/能力边界/职责边界/层级管理模型/演进时机）+ 发散六轴（组合语义/覆盖完备性/一致性/诊断对账/判据先于先例/元编程工具链交互）——轴系开放（增轴 = 准入流程增行）（[23 §1.1](./23-evolution-governance.md)） |
| 判据先于先例 | 原则 35（v6.5）：先例是佐证不是权威；向后锚定（停 1970）与向前锚定（抄某家族）等价失败；引证纪律 = 先判据后先例 + 跨家族取样 + 否决记录保留（[23 §4](./23-evolution-governance.md)） |

## 6. 诊断码位登记表（r25/42-f W3 回写——结构化阶段码全族总账）

| 码位 | 族 | 引入 | 定义锚 |
|------|----|------|--------|
| E0001-E0004 | 阶段码（词法/语法/展开/运行时类型） | Stage 0 | kerf-span/diagnostic.rs（渲染 `error[E000N]`）；运行时类型族经 driver `from_vm` 映射 |
| E0005 | 静态类型检查（保守 R1-R8） | r7 批次 C | typecheck.rs |
| E0006 | 能力权限（R9 fail-closed） | r8 批次 D | capability.rs（`IO_PERMISSION_CODE`） |
| **E0007** | **效应未处理逃逸**（未匹配 tag 上抛到顶层） | **r25/42-f** | effect-language-design D9；messages.rs `err_effect_unhandled`（单源——VM `Perform` 扫描无匹配） |
| **E0008** | **continuation 二次恢复**（线性唯一性违反——含首次恢复位置追踪） | **r25/42-f** | 同上 D3/D9；`err_continuation_resumed_twice`（VM `Call`/`TailCall` 的 Continuation 臂） |
| **E0009** | **resume 元数面**（continuation 调用恰一实参；非 continuation 值被调用的类型面归 E0004 通用族——effect-language-design v1.1 执行注记口径） | **r25/42-f** | 同上 D4；`err_resume_arity` |
| **E0010** | **FFI 令牌失效后使用**（双释/用后传递/用后使用——消费全局生效后的吸收态） | **r30/48-d 落位** | ffi-ownership-model §5；messages.rs `err_ffi_token_invalid`（单源——VM free_external/call_external 令牌校验） |
| **E0011** | **FFI 所有权/类型违规**（释放 Opaque 令牌/非令牌值/实参形状不匹配/零尺寸纵深防御） | **r30/48-d 落位** | 同上 §2/§6-case4；`err_ffi_ownership`（VM 编组与 free 判定序） |
| **E0012** | **FFI 符号解析失败**（extern 符号表未登记——fail-closed；QBE AOT 链接期解析在 VM 路径的调用期对应物） | **r30/48-d 落位** | 同上 §4；`err_ffi_symbol_resolution`（VM CallExternal 符号解析 + 空表默认入口） |
| **E0020** | **保留字绑定禁令（r41 / 62-a 语言形式深审轮——D10 裁定）**：N4 关键字 25 名全域禁作绑定名（define/set!/lambda 参数/let 系/define-syntax 宏名/module 名/import as 别名/handle 双绑定器）——22 §2.1 N4 不变量「不是值、不可引用、不可遮蔽」的编译期执行；Stx 源码面 + CoreExpr 展开产物面双层；syntax-rules 模板数据域跳过；quote 位数据符号豁免 | **已落地（r41——Compile 族，driver front 管线验证**：`verify_reserved_bindings_stx`[expand 前拦截] + `verify_reserved_bindings_core`[展开产物防御纵深]；12 case 锚定 namespace_tests d10 组；837:0:0 净 +12 | [22 §11 D10](./22-namespace-design.md) 深审轮二裁定 + [22 §2.1](./22-namespace-design.md) N4 行三件套执行注记 |
| **E0021** | **已移除旧名引用（r42 / 63-b 移除轮 S1——20 §7 移除轮行兑现；r43 / E5 S2 扩展——22 §12 D29）**：v0.9 起旧名 27 件退役（`REMOVED_BUILTIN_NAMES` 单源 27 对[旧名→现代名]）；**v0.10 起关键字旧形 3 件退役（`lambda`/`set!`/`begin`——S2 腿切换 fn/assign/do 后旧形入同一拒绝面[D29]）**；旧名引用（值位/操作位全域）→ 编译期拒绝，诊断携现代名指引（比裸 E0004 未绑定 actionable——迁移期 DX 最优形态）；遮蔽/接管豁免与 E0014 同口径（N3 局部绑定胜出 + 用户接管合法——退役的是内置注册面/关键字面非符号宇宙层） | **已落地（r42——Compile 族，driver `verify_qualified_refs` 与 E0014 同 traversal**：[E0021] 渲染前缀 + 现代名指引消息；stdlib_tests removed_rejects_* 27 名全覆盖负例组 + namespace_tests s1 组[恢复健康 + 位置面] + 闭合守卫 removed_names_group_covers_all_27 双向对账；注册面 84→57 扁平 + 828:0:0；**r43 扩展：REMOVED_KEYWORD_NAMES 3 对 + s2 负例组[旧形→新形指引 + 接管豁免]） | [20 §7](./20-surface-conventions.md) 移除轮行 + [20 §8](./20-surface-conventions.md) 映射表（单源）+ [22 §12 D29](./22-namespace-design.md)（关键字旧名扩展裁定）+ [23 §3.4](./23-evolution-governance.md) 生命周期四阶段（本码 = 第四阶段「移除」的机器执行——S1 内置腿 + S2 关键字腿双承载） |
| **W1003** | **宏名遮蔽内置名警告（r42 / 63-b——22 §11 D11 排期移除轮同窗兑现）**：`define-syntax` 宏名 ∈ 内置注册面（57 扁平 + 47 限定）→ W 级知会（宏胜出是宏系统本质能力[用户重定义语义合法场景]——遮蔽内置名值得知会；意图不可判定[故意 vs 意外] → W 级是唯一可判定位） | **已落地（r42——driver `collect_warnings`（Stx 层宏名收集——宏展开后名字从 CoreExpr 消失，此面唯一数据源）**：`[W1003]` 渲染前缀 + 每名去重一条；namespace_tests s1_w1003 组 2 case[正例知会 + 非内置名零误报 + 限定名遮蔽 + 宏胜出行为不变]；W 面现状 = W1002/W1003 | [22 §11 D11](./22-namespace-design.md) 裁定原文 + [20 §9.4](./20-surface-conventions.md) W 族设计（W 独立值域 1000+） |
| **E0013-E0019** | **命名空间族（批次 M 预留段）**：import 冲突（两模块同名导出）/限定名不导出（`string/nonexist`）/保留域违例（`kerf-` 前缀用户占用）/重复定义（module 体内同名 define）/别名重复/require 位置违例/未知导入模块 + **W 弃用警告族**（独立值域 1000+——M2 D6 裁定） | **全段落位（r39 M1 + r40 M2——七码全实施**：E0013 import 冲突/E0014 不导出/E0015 保留域/E0016 module 内重复 define/E0017 别名重复/E0018 require 位置[深审 D3]/E0019 未知导入[深审 D2]——driver front 管线验证族，stage 统一 Compile；W1001 弃用族 27 件 + W1002 遮蔽族——**r42/S1 注记：W1001 已随旧名退役下线（旧名升 E0021 编译期阻断——W 面收窄至 W1002/W1003）**，preamble 结构性豁免保留守 W1002 注入遮蔽误报） | [22 §8 实施对账表](./22-namespace-design.md) 各 case 的码位需求面 + [22 §10 D1-D9](./22-namespace-design.md) 深审裁定；「先查本表占位」纪律兑现（r34 预登记 → r39/r40 逐码落位回填） |

**登记纪律**：新码位 = 先查本表占位 → 模型层单源构造（TD-018）→ 回填本表 + 对应族文档锚。禁止未登记静默占位。
