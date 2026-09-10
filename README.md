# kerf

**最小自举系统级编程语言**——Stage 0（语义验证，Rust 100% 实现）。

> **设计蓝图**: docs/lang-design/（stage0.md v5.0 拆分，20 篇编号文档）
> **流程管控**: docs/stage-committee-process.md（sop.md v11.0）
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
./target/release/kerf run examples/fib.krf
```

## 架构（9 crates，零外部依赖）

```text
源码 → kerf-reader（Token/Stx）→ kerf-expander（CoreExpr 9 原语 + 卫生宏）
     → kerf-core（图 IR + CodeValue）→ kerf-compiler（字节码 39 操作码）
     → kerf-vm（switch-dispatch + 元循环求值器双路径）→ kerf-runtime（GC 堆）
编排: kerf-driver（管线 + 相位分离 + 四项接口预留）  基础: kerf-span / kerf-syntax
```

- **9 个正交核心原语**（核心冻结）+ syntax-rules 卫生宏 + 相位分离
- **双执行路径互查**：元循环求值器 vs 字节码 VM（结果逐字节一致）
- **标记-清除 GC**：分配驱动 + 冷却退避 + 显式工作栈
- **Span 全管线传播**：词法→语法→IR→字节码→运行时错误反查
- **四项接口预留冻结**（P2/P3）：Effect Handlers / 多阶段 / 能力模型 I/O / 编译缓存

## 质量状态（§3.2 验收全绿）

| 门禁 | 结果 |
|------|------|
| cargo build --release | ✅ 0 警告 |
| cargo check | ✅ 0 errors / 0 warnings |
| cargo test --release | ✅ **200 通过 / 0 失败** |
| cargo fmt --check | ✅ 零 diff |
| cargo clippy -D warnings | ✅ 0 警告 |

基准：fib(25) 84.7ms/轮（release，含编译）。

## 目录

```text
crates/         9 个成员 crate（§8.4.6 两级结构）
tests/v0/       跨 crate 集成测试（阶段树）
examples/       用户演示（§9.6）
docs/           lang-design / develop / tests / graph / ...
scripts/        环境脚本（rust/setup.sh）
```

## 路线

Stage 1：目标语言子集重写前端 + 类型检查器（Rust ~70% + kerf ~30%）→
Stage 2 完整语言 + 可选 QBE/C 后端 → Stage 3+ 完全自举（LLVM 仅发布构建，
永不进入自举链）。
