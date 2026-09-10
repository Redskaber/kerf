# 作用域集解析（TD-004）测试计划

> **Author**: Super Z（DEV-A/QA-A，Task 32-a）
> **Date**: 2026-09-11（r13，批次 E 首个 MUV）
> **依据**: lang-design 03-macro-system §4 实现状态注记（r13）；tech-debt-register TD-004（已解决 r13）；sop §9.4/§9.5

## 1. 测试目标

验证 TD-004 作用域集解析收口后的 Racket 式语义：编译器与 eval 双路径按
`(name, scopes ⊆)` 子集匹配 + max-cardinality 解析绑定；空作用域集全局绑定
⊆ 任意引用集（名称基兜底）；作用域不匹配 ≠ 名称匹配（判为未绑定）。

## 2. 覆盖场景

| # | 场景 | 类型 | 路径 |
|---|------|------|------|
| 1 | shadowing：内层形参遮蔽全局同名 | 正例 | VM + eval |
| 2 | 嵌套同名 shadowing（lambda (x) (lambda (x) x)） | 正例 | VM + eval |
| 3 | 闭包捕获（捕获绑定携带绑定作用域集） | 正例 | VM + eval |
| 4 | set! 命中词法绑定而非全局（全局值不变） | 正例 | VM + eval |
| 5 | 引用作用域集不含绑定集 → 未绑定 | 负例 | VM（未绑定的全局变量） |
| 6 | 同上 | 负例 | eval（未绑定变量） |
| 7 | set! 目标作用域不匹配 → 未绑定 | 负例 | VM + eval（set! 未绑定变量） |
| 8 | 宏引入绑定不捕获用户同名（卫生 + 作用域双保险） | 负例 | VM + eval |
| 9 | 子集匹配命中对照（同骨架 binder ⊆ ref → 命中形参） | 正例（对照） | VM + eval |

负例 5-7 经手工构造 CoreExpr（`binder ⊄ ref_scopes` 形态）——展开器注入
不变式保证真实源码不产生该形态，故手工构造是该语义的唯一可观测载体。

## 3. 测试统计

- 函数 9（正例 5 含双路径断言 + 负例 4）；断言点 17（值断言 + 错误消息断言）
- 注册：tests/runner.rs 批次 E 分组（`scope_set_tests` mod）
- 500 基线零回归（r13 全量 509:0:0 实测）

## 4. 依赖

- kerf-driver（run_source/eval_source 源码级双路径）
- kerf-compiler（compile_module）+ kerf-vm（run_program/eval_program/Env）
- kerf-core（CoreExpr 手工构造）+ kerf-syntax（ScopeSet/ScopeId/SymbolTable）
