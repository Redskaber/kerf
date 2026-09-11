//! HM 类型推断 PoC（批次 H4——38-d 设计「约束三段式」落地）。
//!
//! **架构（D1 三段式）**：
//! ① 生成：逐顶层形式遍历 CoreExpr → 带.Span 约束集（具体类型错在
//!    生成期即时诊断——消息与 R1-R8 对齐（超集门复用）；变元/结构
//!    关系进约束集）；
//! ② 求解：worklist 合一（数值格扩展 + occurs check）——迭代式零深
//!    递归（自举友好 + 栈安全）；失败逐条收集（多错误——TD-013 协同）；
//! ③ zonk：自由变元 → Dynamic；全局绑定类型输出（验收面）。
//!
//! **裁定落地索引**（hm-inference-design.md §8）：
//! - D2 值限制：泛化仅限语法值（lambda/字面量/变量引用）；set! 目标与
//!   App 结果弱单态；
//! - D3 set! join：具体类型格合并（Int|Float→Num；异型→Dynamic——零
//!   误报）；变元目标（letrec 预置）首次赋值走合一；
//! - D4 递归预置：顶层 define + letrec 展开形双特判——绑定名预置 fresh
//!   变元，体内自引用与其合一（递归元数/域错可检出）；lambda-RHS 检查
//!   完成后泛化；
//! - D5 occurs check：必备、错误级、双端渲染；
//! - D6 双点泛化：顶层 define 序 + let 形状（App-of-Lambda 识别——
//!   用户手写同形同待遇）；
//! - D7 诊断：E0005 族 + 约束携带 Span + 形式级收集桶（P1-3 隔离）+
//!   生成期 512 深度预算（超限零约束零诊断——与 A6 等价）；
//! - D8 PoC 离线：本模块**不接入** driver check_source——测试面并行
//!   验证（超集门 + 零误报门）；旗标期切换另行裁定。
//!
//! **Dynamic 纪律**（A4 渐进逃生舱）：未绑定引用/卫生符号/builtin 值
//! 位置 → Dynamic 原子——不进约束、不泛化、永不报告。

use std::collections::HashMap;
use std::rc::Rc;

use kerf_span::{Diagnostic, Span};
use kerf_syntax::{Symbol, SymbolTable};

use crate::typecheck::{BuiltinSig, TcParam, TcParams, CHECK_DIAG_CODE};
use kerf_core::{CoreExpr, LiteralValue};

/// 生成期深度预算（A6 语义原样：超限子树零约束零诊断）。
const MAX_GEN_DEPTH: usize = 512;

/// HM 类型域（§3.5 十一构造子 + Dynamic 逃逸原子）。
#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    /// 类型变元（求解表索引）。
    Var(u32),
    Int,
    Float,
    /// Int ∪ Float（数值塔格——文档化偏差：非教科书合一）。
    Num,
    Bool,
    Str,
    Nil,
    Sym,
    /// 点对构造子（元素类型可推断）。
    Pair(Rc<Ty>, Rc<Ty>),
    /// 函数构造子（参数类型序列 + 返回类型；元数由序列长度携带）。
    Arrow {
        params: Vec<Rc<Ty>>,
        ret: Rc<Ty>,
    },
    /// 逃逸原子（不进约束、不泛化、永不报告）。
    Dynamic,
}

impl Ty {
    /// 渲染（诊断与验收断言面）。
    pub fn render(&self) -> String {
        match self {
            Ty::Var(i) => format!("α{}", i),
            Ty::Int => "int".into(),
            Ty::Float => "float".into(),
            Ty::Num => "num".into(),
            Ty::Bool => "bool".into(),
            Ty::Str => "str".into(),
            Ty::Nil => "nil".into(),
            Ty::Sym => "symbol".into(),
            Ty::Pair(a, b) => format!("({} . {})", a.render(), b.render()),
            Ty::Arrow { params, ret } => {
                let ps: Vec<String> = params.iter().map(|p| p.render()).collect();
                format!("({} → {})", ps.join(" "), ret.render())
            }
            Ty::Dynamic => "dynamic".into(),
        }
    }

    fn is_num_atom(&self) -> bool {
        matches!(self, Ty::Int | Ty::Float | Ty::Num)
    }
}

/// 环境绑定形态（D2 值限制的载体）。
#[derive(Debug, Clone)]
enum Binding {
    /// 弱单态（set! 目标 / App 结果 / letrec 预置期）——逐用点共享同一实例。
    Mono(Rc<Ty>),
    /// 泛化 scheme：quants 内变元在每次引用时 fresh 实例化。
    Poly { quants: Vec<u32>, body: Rc<Ty> },
}

/// 求解状态：变元替代表 + 待解约束 worklist + 诊断收集。
struct SolveState {
    /// 变元 → 已解类型（路径扁平：resolve 时写回）。
    subst: Vec<Option<Rc<Ty>>>,
    /// 待解约束（生成期累积；形式级排空——P1-3 桶隔离）。
    pending: Vec<(Rc<Ty>, Rc<Ty>, Span)>,
}

impl SolveState {
    fn new() -> Self {
        SolveState {
            subst: Vec::new(),
            pending: Vec::new(),
        }
    }

    fn fresh_var(&mut self) -> Rc<Ty> {
        self.subst.push(None);
        let id = self.subst.len() - 1;
        Rc::new(Ty::Var(id as u32))
    }

    /// 解析（沿替代链；扁平化写回）。
    fn resolve(&self, t: &Rc<Ty>) -> Rc<Ty> {
        let mut cur = t.clone();
        let mut steps: Vec<Rc<Ty>> = Vec::new();
        while let Ty::Var(i) = cur.as_ref() {
            if let Some(next) = self.subst[*i as usize].clone() {
                steps.push(cur.clone());
                cur = next;
            } else {
                break;
            }
        }
        // 路径压缩（写回需要 &mut——经 Cell 语义绕开借用：重建后由调用方
        // 持有；此实现取最终值，压缩留给深度极小的链（≤ 变元数））
        cur
    }

    fn is_free_var(&self, t: &Rc<Ty>) -> Option<u32> {
        match self.resolve(t).as_ref() {
            Ty::Var(i) => Some(*i),
            _ => None,
        }
    }

    /// occurs check：变元 i 是否出现在 t 的已解展开中。
    fn occurs(&self, i: u32, t: &Rc<Ty>) -> bool {
        let r = self.resolve(t);
        match r.as_ref() {
            Ty::Var(j) => *j == i,
            Ty::Pair(a, b) => self.occurs(i, a) || self.occurs(i, b),
            Ty::Arrow { params, ret } => {
                params.iter().any(|p| self.occurs(i, p)) || self.occurs(i, ret)
            }
            _ => false,
        }
    }

    /// 绑定变元（occurs 已检）。
    fn bind(&mut self, i: u32, t: Rc<Ty>) {
        self.subst[i as usize] = Some(t);
    }

    /// 合一（数值格扩展——D1/§3.5 文档化偏差）。失败返回诊断消息。
    fn unify(&mut self, a: &Rc<Ty>, b: &Rc<Ty>) -> Result<(), String> {
        let ra = self.resolve(a);
        let rb = self.resolve(b);
        use Ty::*;
        match (ra.as_ref(), rb.as_ref()) {
            // 逃逸原子吸收（防御——生成期已避免 Dynamic 进约束）
            (Dynamic, _) | (_, Dynamic) => Ok(()),
            // 数值格：Int/Float/Num 互匹（并集域语义）
            (x, y) if x.is_num_atom() && y.is_num_atom() => Ok(()),
            (Var(i), Var(j)) => {
                if i == j {
                    Ok(())
                } else {
                    self.bind(*i, rb.clone());
                    Ok(())
                }
            }
            (Var(i), t) => {
                if self.occurs(*i, &rb) {
                    return Err(format!(
                        "无法构造无限类型：α{} 与 {}（静态检查）",
                        i,
                        rb.render()
                    ));
                }
                let t = Rc::new(t.clone());
                self.bind(*i, t);
                Ok(())
            }
            (t, Var(i)) => {
                // 对称情形：bind i := ra（occurs 先检）
                if self.occurs(*i, &ra) {
                    return Err(format!(
                        "无法构造无限类型：α{} 与 {}（静态检查）",
                        i,
                        ra.render()
                    ));
                }
                let t = Rc::new(t.clone());
                self.bind(*i, t);
                Ok(())
            }
            (Pair(a1, a2), Pair(b1, b2)) => {
                self.unify(a1, b1)?;
                self.unify(a2, b2)
            }
            (
                Arrow {
                    params: ps1,
                    ret: r1,
                },
                Arrow {
                    params: ps2,
                    ret: r2,
                },
            ) => {
                if ps1.len() != ps2.len() {
                    return Err(format!(
                        "过程参数数量不匹配：期望 {} 实际 {}（静态检查）",
                        ps1.len(),
                        ps2.len()
                    ));
                }
                for (p1, p2) in ps1.iter().zip(ps2.iter()) {
                    self.unify(p1, p2)?;
                }
                self.unify(r1, r2)
            }
            (x, y) if x == y => Ok(()),
            (x, y) => Err(format!(
                "类型不一致：{} 与 {}（静态检查）",
                x.render(),
                y.render()
            )),
        }
    }

    /// 排空约束 worklist（失败逐条收集——多错误；残缺状态继续求解）。
    fn drain(&mut self, diags: &mut Vec<Diagnostic>) {
        while let Some((a, b, span)) = self.pending.pop() {
            if let Err(msg) = self.unify(&a, &b) {
                diags.push(Diagnostic::error(Some(CHECK_DIAG_CODE), msg, span));
            }
        }
    }
}

/// HM 检查报告（PoC 验收面：诊断 + 全局绑定最终类型）。
pub struct HmReport {
    /// 诊断（E0005 族；(file_id, start, end) 序——与 check_program 同构）。
    pub diags: Vec<Diagnostic>,
    /// 顶层 define 的 zonk 最终类型（验收断言面：fib : (num → num)）。
    pub globals: HashMap<Symbol, Rc<Ty>>,
}

/// HM 推断 PoC 入口（离线——D8：不接入 driver；测试面并行验证）。
pub fn hm_check_program(
    core: &[Rc<CoreExpr>],
    builtins: &HashMap<Symbol, BuiltinSig>,
    table: &SymbolTable,
) -> HmReport {
    let mut gen = Gen {
        builtins,
        table,
        env: HashMap::new(),
        st: SolveState::new(),
        diags: Vec::new(),
    };
    for form in core {
        gen.infer_form(form, 0);
        // 形式级桶排空（P1-3 隔离：失败不污染跨形式求解）
        gen.st.drain(&mut gen.diags);
    }
    // zonk + 报告
    let mut globals = HashMap::new();
    for (sym, b) in &gen.env {
        let t = match b {
            Binding::Mono(t) => gen.st.resolve(t),
            Binding::Poly { body, .. } => gen.st.resolve(body),
        };
        globals.insert(*sym, zonk(&gen.st, &t));
    }
    gen.diags.sort_by_key(|d| {
        (
            d.primary_span.file_id,
            d.primary_span.start,
            d.primary_span.end,
        )
    });
    HmReport {
        diags: gen.diags,
        globals,
    }
}

/// 自由变元收集（泛化面）。
fn free_vars(st: &SolveState, t: &Rc<Ty>, out: &mut Vec<u32>) {
    let r = st.resolve(t);
    match r.as_ref() {
        Ty::Var(i) => {
            if !out.contains(i) {
                out.push(*i);
            }
        }
        Ty::Pair(a, b) => {
            free_vars(st, a, out);
            free_vars(st, b, out);
        }
        Ty::Arrow { params, ret } => {
            for p in params {
                free_vars(st, p, out);
            }
            free_vars(st, ret, out);
        }
        // _ 臂理由：封闭类型（Con/Int/Bool/Str/Nil 等）不含变元——无自由变元可收集
        _ => {}
    }
}

/// 类型复制（实例化：quants → fresh 映射）。
fn copy_fresh(st: &mut SolveState, t: &Rc<Ty>, map: &mut HashMap<u32, Rc<Ty>>) -> Rc<Ty> {
    let r = st.resolve(t);
    match r.as_ref() {
        Ty::Var(i) => match map.get(i) {
            Some(v) => v.clone(),
            None => r.clone(),
        },
        Ty::Pair(a, b) => Rc::new(Ty::Pair(copy_fresh(st, a, map), copy_fresh(st, b, map))),
        Ty::Arrow { params, ret } => Rc::new(Ty::Arrow {
            params: params.iter().map(|p| copy_fresh(st, p, map)).collect(),
            ret: copy_fresh(st, ret, map),
        }),
        _ => r.clone(),
    }
}

/// zonk：自由变元 → Dynamic（验收渲染面）。
fn zonk(st: &SolveState, t: &Rc<Ty>) -> Rc<Ty> {
    let r = st.resolve(t);
    match r.as_ref() {
        Ty::Var(_) => Rc::new(Ty::Dynamic),
        Ty::Pair(a, b) => Rc::new(Ty::Pair(zonk(st, a), zonk(st, b))),
        Ty::Arrow { params, ret } => Rc::new(Ty::Arrow {
            params: params.iter().map(|p| zonk(st, p)).collect(),
            ret: zonk(st, ret),
        }),
        _ => r.clone(),
    }
}

/// 语法值判定（D2 值限制）。
fn is_syntactic_value(e: &CoreExpr) -> bool {
    matches!(
        e,
        CoreExpr::Literal { .. } | CoreExpr::Lambda { .. } | CoreExpr::VarRef { .. }
    )
}

/// 具体类型格合并（D3 set! join / If 分支汇合——保守契约）。
fn lattice_join(st: &SolveState, a: &Rc<Ty>, b: &Rc<Ty>) -> Rc<Ty> {
    let ra = st.resolve(a);
    let rb = st.resolve(b);
    match (ra.as_ref(), rb.as_ref()) {
        (Ty::Dynamic, _) | (_, Ty::Dynamic) => Rc::new(Ty::Dynamic),
        (x, y) if x.is_num_atom() && y.is_num_atom() => Rc::new(Ty::Num),
        (x, y) if x == y => ra,
        // 具体异型 → Dynamic（保守降级——运行时二选一，静态不收窄）
        _ => Rc::new(Ty::Dynamic),
    }
}

/// 生成器（① 段：遍历 + 约束/即时诊断）。
struct Gen<'a> {
    builtins: &'a HashMap<Symbol, BuiltinSig>,
    table: &'a SymbolTable,
    env: HashMap<Symbol, Binding>,
    st: SolveState,
    diags: Vec<Diagnostic>,
}

impl<'a> Gen<'a> {
    fn diag(&mut self, msg: String, span: Span) {
        self.diags
            .push(Diagnostic::error(Some(CHECK_DIAG_CODE), msg, span));
    }

    fn fresh(&mut self) -> Rc<Ty> {
        self.st.fresh_var()
    }

    fn op_name(&self, sym: &Symbol) -> String {
        self.table.name(*sym).to_string()
    }

    fn constraint(&mut self, a: Rc<Ty>, b: Rc<Ty>, span: Span) {
        self.st.pending.push((a, b, span));
    }

    /// 泛化点前排空待解约束（求值顺序义务）：泛化须看到已解类型——
    /// 否则预置变元/应用结果的类型尚在 worklist 中，quants 误收自由变元。
    fn solve_now(&mut self) {
        let mut diags = std::mem::take(&mut self.diags);
        self.st.drain(&mut diags);
        self.diags = diags;
    }

    fn instantiate(&mut self, b: &Binding) -> Rc<Ty> {
        match b {
            Binding::Mono(t) => t.clone(),
            Binding::Poly { quants, body } => {
                let mut map: HashMap<u32, Rc<Ty>> = HashMap::new();
                for q in quants {
                    map.insert(*q, self.fresh());
                }
                copy_fresh(&mut self.st, body, &mut map)
            }
        }
    }

    /// 环境自由变元集（泛化的非量化边界）。`exclude` = 正被泛化替换的
    /// 绑定名（自污染防护：被替换条目的自由变元不计入环境边界——
    /// 否则 define/letrec 的预置变元使 quants 恒空，泛化永不发生）。
    fn env_free_vars(&self, exclude: Option<Symbol>) -> Vec<u32> {
        let mut out = Vec::new();
        for (sym, b) in &self.env {
            if Some(*sym) == exclude {
                continue;
            }
            match b {
                Binding::Mono(t) => free_vars(&self.st, t, &mut out),
                Binding::Poly { body, .. } => free_vars(&self.st, body, &mut out),
            }
        }
        out
    }

    /// 泛化（值限制已由调用方判定）：quants = free(τ) − free(env)。
    fn generalize(&mut self, t: &Rc<Ty>, exclude: Option<Symbol>) -> Binding {
        let mut free = Vec::new();
        free_vars(&self.st, t, &mut free);
        let env_free = self.env_free_vars(exclude);
        let quants: Vec<u32> = free.into_iter().filter(|v| !env_free.contains(v)).collect();
        if quants.is_empty() {
            Binding::Mono(t.clone())
        } else {
            Binding::Poly {
                quants,
                body: t.clone(),
            }
        }
    }

    /// 顶层/模块体形式（define 序生效——D6 ①泛化点）。
    fn infer_form(&mut self, e: &Rc<CoreExpr>, depth: usize) {
        if depth > MAX_GEN_DEPTH {
            return;
        }
        if let CoreExpr::Define { name, value, .. } = e.as_ref() {
            // D4 递归预置：绑定名 fresh 变元（体内自引用与其合一）
            let alpha = self.fresh();
            self.env.insert(*name, Binding::Mono(alpha.clone()));
            let ty = self.infer_expr(value, depth + 1);
            // 预置变元与 RHS 类型合一（letrec/递归统一机制）
            self.constraint(alpha.clone(), ty, e.span());
            // 泛化点前求解（预置变元的实际类型须已流入）
            self.solve_now();
            // D6 ① + D2 值限制：语法值才泛化
            let binding = if is_syntactic_value(value) {
                self.generalize(&self.st.resolve(&alpha), Some(*name))
            } else {
                Binding::Mono(alpha.clone())
            };
            self.env.insert(*name, binding);
        } else {
            self.infer_expr(e, depth + 1);
        }
    }

    /// 表达式推断（返回类型；具体错即时诊断——R1-R8 消息面复用）。
    fn infer_expr(&mut self, e: &Rc<CoreExpr>, depth: usize) -> Rc<Ty> {
        if depth > MAX_GEN_DEPTH {
            return Rc::new(Ty::Dynamic);
        }
        match e.as_ref() {
            CoreExpr::Literal { value, .. } => Rc::new(literal_ty(value)),
            CoreExpr::VarRef { name, .. } => {
                let hit = self.env.get(name).cloned();
                match hit {
                    Some(b) => self.instantiate(&b),
                    // builtin 值位置（非应用头）与未绑定引用 → Dynamic
                    //（保守——A4 纪律：不进约束、不泛化、不报告）
                    None => Rc::new(Ty::Dynamic),
                }
            }
            CoreExpr::Lambda { params, body, .. } => {
                // 形参 fresh 变元（Mono）+ 体推断 → Arrow
                let mut param_tys = Vec::with_capacity(params.len());
                for p in params {
                    let alpha = self.fresh();
                    param_tys.push(alpha.clone());
                    let old = self.env.insert(*p, Binding::Mono(alpha));
                    let _ = old;
                }
                let ret = self.infer_expr(body, depth + 1);
                for p in params {
                    self.env.remove(p);
                }
                Rc::new(Ty::Arrow {
                    params: param_tys,
                    ret,
                })
            }
            CoreExpr::App { fn_expr, args, .. } => self.infer_app(fn_expr, args, e, depth + 1),
            CoreExpr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                // R1 面复用：条件具体非 bool 即时诊断；变元经约束钉 Bool
                let tc = self.infer_expr(cond, depth + 1);
                let rc = self.st.resolve(&tc);
                match rc.as_ref() {
                    Ty::Bool | Ty::Dynamic | Ty::Var(_) => {
                        if matches!(rc.as_ref(), Ty::Var(_)) {
                            self.constraint(tc.clone(), Rc::new(Ty::Bool), cond.span());
                        }
                    }
                    other => {
                        self.diag(
                            format!("if 条件需要 bool，实际 {}（静态检查）", other.render()),
                            cond.span(),
                        );
                    }
                }
                let tt = self.infer_expr(then_branch, depth + 1);
                let te = self.infer_expr(else_branch, depth + 1);
                // 分支汇合：两支具体 → 格 join（保守——D3/A3）；变元参与
                // → 约束合一（HM 增值面：分支类型一致性）
                let rt = self.st.resolve(&tt);
                let re = self.st.resolve(&te);
                match (rt.as_ref(), re.as_ref()) {
                    (Ty::Var(_), _) | (_, Ty::Var(_)) => {
                        self.constraint(tt.clone(), te, then_branch.span());
                        tt
                    }
                    _ => lattice_join(&self.st, &tt, &te),
                }
            }
            CoreExpr::SetBang { name, value, .. } => {
                // D3 set!：变元目标（letrec/递归预置）→ 合一（D4 统一机制）；
                // 具体目标 → 格 join（保守契约——赋值面零误报）；Poly 目标
                // → 弱单态化（D2——set! 目标永久不泛化）
                let tv = self.infer_expr(value, depth + 1);
                match self.env.get(name).cloned() {
                    Some(b) => {
                        let target = match &b {
                            Binding::Mono(t) => Some(t.clone()),
                            Binding::Poly { .. } => None,
                        };
                        match target {
                            Some(t) if self.st.is_free_var(&t).is_some() => {
                                self.constraint(t, tv, e.span());
                            }
                            Some(t) => {
                                let joined = lattice_join(&self.st, &t, &tv);
                                self.env.insert(*name, Binding::Mono(joined));
                            }
                            None => {
                                // Poly 目标 set!：弱单态化（保留当前值类型）
                                self.env.insert(*name, Binding::Mono(tv));
                            }
                        }
                    }
                    None => {
                        // 未绑定 set!：运行期 E3 职责（保守零诊断——A4）
                    }
                }
                Rc::new(Ty::Dynamic)
            }
            CoreExpr::Begin { body, .. } => {
                // 体首 define 提升形态（展开器产物）与顶层同构（C7：
                // 两路径同点泛化）
                let mut last = Rc::new(Ty::Nil);
                for item in body {
                    last = self.infer_form_value(item, depth + 1);
                }
                last
            }
            CoreExpr::Module { body, .. } => {
                let mut last = Rc::new(Ty::Nil);
                for item in body {
                    last = self.infer_form_value(item, depth + 1);
                }
                last
            }
            CoreExpr::Define { .. } => {
                // 表达式位置的 define（Begin/Module 内）——走形式语义
                self.infer_form(e, depth);
                Rc::new(Ty::Dynamic)
            }
            CoreExpr::Require { .. } => Rc::new(Ty::Dynamic),
            // r25/42-f 效应面（HM PoC 域外——effect-language-design 风险
            // 表「与 HM 推断的效应行交互 = P3/Stage 3」）：Perform 值 =
            // resume 注入的任意值；Handle 值 = 体/handler 体汇合——均
            // 降级 Dynamic（保守契约：不收紧、不误报）
            CoreExpr::Perform { .. } | CoreExpr::Handle { .. } => Rc::new(Ty::Dynamic),
        }
    }

    /// Begin/Module 内项：define 走形式（含预置/泛化）；其余走表达式。
    fn infer_form_value(&mut self, item: &Rc<CoreExpr>, depth: usize) -> Rc<Ty> {
        if matches!(item.as_ref(), CoreExpr::Define { .. }) {
            self.infer_form(item, depth);
            Rc::new(Ty::Dynamic)
        } else {
            self.infer_expr(item, depth + 1)
        }
    }

    fn infer_app(
        &mut self,
        fn_expr: &Rc<CoreExpr>,
        args: &[Rc<CoreExpr>],
        app: &Rc<CoreExpr>,
        depth: usize,
    ) -> Rc<Ty> {
        // ---- letrec 形状识别（D4：App[Lambda, nil] + 体首 SetBang 群） ----
        if let CoreExpr::Lambda { params, body, .. } = fn_expr.as_ref() {
            if let CoreExpr::Begin { body: items, .. } = body.as_ref() {
                // 体首 SetBang 群（全部作用于形参 = letrec 展开形——
                // sugar.rs desugar_letrec 实况核对：((lambda (f...)
                // (begin (set! f e)... body...)) nil...)
                let mut leading_sets: Vec<(Symbol, Rc<CoreExpr>)> = Vec::new();
                let mut all_params = true;
                for it in items {
                    match it.as_ref() {
                        CoreExpr::SetBang { name, value, .. } => {
                            if params.contains(name) {
                                leading_sets.push((*name, value.clone()));
                            } else {
                                all_params = false;
                            }
                        }
                        _ => break,
                    }
                }
                if all_params && !leading_sets.is_empty() && leading_sets.len() == params.len() {
                    return self.infer_letrec(params, &leading_sets, items, depth);
                }
            }
        }
        // ---- let 形状（D6 ②：App[Lambda, args] 同参数数） ----
        if let CoreExpr::Lambda { params, body, .. } = fn_expr.as_ref() {
            if params.len() == args.len() {
                return self.infer_let(params, body, args, depth);
            }
        }
        // ---- 通用应用 ----
        let arg_tys: Vec<Rc<Ty>> = args.iter().map(|a| self.infer_expr(a, depth)).collect();
        // builtin 快路径：应用头 = VarRef 且未被 env 遮蔽
        if let CoreExpr::VarRef { name, .. } = fn_expr.as_ref() {
            if !self.env.contains_key(name) {
                if let Some(sig) = self.builtins.get(name) {
                    return self.infer_builtin_call(name, sig, args, &arg_tys, app, depth);
                }
            }
        }
        let tf = self.infer_expr(fn_expr, depth);
        // R6 面复用：被调表达式具体非 Arrow 即时诊断
        let rf = self.st.resolve(&tf);
        match rf.as_ref() {
            Ty::Dynamic | Ty::Var(_) | Ty::Arrow { .. } => {}
            other => {
                self.diag(
                    format!("不可调用的值：{}（静态检查）", other.render()),
                    fn_expr.span(),
                );
                return Rc::new(Ty::Dynamic);
            }
        }
        // R7 面复用：具体 Arrow 元数即时诊断
        if let Ty::Arrow { params, .. } = rf.as_ref() {
            if params.len() != args.len() {
                self.diag(
                    format!(
                        "过程参数数量不匹配：期望 {} 实际 {}（静态检查）",
                        params.len(),
                        args.len()
                    ),
                    app.span(),
                );
                return Rc::new(Ty::Dynamic);
            }
        }
        let ret = self.fresh();
        self.constraint(
            tf,
            Rc::new(Ty::Arrow {
                params: arg_tys,
                ret: ret.clone(),
            }),
            app.span(),
        );
        ret
    }

    /// let 形状（D6 ②）：形参直绑实参类型；语法值实参可泛化。
    fn infer_let(
        &mut self,
        params: &[Symbol],
        body: &Rc<CoreExpr>,
        args: &[Rc<CoreExpr>],
        depth: usize,
    ) -> Rc<Ty> {
        let mut saved: Vec<(Symbol, Option<Binding>)> = Vec::with_capacity(params.len());
        for (p, a) in params.iter().zip(args.iter()) {
            let tv = self.infer_expr(a, depth);
            // 泛化点前求解（init 内部约束须已解——如 (cons λ nil) 元素型）
            self.solve_now();
            let binding = if is_syntactic_value(a) {
                self.generalize(&self.st.resolve(&tv), None)
            } else {
                Binding::Mono(tv)
            };
            saved.push((*p, self.env.insert(*p, binding)));
        }
        let result = self.infer_expr(body, depth + 1);
        for (p, old) in saved {
            match old {
                Some(b) => {
                    self.env.insert(p, b);
                }
                None => {
                    self.env.remove(&p);
                }
            }
        }
        result
    }

    /// letrec 形状（D4）：预置 → RHS 合一 → lambda-RHS 泛化 → 体。
    fn infer_letrec(
        &mut self,
        params: &[Symbol],
        sets: &[(Symbol, Rc<CoreExpr>)],
        begin_items: &[Rc<CoreExpr>],
        depth: usize,
    ) -> Rc<Ty> {
        let mut saved: Vec<(Symbol, Option<Binding>)> = Vec::with_capacity(params.len());
        // 预置 fresh 变元（全参数——互递归可见）
        for p in params {
            let alpha = self.fresh();
            saved.push((*p, self.env.insert(*p, Binding::Mono(alpha))));
        }
        // RHS 检查（递归自引用见预置变元）→ 合一
        let mut rhs_are_values = true;
        for (name, rhs) in sets {
            let tv = self.infer_expr(rhs, depth);
            if !is_syntactic_value(rhs) {
                rhs_are_values = false;
            }
            if let Some(Binding::Mono(t)) = self.env.get(name).cloned() {
                self.constraint(t, tv, rhs.span());
            }
        }
        // lambda-RHS 泛化（SML letrec-of-lambda 同款——体检查前完成；
        // 泛化点前求解——合一约束须已流入预置变元）
        if rhs_are_values {
            self.solve_now();
            for (name, _) in sets {
                if let Some(Binding::Mono(t)) = self.env.get(name).cloned() {
                    let binding = self.generalize(&self.st.resolve(&t), Some(*name));
                    self.env.insert(*name, binding);
                }
            }
        }
        // 体（set! 之后的形式）
        let mut last = Rc::new(Ty::Nil);
        for item in begin_items.iter().skip(sets.len()) {
            last = self.infer_form_value(item, depth + 1);
        }
        for (p, old) in saved {
            match old {
                Some(b) => {
                    self.env.insert(p, b);
                }
                None => {
                    self.env.remove(&p);
                }
            }
        }
        last
    }

    /// builtin 调用（签名表数据不动、解释层换——§3.5）。
    fn infer_builtin_call(
        &mut self,
        name: &Symbol,
        sig: &BuiltinSig,
        args: &[Rc<CoreExpr>],
        arg_tys: &[Rc<Ty>],
        app: &Rc<CoreExpr>,
        _depth: usize,
    ) -> Rc<Ty> {
        let op = self.op_name(name);
        // R7 面复用：元数区间即时诊断
        let n = args.len();
        if n < sig.min_args || sig.max_args.is_some_and(|m| n > m) {
            let want = match sig.max_args {
                Some(m) => format!("{}..{}", sig.min_args, m),
                None => format!("≥{}", sig.min_args),
            };
            self.diag(
                format!("过程参数数量不匹配：期望 {} 实际 {}（静态检查）", want, n),
                app.span(),
            );
            return Rc::new(Ty::Dynamic);
        }
        // 名字驱动的结构化重解释（§3.5 Pair 行——cons/car/cdr 三项）
        match op.as_str() {
            "cons" if n == 2 => {
                return Rc::new(Ty::Pair(arg_tys[0].clone(), arg_tys[1].clone()));
            }
            "car" if n == 1 => {
                let a = self.fresh();
                let b = self.fresh();
                let arg = arg_tys[0].clone();
                let ra = self.st.resolve(&arg);
                match ra.as_ref() {
                    Ty::Pair(x, _) => return x.clone(),
                    Ty::Dynamic | Ty::Var(_) | Ty::Nil => {
                        if matches!(ra.as_ref(), Ty::Var(_)) {
                            self.constraint(arg, Rc::new(Ty::Pair(a.clone(), b)), args[0].span());
                        }
                        // Nil：car nil = 运行期错误——保守不诊断（R5 口径
                        // 之外的动态边界），结果 Dynamic
                        return Rc::new(Ty::Dynamic);
                    }
                    other => {
                        self.diag(
                            format!("car 需要 pair，实际 {}（静态检查）", other.render()),
                            args[0].span(),
                        );
                        return Rc::new(Ty::Dynamic);
                    }
                }
            }
            "cdr" if n == 1 => {
                let b = self.fresh();
                let arg = arg_tys[0].clone();
                let ra = self.st.resolve(&arg);
                match ra.as_ref() {
                    Ty::Pair(_, y) => return y.clone(),
                    Ty::Dynamic | Ty::Var(_) | Ty::Nil => {
                        if matches!(ra.as_ref(), Ty::Var(_)) {
                            let a = self.fresh();
                            self.constraint(arg, Rc::new(Ty::Pair(a, b)), args[0].span());
                        }
                        return Rc::new(Ty::Dynamic);
                    }
                    other => {
                        self.diag(
                            format!("cdr 需要 pair，实际 {}（静态检查）", other.render()),
                            args[0].span(),
                        );
                        return Rc::new(Ty::Dynamic);
                    }
                }
            }
            // _ 臂理由：其余内置名/元数不匹配——非专项推断面，落入
            // 下方参数规则（sig.params）统一处理（快路径诊断不适用于此）
            _ => {}
        }
        // 参数规则（具体快路径即时诊断——R2/R3/R4/R5/R8 消息面复用；
        // 变元 → 约束）
        match &sig.params {
            TcParams::Variadic(rule) => {
                for (a, t) in args.iter().zip(arg_tys) {
                    self.check_param_rule(&op, *rule, t, a);
                }
            }
            TcParams::Fixed(rules) => {
                for ((a, t), rule) in args.iter().zip(arg_tys).zip(rules.iter()) {
                    self.check_param_rule(&op, *rule, t, a);
                }
            }
            TcParams::NumOrAllStr => {
                // R3 `=` 族：全数值或全字符串。变元参数经具体锚点约束
                //（数值锚 → ~Num；字符串锚且无数值 → ~Str）；全变元无锚
                // → 保守跳过（渐进逃生舱）；具体错型即时诊断。
                let resolved: Vec<Rc<Ty>> = arg_tys.iter().map(|t| self.st.resolve(t)).collect();
                let all_str =
                    !resolved.is_empty() && resolved.iter().all(|t| matches!(t.as_ref(), Ty::Str));
                if all_str {
                    return Rc::new(Ty::Bool);
                }
                let has_str = resolved.iter().any(|t| matches!(t.as_ref(), Ty::Str));
                let has_num = resolved.iter().any(|t| t.is_num_atom());
                for (a, t) in args.iter().zip(arg_tys) {
                    let rt = self.st.resolve(t);
                    match rt.as_ref() {
                        Ty::Dynamic => {}
                        Ty::Var(_) => {
                            let domain = if has_num {
                                Some(Ty::Num)
                            } else if has_str {
                                Some(Ty::Str)
                            } else {
                                None
                            };
                            if let Some(d) = domain {
                                self.constraint(rt.clone(), Rc::new(d), a.span());
                            }
                        }
                        Ty::Int | Ty::Float | Ty::Num => {}
                        other => {
                            self.diag(
                                format!("{} 需要数值，实际 {}（静态检查）", op, other.render()),
                                a.span(),
                            );
                        }
                    }
                }
            }
            TcParams::Ordering => {
                // R3 排序族：全字符串 → TD-011 边界消息；混串 → 需要数值；
                // 变元参数经数值锚点约束（~Num——运行时排序域全数值）。
                let resolved: Vec<Rc<Ty>> = arg_tys.iter().map(|t| self.st.resolve(t)).collect();
                let all_str =
                    !resolved.is_empty() && resolved.iter().all(|t| matches!(t.as_ref(), Ty::Str));
                if all_str {
                    self.diag(
                        format!(
                            "字符串仅支持 = 比较（Stage 0 边界，TD-011）（静态检查）——{}",
                            op
                        ),
                        app.span(),
                    );
                    return Rc::new(Ty::Bool);
                }
                let has_num = resolved.iter().any(|t| t.is_num_atom());
                for (a, t) in args.iter().zip(arg_tys) {
                    let rt = self.st.resolve(t);
                    match rt.as_ref() {
                        Ty::Dynamic => {}
                        Ty::Var(_) => {
                            if has_num {
                                self.constraint(rt.clone(), Rc::new(Ty::Num), a.span());
                            }
                        }
                        Ty::Int | Ty::Float | Ty::Num => {}
                        other => {
                            self.diag(
                                format!("{} 需要数值，实际 {}（静态检查）", op, other.render()),
                                a.span(),
                            );
                        }
                    }
                }
            }
        }
        // 结果类型映射（Unknown → fresh；数值族塔内格归一）
        match sig.result {
            crate::typecheck::TcType::Unknown => self.fresh(),
            crate::typecheck::TcType::Int => Rc::new(Ty::Int),
            crate::typecheck::TcType::Float => Rc::new(Ty::Float),
            crate::typecheck::TcType::Num => Rc::new(Ty::Num),
            crate::typecheck::TcType::Bool => Rc::new(Ty::Bool),
            crate::typecheck::TcType::Str => Rc::new(Ty::Str),
            crate::typecheck::TcType::Nil => Rc::new(Ty::Nil),
            crate::typecheck::TcType::Symbol => Rc::new(Ty::Sym),
            // Pair/Callable 结果类型经结构化路径（cons/car/cdr）或 Dynamic
            crate::typecheck::TcType::Pair | crate::typecheck::TcType::Callable { .. } => {
                Rc::new(Ty::Dynamic)
            }
        }
    }

    /// 参数规则（生成期：具体错即时诊断；变元/域内 → 约束）。
    fn check_param_rule(&mut self, op: &str, rule: TcParam, ty: &Rc<Ty>, arg: &Rc<CoreExpr>) {
        let rt = self.st.resolve(ty);
        if matches!(rt.as_ref(), Ty::Dynamic) {
            return;
        }
        if let Ty::Var(_) = rt.as_ref() {
            // 变元 → 域约束（Int/Float/Num 域内格合一吸收）
            let domain = match rule {
                TcParam::Num | TcParam::Int => Some(Ty::Num),
                TcParam::Bool => Some(Ty::Bool),
                TcParam::Str => Some(Ty::Str),
                TcParam::Symbol => Some(Ty::Sym),
                TcParam::Pair => None, // Pair 经 cons/car/cdr 结构化路径
                _ => None,
            };
            if let Some(d) = domain {
                self.constraint(rt.clone(), Rc::new(d), arg.span());
            }
            return;
        }
        let ok = match rule {
            TcParam::Any => true,
            TcParam::Num => rt.is_num_atom(),
            TcParam::Int => matches!(rt.as_ref(), Ty::Int),
            TcParam::Bool => matches!(rt.as_ref(), Ty::Bool),
            TcParam::Str => matches!(rt.as_ref(), Ty::Str),
            TcParam::Symbol => matches!(rt.as_ref(), Ty::Sym),
            TcParam::Pair => matches!(rt.as_ref(), Ty::Pair(_, _)),
            TcParam::List => matches!(rt.as_ref(), Ty::Pair(_, _) | Ty::Nil),
        };
        if !ok {
            let want = match rule {
                TcParam::Num => "数值",
                TcParam::Int => "int",
                TcParam::Bool => "bool",
                TcParam::Str => "str",
                TcParam::Symbol => "symbol",
                TcParam::Pair => "pair",
                TcParam::List => "list",
                TcParam::Any => unreachable!("Any 已短路"),
            };
            let msg = if rule == TcParam::Num {
                format!("{} 需要数值，实际 {}（静态检查）", op, rt.render())
            } else {
                format!("{} 需要 {}，实际 {}（静态检查）", op, want, rt.render())
            };
            self.diag(msg, arg.span());
        }
    }
}

/// 字面量类型（quote 点对结构化——Pair(τ, τ) 递归）。
fn literal_ty(v: &LiteralValue) -> Ty {
    match v {
        LiteralValue::Int(_) => Ty::Int,
        LiteralValue::Float(_) => Ty::Float,
        LiteralValue::Str(_) => Ty::Str,
        LiteralValue::Bool(_) => Ty::Bool,
        LiteralValue::Nil => Ty::Nil,
        LiteralValue::Symbol(_) => Ty::Sym,
        LiteralValue::Pair(a, b) => Ty::Pair(Rc::new(literal_ty(a)), Rc::new(literal_ty(b))),
    }
}
