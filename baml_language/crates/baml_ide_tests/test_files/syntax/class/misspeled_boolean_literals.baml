class TestLiterals {
  a "SingleLiteral"
  b "boolean" | True | False
}

// error: Type `True` does not exist. Did you mean one of these: `true`, `string`, `int`, `bool`, `false`, `float`, `TestLiterals`?
//   -->  class/misspeled_boolean_literals.baml:3
//    | 
//  2 |   a "SingleLiteral"
//  3 |   b "boolean" | True | False
//    | 
// error: Type `False` does not exist. Did you mean one of these: `false`, `float`, `true`, `int`, `bool`, `string`, `TestLiterals`?
//   -->  class/misspeled_boolean_literals.baml:3
//    | 
//  2 |   a "SingleLiteral"
//  3 |   b "boolean" | True | False
//    |

//----
//- diagnostics
// Error: Unknown type `True`. Did you mean `true`?
//    ╭─[ class_misspeled_boolean_literals.baml:3:17 ]
//    │
//  3 │   b "boolean" | True | False
//    │                 ──┬─  
//    │                   ╰─── Unknown type `True`. Did you mean `true`?
//    │ 
//    │ Note: Error code: E0002
// ───╯
// Error: Unknown type `False`. Did you mean `false`?
//    ╭─[ class_misspeled_boolean_literals.baml:3:24 ]
//    │
//  3 │   b "boolean" | True | False
//    │                        ──┬──  
//    │                          ╰──── Unknown type `False`. Did you mean `false`?
//    │ 
//    │ Note: Error code: E0002
// ───╯
