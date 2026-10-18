# Herramientas del refactor B14

Scripts para ejecutar `docs/PLAN-REFACTOR-B14.md` commit por commit sin errores de
dedo. Se borran junto con la rama cuando el B14 se mergea a `main`.

| Archivo | Qué hace |
| --- | --- |
| `move_domain.py` | Corta un dominio de `db/mod.rs` a `db/<dominio>.rs` según un spec. Verifica el texto exacto de cada borde y aborta sin escribir si algo no coincide. |
| `verify.sh` | Corre los criterios de salida del plan: 168 tests, mismos nombres, clippy `-D warnings`, diff solo en `db/` y pureza del move. |
| `specs/NN-<dominio>.json` | El corte de cada commit, versionado como registro de qué se movió. |

## Procedimiento por commit

```bash
# 1. Línea base (con el árbol limpio)
./scripts/refactor-b14/verify.sh baseline

# 2. Ubicar los rangos actuales por NOMBRE de función: los números de línea
#    del plan son de main@839a8e0 y se corren con cada commit
rg -n 'fn <nombre>' src-tauri/src/db/mod.rs

# 3. Escribir specs/NN-<dominio>.json y mover
python3 -I scripts/refactor-b14/move_domain.py src-tauri/src/db scripts/refactor-b14/specs/NN-<dominio>.json

# 4. Agregar `mod <dominio>;` en db/mod.rs, resolver solo imports faltantes y
#    retirar los `use` que el corte dejó huérfanos (regla 4 del plan)

# 5. Verificar
./scripts/refactor-b14/verify.sh src/db/<dominio>.rs

# 6. Commit con archivos concretos, nunca `git add -A`
git add src-tauri/src/db/mod.rs src-tauri/src/db/<dominio>.rs scripts/refactor-b14/specs/NN-<dominio>.json
git commit -m "checkpoint: extrae db/<dominio>.rs ..."
```

## Formato del spec

```json
{
  "name": "maintenance",
  "uses": ["use rusqlite::{params, Connection, Result};"],
  "test_uses": ["    use super::super::test_support::*;", "    use crate::db::Database;"],
  "items": [],
  "prod":  [[58, 80, "texto exacto de la línea 58", "texto exacto de la línea 80"]],
  "tests": [[2182, 2210, "    #[test]", "    }"]]
}
```

`items` (opcional) son ítems de nivel módulo, como un `struct`, que van fuera del
`impl Database`. Rangos 1-indexados e inclusivos. Cada rango se lleva también la línea en blanco
que lo sigue. Si el rango es el último del bloque (lo sigue la llave `}` de cierre
del `impl` o del `mod tests`), la llave se queda y se lleva la línea en blanco
anterior.
