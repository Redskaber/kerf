# 自举 Expander（E1-α）测试计划

> **Author**: Super Z（DEV-A/QA-A，Task 33-a）
> **Date**: 2026-09-11（r14，批次 E / E1-α）
> **依据**: lang-design 07-bootstrap-strategy §3.2（Expander 属新语言子集）；r6 自举 Reader parity 先例；sop §9.4/§9.5

## 1. 测试目标

验证 E1-α 自举 Expander（expander.krf，VM 上运行）与 Rust 种子
（kerf-expander/*.rs）的行为等价：CoreExpr 结构（原语形态 + Span +
作用域集 + param_scopes）递归一致；错误（消息 + Span）逐字一致；
展开产物端到端可执行（compile + VM 与种子全管线同果）。

## 2. 覆盖场景

| # | 场景 | 类型 |
|---|------|------|
| 1 | 字面量六种 + 顶层符号引用（∅ 作用域集） | parity 正例 |
| 2 | 嵌套 lambda（fresh-scope 注入 / param_scopes / 体内作用域集） | parity 正例 |
| 3 | if（双/三参 + 隐式 nil）/set!/define（值 + 函数糖）/begin | parity 正例 |
| 4 | quote（符号/列表/嵌套/卫生剥离路径） | parity 正例 |
| 5 | 九糖全族（let/letrec/let*/cond/and/or/when/unless/while——loop$hyg$1） | parity 正例 |
| 6 | module（import/export/体）+ require（去重/双能力） | parity 正例 |
| 7 | 内部 define 提升（多名/函数糖嵌套——构造节点 ∅ 作用域） | parity 正例 |
| 8 | 核心形式负例 11（空列表/if 元数/set! 目标/lambda 参数族/define 族/非头部 define） | parity 负例 |
| 9 | 提升路径负例 3（不完整/值元数/非符号——切分名先于值的错误序） | parity 负例 |
| 10 | quote 向量负例 2 + 糖负例 13 + module/require/关键字负例 10 | parity 负例 |
| 11 | define-syntax → E1-α 边界错误（宏属 E1-β） | 边界 |
| 12 | 行为面 3（算术闭包 / cond-while-fib(10) / 内部定义与遮蔽） | 端到端 |

## 3. 测试统计

- 函数 19（parity 正例 8 套件 + 负例 6 套件 + 边界 1 + 行为面 3）；
  断言 = 结构递归比对（core_equiv）+ 消息/Span 逐字 + 值等价
- 注册：tests/runner.rs 批次 E 分组（bootstrap_expander_tests mod）
- 509 基线零回归（r14 全量 528:0:0 实测）

## 4. 依赖

- kerf-driver（bootstrap::read_source / bootstrap_expander::expand_program）
- kerf-expander（种子 oracle：expand_program + ExpandCtxt）
- kerf-compiler + kerf-vm（行为面 compile + run_program）
- 判据边界：expansion_id 不参与（E1-α——r6 Reader parity 同口径）


---

## E1-β 增补（r15，Task 34-a/b——宏收口 + 生产切换）

> **Date**: 2026-09-11（r15）
> **依据**: lang-design 03-macro-system §2.2/§2.4；07-bootstrap §3.2/§3.3；sop §9.4/§9.5

### 测试目标（E1-β）

验证自举 Expander 的**完整宏系统**与 Rust 种子行为等价，且**生产
管线切换**后零回归：

1. **宏 parity**（+17 测试）：define-syntax 注册/nil 字面量、syntax-rules
   解析错误（六类逐字）、卫生 α 重命名（tmp$hyg$N 计数器单调）、省略号
   （零/一/多段 + 复合 (a b ...)）、字面量集合（同名符号匹配 + 子句
   依序）、嵌套列表/向量模式、糖名覆盖（注册表单表替换语义）、module
   体内注册、模式形状不匹配、深度上限 500（自指宏——消息逐字）、
   非列表模式子句解析合法（应用时跳过）；
2. **行为面**（+4）：swap! 运行期交换（卫生不串扰）、递归 my-or 短路、
   卫生无捕获（inc-tmp + 用户 tmp）、深度上限结构化错误（非 VM 帧溢出）；
3. **生产切换守护**（driver 单元，+1）：独立线程活性探针（编译前
   is_loaded=false → 编译后 true）+ 宏产物展开代次 ≥1。

### 关键镜像点（实测驱动发现）

- **Span 并集代次守卫**：节点形状 v2 = (tag s e exp scopes ...)——
  instantiate 的 Span 并集镜像 Span::merge 保守策略（代次不同 → 丢弃
  跨距）；糖产物内宏调用（use-site 代次 ≥1）产物 Span 落模板范围；
- **retag 代次 +1**（bumped_expansion）；全部糖构造器代次来源逐位
  镜像（构造节点 = span 来源节点的代次；dummy 关键字 = 0）；
- **核心节点携带代次**（(tag s e exp ...) → 桥重建 Span）——「展开
  相位标记」诊断特性（E3）经生产切换保持；
- **变换器注册表单表**：用户宏覆盖糖名 + 卫生基名回退对两类同权
  （镜像 ctx.transformers HashMap 惰性注册语义）；
- **foldl/for-each 序章补定义**：chars->str→foldl 此前为死代码路径，
  宏卫生基名回退首次激活（R1 反射——实测驱动补定义）。
