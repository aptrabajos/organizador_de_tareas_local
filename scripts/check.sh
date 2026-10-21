#!/usr/bin/env bash
# Puerta de cada commit. Uso: scripts/check.sh [--windows]
# El modo --e2e (pruebas de punta a punta del demonio) se agrega en F5b.
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ "${1:-}" == "--windows" ]]; then
  # Chequea que compile para Windows (platform/windows.rs no se compila en Linux).
  # No entra en la corrida por defecto, que tiene que andar sin red: necesita la
  # caché de cargo con las dependencias de Windows, que se llena una vez, con red,
  # con `cargo fetch --locked --target x86_64-pc-windows-gnu` (paso manual del
  # usuario o de la CI). Ver 3.10.3 del diseño del tracker.
  (cd src-tauri && cargo check --locked --target x86_64-pc-windows-gnu) || {
    echo "cargo check de Windows falló; si falta la caché, ver 3.10.3" >&2
    exit 1
  }
  exit 0
fi

(
  cd src-tauri
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo test
)
pnpm run lint
pnpm test
