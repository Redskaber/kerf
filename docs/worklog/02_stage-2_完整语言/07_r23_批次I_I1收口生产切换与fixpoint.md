# r23 批次 I 执行：I1 收口（生产切换 + 门 B fixpoint 两次编译自身字节一致 + eval 退役）

> Task ID: 44-a/44-z（plan 42-d + 收尾）· 2026-09-11 · 溯源：flat worklog（kerf/docs/worklog.md 尾部条目）· RELEASE_NOTES v0.4.0-r23

## 概要（1:8 压缩——详录见 flat）

- **42-d（I1 收口——S3 段生产切换 + 自举终局）**：
  `CompilerKind::{Bootstrap,Seed}` pub enum 分派（front_from_core 第
  4 步——镜像 ExpanderKind；生产 = compiler.krf 在 VM 上运行；**bootstrap
  init 恒种子路径**（load_compiler/load_bootstrap/load_expander 三处
  compile_front_seed——无递归 P1 硬约定））；check_source_recover 恢复
  路径同生产口径；守护 production_compiler_is_bootstrap（独立线程双信号：
  is_loaded 前后翻转 + 种子路径不加载）。
- **门 B fixpoint（§21.3 条件 2 终验——首跑全过）**：B₁ = 生产链编译
  自举三件 + preamble；B₂ = 三件 `install_state`（B₁ 产物为新自举状态
  ——「产物编译自身」字面语义）后再编译同源；**硬门判据**：四程序
  bytecode_equal 全结构一致（含 debug_spans）+ **SHA-256 摘要一致**
  （Debug 结构序确定序列化——hash.rs 自研 sha256_hex 零新依赖）；隔离
  纪律 §7.4（关缓存 + 三 reset fresh 起步——缓存命中假阳性防线）。
- **实测两裁定（诚实入档——设计 v1.3 ①②）**：① 测试渲染器魔法符号
  （Symbol(u32::MAX-1)）越界——镜像 driver resolve_symbol（<main>/
  <lambda>）口径修正；② **宏自由件跨链全结构 bytecode_equal 不成立**
  ——两链符号表 intern 序不保证一致（按名反汇编已证结构+名+span 位置
  全等；差异仅 Symbol 数值）——§7.3「名字是唯一稳定口径」预判实证；
  跨链终验口径 = 按名反汇编（加强判据），全结构判据限同链 B₁/B₂。
- **B₁ 可执行面**：gate_b_b1_programs_execute_as_bootstrap_chain——
  B₁ 字节码 install 后作为自举 Compiler 实际运转（VM 执行 = 种子链
  结果）——行为级同证。
- **eval 退役终态（INC7 兑现）**：CLI eval 子命令移除 + driver
  eval_source 删除（**自举编译器 + VM = 唯一生产路径**）；kerf-vm
  eval.rs 存档为 Rust 参考实现（scope_set_tests 语义 oracle 保留）；
  **T1 新口径** = run_source_seed/compile_source_seed pub 参考入口 +
  测试面全量迁移（stage0/1 十文件 + common + 双审计集——断言迁移非
  删除）；TD-017/TD-009 联动 resolved（eval 域随路径注销——INC7 清单
  兑现）；12 §2.5 行 299 终态回写。
- **缓存分桶（B11/P4）**：cache_key 三参（CompilerKind 入指纹构成层
  ——CacheKey 冻结结构不动）；种子路径不查不存；键区分 + 跨桶隔离 +
  集成实测三面。
- **质量口径**：**670:0:0**（665 零回归 + 净 5——单元 205（driver
  64→67）/ 集成 465（bootstrap_compiler 27→29））；§3.2 六命令 clean
  起步全绿；CLI 七冒烟 + 双审计 EXIT 0；**表体两处实测修正**（driver
  单元 58→67 r12 陈旧数 + bootstrap_compiler 29——--list 权威对账）；
  r23 tar.gz（304 条目）+ 包内自举验证（13.14s + 670 复跑 + CLI 一致）；
  web 同步（kerf-data r23 + footer v6.3）+ agent-browser E2E。
- **I1 段交付闭环（S1→S2→S3 三段全成）**：生产管线前段三段（读+展+
  编）= 100% kerf（§21.3 条件 1 + INC8 机器口径）；下一步 42-e I2
  （stdlib/GC/TD 批——TD-009 已随本轮 resolved 清单更新）。
