retry_policy GPT4oRetry {
  max_retries 2
  strategy {
    type exponential_backoff
  }
}

client<llm> GPT35 {
  provider openai
  retry_policy GPT4oRetry
  options {
    model "gpt-4o-mini"
    api_key env.OPENAI_API_KEY
  }
}

class NamedArgsSingleClass {
  key string
  key_two bool
  key_three int
  // TODO: doesn't work with keys with numbers
  // key2 bool
  // key3 int
}

function FnInputNamedClass(myArg: NamedArgsSingleClass) -> string {
  client GPT35
  prompt #"
    Print these values back to me:
    {{myArg.key}}
    {{myArg.key_two}}
    {{myArg.key_three}}
  "#
}

test TestInputNamedClass {
  functions [FnInputNamedClass]
  args {
    myArg {
      key "example",
      key_two true,
      key_three 42
    }
  }
}

function TestMulticlassNamedArgs(myArg: NamedArgsSingleClass, myArg2: NamedArgsSingleClass) -> string {
  client GPT35
  prompt #"
    Print these values back to me:
    {{myArg.key}}
    {{myArg.key_two}}
    {{myArg.key_three}}
    {{myArg2.key}}
    {{myArg2.key_two}}
    {{myArg2.key_three}}
  "#
}
