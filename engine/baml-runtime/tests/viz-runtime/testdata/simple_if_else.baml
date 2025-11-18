function MyFunction1(flag: bool) -> string {
  //# Check
  let result = if (flag) {
    "then-branch"
  } else {
    "else-branch"
  };

  watch let tagged = "branch:" + result;

  tagged
}

function Main() -> string {
  MyFunction1(true)
}

test default {
  functions [Main]
  args { }
}
