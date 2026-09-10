# r6 · B3 自举 Reader（Reader kerf 重写）

> **覆盖**：2026-09-10 · r6 · Task 24-a~d（356 测试）
> **记忆类**：e+s（里程碑：Reader 自举——首个自举面切换）
> **状态**：active · 溯源：RELEASE_NOTES v0.1.0-r6 节

## 摘要（压实自 flat 详录，1:8 压缩）

- reader.krf ~430 行 kerf 源码（bootstrap/reader.krf）——Reader 的 kerf 实现
- VM 宿主调用 API kerf_vm::call_closure（函数入口帧语义）
- 自举桥 bootstrap.rs：种子管线编译 reader.krf → 产物缓存复用
- 生产读路径切换 compile_front（run/eval/compile_source 全管线走 kerf Reader）
- parity 28 函数：正 87/负 307 case 全绿；全套件经自举 Reader 执行
