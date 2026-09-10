# 语法模型：类型化 Token 流、Span 与结构化诊断

> **Author**: kerf-doc-agent
> **Date**: 2026-09-10（v5.4：Token 叶级计数 44→45 + Keyword 21→22——r8 require 声明关键字）
> **Version**: v5.4（前版 v5.2：Token 计数更正 #17 + NFC 保守子集 #18）
> **Status**: Active
> **处理程度**：P0（必须实现——Stage 0 已落地，kerf-reader + kerf-span + kerf-syntax）｜ **所属 Stage**：Stage 0 ｜ **推迟项**：查询式增量编译按 Span 细粒度失效（Stage 2，[15-架构分层 §3.1](./15-architecture-layers.md)）、编译缓存键与 Span 失效关系（接口预留，[13-能力矩阵 §3.1.4](./13-capability-matrix.md)）

> 本文件收录语法前端的设计与实现：类型化 Token 流 Reader（原 §8.1）、Span 全管线传播（原 §8.6）、结构化诊断框架（原 §8.7）、Span 源位置追踪系统与诊断框架的横切架构约束（原 §12.1/§12.2），以及 Reader 实现框架（原 §19.1：词法器/语法器骨架 + 核心不变式 + 实现陷阱）。九个核心原语的语义定义见 [01-核心原语](./01-core-forms.md)；12 个能力模型的完整矩阵与其余能力见 [13-能力矩阵](./13-capability-matrix.md)（其 §2.1/§2.6/§2.7 为本文件 §1/§2/§3 的规范副本——本文件为专题深化版）；Span/诊断作为横切关注点的分层架构论述见 [15-架构分层](./15-architecture-layers.md)。

---

## 1. 类型化 Token 流 Reader（原 §8.1）

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

**职责边界**（v5.1 调和裁定——区分**词法层**与 **Reader 模块**两级）：
- **词法层（tokenize）**：将字符流转为类型化 Token 流；附带 Span；**不做语法分析**、不执行宏展开、不判断类型正确性
- **Reader 模块（完整）**：词法器 + 递归下降**语法器**（本文 §6）——产出 SyntaxObject 树。语法器仅做**括号平衡的 datum 结构化**（S 表达式的 read：列表/向量/字面量/引号简写），**不做传统文法分析**（本语言的语法即数据结构，不存在关键字驱动的语句文法）——这是「同像性」路线下「不做语法分析」的精确含义：不存在传统意义上的 parse 树，read 直接产生可编程的数据结构

> **源文档矛盾调和说明**：stage0.md 原文 §8.1（职责：不做语法分析）与 §19.1（Reader = 词法器 + 递归下降语法器）表述冲突。本拆分按「词法层/Reader 模块」两级裁定调和，并与 Stage 0 实现（kerf-reader：类型化 Token（**`TokenKind` enum 12 变体**，叶级展开 44 种——见下）+ 递归下降语法器读 datum + 'x 简写 → (quote x)）一致。

**Stage 0 实现落点**（kerf-reader / kerf-syntax）：设计层的 `TokenKind`（上图）是 2026 现代方案的**方向性蓝图**（含 `TypeIdentifier / Keyword(fn/let/match) / MacroInvocation` 等扩展位）；Stage 0 实现为同构的类型化 Token——运算符身份显式化而非字符串比较（`Operator` 携带 `Symbol` 句柄）。**Token 种类计数（v5.2 更正，deep-review R1 偏差 #17）**：`TokenKind` enum 为 **12 个变体**（`IntLiteral / FloatLiteral / StringLiteral / BoolLiteral / NilLiteral / Identifier / QuoteShorthand / Keyword / Operator / Delimiter / MacroInvocation / Eof`）；**叶级展开 45 种**（v5.4，r8 批次 D +require）——构成：字面量 5（Int/Float/Str/Bool/Nil）+ `Identifier` 1 + `QuoteShorthand` 1 + `Keyword` 22（lambda/if/set!/define/begin/module/import/export/quote/let/letrec/let*/cond/else/and/or/when/while/unless/define-syntax/syntax-rules/require）+ `Operator` 10（+ - * / mod < > <= >= =）+ `Delimiter` 4（( ) [ ]）+ `MacroInvocation` 1 + `Eof` 1（5+1+1+22+10+4+1+1 = 45）。早期文档的「41 种类」不可验证（既非 enum 数也非叶级数），废弃该口径。两者差异是「设计方向 → 阶段实现」的裁剪：`TypeIdentifier`/关键字驱动的表面语法属于 Stage 1+ 表面语言层（[12-路线图 §2.5](./12-roadmap.md) 演进矩阵）；核心不变式（无损、位置完备、词法层零语义）两级同构。Token 使用的辅助类型定义于：`Symbol / SymbolTable`（kerf-syntax，NFC 一次归一化 + 关键字预内部化）、`ScopeId / ScopeSet`（kerf-syntax，有序去重 + 并集/子集）、`Keyword`（kerf-syntax，22 变体——v5.4 +require 声明关键字，[01-核心原语 §6](./01-core-forms.md)）、`Operator / Delimiter`（kerf-reader 的 TokenKind 变体载荷），`Span / FileId / ByteOffset / ExpansionId`（kerf-span，字节偏移主键 + 行/列派生渲染）。

**NFC 保守子集裁定（v5.2 入设计，deep-review R1 偏差 #18）**：完整 Unicode NFC 归一化需要归一化库；Stage 0 **零外部依赖**约束（自举信任根显式化）下采用**保守子集**策略：ASCII 走快路径；含组合字符（U+0300..U+036F 等）的输入走归一化路径，按「基字符 + 后随重音组合 → 预组字符」的 **Latin-1 常见映射表**折叠（覆盖 ç/Ç 等组合对——组合形式与预组形式必须内部化为同一 Symbol，测试锚定）。映射表之外的组合序列**不被归一并保持原样**（两个 Symbol 风险仅限该子集外的输入）——这是「零依赖 > 完整 NFC」的显式裁定，完整 NFC 于引入 Unicode 依赖的时点（Stage 1+，与 [12-路线图 §2.5](./12-roadmap.md) 评估）升级，接口（intern 时一次且仅一次归一化）不变。

**接口契约**：

```rust
pub trait Reader {
    fn tokenize(&mut self, source: &str) -> Result<Vec<Token>, LexError>;
    // 保证：输出的 Token 流覆盖整个输入（无损）
    // 保证：每个 Token 携带精确的 Span
    // 保证：词法错误返回结构化的 LexError（含位置信息）
}
```

## 2. Span 全管线传播（原 §8.6）

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

## 3. 结构化诊断框架（原 §8.7）

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

## 4. Span 源位置追踪系统：横切架构约束（原 §12.1）

基础设施层的三个组件（Span、诊断、相位分离）是**横切关注点**：它们不处于编译管线的主数据流上，却被管线每个阶段消费。本节论述 Span 的架构约束；诊断框架见本文 §5，相位分离见 [03-宏系统](./03-macro-system.md)。

**设计原则**：源位置信息从 Reader 产生，贯穿 Expander、Compiler、VM，且必须在所有中间数据结构中作为不可变字段存在。

**为什么不可推迟**：没有 Span 的 AST 无法支持增量编译、调试器、或任何形式的有用错误报告。后期补上意味着重写所有数据结构。Span 的规范定义见本文 §2——采用字节偏移表示（`ByteOffset`），行号/列号在渲染诊断时由偏移派生，避免双源存储造成的不一致。

Span 在管线各阶段的消费方式：

| 阶段 | Span 携带位置 | 消费方式 |
|------|-------------|---------|
| Reader | 每个 Token | 词法错误定位、无损覆盖校验 |
| Expander | SyntaxObject + 宏展开代次（expansion_id） | 多阶段错误关联、卫生性追踪 |
| Compiler | 字节码指令 → debug_info_table | 运行时错误反查源码 |
| VM | debug_info_table（按 pc 查询） | 堆栈追踪生成、调试器断点 |
| 增量编译 | 文件 × 字节区间 | 细粒度失效传播（[15-架构分层 §3.1](./15-architecture-layers.md)） |

这张消费表定义了 Span 的**最小字段集**：任何少于上述字段的设计都会在某个阶段失去反查能力——这正是"横切关注点必须早期内置"（[17-设计原则](./17-principles.md) 原则 6）的具体依据。

## 5. 诊断框架：横切架构约束（原 §12.2）

统一错误数据结构包含严重性、错误码、主消息、主 Span、子位置和建议修复（Rust 结构定义见本文 §3）。作为横切关注点，诊断框架的核心架构约束是**"错误是数据而非异常"**：诊断对象在每个阶段被构造、聚合、按严重性排序后统一渲染，而不是通过异常冒泡打断管线。这带来三个直接后果：其一，编译器可以在单次运行中报告多个错误（异常模型只能报告一个）；其二，Expander 可以在错误恢复后继续展开后续形式，为 IDE 提供增量反馈；其三，诊断的结构化字段（severity / code / suggestions）可以被 LSP（Stage 2）直接消费而无需二次解析。多阶段错误关联（同一错误的词法、展开、编译三个层面位置）依赖 Span 的 expansion_id 字段串联。

## 6. Reader 实现框架（原 §19.1）

> **实现框架系列说明（原 §19 章导言，适用于 02/03/04/05 四个文件的实现框架章节）**：本系列将 [13-能力矩阵 §2](./13-capability-matrix.md) 定义的 12 个能力模型落实为可直接照抄实现的伪代码框架。所有伪代码采用 Rust 风格语法（与能力模型定义保持一致），但刻意停留在"控制流 + 数据流"层面，不绑定具体内存布局——同一框架可无损翻译为 OCaml（推荐宿主）或 C（VM 基座）。每节按「算法骨架 → 核心不变式 → 实现陷阱」组织：不变式是测试设计的直接依据，陷阱清单来自历史实现（Guix 自举链、rustc、Racket BC）踩过的坑。

Reader = UTF-8 感知的词法器 + 递归下降语法器：输入字符流，输出携带 Span 的 Token 流与 SyntaxObject 树（能力模型定义见本文 §1，接口契约见同节）。

> **B3 自举交付注记（r6，Stage 1 批次 B）**：本框架已以 **kerf 源码**双实现——
> `kerf-driver/src/bootstrap/reader.krf`（词法 + 语法全逻辑，Stage 0 VM 上运行；
> 生产读路径 `compile_front` 经此实现）与种子 `kerf-reader`（Rust——引导编译
> reader.krf + parity oracle）。两实现行为契约：Token 流（种类/Span/payload）、
> datum 树、错误消息与 Span **逐字节一致**（`tests/v0/stage1/plan/bootstrap_
> reader_tests.rs` 为验收门，356 全套件经自举 Reader 实证）。词法骨架的宿主
> 依赖收敛为 4 个运行时原语（str->pos-chars / char-whitespace? / char-alphabetic? /
> str-int-valid?，见 [09-stdlib](./09-stdlib.md) v5.4）——字符级索引与 Unicode
> 属性判定是与 Racket string-ref/char-whitespace? 同层的运行时服务，非语言语义面。

**词法器骨架**：

```rust
fn lex(source: &str, file_id: FileId) -> Result<Vec<Token>, LexError> {
    let mut chars = source.char_indices().peekable();  // (byte_offset, char)
    let mut toks = Vec::new();
    let (mut line, mut col) = (1u32, 1u32);
    while let Some((off, ch)) = chars.next() {
        match ch {
            '\n' => { line += 1; col = 1; }
            ' ' | '\t' => { col += 1; }
            '(' | ')' | '{' | '}' | '[' | ']' => {
                toks.push(Token::delimiter(ch,
                    Span::new(file_id, off, off + 1, line, col)));
                col += 1;
            }
            c if c.is_ascii_digit() => {
                let start = off;
                let mut text = String::new();
                text.push(c);
                while matches!(chars.peek(), Some((_, d)) if d.is_ascii_digit()) {
                    let (_, d) = chars.next().unwrap();
                    text.push(d);
                }
                toks.push(Token::int_lit(&text, Span::new(file_id, start, off + text.len(), line, col)));
                col += text.len() as u32;
            }
            c if is_id_start(c) => {           // Unicode 感知：XID_Start
                let start = off;
                let mut text = String::from(c);
                while matches!(chars.peek(), Some((_, d)) if is_id_continue(*d)) {
                    text.push(chars.next().unwrap().1);
                }
                let sym = intern(text);         // Symbol 内部化（一次性）
                toks.push(Token::classify(sym, Span::new(file_id, start, off + text.len(), line, col)));
                col += text.chars().count() as u32;
            }
            '"' => { /* 字符串字面量：处理转义与跨行，逐字符更新 line/col */ }
            _ => return Err(LexError::new(ch, Span::new(file_id, off, off + ch.len_utf8(), line, col))),
        }
    }
    toks.push(Token::eof());
    Ok(toks)
}
```

**语法器骨架（递归下降，每非终结符一函数）**：

```rust
fn parse_expr(&mut self) -> Result<Stx, ParseError> {
    match self.peek_kind() {
        Keyword(Let)     => self.parse_let(),       // let x = e in body
        Keyword(Fn)      => self.parse_lambda(),    // fn (params) body
        Keyword(If)      => self.parse_if(),        // if c { t } else { e }
        MacroInvoke(m)   => self.parse_macro_use(m),// 扩展点：原样包装交给 Expander
        Ident(_)         => self.parse_call_or_ref(),
        IntLit(_) | StrLit(_) | BoolLit(_) => self.literal(),
        _ => Err(ParseError::unexpected_token(self.peek())),
    }
}

fn parse_if(&mut self) -> Result<Stx, ParseError> {
    let kw = self.expect(Keyword(If))?;              // 消费并校验关键字
    let cond = self.parse_expr()?;
    let then_b = self.parse_block()?;
    let else_b = if self.peek_is(Keyword(Else)) { self.parse_block()? } else { Stx::unit() };
    Ok(Stx::if_form(cond, then_b, else_b, kw.span.merge(else_b.span)))
}
```

**核心不变式**：
1. **无损性**：Token 流的 Span 并集精确覆盖输入字节区间，无间隙、无重叠——这是 [15-架构分层 §3.1](./15-architecture-layers.md) 查询式增量编译按 Span 细粒度失效的前提
2. **位置完备性**：任何错误路径都能构造携带完整 Span 的 LexError / ParseError，直接进入本文 §3 诊断框架
3. **词法层零语义**：关键字分类是纯查表，不涉及作用域或类型判断；`MacroInvocation` Token 只是扩展点标记，Reader 不尝试展开

**实现陷阱**：
- **列号语义必须二选一**：字节列与字符列不可混用（UTF-8 多字节字符下二者不同）。本设计统一为"字节偏移为主键，行/列仅用于诊断渲染"，Span 存偏移、渲染时派生
- **标识符 Unicode 规范化时机**：NFC 归一化应在 intern 时做一次且仅一次，否则同一视觉标识符会产生两个 Symbol
- **字符串跨行会破坏行号**：词法器内的 line 计数必须在字符串字面量内部继续维护
- **拒绝"贪心数字"**：`123abc` 应报错（数字后紧跟标识符字符），而非拆成两个 Token——贪心拆分会掩盖用户笔误

## 7. 测试锚点（设计驱动测试，测试验证设计）

| 契约/不变式 | 测试锚点（tests/v0/stage0/） | 验证命题 |
|-----------|------------------------------|---------|
| 无损性（§6 不变式 1） | Reader 快照测试 ≥ 20 项（Token 流 Span 并集精确覆盖，无间隙无重叠） | 细粒度失效前提 |
| 位置完备性（§6 不变式 2） | negative_reader_tests：词法/括号三态错误负例（含 Span 精确字节偏移断言——59 case，E0001 码直接断言） | 错误可直接进入诊断 |
| NFC 归一化一次（§6 陷阱 2，保守子集） | 同一视觉标识符 → 同一 Symbol 单测（Latin-1 组合/预组对） | 双 Symbol 分裂防御 |
| 'x 简写（§6 语法器） | quote 读取快照：'(quote x) 等价 + negative_reader_tests::quote_shorthand_at_eof | 同像性入口 |
| 嵌套块注释/字符串跨行 | 词法器单测（line 计数在字面量内部维护）+ negative_reader_tests::unclosed_string_literals / unclosed_block_comments | 行号正确性 |
| 渲染形状（§3 诊断） | negative_reader_tests::reader_error_rendering_shape（error[E0001] + `-->` + 源摘录）/ reader_error_code_is_e0001（DiagnosticCode 结构化断言） | 诊断是数据（§8.7） |

> 测试矩阵完整定义见 [11-测试基础设施 §3](./11-testing.md)；本表是其语法前端侧子集。
