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

class DummyOutput {
  nonce string
  nonce2 string
  @@dynamic
}

function FnOutputSerializationError(input: string) -> DummyOutput {
  client GPT35
  prompt #"
    Say "hello there".
  "#
}

test TestOutputSerializationError {
  functions [FnOutputSerializationError]
}
