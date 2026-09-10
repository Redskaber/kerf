# 设计-实现-测试三者覆盖对照（§14.6.1.3）

> **Author**: kerf-coverage-hidden subagent（Task 15-b，角色 QA-A/ARCH-A）
> **Date**: 2026-09-10
> **Version**: v0.1.0-r3-coverage
> **Status**: Active
> **基线**: lang-design v5.2 / 294 测试函数（290 通过 + 4 文档化忽略，0 失败，release 实测复验）/ 40 操作码 / 24 内置 / 审计集 41 case 全 PASS（本报告实测复跑）
> **审查方法**: §14.6.1.3 四问（设计→实现 / 实现→测试 / 测试→设计 / 三者一致性）；逐文件对照 `docs/lang-design/` 00-19 的可验证设计点（以各文件「处理程度」头 + §测试锚点节界定 Stage 0 范围），实现状态经读码核实（file:line 证据），**全部测试锚点经 Grep 逐一验证真实存在（本报告 §5 抽验记录，73 处锚点名核验，全部命中，零编造）**；测试计数经 `cargo test --release --workspace` 实测（290/0/4）

---

## 1. 执行摘要

- **三者覆盖总判定：PASS（带 2 项 P3 级残余差距）**。133 个 Stage 0 范围设计点中：**128 行完整闭环**（✅/✅）；5 项 DEFERRED（3.8% ≤ 5% 门限，均附理由与登记锚——保守口径按设计承诺面计）；1 项（3 行：let*/when/unless 糖）已实现、负向已测、正向推导断言缺位；另有 1 项文档级轻微不一致（01 §5 测试锚点表对糖推导正向覆盖的表述超前于实测）。
- v5.2 回写（deep-review Round 1 偏差 26 项 + Task 16 修复）后，r1 审查的四类三方漂移（操作码 39/40/41、内置 20/24、App 顺序分裂、文档计数失真）**全部清零**：40 操作码「enum ↔ 守护测试 ↔ 04 文档」三方一致（实测 `opcode_count_matches_spec` 断言 40 且逐项枚举）；24 内置与 09 v5.2 清单逐项一致；App 顺序双路径统一为函数先（`app_evaluates_fn_then_args_left_to_right` + `app_evaluation_order_fn_first_dual_path` 双锚点在位）。
- E 码覆盖：E0/E1–E6、E0001/E0002/E0004 全部有直接断言矩阵；E7/E8/E0003 为不可构造类，按 §9.4.3 规则 2 以文档化存档为验证口径（见 §3 DEFERRED 清单）。

---

## 2. 三列对照表（设计点 / 实现状态 / 测试状态）

> 实现状态列给出 file:line 或 file 证据；测试状态列给出**已验证存在**的锚点（单元 = crate 内 `#[cfg(test)]`，集成 = tests/v0/stage0/）。抽验记录见 §5。

### 2.1 01-core-forms（九原语 + 语法对象 + 糖推导，21 点）

| # | 设计点（01 §2/§3） | 实现状态 | 测试状态 |
|---|---------------------|----------|----------|
| 1 | `Lambda of {params; body}` | ✅ kerf-core/src/expr.rs:80（span 全节点） | ✅ `expr::tests::render_core_forms` / `expander_tests::expand_lambda_and_app`（单元）/ `vm_tests::nine_primitives_semantics` |
| 2 | `App of {fn; args}` | ✅ expr.rs:86 | ✅ 同上 + `compile::tests::app_evaluates_fn_then_args_left_to_right`（字节码序）+ `vm_tests::app_evaluation_order_fn_first_dual_path`（双路径负例） |
| 3 | `If of {cond; then; else}` | ✅ expr.rs:92 | ✅ `vm::tests::run_if` / `vm_tests::truthy_requires_bool` / `compile::tests::if_backpatch_complete` |
| 4 | `VarRef of string` | ✅ expr.rs:99（Symbol 句柄） | ✅ `negative_semantics_tests::e3_unbound_all_contexts`（10 case）+ `free_variables_*`（core 单测） |
| 5 | `Literal of literal_value` | ✅ expr.rs:101 | ✅ `vm::tests::run_literal_program` / `expr::tests::literal_render_pair_list` |
| 6 | `SetBang of {name; value}` | ✅ expr.rs:103 | ✅ `vm_tests::set_unbound_errors_on_vm` / `expander_tests::letrec_derives_with_setbang` / `vm_tests::closures_share_mutable_captures` |
| 7 | `Define of {name; value}` | ✅ expr.rs:109（顶层原语；体内改写见 #21） | ✅ `vm_tests::define_returns_value_dual_path` + `duplicate_define_errors_dual_path`（T1 修复面双锚）+ `e6_duplicate_define_matrix`（9 case） |
| 8 | `Begin of core_expr list` | ✅ expr.rs:115 | ✅ `vm::tests::run_begin_sequence` / `gate_g4_vm_executes_opcode_groups`（begin 序列） |
| 9 | `Module of {name; imports; exports; body}`（import/export = string 裁定） | ✅ expr.rs:117（`imports/exports: Vec<Symbol>`） | ✅ `expander_tests::module_form_snapshot` / `module_phase_lifecycle` / `negative_expander_tests::module_misuse`（4 case）+ `import_export_misuse`（4 case） |
| 10 | `literal_value` 六变体（Int/Float/Str/Bool/Nil/Pair） | ✅ expr.rs:25-32（`Pair(Rc, Rc)`） | ✅ `expr::tests::literal_render_pair_list` / `pipeline_tests::strings_and_floats` |
| 11 | `stx_obj` 四字段（expr/span/scopes/phase） | ✅ kerf-syntax/src/stx.rs（datum+span+scopes+phase） | ✅ `stx::tests::macro_expansion_bumps_phase_and_scopes` / `scope::tests::hygiene_invariant_two_scope_sets_never_merge_to_single` |
| 12 | 糖推导：`let` → `((lambda (x) body) e)` | ✅ kerf-expander/src/expander.rs（desugar_let） | ✅ `expander_tests::let_derives_to_lambda_app` + 单元 `expand_let_derives_to_lambda_app` |
| 13 | 糖推导：`let*`（嵌套 let 链） | ✅ expander.rs:799-807（desugar_let_star） | ⚠️ 负向 ✅：`negative_expander_tests::letstar_misuse`（4 case）——**正向推导断言缺位**（§4.2 差距 #1） |
| 14 | 糖推导：`letrec` → nil 预绑定 + set! | ✅ expander.rs（desugar_letrec + 提升） | ✅ `expander_tests::letrec_derives_with_setbang` / `inner_define_hoisted_to_letrec` |
| 15 | 糖推导：`cond`/`else` → 嵌套 if | ✅ expander.rs（desugar_cond） | ✅ `expander_tests::cond_derives_to_nested_if` + 单元 `expand_cond_chain` / `negative_expander_tests::cond_misuse`（4）+ `else_misuse`（1） |
| 16 | 糖推导：`and` → if 短路 | ✅ expander.rs（desugar_and） | ✅ `expander_tests::and_or_derive` + 单元 `expand_and_or` |
| 17 | 糖推导：`or` | ✅ expander.rs（desugar_or） | ✅ 同上（`and_or_derive` 一锚双形式） |
| 18 | 糖推导：`when` → `(if c (begin body) nil)` | ✅ expander.rs:962-992（desugar_when） | ⚠️ 负向 ✅：`derived_form_misuse`（when/unless/while 零参数 3 case）——**正向推导断言缺位**（§4.2 差距 #1） |
| 19 | 糖推导：`unless` | ✅ expander.rs:990-1022（desugar_unless） | ⚠️ 同 #18 |
| 20 | 糖推导：`while` → letrec 循环 | ✅ expander.rs（desugar_while，fresh loop 符号） | ✅ `expander_tests::while_derives_with_fresh_loop` + 单元 `expand_while_uses_fresh_loop` |
| 21 | `define` 函数简写 + 体内部提升（§19.2 陷阱 3） | ✅ expander.rs（parse_params + 提升） | ✅ `expander_tests::define_function_sugar` / `inner_define_hoisted_to_letrec` / `inner_define_after_expression_is_error` / `vm_tests::lambda_duplicate_params_rejected_at_expand`（重名检查双路径上游） |

### 2.2 02-syntax-model（Token/Span/诊断，17 点）

| # | 设计点 | 实现状态 | 测试状态 |
|---|--------|----------|----------|
| 1 | Token 结构（kind/span/scopes 三字段） | ✅ kerf-reader/src/token.rs:17-23 | ✅ `lexer::tests::spans_are_exact` / `parser` 全套消费 |
| 2 | `TokenKind` enum **12 变体** | ✅ token.rs:39-66（Int/Float/Str/Bool/Nil 字面量 5 + Identifier + QuoteShorthand + Keyword + Operator + Delimiter + MacroInvocation + Eof） | ✅ `lexer::tests::booleans_and_nil` / `float_int_and_scientific` / `quote_shorthand_token` / `keyword_vs_identifier` / `operators_lexed_from_symbols`（逐变体触达） |
| 3 | 叶级展开 **44 种**（5+1+1+21+10+4+1+1） | ✅ Keyword 21（kerf-syntax/src/symbol.rs:24-46 逐项核对）+ Operator 10（token.rs:70-81）+ Delimiter 4（token.rs:85-90） | ✅ `symbol::tests::keyword_roundtrip` / `keywords_prefetched`（21 关键字往返）+ `lexer::tests::operators_lexed_from_symbols` |
| 4 | Reader 契约：**无损性**（Span 并集精确覆盖输入） | ✅ kerf-reader/src/lexer.rs（逐 token 字节推进） | ✅ `lexer::tests::tokens_cover_input_losslessly` + 集成 `reader_tests::tokens_cover_input_losslessly` |
| 5 | Reader 契约：**位置完备**（错误必带 Span） | ✅ ReadError { message, span }（token.rs:118-121） | ✅ `reader_tests::all_reader_errors_carry_spans` + `lexer::tests::illegal_char_has_span` / `parser::tests::unclosed_paren_error_has_span` |
| 6 | 词法层零语义（关键字纯查表） | ✅ `Keyword::from_name` 纯查表（symbol.rs:77-101） | ✅ `parser::tests::keyword_as_symbol_datum`（关键字作 datum 不触发语义） |
| 7 | Span 四字段（file_id/start/end/expansion_id） | ✅ kerf-span/src/span.rs:26-35 | ✅ `span::tests::span_contains_and_len` / `span_merge_union` / `span_expansion_bump` / `span_merge_different_files_falls_back` |
| 8 | 行/列派生渲染（字节偏移主键，UTF-8） | ✅ kerf-span/src/source_map.rs | ✅ `source_map::tests::line_col_basic` / `line_col_utf8` / `render_location_and_excerpt` / `unknown_file_is_explicit` |
| 9 | Diagnostic 结构（severity/code/message/primary_span/children/suggestions） | ✅ kerf-span/src/diagnostic.rs | ✅ `diagnostic::tests::diagnostic_render_full_shape` / `code_render` / `severity_labels` |
| 10 | 错误是数据（渲染形状 error[E0001] + `-->` + 源摘录） | ✅ Diagnostic::render | ✅ `negative_reader_tests::reader_error_rendering_shape`（4 case）+ `reader_error_code_is_e0001`（DiagnosticCode 结构化断言） |
| 11 | NFC 归一化一次且仅一次（保守子集） | ✅ kerf-syntax/src/symbol.rs（intern 时归一化 + Latin-1 组合映射） | ✅ `symbol::tests::nfc_normalization_one_time` / `intern_is_idempotent` |
| 12 | `'x` 简写 → `(quote x)` | ✅ parser.rs（QuoteShorthand 消费） | ✅ `parser::tests::quote_shorthand_expands` + 集成 `reader_tests::quote_shorthand_snapshot` + `negative_reader_tests::quote_shorthand_at_eof`（3 case） |
| 13 | 拒绝贪心数字（`123abc` 报错） | ✅ lexer.rs（贪心数字检测） | ✅ `lexer::tests::greedy_number_rejected` + 集成 `reader_tests::greedy_number_rejected_snapshot` + `negative_reader_tests::greedy_number_rejections`（5 case） |
| 14 | 嵌套块注释 | ✅ lexer.rs（深度计数） | ✅ `lexer::tests::block_comment_nesting` / `unclosed_block_comment_is_error` + `negative_reader_tests::unclosed_block_comments`（3 case 含嵌套） |
| 15 | 字符串跨行（行号在字面量内部维护） | ✅ lexer.rs | ✅ `lexer::tests::string_escapes_and_newline` + `negative_reader_tests::unclosed_string_literals`（4 case 含跨行后 EOF） |
| 16 | 括号三态（未闭合/多余闭合/类型交叉） | ✅ parser.rs（栈式平衡） | ✅ `negative_reader_tests::unclosed_delimiters`（7）/ `stray_closing_delimiters`（5）/ `mismatched_delimiters`（4）+ `parser::tests::stray_close_error` / `mismatched_brackets_error` |
| 17 | 非法字符/转义/溢出负族 | ✅ lexer.rs | ✅ `negative_reader_tests::illegal_characters`（14）/ `illegal_escape_sequences`（3）/ `integer_literal_overflow`（2）/ `bad_exponent_literals`（4） |

### 2.3 03-macro-system（相位分离 + 卫生宏，14 点）

| # | 设计点 | 实现状态 | 测试状态 |
|---|--------|----------|----------|
| 1 | 相位规则 1：Phase 1 只产生 Phase 0 代码 | ✅ expander.rs（transformer 产物再递归展开至核心形式） | ✅ `expander_tests::expansion_depth_limit_errors`（宏→宏→核心形式链）+ `macro_self_reference_preserved` |
| 2 | 相位规则 2：instantiate/visit 分离 | ✅ kerf-expander/src/phase.rs:119-134 | ✅ `phase::tests::declare_visit_instantiate_lifecycle`（未 visit 即 instantiate → Err）+ `negative_expander_tests::module_registry_phase_violations`（4 case） |
| 3 | 相位规则 3：传递依赖相位传播 | ✅ phase.rs:81-117（visit 先行递归导入） | ✅ `phase::tests::transitive_visit_order` / `visit_is_idempotent` |
| 4 | ModuleRegistry **七方法**（declare/visit/instantiate/find/entries/pending_instantiations + new） | ✅ phase.rs:56-153（逐方法对照 03 §1.1 契约） | ✅ `phase::tests::duplicate_declare_fails` / `declare_visit_instantiate_lifecycle` / `expander_tests::module_phase_lifecycle` |
| 5 | ModuleEntry 五字段（name/imports/exports/visited/instantiated） | ✅ phase.rs:37-48 | ✅ `phase::tests::*` 消费全字段（find→visited/instantiated 断言） |
| 6 | **循环依赖检测**（DFS 灰标记 → 结构化 Err；菱形合法） | ✅ phase.rs:99-107（灰标记重入 →「模块循环依赖：Symbol(N) → …」） | ✅ `phase::tests::circular_dependency_detected` / `self_import_cycle_detected` / `diamond_dependency_is_not_a_cycle` + 集成 `negative_expander_tests::module_cycle_dependency_errors`（子进程探针复现栈溢出→结构化 Err 修复面） |
| 7 | instantiate Stage 0 简化（不递归实例化依赖体） | ✅ phase.rs:120-134（标记完成态 + visit 先行校验）——03 §1.1 v5.2 已改述 | ✅ `phase::tests::declare_visit_instantiate_lifecycle`（单模块无可观测差异） |
| 8 | `MAX_EXPANSION_DEPTH = 128`（超限报错而非栈溢出） | ✅ expander.rs:46 + :197-202（深度检查） | ✅ 单元 `expansion_depth_limit` + 集成 `expander_tests::expansion_depth_limit_errors` + 审计集 ⑦ 类（深度超限 2 case） |
| 9 | TransformerKind（Rules/Builtin）+ Transformer.def_scopes | ✅ kerf-expander/src/macro_sys.rs:20-34 | ✅ `macro_sys::tests::syntax_rules_parse_and_simple_match` + `expander_tests::hygiene_renames_introduced_identifiers`（Builtin 路径）+ `ellipsis_macro_expands` |
| 10 | apply_named：产物携带「def_scopes ∪ use_site_scopes」并集 | ✅ macro_sys.rs:49-65（retag_scopes + union） | ✅ `scope::tests::union_and_subset` + `stx::tests::macro_expansion_bumps_phase_and_scopes` + `expander_tests::user_macro_with_hygiene`（单元同名） |
| 11 | 自引用豁免（模板中宏名不重命名） | ✅ macro_sys.rs:57（preserve(self_name)） | ✅ `expander_tests::macro_self_reference_preserved` |
| 12 | HygieneCtx（begin_instantiation/preserve/fresh_symbol/renames + 一致性不变式） | ✅ macro_sys.rs:72-110 | ✅ `macro_sys::tests::hygiene_renames_introduced_identifiers`（同一标识符→同一重命名符号断言） |
| 13 | syntax-rules 文法（单层省略号骨架：模式/字面量/模板/维度匹配） | ✅ macro_sys.rs（SyntaxRules::apply）——**完整文法四子项（点对尾/尾省略号+固定尾/`(… tpl)` 转义/嵌套省略号）未实现，TD-005 Stage 2，03 §2.3 v5.2 注记** | ✅ `macro_sys::tests::ellipsis_matches_zero_or_more` + `negative_expander_tests::syntax_rules_misuse`（2 case）/ `macro_expansion_failures`（6 case 含模式不匹配、维度错误） |
| 14 | 卫生三重保证（引入隔离/自由穿透/自引用豁免） | ✅ expander.rs + macro_sys.rs（$hyg$ 重命名 + driver 回退剥离） | ✅ `expander_tests::hygiene_renames_introduced_identifiers` / `user_macro_programs`（宏内外同名不串扰集成）+ `negative_semantics_tests::t1_regression_hygiene_fallback_dual_path`（3 case，eval 侧接线修复面）+ `e3_macro_introduced_unbound` |

### 2.4 04-bytecode-vm（操作码 40 项八组 + 编译/VM 不变式，20 点）

| # | 设计点 | 实现状态 | 测试状态 |
|---|--------|----------|----------|
| 1 | 栈操作组（7）：PUSH_CONST/PUSH_NIL/PUSH_TRUE/PUSH_FALSE/POP/DUP/SWAP | ✅ kerf-compiler/src/opcode.rs:21-33 + vm.rs 逐指令 | ✅ `vm::tests::isa_stack_ops` + `gate_g4_vm_executes_opcode_groups`（DUP 经 set! 触达） |
| 2 | 变量访问组（7）：LOAD/STORE_LOCAL、LOAD/STORE_GLOBAL、**DEFINE_GLOBAL**、LOAD/STORE_CAPTURED | ✅ opcode.rs:37-51（DefineGlobal=第 40 号冻结契约） | ✅ `gate_g4`（local/captured/global 触达）+ `define_emits_global_store`（编译单测）+ `vm_tests::define_returns_value_dual_path` / `set_unbound_errors_on_vm`（E3/E6 语义）+ `negative_vm_tests` 元数组 |
| 3 | 控制流组（2）：JUMP/JUMP_IF_FALSE | ✅ opcode.rs:55-58 | ✅ `compile::tests::if_backpatch_complete` + `vm::tests::run_if` + `gate_g4` |
| 4 | 函数操作组（3）：CLOSURE（current 栈协议）/CALL/RET | ✅ opcode.rs:62-69 + vm.rs CALL 弹序（先参后 fn） | ✅ `compile::tests::lambda_closure_conversion` / `nested_lambda_captures` + `vm::tests::define_and_call_closure` + `gate_g4`（make-adder 闭包） |
| 5 | 算术与比较组（12）：ADD…NOT | ✅ opcode.rs:72-85 | ✅ `vm::tests` 算术族 + `negative_vm_tests::arithmetic_non_numeric_operands`（60 case）/ `comparison_type_mismatch`（30 case）/ `division_and_modulo_by_zero`（3）等 |
| 6 | 数据构造组（3）：MAKE_PAIR/CAR/CDR | ✅ opcode.rs:88-93 | ✅ `vm::tests::isa_make_pair_car_cdr` + `vm_tests::pair_construction_and_traversal` + `negative_vm_tests::car_cdr_non_pair_operands`（14 case） |
| 7 | 谓词组（5）：IS_NULL/IS_PAIR/IS_INT/IS_BOOL/IS_PROCEDURE | ✅ opcode.rs:96-100 | ✅ `vm::tests::isa_predicates` + `gate_g4`（null?/pair? 触达）+ `negative_vm_tests::unary_predicate_arity`（10 case） |
| 8 | 终止组（1）：HALT | ✅ opcode.rs:104 | ✅ `compile::tests::main_proto_terminates_with_halt` |
| 9 | 总数 **40 三方冻结**（enum ↔ 守护测试 ↔ 04 文档） | ✅ opcode.rs enum 实数 40；模块头八组注释（opcode.rs:1-14 含 DEFINE_GLOBAL） | ✅ `opcode::tests::opcode_count_matches_spec`（opcode.rs:182-240：**逐项显式枚举 + assert 40**——r1 失效守护已重写） |
| 10 | VM 状态六件套（代码/数据栈/调用栈/全局/常量池/调试表） | ✅ kerf-vm/src/vm.rs（Vm 结构） | ✅ `gate_g6_span_propagation`（debug_spans 对账）+ `vm::tests::type_error_carries_span_from_debug_table` |
| 11 | 帧三扩展槽 ext1/2/3（格式冻结，Stage 0 可空） | ✅ vm.rs（FrameExt 构造） | ✅ `gate_g7_to_g10_reserved_interfaces`（ext1→EffectSystem 预留链）+ `four_reserved_interfaces_frozen` |
| 12 | CaptureSource（Local/SharedCell；共享单元格捕获协议） | ✅ kerf-compiler/src/compile.rs（捕获描述符） | ✅ `compile::tests::nested_closure_captures` + `compiler_tests::nested_closure_capture_descriptors` + `vm_tests::closures_share_mutable_captures`（可变捕获语义） |
| 13 | 栈平衡不变式（净变化 = 1） | ✅ compile.rs（每原语栈效应核算） | ✅ `compile::tests::compile_const_and_halt` + `compiler_tests::app_evaluation_order`（含栈序断言） |
| 14 | 回填完备（占位队列零残留） | ✅ compile.rs（CodeBuf 占位队列 + 完备断言） | ✅ `compiler_tests::backpatch_leaves_no_placeholders` + `compile::tests::if_backpatch_complete` |
| 15 | 常量池去重（float 位键） | ✅ kerf-compiler/src/bytecode.rs | ✅ `bytecode::tests::const_pool_dedups` + `compiler_tests::const_pool_dedup` |
| 16 | debug_info 全覆盖（每指令 (pc, Span)） | ✅ compile.rs（emit 自动写入） | ✅ `compiler_tests::debug_info_full_coverage` + `gate_g3_compiler_debug_info_complete` + `gate_g6` |
| 17 | App 求值顺序：**函数先**（06 §1.3/§2 A1，v5.2 修复后双侧一致） | ✅ compile.rs App 分支 fn 先入栈 + vm.rs CALL 弹参后弹 fn | ✅ `compile::tests::app_evaluates_fn_then_args_left_to_right`（字节码序）+ `vm_tests::app_evaluation_order_fn_first_dual_path`（`((undefined-a) undefined-b)` 双路径同报 fn 位错误）+ `negative_semantics_tests::t1_regression_app_evaluation_order`（5 case） |
| 18 | 迭代式主循环 + MAX_FRAMES=100_000（Rust 栈恒定） | ✅ vm.rs:130（MAX_FRAMES）+ 主循环帧栈 | ✅ `vm::tests::deep_recursion_bounded_by_frames_not_stack` + `vm_tests::deep_recursion_10k` + `negative_vm_tests::frame_limit_deep_recursion_vm_path`（结构化报错 1 case） |
| 19 | 堆栈追踪（最内 16 帧，VmError.trace 唯一生产者） | ✅ vm.rs:178（MAX_TRACE_FRAMES=16）+ run_program 错误路径 | ✅ `vm_tests::runtime_error_has_trace` + `negative_semantics_tests::message_shape_call_site_trace`（6 case note 行断言） |
| 20 | truthy 显式（仅 Bool 参与 JUMP_IF_FALSE） | ✅ vm.rs（JumpIfFalse 类型检查） | ✅ `vm::tests::truthy_only_bool` + `vm_tests::truthy_requires_bool` + `negative_vm_tests::if_condition_requires_bool`（6 case）+ `negative_semantics_tests::e1_*`（10 case） |

### 2.5 05-runtime（I/O + GC，13 点）

| # | 设计点 | 实现状态 | 测试状态 |
|---|--------|----------|----------|
| 1 | I/O 双通道（write_line_stdout/read_line_stdin，EOF→None） | ✅ kerf-runtime/src/io.rs:26-41（逐字对照 05 §1 契约） | ✅ `io::tests::write_line_ok` / `runtime_error_shape` + `pipeline_tests::builtin_library_registered` + `examples/usage/io.krf`（print/read-line/str-append 回环，经 examples 重组） |
| 2 | RuntimeError 最小形态（仅 message，Layer 0 无依赖裁定） | ✅ io.rs:12-23 | ✅ `io::tests::runtime_error_shape` + E0004 包装路径（`vm_error_code_is_e0004`） |
| 3 | HeapObj **六变体**（Pair/Str/Int/Float/Bool/Nil） | ✅ kerf-runtime/src/heap.rs:16-29 | ✅ `heap::tests::collect_frees_unreachable` 等（全变体分配触达）+ `pipeline_tests::pair_construction` |
| 4 | GcRef = u32 槽句柄（句柄稳定性） | ✅ heap.rs:12 | ✅ `heap::tests::alloc_reuses_free_slots`（复用即句柄稳定证据） |
| 5 | Slot（obj + marked） | ✅ heap.rs:61-64 | ✅ `heap::tests::unrooted_cycle_is_collected`（标记位消费） |
| 6 | slot-Vec + 空闲表分配（复用优先/追加新槽） | ✅ heap.rs:114-128 | ✅ `heap::tests::alloc_reuses_free_slots` + `reachability_partition` |
| 7 | 类型化分配**七入口**（六类型化 + alloc_boxed） | ✅ heap.rs:116-131（+ alloc_boxed 分派） | ✅ `heap::tests` 全族 + `gc_tests::million_temp_objects_heap_stays_bounded` |
| 8 | 触发判定 `allocs_since_gc > gc_threshold`（严格大于；默认 1024） | ✅ heap.rs（with_threshold(1024)） | ✅ `gc_tests::million_temp_objects_heap_stays_bounded`（分配计数驱动触发 + collections > 0 断言，见 gate_g5） |
| 9 | GC 冷却退避（VM 侧：GC_POLL_INTERVAL=256 / GC_COOLDOWN_MULTIPLIER=64 / 收益<25% / MAX 1024） | ✅ vm.rs:133-137/586-594（vm.rs 实测四常量，归属 VM 侧与 05 v5.2 注记一致） | ✅ `vm::tests::gc_safepoint_collects_garbage` + `vm_tests::deep_recursion_10k`（O(n²) 消退：122s→3.4s 基线）+ `gc_tests::live_data_survives_collections` |
| 10 | RootSet + mark_sweep_cycle（标记→清除→复位） | ✅ kerf-runtime/src/gc.rs:50-52（→ heap.collect 三阶段） | ✅ `gc_tests::live_data_survives_collections` / `reachability_partition` + `heap::tests::collect_keeps_cycle_from_roots`（环） |
| 11 | 根集**五来源**（VM 栈/帧局部/帧捕获/全局/foreign） | ✅ vm.rs:650-665（collect_roots：stack+globals+frames.locals+frames.captures；foreign_roots 由 Heap 持有并入标记栈） | ✅ `gc_tests::closure_captured_data_survives_gc`（帧捕获槽——第五来源回归）+ `foreign_refs_survive_cycle` + `gate_g5_gc_collects_unreferenced` |
| 12 | register_foreign_ref 非 no-op（外部根保存） | ✅ heap.rs（foreign_roots 并入 mark_sweep_cycle 起点） | ✅ `gc_tests::foreign_refs_survive_cycle` + `heap::tests::foreign_roots_survive` |
| 13 | GcStats 观测（性能数据一等公民） | ✅ heap.rs（total_allocs/collections/total_freed） | ✅ `gc_tests::gc_stats_observable` + `gate_g5`（collections > 0 断言） |

### 2.6 06-operational-semantics（R1-R9/E0-E8/T1，24 点）

| # | 设计点（06 §2/§3/§5） | 实现状态 | 测试状态 |
|---|------------------------|----------|----------|
| 1 | R1 Lambda（闭包构造，捕获定义环境） | ✅ eval.rs（闭包值构造）+ compile.rs（CLOSURE） | ✅ `vm_tests::closures_share_mutable_captures` + `higher_order_functions`（双路径互查内置） |
| 2 | R2 App（A1 函数先 + A2 左到右 + A3 βv + A4 δ） | ✅ 双路径（§2.4 #17 证据） | ✅ `t1_regression_app_evaluation_order`（5 case）+ `dual_execution_paths_cross_validate` + `e2_arity_*`（A3 卫式 10 case） |
| 3 | R3 If（I1/I2 分支 + I3 非布尔 Err） | ✅ eval.rs/vm.rs | ✅ `e1_truthy_requires_bool_all_types`（7 case）+ `e1_truthy_nested_positions`（3）+ `if_condition_requires_bool`（6） |
| 4 | R4 Literal（L4a 标量/L4b 点对堆构造/L4c quote 展开期归约） | ✅ expander（quote→Literal）+ eval_literal 堆构造 | ✅ `quote_list_becomes_pairs` + `reader_tests::quote_shorthand_snapshot`（'x → (quote x)）+ `quote_symbol_is_explicit_error`（TD-002 边界显式报错） |
| 5 | R5 SetBang（S1 词法链定位/S2 右部先求值/未绑定 Err） | ✅ eval.rs Env::set + vm.rs StoreGlobal 严格化 | ✅ `set_unbound_errors_on_vm` + `vm_tests::truthy*`（S2 序）+ `t1_regression_global_storage_semantics`（4 case） |
| 6 | R6 Define（D1 右部先求值+顶层新增；返回值 = v） | ✅ eval.rs Env::define（bool）+ DefineGlobal 指令 | ✅ `define_returns_value_dual_path` + `e6_duplicate_define_dual_path`（2 case） |
| 7 | R7 Begin（B1 空序列 nil/B2 值丢弃/B3 源序） | ✅ 双路径 | ✅ `vm::tests::run_begin_sequence` + `expander_tests`（begin 体铺平同类路径审查簇） |
| 8 | R8 VarRef（V1 查找/V2 未绑定 Err） | ✅ 双路径 | ✅ `e3_unbound_all_contexts`（10 case）+ `unbound_variable_vm_message`（消息分裂存档，Err 事实双断言） |
| 9 | R9 Module（M1 单模块全局环境裁定） | ✅ driver（模块体直入全局环境） | ✅ `expander_tests::module_phase_lifecycle` / `module_form_snapshot` + `examples/usage/macros.krf`（顶层 define 序列） |
| 10 | E0 错误吸收态（无后续状态） | ✅ 全管线 Result 短路（TD-013：单错误短路形态） | ✅ `error_recovery_single_error_semantics`（4 case 锚定吸收语义事实） |
| 11 | E1 TypeMismatch | ✅ | ✅ `e1_*` 三函数（10 case）+ `not_requires_bool`（7 case） |
| 12 | E2 ArityMismatch（硬错误） | ✅ | ✅ `e2_arity_mismatch_matrix`（6）+ `e2_arity_dual_path_errors`（4）+ `negative_vm_tests` 元数族（~30 case） |
| 13 | E3 UnboundVariable | ✅ | ✅ `e3_*` 三函数（13 case）+ `unbound_variable_vm_message` |
| 14 | E4 NotCallable | ✅ | ✅ `e4_not_callable_all_types`（7）+ `e4_not_callable_computed_callee`（3）+ `negative_vm_tests::not_callable_values`（7） |
| 15 | E5 BuiltinError（δ 失败） | ✅ builtins.rs 显式 Err | ✅ `e5_builtin_error_matrix`（5 case）+ `division_and_modulo_by_zero`（3）+ `integer_overflow_detected`（3） |
| 16 | E6 DuplicateDefine | ✅ DefineGlobal + Env::define（同层报错/跨层 shadowing 合法） | ✅ `e6_duplicate_define_matrix`（9 case：顶层/糖/内置影子/嵌套提前防御）+ `e6_duplicate_define_dual_path`（2） |
| 17 | E7 RuntimeError（I/O 通道失败） | ✅ io.rs Result 通道存在 | ⏸️ **DEFERRED**：公开 API 不可触发（read-line EOF → nil 为正常语义；write 失败需关闭 stdout——negative_semantics_tests.rs:12-14 头注存档，§9.4.3 规则 2） |
| 18 | E8 内部不变式破坏 | ✅ 编译器不变式防御（栈平衡断言等） | ⏸️ **DEFERRED**：非用户程序可触发（negative_semantics_tests.rs:15-18 头注存档；「栈下溢为 E8 内部不变式」pipeline-test-coverage.md §3 更正记录） |
| 19 | 值域九变体（含 unit——v5.2 补） | ✅ kerf-vm/src/value.rs:20-35（Unit/Nil/Bool/Int/Float/Str/Pair/Closure/Builtin 九变体） | ✅ `vm::tests` 全族 + `e4_not_callable_all_types`（七类型枚举触达）+ `not_requires_bool`（七类型） |
| 20 | 求值顺序契约（§1.3：函数先/参数左到右/Begin 源序） | ✅ 双路径一致（Task 16 修复） | ✅ 见 #2；`message_shape_*` 双路径口径 |
| 21 | L-GC（GC 不可观测性引理） | ✅ 根集完备（§2.5 #11）+ 冷却仅影响时机 | ✅ `gc_tests` 全 6 函数 + 双路径互查（含堆效应程序，164 集成函数内置）+ `gate_g5` |
| 22 | T1 编译正确性定理（必要条件验证） | ✅ compile+VM 双路径 | ✅ `dual_execution_paths_cross_validate` + `negative_vm_tests::dual_path_agrees_on_error_programs`（12 case 错误程序 Err 事实一致） |
| 23 | T1 引理 L1-L4（指令组局部保持/顺序一致/捕获等价/值结构等价） | ✅ L3 = 共享单元格捕获（vm_tests 锚） | ✅ `closures_share_mutable_captures`（L3）+ `compiler_tests::deterministic_compilation_two_passes_identical`（L4 值等价）+ `opcode_count_matches_spec`（L1 检查点） |
| 24 | 错误路径互查口径（Err 事实+Stage 一致；消息分裂存档） | ✅ 双路径（消息文本分裂面已注记：未绑定/if 非布尔，TD-014 关联） | ✅ `expect_dual_err` 断言族 + 头注存档（negative_semantics_tests.rs:25-28） |

### 2.7 07-bootstrap / 13-capability-matrix（四项接口预留，6 点）

| # | 设计点 | 实现状态 | 测试状态 |
|---|--------|----------|----------|
| 1 | EffectSystem（P3：EffectFamily/Effect/EffectSystem 三 trait 形状） | ✅ kerf-driver/src/reserved.rs:26-55（与 13 §3.1.1 v5.2 回填签名一致） | ✅ `reserved::tests::reserved_signatures_are_frozen`（ProbeEffects 编译通过 = 冻结）+ `gate_g7_to_g10`（G7） |
| 2 | MultiStage（P3：quote/splice/run 三方法） | ✅ reserved.rs:69-81 | ✅ 同上（ProbeMultiStage）+ `code_value::tests::compose_with_reserved_unsupported`（G8 组合预留） |
| 3 | CapabilityIO（P2：完整 4 条行为规格） | ✅ reserved.rs:116-122（签名）+ doc 注释 4 条规格 | ✅ `reserved::tests::reserved_signatures_are_frozen` + `gate_g7_to_g10`（G9） |
| 4 | Read/WriteCapability 令牌不可伪造 | ✅ reserved.rs:88-95（私有构造） | ✅ `reserved::tests::capability_tokens_are_unforgeable`（编译期不可实例化证明） |
| 5 | CompilationCache（P2：get/store/invalidate 三方法规格） | ✅ reserved.rs:158-167 | ✅ 同 #1（ProbeCache）+ `gate_g7_to_g10`（G10） |
| 6 | CacheKey 内容寻址（source_hash + config_fingerprint） | ✅ reserved.rs:130-135 | ✅ `reserved::tests::cache_key_is_content_addressed`（等键等值/异键异值） |

### 2.8 09-stdlib（24 内置分七组 + 零内置分层，8 点 = 24 项）

| # | 设计点（09 §2 v5.2 清单） | 实现状态 | 测试状态 |
|---|--------------------------|----------|----------|
| 1 | 算术 5（+/-/*/mod） | ✅ builtins.rs:21-25（Fold 驱动；数值塔+溢出检查+mod 拒浮点） | ✅ `pipeline_tests::builtin_library_registered`（24 全注册断言）+ `negative_vm_tests::arithmetic_*`（60+20+2+5+3+3 case 系统表）+ `vm_tests::fib_25_correct_on_vm` |
| 2 | 比较 5（=/</>/<=/>=，链式） | ✅ builtins.rs:26-30 | ✅ `negative_vm_tests::comparison_*`（4+30+5 case）+ `gate_g4`（= 触达） |
| 3 | 序对 4（cons/car/cdr/list） | ✅ builtins.rs:32-83（list 变长右折叠） | ✅ `vm_tests::pair_construction_and_traversal` + `negative_vm_tests::cons_arity`（3）/ `car_cdr_*`（4+14）+ `gate_g4` |
| 4 | 谓词 6（null?/pair?/int?/bool?/procedure?/eq?） | ✅ builtins.rs:85-128（eq? 恰 2 参） | ✅ `vm::tests::isa_predicates` + `negative_vm_tests::unary_predicate_arity`（10）/ `eq_arity`（3）+ `gate_g4`（null?/pair?/eq?） |
| 5 | not 1（仅 Bool） | ✅ builtins.rs:130-135 | ✅ `negative_vm_tests::not_requires_bool`（7 case）+ `not_and_print_arity`（4） |
| 6 | I/O 2（print/read-line，语言层连字符命名） | ✅ builtins.rs:143-160（→通道层 io.rs） | ✅ `pipeline_tests::strings_and_floats`（print 触达）+ `examples/usage/io.krf`；**read-line 元数不校验为实测存档**（`read_line_arity_ignored`，FS-4，#[ignore] 文档化） |
| 7 | 串 1（str-append 恰 2 参） | ✅ builtins.rs:162-170 | ✅ `negative_vm_tests::str_append_requires_strings`（14 case）/ `str_append_arity`（2）+ `examples/usage/io.krf` 回环 |
| 8 | 语言核心零内置（driver 注册 δ 表） | ✅ builtins.rs register_globals（24 defs，实测计数） | ✅ `builtins::tests::globals_have_core_set` + `pipeline_tests::builtin_library_registered`（注册表对账）+ `builtins::tests::hygiene_fallback_*`（$hyg$ 剥离——注册名与卫生符号互操作） |

### 2.9 10-toolchain / 11-testing / 12-roadmap（工程面，10 点）

| # | 设计点 | 实现状态 | 测试状态 |
|---|--------|----------|----------|
| 1 | CLI 10 子命令（run/eval/check/tokens/stx/core/ir/bc/code/bench） | ✅ src/main.rs:23-33（tokens/stx 经 driver 转发——A4 修复，§14.7.2 B4 合规） | ✅ `driver::tests::compile_output_dumps`（各级 dump）+ `compiler_tests::disassembly_is_renderable`（bc）+ 本报告实测 bench（fib.krf 3 轮 87.8ms/轮，含启动） |
| 2 | 编译即 API（所有阶段可独立调用） | ✅ driver 公共函数（compile_source/run_source/eval_source/dump_*） | ✅ `pipeline_tests::eval_path_complete` / `driver_error_display_format` + 全部集成测试消费面 |
| 3 | IR dump + 阶段跟踪（四层调试之 1/2 层） | ✅ CLI ir/core/bc 子命令 + Stage 标注诊断 | ✅ `compiler_tests::ir_shares_literal_nodes` + `diagnostics_render_per_stage` + `pipeline_end_to_end_snapshot`；交互式调试器为设计既定推迟（10 §2，Stage 1+/2——ext3 槽已预留） |
| 4 | 性能基准框架（编译/执行/自举三类） | ✅ CLI bench（编译+执行口径） | ✅ 本报告实测：`bench examples/usage/fib.krf`（3 轮总 0.263s/每轮 87.820ms，与 status.md 84.4ms release×5 口径一致）；gc_stress 单轮 ~0.17s |
| 5 | 快照测试（≥20 Reader / ≥50 全管线） | ✅ 11 套件 164 集成函数 | ✅ `tokens_snapshot_fib` / `stx_snapshot_nested` / `pipeline_end_to_end_snapshot` 等（294 函数 ≥ 50 门槛） |
| 6 | 双路径互查（T1 验证载体，§2.4） | ✅ driver eval_source/run_source | ✅ `dual_execution_paths_cross_validate` + common::dual_path_agrees（164 集成函数内置断言）+ `eval_path_complete` |
| 7 | 阶段切换门槛（≥100 测试全通过，07 §3.3） | ✅ 294 函数 / 290 全绿 | ✅ 本报告实测 `cargo test --release --workspace`：**290 passed / 0 failed / 4 ignored**（matrix.md 对账一致） |
| 8 | 三层测试记录（Tier1/2/3，§9.5.1） | ✅ docs/tests/pipeline-test-coverage.md r3 | ✅ matrix.md 分套件表与 cargo test 实测逐行核对（本报告 §5） |
| 9 | 正负比例 ≥1:3（§9.4.3） | ✅ 四负测文件 483 case + 审计负向 32 | ✅ 本报告按文件内 case 注释复算：59+96+230+98=483 ✓；正负比 ≈1:3.2 |
| 10 | 门审计集 ≥30 case（§7.3.1） | ✅ examples/audit/stage0_gate_audit_r1.rs（41 case：A12/B12/C8/D6/P3） | ✅ 本报告实测复跑：**41/41 PASS，EXIT 0**，七类覆盖 1/1 2/5 3/1 4/4 5/16 6/1 7/2 |

---

## 3. 统计与 DEFERRED 校验（≤5% 门限）

| 度量 | 值 | 依据 |
|------|-----|------|
| Stage 0 范围设计点总数 | **133** | §2 各表行数（01:21 / 02:17 / 03:14 / 04:20 / 05:13 / 06:24 / 07+13:6 / 09:8 / 10+11+12:10） |
| 已实现 | **133**（100%） | 全部设计点在代码中落地（含显式报错形态的推迟子项） |
| 完整闭环行（✅/✅，正向或不可构造口径） | **128** | 133 − 3（正向缺位行）− 2（表内 ⏸️ 行 E7/E8） |
| DEFERRED（设计承诺面，附理由存档） | **5 项** | 2 项为表内 ⏸️ 行（06 #17 E7 / #18 E8）+ 3 项为断言/承诺范围存档（E0003 编译期码 / quote 符号·向量值 TD-002 / 多错误收集 TD-013——对应表行以显式报错或当前行为锚定为部分闭环） |
| **DEFERRED 比例** | **5/133 = 3.8% ≤ 5% ✅** | 门限满足（保守口径：按承诺面计 5，非仅 ⏸️ 行计 2） |
| 已实现但正向测试缺位 | **1 项**（3 行：let*/when/unless 糖，P3） | §4.2——负向已覆盖，正向推导断言缺位 |
| 三者不一致 | **1 项**（P3） | §4.4——01 §5 锚点表表述超前 |

### DEFERRED 清单（5 项，逐项理由与登记锚）

| # | 设计点 | DEFERRED 理由 | 登记锚 |
|---|--------|---------------|--------|
| D1 | E7（I/O 通道错误）直接断言 | 公开 API 不可触发：read-line EOF → nil 为正常语义；write 失败需关闭 stdout（CLI 场景不可构造）。通道层 `Result` 路径经 io.rs 契约与 E0004 包装间接覆盖 | negative_semantics_tests.rs:12-14 头注（§9.4.3 规则 2 存档） |
| D2 | E8（内部不变式）直接断言 | 非用户程序可触发（栈失衡/pc 越界属编译器质量边界——06 §3 定性为「编译器 P0 缺陷而非用户错误」） | negative_semantics_tests.rs:15-18 头注 + pipeline-test-coverage.md §3 栈下溢更正记录 |
| D3 | E0003（编译期内部防御）直接断言 | 编译错误全部为内部不变式防御（回填完备/栈平衡断言），非用户可构造 | negative_tests.md §3 + negative_semantics_tests.rs:17-18 |
| D4 | quote 符号/向量值类型（01 §2 L4c 值域完备性） | Stage 0 值模型最小化裁定；显式报错（报错>静默 §2.3-4）；TD-002 Stage 1 偿还 | TD-002（负测锚：`quote_symbol_is_explicit_error` + `vector_in_expression_position`——推迟面有边界负测） |
| D5 | 多错误收集/恢复展开（02 §5「单次运行报告多个错误」承诺） | 实现为单错误短路；当前行为已锚定；TD-013 P2 Stage 1（与效应处理时机同批裁定，避免两套恢复机制） | TD-013（锚：`error_recovery_single_error_semantics` / `error_recovery_no_panics_structured`，8 case 存档） |

> **不计入分母的「设计已裁定推迟项」**（设计文档处理程度头/推迟项字段显式声明，非 Stage 0 承诺缺口）：syntax-parse 级宏组合与 syntax-rules 完整文法四子项（TD-005，03 头声明）、迭代式展开解除 128（TD-007）、scope-set 解析（TD-004——03 §4 为方向性骨架，冻结契约不含解析算法）、分代 GC（TD-008）、字符串全序（TD-011）、闭包装箱（TD-010）、LSP/交互式调试器（13 §3.2 完全推迟档）、TCO/JIT/能力模型 I/O 与缓存**实现**（P2/P3 预留即交付物）。这些项在 [技术债登记册](../../../develop/v0/tech-debt-register.md) 与 [隐藏问题评估](./hidden-problems-assessment.md)（本报告姊妹篇）中逐一评估复杂度增长。

---

## 4. 差距清单（§14.6.1.3 四问）

### 4.1 设计要求但未实现（B1，§14.8.1）

**零项**。deep-review R1 的 26 项偏差经 v5.2 回写 + Task 16 修复后全部闭环（B1×2 的 GC 口径/宏组合项均为设计自身裁定为推迟项，非「要求但未实现」）。无新的 B1 发现。

### 4.2 已实现但未测试（实现→测试）

| # | 项 | 严重度 | 现状 | 建议 |
|---|-----|--------|------|------|
| 1 | 糖推导**正向断言**缺位：`desugar_let_star`（expander.rs:799）/ `desugar_when`（:962）/ `desugar_unless`（:990） | P3 | 负向已覆盖（`letstar_misuse` 4 case / `derived_form_misuse` 3 case——零参数与结构错误路径）；**正向推导产物（展开形状）无断言**。全库检索（tests/ + examples/）确认无 `(let* ((…`/`(when …`/`(unless …` 正向程序 | 下一轮补 3 个正向锚点（各 1 case 即可，`(let* ((x 1) (y 2)) y)` → 展开形状断言；`when/unless` → if 推导断言），或改 01 §5 锚点表为「6/9 正向 + 3/9 负向」如实表述 |

### 4.3 设计要求但未测试（测试→设计）

- 除 §4.2 #1 外**零项**：01 §5 声称的「九种糖展开正确」中 let*/when/unless 三种缺正向锚（归入 §4.2 同一差距）；02/03/04/05/06/09 的测试锚点表所列锚点**全部验证存在**（§5）。
- E7/E8/E0003 归入 §3 DEFERRED（不可构造口径），不计为本节缺口。

### 4.4 三者不一致清单

| # | 不一致 | 等级 | 证据 | 处置 |
|---|--------|------|------|------|
| 1 | 01 §5 测试锚点表声称「let/let\*/letrec/cond/and/or/when/unless/while **九种糖的展开正确**」——实测正向锚点 6/9（let/letrec/cond/and/or/while + define 糖 + 提升），let*/when/unless 仅负向 | P3 | §4.2 证据；01 §5 表 vs expander_tests.rs 函数清单 | 二选一：补 3 正向锚（推荐）或 v5.3 修订 01 §5 表述 |

> r1 审查的四类三方漂移（操作码 39/40/41、内置 20/24、App 顺序、Token 41 不可验证）**已全部清零**——本报告以 `opcode_count_matches_spec`（断言 40 逐项枚举）、builtins.rs 24 defs 实数、双路径 App 锚点、TokenKind 12 变体实数为证。

---

## 5. 抽验证据记录（锚点真实性核验，73 处全命中）

方法：对设计文档/矩阵文档引用的全部测试锚点名执行 `rg -n 'fn <name>'`（crates/ + tests/ + examples/），并实际运行测试套件复验计数。

**运行实测（本报告当轮）**：
- `cargo test --release --workspace` → **290 passed / 0 failed / 4 ignored**（294 函数；分套件：单元 130 = span 11/syntax 11/core 10/reader 23/expander 26/compiler 12/runtime 9/vm 14/driver 14；集成 164 = plan 10 套件 156 + gate 8 + negative 忽略 4）
- `cargo run --release --example stage0_gate_audit_r1` → **total 41 / PASS 41 / FAIL 0 / EXIT 0**；七类覆盖 1=1 2=5 3=1 4=4 5=16 6=1 7=2
- `cargo run --release -- bench examples/usage/fib.krf 3` → 75025（fib(25) 正确）/ 0.263s 总 / 87.820ms 每轮
- 负向 case 复算（按文件内注释逐函数累加）：reader 4+3+5+14+3+5+7+4+4+2+3+4+1 = **59** ✓；expander 11+8+3+4+7+4+4+6+6+5+4+4+1+3+8+2+1+1+6+4+1+3 = **96** ✓；vm 60+20+2+5+3+3+4+30+5+3+4+14+10+3+7+4+14+2+7+6+5+2+3+1+12+1 = **230** ✓；semantics 7+3+6+4+10+2+1+7+3+5+9+2+8+6+4+5+3+5+4+4 = **98** ✓——与 matrix.md 483 对账一致

**锚点核验清单（代表性 73 处，全部命中——file:line）**：

| 类别 | 锚点 → 位置 |
|------|------------|
| 操作码 | `opcode_count_matches_spec`→opcode.rs:182；`render_shapes`→opcode.rs:243 |
| 相位/循环依赖 | `declare_visit_instantiate_lifecycle`→phase.rs:161；`duplicate_declare_fails`→:172；`circular_dependency_detected`→:179；`self_import_cycle_detected`→:194；`diamond_dependency_is_not_a_cycle`→:203；`transitive_visit_order`→:219；`visit_is_idempotent`→:230 |
| 预留冻结 | `reserved_signatures_are_frozen`→reserved.rs:218；`cache_key_is_content_addressed`→:260；`capability_tokens_are_unforgeable`→:278 |
| 编译 | `app_evaluates_fn_then_args_left_to_right`→compile.rs:638；`backpatch_leaves_no_placeholders`→compiler_tests.rs:19；`const_pool_dedup`→:37；`debug_info_full_coverage`→:50；`nested_closure_capture_descriptors`→:64；`deterministic_compilation_two_passes_identical`→:107；`define_emits_global_store`→:127 |
| VM/双路径 | `nine_primitives_semantics`→vm_tests.rs:13；`fib_25_correct_on_vm`→:36；`dual_execution_paths_cross_validate`→:115；`define_returns_value_dual_path`→:145；`duplicate_define_errors_dual_path`→:154；`set_unbound_errors_on_vm`→:166；`lambda_duplicate_params_rejected_at_expand`→:177；`app_evaluation_order_fn_first_dual_path`→:187 |
| 语义负例 | `e1_truthy_requires_bool_all_types`→negative_semantics_tests.rs:75；`e1_truthy_nested_positions`→:93；`e2_arity_mismatch_matrix`→:105；`e2_arity_dual_path_errors`→:125；`e3_unbound_all_contexts`→:138；`e3_macro_introduced_unbound`→:154；`e3_macro_error_carries_expansion_mark`→:163；`e4_not_callable_all_types`→:179；`e4_not_callable_computed_callee`→:196；`e5_builtin_error_matrix`→:208；`e6_duplicate_define_matrix`→:223；`e6_duplicate_define_dual_path`→:249；`message_shape_stage_codes_and_span`→:260；`message_shape_call_site_trace`→:291；`t1_regression_global_storage_semantics`→:356；`t1_regression_app_evaluation_order`→:368；`t1_regression_hygiene_fallback_dual_path`→:391；`error_recovery_single_error_semantics`→:437；`error_recovery_no_panics_structured`→:461 |
| 展开负例 | `empty_application_family`→negative_expander_tests.rs:62；`quote_misuse`→:197；`syntax_rules_misuse`→:360；`macro_expansion_failures`→:387；`module_registry_phase_violations`→:423；`module_cycle_dependency_errors`→:484 |
| Reader 负例 | `unclosed_string_literals`→negative_reader_tests.rs:56；`quote_shorthand_at_eof`→:130；`unclosed_block_comments`→:210；`reader_error_rendering_shape`→:224；`reader_error_code_is_e0001`→:252 |
| VM 负例 | `comparison_string_ordering_rejected`→negative_vm_tests.rs:139；`if_condition_requires_bool`→:348；`vm_error_code_is_e0004`→:399；`dual_path_agrees_on_error_programs`→:458；`run_error_yields_no_value`→:490 |
| 门审计 | `gate_g1_reader_parses_nine_primitives`→gate_review_r1.rs:27；`gate_g2`→:49；`gate_g3`→:64；`gate_g4`→:75；`gate_g5`→:97；`gate_g6`→:112；`gate_g7_to_g10_reserved_interfaces`→:123 |
| 管线 | `span_propagates_through_all_stages`→pipeline_tests.rs:23；`convergent_compilation_and_execution`→:75；`four_reserved_interfaces_frozen`→:53 |
| 审计集 | examples/audit/stage0_gate_audit_r1.rs:158（CASES 表）+ 实跑 41/41 |

**代码事实核验**（实现状态列证据抽样）：`MAX_EXPANSION_DEPTH: u32 = 128`→expander.rs:46；GC 四常量 256/64/MAX_FRAMES 100_000/MAX_TRACE_FRAMES 16→vm.rs:133/137/130/178；collect_roots 五来源（stack/globals/frames.locals/frames.captures + Heap.foreign_roots）→vm.rs:650-665；`heap.set_gc_enabled(false)`（TD-009 eval 侧）→driver.rs:275；`resolve_eval_hygiene_fallbacks`（Task 16 修复）→driver.rs:309；HeapObj 六变体→heap.rs:16-29；Value 九变体（含 Unit）→value.rs:20-35；Span 四字段→span.rs:26-35；Keyword 21→symbol.rs:24-46；TokenKind 12→token.rs:39-66；CLI 10 子命令经 driver 转发→main.rs:23-33 + driver.rs dump_tokens/dump_stx；LOC 10806/37 文件实测一致。

---

## 6. 结论与建议

1. **三者覆盖判定：达标（133 点 / 128 完整闭环 / 5 DEFERRED=3.8% / 1 P3 差距）**。§14.6.1.3 四问中 B1 为零、三者不一致仅 1 项 P3、已实现未正向测试仅 1 项 P3——均不构成门禁阻塞（P1/P2 清零维持）。
2. 建议下一轮（Gate R2 复审前）补齐 3 个糖推导正向锚点（§4.2），同步修订 01 §5 锚点表表述——预计 +3 测试函数，工作量 < 0.5 天。
3. E7/E8/E0003 的不可构造口径已按 §9.4.3 规则 2 存档，进入 Stage 1 后若 I/O 能力模型做实（E7 将变为可构造），需同步解除该存档。

## 7. 依赖

- 上游：[matrix.md](../../../tests/matrix.md)（分套件计数权威）、[pipeline-test-coverage.md](../../../tests/pipeline-test-coverage.md)（§14.6.1.1 完整性口径）、[deep-review-round1.md](./deep-review-round1.md)（26 项偏差与回写依据）
- 姊妹篇：[hidden-problems-assessment.md](./hidden-problems-assessment.md)（§14.6.1.4——DEFERRED 项的复杂度增长评估与本报告 §3 清单互为输入）
- 规范：[06-操作语义 §6](../../../lang-design/06-operational-semantics.md)（归约规则↔测试锚点映射表）；[11-测试 §3](../../../lang-design/11-testing.md)（验收汇总）

## 8. 本轮遵循原则

§8.4.5（先查文档+实测取证：全部锚点 Grep 核验 + 三次实跑）；§9.4（锚定原则：设计↔实现↔测试互锚，锚点禁止编造——73 处逐一定位）；§14.8.3（差距清单含「无偏差」记录：B1 零项明确声明）；§2.3-8（设计驱动测试）；§3.2（交付前实际运行验收——测试/审计集/基准三口径当轮复跑）。
