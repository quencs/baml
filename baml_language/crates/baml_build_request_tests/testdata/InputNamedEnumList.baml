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

enum NamedArgsSingleEnumList {
  ONE
  TWO
}

function FnInputNamedEnumList(myArg: NamedArgsSingleEnumList[]) -> string {
  client GPT35
  prompt #"
    Print these values back to me:
    {{myArg}}
  "#
}

test TestInputNamedEnumList {
  functions [FnInputNamedEnumList]
  args {
    myArg [ONE, TWO]
  }
}
