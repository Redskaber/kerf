# r17 批次 G：后端 / FFI / 类型三主线（QBE 后端 PoC + FFI 所有权模型 + HM 设计 + TD-013 resolved）

> Task ID: 38-a ~ 38-f · 2026-09-11 · 溯源：flat worklog（kerf/docs/worklog.md 尾部六条目）· RELEASE_NOTES v0.4.0-r17

## 概要（1:8 压缩——详录见 flat）

- **G3（38-a）**：FFI 所有权模型定义（§21.3 阻塞项解除）——
  ffi-ownership-model.md 216 行：pin/unpin 形式化（Φ 计数簿 σ=(H,R,Φ) +
  P1/P2/U1/U2 归约 + 引理 F-PIN——GC 主循环零改动）+ 线性令牌（可重复
  借用 + 一次性消费；失效 E9 诊断非 UB）+ 三原语责任矩阵 3/3 + 边界
  case 13 + 决策 19（新裁定 12 显式标注）。实现路线：G1 PoC 整数路径
  不触及 pin；批次 I write_stdout 借用路径做实。回写义务 4 处登记。
- **G1（38-b/38-c）**：QBE 后端 PoC（**§21.3 条件 3 兑现——首个非 VM
  后端工作**）：
  - 工具链：QBE 1.3 c9x.me 下载构建落位 `tools/qbe/bin/qbe`（+ 源码
    归档 + scripts/qbe/setup.sh 重建 + docs/tools/qbe/setup.md 记录，
    §3.1 全链）；
  - `kerf-backend` 第 10 crate：契约**迁移**（CodegenBackend 族自
    reserved 迁入正式家——签名零变化（原则 27）+ reserved 薄 re-export
    兼容锁存测试）+ AnnotatedANF **实化**（占位指纹 → 块式 ANF 函数
    定义集；fingerprint 字段保留）+ lowering（phi 值合并 + 跳转回填
    （§19.3 不变式 2 同型）+ arity 静态校验）+ QbeBackend + aot 编排
    （qbe→cc→run 子进程链 + 查找链三级）；
  - CLI 11→13：`anf` / `native`；
  - **端到端：fib(12) ⇒ exit 144 本地码 = VM ⇒ 144 双路径一致**；
  - 实现勘误 5 项如实入档（QBE 语法 2：签名返回类型 / csltl 后缀；
    phi 替代块参数；else/merge 回填；not=ceqw；fingerprint 全量 hash；
    并行测试竞态 2 处）。
- **G2（38-d）**：HM 推断设计轮 303 行——R1-R8 行号锚盘点 + 11 裁定
  （**约束三段式**，否决 W/J）+ P0 冲突 3（letrec nil / 保守契约 /
  数值塔格）→ **GO（有条件）**：PoC = 批次 H 新增 MUV H4。
- **TD-013（38-e）resolved**：双路径恢复（种子 recover.rs + 桥逐形式
  单元素列表——expander.krf 零改动）+ DiagCollector（128/截断/位置序）
  + check_source_recover（E0002+E0005 合并报告；front_from_core 抽段
  单一实现）+ CLI check 恢复模式；r7 验收 5 条全过。
- **收尾（38-f）**：§3.2 六命令全绿（clean 起步 build 11.52s / **605:0:0**）
  + 双审计 EXIT 0 + 对账六面（RELEASE_NOTES v0.4.0 / matrix v-r17 /
  登记册 TD-013 resolved + TD-024 新增 / coverage / roadmap + plan /
  lang-design 13 §3.3.7 + 10-toolchain v6.3）+ r17 tar.gz 282 条目
  1.5MB（**tools/ + scripts/ 入包**——§19.4 命令扩展）+ **包内自举
  605:0:0 + CLI 四冒烟**。

## 量化

测试 553 → **605:0:0**（净 +52：集成 +40（qbe 24 + 恢复 16）+ 单元
+12）；CLI 13 子命令；crate 10 成员；QBE 1.3（670,544B）；r17 包
282 条目 / 1,504,494B；TD-013 resolved + TD-024 登记（P3）。

## 勘误实录（规程记忆——下轮直接避坑）

1. QBE 函数签名必带返回类型（`function l $fib(l %n) {`）；
2. QBE 整数比较带宽度后缀（csltl/ceql/ceqw——非 cslt）；
3. QBE 1.3 无 jmp 带参/块参数语法——值合并走 phi 指令（块首）；
4. lowering 分支目标在 then 内嵌 if 分裂多块时不可 seal 预测——
   占位 + 回填（phi 来源 = 实际跳转块索引）；
5. 测试并行：进程级 env（set_var）污染 find_qbe——纯函数注入式；
   共享目录 remove_dir_all 互删——原子计数器唯一目录；
6. 截断标记的 break 路径不经 push 分支——显式 mark_truncated。
