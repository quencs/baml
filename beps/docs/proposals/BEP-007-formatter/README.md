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
There are two general categories of formatters that are commonly in use.

### Wadler Documents
[Popularized by Philip Wadler](https://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf), this is an algebraic approach, where each line of code is translated into IR as the recursive **Document** type, which can either be printed on a single line in its "flattened" representation, or can be broken into multiple lines at predefined locations. These types of formatters are generally only able to operate on whitespace and do not change syntax.

#### The **Document**
The following is an example adapted from [an OCaml exercise](https://ocaml-sf.org/learn-ocaml-public/exercise.html#id=fpottier/pprint), showing generally how an algebraic formatter works.

```ocaml
type doc =
  | Empty (* The empty document *)
  | HardLine (* A line break followed by indent to match current indent level*)
  | Text of string (* A plaintext string *)
  | Cat of doc * doc (* A concatenation of two documents *)
  | Nest of int * doc (* Nests the provided document at the provided indentation level *)
  | Group of doc (* A marker for the choice of whether or not to print the document flat *)
  | IfFlat of doc * doc () (* Prints the first document if flattening, the second if normal *)
```

The printer has two possible states, **normal** and **flattening**. It is offered the choice between these two states at `Group` documents, a **solver** algorithm dictates whether or not to switch. The simplest solver is one that breaks greedily if it determines that the document will not fit within a specified amount of columns if flattened. The most complicated solvers implement graph search with a utility function, as seen with [dartfmt](https://journal.stuffwithstuff.com/2015/09/08/the-hardest-program-ive-ever-written/). From here, the printing of a document will change based on the current state when it encounters `IfFlat` documents.

### Custom Rules
Some formatters like [rustfmt](https://github.com/rust-lang/rustfmt/blob/main/Design.md) use a much less general approach, and operate on the AST level, re-printing the AST as code. This approach ends up being far more verbose in code, but can provide very good formatting for specific situations. These formatters are also able to modify syntax without altering semantics. This is advantageous for improving consistency in code, for example: reordering imports.

## Comments
Comments and other trivia cause problems for both of these systems. Because comments can be placed basically anywhere, decisions need to be made about whether to leave them where they are, move them, or even delete them in some cases.

For example, rustfmt chooses to simply delete the comment in this case rather than attempt to re-place it:
```rust
fn /* why would you put a comment here */ main() {
    println!("Hello, world!");
}
```

## Suggested Implementation

### Approach
With BAML's lossless CST, it seems that a Wadler-style algebraic formatter would be much easier and in the end more reliable. Formatters with custom rules like rustfmt that operate on the AST can produce better code in some situations, but are much harder to maintain due to their verbosity and many rules.

With this choice, a solver algorithm also needs to be written. For now, a simple greedy algorithm should suffice. dartfmt's implementation of graph search is admirable, but also comes with the cost of maintaining the utility function and increased complexity. The beauty of the Wadler approach is that the algebra is solver-agnostic and the solver can be changed in the future if graph search is desired.

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
The initial implementation will use a canonical, non-configurable style with these defaults:
- **Indent**: 4 spaces
- **Line width**: 120 columns
- **Trailing commas**: Always included in multi-line lists

Future versions may allow configuration through a `.bamlformat` file or similar.

### Integration
The formatter will be accessible through:

- A `baml fmt` (`format` will also work) CLI command for formatting files or directories
- LSP integration for format-on-save in editors
- A `--check` flag for CI/CD to verify code is formatted
- A `baml_onionskin` layer displaying formatted code

### Scope
The formatter will handle all BAML language constructs including:

- Function definitions
- Class definitions
- Enum definitions
- Client and retry policy configurations
- Type annotations
- Comments and documentation

## Formatting Rules

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

But, when required, they will print with each entry on a single line with trailing comma:
```baml
function myHugeArray() -> int[] {
    [
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
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

## Non-Goals
This formatter will not:

- Reorder declarations (imports, functions, etc.)
- Perform syntax refactoring or code transformations
- Enforce naming conventions
- Validate correctness beyond syntax

## Open Questions
1. Should we support a `// baml-fmt: off` comment to disable formatting for specific sections?
2. How should we handle files with syntax errors? Fail gracefully or attempt partial formatting?
3. Should the formatter automatically fix certain deprecated syntax patterns if they exist?