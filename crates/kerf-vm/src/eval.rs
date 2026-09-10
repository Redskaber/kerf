//! 元循环求值器（stage0.md §8.4——参考语义路径）。
//!
//! `CoreExpr` → `Value`：eval/apply 相互递归；环境为 `Rc` 链式结构
//! （§8.4 伪代码的忠实落地）。**词法闭包语义**（§8.5）：捕获定义时环境，
//! 可变捕获经 `RefCell` 共享传播。
//!
//! **堆使用约定**（crate 文档）：eval 路径的 Pair 分配关闭 GC 触发
//! （根集枚举不完整的既定 Stage 0 边界，TD-009）；GC 周期由 VM 路径
//! 在指令边界安全点驱动（§19.4 陷阱 2）。

use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use kerf_core::{CoreExpr, LiteralValue};
use kerf_runtime::{GcRef, Heap, RuntimeError};
use kerf_span::Span;
use kerf_syntax::{ScopeSet, Symbol};

use crate::value::{ClosureValue, Value};
use crate::vm::MAX_FRAMES;

/// eval 参考路径的求值深度上限（D3 修复：原实现无防护 → Rust 栈溢出
/// abort）。**实测标定**（debug 构建，每深度单元 ≈ 3 KiB 物理栈）：
/// 8 MiB 主线程 ≈ 1100 程序层；2 MiB 测试线程 ≈ 275 层（≈ 690 深度
/// 单位）。取 256 留 2.7× 裕度（最小可信栈 2 MiB 下结构化报错先于
/// 物理溢出；release 帧更小、裕度更大）。
/// 注：两路径上限数值不同（256 vs 100000）是参考路径的既定边界
/// （与 TD-009 同类——eval 为语义基准非生产载体）；T1 定理在
/// 常规程序域（深度 ≤ 256，含 fib(25)）内成立。
const MAX_EVAL_DEPTH: usize = 256;

thread_local! {
    /// 求值深度计数（RAII 守卫维护——避免签名侵入式传参）。
    static EVAL_DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// 深度守卫：进入即 +1，退出（含 `?` 早退）自动 -1；超限报结构化错误。
struct DepthGuard;

impl DepthGuard {
    fn enter(span: Span) -> Result<Self, EvalError> {
        let next = EVAL_DEPTH.with(|c| c.get()) + 1;
        if next > MAX_EVAL_DEPTH {
            return Err(EvalError::new(
                format!(
                    "求值深度超过上限 {}（eval 参考路径；VM 生产路径为 {} 帧结构化上限）",
                    MAX_EVAL_DEPTH, MAX_FRAMES
                ),
                span,
            ));
        }
        EVAL_DEPTH.with(|c| c.set(next));
        Ok(DepthGuard)
    }
}

impl Drop for DepthGuard {
    fn drop(&mut self) {
        EVAL_DEPTH.with(|c| c.set(c.get().saturating_sub(1)));
    }
}

/// 求值错误（`{ message, span }`——span 直接取自 CoreExpr 节点）。
#[derive(Debug, Clone, PartialEq)]
pub struct EvalError {
    pub message: String,
    pub span: Span,
}

impl EvalError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        EvalError {
            message: message.into(),
            span,
        }
    }

    /// 从运行时错误提升（span 由求值位置回填——Layer 0 无位置信息）。
    ///
    /// D7 修复后主路径经 `apply_value` 的 `call_span` 回填；本入口保留
    /// 给错误转换兼容场景。
    #[allow(dead_code)]
    fn from_runtime(e: &RuntimeError, span: Span) -> Self {
        EvalError::new(e.message.clone(), span)
    }
}

/// 词法环境绑定项（TD-004/r13：名 + 绑定作用域集 + 值）。
#[derive(Debug)]
struct Binding {
    name: Symbol,
    scopes: ScopeSet,
    value: Value,
}

/// 词法环境（链式；§8.4 `Env = Rc<RefCell<HashMap<Symbol, Value>>>` 的
/// 链式扩展——闭包捕获定义处环境）。
///
/// **作用域集解析（TD-004/r13）**：绑定携带绑定作用域集；查找/赋值按
/// `(name, scopes ⊆)` 子集匹配 + max-cardinality（Racket 集合作用域）。
/// 空作用域集绑定 ⊆ 任意引用集——全局/内置（driver 根环境注入）天然
/// 充当名称基兑底；`$hyg$` 基名回退仍由 driver 在全局层完成（回退路径）。
#[derive(Debug)]
pub struct Env {
    bindings: RefCell<Vec<Binding>>,
    parent: Option<Rc<Env>>,
}

impl Env {
    /// 空环境。
    pub fn new() -> Rc<Env> {
        Rc::new(Env {
            bindings: RefCell::new(Vec::new()),
            parent: None,
        })
    }

    /// 子环境（当前环境成为父）。
    pub fn child(self: &Rc<Env>) -> Rc<Env> {
        Rc::new(Env {
            bindings: RefCell::new(Vec::new()),
            parent: Some(Rc::clone(self)),
        })
    }

    /// 定义全局式绑定（空作用域集——兑底语义；driver 根环境注入用）。
    /// 同层同名重复返回 false（D1/E6 口径）。
    pub fn define(&self, name: Symbol, value: Value) -> bool {
        self.define_scoped(name, ScopeSet::new(), value)
    }

    /// 定义带作用域集的绑定（TD-004：lambda 形参绑定等）。同层同名
    /// （不论作用域）重复返回 false——与编译器 A3/展开器重复形参
    /// 检查同一口径；跨层 shadowing 合法。
    pub fn define_scoped(&self, name: Symbol, scopes: ScopeSet, value: Value) -> bool {
        let mut b = self.bindings.borrow_mut();
        if b.iter().any(|e| e.name == name) {
            return false;
        }
        b.push(Binding {
            name,
            scopes,
            value,
        });
        true
    }

    /// 子集匹配解析（单环境层内）：同名且 `binder.scopes ⊆ ref_scopes`
    /// 的绑定中取 max-cardinality；基数并列时先注册优先（同层同名
    /// 经 define_scoped 已拒绝，并列仅理论可能）。返回索引。
    /// 跨层由 [`Env::lookup`] 的链序（内层先查）消解——与注入不变式
    /// 下「内层绑定基数严格更大」等价（良构程序上与旧名称基链序同解）。
    fn match_index(&self, name: Symbol, ref_scopes: &ScopeSet) -> Option<usize> {
        let b = self.bindings.borrow();
        let mut best: Option<usize> = None;
        let mut best_rank = 0usize;
        for (i, e) in b.iter().enumerate() {
            if e.name != name || !e.scopes.is_subset_of(ref_scopes) {
                continue;
            }
            let rank = e.scopes.len();
            if best.is_none() || rank > best_rank {
                best = Some(i);
                best_rank = rank;
            }
        }
        best
    }

    /// 查找（沿父链——内层先查；TD-004：按 `(name, scopes ⊆)` 匹配，
    /// 层内 max-cardinality；链序 + 注入不变式 ⇒ 与 Racket 全局
    /// max-cardinality 在良构程序上同解）。
    pub fn lookup(&self, name: Symbol, ref_scopes: &ScopeSet) -> Option<Value> {
        if let Some(i) = self.match_index(name, ref_scopes) {
            return Some(self.bindings.borrow()[i].value.clone());
        }
        match &self.parent {
            Some(p) => p.lookup(name, ref_scopes),
            None => None,
        }
    }

    /// 修改绑定（沿父链；TD-004：与 lookup 同一匹配口径——set! 目标
    /// 必须命中同解析的绑定，未命中报 None）。
    pub fn set(&self, name: Symbol, ref_scopes: &ScopeSet, value: Value) -> bool {
        if let Some(i) = self.match_index(name, ref_scopes) {
            self.bindings.borrow_mut()[i].value = value;
            return true;
        }
        match &self.parent {
            Some(p) => p.set(name, ref_scopes, value),
            None => false,
        }
    }
}

/// 求值入口（程序 = 顶层形式序列；返回最后一个值）。
pub fn eval_program(
    exprs: &[Rc<CoreExpr>],
    env: &Rc<Env>,
    heap: &mut Heap,
) -> Result<Value, EvalError> {
    let mut last = Value::Nil;
    for e in exprs {
        last = eval_expr(e, env, heap)?;
    }
    Ok(last)
}

/// 求值单条 CoreExpr（入口函数，§10.1 规则 1）。
pub fn eval_expr(e: &CoreExpr, env: &Rc<Env>, heap: &mut Heap) -> Result<Value, EvalError> {
    let _depth = DepthGuard::enter(e.span())?;
    match e {
        CoreExpr::Literal { value, span } => eval_literal(value, heap, *span),
        CoreExpr::VarRef { name, scopes, span } => env
            .lookup(*name, scopes)
            .ok_or_else(|| EvalError::new("未绑定变量", *span)),
        CoreExpr::Lambda {
            params,
            param_scopes,
            body,
            ..
        } => Ok(Value::Closure(Rc::new(ClosureValue::Eval {
            params: params.clone(),
            param_scopes: param_scopes.clone(),
            body: Rc::clone(body),
            env: Rc::clone(env),
        }))),
        CoreExpr::App {
            fn_expr,
            args,
            span,
        } => {
            let f = eval_expr(fn_expr, env, heap)?;
            let mut arg_vals = Vec::with_capacity(args.len());
            // 求值顺序契约（06 §1.3/§2 A1）：被调函数先求值，参数从左到右
            for a in args {
                arg_vals.push(eval_expr(a, env, heap)?);
            }
            // D7 修复：apply 错误保真透传（不再逐层包装前缀/覆盖 Span）
            apply_value(f, arg_vals, heap, *span)
        }
        CoreExpr::If {
            cond,
            then_branch,
            else_branch,
            span,
        } => {
            let c = eval_expr(cond, env, heap)?;
            match c {
                Value::Bool(true) => eval_expr(then_branch, env, heap),
                Value::Bool(false) => eval_expr(else_branch, env, heap),
                other => Err(EvalError::new(
                    format!(
                        "if 条件需要 bool，实际 {}（truthy 语义显式定义）",
                        other.type_name()
                    ),
                    *span,
                )),
            }
        }
        CoreExpr::SetBang {
            name,
            scopes,
            value,
            span,
        } => {
            let v = eval_expr(value, env, heap)?;
            if env.set(*name, scopes, v.clone()) {
                Ok(v)
            } else {
                Err(EvalError::new("set! 未绑定变量", *span))
            }
        }
        CoreExpr::Define { name, value, span } => {
            let v = eval_expr(value, env, heap)?;
            // D1/E6（[06-操作语义 §2 R6]）：同层重复定义报错。
            if env.define(*name, v.clone()) {
                Ok(v)
            } else {
                Err(EvalError::new("重复定义变量", *span))
            }
        }
        CoreExpr::Begin { body, span } => {
            let mut last = Value::Nil;
            for item in body {
                last = eval_expr(item, env, heap)?;
            }
            let _ = span;
            Ok(last)
        }
        CoreExpr::Module { body, span, .. } => {
            // Stage 0 单模块：模块体在当前（全局）环境求值
            let mut last = Value::Nil;
            for item in body {
                last = eval_expr(item, env, heap)?;
            }
            let _ = span;
            Ok(last)
        }
        CoreExpr::Require { span, .. } => {
            // r8 能力声明：零运行时语义——求值为 nil（与编译路径的
            // 零字节码行为一致；T1 双路径口径）
            let _ = span;
            Ok(Value::Nil)
        }
    }
}

/// 应用函数值（apply 侧，§8.4 `apply`）。
///
/// D7 修复：错误以 `EvalError` 形态保真透传——Eval 闭包体的错误
/// （消息 + 内层 Span）原样上抛，不再逐层包装「求值失败：」前缀
/// （修复前 f→g→错 的报告会累积两层前缀且 Span 落最外调用点）；
/// 无位置信息的错误（元数/不可调用/内置）以 `call_span` 回填。
pub fn apply_value(
    f: Value,
    args: Vec<Value>,
    heap: &mut Heap,
    call_span: Span,
) -> Result<Value, EvalError> {
    match &f {
        Value::Closure(rc) if matches!(rc.as_ref(), ClosureValue::Eval { .. }) => {
            let (params, param_scopes, body, env) = match rc.as_ref() {
                ClosureValue::Eval {
                    params,
                    param_scopes,
                    body,
                    env,
                } => (params, param_scopes, body, env),
                _ => unreachable!("上方已窄化"),
            };
            if params.len() != args.len() {
                return Err(EvalError::new(
                    format!(
                        "过程参数数量不匹配：期望 {} 实际 {}",
                        params.len(),
                        args.len()
                    ),
                    call_span,
                ));
            }
            let call_env = env.child();
            for ((p, ps), a) in params.iter().zip(param_scopes.iter()).zip(args) {
                // A3 卫式：参数表重名报错（同名形参在同层只允许出现一次）。
                // 绑定携带参数绑定作用域集（TD-004：体内引用按子集匹配命中）。
                if !call_env.define_scoped(*p, ps.clone(), a) {
                    return Err(EvalError::new(
                        "过程参数重名（lambda 形参表重复）",
                        call_span,
                    ));
                }
            }
            // 保真透传：体求值错误（含 Span）不改写
            eval_expr(body, &call_env, heap)
        }
        Value::Builtin(b) => b
            .call(heap, args)
            .map_err(|e| EvalError::new(e.message, call_span)),
        other => Err(EvalError::new(
            format!("不可调用的值：{}", other.type_name()),
            call_span,
        )),
    }
}

/// 字面量求值（点对结构在堆上递归构造）。
#[allow(clippy::only_used_in_recursion)] // span 经递归透传至叶子错误（错误位置一致性）
fn eval_literal(v: &LiteralValue, heap: &mut Heap, span: Span) -> Result<Value, EvalError> {
    match v {
        LiteralValue::Int(i) => Ok(Value::Int(*i)),
        LiteralValue::Float(f) => Ok(Value::Float(*f)),
        LiteralValue::Str(s) => Ok(Value::Str(s.clone())),
        LiteralValue::Symbol(s) => Ok(Value::Symbol(s.clone())),
        LiteralValue::Bool(b) => Ok(Value::Bool(*b)),
        LiteralValue::Nil => Ok(Value::Nil),
        LiteralValue::Pair(car, cdr) => {
            let car_v = eval_literal(car, heap, span)?;
            let cdr_v = eval_literal(cdr, heap, span)?;
            let car_r = to_heap_ref(&car_v, heap);
            let cdr_r = to_heap_ref(&cdr_v, heap);
            Ok(Value::Pair(heap.alloc_pair(car_r, cdr_r)))
        }
    }
}

/// 值装箱为堆引用（即时值按类型装箱——与 VM 路径的 box_value 语义一致）。
fn to_heap_ref(v: &Value, heap: &mut Heap) -> GcRef {
    crate::vm::box_value(v, heap)
}
