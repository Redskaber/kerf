# 宏系统：相位分离与卫生宏

> **Author**: kerf-doc-agent
> **Date**: 2026-09-09（v5.1 增补：宏系统契约从 Stage 0 冻结实现回填）
> **Version**: v5.1
> **Status**: Active
> **处理程度**：P1（卫生保证与展开核心 Stage 0 已实现；syntax-parse 等宏组合机制推迟）｜ **所属 Stage**：Stage 0（骨架）→ Stage 1+（组合） ｜ **推迟项**：syntax-parse 类结构化宏 DSL、宏展开调试工具、迭代式工作表展开（TD-007）

> 本文件收录元编程层的设计与实现：相位分离系统（原 §8.9）、基础宏系统（原 §8.10，v5.1 从冻结实现回填完整契约）、模块相位分离系统的横切架构约束（原 §12.3），以及 Expander 实现框架（原 §19.2：展开循环骨架 + 核心不变式 + 实现陷阱）。语法对象模型与 Span 系统是卫生性的数据基础，见 [02-语法模型](./02-syntax-model.md)；九个核心原语见 [01-核心原语](./01-core-forms.md)；字节码编译（展开产物的下游）见 [04-字节码 VM](./04-bytecode-vm.md)；12 个能力模型的完整矩阵见 [13-能力矩阵](./13-capability-matrix.md)（其 §2.9/§2.10 为本文件 §1/§2 的规范副本——本文件为专题深化版，两者内容应保持同步）。

---

## 1. 相位分离系统（原 §8.9）

Phase 0（运行时）/ Phase 1（宏展开时）的严格分离，参考 Racket 设计。

**三个关键规则**：
1. Phase 1 代码只能产生 Phase 0 代码，不能直接执行 Phase 0 代码
2. 区分"实例化"（执行模块体）和"访问"（仅执行 Phase 1 部分）
3. 传递依赖的相位传播

模块生命周期通过 **declare / instantiate / visit** 三种操作管理：declare 登记模块与其相位声明，instantiate 执行模块体（Phase 0 实例化），visit 仅执行 Phase 1 部分（宏变换器加载）。该模型的完整架构约束论述见本文 §3。

### 1.1 相位簿记契约（Stage 0 冻结，kerf-expander/src/phase.rs）

```rust
/// 相位层级（传递依赖的相位传播计算用）。
pub enum PhaseLevel {
    Runtime,   // Phase 0：运行时世界
    Expand,    // Phase 1：宏展开（编译时）世界
}

/// 模块登记条目（declare 的产物）。
pub struct ModuleEntry {
    pub name: Symbol,             // 模块名
    pub imports: Vec<Symbol>,     // 导入的模块名（传递依赖先行次序的输入）
    pub exports: Vec<Symbol>,     // 导出的符号
    pub visited: bool,            // visit 是否完成（Phase 1 变换器加载）
    pub instantiated: bool,       // instantiate 是否完成（Phase 0 执行）
}

/// 模块注册表：相位分离的运行簿记（driver 在管线各阶段调用）。
pub struct ModuleRegistry { /* entries: Vec<ModuleEntry> */ }

impl ModuleRegistry {
    /// 登记模块（名称 / 导入表 / 导出表）。重复登记同名模块报错。
    pub fn declare(&mut self, name: Symbol, imports: Vec<Symbol>, exports: Vec<Symbol>) -> Result<(), String>;
    /// visit：仅执行 Phase 1 部分（宏变换器加载）；
    /// 传递依赖的 visit 先行（递归展开，已访问则幂等跳过）。
    pub fn visit(&mut self, name: Symbol) -> Result<(), String>;
    /// instantiate：执行模块体（Phase 0 实例化）；
    /// 传递依赖的 instantiate + visit 均先行。
    pub fn instantiate(&mut self, name: Symbol) -> Result<(), String>;
    pub fn find(&self, name: Symbol) -> Option<&ModuleEntry>;
    pub fn entries(&self) -> &[ModuleEntry];
    pub fn pending_instantiations(&self) -> usize;
}
```

**职责边界**：`ModuleRegistry` 只做相位簿记（声明依赖、登记完成态、保证先行次序），**不执行**任何展开或求值——展开由 `ExpandCtxt`（本文 §2.2）驱动，求值由 VM/Runtime 执行。这是「相位分离是共享上下文而非独立阶段」（本文 §3）在数据结构上的体现：registry 是 Expander 与 Compiler 共同查询的相位表。

**幂等性与先行次序不变式**：`visit`/`instantiate` 对已完成的模块幂等跳过（防循环依赖无限递归）；对传递依赖先递归调用自身——「传递依赖的相位传播」规则 3 的机械落实。

## 2. 基础宏系统（原 §8.10，v5.1 契约回填）

卫生宏 + SyntaxObject，参考 Racket 但简化实现。Stage 0 的宏系统是"骨架"——支持宏定义、宏展开、卫生性保证，但不支持复杂的宏组合（如 `syntax-parse`）。

**能力边界（P1 处理程度的精确划定）**：
- ✅ 已实现：syntax-rules 宏（模式/字面量/省略号/模板实例化）、Rust 内置变换器（语法糖推导，[01-核心原语 §3](./01-core-forms.md) 推导表）、卫生重命名（引入标识符唯一化）、宏自引用（递归宏）、展开深度上限保护
- ⛔ Stage 0 推迟：`syntax-parse` 类结构化宏 DSL（Stage 2+）、宏展开调试工具（宏展开逐步跟踪，Stage 2+）、过程宏（任意 Rust 代码作为变换器——信任模型未定，[13-能力矩阵 §3.2](./13-capability-matrix.md)）

### 2.1 变换器契约（Stage 0 冻结，kerf-expander/src/macro_sys.rs）

```rust
/// 变换器种类。
pub enum TransformerKind {
    /// 用户 syntax-rules 变换器。
    Rules(SyntaxRules),
    /// Rust 内置变换器（语法糖派生，§3.2 推导表）。
    Builtin(fn(&[Stx], &mut HygieneCtx) -> Result<Stx, String>),
}

/// 变换器封装（携带宏定义处作用域——卫生展开的并集成分）。
pub struct Transformer {
    pub kind: TransformerKind,
    pub def_scopes: ScopeSet,     // 宏定义处作用域集
}

impl Transformer {
    /// 应用变换器（语法 → 语法）。
    /// `self_name` 为宏自身符号——模板中对该符号的引用**不重命名**
    /// （自引用递归宏的正确性要求，Stage 0 裁定）。
    pub fn apply_named(
        &self,
        self_name: Symbol,
        args: &[Stx],
        table: &mut SymbolTable,
        use_site_scopes: &ScopeSet,
    ) -> Result<Stx, String>;
}
```

**卫生性关键**：`apply_named` 的产物自动携带「宏定义处作用域 ∪ 使用处作用域」的并集（`retag_scopes`）——定义处作用域让宏引入的标识符解析到宏定义可见的绑定，使用处作用域让宏参数（用户代码片段）解析到调用点可见的绑定。二者**永不合并为单一集合**（本文 §4 不变式 2）。

### 2.2 展开上下文契约（kerf-expander/src/expander.rs）

```rust
/// 宏展开深度上限（本文 §4 不变式 1 的取值裁定）。
/// 取值 128 与 rustc 默认递归上限一致——真实宏嵌套深度远低于此；
/// 设计文档示例值 10_000 需迭代式工作表展开（TD-007：受限栈环境
/// （测试线程 2MiB）下的安全校准，Stage 1 计划解除）。
pub const MAX_EXPANSION_DEPTH: u32 = 128;

/// 展开上下文。
pub struct ExpandCtxt {
    pub table: SymbolTable,                            // 共享符号表（唯一可信数据源）
    transformers: HashMap<Symbol, Transformer>,        // 变换器注册表（Phase 1 世界）
    depth: u32,                                        // 当前宏展开深度
}

impl ExpandCtxt {
    pub fn register_transformer(&mut self, name: Symbol, transformer: Transformer);
    pub fn lookup_transformer(&self, name: Symbol) -> Option<&Transformer>;
}

/// 顶层入口：展开程序（形式序列 → CoreExpr 序列）。
pub fn expand_program(forms: &[Stx], ctx: &mut ExpandCtxt) -> Result<Vec<Rc<CoreExpr>>, ExpandError>;
/// 展开单个形式（字面量/符号直接映射，列表递归展开）。
pub fn expand_form(stx: &Stx, ctx: &mut ExpandCtxt) -> Result<Rc<CoreExpr>, ExpandError>;
```

### 2.3 syntax-rules 文法（宏定义形式）

```text
(define-syntax name
  (syntax-rules (literal₁ … literalₘ)          ; 字面量集：模式中这些标识符必须精确匹配
    [pattern template]                          ; 规则 1（首个匹配即应用）
    …))                                         ; 规则 n（按序尝试）

pattern ::= identifier                          ; 模式变量（绑定到匹配子形式）
         | literal                              ; 字面量（精确匹配同名标识符）
         | (pattern … pattern . pattern)        ; 点对模式（固定尾部）
         | (pattern … pattern ellipsis)         ; 省略号模式（零或多次重复）
         | (pattern … pattern ellipsis . pattern) ; 尾省略号 + 固定尾部
         | _                                    ; 通配（匹配任意，不绑定）

template ::= identifier                        ; 模式变量的引用（替换为匹配项）
          | (template … template)               ; 逐项替换
          | (template … template ellipsis)      ; 省略号展开（与模式省略号同维）
          | (… template)                        ; 展开转义（显式拼接）
```

**省略号深度规则**：模板中省略号后的子模板按对应模式省略号的绑定做笛卡尔展开（嵌套省略号允许，维度必须匹配，否则展开期报错）——`SyntaxRules::apply` 的核心循环（kerf-expander/src/macro_sys.rs）。

### 2.4 卫生上下文契约

```rust
/// 卫生上下文：引入标识符的唯一化重命名。
/// **一致性不变式**：同一次模板实例化中，同一引入标识符的全部出现
/// 重命名为**同一**新符号（否则宏体内同名绑定自相分裂）。
pub struct HygieneCtx<'t> { /* table: &'t mut SymbolTable */ }

impl HygieneCtx<'t> {
    /// 开启一次模板实例化（后续 fresh_symbol 计数从零起）。
    pub fn begin_instantiation(&mut self);
    /// 豁免符号（保留集）：宏自引用名 / 核心关键字不参与重命名。
    pub fn preserve(&mut self, sym: Symbol);
    /// 为引入标识符分配新鲜符号（base 名保留渲染层，符号身份唯一）。
    pub fn fresh_symbol(&mut self, base: Symbol) -> Symbol;
    /// 本次实例化累计重命名数（卫生测试的量化观测点）。
    pub fn renames(&self) -> u32;
}
```

**卫生性的三重保证**：(1) **引入标识符隔离**——宏模板引入的绑定名经 `fresh_symbol` 唯一化，不与调用点同名变量冲突；(2) **自由标识符穿透**——宏体引用的宏定义处自由变量经 `def_scopes` 并集解析到宏定义处绑定（非调用点绑定）；(3) **自引用豁免**——宏模板中对宏自身名的引用经 `preserve` 保留，递归宏正确展开。

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
1. **展开终止性**：每次宏调用产生的语法对象携带"展开代次 + 1"的 expansion_id（[02-语法模型 §2 的 Span 定义](./02-syntax-model.md)）；超过上限报错而非栈溢出（实现取值 128——rustc 默认对齐 + TD-007 迭代式解除计划，见本文 §2.2）
2. **卫生性保持**：宏引入的标识符作用域集 ≠ 用户代码作用域集，二者在 SyntaxObject 中永不合并为一个集合
3. **相位封闭性**：Phase 1 的 transformer 只能产生 Phase 0 语法对象，不能反向执行 Phase 0 代码（本文 §1 规则 1）
4. **同类路径审查（§20.3 迭代审计）**：lambda 体铺平（4 处构造点统一）、letrec 与提升路径 nil 包裹、while 递归裸符号、卫生一致性（同标识符 → 同一重命名符号）、宏自引用保留集、核心关键字重命名豁免——六项同类修复簇已全部落地（worklog Task 4-b）

**实现陷阱**：
- **非局部展开的 Span 悬空**：宏展开产物中所有 SyntaxObject 必须携带有效 Span（宏定义处或调用处的合成 Span），"空 Span"会让后续诊断失效——这是 rustc 早期实际踩过的坑
- **展开缓存键必须含作用域集**：同一宏名在不同作用域下解析到不同 transformer，缓存键漏掉作用域集会产生错误复用
- **set! 与 define 的展开顺序**：`Define` 在展开期需区分"函数体内部"（转为 SetBang + 局部绑定）与"模块顶层"（保持 Define），处理不当会静默改变语义

## 5. 测试锚点（设计驱动测试，测试验证设计）

| 契约/不变式 | 测试锚点（tests/v0/stage0/） | 验证命题 |
|-----------|------------------------------|---------|
| 卫生性保证（§2.4 三重保证） | plan 套件卫生宏用例：宏内外同名不串扰（集成验证） | 引入隔离 + 自由穿透 |
| syntax-rules 匹配（§2.3 文法） | examples/macros + 展开单测（模式/字面量/省略号/模板实例化） | 首匹配 + 维度匹配报错 |
| 展开终止性（§4 不变式 1） | 深度上限负例：超限报错而非栈溢出 | TD-007 校准值 128 |
| 相位簿记（§1.1 契约） | phase 单测：declare/visit/instantiate 幂等 + 先行次序 | 传递依赖相位传播 |
| 卫生量化观测（§2.4 renames） | HygieneCtx::renames 计数断言 | 一致性不变式 |

> 测试矩阵完整定义见 [11-测试基础设施 §3](./11-testing.md)；本表是其宏系统侧子集。
