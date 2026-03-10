---
id: BEP-008
title: "Function and Lambda Syntax Design"
shepherds: Language Design Team
status: Draft
created: 2026-03-10
---

# Function & Lambda Syntax Design

**Status:** Draft — seeking feedback\
**Authors:** Language Design Team\
**Last updated:** March 2026

---

## 1. Motivation

TypeScript is one of the most widely adopted typed languages for application development, and Python is one of the most widely adopted languages overall. Both have rich type systems and first-class functions. But both suffer from syntactic inconsistencies that accumulated over years of organic growth — particularly around how functions are declared, typed, and passed around.

This document proposes a function and lambda syntax for our language that is **immediately familiar to TypeScript and Python developers** while being **more internally consistent**. The guiding principle is: the same concept should always look the same, regardless of where it appears.

We introduce concepts one at a time, starting from basic function declarations and building toward higher-order functions, lambdas, and error handling.

---

## 2. Background: What TypeScript Gets Wrong

### 2.1 Too Many Ways to Write a Function

TypeScript has six syntactically distinct ways to define a function, each with subtly different behavior:

```ts
// 1. Declaration (hoisted, has `this` binding)
function greet(name: string): string { return name; }

// 2. Expression (not hoisted, has `this` binding)
const greet = function(name: string): string { return name; }

// 3. Arrow — block body (not hoisted, lexical `this`)
const greet = (name: string): string => { return name; }

// 4. Arrow — expression body (not hoisted, lexical `this`, implicit return)
const greet = (name: string): string => name;

// 5. Method in class
class Foo { greet(name: string): string { return name; } }

// 6. Method shorthand in object
const obj = { greet(name: string): string { return name; } }
```

A newcomer must learn all six forms and understand the behavioral differences (`this` binding, hoisting, `arguments` access) to read real-world code.

### 2.2 The Declaration/Type Inconsistency

This is TypeScript's deepest syntactic problem. When you **write** a function, the return type uses a colon:

```ts
function add(a: number, b: number): number { ... }
```

When you **describe the type** of a function, the return type uses `=>`:

```ts
type Add = (a: number, b: number) => number;
```

And when you put a method in an interface, it uses a colon again:

```ts
interface Math {
  add(a: number, b: number): number;
}
```

Three different syntaxes for the same concept: "a function that takes two numbers and returns a number." This means developers must constantly translate between forms.

### 2.3 The Colon is Overloaded

The `:` character means different things in different positions:

```ts
const x: number = 5;           // type annotation
const obj = { key: value };     // object property
const { key: alias } = obj;    // destructuring rename (!!)
condition ? a : b;              // ternary
case "x": break;               // switch case
label: while(true) { }         // label
```

The destructuring rename is the worst offender. `{ name: n }` *looks* like you're assigning `name` to `n`, but it's the opposite — you're binding the value of `name` to the local variable `n`.

### 2.4 Arrow Functions Create Ambiguities

The `=>` token is used for both arrow function values and function type expressions, creating parsing ambiguity — most infamously with generics in `.tsx` files:

```tsx
// The parser thinks <T> is a JSX element
const identity = <T>(x: T) => x;       // ❌ parse error in .tsx

// Common workarounds
const identity = <T,>(x: T) => x;      // trailing comma hack
const identity = <T extends unknown>(x: T) => x; // redundant constraint
```

---

## 3. Background: What Python Gets Wrong

### 3.1 Lambda Is Crippled

Python's `lambda` is restricted to a single expression:

```python
add = lambda x, y: x + y           # fine
transform = lambda x: x if x > 0 else -x  # awkward but works
# Multi-statement lambdas? Not possible.
```

This forces developers to define a named function every time they need a multi-line callback:

```python
def process_item(item):
    cleaned = clean(item)
    validated = validate(cleaned)
    return validated

results = map(process_item, items)
```

### 3.2 The Colon Collision in Type Hints

Python reuses `:` for both type annotations and `lambda` parameter/body separation, which makes typed lambdas essentially impossible:

```python
# Is the colon a type annotation or the lambda body separator?
f = lambda x: int: x + 1   # ❌ syntax error
```

### 3.3 Return Type Syntax Differs from Annotation Syntax

Variable annotations use `:`, but function return types use `->`:

```python
x: int = 5                              # variable annotation: colon
def add(a: int, b: int) -> int: ...     # return type: arrow
```

This split is defensible (it avoids ambiguity with the function body's `:`), but it means the language uses two different symbols for "has type."

---

## 4. How Other Languages Do It

For reference, here's how function syntax works across languages our users are likely to know.

### 4.1 Declarations and Types

| Language | Declaration | Function Type |
|----------|-------------|---------------|
| **TypeScript** | `function f(x: number): number { return x }` | `(x: number) => number` |
| **Python** | `def f(x: int) -> int: return x` | `Callable[[int], int]` |
| **Kotlin** | `fun f(x: Int): Int { return x }` | `(Int) -> Int` |
| **Scala** | `def f(x: Int): Int = x` | `Int => Int` |
| **Swift** | `func f(x: Int) -> Int { return x }` | `(Int) -> Int` |
| **Go** | `func f(x int) int { return x }` | `func(int) int` |
| **Ours** | `function f(x: int) -> int { x }` | `(int) -> int` |

### 4.2 Lambdas and Higher-Order Functions

| Language       | Lambda                           | HOF parameter              |
| -------------- | -------------------------------- | -------------------------- |
| **TypeScript** | `(x) => x + 1`                   | `f: (x: number) => number` |
| **Python**     | `lambda x: x + 1`                | `f: Callable[[int], int]`  |
| **Kotlin**     | `{ x -> x + 1 }`                 | `f: (Int) -> Int`          |
| **Scala**      | `(x: Int) => x + 1`              | `f: Int => Int`            |
| **Swift**      | `{ (x: Int) in x + 1 }`          | `f: (Int) -> Int`          |
| **Go**         | `func(x int) int { return x+1 }` | `f func(int) int`          |
| **Ours**       | `(x) -> x + 1`                   | `f: (int) -> int`          |

### 4.3 Match/When Arms

| Language | Match arm syntax |
|----------|-----------------|
| **Kotlin** | `x -> result` (in `when`) |
| **Scala** | `case x => result` |
| **Rust** | `x => result` |
| **Swift** | `case x: result` |
| **Ours** | `x => result` |

Notable: we are the only language in this set where the declaration, function type, and lambda all use the same arrow (`->`). Kotlin and Swift come close (`->` for types and lambdas) but use `:` for declaration return types. TypeScript and Scala use `:` for declarations and a different arrow for types.

---

## 5. Design Principles

Based on the problems above, we adopt four principles:

1. **One way to write a function.** A single keyword, a single shape. No behavioral variants.
2. **Types mirror values.** The type of a function should look like the function itself with the body removed.
3. **Each symbol has one meaning.** `->` always means "produces/returns." `=>` always means "maps to" (in pattern matching). `:` always means "has type" or "key/value."
4. **Familiarity over novelty.** Where we can reuse syntax from TypeScript, Python, Rust, or Kotlin without violating principles 1–3, we do.

---

## 6. Familiarity for Our User Base

Today, roughly **two-thirds of our users come from Python** and one-third from TypeScript. Both groups should feel at home reading this syntax — but not necessarily for the same reasons.

### 6.1 What Python Developers Already Recognize

Python developers have written `->` for return type annotations since PEP 484 (Python 3.5):

```python
def add(a: int, b: int) -> int:
    return a + b
```

Our syntax preserves this exact pattern — parameter annotations with `:`, return type with `->`:

```
function add(a: int, b: int) -> int {
  a + b
}
```

The differences are cosmetic: `function` instead of `def`, braces instead of colon-plus-indentation, implicit return instead of explicit `return`. The *type-level shape* is identical. A Python developer reading our function signatures can parse them immediately without learning new symbols.

Python's lambda syntax, however, is where our design diverges — intentionally. Python's `lambda x: x + 1` reuses `:` as the body separator, which collides with type annotations and makes typed lambdas impossible (see Section 3.2). Our `(x) -> x + 1` avoids that collision entirely while using the same `->` arrow Python developers already associate with "produces a value."

**Summary for Python users:** `->` means exactly what you think it means. Parameter types use `:` exactly as you'd expect. Lambdas work like you always wished they did.

### 6.2 What TypeScript Developers Already Recognize

TypeScript developers will recognize `function`, parenthesized parameters with `: type` annotations, and brace-delimited bodies. The one adjustment is `->` instead of `:` for return types:

```ts
// TypeScript
function add(x: number, y: number): number { return x + y; }

// Ours
function add(x: int, y: int) -> int { x + y }
```

This is a small change with a large payoff. TypeScript developers already know that `->` means "function" from Python, from Haskell-style type notation, and from their own `=>` arrow functions. The shift from `:` to `->` removes the declaration/type inconsistency that TypeScript developers already find confusing (see Section 2.2) — many will see this as a fix rather than a departure.

For lambdas, the adjustment is similarly minor — `->` instead of `=>`:

```ts
// TypeScript
items.map((x) => x.name)

// Ours
items.map((x) -> x.name)
```

One character difference. The mental model is identical: parameters, arrow, body.

**Summary for TypeScript users:** `function`, `:` annotations, and braces all work as expected. The only change is `->` instead of `:` for return types and `->` instead of `=>` for lambdas — and both changes eliminate inconsistencies you've already bumped into.

### 6.3 Why This Balance Matters

With 2x as many Python users as TypeScript users, we lean toward conventions that Python developers find natural — and `->` for return types is the single most impactful example. But we don't sacrifice TypeScript familiarity to get there: `function`, `:` for annotations, and `{ }` for bodies are all TypeScript staples. The result is a syntax that both communities can read on day one, with the consistency that neither language offers on its own.

---

## 7. Function Declarations

### 7.1 Basic Form

A function declaration uses the `function` keyword, parenthesized parameters with type annotations, `->` for the return type, and a brace-delimited body. The last expression in the body is the return value.

```
function add(x: int, y: int) -> int {
  x + y
}

function greet(name: string) -> string {
  "Hello, " + name
}
```

**Why `function` and not `fn`?** Familiarity. Every TypeScript and JavaScript developer already reads `function` fluently. Brevity is nice, but we value recognition over keystroke savings.

**Why `->` and not `:`?** This is the core departure from TypeScript. Using `->` for the return type means the function's *declaration syntax* and its *type syntax* can share the same shape (see Section 9). The colon remains reserved for "has type" in annotation position: `x: int`.

**Why implicit return?** Explicit `return` is still supported for early returns, but the last expression in a block is its value. This reduces noise in small functions and is familiar from Rust and Kotlin.

### 7.2 Functions with No Return Value

Functions that perform side effects and don't return a meaningful value omit the return type (or optionally annotate it as `void`):

```
function log(message: string) {
  print(message)
}

// Equivalent, if you prefer to be explicit:
function log(message: string) -> void {
  print(message)
}
```

### 7.3 Comparison with TypeScript

| Concept | TypeScript | Ours |
|---------|-----------|------|
| Basic function | `function add(x: number, y: number): number { return x + y; }` | `function add(x: int, y: int) -> int { x + y }` |
| Return type marker | `:` (colon) | `->` (arrow) |
| Implicit return | No (requires `return`) | Yes (last expression) |
| Variations | 6 syntactic forms | 1 form |

---

## 8. Lambdas (Anonymous Functions)

### 8.1 The Problem

TypeScript's arrow functions come in multiple forms and create ambiguity with type syntax (see Section 2 and 3). Python's lambdas are limited to single expressions. We need anonymous functions that are concise for simple cases but scale to complex ones.

### 8.2 Braced Lambdas (Block Body)

A lambda is written as parameters, `->`, and a brace-delimited body:

```
(x: int, y: int) -> {
  let sum = x + y
  sum * 2
}
```

This is identical to a function declaration minus the `function` keyword and name. The braces make it unambiguous — the parser sees `->` followed by `{` and knows it's a function body, not a type.

### 8.3 Braceless Lambdas (Expression Body)

For simple one-expression lambdas, braces can be omitted:

```
(x: int) -> x + 1
(a: string, b: string) -> a + b
```

This is the form that makes `.map()`, `.filter()`, and `.reduce()` ergonomic:

```
items.map((x) -> x.name)
items.filter((x) -> x.active)
items.reduce((a, b) -> a + b)
```

### 8.4 Why This Isn't Ambiguous

At first glance, a braceless lambda and a function type look identical:

```
(x: int) -> int       // Is this a type or a lambda returning a variable called `int`?
```

But in our language, **type names and variable names are always distinguishable**. Built-in types like `int`, `string`, and `bool` are reserved keywords — you cannot name a variable `int`. User-defined types follow a capitalized naming convention (e.g. `Response`, `UserProfile`).

This means the parser can always determine what follows `->`:

```
(x: int) -> int           // `int` is a type → this is a function type
(x: int) -> count         // `count` is a variable → this is a lambda
(x: int) -> int { x + 1 }         // expression with operator → this is a lambda
(x: int) -> T { x + 1 }    // brace → this is a lambda with block body
```

```
array.map((x) -> string { x.name })
array.map((x) -> { x.name })
type foo = (x: string) -> x.name
array.map((x) -> x.name)


array.map((x): string -> x.name)
type foo = (x: string): x.name
array.map((x) -> x.name)
```
### 8.5 Typed Lambdas

Because braces clearly separate "body" from "type," you can optionally annotate a lambda's return type by placing it between `->` and `{`:

```
// Untyped lambda (return type inferred)
(x: int) -> { x + 1 }

// Typed lambda (return type annotated)
(x: int) -> int { x + 1 }
```

The parser sees `-> int` then checks the next token. If it's `{`, this is a typed lambda. If it's end-of-expression, it's a function type. Unambiguous.

Note that **braceless lambdas cannot have return type annotations** — there's no syntactic position for them without introducing ambiguity. This is an acceptable tradeoff: if a lambda is simple enough to be braceless, its return type is almost certainly inferrable.

### 8.6 Comparison with TypeScript

#### How TypeScript Types Lambdas

TypeScript has two ways to annotate a lambda's return type, each with different syntax:

```ts
// 1. Inline annotation — return type before the arrow
const f = (x: number): number => x + 1
const g = (x: number): number => { return x + 1 }

// 2. Variable annotation — type on the left, lambda on the right
const f: (x: number) => number = (x) => x + 1
```

Option 1 is the most common, but it creates visual noise — the `:` return type annotation sits between the parameters and the `=>`, breaking the flow. Option 2 duplicates the parameter list (once in the type, once in the value).

Note that in Option 1, the return type uses `:` (like a declaration), but in Option 2, the function type uses `=>`. This is the declaration/type inconsistency again, now appearing within the same feature.

#### How We Type Lambdas

We have one way: the return type sits between `->` and `{`.

```
// Untyped (inferred)
(x: int) -> x + 1
(x: int) -> { x + 1 }

// Typed — return type before the brace
(x: int) -> int { x + 1 }
(x: int, y: int) -> int {
  let sum = x + y
  sum * 2
}

// Variable annotation also works
let f: (int) -> int = (x) -> x + 1
```

Braceless lambdas cannot have return type annotations — there's no syntactic position for them. This is an acceptable tradeoff: if a lambda is simple enough to be braceless, its return type is almost certainly inferrable.

#### Side-by-Side

| Concept | TypeScript | Ours |
|---------|-----------|------|
| Block lambda | `(x: number) => { return x + 1; }` | `(x: int) -> { x + 1 }` |
| Expression lambda | `(x: number) => x + 1` | `(x: int) -> x + 1` |
| Typed lambda (inline) | `(x: number): number => { return x + 1; }` | `(x: int) -> int { x + 1 }` |
| Typed lambda (variable) | `const f: (x: number) => number = (x) => x + 1` | `let f: (int) -> int = (x) -> x + 1` |
| Return type symbol in lambda | `:` (same as declarations, different from types) | `->` (same everywhere) |
| Object return gotcha | `(x) => ({ key: x })` (parens required) | Not applicable (braces always mean body) |

The TypeScript "object literal return" gotcha — where `(x) => { key: x }` is parsed as a block with a label, not an object — does not exist in our syntax because we do not overload `{}` to mean both "block" and "object literal" in lambda position.

---

## 9. Function Types

### 9.1 The Core Insight

A function's type should be its declaration with the name and body removed. Given:

```
function add(x: int, y: int) -> int {
  x + y
}
```

The type is:

```
(x: int, y: int) -> int
```

You literally strip the keyword, name, and body. The same `->` symbol, the same parameter syntax, the same return type position.

### 9.2 Parameter Names Are Optional in Types

In type position, parameter names serve as documentation only. They don't affect type checking or calling convention:

```
// All three are the same type:
(x: int, y: int) -> int
(a: int, b: int) -> int
(int, int) -> int
```

Named parameters in types help with readability in complex signatures:

```
// Clearer with names
(url: string, timeout: int, retries: int) -> Response

// But structurally identical to
(string, int, int) -> Response
```

**Important:** Parameter names in type position are strictly for documentation. If you return `(x, y) -> { x - y }` from a function typed as `(y: int, x: int) -> int`, the returned function subtracts its second argument from its first. The type-level names don't rearrange arguments.

### 9.3 Higher-Order Function Types

Functions that accept or return other functions use the same `(params) -> return` syntax nested:

```
// A function that takes a transformer and applies it
function apply(f: (int) -> int, value: int) -> int {
  f(value)
}

// A function that returns a function
function makeAdder(n: int) -> (int) -> int {
  (x) -> x + n
}

// A function that takes and returns functions
function compose(
  f: (int) -> string,
  g: (string) -> bool
) -> (int) -> bool {
  (x) -> g(f(x))
}
```

### 9.4 Nesting Reads Left to Right

The `->` arrow is right-associative, so chained function types read naturally:

```
// A function returning a function returning a function
(int) -> (int) -> (int) -> int

// Reads as: takes int, returns (takes int, returns (takes int, returns int))
```

This is the same currying notation familiar from Haskell (`Int -> Int -> Int -> Int`) but with explicit parameter grouping via parentheses.

### 9.5 Comparison with TypeScript

| Concept | TypeScript | Ours |
|---------|-----------|------|
| Function type | `(a: number, b: number) => number` | `(a: int, b: int) -> int` |
| Return type symbol | `=>` (different from `:` in declarations!) | `->` (same as declarations) |
| Method in interface | `add(a: number, b: number): number` | `add(a: int, b: int) -> int` |
| Call signature | `{ (x: number): number }` | `(x: int) -> int` |
| Consistency | 3+ forms for the same concept | 1 form everywhere |

---

## 10. Putting It Together: The Unified Shape

Everything is built from one pattern:

```
(parameters) -> [return type] [{ body }]
```

The presence or absence of each optional piece determines what you're looking at:

| Has return type? | Has body? | What is it? |
|-----------------|-----------|-------------|
| Yes | Yes (`{ ... }`) | Typed lambda |
| No | Yes (`{ ... }`) | Untyped lambda (inferred) |
| Yes | No | Function type |
| No | No | — (not valid alone) |

And a named function declaration is simply this pattern with `function name` prepended:

```
function name(parameters) -> [return type] { body }
```

Everything composes from the same pieces. There is nothing extra to memorize.

---

## 11. Error Handling: `throws` Clauses

### 11.1 Motivation

Many languages treat errors as invisible — you can't tell from a function's signature whether it might fail. Java introduced checked exceptions but made them syntactically heavy. We want a lightweight way to express fallibility in the type system.

### 11.2 Syntax

The `throws` clause sits between the return type and the body (or end of type):

```
// Named function
function divide(x: int, y: int) -> int throws DivideByZero {
  if y == 0 { throw DivideByZero() }
  x / y
}

// Typed lambda
(x: int, y: int) -> int throws DivideByZero {
  if y == 0 { throw DivideByZero() }
  x / y
}

// Braceless lambda — throws but no return type annotation
(x: int) throws ParseError -> parse(x)

// Function type
(int, int) -> int throws DivideByZero
```

### 11.3 Multiple Error Types

Multiple error types are separated by `|` (union):

```
function fetch(url: string) -> Response throws NetworkError | TimeoutError {
  ...
}

// In type position:
(string) -> Response throws NetworkError | TimeoutError
```

### 11.4 Higher-Order Functions with Throws

The `throws` clause composes naturally in higher-order function types:

```
// Accept a fallible function and handle its errors
function withRetry(
  f: (string) -> Response throws NetworkError,
  retries: int
) -> Response throws TimeoutError {
  ...
}
```

### 11.5 Why This Position?

The `throws` clause appears after the return type because it modifies the function's contract as a whole, not just the return value. Reading left to right: "takes these parameters, returns this type, but might throw these errors." It's also the position Java and Kotlin use, so it's familiar.

---

## 12. Pattern Matching: `match` and `=>`

### 12.1 The Arrow Budget

We've established that `->` means "produces a value / returns a type" in function context. Pattern matching also involves a "produces" relationship — a pattern arm produces a result. Should it reuse `->`?

Consider a match expression where the result is a lambda:

```
// Using -> for both match arms and function returns
match mode {
  "add" -> (x: int) -> x + 1
  "sub" -> (x: int) -> x - 1
}
```

Two `->` on the same line with different meanings. Now consider returning a function type:

```
match mode {
  "add" -> (int) -> int
  "sub" -> (int) -> int
}
```

This is technically parseable but visually confusing. Which `->` is the match arm separator and which is the function return arrow?

### 12.2 Decision: `=>` for Match Arms

We use `=>` exclusively for pattern matching, keeping `->` exclusively for functions:

```
match mode {
  "add" => (x: int) -> x + 1
  "sub" => (x: int) -> x - 1
}
```

Now every `->` in the language means "function arrow" and every `=>` means "pattern maps to result." No mixing.

```
match request {
  GET(path) => handleGet(path)
  POST(path, body) => handlePost(path, body)
  _ => (req) -> { defaultHandler(req) }
}
```

### 12.3 Precedent

This matches Rust (`=>` for match arms, `->` for return types) and Scala (`=>` for case arms, `->` for function types). Both languages report that developers internalize the distinction quickly.

### 12.4 Arrow Symbol Summary

| Symbol | Meaning | Example |
|--------|---------|---------|
| `->` | Function returns / produces | `(int) -> int`, `(x) -> x + 1` |
| `=>` | Pattern maps to result | `Some(x) => x`, `"add" => handler` |
| `:` | Has type / key-value | `x: int`, `{ key: value }` |

Three symbols, three meanings, no overlap.

---

## 13. Alternatives Considered

### 13.1 `fn` Instead of `function`

```
fn add(x: int, y: int) -> int { x + y }
```

**Pros:** Shorter. Familiar from Rust.\
**Cons:** Less familiar to the TypeScript/JavaScript audience we're targeting. Keyword brevity is a marginal benefit — modern editors auto-complete it. The full word `function` also acts as a stronger visual anchor when scanning code.

**Decision:** Use `function`. Revisit if community feedback strongly favors `fn`.

### 13.2 `:` for Return Types (TypeScript Style)

```
function add(x: int, y: int): int { x + y }
```

**Pros:** Identical to TypeScript syntax. Zero learning curve for TS developers.\
**Cons:** Creates the declaration/type inconsistency described in Section 2.2. The function type would need a different symbol (TypeScript uses `=>`), and we'd lose the "strip the name and body to get the type" property. This is the single biggest source of syntactic inconsistency in TypeScript and the primary motivation for this design.

**Decision:** Rejected. The consistency gains from `->` outweigh the familiarity cost.

### 13.3 `=>` for Lambdas (TypeScript Style)

```
items.map((x) => x.name)
```

**Pros:** Familiar to every JavaScript developer.\
**Cons:** If `=>` is used for lambdas, it can't also be used for pattern matching without ambiguity. And if we use `->` for declarations but `=>` for lambdas, we've reintroduced the two-arrow problem. The whole point of this design is one arrow for functions.

**Decision:** Rejected. `->` for all function contexts; `=>` reserved for `match`.

### 13.4 Mandatory Braces on All Lambdas

```
items.map((x) -> { x.name })
items.filter((x) -> { x.active })
```

**Pros:** Maximally unambiguous. The parser never needs to distinguish type-vs-value from context because `{` always signals a body.\
**Cons:** Adds visual noise to the most common lambda use case. Chained higher-order functions (map/filter/reduce) become noticeably harder to scan. Go gets away with verbose anonymous functions because its culture favors `for` loops over functional pipelines. If our language encourages functional composition — and our investment in higher-order function syntax suggests it does — the cost is too high.

**Decision:** Rejected. Allow braceless lambdas for single expressions. The type/value ambiguity is resolved by distinguishing type names from variable names (see Section 8.4).

### 13.5 Trailing Closure Syntax (Kotlin/Swift Style)

```
items.filter { x -> x.active }
items.map { x -> x.name }
```

**Pros:** Very concise for single-lambda arguments. Eliminates nested parentheses.\
**Cons:** Introduces a new syntax just for one pattern (last-argument lambdas). Adds complexity to the grammar. Our braceless lambda syntax already handles the common case well enough: `items.filter((x) -> x.active)`.

**Decision:** Deferred. This can be added later as sugar without breaking anything. We focus on getting the core right first.

### 13.6 `->` for Match Arms (Unify Everything Under One Arrow)

```
match x {
  0 -> "zero"
  n -> "other"
}
```

**Pros:** Only one arrow symbol in the language.\
**Cons:** Ambiguity when match arms return functions or function types (see Section 12.1). Two identical symbols with different scoping rules on the same line is a readability failure. The cost of a second arrow symbol is low; the cost of visual ambiguity is high.

**Decision:** Rejected. `=>` for match, `->` for functions.

---

## 14. Summary

The complete syntax at a glance:

```
// ━━━ Named Functions ━━━

function add(x: int, y: int) -> int {
  x + y
}

function fetchData(url: string) -> Response throws NetworkError | TimeoutError {
  ...
}


// ━━━ Lambdas ━━━

// Braceless (expression body, types inferred)
(x) -> x + 1
(a, b) -> a + b

// Braced (block body, optional type annotation)
(x: int) -> { x + 1 }
(x: int) -> int { x + 1 }
(x: int) -> int throws ParseError { parse(x) }


// ━━━ Function Types ━━━

(int) -> int
(string, int) -> Response
(url: string, retries: int) -> Response throws NetworkError


// ━━━ Higher-Order Functions ━━━

function apply(f: (int) -> int, value: int) -> int {
  f(value)
}

function compose(f: (A) -> B, g: (B) -> C) -> (A) -> C {
  (x) -> g(f(x))
}


// ━━━ Pattern Matching ━━━

match status {
  200 => "OK"
  404 => "Not Found"
  code => "Unknown: " + code.toString()
}


// ━━━ Chaining ━━━

items
  .filter((x) -> x.active)
  .map((x) -> x.name)
  .sort((a, b) -> a.compareTo(b))
```

### The Core Rule

Everything follows from one pattern:

```
(params) -> [return type] [throws Errors] [{ body }]
```

Prepend `function name` for a declaration. Remove the body for a type. It's the same shape, everywhere, every time.
