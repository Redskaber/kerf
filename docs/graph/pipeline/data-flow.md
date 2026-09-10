# 管线数据流图（§15 项目图）

> **Author**: kerf-dev-agent
> **Date**: 2026-09-09
> **Version**: v0.1.0
> **Status**: Active

## 编译管线数据流

```mermaid
flowchart TD
    SRC["源文本 .krf"] --> READER["kerf-reader<br/>词法 + 递归下降"]
    READER -->|"Vec<Stx>（Span+Scopes+Phase）"| EXP["kerf-expander<br/>展开 + 卫生宏 + 相位"]
    EXP -->|"Vec<Rc<CoreExpr>>（9 原语）"| IR["kerf-core::lower<br/>图 IR（Arena+共享）"]
    IR -->|"IrGraph"| COMP["kerf-compiler<br/>回填 + 捕获转换"]
    COMP -->|"BcProgram（39 操作码 + debug_info）"| VM["kerf-vm<br/>switch-dispatch"]
    VM -->|"Value（堆引用）"| RT["kerf-runtime<br/>GC 堆 + I/O"]
    DRIVER["kerf-driver（编排）"] -.->|"declare/visit/instantiate"| PH["kerf-expander::phase<br/>模块相位簿记"]
    DRIVER -.->|"内置注册 + 卫生回退"| VM
```

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
