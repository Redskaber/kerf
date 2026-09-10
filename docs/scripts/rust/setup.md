# Rust 环境安装脚本

> **Author**: kerf-dev-agent
> **Date**: 2026-09-09
> **Version**: v0.1.0
> **Status**: Active

## 位置

`scripts/rust/setup.sh`（sop.md §3.4 归档规则）

## 用法

```bash
sh scripts/rust/setup.sh
```

## 行为

1. 检查既有 rustc/cargo（已装则仅补缺失组件后退出）
2. rustup 官方脚本安装 stable + minimal profile + rustfmt + clippy
3. 逐项打印版本验证（§3.1 验证步骤）
4. 失败以非零退出码结束（不静默跳过）

## 实测结果（2026-09-09）

rustc 1.98.1 / cargo 1.98.1 / rustfmt 1.9.0 / clippy 0.1.98
