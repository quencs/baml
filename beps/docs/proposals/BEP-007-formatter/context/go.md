# gofmt Design

## Summary

gofmt is Go's official code formatter and one of the most influential formatters in modern programming. It takes an AST-based approach with a deliberately minimalist philosophy: it operates on the AST to enable semantic transformations (like import grouping), but refuses to enforce line length limits or provide customization options. Instead, gofmt preserves developer's intentional line breaks while standardizing indentation and spacing. This "opinionated but hands-off" approach makes gofmt extremely simple and predictable, though it can result in inconsistent line breaking across codebases.

## Key Design Decisions

### 1. Operate Directly on the AST

gofmt operates directly on Go's Abstract Syntax Tree (AST) using a collection of simple, pre-defined rules. This approach enables semantics-preserving modifications like:

- Grouping and sorting imports (standard library, third-party, local)
- Simplifying composite literals with `-s` flag (e.g., `[]T{T{}}` → `[]T{{}}`)
- Normalizing slice expressions (e.g., `s[a:len(s)]` → `s[a:]`)

The formatter's rule set is much simpler than rustfmt's, reflecting Go's simpler syntax and preference for minimalism.

### 2. No Maximum Line Length

**gofmt does not enforce any maximum line length.** This is a deliberate design decision:

- The Go team believes automatic line wrapping often produces suboptimal results
- Developers are trusted to break long lines manually
- Not having a limit makes behavior more predictable

This means gofmt allows arbitrarily long lines:

```go
result := calculateSomethingVeryComplex(argument1, argument2, argument3, argument4, argument5, argument6, argument7, argument8)
```

For line length enforcement, third-party tools like [golines](https://github.com/segmentio/golines) must be used.

### 3. Preserving Developer Intention

gofmt **preserves intentional formatting decisions**. Manual line breaks are respected while indentation is standardized:

```go
func main() {
	// Developer manually broke this array across multiple lines
	arr := []int{1, 2, 3,
		4, 5,
		6,
	}

	fmt.Println(arr)
}
```

After gofmt, line breaks remain exactly where placed. This means:

- gofmt **never adds** line breaks automatically
- gofmt **preserves** manual line breaks
- gofmt **only** standardizes whitespace (tabs/spaces, blank lines)

### 4. Zero Configuration

**gofmt has essentially no configuration options.** There is no config file, no command-line flags for customization:

- Indentation is always tabs (non-configurable)
- Tab width cannot be set (determined by your editor)
- No options for brace style, spacing, or any formatting rules
- The only flag is `-s` for optional simplifications

This eliminates all bikeshedding and ensures universal consistency across Go codebases. You cannot make gofmt format code differently - every Go project uses identical formatting.

### 5. Comment Handling

At parse-time comments are scanned for and attached to AST nodes:

- Comments generally stay in their original positions
- Rarely deletes comments, even in unusual positions

## Tradeoffs

### Advantages
- **Extreme simplicity**: Very few rules, easy to implement and maintain
- **Total predictability**: Output is always obvious and unsurprising
- **Zero configuration**: No debates, no config files, universal consistency
- **Fast execution**: Minimal logic makes it extremely fast
- **Wide adoption**: ~100% of Go code uses gofmt, creating ecosystem-wide consistency
- **Respects intent**: Preserves developer line-breaking decisions

### Disadvantages
- **No line length enforcement**: Allows arbitrarily long lines that harm readability
- **Manual breaking required**: Developers must break long lines themselves
- **Inconsistent line breaking**: Different developers break lines differently
- **No optimization**: Won't automatically format code in the most readable way
- **Inflexible**: Absolutely no customization possible (can be pro or con)
- **Requires discipline**: Teams must establish conventions for manual line breaks