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

class ClassForNullLiteral {
  a "hi"
}

function FnOutputLiteralOrNull(s: string) -> ClassForNullLiteral {
    client GPT35
    prompt #"
        Return the empty object: {}.
    "#
}

test TestOutputLiteralOrNull {
    functions [FnOutputLiteralOrNull]
    args { s "unused" }
}
