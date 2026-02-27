// Baude Code — The core LLM function
//
// DecideAction takes the full conversation context and returns exactly one
// AgentAction: either a tool call (ReadFile, WriteFile, Bash) or a direct
// text response (Respond).

function DecideAction(context_files: string, messages: Message[]) -> AgentAction {
    client Claude
    prompt #"
        {{ SystemPrompt(context_files, ToolDescriptions()) }}

        {{ ConversationHistory(messages) }}
    "#
}
