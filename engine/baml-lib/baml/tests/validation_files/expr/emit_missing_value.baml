function WatchValid() -> int {
    watch progress: int = 0;
    progress.$watch.notify();
    progress
}

// error: Error validating: Can only access fields on class instances
//   -->  expr/emit_missing_value.baml:3
//    | 
//  2 |     watch progress: int = 0;
//  3 |     progress.$watch.notify();
//    | 
