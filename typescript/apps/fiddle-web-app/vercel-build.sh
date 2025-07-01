#!/bin/bash
set -x
set -e

bash ../../../scripts/setup-dev.sh

export PATH="/vercel/.cargo/bin:$PATH"

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

pnpm build:fiddle-web-app

ls -l
ls -l /vercel/output