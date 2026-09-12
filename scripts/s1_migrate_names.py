#!/usr/bin/env python3
"""r42 / 63-b S1 表面腿——27 旧名 → 现代名机械迁移（词法边界感知）。

设计依据：
- 20-表面规范 §7 移除轮行（旧名 27 件移除 → E0021 + 全语料现代名终态）
- 23 §3.4 生命周期（弃用期跨版本核对后移除合法）
- e5-removal-round-plan.md §3.2 S1 行（行为 parity：r38 已证同分派体同诊断）

边界语义：kerf 标识符字符 = [A-Za-z0-9_?!<>=/+-]。替换仅在名字两侧
均非标识符字符时发生（防 `scar`/`str->pos-chars` 类误命中）。
"""
import re
import sys
from pathlib import Path

# 27 对映射（单源 = 20 §8 / builtins.rs BUILTIN_ALIASES）
PAIRS = [
    ("car", "head"), ("cdr", "tail"),
    ("list-ref", "nth"), ("list-tail", "drop"),
    ("null?", "is-nil"), ("pair?", "is-pair"),
    ("int?", "is-int"), ("bool?", "is-bool"),
    ("procedure?", "is-procedure"), ("string?", "is-string"),
    ("symbol?", "is-symbol"), ("float?", "is-float"),
    ("number?", "is-number"), ("list?", "is-list"),
    ("eq?", "eq"),
    ("str-append", "string-append"), ("str-length", "string-length"),
    ("str-substring", "string-substring"), ("str-index-of", "string-index-of"),
    ("str-contains?", "string-contains"), ("str-prefix?", "string-starts-with"),
    ("str-suffix?", "string-ends-with"),
    ("str-upcase", "string-to-upper"), ("str-downcase", "string-to-lower"),
    ("string->symbol", "string-to-symbol"), ("symbol->string", "symbol-to-string"),
    ("assert-eq?", "assert-eq"),
]

IDENT_CHARS = r"A-Za-z0-9_?!<>=/+\-"

def build_pattern(name: str) -> re.Pattern:
    # 名字两侧均非 kerf 标识符字符（词法边界）
    return re.compile(rf"(?<![{IDENT_CHARS}]){re.escape(name)}(?![{IDENT_CHARS}])")

def migrate_text(text: str, skip_predicate=None) -> tuple[str, int]:
    """skip_predicate(line, idx) → True 则该行跳过（保护区）。"""
    total = 0
    out_lines = []
    for idx, line in enumerate(text.split("\n")):
        if skip_predicate and skip_predicate(line, idx):
            out_lines.append(line)
            continue
        new_line = line
        for old, new in PAIRS:
            new_line, n = build_pattern(old).subn(new, new_line)
            total += n
        out_lines.append(new_line)
    return "\n".join(out_lines), total

def main():
    root = Path("/home/z/my-project/kerf")
    # 文件清单（builtins.rs 保护区由行号判定：defs 区 = 别名机制注释块之前）
    targets = {
        "full": [
            root / "crates/kerf-compiler/src/hm.rs",
            root / "crates/kerf-backend/src/anf.rs",
            root / "crates/kerf-driver/src/bootstrap/compiler.krf",
            root / "crates/kerf-driver/src/bootstrap/expander.krf",
            root / "crates/kerf-driver/src/bootstrap/reader.krf",
            root / "crates/kerf-driver/src/bootstrap/preamble.krf",
            root / "examples/usage/io.krf",
            root / "examples/README.md",
        ]
        + sorted((root / "tests").rglob("*.rs"))
        + sorted((root / "examples/audit").rglob("*.rs"))
        + sorted((root / "examples/usage").glob("*.krf")),
    }
    # builtins.rs：仅 defs 区（别名注册块注释起始行之前）——表与 SIGS
    # 由 MultiEdit 手术处理（保护区：REMOVED 表第一列承载旧名）
    builtins = root / "crates/kerf-driver/src/builtins.rs"
    builtins_text = builtins.read_text(encoding="utf-8")
    cutoff = builtins_text.index("    // ------------------------------------------------------------------\n    // 批次 L（v0.5 别名层，r38）")
    head_part, tail_part = builtins_text[:cutoff], builtins_text[cutoff:]

    head_new, n_head = migrate_text(head_part)
    # 尾区（别名机制 + 表 + SIGS + io/模块注册段 + 测试）：测试区与
    # STDLIB_MODULES 底层名需要迁移，但 BUILTIN_ALIASES 表（将被整体
    # 替换为 REMOVED 表）与 SIGS 旧条目（将被删除）保护区跳过。
    # 策略：尾区全量替换后，MultiEdit 以终态文本重写两表（mangled 文本
    # 作为 old_str 定位）。保护区 = io 注册段之后（避免表区误替换）——
    # 改为：先手术两表 + STDLIB_MODULES，再全量替换尾区剩余（测试区）。
    # 本脚本只处理头区；尾区两表先行手术（MultiEdit），随后本脚本
    # 二次运行 --phase2 处理尾区测试段。
    if "--phase2" in sys.argv:
        # 尾区：仅 STDLIB_MODULES 注册段之后（= 测试区 + 模块签名派生段
        # 不含旧名——实际旧名只在测试区）；保护区 = BUILTIN_ALIASES/
        # REMOVED 表与 SIGS（已手术为终态：REMOVED 表旧名列需保留）。
        # 手术后文本中 REMOVED 表以 "pub static REMOVED_BUILTIN_NAMES"
        # 起始、以 "];" 终止——保护区按标记判定。
        protected = False
        out_lines = []
        n_tail = 0
        for line in tail_part.split("\n"):
            if "pub static REMOVED_BUILTIN_NAMES" in line:
                protected = True
            if protected and line.strip() == "];" and "REMOVED" not in line:
                protected = False
                out_lines.append(line)
                continue
            if protected:
                out_lines.append(line)
                continue
            new_line = line
            for old, new in PAIRS:
                new_line, n = build_pattern(old).subn(new, new_line)
                n_tail += n
            out_lines.append(new_line)
        tail_new = "\n".join(out_lines)
        builtins.write_text(head_new + tail_new, encoding="utf-8")
        print(f"builtins.rs phase2: {n_head}+{n_tail} replacements")
        return
    builtins.write_text(head_new + tail_part, encoding="utf-8")
    print(f"builtins.rs phase1 (defs 区): {n_head} replacements")

    for f in targets["full"]:
        text = f.read_text(encoding="utf-8")
        new_text, n = migrate_text(text)
        if n:
            f.write_text(new_text, encoding="utf-8")
        print(f"{f.relative_to(root)}: {n} replacements")

if __name__ == "__main__":
    main()
