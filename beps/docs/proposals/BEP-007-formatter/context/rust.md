# rustfmt Design

## Summary

rustfmt is Rust's official code formatter that takes an AST-based approach rather than using algebraic document combinators. It applies a collection of hand-written formatting rules directly to AST nodes, which allows it to perform semantic-preserving transformations like import reordering. While this approach is more verbose and harder to maintain than algebraic formatters, it provides fine-grained control over specific language constructs and enables syntax modifications beyond pure whitespace changes.

## Key Design Decisions

### 1. Operate Directly on the AST

rustfmt does not translate code into a Wadler-style document IR. Instead, it operates directly on Rust's Abstract Syntax Tree (AST), walking through each node and applying formatting rules. This approach is crucial for semantics-preserving code modifications like:

- Reordering imports alphabetically
- Normalizing use statements (e.g., `use std::{io, fs}` vs separate imports)
- Simplifying paths and removing redundant parentheses

The formatter uses a collection of pre-defined rules that print AST nodes in various ways based on heuristics such as maximum line lengths, nesting depth, and the type of construct being formatted.

### 2. Variable Maximum Line Length

Different structures in code have [differing maximum line lengths](https://rust-lang.github.io/rustfmt/?version=v1.8.0&search=#array_width) that are independently configurable. For example:

- `max_width`: 100 (default for most constructs)
- `array_width`: 60 (default for array literals)
- `chain_width`: 60 (default for method chains)
- `fn_call_width`: 60 (default for function call arguments)

These are often configurable through `rustfmt.toml`, but most users accept the defaults. This allows different constructs to wrap at different points, producing more readable code. For example, arrays can wrap at a narrower width than the global maximum:

```rust
let v = vec![
        1, 2, 3, 4, 5, 6, 1, 2, 3, 4, 5, 6, 1, 2, 3, 4, 5, 6, 1, 2, 3, 4, 5, 6, 1, 2, 3, 4, 5, 6,
        1, 2, 3, 4, 5, 6, 1, 2, 3, 4, 5, 6, 1, 2, 3, 4, 5, 6,
    ];
```

Note that rustfmt packs multiple elements per line when possible, rather than always breaking to one element per line.

### 3. Handling of Comments

Since rustfmt operates on the AST (which typically doesn't include comments as nodes), it discovers comments by finding source spans that are unused by AST nodes. It then uses heuristics to determine proper placement:

- Leading comments are attached to the following construct
- Trailing comments stay on the same line when possible
- Block comments between tokens are repositioned or deleted based on context

This approach allows for some unusual comment placements to be preserved, but deletes comments in positions where the heuristics can't determine a reasonable placement:

```rust
fn /* this comment will be deleted */ main() {
    println!("Hello, world!");
}
```

This is considered acceptable because comments in such positions are extremely rare and make code harder to read.

## Tradeoffs

### Advantages
- **Syntax transformation**: Can perform semantics-preserving modifications beyond whitespace (import reordering, path simplification, etc.)
- **Fine-grained control**: Different rules for different constructs allow for optimal formatting of each language feature
- **Context-aware formatting**: Can make formatting decisions based on the semantic meaning of code, not just its structure
- **Flexible line breaking**: Variable maximum line lengths per construct type produce more natural-looking code

### Disadvantages
- **High maintenance burden**: Each AST node type requires hand-written formatting logic, leading to thousands of lines of formatting code
- **Rule complexity**: The many formatting rules can interact in unexpected ways, making it difficult to predict output or debug issues
- **Fragile comment handling**: Comments are discovered heuristically rather than being first-class, leading to occasional deletion or misplacement
- **Testing complexity**: Each combination of language features and edge cases requires explicit test coverage
- **No formal model**: Unlike algebraic approaches, there's no mathematical model to reason about formatter correctness or prove properties like idempotency