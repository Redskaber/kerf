# 最小自举单元的能力模型：九个核心原语

> **Author**: kerf-doc-agent
> **Date**: 2026-09-10（v5.4：新增 §6 声明形式 require——r8 批次 D 能力 I/O 基础传递的程序侧声明面 + 核心冻结边界精确化裁定；v5.2：糖推导示例「非穷举」注记（#5））
> **Version**: v5.4
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
