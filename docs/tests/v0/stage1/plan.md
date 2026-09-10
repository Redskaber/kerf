# Stage 1 测试计划（批次级）

> **Author**: kerf-dev-agent
> **Date**: 2026-09-10（r8）
> **Version**: v0.1.0-r8
> **Status**: Active

## 套件

| 套件 | 文件 | 覆盖 |
|------|------|------|
| expansion_worklist_tests（r4） | tests/v0/stage1/plan/expansion_worklist_tests.rs | TD-007 trampoline：200 层链端到端 / 双路径 / 自指宏报错 / 糖交错 |
| stdlib_tests（r5；r7 +TD-016 两函数；r8 I/O 门控前缀更新） | tests/v0/stage1/plan/stdlib_tests.rs | TD-002 符号值消费 + 标准库最小集 24 函数：正例 59 断言 + 负例 172 case（元数/类型/边界矩阵 + 类型全扫描 + Racket 语义边界注记）+ r7 链式比较全操作数前置校验（TD-016）+ r8 门控 I/O 负例 require 前缀（read-line 元数 FS-4 修复激活） |
| bootstrap_reader_tests（r6） | tests/v0/stage1/plan/bootstrap_reader_tests.rs | **B3 自举 Reader parity**：正例 87 case（Stx 树含 Span 递归等价、符号按名）+ 负例 307 case（错误消息+Span 逐字节等价、`assert_negative_parity` 防误收）+ 双错误次序（首错位置契约）+ Unicode/NFC + 深度边界 256/257 + Token 流/dump 格式 parity + 高阶函数直测（map/filter/foldl/for-each，builtin 作 f）+ Reader 原语（str->pos-chars/char-whitespace?/char-alphabetic?/str-int-valid? 含误用）+ 管线集成（run/eval 双路径 + 读错误渲染） |
| typecheck_tests（r7，批次 C） | tests/v0/stage1/plan/typecheck_tests.rs | **保守静态类型检查**：零误报锚（examples/ 6 程序 + 动态边界 12 case + 深嵌套 250 层）+ R1-R8 负例矩阵（if 条件 8 变体 / 算术 5 算子 / 比较 4 算子含 TD-016 静态面 / not / car-cdr / 不可调用 / lambda 与内置元数 / 字符串符号族）+ **静态确定性反向锚**（静态报错程序实跑必报 Run 错——双误报与双漏报）+ 多错误收集（全量/Span 次序/错误后继续）+ check_source 集成（编译错误透传/统计面/双路径不受影响） |
| cache_tests（r7，批次 C） | tests/v0/stage1/plan/cache_tests.rs | **编译缓存**：SHA-256 公共 API 向量 + 冻结 trait 行为规格 1/2/3（命中/幂等覆盖/单键失效）+ 键判别（源/文件名）+ 管线集成（同源二次命中等价 / eval 共享 run 条目 / 停用旁路 / 编译错误不缓存而运行错误前段照常缓存 / **命中产物 == 新编译产物（确定性证明）** / 全量失效）+ check 缓存观测面 |
| capability_tests（r8，批次 D） | tests/v0/stage1/plan/capability_tests.rs | **能力模型 I/O 基础传递**（13 §3.1.3 四条款）：require 声明面正向（可用/幂等/顺序无关/双路径 nil/CheckReport 观测）+ E0006 三路径一致门控（run/eval/check）+ 嵌套/宏卫生基名/粒度互斥 + 用户接管豁免零误报 + 形状负例 4 + 令牌不可伪造 + read-line EOF 子进程确定性探针 + examples 活体 |
| test_runner_tests（r8，批次 D） | tests/v0/stage1/plan/test_runner_tests.rs | **用例运行器 kerf test**：前置/用例切分（define/set!/module/require 为前置）+ PASS 判定（值非 #f）+ 短路（case 内首错即停）+ 恢复（失败后续跑）+ 深位失败零签名污染 + 状态隔离（前置逐用例重放）+ 报告面 + front 错误面（read/expand/E0006 整体 DriverError） |

详见各文件内文档注释与 [docs/tests/matrix.md](../../matrix.md) 分套件统计。
