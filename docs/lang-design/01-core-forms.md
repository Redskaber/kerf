# 最小自举单元的能力模型：九个核心原语

> **Author**: kerf-doc-agent
> **Date**: 2026-09-09
> **Version**: v5.0（源自 stage0.md v5.0 拆分）
> **Status**: Active

> 本文件收录 stage0.md §3「最小自举单元的能力模型」全文，包括五个核心能力模块、九个核心原语（最终定义）、语法对象模型与能力边界的初步划定。九个核心原语是整个语言的核心冻结对象——本文件是 [02-语法模型](./02-syntax-model.md)、[03-宏系统](./03-macro-system.md)、[04-字节码 VM](./04-bytecode-vm.md)、[05-运行时](./05-runtime.md) 各实现文件的语义根基；能力选型的批判性审视与 2026 年现代方案见 [14-替代设计](./14-design-alternatives.md)，Stage 0 的三层分类裁决见 [13-能力矩阵](./13-capability-matrix.md)。

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

**设计约束**：必须正交、必须完备、必须稳定（核心冻结原则——一旦定义，在整个语言生命周期内不变）。

**从 9 个原语推导的语法糖示例**：

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
