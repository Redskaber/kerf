# r25 批次 I 执行：I 后段双主题（Effect M1-M5 语言级效应全落地 + 能力管线泛化 M2）

> Task ID: 46-a/46-z（plan 42-f + 收尾）· 2026-09-11 · 溯源：flat worklog（kerf/docs/worklog.md 尾部条目）· RELEASE_NOTES v0.4.0-r25

## 概要（1:8 压缩——详录见 flat）

- **Effect M1-M5 全交付（46-a 主体）**：原语集 9→11（perform/handle
  入 CoreExpr——resume 非原语，D4 脱糖为 (κ v) App）；VM ext1 具体化
  （HandlerFrame：分派键/原型/捕获单元/水位——原则 27 预留兑现）+
  INSTALL_HANDLER(41)/PERFORM(42) 两指令（43 项三方冻结）；三原型帧
  编排（T trampoline 惰性单例 / H handler / B body thunk——编译器零
  CLOSURE/CALL 发射，帧编排原子化）；continuation 四要素（捕获帧链
  含 handler 帧 + 数据栈快照 + 恢复点 + 线性唯一 consumed Cell——
  E0008 含首恢位置追踪）；TCO 与浅处理正交（10 万深尾递归穿透）。
- **诊断族 E0007-E0009 + GC 六来源**：messages.rs 单源（TD-018 纪律
  ——18 §6 码位登记表 W3）；E0007 逃逸（tag 名符号值直渲染）/E0008
  二次恢复/E0009 元数（非 continuation 值面归 E0004——v1.1 注记⑤）；
  Value::Continuation 根集递归（visited 防嵌套环）+ GcCell has_heap
  + ForeignBox 装箱/解箱追踪——effect_stress 万级分配挂起链存活
  ⇒120。
- **M3 双路径新口径 + native 拒绝**：种子/生产双编译链一致 8 case +
  eval 域 Perform→EvalError.effect 逃逸通道 dispatch 3 case（D7）；
  anf/qbe native 显式拒绝（B1 同型——效应语义由 VM 承载）。
- **能力管线泛化 M2（D12 同轮）**：IoFamily 形状标记（IoGrant =
  Grant<io 族> trait 别名兼容——冻结路径零删改）+ 门控表 net 增行
  评估维持不增行（Stage 2 末窗口零破坏纪律）+ 机器锚 2。
- **质量口径（46-z 收尾复验）**：**706:0:0**（684 零回归 + 净 22：
  effect 17 + bootstrap 效应 3 + M2 2）；§3.2 六命令 clean 起步全绿
  （build 13.87s + **clippy --all-targets --workspace 超集口径 0 警告**
  ——历史命令无 --workspace 在根包工作区下不覆盖成员 crate，本环
  发现并清偿 recover.rs atom_form 死代码（r21 遗留零调用者））；
  r25 tar.gz 311 条目（**§19.3 复位：commit 先行——本 rec 与 flat
  46-z 并包**）+ 包内自举验证两轮（全新构建 14.17s + 706 复跑 +
  CLI 四路径一致：fib 75025/144 + macros (2 1)⇒42 + effect_stress
  ⇒120 + 基础恢复 ⇒16）；性能同会话交错对拍（fib +3.9% /
  gc_stress +4.5% 双带内；非尾形 +6.2% 带外 1pct 如实归因 §4.2——
  对照实验排除单点主因）。
- **下一步**：42-g I3 门审查（§7.3 ≥30 新 case + §21.3 四条锚定 +
  §14 阶段末环）→ 42-h 收尾交付。
