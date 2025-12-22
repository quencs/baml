---
id: BEP-007
title: "Formatter"
shepherds: Avery Townsend <self@codeshaunted.com>
status: Draft
created: 2025-12-17
---

# BEP-007: Formatter

## Summary

The BAML formatter provides a canonical and easy way to format BAML code for better consistency and readability.

## Motivation

- Most newer languages provide a canonical formatter that makes code's appearance consistent across projects (see: gofmt, rustfmt, etc).
- There currently is not a working formatter, official or otherwise for BAML. Code must be formatted manually or with an LLM.

## Background

### Wadler Documents (Algebraic Approach)
Code is translated into an IR of recursive **Document** types that can print in "flattened" (single-line) or "expanded" (multi-line) forms. A **solver** algorithm decides when to switch between these modes at `Group` markers. Solvers range from simple greedy algorithms to sophisticated graph search (dartfmt [1]). These formatters operate only on whitespace and cannot modify syntax.

### AST-Based Formatters
Formatters like rustfmt [2] and gofmt [3] operate directly on the AST, applying hand-written rules to reprint each node. This enables semantic-preserving transformations (import reordering, literal simplification) but requires verbose, construct-specific formatting logic. gofmt takes a minimalist approach with no line length limits, while rustfmt uses variable width limits per construct type.

## Approach Comparison

| Aspect | Wadler/Algebraic | AST-Based (gofmt) | AST-Based (rustfmt) | Graph Search (dartfmt) |
|--------|-----------------|-------------------|---------------------|----------------------|
| **Complexity** | Low-Medium | Very Low | High | Very High |
| **Line breaking** | Automatic | Manual (preserved) | Automatic (heuristic) | Optimal (search) |
| **Syntax transforms** | No | Yes (simple) | Yes (extensive) | No |
| **Maintainability** | Good | Excellent | Poor | Poor |
| **Output quality** | Good | Variable | Good | Excellent |
| **Performance** | Fast | Very Fast | Fast | Slower |
| **Configuration** | Some | None | Extensive | Some |

Key insights:
- **gofmt**: Prioritizes simplicity and developer trust over optimization
- **rustfmt**: Trades complexity for fine-grained control and syntax transformations
- **dartfmt**: Achieves optimal line breaking at the cost of extreme implementation complexity
- **Pure Wadler**: Best balance of maintainability and output quality for most languages

## Suggested Implementation

### Approach
BAML will use a **CST-based formatter** approach, that will process in a similar way to an AST-based formatter, but with the added advantage of parsed tokens for spans that are missing in the AST. It will eventually have **variable line length limits** per construct type, inspired by rustfmt.

**Why CST-based:**
- **Fine-grained control**: Different constructs (client blocks, function signatures, prompts) have different optimal formatting
- **Syntax upgrades**: Like Go's `-s` flag, we can automatically migrate deprecated syntax (e.g., old type syntax → new, unquoted strings → quoted strings)
- **BAML-specific constructs**: Custom logic for unique features (prompt templates, client configurations, test blocks)

**Proposed Variable line length values:**
- Global: 120 columns
- Function signatures: 100 columns (encourages readable parameter lists)
- Prompt templates: Unlimited (breaking templates is problematic)
- Client/retry policy blocks: 80 columns (keeps config concise)

**Syntax migrations** (via `--fix` or `-s` flag):
- Normalize legacy type syntax
- Simplify redundant constructs
- Convert unidiomatic constructs to idiomatic ones

This trades implementation simplicity for better output quality and forward compatibility. BAML's syntax is simpler than Rust's, keeping maintenance reasonable.

### Idempotency
The formatter must be idempotent: running it multiple times on the same file should produce identical output after the first run. This property is critical for:

- Ensuring formatter stability and predictability
- Allowing safe integration into CI/CD pipelines
- Preventing infinite format loops in editor integrations
- Building user trust in the formatter's consistency

The formatter's test suite will verify idempotency by checking that `format(format(code)) == format(code)` for all test cases.

### Comment Handling
The formatter will preserve comments by:

- Attaching leading comments to the next syntax element
- Attaching trailing comments (same-line) to the preceding syntax element
- Preserving standalone comment blocks with surrounding whitespace
- Comments within expressions will be preserved in-place where possible, or moved to the nearest valid position if the formatter's line-breaking would make them syntactically invalid

### Configuration
The formatter will have a canonical, non-configurable style. This will ensure that all BAML code looks the same.

### Integration
The formatter will be accessible through:

- A `baml format` (`fmt` will also work) CLI command for formatting files or directories
- LSP integration for format-on-save in editors
- A `baml_onionskin` layer displaying formatted code

### Scope
The formatter will handle all top-level BAML language constructs including:

- Imperative function definitions
- LLM function definitions
- Class definitions
- Enum definitions
- Client and retry policy configurations
- Type annotations
- Comments

## Variable Line Width Rules

This section defines the specific formatting rules that the BAML formatter will apply to code. Each shows how code will normally look if fit on a single line, as well as how it will look with possible line breaks.

### Function Definitions
Normally signature is on its own line, followed by indented content lines.
```baml
function Foo(x: int) -> i32 {
    x
}
```

If all arguments do not fit we will do as follows (always with trailing commas for arguments):
```baml
function Foo(
    a: int,
    b: int,
    c: int,
    d: int,
    e: int,
) -> int {
    a + b + c + d + e
}
```

### Class Definitions
Classes will always be printed with the keyword, name, and open brace on the first line, followed by indented lines for each additional member.
```baml
class User {
    id int
    name string
    email string

    function GetName(self) -> string {
        self.name
    }
}
```

### Enum Definitions
Very similar to classes, enums will be printed with the keyword, name, and opening brace on the first line, and then each field with its own indented line within.

```baml
enum Status {
    Active
    Inactive
}
```

### Function Calls
Function calls will usually be printed on one line:
```baml
function Foo() -> int {
    Bar(1);
}
```

But, if there are too many arguments, we will break each argument to its own line with trailing comma:
```baml
function Foo() -> int {
    Bar(
        1,
        2,
        3,
        4,
        5,
        6,
        7,
    );
}
```

### Class Constructors
Class constructors will ideally be printed on a single line. Note leading and trailing spaces within braces:
```baml
function foo() -> Foo {
    Foo { a: 1, b: 2, c: 3 }
}
```

But, if needed they may print with each assignment on its own line with trailing comma:
```baml
function bar() -> Bar {
    Bar {
        a: 1,
        b: 2,
        c: 3,
        d: 4,
        e: 5,
    }
}
```

### Array Constructors
Array constructors will usually be printed on a single line:
```baml
function myArray() -> int[] {
    [1, 2, 3]
}
```

When exceeding the line limit, arrays pack multiple elements per line (like rustfmt), breaking only when necessary:
```baml
function myHugeArray() -> int[] {
    [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
        19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
    ]
}
```

### Map Constructors
Map constructors will usually be printed on a single line:
```baml
function myMap() -> int[] {
    {"a": 1, "b": 2, "c": 3}
}
```

But, when required, they will print with each entry on a single line with trailing comma.
```baml
function myHugeMap() -> int[] {
    {
        "a": 1,
        "b": 2,
        "c": 3,
        "d": 4,
        "e": 5,
        "f": 6,
        "g": 7,
    }
}
```

## Open Questions
1. Should we support a `// baml-fmt: off` comment to disable formatting for specific sections?
2. How should we handle files with syntax errors? Fail gracefully or attempt partial formatting?
3. What specific deprecated syntax patterns should be automatically fixed with `--fix`?

## References

[1] Bob Nystrom, "The Hardest Program I've Ever Written" (2015). https://journal.stuffwithstuff.com/2015/09/08/the-hardest-program-ive-ever-written/

[2] rustfmt Design. https://github.com/rust-lang/rustfmt/blob/main/Design.md

[3] gofmt command documentation. https://pkg.go.dev/cmd/gofmt

[4] Philip Wadler, "A prettier printer" (1998). https://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf

[5] OCaml PPrint exercise. https://ocaml-sf.org/learn-ocaml-public/exercise.html#id=fpottier/pprint