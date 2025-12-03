
function MyFunction4(flag: bool) -> string {
  //# Check
  let result = if (flag) {
    "then-branch"
  } else {
    "else-branch"
  };

  watch let tagged = "branch:" + result;

  tagged
}

test default {
  functions [MyFunction4]
  args {
    flag true
  }
}