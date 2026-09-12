# Stage 2 架构设计审查（§14.6.1.2——K2 阶段间深验证）

> **审查日期**：2026-09-15（r36 / K2 / 57-a）｜ **审查者**：ARCH-A 主导 + QA-A 验证
> **方法**：按编译管道逐阶段（reader → expander → core → compiler → backend → vm → runtime → driver）五项审查（完整性/设计对齐/结构清晰性/效率/扩展性）+ §11 合规机械验证

## 1. 逐阶段评分表

| 管道阶段 | crate | 完整性 | 设计对齐 | 结构清晰性 | 效率 | 扩展性 | 评分 |
|---------|-------|--------|---------|-----------|------|--------|------|
| Reader（词法+语法） | kerf-reader | TokenKind 12 变体 + 叶级 48（02 v6.1 对齐——本轮回写后零漂移） | 02-语法模型 v6.1 ✅ | 词法/语法分层 + E0001 结构化 | NFC 保守子集 O(1) 快路径 | 新 Token 类型 = enum 增量 + 穷尽性强制 | ✅ 优秀 |
| Expander（展开） | kerf-expander | 25 Keyword + 糖九件 + 宏 E1-β + require/perform/handle 臂 | 03 v6.3（本轮回写 10_000 口径）+ 01 §6 | core_forms/sugar/macro_sys/phase 分文件（TD-012 拆分先例） | 10_000 深链 0.02s（TD-007/TD-025 清偿） | 新核心形式 = 增臂 + parity 双侧 | ✅ 优秀 |
| Core（IR/值） | kerf-core | CoreExpr 12 变体冻结 + IrGraph + CodeValue | 01 §2/§8.3（迁移映射冻结维持——本轮处置注记） | Arena + 平行表 + 不可变变换 API | 字面量/变量结构键共享 | ADT 演进经 §13.2 流程（expr.rs 文档锚） | ✅ 优秀 |
| Compiler（类型+字节码） | kerf-compiler | typecheck（R1-R8 回归基线）+ hm（旗标期判定面 r29）+ compile 46 操作码发射 | 04 v5.2 + hm-inference v0.2.0 契约 | 三模块分文件（typecheck/hm/compile/opcode/bytecode） | 栈平衡/回填完备断言；常量池去重 | 新操作码 = 增臂（Op 穷尽性强制三面：VM 分派/name/编译发射） | ✅ 优秀 |
| Backend（本地码） | kerf-backend | QBE PoC（G1 r17）+ anf lowering（B1 边界显式拒绝 7 臂） | 04 §1 + TD-024 口径 | anf/aot/codegen/qbe 分文件 | fib native 144 端到端 | CodegenBackend trait 可插拔 | ✅ 优秀（PoC 口径如实） |
| VM（执行） | kerf-vm | 46 操作码主分派穷尽 + TCO + 效应三原型帧 + FFI 三指令 | 04 v5.2 + 05 v6.3（本轮回写六来源） | vm.rs 单文件 2266 行（内聚单一职责——拆分为观察项非必要项） | fib 92.5ms（TD-028 漂移登记）/ TCO 105k 帧恒定 | ext1 具体化先例（r25——预留槽升级路径实证） | ✅ 优秀 |
| Runtime（堆/IO） | kerf-runtime | 8 HeapObj 变体 + 9 分配入口（05 v6.3 本轮回写）+ GC 三阶段 + Φ 簿 | 05 v6.3 + ffi-ownership-model | heap/gc/io 分文件 | mark-sweep 显式工作栈 | ForeignBox tracer 协议（§11 隔离——运行时不依赖 VM 类型） | ✅ 优秀 |
| Driver（组合根） | kerf-driver | front 管线 + R9 能力门控 + 自举三桥（expander/compiler/种子链）+ effects（内部）+ ffi 窗口规程 + cache | 15 §5.1 组合器 + 13 §3.1.3 | bootstrap.rs/driver.rs/capability.rs/effects.rs/ffi.rs/reserved/ 分文件 | CompilerKind 分桶（种子不查不存） | 唯一知全层（15 §5.1——扩展经 reserved 冻结面） | ✅ 优秀 |

## 2. §11 合规验证清单（机械验证——57-s1 实测）

| 检查项 | 验证方法 | 结果 |
|--------|---------|------|
| compiler 不调用 reader | Cargo.toml 依赖 + grep kerf_reader（compiler/src） | **零匹配** ✅ |
| vm 不调 compiler 内部 | grep crate::kerf_compiler（vm/src） | 数据类型 only（BcConst/BcProgram/Op/CaptureSource——正当）+ cfg(test) 编译引用 ✅ |
| compiler 不调 driver | grep kerf_driver（compiler/src） | 代码零（2 处 doc 归属注记）✅ |
| driver 是唯一 reader 调用者 | 全仓 kerf_reader::read/use 扫描 | 生产调用 4 处全在 driver（driver.rs:20/bootstrap.rs:27/bootstrap_expander.rs:577/bootstrap_compiler.rs:886）✅ |
| 元数据预计算 | CompileResult 字段盘点 | debug_info_table 每指令 (pc, Span) ✅ |
| 无 glob exports | grep "pub use.*::\*" | **零匹配** ✅ |
| 错误路径覆盖 | has_errors()/Result 消费盘点 | E0001-E0012 全族显式 + main.rs exit code ✅ |

## 3. 数据流完整性（§14.7.3 五段校验）

source → read_source（tokens 非空 + interner 全 intern ✅ E0001 负例锚）→ expand（CoreExpr 无 unexpanded 残留 ✅ E0002 + parity 逐字节）→ compile（Bytecode + debug_info 全覆盖 ✅ 栈平衡断言）→ vm::run（Value 无 panic ✅ E0004-E0012 + 758 测试）→ runtime 输出（RunOutcome 显式 ✅ CLI 四路径）。

## 4. 结论与改进建议

- **总评**：八阶段全 ✅；§11 七项零违规；数据流五段全绿。架构经 Stage 2 全程（批次 G-K + 三轮插入）**零腐化**（每轮 §3.2 + 四审计集 + 架构审计测试 4 case 守护）。
- **改进建议**（非阻塞——Stage 3 观察项）：①vm.rs/anf.rs 大文件体量在批次 M（命名空间层将动 library 面）时一并评估拆分收益（§14.8.3 条款 5：切换期重构最佳时机）；②TD-028 性能漂移在优化窗口 profile 定锚。

*遵循：§14.6.1.2 / §11 / §2.4.5（层级依赖）/ 15-架构分层 v6.0。输入：57-s1 全量扫描（worklog 57-s1 条目）。*
