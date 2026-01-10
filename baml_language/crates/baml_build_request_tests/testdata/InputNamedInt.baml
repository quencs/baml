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

// test for int
function FnInputNamedInt(myInt: int) -> string {
  client GPT35
  prompt #"
    Return this value back to me: {{myInt}}
  "#
}

test TestInputNamedInt {
  functions [FnInputNamedInt]
  args {
    myInt 42
  }
}
