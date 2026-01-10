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

function FnOutputInt(input: string) -> int {
  client GPT35
  prompt #"
    Return the integer 5 with no additional context.
  "#
}

test TestOutputInt {
  functions [FnOutputInt]
  args {
    input "example input"
  }
}
