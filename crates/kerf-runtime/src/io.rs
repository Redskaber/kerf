//! 最小 I/O（stage0.md §8.8：传统方案——全局函数，非能力模型）。
//!
//! 仅 `read_line()` 与 `write_line()`。Stage 0 不引入能力模型 I/O，
//! 但接口预留见 kerf-driver::reserved::CapabilityIO（P3 级类型定义）。
//!
//! RuntimeError 定义于此（Layer 0 最小形态：**仅 message**——层级依赖规则
//! §2.4.5 规定 Layer 0 无依赖，Span 位于 Layer 1，故源位置由执行层
//! （kerf-vm）经 debug_info_table 反查后回填到包装错误）。

/// 运行时错误（I/O 与分配层共享最小形态）。
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeError {
    pub message: String,
}

impl RuntimeError {
    /// 构造。
    pub fn new(message: impl Into<String>) -> Self {
        RuntimeError {
            message: message.into(),
        }
    }
}

/// 写一行到标准输出（§8.8 最小 I/O）。
pub fn write_line_stdout(s: &str) -> Result<(), RuntimeError> {
    use std::io::Write;
    let mut out = std::io::stdout();
    writeln!(out, "{}", s).map_err(|e| RuntimeError::new(format!("stdout 写入失败：{}", e)))
}

/// 写字符串到标准输出（无换行——`write-string` 的通道层载体，r5 标准库）。
pub fn write_stdout(s: &str) -> Result<(), RuntimeError> {
    use std::io::Write;
    let mut out = std::io::stdout();
    write!(out, "{}", s).map_err(|e| RuntimeError::new(format!("stdout 写入失败：{}", e)))
}

/// 从标准输入读一行（EOF 返回 None）。
pub fn read_line_stdin() -> Result<Option<String>, RuntimeError> {
    use std::io::BufRead;
    let mut line = String::new();
    match std::io::stdin().lock().read_line(&mut line) {
        Ok(0) => Ok(None), // EOF
        Ok(_) => Ok(Some(line.trim_end_matches('\n').to_string())),
        Err(e) => Err(RuntimeError::new(format!("stdin 读取失败：{}", e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_error_shape() {
        let e = RuntimeError::new("boom");
        assert_eq!(e.message, "boom");
    }

    #[test]
    fn write_line_ok() {
        assert!(write_line_stdout("").is_ok());
    }
}
