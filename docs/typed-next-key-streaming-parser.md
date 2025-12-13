# Typed Next-Key Streaming Parser (Design Doc)

## Summary
The current `PromptRenderer::parse()` path is effectively:

1) parse raw LLM text into a highly-ambiguous `jsonish::Value` (often containing nested `AnyOf`)
2) coerce that into the expected `TypeIR`

In streaming/partial cases (especially with unions + “fixing” + markdown), ambiguity compounds and can become exponentially slow.

This document proposes a **type-directed streaming parser** whose primary primitive is:

> Given the expected `TypeIR` and the current input buffer, determine which **key(s)** we should look for next (and extract/parse them incrementally).

The parser is designed to be **bounded-ambiguity** (beam search, not powerset), **union-aware**, and **cycle-safe** for recursive types, while still matching the behaviors covered by the existing deserializer tests under `engine/baml-lib/jsonish/src/tests/*` (unquoted keys/values, trailing commas, comments, markdown fences, triple-backtick strings, constraints, etc.).

**Scope note (important):** this design assumes we are **replacing the current coercer layer** in `engine/baml-lib/jsonish/src/deserializer/coercer/*` (and the `jsonish::Value -> BamlValueWithFlags` coercion step). Coercion becomes an integrated part of the new typed parser, not a second-stage transformation.

---

## Motivation / Requirements from the existing test suite
The deserializer test suite exercises far more than strict JSON:

### Input acquisition / extraction
- Raw text can include prefix/suffix prose; parser must find a JSON-ish “best blob”.
- Markdown fenced blocks (` ```json … ``` `) are common; sometimes multiple blocks.
- There are “code-like” payloads where *strings* contain triple backticks or even fence-like sequences.
- Multiple top-level objects/arrays can appear; some tests expect the “best” match.

### JSON-ish tolerance (“fixing” behaviors)
- Unquoted keys: `{ foo: 1 }`
- Unquoted values (including spaces/newlines): `{ key: some value with spaces }`
- Trailing commas in arrays/objects: `{"k":"v",}` / `[1,2,3,]`
- Single-quoted strings and mixed quoting
- Line and block comments inside JSON-ish (`//`, `/* … */`)
- Missing braces/brackets and partially-streamed payloads (fill with `null`/empty containers where appropriate)

### Type-directed coercion semantics (must be preserved)
- Primitive coercions (examples from tests):
  - ints with commas: `"12,111"` -> `12111`
  - floats from fractions: `"1/5"` -> `0.2`
  - booleans from `True/False` and embedded prose
  - string targets preserve raw content (including quotes) rather than “fixing” it
- Enums:
  - alias matching
  - punctuation stripping / case-insensitive matching heuristics
- Maps and lists:
  - map key parsing rules (stringified numeric/null keys in some cases)
  - partial lists/maps in streaming
- Classes:
  - alias fields (`@alias`)
  - field matching with whitespace-padded keys
  - missing required fields and partial completion behavior
- Constraints:
  - `@check` / `@assert` must be evaluated exactly as today
  - union selection can be influenced by constraint outcomes (scores)
- Unions:
  - unions of classes, primitives, and nested unions
  - “ignore numeric parse” when string is present in a union (tests in `test_unions.rs`)
  - choppy/partial union payloads (tests in `test_partials.rs`)
- Recursive types:
  - self-recursive and mutually recursive classes
  - recursive unions and missing quotes/brackets

**Key requirement:** the new parser must cover these behaviors without constructing exponentially-large intermediate `AnyOf` trees.

---

## Replacement scope (what changes vs what stays)
We are intentionally moving responsibilities around to eliminate the exponential search space.

### Replaced
- `jsonish::parse(...)` returning a branching `jsonish::Value` tree with nested `AnyOf`
- The coercer layer in `engine/baml-lib/jsonish/src/deserializer/coercer/*` (including the union “try everything” behavior)

### Kept (initially)
- `engine/baml-lib/jsonish/src/deserializer/semantic_streaming.rs` (validation + metadata attachment via `@stream.*`)
- Constraint evaluation semantics (checks/asserts) and scoring *as observed by tests* (even if internal implementation changes)
- `BamlValueWithFlags` as the main output type for parsing (so downstream integrations remain stable)

### Updated contracts
- The new parser must directly produce `BamlValueWithFlags` with the correct:
  - `field_type` / shape
  - coercion behavior (primitive/enum/literal rules)
  - defaulting behavior for missing fields (streaming placeholders)
  - flags that encode completion state (`Pending`/`Incomplete`) and key coercion choices sufficiently for downstream consumers

---

## Proposed approach: Type-directed “Next-Key” parsing

### Core idea
Instead of generating all plausible parse trees then coercing, we:

1) tokenize the buffer with a tolerant lexer (supporting the same “JSON-ish” surface as the current fixing parser)
2) maintain a **stack of typed frames** representing where we are in the expected `TypeIR`
3) in object/class contexts, recognize candidate keys and **narrow union candidates** based on those keys
4) incrementally parse only the parts of the input that correspond to expected fields/elements
5) expose `next_keys()` as a first-class operation.

### Why this avoids exponential behavior
Uncertainty is represented as a **bounded `ExpectedTypeSet`** (beam), not nested `AnyOf`:
- each frame carries at most `K` candidate type interpretations
- candidates are narrowed monotonically by evidence (structure, keys, successful value parses, constraints)
- parsing cost becomes ~`O(tokens * depth * K)` rather than exploding.

---

## Architecture

### Pipeline (recommended)
Keep a small pipeline, but remove branching:

1) **Segment selection / extraction** (cheap, deterministic; *top-N, late commitment*)
   - Identify candidate spans (not just one):
     - markdown fenced JSON blocks
     - “grepped” JSON objects/arrays in raw prose
     - fallback: whole buffer
   - Keep **top-N spans** (recommend `N=2` by default) ranked by a fast heuristic.
   - Run the typed parser on spans in priority order and select the **best post-typed score** (required keys satisfied, fewer repairs, higher union confidence), rather than committing to the first span up-front.

2) **Typed streaming parse on the chosen span**
   - Tolerant tokenization
   - Typed frame stack
   - Incremental updates as chunks arrive

3) **Integrated coercion + downstream validation**
   - Coercion is **integrated**: the typed parser emits `BamlValueWithFlags` directly (no `jsonish::Value` intermediate).
   - Semantic streaming validation remains post-parse (`validate_streaming_state`).
   - Constraints/scoring remain post-parse (same semantics as today), but union decision-making may incorporate lightweight pre-scores to keep behavior stable.

This preserves test behavior while attacking the combinatorial explosion at its source.

### Alternative: single-step only
We *could* fully replace `jsonish::parse + coerce` with one integrated parser, but it is higher risk because the current system embeds many heuristics in multiple layers (markdown parsing, fixing parser, coercers, flags, scoring).
The “pipeline without branching” approach is a safer first milestone.

---

## Data model

### `SchemaIndex`
Precompute from `TypeIR` (and IR context for aliases/constraints):
- for classes: allowed keys, required keys, alias keys, field types
- for enums: alias table + match config
- for unions: list of variants + a quick key->variant index for class-like variants
- stable `TypeId` for recursion detection (e.g. class name + params)
 - coercion hints:
   - numeric parsing preferences (comma-separated, fractions)
   - enum matching strategy config (punctuation stripping, case folding)
   - union preference rules (e.g. when `string` is in the union, avoid numeric-from-string unless forced)

### `ParseSession`
Incremental per-response state:
- `scan_offset`: last processed byte offset
- `buffer`: current concatenated content (or rope)
- `stack: Vec<Frame>`
- `partial_value`: partially built typed value graph
- `union_state`: candidate sets and their scores
- `visited_recursion`: `(TypeId, path_hash)` counters to prevent infinite descent
 - `field_states`: per-field bookkeeping to support `Pending` → observed transitions (see StreamState section)

### Frames
- `ObjectFrame { expected: ExpectedTypeSet, seen_keys, pending_key, … }`
- `ArrayFrame { expected_elem: ExpectedTypeSet, index, … }`
- `ValueFrame { expected: ExpectedTypeSet }` (for primitives/enums/literals)

### `ExpectedTypeSet` (bounded ambiguity)
Represents “we might be parsing as one of these types”:
- stored as `Vec<Candidate { type_id, score, violated }>` capped at `K`
- narrowed by:
  - structural cues (`{`/`[`/primitive tokens)
  - observed keys (**soft scoring** during streaming; delayed pruning via beam cutoff or object-close)
  - successful parse of a field value (boost candidate that predicted it)
  - constraint outcomes (post-parse; used to choose among remaining candidates)

---

## Tokenization (tolerant)
We need a streaming lexer that matches the “fixing parser” surface:
- braces/brackets/colons/commas
- quoted strings: `"…"`, `'…'`
- triple-quoted strings (`"""…"""`) if required by current behavior
- triple-backtick strings (```` ```lang\n…``` ````) as a value form (used heavily in tests)
- comments: `//…` and `/*…*/`
- unquoted identifiers and unquoted “values with spaces/newlines” (bounded heuristics)
- ability to resume across chunk boundaries (incomplete string/comment)

Practical implementation path:
- Extract the tokenization portion of the existing fixing parser into a reusable streaming lexer,
  but **do not** build a parse tree of `AnyOf` options.

---

## Parsing rules (type-directed)

### Object/class frames
Recognize key/value patterns:
- key tokens can be:
  - quoted string
  - unquoted identifier (tests require this)
  - whitespace-padded keys that should match after trimming
- accept a key iff it matches at least one candidate’s allowed keys (including aliases)
- when key is accepted:
  - mark as seen
  - narrow `ExpectedTypeSet` to candidates that contain the key (or keep top-K)
  - push a new frame for the field’s expected type

Missing/partial values:
- if object closes or stream ends while a required field is absent, emit `null`/empty container as current logic does in streaming tests.

### Arrays/lists
- track element index and parse elements per expected element type
- tolerate trailing commas and partial last element

### Unions
Unions are modeled by `ExpectedTypeSet` with beam width `K`.

Narrowing signals:
- structural: if we’re inside `{`, drop non-object variants
- keys: during streaming, **prefer scoring over hard elimination**; penalize variants lacking the key and only prune via beam cutoff or when the object scope closes
- primitive parse preference rules:
  - preserve current behavior such as “if string is in the union, don’t eagerly parse numbers from numeric-looking strings”
  - this should be implemented by delegating primitive parsing to the existing coercers, but with union-aware “try order”.

Selection:
- when enough evidence exists, pick the highest score variant
- otherwise retain ambiguity in session state but emit best-effort partial.

---

## Coercion integration (replacing `deserializer/coercer/*`)
Since we’re replacing the coercer layer, the new parser must include a “typed coercion” subsystem with the same outward behaviors as the current coercers.

### Inputs to coercion
- Expected type: `TypeIR` (or a bounded `ExpectedTypeSet` in unions)
- Observed token stream at the current value position
- Streaming mode: `Streaming` vs `NonStreaming`

### Outputs of coercion
- A `BamlValueWithFlags` node that:
  - has the correct `target` type (`field_type()` matches expected after finalize/to_streaming_type)
  - includes flags describing:
    - completion (`Pending`, `Incomplete`)
    - notable coercions (e.g. object→primitive, float→int) as needed for scoring/debuggability

### Required coercion behaviors (non-exhaustive)
These correspond to existing test expectations:
- **Numeric**:
  - int from comma-separated: `"12,111"` → `12111`
  - float from fraction-like: `"1/5"` → `0.2`
  - trim trailing commas in numeric strings
- **Bool**:
  - case-insensitive `true/false`, including embedded prose heuristics where current behavior does that
- **Enum**:
  - alias matching
  - punctuation stripping / case folding heuristics
- **String**:
  - if expected is `string`, preserve raw content (including quotes) and avoid aggressive fixing
  - triple-backtick value syntax must yield string contents (dedented) per `test_code.rs`
- **Class/object**:
  - tolerate unquoted keys/values, comments, trailing commas, missing braces (best-effort)
  - apply `@alias` key mapping
  - fill missing optional fields with `null + Pending`
  - fill missing required fields per existing defaulting rules (often `null + Pending` in streaming)
- **Union**:
  - do not “try all”; use beam selection + soft evidence
  - preserve special-case preference rules (e.g., “string in union”)

### Constraints and scoring interaction
Some tests assert exact scores and union decisions influenced by checks. To preserve behavior:
- After producing a candidate `BamlValueWithFlags`, run constraint evaluation and compute a score.
- For unions, allow a bounded set of candidates (beam) to be scored and select the best according to the legacy scoring semantics.
  - This retains the spirit of “pick best post-coercion” without enumerating exponentially many parses.

---

## Policy details (risk mitigations)

### Segment selection: top-N + post-typed scoring
To preserve the current system’s “try multiple options and pick best” strength, but bounded:
- Extract candidate spans and keep **top N** (start with `N=2`, configurable).
- For each span, run typed parse and compute a **quality score**:
  - required keys satisfied (weighted)
  - number of recognized keys
  - number/severity of repairs applied (comments/trailing commas/unquoted values are cheap; structural guesswork is costly)
  - completion state (in non-streaming: complete preferred)
  - union confidence (score gap between #1 and #2 candidates)
  - constraint outcomes (tie-breaker if constraints are applied post-parse)
- Choose the best span by this score; keep the runner-up as fallback if the best degrades as more stream arrives.

#### Proposed quality score components (initial weights)
These weights are intentionally coarse; the goal is determinism and “obviously right” choices.

| Component | Meaning | Weight (suggested) |
|---|---|---:|
| Required fields satisfied | Count of required class fields successfully parsed | +10 each |
| Optional fields satisfied | Count of optional fields successfully parsed | +2 each |
| Recognized keys | Keys that match schema (including aliases) | +3 each |
| Unknown keys | Keys that don’t match schema (class contexts) | −2 each |
| Structural mismatches | E.g. expected object but only primitives seen, unbalanced braces at end (non-streaming) | −20 each |
| Repairs used | Comments/trailing commas/unquoted keys are “cheap”; missing braces/brackets are “expensive” | cheap: −1, expensive: −8 |
| Completion state | Non-streaming: prefer Complete | complete: +10, incomplete: −5 |
| Union confidence | `score(top1) - score(top2)` | +min(gap, 20) |
| Constraint outcome (tie-break) | If constraints are evaluated post-parse | pass: +5, fail: −5 |

Tie-breakers (in order):
1) more required fields satisfied
2) higher completion state (Complete > Incomplete > Pending)
3) higher union confidence gap
4) fewer expensive repairs
5) earlier span start (prefer earlier in text for determinism)

### Beam width (K): defaults + adaptive collapse
Beam width is load-bearing. Proposed defaults:
- root union contexts: `K=8`
- nested unions: `K=4`
- deep recursion contexts: `K=2` unless confidence is low

Adaptive behavior:
- if `score(top1) - score(top2) >= GAP` (e.g. 20), collapse to `K=1` for that frame
- if parsing stalls (no progress for M tokens) and ambiguity remains, temporarily raise `K` by +2 (capped) to avoid locking in too early

### Debug/scoring flags parity (FirstMatch / UnionMatch)
The legacy path records exhaustive attempts in flags. A replacement parser will not enumerate all alternatives, so we must either:
- **Synthesize equivalent flags from beam history** (recommended): record candidate sets at each narrowing step and emit a summary “beam trace” as flags; or
- Audit all flag consumers and make them robust to non-exhaustive alternatives.

### Recursion: frontier completion
For deep recursive streams (e.g. `Tree = Leaf | Node` with `Node { left: Tree, right: Tree }`), we must avoid infinite descent:
- track `(TypeId, path_hash)` visit counts
- enforce `max_depth` / `max_visits_per_path`
- in streaming mode, when limits are hit, **stop descending and emit an optimistic frontier** (null/empty with `Incomplete`) while continuing to scan for keys at higher frames

---

## Streaming state (“StreamState”) tracking requirements
The current system derives per-node streaming state from `Flag::Pending` / `Flag::Incomplete` on `BamlValueWithFlags`, then attaches `Completion { state, display, required_done }` metadata in `validate_streaming_state`.

If we replace the parse step, we must preserve the same observable behavior:

### State model
- **Pending**: the node’s value is not yet observed in the stream (e.g. missing field filled by default). Represented by `Flag::Pending`.
- **Incomplete**: the node’s value is present but syntactically incomplete (e.g. unterminated string/object/list). Represented by `Flag::Incomplete`.
- **Complete**: the node’s value is fully observed and closed (no `Pending`/`Incomplete` flag).

### Parser responsibilities
- Emit `BamlValueWithFlags` where:
  - missing required/optional fields are populated with the *same* defaulting rules as today and marked with `Flag::Pending`
  - partially-read containers/strings are marked with `Flag::Incomplete`
  - when a previously-pending field becomes observed later, replace the placeholder and remove `Pending`
- Maintain incremental bookkeeping so we can update state as chunks arrive:
  - per object/class frame: seen keys + whether current value token is closed
  - per list frame: whether the last element is closed
  - per string-like token: whether it is terminated

### Interaction with `@stream.*`
We should continue to rely on the existing semantic streaming layer for:
- `@stream.done` / `@@stream.done` required-done validation
- `@stream.not_null` behavior (including “effective null” logic for needed fields)
- `@stream.with_state` display flags

The typed parser must therefore preserve the same flags and field-fill behavior that semantic streaming expects.

### Recursive types
Must handle:
- self-recursion (`Foo { pointer Foo? }`)
- mutual recursion (`Foo { b Bar | int }`, `Bar { f Foo | int }`)
- recursive unions with missing quotes/brackets

Cycle safety:
- stable `TypeId` per class
- enforce `max_depth` and `max_visits_per_path` for repeated `TypeId`
- still allow finding keys at the current level; just avoid infinite “expand expected keys” recursion.

---

## `next_keys()` behavior
At any time, for the current object/class frame:

1) Gather keys from remaining candidates (including aliases)
2) Remove keys already seen
3) Rank:
   - required keys first
   - keys that are unique to a subset of candidates (high discriminative power)
   - keys with simple value types (primitive before nested) to make progress
4) Return `Vec<KeyHint { key, score, reason }>`

This is the primitive we can use to drive incremental parsing and to reduce work.

---

## Compatibility with existing output + streaming semantics
To keep test parity:
- The typed parser should output `BamlValueWithFlags` (or something convertible) with:
  - completion state flags (`Incomplete`)
  - existing coercion flags (`JsonToString`, `ObjectToString`, etc.) where meaningful
- After producing the value, run the same post-processing:
  - constraint evaluation (`@check`, `@assert`)
  - semantic streaming validation (`@stream.*`)
  - scoring

This ensures we match behaviors tested in:
- `test_constraints.rs` (including union selection influenced by checks)
- `test_streaming.rs` (state/done/not_null)

---

## Rollout plan (safe + incremental)
1) Implement the new lexer + typed frame parser behind a feature flag.
2) First use-case: `allow_partials=true` streaming parsing for class/map targets (where blowups happen).
3) Keep existing `jsonish::from_str` as a fallback if:
   - no progress after N tokens
   - candidate span extraction fails
4) Expand coverage:
   - unions-of-classes
   - unions including primitives
   - recursive unions
5) Once stable and performance is proven, consider replacing more of the legacy pipeline.

---

## Open questions
- Best place to encode union-specific primitive preference rules (e.g., string-vs-number parsing).
- How to expose `next_keys()` to callers: internal only, or surfaced for debugging/telemetry?

---

## Test-suite-driven compatibility checklist
This section maps the existing `engine/baml-lib/jsonish/src/tests/*` surface area to responsibilities in the new parser. The intent is: if we can satisfy these categories, we can replace the current parse step end-to-end.

### `tests/mod.rs` (top-level smoke + “string target” semantics)
- **String target must be identity**: if the expected type is `string`, return the raw string (including quotes) rather than attempting repairs.
- **Prefix/suffix prose**: tolerate “The answer is … { … } …” and still extract the blob for non-string targets.
- **Aliases**: class fields with `@alias(...)` must be accepted as keys and mapped to canonical field names.
- **Leading junk**: tolerate leading close braces/brackets before the “real” object.

### `tests/test_basics.rs` (JSON-ish repairs + primitive coercions)
- **Unquoted keys**: `{ foo: 1 }` and whitespace-padded keys must match class fields.
- **Unquoted values**: values may be bare identifiers or multi-word segments (with spaces/newlines).
- **Trailing commas**: objects and arrays with trailing commas must parse.
- **Comments**: `//` and `/* */` inside JSON-ish must be ignored.
- **Markdown fenced JSON**: ` ```json ... ``` ` should be parsed as a preferred segment.
- **Primitive coercions**:
  - ints with commas: `"12,111"` -> `12111`
  - floats from fractions: `"1/5"` -> `0.2`
  - booleans from `True/False`, including when embedded in prose
  - null handling: distinguish `"null"` (string) vs `null` (null) depending on expected type.

### `tests/test_code.rs` (triple-backtick “strings” inside JSON-ish)
- **Triple-backtick string literal syntax** in JSON-ish: `"code": ``` ... ```` should produce a string value containing the codeblock contents (dedented).
- **Strings that contain triple backticks** must remain intact (no premature fence termination).
- **Code-like content containing JSON terminators** (e.g. `}}}]]))`) must not confuse parsing of the surrounding object.

### `tests/test_lists.rs` / `tests/test_maps.rs`
- Lists: tolerate partial last element, trailing commas, nested lists, list-of-classes.
- Maps: tolerate partial entries in streaming; define behavior for map keys that look numeric/null (tests expect stringified keys in some cases).

### `tests/test_enum.rs`
- Enum parsing is not strict: match by aliases and heuristics (case-insensitive, punctuation stripping).
- Fenced blocks can wrap enum values (` ```json\nnull\n``` ` patterns exist).

### `tests/test_literals.rs`
- Literal coercion includes heuristic matching and “object with single key” extraction in some cases.
- Ambiguity notes exist in the test file; replacement must preserve current tie-breaking behavior (even if imperfect).

### `tests/test_unions.rs` (union selection without exponential blowups)
- Union of classes: choose correct variant by observed keys.
- Union of primitive/complex combos: preserve special-cased behaviors (e.g., when `string` is in the union, avoid eagerly parsing numeric-looking strings as numbers).
- Union inside arrays and nested class fields.
- Fenced JSON blocks as the union payload source.

### `tests/test_partials.rs` and `tests/test_streaming.rs` (streaming/partial semantics)
- Partial parsing must produce a partial value consistent with `serialize_partial()` expectations.
- Must tolerate truncated buffers mid-token (string/object/array).
- Must avoid `AnyOf` leaking internal representations into string fields.
- Streaming metadata semantics (`@stream.*` validation) must remain post-parse and compatible.

### `tests/test_class.rs` (classes + recursion + missing quotes/brackets)
- Parse objects with missing braces/quotes and fill missing required fields as current behavior does.
- Recursive and mutually recursive classes and unions must not infinite-loop.
- Single-line dense objects and recursive unions must be handled (`rec_one: { rec_one: 1, ... }`).

### `tests/test_constraints.rs` (checks/asserts + scoring)
- Parser must feed the same constraint engine as today.
- Union decisions can be influenced by constraint outcomes/scoring (some tests assert exact scores).

### `tests/test_international.rs` (unicode)
- Keys/values/enums must handle unicode and normalization safely (no lossy transformations).

---

## Proposed module layout (new parser)
This assumes we fully replace the current parse step (not reusing jsonish parsing), but we can still reuse downstream conversion/constraints/streaming validation if desired.

Recommended new crate module (either inside `jsonish` or adjacent):

- `typed_stream/`
  - `mod.rs` (public API)
  - `schema_index.rs` (TypeIR → indexed schema: keys, aliases, union maps, recursion ids)
  - `lexer.rs` (streaming tolerant lexer: JSON-ish + comments + triple-backtick strings)
  - `session.rs` (append-only buffer + incremental offsets + recursion guards)
  - `parser.rs` (typed frame stack, union beam narrowing, next-keys)
  - `coerce.rs` (primitive/enums/literals coercion semantics compatible with tests)
  - `extract.rs` (segment selection: fenced blocks / grepped objects / fallback spans)
  - `diagnostics.rs` (optional: debug traces, spans, progress counters)

Public API sketch:

```rust
pub struct TypedStreamParser {
  schema: SchemaIndex,
  beam_k: usize,
  max_spans: usize,
}

pub struct ParseSession {
  state: ParserState,
  buffer: String,
  scan_offset: usize,
}

pub struct ParseUpdate {
  pub progressed: bool,
  pub next_keys: Vec<KeyHint>,
}

impl TypedStreamParser {
  pub fn new(root: TypeIR, ir: &IntermediateRepr, beam_k: usize) -> Self;
  pub fn new_session(&self) -> ParseSession;
  pub fn ingest(&self, session: &mut ParseSession, chunk: &str) -> anyhow::Result<ParseUpdate>;
  pub fn next_keys(&self, session: &ParseSession) -> Vec<KeyHint>;
  pub fn finish(&self, session: &ParseSession, mode: StreamingMode) -> anyhow::Result<BamlValueWithFlags>;
}
```

---

## Code skeleton snippets (core pieces)

### 1) Streaming tolerant lexer (append-only)
The lexer must preserve state across chunks for incomplete strings/comments/backticks.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
  LBrace, RBrace, LBracket, RBracket, Colon, Comma,
  Str(String),        // "..." or '...'
  Ident(String),      // unquoted key/value tokens
  Num(String),
  True, False, Null,
  TripleBacktick { lang: Option<String>, body: String, closed: bool },
}

#[derive(Default)]
pub struct LexState {
  mode: Mode,
  carry: String,
}

#[derive(Default)]
enum Mode {
  #[default] Normal,
  InString { quote: char, escaped: bool },
  InLineComment,
  InBlockComment { star: bool },
  InTripleBacktick { ticks: u8, lang: Option<String>, saw_newline: bool },
}

pub fn lex_incremental(state: &mut LexState, appended: &str, out: &mut Vec<Tok>) {
  // 1) append to carry
  // 2) scan chars; emit tokens
  // 3) leave unclosed constructs in carry + Mode for next chunk
}
```

Implementation note: the lexer is where we implement tolerance for `//`, `/* */`, single quotes, and triple-backtick strings, because the tests rely on those constructs appearing inside otherwise-JSON-ish objects.

### 2) Typed frame stack with bounded union beam

```rust
type TypeId = u32;

#[derive(Clone)]
struct Candidate { ty: TypeId, score: i32, dead: bool }

#[derive(Clone)]
struct ExpectedSet { cands: Vec<Candidate> } // always capped to K

enum Frame {
  Object { expected: ExpectedSet, seen: std::collections::HashSet<String>, pending_key: Option<String> },
  Array  { expected_elem: ExpectedSet, index: usize },
  Value  { expected: ExpectedSet },
}

pub struct ParserState {
  frames: Vec<Frame>,
  // partial output graph; could be a custom value builder
  // recursion guards:
  depth: usize,
  type_visits: std::collections::HashMap<(TypeId, u64), u32>,
}
```

Key operations:
- `narrow_by_structure()` when seeing `{` or `[` to drop impossible union candidates
- `narrow_by_key()` when seeing a key (drop class variants that don’t contain it)
- `parse_value_for_expected()` which:
  - for primitives/enums/literals applies coercion rules (matching test expectations)
  - for objects/arrays pushes frames

### 3) Segment extraction (robustness for prose + markdown)
Before tokenization, select the best span:

```rust
pub fn select_candidate_spans(input: &str, max_spans: usize) -> Vec<std::ops::Range<usize>> {
  // Priority order:
  // 1) fenced ```json blocks (first complete / best-scoring)
  // 2) best-looking {...} or [...] span (balanced-ish, contains expected keys)
  // 3) fallback: full input
  vec![0..input.len()]
}
```

This keeps the “try multiple spans” strength of the legacy system, but bounded (`max_spans`).

### 4) Integration point: `PromptRenderer::parse()`
Pseudo-structure:

```rust
pub fn parse(&self, ir: &IntermediateRepr, ctx: &RuntimeContext, raw: &str, allow_partials: bool)
  -> Result<ResponseBamlValue>
{
  let (def, target) = if allow_partials { ... } else { ... };
  let parser = TypedStreamParser::new(target.clone(), ir, /*beam_k*/ 8);
  let mut session = parser.new_session();
  parser.ingest(&mut session, raw)?;
  let typed = parser.finish(&session, if allow_partials { Streaming } else { NonStreaming })?;
  // Then reuse existing: parsed_value_to_response / constraint evaluation / semantic streaming.
  parsed_value_to_response(&ScopedIr::new(ir, ctx), typed, mode)
}
```

If we want to preserve “string target returns raw text” semantics, this should be an early exit before the typed parser is invoked.

---

## Performance invariants (what we must enforce)
- Union ambiguity is always bounded: `ExpectedSet` never grows past `K`.
- Each byte of input is tokenized at most once per session (append-only; track offsets).
- Recursion is guarded by `max_depth` and `max_visits_per_path`.
- Segment extraction runs in linear time and never enumerates parse trees.

---

## Implementation plan (recommended order)

### 1) Build the tolerant incremental lexer first
Deliverable:
- `typed_stream/lexer.rs` incremental tokens + spans + resume across chunk boundaries
- Unit tests focused on the “JSON-ish” surface (comments, trailing commas, unquoted keys/values, triple-backtick strings)

### 2) Build `ExpectedTypeSet` as a standalone abstraction
Deliverable:
- `ExpectedTypeSet` with:
  - `narrow_by_structure`
  - `observe_key` (soft scoring + delayed pruning)
  - `observe_value_parse(success/fail)`
  - `top_k(adaptive_policy)`
- Unit tests for union behaviors (including “string in union” preference rules).

### 3) Typed frame parser + `next_keys()`
Deliverable:
- object/class + list/map parsing with partial completion semantics
- `next_keys()` ranking (required keys first; discriminative keys higher)

### 4) Multi-span selection with late commitment
Deliverable:
- `extract.rs` returns top-N spans
- parse each span and select by post-typed quality score

### 5) Flags parity and consumers
Deliverable:
- beam history (“trace”) and synthesized `FirstMatch`/`UnionMatch` equivalents
- audit of flag consumers to ensure non-exhaustive traces are acceptable

### 6) Parity harness: new parser vs old
Add a compatibility harness to prevent regressions while migrating:

```rust
#[test]
fn new_parser_matches_old_for_curated_cases() {
  // For each curated (schema, target_type, raw_input):
  // old: jsonish::from_str(...)
  // new: typed_stream::parse(...)
  // assert_json_eq!(json(old.into()), json(new.into()))
}
```

Start with curated representatives from each test file, then expand coverage until confidence is high.

---

## Curated parity corpus (initial list)
These are hand-picked “stress” cases to validate behavioral parity and performance early. Each item corresponds to an existing test in `engine/baml-lib/jsonish/src/tests/*`.

### Segment selection + prose + multiple blobs
- `engine/baml-lib/jsonish/src/tests/mod.rs`: `test_string_from_string24` (prose + JSON-ish object with errors)
- `engine/baml-lib/jsonish/src/tests/mod.rs`: `test_leading_close_braces` (leading junk before object)

### Markdown fences + nested tricky content
- `engine/baml-lib/jsonish/src/tests/test_code.rs`: `triple_backticks_in_json_fenced_codeblock`
- `engine/baml-lib/jsonish/src/tests/test_code.rs`: `string_preserves_triple_backticks`
- `engine/baml-lib/jsonish/src/tests/test_code.rs`: `triple_backticks_contains_json_terminators`

### JSON-ish repairs (unquoted keys/values, comments, trailing commas)
- `engine/baml-lib/jsonish/src/tests/test_basics.rs`: `test_unquoted_keys`
- `engine/baml-lib/jsonish/src/tests/test_basics.rs`: `test_trailing_comma_object`
- `engine/baml-lib/jsonish/src/tests/test_basics.rs`: `test_trailing_comma_array_2`
- `engine/baml-lib/jsonish/src/tests/test_basics.rs`: `test_json_with_unquoted_values_with_spaces`
- `engine/baml-lib/jsonish/src/tests/test_basics.rs`: `test_json_with_unquoted_values_with_spaces_and_new_lines`

### Unions (class unions + primitive preference)
- `engine/baml-lib/jsonish/src/tests/test_unions.rs`: `test_union` (union of classes by keys)
- `engine/baml-lib/jsonish/src/tests/test_unions.rs`: `test_union2` (fenced payload + nested union in field)
- `engine/baml-lib/jsonish/src/tests/test_unions.rs`: `test_ignore_float_in_string_if_string_in_union`
- `engine/baml-lib/jsonish/src/tests/test_unions.rs`: `test_ignore_int_if_string_in_union`

### Partials / choppy streaming
- `engine/baml-lib/jsonish/src/tests/test_partials.rs`: `test_partial_choppy`
- `engine/baml-lib/jsonish/src/tests/test_partials.rs`: `test_partial_choppy_union`

### Recursive + recursive unions + missing quotes/brackets
- `engine/baml-lib/jsonish/src/tests/test_class.rs`: `test_recursive_type`
- `engine/baml-lib/jsonish/src/tests/test_class.rs`: `test_recursive_type_missing_brackets_and_quotes`
- `engine/baml-lib/jsonish/src/tests/test_class.rs`: `test_mutually_recursive_with_union`
- `engine/baml-lib/jsonish/src/tests/test_class.rs`: `test_recursive_union_on_multiple_fields_single_line_without_quotes_complex`

### Constraints-driven union scoring
- `engine/baml-lib/jsonish/src/tests/test_constraints.rs`: `test_union_decision_from_check`

### Streaming state semantics
- `engine/baml-lib/jsonish/src/tests/test_streaming.rs`: `test_number_list_state_incomplete`
- `engine/baml-lib/jsonish/src/tests/test_streaming.rs`: `test_done_field_0`
- `engine/baml-lib/jsonish/src/tests/test_streaming.rs`: `test_done_field_1`
- `engine/baml-lib/jsonish/src/tests/test_streaming.rs`: `test_union_not_null_with_null_value`
- `engine/baml-lib/jsonish/src/tests/test_streaming.rs`: `test_streaming_anyof_with_markdown_partial`
