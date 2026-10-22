#!/usr/bin/env bash
# Vercel install step: Rust with the wasm target, and Trunk.
# Vercel's image ships rustup (in its own CARGO_HOME); install it only if missing.
set -euo pipefail

RUST_TOOLCHAIN="1.92.0"
TRUNK_VERSION="0.21.14"
TOOLS="$PWD/.tools/bin"

if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs |
    sh -s -- -y --profile minimal --default-toolchain "$RUST_TOOLCHAIN"
fi
if [ -f "${CARGO_HOME:-$HOME/.cargo}/env" ]; then
  # shellcheck disable=SC1091
  source "${CARGO_HOME:-$HOME/.cargo}/env"
fi
rustup toolchain install "$RUST_TOOLCHAIN" --profile minimal --target wasm32-unknown-unknown
rustup default "$RUST_TOOLCHAIN"

mkdir -p "$TOOLS"
if [ ! -x "$TOOLS/trunk" ]; then
  curl -sSfL "https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-x86_64-unknown-linux-musl.tar.gz" |
    tar -xz -C "$TOOLS"
fi

rustc --version
"$TOOLS/trunk" --version
