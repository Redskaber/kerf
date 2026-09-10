# 宏系统：相位分离与卫生宏

> **Author**: kerf-doc-agent
> **Date**: 2026-09-09
> **Version**: v5.0（源自 stage0.md v5.0 拆分）
> **Status**: Active

> 本文件收录元编程层的设计与实现：相位分离系统（原 §8.9）、基础宏系统（原 §8.10）、模块相位分离系统的横切架构约束（原 §12.3），以及 Expander 实现框架（原 §19.2：展开循环骨架 + 核心不变式 + 实现陷阱）。语法对象模型与 Span 系统是卫生性的数据基础，见 [02-语法模型](./02-syntax-model.md)；九个核心原语见 [01-核心原语](./01-core-forms.md)；字节码编译（展开产物的下游）见 [04-字节码 VM](./04-bytecode-vm.md)；12 个能力模型的完整矩阵见 [13-能力矩阵](./13-capability-matrix.md)。

---

## 1. 相位分离系统（原 §8.9）

Phase 0（运行时）/ Phase 1（宏展开时）的严格分离，参考 Racket 设计。

**三个关键规则**：
1. Phase 1 代码只能产生 Phase 0 代码，不能直接执行 Phase 0 代码
2. 区分"实例化"（执行模块体）和"访问"（仅执行 Phase 1 部分）
3. 传递依赖的相位传播

模块生命周期通过 **declare / instantiate / visit** 三种操作管理：declare 登记模块与其相位声明，instantiate 执行模块体（Phase 0 实例化），visit 仅执行 Phase 1 部分（宏变换器加载）。该模型的完整架构约束论述见本文 §3。

## 2. 基础宏系统（原 §8.10）

卫生宏 + SyntaxObject，参考 Racket 但简化实现。Stage 0 的宏系统是"骨架"——支持宏定义、宏展开、卫生性保证，但不支持复杂的宏组合（如 `syntax-parse`）。

## 3. 模块相位分离系统：横切架构约束（原 §12.3）

完整的相位模型区分 Phase 0（运行时值）和 Phase 1（宏变换器），通过 declare / instantiate / visit 三种操作管理模块生命周期。其三个关键规则（Phase 1 只能产生 Phase 0 代码、实例化与访问分离、传递依赖相位传播）及能力模型定义见本文 §1；本节仅强调其横切属性：相位分离不是独立的管线阶段，而是 Expander（本文 §4）展开决策与 Compiler（[04-字节码 VM §2](./04-bytecode-vm.md)）链接决策的共享上下文——两个阶段通过同一相位表查询模块的相位归属，任何一侧对相位语义的理解偏差都会导致宏展开期/运行期代码的串扰错误。

## 4. Expander 实现框架（原 §19.2）

> **实现框架系列说明（原 §19 章导言）**：本系列（[02-语法模型 §6](./02-syntax-model.md) / 本文 §4 / [04-字节码 VM §2-§3](./04-bytecode-vm.md) / [05-运行时 §4](./05-runtime.md)）将 [13-能力矩阵 §2](./13-capability-matrix.md) 定义的 12 个能力模型落实为可直接照抄实现的伪代码框架。所有伪代码采用 Rust 风格语法，但刻意停留在"控制流 + 数据流"层面，不绑定具体内存布局——同一框架可无损翻译为 OCaml（推荐宿主）或 C（VM 基座）。每节按「算法骨架 → 核心不变式 → 实现陷阱」组织：不变式是测试设计的直接依据，陷阱清单来自历史实现（Guix 自举链、rustc、Racket BC）踩过的坑。

递归展开：处理 9 个核心形式 + 宏调用（Transformer）+ 作用域集查找（能力模型见本文 §2，相位规则见本文 §1）。

**展开循环骨架**：

```rust
fn expand(stx: &Stx, ctx: &mut ExpandCtx) -> Result<Stx, ExpandError> {
    match stx.form() {
        // 1. 核心形式：不展开自身，只递归展开子节点
        Form::Lambda { params, body } => {
            ctx.push_scope();                       // 作用域集入栈：新绑定加入
            ctx.bind_params(params);
            let body2 = expand(body, ctx)?;
            ctx.pop_scope();
            Ok(Stx::lambda(params, body2, stx.span()))
        }
        Form::If { c, t, e } => Ok(Stx::if_form(expand(c, ctx)?, expand(t, ctx)?, expand(e, ctx)?, stx.span())),

        // 2. 宏调用：相位 1 执行 transformer，再递归展开产物（直到不动点）
        Form::MacroUse { name, args } => {
            let transformer = ctx.lookup_transformer(name, stx.scopes())?;
            // 卫生性关键：展开产物自动携带「宏定义处作用域 + 使用处作用域」的并集
            let expanded = transformer.apply(args, ctx)?;
            expand(&expanded, ctx)                  // 宏可以展开出宏，直至核心形式
        }

        // 3. 标识符解析：作用域集决定绑定位
        Form::Id(sym) => {
            match ctx.resolve(sym, stx.scopes()) {
                Some(Binding::Local(slot))    => Ok(Stx::local_ref(slot, stx.span())),
                Some(Binding::Global(name))   => Ok(Stx::global_ref(name, stx.span())),
                None => Err(ExpandError::unbound(sym, stx.span())),
            }
        }
        // ... 其余核心形式同构处理
    }
}
```

**核心不变式**：
1. **展开终止性**：每次宏调用产生的语法对象携带"展开代次 + 1"的 expansion_id（[02-语法模型 §2 的 Span 定义](./02-syntax-model.md)）；超过上限（如 10_000）报错而非栈溢出
2. **卫生性保持**：宏引入的标识符作用域集 ≠ 用户代码作用域集，二者在 SyntaxObject 中永不合并为一个集合
3. **相位封闭性**：Phase 1 的 transformer 只能产生 Phase 0 语法对象，不能反向执行 Phase 0 代码（本文 §1 规则 1）

**实现陷阱**：
- **非局部展开的 Span 悬空**：宏展开产物中所有 SyntaxObject 必须携带有效 Span（宏定义处或调用处的合成 Span），"空 Span"会让后续诊断失效——这是 rustc 早期实际踩过的坑
- **展开缓存键必须含作用域集**：同一宏名在不同作用域下解析到不同 transformer，缓存键漏掉作用域集会产生错误复用
- **set! 与 define 的展开顺序**：`Define` 在展开期需区分"函数体内部"（转为 SetBang + 局部绑定）与"模块顶层"（保持 Define），处理不当会静默改变语义
