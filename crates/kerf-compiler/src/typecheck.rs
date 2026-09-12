//! 保守静态类型检查器（Stage 1 批次 C）。
//!
//! 设计依据：[12-路线图 §2.5.2](../../docs/lang-design/12-roadmap.md) Stage 1
//! L 节点「引入类型检查器」+ 自举陷阱「类型检查器循环依赖」的外部 Rust
//! 实现裁定（[07-自举策略 §2](../../docs/lang-design/07-bootstrap-strategy.md)
//! / sop.md §21.6 缓解策略——Stage 2+ 再迁移到目标语言）。
//!
//! **旗标期角色（D8 阶段 2——r29/50-a）**：`kerf check` 生产判定面已
//! 切换至 [hm.rs](crate::hm)（HM 推断——`hm_check_program`）；本模块
//! R1-R8 **退为回归基线断言**（测试面超集门参照侧——hm_inference_tests
//! / effect_tests 双面检出纪律保留）。保守性契约的旗标期重定义（「零
//! 类型不一致误报」+ occurs/自应用豁免）见 hm-inference-design §2.3。
//!
//! **保守性契约（误报 = P1 缺陷）**：检查器只报告**静态确定**的错误——
//! 每条规则被触发时，对应程序在运行期**必然**以同类错误失败。任何
//! 依赖动态信息的情形（参数值类型未知、宏引入的卫生符号、递归定义）
//! 一律归为 `TcType::Unknown` 并跳过断言。判据：全部既有测试程序与本
//! 检查器零冲突（tests/v0 全量回归 = 保守性的机械验证）。
//!
//! 规则集（R1-R8，每条对应一类**确定**运行时错误）：
//! - R1 `if` 条件静态已知非 bool（运行时 E1 truthy 家族）；
//! - R2 算术族操作数静态已知非数值；
//! - R3 比较族操作数静态已知非数值（TD-016 全操作数口径）/ 混串数值；
//! - R4 `not` 操作数静态已知非 bool；
//! - R5 `car`/`cdr` 操作数静态已知非 pair；
//! - R6 被调表达式静态已知不可调用（运行时 E4 家族）；
//! - R7 元数不匹配：字面量 lambda / 内置签名（运行时 E2/E3 家族）；
//! - R8 字符串/符号族操作数静态已知类型不符。
//!
//! **多错误收集**：单一程序的全部规则触发按 Span 次序汇总返回——
//! TD-013 多错误收集设计的第一个消费者（`kerf check` 报告面）。
//!
//! 深度预算：`MAX_CHECK_DEPTH` 防御深嵌套表达式递归栈溢出（对齐
//! TD-017 的 2 MiB 线程栈实测口径）——超预算子树跳过检查（保守
//! Unknown，不产生误报，也不产生噪音诊断）。

use std::collections::HashMap;
use std::rc::Rc;

use kerf_core::{CoreExpr, LiteralValue};
use kerf_span::{Diagnostic, DiagnosticCode, Span};
use kerf_syntax::{Symbol, SymbolTable};

/// 静态检查诊断码（E0005——结构化阶段码序列 Read/Expand/Compile/Run 之后）。
pub const CHECK_DIAG_CODE: DiagnosticCode = DiagnosticCode(5);

/// 检查递归深度预算（栈安全；超限子树保守跳过）。
///
/// 取值依据：Reader 嵌套上限 256 的 2× 余量；实测标定——debug 构建
/// 帧尺寸 ~1 KiB，2000 深度实测溢出 2 MiB 线程栈（typecheck 单测存档），
/// 512 × 1 KiB = 512 KiB（4× 安全余量）。宏展开可产生超深结构
/// （A3 上限 10_000）——超预算子树保守 Unknown（零误报零噪音）。
pub const MAX_CHECK_DEPTH: usize = 512;

/// 静态类型（检查器内部推断的保守类型格）。
///
/// `Num` = Int/Float 并集（数值塔混合提升的静态上界）；`Unknown` = 任意
/// （⊤——动态信息不足，永不触发断言）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcType {
    /// 任意类型（信息不足——不参与断言）。
    Unknown,
    Int,
    Float,
    /// Int ∪ Float（算术/比较的合法域）。
    Num,
    Bool,
    Str,
    Nil,
    Symbol,
    Pair,
    /// 可调用（lambda 或内置；元数约束供 R7 使用）。
    Callable {
        min_args: usize,
        max_args: Option<usize>,
    },
}

impl TcType {
    /// 渲染为运行时 `type_name` 同口径的短名（错误消息对齐）。
    pub fn render(self) -> &'static str {
        match self {
            TcType::Unknown => "unknown",
            TcType::Int => "int",
            TcType::Float => "float",
            TcType::Num => "num",
            TcType::Bool => "bool",
            TcType::Str => "str",
            TcType::Nil => "nil",
            TcType::Symbol => "symbol",
            TcType::Pair => "pair",
            TcType::Callable { .. } => "procedure",
        }
    }

    /// 是否数值域成员（Int/Float/Num）。
    fn is_num(self) -> bool {
        matches!(self, TcType::Int | TcType::Float | TcType::Num)
    }

    /// 类型格的合并（join）：分支汇合点的保守上界。
    fn join(self, other: TcType) -> TcType {
        use TcType::*;
        match (self, other) {
            (a, b) if matches!((a, b), (Unknown, _) | (_, Unknown)) => Unknown,
            (Int, Float) | (Float, Int) | (Num, Int) | (Int, Num) | (Num, Float) | (Float, Num) => {
                Num
            }
            (a, b) if a == b => a,
            // _ 臂理由：具体类型互异（如 Int 与 Str）——运行时只可能取其一，
            // 静态无法收窄到单一类型，保守归 Unknown
            _ => Unknown,
        }
    }
}

/// 参数类型规则（内置签名表的数据形态——检查器按规则解释，不含具体
/// 内置名知识；内置清单唯一可信源在 kerf-driver builtins.rs，§2.3 原则 10）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcParam {
    /// 任意（不检查）。
    Any,
    /// 数值（Int/Float）。
    Num,
    /// 整数（索引类参数）。
    Int,
    /// 布尔。
    Bool,
    /// 字符串。
    Str,
    /// 符号值（TD-002）。
    Symbol,
    /// 序对。
    Pair,
    /// 列表（Pair ∪ Nil——length/reverse 接受空表）。
    List,
}

/// 参数规则布局：变长（全部同规则）/ 定长逐位 / 比较族特例。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcParams {
    /// 变长：全部参数共享同一规则。
    Variadic(TcParam),
    /// 定长：逐位规则（长度即固定元数）。
    Fixed(&'static [TcParam]),
    /// `=` 族特例：全数值或全字符串（09-stdlib TD-016 边界语义；
    /// TD-011 r24 解决后字符串全序参与全族——与 Ordering 同语义）。
    NumOrAllStr,
    /// 排序比较族（< > <= >=）：全数值或全字符串（码点序）；
    /// 混合即静态确定错误（TD-011 r24：与运行时/`=` 族同口径）。
    Ordering,
}

/// 内置函数静态签名（kerf-driver 注入——与 register_globals 的注册清单
/// 同文件维护，防漂移）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinSig {
    /// 参数规则。
    pub params: TcParams,
    /// 调用结果类型（供上层推断消费）。
    pub result: TcType,
    /// 最小元数。
    pub min_args: usize,
    /// 最大元数（None = 无上限）。
    pub max_args: Option<usize>,
}

impl BuiltinSig {
    /// 变长签名构造（arity ≥ min，全部参数同规则）。const：静态签名表
    /// （driver BUILTIN_SIGS）的编译期构造入口。
    pub const fn variadic(param: TcParam, result: TcType, min_args: usize) -> Self {
        BuiltinSig {
            params: TcParams::Variadic(param),
            result,
            min_args,
            max_args: None,
        }
    }

    /// 定长签名构造（参数规则逐位给出）。const——同上。
    pub const fn fixed(params: &'static [TcParam], result: TcType) -> Self {
        BuiltinSig {
            params: TcParams::Fixed(params),
            result,
            min_args: params.len(),
            max_args: Some(params.len()),
        }
    }

    /// `=` 族比较签名构造（全数值或全字符串 → bool）。const——同上。
    pub const fn num_or_all_str() -> Self {
        BuiltinSig {
            params: TcParams::NumOrAllStr,
            result: TcType::Bool,
            min_args: 2,
            max_args: None,
        }
    }

    /// 排序比较族签名构造（< > <= >=：全数值 → bool）。const——同上。
    pub const fn ordering() -> Self {
        BuiltinSig {
            params: TcParams::Ordering,
            result: TcType::Bool,
            min_args: 2,
            max_args: None,
        }
    }
}

/// 检查核心表达式序列（展开后的顶层形式），返回全部静态诊断（按
/// Span 次序排序——多错误收集，TD-013 设计的消费面）。
///
/// 入口契约：`builtins` 为用户可见内置符号 → 签名映射（driver 从
/// register_globals 同源清单构造）；`table` 为编译该序列使用的符号表
/// （运算名渲染——诊断消息与运行时前缀对齐）。
pub fn check_program(
    core: &[Rc<CoreExpr>],
    builtins: &HashMap<Symbol, BuiltinSig>,
    table: &SymbolTable,
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let mut ctx = TypeCtxt::new(builtins, table);
    for form in core {
        ctx.check_form(form, &mut diags);
    }
    diags.sort_by_key(|d| {
        (
            d.primary_span.file_id,
            d.primary_span.start,
            d.primary_span.end,
        )
    });
    diags
}

struct TypeCtxt<'a> {
    builtins: &'a HashMap<Symbol, BuiltinSig>,
    table: &'a SymbolTable,
    /// 词法环境（符号 → 类型）。lambda 参数以 Unknown 装订；define 顺序
    /// 填充；新绑定遮蔽旧绑定/内置。
    env: HashMap<Symbol, TcType>,
}

impl<'a> TypeCtxt<'a> {
    fn new(builtins: &'a HashMap<Symbol, BuiltinSig>, table: &'a SymbolTable) -> Self {
        TypeCtxt {
            builtins,
            table,
            env: HashMap::new(),
        }
    }

    fn diag(&self, diags: &mut Vec<Diagnostic>, message: String, span: Span) {
        diags.push(Diagnostic::error(Some(CHECK_DIAG_CODE), message, span));
    }

    /// 顶层/模块体内的顺序形式（define 生效于后续形式）。
    fn check_form(&mut self, e: &Rc<CoreExpr>, diags: &mut Vec<Diagnostic>) {
        if let CoreExpr::Define { name, value, .. } = e.as_ref() {
            let ty = self.check_expr(value, 0, diags);
            self.env.insert(*name, ty);
            // define 形式自身的返回值不参与后续推断（保守 Unknown）
        } else {
            self.check_expr(e, 0, diags);
        }
    }

    /// 检查表达式并返回其静态类型。
    fn check_expr(
        &mut self,
        e: &Rc<CoreExpr>,
        depth: usize,
        diags: &mut Vec<Diagnostic>,
    ) -> TcType {
        if depth > MAX_CHECK_DEPTH {
            // 深度预算耗尽：保守跳过（不诊断——避免深嵌套程序产生噪音；
            // 与「未检查」等价，零误报风险）
            return TcType::Unknown;
        }
        match e.as_ref() {
            CoreExpr::Literal { value, .. } => literal_ty(value),
            CoreExpr::VarRef { name, .. } => self.lookup(*name),
            CoreExpr::Lambda { params, body, .. } => {
                // 参数装订 Unknown（动态值），体检查后还原（遮蔽语义）
                let saved: Vec<(Symbol, Option<TcType>)> = params
                    .iter()
                    .map(|p| (*p, self.env.insert(*p, TcType::Unknown)))
                    .collect();
                self.check_expr(body, depth + 1, diags);
                for (sym, old) in saved {
                    restore(&mut self.env, sym, old);
                }
                TcType::Callable {
                    min_args: params.len(),
                    max_args: Some(params.len()),
                }
            }
            CoreExpr::App {
                fn_expr,
                args,
                span,
            } => self.check_app(fn_expr, args, *span, depth, diags),
            CoreExpr::If {
                cond,
                then_branch,
                else_branch,
                span,
            } => {
                let cond_ty = self.check_expr(cond, depth + 1, diags);
                // R1：条件静态已知非 bool（运行时 truthy 显式语义）
                if !matches!(cond_ty, TcType::Unknown | TcType::Bool) {
                    self.diag(
                        diags,
                        format!("if 条件需要 bool，实际 {}（静态检查）", cond_ty.render()),
                        *span,
                    );
                }
                let t = self.check_expr(then_branch, depth + 1, diags);
                let e_ty = self.check_expr(else_branch, depth + 1, diags);
                t.join(e_ty)
            }
            CoreExpr::Begin { body, .. } => {
                // 顺序体（begin 包裹的 define 已由展开器裁定为全局语义——
                // 03c/03d 修复后的统一行为，此处顺序装订与顶层同构）
                let mut last = TcType::Unknown;
                for item in body {
                    last = match item.as_ref() {
                        CoreExpr::Define { name, value, .. } => {
                            let ty = self.check_expr(value, depth + 1, diags);
                            self.env.insert(*name, ty);
                            TcType::Unknown
                        }
                        _ => self.check_expr(item, depth + 1, diags),
                    };
                }
                last
            }
            CoreExpr::SetBang { value, .. } => {
                // set! 返回被赋的值（Stage 0 语义）；不改变绑定已有类型
                self.check_expr(value, depth + 1, diags)
            }
            CoreExpr::Define { name, value, .. } => {
                // 非顶层位置的 define（防御路径：展开器正常不产出）
                let ty = self.check_expr(value, depth + 1, diags);
                self.env.insert(*name, ty);
                TcType::Unknown
            }
            CoreExpr::Module { body, .. } => {
                for form in body {
                    self.check_form(form, diags);
                }
                TcType::Unknown
            }
            // r8 能力声明：权限验证归 driver R9（E0006 家族），静态类型
            // 检查（E0005 家族）不涉——零运行时语义无类型约束
            CoreExpr::Require { .. } => TcType::Unknown,
            // r28/48-b 效应臂收敛（plan §5b——补深审 D3/D8「Unknown 放宽
            // 面」覆盖缺口）：Perform 效应值 / Handle 体与 handler 体受
            // R1-R8 检查（子表达式遍历）；结果类型维持 Unknown——Perform
            // 值 = resume 注入的任意值、Handle 值 = 体/handler 体汇合的
            // 动态结果（effect-language-design：效应行/行多态属 Stage 3
            // 类型层——静态面不收紧裁定维持）
            CoreExpr::Perform { effect, .. } => {
                self.check_expr(effect, depth + 1, diags);
                TcType::Unknown
            }
            // Handle 绑定器装订（与 Lambda 臂 save/restore 同型——遮蔽
            // 纪律镜像）：payload = dispatch 注入的动态值 → Unknown；
            // resume = continuation 调用形态（D4 展开期脱糖为 App——
            // 值位置调用保守零断言）→ Unknown
            CoreExpr::Handle {
                payload_var,
                resume_var,
                handler_body,
                body,
                ..
            } => {
                let saved: Vec<(Symbol, Option<TcType>)> = [payload_var, resume_var]
                    .into_iter()
                    .map(|p| (*p, self.env.insert(*p, TcType::Unknown)))
                    .collect();
                self.check_expr(handler_body, depth + 1, diags);
                self.check_expr(body, depth + 1, diags);
                for (sym, old) in saved {
                    restore(&mut self.env, sym, old);
                }
                TcType::Unknown
            }
        }
    }

    fn check_app(
        &mut self,
        fn_expr: &Rc<CoreExpr>,
        args: &[Rc<CoreExpr>],
        span: Span,
        depth: usize,
        diags: &mut Vec<Diagnostic>,
    ) -> TcType {
        let fn_ty = self.check_expr(fn_expr, depth + 1, diags);
        let arg_tys: Vec<TcType> = args
            .iter()
            .map(|a| self.check_expr(a, depth + 1, diags))
            .collect();
        // 内置调用判定：fn_expr 为 VarRef 且未被用户词法绑定遮蔽
        let builtin_sig = match fn_expr.as_ref() {
            CoreExpr::VarRef { name, .. } => {
                if self.env.contains_key(name) {
                    None
                } else {
                    self.builtins.get(name).copied()
                }
            }
            _ => None,
        };
        if let Some(sig) = builtin_sig {
            let op = match fn_expr.as_ref() {
                CoreExpr::VarRef { name, .. } => self.table.name(*name).to_string(),
                _ => "<内置>".to_string(),
            };
            self.check_builtin_app(&op, &sig, args, &arg_tys, span, diags);
            return sig.result;
        }
        match fn_ty {
            TcType::Unknown => {}
            TcType::Callable { min_args, max_args } => {
                // R7：元数不匹配（字面量 lambda / 已知闭包）
                if args.len() < min_args || max_args.is_some_and(|m| args.len() > m) {
                    let expect = match max_args {
                        Some(m) if m == min_args => format!("期望 {}", min_args),
                        _ => format!("期望至少 {}", min_args),
                    };
                    self.diag(
                        diags,
                        format!(
                            "过程参数数量不匹配：{} 实际 {}（静态检查）",
                            expect,
                            args.len()
                        ),
                        span,
                    );
                }
            }
            // R6：被调表达式静态已知不可调用
            other => {
                self.diag(
                    diags,
                    format!("不可调用的值：{}（静态检查）", other.render()),
                    span,
                );
            }
        }
        TcType::Unknown
    }

    /// 内置调用的规则检查（R2/R3/R4/R5/R7/R8——签名表数据驱动解释）。
    fn check_builtin_app(
        &mut self,
        op: &str,
        sig: &BuiltinSig,
        args: &[Rc<CoreExpr>],
        arg_tys: &[TcType],
        span: Span,
        diags: &mut Vec<Diagnostic>,
    ) {
        // R7：内置元数（floor / ceiling；元数先检——与运行时口径一致）
        if args.len() < sig.min_args || sig.max_args.is_some_and(|m| args.len() > m) {
            let expect = match sig.max_args {
                Some(m) if m == sig.min_args => format!("{} 需要 {} 个参数", op, m),
                _ => format!("{} 至少需要 {} 个参数", op, sig.min_args),
            };
            self.diag(
                diags,
                format!("{}，实际 {}（静态检查）", expect, args.len()),
                span,
            );
            return; // 元数错误后参数规则无意义
        }
        match sig.params {
            TcParams::Variadic(rule) => {
                for (a, ty) in args.iter().zip(arg_tys) {
                    self.check_param_rule(op, rule, *ty, a, diags);
                }
            }
            TcParams::Fixed(rules) => {
                for ((a, ty), rule) in args.iter().zip(arg_tys).zip(rules) {
                    self.check_param_rule(op, *rule, *ty, a, diags);
                }
            }
            TcParams::NumOrAllStr | TcParams::Ordering => {
                // R3（比较族——TD-016 全操作数口径；TD-011 r24 解决后
                // 字符串全序参与全族）：全数值或全字符串链静态放行
                // （码点序运行时合法）；混合链逐参数值域诊断
                // （首个非数值 → `{op} 需要数值`，与运行时消息对齐）。
                let concrete: Vec<TcType> = arg_tys
                    .iter()
                    .copied()
                    .filter(|t| !matches!(t, TcType::Unknown))
                    .collect();
                if !concrete.is_empty() && concrete.iter().all(|t| matches!(t, TcType::Str)) {
                    return; // 全字符串（码点序比较——运行时合法）
                }
                for (a, ty) in args.iter().zip(arg_tys) {
                    if matches!(ty, TcType::Unknown) || ty.is_num() {
                        continue;
                    }
                    self.diag(
                        diags,
                        format!("{} 需要数值，实际 {}（静态检查）", op, ty.render()),
                        expr_span(a, span),
                    );
                }
            }
        }
    }

    fn check_param_rule(
        &mut self,
        op: &str,
        rule: TcParam,
        ty: TcType,
        arg: &Rc<CoreExpr>,
        diags: &mut Vec<Diagnostic>,
    ) {
        if matches!(ty, TcType::Unknown) || rule == TcParam::Any {
            return;
        }
        let ok = match rule {
            TcParam::Any => true,
            TcParam::Num => ty.is_num(),
            TcParam::Int => matches!(ty, TcType::Int),
            TcParam::Bool => matches!(ty, TcType::Bool),
            TcParam::Str => matches!(ty, TcType::Str),
            TcParam::Symbol => matches!(ty, TcType::Symbol),
            TcParam::Pair => matches!(ty, TcType::Pair),
            TcParam::List => matches!(ty, TcType::Pair | TcType::Nil),
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
                format!("{} 需要数值，实际 {}（静态检查）", op, ty.render())
            } else {
                format!("{} 需要 {}，实际 {}（静态检查）", op, want, ty.render())
            };
            self.diag(diags, msg, expr_span(arg, Span::dummy()));
        }
    }

    fn lookup(&self, name: Symbol) -> TcType {
        if let Some(t) = self.env.get(&name) {
            return *t;
        }
        if let Some(sig) = self.builtins.get(&name) {
            return TcType::Callable {
                min_args: sig.min_args,
                max_args: sig.max_args,
            };
        }
        // 宏引入的卫生符号（name$hyg$N）与其他未知引用：动态解析，
        // 保守 Unknown（未绑定检查属运行期 E3 职责——卫生回退机制下
        // 静态断言会误报）
        TcType::Unknown
    }
}

fn restore(env: &mut HashMap<Symbol, TcType>, sym: Symbol, old: Option<TcType>) {
    match old {
        Some(t) => {
            env.insert(sym, t);
        }
        None => {
            env.remove(&sym);
        }
    }
}

fn literal_ty(value: &LiteralValue) -> TcType {
    match value {
        LiteralValue::Int(_) => TcType::Int,
        LiteralValue::Float(_) => TcType::Float,
        LiteralValue::Str(_) => TcType::Str,
        LiteralValue::Bool(_) => TcType::Bool,
        LiteralValue::Nil => TcType::Nil,
        LiteralValue::Symbol(_) => TcType::Symbol,
        LiteralValue::Pair(_, _) => TcType::Pair,
    }
}

/// 表达式自身 Span（CoreExpr 节点全覆盖——见 expr.rs `span()`）。
fn expr_span(e: &Rc<CoreExpr>, fallback: Span) -> Span {
    let own = e.span();
    if own == Span::dummy() {
        fallback
    } else {
        own
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kerf_core::LiteralValue;
    use kerf_span::Span;

    fn lit(v: LiteralValue) -> Rc<CoreExpr> {
        Rc::new(CoreExpr::Literal {
            value: v,
            span: Span::dummy(),
        })
    }

    /// 深度预算（程序化构造面）：超过 MAX_CHECK_DEPTH 的表达式树——
    /// Reader 深度上限 256 使源文本不可达，宏展开可产生超深结构
    /// （A3：展开深度上限 10_000）——预算截断递归、不崩溃、零诊断。
    #[test]
    fn depth_budget_skips_beyond_limit() {
        let mut e = lit(LiteralValue::Int(1));
        for _ in 0..(MAX_CHECK_DEPTH + 500) {
            e = Rc::new(CoreExpr::If {
                cond: lit(LiteralValue::Bool(true)),
                then_branch: e,
                else_branch: lit(LiteralValue::Int(2)),
                span: Span::dummy(),
            });
        }
        let core = vec![e];
        let builtins = HashMap::new();
        let table = SymbolTable::new();
        let diags = check_program(&core, &builtins, &table);
        assert!(
            diags.is_empty(),
            "超预算子树保守跳过——零诊断（实际 {} 条）",
            diags.len()
        );
    }

    /// 深度预算边界内（恰好 MAX_CHECK_DEPTH）：完整检查不截断。
    #[test]
    fn depth_budget_within_limit_full_check() {
        // 构造 (if 1 2 3) 嵌套于预算内的 if-true 链尾——R1 应检出
        let mut e = Rc::new(CoreExpr::If {
            cond: lit(LiteralValue::Int(1)),
            then_branch: lit(LiteralValue::Int(2)),
            else_branch: lit(LiteralValue::Int(3)),
            span: Span::dummy(),
        });
        for _ in 0..100 {
            e = Rc::new(CoreExpr::If {
                cond: lit(LiteralValue::Bool(true)),
                then_branch: e,
                else_branch: lit(LiteralValue::Int(2)),
                span: Span::dummy(),
            });
        }
        let core = vec![e];
        let builtins = HashMap::new();
        let table = SymbolTable::new();
        let diags = check_program(&core, &builtins, &table);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("if 条件需要 bool"));
    }

    /// 空程序/空参数：零诊断（边界防御）。
    #[test]
    fn empty_program_no_diags() {
        let builtins = HashMap::new();
        let table = SymbolTable::new();
        assert!(check_program(&[], &builtins, &table).is_empty());
    }
}
