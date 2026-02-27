# Baude Code

A rewrite of Claude Code in BAML. This port is mainly for exercising
BAML function syntax and the ability to run a full agentic app without
an external harness (i.e. no Python/TS).

## Features

 - **Chat**. Interact with the agent through a chat interface. User
   queries come from stdin and agent responses are returned on stdout.
 - **Coding Assistant Tools**. The chatbot will issue tool calls to
   read files, edit files, and run shell commands.
 - **Context**. AGENTS.md, CLAUDE.md and BAUDE.md will be automatically
   read from the project directory on startup and used to initialize
   each session (So keep them short!).
 - **Memory**. Important facts about the codebase, and the user's
   session, are stored in MEMORY=$XDG_CONFIG/.baude-code/memory/$PROJECT_PATH.
   PROJECT_PATH is a canonicalized version of the a filesystem path of the
   current project. Conversations themselves should live under
   $MEMORY/TIMESTAMP. $MEMORY/index.jsonl should be a list of:
   { session_name: STRING, session_path: PATH, end_ts: UNIX_TIMESTAMP }
 - **Permissions**. The agent's ability to execute various commands
   (read file, write file, call bash) is controlled by combining the
   contents of files from $XDG_CONFIG/.baude-code/config.json and from
   the project directory's .baudecoderc
 - **Commands**. Select between available models by issuing the
   `/model claude-opus-4.6`. Quit with the `/quit` command.
 - **Plan mode**. The assistant may choose to enter "Plan Mode", which
   provides you with a list of multiple-choice questions before writing
   a plan into $MEMORY.
 - **Compaction**. When using models of a known context limit, baude-code
   automatically compacts the session as the history length approaches the
   limit.
 - **Subagents**. The agent may choose to fork a child agent with a parallel
   context to implement some scoped task.

## Non-goals
 
To limit the scope of the experiment while testing the ability of a no-harness
agent, certain features are **out of scope**:

 - **TUI**
 - **IDE Integration**
 - **REPL controls** (up arrow for history, etc)
 - **Login flows** (we'll assume an LLM's API key in the environment)

## BAML Implementation Status

The feature set above requires various BAML language features to be present.
If the feature is present and suitable for use in the implementation,
it's marked with an '*' in the todo list.
If it isn't present but can be stubbed out with an LLM call, it's marked with
'-'.

- Chat
  - [ ] stdin
  - [ ] stdout
- Coding Assistant Tools
  - [ ] LLM Function calls
  - [ ] Tool spec definition
  - [ ] Match on selected tool
  - [ ] Read files
  - [ ] Write files
  - [ ] PWD
  - [ ] Bash
- Context & Memory
  - [ ] Env vars
  - [ ] Read files
  - [ ] System time
  - [ ] PWD
  - [ ] Format strings (to construct paths)
- Permissions
  - [ ] Read files
  - [ ] Write files
  - [ ] Set/HashMap or custom record merging logic
- Commands
  - [ ] Parse strings
  - [ ] Client Registry (to list available models / choose one).
- Subagents
  - [ ] Process fork or start a new baude-code session
  - [ ] A `Return { result: string }` tool call.

## Implementation Notes

- First impl was written by claude opus 4.6, using basically this README.md as the initial prompt.
- It wrote ReadLine as a utility: ```
  // Read one line from stdin.
  function ReadLine() -> string {
      baml.sys.shell("read -r line && echo \"$line\"")
  }
  ```
- Do we have stdin/stdout builtins?
- It described tools manually with a template_string rather than using descriptions.
  Did use {{ ctx.output_format }} but this was sparse.
