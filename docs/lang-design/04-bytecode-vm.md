# 字节码虚拟机与编译器实现

> **Author**: kerf-doc-agent
> **Date**: 2026-09-10（v5.2：操作码全 40 项显式枚举（八组）+ App 求值顺序修复后确认 + 基准锚点对齐）
> **Version**: v5.2
> **Status**: Active
> **处理程度**：P0（必须实现——Stage 0 已落地，kerf-compiler + kerf-vm）｜ **所属 Stage**：Stage 0 ｜ **推迟项**：TCO 尾调用优化（Stage 1+，仅改 Compiler 不改 VM）、JIT/本地码后端（Stage 2+ 可选，[08-后端演化](./08-backend-evolution.md)）、ext1 槽的 Effect System 实装（接口预留档，[13-能力矩阵 §3.1.1](./13-capability-matrix.md)）

> 本文件收录执行后端的设计与实现：字节码 VM（原 §8.12，设计估算约 35 操作码、VM 状态与三扩展槽调用帧）、Compiler 实现框架（原 §19.3：代码生成骨架含跳转回填 + 核心不变式 + 实现陷阱）、VM 执行循环（原 §19.5：执行循环骨架 + 核心不变式 + 实现陷阱）。九个核心原语见 [01-核心原语](./01-core-forms.md)；上游的宏展开见 [03-宏系统](./03-macro-system.md)；运行时基座的另一半（GC 与最小 I/O）见 [05-运行时](./05-runtime.md)；后端策略与性能演化（不引入 LLVM 的论证）见 [08-后端演化](./08-backend-evolution.md)；12 个能力模型的完整矩阵见 [13-能力矩阵](./13-capability-matrix.md)（其 §2.12 为本文件 §1 的规范副本）；编译正确性定理（本文编译器是其编译侧证明对象）见 [06-操作语义 §5](./06-operational-semantics.md)；双路径互查的测试定义见 [11-测试基础设施 §2.4](./11-testing.md)。

---

## 1. 字节码 VM（原 §8.12）

switch-dispatch 循环，设计估算约 35 个操作码（原 §8.12）；**Stage 0 冻结实现为 46 个操作码**（kerf-compiler/src/opcode.rs，enum 显式枚举 + 守护测试 `opcode_count_matches_spec` 逐项列举断言 46——「enum ↔ 测试 ↔ 文档」三方冻结。**r21/42-b 修正**：TAIL_CALL（r18/40-c TD-022 TCO 引入）此前漏列于测试枚举与本文表格——三方冻结漂移按 sop R4（代码为准 + 本次修正文档）补齐，40→41；**r25/42-f 扩**：效应两指令（41→43，第九组——effect-language-design D6 原语集 9→11 的字节码面）；**r30/48-d 扩**：FFI 三指令（43→46，第十组——ffi-ownership-model §2/§8 的字节码面：CALL_EXTERNAL/ALLOC_EXTERNAL/FREE_EXTERNAL；语言面形式 Stage 3，编译臂不发射））。**全 46 项显式枚举（十组，与 opcode.rs 模块头分组逐项一致）**：

| 组 | 操作码（个数） |
|----|----------------|
| 栈操作（7） | `PUSH_CONST k` / `PUSH_NIL` / `PUSH_TRUE` / `PUSH_FALSE` / `POP` / `DUP` / `SWAP` |
| 变量访问（7） | `LOAD_LOCAL i` / `STORE_LOCAL i` / `LOAD_GLOBAL k` / `STORE_GLOBAL k` / `DEFINE_GLOBAL k` / `LOAD_CAPTURED i` / `STORE_CAPTURED i` |
| 控制流（2） | `JUMP t` / `JUMP_IF_FALSE t` |
| 函数操作（4） | `CLOSURE proto, n_captures` / `CALL n` / `TAIL_CALL n`（TD-022/H2 TCO 尾调用帧复用，r18） / `RET` |
| 算术与比较（12） | `ADD` / `SUB` / `MUL` / `DIV` / `MOD` / `NUM_LT` / `NUM_GT` / `NUM_LE` / `NUM_GE` / `NUM_EQ` / `EQ` / `NOT` |
| 数据构造（3） | `MAKE_PAIR` / `CAR` / `CDR` |
| 谓词（5） | `IS_NULL` / `IS_PAIR` / `IS_INT` / `IS_BOOL` / `IS_PROCEDURE` |
| 终止（1） | `HALT` |
| 效应（2，r25/42-f） | `INSTALL_HANDLER handler, body, tag, trampoline`（handle 表达式原子帧编排：压 handler 帧（ext1 = 分派数据 + 数据栈水位）+ body thunk 帧（ret = (trampoline, 0)）→ 转移执行；捕获由 VM 按两原型描述符从当前帧取） / `PERFORM`（效应上抛：弹效应值 `(tag . payload)` → 扫描最近匹配 handler 帧 → continuation 快照（帧链含 handler 帧本身 + 数据栈整栈 + 恢复点）→ 帧变形为 handler 体执行帧 → 栈截回水位；未匹配 = E0007） |
| FFI（3，r30/48-d） | `CALL_EXTERNAL symbol, n_args`（外部函数调用——窗口规程步 2-4 边界包装层执行：装载边界（堆实参 pin Φ[v]+=1 + 令牌校验，Invalid = E0010）→ 宿主调用（VM 挂起）→ 返回包装 + 全部堆实参 unpin；符号经 extern 符号表按名解析，未登记 = E0012 fail-closed；实参求值由先前指令完成） / `ALLOC_EXTERNAL size`（外部域分配——不经过 GC，产出 CPointer 令牌；size=0 编译期拒绝 + 运行期纵深 E0011） / `FREE_EXTERNAL`（消费语义释放：槽级失效全局标记；双重释放 = E0010、Opaque = E0011、非令牌 = E0011）——**语言面形式 Stage 3，编译臂不发射**（操作码直接构造/`compile_ffi_call_program` lowering 面驱动；ffi-ownership-model §2/§8） |

**操作码漂移注记（v5.2 重写，设计 vs 实现对账——早期版本「全部扩落在既有分组内」的表述不实，更正如下）**：超出原 §8.12 七组清单的扩是**真实存在的再分组**：(1) 原设计将 `=`/`<`/`>` 复用算术组并漏列 `<=`/`>=`/`mod`——实现扩为**算术与比较** 12 项显式指令（链式比较按序折叠）；(2) 原设计无**谓词组**——实现的 `IS_NULL/IS_PAIR/IS_INT/IS_BOOL/IS_PROCEDURE` 5 项是 **Stage 0 扩展分组**（`null?` 等内置的底层执行机制，[09-标准库 §2](./09-stdlib.md)）；(3) 零操作数快推 `PUSH_NIL/PUSH_TRUE/PUSH_FALSE`（避免常量池哈希查找的快路径）；(4) 闭包捕获**读写双指令** `LOAD_CAPTURED/STORE_CAPTURED`（共享单元格捕获协议要求可变捕获，本文 §1.1）；(5) **`DEFINE_GLOBAL`**（v5.1 依据 [06-操作语义 §2 R6/E6 与 §5 T1 定理](./06-operational-semantics.md) 新增的第 40 号冻结契约：define 与 set! 的全局存储语义分裂修复——`StoreGlobal` 收紧为 S1/E3 语义「只写已存在绑定，未绑定报错」；`DefineGlobal` 承载 D1/E6 语义「只新增绑定，同层重复报错」；Define 编译模式改为 `value; DUP; DefineGlobal` 使返回值 = v 与 eval 路径对齐）。另注：设计草稿中的 `Ne`（不等比较）**未实现**——不等由 `NOT` 组合 `=`/`NUM_EQ` 表达，冻结清单不含此项。分组数统一为**八组**（[06-操作语义 §5.2 L1](./06-operational-semantics.md) 的指令组归纳同步对齐）。后续演进的正确路径：**新增操作码 = 冻结新契约 + 回填本文档 + 同步守护测试枚举**，禁止未注记的静默漂移。

**VM 状态包含**：代码、数据栈、调用栈、全局环境、常量池和调试信息表。

**调用栈帧包含三个扩展槽**（Stage 0 可以为空，但格式必须保留）：
- `ext1`：效应 handler 帧（**r25/42-f 具体化**：`Option<Rc<HandlerFrame>>`——分派键 tag / handler 原型 / 捕获单元 / 数据栈水位；原则 27 预留时机兑现——槽位格式不变，承载升级。升级路径见 [13-能力矩阵 §3.1.1 EffectSystem 预留 trait](./13-capability-matrix.md)）
- `ext2`：为异常处理表预留（→ 错误运行时语义决策，[13-能力矩阵 §4](./13-capability-matrix.md) 决策 2）
- `ext3`：为调试帧信息预留（→ [15-架构分层 §4.9 编译器自调试工具](./15-architecture-layers.md)）

**Forth 语言的线程码技术展示了 VM 极简设计的极限**：整个 VM 的核心仅由 NEXT、DOCOL、EXIT、LIT 四个原语构成。

**实现指引**：跳转回填的字节码生成与 switch-dispatch 执行循环的完整伪代码见本文 §2 与 §3。原 v3.0 §8.6「字节码 VM 设计」与本节内容重复，v4.0 已合并至此。

### 1.1 CodeBuf 与闭包捕获契约（Stage 0 冻结，kerf-compiler）

```rust
/// 代码缓冲：编译期字节码累积器（回填占位队列的宿主）。
/// 构造时断言占位队列为空（回填完备不变式的机械检查点）。
pub struct CodeBuf {
    // code: Vec<Op>;                  // 指令序列
    // constants: Vec<ConstantValue>;  // 常量池（float 位键去重）
    // jump_patches: Vec<PendingPatch> // 待回填跳转队列
}

/// 闭包捕获描述符：标记捕获源是「定义环境局部」还是「共享单元格」。
/// 捕获源决定 LOAD 时读值还是读单元格——共享单元格（Rc<RefCell<Value>>）
/// 是 letrec 递归与可变捕获语义正确性的关键（R5 SetBang 对闭包可见，
/// [06-操作语义 §5.2 L3 捕获协议等价性](./06-operational-semantics.md)）。
pub enum CaptureSource { /* Local(usize) | SharedCell(usize) */ }
```

**CLOSURE 指令协议**（Stage 0 关键修复的理论定案）：`CLOSURE { proto, n_captures }` 从**当前栈**弹出 n_captures 个捕获值构造闭包——原型切换期从「参数区捕获」改为「current 栈」是回填完备断言暴露的编译器 bug（worklog Task 4-c）。**共享单元格捕获**替代「捕获值快照」是语义正确性修复（快照破坏 R5/S1 对闭包的可见性——正确 > 妥协，[17-设计原则 §1 原则 9](./17-principles.md) 工程细则）。

## 2. Compiler 实现框架（原 §19.3）

> **实现框架系列说明（原 §19 章导言）**：本系列（[02-语法模型 §6](./02-syntax-model.md) / [03-宏系统 §4](./03-macro-system.md) / 本文 §2-§3 / [05-运行时 §4](./05-runtime.md)）将 [13-能力矩阵 §2](./13-capability-matrix.md) 定义的 12 个能力模型落实为可直接照抄实现的伪代码框架。所有伪代码采用 Rust 风格语法，但刻意停留在"控制流 + 数据流"层面，不绑定具体内存布局——同一框架可无损翻译为 OCaml（推荐宿主）或 C（VM 基座）。每节按「算法骨架 → 核心不变式 → 实现陷阱」组织：不变式是测试设计的直接依据，陷阱清单来自历史实现（Guix 自举链、rustc、Racket BC）踩过的坑。

CoreExpr → Bytecode 的直译式编译，含跳转回填、常量池、全局符号表和 debug_info_table 生成（能力模型见本文 §1）。

**代码生成骨架（含跳转回填）**：

```rust
fn compile_expr(e: &CoreExpr, out: &mut CodeBuf) -> Result<(), CompileError> {
    match e {
        Literal(v) => out.emit(Op::PUSH_CONST, &[out.intern_const(v)]),
        VarRef(name) => match out.env().slot(name) {
            Some(slot) => out.emit(Op::LOAD_LOCAL, &[slot]),
            None => out.emit(Op::LOAD_GLOBAL, &[out.intern_global(name)]),
        },
        Lambda { params, body } => {
            let proto = out.begin_closure(params);   // 开启闭包子程序
            compile_expr(body, out)?;
            out.emit(Op::RET, &[]);
            out.end_closure(proto);
            out.emit(Op::CLOSURE, &[proto.id()]);    // 在当前位置构造闭包值
        }
        If { cond, then_b, else_b } => {
            compile_expr(cond, out)?;
            let j_false = out.emit_jump(Op::JUMP_IF_FALSE);   // 目标未知，占位
            compile_expr(then_b, out)?;
            let j_end = out.emit_jump(Op::JUMP);              // 目标未知，占位
            out.patch_jump(j_false, out.here());              // 回填 else 入口
            compile_expr(else_b, out)?;
            out.patch_jump(j_end, out.here());                // 回填汇合点
        }
        App { fn_expr, args } => {
            compile_expr(fn_expr, out)?;          // 求值顺序：**函数先**（06 §1.3/§2 A1 契约）
            for a in args { compile_expr(a, out)?; }  // 实参从左到右入栈
            out.emit(Op::CALL, &[args.len()]);   // CALL：弹 n 参后弹被调者
        }
        // Begin / SetBang / Define / Module 同构处理
    }
    // 每条 emit 自动写入 debug_info_table: (pc, current_span)
    Ok(())
}
```

**核心不变式**：
1. **栈平衡**：任何 CoreExpr 的编译产物执行前后，操作数栈净变化 = 表达式返回值个数（恰好 1）；违反即编译器 bug
2. **回填完备**：`emit_jump` 返回的占位索引在函数返回前必须全部被 `patch_jump` 消费，CodeBuf 析构时断言占位队列为空
3. **调试信息全覆盖**：每条指令都有 (pc, Span) 映射，无裸指令——VM 错误才能反查源码（[15-架构分层 §4.7 运行时错误基础设施](./15-architecture-layers.md)）

**实现陷阱**：
- **尾调用不是 Stage 0 的优化目标**，但 If 编译模式必须为尾位置标注留位（Stage 1+ 的 TCO 只改 Compiler，不改 VM）
- **常量池去重**：`intern_const` 必须按值哈希去重，否则 `(begin 1 1 1)` 会塞 3 个相同的 Int 常量，无谓膨胀字节码
- **模块级全局的链接时机**：Stage 0 单模块编译可立即解析；多模块链接推迟到 VM 加载期（LOAD_GLOBAL 按名惰性解析 + 缓存）

## 3. VM 执行循环（原 §19.5）

switch-dispatch 循环，处理全部 **46 个操作码**（冻结实现，本文 §1 全枚举；操作码按**十组**分类：栈操作 7 / 变量访问 7 / 控制流 2 / 函数操作 4（含 TAIL_CALL——r21 修正补齐） / 算术与比较 12 / 数据构造 3 / 谓词 5 / 终止 1 / 效应 2（r25/42-f——INSTALL_HANDLER/PERFORM） / **FFI 3（r30/48-d——CALL_EXTERNAL/ALLOC_EXTERNAL/FREE_EXTERNAL；语言面形式 Stage 3，编译臂不发射）**——设计估算约 35 的偏差见 §1 漂移注记）。运行时错误捕获含堆栈追踪生成（run_program 错误路径保留**最内 16 帧**调用点，渲染为 note 行——深递归下外层无信息量，防诊断爆炸；`VmError.trace` 的唯一生产者）。**迭代式主循环**：递归经调用帧栈承载，Rust 栈深度恒定（深递归程序不爆宿主栈——与 [05-运行时 §4](./05-runtime.md) 显式工作栈同构的防御；帧数上限 `MAX_FRAMES = 100_000`，超限报结构化「调用帧超过上限」错误）。

**执行循环骨架**：

```rust
fn run(vm: &mut Vm) -> Result<Value, VmError> {
    loop {
        let (op, operands) = vm.fetch_decode();       // 读取 pc 处指令，pc 自增
        match op {
            Op::PUSH_CONST(k)  => vm.push(vm.constant(k).clone()),
            Op::DUP            => vm.dup(),
            Op::LOAD_LOCAL(i)  => vm.push(vm.frame_local(i)?),
            Op::STORE_LOCAL(i) => vm.frame_set_local(i, vm.pop())?,
            Op::LOAD_GLOBAL(n) => vm.push(vm.lookup_global(n)?),

            Op::JUMP(t)        => vm.pc = t,
            Op::JUMP_IF_FALSE(t) => if !vm.truthy(vm.pop()) { vm.pc = t },

            Op::CLOSURE(p)     => vm.push(Value::closure(p, vm.capture_env())),

            Op::CALL(n) => {
                let callee = vm.peek(n)?;             // 被调者在栈顶第 n 项（函数先入栈）
                vm.enter_frame(callee, n,
                    FrameExt::new());                 // 三扩展槽：continuation/异常表/调试帧
            }
            Op::RET => {
                let (v, ret_pc, saved_frame) = vm.leave_frame()?;
                vm.push(v);
                vm.pc = ret_pc;
            }

            Op::ADD => { let (b, a) = vm.pop2()?; vm.push(a.add(b)?); }

            Op::HALT => return Ok(vm.pop()),

            _ => return Err(VmError::bad_opcode(op, vm.pc)),
        }
        // 每轮循环末尾：分配配额检查（GC 安全点，见 [05-运行时 §4](./05-runtime.md)）+
        // 运行时错误在 Err 路径统一经 debug_info_table[pc] 反查 Span 生成堆栈追踪
    }
}
```

**核心不变式**：
1. **帧格式冻结**：调用帧永远携带三个扩展槽（ext1 continuation / ext2 异常表 / ext3 调试帧，本文 §1），Stage 0 允许为空但格式锁定——这是 [13-能力矩阵 §4](./13-capability-matrix.md) 三个待定决策的前置约束
2. **pc 单调性例外**：pc 只在 JUMP / RET / CALL / 异常路径改变；普通指令严格 pc += 1——违反会导致 debug_info_table 反查错位
3. **求值顺序契约（v5.2 修复后确认）**：`App` 的被调函数**先**求值入栈，随后实参从左到右入栈（本文 §2 编译侧与 [06-操作语义 §1.3/§2 A1](./06-operational-semantics.md) 双侧同一契约）；VM 侧 `CALL n` 按「先弹 n 参、后弹被调者」的约定取用，两侧契约不得单独修改——曾存在的「eval 函数先 / 编译参数先」双路径分裂（deep-review R1 偏差 #13，T1 反例面）已于 Task 16 修复并加双路径负例回归（`app_evaluates_fn_then_args` / `app_evaluation_order_fn_first_dual_path`）

**实现陷阱**：
- **Forth 线程码是极限参照而非 Stage 0 目标**：NEXT/DOCOL/EXIT/LIT 四原语的间接线程码更快但调试困难；switch-dispatch 的可读性对 Stage 0 更重要（见本文 §1 的 Forth 注记）
- **`peek(n)` 的栈深校验不可省略**：恶意或损坏的字节码会用 CALL 深度掏空数据栈，所有栈访问必须显式边界检查
- **truthy 语义要显式定义**：Stage 0 建议只有 `Bool` 参与 `JUMP_IF_FALSE` 条件（其余类型报 TypeMismatch），避免 JavaScript 式隐式转换的语义泥潭

## 4. 测试锚点（设计驱动测试，测试验证设计）

| 契约/不变式 | 测试锚点（tests/v0/stage0/） | 验证命题 |
|-----------|------------------------------|---------|
| 栈平衡（§2 不变式 1） | 编译器单测：每个原语编译产物的栈效应断言（SetBang DUP 后存储等边界 case） | 净变化 = 1 |
| 冻结计数守护（§1 全枚举） | opcode 单测 `opcode_count_matches_spec`：逐项显式列举 40 项 + 断言总数 | enum ↔ 测试 ↔ 文档三方一致 |
| 回填完备（§2 不变式 2） | CodeBuf 构造时占位队列断言（零占位交付） | 无悬空跳转 |
| 双路径互查（[06 §5.3](./06-operational-semantics.md) T1 定理） | 全部集成测试（eval 与 VM 结果逐字节一致，含 164 集成函数） | L1–L4 实例化 |
| App 求值顺序（§3 不变式 3） | `app_evaluates_fn_then_args`（编译字节码序断言）+ `app_evaluation_order_fn_first_dual_path`（vm_tests 双路径 `((undefined-a) undefined-b)` 错误排序负例） | 函数先、参数左到右 |
| fib(25) 性能基线 | CLI `kerf bench examples/usage/fib.krf`（含编译 84.4 ms/轮，release ×5——实测 84.443 ms 复现声称 84.7；[12-路线图 §1.1](./12-roadmap.md) 基准框架） | 性能回归对账 |
| 帧三扩展槽格式（§3 不变式 1） | VM 帧构造单测（ext1/2/3 格式冻结） | [13 §4](./13-capability-matrix.md) 前置约束 |
| 常量池去重（§2 陷阱 2） | 编译器单测（float 位键） | 无谓膨胀防御 |

> 测试矩阵完整定义见 [11-测试基础设施 §3](./11-testing.md)；本表是其执行后端侧子集。
