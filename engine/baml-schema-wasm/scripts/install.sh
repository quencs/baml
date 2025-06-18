#!/usr/bin/env bash

set -euo pipefail

printf '%s\n' "entering install.sh"

printf '%s\n' " -> Creating out dir..."
# shellcheck disable=SC2154
mkdir -p "$out"/src

# Check if wasm-bindgen is installed, install if not
if ! command -v wasm-bindgen &> /dev/null; then
    printf '%s\n' " -> Installing wasm-bindgen-cli..."
    cargo install wasm-bindgen-cli
fi

printf '%s\n' " -> Generating $target package"
wasm-bindgen \
  --target "$target" \
  --out-dir "$out"/src \
  target/wasm32-unknown-unknown/release/baml_schema_build.wasm