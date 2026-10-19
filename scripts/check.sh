#!/usr/bin/env bash
# Puerta de cada commit. Uso: scripts/check.sh
# El modo --e2e (pruebas de punta a punta del demonio) se agrega en F5b.
set -euo pipefail
cd "$(dirname "$0")/.."

(
  cd src-tauri
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo test
)
pnpm run lint
pnpm test
