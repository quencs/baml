// Simple filter function that only notifies when value is not empty
function NotEmpty(value: string) -> bool {
  value != ""
}

// Simple test without loops
function SimpleWatchWithFilter() -> int {
  //# Section1
  watch let word: string = "";
  word.$watch.options(baml.WatchOptions{ when: NotEmpty });

  //## Section2
  word = "hello";  // Should notify (not empty)
  //# Section3
  word = "world";  // Should notify (not empty)
  word = "";       // Should NOT notify (empty)
  word = "test";   // Should notify (not empty)
  
  //# Section4

  word.$watch.options(baml.WatchOptions{ channel: "new_name" });
  word = "with_new_name";   // Should notify (not empty)
  word.$watch.notify();

  //# SectionIf
  let x = if (false) {
    //# Section42
    42
  } else if (true) {
    //# Section41
    41
  } else {
    40
  };

  x
}

test TestSimpleWatchWithFilter() {
  functions [SimpleWatchWithFilter]
  args {}
}
