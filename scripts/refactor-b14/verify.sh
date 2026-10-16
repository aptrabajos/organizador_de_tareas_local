#!/usr/bin/env bash
set -uo pipefail

# ─────────────────────────────────────────────────────────
# verify.sh - Chequeos de cada commit del refactor B14
# Uso:
#   ./scripts/refactor-b14/verify.sh baseline           # antes de mover: guarda la línea base
#   ./scripts/refactor-b14/verify.sh src/db/<dominio>.rs # después de mover: compara
#
# La línea base vive en src-tauri/target/b14/ (ignorado por git).
# Criterios: docs/PLAN-REFACTOR-B14.md, secciones 3.1 y 5.
# ─────────────────────────────────────────────────────────

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
STATE="$ROOT/src-tauri/target/b14"
EXPECTED_TESTS=168
cd "$ROOT/src-tauri"
mkdir -p "$STATE"

test_names() {
    cargo test --offline -- --list 2>/dev/null | rg ': test$' | sed -E 's/.*:://' | sort
}

if [[ "${1:-}" == "baseline" ]]; then
    test_names > "$STATE/tests-base.txt"
    echo "Línea base: $(wc -l < "$STATE/tests-base.txt") tests en $STATE/tests-base.txt"
    exit 0
fi

NEW="${1:?Uso: verify.sh baseline | verify.sh src/db/<dominio>.rs}"
[[ -f "$STATE/tests-base.txt" ]] || { echo "Falta la línea base: corré 'verify.sh baseline' antes de mover."; exit 1; }
[[ -f "$NEW" ]] || { echo "No existe $NEW (la ruta es relativa a src-tauri/)."; exit 1; }
fail=0

echo "== tests"
out=$(cargo test --offline 2>&1)
echo "$out" | rg '^test result'
total=$(echo "$out" | rg -o '(\d+) passed' -r '$1' | awk '{s+=$1} END {print s+0}')
[[ "$total" == "$EXPECTED_TESTS" ]] || { echo "ERROR: $total tests en vez de $EXPECTED_TESTS"; fail=1; }

echo "== nombres de test"
test_names > "$STATE/tests-now.txt"
diff "$STATE/tests-base.txt" "$STATE/tests-now.txt" && echo "mismos nombres que la línea base" || fail=1

echo "== clippy --all-targets -- -D warnings"
if cargo clippy --offline --all-targets -- -D warnings > "$STATE/clippy.txt" 2>&1; then
    echo "limpio"
else
    rg '^(error|warning)|-->' "$STATE/clippy.txt" | head -20
    fail=1
fi

echo "== alcance del diff (solo src-tauri/src/db/)"
if git diff --name-only HEAD -- "$ROOT" | rg -v '^src-tauri/src/db/'; then
    echo "ERROR: el diff toca archivos fuera de db/"
    fail=1
fi
git diff --stat HEAD -- "$ROOT"
echo "nuevo: $(wc -l < "$NEW") $NEW"

echo "== pureza del move"
git diff -U0 HEAD -- src/db/mod.rs | rg '^-[^-]' | sed 's/^-//' > "$STATE/removed.txt"
python3 -I - "$STATE/removed.txt" "$NEW" <<'EOF' || fail=1
import sys
from collections import Counter

removed = [l for l in open(sys.argv[1], encoding="utf-8").read().split("\n") if l.strip()]
available = Counter(open(sys.argv[2], encoding="utf-8").read().split("\n"))
missing = []
for line in removed:
    if available[line] > 0:
        available[line] -= 1
    else:
        missing.append(line)
# Regla 4 del plan: retirar `use` huérfanos está permitido; se listan para revisión.
uses = [l for l in missing if l.strip().startswith("use ")]
code = [l for l in missing if l not in uses]
print(f"{len(removed)} líneas no vacías quitadas de mod.rs")
print(f"  `use` retirados (confirmar que clippy los marcaba huérfanos): {uses}")
print(f"  código faltante en {sys.argv[2]}: {code}")
sys.exit(1 if code else 0)
EOF

if [[ $fail == 0 ]]; then echo "== TODO OK"; else echo "== HAY FALLAS"; exit 1; fi
