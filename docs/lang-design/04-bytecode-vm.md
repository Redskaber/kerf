# 字节码虚拟机与编译器实现

> **Author**: kerf-doc-agent
> **Date**: 2026-09-09
> **Version**: v5.0（源自 stage0.md v5.0 拆分）
> **Status**: Active

> 本文件收录执行后端的设计与实现：字节码 VM（原 §8.12，约 35 操作码、VM 状态与三扩展槽调用帧）、Compiler 实现框架（原 §19.3：代码生成骨架含跳转回填 + 核心不变式 + 实现陷阱）、VM 执行循环（原 §19.5：执行循环骨架 + 核心不变式 + 实现陷阱）。九个核心原语见 [01-核心原语](./01-core-forms.md)；上游的宏展开见 [03-宏系统](./03-macro-system.md)；运行时基座的另一半（GC 与最小 I/O）见 [05-运行时](./05-runtime.md)；后端策略与性能演化（不引入 LLVM 的论证）见 [08-后端演化](./08-backend-evolution.md)；12 个能力模型的完整矩阵见 [13-能力矩阵](./13-capability-matrix.md)。

---

## 1. 字节码 VM（原 §8.12）

switch-dispatch 循环，处理约 35 个操作码。完整操作码定义涵盖：
- **栈操作**：PUSH, POP, DUP, SWAP
- **函数操作**：CALL, RET, CLOSURE
- **控制流**：JUMP, JUMP_IF_FALSE
- **数据构造**：MAKE_PAIR, CAR, CDR
- **变量访问**：LOAD_LOCAL, STORE_LOCAL, LOAD_GLOBAL, STORE_GLOBAL
- **算术**：ADD, SUB, MUL, DIV
- **终止**：HALT

**VM 状态包含**：代码、数据栈、调用栈、全局环境、常量池和调试信息表。

**调用栈帧包含三个扩展槽**（Stage 0 可以为空，但格式必须保留）：
- `ext1`：为 continuation/effect handler 预留
- `ext2`：为异常处理表预留  
- `ext3`：为调试帧信息预留

**Forth 语言的线程码技术展示了 VM 极简设计的极限**：整个 VM 的核心仅由 NEXT、DOCOL、EXIT、LIT 四个原语构成。

**实现指引**：跳转回填的字节码生成与 switch-dispatch 执行循环的完整伪代码见本文 §2 与 §3。原 v3.0 §8.6「字节码 VM 设计」与本节内容重复，v4.0 已合并至此。

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
            for a in args { compile_expr(a, out)?; }  // 求值顺序：参数从左到右
            compile_expr(fn_expr, out)?;
            out.emit(Op::CALL, &[args.len()]);
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

switch-dispatch 循环，处理约 35 个操作码（完整清单见本文 §1；操作码按类分组：栈操作、函数、控制流、数据构造、变量访问、算术、终止）。运行时错误捕获含堆栈追踪生成。

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
                let callee = vm.peek(n)?;             // 栈顶第 n 项是被调者
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
3. **求值顺序契约**：CALL 之前参数已完成求值并按序压栈（本文 §2 编译侧保证），VM 侧只按约定取用，两侧契约不得单独修改

**实现陷阱**：
- **Forth 线程码是极限参照而非 Stage 0 目标**：NEXT/DOCOL/EXIT/LIT 四原语的间接线程码更快但调试困难；switch-dispatch 的可读性对 Stage 0 更重要（见本文 §1 的 Forth 注记）
- **`peek(n)` 的栈深校验不可省略**：恶意或损坏的字节码会用 CALL 深度掏空数据栈，所有栈访问必须显式边界检查
- **truthy 语义要显式定义**：Stage 0 建议只有 `Bool` 参与 `JUMP_IF_FALSE` 条件（其余类型报 TypeMismatch），避免 JavaScript 式隐式转换的语义泥潭
