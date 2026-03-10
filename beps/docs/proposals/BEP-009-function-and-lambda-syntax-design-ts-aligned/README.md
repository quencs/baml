---
id: BEP-009
title: "Function and Lambda Syntax Design (TS-aligned)"
shepherds: Language Design Team
status: Proposed
created: 2026-03-10
---

# BEP-009: Function and Lambda Syntax Design (TS-aligned)

**Status:** Proposed\
**Authors:** Language Design Team\
**Last updated:** March 2026\
**Related:** BEP-008 (documents the current `->` unified syntax and its design rationale)

---

## Abstract

This proposal defines the function declaration, lambda, and function type syntax for the language. It adopts TypeScript's existing syntactic conventions — `:` for return types in declarations, `=>` for arrow functions and function types — and makes targeted changes to address known deficiencies: implicit return to eliminate a class of silent bugs, restriction to two function forms to remove behavioral variants, mandatory parentheses on lambda parameters, optional `throws` clauses for checked error types, and `match` expressions that reuse `=>` for pattern arms.

The guiding principle is conservative: adopt proven syntax from the most widely-used typed language in our target audience, and deviate only where there is concrete evidence of developer harm.

---

## 1. Current State of the Art

The language currently uses a unified `->` syntax for all function contexts — declarations, lambdas, and types. This section documents the existing design and its properties, so that this proposal's changes can be evaluated against the current baseline.

### 1.1 Current Syntax

Function declarations use the `function` keyword with `->` for the return type:

```
function add(x: int, y: int) -> int {
  x + y
}

function greet(name: string) -> string {
  "Hello, " + name
}
```

Lambda expressions use the same `->` arrow:

```
// Expression body
(x) -> x + 1
items.map((x) -> x.name)

// Block body
(x: int, y: int) -> {
  let sum = x + y
  sum * 2
}

// Typed lambda — return type between -> and {
(x: int) -> int {
  x + 1
}
```

Function types also use `->`:

```
(int, int) -> int
(string) -> Response
```

Match arms use `=>` to avoid visual collision with `->`:

```
match status {
  200 => "OK"
  404 => "Not Found"
}
```

### 1.2 Properties of the Current Design

The current syntax has one core property: **the type of a function is its declaration with the name and body removed.**

```
// Declaration:
function add(x: int, y: int) -> int { x + y }

// Type (remove `function add` and the body):
(x: int, y: int) -> int
```

The same `->` symbol appears in every function context. There is no notational split between declarations and types.

Additionally, the `->` token serves as a **unique, unambiguous signal** that a return type follows. Because `->` has no other meaning in the language, a reader (human or machine) encountering `->` knows immediately that the next token is a return type — without needing to inspect surrounding context. This is a useful property for LLMs parsing code: the `->` token acts as a structural landmark that reduces the context window needed to understand a function's signature. In TypeScript, `:` serves this role in declarations but is also used for parameter annotations, object keys, and ternary expressions, requiring more context to disambiguate.

Finally, it is worth noting that **when we presented both syntax options to three mainstream AI models — ChatGPT o3, Claude Opus 4.6, and Gemini 2.5 Pro — all three independently preferred the `->` arrow syntax.** Each cited the same reasons: consistency across declarations, lambdas, and types; the clean "strip name and body to get the type" property; and the reduced ambiguity of a dedicated return type symbol. This is notable because these models have been trained overwhelmingly on TypeScript and would be expected to favor familiar syntax. That they prefer the less familiar but more consistent option suggests the design properties of `->` are genuinely valuable, not just aesthetically pleasing to language designers.

### 1.3 What This Proposal Changes

This proposal replaces `->` with TypeScript's conventions: `:` for return types in declarations, `=>` for lambdas and function types. The following table summarizes the differences:

| Construct | Current (`->` unified) | Proposed (TS-aligned) | TypeScript |
|-----------|------------------------|----------------------|------------|
| Declaration | `function f(x: int) -> int { x }` | `function f(x: int): int { x }` | `function f(x: number): number { return x }` |
| Lambda (expr) | `(x) -> x + 1` | `(x) => x + 1` | `(x) => x + 1` |
| Lambda (block) | `(x: int) -> { x + 1 }` | `(x: int) => { x + 1 }` | `(x: number) => { return x + 1 }` |
| Typed lambda | `(x: int) -> int { x + 1 }` | `(x: int): int => { x + 1 }` | `(x: number): number => { return x + 1 }` |
| Function type | `(int) -> int` | `(int) => int` | `(x: number) => number` |
| HOF parameter | `f: (int) -> int` | `f: (int) => int` | `f: (x: number) => number` |
| Declaration + throws | `function f(x: int) -> int throws E { x }` | `function f(x: int): int throws E { x }` | N/A |
| Lambda + throws | `(x: int) -> int throws E { x + 1 }` | `(x: int): int throws E => { x + 1 }` | N/A |
| Match arm | `pattern => result` | `pattern => result` | N/A |
| Return type symbol (decl) | `->` | `:` | `:` |
| Return type symbol (type) | `->` | `=>` | `=>` |
| Implicit return | Yes | Yes | No |

The following aspects are **not changed** by this proposal and are not in contention:

- **Implicit return.** The last expression in a block is its value. `return` is available for early exits only.
- **Two function forms.** `function` for named declarations, an arrow syntax for anonymous lambdas. No function expressions, no behavioral variants.
- **Mandatory parentheses.** Always `(x) => ...`, not `x => ...`.
- **`throws` clauses.** Optional checked error types on braced forms.
- **Pattern matching.** `match` with `=>` arms (Scala/Rust precedent).

The change is narrow: **which arrow symbol to use for functions, and whether declarations and types should share the same notation.**

---

## 2. Motivation

### 2.1 The Core Thesis

**This proposal makes a conscious choice to emulate TypeScript syntax, even where that syntax has known consistency issues, in order to maximize adoptability for human developers.** We believe that languages succeed or fail based on how quickly real people can read and write them — not on how internally elegant their grammar is. The gains from a "purer" syntax do not justify the cost of deviating from a language that millions of developers already know. Even in a world where AI agents write 99% of the code, humans still need to read it, review it, and — critically — *like it*. A language that looks foreign on first glance fails the vibe check, and no amount of internal consistency compensates for that.

The current `->` syntax is internally consistent: one arrow, one shape, types mirror values. That is a real property with real value. But it is a property that primarily appeals to language designers and compiler authors. For the developer writing their first function in this language, the question is not "can I derive the type from the declaration by removing the name?" — it is "does this look like something I already know how to write?"

We should deviate from well-known languages only when the deviation fixes a concrete problem. The current syntax fixes a theoretical inconsistency in TypeScript's notation. This proposal argues that the inconsistency is not a problem developers actually have, and that the fix introduces an unnecessary learning cost.

### 2.2 User Demographics

Our user base is approximately **two-thirds Python developers and one-third TypeScript developers**. Both groups must learn new syntax regardless of which approach is taken. Python developers will encounter braces, the `function` keyword, and a structural type system. TypeScript developers will encounter new primitive type names and implicit return semantics.

The question this proposal addresses is: **given that both groups must adapt, how much additional syntactic novelty is justified?**

This proposal argues for the minimum amount. Every syntactic convention shared with TypeScript is a convention that does not need to be taught, documented, or corrected in AI-generated code. TypeScript is the closest widely-used typed language to what we are building. When we have no strong reason to deviate, we should not deviate.

### 2.3 AI Code Generation

Large language models trained on code have been exposed to orders of magnitude more TypeScript than any novel syntax. When an AI agent generates function code for our language, it will default to TypeScript patterns. Each departure from TypeScript syntax represents a potential error site in generated code that must be caught and corrected.

Adopting TypeScript syntax directly means that AI-generated code is more likely to be syntactically correct without language-specific fine-tuning or few-shot prompting. (Section 5.4 discusses the limitations of this argument.)

---

## 3. Proposed Design

This section specifies the syntax adopted from TypeScript without modification.

### 3.1 Function Declarations

Function declarations use the `function` keyword, parenthesized parameters with colon type annotations, a colon-prefixed return type, and a brace-delimited body:

```ts
function add(x: int, y: int): int {
  x + y
}

function greet(name: string): string {
  "Hello, " + name
}

function log(message: string) {
  print(message)
}
```

When no return type is specified, the function is inferred to return `void`. An explicit `: void` annotation is permitted but not required.

The syntax is identical to TypeScript with two exceptions: primitive type names differ (`int` vs `number`), and the last expression in a block is its return value (see Section 4.1).

### 3.2 Lambda Expressions

Lambda expressions use parenthesized parameters, the `=>` arrow, and either an expression body or a brace-delimited block body:

```ts
// Expression body — single expression, no braces
(x: int) => x + 1
items.map((x) => x.name)
items.filter((x) => x.active)

// Block body — multiple statements, braces required
(x: int, y: int) => {
  let sum = x + y
  sum * 2
}
```

Parentheses around parameters are always required, including for single-parameter lambdas. The form `x => x + 1` (without parentheses) is a syntax error. See Section 4.3 for rationale.

### 3.3 Typed Lambdas

TypeScript provides two mechanisms for annotating the return type of a lambda expression:

```ts
// Inline annotation — return type placed between parameters and arrow
let f = (x: number): number => x + 1
let g = (x: number): number => { return x + 1 }

// Contextual annotation — type placed on the binding, lambda infers from context
let f: (x: number) => number = (x) => x + 1
```

This proposal retains both mechanisms:

```ts
// Inline annotation
let f = (x: int): int => x + 1
let g = (x: int): int => {
  x + 1
}

// Contextual annotation
let f: (x: int) => int = (x) => x + 1
```

The inline form uses `:` for the return type (matching declaration syntax), while the contextual form uses `=>` (matching function type syntax). This is the same declaration/type notational split present in TypeScript; Section 5.1 discusses the tradeoffs of inheriting it.

In practice, most lambda expressions appear in callback position where parameter and return types are inferred from the enclosing function signature:

```ts
// Types of x inferred from the signature of .map()
items.map((x) => x.name)
```

Explicit lambda type annotations are uncommon in idiomatic TypeScript and are expected to be uncommon in our language as well.

### 3.4 Function Types

Function types use the `=>` arrow, matching TypeScript:

```ts
(x: int, y: int) => int
(string) => bool
(url: string, retries: int) => Response
```

Parameter names in type position are optional and serve as documentation only. The following are structurally identical types:

```ts
(x: int, y: int) => int
(a: int, b: int) => int
(int, int) => int
```

### 3.5 Higher-Order Functions

Functions that accept or return other functions compose naturally:

```ts
function apply(f: (int) => int, value: int): int {
  f(value)
}

function compose(f: (int) => string, g: (string) => bool): (int) => bool {
  (x) => g(f(x))
}

function makeAdder(n: int): (int) => int {
  (x) => x + n
}
```

---

## 4. Deviations from TypeScript

Each deviation addresses a documented class of bugs or developer friction. Deviations are enumerated explicitly so that the scope of change is clear.

### 4.1 Implicit Return

**Problem.** In TypeScript, block-body functions require an explicit `return` statement. Omitting it causes the function to return `undefined`. Whether this produces a compile-time error depends on how types flow through the surrounding code:

```ts
// No error — TypeScript infers names as void[], not string[]
const names = items.map((x) => {
  const cleaned = x.name.trim()
  cleaned.toUpperCase()
})
// names is void[] — the bug propagates silently until something downstream fails
```

If the variable were explicitly annotated (`const names: string[] = ...`), TypeScript would catch the mismatch. But most TypeScript code relies on type inference, so the `undefined` propagates — sometimes through multiple layers — before surfacing as a type error far from the original mistake, or as a runtime failure.

The root cause is that TypeScript defines two body forms with different return semantics: expression bodies (implicit return) and block bodies (explicit return). Refactoring from expression body to block body — a routine edit when adding a statement to a callback — changes whether the computed value is returned or discarded. In well-annotated codebases the compiler catches this; in inference-heavy code it often does not.

This pattern is common enough to warrant four separate ESLint rules: `array-callback-return`, `consistent-return`, `no-useless-return`, and `arrow-body-style`. The existence of these rules suggests the type system alone does not reliably prevent this class of bug.

**Resolution.** In our language, the last expression in a block is its return value. The `return` keyword remains available for early exits but is never required at the end of a function body:

```ts
items.map((x) => {
  let cleaned = x.name.trim()
  cleaned.toUpperCase()  // this IS the return value
})
```

This applies uniformly to `function` declarations and `=>` lambdas. Refactoring between expression body and block body requires only adding or removing braces — no change in return behavior.

### 4.2 Two Function Forms

**Problem.** TypeScript provides six syntactically distinct ways to define a function (declaration, function expression, arrow with expression body, arrow with block body, class method, object method shorthand), each with different behavior regarding `this` binding, hoisting, and the `arguments` object. This produces linter rules such as `func-style`, `prefer-arrow-callback`, and `no-loop-func`.

**Resolution.** This language provides two forms:

| Form | Syntax | Use case |
|------|--------|----------|
| Declaration | `function f(x: int): int { ... }` | Named, top-level or nested functions |
| Lambda | `(x: int) => ...` | Anonymous, inline functions |

There is no function expression syntax (`const f = function() {}`). Both forms share identical semantics: same capture behavior, no `this` binding distinction, no hoisting. The choice between forms is purely syntactic — named vs. anonymous.

### 4.3 Mandatory Parentheses

**Problem.** TypeScript permits omitting parentheses for single-parameter arrow functions (`x => x + 1`), producing the linter rule `arrow-parens` and inconsistent formatting across codebases.

**Resolution.** Parentheses are always required: `(x) => x + 1`. The form `x => x + 1` is a syntax error.

### 4.4 Checked Error Types (`throws`)

**Addition.** TypeScript provides no mechanism to express in the type system that a function may throw. This proposal adds an optional `throws` clause:

```ts
function divide(x: int, y: int): int throws DivideByZero {
  if y == 0 { throw DivideByZero() }
  x / y
}
```

The `throws` clause appears after the return type in both declarations and function types:

```ts
// Declaration
function fetch(url: string): Response throws NetworkError | TimeoutError {
  ...
}

// Function type
(string) => Response throws NetworkError | TimeoutError

// Higher-order function accepting a fallible callback
function withRetry(
  f: (string) => Response throws NetworkError,
  retries: int
): Response throws TimeoutError {
  ...
}
```

Multiple error types are separated by `|` (union syntax).

The `throws` clause is valid only on braced forms — declarations and block-body lambdas. Expression-body lambdas cannot carry `throws` annotations. Rationale: if a function is complex enough to have checked error types, it is complex enough to warrant braces. This restriction also avoids syntactic ambiguity in expression-body position.

### 4.5 Pattern Matching

This proposal introduces `match` expressions using `=>` for pattern arms, following the precedent established by Scala and Rust:

```ts
match status {
  200 => "OK"
  404 => "Not Found"
  code => "Unknown: " + code.toString()
}
```

The `=>` token is used for both lambda expressions and match arms. Context always disambiguates: within a `match` block, `=>` separates a pattern from its result expression; outside a `match` block, `=>` introduces a lambda body.

When a match arm produces a lambda, both uses appear on the same line:

```ts
match mode {
  "add" => (x: int) => x + 1
  "sub" => (x: int) => x - 1
}
```

This is the same nesting that Scala has supported since its introduction. The parenthesized parameter list of the lambda provides a clear visual boundary between the match arm separator and the lambda arrow.

---

## 5. Inherited Inconsistencies and Open Concerns

This section documents known syntactic inconsistencies inherited from TypeScript and presents both sides of the argument. These are the most substantive objections to this proposal and deserve rigorous treatment.

### 5.1 The Declaration/Type Notational Split

Function declarations use `:` for return types. Function types use `=>`:

```ts
// Declaration — colon
function add(x: int, y: int): int { x + y }

// Type — fat arrow
(int, int) => int
```

Under this proposal, the type of a function cannot be derived from its declaration by simply removing the name and body — the return type delimiter must also change from `:` to `=>`. The current `->` design does not have this problem.

**The case for accepting this inconsistency:**

1. In over ten years of TypeScript usage, this split has produced no linter rules, no commonly reported bugs, and no significant developer complaints.
2. The two forms appear in syntactically distinct positions (declaration vs. type annotation), so developers are unlikely to confuse them.
3. Resolving the inconsistency requires changing either the declaration syntax (departing from TypeScript) or the type syntax (departing from all established convention). Both options impose a familiarity cost.

**The case against accepting it:**

The absence of linter rules in TypeScript does not prove the absence of friction — it may indicate that the friction is ambient and normalized. TypeScript developers never had a choice; they learned the two forms because there was no alternative. For a new language, this is an opportunity to avoid inheriting a known wart rather than rationalizing it.

The current syntax's property — "remove the name and body to get the type" — is genuinely pedagogically valuable. It means one concept to teach instead of two. This proposal lacks an equivalently concise teaching story. The closest formulation is: "declarations use `:`, types use `=>`, and you learn which is which from context" — which is accurate but not elegant.

**Assessment.** This is the strongest argument against this proposal. Whether it is decisive depends on how much weight one places on syntactic elegance and teachability vs. compatibility with established practice. This proposal places the weight on compatibility, but acknowledges that reasonable designers may disagree.

### 5.2 The `=>` Overloading Problem

The `=>` token appears in three contexts: lambda expressions, function types, and match arms. While these are contextually unambiguous for the parser, the human readability cost is real.

Consider a match arm that returns a lambda:

```ts
match mode {
  "add" => (x: int) => x + 1
  "sub" => (x: int) => x - 1
}
```

Two `=>` tokens on one line with different semantic roles. The current syntax avoids this entirely by reserving `=>` for match arms and `->` for functions:

```
match mode {
  "add" => (x: int) -> x + 1
  "sub" => (x: int) -> x - 1
}
```

In the current design, the arrow symbol itself tells the reader which construct they are looking at. In this proposal, the reader must rely on surrounding context (parenthesized parameters, `match` block boundaries) to distinguish the two uses.

Scala has the same `=>` overloading and has not found it to be a significant usability issue. However, the existence of precedent does not settle the question of whether a new language should inherit the pattern when an alternative is available.

### 5.3 The Familiarity Argument Cuts Both Ways

This proposal argues that TypeScript familiarity is a significant asset. But this language is not TypeScript. It already departs in substantive ways:

- Implicit return (TypeScript requires `return`)
- No `this` binding distinction between forms
- No hoisting
- No `arguments` object
- Different primitive types (`int` vs `number`)
- `match` expressions
- `throws` clauses

A TypeScript developer encountering this language must already recalibrate their mental model. The question is whether `:` vs `->` for return types is a meaningful additional cost on top of all that — and it is difficult to argue that it is.

Meanwhile, this proposal asks the larger group (Python developers, two-thirds of users) to absorb the `:` return type convention when `->` would transfer directly from their Python experience. The proposal essentially asks the larger group to accommodate the smaller group's conventions.

**Counterpoint.** Many Python developers in our user base have prior TypeScript or JavaScript experience. The `:` convention is not alien to them — it is simply not their primary language's convention. And the structural changes Python developers must absorb (braces, `function`, type system) are large enough that the return type symbol is a marginal detail.

### 5.4 The AI Code Generation Argument Is Time-Limited

This proposal's AI compatibility argument is pragmatically compelling today but has a limited shelf life. LLMs adapt to new syntax quickly given even modest training data. Rust, Kotlin, and Swift all receive reasonable code generation despite none of them using TypeScript syntax. Within a year or two of this language having published documentation, example repositories, and open-source code, the AI generation advantage is expected to diminish substantially.

Baking a permanent syntax decision into the language to optimize for a transient property of current models is a questionable tradeoff. The syntax will outlive the current generation of language models by many years.

**Counterpoint.** The advantage may be transient, but the cost of TS-aligned syntax is also low — it is not a bad syntax, merely a familiar one. The question is whether the short-term benefit justifies choosing a syntax that is otherwise acceptable. If the TS-aligned syntax were actively harmful, the time-limited nature of the AI argument would be decisive. Since it is merely "less elegant," the transient benefit still has value.

### 5.5 Long-Term Trajectory

If this language adopts TypeScript syntax now for familiarity, and subsequently diverges further from TypeScript — as new languages invariably do — the result may be a language that inherits TypeScript's inconsistencies without the compensating benefit of proximity to TypeScript. The familiarity argument depreciates as the language develops its own identity.

The current design does not have this problem. Its consistency is an intrinsic property of the syntax, not a relational one. It remains consistent regardless of how far the language diverges from TypeScript.

**Counterpoint.** All design decisions are made in context. A future language evolution that renders the TS alignment moot can also introduce syntax changes at that time. Optimizing for the present user base and current adoption landscape is not unreasonable, provided the syntax is not actively harmful.

---

## 6. Cross-Language Comparison

The following tables compare function syntax across languages likely to be familiar to our user base.

### 6.1 Declarations and Types

| Language       | Declaration                                  | Function Type           |
| -------------- | -------------------------------------------- | ----------------------- |
| **TypeScript** | `function f(x: number): number { return x }` | `(x: number) => number` |
| **Python**     | `def f(x: int) -> int: return x`             | `Callable[[int], int]`  |
| **Kotlin**     | `fun f(x: Int): Int { return x }`            | `(Int) -> Int`          |
| **Scala**      | `def f(x: Int): Int = x`                     | `Int => Int`            |
| **Swift**      | `func f(x: Int) -> Int { return x }`         | `(Int) -> Int`          |
| **Go**         | `func f(x int) int { return x }`             | `func(int) int`         |
| **Ours**       | `function f(x: int): int { x }`              | `(int) => int`          |

### 6.2 Lambdas and Higher-Order Functions

| Language       | Lambda                                     | HOF Parameter              |
| -------------- | ------------------------------------------ | -------------------------- |
| **TypeScript** | `(x) => x + 1`                             | `f: (x: number) => number` |
| **Python**     | `lambda x: x + 1`                          | `f: Callable[[int], int]`  |
| **Kotlin**     | `{ x -> x + 1 }`                           | `f: (Int) -> Int`          |
| **Scala**      | `(x: Int) => x + 1`                        | `f: Int => Int`            |
| **Swift**      | `{ (x: Int) in x + 1 }`                    | `f: (Int) -> Int`          |
| **Go**         | `let foo = func(x int) int { return x+1 }` | `f func(int) int`          |
| **Ours**       | `(x) => x + 1`                             | `f: (int) => int`          |

This proposal produces the shortest syntactic distance from TypeScript of any option considered.

### 6.3 Match/When Arms

| Language | Match Arm Syntax |
|----------|-----------------|
| **Kotlin** | `x -> result` (in `when`) |
| **Scala** | `case x => result` |
| **Rust** | `x => result` |
| **Swift** | `case x: result` |
| **Ours** | `x => result` |

The use of `=>` for match arms follows Scala and Rust.

---

## 7. Tradeoff Analysis

### 7.1 Arguments For This Proposal

**Familiarity.** TypeScript developers constitute approximately one-third of the current user base. This proposal requires zero syntactic adjustment from them for function declarations, lambdas, and function types. The only new concepts are implicit return, `throws`, and `match` — all of which are additive.

**Documentation economy.** The language's function syntax can be specified as "TypeScript with the following exceptions," reducing the documentation surface. Each shared convention is a convention that needs no explanation.

**AI code generation.** Language models produce TypeScript-patterned code by default. Syntactic alignment reduces error rates in generated code in the near term, though this advantage is expected to diminish as language-specific training data becomes available (see Section 5.4).

**Python developer adaptation.** Python developers must learn braces, `function`, and a structural type system regardless of arrow choice. The difference between `:` and `->` for return types is minor relative to these structural changes. Additionally, many Python developers in our user base have prior TypeScript experience.

**Tooling bootstrapping.** Syntax highlighting, tree-sitter grammars, TextMate scopes, and editor integrations can be adapted from TypeScript tooling rather than designed from first principles.

### 7.2 Arguments For Keeping the Current (`->`) Syntax

**Syntactic consistency.** `->` everywhere means one concept to learn, one concept to teach. The type of a function is its declaration with the name and body removed. No symbol swapping, no contextual rules.

**Pedagogical clarity.** The current syntax has a one-liner teaching story: "remove the name and body — that's your type." This proposal's equivalent is "declarations use `:`, types use `=>`, and you learn which is which from context," which is accurate but not elegant.

**Readability in mixed contexts.** The current syntax's `->` for functions and `=>` for match arms gives the reader a visual signal about which construct they are in. This proposal's `=>` overloading requires the reader to rely on surrounding context.

**Better alignment with the larger user group.** Python developers (two-thirds of users) already know `->` for return types. The current syntax preserves this familiarity; this proposal does not.

**Long-term resilience.** The current syntax's consistency is an intrinsic property. It remains clean regardless of how far the language diverges from TypeScript. This proposal's familiarity advantage is relational and depreciates as the language develops its own identity.

**The burden of proof is backwards.** For a new language already making substantive semantic departures from TypeScript, the default should be to design clean syntax — not to inherit known inconsistencies and then justify them. The burden should be on accepting the declaration/type split, not on fixing it.

### 7.3 Assessment

This is a genuine design tension with no objectively correct resolution. The two proposals optimize for different properties:

| Property | Current (`->`) | This proposal |
|----------|---------|---------------|
| Internal consistency | Strong | Weak (inherits TS split) |
| Teachability | One concept | Two notations for one concept |
| TS developer onboarding | Small cost | Zero cost |
| Python developer familiarity | `->` transfers | `:` does not |
| AI compatibility (near-term) | Requires adaptation | Compatible by default |
| Long-term resilience | Intrinsic consistency | Relational familiarity (depreciates) |
| Readability in `match` + lambda | Clear (`->` vs `=>`) | Ambiguous (`=>` vs `=>`) |

This proposal places the highest weight on near-term adoption and practical compatibility. It accepts known inelegances in exchange for a lower barrier to entry. Whether that tradeoff is correct depends on whether one believes this language's long-term success is more constrained by initial adoption friction or by accumulated syntactic debt.

---

## 8. Comparison with Current Syntax

| Aspect | Current (`->` unified) | This proposal (TS-aligned) |
|--------|------------------------|----------------------------|
| Declaration | `function f(x: int) -> int { x }` | `function f(x: int): int { x }` |
| Lambda | `(x) -> x + 1` | `(x) => x + 1` |
| Function type | `(int) -> int` | `(int) => int` |
| Match arm | `pattern => result` | `pattern => result` |
| Return type consistency | Same symbol everywhere (`->`) | `:` in declarations, `=>` in types |
| TS developer familiarity | Small adjustment required | No adjustment required |
| Python developer familiarity | `->` matches Python return types | `:` does not match |
| AI code generation | Requires adaptation | Compatible by default |
| Type derivable from declaration | Yes (remove name and body) | No (must also change `:` to `=>`) |
| Arrow symbols in language | Two (`->` for functions, `=>` for match) | One (`=>` for all) |

---

## 9. Open Questions

The following questions remain unresolved and should be addressed before this proposal advances beyond Draft status:

1. **Quantifying the AI claim.** Has code generation error rate been benchmarked with `->` vs `=>` syntax? The assertion that TS-aligned syntax produces fewer AI errors is plausible but unsubstantiated. If the error rate difference is negligible, this proposal loses one of its primary arguments.

2. **Teaching the declaration/type split.** How should documentation and tutorials explain the `:` vs `=>` split to beginners? The current syntax can say "remove the name and body — that's your type." This proposal needs an equivalently concise pedagogical framing.

3. **Addressing the long-term trajectory.** If the language continues to diverge from TypeScript over time, the familiarity argument weakens. What is the plan for a future in which this language no longer closely resembles TypeScript but still carries its syntactic inconsistencies?

---

## 10. Specification Summary

### 10.1 Complete Syntax

```ts
// ━━━ Named Functions ━━━

function add(x: int, y: int): int {
  x + y
}

function fetchData(url: string): Response throws NetworkError | TimeoutError {
  ...
}


// ━━━ Lambdas ━━━

// Expression body (types inferred)
(x) => x + 1
(a, b) => a + b

// Block body (implicit return)
(x: int) => {
  let y = x * 2
  y + 1
}


// ━━━ Function Types ━━━

(int) => int
(string, int) => Response
(url: string, retries: int) => Response throws NetworkError


// ━━━ Higher-Order Functions ━━━

function apply(f: (int) => int, value: int): int {
  f(value)
}

function compose(f: (A) => B, g: (B) => C): (A) => C {
  (x) => g(f(x))
}


// ━━━ Pattern Matching ━━━

match status {
  200 => "OK"
  404 => "Not Found"
  code => "Unknown: " + code.toString()
}


// ━━━ Chaining ━━━

items
  .filter((x) => x.active)
  .map((x) => x.name)
  .sort((a, b) => a.compareTo(b))
```

### 10.2 Deviations from TypeScript

| # | Change | Rationale |
|---|--------|-----------|
| 1 | Implicit return (last expression is the block's value) | Eliminates silent `undefined` return bugs |
| 2 | Two function forms only (`function` and `=>`) | Removes behavioral variants (`this`, hoisting, `arguments`) |
| 3 | Parentheses always required on lambda parameters | Eliminates `arrow-parens` style inconsistency |
| 4 | `throws` clause (optional, braced forms only) | Enables checked error types in the type system |
| 5 | `match` expressions with `=>` arms | Adds pattern matching (Scala/Rust precedent) |

### 10.3 Syntax Retained from TypeScript

All other function-related syntax — the `function` keyword, `:` for parameter and return type annotations, `=>` for lambdas and function types, brace-delimited bodies, and higher-order function composition — is adopted without modification.
