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
