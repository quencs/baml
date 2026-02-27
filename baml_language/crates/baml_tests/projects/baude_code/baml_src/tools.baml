// Baude Code — Tool execution

function ExecuteReadFile(action: ReadFile) -> ToolResult {
    let content = baml.fs.open(action.path).read();
    ToolResult {
        tool_name: "ReadFile",
        success: true,
        output: content
    }
}

function ExecuteWriteFile(action: WriteFile) -> ToolResult {
    // No native file-write builtin yet; shell workaround.
    // Use a heredoc so content with quotes and special chars is safe.
    let cmd = "cat > " + action.path + " <<'BAUDE_EOF'\n" + action.content + "\nBAUDE_EOF";
    baml.sys.shell(cmd);
    ToolResult {
        tool_name: "WriteFile",
        success: true,
        output: "Wrote to " + action.path
    }
}

function ExecuteBash(action: Bash) -> ToolResult {
    let output = baml.sys.shell(action.command);
    ToolResult {
        tool_name: "Bash",
        success: true,
        output: output
    }
}

function ExecuteTool(action: AgentAction) -> ToolResult {
    match (action) {
        rf: ReadFile  => ExecuteReadFile(rf),
        wf: WriteFile => ExecuteWriteFile(wf),
        b: Bash       => ExecuteBash(b),
        r: Respond    => ToolResult {
            tool_name: "Respond",
            success: true,
            output: r.text
        }
    }
}
