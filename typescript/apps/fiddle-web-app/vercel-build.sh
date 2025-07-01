#!/bin/bash
set -x
set -e

# Install Rust in a Vercel-friendly way
if ! command -v cargo &> /dev/null; then
    echo "Installing Rust for Vercel environment..."
    # Fix HOME directory for Rust installer in Vercel
    export HOME=/root
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y --default-toolchain 1.85.0
    # Restore HOME for Vercel
    export HOME=/vercel
fi

export PATH="/vercel/.cargo/bin:$PATH"
source $HOME/.cargo/env

# Skip Rust installation in setup-dev.sh since we handled it above
bash ../../../scripts/setup-dev.sh --skip-pnpm --skip-cargo-watch --skip-rust
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