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

client<llm> Sonnet {
  provider anthropic
  options {
    model claude-sonnet-4-5-20250929
    api_key env.ANTHROPIC_API_KEY
  }
}

function PromptTestClaude(input: string) -> string {
  client Sonnet
  prompt #"
    Tell me a haiku about {{ input }}
  "#
}


function FnPromptsNoChatMessages(input: string) -> string {
  client GPT35
  // Keep this one longer to test streaming performance
  prompt #"
    Tell me a short story about {{ input }}.
  "#
}

test TestPromptsNoChatMessages {
  functions [FnPromptsNoChatMessages]
  args {
    input #"
      hello world
    "#
  }
}
