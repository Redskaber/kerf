//! 内置函数注册（stage0.md §9 stdlib：语言核心零内置——全部经 driver 注入）。
//!
//! 算术（+ - * / mod）· 比较（= < > <= >=）· 序对（cons head tail list）·
//! 谓词（is-nil is-pair is-int is-bool is-procedure eq）· 逻辑（not）·
//! I/O（print read-line）· 字符串（string-append）。
//!
//! 数值塔：Int×Int → Int（溢出检查）；任一 Float → Float。
//! 比较：链式（(= a b c) 全相等）。
//!
//! 另含自举 Reader 原语（4，B3）：str->pos-chars / char-whitespace? /
//! char-alphabetic? / str-int-valid? ——服务 reader.krf（见各注册处边界注记；
//! 命名属 v0.4 遗留口径——现代化方向见 docs/lang-design/20-surface-conventions.md；
//! 域划分架构（八域四要素/副作用汇聚——「门控表 = I/O 域全集」的根据）见
//! docs/lang-design/21-capability-architecture.md §3；命名机制（五层 N0-N4/
//! 解析/权限矩阵——v0.6 批次 M 实施输入）见 docs/lang-design/22-namespace-design.md）。
//!
//! **批次 L（v0.5 别名层，r38）**：`BUILTIN_ALIASES` 27 现代扁平名双注册
//! （注册面 57→84——20 §6.4 ①/§8 映射表实施单源）；别名共享旧名同一
//! 分派体（同行为同诊断——天然 parity 20 §9.3）；I/O 六门控名零新名
//! （门控表零变更——20 §6.4 ③）。
//!
//! **批次 M 首件 M1（v0.6 命名空间层，r39）**：`STDLIB_MODULES` 七模块
//! export 面注册（N2 限定名可见面——`ns/本地名` 形态，47 限定名，
//! 20 §5.2 单源转译）；io 模块限定名按授权面 fail-closed 注册（22 §5.1
//! 矩阵唯一对齐点）；E0014 不导出/E0015 保留域由 driver 编译期验证承载
//! （verify_qualified_refs——22 §3.3 R-N3 不回落）；限定名签名在
//! `builtin_sigs` 按底层名派生（同分派 → 同静态检查面）。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kerf_compiler::{BcProgram, BuiltinSig, TcParam, TcType};
use kerf_runtime::{BoxedInput, GcRef, Heap, RuntimeError};
use kerf_syntax::{Symbol, SymbolTable};
use kerf_vm::{box_value, render_value, unbox_slot, BuiltinFn, Value};

use crate::capability::{IoGrant, StdCapabilityIO};
use crate::reserved::{CapabilityIO, ReadCapability, WriteCapability};

/// 注册全部内置函数到全局环境（返回全局表）。
pub fn register_globals(table: &mut SymbolTable, grant: &IoGrant) -> HashMap<Symbol, Value> {
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
            let head = box_value(&args[0], heap);
            let tail = box_value(&args[1], heap);
            Ok(Value::Pair(heap.alloc_pair(head, tail)))
        }),
    ));
    defs.push((
        "head",
        BuiltinFn::new("head", |heap, args| {
            one_arg("head", &args)?;
            match &args[0] {
                Value::Pair(r) => match heap.get_pair(*r) {
                    Some((head, _)) => Ok(unbox_slot(head, heap)),
                    None => Err(RuntimeError::new("head 应用于非序对堆槽")),
                },
                other => Err(RuntimeError::new(kerf_vm::err_pair_op(
                    "head",
                    other.type_name(),
                ))),
            }
        }),
    ));
    defs.push((
        "tail",
        BuiltinFn::new("tail", |heap, args| {
            one_arg("tail", &args)?;
            match &args[0] {
                Value::Pair(r) => match heap.get_pair(*r) {
                    Some((_, tail)) => Ok(unbox_slot(tail, heap)),
                    None => Err(RuntimeError::new("tail 应用于非序对堆槽")),
                },
                other => Err(RuntimeError::new(kerf_vm::err_pair_op(
                    "tail",
                    other.type_name(),
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
        "is-nil",
        BuiltinFn::new("is-nil", |_, args| {
            one_arg("is-nil", &args)?;
            Ok(Value::Bool(matches!(args[0], Value::Nil)))
        }),
    ));
    defs.push((
        "is-pair",
        BuiltinFn::new("is-pair", |_, args| {
            one_arg("is-pair", &args)?;
            Ok(Value::Bool(matches!(args[0], Value::Pair(_))))
        }),
    ));
    defs.push((
        "is-int",
        BuiltinFn::new("is-int", |_, args| {
            one_arg("is-int", &args)?;
            Ok(Value::Bool(matches!(args[0], Value::Int(_))))
        }),
    ));
    defs.push((
        "is-bool",
        BuiltinFn::new("is-bool", |_, args| {
            one_arg("is-bool", &args)?;
            Ok(Value::Bool(matches!(args[0], Value::Bool(_))))
        }),
    ));
    defs.push((
        "is-procedure",
        BuiltinFn::new("is-procedure", |_, args| {
            one_arg("is-procedure", &args)?;
            Ok(Value::Bool(matches!(
                args[0],
                Value::Closure(_) | Value::Builtin(_)
            )))
        }),
    ));
    // ---- 类型谓词完备面（r24 / 42-e stdlib 缺口补齐：Value 变体判别）----
    defs.push((
        "is-string",
        BuiltinFn::new("is-string", |_, args| {
            one_arg("is-string", &args)?;
            Ok(Value::Bool(matches!(args[0], Value::Str(_))))
        }),
    ));
    defs.push((
        "is-symbol",
        BuiltinFn::new("is-symbol", |_, args| {
            one_arg("is-symbol", &args)?;
            Ok(Value::Bool(matches!(args[0], Value::Symbol(_))))
        }),
    ));
    defs.push((
        "is-float",
        BuiltinFn::new("is-float", |_, args| {
            one_arg("is-float", &args)?;
            Ok(Value::Bool(matches!(args[0], Value::Float(_))))
        }),
    ));
    defs.push((
        "is-number",
        BuiltinFn::new("is-number", |_, args| {
            one_arg("is-number", &args)?;
            // 数值塔谓词：Int 或 Float（与算术操作数域一致）
            Ok(Value::Bool(matches!(
                args[0],
                Value::Int(_) | Value::Float(_)
            )))
        }),
    ));
    defs.push((
        "is-list",
        BuiltinFn::new("is-list", |heap, args| {
            one_arg("is-list", &args)?;
            // 真表判定：nil 或 tail 链终止于 nil 的序对链。Floyd 龟兔
            // 环检测（环 → false：真表的 tail 链无环；引用 Racket is-list 语义）。
            match &args[0] {
                Value::Nil => Ok(Value::Bool(true)),
                Value::Pair(start) => {
                    let mut slow = *start;
                    let mut fast = *start;
                    loop {
                        // fast 两步 / slow 一步——每步经槽位种类判定
                        for _ in 0..2 {
                            match list_step(heap, fast) {
                                ListStep::End(is_list_end) => return Ok(Value::Bool(is_list_end)),
                                ListStep::Next(r) => fast = r,
                            }
                        }
                        match list_step(heap, slow) {
                            ListStep::End(is_list_end) => return Ok(Value::Bool(is_list_end)),
                            ListStep::Next(r) => slow = r,
                        }
                        if slow == fast {
                            return Ok(Value::Bool(false));
                        }
                    }
                }
                _ => Ok(Value::Bool(false)),
            }
        }),
    ));
    defs.push((
        "eq",
        BuiltinFn::new("eq", |_, args| {
            two_args("eq", &args)?;
            Ok(Value::Bool(args[0].eq_value(&args[1])))
        }),
    ));
    defs.push((
        "not",
        BuiltinFn::new("not", |_, args| {
            one_arg("not", &args)?;
            match &args[0] {
                Value::Bool(b) => Ok(Value::Bool(!b)),
                other => Err(RuntimeError::new(kerf_vm::err_not_bool(other.type_name()))),
            }
        }),
    ));
    // print / read-line 能力参数化注册（r8——13 §3.1.3 条款 4）：
    // 移至 register_io_globals，按 IoGrant 令牌捕获形态注册（未授权
    // 不注册——fail-closed：无令牌的 I/O 无可达入口）
    register_io_globals(&mut defs, grant);
    defs.push((
        "string-append",
        BuiltinFn::new("string-append", |_, args| {
            if args.len() != 2 {
                return Err(RuntimeError::new("string-append 需要 2 个参数"));
            }
            match (&args[0], &args[1]) {
                (Value::Str(a), Value::Str(b)) => {
                    Ok(Value::Str(Rc::from(format!("{}{}", a, b).as_str())))
                }
                _ => Err(RuntimeError::new("string-append 需要 2 个字符串")),
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
                        Some((_, tail)) => {
                            n += 1;
                            cur = unbox_slot(tail, heap);
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
                            Some((head, tail)) => {
                                elems.push(unbox_slot(head, heap));
                                cur = unbox_slot(tail, heap);
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
                        Some((head, tail)) => {
                            let elem = unbox_slot(head, heap);
                            let elem_r = box_value(&elem, heap);
                            acc = heap.alloc_pair(elem_r, acc);
                            cur = unbox_slot(tail, heap);
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
        "nth",
        BuiltinFn::new("nth", |heap, args| {
            two_args("nth", &args)?;
            let n = match &args[1] {
                Value::Int(i) if *i >= 0 => *i as u64,
                Value::Int(i) => {
                    return Err(RuntimeError::new(format!(
                        "nth 索引需要非负整数，实际 {}",
                        i
                    )))
                }
                other => {
                    return Err(RuntimeError::new(format!(
                        "nth 索引需要 int，实际 {}",
                        other.type_name()
                    )))
                }
            };
            let mut cur = args[0].clone();
            let mut k = n;
            loop {
                match cur {
                    Value::Pair(r) => match heap.get_pair(r) {
                        Some((head, tail)) => {
                            if k == 0 {
                                return Ok(unbox_slot(head, heap));
                            }
                            k -= 1;
                            cur = unbox_slot(tail, heap);
                        }
                        None => {
                            return Err(RuntimeError::new("nth 应用于非序对堆槽"));
                        }
                    },
                    _ => return Err(RuntimeError::new(format!("nth 索引 {} 超出列表范围", n))),
                }
            }
        }),
    ));
    defs.push((
        "drop",
        BuiltinFn::new("drop", |heap, args| {
            two_args("drop", &args)?;
            let n = match &args[1] {
                Value::Int(i) if *i >= 0 => *i as u64,
                Value::Int(i) => {
                    return Err(RuntimeError::new(format!(
                        "drop 起始索引需要非负整数，实际 {}",
                        i
                    )))
                }
                other => {
                    return Err(RuntimeError::new(format!(
                        "drop 索引需要 int，实际 {}",
                        other.type_name()
                    )))
                }
            };
            let mut cur = args[0].clone();
            let mut k = n;
            // 每步校验形态（k=0 时非 list 输入也拒绝——类型严格，
            // Racket contract 语义：drop 输入必须是 list）
            loop {
                match cur {
                    Value::Nil => {
                        if k == 0 {
                            return Ok(Value::Nil);
                        }
                        return Err(RuntimeError::new(format!("drop 索引 {} 超出列表范围", n)));
                    }
                    Value::Pair(r) => {
                        if k == 0 {
                            return Ok(Value::Pair(r));
                        }
                        match heap.get_pair(r) {
                            Some((_, tail)) => {
                                k -= 1;
                                cur = unbox_slot(tail, heap);
                            }
                            None => {
                                return Err(RuntimeError::new("drop 应用于非序对堆槽"));
                            }
                        }
                    }
                    other => {
                        return Err(RuntimeError::new(format!(
                            "drop 需要 list，实际 {}",
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
            // 按 eq 逐元素查找；命中返回子表，未命中返回 false
            let mut cur = args[1].clone();
            loop {
                match cur {
                    Value::Nil => return Ok(Value::Bool(false)),
                    Value::Pair(r) => match heap.get_pair(r) {
                        Some((head, tail)) => {
                            let elem = unbox_slot(head, heap);
                            if elem.eq_value(&args[0]) {
                                return Ok(Value::Pair(r));
                            }
                            cur = unbox_slot(tail, heap);
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
            // 点对表按键 eq 查找；命中返回该点对（键值对），未命中 false
            let mut cur = args[1].clone();
            loop {
                match cur {
                    Value::Nil => return Ok(Value::Bool(false)),
                    Value::Pair(r) => match heap.get_pair(r) {
                        Some((entry, tail)) => {
                            let entry_v = unbox_slot(entry, heap);
                            match entry_v {
                                Value::Pair(ep) => match heap.get_pair(ep) {
                                    Some((key, _)) => {
                                        let key_v = unbox_slot(key, heap);
                                        if key_v.eq_value(&args[0]) {
                                            return Ok(Value::Pair(ep));
                                        }
                                        cur = unbox_slot(tail, heap);
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
                        Some((_, tail)) => {
                            let next = unbox_slot(tail, heap);
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
        "string-length",
        BuiltinFn::new("string-length", |_, args| {
            one_arg("string-length", &args)?;
            match &args[0] {
                Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
                other => Err(RuntimeError::new(format!(
                    "string-length 需要 str，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "string-substring",
        BuiltinFn::new("string-substring", |_, args| {
            if args.len() != 3 {
                return Err(RuntimeError::new("string-substring 需要 3 个参数"));
            }
            let s = match &args[0] {
                Value::Str(s) => s.clone(),
                other => {
                    return Err(RuntimeError::new(format!(
                        "string-substring 需要 str，实际 {}",
                        other.type_name()
                    )))
                }
            };
            let (start, end) = match (&args[1], &args[2]) {
                (Value::Int(a), Value::Int(b)) => (*a, *b),
                (a, _) => {
                    return Err(RuntimeError::new(format!(
                        "string-substring 索引需要 int，实际 {}",
                        a.type_name()
                    )))
                }
            };
            let len = s.chars().count() as i64;
            if start < 0 || end < start || end > len {
                return Err(RuntimeError::new(format!(
                    "string-substring 索引越界：{}..{}（长度 {}）",
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
        "string-index-of",
        BuiltinFn::new("string-index-of", |_, args| {
            two_args("string-index-of", &args)?;
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
                    "string-index-of 两参都需要 str，实际 {} 与 {}",
                    a.type_name(),
                    b.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "string-contains",
        BuiltinFn::new("string-contains", |_, args| {
            two_args("string-contains", &args)?;
            match (&args[0], &args[1]) {
                (Value::Str(s), Value::Str(sub)) => Ok(Value::Bool(s.contains(sub.as_ref()))),
                (a, b) => Err(RuntimeError::new(format!(
                    "string-contains 两参都需要 str，实际 {} 与 {}",
                    a.type_name(),
                    b.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "string-starts-with",
        BuiltinFn::new("string-starts-with", |_, args| {
            two_args("string-starts-with", &args)?;
            match (&args[0], &args[1]) {
                (Value::Str(s), Value::Str(pre)) => Ok(Value::Bool(s.starts_with(pre.as_ref()))),
                (a, b) => Err(RuntimeError::new(format!(
                    "string-starts-with 两参都需要 str，实际 {} 与 {}",
                    a.type_name(),
                    b.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "string-ends-with",
        BuiltinFn::new("string-ends-with", |_, args| {
            two_args("string-ends-with", &args)?;
            match (&args[0], &args[1]) {
                (Value::Str(s), Value::Str(suf)) => Ok(Value::Bool(s.ends_with(suf.as_ref()))),
                (a, b) => Err(RuntimeError::new(format!(
                    "string-ends-with 两参都需要 str，实际 {} 与 {}",
                    a.type_name(),
                    b.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "string-to-upper",
        BuiltinFn::new("string-to-upper", |_, args| {
            one_arg("string-to-upper", &args)?;
            match &args[0] {
                Value::Str(s) => {
                    let out: String = s.chars().flat_map(char::to_uppercase).collect();
                    Ok(Value::Str(Rc::from(out.as_str())))
                }
                other => Err(RuntimeError::new(format!(
                    "string-to-upper 需要 str，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "string-to-lower",
        BuiltinFn::new("string-to-lower", |_, args| {
            one_arg("string-to-lower", &args)?;
            match &args[0] {
                Value::Str(s) => {
                    let out: String = s.chars().flat_map(char::to_lowercase).collect();
                    Ok(Value::Str(Rc::from(out.as_str())))
                }
                other => Err(RuntimeError::new(format!(
                    "string-to-lower 需要 str，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "string-to-symbol",
        BuiltinFn::new("string-to-symbol", |_, args| {
            one_arg("string-to-symbol", &args)?;
            match &args[0] {
                Value::Str(s) => Ok(Value::Symbol(s.clone())),
                other => Err(RuntimeError::new(format!(
                    "string-to-symbol 需要 str，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));
    defs.push((
        "symbol-to-string",
        BuiltinFn::new("symbol-to-string", |_, args| {
            one_arg("symbol-to-string", &args)?;
            match &args[0] {
                Value::Symbol(s) => Ok(Value::Str(s.clone())),
                other => Err(RuntimeError::new(format!(
                    "symbol-to-string 需要 symbol，实际 {}",
                    other.type_name()
                ))),
            }
        }),
    ));

    // —— 自举 Reader 原语（4，B3：Reader kerf 重写的运行时服务层）——
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
                        let head = box_value(&off_val, heap);
                        let tail = box_value(&ch_str, heap);
                        let elem = heap.alloc_pair(head, tail);
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

    // —— 基本 I/O（能力门控 6 项全部移至 register_io_globals——r8）——
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
        "assert-eq",
        BuiltinFn::new("assert-eq", |heap, args| {
            two_args("assert-eq", &args)?;
            if args[0].eq_value(&args[1]) {
                Ok(Value::Bool(true))
            } else {
                Err(RuntimeError::new(format!(
                    "assert-eq 断言失败：{} ≠ {}",
                    render_value(&args[0], heap),
                    render_value(&args[1], heap)
                )))
            }
        }),
    ));

    // ------------------------------------------------------------------
    // r42 / 63-b 移除轮 S1 表面腿（v0.9）——旧名 27 件退役：批次 L
    // 别名双注册机制下线（20 §7 移除轮行兑现；注册面 84→57 扁平）。
    // 旧名引用 → E0021 编译期错误（driver `verify_qualified_refs` 内
    // REMOVED_BUILTIN_NAMES 单源判定——诊断携现代名指引）；弃用期
    // 生命周期核对（23 §3.4：W1001 落地 v0.7 → v0.8 一稳定版全绿 →
    // v0.9 移除，跨两版本）。
    // ------------------------------------------------------------------

    let mut globals = HashMap::new();
    for (name, f) in defs {
        let sym = table.intern(name);
        globals.insert(sym, Value::Builtin(f));
    }
    // ------------------------------------------------------------------
    // 批次 M 首件 M1（v0.6 命名空间层，r39）——N2 限定名可见面：
    // 七模块 export 面（47 限定名 `ns/本地名` → 底层共享分派体）。
    // 机制 = 22 §2.1 N2「限定名可见面」+ §3.3 R-N3（不回落：解析仅
    // 查 export 面——E0014 诊断由 driver verify_qualified_refs 编译期
    // 承载）。io 模块 fail-closed：按授权分项注册（22 §5.1 矩阵唯一
    // 对齐点「门控全集 ≡ 导出全集」+ 红线 3 缺省拒绝）。
    // ------------------------------------------------------------------
    for &(ns, local, underlying) in STDLIB_MODULES {
        if ns == "io" {
            // io 限定名：仅当对应授权分项在场才注册（fail-closed——
            // 未授权限定名不在册；编译期 E0006 由 R9 归一化先行阻断）
            let need_read = crate::capability::READ_GATED.contains(&underlying);
            let need_write = crate::capability::WRITE_GATED.contains(&underlying);
            let authorized = if need_read {
                grant.read_handle().is_some()
            } else if need_write {
                grant.write_handle().is_some()
            } else {
                true
            };
            if !authorized {
                continue;
            }
        }
        let u = table.intern(underlying);
        let f = match globals.get(&u) {
            Some(Value::Builtin(f)) => Rc::clone(f),
            _ => panic!("限定名底层内置未注册：{}/{} → {}", ns, local, underlying),
        };
        let qualified = format!("{}/{}", ns, local);
        // --------------------------------------------------------------
        // 批次 M 次件 M2（v0.6，r40）——B1/B2 行为契约现代化（20 §4）：
        // 限定名走**现代契约**（miss→nil）——独立分派体不与底层共享
        // （「新名新契约、旧名旧契约并存至移除」——20 §4 迁移不变量；
        // 旧名/扁平新名维持旧契约，r38 parity 锚不动）。B3（io/read-line
        // 严格 0 参）底层已对齐（FS-4 修复面——20 §4 B3 行实测注记的
        // R4 修正：纯测试锚，零代码变更）。
        // --------------------------------------------------------------
        let contracted: Option<Rc<BuiltinFn>> = match (ns, local) {
            // B1：string/index-of 未找到 -1 → nil（nil-as-absence——
            // read-int EOF→nil 先例同型；哨兵值是错误码时代遗产）
            ("string", "index-of") => {
                let inner = Rc::clone(&f);
                Some(BuiltinFn::new(
                    "string/index-of",
                    move |heap, args| match inner.call(heap, args) {
                        Ok(Value::Int(-1)) => Ok(Value::Nil),
                        other => other,
                    },
                ))
            }
            // B2：list/member 与 list/assoc 未命中 false → nil（返回类型
            // 单一化：表-or-nil，不再与 bool 交叠——配合 HM 类型域纯净化）
            ("list", "member") => {
                let inner = Rc::clone(&f);
                Some(BuiltinFn::new(
                    "list/member",
                    move |heap, args| match inner.call(heap, args) {
                        Ok(Value::Bool(false)) => Ok(Value::Nil),
                        other => other,
                    },
                ))
            }
            ("list", "assoc") => {
                let inner = Rc::clone(&f);
                Some(BuiltinFn::new(
                    "list/assoc",
                    move |heap, args| match inner.call(heap, args) {
                        Ok(Value::Bool(false)) => Ok(Value::Nil),
                        other => other,
                    },
                ))
            }
            _ => None,
        };
        let value = match contracted {
            Some(f2) => Value::Builtin(f2),
            None => Value::Builtin(f),
        };
        globals.insert(table.intern(&qualified), value);
    }
    globals
}

/// r42 / 63-b 移除轮 S1（v0.9）：旧名退役表（27 件——方向 =
/// (旧名, 现代名)）。
///
/// 单源 = docs/lang-design/20-surface-conventions.md §8（57 项映射表
/// ——r38 起冻结）。旧名引用 → E0021 编译期错误（诊断携现代名指引
/// ——比裸 E0004 未绑定更 actionable：20 §7「旧名引用 = E00xx 错误」
/// 兑现）。判定消费方 = driver `verify_qualified_refs`（值位/操作位
/// 全域——与 E0014 同一 traversal，遮蔽/接管豁免同口径）。闭合守卫
/// `removed_names_closed_and_modern_registered`（本文件测试）：27
/// 对双向对账零缺零溢 + 旧名零注册 + 现代名全注册。I/O 六门控名
/// 零旧名（门控表零变更维持——r38 实测口径）。v0.5-v0.8 双注册期
/// 终结（批次 L r38 引入 → W1001 弃用 r40 → 移除 r42——23 §3.4
/// 生命周期四阶段完整走完）。
pub static REMOVED_BUILTIN_NAMES: &[(&str, &str)] = &[
    // 访问器（R1：head/tail 终态）
    ("car", "head"),
    ("cdr", "tail"),
    // 列表（nth←list-ref、drop←list-tail）
    ("list-ref", "nth"),
    ("list-tail", "drop"),
    // 谓词族（R2：`is-命名` 终态）
    ("null?", "is-nil"),
    ("pair?", "is-pair"),
    ("int?", "is-int"),
    ("bool?", "is-bool"),
    ("procedure?", "is-procedure"),
    ("string?", "is-string"),
    ("symbol?", "is-symbol"),
    ("float?", "is-float"),
    ("number?", "is-number"),
    ("list?", "is-list"),
    // eq（R3：eq 终态——恰 2 参）
    ("eq?", "eq"),
    // 字符串族（R1 全词化 + R3 动词化终态）
    ("str-append", "string-append"),
    ("str-length", "string-length"),
    ("str-substring", "string-substring"),
    ("str-index-of", "string-index-of"),
    ("str-contains?", "string-contains"),
    ("str-prefix?", "string-starts-with"),
    ("str-suffix?", "string-ends-with"),
    // 转换族（R4：`a-to-b` 终态）
    ("str-upcase", "string-to-upper"),
    ("str-downcase", "string-to-lower"),
    ("string->symbol", "string-to-symbol"),
    ("symbol->string", "symbol-to-string"),
    // 断言（R3：断言是动词）
    ("assert-eq?", "assert-eq"),
];

/// W1003 宏名遮蔽判定（r42 / S1——22 §11 D11 排期移除轮同窗兑现）：
/// 宏名 ∈ 内置注册面（57 扁平 + 47 限定）→ W 级知会。消费方 = driver
/// `collect_warnings`（Stx 层宏名收集——宏展开后名字从 CoreExpr 消失）。
/// 数据源 = `BUILTIN_SIGS`（56）+ `read-line`（运行时不检查元数不列
/// 签名——注册面补全）+ `STDLIB_MODULES` 限定名派生（47）。
pub(crate) fn builtin_name_exists(name: &str) -> bool {
    if name == "read-line" {
        return true; // 57 扁平名补全（SIGS 56 + read-line）
    }
    if BUILTIN_SIGS.iter().any(|&(n, _)| n == name) {
        return true;
    }
    STDLIB_MODULES
        .iter()
        .any(|&(ns, local, _)| name == format!("{}/{}", ns, local))
}

/// 批次 M 首件 M1（v0.6 命名空间层，r39）标准库模块表：七模块 export 面。
///
/// 行 = (命名空间段, 本地名, 底层内置名)——限定名 `ns/本地名` 注册为
/// 与底层共享同一 `Rc<BuiltinFn>`（同 r38 别名层 parity 形态）。单源 =
/// docs/lang-design/20-surface-conventions.md §5.2（模块树终态版图）；
/// 机制规则 = 22-namespace-design §2-§3（N2 限定名可见面 + R-N3 不
/// 回落——E0014 编译期诊断由 driver `verify_qualified_refs` 承载）。
/// `io` 模块限定名按授权面 fail-closed 注册（22 §5.1 矩阵唯一对齐
/// 点）；运算符族 N1 永驻不入表（R-N7——「core 不导出全局运算符」
/// 20 §8 裁定）；Reader 运行时服务四件（`str->pos-chars` 等）引导
/// 私有不入用户面（20 §5.2 注记）。M2（r40）承载 import 注入面/别名
/// （R-N5）与 R-N1/N2 全序——本表 M1 先落「限定名可见面」。
pub static STDLIB_MODULES: &[(&str, &str, &str)] = &[
    // kerf/core（14）——值谓词域 + 关系域（20 §5.2 首行）
    ("core", "is-nil", "is-nil"),
    ("core", "is-bool", "is-bool"),
    ("core", "is-int", "is-int"),
    ("core", "is-float", "is-float"),
    ("core", "is-number", "is-number"),
    ("core", "is-string", "is-string"),
    ("core", "is-symbol", "is-symbol"),
    ("core", "is-pair", "is-pair"),
    ("core", "is-list", "is-list"),
    ("core", "is-procedure", "is-procedure"),
    ("core", "eq", "eq"),
    ("core", "not", "not"),
    ("core", "error", "error"),
    ("core", "assert-eq", "assert-eq"),
    // kerf/pair（3）——序对域
    ("pair", "cons", "cons"),
    ("pair", "head", "head"),
    ("pair", "tail", "tail"),
    // kerf/list（9）——表域（nth←list-ref、drop←list-tail）
    ("list", "list", "list"),
    ("list", "length", "length"),
    ("list", "append", "append"),
    ("list", "reverse", "reverse"),
    ("list", "nth", "nth"),
    ("list", "drop", "drop"),
    ("list", "member", "member"),
    ("list", "assoc", "assoc"),
    ("list", "last-pair", "last-pair"),
    // kerf/string（11）——字符串域（R4 双向：to-symbol/from-symbol）
    ("string", "append", "string-append"),
    ("string", "length", "string-length"),
    ("string", "substring", "string-substring"),
    ("string", "index-of", "string-index-of"),
    ("string", "contains", "string-contains"),
    ("string", "starts-with", "string-starts-with"),
    ("string", "ends-with", "string-ends-with"),
    ("string", "to-upper", "string-to-upper"),
    ("string", "to-lower", "string-to-lower"),
    ("string", "to-symbol", "string-to-symbol"),
    ("string", "from-symbol", "symbol-to-string"),
    // kerf/symbol（2）——转换域 R4 双向双家（与 string 侧对偶）
    ("symbol", "to-string", "symbol-to-string"),
    ("symbol", "from-string", "string-to-symbol"),
    // kerf/io（6）——I/O 域（唯一对齐点：门控全集 ≡ 导出全集）
    ("io", "print", "print"),
    ("io", "write-string", "write-string"),
    ("io", "newline", "newline"),
    ("io", "read-line", "read-line"),
    ("io", "read-int", "read-int"),
    ("io", "read-num", "read-num"),
    // kerf/char（2）——Unicode 属性判定域（与引导私有的边界裁定 20 §5.2）
    ("char", "is-whitespace", "char-whitespace?"),
    ("char", "is-alphabetic", "char-alphabetic?"),
];

/// I/O 内置的能力参数化注册（r8——13 §3.1.3 条款 4「driver 注册的
/// 内置函数改为能力参数化形态」）。
///
/// **门控语义**：令牌句柄存在才注册（授权面 = 声明面，R9 编译期已对
/// 齐）；闭包捕获 `Rc<RefCell<令牌>>`（F5 线性近似——borrow_mut 互
/// 斥 = 调用期独占）。错误消息与 r7 逐字兼容（既有负向断言锚定）；
/// I/O 副作用全部经冻结契约 `StdCapabilityIO`（规格条款 1/2——令牌
/// 线性占用签名位）。
fn register_io_globals(defs: &mut Vec<(&'static str, Rc<BuiltinFn>)>, grant: &IoGrant) {
    if let Some(write_cap) = grant.write_handle() {
        // —— print：渲染 + 写行（write 能力）——
        defs.push((
            "print",
            BuiltinFn::new("print", {
                let cap = write_cap.clone();
                move |heap, args| {
                    one_arg("print", &args)?;
                    let rendered = render_value(&args[0], heap);
                    cap_write_line(&cap, &rendered)?;
                    Ok(Value::Nil)
                }
            }),
        ));
        // —— newline：写空行（write 能力）——
        defs.push((
            "newline",
            BuiltinFn::new("newline", {
                let cap = write_cap.clone();
                move |_, args| {
                    if !args.is_empty() {
                        return Err(RuntimeError::new(format!(
                            "newline 需要 0 个参数，实际 {}",
                            args.len()
                        )));
                    }
                    cap_write_line(&cap, "")?;
                    Ok(Value::Nil)
                }
            }),
        ));
        // —— write-string：无换行写（write 能力；OS 边界直写——
        // 冻结契约仅覆盖 write_line，write_string 为 r5 通道层扩展）——
        defs.push((
            "write-string",
            BuiltinFn::new("write-string", {
                let cap = write_cap.clone();
                move |_, args| {
                    one_arg("write-string", &args)?;
                    match &args[0] {
                        Value::Str(text) => {
                            // 令牌占用校验（能力在签名——写入经授权面）
                            let _ = cap.try_borrow_mut().map_err(|_| {
                                RuntimeError::new("写能力令牌被并发占用（线性违反）")
                            })?;
                            kerf_runtime::write_stdout(text)?;
                            Ok(Value::Nil)
                        }
                        other => Err(RuntimeError::new(format!(
                            "write-string 需要 str，实际 {}",
                            other.type_name()
                        ))),
                    }
                }
            }),
        ));
    }
    if let Some(read_cap) = grant.read_handle() {
        // —— read-line：读行（read 能力；EOF → nil——与 Stage 0 一致）——
        defs.push((
            "read-line",
            BuiltinFn::new("read-line", {
                let cap = read_cap.clone();
                move |_, args| {
                    if !args.is_empty() {
                        return Err(RuntimeError::new(format!(
                            "read-line 需要 0 个参数，实际 {}",
                            args.len()
                        )));
                    }
                    match cap_read_line(&cap) {
                        Ok(s) => Ok(Value::Str(Rc::from(s.as_str()))),
                        Err(e) if e.message == "EOF" => Ok(Value::Nil),
                        Err(e) => Err(RuntimeError::new(e.message)),
                    }
                }
            }),
        ));
        // —— read-int：读行解析整数（read 能力）——
        defs.push((
            "read-int",
            BuiltinFn::new("read-int", {
                let cap = read_cap.clone();
                move |_, args| {
                    if !args.is_empty() {
                        return Err(RuntimeError::new(format!(
                            "read-int 需要 0 个参数，实际 {}",
                            args.len()
                        )));
                    }
                    match cap_read_line(&cap) {
                        Ok(s) => match s.trim().parse::<i64>() {
                            Ok(v) => Ok(Value::Int(v)),
                            Err(_) => Err(RuntimeError::new(format!(
                                "read-int 解析失败：非整数字符串「{}」",
                                s.trim()
                            ))),
                        },
                        Err(e) if e.message == "EOF" => Ok(Value::Nil),
                        Err(e) => Err(RuntimeError::new(e.message)),
                    }
                }
            }),
        ));
        // —— read-num：读行解析数值（read 能力）——
        defs.push((
            "read-num",
            BuiltinFn::new("read-num", {
                let cap = read_cap.clone();
                move |_, args| {
                    if !args.is_empty() {
                        return Err(RuntimeError::new(format!(
                            "read-num 需要 0 个参数，实际 {}",
                            args.len()
                        )));
                    }
                    match cap_read_line(&cap) {
                        Ok(s) => {
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
                        Err(e) if e.message == "EOF" => Ok(Value::Nil),
                        Err(e) => Err(RuntimeError::new(e.message)),
                    }
                }
            }),
        ));
    }
}

/// 令牌化写行（冻结契约调用——`CapabilityIO::write_line`）。
fn cap_write_line(cap: &Rc<RefCell<WriteCapability>>, s: &str) -> Result<(), RuntimeError> {
    <StdCapabilityIO as CapabilityIO>::write_line(&mut cap.borrow_mut(), s)
        .map_err(|e| RuntimeError::new(e.message))
}

/// 令牌化读行（冻结契约调用——EOF 约定消息「EOF」，见 StdCapabilityIO）。
fn cap_read_line(cap: &Rc<RefCell<ReadCapability>>) -> Result<String, crate::reserved::IOError> {
    <StdCapabilityIO as CapabilityIO>::read_line(&mut cap.borrow_mut())
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

/// `list?` 的 cdr 链单步判定（r24 / 42-e）：当前槽位为序对 → Next(cdr)；
/// Nil → End(true)（真表终止）；其余 → End(false)（点对/异物）。
enum ListStep {
    Next(GcRef),
    End(bool),
}

fn list_step(heap: &Heap, r: GcRef) -> ListStep {
    match heap.get(r).map(|s| &s.obj) {
        Some(kerf_runtime::HeapObj::Pair(_, cdr)) => ListStep::Next(*cdr),
        Some(kerf_runtime::HeapObj::Nil) => ListStep::End(true),
        // _ 臂理由：Foreign/Str/Int/Float/Bool/Symbol 槽位——cdr 链上的
        // 非序对非 nil 终结（点对形态或异物）
        _ => ListStep::End(false),
    }
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

// ---------------------------------------------------------------------------
// 静态类型检查签名表（Stage 1 批次 C 落地；旗标期 D8 阶段 2——
// kerf_compiler::hm_check_program 生产判定面 + check_program 回归基线
// 断言的双消费注入数据——scheme 重解释口径见 hm-inference-design §3.5）
// ---------------------------------------------------------------------------

/// 内置静态签名清单（与 register_globals 的注册表同文件维护——唯一可信
/// 数据源 §2.3 原则 10；`builtin_sigs_subset_of_registered` 测试锚定不漂移）。
///
/// 签名口径与运行时守卫逐项对齐（元数 floor/ceiling 与各注册处的
/// one_arg/two_args/args.len 守卫一致；参数域与 match 守卫一致）。
/// 不列入签名表的内置 = 检查器不静态检查（保守跳过，无风险）。
static BUILTIN_SIGS: &[(&str, BuiltinSig)] = &[
    // 算术：数值域；单位元元数 floor（(+)/(*) 合法 0 参；(- x) 一元取负）
    ("+", BuiltinSig::variadic(TcParam::Num, TcType::Num, 0)),
    ("-", BuiltinSig::variadic(TcParam::Num, TcType::Num, 1)),
    ("*", BuiltinSig::variadic(TcParam::Num, TcType::Num, 0)),
    ("/", BuiltinSig::variadic(TcParam::Num, TcType::Num, 2)),
    ("mod", BuiltinSig::variadic(TcParam::Num, TcType::Num, 2)),
    // 比较：全数值或全字符串（TD-011 r24：字符串全序参与全族——码点序）
    ("=", BuiltinSig::num_or_all_str()),
    ("<", BuiltinSig::ordering()),
    (">", BuiltinSig::ordering()),
    ("<=", BuiltinSig::ordering()),
    (">=", BuiltinSig::ordering()),
    // 序对
    (
        "cons",
        BuiltinSig::fixed(&[TcParam::Any, TcParam::Any], TcType::Pair),
    ),
    ("head", BuiltinSig::fixed(&[TcParam::Pair], TcType::Unknown)),
    ("tail", BuiltinSig::fixed(&[TcParam::Pair], TcType::Unknown)),
    // list：空参 → nil（结果域 Pair∪Nil → Unknown 保守）
    (
        "list",
        BuiltinSig::variadic(TcParam::Any, TcType::Unknown, 0),
    ),
    // 谓词（结果 Bool——上层 if 条件推断消费）
    ("is-nil", BuiltinSig::fixed(&[TcParam::Any], TcType::Bool)),
    ("is-pair", BuiltinSig::fixed(&[TcParam::Any], TcType::Bool)),
    ("is-int", BuiltinSig::fixed(&[TcParam::Any], TcType::Bool)),
    ("is-bool", BuiltinSig::fixed(&[TcParam::Any], TcType::Bool)),
    (
        "is-string",
        BuiltinSig::fixed(&[TcParam::Any], TcType::Bool),
    ),
    (
        "is-symbol",
        BuiltinSig::fixed(&[TcParam::Any], TcType::Bool),
    ),
    ("is-float", BuiltinSig::fixed(&[TcParam::Any], TcType::Bool)),
    (
        "is-number",
        BuiltinSig::fixed(&[TcParam::Any], TcType::Bool),
    ),
    ("is-list", BuiltinSig::fixed(&[TcParam::Any], TcType::Bool)),
    (
        "is-procedure",
        BuiltinSig::fixed(&[TcParam::Any], TcType::Bool),
    ),
    (
        "eq",
        BuiltinSig::fixed(&[TcParam::Any, TcParam::Any], TcType::Bool),
    ),
    // 逻辑
    ("not", BuiltinSig::fixed(&[TcParam::Bool], TcType::Bool)),
    // I/O（副作用面——参数域不检查，元数对齐）
    ("print", BuiltinSig::fixed(&[TcParam::Any], TcType::Nil)),
    ("newline", BuiltinSig::fixed(&[], TcType::Nil)),
    (
        "write-string",
        BuiltinSig::fixed(&[TcParam::Str], TcType::Nil),
    ),
    ("read-int", BuiltinSig::fixed(&[], TcType::Unknown)),
    ("read-num", BuiltinSig::fixed(&[], TcType::Unknown)),
    (
        "error",
        BuiltinSig::variadic(TcParam::Any, TcType::Unknown, 1),
    ),
    // 列表族（List = Pair∪Nil：length/reverse 接受空表）
    ("length", BuiltinSig::fixed(&[TcParam::List], TcType::Int)),
    (
        "reverse",
        BuiltinSig::fixed(&[TcParam::List], TcType::Unknown),
    ),
    (
        "append",
        BuiltinSig::variadic(TcParam::Any, TcType::Unknown, 0),
    ),
    (
        "nth",
        BuiltinSig::fixed(&[TcParam::List, TcParam::Int], TcType::Unknown),
    ),
    (
        "drop",
        BuiltinSig::fixed(&[TcParam::List, TcParam::Int], TcType::Unknown),
    ),
    (
        "member",
        BuiltinSig::fixed(&[TcParam::Any, TcParam::List], TcType::Bool),
    ),
    (
        "assoc",
        BuiltinSig::fixed(&[TcParam::Any, TcParam::List], TcType::Unknown),
    ),
    (
        "last-pair",
        BuiltinSig::fixed(&[TcParam::Pair], TcType::Pair),
    ),
    // 字符串族（r42/S1：全词化/动词化现代名）
    (
        "string-length",
        BuiltinSig::fixed(&[TcParam::Str], TcType::Int),
    ),
    (
        "string-append",
        BuiltinSig::fixed(&[TcParam::Str, TcParam::Str], TcType::Str),
    ),
    (
        "string-substring",
        BuiltinSig::fixed(&[TcParam::Str, TcParam::Int, TcParam::Int], TcType::Str),
    ),
    (
        "string-index-of",
        BuiltinSig::fixed(&[TcParam::Str, TcParam::Str], TcType::Int),
    ),
    (
        "string-contains",
        BuiltinSig::fixed(&[TcParam::Str, TcParam::Str], TcType::Bool),
    ),
    (
        "string-starts-with",
        BuiltinSig::fixed(&[TcParam::Str, TcParam::Str], TcType::Bool),
    ),
    (
        "string-ends-with",
        BuiltinSig::fixed(&[TcParam::Str, TcParam::Str], TcType::Bool),
    ),
    (
        "string-to-upper",
        BuiltinSig::fixed(&[TcParam::Str], TcType::Str),
    ),
    (
        "string-to-lower",
        BuiltinSig::fixed(&[TcParam::Str], TcType::Str),
    ),
    (
        "string-to-symbol",
        BuiltinSig::fixed(&[TcParam::Str], TcType::Symbol),
    ),
    (
        "symbol-to-string",
        BuiltinSig::fixed(&[TcParam::Symbol], TcType::Str),
    ),
    // Reader 原语（B3）
    (
        "str->pos-chars",
        BuiltinSig::fixed(&[TcParam::Str], TcType::Pair),
    ),
    (
        "char-whitespace?",
        BuiltinSig::fixed(&[TcParam::Str], TcType::Bool),
    ),
    (
        "char-alphabetic?",
        BuiltinSig::fixed(&[TcParam::Str], TcType::Bool),
    ),
    (
        "str-int-valid?",
        BuiltinSig::fixed(&[TcParam::Str], TcType::Bool),
    ),
    // 断言（r42/S1：动词化）
    (
        "assert-eq",
        BuiltinSig::fixed(&[TcParam::Any, TcParam::Any], TcType::Bool),
    ),
    // r42/S1 移除注记：批次 L 双名同步段（27 别名条目）随旧名退役删除
    // ——主签名区已直接持有现代名（上列各节 r42/S1 注记）；签名面
    // 56 维持（read-line 不列口径不变——20 §6.4 ② 历史锚）。
    // read-line：运行时不检查元数（忽略多余参数）——签名表不列（不静态断言）
];

/// 构造内置静态签名表（符号 → 签名；check_program 的注入入口）。
///
/// 批次 M 首件 M1（r39）：限定名签名**按模块表派生**——`ns/本地名`
/// 与底层同签名（同分派 → 同静态检查面；`read-line` 底层不列的口径
/// 维持——底层无签名则派生跳过）。派生而非静态枚举：与
/// `STDLIB_MODULES` 自动同步（零漂移——22 §8 R-N1/R-N3 行的静态面）。
pub fn builtin_sigs(table: &mut SymbolTable) -> HashMap<Symbol, BuiltinSig> {
    let mut map: HashMap<Symbol, BuiltinSig> = BUILTIN_SIGS
        .iter()
        .map(|(name, sig)| (table.intern(name), *sig))
        .collect();
    for &(ns, local, underlying) in STDLIB_MODULES {
        let u = table.intern(underlying);
        if let Some(sig) = map.get(&u).copied() {
            let qualified = format!("{}/{}", ns, local);
            map.insert(table.intern(&qualified), sig);
        }
    }
    map
}

fn cmp_builtin(name: &'static str, cmp: Cmp) -> Rc<BuiltinFn> {
    BuiltinFn::new(name, move |_, args| {
        if args.len() < 2 {
            return Err(RuntimeError::new(format!("{} 至少需要 2 个参数", name)));
        }
        // TD-016（批次 C 收紧）前置全参数域校验 + TD-011（r24 解决）：
        // 比较链短路终止不跳过后续操作数的类型检查；**全字符串链按
        // Unicode 码点序参与全部比较族**（含排序族）；其余链逐参数值域
        // 校验——首个非数值 → `{op} 需要数值`（与两参混串/非数值消息
        // 一致，TD-016 两参口径保持）。
        let all_str = args.iter().all(|a| matches!(a, Value::Str(_)));
        if !all_str {
            for a in &args {
                if a.as_number().is_none() {
                    return Err(RuntimeError::new(format!("{} 需要数值", name)));
                }
            }
        }
        // 链式比较：(= a b c) 等价于相邻两两全部成立（类型域已前置
        // 校验——下方错误臂成为防御性路径）
        for w in args.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            let ok = match (&cmp, a, b) {
                (Cmp::Eq, Value::Int(x), Value::Int(y)) => x == y,
                (Cmp::Lt, Value::Int(x), Value::Int(y)) => x < y,
                (Cmp::Gt, Value::Int(x), Value::Int(y)) => x > y,
                (Cmp::Le, Value::Int(x), Value::Int(y)) => x <= y,
                (Cmp::Ge, Value::Int(x), Value::Int(y)) => x >= y,
                // TD-011（r24）：字符串全序——`Rc<str>` 比较即 UTF-8 字节序
                // （UTF-8 编码保序性 → 与 Unicode 码点序全序一致；09-stdlib
                // v6.3 码点序裁定）
                (c, Value::Str(x), Value::Str(y)) => match c {
                    Cmp::Eq => x == y,
                    Cmp::Lt => x < y,
                    Cmp::Gt => x > y,
                    Cmp::Le => x <= y,
                    Cmp::Ge => x >= y,
                },
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

    /// 全授权（读写令牌齐备——I/O 内置注册的测试前提）。
    fn full_grant() -> IoGrant {
        IoGrant::from_requirements(crate::capability::IoRequirements {
            read: true,
            write: true,
        })
    }

    // -----------------------------------------------------------------------
    // 静态签名表 vs 注册表——双向防漂移锚（唯一可信数据源 §2.3 原则 10）
    // -----------------------------------------------------------------------

    /// 签名表名字全部已注册（重命名/删除内置时本测试拦截漂移）。
    #[test]
    fn builtin_sigs_subset_of_registered() {
        let mut t = SymbolTable::new();
        let g = register_globals(&mut t, &full_grant());
        let sigs = builtin_sigs(&mut t);
        for sym in sigs.keys() {
            assert!(g.contains_key(sym), "签名表条目未注册：{}", t.name(*sym));
        }
    }

    /// r42 / S1 移除轮闭合守卫（20 §7 移除轮行——旧名 27 件退役的结构
    /// 锚；r38 别名闭合守卫的移除轮演进形态）：①表长 27；②双向对账
    /// 零重复；③旧名零注册（含零授权面——fail-closed）；④现代名全
    /// 注册；⑤旧名零签名（HM 静态面对旧名同拒——BUILTIN_SIGS 主区
    /// 已直接持现代名）；⑥门控面零旧名（r38 实测口径维持）。
    #[test]
    fn removed_names_closed_and_modern_registered() {
        let mut t = SymbolTable::new();
        let g = register_globals(&mut t, &full_grant());
        let sigs = builtin_sigs(&mut t);
        assert_eq!(
            REMOVED_BUILTIN_NAMES.len(),
            27,
            "退役表长度应为 27（20 §8）"
        );
        // r42/S1 计数锚：全授权面 104（57 扁平 + 47 限定名）
        assert_eq!(
            g.len(),
            104,
            "全授权注册面应为 104（57 扁平 + 47 限定——r42/S1 移除后）"
        );
        let mut seen_old: Vec<&str> = Vec::new();
        let mut seen_new: Vec<&str> = Vec::new();
        for &(old, modern) in REMOVED_BUILTIN_NAMES {
            assert!(!seen_old.contains(&old), "旧名重复：{}", old);
            assert!(!seen_new.contains(&modern), "现代名重复：{}", modern);
            assert_ne!(old, modern, "映射不得指向自身：{}", old);
            seen_old.push(old);
            seen_new.push(modern);
            // ③ 旧名零注册（全授权面）
            let o = t.intern(old);
            assert!(!g.contains_key(&o), "旧名已退役不得注册：{}", old);
            // ⑤ 旧名零签名（静态面同拒）
            assert!(!sigs.contains_key(&o), "旧名不得持签名：{}", old);
            // ④ 现代名全注册 + 持签名（read-line 除外——不列口径）
            let m = t.intern(modern);
            assert!(g.contains_key(&m), "现代名未注册：{}", modern);
            if modern != "read-line" {
                assert!(sigs.contains_key(&m), "现代名缺签名：{}", modern);
            }
            // ⑥ 门控零旧名（20 §6.4 ③ 维持）
            assert!(
                !crate::capability::READ_GATED.contains(&old)
                    && !crate::capability::WRITE_GATED.contains(&old),
                "退役旧名 {} 不应出现在门控面",
                old
            );
        }
        // ③（续）零授权面旧名同零注册
        let mut t2 = SymbolTable::new();
        let none_grant = IoGrant::from_requirements(crate::capability::IoRequirements {
            read: false,
            write: false,
        });
        let g0 = register_globals(&mut t2, &none_grant);
        for &(old, _) in REMOVED_BUILTIN_NAMES {
            let o = t2.intern(old);
            assert!(!g0.contains_key(&o), "零授权面旧名不得注册：{}", old);
        }
    }

    /// 批次 M M1 模块表闭合守卫（r39——22 §8 实施对账表的结构锚）：
    /// ①总行数 47 + 七模块分面计数（20 §5.2 逐行对账：14/3/9/11/2/6/2）；
    /// ②全授权面：每限定名已注册 + 每底层已注册 + 全局面恰 131
    /// （84 基础[57+27 别名] + 47 限定名——计数锚同时拦截限定名撞基础名
    /// 的 HashMap 覆写陷阱）；③限定名与基础面零同名（防覆写）；
    /// ④零授权面：非 io 限定名 41 件恒注册（io 族按授权分项 fail-closed
    /// ——22 §5.1 矩阵）；⑤限定名签名派生 = 底层签名（read-line 除外
    /// ——底层不列口径维持）。
    #[test]
    fn stdlib_modules_closed_and_qualified_registered() {
        let mut t = SymbolTable::new();
        let g = register_globals(&mut t, &full_grant());
        let sigs = builtin_sigs(&mut t);
        // ① 行数与分面计数（20 §5.2）
        assert_eq!(STDLIB_MODULES.len(), 47, "模块表总行数应为 47");
        let mut per_ns: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for &(ns, _, _) in STDLIB_MODULES {
            *per_ns.entry(ns).or_default() += 1;
        }
        for (ns, expect) in [
            ("core", 14),
            ("pair", 3),
            ("list", 9),
            ("string", 11),
            ("symbol", 2),
            ("io", 6),
            ("char", 2),
        ] {
            assert_eq!(
                per_ns.get(ns).copied().unwrap_or(0),
                expect,
                "模块 {} 面计数漂移（20 §5.2）",
                ns
            );
        }
        // ② 全授权：双注册 + 计数锚 104（r42/S1：57 扁平 + 47 限定）
        assert_eq!(g.len(), 104, "全授权注册面应为 104（57 扁平 + 47 限定）");
        for &(ns, local, underlying) in STDLIB_MODULES {
            let q = t.intern(&format!("{}/{}", ns, local));
            assert!(g.contains_key(&q), "限定名未注册：{}/{}", ns, local);
            let u = t.intern(underlying);
            assert!(g.contains_key(&u), "底层未注册：{}", underlying);
            // ⑤ 派生签名与底层一致（read-line 除外——底层不列）
            let sig_q = sigs.get(&q);
            let sig_u = sigs.get(&u);
            if underlying != "read-line" {
                assert!(sig_q.is_some(), "限定名缺派生签名：{}/{}", ns, local);
                assert_eq!(
                    sig_q.copied(),
                    sig_u.copied(),
                    "派生签名漂移：{}/{} vs {}",
                    ns,
                    local,
                    underlying
                );
            }
        }
        // ④ 零授权面：非 io 限定名 41 恒注册（io fail-closed——分项检查）
        let mut t2 = SymbolTable::new();
        let none_grant = IoGrant::from_requirements(crate::capability::IoRequirements {
            read: false,
            write: false,
        });
        let g0 = register_globals(&mut t2, &none_grant);
        // 零授权：基础 51（57 - 6 门控名未注册）+ 41 非 io 限定 = 92
        assert_eq!(
            g0.len(),
            92,
            "零授权面应为 92（51 基础 + 41 非 io 限定——r42/S1）"
        );
        for &(ns, local, _) in STDLIB_MODULES {
            let q = t2.intern(&format!("{}/{}", ns, local));
            let registered = g0.contains_key(&q);
            assert_eq!(
                registered,
                ns != "io",
                "零授权面限定名注册态异常：{}/{}（io 应 fail-closed 不注册）",
                ns,
                local
            );
        }
    }

    /// 三方守卫之门控腿（r38——20 §9.4：注册 ⊇ 签名 ⊆ 门控之「门控 ⊆
    /// 注册」）：R9 数据驱动表六名（READ_GATED/WRITE_GATED）全注册。
    /// r42/S1：别名腿随退役下线（旧名零注册已由
    /// `removed_names_closed_and_modern_registered` ③ 承载）；门控
    /// 名与退役旧名零交集归同守卫 ⑥。
    #[test]
    fn builtin_gating_names_subset_of_registered() {
        let mut t = SymbolTable::new();
        let g = register_globals(&mut t, &full_grant());
        for name in crate::capability::READ_GATED
            .iter()
            .chain(crate::capability::WRITE_GATED.iter())
        {
            let sym = t.intern(name);
            assert!(g.contains_key(&sym), "门控内置未注册：{}", name);
        }
    }

    /// 运算符族签名全覆盖（算术/比较/序对/逻辑/字符串转换/断言——新内置
    /// 加入这些族而未配签名时拦截）。
    #[test]
    fn builtin_sigs_cover_operator_families() {
        let mut t = SymbolTable::new();
        let g = register_globals(&mut t, &full_grant());
        let sigs = builtin_sigs(&mut t);
        let families = [
            "+",
            "-",
            "*",
            "/",
            "mod",
            "=",
            "<",
            ">",
            "<=",
            ">=",
            "cons",
            "head",
            "tail",
            "not",
            "eq",
            "string-length",
            "string-append",
            "string-to-symbol",
            "symbol-to-string",
            "length",
            "assert-eq",
        ];
        for name in families {
            let sym = t.intern(name);
            assert!(g.contains_key(&sym), "内置缺失：{}", name);
            assert!(sigs.contains_key(&sym), "运算符族缺签名：{}", name);
        }
    }

    #[test]
    fn globals_have_core_set() {
        let mut t = SymbolTable::new();
        let g = register_globals(&mut t, &full_grant());
        for name in ["+", "-", "cons", "head", "print", "read-line", "eq"] {
            let sym = t.intern(name);
            assert!(g.contains_key(&sym), "缺少内置 {}", name);
        }
    }

    #[test]
    fn hygiene_fallback_resolves() {
        let mut t = SymbolTable::new();
        let mut g = register_globals(&mut t, &full_grant());
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
        let mut g = register_globals(&mut t, &full_grant());
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
