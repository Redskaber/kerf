# r24 批次 I 执行：I2 stdlib/GC/TD 批（五债清偿 + 谓词/foldr 补齐 + TD-023 根扫描对症）

> Task ID: 45-a/45-z（plan 42-e + 收尾）· 2026-09-11 · 溯源：flat worklog（kerf/docs/worklog.md 尾部条目）· RELEASE_NOTES v0.4.0-r24

## 概要（1:8 压缩——详录见 flat）

- **TD-010 闭包/内置装箱 resolved**：`HeapObj::Foreign(Rc<ForeignBox>)`
  （any: Rc<dyn Any> 类型擦除 Rc 载体——解箱往返恒等 → eq? 按引用）+
  **ForeignTracer 追踪协议**（装箱方注入——标记阶段 children 枚举闭包
  捕获图 Pair 子引用；kerf-runtime 不依赖 kerf-vm 类型 §11）；函数列表
  模式就位（(list f g) + map 应用——stdlib 库化前提）。
- **TD-011 字符串全序 resolved**：全字符串链按 Unicode 码点序参与全部
  比较族（Rc<str> 比较 = UTF-8 字节序 = 码点序——编码保序性）；混合链
  保持「需要数值」（TD-016 口径）；静态面 R3：Ordering ≡ NumOrAllStr。
- **TD-014 嵌套 define 归因 resolved**：专门消息「嵌套 define 重复绑定」
  + Span = 第二次出现处；seed/自举双侧镜像 + parity 2 case 逐字。
- **TD-018 消息单源 resolved**：messages.rs 五族构造器（三消费面同源）+
  Value::truthy 复活为单一实现（死助手 → Result 化）；对拍回归锚断言
  VM/eval 消息文本相等；未绑定族裁定保留（阶段信息差异）。
- **TD-023 根扫描对症 resolved**：基准重定型先行（gc_stress_nontail
  非尾形锚定——改前 144.06ms + 2×→3.2~3.7× 超线性实测成立）；对症
  双件 = GcCell 堆根性摘要（非堆单元 O(1) 跳过 + 写路径 sound 不变式
  回归锚）+ 根扫描缓冲跨周期复用（无分配化路径 B）；**stash 重建 r23
  二进制同会话对拍：非尾形 -27.1%（144.06→105.15ms），尾形 -2.8% /
  fib -3.1% 双噪声带（§14.6.4 验收 ≤5% ✓）**；残留超线性如实归因 =
  帧栈结构性成本（非分配模式缺陷——已治愈）。
- **TD-008 分代 GC/堆压缩裁定 DEFER（Stage 3+ 条件触发）**：实测依据
  三面（①收益面不存在——Stage 2 无长驻程序 ②分代对栈根无通用免除 +
  压缩破坏 GcRef 契约 ③§12 复杂度预算——42-f/42-g 优先）。
- **stdlib 缺口补齐（清单清零——09-stdlib v6.3）**：谓词 5 件（string?
  /symbol?/float?/number?/list?——Floyd 龟兔环安全）+ prelude foldr
  （对偶语义可观测锚：foldr 2 vs foldl -6）+ 57 项清单对齐。
- **质量口径**：**684:0:0**（670 零回归 + 净 14——stdlib +7 / gc +3 /
  prelude +3 / scope_set +1）；§3.2 六命令 clean 起步全绿；CLI 冒烟
  七路径（含新谓词/字符串序 + native print-free 先例口径 + E0006
  exit 1 + eval 退役 exit 2——冒烟管道 exit 码陷阱实测修正）；双审计
  EXIT 0；r24 tar.gz（307 条目）+ 包内自举验证（12.08s + 684 复跑 +
  CLI 一致）；web 同步（kerf-data r24 + footer v6.4）+ 对账七面
  （perf-baseline §4.1 四口径对拍表 + §9.1 复测记录 + matrix 表体四行
  + TD 登记册 + plan + pipeline + 09-stdlib）。
- **I2 段交付闭环**：下一步 42-f I 后段（Effect M1-M5 + 能力 M2 同轮）
  → 42-g I3 门审查（§7.3 ≥30 新 case + §21.3 四条锚定 + §14 阶段末环）。
