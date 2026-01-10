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



function FnInputNamedClassList(myArg: string[]) -> string[] {
  client GPT35
  prompt #"
    Return this value back to me: {{myArg}}
  "#
}

test TestInputNamedClassList {
  functions [FnInputNamedClassList]
  args {
    myArg ["hello", "world"]
  }
}
