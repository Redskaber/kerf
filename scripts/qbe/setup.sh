#!/bin/sh
# kerf/scripts/qbe/setup.sh — QBE 后端工具链安装脚本（sop.md §3.1）
#
# 用途：安装/校验 Stage 2 批次 G（QBE 后端 PoC）必需的 QBE 二进制
# 用法：sh scripts/qbe/setup.sh
# 验证：tools/qbe/bin/qbe -h（存在且可执行）
#
# 说明：
# - 优先使用仓库内二进制（tools/qbe/bin/qbe，amd64_sysv）
# - 缺失时从源码归档重建（tools/qbe/qbe-1.3.tar.xz，C99 + cc，无外部依赖）
# - 失败以非零退出码结束（§3.1 规则：不静默跳过）
# - QBE 永不进入自举链（§21.2）——仅宿主侧子进程调用

set -e

BIN="tools/qbe/bin/qbe"

echo "[kerf] 检查 QBE 二进制..."
if [ -x "$BIN" ]; then
    echo "[kerf] 已就位: $BIN"
    "$BIN" -h 2>&1 | head -1 || true
    exit 0
fi

echo "[kerf] 二进制缺失，从源码归档重建..."
if [ ! -f tools/qbe/qbe-1.3.tar.xz ]; then
    echo "[kerf] 错误: 源码归档缺失（tools/qbe/qbe-1.3.tar.xz）"
    exit 1
fi

WORK=$(mktemp -d)
tar -xJf tools/qbe/qbe-1.3.tar.xz -C "$WORK"
( cd "$WORK/qbe-1.3" && make -j4 )
mkdir -p tools/qbe/bin
cp "$WORK/qbe-1.3/qbe" "$BIN"
rm -rf "$WORK"

echo "[kerf] 重建完成: $BIN"
"$BIN" -h 2>&1 | head -1
