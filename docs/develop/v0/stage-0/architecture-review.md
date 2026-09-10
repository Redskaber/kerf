# Stage 0 架构设计审查（§14.6.1.2）

> **Author**: Super Z（ARCH-A 主导，QA-A 复核）
> **Date**: 2026-09-10
> **Version**: v0.1.0-arch-r1
> **Status**: Active
> **基线**: 290 测试全绿（release，294 函数 = 290 通过 + 4 文档化忽略）/ 40 操作码 8 组 / 24 内置函数 / lang-design v5.2 回写后 / Task 16 修复内循环完成（App 函数先、循环依赖检测、堆栈追踪、eval 卫生回退）
> **审查方法**: 按编译管道 7 阶段逐阶段 × 5 维度（完整性/设计对齐/结构清晰性/效率/扩展性）打分（✅ 优秀 / ⚠️ 需改进 / ❌ 有问题），全部结论附 file:line 或实测命令证据；引用 deep-review-round1.md D1/D5/D6 结论

---

## 1. 总体结论

**架构判定：✅ 通过（含 4 项 ⚠️ 观察项，无 ❌）**。9 crate 依赖 DAG 无环、零外部依赖、单向数据流无回流；7 阶段 35 个维度评分中 31 ✅ / 4 ⚠️ / 0 ❌。4 项 ⚠️ 均有明确处置路径（TD-012 拆分候选 / J6 LOC 观察 / 冻结契约扩展点固有代价 / IR 旁路计算观察），不阻塞阶段切换。

### 1.1 架构级实证（全管线横切）

| 项 | 结论 | 证据 |
|---|---|---|
| 依赖 DAG 无环 | ✅ | `cargo tree --workspace` 实测拓扑序 `{span, runtime} → syntax → {core, reader} → {expander, compiler} → vm → driver → 根`；成员边 24 条 + 根 9 条 + 1 条 dev-dep（expander→reader，仅 `#[cfg(test)]`） |
| 零外部依赖 | ✅ | 全部 10 个 Cargo.toml `[dependencies]` 区仅含 kerf-* 成员（rg 实测无第三方条目）——自举信任根显式化 |
| 显式 re-export、零 glob | ✅ | 9 crate lib.rs 实测 28 条 `pub use` 语句（r1 深审口径 29 条，差异为语句 vs 条目计数口径，本质一致：全部显式列举，`rg 'pub use .*\*'` 零匹配） |
| 模块文档头 | ✅ | 37 个生产 .rs 文件 37/37 以 `//!` 开头（实测 `head -1` 全量核对） |
| §11 合规（§14.7.2 清单） | ✅ | B1/B2/B3/B5/B6 零匹配；B4（driver 唯一 reader 调用者）经 Task 16 A4 修复后全 PASS——根 CLI `main.rs:14` 仅 import `kerf_driver`（+`kerf_core::CodeValue` 用于 `code` 分析子命令，非 reader 调用，合规） |
| 测试基线 | ✅ | `cargo test --release --workspace`：290 passed / 0 failed / 4 ignored；正负比 1:3.3（§9.4.3 达标） |

### 1.2 LOC 分布（实测，2026-09-10）

| crate | LOC | 主要文件 | 备注 |
|---|---|---|---|
| kerf-span | 565 | diagnostic.rs 221 | 错误结构化层 |
| kerf-syntax | 677 | symbol.rs 293 | Interner 预内部化 |
| kerf-core | 1091 | expr.rs 394 / ir.rs 387 | 9 原语 + 图 IR |
| kerf-runtime | 575 | heap.rs 436 | slot-Vec 堆 |
| kerf-reader | 1082 | lexer.rs 581 | 单遍词法 |
| kerf-expander | 2335 | **expander.rs 1351** | ⚠️ TD-012 拆分候选（r1 审 2276，Task 16 +59） |
| kerf-compiler | 1206 | compile.rs 649 | 40 操作码冻结 |
| kerf-vm | 1746 | **vm.rs 1223** | run_program/execute 拆分（r1 审 1713，Task 16 +33 = 堆栈追踪） |
| kerf-driver | 1232 | driver.rs 506 | 编排层（r1 审 1099，Task 16 +133 = dump 转发/eval 卫生回退） |
| 根（src/main.rs） | 297 | CLI 10 子命令 | 全部经 driver 转发 |
| **合计** | **10806** | | 与 worklog Task 16 记录一致 |

---

## 2. 逐阶段架构评分（7 阶段 × 5 维度）

### 2.1 reader（kerf-reader：lexer.rs 581 / parser.rs 317 / token.rs 158）

| 维度 | 评分 | 依据（一句） |
|---|---|---|
| 完整性 | ✅ | Stage 0 词法/语法全量交付：字符串跨行、贪心数字拒绝、嵌套块注释、省略号、Unicode、`'x` quote 简写、括号三态错误带 Span——全部 6 个示例与 41 case 审计集经此入口零缺口 |
| 设计对齐 | ✅ | 02-syntax-model v5.2 回写后 Token 叶级 44 计数与 NFC 保守子集边界注记一致（Task 16 C 对账项，deep-review 偏差 #17/#18 已闭环） |
| 结构清晰性 | ✅ | token/lexer/parser 三文件单一职责（词法状态机/递归下降/类型定义各一），依赖仅 span+syntax，无环 |
| 效率 | ✅ | 单遍词法 + 递归下降一次遍历；标识符经 intern 转为 u32 句柄后续 O(1)；无已知 O(n²) 热点 |
| 扩展性 | ⚠️ | 新 token 类型需改 3 处（token.rs TokenKind 枚举 + lexer.rs 规则 + parser.rs 分支）——词法器固有耦合，可接受但应入扩展指南 |

### 2.2 expander（kerf-expander：expander.rs 1351 / macro_sys.rs 715 / phase.rs 236）

| 维度 | 评分 | 依据（一句） |
|---|---|---|
| 完整性 | ✅ | 9 核心形式 + §3.2 全部糖推导（let/let*/letrec/cond/and/or/when/unless/while）+ syntax-rules（模式/字面量/省略号/模板）+ 相位分离三操作 + 循环依赖检测（phase.rs:83 DFS 灰标记 → 结构化 Err + 菱形依赖合法幂等） |
| 设计对齐 | ✅ | 03-macro-system v5.2 一致：ModuleRegistry 七方法/ModuleEntry 五字段/MAX_EXPANSION_DEPTH=128/HygieneCtx 全契约（deep-review CP3 无偏差确认清单）；instantiate Stage 0 单模块裁定与 syntax-rules 单层省略号边界已注记（偏差 #1/#9 闭环） |
| 结构清晰性 | ⚠️ | crate 内三模块职责清晰，但 expander.rs 单文件 1351 行已贴近 §13.4 J6 mod < 1500 上界——登记为 TD-012 拆分候选，推迟 §13.2 切换期执行 |
| 效率 | ⚠️ | 展开为递归实现（TD-007 迭代化计划 Stage 1 偿还），深度上限 128 显式防护；无已知 O(n²) 算法 |
| 扩展性 | ✅ | 新糖经 Transformer/TransformerKind 注册（macro_sys.rs 扩展点）或核心形式 dispatch 单点新增；parse_params 重名检查是展开期单点防御（两执行路径共同上游） |

### 2.3 core/lower（kerf-core：expr.rs 394 / ir.rs 387 / code_value.rs 287）

| 维度 | 评分 | 依据（一句） |
|---|---|---|
| 完整性 | ✅ | CoreExpr 9 正交原语（span 全节点）+ 图 IR（Arena + node_map 字面量共享 + find_shared）+ CodeValue（良构性绑定感知 + α 重命名 + 组合预留）——12 能力模型的"结构化中间层"全部落地 |
| 设计对齐 | ✅ | 9 原语 ADT 逐字段与 01-core-forms 一致（deep-review CP1 无偏差确认）；01 的 import/export Stage 0 裁定已回写 |
| 结构清晰性 | ✅ | expr/ir/code_value 三文件单一职责（树原语/图 IR/语义检查），依赖仅 span+syntax |
| 效率 | ✅ | node_map HashMap 结构键 O(1) 摊销共享查找（ir.rs:121-125），字面量去重无重复分配 |
| 扩展性 | ✅ | 新 AST 节点 = CoreExpr 枚举单点 + ir.rs lower_expr 单臂 + 下游消费者由编译器穷尽 match 强制同步（Rust 免 default 警告） |

### 2.4 compiler（kerf-compiler：compile.rs 649 / bytecode.rs 269 / opcode.rs 256）

| 维度 | 评分 | 依据（一句） |
|---|---|---|
| 完整性 | ✅ | 40 操作码 8 组冻结（opcode.rs:18 枚举 + :239 守护测试断言 40）、跳转回填（占位队列 + 完备断言）、常量池去重（float 位键，compile.rs:65-69）、闭包捕获描述符（bytecode.rs:59-83 CaptureSource）、逐指令 debug_spans（bytecode.rs:87） |
| 设计对齐 | ✅ | 04-bytecode-vm v5.2 全 40 项显式枚举回写，enum↔守护测试↔文档三方冻结一致（Task 16 A2 修复了 r1 的 39/40/41 三方漂移）；CLOSURE 协议与 ext1→EffectSystem 预留链文档化 |
| 结构清晰性 | ✅ | opcode/bytecode/compile 三文件单一职责（指令集/容器与反汇编/发射逻辑） |
| 效率 | ✅ | 常量池 HashMap 去重 O(1)/常量；跳转回填单遍；编译 fib(25) 全程含在 88ms/轮的 run_source 内（含 VM 执行），编译占比极小 |
| 扩展性 | ⚠️ | 新字节码指令需改 6 处：①opcode.rs:18 枚举 ②opcode.rs:109 name() ③opcode.rs:155 render_operands() ④compile.rs 发射点 ⑤vm.rs execute 臂 ⑥opcode.rs:239 冻结守护测试——第 ⑥ 处是 Stage 0 冻结契约的**设计意图**（三方同步防止漂移），非缺陷；5 处生产代码为指令集双层定义（编译器发射 + VM 消费）的固有代价 |

### 2.5 vm（kerf-vm：vm.rs 1223 / eval.rs 253 / value.rs 246）

| 维度 | 评分 | 依据（一句） |
|---|---|---|
| 完整性 | ✅ | 双执行路径（VM/eval）+ 迭代式主循环 + 帧三扩展槽（ext1/2/3 格式冻结）+ 共享单元格捕获协议（Rc<RefCell> 局部槽）+ 堆栈追踪（run_program/execute 拆分——vm.rs:143 外层在错误路径从 `frames` 快照附加最内 16 帧调用点，vm.rs:178 MAX_TRACE_FRAMES） |
| 设计对齐 | ✅ | 06-operational-semantics v5.2：App 求值顺序函数先双侧一致（compile.rs:258 fn 先求值 + vm.rs:412 CALL 弹参后弹 fn，与 eval 路径一致）；R1-R9/E0-E8 全集双侧对齐（DefineGlobal/set! 的 E3/E6 语义 Task 12 落地） |
| 结构清晰性 | ⚠️ | run_program/execute 拆分后错误路径帧快照单点内聚，但 vm.rs 1223 行为全项目最大单文件，接近 J6 上界（1500）——观察项，与 expander.rs 同列切换期重组候选 |
| 效率 | ✅ | 迭代式循环 Rust 栈恒定（帧数上限 100_000 显式防护）；GC 冷却退避消除根集轮询 O(n²)（历史 122s→3.4s 案例，worklog Task 4-c）；fib(25) 实测 88.8ms 均值（见 performance-baseline.md） |
| 扩展性 | ✅ | 新指令仅 vm.rs execute 增一臂；帧三扩展槽为 Stage 1+ 调试/Effect/多阶段预留（格式冻结、当前可空）；TraceFrame 结构化追踪可扩展 |

### 2.6 runtime（kerf-runtime：heap.rs 436 / gc.rs 52 / io.rs 57）

| 维度 | 评分 | 依据（一句） |
|---|---|---|
| 完整性 | ✅ | HeapObj 六类型装箱（Pair/Str/Int/Float/Bool/Nil）+ 标记-清除（显式工作栈 + free-list 重建 + 复位对称）+ 分配计数驱动触发 + foreign 根登记 + io 双函数（write_line_stdout/read_line_stdin） |
| 设计对齐 | ✅ | 05-runtime v5.2：HeapObj 六变体、根集五来源（含帧捕获槽第五来源）、GC 冷却数值冻结（256/64/25%/1024）、slot-Vec+显式工作栈算法描述对齐（deep-review 偏差 #10/#11/#12/#24/#25 全部闭环） |
| 结构清晰性 | ✅ | heap/gc/io 三文件单一职责（分配器/回收周期/I-O 通道），L0 零依赖（仅 std） |
| 效率 | ✅ | `slots: Vec<Slot>` + GcRef=u32 槽位索引 O(1) 访问（heap.rs:69/178）；标记阶段显式工作栈无递归栈风险；gc_stress 3×10^5 分配实测 160.4ms/轮 |
| 扩展性 | ✅ | 新堆对象类型 = HeapObj 枚举 + alloc 入口 + mark 臂 3 处（单 crate 内聚，无跨阶段扩散） |

### 2.7 driver（kerf-driver：driver.rs 506 / builtins.rs 404 / reserved.rs 288）

| 维度 | 评分 | 依据（一句） |
|---|---|---|
| 完整性 | ✅ | 全管线编排（read→expand→lower→compile→run/eval 五段）+ 相位生命周期接线（registry.visit/instantiate）+ 24 内置函数注册（builtins.rs 实测 defs.push 24 项：+ - * / mod = < > <= >= cons car cdr list not null? pair? int? bool? procedure? eq? str-append print read-line）+ 双路径卫生回退（driver.rs:273 VM 路径 resolve_hygiene_fallbacks / driver.rs:309 eval 路径 resolve_eval_hygiene_fallbacks 镜像）+ dump 转发（dump_tokens/dump_stx 公共 API，Task 16 A4） |
| 设计对齐 | ✅ | 09-stdlib v5.2 的 24 项清单对账（r1 偏差 #15 闭环）；13-capability-matrix §3.1 以 reserved.rs 冻结签名回填（三 trait + 能力 IO P2）；根 CLI 全子命令经 driver（§14.7.2 B4 全 PASS） |
| 结构清晰性 | ✅ | driver/builtins/reserved 三文件单一职责（编排/宿主库/冻结预留）——编排层不掺执行语义 |
| 效率 | ⚠️ | `compile_source`（driver.rs:208）在 run/eval 生产路径上无条件计算 IrGraph 后旁路丢弃（仅 CLI `ir` 子命令消费 `out.ir`）——O(n) 一次性成本、实测无感知，但属"生产路径旁路计算"观察项（详见 refactoring-optimality-review.md §3） |
| 扩展性 | ✅ | reserved.rs 四接口预留冻结（EffectFamily/Effect/EffectSystem + MultiStage + Read/WriteCapability/CapabilityIO + CacheKey/CachedResult/CompilationCache，Probe 测试证明冻结）——Stage 1 能力落位清晰；新内置函数 = builtins.rs defs.push 单点 |

---

## 3. 评分汇总与改进建议

### 3.1 汇总矩阵

| 阶段 | 完整性 | 设计对齐 | 结构清晰性 | 效率 | 扩展性 |
|---|---|---|---|---|---|
| reader | ✅ | ✅ | ✅ | ✅ | ⚠️ |
| expander | ✅ | ✅ | ⚠️ | ⚠️ | ✅ |
| core/lower | ✅ | ✅ | ✅ | ✅ | ✅ |
| compiler | ✅ | ✅ | ✅ | ✅ | ⚠️ |
| vm | ✅ | ✅ | ⚠️ | ✅ | ✅ |
| runtime | ✅ | ✅ | ✅ | ✅ | ✅ |
| driver | ✅ | ✅ | ✅ | ⚠️ | ✅ |
| **合计** | **7✅** | **7✅** | **5✅ 2⚠️** | **5✅ 2⚠️** | **5✅ 2⚠️** |

**设计对齐专项（§13.4 J1）**：7 阶段全部 ✅——lang-design v5.2 回写完成后，deep-review-round1 §6 的 10 大项无偏差确认清单（CP1 9 原语 ADT / CP2 操作码 40 契约 / CP3 宏契约 / CP4 runtime 契约 / CP5 R1-R9·E0-E8 / CP7 CapabilityIO / CP8 Span / CP9 单向无环 / CP10 十二里程碑）经 Task 16 对账闭环，26 项偏差中需回写的 24 项全部落笔（2 项为 Stage 1+ 推迟入册）。

### 3.2 改进建议（按优先级）

| # | 建议 | 类别 | 处置 |
|---|---|---|---|
| 1 | expander.rs（1351 行）拆分：按核心形式展开/糖推导/宏匹配三职责切分，留 re-export | TD-012 | §13.2 切换期执行（Stage 1 前端重写窗口，避免修复期扩大回归面） |
| 2 | vm.rs（1223 行）观察：J6 上界 1500 内，若 Stage 1 追加指令逼近上界则按 execute 臂组拆分 | 观察 | 纳入 Stage 1 plan 输入 |
| 3 | driver IR 旁路计算：run/eval 路径延迟化（仅 `ir`/`code` 子命令触发 lower_program）或文档化为"分析 IR"定位 | P3 | 登记 TD 候选（见 refactoring-optimality-review.md §4） |
| 4 | 新增操作码 6 处同步点写入 04-bytecode-vm 扩展指南（含守护测试强制同步说明），降低未来贡献者漏改风险 | 文档 | Stage 1 文档批次 |
| 5 | 新 token 3 处同步点（TokenKind/lexer/parser）写入 02-syntax-model 扩展注记 | 文档 | Stage 1 文档批次 |

### 3.3 与 §14.6.1 四项强制审查的关系

本文档为四项强制审查之二（§14.6.1.2）。架构结论支撑：①完整性审查（§14.6.1.1）的逐阶段 catch-all 基线（41 生产臂全量裁决，Task 16 C1）②隐藏问题评估（§14.6.1.4）的输入为本文 §3.2 的 ⚠️ 项——均为"进入下一阶段复杂度不增长或线性可控"级（TD-012 拆分在 Stage 1 前端重写时边际成本最低）。
