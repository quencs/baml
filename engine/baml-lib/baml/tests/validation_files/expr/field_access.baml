let x = obj.field;
let y = person.name;
let z = nested.inner.value;
let w = get_object().property;
let a = arr[0].field;
let b = obj.field[0];
let c = map["key"].property;

// warning: Variable assignment is experimental, and will break in the future.
//   -->  expr/field_access.baml:1
//    | 
//    | 
//  1 | let x = obj.field;
//    | 
//

// warning: Variable assignment is experimental, and will break in the future.
//   -->  expr/field_access.baml:2
//    | 
//    | 
//  2 | let y = person.name;
//    | 
//

// warning: Variable assignment is experimental, and will break in the future.
//   -->  expr/field_access.baml:3
//    | 
//    | 
//  3 | let z = nested.inner.value;
//    | 
//

// warning: Variable assignment is experimental, and will break in the future.
//   -->  expr/field_access.baml:4
//    | 
//    | 
//  4 | let w = get_object().property;
//    | 
//

// warning: Variable assignment is experimental, and will break in the future.
//   -->  expr/field_access.baml:5
//    | 
//    | 
//  5 | let a = arr[0].field;
//    | 
//

// warning: Variable assignment is experimental, and will break in the future.
//   -->  expr/field_access.baml:6
//    | 
//    | 
//  6 | let b = obj.field[0];
//    | 
//

// warning: Variable assignment is experimental, and will break in the future.
//   -->  expr/field_access.baml:7
//    | 
//    | 
//  7 | let c = map["key"].property;
//    | 
//