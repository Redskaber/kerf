//! 元循环求值器（stage0.md §8.4——参考语义路径）。
//!
//! `CoreExpr` → `Value`：eval/apply 相互递归；环境为 `Rc` 链式结构
//! （§8.4 伪代码的忠实落地）。**词法闭包语义**（§8.5）：捕获定义时环境，
//! 可变捕获经 `RefCell` 共享传播。
//!
//! **堆使用约定**（crate 文档）：eval 路径的 Pair 分配关闭 GC 触发
//! （根集枚举不完整的既定 Stage 0 边界，TD-009）；GC 周期由 VM 路径
//! 在指令边界安全点驱动（§19.4 陷阱 2）。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kerf_core::{CoreExpr, LiteralValue};
use kerf_runtime::{GcRef, Heap, RuntimeError};
use kerf_span::Span;
use kerf_syntax::Symbol;

use crate::value::{ClosureValue, Value};

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
    fn from_runtime(e: &RuntimeError, span: Span) -> Self {
        EvalError::new(e.message.clone(), span)
    }
}

/// 词法环境（链式；§8.4 `Env = Rc<RefCell<HashMap<Symbol, Value>>>` 的
/// 链式扩展——闭包捕获定义处环境）。
#[derive(Debug)]
pub struct Env {
    bindings: RefCell<HashMap<Symbol, Value>>,
    parent: Option<Rc<Env>>,
}

impl Env {
    /// 空环境。
    pub fn new() -> Rc<Env> {
        Rc::new(Env {
            bindings: RefCell::new(HashMap::new()),
            parent: None,
        })
    }

    /// 子环境（当前环境成为父）。
    pub fn child(self: &Rc<Env>) -> Rc<Env> {
        Rc::new(Env {
            bindings: RefCell::new(HashMap::new()),
            parent: Some(Rc::clone(self)),
        })
    }

    /// 定义绑定。
    pub fn define(&self, name: Symbol, value: Value) {
        self.bindings.borrow_mut().insert(name, value);
    }

    /// 查找（沿父链——词法作用域）。
    pub fn lookup(&self, name: Symbol) -> Option<Value> {
        if let Some(v) = self.bindings.borrow().get(&name) {
            return Some(v.clone());
        }
        match &self.parent {
            Some(p) => p.lookup(name),
            None => None,
        }
    }

    /// 修改绑定（沿父链；未定义报 None——set! 未定义变量为错误）。
    pub fn set(&self, name: Symbol, value: Value) -> bool {
        if self.bindings.borrow().contains_key(&name) {
            self.bindings.borrow_mut().insert(name, value);
            return true;
        }
        match &self.parent {
            Some(p) => p.set(name, value),
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
    match e {
        CoreExpr::Literal { value, span } => eval_literal(value, heap, *span),
        CoreExpr::VarRef { name, span } => env
            .lookup(*name)
            .ok_or_else(|| EvalError::new("未绑定变量", *span)),
        CoreExpr::Lambda { params, body, .. } => Ok(Value::Closure(Rc::new(ClosureValue::Eval {
            params: params.clone(),
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
            // 求值顺序契约（§19.3/§19.5）：参数从左到右
            for a in args {
                arg_vals.push(eval_expr(a, env, heap)?);
            }
            apply_value(f, arg_vals, heap).map_err(|e| EvalError::from_runtime(&e, *span))
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
        CoreExpr::SetBang { name, value, span } => {
            let v = eval_expr(value, env, heap)?;
            if env.set(*name, v.clone()) {
                Ok(v)
            } else {
                Err(EvalError::new("set! 未绑定变量", *span))
            }
        }
        CoreExpr::Define { name, value, .. } => {
            let v = eval_expr(value, env, heap)?;
            env.define(*name, v.clone());
            Ok(v)
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
    }
}

/// 应用函数值（apply 侧，§8.4 `apply`）。
pub fn apply_value(f: Value, args: Vec<Value>, heap: &mut Heap) -> Result<Value, RuntimeError> {
    match &f {
        Value::Closure(rc) if matches!(rc.as_ref(), ClosureValue::Eval { .. }) => {
            let (params, body, env) = match rc.as_ref() {
                ClosureValue::Eval { params, body, env } => (params, body, env),
                _ => unreachable!("上方已窄化"),
            };
            if params.len() != args.len() {
                return Err(RuntimeError::new(format!(
                    "过程参数数量不匹配：期望 {} 实际 {}",
                    params.len(),
                    args.len()
                )));
            }
            let call_env = env.child();
            for (p, a) in params.iter().zip(args) {
                call_env.define(*p, a);
            }
            eval_expr(body, &call_env, heap)
                .map_err(|e| RuntimeError::new(format!("求值失败：{}", e.message)))
        }
        Value::Builtin(b) => b.call(heap, args),
        other => Err(RuntimeError::new(format!(
            "不可调用的值：{}",
            other.type_name()
        ))),
    }
}

/// 字面量求值（点对结构在堆上递归构造）。
#[allow(clippy::only_used_in_recursion)] // span 经递归透传至叶子错误（错误位置一致性）
fn eval_literal(v: &LiteralValue, heap: &mut Heap, span: Span) -> Result<Value, EvalError> {
    match v {
        LiteralValue::Int(i) => Ok(Value::Int(*i)),
        LiteralValue::Float(f) => Ok(Value::Float(*f)),
        LiteralValue::Str(s) => Ok(Value::Str(s.clone())),
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
