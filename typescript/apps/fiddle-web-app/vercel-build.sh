#!/bin/bash
set -x
set -e

export PATH="/vercel/.cargo/bin:$PATH"

# Skip Rust installation in setup-dev.sh since we handled it above
bash ../../../scripts/setup-dev.sh --skip-pnpm --skip-cargo-watch

# Try to source cargo environment from multiple possible locations
if [ -f "$HOME/.cargo/env" ]; then
    source $HOME/.cargo/env
elif [ -f "/vercel/.cargo/env" ]; then
    source /vercel/.cargo/env
elif [ -f "$(eval echo ~$(whoami))/.cargo/env" ]; then
    source "$(eval echo ~$(whoami))/.cargo/env"
fi

# Ensure PATH includes cargo
export PATH="$HOME/.cargo/bin:/vercel/.cargo/bin:$(eval echo ~$(whoami))/.cargo/bin:$PATH"

# Ensure rustup has a default toolchain configured
if ! rustup show active-toolchain &> /dev/null; then
    echo "Setting up default Rust toolchain..."
    rustup default stable
fi
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