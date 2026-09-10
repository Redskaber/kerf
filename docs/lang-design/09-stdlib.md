# Stage 0 最小内置库边界

> **Author**: kerf-doc-agent
> **Date**: 2026-09-10（v5.2：内置清单重写为 24 项准确清单（#15）+ I/O 双层表面说明）
> **Version**: v5.2
> **Status**: Active
> **处理程度**：P1（最小集 Stage 0 已实现；完整标准库生长是 Stage 1→2 切换信号）｜ **所属 Stage**：Stage 0（最小集）→ Stage 1/2（库化生长） ｜ **推迟项**：列表操作/字符串处理库化（Stage 1+）、中缀运算符宏（Stage 1+）、能力模型 I/O（Stage 2）

> 本文件界定 Stage 0 的内置库边界：**语言核心零内置**——算术、比较、序对、谓词、I/O 与 print 等内置函数全部由 driver（宿主侧启动器）在启动时注册为全局函数，而不进入语言核心。设计依据提取自 stage0.md §8.8（最小 I/O）与 §14.5（语言规范与文档流程），并遵循 [01-核心原语 §2](./01-core-forms.md) 的核心冻结原则。相关实现：I/O 与分配器接口见 [05-运行时](./05-runtime.md)，操作码级能力见 [04-字节码 VM §1](./04-bytecode-vm.md)，12 个能力模型矩阵见 [13-能力矩阵](./13-capability-matrix.md)（其 §2.8 为本文件 §2 的规范副本）。

---

## 1. 语言核心零内置：设计依据（新撰章节）

[01-核心原语 §2](./01-core-forms.md) 定义的核心冻结原则要求：9 个核心原语（Lambda / App / If / VarRef / Literal / SetBang / Define / Begin / Module）**一旦定义，在整个语言生命周期内不变**；所有演化通过宏系统在核心之上叠加（核心冻结原则，另见 [17-设计原则 §1 原则 9](./17-principles.md)）。

由此推导出 Stage 0 的内置库边界：

1. **语言核心零内置**：`core_expr` 的 9 个原语中不含任何算术、比较、序对操作、类型谓词或 I/O 形式——它们不是核心形式，而是库函数。
2. **内置函数由 driver 注册**：算术（`+`/`-`/`*`/`/`/`mod`）、比较（`=`/`<`/`>`/`<=`/`>=`）、序对（`cons`/`car`/`cdr`/`list`）、谓词（`null?`/`pair?`/`int?`/`bool?`/`procedure?`/`eq?`（即时值按**内容**：字符串为内容比较——D2 修复））、逻辑（`not`）、I/O（`print`/`read-line`）与字符串（`str-append`）以**全局函数**的形式存在，由宿主侧 driver 在 VM 启动、进入用户程序之前注册进全局环境（`kerf-driver/src/builtins.rs` 的 `register_globals`——δ 函数表，[06-操作语义 §2 A4 规则](./06-operational-semantics.md) 的 δ 语义载体）。用户程序看到的是普通的 `App`（函数调用），不引入任何新的核心形式。
3. **这样分层的理由**：
   - 核心保持正交与完备（[01-核心原语 §2 设计约束](./01-core-forms.md)：任何原语不能被其他原语组合推导）——若把 `+` 或 `car` 提升为核心形式，将破坏最小化与正交性原则；
   - 内置函数可替换、可扩充——Stage 1+ 用目标语言重写这些函数或以宏重新定义语法（如中缀运算符宏）时，核心与编译器均无需改动（[17-设计原则 §1 原则 4 可替换性](./17-principles.md)）；
   - 与接口预留兼容——`register_foreign_ref` 等分配器/VM 接口（见 [05-运行时 §1](./05-runtime.md)）本身就是为宿主注册外部函数而预留的通道，Stage 1+ 升级到能力模型 I/O 时（[13-能力矩阵 §3.1.3](./13-capability-matrix.md)）替换的只是注册进来的实现，而非语言核心。
4. **操作码与内置函数的关系**：VM 操作码（ADD/SUB/MUL/DIV/MOD、NUM_* 比较组、MAKE_PAIR/CAR/CDR、谓词组等，见 [04-字节码 VM §1](./04-bytecode-vm.md)）是这些内置函数的**底层执行机制**——driver 注册的算术/序对/谓词全局函数最终编译为对应操作码序列；`read-line`/`print`/`str-append` 则经由外部函数接口由宿主实现（通道层见 [05-运行时 §1](./05-runtime.md)）。语言表面（核心形式集合）始终只有 9 个原语。

## 2. 最小 I/O 边界（提取自原 §8.8，v5.2 重写）

Stage 0 的 I/O 是**双层表面**：**语言层**仅有 `read-line` 与 `print` 两个用户可见内置函数（经 driver 注册的全局函数，非能力模型）；**通道层**是 [05-运行时 §1](./05-runtime.md) 的 `read_line_stdin()` / `write_line_stdout()`（kerf-runtime/src/io.rs，错误显式返回）。两层经 driver 内置函数接线（语言层 `read-line`/`print` 调用通道层函数）。Stage 0 不引入能力模型 I/O，但 VM 栈帧和分配器接口必须预留 `register_foreign_ref` 等接口（Stage 0 可为 no-op），以便 Stage 1+ 升级到能力模型时无需破坏接口——能力模型 I/O 的类型预留定义见 [13-能力矩阵 §3.1.3](./13-capability-matrix.md)。

**Stage 0 内置函数完整清单（24 项，v5.2 逐项对齐 `kerf-driver/src/builtins.rs` 的 `register_globals`——此前文档写「20 项」且函数名与实现表面错位，deep-review R1 偏差 #15）**：

| 类别 | 函数（个数） | 实现层 |
|------|------------|--------|
| 算术（5） | `+` / `-` / `*` / `/` / `mod` | VM 操作码（ADD/SUB/MUL/DIV/MOD；数值塔 Int×Int→Int 溢出检查、任一 Float→Float；`mod` 拒绝浮点操作数） |
| 比较（5） | `=` / `<` / `>` / `<=` / `>=` | VM 操作码（NUM_EQ/NUM_LT/NUM_GT/NUM_LE/NUM_GE；链式比较；字符串仅支持 `=`——TD-011） |

> **链式比较短路语义（Stage 0 显式裁定，FS-5）**：比较操作数链在首对
> 判定即终止整个链时（如 `(= 1 2 "s")` 首对不等 → false、`(< 3 2 "s")`
> 首对为假 → false），**后续操作数不做类型检查**；仅当链继续时才逐对
> 校验（`(< 1 2 "s")` 报类型错）。该值依赖的短路行为是 Stage 0 的既定
> 语义（实测锚定于 negative_vm_tests 语义边界注记），Stage 1 与类型
> 检查器联动时统一收紧为全操作数静态检查（TD-016）。
| 序对（4） | `cons` / `car` / `cdr` / `list` | VM 操作码（MAKE_PAIR/CAR/CDR）+ driver 注册（`list` 变长参数右折叠 cons） |
| 谓词（6） | `null?` / `pair?` / `int?` / `bool?` / `procedure?` / `eq?` | VM 操作码（IS_NULL/IS_PAIR/IS_INT/IS_BOOL/IS_PROCEDURE）+ EQ（`eq?` 恰 2 参不可链；即时值按值、堆值按引用） |
| 逻辑（1） | `not` | VM 操作码（NOT，仅 Bool） |
| I/O（2） | `print` / `read-line` | driver 注册的外部函数（语言层 → 通道层 read_line_stdin/write_line_stdout；`read-line` 元数不校验为已存档语义发现） |
| 字符串（1） | `str-append` | driver 注册的外部函数（恰 2 参字符串拼接） |

> **表面名与通道名的区分**（v5.2 澄清）：语言层用连字符命名（`read-line`/`str-append`——与 kerf 标识符规则一致）；通道层用 Rust snake_case（`read_line_stdin`/`write_line_stdout`）；谓词是 `null?`（非 `nil?`）。该清单为 Stage 0 的**最小骨架**：仅保证自举与测试所需（同 [12-路线图 §1.1](./12-roadmap.md) 里程碑验证清单的要求）；标准库的完整生长（列表操作、字符串处理、基本 I/O 的库化）是 Stage 1 → Stage 2 的阶段切换信号之一（见 [07-自举策略 §3.3](./07-bootstrap-strategy.md)）。完整清单以 `kerf-driver` 实现为准（δ 函数表）。`and`/`or`/`when`/`unless` 等是**语法糖**（[01-核心原语 §2](./01-core-forms.md) 推导表，展开期处理），不在内置函数表内。

## 3. 语言规范与文档流程（提取自原 §14.5）

文档即代码：Scribble 风格，规范与实现使用相同语言编写。Stage 0 的规范文档即本 `lang-design/` 文档集；随语言生长，规范应迁移为用语言自身编写（Scribble 风格的"规范即程序"），使规范与实现共享同一语法对象与宏系统——这也是 [17-设计原则 §1 原则 15 文档即代码](./17-principles.md) 的落地路径。参考案例见 [19-参考文献 §3.6 工具链（Scribble）](./19-references.md)。
