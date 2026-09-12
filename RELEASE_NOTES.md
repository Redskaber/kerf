## v0.4.0-r36（2026-09-15）——批次 K 次件：K2 大阶段末深审环 + §14.8 设计回写 + §14.9 系统性代码整理（57-a/57-s1/57-s2/57-z/57-web——用户指令「按 sop.md 继续推进 + 同步完整打包 tar.gz 并同步完整更新 web page」，758 维持全绿 + Stage 3 切换 GO 裁定）

### 交付一：K2 大阶段末深审环——§14.5/§14.6/§14.8/§14.9 全协议面（会话 Task 57-a——plan §5c 49-b 批次 K 次件）

- **§14.6.3 三轮深挖**：第 1 轮事实采集（57-s1 子代理十三项代码扫描[C1-C6 + §11 接口隔离 + §14.6.1.1 数据流覆盖] + 57-s2 子代理八设计文档三方对照[§14.6.1.3 + §14.8 偏差候选] + §3.2 基线六命令 clean 起步实测：cargo clean 589 files/166.5MiB → build --release 14.48s 零告警 → check --workspace 0 errors 0 warnings → fmt --check 0 diff → clippy --all-targets --workspace -D warnings 超集 0 → test --release --workspace **758:0:0 零断言修改** + 四审计集 EXIT 0 ×4[41+50+53+46] + CLI 四路径[fib ⇒ 75025 ⇒ 144 exit 0 / macros ⇒ (2 1) ⇒ 42 exit 0 / check ok「2 原型/9 常量/5 全局引用/38 指令」/ check 负例 E0005「+ 需要数值，实际 str」REAL_EXIT 1] + 性能三重采样）→ 第 2 轮发现即修（19 处注释级修复 + 四件设计回写——见交付二/三）→ 第 3 轮收敛（**零新 P0/P1——R6/R7 收敛判据达成**）
- **§14.5 D1-D8 全量三段式深审（大阶段末版——非 r26 批次裁剪版）**：`docs/develop/v0/stage-2/deep-review-round2.md`——八维度各「现状/风险/建议」三段式 + §6.3 五角色投票（ARCH-A 2/DEV-A 1.5/QA-A 1/ALG-C 1/SKL-A 1 不加权）**加权 5.5/5.5 = 100% GO** + §14.8 偏差清单 + §14.9 C1-C6 六维表。结论：P0 = 0 / P1 = 0 / P2 = 0（7 项当场修复）/ P3 = 12（11 修复 + 1 容忍观察）
- **§14.6 六件套深验证文档（全落位——批次 K K2 验收合同「四项审查各 ≥1 产出文档 + P0/P1 = 0 维持」达成）**：`architecture-review.md`（八管道阶段全绿 + §11 七项机械验证零违规 + 数据流五段校验）/ `design-impl-test-coverage.md`（**「枚举-实现-测试」四方互锚零缺项**：46 操作码 opcode_count_matches_spec 逐一断言 / 57 stdlib 逐名核对 / CoreExpr 12 变体三面穷尽 / E0001-E0012 全测试锚 / prelude 五件；B1 净缺口 0 / B3 零新项）/ `hidden-problems-assessment.md`（**强制修复项[复杂度增长 ≥2×] = 0** + Stage 3 就绪度清单 12/12 ✅）/ `refactoring-optimality-review.md`（Stage 2 七项重构 7/7 最优判据零「治症不治根」hack）/ `performance-baseline.md`（新建大阶段末基线：clean 构建 14.48s/二进制 1.92 MiB/fib 92.5ms/gc_tail 42.6ms/gc_nontail 噪声域/全量测试 54.5s + 复测协议）/ `final-assessment.md`（**Stage 3 切换 GO 裁定**——附条件无）+ `pipeline-test-coverage.md` v0.4.0-r36（§4 完整性审查小节 r36 全量重测：catch-all 86 处/24 文件六分类判定全合规[「_ 臂理由」24 + 显式错误臂 ~30 + unreachable 窄化 4 + 空臂注释 5 + None 契约 ~20 + 数值塔语义 ~7]；生产区 expect 23 处全不变式消息化零裸 unwrap；enum 穷尽性四抽查[Op 46 变体 VM 主分派穷尽无 `_`——新变体编译器强制双面更新]；§6 性能基线表更新）
- **TD-028 新登记（P3）**：VM 主分派与 GC 根扫描性能漂移——fib(25) 84.2→92.5ms（+9.9%）/ gc_stress 尾形 38.57→42.6ms（+9.5%）双基准同向漂移三重采样；归因候选三面（r25 效应两操作码 + r30 FFI 三操作码入主分派 match 46 臂分支布局 / GC 六来源入口 / ForeignBox 追踪器入口）待 Stage 3 优化窗口 profile 定锚（判据先于先例——原则 35）；正确性零影响（§14.5.3 D6 条款「非正确性影响的性能记录为 Stage N+2 优化项」）

### 交付二：§14.8 设计回写四件（实现 → 设计单向——代码超前文档的滞后清零）

- **03-macro-system.md v6.2 → v6.3**：深度上限 500 → **10_000** 全口径对齐（TD-007 r18 全解除——Stx Rc 化 + retag 迭代式重建 + 均匀标记 O(1) 共享；§2.2 契约块 + §4 不变式 1 + §5 测试锚行 + 头部版本行；expander.rs:77 / expander.krf:478 / tco_tests「超限 10000」断言三实锚）+ 推迟项清「迭代式工作表展开（TD-007）」（已 resolved）
- **05-runtime.md v6.2 → v6.3**：根集**五来源 → 六来源**（第 6 源 = 活跃 continuation 帧链——r25/M5；vm.rs:1368 实锚 + effect_tests gc_continuation_boxed；§2/§3.2/§4 三节六处——修复 06 §1.2 指针落空）+ 类型化分配**八入口 → 九入口**（alloc_foreign r30 FFI ForeignBox 装箱第 9 入口——heap.rs:202 实锚）+ 推迟项口径对齐（TD-008 DEFER Stage 3+ / net/process Stage 3 触发式）
- **02-syntax-model.md v6.0 → v6.1**：Keyword **22 → 25 变体**（r25 增 perform/handle/resume 效应三关键字——symbol.rs:24-54 实锚）+ 叶级**45 → 48 种** + **§8.1 两语法同核验证改判**（B1-1：原定 Stage 2 前置未执行——重规划至表面现代化批次 L/M[20 §5 模块树 + 12 §2.10] 承载，目标语法引入随 v0.5/v0.6 窗口，同核验证作为该批次验收件）
- **13-capability-matrix.md v6.2 → v6.3**：**§3.1.1 Effect Handlers 节态由 r8 口径升 r25/r28 终态**（语言面 P0：M1-M5 全量[原语集 9→11 + ext1 具体化 Option<Rc<HandlerFrame>> + continuation 四要素 + E0007-E0009 + GC 第六来源] + 静态收敛面[typecheck/hm 双面检出超集]；行多态/效应行 Stage 3——修复主链 01/04/06/12 已回写而本节残留的「节态滞后」）+ 12-roadmap 宏系统行「深度 500」同步（跟随 03 v6.3 单源）

### 交付三：§14.9 系统性代码整理（19 处注释级修复——零行为变更，758:0:0 复验实证）

- **P2 注释时效 7 项全修**（设计口径已改判未同步的「代码注释滞后」）：runtime/lib.rs TD-008 DEFER Stage 3+ / core/ir.rs TD-003 优化窗口 / expander core_forms.rs:444 + bootstrap expander.krf:1373 net-process Stage 3 触发式（**用户面错误消息双侧同步**——capability_tests:201 断言锚「未知能力主体」前缀不受影响，parity 保持） / reserved/multistage.rs 多阶段 DEFER Stage 3（头部 + trait 注释 + 三方法注释）/ expander/lib.rs:32 深度 128→10_000 / driver/effects.rs:24-25 ext1 已激活（r25 M5 HandlerFrame）+ resumption 语义分域（内部层一次性逃逸 vs 语言级 continuation 四要素）/ effects.rs:19 K2 深审复核结论注记
- **P3 12 项修复 11**：catch-all 无臂注释 5 处补齐（backend/anf.rs:195 prim_arity 默认二元 / compiler/hm.rs:688 扫描窗口终止 / compiler/typecheck.rs:420 防御回退 / expander/macro_sys.rs:191 构造性默认 / vm/vm.rs:1591 NaN partial_cmp 保底）+ 注释枚举不全 2 处（vm.rs:1392 + Symbol / runtime/heap.rs:349 + Symbol）+ 文件头时效 4 处（vm/lib.rs 值模型清单补 Symbol/continuation/Foreign/External / compiler/lib.rs 操作码 39→46 / driver/lib.rs eval 退役 VM 唯一生产路径 / expander/lib.rs TD-004 r13 已解决时态）+ 1 容忍观察（backend qbe.rs gen_il 业界惯用缩写）
- **需复核 3 项处置落位**：capability.rs:65 net 窗口终判注记（plan §5d 触发式口径） / effects.rs:19「Stage 2 复核」= 本轮 K2 承办完成 / expr.rs:92-94 ADT 评估清单冻结维持注记（K1+K2 双复核）
- **修复后复验**：build 12.18s / fmt 0 diff / clippy 超集 0 / **test 758:0:0 零断言修改**——零行为变更实证（§14.9 完成标准 7 项全过：TODO/FIXME/HACK/XXX = 0 / glob re-export = 0 / fmt 0 / clippy 0 / test 全绿 / 无死代码 / 整理报告落位 worklog）

### 交付四：对账四面 + 收尾（57-z/57-web 随同执行）

- 对账四面：matrix.md **v0.1.0-r36**（r36 增量行 + **总量行同步债修正**：门审计集「三件 144」→「四件 190」[r35 遗留的总量行未同步——本轮同步债清理先例延续]）/ pipeline-test-coverage.md **v0.4.0-r36**（§4 r36 全量重测 + §6 性能基线）+ performance-baseline 新建 / RELEASE_NOTES r36 节（本节）/ plan.md K2 执行注记（下一步 K3 收尾交付——K2 GO 已满足输入条件）
- TD 登记册：TD-028 新增（28 项总量：**P0/P1 = 0 维持**——解决 17 + 断档 4 + DEFER 1 + 开放 6）
- r36 tar.gz（§19.3 commit-then-package 正序 + §19.4 十二路径）包内自举 + web 同步（kerf-data r36 节点 + download README r36 节 + footer）+ rec 树 20_r36/l + git 双仓库

## v0.4.0-r35（2026-09-15）——批次 K 首件：K1 终门审 + lang-design→stage0/sop 附录级全面同步（56-a/56-b/56-z/56-web——用户三指令驱动，758 维持全绿 + 第四审计集 46 case APPROVED）

### 交付一：lang-design → stage0.md 附录级全面同步 + sop.md 反哺（会话 Task 56-a——用户指令「将更新的 docs/lang-design/ 下所有内容同步到 docs/stage0.md 中，并反哺 docs/sop.md」）

- **同步缺口实测（磁盘对账）**：①附录 A 术语表缺 r32-r34 三轮 33 条（18 §5a 七条 + §5b 十三条 + §5c 十三条）②附录 E 参考文献缺 14 条（19 §7 r33 八查询 + §8 r34 四查询）③§23.1 标题计数漂移（「三十二条」vs 正文 35 条——内容自 v6.4 已同步，标题停在 v6.1 时代）——判定为「拆分面→存档面的附录级同步债」（r32-r34 各轮只做了 §9.x 存档侧镜像 + 原则增量，附录面无 owner）
- **stage0.md v6.4 → v6.5**：附录 A 增补 33 条设计栈术语（is- 前缀谓词/方向词转换/命名空间限定名/别名层/能力-命名空间对齐/表面债/表面现代化动态演进 + 三义定锚/能力四要素/L0-L3/J1-J4/授权三态/六类迁移/新原语两判据/N0-N4/R-N1/R-N2/import 不传播授权/kerf 保留域/原则 34 + 三轴坐标系/双轴归属/授权组合闭包/编译期权威三判据/诊断终止域/横切泛函域/演进六窗/时间治理四红线/变更通道矩阵/受控债务四步/生命周期四阶段/十二审计轴/判据先于先例）+ 附录 E 新增 E.9 节（66-79 十四条知识搜索引用）+ §23.1 标题计数修正 + 三处指针注记（§7 三义消歧 / §9.3.2 LSP×命名空间三交互点 / §21.5 时机治理单源）+ 版本历史 v6.5 条目
- **sop.md v12.3 → v12.4（反哺）**：§8.4.3 语言设计文档目录树由「00-13 + …（13+ 扩展设计文档）」修正为 24 文件全列（设计栈四件 20/21/22/23 显式列出——r32-r34 三轮交付后的目录树漂移）+ §8.4.5 文档优先查询表增 4 行设计栈落点（库表面命名→20 / 能力归属与授权→21 / 命名空间与模块机制→22 / 演进时机与窗口触发→23）+ §16.1 版本历史 v12.4 + **存档纪律吸收**（附录级内容[术语表/参考文献/原则计数]属「同步债」——自本轮起纳入 §8.5 审查检查项）

### 交付二：K1 终门审——stage2_gate_audit_r2 46 case APPROVED（会话 Task 56-b——plan §5c 批次 K 首件）

- **新审计集第 4 件 `examples/audit/stage2_gate_audit_r2.rs`**（Cargo.toml [[example]] 登记 + §7.3.1 可重运行口径）：**46 case 全 PASS EXIT 0 APPROVED**——与 r1（53 case）零源语料重叠，主轴 = **静态判定面**（r29 HM 旗标期切换后 check_source 生产判定面 E0005——r1 零覆盖的半区：A 桶 10 单语句静态 + B 桶 10 多语句/集成静态含 r28 效应臂收敛三 case + E0006 门控函数体位变体 + 空列表 define 值位变体 + 多错误非短路收集）
- **批次 J 修复边界（§7.3.2 三修复面 7 case）**：r28 效应 typecheck 收敛双 case（perform 效果值双面检出[tc+hm 超集纪律] + handle 双体多错误收集）+ r29 HM 旗标双缺口修复双 case（TD-011 字符串全序零误报/混串检出对偶 + car/cdr Nil 漏检归零）+ E0005 定位面（文件名 + 行:列 + Span）+ r30 FFI 双 case（编译面收窄[非字面量实参 CompileError] + VM 三码族[E0010 双重释放/E0011 非令牌释放/E0012 未登记符号 fail-closed]）
- **§21.3 四条件终验（P 桶）**：条件 1 = P06 生产链自编译 compiler.krf + 活性终验（sq(7)=49 与种子一致）；条件 2 = P03 门 B fixpoint（B₁/B₂ bytecode_equal + SHA-256 两件）；条件 3 = P04 QBE 本地码 fib(12) exit 144 端到端；条件 4 = **P05 FFI 真端到端**（write_stdout 经冻结 FfiCall → lowering → 操作码 → 窗口规程 → 宿主真实 I/O → Int(4)——r30 VM 面做实后，较 r1 的「模型冻结断言」升级为运行时实证）
- **§21.5 九信号全核对（P07）**：S1 语义稳定（金路径进程内实测）/ S2 自举验证（P03 证据引用）/ S3 测试覆盖（matrix 758:0:0 v0.1.0-r34 锚）/ S4 性能基线（CLI bench 锚）/ S5 文档同步（lang-design 24 文件实测 + stage0 v6.5）/ S6 能力处理程度（12 §2.5.1 终态注记锚）/ S7 技术债 P0/P1 清零（登记册机械扫描开放项 = 0）/ S8 外循环投票（协议承载——本审计集为证据输入）/ S9 阶段间深验证（K2 承载——如实登记）
- **§6.3 五角色投票**：ARCH-A 2 票 GO + DEV-A 1.5 票 GO + QA-A 1 票 GO + ALG-C 1 票 GO + SKL-A 1 票 GO（不参与加权）——**加权 5.5/5.5 = 100% ≥ 95% GO**（K1 验收合同达成；批次 K 输入条件进入 K2）

### 交付三：GATE 1 §3.2 六命令 clean 起步实测全绿

- cargo clean（589 files 166.5MiB）/ build --release 13.11s 零告警 / check --workspace 0 errors 0 warnings / fmt --check 0 diff / clippy --all-targets --workspace 超集 0 / **test --release --workspace 758:0:0（22 套件零断言修改——代码增量 = 审计器单件，零生产代码变更）** + **四审计集 EXIT 0 ×4**（stage0 41 + stage1 50 + stage2_r1 53 + stage2_r2 46）+ CLI 四路径（run fib ⇒ 75025 ⇒ 144 / run macros ⇒ (2 1) ⇒ 42 / check ok「2 原型/9 常量/5 全局引用/38 指令」/ check 负例 E0005「+ 需要数值，实际 str」REAL_EXIT 1）

## v0.4.0-r34（2026-09-14）——批间插入轮第三弹：设计缺陷深度审计收敛轮（55-a/55-z/55-web——用户审查指令驱动，设计栈治理层补齐 + 原则 35，758 维持全绿）

### 交付一：演进治理与设计收敛（会话 Task 55-a——「定位/权限/能力边界/职责边界/层级处理和管理模型/演进阶段和时机——网状发散、内循环直至收敛」）

- **驱动源**：用户对 r33 产物的第三轮质量反馈（「检索当前 docs/lang-design/ 下存在哪些语言设计上的缺陷面，定位是否具体和清晰，权限是否具体和清晰，能力的边界与具体是否具体和清晰，职责的边界与具体是否完善和清晰，层级处理和管理模型是否具体和清晰，演进阶段和时机是否具体和清晰等等——沿着思路向下继续思考确保思维深度，并网状发散性思维确保思维广度」「这是一个内循环迭代的过程，直至你确保所有可能性的收敛」+「kerf 不是 lisp 系列也不是 c/rust 系列——kerf 并不会限制你什么，限制的是差的设计、规划、理念等内容」）+ 知识搜索四查询实证（tool-results/r34-search/：WASI 无环境权威与组件组合 / Rust proc-macro 编译期任意执行安全缺口与 2026 沙箱化收敛 / Swift resilience 库演进治理 / ocap 组合布线）
- **新设计文件 `docs/lang-design/23-evolution-governance.md` v1.0（治理层——设计栈第四层：20 名字与行为 / 21 能力与职责 / 22 命名机制 / 23 演进治理）**：**十二审计轴系**（用户六轴 + 发散六轴：组合语义/覆盖完备性/一致性/诊断对账/判据先于先例/元编程工具链交互）+ **第一轮 14 项发现全表**（F1-F14 全落位）+ **演进六窗触发表**（K/L/M/E5/T3+——每窗入口信号全满足才开窗 + 出口条件 + 时间治理四红线：禁止时间驱动切换/处理程度倒挂/无信号开窗/窗口合并）+ **变更通道矩阵**（每类变更对象唯一通道 + 前置判据）+ **准入判据总表**（新原语/新域/新模块/新内置/新效应/新授权/新码位/新值型八类 checklist）+ **受控债务四步仲裁**（登记→等价证明→清偿窗口→深审复核——D5 set! 先例流程化）+ **生命周期四阶段**（引入→默认→弃用→移除——引入与移除永不同窗同面）+ **非锚定设计哲学**（判据先于先例的操作化）+ **第二轮复审收敛证明**（0 新 P0/P1 + 残余 7 项全持接口契约）
- **21-能力架构升 v1.1**：§2.5 **三轴坐标系**（语义 L0-L3 × 命名 N0-N4 × 相位 P0/P1——「L3′」记号退役 + L×N 交互矩阵交格 owner 单源表 + `=` 双轴归属）+ §3.1 **十一域覆盖闭合**（补算术运算符域[N1 永驻]/诊断终止域[error/assert-eq——终止 ≠ 可拦截裁定]/横切泛函域[prelude 五件]三卡——57 注册名 + prelude 五件每名有家）+ §4.4 **授权组合规则**（require 传递闭包：模块需求 = 元数据非授权获得，入口程序声明并集 + 编译期核对——WASI「无环境权威」同构）+ §4.5 **编译期权威边界**（Phase 1 零授权面 + 编译期权威三判据[确定性/零外部 I/O 或编译期令牌/展开可终止]——Rust proc-macro 反面教材 + 2026 沙箱化方向双实证）
- **22-命名空间设计升 v1.1**：§4.2 相位授权面注记（Phase 1 零授权接线）+ §5.1 权限矩阵增两行（模块授权需求声明[组合闭包] + Phase 1 宏命名宇宙）+ §8 实施对账增两行
- **原则 35「判据先于先例」三方同步**：先例是佐证不是权威；向后锚定（停 1970）与向前锚定（抄某家族）等价失败形态；引证纪律 = 先判据后先例 + 跨家族取样（≥3 范式家族）+ 否决记录保留（17 v6.5 / sop §2.2 v12.3 + 版本历史 / stage0 §23.1 v6.4 + §9.8 镜像）
- **同步轮 9 文件**：00 v6.5（文档地图 23→24 + 计数漂移修正 F7）/ 09 v6.6（十一域口径）/ 10 v6.4（LSP×命名空间三交互点注记 F10——补全/重命名/格式化）/ 12 v6.8（时机治理单源化注记）/ 18 v6.4（§5c +13 术语 + **§6 批次 M 码位预登记 E0013-E0019**[F9]）/ 19 v6.3（§8 +6 引用）/ 20 v1.2（四层设计栈 + §11 +5 维）/ 13 维持（无增量）/ sop v12.3 / stage0 v6.4
- **GATE 1 §3.2 六命令 clean 起步实测全绿**：cargo clean（600 files 175.4MiB）/ build --release 13.63s 零告警 / check 0/0 / fmt 0 diff / clippy 超集 0 / **test 758:0:0（22 套件零断言修改）** + 三审计集 EXIT 0 ×3 + CLI 四路径（run fib ⇒ 144 / run macros ⇒ 42 / check ok「2 原型/9 常量/5 全局引用/38 指令」/ check 负例 E0005「+ 需要数值，实际 str」EXIT 1）——**零代码变更轮**（设计文档 + tool-results 面唯一增量）

### 交付二：55-z 收尾（对账四面 + r34 tar.gz 包内自举 + web E2E + git 双仓库 + rec 树 18_r34/l + worklog 双源）

- r34 tar.gz（§19.3 commit-then-package 正序）+ 包内自举验证（全新解包构建 + 758 复跑 + CLI 一致 + 包内三审计集）+ web 面（kerf-data r34 节点 + download README r34 节 + E2E）——详录见 root worklog 55-z/55-web 条目

---

## v0.4.0-r33（2026-09-13）——批间插入轮第二弹：能力架构深度设计轮（54-a/54-z/54-web——用户审查指令驱动，lang-design 三层设计栈补齐 + 原则 34，758 维持全绿）

### 交付一：能力架构与命名空间深度设计双文件（会话 Task 54-a——「能力定位/边界/职责/正交 + 命名空间层级/权限控制 + 原语级 ≠ 重命名」）

- **驱动源**：用户对 r32 产物的质量反馈（「整体是否需要清晰的能力定位，能力边界，能力职责，能力正交，是否完整设计规划的命名空间，层级关系，权限控制……对原语级别的内容不是简单的重命名（之前存在相关讨论并已经在计划中推进重构设计）……不止是内置和命名方面」+「kerf 不是 lisp 系列也不是 c/rust 系列——限制的是差的设计、规划、理念」）+ 知识搜索八查询实证（tool-results/r33-search/：ocap 模型 / Pony-Austral 能力安全语言 / 效应组合性 / 模块≠命名空间 / 可见性谱系 / Clojure 命名空间组织 / 库核心-Stdlib 分层 / 最小 kernel 传统）
- **新设计文件 `docs/lang-design/21-capability-architecture.md` v1.0（能力架构——三层设计栈的「能力与职责层」）**：「能力」三义定锚（工程能力/语言能力/授权——r32 前三义混用即缺位证据）+ **语言能力四层 L0-L3 四要素卡**（值域/控制/效应/授权——每层定位/边界/职责/正交）+ **正交判据 J1-J4 各配既有实测锚**（r24 装箱值零控制层改动 = J1 / r25 原语集 9→11 零断言修改 = J2 / r8 门控授权零语义载荷 = J3 / D10 效应-授权正交 = J4——「正交是已验证的测试属性非口号」）+ 八库能力域四要素 + 域正交单向依赖三规则（D1 单向 / D2 零双向 / **D3 副作用汇聚**——「门控表 = I/O 域全集」的架构根据）+ **授权三态**（纯度态静态纪律 / 声明态程序头审查 / 令牌态逐值持有——粒度递进收紧）+ fail-closed 三红线（缺省拒绝 / 授权零语义载荷 / **import 不传播授权**）+ 对齐不变量 A 精确化（修正 20 §5.3「同一语义」过强表述——「边界对齐 + 机制分工两门」）+ **原语四要素卡 11 张 + 六类迁移分类学 M-R/D/I/L/E/A**（重命名/脱糖/索引化/层级迁移/效应化/新增——「重命名只是最浅一类」，E1-E5 既有裁定载荷的结构重构归档）+ 新原语准入两判据（不可归约性 + 单层归属）+ 2026 前沿对照（Pony 变量级 cap 不引入裁定——原则 26 成熟度匹配）
- **新设计文件 `docs/lang-design/22-namespace-design.md` v1.0（命名机制——三层设计栈的「命名机制层」）**：**五层命名层级 N0-N4**（符号宇宙/全局注册/模块/局部绑定/保留字——存在性与可见性分离）+ 解析优先序 R-N1（非限定：N3 内向外 → N2 注入 → N1 全局；限定：仅查 export 面**不回落**）+ 遮蔽许可表 R-N2（六行三分界——合法/警告/错误）+ 限定名词法域 R-N4 + 导入/别名规则 R-N5（别名 = N3 局部绑定）+ prelude 规则化 R-N6（无特权模块）+ 运算符族 N1 永驻 R-N7 + 符号宇宙豁免 R-N8 + **模块×授权权限矩阵**（11 行完整对账——v0.6 批次 M 实施单源表）+ 授权传播三红线命名面 + 冲突三类裁定 + `kerf/` 保留域 + 版本化接口位锁定 + 宏卫生/相位交互（E3 裁定机制面）+ 批次 M 实施对账表 12 行——**v0.6 命名空间层实施输入由此齐备**（此前 20 §5 仅方向概要，属「可推迟决策未配接口契约」违例）
- **20-表面规范升 v1.1**：§1.2 失实叙述修正（「内部已现代」→「方向已冻结、实施有窗口」——原语级迁移是 Stage 3 结构重构非已完成改名）+ §5 分工接线（本文件持方向概要与模块树，22 持机制规范——批次 M 输入以 22 为准）+ §5.3 对齐精确化 + §11 完整性核查 13→18 维（+5：能力四要素/原语分类学/命名机制/授权架构/术语消歧）
- **原则 34「能力正交与授权分层」三方同步**：17-principles v6.4 / sop.md §2.2 v12.2（+ 版本历史 v12.2 行）/ stage0.md §23.1 v6.3（+ stage0 §9.7 存档侧镜像）——正交判据思维吸收进 sop（「正交不是口号是已验证的测试属性——每条正交声明必须配实测锚」：层归属/正交声明/授权变更三类决策先过 J 判据）
- **lang-design 同步轮**：00 v6.4（文档地图 21→23 文件）/ 09 v6.5（§4 扩展指针）/ 12 v6.7（§2.10 批次 M 机制设计输入注记）/ 18 v6.3（§5b 增 14 术语 + §5a 对齐术语精确化）/ 19 v6.2（§7 增 8 条引用——知识搜索参考文献 66-73）/ 13 v6.2（「能力」三义消歧指针）
- **项目级清扫**（用户指令末条「全部更新后对项目系统性扫描」）：01 §8 无同类失实表述（复核通过）/ 代码注释零混用（复核通过）/ builtins.rs 头注 + 21/22 指针（唯一代码面变更——注释级零语义）

### 交付二：54-z 收尾（r33）

- §3.2 六命令全绿（758:0:0 维持——零断言修改；唯一代码变更 = builtins.rs 注释指针）+ 三审计集 EXIT 0 ×3（stage0 41 + stage1 50 + stage2 53）+ CLI 四路径（run fib ⇒ 75025 ⇒ 144 / run macros ⇒ (2 1) ⇒ 42 / check ok / check 负例 E0005「+ 需要数值，实际 str」EXIT 1）+ r33 tar.gz 包内自举验证 + web 同步 + rec 树 17_r33/l 两层 + git 双仓库

### 质量口径

- §3.2 全绿（本轮实跑）：build --release 5.35s 零告警 / check 0 errors 0 warnings / fmt --check 0 diff / clippy --all-targets --workspace 0 / **test --release --workspace 758:0:0**（22 套件全 ok——单元 215 + 集成 543）
- 三审计集 EXIT 0 ×3（144 case）+ CLI 四路径；TD 登记册零新增（TD-027 实施面不受影响——批次 L/M 输入经本轮升级）

### 下一步（批次 K 终批，plan §5c——零接触维持）

K1 终门审（stage2_gate_audit_r2 ≥30 新 case + 批次 J 修复边界 §7.3.2 + §21.3 四条件终验 + §21.5 九信号全核对 + §6.3 投票）→ K2 大阶段末深审环（§14.5 D1-D8 全量 + §14.8 B1-B4 + §14.9 + §14.6 阶段间四项 + final-assessment + Stage 3 切换 GO/NO-GO）→ K3 收尾（12 §2.5.1 终态回写 + v0.5-roadmap（批次 L 经 12 §2.10 预锚 + **本轮 22 §8 实施对账表升级**）+ tar.gz + web）。

---

## v0.4.0-r32（2026-09-12）——批间插入轮：表面现代化设计轮（53-a/53-z——用户审查指令驱动，lang-design 2026 现代化 + 原则 33 + TD-027，758 维持全绿）

### 交付一：库表面现代化设计规范（会话 Task 53-a——「实现端不要停留在 Lisp 家族 1970 年代表达」）

- **审查实测**（全项目扫描）：57 注册内置名中 18 件 `?` 谓词 + 3 件 `->` 转换 + `car`/`cdr` 历史访问器 + 扁平 `str-` 前缀 ×15 + 行为模式遗留（-1 哨兵 / 真值多态 / 元数缺口）；引导语料自铸 41 个 `?` 名 + 9 个 `->` 名；**此前无现代化 owner / 无映射表 / 无 TD 登记**（「未认领半区」——内部 ADT 已由原则 29-31 治理，库表面悬空）
- **新设计文件 `docs/lang-design/20-surface-conventions.md` v1.0**：R1-R6 六规则（全词 / `is-` 谓词前缀 / 动词关系裸形 / `to`/`from` 方向词 / 库面零 `!` 新铸 / 运算符族冻结）+ B1-B5 行为契约（`index-of` miss→nil / `member` 真值多态单一化 / `read-line` 严格元数 / 恒等元与 `eq` 语义**显式裁定保留**）+ 命名空间层（`/` 限定名 + `kerf/<模块>` 模块树七件 + **能力-命名空间对齐**：`kerf/io` ↔ `(require io …)`）+ **57 项完整映射表**（27 新别名 / 24 不变 / 4 引导私有跳过）+ 零破坏三批次迁移（v0.5 批次 L 别名层——别名层零语义变更不变量 / v0.6 批次 M 命名空间层 / Stage 3 移除轮——与 E5 关键字切换同窗）；§6 与既有裁定协调（核心冻结不受影响 / E5 适用域划分 / HM-BUILTIN_SIGS-门控三方同步契约）
- **原则 33「表面现代化动态演进」三方同步**：17-principles v6.3 / sop.md §2.2 v12.1 / stage0.md §23.1 v6.2（+ stage0 §9.6 存档侧镜像）——标点后缀约定 = 前类型系统时代补偿机制；动态演进思维模式吸收进 sop（每个引入新表面的设计时点先对照当期前沿）
- **lang-design 同步轮**：00 v6.3（文档地图 21 文件）/ 09 v6.4（§4 命名规范层指针 + 新语料现代名纪律）/ 12 v6.6（§2.10 迁移登记 + v0.5-roadmap 预锚）/ 18 v6.2（§5a 七术语）/ 02 注记（Reader 原语四件现代化方向）
- **TD-027 登记（P2）**：库表面命名现代化——设计 owner = 20-表面规范 / 实施 owner = TD-027（批次 L）；顺带实测对账修复：builtins.rs 头注 3→4 Reader 原语 + hm-inference-design 49→56 项
- **知识搜索前沿对照实证**（§2）：Swift API 设计指南（clarity at point of use / is 前缀 / 避免缩写）/ Rust `is_`/`to_`/`from_`/`into_` 方向语义学 / Kotlin `isX` / Clojure 命名空间标准库（Lisp 家族内部的现代答案）/ 2026 新语言代际（Mojo 开源 2026-08）——「Lisp 家族 `?` 后缀 vs 2026 跨范式 `is` 前缀共识」的跨语言判定

### 交付二：53-z 收尾（r32）

- §3.2 六命令 clean 起步全绿（758:0:0 维持——单元 215 + 集成 543，零断言修改；唯一代码变更 = builtins.rs 注释）+ 三审计集 EXIT 0 ×3（stage0 41 + stage1 50 + stage2 53）+ CLI 四路径（run fib ⇒ 144 / run macros ⇒ 42 / check ok / check 负例 E0005 检出 EXIT 1）+ r32 tar.gz 包内自举验证 + web 同步 + rec 树 16_r32/l 两层 + git 双仓库

### 质量口径

- §3.2 全绿（本轮实跑）：cargo clean（633 files 208.7MiB）/ build --release 零告警 / check 0 errors 0 warnings / fmt --check 0 diff / clippy --all-targets --workspace -D warnings 0 / **test --release --workspace 758:0:0**（单元 215 + 集成 543）
- 三审计集 EXIT 0 ×3（144 case）+ CLI 四路径；TD 登记册：TD-027 新增（P2——设计已冻结，实施批次 L）

### 下一步（批次 K 终批，plan §5c）

K1 终门审（stage2_gate_audit_r2 ≥30 新 case + 批次 J 修复边界 §7.3.2 + §21.3 四条件终验 + §21.5 九信号全核对 + §6.3 投票）→ K2 大阶段末深审环（§14.5 D1-D8 全量 + §14.8 B1-B4 + §14.9 + §14.6 阶段间四项 + final-assessment + Stage 3 切换 GO/NO-GO）→ K3 收尾（12 §2.5.1 终态回写 + v0.5-roadmap（含 12 §2.10 表面现代化批次 L 预锚）+ tar.gz + web）。

---

## v0.4.0-r31（2026-09-12）——批次 J 收口：J4 批次 J 收尾（48-e——§3.2 复验 + 12 §2.5.1 终态复核 + 对账 + r31 包，758 维持全绿）

### 交付一：48-e J4 批次 J 收尾（会话 Task 52-a——plan §5b 末 MUV，零代码收尾轮）

- **§3.2 六命令 clean 起步全量复验全绿**：cargo clean（350 files 156.2MiB）/ build --release **13.81s 零告警** / check 0 errors 0 warnings / fmt --check 0 diff / clippy --all-targets --workspace（超集口径）0 警告 / **test --release --workspace 758:0:0 维持**（单元 215 + 集成 543——零断言修改零语义变更）+ 三审计集 EXIT 0 ×3（stage0 41 + stage1 50 + stage2 53 = 144 case）+ CLI 四路径（run fib ⇒ 144 / run macros ⇒ (2 1) ⇒ 42 / check fib ok / check 负例 E0005「类型不一致：num 与 str」定位 1:24 EXIT 1——HM 判定面维持）
- **12-roadmap §2.5.1 行注记终态复核 ✅（plan §5b 排程注 4 兑现——v6.5）**：十一行批次 J 终态对账——三处更新（①Effect Handlers 行 48-b 静态收敛面 ✅ 兑现注记（r28/49-a：Perform 效应值 + Handle 两体入 R1-R8/HM 双检出域子表达式遍历，6 违例 case 双面检出 + 零误报对照——深审 D3/D8 覆盖缺口闭环）②字节码 VM+GC 行 FFI 所有权 GC 边界注记（r30/51-a——Heap Φ 计数簿 supp(Φ) 根集第五来源计数化 + ForeignBox 装箱/线性令牌不参与 GC 可达性；§21.3 条件 4 兑现）③类型检查器行批次 J 兑现位终态（默认期（R1-R8 内化）评估移交批次 K 终门审承载——§21.5 九信号全核对））；其余八行 r27 §5d 处置表已终态无需复改
- **rec 树补账（r30 会话遗留缺口——REC-A 纪律）**：14_r30 补建（r30 FFI VM 面做实 1:8 压缩条目）+ 15_r31 新建 + l 两层更新（树导航行 02 扩展至 r31 + 未压实区间刷新至 r31 终态 + flat 51-z 补录）
- **对账四面**：matrix r31 行（758 零增量维持，v0.1.0-r31）/ pipeline-test-coverage v0.4.0-r31（Tier 2 头 543 零增量维持）/ plan §5b J 执行注记 r31 + Status 行（批次 J 全收口）/ RELEASE_NOTES 本节；TD 登记册零事件

### 交付二：52-z 收尾（r31）

- r31 tar.gz（§19.3 commit-then-package 正序）+ 包内自举验证（全新解包构建 + 758 复跑 + CLI 一致 + 包内三审计集）+ web 同步（kerf-data r31 三节点 + footer v7.1 + download README r31 节）+ root worklog 索引 + git 双仓库

### 质量口径

- cargo test --release --workspace：**758:0:0 维持**（批次 J 终态基线——门审计集三件 144 case 另计全过）
- cargo clippy --all-targets --workspace：0 警告（超集口径）；cargo fmt --check：0 diff
- 环境口径：RUST_MIN_STACK=16777216（r29 既有环境级修复维持——深嵌套 case 线程栈）

---

## v0.4.0-r30（2026-09-12）——批次 J 执行：J3 FFI VM 面做实（窗口规程 Φ 簿 + 线性令牌 + E0010-E0012 + 13 边界 case，758 全绿）

### 交付一：48-d J3 FFI VM 面做实（会话 Task 51-a——plan §5b 第三 MUV，§21.3 条件 4 兑现）

- **操作码三指令 43→46**（CALL_EXTERNAL symbol, n_args / ALLOC_EXTERNAL size / FREE_EXTERNAL——04-bytecode-vm §1 第十组 + compiler.krf OP-* 常量表 + bootstrap_compiler.rs 桥三臂，三方冻结同步；**语言面形式 Stage 3，编译臂不发射**——IR/VM 面做实即验收条件 4 兑现，plan §5b 批次 J 排程注 3 如实注记）
- **窗口规程（ffi-ownership-model §2.1 步 2-4，边界包装层执行）**：装载边界（char* 实参装箱堆槽 pin `Φ[v]+=1` + 令牌实参校验 Invalid→E0010）→ 宿主调用（VM 挂起——§6 case 6 时序：外部 malloc 零 kerf 分配计数）→ **unpin 配平先于返回包装**（不依赖实参堆对象）；char* marshalling = Str 值装箱堆槽（GC 追踪形态）+ 数据指针借用出界（窗口借用——F-PIN 存活保证）
- **Heap Φ 计数簿**（kerf-runtime——`register_foreign_ref` 的计数化泛化）：`pin_object`（P1 可重复累计）/ `unpin_object`（U1 归零摘根；U2 下溢 Err——E8 口径）/ `pin_count`/`pinned_refs`（可观测性）；supp(Φ) 并入 mark 起点（F-PIN 引理——根集第五来源计数化，完备性不变式维持）；持久根与窗口计数根并存
- **线性令牌状态机（§5）**：`Value::External(Rc<ExternalToken>)`（CPointer/Opaque + Cell 状态——**消费全局生效**：一处 FreeExternal 后所有共享绑定同步失效）；装箱 `HeapObj::Foreign` 载荷（追踪器 no-op——不参与 GC 可达性，§7）；NULL 哨兵（Valid 空令牌——free = 合法 no-op，case 13）；外部域纪律 = Rust 宿主堆 Box<[u8]>（真 C ABI = QBE AOT 路径）
- **E0010-E0012 诊断族落位**（18 §6 码位登记 r18 预留兑现——E9 FfiTokenInvalid/E10 FfiOwnershipViolation/E11 FfiSymbolResolution）：messages.rs 单源三构造器（TD-018 纪律）+ VM 携码经 driver from_vm 映射；E0012 = 符号解析 fail-closed（QBE AOT 链接期解析在 VM 路径的调用期对应物）；空表默认入口（run_program）E0012
- **§21.3 条件 4 冻结形状消费面**：FfiBoundary 真实实现（driver ffi.rs HeapFfiBoundary——P1/P2/U1 经 Φ 簿；冻结签名 U2 经 expect E8 口径承载）+ compile_ffi_call_program（FfiCall → 操作码序——字面量实参子集 + AllocExternal size=0 编译期拒绝 + FreeExternal 运行时令牌依赖拒绝——诚实收窄）+ default_extern_table（write_stdout char* 注册——ExternalType 形状消费：CInt(Usize)/CPointer(Char)；CStruct/CFunction 子集外注册拒绝）+ run_program_with_externs（extern 注册执行入口）
- **正例端到端**：冻结 FfiCall → lowering → 操作码 → 窗口规程 → kerf_runtime 真实 I/O（CInt 返回 = 写入字节数）
- **ffi_vm_tests 37 case**：13 边界 case 全判定落地（覆盖映射表——case 1 F-PIN/case 2 绑定变更/case 3 CFunction 拒绝/case 4 双面拒绝/case 5 E0010/case 6 外部零分配/case 7 Opaque/case 8 浅 pin 等价/case 9 三容忍观测（错误路径泄漏 + panic unwind + 槽回收）/case 10 单线程注记/case 11 共享失效/case 12 非令牌/case 13 NULL no-op）+ 正负比 7:22（功能点粒度 ≥1:3，§9.4.3）+ Φ/P-U 机制组 6 件 + E0010/E0011/E0012 码断言
- TD-026 登记（P3）：FFI 语言面形式 + 编组消费子集收窄（CStruct 递归编组/CFunction 回调/FreeExternal lowering——显式拒绝非静默；Stage 3 锚）

### 交付二：51-z 收尾（r30）

- §3.2 六命令全绿 + 对账六面（06 §3 E0-E11 + 05 §3.1 Φ 簿/Foreign 令牌注记/根集五来源 supp(Φ) + 13 §3.3.4 实现锚（§21.3 条件 4 ✅）+ 04 §1 十组 46 + 18 §6 落位 + TD 登记册 TD-026 + plan §5b J 执行注记 r30）+ r30 tar.gz + 包内自举验证 + web 同步 + E2E

### 质量口径

- cargo test --release --workspace：**758:0:0**（713 零回归 + 净 45 = ffi_vm_tests 37 集成 + driver ffi 单元 8；门审计集三件 144 case 另计全过）
- cargo clippy --all-targets --workspace：**0 警告**（超集口径）
- cargo fmt --check：0 diff

---

## v0.4.0-r29（2026-09-11）——批次 J 执行：J2 HM 旗标期切换（`kerf check` 判定面 = HM 推断 + 契约重定义 + 双缺口修复，713 全绿）

### 交付一：48-c J2 HM 旗标期切换（会话 Task 50-a——plan §5b 第二 MUV，D8 阶段 2）

- **判定面切换**：driver.rs `check_source` / `check_source_recover` 两入口判定面 = `hm_check_program`（hm.rs 约束三段式——生成/求解/收集；D8 演进轨道阶段 2；**run 路径不触静态面，爆炸半径有界维持**）；R1-R8（typecheck.rs `check_program`）**退为回归基线断言**（测试面超集门参照侧——hm_inference_tests / effect_tests 双面检出纪律保留）；typecheck.rs / hm.rs / builtins.rs / driver.rs 四处头注旗标期角色注记同步（R4 文档随代码）
- **契约重定义显式登记（P0-2 兑现——hm-inference-design §2.3 旗标期节 + §3.7 阶段 2 实锚，文档 v0.1.0 → v0.2.0）**：保守性断言更新为「零类型不一致误报」口径（类型不一致即报——含运行期可存活的类型不一致）；**occurs/自应用误报面文档化为政策接受行为**（无限类型静态报错——运行期行为不承诺双向锚）；双向锚 `static_error_is_runtime_error` 范围重锚（超集门内维持 / occurs 面豁免——typecheck_tests 头注 + 函数 doc 同步）
- **断言重锚（5 处子串——语义/检出/定位不变，渲染措辞随判定面）**：`(int . int)` / `(int . nil)` / `(α0 → α0)`（HM 结构化类型渲染——R1-R8 域名 pair/procedure 的精确化）+ 元数区间格式（「过程参数数量不匹配：期望 1..1 实际 2」——内置元数 5 case）
- **切换实测双缺口当场修复（超集门/零误报门经生产入口实测纪律的兑现——PoC 离线语料未覆盖生产入口负例矩阵全貌）**：① hm.rs Ordering 臂未同步 TD-011 r24 字符串全序（`(< "a" "b")` 曾误报「字符串仅支持 =」——PoC r18 遗留旧口径，typecheck.rs 已在前）→ all_str → 合法返回 Bool；② hm.rs car/cdr 臂 `Ty::Nil` 误入保守跳过（`(car nil)` 曾漏检——注释声称「R5 口径之外的动态边界」为错误归因：Nil 是静态确定类型，运行期必然 E5）→ Nil 归入诊断臂（与 R1-R8 R5 负例矩阵对齐）
- **新增旗标期判定面测试组 3 件（typecheck_tests 25 → 28）**：增值面生产证据（`(f "s")` 约束传播检出「类型不一致」——R1-R8 静默面，判定面切换直接证据）+ occurs 豁免政策锚（`(cons x (f x))` 无限类型报出但不断言双向锚——契约政策行为）+ 双面修复锚（TD-011 零误报 ×2 + car nil 检出 + 双向锚维持）
- **12-roadmap §2.5.1 类型检查器行「迁移评估」结论 ✅（v6.4）**：HM 旗标期切换落地 + **自举内迁裁定 = 留 Rust**（INC8 三段口径——读+展开+编译 100% kerf 即「kerf ~80%」；类型检查器属静态分析面不在自举关键路径（INC3 实证 front_from_core 无 check），归 07 §3.4 Rust 保留面；默认期（R1-R8 语义内化）= Stage 2 末评估）
- 生产实证：`kerf check` 负例（`(define (f x) (+ x 1)) (f "s")`）→ `error[E0005]: 类型不一致：num 与 str` 精确定位 1:24（R1-R8 时代此程序静默——HM 增值面 CLI 可见）

### 交付二：50-z 收尾（r29）

- §3.2 六命令全绿（build 零告警 / check 0/0 / fmt 0 diff / clippy --all-targets --workspace 超集 0 / test **713:0:0**（710 零回归 + 净 3）/ 三审计集 EXIT 0 ×3 + CLI 四路径（fib ⇒ 144 / macros ⇒ 42 / check ok / check 负例 E0005 HM 面））
- 对账六面：matrix r29 行（710 → 713 + 汇总链 + 表体两行实测修正 24→28）/ pipeline-test-coverage v0.4.0-r29（Tier 2 头 503→506）/ plan §5b J 执行注记 r29 / RELEASE_NOTES r29 / 12-roadmap v6.4 / hm-inference-design v0.2.0；TD 登记册零事件（双缺口为当场修复非登记债）
- 环境注记：沙箱重置后测试线程默认栈不足（深嵌套 case 爆栈——基线 stash 复跑同型归因环境而非代码）→ RUST_MIN_STACK=16777216 环境级修复（同 r21 工具链重装同型环境恢复操作，worklog 登记）
- r29 tar.gz（§19.3 commit-then-package 正序）+ 包内自举验证 + web 同步 + E2E

### 质量口径

- cargo test --workspace：**713:0:0**（单元 207 + 集成 506；净 +3 集成；门审计集三件 144 case 另计全过）
- cargo clippy --all-targets --workspace：**0 警告**（超集口径）
- cargo fmt --check：0 diff
- 三审计集：stage0/stage1/stage2_gate_audit_r1 全 **EXIT 0**（复验）

---

## v0.4.0-r28（2026-09-11）——批次 J 执行启动：J1 效应 typecheck 收敛（Perform/Handle 子表达式遍历 + 静态负例组双面检出，710 全绿）

### 交付一：48-b J1 效应 typecheck 收敛（会话 Task 49-a——plan §5b 首 MUV）

- **typecheck.rs 效应臂收敛**（补深审 D3/D8「Unknown 放宽面」覆盖缺口——r25 设计性放宽的收敛位）：Perform 臂——效应值表达式入 R1-R8 检查域（子表达式遍历）；Handle 臂——handler 体与被保护计算体均入检出域 + payload/resume 绑定器以 Unknown 装订（遮蔽纪律与 Lambda 臂 save/restore 同型镜像）；**结果类型维持 Unknown**（Perform 值 = resume 注入的任意值 / Handle 值 = 体汇合动态结果——行多态属 Stage 3 类型层，effect-language-design 静态面不收紧裁定维持）
- **hm.rs 效应臂同口径收敛**（HM PoC 域内违例检出）：Perform 效应值进推断（约束集检查）；Handle 两体进推断 + 绑定器 Binding::Mono(Dynamic) 装订（infer_let save/restore 同型）；**结果类型维持 Dynamic**（effect-language-design 风险表「与 HM 推断的效应行交互 = P3/Stage 3」——不收紧、不误报）
- **静态负例组 4 测试（6 违例 case，双面检出——超集纪律：tc 面报 → hm 面亦报）**：Perform 效应值违例（R2 算术混串 + R1 if 非真值）+ handler 体违例（R5 car/cdr 非 pair）+ handle 体违例（R1）+ 双体多错误收集（handler 体 R2 + handle 体 R1 → Span 序合并）；**零误报对照**：r25 六正例 + 绑定器动态用点（payload Unknown/Dynamic 算术与点对）+ effect_stress.krf M5 语料——双面 0 诊断（保守契约维持）
- 生产入口实证：`kerf check` 对 `(handle log ((p k) (car 42)) 1)` → `error[E0005]: car 需要 pair，实际 int` 精确定位 handler 子句体 1:25；合法效应程序 → `ok`
- 改动面：≤3 文件（typecheck.rs + hm.rs + effect_tests.rs——深审 D1 预估口径内）；编译面零改动（bootstrap_compiler_tests 效应 parity 维持 32/32 零回归）

### 交付二：49-z 收尾（r28）

- §3.2 六命令 clean 起步全绿（build 13.32s 零告警 / check 0/0 / fmt 0 diff（一处排版当场 apply）/ clippy --all-targets --workspace 超集 0 / test **710:0:0**（706 零回归 + 净 4）/ 三审计集 EXIT 0 + CLI 四路径（fib ⇒ 144 / macros ⇒ 42 / effect_stress ⇒ 120 / io ⇒ 42）+ check ok）
- 对账六面：matrix r28 行（706 → 710 + 汇总链）/ pipeline-test-coverage v0.4.0-r28（**R4 修正：Tier 2 头 463→503——r26 补账声称「499」未落面，本轮回写实测值**+ effect_tests 行 17→21）/ plan Status r28 + §5b 执行注记 / RELEASE_NOTES r28 / TD 登记册零事件 / worklog 双层 + rec 树 12_r28
- 深审 D3/D8 闭合注记（deep-review-round1.md——「批次 J 引入效应 typecheck 时补静态负例组（≥3）」建议兑现）
- r28 tar.gz（§19.3 commit-then-package 正序）+ 包内自举验证 + web 同步 + E2E

### 质量口径

- cargo test --release --workspace：**710:0:0**（单元 207 + 集成 503 逐二进制实测；净 +4 集成；门审计集三件 144 case 另计全过）
- cargo clippy --all-targets --workspace -- -D warnings：**0 警告**（超集口径）
- cargo fmt --check：0 diff
- 三审计集：stage0/stage1/stage2_gate_audit_r1 全 **EXIT 0**（复验）

---

## v0.4.0-r27（2026-09-11）——批次 J 规划轮：Stage 2 收官批细化 + 余下面处置（§5b/§5c/§5d，零代码轮 706 全绿）

### 交付一：批次 J 细化（48-a——plan §5b/§5c/§5d 三节）

- **§5b 批次 J 四 MUV（Stage 2 收官主体批）**：48-b J1 效应 typecheck 收敛（typecheck.rs/hm.rs Perform/Handle 臂**子表达式遍历**——补深审 D3/D8「Unknown 放宽面」覆盖缺口：效应体内 R1-R8 违例当前不诊断（typecheck.rs L353-357 实锚——`{ .. }` 早退不递归）；结果类型维持 Unknown——行多态 Stage 3 边界如实；负例组 ≥3）+ 48-c J2 HM 旗标期切换（**D8 阶段 2 GO 裁定落定**——深审 D4「早期决策点 / 防 PoC 长期化」兑现；切换面 = check_source L563 + check_source_recover L626 两入口（run 路径不触静态面——爆炸半径有界）；契约重定义显式登记（P0-2 兑现：保守性断言更新 + occurs/自应用误报面文档化 + 双向锚重锚）；类型检查器行「迁移评估」结论随轮登记）+ 48-d J3 FFI VM 面做实（write_stdout char* 按 ffi-ownership-model §8 窗口规程（Str 堆槽 pin + 借用出界 + 返回后 unpin）+ AllocExternal/FreeExternal 开放 + E9/E10/E11 落位 E0010-E0012 + 13 边界 case 对齐 + 回写四处——§21.3 条件 4 完整版兑现）+ 48-e J4 收尾
- **§5c 批次 K 概排（Stage 2 终批，49-x）**：49-a 终门审（stage2_gate_audit_r2 ≥30 新 case + 批次 J 三修复边界 + §21.3 四条件终验 + **§21.5 九信号全核对**）+ 49-b 大阶段末深审环（§14.5 D1-D8 全量 + §14.8/§14.9 + §14.6 阶段间深验证四项 + final-assessment Stage 2 版 + **Stage 3 切换 GO/NO-GO**）+ 49-z 收尾（12 §2.5.1 终态回写 + v0.5-roadmap Stage 2 行）
- **§5d Stage 2 余下面处置表（12-roadmap §2.5.1 十一行逐行落位——规划轮内闭环）**：已兑并行五（Token/CodeValue 增量/元循环/闭包/VM+GC PoC）+ Effect（r25 + 48-b 补静态面）+ **多阶段与缓存增量两行 DEFER Stage 3**（§21.3 不含 + 成熟度研究前沿/收益边际——矩阵自述「可依据当时成熟度重新评估」结构启用）+ **net/process 触发式**（net 窗口核对四依据：效应依赖 ✅ + 零消费 + 沙箱网络受限 + D11 手术面收敛——「预留长期不做实是允许的」条款）+ 宏完整化改判 Stage 3（在用宏面零缺口）+ 类型检查器行 48-c 兑现位
- **TD 登记册排期改判三行**（等级/状态不动）：TD-003/TD-015 → Stage 3 优化窗口 + TD-005 → Stage 3 宏增强窗口（R6/R7 收敛纪律；批次 J 实施发现消费面则回判）

### 交付二：48-z 收尾

- §3.2 六命令 clean 起步全绿（build 14.41s 零告警 / check 0/0 / fmt 0 / clippy --all-targets --workspace 超集 0 / test **706:0:0** / 三审计集 EXIT 0 + CLI 四路径 + check ok）
- 对账六面：matrix r27 行（零计数增量）/ pipeline 零增量（无代码变化——r26 口径维持）/ plan Status r27 / RELEASE_NOTES r27 / TD 登记册 r27 事件（三行改判）/ worklog 双层 + rec 树 11_r27
- 12-roadmap v6.3（九行处置注记回写）+ r27 tar.gz（§19.3 commit-then-package 正序）+ 包内自举验证 + web 同步 + E2E

### 质量口径

- cargo test --release --workspace：**706:0:0**（零代码规划轮——r26 基线零回归；门审计集三件 144 case 另计全过）
- cargo clippy --all-targets --workspace -- -D warnings：**0 警告**（超集口径）
- cargo fmt --check：0 diff
- 三审计集：stage0/stage1/stage2_gate_audit_r1 全 **EXIT 0**（复验）

---

## v0.4.0-r26（2026-09-11）——批次 I 收口：I3 门审查 + 收尾交付（门审计集 53 case APPROVED + 深审五角色全票 GO，706 全绿）

### 交付一：I3 门审查（42-g / 47-a——plan §5a 合同逐项兑现）

- **门审计集第 3 件 stage2_gate_audit_r1（53 case，examples/audit/——§7.3.1 规则 3 可重运行口径）**：A12 单语句负向（基础类型系统——含效应类型面）+ B12 多语句负向（糖九件/module/require/宏/**能力门控 E0006 编译期静态拒绝**（read-line 未授权 → Compile 阶段「能力权限不足」——比 Run 未绑定更强的 fail-closed 形态，实测如实升级断言））+ C10 复杂负向（⑥循环依赖 + ⑦宏深度 + E0007 逃逸深链 + E0008 逃逸续恢 + E0009 元数 + GC 压力 × 类型错 + module 体 inline 运行错）+ D6 错误恢复（含 E0007/E0008 后效应金路径恢复——VM 效应状态无泄漏实证）+ **E7 上轮修复边界（§7.3.2）**：门 B fixpoint 活性（B₁ install 后编译执行 = 种子）/ 缓存 CompilerKind 分桶（生产命中 + 种子不查不存条目数不变）/ E0008 首次恢复位置追踪 / TD-014 专门归因（无失真兜底回退）/ TD-011 全序正负双面 / TD-018 消息单源（if 直用 = cond 脱糖同文）/ recover 多错误收集（46-z 死代码清偿边界）+ **P6 正向含 §21.3 四条件锚定探针**（fib 144 双路径 / prelude 管道 50 / 门 B 轻量终验 B₁/B₂ bytecode_equal + SHA-256 / QBE native fib exit 144 / FFI 模型冻结文档锚 / TCO × 效应 10 万深 ⇒ 100）；run_negative 显式 E 码字段扩展（E0007/E0008/E0009 结构化族）+ 双路径同 Err 机械校验；实测 **53/53 PASS EXIT 0 —— APPROVED 零新发现**（七类覆盖 1/4/2/3/12/1/2 + §7.3.1 配比全过 + §7.3.2 边界 7 ≥ 5）
- **§14.5 深审报告（docs/develop/v0/stage-2/deep-review-round1.md——Stage 2 首篇）**：D1-D8 八维度三段式（§0 如实定位：批次 I 末门审深审，非大阶段末——Stage 2 终批另做 §14.6）+ §14.8 B1-B4 偏差清单（闭环 8 行对照记录 + B1 残留 3 行如实：HM 生产切换（D8 旗标期）/ FFI 实现（模型冻结）/ net 门控行（裁定维持——Stage 2 末窗口）——均纳后续批次计划）+ §6.3 五角色投票 **5.5/5.5 = 100% ≥ 95% GO** + 行动计划 W1-W5
- **§14.9 C1-C6 代码整理（发现即修 4 处）**：clippy --all-targets --workspace 超集口径抓出审计器 2 处 result_large_err（闭包 Result 大 Err 链 → for 循环收集重写）+ hm.rs 2 处 `_ => {}` catch-all 无注释（§14.6.1.1「无注释视为违规」→ 补「_ 臂理由」：free_vars 封闭类型臂 + 内置签名 fall-through 臂）；C2 glob re-export 0 / C3 TODO 0；全链复跑 fmt 0 diff + clippy 0 + **706:0:0 零回归**

### 交付二：42-h 收尾（47-z）

- §3.2 六命令 clean 起步全绿：build --release --workspace 13.56s 零告警 / check 0 errors 0 warnings / fmt 0 diff / clippy --all-targets --workspace 超集口径 0 / test --release --workspace **706:0:0** / 三审计集 EXIT 0（stage0 + stage1 + stage2 新）+ CLI 四路径冒烟（fib ⇒ 144 / macros ⇒ 42 / effect_stress ⇒ 120 / prelude foldl ⇒ 21 + check ok）
- 对账六面：matrix.md r26 增量行（零 cargo 计数 + 审计集 144 case 三件注记）/ pipeline-test-coverage v0.4.0-r26（**滞后两轮补账**——r24 +14 / r25 +22 表体行 + catch-all 小节 r26 实测）/ plan.md Status r26 / RELEASE_NOTES r26 / TD 登记册零事件 / worklog 双层 + rec 树 10_r26
- r26 tar.gz（§19.3 commit-then-package 正序）+ 包内自举验证 + web 同步 + E2E

### 质量口径

- cargo test --release --workspace：**706:0:0**（单元 207 + 集成 499——r25 基线零回归；门审计集三件合计 144 case 另计）
- cargo clippy --all-targets --workspace -- -D warnings：**0 警告**（超集口径）
- cargo fmt --check：0 diff
- 门审计：stage2_gate_audit_r1 53/53 PASS **EXIT 0（APPROVED）** + 深审五角色 100% GO

## v0.4.0-r25（2026-09-11）——批次 I 执行：I 后段双主题（Effect M1-M5 语言级效应全落地 + 能力管线泛化 M2，706 全绿）

### 交付一：Effect M1-M5（42-f 主体——effect-language-design v1.1 全裁定兑现）

- **原语集 9→11**：`perform`/`handle` 两核心形式入 CoreExpr（`resume` 非原语——展开期脱糖为 `(κ v)` App，D4）；Keyword +3；fresh scope 深注入 handle 子句（绑定器 + handler 体——TD-004 口径）；核心形式冻结证明 12 实例（architecture_audit）；自举 expander.krf 三展开臂 + 桥两向（parity 全链）
- **VM ext1 具体化（D6）**：`FrameExt.ext1 = Option<Rc<HandlerFrame>>`（分派键 tag/handler 原型/捕获单元/数据栈水位——原则 27 预留槽位兑现）；`INSTALL_HANDLER{handler, body, tag, trampoline}`（41）+ `PERFORM`（42）两指令（43 项三方冻结——opcode/测试/04 文档）；**三原型帧编排**（T trampoline 惰性单例 code=[Ret] / H handler 原型 params=[payload, resume] / B body thunk 体= lambda 体语义尾位穿线）——编译器零 CLOSURE/CALL 发射（帧编排原子化）
- **continuation 四要素**：捕获帧链（**含 handler 帧本身**——快照 ext1 清除保 D2 浅处理）+ 数据栈快照（VM flat 栈整栈还原——设计三要素之外的实现必要补充）+ 恢复点 + 线性唯一性（consumed Cell 跨帧共享——E0008 含首恢位置追踪）；resume 控制转移语义（Call/TailCall 的 Continuation 臂——当前 H 执行帧拆除）
- **诊断族 E0007-E0009（D9/M4）**：messages.rs 单源（TD-018 纪律）——E0007 逃逸（tag 名经符号值文本直渲染）/ E0008 二次恢复（首恢位置）/ E0009 元数（非 continuation 值面归 E0004 通用族——v1.1 执行注记口径）；driver `from_vm` 码映射（None→E0004）
- **GC 六来源（M5）**：`Value::Continuation` 根集递归（帧链 locals/captures + 数据栈快照——visited 防嵌套环）+ `GcCell` has_heap 判据扩展 + ForeignBox 装箱/解箱（往返恒等——TD-010 协议复用 + 专用追踪器）
- **M3 双路径新口径**：resume 一致性 T1 承载面 = 种子/生产双编译链（42-d eval 退役终态）——双路径 8 case；eval 域（树走）经 `EvalError.effect` 逃逸通道承载 dispatch（D7）+ resume 哨兵域限（显式报错——`call_closure` 拒绝 Eval 闭包同型先例）
- **native 显式拒绝（B1 同型）**：anf/qbe 路径 perform/handle 报 PoC 边界错（效应语义由 VM 承载——Stage 3 后端演进评估项）
- **回写 W1/W3/W4/W5**：01-core-forms §7.3（原语集 9→11 状态翻转 + §8.3 映射行）/ 18-terminology §6（诊断码位登记表 E0001-E0012 全族总账）/ 06-operational-semantics（R10/R11 归约规则 + continuation 值域 + 根集六来源）/ 04-bytecode-vm（43 项九组 + ext1 具体化 + INSTALL_HANDLER/PERFORM 语义）+ effect-language-design v1.1（九条执行注记——数据栈快照/三原型/快照含 handler 帧/E0009 口径/M3 新口径等）
- **测试**：effect_tests 17（设计锚正 6 + M3 双路径 8 + eval 域 3 + 负例 7 + GC 存活 2）+ bootstrap_compiler_tests 门 A 效应组 3（parity 基础/trampoline 共享/行为面）——**706:0:0 零回归 + 净 22**；效应 GC 压力语料 `examples/usage/effect_stress.krf`（万级分配下挂起链存活 ⇒ 120）

### 交付二：能力管线泛化 M2（capability-model-design §7 M2 别名兼容路径）

- **`IoFamily` 形状标记**：`IoGrant` = `Grant<io 族>` 的 trait 形态承载（`CapabilityModelFamily` 关联类型别名兼容语义——冻结路径零删改，原则 27）；归位证明测试（族形状 + Token 关联类型等式 + 授权面回读）
- **门控表 net 增行评估**：依赖条件①效应系统成熟（12 §2.4.5）已就位（本批 Effect M1-M5）+ ②手术面三点加法（D11）；评估结论**维持不增行**——net 语音面属 Stage 2 末窗口（I3 门审后；零破坏纪律：既有门控表/授权管线/require 语法零改动）；`net_gate_rows_unchanged_in_42f` 机器锚
- **D12 同轮协调**：driver 组合根双接触面（Effect ext1 激活 + M2 族形状归位）经 worklog 交叉引用锚定（capability-model-design v1.1）

### 交付三：对账与收尾（46-z）

- 文档同步：matrix v0.1.0-r25（684→706；effect_tests 17 行 + bootstrap 29→32 + 集成 header 499）+ plan.md Status（42-f 交付——I 后段闭环）+ 18 §6 码位登记 + 04/06/01 lang-design 回写 + 两设计文档 v1.1 + compiler.krf 头部契约（节点协议 + OP 表 0..=42）+ examples/README（effect_stress 行）+ TD 登记册（TD-024 效应边界注记）
- **§3.2 六命令全绿**（clean 起步终验）+ r25 tar.gz（§19.4）包内自举验证 + web 同步（kerf-data r25）+ git 入账 + rec 树压实

## v0.4.0-r24（2026-09-11）——批次 I 执行：I2 stdlib/GC/TD 批（五债清偿 + 谓词/foldr 补齐 + TD-023 根扫描对症，684 全绿）

### 交付一：TD 五项清偿（42-e 主体）

- **TD-010 闭包/内置装箱 resolved**：`HeapObj::Foreign(Rc<ForeignBox>)`——类型擦除 Rc 载体（解箱往返恒等 → eq? 按引用）+ **追踪器协议**（`ForeignTracer = fn(&Rc<dyn Any>, &mut Vec<GcRef>)` 由装箱方注入——标记阶段枚举闭包捕获图，kerf-runtime 不依赖 kerf-vm 类型，§11 接口隔离）；原「标记字符串占位」路径删除；渲染 `#<procedure>`/`#<builtin:名>`；**函数列表模式就位**（`(list f g)` + `map` 应用——stdlib 库化前提）
- **TD-011 字符串全序 resolved**：全字符串链按 **Unicode 码点序**参与全部比较族（`Rc<str>` 比较 = UTF-8 字节序 = 码点序——编码保序性）；混合链保持 `{op} 需要数值`（TD-016 口径）；静态面 R3：`Ordering ≡ NumOrAllStr`（全字符串链放行——运行时/静态一致）；HM 超集门语料同步
- **TD-014 嵌套 define 归因 resolved**：专门消息「嵌套 define 重复绑定（同名内部变量只允许出现一次）」+ Span = 第二次出现处 define 形式自身（原「lambda 参数重名」提升兜底——归因失真 + Span 指向体首合成节点）；seed `expand_body` + 自举 `expand-body`/`dup-define-scan` 双侧镜像（判定序一致）；parity_err +2 case（消息 + Span 逐字）
- **TD-018 消息单源 resolved**：`kerf-vm/src/messages.rs` 单源构造器（if 条件 / not / car·cdr / set! 未绑定五族——VM 操作码 + eval 参考臂 + driver 内置三消费面同源）；**`Value::truthy` 复活为单一实现**（原无调用方死助手 → `Result<bool, RuntimeError>` 化——VM `JumpIfFalse` 与 eval if 臂同文；scope_set_tests 对拍回归断言消息文本相等）；前缀统一「if 条件需要 bool」（与静态面 R1 一致）；未绑定族裁定保留（阶段信息差异）
- **TD-023 根扫描对症 resolved**：**基准重定型先行**（新增 `gc_stress_nontail.krf` 非尾形锚定——改前 144.06ms/轮 + 2×→3.2~3.7× 超线性实测成立）；**对症双件**：① `GcCell` 堆根性摘要（`Rc<RefCell<Value>>` → `Rc<GcCell>`——`has_heap` 标志由写路径维护（sound 不变式：每写必置），根集枚举对非堆单元 O(1) 跳过）② 根扫描缓冲跨周期复用（root Vec + visited HashSet——take/归还零 API 变更，无分配化路径 B）；**对拍实测（stash 重建 r23 二进制同会话）**：非尾形 **144.06→105.15ms（-27.1%）**；尾形 gc_stress 39.68→38.57ms（-2.8%）+ fib(25) 86.97→84.22ms（-3.1%）——双噪声带（验收 ≤5% ✓）；残留超线性如实归因 = 帧栈内存 churn + 每周期固定成本（精确 MS 栈根扫描的结构性成本）；写路径 soundness 回归锚 `gc_cell_flag_flips_on_pair_write`
- **TD-008 分代 GC/堆压缩裁定 DEFER（Stage 3+ 条件触发）**：实测依据三面——①收益面不存在（Stage 2 无长驻程序：CLI 单趟/测试/自举管线，分配有界；尾形 38.57ms/非尾形 105.15ms/分配主导 23.76ms 全过验收门）②分代对栈根扫描无通用免除 + 压缩破坏 GcRef=槽位索引契约（转发表 = P1 级全量改写）③复杂度预算（§12——42-f Effect/M2 + 42-g 门审查优先）；重评估触发条件入册

### 交付二：stdlib 缺口补齐（清单清零——09-stdlib v6.3）

- **类型谓词 5 件**：`string?`/`symbol?`/`float?`/`number?`（数值塔域 Int∪Float）/`list?`（真表判定 = nil 或 cdr 链终止于 nil——**Floyd 龟兔环安全**，环 → false）；BUILTIN_SIGS 同步（TcParam::Any → Bool）——Value 变体判别完备面（57 项清单）
- **prelude `foldr`**：foldl 对偶（从表尾累积 `(f 首元素 递归果)` 形态——与 Racket 同序）；prelude_tests +3（用户面/对偶语义可观测（foldr 2 vs foldl -6）/双路径）
- **缺口盘点口径（三面）**：①文档合同面 52/52 对齐（数量+逐项）②值模型谓词完备面（5 缺 → 补齐）③prelude 库化对称面（foldr 缺 → 补齐）——**I2 stdlib 缺口清单清零**

### 交付三：对账与收尾（45-z）

- 文档同步：09-stdlib v6.3（52→57 项 + 比较行重写 + prelude foldr）+ TD 登记册（五 resolved + 一裁定 + 索引/详情/Status）+ performance-baseline §4.1（r24 复测四口径对拍表）+ §9.1 复测记录 + §10 热点行 resolved + matrix v0.1.0-r24（670→684；表体 gc 9/stdlib 24/scope_set 10/prelude 10）+ pipeline 性能小节 + plan.md Status（42-e 交付）+ 本 RELEASE_NOTES
- **§3.2 六命令全绿**（clean 起步终验）+ r24 tar.gz（§19.4）包内自举验证 + web 同步（kerf-data r24）+ git 入账 + rec 树压实

## v0.4.0-r23（2026-09-11）——批次 I 执行：I1 收口（生产切换 + 门 B fixpoint 两次编译自身字节一致 + eval 退役终态，670 全绿）

### 交付一：CompilerKind 生产切换（42-d 主体，S3 段——P1/P2）

- **`CompilerKind::{Bootstrap,Seed}` 分派**（driver.rs——镜像 `ExpanderKind` 先例）：分派位 `front_from_core` 第 4 步——生产（`compile_front`）= 自举 Compiler（compiler.krf 在 VM 上运行），种子（`compile_front_seed`）= Rust Compiler（bootstrap 加载引导 + parity oracle）；**bootstrap init 恒种子路径**（P1 硬约定——无递归）；`check_source_recover` 恢复路径同生产口径
- **守护测试 `production_compiler_is_bootstrap`**（P2——独立线程活性探针双信号：生产编译后 is_loaded 翻转 + 种子路径不加载自举 Compiler（引导恒种子的实测面））
- 三自举模块新增 **`install_state`/`reset_state` 钩子**（门 B fixpoint 的 B₂ 轮「以 B₁ 为新 bootstrap 程序」语义——状态构造提取为共享 `build_state` 单一实现）

### 交付二：门 B fixpoint——两次编译自身字节一致（§21.3 条件 2 终验）

- **`gate_b_fixpoint_two_self_compiles_byte_identical`**：B₁ = 生产链（自举读+展+编）编译自举三件 + preamble；B₂ = 以 B₁ 产物为新自举状态（三件 install）再编译同源；**硬门判据全过**：B₁[i]/B₂[i] 四程序 `bytecode_equal` 全结构一致（含 debug_spans）+ **SHA-256 摘要一致**（`BcProgram` Debug 结构序确定序列化——`sha256_hex` 自研 hash.rs 零新依赖）；隔离纪律 §7.4（关缓存 + fresh 状态——缓存命中假阳性防线）
- **加强判据（非硬门）实测两裁定（GATE 1 诚实入档）**：① B₀（种子链）vs B₁（自举链）**按名反汇编全等**（四程序——结构 + 名 + span 位置；E1 边界 expansion_id 不参与）；② 宏自由件（compiler.krf）**跨链全结构 bytecode_equal 实测不成立**——两链符号表各自 intern 序不保证一致（差异仅 Symbol 数值——i1-design §7.3「名字是唯一稳定口径」预判的实证；全结构判据在同链 B₁/B₂ 下成立）
- **`gate_b_b1_programs_execute_as_bootstrap_chain`**（行为面）：B₁ 字节码作为自举 Compiler 实际运转——install 后编译的程序 VM 执行 = 种子链结果（「产物编译自身」不止字节一致，可执行性同证）

### 交付三：eval 退役终态裁定（P5/INC7）+ 缓存键分桶（B11/P4）

- **eval_source 从生产路径退役**：CLI `eval` 子命令移除（退役提示 + 指向 run）；driver pub API 移除（`eval_source`/`resolve_eval_hygiene_fallbacks`/`collect_global_refs` 删除）；kerf-vm eval.rs **存档为 Rust 参考实现**（scope_set_tests 语义 oracle 消费面保留——12 §2.5 行 299 终态回写）
- **T1 双路径互查新口径**：`run_source`（生产链）vs **`run_source_seed`**（种子链公共参考入口——新增 pub API）+ `compile_source_seed`（fixpoint B₀ 基准）；T1 测试面全量迁移（stage0/1 十文件 + common + 双审计集——eval 侧断言迁移为种子链对拍/编译 parity 断言，§9.4.3 断言迁移非删除）
- **TD-017/TD-009 联动注销**（INC7 清单）：TD-017 resolved（256 深度上限域随 eval 退役注销——深尾递归生产路径经 TCO 无上限，`deep_tail_recursion_production_path_tco` 重写口径）；TD-009 resolved（eval GC 根集域随路径注销——生产 run 路径根集完整）
- **缓存键 CompilerKind 分桶**（B11）：`cache_key(source, filename, kind)`——维度入 config_fingerprint 构成层（CacheKey 冻结字段结构不动）；种子路径不经缓存（不查不存——`pipeline_seed_path_cache_isolated` 集成实测）+ 键区分 + 跨桶隔离双测试

### 交付四：对账与收尾（44-z）

- 文档同步六面：plan.md Status（I1 收口交付 r23）+ i1-design v1.3（S3 执行注记——跨链符号值实测裁定 + 门 B 首跑全过 + 缓存分桶实测）+ matrix v0.1.0-r23（665→670；单元 202→205 + 集成 463→465；表体 driver 单元 58→67 实测修正（r12 时代陈旧数）+ bootstrap_compiler_tests 27→29）+ pipeline v0.4.0-r23 + TD 登记（TD-017/TD-009 resolved）+ 12-roadmap §2.5 行 299 终态回写 + 本 RELEASE_NOTES
- **§3.2 六命令全绿**（clean 起步终验）：build --release 12.84s 零告警 / check 0 errors 0 warnings / fmt 零 diff / clippy --all-targets -D warnings 0 / **test --release --workspace 670:0:0**（665 基线零回归 + 净 5：单元 +3（守护 + 缓存 ×2）+ 集成 +2（fixpoint ×2）；23.9s）；CLI 冒烟（VM fib 75025/144 + macros `(2 1)`⇒42 + io 门控端到端 + check ok 38 指令 + native fib exit 144（print-free PoC 边界先例口径）+ E0006 负例 fail-closed + **eval 退役提示 exit 2**）+ 双审计集 EXIT 0
- r23 tar.gz 打包（§19.4 命令）+ 包内自举验证 + web 同步（kerf-data r23）+ git 入账

## v0.4.0-r22（2026-09-11）——批次 I 执行：I1 中段糖/module/require 面全迁移（门 A 扩展组全臂 parity，665 全绿）

### 交付一：compiler.krf module/require 两臂迁移（42-c 主体，S2 段）

- **module 臂**（`compile-module-body` + `compile-module-seq`）：镜像 compile.rs:416 逐语义——体 inline 编译**逐项恒非尾位**（区别于 begin 末项继承尾位：module 项不构成尾位——TCO 不参与）+ 中间值 Pop 携项自身 Span + 空体 PushNil 携 module 自身 Span + 首原型名覆写 `'<main>'` **值等价无操作**（Rust `pm.name = Symbol(u32::MAX-1)` 与 proto 0 创建/回写值恒等——设计 v1.2 注记）
- **require 臂**：PushNil（r8 能力声明零字节码语义——顶层「每形式一值」栈不变式维持；权限验证在 driver 前端 R9/E0006 完成——编译段零感知与种子同口径）
- 42-b 显式边界错误两处移除——**边界不对称消除**（生产切换点 CompilerKind 属 42-d）；fvo 面 42-b 已预置（module → 体遍历 / require → 空——与 kerf-core expr.rs:358/365 一致），本轮零改动通过

### 交付二：门 A 扩展组 parity 套件（42-c 验收面，19 → 27 测试函数 / 净 +8）

- **parity 扩展组 46 case**（≥12 超额）：module/require 正例组（多项体 Pop + 导入/导出面 + 尾位非继承 + 确定性双跑）/ 糖九件（let 家族 8 + cond/when/unless 9 + and/or/while 10——**全管线 parity**：expander 脱糖 → 双编译路径 bytecode_equal）/ 宏语料 3（swap!（let+set! 混合）+ my-or（递归省略号）+ def-twice（begin 多模式））/ **prelude 注入序**（preamble.krf 全文真实语料——生产前端 import kerf-prelude 实际注入的编译对象 + 用户 module import 面同编）/ **examples/usage 全六件双路径 bytecode_equal**（fib/closures/higher_order/macros/gc_stress/io——require/宏/GC 压力/高阶函数全谱系）
- 边界断言改写：42-b 的 `parity_err_module_require_boundary`（module/require 自举报边界错误）改写为 `parity_module_require_arms` 正例 parity + 确定性断言（生产切换守护属 42-d）
- **行为面 +2 组**：糖九件语义（let*/letrec 互递归/while 计数/and/or/when/unless——自举编译段产物 VM 执行 = 生产管线结果）+ module 臂行为面（inline 编译产物执行 = 生产管线含 registry 前端面；闭包+糖+module 混合语料）
- **语料实测勘误两处（GATE 1 诚实记录）**：① 条件位严格 bool（`(and 1 2 3)` 触 E0004「条件位置需要 bool」——kerf truthy 语义显式定义，行为语料改 `(and true true 3)`；parity 不受影响——编译段不类型检查）；② 同层 let 重名绑定是展开器错误（lambda 形参重名拒绝——语料改跨层嵌套遮蔽）

### 交付三：对账与收尾（43-z）

- 文档同步六面：plan.md Status（42-c 交付 r22 + 42-d 执行待续）+ 切口设计 v1.2（S2 执行注记 + module 名覆写值等价注记）+ matrix v0.1.0-r22（657→665；单元 202 + 集成 463）+ pipeline v0.4.0-r22（Tier 2 表体 bootstrap_compiler_tests 行补齐——r21 头部有表体漏的 R4 同型修正）+ v0.5-roadmap 批次 I 行 + 本 RELEASE_NOTES
- **§3.2 六命令全绿**（clean 起步终验）：build --release 12.87s 零告警 / check 0 errors 0 warnings / fmt 零 diff / clippy --all-targets -D warnings 0 / **test --release --workspace 665:0:0**（657 基线零回归 + 8；36.6s）；CLI 冒烟六路径（VM fib 75025/144 + macros `(2 1)`⇒42 + io 门控端到端 + check ok 38 指令 + native fib exit 144 + E0006 负例 fail-closed）+ 双审计集 EXIT 0（stage0 41 + stage1 50 APPROVED）
- r22 tar.gz 打包（§19.4 命令）+ 包内自举验证 + web 同步（kerf-data r22 + footer）+ agent-browser E2E + git 入账

## v0.4.0-r21（2026-09-11）——批次 I 执行：I1 前段基础核心形式 kerf 化（三件套第三实例，门 A parity 657 全绿）

### 交付一：compiler.krf——八臂编译段 kerf 实现（42-b 主体，S1 段）

- `crates/kerf-driver/src/bootstrap/compiler.krf`（~790 行）：**compile.rs 798 行的逐语义镜像**——三件套第三实例（r6 Reader / r14/r15 Expander 之后）。八臂全量：Literal（标记化分派 + Bool/Nil 直发不进池 + 引号点对递归先 car 后 cdr 对齐 MAKE_PAIR）/ VarRef（三源解析）/ If（跳转回填 j_false→then→j_end→patch 序）/ Begin（末项继承尾位 + Pop 挂项自身 Span）/ Set!（值→DUP→存储三路径）/ Define（入口原型守卫 + D1 消息逐字）/ App（被调先、参数从左到右 + 尾位 TailCall）/ Lambda（自由变量出现 → 捕获解析 → 新原型 → 新帧 → 体编译 tail=true → RET → 回写 → 弹帧 → CLOSURE 发射）
- **作用域解析机逐语义镜像**（B3 最难段）：`match-bindings`（同名 + `binder.scopes ⊆ ref_scopes` 子集匹配 + max-cardinality 严格大于替换——并列先注册优先）+ `resolve-var`（帧栈自顶向下；帧 0 仅局部；其余局部先于捕获；首个含子集匹配帧胜出；无候选 → 全局）+ 自由变量遍历（绑定屏蔽 + letrec* 序 + **shadow-pop 计数口径逐位镜像**——形参「新名才入栈 + 弹出恒为形参数」的 Rust 既有行为原样保留）
- **确定性纪律**（§7/B8）：入口全量复位（CC-* 14 项状态——镜像 CompileCtxt::new 每调用新上下文）+ 常量池/全局索引关联列表首插序（禁哈希序天然满足）+ 符号一律 str 携带桥回 intern + `const-eq?` 结构相等（eq? 对序对是引用相等——逐字段递归；Float 按 f64 == 镜像 Rust HashMap Hash+Eq 去重行为：±0.0 合一、NaN 永不去重）
- module/require 两臂显式边界错误（42-c 迁移面——§2.3-4 显式失败不静默）

### 交付二：bootstrap_compiler.rs 桥 + 门 A parity 套件（42-b 验收面）

- `crates/kerf-driver/src/bootstrap_compiler.rs`（~640 行）：CoreExpr → core 节点（四头字段协议 tag/s/e/exp——expander 输出同形态）→ `lexc-compile-program` VM 调用 → `('prog ...)` → BcProgram 类型重建（原型名形三态 `'main/'anon/'name`——魔法符号 Symbol(u32::MAX-1/2) 桥侧重建；操作码码表 0..=40 = opcode.rs 声明序；span 三元组桥侧注入 file_id）。**42-b 边界：parity 影子路径**（生产仍走种子 compile_module——CompilerKind 切换属 42-d）
- **INC4 实现期修订**（登记于切口设计文档注记）：字面量值消歧标记形态（`('int v)('float v)('str s)('sym 名)('pair l r)`——Str/Symbol/Float 在 VM 值面无谓词区分，桥侧定型传递）；span 携带 (s e exp) 三元组（bytecode_equal 判据含 expansion_id——INC4 原文「span对」修正）
- `tests/v0/stage2/plan/bootstrap_compiler_tests.rs`（19 测试 / 门 A 基础组 ≥8 超额）：**parity 13**（bytecode_equal 全结构含 debug_spans——字面量/常量池去重/引号点对/全局 define/if 回填/begin 尾位/嵌套捕获三链+遮蔽/set! 三路径/尾位穿线含相互尾递归/深嵌套 100 层/确定性双跑/空程序）+ **负例 2**（define 位置 D1 消息+Span 逐字；module/require 42-c 边界不对称断言）+ **行为面 4**（fib 144 / closures 计数器 (4 2) / higher_order map 平方 / 10 万层深尾递归——自举编译段产物 VM 执行 = 生产管线结果，examples/usage 基础件核心语义）
- **语言陷阱实测发现**（krf 侧修复三处）：`'true/'false/'nil` 在 kerf 中是 bool/nil **字面量**而非符号——标签分派必须经 `symbol->string` 字符串比较（expander.krf tname 同款纪律）；编译期两轮括号失衡（逐行平衡检查修复）

### 交付三：R4 发现项修复——操作码三方冻结漂移补齐

- **opcode_count_matches_spec 修正（40→41）**：TailCall（r18/40-c TD-022 TCO 引入）此前漏列于守护测试枚举——enum 实有 41 变体 vs 测试断言 40（数组恰好 40 项而断言同值——测试自洽但与 enum 漂移）。按 sop 附录 A R4（代码为准 + 本次修正文档）：opcode.rs 测试枚举补齐 + `04-bytecode-vm.md` 三处同步（§1 冻结计数 + 函数操作表行 3→4 + §3 执行循环组计数）
- 该发现证明门 A parity 语料经双实现实跑的对账价值（§2.3-11 先实测禁臆测——三方冻结契约的漏网只有全量对账才可见）

### 交付四：对账与收尾（42-z'）

- 文档同步五面：plan.md Status（42-b 交付 r21）+ 切口设计 Status/INC4 注记 + matrix v0.1.0-r21（638→657 净 +19 集成；单元 202 + 集成 455）+ pipeline-test-coverage v0.4.0-r21（Tier 2 行 + 657 基线）+ 本 RELEASE_NOTES
- **§3.2 六命令全绿**（clean 起步终验）：build --release / check 0/0 / fmt 零 diff / clippy --all-targets -D warnings 0 / **test --release --workspace 657:0:0**（638 基线零回归 + 19）；CLI 冒烟 + 双审计集 EXIT 0
- r21 tar.gz 打包（§19.4 命令）+ 包内自举验证 + web 同步（kerf-data r21 + footer）+ agent-browser E2E + git 入账

## v0.4.0-r20（2026-09-11）——批次 I 执行启动：I1 切口评估与迁移设计（638 零回归）

### 交付一：I1 切口评估与迁移设计（42-a——批次 I 首 MUV）

- `stage-2/i1-incision-migration-design.md`（10 节 + 附录）：**现状基线 12 实锚 B1-B12**（compile_module 单一入口 :229 / 十臂全景 / 作用域解析机 / bytecode_equal :145 现成 §21.3 判据 / 三件套先例×2 / analyzing 不在生产编译路径（front_from_core 实证）/ eval 参考路径独立 / SCOPE-NEXT 入口复位 :1471 / TCO 尾位穿线 / $hyg$N 回退 / 缓存键缺口 / 全结构比较无灰区）+ **本体盘点表**（S0-S6 段×文件×行数×依赖×裁定——compile.rs 798 迁移主体 / 字节码域 534 留 Rust / 组合根留 Rust / typecheck+hm 1709 绑 42-f / eval.rs 391 排退役）
- **切口裁定 INC1-INC8**：切口位置 = compile_module 单点（三件套第三实例——compiler.krf + bootstrap_compiler.rs 桥 + 种子 oracle）；值树契约复用 expander.krf 输出格式（输入零新设计）；字节码域留 Rust（VM 宿主契约）；组合根留 Rust（编排非编译逻辑）；**analyzing 段不迁**（I1 范围内——自举命题不依赖 typecheck，迁移评估绑 42-f/HM 同轮）；**eval 退役裁定排 42-d**（12 §2.5 行 299 口径）；**~80% 口径精确化**（读+展开+编译三段 100% kerf = §21.3 条件 1 机器口径）
- **段序 S1-S3（DAG 无环）**：S1=42-b 基础八臂（含作用域机+闭包捕获+回填——最难段 B3 风险前置）→ S2=42-c module/require 面+糖全管线 parity → S3=42-d 生产切换+自举终局
- **parity 三门 A/B/C**：门 A 段 parity（`bytecode_equal` 全结构含 debug_spans——基础组 ≥8 + 扩展组 ≥12）；门 B 自举一致性（§21.3 条件 2 机器判据——自举链 B₁/B₂ 隔离运行四程序逐一 bytecode_equal + 加强判据 B₀/B₁ 种子-自举全链终验）；门 C 回归门（全套件+双审计+T1 收口+CLI 冒烟）
- **切换点 P1-P5**：CompilerKind 镜像 ExpanderKind / 守护 production_compiler_is_bootstrap / 无 parity 不切换 + 单点回退 / **缓存键分桶（B11——种子与自举产物不得混享缓存）** / eval 退役终态口径
- **确定性纪律**（fixpoint 先决）：入口复位（SCOPE-NEXT :1471 实证延伸）+ 插入序即索引 + 符号 str 携带桥回 intern + 门 B 隔离运行

### 交付二：对账与收尾（42-z）

- plan.md Status 行（批次 I 执行启动 r20 交付 + 42-b 执行待续）+ §5a 42-b/c/d 行引用本设计为验收合同；matrix v0.1.0-r20（零测试增量注记 + 638 复跑）/ pipeline v0.3.0-r20 / 登记册 v0.3.0-r20（设计轮零债务面 + eval 退役排 42-d 注记）/ v0.5-roadmap r20 行
- **§3.2 六命令全绿**（clean 起步终验）：build --release 12.94s 零告警 / check 0/0 / fmt 零 diff / clippy -D 0 / **test --release --workspace 638:0:0**（单元 202 + 集成 436——零回归精确复现 r19 基线）；CLI 冒烟四路径：VM run fib ⇒ 144（print fib(25)=75025 + 终值 fib(12)）/ native fib（print-free 变体——TD-024 PoC 边界内）exit 144 / check ok（2 原型/9 常量/38 指令）/ require 门控端到端（print 输出 + nil）+ **E0002 负例**（未知能力项「print」fail-closed 拒绝——R9 防线实证）
- r20 tar.gz 打包（§19.4 r17 版命令）+ 包内自举验证 + web 同步（kerf-data r20 三节点 + footer）+ git 入账

## v0.4.0-r19（2026-09-11）——批间插入轮：能力模型泛化设计 + 模型层骨架冻结 + 批次 I 细化（638 全绿）

### 交付一：能力模型定位审思与泛化设计（41-a——用户指令轮）

- `stage-2/capability-model-design.md`（10 节）：**七面对照**（设计/契约接口/职责/能力覆盖/边界/命名/扩展面——真实 vs 当前逐面代码实锚 B1-B8）+ 术语裁定（「能力模型」双义消歧：语言能力 vs 权限安全模型）+ **族分类学 10 族候选**（io ✅ / ffi ✅设计冻结 / net / process / file / time / random / eval-stage / compilation-service）+ **令牌演算 6 操作**（mint ✅ + delegate 隐式 + attenuate/revoke/compose/amplify 预留位）+ 分层架构（模型层/族层/管线层/消费层四层归位）
- **裁定 D1：当前定位过窄成立**——能力安全模型被 IO 第一实例在命名（IoGrant/IoRequirements/IOError）、类型（Capability 枚举 2 变体）、职责（capability.rs 五职责全 IO 具体化）三面遮蔽；模型层无冻结位（13 §3.3 预留原则违例面——net/process 引入将触「五点手术」）
- **化学反应矩阵 C1-C6**（全部「正交可组合不合并」——§11 + D10 边界条件）：能力×效应（perform 需令牌 + handler=权限作用域 + 「不可撤销效应」统一窗口）/ 能力×多阶段（代码值携带能力集合——quote 零改动 + run 验证 D6）/ 能力×缓存（CacheKey 加能力面 D7）/ 能力×FFI（CPointer 归位第二实例 D8）/ 能力×HM（令牌类型禁泛化 D9——与值限制同型）/ 能力×工具链（编译即服务=能力合同 D10b）
- 裁定表 D1-D12 + 迁移路径 M1-M6（零破坏——冻结契约全程不动）+ 测试锚点正 6 负 6 + 风险 4 项 + 回写义务 6（M6 本轮全兑现）

### 交付二：模型层骨架 P3 冻结（M1 落地——reserved/ 第八子模块）

- `reserved/capability_model.rs`：`CapabilityModelFamily`（族形状——`type Token` + `family_name()`）+ `TokenCalculus`（演算位——`attenuate(&Token) -> Token` / `revoke(&mut Token)`，**无默认体**——P3 位不可被实现体污染）；模型层公开面零 Io 前缀（防层次再耦合）；生产面子模块零依赖（J3 维持——归属证明在测试面）
- **Probe 四测试**：骨架冻结（Probe net 族）/ **io 族两令牌归属证明**（`io_family_tokens_satisfy_model_shape`——IO ⊂ 能力模型的机器验证，用户判断的代码面证明）/ ffi 令牌归属证明（ExternalType::CPointer 载体）/ 演算位签名证明（函数指针形态）
- 兼容实证：capability.rs 12 测试 + reserved_ext_tests 12 集成零改动通过（原则 27）；mod.rs 速览表八子模块 + re-export；capability_io.rs / capability.rs 头部层次定位注记
- 文档回写：13 §3.1 标题术语消歧（v7.0——「4 个能力模型」→「4 个语言能力」）+ 13 §3.1.3 v7.0 三层形态注 + 12 §2.4.5 r19 注（手术面 D11 五点→三点）

### 交付三：批次 I 细化 + 收尾（41-b/41-c）

- `stage-2/plan.md` §5a：**42-x 八 MUV 六字段分解**——42-a I1 切口评估与迁移设计 / 42-b 基础核心形式 kerf 化 / 42-c 糖+module/require 面 / 42-d 两次编译自身字节一致（§21.3 条件 2 SHA-256 终验 + eval 退役终态）/ 42-e I2 stdlib+TD 批（TD-008/023/009/010/011/014/018）/ 42-f **I 后段 Effect M1-M5 + 能力管线泛化 M2 同轮**（driver 组合根双接触面协调）/ 42-g I3 门审查 + §14 阶段末环 / 42-h r20 收尾
- §3.2 六命令全绿（clean 起步终验）：build --release 12.49s 零告警 / check 0/0 / fmt 零 diff / clippy -D 0 / **test --release --workspace 638:0:0**（634 + 4——单元 202 + 集成 436）；CLI 冒烟：VM fib ⇒ 144 / native fib exit 144 / check ok / 能力门控 require 路径
- **对账六面**：RELEASE_NOTES（本条）/ matrix v0.1.0-r19（638 总量 + r19 增量行 + r18 集成计数勘误 438→436——内部矛盾 198+438=636≠634 修正）/ 登记册 v0.3.0-r19（零债务面）/ pipeline v0.3.0-r19（Tier 1 202 + driver 64/64 + Tier 2 436）/ v0.5-roadmap r19 行 / plan.md §5a
- r19 tar.gz 打包（§19.4 r17 版命令——tools/ + scripts/ 入包）+ 包内自举验证 + web 同步（kerf-data r19 三节点 + footer）+ git 入账

## v0.4.0-r18（2026-09-11）——批次 H：语义演进评估轮 + 三债清偿 + HM PoC（TCO 尾调用优化 + TD-007 完整口径 + Effect 语言级设计 + HM 推断 PoC，634 全绿）

### 交付一：H1 8 原语迁移五项语义层评估 + §6.3 全票投票（40-b）

- `stage-2/primitive-migration-evaluation.md`（6 节）：五项评估 × 三段式（收益/成本/风险）+ §13.4 J1-J6 判据 30 检查点 + 五角色逐项投票 20 票全记录（5.5/5.5 × 5 全票——E1 DEFER-TO-STAGE3 / E2 GO-DESIGN / E3 分层 / E4 SPEC-ANCHOR / E5 REJECT-STANDALAND）+ 代码实况锚 3 项（A1 词法寻址已实现 / A2 块式 ANF 实化 / A3 parity 链在飞）
- **总裁定：Stage 2 内原语集零变更**（核心冻结原则 9 维持）——8 原语形态整体迁移 = Stage 3 切换期候选（§13.2 登记）；§21.3 验收面与 12-roadmap 承诺面口径调和显式化

### 交付二：H2 三债清偿 + 一裁定（TCO 落地——40-c）

- **TD-022 resolved（TCO 兑现）**：VM `Op::TailCall` 帧复用（拆帧承返回地址——帧数净零）+ 编译器尾位穿线（`compile_expr(ctx, e, tail)`——Lambda 体 / If 两臂 / Begin 末项传递；顶层恒 false 主原型 Halt 终止）+ 内建尾调用隐式 RET + 指令预算护栏 `MAX_INSTRUCTIONS=10^9`（TCO 后帧数不增的无限尾循环兜底——结构化报错非挂死）+ `run_program_with_budget` 测试注入入口；实证：127,780B 源（5,000 define——>10^5 字符边界）自举管线完整通过 + 自举 expander 10_000 深度链端到端（TD-007/022 耦合解除）
- **TD-007 resolved（完整 10_000 口径）**：Stx Rc 共享化（`List/Vector → Rc<Vec<Stx>>` clone O(1)）+ retag 迭代式重建（显式工作表后序——栈深恒定）+ **均匀标记** `uniform_tag`（retag 输出子树均匀作用域证书 + `add_scope_to_all` 注入清除保健全性——链 N 步总工作量 O(N)）+ 扁平 Drop（唯一持有脊柱工作表拆除）；上限 500→10_000（种子 + 自举 expander.krf 同步）；10_000 链 0.02s + 边界 10_001 报错
- **TD-017 裁定维持 256**：eval 参考路径 I1 退役在即——大栈线程化不成立；T1 域注记（VM 尾递归超 256 深度域在双路径互查域外）
- **TD-023 P2→P3 降级重定型**：TCO 副作用实测 gc_stress 61-62ms × 5 轮稳定（r16 回归值 207-235ms → **-70%**；Stage 0 基线 160.4ms → -62%）——原基准不再复现回归（P2 证据基础失效）；残留非尾形根扫描模式待新基准（绑定 I2）
- **语义变更注记**：尾递归恒定帧（旧 105_001 尾递归报帧上限反转为通过项——tco_tests 正例；帧上限负例改非尾形态；尾调用帧不出现在追踪链——GCC/clang -O2 同行为）

### 交付三：H3 Effect 语言级设计（E2 GO-DESIGN 兑现——40-d）

- `stage-2/effect-language-design.md`（8 节）：设计裁定 12 项（D1 perform/handle 二形式 / D2 浅处理 / D3 continuation 线性唯一（Fresh→Resumed 动态防线 + E0008）/ D4 resume 非独立原语 / D5 set!→Perform(State) = 等价证明非实现 / D6 ext1 具体化 / D7 eval 逃逸映射 + T1 域收窄 / D8 TCO 正交 / D9 E0007-E0009 诊断族 + FFI 族 E0010-E0012 码位预留 / D10 能力-效应正交 / D11 多次恢复不实现 / D12 实现窗口 = 批次 I 后段）+ R10/R11 归约规则 6 条 + E4 三要素兑现表 + 迁移路径 M1-M5 + 风险 5 项全附缓解；实现窗口裁定 D12（设计做实 Stage 2 / 实现批次 I 后段——roadmap 口径回写）

### 交付四：H4 HM 推断 PoC（38-d 设计 GO 有条件兑现——40-e）

- `kerf-compiler/src/hm.rs`（~850 行）：**约束三段式**（生成 → worklist 求解 → zonk）——数值格扩展合一（Int/Float/Num 互匹）+ occurs check + 元数结构 + 失败逐条收集（多错误）；D2 值限制 / D3 set! join / D4 递归预置（define + letrec 双形状——sugar.rs 实况核对）/ D6 双点泛化（顶层序 + let 形状 App-of-Lambda 识别）/ D7 诊断（E0005 族 + 形式级桶隔离 + 512 生成期预算）；BUILTIN_SIGS 解释层重解释（cons/car/cdr 结构化 Pair(τ,τ) + NumOrAllStr/Ordering 变元锚点约束）
- **双门全过**：超集门 29 程序（R1-R8 检出 → HM 亦检出——双检查器并行对照）+ 零误报门（examples 六件套 + 动态边界 15 case 零诊断）；**四类缺口检出证明**（用户 lambda 实参错 / car 元素类型 / 分支分歧 / 递归元数域错——R1-R8 静默放过面）；occurs ≥3 + 值限制 ≥2 + 多错误 Span 序；**验收口径（R4）**：fib : (num → num) 非 Dynamic（设计预期 Int→Int 修正——n 全用点数值域约束，格合一最小解 = Num）
- 实现勘误实录：generalize 自污染（exclude + 泛化点前 solve_now）/ Ordering 变元锚点约束缺失两轮修复

### 交付五：40-g 接口预留层标准化拆分（用户指令插入 MUV）

- `reserved/mod.rs` 344 → **66 行纯声明**（模块声明 + re-export）：四能力族独立文件——`effect_handlers.rs`（79 行）/ `multistage.rs`（69 行）/ `capability_io.rs`（79 行）/ `compilation_cache.rs`（120 行），对齐 13 §3.1.1-§3.1.4 分节结构（§13.4 J1-J6 全过）；契约与实现的模块分离形态注记（P2 消费面 crate::capability/crate::cache）；**原则 27 兼容实证**：reserved_ext_tests 零改动通过（re-export 路径恒有效）；Probe 冻结测试随迁拆分（合并测试 → 4 子模块独立 Probe，净 +2）

### 交付六：质量口径与收尾

- **§3.2 六命令实跑全绿**（clean 起步：build --release 12.78s 零告警 / check 0/0 / fmt 0 diff / clippy -D 0 / **test --release --workspace 634:0:0**（34s——集成 438 = 409 + TCO 12 + HM 15 + Probe 拆分 2；单元 196——逐二进制实测：span 11 + syntax 11 + core 10 + reader 23 + expander 32 + compiler 15 + runtime 9 + vm 18 + driver 60 + backend 9））；CLI 冒烟：VM run fib ⇒ 144 / native fib exit 144 / check ok / 恢复模式 E0002 合并报告
- TD-025 resolved（40-f 收尾轮补修）：krf 六头字段协议（`(tag s e exp scopes uni . fields)`——uni = ('uni . scopes) 均匀证书：make-node 默认 nil / retag 置位 / inject 清除 / 桥 stx_to_node 同步）+ retag-scope 快路径（种子 uniform_tag 镜像）——门审计 C02 包装链 **>540s → 7.37s（73×+）**；双审计集 EXIT 0（stage0 41 + stage1 51）；实现勘误实录：六头协议初版桥侧编辑未生效（fmt 重排致静默未匹配——最小形式全崩定位）+ 弱内容等快路径被 parity 套件捕获否决（def⊆use 顶层角共享过早——展开代次不提升分歧）→ 真标记协议
- r18 tar.gz 打包（§19.4 r17 版命令——tools/ + scripts/ 入包）+ 包内自举验证（634:0:0 + CLI 冒烟）+ web 三层同步 + agent-browser E2E + git 入账 + worklog 树压实（38-g 补记 + 40-a~40-g 全条目）

## v0.4.0-r17（2026-09-11）——批次 G：后端 / FFI / 类型三主线（QBE 后端 PoC fib 本地码端到端 + FFI 所有权模型 + HM 设计轮 + TD-013 恢复实现，605 全绿）

### 交付一：G1 QBE 后端 PoC（首个非 VM 后端——§21.3 条件 3 兑现）

- **QBE 1.3 工具链落位**（§3.1 链：scripts/ → tools/ → docs/tools/ → 安装 → 记录）：`tools/qbe/bin/qbe`（670,544 B，amd64_sysv 六目标）+ 源码归档（可重建）+ `scripts/qbe/setup.sh` + `docs/tools/qbe/setup.md`（含三段冒烟实录 + 语法勘误两条：函数签名必带返回类型 / 比较指令宽度后缀 csltl）
- **kerf-backend 新 crate**（第 10 成员——后端层独立，§11）：`codegen.rs`（**契约迁移**：CodegenBackend/AnnotatedANF 自 kerf-driver/reserved 迁入正式家——签名零变化（原则 27），AnnotatedANF 从指纹占位**实化**为函数定义集 IR，fingerprint 字段保留；reserved/codegen 改薄 re-export，旧路径继续可用——迁移兼容锁存测试实证）+ `anf.rs`（块式 ANF IR + CoreExpr lowering：phi 值合并 / 跳转回填（§19.3 不变式 2 同型）/ 形式级两遍扫描 / arity 静态校验）+ `qbe.rs`（IL 生成 + QbeBackend 首个做实实现——QBE 内建 pass 声明 ssa/gvn/gcm/rega）+ `aot.rs`（外部进程编排：qbe → 汇编 → cc → 可执行 → 运行；查找链 KERF_QBE → 安装布局 → 编译期锚；纯函数注入式 env——并行测试零竞态）
- **CLI 13 子命令**（11 → +2）：`kerf anf <file>`（IR 摘要 dump）/ `kerf native <file>`（AOT 编译 + 运行——产物三路径打印 + exit code 口径）
- **端到端实测**：`(fib 12)` ⇒ **本地码 exit 144**（= VM 路径 ⇒ 144 双路径一致）；IL 结构断言（csltl/call $fib/sub/add/jnz/export main 全命中）；值上下文 if 经 phi 合并（`(- (if (< 1 2) 10 20) 5)` ⇒ 5）；嵌套 if 组合（classify ± / 111/144）——**PoC 边界 B1 登记（TD-024）**：整数域原语十二项 + 递归调用；闭包/Float/Str/Pair/set!/module/print·IO/函数值一等均显式边界外错误（非静默降级）
- **测试 +40 集成**（qbe_backend_tests：端到端 6 + 结构 4 + 一致性采样 6 + 负例 10 + 契约 2）+ **+9 单元**（backend crate）——585 中间基线全绿

### 交付二：G3 FFI 所有权模型定义（§21.3 阻塞项解除——子代理 ARCH-A/ALG-A 交付）

- `stage-2/ffi-ownership-model.md`（216 行）：三原语责任矩阵 3/3（CallExternal 借用窗口 + CInt/CPointer/Opaque 归属三分法 / AllocExternal 线性所有权 / FreeExternal 消费释放）+ **pin/unpin 形式化**（Φ 计数簿扩展 σ=(H,R,Φ) + P1/P2/U1/U2 归约 + 引理 F-PIN + mark_all 起点快照并入——主循环零改动）+ **线性令牌语义**（可重复借用传递 + 一次性消费全局生效；失效 E9 诊断而非 UB）+ ExternalPointer 状态机（非法迁移 7 条 E9-E11）+ 边界 case 13 个全判定 + 决策记录 19 条（新裁定 12 项显式标注）+ 实现路线（G1 PoC 整数路径不触及 pin；批次 I write_stdout 借用路径做实）+ Stage 3 锚点 6 项
- 回写义务 4 处登记（06 §3 E9-E11 / 05 §3.1 Foreign / 13 §3.3.4 行为规格引用 / TD-010）——落地轮执行

### 交付三：G2 HM 推断设计轮（子代理 ALG-A/ARCH-A 交付——GO 有条件）

- `stage-2/hm-inference-design.md`（303 行）：R1-R8 逐条行号锚盘点（8 规则 + 8 架构事实）+ 11 项核心裁定（**约束三段式算法**（否决 W/J——TD-013 多错误协同 + TD-022 栈安全）/ 值限制 OCaml 式 / set! join / letrec fresh 预置双形状特判 / occurs check / TcType→HM 十一构造子映射 / 双点泛化 / 诊断集成 / 三阶段演进轨道）+ 16 参照对照 6 行 + 冲突 7 项（P0×3：letrec nil 预绑定 / 保守契约重定义 / 数值塔格合一偏差）+ 风险 P0×3/P1×3/P2×3/P3×1 全附缓解
- **裁定**：GO（有条件）——PoC 排批次 H 新增 MUV（H4），默认期早于 I1 自举迁移（推断器本体 kerf 化前置）

### 交付四：TD-013 多错误收集与恢复展开实现（P2 清偿——r7 设计验收 5 条全过）

- **种子路径**（kerf-expander `recover.rs`）：DiagCollector（push / is_full(128) / mark_truncated / into_sorted——(file_id,start,end) 稳定排序）+ expand_program_recover（形式级恢复：错形式收集跳过继续；满即显式截断 + 提前终止——38-e 勘误：break 路径须显式置位）
- **自举桥路径**（bootstrap_expander `expand_program_recover`）：逐形式单元素列表调用（expander.krf 协议零改动——调用粒度桥侧切换）；双路径同构测试（诊断数/产物数/错误消息族一致——parity 纪律）
- **driver 消费面**：`check_source_recover`（front_from_core 抽段重构——R9 fail-closed + 簿记 + 字节码共享段单一实现）——E0002（展开）+ E0005（类型）**全量合并报告** + 位置序 + 截断尾注；**CLI `kerf check` 切换恢复模式**（单错误短路保留库 API check_source——run/eval 执行路径不变，r7 设计 §5）
- **实测**：`(define x 1) () (define y 2) y` → 1×E0002 + **y 完整进编译产物**（指令数 = 无错对照一致——r7 验收 1 兑现）+ 混合诊断 E0002/E0005 同报位置序 + >128 截断提示 + read/E0006 短路边界维持 + **16 集成测试全过**

### 交付五：质量口径与收尾

- **§3.2 六命令实跑全绿**（clean 起步：build --release 11.52s 零告警 / check 0/0 / fmt 0 diff / clippy -D 0 / **test --release --workspace 605:0:0**（28s））；双审计集 EXIT 0；CLI 冒烟：VM run fib ⇒ 144 / **native fib exit 144** / test 2/2
- 测试增量对账（553 → **605**，净 +52）：集成 +40（qbe 24 + 恢复 16）+ 单元 +12（backend 9 新 crate + expander +4 recover - reserved codegen 2 迁移 + 兼容锚 1）
- 文档同步：matrix r17 对账 + 登记册（TD-013 **resolved** + TD-024 新增 B1 边界）+ pipeline-test-coverage Stage 2 节 + v0.5-roadmap 批次 G 行 + stage-2/plan Status 更新 + lang-design 13 §3.3.7 迁移注记 + 10-toolchain CLI 13 子命令
- r17 tar.gz（§19.4 整目录含 tools/qbe + 包内自举验证 605:0:0 + CLI 一致）+ web 同步 + E2E + git commit

### 下一步（批次 H：语义演进评估轮）

H1 8 原语迁移五项评估（§13.2 + 委员会投票）∥ H2 TCO 决策 + TD-007 Rc 化（10_000 口径）+ TD-017/TD-022 裁定 ∥ H3 Effect 语言级设计 ∥ **H4 HM 推断 PoC（38-d 裁定新增——三阶段轨道第一段）**；I 批次（编译器本体 kerf ~80% 迁移 + 两次一致 + stdlib 完整化 + TD-008/023）。

---

## v0.3.0-r16（2026-09-11）——批次 F：Stage 1 深审收尾环（§14 阶段末全协议 + §14.6 阶段间验证 GO + TD-023 登记 + lang-design v6.2）

### 交付一：§14.5 D1-D8 深度审查（deep-review-round1.md + 委员会投票 GO-WITH-CONDITIONS）

- **P0/P1 = 0**；P2×2（TD-023 gc_stress 回归新登记 + 登记册目标时机过期家族）；P3×9；**无系统性 B3「实现违反设计」**——全部偏差为「实现前进、文档滞后」方向
- 偏差清单 17 项（子代理 36-a-facts 独立扫描 20 篇 lang-design + 主代理亲验五项抽查全中）+ 无偏差确认 12 组（40 操作码/52 内置/parity 36/28/预留 14 项等核心契约面全对上）
- §6.3 批次 F 批准补票（stage-2/plan「规划批准中」悬空项）：五角色加权 5.5/5.5 = 100%
- §3.2 基线 clean 全量复验：build 9.86s 零告警 / check 0/0 / fmt 0 diff / clippy 0 / **553:0:0**（25s）+ CLI 冒烟

### 交付二：§14.8 设计回写（lang-design v6.1→v6.2 十篇 + 登记册 v0.3.0-r16）

- **TD-002 符号值家族三篇**：05（HeapObj 七变体/八入口/三通道函数/TD-023 注记）、06（值域十变体 + 语义字段限定）、01（literal_value + Symbol）
- **批次 E 滞后四面**：09（TD-021 r15 解决注记）、11（553 口径）、07（471 行 + 全模式面限定）、03（ExpandCtxt 四字段）
- **12-roadmap 两行矩阵改写**：元循环求值器（保持 Rust 参考，eval 自举滑入 Stage 2）/ 宏系统（E1-β 收口 ✅，调试工具 Stage 2+）；10-toolchain 补 CLI 11 子命令清单（B4 收口）
- **登记册全量对账**：索引补全 23 行（TD-015+ 此前无索引行的结构性缺口修复）+ TD-012 resolved（批次 B 六文件实证）+ TD-013/009/010/014/017/018 改判 Stage 2（附门放行裁定）+ TD-019/020 断档登记（禁复用）+ **TD-023 新增（P2——绑定批次 I2）**

### 交付三：§14.6 阶段间深度验证（六篇 + 自举 vs 种子前段实测）

- **architecture-review**（32✅/3⚠️/0❌——Stage 0 ⚠️×4 收敛 2）/**design-impl-test-coverage**（B1 全登记零未登记缺口 + 三者不一致零项）/**hidden-problems-assessment**（复杂度 ≥2× 强制修复触发 0 项）/**refactoring-optimality-review**（7 重构 7 优 0 hack）/**performance-baseline**/**final-assessment**（GO + 门审 checklist 增补：登记册核对 + 对账面六面清单）
- **自举 vs 种子前段开销实测**（§21.5 信号 4 补录）：**比值 69×（fib）/157×（宏）/116×/388×（300-defines）** + 冷启动 16.1ms + fib bench 89.4~89.6ms（**自举切换零性能代价 +0.8% 噪声带**）——**stage-2 DAG 的 H2→I1 次序被实测验证**（TCO 是编译器本体迁移的实质前置）
- **gc_stress 回归**（+29~46% 超线性，超 §14.6.4 10% 阈值）：TD-023 登记（per-cycle HashSet 分配 + Value 宽度候选根因）→ 批次 I2（TD-008 分代同轮）；正确性无影响不阻塞
- pipeline-test-coverage.md 全量重写（294→553 口径 + Stage 1 套件 11 行 + parity 印证节 + catch-all 42→1 通配实测）

### 交付四：§14.9 C1-C6 代码整理（零语义变化）

- 三处注释级修正（driver.rs 切换守护方向反写 / kerf-runtime「四类根」→五来源 / 堆语义注释过时）——**553 零断言修改逐一等价复跑实证**（探针临时部署/移除各一次全绿）
- C1-C6 全维检查：TODO/FIXME 0、glob 0、//! 66/66、§11 五项合规、真通配 catch-all 1 处（带臂级注释）

### 交付五：质量口径与收尾

- **§3.2 六命令实跑全绿**（clean 420 files → build 9.86s → check 0/0 → fmt 0 diff → clippy -D 0 → test **553:0:0**）；双审计集 EXIT 0（41 + 50）；探针移除后全量复验 553:0:0
- 文档同步：v0.5-roadmap Stage 1 行 ✅ + matrix r16 对账（表体两行尾差修复）+ stage-1/plan 批次 F 注记 + stage-2/plan TD 绑定行（G2←TD-013 / I2←TD-023 / H2←TD-017）
- r16 tar.gz（§19.4 整目录 + 包内自举验证 553:0:0 + CLI 一致）+ web 同步 + E2E + git commit（r13-r16 积欠统一入账 per §6.4）

### 下一步（Stage 2 主体——批次 G 起）

G3 FFI 所有权模型定义（§21.3 阻塞项先行）→ G1 QBE 后端 PoC（QBE 二进制按 §3.1 链位安装）∥ G2 HM 推断设计轮（+ TD-013 多错误恢复绑定）；H2（TCO + TD-007 + TD-022 + TD-017——**I1 实质前置，性能基线实测锚定**）；I2（TD-008 分代 + TD-023 根扫描对症）。

---

## v0.3.0-r15（2026-09-11）——批次 E 收口：E1-β 宏系统 + 生产切换 + TD-021 prelude + Stage 1 门审查 APPROVED（553 全绿）

### 交付一：E1-β 宏收口——expander.krf 完整宏系统（syntax-rules 卫生宏，VM 上运行）

- **宏机制**（expander.krf +~470 行）：define-syntax 注册（变换器注册表 = 名字单表——用户宏 'rules / 糖惰性注册 'builtin，HashMap 替换语义镜像：用户宏可覆盖糖名）+ syntax-rules 解析（字面量集合/子句 (模式 模板)/五类解析错误逐字镜像）+ **模式匹配**（通配 `_`/字面量（同名 sym）/省略号（零/多段/复合单层）/字面量 datum 值相等/嵌套列表/向量 `[...]`——trial 语义经函数式绑定传递）+ **模板实例化**（模式变量替换保留用户 Span/作用域、(var ...) 省略号拼接跳格、悬垂省略号按普通符号走实例化——报错优于静默）+ **卫生 α 重命名**（`name$hyg$N`——num->str 递归构造；同标识符同实例化恒同映射；保留集（宏自引用）+ 核心关键字豁免）+ **深度计数 500**（链长语义——expand-form 入口快照/出口回滚，兄弟不累计；超限结构化报错非栈溢出）+ retag def∪use 并集 + 卫生基名回退（rsplit $hyg$ + 后缀纯数字保守校验）
- **Span 并集代次守卫镜像**（实测驱动的发现——parity 失败暴露）：节点形状升 v2 = `(tag s e exp scopes ...)`（exp = Span.expansion_id，桥侧发射/读回）；**instantiate 的 Span 并集须镜像 Span::merge 保守策略——代次不同 → 丢弃该跨距**（糖产物内宏调用：use-site 代次 ≥1 vs 模板 0 → 产物 Span 落模板范围）；retag 代次 +1（bumped_expansion）；全部构造器（糖/提升/宏）逐位镜像代次来源
- **核心节点携带代次**（生产切换的相位可观测性保持）：core 节点 `(tag s e exp ...)` → 桥重建 `Span{expansion_id}`——「宏引入引用未绑定错误应携带 expansion N 标记」（E3 诊断特性）经切换后保持
- **实测驱动修复**（§2.3-11——全部经 parity 双实现实跑发现）：foldl/for-each 序章补定义（chars->str→foldl 此前为死代码——卫生基名回退首次激活）+ 宏产物品类 6 处（let 括号/字面量 tag 铸造/letrec 先用后绑/...的 r14 既有 + E1-β 新发现：set/require/define-syntax 代次字段漏挂×3、模块注册簿错误无 Span 无渲染×2）

### 交付二：生产切换（E1-β——读 + 展开两段全自举）

- **compile_front 展开段** → `bootstrap_expander::expand_program`（VM 上 expander.krf）——**「语言能表达自身前端」的完整生产命题**；Rust 种子保留双角色：自举引导（reader.krf/expander.krf/preamble.krf 的编译经 compile_front_seed）+ parity oracle（测试对照）
- **切换守护**：driver 单元 `production_expander_is_bootstrap`——独立线程（thread_local 零残留）编译前活性探针 `is_loaded()` = false → 编译后 = true（实测判别非推断）+ 宏产物展开代次标记 ≥1 双信号
- **553:0:0**（528 基线零回归——全套件经自举 Reader + 自举 Expander 生产管线执行）

### 交付三：TD-021 高阶函数用户面注入（P3 清偿——模块/import 承载）

- **kerf-prelude 模块**（`bootstrap/preamble.krf`）：map/filter/foldl/for-each（reader.krf 序章同源）；用户程序 `(module 名 (import kerf-prelude) ...)` 声明导入
- **注入机制**（P1/P3/P5 三否决全规避）：read 后、expand 前 forms 级合并（独立 file_id——Span 指向 preamble.krf，无源码拼接诊断污染）；单一编译单元（无跨程序全局合并）；零 VM 再入；**多模块按序 declare + 主模块 visit**（§8.9 单模块生命周期扩展——import 边传递依赖）；同名 define 显式报「重复定义」（不静默遮蔽）；无 import 声明保持未绑定（opt-in 语义）
- **prelude_tests 7 case**：用户面可见/组合管道（filter→map→foldl = 50）/for-each 副作用/双路径一致/opt-in 负例/名字捕获显式失败/未知导入

### 交付四：Stage 1 门审查（§7.3 + §21.3 四条验收——APPROVED）

- **审计集** `examples/audit/stage1_gate_audit_r1.rs`（50 case，可重运行）：§7.3.1 配比满足（A 单语句 12 / B 多语句 12 / C 复杂 10 / D 恢复 6 / E 边界 6 / P 正向 4）+ §7.1.1 **七类全覆盖**（语法/未绑定/空应用/参数个数/类型不匹配/循环依赖（双模块源码 → DFS 灰标记结构化 Err）/宏深度（自指宏经生产路径））+ §7.3.2 边界 6（本批次修复面：Span 代次守卫糖内宏/require·set 代次/prelude/模块内宏注册）——全部 case 经**生产管线**（自举读+展开）
- **审计发现项 2（已修复）**：模块注册簿错误（未声明模块/循环依赖）无 Span 定位 + 无诊断渲染——修复：挂 module 形式 Span + render_diagnostic（P2 诊断质量）
- **§21.3 四条件锚定**：① 子集表达前端——生产管线自举（VM Reader + VM Expander 含宏）② VM 上正确运行——自举 Expander 承载全部生产展开 ③ 标准库最小集——prelude 管道（列表 hofs 用户面）+ stdlib（列表/字符串/I/O）④ 增量编译——P04 缓存确定性 + 第二次 check 命中观测 + 静态检查 0 错
- **结论**：APPROVED（50/50 PASS + 配比满足 + 七类全覆盖 + 零新 P0/P1）——Stage 1 全批次（A/B/C/D/吸收/E）交付闭环，进入 Stage 2 准备

### 交付五：质量口径

- **§3.2 六命令实跑全绿**（clean 起步：498 files/123.6MiB → build 9.39s 零告警 → check 0/0 → fmt 零 diff → clippy -D warnings 0 → test **553:0:0**（单元 184 + 集成 369））；双审计集 EXIT 0（stage0 41 + stage1 50）；CLI 冒烟 fib 75025 + ⇒ 144 + kerf test 2/2 + 宏+prelude 自检 ⇒ 10
- 文档同步：tech-debt-register TD-021 → 已解决（r15）+ matrix 553 对账 + stage-1/plan.md 批次 E 行收口 + bootstrap-expander.md E1-β 增补 + prelude.md + stage1 门审计测试计划

### 下一步（Stage 2 准备——§21 阶段规划先行）

Stage 2 启动条件核对（§7.3.3 收敛裁定 + §21.5 切换信号）→ 阶段规划（§21 + §13.1 设计对齐 + §17 排版图）→ 后端策略裁定（LLVM 可选后端引入评估——永不入自举链边界重申）+ TD-007 残留（10_000 完整口径）与 TD-022（Reader 帧消耗 O(n)）的 TCO 决策点。

---

# RELEASE_NOTES

## v0.1.0 —— Stage 0 语义验证（2026-09-09）

### 交付

- **12（10+2）能力模型全量落地**（P0/P1）：
  类型化 Token Reader / 图 IR（共享节点）/ 结构化 CodeValue /
  元循环求值器 / 基础闭包（共享单元格捕获）/ Span 全管线 /
  结构化诊断 / 最小 I/O / 相位分离 / 卫生宏（syntax-rules）/
  标记-清除 GC / 字节码 VM（39 操作码）
- **四项接口预留冻结**（P2/P3）：Effect Handlers / 多阶段编程 /
  能力模型 I/O / 编译缓存
- **9 crates Cargo Workspace**（零外部依赖）+ CLI（10 子命令）
- **200 项测试全绿**；§3.2 验收全绿（build/check/test/fmt/clippy）
- **文档**：lang-design 20 篇（stage0.md v5.0 拆分）+ develop/tests/graph 树

### 语义验证亮点

- fib(25) = 75025（VM 与 eval 双路径一致）
- 卫生宏：宏内外同名不串扰（一致性 α 重命名 + 自引用保留 + 关键字保留）
- letrec 互递归 / 闭包可变捕获（独立计数器）/ 深递归 10^4（迭代式帧栈）
- GC：3×10^5 临时分配堆有界；环回收；2×10^5 深链标记不爆栈

### 已知边界

TD-002~TD-011（P2/P3，显式推迟）——见
docs/develop/v0/tech-debt-register.md。无 P0/P1 遗留。

### 基准

- fib(25)（含编译）：84.7 ms/轮（release）
- GC 压力：0.72 s / 3×10^5 分配

### 下一步

Stage 1（自举验证）：目标语言子集重写编译器前端 + 类型检查器。

---

## v0.1.0-r2 —— lang-design v5.1 收敛 + 双路径语义分裂修复（2026-09-10）

### lang-design v5.1（设计文档收敛审查）

- **06-操作语义从 21 行空壳重写**（~200 行）：9 原语小步归约规则组 R1–R9
  （含 βv/δ 辅助规则）+ 错误吸收语义 E0–E8 + GC 不可观测性引理 L-GC +
  编译正确性定理 T1（L1–L4 三引理证明纲要 + 适用边界）——补齐
  stage0.md §13.4 的源文档理论欠账
- **03-宏系统 / 05-运行时契约回填**（从冻结实现）：
  Transformer/TransformerKind/ExpandCtxt/ModuleRegistry/syntax-rules 文法；
  Heap/GcRef/Slot/RootSet/I-O 通道契约
- 01–05/09–11 统一增补「处理程度（P0–P4）/所属 Stage/推迟项」标注 +
  「测试锚点」节（设计驱动测试锚定）；02 职责矛盾调和（词法层 vs
  Reader 模块）；04 操作码漂移注记 + CodeBuf/CaptureSource 契约；
  断链修正（19-参考文献 §3.5/§3.6）×3；术语计数 42；全库 0 断链

### 双路径语义分裂修复（依据 06 定理 T1 审查发现，CLI 实证三组反例）

- **新增第 40 号冻结契约 `DefineGlobal`**：define 与 set! 的全局存储
  语义分裂修复——`StoreGlobal` 收紧为 S1/E3（未绑定报错）；
  `DefineGlobal` 承载 D1/E6（同层重复定义报错）
- **Define 返回值统一**（T1 反例）：编译模式改为 `value; DUP;
  DefineGlobal`，与 eval 路径一致（此前 VM 返回 nil）
- **lambda 形参表重名展开期拒绝**（A3 卫式，两路径共同上游）
- 修正 2 处依赖旧错误语义的测试断言（去除兼容思维）
- **204 项测试全绿**（200 基线 + 4 组新语义双路径对账）；
  §3.2 验收全绿（build/check/test/fmt/clippy 零警告）

### 交付包

kerf-stage0-v0.1.0-stage0.1-12-caps-langdesign-v5.1-semantics-r2.tar.gz
（312 KB / 165 文件；包内解压自举验证 204 测试全绿）

---

## v0.1.0-r3 —— deep-review R1 修复 + 负测扩张 + 门审计集（2026-09-10）

### 深度审查修复（P1×4 清零）

- **App 求值顺序统一「函数先」**（偏差 #13，T1 反例面）：compile.rs/vm.rs 双侧对齐
  06 §1.3/§2 A1 契约；双路径错误排序负例回归（app_evaluates_fn_then_args /
  app_evaluation_order_fn_first_dual_path）
- **操作码冻结守护测试重写**（偏差 #7）：40 项显式枚举（八组：栈 7/变量访问 7/控制流 2/
  函数 3/算术比较 12/数据 3/谓词 5/终止 1）——enum ↔ 测试 ↔ 文档三方冻结
- **模块循环依赖检测**：phase.rs DFS 灰标记 → 结构化 Err「模块循环依赖：…」
  （菱形依赖合法；§7.1.1 类 6 补齐）
- **§7.3.1 门审计集就位**：examples/audit/stage0_gate_audit_r1.rs——41 case
  （负向 32 + 恢复 6 + 正向 3），§7.1.1 七类全覆盖；r1 的 gate PASS 判定缺陷闭环
- **驱动修复**：VM 堆栈追踪（run_program 最内 16 帧 note）；eval 路径卫生回退接线
  （与 VM 路径镜像，T1）；driver 公共 API dump_tokens/dump_stx（CLI 经 driver 转发，
  §14.7.2 B4 合规）

### 负向测试扩张（§9.4.3 1:3 门限达标）

- 四个表格驱动负测文件：negative_reader（60 case）/negative_expander（98）/
  negative_vm（231）/negative_semantics（100+）≈ **490 case**（r1 基线 37）

### T17-a 对抗深挖修复批（r3 末轮——§14.6.3 独立深挖发现）

- **D1** 函数体内 begin 包裹 define 全局泄漏（VM）vs 词法（eval）——T1 反例：
  编译期结构化拒绝（E0003，两路径共享 compile_source 双侧一致）
- **D2** `eq?` 字符串指针比较 → 内容比较（VM 常量池去重路径与 eval 分裂）
- **D3** eval 深递归栈溢出 abort → MAX_EVAL_DEPTH=256 结构化上限（实测标定）
- **D4** `i64::MIN /± -1` Rust panic → checked_div/rem 结构化错误
- **D5/D9** `(+)` 越界 panic → 单位元 0；`(*)` → 1；`(- x)` → 取负
- **D6** 宏调宏未绑定 → 展开器基名回退（卫生穿透落地）
- **D7** eval 错误逐层「求值失败：」前缀累积 + Span 丢失 → 保真透传
- FS-1 Reader 嵌套守卫失效（600 层即溢出）→ MAX_NESTING_DEPTH=256
- 全局正负比 1:0.24 → **≈1:3.2**（负 515 vs 正 ~160，case 口径）；
  E1–E6 + E0001/E0002/E0004 直接断言矩阵；E7/E8/E0003 不可触发性文档化存档
- **297 项测试全绿**（298 函数，0 失败 / 1 忽略-文档化）；§3.2 验收全绿（release）

### 工程整理

- examples/ 重组：usage/（6 个 .krf）+ audit/（审计集）+ README 索引（§9.6.2）
- lang-design v5.2：deep-review §6 偏差清单 26 项全量回写（操作码全枚举/HeapObj 六变体/
  根集五来源/GC 数值冻结/内置 24 项清单/reserved.rs 签名回填/7 层→9 crate 映射表等）
- 工程文档对账：matrix / pipeline-test-coverage（§9.5.1 三层 + §14.6.1.1 完整性）/
  negative-tests.md 新建 / TD-012~014 登记

### 基准

- fib(25)（含编译）：84.4 ms/轮（release，复现声称 84.7）
- GC 压力：3×10^5 分配单轮 ~0.17 s（更正 0.72s 陈旧口径；gc_tests 10^6 为验收权威口径）

### 下一步

Gate R2 门审查复审（审计集就位后按 §7.3 重跑）→ §6.3 外循环投票 → Stage 1 规划输入。

## v0.1.0-r4（2026-09-10）——Stage 1 批次 A：切换期重构 + 第一批工作项

**SOP 流程**：Stage 1 启动（§21 规划 → §17 排版图 → §18 依赖审查 → §13.1 设计对齐 → §4 MUV 批次 A）。

### 交付（3 MUV + 1 重排）

- **TD-015 已解决**：`compile_source` 按消费方分流——run/eval 生产路径改走
  `compile_front` 前段（不构造图 IR）；完整入口保留给 `ir`/`code`/`bc`/`check`
  dump 与检查子命令；分流守护测试（快路径与完整编译字节码逐指令一致）。
- **TD-012 已解决**：expander.rs 1372 → 517 行（-62%）——三职责分置
  （core_forms.rs 511 / sugar.rs 426，crate 私有模块，公共 API 零变化）；
  测试整体保留主控走公共入口（拆分等价性天然回归）。
- **TD-007 部分解决**：宏展开 trampoline 工作表（宏产物头部仍是宏调用时
  迭代继续，展开控制流栈深与链长解耦）；深度上限 128 → **500**（实测
  标定：2MiB 测试线程 1_000 通过/2_000 溢出，2× 裕度——TD-017 同型
  实测法；探针 example 双环境数据记录于 TD 登记）；完整 10_000 口径
  依赖 Stx Rc 化（批次 B 前端重写）。
- **TD-004 重排批次 B**：完整实现 = 绑定 scope 注入 + CoreExpr::VarRef
  scope 桥 + 双路径解析体系切换（≥800 LOC 跨 5 crate）——超出单 MUV
  容量，与 TD-002/标准库同批（依据 §1.2.1 只升不降 + §12 最优>最小）。

### 质量口径

- §3.2 全绿：build --release 0 警告 / check --all-targets 0/0 /
  fmt 零 diff / clippy -D warnings 零警告 / test --release **304:0:1**
- 审计集 41/41 复跑 EXIT 0；新增集成套件 expansion_worklist_tests（4 例）
- SOP 文件更名：`stage-committee-process.md` → `sop.md`（引用同步 3 处）
- worklog Task 18-21 全记录；lang-design 03-macro-system TD-007 注记回写

## v0.1.0-r5（2026-09-10）——Stage 1 批次 B：TD-002 符号值 + 标准库最小集

**SOP 流程**：批次 B 按 plan.md §5 序列推进；MUV 22-a（TD-002）与
MUV 22-c（标准库最小集）各走完整内循环（22-b TD-004 按重排裁定留批次 E 收口批次整体推进）。

### 交付（2 MUV）

- **TD-002 符号部分已解决**：`(quote sym)` / `'sym` → 符号值——全链落地：
  - `LiteralValue::Symbol(Rc<str>)`（kerf-core；存剥离卫生后缀的基名，
    Racket 语义近似——与 resolve_hygiene_fallbacks 同一 $hyg$ 剥离口径）
  - `Value::Symbol`（kerf-vm）+ eq? 按名相等 + type_name "symbol"
    + render（裸名）/ 序对渲染（slot_terminal/render_slot）
  - `BcConst::SymLit`（kerf-compiler 常量池；与全局名索引 `Symbol`
    变体语义严格区分——§11 接口隔离）
  - `HeapObj::Symbol` / `ValueSlot::Symbol` / `Heap::alloc_symbol`
    （kerf-runtime；符号入序对 + car/cdr 往返）
  - 双路径同步：VM `const_to_value` 与 eval `eval_literal`；
    宏模板内 quote 符号卫生后缀 datum 层剥离（实测验证）
- **向量 datum 仍显式报错**（TD-002 向量部分开放，后续阶段）

### 质量口径

- §3.2 全绿：build --release 0 警告 / fmt 零 diff / clippy -D warnings
  零警告 / test --release **324:0:1**（304 基线 + 20：TD-002 5 +
  stdlib 15）/ 审计集 41/41 复跑 EXIT 0
- 负向锚点迁移：符号 datum 报错负例 → 符号值语义负例（negative_vm
  symbol_value_misuse 6 case 实跑校准：算术/条件/序对/比较位置）；
  全局正负比 1:3.1 维持（stdlib 负例 172 case 三维矩阵）

### 追加交付（MUV 22-c：标准库最小集）

- **24 项新内置函数**（48 项总量；07 §3.3 阶段门条件 3：列表/字符串/I/O
  各 ≥8——driver `register_globals` 注册，语言核心零内置原则不变）：
  - 列表操作（8）：`length`/`append`/`reverse`/`list-ref`/`list-tail`/
    `member`/`assoc`/`last-pair`——堆序对链遍历，nil 终结契约；
    `member`/`assoc` 按 `eq?` 查找
  - 字符串处理（10）：`str-length`/`str-substring`/`str-index-of`/
    `str-contains?`/`str-prefix?`/`str-suffix?`/`str-upcase`/
    `str-downcase`/`string->symbol`/`symbol->string`——字符索引
    Unicode 安全（`str-length "héllo"` = 5）；符号互转联动 TD-002
  - 基本 I/O（6）：`newline`/`write-string`（通道层 `write_stdout` 新增）/
    `read-int`/`read-num`（行解析，失败结构化报错）`/`error`/`assert-eq?`
- **高阶函数（map/filter/foldl/for-each）显式推迟 B3**：用 kerf 源码
  preamble 实现是 B3「Reader kerf 重写」的自举验证命题本体（§12
  最优>最小：Rust 抢实现移除 B3 验证内容；源码拼接方案的 Span 污染
  为真实 P1 缺陷，worklog 22-c 记录裁定依据）
- **修复伴随缺陷 2 项**（std 函数实跑发现）：`list`/`reverse` 空参
  曾返回 `(nil)` 包装形态（堆 nil 槽包成序对）——改 nil 值形态与 `'()`
  一致；`list-tail` k=0 曾对非 list 输入静默返回（Racket contract
  严格语义：每步形态校验）
- 测试：stdlib_tests 15 函数（正例 59 断言 + 负例 172 case——元数/
  类型/边界三维矩阵 + 类型全扫描）；全局正负比 1:3.1 维持

### 文档回写（合并）

- 09-stdlib v5.3（48 项清单 + 高阶函数 B3 推迟注记）/ capability-
  boundaries（48 内置）/ matrix 324 对账（stdlib 15 函数 172 负 case）/
  status r5 / tests/v0/stage1/plan.md + plan/stdlib.md 新建

## v0.1.0-r6（2026-09-10）——Stage 1 批次 B 收官：B3 自举 Reader（Reader kerf 重写）

**SOP 流程**：批次 B 收官按 plan.md §5 序列（Task 23 尾注）；MUV 24-a
（VM 宿主调用 API）→ 24-b（reader.krf + 桥 + 生产读路径切换）→ 24-c
（parity 套件）→ 24-d（交付闭环）各走完整内循环。

### 交付（B3：Reader 以 kerf 源码重写，在 Stage 0 VM 上运行）

- **reader.krf**（`kerf-driver/src/bootstrap/reader.krf`，~430 行 kerf 源码）：
  完整词法 + 语法逻辑——`lex-src` / `parse-tokz` 两入口（与种子
  `lex_source`/`parse_tokens` 接口形状对齐，§11）；尾调用链形态的错误
  传播（err 标签值返回，不依赖异常）；高阶函数序章（map/filter/foldl/
  for-each 以 kerf 源码实现——r5 裁定「自举验证命题本体」的交付面）
- **VM 宿主调用 API**（24-a）：`kerf_vm::call_closure`——函数入口帧语义
  （底帧 RET = 程序化返回值）；跨程序闭包显式拒绝（P3 否决的运行时面）；
  错误追踪与 run_program 同构
- **自举桥**（`kerf-driver/src/bootstrap.rs`）：种子管线编译 reader.krf
  （thread_local 惰性加载，无递归）+ 值树 ↔ Token/Stx 转换 + 数字文本
  同源 parse（i64/f64 转换是宿主类型边界）+ 持久堆（GC 根集含全局）
- **生产读路径切换**：`compile_front`（run/eval/compile_source 全管线）
  与 `dump_tokens`/`dump_stx` 经自举 Reader；种子（kerf-reader）保留为
  引导实现 + parity oracle（`compile_front_seed`）
- **+4 自举 Reader 原语**（52 项总量）：`str->pos-chars`（(字节偏移 .
  单字符) 列表）/ `char-whitespace?` / `char-alphabetic?`（Unicode 属性）/
  `str-int-valid?`（i64 域——扫描序内溢出前置校验，维持首错位置 parity）
- **错误次序契约**：数字溢出在 VM 词法扫描序内前置报错（与种子一致）；
  数字文本最终转换桥侧同源 parse（消息与 Span 逐字节一致）

### parity 验收（bootstrap_reader_tests：28 函数，正 87 / 负 307 case）

- **Stx 树等价**：datum + Span 递归比对（符号按名——两实现 intern 次序
  不保证一致）；语料 = 种子 reader 测试全集 + 奇异边界（省略号族/
  多行字符串/科学计数法/前导零/Unicode 标识符/NFC 组合归一）
- **错误 parity**：消息 + Span 逐字节（28 负例语料 + 系统化矩阵 266：
  未闭合深度 1..30 / 括号错配矩阵 / 非法转义 32 字符（含多字节——
  种子 eo+1 字节口径逐字节复刻）/ 贪婪数字 10×5 矩阵 / 溢出扫描
  19..26 位 × 3 符号 / 非法字符 8×3 上下文 / 双错误次序 / 注释嵌套）
- **双错误次序**：溢出 + 后置词法错误（首错 = 溢出）与前置错误反向——
  两实现首错位置逐字节一致
- **高阶函数直测**：map/filter/foldl/for-each 经自举桥（builtin 作 f——
  跨程序安全的函数值）
- **防误收断言**：全部负例先断言种子确实报错（`assert_negative_parity`）

### 质量口径

- §3.2 全绿：build --release 0 警告 / fmt 零 diff / clippy -D warnings
  零警告 / test --release **356:0:1**（324 基线 + 32：VM call_closure 4 +
  parity 28）/ 审计集 41/41 复跑 EXIT 0
- **356 全套件（含全部既有负向消息断言）经自举 Reader 执行**——整体
  行为等价的实证（孤立正确 → 集成正确的 §7 防崩验证）
- 全局正负比 **≈1:3.2**（负 1000 case / 正 ≈311；§9.4.3 门限维持）

### 技术债登记

- TD-021（P3）：高阶函数用户面注入缺载体（P1/P3/P5 否决——批次 E 模块系统）
- TD-022（P3）：自举 Reader 帧消耗 O(源字符数)（无 TCO 边界——Stage 2 决策点）

### 文档同步

07-bootstrap v5.3（§3.2 B3 交付注记）/ 09-stdlib v5.4（4 原语 + hof
注记）/ 02-syntax-model（B3 双实现注记）/ stage-1 plan §5（B3 ✅）/
matrix r6 对账 / tests/v0/stage1/plan.md（parity 套件行）/ status r6 /
data-flow（自举 Reader 读路径）

## v0.1.0-r7（2026-09-10）——Stage 1 批次 C：类型检查器 + 编译缓存 + TD-016/013

**SOP 流程**：批次 C 按 plan.md §5 序列（Task 24-d 尾注——批次 B
收官后承接）；MUV 25-a（类型检查器本体）→ 25-b（编译缓存）→ 25-c
（TD-016 收紧 + check CLI）→ 25-d（TD-013 设计批）→ 25-e（交付闭环）。
L3 全量内循环（跨 kerf-compiler/kerf-driver/kerf-syntax/CLI/web 五面）。

### 交付一：保守静态类型检查器（25-a——§21.6 循环依赖缓解落地）

- **kerf-compiler/src/typecheck.rs**：`check_program(core, builtin_sigs, table)`
  → `Vec<Diagnostic>`（E0005）。**保守性契约**：只报告静态确定的错误——
  每条规则触发时对应程序运行期必然以同类错误失败；动态信息不足一律
  Unknown 跳过（误报 = P1 缺陷的工程口径）
- **R1-R8 规则集**：if 条件非 bool / 算术非数值 / 比较族（TD-016
  全操作数口径 + TD-011 字符串边界）/ not / car-cdr 非序对 / 不可调用
  值 / 元数（字面量 lambda + 内置签名）/ 字符串符号族
- **内置签名表注入**（§2.3-10 唯一可信源）：builtins.rs 的
  BUILTIN_SIGS（49 项）与 register_globals 同文件维护——签名表 ⊆
  注册表双向防漂移锚（`builtin_sigs_subset_of_registered` /
  `builtin_sigs_cover_operator_families`）
- **多错误收集**：全量诊断按 Span 次序（TD-013 设计的首个实证消费面）
- **深度预算** MAX_CHECK_DEPTH=512（Reader 256 上限 2×；debug 帧实测
  ~1 KiB，2000 深度实测溢出 2 MiB 栈——程序化构造单测存档）
- **消费面**：`kerf check` 子命令（编译 + 静态报告 + 缓存观测，发现
  问题 exit 1）+ `check_source` 库 API + web /api/check

### 交付二：编译缓存（25-b——13 §3.1.4 三方法规格做实，§21.3 条件 4）

- **InMemoryCompilationCache**（kerf-driver/src/cache.rs）：冻结 trait
  `get_cached`/`store`/`invalidate` 行为规格 1/2/3 逐条测试锁定 +
  管线富入口（`lookup_front`/`store_front`——完整前端输出含符号表/
  源映射/模块簿记；快照语义克隆）
- **SHA-256 内容寻址**（hash.rs 零外部依赖自实现，FIPS 180-4；NIST
  标准向量锚定）：`CacheKey{source_hash 截断 u64, config_fingerprint
  = 阶段种子 + 文件名}`（位置信息入指纹——SourceMap 产物等价性要求）
- **管线接线**：run/eval/compile/check 经 `compile_front_cached`
  （同源二次命中——eval 共享 run 条目；错误路径不缓存；`cache_stats`
  观测 + `set_cache_enabled` 基准对照开关）
- **确定性证明**：`cached_program_equals_fresh_compile`——命中产物 ==
  新编译产物（BcProgram PartialEq 逐字段）

### 交付三：TD-016 收紧（25-c——链式比较全操作数前置校验，双侧）

- 运行时面：`cmp_builtin` 前置全参数数值校验（全字符串+排序族 →
  TD-011 消息；其余首个非数值 → `{op} 需要数值`；**既有两参消息
  逐条兼容**——负例矩阵全绿回归）
- 静态面：R3 规则同口径（Ordering / NumOrAllStr）
- 语义收敛：`(< 3 1 "a")` 静默 false → 结构化错误（FS-5 边界闭环）；
  `(= 1 2 "s")` 同理；`(< "a" "b" 1)` 混串消息统一为「需要数值」
- 09-stdlib §2 v5.5 重写 + negative_vm_tests 语义边界注记更新

### 交付四：TD-013 设计批（25-d——多错误收集设计冻结）

- docs/develop/v0/stage-1/multi-error-recovery-design.md：恢复粒度 =
  形式级（表达式级不恢复——半展开状态重建成本）；恢复机制 = 编译期
  控制流（非 effect——§11 接口隔离裁定）；收集上限 128 + 截断提示；
  输出按 Span 次序；实现绑定批次 E（验收标准 5 项）
- 登记册 TD-013 → 设计完成；TD-016 → 已解决（r7）

### 质量口径

- §3.2 全绿：build --release 0 警告 / fmt 零 diff / clippy -D warnings
  零警告 / test --release **408:0:1**（356 基线 + 52：typecheck 24 +
  cache 13 + stdlib TD-016 2 + compiler 单元 3 + builtins 签名 2 +
  cache 单元 6 + hash 单元 2）/ 审计集 41/41 复跑 EXIT 0
- 全局正负比 **≈1:3.2 维持**（负 1077 / 正 ≈340 case；r7 typecheck
  负例 68 全部携带静态确定性反向锚——静态报错程序实跑必报 Run 错）
- **零误报保守性机械验证**：examples/ 6 程序 + 既有 408 全套件零
  新诊断（缓存行为等价 + 检查器保守性双证明）

### 下一步（批次 D，plan §5）

Effect 内部最小实现（编译器错误恢复用）+ 能力 I/O 基础传递
（13 §3.1.3 规格）。

## v0.2.0-r8（2026-09-10）——Stage 1 批次 D：能力 I/O 基础传递 + 内部效应做实 + 用例运行器

### 交付一：Effect Handlers 编译器内部做实（12-roadmap §2.4.3 第二级）

- `kerf-driver/src/effects.rs`（新，414 行）：**类型化一次性逃逸层**
  `handle_escape<R,T>` / `perform_escape<T>`——载荷从任意嵌套深度
  上展开至最近同类型边界，零签名污染（「任意流程节点能力」的机械
  实现）；线程局部深度计数 + `catch_unwind`/`resume_unwind` std-only
  载荷逃逸 + 私有载荷类型判别的 panic hook 过滤（效应控制流零噪声、
  真实 panic 照常穿透——12 单元测试锁定）
- **冻结契约层** `InternalEffectSystem`：reserved.rs `EffectSystem`
  P3 形状的真实现（与 Probe 测试构成「可编译/可承载」契约双证）；
  语言面保持 P3（D1 裁定——语言级 perform/handle 语义留 Stage 2）
- VM 帧 `ext1` 槽位不激活（Stage 2 语言级效应时启用）

### 交付二：能力模型 I/O 基础传递（13 §3.1.3 四条款做实）

- **声明形式 `(require io read|write)`**：`CoreExpr::Require`（零运行时
  语义——求值恒 nil 双路径一致 / 编译产 PushNil / IR 降级 nil 节点）；
  幂等集合语义；核心冻结边界精确化（「语义原语集冻结 + 声明变体可
  追加」——01-core-forms §6）
- **R9 保守静态权限验证**：门控内置名（read-line/read-int/read-num/
  print/newline/write-string）任意位置引用未声明 → **E0006 编译期
  错误**（front 管线 run/eval/check/compile 全路径单一验证点；用户
  接管豁免 + 卫生回退基名判定——零误报纪律）
- **令牌授权面**：`mint_read_token`/`mint_write_token`（pub(crate)
  构造面控制）+ `IoGrant` 按声明铸造 + `StdCapabilityIO`（冻结 trait
  的 stdio 实现）+ `register_globals` 能力参数化（未声明即不注册——
  fail-closed）
- **FS-4 修复**：read-line 元数校验补齐（能力参数化重写时顺带——
  negative_vm_tests 存档断言激活）

### 交付三：用例运行器 `kerf test`（测试面即语言面）

- `test_source` API + CLI `kerf test` 子命令：前置形式（define/set!/
  module/require）逐用例重放 + 顶层表达式 = 用例（值非 #f = PASS）；
  **短路 + 恢复**（效应系统消费面——case 内任意深度失败即停、边界
  捕获后下一 case 续跑）+ 状态隔离（每 case 全新环境与堆）
- 设计文档：[11-testing §4](docs/lang-design/11-testing.md)

### 交付四：文档回写 v5.4（lang-design 七文件 + sop.md v11.1）

- lang-design：00（v5.4 修订记录 + next.md 吸收审计结论）/ 01（§6
  require 声明形式设计）/ 02（Token 叶级 45）/ 09（v5.6 能力门控）/
  11（§4 用例运行器）/ 12（v5.4 演进矩阵状态）/ 13（v5.4 双做实注记）
- sop.md v11.1：§21.9 矩阵现状对账（四项 ✅ + 类型检查器提前引入
  偏差登记）+ §16.1 变更日志 + §21.7.1 P4 行注记
- 测试计划：capability.md / test-runner.md（新）+ matrix.md r8 全量
  对账（含 r7 陈旧计数修正）

### 质量口径

- §3.2 全绿：build --release 0 警告 / fmt 零 diff / clippy -D warnings
  零警告 / test --release **476:0:0**（408 基线 + 68：capability 24 +
  test_runner 18 + driver 单元 25（effects 12 + capability 13）+
  negative_vm +1（FS-4 激活））/ 审计集 41/41 复跑 EXIT 0
- 全局正负比 **≈1:3.15 维持**（负 ≈1118 / 正 ≈355 case；E0006 能力
  权限码逐条断言——第七族结构码就位）
- 确定性纪律：read-line EOF 语义经子进程探针（Stdio::null()——不依赖
  运行器 stdin 形态）

### 下一步（批次 E，plan §5）

Expander kerf 重写 + TD-004 scope-set 解析收口 + TD-021 hof 用户面
注入（模块机制）→ Stage 1 门审查（§7.3 + §21.3 四条验收）。

## v0.2.0-r9（2026-09-10）——next2.md 吸收轮 + 测试入口架构重构 + lang-design v5.5

### 交付一：next2.md 五轮讨论增量吸收（lang-design v5.5）

- 与 next.md 轮（零缺口）不同，本轮识别**实质缺口**（Perform/Handle/de Bruijn/四层正交/六 IR/MLton/Koka/comptime 于既有文档集零命中）并全量吸收至九文件：01 §7（核心原语 9 vs 8 vs 7 数量真相 + 命名精确性表 + 效应原语化 Stage 2 演进对照裁定——核心冻结不动摇）/ 14 §4（2026 前沿五维成熟度矩阵 + comptime 生产就绪档登记 + 批判审查史 + 可行性评分对照）/ 15 §5（四层正交 crate 对照——r8 实现与 next2 最终版架构同构实证 + IR 六层演进表 + MLton 闭包/Koka 效应消除锚点）/ 12 §2.4.1（S 表达式量化背书 300 vs 3000 行 + 皮肤骨架论）/ 17（六原则对照映射）/ 18 §3（八条术语）/ 19（六行参考）/ 13（comptime 注记）/ 00（v5.5 修订记录）

### 交付二：测试入口架构重构（sop.md §8.4.6 v11.2——Cargo.toml 干净精要）

- **`tests/runner.rs` 单一总入口**：cargo 自动发现（零 [[test]] 配置），`#[path]` mod 树挂载 v0/stage-N 全部 18 个测试文件——阶段/plan/gate 目录语义不变，仅入口收敛；**Cargo.toml 125 行 → 50 行**（[[test]] 18 块清零，仅保留 [[example]] 嵌套声明）
- 共享辅助单实例化：`tests/common/` 由每文件 `mod common` 重复加载（clippy duplicate_mod）改为 runner 单实例 + `use crate::common`（10 个测试文件迁移；3 个未使用者清理）
- sop.md v11.2：§8.4.6「测试入口架构意图」+ 强制规则 8（禁止 [[test]] 逐文件声明）+ §9.1 树更新 + §16.1 变更日志；testing-guide.md r9（运行命令与编写规范更新）
- **测试总数 476 与逐模块计数完全不变**（组织收敛零语义变化）；选择性运行 `cargo test --test runner <module>::`（模块路径即过滤器）

### 质量口径

- §3.2 全绿：build 0 警告 / fmt 零 diff / clippy -D 0 / test **476:0:0**（runner 单二进制 301 + 单元 175）/ 审计集 41/41 EXIT 0
- 集成测试单二进制运行时间 6.95s（18 二进制顺序执行 → 1 二进制并行——加速且零重复编译）

## v0.3.0-r10（2026-09-10）——next3.md 七轮吸收 + 内部语法三原则落地 + 架构合规审计

### 交付一：next3.md 七轮讨论吸收（stage0.md v6.0 + lang-design v6.0 双层收敛）

- **upload/stage0.md v5.0 → v6.0**（原语批判演进版，3706 → 4216 行）：新增 §6.9-§6.12（9/8/7 数量真相与不可消除性证明 / 2026 理论前沿全景与整合决策矩阵 / 四层正交架构与 8 原语重设计 / 最终修正（显式 continuation 类型 / 真解耦四层 / 六 IR / MLton 闭包三策略 / Koka 四阶段效应消除）+ 可行性审查（五目标 A+/A/A/A+/B+）+ §6.12.6 本项目收敛裁定）；§7.3 表面语法决策（S 表达式 = 工程捷径，300 vs 3000 行，皮肤/骨架分离，分阶段语法策略）；§7.4 内部语法设计（告别 `#%` 派生关键词——类型安全 ADT 三原则 + 2026 内部 AST 完整 Rust 定义 + 旧→新迁移映射）；设计原则第 29-31 条；附录 A +17 术语 / E +8 引用 / F 设计讨论演进记录（七轮档案）
- **lang-design v5.5 → v6.0**（九文件回写 + 12 维度系统审查）：01 §8 内部语法设计（三原则 + 当前实现三项合规核验 + 十二行迁移映射 + §8.5 测试锚点）/ 02 §8 表面语法决策 / 15 §5.3 表面-内部分离 = 四层正交第五轴 / 14 §4.4-§4.5 ADT 对照与演进链 / 17 原则 29-31 + 附表四处编号误引修正 / 18 §4 六术语 / 19 §5 三引用（de Bruijn 1972 / Flanagan 1993 / Plotkin & Pretnar 2009）/ 12 §2.5.1 评估清单补齐（悬空引用修复）/ 00 v6.0 修订记录
- **审查修复六类缺陷**：版本号漂移 ×3（01/18/19 头部停在 v5.0/v5.4）、12 悬空登记、13 Version 字段污染、17 附表误引 ×4、**打包自包含性**（上游蓝图存档 docs/stage0.md 243,952 B + lang-design 三处外链改内）
- **收敛裁定**（stage0 §6.12.6 / 01 §8.2-§8.3 / sop v11.3 三处一致）：Stage 0-1 冻结 9 原语（+Require）不变；8 原语形态 + 四层/六 IR/MLton 闭包/Koka 效应消除 = Stage 2「目标语言完整化」评估清单；三原则即刻生效为架构验证基准

### 交付二：sop.md v11.3（原则 29-31 接入流程权威）

- §2.2 三十一条（29 命名行为导向 / 30 类型安全优于命名安全 / 31 表面-内部语法严格分离——各含违反示例）+ 协同关系块第五组
- §8.4.5 文档清单 + §9.1 docs 树登记蓝图存档；§8.4.6 补设计侧锚点接线（lang-design 测试锚点节 ↔ tests/ 树双向锚定）；§16.1 v11.3 变更日志；反臃肿四问全过

### 交付三：架构合规审计落地（代码 + 测试）

- **tests/v0/stage0/plan/architecture_audit_tests.rs（+4）**：十变体穷尽 match 冻结证明（编译期机器证明——新增/删除变体即编译失败）+ 逐变体 Span 独立携带 + kind_name 全映射互异 / Reader 产出 Stx 类型级隔离 / Expander 唯一 Stx→CoreExpr 桥（翻译发生证明）/ 同源双编译 render_core 全等（表面语法可替换性确定性基线）
- **kerf-core expr.rs 迁移映射文档块**（冻结边界 + 三原则合规 + Stage 2 演进目标——§8.4.5 文档随代码）
- **九 crate 五正交轴定位文档**（15 §5.3 落地：span=元数据基座 / syntax=表面语法层 / reader=轴 1 唯一桥接点 / core=骨架 / expander=翻译执行面 / compiler=轴 2 桥接 / vm=轴 3/4 运行时消费 / runtime=通道层 / driver=组合根）

### 质量口径

- §3.2 全绿：clean + build --release ✓ / check 0 error 0 warning / fmt 零 diff / clippy -D 0 / test --release --workspace **480:0:0**（+4 架构审计；单元 175 + 集成 305）
- 矩阵 r10 口径（逐模块计数与 r9 完全一致 + 新增套件）

### 下一步（批次 E，plan §5）

Expander kerf 重写 + TD-004 scope-set 收口 + TD-021 hof 用户面注入 → Stage 1 门审查（§7.3 + §21.3 四条验收）。

---

## v0.3.0-r14（2026-09-11）——批次 E / E1-α：自举 Expander（核心形式 + 九糖，VM 上运行，parity 19 测试）

### 交付一：expander.krf——Expander 以 kerf 源码重写（07 §3.2「Expander（新语言子集）」α 阶段）

- **自举程序**：`bootstrap/expander.krf`（~910 行 kerf 源码，种子管线编译为 **208 原型 / 4685 指令**——迄今最大 kerf 程序）：9 核心形式（lambda/if/set!/define/begin/module/quote/require）+ 九糖（let/letrec/let*/cond/and/or/when/unless/while）+ 内部 define 提升（含切分期名校验/值校验/错误短路序镜像）+ **TD-004 fresh-scope 深注入与作用域集携带**（r13 语义的 kerf 侧镜像）+ trampoline 糖链循环 + retag use-site 替换语义 + quote datum 转换（$hyg$ 剥离经字符表模式匹配——算术索引不经 int 参数内置）
- **值桥**：`bootstrap_expander.rs`——Stx → VM datum 节点（tag s e scopes，作用域集 int 列表）→ VM core 节点 → CoreExpr（作用域集排序去重重建；符号经用户表 intern）；共享 bootstrap.rs 走查辅助（§2.3-10 单一定义）；种子管线加载（compile_front_seed——Rust Reader/Expander 编译 expander.krf，无递归）
- **parity 验收**（tests/v0/stage1/plan/bootstrap_expander_tests.rs，19 测试）：结构（原语形态 + Span(start,end) + **作用域集 + param_scopes** 递归一致）+ 错误（消息 + Span 逐字一致——含提升路径短消息口径）+ 行为面（自举展开产物经 compile + VM 执行与种子全管线一致——**「语言能表达自身前端」的自举验证命题在 Expander 子集上成立**）；509 基线零回归 = **528:0:0**
- **E1-α 边界**（显式报错不静默，§2.3 原则 4）：define-syntax → 边界错误（宏属 E1-β）；糖产物 Span 展开代次不参与 parity 判据（r6 Reader 同口径）；影子路径——生产展开仍走 Rust 种子（切换随 E1-β 宏收口）
- **接口最小放宽**：`IoGrant::none()` pub 化（宿主/测试侧构造无能力全局环境的合法入口；令牌铸造面保持 crate 私有）

### 交付二：质量口径

- **§3.2 六命令实跑全绿**（clean 起步）；审计集 §7.3.1 EXIT 0；CLI 冒烟 fib=75025 + test 2/2 + **expander.krf 自检 ok（208 原型/4685 指令）**
- 文档同步：matrix 528 对账（正例 8 套件 + 负例 6 套件 + 边界 1 + 行为面 3）+ stage-1/plan.md 批次 E 行 + docs/tests/v0/stage1/plan/bootstrap-expander.md 测试计划

### 下一步（批次 E 续）

E1-β 宏收口（syntax-rules + HygieneCtx α 重命名 + 展开深度计数 + 宏产物 Span 代次 + **生产路径切换** + 基础宏定义）+ TD-021 hof 用户面注入（随模块系统）→ Stage 1 门审查（§7.3 + §21.3 四条验收）。

---

## v0.3.0-r13（2026-09-11）——批次 E 启动：TD-004 作用域集解析收口（Racket 式 (name, scopes ⊆) 双路径 + 509 全绿）

### 交付一：TD-004 作用域集解析（P2 清偿——E1 Expander kerf 重写的前置语义基座）

- **展开器**：`ExpandCtxt` 作用域分配器（fresh_scope 单调递增）+ 绑定形式（lambda）fresh scope 深注入——绑定器与全体体形式（`Stx::add_scope_to_all`，新 kerf-syntax 公共 API）；注入先于体展开 ⇒ 宏产物作用域 ⊇ use-site ⊇ {fresh}，自由标识符穿透保持；α 重命名（$hyg$N）保留为第二道卫生保险
- **核心层**：`CoreExpr::VarRef/SetBang` 携带引用作用域集（`scopes`）+ `Lambda.param_scopes` 携带绑定作用域集（与 params 平行）；`free_var_occurrences`（名 + 首现作用域集）——闭包捕获分析与名称版共用单一定义（名称版 = 其投影）
- **编译器**：`ScopeFrame` 绑定项化（名 + 作用域集）+ `resolve_var(name, scopes)` 帧序 + 帧内 max-cardinality 子集匹配；捕获描述符携带命中绑定的绑定作用域集（内层原型经同一子集匹配命中捕获槽）
- **eval 双路径**：`Env` 绑定项化 + `lookup/set` 同一子集匹配口径（T1 一致）；`ClosureValue::Eval` 携带 param_scopes，`apply_value` 经 `define_scoped` 绑定；**空作用域集绑定 ⊆ 任意引用集** ⇒ driver 根环境全局/内置天然兜底，`$hyg$` 基名回退（双侧）降级为回退路径
- **语义判别点**（与旧名称基差异）：名称匹配但作用域不匹配 ≠ 解析命中——按 Racket 语义判为未绑定（VM → 全局兜底后「未绑定的全局变量」；eval → 「未绑定变量」）

### 交付二：质量口径

- **509:0:0**（500 基线零回归 + 9 锚点新增；单元 183 + 集成 326）；§3.2 六命令全绿（clean 起步实测）；审计集 §7.3.1 配比满足 EXIT 0；CLI 冒烟 fib 75025 + test 2/2
- 锚点 = tests/v0/stage1/plan/scope_set_tests.rs（正例 5 双路径：shadowing/嵌套 shadowing/闭包捕获/set! 词法命中/子集对照；负例 4：作用域不匹配未绑定 VM/eval/set! + 宏引入不捕获）+ docs/tests/v0/stage1/plan/scope-set.md 测试计划
- 文档同步：tech-debt-register TD-004 → 已解决（r13）+ matrix 509 对账 + 03-macro-system 实现状态注记 + stage-1/plan.md 批次 E 行更新

### 下一步（批次 E 续）

Expander kerf 重写（E1——scope-set 语义基座就绪后重写，Rust 实现保留为 parity oracle）+ TD-021 hof 用户面注入（随模块系统设计）→ Stage 1 门审查（§7.3 + §21.3 四条验收）。

---

## v0.3.0-r12（2026-09-11）——接口预留完整性扩展轮（next4 第八轮：docs 三层 + 预留层代码冻结 + 500 全绿）

### 交付一：next4.md 第八轮三层吸收（stage0 v6.1 + lang-design v6.1 + sop v11.4）

- **stage0.md v6.0 → v6.1**（4216 → 4757 行）：新增 §9.3-§9.5 接口预留完整性审查（覆盖度评估六类 ~70% → 10 个新识别关键接口：LSP/IDE 查询/调试信息/FFI 边界/增量编译查询/编译器即服务/多目标后端/包管理/AI 辅助 + 效应扩展/能力委托增补 → 完整预留矩阵 14 项 → P0-P3 优先级 + ~3 周成本核算 + 全景图）；§9.2 边界调和（LSP/FFI 重分类——「完全推迟 = 未承诺采用」语义精确化）；§7.1 矩阵 4→14 项；§21.9 预留成本核算；原则 32「预留留白」；附录 E.8 + F 第八轮登记
- **lang-design v6.1**（八文件）：13 主体（§1.1.1 矩阵 4→14 + §3.2 调和 + §3.3-§3.5 完整审查 + 测试锚点注记）；12 §2.9 成本核算 + Phase 4 更新；10 LSP 重分类注记；08 CodegenBackend 预留注记；15 预留层注记 + 查询双预留互链；17 原则 32 + 附三；18 §5 术语 13 条；19 §6 引用 3 条；00 v6.1 修订记录；kerf/docs/stage0.md 存档再生成 v6.1
- **sop v11.3 → v11.4**：§2.2 三十二条（第 32 条预留留白 + 违反示例）；新增 §21.12 接口预留时机（能力引入时机 vs 数据结构冻结时机叠加规则 + §13.1/§6.2 双接线 + 成本口径）；§21.11 风险表「接口预留过多/不足」行对账（四项 → 14 项矩阵清单）；§8.4.5 查询表 + 接口预留行；§16.1 v11.4 行
- **审计**：关键词矩阵 74/74（stage0）+ 29/29（lang-design 全集）全命中；LSP/FFI 陈旧行 0；全 20 文件锚点 GitHub 算法校验零坏链；三面原则编号链一致（29-31 → 32）

### 交付二：预留层代码冻结（reserved/ 模块 4→14 项，P0 位置跨 crate 断言）

- **重构 reserved.rs → reserved/ 模块目录**（§8.4.6 落位 + 既有四接口零迁移）：`mod.rs`（既有 4 项 + 文档 v6.1）+ `toolchain.rs`（P0 三项：LanguageService/IncrementalAst + DebugInfoGenerator/DebugTraceable + QuerySystem/Query；P1：CompilerService/Serializable；P2：PackageManager/ExternalModule/AiAssistant）+ `ffi.rs`（P1：ExternalType/FfiCall/FfiBoundary——GC pin/unpin 隔离协议 + 自举合规注记）+ `codegen.rs`（P1：CodegenBackend/WasmBackend——目标中立类型级证明）
- **P0 数据结构位置对账**（集成断言 12 项）：每节点 Span ✓（CoreExpr::span()）/ 绑定作用域 ✓（NodeMetadata.scopes）/ 稳定 ID ✓（ir::NodeId）/ 函数边界 ✓（BcProgram::protos）/ 调试帧槽 ✓（VM ext3）/ 内容寻址口径统一 ✓（QueryDescriptor ↔ CacheKey）
- **测试 +20（480 → 500）**：Probe 冻结 8（单元：toolchain 3 + ffi 3 + codegen 2——「测试实现体编译通过 = 契约冻结」先例沿用）+ 跨 crate 位置断言与形状行为 12（集成 tests/v0/stage1/plan/reserved_ext_tests.rs，经 runner.rs 总入口挂载）

### 交付三：质量口径

- cargo clean + build --release / check / fmt --check / clippy --all-targets -D warnings 全绿；**test --release --workspace 500:0:0**（单元 183 + 集成 317）；审计集 §7.3.1 配比满足 EXIT 0；CLI 冒烟 fib 75025 + kerf test 3/3
- 语义核心冻结零变动（CoreExpr/9 原语/操作语义）；既有四预留契约签名零迁移（reserved_signatures_are_frozen 先例测试保持）
- 下一步（批次 E，plan §5）：Expander kerf 重写 + TD-004 scope-set 收口 + TD-021 hof 用户面注入 → Stage 1 门审查（§7.3 + §21.3 四条）
---

## v0.3.0-r11（2026-09-10）——吸收完整性复核轮 + 核心原语三面同步（docs / web / 打包）

### 交付一：next*.md 三源吸收完整性复核（审计轮，零实质缺口确认）

- **三源关键词矩阵 + 章节结构映射审计**：next.md（4 文档：超越 Lisp 范式替代方案 / 五大能力历史与创新原则 / 2026 深度设计 / Stage 0 完整能力模型）→ stage0.md Part II §4 + Part III（拆分至 lang-design 14/13/16）；next2.md（六轮：8vs9 真相 / 前沿全景 / 四层正交 / 最终修正 / 可行性 / 表面语法）→ stage0 §6.9-§6.12 + lang-design v5.5；next3.md（= next2 六轮 + 第七轮内部语法重构）→ stage0 v6.0 §7.3/§7.4 + lang-design v6.0——**全部特征概念在吸收面命中**（三原则 / CoreExpr ADT / 迁移映射 / 派生关键词批判 / 9-8-7 真相 / 四层正交 / 六 IR / MLton 闭包 / Koka 效应消除 / de Bruijn / continuation 类型安全 / comptime / S 表达式决策 / 皮肤骨架分离 / 联合交叉类型 / 行多态 / Janet/Céu 归入 PEG/同步语言概念族）
- **存档对账**：kerf/docs/stage0.md（上游蓝图存档）与 upload/stage0.md 同为 v6.0（+496B 归档说明头，预期设计）；sop v11.3 三十一条（29-31）+ §16.1 变更日志 + §8.4.6 测试入口架构意图全部在位

### 交付二：核心原语相关变动三面同步（用户指令专项——P2 文档债修复）

- **kerf README.md**（§8.4.5 文档随代码——r6-r10 陈旧对账）：版本 v5.4/v11.1 → v6.0/v11.3；状态行批次 D → r9/r10 吸收审计轮；架构行补 能力参数化 / 内部效应 / kerf test；特性清单补 **核心原语演进登记**（9 冻结 + 8 原语 Stage 2 语义等价映射 + 三原则）+ 自举 Reader（r6）+ 能力门控 I/O（r8）+ 内部效应（r8）四条；质量状态 r7/476 → r10/480（runner.rs 单一总入口口径）
- **develop plan.md**：批次表新增「吸收/审计轮」行（r9 测试入口重构 ✅ + r10 next3 双层吸收 + 架构合规审计 4 测试 480:0:0 ✅ + 语义核心冻结零变动 + 三面同步验收）
- **stage-0 status.md**：测试总数 356（r6 陈旧）→ 480 全绿（r5-r10 增量分项）；§5 新增架构合规审计测试与核心原语演进登记两条
- **web（kerf 官网）**：能力区新增「核心原语 · 语义核心」主内容区块——9 冻结原语网格（名称/元数/语义，与 lang-design 01 §2 最终定义逐项对齐）+ 核心原语演进登记卡（§6.12.6 收敛裁定 / 9-8-7 数量真相 / 9→8 迁移映射表 8 行 / 内部语法三原则 / 四层正交 + 第五正交轴 / 架构合规审计 4 测试）；路线图 Stage 2 补核心原语演进迁移要点；此前 v6.0/v11.3/r10 版本口径与 footer 包清单已同步（r10 轮交付，本轮复核确认）

### 质量口径

- §3.2 全绿（本轮实跑）：clean + build --release 9.37s ✓ / check 0/0 / fmt 零 diff / clippy --all-targets -D 0 / test --release --workspace **480:0:0**（单元 175 + 集成 305——--workspace 口径，根 package 单跑为 305 属预期组织形态）
- 审计集 41 case EXIT 0（§7.1.1 七类全覆盖）；CLI 冒烟：fib 75025 + ⇒ 144 + kerf test 2/2 通过

### 下一步（批次 E，plan §5）

Expander kerf 重写 + TD-004 scope-set 收口 + TD-021 hof 用户面注入 → Stage 1 门审查（§7.3 + §21.3 四条验收）。
