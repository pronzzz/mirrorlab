#!/usr/bin/env bash
# Vercel build step: compile the Rust/WebAssembly editor into app/dist.
set -euo pipefail
if [ -f "${CARGO_HOME:-$HOME/.cargo}/env" ]; then
  # shellcheck disable=SC1091
  source "${CARGO_HOME:-$HOME/.cargo}/env"
fi
export PATH="$PWD/.tools/bin:$PATH"
cd app
trunk build --release
ls -la dist
