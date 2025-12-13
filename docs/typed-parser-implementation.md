# Typed Next-Key Streaming Parser - Implementation Specification

This document translates the design in `typed-next-key-streaming-parser.md` into concrete code changes, file structures, and pseudocode.

---

## Overview: What We're Building

**Goal:** Replace the exponential `AnyOf` parsing pipeline with a bounded beam-search typed parser.

**Current flow (to be replaced):**
```
raw_string
  -> jsonish::parse() -> Value (with nested AnyOf)
  -> TypeIR::coerce() -> BamlValueWithFlags
```

**New flow:**
```
raw_string
  -> TypedStreamParser::parse() -> BamlValueWithFlags (directly)
```

---

## File Structure

Create a new module at `engine/baml-lib/jsonish/src/typed_stream/`:

```
typed_stream/
├── mod.rs              # Public API + TypedStreamParser
├── lexer.rs            # Tolerant streaming tokenizer
├── schema_index.rs     # TypeIR -> indexed schema for fast lookups
├── expected_set.rs     # ExpectedTypeSet (bounded beam)
├── frames.rs           # Frame types (Object, Array, Value)
├── session.rs          # ParseSession state
├── parser.rs           # Core parsing logic + next_keys()
├── coerce.rs           # Primitive/enum/literal coercion
├── extract.rs          # Segment selection (markdown/grep)
└── tests/              # Unit tests for each component
    ├── lexer_tests.rs
    ├── expected_set_tests.rs
    └── parser_tests.rs
```

---

## Component 1: Tolerant Streaming Lexer

**File:** `typed_stream/lexer.rs`

### Token Types

Each token carries its own completion state, which propagates up to the parsed value.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Structural - always complete (atomic)
    LBrace,           // {
    RBrace,           // }
    LBracket,         // [
    RBracket,         // ]
    Colon,            // :
    Comma,            // ,

    // Values - carry completion state
    String {
        content: String,
        quote: QuoteStyle,    // Double, Single, None (unquoted)
        complete: bool,       // false if no closing quote seen
    },
    Number {
        raw: String,
        complete: bool,       // false if mid-digits at chunk boundary
    },
    True,             // Always complete (atomic keyword)
    False,            // Always complete
    Null,             // Always complete

    // Special: triple-backtick code blocks as values
    CodeBlock {
        lang: Option<String>,
        content: String,
        complete: bool,       // false if no closing ``` seen
    },

    // Whitespace/comments (skipped but tracked for resume)
    // Not emitted, but lexer handles internally
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuoteStyle {
    Double,    // "..."
    Single,    // '...'
    Unquoted,  // bare identifier or value
}

impl Token {
    /// Returns the completion state of this token
    pub fn is_complete(&self) -> bool {
        match self {
            // Structural tokens are always complete
            Token::LBrace | Token::RBrace |
            Token::LBracket | Token::RBracket |
            Token::Colon | Token::Comma |
            Token::True | Token::False | Token::Null => true,

            // Value tokens carry their own state
            Token::String { complete, .. } => *complete,
            Token::Number { complete, .. } => *complete,
            Token::CodeBlock { complete, .. } => *complete,
        }
    }
}
```

### Lexer State Machine

```rust
pub struct Lexer {
    /// Buffered input not yet fully consumed
    buffer: String,
    /// Current position in buffer
    pos: usize,
    /// Mode for handling incomplete constructs across chunks
    mode: LexMode,
    /// Accumulated content for current incomplete token
    pending: String,
}

#[derive(Debug, Clone, Default)]
enum LexMode {
    #[default]
    Normal,
    InString {
        quote: char,
        escaped: bool,
    },
    InLineComment,
    InBlockComment {
        saw_star: bool,
    },
    InCodeBlock {
        backtick_count: u8,
        lang: Option<String>,
        saw_opening_newline: bool,
        closing_backticks: u8,
    },
    InUnquotedValue {
        // For unquoted values, we need context to know when to stop
        // Stop at: , } ] : (in key position) or newline
    },
}

impl Lexer {
    pub fn new() -> Self { ... }

    /// Append new chunk to buffer
    pub fn append(&mut self, chunk: &str) {
        self.buffer.push_str(chunk);
    }

    /// Extract all complete tokens, leaving incomplete state in buffer
    pub fn drain_tokens(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(tok) = self.next_token() {
            tokens.push(tok);
        }
        tokens
    }

    /// Try to extract next token, returns None if need more input
    fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace_and_comments();

        let c = self.peek_char()?;

        match self.mode {
            LexMode::Normal => self.lex_normal(c),
            LexMode::InString { quote, escaped } => self.lex_string(quote, escaped),
            LexMode::InLineComment => { self.skip_to_newline(); self.next_token() }
            LexMode::InBlockComment { saw_star } => { self.skip_block_comment(saw_star); self.next_token() }
            LexMode::InCodeBlock { .. } => self.lex_code_block(),
            LexMode::InUnquotedValue => self.lex_unquoted_value(),
        }
    }

    fn lex_normal(&mut self, c: char) -> Option<Token> {
        match c {
            '{' => { self.advance(); Some(Token::LBrace) }
            '}' => { self.advance(); Some(Token::RBrace) }
            '[' => { self.advance(); Some(Token::LBracket) }
            ']' => { self.advance(); Some(Token::RBracket) }
            ':' => { self.advance(); Some(Token::Colon) }
            ',' => { self.advance(); Some(Token::Comma) }
            '"' | '\'' => self.start_string(c),
            '`' if self.peek_ahead(3) == "```" => self.start_code_block(),
            '/' if self.peek_ahead(2) == "//" => { self.mode = LexMode::InLineComment; self.next_token() }
            '/' if self.peek_ahead(2) == "/*" => { self.mode = LexMode::InBlockComment { saw_star: false }; self.next_token() }
            _ if c.is_ascii_digit() || c == '-' || c == '.' => self.lex_number(),
            _ => self.lex_unquoted(),
        }
    }

    /// Lex unquoted key or value
    /// Handles: `foo`, `some value with spaces`, `true`, `false`, `null`
    fn lex_unquoted(&mut self) -> Option<Token> {
        let start = self.pos;

        // Collect until we hit a structural char or end
        // For keys: stop at ':'
        // For values: stop at ',' '}' ']' or newline
        while let Some(c) = self.peek_char() {
            if matches!(c, '{' | '}' | '[' | ']' | ':' | ',' | '\n') {
                break;
            }
            self.advance();
        }

        let raw = self.buffer[start..self.pos].trim();

        // Check for keywords
        match raw.to_lowercase().as_str() {
            "true" => Some(Token::True),
            "false" => Some(Token::False),
            "null" => Some(Token::Null),
            _ => Some(Token::String {
                content: raw.to_string(),
                quote: QuoteStyle::Unquoted,
                complete: true,
            }),
        }
    }
}
```

### Key Behaviors to Match

From existing `fixing_parser/json_parse_state.rs`:
- Unquoted keys: `{ foo: 1 }` -> key is `"foo"`
- Unquoted values with spaces: `{ key: some value }` -> value is `"some value"`
- Trailing commas: `[1,2,]` -> valid
- Comments: `{ /* comment */ "key": 1 }` -> skip comment
- Triple-backtick strings: `` `​`​`lang\ncontent\n`​`​` `` -> CodeBlock token

---

## Component 2: Schema Index

**File:** `typed_stream/schema_index.rs`

Pre-compute lookups from `TypeIR` for fast parsing decisions.

```rust
use std::collections::{HashMap, HashSet};
use internal_baml_core::ir::TypeIR;

pub type TypeId = u32;

/// Pre-indexed schema for a type tree
pub struct SchemaIndex {
    /// TypeId counter
    next_id: TypeId,
    /// TypeIR -> TypeId mapping (for recursion detection)
    type_ids: HashMap<TypeIRKey, TypeId>,
    /// Per-type metadata
    type_info: HashMap<TypeId, TypeInfo>,
}

/// Key for deduplicating TypeIR (class name + type params hash)
#[derive(Hash, Eq, PartialEq, Clone)]
struct TypeIRKey(String);

impl TypeIRKey {
    fn from_type(ty: &TypeIR) -> Self {
        // Hash class name + params for stable identity
        TypeIRKey(format!("{:?}", ty))
    }
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub kind: TypeKind,
    pub id: TypeId,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Primitive(PrimitiveKind),
    Enum {
        name: String,
        /// value -> canonical name (for alias matching)
        values: HashMap<String, String>,
        /// lowercase/normalized -> canonical (for fuzzy matching)
        fuzzy_map: HashMap<String, String>,
    },
    Literal(LiteralKind),
    Class {
        name: String,
        /// field_name -> FieldInfo
        fields: HashMap<String, FieldInfo>,
        /// alias -> canonical field name
        alias_map: HashMap<String, String>,
        /// Required field names
        required: HashSet<String>,
    },
    List {
        element: TypeId,
    },
    Map {
        key: TypeId,
        value: TypeId,
    },
    Union {
        variants: Vec<TypeId>,
        /// key -> which variant indices have this key (for narrowing)
        key_to_variants: HashMap<String, Vec<usize>>,
    },
    RecursiveAlias {
        target: TypeId,
    },
}

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: String,
    pub type_id: TypeId,
    pub required: bool,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum PrimitiveKind {
    String,
    Int,
    Float,
    Bool,
    Null,
}

#[derive(Debug, Clone)]
pub enum LiteralKind {
    String(String),
    Int(i64),
    Bool(bool),
}

impl SchemaIndex {
    /// Build index from root TypeIR
    pub fn build(root: &TypeIR) -> Self {
        let mut index = SchemaIndex {
            next_id: 0,
            type_ids: HashMap::new(),
            type_info: HashMap::new(),
        };
        index.index_type(root);
        index
    }

    fn index_type(&mut self, ty: &TypeIR) -> TypeId {
        let key = TypeIRKey::from_type(ty);

        // Return existing ID if already indexed (handles recursion)
        if let Some(&id) = self.type_ids.get(&key) {
            return id;
        }

        // Allocate ID first (before recursing) to handle cycles
        let id = self.next_id;
        self.next_id += 1;
        self.type_ids.insert(key, id);

        let kind = match ty {
            TypeIR::Primitive(prim, _) => TypeKind::Primitive(match prim {
                TypeValue::String => PrimitiveKind::String,
                TypeValue::Int => PrimitiveKind::Int,
                TypeValue::Float => PrimitiveKind::Float,
                TypeValue::Bool => PrimitiveKind::Bool,
                TypeValue::Null => PrimitiveKind::Null,
            }),

            TypeIR::Class { name, fields, .. } => {
                let mut field_map = HashMap::new();
                let mut alias_map = HashMap::new();
                let mut required = HashSet::new();

                for field in fields {
                    let field_type_id = self.index_type(&field.r#type);
                    let info = FieldInfo {
                        name: field.name.clone(),
                        type_id: field_type_id,
                        required: !field.r#type.is_optional(),
                        aliases: field.aliases.clone(),
                    };

                    if info.required {
                        required.insert(field.name.clone());
                    }

                    for alias in &field.aliases {
                        alias_map.insert(alias.to_lowercase(), field.name.clone());
                    }

                    field_map.insert(field.name.clone(), info);
                }

                TypeKind::Class {
                    name: name.clone(),
                    fields: field_map,
                    alias_map,
                    required,
                }
            }

            TypeIR::Union(options, _) => {
                let variants: Vec<TypeId> = options
                    .iter_skip_null()
                    .map(|opt| self.index_type(opt))
                    .collect();

                // Build key -> variant index map
                let mut key_to_variants: HashMap<String, Vec<usize>> = HashMap::new();
                for (i, &var_id) in variants.iter().enumerate() {
                    if let Some(TypeInfo { kind: TypeKind::Class { fields, alias_map, .. }, .. }) = self.type_info.get(&var_id) {
                        for key in fields.keys() {
                            key_to_variants.entry(key.clone()).or_default().push(i);
                        }
                        for alias in alias_map.keys() {
                            key_to_variants.entry(alias.clone()).or_default().push(i);
                        }
                    }
                }

                TypeKind::Union { variants, key_to_variants }
            }

            TypeIR::List(elem, _) => {
                let elem_id = self.index_type(elem);
                TypeKind::List { element: elem_id }
            }

            // ... other cases
            _ => todo!(),
        };

        self.type_info.insert(id, TypeInfo { kind, id });
        id
    }

    pub fn get(&self, id: TypeId) -> Option<&TypeInfo> {
        self.type_info.get(&id)
    }

    pub fn root_id(&self) -> TypeId {
        0 // First indexed type is root
    }
}
```

---

## Component 3: ExpectedTypeSet (Bounded Beam)

**File:** `typed_stream/expected_set.rs`

```rust
use super::schema_index::TypeId;

/// Maximum candidates to track (beam width)
pub const DEFAULT_BEAM_K: usize = 8;

#[derive(Debug, Clone)]
pub struct Candidate {
    pub type_id: TypeId,
    pub score: i32,
    pub dead: bool,  // Eliminated by hard evidence
}

/// Bounded set of possible types at a parse position
#[derive(Debug, Clone)]
pub struct ExpectedTypeSet {
    candidates: Vec<Candidate>,
    max_k: usize,
}

impl ExpectedTypeSet {
    pub fn single(type_id: TypeId) -> Self {
        ExpectedTypeSet {
            candidates: vec![Candidate { type_id, score: 0, dead: false }],
            max_k: DEFAULT_BEAM_K,
        }
    }

    pub fn from_union(variant_ids: &[TypeId], max_k: usize) -> Self {
        let candidates = variant_ids.iter()
            .map(|&id| Candidate { type_id: id, score: 0, dead: false })
            .collect();
        ExpectedTypeSet { candidates, max_k }
    }

    /// Narrow by structural token (saw `{` or `[`)
    pub fn narrow_by_structure(&mut self, schema: &SchemaIndex, saw_brace: bool) {
        for cand in &mut self.candidates {
            if cand.dead { continue; }

            let info = schema.get(cand.type_id);
            let is_compatible = match (saw_brace, info.map(|i| &i.kind)) {
                (true, Some(TypeKind::Class { .. })) => true,
                (true, Some(TypeKind::Map { .. })) => true,
                (false, Some(TypeKind::List { .. })) => true,  // saw `[`
                _ => false,
            };

            if !is_compatible {
                cand.dead = true;
            }
        }
        self.prune_dead();
    }

    /// Score by observed key (soft narrowing - don't hard-eliminate during streaming)
    pub fn observe_key(&mut self, schema: &SchemaIndex, key: &str, streaming: bool) {
        for cand in &mut self.candidates {
            if cand.dead { continue; }

            let has_key = match schema.get(cand.type_id).map(|i| &i.kind) {
                Some(TypeKind::Class { fields, alias_map, .. }) => {
                    fields.contains_key(key) || alias_map.contains_key(&key.to_lowercase())
                }
                Some(TypeKind::Map { .. }) => true, // Maps accept any key
                _ => false,
            };

            if has_key {
                cand.score += 10;  // Boost for matching key
            } else if streaming {
                cand.score -= 5;   // Soft penalty during streaming
            } else {
                cand.dead = true;  // Hard eliminate when not streaming
            }
        }

        self.prune_dead();
        self.keep_top_k();
    }

    /// After successful field value parse
    pub fn observe_value_success(&mut self, type_id: TypeId) {
        for cand in &mut self.candidates {
            if cand.type_id == type_id {
                cand.score += 5;
            }
        }
        self.keep_top_k();
    }

    /// Collapse to single best if gap is large enough
    pub fn maybe_collapse(&mut self, gap_threshold: i32) {
        if self.candidates.len() < 2 { return; }

        self.candidates.sort_by_key(|c| std::cmp::Reverse(c.score));

        let gap = self.candidates[0].score - self.candidates[1].score;
        if gap >= gap_threshold {
            self.candidates.truncate(1);
        }
    }

    fn prune_dead(&mut self) {
        self.candidates.retain(|c| !c.dead);
    }

    fn keep_top_k(&mut self) {
        if self.candidates.len() <= self.max_k { return; }

        self.candidates.sort_by_key(|c| std::cmp::Reverse(c.score));
        self.candidates.truncate(self.max_k);
    }

    pub fn best(&self) -> Option<TypeId> {
        self.candidates.iter()
            .filter(|c| !c.dead)
            .max_by_key(|c| c.score)
            .map(|c| c.type_id)
    }

    pub fn is_resolved(&self) -> bool {
        self.candidates.iter().filter(|c| !c.dead).count() == 1
    }

    pub fn all_candidates(&self) -> impl Iterator<Item = TypeId> + '_ {
        self.candidates.iter().filter(|c| !c.dead).map(|c| c.type_id)
    }
}
```

---

## Component 4: Parse Frames + CompletionState Tracking

**File:** `typed_stream/frames.rs`

### CompletionState Overview

The parser must track **three completion states** for every value node:

```rust
/// Mirrors baml_types::CompletionState
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionState {
    /// Value is fully observed and syntactically closed
    Complete,
    /// Value is present but syntactically incomplete (unterminated string, unclosed brace)
    Incomplete,
    /// Value has not yet appeared in the stream (placeholder)
    Pending,
}
```

**Rules for determining CompletionState:**

| Value Type | Complete when... | Incomplete when... |
|------------|------------------|-------------------|
| String (quoted) | Closing quote seen | No closing quote yet |
| String (unquoted) | Delimiter seen (`,` `}` `]`) or stream done | Mid-token at stream end |
| Number | Delimiter seen or stream done | Mid-digits at stream end |
| Bool/Null | Keyword fully matched | Never (atomic tokens) |
| Object | `}` seen | `{` seen but no `}` |
| Array | `]` seen | `[` seen but no `]` |
| Field value | Child value is Complete | Child value is Incomplete/Pending |
| Missing field | N/A | N/A → use **Pending** |

### Frame Structures

```rust
use std::collections::HashSet;
use super::expected_set::ExpectedTypeSet;
use super::schema_index::TypeId;

#[derive(Debug, Clone)]
pub enum Frame {
    Object(ObjectFrame),
    Array(ArrayFrame),
    Value(ValueFrame),
}

#[derive(Debug, Clone)]
pub struct ObjectFrame {
    /// Which types we might be parsing
    pub expected: ExpectedTypeSet,
    /// Keys we've seen
    pub seen_keys: HashSet<String>,
    /// Current key being parsed (after `:` but before value)
    pub pending_key: Option<String>,
    /// Parsed field values so far (with completion state per field)
    pub fields: Vec<(String, ParsedValue)>,
    /// Whether we've seen the closing `}`
    pub closed: bool,
    /// Tracks whether the object itself is complete
    pub completion: CompletionState,
}

#[derive(Debug, Clone)]
pub struct ArrayFrame {
    /// Element type
    pub expected_elem: ExpectedTypeSet,
    /// Current element index
    pub index: usize,
    /// Parsed elements (with completion state per element)
    pub elements: Vec<ParsedValue>,
    /// Whether we've seen the closing `]`
    pub closed: bool,
    /// Tracks whether the array itself is complete
    pub completion: CompletionState,
}

#[derive(Debug, Clone)]
pub struct ValueFrame {
    /// Which types we might be parsing
    pub expected: ExpectedTypeSet,
    /// Completion state of the value being parsed
    pub completion: CompletionState,
}

/// Intermediate parsed value WITH completion state
#[derive(Debug, Clone)]
pub struct ParsedValue {
    pub value: ParsedValueKind,
    pub completion: CompletionState,
}

#[derive(Debug, Clone)]
pub enum ParsedValueKind {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Object {
        type_id: TypeId,
        fields: Vec<(String, ParsedValue)>,
    },
    Array(Vec<ParsedValue>),
}

impl ParsedValue {
    pub fn complete(value: ParsedValueKind) -> Self {
        ParsedValue { value, completion: CompletionState::Complete }
    }

    pub fn incomplete(value: ParsedValueKind) -> Self {
        ParsedValue { value, completion: CompletionState::Incomplete }
    }

    pub fn pending() -> Self {
        ParsedValue {
            value: ParsedValueKind::Null,
            completion: CompletionState::Pending
        }
    }
}

impl ObjectFrame {
    pub fn new(expected: ExpectedTypeSet) -> Self {
        ObjectFrame {
            expected,
            seen_keys: HashSet::new(),
            pending_key: None,
            fields: Vec::new(),
            closed: false,
            completion: CompletionState::Incomplete, // Start as incomplete until `}` seen
        }
    }

    /// Derive completion state from children
    pub fn derive_completion(&self) -> CompletionState {
        if !self.closed {
            return CompletionState::Incomplete;
        }
        // Object is complete only if all fields are complete
        if self.fields.iter().all(|(_, v)| v.completion == CompletionState::Complete) {
            CompletionState::Complete
        } else {
            CompletionState::Incomplete
        }
    }
}

impl ArrayFrame {
    pub fn new(expected_elem: ExpectedTypeSet) -> Self {
        ArrayFrame {
            expected_elem,
            index: 0,
            elements: Vec::new(),
            closed: false,
            completion: CompletionState::Incomplete,
        }
    }

    /// Derive completion state from children
    pub fn derive_completion(&self) -> CompletionState {
        if !self.closed {
            return CompletionState::Incomplete;
        }
        if self.elements.iter().all(|v| v.completion == CompletionState::Complete) {
            CompletionState::Complete
        } else {
            CompletionState::Incomplete
        }
    }
}
```

### CompletionState Propagation Rules

The completion state flows **bottom-up** from tokens to the root value:

```
Token Level (Lexer)
├── String token: complete=closing_quote_seen
├── Number token: complete=delimiter_seen (or stream_done if non-streaming)
├── Bool/Null: always complete (atomic)
└── Structural tokens ({, }, [, ]): trigger frame state changes

Frame Level (Parser)
├── ValueFrame: completion = token.complete
├── ArrayFrame: completion = closed && all_elements_complete
└── ObjectFrame: completion = closed && all_fields_complete

Output Level (BamlValueWithFlags)
├── Flag::Pending: field was never seen (filled with default)
├── Flag::Incomplete: value is present but not syntactically closed
└── (no flag): value is complete
```

### Streaming vs Non-Streaming Semantics

```rust
/// Determines how to interpret incomplete tokens at end of input
pub enum StreamingMode {
    /// Parsing a complete response - incomplete tokens are errors
    NonStreaming,
    /// Parsing a partial stream - incomplete tokens become Incomplete state
    Streaming,
}

impl TypedStreamParser {
    fn finalize_completion(&self, value: &mut ParsedValue, mode: StreamingMode) {
        match mode {
            StreamingMode::NonStreaming => {
                // In non-streaming, promote Incomplete to Complete for numbers
                // (we know no more digits are coming)
                if value.completion == CompletionState::Incomplete {
                    if matches!(value.value, ParsedValueKind::Int(_) | ParsedValueKind::Float(_)) {
                        value.completion = CompletionState::Complete;
                    }
                }
            }
            StreamingMode::Streaming => {
                // Keep Incomplete as-is during streaming
            }
        }
    }
}
```

---

## Component 5: Parse Session

**File:** `typed_stream/session.rs`

```rust
use std::collections::HashMap;
use super::frames::Frame;
use super::schema_index::TypeId;

/// Per-parse state
pub struct ParseSession {
    /// Input buffer (append-only)
    pub buffer: String,
    /// Byte offset of last processed position
    pub scan_offset: usize,
    /// Frame stack
    pub stack: Vec<Frame>,
    /// Recursion tracking: (TypeId, depth) -> visit count
    pub recursion_visits: HashMap<(TypeId, usize), u32>,
    /// Maximum allowed depth
    pub max_depth: usize,
    /// Maximum visits per type per depth
    pub max_visits_per_type: u32,
}

impl ParseSession {
    pub fn new(root_frame: Frame) -> Self {
        ParseSession {
            buffer: String::new(),
            scan_offset: 0,
            stack: vec![root_frame],
            recursion_visits: HashMap::new(),
            max_depth: 64,
            max_visits_per_type: 16,
        }
    }

    pub fn append(&mut self, chunk: &str) {
        self.buffer.push_str(chunk);
    }

    pub fn current_frame(&self) -> Option<&Frame> {
        self.stack.last()
    }

    pub fn current_frame_mut(&mut self) -> Option<&mut Frame> {
        self.stack.last_mut()
    }

    pub fn push_frame(&mut self, frame: Frame) -> bool {
        if self.stack.len() >= self.max_depth {
            return false; // Depth limit
        }
        self.stack.push(frame);
        true
    }

    pub fn pop_frame(&mut self) -> Option<Frame> {
        self.stack.pop()
    }

    /// Check and record recursion visit
    pub fn check_recursion(&mut self, type_id: TypeId) -> bool {
        let depth = self.stack.len();
        let key = (type_id, depth);
        let count = self.recursion_visits.entry(key).or_insert(0);
        *count += 1;
        *count <= self.max_visits_per_type
    }
}
```

---

## Component 6: Core Parser

**File:** `typed_stream/parser.rs`

```rust
use super::*;

pub struct TypedStreamParser {
    schema: SchemaIndex,
    lexer: Lexer,
    beam_k: usize,
}

pub struct ParseUpdate {
    pub progressed: bool,
    pub next_keys: Vec<KeyHint>,
}

#[derive(Debug, Clone)]
pub struct KeyHint {
    pub key: String,
    pub required: bool,
    pub discriminative: bool,  // True if only some union variants have this key
}

impl TypedStreamParser {
    pub fn new(root: &TypeIR, beam_k: usize) -> Self {
        let schema = SchemaIndex::build(root);
        TypedStreamParser {
            schema,
            lexer: Lexer::new(),
            beam_k,
        }
    }

    pub fn new_session(&self) -> ParseSession {
        let root_id = self.schema.root_id();
        let root_info = self.schema.get(root_id).unwrap();

        let root_frame = match &root_info.kind {
            TypeKind::Class { .. } => Frame::Object(ObjectFrame::new(
                ExpectedTypeSet::single(root_id)
            )),
            TypeKind::List { element } => Frame::Array(ArrayFrame::new(
                ExpectedTypeSet::single(*element)
            )),
            TypeKind::Union { variants, .. } => Frame::Value(ValueFrame {
                expected: ExpectedTypeSet::from_union(variants, self.beam_k)
            }),
            _ => Frame::Value(ValueFrame {
                expected: ExpectedTypeSet::single(root_id)
            }),
        };

        ParseSession::new(root_frame)
    }

    /// Process a chunk of input
    pub fn ingest(&self, session: &mut ParseSession, chunk: &str) -> anyhow::Result<ParseUpdate> {
        session.append(chunk);
        self.lexer.append(chunk);

        let tokens = self.lexer.drain_tokens();
        let mut progressed = false;

        for tok in tokens {
            if self.process_token(session, tok)? {
                progressed = true;
            }
        }

        Ok(ParseUpdate {
            progressed,
            next_keys: self.next_keys(session),
        })
    }

    fn process_token(&self, session: &mut ParseSession, tok: Token) -> anyhow::Result<bool> {
        let frame = session.current_frame_mut()
            .ok_or_else(|| anyhow::anyhow!("No frame"))?;

        match frame {
            Frame::Object(obj) => self.process_object_token(session, obj, tok),
            Frame::Array(arr) => self.process_array_token(session, arr, tok),
            Frame::Value(val) => self.process_value_token(session, val, tok),
        }
    }

    fn process_object_token(
        &self,
        session: &mut ParseSession,
        obj: &mut ObjectFrame,
        tok: Token,
    ) -> anyhow::Result<bool> {
        match tok {
            Token::RBrace => {
                obj.closed = true;
                // Pop frame and attach to parent
                self.complete_object(session)?;
                Ok(true)
            }

            Token::String { content, .. } if obj.pending_key.is_none() => {
                // This is a key
                let key = content.trim().to_string();
                obj.pending_key = Some(key.clone());
                obj.seen_keys.insert(key.clone());

                // Narrow union candidates by observed key
                obj.expected.observe_key(&self.schema, &key, true);
                Ok(true)
            }

            Token::Colon => {
                // After colon, next token is value
                // Push appropriate frame for field type
                if let Some(key) = &obj.pending_key {
                    self.push_field_frame(session, obj, key)?;
                }
                Ok(true)
            }

            Token::Comma => {
                // Between fields
                obj.pending_key = None;
                Ok(true)
            }

            _ if obj.pending_key.is_some() => {
                // This is a value token, process in value context
                // (delegates to nested frame)
                self.process_value_in_object(session, obj, tok)
            }

            _ => Ok(false), // Ignore unexpected tokens
        }
    }

    fn push_field_frame(
        &self,
        session: &mut ParseSession,
        obj: &ObjectFrame,
        key: &str,
    ) -> anyhow::Result<()> {
        // Find field type from best candidate
        let field_type_id = self.resolve_field_type(&obj.expected, key);

        match field_type_id {
            Some(type_id) => {
                let info = self.schema.get(type_id);
                let frame = self.frame_for_type(type_id, info);
                session.push_frame(frame);
            }
            None => {
                // Unknown key - push generic value frame
                session.push_frame(Frame::Value(ValueFrame {
                    expected: ExpectedTypeSet::single(0), // Fallback
                }));
            }
        }
        Ok(())
    }

    fn resolve_field_type(&self, expected: &ExpectedTypeSet, key: &str) -> Option<TypeId> {
        for type_id in expected.all_candidates() {
            if let Some(info) = self.schema.get(type_id) {
                if let TypeKind::Class { fields, alias_map, .. } = &info.kind {
                    // Check direct field
                    if let Some(field) = fields.get(key) {
                        return Some(field.type_id);
                    }
                    // Check alias
                    if let Some(canonical) = alias_map.get(&key.to_lowercase()) {
                        if let Some(field) = fields.get(canonical) {
                            return Some(field.type_id);
                        }
                    }
                }
            }
        }
        None
    }

    fn frame_for_type(&self, type_id: TypeId, info: Option<&TypeInfo>) -> Frame {
        match info.map(|i| &i.kind) {
            Some(TypeKind::Class { .. }) => {
                Frame::Object(ObjectFrame::new(ExpectedTypeSet::single(type_id)))
            }
            Some(TypeKind::List { element }) => {
                Frame::Array(ArrayFrame::new(ExpectedTypeSet::single(*element)))
            }
            Some(TypeKind::Union { variants, .. }) => {
                Frame::Value(ValueFrame {
                    expected: ExpectedTypeSet::from_union(variants, self.beam_k)
                })
            }
            _ => {
                Frame::Value(ValueFrame {
                    expected: ExpectedTypeSet::single(type_id)
                })
            }
        }
    }

    /// Compute next expected keys for current object frame
    pub fn next_keys(&self, session: &ParseSession) -> Vec<KeyHint> {
        let frame = match session.current_frame() {
            Some(Frame::Object(obj)) => obj,
            _ => return vec![],
        };

        let mut hints = Vec::new();
        let mut seen_keys = std::collections::HashSet::new();

        for type_id in frame.expected.all_candidates() {
            if let Some(info) = self.schema.get(type_id) {
                if let TypeKind::Class { fields, required, alias_map, .. } = &info.kind {
                    for (key, field) in fields {
                        if frame.seen_keys.contains(key) {
                            continue;
                        }
                        if seen_keys.contains(key) {
                            continue;
                        }
                        seen_keys.insert(key.clone());

                        hints.push(KeyHint {
                            key: key.clone(),
                            required: required.contains(key),
                            discriminative: self.is_discriminative(&frame.expected, key),
                        });
                    }
                }
            }
        }

        // Sort: required first, then discriminative, then alphabetical
        hints.sort_by(|a, b| {
            match (a.required, b.required) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => match (a.discriminative, b.discriminative) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.key.cmp(&b.key),
                }
            }
        });

        hints
    }

    fn is_discriminative(&self, expected: &ExpectedTypeSet, key: &str) -> bool {
        let mut has_key_count = 0;
        let mut total_count = 0;

        for type_id in expected.all_candidates() {
            total_count += 1;
            if let Some(info) = self.schema.get(type_id) {
                if let TypeKind::Class { fields, alias_map, .. } = &info.kind {
                    if fields.contains_key(key) || alias_map.contains_key(&key.to_lowercase()) {
                        has_key_count += 1;
                    }
                }
            }
        }

        // Discriminative if not all candidates have the key
        has_key_count > 0 && has_key_count < total_count
    }

    /// Finalize and produce BamlValueWithFlags
    pub fn finish(
        &self,
        session: &ParseSession,
        streaming: bool,
    ) -> anyhow::Result<BamlValueWithFlags> {
        // Close any unclosed frames
        // Convert ParsedValue tree to BamlValueWithFlags
        // Apply coercions and flags
        todo!("Implement finish")
    }
}
```

---

## Component 7: Segment Extraction

**File:** `typed_stream/extract.rs`

```rust
/// Candidate span with score
pub struct CandidateSpan {
    pub range: std::ops::Range<usize>,
    pub score: i32,
    pub kind: SpanKind,
}

#[derive(Debug, Clone, Copy)]
pub enum SpanKind {
    MarkdownFence,
    GreppedObject,
    GreppedArray,
    FullInput,
}

/// Extract candidate JSON-ish spans from raw input
pub fn extract_spans(input: &str, max_spans: usize) -> Vec<CandidateSpan> {
    let mut spans = Vec::new();

    // 1) Find markdown fenced blocks
    spans.extend(find_markdown_fences(input));

    // 2) Find {...} and [...] spans
    spans.extend(find_json_objects(input));
    spans.extend(find_json_arrays(input));

    // 3) Fallback: full input
    spans.push(CandidateSpan {
        range: 0..input.len(),
        score: -100, // Low priority
        kind: SpanKind::FullInput,
    });

    // Sort by score descending
    spans.sort_by_key(|s| std::cmp::Reverse(s.score));

    // Keep top N
    spans.truncate(max_spans);

    spans
}

fn find_markdown_fences(input: &str) -> Vec<CandidateSpan> {
    let mut spans = Vec::new();
    let fence_pattern = "```";

    let mut pos = 0;
    while let Some(start) = input[pos..].find(fence_pattern) {
        let abs_start = pos + start;

        // Find end of opening fence (after lang tag + newline)
        let content_start = input[abs_start + 3..]
            .find('\n')
            .map(|i| abs_start + 3 + i + 1)
            .unwrap_or(abs_start + 3);

        // Find closing fence
        if let Some(end_offset) = input[content_start..].find(fence_pattern) {
            let content_end = content_start + end_offset;

            spans.push(CandidateSpan {
                range: content_start..content_end,
                score: 100, // High priority for markdown fences
                kind: SpanKind::MarkdownFence,
            });

            pos = content_end + 3;
        } else {
            // Unclosed fence - treat as incomplete
            spans.push(CandidateSpan {
                range: content_start..input.len(),
                score: 50,
                kind: SpanKind::MarkdownFence,
            });
            break;
        }
    }

    spans
}

fn find_json_objects(input: &str) -> Vec<CandidateSpan> {
    let mut spans = Vec::new();

    // Simple heuristic: find { and attempt to find matching }
    for (i, c) in input.char_indices() {
        if c == '{' {
            if let Some(end) = find_matching_brace(input, i) {
                spans.push(CandidateSpan {
                    range: i..end + 1,
                    score: 50,
                    kind: SpanKind::GreppedObject,
                });
            }
        }
    }

    spans
}

fn find_matching_brace(input: &str, start: usize) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;

    for (i, c) in input[start..].char_indices() {
        if escape {
            escape = false;
            continue;
        }

        match c {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(start + i);
                }
            }
            _ => {}
        }
    }

    None
}

fn find_json_arrays(input: &str) -> Vec<CandidateSpan> {
    // Similar to find_json_objects but for [...]
    todo!()
}
```

---

## Component 8: Coercion + CompletionState → Flag Mapping

**File:** `typed_stream/coerce.rs`

Reuse logic from existing coercers but integrated into typed parsing. The key responsibility here is converting `ParsedValue` (with `CompletionState`) into `BamlValueWithFlags` (with `Flag::Incomplete`/`Flag::Pending`).

### CompletionState to Flag Mapping

```rust
use super::frames::{ParsedValue, ParsedValueKind, CompletionState};
use super::schema_index::{TypeId, TypeKind, PrimitiveKind, SchemaIndex};
use crate::deserializer::types::BamlValueWithFlags;
use crate::deserializer::deserialize_flags::{DeserializerConditions, Flag};

/// Convert CompletionState to appropriate Flag(s)
fn completion_to_flags(completion: CompletionState, conditions: &mut DeserializerConditions) {
    match completion {
        CompletionState::Complete => {
            // No flag needed - absence of Incomplete/Pending means complete
        }
        CompletionState::Incomplete => {
            conditions.add_flag(Flag::Incomplete);
        }
        CompletionState::Pending => {
            conditions.add_flag(Flag::Pending);
        }
    }
}

/// Coerce a ParsedValue to BamlValueWithFlags given expected type
pub fn coerce_value(
    schema: &SchemaIndex,
    type_id: TypeId,
    parsed: &ParsedValue,  // Now contains (value, completion) tuple
    streaming: bool,
) -> anyhow::Result<BamlValueWithFlags> {
    let info = schema.get(type_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown type ID"))?;

    // Start with conditions that include completion state
    let mut conditions = DeserializerConditions::new();
    completion_to_flags(parsed.completion, &mut conditions);

    let value = &parsed.value;

    match (&info.kind, value) {
        // Primitives
        (TypeKind::Primitive(PrimitiveKind::String), ParsedValueKind::String(s)) => {
            Ok(BamlValueWithFlags::String(ValueWithFlags {
                value: s.clone(),
                target: /* target TypeIR */,
                flags: conditions,
            }))
        }

        (TypeKind::Primitive(PrimitiveKind::Int), ParsedValueKind::String(s)) => {
            // Handle comma-separated ints: "12,111" -> 12111
            let cleaned = s.replace(',', "");
            let n: i64 = cleaned.parse()?;
            Ok(BamlValueWithFlags::Int(ValueWithFlags {
                value: n,
                target: /* target */,
                flags: conditions,
            }))
        }

        (TypeKind::Primitive(PrimitiveKind::Int), ParsedValueKind::Int(n)) => {
            Ok(BamlValueWithFlags::Int(ValueWithFlags {
                value: *n,
                target: /* target */,
                flags: conditions,
            }))
        }

        (TypeKind::Primitive(PrimitiveKind::Float), ParsedValueKind::String(s)) => {
            // Handle fractions: "1/5" -> 0.2
            if let Some((num, denom)) = s.split_once('/') {
                let n: f64 = num.trim().parse()?;
                let d: f64 = denom.trim().parse()?;
                return Ok(BamlValueWithFlags::Float(ValueWithFlags {
                    value: n / d,
                    target: /* target */,
                    flags: conditions,
                }));
            }
            let f: f64 = s.parse()?;
            Ok(BamlValueWithFlags::Float(ValueWithFlags {
                value: f,
                target: /* target */,
                flags: conditions,
            }))
        }

        (TypeKind::Primitive(PrimitiveKind::Bool), ParsedValueKind::String(s)) => {
            // Case-insensitive bool parsing
            let b = match s.to_lowercase().as_str() {
                "true" | "yes" | "1" => true,
                "false" | "no" | "0" => false,
                _ => anyhow::bail!("Cannot parse '{}' as bool", s),
            };
            Ok(BamlValueWithFlags::Bool(ValueWithFlags {
                value: b,
                target: /* target */,
                flags: conditions,
            }))
        }

        (TypeKind::Primitive(PrimitiveKind::Bool), ParsedValueKind::Bool(b)) => {
            Ok(BamlValueWithFlags::Bool(ValueWithFlags {
                value: *b,
                target: /* target */,
                flags: conditions,
            }))
        }

        // Enums
        (TypeKind::Enum { values, fuzzy_map, .. }, ParsedValueKind::String(s)) => {
            // Try exact match first
            if let Some(canonical) = values.get(s) {
                return Ok(BamlValueWithFlags::Enum(
                    info.kind.enum_name().unwrap().to_string(),
                    /* target */,
                    ValueWithFlags {
                        value: canonical.clone(),
                        target: /* target */,
                        flags: conditions,
                    },
                ));
            }

            // Try fuzzy match
            let normalized = normalize_enum_value(s);
            if let Some(canonical) = fuzzy_map.get(&normalized) {
                conditions.add_flag(Flag::StrippedNonAlphaNumeric(s.clone()));
                return Ok(BamlValueWithFlags::Enum(
                    info.kind.enum_name().unwrap().to_string(),
                    /* target */,
                    ValueWithFlags {
                        value: canonical.clone(),
                        target: /* target */,
                        flags: conditions,
                    },
                ));
            }

            anyhow::bail!("No matching enum value for '{}'", s)
        }

        // Objects/Classes - completion state derives from children
        (TypeKind::Class { name, fields, .. }, ParsedValueKind::Object { fields: parsed_fields, .. }) => {
            let mut result_fields = baml_types::BamlMap::new();

            for (key, field_info) in fields {
                let parsed = parsed_fields.iter()
                    .find(|(k, _)| k == key || field_info.aliases.contains(k));

                match parsed {
                    Some((_, val)) => {
                        let coerced = coerce_value(schema, field_info.type_id, val, streaming)?;
                        result_fields.insert(key.clone(), coerced);
                    }
                    None if streaming => {
                        // Missing field in streaming - create Pending placeholder
                        let mut field_conditions = DeserializerConditions::new();
                        field_conditions.add_flag(Flag::Pending);
                        field_conditions.add_flag(Flag::DefaultFromNoValue);

                        let default = default_for_type_with_flags(schema, field_info.type_id, field_conditions);
                        result_fields.insert(key.clone(), default);
                    }
                    None if !field_info.required => {
                        // Optional field missing - use null with flag
                        let mut field_conditions = DeserializerConditions::new();
                        field_conditions.add_flag(Flag::OptionalDefaultFromNoValue);

                        let null = BamlValueWithFlags::Null(/* target */, field_conditions);
                        result_fields.insert(key.clone(), null);
                    }
                    None => {
                        anyhow::bail!("Missing required field: {}", key);
                    }
                }
            }

            // Class-level completion: complete only if all fields are complete
            // The `conditions` variable already has the object-level completion from parsed.completion
            Ok(BamlValueWithFlags::Class(
                name.clone(),
                conditions,
                /* target */,
                result_fields,
            ))
        }

        // Lists - completion state derives from children
        (TypeKind::List { element }, ParsedValueKind::Array(items)) => {
            let coerced: Result<Vec<_>, _> = items.iter()
                .map(|item| coerce_value(schema, *element, item, streaming))
                .collect();

            Ok(BamlValueWithFlags::List(
                conditions,  // Array-level completion
                /* target */,
                coerced?,
            ))
        }

        // Null with Pending (field never seen)
        (_, ParsedValueKind::Null) if parsed.completion == CompletionState::Pending => {
            conditions.add_flag(Flag::DefaultFromNoValue);
            Ok(BamlValueWithFlags::Null(/* target */, conditions))
        }

        // Regular null
        (TypeKind::Primitive(PrimitiveKind::Null), ParsedValueKind::Null) => {
            Ok(BamlValueWithFlags::Null(/* target */, conditions))
        }

        _ => anyhow::bail!(
            "Cannot coerce {:?} to {:?}",
            value,
            info.kind
        ),
    }
}

fn normalize_enum_value(s: &str) -> String {
    // Lowercase, strip punctuation, normalize whitespace
    s.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn default_for_type(schema: &SchemaIndex, type_id: TypeId) -> BamlValueWithFlags {
    match schema.get(type_id).map(|i| &i.kind) {
        Some(TypeKind::Primitive(PrimitiveKind::Null)) => {
            BamlValueWithFlags::Null(/* target */, Default::default())
        }
        Some(TypeKind::List { .. }) => {
            BamlValueWithFlags::List(Default::default(), /* target */, vec![])
        }
        Some(TypeKind::Map { .. }) => {
            BamlValueWithFlags::Map(Default::default(), /* target */, Default::default())
        }
        _ => BamlValueWithFlags::Null(/* target */, Default::default()),
    }
}
```

---

## Component 9: Integration with Semantic Streaming

**File:** No new file - interfaces with existing `deserializer/semantic_streaming.rs`

The new parser produces `BamlValueWithFlags` with `Flag::Incomplete` and `Flag::Pending` markers. The existing `validate_streaming_state()` function consumes these to:

1. Derive `CompletionState` per node (from flags)
2. Enforce `@stream.done` requirements
3. Enforce `@stream.not_null` requirements
4. Attach `Completion` metadata for serialization

### The Contract: What Parser Must Provide

The parser output must satisfy these invariants for `semantic_streaming.rs` to work correctly:

```rust
// From semantic_streaming.rs:357-365
fn completion_state(flags: &[Flag]) -> CompletionState {
    if flags.iter().any(|f| matches!(f, Flag::Pending)) {
        CompletionState::Pending
    } else if flags.iter().any(|f| matches!(f, Flag::Incomplete)) {
        CompletionState::Incomplete
    } else {
        CompletionState::Complete
    }
}
```

**Parser must ensure:**

| Scenario | Flags to emit |
|----------|---------------|
| Field never seen in stream | `Flag::Pending` + `Flag::DefaultFromNoValue` |
| Value started but token incomplete | `Flag::Incomplete` |
| Object `{` seen but no `}` | `Flag::Incomplete` on object |
| Array `[` seen but no `]` | `Flag::Incomplete` on array |
| String `"` seen but no closing `"` | `Flag::Incomplete` on string |
| Number mid-digits at stream end | `Flag::Incomplete` on number (streaming mode only) |
| Fully closed value | No Incomplete/Pending flags |

### Streaming Validation Flow

```
TypedStreamParser::finish()
    → BamlValueWithFlags (with Flag::Incomplete / Flag::Pending)
        → validate_streaming_state()
            → Checks @stream.done: if required_done && !Complete → Error
            → Checks @stream.not_null: if needed_field is null → Error
            → Attaches Completion metadata to each node
                → BamlValueWithMeta<Completion>
```

### Example: Partial Object Streaming

Input stream so far: `{"name": "Ali`

```rust
// Parser produces:
BamlValueWithFlags::Class(
    "Person",
    DeserializerConditions { flags: [Flag::Incomplete] },  // Object not closed
    target,
    BamlMap {
        "name" => BamlValueWithFlags::String(ValueWithFlags {
            value: "Ali",
            flags: [Flag::Incomplete],  // String not closed
        }),
        "age" => BamlValueWithFlags::Null(
            target,
            DeserializerConditions { flags: [Flag::Pending, Flag::DefaultFromNoValue] }
        ),
    }
)

// After validate_streaming_state():
BamlValueWithMeta::Class(
    "Person",
    BamlMap {
        "name" => BamlValueWithMeta::String("Ali", Completion {
            state: CompletionState::Incomplete,
            display: false,  // from @stream.with_state
            required_done: false,
        }),
        "age" => BamlValueWithMeta::Null(Completion {
            state: CompletionState::Pending,
            display: false,
            required_done: false,
        }),
    },
    Completion {
        state: CompletionState::Incomplete,
        display: false,
        required_done: false,
    }
)
```

### Key Behaviors to Preserve

From `semantic_streaming.rs`:

1. **`required_done` calculation** (lines 294-355):
   - Primitives except String: always done
   - Enums, Literals: always done
   - Lists, Maps, Classes: not inherently done
   - Unions: depends on which variant matches
   - User-annotated `@stream.done`: force done

2. **Null filler for missing fields** (lines 264-270):
   - Fields in class definition but not in parsed value get null with `Pending`

3. **Field ordering preservation** (lines 209-219):
   - Output fields should match class definition order

---

## Integration: Public API

**File:** `typed_stream/mod.rs`

```rust
mod lexer;
mod schema_index;
mod expected_set;
mod frames;
mod session;
mod parser;
mod coerce;
mod extract;

pub use parser::{TypedStreamParser, ParseUpdate, KeyHint};
pub use session::ParseSession;

use baml_types::StreamingMode;
use internal_baml_core::ir::TypeIR;
use crate::deserializer::types::BamlValueWithFlags;

/// Main entry point: parse raw string to typed value
pub fn parse(
    root: &TypeIR,
    raw: &str,
    streaming: bool,
) -> anyhow::Result<BamlValueWithFlags> {
    let parser = TypedStreamParser::new(root, 8);
    let mut session = parser.new_session();

    // Extract candidate spans
    let spans = extract::extract_spans(raw, 2);

    // Try each span, pick best result
    let mut best_result: Option<(i32, BamlValueWithFlags)> = None;

    for span in spans {
        let segment = &raw[span.range.clone()];

        let result = parser.ingest(&mut session, segment);
        if result.is_err() { continue; }

        match parser.finish(&session, streaming) {
            Ok(value) => {
                let score = compute_quality_score(&value, &span);
                if best_result.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
                    best_result = Some((score, value));
                }
            }
            Err(_) => continue,
        }

        // Reset session for next span
        session = parser.new_session();
    }

    best_result
        .map(|(_, v)| v)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse any span"))
}

fn compute_quality_score(value: &BamlValueWithFlags, span: &extract::CandidateSpan) -> i32 {
    let mut score = span.score;

    // Boost for complete values
    if !value.conditions().flags.iter().any(|f| matches!(f, Flag::Incomplete | Flag::Pending)) {
        score += 10;
    }

    // Boost for fewer repair flags
    let repair_count = value.conditions().flags.iter()
        .filter(|f| matches!(f, Flag::ObjectFromFixedJson(_)))
        .count();
    score -= repair_count as i32 * 2;

    score
}
```

---

## Migration: Hooking into Existing Code

**File:** `engine/baml-lib/jsonish/src/lib.rs` (modify)

```rust
// Add feature flag
#[cfg(feature = "typed_stream_parser")]
pub mod typed_stream;

pub fn from_str(
    of: &OutputFormatContent,
    target: &TypeIR,
    raw_string: &str,
    raw_string_is_done: bool,
) -> Result<BamlValueWithFlags> {
    // Early exit for string target (preserve existing behavior)
    if matches!(target, TypeIR::Primitive(TypeValue::String, _)) {
        return Ok(BamlValueWithFlags::String(
            (raw_string.to_string(), target).into(),
        ));
    }

    // NEW: Use typed stream parser if enabled
    #[cfg(feature = "typed_stream_parser")]
    {
        let streaming = !raw_string_is_done;
        match typed_stream::parse(target, raw_string, streaming) {
            Ok(value) => return Ok(value),
            Err(e) => {
                log::debug!("Typed parser failed, falling back: {}", e);
                // Fall through to legacy parser
            }
        }
    }

    // EXISTING: Legacy parser path
    let value = jsonish::parse(
        raw_string,
        jsonish::ParseOptions::default(),
        raw_string_is_done,
    )?;

    let ctx = ParsingContext::new(of, if raw_string_is_done {
        baml_types::StreamingMode::NonStreaming
    } else {
        baml_types::StreamingMode::Streaming
    });

    target.coerce(&ctx, target, Some(&value))
}
```

---

## Testing Strategy

### 1. Unit Tests per Component

```rust
// typed_stream/tests/lexer_tests.rs
#[test]
fn test_lex_unquoted_key() {
    let mut lexer = Lexer::new();
    lexer.append("{ foo: 1 }");
    let tokens = lexer.drain_tokens();
    assert_eq!(tokens, vec![
        Token::LBrace,
        Token::String { content: "foo".into(), quote: QuoteStyle::Unquoted, complete: true },
        Token::Colon,
        Token::Number { raw: "1".into(), complete: true },
        Token::RBrace,
    ]);
}

#[test]
fn test_lex_triple_backtick() {
    let mut lexer = Lexer::new();
    lexer.append("```python\nprint('hi')\n```");
    let tokens = lexer.drain_tokens();
    assert!(matches!(tokens[0], Token::CodeBlock { lang: Some(ref l), .. } if l == "python"));
}
```

### 2. Parity Tests

```rust
// typed_stream/tests/parity_tests.rs
fn check_parity(schema: &str, input: &str) {
    let ir = parse_schema(schema);
    let target = ir.root_type();

    let old = crate::from_str_legacy(&ir, &target, input, true).unwrap();
    let new = crate::typed_stream::parse(&target, input, false).unwrap();

    assert_eq!(
        serde_json::to_value(&old).unwrap(),
        serde_json::to_value(&new).unwrap(),
    );
}

#[test]
fn parity_unquoted_keys() {
    check_parity(
        "class Foo { a int, b string }",
        "{ a: 1, b: hello }"
    );
}

#[test]
fn parity_union_by_keys() {
    check_parity(
        "class A { x int } class B { y int } type U = A | B",
        r#"{"x": 42}"#
    );
}
```

### 3. Curated Stress Tests

Import existing test cases from `jsonish/src/tests/*` and run against new parser.

---

## Rollout Plan

1. **Phase 1: Lexer + ExpectedTypeSet** (low risk)
   - Implement and test independently
   - No changes to production code path

2. **Phase 2: Parser skeleton**
   - Object/Array/Value frames
   - Basic next_keys()
   - Behind feature flag

3. **Phase 3: Coercion integration**
   - Port primitive/enum coercion logic
   - Run parity tests

4. **Phase 4: Segment extraction**
   - Markdown fence detection
   - Multi-span selection

5. **Phase 5: Production opt-in**
   - Feature flag in runtime
   - A/B testing with metrics

6. **Phase 6: Replace legacy**
   - Remove AnyOf-based parser
   - Clean up dead code

---

## Performance Invariants

- **O(input_length)** tokenization (single pass)
- **O(input_length * depth * K)** parsing where K is beam width
- **No exponential branching** - `ExpectedTypeSet` always bounded
- **Recursion-safe** - depth + visit limits enforced
- **Minimal string copies** - borrow from input buffer where possible

---

## String Handling: Avoiding Copies with `Cow<'a, str>`

### The Problem

Naive string handling copies data at every layer:
```
Input buffer → Token::String(String) → ParsedValue::String(String) → BamlValueWithFlags::String(String)
```

Each `String` is a heap allocation + memcpy. For large payloads with many fields, this adds up.

### Solution: `Cow<'a, str>` Throughout the Parse Phase

Use copy-on-write strings that borrow from the input buffer when possible:

```rust
use std::borrow::Cow;

/// Token with borrowed string content
#[derive(Debug, Clone)]
pub enum Token<'a> {
    LBrace,
    RBrace,
    // ...
    String {
        content: Cow<'a, str>,  // Borrows from input when no escapes
        quote: QuoteStyle,
        complete: bool,
    },
    Number {
        raw: Cow<'a, str>,      // Always borrows (numbers never need escaping)
        complete: bool,
    },
    // ...
}

/// Parsed value with borrowed strings
#[derive(Debug, Clone)]
pub struct ParsedValue<'a> {
    pub value: ParsedValueKind<'a>,
    pub completion: CompletionState,
}

#[derive(Debug, Clone)]
pub enum ParsedValueKind<'a> {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Cow<'a, str>),
    Object {
        type_id: TypeId,
        fields: Vec<(Cow<'a, str>, ParsedValue<'a>)>,  // Keys borrow too
    },
    Array(Vec<ParsedValue<'a>>),
}
```

### When to Borrow vs Copy

| Scenario | Borrow or Copy? | Reason |
|----------|-----------------|--------|
| Quoted string, no escapes | **Borrow** | `"hello"` → slice `[1..6]` of input |
| Quoted string with escapes | **Copy** | `"hel\"lo"` → must process escapes |
| Unquoted value | **Borrow** | `foo` → slice directly |
| Unquoted value, needs trim | **Copy** | `  foo  ` → allocate trimmed |
| Number | **Borrow** | Never needs modification |
| Object key | **Borrow** | Usually no escapes in keys |
| Enum coercion (normalized) | **Copy** | Lowercase/strip punctuation |

### Lexer Implementation

```rust
pub struct Lexer<'a> {
    input: &'a str,           // The full input buffer (borrowed)
    pos: usize,               // Current position
    mode: LexMode,
    pending_start: usize,     // Start of current token for borrowing
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer {
            input,
            pos: 0,
            mode: LexMode::Normal,
            pending_start: 0,
        }
    }

    /// Lex a quoted string, borrowing when possible
    fn lex_quoted_string(&mut self, quote: char) -> Option<Token<'a>> {
        let start = self.pos;
        self.advance(); // Skip opening quote

        let content_start = self.pos;
        let mut has_escapes = false;

        while let Some(c) = self.peek_char() {
            match c {
                '\\' => {
                    has_escapes = true;
                    self.advance();
                    self.advance(); // Skip escaped char
                }
                c if c == quote => {
                    let content_end = self.pos;
                    self.advance(); // Skip closing quote

                    let content = if has_escapes {
                        // Must allocate to process escapes
                        Cow::Owned(unescape_string(&self.input[content_start..content_end]))
                    } else {
                        // Zero-copy borrow from input
                        Cow::Borrowed(&self.input[content_start..content_end])
                    };

                    return Some(Token::String {
                        content,
                        quote: QuoteStyle::Double,
                        complete: true,
                    });
                }
                _ => self.advance(),
            }
        }

        // Incomplete string - still borrow what we have
        Some(Token::String {
            content: Cow::Borrowed(&self.input[content_start..self.pos]),
            quote: QuoteStyle::Double,
            complete: false,
        })
    }

    /// Lex a number - always borrows
    fn lex_number(&mut self) -> Option<Token<'a>> {
        let start = self.pos;

        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
                self.advance();
            } else {
                break;
            }
        }

        // Numbers never need escaping - always borrow
        Some(Token::Number {
            raw: Cow::Borrowed(&self.input[start..self.pos]),
            complete: true, // Complete if followed by delimiter
        })
    }
}

fn unescape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some(c) => {
                    result.push('\\');
                    result.push(c);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}
```

### Session Lifetime Management

The key challenge: `ParseSession` borrows from input, but `BamlValueWithFlags` must own its strings.

```rust
/// Session borrows from input
pub struct ParseSession<'a> {
    input: &'a str,
    stack: Vec<Frame<'a>>,
    // ...
}

/// Coercion converts borrowed → owned at the boundary
pub fn coerce_value<'a>(
    schema: &SchemaIndex,
    type_id: TypeId,
    parsed: &ParsedValue<'a>,  // Borrows from input
    streaming: bool,
) -> anyhow::Result<BamlValueWithFlags> {  // Owns strings
    match &parsed.value {
        ParsedValueKind::String(cow) => {
            // Convert Cow to owned String only at output boundary
            Ok(BamlValueWithFlags::String(ValueWithFlags {
                value: cow.clone().into_owned(),  // Single allocation here
                // ...
            }))
        }
        // ...
    }
}
```

### Streaming: Buffer Ownership

For streaming, we accumulate chunks. Two options:

**Option A: Concatenate chunks, borrow from concatenated buffer**
```rust
pub struct StreamingSession {
    buffer: String,  // Owns concatenated chunks
    lexer: Lexer<'static>,  // Actually borrows from buffer via unsafe
}

impl StreamingSession {
    pub fn append(&mut self, chunk: &str) {
        let old_len = self.buffer.len();
        self.buffer.push_str(chunk);
        // Lexer continues from old_len, borrowing from self.buffer
    }
}
```

**Option B: Use a rope or arena allocator**
```rust
use bumpalo::Bump;

pub struct StreamingSession<'arena> {
    arena: &'arena Bump,
    chunks: Vec<&'arena str>,  // Chunks allocated in arena
    lexer: Lexer<'arena>,
}

impl<'arena> StreamingSession<'arena> {
    pub fn append(&mut self, chunk: &str) {
        let owned = self.arena.alloc_str(chunk);
        self.chunks.push(owned);
    }
}
```

**Recommendation:** Start with Option A (simple `String` buffer). The copy on `push_str` is unavoidable with streaming, but we avoid copies *during* parsing. Optimize to Option B only if profiling shows it matters.

### SchemaIndex: Intern Common Strings

For schema strings (field names, enum values), use interning:

```rust
use string_interner::{StringInterner, DefaultSymbol};

pub struct SchemaIndex {
    interner: StringInterner,
    // Store symbols instead of Strings
    type_info: HashMap<TypeId, TypeInfo>,
}

pub struct FieldInfo {
    pub name: DefaultSymbol,      // Interned, not String
    pub type_id: TypeId,
    pub required: bool,
    pub aliases: Vec<DefaultSymbol>,
}

impl SchemaIndex {
    pub fn get_field_name(&self, sym: DefaultSymbol) -> &str {
        self.interner.resolve(sym).unwrap()
    }
}
```

This way, comparing field names is just comparing integer symbols - no string comparison needed.

### Summary: Where Copies Happen

| Location | Copy? | Notes |
|----------|-------|-------|
| Lexer tokens | **No** | Borrow from input |
| ParsedValue strings | **No** | Borrow from input |
| ParsedValue keys | **No** | Borrow from input |
| Escaped strings | **Yes** | Must process escapes |
| Enum normalization | **Yes** | Lowercase/strip |
| Final BamlValueWithFlags | **Yes** | Must own for output lifetime |
| Schema field names | **Interned** | Allocated once at schema build |

**Net effect:** Only one allocation per output string, at the `coerce_value` boundary. No intermediate copies during lexing/parsing.

---

## Summary: CompletionState Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           LEXER (lexer.rs)                                  │
│                                                                             │
│  Input chunks → Tokens with completion state                                │
│                                                                             │
│  Token::String { content: "hello", complete: false }  ← no closing quote    │
│  Token::Number { raw: "123", complete: true }         ← delimiter seen      │
│  Token::LBrace                                        ← always complete     │
└────────────────────────────────┬────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           PARSER (parser.rs)                                │
│                                                                             │
│  Tokens → ParsedValue with CompletionState                                  │
│                                                                             │
│  ParsedValue {                                                              │
│    value: ParsedValueKind::Object { fields: [...] },                        │
│    completion: CompletionState::Incomplete,  ← no `}` seen yet              │
│  }                                                                          │
│                                                                             │
│  Child completion propagates up:                                            │
│    - Object incomplete if any field incomplete OR no closing brace          │
│    - Array incomplete if any element incomplete OR no closing bracket       │
└────────────────────────────────┬────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           COERCION (coerce.rs)                              │
│                                                                             │
│  ParsedValue → BamlValueWithFlags                                           │
│                                                                             │
│  CompletionState → Flag mapping:                                            │
│    Complete   → (no flag)                                                   │
│    Incomplete → Flag::Incomplete                                            │
│    Pending    → Flag::Pending + Flag::DefaultFromNoValue                    │
│                                                                             │
│  BamlValueWithFlags::Class(                                                 │
│    "Person",                                                                │
│    DeserializerConditions { flags: [Flag::Incomplete] },                    │
│    ...                                                                      │
│  )                                                                          │
└────────────────────────────────┬────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    SEMANTIC STREAMING (semantic_streaming.rs)               │
│                    (existing code - unchanged)                              │
│                                                                             │
│  BamlValueWithFlags → BamlValueWithMeta<Completion>                         │
│                                                                             │
│  1. Derives CompletionState from flags                                      │
│  2. Enforces @stream.done requirements                                      │
│  3. Enforces @stream.not_null requirements                                  │
│  4. Attaches Completion { state, display, required_done } metadata          │
│                                                                             │
│  Output ready for serialization with streaming state                        │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Quick Reference: When to Use Each State

| State | Meaning | Example | Flag |
|-------|---------|---------|------|
| **Complete** | Value fully parsed and closed | `"hello"` with closing quote | (none) |
| **Incomplete** | Value present but syntactically open | `"hello` without closing quote | `Flag::Incomplete` |
| **Pending** | Value not yet started/seen | Missing field in streaming | `Flag::Pending` |

### Critical Test Cases for CompletionState

These existing tests validate completion state behavior:

1. `test_streaming.rs::test_number_list_state_incomplete` - Numbers in arrays during streaming
2. `test_streaming.rs::test_done_field_*` - `@stream.done` enforcement
3. `test_partials.rs::test_partial_choppy` - Partial object parsing
4. `test_partials.rs::test_partial_choppy_union` - Unions with incomplete values
