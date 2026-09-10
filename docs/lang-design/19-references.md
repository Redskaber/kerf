# 参考案例、代码量估算与参考文献

> **Author**: kerf-doc-agent
> **Date**: 2026-09-09
> **Version**: v5.0（源自 stage0.md v5.0 拆分）
> **Status**: Active

> 本文件收录 stage0.md 附录 B（参考案例与关键数据）、附录 C（代码量估算）与附录 E（参考文献、规范与相关链接，按主题分类）。各条目的「文档引用位置」已从 stage0.md 的内部锚点改写为指向本目录对应设计文件的相对链接。术语表与术语源流考见 [18-术语文档](./18-terminology.md)。

---

## 1. 参考案例与关键数据（原附录 B）

| 项目 | 启示 | 关键数据 |
|------|------|---------|
| Racket | 极小核心+卫生宏的完整实践 | 9 个形式；BC→CS 证明核心不变可换后端 |
| Chez Scheme | 从极小核心生长出高性能编译器 | flonum unboxing 显著提升性能 |
| Guix | 357 字节种子构建完整系统 | hex0 → 22,000+ 包 |
| Rust 编译器 | 三阶段自举流水线 | `cfg(bootstrap)` 条件编译 |
| PyPy | 元追踪 JIT 性能数据 | 追踪阶段约 900 倍减速（仅预热期） |
| Julia | femtolisp 到 LLVM JIT | femtolisp 仅用于编译期 |
| C 语言 | 高度自举但有汇编漏洞 | 预处理器非图灵完备 |
| Cone 编译器 | LLVM 后端性能开销 | 前端 800μSec vs 后端 115,500μSec |
| Zig | 自建后端替换 LLVM | 构建时间几乎减半 |
| QBE | 极简编译后端 | 10% 代码提供 70% 性能 |
| Cranelift | 快速代码生成后端 | 比 LLVM 快约 40% |
| Salsa | 查询式增量计算框架 | 从 rustc 提取 |
| APL/J/K | 数组同像性范式 | 数组作为代码表示 |
| Prolog | 一阶项同像性 | assert/retract 元编程 |
| Forth | 栈与字典范式 | 4 个核心原语（NEXT/DOCOL/EXIT/LIT） |
| MetaOCaml | 多阶段编程类型化 | 良构、良类型、良作用域保证 |
| OCaml 5 | Effect Handlers | 模块化效应处理 |
| Mojo（2026） | Python 语法 + 编译期元编程 | 内存安全模型仍在完善 |
| Gleam | 类型安全 + 可扩展编译器 | 现代 BEAM 语言设计 |
| Esterel | 同步语言信号流图 | 程序编译为有限状态机 |
| MLton | 全程序优化编译器（Standard ML） | ~1x C 效率 + 小体积可执行文件（v5.5 增补，next2） |
| Koka | 行多态效应类型 + 效应消除编译 | 静态消除处理器显著提升性能（v5.5 增补，next2） |
| Unison | 能力（abilities）系统 | 生产级能力模型实践（v5.5 增补，next2） |
| CompCert | 形式化验证编译器 | 唯一经机器辅助证明免误编译的编译器（航空认证）（v5.5 增补，next2） |
| Zig comptime | 编译期求值 | 元编程逻辑与常规代码同语言（v5.5 增补，next2） |

---

## 2. 代码量估算（原附录 C）

```text
Stage 0（约 6500 行）：
  OCaml/Rust 前端（约 5000 行）
  C VM + Runtime + GC（约 1500 行）

Stage 1（约 8000 行，目标语言子集）
Stage 2（约 20000-50000 行，完整目标语言）

[v3.0] Stage 0 接口预留层（约 500-800 行）
  Effect Handlers 类型定义（约 100 行）
  多阶段编程类型定义（约 100 行）
  能力模型 I/O 类型定义（约 100 行）
  编译缓存接口定义（约 100 行）
  Span/诊断/相位分离的辅助类型（约 200-400 行）

Stage 0 合计：约 7,000-7,300 行 = 6,500 行核心实现 + 500-800 行接口预留层
（v3.0 的"约 6500 行"仅指核心实现；v4.0 起将接口预留层计入交付总量，见 [12-路线图 §3.3](./12-roadmap.md) 验证清单）
```

---

## 3. 参考文献、规范与相关链接（原附录 E）

> **本附录提供 v3.0 文档涉及的所有参考文献、规范文档与相关链接，按主题分类组织。** 所有链接已校验（截至 2026 年 9 月），并标注其在拆分后文档集中的引用位置。

### 3.1 自举理论与历史（原 E.1）

#### 3.1.1 关键论文（原 E.1.1）

1. **McCarthy, J. (1960)**. *Recursive Functions of Symbolic Expressions and Their Computation by Machine, Part I*. Communications of the ACM, 3(4), 184-195.
   - LISP 的原始论文，引入元循环求值器
   - 文档引用位置：[00-总览 §4](./00-overview.md)、[14-替代设计 §2.1](./14-design-alternatives.md)、[18-术语文档 §2.1](./18-terminology.md)

2. **Landin, P. J. (1964)**. *The mechanical evaluation of expressions*. Computer Journal, 6(4), 308-320.
   - 引入闭包与 SECD 机器
   - 文档引用位置：[14-替代设计 §2.4](./14-design-alternatives.md)、[18-术语文档 §2.5](./18-terminology.md)

3. **Church, A. (1936)**. *An unsolvable problem of elementary number theory*. American Journal of Mathematics, 58(2), 345-363.
   - λ 演算的奠基论文
   - 文档引用位置：[14-替代设计 §2.3](./14-design-alternatives.md)、[18-术语文档 §2.4](./18-terminology.md)

4. **Quine, W. V. O. (1940)**. *Mathematical Logic*. Harvard University Press.
   - 引入准引用（quasiquotation）概念
   - 文档引用位置：[18-术语文档 §2.4.3](./18-terminology.md)

#### 3.1.2 工程实践（原 E.1.2）

5. **Guix Full-Source Bootstrap**：https://guix.gnu.org/manual/en/html_node/Full_002dSource-Bootstrap.html
   - 357 字节 hex0 种子到 22,000+ 包的完整自举链
   - 文档引用位置：[00-总览 §4.2](./00-overview.md)

6. **Rust Bootstrapping**：https://rustc-dev-guide.rust-lang.org/building/bootstrapping.html
   - Rust 编译器的三阶段自举流水线与 `cfg(bootstrap)` 机制
   - 文档引用位置：[00-总览 §4.1](./00-overview.md)

7. **Racket `racket/kernel`**：https://docs.racket-lang.org/reference/kernel.html
   - 9 个核心原语与卫生宏系统
   - 文档引用位置：[00-总览 §3.1](./00-overview.md)、[00-总览 §4.3](./00-overview.md)

8. **Julia femtolisp**：https://github.com/JuliaLang/julia/tree/master/contrib
   - Julia 语言的引导解析器
   - 文档引用位置：[00-总览 §4.4](./00-overview.md)

### 3.2 同像性与替代设计（原 E.2）

9. **Joel Kuiper: Homoiconicity**：http://joelkuiper.eu/homoiconicity
   - 对同像性的系统分析与批判
   - 文档引用位置：[14-替代设计 §1.2](./14-design-alternatives.md)

10. **Hacker News: Homoiconicity discussion**：https://news.ycombinator.com/item?id=7418055
    - 关于"AST 与数据结构关系"的深度讨论
    - 文档引用位置：[14-替代设计 §1.2](./14-design-alternatives.md)

11. **Stack Overflow: What is homoiconicity?**：https://stackoverflow.com/questions/267862/what-is-homoiconicity
    - 同像性作为"谱系"而非二元属性的讨论
    - 文档引用位置：[14-替代设计 §1.2](./14-design-alternatives.md)

#### 3.2.1 数组语言（原 E.2.1）

12. **APL Wiki**：https://aplwiki.com/
    - APL 语言社区百科
    - 文档引用位置：[14-替代设计 §1.3.1](./14-design-alternatives.md)

13. **J Language**：https://www.jsoftware.com/
    - Iverson 创建的 ASCII 字符 APL 后继
    - 文档引用位置：[14-替代设计 §1.3.1](./14-design-alternatives.md)

14. **K/Q (KX Systems)**：https://kx.com/
    - 商业数组语言，金融领域应用
    - 文档引用位置：[14-替代设计 §1.3.1](./14-design-alternatives.md)

#### 3.2.2 Prolog 与一阶项（原 E.2.2）

15. **SWI-Prolog Documentation**：https://www.swi-prolog.org/pldoc/man?section=manipulate
    - 项操纵与元编程
    - 文档引用位置：[14-替代设计 §1.3.2](./14-design-alternatives.md)

#### 3.2.3 Forth 与栈语言（原 E.2.3）

16. **Forth Standard**：https://forth-standard.org/
    - Forth 200x 标准
    - 文档引用位置：[14-替代设计 §1.3.3](./14-design-alternatives.md)

17. **Jonesforth**：https://github.com/nornagon/jonesforth
    - 可读的 Forth 实现注释
    - 文档引用位置：[04-字节码 VM §1](./04-bytecode-vm.md)

#### 3.2.4 图结构 IR（原 E.2.4）

18. **Cranelift**：https://cranelift.dev/
    - 快速代码生成后端，使用图 IR
    - 文档引用位置：[14-替代设计 §1.3.4](./14-design-alternatives.md)、本文 §1（参考案例与关键数据）

19. **LLVM IR**：https://llvm.org/docs/LangRef.html
    - LLVM 中间表示，图结构 SSA 形式
    - 文档引用位置：[14-替代设计 §1.3.4](./14-design-alternatives.md)

#### 3.2.5 PEG 与文法（原 E.2.5）

20. **LPeg (Lua PEG library)**：http://www.inf.puc-rio.br/~roberto/lpeg/
    - PEG 作为可嵌入"语言"的实践
    - 文档引用位置：[14-替代设计 §1.3.5](./14-design-alternatives.md)

#### 3.2.6 Rust proc-macro（原 E.2.6）

21. **Rust Procedural Macros**：https://doc.rust-lang.org/reference/procedural-macros.html
    - Rust 官方过程宏文档
    - 文档引用位置：[14-替代设计 §1.3.6](./14-design-alternatives.md)、[14-替代设计 §3.2](./14-design-alternatives.md)

22. **`syn` crate**：https://docs.rs/syn/latest/syn/
    - Rust 类型化 AST 解析库
    - 文档引用位置：[14-替代设计 §1.3.6](./14-design-alternatives.md)

#### 3.2.7 MetaOCaml（原 E.2.7）

23. **MetaOCaml (Oleg Kiselyov)**：http://okmij.org/ftp/ML/MetaOCaml.html
    - 多阶段编程的官方资源
    - 文档引用位置：[14-替代设计 §1.3.7](./14-design-alternatives.md)、[14-替代设计 §3.1](./14-design-alternatives.md)

24. **Multi-stage Programming Paper**：Taha, W. (1999). *A sound reduction semantics for untyped CBN multi-stage computation*.
    - 多阶段编程的理论基础
    - 文档引用位置：[14-替代设计 §3.1](./14-design-alternatives.md)

#### 3.2.8 Esterel（原 E.2.8）

25. **Esterel (Gérard Berry)**：https://www-sop.inria.fr/members/Gerard.Berry/Esterel.html
    - 同步语言的创始资源
    - 文档引用位置：[14-替代设计 §1.3.8](./14-design-alternatives.md)

### 3.3 2026 年现代方案（原 E.3）

#### 3.3.1 OCaml 5 Effects（原 E.3.1）

26. **OCaml 5 Effects Manual**：https://v2.ocaml.org/manual/effects.html
    - OCaml 5 效应系统官方文档
    - 文档引用位置：[14-替代设计 §3.4](./14-design-alternatives.md)

27. **Anil Madhavpeddy - Effects for Compiler Pipelines**：https://anil.recoil.org/
    - Anil 关于效应系统应用于编译器管线的研究
    - 文档引用位置：[14-替代设计 §3.1](./14-design-alternatives.md)、[14-替代设计 §3.4](./14-design-alternatives.md)

28. **OCaml Multicore**：https://github.com/ocaml/ocaml
    - OCaml 5 多核与效应系统的源代码
    - 文档引用位置：[13-能力矩阵 §3.1.1](./13-capability-matrix.md)

#### 3.3.2 Mojo 语言（原 E.3.2）

29. **Mojo Documentation**：https://docs.modular.com/mojo/
    - Mojo 语言官方文档
    - 文档引用位置：[14-替代设计 §3.2](./14-design-alternatives.md)、本文 §1（参考案例与关键数据）

30. **Modular Inc.**：https://www.modular.com/
    - Mojo 母公司，Chris Lattner 创立
    - 文档引用位置：[14-替代设计 §3.5](./14-design-alternatives.md)

#### 3.3.3 Gleam（原 E.3.3）

31. **Gleam Language**：https://gleam.run/
    - 类型安全的 BEAM 语言
    - 文档引用位置：[14-替代设计 §3.2](./14-design-alternatives.md)

#### 3.3.4 Capability-based Security（原 E.3.4）

32. **Capability-based Security (Wikipedia)**：https://en.wikipedia.org/wiki/Capability-based_security
    - 能力模型安全机制概览
    - 文档引用位置：[14-替代设计 §3.5](./14-design-alternatives.md)

33. **Capability Myths Demolished**：https://srl.cs.jhu.edu/pubs/SRL2003-02.pdf
    - Mark Miller 等关于能力模型的权威分析
    - 文档引用位置：[14-替代设计 §3.5](./14-design-alternatives.md)

### 3.4 编译器工程实践（原 E.4）

#### 3.4.1 rustc 与 Salsa（原 E.4.1）

34. **rustc Dev Guide**：https://rustc-dev-guide.rust-lang.org/
    - Rust 编译器开发指南，包含查询系统
    - 文档引用位置：[15-架构分层 §3.1](./15-architecture-layers.md)

35. **Salsa Framework**：https://github.com/salsa-rs/salsa
    - 从 rustc 提取的查询式增量计算框架
    - 文档引用位置：[15-架构分层 §3.1](./15-architecture-layers.md)、本文 §1（参考案例与关键数据）

#### 3.4.2 后端（原 E.4.2）

36. **QBE**：https://c9x.me/compile/
    - 极简编译后端，10% 代码提供 70% 性能
    - 文档引用位置：[08-后端演化 §2](./08-backend-evolution.md)、本文 §1（参考案例与关键数据）

37. **Cranelift**：https://cranelift.dev/
    - 快速代码生成后端
    - 文档引用位置：[08-后端演化 §2](./08-backend-evolution.md)、本文 §1（参考案例与关键数据）

38. **LLVM**：https://llvm.org/
    - 工业级编译器基础设施
    - 文档引用位置：[08-后端演化 §1](./08-backend-evolution.md)

#### 3.4.3 信任与安全（原 E.4.3）

39. **Thompson, K. (1984)**. *Reflections on Trusting Trust*. Communications of the ACM, 27(8), 761-763.
    - Trusting Trust 攻击的原始论文
    - 文档引用位置：[00-总览 §3.3](./00-overview.md)、[15-架构分层 §4.1](./15-architecture-layers.md)

40. **Wheeler, D. A. (2009)**. *Fully Countering Trusting Trust through Diverse Double-Compiling*.
    - DDC（多样化双重编译）方法
    - 文档引用位置：[15-架构分层 §4.1](./15-architecture-layers.md)

### 3.5 教学资源（原 E.5）

41. **SICP (Abelson & Sussman)**：https://mitpress.mit.edu/sites/default/files/sicp/index.html
    - 《计算机程序的构造和解释》
    - 文档引用位置：[14-替代设计 §2.1](./14-design-alternatives.md)

42. **EOPL (Essentials of Programming Languages)**：https://eopl3.com/
    - 编程语言本质，元语言抽象章节
    - 文档引用位置：[14-替代设计 §2.1](./14-design-alternatives.md)

43. **Types and Programming Languages (Pierce)**：https://www.cis.upenn.edu/~bcpierce/tapl/
    - 类型系统基础理论
    - 文档引用位置：[06-操作语义](./06-operational-semantics.md)

### 3.6 工具链（原 E.6）

44. **rust-analyzer**：https://rust-analyzer.github.io/
    - LSP 实现参考
    - 文档引用位置：[10-工具链 §1](./10-toolchain.md)

45. **Scribble**：https://docs.racket-lang.org/scribble/
    - Racket 的文档即代码系统
    - 文档引用位置：[09-标准库 §3](./09-stdlib.md)

### 3.7 v3.0 引用的额外资源（v4.0 保留）（原 E.7）

46. **TRAC Language (Mooers)**：https://en.wikipedia.org/wiki/TRAC_(programming_language)
    - 同像性术语的发源语言
    - 文档引用位置：[14-替代设计 §2.2](./14-design-alternatives.md)、[18-术语文档 §2.3](./18-terminology.md)

47. **SECD Machine**：https://en.wikipedia.org/wiki/SECD_machine
    - Landin 的抽象机器
    - 文档引用位置：[14-替代设计 §2.4](./14-design-alternatives.md)、[18-术语文档 §2.5](./18-terminology.md)

48. **Forester 6.0**：https://opentype.us/forester/
    - 使用 OCaml 5 Effects 的实际项目
    - 文档引用位置：[13-能力矩阵 §1.1.1](./13-capability-matrix.md)、[13-能力矩阵 §2.4](./13-capability-matrix.md)

49. **PyPy Meta-tracing JIT**：https://www.pypy.org/
    - 元追踪 JIT 性能数据
    - 文档引用位置：[08-后端演化 §3](./08-backend-evolution.md)、本文 §1（参考案例与关键数据）

50. **Cone Compiler**：https://www.jondgoodwin.com/cone/
    - LLVM 后端性能开销实测数据来源
    - 文档引用位置：[08-后端演化 §1](./08-backend-evolution.md)、本文 §1（参考案例与关键数据）

## 4. next2 讨论引用增补（v5.5 吸收）

- **MLton**：https://mlton.org/ ——全程序优化（ANF + 闭包转换 + SSA）达 ~1x C 的实证；三种闭包表示（Toplevel/Flat/Linked）为本设计 Stage 2 优化锚点
- **Koka**：https://koka-lang.github.io/ ——行多态效应类型 + 类型导向编译（效应消除四阶段：纯函数检测/重排/内联/已知状态消除）
- **Unison**：https://www.unison-lang.org/ ——abilities 能力系统（能力=不可撤销效应的视角）
- **CompCert**：https://compcert.org/ ——形式化验证编译器（Stage 3+ 安全目标参照）
- **Zig comptime**：https://ziglang.org/documentation/master/#comptime ——编译期求值的生产实践（多阶段编程的务实前身）
- **λ○▷ staging 演算**（研究跟踪）——let-splice 绑定机制（Stage 3+ 多阶段复杂度缓解候选）
