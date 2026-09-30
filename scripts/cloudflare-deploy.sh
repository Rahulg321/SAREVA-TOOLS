#!/usr/bin/env bash
# Workers Builds images include Node but not Rust. Install rustup before wrangler deploy.
set -euo pipefail

log() {
  echo "[cloudflare-deploy] $(date -u +"%Y-%m-%dT%H:%M:%SZ") $*"
}

ensure_cargo_env() {
  if [[ -f "${HOME}/.cargo/env" ]]; then
    # shellcheck disable=SC1091
    source "${HOME}/.cargo/env"
  fi
  export PATH="${HOME}/.cargo/bin:${PATH}"
}

ensure_cargo_env

if ! command -v cargo >/dev/null 2>&1; then
  log "Installing Rust via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  ensure_cargo_env
fi

log "Adding wasm32-unknown-unknown target..."
rustup target add wasm32-unknown-unknown

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Workers Builds runs `npm clean-install` before the deploy command.
if [[ ! -d node_modules/wrangler ]] && [[ -f package-lock.json ]]; then
  log "Installing npm dependencies..."
  npm ci
fi

log "Building Rust worker (expect 5–15 min on first deploy)..."
bash scripts/build-worker.sh

log "Uploading worker..."
export WORKER_BUILD_SKIP=1
exec npx wrangler deploy --no-bundle "$@"
