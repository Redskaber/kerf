# r8 · 批次 D：能力 I/O 基础传递 + 内部效应 + 用例运行器

> **覆盖**：2026-09-10 · r8 · Task 27-a~e（476 测试）
> **记忆类**：e+s（能力模型 I/O P2 基础传递做实；InternalEffectSystem 编译器内部做实）
> **状态**：active · 溯源：RELEASE_NOTES v0.2.0-r8 节

## 摘要（压实自 flat 详录，1:8 压缩）

- 冻结契约层 InternalEffectSystem（reserved.rs EffectSystem 形状的编译器内部做实——一次性逃逸层）
- 声明形式 (require io read|write)：CoreExpr::Require（零运行时语义，编译期权限验证面）
- R9 保守静态权限验证（门控 I/O 内置名）+ E0006 诊断码
- 令牌授权面 mint_read/write_token（pub(crate) 构造面控制）+ IoGrant
- 用例运行器 kerf test（测试面即语言面）+ FS-4 修复
