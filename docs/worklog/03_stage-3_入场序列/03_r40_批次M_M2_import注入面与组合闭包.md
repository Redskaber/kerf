# 03_r40 — 批次 M 次件 M2：import 注入面/别名 + 组合闭包 + W 族（2026-09-15）

> 压缩比 1:8（flat 详录 = kerf/docs/worklog.md 61-a/61-b/61-z/61-web + root worklog 61-x）；会话 Task 61-x。
> 驱动源：用户三指令（「深审缺陷面 + 知识搜索补充修正」+「按 sop.md 继续推进任务」+「同步完整打包项目 tar.gz 并同步完整更新 web page」）。
> 结构：r40 = 61-a（深审轮）+ 61-b（M2 本体）+ 61-z（打包 + rec 树）+ 61-web（web 面）合轮；深审与 M2 同轮耦合（用户首位指令深审，M2 主题即语言设计实施面）。

## 交付零：61-a 深审轮（六维度缺陷面 + 四查询实证）

- **深审裁定 D1-D9**（22 §10 新增节）：D1 `as` contextual keyword（N4 零改动）/ D2 E0019 未知导入 / D3 E0018 位置纪律 / **D4 P1 缺口当场修复**（`IoRequirements::from_core` 递归入 Module body 把模块内 require 计入程序级授权 = 环境继承反面形态——WASI「无环境权威」批判对象；修正顶层口径）/ D5 B1-B3 限定名独享 / D6 W 独立值域 1000+ / D7 过渡期知悉项 / D8 编译期交集 + preamble 结构豁免 / D9 零配置维持
- 四查询实证（tool-results/r40-search/）：q1 langdev 导入语义 / q2 滚动弃用（HN + Kevin Cox）/ q3 WASI 无环境权威 + ocap / q4 Clojure refer 逃逸阀

## 交付一：61-b M2 本体（R-N5 + 组合闭包 + B1/B2 + W 族 + 五码）

- **import 注入面三形态**：`collect_import_face`（Stx 层前瞻 as 解析——as 形态不注入非限定名[误报 E0013 的开发实录：初版 push 后回溯，前瞻解析从源头分立]）+ `rewrite_alias_refs` 编译期归一（`str/append` → `string/append`——HM/编译/运行全链一致；CoreExpr 12 臂重建）+ registry stdlib 预 declare（七模块 visit 叶子；**prelude 除外**[preamble 自带 declare——重复冲突开发实录]）+ expander 双镜像（core_forms.rs + expander.krf as 对跳过——**自举语料无 cddr** 开发实录：`(cdr (cdr …))` 展开修复）
- **组合闭包**（21 §4.4）：`verify_capability_closure` E0006 增强形态（先于 R9 基础形态——顺序开发实录：初版置后导致增强形态被基础形态先报，对调修复）+ `module_requirements` 需求元数据 + **D4 修复**（from_core 顶层口径）
- **B1/B2 契约**：限定名独立分派体 miss→nil（旧名/扁平名 parity 不动——20 §4 迁移不变量）；**B3 纯测试锚**（底层 FS-4 已对齐——20 §4 行 R4 修正：过期注记）
- **W1001/W1002**：弃用 27 件每名去重 + 遮蔽 + preamble 结构性豁免（初版 span file_id 豁免失效——自举桥展开后 Span 传递不可依赖，改结构面豁免的开发实录）+ 三产物面 warnings + CLI stderr；**谓词反置开发实录**（BUILTIN_ALIASES = (现代名, 旧名)——初版 find 匹配第一元素，旧名匹配用第二元素）
- **五码**：E0013/E0016/E0017/E0018/E0019（stage 统一 Compile——front 管线验证族先例对齐）
- **存量语义演进修正七处**：D4 行为锚反转（capability 单测 + architecture_audit require 上移）+ E0019 接管（negative_expander + prelude）+ B1 断言（namespace_tests -1→nil）+ 审计集 4 case E0019 同步 + stage_e_code 族扩展（E0013-E0019 合法于 Compile）
- **§3.2 六命令全绿**：clean 2821/650.8MiB → build 14.08s → check 0/0 → fmt 0 → clippy 0 → **825:0:0**（806→825 净 +19：namespace_tests M2 组 19 + capability 断言反转）+ 四审计集 EXIT 0 ×4（190）+ CLI 四路径 + import/别名/组合闭包/W 端到端实证

## 交付二：61-z/61-web 打包与 web 面

- r40 tar.gz（§19.3 正序 git 85e0924：kerf-stage3-v0.7.0-r40-batchM-m2importface-825tests.tar.gz 2.12MB/349 条目）+ 包内自举（全新解包构建 15.01s + **825:0:0 复跑** + 四审计集 ×4 + CLI 一致[fib 144/别名 ab/check ok/负例 REAL_EXIT 1/组合闭包] + qbe 在包）；web 面 kerf-data r40 + footer v7.10 + download README r40 节 + PACKAGE_CONTENTS + E2E 双端 + lint + git 双仓

## 里程碑

- **批次 M M2 ✅**（import 面 + 组合闭包 + W 族 + 五码——825:0:0 净 +19）；**M3 = 全表对账收口 + 门审（§7.3 ≥30 case + E2E 全导入路径）→ 移除轮（与 E5 同窗，23 §2.2 驱动）**
