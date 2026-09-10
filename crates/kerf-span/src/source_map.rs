//! SourceMap：file_id → 源文本/行号表的唯一可信数据源（sop.md §2.3 原则 10）。
//!
//! 职责：Span（字节偏移）→ 行/列的派生渲染；源文本行内容查询（诊断摘录）。
//! 行号从 1 起、列号按**字符数**（非字节）从 1 起渲染——字节偏移仍是主键。

use std::rc::Rc;

/// 单个源文件（源文本 + 行起始偏移表）。
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// 文件名（展示用）。
    pub name: String,
    /// 源文本。
    pub src: Rc<str>,
    /// 每行起始字节偏移（第 0 行从 0 开始）。
    line_starts: Vec<u32>,
}

impl SourceFile {
    fn new(name: &str, src: &str) -> Self {
        let line_starts = compute_line_starts(src);
        SourceFile {
            name: name.to_string(),
            src: Rc::from(src),
            line_starts,
        }
    }

    /// 字节偏移 → (行, 字符列)，均从 1 起。列按字符计（UTF-8 感知）。
    fn line_col(&self, offset: u32) -> (u32, u32) {
        let offset = (offset as usize).min(self.src.len()) as u32;
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let line_start = self.line_starts[line_idx] as usize;
        let col_chars = self.src[line_start..offset as usize].chars().count() as u32 + 1;
        (line_idx as u32 + 1, col_chars)
    }

    /// 取整行文本（用于诊断摘录），行号从 1 起。
    fn line_text(&self, line: u32) -> &str {
        let idx = (line as usize).saturating_sub(1);
        if idx >= self.line_starts.len() {
            return "";
        }
        let start = self.line_starts[idx] as usize;
        let end = self
            .line_starts
            .get(idx + 1)
            .map(|&e| e as usize)
            .unwrap_or(self.src.len());
        self.src[start..end].trim_end_matches(['\n', '\r'])
    }
}

fn compute_line_starts(src: &str) -> Vec<u32> {
    let mut starts = vec![0u32];
    for (i, b) in src.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i as u32 + 1);
        }
    }
    starts
}

/// 源文件注册表：Span 的 file_id 在此解析为源文本与行/列。
#[derive(Debug, Default, Clone)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        SourceMap { files: Vec::new() }
    }

    /// 登记源文件，返回分配的 file_id。
    pub fn add_file(&mut self, name: &str, src: &str) -> u32 {
        let id = self.files.len() as u32;
        self.files.push(SourceFile::new(name, src));
        id
    }

    /// 查询源文件（file_id 越界返回 `None`——显式错误，不静默，§2.3 原则 4）。
    pub fn file(&self, file_id: u32) -> Option<&SourceFile> {
        self.files.get(file_id as usize)
    }

    /// 文件数量。
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Span → 人类可读位置（"main.krf:3:5"）。代次 > 0 时附加标记。
    pub fn render_location(&self, file_id: u32, offset: u32, expansion_id: u32) -> String {
        match self.file(file_id) {
            Some(f) => {
                let (line, col) = f.line_col(offset);
                if expansion_id > 0 {
                    format!("{}:{}:{} (expansion {})", f.name, line, col, expansion_id)
                } else {
                    format!("{}:{}:{}", f.name, line, col)
                }
            }
            None => format!("<unknown file {}>", file_id),
        }
    }

    /// 诊断摘录：定位行文本 + 列指示（^ 指到行尾）。
    pub fn excerpt(&self, file_id: u32, offset: u32) -> String {
        match self.file(file_id) {
            Some(f) => {
                let (line, col) = f.line_col(offset);
                let text = f.line_text(line);
                let pad = " ".repeat(col.saturating_sub(1) as usize);
                format!("{:>4} | {}\n     | {}^", line, text, pad)
            }
            None => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_basic() {
        let mut sm = SourceMap::new();
        let id = sm.add_file("t.krf", "(define x 1)\n(print x)\n");
        let f = sm.file(id).unwrap();
        assert_eq!(f.line_col(0), (1, 1));
        assert_eq!(f.line_col(1), (1, 2));
        // 第二行从 13 开始（"(define x 1)\n" = 13 字节）
        assert_eq!(f.line_col(12), (1, 13));
        assert_eq!(f.line_col(13), (2, 1));
        assert_eq!(f.line_col(14), (2, 2));
    }

    #[test]
    fn line_col_utf8() {
        let mut sm = SourceMap::new();
        let id = sm.add_file("t.krf", "(+ \"你好\" 1)");
        let f = sm.file(id).unwrap();
        // 字节 4 是 "你" 的第一个字节；列按字符计：src[0..4] = `(+ "` → 第 5 列
        assert_eq!(f.line_col(4), (1, 5));
        // 字节 7 是 "好" 的第一个字节：src[0..7] 前 4 字符 + 1 个汉字 = 第 6 列
        assert_eq!(f.line_col(7), (1, 6));
    }

    #[test]
    fn render_location_and_excerpt() {
        let mut sm = SourceMap::new();
        let id = sm.add_file("t.krf", "(define x 1)\n");
        let loc = sm.render_location(id, 9, 0);
        assert_eq!(loc, "t.krf:1:10");
        let exp = sm.excerpt(id, 9);
        assert!(exp.contains("(define x 1)"));
        assert!(exp.contains('^'));
    }

    #[test]
    fn unknown_file_is_explicit() {
        let sm = SourceMap::new();
        assert_eq!(sm.render_location(7, 0, 0), "<unknown file 7>");
    }
}
