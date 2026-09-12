//! 基础宏系统：卫生宏 + SyntaxObject（stage0.md §8.10，§19.2）。
//!
//! Stage 0 的宏系统是「骨架」——支持宏定义、宏展开、卫生性保证，
//! 不支持复杂的宏组合（如 `syntax-parse`，显式推迟，技术债 TD-005）。
//!
//! **变换器（Transformer）**：Phase 1 世界的一等公民。
//! - `Builtin`：Rust 实现的内置变换器（let/letrec/cond/... 语法糖，§3.2 推导表）；
//! - `SyntaxRules`：用户定义的模式/模板变换器（匹配 → 替换 → 重命名）。
//!
//! **卫生性**（P1 级保证，§19.2 不变式 2）：
//! - 模板中**非模式变量**的标识符（引入标识符）在展开时 α 重命名为唯一符号
//!   （`name$hyg$N`）——宏内外同名不串扰；
//! - 模式变量替换保留用户代码的标识符（用户符号引用用户绑定）；
//! - 展开产物携带「宏定义处作用域 ∪ 使用处作用域」的并集（§19.2 卫生性关键）。

use kerf_syntax::{Keyword, ScopeSet, Stx, StxDatum, Symbol, SymbolTable};

/// 变换器：语法 → 语法（Phase 1 世界）。
#[derive(Debug, Clone)]
pub enum TransformerKind {
    /// 用户 syntax-rules 变换器。
    Rules(SyntaxRules),
    /// Rust 内置变换器（语法糖派生，§3.2 推导表）。
    Builtin(fn(&[Stx], &mut HygieneCtx) -> Result<Stx, String>),
}

/// 变换器封装（携带宏定义处作用域——卫生展开的并集成分）。
#[derive(Debug, Clone)]
pub struct Transformer {
    /// 变换器种类。
    pub kind: TransformerKind,
    /// 宏定义处作用域集。
    pub def_scopes: ScopeSet,
}

impl Transformer {
    /// 应用变换器（语法 → 语法；不保留任何符号——旧接口，见 apply_named）。
    pub fn apply(
        &self,
        args: &[Stx],
        table: &mut SymbolTable,
        use_site_scopes: &ScopeSet,
    ) -> Result<Stx, String> {
        self.apply_named(Symbol(u32::MAX), args, table, use_site_scopes)
    }

    /// 应用变换器（语法 → 语法；`self_name` 为宏自身符号——模板中对该符号的
    /// 引用**不重命名**（自引用递归宏的正确性要求，Stage 0 裁定）。
    pub fn apply_named(
        &self,
        self_name: Symbol,
        args: &[Stx],
        table: &mut SymbolTable,
        use_site_scopes: &ScopeSet,
    ) -> Result<Stx, String> {
        let mut hygiene = HygieneCtx::new(table);
        hygiene.preserve(self_name);
        let output = match &self.kind {
            TransformerKind::Rules(rules) => rules.apply(args, &mut hygiene)?,
            TransformerKind::Builtin(f) => f(args, &mut hygiene)?,
        };
        // 卫生性关键（§19.2）：展开产物携带「宏定义处作用域 ∪ 使用处作用域」的并集
        let out_scopes = self.def_scopes.union(use_site_scopes);
        Ok(retag_scopes(&output, &out_scopes))
    }
}

/// 卫生上下文：引入标识符的唯一化重命名。
///
/// **一致性不变式**：同一次模板实例化中，同一引入标识符的全部出现
/// 必须重命名为**同一**符号（`renames` 映射保证——否则绑定与引用脱钩）。
pub struct HygieneCtx<'t> {
    table: &'t mut SymbolTable,
    counter: u32,
    /// 保留集：这些符号**不重命名**（宏自引用等）。
    preserve: Vec<Symbol>,
    /// 引入符号 → 重命名符号（本次实例化内一致）。
    renames: std::collections::HashMap<Symbol, Symbol>,
}

impl<'t> HygieneCtx<'t> {
    /// 构造（计数从宏调用点唯一化）。
    pub fn new(table: &'t mut SymbolTable) -> Self {
        HygieneCtx {
            table,
            counter: 0,
            preserve: Vec::new(),
            renames: std::collections::HashMap::new(),
        }
    }

    /// 开始一次模板实例化（重命名映射复位——每次展开独立编号）。
    pub fn begin_instantiation(&mut self) {
        self.renames.clear();
    }

    /// 登记保留符号（不参与 α 重命名）。
    pub fn preserve(&mut self, sym: Symbol) {
        if sym.0 != u32::MAX {
            self.preserve.push(sym);
        }
    }

    /// 引入标识符 → 唯一化符号（α 重命名 `name$hyg$N`；**同标识符多次
    /// 出现映射到同一符号**——一致性不变式）。
    pub fn fresh_symbol(&mut self, base: Symbol) -> Symbol {
        if let Some(&s) = self.renames.get(&base) {
            return s;
        }
        self.counter += 1;
        let name = self.table.name(base);
        let sym = self.table.intern(&format!("{}$hyg${}", name, self.counter));
        self.renames.insert(base, sym);
        sym
    }

    /// 当前重命名计数（测试与调试观测）。
    pub fn renames(&self) -> u32 {
        self.counter
    }

    /// 符号表访问（内置变换器构造引入符号用）。
    pub fn table(&mut self) -> &mut SymbolTable {
        self.table
    }

    /// 只读名字查询（关键字判定等只读路径）。
    pub fn peek_name(&self, sym: Symbol) -> &str {
        self.table.name(sym)
    }

    /// 是否在保留集内。
    fn is_preserved(&self, sym: Symbol) -> bool {
        self.preserve.contains(&sym)
    }
}

/// 递归重打作用域标签（保持 datum/span/phase，仅更新 scopes 字段）。
///
/// **迭代式 + 均匀标记共享（TD-007/H2 双解除）**：
/// 1. **迭代式**：显式工作表后序重建（`Visit` 下行 / `Assemble` 组装），
///    Rust 栈深恒定——旧递归版在 10_000 层链上溢出；
/// 2. **均匀标记快路径**：输入节点 `uniform_tag == Some(target)`（整棵
///    子树作用域均匀 = target——由本函数重建时设置、`add_scope_to_all`
///    注入时清除）时直接浅共享返回（Rc 计数 +1）。trampoline 链各步的
///    产物（前步 retag 输出）命中共几何：链长 N 的总工作量 O(N)（旧版
///    每步全树重建 O(N²)）。
fn retag_scopes(stx: &Stx, scopes: &ScopeSet) -> Stx {
    // 快路径：均匀命中——整棵子树作用域已 = target，浅共享。
    if stx.uniform_tag.as_ref() == Some(scopes) {
        return stx.clone();
    }
    // 工作表：后序重建（子先建、父组装）；栈深恒定。
    enum Task<'a> {
        Visit(&'a Stx),
        Assemble { input: &'a Stx, child_count: usize },
    }
    let mut tasks: Vec<Task<'_>> = vec![Task::Visit(stx)];
    let mut out: Vec<Stx> = Vec::new();
    while let Some(task) = tasks.pop() {
        match task {
            Task::Visit(node) => {
                // 子级均匀命中可整体共享（免重建子树）
                if node.uniform_tag.as_ref() == Some(scopes) {
                    out.push(node.clone());
                    continue;
                }
                match &node.datum {
                    StxDatum::Symbol(s) => out.push(retag_leaf(StxDatum::Symbol(*s), node, scopes)),
                    StxDatum::Literal(l) => {
                        out.push(retag_leaf(StxDatum::Literal(l.clone()), node, scopes))
                    }
                    StxDatum::List(items) | StxDatum::Vector(items) => {
                        let n = items.len();
                        tasks.push(Task::Assemble {
                            input: node,
                            child_count: n,
                        });
                        for item in items.iter().rev() {
                            tasks.push(Task::Visit(item));
                        }
                    }
                }
            }
            Task::Assemble { input, child_count } => {
                // 子项按序位于 out 顶部（LIFO 处理顺序保证）
                let start = out.len() - child_count;
                let items: Vec<Stx> = out.drain(start..).collect();
                let datum = match &input.datum {
                    StxDatum::List(_) => StxDatum::List(std::rc::Rc::new(items)),
                    // 构造性默认：仅 List/Vector 两容器变体可到达本重建
                    // 路径（Assemble 上游已窄化）——保持原容器类别不变
                    _ => StxDatum::Vector(std::rc::Rc::new(items)),
                };
                out.push(retag_leaf(datum, input, scopes));
            }
        }
    }
    out.pop().expect("根重建产物不变式")
}

/// 叶/组装节点重建：datum 给定，scopes → target，span 提升展开代次，
/// phase 保持；**uniform_tag 置 Some(target)**（子树均匀性由构造保证：
/// 叶自身 / 组装的子项均为 target 标记节点或共享均匀子树）。
fn retag_leaf(datum: StxDatum, input: &Stx, scopes: &ScopeSet) -> Stx {
    Stx {
        datum,
        span: input.span.bumped_expansion(),
        scopes: scopes.clone(),
        phase: input.phase,
        uniform_tag: Some(scopes.clone()),
    }
}

/// 用户卫生宏：`(syntax-rules (literal...) (pattern template)...)`。
///
/// 模式语法：
/// - `_`：通配（匹配任意，不绑定）；
/// - `...`：省略号（列表尾部零或多次重复前导模式）；
/// - 符号：模式变量（绑定匹配片段），字面量集合中的符号除外（按字面匹配）；
/// - 字面量 datum（数字/字符串/布尔）：按相等匹配；
/// - 列表/向量：结构匹配。
///
/// 模板语法：
/// - 模式变量：替换为绑定值；
/// - `(x ...)`：省略号拼接绑定的序列；
/// - 其余标识符：**引入标识符** → α 重命名（卫生）；
/// - datum：原样复制。
#[derive(Debug, Clone)]
pub struct SyntaxRules {
    /// 字面量集合（按符号匹配，不作为模式变量）。
    literals: Vec<Symbol>,
    /// (模式, 模板) 子句序列（依序尝试）。
    clauses: Vec<(Stx, Stx)>,
    /// 定义处的模板快照作用域。
    def_scopes: ScopeSet,
}

impl SyntaxRules {
    /// 从 `(syntax-rules (lit...) (pat tpl)...)` 语法对象构造。
    /// 返回 (SyntaxRules, 错误列表中首个失败)。
    pub fn parse(form: &Stx) -> Result<SyntaxRules, String> {
        let items = form
            .datum
            .as_list()
            .ok_or_else(|| "syntax-rules 必须是列表".to_string())?;
        if items.len() < 2 {
            return Err("syntax-rules 缺少参数（至少需要字面量集合与一个子句）".to_string());
        }
        // 头必须是 syntax-rules 关键字（调用方已保证，此处防御性校验）
        if !matches!(items[0].datum, StxDatum::Symbol(_)) {
            return Err("syntax-rules 首元素必须是符号".to_string());
        }
        // 字面量集合
        let lits = match &items[1].datum {
            StxDatum::List(lits) => {
                let mut out = Vec::new();
                for l in lits.iter() {
                    match l.datum.as_symbol() {
                        Some(s) => out.push(s),
                        None => return Err("syntax-rules 字面量必须是符号".to_string()),
                    }
                }
                out
            }
            _ => return Err("syntax-rules 第二元素必须是字面量列表".to_string()),
        };
        // 子句
        let mut clauses = Vec::new();
        for clause in &items[2..] {
            let pair = clause
                .datum
                .as_list()
                .filter(|c| c.len() == 2)
                .ok_or_else(|| "syntax-rules 子句必须是 (pattern template) 二元列表".to_string())?;
            clauses.push((pair[0].clone(), pair[1].clone()));
        }
        if clauses.is_empty() {
            return Err("syntax-rules 至少需要一个子句".to_string());
        }
        Ok(SyntaxRules {
            literals: lits,
            clauses,
            def_scopes: form.scopes.clone(),
        })
    }

    /// 应用：逐子句匹配 → 命中后模板展开。
    /// 入参 `args` 为宏调用形式中除宏名外的全部实参。
    pub fn apply(&self, args: &[Stx], hygiene: &mut HygieneCtx) -> Result<Stx, String> {
        for (pattern, template) in &self.clauses {
            let mut bindings = Vec::new();
            // 模式 (macro-name . rest) 与调用 (args...) 对齐：
            // 模式首元素匹配宏名占位（按约定为 `_` 或符号）
            if let Some(matched) = self.match_clause(pattern, args, &mut bindings) {
                if matched {
                    return self.instantiate(template, &bindings, hygiene);
                }
            }
        }
        Err("宏调用与全部子句模式均不匹配".to_string())
    }

    /// 定义处作用域集（卫生并集成分；Transformer::apply 消费）。
    pub fn definition_scopes(&self) -> &ScopeSet {
        &self.def_scopes
    }

    /// 子句匹配：模式（含宏名首元素）对实参序列。
    fn match_clause(
        &self,
        pattern: &Stx,
        args: &[Stx],
        bindings: &mut Vec<(Symbol, BoundValue)>,
    ) -> Option<bool> {
        // 展平模式为「序列模式」：列表模式 (p0 p1 ... rest...) 依次对 args
        let pats = pattern.datum.as_list()?;
        // 首元素是宏名（或 `_`）：跳过
        let arg_pats = &pats[1.min(pats.len())..];
        match match_sequence(arg_pats, args, &self.literals, bindings) {
            true => Some(true),
            false => None,
        }
    }

    /// 模板实例化：替换模式变量 + 省略号拼接 + 引入标识符 α 重命名。
    fn instantiate(
        &self,
        template: &Stx,
        bindings: &[(Symbol, BoundValue)],
        hygiene: &mut HygieneCtx,
    ) -> Result<Stx, String> {
        hygiene.begin_instantiation();
        Ok(instantiate_template(template, bindings, hygiene, None))
    }
}

/// 匹配绑定的值形态。
#[derive(Debug, Clone)]
enum BoundValue {
    /// 单个匹配片段。
    Single(Stx),
    /// 省略号展开的序列（零或多个）。
    Sequence(Vec<Stx>),
}

/// 序列匹配：模式序列（含尾部 `...`）对实参序列。
fn match_sequence(
    pats: &[Stx],
    args: &[Stx],
    literals: &[Symbol],
    bindings: &mut Vec<(Symbol, BoundValue)>,
) -> bool {
    // 识别尾部省略号：`(p ...)` 形式（p 为倒数第二个元素，最后一个是 `...`）
    let has_ellipsis = pats.len() >= 2
        && pats[pats.len() - 1]
            .datum
            .as_symbol()
            .map(is_ellipsis)
            .unwrap_or(false);
    let (fixed_pats, ellipsis_pat) = if has_ellipsis {
        (&pats[..pats.len() - 2], Some(&pats[pats.len() - 2]))
    } else {
        (pats, None)
    };
    if args.len() < fixed_pats.len() {
        return false;
    }
    // 固定段
    for (p, a) in fixed_pats.iter().zip(args.iter()) {
        if !match_datum(p, a, literals, bindings) {
            return false;
        }
    }
    // 省略号段
    if let Some(ep) = ellipsis_pat {
        let rest = &args[fixed_pats.len()..];
        // 省略号模式变量绑定序列
        if let Some(var) = ep.datum.as_symbol() {
            let seq: Vec<Stx> = rest.to_vec();
            bindings.retain(|(n, _)| *n != var);
            bindings.push((var, BoundValue::Sequence(seq)));
            return true;
        }
        // 复合省略号模式（如 (a b ...）结构）：对每个 rest 元素结构匹配
        for r in rest {
            let mut inner = bindings.clone();
            if !match_datum(ep, r, literals, &mut inner) {
                return false;
            }
            // 合并 inner 绑定（简化：仅支持单层复合——Stage 0 骨架边界）
            *bindings = inner;
        }
        true
    } else {
        args.len() == fixed_pats.len()
    }
}

/// 单元素匹配：模式 datum 对实参 datum。
fn match_datum(
    pat: &Stx,
    arg: &Stx,
    literals: &[Symbol],
    bindings: &mut Vec<(Symbol, BoundValue)>,
) -> bool {
    match &pat.datum {
        StxDatum::Symbol(s) => {
            if is_ellipsis(*s) {
                return false; // 省略号只在序列位置合法
            }
            if is_wildcard(*s) {
                return true; // `_` 通配
            }
            if literals.contains(s) {
                // 字面量：符号相等才匹配
                matches!(&arg.datum, StxDatum::Symbol(a) if a == s)
            } else {
                // 模式变量：绑定
                bindings.retain(|(n, _)| *n != *s);
                bindings.push((*s, BoundValue::Single(arg.clone())));
                true
            }
        }
        StxDatum::Literal(l) => matches!(&arg.datum, StxDatum::Literal(a) if a == l),
        StxDatum::List(pats) => {
            if let Some(inner) = arg.datum.as_list() {
                let mut fresh = bindings.clone();
                if match_sequence(pats, inner, literals, &mut fresh) {
                    *bindings = fresh;
                    return true;
                }
            }
            false
        }
        StxDatum::Vector(pats) => {
            if let StxDatum::Vector(inner) = &arg.datum {
                let mut fresh = bindings.clone();
                if match_sequence(pats, inner, literals, &mut fresh) {
                    *bindings = fresh;
                    return true;
                }
            }
            false
        }
    }
}

/// 模板实例化：`depth` = 省略号拼接的当前嵌套深度（None = 顶层）。
fn instantiate_template(
    template: &Stx,
    bindings: &[(Symbol, BoundValue)],
    hygiene: &mut HygieneCtx,
    seq_index: Option<usize>,
) -> Stx {
    match &template.datum {
        StxDatum::Symbol(s) => {
            // 模式变量替换（保留用户符号与 Span——引用用户绑定）
            if let Some((_, BoundValue::Single(v))) = bindings.iter().find(|(n, _)| n == s) {
                return v.clone();
            }
            if let Some((_, BoundValue::Sequence(seq))) = bindings.iter().find(|(n, _)| n == s) {
                // 序列变量在非省略号位置：取当前索引元素（拼接上下文）
                if let Some(idx) = seq_index {
                    if let Some(v) = seq.get(idx) {
                        return v.clone();
                    }
                }
                // 越界或无索引：保留为符号（展开器将按未绑定处理）
                return template.clone();
            }
            // 引入标识符：α 重命名（卫生核心）。
            // 例外：保留集（宏自引用）与核心形式关键字（语法分派词，非引用）。
            if hygiene.is_preserved(*s) || is_core_keyword(*s, hygiene) {
                return template.clone();
            }
            let fresh = hygiene.fresh_symbol(*s);
            Stx {
                datum: StxDatum::Symbol(fresh),
                span: template.span,
                scopes: template.scopes.clone(),
                phase: template.phase,
                uniform_tag: None,
            }
        }
        StxDatum::Literal(l) => Stx {
            datum: StxDatum::Literal(l.clone()),
            span: template.span,
            scopes: template.scopes.clone(),
            phase: template.phase,
            uniform_tag: None,
        },
        StxDatum::List(items) | StxDatum::Vector(items) => {
            let mut out: Vec<Stx> = Vec::new();
            let is_vector = matches!(template.datum, StxDatum::Vector(_));
            let mut i = 0;
            while i < items.len() {
                let item = &items[i];
                // 检测 `(var ...)` 省略号拼接
                let next_is_ellipsis = items
                    .get(i + 1)
                    .and_then(|n| n.datum.as_symbol())
                    .map(is_ellipsis)
                    .unwrap_or(false);
                if next_is_ellipsis {
                    if let Some(var) = item.datum.as_symbol() {
                        if let Some((_, BoundValue::Sequence(seq))) =
                            bindings.iter().find(|(n, _)| *n == var)
                        {
                            for v in seq {
                                out.push(v.clone());
                            }
                            i += 2;
                            continue;
                        }
                    }
                    // 非序列绑定的省略号：报错优于静默——此处保留原样
                    // 由展开器对未知符号统一报「未绑定」错误
                }
                out.push(instantiate_template(item, bindings, hygiene, seq_index));
                i += 1;
            }
            let span = out
                .first()
                .map(|f| {
                    let mut sp = f.span;
                    for x in &out {
                        sp = sp.merge(x.span);
                    }
                    sp
                })
                .unwrap_or(template.span);
            let datum = if is_vector {
                StxDatum::Vector(std::rc::Rc::new(out))
            } else {
                StxDatum::List(std::rc::Rc::new(out))
            };
            Stx {
                datum,
                span,
                scopes: template.scopes.clone(),
                phase: template.phase,
                uniform_tag: None,
            }
        }
    }
}

/// 核心形式关键字判定（经当前符号表名查询；关键字不参与卫生重命名）。
fn is_core_keyword(s: Symbol, hygiene: &HygieneCtx) -> bool {
    let name = hygiene.peek_name(s);
    Keyword::from_name(name).is_some()
}

/// `...` 符号判定（经符号表名查询）。
fn is_ellipsis(s: Symbol) -> bool {
    // 省略号符号在词法层以标识符 "..." intern（SymbolTable 保证名字唯一性）。
    // 此处以符号名比较（局部 table 不可用时用句柄一致性的保守近似：
    // 展开器构造 HygieneCtx 前已确保 "..." 的唯一 intern）。
    ELLIPSIS.with(|e| *e.borrow() == s)
}

thread_local! {
    static ELLIPSIS: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
}

/// 登记省略号符号句柄（展开器初始化时调用一次）。
pub fn register_ellipsis(table: &mut SymbolTable) {
    let sym = table.intern("...");
    ELLIPSIS.with(|e| *e.borrow_mut() = sym);
}

/// `_` 通配符判定。
fn is_wildcard(s: Symbol) -> bool {
    WILDCARD.with(|w| *w.borrow() == s)
}

thread_local! {
    static WILDCARD: std::cell::RefCell<Symbol> = const { std::cell::RefCell::new(Symbol(u32::MAX)) };
}

/// 登记通配符符号句柄（展开器初始化时调用一次）。
pub fn register_wildcard(table: &mut SymbolTable) {
    let sym = table.intern("_");
    WILDCARD.with(|w| *w.borrow_mut() = sym);
}

/// 内置变换器集合的注册入口（expander 使用）。
pub struct BuiltinTransformers;

impl BuiltinTransformers {
    /// 初始化标记符号（ellipsis / wildcard 的唯一 intern）。
    pub fn init_markers(table: &mut SymbolTable) {
        register_ellipsis(table);
        register_wildcard(table);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kerf_span::Span;

    fn setup() -> (SymbolTable, Vec<Symbol>) {
        let mut t = SymbolTable::new();
        let syms = vec![
            t.intern("swap"),
            t.intern("let"),
            t.intern("x"),
            t.intern("y"),
            t.intern("tmp"),
            t.intern("do"),
            t.intern("assign"),
            t.intern("quote"),
        ];
        BuiltinTransformers::init_markers(&mut t);
        (t, syms)
    }

    fn sym_stx(s: Symbol) -> Stx {
        Stx::symbol(s, Span::dummy(), ScopeSet::new())
    }

    #[test]
    fn syntax_rules_parse_and_simple_match() {
        let (mut t, syms) = setup();
        // (syntax-rules () (swap a b) (do (assign tmp a) (assign a b) (assign b tmp)))
        // 构造模式与模板
        let a = t.intern("a");
        let b = t.intern("b");
        let pattern = Stx::list(
            vec![sym_stx(syms[0]), sym_stx(a), sym_stx(b)],
            Span::dummy(),
            ScopeSet::new(),
        );
        let template = Stx::list(
            vec![
                sym_stx(syms[5]), // do
                Stx::list(
                    vec![sym_stx(syms[6]), sym_stx(syms[5]), sym_stx(a)],
                    Span::dummy(),
                    ScopeSet::new(),
                ),
            ],
            Span::dummy(),
            ScopeSet::new(),
        );
        // 直接构造 SyntaxRules 字段（parse 路径由 expander 集成测试覆盖）
        let rules = SyntaxRules {
            literals: vec![],
            clauses: vec![(pattern, template)],
            def_scopes: ScopeSet::new(),
        };
        // 调用 (swap 1 2)
        let call_args = vec![
            Stx::literal(
                kerf_syntax::StxLiteral::Int(1),
                Span::dummy(),
                ScopeSet::new(),
            ),
            Stx::literal(
                kerf_syntax::StxLiteral::Int(2),
                Span::dummy(),
                ScopeSet::new(),
            ),
        ];
        let mut hygiene = HygieneCtx::new(&mut t);
        let out = rules.apply(&call_args, &mut hygiene).unwrap();
        // do 保留；模式变量 a 替换为 1
        let rendered = out.render(&|s| t.name(s).to_string());
        assert!(rendered.contains("do"));
        assert!(rendered.contains("assign"));
    }

    #[test]
    fn hygiene_renames_introduced_identifiers() {
        let (mut t, syms) = setup();
        // 模板 (assign tmp x)：tmp 为引入标识符（非模式变量）→ 重命名
        let x = t.intern("x");
        let pattern = Stx::list(
            vec![sym_stx(syms[0]), sym_stx(x)],
            Span::dummy(),
            ScopeSet::new(),
        );
        let template = Stx::list(
            vec![
                sym_stx(syms[6]),
                sym_stx(syms[4]), /* tmp */
                sym_stx(x),
            ],
            Span::dummy(),
            ScopeSet::new(),
        );
        let rules = SyntaxRules {
            literals: vec![],
            clauses: vec![(pattern, template)],
            def_scopes: ScopeSet::new(),
        };
        let call_args = vec![Stx::literal(
            kerf_syntax::StxLiteral::Int(42),
            Span::dummy(),
            ScopeSet::new(),
        )];
        let mut hygiene = HygieneCtx::new(&mut t);
        let out = rules.apply(&call_args, &mut hygiene).unwrap();
        assert!(hygiene.renames() >= 1, "引入标识符必须被重命名");
        let rendered = out.render(&|s| t.name(s).to_string());
        assert!(
            rendered.contains("tmp$hyg$"),
            "引入标识符应重命名为唯一符号：{}",
            rendered
        );
    }

    #[test]
    fn ellipsis_matches_zero_or_more() {
        let (mut t, _syms) = setup();
        let my_or = t.intern("my-or");
        let a = t.intern("a");
        let rest = t.intern("rest");
        // 模式 (my-or a rest ...)
        let pattern = Stx::list(
            vec![
                sym_stx(my_or),
                sym_stx(a),
                sym_stx(rest),
                sym_stx(ellipsis_sym(&t)),
            ],
            Span::dummy(),
            ScopeSet::new(),
        );
        // 模板 (do a rest ...)
        let template = Stx::list(
            vec![
                sym_stx(t.keyword_symbol(kerf_syntax::Keyword::Begin)),
                sym_stx(a),
                sym_stx(rest),
                sym_stx(ellipsis_sym(&t)),
            ],
            Span::dummy(),
            ScopeSet::new(),
        );
        let rules = SyntaxRules {
            literals: vec![],
            clauses: vec![(pattern, template)],
            def_scopes: ScopeSet::new(),
        };
        // 调用 (my-or 1 2 3) → do 1 2 3
        let args: Vec<Stx> = [1i64, 2, 3]
            .iter()
            .map(|v| {
                Stx::literal(
                    kerf_syntax::StxLiteral::Int(*v),
                    Span::dummy(),
                    ScopeSet::new(),
                )
            })
            .collect();
        let mut hygiene = HygieneCtx::new(&mut t);
        let out = rules.apply(&args, &mut hygiene).unwrap();
        let rendered = out.render(&|s| t.name(s).to_string());
        assert_eq!(rendered, "(do 1 2 3)");
        // 零省略段：(my-or 1) → do 1
        let one = vec![Stx::literal(
            kerf_syntax::StxLiteral::Int(1),
            Span::dummy(),
            ScopeSet::new(),
        )];
        let out0 = rules.apply(&one, &mut HygieneCtx::new(&mut t)).unwrap();
        assert_eq!(out0.render(&|s| t.name(s).to_string()), "(do 1)");
    }

    fn ellipsis_sym(_t: &SymbolTable) -> Symbol {
        ELLIPSIS.with(|e| *e.borrow())
    }
}
