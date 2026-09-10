# 术语表与术语源流考

> **Author**: kerf-doc-agent
> **Date**: 2026-09-09
> **Version**: v5.0（源自 stage0.md v5.0 拆分）
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
