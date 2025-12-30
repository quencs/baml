function Foo() -> int {
  1 + "foo"
}

// Test case for info_span: type annotation shows where expectation came from
function Bar() -> int {
  let x: string = 123
  0
}
