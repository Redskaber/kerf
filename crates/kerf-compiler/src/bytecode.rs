//! 字节码程序结构（§8.12 + §10.1 Bc 前缀规则）。
//!
//! **VM 状态包含**（§8.12）：代码（protos）、数据栈、调用栈、全局环境、
//! 常量池和调试信息表（本 crate 定义代码侧；运行侧状态见 kerf-vm）。
//!
//! **DebugInfoTable**（§8.12 / 原则 17 编译时生成，运行时查询）：
//! 每条指令 (pc, Span) 映射，无裸指令（§19.3 不变式 3）。

use std::collections::HashMap;
use std::rc::Rc;

use kerf_span::Span;
use kerf_syntax::Symbol;

use crate::opcode::Op;

/// 常量池条目（intern_const 按值去重——§19.3 陷阱：常量池去重）。
///
/// `Eq`/`Hash` 手工实现：Float 按位比较（NaN 归一化为标准位模式）——
/// f64 不满足原生 Eq/Hash 约束的既定工程处理。
#[derive(Debug, Clone, PartialEq)]
pub enum BcConst {
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    Bool(bool),
    Nil,
    /// 符号（全局变量名按名解析：LOAD_GLOBAL/STORE_GLOBAL 的操作数）。
    Symbol(Symbol),
}

impl Eq for BcConst {}

impl std::hash::Hash for BcConst {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            BcConst::Int(v) => v.hash(state),
            BcConst::Float(v) => float_key(*v).hash(state),
            BcConst::Str(s) => s.hash(state),
            BcConst::Bool(b) => b.hash(state),
            BcConst::Nil => {}
            BcConst::Symbol(s) => s.hash(state),
        }
    }
}

/// 浮点键（+0.0/-0.0 归一 + NaN 归一）。
fn float_key(f: f64) -> u64 {
    if f == 0.0 {
        0u64
    } else if f.is_nan() {
        0x7ff8_0000_0000_0000u64
    } else {
        f.to_bits()
    }
}

/// 闭包捕获源（编译期解析的捕获描述——CLOSURE 指令的伴随数据）。
///
/// 语义：捕获**共享单元格**（`Rc<RefCell<Value>>`）——词法闭包的可变
/// 捕获语义（set! 经共享单元传播，letrec 递归成立）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureSource {
    /// 外围帧局部槽（共享该槽的单元格）。
    Local(u32),
    /// 外围闭包捕获槽（传递共享）。
    Captured(u32),
}

/// 原型（子程序）：参数 + 局部槽布局 + 捕获名表 + 代码 + 调试信息。
#[derive(Debug, Clone, PartialEq)]
pub struct BcProto {
    /// 原型名（诊断/堆栈追踪用）。
    pub name: Symbol,
    /// 参数符号（局部槽 0..n）。
    pub params: Vec<Symbol>,
    /// 局部槽数（参数 + let 绑定）。
    pub n_locals: u32,
    /// 捕获名表（闭包捕获槽 0..m——与 CLOSURE 指令捕获序一致）。
    pub capture_names: Vec<Symbol>,
    /// 捕获源描述（与 capture_names 对齐——VM 据此共享外围单元格）。
    pub capture_sources: Vec<CaptureSource>,
    /// 指令序列。
    pub code: Vec<Op>,
    /// 调试信息表：与 code 逐 pc 对齐的 Span（无裸指令，§19.3 不变式 3）。
    pub debug_spans: Vec<Span>,
    /// 自由变量集合（捕获 + 全局引用的并集，供工具链/CodeValue 检查）。
    pub free_vars: Vec<Symbol>,
}

impl BcProto {
    /// 参数数量。
    pub fn arity(&self) -> u32 {
        self.params.len() as u32
    }

    /// 指令数。
    pub fn len(&self) -> usize {
        self.code.len()
    }

    /// 是否为空原型。
    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }
}

/// 完整字节码程序。
#[derive(Debug, Clone, PartialEq)]
pub struct BcProgram {
    /// 原型表（proto 0 = main 入口）。
    pub protos: Vec<BcProto>,
    /// 常量池（去重）。
    pub consts: Vec<BcConst>,
    /// 入口原型索引（固定 0；Stage 2 多模块链接期解析）。
    pub entry: u32,
    /// 程序引用的全部全局名（含内置函数——driver 注册时校验）。
    pub global_refs: Vec<Symbol>,
    /// 模块元信息（Stage 0 单模块）。
    pub module_name: Option<Symbol>,
}

impl BcProgram {
    /// 入口原型。
    pub fn entry_proto(&self) -> &BcProto {
        &self.protos[self.entry as usize]
    }

    /// 原型数。
    pub fn proto_count(&self) -> usize {
        self.protos.len()
    }

    /// 指令总数（全部原型求和——性能基准的代码量度量）。
    pub fn total_instructions(&self) -> usize {
        self.protos.iter().map(|p| p.len()).sum()
    }

    /// 与另一程序逐操作码比较（同结果测试：两次编译输出必须一致，§21.3）。
    pub fn bytecode_equal(&self, other: &BcProgram) -> bool {
        self == other
    }
}

/// 调试信息表类型别名（每原型独立持有；本别名保留 §8.12 术语映射）。
pub type DebugInfoTable = Vec<Span>;

/// 反汇编渲染（人类可感知输出是一等公民，§2.2 原则 16）。
pub fn disassemble_program(program: &BcProgram, resolve: &dyn Fn(Symbol) -> String) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "== kerf bytecode: {} protos, {} consts, {} globals ==\n",
        program.protos.len(),
        program.consts.len(),
        program.global_refs.len()
    ));
    for (pi, proto) in program.protos.iter().enumerate() {
        out.push_str(&format!(
            "\nproto {} {}  params=[{}] locals={} captures=[{}]\n",
            pi,
            resolve(proto.name),
            proto
                .params
                .iter()
                .map(|p| resolve(*p))
                .collect::<Vec<_>>()
                .join(" "),
            proto.n_locals,
            proto
                .capture_names
                .iter()
                .map(|c| resolve(*c))
                .collect::<Vec<_>>()
                .join(" ")
        ));
        for (pc, op) in proto.code.iter().enumerate() {
            let span = proto.debug_spans.get(pc).copied().unwrap_or_default();
            out.push_str(&format!(
                "  {:4} {:<14}{}        ; span {}..{}\n",
                pc,
                op.name(),
                op.render_operands(),
                span.start,
                span.end
            ));
        }
    }
    out
}

/// 常量池去重表（编译器内部）。
pub(crate) struct ConstPool {
    consts: Vec<BcConst>,
    index: HashMap<BcConst, u32>,
}

impl ConstPool {
    pub fn new() -> Self {
        ConstPool {
            consts: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// 按值去重 intern（§19.3：常量池去重——`(begin 1 1 1)` 不膨胀）。
    pub fn intern(&mut self, c: BcConst) -> u32 {
        if let Some(&i) = self.index.get(&c) {
            return i;
        }
        let i = self.consts.len() as u32;
        self.consts.push(c.clone());
        self.index.insert(c, i);
        i
    }

    /// 提取常量池。
    pub fn finish(self) -> Vec<BcConst> {
        self.consts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_pool_dedups() {
        let mut pool = ConstPool::new();
        let a = pool.intern(BcConst::Int(1));
        let b = pool.intern(BcConst::Int(1));
        let c = pool.intern(BcConst::Int(2));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(pool.finish().len(), 2);
    }

    #[test]
    fn disassemble_renders() {
        let proto = BcProto {
            name: Symbol(0),
            params: vec![Symbol(1)],
            n_locals: 1,
            capture_names: vec![],
            capture_sources: vec![],
            code: vec![Op::LoadLocal(0), Op::PushConst(0), Op::Add, Op::Ret],
            debug_spans: vec![
                Span::new(0, 0, 4),
                Span::new(0, 5, 6),
                Span::new(0, 0, 9),
                Span::new(0, 0, 9),
            ],
            free_vars: vec![],
        };
        let program = BcProgram {
            protos: vec![proto],
            consts: vec![BcConst::Int(1)],
            entry: 0,
            global_refs: vec![],
            module_name: None,
        };
        let text = disassemble_program(&program, &|s| format!("s{}", s.0));
        assert!(text.contains("LOAD_LOCAL"));
        assert!(text.contains("PUSH_CONST"));
        assert!(text.contains("RET"));
        assert!(text.contains("ADD"));
        assert_eq!(program.total_instructions(), 4);
    }
}
