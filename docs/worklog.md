# kerf Worklog（sop.md §8.6 协议）

> 所有 Agent（主 + 子）共享。同步镜像：`docs/worklog.md`。

---
Task ID: 3
Agent: Super Z (main) — ARCH-A
Task: 按 sop.md §8.4.6/§18.5 脚手架 kerf Cargo Workspace

Work Log:
- 根 Cargo.toml：[workspace] 9 成员 + 根 crate（承载跨 crate 集成测试与 CLI）
- 9 个 crates/kerf-* Cargo.toml（依赖 DAG 见 docs/graph/pipeline/data-flow.md）
- 目录树：tests/v0/stage0/{plan,gate} + common + examples + benchmarks + docs 全树
- scripts/rust/setup.sh + 归档文档（§3.4）
- 依赖定向裁定：§18.5 图 L6→L7 边按 §2.4.5 Layer 0 裁定为 kerf-vm → kerf-runtime；
  Op 定义于 kerf-compiler（VM 为消费者）

Stage Summary:
- Workspace 落成；零外部依赖策略（自举信任根显式化）
- 交付物：Cargo.toml ×10 + 目录结构 + 环境脚本

---
Task ID: 4-a
Agent: Super Z (main) — DEV-A
Task: 实现 kerf-span + kerf-syntax + kerf-core

Work Log:
- kerf-span：Span（字节偏移主键 + 展开代次）/ SourceMap（行/列派生渲染）/
  Diagnostic（E0004 级结构化 + 摘录渲染）
- kerf-syntax：SymbolTable（NFC 一次归一化 + 关键字预内部化）/
  ScopeSet（有序去重 + 并集/子集）/ Stx 语法对象（datum/span/scopes/phase）
- kerf-core：CoreExpr 9 正交原语（span 全节点）+ 图 IR（Arena + node_map
  字面量共享 + find_shared）+ CodeValue（良构性绑定感知 + α 重命名 + 组合预留）
- 修复：`render(impl Fn)` 递归类型无限展开（改 &dyn Fn）；
  doctest 误编译设计文档代码块（改 ```text）；NFC 组合顺序 bug

Stage Summary:
- 32 单元测试全绿；接口契约冻结（Span/Symbol/Stx/CoreExpr/IR/CodeValue）

---
Task ID: 4-b
Agent: Super Z (main) — DEV-A
Task: 实现 kerf-reader + kerf-expander（宏系统/相位分离/卫生）

Work Log:
- Reader：类型化 Token（41 种类 + Operator 携带 Symbol 句柄）/
  词法器（字符串跨行/贪心数字拒绝/嵌套块注释/省略号/Unicode）/
  递归下降语法器（'x 简写 → (quote x)；括号三态错误带 Span）
- Expander：9 核心形式 + §3.2 全部糖推导（let/let*/letrec/cond/and/or/
  when/unless/while）+ syntax-rules（模式/字面量/省略号/模板实例化）
- 相位分离：ModuleRegistry declare/visit/instantiate（传递依赖先行）
- **同类路径审查（§20.3）修复簇**：
  lambda 体铺平（4 处构造点）/ letrec 与提升路径 nil 包裹 /
  while 递归裸符号 / 卫生一致性（同标识符→同一重命名符号）/
  宏自引用保留集 / 核心关键字重命名豁免
- 展开深度上限校准 128（rustc 默认对齐；TD-007 迭代式解除计划）

Stage Summary:
- 46 单元测试全绿；卫生保证 P1 完整（宏内外同名不串扰经集成测试验证）

---
Task ID: 4-c
Agent: Super Z (main) — DEV-A
Task: 实现 kerf-compiler + kerf-runtime + kerf-vm + kerf-driver

Work Log:
- Compiler：39 操作码冻结 / 跳转回填（占位队列 + 完备断言）/
  常量池去重（float 位键）/ 闭包捕获描述符（CaptureSource）/
  debug_info 逐指令 Span / 栈平衡（SetBang DUP 后存储）
- Runtime：HeapObj 全类型装箱（Pair/Str/Int/Float/Bool/Nil）/
  标记-清除（显式工作栈 + free-list 重建 + 复位对称）/
  分配计数驱动触发 + foreign 根
- VM：迭代式主循环（帧栈承载递归——Rust 栈恒定）/
  帧三扩展槽（ext1/2/3 格式冻结）/**共享单元格捕获协议**
  （帧局部槽 Rc<RefCell> 化——letrec 递归与可变捕获语义正确）/
  GC 冷却退避（深递归 O(n²) 消除：122s→3.4s）/ 堆栈追踪
- Driver：全管线编排 + 相位生命周期接线 + 内置函数 20 项 +
  卫生回退解析（$hyg$ 后缀剥离）+ RunOutcome（值+存活堆）
- **关键修复**：CLOSURE 原型切换（current 栈）；
  捕获值快照→共享单元格重构（正确 > 妥协）

Stage Summary:
- 49 单元测试全绿；双执行路径 VM/eval 结果逐字节一致（互查验证）

---
Task ID: 4-d
Agent: Super Z (main) — DEV-A
Task: 根 crate CLI + 集成测试 + 接口预留

Work Log:
- CLI 10 子命令（run/eval/check/tokens/stx/core/ir/bc/code/bench）
- tests/v0/stage0/{plan,gate} 7 套件 73 项 + common 辅助
  （Cargo.toml [[test]] 显式声明 + #[path] mod 修正）
- gate_review_r1：§22.3 清单 12 项可执行审计
- 预留层：EffectFamily/Effect/EffectSystem + MultiStage +
  ReadCapability/WriteCapability/CapabilityIO + CacheKey/CachedResult/
  CompilationCache（行为规格完整，Probe 实现证明冻结）
- examples 6 个（fib/closures/macros/higher_order/gc_stress/io）

Stage Summary:
- 200 项测试全绿（单元 127 + 集成 73）

---
Task ID: 7
Agent: Super Z (main) — QA-A
Task: §3.2 交付前验收

Work Log:
- cargo clean → cargo build --release：✅ 0 警告
- cargo check：✅ 0 errors / 0 warnings
- cargo test --release --workspace：✅ 200 passed / 0 failed
- cargo fmt --check：✅ exit 0 零 diff
- cargo clippy --all-targets -- -D warnings：✅ 0 警告
  （修复 18 项 clippy：Default/needless_lifetimes/let_and_return/
  const thread_local/redundant closure/type_complexity/
  is_multiple_of/saturating_sub/result_large_err/len_zero 等）
- 基准：fib(25)（含编译）84.7 ms/轮（release × 5）
- Git commit：feat(stage0)——12 能力模型全量落地

Stage Summary:
- §3.2 全绿；验收数据已记录 docs/develop/v0/stage-0/gate-review.md

---
Task ID: 6
Agent: Super Z (main) — PM-A/REV-A
Task: 工程文档树（§8.4 全量）

Work Log:
- docs/develop/v0/stage-0/{plan,status,dev-log,gate-review}.md
- docs/develop/v0/{tech-debt-register,calibration-data,
  v0.1-capability-boundaries,v0.5-roadmap}.md
- docs/tests/{README,matrix,pipeline-test-coverage}.md +
  v0/stage0/{plan ×6, gate ×1} 双向印证文档
- docs/graph/pipeline/data-flow.md（管线数据流 + 依赖图 mermaid + 定向裁定）
- docs/scripts+rust / docs/tools/rust（setup 文档）
- docs/vm/README + docs/backend/README（Stage 2+ Reserved）
- docs/build-guide.md + docs/testing-guide.md
- docs/stage-committee-process.md（sop.md v11.0 全文）
- 根 README.md + RELEASE_NOTES.md

Stage Summary:
- 文档树符合 §8.4.1/§8.4.4（元数据头/相对路径交叉引用/mermaid）
- 阶段门审查结论 PASS（§21.5 切换信号满足）

---
Task ID: 9
Agent: Super Z (main)
Task: §19 阶段打包 + 最终验证

Work Log:
- tar -czf download/kerf-stage0-v0.1.0-stage0.1-process-doc-v11.0-12-capabilities-r1.tar.gz
  --exclude target/.git → 286,936 B 压缩 / 1,034,240 B 原始（>1MB 验收口径）/ 165 文件
- Web 端到端自验证通过（Playground 执行本二进制：fib/宏/文档/下载全链路）
- 最终 git commit（交付基线）

Stage Summary:
- Stage 0 交付闭环：验收全绿 + 门审查 PASS + 交付包就位

---
Task ID: 10
Agent: Super Z (main) — ARCH-A（三路并行审查：T10-a/b/c 子代理）
Task: 对照 stage0.md 全文审查 docs/lang-design/ 20 文件缺口矩阵

Work Log:
- 三路并行精读：T10-a（01–05 核心语言组）/ T10-b（07–12 策略路线组）/
  T10-c（13–19 矩阵参考组），逐文件对照源章节
- 缺口矩阵汇总：P0×1（06 操作语义空壳——源 §13.4 本身仅一句承诺的
  继承性欠账）、P1×2（03/05 接口契约系统性缺失）、P2×7
  （01–05 P0–P4 标注缺失、02 职责矛盾、04 操作码漂移、
  19§2 系统性误引×3、18 计数、15 双入口注记、重复关系未声明）
- 断链机械验证：全库链接 0 错误（拆分忠实度的最大优点确认）

Stage Summary:
- 缺口矩阵完整产出；修复优先级排序确定（P0 理论欠账优先）

---
Task ID: 11
Agent: Super Z (main) — DEV-A
Task: lang-design v5.1 收敛完善（T11-a/b/c/d 四 MUV）

Work Log:
- T11-a：06-operational-semantics 从 21 行空壳重写（~200 行）：
  语义域定义（值/环境/堆/求值状态）+ 9 原语小步归约规则组
  R1–R9（含 βv/δ 辅助规则）+ 错误吸收语义 E0–E8 + GC 不可观测性
  引理 L-GC + 编译正确性定理 T1（L1–L4 三引理证明纲要 + 适用边界）
  + 归约规则 ↔ 测试锚点映射表 + 演进义务（核心冻结承诺）
- T11-b：03-macro-system 契约回填（81→245 行）：§1.1 ModuleRegistry
  簿记契约（declare/visit/instantiate + ModuleEntry + 幂等/先行次序
  不变式）、§2.1 Transformer/TransformerKind、§2.2 ExpandCtxt +
  MAX_EXPANSION_DEPTH=128（TD-007 校准注记）、§2.3 syntax-rules 文法
  （pattern/template/省略号维度规则）、§2.4 HygieneCtx 契约 + 卫生
  三重保证；§4 不变式 4（同类路径审查六项修复簇）；§5 测试锚点表
- T11-c：05-runtime 契约回填（73→176 行）：§1 I/O 通道契约
  （write_line_stdout/read_line_stdin + CapabilityIO 升级路径）、
  §3.1 Heap/GcRef/Slot/HeapObj 分配器契约（六类型化分配入口 +
  register_foreign_ref 非 no-op 裁定 + GC 冷却退避注记）、
  §3.2 RootSet/mark_sweep_cycle 契约 + 根集四来源；§5 测试锚点表
- T11-d：01（import_spec/export_spec Stage 0 裁定定义 + 06/17 锚链 +
  测试锚点）、02（职责矛盾调和：词法层 vs Reader 模块两级裁定 +
  Stage 0 实现落点 + 五辅助类型指向 + 测试锚点）、04（操作码漂移
  注记 + CodeBuf/CaptureSource 契约 + CLOSURE 协议定案 + 双路径互查
  链接 + ext1→EffectSystem 预留链 + 测试锚点）、09（20 项内置对账 +
  头部标注）、10/18/15（断链修正 §3.5/§3.6 + 计数 42 + 双入口注记 +
  §3.4 欠账补齐声明）、11（Week 2 核心形式判据 + 200 项落地对账）
- 00-overview：v5.1 修订记录 + 06 文档地图行更新
- 链接完整性终验：全库 0 断链（含 § 级引用抽查 15 处全中）

Stage Summary:
- 遵循原则：§2.1.1-11（确定性边界：先判 P 级再动笔）、§2.2 原则 8
  （语义形式化）、§2.3-8（设计驱动测试：每文件测试锚点节）、
  §2.3-9（正确>妥协：06 从实现反向提炼而非保留空壳）
- lang-design 4134→约 4900 行；全部 20 文件含处理程度标注（01–06/09–11）

---
Task ID: 12
Agent: Super Z (main) — REV-A → DEV-A
Task: 依据完善后 06 操作语义审查 Stage 0 实现 → 双路径语义分裂修复

Work Log:
- 审查发现（依据 06 §2 R5/R6 + §3 E3/E6 + §5 T1）并经 CLI 实证
  （eval vs run 三组 case 全部实锤）：
  (1) Define 返回值分裂：eval 返回 v / VM 返回 nil（T1 反例）
  (2) set! 未绑定全局：eval 报 E3 / VM 静默创建（T1 反例）
  (3) 重复 define：双路径均静默覆盖（E6 双侧缺失）
  (4) lambda 形参表重名：双侧无检查（A3 卫式缺失）
- 修复（通解>特解 §2.1.1-4；正确>妥协 §2.3-9）：
  - opcode.rs：新增第 40 号冻结契约 DefineGlobal(u32)（D1/E6 语义）
  - compile.rs：Define 编译模式改为 value;DUP;DefineGlobal
    （返回值 = v，与 eval 对齐；移除 PushNil）
  - vm.rs：StoreGlobal 严格化（S1/E3：未绑定报错不再静默创建）+
    DefineGlobal 实现（E6：同层重复报错）
  - eval.rs：Env::define 返回 bool（同层新增语义；跨层 shadowing
    合法）+ Define 分支 E6 报错 + apply_value 参数重名报错
  - expander.rs：parse_params 重名检查（展开期单点防御——两路径
    共同上游）
  - driver.rs：root.define 宿主信任注记
- 测试：4 组新增（define_returns_value_dual_path /
  duplicate_define_errors_dual_path / set_unbound_errors_on_vm /
  lambda_duplicate_params_rejected_at_expand）；修正 2 处依赖旧
  错误语义的断言（compile 单测字节码快照 + gate_g4 源码先 define
  后 set!——去除兼容思维 §2.3-5）
- 文档回填：04（39→40 操作码 + DefineGlobal 契约注记）、06
  （40 操作码）、vm/README、data-flow 图
- 验收：204 项测试全绿（200 基线 + 4 新增）/ fmt 零 diff /
  clippy -D warnings 零警告

Stage Summary:
- T1 定理三处直接反例清零；双路径全局存储语义一致（define/set! 
  分指令承载）；E0–E8 错误全集双侧对齐
- 遵循原则：§2.1.1-4（通解）、§2.3-4（报错>静默）、§2.3-9
  （正确>妥协）、§9.4.3（正负测试 1:3——4 组负例 + 正例对账）

---
Task ID: 13
Agent: Super Z (main) — QA-A / REC-A
Task: §3.2 终验 + web 同步 + §19 打包（v0.1.0-r2）

Work Log:
- §3.2 全绿：cargo build --release（0 警告）/ test --workspace
  204 passed / fmt --check 零 diff / clippy -D warnings 零警告
- web 同步：kerf-data.ts（40 操作码/204 测试/操作语义 R1–R9 badge/
  semantics 示例）；docs API 动态读 lang-design（v5.1 自动同步）；
  matrix/status 计数对账
- web 交互缺陷修复（Agent Browser 发现）：文档内相对链接整页跳转
  404 → MarkdownView onDocLink 拦截（data-doc-link 委托 + 白名单
  正则，外部链接 target=_blank）
- Agent Browser 端到端自验证：Playground（(define x 5) ⇒ 5 新语义 /
  E6 重复定义报错带 Span / E3 set! 未绑定报错）+ docs（v5.1 修订
  记录 + 06 归约规则渲染 + 链接拦截）+ 响应式（390×844）+ 零
  console 错误
- §19 打包：r2 包 312KB / 165 文件（exclude ./target ./.git 修正
  后）；**包内解压自举验证 204 测试全绿**
- RELEASE_NOTES v0.1.0-r2 章节 + worklog 镜像

Stage Summary:
- 遵循原则：§3.2（交付前实际运行验收）、§9.4.3（正负测试）、
  §19（打包规则）、浏览器验证标准（非"编译通过"即完成）
- v0.1.0-r2 交付闭环：lang-design v5.1 收敛 + T1 定理三反例清零 +
  204 全绿 + web 同步 + 自举验证包

---
Task ID: 14
Agent: Super Z (main) — ARCH-A（事实采集：T14-a/b/c/d 四路子代理）
Task: §14.5 阶段末深度审查 D1-D8 + §14.8 设计偏差清单（Stage 0 切换点强制轮）

Work Log:
- 四路并行事实采集（全部只读，结论附 file:line 证据）：
  T14-a 架构：依赖图 24+9+1dev 边 DAG 无环零外部依赖；§14.7.2 六项
  5 PASS + B4 根 CLI 直调 reader 待裁定；生产 unwrap=0/expect=11 全带
  理由/catch-all 42 处（19 无注释，vm.rs:648 唯一生产静默空臂）；
  TODO 零；//! 头 37/37；TD-002~011 全开放（登记册缺 TD-001/006 去向+3 详情）
  T14-b 测试：204/0/0 全绿复验（127 单元+77 集成）；正负比实测
  154:37=1:0.24（三口径），7 分类负测为零；§7.1.1 矩阵 5/7（缺空
  应用/循环依赖）；E4/E7/E8/E0002-4 零直接断言；§7.3.1 审计集不存在；
  matrix.md 表列失真（合计 200 vs 头 204）；pipeline-test-coverage.md
  结论夸大+缺 §14.6.1.1 小节
  T14-c 偏差：10 检查点对照 → 26 项偏差（B1×2/B2×4/B3×13/B4×7）
  + 10 大项确认一致；最重：opcode.rs 冻结守护测试断言 39/漏
  DefineGlobal/注释 41（P1）；App 求值顺序双路径分裂 eval fn 先 vs
  compile 参数先（P1，T1 反例面）
  T14-d 性能文档：fib(25) 84.443ms 复现 ✅；GC 单轮 0.17s（声称
  0.72s 陈旧夸大）；clean 构建 5.75s/二进制 1.01MiB；docs 47 文件
  45/47 元数据合规；worklog 镜像 diff=0；LOC 10573（+51）；重复
  setup.md×2
- 产出 docs/develop/v0/stage-0/deep-review-round1.md：D1-D8 三段式、
  委员会投票 0/5.5 NEEDS REVISION、行动计划 A/B/C/D、偏差清单 26 项
  + 无偏差确认清单

Stage Summary:
- 结论 NEEDS REVISION：P1×4（正负比/审计集缺位/守护测试失效/App 顺序
  分裂）+ P2×8 + P3×5；gate R1 的 PASS 因 §7.3.1 强制失败条件判定无效，
  修复后须重跑 Gate R2
- 遵循原则：§14.5（切换点强制深审）、§9.4.3/§7.3.1/§7.1.1（负向三
  条款实测裁决）、§2.3-9（App 顺序判设计优）、§8.4.5（先查文档实测
  取证）、§14.8.3（偏差清单不可省略含"无偏差"记录）

---
Task ID: 16
Agent: Super Z (main) — DEV-A（执行：主会话 + 16-a/b/c/d 四子代理）
Task: 修复内循环——deep-review R1 的 P1×4 + P2 项修复（App 顺序/
opcode 守护/审计集/负测扩张 1:3/循环依赖/堆栈追踪/eval 卫生回退/
driver 转发）+ 文档对账回写 + examples 重组 + §3.2 全绿

Work Log:
- A1 App 求值顺序统一（06 §2 A1 函数先）：compile.rs fn 先求值 + 
  vm.rs CALL 弹参/弹 fn 对调 + 3 处快照断言更正（含测试改名
  app_evaluates_fn_then_args）+ 新增双路径负例
  app_evaluation_order_fn_first_dual_path（span 判定 fn 位 1:3 实证）
- A2 opcode.rs 冻结守护测试重写：40 项显式枚举（8 组标注）+ 模块头
  分组注释补 DEFINE_GLOBAL（修复断言 39/漏 DefineGlobal/注释 41 三失真）
- A3 审计集（16-a）：examples/audit/stage0_gate_audit_r1.rs 41 case
  （单语句 12/多语句 12/复杂 8/恢复 6/正向 3；负向 32≥22）+ Cargo.toml
  [[example]] + §7.1.1 七类全覆盖 + 配比机械校验；**审计集发现 3 缺陷
  并当场修复**：C07 循环依赖栈溢出（phase.rs visit DFS 灰标记 → 
  Err「模块循环依赖：Symbol(N) → …」+ 菱形合法）+ C03 堆栈追踪未实现
  （run_program 拆 execute 内层 + 错误路径附加最内 16 帧调用点 note）
  + C08 eval 缺卫生回退（driver.rs resolve_eval_hygiene_fallbacks 
  镜像 VM 侧语义，宏→内置双路径一致 42）
- A4 根 CLI reader 直调改 driver 转发：dump_tokens/dump_stx 公共 API
  + main.rs 改调用（§14.7.2 B4 全 PASS）
- B 负测扩张（16-b 超时但产出 + 主会话补齐）：四文件表驱动
  negative_reader(60 case)/negative_expander(98)/negative_vm(231)/
  negative_semantics(100)——负向 case 总计 ≈523 vs 正向 ≈160，
  **正负比 1:3.3（§9.4.3 达标）**；7 分类零负测清零；E0001-E0004/
  E1-E6 直接断言矩阵；16-b 语义发现存档（FS-3/4/5：链式比较短路、
  read-line 元数不校验等）
- C 文档对账（16-d 超时前完成 29 文件 676 行）：04 全 40 枚举/05 
  HeapObj 六变体+五来源+GC 数值冻结/06 Unit+App 顺序确认/09 24 内置/
  13 reserved.rs 签名回填+能力 IO P2/15 七层→九 crate 映射/03 循环检测
  与 Stage 0 裁定/02 Token 计数+NFC 边界/12 GC 口径+P2/matrix 290 全
  套件对账/pipeline-test-coverage §9.5.1+§14.6.1.1 重写/negative-tests.md
  新建/README/status/RELEASE_NOTES r3/TD-012/013/014 新增+TD-001/006
  去向+3 详情/dev-log r3/00-overview v5.2 修订记录
- C1 卫生（16-c）：14 处 catch-all 臂级注释（41 生产臂全量裁决，
  vm.rs GC 静默空臂闭环）
- examples 重组（§9.6）：6 个 .krf → usage/ + README 索引重写 +
  web stats API 路径同步
- §3.2 全绿（clean 后实测）：build --release 0 警告 / check 0/0 /
  test --release --workspace **290 passed 0 failed 4 ignored** /
  fmt --check 零 diff / clippy -D warnings 零警告
- 审计集 release 复跑：41/41 PASS（XFAIL-WARN 0——三项修复经审计
  case 自动升级为严格 PASS 验证）

Stage Summary:
- deep-review R1 全部 P1 清零：正负比 1:0.24→1:3.3、审计集 0→41 case、
  App 顺序 T1 反例面清零、opcode 守护恢复；审计集驱动的三额外缺陷
  （循环依赖/堆栈追踪/eval 卫生回退）一并修复——§7.3.1 教训
  「覆盖广度而非数量」的实证
- 测试 204→294 函数（290 通过+4 文档化忽略）；LOC 10573→10806；
  docs 47→48 文件（negative-tests.md）
- 遵循原则：§2.3-9（正确>妥协：App 按设计修编译侧）、§9.4.3/§7.3.1/
  §7.1.1（负向三条款达标）、§7.3.1 规则 4（审计集比上轮更大：
  0→41）、§14.9-C1（catch-all 全裁决）、§8.4.5/§14.8（文档以代码
  为准回写）、§5.3 八项硬条件全满足（P0/P1 清零+集成测试+文档同步
  +fmt/clippy/test 全绿）
- 待办：16-d 超时未及 worklog 落款（其产出经主会话验收采纳：29 文件
  修改全部生效、计数与实测一致）

---
Task ID: 15
Agent: Super Z (main) — ARCH-A/QA-A（执行：T15-a/T15-b 并行子代理 + 主会话收尾）
Task: §14.6 阶段间深度验证——四项强制审查 + §14.6.2/14.6.4 输出 5 份文档

Work Log:
- T15-a：architecture-review.md（7 阶段×5 维度：31✅/4⚠️/0❌；DAG 实证/
  设计对齐 7/7；⚠️= expander 1351 行 TD-012/vm 1223 行/操作码 6 处同步
  点/IR 旁路）；refactoring-optimality-review.md（7 项重构全判最优+治根，
  0 hack；数据结构 5 项三维达标；新发现 O-1）；performance-baseline.md
  （8 指标实测：clean 构建 5.96s/二进制 1.021MiB/fib(25) 88.8ms（+5.1%
  带内观察）/GC 160.4ms（-5.6% 改善）/测试 13.6s；口径说明 6 条）
- T15-b：design-impl-test-coverage.md（133 设计点三列对照，DEFERRED 
  5 项=3.8%≤5% ✅，73 处锚点抽验零编造；差距 2 项 P3）；hidden-
  problems-assessment.md（18 项评估：≥2× 5 项均为 roadmap 冻结的计划内
  偿还（四证裁定）；GO-WITH-CONDITIONS 四条件）
- 主会话收尾修复（§14.6.1.4 规则 4 裁定项落地）：
  - FS-1 修复：MAX_NESTING_DEPTH 10_000→256（实测 600 层溢出 2MiB 栈
    →留 >8× 裕度）；ignored 测试激活 + 255 层边界正例
  - 糖正向锚点补齐：derived_forms_sugar_expansion_anchors（let* 嵌套
    lambda/when/unless 推导快照——T15-b 差距项清零）
  - FS-5 显式裁定：09-stdlib §2 链式比较短路语义段 + TD-016 登记
  - O-1/TD-015 登记（IrGraph 无条件计算，Stage 1 切换期与 TD-012 同批）
  - 三文档计数同步（292 通过/295 函数/3 忽略）
- 全绿维持：test 292:0:3 / fmt 零 diff / clippy -D 零警告

Stage Summary:
- §14.6.1 四项强制审查完成：①完整性（pipeline-test-coverage §4 小节，
  16-d 产出）②架构（0❌）③三者覆盖（3.8% DEFERRED 合规）④隐藏问题
  （计划内 vs 隐藏问题四证裁定成立）
- §14.6.2/§14.6.4 文档齐备；§14.6.6 输出集合尚缺 final-assessment.md
  （Task 17 产出）
- 遵循原则：§14.6.1（四项强制审查依次完成）、§14.6.1.4 规则 4（FS-1
  当轮修复）、§8.4.5（计数实测对账）、§2.3-9（糖锚点/守卫修复不妥协）

---
Task ID: 17
Agent: Super Z (main) — REV-A/ALG-C→DEV-A→PM-A（T17-a 独立深挖子代理）
Task: §14.6.3 多轮深挖（第 4 轮对抗）+ 六 P1 修复批 + §6.3 外循环投票
+ Gate R2 重跑 + final-assessment

Work Log:
- T17-a 对抗深挖（57 探针双路径逐字节对比 + 读码）：发现 D1（begin
  包裹 define 全局泄漏 vs 词法——T1 反例）/D2（eq? 字符串指针比较
  分裂——T1 反例）/D3（eval 深递归 abort）/D4（i64::MIN 除模 panic）/
  D5（(+) 越界 panic）/D6（宏调宏未绑定——能力洞）六 P1 + D7 P2 +
  D9/FS-1 P3；同时确认 8 面一致（闭包捕获/letrec/数值塔/字符串渲染/
  quote 深结构/if 分支/GC 不可观测/宏卫生）
- 六 P1 + D7/D9/FS-1 全部当轮修复（附回归测试）：
  D1→compile.rs Define 仅入口原型（E0003 结构化拒绝，两路径共享
  compile_source 双侧一致——产生首个用户可触发 E0003）
  D2→value.rs eq_value Str 内容比较（L4 即时值按值）
  D3→eval.rs MAX_EVAL_DEPTH=256（实测标定：2MiB 线程栈物理极限
  ≈690 深度单元/8MiB≈1100 程序层；4096 实测仍先溢出后校准至 256）
  D4→builtins checked_div/checked_rem
  D5/D9→单位元 (+)=0 (*)=1、(- x) 取负、/ 与 mod 至少 2 参
  D6→expander.rs 变换器基名回退（hygienic_base_symbol——卫生穿透
  落地，宏调宏双路径 42）
  D7→apply_value 签名重构（EvalError 保真透传，无前缀累积+最内 Span）
  FS-1→MAX_NESTING_DEPTH 10_000→256（实测 600 层溢出 2MiB）
- 文档同步：06 r3 补充引理（D1/D3/D6/D7 锚定）/09 算术元数与 eq?
  语义/03 卫生穿透回退节/matrix 297/status/RELEASE_NOTES T17-a 段/
  TD-017（eval 深度不对称）/TD-018（消息文本分裂）
- §3.2 全绿（clean+release）：build 6.40s 0 警告/test 297:0:1/fmt/
  clippy/审计集 41 case EXIT 0
- 产出：deep-review-round2.md（R1 P1×4 + 深挖六 P1 全清零，§6.3
  加权 5.5/5.5=100% 通过，GO）/gate-review-round2.md（R1 判定作废
  重跑 PASS）/final-assessment.md（§14.6.6 集合核对+四轮深挖记录+
  GO 判定+Stage 1 规划输入）

Stage Summary:
- 判定 GO：P0/P1 存量 0；四轮独立深挖收敛（连续两轮无新 P1）；
  外循环 100% 通过；阶段切换信号全部满足
- 测试 292→297（D 系回归 + D6 宏调宏）；ignore 3→1（D3/D4 修复激活）
- 遵循原则：§14.6.3（多轮独立深挖——覆盖广度实证：两轮共发现 10 P1
  全修复）、§14.5.3（P1 本阶段修复不带入）、§6.3（外循环加权投票）、
  §2.3-9（D1/D2 按设计修实现侧）、§2.1.1-12（D3 守卫先实测后定值，
  4096 首测失败再校准——真实设计 vs 临时桩的判定留痕）

---
Task ID: 18
Agent: Super Z (main) — REC-A（断档补落款）
Task: r3 打包追认落款 + SOP 文件更名同步 + §3.2 前置清障

Work Log:
- 断档补记（§8.6 只追加）：final-assessment 预告「打包于 Task 18 执行」——
  download/kerf-stage0-v0.1.0-stage0.1-langdesign-v5.2-gate-audit-r3.tar.gz
  （443,174 B，2026-09-10 生成）实际存在但 worklog 无落款，本条目追认
  该产出归属 Task 18（内容 = Stage 0 r3 交付包：lang-design v5.2 +
  Gate R2 审计 + 297 测试基线）
- 用户指令执行：docs/stage-committee-process.md 重命名为 docs/sop.md
  （与 upload/sop.md 逐字节一致，cmp 验证）
- 引用同步（§8.4.5 文档同步义务）：sop.md ×2（目录树自注 + §8.4.5
  查询时机表）+ README.md ×1；历史记录保留原文不改（sop.md v10.0
  变更日志行、worklog Task 6 历史条目、deep-review-round1.md）
- §3.2 前置清障：cargo check --all-targets 发现 2 处测试残留警告
  （builtins.rs:407 unused `plain` / expr.rs:306 dead `fn resolve`）——
  清除后 --all-targets 0 警告；基线复验 297:0:1 不变（与 final-assessment
  §3 终态指标一致）

Stage Summary:
- SOP 权威文件名统一为 docs/sop.md（本会话起所有引用以此为准）
- 交付门清障完成：--all-targets 0 警告 + 297 基线确认，Stage 1 启动
  前置条件就绪
- 遵循原则：§8.6（worklog 只追加 + 断档补记先例 TD-001/006 同理）、
  §8.4.5（活文档同步/历史保留）、§3.2（0 警告交付门前置清障）

---
Task ID: 19
Agent: Super Z (main) — PM-A/ARCH-A/PL-A
Task: Stage 1 启动规划套件（§21/§13.1/§17/§18/§4 五合一）

Work Log:
- §0 启动协议完成：自我定位声明 7 要素（L3 全量；多角色序列
  PM-A→DEV-A→REV-A→QA-A→REC-A）
- §21 阶段规划确认：目标=自举验证；Rust~70%+kerf~30%；后端=Stage 0 VM；
  验收四条（§21.3）+ 切换信号 4/4 核对通过
- §13.1 设计对齐：lang-design v5.2 十文档 → Stage 1 需求映射（结论：
  无缺口阻塞——TD-004/007/012/015 均有契约锚点）
- §17 七步排版图：Step 1 强制扫描 8 文档（技术债 16 项开放/校准基线
  问题簇计轮/能力边界/测试 297:0:1/GO 判定/v0.5 路线图对齐）；
  Step 2-4 依赖图 DAG 五批次（A 切换期重构 → B 标准库+Reader 重写 →
  C 类型检查器+缓存 → D Effects+能力 → E Expander 重写+门审查）；
  Step 5-7 三位一体节点流 + 缺陷纳入 + 审查通过
- §18 依赖审查四项：基础设施/前置项/其他依赖（3 项非阻塞——栈约束/
  值模型表达力/reserved 签名）/全面审查——结论无缺失项
- §4 MUV 批次 A 六字段表（A1=20-b TD-015 / A2=20-a TD-012 /
  A3=20-c TD-007 / A4=20-d TD-004）；批次 B-E 预留 Task ID 22-25
- 产出 docs/develop/v0/stage-1/plan.md + tests/v0/stage1/plan/ 目录

Stage Summary:
- Stage 1 正式启动：GO 四条件全部映射到批次节点（条件 1→A3/A4；
  条件 2→C3；条件 3→A1/A2；条件 4→C3）
- 遵循原则：§21（规划先行）、§17.2（扫描强制 8 文档）、§13.1（设计
  对齐 10 文档）、§18.1（依赖四项审查）、§4.1（六字段）、§8.4.5
  （决策附条款号）

---
Task ID: 20-b
Agent: Super Z (main) — DEV-A
Task: TD-015 偿还——compile_source 按消费方分流（IrGraph 旁路计算消除）

Work Log:
- 病灶：compile_source 在 run/eval 生产路径无条件 lower_program 后旁路
  丢弃（仅 kerf ir 子命令与 CodeValue 检查消费——后者自 lower）
- 修复（登记册方案 A：拆分编译入口）：
  - 抽取私有前段 compile_front（read → expand → 相位簿记 → 字节码，
    不含 lower）+ FrontOutput 结构
  - compile_source 公共 API 不变（完整管线含图 IR——dump/检查/CodeValue
    消费方出口）
  - run_source/eval_source 改走 compile_front 快路径（生产执行路径
    不再构造 IrGraph）
  - CLI check/core/bc/code 子命令维持 compile_source（§14.9 自调试
    语义 = 完整管线检查，非病灶）
- 新增守护测试 fast_path_bytecode_matches_full_compile：同一源两出口
  字节码逐指令一致（防分流漂移——§7.2 Q2 孤立正确防线）
- 回归：298 通过（297 基线 + 1 新增）/ 0 失败 / 1 ignored；CLI ir/code/
  bc/check 输出不变（disassemble 等价验证于守护测试）

Stage Summary:
- TD-015 已解决（状态开放 → 已解决）；生产路径恒定开销消除
- 遵循原则：§12（最优>最小：单一编译逻辑双出口而非复制管线）、
  §2.1.1-11（确定性边界：病灶定义于登记册后动笔）、§7.2 Q2（孤立
  正确防线——分流守护测试）、§10.1 规则 1（入口自由函数命名不变）

---
Task ID: 20-a
Agent: Super Z (main) — DEV-A
Task: TD-012 偿还——expander.rs 拆分（核心形式/糖推导/相位驱动三职责分置）

Work Log:
- 拆分前基线：expander.rs 1372 行（占 crate 58%——登记册 D1 风险项②）
- 新模块布局（登记册方案落地）：
  - expander.rs（主控，517 行）：公共类型（ExpandCtxt/ExpandError）+
    入口分派（宏→糖→核心形式→函数应用）+ 宏调用展开 +
    构造底座（WORD_* thread_local/init_keywords/kw_symbol/字面量构造/
    make_setbang/is_head_keyword）+ mod tests 整体保留
  - core_forms.rs（511 行，crate 私有）：expand_core_form 分派 +
    lambda/if/set!/define/begin/module/quote/define-syntax 展开 +
    parse_params/expand_body（define 提升路径）/define_name/
    define_value_stx/datum_to_value
  - sugar.rs（426 行，crate 私有）：desugar_let/letrec/let*/cond/and/
    or/when/unless/while 九糖推导 + is_else_symbol
  - phase.rs/macro_sys.rs 不动（相位驱动与变换器机制已独立）
- 可见性裁定（§11 接口隔离）：core_forms/sugar 为 mod（crate 私有）——
  外部仅见 expander 入口 re-export，公共 API 面零变化
- 搬移原则：纯机械搬移（零逻辑修改）；仅可见性 pub(crate) 化 +
  1 处 Ok 包装修正（literal_from_stx 返回值语义保持）
- 回归：298 通过 / 0 失败 / 1 ignored（拆分前后逐一等价，测试零断言
  修改）；clippy -D warnings 零警告；fmt 已应用

Stage Summary:
- TD-012 已解决：expander.rs 1372→517 行（-62%）；crate 2356→2451 行
  （模块头 doc + use 声明的结构性增量）；单文件全部 ≤715 行
- 拆分为 20-c（TD-007 迭代式展开）与 20-d（TD-004 scope-set）提供了
  干净的实施基座（plan.md 批次 A 依赖拓扑 A2→A3/A4 就位）
- 遵循原则：§11（接口隔离：内部模块私有化）、§12.3（重构治根：职责
  分置而非行数切割）、§8.4.5（模块头 doc 职责边界声明）、§2.3-9
  （正确>妥协：测试整体保留主控走公共入口=拆分等价性天然回归）

---
Task ID: 20-c
Agent: Super Z (main) — DEV-A
Task: TD-007 偿还（部分）——宏展开 trampoline 工作表 + 深度上限实测标定

Work Log:
- 设计：expand_form 重构为 trampoline（while let 循环）——宏产物头部
  仍是宏调用（用户宏/糖）时迭代继续不递归；try_macro_step 单步
  （宏头判定含卫生基名回退 + 糖惰性注册 + apply）；ctx.depth 语义
  保持「展开路径宏总数」（入口快照/成功出口回滚——兄弟不累计）
- expand_macro_call 删除（职责并入 trampoline）；expand_list 简化
  为纯核心形式/应用分派；E1 修复：items[1..].to_vec() → &[Stx] 借用
  （消除 trampoline 每步冗余深拷贝）
- 深度上限标定（TD-017 同型实测法，探针 example + 测试线程双环境）：
  - 2MiB 测试线程：1_000 层通过 / 2_000 层溢出
  - 8MiB 主线程（CLI 生产）：4_000 层通过 / 5_000 层溢出
  - 残余栈约束 = Stx 值语义深树 clone/drop 递归（数据结构层，非展开
    控制流）——探针确认 trampoline 后溢出点来自 macro_sys 绑定
    clone（match_datum arg.clone）与产物 drop 递归
  - **裁定 500**（实测通过值 2× 裕度；不变式 1「超限报错而非栈溢出」
    须在环境波动下成立——上限必须落在实测边界内侧）
- 测试：单元 2 例（500 层构造链正例 + 501 层超限负例——构造 Stx
  绕过 reader 256 嵌套限制）+ 集成 4 例（tests/v0/stage1/plan/
  expansion_worklist_tests.rs：200 层链端到端 VM / 双路径一致 /
  无限自指宏 Expand 阶段结构化报错 / 宏链与糖交错）+ Cargo.toml
  [[test]] 声明
- 负例消息同步：128→500（negative_expander_tests 深度超限 case）
- 文档回写：03-macro-system（§2.2 契约 + §4 不变式 1 + §5 锚点表）；
  TD 登记册 TD-007 → 部分解决（残留 10_000 完整口径绑定批次 B Rc 化）
- 回归：300 通过（298 + 单元 2）/ 0 失败 / 1 ignored + 集成 4 例；
  clippy -D 零警告；fmt 已应用

Stage Summary:
- TD-007 部分解决：128→500（3.9×）+ trampoline 工程价值交付（控制流
  栈深与链长解耦）；完整解除依赖 Stx Rc 化（批次 B 前端重写范围）
- 探针失误教训（worklog 透明记录）：sed 链式替换探针两次污染数据
  （替换模式不匹配导致测错值）——改用独立探针 example（参数化 argv）
  修正；最终标定基于双环境可靠数据点
- 遵循原则：§2.1.1-12（守卫先实测后定值——4096 首测失败再校准的
  TD-017 先例重演：10_000 目标 → 实测约束 → 500 落定）、§2.3-4
  （报错>静默：超限结构化 E 错）、§7.2 Q5（栈溢出类崩溃为零——
  双环境探针验证）、§9.4.3（正负例成对：500/501 边界含头含尾）

---
Task ID: 20-d（重排裁定）
Agent: Super Z (main) — PM-A/ARCH-A
Task: TD-004 scope-set 解析——批次重排裁定（A4 → 批次 B 头部）

Work Log:
- 执行前知识搜索（§2.3-11：先查文档与代码禁猜测）：
  (1) grep 确认绑定形式 scope 注入现状：expander 全库 `scopes.add`/
      `.add(` 零命中——ScopeSet 数据结构全程携带但**绑定形式从不
      注入 scope mark**（scope.rs doc 的设计承诺未实现，即 TD-004 本体）
  (2) 编译器 resolve_var 现状：纯名称基栈查找（Symbol 匹配，无 scope）
  (3) CoreExpr::VarRef 现状：{name, span}——无 scopes 字段（展开层
      语法对象的 scope 信息在 lowering 时丢失）
- 完整实现量级评估（依据 §4.2 MUV 粒度）：
  - 绑定形式（lambda/let/define）scope 注入基建（expander 遍历加 mark）
  - CoreExpr::VarRef 携带 ScopeSet（**中枢类型变更**——波及 kerf-core/
    expander/compiler/eval/CodeValue/IR 全链构造与消费点）
  - 双路径解析体系切换：VM 侧 slot 解析（名称栈→(name,scopes⊆)匹配）
    + eval 侧 Env 查找（名称基→scope 匹配）——T1 双路径一致要求
    两侧同步切换
  - 保守 ≥800 LOC / 跨 5+ crate / 全部 300+ 语义回归
- 备选方案否决（§12 最优>最小）：无行为变化的「平行 scope 匹配解析
  切片」（验证性绑定表）= 死代码路径 + 双解析体系漂移风险，违背
  §11 接口隔离——正确 > 妥协的反面应用：不做看起来像但不是的
  半吊子解析器
- **裁定**：TD-004 移批次 B 头部（绑定表基建与 TD-002 符号值、标准库
  同批——均为解析体系前置），批次 E Expander 重写时收口；
  plan.md §5 批次 A 表 A4 行 + 批次 B 行同步修订（重排注记含依据）

Stage Summary:
- 批次 A 实际交付：A1/A2/A3 三 MUV（TD-015/012/007）；A4 重排
- 复杂度发现升级遵循 §1.2.1（只升不降）；裁定依据引用 §2.3-11/
  §12/§11/§4.2
- 本条目为规划修订记录（无代码变更）

---
Task ID: 21
Agent: Super Z (main) — QA-A/REC-A
Task: §3.2 终验全绿 + web 同步 + §19 打包 r4（Stage 1 批次 A 交付闭环）

Work Log:
- §3.2 六命令实测全绿（硬性门 1）：
  (1) cargo clean ✅（5080 文件 782.4MiB）
  (2) cargo build --release：0 警告 ✅
  (3) cargo check --workspace --all-targets：0 errors / 0 warnings ✅
  (4) cargo fmt --check：零 diff（修复 driver.rs 守护测试 1 处格式后达成）✅
  (5) cargo clippy --workspace --all-targets -- -D warnings：0 ✅
  (6) cargo test --release --workspace：**304 通过 / 0 失败 / 1 忽略** ✅
      （297 基线 + 7 增量：TD-015 守护 1 + TD-007 单元 2 + 集成 4）
- 审计集复跑（release）：41/41 PASS / XFAIL-WARN 0 / EXIT 0；§7.3.1
  配比与七类覆盖全部满足
- web 同步：roadmap Stage 1 →「进行中（批次 A 已交付）」+ 批次 A 三
  MUV 要点；kerf-data 测试计数 297→304（r4 口径）；Stage 0 卡片
  同步 304
- Agent Browser 端到端自验证：
  - 页面渲染：Stage 1 状态徽章 + 批次 A 文本 + trampoline 关键词 ✅
  - Playground 执行：fib → 55（API）/ 宏链 (m (m (m 42))) → 42
    （trampoline 语义经 web 全链路）/ 无限自指宏 → E0002 结构化
    报错「宏展开深度超过上限 500」带 Span 定位（新上限生效实证）
  - UI 输出渲染「⇒ 144」正常；console 零错误（仅 DevTools/HMR 日志）
  - docs API 动态读取 03-macro-system TD-007 注记（v5.2 自动同步）
  - 响应式 390×844 无 undefined
- 文档对账回写：matrix.md（304 + 新套件 expansion_worklist 行）/
  status.md（r4 头 + 计数）/ RELEASE_NOTES（r4 章节）/ data-flow.md
  （TD-015 分流注记——lower 从主链改消费方旁路 + compile_front 快路径）
- git commit：feat(stage1-batchA)——TD-015/012/007 偿还 + TD-004 重排
- §19 打包 r4：kerf-stage1-v0.2.0-batchA-td015-td012-td007-trampoline-
  304tests-r4.tar.gz（463,717 B 压缩 / 185 文件；exclude target/.git/
  download）；**包内解压全 workspace 自举验证 304:0:1 与交付环境一致**

Stage Summary:
- Stage 1 批次 A 交付闭环：§3.2 全绿 + 审计集 + web 同步 + 自举
  验证包 + git 提交（r4）
- 遵循原则：§3.2（交付前实测——六命令逐条记录）、§7.3.1（审计集
  release 复跑）、§19（打包 + 包内自举验证）、浏览器验证标准（非
  "编译通过"即完成——Playground/渲染/响应式/console 四面验证）

---
Task ID: 22-a
Agent: Super Z (main) — PM-A/PL-A 裁定 → DEV-A 实现 → QA-A 验收
Task: TD-002 符号值偿还（Stage 1 批次 B 首个 MUV）——quote 符号 datum → Value::Symbol 全链

Work Log:
- 启动协议（§0）：读 sop.md §1 路由（写代码/写测试/写文档/交付验收四类
  任务约束）→ 复杂度 L3 判定（TD-004 中枢类型变更在本批；本 MUV 实际
  L2 触点 9 文件）→ worklog 摘 Task 20-d/21（无冲突）→ MUV 六字段拆分
  （22-a TD-002 / 22-b TD-004）
- 知识搜索（§2.3-11 先查现状禁猜测）：触点链实测定位——LiteralValue
  （expr.rs）/ LiteralKey（ir.rs）/ BcConst::Symbol=全局名索引（bytecode.rs
  ——与符号值语义冲突点：vm const_to_value 对 Symbol 返回 Nil「不会压栈」）/
  Value 枚举（value.rs）/ HeapObj+BoxedInput+ValueSlot 三处装箱
  （heap.rs——序对元素必须堆存，触点比登记册预估扩大）/ compile.rs
  intern_const / eval.rs eval_literal / core_forms datum_to_value
- 实现全链（9 文件）：
  1. LiteralValue::Symbol(Rc<str>) + render 裸名（kerf-core/expr.rs）
  2. LiteralKey::Symbol（ir.rs 字面量共享去重）
  3. BcConst::SymLit(Rc<str>)（bytecode.rs——与全局名索引 Symbol 变体
     语义严格区分，§11 接口隔离；Hash 手工 impl 补臂）
  4. compile_literal Symbol → SymLit 常量 + compile_literal_value 同步
     （compile.rs 222/365 两处）
  5. Value::Symbol + type_name "symbol" + eq_value 按名相等 + render_value
     + slot_terminal/render_slot（value.rs 5 处）
  6. const_to_value SymLit → Value::Symbol + box_value alloc_symbol +
     unbox_slot ValueSlot::Symbol（vm.rs 3 处）
  7. HeapObj::Symbol + alloc_symbol + unbox + ValueSlot::Symbol（heap.rs）
  8. eval_literal Symbol 分支（eval.rs——双路径同步）
  9. datum_to_value：Symbol 分支基名剥离 $hyg$ 后缀（core_forms.rs——
     Racket 语义近似：符号值名=用户可见名，与 driver resolve_hygiene_
     fallbacks 同一口径；宏模板内符号被 instantiate_template 卫生重命名
     → quote 后剥离回原名，实测验证）；Vector 分支保持显式报错（消息
     去 TD-002 引用）
- 卫生语义裁定（worklog 透明记录）：instantiate_template 对模板所有
  非保留/非模式变量符号重命名（含 quote 内 datum——与 Racket 的
  syntax->datum 纯数据语义有差）；剥离策略 = 名称基实现下的显式近似，
  TD-004 scope-set 解析落地后 datum 层天然不受解析影响（剥离保持）
- 测试（+5 函数 / +6 负例 case）：
  - expander_tests：quote_symbol_becomes_symbol_literal（改自旧报错
    负例——datum 正例 4 断言）+ quote_symbol_from_macro_template_strips_
    hygiene（宏模板卫生剥离端到端）
  - vm_tests：quote_symbol_value_semantics（quote/列表/混合/eq? 7 断言）+
    symbol_construction_and_extraction（cons/list/car 往返）+
    quote_symbol_dual_path_agreement（T1 双路径 5 断言）
  - negative_vm_tests：symbol_value_misuse（6 case 算术/条件/序对/比较
    ——消息实跑校准：+ 需要 int / 条件位置需要 bool，实际 symbol /
    car 需要 pair / = 需要数值）
  - expander.rs 单元：quote_symbol_is_explicit_error 改名
    quote_symbol_becomes_symbol_literal（正例化）
  - 负例迁移：negative_expander_tests::quote_misuse 6→4 case（符号
    datum case 转正后负向锚点转向值消费端 VM 层）；negative_semantics_
    tests::scope_closure_negatives 末 case 改双路径符号算术负例
- 负例消息实测（§9.4.3 实跑校准纪律）：探针 11 程序实跑（含 VM 渲染
  Str 不带引号 vs LiteralValue 带引号的分层差异——断言修正实录）
- 回归：309:0:1（304 基线 + 5）；release 同口径；clippy -D 零警告；
  fmt 零 diff（expand_quote 签名单行化 1 处）；审计集 41/41 EXIT 0
- 文档回写：TD 登记 TD-002 → 已解决（符号部分，向量开放注记）/
  capability-boundaries（quote 符号移入支持行 + 显式不支持改为 quote
  向量）/ matrix.md 309 对账（负 case 489 + 审计 32 = 521，正负比
  1:3.2 维持）/ status.md r5 / RELEASE_NOTES r5 章节

Stage Summary:
- TD-002 符号部分解决：quote 符号全链（expander datum→core→compiler
  常量池→VM/eval 双路径→堆装箱→eq?/渲染）+ 卫生剥离语义锚定
- 触点扩大教训：登记册「影响范围」低估（expander/vm 两处 → 实际 9
  文件含 runtime 装箱三处）——序对元素必经堆路径，后续值类型变更
  前先查 HeapObj 触点（登记时注记）
- 遵循原则：§2.3-11（触点链先查禁猜——BcConst::Symbol 语义冲突因此
  提前发现，避免常量池双义）、§11（SymLit 与全局名索引严格分立）、
  §9.4.3（负例实跑校准 + 正负比维持）、§2.3-4（符号值语义误用显式
  报错不静默）、§5.3（八项内循环退出——1-7 项全过，8 非阶段末轮）

---
Task ID: 22-c
Agent: Super Z (main) — PM-A 裁定 → DEV-A 实现 → QA-A 验收
Task: 标准库最小集（批次 B 第二 MUV）——列表/字符串/I/O 各 ≥8 函数（07 §3.3 阶段门条件 3）

Work Log:
- MUV 重排裁定（PM-A，§12 最优>最小 + plan §5 批次 B 序列）：
  22-b（TD-004）按 20-d 重排裁定留批次 E 收口批次整体推进（≥800 LOC
  中枢变更不可分割——绑定注入/VarRef 桥/解析切换一体交付才有意义，
  单独注入 = 无消费死数据）；本 MUV 先行（独立于 scope 解析）
- 高阶函数实现路径裁定（worklog 透明记录）：map/filter/foldl/for-each
  推迟 B3「Reader kerf 重写」——用 kerf 源码 preamble 实现是 B3 自举
  验证命题本体；三方案否决实录：P1 源码拼接（Span 诊断污染 = P1 缺陷）、
  P3 跨程序全局合并（独立 SymbolTable id 不可比）、P5 Builtin 调闭包
  （需 VM 递归 re-entry——架构越界 §11）
- 实现 24 新函数（builtins.rs register_globals + io.rs write_stdout
  通道层 + lib.rs 导出）：
  - 列表 8（length/append/reverse/list-ref/list-tail/member/assoc/
    last-pair）：堆序对链遍历模式（Pair → unbox cdr 递进）；nil 终结
    契约；member/assoc 按 eq?（命中子表/点对，未命中 false）；append
    末参原样（Racket improper 尾语义）
  - 字符串 10：字符索引 Unicode 安全（str-length "héllo"=5；
    str-substring chars().skip/take；str-index-of 字节 find→字符位
    换算）；大小写 Unicode char 变换；string->symbol/symbol->string
    TD-002 联动
  - I/O 6：newline/write-string（通道层 write_stdout 新增）/
    read-int/read-num（行解析 i64→f64 降级，失败结构化报错，
    EOF→nil）/error（消息部件 str 原文+其余类型名）/assert-eq?
    （eq? 断言失败渲染两值）
- 实跑发现并修复 2 伴随缺陷（P1 级——测试先行发现的实现 bug）：
  1. list/reverse 空参返回 (nil) 包装（堆 nil 槽被包成序对）——
     改 nil 值形态与 '() 一致（空表即 nil 语义）
  2. list-tail k=0 对非 list 输入静默返回原值——重写为每步形态
     校验（Racket contract 严格：输入必须是 list）
- 消息校准两轮（§9.4.3 实跑禁臆测）：批量探针 21+48 程序；
  修正 str 四函数两参错误消息误导（第二参错误时报第一参类型 →
  改两参类型并报）；类型消息模板多样本实测后矩阵推广
- 测试 stdlib_tests.rs（15 函数 / 正例 59 断言 + 负例 172 case）：
  - 正例三组（列表 16/字符串 17 含 Unicode/I-O 6）+ Racket 语义
    边界注记（(append 1)=1 恒等、(member 1 (cons 1 2)) improper
    首命中、(list-tail lst 0) 恒等）+ eq? 语义边界（堆值按引用
    断言失败/无数值塔 1≠1.0/字符串按内容/符号按名）
  - 负例矩阵四层（元数 25/类型 86 含全扫描/边界 12/语义）——
    全局正负比 1:3.1 维持（§9.4.3 门限）
  - 双路径一致 14 断言（T1）
- 负例探针校准实录：(append '() '() 5) 期望末参报错实测 Ok——
  末参原样是设计语义（末参 improper 允许），测试改四参中间位
  （负例「期望报错实际 Ok」的探针自我纠错——写测试前先实测的
  流程价值实证）
- 回归：324:0:1（304 基线 + 20）；release 同口径；clippy -D
  零警告（修 unused ty 1 处）；fmt 零 diff；审计集 41/41 EXIT 0
- 文档回写：09-stdlib v5.3（24→48 项清单 + 高阶函数 B3 推迟
  注记）/ capability-boundaries（48 内置）/ matrix 324 对账 +
  stdlib 行/ status r5（+20）/ RELEASE_NOTES r5 修正结构 /
  tests/v0/stage1/plan.md + plan/stdlib.md 新建（§9.2 双向印证）

Stage Summary:
- 阶段门条件 3（07 §3.3「标准库已包含：列表操作、字符串处理、
  基本 I/O」）的 Stage 1 最小集交付：48 内置函数
- 2 项实现缺陷经实跑发现即修（空表包装形态/list-tail 静默通过
  ——「报错>静默」§2.3-4 的实证）；负例矩阵四层维持全局 ≥1:3
- 遵循原则：§12（高阶函数 B3 时序裁定——最优>最小的时间维度）、
  §2.3-4（list-tail 类型严格）、§9.4.3（172 负 case + 消息全实跑
  校准 + 正负比维持）、§8.4.6（write_stdout 落 kerf-runtime 通道层
  ——语言层 kerf-driver 注册的双层表面架构一致性）、§11（Builtin
  无 VM re-entry 的边界尊重——高阶函数不走 builtins 通道）

---
Task ID: 23
Agent: Super Z (main) — QA-A/REC-A
Task: §3.2 终验全绿 + web 同步 + §19 打包 r5（Stage 1 批次 B 交付闭环）

Work Log:
- §3.2 六命令实测全绿（硬性门 1，cargo clean 起步）：
  (1) cargo clean ✅（4890 文件 829.5MiB）
  (2) cargo build --release：0 警告 ✅
  (3) cargo check --workspace --all-targets：0 errors / 0 warnings ✅
  (4) cargo fmt --check：零 diff ✅
  (5) cargo clippy --workspace --all-targets -- -D warnings：0 ✅
  (6) cargo test --release --workspace：**324 通过 / 0 失败 / 1 忽略** ✅
      （304 基线 + 20：TD-002 符号值 5 + 标准库最小集 15）
- 审计集复跑（release）：41/41 PASS / XFAIL-WARN 0 / EXIT 0；
  §7.3.1 配比与七类覆盖全部满足
- web 同步（Next.js / 路由 / 唯一用户可见面）：
  - kerf-data：roadmap Stage 1 →「进行中（批次 A/B 已交付）」+ 批次 B
    要点（TD-002 全链 + 48 内置 + 阶段门条件 3）；Stage 0 卡片 324；
    PACKAGE_CONTENTS 324；capabilities I/O 行 write_stdout；playground
    文案 48 内置
  - **stats API 包名正则修复**：`/kerf-stage0-*.tar.gz/` 只匹配 stage0
    前缀——r4 的 stage1 包从未被正确显示（r4 会话 web 同步遗漏，
    本次发现即修 §8.4.5 文档-代码一致性同型）；改通用 `/kerf-*.tar.gz/`
    + mtime 降序选最新
- Agent Browser 端到端自验证：
  - 页面渲染：测试计数 324（stats API 从 matrix.md 自动）+ Stage 1
    批次 A/B 状态徽章 + 48 内置文本 ✅
  - Playground 执行（web → API → kerf run 全链路）：quote 符号 →
    hello-symbol（TD-002 语义经 web 实证）/ str-length "héllo wörld"
    → 11（Unicode）/ str-upcase → AÉ / length → 4 / reverse → (3 2 1) /
    append → (a b c d) / assoc → (b 2) / member → (3 4)；
    API 直连：(eq? (string->symbol "foo") 'foo) → ⇒ true / str-index-of
    → 6 / list-tail → (c d) / last-pair → (3)
  - 负例：(+ 1 'a) → E0004 结构化报错 + Span 源码摘录渲染（exit 1）
  - 响应式 390×844：无横向滚动、无 undefined、console 零错误
  - 桌面 1440×900：footer 正常 + 截图存档
- git commit：feat(stage1-batchB) TD-002（22-a）+ stdlib（22-c）两条
- §19 打包 r5：kerf-stage1-v0.2.0-batchB-td002-symbolvalue-stdlib48-
  324tests-r5.tar.gz（489,485 B / 194 文件；exclude target/.git/
  download/tool-results）；**包内解压全 workspace 自举验证 324:0:1
  与交付环境一致**

Stage Summary:
- Stage 1 批次 B 交付闭环：TD-002 符号值 + 标准库最小集（48 内置）
  + §3.2 全绿 + 审计集 + web 同步（含 stats API 正则缺陷修复）+
  自举验证包 + git 两条提交（r5）
- 批次 B 剩余：TD-004（重排裁定批次 E 收口批次整体推进）+ B3 Reader
  kerf 重写 + 高阶函数 preamble（下会话按 plan §5 序列）
- 遵循原则：§3.2（交付前六命令实测）、§7.3.1（审计集 release 复跑）、
  §19（打包 + 包内自举验证）、§8.4.5（stats 正则缺陷发现即修——
  文档-代码一致性的运行时面）、浏览器验证标准（Playground 语义/
  渲染/响应式/console 四面）
---
Task ID: 24-a
Agent: Super Z (main) — DEV-A
Task: VM 宿主调用 API call_closure（B3 前置——自举 Reader 的程序化调用入口）

Work Log:
- §0 启动协议：sop.md §1 路由（写代码/写测试约束）→ L3 判定（中枢
  read 路径切换在本批）→ worklog 摘 Task 20-d/21/22-a/22-c/23（无冲突：
  Task 23 尾注「下会话按 plan §5 序列」→ B3）→ MUV 六字段拆分
  （24-a/b/c/d）
- 实现（kerf-vm/src/vm.rs）：`call_closure(program, globals, heap,
  callee, args)` ——函数入口帧进入执行循环：Bytecode 闭包提取 + 原型
  越界防御（跨程序闭包显式拒绝——P3 否决的运行时面，§2.3-4 报错>
  静默）+ 元数检查（与 CALL 同口径）+ 参数入单元格 + 错误追踪帧链
  快照（与 run_program 同构）
- RET 底帧语义泛化：`frames.len() <= 1` 时返回弹栈值（原为「主原型
  出现 RET」报错——run_program 主原型以 Halt 终止不经此路径；查全库
  无该错误消息的断言依赖）
- 测试（+4）：带参调用/多次调用独立性 / 非闭包与元数错误 / 跨程序
  原型拒绝 / 错误传播与追踪
- 回归：kerf-vm 18:0（14 + 4）；cargo check 全绿

Stage Summary:
- call_closure 交付：宿主信任层对已加载程序的程序化调用（§11 与
  run_program 同级——builtin 内部递归 re-entry 仍禁止，P5 否决维持）
- 遵循原则：§11（接口隔离——入口与 run_program 同级而非 builtin 层
  越权）、§2.3-4（跨程序闭包显式报错而非越界 panic）

---
Task ID: 24-b
Agent: Super Z (main) — DEV-A
Task: B3 自举 Reader 本体——reader.krf（kerf 源码词法+语法+高阶函数）+ 自举桥 + 生产读路径切换

Work Log:
- 知识搜索（§2.3-11 先查禁猜）：种子 reader 全读（lexer 581/parser
  321/token 158 行——错误消息/Span/次序口径逐项摘录）；driver 管线
  （compile_front/dump_*）；VM 值模型 + CALL/RET/Halt + GC 根集
  （栈+帧+全局——call_closure 复用同一 execute 循环天然安全）；
  SymbolTable 语义（intern("lambda")==keyword_symbol——桥侧符号
  直接按名 intern 即一致）；内置面盘点（无 char 原语→4 新原语裁定）
- 4 新原语（builtins.rs，§8.4.6 两级语义——运行时服务层非语言语义面）：
  str->pos-chars（(字节偏移 . 单字符) 列表——偏移差分得 UTF-8 长度）/
  char-whitespace?/char-alphabetic?（Unicode 属性——语言内不可枚举）/
  str-int-valid?（i64 域 = Rust parse 同源——错误次序 parity 的前置
  校验；正确舍入的 f64 转换与 i64 域是宿主类型边界）
- reader.krf（~430 行 kerf，crates/kerf-driver/src/bootstrap/）：
  - 契约：lex-src(src, blen)/parse-tokz(toks) 两入口（与种子
    lex_source/parse_tokens 接口形状对齐 §11）；Token=(kind start end
    payload) 15 种类；datum=(tag start end ...)；err=('err 消息 起 止)
    值编码（不依赖异常——VM span 指向 reader.krf 而非用户源）
  - 词法：skip-trivia/嵌套块注释/字符串转义（含种子 eo+1 字节口径
    逐字节复刻——多字节转义字符的 Span 怪癖保持）/数字扫描（指数/
    贪婪拒绝/溢出前置 str-int-valid?——扫描序 parity 关键）/省略号族
    （"..."→id 续字符 / ".." "...."→标识符——精确复刻种子分支）
  - 语法：递归下降 + 深度 256 上限 + 括号配对矩阵 + 'x → (quote x)
    （头符号 Span=引号字符）
  - 高阶函数序章：map/filter/foldl/for-each（r5 裁定「自举验证命题
    本体」——lex-scan-with 为真实高阶消费点：scan-ident/dots/number
    作函数值传入）
  - 错误传播：尾调用链形态（lex-loop 尾递归——深层 err 经尾链免费
    上浮；parse-datum 非尾位经单点检查续传）
- 自举桥（bootstrap.rs）：thread_local 惰性加载（种子编译 reader.krf
  → run_program 顶层定义——无递归）+ 持久堆（GC 根集含全局——
  KEYWORDS 等序对树全局跨调用存活）+ 值树→Token/Stx 走查 + 数字
  同源 parse（消息/Span 逐字节）+ as_err_form 值错误解码 + VM 内部
  错误防御路径（reader.krf 源映射渲染）+ 自检面（selfcheck_call/
  list/render——§14.9 编译器自调试族，builtin 直调分支）
- driver.rs 重构：compile_front（自举读）与 compile_front_seed（种子
  读）经 front_from_forms 共享后段（expand/簿记/compile 单一实现）；
  dump_tokens/dump_stx 切换自举读；FrontOutput pub(crate)（bootstrap
  消费三字段）
- kerf-reader：operator_of 导出（桥复用同一运算符映射——唯一可信
  数据源 §2.3-10）
- 开发中发现并修复 2 缺陷（P1 级——既有测试立即拦截，§5.2 当轮内循环）：
  1. tok-end 命名冲突（游标助手 vs Token 访问器——E6 重复定义在
     reader.krf 加载时暴露：种子的重复定义守卫生效实证）
  2. 'nil/'true/'false 引用后是字面量 datum（TD-002 语义）非符号——
     tag 改经 string->symbol 构造 + lex-string 解构字段序（second→
     third——string token payload 曾误取 kind 整数）
- 回归：全套件 328:0:1 经自举 Reader（含全部负向消息断言）——整体
  行为等价实证

Stage Summary:
- B3 交付：Reader 以 kerf 源码运行于 Stage 0 VM（07 §3.2 混合期构成
  的 Reader 项 ✅）；生产读路径整体切换；种子保留双职责（引导编译 +
  parity oracle）
- 触点教训（登记 calibration-data）：跨表示边界（Rust 值 ↔ kerf 值）
  的字段/tag 约定无类型兜底——契约文档化 + parity 逐字节断言为对策
- 遵循原则：§2.3-11（种子全读后动手）、§11（两入口接口形状对齐 /
  call_closure 与 run_program 同级 / 原语归运行时服务层）、§2.3-4
  （err 值编码显式返回 / 跨程序闭包拒绝 / VM 内部错误防御渲染）、
  §12（数字转换归宿主原语——不做语言算术的不可靠重实现）、§2.1.1-3
  （front_from_forms 单一实现——两读入口零逻辑漂移）

---
Task ID: 24-c
Agent: Super Z (main) — QA-A
Task: B3 parity 套件（bootstrap_reader_tests：正 87 / 负 307 case）+ 全局正负比维持

Work Log:
- tests/v0/stage1/plan/bootstrap_reader_tests.rs（28 函数）+
  Cargo.toml [[test]] 声明 + tests/v0/stage1/plan.md 套件行
- parity 走查器：stx_equiv（datum+Span 递归——符号按名：两实现 intern
  次序不保证一致）+ assert_read_parity（Ok 树等价 / Err 消息+Span
  逐字节 / 形态一致）+ assert_negative_parity（种子确实报错断言——
  防语料误收正例；开发中即拦下 2 处误收：[(a) b] 有效 / 1..2 可分词）
- 正例 87 case：种子 reader 测试全集语料 + 奇异边界（省略号族/前导零/
  边界值 ±max/NFC 组合归一/NNBSP 异形空白/多字节标识符 Span）+
  Token 流 parity（种类+Span）+ dump 格式 parity（种子格式循环镜像）+
  深度 256 ok / 高阶函数直测（map/filter/foldl/for-each 经 selfcheck
  ——builtin 作 f，跨程序安全）+ Reader 原语正例 + 管线集成
  （run fib→55 / 读错误渲染 / T1 双路径 / dump_stx）
- 负例 307 case：28 负例语料 + 双错误次序 5（首错位置契约——溢出
  前置 vs 后置词法错误双向）+ 深度 257/300 + 系统化矩阵 266（未闭合
  深度 1..30 / 括号错配矩阵 / 非法转义 32 字符含多字节 eo+1 口径 /
  贪婪数字 10×5 矩阵 / 溢出扫描 19..26 位 × 3 符号 + 边界外一格 +
  前导零 / 非法字符 8×3 上下文 / 双错误 9 / 注释嵌套与 EOF 16）+
  原语误用 6
- 自检面（bootstrap.rs）：call_entry 双形态（kerf 闭包走 call_closure；
  builtin 直调——str->pos-chars 等原语经 selfcheck 驱动）
- 回归：bootstrap_reader_tests 28:0；全套件 356:0:1；clippy -D 0；
  fmt 零 diff；审计集 41/41 EXIT 0
- 文档对账：matrix.md r6（356 全绿 + 正负比 1:3.2 全局口径——负 1000
  case / 正 ≈311）

Stage Summary:
- parity 验收门交付：双实现行为契约（树/消息/Span/次序）逐字节锁定；
  §9.4.3 正负比经负例矩阵维持（1:3.2——r6 增量正 87/负 307）
- 遵循原则：§9.4.3（正负例成对 + 实跑断言防误收）、§7.1（集成验证
  ≥3：parity/管线/hof）、§2.3-11（断言全部经双实现实跑比对——
  零手写期望值快照）
