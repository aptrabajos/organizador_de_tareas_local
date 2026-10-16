"""Mueve un dominio de db/mod.rs a db/<name>.rs (refactor B14).

Uso (desde la raíz del repo):
  python3 -I scripts/refactor-b14/move_domain.py src-tauri/src/db scripts/refactor-b14/specs/NN-<dominio>.json

Después: agregar `mod <dominio>;` en db/mod.rs y correr verify.sh.

spec = {
  "name": "maintenance",
  "uses": ["use rusqlite::...;", ...],          # cabecera del archivo nuevo
  "test_uses": ["use super::super::test_support::*;", ...],
  "prod":  [[inicio, fin, "texto exacto línea inicio", "texto exacto línea fin"], ...],
  "tests": [[inicio, fin, "texto inicio", "texto fin"], ...]
}
Rangos 1-indexados e inclusivos. Cada rango se lleva además la línea en blanco
siguiente (debe estar vacía). Las guardas abortan sin escribir nada si un borde
no coincide.
"""
import json
import sys
from pathlib import Path

db_dir = Path(sys.argv[1])
spec = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
mod_path = db_dir / "mod.rs"
new_path = db_dir / f"{spec['name']}.rs"
if new_path.exists():
    sys.exit(f"{new_path} ya existe")
lines = mod_path.read_text(encoding="utf-8").split("\n")


def check(rng):
    start, end, first, last = rng
    if lines[start - 1] != first:
        sys.exit(f"guarda: línea {start} = {lines[start - 1]!r}, esperaba {first!r}")
    if lines[end - 1] != last:
        sys.exit(f"guarda: línea {end} = {lines[end - 1]!r}, esperaba {last!r}")
    if lines[end] != "":
        sys.exit(f"guarda: línea {end + 1} debería estar vacía: {lines[end]!r}")


def take(ranges):
    blocks, removed = [], set()
    for rng in ranges:
        check(rng)
        start, end = rng[0], rng[1]
        blocks.append(lines[start - 1:end])
        removed |= set(range(start, end + 2))  # + la línea en blanco siguiente
    return blocks, removed


prod_blocks, rm_prod = take(spec["prod"])
test_blocks, rm_tests = take(spec.get("tests", []))
if rm_prod & rm_tests:
    sys.exit("rangos de producción y tests se superponen")


def join(blocks):
    out = []
    for i, b in enumerate(blocks):
        if i:
            out.append("")
        out.extend(b)
    return out


body = list(spec["uses"]) + ["", "use super::Database;", "", "impl Database {"]
body += join(prod_blocks) + ["}"]
if test_blocks:
    body += ["", "#[cfg(test)]", "mod tests {"] + list(spec["test_uses"]) + [""]
    body += join(test_blocks) + ["}"]
body.append("")

kept = [l for i, l in enumerate(lines, 1) if i not in rm_prod | rm_tests]
new_path.write_text("\n".join(body), encoding="utf-8")
mod_path.write_text("\n".join(kept), encoding="utf-8")
n_prod = sum(map(len, prod_blocks))
n_test = sum(map(len, test_blocks))
print(f"{spec['name']}.rs: {n_prod} líneas de producción + {n_test} de tests -> {len(body)} líneas")
print(f"mod.rs: {len(lines)} -> {len(kept)} líneas")
