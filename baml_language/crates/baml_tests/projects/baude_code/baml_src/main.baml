function main() {

  let agents_md_contents = baml.fs.read_file("AGENTS.MD") catch (_) {
    ""
  };

  let claude_md_contents = baml.fs.read_file("CLAUDE.MD") catch (_) {
    ""
  };

  let chat_context = ChatContext {
    memory_files: { "AGENTS.MD": agents_md_contents, "CLAUDE.MD": claude_md_contents },
    messages: [],
  };

  while (true) {

    let user_line = baml.sys.read_line();
    if (user_line == "/quit") {
      break;
    }

    chat_context.messages.push(UserMessage {
      contents: user_line,
      timestamp: baml.sys.now_unix_timestamp(),
    };

    let agent_finished = false;

    while (!agent_finished) {

      let tool = ChooseTool(chat_context);
      let tool_result = execute_tool(tool);

      if tool.type == "respond" {
        agent_finished = true;
      }

      chat_context.messages.push(tool_result);

  }


}
