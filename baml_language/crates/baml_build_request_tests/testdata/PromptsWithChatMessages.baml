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

client<llm> Claude {
  provider anthropic
  options {
    model claude-3-haiku-20240307
    api_key env.ANTHROPIC_API_KEY
    max_tokens 1000
  }
}

function FnPromptsWithChatMessages(input: string) -> string {
  client GPT35
  prompt #"
    {{ _.role("system") }}
    You are an assistant that always responds in a very excited way with emojis and also outputs this word 4 times after giving a response: {{ input }}

    {{ _.role("user") }}
    Tell me a haiku about {{ input }}
  "#
}

function PromptTestOpenAIChatNoSystem(input: string) -> string {
  client GPT35
  prompt #"
    You are an assistant that always responds in a very excited way with emojis and also outputs this word 4 times after giving a response: {{ input }}

    {{ _.role("user") }}
    Tell me a haiku about {{ input }}
  "#
}

function PromptTestClaudeChat(input: string) -> string {
  client Claude
  prompt #"
    {{ _.role("system") }}
    You are an assistant that always responds in a very excited way with emojis and also outputs this word 4 times after giving a response: {{ input }}

    {{ _.role("user") }}
    Tell me a haiku about {{ input }}
  "#
}

function PromptTestClaudeChatNoSystem(input: string) -> string {
  client Claude
  prompt #"
    You are an assistant that always responds in a very excited way with emojis and also outputs this word 4 times after giving a response: {{ input }}

    {{ _.role("user") }}
    Tell me a haiku about {{ input }}
  "#
}

test TestPromptsWithChatMessages {
  functions [FnPromptsWithChatMessages, PromptTestOpenAIChatNoSystem, PromptTestClaudeChat, PromptTestClaudeChatNoSystem]
  args {
    input "cats"
  }
}

