# 全局测试矩阵（覆盖率追踪）

> **Author**: kerf-dev-agent（QA-A 角色）
> **Date**: 2026-09-10（**r10 架构合规审计**：+4 architecture_audit_tests（sop §2.2 原则 29-31 形态审计——lang-design 01 §8.5/02 §8.1 锚点落地：十变体穷尽 match 冻结证明/Span 独立/Reader-Stx 类型隔离/Expander 唯一桥/同源同核确定性）——476 → 480；r9 测试入口架构重构：Cargo.toml [[test]] 18 块 → tests/runner.rs 单一总入口 mod 树，sop.md §8.4.6 v11.2）
> **r12 增量**（2026-09-11）：+20 接口预留扩展测试（单元 8：reserved/ Probe 冻结——toolchain 3 + ffi 3 + codegen 2；集成 12：reserved_ext_tests——P0 位置跨 crate 断言 + 14 项 API 可达性 + 形状行为含负向）——480 → **500**。
> **r13 增量**（2026-09-11，批次 E 首个 MUV——TD-004 作用域集解析收口）：+9 集成（scope_set_tests——Racket 式 `(name, scopes ⊆)` 双路径语义锚点：嵌套 shadowing/闭包捕获/set! 词法命中 4 正 + 作用域不匹配未绑定 VM/eval/set! 3 负 + 宏引入不捕获 + 子集对照）——500 → **509**。
> **r14 增量**（2026-09-11，批次 E / E1-α 自举 Expander）：+19 集成（bootstrap_expander_tests——expander.krf（核心形式 + 九糖，VM 上运行）与 Rust 种子 parity：结构+Span+作用域集+param_scopes 递归一致 / 错误消息+Span 逐字一致 / define-syntax E1-α 边界 / 行为面端到端可执行）——509 → **528**。
> **r15 增量**（2026-09-11，批次 E / E1-β 宏收口 + 生产切换 + TD-021）：+24 集成（bootstrap_expander_tests 19→36：宏 parity 17——define-syntax/syntax-rules/卫生 α 重命名/省略号（零/多段/复合）/字面量/多子句/糖覆盖/深度上限/向量模式 + prelude_tests 7——TD-021 hofs 用户面/组合管道/双路径/opt-in/显式失败/未知导入）+ +1 单元（driver 生产切换守护 production_expander_is_bootstrap——独立线程活性探针 + 展开代次标记）——528 → **553**。
> **Version**: v0.1.0-r15
> **Status**: Active

## 总量

**553 通过 / 0 失败 / 0 忽略**（553 个测试函数 = 单元 184 + 集成 369，逐二进制实测汇总；r10 +4 架构审计 + r12 +20 预留扩展 + r13 +9 作用域集锚点 + r14 +19 自举 Expander parity + r15 +25 宏收口/prelude/生产切换守护）。
§3.2 release 验收基线 204（r2）→ r3 负向测试扩张 + 审计集就位 + FS-1 守卫修复 + 糖正向锚点 + T17-a 六缺陷修复回归后 297 → r4（Stage 1 批次 A）304 → r5（批次 B TD-002 符号值 + 标准库最小集）324 → r6（批次 B 收官 B3 自举 Reader）356 → r7（批次 C 类型检查器 + 编译缓存 + TD-016）408 → **r8（批次 D 能力 I/O + 内部效应 + 用例运行器）476** → **r10（架构合规审计 +4）480**：+24 capability_tests（require 声明面 + E0006 三路径门控 + 豁免/形状/令牌 + EOF 子进程探针）+ +18 test_runner_tests（前置切分 + PASS 判定 + 短路/恢复/隔离 + front 错误面）+ +25 kerf-driver 单元（effects.rs 12：逃逸层/最近匹配/载荷保真/穿透契约 + capability.rs 13：R9 验证/豁免/编组）+ +1 negative_vm_tests（read_line_arity 自 ignore 激活——**FS-4 修复**，能力参数化重写时补齐元数校验）→ **r12（接口预留完整性扩展 +20）500**：+8 单元（reserved/ 模块 Probe 冻结——「测试实现体编译通过 = 契约冻结」先例沿用）+ +12 集成 reserved_ext_tests（P0 数据结构位置六项断言 + 预留 API 跨 crate 可达 + 负向形状：空目标拒绝/空片段类型检查失败/rename 错误面）→ **r13（批次 E·TD-004 作用域集解析收口 +9）509**：+9 集成 scope_set_tests（双路径语义锚点 + 作用域不匹配负例——见 docs/tests/v0/stage1/plan/scope-set.md）→ **r14（批次 E·E1-α 自举 Expander +19）528**：+19 集成 bootstrap_expander_tests（expander.krf 与 Rust 种子 parity——结构/Span/作用域集/param_scopes/错误消息逐字一致 + 行为面端到端；见 docs/tests/v0/stage1/plan/bootstrap-expander.md）→ **r15（批次 E·E1-β 宏收口 + 生产切换 + TD-021 prelude +25）553**：+17 宏 parity（镜像 macro_sys.rs：变换器注册表单表语义/卫生基名回退/省略号/字面量/Span 并集代次守卫（expansion_id 镜像——节点第 4 字段 + retag +1）/深度上限 500 消息逐字）+ +7 prelude_tests（TD-021 模块/import 承载——forms 级合并注入单一编译单元）+ +1 单元生产切换守护（compile_front 展开段经 bootstrap_expander——独立线程活性探针实测）。**全套件经自举 Reader + 自举 Expander（均 kerf 源码，VM 上运行）执行——生产管线读+展开两段全自举（E1-β）。**

> **r7 计数修正**（r8 对账发现，§8.4.5 规则 2——以实测为准）：r7 版本矩阵的分套件表存在陈旧数（头部「集成 173 函数」为 r3 时代口径；单元表 130 实为 150——driver 14→25 / expander 26→28 / compiler 12→15 的 r4-r7 增长未回写；cache_tests 13 实为 14；negative_vm 29 为排除 ignore 的口径）。r7 实际 = 150 单元 + 260 集成函数（259 通过 + 1 ignore）= 408:0:1 ✓（总量正确、分项陈旧）。r8 起全部逐二进制实测。
>
> r8 起 `#[ignore]` 清零：FS-4（read-line 元数）随批次 D 能力参数化重写修复激活；
> r3 内已修复激活（原 4 个 ignore → 1）：FS-1 嵌套守卫（10_000→256）、
> D3 eval 深度上限（MAX_EVAL_DEPTH=256 结构化）、D4 mod/除 i64::MIN
> 溢出（checked_div/rem）。

## 分套件统计（2026-09-10 r8 实测）

### 单元测试（183，crates 内联——r12 +8：reserved/ Probe 冻结）

| 套件 | 层级 | 文件/位置 | 测试数 |
|------|------|----------|--------|
| kerf-span 单元 | crate 内联 | crates/kerf-span/src/*.rs | 11 |
| kerf-syntax 单元 | crate 内联 | crates/kerf-syntax/src/*.rs | 11 |
| kerf-core 单元 | crate 内联 | crates/kerf-core/src/*.rs | 10 |
| kerf-reader 单元 | crate 内联 | crates/kerf-reader/src/*.rs | 23 |
| kerf-expander 单元 | crate 内联 | crates/kerf-expander/src/*.rs | 28 |
| kerf-compiler 单元 | crate 内联 | crates/kerf-compiler/src/*.rs | 15 |
| kerf-runtime 单元 | crate 内联 | crates/kerf-runtime/src/*.rs | 9 |
| kerf-vm 单元 | crate 内联 | crates/kerf-vm/src/*.rs | 18 |
| kerf-driver 单元 | crate 内联 | crates/kerf-driver/src/*.rs | **58（r8 +25：effects.rs 12 + capability.rs 13；r12 +8：reserved/ Probe 冻结——toolchain 3 + ffi 3 + codegen 2）** |

### 集成测试（345 函数，tests/ 阶段树——r9 起经 runner.rs 单一总入口组织；r10 +4 审计；r12 +12 预留扩展；r13 +9 作用域集锚点；r14 +19 自举 Expander parity）

> **入口口径（r9）**：`tests/runner.rs` 为唯一集成测试目标（cargo 自动发现，Cargo.toml 零 [[test]] 声明）；下表各「套件」现为 runner 内 `#[path]` mod 树的**模块**（选择性运行 `cargo test --test runner <module>::`）——逐模块计数与 r8 逐二进制口径完全一致（476 总数不变，组织收敛）。共享辅助 `tests/common/` 经 runner 单实例共享（`use crate::common`——替代原每文件 `mod common` 重复加载）。

| 套件 | 文件 | 函数数 |
|------|------|--------|
| reader_tests | tests/v0/stage0/plan/reader_tests.rs | 10 |
| expander_tests | tests/v0/stage0/plan/expander_tests.rs | 18 |
| compiler_tests | tests/v0/stage0/plan/compiler_tests.rs | 10 |
| vm_tests | tests/v0/stage0/plan/vm_tests.rs | 20 |
| gc_tests | tests/v0/stage0/plan/gc_tests.rs | 6 |
| pipeline_tests | tests/v0/stage0/plan/pipeline_tests.rs | 11 |
| architecture_audit_tests（r10） | tests/v0/stage0/plan/architecture_audit_tests.rs | **4（r10 新增）** |
| gate_review_r1 | tests/v0/stage0/gate/gate_review_r1.rs | 8 |
| negative_reader_tests | tests/v0/stage0/plan/negative_reader_tests.rs | 14 |
| negative_expander_tests | tests/v0/stage0/plan/negative_expander_tests.rs | 23 |
| negative_vm_tests（r8 +1 激活） | tests/v0/stage0/plan/negative_vm_tests.rs | 30 |
| negative_semantics_tests | tests/v0/stage0/plan/negative_semantics_tests.rs | 23 |
| stdlib_tests（r5；r7 +TD-016；r8 门控前缀） | tests/v0/stage1/plan/stdlib_tests.rs | 17 |
| bootstrap_reader_tests（r6） | tests/v0/stage1/plan/bootstrap_reader_tests.rs | 28 |
| expansion_worklist_tests（r4） | tests/v0/stage1/plan/expansion_worklist_tests.rs | 4 |
| typecheck_tests（r7） | tests/v0/stage1/plan/typecheck_tests.rs | 24 |
| cache_tests（r7） | tests/v0/stage1/plan/cache_tests.rs | 14 |
| **capability_tests（r8，批次 D）** | tests/v0/stage1/plan/capability_tests.rs | **24** |
| **test_runner_tests（r8，批次 D）** | tests/v0/stage1/plan/test_runner_tests.rs | **18** |
| **reserved_ext_tests（r12，预留扩展）** | tests/v0/stage1/plan/reserved_ext_tests.rs | **12**（P0 位置断言 4 + API 可达 1 + 形状行为 7——含负向：空目标拒绝/空片段类型检查失败/rename 错误面） |
| **scope_set_tests（r13，批次 E·TD-004）** | tests/v0/stage1/plan/scope_set_tests.rs | **9**（双路径正例 5：shadowing/嵌套 shadowing/闭包捕获/set! 词法命中/子集对照；负例 4：作用域不匹配未绑定 VM/eval/set! 三锚 + 宏引入不捕获） |
| **bootstrap_expander_tests（r14，批次 E·E1-α）** | tests/v0/stage1/plan/bootstrap_expander_tests.rs | **19**（parity 正例 8 套件：字面量/符号引用/嵌套作用域/if-set!-define-begin/quote/九糖/let 族/module-require/内部 define 提升；负例 6 套件：核心形式/提升路径/quote 向量/糖/module-require-关键字；边界 1：define-syntax；行为面 3：算术闭包/cond-while-fib/内部定义） |

### 负向测试规模与正负比（§9.4.3 对账）

| 负测文件 | 函数数 | case 数（按文件内注释汇总） |
|----------|--------|------------------------------|
| negative_reader_tests.rs | 14 | 59 |
| negative_expander_tests.rs | 23 | 96 |
| negative_vm_tests.rs | 30 | 237（r5 +6 符号值误用；r8 +1 read-line 元数 FS-4 修复锚） |
| negative_semantics_tests.rs | 23 | 98 |
| **四文件合计** | **90** | **490** |
| stdlib_tests（r5+r7+r8，tests/v0/stage1） | 17 | 181+（元数 25/类型 86/边界 12/语义 31/双参扫描 18/r7 TD-016 +9——r8 起门控 I/O 负例携带 require 前缀，case 集不变） |
| typecheck_tests（r7，tests/v0/stage1） | 24 | 负例 68 case + 正例锚 25 case |
| bootstrap_reader_tests（r6，tests/v0/stage1） | 28 | 307 |
| 审计集（examples/audit/stage0_gate_audit_r1.rs） | — | 41（负向 32 + 恢复 6 + 正向 3） |
| reserved_ext_tests（r12，负向形状） | 12 | 负向形状 case 3（空目标/空片段/rename 错误面）+ pin/unpin 对称断言 |

- **全局正负比（case 口径）≈ 1:3.15 维持**：r8 新增负向 case ≈
  capability_tests 15+（门控 8 + 形状 4 + 边界 3）+ test_runner_tests 12
  （失败/短路/front 错误面）+ effects/capability 单元负向 15 ≈
  **1118 vs 正向 ≈355**（r7 1077:340 基础上同步扩张）——
  **§9.4.3 的 ≥1:3 门限维持达标**（r1 审查时为 1:0.24）。
- 逐分类负测非零：reader/expander/compiler/vm/gc/pipeline/gate/driver 全部含负向 case。
- §7.1.1 七类负向矩阵 7/7（含空应用与模块循环依赖）；E 码直接断言：E1–E6 全部
  （negative_semantics_tests 逐码矩阵）+ E0001/E0002/E0004 结构化断言
  （negative_reader/expander/vm）+ E0005 静态检查码（r7 typecheck_tests 逐条断言）
  + **E0006 能力权限码（r8 capability_tests 逐条断言——第七族结构码就位）**；
  E7/E8/E0003 经公开 API 不可触发——文档化存档。
- 负测文档锚点：[negative-tests.md](./v0/stage0/plan/negative-tests.md)。

## 需求覆盖（sop.md §21.3 Stage 0 验收标准 → 测试）

| 验收项 | 覆盖测试 |
|--------|---------|
| (1) 9 原语语义正确 | vm_tests::nine_primitives_semantics + gate_g1/g2 + negative_semantics E 码矩阵 |
| (2) 50+ 快照测试 | 全套件（476 ≥ 50） |
| (3) 自举测试（同结果） | compiler_tests::deterministic + pipeline_tests::convergent |
| (4) Span 全管线传播 | pipeline_tests::span_propagates + gate_g6 + negative_reader/expander Span 精确断言 |
| (5) 性能基准基线 | CLI bench（fib(25) ~86-89ms/轮实测；examples/usage/fib.krf） |
| (6) 四项接口预留冻结 | gate_g7_to_g10 + reserved.rs Probe 测试（reserved_signatures_are_frozen 等）+ **r8：EffectSystem/InternalEffectSystem 真实现与 Probe 构成契约双证；CapabilityIO 经 StdCapabilityIO 做实** |
| (7) §7.3.1 门审计集 ≥30 case | examples/audit/stage0_gate_audit_r1.rs（41 case，配比满足） |

## 测试类型分布

- 快照/黄金输出：Token 快照 / Stx 渲染 / CoreExpr 渲染 / 反汇编 / 运行结果渲染
- 语义断言：9 原语 / fib / 闭包 / 宏 / quote / GC
- 负向/错误（表格驱动，每行 = 1 case）：词法 59 / 展开 96 / VM 237 / 语义 98 / 审计 41 / 能力门控 8+ / 用例失败 12+——全阶段错误路径
- 双路径互查：VM vs eval（全部集成函数内置；错误程序断言 Err 事实一致）
- 压力/稳健：10^6 有界分配（gc_tests，验收口径）/ 3×10^5（gc_stress 示例口径）/ 10^4 深递归 / 2×10^5 深链标记
- **r8 新形态**：效应系统契约测试（一次性逃逸层的最近匹配/载荷保真/真实 panic 穿透）；能力授权链端到端（声明→验证→令牌→门控注册）；子进程确定性探针（EOF 语义——不依赖运行器 stdin 形态）
