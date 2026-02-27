// Baude Code — Core type definitions

// ---------------------------------------------------------------------------
// Tool call types: The LLM returns one of these on each turn.
// ---------------------------------------------------------------------------

class ReadFile {
    path string @description("Path of the file to read")
}

class WriteFile {
    path string @description("Path of the file to write")
    content string @description("Content to write to the file")
}

class Bash {
    command string @description("Shell command to execute")
}

class Respond {
    text string @description("Text response to show the user")
}

// The union that the LLM picks from each turn
type AgentAction = ReadFile | WriteFile | Bash | Respond

// ---------------------------------------------------------------------------
// Conversation history
// ---------------------------------------------------------------------------

type Role = "user" | "assistant" | "tool"

class Message {
    role Role
    content string
}

// ---------------------------------------------------------------------------
// Tool execution result
// ---------------------------------------------------------------------------

class ToolResult {
    tool_name string
    success bool
    output string
}
