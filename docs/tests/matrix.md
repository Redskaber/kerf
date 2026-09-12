# 全局测试矩阵（覆盖率追踪）

> **Author**: kerf-dev-agent（QA-A 角色）
> **Date**: 2026-09-15（**r36 对账：K2 大阶段末深审环全交付（57-a/57-z/57-web——用户指令「按 sop.md 继续推进任务」+「同步完整打包 tar.gz 并同步完整更新 web page」）：**758 → 758（零测试函数增量——注释级修复轮；§3.2 六命令 clean 起步全绿：build 14.48s/check 0/0/fmt 0/clippy 超集 0/758:0:0 零断言修改 + 修复后复验 build 12.18s/758:0:0）+ 四审计集 EXIT 0 ×4（41+50+53+46）+ CLI 四路径（含 E0005 REAL_EXIT 1）**——§14.9 系统性代码整理：P2 注释时效 7 项 + P3 12 项修复（零行为变更——758:0:0 复验实证）+ §14.8 设计回写四件（03 v6.3 深度 10_000 / 05 v6.3 根集六来源 + 九入口 / 02 v6.1 Keyword 25-叶级 48 + §8.1 同核验证改判 / 13 v6.3 Effect r25 终态 + 12 宏行同步）+ §14.6 六件套深验证文档（deep-review-round2 D1-D8 + 投票 100% GO / architecture / design-impl-test / hidden-problems / refactoring-optimality / performance-baseline 新建 + final-assessment **Stage 3 切换 GO** 裁定）+ TD-028 登记（性能漂移 ~10% 三重采样诚实归因）+ 总量行同步债修正（三件 144 → 四件 190——r35 遗留）；本行 + RELEASE_NOTES r36 节 + pipeline v0.4.0-r36（§4 完整性小节 r36 全量重测 + §6 性能基线）+ plan K2 执行注记**）；2026-09-15（**r35 对账：K1 终门审交付（56-a 同步轮 + 56-b 批次 K 首件）——新审计集第 4 件 stage2_gate_audit_r2 46 case APPROVED（静态判定面主轴：A/B 桶 20 静态 E0005 + 批次 J 修复边界 7[r28 双面检出 ×2/r29 TD-011 对偶 + Nil 归零 + 定位面/r30 FFI 编译面收窄 + VM 三码族] + §21.3 四条件终验[P03 fixpoint/P04 QBE fib 144/P05 FFI 真端到端 write_stdout Int(4)/P06 自编译活性] + §21.5 九信号核对[P07]；§3.2 六命令 clean 起步全绿：build 13.11s 零告警/check 0/0/fmt 0（审计器 fmt 后修正）/clippy 超集 0/758:0:0 零断言修改；三审计集 EXIT 0 ×3 维持；stage0.md v6.5（附录 A +33 术语/附录 E +14 引用/§23.1 计数修正/三指针注记）+ sop v12.4（§8.4.3 目录树 24 文件/§8.4.5 +4 查询行/存档纪律吸收）**）；2026-09-10（**r10 架构合规审计**：+4 architecture_audit_tests（sop §2.2 原则 29-31 形态审计——lang-design 01 §8.5/02 §8.1 锚点落地：十变体穷尽 match 冻结证明/Span 独立/Reader-Stx 类型隔离/Expander 唯一桥/同源同核确定性）——476 → 480；r9 测试入口架构重构：Cargo.toml [[test]] 18 块 → tests/runner.rs 单一总入口 mod 树，sop.md §8.4.6 v11.2）
> **r12 增量**（2026-09-11）：+20 接口预留扩展测试（单元 8：reserved/ Probe 冻结——toolchain 3 + ffi 3 + codegen 2；集成 12：reserved_ext_tests——P0 位置跨 crate 断言 + 14 项 API 可达性 + 形状行为含负向）——480 → **500**。
> **r22 增量**（2026-09-11，批次 I 执行 / 42-c I1 中段糖与 module/require 面）：**657 → 665（净 +8，集成侧）**——bootstrap_compiler_tests 19 → 27：边界负例 1 改写为正例 parity（module/require 两臂迁移落地——边界不对称消除）+ 新增 9 函数（module/require 正例组 + 确定性双跑 / 糖三组：let 家族 8 + cond/when/unless 9 + and/or/while 10 / 宏语料 3 / prelude 注入序 2（preamble.krf 真实语料）/ examples 全六件双路径 / 行为面 +2 组：糖九件 + module 臂）；case 口径扩展组 46 ≥ 12 超额。口径：单元 202 不变 + 集成 455 → 463。
> **r21 增量**（2026-09-11，批次 I 执行 / 42-b I1 前段基础核心形式 kerf 化）：**638 → 657（净 +19）**——集成 +19（bootstrap_compiler_tests：门 A parity 13——字面量/常量池去重/引号点对递归/全局与 define/if 跳转回填/begin 尾位继承/lambda 嵌套捕获三链+形参遮蔽/set! 三路径/尾位穿线含相互尾递归与 let 脱装/深嵌套 100 层/确定性双跑与空程序 + 负例 2（define 位置 D1 消息+Span 逐字；module/require 42-c 边界显式断言）+ 行为面 4（fib 144/closures 计数器 (4 2)/higher_order map 平方/10 万层深尾递归——自举编译段产物 VM 执行 = 生产管线结果））；**opcode_count 冻结测试修正（40→41）**：TailCall（r18 TCO 引入）漏列于守护枚举——R4（代码为准 + 本次修正文档）三方同步（opcode.rs 测试 + 04-bytecode-vm 三处）。口径：单元 202 不变 + 集成 436 → 455。
> **r20 增量**（2026-09-11，批次 I 执行启动 / 42-a I1 切口评估与迁移设计）：**638 → 638（零测试增量——纯设计轮）**：i1-incision-migration-design.md（切口裁定 INC1-INC8 + 段序 S1-S3 + parity 三门 A/B/C + CompilerKind 切换点 P1-P5 + 确定性纪律）；plan.md §5a 42-b/c/d 行引用该设计为验收合同。638 零回归全绿复跑（§3.2 clean 起步）。
> **r19 增量**（2026-09-11，批间插入轮 / 41-a 能力模型泛化设计）：**634 → 638（净 +4）**——单元 +4（kerf-driver reserved/capability_model.rs Probe：骨架冻结 + io 族两令牌归属证明 + ffi 令牌归属证明 + 演算位签名证明）。口径注记：单元 198 → 202（driver 60 → 64）；集成 436 不变。零回归实证：capability.rs 12 测试 + reserved_ext_tests 12 集成零改动通过（原则 27）。
>
> **r18 增量**（2026-09-11，批次 H / 语义演进评估轮 + 三债清偿 + HM PoC）：**605 → 634（净 +29）**——集成 +27（tco_tests 12：TD-022 尾调用帧复用/尾位传播/指令预算护栏/TD-007·022 耦合自举端到端 + hm_inference_tests 15：H4 HM PoC 超集门/零误报门/四类缺口/occurs/值限制/多错误）+ 单元 +2（40-g reserved Probe 拆分：合并冻结测试 → 4 子模块独立 Probe）。负例改写注记：帧上限/深递归追踪/双层调用链三用例改非尾形态（TCO 语义变更——尾递归不再耗帧）；编译器两单测指令序列断言 Call→TailCall；expander 深度消息 500→10000 三处同步。口径注记：单元 196 → 198（driver 58 → 60——Probe 拆分净增）；集成 409 → 436。
> **r13 增量**（2026-09-11，批次 E 首个 MUV——TD-004 作用域集解析收口）：+9 集成（scope_set_tests——Racket 式 `(name, scopes ⊆)` 双路径语义锚点：嵌套 shadowing/闭包捕获/set! 词法命中 4 正 + 作用域不匹配未绑定 VM/eval/set! 3 负 + 宏引入不捕获 + 子集对照）——500 → **509**。
> **r14 增量**（2026-09-11，批次 E / E1-α 自举 Expander）：+19 集成（bootstrap_expander_tests——expander.krf（核心形式 + 九糖，VM 上运行）与 Rust 种子 parity：结构+Span+作用域集+param_scopes 递归一致 / 错误消息+Span 逐字一致 / define-syntax E1-α 边界 / 行为面端到端可执行）——509 → **528**。
> **r17 增量**（2026-09-11，批次 G / 后端·FFI·类型三主线）：**553 → 605（净 +52）**——集成 +40（qbe_backend_tests 24：G1 QBE 后端 PoC 端到端/结构/一致性/负例/契约 + multi_error_recovery_tests 16：TD-013 恢复双路径/合并诊断/上限/次序/短路边界）+ 单元 +12（kerf-backend 新 crate 9：codegen 契约迁移 3 + qbe 2 + aot 4；kerf-expander +4 recover 单元；kerf-driver reserved/codegen 契约测试 2 迁移 -2 + 兼容锚 1）。口径注记：单元分项以逐二进制实测为准（196 = span 11 + syntax 11 + core 10 + reader 23 + expander 32 + compiler 15 + runtime 9 + vm 18 + driver 58 + backend 9）。
>
> **r16 增量**（2026-09-11，批次 F / Stage 1 深审收尾环）：零测试变更（纯审查 + 文档 + 注释轮——**553 零断言修改逐一等价复跑**：探针临时部署/移除各一次全绿验证）；本行 + 表体两行对账（bootstrap_expander_tests 19→36 的 r15 尾差 + prelude_tests 行补录——31-e 起 header 增量与表体同步义务的漏网，36-d 发现）。
> **r15 增量**（2026-09-11，批次 E / E1-β 宏收口 + 生产切换 + TD-021）：+24 集成（bootstrap_expander_tests 19→36：宏 parity 17——define-syntax/syntax-rules/卫生 α 重命名/省略号（零/多段/复合）/字面量/多子句/糖覆盖/深度上限/向量模式 + prelude_tests 7——TD-021 hofs 用户面/组合管道/双路径/opt-in/显式失败/未知导入）+ +1 单元（driver 生产切换守护 production_expander_is_bootstrap——独立线程活性探针 + 展开代次标记）——528 → **553**。
> **Version**: v0.1.0-r36

r36 增量行（2026-09-15，批次 K 次件 / K2 大阶段末深审环（会话 Task 57-a/57-s1/57-s2/57-z/57-web）：**758 → 758（零测试函数增量——§3.2 六命令 clean 起步全绿 + 修复后复验双绿 + 四审计集 EXIT 0 ×4 + CLI 四路径）**——§14.5/14.6/14.8/14.9 全协议面：①**D1-D8 全量三段式深审**（deep-review-round2.md——含 §14.8 偏差清单 B2 五项全回写 + B1-1 改判批次 L/M + B4-1 裁定登记册承载 + §14.9 C1-C6 六维表）；②**§14.6 六件套**（architecture-review 八阶段全绿 + §11 七项零违规 / design-impl-test-coverage 四方互锚零缺项 / hidden-problems 强制修复项 = 0 + Stage 3 就绪 12/12 / refactoring-optimality 7/7 最优 / performance-baseline 新建 + TD-028 / final-assessment **Stage 3 GO**）；③**§6.3 五角色投票 5.5/5.5 = 100% GO**；④§14.9 修复面：19 处注释级（P2 时效 7 + P3 12）零行为变更；本行 + RELEASE_NOTES r36 节 + pipeline v0.4.0-r36 + plan K2 执行注记 + rec 树 20_r36/l。

r35 增量行（2026-09-15，批次 K 首件 / K1 终门审 + 同步轮（会话 Task 56-a/56-b/56-z——用户三指令：「将更新的 docs/lang-design/ 下所有内容同步到 docs/stage0.md 并反哺 docs/sop.md」+「按 sop.md 继续推进任务」+「同步完整打包 tar.gz 并同步完整更新 web page」）：**758 → 758（零 cargo test 计数增量——审计集为 examples/audit/ 可重运行口径另计；§3.2 六命令 clean 起步全绿 + 758:0:0 零断言修改）+ 四审计集 EXIT 0 ×4（stage0 41 + stage1 50 + stage2_r1 53 + **stage2_r2 46 新件 APPROVED**）+ CLI 四路径**——56-a 同步轮：stage0.md v6.4 → v6.5（附录 A +33 设计栈术语[18 §5a 七条 + §5b 十三条 + §5c 十三条] + 附录 E 新增 E.9 节[19 §7/§8 共 14 条] + §23.1 标题计数修正[三十二条→三十五条] + §7 三义消歧/§9.3.2 LSP×命名空间/§21.5 时机治理单源三指针注记）+ sop.md v12.3 → v12.4（§8.4.3 目录树 24 文件全列 + §8.4.5 查询表 +4 设计栈落点行 + §16.1 版本历史 + 存档纪律吸收：附录级内容属「同步债」纳入 §8.5 审查）；56-b K1 终门审：**stage2_gate_audit_r2 46 case**（静态判定面主轴[E0005 HM 旗标期生产判定面——r1 零覆盖半区] + 批次 J 三修复面边界 7 + §21.3 四条件终验 + §21.5 九信号核对[机械可核 7 项进程内实测 + S8 投票协议承载 + S9 K2 承载]）+ §6.3 五角色投票 5.5/5.5 = 100% GO；本行 + RELEASE_NOTES r35 节 + pipeline v0.4.0-r35 + plan K1 执行注记 + rec 树 19_r35/l。

r34 增量行（2026-09-14，批间插入轮第三弹 / 设计缺陷深度审计收敛轮（会话 Task 55-a/55-z/55-web——用户审查指令「定位/权限/能力边界/职责边界/层级处理和管理模型/演进阶段和时机——内循环迭代直至收敛」）：**758 → 758（零 cargo test 计数增量——§3.2 六命令 clean 起步全绿：build --release 13.63s 零告警 / check 0/0 / fmt 0 diff / clippy 超集 0 / test 758:0:0（22 套件），零断言修改；零代码变更轮）+ 三审计集 EXIT 0 ×3（41+50+53）+ CLI 四路径（fib ⇒ 144 / macros ⇒ 42 / check ok / E0005 EXIT 1）**——产出全为设计治理面：新文件 23-evolution-governance.md v1.0（十二审计轴 + 14 项发现全表 + 演进六窗触发表 + 变更通道矩阵 + 准入总表 + 受控债务四步 + 生命周期四阶段 + 收敛证明）+ 21 v1.1（三轴坐标系 L×N×P + 十一域覆盖闭合 + 授权组合闭包 + 编译期权威三判据）+ 22 v1.1（权限矩阵 +2 行）+ 原则 35 三方同步 + 18 §6 批次 M 码位预登记 E0013-E0019 + 同步轮 9 文件 + 知识搜索四查询实证（tool-results/r34-search/）；本行 + RELEASE_NOTES r34 节 + plan r34 批间插入注记 + rec 树 18_r34/l。
> **r33 增量**（2026-09-13，批间插入轮第二弹 / 能力架构深度设计轮（会话 Task 54-a/54-z/54-web——用户审查指令驱动：「能力定位/边界/职责/正交 + 命名空间层级/权限控制 + 原语级 ≠ 重命名 + 不止内置和命名」）：**758 → 758（零 cargo test 计数增量——§3.2 六命令全绿：build --release 5.35s 零告警 / check 0/0 / fmt 0 diff / clippy --all-targets --workspace 超集 0 / test --release --workspace 758:0:0（22 套件全 ok——单元 215 + 集成 543），零断言修改；唯一代码变更 = builtins.rs 头注 21/22 设计指针——注释级零语义）+ 三审计集 EXIT 0 ×3（41+50+53）+ CLI 四路径（run fib ⇒ 75025 ⇒ 144 / run macros ⇒ (2 1) ⇒ 42 / check ok / check 负例 E0005「+ 需要数值，实际 str」EXIT 1）**——产出全为设计/架构面：lang-design 新文件双份 21-capability-architecture.md v1.0（三义定锚 + L0-L3 四要素卡 + 正交判据 J1-J4 实测锚 + 八库能力域 + 授权三态 + 原语四要素卡 11 张 + 六类迁移分类学 M-R/D/I/L/E/A）+ 22-namespace-design.md v1.0（五层 N0-N4 + 解析 R-N1~N8 + 遮蔽许可表 + 权限矩阵 + 冲突三类 + kerf/ 保留域 + 批次 M 实施对账表 12 行）+ 20 v1.1 修正（失实叙述 + 分工接线 + 对齐精确化 + 完整性核查 13→18 维）+ 原则 34 三方同步（17 v6.4 / sop §2.2 v12.2 / stage0.md §23.1 v6.3 + §9.7 镜像）+ 00 v6.4 / 09 v6.5 / 12 v6.7 / 18 v6.3（§5b +14 术语）/ 19 v6.2（§7 +8 引用）/ 13 v6.2（三义消歧指针）+ 知识搜索八查询实证（tool-results/r33-search/）；本行 + RELEASE_NOTES r33 + plan r33 批间插入注记 + rec 树 17_r33/l。
> **r32 增量**（2026-09-12，批间插入轮 / 表面现代化设计轮（会话 Task 53-a/53-z——用户审查指令驱动：「实现端不要停留在 Lisp 家族 1970 年代表达」）：**758 → 758（零 cargo test 计数增量——§3.2 六命令 clean 起步全绿：build 5.35s（增量）/ check 0/0 / fmt 0 diff / clippy --all-targets --workspace 超集 0 / test --release --workspace 758:0:0，零断言修改；唯一代码变更 = builtins.rs 文件头/注册段注释 3→4 Reader 原语实测对账 + 现代化方向指针——零语义变更）+ 三审计集 EXIT 0 ×3（41+50+53）+ CLI 四路径（run fib ⇒ 144 / run macros ⇒ 42 / check ok / check 负例 E0005 检出 EXIT 1）**——产出全为设计/规范面：lang-design 新文件 20-surface-conventions.md v1.0（R1-R6 六规则 + B1-B5 行为契约 + 命名空间层 + 57 项映射表 + 零破坏三批次迁移 v0.5/v06/Stage 3）+ 原则 33 三方同步（17 v6.3 / sop §2.2 v12.1 / stage0.md §23.1 v6.2）+ 09 §4 / 12 §2.10 / 18 §5a / 00 v6.3 / 02 注记 + TD-027 登记（P2）+ hm 文档 49→56 实测对账；本行 + RELEASE_NOTES r32 + plan r32 执行注记 + rec 树 16_r32/l。
> **r29 增量**（2026-09-11，批次 J 执行 / 48-c J2 HM 旗标期切换（会话 Task 50-a）：driver.rs check_source/check_source_recover 两入口判定面 = hm_check_program——D8 阶段 2 落地）：**710 → 713（净 +3 集成：typecheck_tests 25 → 28——旗标期判定面组（增值面生产证据：`(f "s")` 约束传播检出「类型不一致」（R1-R8 静默面）+ occurs 豁免政策锚（无限类型报出但不断言双向锚——契约 §2.3 政策行为）+ 双面修复锚（TD-011 字符串全序零误报 + car nil 检出））**；断言重锚 5 处子串（HM 渲染措辞：`(int . int)`/`(int . nil)`/`(α0 → α0)`/元数区间格式——语义/检出/定位不变）；**双缺口当场修复**：hm.rs Ordering 臂同步 TD-011 r24 字符串全序（`(< "a" "b")` 曾误报）+ car/cdr 臂 Nil 移出保守跳过（`(car nil)` 曾漏检）——超集门/零误报门经生产入口实测全过；R1-R8 退为回归基线断言（hm_inference_tests/effect_tests 双面检出纪律保留）；run 路径零接触维持；门审计集三件 144 case 另计全过；CLI check 负例生产实证（E0005 定位 1:24）。

> **r31 增量**（2026-09-12，批次 J 收口 / 48-e J4 批次 J 收尾（会话 Task 52-a/52-z——零代码收尾轮）：**758 → 758（零 cargo test 计数增量——§3.2 六命令 clean 起步复验全绿：build 13.81s / check 0/0 / fmt 0 diff / clippy --all-targets --workspace 超集 0 / test --release --workspace 758:0:0，零断言修改零语义变更）+ 三审计集 EXIT 0 ×3 + CLI 四路径（run fib ⇒ 144 / run macros ⇒ 42 / check ok / check 负例 E0005「类型不一致：num 与 str」定位 1:24 EXIT 1）**——产出全为流程面：12 §2.5.1 十一行注记批次 J 终态复核（v6.5——Effect Handlers 48-b 兑现注记 + 字节码 VM+GC FFI Φ 簿注记 + 类型检查器行终态（默认期评估移交批次 K 终门审））+ plan §5b J 执行注记 r31 + Status 行 + RELEASE_NOTES r31 + rec 树补账（14_r30 + 15_r31 + l 两层）+ r31 tar.gz 包内自举 + web 同步；TD 登记册零事件。
> **r30 增量**（2026-09-12，批次 J 执行 / 48-d J3 FFI VM 面做实（会话 Task 51-a，§21.3 条件 4 兑现）：**713 → 758（净 +45：ffi_vm_tests 37 新文件（13 边界 case 全判定落地映射表 + 正负比 7:22 功能点粒度 ≥1:3 + Φ/P-U 机制组 6 + E0010/E0011/E0012 码断言）+ driver ffi.rs 单元 8（FfiBoundary P1/P2/U1 + F-PIN + lowering 三拒绝 + 注册面 + 往返恒等）——操作码三指令 43→46（语言面形式 Stage 3 编译臂不发射）+ 窗口规程 Φ 簿 + 线性令牌状态机 + E0010-E0012 落位（18 §6 r18 预留兑现）+ 正例端到端（冻结 FfiCall → lowering → 操作码 → 窗口规程 → 真实 I/O））；门审计集三件 144 case 另计全过。
> **r28 增量**（2026-09-11，批次 J 执行启动 / 48-b J1 效应 typecheck 收敛（会话 Task 49-a）：typecheck.rs + hm.rs Perform/Handle 臂子表达式遍历——补深审 D3/D8「Unknown 放宽面」覆盖缺口）：**706 → 710（+4 集成：effect_tests 17→21——静态收敛组（Perform 效应值 R2/R1 + handler 体 R5 + handle 体 R1 + 双体多错误收集 + 零误报对照（六正例 + 绑定器动态用点 + effect_stress.krf 语料）——双面检出（check_program 保守面 + hm_check_program PoC 面，超集纪律））**；两臂收敛形态：Perform 效应值受 R1-R8/约束集检查、Handle 体与 handler 体入检出域（payload/resume 装订动态值——遮蔽纪律与 Lambda 同型）；结果类型维持 Unknown/Dynamic（行多态 Stage 3 边界如实）；bootstrap_compiler_tests 效应 parity 维持（编译面零改动，32/32 零回归）。
> **r27 增量**（2026-09-11，批次 J 规划轮 / 48-a：§5b 批次 J 细化 + §5c 批次 K 概排 + §5d Stage 2 余下面处置）：**706 → 706（零 cargo test 计数增量——零代码规划轮，§3.2 六命令 clean 起步复验全绿）**——产出全为文档面：plan §5b 四 MUV（48-b 效应 typecheck 收敛 / 48-c HM 旗标期切换 / 48-d FFI VM 面做实 / 48-e 收尾）+ §5c 终批三 MUV（49-x）+ §5d 处置表（12 §2.5.1 十一行落位 + net 窗口核对闭环 Stage 3 触发式 + TD-003/005/015 排期改判 Stage 3）；零断言修改零语义变更。
> **r26 增量**（2026-09-11，批次 I 收口 / 42-g I3 门审查 + 42-h 收尾）：**706 → 706（零 cargo test 计数增量——门审环）**——新增**门审计集第 4 件：stage2_gate_audit_r1（53 case，examples/audit/——§7.3.1 规则 3 口径为 example 可重运行审计器，不入 cargo test 套件计数）**：A12 单语句 + B12 多语句（糖/module/require/宏/能力门控 E0006 编译期拒绝面）+ C10 复杂（⑥循环 ⑦深度 + E0007/E0008/E0009 深链）+ D6 恢复 + E7 上轮修复边界（§7.3.2——r23/r24/r25 修复面逐项锚定）+ P6 正向含 §21.3 四条件锚定探针；实测 53/53 PASS EXIT 0（七类全覆盖 1/4/2/3/12/1/2 + 配比全过 + 零发现）；伴随 C 层整理：hm.rs 2 处 catch-all 注释补齐（§14.6.1.1）+ 审计器 2 处 clippy 修复——零语义变化零计数变更。
> **Status**: Active

> **r23 增量**（2026-09-11，批次 I 执行 / 42-d I1 收口：生产切换 + 门 B fixpoint + eval 退役）：**665 → 670（净 +5）**——集成 +2（bootstrap_compiler_tests 27 → 29：**门 B fixpoint**（B₁/B₂ 四程序 bytecode_equal + SHA-256 摘要一致——§21.3 条件 2 终验）+ **B₁ 产物可执行面**（install 后自举链运转 = 种子链结果））+ 单元 +3（driver 64 → 67：生产编译守护 production_compiler_is_bootstrap（独立线程活性探针双信号）+ 缓存 CompilerKind 分桶 ×2（键区分 + 跨桶隔离——B11/P4））。口径：单元 202 → 205 + 集成 463 → 465；T1 双路径面全量迁移（eval → 种子链对拍——十测试文件 + common + 双审计集，断言迁移非删除：净零计数变更）。
> **r25 增量**（2026-09-11，批次 I 执行 / 42-f I 后段双主题：Effect M1-M5 + 能力管线泛化 M2）：**684 → 706（净 +22）**——集成 +20（**effect_tests 17 新建**（效应语言面验收：设计锚正 6——单 handler 单恢复/嵌套逃逸外层/纯体零效应/恢复后环境一致性（共享单元格跨 resume 可见）/效应值先求值序/TCO 10 万深穿透 handler 帧 + M3 双路径 8 case（42-d 新口径：种子/生产双编译链）+ eval 域 dispatch 一致 3 case + 负例 7（E0007 逃逸/E0008 二次恢复含首恢位置/E0009 元数/非 continuation 值（E0004 通用族口径）/handler 子句形态 E0002×2/perform 非点对/非符号 tag）+ GC 存活 2（M5 第六来源——payload 跨 5 万分配压力 + continuation 装箱解箱存活））+ bootstrap_compiler_tests 29 → 32（门 A 效应组 3：perform/handle 基础 parity + trampoline 共享/捕获面 parity + resume 往返行为面））+ 单元 +2（capability.rs M2：io_family_grant_satisfies_model_family（IoFamily 形状 = Grant<io 族> 别名兼容证明）+ net_gate_rows_unchanged_in_42f（门控表零增行评估锚））。**case 级注记**：architecture_audit 冻结证明 10 → 12 实例（原语集 9→11——Perform/Handle 入冻结集）；opcode_count 41 → 43（效应两指令三方冻结）。口径：单元 205 → 207 + 集成 479 → 499。
> **r24 增量**（2026-09-11，批次 I 执行 / 42-e I2：stdlib/GC/TD 批——五债清偿 + 谓词/foldr 补齐 + TD-023 对症）：**670 → 684（净 +14，集成侧）**——stdlib_tests 17 → 24（+7：字符串全序正 12 case/负 3 + 装箱往返正 10/负 2（TD-010）+ 类型谓词正 17/环安全 2/负 3）+ gc_tests 6 → 9（+3：装箱闭包捕获存活 + 捕获链传递 + GcCell 写路径 sound 不变式（TD-023））+ prelude_tests 7 → 10（+3：foldr 用户面/对偶语义/双路径）+ scope_set_tests 9 → 10（+1：TD-018 if 消息双路径同文对拍回归）。**case 级净增**：parity_err +2（TD-014 嵌套 define 消息+Span 逐字）+ R3 静态面 2 case 语义反转（全字符串排序链放行——TD-011）+ HM 超集门语料 1 例换混串链 + 负例改写 1（字符串排序链拒绝 → 混合链拒绝——语义边界迁移）+ e6 矩阵断言更新（TD-014 专门消息）。口径：单元 205 不变 + 集成 465 → 479。

## 总量

**758 通过 / 0 失败 / 0 忽略**（758 个测试函数 = 单元 215 + 集成 543，逐二进制实测汇总；另**门审计集四件合计 190 case**（stage0 41 + stage1 50 + stage2_r1 53 + stage2_r2 46——examples/audit/ 可重运行审计器，§7.3.1 规则 3 口径，不入套件计数；r36 修正：原「三件 144」为 r35 增量行后未同步总量行的同步债——本轮清理）；r10 +4 架构审计 + r12 +20 预留扩展 + r13 +9 作用域集锚点 + r14 +19 自举 Expander parity + r15 +25 宏收口/prelude/生产切换守护 + r17 +52 批次 G：QBE 后端 PoC 40 + TD-013 恢复 12 + r18 +29 批次 H：TCO 12 + HM PoC 15 + Probe 拆分 2 + r19 +4 批间插入轮：能力模型骨架 Probe 4 + r20 +0 设计轮：I1 切口设计（parity 三门 A/B/C 为 42-b/c/d 增量测试的验收合同）+ r21 +19 批次 I·I1 前段：自举 Compiler 门 A parity（bytecode_equal 全结构判据）+ 42-c 边界负例 + 行为面端到端 + **r22 +8 批次 I·I1 中段：module/require 两臂 + 糖九件全管线 + 宏 + prelude 注入序 + examples 六件双路径（扩展组 46 case）+ 行为面糖/module** + **r23 +5 批次 I·I1 收口：门 B fixpoint（B₁/B₂ 字节一致 + SHA-256）+ B₁ 可执行面 + 生产编译守护 + 缓存分桶 ×2（CompilerKind 生产切换 42-d）** + **r24 +14 批次 I·I2：stdlib/GC/TD 批（TD-010/011/014/018/023 五债清偿 + 谓词 5 + foldr + GcCell 对症——42-e）** + **r25 +22 批次 I·I 后段：Effect M1-M5（原语集 9→11 + VM ext1/INSTALL_HANDLER/PERFORM + continuation 四要素 + E0007-E0009 + GC 六来源 + 双路径/parity/行为面）+ 能力 M2（IoFamily 别名兼容 + 门控表零增行）——42-f** + **r26 +0 批次 I 收口：I3 门审（stage2_gate_audit_r1 53 case APPROVED + 深审 D1-D8 + 五角色全票 GO——42-g/47-a）** + **r27 +0 批次 J 规划轮：§5b/§5c/§5d 三节 + 余下面处置（零代码——48-a）** + **r28 +4 批次 J·J1：效应 typecheck 收敛（typecheck.rs/hm.rs Perform/Handle 子表达式遍历 + 静态负例组双面检出——48-b/49-a）** + **r29 +3 批次 J·J2：HM 旗标期切换（driver 两入口判定面 = hm_check_program + 契约重定义显式登记 + 断言重锚 + 双缺口修复——48-c/50-a）** + **r30 +45 批次 J·J3：FFI VM 面做实（操作码 43→46 + 窗口规程 Φ 簿 + 令牌状态机 + E0010-E0012 + 13 边界 case——48-d/51-a）**）。
§3.2 release 验收基线 204（r2）→ r3 负向测试扩张 + 审计集就位 + FS-1 守卫修复 + 糖正向锚点 + T17-a 六缺陷修复回归后 297 → r4（Stage 1 批次 A）304 → r5（批次 B TD-002 符号值 + 标准库最小集）324 → r6（批次 B 收官 B3 自举 Reader）356 → r7（批次 C 类型检查器 + 编译缓存 + TD-016）408 → **r8（批次 D 能力 I/O + 内部效应 + 用例运行器）476** → **r10（架构合规审计 +4）480**：+24 capability_tests（require 声明面 + E0006 三路径门控 + 豁免/形状/令牌 + EOF 子进程探针）+ +18 test_runner_tests（前置切分 + PASS 判定 + 短路/恢复/隔离 + front 错误面）+ +25 kerf-driver 单元（effects.rs 12：逃逸层/最近匹配/载荷保真/穿透契约 + capability.rs 13：R9 验证/豁免/编组）+ +1 negative_vm_tests（read_line_arity 自 ignore 激活——**FS-4 修复**，能力参数化重写时补齐元数校验）→ **r12（接口预留完整性扩展 +20）500**：+8 单元（reserved/ 模块 Probe 冻结——「测试实现体编译通过 = 契约冻结」先例沿用）+ +12 集成 reserved_ext_tests（P0 数据结构位置六项断言 + 预留 API 跨 crate 可达 + 负向形状：空目标拒绝/空片段类型检查失败/rename 错误面）→ **r13（批次 E·TD-004 作用域集解析收口 +9）509**：+9 集成 scope_set_tests（双路径语义锚点 + 作用域不匹配负例——见 docs/tests/v0/stage1/plan/scope-set.md）→ **r14（批次 E·E1-α 自举 Expander +19）528**：+19 集成 bootstrap_expander_tests（expander.krf 与 Rust 种子 parity——结构/Span/作用域集/param_scopes/错误消息逐字一致 + 行为面端到端；见 docs/tests/v0/stage1/plan/bootstrap-expander.md）→ **r15（批次 E·E1-β 宏收口 + 生产切换 + TD-021 prelude +25）553**：+17 宏 parity（镜像 macro_sys.rs：变换器注册表单表语义/卫生基名回退/省略号/字面量/Span 并集代次守卫（expansion_id 镜像——节点第 4 字段 + retag +1）/深度上限 500 消息逐字）+ +7 prelude_tests（TD-021 模块/import 承载——forms 级合并注入单一编译单元）+ +1 单元生产切换守护（compile_front 展开段经 bootstrap_expander——独立线程活性探针实测）。**全套件经自举 Reader + 自举 Expander（均 kerf 源码，VM 上运行）执行——生产管线读+展开两段全自举（E1-β）。**

> **r7 计数修正**（r8 对账发现，§8.4.5 规则 2——以实测为准）：r7 版本矩阵的分套件表存在陈旧数（头部「集成 173 函数」为 r3 时代口径；单元表 130 实为 150——driver 14→25 / expander 26→28 / compiler 12→15 的 r4-r7 增长未回写；cache_tests 13 实为 14；negative_vm 29 为排除 ignore 的口径）。r7 实际 = 150 单元 + 260 集成函数（259 通过 + 1 ignore）= 408:0:1 ✓（总量正确、分项陈旧）。r8 起全部逐二进制实测。
>
> r8 起 `#[ignore]` 清零：FS-4（read-line 元数）随批次 D 能力参数化重写修复激活；
> r3 内已修复激活（原 4 个 ignore → 1）：FS-1 嵌套守卫（10_000→256）、
> D3 eval 深度上限（MAX_EVAL_DEPTH=256 结构化）、D4 mod/除 i64::MIN
> 溢出（checked_div/rem）。

## 分套件统计（2026-09-10 r8 实测）

### 单元测试（202，crates 内联——r12 +8：reserved/ Probe 冻结；r17 +13：backend 9 + expander 4；r19 +4：capability_model Probe）

| 套件 | 层级 | 文件/位置 | 测试数 |
|------|------|----------|--------|
| kerf-span 单元 | crate 内联 | crates/kerf-span/src/*.rs | 11 |
| kerf-syntax 单元 | crate 内联 | crates/kerf-syntax/src/*.rs | 11 |
| kerf-core 单元 | crate 内联 | crates/kerf-core/src/*.rs | 10 |
| kerf-reader 单元 | crate 内联 | crates/kerf-reader/src/*.rs | 23 |
| kerf-expander 单元 | crate 内联 | crates/kerf-expander/src/*.rs | **32（r17 +4：recover.rs 形式级恢复/上限/次序/干净路径）** |
| kerf-compiler 单元 | crate 内联 | crates/kerf-compiler/src/*.rs | 15 |
| kerf-runtime 单元 | crate 内联 | crates/kerf-runtime/src/*.rs | 9 |
| kerf-vm 单元 | crate 内联 | crates/kerf-vm/src/*.rs | 18 |
| kerf-driver 单元 | crate 内联 | crates/kerf-driver/src/*.rs | **67（r23 实测修正：r22 版 64（r15 +1 生产切换守护 / r19 +4 capability_model Probe / r12 +8 reserved Probe / r8 +25 effects+capability——表体 58 为 r12 时代陈旧数，本轮 --list 权威计数对账）；r23 +3：production_compiler_is_bootstrap 生产编译守护 + cache 分桶 ×2（B11））** |
| kerf-backend 单元 | crate 内联 | crates/kerf-backend/src/*.rs | **9（r17 新 crate：codegen 契约迁移 3 + qbe 2 + aot 4）** |

### 集成测试（499 函数，tests/ 阶段树——r9 起经 runner.rs 单一总入口组织；r10 +4 审计；r12 +12 预留扩展；r13 +9 作用域集锚点；r14 +19 自举 Expander parity；r22 +8 bootstrap_compiler 扩展组；r24 +14 stdlib/GC/TD 批；r25 +20 效应面 effect_tests 17 + bootstrap 效应组 3）

> **入口口径（r9）**：`tests/runner.rs` 为唯一集成测试目标（cargo 自动发现，Cargo.toml 零 [[test]] 声明）；下表各「套件」现为 runner 内 `#[path]` mod 树的**模块**（选择性运行 `cargo test --test runner <module>::`）——逐模块计数与 r8 逐二进制口径完全一致（476 总数不变，组织收敛）。共享辅助 `tests/common/` 经 runner 单实例共享（`use crate::common`——替代原每文件 `mod common` 重复加载）。

| 套件 | 文件 | 函数数 |
|------|------|--------|
| reader_tests | tests/v0/stage0/plan/reader_tests.rs | 10 |
| expander_tests | tests/v0/stage0/plan/expander_tests.rs | 18 |
| compiler_tests | tests/v0/stage0/plan/compiler_tests.rs | 10 |
| vm_tests | tests/v0/stage0/plan/vm_tests.rs | 20 |
| gc_tests（r24 +3 TD-010/023） | tests/v0/stage0/plan/gc_tests.rs | 9 |
| pipeline_tests | tests/v0/stage0/plan/pipeline_tests.rs | 11 |
| architecture_audit_tests（r10） | tests/v0/stage0/plan/architecture_audit_tests.rs | **4（r10 新增）** |
| gate_review_r1 | tests/v0/stage0/gate/gate_review_r1.rs | 8 |
| negative_reader_tests | tests/v0/stage0/plan/negative_reader_tests.rs | 14 |
| negative_expander_tests | tests/v0/stage0/plan/negative_expander_tests.rs | 23 |
| negative_vm_tests（r8 +1 激活） | tests/v0/stage0/plan/negative_vm_tests.rs | 30 |
| negative_semantics_tests | tests/v0/stage0/plan/negative_semantics_tests.rs | 23 |
| stdlib_tests（r5；r7 +TD-016；r24 +7 全序/装箱/谓词） | tests/v0/stage1/plan/stdlib_tests.rs | 24 |
| bootstrap_reader_tests（r6） | tests/v0/stage1/plan/bootstrap_reader_tests.rs | 28 |
| expansion_worklist_tests（r4） | tests/v0/stage1/plan/expansion_worklist_tests.rs | 4 |
| typecheck_tests（r7；r29 旗标期判定面组 +3 + 断言重锚——实测 28，r7 版 24 口径漂移修正） | tests/v0/stage1/plan/typecheck_tests.rs | 28 |
| cache_tests（r7；r22 对账实测 13——r7 版 14 口径漂移修正） | tests/v0/stage1/plan/cache_tests.rs | 13 |
| **capability_tests（r8，批次 D）** | tests/v0/stage1/plan/capability_tests.rs | **24** |
| **test_runner_tests（r8，批次 D）** | tests/v0/stage1/plan/test_runner_tests.rs | **18** |
| **reserved_ext_tests（r12，预留扩展）** | tests/v0/stage1/plan/reserved_ext_tests.rs | **12**（P0 位置断言 4 + API 可达 1 + 形状行为 7——含负向：空目标拒绝/空片段类型检查失败/rename 错误面） |
| **scope_set_tests（r13，批次 E·TD-004；r24 +1 TD-018 对拍）** | tests/v0/stage1/plan/scope_set_tests.rs | **10**（双路径正例 5：shadowing/嵌套 shadowing/闭包捕获/set! 词法命中/子集对照；负例 4：作用域不匹配未绑定 VM/eval/set! 三锚 + 宏引入不捕获） |
| **bootstrap_expander_tests（r14/r15，批次 E·E1-α + E1-β 宏收口）** | tests/v0/stage1/plan/bootstrap_expander_tests.rs | **36**（r14 E1-α：parity 正例 8 + 负例 6 + 边界 1 + 行为面 3；r15 +17 宏 parity：define-syntax/syntax-rules 全模式面/卫生 α/省略号零-多段-复合/字面量/多子句/糖覆盖/深度上限消息逐字/向量模式 + retag 代次守卫） |
| **prelude_tests（r15，批次 E·TD-021；r24 +3 foldr）** | tests/v0/stage1/plan/prelude_tests.rs | **10**（hofs 用户面可见/组合管道 filter→map→foldl=50/for-each 副作用/双路径一致/opt-in 负例/名字捕获显式失败/未知导入） |
| **qbe_backend_tests（r17，批次 G·G1）** | tests/v0/stage2/plan/qbe_backend_tests.rs | **24**（端到端 6 + 结构 4 + 一致性 6 + 负例 10——PoC 边界：lambda 值位/define 非 lambda/Str/Float/set!/module/未定义/arity/自由变量/IO/函数值） |
| **multi_error_recovery_tests（r17，批次 G·G2）** | tests/v0/stage2/plan/multi_error_recovery_tests.rs | **16**（种子恢复 6 + driver 8 + 双路径同构 1 + 执行路径不变 1——TD-013 双路径恢复 + 合并报告） |
| **tco_tests（r18，批次 H·H2）** | tests/v0/stage2/plan/tco_tests.rs | **12**（TCO 正例 8（105_001 恒定帧/相互尾递归/if 两臂/begin 末项三层/let 糖/内建隐式 RET/闭包值尾位/自举 10_000 深度链）+ 负例 4（指令预算护栏/尾调用 arity/非可调用/非尾深递归仍帧上限）） |
| **hm_inference_tests（r18，批次 H·H4）** | tests/v0/stage2/plan/hm_inference_tests.rs | **15**（双门：超集门 29 程序 + 零误报门（examples 六件套 + 动态边界）+ 四类缺口检出 + occurs/值限制/let-letrec 泛化/多错误 Span 序/Dynamic 逃生舱/512 预算） |
| **bootstrap_compiler_tests（r21/r22/r23/r25，批次 I·I1 自举 Compiler + 效应组）** | tests/v0/stage2/plan/bootstrap_compiler_tests.rs | **32**（r25 效应组 3：perform/handle parity 基础 + trampoline 共享/捕获面 + resume 往返行为面；r21 门 A 基础组：bytecode_equal 全结构 parity 13 + 负例 2（define 位置 D1 逐字）+ 行为面 4；r22 扩展组：module/require 两臂正例组 + 糖九件全管线 46 case + 宏 3 + prelude 注入序 2（preamble.krf 真实语料）+ examples 六件双路径 + 行为面糖/module 2 组——42-b 边界负例改写正例（两臂迁移落地）；r23 门 B：fixpoint 两次编译自身字节一致（B₁/B₂ 四程序 + SHA-256 + 加强判据按名反汇编）+ B₁ 产物可执行面（install 后自举链 = 种子链结果）） |
| **ffi_vm_tests（r30，批次 J·J3 FFI VM 面——48-d/51-a）** | tests/v0/stage2/plan/ffi_vm_tests.rs | **37**（正例 7：write_stdout 端到端真实 I/O/窗口配平（Φ 配平 + total_allocs Δ=1 case 6）/重复窗口/分配释放往返/令牌借用传递/NULL 哨兵 no-op case 13/CInt 往返 + Φ 机制 6：P1 累计/P2 非堆 no-op/U1 摘根/F-PIN case 1/浅 pin 等价 case 8/绑定变更 case 2 + case 9 三容忍观测（错误路径 pin 泄漏 + panic unwind + 槽回收令牌真相）+ 负例 22：E0010 ×4（双释 case 5/用后传递/共享失效 case 11/U2 下溢）+ E0011 ×9（Opaque/非令牌 ×2/size0 双面 case 4/元数 ×2/类型 ×2/lowering ×2）+ E0012 ×2 + E0004 宿主传播 + 码面防御 ×2 + 子集外拒绝 case 3） |
| **effect_tests（r25，批次 I·I 后段 Effect M1-M5——42-f）** | tests/v0/stage2/plan/effect_tests.rs | **17**（设计锚正 6：单恢复/嵌套逃逸/纯体/环境一致性（共享单元格跨 resume）/求值序/TCO 穿透 10 万深 + M3 双路径 8 + eval 域 3 + 负例 7：E0007/E0008（首恢位置）/E0009/非 continuation（E0004 口径）/子句形态 E0002 ×2/非点对/非符号 tag + GC 存活 2：payload 跨 5 万分配/装箱解箱往返） |

### 负向测试规模与正负比（§9.4.3 对账）

| 负测文件 | 函数数 | case 数（按文件内注释汇总） |
|----------|--------|------------------------------|
| negative_reader_tests.rs | 14 | 59 |
| negative_expander_tests.rs | 23 | 96 |
| negative_vm_tests.rs | 30 | 237（r5 +6 符号值误用；r8 +1 read-line 元数 FS-4 修复锚） |
| negative_semantics_tests.rs | 23 | 98 |
| **四文件合计** | **90** | **490** |
| stdlib_tests（r5+r7+r8，tests/v0/stage1） | 17 | 181+（元数 25/类型 86/边界 12/语义 31/双参扫描 18/r7 TD-016 +9——r8 起门控 I/O 负例携带 require 前缀，case 集不变） |
| typecheck_tests（r7，tests/v0/stage1；r29 旗标期 +3） | 28 | 负例 68 case + 正例锚 25 case + 旗标期判定面组 3（增值面生产证据 + occurs 豁免政策锚 + 双面修复锚） |
| bootstrap_reader_tests（r6，tests/v0/stage1） | 28 | 307 |
| 审计集（examples/audit/stage0_gate_audit_r1.rs） | — | 41（负向 32 + 恢复 6 + 正向 3） |
| reserved_ext_tests（r12，负向形状） | 12 | 负向形状 case 3（空目标/空片段/rename 错误面）+ pin/unpin 对称断言 |

- **全局正负比（case 口径）≈ 1:3.15 维持**：r8 新增负向 case ≈
  capability_tests 15+（门控 8 + 形状 4 + 边界 3）+ test_runner_tests 12
  （失败/短路/front 错误面）+ effects/capability 单元负向 15 ≈
  **1118 vs 正向 ≈355**（r7 1077:340 基础上同步扩张）——
  **§9.4.3 的 ≥1:3 门限维持达标**（r1 审查时为 1:0.24）。
- 逐分类负测非零：reader/expander/compiler/vm/gc/pipeline/gate/driver 全部含负向 case。
- §7.1.1 七类负向矩阵 7/7（含空应用与模块循环依赖）；E 码直接断言：E1–E6 全部
  （negative_semantics_tests 逐码矩阵）+ E0001/E0002/E0004 结构化断言
  （negative_reader/expander/vm）+ E0005 静态检查码（r7 typecheck_tests 逐条断言）
  + **E0006 能力权限码（r8 capability_tests 逐条断言——第七族结构码就位）**；
  E7/E8/E0003 经公开 API 不可触发——文档化存档。
- 负测文档锚点：[negative-tests.md](./v0/stage0/plan/negative-tests.md)。

## 需求覆盖（sop.md §21.3 Stage 0 验收标准 → 测试）

| 验收项 | 覆盖测试 |
|--------|---------|
| (1) 9 原语语义正确 | vm_tests::nine_primitives_semantics + gate_g1/g2 + negative_semantics E 码矩阵 |
| (2) 50+ 快照测试 | 全套件（476 ≥ 50） |
| (3) 自举测试（同结果） | compiler_tests::deterministic + pipeline_tests::convergent |
| (4) Span 全管线传播 | pipeline_tests::span_propagates + gate_g6 + negative_reader/expander Span 精确断言 |
| (5) 性能基准基线 | CLI bench（fib(25) ~86-89ms/轮实测；examples/usage/fib.krf） |
| (6) 四项接口预留冻结 | gate_g7_to_g10 + reserved.rs Probe 测试（reserved_signatures_are_frozen 等）+ **r8：EffectSystem/InternalEffectSystem 真实现与 Probe 构成契约双证；CapabilityIO 经 StdCapabilityIO 做实** |
| (7) §7.3.1 门审计集 ≥30 case | examples/audit/stage0_gate_audit_r1.rs（41 case，配比满足） |

## 测试类型分布

- 快照/黄金输出：Token 快照 / Stx 渲染 / CoreExpr 渲染 / 反汇编 / 运行结果渲染
- 语义断言：9 原语 / fib / 闭包 / 宏 / quote / GC
- 负向/错误（表格驱动，每行 = 1 case）：词法 59 / 展开 96 / VM 237 / 语义 98 / 审计 41 / 能力门控 8+ / 用例失败 12+——全阶段错误路径
- 双路径互查：VM vs eval（全部集成函数内置；错误程序断言 Err 事实一致）
- 压力/稳健：10^6 有界分配（gc_tests，验收口径）/ 3×10^5（gc_stress 示例口径）/ 10^4 深递归 / 2×10^5 深链标记
- **r8 新形态**：效应系统契约测试（一次性逃逸层的最近匹配/载荷保真/真实 panic 穿透）；能力授权链端到端（声明→验证→令牌→门控注册）；子进程确定性探针（EOF 语义——不依赖运行器 stdin 形态）
