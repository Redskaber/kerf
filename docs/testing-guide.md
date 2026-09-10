# 测试指南

> **Author**: kerf-dev-agent
> **Date**: 2026-09-09
> **Version**: v0.1.0
> **Status**: Active

## 结构（sop.md §9.1）

```text
tests/
├── v0/stage0/plan/    # 开发轮集成测试（reader/expander/compiler/vm/gc/pipeline）
├── v0/stage0/gate/    # 审查轮审计测试（gate_review_r1）
└── common/            # 共享辅助（run/run_rendered/dual_path_agrees）
```

crate 单元测试内联于 `src/*.rs` 的 `#[cfg(test)]`。

## 运行

```bash
cargo test --workspace                     # 全量
cargo test -p kerf-vm                      # 单 crate
cargo test --test vm_tests                 # 单集成套件
cargo test -p kerf-vm -- deep_recursion    # 单测试
```

## 编写规范

- 新测试按 `tests/v0/stage-N/plan/` 放置 + `Cargo.toml [[test]]` 声明
- 每个测试文件对应 `docs/tests/v0/stage-N/plan/X.md`（双向印证）
- 负向/错误测试优先（§9.4.3）；快照黄金输出经实际运行校准
- 双路径互查：`common::dual_path_agrees`（VM vs eval 一致性）
