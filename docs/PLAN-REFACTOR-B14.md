# Plan de refactor B14 — `db/mod.rs` monolito → módulos por dominio

**Estado:** propuesta. Nada de esto está implementado todavía.
**Alcance:** exclusivamente `src-tauri/src/db/`. Refactor puro, cero cambio de comportamiento.

---

## 0. De dónde viene esto

El B14 del plan de arreglo de bugs no se "dejó sin arreglar": se decidió **no
fragmentar los monolitos durante el ciclo de bugs**, porque mezclar un
movimiento masivo de código con correcciones de lógica hace imposible saber cuál
de las dos cosas rompió algo. Está registrado en
[`reporte-fixes-v0.6.1.md`](./reporte-fixes-v0.6.1.md), secciones *"B14 — No se
fragmentan los monolitos por ahora"* y *"Deuda conocida que queda abierta"*.

Aquel ciclo ya cerró. Este documento es el plan para saldar esa deuda, ahora sí,
en una sesión propia y sin ninguna corrección de lógica encima.

### El mapa de tamaños hoy

| # | Archivo | Líneas |
| - | ------- | -----: |
| 1 | `src-tauri/src/db/mod.rs` | **3.844** |
| 2 | `src-tauri/src/commands/mod.rs` | 2.043 |
| 3 | `src-tauri/src/backup/mod.rs` | 803 |
| 4 | `src-tauri/src/pdf_export/mod.rs` | 603 |
| 5 | `src-tauri/src/platform/detection.rs` | 376 |
| 6 | `src-tauri/src/platform/linux.rs` | 356 |

**Este plan ataca solo el #1.** `commands/mod.rs` es el segundo monolito y merece
su propio plan; no se toca acá. Todo el resto del backend ya está organizado por
dominio (`models/`, `config/`, `platform/`, `tracking/`, `pdf_export/`,
`backup/`). `db/` es el único que quedó como un solo `mod.rs` gigante.

### El dato que cambia todo

`src-tauri/src/db/mod.rs` son 3.844 líneas, pero:

- **Líneas 1–2027:** código de producción (~2.027 líneas).
- **Líneas 2029–3844:** `#[cfg(test)] mod tests` (~1.816 líneas, 86 tests).

Casi la mitad del archivo son tests. Y el segundo dato, todavía más importante:

> **La superficie pública del módulo `db` son DOS símbolos.**
> `Database` y `TrashItem`. Nada más.

Verificado sobre todo `src-tauri/src` fuera de `db/`:

```
db::Database   → backup/mod.rs:21, commands/guards.rs:10,78,
                 commands/mod.rs:12,1802, main.rs:14, pdf_export/mod.rs:1
db::TrashItem  → commands/mod.rs:185
```

Eso significa que el contrato que hay que preservar es chiquito y explícito. Los
79 comandos Tauri no consumen `db::algo`: consumen **métodos de `Database`**.

---

## 1. Inventario real del monolito

### 1.1 Piezas transversales (las que todo módulo hijo va a necesitar)

| Pieza | Líneas | Qué es | Quién la usa |
| ----- | -----: | ------ | ------------ |
| `use` de cabecera | 1–7 | `rusqlite`, `log`, `HashMap`, `PathBuf`, `Mutex`, 15 tipos de `models::project` | todos |
| `PROJECT_COLUMNS` (const privada) | 19–32 | Las 23 columnas de `projects` EN ORDEN. El orden **es** el contrato de `row_to_project`. | projects, dashboard, groups |
| `row_to_project` (fn libre privada) | 34–66 | Arma un `Project` leyendo por posición. Deja `links: None` a propósito (evita el N+1). | projects (420, 437, 493, 875), dashboard (1641), groups (1733, 1767) |
| `pub struct Database` | 68–70 | `{ conn: Mutex<Connection> }` | todo el crate |
| `Database::conn()` (privada) | 91–98 | Toma el lock recuperándose del envenenamiento (B15). Documentada largo y tendido. | los 50 métodos públicos |
| `Database::empty_to_null()` (privada) | 367–374 | Normaliza `String` vacío → `None`. | projects (403–410, 541–568), journal (1296, 1359) |

### 1.2 Dominios funcionales

Rangos de línea sobre el código de producción. Los cortes **no son inventados**,
pero tampoco salen de leer los separadores del archivo: cada rango se validó por
inspección directa, contrastándolo contra las firmas de funciones y la estructura
de bloques del `impl Database`, uno por uno. **La tabla de abajo es la fuente de
verdad del corte.**

Conviene ser exacto con lo que el archivo trae y lo que no, porque el ejecutor va
a buscar esos separadores. Existen, pero con el formato largo —
`// ==================== MÉTODOS PARA ARCHIVOS ADJUNTOS ====================` —
y **solo cubren 7 de los 13 dominios de producción**:

| Dominio con separador | Línea |
| --------------------- | ----: |
| Adjuntos | 1207 |
| Journal | 1288 |
| Todos | 1405 |
| Estado / favoritos | 1528 |
| Dashboard | 1628 |
| Grupos de proyectos | 1719 |
| Time tracking | 1896 |

**Los otros seis dominios no tienen separador propio**: schema/migraciones,
mantenimiento, proyectos, papelera, enlaces y actividad. Para esos, el corte se
apoya únicamente en las firmas y en el inventario de esta sección.

Dentro de `mod tests` hay otros cuatro separadores, con el mismo formato largo:
*"Paso 1: Inicialización y CRUD de Proyectos"* (2412), *"Paso 2: CRUD de Links"*
(2882), *"Paso 3: Journal, Todos, Attachments"* (3016) y *"Paso 4: Analytics,
Status, Groups, Dashboard, Time Tracking"* (3306).

| # | Dominio | Rango(s) de línea | Líneas aprox. | Funciones públicas | Privadas |
| - | ------- | ----------------- | ------------: | ------------------ | -------- |
| 1 | **Schema / migraciones** | 100–346 | 247 | `new` | — |
| 2 | **Mantenimiento** | 347–366 | 20 | `backup_to`, `verify_db_file` | — |
| 3 | **Proyectos** | 375–616, 841–897 | 299 | `create_project`, `get_all_projects`, `get_project`, `update_project`, `is_registered_project_path`, `active_project_paths`, `search_projects` | — |
| 4 | **Papelera** | 617–840 (+ `TrashItem` 9–17) | 232 | `delete_project`, `restore_project`, `purge_project`, `empty_trash`, `list_trash` | `purge_project_internal` |
| 5 | **Enlaces** | 898–1078 | 181 | `create_link`, `get_project_links`, `update_link`, `delete_link` | `get_links_for_projects`, `get_project_links_internal` |
| 6 | **Actividad / estadísticas** | 1079–1208 | 130 | `track_project_open`, `add_project_time`, `get_project_stats`, `get_project_activities` | — |
| 7 | **Adjuntos** | 1209–1289 | 81 | `add_attachment`, `get_attachments`, `delete_attachment` | — |
| 8 | **Journal** | 1290–1406 | 117 | `create_journal_entry`, `get_journal_entries`, `update_journal_entry`, `delete_journal_entry` | — |
| 9 | **Todos** | 1407–1527 | 121 | `create_todo`, `get_project_todos`, `update_todo`, `delete_todo` | — |
| 10 | **Estado / pin / orden** | 1528–1627 | 100 | `update_project_status`, `toggle_pin_project`, `reorder_pinned_projects`, `update_project_order` | — |
| 11 | **Dashboard** | 1628–1718 | 91 | `get_recent_projects`, `get_all_pending_todos`, `get_recent_journal_entries` | — |
| 12 | **Grupos jerárquicos** | 1719–1895 | 177 | `get_root_projects`, `get_subprojects`, `get_project_with_children`, `count_subprojects`, `assign_project_to_group` | `ancestor_chain_contains` |
| 13 | **Time tracking** | 1896–2027 | 132 | `create_tracking_session`, `end_tracking_session`, `get_tracking_sessions`, `get_time_stats` | — |
| — | *(tests)* | 2029–3844 | 1.816 | 86 `#[test]` + 7 helpers | — |

**Totales de producción:** 50 métodos `pub fn`, 6 `fn` privadas, 1 fn libre
privada, 1 const privada, 2 tipos públicos (`Database`, `TrashItem`).

#### Aclaración: acá NO hay dominio de "config"

La configuración de la app **no vive en `db/`**. Vive en `src-tauri/src/config/`
(`schema.rs`, `defaults.rs`, `manager.rs`) y se persiste en archivo, no en
SQLite. Los comandos `get_config` / `update_config` / `reset_config` /
`get_shortcuts_config` / `update_shortcuts_config` no tocan `Database`. Si venías
esperando un `db/config.rs`: no hay nada que mover.

### 1.3 Qué comando consume qué dominio

Los 79 handlers registrados en `main.rs` vía `tauri::generate_handler![]`. Solo
se listan los que efectivamente tocan `Database`.

| Dominio | Comandos Tauri que lo consumen | Otros consumidores |
| ------- | ------------------------------ | ------------------ |
| Schema | *(ninguno directo — `Database::new` se llama en `main.rs:31`)* | `backup/mod.rs`, `commands/guards.rs:84`, `pdf_export/mod.rs:455` (tests) |
| Mantenimiento | `backup_database`, `restore_backup` (vía `backup/`) | `backup/mod.rs:115,625` |
| Proyectos | `create_project`, `get_all_projects`, `get_project`, `update_project`, `search_projects`, + `get_project` como lectura previa en `export_project_to_pdf`, `init_tracking`, `get_time_stats`, `start_work_session` | `commands/guards.rs:88`, `pdf_export/mod.rs:459`, `backup/mod.rs:622,639,740,773,793` |
| Papelera | `delete_project`, `restore_project`, `list_trash`, `purge_project`, `empty_trash` | `commands/guards.rs:140` |
| Enlaces | `create_project_link`, `get_project_links`, `update_project_link`, `delete_project_link` | `pdf_export/mod.rs:261,477` |
| Actividad | `track_project_open`, `add_project_time`, `get_project_stats`, `get_project_activities` | — |
| Adjuntos | `add_attachment`, `get_attachments`, `delete_attachment` | — |
| Journal | `create_journal_entry`, `get_journal_entries`, `update_journal_entry`, `delete_journal_entry` | — |
| Todos | `create_todo`, `get_project_todos`, `update_todo`, `delete_todo` | — |
| Estado/pin | `update_project_status`, `toggle_pin_project`, `reorder_pinned_projects`, `update_project_order` | — |
| Dashboard | `get_dashboard_data` (usa los tres de una) | — |
| Grupos | `get_root_projects`, `get_subprojects`, `get_project_with_children`, `count_subprojects`, `assign_project_to_group` | — |
| Time tracking | `init_tracking`, `get_tracking_sessions`, `get_time_stats`, `start_work_session`, `stop_work_session`, `stop_active_session_sync` | — |

### 1.4 El grafo de acoplamiento interno

Esto es lo que decide el orden de extracción. Solo hay **seis** aristas
cruzadas entre dominios:

```
common ──> projects, dashboard, groups     (PROJECT_COLUMNS, row_to_project)
common ──> projects, journal               (empty_to_null)
links  ──> projects, groups                (get_links_for_projects)
links  ──> projects                        (get_project_links_internal)
projects ──> groups                        (get_project, ya es pub)
```

Todos los demás dominios (schema, mantenimiento, papelera, actividad, adjuntos,
todos, estado/pin, tracking) **no llaman a nada de otro dominio**. Son hojas.
Se pueden mover en cualquier orden y en cualquier momento.

---

## 2. Diseño del target

### 2.1 El mecanismo de Rust que hace esto casi gratis

Acá está el concepto, y si no lo entendés el resto del plan parece riesgoso
cuando no lo es:

> **En Rust, los bloques `impl` inherentes de un tipo pueden vivir repartidos en
> varios módulos del mismo crate.** Un `impl Database { pub fn create_project(…) }`
> escrito en `db::projects` agrega ese método al tipo `crate::db::Database`. El
> que llama nunca ve el módulo.

Consecuencia directa: **los 50 métodos públicos no necesitan ni un solo
`pub use`.** Están sobre el tipo, no sobre el módulo. `db.create_project(...)` en
`commands/mod.rs:129` sigue compilando sin tocar una coma, viva donde viva el
`impl`.

Lo único que sí necesita re-exportarse es el tipo libre `TrashItem`, porque
`commands/mod.rs:185` lo nombra por ruta: `crate::db::TrashItem`.

Segunda consecuencia, igual de importante:

> **Un item privado es visible en el módulo donde se define Y en todos sus
> descendientes.**

Por eso `Database::conn()` se queda en `mod.rs` como `fn` privada **sin cambiar
la visibilidad**: `db::projects` es descendiente de `db`, así que la ve. Ni un
`pub(crate)` de más.

Y lo mismo aplica en la otra dirección: para los helpers que se mueven a un
**hermano**, `pub(super)` alcanza y sobra. `pub(super)` desde `db::common`
significa "visible en `db`", y eso incluye a todos los descendientes de `db`.
Nunca hace falta `pub(crate)`, y `pub` a secas sería una fuga de API.

Tercera consecuencia, la que cierra el círculo: si los helpers privados se
mueven **como funciones asociadas de `Database`** (no como funciones libres),
los call sites tampoco cambian. `Self::empty_to_null(...)` y
`self.get_links_for_projects(...)` resuelven a través del tipo, no del módulo.

**Resultado: el diff de cada commit es un movimiento de bloques + una cabecera
de `use` + cuatro `fn` que pasan a `pub(super) fn`. Los cuerpos y las llamadas
no se tocan.**

### 2.2 Árbol de archivos objetivo

```
src-tauri/src/db/
├── mod.rs           (~90)   fachada: struct Database, conn(), mod …, pub use TrashItem
├── common.rs        (~70)   PROJECT_COLUMNS, row_to_project, empty_to_null
├── schema.rs        (~250)  Database::new() — PRAGMAs, CREATE TABLE, 17 ALTER TABLE, índices
├── maintenance.rs   (~30)   backup_to, verify_db_file
├── projects.rs      (~310)  CRUD de proyectos + búsqueda + consultas de ruta
├── trash.rs         (~245)  TrashItem + borrado lógico, restore, purge, empty_trash, list_trash
├── links.rs         (~190)  CRUD de enlaces + los dos helpers de carga en lote (B16)
├── activity.rs      (~140)  apertura, tiempo acumulado, stats, timeline de actividad
├── attachments.rs   (~90)   adjuntos
├── journal.rs       (~125)  bitácora
├── todos.rs         (~130)  tareas
├── status.rs        (~110)  estado, pin, orden de visualización
├── dashboard.rs     (~100)  las tres consultas agregadas del dashboard
├── groups.rs        (~190)  jerarquía padre/hijo, ciclos, asignación a grupo
├── tracking.rs      (~140)  sesiones de time tracking y sus estadísticas
└── test_support.rs  (~120)  #[cfg(test)] helpers compartidos entre los tests de dominio
```

**16 archivos.** `mod.rs` pasa de 3.844 a ~90 líneas. Ningún archivo de dominio
supera las ~310 líneas contando sus tests movidos.

### 2.3 Qué expone cada módulo

La regla es una sola: **el módulo hijo se declara privado (`mod x;`, nunca
`pub mod x;`)**. Así no aparece ninguna ruta pública nueva y la superficie del
crate queda byte a byte idéntica.

| Módulo | Lo que expone hacia afuera de sí mismo | Visibilidad |
| ------ | -------------------------------------- | ----------- |
| `common` | `PROJECT_COLUMNS`, `row_to_project` | `pub(super)` |
| `common` | `Database::empty_to_null` | `pub(super) fn` asociada |
| `links` | `Database::get_links_for_projects`, `Database::get_project_links_internal` | `pub(super) fn` asociadas |
| `trash` | `struct TrashItem` | `pub` + `pub use trash::TrashItem;` en `mod.rs` |
| `groups` | `ancestor_chain_contains` | **privada** — no sale del módulo |
| `trash` | `purge_project_internal` | **privada** — no sale del módulo |
| todos | los métodos `pub fn` de `impl Database` | `pub` (llegan por el tipo, sin re-export) |

### 2.4 Cómo queda `db/mod.rs`

Esqueleto objetivo (ilustrativo, no es código a copiar tal cual):

```rust
use log::error;
use rusqlite::Connection;
use std::sync::Mutex;

mod common;
mod schema;
mod maintenance;
mod projects;
mod trash;
mod links;
mod activity;
mod attachments;
mod journal;
mod todos;
mod status;
mod dashboard;
mod groups;
mod tracking;

#[cfg(test)]
mod test_support;

/// Único re-export necesario: `commands/mod.rs:185` nombra este tipo por ruta.
pub use trash::TrashItem;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// (se conserva el doc comment completo de B15, sin tocar una palabra)
    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> { /* … idéntico … */ }
}
```

Eso es toda la fachada. No hay `pub use` de métodos, porque no hacen falta.

### 2.5 Los tests

Los 86 tests **viajan con su dominio**, aprovechando que ya están agrupados por
los cuatro separadores `// ==================== Paso N: … ====================`
del `mod tests` actual (2412, 2882, 3016, 3306). Cada módulo termina
con su propio `#[cfg(test)] mod tests`.

Los 7 helpers compartidos (`test_db`, `seed_project_at`, `seed_link`,
`link_titles`, `test_project_dto`, `seed_all_child_tables`, `child_row_counts`)
se van a `db/test_support.rs`, declarado `#[cfg(test)] mod test_support;`, con
cada helper en `pub(super)`. Cada `mod tests` de dominio abre con:

```rust
use super::super::test_support::*;
use crate::db::Database;
use crate::models::project::*;
```

Los nombres de los tests **no cambian**, solo su ruta completa
(`db::tests::test_create_project` → `db::projects::tests::test_create_project`).
Eso importa para la verificación del punto 5.

---

## 3. Estrategia: refactor puro, cero cambio de comportamiento

### 3.1 Las cinco reglas innegociables

1. **Un dominio por commit.** Nada de "aprovecho y muevo dos".
2. **Cada commit compila y pasa `cargo test` completo.** No se commitea rojo
   "porque lo arreglo en el siguiente". Si un commit no cierra solo, el corte
   estaba mal elegido: volvé atrás y cortá distinto.
3. **El diff es solo *move*.** Los cuerpos de las funciones se copian textualmente,
   con sus comentarios, sus `debug!`/`error!`, su formato y hasta sus espacios.
   Prohibido "ya que estoy" arreglar un nombre, un `clippy`, un typo o un SQL.
4. **El único cambio tolerado dentro de un commit de movimiento** es el mínimo
   inevitable por visibilidad y por imports: `fn` → `pub(super) fn` en los cuatro
   helpers cruzados, la cabecera de `use` del archivo nuevo, y **retirar de
   `mod.rs` los `use` que ese mismo corte dejó huérfanos**. Nada más.

   > **Por qué el `use` huérfano se limpia acá y no recién en el commit 17.** La
   > regla 2 y el criterio de salida de cada fase exigen
   > `cargo clippy --all-targets -- -D warnings` limpio en **cada** commit. Al
   > sacar un dominio de `mod.rs`, los `use` que solo ese dominio consumía quedan
   > sin usuario: clippy los marca `unused_imports` y, con `-D warnings`, el
   > commit falla **a mitad del refactor**. Peor todavía, el ejecutor no puede
   > distinguir "corté mal el dominio" de "me sobró un `use`", que son problemas
   > de gravedad opuesta.
   >
   > Sacar esos `use` es limpieza de imports, **no** un cambio de comportamiento:
   > no toca una sola línea ejecutable y no viola el espíritu del *move* puro. El
   > alcance es estricto — **solo** los `use` que quedaron huérfanos por ese corte,
   > verificables porque clippy los nombra uno por uno. Cualquier otro `use` no se
   > toca.
   >
   > *Opción B, evaluada y descartada:* correr clippy **sin** `-D warnings` durante
   > las fases 1–3 y exigirlo estricto recién en la Fase 4. Funciona, pero apaga la
   > red de seguridad justo en los 15 commits donde más código se mueve, que es
   > exactamente cuando más la necesitás. **Se elige la opción A: `-D warnings`
   > estricto de punta a punta, y el `use` huérfano se retira en el mismo commit
   > que lo dejó huérfano.**
5. **Cero cambios de esquema.** Ni una `ALTER TABLE`, ni un índice, ni un
   `PRAGMA`. El bloque de migraciones se mueve entero y tal cual.

### 3.2 El procedimiento, paso a paso, para cada dominio

Para cada dominio D, en el commit de D:

1. `git switch -c refactor/db-modulos` (una sola vez, al principio).
2. Crear `src-tauri/src/db/D.rs` con la cabecera de `use` mínima + `use super::Database;`.
3. **Cortar** (no copiar-y-dejar) el rango de líneas de producción de D desde
   `mod.rs` y pegarlo dentro de un `impl Database { … }` en `D.rs`.
4. **Cortar** los `#[test]` de D desde `mod tests` de `mod.rs` y pegarlos en un
   `#[cfg(test)] mod tests` al final de `D.rs`.
5. Agregar `mod D;` en la lista de `mod.rs` (orden alfabético o el del árbol de 2.2;
   elegí uno y respetalo).
6. `cargo build` → resolver **solo** errores de visibilidad y de `use` faltante.
   Si aparece un error que no es de esos dos tipos, **parate**: algo se movió mal.
7. `cargo test` → 168 verdes, ni uno menos.
8. Retirar de `mod.rs` los `use` que el corte dejó huérfanos (regla 4 de 3.1:
   solo esos, nada más) y correr
   `cargo clippy --all-targets -- -D warnings` → limpio.
9. `git add` de los archivos concretos (`db/mod.rs`, `db/D.rs`, su spec) y
   `git commit -m "checkpoint: extrae db/D.rs del monolito"`. Nunca `git add -A`
   (regla de `CLAUDE.md`, "Nada de commits automáticos").

> **Notas de ejecución (2026-10-16, desde el commit 4).** Los pasos 2–8 están
> automatizados en `scripts/refactor-b14/` (ver su `README.md`; se borraron al
> mergear el B14 y siguen disponibles en `git show 91a61b4:scripts/refactor-b14/README.md`). Los números de
> línea de este plan son de `main@839a8e0`: cada commit los corre, así que los
> rangos se ubican por nombre de función. El criterio de clippy `-D warnings` se
> pudo exigir completo recién después de `05fa427`, que limpió 8 hallazgos previos
> ubicados fuera de `db/`.

### 3.3 Orden de extracción (de lo más independiente a lo más acoplado)

El orden sale directo del grafo de 1.4.

**Primero `common` y `test_support`**, porque son los habilitadores: sin ellos
ningún dominio hijo puede compilar solo.

**Después las ocho hojas** (`schema`, `maintenance`, `attachments`, `journal`,
`todos`, `activity`, `status`, `tracking`). No dependen de nadie salvo de
`common`. `journal` es la única de las ocho que necesita `empty_to_null`, o sea
que igual ya está cubierta.

**Después `links`**, que es la que *provee* helpers a otros dos dominios. Se
mueve antes que sus consumidores para que cuando les toque a ellos, el
`pub(super)` ya esté puesto.

**Después los tres consumidores**: `projects` (usa `common` + `links`), `groups`
(usa `common` + `links` + `Database::get_project`), `dashboard` (usa `common`).

**Último `trash`**, que además arrastra el tipo público `TrashItem` y por lo
tanto es el único commit que toca la superficie externa del módulo.

### 3.4 `pub(crate)` vs `pub(super)` vs privado — el criterio

| Situación | Visibilidad | Por qué |
| --------- | ----------- | ------- |
| Helper usado solo dentro de su propio módulo (`ancestor_chain_contains`, `purge_project_internal`) | privado (`fn`) | No cruza ningún límite. Dejarlo privado documenta que es interno. |
| Helper de `db::X` usado por `db::Y` (`row_to_project`, `empty_to_null`, `get_links_for_projects`, `get_project_links_internal`, `PROJECT_COLUMNS`) | **`pub(super)`** | Visible en `db` y sus descendientes. Exactamente el alcance que hace falta, ni uno más. |
| Helper de `db` usado por `commands/`, `backup/`, `pdf_export/` | `pub(crate)` | **Hoy no existe ninguno.** Si alguna vez aparece, este es su nivel — no `pub`. |
| Método que consume un comando Tauri | `pub` | Ya lo son los 50. No se toca ninguno. |
| Submódulos de `db` | privado (`mod x;`) | Que ninguna ruta nueva se filtre al exterior del módulo. |

Regla mental: **empezá con lo más cerrado que compile, y solo aflojá cuando el
compilador te lo pida.** `pub(super)` primero; `pub(crate)` solo si `pub(super)`
no alcanza; `pub` nunca, salvo `TrashItem` que ya lo era.

---

## 4. Fases y plan de commits

**17 commits en 5 fases.** Todos en la rama `refactor/db-modulos`, todos en
español siguiendo `checkpoint: <descripción>` según `CLAUDE.md`.

### Fase 0 — Preparación y medición base

**Objetivo:** dejar por escrito el "antes" contra el cual se va a comparar el
"después". **No produce ningún commit de código.**

| Paso | Comando |
| ---- | ------- |
| Rama | `git switch -c refactor/db-modulos` |
| Baseline de tests | `cd src-tauri && cargo test 2>&1 \| tee /tmp/db-refactor-baseline-tests.txt` |
| Lista de tests | `cargo test -- --list \| sort > /tmp/db-refactor-baseline-testnames.txt` |
| Snapshot de símbolos | `rg -n '^\s*(pub )?fn \|^pub (struct\|enum\|const)' src/db/mod.rs \| sed -E 's/^[0-9]+:\s*//' \| sort > /tmp/db-refactor-baseline-symbols.txt` |
| Snapshot de consumidores | `rg -no 'db\.[a-z_]+\(\|db::[A-Za-z_]+' src --glob '!src/db/**' \| sort > /tmp/db-refactor-baseline-consumers.txt` |
| Clippy base | `cargo clippy --all-targets 2>&1 \| tail -20` |

**Medición ya tomada (2026-09-17, `main` en `839a8e0`):**
`cargo test` → **168 tests OK** (160 unitarios + 8 de `tests/security_config.rs`),
0 fallidos, exit code 0. Ese es el número que hay que ver al final de cada uno de
los 17 commits.

**Criterio de salida:** los cinco archivos de baseline existen y `cargo test`
cierra en verde.

---

### Fase 1 — Los habilitadores

**Objetivo:** que exista el piso compartido (transversales + helpers de test)
para que cualquier dominio pueda extraerse después sin arrastrar nada.

| Commit | Archivos tocados | Qué mueve |
| ------ | ---------------- | --------- |
| 1 | `db/mod.rs`, **`db/common.rs`** (nuevo) | `PROJECT_COLUMNS` (19–32), `row_to_project` (34–66), `empty_to_null` (367–374). Los tres pasan a `pub(super)`. |
| 2 | `db/mod.rs`, **`db/test_support.rs`** (nuevo) | Los 7 helpers de test (2034–2077, 2343–2411), todos `pub(super)`, bajo `#[cfg(test)]`. |

**Commits sugeridos:**
```
checkpoint: extrae db/common.rs con las piezas transversales de la capa SQLite
checkpoint: extrae db/test_support.rs con los helpers compartidos de test
```

**Criterio de salida por commit:** `cargo build` limpio, `cargo test` = 168 OK,
`cargo clippy --all-targets -- -D warnings` sin hallazgos nuevos — **retirando en
ese mismo commit los `use` que el corte dejó huérfanos en `mod.rs`** (regla 4 de
3.1). El `-D warnings` es estricto desde el commit 1: no se afloja en las fases
1–3 para reapretarlo en la 4.

---

### Fase 2 — Dominios independientes (las hojas)

**Objetivo:** vaciar el 55% del monolito sin tocar ninguna arista del grafo.
Estos ocho commits son intercambiables entre sí: si uno se complica, salteálo y
seguí con el siguiente.

| Commit | Archivo nuevo | Rango de producción | Rango de tests |
| ------ | ------------- | ------------------- | -------------- |
| 3 | `db/schema.rs` | 100–346 (`new`) | 2415–2435, 2802–2816, 3686–3844 (migraciones) |
| 4 | `db/maintenance.rs` | 347–366 | 2614–2643 |
| 5 | `db/attachments.rs` | 1209–1289 | 3216–3305 |
| 6 | `db/journal.rs` | 1290–1406 | 3018–3098 |
| 7 | `db/todos.rs` | 1407–1527 | 3099–3215 |
| 8 | `db/activity.rs` | 1079–1208 | 3308–3343 |
| 9 | `db/status.rs` | 1528–1627 | 3344–3440 |
| 10 | `db/tracking.rs` | 1896–2027 | 3614–3669 |

**Commits sugeridos:**
```
checkpoint: extrae db/schema.rs con la creación de tablas y las migraciones
checkpoint: extrae db/maintenance.rs con backup y verificación del archivo
checkpoint: extrae db/attachments.rs del monolito
checkpoint: extrae db/journal.rs del monolito
checkpoint: extrae db/todos.rs del monolito
checkpoint: extrae db/activity.rs con apertura, tiempo y estadísticas
checkpoint: extrae db/status.rs con estado, pin y orden de proyectos
checkpoint: extrae db/tracking.rs con las sesiones de time tracking
```

**Criterio de salida por commit:** idéntico a la Fase 1 — incluida la limpieza de
los `use` huérfanos de `mod.rs` dentro del mismo commit, que en esta fase es donde
más va a aparecer (ocho dominios salen de golpe).
**Nota sobre el commit 3:** es el más grande (247 líneas de producción) pero el
más inofensivo — `new()` no llama a nada de ningún otro dominio. El riesgo no es
técnico, es de dedo: cuidado con no perder ninguna de las **17** `ALTER TABLE`
de producción (líneas 143–293, todas dentro de `Database::new`). Contalas antes y
después **anclando en la comilla de apertura del string SQL**:

```bash
rg -c '"ALTER TABLE' src/db/schema.rs      # debe dar 17
```

> **Por qué la comilla y no `rg -c 'ALTER TABLE'` a secas:** el conteo global
> sobre el archivo da **19**, no 17, y confundiría al ejecutor. Son las 17 de
> producción **más 2 menciones del texto en comentarios** que viajan con este
> mismo commit: `mod.rs:3672` (*"las 21 `ALTER TABLE` de `Database::new()`"*) y
> `mod.rs:3748` (*"Las columnas que las ALTER TABLE agregan"*), ambas dentro del
> test de migraciones — que en esta tabla se mueve a `schema.rs` en los rangos
> 2802–2816 y 3686–3844. Las dos menciones usan backtick o texto pelado, nunca
> `"ALTER TABLE`, así que el ancla de la comilla las excluye y deja las 17 netas.
>
> Tampoco sirve anclar a principio de línea (`rg -c '^\s*"ALTER TABLE'`): da
> **16**, porque una de las migraciones va inline dentro de su `conn.execute(...)`
> en vez de ir sola en su línea.

> **Dato verificado, y no coincide con lo que dice el código:** el comentario del
> test de migraciones (`mod.rs:3672`) habla de *"las 21 `ALTER TABLE` de
> `Database::new()`"*. Contadas sobre el rango 100–346 son **17**. El comentario
> quedó desactualizado en algún ciclo anterior. **No lo corrijas durante este
> refactor** (regla 7 de la sección 7): el comentario se mueve tal cual está,
> se anota, y se arregla en un commit aparte después. El número contra el que
> validás es 17.

---

### Fase 3 — Dominios acoplados

**Objetivo:** mover lo que tiene aristas, en el orden proveedor → consumidor.
Acá el orden **sí** importa.

| Commit | Archivo nuevo | Rango producción | Qué expone / qué consume |
| ------ | ------------- | ---------------- | ------------------------ |
| 11 | `db/links.rs` | 898–1078 | **expone** `get_links_for_projects` + `get_project_links_internal` en `pub(super)` |
| 12 | `db/projects.rs` | 375–616, 841–897 | **consume** `common::*` + los dos de `links` |
| 13 | `db/groups.rs` | 1719–1895 | **consume** `common::*`, `links::get_links_for_projects`, `Database::get_project` |
| 14 | `db/dashboard.rs` | 1628–1718 | **consume** `common::{PROJECT_COLUMNS, row_to_project}` |
| 15 | `db/trash.rs` | 9–17 (`TrashItem`) + 617–840 | **mueve el tipo público**; `mod.rs` gana `pub use trash::TrashItem;` |

**Commits sugeridos:**
```
checkpoint: extrae db/links.rs con la carga en lote de enlaces
checkpoint: extrae db/projects.rs con el CRUD y la búsqueda de proyectos
checkpoint: extrae db/groups.rs con la jerarquía de grupos
checkpoint: extrae db/dashboard.rs con las consultas agregadas
checkpoint: extrae db/trash.rs con la papelera y el tipo TrashItem
```

**Criterio de salida por commit:** el mismo —con la limpieza de `use` huérfanos
de `mod.rs` en el propio commit—, más —solo en el commit 15— verificar
a mano que `commands/mod.rs:185` sigue compilando sin modificar (`crate::db::TrashItem`).

---

### Fase 4 — Fachada limpia y cierre

**Objetivo:** que `mod.rs` quede siendo una fachada y nada más, y que no queden
`use` huérfanos en ningún archivo.

| Commit | Archivos tocados | Qué hace |
| ------ | ---------------- | -------- |
| 16 | `db/mod.rs` | Deja solo: `use` mínimos, las 15 declaraciones `mod`, `pub use trash::TrashItem`, `struct Database`, `impl Database { fn conn() }`. Borra el `mod tests` vacío. |
| 17 | todos los `db/*.rs` | Limpieza final: los `use` huérfanos de `mod.rs` ya se fueron commit a commit (regla 4 de 3.1); acá se barren los que quedaron **en los archivos nuevos**, se corre `cargo fmt` sobre `src/db/`, y se cierra cualquier warning residual de `clippy`. |

**Commits sugeridos:**
```
checkpoint: db/mod.rs queda como fachada del módulo de base de datos
checkpoint: limpia imports y formato en los módulos de db
```

**Criterio de salida de la fase:**
- `wc -l src-tauri/src/db/mod.rs` ≈ 90.
- `cargo clippy --all-targets -- -D warnings` completamente limpio.
- `cargo fmt --check` limpio sobre `src/db/`.
- La verificación completa del punto 5.

---

### Resumen de fases

| Fase | Commits | Objetivo | Líneas que saca de `mod.rs` |
| ---- | ------: | -------- | --------------------------: |
| 0 | 0 | Medición base | 0 |
| 1 | 2 | Habilitadores (`common`, `test_support`) | ~190 |
| 2 | 8 | Hojas sin aristas | ~1.520 |
| 3 | 5 | Dominios acoplados | ~1.850 |
| 4 | 2 | Fachada + limpieza | ~190 |
| **Total** | **17** | | **~3.750** |

---

## 5. Validación y criterio de "listo"

### 5.1 Verificación por commit (barata, se corre 17 veces)

```bash
cd src-tauri
cargo build                                    # compila
cargo test                                     # 168 OK, 0 failed
cargo clippy --all-targets -- -D warnings      # sin hallazgos nuevos
```

Si `cargo test` da 167, **el commit está mal**: se perdió un test en el corte.
No sigas: `git reset` y rehacé el corte.

### 5.2 Verificación al final (la que prueba que no cambió el comportamiento)

**a) El set de tests es el mismo, nombre por nombre.**

Los tests cambian de ruta (`db::tests::X` → `db::projects::tests::X`), así que se
compara por el nombre final, no por la ruta completa:

```bash
cargo test -- --list | rg -o '[a-z_0-9]+: test$' | sort > /tmp/db-refactor-after-testnames.txt
# y contra el baseline, normalizado igual:
diff <(rg -o '[a-z_0-9]+: test$' /tmp/db-refactor-baseline-testnames.txt | sort) \
     /tmp/db-refactor-after-testnames.txt
```
**Debe dar vacío.** Si un test desapareció, se perdió en un corte.

**b) La superficie pública es la misma.**

```bash
rg -n '^\s*pub fn |^\s*pub\(crate\) fn ' src/db/*.rs | sed -E 's/^[^:]+:[0-9]+:\s*//' | sort \
  > /tmp/db-refactor-after-symbols.txt
diff /tmp/db-refactor-baseline-symbols.txt /tmp/db-refactor-after-symbols.txt
```
Las 50 firmas `pub fn` tienen que estar las 50, idénticas. Opcional si querés el
chequeo canónico: `cargo doc --no-deps --document-private-items` y comparar la
página de `Database` antes/después — o `cargo public-api` si estás dispuesto a
bancarte el toolchain nightly.

**c) Los consumidores no se tocaron.**

```bash
git diff --stat main..refactor/db-modulos -- src-tauri/src
```
**Solo puede aparecer `src-tauri/src/db/`.** Si aparece `commands/`, `backup/`,
`pdf_export/`, `main.rs` o `models/` en ese `--stat`, el refactor dejó de ser
puro y hay que entender por qué antes de seguir.

**d) Los 79 comandos siguen registrados.**

```bash
rg 'generate_handler' -A 200 src-tauri/src/main.rs | rg -o '^\s*[0-9]+-\s+[a-z_0-9:]+,' | wc -l
```
Debe dar **79**. Y `main.rs` no debe figurar en el `git diff --stat` de (c).

**e) Los movimientos fueron movimientos.**

```bash
git log --oneline main..refactor/db-modulos | rg -c 'checkpoint:'   # 17
git show --stat -C -M --find-copies-harder <sha>                    # por commit
```
Con `-C -M --find-copies-harder`, git debería reconocer los bloques movidos. En
la práctica, el chequeo que más sirve es el aritmético: en cada commit, las
líneas `+` del archivo nuevo y las `-` de `mod.rs` tienen que ser casi iguales
(la diferencia son la cabecera de `use` y las llaves del `impl`). Una asimetría
grande = alguien reescribió algo.

`git diff -w` sirve para confirmar que no hubo reindentación creativa, pero
**no** prueba un move puro por sí solo: úsalo como complemento, no como prueba.

**f) La app real arranca y opera.**

```bash
pnpm test              # Vitest — el frontend no se tocó, tiene que dar igual que en main
pnpm run tauri:dev     # levantar la app de verdad
```
Ver 6.7 para cómo hacerlo sin ponerle un dedo encima a la base de producción.

### 5.3 Definición de "listo"

El refactor está **listo** cuando, y solo cuando, se cumplen las ocho:

1. `cargo build` compila sin errores ni warnings nuevos.
2. `cargo test` = **168 tests OK**, y el `diff` de nombres de test da vacío.
3. `cargo clippy --all-targets -- -D warnings` limpio.
4. `pnpm test` (Vitest) igual que en `main`.
5. Los **79 comandos** siguen registrados en `generate_handler![]` y `main.rs` no
   aparece en el diff.
6. `git diff --stat main..refactor/db-modulos -- src-tauri/src` toca **solo**
   `src-tauri/src/db/`.
7. **Cero cambios de esquema**:
   `rg -c '"ALTER TABLE' src/db/schema.rs` da **17** (ver la nota del commit 3 de
   la Fase 2: sin el ancla de la comilla el conteo se va a 19 por dos menciones
   en comentarios del test de migraciones), y
   `rg -c 'CREATE TABLE|CREATE INDEX|PRAGMA' src/db/schema.rs` da los mismos
   conteos que el baseline sobre `mod.rs`.
8. La app levanta con `pnpm run tauri:dev` y se ejerce a mano el recorrido
   completo: crear proyecto → agregar enlace / adjunto / entrada de journal /
   todo → armar un grupo con subproyecto → mandarlo a la papelera → restaurarlo →
   abrir el dashboard → arrancar y parar una sesión de tracking.

Si falta una sola, no está listo. No es negociable.

---

## 6. Riesgos y mitigaciones

### 6.1 Borrow checker al mover métodos que usan el estado de `Database`

**Riesgo:** varios métodos hacen `let conn = self.conn();` y después abren una
transacción con `conn.unchecked_transaction()` o llaman a otro método de `self`
(el caso más delicado: `update_project` en 597–602 **libera la conexión a
propósito** antes de llamar a `self.get_project(id)`, justo para no deadlockear
contra su propio `Mutex`).

**Realidad:** el borrow checker **no sabe ni le importa en qué módulo vivís**.
Mover un `impl` no cambia ni un solo lifetime. Si compilaba antes, compila
después.

**Mitigación:** ninguna especial. Pero el comentario de la línea 597
(*"Liberar la conexión antes de llamar a get_project"*) **se mueve con el
código**. Ese comentario documenta un deadlock evitado y perderlo en el corte es
regalarle a la próxima persona un bug de 4 horas.

### 6.2 Visibilidad de rustc

**Riesgo:** el compilador escupe `private function` / `private item` al mover un
helper a un módulo hermano.

**Mitigación:** ya está mapeado en 1.4 y 3.4. Son **exactamente cuatro helpers y
una const**. Si aparece un quinto error de visibilidad que no estaba en esa lista,
**es señal de que un bloque se movió al módulo equivocado** — no lo tapes con un
`pub(crate)`: volvé y revisá el corte.

**Antipatrón a evitar:** resolver todo con `pub(crate)` "para no pelear". Eso
convierte helpers internos en API del crate y la próxima persona los va a usar
desde `commands/`. `pub(super)` primero, siempre.

### 6.3 Ciclos de dependencia entre dominios

**Riesgo:** `projects` necesita algo de `links` y `links` necesita algo de
`projects` → nada compila.

**Realidad:** en Rust los módulos del mismo crate **pueden** referenciarse
mutuamente; un "ciclo" entre módulos hermanos no es un error de compilación como
sí lo sería entre crates.

**Mitigación aun así:** el grafo de 1.4 es un DAG, sin ciclos. El orden de la
Fase 3 (proveedor antes que consumidor) mantiene esa propiedad. Y si en el
camino aparece una arista nueva que vuelve para atrás, la pregunta correcta no es
"¿cómo lo hago compilar?" sino **"¿este helper no pertenecía en realidad a
`common`?"**.

### 6.4 `pub use` que esconde o filtra símbolos

**Riesgo:** un `pub use links::*;` de más abre rutas nuevas (`db::links::…`)
hacia afuera y convierte en API pública cosas que no lo eran; un `pub use` de
menos rompe `commands/mod.rs:185`.

**Mitigación:**
- **Un solo `pub use` en todo el refactor:** `pub use trash::TrashItem;`. Si te
  aparece la tentación de un segundo, pará y preguntate qué se movió mal.
- **Nunca `pub use x::*;`.** Glob re-exports son exactamente el mecanismo que
  hace que después nadie sepa de dónde sale un símbolo.
- **Todos los `mod` hijos privados.** `mod projects;`, jamás `pub mod projects;`.
- El chequeo (b) de 5.2 detecta cualquiera de los dos errores.

### 6.5 Un commit gigante que rompe `git bisect`

**Riesgo:** mover todo de una hace que cuando dentro de tres meses aparezca un
bug, `git bisect` te deje parado sobre un commit de 3.800 líneas, que es lo mismo
que no tener bisect.

**Mitigación:** los 17 commits de la sección 4. El más grande (commit 3,
`schema.rs`) son ~400 líneas contando tests. **Cada commit compila y pasa los
168 tests**, que es la condición sin la cual `bisect` no funciona. Esa es toda la
razón de ser de la regla 2 de 3.1.

### 6.6 Duración y fatiga

**Estimación honesta:** entre **4 y 6 horas** de trabajo enfocado.

| Fase | Estimación |
| ---- | ---------- |
| 0 — medición | 15 min |
| 1 — habilitadores | 45 min (el primero siempre es el que más cuesta) |
| 2 — 8 hojas | ~2 h (≈15 min cada una) |
| 3 — 5 acopladas | ~1 h 30 (las que tienen aristas llevan más) |
| 4 — fachada + limpieza | 45 min |
| 5 — validación final + prueba manual | 45 min |

**Mitigación:** el riesgo real no es el reloj, es el cansancio en la fase 3, que
es cuando empiezan los "ya que estoy". La rama está diseñada para cortarse en
cualquier commit: **todo commit es un punto de parada válido**, porque todos
compilan y pasan tests. Si estás cansado, parás en el commit 10 con las hojas
afuera y seguís mañana. No hay ninguna fase que obligue a llegar hasta el final.

### 6.7 Probar la app real sin romper tu base de datos

**Riesgo:** correr `pnpm run tauri:dev` apunta a la base real del usuario. Si el
refactor tuviera un bug —no debería, pero para eso se prueba— podrías corromper
datos propios.

**Mitigación:**
1. **Copia de seguridad primero**, siempre. Localizá la base con
   `rg -n 'app_data_dir|gestor.*\.db' src-tauri/src/main.rs` y copiala aparte
   antes de la primera corrida.
2. Si el binario respeta una variable de entorno de base (revisar `main.rs`
   antes de asumirlo), levantá con una base descartable:
   `GESTOR_DB=/tmp/gestor-refactor.db pnpm run tauri:dev`. **Verificá que esa
   variable exista de verdad antes de confiar en ella** — si no existe, no
   inventes: usá el camino 1.
3. Alternativa más segura y sin depender de nada: `pnpm run tauri:dev` ya crea el
   esquema completo sobre una base vacía (los `CREATE TABLE IF NOT EXISTS` de
   `Database::new` lo garantizan), así que renombrar temporalmente la base real
   te da un entorno limpio y reversible.
4. El recorrido manual del punto 8 de 5.3 se hace sobre esa base descartable.

### 6.8 Perder comentarios en el corte

**Riesgo:** este archivo tiene documentación densa y valiosa — el doc comment de
`conn()` sobre el envenenamiento del mutex (B15), el de `PROJECT_COLUMNS` sobre
el contrato posicional, el de `get_links_for_projects` sobre el N+1 (B16), el de
`is_registered_project_path` sobre la revocación de permisos git. **Esos
comentarios son el registro de cuatro bugs ya pagados.**

**Mitigación:** cortá siempre **desde el primer `///` del bloque**, no desde el
`pub fn`. El baseline ya está medido: el código de producción (líneas 1–2027)
tiene **139 líneas de doc comment `///`**. Chequeo al final:

```bash
rg -c '^\s*///' src/db/*.rs | sed -E 's/^.*://' | awk '{t+=$1} END {print t}'
```

Tiene que dar **139 o más** (más, solo si agregaste un doc comment de módulo por
archivo — que está bien y es deseable). Si da menos, se perdió documentación en
algún corte.

### 6.9 `#[cfg(test)]` y helpers que quedan sin usar

**Riesgo:** al repartir los 86 tests, un módulo se queda con helpers de
`test_support` que ya no usa y clippy lo marca; o al revés, dos módulos necesitan
un helper que quedó local en uno.

**Mitigación:** `test_support.rs` es el único lugar donde viven los helpers
compartidos, y todos en `pub(super)`. Los `use super::super::test_support::*;`
con glob **están bien acá** (es código de test, no API) y evitan la lista
interminable. La limpieza fina va en el commit 17, no antes.

---

## 7. Qué NO hacer — explícito

Esto no es una lista de sugerencias. Cualquiera de estas cosas **convierte el
refactor en otra cosa** y hace que la validación del punto 5 deje de significar
nada.

1. **No cambiar el esquema SQLite.** Ni una tabla, ni una columna, ni un índice,
   ni un `PRAGMA`. Si ves una columna que "sobra", anotala en un issue y seguí.
2. **No tocar las migraciones existentes.** Las 17 `ALTER TABLE` con `let _ =`
   que ignoran el error son **deliberadas** (así es como se migra sin
   `user_version` en este proyecto). Se mueven textualmente. No las "mejores".
3. **No renombrar comandos Tauri.** Los 79 nombres de `generate_handler![]` son
   el contrato con el frontend. `main.rs` **no aparece en el diff**, punto.
4. **No renombrar métodos de `Database`.** Las 50 firmas `pub fn` quedan idénticas.
   Si un nombre te parece malo, va a otro plan.
5. **No migrar a un ORM** (`diesel`, `sea-orm`, `sqlx`). Es un cambio de
   comportamiento disfrazado de refactor: cambia el manejo de errores, los tipos,
   las transacciones y probablemente el SQL generado. Otro plan, otra discusión.
6. **No agregar features nuevas.** Ni un método, ni un comando, ni un campo.
7. **No arreglar bugs en el mismo commit.** Si encontrás uno moviendo código —y
   es probable— anotalo, terminá el refactor, y arreglalo en un commit aparte
   **después**, sobre la base ya modularizada. Esa es exactamente la lección del
   B14 original.
8. **No reformatear de más.** Nada de `cargo fmt` sobre todo el crate: el ruido
   tapa el movimiento real. `cargo fmt` solo sobre `src/db/`, y solo en el
   commit 17.
9. **No tocar `commands/mod.rs`.** Es el segundo monolito (2.043 líneas) y tiene
   su propio problema. Meterlo acá duplica el riesgo y arruina la trazabilidad
   de ambos.
10. **No hacer `push` ni abrir PR hasta que las ocho condiciones de 5.3 estén
    verdes.** Una rama de refactor a medias es peor que no haberla empezado.

---

## 8. Cierre

El monolito de `db/mod.rs` parece un problema grande: 3.844 líneas, 13 dominios
mezclados, 79 comandos colgando de él. Pero cuando lo mirás de verdad:

- La mitad son tests, y **ya están agrupados por dominio** en el archivo.
- La superficie pública son **dos símbolos**.
- El grafo de acoplamiento interno tiene **seis aristas**, y es un DAG.
- Y Rust permite repartir los `impl` inherentes entre módulos del mismo crate,
  así que **los 50 métodos no necesitan ni un `pub use`**.

Eso no es un refactor peligroso. Es un refactor **aburrido**, que es exactamente
lo que un refactor tiene que ser. El riesgo no está en la técnica: está en la
tentación de aprovechar el viaje para arreglar otras cosas.

No lo hagas. Mové código, corré los tests, commiteá. Diecisiete veces.
