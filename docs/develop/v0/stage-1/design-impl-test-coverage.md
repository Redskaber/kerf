# Stage 1 设计-实现-测试三者覆盖审查（§14.6.1.3）

> **Author**: Super Z（QA-A 主导，ARCH-A 复核，REV-A 验证）
> **Date**: 2026-09-11
> **Version**: v0.3.0-ditc-r16
> **Status**: Active
> **基线**: lang-design v6.2（36-b 回写后）/ 553:0:0 / 双审计集 91 case / parity 36+28 测试
> **审查方法**: §14.6.1.3 四向核对（设计→实现 / 实现→测试 / 测试→设计 / 三者一致性）；数据源 = deep-review-round1.md §6 偏差清单（17 项）+ 无偏差确认清单（12 组）+ matrix.md 分套件表（36-a/36-b 实测）

---

## 1. 总体结论

**三者一致性判定：✅ 通过**。设计（lang-design v6.2）→ 实现（crates + 自举 kerf）→ 测试（553 + 91 审计 case）的映射无 **B1 级未登记缺口**（设计要求但实现未做且未入 TD 登记册的项 = 0）；已知 B1 全部有 TD 编号 + Stage 2 批次绑定（TD-005/011/013/022 等）；实现→测试方向发现 2 项低风险观察（性能回归无断言面——按 §14.6.4 设计即基线文档复测而非断言，非缺口；`kerf code` 子命令的 CodeValue 分析面测试锚点稀疏——Probe 冻结覆盖）。

## 2. 设计→实现→测试三列对照表（按 lang-design 文档）

| 设计文档（v6.2） | 设计点 | 实现状态 | 测试状态 |
|---|---|---|---|
| 01 核心原语 | 9 原语 + literal_value（含 Symbol r5）+ 作用域元数据（r13） | ✅ expr.rs（十变体字面量 + VarRef/SetBang 作用域集 + param_scopes） | ✅ 单元 10 + scope_set_tests 9 + 负例矩阵（E1-E6） |
| 01 §6 | require 声明形式（第 10 变体，零运行时语义） | ✅ expr.rs Require + PushNil 编译 | ✅ capability_tests 23 case |
| 02 语法模型 | TokenKind 12 / 叶级 45 / Keyword 22 / Stx 四字段 + 代次 / NFC 保守子集 | ✅ token.rs / symbol.rs / stx.rs | ✅ reader 单元 23 + negative_reader 59 case |
| 03 宏系统 | define-syntax/syntax-rules 全模式面（单层边界）/ 卫生 α / 深度 500 / trampoline / ModuleRegistry DFS | ✅ macro_sys.rs + expander.krf 镜像（E1-β） | ✅ expander 单元 28 + negative_expander 96 case + bootstrap_expander_tests 36（parity 含宏 17） |
| 04 字节码 VM | 40 操作码 8 组 / 帧协议 / MAX_FRAMES 10^5 / Define 编译序 / App 函数先 / 调试帧三扩展位 | ✅ opcode.rs + vm.rs | ✅ compiler 单元 15（40 操作码守护）+ vm 单元 18 + vm_tests 20 + negative_vm 230 case |
| 05 运行时 | HeapObj 七变体 / 八入口 / 五来源根 / 冷却四参数 / 三通道 I/O | ✅ heap.rs / gc.rs / io.rs | ✅ runtime 单元 9 + gc_tests 6（10^6 有界） |
| 06 操作语义 | R1-R9 归约 / E1-E8 错误族 / 值域十变体 / T1 双路径一致 | ✅ eval.rs + vm.rs 双路径 | ✅ negative_semantics 98 case（T1 回归 + 双路径互查）+ vm_tests 双路径 |
| 07 自举策略 | 两段自举（读 r6 + 展开 r15）/ 种子双角色（引导 + oracle）/ 切换守护 | ✅ bootstrap.rs + bootstrap_expander.rs + 3 krf 载荷 | ✅ bootstrap_reader_tests 28 + bootstrap_expander_tests 36（**逐字节 parity**）+ production 切换守护 1（活性探针 + 代次双信号） |
| 09 标准库 | 52 内置（48 用户面 + 4 自举原语）/ I/O 六项能力门控 / hofs prelude（r15 TD-021） | ✅ builtins.rs（52 defs）+ preamble.krf | ✅ stdlib_tests + prelude_tests 7 + capability 门控负例 |
| 10 工具链 | CLI 11 子命令 / 四层调试（1、2 层交付） | ✅ main.rs | ✅ CLI 冒烟（交付轮四示例 + fib）+ dump 层经 driver 转发测试 |
| 11 测试 | 双路径互查 / 用例运行器 / parity | ✅ tests/ 树 + runner.rs + kerf test | ✅ test_runner_tests 18（自举形态前身） |
| 12 路线图 | §2.5.1 矩阵（v6.2 改写后）Stage 1 列逐行 | ✅ 见偏差 #5/#14 已回写 | ✅ matrix.md 553 对账 + r13-r15 增量行 |
| 13 能力矩阵 | 预留 14 项签名冻结 / 能力 I/O 做实（r8）/ 内部效应（r8） | ✅ reserved/ 三文件 + capability.rs + effects.rs | ✅ reserved_ext_tests 12（P0 位置断言 + 可达性 + 负向形状）+ Probe 冻结 8 |
| 08/14-19 | 策略/分析/原则/术语层（无实现状态断言） | —（对照无偏差确认） | — |

## 3. 三向缺口清单

### 3.1 设计要求但未实现（B1——§14.8.1 口径）

| # | 设计点 | 实现状态 | TD 登记 | Stage 2 节点 |
|---|---|---|---|---|
| 1 | syntax-parse 级宏组合（点对尾部/尾省略号/展开转义/嵌套省略号） | 未实现（03 §2.3 边界声明 + 36-b 限定词） | TD-005（P3） | 批次 G2/H 评估 |
| 2 | 字符串全序比较 | 未实现（cmp 仅 `=`） | TD-011（P3） | 批次 I2 批量清偿 |
| 3 | 多错误恢复展开（expander 侧） | 未实现（typecheck 侧 r7 已在） | TD-013（P2，r16 改判） | 批次 G2 |
| 4 | quote 向量字面量 | 未实现（符号 r5 已交付） | TD-002 向量部分 | Stage 2 |
| 5 | Effect 语言级 | 未实现（编译器内部 r8 已做实） | 13 §3.3 做实窗口 | 批次 H3 |
| 6 | FFI / 多目标后端 / LSP 等 14 项预留 | 预留冻结（设计即如此） | 13 §3.3-§3.5 | 批次 G（G3/G1） |

**结论**：B1 项 6 组全部有登记 + 节点绑定——**无未登记的设计缺口**。

### 3.2 已实现但未测试（B2 方向的风险面）

| # | 实现点 | 测试覆盖 | 判定 |
|---|---|---|---|
| 1 | gc_stress 性能回归（+29~46%） | 性能数字不在断言面 | **非缺口**——§14.6.4 设计即「基线文档复测」而非测试断言（§9 性能回归检测协议）；TD-023 登记 + I2 节点 |
| 2 | `kerf code` 子命令（CodeValue 检查消费面） | 无直接集成测试 | 低风险——CodeValue 本体经 kerf-core 单元 + fast_path 守护测试覆盖；CLI 壳层同 bench（冒烟口径） |
| 3 | 冷启动 16.1ms 自举装载 | 外部口径（示例 4 项 16-20ms） | 覆盖（性能基线 §5/§6） |

**结论**：无「已实现但零测试」的功能面；性能面按协议走基线复测。

### 3.3 设计要求但未测试（场景级缺口）

- 深审 D3 已核：七类负向矩阵 7/7 + E 码直接断言矩阵（Stage 0 深审 P1 教训全数兑现）+ 正负比 1:3.15。
- **遗留观察 1 项**：pipeline-test-coverage.md 的 Tier 表未含 Stage 1 新增 7 套件（文档载体滞后——36-d 同批重写该文件收口，非测试缺口）。

### 3.4 三者不一致清单

- **零项**（36-a 偏差扫描：无系统性 B3「实现违反设计」；全部偏差为文档滞后方向，36-b 已回写收口——lang-design v6.2 后三列一致）。

## 4. 一致性核验方法记录（供复用）

1. 设计→实现：20 篇 lang-design 逐文档对照（子代理 36-a-facts + 主代理亲验五项抽查）
2. 实现→测试：matrix.md 分套件表（553 = 单元 184 + 集成 369）× crates 模块映射
3. 测试→设计：负例矩阵（七类 + E 码族）× 06 错误族表
4. 三者一致：parity 36/28（设计 = 实现的双实现互证）+ 双审计集 91 case（走生产管线）

*遵循：§14.6.1.3（三者覆盖四向核对）、§9.4（锚定原则）、§14.8.1（B1-B4 口径）、§2.3-11（实测取证）。*
