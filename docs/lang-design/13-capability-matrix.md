# Stage 0 能力矩阵与 12 个能力模型完整设计

> **Author**: kerf-doc-agent
> **Date**: 2026-09-10（v5.3：r7 批次 C——编译缓存 P2 做实（内存内容寻址 + 管线复用）注记 + 类型检查器 Stage 1 引入交付注记；v5.2：§3.1 四项预留接口以 reserved.rs 冻结签名回填（#16）+ 能力模型 I/O 处理程度 P3→P2（#21））
> **Version**: v5.3
> **Status**: Active

> 本文件是 12 个能力模型详细设计（原 §8）的**唯一完整副本**，同时收录 Stage 0 能力矩阵与职责边界（原 §7：三层分类矩阵与架构原则）、接口预留与完全推迟的能力（原 §9 全文）、以及三个待定决策的前置架构约束（原 §15）。其中 §8.1/§8.6/§8.7、§8.9/§8.10、§8.12、§8.8/§8.11 的正文同时收录于对应主题文件（[02-语法模型](./02-syntax-model.md) / [03-宏系统](./03-macro-system.md) / [04-字节码 VM](./04-bytecode-vm.md) / [05-运行时](./05-runtime.md)）以保证自包含；关键算法伪代码收口于 02-05 的「实现框架」章节。能力选型的批判性审视与 2026 现代方案见 [14-替代设计](./14-design-alternatives.md)；能力引入时机与处理程度（P0-P4）的进程视角见 [12-路线图 §2](./12-roadmap.md)。

---

## 1. Stage 0 能力矩阵与职责边界（原 §7）

> **截至 2026 年，Stage 0 应采用"混合务实"策略——核心语义层使用经过验证的传统方案（元循环求值器、闭包、基础同像性），基础设施层采用 2026 年已成熟的现代方案（类型化 Token 流、Span 追踪、结构化诊断），而高级 2026 方案（Effect Handlers、多阶段编程、能力模型 I/O）应仅设计接口预留而不实现——因为它们要么生态尚未完全成熟（如 Mojo 的"内存安全模型尚未完成"），要么仍主要处于研究阶段（如 MetaOCaml 的"实际实现往往使用启发式方法"）。**

### 1.1 能力矩阵 v4.0：三层分类（必须实现 / 接口预留 / 完全推迟）（原 §7.1）

#### 1.1.1 完整三层分类矩阵（原 §7.1.1）

| 能力模型 | 分类 | 2026 技术选择 | 成熟度评估 | 推迟到 |
|---------|------|-------------|-----------|--------|
| **类型化 Token 流 Reader** | ✅ 必须实现 | Rust proc-macro 风格 | ✅ 成熟（Rust 生产验证） | - |
| **图 IR（含共享）** | ✅ 必须实现 | 类型化节点 + 共享边 | ⚠️ 新兴（LLVM/GCC 生产验证） | - |
| **结构化 CodeValue** | ✅ 必须实现 | 携带类型/Span/Scope 的代码值 | ✅ 成熟（Rust + MetaOCaml 验证） | - |
| **元循环求值器（简化版）** | ✅ 必须实现 | 传统 eval/apply（不用 Effects） | ✅ 极成熟（60 年验证） | - |
| **基础闭包** | ✅ 必须实现 | 传统词法闭包 | ✅ 极成熟 | - |
| **Span 全管线传播** | ✅ 必须实现 | rustc 风格 Span 系统 | ✅ 成熟 | - |
| **结构化诊断框架** | ✅ 必须实现 | rustc diagnostic 风格 | ✅ 成熟 | - |
| **最小 I/O（传统）** | ✅ 必须实现 | 单 syscall / stdin/stdout | ✅ 极成熟 | - |
| **相位分离系统** | ✅ 必须实现 | Racket 风格 Phase 0/1 | ✅ 成熟 | - |
| **基础宏系统** | ✅ 必须实现 | 卫生宏 + SyntaxObject | ✅ 成熟（Racket 验证） | - |
| **标记-清除 GC** | ✅ 必须实现 | bump-pointer + 递归标记 | ✅ 成熟 | - |
| **字节码 VM** | ✅ 必须实现 | switch-dispatch，约 35 操作码 | ✅ 成熟 | - |
| **Effect Handlers** | ⚠️ 接口预留 | OCaml 5 风格（仅类型定义） | ⚠️ 新兴（正在用于 Forester 6.0） | Stage 2 |
| **多阶段编程** | ⚠️ 接口预留 | MetaOCaml 风格（仅类型定义） | ⚠️ 研究（实际实现使用启发式） | Stage 2 |
| **能力模型 I/O** | ⚠️ 接口预留 | 线性类型 + capability | ⚠️ 新兴（Rust 所有权验证） | Stage 1 |
| **编译缓存** | ✅ 做实（r7，P2 最小体） | 查询式缓存（内存内容寻址：SHA-256 截断键 + 三方法 + 管线富入口；salsa 依赖图 Stage 2 深化） | ✅ 成熟（rustc 增量验证） | Stage 2 深化 |
| **LSP 集成** | ❌ 完全推迟 | JSON-RPC over stdio | ✅ 成熟（非必需） | Stage 2 |
| **本地码生成** | ❌ 完全推迟 | QBE/LLVM/C 转译 | ✅ 成熟（非 Stage 0 目标） | Stage 2 |
| **类型检查器** | ✅ Stage 1 已引入（r7，外部 Rust 实现） | 保守静态检查（R1-R8 确定性规则 + 多错误收集；Hindley-Milner 完全体 Stage 2+） | ✅ 成熟（循环依赖经外部实现缓解） | Stage 2 完全体 |
| **优化器** | ❌ 完全推迟 | 分代/增量 | ✅ 成熟（非 Stage 0 目标） | Stage 3+ |
| **GC（高级）** | ❌ 完全推迟 | 分代/增量/并发 | ✅ 成熟 | Stage 2+ |
| **JIT** | ❌ 完全推迟 | 元追踪 | ✅ 成熟 | Stage 3 |
| **FFI** | ❌ 完全推迟 | C ABI / nan-boxing | ✅ 成熟 | Stage 2 |
| **并发/线程** | ❌ 完全推迟 | Actor / CSP / STM | ✅ 成熟 | Stage 3 |

#### 1.1.2 三层分类的设计哲学（原 §7.1.2）

**为何采用三层分类而非二元"实现/推迟"**：

1. **必须实现层** = Stage 0 的核心交付物。语义引擎用最成熟的传统方案，避免技术风险；基础设施用 2026 年已成熟的现代方案，提升工程质量
2. **接口预留层** = 为 2026 年前沿技术留好位置。仅定义类型签名和 trait/模块接口，不写实现。Stage 1+ 可以无缝替换为前沿方案的实现，而无需破坏现有调用方
3. **完全推迟层** = 不在 Stage 0 设计范围内。要么存在循环依赖（类型检查器），要么非 Stage 0 目标（优化器、JIT），要么会破坏自举闭环（FFI）

### 1.2 2026 年架构原则总结（原 §7.2）

**Stage 0 的设计哲学是"用经过验证的成熟技术构建语义引擎，用 2026 年已成熟的基础设施技术（类型化 Token 流、Span 系统、结构化诊断）提升工程质量，为 2026 年的前沿技术（Effect Handlers、多阶段编程、能力模型）预留演进空间"。**

这不是保守主义，而是工程务实——2026 年的技术前沿（如 Mojo 的"内存安全模型尚未完成"、MetaOCaml 的"实际实现使用启发式"）表明，直接采用最前沿方案存在风险。**正确策略是：在数据结构和接口层面为前沿技术留好位置，在实现层面使用成熟方案，在 Stage 2+ 逐步替换为前沿方案的实现。**

**四个核心架构原则**：
1. **成熟技术优先原则**：Stage 0 的每个组件都使用至少在生产环境验证过的技术
2. **接口先行原则**：为 2026 前沿技术（Effects、多阶段、能力 I/O）预留接口但不实现
3. **传统+现代混合原则**：核心语义用传统方案（60 年验证），基础设施用现代方案（Rust 生态验证）
4. **渐进演进原则**：Stage 1+ 可以替换 Stage 0 的实现，但接口契约不变

本章确立的"混合务实"策略（成熟语义方案 + 现代基础设施 + 前沿技术接口预留）是后续所有架构决策的裁决标准：本文 §2 按「能力模型 → 职责边界 → 接口契约」三段式展开 12 个必须实现的能力，本文 §3 锁定接口预留与完全推迟的边界，[15-架构分层](./15-architecture-layers.md) 给出分层与调用关系的总览。当任何新设计诉求与三层分类冲突时，回到本文 §1.1.2 的三条设计哲学重新裁决。

---

## 2. 必须实现的 12 个能力模型详细设计（原 §8）

> 本章覆盖本文 §1.1 矩阵中「必须实现」层的全部 12 个能力模型 = **10 个语义与编译期能力**（Reader、图 IR、CodeValue、元循环求值器、闭包、Span、诊断、最小 I/O、相位分离、宏系统）+ **2 个运行时基座**（标记-清除 GC、字节码 VM）。原 v3.0 将其表述为"10+2"，v4.0 起统一为"12 个能力模型（10 + 2）"。每个能力按「能力模型 → 职责边界 → 接口契约」三段式组织；关键算法的完整可落地伪代码统一收口在「实现框架」系列文档：[02-语法模型 §6（Reader）](./02-syntax-model.md)、[03-宏系统 §4（Expander）](./03-macro-system.md)、[04-字节码 VM §2/§3（Compiler/VM）](./04-bytecode-vm.md)、[05-运行时 §4（GC）](./05-runtime.md)。

### 2.1 类型化 Token 流 Reader（原 §8.1）

**能力模型**：

```rust
// 2026 成熟方案：Rust proc-macro 风格的类型化 Token
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,      // 类型化的 Token 种类
    pub span: Span,           // 源位置（file, start_line, start_col, end_line, end_col）
    pub scope_id: ScopeId,   // 作用域标识
}

#[derive(Debug, Clone)]
pub enum TokenKind {
    // 字面量
    IntLiteral(i64), FloatLiteral(f64), 
    StringLiteral(Rc<str>), BoolLiteral(bool),
    
    // 标识符
    Identifier(Symbol),         // interned string
    TypeIdentifier(Symbol),
    
    // 关键字
    Keyword(Keyword),           // fn, let, if, match, ...
    
    // 运算符（携带优先级信息）
    Operator(Operator),
    
    // 分隔符
    Delimiter(Delimiter),       // (, ), {, }, [, ]
    
    // 宏扩展点
    MacroInvocation(Symbol),
    
    Eof,
}
```

**职责边界**：
- **做什么**：将字符流转为类型化 Token 流；附带 Span 和 Scope 信息
- **不做什么**：不做语法分析；不执行宏展开；不判断类型正确性

**接口契约**：

```rust
pub trait Reader {
    fn tokenize(&mut self, source: &str) -> Result<Vec<Token>, LexError>;
    // 保证：输出的 Token 流覆盖整个输入（无损）
    // 保证：每个 Token 携带精确的 Span
    // 保证：词法错误返回结构化的 LexError（含位置信息）
}
```

### 2.2 图 IR（含共享节点）（原 §8.2）

**能力模型**：

```rust
pub struct GraphIR {
    pub nodes: Arena<IRNode>,     // arena 分配，NodeId 索引
    pub edges: Vec<IEdge>,
    pub node_map: FxHashMap<IRNodeHash, Vec<NodeId>>,  // 哈希 → 节点（支持共享查找）
}

pub enum IRNode {
    // 操作节点
    BinOp { op: Operator, lhs: NodeId, rhs: NodeId },
    Call { callee: NodeId, args: Vec<NodeId> },
    Lambda { params: Vec<Symbol>, body: NodeId },
    
    // 数据节点
    Literal(LiteralValue),
    Variable(Symbol),
    
    // 控制流
    If { cond: NodeId, then_branch: NodeId, else_branch: NodeId },
    
    // 每个节点都携带 Span 和类型信息
    // (在 Arena 中通过辅助表存储)
}

pub struct NodeMetadata {
    pub span: Span,
    pub type_info: Option<TypeInfo>,
    pub scope: ScopeSet,
}
```

**职责边界**：
- **做什么**：表示程序的结构化形式；支持节点共享（公共子表达式）；携带完整的元信息
- **不做什么**：不执行求值；不做类型推断；不处理控制流执行顺序

**接口契约**：

```rust
pub trait IR {
    type NodeId;
    
    // 构造
    fn add_node(&mut self, node: IRNode, meta: NodeMetadata) -> NodeId;
    fn add_edge(&mut self, from: NodeId, to: NodeId, kind: EdgeKind);
    
    // 查询
    fn get_node(&self, id: NodeId) -> &IRNode;
    fn get_metadata(&self, id: NodeId) -> &NodeMetadata;
    fn find_shared(&self, node: &IRNode) -> Vec<NodeId>;  // 查找共享节点
    
    // 变换（不可变，返回新 IR）
    fn map_nodes(&self, f: impl Fn(NodeId, &IRNode) -> IRNode) -> Self;
    fn substitute(&self, var: Symbol, replacement: NodeId) -> Self;
}
```

### 2.3 结构化 CodeValue（原 §8.3）

**能力模型**：

```rust
pub struct CodeValue {
    pub ir: GraphIR,             // 完整的图 IR
    pub span: Span,              // 源位置
    pub scope: ScopeSet,         // 作用域
    pub type_info: Option<TypeInfo>,  // 类型信息（如果已知）
    pub free_vars: Vec<Symbol>,  // 自由变量列表
    pub stage: Stage,            // 阶段信息（为多阶段编程预留）
}
```

**接口契约**：

```rust
pub trait CodeValue {
    // 检查
    fn is_well_formed(&self) -> bool;
    fn free_variables(&self) -> Vec<Symbol>;
    
    // 变换（不可变）
    fn substitute(&self, var: Symbol, val: &CodeValue) -> Self;
    fn alpha_rename(&self, from: Symbol, to: Symbol) -> Self;
    
    // 多阶段编程的接口预留（Stage 2 实现）
    fn compose_with(&self, other: &Self) -> Result<Self, CompositionError>;
    
    // 职责边界：仅作为代码的值表示，不执行隐式编译
}
```

### 2.4 元循环求值器（简化版）（原 §8.4）

**为什么用传统方案而非 Effect Handlers**：OCaml 5 的效应系统虽然"正在 Forester 6.0 中实际使用"，但主要用于工具和基础设施，而非作为语言核心的求值器。Stage 0 用传统的 eval/apply 相互递归更安全。

**能力模型**：

```rust
// 简化的元循环求值器（不用 Effects）
pub enum Value {
    Closure { params: Vec<Symbol>, body: NodeId, env: Env },
    Literal(LiteralValue),
    Unit,
}

pub type Env = Rc<RefCell<HashMap<Symbol, Value>>>;

pub fn eval(ir: &GraphIR, node: NodeId, env: &Env) -> Result<Value, EvalError> {
    match ir.get_node(node) {
        IRNode::Literal(v) => Ok(Value::Literal(v.clone())),
        
        IRNode::Variable(name) => {
            env.borrow().get(name).cloned()
                .ok_or(EvalError::UnboundVariable(name.clone()))
        }
        
        IRNode::Lambda { params, body } => {
            Ok(Value::Closure { 
                params: params.clone(), 
                body: *body, 
                env: env.clone() 
            })
        }
        
        IRNode::Call { callee, args } => {
            let f = eval(ir, *callee, env)?;
            let mut arg_vals = Vec::new();
            for arg in args {
                arg_vals.push(eval(ir, *arg, env)?);
            }
            apply(f, arg_vals, ir)
        }
        
        IRNode::If { cond, then_b, else_b } => {
            let c = eval(ir, *cond, env)?;
            match c {
                Value::Literal(LiteralValue::Bool(true)) => 
                    eval(ir, *then_b, env),
                Value::Literal(LiteralValue::Bool(false)) => 
                    eval(ir, *else_b, env),
                _ => Err(EvalError::TypeMismatch)
            }
        }
        
        // ... 其他节点类型
    }
}

pub fn apply(f: Value, args: Vec<Value>, ir: &GraphIR) -> Result<Value, EvalError> {
    match f {
        Value::Closure { params, body, env } => {
            let mut new_env = env.as_ref().clone();
            for (param, arg) in params.iter().zip(args) {
                new_env.borrow_mut().insert(param.clone(), arg);
            }
            eval(ir, body, &Rc::new(new_env))
        }
        _ => Err(EvalError::NotCallable)
    }
}
```

### 2.5 基础闭包（原 §8.5）

传统词法作用域闭包，不做任何花哨的扩展。具体而言：
- **环境捕获**：捕获定义时的词法作用域
- **不可变捕获**：捕获的环境通过 `Rc<RefCell<...>>` 共享，允许可变状态但禁止逃逸
- **不引入 Effect 系统**：保持纯词法闭包，避免 Stage 0 的复杂度爆炸

### 2.6 Span 全管线传播（原 §8.6）

rustc 风格，每个 Token / AST 节点 / IR 节点 / 字节码指令都携带 Span：

```rust
pub struct Span {
    pub file_id: FileId,
    pub start: ByteOffset,
    pub end: ByteOffset,
    pub expansion_id: ExpansionId,  // 宏展开代次
}
```

Span 在每个数据结构中作为不可变字段存在，使得：
- 任何错误都能精确定位到源码
- 调试器可以反查字节码 → IR → 源码
- 增量编译可以基于 Span 进行细粒度失效

### 2.7 结构化诊断框架（原 §8.7）

借鉴 rustc 的 Diagnostic 结构：

```rust
pub struct Diagnostic {
    pub severity: Severity,         // Error / Warning / Note / Help
    pub code: Option<DiagnosticCode>,
    pub message: String,
    pub primary_span: Span,
    pub children: Vec<SubDiagnostic>,  // 关联的次要诊断
    pub suggestions: Vec<Suggestion>,  // 修复建议
}
```

**架构原则**：错误是数据而非异常；支持错误恢复策略；多阶段错误关联。

### 2.8 最小 I/O（传统）（原 §8.8）

仅 `read_line()` 和 `write_line()`，通过全局函数（非能力模型）。Stage 0 不引入能力模型 I/O，但 VM 栈帧和分配器接口必须预留 `register_foreign_ref` 等接口（Stage 0 可为 no-op），以便 Stage 1+ 升级到能力模型时无需破坏接口。

### 2.9 相位分离系统（原 §8.9）

Phase 0（运行时）/ Phase 1（宏展开时）的严格分离，参考 Racket 设计。

**三个关键规则**：
1. Phase 1 代码只能产生 Phase 0 代码，不能直接执行 Phase 0 代码
2. 区分"实例化"（执行模块体）和"访问"（仅执行 Phase 1 部分）
3. 传递依赖的相位传播

模块生命周期通过 **declare / instantiate / visit** 三种操作管理：declare 登记模块与其相位声明，instantiate 执行模块体（Phase 0 实例化），visit 仅执行 Phase 1 部分（宏变换器加载）。该模型的完整架构约束论述见 [03-宏系统 §3（模块相位分离系统）](./03-macro-system.md)。

### 2.10 基础宏系统（原 §8.10）

卫生宏 + SyntaxObject，参考 Racket 但简化实现。Stage 0 的宏系统是"骨架"——支持宏定义、宏展开、卫生性保证，但不支持复杂的宏组合（如 `syntax-parse`）。

### 2.11 标记-清除 GC（最小实现）（原 §8.11）

Bump-pointer 分配 + 递归标记 + 堆遍历清除。根集包括 VM 栈、调用栈局部变量、全局环境和 C 栈显式根。Stage 0 用最简单的 mark-sweep，避免分代/增量/并发的复杂度。

**实现指引**：分配、标记、清除三个阶段的完整伪代码与栈溢出对策见 [05-运行时 §4](./05-runtime.md)。

### 2.12 字节码 VM（原 §8.12）

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

**实现指引**：跳转回填的字节码生成与 switch-dispatch 执行循环的完整伪代码见 [04-字节码 VM §2 与 §3](./04-bytecode-vm.md)。原 v3.0 §8.6「字节码 VM 设计」与本节内容重复，v4.0 已合并至此。

---

## 3. 接口预留与完全推迟的能力（原 §9）

> **接口预留层与完全推迟层共同构成 Stage 0 的"未来边界"。** 二者的区别是本质性的：接口预留层**已确定将被采用**，仅推迟实现（Stage 1-2 落地，Stage 0 冻结类型签名）；完全推迟层**尚未承诺采用**（存在明确的推迟理由，Stage 1+ 重新评估）。这一区分直接决定了本章两类内容的阅读方式——前者是"必须兑现的期票"，后者是"保留的选择权"。

### 3.1 接口预留的 4 个能力模型（原 §9.1）

> **v5.2 签名权威声明（deep-review R1 偏差 #16）**：本节四项预留接口的**冻结签名权威是 `kerf-driver/src/reserved.rs`**（冻结性经 Probe 实现测试 `reserved_signatures_are_frozen` 证明——「测试实现体编译通过 = 契约冻结」）。早期版本的伪签名（如 `Self::Effect::Result` 关联类型路径、`splice(code) -> Self::Code::Inner` 等不可编译形态）仅为设计草稿，以下全部回填为可编译的真实冻结签名。

#### 3.1.1 Effect Handlers（接口预留）（原 §9.1.1）

**预留接口（Stage 0 冻结，P3——仅类型形状）**：

```rust
// reserved.rs 冻结签名（可编译）
pub trait EffectFamily {
    /// 执行该效应后的结果值（P3 形状：以 Value 具体化——Stage 2 可细化为泛型关联）。
    type Result;
}

pub trait Effect {
    /// 所属效应族。
    type Family: EffectFamily<Result = Value>;
}

pub trait EffectSystem {
    type Effect: Effect;
    type Handler;
    type Continuation;

    /// 执行效应：挂起并向上传递（Stage 2 实现；P3 形状：结果经 Value）。
    fn perform(&self, effect: Self::Effect) -> Value;

    /// 安装效应处理器并执行计算（Stage 2 实现）。
    fn handle(&self, handler: Self::Handler, computation: impl FnOnce() -> Value) -> Value;
}
```

**行为规格**（完整语义见 stage0.md §6.4）：`perform` 挂起当前计算，向上查找匹配的 handler；`handle` 安装 handler 并执行计算——效应触发时以 continuation 恢复。Stage 0 裁定：传统闭包（§21.10 决策点 2）；Effects 于 Stage 2 语言级引入。

**预留原因**：OCaml 5 的效应系统正在"实际使用于编译器基础设施"，但作为语言核心的求值器仍属前沿。

#### 3.1.2 多阶段编程（接口预留）（原 §9.1.2）

**预留接口（Stage 0 冻结，P3——仅类型形状）**：

```rust
// reserved.rs 冻结签名（可编译）
pub trait MultiStage {
    /// 代码值类型（Stage 2 具体化为 `CodeValue`）。
    type Code;

    /// 引号：表达式 → 代码值（Stage 2 实现）。
    fn quote(&self, expr: &CoreExpr) -> Self::Code;

    /// 拼接：代码值 → 当前阶段代码（Stage 2 实现）。
    fn splice(&self, code: &Self::Code) -> Result<CoreExpr, RuntimeError>;

    /// 执行代码值（Stage 2 实现）。
    fn run(&self, code: &Self::Code) -> Result<Value, RuntimeError>;
}
```

**行为规格**（MetaOCaml 语义，stage0.md §6.1）：`quote` 将表达式提升为代码值（良构/良作用域保证）；`splice` 将代码值拼接进当前阶段语法；`run` 执行未来阶段代码（Stage 2 做实为编译期计算）。Stage 0 裁定：元循环求值器（§21.10 决策点 1）；多阶段于 Stage 2+ 替换。

**预留原因**：MetaOCaml 虽然理论上成熟（"良构的、良类型的和良作用域的"），但"实际实现往往使用启发式方法"。

#### 3.1.3 能力模型 I/O（接口预留）（原 §9.1.3）

**处理程度（v5.2 修订，deep-review R1 偏差 #21）**：**P2**（原记 P3）——reserved.rs 的 `CapabilityIO` 冻结签名附带**完整行为规格**（下述 4 条），满足 P2 判据「冻结类型签名与行为规格，无实现」（[12-路线图 §2.1.2](./12-roadmap.md) 五级标度）——与 [17-设计原则 §1 原则 26](./17-principles.md) 三级成熟度匹配规则的「研究前沿 → P3」裁定并不冲突：规则给出的是**上限**，实现策略上为Stage 1 直达预留了完整规格（超规格交付）。

**预留接口（Stage 0 冻结，P2）**：

```rust
// reserved.rs 冻结签名（可编译）
/// 读能力令牌（不可伪造——私有构造，经权限传递获得）。
pub struct ReadCapability { _private: () }

/// 写能力令牌（不可伪造）。
pub struct WriteCapability { _private: () }

/// 能力模型 I/O 错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IOError { pub message: String }

pub trait CapabilityIO {
    /// 读一行（需要读能力，Stage 1+ 实现）。
    fn read_line(cap: &mut ReadCapability) -> Result<String, IOError>;

    /// 写一行（需要写能力，Stage 1+ 实现）。
    fn write_line(cap: &mut WriteCapability, s: &str) -> Result<(), IOError>;
}
```

**完整行为规格**（P2——Stage 1 可直接按规格实现）：
1. `read_line` 仅在持有 `ReadCapability` 时可调用——令牌经线性传递，不可复制、不可伪造；
2. `write_line` 同理（`WriteCapability`）；
3. 无令牌的 I/O 调用为**编译期错误**（权限验证，非运行时检查）；
4. 与 Stage 0 传统 I/O（kerf-runtime::io 全局函数）的替换关系：渐进替换原则 §28——driver 注册的内置函数改为能力参数化形态，全局函数逐步退役。

Stage 0 裁定：传统 I/O（§21.10 决策点 3）；能力模型于 Stage 1 基础 / Stage 2 完整引入。

#### 3.1.4 编译缓存（P2 做实——r7 批次 C 交付注记）

> **r7 交付注记（2026-09-10，批次 C）**：三方法规格已按原文落地——
> `kerf-driver/src/cache.rs` 的 `InMemoryCompilationCache` 实现本节冻结
> trait（`get_cached`/`store`/`invalidate` 行为规格 1/2/3 逐条测试锁定），
> 另增管线富入口（`lookup_front`/`store_front` 携带完整前端输出——run/eval/check
> 路径复用）与 SHA-256 内容寻址键（源哈希截断 u64 + 配置指纹=阶段种子+文件名，
> `kerf-driver/src/hash.rs` 零依赖自实现，NIST 向量锚定）。
> §21.3 Stage 1 条件 4「增量编译基础设施可用」就位；Stage 2 salsa
> 依赖图深化时三方法签名不变（规格条款 4）。

**预留接口（Stage 0 冻结，P2）**：

```rust
// reserved.rs 冻结签名（可编译）
/// 缓存键（源哈希 + 编译配置指纹——确定性编译的键）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub source_hash: u64,        // 源文本内容哈希（Stage 1：SHA-256 内容寻址）
    pub config_fingerprint: u64, // 编译配置指纹（阶段版本 + 选项）
}

/// 缓存结果（编译产物——字节码 + 核心表达式 + 代码值）。
#[derive(Debug, Clone)]
pub struct CachedResult {
    pub program: BcProgram,
    pub core: Vec<Rc<CoreExpr>>,
    pub code_value: Option<CodeValue>,
}

pub trait CompilationCache {
    /// 查询缓存。
    fn get_cached(&self, key: &CacheKey) -> Option<CachedResult>;

    /// 写入缓存。
    fn store(&mut self, key: CacheKey, result: CachedResult);

    /// 失效缓存项。
    fn invalidate(&mut self, key: &CacheKey);
}
```

**完整行为规格**（P2——Stage 1 可直接按规格实现）：
1. `get_cached`：键命中返回产物（内容寻址——源不变 + 配置不变 → 产物必然等价，可直接复用）；
2. `store`：写入缓存（幂等——同键覆盖）；
3. `invalidate`：显式失效（源变更由键哈希自然区分；此方法用于编译器升级等全量失效场景）；
4. Stage 2 深化：查询式增量编译（salsa 风格依赖图），本三方法保持签名不变（渐进替换原则 §28）。

### 3.2 完全推迟的能力（原 §9.2）

| 能力 | 推迟到 | 理由 |
|------|--------|------|
| LSP 集成 | Stage 2 | 非核心功能，需要稳定的编译器 API |
| 本地码生成 | Stage 2 | Stage 0 只需字节码 VM |
| 类型检查器 | **Stage 1（r7 已引入——外部 Rust 保守静态检查，循环依赖缓解落地；HM 完全体 Stage 2+）** | 循环依赖问题（需先有稳定的核心；§21.6 缓解：外部实现） |
| 高级 GC | Stage 2+ | Stage 0 用最简单的 mark-sweep |
| 优化器 | Stage 3 | 需要性能基准数据 |
| FFI | Stage 2 | 破坏自举闭环 |
| 并发 | Stage 3 | 语义复杂 |
| JIT | Stage 3 | 依赖 profiling 数据 |

---

## 4. 三个待定决策的前置架构约束（原 §15）

| 决策 | 前置架构约束 | 预期解决时机 |
|------|-------------|-------------|
| **并发内存模型** | IR 中共享内存访问必须显式标记为 `SharedLoad/SharedStore`，Stage 0 保守视为 SeqCst | Stage 3 |
| **错误运行时语义** | VM 栈帧必须预留 3 个扩展槽（continuation/异常/调试） | Stage 2 |
| **FFI 所有权模型** | 分配器必须包含 `register_foreign_ref` 等接口（Stage 0 可为 no-op） | Stage 2 |

> 本节的三个待定决策（运行时语义的未决项）与 [12-路线图 §2.6](./12-roadmap.md) 的三个关键决策点（能力模型选型的权衡）是不同层面的问题，二者的区别详见该文件。
