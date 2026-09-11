# 全局测试矩阵（覆盖率追踪）

> **Author**: kerf-dev-agent（QA-A 角色）
> **Date**: 2026-09-10（**r10 架构合规审计**：+4 architecture_audit_tests（sop §2.2 原则 29-31 形态审计——lang-design 01 §8.5/02 §8.1 锚点落地：十变体穷尽 match 冻结证明/Span 独立/Reader-Stx 类型隔离/Expander 唯一桥/同源同核确定性）——476 → 480；r9 测试入口架构重构：Cargo.toml [[test]] 18 块 → tests/runner.rs 单一总入口 mod 树，sop.md §8.4.6 v11.2）
> **r12 增量**（2026-09-11）：+20 接口预留扩展测试（单元 8：reserved/ Probe 冻结——toolchain 3 + ffi 3 + codegen 2；集成 12：reserved_ext_tests——P0 位置跨 crate 断言 + 14 项 API 可达性 + 形状行为含负向）——480 → **500**。
> **r22 增量**（2026-09-11，批次 I 执行 / 42-c I1 中段糖与 module/require 面）：**657 → 665（净 +8，集成侧）**——bootstrap_compiler_tests 19 → 27：边界负例 1 改写为正例 parity（module/require 两臂迁移落地——边界不对称消除）+ 新增 9 函数（module/require 正例组 + 确定性双跑 / 糖三组：let 家族 8 + cond/when/unless 9 + and/or/while 10 / 宏语料 3 / prelude 注入序 2（preamble.krf 真实语料）/ examples 全六件双路径 / 行为面 +2 组：糖九件 + module 臂）；case 口径扩展组 46 ≥ 12 超额。口径：单元 202 不变 + 集成 455 → 463。
> **r21 增量**（2026-09-11，批次 I 执行 / 42-b I1 前段基础核心形式 kerf 化）：**638 → 657（净 +19）**——集成 +19（bootstrap_compiler_tests：门 A parity 13——字面量/常量池去重/引号点对递归/全局与 define/if 跳转回填/begin 尾位继承/lambda 嵌套捕获三链+形参遮蔽/set! 三路径/尾位穿线含相互尾递归与 let 脱装/深嵌套 100 层/确定性双跑与空程序 + 负例 2（define 位置 D1 消息+Span 逐字；module/require 42-c 边界显式断言）+ 行为面 4（fib 144/closures 计数器 (4 2)/higher_order map 平方/10 万层深尾递归——自举编译段产物 VM 执行 = 生产管线结果））；**opcode_count 冻结测试修正（40→41）**：TailCall（r18 TCO 引入）漏列于守护枚举——R4（代码为准 + 本次修正文档）三方同步（opcode.rs 测试 + 04-bytecode-vm 三处）。口径：单元 202 不变 + 集成 436 → 455。
> **r20 增量**（2026-09-11，批次 I 执行启动 / 42-a I1 切口评估与迁移设计）：**638 → 638（零测试增量——纯设计轮）**：i1-incision-migration-design.md（切口裁定 INC1-INC8 + 段序 S1-S3 + parity 三门 A/B/C + CompilerKind 切换点 P1-P5 + 确定性纪律）；plan.md §5a 42-b/c/d 行引用该设计为验收合同。638 零回归全绿复跑（§3.2 clean 起步）。
> **r19 增量**（2026-09-11，批间插入轮 / 41-a 能力模型泛化设计）：**634 → 638（净 +4）**——单元 +4（kerf-driver reserved/capability_model.rs Probe：骨架冻结 + io 族两令牌归属证明 + ffi 令牌归属证明 + 演算位签名证明）。口径注记：单元 198 → 202（driver 60 → 64）；集成 436 不变。零回归实证：capability.rs 12 测试 + reserved_ext_tests 12 集成零改动通过（原则 27）。
>
> **r18 增量**（2026-09-11，批次 H / 语义演进评估轮 + 三债清偿 + HM PoC）：**605 → 634（净 +29）**——集成 +27（tco_tests 12：TD-022 尾调用帧复用/尾位传播/指令预算护栏/TD-007·022 耦合自举端到端 + hm_inference_tests 15：H4 HM PoC 超集门/零误报门/四类缺口/occurs/值限制/多错误）+ 单元 +2（40-g reserved Probe 拆分：合并冻结测试 → 4 子模块独立 Probe）。负例改写注记：帧上限/深递归追踪/双层调用链三用例改非尾形态（TCO 语义变更——尾递归不再耗帧）；编译器两单测指令序列断言 Call→TailCall；expander 深度消息 500→10000 三处同步。口径注记：单元 196 → 198（driver 58 → 60——Probe 拆分净增）；集成 409 → 436。
> **r13 增量**（2026-09-11，批次 E 首个 MUV——TD-004 作用域集解析收口）：+9 集成（scope_set_tests——Racket 式 `(name, scopes ⊆)` 双路径语义锚点：嵌套 shadowing/闭包捕获/set! 词法命中 4 正 + 作用域不匹配未绑定 VM/eval/set! 3 负 + 宏引入不捕获 + 子集对照）——500 → **509**。
> **r14 增量**（2026-09-11，批次 E / E1-α 自举 Expander）：+19 集成（bootstrap_expander_tests——expander.krf（核心形式 + 九糖，VM 上运行）与 Rust 种子 parity：结构+Span+作用域集+param_scopes 递归一致 / 错误消息+Span 逐字一致 / define-syntax E1-α 边界 / 行为面端到端可执行）——509 → **528**。
> **r17 增量**（2026-09-11，批次 G / 后端·FFI·类型三主线）：**553 → 605（净 +52）**——集成 +40（qbe_backend_tests 24：G1 QBE 后端 PoC 端到端/结构/一致性/负例/契约 + multi_error_recovery_tests 16：TD-013 恢复双路径/合并诊断/上限/次序/短路边界）+ 单元 +12（kerf-backend 新 crate 9：codegen 契约迁移 3 + qbe 2 + aot 4；kerf-expander +4 recover 单元；kerf-driver reserved/codegen 契约测试 2 迁移 -2 + 兼容锚 1）。口径注记：单元分项以逐二进制实测为准（196 = span 11 + syntax 11 + core 10 + reader 23 + expander 32 + compiler 15 + runtime 9 + vm 18 + driver 58 + backend 9）。
>
> **r16 增量**（2026-09-11，批次 F / Stage 1 深审收尾环）：零测试变更（纯审查 + 文档 + 注释轮——**553 零断言修改逐一等价复跑**：探针临时部署/移除各一次全绿验证）；本行 + 表体两行对账（bootstrap_expander_tests 19→36 的 r15 尾差 + prelude_tests 行补录——31-e 起 header 增量与表体同步义务的漏网，36-d 发现）。
> **r15 增量**（2026-09-11，批次 E / E1-β 宏收口 + 生产切换 + TD-021）：+24 集成（bootstrap_expander_tests 19→36：宏 parity 17——define-syntax/syntax-rules/卫生 α 重命名/省略号（零/多段/复合）/字面量/多子句/糖覆盖/深度上限/向量模式 + prelude_tests 7——TD-021 hofs 用户面/组合管道/双路径/opt-in/显式失败/未知导入）+ +1 单元（driver 生产切换守护 production_expander_is_bootstrap——独立线程活性探针 + 展开代次标记）——528 → **553**。
> **Version**: v0.1.0-r22
> **Status**: Active

## 总量

**665 通过 / 0 失败 / 0 忽略**（665 个测试函数 = 单元 202 + 集成 463，逐二进制实测汇总；r10 +4 架构审计 + r12 +20 预留扩展 + r13 +9 作用域集锚点 + r14 +19 自举 Expander parity + r15 +25 宏收口/prelude/生产切换守护 + r17 +52 批次 G：QBE 后端 PoC 40 + TD-013 恢复 12 + r18 +29 批次 H：TCO 12 + HM PoC 15 + Probe 拆分 2 + r19 +4 批间插入轮：能力模型骨架 Probe 4 + r20 +0 设计轮：I1 切口设计（parity 三门 A/B/C 为 42-b/c/d 增量测试的验收合同）+ r21 +19 批次 I·I1 前段：自举 Compiler 门 A parity（bytecode_equal 全结构判据）+ 42-c 边界负例 + 行为面端到端 + **r22 +8 批次 I·I1 中段：module/require 两臂 + 糖九件全管线 + 宏 + prelude 注入序 + examples 六件双路径（扩展组 46 case）+ 行为面糖/module**）。
§3.2 release 验收基线 204（r2）→ r3 负向测试扩张 + 审计集就位 + FS-1 守卫修复 + 糖正向锚点 + T17-a 六缺陷修复回归后 297 → r4（Stage 1 批次 A）304 → r5（批次 B TD-002 符号值 + 标准库最小集）324 → r6（批次 B 收官 B3 自举 Reader）356 → r7（批次 C 类型检查器 + 编译缓存 + TD-016）408 → **r8（批次 D 能力 I/O + 内部效应 + 用例运行器）476** → **r10（架构合规审计 +4）480**：+24 capability_tests（require 声明面 + E0006 三路径门控 + 豁免/形状/令牌 + EOF 子进程探针）+ +18 test_runner_tests（前置切分 + PASS 判定 + 短路/恢复/隔离 + front 错误面）+ +25 kerf-driver 单元（effects.rs 12：逃逸层/最近匹配/载荷保真/穿透契约 + capability.rs 13：R9 验证/豁免/编组）+ +1 negative_vm_tests（read_line_arity 自 ignore 激活——**FS-4 修复**，能力参数化重写时补齐元数校验）→ **r12（接口预留完整性扩展 +20）500**：+8 单元（reserved/ 模块 Probe 冻结——「测试实现体编译通过 = 契约冻结」先例沿用）+ +12 集成 reserved_ext_tests（P0 数据结构位置六项断言 + 预留 API 跨 crate 可达 + 负向形状：空目标拒绝/空片段类型检查失败/rename 错误面）→ **r13（批次 E·TD-004 作用域集解析收口 +9）509**：+9 集成 scope_set_tests（双路径语义锚点 + 作用域不匹配负例——见 docs/tests/v0/stage1/plan/scope-set.md）→ **r14（批次 E·E1-α 自举 Expander +19）528**：+19 集成 bootstrap_expander_tests（expander.krf 与 Rust 种子 parity——结构/Span/作用域集/param_scopes/错误消息逐字一致 + 行为面端到端；见 docs/tests/v0/stage1/plan/bootstrap-expander.md）→ **r15（批次 E·E1-β 宏收口 + 生产切换 + TD-021 prelude +25）553**：+17 宏 parity（镜像 macro_sys.rs：变换器注册表单表语义/卫生基名回退/省略号/字面量/Span 并集代次守卫（expansion_id 镜像——节点第 4 字段 + retag +1）/深度上限 500 消息逐字）+ +7 prelude_tests（TD-021 模块/import 承载——forms 级合并注入单一编译单元）+ +1 单元生产切换守护（compile_front 展开段经 bootstrap_expander——独立线程活性探针实测）。**全套件经自举 Reader + 自举 Expander（均 kerf 源码，VM 上运行）执行——生产管线读+展开两段全自举（E1-β）。**

> **r7 计数修正**（r8 对账发现，§8.4.5 规则 2——以实测为准）：r7 版本矩阵的分套件表存在陈旧数（头部「集成 173 函数」为 r3 时代口径；单元表 130 实为 150——driver 14→25 / expander 26→28 / compiler 12→15 的 r4-r7 增长未回写；cache_tests 13 实为 14；negative_vm 29 为排除 ignore 的口径）。r7 实际 = 150 单元 + 260 集成函数（259 通过 + 1 ignore）= 408:0:1 ✓（总量正确、分项陈旧）。r8 起全部逐二进制实测。
>
> r8 起 `#[ignore]` 清零：FS-4（read-line 元数）随批次 D 能力参数化重写修复激活；
> r3 内已修复激活（原 4 个 ignore → 1）：FS-1 嵌套守卫（10_000→256）、
> D3 eval 深度上限（MAX_EVAL_DEPTH=256 结构化）、D4 mod/除 i64::MIN
> 溢出（checked_div/rem）。

## 分套件统计（2026-09-10 r8 实测）

### 单元测试（202，crates 内联——r12 +8：reserved/ Probe 冻结；r17 +13：backend 9 + expander 4；r19 +4：capability_model Probe）

| 套件 | 层级 | 文件/位置 | 测试数 |
|------|------|----------|--------|
| kerf-span 单元 | crate 内联 | crates/kerf-span/src/*.rs | 11 |
| kerf-syntax 单元 | crate 内联 | crates/kerf-syntax/src/*.rs | 11 |
| kerf-core 单元 | crate 内联 | crates/kerf-core/src/*.rs | 10 |
| kerf-reader 单元 | crate 内联 | crates/kerf-reader/src/*.rs | 23 |
| kerf-expander 单元 | crate 内联 | crates/kerf-expander/src/*.rs | **32（r17 +4：recover.rs 形式级恢复/上限/次序/干净路径）** |
| kerf-compiler 单元 | crate 内联 | crates/kerf-compiler/src/*.rs | 15 |
| kerf-runtime 单元 | crate 内联 | crates/kerf-runtime/src/*.rs | 9 |
| kerf-vm 单元 | crate 内联 | crates/kerf-vm/src/*.rs | 18 |
| kerf-driver 单元 | crate 内联 | crates/kerf-driver/src/*.rs | **58（r8 +25：effects.rs 12 + capability.rs 13；r12 +8：reserved/ Probe 冻结；r17：codegen 契约 2 迁移 kerf-backend + 兼容锚 1——净效应由逐二进制实测吸收）** |
| kerf-backend 单元 | crate 内联 | crates/kerf-backend/src/*.rs | **9（r17 新 crate：codegen 契约迁移 3 + qbe 2 + aot 4）** |

### 集成测试（463 函数，tests/ 阶段树——r9 起经 runner.rs 单一总入口组织；r10 +4 审计；r12 +12 预留扩展；r13 +9 作用域集锚点；r14 +19 自举 Expander parity；r22 +8：bootstrap_compiler 扩展组）

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
| cache_tests（r7；r22 对账实测 13——r7 版 14 口径漂移修正） | tests/v0/stage1/plan/cache_tests.rs | 13 |
| **capability_tests（r8，批次 D）** | tests/v0/stage1/plan/capability_tests.rs | **24** |
| **test_runner_tests（r8，批次 D）** | tests/v0/stage1/plan/test_runner_tests.rs | **18** |
| **reserved_ext_tests（r12，预留扩展）** | tests/v0/stage1/plan/reserved_ext_tests.rs | **12**（P0 位置断言 4 + API 可达 1 + 形状行为 7——含负向：空目标拒绝/空片段类型检查失败/rename 错误面） |
| **scope_set_tests（r13，批次 E·TD-004）** | tests/v0/stage1/plan/scope_set_tests.rs | **9**（双路径正例 5：shadowing/嵌套 shadowing/闭包捕获/set! 词法命中/子集对照；负例 4：作用域不匹配未绑定 VM/eval/set! 三锚 + 宏引入不捕获） |
| **bootstrap_expander_tests（r14/r15，批次 E·E1-α + E1-β 宏收口）** | tests/v0/stage1/plan/bootstrap_expander_tests.rs | **36**（r14 E1-α：parity 正例 8 + 负例 6 + 边界 1 + 行为面 3；r15 +17 宏 parity：define-syntax/syntax-rules 全模式面/卫生 α/省略号零-多段-复合/字面量/多子句/糖覆盖/深度上限消息逐字/向量模式 + retag 代次守卫） |
| **prelude_tests（r15，批次 E·TD-021）** | tests/v0/stage1/plan/prelude_tests.rs | **7**（hofs 用户面可见/组合管道 filter→map→foldl=50/for-each 副作用/双路径一致/opt-in 负例/名字捕获显式失败/未知导入） |
| **qbe_backend_tests（r17，批次 G·G1）** | tests/v0/stage2/plan/qbe_backend_tests.rs | **24**（端到端 6 + 结构 4 + 一致性 6 + 负例 10——PoC 边界：lambda 值位/define 非 lambda/Str/Float/set!/module/未定义/arity/自由变量/IO/函数值） |
| **multi_error_recovery_tests（r17，批次 G·G2）** | tests/v0/stage2/plan/multi_error_recovery_tests.rs | **16**（种子恢复 6 + driver 8 + 双路径同构 1 + 执行路径不变 1——TD-013 双路径恢复 + 合并报告） |
| **tco_tests（r18，批次 H·H2）** | tests/v0/stage2/plan/tco_tests.rs | **12**（TCO 正例 8（105_001 恒定帧/相互尾递归/if 两臂/begin 末项三层/let 糖/内建隐式 RET/闭包值尾位/自举 10_000 深度链）+ 负例 4（指令预算护栏/尾调用 arity/非可调用/非尾深递归仍帧上限）） |
| **hm_inference_tests（r18，批次 H·H4）** | tests/v0/stage2/plan/hm_inference_tests.rs | **15**（双门：超集门 29 程序 + 零误报门（examples 六件套 + 动态边界）+ 四类缺口检出 + occurs/值限制/let-letrec 泛化/多错误 Span 序/Dynamic 逃生舱/512 预算） |
| **bootstrap_compiler_tests（r21/r22，批次 I·I1 自举 Compiler）** | tests/v0/stage2/plan/bootstrap_compiler_tests.rs | **27**（r21 门 A 基础组：bytecode_equal 全结构 parity 13 + 负例 2（define 位置 D1 逐字）+ 行为面 4；r22 扩展组：module/require 两臂正例组 + 糖九件全管线 46 case + 宏 3 + prelude 注入序 2（preamble.krf 真实语料）+ examples 六件双路径 + 行为面糖/module 2 组——42-b 边界负例改写正例（两臂迁移落地）） |

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
