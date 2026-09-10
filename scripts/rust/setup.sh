#!/bin/sh
# kerf/scripts/rust/setup.sh — Rust 工具链安装脚本（sop.md §3.1 环境工具检查与准备）
#
# 用途：安装 Stage 0 必需的 Rust 工具链（rustc + cargo + rustfmt + clippy）
# 用法：sh scripts/rust/setup.sh
# 验证：rustc --version && cargo --version && cargo fmt --version && cargo clippy --version
#
# 说明：
# - 采用 rustup 官方安装脚本（https://sh.rustup.rs），stable 工具链 + minimal profile
# - rustfmt / clippy 作为组件随工具链安装
# - Stage 0 不引入 LLVM/QBE（§21.2 后端策略：自建字节码 VM）
# - 安装失败时不静默跳过（§3.1 规则）：输出错误并以非零退出码结束

set -e

echo "[kerf] 检查既有 Rust 工具链..."
if command -v rustc >/dev/null 2>&1 && command -v cargo >/dev/null 2>&1; then
    echo "[kerf] 已安装: $(rustc --version) / $(cargo --version)"
    MISSING=""
    command -v rustfmt >/dev/null 2>&1 || MISSING="$MISSING rustfmt"
    command -v cargo-clippy >/dev/null 2>&1 || MISSING="$MISSING clippy"
    if [ -z "$MISSING" ]; then
        echo "[kerf] rustfmt 与 clippy 组件齐备，无需安装。"
        exit 0
    fi
    echo "[kerf] 补装缺失组件:$MISSING"
    rustup component add $MISSING
    exit 0
fi

echo "[kerf] 未检测到 Rust，开始安装（rustup stable + minimal profile）..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh
sh /tmp/rustup-init.sh -y --default-toolchain stable --profile minimal --component rustfmt,clippy

# source 环境变量（当前 shell）
. "$HOME/.cargo/env" 2>/dev/null || export PATH="$HOME/.cargo/bin:$PATH"

echo "[kerf] 验证安装..."
rustc --version
cargo --version
cargo fmt --version
cargo clippy --version
echo "[kerf] Rust 工具链安装完成。"
