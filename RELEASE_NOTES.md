# RELEASE_NOTES

## v0.1.0 —— Stage 0 语义验证（2026-09-09）

### 交付

- **12（10+2）能力模型全量落地**（P0/P1）：
  类型化 Token Reader / 图 IR（共享节点）/ 结构化 CodeValue /
  元循环求值器 / 基础闭包（共享单元格捕获）/ Span 全管线 /
  结构化诊断 / 最小 I/O / 相位分离 / 卫生宏（syntax-rules）/
  标记-清除 GC / 字节码 VM（39 操作码）
- **四项接口预留冻结**（P2/P3）：Effect Handlers / 多阶段编程 /
  能力模型 I/O / 编译缓存
- **9 crates Cargo Workspace**（零外部依赖）+ CLI（10 子命令）
- **200 项测试全绿**；§3.2 验收全绿（build/check/test/fmt/clippy）
- **文档**：lang-design 20 篇（stage0.md v5.0 拆分）+ develop/tests/graph 树

### 语义验证亮点

- fib(25) = 75025（VM 与 eval 双路径一致）
- 卫生宏：宏内外同名不串扰（一致性 α 重命名 + 自引用保留 + 关键字保留）
- letrec 互递归 / 闭包可变捕获（独立计数器）/ 深递归 10^4（迭代式帧栈）
- GC：3×10^5 临时分配堆有界；环回收；2×10^5 深链标记不爆栈

### 已知边界

TD-002~TD-011（P2/P3，显式推迟）——见
docs/develop/v0/tech-debt-register.md。无 P0/P1 遗留。

### 基准

- fib(25)（含编译）：84.7 ms/轮（release）
- GC 压力：0.72 s / 3×10^5 分配

### 下一步

Stage 1（自举验证）：目标语言子集重写编译器前端 + 类型检查器。

---

## v0.1.0-r2 —— lang-design v5.1 收敛 + 双路径语义分裂修复（2026-09-10）

### lang-design v5.1（设计文档收敛审查）

- **06-操作语义从 21 行空壳重写**（~200 行）：9 原语小步归约规则组 R1–R9
  （含 βv/δ 辅助规则）+ 错误吸收语义 E0–E8 + GC 不可观测性引理 L-GC +
  编译正确性定理 T1（L1–L4 三引理证明纲要 + 适用边界）——补齐
  stage0.md §13.4 的源文档理论欠账
- **03-宏系统 / 05-运行时契约回填**（从冻结实现）：
  Transformer/TransformerKind/ExpandCtxt/ModuleRegistry/syntax-rules 文法；
  Heap/GcRef/Slot/RootSet/I-O 通道契约
- 01–05/09–11 统一增补「处理程度（P0–P4）/所属 Stage/推迟项」标注 +
  「测试锚点」节（设计驱动测试锚定）；02 职责矛盾调和（词法层 vs
  Reader 模块）；04 操作码漂移注记 + CodeBuf/CaptureSource 契约；
  断链修正（19-参考文献 §3.5/§3.6）×3；术语计数 42；全库 0 断链

### 双路径语义分裂修复（依据 06 定理 T1 审查发现，CLI 实证三组反例）

- **新增第 40 号冻结契约 `DefineGlobal`**：define 与 set! 的全局存储
  语义分裂修复——`StoreGlobal` 收紧为 S1/E3（未绑定报错）；
  `DefineGlobal` 承载 D1/E6（同层重复定义报错）
- **Define 返回值统一**（T1 反例）：编译模式改为 `value; DUP;
  DefineGlobal`，与 eval 路径一致（此前 VM 返回 nil）
- **lambda 形参表重名展开期拒绝**（A3 卫式，两路径共同上游）
- 修正 2 处依赖旧错误语义的测试断言（去除兼容思维）
- **204 项测试全绿**（200 基线 + 4 组新语义双路径对账）；
  §3.2 验收全绿（build/check/test/fmt/clippy 零警告）

### 交付包

kerf-stage0-v0.1.0-stage0.1-12-caps-langdesign-v5.1-semantics-r2.tar.gz
（312 KB / 165 文件；包内解压自举验证 204 测试全绿）

---

## v0.1.0-r3 —— deep-review R1 修复 + 负测扩张 + 门审计集（2026-09-10）

### 深度审查修复（P1×4 清零）

- **App 求值顺序统一「函数先」**（偏差 #13，T1 反例面）：compile.rs/vm.rs 双侧对齐
  06 §1.3/§2 A1 契约；双路径错误排序负例回归（app_evaluates_fn_then_args /
  app_evaluation_order_fn_first_dual_path）
- **操作码冻结守护测试重写**（偏差 #7）：40 项显式枚举（八组：栈 7/变量访问 7/控制流 2/
  函数 3/算术比较 12/数据 3/谓词 5/终止 1）——enum ↔ 测试 ↔ 文档三方冻结
- **模块循环依赖检测**：phase.rs DFS 灰标记 → 结构化 Err「模块循环依赖：…」
  （菱形依赖合法；§7.1.1 类 6 补齐）
- **§7.3.1 门审计集就位**：examples/audit/stage0_gate_audit_r1.rs——41 case
  （负向 32 + 恢复 6 + 正向 3），§7.1.1 七类全覆盖；r1 的 gate PASS 判定缺陷闭环
- **驱动修复**：VM 堆栈追踪（run_program 最内 16 帧 note）；eval 路径卫生回退接线
  （与 VM 路径镜像，T1）；driver 公共 API dump_tokens/dump_stx（CLI 经 driver 转发，
  §14.7.2 B4 合规）

### 负向测试扩张（§9.4.3 1:3 门限达标）

- 四个表格驱动负测文件：negative_reader（60 case）/negative_expander（98）/
  negative_vm（231）/negative_semantics（100+）≈ **490 case**（r1 基线 37）

### T17-a 对抗深挖修复批（r3 末轮——§14.6.3 独立深挖发现）

- **D1** 函数体内 begin 包裹 define 全局泄漏（VM）vs 词法（eval）——T1 反例：
  编译期结构化拒绝（E0003，两路径共享 compile_source 双侧一致）
- **D2** `eq?` 字符串指针比较 → 内容比较（VM 常量池去重路径与 eval 分裂）
- **D3** eval 深递归栈溢出 abort → MAX_EVAL_DEPTH=256 结构化上限（实测标定）
- **D4** `i64::MIN /± -1` Rust panic → checked_div/rem 结构化错误
- **D5/D9** `(+)` 越界 panic → 单位元 0；`(*)` → 1；`(- x)` → 取负
- **D6** 宏调宏未绑定 → 展开器基名回退（卫生穿透落地）
- **D7** eval 错误逐层「求值失败：」前缀累积 + Span 丢失 → 保真透传
- FS-1 Reader 嵌套守卫失效（600 层即溢出）→ MAX_NESTING_DEPTH=256
- 全局正负比 1:0.24 → **≈1:3.2**（负 515 vs 正 ~160，case 口径）；
  E1–E6 + E0001/E0002/E0004 直接断言矩阵；E7/E8/E0003 不可触发性文档化存档
- **297 项测试全绿**（298 函数，0 失败 / 1 忽略-文档化）；§3.2 验收全绿（release）

### 工程整理

- examples/ 重组：usage/（6 个 .krf）+ audit/（审计集）+ README 索引（§9.6.2）
- lang-design v5.2：deep-review §6 偏差清单 26 项全量回写（操作码全枚举/HeapObj 六变体/
  根集五来源/GC 数值冻结/内置 24 项清单/reserved.rs 签名回填/7 层→9 crate 映射表等）
- 工程文档对账：matrix / pipeline-test-coverage（§9.5.1 三层 + §14.6.1.1 完整性）/
  negative-tests.md 新建 / TD-012~014 登记

### 基准

- fib(25)（含编译）：84.4 ms/轮（release，复现声称 84.7）
- GC 压力：3×10^5 分配单轮 ~0.17 s（更正 0.72s 陈旧口径；gc_tests 10^6 为验收权威口径）

### 下一步

Gate R2 门审查复审（审计集就位后按 §7.3 重跑）→ §6.3 外循环投票 → Stage 1 规划输入。

## v0.1.0-r4（2026-09-10）——Stage 1 批次 A：切换期重构 + 第一批工作项

**SOP 流程**：Stage 1 启动（§21 规划 → §17 排版图 → §18 依赖审查 → §13.1 设计对齐 → §4 MUV 批次 A）。

### 交付（3 MUV + 1 重排）

- **TD-015 已解决**：`compile_source` 按消费方分流——run/eval 生产路径改走
  `compile_front` 前段（不构造图 IR）；完整入口保留给 `ir`/`code`/`bc`/`check`
  dump 与检查子命令；分流守护测试（快路径与完整编译字节码逐指令一致）。
- **TD-012 已解决**：expander.rs 1372 → 517 行（-62%）——三职责分置
  （core_forms.rs 511 / sugar.rs 426，crate 私有模块，公共 API 零变化）；
  测试整体保留主控走公共入口（拆分等价性天然回归）。
- **TD-007 部分解决**：宏展开 trampoline 工作表（宏产物头部仍是宏调用时
  迭代继续，展开控制流栈深与链长解耦）；深度上限 128 → **500**（实测
  标定：2MiB 测试线程 1_000 通过/2_000 溢出，2× 裕度——TD-017 同型
  实测法；探针 example 双环境数据记录于 TD 登记）；完整 10_000 口径
  依赖 Stx Rc 化（批次 B 前端重写）。
- **TD-004 重排批次 B**：完整实现 = 绑定 scope 注入 + CoreExpr::VarRef
  scope 桥 + 双路径解析体系切换（≥800 LOC 跨 5 crate）——超出单 MUV
  容量，与 TD-002/标准库同批（依据 §1.2.1 只升不降 + §12 最优>最小）。

### 质量口径

- §3.2 全绿：build --release 0 警告 / check --all-targets 0/0 /
  fmt 零 diff / clippy -D warnings 零警告 / test --release **304:0:1**
- 审计集 41/41 复跑 EXIT 0；新增集成套件 expansion_worklist_tests（4 例）
- SOP 文件更名：`stage-committee-process.md` → `sop.md`（引用同步 3 处）
- worklog Task 18-21 全记录；lang-design 03-macro-system TD-007 注记回写

## v0.1.0-r5（2026-09-10）——Stage 1 批次 B：TD-002 符号值 + 标准库最小集

**SOP 流程**：批次 B 按 plan.md §5 序列推进；MUV 22-a（TD-002）与
MUV 22-c（标准库最小集）各走完整内循环（22-b TD-004 按重排裁定留批次 E 收口批次整体推进）。

### 交付（2 MUV）

- **TD-002 符号部分已解决**：`(quote sym)` / `'sym` → 符号值——全链落地：
  - `LiteralValue::Symbol(Rc<str>)`（kerf-core；存剥离卫生后缀的基名，
    Racket 语义近似——与 resolve_hygiene_fallbacks 同一 $hyg$ 剥离口径）
  - `Value::Symbol`（kerf-vm）+ eq? 按名相等 + type_name "symbol"
    + render（裸名）/ 序对渲染（slot_terminal/render_slot）
  - `BcConst::SymLit`（kerf-compiler 常量池；与全局名索引 `Symbol`
    变体语义严格区分——§11 接口隔离）
  - `HeapObj::Symbol` / `ValueSlot::Symbol` / `Heap::alloc_symbol`
    （kerf-runtime；符号入序对 + car/cdr 往返）
  - 双路径同步：VM `const_to_value` 与 eval `eval_literal`；
    宏模板内 quote 符号卫生后缀 datum 层剥离（实测验证）
- **向量 datum 仍显式报错**（TD-002 向量部分开放，后续阶段）

### 质量口径

- §3.2 全绿：build --release 0 警告 / fmt 零 diff / clippy -D warnings
  零警告 / test --release **324:0:1**（304 基线 + 20：TD-002 5 +
  stdlib 15）/ 审计集 41/41 复跑 EXIT 0
- 负向锚点迁移：符号 datum 报错负例 → 符号值语义负例（negative_vm
  symbol_value_misuse 6 case 实跑校准：算术/条件/序对/比较位置）；
  全局正负比 1:3.1 维持（stdlib 负例 172 case 三维矩阵）

### 追加交付（MUV 22-c：标准库最小集）

- **24 项新内置函数**（48 项总量；07 §3.3 阶段门条件 3：列表/字符串/I/O
  各 ≥8——driver `register_globals` 注册，语言核心零内置原则不变）：
  - 列表操作（8）：`length`/`append`/`reverse`/`list-ref`/`list-tail`/
    `member`/`assoc`/`last-pair`——堆序对链遍历，nil 终结契约；
    `member`/`assoc` 按 `eq?` 查找
  - 字符串处理（10）：`str-length`/`str-substring`/`str-index-of`/
    `str-contains?`/`str-prefix?`/`str-suffix?`/`str-upcase`/
    `str-downcase`/`string->symbol`/`symbol->string`——字符索引
    Unicode 安全（`str-length "héllo"` = 5）；符号互转联动 TD-002
  - 基本 I/O（6）：`newline`/`write-string`（通道层 `write_stdout` 新增）/
    `read-int`/`read-num`（行解析，失败结构化报错）`/`error`/`assert-eq?`
- **高阶函数（map/filter/foldl/for-each）显式推迟 B3**：用 kerf 源码
  preamble 实现是 B3「Reader kerf 重写」的自举验证命题本体（§12
  最优>最小：Rust 抢实现移除 B3 验证内容；源码拼接方案的 Span 污染
  为真实 P1 缺陷，worklog 22-c 记录裁定依据）
- **修复伴随缺陷 2 项**（std 函数实跑发现）：`list`/`reverse` 空参
  曾返回 `(nil)` 包装形态（堆 nil 槽包成序对）——改 nil 值形态与 `'()`
  一致；`list-tail` k=0 曾对非 list 输入静默返回（Racket contract
  严格语义：每步形态校验）
- 测试：stdlib_tests 15 函数（正例 59 断言 + 负例 172 case——元数/
  类型/边界三维矩阵 + 类型全扫描）；全局正负比 1:3.1 维持

### 文档回写（合并）

- 09-stdlib v5.3（48 项清单 + 高阶函数 B3 推迟注记）/ capability-
  boundaries（48 内置）/ matrix 324 对账（stdlib 15 函数 172 负 case）/
  status r5 / tests/v0/stage1/plan.md + plan/stdlib.md 新建
