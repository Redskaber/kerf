//! 运行时值模型（stage0.md §8.4 `Value` / §8.5 基础闭包）。
//!
//! 双路径闭包形态：
//! - `ClosureValue::Eval`：树走查路径（`Rc` 环境链——§8.4 伪代码的忠实落地）；
//! - `ClosureValue::Bytecode`：VM 路径（原型索引 + 共享可变捕获单元——
//!   词法闭包的 set! 语义经 `Rc<GcCell>` 传播；TD-023 r24：单元携带
//!   堆根性摘要标志，GC 根扫描 O(1) 跳过非堆单元）。

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use kerf_core::CoreExpr;
use kerf_runtime::{GcRef, Heap, RuntimeError};
use kerf_syntax::{ScopeSet, Symbol};

use crate::eval::Env;

/// 运行时值（双路径共享）。
#[derive(Debug, Clone)]
pub enum Value {
    /// 单元（未定义返回值——Stage 0 与 Nil 区分以支持未来的多值/效应扩展）。
    Unit,
    /// 空（false 的数据对应物；经典 Lisp 的 '()）。
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    /// 符号值（quote 符号 datum 的运行时形态——TD-002；按名相等）。
    Symbol(Rc<str>),
    /// 堆引用：序对 (a . b)（kerf-runtime GC 管理）。
    Pair(GcRef),
    /// 闭包。
    Closure(Rc<ClosureValue>),
    /// 内置函数（driver 注册进全局环境）。
    Builtin(Rc<BuiltinFn>),
}

impl Value {
    /// 类型名（诊断）。
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Unit => "unit",
            Value::Nil => "nil",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "str",
            Value::Symbol(_) => "symbol",
            Value::Pair(_) => "pair",
            Value::Closure(_) => "procedure",
            Value::Builtin(_) => "builtin-procedure",
        }
    }

    /// truthy 判定（§19.5 陷阱 3：仅 Bool——其余类型报错显式化）。
    /// **单一实现**（TD-018）：VM 条件跳转与 eval if 臂共用本入口，
    /// 消息经 `messages::err_if_cond_bool` 单源构造（此前两路径文本分裂）。
    pub fn truthy(&self) -> Result<bool, RuntimeError> {
        match self {
            Value::Bool(b) => Ok(*b),
            other => Err(RuntimeError::new(crate::messages::err_if_cond_bool(
                other.type_name(),
            ))),
        }
    }

    /// 数值提取（算术操作数）。
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// 数值化为 i64（整型严格路径）。
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// 通用相等（eq? 语义：即时值按值、堆值按引用、闭包按标识）。
    ///
    /// Str 为即时值（`Rc<str>` 载体）——按**内容**比较（D2 修复：
    /// 原指针比较使 VM 常量池去重路径与 eval 独立分配路径分裂，
    /// 违反 T1；且与本注释「即时值按值」的自述矛盾）。
    pub fn eq_value(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Unit, Value::Unit) => true,
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::Pair(a), Value::Pair(b)) => a == b,
            (Value::Closure(a), Value::Closure(b)) => Rc::ptr_eq(a, b),
            (Value::Builtin(a), Value::Builtin(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

/// 闭包值（双路径形态）。
#[derive(Debug)]
pub enum ClosureValue {
    /// 元循环求值器路径：参数 + 参数绑定作用域集（TD-004/r13，
    /// 与 `params` 平行等长）+ 体表达式 + 定义处环境。
    Eval {
        params: Vec<Symbol>,
        param_scopes: Vec<ScopeSet>,
        body: Rc<CoreExpr>,
        env: Rc<Env>,
    },
    /// 字节码 VM 路径：原型索引 + 捕获单元（共享可变——GcCell
    /// 承载，堆根性摘要标志随写维护）。
    Bytecode {
        proto: u32,
        captures: Vec<Rc<GcCell>>,
    },
}

/// VM 路径共享可变单元（TD-023 对症，r24/42-e）：值 + **堆根性摘要
/// 标志**。标志由写路径（`set`）维护：Pair/Closure → true、其余 →
/// false——是当前值的精确保守摘要（每写必置，sound 不变式）。
/// GC 根集枚举对非堆单元 O(1) 跳过（深帧根扫描实测主导成本——
/// 非尾形基准 2×→3.2-3.7× 超线性的对症面；见 performance-baseline §4）。
#[derive(Debug)]
pub struct GcCell {
    has_heap: Cell<bool>,
    value: RefCell<Value>,
}

impl GcCell {
    pub fn new(v: Value) -> Self {
        let has_heap = matches!(v, Value::Pair(_) | Value::Closure(_));
        GcCell {
            has_heap: Cell::new(has_heap),
            value: RefCell::new(v),
        }
    }

    /// 写入（维护堆根性摘要——TD-023 sound 不变式：每写必置）。
    pub fn set(&self, v: Value) {
        self.has_heap
            .set(matches!(v, Value::Pair(_) | Value::Closure(_)));
        *self.value.borrow_mut() = v;
    }

    /// 读取。
    pub fn get(&self) -> std::cell::Ref<'_, Value> {
        self.value.borrow()
    }

    /// 堆根性（GC 根扫描跳过判据：false ⇒ 当前值必为即时值/内置——
    /// 无堆子引用，无需入根集）。
    pub fn has_heap(&self) -> bool {
        self.has_heap.get()
    }
}

/// 内置函数实现类型（§10.1 规则 2：`-er` 后缀语义）。
pub type BuiltinImpl = Rc<dyn Fn(&mut Heap, Vec<Value>) -> Result<Value, RuntimeError>>;

/// 内置函数（driver 注册；签名接收 `&mut Heap` 以支持序对构造）。
pub struct BuiltinFn {
    /// 名称（诊断与注册键）。
    pub name: &'static str,
    /// 实现。
    pub f: BuiltinImpl,
}

impl std::fmt::Debug for BuiltinFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BuiltinFn({})", self.name)
    }
}

impl BuiltinFn {
    /// 构造。
    pub fn new(
        name: &'static str,
        f: impl Fn(&mut Heap, Vec<Value>) -> Result<Value, RuntimeError> + 'static,
    ) -> Rc<BuiltinFn> {
        Rc::new(BuiltinFn {
            name,
            f: Rc::new(f),
        })
    }

    /// 调用。
    pub fn call(&self, heap: &mut Heap, args: Vec<Value>) -> Result<Value, RuntimeError> {
        (self.f)(heap, args)
    }
}

/// 值渲染（print/write-line 的输出形态；序对经堆遍历）。
pub fn render_value(v: &Value, heap: &Heap) -> String {
    match v {
        Value::Unit => "unit".to_string(),
        Value::Nil => "nil".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => {
            if f.fract() == 0.0 && f.is_finite() {
                format!("{:.1}", f)
            } else {
                f.to_string()
            }
        }
        Value::Str(s) => s.to_string(),
        Value::Symbol(s) => s.to_string(),
        Value::Pair(r) => render_pair(*r, heap, &mut HashMap::new()),
        Value::Closure(_) => "#<procedure>".to_string(),
        Value::Builtin(b) => format!("#<builtin:{}>", b.name),
    }
}

/// 序对渲染（经典 Scheme 列表打印：`(1 2 3)` / 点对 `(1 . 2)`；
/// 自环以 `...` 截断——人类可感知输出，§2.2 原则 16）。
fn render_pair(r: GcRef, heap: &Heap, seen: &mut HashMap<GcRef, ()>) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut cur = r;
    loop {
        if seen.contains_key(&cur) {
            parts.push("...".to_string());
            return format!("({})", parts.join(" "));
        }
        seen.insert(cur, ());
        match heap.get_pair(cur) {
            Some((car, cdr)) => {
                parts.push(render_slot(car, heap));
                // 尾部判定：nil 终止 / 非序对点对 / 序对续行 / 自环截断
                match heap.get_pair(cdr) {
                    None => match slot_terminal(cdr, heap) {
                        Some(rendered) => {
                            if rendered == "nil" {
                                return format!("({})", parts.join(" "));
                            }
                            parts.push(".".to_string());
                            parts.push(rendered);
                            return format!("({})", parts.join(" "));
                        }
                        None => return format!("({})", parts.join(" ")),
                    },
                    Some((_, cdr2)) if cdr2 == cdr => {
                        parts.push(".".to_string());
                        parts.push("...".to_string());
                        return format!("({})", parts.join(" "));
                    }
                    Some(_) => {
                        cur = cdr;
                    }
                }
            }
            None => {
                parts.push(render_slot(cur, heap));
                return format!("({})", parts.join(" "));
            }
        }
    }
}

/// 槽位终止值渲染（nil / 即时值；序对槽返回 None——由调用方续行）。
/// TD-010（r24）：Foreign 槽位（闭包/内置）按函数值渲染。
fn slot_terminal(r: GcRef, heap: &Heap) -> Option<String> {
    match heap.get(r).map(|s| s.obj.clone()) {
        Some(kerf_runtime::HeapObj::Nil) => Some("nil".to_string()),
        Some(kerf_runtime::HeapObj::Int(v)) => Some(v.to_string()),
        Some(kerf_runtime::HeapObj::Float(v)) => Some(v.to_string()),
        Some(kerf_runtime::HeapObj::Bool(b)) => Some(b.to_string()),
        Some(kerf_runtime::HeapObj::Str(s)) => Some(s.to_string()),
        Some(kerf_runtime::HeapObj::Symbol(s)) => Some(s.to_string()),
        Some(kerf_runtime::HeapObj::Foreign(b)) => Some(render_foreign(&b)),
        _ => None,
    }
}

/// 槽位渲染（任意槽位形态）。
fn render_slot(r: GcRef, heap: &Heap) -> String {
    match heap.get(r).map(|s| s.obj.clone()) {
        Some(kerf_runtime::HeapObj::Pair(..)) => render_pair(r, heap, &mut HashMap::new()),
        Some(kerf_runtime::HeapObj::Str(s)) => s.to_string(),
        Some(kerf_runtime::HeapObj::Int(v)) => v.to_string(),
        Some(kerf_runtime::HeapObj::Float(v)) => v.to_string(),
        Some(kerf_runtime::HeapObj::Bool(b)) => b.to_string(),
        Some(kerf_runtime::HeapObj::Symbol(s)) => s.to_string(),
        Some(kerf_runtime::HeapObj::Nil) => "nil".to_string(),
        Some(kerf_runtime::HeapObj::Foreign(b)) => render_foreign(&b),
        None => "#<invalid-slot>".to_string(),
    }
}

/// Foreign 装箱值渲染（TD-010）：与直接函数值渲染同形——闭包
/// `#<procedure>` / 内置 `#<builtin:名>`。
fn render_foreign(b: &Rc<kerf_runtime::ForeignBox>) -> String {
    if b.any.as_ref().downcast_ref::<ClosureValue>().is_some() {
        "#<procedure>".to_string()
    } else if let Some(f) = b.any.as_ref().downcast_ref::<BuiltinFn>() {
        format!("#<builtin:{}>", f.name)
    } else {
        "#<foreign>".to_string()
    }
}

/// PartialEq 经 eq_value 语义（测试断言与容器使用；生产比较请用 eq_value 显式调用）。
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.eq_value(other)
    }
}
