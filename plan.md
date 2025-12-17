# LSP Migration Plan: Porting to baml_language Compiler

## Executive Summary

This document outlines the plan for migrating the BAML Language Server (`engine/language_server/`) from the current Pest-based compiler (`baml-runtime` + `internal-baml-*` crates) to the new Salsa-based `baml_language` compiler infrastructure.

## Current Architecture

### Current LSP Stack
```
engine/language_server/
├── BamlProject         # File management + hash-based runtime caching
├── Project             # Wraps BamlProject + current/last-successful runtime
├── BamlRuntimeExt      # Extension trait for symbol lookup on BamlRuntime
└── Request Handlers    # Hover, GoToDefinition, Diagnostics, etc.

Dependencies:
├── baml-runtime        # Runtime with Pest-based parser
├── internal-baml-core  # Validation and IR
├── internal-baml-ast   # Pest-based AST
└── internal-baml-diagnostics  # Error types
```

### Key Integration Points
1. **`BamlProject::runtime()`** - Creates/caches `BamlRuntime` from file contents
2. **`BamlRuntimeExt`** - Provides `search_for_symbol()`, `list_functions()`, etc.
3. **`runtime.diagnostics()`** - Returns parse/validation errors
4. **`runtime.ir`** - Provides `find_class()`, `find_enum()`, `find_function()`, etc.

## New Architecture: baml_language

### Compiler Pipeline
```
baml_language/crates/
├── baml_base      # Core types: FileId, Span, Name, SourceFile
├── baml_lexer     # Logos-based tokenization
├── baml_syntax    # Rowan syntax tree + AST nodes
├── baml_parser    # Lossless parsing with error recovery
├── baml_hir       # High-level IR: ItemTree, signatures, bodies
├── baml_thir      # Type inference and checking
├── baml_codegen   # Bytecode compilation
├── baml_workspace # Project/file management
├── baml_diagnostics # Error rendering
└── baml_db        # Root Salsa database
```

### Key APIs
```rust
// Database creation
let db = baml_db::RootDatabase::new();

// File management
let file = db.add_file("path/to/file.baml", source_text);
let project = db.set_project_root("path/to/root");
project.set_files(&db, vec![file1, file2]);

// Querying items
let items = baml_hir::file_items(&db, file);
let signature = baml_hir::function_signature(&db, func_loc);
let body = baml_hir::function_body(&db, func_loc);
let class_fields = baml_hir::class_fields(&db, class_loc);

// Diagnostics
let parse_errors = baml_parser::parse_errors(&db, file);
let name_errors = baml_hir::validate_duplicate_names(&db, project);
let type_result = baml_thir::infer_function(&db, sig, body, globals, class_fields);
```

## Migration Strategy

### Phase 1: Foundation Layer

#### 1.1 Create LSP Database Wrapper
Create a new `LspDatabase` struct that wraps `RootDatabase` and provides LSP-friendly APIs.

**File:** `engine/language_server/src/lsp_db/mod.rs`
```rust
pub struct LspDatabase {
    db: baml_db::RootDatabase,
    project: Option<baml_workspace::Project>,
    file_map: HashMap<PathBuf, baml_base::SourceFile>,
}

impl LspDatabase {
    pub fn new() -> Self;
    pub fn add_or_update_file(&mut self, path: &Path, content: &str) -> SourceFile;
    pub fn remove_file(&mut self, path: &Path);
    pub fn set_project_root(&mut self, path: &Path);
}
```

#### 1.2 Implement Symbol Location Mapping
Create utilities to convert between `baml_language` spans and LSP positions/ranges.

**File:** `engine/language_server/src/lsp_db/position.rs`
```rust
pub fn span_to_lsp_range(db: &dyn baml_hir::Db, span: &Span) -> lsp_types::Range;
pub fn lsp_position_to_offset(content: &str, pos: &Position) -> usize;
pub fn offset_to_lsp_position(content: &str, offset: usize) -> Position;
```

### Phase 2: Symbol Resolution

#### 2.1 Implement Symbol Lookup
Replace `BamlRuntimeExt::search_for_symbol()` with HIR-based lookup.

**File:** `engine/language_server/src/lsp_db/symbols.rs`
```rust
pub struct SymbolLocation {
    pub file: SourceFile,
    pub span: Span,
    pub kind: SymbolKind,
}

pub enum SymbolKind {
    Function, Class, Enum, TypeAlias, Client, Test
}

impl LspDatabase {
    /// Find a symbol by name across all files
    pub fn find_symbol(&self, name: &str) -> Option<SymbolLocation>;

    /// Find all locations where a symbol is defined (for multi-file classes)
    pub fn find_symbol_locations(&self, name: &str) -> Vec<SymbolLocation>;

    /// Get the symbol at a specific position
    pub fn symbol_at_position(&self, file: SourceFile, pos: Position) -> Option<SymbolInfo>;
}
```

#### 2.2 Implement Type-Aware Symbol Resolution
For hover and go-to-definition on field accesses, we need type information.

```rust
impl LspDatabase {
    /// Resolve a path expression to its target
    /// e.g., `user.name` -> field `name` on class `User`
    pub fn resolve_path_at_position(
        &self,
        file: SourceFile,
        pos: Position
    ) -> Option<ResolvedSymbol>;
}
```

### Phase 3: Diagnostics Integration

#### 3.1 Aggregate All Error Types
Collect errors from all compiler phases.

**File:** `engine/language_server/src/lsp_db/diagnostics.rs`
```rust
pub fn collect_diagnostics(db: &LspDatabase) -> Vec<LspDiagnostic> {
    let mut diagnostics = Vec::new();

    // Parse errors (per file)
    for file in db.files() {
        for error in baml_parser::parse_errors(&db.db, file) {
            diagnostics.push(convert_parse_error(error));
        }
    }

    // Name resolution errors (project-wide)
    if let Some(project) = db.project {
        for error in baml_hir::validate_duplicate_names(&db.db, project) {
            diagnostics.push(convert_name_error(error));
        }
    }

    // Type errors (per function)
    for func in db.all_functions() {
        let inference = infer_function_for_lsp(db, func);
        for error in inference.errors {
            diagnostics.push(convert_type_error(error));
        }
    }

    diagnostics
}
```

#### 3.2 Diagnostic Conversion
Map `baml_language` error types to LSP diagnostics.

```rust
fn convert_parse_error(error: ParseError) -> LspDiagnostic {
    LspDiagnostic {
        range: span_to_range(error.span),
        severity: DiagnosticSeverity::ERROR,
        message: error.message,
        source: "baml".to_string(),
    }
}
```

### Phase 4: Request Handler Migration

#### 4.1 Hover Handler
**Current:** Uses `BamlRuntimeExt::search_for_symbol()` to find definition text.

**New:** Use HIR to get item definition and signature.

```rust
// engine/language_server/src/server/api/requests/hover.rs
impl SyncRequestHandler for Hover {
    fn run(session: &mut Session, ...) -> Result<Option<lsp_types::Hover>> {
        let db = session.get_lsp_database(&path)?;

        // Find what's at the cursor position
        let symbol = db.symbol_at_position(file, position)?;

        // Generate hover content based on symbol type
        let content = match symbol.kind {
            SymbolKind::Function(func_loc) => {
                let sig = baml_hir::function_signature(&db, func_loc);
                format_function_signature(&sig)
            }
            SymbolKind::Class(class_loc) => {
                let fields = baml_hir::class_fields(&db, class_loc);
                format_class_definition(&class_loc, &fields)
            }
            // ... other cases
        };

        Ok(Some(Hover { contents: HoverContents::Markup(content), .. }))
    }
}
```

#### 4.2 Go-to-Definition Handler
**Current:** Uses `BamlRuntimeExt::search_for_symbol()`.

**New:** Use HIR item lookup with proper span information.

```rust
// engine/language_server/src/server/api/requests/go_to_definition.rs
impl SyncRequestHandler for GotoDefinition {
    fn run(session: &mut Session, ...) -> Result<Option<GotoDefinitionResponse>> {
        let db = session.get_lsp_database(&path)?;

        // Get word at position and find its definition
        let word = get_word_at_position(&doc.text, &position);
        let locations = db.find_symbol_locations(&word);

        match locations.len() {
            0 => Ok(None),
            1 => Ok(Some(GotoDefinitionResponse::Scalar(locations[0].to_lsp()))),
            _ => Ok(Some(GotoDefinitionResponse::Array(
                locations.iter().map(|l| l.to_lsp()).collect()
            ))),
        }
    }
}
```

#### 4.3 Diagnostics Handler
**Current:** Calls `runtime.diagnostics()` on `BamlRuntime`.

**New:** Use `collect_diagnostics()` from new infrastructure.

```rust
// engine/language_server/src/server/api/diagnostics.rs
pub fn project_diagnostics(db: &LspDatabase) -> HashMap<Url, Vec<Diagnostic>> {
    let all_diagnostics = collect_diagnostics(db);

    // Group by file
    let mut by_file: HashMap<Url, Vec<Diagnostic>> = HashMap::new();
    for diag in all_diagnostics {
        let url = file_to_url(diag.file);
        by_file.entry(url).or_default().push(diag.to_lsp());
    }

    by_file
}
```

#### 4.4 Rename Handler
**Current:** Partially implemented for classes/enums.

**New:** Full implementation using HIR for finding all references.

```rust
impl SyncRequestHandler for Rename {
    fn run(session: &mut Session, ...) -> Result<Option<WorkspaceEdit>> {
        let db = session.get_lsp_database(&path)?;

        // Find the symbol being renamed
        let symbol = db.symbol_at_position(file, position)?;

        // Find all references to this symbol
        let references = db.find_all_references(&symbol.name);

        // Create text edits for each reference
        let edits = references.iter().map(|r| TextEdit {
            range: r.span.to_lsp_range(),
            new_text: new_name.clone(),
        }).collect();

        Ok(Some(WorkspaceEdit { changes: edits, .. }))
    }
}
```

### Phase 5: Session Management

#### 5.1 Replace BamlProject with LspDatabase
Modify `Session` to use `LspDatabase` instead of `BamlProject`.

```rust
// engine/language_server/src/session.rs
pub struct Session {
    // OLD: pub projects: HashMap<PathBuf, Arc<Mutex<Project>>>,
    // NEW:
    pub databases: HashMap<PathBuf, LspDatabase>,
    // ...
}

impl Session {
    pub fn get_or_create_database(&mut self, path: &Path) -> &mut LspDatabase {
        // ...
    }
}
```

#### 5.2 File Change Handling
Update file change handlers to use Salsa's incremental system.

```rust
// engine/language_server/src/server/api/notifications/did_change.rs
impl NotificationHandler for DidChangeTextDocument {
    fn run(session: &mut Session, ...) {
        let db = session.get_or_create_database(&path);

        // Salsa handles incrementality automatically
        db.add_or_update_file(&path, &new_content);

        // Diagnostics will be recomputed on demand
        publish_diagnostics(notifier, db);
    }
}
```

### Phase 6: Feature Parity Checklist

| Feature | Current Implementation | New Implementation | Status |
|---------|----------------------|-------------------|--------|
| Diagnostics | `runtime.diagnostics()` | `collect_diagnostics()` | TODO |
| Hover | `search_for_symbol()` | `symbol_at_position()` + signatures | TODO |
| Go-to-Definition | `search_for_symbol()` | `find_symbol_locations()` | TODO |
| Rename | Partial (class/enum) | `find_all_references()` | TODO |
| Completions | Commented out | HIR item enumeration | TODO |
| Code Actions | Open playground | Keep as-is | N/A |
| Formatting | `format_schema()` | TBD (need formatter) | TODO |
| Code Lens | Basic | Keep as-is initially | N/A |

### Phase 7: Testing Strategy

#### 7.1 Unit Tests
Test individual components in isolation:
- `LspDatabase` file management
- Symbol lookup accuracy
- Diagnostic conversion
- Position/span conversions

#### 7.2 Integration Tests
Test full LSP request/response cycles:
- Hover on various symbol types
- Go-to-definition across files
- Diagnostics for various error types
- Rename refactoring

#### 7.3 Regression Tests
Ensure existing functionality works:
- Compare diagnostics output (old vs new)
- Compare symbol locations (old vs new)

## Implementation Order

1. **Week 1: Foundation**
   - [ ] Create `LspDatabase` wrapper
   - [ ] Implement position/span conversion utilities
   - [ ] Add basic file management

2. **Week 2: Diagnostics**
   - [ ] Implement `collect_diagnostics()`
   - [ ] Migrate diagnostic publishing
   - [ ] Test error reporting

3. **Week 3: Symbol Resolution**
   - [ ] Implement `find_symbol()`
   - [ ] Implement `symbol_at_position()`
   - [ ] Migrate Hover handler

4. **Week 4: Navigation**
   - [ ] Implement `find_symbol_locations()`
   - [ ] Migrate Go-to-Definition handler
   - [ ] Add multi-location support

5. **Week 5: Refactoring**
   - [ ] Implement `find_all_references()`
   - [ ] Complete Rename handler
   - [ ] Add reference highlighting

6. **Week 6: Polish**
   - [ ] Implement completions (if time)
   - [ ] Performance testing
   - [ ] Bug fixes and edge cases

## Dependencies and Blockers

### Required from baml_language
1. **Span tracking in HIR** - Need accurate source locations for all items
2. **Source text access** - Need to retrieve original source text for hover
3. **Cross-file analysis** - Need project-wide symbol resolution

### Potential Issues
1. **Generator validation** - Currently in old runtime, needs migration path
2. **Codegen integration** - LSP triggers codegen, need new path
3. **Feature flags** - Need to preserve feature flag handling

## Appendix A: Intermediate Types to Eliminate

### Current Type Hierarchy (Too Many Layers)

```
Old Compiler                    LSP Types                      LSP Protocol
─────────────                   ─────────                      ────────────
DatamodelError ──────────────►  BamlError ──────────────────►  lsp_types::Diagnostic
    │                               │
    └── Span ──────────────────►  BamlSpan (redundant)
                                    │
                                    └── file_path, start, end,
                                        start_line, end_line

SymbolLocation (runtime) ──────► SymbolLocation (lsp-types) ──► lsp_types::Location
```

### Types That Can Be Eliminated

#### 1. `BamlError` and `BamlDiagnosticError` (baml-lsp-types)
**Current:**
```rust
pub struct BamlError {
    pub r#type: String,
    pub file_path: String,
    pub start_ch: usize,
    pub end_ch: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub message: String,
}
```

**Problem:** This is just an intermediate representation between compiler errors and LSP Diagnostics.

**Solution:** Convert directly from `baml_diagnostics::{ParseError, TypeError, NameError}` to `lsp_types::Diagnostic`:

```rust
fn parse_error_to_diagnostic(error: &ParseError, db: &RootDatabase) -> lsp_types::Diagnostic {
    let range = span_to_lsp_range(db, &error.span());
    lsp_types::Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        message: error.message(),
        source: Some("baml".to_string()),
        ..Default::default()
    }
}
```

#### 2. `BamlSpan` (baml-lsp-types)
**Current:**
```rust
pub struct BamlSpan {
    pub file_path: String,
    pub start: usize,      // byte offset
    pub end: usize,        // byte offset
    pub start_line: usize, // redundant - can compute from offset
    pub end_line: usize,   // redundant - can compute from offset
}
```

**Problem:** Stores redundant information (both byte offsets AND precomputed line numbers).

**Solution:** Use `baml_base::Span` directly and compute line/column only when needed for LSP:
```rust
// baml_base::Span is just:
pub struct Span {
    pub file_id: FileId,
    pub range: TextRange,  // byte offsets only
}

// Convert to LSP only when needed:
fn span_to_lsp_range(db: &dyn Db, span: &Span) -> lsp_types::Range {
    let file = db.lookup_file(span.file_id);
    let text = file.text(db);
    let line_index = LineIndex::new(&text);

    lsp_types::Range {
        start: offset_to_position(&line_index, span.range.start()),
        end: offset_to_position(&line_index, span.range.end()),
    }
}
```

#### 3. `SymbolLocation` (baml-lsp-types)
**Current:**
```rust
pub struct SymbolLocation {
    pub uri: String,
    pub start_line: usize,
    pub start_character: usize,
    pub end_line: usize,
    pub end_character: usize,
}
```

**Problem:** This is almost identical to `lsp_types::Location`.

**Solution:** Return `lsp_types::Location` directly from symbol lookup:
```rust
fn find_symbol(db: &LspDatabase, name: &str) -> Option<lsp_types::Location> {
    // ... lookup logic ...
    Some(lsp_types::Location {
        uri: Url::from_file_path(file_path).unwrap(),
        range: span_to_lsp_range(db, &span),
    })
}
```

### Types to Keep (Serve Real Purpose)

#### Playground/Test Types
These types serve the VSCode extension's playground and test runner:
- `BamlFunction` - Function metadata for test UI
- `BamlFunctionTestCasePair` - Test case info
- `BamlParam` - Parameter info for test inputs
- `BamlGeneratorConfig` - Generator info

**Recommendation:** Keep these but review if they can be simplified or generated from HIR queries.

#### Notification Types
- `RuntimeUpdated` - Custom LSP notification
- `BamlNotification` - Enum of custom notifications

**Recommendation:** Keep these for now; they're protocol extensions.

### Error Type Design: Single Source of Truth for Messages

To avoid duplicating user-facing error messages across Ariadne and LSP conversion paths, the error types themselves should provide accessor methods for all display-relevant data:

```rust
impl<T: std::fmt::Display> TypeError<T> {
    /// The user-facing error message (SINGLE SOURCE OF TRUTH)
    pub fn message(&self) -> String {
        match self {
            TypeError::TypeMismatch { expected, found, .. } => {
                format!("Expected `{expected}`, found `{found}`")
            }
            TypeError::UnknownVariable { name, .. } => {
                format!("Unknown variable `{name}`")
            }
            TypeError::NoSuchField { ty, field, .. } => {
                format!("Type `{ty}` has no field `{field}`")
            }
            // ... etc
        }
    }

    /// Primary error location
    pub fn span(&self) -> Span { ... }

    /// Error code for documentation/filtering
    pub fn code(&self) -> ErrorCode { ... }

    /// Secondary locations with explanatory messages
    /// e.g., "expected type defined here", "first definition here"
    pub fn related_spans(&self) -> Vec<(Span, String)> { ... }
}
```

Then both conversion paths use these methods without duplicating message logic:

```rust
// Ariadne conversion - uses error.message()
fn type_error_to_ariadne<T: Display>(error: &TypeError<T>) -> Report<Span> {
    let mut builder = Report::build(ReportKind::Error, error.span())
        .with_message(error.message())
        .with_label(Label::new(error.span()).with_message(error.message()));

    for (span, msg) in error.related_spans() {
        builder = builder.with_label(Label::new(span).with_message(msg));
    }

    builder.with_note(format!("Error code: {}", error.code())).finish()
}

// LSP conversion - uses same error.message()
fn type_error_to_lsp<T: Display>(error: &TypeError<T>, db: &dyn Db) -> Diagnostic {
    Diagnostic {
        range: span_to_range(db, &error.span()),
        message: error.message(),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(error.code().to_string())),
        related_information: Some(
            error.related_spans()
                .iter()
                .map(|(span, msg)| DiagnosticRelatedInformation {
                    location: span_to_location(db, span),
                    message: msg.clone(),
                })
                .collect()
        ),
        ..Default::default()
    }
}
```

**Benefits:**
- Message text defined once, used by both CLI and LSP
- Error types are the single source of truth
- Conversion functions only handle structural/coordinate transformation
- Easy to add new output formats (JSON, SARIF, etc.) without duplicating messages

**Apply this pattern to all error types:**
- `ParseError::message()`, `span()`, `code()`
- `TypeError::message()`, `span()`, `code()`, `related_spans()`
- `NameError::message()`, `span()`, `code()`, `related_spans()`

### Ariadne Reports vs LSP Diagnostics

**Important:** The new compiler uses Ariadne for beautiful CLI error rendering:
```rust
render_parse_error(error, sources, color) -> String  // Pretty terminal output
```

**For LSP, we should NOT use Ariadne.** LSP needs structured data:
```rust
// DON'T do this for LSP:
let pretty_message = render_parse_error(&error, &sources, false);

// DO this instead:
let diagnostic = lsp_types::Diagnostic {
    range: span_to_lsp_range(db, &error.span()),
    severity: Some(DiagnosticSeverity::ERROR),
    code: Some(NumberOrString::String(error.code().to_string())),
    message: error.message(),  // Plain message, not rendered
    ..Default::default()
};
```

Ariadne rendering is for:
- CLI output (`baml-cli check`)
- Terminal in `baml_onionskin`

LSP diagnostics need:
- Structured range/severity/message
- No ANSI colors
- Machine-parseable format

### Simplified Architecture After Cleanup

```
New Compiler                                    LSP Protocol
────────────                                    ────────────
baml_diagnostics::ParseError ─────────────────► lsp_types::Diagnostic
baml_diagnostics::TypeError  ─────────────────► lsp_types::Diagnostic
baml_diagnostics::NameError  ─────────────────► lsp_types::Diagnostic

baml_base::Span + LineIndex  ─────────────────► lsp_types::Range

baml_hir::ItemId + Span      ─────────────────► lsp_types::Location
```

**Benefits:**
1. Fewer allocations (no intermediate String copies)
2. Simpler code (direct conversion)
3. Single source of truth for spans
4. Better type safety (no stringly-typed file paths)

## Appendix B: API Mapping

### Old → New API Equivalents

| Old API | New API |
|---------|---------|
| `BamlRuntime::from_file_content()` | `RootDatabase::new()` + `add_file()` |
| `runtime.ir.find_class(name)` | `project_items()` + filter by name |
| `runtime.ir.find_enum(name)` | `project_items()` + filter by name |
| `runtime.ir.find_function(name)` | `project_items()` + filter by name |
| `runtime.diagnostics()` | `parse_errors()` + `validate_duplicate_names()` |
| `walker.span()` | `FunctionLoc::file()` + syntax tree span |
| `span.line_and_column()` | Line index computation from offset |

### New Queries Available

```rust
// HIR Queries
file_item_tree(db, file) -> Arc<ItemTree>
file_items(db, file) -> FileItems<'db>
project_items(db, root) -> ProjectItems<'db>
function_signature(db, func) -> Arc<FunctionSignature>
function_body(db, func) -> Arc<FunctionBody>
class_fields(db, class) -> ClassFields<'db>
project_class_fields(db, root) -> ProjectClassFields<'db>

// THIR Queries
infer_function(db, sig, body, globals, class_fields) -> InferenceResult
build_typing_context_from_files(db, files) -> HashMap<Name, Ty>
build_class_fields_from_files(db, project) -> HashMap<Name, HashMap<Name, Ty>>

// Parser Queries
parse_result(db, file) -> ParseResult
parse_errors(db, file) -> Vec<ParseError>
syntax_tree(db, file) -> SyntaxNode
```
