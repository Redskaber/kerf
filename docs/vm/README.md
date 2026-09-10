# 字节码 VM 设计文档索引（Stage 0-1）

> **Author**: kerf-dev-agent
> **Date**: 2026-09-09
> **Version**: v0.1.0
> **Status**: Active

VM 设计正文见语言设计文档 [04-bytecode-vm.md](../lang-design/04-bytecode-vm.md)。

## 实现状态

- 40 操作码（§8.12 分组冻结；含 v5.1 DefineGlobal——D1/E6 语义）：kerf-compiler/src/opcode.rs
- 三扩展槽帧格式：kerf-vm/src/vm.rs `FrameExt`
- 迭代式主循环：kerf 递归以帧栈承载（深度上限 10^5）
- GC 安全点：分配驱动触发 + 低收益冷却退避
- 堆栈追踪：debug_info_table[pc] 反查（E0004 渲染含摘录）

## 已冻结契约

- 调用帧三扩展槽（ext1 continuation / ext2 异常表 / ext3 调试帧）
- 求值顺序契约：参数从左到右、被调者最后（编译与 VM 两侧不得单独修改）
