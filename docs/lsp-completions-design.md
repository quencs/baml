## BAML Language Server: Code Completions Design

### Goals
- Provide fast, context-aware completions for `.baml` files:
  - Top-level declarations: `function`, `class`, `enum`, `client`, `generator`, `retry_policy`, `template_string`, `type`
  - Attributes: `@alias`, `@description`, `@check`, `@assert`, `@stream.done`, `@stream.not_null`, `@stream.with_state`, block attributes `@@dynamic`, `@@alias`, `@@assert`
  - Prompt helpers inside template bodies: `_.role("system"|"user"|"assistant")`, `ctx.output_format`, `ctx.client`, `ctx.client.name`, `ctx.client.provider`
  - IR-derived symbols (runtime-aware): function names, class/enum/type alias names
- Respect trigger characters: '@', '.', '"'
- Avoid blocking the main loop; reuse existing runtime caching and session/project mechanisms

### Current State
- `engine/language_server/src/server/api/requests/completion.rs` returns `Ok(None)`.
- Server capabilities already advertise Completion with triggers `@`, `"`, `.`.
- Hover, go-to-definition, and diagnostics already use `Project::runtime()` and `get_word_at_position()` utilities.

### Approach
1. Parse context
   - Get the document and cursor position
   - Extract current line and token using `get_word_at_position`, `get_symbol_before_position`
   - Detect simple contexts:
     - Attribute context: prefix `@` or `@@` at current token
     - Dot context: `ctx.` or `ctx.client.` or `_.role(` prefix
     - String-start context: after `"` in `client` shorthand or enum values
2. Suggest sets
   - Attributes
     - Field attributes (single `@`): `alias`, `description`, `check`, `assert`, `stream.done`, `stream.not_null`, `stream.with_state`
     - Block attributes (double `@@`): `dynamic`, `alias`, `assert`
   - Prompt helpers
     - For `_.role(` propose `system`, `user`, `assistant` snippet variants
     - For `ctx.` propose `output_format`, `client`
     - For `ctx.client.` propose `name`, `provider`
   - Keywords/top-level declarations
     - `function`, `class`, `enum`, `client`, `generator`, `retry_policy`, `template_string`, `type`
   - Runtime IR symbols (uses cached runtime): function names, class names, enum names, type aliases
3. Build LSP items
   - Use `CompletionResponse::List(CompletionList { is_incomplete: false, items })`
   - Provide `kind`, `detail`, and `insertText` where appropriate; snippets for `_.role("${1:system}")` and `@alias("${1:name}")`
   - Optionally set `filterText` to support minimal prefix filtering

### File Changes
- `engine/language_server/src/server/api/requests/completion.rs`
  - Implement `SyncRequestHandler::run` using `Session::get_or_create_project`, `DocumentKey::from_url` and current file contents
  - Detect context and construct a list of `CompletionItem`
  - Query runtime via `project.lock().runtime()` for IR names
- `engine/language_server/src/baml_project/position_utils.rs`
  - Already contains `get_word_at_position` and helpers; reuse as-is

### Examples
- Attribute completions
  - Typing `@a` -> `@alias`, `@assert`, `@alias("...")` (snippet)
- Prompt helpers
  - Typing `{{ _.ro` -> `_.role("system")`, `_.role("user")`, `_.role("assistant")`
  - Typing `{{ ctx.` -> `output_format`, `client`
  - Typing `{{ ctx.client.` -> `name`, `provider`
- Top-level declarations
  - At file start: `function`, `class`, `enum`, `client`, `generator`, `retry_policy`
- IR symbols
  - In references: suggest available `FunctionName`, `ClassName`, `EnumName`, `TypeAliasName`

Code sample (shape only):
```rust
// completion.rs (excerpt)
let symbol_before = get_symbol_before_position(&doc.contents, &pos);
let word = get_word_at_position(&doc.contents, &pos);
let cleaned = trim_line(&word);
let mut items = Vec::new();
match () {
  _ if cleaned.starts_with("@@") || symbol_before == "@" && cleaned.starts_with("@") => {
    items.extend(block_or_field_attribute_items(cleaned));
  }
  _ if cleaned.ends_with("_.role(") || cleaned.contains("_.role(") => {
    items.extend(role_items());
  }
  _ if cleaned.ends_with("ctx.") || cleaned.contains("ctx.") => {
    items.extend(ctx_items(cleaned));
  }
  _ if is_top_level_context(&doc.contents, &pos) => {
    items.extend(top_level_keywords());
  }
  _ => {
    // IR-driven symbols
    if let Ok(rt) = guard.runtime() {
      items.extend(ir_symbol_items(rt));
    }
  }
}
Ok(Some(CompletionResponse::List(CompletionList { is_incomplete: false, items })))
```

### Testing Strategy
- Unit tests in `engine/language_server/src/tests.rs` using in-memory LSP harness:
  - Open a `.baml` doc and request completion at various contexts
  - Assert returned items include expected labels and kinds
- Run `cargo test --lib` at `engine/`

### Performance
- Reuse runtime caching via `BamlProject::runtime` (already hashed across files and flags)
- Avoid expensive work when no project can be resolved

### Future Enhancements
- Snippet completions for function templates and scaffolding
- Type-aware suggestions inside blocks (e.g., class fields and types)
- Completion resolve support for detailed docs
