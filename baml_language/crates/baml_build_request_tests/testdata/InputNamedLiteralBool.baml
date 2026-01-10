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

function FnInputNamedLiteralBool(myBool: true) -> string {
  client GPT35
  prompt #"
    Return this value back to me: {{myBool}}
  "#
}

test TestInputNamedLiteralBool {
  functions [FnInputNamedLiteralBool]
  args {
    myBool true
  }
}
