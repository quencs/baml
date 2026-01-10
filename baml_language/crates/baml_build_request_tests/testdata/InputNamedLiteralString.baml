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

function FnInputNamedLiteralString(myString: "My String") -> string {
  client GPT35
  prompt #"
    Return this value back to me: {{myString}}
  "#
}

test TestInputNamedLiteralString {
  functions [FnInputNamedLiteralString]
  args {
    myString "My String"
  }
}
