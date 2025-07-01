#!/bin/bash
set -x
set -e

export PATH="/vercel/.cargo/bin:$PATH"

# Skip Rust installation in setup-dev.sh since we handled it above
bash ../../../scripts/setup-dev.sh --skip-pnpm --skip-cargo-watch

source $HOME/.cargo/env
# clang --version
#llvm-config --version
# g++ --version

dnf install -y llvm
dnf install -y clang

cd ../../../engine/baml-schema-wasm
export OPENSSL_NO_VENDOR=1
# cargo install
rustup target add wasm32-unknown-unknown

cd ../../

pnpm build:fiddle-web-app --force

ls -l
ls -l /vercel/output