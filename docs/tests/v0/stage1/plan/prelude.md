# prelude 模块注入（TD-021）测试计划

> **Author**: Super Z（DEV-A/QA-A，Task 34-c）
> **Date**: 2026-09-11（r15，批次 E / E1-β）
> **依据**: tech-debt-register TD-021（三否决后的模块承载裁定）；lang-design 09-stdlib（hofs 基线）；sop §9.4/§9.5

## 1. 测试目标

验证 TD-021 偿还：用户程序经 `(module 名 (import kerf-prelude) ...)`
声明导入后，map/filter/foldl/for-each 以普通全局函数可调用（模块系统
承载——forms 级合并注入单一编译单元，非源码拼接/跨程序合并/builtin
调闭包三否决路径）。

## 2. 覆盖场景

| # | 场景 | 类型 |
|---|------|------|
| 1 | import 后 map 用户面可调用（(2 4 6 8 10)） | 行为正例 |
| 2 | 组合管道 filter→map→foldl（= 50） | 行为正例 |
| 3 | for-each 副作用序（遍历求和 = 6） | 行为正例 |
| 4 | 双路径一致（run/eval 同果 = 10） | T1 正例 |
| 5 | 无 import 声明 → map 未绑定（opt-in 语义） | 负例 |
| 6 | 同名 define → 「重复定义」显式报错（不静默遮蔽） | 负例 |
| 7 | 未知 import → 「未声明的模块」（registry visit） | 负例 |

## 3. 测试统计

- 7 测试函数（tests/v0/stage1/plan/prelude_tests.rs，runner.rs 批次 E
  r15 分组注册）；正负比 4:3。

## 4. 依赖

- kerf-driver（front_from_forms 注入 + 多模块 declare + 主模块 visit）
- bootstrap/preamble.krf（kerf-prelude 模块——include_str 嵌入）

## 5. 设计锚点

- 注入位置：read 后、expand 前（forms 级——单一编译单元保证编译/
  执行语义与普通顶层形式完全一致）；
- Span 归属：preamble.krf 独立 file（P1 否决口径规避——无诊断污染）；
- 名字捕获：显式失败（重复定义变量——§2.3 原则 4）。
