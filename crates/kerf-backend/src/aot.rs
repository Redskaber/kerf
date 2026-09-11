//! AOT 编排（批次 G / 38-c——宿主侧外部进程链）。
//!
//! `QBE IL` →（`tools/qbe/bin/qbe` 子进程）→ 汇编 `.s` →（`cc`）→
//! 本地可执行 → 运行（exit code）。**永不进入自举链**（§21.2——本模块
//! 是 12-roadmap §2「构建引导 + 后端」Rust ~20% 基建的一部分）。
//!
//! **查找链**（§3.1 口径在库侧的运行时版）：`KERF_QBE` 环境变量 →
//! 可执行文件同侧 `../tools/qbe/bin/qbe` → 编译期
//! `CARGO_MANIFEST_DIR/../../tools/qbe/bin/qbe`。缺失 = 显式错误
//! （不静默跳过）。

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::codegen::CodegenError;

/// AOT 产物（三个落盘文件 + 运行入口）。
#[derive(Debug, Clone)]
pub struct NativeArtifacts {
    /// QBE IL 源（`*.ssa`）。
    pub il_path: PathBuf,
    /// 汇编（`*.s`——qbe 输出）。
    pub asm_path: PathBuf,
    /// 本地可执行。
    pub exe_path: PathBuf,
}

/// 进程调用统一封装（stderr 捕获入错误——诊断不丢失）。
fn run_command(mut cmd: Command, what: &str) -> Result<std::process::Output, CodegenError> {
    cmd.output()
        .map_err(|e| CodegenError::new(format!("{} 启动失败：{}", what, e)))
}

/// 解析 QBE 二进制路径（§3.1 运行时查找链）。
pub fn find_qbe() -> Result<PathBuf, CodegenError> {
    find_qbe_from(std::env::var("KERF_QBE").ok())
}

/// 查找链纯函数核（env 值注入式——测试无全局副作用，避免并行竞态：
/// 38-c 实测勘误，set_var 污染同进程其余测试的 find_qbe）。
fn find_qbe_from(kerf_qbe: Option<String>) -> Result<PathBuf, CodegenError> {
    // 1. 环境变量显式覆盖
    if let Some(p) = kerf_qbe {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Ok(pb);
        }
        return Err(CodegenError::new(format!(
            "KERF_QBE 指向的 QBE 不存在：{}",
            pb.display()
        )));
    }
    // 2. 可执行文件同侧 ../tools/qbe/bin/qbe（安装布局）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(root) = exe
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
        {
            let cand = root.join("tools/qbe/bin/qbe");
            if cand.is_file() {
                return Ok(cand);
            }
        }
    }
    // 3. 编译期锚（开发/测试布局：crates/kerf-backend → 仓库根）
    if let Some(dir) = option_env!("CARGO_MANIFEST_DIR") {
        let cand = Path::new(dir).join("../../tools/qbe/bin/qbe");
        if cand.is_file() {
            return Ok(cand);
        }
    }
    Err(CodegenError::new(
        "未找到 QBE 二进制（查找链：KERF_QBE 环境变量 → 安装布局 \
         tools/qbe/bin/qbe → 编译期仓库锚；安装：sh scripts/qbe/setup.sh）",
    ))
}

/// 完整 AOT 构建：IL 文本 → 汇编 → 可执行（`out_dir` 会被创建）。
pub fn build_native(
    il: &str,
    out_dir: &Path,
    stem: &str,
    qbe_bin: &Path,
) -> Result<NativeArtifacts, CodegenError> {
    std::fs::create_dir_all(out_dir)
        .map_err(|e| CodegenError::new(format!("创建输出目录失败：{}", e)))?;
    let il_path = out_dir.join(format!("{}.ssa", stem));
    let asm_path = out_dir.join(format!("{}.s", stem));
    let exe_path = out_dir.join(stem);

    // 1. IL 落盘
    std::fs::write(&il_path, il)
        .map_err(|e| CodegenError::new(format!("写 IL 文件失败：{}", e)))?;

    // 2. qbe：IL → 汇编
    let mut cmd_qbe = Command::new(qbe_bin);
    cmd_qbe.arg("-o").arg(&asm_path).arg(&il_path);
    let out = run_command(cmd_qbe, "QBE")?;
    if !out.status.success() {
        return Err(CodegenError::new(format!(
            "QBE 编译失败（exit {}）：{}",
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stderr)
        )));
    }

    // 3. cc：汇编 → 可执行
    let mut cmd_cc = Command::new("cc");
    cmd_cc.arg("-o").arg(&exe_path).arg(&asm_path);
    let out = run_command(cmd_cc, "cc（系统汇编器/链接器）")?;
    if !out.status.success() {
        return Err(CodegenError::new(format!(
            "cc 链接失败（exit {}）：{}",
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stderr)
        )));
    }

    Ok(NativeArtifacts {
        il_path,
        asm_path,
        exe_path,
    })
}

/// 运行本地可执行（exit code → i64；POSIX 低 8 位口径由调用方文档化）。
pub fn run_native(exe: &Path) -> Result<i64, CodegenError> {
    let out = run_command(Command::new(exe), "本地可执行")?;
    match out.status.code() {
        Some(c) => Ok(c as i64),
        None => Err(CodegenError::new(format!("进程被信号终止：{}", out.status))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 查找链负向：KERF_QBE 指向不存在路径 → 显式错误（不静默）。
    /// 纯函数注入式（38-c 勘误：真实 set_var 会并行污染其余测试）。
    #[test]
    fn find_qbe_env_override_missing_fails_loudly() {
        let r = find_qbe_from(Some("/nonexistent/qbe".into()));
        assert!(r.is_err());
        assert!(r.unwrap_err().reason.contains("KERF_QBE"));
    }

    /// 查找链正向（env 显式路径注入）：存在即命中。
    #[test]
    fn find_qbe_from_env_explicit_hit() {
        // 用自身二进制充当「存在的文件」（不依赖 qbe 布局的正向样本）
        let me = std::env::current_exe().expect("测试进程可执行路径");
        let r = find_qbe_from(Some(me.to_string_lossy().into_owned()));
        assert!(r.is_ok());
    }

    /// 查找链正向：仓库内二进制可达（本机工具链就位——38-b 落位产物）。
    #[test]
    fn find_qbe_repo_layout_finds_binary() {
        let r = find_qbe();
        assert!(r.is_ok(), "应找到仓库内 QBE：{:?}", r.err());
    }

    /// AOT 负向：qbe 路径是普通文本文件（非可执行）→ 启动失败错误。
    #[test]
    fn build_native_bad_qbe_fails_loudly() {
        let tmp = std::env::temp_dir().join(format!("kerf-aot-test-{}", std::process::id()));
        let fake = tmp.join("fake-qbe");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(&fake, "not a binary").unwrap();
        let r = build_native(
            "export function l $main() {\n@l0\n\tret 0\n}\n",
            &tmp,
            "t_bad",
            &fake,
        );
        assert!(r.is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
