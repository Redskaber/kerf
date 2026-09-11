# r22 批次 I 执行：I1 中段糖/module/require 面全迁移（全臂 parity 就绪）

> Task ID: 43-a/43-z（plan 42-c + 收尾）· 2026-09-11 · 溯源：flat worklog（kerf/docs/worklog.md 尾部条目）· RELEASE_NOTES v0.4.0-r22

## 概要（1:8 压缩——详录见 flat）

- **42-c（I1 中段——module/require 两臂 + 糖全管线）**：
  `compiler.krf` 两臂迁移落地——module 体 inline（`compile-module-body`
  + `compile-module-seq` 无 tail 参数——**逐项恒非尾位**（区别于
  begin 末项继承）+ 中间 Pop 携项自身 Span + 空体 PushNil 携 module
  Span）+ require = PushNil（r8 零字节码语义——R9 在 driver 前端，
  编译段零感知）；42-b 边界 cerr 两处移除——**边界不对称消除**
  （生产切换 CompilerKind 属 42-d）。
- **值等价发现**（设计 v1.2 ①）：Rust `pm.name = Symbol(u32::MAX-1)`
  与 proto 0 创建（:235 同值）恒等——krf 侧 proto 0 名形创建/回写均
  '(main)'（桥映射同值）→ 镜像不引入冗余突变；等价性经 bytecode_equal
  全结构判据（含 name 字段）46 case 实证。
- **门 A 扩展组（19→27 函数 / 46 parity case ≥12 超额）**：糖九件
  全管线（let 家族 8 + cond/when/unless 9 + and/or/while 10——
  expander 脱糖 → 双编译路径）+ 宏 3（swap!/my-or/def-twice）+
  **prelude 注入序**（preamble.krf 全文真实语料——生产前端实际注入
  对象）+ **examples 六件双路径 bytecode_equal** + 行为面 2 组
  （糖 + module 臂 = 生产管线含 registry 前端面）+ 确定性双跑。
- **语料实测勘误两处（诚实入档）**：条件位严格 bool（(and 1 2 3)
  E0004——行为语料改 bool 条件；parity 不受影响）+ 同层 let 重名是
  展开器错误（lambda 形参重名拒绝）。
- **表体对账三处补齐**（--list 权威计数发现）：matrix 表体 Stage 2
  五行（r17-r22 漏行）+ cache 14→13 实测修正 + pipeline Tier 2
  bootstrap_compiler 行（r21 头部有表体漏）——r16 同型 header-表体
  同步义务漏网。
- **收尾（43-z）**：665:0:0（657 零回归 + 8）；§3.2 六命令 clean
  起步全绿（build 12.87s/test 36.6s）；CLI 冒烟六路径 + 双审计
  EXIT 0（41+50）；对账六面；r22 tar.gz（303 条目）包内自举验证
  （全新构建 12.51s + 665 复跑 + CLI 一致）；web 同步（kerf-data +
  footer v6.2 + download README）+ git 双仓库 + 本 rec。

## 状态与索引

- 测试口径：**665:0:0**（单元 202 + 集成 463——净 +8 集成）
- I1 状态：**全臂 parity 就绪**（十臂编译段实现面完成——余 42-d
  生产切换 + 门 B fixpoint + eval 退役）
- 下一步：42-d I1 收口（CompilerKind 切换 + production_compiler_
  is_bootstrap 守护 + §21.3 条件 2 SHA-256 终验 + TD-017 终验 +
  T1 收口 + 缓存键分桶 B11）
- 检索锚：`compile-module-body` / `compile-module-seq` / `值等价` /
  `门 A 扩展组` / `preamble.krf 真实语料` / `边界不对称消除` /
  `表体对账补齐`
