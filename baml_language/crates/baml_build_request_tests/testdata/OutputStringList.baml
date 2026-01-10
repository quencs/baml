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

function FnOutputStringList(input: string) -> string[] {
  client GPT35
  prompt #"
    Return a list of strings in json format like ["string1", "string2", "string3"].

    JSON:
  "#
}

test TestOutputStringList {
  functions [FnOutputStringList]
  args {
    input "example input"
  }
}
