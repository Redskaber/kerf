//! 内置函数注册（stage0.md §9 stdlib：语言核心零内置——全部经 driver 注入）。
//!
//! 算术（+ - * / mod）· 比较（= < > <= >=）· 序对（cons car cdr list）·
//! 谓词（null? pair? int? bool? procedure? eq?）· 逻辑（not）·
//! I/O（print read-line）· 字符串（str-append）。
//!
//! 数值塔：Int×Int → Int（溢出检查）；任一 Float → Float。
//! 比较：链式（(= a b c) 全相等）。

use std::collections::HashMap;
use std::rc::Rc;

use kerf_compiler::BcProgram;
use kerf_runtime::{BoxedInput, RuntimeError};
use kerf_syntax::{Symbol, SymbolTable};
use kerf_vm::{box_value, render_value, unbox_slot, BuiltinFn, Value};

/// 注册全部内置函数到全局环境（返回全局表）。
pub fn register_globals(table: &mut SymbolTable) -> HashMap<Symbol, Value> {
    let mut defs: Vec<(&'static str, Rc<BuiltinFn>)> = Vec::new();
    defs.push(("+", arith_builtin("+", Fold::Add, 0)));
    defs.push(("-", arith_builtin("-", Fold::Sub, 1)));
    defs.push(("*", arith_builtin("*", Fold::Mul, 0)));
    defs.push(("/", arith_builtin("/", Fold::Div, 2)));
    defs.push(("mod", arith_builtin("mod", Fold::Mod, 2)));
    defs.push(("=", cmp_builtin("=", Cmp::Eq)));
    defs.push(("<", cmp_builtin("<", Cmp::Lt)));
    defs.push((">", cmp_builtin(">", Cmp::Gt)));
    defs.push(("<=", cmp_builtin("<=", Cmp::Le)));
    defs.push((">=", cmp_builtin(">=", Cmp::Ge)));
    defs.push((
        "cons",
        BuiltinFn::new("cons", |heap, args| {
            two_args("cons", &args)?;
            let car = box_value(&args[0], heap);
            let cdr = box_value(&args[1], heap);
            Ok(Value::Pair(heap.alloc_pair(car, cdr)))
        }),
    ));
    defs.push((
        "car",
        BuiltinFn::new("car", |heap, args| {
            one_arg("car", &args)?;
            match &args[0] {
                Value::Pair(r) => match heap.get_pair(*r) {
                    Some((car, _)) => Ok(unbox_slot(car, heap)),
                    None => Err(RuntimeError::new("car 应用于非序对堆槽")),
                },
                other => Err(RuntimeError::new(format!(
                    "car 需要 pair，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "cdr",
        BuiltinFn::new("cdr", |heap, args| {
            one_arg("cdr", &args)?;
            match &args[0] {
                Value::Pair(r) => match heap.get_pair(*r) {
                    Some((_, cdr)) => Ok(unbox_slot(cdr, heap)),
                    None => Err(RuntimeError::new("cdr 应用于非序对堆槽")),
                },
                other => Err(RuntimeError::new(format!(
                    "cdr 需要 pair，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "list",
        BuiltinFn::new("list", |heap, args| {
            // 右折叠构造
            let mut acc = heap.alloc_boxed(BoxedInput::Nil);
            for a in args.iter().rev() {
                let elem = box_value(a, heap);
                acc = heap.alloc_pair(elem, acc);
            }
            Ok(Value::Pair(acc))
        }),
    ));
    defs.push((
        "null?",
        BuiltinFn::new("null?", |_, args| {
            one_arg("null?", &args)?;
            Ok(Value::Bool(matches!(args[0], Value::Nil)))
        }),
    ));
    defs.push((
        "pair?",
        BuiltinFn::new("pair?", |_, args| {
            one_arg("pair?", &args)?;
            Ok(Value::Bool(matches!(args[0], Value::Pair(_))))
        }),
    ));
    defs.push((
        "int?",
        BuiltinFn::new("int?", |_, args| {
            one_arg("int?", &args)?;
            Ok(Value::Bool(matches!(args[0], Value::Int(_))))
        }),
    ));
    defs.push((
        "bool?",
        BuiltinFn::new("bool?", |_, args| {
            one_arg("bool?", &args)?;
            Ok(Value::Bool(matches!(args[0], Value::Bool(_))))
        }),
    ));
    defs.push((
        "procedure?",
        BuiltinFn::new("procedure?", |_, args| {
            one_arg("procedure?", &args)?;
            Ok(Value::Bool(matches!(
                args[0],
                Value::Closure(_) | Value::Builtin(_)
            )))
        }),
    ));
    defs.push((
        "eq?",
        BuiltinFn::new("eq?", |_, args| {
            two_args("eq?", &args)?;
            Ok(Value::Bool(args[0].eq_value(&args[1])))
        }),
    ));
    defs.push((
        "not",
        BuiltinFn::new("not", |_, args| {
            one_arg("not", &args)?;
            match &args[0] {
                Value::Bool(b) => Ok(Value::Bool(!b)),
                other => Err(RuntimeError::new(format!(
                    "not 需要 bool，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "print",
        BuiltinFn::new("print", |heap, args| {
            one_arg("print", &args)?;
            let rendered = render_value(&args[0], heap);
            kerf_runtime::write_line_stdout(&rendered)?;
            Ok(Value::Nil)
        }),
    ));
    defs.push((
        "read-line",
        BuiltinFn::new(
            "read-line",
            |_, _args| match kerf_runtime::read_line_stdin()? {
                Some(s) => Ok(Value::Str(Rc::from(s.as_str()))),
                None => Ok(Value::Nil),
            },
        ),
    ));
    defs.push((
        "str-append",
        BuiltinFn::new("str-append", |_, args| {
            if args.len() != 2 {
                return Err(RuntimeError::new("str-append 需要 2 个参数"));
            }
            match (&args[0], &args[1]) {
                (Value::Str(a), Value::Str(b)) => {
                    Ok(Value::Str(Rc::from(format!("{}{}", a, b).as_str())))
                }
                _ => Err(RuntimeError::new("str-append 需要 2 个字符串")),
            }
        }),
    ));

    let mut globals = HashMap::new();
    for (name, f) in defs {
        let sym = table.intern(name);
        globals.insert(sym, Value::Builtin(f));
    }
    globals
}

/// 卫生回退解析：`$hyg$N` 后缀剥离（Stage 0 名称基解析的既定近似，见
/// driver 模块文档）。编译产物中未注册的全局名尝试剥离后缀回退。
pub fn resolve_hygiene_fallbacks(
    program: &BcProgram,
    table: &mut SymbolTable,
    globals: &mut HashMap<Symbol, Value>,
) {
    for sym in program.global_refs.iter().copied() {
        if globals.contains_key(&sym) {
            continue;
        }
        let owned_name = table.name(sym).to_string();
        if let Some((base, suffix)) = owned_name.rsplit_once("$hyg$") {
            // 截取剩余后缀必须是数字（保守判定）
            if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
                let base_sym = table.intern(base);
                if let Some(v) = globals.get(&base_sym).cloned() {
                    globals.insert(sym, v);
                }
            }
        }
    }
}

fn one_arg(name: &str, args: &[Value]) -> Result<(), RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(format!(
            "{} 需要 1 个参数，实际 {}",
            name,
            args.len()
        )));
    }
    Ok(())
}

fn two_args(name: &str, args: &[Value]) -> Result<(), RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(format!(
            "{} 需要 2 个参数，实际 {}",
            name,
            args.len()
        )));
    }
    Ok(())
}

#[derive(PartialEq)]
enum Fold {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

fn arith_builtin(name: &'static str, fold: Fold, min_args: usize) -> Rc<BuiltinFn> {
    BuiltinFn::new(name, move |_, args| {
        // 元数与单位元边界（D5/D9 修复，Scheme 惯例）：
        // (+) → 0、(*) → 1（单位元）；(- x) → -x（一元取负）；
        // / 与 mod 无单位元——空参/不足报元数错误（修复前 (+) 直接
        // args[0] 越界 panic；(- 5) 错误地返回 5）。
        if args.is_empty() {
            return match fold {
                Fold::Add => Ok(Value::Int(0)),
                Fold::Mul => Ok(Value::Int(1)),
                _ => Err(RuntimeError::new(format!(
                    "{} 至少需要 {} 个参数",
                    name, min_args
                ))),
            };
        }
        if args.len() < min_args {
            return Err(RuntimeError::new(format!(
                "{} 至少需要 {} 个参数",
                name, min_args
            )));
        }
        if args.len() == 1 && fold == Fold::Sub {
            // 一元取负（D9）：(- x) = 0 - x
            return match &args[0] {
                Value::Int(v) => v
                    .checked_neg()
                    .map(Value::Int)
                    .ok_or_else(|| RuntimeError::new("整数减法溢出")),
                Value::Float(v) => Ok(Value::Float(-v)),
                _ => Err(RuntimeError::new(format!("{} 需要 int", name))),
            };
        }
        let has_float = args.iter().any(|a| matches!(a, Value::Float(_)));
        if has_float {
            // 浮点路径
            let mut acc = args[0]
                .as_number()
                .ok_or_else(|| RuntimeError::new(format!("{} 需要数值", name)))?;
            for a in &args[1..] {
                let v = a
                    .as_number()
                    .ok_or_else(|| RuntimeError::new(format!("{} 需要数值", name)))?;
                acc = match fold {
                    Fold::Add => acc + v,
                    Fold::Sub => acc - v,
                    Fold::Mul => acc * v,
                    Fold::Div => acc / v,
                    Fold::Mod => {
                        return Err(RuntimeError::new("mod 不接受浮点操作数"));
                    }
                };
            }
            Ok(Value::Float(acc))
        } else {
            // 整数路径（溢出检查）
            let mut acc = args[0]
                .as_int()
                .ok_or_else(|| RuntimeError::new(format!("{} 需要 int", name)))?;
            for a in &args[1..] {
                let v = a
                    .as_int()
                    .ok_or_else(|| RuntimeError::new(format!("{} 需要 int", name)))?;
                acc = match fold {
                    Fold::Add => acc
                        .checked_add(v)
                        .ok_or_else(|| RuntimeError::new("整数加法溢出"))?,
                    Fold::Sub => acc
                        .checked_sub(v)
                        .ok_or_else(|| RuntimeError::new("整数减法溢出"))?,
                    Fold::Mul => acc
                        .checked_mul(v)
                        .ok_or_else(|| RuntimeError::new("整数乘法溢出"))?,
                    Fold::Div => {
                        if v == 0 {
                            return Err(RuntimeError::new("整数除零"));
                        }
                        // D4 修复：i64::MIN / -1 溢出 panic → 结构化错误
                        acc.checked_div(v)
                            .ok_or_else(|| RuntimeError::new("整数除法溢出"))?
                    }
                    Fold::Mod => {
                        if v == 0 {
                            return Err(RuntimeError::new("整数取模除零"));
                        }
                        // D4 修复：i64::MIN % -1 溢出 panic → 结构化错误
                        acc.checked_rem(v)
                            .ok_or_else(|| RuntimeError::new("整数取模溢出"))?
                    }
                };
            }
            Ok(Value::Int(acc))
        }
    })
}

enum Cmp {
    Eq,
    Lt,
    Gt,
    Le,
    Ge,
}

fn cmp_builtin(name: &'static str, cmp: Cmp) -> Rc<BuiltinFn> {
    BuiltinFn::new(name, move |_, args| {
        if args.len() < 2 {
            return Err(RuntimeError::new(format!("{} 至少需要 2 个参数", name)));
        }
        // 链式比较：(= a b c) 等价于相邻两两全部成立
        for w in args.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            let ok = match (&cmp, a, b) {
                (Cmp::Eq, Value::Int(x), Value::Int(y)) => x == y,
                (Cmp::Lt, Value::Int(x), Value::Int(y)) => x < y,
                (Cmp::Gt, Value::Int(x), Value::Int(y)) => x > y,
                (Cmp::Le, Value::Int(x), Value::Int(y)) => x <= y,
                (Cmp::Ge, Value::Int(x), Value::Int(y)) => x >= y,
                (Cmp::Eq, Value::Str(x), Value::Str(y)) => x == y,
                (_, Value::Str(_), Value::Str(_)) => {
                    return Err(RuntimeError::new(
                        "字符串仅支持 = 比较（Stage 0 边界，TD-011）",
                    ))
                }
                // _ 臂理由：含 Float 或跨类型的组合——经 as_number 提升为 f64 统一比较，非数值在此报错
                _ => {
                    let x = a
                        .as_number()
                        .ok_or_else(|| RuntimeError::new(format!("{} 需要数值", name)))?;
                    let y = b
                        .as_number()
                        .ok_or_else(|| RuntimeError::new(format!("{} 需要数值", name)))?;
                    match cmp {
                        Cmp::Eq => x == y,
                        Cmp::Lt => x < y,
                        Cmp::Gt => x > y,
                        Cmp::Le => x <= y,
                        Cmp::Ge => x >= y,
                    }
                }
            };
            if !ok {
                return Ok(Value::Bool(false));
            }
        }
        Ok(Value::Bool(true))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globals_have_core_set() {
        let mut t = SymbolTable::new();
        let g = register_globals(&mut t);
        for name in ["+", "-", "cons", "car", "print", "read-line", "eq?"] {
            let sym = t.intern(name);
            assert!(g.contains_key(&sym), "缺少内置 {}", name);
        }
    }

    #[test]
    fn hygiene_fallback_resolves() {
        let mut t = SymbolTable::new();
        let mut g = register_globals(&mut t);
        let hyg = t.intern("+$hyg$3");
        let program = BcProgram {
            protos: vec![],
            consts: vec![],
            entry: 0,
            global_refs: vec![hyg],
            module_name: None,
        };
        resolve_hygiene_fallbacks(&program, &mut t, &mut g);
        assert!(g.contains_key(&hyg), "卫生回退应解析 +$hyg$3 → +");
    }

    #[test]
    fn hygiene_fallback_rejects_non_suffix() {
        let mut t = SymbolTable::new();
        let mut g = register_globals(&mut t);
        let not_hyg = t.intern("some-user-name");
        let program = BcProgram {
            protos: vec![],
            consts: vec![],
            entry: 0,
            global_refs: vec![not_hyg],
            module_name: None,
        };
        resolve_hygiene_fallbacks(&program, &mut t, &mut g);
        assert!(!g.contains_key(&not_hyg), "普通未绑定名不得被静默解析");
    }
}
