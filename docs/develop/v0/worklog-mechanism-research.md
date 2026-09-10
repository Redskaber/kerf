# worklog 管理机制设计研究（31-e 前置检索）

> 任务编号：31-e-research（纯研究，不改代码）
> 目的：为「worklog 管理查询机制」（文件/目录结构型知识数据库，支撑 AI Agent 超长流程的知识查询/上下文检索/上下文存储）检索交叉领域的优秀设计，供 31-e 主设计吸收。
> 需求原型：时间维度（会话/日/周/阶段/里程碑）× 描述精度（摘要/详录）两轴多层级组织；每层目录/文件以「编号_概述」命名；每层附路由/索引文件 `l`；结构递归同构（`rec(dir(编号_概述) | file(编号_概述.md) | l)/`）。

---

## 1. 检索概览（查询词与命中数）

共执行 **22 次 web_search** + **2 次 page_reader 深读**（1 成功：Letta Filesystem 基准文；Anthropic context engineering 文页面抓取失败 502，但其要点由多条搜索快照交叉证实）。累计命中约 **130 条**结果，按五个领域分布：

| # | 领域 | 查询词（摘要） | 命中 |
|---|------|----------------|------|
| 1 | Agent 记忆 | MemGPT/Letta hierarchical memory paging | 7 |
| 2 | Agent 记忆 | LangGraph long-term memory semantic/episodic/procedural | 7 |
| 3 | Agent 记忆 | hierarchical memory summarization pyramid LLM | 8 |
| 4 | Agent 记忆 | temporal knowledge graph agent memory (Zep/mem0) | 6 |
| 5 | Agent 记忆 | A-MEM agentic memory Zettelkasten | 6 |
| 6 | 知识管理 | Johnny.Decimal 编号系统 | 8 |
| 7 | 知识管理 | Zettelkasten progressive summarization | 4 |
| 8 | 知识管理 | PARA method | 4 |
| 9 | 知识管理 | ADR architecture decision record index | 6 |
| 10 | 知识管理 | date-based folder hierarchy YYYY/MM/DD | 6 |
| 11 | 数据库 | LSM-tree levels/compaction | 8 |
| 12 | 数据库 | time-partitioned tables + partition pruning | 6 |
| 13 | 数据库 | salsa / rustc query system 增量计算 | 7 |
| 14 | 数据库 | B-tree / sparse index | 6 |
| 15 | 数据库 | log-structured merge tree 原始论文思想 | 6 |
| 16 | 数据库 | git bisect 历史二分 | 6 |
| 17 | 上下文工程 | context engineering agents（Anthropic/LangChain） | 6 |
| 18 | 上下文工程 | agentic grep-based retrieval vs RAG | 4 |
| 19 | 上下文工程 | CLAUDE.md 记忆层级 | 6 |
| 20 | 上下文工程 | session handoff / 长任务上下文接力 | 8 |
| 21 | 文件系统 | filesystem as database | 8 |
| 22 | 文件系统 | Letta Filesystem benchmark + sleep-time compute | 4 |

---

## 2. 各领域优秀设计理念与借鉴点（12 条）

### ① MemGPT/Letta：OS 虚拟内存式分层记忆，agent 主动换页
- **来源**：Packer et al., *MemGPT: Towards LLMs as Operating Systems*（arxiv，2023，被引 1600+）；letta.com / docs.letta.com（Memory & dreaming）。
- **机制**：把 LLM 上下文类比为 RAM，外部存储（会话历史、archival memory）类比为磁盘；agent 通过函数调用自主决定把什么换入 main context、什么换出归档。分层的目的不是「存得下」而是「找得到且省 token」。
- **借鉴点**：worklog 的「当前详录 vs 上层摘要」正是这种主存/外存分层；`l` 路由文件 ≈ 页表/目录（page table）。核心启示：**检索决策权交给 agent（读哪个文件），机制只负责让「换页」便宜**——即 l 文件要小、命名要能自解释。

### ② Letta「Filesystem All You Need」：纯文件 + agentic search 胜过专用记忆系统
- **来源**：*Benchmarking AI Agent Memory: Is a Filesystem All You Need?*（letta.com/blog/benchmarking-ai-agent-memory，2025-08-12，本次已深读全文）。
- **机制**：把对话历史原样放进一个文件挂给 agent，只给 grep / search_files / open/close 四个文件原语，gpt-4o-mini 即在 LoCoMo 长对话问答基准拿到 **74.0%**，超过 Mem0 专用记忆工具的 68.5%。结论：记忆效果取决于 agent 管理上下文的能力，而非检索机制本身；文件操作是模型训练数据中高度熟悉的原语。
- **交叉证据**：arxiv《Is Grep All You Need? How Agent Harnesses Reshape Retrieval》——inline 送达时 grep（词法）在每个 harness–model 组合上都强于 dense retrieval；Claude Code 生产实现即用实时 ripgrep 而非持久向量索引（pub.towardsai.net《The Secret Behind Claude Code's Retrieval》）。
- **借鉴点**：**直接验证了用户「文件/目录即知识数据库」的路线**，且明确无需引入向量库/embedding——只要保证：目录可枚举、文本可 grep、文件可增量打开。l 文件与「编号_概述」命名就是为这三原语服务的。

### ③ 递归摘要金字塔 + compaction/handoff：会话边界即压缩点，O(log N) 导航
- **来源**：*Episodic Memory Verbalization using Hierarchical Representations*（arxiv 2409.17702）：LLM 分段→摘要→递归生成更高层抽象；Medium《The key to salient episodic memory for AI Agents》：层级摘要结构使 agent 以 **O(log N) 查找替代 O(N) 扫描**；*Hierarchical Memory for High-Efficiency Long-Term Reasoning*（arxiv 2507.22925，H-MEM）；Anthropic《Effective context engineering for AI agents》（compaction：临上下文极限时把对话蒸馏为高保真摘要并重启新上下文）；aihero.dev /handoff 与 softaworks agent-toolkit session-handoff（把长会话压缩成交接文档供下一个 agent 冷启动）。
- **借鉴点**：会话→日→周→阶段→里程碑 的层级正是递归摘要金字塔；**每层摘要必须保留指向下层的指针**（压缩自哪些条目），才能既省 token 又不丢失回溯路径。本项目现状（每 Task 结束写 worklog section）恰好是「会话边界压缩点」的雏形，缺的是上层日/周/阶段压实。

### ④ LangGraph/LangMem 记忆类型学：semantic / episodic / procedural 三分
- **来源**：docs.langchain.com *Memory overview*；langchain.com *LangMem SDK*（2025-02）。
- **机制**：人类记忆三分——semantic（事实/知识）、episodic（经历/事件）、procedural（规则/流程），agent 记忆同样应区分三类，检索入口不同。
- **借鉴点**：worklog 条目天然混合三类：做了什么（episodic，走时间轴）、裁定/结论（semantic，如「9-8-7 真相」「冻结原语」）、SOP/规则（procedural，如 §0 启动协议）。`l` 路由文件宜按三类提供不同视图/字段，避免「只按时间找、找不到结论」。

### ⑤ Zep 时态知识图谱：事实携带有效期，支持 as-of 查询
- **来源**：*Zep: A Temporal Knowledge Graph Architecture for Agent Memory*（arxiv 2501.13956）；mem0.ai / supermemory.ai 时态图谱博文。
- **机制**：事实存为带 valid_from/valid_to 的时间戳三元组，能回答「X 当时是什么、现在是什么、何时变的」，优于 MemGPT 基线。
- **借鉴点**：kerf 的裁定会演进（如 r6-r10 文档债对账、9→8 迁移映射）。上层摘要条目应带**状态与有效期**（active / superseded by N / 收敛于 rN），时间分区天然支持 as-of 查询——这是纯向量记忆做不到、目录时间轴免费提供的。

### ⑥ Johnny.Decimal：编号_概述命名 + 稳定排序 + 扇出上限
- **来源**：johnnydecimal.com 及其文档（Introduction）；Reddit r/datacurator 与 lucaf.eu 实践评论。
- **机制**：每个目录/条目获得 `NN.MM` 形式 ID + 简述（如 `11.21 Health records`）；两级各限 10 个（00-09…90-99），总扇出硬上限 100；编号使 ls 输出顺序稳定且可口头引用（「在 15.03」）。
- **借鉴点**：与用户「编号_概述」命名**完全同构**，可吸收两点硬约束：(a) **每层扇出上限**（≤10-12），超限即再分层，防止单目录/单 l 膨胀；(b) 编号定宽零填充，保证**字典序 = 逻辑序**，`ls` 即有序索引。

### ⑦ Zettelkasten / A-MEM / ADR：原子条目 + 链接 + supersede 演化 + 状态索引
- **来源**：*A-MEM: Agentic Memory for LLM Agents*（arxiv 2502.12110，被引 1000+：note construction → link generation → memory evolution，Zettelkasten 启发的自组织）；zettelkasten.de《A Tale of Complexity》；martinfowler.com《Architecture Decision Record》与 hidekazu-konishi.com ADR 实践（「维持一个 index 文件，按编号/标题/状态列出每条 ADR；决策变更不修改原记录，而是写新记录并 supersede 链接」）。
- **借鉴点**：纯时间分区的最大弱点是主题检索（「TD-021 何时定的？」需扫全树）。解法即 A-MEM/ADR 的**条目间链接与标签**：每条 worklog 条目带 Task ID、关键词、`supersedes/see-also` 指针；`l` 文件同时承担 ADR index 的角色（编号/概述/状态一览）。

### ⑧ PARA + Progressive Summarization：按可操作性分层 + 精度轴压缩
- **来源**：fortelabs.com《The PARA Method》（Projects/Areas/Resources/Archives，按可操作性递减组织，归档冻结）、《Progressive Summarization: A Practical Technique》（笔记分层加粗：第 1 层原文、第 2 层要点、第 3 层摘要，逐层压缩、上层永远小于下层）；zettelkasten.de 对 PS 的批评（PS 只是起点，最终要生长为结构化网络）。
- **借鉴点**：用户需求中的「描述精度轴」= progressive summarization 的分层压缩；PARA 的「active 项目在浅层、沉寂内容归档冻结」对应 kerf 的 stage/里程碑封存（r10/r11 目录只读）。也吸收批评意见：摘要层不是终点，要靠链接（⑦）长成网络。

### ⑨ LSM-tree：append-only 写入 + 分层 compaction + 旧层不可变
- **来源**：O'Neil et al. 1996《The Log-Structured Merge-Tree》(cs.umb.edu)；aerospike.com / scylladb.com / alibabacloud.com LSM 综述；bitsxpages.com 读写/空间放大分析。
- **机制**：写永远先入 memtable（内存追加），刷成 L0 SSTable，再逐层 compaction 合并压实；**已写层不可变**，合并产生新文件而非原地改。
- **借鉴点**：worklog 的「只追加禁止覆盖」协议 = memtable/L0 追加写；日/周/阶段摘要的定期生成 = **compaction 压实**；已封存里程碑目录 = immutable SSTable（只读、可审计、永不重写）。术语可直接沿用：**写入路径**（追加详录）与**压实路径**（生成上层摘要）分离，压实可异步/延迟（“懒压实”），不必实时。

### ⑩ 时间分区表 + partition pruning：路径名即分区键
- **来源**：docs.pingcap.com（TiDB partitioned tables 最佳实践）；questdb.com《Partition Pruning》；last9.io《Database Partitioning》；Google BigQuery 分区裁剪文档。
- **机制**：按日期 range 分区，查询谓词命中分区键时优化器直接**裁掉无关分区**（不打开、不扫描）；分区边界对齐常用查询粒度（日/月）。
- **借鉴点**：目录按时间命名（`2026-01/`、`03-15/`、`stage-1/`）使 agent **只凭路径字符串即可裁剪 99% 的分区**——一次 `ls` 顶层的成本换掉全库扫描。这是「时间维度轴」的数据库学依据：分区粒度必须对齐查询粒度（会话/日/周/阶段）。

### ⑪ B-tree/B+ 树稀疏索引：`l` = 目录级稀疏索引，多级 l = 树的内部节点
- **来源**：en.wikipedia.org/wiki/B-tree（多级树索引、块级辅助索引）；knowledgegate.ai（**sparse index：每个数据块只记一条首键→指针**，索引远小于数据，代价是定位后需块内顺序扫描）；planetscale.com B-trees and database indexes；OSDev.org（ext4/NTFS 用 B+ 树做**目录索引**——文件系统自身就靠 B-tree 组织目录查找）。
- **借鉴点**：`l` 文件的正确形态即**稀疏索引**：每个子项一行「编号 | 概述关键词 | 指针 | 时间范围 | 状态」，行数=扇出而非=内容量，故体积极小（数 KB），一次读入；多级 l 串联 = B+ 树从根到叶的下降（root l → 层 l → 叶 l → 打开目标文件）。**「每层一个 l」的递归同构恰好把 B-tree 显式文件化**——这是用户方案最漂亮的一点。

### ⑫ salsa / rustc 查询系统：依赖追踪 + 红/绿失效传播（摘要层的对账机制）
- **来源**：github.com/salsa-rs/salsa（on-demand 增量计算，rust-analyzer 用于 IDE 实时响应）；rustc-dev-guide.rust-lang.org（query system + 红绿算法：输入未变→复用 memo（绿），依赖变更→沿依赖图传播失效（红）→重算）。
- **借鉴点**：上层摘要 = 下层详录的 memo 化视图。kerf 已有「文档债/对账轮」实践（30-b：代码变了文档没变=stale 缓存）。摘要条目应记录**溯源集合**（「本日摘要压缩自 Task #31a~#31f」）与生成时间戳，下层追加后 l 中标记该摘要 stale（红），压实任务重算（变绿）。这让「摘要与详录不一致」成为可检测状态而非沉默漂移。

---

## 3. 与本项目需求的映射分析

用户方案三要素与检索到的优秀设计对应关系：

| 需求要素 | 对应设计（编号见上） | 对应强度 |
|----------|---------------------|----------|
| 时间维度轴（会话/日/周/阶段/里程碑） | ⑩ 时间分区+裁剪（分区键=路径名）；⑨ LSM 分层（层=时间粒度递增）；③ episodic 时间线；date-based daily notes（YYYY/MM/DD，Obsidian 社区标准实践） | 强：时间分区是数据库界最成熟的模式，目录名即谓词 |
| 描述精度轴（摘要/详录） | ⑧ progressive summarization（逐层压缩）；③ 递归摘要金字塔（O(log N)）；⑨ compaction（L0 详录→上层压实）；⑪ sparse index（每块一 entry） | 强：与 LSM/递归摘要在数学上同构——每层体积按固定比压缩 |
| 路由索引 `l` 文件 | ⑪ B+ 树稀疏索引（多级内部节点）；⑥ JD 稳定编号目录；⑦ ADR index.md；① MemGPT 页表；19 号检索 CLAUDE.md 分层记忆（机器可读入口） | 强：`l` 显式实现了「B-tree 的目录文件化」 |
| 「编号_概述」命名 | ⑥ Johnny.Decimal（ID+描述）；⑦ ADR 编号+标题+状态 | 强：JD 与 ADR 双重先例；编号给排序，概述给语义（grep 命中面） |
| `rec(dir|file|l)` 递归同构 | ⑪ B-tree 节点同构；⑨ LSM 每层同构；Zettelkasten 的 scale-free 自相似 | 中强：同构使 agent 学一次协议全树通用，是面向 LLM 的优点 |
| append-only（与现有 worklog 协议兼容） | ⑨ LSM 追加写；⑦ ADR supersede（不改旧记录，新记录接替） | 强 |

**用户方案相对已有系统的独特/领先之处**：
1. **双轴显式正交参数化**（时间粒度 × 描述精度）。现有系统多为单轴隐式：MemGPT/Letta 只分「上下文内/外」，Zettelkasten 只按主题，PARA 只按可操作性，daily-notes 只按时间。把两轴作为一等设计变量、允许「日粒度×详录」「阶段粒度×摘要」等组合格点，是检索到的文献里没有显式出现的提法。
2. **`l` 路由文件作为每层一等公民**。ADR index、JD 目录、CLAUDE.md 都是「树根才有索引」，用户方案是**每层都有**——等价于把 B+ 树内部节点物化为文件，使任意子树可独立装载/独立校验。
3. **面向 LLM 检索原语的命名设计**：路径中直接携带「编号+概述关键词」，则 `ls` 输出=免费稀疏索引、`grep -r 目录名` 也能命中语义、路径本身即可作为 prompt 引用。Letta/Anthropic 的证据（②）表明这比向量检索更贴合 agent 的原生工具面。
4. **与 git/现有 worklog 协议无缝**：append-only + 可 compaction 的设计使「唯一共享事实源」性质从单文件升级为树，且历史不可篡改（⑨⑦）。

**检索暴露的三个缺口（主设计必须补）**：
- **主题/意图入口缺失**：纯时间分区回答不了「TD-021 是什么时候定的」（需全树扫描）。解法在 ⑦（A-MEM 链接 + l 提供 by-topic 视图）与 ④（semantic/procedural 条目单独可寻址）。
- **摘要漂移无检测**：上层概述与下层内容失配即「文档债」。解法在 ⑫（溯源集合 + stale 标记）。
- **扇出/体积失控**：单目录 500 个文件会毁掉稀疏索引。解法在 ⑥（扇出上限触发再分层）。

---

## 4. 设计建议要点（供 31-e 主设计吸收）

1. **三级读写路径**（借鉴 ⑨⑩）：写入路径=会话详录 append-only（等价 memtable/L0）；压实路径=日/周/阶段结束时把下层条目合并为上层摘要（compaction，可延迟执行）；已封存里程碑目录冻结只读（immutable），修正一律走 supersede 新条目（⑦ADR 风格），不改历史。
2. **`l` = 稀疏索引，限额设计**（⑪）：每行固定字段「编号 | 概述(含 2-4 检索关键词) | 类型(dir/file) | 覆盖时间范围 | 状态(active/superseded/stale)」；行数 = 该层扇出，硬上限 10-12 行（⑥）；文件体积极限（如 ≤8KB）保证一次读入即全量索引。
3. **编号规范**（⑥）：定宽零填充 + 单调递增，保证字典序=时间序；`ls` 天然有序，`sort` 免费可用，agent 可对时间做二分（16 号检索 git bisect 思想：log N 步定位）。
4. **检索三原语协议**（②）：a) 目录枚举=时间粗定位（分区裁剪）；b) 读 `l`=语义精定位（稀疏索引下降）；c) `grep -r`=关键词兜底（全文命中，含路径名命中概述）。不引入向量库/embedding，与 Claude Code 生产实践一致。
5. **摘要条目必须带溯源指针**（③⑫）：「本摘要压缩自 <下层条目编号列表>，生成于 <戳>」；下层追加/变更后置 stale，压实轮重算并清除——把「对账」从纪律变成可检测状态。
6. **补主题第二入口**（⑦④）：每条目规范携带 Task ID 与关键词标签；顶层 `l`（或独立索引文件）提供 by-topic / by-decision 视图；条目间 `supersedes` / `see-also` 链接显式化为字段。
7. **记忆类型三分的字段化**（④）：条目头标注 episodic（做了什么）/ semantic（结论/裁定）/ procedural（规则/SOP），l 中可见，让「找结论」不必翻时间线。
8. **时间粒度对齐查询粒度**（⑩）：目录层级严格取 会话→日→周→阶段→里程碑 五档，不引入非查询粒度的中间层；每档在其 l 中标注本档覆盖范围。
9. **精度轴的压缩比约定**（⑧③）：上层摘要体积建议压缩到下层的 1/5~1/10（概述一行 vs 详录一节），保证 O(log N) 层级导航的 token 经济性；详录永不删除（audit 底座）。
10. **会话边界即压缩点**（③Anthropic compaction / handoff）：每次会话结束写详录条目（=现有 worklog section 协议平移）；新会话冷启动=只读根 `l`（+必要时逐层下降），即 MemGPT 的「换入最小工作集」。

---

*检索与撰写：31-e-research 子代理；原始命中 JSON 存于会话沙箱 /tmp/ws/（22 个查询文件 + 1 篇深读）。本文档为 31-e 主设计的输入，非最终设计。*
