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

// string[]
function FnInputNamedStringList(myStringArray: string[]) -> string {
  client GPT35
  prompt #"
    Return this value back to me: {{myStringArray}}
  "#
}

test TestInputNamedStringList {
  functions [FnInputNamedStringList]
  args {
    myStringArray ["example1", "example2", "example3"]
  }
}
