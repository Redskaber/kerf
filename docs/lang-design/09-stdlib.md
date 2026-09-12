# Stage 0 最小内置库边界

> **Author**: kerf-doc-agent
> **Date**: 2026-09-12（**v6.4：r32 / 53-a 表面现代化审查轮**——新增 §4 命名规范层指针（[20-表面规范](./20-surface-conventions.md) R1-R6 + 57 项映射表——谓词 `?`→`is-`、转换 `->`→`to/from`、`car/cdr`→`head/tail`、`str-` 全词化；§2 表标注为 v0.4 遗留口径——新语料按现代名书写）；v6.3：r24 / 42-e stdlib 缺口补齐**——类型谓词 5 件（string?/symbol?/float?/number?/list?——Value 变体判别完备面 + Floyd 龟兔环安全）+ prelude foldr（foldl 对偶）+ **TD-011 解决**：字符串全序码点序参与全部比较族（静态面 R3 同步）；v6.2：批次 F 深审回写——TD-021 hofs 用户面已 r15 解决；v5.6：r8——I/O 六内置能力门控；v5.5：r7——TD-016 链式比较全操作数前置校验；v5.4：r6 B3——高阶函数四件套 kerf 源码化 + 4 自举 Reader 原语）
> **Version**: v6.9（r40 / 61-b 批次 M M2：**W1001 旧名弃用警告族 v0.6 弃用期启动**——27 旧名引用 → `[W1001]` stderr 非阻断警告（每名去重 + 现代名指引 + preamble 结构性豁免[引导语料 20 §6.5 纪律]）；W1002 遮蔽注入名警告（22 §3.2 R-N2 第二行）；B1/B2 限定名新契约 miss→nil——旧名/扁平名契约不动[20 §4 迁移不变量]；v1.0 前夜移除轮清算）（**r39 / 60-a 批次 M M1**：§2 增限定名可见面注记[47 限定名 + E0014/E0015]；v6.7：r38 批次 L 别名层实施；v6.6：r34 十一域口径；v6.5：r33；v6.4：r32 命名规范层首增）
> **Status**: Active
> **处理程度**：P1（最小集 Stage 0 已实现；标准库最小集（阶段门条件 3：列表/字符串/I/O 各 ≥8）r5 已交付；高阶函数 kerf 源码实现 r6 已交付（reader.krf 序章）；**r24 / 42-e：谓词完备面 + foldr + 字符串全序交付——I2 stdlib 缺口清单清零**；完整库化生长是 Stage 2 切换信号）｜ **所属 Stage**：Stage 0（最小集）→ Stage 1（r5 最小集补齐 / r6 hof 源码化）→ Stage 2（库化生长） ｜ **推迟项**：~~高阶函数用户面注入~~（**已解决 r15**——kerf-prelude 模块承载）、~~字符串全序比较~~（**已解决 r24**——TD-011 码点序，见 §2 表）、中缀运算符宏（Stage 2）、能力模型 I/O（Stage 2）

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

## 2. 最小 I/O 边界（提取自原 §8.8，v5.2 重写；v5.6 能力门控注记）

Stage 0 的 I/O 是**双层表面**：**语言层**仅有 `read-line` 与 `print` 两个用户可见内置函数（经 driver 注册的全局函数，非能力模型）；**通道层**是 [05-运行时 §1](./05-runtime.md) 的 `read_line_stdin()` / `write_line_stdout()`（kerf-runtime/src/io.rs，错误显式返回）。两层经 driver 内置函数接线（语言层 `read-line`/`print` 调用通道层函数）。Stage 0 不引入能力模型 I/O，但 VM 栈帧和分配器接口必须预留 `register_foreign_ref` 等接口（Stage 0 可为 no-op），以便 Stage 1+ 升级到能力模型时无需破坏接口——能力模型 I/O 的类型预留定义见 [13-能力矩阵 §3.1.3](./13-capability-matrix.md)。

> **r8 能力门控注记（2026-09-10，批次 D，v5.6）**：上段「Stage 0 不引入能力模型」描述的是 Stage 0 基线；r8 起 I/O 内置进入**能力门控形态**（[13-能力矩阵 §3.1.3](./13-capability-matrix.md) r8 注记——「基础传递」做实）：程序须声明 `(require io read|write)` 才能引用门控内置（`print`/`newline`/`write-string` 需 write；`read-line`/`read-int`/`read-num` 需 read）；未声明引用 → **E0006 编译期错误**（R9 保守验证，front 全路径）；`register_globals` 按授权面注册（未声明即不注册——fail-closed）。语言层/通道层双层表面不变，只是语言层入口加了权限门。门控不覆盖非 I/O 内置（算术/比较/序对/谓词/字符串——它们无副作用，无需授权）。
**Stage 0/1/2 内置函数完整清单（57 项，v6.3：+5 类型谓词（r24/42-e 缺口补齐——Value 变体判别完备面）；v5.4：48 项用户面（v5.3）+ 4 项自举 Reader 原语（r6，B3）——逐项对齐 `kerf-driver/src/builtins.rs` 的 `register_globals`）**：

> **r39 批次 M M1 限定名可见面注记（v6.8）**：上表之上叠加**七模块 47 限定名**（`core/is-nil`、`string/append`、`io/print`…——`STDLIB_MODULES` 单源转译 20 §5.2；`ns/本地名` = 底层共享分派体；io 族按授权分项 fail-closed 注册；未命中 export 面 → E0014「不导出」编译期[R-N3 不回落]；`kerf-` 模块名前缀保留域 E0015）。限定名签名按模块表派生（check/hm 对 `ns/name` 同判）。import 注入面/别名 = M2。
>
> **r38 批次 L 双注册注记（v6.7）**：上表 57 项为 **v0.4 遗留命名口径的规范事实**（δ 函数表事实源不变）；r38 起 `register_globals` 在其上叠加 **27 现代扁平名别名双注册**（注册面 57→84——`BUILTIN_ALIASES` 单源；同名共享分派体：新旧名同行为同诊断，27 parity case + 三方漂移守卫锄定——[20 §6.4/§8](./20-surface-conventions.md)）。旧名存量面继续工作（移除轮 Stage 3）；**新语料一律现代名书写**（§4 纪律——r32 已登记，r38 起生效）。

| 类别 | 函数（个数） | 实现层 |
|------|------------|--------|
| 算术（5） | `+` / `-` / `*` / `/` / `mod` | VM 操作码（ADD/SUB/MUL/DIV/MOD；数值塔 Int×Int→Int 溢出检查、任一 Float→Float；`mod` 拒绝浮点操作数） |
| 比较（5） | `=` / `<` / `>` / `<=` / `>=` | driver 注册（cmp_builtin；链式比较；**全字符串链按 Unicode 码点序参与全族**——TD-011 r24 解决：`Rc<str>` 比较 = UTF-8 字节序 = 码点序（编码保序性）；混合链报 `{op} 需要数值`（TD-016 首个非数值归因）；NUM_* 操作码为 ISA 级数值域原语——编译器不特判，比较走全局 builtin 分派） |

> **链式比较全操作数前置校验（Stage 1 批次 C收紧，TD-016 已解决，v5.5）**：比较操作数链的
> **全部操作数先做类型检查再逐对比较**（`cmp_builtin` 前置校验：全数值或（仅 `=`）全字符
> 串；否则首个非数值操作数报 `{op} 需要数值`）。修复前的 Stage 0 短路语义（FS-5 边界：
> `(< 3 1 "a")` 首对为假即静默返回 false、后续操作数不检查）已收敛——TD-016 登记项
> 与类型检查器（r7 `kerf check` 静态面 R3 规则）同步落地。静态检查面对同样口径：
> 混串排序链报「需要数值」、全字符串排序链报 TD-011 边界消息（口径与运行时一致）。
> 消息兼容性：既有两参消息逐条保持（负例矩阵回归锚 negative_vm_tests）。
> 顺带收敛：`(< "a" "b" 1)` 混串链由 TD-011 消息统一为「< 需要数值」
> （首个非数值操作数归因——两参口径不变）。
| 序对（4） | `cons` / `car` / `cdr` / `list` | VM 操作码（MAKE_PAIR/CAR/CDR）+ driver 注册（`list` 变长参数右折叠 cons；空参 → nil 值形态，r5） |
| 列表操作（8，r5） | `length` / `append` / `reverse` / `list-ref` / `list-tail` / `member` / `assoc` / `last-pair` | driver 注册（堆序对链遍历；nil 终结契约——improper 拒绝，除 `append` 末参原样与 `member` 首匹配；`member`/`assoc` 按 `eq?` 查找，命中返回子表/点对、未命中 false） |
| 谓词（11） | `null?` / `pair?` / `int?` / `bool?` / `procedure?` / `eq?` / `string?` / `symbol?` / `float?` / `number?` / `list?` | driver 注册（即时值判定；`eq?` 恰 2 参不可链——即时值按值、堆值按引用；**r24/42-e +5**：`number?` = Int∪Float（数值塔域）；`list?` 真表判定 = nil 或 cdr 链终止于 nil 的序对链——Floyd 龟兔环安全（环 → false，引用 Racket 语义）） |
| 逻辑（1） | `not` | VM 操作码（NOT，仅 Bool） |
| I/O（2） | `print` / `read-line` | driver 注册的外部函数（语言层 → 通道层 read_line_stdin/write_line_stdout；**能力门控**（v5.6/r8：print 需 `(require io write)`、read-line 需 read——未声明报 E0006）；`read-line` 元数校验已补齐（FS-4 修复，恰 0 参）） |
| 字符串（1） | `str-append` | driver 注册的外部函数（恰 2 参字符串拼接） |
| 字符串处理（10，r5） | `str-length` / `str-substring` / `str-index-of` / `str-contains?` / `str-prefix?` / `str-suffix?` / `str-upcase` / `str-downcase` / `string->symbol` / `symbol->string` | driver 注册（字符索引 Unicode 安全——非字节；`str-index-of` 未找到 -1；大小写 Unicode 变换；符号互转依赖 TD-002 符号值） |
| 基本 I/O（6，r5） | `newline` / `write-string` / `read-int` / `read-num` / `error` / `assert-eq?` | driver 注册（`newline` 0 参；`write-string` 无换行——通道层 write_stdout（r5 新增）；`read-int`/`read-num` 行解析（失败结构化报错，EOF → nil）；`error` ≥1 参消息部件（str 原文、其余类型名）；`assert-eq?` 按 `eq?` 断言；**前四项能力门控**（v5.6/r8：newline/write-string 需 write、read-int/read-num 需 read）） |
| 自举 Reader 原语（4，r6/B3） | `str->pos-chars` / `char-whitespace?` / `char-alphabetic?` / `str-int-valid?` | driver 注册（**运行时服务层，非语言语义面**：服务 reader.krf——字符级索引（(字节偏移 . 单字符 str) 列表）、Unicode White_Space/Alphabetic 属性判定、i64 域校验（Rust parse 同源——维持错误次序 parity）。与 Racket 的 string-ref/char-whitespace? 同层） |

> **高阶函数（r6 B3 交付 + r24/42-e foldr 补齐）**：`map` / `filter` / `foldl` / `foldr` / `for-each` 已以 **kerf 源码**
> 实现于 `kerf-driver/src/bootstrap/reader.krf` 序章（语言自描述的标准库片段），在
> Stage 0 VM 上运行并经自举桥（`bootstrap::selfcheck_call`）直测——这是「高阶函数
> 用 kerf 源码 preamble 实现」的自举验证命题本体（r5 裁定）的交付面。**用户面注入
> 推迟批次 E**：P1 源码拼接（Span 诊断污染 = P1 缺陷）/ P3 跨程序全局合并（SymbolTable
> id 不可比 + 原型索引程序局部）/ P5 builtin 调闭包（VM 递归 re-entry 越界 §11）三方案
> 已否决——正确载体是模块系统（批次 E Expander 重写时设计）。在此之前 hofs 仅为
> Reader 内部全局（用户程序引用 `map` 报未绑定变量）——TD-021。
>
> **v6.2 / r15 解决注记（TD-021 已闭环）**：用户面注入已随 E1-β 交付——
> `kerf-prelude` 模块（`bootstrap/preamble.krf`）导出 map/filter/foldl/**foldr**/
> for-each（reader.krf 序章同源；foldr 为 r24/42-e 补齐——foldl 的对偶，
> 从表尾累积 `(f 首元素 递归果)` 形态）；用户程序 `(module 名 (import kerf-prelude) ...)`
> 声明后以普通全局函数可调用（forms 级合并注入单一编译单元——独立 file_id，
> Span 指向 preamble.krf 自身，无源码拼接诊断污染；P1/P3/P5 三否决方案全规避）。
> **opt-in 语义**：无 import 声明仍报未绑定（显式失败优于静默遮蔽）；同名
> define 显式报「重复定义」。测试锚点 prelude_tests 7 case。上段「报未绑定」
> 描述的是 r15 之前的边界，保留作历史口径。

> **表面名与通道名的区分**（v5.2 澄清——v6.4 起由 [§4 命名规范层](#4-命名规范层v64-新增r32-表面现代化) 承载更新版，本段保留作历史锚）：语言层用连字符命名；通道层用 Rust snake_case（`read_line_stdin`/`write_line_stdout`）。`and`/`or`/`when`/`unless` 等是**语法糖**（[01-核心原语 §2](./01-core-forms.md) 推导表，展开期处理），不在内置函数表内。

## 3. 语言规范与文档流程（提取自原 §14.5）

文档即代码：Scribble 风格，规范与实现使用相同语言编写。Stage 0 的规范文档即本 `lang-design/` 文档集；随语言生长，规范应迁移为用语言自身编写（Scribble 风格的"规范即程序"），使规范与实现共享同一语法对象与宏系统——这也是 [17-设计原则 §1 原则 15 文档即代码](./17-principles.md) 的落地路径。参考案例见 [19-参考文献 §3.6 工具链（Scribble）](./19-references.md)。

## 4. 命名规范层与架构层（v6.4 新增——r32 表面现代化；v6.5 r33 扩展指针；v6.6 r34 十一域口径）

> 本节为指针节：§2 的 57 项清单是 **v0.4 遗留命名口径的规范事实**（与 `builtins.rs` 逐名对齐——δ 函数表事实源不变；**r38 批次 L 起叠加 27 现代名别名双注册[注册面 84——§2 r38 注记]，实施状态 ✅ 详见 [20 §6.4/§7](./20-surface-conventions.md)**）；**命名规范与迁移方向由 [20-表面规范](./20-surface-conventions.md) 独立承载**（2026 现代化：谓词 `?`→`is-` 前缀 / 转换 `->`→`to`/`from` 方向词 / `car`/`cdr`→`head`/`tail` / `str-` 前缀→全词 + v0.6 命名空间 `kerf/<模块>` 限定 + 行为契约 B1-B3 哨兵/真值多态现代化）；**能力域架构由 [21-能力架构](./21-capability-architecture.md) 承载**（r33 起八域 → **r34/v1.1 十一域四要素划分**——补算术运算符域（N1 永驻）/诊断终止域（`error`/`assert-eq?`）/横切泛函域（prelude 五件）三卡，**57 注册名 + prelude 五件每名有家**（覆盖闭合）+ 域正交依赖单向图 + 副作用汇聚规则 D3「门控表 = I/O 域全集」的架构根据）；**命名机制由 [22-命名空间设计](./22-namespace-design.md) 承载**（r33——五层命名层级/解析优先序/遮蔽许可表/权限矩阵——v0.6 批次 M 实施输入；r34/v1.1 增组合闭包与相位授权两行）；**演进治理由 [23-演进治理](./23-evolution-governance.md) 承载**（r34——演进六窗触发表 + 准入判据总表 + 生命周期节律——批次 L/M/E5 的时间治理 owner）。**纪律**（批次 L 起）：新语料（bootstrap 新增代码/examples 新件/测试新 case）一律现代名书写；存量旧名经 v0.5 别名层双注册过渡（行为 parity 锚定）、v0.6 命名空间层终态化、Stage 3 移除（与 E5 关键字切换同窗）。v5.2 版注记「谓词是 `null?`（非 `nil?`）」自此读作**遗留口径描述**（v0.4 事实）而非**命名规范**（规范 = 20 §3 R2）。

**表面名与通道名的区分**（v5.2 澄清，保留——通道层不变）：通道层用 Rust snake_case（`read_line_stdin`/`write_line_stdout`）；语言层命名规范见 [20-表面规范 §3](./20-surface-conventions.md)（R1-R6）。该清单为 Stage 0 的**最小骨架**：仅保证自举与测试所需（同 [12-路线图 §1.1](./12-roadmap.md) 里程碑验证清单的要求）；标准库的完整生长（列表操作、字符串处理、基本 I/O 的库化）是 Stage 1 → Stage 2 的阶段切换信号之一（见 [07-自举策略 §3.3](./07-bootstrap-strategy.md)）；库化生长的**命名与组织规范** = [20-表面规范 §5](./20-surface-conventions.md) 模块树（`kerf/core`/`kerf/list`/`kerf/string`/`kerf/io`——能力-命名空间对齐）。完整清单以 `kerf-driver` 实现为准（δ 函数表）。`and`/`or`/`when`/`unless` 等是**语法糖**（[01-核心原语 §2](./01-core-forms.md) 推导表，展开期处理），不在内置函数表内。
