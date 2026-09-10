//! kerf CLI（根 crate 入口——编排 kerf-driver）。
//!
//! 子命令：
//! - `run <file>`：编译 + VM 执行（生产路径），打印最终值；
//! - `eval <file>`：元循环求值器执行（参考路径）；
//! - `check <file>`：干编译（仅诊断）；
//! - `tokens / stx / core / ir / bc / code <file>`：管线各级 dump
//!   （§16 人类可感知输出 + §14.9 编译器自调试工具）；
//! - `bench <file> [N]`：性能基准（§14.8——fib 等基准的基线采集）。

use std::rc::Rc;

use kerf_core::CodeValue;
use kerf_driver::{compile_source, eval_source, run_source};
use kerf_reader::lex_source;
use kerf_syntax::{Symbol, SymbolTable};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(2);
    }
    let cmd = args[1].as_str();
    let file = args.get(2).map(|s| s.as_str());
    let exit = match (cmd, file) {
        ("run", Some(f)) => cmd_run(f),
        ("eval", Some(f)) => cmd_eval(f),
        ("check", Some(f)) => cmd_check(f),
        ("tokens", Some(f)) => cmd_tokens(f),
        ("stx", Some(f)) => cmd_stx(f),
        ("core", Some(f)) => cmd_core(f),
        ("ir", Some(f)) => cmd_ir(f),
        ("bc", Some(f)) => cmd_bc(f),
        ("code", Some(f)) => cmd_code(f),
        ("bench", Some(f)) => cmd_bench(f, args.get(3).and_then(|n| n.parse::<u32>().ok())),
        ("run", None) | ("eval", None) | ("check", None) => {
            eprintln!("错误：{} 需要文件参数", cmd);
            2
        }
        _ => {
            print_usage();
            2
        }
    };
    std::process::exit(exit);
}

fn print_usage() {
    eprintln!(
        "kerf {} — 最小自举系统级编程语言（Stage 0）",
        env!("CARGO_PKG_VERSION")
    );
    eprintln!();
    eprintln!("用法：kerf <子命令> <文件.krf>");
    eprintln!();
    eprintln!("子命令：");
    eprintln!("  run <file> [N]       编译 + VM 执行（打印最终值）");
    eprintln!("  eval <file>          元循环求值器执行（参考路径）");
    eprintln!("  check <file>         干编译（read→expand→compile，仅诊断）");
    eprintln!("  tokens <file>        Token 流 dump");
    eprintln!("  stx <file>           语法对象 dump");
    eprintln!("  core <file>          CoreExpr dump");
    eprintln!("  ir <file>            图 IR dump（节点 + 共享查找）");
    eprintln!("  bc <file>            字节码反汇编（含调试信息）");
    eprintln!("  code <file>          CodeValue 检查（良构性/自由变量）");
    eprintln!("  bench <file> [N]     性能基准（N 轮，默认 10）");
}

fn read_file(path: &str) -> Result<String, i32> {
    std::fs::read_to_string(path).map_err(|e| {
        eprintln!("错误：无法读取文件 {}：{}", path, e);
        1
    })
}

fn cmd_run(path: &str) -> i32 {
    let src = match read_file(path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    match run_source(&src, path) {
        Ok(outcome) => {
            // 最终值渲染（⇒ 前缀；print 内置输出已直接写 stdout；堆随值存活）
            println!("⇒ {}", kerf_vm::render_value(&outcome.value, &outcome.heap));
            0
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

fn cmd_eval(path: &str) -> i32 {
    let src = match read_file(path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    match eval_source(&src, path) {
        Ok(outcome) => {
            println!("⇒ {}", kerf_vm::render_value(&outcome.value, &outcome.heap));
            0
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

fn cmd_check(path: &str) -> i32 {
    let src = match read_file(path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    match compile_source(&src, path) {
        Ok(out) => {
            println!(
                "ok：{} 原型 / {} 常量 / {} 全局引用 / {} 指令",
                out.program.proto_count(),
                out.program.consts.len(),
                out.program.global_refs.len(),
                out.program.total_instructions()
            );
            0
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

fn cmd_tokens(path: &str) -> i32 {
    let src = match read_file(path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    let mut table = SymbolTable::new();
    match lex_source(&src, 0, &mut table) {
        Ok(toks) => {
            for t in &toks {
                let name = match &t.kind {
                    kerf_reader::TokenKind::Identifier(s) => table.name(*s).to_string(),
                    kerf_reader::TokenKind::Keyword(k) => k.as_str().to_string(),
                    kerf_reader::TokenKind::Operator(o, s) => {
                        format!("{:?}({})", o, table.name(*s))
                    }
                    other => format!("{:?}", other),
                };
                println!("{:>4}..{:<4} {}", t.span.start, t.span.end, name);
            }
            0
        }
        Err(e) => {
            eprintln!("read error：{}", e.message);
            1
        }
    }
}

fn cmd_stx(path: &str) -> i32 {
    let src = match read_file(path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    let mut table = SymbolTable::new();
    match kerf_reader::read_source(&src, 0, &mut table) {
        Ok(forms) => {
            for f in &forms {
                println!("{}", f.render(&|s: Symbol| table.name(s).to_string()));
            }
            0
        }
        Err(e) => {
            eprintln!("read error：{}", e.message);
            1
        }
    }
}

fn cmd_core(path: &str) -> i32 {
    let src = match read_file(path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    match compile_source(&src, path) {
        Ok(out) => {
            println!("{}", out.render_core());
            0
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

fn cmd_ir(path: &str) -> i32 {
    let src = match read_file(path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    match compile_source(&src, path) {
        Ok(out) => {
            println!("节点总数：{}", out.ir.len());
            println!("根节点：{:?}", out.ir.roots());
            for (id, node) in out.ir.iter_nodes() {
                let meta = out.ir.get_metadata(id);
                println!(
                    "  #{:<3} {:<40} span {}..{} scopes {}",
                    id,
                    format!("{:?}", node),
                    meta.span.start,
                    meta.span.end,
                    meta.scopes.len()
                );
            }
            0
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

fn cmd_bc(path: &str) -> i32 {
    let src = match read_file(path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    match compile_source(&src, path) {
        Ok(out) => {
            print!("{}", out.disassemble());
            0
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

fn cmd_code(path: &str) -> i32 {
    let src = match read_file(path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    match compile_source(&src, path) {
        Ok(out) => {
            for e in &out.core {
                let cv = CodeValue::from_expr(e);
                let free: Vec<String> = cv
                    .free_variables()
                    .iter()
                    .map(|s| out.table.name(*s).to_string())
                    .collect();
                println!(
                    "code-value：良构={} 自由变量=[{}] stage={:?}",
                    cv.is_well_formed(),
                    free.join(", "),
                    cv.stage
                );
                let _ = Rc::new(());
            }
            0
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

fn cmd_bench(path: &str, rounds: Option<u32>) -> i32 {
    let src = match read_file(path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    let n = rounds.unwrap_or(10);
    // 先验证一次正确性
    match run_source(&src, path) {
        Ok(outcome) => {
            println!(
                "验证：⇒ {}",
                kerf_vm::render_value(&outcome.value, &outcome.heap)
            );
        }
        Err(e) => {
            eprintln!("{}", e);
            return 1;
        }
    }
    let start = std::time::Instant::now();
    for _ in 0..n {
        if let Err(e) = run_source(&src, path) {
            eprintln!("{}", e);
            return 1;
        }
    }
    let elapsed = start.elapsed();
    let per_round = elapsed.as_secs_f64() / n as f64;
    println!(
        "基准：{} 轮 / 总 {:.3}s / 每轮 {:.3}ms",
        n,
        elapsed.as_secs_f64(),
        per_round * 1000.0
    );
    0
}
