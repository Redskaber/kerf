# 综合技术债登记册

> **Author**: kerf-dev-agent（ARCH-A 角色）
> **Date**: 2026-09-11（r17：TD-013 resolved（双路径恢复 + 合并报告 + CLI 切换）+ TD-024 新增（本地码 PoC 边界 B1）；r16：批次 F 深审全量对账——索引表补全（r3 后新增 TD-015+ 此前无索引行）+ TD-012 标记 resolved / TD-013 与 TD-009/010/014/017/018 目标时机改判 Stage 2（附 Stage 1 门放行裁定）/ TD-019/020 断档登记 / TD-023 新增（P2）；r3：TD-001/006 断档记录 + TD-005/008/011 详情补齐 + TD-012/013/014 新增 + TD-009 注记更新）
> **Version**: v0.3.0-r19
> **Status**: Active（r19 批间插入轮：无新增/无 resolved——设计轮 + 骨架冻结零债务面；口径注记：矩阵 r18 版集成计数 438 勘误为 436（198+438=636≠634 内部矛盾——实测 436 与 r17 增量链 409+27 吻合））
> **规则**: sop.md §6.2.1——新增已解决项/调整剩余项优先级（每子阶段必检）

## 索引

| TD ID | 标题 | 等级 | 状态 | 目标阶段 |
|-------|------|------|------|---------|
| TD-001 | （编号断档——不可考） | — | 断档存档 | — |
| TD-002 | quote 符号/向量值类型缺失 | P3 | **已解决（符号 r5；向量开放→Stage 2）** | Stage 1 |
| TD-003 | 图 IR 复合节点 CSE 共享 | P3 | 开放 | Stage 2 |
| TD-004 | 作用域集解析（Racket 式）替换名称基解析 | P2 | **已解决（r13——编译器/eval 双路径 (name, scopes ⊆) + max-cardinality；锚点 9 测试）** | Stage 1 |
| TD-005 | syntax-parse 级宏组合 | P3 | 开放 | Stage 2 |
| TD-006 | （编号断档——不可考） | — | 断档存档 | — |
| TD-007 | 迭代式展开工作表（深度上限解除：128→500→**10_000**） | P2 | **已解决（r18 / 40-c——完整口径）**：Stx Rc 共享化（List/Vector → `Rc<Vec<Stx>>` clone O(1)）+ retag 迭代式重建（显式工作表后序——栈深恒定）+ 均匀标记 O(1) 共享（链 N 步总工作量 O(N)，旧 O(N²)）+ 扁平 Drop（唯一持有脊柱工作表拆除）；上限 500→10_000（种子 + 自举 expander.krf 同步）；10_000 链测试 0.02s 通过 + 10_001 边界报错 | ~~Stage 2 批次 H2~~ 已交付 |
| TD-008 | 分代 GC / 堆压缩 | P3 | 开放 | Stage 2 批次 I2 |
| TD-009 | eval 路径 GC 根集枚举 | P3 | 开放 | **Stage 2**（r16 改判：原「Stage 1」未落地；eval 重写（12 §2.5.1）同轮评估） |
| TD-010 | 闭包/内置函数装箱（pair 元素） | P3 | 开放 | **Stage 2**（r16 改判：原「Stage 1」未落地；HeapObj::Foreign 承载） |
| TD-011 | 字符串全序比较 | P3 | 开放 | Stage 2 |
| TD-012 | expander.rs 单文件拆分候选 | P3 | **已解决（批次 B 前端重写落地六文件）** | ~~Stage 1（切换期）~~ |
| TD-013 | 多错误收集 / Expander 恢复展开（单错误短路） | — | **resolved（r17 / 38-e）**：种子+自举桥双路径形式级恢复 + DiagCollector（128 上限/截断/位置序）+ check_source_recover 合并报告（E0002+E0005）+ CLI check 切换恢复模式；16 集成测试全过 | ~~Stage 2 批次 G2~~ 已交付 |
| TD-014 | 展开期错误消息归因失真（嵌套 define） | P3 | 开放 | **Stage 2**（r16 改判：消息质量批与 TD-018 同批） |
| TD-015 | IrGraph 无条件计算旁路丢弃 | P3 | 开放 | Stage 2（切换期重构） |
| TD-016 | 链式比较短路语义静态收紧 | P3 | **已解决（r7）** | ~~Stage 1~~ |
| TD-017 | eval 参考路径深度上限不对称 | P3 | **裁定维持 256 边界（r18 / 40-c）**：eval = 参考路径（Stage 2 I1 编译器替换将退役——07 §3.2 混合期）；大栈线程化为将退役路径加复杂度不成立。TCO 后新不对称注记：VM 尾递归超 256 深度域在 T1 检查域外（tco_tests 头注口径） | Stage 2（I1 eval 重写时随迁评估） |
| TD-018 | 双路径错误消息文本分裂 | P3 | 开放 | **Stage 2**（r16 改判：消息质量批与 TD-014 同批） |
| TD-019 | （编号断档——不可考，r16 登记） | — | 断档存档 | — |
| TD-020 | （编号断档——不可考，r16 登记） | — | 断档存档 | — |
| TD-021 | 高阶函数用户面注入缺载体 | P3 | **已解决（r15——kerf-prelude 模块/import 承载）** | ~~Stage 1 批次 E~~ |
| TD-022 | 自举 Reader/Expander 帧消耗 O(源字符数) | P3 | **已解决（r18 / 40-c——TCO 兑现）**：VM 尾调用优化（Op::TailCall 帧复用 + 编译器尾位穿线（Lambda 体/If 两臂/Begin 末项）+ 内建尾调用隐式 RET + 指令预算护栏 10^9 兜底无限尾循环）；实证：127,780B 源（>10^5 字符边界）自举管线完整通过 + 自举 expander 10_000 深度链端到端（TD-007/TD-022 耦合解除） | ~~Stage 2 批次 H2~~ 已交付 |
| TD-023 | gc_stress 深递归 GC 根扫描回归（超线性） | P3（r18 降级） | **开放（重定型）**：r18/40-c TCO 副作用实测——gc_stress 61-62ms × 5 轮稳定（r16 回归值 207-235ms → **-70%**；Stage 0 基线 160.4ms → -62%）：spin 尾递归经帧复用后深帧根扫描压力消失，**原基准不再复现回归**（P2 证据基础失效——ARCH-A 降级 P3）；残留：非尾形深递归 + GC 的根扫描 per-cycle HashSet 分配模式未测量（需新建非尾形基准锚定） | **Stage 2 批次 I2**（与 TD-008 同轮；基准重定型先行） |
| TD-024 | 本地码后端 PoC 边界（整数域十二原语 + 直接调用；闭包/Float/Str/Pair/set!/module/IO/函数值一等边界外） | P3 | **登记（r17 / 38-b·38-c）**：显式错误非静默降级（B1 类）；FFI 面（print/write_stdout）按 ffi-ownership-model 批次 I 做实；闭包/GC 协同批次 H/I | Stage 2 批次 H-I（GC-后端协同轮） |
| TD-025 | 自举侧 retag 无均匀标记快路径（包装宏链 O(N²) VM 工作） | P3 | **resolved（r18 / 40-f）**：krf 六头字段协议落地——`(tag s e exp scopes uni . fields)`（`uni = ('uni . scopes)` 均匀证书，make-node 默认 nil + retag 重建置位 + inject-scope 注入清除 + 桥 stx_to_node 同步）；retag-scope 快路径（uni 命中 → 整棵子树 O(1) 共享——种子 uniform_tag 镜像）；实测门审计 C02 包装链 **>540s → 7.37s（73×+）**；双审计集 EXIT 0（stage0 41 + stage1 51，20s/28s） | ~~Stage 3~~ 已交付 |

## 详情

### TD-001 / TD-006 编号断档记录（r3 登记）

**编号断档：早期开发轮次消化，具体去向不可考——登记于 r3。** 考证过程（2026-09-10）：
(1) 全库 git 历史（4 commits）与 worklog 全文检索「TD-001」「TD-006」零命中——两个编号
**从未被登记过任何内容**；(2) 登记册于 worklog Task 6（工程文档树）创建时即自 TD-002
起编（TD-005/TD-008/TD-011 同批仅索引行），推断为创建时的**编号跳号笔误**而非
「已解决项消失」——不存在应留而未留的解决痕迹。**处置**：两个编号永久保留为断档占位
（禁止复用），以满足 §6.2 规则 2「大阶段末全审须闭环『已解决项确实消失』」的审计要求；
后续新增债一律从 TD-015 起编。

### TD-002 quote 符号/向量值类型缺失（r5 符号部分解决）
- **描述**：`LiteralValue` 无 Symbol/Vector 变体——`(quote sym)` 显式报错
- **根因**：Stage 0 值模型最小化（§3.2 literal_value 定义如此）
- **修复方案**：Stage 1 增加 `Value::Symbol`；quote 符号 datum → 符号值
- **r5 执行注记**（2026-09-10，批次 B Task 22-a）：**符号部分已解决**——
  全链落地：`LiteralValue::Symbol(Rc<str>)`（存剥离卫生后缀的基名，
  Racket 语义近似——与 resolve_hygiene_fallbacks 同一剥离口径）/
  `Value::Symbol` + eq? 按名相等 + `BcConst::SymLit`（与全局名索引
  `Symbol` 变体严格区分，§11 接口隔离）/ `HeapObj::Symbol`（符号入序对）
  / `ValueSlot::Symbol`（car/cdr 往返）；双路径（VM const_to_value +
  eval eval_literal）同步覆盖；宏模板内 quote 符号的卫生后缀在 datum
  层剥离（实测验证）。**向量部分仍开放**（显式报错，后续阶段）；
  正负测试 +5 函数（expander 2 / vm 3）+ 负例 6 case（符号值语义误用
  ——算术/条件/序对/比较位置），实跑校准消息。
- **影响范围**：expander::datum_to_value / vm::value / runtime::heap /
  compiler::bytecode（r5 实际触点比登记时扩大——含堆装箱三处）
- **代码锚**：kerf-core（literal_value 定义）；负测锚点：negative_expander_tests::quote_misuse（向量 case 保留）
- **优先级依据**：符号值影响宏编程体验（P3——不影响语义验证闭环）

### TD-003 图 IR 复合节点 CSE 共享
- **描述**：字面量/变量节点按结构键共享；复合节点不做激进 CSE
- **根因**：§21.11 风险表既定缓解（图 IR 过于复杂 → 树承载 + Stage 2 迁移）
- **修复方案**：Stage 2 哈希cons 复合节点 + SSA 化
- **workaround**：字面量共享已满足「公共子表达式」演示与验收

### TD-004 作用域集解析
- **状态（r13，批次 E 首个 MUV）**：**已解决**——展开器绑定形式 fresh scope 深注入（`Stx::add_scope_to_all`）+ `CoreExpr::VarRef/SetBang` 携带引用作用域集 + `Lambda.param_scopes` 携带绑定作用域集 + 编译器 `resolve_var` 与 eval `Env::lookup/set` 双路径切换为 `(name, scopes ⊆)` 子集匹配 + max-cardinality；空作用域集全局绑定 ⊆ 任意引用集——全局/内置天然兑底，`$hyg$` 基名回退（driver 双侧）降级为回退路径；α 重命名保留为第二道卫生保险（作用域注入不变式下两层同解）。锚点 = tests/v0/stage1/plan/scope_set_tests.rs（9 测试：双路径正例 5 + 作用域不匹配负例 3 + 宏引入不捕获 1）；500 基线零回归（r13 实测 509:0:0）。
- **描述**：标识符解析为名称基（编译期 slot 查找 + 运行期全局名）；
  Racket 式 scope-set 子集匹配未实现（ScopeSet 已在数据结构中全程携带）
- **根因**：名称基 + 一致性卫生重命名已满足 P1 卫生保证；
  scope-set 解析是 Stage 1 语言级宏的前置
- **修复方案**：编译器 resolve 按 (name, scopes ⊆) 匹配绑定（✅ r13 落地）
- **卫生回退**：$hyg$ 后缀剥离（driver::resolve_hygiene_fallbacks +
  **driver::resolve_eval_hygiene_fallbacks**——r3 已接线 eval 路径，双路径镜像）为
  名称基解析的显式近似
- **测试锚点**：negative_semantics_tests::t1_regression_hygiene_fallback_dual_path（3 case）

### TD-005 syntax-parse 级宏组合（r3 补详情）
- **描述**：syntax-rules 为骨架子集——**单层省略号边界**；点对模式尾部、尾省略号 + 固定尾部、
  展开转义 `(… template)`、嵌套省略号（多维笛卡尔展开）未实现（03 §2.3 v5.2 Stage 0 裁定注记）
- **根因**：宏组合表达力非 Stage 0 语义验证闭环的必要项（deep-review R1 偏差 #1 的
  「实现合理（Stage 0 骨架）」裁定）
- **修复方案**：Stage 2 syntax-parse 类结构化宏 DSL + 文法全量实现
- **代码锚**：kerf-expander/src/macro_sys.rs:4（显式推迟注记）
- **workaround**：单层省略号 + Rust 内置变换器（§3.2 糖推导全表）覆盖 Stage 0 全部需求
- **测试锚点**：negative_expander_tests::syntax_rules_misuse / macro_expansion_failures

### TD-007 迭代式展开（r4 部分解决注记）
- **描述**：展开深度上限 128（rustc 默认对齐）；文档示例 10_000 需迭代式
- **修复方案**：展开工作表化（显式队列替代递归下降）
- **测试锚点**：negative_expander_tests 宏深度超限负例（超限报错而非栈溢出）
- **r4 执行注记**（Stage 1 批次 A3，2026-09-10）：trampoline 工作表已落地
  （expand_form 顶层循环——宏产物头部仍是宏调用时迭代继续，展开控制流
  栈深与链长解耦）；上限 128→500（实测标定：2MiB 测试线程 1_000 通过/
  2_000 溢出，500 = 2× 裕度——TD-017 同型实测法）。**残留**：完整 10_000
  口径受 Stx 值语义深树的 clone/drop 递归约束（数据结构层）——批次 B
  前端重写时 Rc 化解除；探针实测记录：8MiB 主线程 4_000 通过/5_000 溢出

### TD-007 迭代式展开（r4 部分解决注记；r18 resolved）

- **r18 收口注记（批次 H / Task 40-c）**：完整 10_000 口径交付——
  ① `StxDatum::List/Vector` → `Rc<Vec<Stx>>`（clone O(1) 浅共享）；
  ② `retag_scopes` 迭代式重建（显式工作表后序 Visit/Assemble——
  Rust 栈深恒定，旧递归版 10_000 层链溢出）；③ **均匀标记**（`Stx
  .uniform_tag: Option<ScopeSet>`——retag 输出子树均匀作用域标记 +
  `add_scope_to_all` 注入时清除保健全性；链 N 步总工作量 O(N)，旧
  每步全树重建 O(N²)）；④ 扁平 Drop（`impl Drop for Stx`——唯一
  持有脊柱工作表拆除，共享子树计数递减天然无递归）；⑤ 上限
  500→10_000（种子 expander.rs + 自举 expander.krf `MAX-EXP-DEPTH`
  同步——parity 消息一致）；⑥ 均匀标记不参与 PartialEq（手写实现
  忽略性能字段）。正例 10_000 链 0.02s（2MiB 测试线程——恒定栈深）
  + 边界 10_001 报错含头含尾。
- 耦合解除实证：自举 expander 路径 10_000 深度链此前先撞 VM 帧上限
  （expander.krf trampoline 尾调用链耗帧）——TCO（TD-022 同轮）后
  端到端到达深度上限结构化报错（tco_tests 锚定）。

- **描述**：展开深度上限 128（rustc 默认对齐）；文档示例 10_000 需迭代式
- **修复方案**：展开工作表化（显式队列替代递归下降）
- **测试锚点**：negative_expander_tests 宏深度超限负例（超限报错而非栈溢出）
- **r4 执行注记**（Stage 1 批次 A3，2026-09-10）：trampoline 工作表已落地
  （expand_form 顶层循环——宏产物头部仍是宏调用时迭代继续，展开控制流
  栈深与链长解耦）；上限 128→500（实测标定：2MiB 测试线程 1_000 通过/
  2_000 溢出，500 = 2× 裕度——TD-017 同型实测法）。**残留**：完整 10_000
  口径受 Stx 值语义深树的 clone/drop 递归约束（数据结构层）——批次 B
  前端重写时 Rc 化解除；探针实测记录：8MiB 主线程 4_000 通过/5_000 溢出

### TD-008 分代 GC / 堆压缩（r3 补详情）
- **描述**：slot-Vec + 空闲表（free 表）方案**不移动对象、不压缩**——长运行程序的堆碎片
  治理缺位（05 §4 v5.2 对齐：清除阶段全量重建空闲表，句柄稳定但空间不回收整理）
- **根因**：Stage 0 程序短生命周期 + mark-sweep 最小复杂度裁定（避免分代/增量/并发的复杂度）
- **修复方案**：Stage 2 分代 GC / 堆压缩（05 §4 陷阱 3 的既定推迟路径）
- **代码锚**：kerf-runtime/src/lib.rs:19（推迟注记）
- **workaround**：Stage 0 测试与示例程序分配量有界（gc_tests 10^6 / gc_stress 3×10^5 均堆有界）

### TD-009 eval 路径 GC 根集（r3 注记更新）
- **描述**：元循环求值器关闭 GC 触发（根集枚举需遍历 Rc 环境链）
- **修复方案**：Stage 1 根集遍历或（按 §21.9 演进矩阵）eval 被编译器替换
- **代码锚**：driver.rs（eval 路径 `heap.set_gc_enabled(false)`）+ eval.rs:8
- **r3 关联注记**：eval 路径的**卫生回退解析已接线**（driver::resolve_eval_hygiene_fallbacks，
  Task 16 修复双路径全局引用分裂，TD-004 的 eval 侧镜像）——但该路径 **GC 仍禁用**：
  卫生回退是符号解析层修复，与根集枚举（Rc 环境链/闭包可达图遍历）**无耦合**，属两个
  独立边界；GC 触发仍仅由 VM 路径承担（安全点轮询 + 五来源根集），L-GC 不可观测性不受影响

### TD-010 闭包装箱
- **描述**：序对元素为闭包/内置时以标记字符串占位（不可达路径）
- **修复方案**：Stage 1 HeapObj::Foreign(Rc<dyn Any>)
- **代码锚**：heap.rs（BoxedInput 显式限制）/ vm.rs:626（占位注记）；05 §3.1 v5.2（HeapObj 六变体对齐——早期文档误写 Boxed 变体的更正出处）

### TD-011 字符串全序比较（r3 补详情）
- **描述**：比较操作符（`<` `<=` `>` `>=`）仅支持数值塔——字符串操作数报
  「字符串仅支持 = 比较」（`= `可判等字符串；`eq?` 即时值按值/堆值按引用）
- **根因**：字符串全序语义（Unicode 码点序 vs 本地化序）裁定推迟——Stage 0 避免隐式语义承诺
- **修复方案**：Stage 2 实现字符串序比较（数值塔之外的比较路径分流）
- **代码锚**：kerf-driver/src/builtins.rs:330（报错消息注记）
- **测试锚点**：negative_vm_tests::comparison_string_ordering_rejected（4 case 断言报错形态）

### TD-012 expander.rs 单文件拆分候选（r3 新增；r16 标记已解决）

- **r16 收口注记（批次 F 深审 D2 全审发现）**：拆分已随**批次 B 前端重写**
  实际落地——kerf-expander 现六文件（expander.rs 632 / core_forms.rs 604 /
  macro_sys.rs 715 / sugar.rs / phase.rs / lib.rs），原「单文件 1350 行」不复
  存在；登记册状态列此前未同步（深审发现项）。
- **描述**：kerf-expander 2276 LOC 占 crates 22.2%，其中 expander.rs 单文件 1350 行
  （deep-review R1 D1 风险项②）——职责仍单一（9 核心形式 + 糖推导 + 相位驱动）但认知负担
  已达拆分阈值
- **等级**：P3（非性能、非语义——纯可维护性）
- **目标时机**：Stage 1 切换期（前端重写时顺带拆分：核心形式/糖推导/相位驱动三模块）；
  拆分前禁止向该文件继续新增职责（防 1500+ 失控）
- **关联**：worklog Task 14-a（D1 架构审查）；TD-007（迭代式展开重写时自然触碰）

### TD-013 多错误收集 / Expander 恢复展开（r3 新增；r7 设计完成；r16 改判；**r17 resolved / 38-e**）

- **r17 交付注记（批次 G / Task 38-e）**：双路径实现全落地——种子
  `kerf-expander/src/recover.rs`（DiagCollector：push / is_full(128) /
  mark_truncated / into_sorted 位置序 + expand_program_recover 形式级
  恢复）+ 自举桥 `bootstrap_expander::expand_program_recover`（逐形式
  单元素列表调用——expander.krf 协议零改动）+ driver
  `check_source_recover`（front_from_core 抽段共享——E0002/E0005
  全量合并 + 截断尾注）+ CLI `kerf check` 切换恢复模式（单错误短路
  保留库 API check_source；run/eval 执行路径不变——r7 设计 §5）。
  **r7 验收 5 条全过**：恢复跳过（产物含后续 define，指令数 = 无错
  对照一致）/ 上限 128 + 截断提示 / 位置序 / 既有零破坏 / 16 case。
  实测勘误 2 项如实记录：截断标记 break 路径须显式置位（mark_truncated）；
  并行测试 env 竞态（find_qbe 纯函数注入式重构）。

- **r16 改判注记（批次 F 深审 D2 发现）**：原「实现绑定批次 E」未纳入
  E1-α（自举 Expander 核心形式）/E1-β（宏收口 + 生产切换）范围——批次 E
  已收口（r15 门审 APPROVED）而展开段仍单错误短路。**Stage 1 门放行裁定**：
  诊断吞吐量面（P2）非语义正确性阻塞，按「P2 开放不阻塞 Stage 2 启动」放行
  （34-d 投票记录口径）。改判 **Stage 2 批次 G2**（HM 推断设计轮同批——
  错误恢复与推断升级同为前端诊断主线）；设计文档 multi-error-recovery-
  design.md 继续有效，`kerf check` 首个消费面（r7）已在。
- **描述**：stage0.md §8.7 / 02 §5 承诺「单次运行报告多个错误 + Expander 错误恢复后继续
  展开后续形式（IDE 增量反馈）」——实现为**单错误短路**（首个错误终止管线，后续形式不再
  展开/求值）。deep-review R1 存档确认，negative_semantics_tests::error_recovery_* 锚定
  当前事实行为（4 case）
- **等级**：P2（设计承诺与实现存在可观测差距——诊断吞吐量面）
- **r7 设计批交付注记**（批次 C Task 25-d，2026-09-10）：设计已冻结于
  [multi-error-recovery-design.md](./v0/stage-1/multi-error-recovery-design.md)——
  恢复粒度=形式级（表达式级不恢复：半展开状态重建成本不成比例）；恢复机制=
  编译期控制流（**非** effect——§11 接口隔离，效应联动裁定已归档）；收集上限
  128 + 截断提示；输出次序按 Span（与 check_program 一致）。**首个消费面已实证**：
  `kerf check`（r7）静态检查多错误全量收集（typecheck_tests::multi_error_collection_span_order
  锚定三错误全量 + Span 次序）。实现绑定批次 E（Expander kerf 重写同批——避免双恢复机制）
- **目标时机**：Stage 1 批次 E（验收标准 5 项见设计文档 §6）
- **代码锚**：driver.rs（管线短路返回）；negative_semantics_tests.rs:433（存档注记）；
  kerf-compiler/src/typecheck.rs（多错误收集的实证消费面）

### TD-014 展开期错误消息归因失真（r3 新增）
- **描述**：嵌套 define 重复（`(define (f) (define x 1) (define x 2))` 类形态）在**展开期**
  被体内部提升机制拒绝，但消息为「lambda 参数重名」——**归因失真**（语义上等价 E6 的
  提前防御，消息误导排查方向）。同批存档：eval 路径错误诊断保真度弱于 VM（错误包装
  前缀「求值失败：」+ Span 指向差异——双路径消息分裂面，见 negative_semantics_tests 头注）
- **等级**：P3（消息质量——不影响 Err 事实与双路径一致性）
- **目标时机**：Stage 1（消息质量专项：展开期错误归因到「嵌套 define 重复」；eval 错误
  包装对齐 VM 诊断形状）
- **代码锚**：expander.rs（parse_params 重名检查——两路径共同上游）；06 §5.3 v5.2（错误路径
  互查口径补注）

### TD-015：IrGraph 无条件计算旁路丢弃（P3）

- **等级**：P3（不影响正确性，恒定开销）
- **状态**：开放
- **目标阶段**：Stage 1 切换期（与 TD-012 expander 拆分同批）
- **描述**：`compile_source` 在 run/eval 生产路径无条件计算 `IrGraph` 后
  旁路丢弃（仅 `kerf ir` 子命令与 CodeValue 检查消费）。发现于 §14.6.2
  重构最优性审查（O-1）。
- **偿还计划**：按消费方拆分编译入口（`compile_source_lowered` /
  `compile_source_fast`），或延迟到 IrGraph 消费点。

### TD-016：链式比较短路语义待静态收紧（P3）

- **等级**：P3（语义已显式裁定，行为稳定）
- **状态**：**已解决（r7，批次 C Task 25-c）**
- **目标阶段**：Stage 1（类型检查器联动）
- **描述**：比较链在首对判定终止时后续操作数不做类型检查
  （FS-5，09-stdlib §2 显式裁定段）。Stage 1 类型检查引入后统一为
  全操作数静态检查。
- **r7 偿还注记**：双侧落地——① 运行时面：`cmp_builtin` 前置全参数
  数值校验（全字符串+排序族 → TD-011 消息；其余首个非数值 →
  `{op} 需要数值`；既有两参消息逐条兼容，负例矩阵回归锚全绿）；
  ② 静态面：类型检查器 R3 规则（typecheck.rs Ordering/NumOrAllStr）
  同口径。语义文档 09-stdlib §2 v5.5 重写（FS-5 边界收敛）；负例
  +9 case（stdlib_tests comparison_chain_* 两函数）+ 静态面
  typecheck_tests R3 四函数。
- **偿还计划**：已交付（双侧 + 文档 + 测试）

### TD-017：eval 参考路径深度上限不对称（P3）

- **等级**：P3（已结构化+文档化，语义边界非缺陷）
- **状态**：开放
- **目标阶段**：Stage 1（判定是否大栈线程化）
- **描述**：eval 参考路径 `MAX_EVAL_DEPTH=256`（2 MiB 线程栈实测标定）
  vs VM 生产路径 `MAX_FRAMES=100_000`（D3 修复引入——修复前 eval 深递归
  为 Rust 栈溢出 abort）。T1 定理在深度 ≤ 256 的常规程序域成立。
- **偿还计划**：eval_source 在大栈专用线程（32 MiB）执行 + 上限对齐
  MAX_FRAMES；或维持参考路径边界声明。

### TD-018：双路径错误消息文本分裂（P3）

- **等级**：P3（Err 事实与阶段一致，仅文本不同）
- **状态**：开放
- **目标阶段**：Stage 1（消息质量批——与 TD-014 同批）
- **描述**：未绑定变量/if 非布尔等错误 VM 与 eval 文本不同（D8——
  T17-a 深挖发现）。E 码/阶段/Span 三要素一致，文本常量分散于
  vm.rs/eval.rs。
- **偿还计划**：共享消息常量模块（kerf-span 或 kerf-vm 公共层）。

### TD-021：高阶函数用户面注入缺载体（P3）

- **等级**：P3（hofs 已以 kerf 源码交付并直测——仅用户程序不可见）
- **状态**：**已解决（r15——模块/import 承载：kerf-prelude 模块
  （bootstrap/preamble.krf）经 forms 级合并注入单一编译单元；用户程序
  `(module 名 (import kerf-prelude) ...)` 声明后 map/filter/foldl/
  for-each 以普通全局函数可调用；同名 define 显式报「重复定义」；
  无 import 声明保持未绑定（opt-in）；测试 prelude_tests 7 case）**
- **目标阶段**：Stage 1 批次 E（已随 E1-β 交付）
- **描述**：map/filter/foldl/for-each 已实现于 reader.krf 序章（r6/B3，
  经 bootstrap 桥直测——「用 kerf 源码 preamble 实现」的自举验证命题
  本体已交付），但用户程序引用报未绑定变量。三方案已否决（r5 裁定）：
  P1 源码拼接（Span 诊断污染 = P1 缺陷）、P3 跨程序全局合并
  （SymbolTable id 不可比 + 原型索引程序局部——call_closure 运行时
  显式拒绝）、P5 builtin 调闭包（VM 递归 re-entry 越界 §11）。
- **偿还计划**：模块/import 机制承载 preamble（批次 E Expander 重写
  时设计）——正确 > 妥协：不做看起来像但不是的半吊子注入。
- **实现**（r15）：P1/P3/P5 全规避——preamble.krf 以独立 file_id 读入
  （Span 指向自身文件，无源码拼接污染）；forms 前置合并进同一编译
  单元（无跨程序合并）；零 VM 再入。注册表多模块按序 declare + 主
  模块 visit（import 边传递依赖——§8.9 单模块生命周期扩展）。

### TD-022：自举 Reader 帧消耗 O(源字符数)（P3）

- **等级**：P3（语言无 TCO 的既定边界；测试语料全部远低于上限）
- **状态**：开放
- **目标阶段**：Stage 2（TCO 决策点——12-roadmap §2.5）
- **描述**：reader.krf 的词法/语法主循环按 Token/字符递归（尾调用链
  形态），无 TCO 下每步消耗一 VM 帧（MAX_FRAMES=10^5）：约 10^5 字符
  以上源文件将以「调用帧超过上限」终止（非读语义错误）。种子 Reader
  无此限制（Rust 循环）。Stage 1 验收语料（≤ 数 KB）不受影响。
- **偿还计划**：Stage 2 TCO 裁定时一并评估（帧复用或迭代式驱动）；
  若 TCO 继续推迟，评估分块驱动协议（宿主侧步进）。

### TD-019 / TD-020 编号断档记录（r16 登记）

**编号断档：TD-018 与 TD-021 之间的两个编号从未被登记任何内容**（r16 批次
F 深审 D2 全审时全库 grep「TD-019」「TD-020」零命中——含 worklog/matrix/
lang-design 全部对账面）。推断与 TD-001/006 同型：跳号笔误而非已解决项消失
（TD-015~018 登记于深审修复内循环，TD-021 首现于 r12——两编号在其间跳过）。
**处置**：沿用 TD-001/006 先例——永久保留为断档占位（禁复用），后续新增债
从 TD-024 起编。

### TD-024 本地码后端 PoC 边界（P3，r17 新增——38-b/38-c B1 类登记）

- **描述**：QBE 后端 PoC（G1）支持面 = 整数域原语十二项（+ - * / mod =
  < > <= >= not eq?）+ 顶层 define 函数直接调用（静态 arity 校验）+ If
  （值/尾上下文——phi 合并）+ Begin；**边界外**（显式错误非静默降级）：
  lambda 值位置（闭包捕获）/ Float/Str/Symbol/Pair/Bool/Nil 字面量 /
  set! / module / print·IO 内置 / 未定义调用 / 自由变量 / 函数值一等传递
- **等级**：P3（PoC 范围裁定的显式声明——§21.3 条件 3 以 fib 端到端
  满足；边界扩张属既定路线非缺陷）
- **承接路线**：FFI 面（print/write_stdout）按 stage-2/ffi-ownership-
  model.md 批次 I 做实（38-a 交付）；闭包/GC-后端协同（根扫描本地码
  化 + pin 根集）批次 H/I；Float 域 QBE d 类型扩展随后
- **代码锚**：kerf-backend/src/anf.rs（lower 边界错误族）；tests/v0/
  stage2/plan/qbe_backend_tests.rs（负例 10 项锚定）

### TD-023 gc_stress 深递归 GC 根扫描回归（P2，r16 新增——批次 F 深审 D6 实测发现）

- **等级**：P2（性能回归超 §14.6.4 的 10% 阈值；不影响语义正确性——存活
  数据跨 GC 验证全过、L-GC 不可观测性不受影响）
- **状态**：开放
- **目标阶段**：**Stage 2 批次 I2**（与 TD-008 分代 GC / 堆压缩评估同轮——
  分代/压缩是对症路径）
- **描述**：gc_stress（3×10^5 分配 + 30k 深递归）单轮 207~235ms，较 Stage 0
  基线 160.4ms 回归 **+29%~46%**（超回归阈值）；且**超线性缩放**——spin
  15000 → 59.5ms、spin 30000 → ~215ms（2× 分配 → 3.6× 耗时）。冷却退避
  四参数（256/64/25%/1024）未变——Stage 1 新增变量为候选根因：① 根集遍历
  per-cycle `HashSet<usize>` 去重分配（vm.rs `collect_value_roots`——每
  GC 周期重建）；② `Value` 枚举宽度两轮增长（r5 Symbol + 闭包共享捕获）的
  栈/帧内存布局与缓存效应。
- **证据**：深审 D6 实测三组（207/214/235ms）+ 减半缩放实验；完整口径见
  [stage-1/performance-baseline.md](./stage-1/performance-baseline.md)（36-d 落位）
- **偿还计划**：批次 I2 评估三路径——分代 GC（年轻代扫描集缩小 = 对症）/
  根遍历 visited 结构无分配化（arena/bitset 复用）/ Value 瘦身（大变体
  Box 化）。在此之前每次合入按 §14.6.4 协议复测 gc_stress 5 轮（超 10%
  阈值保持追踪）。
