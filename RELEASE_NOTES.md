# RELEASE_NOTES

## v0.1.0 —— Stage 0 语义验证（2026-09-09）

### 交付

- **12（10+2）能力模型全量落地**（P0/P1）：
  类型化 Token Reader / 图 IR（共享节点）/ 结构化 CodeValue /
  元循环求值器 / 基础闭包（共享单元格捕获）/ Span 全管线 /
  结构化诊断 / 最小 I/O / 相位分离 / 卫生宏（syntax-rules）/
  标记-清除 GC / 字节码 VM（39 操作码）
- **四项接口预留冻结**（P2/P3）：Effect Handlers / 多阶段编程 /
  能力模型 I/O / 编译缓存
- **9 crates Cargo Workspace**（零外部依赖）+ CLI（10 子命令）
- **200 项测试全绿**；§3.2 验收全绿（build/check/test/fmt/clippy）
- **文档**：lang-design 20 篇（stage0.md v5.0 拆分）+ develop/tests/graph 树

### 语义验证亮点

- fib(25) = 75025（VM 与 eval 双路径一致）
- 卫生宏：宏内外同名不串扰（一致性 α 重命名 + 自引用保留 + 关键字保留）
- letrec 互递归 / 闭包可变捕获（独立计数器）/ 深递归 10^4（迭代式帧栈）
- GC：3×10^5 临时分配堆有界；环回收；2×10^5 深链标记不爆栈

### 已知边界

TD-002~TD-011（P2/P3，显式推迟）——见
docs/develop/v0/tech-debt-register.md。无 P0/P1 遗留。

### 基准

- fib(25)（含编译）：84.7 ms/轮（release）
- GC 压力：0.72 s / 3×10^5 分配

### 下一步

Stage 1（自举验证）：目标语言子集重写编译器前端 + 类型检查器。
