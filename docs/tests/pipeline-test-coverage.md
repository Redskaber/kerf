# 流水线路径覆盖（sop.md §9.5.1）

> **Author**: kerf-dev-agent
> **Date**: 2026-09-09
> **Version**: v0.1.0
> **Status**: Active

## 编译流水线测试路径覆盖矩阵

| 路径 | 覆盖测试 | 结果 |
|------|---------|------|
| read：词法正确 | reader_tests（快照/无损/注释/Unicode） | ✅ |
| read：词法错误 | reader_tests::all_reader_errors_carry_spans / greedy | ✅ |
| read：语法错误 | 括号不平衡（开/闭/错配） | ✅ |
| expand：9 核心形式 | expander_tests / vm_tests::nine_primitives | ✅ |
| expand：糖推导 | let/letrec/let*/cond/and/or/while（§3.2 全表） | ✅ |
| expand：宏卫生 | 引入重命名一致 / 用户名保留 / 自引用保留 | ✅ |
| expand：错误路径 | 深度上限 / 非法形式 / quote 符号 / 内部 define 位置 | ✅ |
| lower：图 IR | 共享节点 / Span 元信息 / roots | ✅ |
| compile：回填 | backpatch_leaves_no_placeholders | ✅ |
| compile：常量池 | const_pool_dedup | ✅ |
| compile：闭包 | 捕获描述符 / 嵌套捕获 | ✅ |
| vm：执行语义 | fib(25) / 深递归 / 闭包计数器 / 高阶 | ✅ |
| vm：错误路径 | truthy / arity / 除零 / 未绑定 / 栈下溢 | ✅ |
| runtime：GC | 回收 / 存活保护 / 环 / 深链 / foreign / 统计 | ✅ |
| driver：管线 | E2E / 双路径互查 / 确定性 / 诊断渲染 | ✅ |
| reserved：冻结 | 四项 Probe 实现 + 令牌不可伪造 | ✅ |

覆盖率结论：流水线全部 16 类路径均有正负向覆盖。
