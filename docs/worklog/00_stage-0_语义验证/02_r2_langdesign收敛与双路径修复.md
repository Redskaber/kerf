# r2 · lang-design v5.1 收敛 + 双路径语义分裂修复

> **覆盖**：2026-09-10 · r2 · Task 10/11（v5.1 收敛审查）
> **记忆类**：e+s（修复：双路径一致性；语义：第 40 号冻结契约 DefineGlobal）
> **状态**：active · 溯源：RELEASE_NOTES v0.1.0-r2 节

## 摘要（压实自 flat 详录，1:8 压缩）

- 06-操作语义从空壳重写：9 原语归约规则 R1-R9 + 编译正确性定理 T1 三引理
- 03-宏系统/05-运行时从冻结实现回填契约（Transformer/Heap/根集）
- 新增第 40 号冻结契约 DefineGlobal（define 与 set! 全局存储语义）
- Define 返回值统一（T1 反例修复：编译模式 value;DUP;SET_GLOBAL）
- lambda 形参重名展开期拒绝（两路径共同上游卫式）
