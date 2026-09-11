# 11_r14 E1-α 自举 Expander（核心形式 + 九糖 parity）

**覆盖**：r14 · Task 33-a/33-b · 2026-09-11 · 记忆类：e+s+p

## 概述

批次 E / E1-α：Expander 以 kerf 源码重写（`bootstrap/expander.krf`
~910 行——种子管线编译 208 原型 / 4685 指令，迄今最大 kerf 程序），
在 Stage 0 VM 上运行；与 Rust 种子（oracle）parity 19 测试全过；
509 基线零回归（528:0:0）。**影子路径**——生产展开仍走 Rust 种子，
宏与生产切换属 E1-β。

## 压实摘要

- **架构**（r6 自举 Reader 先例同型）：自举程序 + 值桥
  （`bootstrap_expander.rs`：Stx → VM datum 节点 `(tag s e scopes ...)`
  → VM core 节点 → CoreExpr——作用域集 int 列表随节点携带、桥侧排序
  去重重建）+ 种子 oracle
- **镜像语义**：9 核心形式 + 九糖（loop$hyg$1 常量——HygieneCtx 每实例
  化计数重置镜像）+ 内部 define 提升（切分期名校验先于值校验的错误
  短路序）+ **r13 fresh-scope 深注入与作用域集携带** + trampoline +
  retag use-site 替换 + quote datum 转换（$hyg$ 剥离经字符表模式匹配
  ——绕开保守类型检查的 num 算术索引边界）
- **parity 判据**：结构（原语形态 + Span(start,end) + 作用域集 +
  param_scopes 递归一致）+ 错误（消息 + Span 逐字——含提升路径短
  消息口径）+ 行为面（产物经 compile + VM 与种子全管线同果）；
  expansion_id 不参与（E1-α 边界——r6 Reader 同口径）
- **实测驱动修复六类缺陷**：let 括号缺失（body 吞入绑定组）/
  'true-'false-'nil 字面量 vs 符号（tag 比较恒假→string->symbol 铸造）/
  desugar-letrec s/e 先用后绑 / module-header 名单误取首名 /
  num 算术索引×保守类型检查两处重写 / 点对语料移除
- **接口**：`IoGrant::none()` pub 化（接口最小放宽——宿主/测试侧
  无能力全局环境构造入口；令牌铸造面保持 crate 私有）
- **交付**：§3.2 六命令全绿；审计 EXIT 0；r14 tar.gz（根 worklog.md
  排除 + rec 树入包 + 包内自举验证 528:0:0）；web 同步；文档五处
  （matrix/plan/bootstrap-expander.md/RELEASE_NOTES/07-bootstrap）

## 溯源指针

详录 = flat worklog.md Task 33-a / 33-b 节；RELEASE_NOTES v0.3.0-r14
节；docs/tests/v0/stage1/plan/bootstrap-expander.md；07-bootstrap
§3.2 进度标记。
