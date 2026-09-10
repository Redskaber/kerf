# 流水线路径覆盖（sop.md §9.5.1 三层覆盖记录 + §14.6.1.1 完整性审查）

> **Author**: kerf-dev-agent（QA-A 角色）
> **Date**: 2026-09-10（r3 负测扩张后全文重写——修正 r1 审查发现的「结论夸大 + 缺完整性小节」两项缺陷）
> **Version**: v0.1.0-r3
> **Status**: Active

## 1. 测试目标

记录编译流水线（read → expand → lower → compile → vm → runtime → driver）的**路径覆盖状态**，
作为外循环投票（§6.3）的数据源。计数基线：294 测试函数 / 290 通过 / 0 失败 / 4 忽略
（[matrix.md](./matrix.md)）；负向 case 口径见 §2 统计行。

## 2. 三层覆盖记录（§9.5.1 格式：Tier / 名称 / 覆盖阶段 / 预期输出 / 状态）

### Tier 1 —— 阶段内（单元测试，130 函数）

| ID | 名称 | 覆盖阶段 | 预期输出 | 状态 |
|----|------|---------|---------|------|
| T1-span | kerf-span 单测 | Span/诊断 | Span 派生/诊断渲染契约 | ✅ 11/11 |
| T1-syntax | kerf-syntax 单测 | 符号/作用域 | NFC 归一化/ScopeSet 运算 | ✅ 11/11 |
| T1-core | kerf-core 单测 | CoreExpr/图 IR/CodeValue | 9 原语构造/共享/良构性 | ✅ 10/10 |
| T1-reader | kerf-reader 单测 | 词法/语法 | Token 快照/括号三态 | ✅ 23/23 |
| T1-expander | kerf-expander 单测 | 展开/相位 | 核心形式/卫生/簿记（含循环依赖检测） | ✅ 26/26 |
| T1-compiler | kerf-compiler 单测 | 编译 | 栈平衡/回填/常量池/**40 操作码守护**/**App 函数先序** | ✅ 12/12 |
| T1-runtime | kerf-runtime 单测 | 堆/GC | 分配/回收/foreign 根 | ✅ 9/9 |
| T1-vm | kerf-vm 单测 | 执行 | 帧协议/错误形状 | ✅ 14/14 |
| T1-driver | kerf-driver 单测 | 管线编排 | 全管线/内置注册/预留冻结（Probe） | ✅ 14/14 |

### Tier 2 —— 阶段间（集成测试，164 函数 = 160 通过 + 4 忽略）

| ID | 名称 | 覆盖阶段 | 预期输出 | 状态 |
|----|------|---------|---------|------|
| T2-reader | reader_tests | read | Token 流无损 + 错误带 Span | ✅ 10/10 |
| T2-neg-reader | negative_reader_tests | read（负） | E0001 全错误族 + Span 精确断言 + 渲染形状 | ✅ 13+1 忽略（59 case） |
| T2-expander | expander_tests | read→expand | 展开产物/糖推导/卫生 | ✅ 16/16 |
| T2-neg-expander | negative_expander_tests | expand（负） | E0002 全族 + 空应用 + 关键字误用矩阵 + 循环依赖 | ✅ 23/23（96 case） |
| T2-compiler | compiler_tests | expand→compile | 确定性编译/字节码形状 | ✅ 10/10 |
| T2-vm | vm_tests | compile→vm | 执行语义 + 双路径互查（含 App 顺序回归） | ✅ 17/17 |
| T2-neg-vm | negative_vm_tests | vm（负） | 24 内置 × 类型/元数系统表 + E0004 断言 | ✅ 26+3 忽略（230 case） |
| T2-neg-sem | negative_semantics_tests | 全阶段语义（负） | E1–E6 矩阵 + T1 回归 + 消息形状 + 错误恢复 | ✅ 20/20（98 case） |
| T2-gc | gc_tests | runtime | 10^6 有界分配/环/深链/foreign | ✅ 6/6 |
| T2-pipeline | pipeline_tests | read→vm 全链 | E2E/双路径/确定性/Span 传播 | ✅ 11/11 |

### Tier 3 —— 全流程（阶段门审计）

| ID | 名称 | 覆盖阶段 | 预期输出 | 状态 |
|----|------|---------|---------|------|
| T3-gate | gate_review_r1 | §22.3 十二项里程碑 | 12 项可执行审计全绿 | ✅ 8/8 |
| T3-audit | examples/audit/stage0_gate_audit_r1.rs | §7.3.1 门审计集 | 41 case（负 32 + 恢复 6 + 正 3）退出码 0；§7.1.1 七类全覆盖 | ✅ 全 PASS |

### 正负比例统计（§9.5.1 要求的统计行）

- **函数口径**：294 测试函数 = 正向/混合 208 + 负向文件 86（negative_* 四文件）。
- **case 口径（权威）**：负向 case 483（四文件，按文件内注释汇总）+ 审计集负向 32 = **515**；
  正向 case ≈ 160 → **全局正负比 ≈ 1:3.2**（r1 审查时 1:0.24；§9.4.3 的 ≥1:3 门限达标）。
- 逐分类负测非零：read/expand/compile（间接）/vm/driver/gate 全覆盖；runtime（GC）与
  reserved 为不可构造类（见 §3 注记）。

## 3. 编译流水线路径覆盖矩阵（16 类——真实状态）

> r1 版本「16 类均有正负向覆盖」的结论**不实**（夸大）。下表为 r3 负测扩张后的真实状态：
> 正向 16/16 全覆盖；**负向 12/16 本轮补齐**；其余 4 类为编译器/回收器内部不变式或
> 不可观测语义，用户程序无法构造负例，以「不变式断言 + 文档化存档」为验证口径。

| 路径 | 正向 | 负向 | 状态与验证载体 |
|------|------|------|----------------|
| read：词法正确 | ✅ reader_tests（快照/无损/注释/Unicode） | — | 正向即可证 |
| read：词法错误 | — | ✅ 本轮补：negative_reader（59 case：未闭合字符串/非法转义/贪心数字/非法字符/块注释 + E0001 码直接断言 + 渲染形状） | 正负双覆盖 |
| read：语法错误 | — | ✅ 本轮补：negative_reader（括号三态 16 case + 引号 EOF + Span 精确断言） | 正负双覆盖 |
| expand：9 核心形式 | ✅ expander_tests / vm_tests | ✅ 本轮补：negative_expander 关键字误用矩阵（21 关键字/96 case） | 正负双覆盖 |
| expand：糖推导 | ✅ let/letrec/let*/cond/and/or/while（§3.2 全表） | ✅ 本轮补：negative_expander（let/letrec/let*/cond/else/derived_form 误用） | 正负双覆盖 |
| expand：宏卫生 | ✅ 引入重命名一致/用户名保留/自引用 | ✅ 本轮补：negative_semantics::t1_regression_hygiene_fallback_dual_path + e3_macro_introduced_unbound | 正负双覆盖 |
| expand：错误路径 | — | ✅ 本轮补：深度超限/模式不匹配/syntax-rules 误用/相位簿记违规/循环依赖（DFS 检测负例） | 正负双覆盖 |
| lower：图 IR | ✅ 共享节点/Span 元信息/roots（kerf-core 单测） | ⚠️ 不可构造 | IR 由编译器内部构造——损坏 IR 非用户程序可触发；不变式断言为验证口径（文档化存档） |
| compile：回填 | ✅ backpatch_leaves_no_placeholders（零占位断言） | ⚠️ 不可构造 | E0003 类编译器内部不变式（非用户可触发，negative_semantics 头注存档） |
| compile：常量池 | ✅ const_pool_dedup | ⚠️ 不可构造 | 同上 |
| compile：闭包捕获 | ✅ 捕获描述符/嵌套捕获 | ✅ 间接：negative_vm::lambda_arity + negative_semantics::e2（运行时侧）+ compile 单测字节码序断言 | 正负双覆盖（运行时侧） |
| vm：执行语义 | ✅ fib(25)/深递归/闭包计数器/高阶 | — | 正向即可证 |
| vm：错误路径 | — | ✅ 本轮补：negative_vm（230 case：24 内置 × 类型/元数系统表、div-zero、not-callable、帧上限；E0004 直接断言）。注：**栈下溢为 E8 内部不变式，非用户可触发**（r1 版「栈下溢 ✅」无对应测试，已更正为文档化存档） | 正负双覆盖 |
| runtime：GC | ✅ gc_tests（回收/存活保护/环/深链/foreign/统计） | ⚠️ 语义级不可构造 | GC 不可观测（[06 §4 L-GC](../lang-design/06-operational-semantics.md)）——无用户可触发负例；压力（10^6 有界）+ 双路径互查为验证口径 |
| driver：管线 | ✅ E2E/双路径互查/确定性/诊断渲染 | ✅ 本轮补：negative_* 四文件全部经 run_source/eval_source 驱动（Err 事实 + Stage + 消息形状断言） | 正负双覆盖 |
| reserved：冻结 | ✅ 四项 Probe 实现 + 令牌不可伪造 | ⚠️ 不可构造 | 接口冻结的验证载体是「测试实现体编译通过 = 契约冻结」（reserved.rs）；非运行时可触达路径 |

## 4. 完整性审查小节（§14.6.1.1）

> 数据实测于 2026-09-10（r3 修复轮后），方法：全库 `rg -c '_ =>'`（crates + 根 CLI）
> 与逐处人工核对。

- **catch-all（`_ =>`）总数：42 处**（crates 41 + 根 CLI 1）。逐文件计数：vm.rs 11 /
  value.rs 4 / parser.rs 4 / driver.rs 3 / heap.rs 3 / stx.rs 3 / lexer.rs 2 / symbol.rs 2 /
  expander.rs 2 / builtins.rs 2 / eval.rs 1 / compile.rs 1 / opcode.rs 1 / token.rs 1 /
  macro_sys.rs 1 / main.rs 1。
- **静默空臂数：1 处**——vm.rs `collect_value_roots` 的即时值臂（`_ => {}`）：r1 审查的
  唯一生产静默空臂，**r3 已补臂级注释**（「即时值（Unit/Nil/Bool/Int/Float/Str/Builtin）
  无堆子引用，无需入根集」）。
- **无臂级注释的其余臂**：均为显式回退/报错形态（`return None` / `Err(...)` / `false` /
  回退分支），臂表达式自文档化——r1 审查记录的 19 处无注释项中，空臂 1 处已修复，
  其余以回退形态归档（C1 卫生注记完成态）。
- **生产 `expect`：12 处**（r2 基线 11 + r3 循环依赖检测新增 1），全部携带不变式消息
  （「上方已检查」「调用方已校验头为符号」等形态）——零裸 expect。
- **TODO / FIXME：0**（全库复核）。
- **`//!` 模块头：37/37**（crates 36 + 根 main.rs 1）。

## 5. 依赖

- 上游：[matrix.md](./matrix.md)（分套件计数权威）、[negative-tests.md](./v0/stage0/plan/negative-tests.md)（负测文档锚点）
- 代码：tests/v0/stage0/{plan,gate} 双向印证；examples/audit/stage0_gate_audit_r1.rs（§7.3.1）
- 规范：[06-操作语义](../lang-design/06-operational-semantics.md)（E 码语义 / T1）、[11-测试基础设施](../lang-design/11-testing.md)
