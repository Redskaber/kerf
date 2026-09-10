# 管线数据流图（§15 项目图）

> **Author**: kerf-dev-agent
> **Date**: 2026-09-10（r7：批次 C——编译缓存旁路（compile_front_cached 内容寻址键命中返回前端快照）+ 静态检查器旁路（check_program）+ check 消费面；r6：B3 自举 Reader 读路径——生产 read 经 kerf 源码 Reader（VM 上运行），种子保留为引导 + oracle）
> **Version**: v0.1.1
> **Status**: Active

## 编译管线数据流

```mermaid
flowchart TD
    SRC["源文本 .krf"] --> BREADER["自举 Reader（B3，r6）<br/>reader.krf（kerf 源码）<br/>lex-src / parse-tokz 两入口"]
    subgraph BOOT["自举加载（thread_local 惰性，每线程一次）"]
        SEED["种子 kerf-reader<br/>（Rust）"] -->|"compile_front_seed<br/>编译 reader.krf"| BPROG["BcProgram<br/>（Reader 字节码）"]
        BPROG -->|"run_program 顶层定义"| BGLB["Reader 全局<br/>（含 hofs）"]
    end
    BREADER -->|"call_closure（VM 宿主调用）"| VMEXEC["kerf-vm 执行<br/>（持久堆 + GC）"]
    VMEXEC -->|"值树（Token/datum 编码）"| BRIDGE["bootstrap 桥<br/>值树→Token/Stx<br/>数字同源 parse"]
    BRIDGE -->|"Vec<Stx>（Span+Scopes+Phase）"| EXP["kerf-expander<br/>展开 + 卫生宏 + 相位"]
    EXP -->|"Vec<Rc<CoreExpr>>（9 原语）"| COMP["kerf-compiler<br/>回填 + 捕获转换"]
    COMP -->|"BcProgram（40 操作码 + debug_info）"| VM["kerf-vm<br/>switch-dispatch"]
    VM -->|"Value（堆引用）"| RT["kerf-runtime<br/>GC 堆 + I/O"]
    EXP -.->|"仅消费方（ir/code dump）：compile_source 完整入口"| IR["kerf-core::lower<br/>图 IR（Arena+共享）"]
    DRIVER["kerf-driver（编排）"] -.->|"declare/visit/instantiate"| PH["kerf-expander::phase<br/>模块相位簿记"]
    DRIVER -.->|"内置注册 + 卫生回退（含 4 Reader 原语）"| VM
    DRIVER -.->|"compile_front 快路径（TD-015：run/eval 不构造 IR）"| COMP
    SEED -.->|"parity oracle（bootstrap_reader_tests）"| BREADER
    CACHE["InMemoryCompilationCache（r7）<br/>内容寻址键：SHA-256(源)+指纹(种子+文件名)<br/>FrontOutput 快照克隆"] -.->|"命中：跳过 read/expand/compile"| DRIVER
    DRIVER -.->|"未中：编译后 store_front"| CACHE
    TC["kerf-compiler::typecheck（r7）<br/>check_program R1-R8<br/>多错误收集（E0005）"] -.->|"check_source 消费（旁路报告，不影响执行产物）"| DRIVER
    DRIVER -.->|"签名表注入（builtin_sigs）"| BUILTINS["kerf-driver::builtins<br/>BUILTIN_SIGS 49 项"]
```

> **B3 读路径注记（r6）**：生产 read（`compile_front`/`dump_tokens`/
> `dump_stx`）全部经自举 Reader；种子（kerf-reader）承担两职责——①
> 引导：编译 reader.krf 本身（无递归——种子编译自举实现）；② oracle：
> parity 套件逐字节对照（正 87 / 负 307 case）。错误经 ('err 消息 起 止)
> 值编码返回（不依赖异常）；VM 级内部错误防御路径经 reader.krf 源映射渲染。

## 依赖图（Cargo Workspace，§18.5 的定向裁定）

```mermaid
flowchart TD
    subgraph Crates["Cargo Workspace（零外部依赖）"]
        SPAN["kerf-span<br/>Span/SourceMap/诊断"]
        SYN["kerf-syntax<br/>Symbol/ScopeSet/Stx"]
        CORE["kerf-core<br/>CoreExpr/IR/CodeValue"]
        READER["kerf-reader"]
        EXP["kerf-expander"]
        COMP["kerf-compiler"]
        RT["kerf-runtime<br/>Layer 0 基座（无依赖）"]
        VM["kerf-vm"]
        DRV["kerf-driver"]
    end
    SPAN --> SYN
    SYN --> READER
    SPAN --> CORE
    SYN --> CORE
    SYN --> EXP
    CORE --> EXP
    CORE --> COMP
    RT --> VM
    COMP --> VM
    READER --> DRV
    EXP --> DRV
    COMP --> DRV
    VM --> DRV
    RT --> DRV
```

**定向裁定**（worklog Task 2）：§18.5 图的 L6→L7（vm→runtime）边按
§2.4.5「Layer 0 = 最小 I/O Runtime（无依赖）」裁定为 **kerf-vm 依赖
kerf-runtime**（运行时为 VM 提供对象模型与 GC 基座）；Op 定义于
kerf-compiler（字节码为编译产物、VM 为消费者——可替换性原则 4）。
