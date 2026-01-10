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

function FnInputNamedLiteralInt(myInt: 1) -> string {
  client GPT35
  prompt #"
    Return this value back to me: {{myInt}}
  "#
}

test TestInputNamedLiteralInt {
  functions [FnInputNamedLiteralInt]
  args {
    myInt 1
  }
}
