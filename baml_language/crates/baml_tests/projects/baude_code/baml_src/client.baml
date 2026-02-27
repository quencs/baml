// Baude Code — LLM client configuration

retry_policy DefaultRetry {
    max_retries 2
}

client<llm> Claude {
    provider anthropic
    retry_policy DefaultRetry
    options {
        model "claude-sonnet-4-20250514"
        max_tokens 4096
        api_key env.ANTHROPIC_API_KEY
    }
}
