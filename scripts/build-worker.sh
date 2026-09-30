#!/usr/bin/env bash
set -euo pipefail

if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi
export PATH="${HOME}/.cargo/bin:${PATH}"

if [[ "${WORKER_BUILD_SKIP:-0}" == "1" ]]; then
  echo "[build-worker] Skipping (WORKER_BUILD_SKIP=1)"
  exit 0
fi

log() {
  echo "[build-worker] $(date -u +"%Y-%m-%dT%H:%M:%SZ") $*"
}

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found. Install Rust (https://rustup.rs) or run scripts/cloudflare-deploy.sh on CI." >&2
  exit 127
fi

log "rustc: $(rustc --version)"
log "cargo: $(cargo --version)"

if ! command -v worker-build >/dev/null 2>&1; then
  log "Installing worker-build@=0.8.6 (first CI run can take several minutes)..."
  cargo install "worker-build@=0.8.6"
fi

log "Running worker-build --release..."
exec worker-build --release -- -v
