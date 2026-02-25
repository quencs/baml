# AGENTS.md

## Cursor Cloud specific instructions

### Overview
BAML is a DSL for building AI workflows. The repo has two Rust workspaces (`engine/` and `baml_language/`) and a TypeScript/pnpm monorepo at the root.

### Prerequisites (installed by VM snapshot)
- **mise** manages tool versions (Rust, Go, Python, Ruby, Node, pnpm, uv, maturin, wasm-pack, etc.) — see `mise.toml`.
- `rust-toolchain.toml` pins Rust 1.93.0 (overrides the mise.toml Rust version).
- System packages needed: `libyaml-dev`, `libreadline-dev`, `libffi-dev`, `libssl-dev`, `pkg-config`, `protobuf-compiler`.

### Key commands

| Task | Command |
|---|---|
| Build engine | `cd engine && cargo build` |
| Build baml_language | `cd baml_language && cargo build` |
| Unit tests (engine) | `cd engine && cargo test --lib` |
| Unit tests (baml_language) | `cd baml_language && cargo test --lib` |
| Format check (engine) | `cd engine && cargo fmt -- --config imports_granularity="Crate" --config group_imports="StdExternalCrate" --check` |
| Format fix (engine) | `cd engine && cargo fmt -- --config imports_granularity="Crate" --config group_imports="StdExternalCrate"` |
| Format (baml_language) | `cd baml_language && cargo fmt --all -- --config imports_granularity=Crate --config group_imports=StdExternalCrate` |
| Lint (baml_language) | `cd baml_language && cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| BAML CLI generate | `cd integ-tests/python && cargo run --manifest-path ../../engine/Cargo.toml --bin baml-cli -- generate --from ../baml_src` |
| Install TS deps | `pnpm install` (from repo root) |
| TS typecheck | `pnpm typecheck` (from repo root) |

### Gotchas
- **Always run `cargo fmt`** (with the import config flags above) in `engine/` before committing Rust changes there. The workspace rule requires this.
- **Integration tests require LLM API keys** (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, etc.) in `integ-tests/.env`. Unit tests (`cargo test --lib`) do not need API keys.
- **Two separate Rust workspaces**: `engine/` and `baml_language/` are independent Cargo workspaces — build/test each separately.
- **pnpm version**: The repo uses pnpm 9.12.0 (see `package.json` `packageManager` field), even though mise.toml pins 10.14.0. The lockfile works with 9.x.
- **mise activation**: Use `eval "$(mise activate bash --shims)"` in your shell to get mise-managed tool versions on PATH.
- Refer to `CONTRIBUTING.md` and `README-DEV.md` for full development workflow documentation.
