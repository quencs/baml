
function LlmEcho(input: string) -> string {
  // see clients.baml
  client "openai/gpt-4o-mini"

  // The prompt uses Jinja syntax. Change the models or this text and watch the prompt preview change!
  prompt #"
    Repeat the following text back to me: {{ input }}
  "#
}


function MyFunction5(flag: bool) -> string {
  //# Check
  let result = if (flag) {
    "then-branch"
  } else {
    "else-branch"
  };

  //# Extract1
  watch let item = "before";
  LlmEcho(item);

  //# Extract2
  item = "extract2";
  LlmEcho(item);

  //# Extract3
  item = "extract3";
  LlmEcho(item);

  //# Extract4
  item = "extract4";
  LlmEcho(item);

  item
}

function Main() -> string {
  MyFunction5(true)
}