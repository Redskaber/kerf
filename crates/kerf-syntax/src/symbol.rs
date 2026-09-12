//! Symbol 内部化（interning）：标识符的 u32 句柄与全局符号表。
//!
//! 设计（§19.1 实现陷阱 2）：NFC 归一化在 intern 时做一次且仅一次，
//! 否则同一视觉标识符会产生两个 Symbol。
//!
//! `Symbol` 满足 Copy + Ord + Hash——可以作为 HashMap 键与比较句柄传递。

use std::collections::HashMap;
use std::rc::Rc;

/// 内部化标识符句柄（表内索引）。句柄之间可比较、可哈希。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(pub u32);

impl Symbol {
    /// 解析为字符串切片（符号表为唯一可信数据源；表缺失时显式报错）。
    pub fn as_str(self, table: &SymbolTable) -> &str {
        table.name(self)
    }
}

/// 核心形式与语法糖的关键字（§10.1 规则 2：Reader 归类为查表，词法层零语义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    Fn,
    If,
    Assign,
    Define,
    Do,
    Module,
    Import,
    Export,
    Quote,
    Let,
    LetRec,
    LetStar,
    Cond,
    Else,
    And,
    Or,
    When,
    While,
    Unless,
    DefineSyntax,
    SyntaxRules,
    Require,
    /// `perform`（r25/42-f——效应执行核心原语，D1 表面语法二形式之一）。
    Perform,
    /// `handle`（效应处理核心原语——浅处理，D2）。
    Handle,
    /// `resume`（continuation 调用的表面关键字——脱糖为 `(κ v)` Apply，D4
    /// 非独立原语）。
    Resume,
}

impl Keyword {
    /// 全部关键字（单一枚举面——22 §13 D36 枚举内一致性：变体名 =
    /// 字面名同词根；预内部化与 roundtrip 断言共享此单源）。
    pub const ALL: [Keyword; 25] = [
        Keyword::Fn,
        Keyword::If,
        Keyword::Assign,
        Keyword::Define,
        Keyword::Do,
        Keyword::Module,
        Keyword::Import,
        Keyword::Export,
        Keyword::Quote,
        Keyword::Let,
        Keyword::LetRec,
        Keyword::LetStar,
        Keyword::Cond,
        Keyword::Else,
        Keyword::And,
        Keyword::Or,
        Keyword::When,
        Keyword::While,
        Keyword::Unless,
        Keyword::DefineSyntax,
        Keyword::SyntaxRules,
        Keyword::Require,
        Keyword::Perform,
        Keyword::Handle,
        Keyword::Resume,
    ];

    /// 关键字源文本。
    pub fn as_str(self) -> &'static str {
        match self {
            Keyword::Fn => "fn",
            Keyword::If => "if",
            Keyword::Assign => "assign",
            Keyword::Define => "define",
            Keyword::Do => "do",
            Keyword::Module => "module",
            Keyword::Import => "import",
            Keyword::Export => "export",
            Keyword::Quote => "quote",
            Keyword::Let => "let",
            Keyword::LetRec => "letrec",
            Keyword::LetStar => "let*",
            Keyword::Cond => "cond",
            Keyword::Else => "else",
            Keyword::And => "and",
            Keyword::Or => "or",
            Keyword::When => "when",
            Keyword::While => "while",
            Keyword::Unless => "unless",
            Keyword::DefineSyntax => "define-syntax",
            Keyword::SyntaxRules => "syntax-rules",
            Keyword::Require => "require",
            Keyword::Perform => "perform",
            Keyword::Handle => "handle",
            Keyword::Resume => "resume",
        }
    }

    /// 查表归类（纯查表，无语义判断，§19.1 不变式 3）。
    pub fn from_name(name: &str) -> Option<Keyword> {
        Some(match name {
            "fn" => Keyword::Fn,
            "if" => Keyword::If,
            "assign" => Keyword::Assign,
            "define" => Keyword::Define,
            "do" => Keyword::Do,
            "module" => Keyword::Module,
            "import" => Keyword::Import,
            "export" => Keyword::Export,
            "quote" => Keyword::Quote,
            "let" => Keyword::Let,
            "letrec" => Keyword::LetRec,
            "let*" => Keyword::LetStar,
            "cond" => Keyword::Cond,
            "else" => Keyword::Else,
            "and" => Keyword::And,
            "or" => Keyword::Or,
            "when" => Keyword::When,
            "while" => Keyword::While,
            "unless" => Keyword::Unless,
            "define-syntax" => Keyword::DefineSyntax,
            "syntax-rules" => Keyword::SyntaxRules,
            "require" => Keyword::Require,
            "perform" => Keyword::Perform,
            "handle" => Keyword::Handle,
            "resume" => Keyword::Resume,
            _ => return None,
        })
    }
}

/// 符号表：Symbol → 名字的唯一可信数据源。
///
/// 关键字在构造时预内部化（`keyword_symbol` 查询为 O(1) 查表）。
/// Clone：编译缓存复用（Stage 1 批次 C——缓存命中返回表快照，ID 一致性
/// 由 intern 幂等性保证：同名 → 同 Symbol）。
#[derive(Debug, Default, Clone)]
pub struct SymbolTable {
    names: Vec<Rc<str>>,
    map: HashMap<Rc<str>, Symbol>,
    keywords: HashMap<Keyword, Symbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut table = SymbolTable {
            names: Vec::new(),
            map: HashMap::new(),
            keywords: HashMap::new(),
        };
        // 预内部化全部关键字（一次且仅一次，§19.1 陷阱 2——单源
        // `Keyword::ALL`，完备性随枚举同步）。
        for kw in Keyword::ALL {
            let sym = table.intern(kw.as_str());
            table.keywords.insert(kw, sym);
        }
        table
    }

    /// 内部化（含一次性 NFC 归一化）。
    pub fn intern(&mut self, name: &str) -> Symbol {
        let normalized: Rc<str> = if is_nfc(name) {
            Rc::from(name)
        } else {
            Rc::from(normalize_nfc(name))
        };
        if let Some(&sym) = self.map.get(&normalized) {
            return sym;
        }
        let sym = Symbol(self.names.len() as u32);
        self.names.push(normalized.clone());
        self.map.insert(normalized, sym);
        sym
    }

    /// 关键字符号（预内部化句柄）。
    pub fn keyword_symbol(&self, kw: Keyword) -> Symbol {
        self.keywords[&kw]
    }

    /// 判断符号是否为指定关键字。
    pub fn is_keyword(&self, sym: Symbol, kw: Keyword) -> bool {
        self.keywords.get(&kw) == Some(&sym)
    }

    /// 解析名字（越界为内部不变式破坏，直接 panic——数据源由本表独占写入）。
    pub fn name(&self, sym: Symbol) -> &str {
        &self.names[sym.0 as usize]
    }

    /// 表内符号总数。
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// 表是否为空。
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// NFC 判定（ASCII 快路径 + 常见组合字符检查的保守实现）。
///
/// 完整 Unicode NFC 判定需要归一化库；Stage 0 零依赖约束下采用保守策略：
/// 含任何组合字符（U+0300..U+036F 等）的输入走归一化路径，其余视为已归一。
/// 组合字符的规约采用「基字符 + 组合字符」折叠为预组字符的常见映射。
fn is_nfc(s: &str) -> bool {
    !s.chars().any(is_combining_mark)
}

fn is_combining_mark(c: char) -> bool {
    matches!(c, '\u{0300}'..='\u{036F}' | '\u{1AB0}'..='\u{1AFF}' | '\u{20D0}'..='\u{20FF}')
}

/// 最小 NFC 归一化：基字符 + 后随重音组合 → 预组合字符（stage0.md 陷阱 2 的务实落地）。
fn normalize_nfc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if is_combining_mark(c) {
            if let Some(base) = out.chars().last() {
                if let Some(pre) = compose_mark(base, c) {
                    out.pop();
                    out.push(pre);
                    continue;
                }
            }
            out.push(c);
        } else {
            out.push(c);
        }
    }
    out
}

/// 基字符 + 重音组合 → 预组合字符（覆盖 Latin-1 常见集；无映射返回 `None`）。
fn compose_mark(base: char, mark: char) -> Option<char> {
    match (base, mark) {
        ('A', '\u{0300}') => Some('À'),
        ('A', '\u{0301}') => Some('Á'),
        ('A', '\u{0302}') => Some('Â'),
        ('E', '\u{0301}') => Some('É'),
        ('e', '\u{0301}') => Some('é'),
        ('a', '\u{0300}') => Some('à'),
        ('a', '\u{0301}') => Some('á'),
        ('u', '\u{0308}') => Some('ü'),
        ('u', '\u{0301}') => Some('ú'),
        ('n', '\u{0303}') => Some('ñ'),
        ('N', '\u{0303}') => Some('Ñ'),
        ('c', '\u{0327}') => Some('ç'),
        ('C', '\u{0327}') => Some('Ç'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_is_idempotent() {
        let mut t = SymbolTable::new();
        let a = t.intern("fib");
        let b = t.intern("fib");
        assert_eq!(a, b);
        let c = t.intern("other");
        assert_ne!(a, c);
        assert_eq!(t.name(a), "fib");
    }

    #[test]
    fn keywords_prefetched() {
        let mut t = SymbolTable::new();
        let sym = t.intern("fn");
        assert_eq!(t.keyword_symbol(Keyword::Fn), sym);
        assert!(t.is_keyword(sym, Keyword::Fn));
        assert!(!t.is_keyword(sym, Keyword::If));
    }

    #[test]
    fn nfc_normalization_one_time() {
        let mut t = SymbolTable::new();
        // "e" + U+0301 (combining acute) → "é"
        let a = t.intern("e\u{0301}");
        let b = t.intern("\u{00E9}");
        assert_eq!(a, b, "组合形式与预组形式必须内部化为同一 Symbol");
        assert_eq!(t.name(a), "\u{00E9}");
    }

    #[test]
    fn keyword_roundtrip() {
        for kw in [
            Keyword::Fn,
            Keyword::Assign,
            Keyword::Do,
            Keyword::DefineSyntax,
            Keyword::SyntaxRules,
        ] {
            assert_eq!(Keyword::from_name(kw.as_str()), Some(kw));
        }
        assert_eq!(Keyword::from_name("not-a-keyword"), None);
    }

    #[test]
    fn keyword_all_roundtrip_and_distinct() {
        // 22 §13 D36 枚举一致性：ALL 25 名全量 roundtrip（变体 ↦ 字面名
        // ↦ 变体恒等）+ 字面名互不重叠（E0020 全域排他前提）+ ALL 与
        // E0020 禁绑面同一计数锚（25）。
        let mut names: Vec<&'static str> = Keyword::ALL.iter().map(|k| k.as_str()).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(total, 25, "关键字计数锚（E0020 严格保留字 25 名）");
        assert_eq!(names.len(), total, "关键字字面名互不重叠");
        for kw in Keyword::ALL {
            assert_eq!(
                Keyword::from_name(kw.as_str()),
                Some(kw),
                "roundtrip（{}）",
                kw.as_str()
            );
        }
    }
}
