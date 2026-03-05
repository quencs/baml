# Type Simplification Tests

## Format

Each test has BAML source, a target path (`ClassName.field_name`), and
expected simplified type `Display` output.

- **Materialized**: the simplified type for non-streaming codegen.
- **Streaming**: placeholder (mirrors Materialized until streaming simplification lands).

---

# Primitives

## string_field

```baml
class T { f string }
```

### target: `T.f`

- Materialized: `string`

---

## int_field

```baml
class T { f int }
```

### target: `T.f`

- Materialized: `int`

---

## float_field

```baml
class T { f float }
```

### target: `T.f`

- Materialized: `float`

---

## bool_field

```baml
class T { f bool }
```

### target: `T.f`

- Materialized: `bool`

---

## null_field

```baml
class T { f null }
```

### target: `T.f`

- Materialized: `null`

---

# Optionals

## optional_string

```baml
class T { f string? }
```

### target: `T.f`

- Materialized: `string | null`

---

## optional_int

```baml
class T { f int? }
```

### target: `T.f`

- Materialized: `int | null`

---

## double_optional

```baml
class T { f int?? }
```

### target: `T.f`

- Materialized: `int | null`

---

# Unions

## simple_union

```baml
class T { f int | string }
```

### target: `T.f`

- Materialized: `int | string`

---

## nullable_union

```baml
class T { f int | string | null }
```

### target: `T.f`

- Materialized: `int | string | null`

---

## optional_union

```baml
class T { f (int | string)? }
```

### target: `T.f`

- Materialized: `int | string | null`

---

## duplicate_union_members

```baml
class T { f int | int | string }
```

### target: `T.f`

- Materialized: `int | string`

---

## nested_union

```baml
class T { f (int | null) | string }
```

### target: `T.f`

- Materialized: `int | string | null`

---

## nested_union_with_repeats

```baml
class T { f int | (int | null) | string }
```

### target: `T.f`

- Materialized: `int | string | null`

---

## single_member_union

```baml
class T { f (int) }
```

### target: `T.f`

- Materialized: `int`

---

# Containers

## list_field

```baml
class T { f string[] }
```

### target: `T.f`

- Materialized: `string[]`

---

## optional_list

```baml
class T { f string[]? }
```

### target: `T.f`

- Materialized: `string[] | null`

---

## list_of_union

```baml
class T { f (int | string)[] }
```

### target: `T.f`

- Materialized: `int | string[]`

---

## map_field

```baml
class T { f map<string, int> }
```

### target: `T.f`

- Materialized: `map<string, int>`

---

## map_with_union_value

```baml
class T { f map<string, int | string> }
```

### target: `T.f`

- Materialized: `map<string, int | string>`

---

## optional_map

```baml
class T { f map<string, int>? }
```

### target: `T.f`

- Materialized: `map<string, int> | null`

---

# Class and Enum references

## class_field

```baml
class Inner { x int }
class T { f Inner }
```

### target: `T.f`

- Materialized: `Inner`

---

## optional_class

```baml
class Inner { x int }
class T { f Inner? }
```

### target: `T.f`

- Materialized: `Inner | null`

---

## enum_field

```baml
enum Color { Red Green Blue }
class T { f Color }
```

### target: `T.f`

- Materialized: `Color`

---

## optional_enum

```baml
enum Color { Red Green Blue }
class T { f Color? }
```

### target: `T.f`

- Materialized: `Color | null`

---

# Union of class and primitive

## class_or_string

```baml
class Inner { x int }
class T { f Inner | string }
```

### target: `T.f`

- Materialized: `Inner | string`

---

## class_or_null

```baml
class Inner { x int }
class T { f Inner | null }
```

### target: `T.f`

- Materialized: `Inner | null`

---

# Lists of classes

## list_of_class

```baml
class Inner { x int }
class T { f Inner[] }
```

### target: `T.f`

- Materialized: `Inner[]`

---

# Type aliases

## simple_alias

```baml
type MyInt = int
class T { f MyInt }
```

### target: `T.f`

- Materialized: `int`

---

## optional_alias

```baml
type MyInt = int
class T { f MyInt? }
```

### target: `T.f`

- Materialized: `int | null`

---

## union_alias

```baml
type IntOrString = int | string
class T { f IntOrString }
```

### target: `T.f`

- Materialized: `int | string`

---

## optional_union_alias

```baml
type IntOrString = int | string
class T { f IntOrString? }
```

### target: `T.f`

- Materialized: `int | string | null`

---

# Deeply nested

## nested_optional_union

```baml
class T { f ((int | null) | string)? }
```

### target: `T.f`

- Materialized: `int | string | null`

---

## all_null_union

```baml
class T { f null | null }
```

### target: `T.f`

- Materialized: `null`

---

# Media types

## image_field

```baml
class T { f image }
```

### target: `T.f`

- Materialized: `image`

---

## optional_image

```baml
class T { f image? }
```

### target: `T.f`

- Materialized: `image | null`

---

## audio_field

```baml
class T { f audio }
```

### target: `T.f`

- Materialized: `audio`

---

# Recursive and self-referencing types

## self_referencing_class_list

```baml
class Node { children Node[] }
```

### target: `Node.children`

- Materialized: `Node[]`

---

## self_referencing_class_optional

```baml
class Node { parent Node? }
```

### target: `Node.parent`

- Materialized: `Node | null`

---

## recursive_type_alias

```baml
type Json = float | int | bool | string | Json[] | map<string, Json>
class T { f Json }
```

### target: `T.f`

- Materialized: `Json`

---

## optional_recursive_alias

```baml
type Json = float | int | bool | string | Json[] | map<string, Json>
class T { f Json? }
```

### target: `T.f`

- Materialized: `Json | null`

---

## recursive_alias_in_union

Recursive aliases are opaque (`TypeAlias`) after conversion — the simplifier
cannot look inside `Json` to see that it contains `int`, so `int` is NOT
absorbed. This is correct: expanding a recursive alias would loop forever.

```baml
type Json = float | int | bool | string | Json[] | map<string, Json>
class T { f Json | int }
```

### target: `T.f`

- Materialized: `Json | int`

---

## recursive_alias_in_list

```baml
type Json = float | int | bool | string | Json[] | map<string, Json>
class T { f Json[] }
```

### target: `T.f`

- Materialized: `Json[]`

---

## mutual_class_reference

```baml
class A { b B? }
class B { a A? }
```

### target: `A.b`

- Materialized: `B | null`
