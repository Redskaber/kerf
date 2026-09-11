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
---
Task ID: 24-d
Agent: Super Z (main) — QA-A/REC-A
Task: §3.2 终验全绿 + 文档回写 + web 同步 + Agent Browser 验证 + §19 打包 r6（批次 B 收官 B3 交付闭环）

Work Log:
- §3.2 六命令实测全绿（硬性门 1，cargo clean 起步）：
  (1) cargo clean ✅（4779 文件 842.3MiB）
  (2) cargo build --release：0 警告 ✅
  (3) cargo check --workspace --all-targets：0 errors / 0 warnings ✅
  (4) cargo fmt --check：零 diff ✅
  (5) cargo clippy --workspace --all-targets -- -D warnings：0 ✅
  (6) cargo test --release --workspace：**356 通过 / 0 失败 / 1 忽略** ✅
      （324 基线 + 32：VM call_closure 4 + parity 套件 28）
- 审计集复跑（release）：41/41 PASS / EXIT 0；§7.3.1 配比与七类覆盖满足
- 文档回写（§8.4/8.5 对账）：
  - 07-bootstrap v5.3（§3.2 B3 交付注记 + 混合期构成三项状态）
  - 09-stdlib v5.4（52 项清单：+4 Reader 原语行 + hof r6 交付注记）
  - 02-syntax-model（§6 B3 双实现注记——框架的 kerf 源码落地）
  - stage-1/plan §5（批次 B 行 B3 ✅ + TD-004 批次 E + TD-021 注记）
  - matrix.md r6（356 对账 + 正负比 1:3.2 全局口径：负 1000 case）
  - tests/v0/stage1/plan.md（parity 套件行）+ status.md r6 + RELEASE_NOTES r6
  - tech-debt-register：TD-021（hof 用户面注入——P1/P3/P5 否决 + 批次 E
    模块系统载体）+ TD-022（自举 Reader 帧消耗 O(字符)——Stage 2 TCO 决策点）
  - graph/pipeline/data-flow.md（自举读路径图：BOOT 子图 + 桥 + oracle
    虚线）+ calibration-data.md（r6 教训：跨表示边界字段/tag 错位对策）
  - capability-boundaries（52 内置）
- web 同步（Next.js / 路由 / 唯一用户可见面）：
  - kerf-data：roadmap Stage 1 →「批次 A/B 已交付；B 收官」+ B3 要点
    （356 经 kerf Reader 执行 / parity 正 87 负 307 / hof 源码化 TD-021）；
    Stage 0 卡片 356；PACKAGE_CONTENTS 356；capabilities Reader 行
    （kerf-reader + bootstrap/reader.krf 双载体）
  - playground 文案：读阶段经自举 Reader + 52 内置
  - stats API：从 matrix.md 自动（356）+ 包名正则 r5 修复沿用
- Agent Browser 端到端自验证：
  - 页面渲染：356 计数 + B3/自举 Reader/parity 套件/reader.krf 文本 ✅
  - Playground（web → API → kerf run → 自举 Reader 全链）：
    fib(10) → 55；Unicode/stdlib 组合 → (11 AÉ true (3 2 1))
    （str-length/str-upcase/string->symbol eq?/reverse 经 kerf Reader）；
    负例 (+ 1 → E0001「括号未闭合」+ Span 源码摘录渲染（exit 1）
  - docs API：07-bootstrap-strategy.md B3 交付注记动态读取 ✅（缓存
    TTL 刷新后确认）
  - 响应式 390×844：无横向滚动（scrollWidth=390）、无 undefined 文本；
    console 零错误；桌面 1440×900 截图存档
- git commit：feat(stage1-batchB) B3 自举 Reader（parity 28 函数）
- §19 打包 r6：kerf-stage1-v0.2.0-batchB-b3-bootstrap-reader-356tests-
  r6.tar.gz（525,880 B / 198 文件；exclude target/.git/download/
  tool-results）；**包内解压全 workspace 自举验证 356:0:1 与交付环境
  一致 + 审计集 41/41 + fib→55 CLI 实跑**；download/README.md r6 章节

Stage Summary:
- Stage 1 批次 B 收官闭环：B3 自举 Reader（Reader 以 kerf 源码在 VM 上
  运行——07 §3.2 混合期 Reader 项 ✅）+ §3.2 全绿 + 审计集 + 文档 8 处
  回写 + web 同步 + 浏览器四层实证 + 自举验证包 + git 提交（r6）
- 批次 B 全景：B1 TD-002 符号值（22-a）+ B2 标准库最小集（22-c）+
  B3 自举 Reader（24-a/b/c）——A4/TD-004 重排批次 E 收口（20-d 裁定维持）
- 下会话序列（plan §5）：批次 C（类型检查器 → 编译缓存 → TD-016/013）
- 遵循原则：§3.2（交付前六命令实测——逐条记录）、§7.3.1（审计集
  release 复跑）、§19（打包 + 包内自举验证）、§8.4.5（文档 8 处回写
  对账）、浏览器验证标准（Playground 语义/渲染/响应式/console 四面——
  定位器失误两次自纠后全绿：find text 匹配到提示文本而非按钮，改
  role locator 解决）
- 追加（交付闭环内发现即修，§8.4.5 文档-代码一致性同型）：**download 路由
  包名正则缺陷**——`/api/download` 仍持 r5 修复前的 `/^kerf-stage0-*.tar\.gz$/`
  前缀匹配（stats 已修而 download 未同步——r5 会话只修了 stats），实际
  服务 r3 stage0 旧包（443,174B）而非 stage1 包。镜像 stats 修复（通用
  `/^kerf-.*\.tar\.gz$/` + mtime 降序选最新）后 525,880B 与 r6 tarball
  字节一致（cmp 验证）；lint 全绿。教训：同型缺陷修复须全消费面扫描
  （download/stats 两路由当时各持同一正则）。

---
Task ID: 25-a
Agent: Super Z (main) — DEV-A
Task: 批次 C MUV1——保守静态类型检查器本体（kerf-compiler/typecheck.rs，§21.6 循环依赖缓解落地）

Work Log:
- §0 启动协议：sop.md §1 路由（写代码+测试+文档+交付四类）→ L3 判定
  （跨 compiler/driver/syntax/CLI/web 五面，~1500 LOC）→ worklog 摘
  Task 24-d 尾注（批次 C 序列承接，无冲突）→ MUV 25-a~e 拆分
- 设计收敛（§2.3-11 先查禁猜）：精读 12-roadmap §2.5.2（L 节点）/07 §2
  （循环依赖陷阱对策）/13 §3.2（类型检查器推迟行）/builtins.rs 52 内置
  守卫逐项核对（one_arg/two_args/args.len/类型 match）——签名表口径
  与运行时守卫一致
- typecheck.rs（~600 行）：TcType 保守类型格（Unknown/Int/Float/Num/
  Bool/Str/Nil/Symbol/Pair/Callable{min,max}）+ join 合并；R1-R8 规则
  （if 条件/算术/比较族 TD-016 全操作数+TD-011 字符串边界/not/car-cdr/
  不可调用/元数 lambda+内置/字符串符号族）；词法环境（params 装订
  Unknown + define 顺序填充 + 内置遮蔽判定）；多错误收集（全量 +
  Span 次序排序）；MAX_CHECK_DEPTH=512（Reader 256 上限 ×2——
  2000 初值实测 debug 栈溢出，依据链写入常量文档）
- BuiltinSig 数据驱动签名（TcParam/TcParams 规则枚举——检查器不含
  内置名知识，§2.3-10 唯一可信源：表在 driver builtins.rs）
- builtins.rs：BUILTIN_SIGS 49 项静态表 + builtin_sigs() 注入 + 双向
  防漂移锚测试（签名表 ⊆ 注册表 + 运算符族全覆盖）
- 深度预算程序化单测 ×3（源文本不可达 Reader 256 上限——构造面测试）

Stage Summary:
- 保守性契约成文：只报静态确定错误（运行期必然失败）——误报 = P1
  的工程口径；零误报的机械验证 = 全部既有 408 套件零新诊断
- 遵循原则：§21.6（外部 Rust 实现缓解循环依赖）、§11（签名表注入
  而非 compiler 依赖 driver）、§2.3-10（表与注册表同文件唯一源）

---
Task ID: 25-b
Agent: Super Z (main) — DEV-A
Task: 批次 C MUV2——编译缓存落地（13 §3.1.4 三方法规格做实 + SHA-256 内容寻址 + 管线接线）

Work Log:
- hash.rs（~150 行）：SHA-256 零外部依赖自实现（FIPS 180-4：K 常数/
  compress/填充）+ NIST 四向量锚 + content_hash64（摘要前 8 字节
  大端——u64 冻结字段口径）+ 截断生日界文档
- cache.rs（~300 行）：InMemoryCompilationCache（冻结 trait 实现
  get_cached/store/invalidate 规格条款 1/2/3 + 管线富入口
  lookup_front/store_front 携带 FrontOutput 完整快照）+ CacheStats
  观测 + thread_local 会话实例（每测试线程天然隔离）+
  COMPILE_CONFIG_SEED 常量（编译器升级 → 键变全量自然失效）
- SymbolTable/ModuleRegistry/FrontOutput 补 Clone 派生（快照语义：
  intern 幂等保证 ID 一致性；相位标记只作用副本）
- driver 接线：compile_front_cached（cache_enabled 开关 + 命中克隆
  + 未中编译存入 + 错误路径不缓存）；run/eval/compile_source 三入口
  经缓存路径（eval 共享 run 条目——T1 双路径产物同源）
- 键设计：config_fingerprint = SHA-256(种子 + 文件名)——产物内嵌
  SourceMap 位置信息，「源不变+配置不变 → 产物必然等价」要求位置
  一致（文档化裁定）
- 单测 6（trait 规格）+ hash 2 + cache_tests 13（集成：同源二次
  命中等价/确定性证明 cached_program_equals_fresh_compile——
  BcProgram PartialEq 逐字段）

Stage Summary:
- §21.3 Stage 1 条件 4「增量编译基础设施可用」就位：内容寻址缓存 +
  三方法冻结契约做实 + 确定性证明锁存
- 遵循原则：§11（冻结 trait 与管线服务分离的两层入口）、§12（最优
  >最小——缓存全前端而非仅产物）、13 §3.1.4（签名不动，行为规格
  逐条落地）

---
Task ID: 25-c
Agent: Super Z (main) — DEV-A/QA-A
Task: 批次 C MUV3——TD-016 收紧（比较族全操作数前置校验，运行时+静态双侧）+ check CLI/API 升级

Work Log:
- builtins.rs cmp_builtin：前置全参数校验（全字符串+排序族 → TD-011
  消息；其余首个非数值 → {op} 需要数值）——既有两参消息逐条兼容
  （comparison_type_mismatch 30 case 全绿回归）；下方逐对比较错误臂
  转为防御性路径
- 行为收敛（TD-016 目标）：(< 3 1 "a") 静默 false → 结构化错误；
  (= 1 2 "s") 同理；(< "a" "b" 1) 消息统一为「需要数值」（文档化
  收敛，两参口径不变）
- driver.rs：check_source（CheckReport{diagnostics/rendered/
  cache_hit/统计四项}——编译缓存路径 + check_program + 渲染）+
  lib.rs 导出
- main.rs cmd_check：编译 + 静态报告双段（E0005 渲染到 stderr +
  汇总行 stdout；发现问题 exit 1）+ 缓存观测行（会话命中 N/M）
- CLI 冒烟：fib.krf → ok + 0 诊断；构造 3 错程序 → 3 条 E0005 全量
  渲染（含 ^ 标记源摘录）；(< 3 1 "a") run → [run] E0004 需要数值
- 测试：stdlib_tests +2 函数（TD-016 负例 9 case + 正向回归锚
  4 case）；negative_vm_tests 头注 FS-5 边界更新（v5.5 语义）

Stage Summary:
- TD-016 双侧落地（运行时前置校验 + 静态 R3 规则同口径）——
  09-stdlib §2 v5.5 重写、登记册转已解决
- 遵循原则：§2.3-4（显式报错>静默——短路静默 false 即错误掩盖）；
  保守消息兼容（回归锚先行验证再改语义）

---
Task ID: 25-d
Agent: Super Z (main) — ARCH-A
Task: 批次 C MUV4——TD-013 多错误收集设计批（设计冻结，实现绑定批次 E）

Work Log:
- docs/develop/v0/stage-1/multi-error-recovery-design.md（新撰）：
  恢复粒度 = 形式级（表达式级不恢复——半展开状态重建成本 vs IDE
  反馈收益不成比例）；恢复机制 = 编译期控制流（非 effect——§11
  接口隔离，效应联动裁定归档：不把展开器内部控制流暴露到语言语义
  面，Stage 2 复核点已记）；DiagCollector 契约（上限 128 + 截断
  标记 + into_sorted Span 次序——与 check_program 一致）；消费面
  四行表（kerf check 已实证——typecheck_tests 多错误断言锚定）；
  验收标准 5 项（批次 E 实现时）；决策记录表 5 行
- tech-debt-register：TD-013 → 设计完成（r7 注记 + 代码锚补
  typecheck.rs）；TD-016 → 已解决（双侧偿还注记）
- 设计依据链：§8.7（错误是数据）、§12（单批单恢复机制）、§21.7
  （不为 Stage 2+ 预留投机钩子——效应重述属届时演进）

Stage Summary:
- TD-013 从 P2 开放债转为「设计完成 + 首消费面实证 + 批次 E 实现绑定」
  ——check_source 多错误收集即活体设计样例（三错误全量 + Span 次序）

---
Task ID: 25-e
Agent: Super Z (main) — QA-A/REC-A
Task: 批次 C MUV5——§3.2 终验全绿 + 文档回写 + web 同步 + 打包 r7 + 浏览器验证

Work Log:
- §3.2 六命令实测（cargo clean 起步）：(1) clean ✅ (2) build
  --release 0 警告 ✅ (3) check --all-targets 0/0 ✅ (4) fmt --check
  零 diff ✅ (5) clippy -D warnings 0 ✅ (6) test --release
  **408:0:1** ✅（356 基线 + 52：typecheck 24/cache 13/stdlib 2/
  compiler 单元 3/builtins 2/cache 单元 6/hash 2）+ 审计集 41/41
  EXIT 0
- 零误报保守性双证明：examples/ 6 程序零诊断 + 既有 408 全套件
  零新诊断（缓存行为等价 + 检查器保守性）；typecheck 68 负例全部
  携带运行期反向锚
- 测试文档：typecheck_tests.rs + cache_tests.rs 新套件（[[test]]
  注册）+ tests/v0/stage1/plan.md 两行 + matrix.md r7（408 对账 +
  逐二进制实测计数 + E0005 断言 + 正负比 1077/340 ≈1:3.2 维持）
- 文档回写 13 处：09-stdlib v5.5（TD-016 语义段重写）/13 v5.3
  （缓存做实 + 类型检查器引入两表行 + §3.1.4 交付注记）/12 v5.3
  （矩阵行）/tech-debt-register（TD-016 解决 + TD-013 设计完成）/
  RELEASE_NOTES r7/README（408 + 52 内置 + 新特性行）/graph
  data-flow v0.1.1（缓存/检查器/签名表三节点）/calibration-data
  （深度预算标定教训）/develop plan.md 批次 C 行 ✅/multi-error
  design（25-d）
- web 同步（Next.js / 路由）：kerf-data（roadmap 批次 C 两要点 +
  Stage 1 状态/Stage 0 卡 408/PIPELINE 编译器步 + 检查器/crates
  两行/RESERVATIONS 缓存做实/HERO 两徽章/PACKAGE 408）+ 新
  POST /api/check 路由（zod 校验→临时文件→kerf check→E0005 诊断
  与汇总行，路径重写 check.krf）+ playground 静态检查按钮（琥珀
  ShieldCheck + 终端面板 check 模式渲染：诊断琥珀/汇总按 exit 着色）
- stats API 自动化：matrix.md「408 通过」解析 + 包名正则取 r7
  （mtime 降序）
- git commit + §19 打包 r7 + 包内解压自举验证（见交付段）

Stage Summary:
- Stage 1 批次 C 交付闭环：类型检查器（保守静态 R1-R8 + 多错误
  E0005）+ 编译缓存（内容寻址 + 确定性证明）+ TD-016 双侧收紧 +
  TD-013 设计冻结——§21.3 条件 4 就位，408:0:1 全绿
- 遵循原则：§3.2（六命令实测逐条记录）、§7.3.1（审计集 release
  复跑）、§8.4.5（文档 13 处对账——代码-文档一致性）、§9.4.3
  （正负比维持 + 反向锚新形态）、§19（打包 + 包内自举验证）

---
Task ID: 26-a
Agent: Super Z (main) — ARCH-A/REV-A
Task: 接口契约与可替换性深审（批次 D 前置设计对齐）——四项预留契约 × 四维标尺 + 12 模块边界矩阵 + 五条替换路径审计

Work Log:
- §0 启动协议：sop.md §1 路由（评审 Agent + 新阶段）→ L3 判定 → worklog
  摘 Task 26（r7 批次 C）/plan §5 批次 D 行 → 26-x 序号裁定（预排 24-x
  已被 B3 占用，§8.6 唯一性顺延）
- 证据收集：reserved.rs 冻结契约 289 行实读 / builtins.rs I/O 消费面
  （6 内置直调 kerf_runtime 全局函数——F2 实据）/ vm.rs FrameExt 三槽
  （ext1 静态休眠——F3 实据）/ heap.rs register_foreign_ref（已落地——
  F6 正面确认）/ 12-roadmap §2.4.3 四级做实主题 + §2.5.1 矩阵 Stage 1
  行（Effect=编译器内部做实 / 能力 I/O=基础传递）/ multi-error-
  recovery-design §4 效应联动裁定（编译期恢复≠effect，继续有效）
- 撰写 docs/develop/v0/stage-1/interface-contract-review.md：四维标尺
  （D-强度/D-任意节点/D-边界/D-可替换）逐契约评定 + 12 模块边界矩阵
  （8/12 活体替换证明）+ 五条替换路径审计 + 「任意流程节点」专项裁定
  + 发现分级 F1-F6
- 关键裁定：两处 P2（F1 Effect Value 耦合、F2 I/O 硬接线）恰为批次 D
  做实目标本身——审查与实现收敛（做实即修复）；D1 语言面零暴露（12
  §2.4.3「编译器内部使用属实现策略」）；ext1 激活记 Stage 2 边界

Stage Summary:
- 深审报告冻结：契约形状均足以承载目标语义；12 模块边界零漂移；
  8/12 有活体替换证明（B3 Reader 换实现 + T1 双路径 + TD-015 分流为
  最强三证）；批次 D 范围由 F1/F2 直接驱动
- 遵循原则：§13.1（设计对齐先行）、§6.1（分级处置）、§11（内部效应
  不进语言面）、§21.7（不做实不预留投机钩子——MultiStage 维持 P3
  零行动）

---
Task ID: 27-a
Agent: Super Z (main) — REV-A/ARCH-A
Task: next.md 吸收完整性审计（lang-design 收敛验证）——四轮 Q&A 逐节映射 + 关键词矩阵 + 抽样段落三级核对

Work Log:
- §0 启动协议：sop.md §1 路由（评审 Agent + 新阶段）→ L3 判定 →
  worklog 摘 Task 26-a（批次 D 前置深审）/26（r7）→ 发现批次 D 在飞
  未提交工作（capability/effects/test_source 均就位、断点 =
  test_runner_tests.rs 缺失）→ 承接裁定（§8.6 无冲突顺延）
- 磁盘事实核验：upload/next.md（1604 行，四轮 Q&A：超越 Lisp 范式 →
  五能力术语起源/创新原则/2026 推荐 → 五方案深度设计 → Stage 0 混合
  务实）vs lang-design 20 文件现状
- 吸收链核验：stage0.md v3.0（合并 next.md：替代设计/术语起源/2026
  推荐/三层分类——00-overview 版本历史实证）+ v5.0（进程规划：
  成熟度/P0-P4/演进矩阵）→ 拆分（Task 5）→ v5.1/v5.2 收敛 → 本轮核验
- 三级核对执行：关键词矩阵（Esterel/谱系/TRAC-Mooers/创新原则/
  AST 构造-操纵-执行/混合务实/Mojo/MetaOCaml/五方案共同原则 12 组
  特征词全部命中）+ 结构映射（Q1-Q3 → 14 §1/§2/§3 逐节；Q4 → 13/15/
  12 + sop §21.7-21.11）+ 抽样段落（同像性谱系论/McCarthy 考据/
  批判性评估表/混合务实结论逐段比对）——零实质缺口
- 审计结论写入 00-overview.md v5.4 修订记录第 (1) 条

Stage Summary:
- next.md 吸收链闭合实证：v3.0/v5.0 两轮合并 + 拆分分布六文件群 +
  本轮三级核对零缺口——「吸收并融入直至完全收敛」判定达成（无需
  重建 stage0.md 中间态——00-overview 头部明示 lang-design 为其
  现行收敛形态，重建将回退 v5.1-v5.5 演化并违反唯一可信数据源
  §2.3-10，裁定记录于本条目）
- 遵循原则：§8.4.5 规则 1（先查文档）、§2.3-10（唯一可信数据源）、
  §3.3（路由>通读——按映射表精读而非全文重排）

---
Task ID: 27-b
Agent: Super Z (main) — REV-A/REC-A
Task: lang-design 批次 D 设计回写（新语言面先文档后验收）+ v5.4 版本推进

Work Log:
- 事实采集：Keyword 21→22（+Require）、Token 叶级 44→45、
  CoreExpr::Require 全链处理核实（compile→PushNil / eval→Ok(Nil) /
  IR→nil 共享节点 / typecheck→Unknown——T1 双路径一致）
- 01-core-forms v5.4：新增 §6 声明形式 require（文法/OCaml 形态/
  零运行时语义/测试锚点）+ **核心冻结边界精确化裁定**（「语义原语集
  冻结 + 声明变体可追加」——Require 为元数据节点，R1-R9 归约不变、
  可整体删除不改行为、与 Racket #%require 同构；§5 锚点表 +1 行）
- 13-capability-matrix v5.4：§3.1.1 r8 注记（Effect 编译器内部做实：
  一次性逃逸层 + InternalEffectSystem 双层，语言面保持 P3）+ §3.1.3
  r8 注记（能力 I/O 基础传递做实：三层分工 + R9/E0006 + IoGrant）
- 12-roadmap v5.4：§2.4.3/§2.4.5 四级做实表 ✅ 注记 + §2.5.1 演进
  矩阵三行状态（能力 I/O「预留保持」→「做实引入 ✅」+ 附加类型检查
  器行）+ r8 交付注记段
- 11-testing v5.4：新增 §4 用例运行器（动机/约定六条/效应消费面/
  形态学对照 rackunit）+ 处理程度对账至 476
- 09-stdlib v5.6：§2 能力门控注记（六 I/O 内置 require 门控 +
  FS-4 修复）+ 清单表两行更新
- 02-syntax-model v5.4：Token 叶级 44→45 + Keyword 22（+require）
- 00-overview v5.4：修订记录（吸收审计结论 + 批次 D 回写清单）
- 交叉引用一致性：11-testing §5→§4 重编号后三处引用同步修正
  （01/12/13）；表格列数错误当场修正（✅ 注记并入单元格）

Stage Summary:
- 批次 D 新语言面元素（require 声明/能力门控/kerf test）全部具备
  设计文档（§8.4.5 规则 3 新功能必须有设计文档——先行达成）；
  lang-design 七文件 v5.4-v5.6 状态与代码一致
- 遵循原则：§8.4.5 规则 2/3、§21.9 规则 3（矩阵一致性——状态对账
  与偏差登记）、§17 原则 9（核心冻结——边界精确化而非破坏）

---
Task ID: 27-c
Agent: Super Z (main) — ARCH-A/REV-A
Task: sop.md 同步更新（v11.1）——§21.9 演进矩阵现状对账 + 变更日志

Work Log:
- §21.9 矩阵：四行 ✅ 做实注记（编译缓存 r7 / 类型检查器 r7 提前
  引入 / Effect r8 编译器内部 / 能力 I/O r8 基础传递）+ 附加类型
  检查器行 + 实施状态对账注记段（含「预留保持→做实引入」单元格
  更新依据——r8 实际交付基础传递，单调深化成立）
- §21.7.1 三档成熟度表 P4 行：类型检查器提前引入偏差登记（§21.6
  风险缓解路径显式裁定，不构成倒挂）
- §16.1 变更日志：v11.1 行（三处变更清单）；尾部版本说明更新
- 工具事故与修复：MultiEdit 部分生效导致 v11.1 变更日志行重复 →
  行级去重；表格 old_str 匹配失败两次 → 改用精确文本/行级插入

Stage Summary:
- sop.md v11.1 就位：流程权威文档与实施现状对账一致（§8.4.5 规则
  2 文档随代码）；类型检查器提前引入按 §21.9 解读规则 3 显式登记
- 遵循原则：§3.3（合并>新增、精要>冗长——最小变更面三处）、
  §21.9 规则 3（gate 核对矩阵一致性）

---
Task ID: 27-d
Agent: Super Z (main) — DEV-A/QA-A
Task: 批次 D 承接收尾——test_runner_tests.rs 补齐 + 断点缺陷修复 + §3.2 全绿 + 文档对账

Work Log:
- 断点定位：Cargo.toml 声明 test_runner_tests 而文件缺失（clippy
  target resolution error 实证）→ 按 test_source 契约补齐 18 case
  （正向 6 + 负向/边界 12：短路/恢复/深位失败/状态隔离/front 错误面）
- 断点缺陷修复（在飞代码两处）：①capability_tests example 路径
  ../../examples → examples（workspace cwd 口径）；②read-line EOF
  测试阻塞真实 stdin（非确定性）→ 改子进程确定性探针
  （CARGO_BIN_EXE_kerf + Stdio::null()）
- 测试语义修正：状态隔离用例重设计（前置重放语义下 set! 在前置
  会让所有用例失败——改为 case 内 set! + 次用例独立重放验证隔离）
- clippy 清零五处：builtins auto-deref ×2 / capability needless_
  lifetimes ×1 / result_large_err ×2（driver 入口既有 allow 约定）
- fmt：builtins.rs 排版修正
- §3.2 六命令 clean release 前台实测：build 9.10s 0 警告 / fmt 零
  diff / clippy -D 0 / test --release **476:0:0** / 审计集 41/41
  EXIT 0（七类覆盖 1=1 2=5 3=1 4=4 5=16 6=1 7=2）
- 文档对账：matrix.md r8 全量重写（含 r7 陈旧计数修正：r7 实际 =
  150 单元 + 260 集成 = 408:0:1，分项表多处陈旧已修正）+
  RELEASE_NOTES r8 段 + plan.md 批次 D 行 ✅ + README 476/v5.4/
  v11.1 + calibration §4 批次 D 行 + 测试计划 capability.md/
  test-runner.md 新两篇 + stage1/plan.md 套件表 +2 行

Stage Summary:
- 批次 D 交付闭环：476 测试全绿（408 基线 + 68）+ FS-4 随能力
  参数化修复激活（ignore 清零）+ E0006 第七族结构码就位 + 全局
  正负比 ≈1:3.15 维持
- 断点承接方法论沉淀：worklog 唯一事实源使跨会话在飞工作可无损
  承接（calibration §4 已记录）
- 遵循原则：§3.2（六命令逐条实测）、§9.4.3（正负比 + 确定性纪律
  ——stdin 阻塞探针改子进程）、§8.5（文档 15 项对账）、§2.3-4
  （报错>静默——效应逃逸无处理器时清晰终止）

---
Task ID: 27-e
Agent: Super Z (main) — REC-A/QA-A
Task: web 同步（批次 D 内容/stats/预设）+ §19 打包 r8 + agent-browser E2E 终验

Work Log:
- web 同步：kerf-data 六面（fib 预设 +require 前缀——I/O 门控正确性
  关键修复；新增能力门控预设（GC 预设保留）；管线/9 crates/预留卡
  Effect+能力 I/O 做实注记/路线图批次 D 三要点+批次 E 待启动/包清单
  476-v5.4；HERO_FEATURES +能力门控 I/O+内部效应系统）+ hero（Stage 1
  徽章 + 副标题 + hint 单元 175/集成 301）+ footer（批次 A/B/C/D +
  476 + v0.2.0 + sop v11.1）
- stats API 自动对账：testCount 476（matrix.md 解析）/ rustLoc 15,832
  / r8 包 mtime 降序识别 ✓
- §19 打包 r8：kerf-stage1-v0.2.0-batchD-capabilityio-effects-
  testrunner-476tests-r8.tar.gz（615,655 B / 153 文件 = r7 146 + 7
  新文件；file 验证 gzip；排除 target/.git/download/tool-results）
- 包内解压自举验证（/tmp 独立解压）：cargo test --release
  **476:0:0** 与交付环境一致 + fib→75025/⇒144 + kerf test 实跑
  （PASS 3/共 5：短路 case+恢复 case+exit 1）+ E0006 未声明 print
  完整诊断（--> 位置+摘录+^）+ 审计集 41/41 EXIT 0
- download/README.md r8 段（头部插入；旧尾部 r7 重复段去重）
- agent-browser E2E 终验：页面加载 0 console/0 页面错误；统计实时
  （15,832/476/12/0）；Playground 实跑 fib（6765+⇒144）+ 能力门控
  预设（已授权输出+⇒42）+ **E0006 负例直连**（(print 1) → 红色
  error[E0006] 含 Span 摘录——新语言面端到端）；路线图批次 D/
  预留卡做实标记/footer 476 全渲染；390×844 无横向溢出
  （scrollWidth=390）；footer 平滑滚动完成后视口底精确对齐
  （footerBottom=844=viewportH——长内容自然下推双形态验证）；
  文档区 00-overview v5.4 修订记录渲染（next.md 吸收审计可见）；
  VLM 双截图审查：无重叠/截断/对比度问题，zinc+emerald 主题合规
  （发现并修复 hero Stage 0 陈旧徽章 → Stage 1 批次 D）
- web 质量闭环：bun run lint 0 错误；dev.log 无编译错误（仅正常
  200 请求日志）

Stage Summary:
- 会话五项任务全部完成：SOP 启动协议 + next.md 吸收完整性审计
  （零缺口——吸收链 v3.0/v5.0→拆分→v5.2 闭合实证）→ lang-design
  v5.4 系统回写（require 声明形式 + 核心冻结边界精确化 + 演进矩阵
  对账，七文件）→ sop.md v11.1（§21.9 现状对账 + 变更日志）→
  批次 D 承接收尾（断点 test_runner_tests 补齐 + FS-4 修复 + 476
  全绿）→ web 同步 + r8 交付包（615KB/包内自举验证一致）
- 遵循原则：§19（打包规则：全绿后打包 + 包内自举验证 + 文件数
  对账）、§3.2（交付前实测全绿）、浏览器验证标准（交互/渲染/
  响应式/console 四面 + VLM 视觉审查）

---
Task ID: 28-a
Agent: Super Z (main) — REV-A/ARCH-A
Task: next2.md 吸收审计（五轮讨论对照）——增量缺口识别

Work Log:
- 读 upload/next2.md 全文（2180 行五轮：①核心原语 9→8→6 最小性/
  命名精确性/de Bruijn/效应原语化 ②2026 前沿全景五维 ③四层正交+
  八 IR 批判（六缺陷）④六 IR 最终修正 + MLton 闭包 + Koka 效应消除
  + continuation 类型安全（五弱点）⑤Stage 0 S 表达式决策量化论证）
- 关键词矩阵核对：Perform/Handle/de Bruijn/四层正交/六 IR/MLton/
  Koka/comptime 于 lang-design 20 文件**零命中**——与 next.md 轮
  （零缺口）相反，本轮判定**实质缺口**，需全量吸收
- 增量清单确定：01（最小性对照+演进裁定）/14（前沿全景+审查史）/
  15（四层对照+IR 演进）/13（comptime）/12（S 表达式背书）/17/
  18/19/00 九文件落点
- Cargo.toml 现状核查：[[test]] 18 块显式声明（用户「干净精要仅总
  入口」意图的改造对象）；mod common 挂载方式确认（#[path] 相对
  文件目录解析——合并可行性前提）

Stage Summary:
- 审计结论：next2.md 存在实质增量（核心原语批判审查史 + 前沿全景
  矩阵 + 四层正交/六 IR 修正版 + MLton/Koka 性能路径 + S 表达式
  量化论证）；kerf r8 实现与 next2 最终修正版架构同构（四层解耦
  对照待写入 15 §5）
- 遵循原则：§8.4.5 规则 1（先查文档）、§2.3-11（知识搜索>猜测）

---
Task ID: 28-b
Agent: Super Z (main) — REV-A/REC-A
Task: lang-design v5.5 增量回写（next2 五轮 → 九文件）

Work Log:
- 01-core-forms §7：9（本设计）vs 8（Racket kernel 真实数）vs 7
  （不可消除最小集——Define 是糖）数量真相 + 命名精确性九行对照表
  （裁定：核心 ADT 冻结期不变，表作为 Stage 2 表面语言命名参考）+
  §7.3 效应原语化（Perform/Handle）Stage 2 演进对照裁定（与当前
  路径同归殊途——语言级引入时的首选评估对象）+ de Bruijn/continuation
  类型安全登记
- 14-design-alternatives §4：前沿五维成熟度矩阵（与 12 §2.1.1 三档
  分级一致性核对——唯一增量 comptime 生产就绪档）+ 批判审查史链
  （推荐→批判→修正×3 轮与本项目内循环结构同构的方法论注记）+
  可行性评分五目标对照 + 四风险缓解
- 15-architecture-layers §5：四层正交 crate 对照表（语法/类型/效应/
  能力/组合器五行逐一映射 kerf-core::expr、typecheck、effects、
  capability、driver front——**r8 实现与 next2 最终版架构同构实证**）
  + IR 六层演进表（当前 4 层对照 + ANF/SSA Stage 2 主题）+ MLton
  三种闭包表示 + Koka 效应消除四阶段 + 性能路径量化
- 12-roadmap §2.4.1：S 表达式量化背书（Reader 300 vs 3000 行 +
  脱糖恒等 + 宏直接性——「脚手架非终点」皮肤骨架论）+ 分阶段语法
  策略表（r6 B3 已兑现 Stage 1 行）
- 17-principles：next2 六原则对照映射表（合并>新增——全部映射到
  既有原则 2/9/22/24/5/12/27/28）；前沿整合五原则同构注记
- 18-terminology §3：ANF/CPS/de Bruijn/行多态/效应安全/闭包转换/
  comptime/continuation 八条术语
- 19-references：§1 表 +6 行（MLton/Koka/Unison/CompCert/Zig
  comptime）+ §4 引用链接（+λ○▷ 研究跟踪）
- 13-capability-matrix v5.5：附注（comptime Stage 1+ 主题登记——
  多阶段 P3 预留的务实前身）
- 00-overview v5.5 修订记录（吸收审计结论 + 九文件落点 + 关键裁定）

Stage Summary:
- lang-design v5.5 就位：next2 五轮全部增量有文档归宿；核心冻结
  不动摇（效应原语化 = Stage 2 评估对照而非当前变更）；交叉引用
  均为既有文件锚点（01§7/14§4/15§5/12§2.4.1 互链）
- 遵循原则：§8.4.5 规则 3（新内容有设计文档）、§3.3（合并>新增）、
  §17 原则 9（核心冻结）

---
Task ID: 28-c
Agent: Super Z (main) — ARCH-A/PM-A
Task: sop.md v11.2——测试入口架构意图 + Cargo.toml 精要原则

Work Log:
- §8.4.6 新增「测试入口架构意图」块（Cargo.toml = workspace 清单
  与根 package 总入口，不承载逐测试目标机械声明——[[test]] 反
  模式论证：清单与 tests/ 树双维护违反精要与单一事实源；方案 =
  runner.rs 单一总入口 #[path] mod 树，阶段/plan/gate 语义不变；
  选择性运行口径；[[example]] 例外保留）
- §8.4.6 强制规则 8（测试入口精要：禁止新增 [[test]]，新测试 =
  runner 加一行 mod + 新文件）；§8.4.6 树图注释更新；§9.1 测试树
  加 runner.rs 行 + common 单实例注记
- §16.1 变更日志 v11.2 行 + 头部 Version 11.2 + 尾部版本说明
- 工具事故：MultiEdit 多处 old_str 非唯一/部分生效（规则块标题
  多处命中）→ 改 python 行级锚点补丁（v11.1 轮同类事故——两次
  会话均遇，MultiEdit 对 sop.md 大文件多处同名锚点稳定性不足，
  已形成「python 锚点补丁优先」的操作记忆）

Stage Summary:
- sop.md v11.2 就位：测试架构意图入流程权威（§8.4.6 + §9.1 +
  规则 8），Cargo.toml 精要原则可执行可审计；lang-design v5.5
  同步登记于变更日志
- 遵循原则：§3.3（精要>冗长——单一意图块+一条规则，不发明重
  流程）、§8.6（变更日志追加）

---
Task ID: 28-d
Agent: Super Z (main) — DEV-A/QA-A
Task: 测试入口架构重构——Cargo.toml [[test]] 18 块清零 + runner.rs 总入口 + 全绿保持

Work Log:
- 新建 tests/runner.rs（18 个 #[path] mod 声明按 stage/批次分组
  + 文档注释：总入口/选择性运行/新增动作/相对 path 解析基准）
- Cargo.toml 重写测试段：[[test]] 18 块全部移除（125 行 → 50 行），
  保留 [[example]] 嵌套声明 + 新增测试入口注释（指向 sop §8.4.6）
- 全量测试首轮通过：runner 单二进制 301 集成 + 175 单元 = 476；
  逐模块计数与 r8 逐二进制口径**完全一致**（--list 逐模块统计核对）
- clippy duplicate_mod 修复：10 个测试文件原 `#[path] mod common`
  重复加载 → runner 单实例 `#[path = "common/mod.rs"] mod common`
  + 各文件 `use crate::common`（10 文件迁移；其中 reader/expander/
  compiler 3 文件实际未使用 common——清理未用 import；教训：早期
  grep head -5 截断导致首轮只发现 5/10 个引用文件，第二轮全量
  grep 补齐）
- 文档对账：testing-guide.md r9 重写（结构树 + 运行命令 + 编写
  规则「runner 加一行」）+ matrix.md r9（入口口径注记 + 逐模块
  一致性声明）+ common/mod.rs 头注（单实例共享）+ RELEASE_NOTES
  r9 段
- §3.2 六命令 clean release 实测全绿：build 8.93s / fmt OK /
  clippy 0 / test --release **476:0:0** / 审计集 41/41 EXIT 0；
  集成测试单二进制 6.95s（原 18 二进制顺序执行——合并后并行且
  零重复编译）

Stage Summary:
- 测试入口架构落地：Cargo.toml [[test]] = 0（仅剩 1 个 [[example]]
  例外声明）；runner.rs 为唯一总入口；测试总数与逐模块计数零变化
  （组织收敛零语义变化——476 保持）；共享辅助单实例化
- 遵循原则：§8.4.6 规则 8（本轮自身落地）、§3.2（六命令逐条实测）、
  §9.4.3（正负比与总数不减的硬约束——合并前后逐模块对账）、
  §2.3-4（报错>静默——duplicate_mod 当场修复非 allow 压制）

---
Task ID: 29-a
Agent: Super Z (main) — ARCH-A/REC-A（L3 多角色会话）
Task: MUV 29-a：next3.md 七轮讨论吸收进 upload/stage0.md 直至完全收敛（stage0.md v5.0 → v6.0）

Work Log:
- §0 启动协议：sop.md §1 路由 → L3 → worklog 摘 Task 28（r9 基线，无冲突）
  → next3.md 全文 2500 行读取（= next2.md 六轮 + 第七轮"内部语法的根本性重构"）
- 关键词矩阵审计（吸收前基线）：stage0.md 对 Perform/de Bruijn/MLton/Koka/ANF/
  comptime/四层正交/内部语法/派生关键词等 25 项关键词全部零命中——Thread B
  （next2+next3 七轮）完全缺失，吸收范围确认
- 吸收落位（§8.4.6 合并>新增，既有章节编号/锚点零变动）：
  - §6.9-§6.12（Part II 新四节，前五轮）：9/8/7 数量真相 + 不可消除性证明 +
    命名精确性审查 + 三方案（A 六原语/B CPS 否决/C 效应导向）；五维前沿成熟度
    矩阵 + 11 项引入/预留/忽略决策；四层正交架构 + 8 原语完整定义（+Let+Handle）
    + IR 八层形态；五弱点修正（显式 continuation 类型/组合器解耦/六 IR/MLton
    闭包三策略/Koka 四阶段效应消除）+ 可行性审查（五目标 A+/A/A/A+/B+，四风险，
    30-44 周时间线 + 口径调和）+ §6.12.6 本项目收敛裁定（9→8 语义等价映射表
    + Stage 2 评估清单登记）
  - §7.3（第六轮）：表面语法决策——S 表达式 = 工程捷径（300 vs 3000 行、皮肤/
    骨架分离、分阶段语法策略、Racket #lang 实证、与 kerf B3 自举 Reader 对照）
  - §7.4（第七轮）：内部语法设计——告别 #% 派生关键词（同像性历史妥协）、
    三原则（类型安全>命名安全/语义化命名/零冗余）、2026 内部 AST 完整 Rust
    定义（8 变体）、六维度对比、旧→新迁移映射、§7.4.4 落地口径（当前实现三项
    合规核验 + 语义化命名 Stage 2 演进目标）
  - §23.1 原则 29-31（命名行为导向/类型安全优于命名安全/表面-内部语法严格分离）
  - 附录 A +17 术语、附录 E +8 参考文献（51-58）、附录 F（七轮演进记录——
    用户要求的"演进进程"档案化：对照表 + 演进逻辑链 + 两线程关系）
  - 头部：版本历史 v6.0 条目、文档状态行 v6.0、目录附录 F、阅读指南迁移说明
  - §3.2/§3.4：批判性对照指针（冻结定义不动）
- 验收（量化）：关键词矩阵审计 25/25 零缺口（Perform 30/de Bruijn 16/MLton 17/
  Koka 15/ANF 24/comptime 8/CPS 5/四层 18/六 IR 8/内部语法 19/派生关键词 9/
  continuation 24/效应行 6/行多态 9/闭包表示 6/效应消除 12/语义化命名 13/
  类型安全 47/Roc 4/Mojo 16/Unison 4/CompCert 4/Zig 16/Branch 16/Const 22）；
  结构完整（§6.9-6.12 ×4、§7.3/§7.4 ×2、附录 F ×1、原则 29-31 ×3）；
  3706 → 4216 行

Stage Summary:
- stage0.md v6.0（原语批判演进版）收敛：next3.md 七轮全部吸收且与既有
  v5.0 结构/冻结裁定自洽（核心冻结不动摇——§6.12.6/§7.4.4 双裁定锚定）
- 收敛三裁定登记：Stage 0-1 冻结 9 原语不变；8 原语形态 + 四层/六 IR/
  MLton 闭包/Koka 效应消除 = Stage 2 演进目标；表面/内部语法三原则即刻
  生效为架构验证基准（当前实现三项合规）
- 遵循原则：§0（启动协议先于工作）、§8.4.6（合并>新增——零章节顺移，
  新内容以子节/附录扩展）、§3.3（精要>冗长——收敛化吸收而非原文搬运）、
  §2.3-11（知识搜索>猜测——7 原语不可消除性/成熟度矩阵逐项核对）、
  §17 原则 9 / §23.1 原则 9（核心冻结——收敛裁定依据）
- 下一步：MUV 29-b——stage0.md v6.0 审查融入 docs/lang-design/（v6.0）
  + 19 文件 ~12 维度系统梳理

---
Task ID: 29-b
Agent: Super Z (main) — ARCH-A/REV-A（L3 多角色会话）
Task: MUV 29-b：stage0.md v6.0 融入 docs/lang-design/（v6.0）+ 19 文件 12 维度系统梳理收敛

Work Log:
- 融合对照：next3 前五轮已由 v5.5 覆盖（01 §7 / 14 §4 / 15 §5 / 12 §2.4.1 核对无缺口），
  增量 = 第六/七轮 → 六文件回写：
  - 01-core-forms §8（新）：内部语法设计——三原则 + 当前实现三项合规核验
    （私有 ADT ✓/Reader 桥接 ✓/Span 独立 ✓）+ 旧→新迁移映射（十二行含 Require 行）
    + 表面/内部分离不变量 + §8.5 测试锚点；头部 v5.4→v6.0（兼修 v5.5 未回写版本号）
  - 02-syntax-model §8（新）：表面语法决策架构定位 + 分阶段策略指针 + §8.1 锚点
    （含 Stage 2 两语法同核验证前置登记）
  - 15-architecture-layers §5.3（新）：表面/内部语法分离 = 四层正交第五轴
    （分离轴对照表：reader/typecheck/effects/capability 四桥接点）
  - 14-design-alternatives §4.4/§4.5（新）：派生关键词 vs 类型安全 ADT 六维度对照
    + next3 七轮批判-修正-收敛逻辑链
  - 17-principles：原则 29-31 增补（命名行为导向/类型安全优于命名安全/表面-内部
    语法严格分离）+ 附二 next3 三原则对照 + **v5.5 附表四处原则编号误引修正**
    （原则 5/10/12/24 误引 → 正确条目或"实现层主题"）
  - 18-terminology §4 / 19-references §5（新）：六术语 + 三引用
    （de Bruijn 1972 / Flanagan 1993 ANF / Plotkin & Pretnar 2009）
  - 00-overview：v6.0 版本历史（补记 v5.5 目录级增量）+ 文档地图更新 + 蓝图存档注记
- **12 维度系统审查**（发现并修复 6 类缺陷）：
  ①内容对应：12 §2.5.1 悬空登记（01 §7.3 声称登记于此但不存在——v5.5 遗漏）
    → 补齐「原语演进评估清单注记」五项；裁定一致性跨 00/01/12/14/15/17 逐条核验
  ②完整性：第六/七轮全部融入，关键词矩阵零缺口（派生关键词 9/类型安全 ADT 13/
    语义化命名 10/表面语法 31/内部语法 33/迁移映射 9）
  ③④边界与职责：能力规格文件（01-06/09/11）处理程度/所属Stage/推迟项字段齐备；
    13 Version 字段污染（修订说明误并入）→ 修正
  ⑤⑥层级与时机：15 §5.3 新增第五正交轴；12 矩阵 + 评估清单就位
  ⑦自举合规：08 LLVM 策略 + sop 头部不变量 + 15"永不"声明齐备
  ⑧⑨拓展：13 §3 冻结签名（Probe 契约测试）+ 01 §8.3 迁移映射 + 15 §5.2 定量
    性能路径（1/100→1/1.2 C）
  ⑩整体性：**版本号漂移 ×3 修复**（01/18/19 头部停在 v5.0/v5.4 而内容已 v5.5）；
    **打包自包含性修复**（3 处 ../../upload/stage0.md 链接 → 上游蓝图存档至
    docs/stage0.md（243,952 B，含归档说明）+ 链接改 ../stage0.md）；链接完整性
    终验 0 断链
  ⑪测试：实现类文件锚点节齐备（01×4/02×3/03-06×2）；新增 §8.5/§8.1 锚点；
    09/13 内联锚点（negative_vm_tests/Probe）判定合格
  ⑫理论：19 §5 三引用增补 + 既有引用位置核对
- 终验：9 文件 v6.0（00/01/02/12/14/15/17/18/19），其余 10 文件维持各自版本
  （未被本轮触碰，合法）；裁定一致性（冻结 9 原语 → 00/01/14 三处声明一致）

Stage Summary:
- lang-design v6.0 收敛：19 文件全部通过 12 维度审查，缺陷 6 类全修复
- 交付物：7 文件 v6.0 更新（01/02/12/14/15/17/18/19/00）+ docs/stage0.md
  蓝图存档（自包含性）+ 13/00 修正
- 遵循原则：§8.4.5 规则 2（文档随代码——版本号漂移即违反，逐一对齐）、
  §3.3 合并>新增（新内容以子节扩展，既有结构零变动）、§2.3-4（报错>静默——
  悬空登记/误引当场修复而非容忍）、§9.4（设计-测试锚定——新增锚点两节）
- 下一步：MUV 29-c——lang-design v6.0 精华反哺 sop.md（v11.3：原则 29-31
  接入 §2.2、测试架构意图核对、§16.1 变更日志、反臃肿检查）

---
Task ID: 29-c
Agent: Super Z (main) — ARCH-A/REC-A（L3 多角色会话）
Task: MUV 29-c：lang-design v6.0 精华反哺 docs/sop.md（v11.2 → v11.3）

Work Log:
- §2.2：二十八条 → 三十一条（表格新增 29 命名行为导向/30 类型安全优于命名安全/
  31 表面-内部语法严格分离——各含违反示例列）；来源注更新（stage0.md v6.0 +
  docs/stage0.md 存档 + 17 §1.1 指针）；协同关系块补第五组
- 目录行/§2.3 引言同步三十一条；残留"二十八"扫描确认仅存于历史 changelog（合法）
- §8.4.5 文档清单 + §9.1 docs 树：登记上游蓝图存档 docs/stage0.md（v6.0——
  打包自包含性落位）
- §8.4.6 测试入口架构意图（v11.2 已就位——Cargo.toml 干净精要仅总入口）补
  「设计侧锚点接线」：lang-design 测试锚点节 ↔ tests/ 树双向锚定（§9.4）
- §16.1 changelog v11.3 条目（四点：原则扩展/存档登记/锚点接线/lang-design v6.0
  同步轮明细）+ 尾部版本注更新
- 反臃肿检查（§3.3 四问）：29-31 无重复表述（17 附表映射验证）✓ 有来源
  （stage0 v6.0）✓ 被引用（01 §8/17/02）✓ 表格形式 ✓

Stage Summary:
- sop.md v11.3 收敛：设计原则 29-31 接入 §2.2、蓝图存档入文档组织规则、
  测试架构意图完成设计侧接线
- 遵循原则：§3.3（合并>新增——新原则入既有表格而非新节；精要>冗长）；
  §8.4.5 规则 2（文档随代码——v6.0 裁定同步入 sop 裁决体系）；§16.1 变更日志
- 下一步：MUV 29-d——依 lang-design v6.0 重构 Stage 0-1（架构合规审计测试
  落地 01 §8.5/02 §8.1 锚点 + crate 边界文档化 + §3.2 六命令全绿）

---
Task ID: 29-d
Agent: Super Z (main) — DEV-A/QA-A（L3 多角色会话）
Task: MUV 29-d：依 lang-design v6.0 重构完善 kerf Stage 0-1（架构合规落地 + §3.2 全绿）

Work Log:
- 代码线（依 29-b 收敛裁定——冻结不动、三原则即刻生效为架构验证基准）：
  - tests/v0/stage0/plan/architecture_audit_tests.rs（新，+4 测试）：①十变体
    穷尽 match 冻结证明（编译期机器证明——无通配臂，新增/删除/重命名变体
    即编译失败）+ 逐变体构造字段形状锚定 + Span 独立携带全通 + kind_name
    全映射互异；②Reader 产出 Stx 类型级隔离（Vec<Stx> 绑定即断言）；
    ③Expander 唯一 Stx→CoreExpr 桥（42 → CoreExpr::Literal 翻译发生证明）；
    ④同源双编译 render_core 逐字节全等（表面语法可替换性确定性基线——
    02 §8.1 Stage 2 两语法同核验证的前置）
  - kerf-core/src/expr.rs：CoreExpr 文档块扩展（冻结边界精确化 + 三原则
    合规声明 + Stage 2 迁移映射十二行——§8.4.5 文档随代码）
  - 九 crate lib.rs 五正交轴定位文档（15 §5.3 落地：span=元数据基座/
    syntax=表面语法层/reader=轴 1 唯一桥接点/core=骨架/expander=翻译
    执行面/compiler=轴 2 桥接/vm=轴 3/4 运行时消费/runtime=通道层/
    driver=组合根）
  - runner.rs 挂载新套件（§8.4.6 规则 8——一行 mod 声明）
- 修复过程：Rc<CoreExpr> 切片模式语法错误 → first()+**first 模式；print
  缺 require 声明（E0006 能力门控正确拦截——负向自证）；clippy doc 注释
  续行 `+` 误判列表标记 → 重组措辞（§2.3-4 报错>静默，当场修复）
- 验收（§3.2 六命令实测全绿）：cargo clean ✓ / build --release ✓ /
  check 0 error 0 warning / fmt --check exit 0 / clippy -D 0 /
  test --release --workspace **480:0:0**（r9 476 + 4 审计；单元 175 +
  集成 305——逐套件实测）
- 文档同步：matrix.md r10（+4 套件行 + 总量口径 + 版本历史链）/
  RELEASE_NOTES.md r10 节（三交付 + 质量口径 + 下一步批次 E 不变）
- git commit（28 文件 +4922/-44，含 docs/stage0.md 蓝图存档与新测试文件）

Stage Summary:
- Stage 0-1 架构合规闭环：三原则（29-31）从设计裁定 → 文档锚点 →
  可执行审计测试全链落地；冻结核心经机器证明（穷尽 match）
- §3.2 全绿 480:0:0（测试数只增不减，正负比不变——新增为正向审计）
- 遵循原则：§2.2 原则 9/29/30/31（本轮落地对象）、§3.2（六命令逐条
  实测）、§8.4.6 规则 8（runner 挂载方式）、§9.4（设计-测试锚定——
  01 §8.5/02 §8.1 锚点全部兑现为可执行测试）
- 下一步：MUV 29-e——r10 打包 tar.gz（不含根 worklog.md）+ web 完整
  更新（footer 下载）+ agent-browser E2E 验证

---
Task ID: 29-e
Agent: Super Z (main) — DEV-A/QA-A/REC-A（L3 多角色会话）
Task: MUV 29-e：r10 打包 tar.gz（不含根 worklog.md）+ web 完整更新（footer 下载）+ agent-browser E2E 验证

Work Log:
- **r10 打包（§19 规则）**：kerf-stage1-v0.3.0-r10-next3absorb-langdesign-v6-archaudit-480tests.tar.gz
  （705,860 B / 212 条目）；--exclude 前置修正（tar 参数序）；**根 worklog.md
  不入包**（用户指令 v3——docs/worklog.md 已镜像 ✓）；包内容 diff 对账：
  -根 worklog.md + docs/stage0.md 蓝图存档 + architecture_audit_tests.rs
- **包内自举验证**：解压 /tmp 全新构建 → cargo test --release --workspace
  **480:0:0 与交付环境一致**；CLI 冒烟（run fib=75025 / test io 3/3 PASS）
- **download/README.md r10 节**（四交付 + 质量口径 + 自举验证记录）
- **web 更新**：kerf-data.ts（ROADMAP r9/r10 双吸收轮 + 审计轮条目、
  PACKAGE_CONTENTS 480/v6.0/存档、HERO +类型安全 ADT 徽章、Stage 0 基线
  480）+ site-footer.tsx（版本行 v6.0/v11.3/原则 29-31 + 480）；动态面
  （/api/stats 按包 mtime 自动选 r10 / /api/docs 读 v6.0 内容 / /api/download
  流式 705,860 B）零改动即生效
- **lint 全绿**（eslint exit 0）
- **agent-browser E2E（§3.2 交付后浏览器验证标准）**：页面加载零错误零
  控制台异常；stats 实时 r10（689.3 KB/480 测试/15,867 LOC/9 crates）；
  下载流 application/gzip 705,860 B ✓；Playground 黄金路径（fib 20=6765
  渲染 + ⇒ 144）；footer 版本行 v6.0/v11.3 ✓；五项内容抽查（Hero/三十一条/
  上游蓝图存档/穷尽 match/原则 29-31）全中；移动端 375×812 footer 自然
  下推无重叠；双截图存档（desktop 171 KB / mobile 79 KB——实质渲染）
- dev.log 终态：docs/stats/download/playground 四路由全 200 无错误

Stage Summary:
- r10 交付闭环：七轮吸收（stage0 v6.0 + lang-design v6.0 + sop v11.3）→
  架构合规审计（480:0:0）→ 打包（自包含 + 自举验证一致）→ web 同步
  （动态自动 + 静态更新）→ 浏览器 E2E 全过
- 遵循原则：§19（打包规则——排除项/命名/包内验证）、§3.2（交付前实测
  全绿）、浏览器验证标准（渲染/交互/数据/响应式/错误五面）、§8.6
  （worklog 三份同步——根/kerf/docs 镜像）
- 任务链 29-a~29-e 全部完成；下一会话：Stage 1 批次 E（Expander kerf
  重写 + TD-004 + TD-021）→ Stage 1 门审查（§7.3 + §21.3）

---
Task ID: 30-a
Agent: Super Z (main) — PM-A/ARCH-A/REV-A（L3 多角色会话，审计轮）
Task: MUV 30-a：next*.md 三源吸收完整性复核（关键词矩阵 + 章节结构映射 + 核心原语专项）

Work Log:
- §0 启动协议：sop.md §1 路由 → L3（审计+交付型，跨 docs/web/打包三域）→
  worklog 摘 Task 29-a~e（r10 基线，git clean 无在飞断点）→ MUV 30-a~d 四拆分
- 初轮关键词矩阵（46 词）表观缺口甄别：src=N 猜测词判为审计噪声；
  连字形式（S-表达式/9 正交原语）修正后重审
- 章节结构映射：next.md 四文档 → stage0 Part II/III + lang-design 14/13/16；
  next2 六轮 → stage0 §6.9-§6.12 + lang-design v5.5；next3 第七轮 →
  stage0 v6.0 §7.3/§7.4 + lang-design 01 §8 等
- 核心原语专项：三原则原文（类型安全而非命名安全/语义化命名/零冗余）+
  CoreExpr 八变体 + 9→8 语义等价映射表（§6.12.6）在 stage0/lang-design/sop
  三面全命中；四层正交/六 IR/MLton 闭包/Koka 效应消除/de Bruijn/continuation
  类型安全/comptime/行多态/联合交叉类型全命中
- 存档对账：kerf/docs/stage0.md ≡ upload/stage0.md v6.0（+496B 归档头，预期）；
  Janet/Céu 归入 PEG/同步语言概念族（14/19 呈现）
- sop v11.3 复核：§2.2 原则 29-31 + §16.1 changelog + §8.4.6 测试入口
  架构意图（Cargo.toml 总入口）全在位

Stage Summary:
- 三源 → 三吸收面零实质缺口（关键词矩阵 + 结构映射双重验证）；
  收敛裁定链一致（stage0 §6.12.6 / lang-design 01 §8 / sop v11.3）
- 遵循：§0（启动协议）、§8.4.5 规则 1（先查文档）、§14 审查协议（审计轮
  以 12 维度复核实质替代 §14.5——与交付轮先例一致）
- 下一步：30-b 核心原语 docs/web 双面同步核查

---
Task ID: 30-b
Agent: Super Z (main) — ARCH-A/DEV-A
Task: MUV 30-b：核心原语变动 docs/ 与 web 双面核查 + 缺口修复（P2 文档债清偿）

Work Log:
- docs 侧核查：lang-design/stage0/sop/matrix/RELEASE_NOTES 完整；发现三处
  §8.4.5 规则 2（文档随代码）违例——README.md（v5.4/v11.1/r7/476 停在 r8
  之前口径 + 缺自举 Reader/能力门控/内部效应/v6.0 原语演进）、plan.md（批次
  表止于 r8，r9/r10 两轮未登记）、status.md（测试总数 356 停在 r6）
- 修复 README：版本行 v6.0/v11.3；状态行 r9/r10 吸收审计轮；架构行 driver
  补 能力参数化/内部效应/kerf test；特性清单 +4（核心原语演进登记/自举
  Reader r6/能力门控 I/O r8/内部效应 r8）；质量状态 r10/480（--workspace
  口径注记）；markdown 链接全角括号笔误修复
- 修复 plan.md：批次表 D 与 E 之间新增「吸收/审计轮」行（r9 测试入口重构 ✅
  + r10 next3 双层吸收 + 架构合规审计 480:0:0 ✅ + 语义核心冻结零变动 +
  核心原语三面同步验收）
- 修复 status.md：header r6 → r10；测试总数 356 → 480（r5-r10 增量分项
  详列）；§5 新增 架构合规审计测试 + 核心原语演进登记 两条；Status 行
  RELEASE_NOTES r4–r10
- web 侧核查：hero 徽章（9 正交原语/类型安全 ADT）与 footer 版本行（v6.0/
  v11.3/29-31）r10 已同步；主内容区缺口——核心原语仅存在于 footer 包清单
  括号注记，架构/能力两个 section 无正面呈现（不满足「完整且清晰」）
- web 修复：kerf-data.ts 新增 CORE_PRIMITIVES（9 项：名称/元数/语义——与
  lang-design 01 §2 逐项对齐）+ PRIMITIVE_EVOLUTION（收敛裁定/数量真相/
  8 行迁移映射/三原则/四层正交+第五轴/审计 4 测试）；capabilities.tsx 新增
  「核心原语 · 语义核心」网格区块 + 「核心原语演进登记」卡（映射表 + 三原则
  列表 + 正交注记）；ROADMAP Stage 2 补核心原语演进迁移要点；section 标题
  与描述更新；lint exit 0

Stage Summary:
- docs 三文件 P2 债清偿（§8.4.5）；web 核心原语从 footer 注记升级为主内容
  区正面呈现（9 原语网格 + 演进登记卡 + 路线图要点）
- 遵循：§8.4.5 规则 2（文档随代码——r6-r10 陈旧一次对账）、§2.3-4（报错
  > 静默——测试口径 --workspace 显式注记）、§14.8 设计回写精神
- 下一步：30-c §3.2 全绿 + r11 打包

---
Task ID: 30-c
Agent: Super Z (main) — QA-A/REC-A
Task: MUV 30-c：§3.2 六命令实跑全绿 + RELEASE_NOTES r11 + git commit

Work Log:
- cargo clean（Removed 400 files, 82.1MiB）→ build --release 9.37s 零告警
- cargo check 0/0；cargo fmt --check 零 diff exit 0；cargo clippy
  --all-targets -- -D warnings exit 0
- cargo test --release --workspace **480:0:0**（305 集成 + 175 单元；根
  package 单跑 305 属预期组织形态——runner.rs 单一总入口挂载全部集成文件）
- 审计集 41 case EXIT 0（§7.1.1 七类覆盖 1/5/1/4/16/1/2——配比全满足）
- CLI 冒烟：kerf run fib.krf → 75025 + ⇒ 144；kerf test → 通过 2/共 2
- RELEASE_NOTES.md r11 段（复核轮交付三节 + 质量口径 + 下一步批次 E）
- git commit 6f13dad：docs(r11) 核心原语三面同步（4 文件 +46/-11）

Stage Summary:
- §3.2 六命令全绿实测（480:0:0 + 审计 41 EXIT 0 + CLI 双冒烟）；
  r10 质量基线保持零回归
- 遵循：§3.2（六命令逐条实测——含 clean 全量重编）、§7.3.1（审计集
  release 复跑）、§19.3 打包前检查（commit + RELEASE_NOTES + 版本）
- 下一步：30-d web E2E + worklog + r11 tar.gz

---
Task ID: 30-d
Agent: Super Z (main) — DEV-A/REC-A
Task: MUV 30-d：web E2E 验证 + worklog 双镜像 + r11 打包交付

Work Log:
- agent-browser E2E：页面加载 0 console/0 页面错误；核心原语区块全文本
  渲染（核心原语 · 语义核心/Lambda/SetBang/演进登记/语义核心不变/三原则/
  四层正交/数量真相 全 true）
- Playground 金路径：运行 fib → 6765 + ⇒ 144（POST /api/playground 200）；
  静态检查错误程序 → E0005 全格式渲染（--> 3:6 + 源摘录 + ^ 标记 + 汇总
  行「发现 1 个静态问题」；POST /api/check 200）
- 文档区 01-核心原语 加载（九个核心原语/内部语法设计 命中）；stats API
  实时（testCount 480——README 对账自动生效）
- 响应式 390px：scrollWidth=390 零横向溢出；footer 滚底双真
  （atDocBottom + footerAtViewportBottom）；桌面 1280 同过
- VLM 双截图审查：桌面 PASS（3x3 网格对齐/表格完整/对比度达标）+ 移动
  PASS（双列换行/文字完整）；lint exit 0
- worklog：kerf/worklog.md 30-a~30-d 四节 + docs/worklog.md 镜像 +
  根 worklog.md 摘要；r11 tar.gz 打包 + download/README.md r11 节

Stage Summary:
- E2E 全绿（渲染/交互/响应式/console 四面 + VLM 视觉双审）；任务链
  30-a~d 闭环：吸收复核零缺口 → 三面同步 → §3.2 全绿 → 打包交付
- 遵循：§8.6（worklog 协议 + 镜像）、§19（打包 + 包内自举验证）、
  浏览器验证标准（金路径 + 静态检查 + 文档 + 下载四流）
- 下一步（批次 E，plan §5）：Expander kerf 重写 + TD-004 scope-set
  收口 + TD-021 hof 用户面注入 → Stage 1 门审查（§7.3 + §21.3 四条）

---
Task ID: 31-a
Agent: Super Z (main) — ARCH-A（L3 多角色会话，吸收轮）
Task: MUV 31-a：next4.md → upload/stage0.md v6.1 吸收（2026 接口预留完整性审查）

Work Log:
- §0 启动协议：磁盘态验证（upload/ 四源文件 + git a472d10 clean）→ sop.md §1
  路由 L3 → worklog 摘 Task 30-a~d（r11 复核交付轮闭环）→ Task 31 六 MUV 拆分
- next4.md 全文精读（415 行——第八轮：既有预留覆盖 ~70%，缺失 30% = 10 个
  关键接口 + P0-P3 策略 + ~3 周成本 vs 数月级破坏性重构）
- stage0.md v6.0 → v6.1 编辑（4216 → 4757 行，+541 行 / +18.5KB）：
  ① 头部/版本历史/迁移注记（v6.1 条目 + 从 v6.0 迁移读者路径）
  ② §7.1.1 三层矩阵：LSP/FFI 重分类 + 6 新行（调试/增量查询/服务化/多后端/
  包管理/AI）→ 预留层 4→14 项；§7.1.2 前增补说明
  ③ §7.2 接口先行原则扩容 + 章末 §9.4/原则 32 裁决基线注记
  ④ §9 章首 v6.1 增补说明 + §9.1 尾交叉引用（缺失需求承接表）
  ⑤ §9.2 边界调和（LSP/FFI 移出完全推迟 + 语义精确化：未承诺采用 vs 期票）
  ⑥ 新增 §9.3（9.3.1 覆盖度评估表 + 9.3.2-9.3.9 八接口详细设计 + 9.3.10
  效应扩展/能力委托边界增补）+ §9.4 完整预留矩阵（14 项唯一权威清单）+
  §9.5 优先级策略（P0-P3 表 + 3 周成本核算 + 全景 mermaid）
  ⑦ §13.1 查询式架构与 §9.3.5 互链（数据面 + 架构面双预留）
  ⑧ 新增 §21.9 接口预留成本核算（成本/净影响关系表 + 处理程度标度接口维度
  延伸：能力引入时机 vs 数据结构冻结时机）
  ⑨ §23.1 三十一条 → 三十二条（原则 32「预留留白原则」）
  ⑩ 附录 E.8 工具链生态接口文献（LSP/DWARF/WASM 组件模型/rust-analyzer
  #59-62）+ 附录 F 第八轮登记（F.1 表 + F.2 链 + F.3 线程关系）
- 吸收完整性审计（沿用 30-a 方法）：关键词矩阵 74/74 全命中（含全部 trait/
  方法名/成本数字/引句）；§9 结构 9.1-9.5 完整无重复节号；160 代码栅栏
  平衡；14 mermaid 块（+1 全景图）；伪代码 → 规范化 Rust（→ 箭头修正、
  补签名，与 §9.1 既有风格一致）
- 收敛裁定对齐：§9.5.3 图标注采用 §6.12.6 口径（Stage 0-1 冻结 9 原语 /
  Stage 2 演进 8 原语形态）；FFI 预留加自举合规注记（不入自举链，§21 同口径）

Stage Summary:
- stage0.md v6.1 收敛完成：next4.md 第八轮意图与精华 100% 吸收（关键词矩阵
  零缺口 + 结构映射完整）；接口预留层 4→14 项、完全推迟层 8→6 项；
  §9.4 矩阵为预留层唯一权威清单
- 遵循：§0（启动协议）、§8.4.5 规则 1（先查文档——比对 v6.0 既有裁定链）、
  §2.3-9（正确>妥协——LSP/FFI 重分类直面矛盾而非含糊并存）、§14 审计
  精神（关键词矩阵 + 结构双验证）
- 下一步：31-b v6.1 → kerf/docs/lang-design/ 融合 + 20 文件系统审查

---
Task ID: 31-b
Agent: Super Z (main) — ARCH-A/REV-A（L3 多角色会话，融合审查轮）
Task: MUV 31-b：stage0.md v6.1 → kerf/docs/lang-design/ 融合 + 20 文件系统性审查

Work Log:
- 归档同步：kerf/docs/stage0.md 再生为 v6.1（归档说明更新——镜像声明 20 文件 v6.1 同步）
- 13-capability-matrix（主体融合，v5.5 → v6.1，19.7KB → 36KB）：
  ① 头部版本/日期；② §1.1.1 矩阵 LSP/FFI 重分类 + 6 新行（保留 r7/r8 做实注记）
  ③ §1.1.2 前增补说明；④ §1.2 接口先行原则扩容 + 章末 §3.4/原则 32 裁决基线
  ⑤ §3 章首 v6.1 增补说明 + §3.1 尾缺失需求承接交叉引用
  ⑥ §3.2 边界调和（LSP/FFI 移出 + 完全推迟语义精确化 + 类型检查器 r7 注记保留）
  ⑦ 新增 §3.3-§3.5（14KB——从 stage0 §9.3-§9.5 引用适配转换：§8.x→§2.x、
  §13.1→15 §3.1、§11→08、§21.x→12 §2.x、原则→17、锚点全链转换，
  零残留 §9.x 引用）；⑧ §3.3 测试锚点注记（reserved.rs Probe 先例 + r12 交付）
- 12-roadmap：新增 §2.9 接口预留成本核算（21.9 适配）；Phase 4 描述 4→14 项更新
- 10-toolchain：LSP 重分类注记（LanguageService/IncrementalAst trait + P0）
- 08-backend-evolution：CodegenBackend/WasmBackend 预留注记（LLVM 不入自举链不变）
- 15-architecture-layers：§3.1 查询双预留互链 + §1.1 总图预留层 4→14 注记
- 17-principles：原则 32「预留留白」+ 附三 next4 对照（落地状态三行）
- 00-overview：v6.1 修订记录（吸收内容/回写落点七处/关键裁定三段式）
- 19-references：§6 next4 引用增补（LSP/DWARF/WASM 组件模型 #63-65；
  rust-analyzer 已在 §3.6 不重复）；18-terminology：§5 增补术语 13 条
- 系统性审查（12 维度）：关键词矩阵 29/29 全命中；LSP/FFI 陈旧「完全推迟」
  行 0；全 20 文件内锚点 + 跨文件锚点经 GitHub 算法校验零坏链；代码栅栏
  全平衡；版本声明分级合理（未触及文件保留自身版本史）

Stage Summary:
- lang-design 全集 v6.1 收敛完成：主体落位 13（矩阵 14 项 + §3.3-§3.5 完整
  审查），联动七文件；三面裁定链一致（stage0 v6.1 §9.4 / lang-design 13 §3.4 /
  sop 待 31-c）；零陈旧引用零坏链
- 遵循：§8.4.5 规则 2（文档随代码同步）、§2.3-9（正确>妥协——重分类直面
  边界矛盾）、§14 审计（关键词矩阵 + 锚点全链校验）、12 维度审查
- 下一步：31-c 精华反哺 sop.md（接口预留原则 + §21.9 时机条款）

---
Task ID: 31-c
Agent: Super Z (main) — ARCH-A/流程维护者（L3 多角色会话，反哺轮）
Task: MUV 31-c：lang-design v6.1 精华反哺 sop.md（v11.3 → v11.4）

Work Log:
- §2.2 设计原则三十一条 → 三十二条：新增第 32 条「预留留白原则」（含违反
  示例两则——以功能未实现为由拒绝冻结形状 / 预留实现成半成品）；来源注记
  更新（stage0 v6.1 + next4 第八轮 + 17-principles 附三指针）
- 新增 §21.12 接口预留时机（流程接入四条）：① 与 §21.7-§21.11 的叠加规则
  （能力引入时机=实现推迟可协商；数据结构冻结时机=P0 必须Stage 0/P2 必须
  Stage 1，不可协商）② §13.1 设计对齐接线（AST/IR/架构数据结构新设计须
  显式声明 P0 预留挂点）③ §6.2 技术债接线（预留位置缺失按 P1 登记，
  禁因「功能未实现」降 P3）④ 预留成本口径（~3 周 / 裁剪仅限 P2/P3）
- §21.11 风险表对账（§8.4.5 规则 2 冲突修复）：「接口预留过多」行四项核心
  清单 → 14 项矩阵清单（裁剪须走 §13.1 重裁）；「接口预留不足」行三项
  不动摇 → P0 三项 + 既有四项不动摇
- §8.4.5 查询时机表新增「修改/新增接口预留」行（13 §3.4 唯一权威清单 +
  §3.5.1 优先级）；§8.4.1 目录树与 §8.1 文档清单蓝图存档 v6.0 → v6.1
- §16.1 changelog v11.4 行（五点变更）；文档尾版本链注记前插 v11.4 摘要
- 完整性验证：§2.2 表行 28-32 全在位；§21.12 位置正确（§21.11 后、footer
  前）；代码栅栏计数与改动前一致（存量 127，行级深度平衡——非本轮引入）

Stage Summary:
- sop.md v11.4 完成：接口预留精华四点接入（原则 32 / §21.12 流程接线 /
  风险表对账 / 查询表登记）；与 stage0 §23.1 / lang-design 17 §1 三面
  原则编号链一致（29-31 → 32）
- 遵循：§8.4.5 规则 2（文档与设计冲突——风险表旧口径本次修正）、
  §3.3（演进原则：合并>新增——接入既有 §13.1/§6.2/§21 节点而非新开协议）、
  原则 32 自身（本条款以预留挂点方式接入既有流程结构）
- 下一步：31-d reserved.rs 扩展 10 接口冻结实现

---
Task ID: 31-d
Agent: Super Z (main) — DEV-A/QA-A（L3 多角色会话，代码轮）
Task: MUV 31-d：预留层代码冻结——reserved/ 模块 4→14 项接口 + Probe 冻结测试 + P0 位置断言

Work Log:
- §8.4.6 落位裁决：reserved.rs 单文件 → reserved/ 模块目录（mod.rs 保留
  既有四接口零迁移 + toolchain.rs/ffi.rs/codegen.rs 三子模块）——
  crate::reserved::X 路径全兼容，lib.rs 零改动
- toolchain.rs（~740 行）：P0 三项 LanguageService+IncrementalAst /
  DebugInfoGenerator+DebugTraceable / QuerySystem+Query（Query::execute
  以 dyn QuerySystem<Input=Self::Input> 修正对象安全）；P1 CompilerService
  +Serializable；P2 PackageManager+ExternalModule+AiAssistant；共享形状
  类型复用 kerf 既有真实类型（FileId/Span/Diagnostic/NodeId/Symbol/
  BcProgram）+ 20 个新形状类型（Position/Location/TextEdit/...）
- ffi.rs（~200 行）：ExternalType（递归 C 类型子集）/FfiCall（外部调用
  核心表达式表示——args 以 CoreExpr 承载）/FfiBoundary（GC pin/unpin
  隔离协议 + 自举合规注记：不入自举链）；codegen.rs（~180 行）：
  CodegenBackend/WasmBackend + AnnotatedANF 占位（原则 13 目标中立的
  类型级证明——compile 入参是 IR 非 CoreExpr）
- 集成测试 tests/v0/stage1/plan/reserved_ext_tests.rs（12 case，经
  runner.rs 总入口挂载——§8.4.6 强制规则 8）：P0 位置跨 crate 断言
  （Span/scopes/NodeId/protos/ext3/内容寻址统一）+ 14 项 API 外部
  可达性 + 形状行为（含负向：空目标拒绝/空片段类型检查失败/rename 未
  实现错误面）
- 单元测试 +8（Probe 冻结——toolchain 3 + ffi 3 + codegen 2，沿用
  reserved_signatures_are_frozen 先例）
- 修复三处编译问题：TargetTriple 补 Default；doc 懒续行 clippy；mod.rs
  再导出补包管理 6 类型 + NodeId 类型别名用法（u32 构造 → 字面量）
- 验证：cargo check/clippy -D/fmt --check 全绿；test --release
  --workspace **500:0:0**（480 基线 + 20：单元 183 + 集成 317）；
  审计集 §7.3.1 配比满足 EXIT 0；CLI 冒烟 fib 75025 + test 3/3
- 文档同步（§8.4.5 规则 2）：README（v6.1/v11.4/500 + 特性清单 +接口
  预留完整性扩展）+ status.md（500 + r12 增量分项）+ plan.md（r12
  吸收轮行）+ RELEASE_NOTES（r12 三节交付）
- 效应扩展/能力委托（13 §3.3.10 裁定）：mod.rs 文档登记扩展方向，
  不新增数据结构位置——按裁定执行零代码

Stage Summary:
- 预留层 14 项全部代码冻结（P0 三项 trait + 位置对账六项 ✓ + P1 三组 +
  P2 两组）；500:0:0 全绿零回归；语义核心与既有四契约零变动
- 遵循：§2.2 原则 27/28/32（接口稳定/渐进替换/预留留白——形状冻结
  而非实现）、§8.4.6（落位 + 测试入口架构）、§10.1 规则 4（显式
  re-export）、§9.4（设计-测试锚定——13 §3.3 测试锚点注记兑现）
- 下一步：31-e worklog 管理机制设计（研究文档吸收 + docs/worklog/ 初始
  结构 + sop §8.6 扩展）

---
Task ID: 31-e
Agent: Super Z (main) — ARCH-A/PM-A（L3 多角色会话，机制设计轮；前置研究 31-e-research 已由子代理完成）
Task: MUV 31-e：worklog 管理查询机制设计（docs/worklog/ rec 树）+ sop v12.0 集成 + 初始树落地

Work Log:
- 知识检索（用户指令要求）：31-e-research 子代理 22 次 web_search + 2 次深读
  （Letta Filesystem 基准文全文），产出 docs/develop/v0/worklog-mechanism-research.md
  （141 行：五领域 12 条借鉴理念 + 需求映射分析 + 10 条设计建议 + 三缺口）
- 核心借鉴吸收：① LSM 追加写/分层压实/旧层不可变 → 三级读写路径；
  ② Letta 文件记忆实证（grep 原语 74.0% > Mem0 68.5%）→ 不引入向量库；
  ③ B+ 树稀疏索引 → l = 每层内部节点物化（用户方案最亮点确认）；④ 时间
  分区裁剪 → 目录名即分区键；⑤ JD 编号 → 字典序=时间序 + 扇出 ≤12；
  ⑥ ADR supersede → 修正不改历史；⑦ salsa 红/绿 → stale 对账可检测；
  ⑧ 记忆三分 e/s/p → by-topic 第二入口补时间分区盲区
- sop.md v11.4 → v12.0：§8.6 扩展为三路径协议（§8.6.1 写路径逐字不变 +
  §8.6.2 rec 树文法 + §8.6.3 l 稀疏索引格式（≤8KB/扇出行数解耦）+
  §8.6.4 三原语检索 + 冷启动 + §8.6.5 压实规则（时机挂 §19/§14.9/§6.3
  既有节点 + stale 检测 + 里程碑不可变）+ §8.6.6 与既有协议关系表）；
  §8.4.1 目录树 + §8.4.5 查询时机表接线；§16.1 v12.0 行；footer 版本链
- 初始树 bootstrap（docs/worklog/）：根 l（2.4KB——未压实区间 + 树导航 +
  by-topic 决策视图 6 行 + procedural 速查）+ 00_stage-0_语义验证/
  （3 条目 r1-r3 + l 846B，封存只读）+ 01_stage-1_能力引入/（9 条目
  r4-r12 + l 1771B，active + 未压实区间行）；条目 = 压实摘要（1:8 压缩
  自 RELEASE_NOTES/flat Stage Summary）+ 溯源指针（Task ID → flat 节）
- 三原语检索集成验证：① ls 枚举字典序=时间序 ✓；② 读 l 定位「预留 14
  项」→ r12 行 ✓；③ grep -r 主题查询「TD-002 何时定的」→ r5 条目 +
  l ✓、「自举边界」→ r6 条目 ✓（冷启动协议实操走通）
- 打包含盖确认：§19.4 命令整目录含 docs/ → 树自动入包；根 worklog.md
  排除不变；§8.4.1 树与根 l 体积均 ≤8KB 上限

Stage Summary:
- worklog 机制「文件/目录型知识数据库」设计闭环：研究 → 机制（时间×精度
  双轴 + l 路由 + 三原语 + 压实对账）→ sop 规范（v12.0）→ 初始树落地
  （12 条目）→ 检索实证；flat 协议零破坏（写路径不变）
- 遵循：§3.3 演进原则（合并>新增——压实时机挂既有节点不新增会议；
  精要>冗长——normative 条款入 sop、研究佐证留 develop 文档）、
  §2.3-11（知识搜索>猜测——先检索后设计）、§8.6 自身（树条目含
  溯源指针 + stale 边界）
- 下一步：31-f §3.2 全绿 + r12 tar.gz + web 更新 + E2E + r12 终态压实

---
Task ID: 31-f
Agent: Super Z (main) — QA-A/REC-A（L3 多角色会话，交付轮；含 r12 终态压实）
Task: MUV 31-f：§3.2 全绿 + r12 tar.gz + web 同步 + E2E + r12 终态压实

Work Log:
- §3.2 六命令实跑全绿（clean 起步）：clean（470 files/127MB）→ build
  --release 9.20s 零告警 → check 0/0 → fmt --check 零 diff → clippy
  --all-targets -D warnings 0 → test --release --workspace **500:0:0**
  （单元 183 + 集成 317）
- 审计集 §7.3.1 配比满足 EXIT 0；CLI 冒烟：fib 75025 + ⇒ 144 + kerf
  test 3/3（clean 重编后复跑）
- r12 tar.gz 打包：780,157 B / 230 条目（根 worklog.md 排除 ✓；docs/
  worklog/ rec 树入包 ✓）；**包内自举验证**：解压 → 500:0:0 + CLI 一致
- 文档债清偿（31-f 发现，§8.4.5 规则 2）：docs/tests/matrix.md 停在 480
  （31-d 遗漏）→ 500 全量对账（headline/历史链/单元 183 分项/driver 58/
  集成 317/reserved_ext_tests 套件行 + 负向形状 case 行）
- web 同步：kerf-data（RESERVATIONS 4→14 卡——P0 三项含位置对账 + P1
  三组 + P2 两组 + 增补两项；ROADMAP Stage 0/1/2 更新——r12 吸收轮 +
  预留冻结轮双行 + 500 基线；PACKAGE_CONTENTS 500/v6.1/worklog 树；
  HERO +「接口预留 14 项」徽章）+ capabilities（14 预留卡注释）+
  site-footer（v6.1/v12.0/500/原则 29-32/树）；动态面自动：stats 实时
  500 + r12 包（761.9 KB）/ docs 读 v6.1 / download 流 r12
- agent-browser E2E：页面加载零错误零 console 异常；14 项预留内容全
  渲染（LSP/IDE 查询/增量编译查询/DebugInfoGenerator/接口预留 14 项/
  预留留白 全 true）；footer 版本行 v6.1/v12.0/500 true；stats API
  testCount 500 + packageName r12
- r12 终态压实（§8.6.5 首次实操）：09_r12 条目扩展（31-e/31-f 增量 +
  覆盖 Task 31-a~f）+ 层 l 行更新 + 根 l 未压实区间清零（flat 与树均
  压实至 r12 终态）+ by-topic 增 worklog 树机制行
- lint exit 0；download/README.md r12 节（五要点——三层吸收/代码冻结/
  树机制/六命令/包内验证）

Stage Summary:
- r12 交付闭环：三层吸收（stage0 v6.1 + lang-design v6.1 + sop v12.0）
  → 预留层代码冻结（500:0:0）→ worklog 树机制 → 打包（含包内自举验证
  与 rec 树）→ web 同步 → E2E 全过 → 终态压实（树协议首次完整走通）
- 遵循：§3.2（六命令逐条实测——clean 全量重编）、§19（打包 + 包内
  验证 + README 归档）、§8.4.5 规则 2（matrix.md 陈旧对账）、§8.6.5
  （压实规则首次实操——stale 区间清零）、浏览器验证标准（渲染/数据/
  交互/响应式）
- 下一步（批次 E，plan §5）：Expander kerf 重写 + TD-004 scope-set
  收口 + TD-021 hof 用户面注入 → Stage 1 门审查（§7.3 + §21.3 四条）
- E2E 修复附录（31-f 内补）：agent-browser 移动端 390px 检出横向溢出
  （scrollWidth 716 > 390）——根因 = 本轮新增 RESERVATIONS 14 卡网格中
  无空格长函数串（93 字符 AI 五方法链）不可折行 + grid item
  min-width:auto；修复 = Card 加 min-w-0 + desc 段落加 break-words
  （capabilities.tsx），修复后 scrollWidth=390 零溢出；另将演进表
  wrapper 加 min-w-0（存量隐患顺带修复）；Playground 金路径（fib
  递归 → 运行 → 6765 + 144）通过；footer 滚底双真（bottom=900=
  viewport）；console/页面错误 0；VLM 双截图审查全 PASS（桌面 4/4 +
  移动 3/3——卡片对齐/文字完整/表格可读/无溢出）


---
Task ID: 32-a
Agent: Super Z (main) — ARCH-A/PM-A 定调 + DEV-A 实施（L3 多角色会话，批次 E 首个 MUV）
Task: MUV 32-a：TD-004 作用域集解析收口（Racket 式 (name, scopes ⊆) 双路径——E1 Expander kerf 重写前置语义基座）

Work Log:
- 启动协议执行：§1 路由 + 7 项自我定位声明（L3：LOC ≥500 跨 5 crate /
  中枢类型变更）；worklog 树冷启动（根 l + flat 尾部）确认 r12 终态、
  批次 E 为登记的下一步、无冲突；用户任务 1（worklog 机制迁移）复核
  = 已完成（31-e 交付，树文法/路由/未压实区间零/by-topic 全在位）
- 设计（§2.3-11 边界先行 + §12 最优>最小）：scope-set 语义权威 +
  空作用域集全局兜底（空集 ⊆ 任意引用集 ⇒ 全局/内置/`$hyg$` 基名
  回退自动降级为回退路径）+ α 重命名保留为第二道卫生保险；注入
  不变式（绑定形式 fresh scope 深注入绑定器+全体体形式 ⇒ 内层绑定
  作用域集严格包含外层 ⇒ max-cardinality 与帧序在良构程序上同解）
- 实施（跨 5 crate）：
  - kerf-syntax：`Stx::add_scope_to_all` 深注入公共 API
  - kerf-expander：`ExpandCtxt` 作用域分配器（fresh_scope 单调
    递增）+ `expand_lambda` 注入（先于体展开——宏产物作用域 ⊇
    use-site ⊇ {fresh}，自由标识符穿透保持）+ VarRef 发射携带
    scopes + SetBang 目标携带 scopes + parse_params 返回
    (Symbol, ScopeSet)
  - kerf-core：`VarRef/SetBang.scopes` + `Lambda.param_scopes` +
    `free_var_occurrences`（捕获分析数据源；名称版 = 其投影——
    单一定义防双体系漂移）
  - kerf-compiler：ScopeFrame 绑定项化 + `resolve_var(name, scopes)`
    帧序 + 帧内 max-cardinality 子集匹配 + 捕获描述符携带命中绑定
    作用域集（内层原型经同一子集匹配命中捕获槽）
  - kerf-vm：`ClosureValue::Eval.param_scopes` + `Env` 绑定项化 +
    `lookup/set` 同一子集口径 + `define_scoped`（apply 绑定形参）
- 测试：新增 tests/v0/stage1/plan/scope_set_tests.rs（9 锚点：双路径
  正例 5——shadowing/嵌套 shadowing/闭包捕获/set! 词法命中/子集
  对照；负例 4——作用域不匹配未绑定 VM/eval/set! + 宏引入不捕获）
  + runner.rs 批次 E 分组注册；构造点批量补齐（vm.rs/capability.rs/
  code_value.rs/ir.rs/tests 树 perl 批处理）
- 文档同步（§8）：tech-debt-register TD-004 → 已解决（r13）+ matrix
  500→509 对账（总量/历史链/集成 317→326/套件行）+ 03-macro-system
  实现状态注记（ctx.resolve 伪代码处）+ stage-1/plan.md 批次 E 行
  更新 + docs/tests/v0/stage1/plan/scope-set.md 测试计划 + RELEASE_NOTES
  r13 节

Stage Summary:
- TD-004 P2 清偿闭环：设计 → 跨 5 crate 实施 → 500 基线零回归 +
  9 锚点（509:0:0）→ 文档六处同步；语义判别点落地 = 名称匹配但
  作用域不匹配按 Racket 语义判未绑定（旧名称基会误命中）
- 遵循：§2.3-10（唯一可信数据源——free_variables 投影自单一定义）、
  §2.3-11（确定性边界先行——注入不变式先证明后实施）、§11（接口
  隔离——解析留在编译器/eval 消费侧，IR/字节码零变化）、§9.4
  （设计-测试锚定——每个语义判别点一个锚点测试）
- 下一步：32-b 交付环（§3.2 全绿 + r13 tar.gz + web 同步 + E2E +
  树压实）

---
Task ID: 32-b
Agent: Super Z (main) — QA-A/REC-A（L3 多角色会话，交付轮；含 r13 树压实）
Task: MUV 32-b：§3.2 全绿 + r13 tar.gz 打包（包内自举验证）+ web 同步 + E2E + r13 树压实

Work Log:
- §3.2 六命令实跑全绿（clean 起步）：build --release 零告警 → check
  0/0 → fmt --check 零 diff → clippy --all-targets -D warnings 0 →
  test --release --workspace **509:0:0**（单元 183 + 集成 326——500
  基线零回归 + r13 锚点 9）
- 审计集 §7.3.1 配比满足 EXIT 0（七类全覆盖）；CLI 冒烟：fib 75025
  + ⇒ 144 + kerf test 2/2（clean 重编后复跑）
- r13 tar.gz 打包（§19.4 命令整目录；根 worklog.md 排除 ✓；
  docs/worklog/ rec 树 + docs/worklog.md 镜像入包 ✓）+ 包内自举
  验证（解压 → 509:0:0 + CLI 一致）
- web 同步：kerf-data（ROADMAP Stage 1 批次 E 行——r13 TD-004 收口 +
  509 基线；PACKAGE_CONTENTS 509/r13）+ site-footer 版本链 + download/
  README.md r13 节 + 动态面（stats 509 + r13 包 / docs / download 流）
- agent-browser E2E：页面加载零错误零 console 异常；r13 新内容渲染
  核对（footer 版本 509/批次 E）；Playground 金路径复跑
- r13 树压实（§8.6.5）：10_r13 条目（32-a/32-b + 溯源指针）+ 层 l
  行更新 + 根 l 未压实区间清零 + by-topic 增「scope-set 解析」行
- lint exit 0（web 工程）

Stage Summary:
- r13 交付闭环：TD-004 收口（509:0:0）→ 打包（包内自举验证 + rec
  树）→ web 同步 → E2E → 树压实（第二次完整实操——冷启动即见 r13）
- 遵循：§3.2（六命令逐条实测——clean 全量重编）、§19（打包 + 包内
  验证 + README 归档）、§8.6.5（压实规则——时机挂 §19 打包轮）、
  §8.4.5（文档对账——matrix 509 分项）
- 下一步（批次 E 续）：Expander kerf 重写（E1——Rust 实现保留为
  parity oracle）+ TD-021 hof 用户面注入（随模块系统设计）→ Stage 1
  门审查（§7.3 + §21.3 四条验收）

---
Task ID: 33-a
Agent: Super Z (main) — ARCH-A/PM-A 定调 + DEV-A 实施（L3 多角色会话，批次 E 续）
Task: MUV 33-a：E1-α 自举 Expander——expander.krf（核心形式 + 九糖 + 提升 + 作用域注入 + quote）+ bootstrap_expander.rs 值桥 + parity 19 测试（影子路径）

Work Log:
- 启动协议轻量复验（同会话续轮）：磁盘 = r13 终态、未压实区间零、无
  冲突；E1 为 32-b 登记的下一步；7 项声明（L3：expander.krf ~910 行
  + 桥 ~500 + 测试 ~500，跨 driver/expander 语义边界）
- 设计（r6 自举 Reader 先例同型三件套 + oracle 逐字镜像策略）：
  - 节点格式 (tag s e scopes ...)——作用域集 int 列表随节点携带
    （物理复制语义镜像：构造节点 ∅ 起步，与 r13 注入语义一致）
  - ctx 不线程化——镜像 Rust 节点级注入/retag（糖产物 use-site 替换
    语义；提升路径构造 ∅ 无 retag）
  - 糖引入名 loop$hyg$1 常量（HygieneCtx 每实例化计数重置的镜像）
  - E1-α 边界：define-syntax 显式报错（宏 E1-β）；expansion_id 不
    参与 parity 判据（r6 同口径）
- 实施：expander.krf（9 核心形式 + 九糖 + 内部 define 提升含切分
  期名/值校验错误短路序 + fresh-scope 深注入 + trampoline + retag +
  quote datum 转换——$hyg$ 剥离经字符表模式匹配绕开 num 算术索引的
  保守类型检查边界；node-field 经 drop-k cdr 步进同因）；种子管线
  编译 208 原型 / 4685 指令（迄今最大 kerf 程序）
- 桥：bootstrap_expander.rs——Stx→VM datum 节点→VM core 节点→
  CoreExpr（作用域集排序去重重建；共享 bootstrap.rs 走查辅助 pub(crate)
  化——§2.3-10 单一定义）；IoGrant::none() pub 化（接口最小放宽）
- 实测驱动的缺陷修复（§2.3-11 先实测禁臆测——全部经 parity 实跑
  发现）：①let 形式括号缺失（body 被吞入绑定组）②'true/'false/'nil
  是字面量非符号——tag 比较恒假（改 string->symbol 铸造 + 字符串
  比较）③desugar-letrec 的 s/e 先用后绑 ④module-header import/export
  名单误取首名（car r → r）⑤点对语料移除（reader 不支持）⑥类型
  检查 num 联合边界两处重写（上）
- parity 19 测试 + runner.rs 注册：结构（原语+Span+作用域集+
  param_scopes 递归）/错误（消息+Span 逐字——含提升短消息口径）/
  边界（define-syntax）/行为面（产物经 compile+VM 与种子全管线同果）
- 文档同步：matrix 509→528 对账 + plan.md 批次 E 行 + bootstrap-
  expander.md 测试计划 + RELEASE_NOTES r14 节 + 07-bootstrap 进度标记

Stage Summary:
- E1-α 交付闭环：设计→自举程序（208 原型）→桥→parity 19 全过
  （528:0:0 零回归）→文档五处同步；「语言能表达自身前端」的自举
  命题在 Expander 核心子集上成立（行为面端到端证明）
- 遵循：§9.4（设计-测试锚定——oracle 逐 span/逐消息镜像）、
  §2.3-11（先实测禁臆测——六类缺陷全部 parity 实跑发现）、§2.3-10
  （走查辅助单一定义）、§11（IoGrant 接口最小放宽）
- 下一步：33-b 交付环（§3.2 + r14 tar.gz + web + E2E + 树压实）

---
Task ID: 33-b
Agent: Super Z (main) — QA-A/REC-A（L3 多角色会话，交付轮；含 r14 树压实）
Task: MUV 33-b：§3.2 全绿 + r14 tar.gz（包内自举验证）+ web 同步 + E2E + r14 树压实

Work Log:
- §3.2 六命令实跑全绿（clean 起步）：build --release 零告警 → check
  0/0 → fmt --check 零 diff → clippy --all-targets -D warnings 0 →
  test --release --workspace **528:0:0**（单元 183 + 集成 345——
  509 基线零回归 + E1-α parity 19）
- 审计集 §7.3.1 配比满足 EXIT 0；CLI 冒烟：fib 75025 + ⇒ 144 +
  kerf test 2/2 + expander.krf 自检 ok（208 原型/4685 指令）
- r14 tar.gz 打包（§19.4 命令整目录；根 worklog.md 排除 ✓；
  docs/worklog/ rec 树 + 镜像入包 ✓）+ 包内自举验证（解压 →
  528:0:0 + CLI 一致）
- web 同步：kerf-data（ROADMAP Stage 1 批次 E 行——r14 E1-α 自举
  Expander + 528 基线；PACKAGE_CONTENTS 528/r14）+ site-footer 版本
  链 + download/README.md r14 节 + 动态面（stats 528 + r14 包）
- agent-browser E2E：页面加载零错误零 console 异常；r14 新内容渲染
  核对（footer 版本 528/批次 E）；Playground 金路径复跑
- r14 树压实（§8.6.5）：11_r14 条目（33-a/33-b + 溯源指针）+ 层 l
  行更新 + 根 l 未压实区间清零 + by-topic 增「自举 Expander E1-α」行
- lint exit 0（web 工程）

Stage Summary:
- r14 交付闭环：E1-α 自举 Expander（528:0:0）→ 打包（包内自举
  验证 + rec 树）→ web 同步 → E2E → 树压实（冷启动即见 11_r14）
- 遵循：§3.2（六命令逐条实测）、§19（打包 + 包内验证 + README
  归档）、§8.6.5（压实规则第三次实操）、§8.4.5（matrix 528 分项）
- 下一步（批次 E 续）：E1-β 宏收口（syntax-rules + HygieneCtx
  α 重命名 + 深度计数 + Span 代次 + 生产路径切换 + 基础宏定义）
  + TD-021 hof 用户面注入（随模块系统）→ Stage 1 门审查

---
Task ID: 34-a
Agent: Super Z (main) — ARCH-A/PM-A 定调 + DEV-A 实施（L3 多角色会话，批次 E 续）
Task: MUV 34-a：E1-β 宏收口——expander.krf 完整宏系统（define-syntax/syntax-rules 全模式面 + 卫生 α + 深度 500 + Span 并集代次守卫）+ parity 测试 17 + 行为 4

Work Log:
- 设计（33-b 登记 E1-β 范围的镜像式落地）：变换器注册表 = 名字 str
  单表（('rules 字面量 子句 def作用域) | ('builtin)——HashMap 替换
  语义：用户宏覆盖糖名 + 惰性糖注册 + 卫生基名回退对两类同权）；深度
  = 链长语义（expand-form 入口快照/出口回滚——兄弟不累计，镜像
  expand_form）；卫生上下文每宏应用重置（镜像 HygieneCtx::new 于
  apply_named）、renames 每子句实例化复位、计数器跨子句保持
- 实施（expander.krf +~470 行）：syntax-rules 解析（五类错误消息
  逐字）+ match-datum/match-sequence（通配/字面量同名/省略号尾部
  （零/多段固定前缀 + 复合单层）/字面量 datum 值相等/嵌套列表/向量
  trial 语义）+ instantiate-template（模式变量原样/省略号拼接跳格/
  悬垂省略号走实例化/α 重命名 num->str 递归构造 + 保留集 + 关键字
  豁免）+ transformer-step（深度计数 → 应用 → retag def∪use →
  expand-form 再入）
- **实测驱动的两轮重大发现**（§2.3-11——全部经 parity 双实现实跑暴露）：
  ①Span 并集代次守卫：oracle 对糖产物内宏调用的产物 Span 落模板
  范围（用户跨距按 Span::merge 的 expansion_id 不等守卫丢弃）——
  节点形状升 v2 = (tag s e exp scopes ...)（桥发射/读回）；retag
  代次 +1（bumped_expansion）；全部糖构造器代次来源逐位镜像（构造
  节点 = span 来源节点代次；dummy 关键字 = 0）②foldl/for-each 序章
  缺定义（chars->str→foldl 为 r14 死代码——卫生基名回退首次激活，
  R1 反射补定义）③宏产物品类 3 处代次漏挂（set/require/define-
  syntax——fields[3] 落 str/nil 的实跑暴露）
- 测试：bootstrap_expander_tests 19→36（宏 parity 17：注册/nil、
  卫生重命名计数器单调、省略号零一多、字面量+多子句、嵌套/向量模式、
  糖覆盖、复合省略号、module 体内注册、非列表模式子句解析合法、
  负例 6 组消息+Span 逐字）+ 行为 4（swap 运行期/递归 or 短路/卫生
  无捕获/深度结构化错误）；移除 E1-α 边界测试
- 文档：bootstrap-expander.md E1-β 增补节 + expander.krf 文件头
  E1-α→E1-β 边界改述

Stage Summary:
- E1-α 影子路径 → E1-β 宏收口：宏系统全模式面在 VM 上运行且与
  Rust 种子逐字 parity（528 → 545:0:0——r14 基线零回归 + 17 宏
  parity）；「语言能表达自身前端（含宏）」的自举命题在 kerf 侧
  完整成立（生产切换属 34-b）
- 遵循：§9.4（设计-测试锚定——每个语义判别点一组 parity）、
  §2.3-11（先实测禁臆测——代次守卫/foldl/三处漏挂全部实跑发现）、
  §12（最优>最小——节点形状 v2 全量重构而非局部补丁）
- 下一步：34-b 生产切换（compile_front 展开段 → bootstrap_expander
  + 切换守护 + 零回归验证）

---
Task ID: 34-b
Agent: Super Z (main) — ARCH-A 裁定 + DEV-A 实施（L3 多角色会话）
Task: MUV 34-b：E1-β 生产切换——compile_front 展开段经 bootstrap_expander（读+展开两段全自举）+ 切换守护测试 + exp 相位标记保持

Work Log:
- 切换实施：ExpanderKind 枚举（Bootstrap | Seed）——compile_front
  （生产）→ Bootstrap；compile_front_seed（bootstrap 加载 + parity
  oracle）→ Seed；front_from_forms 参数化（无递归：种子编译自举
  实现）；核心节点携带代次（(tag s e exp ...) → 桥重建
  Span{expansion_id}——「展开相位标记」E3 诊断特性经切换保持）
- 发现项（切换后实测）：e3_macro_error_carries_expansion_mark/
  message_shape_call_site_trace 两测试红——相位标记丢失（桥 core
  读取丢弃代次）→ 修复：core 节点发射/读回代次（krf 全部 core
  构造器 +x；桥 fields[3]）→ 双测试复绿
- 切换守护（driver 单元 production_expander_is_bootstrap）：独立
  线程（thread_local 零残留——thread::spawn + 缓存 thread_local
  确定性）编译前 is_loaded()=false → 编译后 = true（活性探针
  pub(crate) + #[cfg(test)]）+ 宏产物 Begin 代次 ≥1 双信号——
  §2.3-11 实测判别非推断
- bootstrap_expander.rs 头注释 E1-α 影子 → E1-β 生产切换改述
  （三件套角色：展开逻辑/桥/种子双角色）+ is_loaded 探针

Stage Summary:
- 生产管线读+展开两段全自举（VM 上 reader.krf + expander.krf 含
  宏）——「语言能表达自身前端」完整生产命题兑现；553 全绿 = 528
  基线经生产切换零回归（+切换守护 1 单元）
- 遵循：§2.3-11（守护 = 实测活性判别）、§7.2 Q3（端到端覆盖——
  全套件即生产管线回归）、§8.4.5（E3 相位标记特性不可静默丢弃——
  发现即修复）
- 下一步：34-c TD-021 prelude 模块注入（import 解析 + 多模块
  declare + 前置合并）

---
Task ID: 34-c
Agent: Super Z (main) — ARCH-A 设计 + DEV-A 实施（L3 多角色会话）
Task: MUV 34-c：TD-021 hofs 用户面注入——kerf-prelude 模块（preamble.krf）+ import 解析 forms 级合并 + 多模块按序 declare + prelude_tests 7

Work Log:
- 设计（登记册三否决的规避路径落地）：(module 名 (import
  kerf-prelude) ...) 声明 → 前段 read 后、expand 前 forms 前置合并
  （种子 Reader 读入固定库工件——与 expander.krf 同口径；独立
  file_id → Span 指向 preamble.krf，无 P1 源码拼接诊断污染；单一
  编译单元 → 无 P3 跨程序合并；零 VM 再入 → 无 P5 越界）
- 实施：bootstrap/preamble.krf（map/filter/foldl/for-each +
  (import)/(export) 头部）+ driver imports_prelude（Stx 层 module
  头游走——镜像 expand_module 头部序）+ resolve_prelude_imports
  （include_str 嵌入）+ 多模块 declare（全部 module 形式按出现序）
  + 主模块 = 最后 module 形式 visit（import 边传递依赖）+ 无 module
  程序 main 兜底声明（保持单模块时代行为）
- 实测修复：①preamble 头部 () 不可用（module 头部区仅 (import)/
  (export) 关键字形式——空列表按体处理报错）②多模块 declare 初版
  破坏无模块程序（main 未声明 → 「未声明的模块」——reader.krf 加载
  链即触发）→ main 兜底 declare 修复
- 测试：prelude_tests 7（用户面 map/组合管道 50/for-each 副作用/
  双路径一致/opt-in 未绑定负例/名字捕获显式失败/未知导入）+ runner
  r15 分组注册

Stage Summary:
- TD-021 P3 清偿闭环：hofs 用户面注入经模块/import 承载——显式
  opt-in + 显式失败（同名 define → 「重复定义」）+ 双路径一致；
  553:0:0（+7）
- 遵循：§12（正确>妥协——不做半吊子注入）、§11（接口隔离——注入
  留在前段管线层，expand/compile 零感知）、§2.3-4（显式失败）
- 下一步：34-d Stage 1 门审查（§7.3 审计 + §21.3 四条 + 发现项修复）

---
Task ID: 34-d
Agent: Super Z (main) — QA-A/ARCH-A（L3 多角色会话，门审查轮）
Task: MUV 34-d：Stage 1 门审查——stage1_gate_audit_r1（50 case）+ §7.3.1/§7.3.2 配比机械校验 + §21.3 四条件锚定 + 发现项修复 2

Work Log:
- 审计集（examples/audit/stage1_gate_audit_r1.rs，50 case 可重运行，
  cargo run --example）：A 单语句 12 / B 多语句 12 / C 复杂 10 /
  D 恢复 6 / E 边界 6（§7.3.2 本批次修复面：Span 代次守卫糖内宏/
  require·set 代次/prelude/模块内宏注册/卫生交换行为） / P 正向 4
  （§21.3 锚定：fib 生产管线/prelude 管道/递归宏短路/缓存确定性 +
  第二次 check 命中观测）；七类全覆盖（⑥ 循环依赖经双模块源码 →
  DFS 灰标记结构化 Err；⑦ 宏深度自指宏经生产路径）；极性 34 负向
  ≥22 / 恢复 6 / 正向 10；配比 main() 机械校验（违规 → exit 1）
- 审计发现项 2（修复）：模块注册簿错误（declare 重复声明 / visit
  未声明模块 / 循环依赖）此前 Span::dummy + 无诊断渲染 → 挂 module
  形式 Span + render_diagnostic（B07/C01 case 驱动——P2 诊断质量
  等级，非 soundness）
- 消息校准 8 处（审计预期 ↔ 真实消息对齐：需要 int/需要 bool/
  参数数量不匹配 + C03 eval 深度上限先于未绑达用例调整 deep 100→10
  + D06 计数器期望 (4 2) 算术修正 + P03 my-or 全 bool 语料）
- 复跑：50/50 PASS + stage0_gate_audit_r1 41/41 保持 EXIT 0
- §21.3 四条件锚定输出（审计集 main 尾部四行）：条件 1 生产管线
  自举含宏（全体 case 经生产路径）/ 条件 2 VM 承载全部生产展开 /
  条件 3 prelude 管道 + stdlib 套件 / 条件 4 缓存确定性 + 命中
  观测 + 静态检查 0 错

Stage Summary:
- Stage 1 门审查 APPROVED：50/50 + 配比满足 + 七类全覆盖 + 边界 ≥5
  + 零新 P0/P1（发现项 2 为 P2 诊断质量——当场修复复验）；§6.3
  投票记录：五角色（PM/ARCH/DEV/QA/REC）全票 GO（依据：553:0:0
  §3.2 全绿 + 双审计 APPROVED + §21.3 四条件锚定 + TD-002~022
  状态对账——开放项均 P3 且不阻塞 Stage 2 启动条件）
- 遵循：§7.3.1/§7.3.2（强制配比 + 边界 case）、§7.1.1（七类矩阵）、
  §2.3-11（审计 = 实测非清单自查）、§8.4.5（发现项修复附条款）
- 下一步：34-e 交付环（§3.2 + r15 tar.gz + web + E2E + 树压实）

---
Task ID: 34-e
Agent: Super Z (main) — QA-A/REC-A（L3 多角色会话，交付轮；含 r15 树压实）
Task: MUV 34-e：§3.2 全绿 + r15 tar.gz 打包（包内自举验证）+ web 同步 + E2E + r15 树压实

Work Log:
- §3.2 六命令实跑全绿（clean 起步）：clean（498 files/123.6MiB）→
  build --release 9.39s 零告警（clippy allow 一处：resolve_prelude_
  imports result_large_err——同 front 入口约定）→ check 0/0 → fmt
  --check 零 diff（prelude imports 一处格式化）→ clippy
  --all-targets -D warnings 0 → test --release --workspace
  **553:0:0**（单元 184 + 集成 369）
- 双审计集 EXIT 0（stage0 41 + stage1 50——§7.3.1 规则 4 不降规模）；
  CLI 冒烟：fib 75025 + ⇒ 144 + kerf test 2/2 + 宏+prelude 自检
  ⇒ 10（(m (foldl + 0 (map ... (filter ...)))) 经生产管线）
- r15 tar.gz 打包（§19.4 命令整目录；根 worklog.md 排除 ✓；
  docs/worklog/ rec 树 + docs/worklog.md 镜像入包 ✓）+ 包内自举
  验证（解压 → 553:0:0 + CLI 一致）
- 文档同步（§8）：tech-debt-register TD-021 → 已解决（r15）+
  matrix 553 对账（r15 增量行 + 历史链 + 集成 345→369）+ stage-1/
  plan.md 批次 E 行收口（E1-β + 生产切换 + TD-021 + 门审查
  APPROVED）+ RELEASE_NOTES r15 节（五交付）+ 03-macro-system
  E1-β 能力边界行 + 07-bootstrap Expander 交付标记 + docs/tests/
  v0/stage1/plan/{prelude,gate-audit-r15}.md 测试计划 + bootstrap-
  expander.md E1-β 增补
- web 同步 + agent-browser E2E + r15 树压实（见本条目后续追加）

Stage Summary:
- r15 交付闭环：E1-β 宏收口（全模式面 + 代次守卫镜像）→ 生产切换
  （读+展开两段全自举 + 实测活性守护）→ TD-021（模块/import 承载
  prelude）→ Stage 1 门审查 APPROVED（50 case + 四条件锚定）→
  打包（包内自举验证）→ web → E2E → 树压实
- 遵循：§3.2（六命令逐条实测——clean 全量重编）、§19（打包 + 包内
  验证 + README 归档）、§7.3.3（收敛裁定依据——本轮 0 新 P0/P1）、
  §8.4.5（matrix 分项对账）
- 下一步（Stage 2 准备）：§21 阶段规划先行（切换信号核对 + 后端
  策略裁定——LLVM 永不入自举链边界重申）+ TD-007 残留/TD-022 TCO
  决策点

---
Task ID: 35-a
Agent: Super Z (main) — QA-A/REC-A（L3 多角色会话，跨会话恢复交付轮）
Task: MUV 35-a：r15 交付包完整性复验（归档对账 + 包内自举验证 + CLI 冒烟）

Work Log:
- 跨会话恢复（PHASE 4 上下文耗尽协议）：新会话第一动作读 worklog——34-e 尾部「web 同步 +
  E2E + 树压实见后续追加」未兑现（会话截断）；对账结论：kerf 侧 553/r15 完成 vs
  web 侧 kerf-data.ts 停在 528/r14 = 落后一轮，即本轮缺口
- tar 归档完整性：244 条目解压成功（851,252 B）+ docs/worklog/ rec 树 +
  docs/worklog.md 镜像均在包内
- 包内自举验证（§19 复验，实跑）：解压 → cargo test --release --workspace
  **553:0:0**（全二进制汇总 553 passed / 0 failed / 0 errors / 0 warnings）
  + CLI 冒烟 fib 25 → 75025 与交付环境一致
- download/README.md r15 节五要点确认（34-e 已写）——本轮零补丁

Stage Summary:
- r15 包完整性实证闭环：244 条目 + 553:0:0 包内复现 + CLI 一致；34-e 打包侧
  工作全部落地（唯一遗留 = web 同步，由 35-b 兑现）
- 遵循：§19（包内验证）、§2.3-11（实测非清单自查）、PHASE 4（上下文耗尽
  恢复协议）

---
Task ID: 35-b
Agent: Super Z (main) — DEV-A/REC-A（L3 多角色会话，web 同步轮）
Task: MUV 35-b：web 官网 r15 全量同步（kerf-data 七处 + footer 三处 + lint + E2E 双端）

Work Log:
- kerf-data.ts（528/r14 → 553/r15）：ROADMAP Stage 0 累计行（553 基线 +
  双审计 91 case + 宏 parity 17 / prelude 7 / 守护 1）/ Stage 1 状态翻转
  「进行中」→「已完成（批次 A-E 全交付 · 门审查 APPROVED）」+ r15 两行 /
  Stage 2「未来」→「规划中（§21 阶段规划）」+ 规划行 / PACKAGE_CONTENTS
  553 项 + 15 条目 r1-r15 / HERO_FEATURES +2 徽章（前端全自举 + 门审查
  APPROVED）
- site-footer.tsx：状态行（门审查 APPROVED + 553 + 正负比 1:3.15 权威口径）/
  特性行（E1-β 宏收口 + 生产切换 r15）/ 底部版本行 v0.2.0→v0.3.0（Stage 1
  收官）
- lint exit 0；agent-browser E2E：页面零错误零 console 异常；内容验证 553 /
  门审查 APPROVED / 前端全自举 / r15 全命中；stats API 实时 553 + r15 包名
  （831.3 KB）；hero 统计卡 553；Playground 金路径 fib → 6765（真实编译器
  执行）；footer 滚底双形态（bottom = viewportH = 800）+ 桌面 1280 / 移动
  390 零横向溢出；移动截图存档 tool-results/r15-web-mobile.png
- 修正记录（实测发现）：初测 footer bottom 9011 误报——滚动动画时序误差，
  复测 bottom=800=viewportH 正常；Playground「运行」find text 误命中说明
  文字 → role button 精确定位通过

Stage Summary:
- web 侧 r15 同步闭环：34-e 截断缺口全部兑现；静态（kerf-data）+ 动态
  （stats/docs/playground API）+ 组件（footer 版本行）三层次一致
- 遵循：§8.4.5（文档随代码——web 是 kerf 对账面）、浏览器验证标准（渲染/
  数据/交互/响应式/footer 双形态）

---
Task ID: 35-c
Agent: Super Z (main) — PM-A/ARCH-A/PL-A（L3 多角色会话，阶段规划轮）
Task: MUV 35-c：Stage 2 阶段规划（§21 阶段规划 + §21.5 切换信号逐项核对 + §13.1 设计对齐 + §17/§18 + 批次 F-I 拆分）

Work Log:
- 落位 docs/develop/v0/stage-2/plan.md（v0.4.0-plan——镜像 stage-0/1 六节
  惯例 + §0 规划轮定位声明）；依据 §1.2「进入新阶段」路由：§21 → §13.1
  → §17/§4 → §7.3/§14.6
- §21.5 切换信号九条逐项实测核对：**7✅ + 1⚠️ + 1❌**——核心发现：信号 4
  （Stage 1 性能基线未单独建立）+ 信号 9（§14.6 阶段间深度验证未执行——
  连带 §14.5 D1-D8 / §14.8 B1-B4 / §14.9 C1-C6 阶段末环；Stage 0 对照物 =
  六篇深审产物 + final-assessment，Stage 1 仅有 §7.3 门审 + §6.3 投票）
- 裁定：**批次 F（Stage 1 深审收尾环，Task 36-a~e）= Stage 2 主体批次前置**
  （§21.5「全部满足」语义 + §1.3 L3 次序 Commit→Deep→Writeback→CodeClean
  →CrossStage→GO）
- 后端策略裁定：QBE 首选（08 §2：10% 代码 ≈ 70% 性能）+ C 转译并行评估 +
  Cranelift 备选；LLVM 永不入自举链（§21.2 约束四条重申）
- §13.1 十文档映射（08/13/01/06/02/07/09/05/12-roadmap/stage0 §6.12.6）
  → 对齐结论无缺口阻塞（FFI 所有权模型 = 唯一行为语义空缺 → 批次 G3 先行
  定义，§21.3 阻塞项缓解）
- §17 七步排版图：批次 F（深审收尾 36-x）→ G（G3 FFI 所有权 → G1 QBE
  PoC ∥ G2 HM 推断设计，38-x）→ H（H1 8 原语五项评估 ∥ H2 TCO + TD-007
  Rc 化 ∥ H3 Effect 语言级，40-x）→ I（I1 两次自举一致 → I2 stdlib 完整化
  → I3 门审查，42-x）；§18 四项依赖审查全过（QBE 二进制按 §3.1 工具链
  链位不阻塞批次 F）
- stage-1/plan.md 补 r15 后注记（六批次关闭 + 批次 F 前置衔接）；web
  Stage 2 行同步（35-b 内完成）

Stage Summary:
- Stage 2 规划闭环：切换信号诚实核对（深审缺口显式登记非静默带过——
  §2.3-4）+ 批次 F-I 四批次概排（批次 F 六字段全齐 + Task ID 唯一性核验
  36-x 未冲突）；规划轮不触发 §19 打包（无 kerf 代码变更）
- 遵循：§1.2（进入新阶段路由全章）、§21/§13.1/§17.2/§18.1/§4.1、
  §14.1-14.6（缺口识别 = 本计划核心发现）、§8.4.5（决策附条款号）
- 下一步：批次 F 执行（36-a §14.5 D1-D8 深审报告起步——输入 = r15 全量
  基线 + stage-2/plan.md）

---
Task ID: 36-a
Agent: Super Z (main) — ARCH-A/QA-A/REV-A/PM-A（L3 多角色会话，批次 F 深审轮）
Task: MUV 36-a：§14.5 D1-D8 深审报告（含 §6.3 批次 F 批准补票 + §3.2 基线复验 + §14.8 偏差清单）

Work Log:
- §6.3 批次 F 批准（补 stage-2/plan「规划批准中」悬空项）：五角色投票
  ARCH/DEV/QA/ALG-C GO + SKL-A GO（不加权）——加权 5.5/5.5 = 100% ≥ 95%
  （依据 §6.3 准入规则 + plan DAG 无环 + 基线全绿）
- §3.2 基线 clean 全量复验：build --release 9.86s 零告警 → check 0/0 →
  fmt 0 diff → clippy -D warnings 0 → test --release --workspace **553:0:0**
  （25s）+ CLI 冒烟 fib ⇒ 144（依据 §3.2 六命令逐条实测 + GATE 1）
- 数据采集：crate 依赖矩阵（DAG 无环 + driver 唯一组合根）/ §14.7.2 五项
  合规 grep 全 PASS / 逐 TD 代码核验（发现 TD-012 已完成未标记、TD-013
  绑定已关闭批次 E、TD-009/010/014/017/018 目标过期、TD-019/020 断档）/
  TD-002 符号家族亲验（Value 十变体/HeapObj 七变体八入口/ExpandCtxt 四字段
  /CLI 11 子命令）/ 性能实测（fib 89.4-89.6ms +0.8% 噪声带 / trivial
  0.015ms 自举前段≈免费 / 冷进程 16-20ms / **gc_stress 207-235ms vs
  Stage 0 基线 160.4ms = +29~46% 超回归阈值 + 超线性缩放实证**）
- 偏差扫描子代理 36-a-facts（Explore）：lang-design 20 篇对照 → 17 项候选
  偏差（13 新 + 2 known-B1 + 2 注释级）+ 12 组无偏差确认；关键断言主代理
  逐条亲验五项全中（依据 §2.3-11 实测非清单自查 + §1.6 委派分工）
- 深审报告落位 docs/develop/v0/stage-1/deep-review-round1.md（D1-D8 三段式
  + 委员会投票 GO-WITH-CONDITIONS 5.5/5.5 + §14.8 B1-B4 偏差清单 17 项 +
  无偏差确认 12 组 + 行动计划 = 36-b/c/d/e）
- 核心发现：P0/P1 = 0；P2×2（TD-023 gc_stress 回归新登记绑定批次 I2 /
  登记册目标时机过期家族）；无系统性 B3 实现违规——全部偏差为「实现前进
  文档滞后」方向

Stage Summary:
- Stage 1 深审闭环：553 全绿复验 + 八维度结论 + 偏差 17 项定量清单 +
  自举切换零性能代价实证（fib +0.8%）+ gc_stress 回归捕获（+29~46% 超线性）
- 遵循：§14.5.1（D1-D8 + 三段式 + 投票）、§14.5.3（P2 处置裁定）、
  §6.2 规则 2（登记册大阶段末全审）、§14.6.4（10% 回归阈值）、§3.2、
  §8.4.5（偏差清单即对账产物）、GATE 1（交付前实测全绿）
- 下一步：36-b 文档回写与登记册收口（B1-B5 行动项）

---
Task ID: 36-b
Agent: Super Z (main) — REC-A/ARCH-A（L3 多角色会话，批次 F 回写轮）
Task: MUV 36-b：§14.8 B1-B4 设计回写（lang-design v6.1→v6.2 十篇 + 登记册 v0.3.0-r16 全量收口 + 计划文档增补）

Work Log:
- lang-design 十篇回写（偏差清单 17 项落位，逐项附 B 类与依据 §14.8.1/§14.8.3）：
  05（HeapObj 七变体/八入口 alloc_symbol/I/O 三函数 write_stdout/TD-023
  回归注记）→ 06（值域十变体 Symbol 按名相等 +「语义字段一致」限定）→ 01
  （literal_value + Symbol 变体 + 作用域元数据注记）→ 09（TD-021 hofs r15
  解决注记（kerf-prelude opt-in import）+ 推迟项收口）→ 11（476→553 r15
  口径）→ 07（reader.krf 471 行 + 全模式面限定词）→ 03（ExpandCtxt 契约块
  四字段 next_scope + 全模式面限定）→ 12（§2.5.1 两行矩阵按交付实况改写：
  元循环求值器保持 Rust 参考/宏系统 E1-β 收口 ✅）→ 10（CLI 11 子命令表面
  清单——B4 灰区收口）→ 00（v6.2 修订记录块五组）
- tech-debt-register v0.3.0-r16：索引表补全 23 行（发现并修复 r3 后新增
  TD-015+ 无索引行的结构性缺口）+ TD-012 标记 resolved（批次 B 落地六文件
  实证）+ TD-013/009/010/014/017/018 六条目标时机改判 Stage 2（附 Stage 1
  门放行裁定注记——P2/P3 非阻塞口径）+ TD-007 残留改判 H2 + TD-019/020
  断档登记（全库 grep 零命中——TD-001/006 先例处置：禁复用，后续从 TD-024
  起编）+ TD-023 新增（P2 gc_stress 回归，绑定批次 I2）+ TD-015~022 标题
  层级 ## → ### 归一
- stage-1/plan.md 批次 F 执行注记（36-a~e 实录位）+ stage-2/plan.md 三处
  批次行增补（G2 绑定 TD-013 / I2 绑定 TD-023 / H2 绑定 TD-017——r16 改判
  全部落到 Stage 2 节点）
- 事故与修复（如实记录）：stage-2/plan.md 曾被占位脚本 'w' 模式误清空——
  立即从会话完整读取内容恢复（170 行 + 三处增补行 + Status 更新「批次 F
  已 §6.3 批准」）；恢复后 grep 核验 TD-013/023/017 绑定 7 处命中
- git 状态核查：r13-r15 变更 53 文件未提交（34-e 会话截断遗留——§19.3
  前置项缺口，36-e 统一补 commit per §6.4 规范）

Stage Summary:
- §14.8 回写闭环：偏差清单 17 项全落位（B1-known 2 项维持登记 + B2×8 +
  B3×3 + B4×4）；lang-design 全局 v6.1→v6.2；登记册索引/状态/目标时机三
  列对齐代码实态（大阶段末全审 §6.2 规则 2 兑现）
- 遵循：§14.8.3（回写单向「实现→设计」+ 内容最小化）、§14.8.1（B1-B4
  分类）、§6.2.1（登记册更新规则——索引补全/断档处置/只追加不删除）、
  §8.4.5（文档随代码）、R4（代码为准修文档）
- 下一步：36-c §14.9 C1-C6 代码整理（零语义变化 + 553 等价复跑）

---
Task ID: 36-c
Agent: Super Z (main) — DEV-A/REV-A（L3 多角色会话，批次 F 整理轮）
Task: MUV 36-c：§14.9 C1-C6 系统性代码整理（零语义变化 + 553 等价复跑）

Work Log:
- C1 代码清洁：build 0 warnings + clippy -D warnings 0（基线实测）+ TODO/
  FIXME/HACK/XXX = 0 + 死代码零项；catch-all 全量核查——「1 处无注释」
  为正则误报（vm.rs:779 前置行注释在位）；真 `_ => {}` 通配臂全库仅 1 处
  且有臂级理由，其余 14 处空臂均为显式变体臂（语义自明 no-op，如
  BcConst::Nil 在 hash、TcType::Unknown 无约束）——非 §14.6.1.1 违规
- C2 同类聚块：glob re-export = 0（§10.1 规则 4 合规）；9 crate 职责
  单一 + driver 唯一组合根；最大文件 vm.rs 1526 < 1600 失控线
- C3 注释时效（三处修正，实测发现）：driver.rs:263 切换守护注释方向
  反写（「代次 = 0——种子恒 ≥1」→ 依守护测试 944-959 与桥侧 195-196
  实况改为「双信号：is_loaded 前后翻转 + 产物代次 ≥1（retag +1，输入
  恒 0）」）；kerf-runtime lib.rs「四类根」→「五来源根」（05 v5.2 文档
  修订未及注释——罕见方向：代码注释落后文档）；同文件「Stage 0 堆语义
  GC 仅管 Pair」→ 堆语义（装箱叶节点六类含 r5 Symbol——注释过时一版）
- C4 文件头：66 个 .rs 全有 //! 头（逐一核验零缺失）
- C5 命名：pub fn/struct/enum 抽查合规（arity/len/new/intern/finish 为
  Rust 惯用名，§10 允许）
- C6 数据结构与流：§11 隔离五项 grep 全 PASS（D1 已实测）+ pub 可见性
  合理（driver 公共面 = 组合根导出 11 函数 + 4 模块组）
- GATE 1 实测：fmt 0 diff + clippy 0 + **test --release --workspace
  553:0:0（零断言修改逐一等价）**——零语义变化约束达成

Stage Summary:
- C1-C6 六维度全过；代码侧仅三处注释级修正（与深审 D4 偏差 #4/#10 对应）
  ——零语义变化，553 逐一等价实证
- 遵循：§14.9.1（六维度）、§14.9.3（完成标准 1-7——整理报告即本条目）、
  §14.6.1.1 规则 2（catch-all 注释核查——区分通配臂与显式空臂）、
  GATE 1（实测非清单自查）
- 下一步：36-d §14.6 四项审查 + 性能基线（自举 vs 种子探针实测）

---
Task ID: 36-d
Agent: Super Z (main) — ARCH-A/QA-A/REV-A（L3 多角色会话，批次 F 四审 + 基线轮）
Task: MUV 36-d：§14.6 四项强制审查 + 重构最优性 + 性能基线（自举 vs 种子实测）+ final-assessment + pipeline-test-coverage 重写

Work Log:
- **自举 vs 种子前段开销实测**（§21.5 信号 4 补录）：临时探针单元测试（kerf-driver
  内访问 pub(crate) 双路径 compile_front / compile_front_seed）→ 四语料 warm 20 次
  均值 + 冷启动实测 → **比值 69×（fib）/ 157×（宏）/ 116×（gc）/ 388×（300-defines）+
  冷启动 16.1ms**；采集后探针移除（方法与源码记入基线附录 A——30 行可重建复测）；
  553:0:0 移除后复验
- 七篇产出落位 stage-1/：architecture-review（32✅/3⚠️/0❌——Stage 0 ⚠️×4 收敛追踪）
  / design-impl-test-coverage（三列对照 13 文档组 + B1 全登记零未登记缺口 + 三者
  不一致零项）/ hidden-problems-assessment（复杂度四档：指数 0 / 5× 0 / 2× 2（均
  节点绑定 + 豁免依据）/ 不变 7——**§14.6.1.4 强制修复触发 0 项**）/ 
  refactoring-optimality-review（7 项重构 7 优 0 hack + 2 项方案否决记录 = §12
  执行证据）/ performance-baseline（9 指标 + 新口径三组 + TD-023 追踪协议）/
  final-assessment（GO + 门审 checklist 增补两项——登记册核对 + 对账面六面清单）
- pipeline-test-coverage.md 全量重写（r3 的 294 口径 → 553）：Tier 2 补 Stage 1 套件
  11 行（stdlib/bootstrap_reader/worklist/typecheck/cache/capability/testrunner/
  reserved_ext/scope_set/bootstrap_expander 36/prelude 7）+ Tier 3 双审计 91 +
  **parity 印证节**（64 测试逐字节双源锁定——设计流 = 管道流）+ 完整性小节 r16 实测
  （catch-all 42→1 通配 + 显式空臂 14 + 生产 expect 4 + //! 66/66）+ 性能基线同步节
- 核心裁定：388× 前段比值 → **stage-2/plan DAG 的 H2→I1 次序被实测验证为正确**
  （非仅拓扑偏好——H2 TCO 是 I1 编译器本体迁移的实质前置）；TD-023 按正确性豁免
  §14.6.1.4 强制修复（L-GC 不可观测性不受影响）
- 发现并登记：matrix 表体两行 r15 滞后（bootstrap_expander 19→36 未回写 + prelude_tests
  行缺）——36-e 对账收口

Stage Summary:
- §14.6 全协议闭环：四审 + 最优性 + 基线 + 深挖 3 路记录 + final-assessment GO——
  信号 9（阶段间深度验证）与信号 4（性能基线）双双兑现
- 遵循：§14.6.1.1-14.6.6（四审 + 输出集合核对）、§14.6.4（性能基线 + 10% 阈值 +
  回归协议）、§14.6.3（≥3 轮独立深挖——实做 3 路）、§14.5.1 完成标准 8 条对照
- 下一步：36-e 收尾交付（matrix 对账 + roadmap 注记 + RELEASE_NOTES r16 + tar.gz
  包内验证 + web 同步 + E2E + git commit）

---
Task ID: 36-e
Agent: Super Z (main) — QA-A/REC-A（L3 多角色会话，批次 F 交付轮）
Task: MUV 36-e：收尾交付（matrix/roadmap/RELEASE_NOTES 对账 + r16 tar.gz 包内自举验证 + git commit + web 同步 + E2E）

Work Log:
- 对账面六面清单执行（final-assessment 固化协议首次实操）：lang-design 涉及篇
  （v6.2 十篇——36-b）✓ / matrix（r16 增量行 + 表体两行尾差修复：bootstrap_
  expander_tests 19→36 + prelude_tests 行补录）✓ / tech-debt-register（v0.3.0-r16）
  ✓ / pipeline-test-coverage（全量重写 553 口径）✓ / RELEASE_NOTES（r16 节五交付）✓
  / web（本轮同步）✓
- v0.5-roadmap Stage 1 行 ✅（深审收尾环闭环注记）+ stage-1/plan 批次 F 注记 +
  stage-2/plan TD 绑定行（G2←TD-013 / I2←TD-023 / H2←TD-017）
- r16 tar.gz 打包（§19.4 整目录命令）：**254 条目 / 908,667 B**（根 worklog.md
  排除 ✓；docs/worklog/ rec 树 13 条目 + docs/worklog.md 镜像入包 ✓）
- **包内自举验证**（§19 复验，实跑）：解压 → cargo build --release 9.30s 零告警
  → cargo test --release --workspace **553:0:0** → CLI 冒烟 fib ⇒ 144 与交付
  环境一致（总 34s）
- git commit（§6.4 规范——r13-r16 积欠统一入账，34-e 会话截断遗留的 53 文件
  未提交变更本轮收口）
- web 同步（kerf-data + footer + docs 浏览器 + stats 实时）+ agent-browser E2E
  双端——见 36-f 终验条目
- 终态重打包：36-e/36-f 条目写入 flat 后重跑 §19.4 命令（包含完整 worklog）
  + 复验

Stage Summary:
- r16 交付闭环：深审 → 回写 → 整理 → 四审 → 基线 → final GO → 对账 → 打包
  （包内 553:0:0）→ commit → web——批次 F（Stage 1 深审收尾环）全五 MUV 交付，
  §21.5 信号 4/9 兑现，Stage 2 启动条件全绿
- 遵循：§19（打包 + 包内验证 + 命名约定）、§6.4（Git Commit 规范——关联
  plan 引用 + 遗留债务清单 + 轮次记录 + 投票结果脚注）、§8.4.5（文档随代码——
  六面清单）、GATE 1（交付前实测全绿——本条目所有数字均为实跑）
- 下一步：Stage 2 批次 G（G3 FFI 所有权模型先行——QBE 安装按 §3.1 链位）

---
Task ID: 36-f
Agent: Super Z (main) — QA-A/REC-A（L3 多角色会话，批次 F 终验轮）
Task: MUV 36-f：批次 F 终验（lint + E2E 汇总 + 树压实核对 + 终态包复验）

Work Log:
- lint exit 0（web 侧 eslint——kerf-data 七处 + footer 四处改动后）
- agent-browser E2E 双端全过：渲染（零错误零 console 异常）/ 数据（r16 五关键词
  全命中 + stats API 553 + r16 包名 888.2 KB + docs API v6.2 命中）/ 交互
  （Playground fib → 6765）/ 响应式（1280 + 390 零横向溢出）/ footer 双形态
  （滚底 bottom=viewportH，diff=0）；移动截图存档 tool-results/r16-web-mobile.png
- dev.log 运行期错误检查：零错误
- 树压实核对：docs/worklog/ rec 树 13 条目（01 层 r4-r16）+ 根 l stale 区间
  清零（35-x 吸收进 01/13）+ 根 l 4,430B / 01 层 l 2,824B（≤8KB 上限 ✓）
- git commit 123e13d（r13-r16 统一入账——63 files / 7532 insertions；§6.4
  脚注四要素齐：plan 引用 + 债务清单（真实等级确认）+ 轮次记录 + 双投票结果）
- 终态包复验（36-e 承诺兑现）：36-f 条目入 flat 后重跑 §19.4——包含完整
  worklog（36-x 五条目）的 r16 tar.gz 复验 553:0:0 + CLI 一致

Stage Summary:
- 批次 F（Stage 1 深审收尾环）全六 MUV 交付闭环：§14.5 深审（P0/P1=0 +
  偏差 17 项）→ §14.8 回写（lang-design v6.2 + 登记册 23 行）→ §14.9 整理
  （553 等价）→ §14.6 阶段间验证（六篇 + 69~388× 实测 + final GO）→ §19
  打包（包内自举 553:0:0）→ web 同步（三层一致 + E2E 全过）→ git 入账 →
  终验
- 遵循：GATE 1-5 全程（交付前实测全绿/八项退出条件/条款号引用/外循环
  通过后收口/审计集不降规模）；PHASE 4 长会话压缩自检三轮（36-b/36-d/36-f
  节点——角色正确/worklog 追加镜像/技术债零未记录/复杂度无需再升/以文档
  为准）
- 下一步：Stage 2 批次 G 启动（G3 FFI 所有权模型先行——QBE 二进制按
  §3.1 链位安装于 G1 前落位；新会话第一动作 = 读 worklog + 树 l 恢复状态）

---
Task ID: 38-a
Agent: 子代理 ARCH-A/ALG-A（联合——主代理并行分派）
Task: MUV 38-a（G3）：FFI 所有权模型定义（§21.3 阻塞项先行裁定——13 §3.3.4 冻结契约的行为补全）

Work Log:
- 前置阅读按序：sop §2.3/§8.4.5、13 §3.3.4、05 §3/§4、06 §1.2/§3/§4、
  ffi.rs P1 冻结注记、gc.rs 根集契约、stage-2/plan §1/§5
- 三原语责任矩阵 3/3：CallExternal 借用窗口 + CInt/CPointer/Opaque
  归属三分法；AllocExternal 外部 malloc 域线性所有权；FreeExternal
  消费释放
- pin/unpin 形式化：Φ 计数簿扩展 σ=(H,R,Φ)、P1/P2/U1/U2 归约规则、
  引理 F-PIN、mark_all 起点快照并入（主循环零改动）、L-GC 保持
- 线性令牌主裁定：可重复借用传递 + 一次性消费（消费全局生效）；
  失效 E9 诊断而非 UB；防复制性留类型级线性性 Stage 3 锚
- ExternalPointer 状态机 + 非法迁移 7 条（E9/E10/E11 诊断码族）
- 边界 case 13 个全判定；实现路线锚定 G1 PoC（整数路径不触及 pin）
  与批次 I（write_stdout 借用路径做实）

Stage Summary:
- §21.3 阻塞项「FFI 所有权模型未定义」解除：stage-2/ffi-ownership-
  model.md 交付（216 行；三原语矩阵 3/3、迁移 7/7、case 13/13、
  决策 19/19 条款化——新裁定 12 项显式标注）
- 遵循：§8.4.5（决策附条款号）、13 §3.3.4（契约零改动——只裁行为）、
  §2.3-1/2（最优与显式失败口径）
- 回写义务 4 处登记（批次 I 落地轮执行）；Stage 3 锚点 6 项

---
Task ID: 38-d
Agent: 子代理 ALG-A/ARCH-A（联合——主代理并行分派）
Task: MUV 38-d（G2）：HM 推断设计轮（R1-R8 保守 → HM 升级评估 + 16 参照复核）

Work Log:
- 通读 typecheck.rs 669 行 + BUILTIN_SIGS 49 项 + driver check_source
  ——R1-R8 逐条行号锚盘点（8 规则 + 8 架构事实 A1-A8 + 保守性契约
  「误报 = P1」）
- 复核 16-reference-analysis v5.0：HM 覆盖仅 §2.2 一行——无系统推断
  章节；对照表六语言公开事实自足 + 诚实登记缺口
- 识别 P0 语义冲突：letrec nil 预绑定（Nil~(α→β) 合一失败）+ 顶层
  递归自引用 Unknown 回退（typecheck.rs L548-562 实锚）
- 11 项核心裁定（D1-D11）：约束三段式（否决 W/J）+ 值限制 OCaml 式
  + set! join + letrec fresh 预置双形状特判 + occurs check + TcType
  →HM 十一构造子 + 双点泛化 + E0005/Span 诊断集成 + 三阶段演进轨道
- 冲突清单 7 项 + 风险 P0×3/P1×3/P2×3/P3×1（全附缓解）

Stage Summary:
- stage-2/hm-inference-design.md 交付（303 行；裁定 11 项 ≥7 要求、
  参照表 6 行 + 复核结论、行号锚实况引用零猜测）
- **裁定：GO（有条件）**——约束三段式算法；PoC 排批次 H 新增 MUV
  （H4），默认期早于 I1 自举迁移
- 遵循：§2.3-4（缺口显式不静默）、§8.4.5、§21.5（批次逐批细化）

---
Task ID: 38-b
Agent: Super Z (main) — SKL-A/DEV-A（L3 多角色会话，批次 G 工具链 + IR 实化轮）
Task: MUV 38-b：QBE 工具链 §3.1 链落位 + AnnotatedANF 实化（kerf-backend 新 crate + 契约迁移）

Work Log:
- §3.1 查找链实操：which qbe 无 → scripts/ 无 qbe → tools/ 无 →
  docs/tools/ 无 → 安装：c9x.me releases 页探测（首页/compile 目录
  枚举）→ qbe-1.3.tar.xz（281,332B）下载 → make -j4 零告警（670,544B
  amd64_sysv + arm64/rv64 六目标）→ 落位 tools/qbe/bin + 源码归档 +
  scripts/qbe/setup.sh（重建路径）+ docs/tools/qbe/setup.md（记录）
- 三段冒烟：fib 递归 IL → qbe -o → cc → 运行 fib(20) exit 109 =
  6765 mod 256（数值正确——exit 8 位截断口径；完整值验证走
  fib(12)=144）——**语法勘误两条实测**：函数签名必须带返回类型
  （function l $fib(...) 而非 function $fib）；整数比较带宽度后缀
  （csltl 非 cslt）——勘误入 setup.md（诚实记录）
- kerf-backend 新 crate（第 10 成员——§11 后端层独立）：codegen.rs
  契约**迁移**（CodegenBackend/WasmBackend/AnnotatedANF 自
  kerf-driver/reserved 迁入——签名零变化原则 27；AnnotatedANF 占位
  →实化：funcs 函数定义集 + fingerprint 保留；reserved/codegen 改薄
  re-export + 兼容锁存测试）+ anf.rs（块式 ANF：ABlock{phis,stmts,
  ctrl} + APhi 块首值合并 + LowerCtxt 两遍扫描 + arity 静态校验 +
  卫生基名渲染 + fingerprint 内容寻址）
- workspace 9→10 成员 + 根 crate 依赖 + driver 依赖（组合根组装）

Stage Summary:
- QBE 1.3 工具链闭环（安装记录 + 重建脚本 + 冒烟实录）；契约迁移
  原则 27 兑现（旧路径 reserved_ext_tests 零改动继续过——迁移兼容
  实证）
- 遵循：§3.1（查找链全程 + 安装记录归档）、§2.2 原则 27/32（签名
  兼容 + 预留本质是兼容性）、§11（后端层独立 crate）
- 下一步：38-c QbeBackend + IL 生成 + AOT 端到端

---
Task ID: 38-c
Agent: Super Z (main) — DEV-A/QA-A（L3 多角色会话，QBE 后端 PoC 实现轮）
Task: MUV 38-c：QbeBackend 做实 + QBE IL 生成 + AOT 编排 + fib 端到端本地码（§21.3 条件 3）

Work Log:
- qbe.rs IL 生成（全 l 64 位与 Int(i64) 对齐；比较产 w；phi 块首
  渲染；QbeBackend 三契约方法——目标门 + IL 信封 + QBE 内建 pass
  四项声明）+ aot.rs 编排（qbe 子进程 → .s → cc → 可执行 → 运行；
  查找链 KERF_QBE → 安装布局 → 编译期锚）
- CLI +2 子命令：anf（IR 摘要）/ native（AOT 全链 + exit code 口径）
  ——11→13 子命令（10-toolchain v6.3 回写）
- **实现勘误实录（GATE 1 诚实记录，全部实测发现）**：① jmp 带参/
  块参数语法 QBE 1.3 不存在——值合并改 phi 指令（%r =l phi @l1 %v1,
  @l2 %v2，块首约束）；② Br 的 else/merge 目标 seal 时预测在 then
  分支内嵌 if 分裂多块时错位——**跳转回填机制**（§19.3 不变式 2 同型：
  占位 u32::MAX + 分支落定后回填三处；phi 来源 = 实际跳转块索引）；
  ③ not 翻译 ceqw 非 ceql（w 操作数）；④ fingerprint 仅长度 hash 使
  (+ 1 2)/(+ 1 3) 同指纹——操作数全量入 hash；⑤ 测试并行竞态两处：
  KERF_QBE env 全局污染（find_qbe 重构纯函数注入式）+ 共享目录
  remove_dir_all 互删（原子计数器唯一目录）
- **端到端实测**：fib(12) ⇒ **本地码 exit 144** = VM 路径 ⇒ 144
  （双路径一致）；IL 结构断言全命中；值上下文 if phi 合并
  （(- (if (< 1 2) 10 20) 5) ⇒ 5）；嵌套 if（classify -25/44 ⇒
  111/144）；算术/begin/not·eq?/mod 等价采样
- 测试 +24 集成（端到端 6 + 结构 4 + 一致性 6 + 负例 10 + 契约 2）
  + 9 单元（backend crate）

Stage Summary:
- **§21.3 条件 3 兑现：首个非 VM 后端工作（fib 端到端本地码）**；
  中间基线 585:0:0 全绿（553 + 32）
- PoC 边界 B1 登记（TD-024）：整数域十二原语 + 直接调用；闭包/
  Float/Str/Pair/set!/module/IO/函数值一等显式边界外错误
- 遵循：§9.4.3（正负比 1:3+——正 16 负 10 外加结构/契约）、
  §21.3（条件 3 锚定）、§19.3 不变式 2 同型回填、GATE 1（勘误
  实录如实入档）
- 下一步：38-e TD-013 恢复实现

---
Task ID: 38-e
Agent: Super Z (main) — DEV-A/QA-A（L3 多角色会话，TD-013 清偿轮）
Task: MUV 38-e：TD-013 多错误收集与恢复展开实现（r7 设计验收 5 条全过——P2 清偿）

Work Log:
- 种子路径（kerf-expander/recover.rs）：DiagCollector（push/is_full
  (128)/mark_truncated/into_sorted——(file_id,start,end) 稳定排序）+
  expand_program_recover（形式级：错形式收集跳过继续；满即截断+终止）
- 自举桥路径（bootstrap_expander::expand_program_recover）：逐形式
  **单元素列表**调用（expander.krf 协议零改动——调用粒度桥侧切换；
  全局状态跨调用持续）；双路径同构测试（诊断数/产物数/错误消息族）
- driver 消费面：check_source_recover（front_from_core 抽段重构——
  R9 fail-closed + 相位簿记 + 字节码共享段单一实现 §12）——E0002+
  E0005 全量合并 + 位置序 + 截断尾注；**CLI check 切换恢复模式**
  （单错误短路保留库 API check_source；run/eval 不变——r7 §5）
- 实现勘误：截断标记 break 路径须显式置位（mark_truncated——38-e
  实测发现，push 分支够不到）；Python 脚本语法错回滚重放（勘误入
  档）
- **r7 验收 5 条全过**：①恢复跳过 + 产物含后续 define（指令数 =
  无错对照一致）②上限 128+截断提示 ③位置序 ④既有零破坏（短路
  API 不变锚）⑤16 case 全绿
- 测试 +16 集成（种子 6 + driver 8 + 同构 1 + 执行路径 1）+ 4 单元

Stage Summary:
- TD-013 **resolved**（登记册 v0.3.0-r17 更新 + 详情节交付注记）；
  605:0:0 全绿（553 + 52 净增）
- 遵循：r7 设计 §2/§3/§5（形式级/收集器契约/消费面表格逐条兑现）、
  §11（编译期控制流非 effect）、§12（front_from_core 单一实现）
- 下一步：38-f 收尾交付（§3.2 六命令 + 对账 + tar.gz + web + git）

---
Task ID: 38-f
Agent: Super Z (main) — QA-A/REC-A（L3 多角色会话，批次 G 收尾交付轮）
Task: MUV 38-f：收尾交付（§3.2 六命令 + 对账五面 + r17 tar.gz 包内自举验证 + web 同步 + E2E + git + 树压实）

Work Log:
- **§3.2 六命令实跑全绿**（clean 起步）：clean → build --release
  11.52s 零告警 → check 0/0 → fmt --check 零 diff → clippy
  --all-targets -D warnings 0 → **test --release --workspace
  605:0:0**（28s；集成 409 = 369+24 qbe+16 恢复，单元 196——逐二进制
  实测：span 11 + syntax 11 + core 10 + reader 23 + expander 32 +
  compiler 15 + runtime 9 + vm 18 + driver 58 + backend 9）
- 双审计集 EXIT 0（stage0 41 + stage1 50 case）；CLI 冒烟：VM run
  fib ⇒ 144 / **native fib exit 144**（双路径一致）/ test 2/2
- 对账面更新（六面清单）：RELEASE_NOTES **v0.4.0-r17**（Stage 2 minor
  bump——批次 G 五交付节）/ matrix v0.1.0-r17（605 总量 + r17 增量
  行 + 表体：backend 单元行 + expander 32 + 单元 196 标题）/ 登记册
  v0.3.0-r17（TD-013 **resolved** 详情注记 + TD-024 新增 B1 边界 +
  索引行）/ pipeline-test-coverage（Tier 1 196 / Tier 2 409 + Stage 2
  两套件行）/ v0.5-roadmap 批次 G 行 + stage-2/plan Status 与执行注记
  / lang-design 13 §3.3.7 迁移注记 + 10-toolchain v6.3（CLI 13 子命令）
- **r17 tar.gz 打包**（§19.4 命令扩展 +tools/ +scripts/——批次 G
  工具链入包）：282 条目 / 1,504,494 B（qbe 二进制 670KB + 源码
  281KB 入包）
- **包内自举验证（§19 复验，实跑）**：解压 → build --release 11.27s
  零告警 → **605:0:0** → CLI 四冒烟：native fib exit 144 / run ⇒ 144 /
  check ok / 恢复模式 E0002 合并报告 ✓
- web 同步（kerf-data + footer + docs 浏览器 + stats 实时）+
  agent-browser E2E——见 38-g 终验条目
- 终态重打包（36-e 先例）：38-f/38-g 条目入 flat 后重跑打包命令
  （含完整 worklog）+ 复验

Stage Summary:
- **批次 G（后端/FFI/类型三主线）全六 MUV 交付闭环**：G3 FFI 所有权
  模型（阻塞项解除）→ G1 QBE 后端 PoC（**§21.3 条件 3 兑现——fib
  本地码端到端 + VM 一致**）→ G2 HM 设计轮（GO 有条件——H4 新增
  MUV）→ TD-013 resolved（P2 清偿）→ 收尾（605:0:0 + 包内验证 +
  web 对账三层一致）
- 遵循：§3.2（六命令逐条实测——clean 全量重编）、§19（打包 + 包内
  验证 + 命名约定——命令扩展 tools/ 如实注记）、§8.4.5（六面对账——
  文档随代码 R4）、GATE 1-5（全程实测口径）
- 下一步：批次 H（H1 原语评估 ∥ H2 TCO+TD-007 ∥ H3 Effect 设计 ∥
  **H4 HM PoC（38-d 裁定新增）**）
