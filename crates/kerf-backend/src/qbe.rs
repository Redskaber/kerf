//! QBE IL 生成（批次 G / 38-c——G1 后段）。
//!
//! `AnnotatedANF` → QBE IL 文本（`function`/`@label`/`jnz`/`phi`——
//! 值合并经 phi 指令）。**全 `l`（64 位）类型**——与 kerf `Int(i64)`
//! 语义对齐；比较结果 `w`（布尔 0/1）。
//!
//! **QBE 语法锚（38-b/38-c 冒烟实测勘误——docs/tools/qbe/setup.md）**：
//! - 函数签名必须带返回类型：`function l $fib(l %t0) {`；
//! - 整数比较指令带宽度后缀：`csltl`/`ceql`（非 `cslt`）；
//! - 值合并：phi 指令（`%r =l phi @l1 %v1, @l2 %v2`，须位于块首——
//!   带参 jmp/块参数语法在 QBE 1.3 不存在，38-c 实测勘误）；
//! - 指令行以制表符缩进（QBE 硬性词法要求）。

use crate::anf::{AAtom, ACtrl, AFuncDef, AOp, APhi, APrim, AStmt, ATemp, AnnotatedANF};
use crate::codegen::{CodegenBackend, CodegenError, CompiledModule, PassDescriptor, TargetTriple};

// ---------------------------------------------------------------------------
// IL 渲染（§10.1 规则 4：文本产物函数带 `_il` 后缀）
// ---------------------------------------------------------------------------

/// 生成完整 QBE IL 文本（main = `export function l $main()`——返回值即
/// 进程退出码，POSIX 取低 8 位；其余函数内部可见）。
pub fn gen_il(ir: &AnnotatedANF) -> String {
    let mut out = String::new();
    for (i, f) in ir.funcs.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        gen_func_il(&mut out, f, i == 0);
    }
    out
}

/// 单函数渲染（blocks[i] ↔ 标签 `@l{i}`——entry 块 0 与 LowerCtxt 的
/// start_block 顺序分配对齐）。
fn gen_func_il(out: &mut String, f: &AFuncDef, is_main: bool) {
    let export = if is_main { "export " } else { "" };
    let params: Vec<String> = f
        .params
        .iter()
        .map(|t| format!("l {}", render_temp(t)))
        .collect();
    out.push_str(&format!(
        "{}function l ${}({}) {{\n",
        export,
        f.name,
        params.join(", ")
    ));
    for (bi, b) in f.body.blocks.iter().enumerate() {
        out.push_str(&format!("@l{}\n", bi));
        for phi in &b.phis {
            gen_phi_il(out, phi);
        }
        for s in &b.stmts {
            gen_stmt_il(out, s);
        }
        gen_ctrl_il(out, &b.ctrl);
    }
    out.push_str("}\n");
}

/// phi 绑定渲染（块首——`%dst =l phi @l{i} %v, @l{j} %w`）。
fn gen_phi_il(out: &mut String, phi: &APhi) {
    let sources: Vec<String> = phi
        .sources
        .iter()
        .map(|(lbl, a)| format!("@l{} {}", lbl.0, render_atom(a)))
        .collect();
    out.push_str(&format!(
        "\t{} =l phi {}\n",
        render_temp(&phi.dst),
        sources.join(", ")
    ));
}

/// 语句渲染（`%dst =<t> <op> <args>`）。
fn gen_stmt_il(out: &mut String, s: &AStmt) {
    match &s.op {
        AOp::Prim(prim, atoms) => {
            let (ty, instr) = render_prim(*prim);
            // Not 特例：(not x) ≡ (x = 0) ——ceql 比对 0 补位
            let mut args: Vec<String> = atoms.iter().map(render_atom).collect();
            if *prim == APrim::Not {
                args.push("0".to_string());
            }
            out.push_str(&format!(
                "\t{} ={} {} {}\n",
                render_temp(&s.dst),
                ty,
                instr,
                args.join(", ")
            ));
        }
        AOp::Call { func, args } => {
            let rendered: Vec<String> = args.iter().map(render_atom).collect();
            out.push_str(&format!(
                "\t{} =l call ${}(l {})\n",
                render_temp(&s.dst),
                func,
                rendered.join(", l ")
            ));
        }
    }
}

/// 控制流终结渲染。
fn gen_ctrl_il(out: &mut String, ctrl: &ACtrl) {
    match ctrl {
        ACtrl::Ret(a) => {
            out.push_str(&format!("\tret {}\n", render_atom(a)));
        }
        ACtrl::Br { cond, t, f } => {
            out.push_str(&format!(
                "\tjnz {}, @l{}, @l{}\n",
                render_temp(cond),
                t.0,
                f.0
            ));
        }
        ACtrl::Jmp { target } => {
            out.push_str(&format!("\tjmp @l{}\n", target.0));
        }
    }
}

/// 渲染临时名（`%tN`）。
fn render_temp(t: &ATemp) -> String {
    format!("%t{}", t.0)
}

/// 渲染原子。
fn render_atom(a: &AAtom) -> String {
    match a {
        AAtom::Int(v) => format!("{}", v),
        AAtom::Var(t) => render_temp(t),
    }
}

/// 渲染原语（QBE 指令名——返回 (结果类型, 指令)；算术全 l、比较产 w）。
fn render_prim(p: APrim) -> (&'static str, &'static str) {
    match p {
        APrim::Add => ("l", "add"),
        APrim::Sub => ("l", "sub"),
        APrim::Mul => ("l", "mul"),
        APrim::Div => ("l", "div"),
        APrim::Mod => ("l", "rem"),
        APrim::Eq => ("w", "ceql"),
        APrim::Lt => ("w", "csltl"),
        APrim::Gt => ("w", "csgtl"),
        APrim::Le => ("w", "cslel"),
        APrim::Ge => ("w", "csgel"),
        // (not x) ≡ (x = 0)——ceqw（操作数是布尔 w 值；ceqw 补 0 位，
        // gen_stmt_il 处理——38-c 实测勘误：l 后缀对 w 操作数非法）
        APrim::Not => ("w", "ceqw"),
    }
}

// ---------------------------------------------------------------------------
// QbeBackend（CodegenBackend 契约实现——13 §3.3.7 首个做实后端）
// ---------------------------------------------------------------------------

/// QBE 后端（IL 生成本体——外部进程编排在 [`crate::aot`]）。
///
/// **契约行为**：
/// - `supported_targets`：QBE 1.3 实测目标面（amd64_sysv 本机默认 +
///   arm64——PoC 交付锚定本机 amd64_sysv）；
/// - `compile`：AnnotatedANF → QBE IL 文本（`CompiledModule.bytes` =
///   IL UTF-8 字节；`.ssa` 是 QBE 的目标码——AOT 层再降汇编/本地码）；
/// - `backend_optimizations`：QBE 内建 pass（ssa/gvn/gcm/rega——
///   「10% 代码 ≈ 70% 性能」的兑现面，08-backend-evolution §2）。
#[derive(Debug, Clone)]
pub struct QbeBackend;

impl CodegenBackend for QbeBackend {
    fn supported_targets(&self) -> Vec<TargetTriple> {
        vec![
            TargetTriple("x86_64-unknown-linux-qbe".into()),
            TargetTriple("aarch64-unknown-linux-qbe".into()),
        ]
    }

    fn compile(
        &self,
        ir: &AnnotatedANF,
        target: &TargetTriple,
    ) -> Result<CompiledModule, CodegenError> {
        if !self.supported_targets().iter().any(|t| t.0 == target.0) {
            return Err(CodegenError::new(format!(
                "QBE 后端不支持目标 `{}`（支持：amd64_sysv / arm64）",
                target.0
            )));
        }
        let il = gen_il(ir);
        Ok(CompiledModule {
            target: target.clone(),
            bytes: il.into_bytes(),
        })
    }

    fn backend_optimizations(&self) -> Vec<PassDescriptor> {
        vec![
            PassDescriptor { name: "qbe-ssa" },
            PassDescriptor { name: "qbe-gvn" },
            PassDescriptor { name: "qbe-gcm" },
            PassDescriptor { name: "qbe-rega" },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// IL 生成冒烟（结构断言——不依赖外部进程）：
    /// 无函数空程序 → 空 IL 文本。
    #[test]
    fn gen_il_empty_program_is_empty() {
        let ir = AnnotatedANF::default();
        assert_eq!(gen_il(&ir), "");
    }

    /// QbeBackend 契约：目标门（负向——不支持目标拒绝）+ IL 产物正向。
    #[test]
    fn qbe_backend_target_gate_and_module() {
        let b = QbeBackend;
        let ir = AnnotatedANF::default();
        // 负向：不支持目标
        assert!(b.compile(&ir, &TargetTriple("wasm32".into())).is_err());
        // 正向：IL 文本产物（空程序 → 空文本入信封）
        let m = b
            .compile(&ir, &TargetTriple("x86_64-unknown-linux-qbe".into()))
            .unwrap();
        assert_eq!(m.target.0, "x86_64-unknown-linux-qbe");
        assert!(m.bytes.is_empty());
        // 优化清单：QBE 内建四 pass 声明
        assert_eq!(b.backend_optimizations().len(), 4);
    }
}
