# 测试指南

> **Author**: kerf-dev-agent
> **Date**: 2026-09-10（r9：测试入口架构重构——runner.rs 单一总入口）
> **Version**: v0.1.0-r9
> **Status**: Active

## 结构（sop.md §9.1 + §8.4.6 测试入口架构意图 v11.2）

```text
tests/
├── runner.rs          # 跨 crate 集成测试唯一总入口（cargo 自动发现，
│                      #   Cargo.toml 零 [[test]] 声明——#[path] mod 树
│                      #   挂载下列全部测试文件）
├── v0/stage0/plan/    # 开发轮集成测试（reader/expander/compiler/vm/gc/pipeline）
├── v0/stage0/gate/    # 审查轮审计测试（gate_review_r1）
├── v0/stage1/plan/    # Stage 1 批次套件（worklist/stdlib/bootstrap/typecheck/cache/capability/test_runner）
└── common/            # 共享辅助（run/run_rendered/dual_path_agrees——
                       #   runner 单实例共享，各模块 use crate::common）
```

crate 单元测试内联于 `src/*.rs` 的 `#[cfg(test)]`。

## 运行

```bash
cargo test --workspace                              # 全量（476 = 单元 175 + 集成 301）
cargo test -p kerf-vm                               # 单 crate
cargo test --test runner                            # 全部集成测试（单二进制）
cargo test --test runner vm_tests::                 # 单集成模块（模块路径即过滤器）
cargo test --test runner negative_vm_tests::read_line_arity   # 单测试
cargo test -p kerf-vm -- deep_recursion             # 单单元测试
```

## 编写规范

- 新测试按 `tests/v0/stage-N/plan/` 放置 + **`tests/runner.rs` 对应 stage
  分组加一行 `#[path]` mod 声明**（禁止 `Cargo.toml [[test]]` 逐文件声明
  ——sop.md §8.4.6 强制规则 8；Cargo.toml 仅总入口，保持干净精要）
- 共享辅助统一经 `use crate::common` 消费（runner 单实例——勿在测试
  文件内 `mod common` 重复加载）
- 每个测试文件对应 `docs/tests/v0/stage-N/plan/X.md`（双向印证）
- 负向/错误测试优先（§9.4.3）；快照黄金输出经实际运行校准
- 双路径互查：`common::dual_path_agrees`（VM vs eval 一致性）
- 嵌套 example（examples/audit/ 审计脚本）仍需 `[[example]]` 声明
  （cargo 对 example 无 mod 树等价机制——唯一例外）
