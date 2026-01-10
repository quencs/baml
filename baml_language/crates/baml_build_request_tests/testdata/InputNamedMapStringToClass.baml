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

class StringToClassEntry {
  word string
}
// test string
function FnInputNamedMapStringToClass(myMap: map<string, StringToClassEntry>) -> map<string, StringToClassEntry> {
  client GPT35
  prompt #"
    Return this value back to me: {{myMap}}
  "#
}

test TestInputNamedMapStringToClass {
  functions [FnInputNamedMapStringToClass]
  args {
    myMap {
      "key" {
        word "lorem ipsum"
      }
    }
  }
}
