# Markdown Header Execution Tracking

## Overview
- Introduce a dedicated AST node (`Stmt::MarkdownHeaderComment`) to preserve markdown header comments during parsing.
- Lower the new AST node into runtime-visible watch notifications so `BamlRuntime::call_function` can surface header-crossing events to a `watch_handler`.
- Reuse the watch-channel infrastructure by creating a synthetic watched variable for headers while keeping the type checker unchanged; both the variable name and channel name remain `__baml_markdown_header`, while the variable’s value is derived from the header’s source span (`file-path:start:end`).

## Parsing Changes
- `engine/baml-lib/ast/src/ast/stmt.rs`
  - Add `MarkdownHeaderCommentStmt { header: Arc<Header>, span: Span }` and the `Stmt::MarkdownHeaderComment` enum variant (no separate id field required because the span encodes uniqueness).
  - Update `fmt::Display`, `assert_eq_up_to_span`, `identifier()`, `span()`, and re-export the new type from `ast.rs`.
- `engine/baml-lib/ast/src/parser/parse_expr.rs`
  - When scanning comment blocks (`//#`, `//##`, …), create `Stmt::MarkdownHeaderComment` alongside the existing header annotation flow.
  - Maintain the existing annotation handling so the same `Arc<Header>` feeds into statement annotations while also producing the explicit statement.
  - Ensure `bind_headers_to_statement` skips attaching annotations to `Stmt::MarkdownHeaderComment` itself.
- `engine/baml-lib/ast/src/parser/parse.rs`
  - Apply the same transformation at the top level so headers before functions become explicit statements.

## Lowering and IR Updates
- `engine/baml-compiler/src/hir/lowering.rs`
  - At the start of every function block, insert a synthetic `Statement::Let` named `__baml_markdown_header`, initialize it with the span key (`"<file>::<start>::<end>"`), and attach a `WatchSpec` whose `name` is also `__baml_markdown_header`.
  - Translate each `Stmt::MarkdownHeaderComment` into a `Statement::WatchNotify { variable: "__baml_markdown_header", span }`; the runtime reads the watched variable’s current value (the span key) when firing the notification.
  - Record header metadata (span key, title, level, span) in the surrounding HIR block/function so later passes can resolve names and levels from the span key.
- `engine/baml-compiler/src/thir.rs`
  - Propagate the injected let statement and metadata into THIR structures (blocks, functions) so typed lowering retains header information.

## Type Checking & Interpretation
- `engine/baml-compiler/src/thir/typecheck.rs`
  - No special-case error suppression needed because the synthetic `Let` introduces the watched variable ahead of time.
  - Verify the generated `WatchSpec` is well-typed and that each `WatchNotify` resolves to the declared header variable.
- `engine/baml-compiler/src/thir/interpret.rs`
  - When evaluating the injected let, register the watched variable as usual; its value (the span key) sits in the observable state.
  - On the accompanying `WatchNotify`, look up the stored header metadata and emit `WatchNotification::new_block(span_key, title, level, function_name)`, extending `WatchBamlValue::Block` as needed to carry metadata.
- `engine/baml-compiler/src/watch/watch_event.rs`
  - Expand `WatchBamlValue::Block` to include span key, title, level (and span if desired) so downstream runtimes receive rich context.

## Watch Channel Analysis & Codegen
- `engine/baml-compiler/src/watch.rs`
  - Consume the per-function header metadata collected during lowering and register a `ChannelType::MarkdownHeader` entry using the fixed channel name `__baml_markdown_header`. Use the span key to distinguish headers within tooling.
- `engine/generators/languages/{python,typescript}/src/watchers.rs`
  - Include markdown header channels when emitting watcher manifests, using the metadata (span key/title) to expose distinct headers in generated bindings.

## Runtime Integration
- `engine/baml-runtime/src/{async_vm_runtime,async_interpreter_runtime}.rs`
  - Ensure watch handlers forward the richer block notifications (span key/title/level) without loss, and treat the watched variable’s value as the lookup key for header metadata on the receiving side.
- Any CLI/UI surfaces that display watch events should be updated to recognize markdown header notifications if necessary.
