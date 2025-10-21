function WorkflowWatch() -> int {
  watch x: int = 10;
  watch y: bool = true;

  y = false;
  x = WorkflowWatchChild();
  x
}

function WorkflowWatchChild() -> int {
  watch x: string = "Hello";
  100
}

// error: Error validating: This line is invalid. It does not start with any known Baml schema keyword.
//   -->  expr/watch_nested.baml:1
//    | 
//    | 
//  1 | function WorkflowWatch() -> int {
//  2 |   watch x: int = 10;
//    | 
