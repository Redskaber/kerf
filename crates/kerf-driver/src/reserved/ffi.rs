//! FFI 边界接口预留（stage0.md §9.3.4 / lang-design 13 §3.3.4——P1，
//! next4 第八轮吸收，r12 批次冻结）。
//!
//! **为什么预留**：系统级语言必须与 C/系统 API 交互——类型系统必须能表示
//! 「外部类型」，GC 必须能识别「外部引用」（pin/unpin 协议）。
//!
//! **自举合规注记**：FFI 的**实现**推迟至 Stage 2（破坏自举闭环，13 §3.2）；
//! 本预留仅为类型形状与 GC 边界协议，不引入对宿主 C ABI 的编译期依赖，
//! 不触碰自举链（§21 后端策略同口径）。

use kerf_core::CoreExpr;
use kerf_syntax::Symbol;
use kerf_vm::Value;

// ---------------------------------------------------------------------------
// 外部类型表示（P3 形状——C ABI 类型系统子集）
// ---------------------------------------------------------------------------

/// C 整型尺寸（C ABI 对齐口径）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CIntSize {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    Isize,
    Usize,
}

/// 指针指向类型（P3 形状——指向物描述）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointeeType {
    /// `void*`——不透明。
    Void,
    /// `uint8_t*`——字节缓冲。
    U8,
    /// `char*`——NUL 结尾字符串。
    Char,
}

/// 外部函数调用的类型表示（P3 形状——递归 C 类型子集）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExternalType {
    /// C 整型。
    CInt(CIntSize),
    /// C 指针。
    CPointer(PointeeType),
    /// C 结构体（字段序 = 内存布局序）。
    CStruct(Vec<ExternalType>),
    /// C 函数类型（参数 → 结果）。
    CFunction {
        param: Box<ExternalType>,
        result: Box<ExternalType>,
    },
    /// 不透明的外部类型（经头文件声明，不可构造）。
    Opaque(String),
}

// ---------------------------------------------------------------------------
// FFI 调用原语（P3 形状——核心表达式可表示「外部调用」的预留位置）
// ---------------------------------------------------------------------------

/// FFI 调用的核心原语（Stage 2 编译目标；Stage 0 仅冻结形状）。
///
/// **预留契约**：核心表达式必须能表示「外部调用」——本枚举即该表示的
/// 形状冻结；外部内存分配/释放不经过 GC（显式生命周期）。
#[derive(Debug, Clone, PartialEq)]
pub enum FfiCall {
    /// 调用外部函数（符号 + 实参 + 返回类型）。
    CallExternal {
        symbol: Symbol,
        args: Vec<CoreExpr>,
        return_type: ExternalType,
    },
    /// 分配外部内存（不经过 GC）。
    AllocExternal { size: usize },
    /// 释放外部内存。
    FreeExternal { ptr: CoreExpr },
}

// ---------------------------------------------------------------------------
// FFI 边界（GC 与外部内存的隔离协议——P1）
// ---------------------------------------------------------------------------

/// FFI 边界（GC 隔离协议）。
///
/// **完整行为规格**（P1——Stage 2 可直接按规格实现）：
/// 1. `pin_object`：标记对象被外部代码引用——GC **不可回收**（引计数
///    式屏障，进入根集）；
/// 2. `unpin_object`：解除标记（外部释放引用后）；
/// 3. `ExternalPointer`：外部指针的包装类型——包装值不受 GC 追踪，
///    解引用为显式操作；
/// 4. 与 05-运行时 §3 分配器协议的关系：「FFI 外部引用追踪」是其
///    既登记的演进方向——本 trait 是该方向的接口冻结。
pub trait FfiBoundary {
    /// 外部指针的包装类型。
    type ExternalPointer;

    /// 标记对象被外部代码引用（GC 不可回收）。
    fn pin_object(&mut self, obj: Value);

    /// 解除外部引用标记。
    fn unpin_object(&mut self, obj: Value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use kerf_core::LiteralValue;
    use kerf_span::Span;

    /// P1 冻结性测试：FFI 形状以「测试实现体编译通过」证明契约冻结。
    #[derive(Default)]
    struct ProbeFfiBoundary {
        pinned: u32,
    }
    impl FfiBoundary for ProbeFfiBoundary {
        type ExternalPointer = *const u8;
        fn pin_object(&mut self, _obj: Value) {
            self.pinned += 1;
        }
        fn unpin_object(&mut self, _obj: Value) {
            self.pinned = self.pinned.saturating_sub(1);
        }
    }

    #[test]
    fn ffi_reserved_signatures_are_frozen() {
        let mut b = ProbeFfiBoundary::default();
        b.pin_object(Value::Nil);
        assert_eq!(b.pinned, 1);
        b.unpin_object(Value::Nil);
        assert_eq!(b.pinned, 0);
    }

    #[test]
    fn external_type_is_recursive_and_hashable() {
        // CStruct/CFunction 递归形状 + 内容寻址可哈希（缓存键口径）
        let t1 = ExternalType::CStruct(vec![
            ExternalType::CInt(CIntSize::I32),
            ExternalType::CPointer(PointeeType::U8),
        ]);
        let t2 = ExternalType::CStruct(vec![
            ExternalType::CInt(CIntSize::I32),
            ExternalType::CPointer(PointeeType::U8),
        ]);
        assert_eq!(t1, t2);
        let t3 = ExternalType::CFunction {
            param: Box::new(ExternalType::CInt(CIntSize::I64)),
            result: Box::new(ExternalType::Opaque("FILE".into())),
        };
        assert_ne!(t1, t3);
        // Hash 派生可用（内容寻址）
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h1 = DefaultHasher::new();
        t1.hash(&mut h1);
        let mut h2 = DefaultHasher::new();
        t2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn ffi_call_carries_core_expr_arguments() {
        // 「核心表达式必须能表示外部调用」——FfiCall 的参数槽以 CoreExpr
        // 承载（形状证明：可构造、可携带 Span 语义值）
        let arg = CoreExpr::Literal {
            value: LiteralValue::Int(7),
            span: Span::dummy(),
        };
        let call = FfiCall::CallExternal {
            symbol: Symbol(3),
            args: vec![arg.clone()],
            return_type: ExternalType::CInt(CIntSize::I32),
        };
        match &call {
            FfiCall::CallExternal { symbol, args, .. } => {
                assert_eq!(*symbol, Symbol(3));
                assert_eq!(args.len(), 1);
            }
            _ => unreachable!("构造即 CallExternal"),
        }
        let alloc = FfiCall::AllocExternal { size: 128 };
        assert_eq!(
            alloc,
            FfiCall::AllocExternal { size: 128 },
            "usize 尺寸字段可比较"
        );
        let _free = FfiCall::FreeExternal { ptr: arg };
    }
}
