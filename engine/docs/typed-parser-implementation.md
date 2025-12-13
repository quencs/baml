# Typed Stream Parser: Design Analysis and Implementation Plan

## Executive Summary

The typed stream parser (`typed_stream`) was designed to replace the legacy two-phase parsing pipeline (jsonish::parse → TypeIR::coerce) with a direct type-directed parser. After integration, tests showed **~175 passing, ~332 failing**. This document analyzes the root causes and proposes a design that properly integrates with the existing type system.

## Current Architecture

### Legacy Parser Flow
```
raw_string → jsonish::parse() → jsonish::Value → TypeIR::coerce(ctx) → BamlValueWithFlags
                                                        ↓
                                              ParsingContext has:
                                              - OutputFormatContent
                                              - Classes with field definitions
                                              - Enums with value definitions
```

### Key Legacy Components
1. **ParsingContext** (`coercer/mod.rs:21-28`):
   - Holds `&OutputFormatContent` with all class/enum definitions
   - Tracks visited classes for recursion detection
   - Has streaming mode info

2. **OutputFormatContent** (`jinja-runtime/src/output_format/types.rs:93-99`):
   ```rust
   pub struct OutputFormatContent {
       pub enums: Arc<IndexMap<String, Enum>>,
       pub classes: Arc<IndexMap<(String, StreamingMode), Class>>,
       pub recursive_classes: Arc<IndexSet<String>>,
       pub structural_recursive_aliases: Arc<IndexMap<String, TypeIR>>,
       pub target: TypeIR,
   }
   ```

3. **Class structure** (`types.rs:63-72`):
   ```rust
   pub struct Class {
       pub name: Name,
       pub description: Option<String>,
       pub namespace: StreamingMode,
       pub fields: Vec<(Name, TypeIR, Option<String>, bool)>,  // (name, type, description, streaming_needed)
       pub constraints: Vec<Constraint>,
       pub streaming_behavior: StreamingBehavior,
   }
   ```

4. **IrRef dispatch** (`ir_ref/mod.rs:53-55`):
   ```rust
   IrRef::Class(c, mode) => match ctx.of.find_class(mode, c.as_str()) {
       Ok(c) => c.coerce(ctx, target, value),  // c is &Class with all field info
       ...
   }
   ```

### Current typed_stream Flow
```
raw_string → extract_spans() → TypedStreamParser::ingest() → ParsedValue → coerce → BamlValueWithFlags
                                        ↓
                              SchemaIndex (built from TypeIR only)
                              - No OutputFormatContent
                              - No class field definitions
                              - No enum value definitions
```

## Root Cause Analysis

### Category 1: Missing Schema Information (Primary Issue)

**Problem**: SchemaIndex is built from TypeIR alone, but TypeIR::Class only contains:
- `name: String`
- `mode: StreamingMode`

It does NOT contain:
- Field definitions (name, type, required, aliases)
- Constraints
- Streaming behavior

**Affected Tests**: All class parsing tests (~200+ tests)

**Example Failure**:
```
test_json_md_example_1: "json atom at path '.array' is missing"
```
The parser doesn't know class `Test` has a field called `array`.

### Category 2: Enum Value Resolution

**Problem**: TypeIR::Enum only contains the enum name. Actual values come from OutputFormatContent.

**Current**: SchemaIndex builds empty enum value maps
**Needed**: Access to `OutputFormatContent.enums` for value/alias resolution

**Affected Tests**: All enum parsing tests with aliases or fuzzy matching

### Category 3: Field Aliasing

**Problem**: The legacy parser uses `matches_string_to_string(ctx, key, name.rendered_name())` for key matching, which handles aliases. The new parser doesn't have access to Name objects.

**Example**: A field named `foo` with alias `@alias("bar")` should match both `"foo"` and `"bar"` in input.

### Category 4: Metadata Preservation (Partially Fixed)

**Problem**: Tests compare `value.field_type() == target_type`, but target_type has `needed: true` from `ir.finalize_type()`.

**Status**: Fixed by storing `source_type` in SchemaIndex

### Category 5: Coercion Missing Cases (Partially Fixed)

Fixed:
- Int/Float to String coercion
- Boolean extraction from prose
- Thousand separator numbers

Still Missing:
- Complex union narrowing
- Recursive type handling
- Default values for missing fields

## Proposed Design

### Option A: Pass OutputFormatContent to SchemaIndex (Recommended)

**Architecture**:
```
raw_string → extract_spans() → TypedStreamParser::ingest() → ParsedValue → coerce → BamlValueWithFlags
                                        ↓
                              SchemaIndex (built from TypeIR + OutputFormatContent)
                              - Class fields from of.classes
                              - Enum values from of.enums
                              - Full metadata preserved
```

**Changes Required**:

1. **SchemaIndex::build** takes `OutputFormatContent`:
   ```rust
   pub fn build(root: &TypeIR, of: &OutputFormatContent) -> Self
   ```

2. **FieldInfo** includes Name for alias support:
   ```rust
   pub struct FieldInfo {
       pub name: String,        // rendered_name
       pub real_name: String,   // for output
       pub type_id: TypeId,
       pub required: bool,
       pub aliases: Vec<String>,
       pub source_type: TypeIR, // original TypeIR
   }
   ```

3. **TypeKind::Class** stores full field info:
   ```rust
   TypeKind::Class {
       name: String,
       fields: IndexMap<String, FieldInfo>,  // ordered for output
       alias_map: HashMap<String, String>,   // alias → field name
       required: HashSet<String>,
       source_class: Class,  // Reference to original Class
   }
   ```

4. **TypeKind::Enum** stores value info:
   ```rust
   TypeKind::Enum {
       name: String,
       values: HashMap<String, String>,      // exact match
       fuzzy_map: HashMap<String, String>,   // lowercase → canonical
   }
   ```

### Option B: Hybrid Parser (Fallback for Complex Types)

Use typed_stream for:
- Primitives (string, int, float, bool)
- Lists of primitives
- Simple enums

Fall back to legacy for:
- Classes
- Complex unions
- Nested structures

**Downside**: Doesn't achieve the goal of avoiding exponential AnyOf blowup in complex cases.

### Option C: Full Rewrite with ParsingContext

Create a new parser that mirrors the legacy architecture more closely:

```rust
pub struct TypedParser<'a> {
    ctx: ParsingContext<'a>,  // Same as legacy
    lexer: Lexer,
    beam: ExpectedTypeSet,
}
```

**Pros**: More compatible with legacy behavior
**Cons**: More work, may duplicate code

## Recommended Implementation Plan

### Phase 1: Fix SchemaIndex to use OutputFormatContent

1. Update `SchemaIndex::build(root: &TypeIR, of: &OutputFormatContent)`
2. For TypeKind::Class, lookup class from `of.classes.get(&(name, mode))`
3. For TypeKind::Enum, lookup enum from `of.enums.get(&name)`
4. Store original TypeIR in each TypeInfo for metadata preservation

### Phase 2: Update Coercion Layer

1. Pass `&OutputFormatContent` through to coercion functions
2. Implement field alias matching using `Name.rendered_name()` and aliases
3. Add constraint evaluation (reuse `run_user_checks` from legacy)
4. Handle default values for missing optional fields

### Phase 3: Handle Edge Cases

1. Recursive classes (track visited for cycle detection)
2. Complex unions with discriminator keys
3. Streaming mode handling (@@stream.done, @@stream.needed)

### Phase 4: Test Parity

1. Run full test suite
2. Compare behavior with legacy parser
3. Add any missing coercion cases

## Code Structure

```
typed_stream/
├── mod.rs              # Entry point: parse()
├── lexer.rs            # Tokenization
├── parser.rs           # Beam-based parsing
├── schema_index.rs     # Type indexing with OutputFormatContent
├── coerce.rs           # BamlValueWithFlags conversion
├── expected_set.rs     # Beam candidates
├── frames.rs           # ParsedValue types
├── extract.rs          # Span extraction
└── session.rs          # Parse state
```

## Key Data Structures

### SchemaIndex (Updated)
```rust
pub struct SchemaIndex {
    next_id: TypeId,
    type_ids: HashMap<TypeKey, TypeId>,
    pub type_info: HashMap<TypeId, TypeInfo>,
    root_id: TypeId,
}

pub struct TypeInfo {
    pub kind: TypeKind,
    pub id: TypeId,
    pub source_type: TypeIR,  // Original TypeIR with finalized metadata
}
```

### Class Field Lookup
```rust
impl SchemaIndex {
    /// Get field info by any valid key (name or alias)
    pub fn get_field(&self, class_id: TypeId, key: &str) -> Option<&FieldInfo> {
        match self.get(class_id)?.kind {
            TypeKind::Class { fields, alias_map, .. } => {
                // Try exact match first
                fields.get(key)
                    // Then try alias
                    .or_else(|| alias_map.get(key).and_then(|n| fields.get(n)))
            }
            _ => None
        }
    }
}
```

## Test Categories by Priority

1. **High Priority** (blocking most tests):
   - Class field resolution
   - Enum value matching

2. **Medium Priority**:
   - Field aliases
   - Optional field defaults
   - Constraint evaluation

3. **Lower Priority**:
   - Complex union narrowing
   - Recursive types
   - Edge case coercions

## Appendix: Key File Locations

- Legacy coercer: `jsonish/src/deserializer/coercer/`
- Class coercion: `jsonish/src/deserializer/coercer/ir_ref/coerce_class.rs`
- OutputFormatContent: `jinja-runtime/src/output_format/types.rs`
- SchemaIndex: `jsonish/src/typed_stream/schema_index.rs`
- Test macros: `jsonish/src/tests/macros.rs`
