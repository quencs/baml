// Agent tool calls.

type Tool = ReadFile | CreateFile | PatchFile | Shell

type ProjectPath = baml.fs.Path @description("File path relative to project root")

class ReadFile {
  type "read_file"
  path ProjectPath
}

class CreateFile {
  type "create_file"
  path ProjectPath
  contents string @description("new file contents")
}

class PatchFile {
  type "patch_file"
  path ProjectPath
  patch_contents string @description("diff-style patch to apply")
}

class Shell {
  type "shell"
  command string
}

// Tool call results
class ToolResult {
  tool_type string

  /// Unix timestamp of the tool result.
  timestamp int
  contents string
}

function execute_tool(tool_call: Tool) -> ToolResult {

  let timestamp = baml.sys.now_unix_timestamp();

  // Execute tools, assume happy path in each arm but handle exceptions
  // uniformly after the match, because we'll do the same thing for any
  // exception.
  let contents = match (tool_call) {
    ReadFile { path, .. } => {
      baml.fs.read_file(path)
    },
    CreateFile { path, contents, .. } => {
      baml.fs.write_file(path, contents);
      "success"
    },
    PatchFile { path, patch_contents, .. } => {
      baml.fs.patch_file(path, patch_contents);
      "success"
    },
    Shell { command, .. } => {
      baml.sys.shell(command)
    }
  } catch (e: Exception) {
    "Error: " + baml.to_string(e)
  };

  ToolResult {
    tool_type: tool_call.type,
    tool_result_contents,
    timestamp,
  }

}
