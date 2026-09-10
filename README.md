# kerf

**最小自举系统级编程语言**——Stage 0（语义验证，Rust 100% 实现）→ Stage 1（自举验证进行中）。

> **设计蓝图**: docs/lang-design/（stage0.md v6.1 拆分，20 篇编号文档，v6.1——含 next4 第八轮吸收：接口预留完整性审查 14 项 / 原则 32 预留留白 / 内部语法三原则 / 8 原语 Stage 2 迁移映射）+ 上游蓝图存档 docs/stage0.md（v6.1 全文）
> **流程管控**: docs/sop.md（v11.4——三十二条设计原则 + §21.12 接口预留时机；原名 stage-committee-process.md）
> **阶段状态**: Stage 1 批次 A-D 交付 + r9-r12 吸收审计轮（r12：next4 接口预留完整性扩展——预留层 4→14 项 trait 冻结 + 20 新测试，500 全绿）——[阶段计划](docs/develop/v0/stage-1/plan.md)

## 30 秒了解

```scheme
; fib.krf
(define (fib n)
  (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2)))))
(print (fib 25))   ; 75025

; 卫生宏
(define-syntax swap!
  (syntax-rules ()
    ((swap! x y) (let ((tmp x)) (set! x y) (set! y tmp)))))

; 闭包（可变捕获）
(define (make-counter)
  (let ((n 0)) (lambda () (set! n (+ n 1)) n)))
```

```bash
cargo build --release
./target/release/kerf run examples/usage/fib.krf   # 75025
```

## 架构（9 crates，零外部依赖）

```text
源码 → kerf-reader（Token/Stx）→ kerf-expander（CoreExpr 9 原语 + 卫生宏）
     → kerf-core（图 IR + CodeValue）→ kerf-compiler（字节码 40 操作码 + 静态检查器）
     → kerf-vm（switch-dispatch + 元循环求值器双路径）→ kerf-runtime（GC 堆）
编排: kerf-driver（管线 + 相位分离 + 四项接口预留 + 52 内置能力参数化 + 编译缓存 + 内部效应 + kerf test 用例运行器）  基础: kerf-span / kerf-syntax
```

- **9 个正交核心原语**（核心冻结——穷尽 match 机器证明，r10 架构合规审计）+ syntax-rules 卫生宏 + 相位分离（含模块循环依赖检测）
- **核心原语演进登记**（v6.0）：8 原语形态 = Stage 2 评估目标（语义等价映射：SetBang→Perform(State) / Define→脱糖 / Begin→Let 链 + 新增 Let·Perform·Handle）；内部语法三原则即刻生效——类型安全优于命名安全 / 语义化命名 / 零冗余（类型安全 ADT）
- **接口预留完整性扩展**（v6.1/r12）：预留层 4→14 项——P0 三项（LSP/IDE 查询 `LanguageService`+`IncrementalAst` / 调试信息 `DebugInfoGenerator`+`DebugTraceable` / 增量编译查询 `QuerySystem`+`Query`）+ P1（FFI 边界 `ExternalType`/`FfiCall`/`FfiBoundary` + 多目标后端 `CodegenBackend`/`WasmBackend` + 编译器即服务 `CompilerService`/`Serializable`）+ P2（包管理 `PackageManager`/`ExternalModule` + AI 辅助 `AiAssistant`）——reserved/ 模块三文件冻结，Probe 测试 + P0 位置跨 crate 断言 20 项交付
- **自举 Reader**（r6）：reader.krf（~430 行 kerf 源码）在 Stage 0 VM 上运行——生产读路径整体切换，与种子逐字节等价（parity 正 87/负 307）
- **能力门控 I/O**（r8）：(require io read|write) 声明 → R9 保守验证（E0006 编译期）→ 不可伪造令牌——fail-closed
- **双执行路径互查**：元循环求值器 vs 字节码 VM（结果逐字节一致；App 求值顺序双侧函数先）
- **标记-清除 GC**：分配驱动 + 冷却退避 + 显式工作栈（根集五来源）
- **Span 全管线传播**：词法→语法→IR→字节码→运行时错误反查（含调用点追踪 note 帧）
- **保守静态类型检查器**（r7）：`kerf check` 多错误收集（E0005，R1-R8
  确定性规则——零误报契约，误报 = P1）；TD-016 全操作数比较前置校验（运行时 + 静态双侧）
- **编译缓存**（r7，13 §3.1.4 做实）：内存内容寻址（SHA-256 键）+
  管线复用（同源二次执行命中——确定性证明锁存）
- **内部效应系统**（r8）：类型化一次性逃逸层（任意嵌套深度零签名污染）——消费面 `kerf test`（前置逐用例重放 / 效应短路与恢复）
- **driver 公共调试 API**：`dump_tokens` / `dump_stx`（CLI `tokens` / `stx` 子命令经 driver 转发）

## 质量状态（§3.2 验收全绿，r10）

| 门禁 | 结果 |
|------|------|
| cargo build --release | ✅ 0 警告 |
| cargo check | ✅ 0 errors / 0 warnings |
| cargo test --workspace | ✅ **500 通过 / 0 失败 / 0 忽略**（500 函数 = 单元 183 + 集成 317，r9 起经 tests/runner.rs 单一总入口组织；含 r10 架构合规审计 4 + r12 预留扩展 20（Probe 冻结 8 + 跨 crate 位置断言 12）；负向 case ≈1118，正负比 ≈1:3.2） |
| cargo fmt --check | ✅ 零 diff |
| cargo clippy -D warnings | ✅ 0 警告 |

基准：fib(25) 84.4ms/轮（release，含编译，CLI `bench`）；GC 压力 3×10^5 分配 ~0.17s/轮。
门审计集：`cargo run --example stage0_gate_audit_r1`（41 case，§7.3.1）。

## 目录

```text
crates/         9 个成员 crate（§8.4.6 两级结构）
tests/v0/       跨 crate 集成测试（阶段树；含 negative_* 四文件负测）
examples/       usage/（6 个 .krf 演示）+ audit/（门审计集）+ README
benchmarks/     空占位（实际载体：CLI bench + examples/usage/）
docs/           lang-design / develop / tests / graph / ...
scripts/        环境脚本（rust/setup.sh）
```

## 路线

Stage 1：目标语言子集重写前端 + 类型检查器（Rust ~70% + kerf ~30%）→
Stage 2 完整语言 + 可选 QBE/C 后端 → Stage 3+ 完全自举（LLVM 仅发布构建，
永不进入自举链）。
