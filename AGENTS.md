# AGENTS.md

## Cursor Cloud specific instructions

### Overview

BAML is a compiler + runtime + IDE toolchain for building AI applications. It is a polyglot monorepo with two primary Rust workspaces (`engine/` and `baml_language/`) plus TypeScript packages, Python/Go/Ruby language clients, and integration tests.

### Tool Management

All dev tools are managed by [mise](https://mise.jdx.dev/) and configured in `/workspace/mise.toml` (and `/workspace/baml_language/mise.toml`). Mise activation is in `~/.bashrc`. When entering shell sessions, tools like `rustc`, `node`, `pnpm`, `python`, `uv`, `maturin`, `wasm-pack` etc. are available via mise shims.

**Gotcha:** The `pipx` backend tools (`uv`, `ruff`, `maturin`) require `pipx` to be installed first. If `mise install` fails on these, run `pip install --user pipx` then retry `mise install`.

**Gotcha:** `protobuf-compiler` (`protoc`) must be installed system-wide (`apt-get install protobuf-compiler`) — it is needed by the Rust language client for Go/Rust code generation tests in `engine/`.

### Building and Testing

Standard commands (see `CONTRIBUTING.md` and `README-DEV.md` for full details):

| Task | Command | Directory |
|------|---------|-----------|
| Build engine | `cargo build` | `engine/` |
| Build baml_language | `cargo build` | `baml_language/` |
| Unit tests (engine) | `cargo test --lib` | `engine/` |
| Unit tests (baml_language) | `cargo test --lib` | `baml_language/` |
| Rust fmt (engine) | `cargo fmt -- --config imports_granularity="Crate" --config group_imports="StdExternalCrate"` | `engine/` |
| Biome lint/format | `pnpm format:ci` | root |
| Install JS deps | `pnpm install` | root |
| BAML CLI | `cargo run -p baml-cli -- <subcommand>` | `engine/` |

### Integration Tests

Integration tests in `integ-tests/` call real LLM APIs and require API keys (at minimum `OPENAI_API_KEY`). See `CONTRIBUTING.md` for setup. For Python integ tests:

1. `cd integ-tests/python && uv sync`
2. `uv run maturin develop --uv --manifest-path ../../engine/language_client_python/Cargo.toml`
3. `uv run baml-cli generate --from ../baml_src`
4. `dotenv -e ../.env -- uv run pytest`

### Non-obvious Notes

- The root `pnpm-workspace.yaml` includes packages across `typescript/`, `engine/`, `integ-tests/`, `fern/`, `baml_language/`, and `examples/`. Running `pnpm install` at root installs all JS dependencies.
- The `packageManager` field in root `package.json` is `pnpm@9.12.0`, but mise installs `pnpm@10.14.0`. The lockfile uses pnpm v9 format; `pnpm install` works fine.
- Two Rust workspaces co-exist: `engine/` (v1/legacy, edition 2021) and `baml_language/` (v2/next-gen, edition 2024). Both use Rust 1.93.0 (from `rust-toolchain.toml`).
- `baml_language/` has its own `mise.toml` that needs `mise trust` on first use.
- The `engine/` workspace's `baml-schema-wasm` is excluded from default members to avoid feature conflicts; it is built separately for WASM targets.
