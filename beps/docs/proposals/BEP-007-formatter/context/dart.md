# dartfmt Design

## Summary

dartfmt is Dart's official code formatter that uses a sophisticated algebraic approach combined with best-first graph search to find optimal line breaks. Unlike simple Wadler-style formatters that greedily break lines, dartfmt explores the solution space of possible line-break combinations to minimize both line overflow and unnecessary breaks. This approach requires significant implementation complexity and careful optimization to avoid exponential runtime.

## Key Design Decisions

### 1. Intermediate Representation with Chunks, Rules, and Spans

Rather than pure Wadler documents or direct AST reprinting, dartfmt uses a custom IR with three interconnected abstractions:

- **Chunks**: Atomic units of formatting—contiguous character sequences guaranteed to contain no line breaks. These are the building blocks of formatted code.

- **Rules**: Control where splits can occur and assign each potential split a "value" (cost). Higher values mean the formatter prefers not to break there. Think of rules like dials that can be turned to different positions.

- **Spans**: Mark series of contiguous chunks that should be kept together when possible. They function like "rubber bands" around code regions, resisting splitting. This is the key abstraction for expressing natural grouping (e.g., keeping a short function call on one line).

### 2. Line Length Enforcement

Unlike gofmt which has no line limit, dartfmt **actively enforces line length constraints** (typically 80 columns). This is the primary source of complexity—the formatter must find combinations of line breaks that keep code within the limit while minimizing unnecessary breaks.

### 3. Best-First Graph Search

The core algorithm uses **best-first graph search** rather than greedy heuristics or dynamic programming. The solution space is modeled as a graph where:

- Each **node** represents a partial solution with some rules bound to specific values (split or don't split)
- **Edges** connect partial solutions by binding one additional rule
- Search proceeds in order of **increasing cost**, prioritizing solutions likely to be optimal

This approach explores multiple formatting possibilities simultaneously rather than committing to the first option that works.

### 4. Combinatorial Explosion Problem

The fundamental challenge is that even small expressions have thousands of possible line-break combinations. An example given by the developer was a place with 13 possible line breaks yields **8,192 different combinations** if brute-forced. For larger expressions, the search space grows exponentially.

### 5. Three Key Optimizations

To make graph search practical, dartfmt uses three critical heuristics:

**a) Early termination**: Stop immediately upon finding any solution that fits within the line limit. The first valid solution found is often good enough.

**b) Focused rule binding**: Only modify rules affecting lines that overflow the limit. Don't touch rules on lines that already fit—they're already optimal.

**c) Branch pruning**: Discard entire solution branches when one partial solution provably dominates another (is better in all respects). This dramatically reduces the search space.

### 6. Two-Phase Cost Calculation

The formatter optimizes two metrics independently:

1. **Primary**: Minimize overflow characters (total characters exceeding the line limit)
2. **Secondary**: Among solutions with equal overflow, minimize rule-split costs (prefer keeping related code together)

Spans are the primary mechanism for expressing which code naturally belongs together.

## Tradeoffs

### Advantages
- **Optimal line breaking**: Explores multiple options to find better formatting than greedy algorithms
- **Line length enforcement**: Actively keeps code within configured width
- **Context-aware decisions**: Can look ahead to find globally optimal solutions
- **Sophisticated cost model**: Spans allow expressing natural code grouping elegantly
- **Consistent output**: Same input always produces same output (deterministic search)
- **Algebraic foundation**: Based on solid CS principles (graph search, cost optimization)

### Disadvantages
- **Extreme implementation complexity**: The author describes it as "the hardest program I've ever written"
- **Performance concerns**: Graph search can be slow without aggressive optimization
- **Difficult to debug**: Non-obvious why specific formatting choices were made
- **Hard to maintain**: Complex algorithms require deep understanding to modify
- **Edge case handling**: Combinatorial explosion requires careful heuristics
- **No semantic transformations**: Unlike AST formatters, cannot reorder imports or simplify syntax
- **Tuning required**: Cost functions and heuristics require experimentation to get right