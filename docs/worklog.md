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
