// Simple filter function that only notifies when value is not empty
function NotEmpty(prev: string, next: string) -> bool {
  next != ""
}

// Simple test without loops
function SimpleWatchWithFilter() -> int {
  watch let word: string = "";
  word.$watch.options(baml.WatchOptions{ when: NotEmpty });

  word = "hello";  // Should notify (not empty)
  word = "world";  // Should notify (not empty)
  word = "";       // Should NOT notify (empty)
  word = "test";   // Should notify (not empty)

  word.$watch.options(baml.WatchOptions{ name: "new_name" });
  word = "with_new_name";   // Should notify (not empty)

  42
}

test TestSimpleWatchWithFilter() {
  functions [SimpleWatchWithFilter]
  args {}
}
