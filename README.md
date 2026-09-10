# kerf

**最小自举系统级编程语言**——Stage 0（语义验证，Rust 100% 实现）。

> **设计蓝图**: docs/lang-design/（stage0.md v5.0 拆分，20 篇编号文档）
> **流程管控**: docs/sop.md（v11.0；原名 stage-committee-process.md）
> **阶段状态**: Stage 0 完成——[状态报告](docs/develop/v0/stage-0/status.md)

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
编排: kerf-driver（管线 + 相位分离 + 四项接口预留 + 52 内置注册 + 编译缓存）  基础: kerf-span / kerf-syntax
```

- **9 个正交核心原语**（核心冻结）+ syntax-rules 卫生宏 + 相位分离（含模块循环依赖检测）
- **双执行路径互查**：元循环求值器 vs 字节码 VM（结果逐字节一致；App 求值顺序双侧函数先）
- **标记-清除 GC**：分配驱动 + 冷却退避 + 显式工作栈（根集五来源）
- **Span 全管线传播**：词法→语法→IR→字节码→运行时错误反查（含调用点追踪 note 帧）
- **保守静态类型检查器**（r7）：`kerf check` 多错误收集（E0005，R1-R8
  确定性规则——零误报契约，误报 = P1）；TD-016 全操作数比较前置校验（运行时 + 静态双侧）
- **编译缓存**（r7，13 §3.1.4 做实）：内存内容寻址（SHA-256 键）+
  管线复用（同源二次执行命中——确定性证明锁存）
- **driver 公共调试 API**：`dump_tokens` / `dump_stx`（CLI `tokens` / `stx` 子命令经 driver 转发）

## 质量状态（§3.2 验收全绿，r7）

| 门禁 | 结果 |
|------|------|
| cargo build --release | ✅ 0 警告 |
| cargo check | ✅ 0 errors / 0 warnings |
| cargo test --workspace | ✅ **408 通过 / 0 失败 / 1 忽略**（409 函数；负向 case 1077，正负比 ≈1:3.2） |
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
