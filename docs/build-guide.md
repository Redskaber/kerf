# 构建指南

> **Author**: kerf-dev-agent
> **Date**: 2026-09-09
> **Version**: v0.1.0
> **Status**: Active

## 环境要求

- Rust stable（≥ 1.98）+ rustfmt + clippy（安装：`sh scripts/rust/setup.sh`）
- **零外部 crate 依赖**——离线可构建

## 构建与验证

```bash
cargo build --release          # 发布构建（二进制 target/release/kerf）
cargo check                    # 快速类型检查（0 警告门禁）
cargo test --release --workspace  # 全量测试（200 项）
cargo fmt --check              # 格式门禁
cargo clippy --all-targets -- -D warnings  # lint 门禁
```

## CLI 用法

```bash
kerf run <file.krf>     # 编译 + VM 执行（打印最终值 ⇒）
kerf eval <file.krf>    # 元循环求值器（参考路径）
kerf check <file.krf>   # 干编译（诊断输出）
kerf tokens/stx/core/ir/bc/code <file>  # 管线各级 dump
kerf bench <file> [N]   # 性能基准
```

## 基准基线（2026-09-09，release）

- fib(25)（含编译）：84.7 ms/轮
- GC 压力（3×10^5 临时分配）：0.72 s（堆有界）

## 打包（§19）

```bash
tar -czf /home/z/my-project/download/kerf-stage0-v0.1.0-<details>-r<N>.tar.gz \
    --exclude=target --exclude=.git .
```
