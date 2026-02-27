function ChooseTool(chat_context: ChatContext) -> Tool {

  client "anthropic/claude-opus-4.6"
  prompt #"
    You are baude-code, a Claude Code clone built to help founders ship unbreakable agents.
    (You're a coding assistant.)

    Choose a tool to help the user satisfy their goal.

    This sessions's history:
    {% for message in chat_context.messages %}
      {{ m.type }}
      {{ m.timestamp }}
      {{ m.contents }}
    {% endfor %}

    {{ ctx.output_format }}
  "#

}

class ChatContext {
  memory_files: map<baml.fs.Path, string>,
  messages: Message[]
}

type Message = UserMessage | ToolResult

class UserMessage {
  type "user_message"
  timestamp int
  contents string
}
