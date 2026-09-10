//! 内置函数注册（stage0.md §9 stdlib：语言核心零内置——全部经 driver 注入）。
//!
//! 算术（+ - * / mod）· 比较（= < > <= >=）· 序对（cons car cdr list）·
//! 谓词（null? pair? int? bool? procedure? eq?）· 逻辑（not）·
//! I/O（print read-line）· 字符串（str-append）。
//!
//! 数值塔：Int×Int → Int（溢出检查）；任一 Float → Float。
//! 比较：链式（(= a b c) 全相等）。
//!
//! 另含自举 Reader 原语（3，B3）：str->pos-chars / char-whitespace? /
//! char-alphabetic? ——服务 reader.krf（见各注册处边界注记）。

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
            // 右折叠构造；空参 → nil 值形态（与 '() 一致——空表即 nil）
            if args.is_empty() {
                return Ok(Value::Nil);
            }
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

    // ------------------------------------------------------------------
    // 标准库最小集扩展（r5，批次 B——07 §3.3 阶段门条件 3：
    // 列表操作/字符串处理/基本 I/O 各 ≥8 函数）
    // ------------------------------------------------------------------

    // —— 列表操作（8）——
    defs.push((
        "length",
        BuiltinFn::new("length", |heap, args| {
            one_arg("length", &args)?;
            let mut n: u64 = 0;
            let mut cur = args[0].clone();
            loop {
                match cur {
                    Value::Nil => return Ok(Value::Int(n as i64)),
                    Value::Pair(r) => match heap.get_pair(r) {
                        Some((_, cdr)) => {
                            n += 1;
                            cur = unbox_slot(cdr, heap);
                        }
                        None => {
                            return Err(RuntimeError::new("length 应用于非序对堆槽"));
                        }
                    },
                    other => {
                        return Err(RuntimeError::new(format!(
                            "length 需要 list（nil 终结的序对链），实际 {}",
                            other.type_name()
                        )))
                    }
                }
            }
        }),
    ));
    defs.push((
        "append",
        BuiltinFn::new("append", |heap, args| {
            // 可变参：0 参 → nil；末参原样（允许 improper）；前参必须 list
            if args.is_empty() {
                return Ok(Value::Nil);
            }
            let last = args[args.len() - 1].clone();
            if args.len() == 1 {
                return Ok(last);
            }
            // 收集前 n-1 个链的元素（扁平化到向量——每参必须 nil 终结）
            let mut elems: Vec<Value> = Vec::new();
            for (i, a) in args[..args.len() - 1].iter().enumerate() {
                let mut cur = a.clone();
                loop {
                    match cur {
                        Value::Nil => break,
                        Value::Pair(r) => match heap.get_pair(r) {
                            Some((car, cdr)) => {
                                elems.push(unbox_slot(car, heap));
                                cur = unbox_slot(cdr, heap);
                            }
                            None => {
                                return Err(RuntimeError::new("append 应用于非序对堆槽"));
                            }
                        },
                        other => {
                            return Err(RuntimeError::new(format!(
                                "append 第 {} 参需要 list，实际 {}",
                                i + 1,
                                other.type_name()
                            )))
                        }
                    }
                }
            }
            // 右折叠重建（新链——不共享前参结构；末参共享尾部）
            let mut acc = box_value(&last, heap);
            for e in elems.iter().rev() {
                let elem = box_value(e, heap);
                acc = heap.alloc_pair(elem, acc);
            }
            Ok(Value::Pair(acc))
        }),
    ));
    defs.push((
        "reverse",
        BuiltinFn::new("reverse", |heap, args| {
            one_arg("reverse", &args)?;
            // 空表恒等返回（nil 值形态——不包装为序对）
            if matches!(args[0], Value::Nil) {
                return Ok(Value::Nil);
            }
            let mut acc = heap.alloc_boxed(BoxedInput::Nil);
            let mut cur = args[0].clone();
            loop {
                match cur {
                    Value::Nil => return Ok(Value::Pair(acc)),
                    Value::Pair(r) => match heap.get_pair(r) {
                        Some((car, cdr)) => {
                            let elem = unbox_slot(car, heap);
                            let elem_r = box_value(&elem, heap);
                            acc = heap.alloc_pair(elem_r, acc);
                            cur = unbox_slot(cdr, heap);
                        }
                        None => {
                            return Err(RuntimeError::new("reverse 应用于非序对堆槽"));
                        }
                    },
                    other => {
                        return Err(RuntimeError::new(format!(
                            "reverse 需要 list，实际 {}",
                            other.type_name()
                        )))
                    }
                }
            }
        }),
    ));
    defs.push((
        "list-ref",
        BuiltinFn::new("list-ref", |heap, args| {
            two_args("list-ref", &args)?;
            let n = match &args[1] {
                Value::Int(i) if *i >= 0 => *i as u64,
                Value::Int(i) => {
                    return Err(RuntimeError::new(format!(
                        "list-ref 索引需要非负整数，实际 {}",
                        i
                    )))
                }
                other => {
                    return Err(RuntimeError::new(format!(
                        "list-ref 索引需要 int，实际 {}",
                        other.type_name()
                    )))
                }
            };
            let mut cur = args[0].clone();
            let mut k = n;
            loop {
                match cur {
                    Value::Pair(r) => match heap.get_pair(r) {
                        Some((car, cdr)) => {
                            if k == 0 {
                                return Ok(unbox_slot(car, heap));
                            }
                            k -= 1;
                            cur = unbox_slot(cdr, heap);
                        }
                        None => {
                            return Err(RuntimeError::new("list-ref 应用于非序对堆槽"));
                        }
                    },
                    _ => {
                        return Err(RuntimeError::new(format!(
                            "list-ref 索引 {} 超出列表范围",
                            n
                        )))
                    }
                }
            }
        }),
    ));
    defs.push((
        "list-tail",
        BuiltinFn::new("list-tail", |heap, args| {
            two_args("list-tail", &args)?;
            let n = match &args[1] {
                Value::Int(i) if *i >= 0 => *i as u64,
                Value::Int(i) => {
                    return Err(RuntimeError::new(format!(
                        "list-tail 起始索引需要非负整数，实际 {}",
                        i
                    )))
                }
                other => {
                    return Err(RuntimeError::new(format!(
                        "list-tail 索引需要 int，实际 {}",
                        other.type_name()
                    )))
                }
            };
            let mut cur = args[0].clone();
            let mut k = n;
            // 每步校验形态（k=0 时非 list 输入也拒绝——类型严格，
            // Racket contract 语义：list-tail 输入必须是 list）
            loop {
                match cur {
                    Value::Nil => {
                        if k == 0 {
                            return Ok(Value::Nil);
                        }
                        return Err(RuntimeError::new(format!(
                            "list-tail 索引 {} 超出列表范围",
                            n
                        )));
                    }
                    Value::Pair(r) => {
                        if k == 0 {
                            return Ok(Value::Pair(r));
                        }
                        match heap.get_pair(r) {
                            Some((_, cdr)) => {
                                k -= 1;
                                cur = unbox_slot(cdr, heap);
                            }
                            None => {
                                return Err(RuntimeError::new("list-tail 应用于非序对堆槽"));
                            }
                        }
                    }
                    other => {
                        return Err(RuntimeError::new(format!(
                            "list-tail 需要 list，实际 {}",
                            other.type_name()
                        )))
                    }
                }
            }
        }),
    ));
    defs.push((
        "member",
        BuiltinFn::new("member", |heap, args| {
            two_args("member", &args)?;
            // 按 eq? 逐元素查找；命中返回子表，未命中返回 false
            let mut cur = args[1].clone();
            loop {
                match cur {
                    Value::Nil => return Ok(Value::Bool(false)),
                    Value::Pair(r) => match heap.get_pair(r) {
                        Some((car, cdr)) => {
                            let elem = unbox_slot(car, heap);
                            if elem.eq_value(&args[0]) {
                                return Ok(Value::Pair(r));
                            }
                            cur = unbox_slot(cdr, heap);
                        }
                        None => {
                            return Err(RuntimeError::new("member 应用于非序对堆槽"));
                        }
                    },
                    other => {
                        return Err(RuntimeError::new(format!(
                            "member 第二参需要 list，实际 {}",
                            other.type_name()
                        )))
                    }
                }
            }
        }),
    ));
    defs.push((
        "assoc",
        BuiltinFn::new("assoc", |heap, args| {
            two_args("assoc", &args)?;
            // 点对表按键 eq? 查找；命中返回该点对（键值对），未命中 false
            let mut cur = args[1].clone();
            loop {
                match cur {
                    Value::Nil => return Ok(Value::Bool(false)),
                    Value::Pair(r) => match heap.get_pair(r) {
                        Some((entry, cdr)) => {
                            let entry_v = unbox_slot(entry, heap);
                            match entry_v {
                                Value::Pair(ep) => match heap.get_pair(ep) {
                                    Some((key, _)) => {
                                        let key_v = unbox_slot(key, heap);
                                        if key_v.eq_value(&args[0]) {
                                            return Ok(Value::Pair(ep));
                                        }
                                        cur = unbox_slot(cdr, heap);
                                    }
                                    None => {
                                        return Err(RuntimeError::new(
                                            "assoc 表项应应用于非序对堆槽",
                                        ))
                                    }
                                },
                                other => {
                                    return Err(RuntimeError::new(format!(
                                        "assoc 表项需要 pair，实际 {}",
                                        other.type_name()
                                    )))
                                }
                            }
                        }
                        None => {
                            return Err(RuntimeError::new("assoc 应用于非序对堆槽"));
                        }
                    },
                    other => {
                        return Err(RuntimeError::new(format!(
                            "assoc 第二参需要 list，实际 {}",
                            other.type_name()
                        )))
                    }
                }
            }
        }),
    ));
    defs.push((
        "last-pair",
        BuiltinFn::new("last-pair", |heap, args| {
            one_arg("last-pair", &args)?;
            let mut cur = args[0].clone();
            loop {
                match cur {
                    Value::Pair(r) => match heap.get_pair(r) {
                        Some((_, cdr)) => {
                            let next = unbox_slot(cdr, heap);
                            if matches!(next, Value::Nil) {
                                return Ok(Value::Pair(r));
                            }
                            cur = next;
                        }
                        None => {
                            return Err(RuntimeError::new("last-pair 应用于非序对堆槽"));
                        }
                    },
                    Value::Nil => {
                        return Err(RuntimeError::new("last-pair 需要非空 list"));
                    }
                    other => {
                        return Err(RuntimeError::new(format!(
                            "last-pair 需要 list，实际 {}",
                            other.type_name()
                        )))
                    }
                }
            }
        }),
    ));

    // —— 字符串处理（10，含 TD-002 联动互转）——
    defs.push((
        "str-length",
        BuiltinFn::new("str-length", |_, args| {
            one_arg("str-length", &args)?;
            match &args[0] {
                Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
                other => Err(RuntimeError::new(format!(
                    "str-length 需要 str，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "str-substring",
        BuiltinFn::new("str-substring", |_, args| {
            if args.len() != 3 {
                return Err(RuntimeError::new("str-substring 需要 3 个参数"));
            }
            let s = match &args[0] {
                Value::Str(s) => s.clone(),
                other => {
                    return Err(RuntimeError::new(format!(
                        "str-substring 需要 str，实际 {}",
                        other.type_name()
                    )))
                }
            };
            let (start, end) = match (&args[1], &args[2]) {
                (Value::Int(a), Value::Int(b)) => (*a, *b),
                (a, _) => {
                    return Err(RuntimeError::new(format!(
                        "str-substring 索引需要 int，实际 {}",
                        a.type_name()
                    )))
                }
            };
            let len = s.chars().count() as i64;
            if start < 0 || end < start || end > len {
                return Err(RuntimeError::new(format!(
                    "str-substring 索引越界：{}..{}（长度 {}）",
                    start, end, len
                )));
            }
            let out: String = s
                .chars()
                .skip(start as usize)
                .take((end - start) as usize)
                .collect();
            Ok(Value::Str(Rc::from(out.as_str())))
        }),
    ));
    defs.push((
        "str-index-of",
        BuiltinFn::new("str-index-of", |_, args| {
            two_args("str-index-of", &args)?;
            match (&args[0], &args[1]) {
                (Value::Str(s), Value::Str(sub)) => {
                    // 字符索引（Unicode 安全：先字节查找再换算字符位）
                    let idx = s
                        .find(sub.as_ref())
                        .map(|b| s[..b].chars().count() as i64)
                        .unwrap_or(-1);
                    Ok(Value::Int(idx))
                }
                (a, b) => Err(RuntimeError::new(format!(
                    "str-index-of 两参都需要 str，实际 {} 与 {}",
                    a.type_name(),
                    b.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "str-contains?",
        BuiltinFn::new("str-contains?", |_, args| {
            two_args("str-contains?", &args)?;
            match (&args[0], &args[1]) {
                (Value::Str(s), Value::Str(sub)) => Ok(Value::Bool(s.contains(sub.as_ref()))),
                (a, b) => Err(RuntimeError::new(format!(
                    "str-contains? 两参都需要 str，实际 {} 与 {}",
                    a.type_name(),
                    b.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "str-prefix?",
        BuiltinFn::new("str-prefix?", |_, args| {
            two_args("str-prefix?", &args)?;
            match (&args[0], &args[1]) {
                (Value::Str(s), Value::Str(pre)) => Ok(Value::Bool(s.starts_with(pre.as_ref()))),
                (a, b) => Err(RuntimeError::new(format!(
                    "str-prefix? 两参都需要 str，实际 {} 与 {}",
                    a.type_name(),
                    b.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "str-suffix?",
        BuiltinFn::new("str-suffix?", |_, args| {
            two_args("str-suffix?", &args)?;
            match (&args[0], &args[1]) {
                (Value::Str(s), Value::Str(suf)) => Ok(Value::Bool(s.ends_with(suf.as_ref()))),
                (a, b) => Err(RuntimeError::new(format!(
                    "str-suffix? 两参都需要 str，实际 {} 与 {}",
                    a.type_name(),
                    b.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "str-upcase",
        BuiltinFn::new("str-upcase", |_, args| {
            one_arg("str-upcase", &args)?;
            match &args[0] {
                Value::Str(s) => {
                    let out: String = s.chars().flat_map(char::to_uppercase).collect();
                    Ok(Value::Str(Rc::from(out.as_str())))
                }
                other => Err(RuntimeError::new(format!(
                    "str-upcase 需要 str，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "str-downcase",
        BuiltinFn::new("str-downcase", |_, args| {
            one_arg("str-downcase", &args)?;
            match &args[0] {
                Value::Str(s) => {
                    let out: String = s.chars().flat_map(char::to_lowercase).collect();
                    Ok(Value::Str(Rc::from(out.as_str())))
                }
                other => Err(RuntimeError::new(format!(
                    "str-downcase 需要 str，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "string->symbol",
        BuiltinFn::new("string->symbol", |_, args| {
            one_arg("string->symbol", &args)?;
            match &args[0] {
                Value::Str(s) => Ok(Value::Symbol(s.clone())),
                other => Err(RuntimeError::new(format!(
                    "string->symbol 需要 str，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "symbol->string",
        BuiltinFn::new("symbol->string", |_, args| {
            one_arg("symbol->string", &args)?;
            match &args[0] {
                Value::Symbol(s) => Ok(Value::Str(s.clone())),
                other => Err(RuntimeError::new(format!(
                    "symbol->string 需要 symbol，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));

    // —— 自举 Reader 原语（3，B3：Reader kerf 重写的运行时服务层）——
    // 边界（§8.4.6 两级语义 + §11）：词法/语法逻辑在 reader.krf（kerf 源码，
    // VM 上运行）；字符级索引与 Unicode 属性判定是运行时原语（与 Racket 的
    // string-ref/char-whitespace? 同层）——不属于语言层内置函数的语义面。
    defs.push((
        "str->pos-chars",
        BuiltinFn::new("str->pos-chars", |heap, args| {
            one_arg("str->pos-chars", &args)?;
            match &args[0] {
                Value::Str(s) => {
                    // 源文本 → ((字节偏移 . 单字符 str) ...) 列表：
                    // 字节偏移经相邻差分可得 UTF-8 长度（末字符用源字节长度）
                    let mut acc = heap.alloc_boxed(BoxedInput::Nil);
                    let pairs: Vec<(usize, char)> = s.char_indices().collect();
                    for (off, ch) in pairs.iter().rev() {
                        let mut buf = [0u8; 4];
                        let ch_str = Value::Str(Rc::from(ch.encode_utf8(&mut buf)));
                        let off_val = Value::Int(*off as i64);
                        let car = box_value(&off_val, heap);
                        let cdr = box_value(&ch_str, heap);
                        let elem = heap.alloc_pair(car, cdr);
                        acc = heap.alloc_pair(elem, acc);
                    }
                    if s.is_empty() {
                        Ok(Value::Nil)
                    } else {
                        Ok(Value::Pair(acc))
                    }
                }
                other => Err(RuntimeError::new(format!(
                    "str->pos-chars 需要 str，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "char-whitespace?",
        BuiltinFn::new("char-whitespace?", |_, args| {
            one_arg("char-whitespace?", &args)?;
            match &args[0] {
                Value::Str(s) => {
                    let mut it = s.chars();
                    let (first, len_ok) = (it.next(), it.next().is_none());
                    match first {
                        Some(c) if len_ok => Ok(Value::Bool(c.is_whitespace())),
                        _ => Err(RuntimeError::new(
                            "char-whitespace? 需要 1 字符 str，实际多字符",
                        )),
                    }
                }
                other => Err(RuntimeError::new(format!(
                    "char-whitespace? 需要 str，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "char-alphabetic?",
        BuiltinFn::new("char-alphabetic?", |_, args| {
            one_arg("char-alphabetic?", &args)?;
            match &args[0] {
                Value::Str(s) => {
                    let mut it = s.chars();
                    let (first, len_ok) = (it.next(), it.next().is_none());
                    match first {
                        Some(c) if len_ok => Ok(Value::Bool(c.is_alphabetic())),
                        _ => Err(RuntimeError::new(
                            "char-alphabetic? 需要 1 字符 str，实际多字符",
                        )),
                    }
                }
                other => Err(RuntimeError::new(format!(
                    "char-alphabetic? 需要 str，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "str-int-valid?",
        BuiltinFn::new("str-int-valid?", |_, args| {
            one_arg("str-int-valid?", &args)?;
            match &args[0] {
                // i64 域判定 = Rust parse 语义原样（宿主类型边界——错误消息
                // 与字节级行为由桥侧同源 parse 保证，kerf 侧仅作扫描序前置
                // 校验以维持错误次序 parity）
                Value::Str(s) => Ok(Value::Bool(s.parse::<i64>().is_ok())),
                other => Err(RuntimeError::new(format!(
                    "str-int-valid? 需要 str，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));

    // —— 基本 I/O（6）——
    defs.push((
        "newline",
        BuiltinFn::new("newline", |_, args| {
            if !args.is_empty() {
                return Err(RuntimeError::new(format!(
                    "newline 需要 0 个参数，实际 {}",
                    args.len()
                )));
            }
            kerf_runtime::write_line_stdout("")?;
            Ok(Value::Nil)
        }),
    ));
    defs.push((
        "write-string",
        BuiltinFn::new("write-string", |_, args| {
            one_arg("write-string", &args)?;
            match &args[0] {
                Value::Str(s) => {
                    kerf_runtime::write_stdout(s)?;
                    Ok(Value::Nil)
                }
                other => Err(RuntimeError::new(format!(
                    "write-string 需要 str，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "read-int",
        BuiltinFn::new("read-int", |_, args| {
            if !args.is_empty() {
                return Err(RuntimeError::new(format!(
                    "read-int 需要 0 个参数，实际 {}",
                    args.len()
                )));
            }
            let line = kerf_runtime::read_line_stdin()?;
            match line {
                None => Ok(Value::Nil),
                Some(s) => match s.trim().parse::<i64>() {
                    Ok(v) => Ok(Value::Int(v)),
                    Err(_) => Err(RuntimeError::new(format!(
                        "read-int 解析失败：非整数字符串「{}」",
                        s.trim()
                    ))),
                },
            }
        }),
    ));
    defs.push((
        "read-num",
        BuiltinFn::new("read-num", |_, args| {
            if !args.is_empty() {
                return Err(RuntimeError::new(format!(
                    "read-num 需要 0 个参数，实际 {}",
                    args.len()
                )));
            }
            let line = kerf_runtime::read_line_stdin()?;
            match line {
                None => Ok(Value::Nil),
                Some(s) => {
                    let t = s.trim();
                    if let Ok(v) = t.parse::<i64>() {
                        return Ok(Value::Int(v));
                    }
                    match t.parse::<f64>() {
                        Ok(v) => Ok(Value::Float(v)),
                        Err(_) => Err(RuntimeError::new(format!(
                            "read-num 解析失败：非数字字符串「{}」",
                            t
                        ))),
                    }
                }
            }
        }),
    ));
    defs.push((
        "error",
        BuiltinFn::new("error", |_, args| {
            if args.is_empty() {
                return Err(RuntimeError::new("error 需要 ≥1 个参数（错误消息）"));
            }
            let mut parts: Vec<String> = Vec::new();
            for a in args {
                // 消息部件：str 原文，其余渲染形式（符号名/数值）
                match a {
                    Value::Str(s) => parts.push(s.to_string()),
                    other => parts.push(other.type_name().to_string()),
                }
            }
            Err(RuntimeError::new(parts.join(" ")))
        }),
    ));
    defs.push((
        "assert-eq?",
        BuiltinFn::new("assert-eq?", |heap, args| {
            two_args("assert-eq?", &args)?;
            if args[0].eq_value(&args[1]) {
                Ok(Value::Bool(true))
            } else {
                Err(RuntimeError::new(format!(
                    "assert-eq? 断言失败：{} ≠ {}",
                    render_value(&args[0], heap),
                    render_value(&args[1], heap)
                )))
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
