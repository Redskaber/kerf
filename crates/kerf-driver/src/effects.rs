//! 编译器内部效应系统（r8 批次 D——D1：12-roadmap §2.4.3 Stage 1
//! 「做实引入（编译器内部）」，**不进入语言语义**）。
//!
//! **架构**（interface-contract-review F1 裁定的双层 API）：
//! - **类型化逃逸层**（`handle_escape` / `perform_escape`）：一次性逃逸
//!   （one-shot escape / 浅处理器）——`perform_escape::<T>(载荷)` 从任意
//!   嵌套深度向上展开至最近的 `handle_escape::<T>` 边界，**零签名污染**
//!   （「任意流程节点能力」的机械实现——深位触发无需途经签名线程化）；
//! - **冻结契约层**（`InternalEffectSystem`）：reserved.rs `EffectSystem`
//!   P3 形状的真实现（同签名语义化做实，与 Probe 测试双证契约可实现）。
//!
//! **机制**：线程局部处理器深度 + `catch_unwind`/`resume_unwind` 载荷
//! 逃逸（std-only，零外部依赖约束保持）。私有载荷类型 `EffectUnwind`
//! 是 panic hook 的判别标记——效应逃逸**不打印 panic 噪声**（payload
//! 类型判别，仅效应载荷被抑制；真实 panic 照常报告）。
//!
//! **语义边界**（与 multi-error-recovery-design §4 联动裁定一致）：
//! - 编译期错误恢复（展开器形式级）= 控制流，**非效应**（§11 接口
//!   隔离——不把编译器内部控制流暴露到语言语义面，Stage 2 复核）；
//! - 本系统的消费面 = **运行期/工具链**跨嵌套逃逸：`kerf test` 的用例
//!   短路与错误恢复（case 内任意深度失败 → 边界捕获 → 下一用例续跑）；
//! - 多次恢复（resumption）不实现——`Continuation` 留 unit 形状（P3
//!   留白，Stage 2 语言级引入时定语义）；
//! - VM 帧 `ext1` 槽位**不激活**（review F3 边界记录：Rust 层一次性
//!   逃逸无需帧槽；ext1 激活 = Stage 2 语言级效应）。
//!
//! **构建约束**：本机制要求 unwind profile（全仓无 `panic = "abort"`
//! 配置——已核验；若未来引入 abort 配置须先重构本模块）。

use std::any::Any;
use std::cell::Cell;
use std::panic::{catch_unwind, resume_unwind, set_hook, take_hook, AssertUnwindSafe};
use std::rc::Rc;

use kerf_vm::Value;

use crate::reserved::{Effect, EffectFamily, EffectSystem};

// ---------------------------------------------------------------------------
// 1. 类型化一次性逃逸层（编译器内部机械——「任意流程节点」的载体）
// ---------------------------------------------------------------------------

/// 效应逃逸载荷包装（**私有标记类型**——panic hook 以此判别效应逃逸
/// 并抑制噪声打印；构造仅在本模块，真实 panic 不会被误吞）。
struct EffectUnwind(Box<dyn Any + Send>);

thread_local! {
    /// 处理器深度（0 = 当前线程无任何逃逸边界）。
    static HANDLER_DEPTH: Cell<u64> = const { Cell::new(0) };
}

/// 深度守卫（Drop 安全——逃逸/panic 双路径均正确递减）。
struct DepthGuard;

impl DepthGuard {
    fn install() -> Self {
        HANDLER_DEPTH.with(|d| d.set(d.get() + 1));
        DepthGuard
    }
}

impl Drop for DepthGuard {
    fn drop(&mut self) {
        HANDLER_DEPTH.with(|d| d.set(d.get() - 1));
    }
}

/// 安装一次性逃逸边界并运行计算：期间任何 `perform_escape::<T>` 从
/// **任意嵌套深度**向上展开至此。
///
/// 返回 `Ok(计算结果)`；返回 `Err(载荷)` = 该边界捕获的 T 型效应。
/// 非本类型载荷（外层类型的效应或真实 panic）原样上抛（最近匹配
/// 语义——内层同类型边界必然先捕获）。
pub fn handle_escape<R, T: Any + Send>(computation: impl FnOnce() -> R) -> Result<R, T> {
    let _guard = DepthGuard::install();
    match catch_unwind(AssertUnwindSafe(computation)) {
        Ok(v) => Ok(v),
        Err(payload) => match payload.downcast::<EffectUnwind>() {
            Ok(inner) => match inner.0.downcast::<T>() {
                Ok(t) => Err(*t),
                // 非本类型效应载荷：重新包装上抛（外层边界接手）
                Err(other) => {
                    resume_unwind(Box::new(EffectUnwind(other)));
                }
            },
            // 真实 panic：原样上抛（不吞、不改——报错>静默 §2.3-4）
            Err(real) => resume_unwind(real),
        },
    }
}

/// 执行一次性效应：从当前调用深度向上展开至最近的
/// `handle_escape::<T>` 边界（无匹配边界则带清晰消息终止——不产生
/// 不透明载荷 panic）。
///
/// **不返回**（发散）——一次性逃逸无恢复续体（resumption 属 Stage 2
/// 语言级语义）。
pub fn perform_escape<T: Any + Send>(payload: T) -> ! {
    let has_handler = HANDLER_DEPTH.with(|d| d.get() > 0);
    if !has_handler {
        // 无任何边界：清晰终止（与 OCaml 未处理异常语义同构——
        // 「效应逃逸无处理器」为程序员错误，必须可读可定位）
        panic!(
            "效应逃逸无处理器（perform_escape 于深度 0 调用）——载荷类型 {}",
            std::any::type_name::<T>()
        );
    }
    resume_unwind(Box::new(EffectUnwind(Box::new(payload))))
}

// ---------------------------------------------------------------------------
// 2. 冻结契约层（reserved.rs EffectSystem P3 形状的真实现）
// ---------------------------------------------------------------------------

/// 值效应族（冻结契约的具体化——`Result = Value`，P3 形状以 Value 具体化）。
pub struct ValueEffectFamily;

impl EffectFamily for ValueEffectFamily {
    type Result = Value;
}

/// 值效应（载荷 = kerf Value——P3 形状约束）。
pub struct ValueEffect {
    /// 效应载荷（Str 无损往返；标量经渲染编组——F1 适配层成本，
    /// Stage 2 泛型关联细化时消除）。
    pub payload: Value,
}

impl Effect for ValueEffect {
    type Family = ValueEffectFamily;
}

/// 值处理器（载荷 → 结果值的函数）。
pub struct ValueHandler {
    /// 处理函数（`Fn` 共享——处理器可在多边界间复用）。
    pub f: Rc<dyn Fn(Value) -> Value>,
}

/// 内部效应系统（冻结契约真实现——与 Probe 实现测试构成契约可实现
/// 双证：Probe 证明形状可编译，本类型证明语义可承载）。
///
/// 载荷编组：Value 经 `payload_to_sendable` 字符串化（unwind 载荷须
/// `Send`，而 `Value` 含 `Rc` 非 Send——F1 裁定的适配成本），处理器
/// 收到 `Value::Str` 形态。
pub struct InternalEffectSystem;

impl InternalEffectSystem {
    /// Value → Send 载荷编组（Str 无损；标量渲染；复合经类型名——
    /// 编组能力边界已在文档声明）。
    fn payload_to_sendable(v: &Value) -> String {
        match v {
            Value::Str(s) => s.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Nil => "nil".to_string(),
            Value::Unit => "unit".to_string(),
            Value::Symbol(s) => s.to_string(),
            other => format!("<{}>", other.type_name()),
        }
    }
}

impl EffectSystem for InternalEffectSystem {
    type Effect = ValueEffect;
    type Handler = ValueHandler;
    /// 一次性逃逸语义下无恢复续体（P3 留白——Stage 2 语言级引入时
    /// 具体化，届时本关联类型变更属契约内演进）。
    type Continuation = ();

    fn perform(&self, effect: Self::Effect) -> Value {
        // 编组为 Send 载荷后逃逸；最近 ValueHandler 边界接管并返回
        // 处理结果（本方法调用点视作不返回——一次性逃逸）
        perform_escape(ValueEffectSendable(Self::payload_to_sendable(
            &effect.payload,
        )))
    }

    fn handle(&self, handler: Self::Handler, computation: impl FnOnce() -> Value) -> Value {
        match handle_escape::<Value, ValueEffectSendable>(computation) {
            Ok(v) => v,
            Err(payload) => {
                // 载荷回编为 Value（Str 形态——F1 适配层的往返成本）
                (handler.f)(Value::Str(Rc::from(payload.0.as_str())))
            }
        }
    }
}

/// 冻结契约载荷的 Send 载体（编组后字符串——见 `payload_to_sendable`）。
struct ValueEffectSendable(String);

// ---------------------------------------------------------------------------
// 3. panic hook 过滤（效应逃逸零噪声——私有类型判别）
// ---------------------------------------------------------------------------

/// 安装全局 hook 过滤（进程一次；幂等）。
///
/// 判别规则：payload 为 `EffectUnwind` → 抑制打印（效应控制流，非
/// 错误）；其余 → 链回前序 hook（真实 panic 照常报告）。私有类型
/// 保证判别不可伪造——用户 panic 不可能被误吞。
static HOOK_INSTALLED: std::sync::Once = std::sync::Once::new();

pub(crate) fn ensure_hook_installed() {
    HOOK_INSTALLED.call_once(|| {
        // 链式替换：前序 hook 保留（真实 panic 照常报告）；效应逃逸
        // 载荷（私有类型判别——不可伪造）静默抑制
        let prev = take_hook();
        set_hook(Box::new(move |info| {
            if info.payload().downcast_ref::<EffectUnwind>().is_some() {
                // 效应逃逸载荷：控制流而非错误——不打印
                return;
            }
            prev(info);
        }));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install() {
        ensure_hook_installed();
    }

    #[test]
    fn handle_returns_computation_result() {
        install();
        let r = handle_escape::<i32, String>(|| 42);
        assert_eq!(r, Ok(42));
    }

    #[test]
    fn perform_unwinds_to_nearest_boundary() {
        install();
        let out = handle_escape::<i64, String>(|| {
            // 三层嵌套深处逃逸——零签名污染（内层闭包签名不涉效应）
            fn level3() -> i64 {
                perform_escape::<String>(String::from("深度逃逸"));
            }
            fn level2() -> i64 {
                level3()
            }
            fn level1() -> i64 {
                level2()
            }
            level1()
        });
        assert_eq!(out, Err(String::from("深度逃逸")));
    }

    #[test]
    fn nearest_matching_boundary_wins() {
        install();
        // 内层 String 边界捕获 String 效应；外层 u32 边界保持完好
        let inner_caught = handle_escape::<bool, u32>(|| {
            let caught = handle_escape::<i64, String>(|| {
                perform_escape::<String>(String::from("内层"));
            });
            caught.is_err()
        });
        // Ok(true) = 内层捕获且外层零触发；Err = 外层被穿透（违最近匹配）
        assert!(matches!(inner_caught, Ok(true)));
    }

    #[test]
    fn outer_boundary_receives_non_local_payload() {
        install();
        // 内层 u32 边界不匹配 String 载荷 → 上抛至外层 String 边界
        let outer = handle_escape::<(), String>(|| {
            let _inner_saw_it = handle_escape::<(), u32>(|| {
                perform_escape::<String>(String::from("穿透"));
            });
            unreachable!("内层边界不匹配 String，不应到达此处")
        });
        assert_eq!(outer, Err(String::from("穿透")));
    }

    #[test]
    fn payload_keeps_type_fidelity() {
        install();
        #[derive(Debug, PartialEq)]
        struct Case {
            id: u32,
            detail: String,
        }
        let r = handle_escape::<(), Case>(|| {
            perform_escape::<Case>(Case {
                id: 7,
                detail: "载荷保真".into(),
            })
        });
        assert_eq!(
            r,
            Err(Case {
                id: 7,
                detail: "载荷保真".into()
            })
        );
    }

    #[test]
    fn multiple_payload_types_coexist() {
        install();
        // 同一计算内不同类型效应按最近同类型边界分发
        let r = handle_escape::<(), &'static str>(|| {
            let _ = handle_escape::<(), i64>(|| perform_escape::<i64>(99));
            perform_escape::<&'static str>("str 载荷")
        });
        assert_eq!(r, Err("str 载荷"));
    }

    #[test]
    #[should_panic(expected = "效应逃逸无处理器")]
    fn perform_without_boundary_terminates_clearly() {
        install();
        perform_escape::<String>(String::from("无边界"));
    }

    #[test]
    fn real_panics_pass_through_boundary() {
        install();
        // 真实 panic 穿透效应边界（不吞）——由外层 catch_unwind 断言
        let caught = catch_unwind(AssertUnwindSafe(|| {
            let _ = handle_escape::<(), String>(|| {
                panic!("真实 panic");
            });
        }));
        assert!(caught.is_err(), "真实 panic 必须穿透效应边界");
    }

    #[test]
    fn frozen_contract_handle_computation_ok() {
        install();
        let sys = InternalEffectSystem;
        let v = sys.handle(
            ValueHandler {
                f: Rc::new(|v| match v {
                    Value::Str(s) => Value::Str(Rc::from(format!("已处理：{}", s))),
                    other => other,
                }),
            },
            || Value::Int(42),
        );
        assert!(matches!(v, Value::Int(42)));
    }

    #[test]
    fn frozen_contract_perform_unwinds_to_handle() {
        install();
        let sys = InternalEffectSystem;
        let v = sys.handle(
            ValueHandler {
                f: Rc::new(|v| match v {
                    Value::Str(s) => Value::Str(Rc::from(format!("效应已处理：{}", s))),
                    other => other,
                }),
            },
            || {
                // 冻结契约 perform：从嵌套闭包深处触发（无签名污染）
                fn deep(sys: &InternalEffectSystem) -> Value {
                    sys.perform(ValueEffect {
                        payload: Value::Str(Rc::from("中止信号")),
                    })
                }
                deep(&sys)
            },
        );
        match v {
            Value::Str(s) => assert_eq!(&*s, "效应已处理：中止信号"),
            other => panic!("处理器应收到 Str 载荷，实际 {:?}", other.type_name()),
        }
    }

    #[test]
    fn frozen_contract_nested_boundaries() {
        install();
        let sys = InternalEffectSystem;
        // 外层标记 +1，内层标记 +2；效应被内层处理器消费
        let outer_hits = Rc::new(std::cell::Cell::new(0u32));
        let inner_hits = Rc::new(std::cell::Cell::new(0u32));
        let o = Rc::clone(&outer_hits);
        let i = Rc::clone(&inner_hits);
        let v = sys.handle(
            ValueHandler {
                f: Rc::new(move |v| {
                    o.set(o.get() + 1);
                    v
                }),
            },
            || {
                sys.handle(
                    ValueHandler {
                        f: Rc::new(move |_v| {
                            i.set(i.get() + 1);
                            Value::Int(7)
                        }),
                    },
                    || {
                        sys.perform(ValueEffect {
                            payload: Value::Str(Rc::from("内层")),
                        })
                    },
                )
            },
        );
        assert!(matches!(v, Value::Int(7)));
        assert_eq!(inner_hits.get(), 1, "内层处理器应恰好消费一次");
        assert_eq!(outer_hits.get(), 0, "外层处理器不应被触发");
    }

    #[test]
    fn handler_depth_recovers_after_escape() {
        install();
        // 逃逸后深度正确递减（Drop 守卫）——后续边界正常工作
        let first = handle_escape::<(), u8>(|| perform_escape::<u8>(1));
        assert_eq!(first, Err(1));
        let second = handle_escape::<(), u8>(|| perform_escape::<u8>(2));
        assert_eq!(second, Err(2));
    }
}
