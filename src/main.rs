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
use kerf_driver::{
    cache_stats, check_source, compile_source, dump_stx, dump_tokens, eval_source, run_source,
    test_source,
};
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
        ("test", Some(f)) => cmd_test(f),
        ("tokens", Some(f)) => cmd_tokens(f),
        ("stx", Some(f)) => cmd_stx(f),
        ("core", Some(f)) => cmd_core(f),
        ("ir", Some(f)) => cmd_ir(f),
        ("bc", Some(f)) => cmd_bc(f),
        ("code", Some(f)) => cmd_code(f),
        ("bench", Some(f)) => cmd_bench(f, args.get(3).and_then(|n| n.parse::<u32>().ok())),
        ("run", None) | ("eval", None) | ("check", None) | ("test", None) => {
            eprintln!("错误：{} 需要文件参数", cmd);
            2
        }
        // _ 臂理由：未知子命令或 dump/bench 类子命令缺文件参数——打印用法并以码 2 退出
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
    eprintln!("  test <file>          用例运行器（表达式形式=用例；短路+错误恢复）");
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

fn cmd_test(path: &str) -> i32 {
    let src = match read_file(path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    match test_source(&src, path) {
        Ok(report) => {
            for case in &report.cases {
                if case.pass {
                    println!("PASS  {:>3} {}", case.index, case.name);
                } else {
                    println!("FAIL  {:>3} {}", case.index, case.name);
                    if !case.detail.is_empty() {
                        println!("      └ {}", case.detail);
                    }
                }
            }
            println!(
                "通过 {} / 共 {}（失败 {}）",
                report.passed,
                report.cases.len(),
                report.failed
            );
            if report.all_passed() {
                0
            } else {
                1
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
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
    // 批次 C：编译（缓存路径）+ 保守静态类型检查（多错误全量收集）
    match check_source(&src, path) {
        Ok(report) => {
            let cache_note = if report.cache_hit {
                "缓存命中"
            } else {
                "缓存未中"
            };
            let stats = cache_stats();
            if report.diagnostics.is_empty() {
                println!(
                    "ok：{} 原型 / {} 常量 / {} 全局引用 / {} 指令（{}；会话命中 {}/{}）",
                    report.proto_count,
                    report.const_count,
                    report.global_ref_count,
                    report.instruction_count,
                    cache_note,
                    stats.hits,
                    stats.hits + stats.misses
                );
                0
            } else {
                for line in &report.rendered {
                    eprintln!("{}", line);
                }
                println!(
                    "发现 {} 个静态问题（E0005；{} 原型 / {} 指令；{}）",
                    report.diagnostics.len(),
                    report.proto_count,
                    report.instruction_count,
                    cache_note
                );
                1
            }
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
    // §11/§14.7.2 B4：reader 仅 driver 调用——CLI 经 dump_tokens 转发
    match dump_tokens(&src, path) {
        Ok(out) => {
            print!("{}", out);
            0
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

fn cmd_stx(path: &str) -> i32 {
    let src = match read_file(path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    match dump_stx(&src, path) {
        Ok(out) => {
            print!("{}", out);
            0
        }
        Err(e) => {
            eprintln!("{}", e);
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
