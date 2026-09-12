# r30 批次 J 执行：J3 FFI VM 面做实（48-d——会话 Task 51-a/51-z/51-web）

> Task ID: 51-a/51-z/51-web（plan §5b MUV 48-d / §21.3 条件 4 兑现）· 2026-09-12 · 溯源：flat worklog（kerf/docs/worklog.md 尾部条目 51-a + root worklog 索引 51-z/51-web）· RELEASE_NOTES v0.4.0-r30
> **补建注记**：本条目由 r31/52-a 会话补录（r30 会话上下文耗尽于 flat 51-z/51-web 落笔前——详录以 root worklog 索引条目为准；本条目 1:8 压缩自该索引 + flat 51-a）。

## 概要（1:8 压缩——详录见 flat 51-a + root 索引）

- **操作码三指令 43→46**（CALL_EXTERNAL/ALLOC_EXTERNAL/
  FREE_EXTERNAL——04 §1 第十组 + compiler.krf OP-* 常量表 +
  bootstrap_compiler.rs 桥三臂三方冻结同步；**语言面形式 Stage 3，
  编译臂不发射**——plan §5b 排程注 3 如实注记）。
- **窗口规程步 2-4（ffi-ownership-model §2.1）**：装载边界（char*
  实参 Str 值装箱堆槽 pin Φ[v]+=1 + Rc<str> 数据指针借用出界 +
  令牌校验 Invalid→E0010）→ 宿主调用（VM 挂起——外部零 kerf 分配，
  case 6 时序）→ **unpin 配平先于返回包装**；返回三分法（CInt→Int /
  NullPtr→空令牌 / CPtr→移交令牌 / Opaque→外部持有）。
- **Heap Φ 计数簿**：pin_object（P1 累计）/unpin_object（U1 归零摘根；
  U2 下溢 Err——E8 口径）+ supp(Φ) 并入 mark 起点（F-PIN 引理——
  根集第五来源计数化）。
- **线性令牌状态机**：Value::External(Rc<ExternalToken>) 消费全局生效
  （共享 Cell）+ NULL 哨兵 no-op（case 13）+ 外部域 = Rust 宿主堆
  Box<[u8]>（真 C ABI = QBE AOT）。
- **E0010-E0012 诊断族落位**（18 §6 r18 预留兑现——messages.rs 单源
  三构造器；E0012 fail-closed 符号解析）。
- **§21.3 条件 4 冻结形状消费面**：HeapFfiBoundary（FfiBoundary 真实
  实现）+ compile_ffi_call_program（FfiCall 字面量 lowering——
  size=0 编译期拒绝 + FreeExternal 令牌依赖拒绝——诚实收窄）+
  default_extern_table（write_stdout char* 注册）+ run_program_with_externs。
- **ffi_vm_tests 37 case**：13 边界 case 全判定落地 + 正负比 7:22
  功能点粒度 ≥1:3 + Φ/P-U 机制组 6；driver ffi 单元 8——
  **758:0:0 零回归 + 净 45**；TD-026 登记（P3：语言面形式 +
  编组消费子集收窄——Stage 3 锚）。
- **GATE 1**：§3.2 六命令实测全绿（build 13.42s / check 0/0 / fmt 0 /
  clippy --workspace 超集 0 / test 758:0:0）+ 三审计集 EXIT 0 ×3 +
  CLI 四路径；回写六面（06/05/13/04/18/TD）+ 对账（matrix r30 +
  pipeline r30 + plan 注记 + RELEASE_NOTES r30）。
- **51-z 收尾**：r30 tar.gz（323 条目 / 1.87MB）+ 包内自举（全新解包
  构建 13.25s + 758 复跑 + CLI 三路径 + 包内三审计集）+ download
  README r30 节；51-web web 面：kerf-data r30 三节点 + footer v7.0 +
  E2E 双端全过（758 ×10 / r30 ×20 / FFI ×24 关键词命中 + Playground
  金路径 + footer 贴底亚像素 + 移动端零横溢）——git kerf fb8bc69 /
  web 578f63a。

## 下一步

48-e J4 批次 J 收尾（§3.2 + 对账 + r 末 tar.gz 包内自举 + web 同步 +
12 §2.5.1 行注记终态复核）→ 批次 K 终批（§14.6 阶段间深验证 +
§21.5 九信号 + Stage 3 切换评估）。
