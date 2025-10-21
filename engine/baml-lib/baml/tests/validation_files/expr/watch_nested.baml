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
