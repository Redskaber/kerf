//! AnnotatedANF IR 实化与 CoreExpr lowering（批次 G / 38-b——G1 前段）。
//!
//! **定位**：`CoreExpr` → `AnnotatedANF`（块式 ANF：绑定序列 + 块图控制
//! 流）。ANF = A-Normal Form——全部中间值具名（临时绑定），函数调用
//! 与原语参数原子化。块式变体天然对齐 QBE 的 function/label/jmp 拓扑
//! （值上下文 if 经块参数 merge——QBE phi 的显式等价物）。
//!
//! **PoC 边界裁定（B1 登记——批次 G 范围声明）**：
//! - 支持：整数域字面量 / Var（函数形参）/ 算术与比较原语十项
//!   （`+ - * / mod = < > <= >=`）+ `not` + `eq`（整数域视作 `=`）/
//!   Apply（被调者为顶层 `define` 函数的直接调用，含静态 arity 校验）/
//!   If（含值上下文——块参数 merge）/ Do / 顶层 `define`（值必须为
//!   Fn）/ Require（零运行时语义——跳过，与 VM 字节码口径一致）；
//! - 边界外（明确错误，非静默降级）：Fn 出现在值位置（闭包捕获未
//!   进 PoC——语言面全闭包语义由批次 H/I 的 GC-后端协同承接）/
//!   Float/Str/Symbol/Pair/Bool/Nil 字面量 / `set!` / `module` /
//!   未定义函数调用 / arity 不匹配 / 函数值一等传递。
//!
//! **名称解析口径（B1 偏差登记）**：lowering 采用名称基词法栈（不做
//! ScopeSet 子集匹配）——良构程序上与编译器名称基栈序同解（r13 收口
//! 注记的既定等价条件）；TD-004 全量匹配留给字节码路径（本 crate 不
//! 承接双路径一致性——VM 是语义参照，本地码是 PoC 交付面）。

use std::collections::HashMap;
use std::rc::Rc;

use kerf_core::{CoreExpr, LiteralValue};
use kerf_span::Span;
use kerf_syntax::{Symbol, SymbolTable};

// ---------------------------------------------------------------------------
// ANF IR 数据结构（实化 AnnotatedANF 的函数定义集）
// ---------------------------------------------------------------------------

/// 注解 ANF 程序（契约实化家在 [`crate::codegen`]——`fingerprint` 保留）。
///
/// 惯例：`funcs[0]` 为 main 入口（无参数，返回值即进程退出码——C `main`
/// 语义，POSIX 取低 8 位）；其余为顶层 `define` 函数（QBE 符号
/// `$name`，基名为卫生后缀剥离后的符号名）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnnotatedANF {
    pub funcs: Vec<AFuncDef>,
    pub fingerprint: u64,
}

/// ANF 函数定义（`define name (lambda (params...) body)` 的降落产物）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AFuncDef {
    /// QBE 全局符号基名（卫生后缀已剥离）。
    pub name: String,
    /// 形参临时（`%tN` 序列的命名锚——N 从 0 起，函数内唯一）。
    pub params: Vec<ATemp>,
    /// 块图（blocks[0] = entry）。
    pub body: ABody,
}

/// 块图函数体。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ABody {
    pub blocks: Vec<ABlock>,
}

/// 基本块：phi 绑定（值上下文 if 的 merge 锚——QBE `phi @lbl %v, ...`
/// 形态）+ 绑定序列 + 控制流终结。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ABlock {
    /// phi 绑定（必须位于块首——QBE 词法约束的 IR 侧对应）。
    pub phis: Vec<APhi>,
    /// 绑定序列（顺序执行）。
    pub stmts: Vec<AStmt>,
    /// 控制流终结（块必以 Ret/Br/Jmp 之一收尾——强不变式）。
    pub ctrl: ACtrl,
}

/// phi 绑定：dst = phi (来源标签, 来源原子)...
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct APhi {
    pub dst: ATemp,
    /// 来源（来源块标签 → 该路径携带的值）。
    pub sources: Vec<(ALabel, AAtom)>,
}

/// 绑定语句：`dst = op(args)`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AStmt {
    pub dst: ATemp,
    pub op: AOp,
    pub span: Span,
}

/// 右侧操作（原子参数的原语或直接调用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AOp {
    /// 原语（整数域十项 + not/eq?——见 [`APrim`]）。
    Prim(APrim, Vec<AAtom>),
    /// 直接函数调用（被调者为顶层 define 的全局函数）。
    Call { func: String, args: Vec<AAtom> },
}

/// 原语集（PoC 整数域——QBE 指令的直接映射）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum APrim {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Lt,
    Gt,
    Le,
    Ge,
    Not,
}

impl APrim {
    /// kerf 内置名 → 原语（PoC 支持面判定）。
    pub fn from_builtin_name(name: &str) -> Option<APrim> {
        Some(match name {
            "+" => APrim::Add,
            "-" => APrim::Sub,
            "*" => APrim::Mul,
            "/" => APrim::Div,
            "mod" => APrim::Mod,
            "=" => APrim::Eq,
            "<" => APrim::Lt,
            ">" => APrim::Gt,
            "<=" => APrim::Le,
            ">=" => APrim::Ge,
            "not" => APrim::Not,
            "eq" => APrim::Eq,
            _ => return None,
        })
    }
}

/// 原子（ANF 叶：字面量或临时引用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AAtom {
    Int(i64),
    Var(ATemp),
}

/// 临时编号（函数内唯一；渲染 `%t{0}`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ATemp(pub u32);

/// 块标签（函数内唯一；渲染 `@l{0}`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ALabel(pub u32);

/// 控制流终结指令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ACtrl {
    /// 返回原子。
    Ret(AAtom),
    /// 条件分支（cond 为布尔临时；两分支均不带参数——值合并走 Jmp）。
    Br { cond: ATemp, t: ALabel, f: ALabel },
    /// 无条件跳转（值合并经目标块的 [`APhi`] 承载）。
    Jmp { target: ALabel },
}

impl Default for ACtrl {
    /// 缺省终结：返回 0（C main 惯例——ABlock/ABody/AFuncDef 的
    /// Default 派生链需要）。
    fn default() -> Self {
        ACtrl::Ret(AAtom::Int(0))
    }
}

// ---------------------------------------------------------------------------
// 错误（lowering 边界外 = 显式失败，非静默降级——§2.3-4）
// ---------------------------------------------------------------------------

/// lowering 错误（PoC 边界外与静态校验失败统一面）。
#[derive(Debug, Clone, PartialEq)]
pub struct LowerError {
    pub message: String,
    pub span: Span,
}

impl LowerError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        LowerError {
            message: message.into(),
            span,
        }
    }
}

/// 原语 arity（Not/Eq 等）——静态校验用。
fn prim_arity(p: APrim) -> (usize, usize) {
    match p {
        APrim::Not => (1, 1),
        // _ 臂理由：其余原语均为二元（算术/比较/cons 族）——静态校验默认域
        _ => (2, 2),
    }
}

// ---------------------------------------------------------------------------
// Lowering（§10.1 规则 2：Ctxt 后缀）
// ---------------------------------------------------------------------------

/// 顶层全局函数登记（两遍扫描的第一遍产物）。
#[derive(Debug, Default)]
struct GlobalFuncs {
    /// Symbol → QBE 基名。
    names: HashMap<Symbol, String>,
    /// Symbol → 形参数（arity 静态校验）。
    arities: HashMap<Symbol, usize>,
}

/// 函数内 lowering 上下文（块构建器 + 名称基解析）。
struct LowerCtxt<'a> {
    /// 顶层全局函数表（只读）。
    globals: &'a GlobalFuncs,
    /// 符号名渲染表（Symbol → 基名——卫生后缀剥离在 [`render_name`]）。
    table: &'a SymbolTable,
    /// 词法形参栈（名称基：符号 → 当前帧临时）。
    locals: Vec<HashMap<Symbol, ATemp>>,
    /// 已发射块（含 phis/stmts；ctrl 由 [`seal_block`] 填）。
    blocks: Vec<(Vec<APhi>, Vec<AStmt>)>,
    /// 块终结指令队列（与 blocks 对位；seal 时合入）。
    ctrls: Vec<Option<ACtrl>>,
    /// 下一临时号。
    next_temp: u32,
}

impl<'a> LowerCtxt<'a> {
    fn new(globals: &'a GlobalFuncs, table: &'a SymbolTable, params: &[Symbol]) -> Self {
        let mut locals = vec![HashMap::new()];
        for (i, p) in params.iter().enumerate() {
            locals[0].insert(*p, ATemp(i as u32));
        }
        LowerCtxt {
            globals,
            table,
            locals,
            blocks: Vec::new(),
            ctrls: Vec::new(),
            next_temp: params.len() as u32,
        }
    }

    fn fresh_temp(&mut self) -> ATemp {
        let t = ATemp(self.next_temp);
        self.next_temp += 1;
        t
    }

    /// 开新块（可带 phi 绑定集）——**块标签 = 块索引**（单一编号系：
    /// gen 侧以 blocks[i] ↔ @l{i} 渲染；Br/Jmp 目标在 seal 前以
    /// `blocks.len()` 预测未来块索引——见 lower_tail/lower_value_if）。
    fn start_block(&mut self, phis: Vec<APhi>) -> ALabel {
        let label = ALabel(self.blocks.len() as u32);
        self.blocks.push((phis, Vec::new()));
        self.ctrls.push(None);
        label
    }

    /// 封块（控制流终结填入——强不变式：一坑一终结，§2.3-4 显式失败）。
    fn seal_block(&mut self, ctrl: ACtrl) {
        let idx = self.blocks.len() - 1;
        if self.ctrls[idx].is_some() {
            panic!("lowering 内部不变式破坏：块 {} 被二次终结", idx);
        }
        self.ctrls[idx] = Some(ctrl);
    }

    /// 绑定名查找（名称基词法栈——自顶向下）。
    fn lookup(&self, name: Symbol) -> Option<ATemp> {
        for frame in self.locals.iter().rev() {
            if let Some(&t) = frame.get(&name) {
                return Some(t);
            }
        }
        None
    }

    /// 发射绑定语句。
    fn emit(&mut self, op: AOp, span: Span) -> ATemp {
        let dst = self.fresh_temp();
        let idx = self.blocks.len() - 1;
        self.blocks[idx].1.push(AStmt { dst, op, span });
        dst
    }
}

/// 符号基名渲染（卫生后缀 `$hyg$N` 剥离——与编译器 LOAD_GLOBAL 回退
/// 口径一致）+ QBE 符号字符集净化（`[A-Za-z0-9_.]` 外替换 `_`）。
fn render_name(sym: Symbol, table: &SymbolTable) -> String {
    let raw = table.name(sym);
    let base = match raw.find("$hyg$") {
        Some(i) => &raw[..i],
        None => raw,
    };
    let sanitized: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "anon".to_string()
    } else {
        sanitized
    }
}

/// 原子化复杂度判定的尾表达式上下文（块终结前的位置感知）。
enum TailCtx {
    /// 值上下文（merge/jmp/ret 的载荷）。
    Value,
    /// 函数体尾（直接 Ret）。
    BodyTail,
}

/// Lowering 主入口（§10.1 规则 1：`<verb>_<noun>` 自由函数）。
///
/// 输入：顶层 CoreExpr 序列（expander 产物——与 `compile_module` 同源）。
/// 输出：实化 `AnnotatedANF`（`funcs[0]` = main）。
/// 内容寻址：`fingerprint` 对 IL 文本做 std DefaultHasher（缓存键口径
/// 与契约字段语义一致——内容寻址，非密码学）。
pub fn lower_program(
    exprs: &[Rc<CoreExpr>],
    table: &SymbolTable,
) -> Result<AnnotatedANF, LowerError> {
    // 第一遍：全局函数登记（define → (Symbol, 基名, arity)）
    let mut globals = GlobalFuncs::default();
    for e in exprs {
        if let CoreExpr::Define { name, value, .. } = e.as_ref() {
            let arity = lambda_arity(value).ok_or_else(|| {
                LowerError::new(
                    "本地码 PoC 边界：define 的值必须是函数（lambda）——\
                     非函数全局（数据/闭包值）未进 PoC",
                    e.span(),
                )
            })?;
            globals.names.insert(*name, render_name(*name, table));
            globals.arities.insert(*name, arity);
        }
    }

    // 第二遍：逐函数 lower + main 语句序列
    let mut funcs: Vec<AFuncDef> = Vec::new();
    for e in exprs {
        match e.as_ref() {
            CoreExpr::Define { name, value, .. } => {
                if let CoreExpr::Fn {
                    params, body, span, ..
                } = value.as_ref()
                {
                    let afunc = lower_lambda(
                        &globals,
                        table,
                        &render_name(*name, table),
                        params,
                        body,
                        *span,
                    )?;
                    funcs.push(afunc);
                } else {
                    // 第一遍已拒绝（此分支不可达——防御性，§2.3-4）
                    return Err(LowerError::new(
                        "define 值非 lambda（第一遍应已拒绝）",
                        e.span(),
                    ));
                }
            }
            CoreExpr::Require { .. } => {
                // 零运行时语义（与字节码路径口径一致——不产指令）
            }
            _ => {
                // 非定义顶层表达式 → main 序列（延迟到 main 构建）
            }
        }
    }

    // main：全部非 define/require 顶层表达式依序求值（中间值丢弃），
    // 最后一个的值作为 main 返回（空序列 → 返回 0——C main 惯例）。
    let main = lower_main(&globals, table, exprs)?;
    funcs.insert(0, main);

    let fingerprint = fingerprint_of(&funcs);
    Ok(AnnotatedANF { funcs, fingerprint })
}

/// define 值的形参数（非 lambda → None）。
fn lambda_arity(value: &CoreExpr) -> Option<usize> {
    match value {
        CoreExpr::Fn { params, .. } => Some(params.len()),
        _ => None,
    }
}

/// 内容寻址指纹（std DefaultHasher——缓存键口径）。
fn fingerprint_of(funcs: &[AFuncDef]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for f in funcs {
        f.name.hash(&mut h);
        f.params.len().hash(&mut h);
        for b in &f.body.blocks {
            // 内容寻址完备性（38-c 实测勘误：仅长度 hash 使 (+ 1 2) 与
            // (+ 1 3) 同指纹——操作数/phi 来源/终结载荷全量进 hash）
            format!("{:?}", b.phis).hash(&mut h);
            format!("{:?}", b.stmts).hash(&mut h);
            format!("{:?}", b.ctrl).hash(&mut h);
        }
    }
    h.finish()
}

/// lower 一条 lambda（define 值）。
fn lower_lambda(
    globals: &GlobalFuncs,
    table: &SymbolTable,
    name: &str,
    params: &[Symbol],
    body: &Rc<CoreExpr>,
    _span: Span,
) -> Result<AFuncDef, LowerError> {
    let mut ctx = LowerCtxt::new(globals, table, params);
    ctx.start_block(Vec::new()); // entry（label 0）
                                 // BodyTail 契约：lower_tail 自保证全部块终结（尾位 if 双分支各自
                                 // Ret；其余形状在尾原子化后即地 Ret）——调用方不再封尾
    lower_tail(&mut ctx, body, TailCtx::BodyTail)?;
    finish_func(name, params, ctx)
}

/// lower main（顶层语句序列）。
fn lower_main(
    globals: &GlobalFuncs,
    table: &SymbolTable,
    exprs: &[Rc<CoreExpr>],
) -> Result<AFuncDef, LowerError> {
    let stmts: Vec<&Rc<CoreExpr>> = exprs
        .iter()
        .filter(|e| {
            !matches!(
                e.as_ref(),
                CoreExpr::Define { .. } | CoreExpr::Require { .. }
            )
        })
        .collect();

    let mut ctx = LowerCtxt::new(globals, table, &[]);
    ctx.start_block(Vec::new()); // entry
    if stmts.is_empty() {
        // 空程序：main 返回 0（C 惯例）
        ctx.seal_block(ACtrl::Ret(AAtom::Int(0)));
    } else {
        for (i, e) in stmts.iter().enumerate() {
            let atom = lower_tail(&mut ctx, e, TailCtx::Value)?;
            if i + 1 == stmts.len() {
                ctx.seal_block(ACtrl::Ret(atom));
            }
            // 中间值丢弃：原子结果直接不复用（无副作用资源需清理——
            // 整数域无析构）
        }
    }
    finish_func("main", &[], ctx)
}

/// 收块图为函数定义（终结完备性校验——§19.3 不变式 2 同口径）。
fn finish_func(name: &str, params: &[Symbol], ctx: LowerCtxt<'_>) -> Result<AFuncDef, LowerError> {
    let mut blocks = Vec::with_capacity(ctx.blocks.len());
    for (i, (phis, stmts)) in ctx.blocks.into_iter().enumerate() {
        let ctrl = ctx.ctrls[i].clone().ok_or_else(|| {
            LowerError::new(
                format!("lowering 不变式破坏：块 {} 缺控制流终结", i),
                Span::default(),
            )
        })?;
        blocks.push(ABlock { phis, stmts, ctrl });
    }
    Ok(AFuncDef {
        name: name.to_string(),
        params: (0..params.len()).map(|i| ATemp(i as u32)).collect(),
        body: ABody { blocks },
    })
}

/// 原子化：复杂表达式求值为临时；原子直接返回。
fn lower_atom(ctx: &mut LowerCtxt<'_>, e: &CoreExpr) -> Result<AAtom, LowerError> {
    match e {
        CoreExpr::Literal { value, .. } => match value {
            LiteralValue::Int(v) => Ok(AAtom::Int(*v)),
            LiteralValue::Float(_)
            | LiteralValue::Str(_)
            | LiteralValue::Bool(_)
            | LiteralValue::Nil
            | LiteralValue::Symbol(_)
            | LiteralValue::Pair(_, _) => Err(LowerError::new(
                "本地码 PoC 边界：仅支持整数域字面量（Float/Str/Bool/Nil/Symbol/Pair 未进 PoC）",
                e.span(),
            )),
        },
        CoreExpr::Var { name, .. } => {
            if let Some(t) = ctx.lookup(*name) {
                Ok(AAtom::Var(t))
            } else if ctx.globals.names.contains_key(name) {
                Err(LowerError::new(
                    "本地码 PoC 边界：函数值一等传递未支持——\
                     函数名仅可作为调用目标",
                    e.span(),
                ))
            } else {
                // 未绑定且非全局函数——builtin 原语不可作值（同口径）
                Err(LowerError::new(
                    "本地码 PoC 边界：自由变量（未绑定到形参）未支持",
                    e.span(),
                ))
            }
        }
        CoreExpr::If { .. } => {
            // 值上下文 if：原子化到临时（块参数 merge）
            let t = lower_value_if(ctx, e)?;
            Ok(AAtom::Var(t))
        }
        CoreExpr::Apply { .. } => {
            let t = lower_call(ctx, e)?;
            Ok(AAtom::Var(t))
        }
        CoreExpr::Do { .. } => {
            let t = lower_begin_value(ctx, e)?;
            Ok(AAtom::Var(t))
        }
        CoreExpr::Fn { span, .. } => Err(LowerError::new(
            "本地码 PoC 边界：lambda 出现在值位置（闭包捕获未进 PoC——\
             语言面闭包语义由后续批次的 GC-后端协同承接）",
            *span,
        )),
        CoreExpr::Assign { span, .. } => Err(LowerError::new(
            "本地码 PoC 边界：assign（可变赋值）未进 PoC",
            *span,
        )),
        CoreExpr::Define { span, .. } => Err(LowerError::new(
            "本地码 PoC 边界：define 仅在模块顶层（函数体内 define 由展开器改写——\
             若见此错误说明输入未经展开器规范化）",
            *span,
        )),
        CoreExpr::Module { span, .. } => Err(LowerError::new(
            "本地码 PoC 边界：module 形式未进 PoC",
            *span,
        )),
        CoreExpr::Require { span, .. } => Err(LowerError::new(
            "本地码 PoC 边界：require 无值语义（出现在表达式位置非法）",
            *span,
        )),
        // r25/42-f：效应形式在 native 路径显式拒绝（PoC 边界 B1 同型
        // 口径——require+print 先例；效应语义由 VM 路径承载，native
        // 效应化属 Stage 3 后端演进评估项）
        CoreExpr::Perform { span, .. } | CoreExpr::Handle { span, .. } => Err(LowerError::new(
            "本地码 PoC 边界：效应形式（perform/handle）未进 PoC——效应语义由 VM 路径承载",
            *span,
        )),
    }
}

/// 尾表达式 lowering（Value=原子化；BodyTail=直接作 Ret 载荷）。
fn lower_tail(ctx: &mut LowerCtxt<'_>, e: &CoreExpr, tctx: TailCtx) -> Result<AAtom, LowerError> {
    match tctx {
        TailCtx::BodyTail => {
            // 尾位 if：直接结构化（then/else 各自 Ret——无需 merge）
            if let CoreExpr::If {
                cond,
                then_branch,
                else_branch,
                span,
            } = e
            {
                let cond_atom = lower_bool(ctx, cond)?;
                let cond_t = ensure_temp(ctx, cond_atom, *span)?;
                // then 目标可即定（下一块）；else 目标须回填——then 分支
                // 内嵌 if 会分裂多块，seal 时不可预测（跳转回填，§19.3
                // 不变式 2 同型：占位 u32::MAX，分支 lower 完成后回填）
                let br_idx = ctx.blocks.len() - 1;
                let then_label = ALabel(ctx.blocks.len() as u32);
                ctx.seal_block(ACtrl::Br {
                    cond: cond_t,
                    t: then_label,
                    f: ALabel(u32::MAX),
                });
                // then 块：BodyTail 契约自终结（分支项为 if 时递归分裂）
                ctx.start_block(Vec::new());
                lower_tail(ctx, then_branch, TailCtx::BodyTail)?;
                // else 块索引 = then 链落定后的当前长度（回填）
                let else_label = ALabel(ctx.blocks.len() as u32);
                if let Some(ACtrl::Br { f, .. }) = ctx.ctrls[br_idx].as_mut() {
                    *f = else_label;
                }
                ctx.start_block(Vec::new());
                lower_tail(ctx, else_branch, TailCtx::BodyTail)?;
                Ok(AAtom::Int(0)) // 不可达载荷（两分支均已终结）
            } else if let CoreExpr::Do { body, .. } = e {
                // 尾位 begin：逐值丢弃 + 尾递归（BodyTail 契约传递）
                lower_begin_tail(ctx, body)
            } else {
                // 非分支尾：原子化后即地终结
                let atom = lower_atom(ctx, e)?;
                ctx.seal_block(ACtrl::Ret(atom));
                Ok(atom)
            }
        }
        TailCtx::Value => lower_atom(ctx, e),
    }
}

/// 布尔条件原子化（`(< a b)` 等比较原语产布尔临时——整数 0/1）。
fn lower_bool(ctx: &mut LowerCtxt<'_>, e: &CoreExpr) -> Result<AAtom, LowerError> {
    lower_atom(ctx, e)
}

/// 原子 → 临时（若已是 Var 直接返回；Int 则物化绑定）。
fn ensure_temp(ctx: &mut LowerCtxt<'_>, a: AAtom, span: Span) -> Result<ATemp, LowerError> {
    match a {
        AAtom::Var(t) => Ok(t),
        AAtom::Int(v) => {
            // 字面量条件：常量折叠为临时绑定（如 (if 1 ...) ——类型宽口）
            let t = ctx.fresh_temp();
            let idx = ctx.blocks.len() - 1;
            // 走 emit 的同构路径（Prim 映射缺一元恒等——用 Add 0 实现）
            ctx.blocks[idx].1.push(AStmt {
                dst: t,
                op: AOp::Prim(APrim::Add, vec![AAtom::Int(v), AAtom::Int(0)]),
                span,
            });
            Ok(t)
        }
    }
}

/// 值上下文 if：块参数 merge（then/else 各自 Jmp merge(值)）。
fn lower_value_if(ctx: &mut LowerCtxt<'_>, e: &CoreExpr) -> Result<ATemp, LowerError> {
    let CoreExpr::If {
        cond,
        then_branch,
        else_branch,
        span,
    } = e
    else {
        unreachable!("调用点已匹配 If");
    };
    let cond_atom = lower_atom(ctx, cond)?;
    let cond_t = ensure_temp(ctx, cond_atom, *span)?;
    // 回填协议（then/else 内嵌 if 均可分裂多块——三处回填：Br.f、
    // 两处尾 Jmp.target；phi 来源标签 = 实际跳转块索引）
    let br_idx = ctx.blocks.len() - 1;
    let then_label = ALabel(ctx.blocks.len() as u32);
    ctx.seal_block(ACtrl::Br {
        cond: cond_t,
        t: then_label,
        f: ALabel(u32::MAX),
    });
    // then 块 → 尾 Jmp merge 占位（tv 物化——phi 源须为临时）
    ctx.start_block(Vec::new());
    let tv = lower_tail(ctx, then_branch, TailCtx::Value)?;
    let tv_t = ensure_temp(ctx, tv, then_branch.span())?;
    let then_tail = ctx.blocks.len() - 1;
    ctx.seal_block(ACtrl::Jmp {
        target: ALabel(u32::MAX),
    });
    // else 块（索引回填 Br.f）
    let else_label = ALabel(ctx.blocks.len() as u32);
    if let Some(ACtrl::Br { f, .. }) = ctx.ctrls[br_idx].as_mut() {
        *f = else_label;
    }
    ctx.start_block(Vec::new());
    let ev = lower_tail(ctx, else_branch, TailCtx::Value)?;
    let ev_t = ensure_temp(ctx, ev, else_branch.span())?;
    let else_tail = ctx.blocks.len() - 1;
    ctx.seal_block(ACtrl::Jmp {
        target: ALabel(u32::MAX),
    });
    // merge 块：回填两处 Jmp + phi（来源 = 尾块标签）
    let merge_label = ALabel(ctx.blocks.len() as u32);
    if let Some(ACtrl::Jmp { target }) = ctx.ctrls[then_tail].as_mut() {
        *target = merge_label;
    }
    if let Some(ACtrl::Jmp { target }) = ctx.ctrls[else_tail].as_mut() {
        *target = merge_label;
    }
    let result = ctx.fresh_temp();
    let then_phi_src = ALabel(then_tail as u32);
    let else_phi_src = ALabel(else_tail as u32);
    ctx.start_block(vec![APhi {
        dst: result,
        sources: vec![
            (then_phi_src, AAtom::Var(tv_t)),
            (else_phi_src, AAtom::Var(ev_t)),
        ],
    }]);
    Ok(result)
}

/// Apply lowering（调用——被调者必须为 Var 命中全局函数或原语）。
fn lower_call(ctx: &mut LowerCtxt<'_>, e: &CoreExpr) -> Result<ATemp, LowerError> {
    let CoreExpr::Apply {
        fn_expr,
        args,
        span,
    } = e
    else {
        unreachable!("调用点已匹配 Apply");
    };
    // 被调者形状：Var（直接调用/原语）——其余（lambda 直接应用等）边界外
    let CoreExpr::Var { name, .. } = fn_expr.as_ref() else {
        return Err(LowerError::new(
            "本地码 PoC 边界：被调者必须是函数名（直接调用）——\
             计算出的函数值未支持",
            fn_expr.span(),
        ));
    };
    let name_str = ctx.table.name(*name);

    // 原语路径（builtin 十二项 PoC 面）
    if let Some(prim) = APrim::from_builtin_name(name_str) {
        if !ctx.globals.names.contains_key(name) {
            let (lo, hi) = prim_arity(prim);
            if args.len() < lo || args.len() > hi {
                return Err(LowerError::new(
                    format!(
                        "原语 `{}` 参数数不符：{}（期望 {}..={}）",
                        name_str,
                        args.len(),
                        lo,
                        hi
                    ),
                    *span,
                ));
            }
            let mut atoms = Vec::with_capacity(args.len());
            for a in args {
                atoms.push(lower_atom(ctx, a)?);
            }
            return Ok(ctx.emit(AOp::Prim(prim, atoms), *span));
        }
        // 同名用户函数遮蔽原语（全局函数表优先——与 VM LOAD_GLOBAL 前置
        // 语义一致的保守裁定：define 遮蔽时走调用路径）
    }

    // 全局函数调用路径
    let Some(fname) = ctx.globals.names.get(name).cloned() else {
        return Err(LowerError::new(
            format!(
                "本地码 PoC 边界：调用目标 `{}` 不是已定义函数，\
                 也不在 PoC 原语面（print/IO 等内置未进本地码）",
                name_str
            ),
            *span,
        ));
    };
    let expect = ctx.globals.arities[name];
    if args.len() != expect {
        return Err(LowerError::new(
            format!(
                "调用 `{}` 参数数不符：{}（定义 {}）",
                name_str,
                args.len(),
                expect
            ),
            *span,
        ));
    }
    let mut atoms = Vec::with_capacity(args.len());
    for a in args {
        atoms.push(lower_atom(ctx, a)?);
    }
    Ok(ctx.emit(
        AOp::Call {
            func: fname,
            args: atoms,
        },
        *span,
    ))
}

/// 值上下文 Do（中间值丢弃，尾值返回）。
fn lower_begin_value(ctx: &mut LowerCtxt<'_>, e: &CoreExpr) -> Result<ATemp, LowerError> {
    let CoreExpr::Do { body, .. } = e else {
        unreachable!("调用点已匹配 Do");
    };
    if body.is_empty() {
        return Err(LowerError::new(
            "本地码 PoC 边界：空 begin 无值（VM 路径产 Nil——整数域外）",
            e.span(),
        ));
    }
    for (i, sub) in body.iter().enumerate() {
        let atom = lower_tail(ctx, sub, TailCtx::Value)?;
        if i + 1 == body.len() {
            return ensure_temp(ctx, atom, sub.span());
        }
    }
    unreachable!("空 begin 已拒绝")
}

/// 尾位 Do（丢弃中间值 + 尾递归）。
fn lower_begin_tail(ctx: &mut LowerCtxt<'_>, body: &[Rc<CoreExpr>]) -> Result<AAtom, LowerError> {
    if body.is_empty() {
        return Err(LowerError::new(
            "本地码 PoC 边界：空 lambda 体（VM 路径产 Nil——整数域外）",
            Span::default(),
        ));
    }
    for (i, sub) in body.iter().enumerate() {
        if i + 1 == body.len() {
            return lower_tail(ctx, sub, TailCtx::BodyTail);
        }
        // 中间值丢弃（原子化求值副作用）
        lower_tail(ctx, sub, TailCtx::Value)?;
    }
    unreachable!("空 body 已拒绝")
}
