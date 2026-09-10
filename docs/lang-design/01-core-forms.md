# 最小自举单元的能力模型：九个核心原语

> **Author**: kerf-doc-agent
> **Date**: 2026-09-10（v6.0：新增 §8 内部语法设计——next3.md 第七轮吸收：类型安全 ADT 三原则 + 当前实现合规核验 + 旧→新迁移映射；v5.5：新增 §7 核心原语理论最小性与 2026 演进对照——next2.md 五轮吸收；v5.4：新增 §6 声明形式 require + 核心冻结边界精确化裁定；v5.2：糖推导示例「非穷举」注记（#5））
> **Version**: v6.0
> **Status**: Active（核心冻结对象，全生命周期不变）
> **处理程度**：P0（必须实现——Stage 0 已落地，kerf-core/src/expr.rs）｜ **所属 Stage**：Stage 0 定义、全生命周期冻结 ｜ **推迟项**：无（原语集合自身不变；周边能力的分级见 [13-能力矩阵](./13-capability-matrix.md)）

> 本文件收录 stage0.md §3「最小自举单元的能力模型」全文，包括五个核心能力模块、九个核心原语（最终定义）、语法对象模型与能力边界的初步划定。九个核心原语是整个语言的核心冻结对象——本文件是 [02-语法模型](./02-syntax-model.md)、[03-宏系统](./03-macro-system.md)、[04-字节码 VM](./04-bytecode-vm.md)、[05-运行时](./05-runtime.md) 各实现文件的语义根基；其**行为规范**（归约规则）见 [06-操作语义](./06-operational-semantics.md)；核心冻结原则的规范出处见 [17-设计原则 §1 原则 9](./17-principles.md)；能力选型的批判性审视与 2026 年现代方案见 [14-替代设计](./14-design-alternatives.md)，Stage 0 的三层分类裁决见 [13-能力矩阵](./13-capability-matrix.md)。

---

## 1. 五个核心能力模块（原 §3.1）

| 能力模型 | 实现语言 | 职责 | 接口契约 |
|---------|---------|------|---------|
| **Reader** | 宿主语言 | 字符流 → 语法对象 | `read : string → Result<SyntaxObject, ReadError>` |
| **Expander** | 宿主语言 | 语法对象 → 完全展开形式 | `expand : SyntaxObject → Result<CoreExpr, ExpandError>` |
| **Core Forms** | - | 9 个正交原语 | 代数数据类型定义 |
| **Bytecode VM** | 宿主语言/C | 执行字节码，验证语义 | `execute : Bytecode → Result<Value, RuntimeError>` |
| **Minimal Runtime** | C | 内存分配、基础数据操作 | C ABI 接口 |

## 2. 九个核心原语（最终定义）（原 §3.2）

```ocaml
type core_expr =
  | Lambda of { params : string list; body : core_expr }
  | App of { fn : core_expr; args : core_expr list }
  | If of { cond : core_expr; then_branch : core_expr; else_branch : core_expr }
  | VarRef of string
  | Literal of literal_value
  | SetBang of { name : string; value : core_expr }
  | Define of { name : string; value : core_expr }
  | Begin of core_expr list
  | Module of { name : string; imports : import_spec list;
                exports : export_spec list; body : core_expr list }

and literal_value =
  | Int of int | Float of float | String of string
  | Bool of bool | Nil
  | Pair of literal_value * literal_value
```

**设计约束**：必须正交、必须完备、必须稳定（核心冻结原则——一旦定义，在整个语言生命周期内不变，[17-设计原则 §1 原则 9](./17-principles.md)）。九原语的小步归约规则（行为规范）见 [06-操作语义 §2](./06-operational-semantics.md)；运行时的归约状态与值域见同文件 §1。

**`import_spec` / `export_spec` 子类型定义（v5.1 补齐——Stage 0 裁定）**：stage0.md 原文引用了这两个类型名但未定义。Stage 0 实现裁定（kerf-core/src/expr.rs）：

```ocaml
type import_spec = string        (* 被导入模块名——Stage 0 为名字引用（符号），
                                    无前缀重命名/按需选择子（Stage 1+ 考虑，
                                    [12-路线图 §2.5](./12-roadmap.md) 演进矩阵） *)
type export_spec = string        (* 被导出符号名——Stage 0 为裸符号（Phase 0 值），
                                    无相位标注导出（for-syntax 导出推迟至多模块隔离） *)
```

该裁定的相位簿记落地见 [03-宏系统 §1.1 ModuleRegistry 契约](./03-macro-system.md)（`imports: Vec<Symbol>` / `exports: Vec<Symbol>`）；R9 模块归约规则见 [06-操作语义 §2](./06-operational-semantics.md)。

**从 9 个原语推导的语法糖示例**（**非穷举**——v5.2 注，deep-review R1 偏差 #5：下表仅为代表性示例；`when`/`unless`/`let*` 及嵌套 cond/and/or 多参形态的推导未列出，完整推导以展开器实现为准（kerf-expander，[03-宏系统 §2.2](./03-macro-system.md)），负测覆盖见 negative_expander_tests 的 21 关键字误用矩阵）：

```text
let x = e in body        → ((lambda (x) body) e)
letrec f = e in body     → let f = nil in (set! f e) in body
cond [c1 e1] [else e3]   → (if c1 e1 e3)
and a b                  → (if a b false)
or a b                   → (if a true b)
while cond body          → (letrec loop (lambda () (if cond (begin body (loop)) nil))) (loop)
```

## 3. 语法对象模型（原 §3.3）

```ocaml
type stx_obj = {
  expr : stx_expr;
  span : span;            (* 源码位置——必须字段，不可为空 *)
  scopes : scope_set;     (* 作用域集合 *)
  phase : int;            (* 所属相位：0=运行时, 1=宏展开时 *)
}
```

语法对象是卫生宏系统的基础——它携带源位置、作用域集和相位信息，使得展开器可以区分"宏引入的标识符"和"用户代码中的标识符"。

## 4. 能力边界的初步划定（原 §3.4）

本节给出的五个能力模块（Reader / Expander / Core Forms / Bytecode VM / Minimal Runtime）是 Stage 0 的"骨架"。但其内部技术选型——例如同像性是否要采用 S 表达式、闭包是否要升级为 Effect Handlers、I/O 是否要采用能力模型——是一个需要结合 2026 年技术前沿重新审视的问题。接下来的 [14-替代设计 §1（超越 Lisp 范式）](./14-design-alternatives.md) 至 [14-替代设计 §3（2026 年五大能力现代方案）](./14-design-alternatives.md) 将系统化展开这一审视过程，并在 [13-能力矩阵 §1（Stage 0 能力矩阵与职责边界）](./13-capability-matrix.md) 给出最终的三层分类矩阵（必须实现 / 接口预留 / 完全推迟）。

## 5. 测试锚点（设计驱动测试，测试验证设计）

| 契约/不变式 | 测试锚点（tests/v0/stage0/） | 验证命题 |
|-----------|------------------------------|---------|
| 9 原语完备展开（糖推导表 §2） | plan 套件：let/let*/letrec/cond/and/or/when/unless/while 九种糖的展开正确 + "9 个核心形式展开正确"周判据 | 推导表与展开器一致 |
| 正交性（不可互相推导） | 排除实验：任一原语被移除后推导失败（设计审查论证，见 [17-设计原则 §1 原则 2](./17-principles.md)） | 无冗余原语 |
| 核心冻结（实现与定义一致） | kerf-core 单测：CoreExpr 9 变体逐字段对照本文件 §2 的 OCaml 定义 | 规范 ↔ 代码互锚 |
| 归约规则组（[06 §2](./06-operational-semantics.md)） | 双执行路径互查（全 73 集成测试） | R1–R9 实例化 |
| require 声明形式（§6） | stage1/plan 套件：capability_tests 23 case（声明/门控/豁免/形状负例） + test_runner_tests 前置切分 | 声明面与验证面一致 |

> 测试矩阵完整定义见 [11-测试基础设施 §3](./11-testing.md)；本表是其核心形式侧子集。

## 6. 声明形式 require（r8 批次 D 新增——能力 I/O 基础传递的程序侧声明面）

**形式文法**：`(require <主体> <能力>...)`——Stage 1 主体仅 `io`，能力项 `read`/`write`（声明为**幂等集合语义**，重复项去重；主体与能力项均为符号——非符号即展开期 E2 错误；未知主体/未知能力项报错并提示 Stage 2 扩展面 net/process）。

```ocaml
type capability =
  | IoRead    (* stdin 读：门控 read-line / read-int / read-num *)
  | IoWrite   (* stdout 写：门控 print / newline / write-string *)

type core_expr =
  | ...（9 原语不变）
  | Require of { caps : capability list }   (* 声明形式——零运行时语义 *)
```

**语义**：**零运行时语义**——不产生副作用，不求值结果恒为 `nil`（编译侧产 `PushNil`，eval 侧 `Ok(Nil)`，IR 侧降级 nil 共享字面量节点——T1 双路径一致）；不参与作用域分析（无变量引用、无自由变量）与静态类型检查（`Unknown`——权限验证归 R9）。它的**全部语义在编译期**：供 driver front 管线的 R9 保守权限验证（门控内置名引用未声明 → **E0006 编译期错误**，[13-能力矩阵 §3.1.3](./13-capability-matrix.md) 条款 3）与令牌铸造（`IoGrant` 按声明面授权）消费。

**核心冻结边界的精确化裁定**（本节 v5.4）：[17-设计原则 §1 原则 9](./17-principles.md) 冻结的是**语义原语**（九个原语正交完备、运行时语义全量流经它们——本文件 §2 定义不变）；`Require` 是**声明/注记变体**（ADT 第 10 变体——元数据节点，非语义节点）：它不增加任何归约规则（[06-操作语义](./06-operational-semantics.md) 的 R1–R9 不变，求值恒 nil 由编译/求值侧常量化处理）、不与任何原语组合推导、语义上可从程序中整体删除而不改变行为（R9 验证同时移除后程序仍等价——权限门控是外部约束而非程序语义）。**冻结的判定标准因此精确化为「语义原语集冻结 + 声明变体可追加」**——与 Racket `#%require` 形（模块导入声明，非语义原语）同构。Stage 2 语言级能力令牌值化时，`Require` 仍是声明面（令牌铸造的触发器），语义面由值语义承载——两层不会合流。

**测试锚点**：声明后可用（授权链）/未声明报 E0006（run/eval/check 三路径一致门控）/豁免（用户 define 同名接管不误报）/形状负例（缺参/未知主体/未知能力/非符号）——tests/v0/stage1/plan/capability_tests.rs（23 case）；require 为前置形式的切分约定见 [11-测试 §4](./11-testing.md)。

## 7. 核心原语的理论最小性与 2026 演进对照（v5.5 吸收自 next2.md 讨论轮）

> 本节吸收外部讨论（next2.md 第 1/3/4 轮）的核心原语批判性分析，作为**冻结 9 原语的对照坐标与 Stage 2+ 演进评估锚点**——不改变本文件 §2 的核心冻结裁定。

### 7.1 数量真相：9（本设计）vs 8（Racket kernel）vs 7（不可消除最小集）

**Racket kernel 实际是 8 个形式**（`#%plain-lambda`/`#%plain-app`/`#%plain-module-begin`/`#%datum`→`quote`/`quote`/`if`/`set!`/`define`）；本设计 9 原语中 **`Define` 理论上是 `Lambda + App + SetBang` 的语法糖**（`define x v ≡ ((lambda (x) …后续顶层…) v)` 的顶层锚定形态），**真正不可消除的最小集是 7 个**：`Lambda / App / If / VarRef / Literal / SetBang / Begin`——对应 Lambda 演算的 3 个（变量/抽象/应用）加 4 个实用扩展（分支/数据/副作用/顺序组合）。`Module`（第 8 语义原语，本设计超出 Racket kernel 的部分）服务相位分离——这是本设计对可拓展性的加法而非最小性的减损。

### 7.2 命名精确性对照（历史命名 vs 行为导向命名）

| 本设计命名 | 历史来源 | 精确性评价 | 行为导向替代 | 备注 |
|-----------|---------|-----------|------------|------|
| `Lambda` | Church 1932 | ✅ 精确 | `Fn` / `Abs` | 简洁性权衡保留 |
| `App` | β-归约传统 | ⚠️ 名词非行为 | `Apply` | Stage 2 表面语言命名参考 |
| `If` | 传统关键字 | ⚠️ 描述语法非行为 | `Branch` | 同上 |
| `VarRef` | 编译器术语 | ⚠️ 实现导向 | `Var`（de Bruijn 时 `Index`） | Stage 2 重写评估 |
| `Literal` | 编译器术语 | ⚠️ 不直观 | `Const` | 强调不可变 |
| `SetBang` | Scheme `!` 约定 | ❌ 极不直观 | `Assign` / `Mutate` | Stage 2 表面语法候选 |
| `Define` | 传统关键字 | ❌ 语义模糊 | `Bind` | 描述行为 |
| `Begin` | Scheme 传统 | ❌ 不精确 | `Seq` | 顺序求值 |
| `Module` | 通用术语 | ⚠️ 模糊 | `Phase`（强调相位） | 本设计语义即相位单元 |

**裁定**：核心 ADT 命名在冻结期内不变（§2 核心冻结）；本表作为 **Stage 2 表面语言（用户可见关键字）与目标语法 Reader 的命名参考**——表面语法与核心 ADT 解耦（[02-语法模型 §1](./02-syntax-model.md) 表面语法层可替换）。

### 7.3 Stage 2+ 演进候选：效应原语化（next2 推荐 8 原语形态）

next2 讨论的最终推荐（不考虑兼容性的重新设计）为 8 原语：`Fn / Let / Apply / Const / Var / Branch / Perform / Handle`——其中：
- **`Let`**（ANF 绑定）：Stage 2 ANF 转换层引入（[15-架构分层 §5](./15-architecture-layers.md) IR 层级演进——`Begin` 可模拟 `Let`（`begin a b → ((lambda (x) b) a)`）但无 ANF 优化能力）；
- **`Perform` / `Handle`**（效应执行 + 处理原语化）：与本设计当前路径（效应经 [13-能力矩阵 §3.1.1](./13-capability-matrix.md) 预留 + 编译器内部做实，I/O 经内置函数 + 能力门控）**同归殊途**——next2 路线把效应升为核心原语获得语言级表达力，本设计路径把效应留在元层保持核心极小。**Stage 2 语言级效应引入时此为首选评估对照**（`set! → Perform(State)`、`begin → Let 链` 的展开规则已由 next2 论证完备性）；
- **de Bruijn 索引**（消除变量名）：Stage 1 Expander 重写（批次 E）与 Stage 2 ANF 层的共同评估主题——"允许替换而不做 lambda 转换"（简化求值器与闭包转换）；
- **continuation 类型安全**（`Handle` 携带 resumption 类型三要素 + 线性唯一性）：Stage 2 效应安全的设计规格锚点——OCaml 5 已知缺陷（不静态确保效应被处理）的规避方案。

**本设计的对照结论**：冻结 9 原语（+Require 声明变体）在 Stage 0-1 不变；上列四项作为 **Stage 2 「目标语言完整化」门审查的评估清单**登记于 [12-路线图 §2.5.1](./12-roadmap.md) 演进矩阵注记。核心冻结原则（§2/原则 9）的精确化表述（v5.4 §6）不变：语义原语集冻结 + 声明变体可追加；效应原语化属于 Stage 2 语义层变更，须经 §13.2 切换期重构流程 + 委员会投票。8 原语语义等价迁移映射（9 冻结原语 → 8 原语形态的逐项映射表）见 §8.3 与 [upload/stage0.md §6.12.6](../stage0.md)。

## 8. 内部语法设计：类型安全 ADT 与语义化命名（v6.0 吸收自 next3.md 第七轮）

> 本节回答「内部语法该用派生关键词还是类型安全 ADT」——next3.md 终轮裁定：**不应该使用旧时代的派生关键词设计**（Racket `#%` 前缀标识符或 Scheme `define`/`set!` 历史名称）——`#%` 前缀是 S 表达式语境区分用户层/编译器层的历史妥协（仅当语法与 AST 同构时必要），2026 年正确设计是「类型安全 ADT + 语义化命名 + 零历史包袱」：内部 AST 节点是编译器私有数据结构而非用户可见标识符，安全性由类型系统而非命名约定保证。

### 8.1 三原则

1. **类型安全而非命名安全**：AST 节点是编译器私有类型（`enum CoreExpr`）——安全性由类型系统保证，用户代码不可能构造 `CoreExpr::Lambda`（除非经编译器 API）。旧设计依赖 `#%` 前缀命名约定（用户不可 shadow 的逃生舱）——从「约定」到「强制」；
2. **语义化命名而非历史命名**：名称精确描述节点行为（`Fn`/`Apply`/`Branch`/`Let`/`Const`/`Var`/`Perform`/`Handle`）而非 1960 年代数学传统（lambda/if/set!/define/begin）或逃生舱机制；
3. **零冗余而非多层转义**：`42 → Const(Int(42))` 直接映射无中间层——对比 Racket 多层转义 `42 → #%datum 42 → (quote 42) → 42`。

### 8.2 当前实现合规核验（三原则即刻生效为架构验证基准）

| 原则 | 核验项 | 结论 |
|------|--------|------|
| 类型安全 > 命名安全 | kerf-core 的 CoreExpr 是编译器私有 ADT（Rust enum 私有构造语义，用户代码无法构造） | ✅ 合规 |
| 语义化命名 > 历史命名 | 表面 S 表达式经 Reader 桥接到 CoreExpr（语法与 AST 分离） | ✅ 合规（表面/内部分离已落地） |
| 零冗余 | Span 系统独立于命名携带元数据（`kind_name` 仅诊断渲染用） | ✅ 合规 |

**命名本身**（Lambda/App/If/…）在冻结期内不变（§2 核心冻结 + §7.2 裁定）——三原则约束的是**架构形态**（私有 ADT / Reader 桥接 / Span 元数据），当前实现三项全合规；语义化命名形态（`Fn/Let/Apply/Const/Var/Branch/Perform/Handle`）作为 Stage 2 ADT 演进目标（§8.3）。

### 8.3 旧→新迁移映射（Stage 2 ADT 演进目标）

| 冻结原语（本设计 §2） | 8 原语形态 | 迁移性质 |
|---------------------|-----------|--------|
| Lambda | `Fn`（params: usize，de Bruijn） | 重命名 + 索引化 |
| App | `Apply` | 重命名（动词化） |
| If | `Branch` | 重命名（行为化） |
| VarRef | `Var`（index，de Bruijn） | 重命名 + 索引化 |
| Literal | `Const`（吸收 quote/#%datum 两层） | 重命名 + 零冗余化 |
| SetBang | `Perform(State)` | 副作用 → 效应 |
| Define | （消除——`Apply[Fn, value]` 语法糖） | 脱糖 |
| Begin | `Let` 链（ANF 顺序） | 脱糖（`Let` 为新增原语） |
| Module | （移至模块系统层，非语义原语） | 层级迁移 |
| （新增）Let | `Let` | ANF 必需的新原语 |
| （新增）Perform/Handle | `Perform`/`Handle` | 效应执行/处理配对 |
| Require（声明变体） | （保留——声明面与语义面两层不合流，§6 裁定） | 不变 |

迁移须走 Stage 2 「目标语言完整化」门审查（同 §7.3 裁定：§13.2 切换期重构流程 + 委员会投票）。2026 形态完整 Rust ADT 定义（8 变体 + EffectKind + HandlerClause + LiteralValue）见 [upload/stage0.md §7.4.3](../stage0.md)；六维度对比表（安全性/精确性/冗余度/模式匹配/元数据/可扩展性）见同文件 §7.4.3。

### 8.4 表面/内部语法严格分离（架构不变量）

表面语法是可替换的用户接口（皮肤——S 表达式/中缀/DSL 均可，经 Reader 桥接），内部语法是编译器私有不变量（骨架——核心原语不变）：任何表面语法的编译产物为相同 CoreExpr，语义验证与语法选择正交（[02-语法模型 §1](./02-syntax-model.md) 表面语法层可替换 + [12-路线图 §2.4.1](./12-roadmap.md) 分阶段语法策略：Stage 0 S-expr → Stage 1 S-expr on VM（r6 B3 已兑现）→ Stage 2 目标语法 → Stage 2+ 多语法）。设计哲学定位：**这不是对 Racket 设计的否定，而是站在 Racket 30 年经验之上的「青出于蓝」**——`#%` 前缀是 S 表达式语境的最优解，类型安全 ADT 是现代编译器语境的最优解。

### 8.5 测试锚点（§8 新增）

| 锚点 | 验证方式 | 状态 |
|------|---------|------|
| 内部 ADT 私有性 | kerf-core 单测：CoreExpr 十变体字段对照本文件 §2 + §8.2 三项合规（私有构造/Reader 桥接/Span 独立） | ✅ r9 在位（01 §5 既有锚点覆盖变体对照；三原则合规为架构审计项） |
| 迁移映射完备性 | 设计审计：§8.3 十二行映射逐行覆盖 §2 全部原语（含 Require）+ 新增项 | ✅ 本节自检（十二行 = 9 原语 + Require + Let + Perform/Handle） |
| 表面/内部分离 | 集成：同一 kerf 程序经 Reader 唯一入口产 CoreExpr（无表面语法直通内部 AST 的旁路） | ✅ r9 在位（driver front 管线唯一组合根，[15 §5.1](./15-architecture-layers.md)） |
