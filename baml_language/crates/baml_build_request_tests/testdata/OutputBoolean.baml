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

function FnOutputBoolean(input: string) -> bool {
  client GPT35
  prompt #"
    Return "true"
  "#
}

test TestOutputBoolean {
  functions [FnOutputBoolean]
  args {
    input "example input"
  }
}
