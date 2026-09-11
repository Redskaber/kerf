## v0.3.0-r16（2026-09-11）——批次 F：Stage 1 深审收尾环（§14 阶段末全协议 + §14.6 阶段间验证 GO + TD-023 登记 + lang-design v6.2）

### 交付一：§14.5 D1-D8 深度审查（deep-review-round1.md + 委员会投票 GO-WITH-CONDITIONS）

- **P0/P1 = 0**；P2×2（TD-023 gc_stress 回归新登记 + 登记册目标时机过期家族）；P3×9；**无系统性 B3「实现违反设计」**——全部偏差为「实现前进、文档滞后」方向
- 偏差清单 17 项（子代理 36-a-facts 独立扫描 20 篇 lang-design + 主代理亲验五项抽查全中）+ 无偏差确认 12 组（40 操作码/52 内置/parity 36/28/预留 14 项等核心契约面全对上）
- §6.3 批次 F 批准补票（stage-2/plan「规划批准中」悬空项）：五角色加权 5.5/5.5 = 100%
- §3.2 基线 clean 全量复验：build 9.86s 零告警 / check 0/0 / fmt 0 diff / clippy 0 / **553:0:0**（25s）+ CLI 冒烟

### 交付二：§14.8 设计回写（lang-design v6.1→v6.2 十篇 + 登记册 v0.3.0-r16）

- **TD-002 符号值家族三篇**：05（HeapObj 七变体/八入口/三通道函数/TD-023 注记）、06（值域十变体 + 语义字段限定）、01（literal_value + Symbol）
- **批次 E 滞后四面**：09（TD-021 r15 解决注记）、11（553 口径）、07（471 行 + 全模式面限定）、03（ExpandCtxt 四字段）
- **12-roadmap 两行矩阵改写**：元循环求值器（保持 Rust 参考，eval 自举滑入 Stage 2）/ 宏系统（E1-β 收口 ✅，调试工具 Stage 2+）；10-toolchain 补 CLI 11 子命令清单（B4 收口）
- **登记册全量对账**：索引补全 23 行（TD-015+ 此前无索引行的结构性缺口修复）+ TD-012 resolved（批次 B 六文件实证）+ TD-013/009/010/014/017/018 改判 Stage 2（附门放行裁定）+ TD-019/020 断档登记（禁复用）+ **TD-023 新增（P2——绑定批次 I2）**

### 交付三：§14.6 阶段间深度验证（六篇 + 自举 vs 种子前段实测）

- **architecture-review**（32✅/3⚠️/0❌——Stage 0 ⚠️×4 收敛 2）/**design-impl-test-coverage**（B1 全登记零未登记缺口 + 三者不一致零项）/**hidden-problems-assessment**（复杂度 ≥2× 强制修复触发 0 项）/**refactoring-optimality-review**（7 重构 7 优 0 hack）/**performance-baseline**/**final-assessment**（GO + 门审 checklist 增补：登记册核对 + 对账面六面清单）
- **自举 vs 种子前段开销实测**（§21.5 信号 4 补录）：**比值 69×（fib）/157×（宏）/116×/388×（300-defines）** + 冷启动 16.1ms + fib bench 89.4~89.6ms（**自举切换零性能代价 +0.8% 噪声带**）——**stage-2 DAG 的 H2→I1 次序被实测验证**（TCO 是编译器本体迁移的实质前置）
- **gc_stress 回归**（+29~46% 超线性，超 §14.6.4 10% 阈值）：TD-023 登记（per-cycle HashSet 分配 + Value 宽度候选根因）→ 批次 I2（TD-008 分代同轮）；正确性无影响不阻塞
- pipeline-test-coverage.md 全量重写（294→553 口径 + Stage 1 套件 11 行 + parity 印证节 + catch-all 42→1 通配实测）

### 交付四：§14.9 C1-C6 代码整理（零语义变化）

- 三处注释级修正（driver.rs 切换守护方向反写 / kerf-runtime「四类根」→五来源 / 堆语义注释过时）——**553 零断言修改逐一等价复跑实证**（探针临时部署/移除各一次全绿）
- C1-C6 全维检查：TODO/FIXME 0、glob 0、//! 66/66、§11 五项合规、真通配 catch-all 1 处（带臂级注释）

### 交付五：质量口径与收尾

- **§3.2 六命令实跑全绿**（clean 420 files → build 9.86s → check 0/0 → fmt 0 diff → clippy -D 0 → test **553:0:0**）；双审计集 EXIT 0（41 + 50）；探针移除后全量复验 553:0:0
- 文档同步：v0.5-roadmap Stage 1 行 ✅ + matrix r16 对账（表体两行尾差修复）+ stage-1/plan 批次 F 注记 + stage-2/plan TD 绑定行（G2←TD-013 / I2←TD-023 / H2←TD-017）
- r16 tar.gz（§19.4 整目录 + 包内自举验证 553:0:0 + CLI 一致）+ web 同步 + E2E + git commit（r13-r16 积欠统一入账 per §6.4）

### 下一步（Stage 2 主体——批次 G 起）

G3 FFI 所有权模型定义（§21.3 阻塞项先行）→ G1 QBE 后端 PoC（QBE 二进制按 §3.1 链位安装）∥ G2 HM 推断设计轮（+ TD-013 多错误恢复绑定）；H2（TCO + TD-007 + TD-022 + TD-017——**I1 实质前置，性能基线实测锚定**）；I2（TD-008 分代 + TD-023 根扫描对症）。

---

## v0.3.0-r15（2026-09-11）——批次 E 收口：E1-β 宏系统 + 生产切换 + TD-021 prelude + Stage 1 门审查 APPROVED（553 全绿）

### 交付一：E1-β 宏收口——expander.krf 完整宏系统（syntax-rules 卫生宏，VM 上运行）

- **宏机制**（expander.krf +~470 行）：define-syntax 注册（变换器注册表 = 名字单表——用户宏 'rules / 糖惰性注册 'builtin，HashMap 替换语义镜像：用户宏可覆盖糖名）+ syntax-rules 解析（字面量集合/子句 (模式 模板)/五类解析错误逐字镜像）+ **模式匹配**（通配 `_`/字面量（同名 sym）/省略号（零/多段/复合单层）/字面量 datum 值相等/嵌套列表/向量 `[...]`——trial 语义经函数式绑定传递）+ **模板实例化**（模式变量替换保留用户 Span/作用域、(var ...) 省略号拼接跳格、悬垂省略号按普通符号走实例化——报错优于静默）+ **卫生 α 重命名**（`name$hyg$N`——num->str 递归构造；同标识符同实例化恒同映射；保留集（宏自引用）+ 核心关键字豁免）+ **深度计数 500**（链长语义——expand-form 入口快照/出口回滚，兄弟不累计；超限结构化报错非栈溢出）+ retag def∪use 并集 + 卫生基名回退（rsplit $hyg$ + 后缀纯数字保守校验）
- **Span 并集代次守卫镜像**（实测驱动的发现——parity 失败暴露）：节点形状升 v2 = `(tag s e exp scopes ...)`（exp = Span.expansion_id，桥侧发射/读回）；**instantiate 的 Span 并集须镜像 Span::merge 保守策略——代次不同 → 丢弃该跨距**（糖产物内宏调用：use-site 代次 ≥1 vs 模板 0 → 产物 Span 落模板范围）；retag 代次 +1（bumped_expansion）；全部构造器（糖/提升/宏）逐位镜像代次来源
- **核心节点携带代次**（生产切换的相位可观测性保持）：core 节点 `(tag s e exp ...)` → 桥重建 `Span{expansion_id}`——「宏引入引用未绑定错误应携带 expansion N 标记」（E3 诊断特性）经切换后保持
- **实测驱动修复**（§2.3-11——全部经 parity 双实现实跑发现）：foldl/for-each 序章补定义（chars->str→foldl 此前为死代码——卫生基名回退首次激活）+ 宏产物品类 6 处（let 括号/字面量 tag 铸造/letrec 先用后绑/...的 r14 既有 + E1-β 新发现：set/require/define-syntax 代次字段漏挂×3、模块注册簿错误无 Span 无渲染×2）

### 交付二：生产切换（E1-β——读 + 展开两段全自举）

- **compile_front 展开段** → `bootstrap_expander::expand_program`（VM 上 expander.krf）——**「语言能表达自身前端」的完整生产命题**；Rust 种子保留双角色：自举引导（reader.krf/expander.krf/preamble.krf 的编译经 compile_front_seed）+ parity oracle（测试对照）
- **切换守护**：driver 单元 `production_expander_is_bootstrap`——独立线程（thread_local 零残留）编译前活性探针 `is_loaded()` = false → 编译后 = true（实测判别非推断）+ 宏产物展开代次标记 ≥1 双信号
- **553:0:0**（528 基线零回归——全套件经自举 Reader + 自举 Expander 生产管线执行）

### 交付三：TD-021 高阶函数用户面注入（P3 清偿——模块/import 承载）

- **kerf-prelude 模块**（`bootstrap/preamble.krf`）：map/filter/foldl/for-each（reader.krf 序章同源）；用户程序 `(module 名 (import kerf-prelude) ...)` 声明导入
- **注入机制**（P1/P3/P5 三否决全规避）：read 后、expand 前 forms 级合并（独立 file_id——Span 指向 preamble.krf，无源码拼接诊断污染）；单一编译单元（无跨程序全局合并）；零 VM 再入；**多模块按序 declare + 主模块 visit**（§8.9 单模块生命周期扩展——import 边传递依赖）；同名 define 显式报「重复定义」（不静默遮蔽）；无 import 声明保持未绑定（opt-in 语义）
- **prelude_tests 7 case**：用户面可见/组合管道（filter→map→foldl = 50）/for-each 副作用/双路径一致/opt-in 负例/名字捕获显式失败/未知导入

### 交付四：Stage 1 门审查（§7.3 + §21.3 四条验收——APPROVED）

- **审计集** `examples/audit/stage1_gate_audit_r1.rs`（50 case，可重运行）：§7.3.1 配比满足（A 单语句 12 / B 多语句 12 / C 复杂 10 / D 恢复 6 / E 边界 6 / P 正向 4）+ §7.1.1 **七类全覆盖**（语法/未绑定/空应用/参数个数/类型不匹配/循环依赖（双模块源码 → DFS 灰标记结构化 Err）/宏深度（自指宏经生产路径））+ §7.3.2 边界 6（本批次修复面：Span 代次守卫糖内宏/require·set 代次/prelude/模块内宏注册）——全部 case 经**生产管线**（自举读+展开）
- **审计发现项 2（已修复）**：模块注册簿错误（未声明模块/循环依赖）无 Span 定位 + 无诊断渲染——修复：挂 module 形式 Span + render_diagnostic（P2 诊断质量）
- **§21.3 四条件锚定**：① 子集表达前端——生产管线自举（VM Reader + VM Expander 含宏）② VM 上正确运行——自举 Expander 承载全部生产展开 ③ 标准库最小集——prelude 管道（列表 hofs 用户面）+ stdlib（列表/字符串/I/O）④ 增量编译——P04 缓存确定性 + 第二次 check 命中观测 + 静态检查 0 错
- **结论**：APPROVED（50/50 PASS + 配比满足 + 七类全覆盖 + 零新 P0/P1）——Stage 1 全批次（A/B/C/D/吸收/E）交付闭环，进入 Stage 2 准备

### 交付五：质量口径

- **§3.2 六命令实跑全绿**（clean 起步：498 files/123.6MiB → build 9.39s 零告警 → check 0/0 → fmt 零 diff → clippy -D warnings 0 → test **553:0:0**（单元 184 + 集成 369））；双审计集 EXIT 0（stage0 41 + stage1 50）；CLI 冒烟 fib 75025 + ⇒ 144 + kerf test 2/2 + 宏+prelude 自检 ⇒ 10
- 文档同步：tech-debt-register TD-021 → 已解决（r15）+ matrix 553 对账 + stage-1/plan.md 批次 E 行收口 + bootstrap-expander.md E1-β 增补 + prelude.md + stage1 门审计测试计划

### 下一步（Stage 2 准备——§21 阶段规划先行）

Stage 2 启动条件核对（§7.3.3 收敛裁定 + §21.5 切换信号）→ 阶段规划（§21 + §13.1 设计对齐 + §17 排版图）→ 后端策略裁定（LLVM 可选后端引入评估——永不入自举链边界重申）+ TD-007 残留（10_000 完整口径）与 TD-022（Reader 帧消耗 O(n)）的 TCO 决策点。

---

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

## v0.1.0-r6（2026-09-10）——Stage 1 批次 B 收官：B3 自举 Reader（Reader kerf 重写）

**SOP 流程**：批次 B 收官按 plan.md §5 序列（Task 23 尾注）；MUV 24-a
（VM 宿主调用 API）→ 24-b（reader.krf + 桥 + 生产读路径切换）→ 24-c
（parity 套件）→ 24-d（交付闭环）各走完整内循环。

### 交付（B3：Reader 以 kerf 源码重写，在 Stage 0 VM 上运行）

- **reader.krf**（`kerf-driver/src/bootstrap/reader.krf`，~430 行 kerf 源码）：
  完整词法 + 语法逻辑——`lex-src` / `parse-tokz` 两入口（与种子
  `lex_source`/`parse_tokens` 接口形状对齐，§11）；尾调用链形态的错误
  传播（err 标签值返回，不依赖异常）；高阶函数序章（map/filter/foldl/
  for-each 以 kerf 源码实现——r5 裁定「自举验证命题本体」的交付面）
- **VM 宿主调用 API**（24-a）：`kerf_vm::call_closure`——函数入口帧语义
  （底帧 RET = 程序化返回值）；跨程序闭包显式拒绝（P3 否决的运行时面）；
  错误追踪与 run_program 同构
- **自举桥**（`kerf-driver/src/bootstrap.rs`）：种子管线编译 reader.krf
  （thread_local 惰性加载，无递归）+ 值树 ↔ Token/Stx 转换 + 数字文本
  同源 parse（i64/f64 转换是宿主类型边界）+ 持久堆（GC 根集含全局）
- **生产读路径切换**：`compile_front`（run/eval/compile_source 全管线）
  与 `dump_tokens`/`dump_stx` 经自举 Reader；种子（kerf-reader）保留为
  引导实现 + parity oracle（`compile_front_seed`）
- **+4 自举 Reader 原语**（52 项总量）：`str->pos-chars`（(字节偏移 .
  单字符) 列表）/ `char-whitespace?` / `char-alphabetic?`（Unicode 属性）/
  `str-int-valid?`（i64 域——扫描序内溢出前置校验，维持首错位置 parity）
- **错误次序契约**：数字溢出在 VM 词法扫描序内前置报错（与种子一致）；
  数字文本最终转换桥侧同源 parse（消息与 Span 逐字节一致）

### parity 验收（bootstrap_reader_tests：28 函数，正 87 / 负 307 case）

- **Stx 树等价**：datum + Span 递归比对（符号按名——两实现 intern 次序
  不保证一致）；语料 = 种子 reader 测试全集 + 奇异边界（省略号族/
  多行字符串/科学计数法/前导零/Unicode 标识符/NFC 组合归一）
- **错误 parity**：消息 + Span 逐字节（28 负例语料 + 系统化矩阵 266：
  未闭合深度 1..30 / 括号错配矩阵 / 非法转义 32 字符（含多字节——
  种子 eo+1 字节口径逐字节复刻）/ 贪婪数字 10×5 矩阵 / 溢出扫描
  19..26 位 × 3 符号 / 非法字符 8×3 上下文 / 双错误次序 / 注释嵌套）
- **双错误次序**：溢出 + 后置词法错误（首错 = 溢出）与前置错误反向——
  两实现首错位置逐字节一致
- **高阶函数直测**：map/filter/foldl/for-each 经自举桥（builtin 作 f——
  跨程序安全的函数值）
- **防误收断言**：全部负例先断言种子确实报错（`assert_negative_parity`）

### 质量口径

- §3.2 全绿：build --release 0 警告 / fmt 零 diff / clippy -D warnings
  零警告 / test --release **356:0:1**（324 基线 + 32：VM call_closure 4 +
  parity 28）/ 审计集 41/41 复跑 EXIT 0
- **356 全套件（含全部既有负向消息断言）经自举 Reader 执行**——整体
  行为等价的实证（孤立正确 → 集成正确的 §7 防崩验证）
- 全局正负比 **≈1:3.2**（负 1000 case / 正 ≈311；§9.4.3 门限维持）

### 技术债登记

- TD-021（P3）：高阶函数用户面注入缺载体（P1/P3/P5 否决——批次 E 模块系统）
- TD-022（P3）：自举 Reader 帧消耗 O(源字符数)（无 TCO 边界——Stage 2 决策点）

### 文档同步

07-bootstrap v5.3（§3.2 B3 交付注记）/ 09-stdlib v5.4（4 原语 + hof
注记）/ 02-syntax-model（B3 双实现注记）/ stage-1 plan §5（B3 ✅）/
matrix r6 对账 / tests/v0/stage1/plan.md（parity 套件行）/ status r6 /
data-flow（自举 Reader 读路径）

## v0.1.0-r7（2026-09-10）——Stage 1 批次 C：类型检查器 + 编译缓存 + TD-016/013

**SOP 流程**：批次 C 按 plan.md §5 序列（Task 24-d 尾注——批次 B
收官后承接）；MUV 25-a（类型检查器本体）→ 25-b（编译缓存）→ 25-c
（TD-016 收紧 + check CLI）→ 25-d（TD-013 设计批）→ 25-e（交付闭环）。
L3 全量内循环（跨 kerf-compiler/kerf-driver/kerf-syntax/CLI/web 五面）。

### 交付一：保守静态类型检查器（25-a——§21.6 循环依赖缓解落地）

- **kerf-compiler/src/typecheck.rs**：`check_program(core, builtin_sigs, table)`
  → `Vec<Diagnostic>`（E0005）。**保守性契约**：只报告静态确定的错误——
  每条规则触发时对应程序运行期必然以同类错误失败；动态信息不足一律
  Unknown 跳过（误报 = P1 缺陷的工程口径）
- **R1-R8 规则集**：if 条件非 bool / 算术非数值 / 比较族（TD-016
  全操作数口径 + TD-011 字符串边界）/ not / car-cdr 非序对 / 不可调用
  值 / 元数（字面量 lambda + 内置签名）/ 字符串符号族
- **内置签名表注入**（§2.3-10 唯一可信源）：builtins.rs 的
  BUILTIN_SIGS（49 项）与 register_globals 同文件维护——签名表 ⊆
  注册表双向防漂移锚（`builtin_sigs_subset_of_registered` /
  `builtin_sigs_cover_operator_families`）
- **多错误收集**：全量诊断按 Span 次序（TD-013 设计的首个实证消费面）
- **深度预算** MAX_CHECK_DEPTH=512（Reader 256 上限 2×；debug 帧实测
  ~1 KiB，2000 深度实测溢出 2 MiB 栈——程序化构造单测存档）
- **消费面**：`kerf check` 子命令（编译 + 静态报告 + 缓存观测，发现
  问题 exit 1）+ `check_source` 库 API + web /api/check

### 交付二：编译缓存（25-b——13 §3.1.4 三方法规格做实，§21.3 条件 4）

- **InMemoryCompilationCache**（kerf-driver/src/cache.rs）：冻结 trait
  `get_cached`/`store`/`invalidate` 行为规格 1/2/3 逐条测试锁定 +
  管线富入口（`lookup_front`/`store_front`——完整前端输出含符号表/
  源映射/模块簿记；快照语义克隆）
- **SHA-256 内容寻址**（hash.rs 零外部依赖自实现，FIPS 180-4；NIST
  标准向量锚定）：`CacheKey{source_hash 截断 u64, config_fingerprint
  = 阶段种子 + 文件名}`（位置信息入指纹——SourceMap 产物等价性要求）
- **管线接线**：run/eval/compile/check 经 `compile_front_cached`
  （同源二次命中——eval 共享 run 条目；错误路径不缓存；`cache_stats`
  观测 + `set_cache_enabled` 基准对照开关）
- **确定性证明**：`cached_program_equals_fresh_compile`——命中产物 ==
  新编译产物（BcProgram PartialEq 逐字段）

### 交付三：TD-016 收紧（25-c——链式比较全操作数前置校验，双侧）

- 运行时面：`cmp_builtin` 前置全参数数值校验（全字符串+排序族 →
  TD-011 消息；其余首个非数值 → `{op} 需要数值`；**既有两参消息
  逐条兼容**——负例矩阵全绿回归）
- 静态面：R3 规则同口径（Ordering / NumOrAllStr）
- 语义收敛：`(< 3 1 "a")` 静默 false → 结构化错误（FS-5 边界闭环）；
  `(= 1 2 "s")` 同理；`(< "a" "b" 1)` 混串消息统一为「需要数值」
- 09-stdlib §2 v5.5 重写 + negative_vm_tests 语义边界注记更新

### 交付四：TD-013 设计批（25-d——多错误收集设计冻结）

- docs/develop/v0/stage-1/multi-error-recovery-design.md：恢复粒度 =
  形式级（表达式级不恢复——半展开状态重建成本）；恢复机制 = 编译期
  控制流（非 effect——§11 接口隔离裁定）；收集上限 128 + 截断提示；
  输出按 Span 次序；实现绑定批次 E（验收标准 5 项）
- 登记册 TD-013 → 设计完成；TD-016 → 已解决（r7）

### 质量口径

- §3.2 全绿：build --release 0 警告 / fmt 零 diff / clippy -D warnings
  零警告 / test --release **408:0:1**（356 基线 + 52：typecheck 24 +
  cache 13 + stdlib TD-016 2 + compiler 单元 3 + builtins 签名 2 +
  cache 单元 6 + hash 单元 2）/ 审计集 41/41 复跑 EXIT 0
- 全局正负比 **≈1:3.2 维持**（负 1077 / 正 ≈340 case；r7 typecheck
  负例 68 全部携带静态确定性反向锚——静态报错程序实跑必报 Run 错）
- **零误报保守性机械验证**：examples/ 6 程序 + 既有 408 全套件零
  新诊断（缓存行为等价 + 检查器保守性双证明）

### 下一步（批次 D，plan §5）

Effect 内部最小实现（编译器错误恢复用）+ 能力 I/O 基础传递
（13 §3.1.3 规格）。

## v0.2.0-r8（2026-09-10）——Stage 1 批次 D：能力 I/O 基础传递 + 内部效应做实 + 用例运行器

### 交付一：Effect Handlers 编译器内部做实（12-roadmap §2.4.3 第二级）

- `kerf-driver/src/effects.rs`（新，414 行）：**类型化一次性逃逸层**
  `handle_escape<R,T>` / `perform_escape<T>`——载荷从任意嵌套深度
  上展开至最近同类型边界，零签名污染（「任意流程节点能力」的机械
  实现）；线程局部深度计数 + `catch_unwind`/`resume_unwind` std-only
  载荷逃逸 + 私有载荷类型判别的 panic hook 过滤（效应控制流零噪声、
  真实 panic 照常穿透——12 单元测试锁定）
- **冻结契约层** `InternalEffectSystem`：reserved.rs `EffectSystem`
  P3 形状的真实现（与 Probe 测试构成「可编译/可承载」契约双证）；
  语言面保持 P3（D1 裁定——语言级 perform/handle 语义留 Stage 2）
- VM 帧 `ext1` 槽位不激活（Stage 2 语言级效应时启用）

### 交付二：能力模型 I/O 基础传递（13 §3.1.3 四条款做实）

- **声明形式 `(require io read|write)`**：`CoreExpr::Require`（零运行时
  语义——求值恒 nil 双路径一致 / 编译产 PushNil / IR 降级 nil 节点）；
  幂等集合语义；核心冻结边界精确化（「语义原语集冻结 + 声明变体可
  追加」——01-core-forms §6）
- **R9 保守静态权限验证**：门控内置名（read-line/read-int/read-num/
  print/newline/write-string）任意位置引用未声明 → **E0006 编译期
  错误**（front 管线 run/eval/check/compile 全路径单一验证点；用户
  接管豁免 + 卫生回退基名判定——零误报纪律）
- **令牌授权面**：`mint_read_token`/`mint_write_token`（pub(crate)
  构造面控制）+ `IoGrant` 按声明铸造 + `StdCapabilityIO`（冻结 trait
  的 stdio 实现）+ `register_globals` 能力参数化（未声明即不注册——
  fail-closed）
- **FS-4 修复**：read-line 元数校验补齐（能力参数化重写时顺带——
  negative_vm_tests 存档断言激活）

### 交付三：用例运行器 `kerf test`（测试面即语言面）

- `test_source` API + CLI `kerf test` 子命令：前置形式（define/set!/
  module/require）逐用例重放 + 顶层表达式 = 用例（值非 #f = PASS）；
  **短路 + 恢复**（效应系统消费面——case 内任意深度失败即停、边界
  捕获后下一 case 续跑）+ 状态隔离（每 case 全新环境与堆）
- 设计文档：[11-testing §4](docs/lang-design/11-testing.md)

### 交付四：文档回写 v5.4（lang-design 七文件 + sop.md v11.1）

- lang-design：00（v5.4 修订记录 + next.md 吸收审计结论）/ 01（§6
  require 声明形式设计）/ 02（Token 叶级 45）/ 09（v5.6 能力门控）/
  11（§4 用例运行器）/ 12（v5.4 演进矩阵状态）/ 13（v5.4 双做实注记）
- sop.md v11.1：§21.9 矩阵现状对账（四项 ✅ + 类型检查器提前引入
  偏差登记）+ §16.1 变更日志 + §21.7.1 P4 行注记
- 测试计划：capability.md / test-runner.md（新）+ matrix.md r8 全量
  对账（含 r7 陈旧计数修正）

### 质量口径

- §3.2 全绿：build --release 0 警告 / fmt 零 diff / clippy -D warnings
  零警告 / test --release **476:0:0**（408 基线 + 68：capability 24 +
  test_runner 18 + driver 单元 25（effects 12 + capability 13）+
  negative_vm +1（FS-4 激活））/ 审计集 41/41 复跑 EXIT 0
- 全局正负比 **≈1:3.15 维持**（负 ≈1118 / 正 ≈355 case；E0006 能力
  权限码逐条断言——第七族结构码就位）
- 确定性纪律：read-line EOF 语义经子进程探针（Stdio::null()——不依赖
  运行器 stdin 形态）

### 下一步（批次 E，plan §5）

Expander kerf 重写 + TD-004 scope-set 解析收口 + TD-021 hof 用户面
注入（模块机制）→ Stage 1 门审查（§7.3 + §21.3 四条验收）。

## v0.2.0-r9（2026-09-10）——next2.md 吸收轮 + 测试入口架构重构 + lang-design v5.5

### 交付一：next2.md 五轮讨论增量吸收（lang-design v5.5）

- 与 next.md 轮（零缺口）不同，本轮识别**实质缺口**（Perform/Handle/de Bruijn/四层正交/六 IR/MLton/Koka/comptime 于既有文档集零命中）并全量吸收至九文件：01 §7（核心原语 9 vs 8 vs 7 数量真相 + 命名精确性表 + 效应原语化 Stage 2 演进对照裁定——核心冻结不动摇）/ 14 §4（2026 前沿五维成熟度矩阵 + comptime 生产就绪档登记 + 批判审查史 + 可行性评分对照）/ 15 §5（四层正交 crate 对照——r8 实现与 next2 最终版架构同构实证 + IR 六层演进表 + MLton 闭包/Koka 效应消除锚点）/ 12 §2.4.1（S 表达式量化背书 300 vs 3000 行 + 皮肤骨架论）/ 17（六原则对照映射）/ 18 §3（八条术语）/ 19（六行参考）/ 13（comptime 注记）/ 00（v5.5 修订记录）

### 交付二：测试入口架构重构（sop.md §8.4.6 v11.2——Cargo.toml 干净精要）

- **`tests/runner.rs` 单一总入口**：cargo 自动发现（零 [[test]] 配置），`#[path]` mod 树挂载 v0/stage-N 全部 18 个测试文件——阶段/plan/gate 目录语义不变，仅入口收敛；**Cargo.toml 125 行 → 50 行**（[[test]] 18 块清零，仅保留 [[example]] 嵌套声明）
- 共享辅助单实例化：`tests/common/` 由每文件 `mod common` 重复加载（clippy duplicate_mod）改为 runner 单实例 + `use crate::common`（10 个测试文件迁移；3 个未使用者清理）
- sop.md v11.2：§8.4.6「测试入口架构意图」+ 强制规则 8（禁止 [[test]] 逐文件声明）+ §9.1 树更新 + §16.1 变更日志；testing-guide.md r9（运行命令与编写规范更新）
- **测试总数 476 与逐模块计数完全不变**（组织收敛零语义变化）；选择性运行 `cargo test --test runner <module>::`（模块路径即过滤器）

### 质量口径

- §3.2 全绿：build 0 警告 / fmt 零 diff / clippy -D 0 / test **476:0:0**（runner 单二进制 301 + 单元 175）/ 审计集 41/41 EXIT 0
- 集成测试单二进制运行时间 6.95s（18 二进制顺序执行 → 1 二进制并行——加速且零重复编译）

## v0.3.0-r10（2026-09-10）——next3.md 七轮吸收 + 内部语法三原则落地 + 架构合规审计

### 交付一：next3.md 七轮讨论吸收（stage0.md v6.0 + lang-design v6.0 双层收敛）

- **upload/stage0.md v5.0 → v6.0**（原语批判演进版，3706 → 4216 行）：新增 §6.9-§6.12（9/8/7 数量真相与不可消除性证明 / 2026 理论前沿全景与整合决策矩阵 / 四层正交架构与 8 原语重设计 / 最终修正（显式 continuation 类型 / 真解耦四层 / 六 IR / MLton 闭包三策略 / Koka 四阶段效应消除）+ 可行性审查（五目标 A+/A/A/A+/B+）+ §6.12.6 本项目收敛裁定）；§7.3 表面语法决策（S 表达式 = 工程捷径，300 vs 3000 行，皮肤/骨架分离，分阶段语法策略）；§7.4 内部语法设计（告别 `#%` 派生关键词——类型安全 ADT 三原则 + 2026 内部 AST 完整 Rust 定义 + 旧→新迁移映射）；设计原则第 29-31 条；附录 A +17 术语 / E +8 引用 / F 设计讨论演进记录（七轮档案）
- **lang-design v5.5 → v6.0**（九文件回写 + 12 维度系统审查）：01 §8 内部语法设计（三原则 + 当前实现三项合规核验 + 十二行迁移映射 + §8.5 测试锚点）/ 02 §8 表面语法决策 / 15 §5.3 表面-内部分离 = 四层正交第五轴 / 14 §4.4-§4.5 ADT 对照与演进链 / 17 原则 29-31 + 附表四处编号误引修正 / 18 §4 六术语 / 19 §5 三引用（de Bruijn 1972 / Flanagan 1993 / Plotkin & Pretnar 2009）/ 12 §2.5.1 评估清单补齐（悬空引用修复）/ 00 v6.0 修订记录
- **审查修复六类缺陷**：版本号漂移 ×3（01/18/19 头部停在 v5.0/v5.4）、12 悬空登记、13 Version 字段污染、17 附表误引 ×4、**打包自包含性**（上游蓝图存档 docs/stage0.md 243,952 B + lang-design 三处外链改内）
- **收敛裁定**（stage0 §6.12.6 / 01 §8.2-§8.3 / sop v11.3 三处一致）：Stage 0-1 冻结 9 原语（+Require）不变；8 原语形态 + 四层/六 IR/MLton 闭包/Koka 效应消除 = Stage 2「目标语言完整化」评估清单；三原则即刻生效为架构验证基准

### 交付二：sop.md v11.3（原则 29-31 接入流程权威）

- §2.2 三十一条（29 命名行为导向 / 30 类型安全优于命名安全 / 31 表面-内部语法严格分离——各含违反示例）+ 协同关系块第五组
- §8.4.5 文档清单 + §9.1 docs 树登记蓝图存档；§8.4.6 补设计侧锚点接线（lang-design 测试锚点节 ↔ tests/ 树双向锚定）；§16.1 v11.3 变更日志；反臃肿四问全过

### 交付三：架构合规审计落地（代码 + 测试）

- **tests/v0/stage0/plan/architecture_audit_tests.rs（+4）**：十变体穷尽 match 冻结证明（编译期机器证明——新增/删除变体即编译失败）+ 逐变体 Span 独立携带 + kind_name 全映射互异 / Reader 产出 Stx 类型级隔离 / Expander 唯一 Stx→CoreExpr 桥（翻译发生证明）/ 同源双编译 render_core 全等（表面语法可替换性确定性基线）
- **kerf-core expr.rs 迁移映射文档块**（冻结边界 + 三原则合规 + Stage 2 演进目标——§8.4.5 文档随代码）
- **九 crate 五正交轴定位文档**（15 §5.3 落地：span=元数据基座 / syntax=表面语法层 / reader=轴 1 唯一桥接点 / core=骨架 / expander=翻译执行面 / compiler=轴 2 桥接 / vm=轴 3/4 运行时消费 / runtime=通道层 / driver=组合根）

### 质量口径

- §3.2 全绿：clean + build --release ✓ / check 0 error 0 warning / fmt 零 diff / clippy -D 0 / test --release --workspace **480:0:0**（+4 架构审计；单元 175 + 集成 305）
- 矩阵 r10 口径（逐模块计数与 r9 完全一致 + 新增套件）

### 下一步（批次 E，plan §5）

Expander kerf 重写 + TD-004 scope-set 收口 + TD-021 hof 用户面注入 → Stage 1 门审查（§7.3 + §21.3 四条验收）。

---

## v0.3.0-r14（2026-09-11）——批次 E / E1-α：自举 Expander（核心形式 + 九糖，VM 上运行，parity 19 测试）

### 交付一：expander.krf——Expander 以 kerf 源码重写（07 §3.2「Expander（新语言子集）」α 阶段）

- **自举程序**：`bootstrap/expander.krf`（~910 行 kerf 源码，种子管线编译为 **208 原型 / 4685 指令**——迄今最大 kerf 程序）：9 核心形式（lambda/if/set!/define/begin/module/quote/require）+ 九糖（let/letrec/let*/cond/and/or/when/unless/while）+ 内部 define 提升（含切分期名校验/值校验/错误短路序镜像）+ **TD-004 fresh-scope 深注入与作用域集携带**（r13 语义的 kerf 侧镜像）+ trampoline 糖链循环 + retag use-site 替换语义 + quote datum 转换（$hyg$ 剥离经字符表模式匹配——算术索引不经 int 参数内置）
- **值桥**：`bootstrap_expander.rs`——Stx → VM datum 节点（tag s e scopes，作用域集 int 列表）→ VM core 节点 → CoreExpr（作用域集排序去重重建；符号经用户表 intern）；共享 bootstrap.rs 走查辅助（§2.3-10 单一定义）；种子管线加载（compile_front_seed——Rust Reader/Expander 编译 expander.krf，无递归）
- **parity 验收**（tests/v0/stage1/plan/bootstrap_expander_tests.rs，19 测试）：结构（原语形态 + Span(start,end) + **作用域集 + param_scopes** 递归一致）+ 错误（消息 + Span 逐字一致——含提升路径短消息口径）+ 行为面（自举展开产物经 compile + VM 执行与种子全管线一致——**「语言能表达自身前端」的自举验证命题在 Expander 子集上成立**）；509 基线零回归 = **528:0:0**
- **E1-α 边界**（显式报错不静默，§2.3 原则 4）：define-syntax → 边界错误（宏属 E1-β）；糖产物 Span 展开代次不参与 parity 判据（r6 Reader 同口径）；影子路径——生产展开仍走 Rust 种子（切换随 E1-β 宏收口）
- **接口最小放宽**：`IoGrant::none()` pub 化（宿主/测试侧构造无能力全局环境的合法入口；令牌铸造面保持 crate 私有）

### 交付二：质量口径

- **§3.2 六命令实跑全绿**（clean 起步）；审计集 §7.3.1 EXIT 0；CLI 冒烟 fib=75025 + test 2/2 + **expander.krf 自检 ok（208 原型/4685 指令）**
- 文档同步：matrix 528 对账（正例 8 套件 + 负例 6 套件 + 边界 1 + 行为面 3）+ stage-1/plan.md 批次 E 行 + docs/tests/v0/stage1/plan/bootstrap-expander.md 测试计划

### 下一步（批次 E 续）

E1-β 宏收口（syntax-rules + HygieneCtx α 重命名 + 展开深度计数 + 宏产物 Span 代次 + **生产路径切换** + 基础宏定义）+ TD-021 hof 用户面注入（随模块系统）→ Stage 1 门审查（§7.3 + §21.3 四条验收）。

---

## v0.3.0-r13（2026-09-11）——批次 E 启动：TD-004 作用域集解析收口（Racket 式 (name, scopes ⊆) 双路径 + 509 全绿）

### 交付一：TD-004 作用域集解析（P2 清偿——E1 Expander kerf 重写的前置语义基座）

- **展开器**：`ExpandCtxt` 作用域分配器（fresh_scope 单调递增）+ 绑定形式（lambda）fresh scope 深注入——绑定器与全体体形式（`Stx::add_scope_to_all`，新 kerf-syntax 公共 API）；注入先于体展开 ⇒ 宏产物作用域 ⊇ use-site ⊇ {fresh}，自由标识符穿透保持；α 重命名（$hyg$N）保留为第二道卫生保险
- **核心层**：`CoreExpr::VarRef/SetBang` 携带引用作用域集（`scopes`）+ `Lambda.param_scopes` 携带绑定作用域集（与 params 平行）；`free_var_occurrences`（名 + 首现作用域集）——闭包捕获分析与名称版共用单一定义（名称版 = 其投影）
- **编译器**：`ScopeFrame` 绑定项化（名 + 作用域集）+ `resolve_var(name, scopes)` 帧序 + 帧内 max-cardinality 子集匹配；捕获描述符携带命中绑定的绑定作用域集（内层原型经同一子集匹配命中捕获槽）
- **eval 双路径**：`Env` 绑定项化 + `lookup/set` 同一子集匹配口径（T1 一致）；`ClosureValue::Eval` 携带 param_scopes，`apply_value` 经 `define_scoped` 绑定；**空作用域集绑定 ⊆ 任意引用集** ⇒ driver 根环境全局/内置天然兜底，`$hyg$` 基名回退（双侧）降级为回退路径
- **语义判别点**（与旧名称基差异）：名称匹配但作用域不匹配 ≠ 解析命中——按 Racket 语义判为未绑定（VM → 全局兜底后「未绑定的全局变量」；eval → 「未绑定变量」）

### 交付二：质量口径

- **509:0:0**（500 基线零回归 + 9 锚点新增；单元 183 + 集成 326）；§3.2 六命令全绿（clean 起步实测）；审计集 §7.3.1 配比满足 EXIT 0；CLI 冒烟 fib 75025 + test 2/2
- 锚点 = tests/v0/stage1/plan/scope_set_tests.rs（正例 5 双路径：shadowing/嵌套 shadowing/闭包捕获/set! 词法命中/子集对照；负例 4：作用域不匹配未绑定 VM/eval/set! + 宏引入不捕获）+ docs/tests/v0/stage1/plan/scope-set.md 测试计划
- 文档同步：tech-debt-register TD-004 → 已解决（r13）+ matrix 509 对账 + 03-macro-system 实现状态注记 + stage-1/plan.md 批次 E 行更新

### 下一步（批次 E 续）

Expander kerf 重写（E1——scope-set 语义基座就绪后重写，Rust 实现保留为 parity oracle）+ TD-021 hof 用户面注入（随模块系统设计）→ Stage 1 门审查（§7.3 + §21.3 四条验收）。

---

## v0.3.0-r12（2026-09-11）——接口预留完整性扩展轮（next4 第八轮：docs 三层 + 预留层代码冻结 + 500 全绿）

### 交付一：next4.md 第八轮三层吸收（stage0 v6.1 + lang-design v6.1 + sop v11.4）

- **stage0.md v6.0 → v6.1**（4216 → 4757 行）：新增 §9.3-§9.5 接口预留完整性审查（覆盖度评估六类 ~70% → 10 个新识别关键接口：LSP/IDE 查询/调试信息/FFI 边界/增量编译查询/编译器即服务/多目标后端/包管理/AI 辅助 + 效应扩展/能力委托增补 → 完整预留矩阵 14 项 → P0-P3 优先级 + ~3 周成本核算 + 全景图）；§9.2 边界调和（LSP/FFI 重分类——「完全推迟 = 未承诺采用」语义精确化）；§7.1 矩阵 4→14 项；§21.9 预留成本核算；原则 32「预留留白」；附录 E.8 + F 第八轮登记
- **lang-design v6.1**（八文件）：13 主体（§1.1.1 矩阵 4→14 + §3.2 调和 + §3.3-§3.5 完整审查 + 测试锚点注记）；12 §2.9 成本核算 + Phase 4 更新；10 LSP 重分类注记；08 CodegenBackend 预留注记；15 预留层注记 + 查询双预留互链；17 原则 32 + 附三；18 §5 术语 13 条；19 §6 引用 3 条；00 v6.1 修订记录；kerf/docs/stage0.md 存档再生成 v6.1
- **sop v11.3 → v11.4**：§2.2 三十二条（第 32 条预留留白 + 违反示例）；新增 §21.12 接口预留时机（能力引入时机 vs 数据结构冻结时机叠加规则 + §13.1/§6.2 双接线 + 成本口径）；§21.11 风险表「接口预留过多/不足」行对账（四项 → 14 项矩阵清单）；§8.4.5 查询表 + 接口预留行；§16.1 v11.4 行
- **审计**：关键词矩阵 74/74（stage0）+ 29/29（lang-design 全集）全命中；LSP/FFI 陈旧行 0；全 20 文件锚点 GitHub 算法校验零坏链；三面原则编号链一致（29-31 → 32）

### 交付二：预留层代码冻结（reserved/ 模块 4→14 项，P0 位置跨 crate 断言）

- **重构 reserved.rs → reserved/ 模块目录**（§8.4.6 落位 + 既有四接口零迁移）：`mod.rs`（既有 4 项 + 文档 v6.1）+ `toolchain.rs`（P0 三项：LanguageService/IncrementalAst + DebugInfoGenerator/DebugTraceable + QuerySystem/Query；P1：CompilerService/Serializable；P2：PackageManager/ExternalModule/AiAssistant）+ `ffi.rs`（P1：ExternalType/FfiCall/FfiBoundary——GC pin/unpin 隔离协议 + 自举合规注记）+ `codegen.rs`（P1：CodegenBackend/WasmBackend——目标中立类型级证明）
- **P0 数据结构位置对账**（集成断言 12 项）：每节点 Span ✓（CoreExpr::span()）/ 绑定作用域 ✓（NodeMetadata.scopes）/ 稳定 ID ✓（ir::NodeId）/ 函数边界 ✓（BcProgram::protos）/ 调试帧槽 ✓（VM ext3）/ 内容寻址口径统一 ✓（QueryDescriptor ↔ CacheKey）
- **测试 +20（480 → 500）**：Probe 冻结 8（单元：toolchain 3 + ffi 3 + codegen 2——「测试实现体编译通过 = 契约冻结」先例沿用）+ 跨 crate 位置断言与形状行为 12（集成 tests/v0/stage1/plan/reserved_ext_tests.rs，经 runner.rs 总入口挂载）

### 交付三：质量口径

- cargo clean + build --release / check / fmt --check / clippy --all-targets -D warnings 全绿；**test --release --workspace 500:0:0**（单元 183 + 集成 317）；审计集 §7.3.1 配比满足 EXIT 0；CLI 冒烟 fib 75025 + kerf test 3/3
- 语义核心冻结零变动（CoreExpr/9 原语/操作语义）；既有四预留契约签名零迁移（reserved_signatures_are_frozen 先例测试保持）
- 下一步（批次 E，plan §5）：Expander kerf 重写 + TD-004 scope-set 收口 + TD-021 hof 用户面注入 → Stage 1 门审查（§7.3 + §21.3 四条）
---

## v0.3.0-r11（2026-09-10）——吸收完整性复核轮 + 核心原语三面同步（docs / web / 打包）

### 交付一：next*.md 三源吸收完整性复核（审计轮，零实质缺口确认）

- **三源关键词矩阵 + 章节结构映射审计**：next.md（4 文档：超越 Lisp 范式替代方案 / 五大能力历史与创新原则 / 2026 深度设计 / Stage 0 完整能力模型）→ stage0.md Part II §4 + Part III（拆分至 lang-design 14/13/16）；next2.md（六轮：8vs9 真相 / 前沿全景 / 四层正交 / 最终修正 / 可行性 / 表面语法）→ stage0 §6.9-§6.12 + lang-design v5.5；next3.md（= next2 六轮 + 第七轮内部语法重构）→ stage0 v6.0 §7.3/§7.4 + lang-design v6.0——**全部特征概念在吸收面命中**（三原则 / CoreExpr ADT / 迁移映射 / 派生关键词批判 / 9-8-7 真相 / 四层正交 / 六 IR / MLton 闭包 / Koka 效应消除 / de Bruijn / continuation 类型安全 / comptime / S 表达式决策 / 皮肤骨架分离 / 联合交叉类型 / 行多态 / Janet/Céu 归入 PEG/同步语言概念族）
- **存档对账**：kerf/docs/stage0.md（上游蓝图存档）与 upload/stage0.md 同为 v6.0（+496B 归档说明头，预期设计）；sop v11.3 三十一条（29-31）+ §16.1 变更日志 + §8.4.6 测试入口架构意图全部在位

### 交付二：核心原语相关变动三面同步（用户指令专项——P2 文档债修复）

- **kerf README.md**（§8.4.5 文档随代码——r6-r10 陈旧对账）：版本 v5.4/v11.1 → v6.0/v11.3；状态行批次 D → r9/r10 吸收审计轮；架构行补 能力参数化 / 内部效应 / kerf test；特性清单补 **核心原语演进登记**（9 冻结 + 8 原语 Stage 2 语义等价映射 + 三原则）+ 自举 Reader（r6）+ 能力门控 I/O（r8）+ 内部效应（r8）四条；质量状态 r7/476 → r10/480（runner.rs 单一总入口口径）
- **develop plan.md**：批次表新增「吸收/审计轮」行（r9 测试入口重构 ✅ + r10 next3 双层吸收 + 架构合规审计 4 测试 480:0:0 ✅ + 语义核心冻结零变动 + 三面同步验收）
- **stage-0 status.md**：测试总数 356（r6 陈旧）→ 480 全绿（r5-r10 增量分项）；§5 新增架构合规审计测试与核心原语演进登记两条
- **web（kerf 官网）**：能力区新增「核心原语 · 语义核心」主内容区块——9 冻结原语网格（名称/元数/语义，与 lang-design 01 §2 最终定义逐项对齐）+ 核心原语演进登记卡（§6.12.6 收敛裁定 / 9-8-7 数量真相 / 9→8 迁移映射表 8 行 / 内部语法三原则 / 四层正交 + 第五正交轴 / 架构合规审计 4 测试）；路线图 Stage 2 补核心原语演进迁移要点；此前 v6.0/v11.3/r10 版本口径与 footer 包清单已同步（r10 轮交付，本轮复核确认）

### 质量口径

- §3.2 全绿（本轮实跑）：clean + build --release 9.37s ✓ / check 0/0 / fmt 零 diff / clippy --all-targets -D 0 / test --release --workspace **480:0:0**（单元 175 + 集成 305——--workspace 口径，根 package 单跑为 305 属预期组织形态）
- 审计集 41 case EXIT 0（§7.1.1 七类全覆盖）；CLI 冒烟：fib 75025 + ⇒ 144 + kerf test 2/2 通过

### 下一步（批次 E，plan §5）

Expander kerf 重写 + TD-004 scope-set 收口 + TD-021 hof 用户面注入 → Stage 1 门审查（§7.3 + §21.3 四条验收）。
