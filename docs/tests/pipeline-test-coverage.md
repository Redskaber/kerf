# 流水线路径覆盖（sop.md §9.5.1 三层覆盖记录 + §14.6.1.1 完整性审查）

> **Author**: Super Z（QA-A 角色）
> **Date**: 2026-09-16（**r43 对账（E5 S2 关键字腿 + 关键字完全重筛深审——64-a/64-b/64-e/64-z/64-web）：828 → 832 基线（净 +4——namespace_tests s2 组 4 case：E0021 关键字旧形四形态 + 接管豁免 + E0020 新名禁绑 + 门 A 行为 parity）；生产代码 = symbol.rs[Keyword::as_str/from_name 三件切换 fn/assign/do——变体名不变] + driver.rs[head 匹配同步 + REMOVED_KEYWORD_NAMES 3 对 + E0021 分源扩展 + W1002 消息] + core/expr.rs[render 表面跟随 + <lambda>→<fn> 魔法名] + core_forms/anf/vm 消息面 + 语料迁移 800+ 处[三域分离——表面关键字域切换/桥 tag 域零变更/core quote 符号域零变更]；深审 = 22 §12 D19-D33（25 名三判定 + 名空间经济判据 + set!→assign/begin→do 候选修正 + 否决记录八行 + 八查询实证）；§3.2 六命令全绿 + 五审计集 EXIT 0 ×5（233）+ CLI 四路径 + E0021 端到端 ×3；对账：22 v1.6/01/02 v6.4/20/18/e5-plan v1.1/RELEASE v0.10.0-r43/matrix v0.1.0-r43/23 E5 S2 行 ✅**）；837 → 828 基线（净 -9——移除轮测试面合法收缩）；生产代码 = builtins.rs[主注册 27 对现代名迁移 + REMOVED_BUILTIN_NAMES 退役表 + builtin_name_exists W1003 判定 + SIGS 旧条目删除 83→56 + STDLIB_MODULES 底层 4 名] + driver.rs[E0021 verify_qualified_refs 扩展 + W1003 collect_macro_names/collect_warnings + W1001 退役] + hm.rs[head/tail 结构重解释臂] + anf.rs[eq] + 语料迁移[.krf ×4 + 21 测试文件 + examples——grep 零旧名]；§3.2 六命令全绿 + 五审计集 EXIT 0 ×5（233）+ CLI 四路径 + E0021/W1003 端到端；对账：18 v6.7 E0021/W1003 落位 + 20 v1.5 / 22 v1.5 / 23 v1.5 / 09 v6.11 + TD-027 清偿关闭 + matrix v0.1.0-r42 + e5-removal-round-plan.md v1.0[新] + v0.5-roadmap v0.6.0**）；2026-09-15（**r39 对账（批次 M 首件 M1——60-a/60-z/60-web，命名空间层限定名可见面）：788 → 806 基线（净 +18——namespace_tests 新文件[tests/v0/stage3/plan/ Stage 3 层首件] 17 集成 + 1 单元守卫）；生产代码 = builtins.rs[STDLIB_MODULES 47 行 + 限定名注册（io fail-closed 分项）+ 签名派生] + capability.rs[gated_name io 前缀归一化——R9 对 io/* 同判] + driver.rs[verify_qualified_refs E0014/E0015 编译期 + Lambda 参数遮蔽 R-N1]；R-N4 词法零改动（`/` 已在 ID 域——双路径天然满足）；§3.2 六命令全绿 + 四审计集 EXIT 0 ×4（190 维持）+ CLI 四路径 + 限定名端到端实证；对账：18 v6.4 E0014/E0015 落位/20 v1.4/22 v1.2/12 v6.11/23 v1.2/09 v6.8 + v0.5-roadmap 批次 M M1 行 + matrix v0.1.0-r39**）；> 2026-09-15（**r38 对账（批次 L 别名层——59-a/59-z/59-web，Stage 3 入场序列首件）：758 → 788 基线（净 +30——批次 L 功能批）；集成 +28（stdlib_tests 27 parity case[别名 ≡ 旧名：正例双名同值 + 负例阶段/诊断码/消息体三面] + 闭合守卫 1[静态名单 ↔ BUILTIN_ALIASES 双向对账]）+ 单元 +2（kerf-driver builtins.rs 三方漂移守卫 ×2——注册 ⊇ 签名[逐项相等] + 门控 ⊆ 注册）；生产代码 = builtins.rs（BUILTIN_ALIASES 27 行静态表 + register_globals 同 Rc 双注册段 + BUILTIN_SIGS 27 别名条目 56→83——同分派体共享：新旧名同行为同诊断天然 parity，词法/语法零变更[02 §8.1 B1-1 v0.5 面澄清——全为既有词法形态的扁平符号]）；§3.2 六命令全绿 + 四审计集 EXIT 0 ×4（190 维持）+ CLI 四路径 + 别名端到端实证（E0006 fail-closed 维持）；对账六面：20 v1.3/12 v6.10/23 v1.1/09 v6.7/02 v6.2/07 v6.3 + v0.5-roadmap v0.3.0 + TD-027 进展注记 + matrix v0.1.0-r38**）；2026-09-15（**r37 对账（K3 收尾交付——58-c/58-a/58-z/58-web）：758 基线维持（零测试函数增量——零代码收尾轮）；§3.2 六命令 clean 起步全绿：build 13.15s/check 0/fmt 0/clippy -D warnings 超集 0/758:0:0 零断言修改 + 四审计集 EXIT 0 ×4（41+50+53+46）+ CLI 四路径（E0005 REAL_EXIT 1）；批次 K 全闭环（K1 APPROVED + K2 GO + K3 收尾）→ Stage 2 全收口：对账六面 + 12 §2.5.1 类型检查器行 K3 终态注记（默认期评估闭环——HM 旗标期维持 + R1-R8 回归基线断言 + 切换窗口归 23 §2.2 治理触发表 Stage 3 承载）+ v0.5-roadmap v0.2.0 Stage 2 行收口 + r37 tar.gz 包内自举 + web E2E + rec 树 21_r37/l**）；2026-09-15（**r36 对账（K2 大阶段末深审环——57-a）：758 基线维持（零测试函数增量）；§3.2 六命令 clean 起步全绿：build 14.48s/check 0/fmt 0/clippy 超集 0/758:0:0 零断言修改 + 四审计集 EXIT 0 ×4（41+50+53+46）+ CLI 四路径；§14.9 系统性代码整理：P2 注释时效 7 项 + P3 注释/臂注释 12 项修复（零行为变更——修复后 build 12.18s/fmt 0/clippy 0/758:0:0 复验全绿）；§4 完整性小节 r36 全量重测：catch-all 86 处分类判定全合规（含本轮 5 处臂注释补齐 + 2 处注释枚举补全）；性能基线三重采样 + TD-028 登记（fib 92.5ms / gc_tail 42.6ms——~10% 漂移诚实归因）；§14.8 设计回写四件（03 v6.3 深度 10_000 / 05 v6.3 根集六来源 + 九入口 / 02 v6.1 Keyword 25-叶级 48 / 13 v6.3 Effect 节态 r25 终态）**）；2026-09-14（**r35 对账：758 基线维持 + 第四审计集（K1 终门审——56-a 同步轮 + 56-b 批次 K 首件：stage2_gate_audit_r2 46 新 case APPROVED——静态判定面主轴[HM E0005 生产判定面——r1 零覆盖半区：A/B 桶 20 case] + 批次 J 修复边界 7[r28 效应双面检出 ×2/r29 TD-011 对偶 + Nil 归零 + 定位面/r30 FFI 编译面收窄 + VM 三码族 E0010/E0011/E0012] + §21.3 四条件终验[P03 fixpoint/P04 QBE fib 144/P05 FFI 真端到端 write_stdout Int(4)/P06 自编译活性] + §21.5 九信号核对[P07——机械可核 7 项进程内实测 + S8 投票协议承载 + S9 K2 承载]；零 cargo test 计数增量（审计集 examples/audit/ 可重运行口径另计）；§3.2 六命令 clean 起步全绿：build 13.11s/check 0/fmt 0/clippy 超集 0/758:0:0 零断言修改；四审计集 EXIT 0 ×4（41+50+53+46）；代码增量 = 审计器单件 46 case（零生产代码变更）；同步面：stage0.md v6.5（附录 A +33 术语/附录 E +14 引用/§23.1 计数修正/三指针注记）+ sop v12.4（§8.4.3 目录树 24 文件/§8.4.5 +4 查询行）**）；（**r34 对账：758 基线维持（批间插入轮第三弹——设计缺陷深度审计收敛轮 55-a/55-z/55-web，零测试增量；§3.2 六命令 clean 起步全绿（build 13.63s/check 0/fmt 0/clippy 超集 0/758:0:0）+ 三审计集 EXIT 0 ×3；零代码变更轮——产出全为设计治理面（23-演进治理 v1.0 + 21 v1.1 + 22 v1.1 + 原则 35）**）；2026-09-13（**r33 对账：758 基线维持（批间插入轮第二弹——能力架构深度设计轮 54-a/54-z/54-web，零测试增量；§3.2 六命令全绿（build 5.35s/check 0/fmt 0/clippy 超集 0/758:0:0）+ 三审计集 EXIT 0 ×3；唯一代码变更 = builtins.rs 头注 21/22 设计指针——注释级零语义）**；**r32 对账：758 基线维持（批间插入轮——表面现代化设计轮 53-a/53-z，零测试增量；§3.2 六命令 clean 起步全绿 + 三审计集 EXIT 0 ×3；唯一代码变更 = builtins.rs 注释对账 3→4 + 现代化指针——零语义变更）**；**r31 对账：758 基线维持（批次 J 收口——48-e J4 零代码收尾轮，零测试增量；§3.2 六命令 clean 起步复验全绿）**；**r30 对账：758 基线（批次 J 执行——48-d J3 FFI VM 面做实 +45：ffi_vm_tests 37 新集成文件（13 边界 case 全判定 + 正负比 7:22 功能点粒度 + Φ/P-U 机制组）+ driver ffi.rs 单元 8；Tier 2 头 503→543）**；**r29 对账：713 基线（批次 J 执行——48-c J2 HM 旗标期切换 +3：typecheck_tests 25→28 旗标期判定面组（增值面生产证据 + occurs 豁免政策锚 + 双面修复锚）+ 断言重锚 5 处 + 双缺口修复（hm.rs Ordering TD-011 同步 + car/cdr Nil 检出）——driver 两入口判定面 = hm_check_program，R1-R8 退为回归基线断言）**；r28 对账：710 基线（批次 J 执行启动——48-b J1 效应 typecheck 收敛 +4：effect_tests 17→21 静态收敛组双面检出；含 R4 修正：Tier 2 头计数 463→503——r23-r25 累计增量（+2/+14/+20）与 r26 补账声称的 499 口径均未落面，本轮回写实测值并注记）**；r26 对账：706 基线（批次 I 收口——门审环零 cargo 计数增量 + 滞后两轮补账 r24 +14 / r25 +22 + 新审计集第 3 件 stage2_gate_audit_r1 53 case）；r18 对账：634 基线（批次 H +29——TCO 12 + HM PoC 15 + Probe 拆分 2）+ Stage 2 两套件行 + 双门注记；r16 全量重写——批次 F 深审 D8 载体更新：Stage 0 r3 版停在 294 口径，本轮对账至 553 + Stage 1 套件 + parity 印证节 + 性能基线同步节；r3 版：负测扩张后全文重写）
> **Version**: v0.8.0-r43（E5 S2 关键字腿——Tier 2 头 610→614[s2 组 +4]；v0.7.0：r42）
> （遗留头：v0.7.0-r42（E5 窗开启 + 移除轮 S1——Tier 2 头 619→610[移除轮测试面合法收缩：837→828——28 parity case 收敛为 17 退役负例 + 2 W1001 case 演进为 4 W1003/E0021 case]；v0.6.0：r41）
> **Status**: Active（**r26 对账（滞后两轮补账——深审 D7/W1 项）：706 基线**——①r24 增量（+14 集成：stdlib_tests 17→24（字符串全序 12/负 3 + 装箱往返正 10/负 2 + 谓词 17/环安全 2/负 3）+ gc_tests 6→9（TD-023 GcCell 写路径）+ prelude_tests 7→10（foldr）+ scope_set_tests 9→10（TD-018 双路径同文对拍））；②r25 增量（+20 集成：**effect_tests 17 新建**（设计锚正 6 + M3 双路径 8 + eval 域 3 + 负例 7 + GC 存活 2）+ bootstrap_compiler_tests 29→32（门 A 效应组 3）+ 单元 +2（capability M2 两锚））；③**r26 门审环（零 cargo 计数增量）**：stage2_gate_audit_r1 新审计集 53 case（examples/audit/ 可重运行口径）EXIT 0 + hm.rs 2 处 catch-all 注释补齐（§4 小节同步）+ 深审报告 deep-review-round1.md（D1-D8 + 偏差清单 + 五角色全票 GO）；r23 对账：670 基线（批次 I 执行 42-d +5——bootstrap_compiler_tests 27→29：门 B fixpoint（B₁/B₂ 四程序 bytecode_equal + SHA-256——§21.3 条件 2 终验）+ B₁ 产物可执行面；单元 +3：driver 64→67 生产编译守护 + 缓存 CompilerKind 分桶 ×2（B11/P4）；T1 双路径面全量迁移（eval → 种子链对拍——十测试文件 + common + 双审计集；表体 driver 单元行 58→67 实测修正）；r22 对账：665 基线（批次 I 执行 42-c +8）；r21 对账：657 基线（42-b +19）；r20 对账：638 零增量（设计轮）；r19 对账：638（+4 Probe））

## 1. 测试目标

记录编译流水线（read → expand → lower → compile → vm → runtime → driver）的**路径覆盖状态**，
作为外循环投票（§6.3）的数据源。计数基线：**665 测试函数 / 665 通过 / 0 失败 / 0 忽略**
（[matrix.md](./matrix.md) r22 口径——单元 202 + 集成 463）；负向 case 口径见 §2 统计行。

## 2. 三层覆盖记录（§9.5.1 格式：Tier / 名称 / 覆盖阶段 / 预期输出 / 状态）

### Tier 1 —— 阶段内（单元测试，205 函数，crates 内联——r9 起 runner 化仅集成侧；r17 +13：backend 9 + expander recover 4；r18 +2：reserved Probe 拆分；r19 +4：capability_model Probe（driver 60→64）；**r23 +3：生产编译守护 + 缓存分桶 ×2（driver 64→67）**）

| ID | 名称 | 覆盖阶段 | 预期输出 | 状态 |
|----|------|---------|---------|------|
| T1-span | kerf-span 单测 | Span/诊断 | Span 派生/诊断渲染契约 | ✅ 11/11 |
| T1-syntax | kerf-syntax 单测 | 符号/作用域 | NFC 归一化/ScopeSet 运算/Stx 代次 | ✅ 11/11 |
| T1-core | kerf-core 单测 | CoreExpr/图 IR/CodeValue | 9+1 原语构造/共享/良构性（含 Symbol 键） | ✅ 10/10 |
| T1-reader | kerf-reader 单测 | 词法/语法 | Token 快照/括号三态 | ✅ 23/23 |
| T1-expander | kerf-expander 单测 | 展开/相位 | 核心形式/卫生/簿记/循环依赖/**trampoline 深度 500** | ✅ 28/28 |
| T1-compiler | kerf-compiler 单测 | 编译/类型检查 | 栈平衡/回填/常量池/**40 操作码守护**/App 函数先 | ✅ 15/15 |
| T1-runtime | kerf-runtime 单测 | 堆/GC | 分配/回收/foreign 根/**alloc_symbol** | ✅ 9/9 |
| T1-vm | kerf-vm 单测 | 执行 | 帧协议/错误形状/**Value 十变体** | ✅ 18/18 |
| T1-driver | kerf-driver 单测 | 管线编排/能力/效应/缓存/自举桥 | 全管线/**52 内置注册**/预留冻结（Probe——**r19 增 capability_model 骨架 + 族归属证明**）/effects 12/capability 13/**生产切换守护（活性探针 + 代次双信号）** | ✅ **64/64**（r8 +25 / r12 +8 Probe / r15 +1 切换守护 / r18 +2 拆分 / r19 +4 模型骨架） |

### Tier 2 —— 阶段间（集成测试，614 函数（**r43 对账：610 → 614（+4）——E5 S2 关键字腿：namespace_tests s2 组（E0021 关键字旧形 + 接管豁免 + E0020 新名 + 门 A 行为 parity）**；**r42 对账：619 → 610（-9）——移除轮 S1：stdlib_tests parity 组 28→17 退役负例组 + namespace_tests W1001 组 2→4 W1003/E0021 组（移除轮的测试面合法收缩——E0021 负例组 27 名全覆盖为新增承载）**；**r41 对账：607 → 619（+12）——批次 M M3 深审 D10 namespace_tests d10 组（E0020 保留字绑定禁令 12 case：正例 3 + 负例 8 + 值位 fail-closed 1）**；**r40 对账：588 → 607（+19）——批次 M M2 namespace_tests 扩展组（R-N5 import 三形态 + E0013/E0016-E0019 五码 + 组合闭包 E0006 增强 + B1/B2/B3 契约 + W1001/W1002 + R-N1 全序 + 红线 1 + 别名归一 + 双路径）**；**r39 对账：571 → 588（+17）——批次 M M1 namespace_tests 新文件（七模块限定名正路 + R-N3 E0014 + E0015 保留域 + io 双门 E0006 + R-N4/N7/N8 锚 + 遮蔽/接管豁免 + 静态面 + 双路径）**；**r38 对账：543 → 571（+28）——批次 L 别名层 parity 组**；**r31 对账：零增量维持——批次 J 收口零代码轮（48-e/52-a）；r30 实测复验 543 维持**；**r30 对账**：506 + r30 +37——ffi_vm_tests 新文件（FFI 窗口规程/令牌状态机/E0010-E0012/13 边界 case——48-d/51-a）；**r29 对账**：503 + r29 +3——typecheck_tests 25→28 旗标期判定面组（HM 判定面生产证据 + occurs 豁免政策锚 + 双面修复锚；断言重锚 5 处子串为 HM 渲染措辞——语义/检出/定位不变）；**r28 R4 修正**：r26 补账声称「463→499」未落面——本行实际停在 r22 末 463；r28 回写实测 503 = 463 + r23 +2 + r24 +14 + r25 +20 + r28 +4，深审 D8 路径行同步补齐），tests/runner.rs 单一总入口 mod 树——r17 +40：qbe_backend_tests 24 + multi_error_recovery_tests 16；r18 +27：tco_tests 12 + hm_inference_tests 15；r19 零变更（骨架冻结零集成接触）；r21 +19：bootstrap_compiler_tests 门 A 基础组 parity（I1 前段自举 Compiler——三件套第三实例：bytecode_equal 全结构判据 + 行为面端到端）；**r22 +8：42-c 扩展组（module/require 两臂 + 糖九件全管线 + 宏 + prelude 注入序 + examples 六件双路径——边界负例改写正例）**；r23 +2 / r24 +14 / r25 +20（效应面新建 + 门 A 效应组）；**r28 +4：48-b 效应静态收敛组（typecheck.rs/hm.rs Perform/Handle 子表达式遍历——双面检出 + 零误报对照）**；**r29 +3：48-c HM 旗标期判定面组（D8 阶段 2——driver check 两入口判定面切换的测试锚）**）

| ID | 名称 | 覆盖阶段 | 预期输出 | 状态 |
|----|------|---------|---------|------|
| T2-reader | reader_tests | read | Token 流无损 + 错误带 Span | ✅ 10/10 |
| T2-neg-reader | negative_reader_tests | read（负） | E0001 全错误族 + Span 精确断言 + 渲染形状 | ✅ 14 函数（59 case） |
| T2-expander | expander_tests | read→expand | 展开产物/糖推导/卫生 | ✅ 18/18 |
| T2-neg-expander | negative_expander_tests | expand（负） | E0002 全族 + 空应用 + 关键字误用矩阵 + 循环依赖 | ✅ 23 函数（96 case） |
| T2-compiler | compiler_tests | expand→compile | 确定性编译/字节码形状 | ✅ 10/10 |
| T2-vm | vm_tests | compile→vm | 执行语义 + 双路径互查（App 顺序回归） | ✅ 20/20 |
| T2-neg-vm | negative_vm_tests | vm（负） | 52 内置 × 类型/元数系统表 + E0004 断言 + 符号值误用 | ✅ 30 函数（237 case） |
| T2-neg-sem | negative_semantics_tests | 全阶段语义（负） | E1-E6 矩阵 + T1 回归 + 消息形状 + 错误恢复 | ✅ 23 函数（98 case） |
| T2-gc | gc_tests | runtime | 10^6 有界分配/环/深链/foreign + **r24 +3（装箱闭包捕获存活/捕获链传递/GcCell 写路径 sound——TD-023）** | ✅ 9/9（r24） |
| T2-pipeline | pipeline_tests | read→vm 全链 | E2E/双路径/确定性/Span 传播 | ✅ 11/11 |
| T2-arch（r10） | architecture_audit_tests | 架构形态 | 十变体穷尽 match 证明/Span 独立/Reader-Stx 隔离/Expander 唯一桥/同源同核 | ✅ 4/4 |
| T2-gate0 | gate_review_r1 | Stage 0 §22.3 十二里程碑 | 12 项可执行审计 | ✅ 8/8 |
| T2-stdlib（r5-r8，r24 扩） | stdlib_tests | driver→内置 | 52 内置行为 + TD-016 链式比较 + 门控前缀 + **r24 +7（TD-011 全序正 12/负 3 + TD-010 装箱往返正 10/负 2 + 谓词 17/环安全 2/负 3）** | ✅ 24/24（r24） |
| T2-boot-reader（r6） | bootstrap_reader_tests | read（自举） | reader.krf 与种子逐字节 parity | ✅ **28/28** |
| T2-worklist（r4） | expansion_worklist_tests | expand（迭代） | trampoline 500 层/兄弟不累计 | ✅ 4/4 |
| T2-typecheck（r7） | typecheck_tests | compile→静态检查 | R1-R8 + E0005 多错误 + Span 次序 | ✅ 24/24 |
| T2-cache（r7） | cache_tests | driver→缓存 | 内容寻址/命中观测/失效 | ✅ 14/14 |
| T2-capability（r8） | capability_tests | driver→能力门控 | require 声明面 + E0006 三路径 + 令牌 + EOF 探针 | ✅ 24/24 |
| T2-testrunner（r8） | test_runner_tests | driver→用例运行器 | 前置切分/PASS 判定/短路恢复隔离 | ✅ 18/18 |
| T2-reserved-ext（r12） | reserved_ext_tests | reserved 14 项 | P0 位置断言 + API 可达 + 负向形状 | ✅ 12/12 |
| T2-scope-set（r13，r24 扩） | scope_set_tests | expand→compile/eval（作用域） | (name, scopes ⊆) 双路径 + 不匹配负例 + **r24 +1（TD-018 if 消息双路径同文对拍回归）** | ✅ 10/10（r24） |
| T2-boot-exp（r14/r15） | bootstrap_expander_tests | expand（自举） | **expander.krf 与种子逐字节 parity：结构/Span/作用域集/param_scopes/错误消息逐字 + 宏 17（E1-β 全模式面）** | ✅ **36/36** |
| T2-prelude（r15，r24 扩） | prelude_tests | 模块/导入 | hofs 用户面/组合管道/双路径/opt-in/显式失败 + **r24 +3（foldr 用户面/对偶语义/双路径）** | ✅ 10/10（r24） |
| Stage 2 批次 G | qbe_backend_tests（tests/v0/stage2/plan/） | 端到端 6（fib 144 本地码 = VM 一致 / 算术 / phi 合并 / begin / not·eq / 嵌套 if）+ 结构 4（IL 断言 / 契约门 / require 跳过 / 空程序）+ 一致性 6 + **负例 10（PoC 边界：lambda 值位 / define 非 lambda / Str / Float / set! / module / 未定义 / arity / 自由变量 / IO / 函数值）** + 契约 2 | r17 | ✅ 24/24 |
| Stage 2 批次 G | multi_error_recovery_tests（tests/v0/stage2/plan/） | 种子恢复 6（跳过/全报/位置序/上限截断/收集器单元/干净）+ driver 8（E0002+E0005 合并 / 部分产物 / 渲染 / read 短路 / E0006 短路 / 短路 API 不变 / 既有 typecheck 多错延续 / 截断尾注）+ 双路径同构 1 + 执行路径不变 1 | r17 | ✅ 16/16 |
| Stage 2 批次 H | tco_tests（tests/v0/stage2/plan/） | TCO 正例 8（105_001 尾递归恒定帧 / 相互尾递归 30 万步 / if 两臂尾位传播 / begin 末项三层 / let 糖 / 内建隐式 RET / 闭包值尾位 / 自举 10_000 深度链端到端）+ **负例 4（指令预算护栏（预算注入）/ 尾调用 arity / 非可调用 / 非尾深递归仍帧上限）** | r18 | ✅ 12/12 |
| Stage 2 批次 H | hm_inference_tests（tests/v0/stage2/plan/） | **双门**：超集门 29 程序（R1-R8 检出 → HM 亦检出——双检查器并行对照）+ 零误报门（examples 六件套 + 动态边界 15 case）；**四类缺口检出**（lambda 实参 / car 元素 / 分支分歧 / 递归元数域）+ occurs ≥3 + 值限制 ≥2 + let/letrec 泛化 + 多错误 Span 序 + Dynamic 逃生舱 + 512 预算 | r18 | ✅ 15/15 |
| Stage 2 批次 I（r21 基础组） | bootstrap_compiler_tests（tests/v0/stage2/plan/） | **门 A parity**：bytecode_equal 全结构（含 debug_spans）13——字面量/常量池去重/引号点对/回填/begin 尾位/嵌套捕获三链/遮蔽/set! 三路径/尾位穿线/深嵌套 100 层/双跑确定性/空程序 + 负例 2（define 位置 D1 逐字）+ 行为面 4（fib/closures/higher_order/10 万层深尾递归） | r21 | ✅ 19/19 | 
| Stage 2 批次 I（r22 扩展组） | bootstrap_compiler_tests 同文件 | **门 A 扩展组 46 case**：module/require 两臂正例（多项 Pop + 空体 + 尾位非继承 + 确定性双跑）/ 糖九件全管线（let 家族 8 + cond/when/unless 9 + and/or/while 10）/ 宏 3（swap!/my-or/def-twice）/ prelude 注入序 2（preamble.krf 真实语料）/ **examples 全六件双路径 bytecode_equal** + 行为面 2（糖九件 + module 臂——VM 执行 = 生产管线） | r22 | ✅ 27/27（19→27 合计） |
| Stage 2 批次 I（r23 收口组） | bootstrap_compiler_tests 同文件 | **门 B fixpoint（§21.3 条件 2 终验）**：B₁/B₂ 四程序 bytecode_equal + SHA-256 摘要 + B₁ 产物可执行面（install 后自举链运转 = 种子链结果） | r23 | ✅ 29/29（27→29 合计） |
| Stage 2 批次 I（r25 效应组） | bootstrap_compiler_tests 同文件 | **门 A 效应组 3 case**：perform/handle 基础 parity + trampoline 共享/捕获面 parity + resume 往返行为面 | r25 | ✅ 32/32（29→32 合计） |
| Stage 2 批次 I（r25 效应面）+ 批次 J（r28 静态收敛） | **effect_tests（tests/v0/stage2/plan/，新建）** | **效应语言面验收 21 函数（r25 17 + r28 +4）**：设计锚正 6（单 handler 单恢复/嵌套逃逸/纯体零效应/恢复后环境一致性/效应值先求值序/TCO 10 万深穿透 handler 帧）+ M3 双路径 8 + eval 域 dispatch 一致 3 + 负例 7（E0007/E0008 含首恢位置/E0009/非 continuation 值/handler 形态×2/perform 非点对/非符号 tag）+ GC 存活 2（M5 第六来源——5 万分配压力 + 装箱解箱）+ **r28 静态收敛组 4（48-b——typecheck.rs/hm.rs Perform/Handle 臂子表达式遍历）：Perform 效应值违例 2（R2 算术混串 + R1 if 非真值）+ handler 体违例 2（R5 car/cdr 非 pair）+ handle 体违例与双体多错误收集 1（R1 + Span 序合并）+ 零误报对照 1（六正例 + 绑定器动态用点 2 + effect_stress.krf 语料——双面（check_program + hm_check_program）超集纪律）** | r25 / r28 | ✅ 21/21 |

### Tier 3 —— 全流程（阶段门审计 + parity 双实现印证）

| ID | 名称 | 覆盖阶段 | 预期输出 | 状态 |
|----|------|---------|---------|------|
| T3-gate0 | examples/audit/stage0_gate_audit_r1.rs | §7.3.1 门审计（Stage 0） | 41 case（负 32 + 恢复 6 + 正 3）；§7.1.1 七类全覆盖 | ✅ EXIT 0 |
| T3-gate1 | examples/audit/stage1_gate_audit_r1.rs | §7.3.1 + §21.3 四条件（Stage 1） | **50 case**（A 12 / B 12 / C 10 / D 恢复 6 / E 边界 6 / P 4）+ 七类全覆盖 + 边界 6——**全部经生产管线（自举读+展开）** | ✅ EXIT 0（34-d APPROVED） |
| T3-parity | bootstrap_{reader,expander}_tests（28+36） | **双实现逐字节互证** | 自举 kerf 实现 ↔ Rust 种子：结构/Span/作用域/代次/错误消息逐字一致 | ✅ 64 测试（Stage 1 特有最强印证形态） |

### 正负比例统计（§9.5.1 要求的统计行）

- **函数口径**：638 测试函数（r15 权威拆分口径 553 = 正向/混合 436 + 负向 117；r17-r19 净增 +85——主体为正向 parity/后端/推断/Probe 冻结，负向分项未重拆）。
- **case 口径（权威）**：负向 case 490（四文件 59+96+237+98）+ 审计集负向 32+18 + 各集成套件内负例 ≈ 26；
  正向 case ≈ 156 → **全局正负比 ≈ 1:3.15**（r15 门审权威口径；§9.4.3 的 ≥1:3 门限达标 ✓）。
- 逐分类负测非零：read/expand/compile（间接）/vm/driver/gate/capability（E0006）/
  typecheck（E0005）/prelude（opt-in 负例）/模块（循环依赖）全覆盖；runtime（GC）与
  reserved 为不可构造类（§3 注记）。

## 3. 编译流水线路径覆盖矩阵（16 类 + Stage 1 增量 4 行——真实状态）

> 沿用 r3 版诚实口径：正向全覆盖；负向以「用户程序可构造」为边界，不可构造类以
> 不变式断言 + 文档化存档为验证口径（下表 5 类）。Stage 1 新增能力面补 4 行。

| 路径 | 正向 | 负向 | 状态与验证载体 |
|------|------|------|----------------|
| read：词法/语法正确/错误 | ✅ reader_tests | ✅ negative_reader（59 case + E0001 直接断言） | 正负双覆盖 |
| expand：9 核心形式/糖/宏卫生/错误路径 | ✅ expander + bootstrap parity | ✅ negative_expander（96 case）+ negative_semantics 卫生回归 | 正负双覆盖 |
| **expand：宏深度（自指链）** | ✅ worklist（500 层） | ✅ 深度超限结构化报错（双实现消息逐字 parity） | 正负双覆盖（r14/r15 增） |
| lower：图 IR | ✅ kerf-core 单测（共享/Span/roots） | ⚠️ 不可构造 | 编译器内部构造；不变式断言存档 |
| compile：回填/常量池 | ✅ backpatch 零占位 + dedup | ⚠️ 不可构造 | E0003 内部不变式存档 |
| compile：闭包捕获 | ✅ 捕获描述符/嵌套 | ✅ 间接（lambda_arity + E2 运行时侧） | 双覆盖（运行时侧） |
| **compile：类型检查（静态负例）** | ✅ typecheck（R1-R8 正例） | ✅ E0005 族 + 多错误收集 + Span 次序 | 正负双覆盖（r7 增） |
| vm：执行语义 | ✅ fib(25)/深递归/闭包/高阶 | — | 正向即可证 |
| vm：错误路径 | ✅ — | ✅ negative_vm（237 case：52 内置系统表/div-zero/not-callable/帧上限/E0004） | 正负双覆盖 |
| runtime：GC | ✅ gc_tests（10^6 有界） | ⚠️ 语义级不可构造 | L-GC 不可观测；压力 + 双路径互查为口径 |
| driver：管线 | ✅ E2E/双路径/确定性/诊断渲染 | ✅ negative_* 全经 run_source/eval_source 驱动 | 正负双覆盖 |
| **driver：能力门控** | ✅ capability_tests（声明/授权/豁免） | ✅ E0006 三路径 + EOF 探针 | 正负双覆盖（r8 增） |
| **driver：模块/导入（prelude）** | ✅ prelude_tests（组合管道） | ✅ opt-in 未声明未绑定/同名重复定义/未知导入/循环依赖 DFS | 正负双覆盖（r15 增） |
| reserved：冻结（14 项） | ✅ Probe 实现 + 位置断言 | ⚠️ 不可构造 | 「测试实现体编译通过 = 契约冻结」+ P0 位置跨 crate 断言（r12） |
| **自举切换（生产）** | ✅ 切换守护（活性 + 代次双信号）+ 553 全套件经生产管线 | ✅ parity 负例（错误消息逐字） | 正负双覆盖（r15 增） |
| 效应逃逸（内部） | ✅ effects 12 单测（穿透契约/最近匹配） | ✅ 未捕获效应报错形态 | 正负双覆盖（r8 增） |

## 4. 完整性审查小节（§14.6.1.1）

> **r36 全量重测（K2/57-a——§14.6.1.1 阶段间深验证完整口径，基线日 2026-09-15）**：方法 = 全库
> grep `_ =>`（含带值臂——较 r16/r26 口径扩大到全 catch-all 面）+ 逐处上下文核对 + cfg(test)
> 分区计数 + enum 穷尽性抽查。**r16 基线 1 处 → r26 实测 4 处空臂全注释合规 → r36 全口径 86 处
> （生产 83 + 测试 3）分类判定全合规**。

- **catch-all（`_ =>` 通配/兜底臂）全口径：86 处 / 24 文件（生产 83）**，六分类判定：
  ①「_ 臂理由」显式注释 24 处（项目约定固化——core_forms/vm/token/parser/builtins/heap/
  driver/typecheck/hm/opcode/expr/anf）；②显式错误臂（响亮非静默）~30 处（VmError/
  RuntimeError/CompileError/ReadError 族）；③`unreachable!` 带窄化说明 4 处；④空
  `_ => {}` 5 处（全带注释——hm ×2 / vm ×2 / anf ×1）；⑤Option 契约负查询 `_ => None`
  ~20 处（doc 自述「否则 None」）；⑥数值塔/谓词语义臂 ~7 处（节级注释在位）。
  **本轮修复 7 处（§14.6.3 第二轮发现即修——零行为变更）**：无臂注释 5 处补齐（anf.rs:195
  prim_arity 默认二元 / hm.rs:688 扫描窗口终止 / typecheck.rs:420 防御回退 / macro_sys.rs:191
  构造性默认 / vm.rs:1591 NaN 保底）+ 注释枚举不全 2 处补全（vm.rs:1392 + Symbol /
  heap.rs:349 + Symbol）。**静默无注释臂（修复后）：0 处。**
- **显式变体空臂：14 处**（`BcConst::Nil => {}`（hash 无贡献）/ `TcType::Unknown => {}`
  （无约束）等）——语义自明 no-op，显式臂在新变体加入时由穷尽性检查强制人工触达（非
  §14.6.1.1 风险口径）。
- **生产区 `expect`：23 处（全量重测——r26 口径 4 处为抽样，本轮全口径扫描）**——全部为
  带不变式消息的 `.expect`（vm 栈深守卫 ×8 / phase ×3 / expander ×1 / core_forms ×3 /
  sugar ×1 / macro_sys ×1 / driver ffi E8 口径 ×1 / bootstrap 惰性初始化 ×2 / hash SHA-256
  长度不变式 ×1 等）；**生产代码零裸 `.unwrap()`**；`src/main.rs` 零 unwrap/expect/panic
  （exit code 错误处理）；测试区 131 处不在约束口径。dead_code allow 1 处（eval.rs:114——
  TD-009/TD-017 存档面登记自证）。
- **enum 穷尽性抽查（4 核心 enum）**：`Op` 46 变体——VM 主分派 match **穷尽无 `_`**（新
  变体编译器强制双面更新——结构性安全）+ `Op::name()` 穷尽；`Value` 12 变体——type_name/
  box_value/render_value 穷尽；`HeapObj` 8 变体——Debug/PartialEq 穷尽 + GC children 显式
  出边 + 注释叶子臂；`CoreExpr` 12 变体——compile_expr/eval_expr/anf lowering 三面穷尽
  （anf 7 边界显式 LowerError 拒绝）。「match 遗漏 variant 且无 `_` 兜底」经 Rust 穷尽性
  检查在编译期不可达——无违规证据。
- **TODO / FIXME / HACK / XXX：0**（r36 全库复核——大小写敏感扫描零匹配）。
- **`//!` 模块头：66/66**（r16 C4 口径维持；本轮 §14.9 C3/C4 修复 4 处头部正文时效
  ——vm 值模型 / compiler 操作码计数 / driver 双路径口径 / expander TD-004 时态——头
  首行描述全准确）。

## 5. parity 双实现印证（Stage 1 特有——Tier 3 最强印证形态）

**设计流 = 编译管道流的双向锚定**：自举实现（reader.krf 471 行 / expander.krf ~1,180 行，
VM 上运行）与 Rust 种子（parity oracle）在 64 个测试上**逐字节比对**——结构 / Span /
作用域集 / param_scopes / 展开代次 / 错误消息（含宏深度超限消息逐字）。这比「测试
通过」更强的印证：**两个独立实现产出完全相同的中间表示**，则设计文档描述的管道语义
被双源锁定（单实现的测试只能证明「自洽」，parity 证明「符合规范」）。切换守护
（生产切换后全套件 553 经自举管线执行）进一步证明 parity 对象就是生产对象。

## 6. 性能基线同步小节（§14.6.4 执行协议 5）

| 指标 | 最新实测 | 状态 |
|------|---------|------|
| fib(25) bench 每轮 | **92.3-92.8ms（r36/K2 三重采样，中位 ~92.5ms**；r24 = 84.2ms → +9.9% 漂移——TD-028 登记诚实归因：r25/r30 五新操作码入主分派 + GC 六来源入口候选；正确性零影响） | ⚠️ TD-028（Stage 3 优化窗口） |
| gc_stress（尾形）每轮 | **42.2-42.9ms（r36/K2）**（r24 = 38.57ms → +9.5% 同向漂移——TD-028 同登记） | ⚠️ TD-028 |
| gc_stress_nontail（非尾形）每轮 | **91.9-116.9ms（r36/K2，方差带 ±13%）**（r24 = 105.15ms——方差域重叠，噪声带口径） | ✅ 噪声域 |
| 冷启动（自举装载） | 16.1ms（r24 口径维持） | ✅ |
| clean 构建 / 二进制 | **14.48s / 1.92 MiB（r36/K2）**（二进制 1.48 MiB → 1.92 MiB：r25 效应 + r30 FFI 代码体——预期增长） | ✅ 与 LOC 同步 |
| 全量测试套件 wall | **54.5s（r36/K2 实测，758 case）** | ✅ 新口径 |

> 完整口径、复测协议与探针方法见 [stage-1/performance-baseline.md](../develop/v0/stage-1/performance-baseline.md)（§9 复测记录滚动维护）+ **[stage-2/performance-baseline.md](../develop/v0/stage-2/performance-baseline.md)（r36/K2 新建——大阶段末 §14.6.4 基线锚）。**

## 7. 依赖

- 上游：[matrix.md](./matrix.md)（分套件计数权威——r16 对账：表体 bootstrap_expander_tests 行 19→36 + prelude_tests 行补录由 36-e 收口）、[negative-tests.md](./v0/stage0/plan/negative-tests.md)（负测文档锚点）
- 代码：tests/v0/{stage0,stage1}/{plan,gate} 双向印证；examples/audit/ 双审计集（41+50 case）
- 规范：[06-操作语义](../lang-design/06-operational-semantics.md)（E 码/T1）、[11-测试基础设施](../lang-design/11-testing.md)、[07-自举策略](../lang-design/07-bootstrap-strategy.md)（parity 口径）
