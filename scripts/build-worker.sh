#!/usr/bin/env bash
set -euo pipefail

export PATH="${HOME}/.cargo/bin:${PATH}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found. Install Rust (https://rustup.rs) or run scripts/cloudflare-deploy.sh on CI." >&2
  exit 127
fi

if ! command -v worker-build >/dev/null 2>&1; then
  cargo install -q "worker-build@=0.8.6"
fi

exec worker-build --release
