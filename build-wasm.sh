#!/bin/sh
# Builds the browser solver into web/solver.wasm and prints its SHA-256.
# Build paths are remapped so the file is the same on every machine.
set -eu
cd "$(dirname "$0")"
RUSTFLAGS="--remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=~/.cargo --remap-path-prefix=$(pwd)=/unaligned" \
	cargo build --locked --target wasm32-unknown-unknown --profile wasm
cp target/wasm32-unknown-unknown/wasm/unaligned_pow.wasm web/solver.wasm
if command -v sha256sum >/dev/null; then sha256sum web/solver.wasm; else shasum -a 256 web/solver.wasm; fi
