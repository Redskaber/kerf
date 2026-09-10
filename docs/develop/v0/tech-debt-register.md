# 综合技术债登记册

> **Author**: kerf-dev-agent（ARCH-A 角色）
> **Date**: 2026-09-09
> **Version**: v0.1.0
> **Status**: Active
> **规则**: sop.md §6.2.1——新增已解决项/调整剩余项优先级（每子阶段必检）

## 索引

| TD ID | 标题 | 等级 | 状态 | 目标阶段 |
|-------|------|------|------|---------|
| TD-002 | quote 符号/向量值类型缺失 | P3 | 开放 | Stage 1 |
| TD-003 | 图 IR 复合节点 CSE 共享 | P3 | 开放 | Stage 2 |
| TD-004 | 作用域集解析（Racket 式）替换名称基解析 | P2 | 开放 | Stage 1 |
| TD-005 | syntax-parse 级宏组合 | P3 | 开放 | Stage 2 |
| TD-007 | 迭代式展开工作表（解除深度上限 128） | P2 | 开放 | Stage 1 |
| TD-008 | 分代 GC / 堆压缩 | P3 | 开放 | Stage 2 |
| TD-009 | eval 路径 GC 根集枚举 | P3 | 开放 | Stage 1 |
| TD-010 | 闭包/内置函数装箱（pair 元素） | P3 | 开放 | Stage 1 |
| TD-011 | 字符串全序比较 | P3 | 开放 | Stage 2 |

## 详情

### TD-002 quote 符号/向量值类型缺失
- **描述**：`LiteralValue` 无 Symbol/Vector 变体——`(quote sym)` 显式报错
- **根因**：Stage 0 值模型最小化（§3.2 literal_value 定义如此）
- **修复方案**：Stage 1 增加 `Value::Symbol`；quote 符号 datum → 符号值
- **影响范围**：expander::datum_to_value / vm::value
- **优先级依据**：符号值影响宏编程体验（P3——不影响语义验证闭环）

### TD-003 图 IR 复合节点 CSE 共享
- **描述**：字面量/变量节点按结构键共享；复合节点不做激进 CSE
- **根因**：§21.11 风险表既定缓解（图 IR 过于复杂 → 树承载 + Stage 2 迁移）
- **修复方案**：Stage 2 哈希cons 复合节点 + SSA 化
- **workaround**：字面量共享已满足「公共子表达式」演示与验收

### TD-004 作用域集解析
- **描述**：标识符解析为名称基（编译期 slot 查找 + 运行期全局名）；
  Racket 式 scope-set 子集匹配未实现（ScopeSet 已在数据结构中全程携带）
- **根因**：名称基 + 一致性卫生重命名已满足 P1 卫生保证；
  scope-set 解析是 Stage 1 语言级宏的前置
- **修复方案**：编译器 resolve 按 (name, scopes ⊆) 匹配绑定
- **卫生回退**：$hyg$ 后缀剥离（driver::resolve_hygiene_fallbacks）为
  名称基解析的显式近似

### TD-007 迭代式展开
- **描述**：展开深度上限 128（rustc 默认对齐）；文档示例 10_000 需迭代式
- **修复方案**：展开工作表化（显式队列替代递归下降）

### TD-009 eval 路径 GC 根集
- **描述**：元循环求值器关闭 GC 触发（根集枚举需遍历 Rc 环境链）
- **修复方案**：Stage 1 根集遍历或（按 §21.9 演进矩阵）eval 被编译器替换

### TD-010 闭包装箱
- **描述**：序对元素为闭包/内置时以标记字符串占位（不可达路径）
- **修复方案**：Stage 1 HeapObj::Foreign(Rc<dyn Any>)
