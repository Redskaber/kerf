# Stage 0 开发日志

> **Author**: kerf-dev-agent
> **Date**: 2026-09-09
> **Version**: v0.1.0
> **Status**: Active

## 轮次 R1：环境 + 脚手架（Task 1/3）

- rustup 安装 stable 1.98.1 + rustfmt + clippy（§3.1 流程：检查→官方脚本→验证→归档）
- Cargo Workspace 落成：9 成员 crate + 根 crate（§8.4.6 两级结构语义）
- 零外部依赖策略（自举信任根显式化，原则 11）

## 轮次 R2：基础三 crate（Task 4-a）

- kerf-span：Span（字节偏移主键+展开代次）/ SourceMap（行/列渲染派生）/ Diagnostic
- kerf-syntax：SymbolTable（NFC 一次归一化）/ ScopeSet / Stx 语法对象
- kerf-core：CoreExpr 9 原语 / 图 IR（Arena + 字面量共享）/ CodeValue（良构性绑定感知修正）
- 修复：`render(&dyn Fn)` 递归类型无限展开；doctest 误编译设计文档代码块

## 轮次 R3：Reader + Expander（Task 4-b）

- 词法器：字符串跨行行号/贪心数字拒绝/嵌套块注释/省略号标识符
- 展开器：9 核心形式 + §3.2 全部语法糖推导 + syntax-rules（模式/省略号/模板）
- **关键修复**（同类路径审查 §20.3）：
  1. lambda 体铺平 bug（4 处构造点——body 形式必须直接铺入 lambda 列表）
  2. letrec/内部 define 提升的 nil 实参额外包裹 bug（(nil) 被求值为应用）
  3. while 递归调用为裸符号而非调用形式
  4. 卫生重命名一致性（同一模板内同标识符必须映射同一符号——renames 映射）
  5. 宏自引用保留集（递归宏正确性）
  6. 核心关键字不参与重命名（展开产物可分派）
- 展开深度上限校准 128（rustc 默认递归上限对齐，§19.2 不变式的栈安全实现）

## 轮次 R4：Compiler + Runtime + VM + Driver（Task 4-c）

- 编译器：跳转回填（占位断言）/ 常量池去重 / 闭包捕获描述符 / debug_info 全覆盖
- **关键修复**：
  1. CLOSURE 指令发射原型切换（当前原型栈——体编译后切回外围）
  2. **捕获协议重构**：值快照 → 共享单元格（CaptureSource 描述符 + 帧局部槽
     Rc<RefCell> 化）——letrec 递归与可变捕获语义正确性（正确 > 妥协）
  3. SetBang 栈平衡（DUP 后存储）
- VM：迭代式主循环（kerf 递归以帧栈承载，Rust 栈恒定）+ 三扩展槽 + 堆栈追踪
- GC：分配计数驱动触发 + 低收益冷却退避（深递归下根集扫描 O(n²) 消除：
  122s → 3.4s）+ 显式工作栈（2×10^5 深链不爆栈）
- Driver：全管线 + 相位生命周期 + 卫生回退解析（$hyg$ 后缀剥离）

## 轮次 R5：CLI + 集成测试 + 验收（Task 4-d/7）

- CLI 十个子命令（run/eval/check/tokens/stx/core/ir/bc/code/bench）
- tests/v0/stage0/{plan,gate} 树 + tests/common 辅助（§9.1 结构）
- **§3.2 验收全绿**：clean → build --release → check（0 警告）→
  test --release（200/0）→ fmt --check → clippy -D warnings（0）
- 基线：fib(25) 84.7ms/轮（release）

## 遵循原则记录（§2.2/§2.3 执行要求）

- 原则 9（正确 > 妥协）：捕获协议重构放弃值快照捷径
- 原则 4（可替换）/12（接口预留）：四项 P2/P3 冻结
- 原则 16（人类可感知）：诊断渲染含摘录 + 反汇编含调试信息
- 原则 6（横切早期内置）：Span/诊断/相位从第一行代码即贯穿
- §2.3 原则 10（唯一可信源）：SymbolTable/ScopeStack/GC Heap 单点定义

## 轮次 R6：深度审查修复 + 负测扩张（Task 16，r3——deep-review R1 → NEEDS REVISION 闭环）

- 代码修复（P1×4 清零，§4 行动计划 A）：
  1. **App 求值顺序统一「函数先」**（A1/偏差 #13）：compile.rs App 分支 fn 先入栈 +
     vm.rs CALL 弹序对调——06 §1.3/§2 A1 契约双侧对齐（T1 反例面清零）；
     回归：app_evaluates_fn_then_args（编译序断言）+ app_evaluation_order_fn_first_dual_path
     （双路径错误排序负例）
  2. **opcode.rs 冻结守护测试重写**（A2/偏差 #7）：40 项显式枚举（八组），模块头分组
     注释补 DEFINE_GLOBAL——enum ↔ 测试 ↔ 文档三方冻结
  3. **§7.3.1 门审计集**（A3）：examples/audit/stage0_gate_audit_r1.rs——41 case
     （负向 32 + 恢复 6 + 正向 3），§7.1.1 七类全覆盖（含空应用与模块循环依赖）；
     审计驱动三修复落地（见下）
  4. **根 CLI reader 直调改走 driver**（A4）：driver 公共 API dump_tokens/dump_stx，
     main.rs tokens/stx 子命令经 driver 转发（§14.7.2 B4 合规）
- 审计集驱动修复（发现项 C03/C07/C08）：
  - 模块循环依赖检测（phase.rs visit DFS 灰标记 → Err「模块循环依赖：Symbol(N) → …」；
    菱形依赖合法）
  - VM 堆栈追踪（run_program 错误路径最内 16 帧 note——VmError.trace 有生产者）
  - eval 路径卫生回退接线（driver::resolve_eval_hygiene_fallbacks——与 VM 路径
    resolve_hygiene_fallbacks 语义镜像，T1）
- 负测扩张（行动计划 B，§9.4.3 1:3 门限达标）：四文件表格驱动负测
  negative_reader（59 case）/ negative_expander（96）/ negative_vm（230）/
  negative_semantics（98）= 483 case；正负比 1:0.24 → ≈1:3.2；
  E1–E6 + E0001/E0002/E0004 直接断言；4 项 #[ignore] 文档化存档（守卫/元数/eval 栈/
  i64::MIN mod）
- 工程整理（行动计划 D）：examples/ 重组（usage/ 6 个 .krf + audit/ + README 索引，
  §9.6.2）；catch-all 臂级注释（vm.rs GC 静默空臂等）
- 文档对账（行动计划 C，Task 16-d）：lang-design v5.2（deep-review §6 偏差清单 26 项
  回写）+ matrix/pipeline-test-coverage/negative-tests/TD 登记（TD-012/013/014 新增，
  TD-001/006 断档存档）
- 验收：**290 通过 / 0 失败 / 4 忽略**（294 函数）；审计集 41 case 全 PASS；
  fib(25) 84.4ms/轮复现；GC 单轮 0.17s（更正 0.72s 陈旧口径）
