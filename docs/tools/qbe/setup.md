# QBE 后端工具链安装记录（批次 G / 38-b）

> **Author**: Super Z (main) — SKL-A
> **Date**: 2026-09-11
> **Version**: v0.1.0
> **Status**: Active

## 位置

- 二进制：`tools/qbe/bin/qbe`（随仓库分发，amd64_sysv 构建）
- 源码归档：`tools/qbe/qbe-1.3.tar.xz`（可重建）
- 安装脚本：`scripts/qbe/setup.sh`

## 用法

```bash
sh scripts/qbe/setup.sh          # 安装/校验
tools/qbe/bin/qbe -h             # 帮助
qbe -o out.s in.ssa              # IL → 汇编
cc -o prog out.s                 # 汇编 → 本地可执行
```

## 版本

QBE 1.3（c9x.me/compile/release/qbe-1.3.tar.xz，281,332 B 源码包；
本机构建产物 670,544 B）。

## 构建方式

```bash
tar -xJf tools/qbe/qbe-1.3.tar.xz
cd qbe-1.3 && make -j4           # C99，cc 构建，无外部依赖
```

## 实测结果（2026-09-11，38-b 落位）

- `make` 零告警完成，产物 `qbe` 支持 `amd64_sysv`（默认）/`amd64_apple`/
  `amd64_win`/`arm64`/`arm64_apple`/`rv64` 六目标
- 三段冒烟：fib 递归 IL → `qbe -o` 汇编 → `cc` 链接 → 运行
  `fib(20)` 递归结果 exit 109 = 6765 mod 256（exit code 8 位截断，数值
  正确；完整值验证走 `fib(12)=144` 口径——与 CLI 冒烟惯例一致）
- 语法勘误实录（GATE 1 诚实记录）：函数签名必须带返回类型
  （`function w $fib(w %n) {`，非 `function $fib(...)`）；整数比较指令
  带宽度后缀（`csltw` 而非 `cslt`）

## SOP 依据

- §3.1（工具缺失查找链：`scripts/` → `tools/` → `docs/tools/` → 安装 → 记录）
- §3.4（安装记录归档规则——本文件即记录）
- 08-backend-evolution §2（Stage 2a QBE 后端引入时机——语义稳定后，
  553:0:0 全绿即满足）
- §21.2 约束（QBE 首选 + 永不入自举链——本工具仅宿主侧调用）
