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
