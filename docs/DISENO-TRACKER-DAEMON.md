# Diseño: `gestor-trackerd`, el demonio de medición de tiempo

> **Estado:** aprobado (v3.1). No hay nada implementado. Las decisiones DX1–DX5
> de la sección 9.2 quedaron **confirmadas por el usuario el 2026-10-18** con la
> propuesta de cada una. DX3/DX4 siguen pidiendo un "sí" caso por caso antes de
> reescribir filas (commit 55). DX5 (CI) se agrega cuando se autorice el push a GitHub.
> **Fecha:** 2026-10-18 · **Base:** `main@c8b55ae` (B14 ya mergeado)
> **Alcance:** solo Linux (systemd). Instalación soportada: `install.sh` / `scripts/release.sh --install`.
> **Plan:** 10 fases (F0–F8, con F5 partida en F5a y F5b), **56 commits** `checkpoint:`
> más el `release:` que genera el script. Estimación: **20–27 días efectivos** (4.11).
> **Revisión:** la v1 pasó una revisión adversarial con 25 hallazgos (8 críticos),
> incorporados en la v2. La v2 pasó una auditoría con **59 hallazgos** (9
> bloqueantes), incorporados en la v3; la v3.1 aplica la verificación de la v3
> contra el código. Resumen en la sección 11 y detalle en el Anexo A.

Un programa aparte, `gestor-trackerd`, corre en segundo plano como servicio de
systemd del usuario. Mira los archivos de cada proyecto y registra **en vivo** el
tiempo en que realmente se trabajó en cada uno, esté la app abierta o cerrada. La
app pasa a ser la que **muestra** ese tiempo: contadores por proyecto que avanzan
con cada guardado y un indicador del estado del demonio. El botón "Trabajar" sigue
existiendo, y su tiempo se combina con el del demonio sin contar doble. El demonio
se puede apagar desde la app desde el primer día en que se instala.

---
## Camino rápido

| Fase | Commits | Qué entrega | Por qué va ahí |
|---|---|---|---|
| **F0** | 1–9 | Puerta de calidad (`cargo fmt` + `scripts/check.sh`) y arreglos que el demonio hereda: cierre idempotente, base abierta **después** de la instancia única, sin `try_lock`, errores visibles, botón Parar, regla única de duración mínima, Dashboard igual a "Trabajar" | Son bugs de hoy que con dos procesos se vuelven graves; sin `check.sh` ningún commit posterior tiene puerta |
| **F1** | 10–20 | Esquema versionado (`user_version`), `last_activity_at`, `tracker_status`, recuperación de huérfanas, **unión de tramos** en hora local, tiempo visible sin `.gestor/`, diff con datos reales, fin del contador | Sin esto, demonio + "Trabajar" cuentan doble, y ninguna columna nueva entra sin versión |
| **F2** | 21–27 | Rutas compartidas, restore seguro, transacciones `IMMEDIATE`, permisos `0600`, después WAL | Sin esto, app y demonio se bloquean o el restore corrompe la base. Se valida sobre una **copia de la base real** |
| **F3** | 28–30 | `lib.rs`, binario `gestor-trackerd` con lock y guardas, sección `tracking` de la config | El demonio reutiliza `db/` sin duplicarlo; la config existe antes de que alguien la lea |
| **F4** | 31–35 | Motor del demonio: lógica pura, reloj inyectado, atribución, filtros, escritor | Lo difícil se prueba primero, sin disco real |
| **F5a** | 36–38 | Vigilante (`notify` + `ignore`, conteo de referencias, desborde, tope) y lector de config | La parte que toca el disco, todavía sin bucle |
| **F5b** | 39–42 | Bucle, `sd-notify`, señales, kill switch del lado del demonio, y los **E2E** | El demonio corre a mano de punta a punta; desde acá cada fase pasa los E2E |
| **F6** | 43–48 | Instalación desde la app y los scripts, **toggle e indicador mínimos**, E16 y E22 con `systemd-run`, restore con systemd | "Se levanta la primera vez y queda corriendo", y nunca sin forma de apagarlo |
| **F7** | 49–53 | Contadores en vivo, indicador de 6 estados, Ajustes completos, avisos por carpeta | Lo que el usuario ve |
| **F8** | 54–56 | Medición de recursos, script de datos históricos (DX3/DX4), corrida triple de E2E, documentación y release `minor` | Definición de "listo" (sección 6) |

Al terminar cada fase, `main` compila, `scripts/check.sh` pasa, la app se puede
usar como antes (o mejor) y la fase se mergea sola con fast-forward. F0–F2 ya
mejoran el tracking actual antes de que exista el demonio. Hasta F6 el demonio no
se instala en ningún lado: se corre a mano, con una base temporal.

---
## 1. Requisitos

| # | Requisito | Origen |
|---|---|---|
| R1 | Medición **en vivo y real** por proyecto. Con varios proyectos activos a la vez, cada uno suma **sus propios tramos**, sin repartir. Si un proyecto contiene a otro, el cambio cuenta para el **más específico**; si dos tienen la misma carpeta, para los dos (3.4) | "no no no, quiero medición en vivo, real!!!" |
| R2 | Se mide **actividad en archivos**: guardar, crear, renombrar o borrar dentro de la carpeta del proyecto. Leer no cuenta, ni lo que ignora el `.gitignore`. Cada sesión acredita un margen de hasta **60 s** antes de su primer cambio (DX1, confirmada, 9) | Definición acordada; margen propuesto en la v3 |
| R3 | Corte por inactividad: **3 minutos** sin cambios cierran la sesión (configurable de 1 a 60). La sesión termina en el **último cambio real** | Propuesta aceptada |
| R4 | "Trabajar" **convive** con el demonio. Si se superponen, se cuenta la **unión** de los tramos | Aprobado |
| R5 | **Solo Linux** | Aprobado |
| R6 | El demonio **sigue corriendo con la app cerrada**. Se instala y arranca la primera vez que se abre la app instalada, y desde ahí arranca solo en cada inicio de sesión. Se puede apagar desde Ajustes, y la app no lo vuelve a encender si el usuario lo deshabilitó con `systemctl` (3.8) | "la app cerrada el demonio tiene qué seguir corriendo" |
| R7 | Carpetas que no se pueden medir bien **no se vigilan**: demasiado amplias (`/`, `/home`, `$HOME`), inexistentes, sin permiso, en un disco de red, o con más carpetas que el tope (DX2, confirmada, 9). La app las marca, dice por qué y el usuario las corrige | Propuesta aceptada; motivos ampliados en la v3 (3.5) |

**Fuera de alcance:**
- Windows y macOS.
- Medir sin sesión iniciada (lingering).
- Editar sesiones a mano.
- Reportes nuevos.
- Paquetes `.deb`/`.rpm`/AppImage: la app **no instala** el demonio si no corre
  desde `BIN_DIR` (`~/.local/bin`), y lo informa (3.8). **Pendiente de verificar**
  si el bundler de Tauri 2 mete en el paquete el segundo binario (tarea del commit
  29, B4). Si lo mete, esta línea se corrige, pero el comportamiento no cambia: un
  `gestor-trackerd` en `/usr/bin` no se instala solo.
- Corregir datos históricos **sin consentimiento**. Las sesiones de más de 12 h,
  las huérfanas y los proyectos con carpeta inexistente (cifras en 2.1) se listan
  en el diff con datos reales (commit 19) y se deciden en 9.2 (DX3, DX4). El código
  nuevo no reescribe filas viejas.

---
## 2. Punto de partida

Relevado sobre `main@c8b55ae`. Rutas relativas a `src-tauri/src` salvo indicación.
Los números de la base real se midieron en modo solo lectura sobre
`~/.local/share/gestor-proyectos/projects.db` el 2026-10-18.

### 2.1 Cómo se mide hoy

- Hay una sola sesión a la vez y vive en memoria (`ActiveSession`, `commands/mod.rs:17-37`).
  "Trabajar" la abre (`:1502`), y se cierra al abrir otra o al cerrar la ventana
  (`main.rs:78-92`).
- La duración se mide con `Instant`, que es monotónico y **no avanza durante una
  suspensión**. Si la app muere sin cerrar la ventana, la fila queda con
  `ended_at = NULL` para siempre. Esto se vio en la prueba manual del B14.
- `time_tracking_sessions` (`db/schema.rs:205-217`) no tiene restricciones que
  impidan dos sesiones abiertas del mismo proyecto.
- `end_tracking_session` siempre cierra con `ended_at = CURRENT_TIMESTAMP`
  (`db/tracking.rs:32`): no hay forma de cerrar una sesión en otro instante.
- Regla de los 10 s: una sesión de 10 s o menos se guarda con
  `duration_seconds = 0`, pero **solo** al cambiar de proyecto (detalle y líneas
  en 3.2, "Regla única de duración mínima").
- "Trabajar" crea `.gestor/` en el proyecto si no existe
  (`commands/mod.rs:1519-1524`), y la tarjeta solo muestra estadísticas si existe (P15).

**La base real hoy:** `user_version = 0`, `journal_mode = delete`,
`page_size = 4096`. Hay 193 sesiones (190 `work_button`, 2 `shell_hook`, 1
`manual`); **81** quedaron abiertas para siempre; **9** duran más de 12 h y suman
936.392 s, el **69 %** de los 1.350.421 s registrados (la 191, de 96.653 s, es una
de ellas). El contador `total_time_seconds` difiere de la suma de sesiones en un
solo proyecto (id 3: 30.348 s contra 38.448 s).

### 2.2 Problemas actuales que el demonio vuelve graves

| # | Problema | Dónde | Con dos procesos |
|---|---|---|---|
| P1 | Hay **dos totales**: `SUM(duration_seconds)` y el contador `projects.total_time_seconds`. Este también lo sube `add_project_time` sin crear sesión | `db/tracking.rs:94-101`, `db/activity.rs:28-38,63-68` | Divergen, y con superposición los dos cuentan doble |
| P2 | `end_tracking_session` no verifica `ended_at IS NULL`, así que cerrar dos veces suma dos veces. Agregar el filtro al primer `UPDATE` no alcanza: el segundo, el del contador, suma igual aunque el primero no haya cerrado nada | `db/tracking.rs:26-47` (el segundo `UPDATE` en `:38-43`) | Una carrera entre app y demonio inventa tiempo |
| P3 | "Hoy", "semana" y `active_today` se calculan en **UTC**. Además, la "semana" (`started_at >= DATE('now','-7 days')`) abarca **8 fechas** | `db/tracking.rs:121-140`, `db/activity.rs:54-61` | En Argentina el día cambia a las 21:00 |
| P4 | Las sesiones huérfanas nunca se recuperan (hoy hay 81) | — | Se pierde tiempo, o se infla si se calcula "hasta ahora" |
| P5 | El badge de la tarjeta carga una vez y no se refresca | `TimeTracker.tsx:265-338` | No hay "en vivo" posible |
| P6 | `restore_backup_to` reemplaza el archivo con `fs::rename` | `backup/mod.rs:427-499` | El demonio escribe en el archivo viejo, y con WAL un `-wal` sobrante **corrompe** el restaurado |
| P7 | `handleOpenTerminal` se traga el error de `startWorkSession` | `src/components/ProjectList.tsx:209-213` | Fallos invisibles |
| P8 | "Abrir" desde el Dashboard no mide | `src/components/Dashboard.tsx:55-65` | Dos botones iguales y uno no cuenta |
| P9 | No hay botón para parar | `src/services/api.ts:507` sin uso | Solo se para cerrando la app |
| P10 | `start-app.sh` hace `pkill -f "gestor-proyectos"` | `start-app.sh:19-21` | Mata al demonio: su `ExecStart` lleva `--data-dir …/gestor-proyectos` (3.7) |
| P11 | La base se abre, se migra, se lee la config y se hace el auto-backup **antes** del control de instancia única | `main.rs:29-53` contra `:60` | Lo que se ejecute al arrancar corre también en la segunda instancia, que va a salir: un segundo auto-backup, una segunda migración |
| P12 | `ConfigManager` crea el archivo si falta y **renombra a `.corrupto-*`** la config que no entiende | `config/manager.rs:63-75,82-134` | Un demonio de otra versión borraría la config del usuario |
| P13 | Hay 6 transacciones diferidas (`unchecked_transaction`). Dos **leen antes de escribir**: `restore_project` (`trash.rs:68-80`) y `purge_project` (`trash.rs:145-159`). Además, las dos validan la guarda (`SELECT deleted_at`) **fuera** de la transacción (`trash.rs:56-66` y `:133-143`), y `empty_trash` lee los ids fuera también (`trash.rs:205-211`) | `db/tracking.rs:28`, `db/status.rs:75`, `db/trash.rs:22,68,145,211` | Con WAL, las que leen primero fallan con `SQLITE_BUSY_SNAPSHOT` si el otro proceso escribe entre la lectura y la escritura, y `busy_timeout` no las salva. Las otras cuatro empiezan escribiendo y solo esperan |
| P14 | Las migraciones **no tienen versión**: 21 `let _ = conn.execute(...)` (17 `ALTER TABLE` y 4 `CREATE INDEX`) descartan **cualquier** error, no solo "la columna ya existe" | `db/schema.rs:50-188,220-227`; no hay `PRAGMA user_version` en el código | Un `SQLITE_BUSY` porque el otro proceso escribe se traga en silencio: la columna no se crea y el error aparece después, lejos de su causa. Tampoco hay forma de saber si la base es de una versión más nueva |
| P15 | La tarjeta solo carga estadísticas si existe `.gestor/` en la carpeta del proyecto | `src/components/TimeTracker.tsx:276-284` | El demonio no crea `.gestor/`: el tiempo medido sin haber usado "Trabajar" **no se ve** |
| P16 | `get_project` y `update_project` toman la conexión con `try_lock`: si el Mutex está ocupado devuelven `SQLITE_BUSY` al instante. Tampoco se recuperan del envenenamiento, al revés que `conn()` | `db/projects.rs:113,154` contra `db/mod.rs:50-57` | "Trabajar" llama primero a `get_project` (`commands/mod.rs:1511`). Con latidos y escrituras que esperan al demonio, el Mutex está tomado más seguido y "Trabajar" falla más; con P7 resuelto, el fallo se ve |
| P17 | La ruta de la base (`live_db_path`) vive en `backup/`, que depende de `ConfigManager` y de `platform` | `backup/mod.rs:394-401`, usada en `main.rs:29` | El demonio necesita la misma ruta sin arrastrar el módulo de backups |
| P18 | Las estadísticas convierten un error de base en 0 s (`.unwrap_or(0)`) | `db/tracking.rs:101,119,130,141`, `db/activity.rs:61,68` | Un `SQLITE_BUSY` se muestra como "no trabajaste", sin aviso |
| P19 | La base es legible por cualquier usuario: `projects.db` en `0644` y su carpeta en `0755` (medido) | `~/.local/share/gestor-proyectos/` | El demonio corre siempre; los `-wal`/`-shm` y los backups heredan esos permisos |

### 2.3 Lo que juega a favor

- `db/`, `models/`, `tracking/`, `config/` y `logging.rs` **no dependen de Tauri** (verificado: ninguno nombra `tauri`). Solo lo hacen `main.rs` y `commands/mod.rs`.
- El B14 dejó `db/` partido por dominio.
- `busy_timeout = 5000` ya está (`db/schema.rs:21`) y `foreign_keys = ON` también (`:32`). Los dos se mantienen como requisito.
- La ruta de datos sale de `dirs::data_local_dir()` (`backup/mod.rs:395`), que respeta `XDG_DATA_HOME`: los tests pueden aislar la base sin tocar la real.
- La config se guarda de forma atómica (temporal + `rename`, `config/manager.rs:140-153`), así que leerla desde otro proceso es seguro.
- No hay ninguna fecha en RFC3339 en `time_tracking_sessions` (medido): todo está en el formato de `CURRENT_TIMESTAMP`.
- Medido: 63 carpetas reales suman **~52.400 directorios** respetando `.gitignore`. El límite de inotify es **524.288** (10% de uso).

---
## 3. Arquitectura

```
                    ┌──────────────── systemd --user ────────────────┐
 archivos de los    │  gestor-trackerd                               │
 proyectos ────────►│   ├─ Vigilante (notify + ignore, 1 hilo)       │
 (inotify)          │   │     evento ─► filtro ─► proyecto(s)        │
                    │   ├─ Motor (lógica pura, reloj inyectado)      │
                    │   │     abrir / extender / cerrar              │
                    │   └─ Escritor (rusqlite, IMMEDIATE, no migra)  │
                    │         sesiones · tracker_status cada 15 s    │
                    └───────────────────────┬────────────────────────┘
                                            ▼
            ~/.local/share/gestor-proyectos/projects.db  (WAL, 0600, user_version)
            ~/.local/share/gestor-proyectos/trackerd.lock (flock)
                                            ▲
                    ┌───────────────────────┴────────────────────────┐
                    │  Gestor de Proyectos (Tauri)                   │
                    │   migra · lee cada 5 s · "Trabajar" · servicio │
                    └────────────────────────────────────────────────┘
```

**Principios:**
1. **La base es el único canal** entre app y demonio: no hay sockets ni IPC.
2. **El motor es puro.** Recibe eventos con su instante y devuelve acciones, así que se testea con un reloj falso.
3. **Se guardan tramos y el total se calcula.** No hay contadores acumulados.
4. **Nunca inventar tiempo.** Ante la duda (huérfanas, eventos perdidos, suspensión) se cuenta de menos, nunca de más. La única excepción es explícita y acotada: el margen de `min_credit_seconds` antes del primer cambio de una sesión (3.4).
5. **Solo la app migra.** El esquema tiene versión (`PRAGMA user_version`). El demonio nunca altera el esquema: si la base está atrasada, espera; si es de una versión más nueva que la que conoce, no escribe sesiones (3.2).
### 3.1 Estructura del crate

**Decisión: biblioteca y segundo binario dentro del mismo paquete `src-tauri`.**

```
src-tauri/
├── Cargo.toml          # default-run = "gestor-proyectos" + [lib] gestor_proyectos_lib + [[bin]] gestor-trackerd
└── src/
    ├── lib.rs          # pub mod db; models; tracking; trackerd; time; paths; config; logging;
    ├── main.rs         # la app: use gestor_proyectos_lib::{db, models, …}
    ├── bin/gestor-trackerd.rs   # punto de entrada fino del demonio
    ├── paths.rs        # NUEVO: rutas de datos, base, lock y config (P17)
    ├── time.rs         # NUEVO: formato único de fechas y rangos locales (3.3)
    └── trackerd/       # NUEVO: motor, vigilante, escritor, servicio
```

| Opción | A favor | En contra | Veredicto |
|---|---|---|---|
| **A. lib + bin en el mismo paquete** | Cambio chico. Es el patrón de Tauri 2 (lib `*_lib` + `main.rs`). `release.sh` sigue con un solo manifiesto | El demonio compila con el árbol de dependencias de Tauri | **Elegida** |
| B. Workspace con `gestor-core`, `trackerd` y `src-tauri` | El demonio compila sin WebKit | Otro movimiento grande de `db/` justo después del B14. `release.sh:91` reescribe versiones con `sed` sobre un solo `Cargo.toml` | Para más adelante, si crece |

La lib se llama `gestor_proyectos_lib`, para no chocar con el binario
`gestor-proyectos` en `cargo doc`. `config` va a la lib, pero el demonio **no**
usa `ConfigManager` (P12): tiene su lector propio (3.5). `backup/` **no** va a la
lib: depende de `ConfigManager` y de `platform` (`backup/mod.rs:66-84`), y el
demonio no hace backups. El **núcleo del restore**, en cambio, sí va a la lib
(A3): hoy `restore_backup_to` es privada de `backup/` (`backup/mod.rs:427`) y un
test de integración en `tests/` no la puede llamar. El commit 22 lo escribe como
`db::restore::tomar_lock` + `db::restore::restaurar_desde` (lock, `page_size`,
`Backup::step(-1)`, `migrar`, `integrity_check`, 3.6); entra en la lib con el resto de
`db/` en el commit 28, y `backup::restore_backup` queda como envoltorio de la app
(validación del nombre, `ActiveSession`, systemd, salida).

**`default-run = "gestor-proyectos"` en `[package]`.** Hoy `Cargo.toml` no lo
tiene porque hay un solo binario. Con dos, `cargo run` sin `--bin` falla con
"could not determine which binary to run", y `tauri dev` lanza la app con
`cargo run`. Va en el **mismo** commit que agrega el `[[bin]]`, no después.

**`paths.rs` (P17):** una sola fuente de verdad para las rutas, sin efectos
secundarios salvo crear la carpeta de datos.

| Función | Devuelve | Hoy |
|---|---|---|
| `data_dir()` | `dirs::data_local_dir()/gestor-proyectos`, creada con `0700` (D11, 3.6) | `backup/mod.rs:395-399`, creada con el umask |
| `db_file()` | `data_dir()/projects.db` | `backup::live_db_path` (`backup/mod.rs:394-401`) |
| `trackerd_lock_file()` | `data_dir()/trackerd.lock`, **siempre** junto a la base (3.4) | — |
| `config_file()` | `dirs::config_dir()/gestor-proyectos/config.json`, sin crear nada | `ConfigManager::get_config_dir` (`config/manager.rs:39-60`) más `get_config_path` (`:28-36`), que sí crea la carpeta |

`backup::live_db_path` desaparece: `main.rs:29` y `backup::restore_backup`
(`backup/mod.rs:404-407`) pasan a `paths::db_file()`. `ConfigManager` delega en
`paths::config_file()` y sigue creando la carpeta él mismo; el lector del
demonio usa la misma función y nunca crea nada.

**Empaquetado de Tauri (pendiente de verificar).** `tauri.conf.json:57-59` tiene
`"bundle": { "active": true, "targets": "all" }`. No está comprobado si el
bundler de Tauri 2 mete en el `.deb`/AppImage **todos** los `[[bin]]` del paquete
o solo el principal, ni si `pnpm tauri build` compila los dos. Hay una tarea de
verificación en el commit del `[[bin]]`: `pnpm tauri build --bundles deb` y
`dpkg -c` sobre el resultado, y ver qué queda en `target/release/`.
`release.sh:302` usa `cargo build --release`, que compila todos los binarios, así
que la instalación soportada no depende del resultado. El diseño funciona en los
dos casos, porque el ciclo de 3.8 solo instala el servicio si el binario de la
app está en `~/.local/bin` (B9): un `gestor-trackerd` en `/usr/bin` venido de un
paquete no se instala solo. Si la verificación muestra que el paquete **sí** lo
trae, se corrige la línea de §1 "Fuera de alcance" que dice que no.
### 3.2 Modelo de datos

**Esquema versionado (P14).** Las migraciones pasan a tener número, guardado en
`PRAGMA user_version`. La base real está hoy en `0`.

| Paso | Contenido | Commit |
|---|---|---|
| 1 | Todo lo que hoy hace `Database::new` (`db/schema.rs:34-227`): las `CREATE TABLE IF NOT EXISTS`, las 17 `ALTER TABLE` y los 4 índices. Es la línea de base: sobre una base existente no cambia nada | Migraciones versionadas |
| 2 | `time_tracking_sessions.last_activity_at` + índice `(project_id, ended_at)` | `last_activity_at` |
| 3 | Tabla `tracker_status` | `tracker_status` |

`migrar(conn)`, que **solo corre la app**:

1. Fuera de toda transacción: `busy_timeout`, `foreign_keys`, `journal_mode` y `synchronous` (3.6). `journal_mode` no se puede cambiar dentro de una transacción.
2. `BEGIN IMMEDIATE`. Si la base sigue ocupada después de `busy_timeout`, el error sube: no se sigue con el esquema a medias.
3. Lee `user_version`. Si es **mayor** que `ESQUEMA_ACTUAL`, hace rollback y devuelve: "La base de datos es de una versión más nueva de Gestor de Proyectos. Actualizá la app antes de abrirla."
4. Aplica los pasos `user_version + 1 ..= ESQUEMA_ACTUAL`. Un `ALTER TABLE … ADD COLUMN` que falla con `duplicate column name` se ignora (la columna ya estaba, por ejemplo en el paso 1 sobre una base real); **cualquier otro error se propaga**. Los `CREATE INDEX` ya no van con `let _`.
5. `PRAGMA user_version = ESQUEMA_ACTUAL` y `COMMIT`. `user_version` vive en la página 1 de la base, así que entra en la misma transacción: o queda todo o no queda nada.

**El demonio no migra.** Abre la base sin `SQLITE_OPEN_CREATE` (si no existe,
espera a que la app la cree) y lee `user_version` y `journal_mode` antes de
escribir nada. Esta tabla es la única definición; el orden de arranque (3.4) la
referencia:

| Estado de la base | Qué hace el demonio |
|---|---|
| `user_version = ESQUEMA_ACTUAL` y `journal_mode = wal` | Arranca normal |
| `user_version <` (primer arranque tras actualizar, o restore en curso) **o** `journal_mode ≠ wal` (la app todavía no lo activó, 3.6) | No vigila ni escribe sesiones. Reintenta cada 5 s, registrándolo en el log, hasta que la app migre. Si la tabla `tracker_status` ya existe, pone `estado = 'esperando_migracion'`. Mientras espera manda `EXTEND_TIMEOUT_USEC` para no chocar con `TimeoutStartSec`; pasados 2 min avisa `READY=1` y sigue esperando ocioso. Sigue leyendo la config cada 5 s: el kill switch se respeta también acá (3.4) |
| `user_version >` | No escribe sesiones. Si `tracker_status` existe, deja `estado = 'esquema_nuevo'` y en `last_error` "El tracker está desactualizado: reinstalalo", y queda ocioso. No sale con error, para no gastar reinicios. Lo vuelve a mirar en cada relectura (60 s, 3.5): el `try-restart` de la próxima instalación lo reemplaza (3.8) |

**`time_tracking_sessions`:**

| Columna | Cambio | Para qué |
|---|---|---|
| `source` | Valores `work_button` y `file_watch`. `shell_hook` y `manual` quedan solo en filas viejas (hoy 2 y 1) | Saber quién la creó |
| `last_activity_at DATETIME` | **Nueva** (paso 2). Se escribe **al abrir** (= `started_at`) en las dos fuentes | Latido de la sesión, y hora real para recuperar huérfanas |
| `last_activity_at` en `file_watch` | El último cambio real. El demonio la vuelca al abrir y después con el ritmo de 3.4 | El contador en vivo nunca pasa del último cambio (R3) |
| `last_activity_at` en `work_button` | El latido de "Trabajar", cada 60 s | Saber si la sesión de otra instancia está viva |
| `duration_seconds` en una `work_button` **abierta** | El mismo latido escribe lo medido con `Instant` hasta ese momento | Una huérfana se recupera con tiempo medido, que **excluye la suspensión**. Con `last_activity_at − started_at` se contaría una suspensión que haya pasado entre dos latidos |
| Índice `(project_id, ended_at)` | **Nuevo** (paso 2) | Encontrar rápido las sesiones abiertas |

**Cómo se cierra cada sesión (M7).** Todo cierre escribe `ended_at` y
`duration_seconds` juntos, con la invariante
`ended_at − started_at ≥ duration_seconds`, que es **igualdad** en `file_watch`:

| Cierre | `ended_at` | `duration_seconds` |
|---|---|---|
| `file_watch` por inactividad, papelera, cambio de carpeta o SIGTERM | `ultimo_cambio` | `ended_at − started_at` |
| `work_button` al cambiar de proyecto, con Parar o al cerrar la ventana | Ahora (pared) | Lo medido con `Instant`, con la regla mínima de abajo |
| Huérfana `file_watch` (el demonio, al arrancar) | `COALESCE(last_activity_at, started_at)` | `ended_at − started_at`. Si da 0, la fila se borra |
| Huérfana de otra fuente (la app, en `.setup()`, solo si `COALESCE(last_activity_at, started_at)` tiene más de 2 min) | `COALESCE(last_activity_at, started_at)` | `MIN(COALESCE(duration_seconds, 0), ended_at − started_at)`, con la regla mínima |

Las 81 huérfanas reales no tienen `last_activity_at` ni `duration_seconds`:
cierran en su `started_at` con 0 s. Hoy ya suman 0, así que el total no cambia.

**Métodos de `Database` con instante explícito (D14).** Nadie más escribe
`CURRENT_TIMESTAMP` en las sesiones:

- `abrir_sesion(project_id, source, started_at) -> Result<i64>`: escribe `started_at` y `last_activity_at`.
- `latir(id, last_activity_at, duration_seconds: Option<i64>) -> Result<bool>`: `WHERE id = ? AND ended_at IS NULL`. Devuelve `false` si no tocó ninguna fila (sesión cerrada o proyecto purgado, 3.4).
- `cerrar_sesion(id, ended_at, duration_seconds) -> Result<bool>`: `WHERE id = ? AND ended_at IS NULL`. Devuelve `false` si ya estaba cerrada.

`create_tracking_session` y `end_tracking_session` quedan como envoltorios con
"ahora" para no tocar los llamadores de golpe.

**Regla única de duración mínima (M8).** Hoy la regla de los 10 s se aplica solo
al cambiar de proyecto (`commands/mod.rs:1569-1579`); Parar y cerrar la ventana
(`:1615-1621`) guardan cualquier duración, y el cierre con 0 s descarta su error
(`let _`, `:1578`). Pasa a una sola función, `duracion_a_guardar(source, segundos)`,
que usan **todos** los cierres:

- `work_button` y fuentes viejas: 10 s o menos se guardan como 0. La fila se conserva, con su `ended_at`, pero no cuenta como sesión (3.3). El error del cierre se propaga.
- `file_watch`: sin regla de 10 s; la cubre el margen de abajo.
- Las filas existentes **no se reescriben**: las 5 sesiones reales de 1 a 10 s siguen sumando lo que suman. Aparecen en el diff de datos reales (D3).

**Guardados espaciados: margen mínimo (DX1, confirmada).** Cada sesión
`file_watch` acredita hasta `tracking.min_credit_seconds` (60 s por defecto)
**antes** de su primer cambio, así que su `started_at` ya viene corrido hacia
atrás. La fórmula, sus cotas y el caso `0` están en 3.4 ("Margen por guardado");
la superposición con "Trabajar" la resuelve la unión (3.3).

**Tabla nueva `tracker_status`** (paso 3, una sola fila, `id = 1`). Solo la
escribe el demonio; la app la lee. Sin fila significa que el demonio nunca corrió:

| Columna | Contenido |
|---|---|
| `pid`, `version`, `started_at` | Identidad del proceso; `pid = NULL` al salir limpio (3.4). `version` se compara con la de la app para detectar actualizaciones (3.8) |
| `schema_version` | El `ESQUEMA_ACTUAL` que conoce ese binario |
| `estado` | `activo`, `desactivado` (leyó `daemon_enabled = false`), `esperando_migracion` o `esquema_nuevo` |
| `last_heartbeat` | Cada 15 s. El demonio está vivo si `ahora − last_heartbeat < 45 s` |
| `idle_minutes`, `min_credit_seconds` | Valores **efectivos** que usa el demonio, ya acotados. La UI muestra cuándo se aplicó un cambio de config |
| `watched_projects`, `watched_dirs` | Contadores para Ajustes |
| `scan_completed_at` | Fin del recorrido inicial (NULL mientras recorre). El indicador "Preparando" (3.9) y los E2E con centinela la esperan |
| `unwatched_projects` | JSON: `[{"id": 30, "motivo": "carpeta_amplia"}, …]`. Los motivos (`carpeta_amplia`, `no_existe`, `sin_permiso`, `en_red`, `demasiadas_carpetas`, `limite_inotify`) los define 3.5 |
| `last_error`, `last_error_at` | Último problema: límite de inotify, carpeta inaccesible, esquema. Va en español si el usuario puede corregirlo (`CLAUDE.md`, "Mensajes de error") |

La UI no depende de `tracker_status` para **calcular** tiempo, solo para
mostrar el estado (3.3).

**`projects.total_time_seconds`:** queda **deprecada**. La columna se mantiene
en el esquema para no romper bases viejas ni backups, pero el código deja de
leerla y de escribirla en dos commits (D2):

- **Lectura por unión:** `get_time_stats` y `get_project_stats.total_time_hours` (`db/activity.rs:63-69`) pasan a `tiempo_trabajado` (3.3). Nada se borra todavía.
- **Sacar el contador:** el segundo `UPDATE` de `end_tracking_session` (`db/tracking.rs:37-43`); `add_project_time` (`db/activity.rs:28-39`), su comando (`commands/mod.rs:501-508`), su registro (`main.rs:113`) y su función de API sin llamadores (`src/services/api.ts:161-166`); el campo en `PROJECT_COLUMNS` y en el mapeo (`db/common.rs:18,43`), en el modelo (`models/project.rs:30`) y en el tipo (`src/types/project.ts:33`); los tests que lo afirman (`db/tracking.rs:170-172,185,191-195`, `db/activity.rs:164-172`) y el fixture (`models/project.rs:250`); y `ARQUITECTURA.md:442`.
### 3.3 Cálculo del tiempo: unión de tramos

**Formato único de fechas** (`time.rs`): todo se guarda como
`'%Y-%m-%d %H:%M:%S'` en UTC, el mismo formato que produce `CURRENT_TIMESTAMP`.
Se prohíbe escribir RFC3339 en la base: los filtros por rango comparan texto, y
mezclar formatos los rompería sin aviso. Todo el código pasa por un único
formateador y un único parser.

**Excepción explícita: `projects.deleted_at`** (D5). `delete_project` lo escribe
en hora **local** (`db/trash.rs:21`), la papelera lo muestra tal cual y sirve
para identificar el lote borrado junto (`:17-18`). Queda **fuera** de `time.rs`
y así se documenta en su cabecera: nunca se compara con otra fecha ni entra en un
rango. El demonio solo pregunta `deleted_at IS NULL`. Pasarlo a UTC cambiaría lo
que se ve en la papelera y obligaría a reescribir filas, sin ganar nada.

**Qué tramo representa cada fila** (cuidando de no inventar tiempo):

| Fila | Tramo |
|---|---|
| Cerrada, `file_watch` | `[started_at, ended_at]` (igual a `started_at + duration_seconds`, 3.2). `started_at` ya incluye el margen de DX1 |
| Cerrada, otra fuente | `[started_at, started_at + duration_seconds]`. **No** se usa `ended_at`, porque la duración medida con `Instant` excluye la suspensión y respeta la regla de los 10 s (que deja 0) |
| Abierta, y es la `activa` (la `ActiveSession` de **esta** app) | `[started_at, started_at + activa.segundos]`, con los segundos del `Instant` en el momento de la consulta (M5). Es lo mismo que se va a guardar al cerrar, así que el número no baja después de una suspensión |
| Abierta, `file_watch` | `[started_at, COALESCE(last_activity_at, started_at)]`: hasta el último cambio real volcado |
| Abierta, otra fuente | `[started_at, started_at + COALESCE(duration_seconds, 0)]`: lo medido hasta el último latido (3.2), o 0 s si nunca latió |

Ninguna fila depende de `tracker_status` (M6). Un `file_watch` abierto cuenta
lo mismo esté vivo o muerto el demonio: hasta su último cambio, que es trabajo
real. El tramo de una fila abierta que no es la `activa` se queda quieto entre
latidos, y la próxima consulta lo alcanza.

**Función única:**

```rust
/// La sesión de "Trabajar" viva de esta app, con lo medido por su `Instant`.
pub struct SesionActiva {
    pub session_id: i64,
    pub segundos: i64,
}

/// Segundos trabajados en `project_id` dentro de [desde, hasta), contando cada
/// instante una sola vez aunque haya sesiones superpuestas.
pub fn tiempo_trabajado(
    &self,
    project_id: i64,
    rango: RangoUtc,
    activa: Option<SesionActiva>,
) -> Result<i64>
```

1. Consulta: `started_at < hasta AND (ended_at IS NULL OR ended_at > desde)`. En filas cerradas de otra fuente `ended_at ≥ started_at + duration_seconds`, así que el filtro trae de más y nunca de menos.
2. Arma el tramo de cada fila según la tabla de arriba y lo recorta a `[desde, hasta)`. `activa` solo se aplica a la fila con ese `id` que siga abierta.
3. Ordena, fusiona los superpuestos y suma.

Para `get_project_stats`, una variante hace **una** consulta para todos los
proyectos fuera de la papelera y fusiona por proyecto en Rust, en vez de una
consulta por proyecto.

**Quién pasa `activa`.** Los comandos que calculan tiempo (`get_time_stats`,
`get_project_stats`) reciben también `State<ActiveSession>`: toman su lock, copian
`session_id` y `started_at.elapsed()`, y lo **sueltan** antes de tocar la base. El
orden de locks es siempre `ActiveSession → Database`, nunca al revés:
`start_new_session_sync` ya sostiene el de la sesión mientras escribe en la base
(`commands/mod.rs:1559-1591`). El demonio llama con `activa = None`.

**Hora local:** "hoy" va de la medianoche local a la medianoche local, como rango
`[desde, hasta)` en UTC antes de consultar. Una sesión que cruza la medianoche
suma a cada día lo suyo.

- `rango_dia<Tz: TimeZone>(tz: &Tz, fecha: NaiveDate) -> RangoUtc` es genérica (D13). `chrono::Local` aparece solo en el borde, en los comandos.
- Medianoche **ambigua** (el reloj vuelve atrás y la medianoche pasa dos veces): `.earliest()`.
- Medianoche **inexistente** (el reloj salta hacia adelante justo a las 00:00): `.earliest()` devuelve `None`, así que no alcanza, y un `unwrap` haría panic. El día empieza en la primera hora local que existe: se avanza de a un minuto desde las 00:00 hasta encontrarla.
- **Semana** (D4): los últimos **7 días calendario locales, incluido hoy**: `[inicio de (hoy − 6), inicio de (hoy + 1))`. Hoy son 8 fechas (P3).

**Estadísticas (`get_time_stats`), definidas de nuevo:**

| Campo | Definición |
|---|---|
| `total_seconds`, `today_seconds`, `week_seconds` | `tiempo_trabajado` sobre su rango (unión), con la `activa` |
| `session_count` | Sesiones cerradas con duración > 0 |
| `avg_session_seconds`, `longest_session_seconds` | Sobre las sesiones **individuales** cerradas con duración > 0, sin unir: describen sesiones, no el total |

Un error de base **se propaga** (D6, P18): se reemplazan los `.unwrap_or(0)` de
`db/tracking.rs:101,119,130,141` y `db/activity.rs:61,68`. El mensaje es de
sistema y queda en inglés, según el criterio de `CLAUDE.md`.

`get_project_stats` también pasa a la unión y a la hora local:
`total_time_hours` es la suma de `tiempo_trabajado` de los proyectos fuera de la
papelera, y `active_today` cuenta los proyectos con actividad en
`project_activity` **o** con tiempo trabajado en el día local. Lo segundo es
nuevo: con el demonio, un proyecto editado con la app cerrada también estuvo
activo.

> **Por qué en Rust y no en SQL:** fusionar intervalos en SQLite exige consultas
> recursivas difíciles de leer y de testear. En Rust son unas 20 líneas con tests
> de tabla. Hoy hay 193 sesiones; aun con 50.000 el cálculo es instantáneo.

**Tests de zona horaria (D13):** nunca se cambia `TZ` del proceso; cada caso pasa
su zona a `rango_dia` con `chrono-tz`. Casos y precondiciones en 5.1.
### 3.4 El motor del demonio

**Reloj:** las marcas de tiempo son de **pared** (UTC del sistema), porque son las
que se guardan. No se usa `Instant` para la inactividad: en Linux es
`CLOCK_MONOTONIC`, no avanza durante una suspensión y ocultaría el corte. El
motor recibe el reloj inyectado, así que todo lo de esta sección se prueba con un
reloj falso (commit 31).

- **Reloj que retrocede** (corrección de NTP, cambio manual): se toma
  `max(t, ultimo_cambio)`, así que una duración nunca es negativa. Si el salto
  hacia atrás supera **5 min**, además se cierran las sesiones abiertas en su
  `ultimo_cambio` y se anota en `tracker_status.last_error`
  ("El reloj del sistema retrocedió 42 min; se cerraron las sesiones abiertas") (M16).
- **Zona horaria:** el demonio solo escribe UTC con el formateador de `time.rs`
  (3.3). No guarda ni cachea el desfase local: "hoy" se recalcula en cada
  consulta de la app (M16).
- **Suspensión detectada:** en cada `tick` el bucle compara cuánto avanzó
  `CLOCK_BOOTTIME` (cuenta la suspensión) contra `CLOCK_MONOTONIC` (no la cuenta).
  Si la diferencia creció más de 30 s, entra `reanudado(t)` al motor. Sirve para
  acotar el margen de DX1: nunca se acredita tiempo de antes del despertar.

**Estado por proyecto:**
`Inactivo { frontera }` o `Activo { sesion_id, inicio, ultimo_cambio, frontera }`.
`frontera` es el instante antes del cual no se puede acreditar nada: el `ended_at`
de la última sesión `file_watch` del proyecto, el último `reanudado` o el arranque
del proyecto en el demonio, el que sea mayor. Al arrancar se carga de la base
(`MAX(ended_at)` de `file_watch` por proyecto).

**Margen por guardado (DX1, confirmada):** cada evento que **abre**
una sesión acredita un margen hacia atrás de `min_credit_seconds` (default **60 s**),
porque guardar es el final de un rato de trabajo, no el principio. Esta es la
definición única; 3.2 y 9.2 la referencian:

```
margen = min(min_credit_seconds, idle_minutes × 60)     # rango configurable: 0 a 300 s
inicio = max(t − margen, frontera)
```

- Un guardado aislado vale 60 s, y dos guardados separados 2 min (con inactividad de 3) valen `[t1 − 60, t2]` = 3 min.
- El margen nunca se superpone con la sesión `file_watch` anterior del mismo proyecto ni cruza una suspensión: lo impide `frontera`, que al arrancar se carga después de recuperar huérfanas.
- Se acota por el corte de inactividad porque un margen más largo uniría huecos que el corte declara "no trabajo".
- Con `min_credit_seconds = 0` es el comportamiento de la v2: un guardado aislado da 0 s y su fila **se borra**.
- Reemplaza la Q3 vieja ("un guardado suelto no cuenta", 9.1). El principio "nunca inventar más que el margen" se mantiene, y el valor efectivo queda visible en `tracker_status.min_credit_seconds`.

| Entrada | Si está Inactivo | Si está Activo |
|---|---|---|
| `cambio(t)` | **Abrir** con `started_at = inicio` (fórmula de arriba) y volcar `last_activity_at = t` enseguida | Si `t − ultimo_cambio ≥ inactividad`: **Cerrar** en `ultimo_cambio`, `frontera = ultimo_cambio`, y **Abrir** en `t` con margen. Si no: `ultimo_cambio = t` (se vuelca según el latido, ver abajo) |
| `tick(ahora)` cada 15 s | — | Si `ahora − ultimo_cambio ≥ inactividad`: **Cerrar** en `ultimo_cambio` |
| `reanudado(t)` | `frontera = t` | **Cerrar** en `ultimo_cambio` (la pausa nunca queda adentro), `frontera = t` |
| Proyecto pasa a la papelera, se purga, cambia de carpeta o deja de ser medible | — | **Cerrar** en `ultimo_cambio` y dejar de vigilar |
| `UPDATE` de su sesión afecta **0 filas** (M12) | — | Soltar el proyecto como si se hubiera purgado; se reincorpora en la próxima relectura si sigue existiendo |
| `tracking.daemon_enabled = false` (B5) | — | **Cerrar** todas, salir con código 0 (ver "Kill switch") |
| SIGTERM / SIGINT | — | **Cerrar** todas en su `ultimo_cambio` |

La regla de `cambio` con hueco ≥ inactividad es la que corta bien en dos casos: un
evento que llega antes del próximo `tick`, y un evento que llega justo después de
una suspensión de horas. El tramo nunca cruza la pausa.

**Inactividad efectiva (M14):** `idle_minutes` de la config se **acota a 1–60**.
Un valor fuera de rango o no numérico usa el más cercano (o el default 3) y lo
anota en `last_error`. El valor efectivo se publica en `tracker_status.idle_minutes`
cada vez que cambia, también en una recarga en caliente (test).

**Cierres coherentes (M7):** todo cierre, venga del motor, de la recuperación de
huérfanas o de SIGTERM, pasa por `cerrar_sesion(id, fin, fin − started_at)` de
3.2: un solo `UPDATE` con `WHERE id = ? AND ended_at IS NULL` que deja
`ended_at = fin` y `duration_seconds = fin − started_at`. Los latidos usan
`latir(id, t, None)`; las dos devuelven `false` si no tocaron ninguna fila (M12). Nunca queda una fila con `ended_at` y
`duration_seconds` que digan cosas distintas. Si la duración da 0 (solo posible con
`min_credit_seconds = 0`), la fila se borra (3.2).

**Latido de sesión (M9):** el demonio escribe `last_activity_at`:
- al **abrir** (siempre, para que una huérfana nunca quede con `NULL`, M7);
- después, **como máximo una vez cada 5 s mientras hay actividad**: un evento
  marca la sesión como "sucia" y el temporizador de volcado (5 s) la escribe;
- **nada** mientras no hay actividad. Sin eventos no hay escrituras de sesión.

Con WAL y `synchronous=NORMAL` (3.6) un commit no hace `fsync` (solo el
checkpoint), así que una hora de trabajo continuo son ~720 commits baratos y una
hora sin tocar nada, cero. El contador de la app ve cada cambio con hasta ~10 s de
retraso: ≤ 5 s del volcado más ≤ 5 s del sondeo de la app (3.9).

**Latido del proceso:** `tracker_status.last_heartbeat` se escribe cada **15 s**
(3.2). La app considera vivo al demonio si `ahora − last_heartbeat < 45 s` **y**
`pid` no es `NULL`. Son 5.760 commits por día, pero con WAL y
`synchronous=NORMAL` no hacen `fsync` y tocan siempre la misma página, que el
checkpoint escribe una vez: el desgaste que preocupaba en M9 viene de las sesiones,
no de esta fila. Al salir limpio (SIGTERM, kill switch) el demonio pone
`pid = NULL`, así que "apagado" se ve en el próximo sondeo y no a los 45 s.

**Kill switch (B5):** si al releer la config `tracking.daemon_enabled` es `false`,
el demonio cierra todas las sesiones, pone `estado = 'desactivado'` y
`pid = NULL`, avisa `STOPPING=1` y **sale con código 0**. Se eligió salir y no quedar ocioso porque:
- con `Restart=on-failure` (3.7), una salida 0 **después de `READY=1`** no se reinicia;
- libera todas las vigilancias de inotify y la memoria;
- `systemctl --user status` muestra `inactive (dead)`, que es lo que el usuario
  espera ver después de apagarlo.

**Con `false` desde el arranque (A1), `READY=1` va antes de salir.** Con
`Type=notify`, si el proceso principal sale con 0 **antes** de mandar `READY=1`,
systemd no lo toma como éxito: la unidad termina con `Result=protocol` y queda
`failed` (verificado en `src/core/service.c` de systemd, `service_sigchld_event`:
"Only enter running through a notification", rama `SERVICE_FAILURE_PROTOCOL`).
`Restart=on-failure` reinicia ante cualquier resultado distinto de `success`, y
con `StartLimitIntervalSec=0` (3.7) eso sería un **bucle** de un arranque cada
5 s a 5 min. Por eso el demonio lee la config **antes** de abrir la base (paso 4
de abajo) y, si encuentra `false`, manda `READY=1`, después `STOPPING=1`, y recién
ahí sale con 0: la unidad queda `inactive (dead)` con `Result=success`. Lo mismo
si el `false` llega mientras espera una migración (paso 5). No hace falta
`RestartPreventExitStatus=`: la salida es 0 y llega después de `READY`. Lo prueba
E22 con el systemd real (5.2).

Así, si el unit sigue habilitado (por ejemplo, alguien puso `false` editando el
JSON con la app cerrada), en el próximo inicio de sesión arranca, lee `false`, avisa
`READY` y `STOPPING` y sale 0: no hay bucle de reinicios. El camino normal es
Ajustes, que además corre `disable --now` (3.8).

**Orden de arranque del demonio:**
1. **Rutas.** `data_dir` y `config_dir` llegan como argumentos (`--data-dir`,
   `--config-dir`) que escribe el unit (3.7). Sin argumentos se resuelven con el
   módulo de rutas compartido de la lib (D1), igual que la app.
2. **Guardas de desarrollo (B8).** En un build de debug, si no hay `--data-dir`
   ni `XDG_DATA_HOME` explícita, **se niega a arrancar** ("En debug pasá
   `--data-dir` o `XDG_DATA_HOME`: no toco la base real"). Si
   `GESTOR_TRACKERD_TEST_TIMINGS` está definida y `data_dir` resuelve a la base
   real del usuario (`$HOME/.local/share/gestor-proyectos`), también se niega, en
   cualquier build.
3. **Lock (B8).** `flock` exclusivo y no bloqueante en
   **`<data_dir>/trackerd.lock`**, junto a la base. Nunca en `$XDG_RUNTIME_DIR`:
   dos procesos con entornos distintos tomarían locks distintos y escribirían la
   misma base. Si está tomado, sale con código **75** (`EX_TEMPFAIL`) y un mensaje
   claro. La app toma este mismo lock durante el restore (3.6). Esta es la
   justificación única de la ubicación del lock; 3.1, 3.7, 3.8 y 8 la referencian.
4. **Config (3.5).** Se lee **antes** de la base, para respetar el kill switch
   aunque la base todavía no esté lista. Con `daemon_enabled = false`:
   `READY=1`, `STOPPING=1`, salida 0 (A1, arriba), sin tocar la base.
5. **Base, sin migrar (B1).** Abre la base **sin** `SQLITE_OPEN_CREATE` con
   `busy_timeout = 5000` y aplica la tabla de 3.2: sigue solo con
   `user_version = ESQUEMA_ACTUAL` y `journal_mode = wal`; si no, espera (atrasada
   o sin WAL) o queda ocioso (más nueva). Mientras espera, relee la config cada
   5 s y, si pasa a `false`, sale como en el paso 4. El demonio **nunca migra**.
6. **Huérfanas `file_watch`.** Las cierra en su `last_activity_at` con la regla de
   cierres coherentes; si es `NULL` (filas de antes de M7), en `started_at`, y se
   borra si queda en 0 s.
7. **Proyectos.** Lee la lista de proyectos y carga la `frontera` de cada uno.
8. **`READY=1`** a systemd. "Activo" significa "base abierta, huérfanas cerradas y
   bucle andando", **no** "vigilancia completa" (M13).
9. **Recorrido inicial en un hilo aparte** (3.5). Publica el progreso en
   `tracker_status.watched_dirs` y marca el final del recorrido (columna nueva, ver
   abajo). Los eventos de las carpetas ya vigiladas cuentan desde el primer momento.

Así ningún recorrido de 50.000 carpetas puede pasar `TimeoutStartSec` (M13).

**Watchdog atado al progreso (M13):** `WATCHDOG=1` lo manda **solo el hilo
principal**, al completar una vuelta del bucle (evento, temporizador o
`recv_timeout`). Un hilo aparte que mande `WATCHDOG=1` a ciegas mantendría vivo un
bucle colgado. Las altas de vigilancias se hacen en lotes de 1.000 entre vueltas,
para que una tanda grande no frene el bucle.

**Hilos:**

| Hilo | Hace | Nunca |
|---|---|---|
| Principal | Motor, escritor (rusqlite), temporizadores, registro de vigilancias, `sd_notify` | Recorrer disco |
| `notify` (interno del crate) | Lee inotify y manda eventos por un canal | Tocar la base |
| Recorridos | `WalkBuilder` inicial, de directorios nuevos y de rescate tras desborde (3.5). Devuelve listas de carpetas al principal | Agregar vigilancias ni tocar la base |
| Señales (`signal-hook`) | Convierte SIGTERM/SIGINT en un mensaje al principal | — |

**Errores de base no fatales:** si un `INSERT`/`UPDATE` de un proyecto falla por
clave foránea o restricción, **o afecta 0 filas** (M12: `changes() == 0`, que es lo
que pasa cuando la sesión o el proyecto se purgaron en el medio), el demonio suelta
ese proyecto y sigue. Un `SQLITE_BUSY` que supera el `busy_timeout` deja el volcado
pendiente en memoria y se reintenta en el próximo volcado. No se reinicia por
errores de datos, así que nunca agota los reinicios de systemd.

**`tracker_status`:** el motor escribe todas las columnas de la tabla de 3.2. Para
`demasiadas_carpetas`, la entrada de `unwatched_projects` suma la cantidad:
`{"id": 55, "motivo": "demasiadas_carpetas", "dirs": 27117}`.

**Qué evento es "trabajo":**

| Cuenta | No cuenta |
|---|---|
| `Create`, `Remove`, `Modify(Data)`, `Modify(Name)`, `Access(Close(Write))` | `Access(Open)`, `Access(Close(Read))`, `Modify(Metadata)`, eventos de rutas ignoradas (3.5), creación de una carpeta ignorada por nombre o por `.gitignore` |

**Qué carpeta es de qué proyecto (DX2, confirmada):**
- Las rutas se comparan con `Path::starts_with`, por componentes, contra la ruta
  **canonizada** una sola vez al vigilar. Así `/a/proj` no captura `/a/proj2`, y
  las barras finales que hoy tienen varios `local_path` (75, 22, 29, 13) no cambian
  nada.
- **Gana el más específico:** el `local_path` con **más componentes** que contiene
  al evento. Con un proyecto paraguas (por ejemplo `~/2025`) y uno adentro
  (`~/2025/telwinet/casum`), un guardado en `casum` cuenta **solo** para `casum`, y
  uno en `~/2025/otra-cosa` cuenta para el paraguas. En la base real esto ya pasa
  con 84 ⊃ 85 (`sensoresyactuadoresGRAL` y su `AP9335TH`).
- **Empate exacto** (misma carpeta canonizada, hoy 90 y 91 en `~/prueba-b14`): el
  cambio cuenta para **los dos**, porque es trabajo real en ambos (R1).
- La app muestra un aviso cuando un proyecto contiene a otro (3.9). La función de
  contención vive en la lib y la usan los dos procesos, así que el aviso no depende
  de que el demonio esté corriendo.
### 3.5 Vigilante y configuración

| Decisión | Detalle | Por qué |
|---|---|---|
| Crate | `notify` **8.2**, eventos crudos, **sin debouncer** | El corte por inactividad ya cumple esa función. `debouncer-mini` pierde el tipo de evento |
| Modo | Cada directorio con `RecursiveMode::NonRecursive` | El modo recursivo vigila `node_modules`, `target` y `.git` |
| Recorrido | `ignore` **0.4** `WalkBuilder` con `git_ignore(true)`, `git_exclude(true)`, `git_global(true)`, `require_git(false)`, `hidden(false)`, `follow_links(false)` y `.gestorignore` como archivo de ignorados propio. Corre en el hilo de recorridos (3.4) | Respeta `.gitignore` aunque la carpeta no sea un repo |
| **Registro de vigilancias con conteo de referencias (M2)** | `HashMap<PathBuf, {wd, proyectos}>` propio. Se llama a `watch()` solo cuando una carpeta pasa de 0 a 1 proyectos, y a `unwatch()` solo cuando pasa de 1 a 0 | inotify devuelve **el mismo** descriptor si se vigila dos veces la misma carpeta. Sin conteo, sacar un proyecto (o un paraguas) le saca la vigilancia a los demás que comparten carpetas |
| **Filtro por `.gitignore` en cada evento (M3)** | Además del recorrido, cada evento pasa por los matchers del proyecto: un `Gitignore` por cada `.gitignore` encontrado (anidados incluidos), `.git/info/exclude`, el global de git y `.gestorignore`, del más profundo al más alto. Se consulta `matched_path_or_any_parents` **solo** si la ruta está bajo la raíz del matcher, porque si no, entra en pánico | En carpetas vigiladas aparecen `*.log`, `.eslintcache`, `*.tsbuildinfo` que el `.gitignore` ignora y que antes contaban como trabajo |
| Rutas borradas | Para un `Remove` no se sabe si era carpeta. Si estaba en el registro, era carpeta vigilada (no ignorada); si no, se prueba el matcher con `is_dir` en `true` y en `false`, y alcanza con que uno ignore | Un `rm -rf dist2/` ignorado no cuenta |
| Directorio nuevo | Si el nombre está en "carpetas siempre ignoradas" o lo ignora el `.gitignore`, **no cuenta ni se vigila** (filtro sobre el evento del padre). Si no, se agrega su vigilancia, **después** se recorre el subárbol y lo que haya cuenta como actividad | inotify no ve lo creado antes de vigilarlo (`mkdir -p`, `git checkout`, descomprimir). Crear `target/` o `node_modules/` en la raíz no es trabajo |
| Directorio renombrado o movido | Se quita la vigilancia del subárbol viejo (con el conteo) y se recorre la ruta nueva. Si entra desde afuera, se vigila; si sale hacia afuera, se suelta | Las vigilancias quedan atadas a la ruta vieja |
| Directorio borrado | Sus entradas salen del registro (con el conteo) | Sin fugas |
| `.gitignore` o `.gestorignore` cambiado | Se reconstruyen los matchers de **ese** proyecto y se aplica la diferencia: se vigilan las carpetas que dejaron de estar ignoradas y se sueltan las que pasaron a estarlo | Test: agregar `logs/` al `.gitignore` y escribir en `logs/x.log` deja de contar sin reiniciar |
| Desborde (`Flag::Rescan`, M10) | No se acredita tiempo. Se agenda un **rescate diferido**: cuando pasan **30 s sin eventos**, el hilo de recorridos recorre los proyectos vigilados y devuelve solo las carpetas que **faltan** en el registro, que se agregan en lotes. Como máximo un rescate cada 10 min; si llega otro desborde en el medio, se reagenda | El desborde de la cola (`Q_OVERFLOW`) es global y pierde `Create` de carpetas: sin rescate, lo creado durante una compilación grande queda sin vigilar para siempre. Esperar calma evita re-recorrer en plena ráfaga y encadenar desbordes |
| Límite (`MaxFilesWatch`) | `last_error` + el proyecto queda en `unwatched_projects` con motivo `limite_inotify` y la app lo muestra | Nunca una caída silenciosa |
| **Tope por proyecto (DX2, confirmada)** | `max_dirs_per_project` (default **5.000**). Si el recorrido de un proyecto lo supera, se sueltan sus vigilancias (con el conteo) y queda en `unwatched_projects` con motivo `demasiadas_carpetas` y la cantidad | Un paraguas como `~/2025` no se come el límite de inotify compartido (O3). **Ojo:** en la base real, Moodle Potenciarg (55) tiene **~27.100** carpetas respetando `.gitignore`: con el default queda no medible. Opciones para el usuario: subir el tope, o ignorar el código de Moodle con `.gestorignore` |
| Lista de proyectos | Se relee cada 60 s, **o antes** si cambió `PRAGMA data_version` (se consulta cada 5 s, junto con el volcado). Se aplica **la diferencia**: altas, bajas, papelera, cambio de carpeta | No re-recorre lo que no cambió. `data_version` cambia solo cuando **otra** conexión hizo commit, así que la papelera se nota en ≤ 5 s y no en 60 (M12) |
| Papelera | Proyectos con `deleted_at` no se vigilan; su sesión abierta se cierra en su `ultimo_cambio` | Entre que se manda a la papelera y la relectura pueden acreditarse hasta 5 s más (60 s si `data_version` no estuviera disponible). Documentado (M12) |

**Carpetas siempre ignoradas** (además del `.gitignore`): `.git`, `node_modules`,
`target`, `dist`, `build`, `out`, `.next`, `.nuxt`, `.svelte-kit`, `.turbo`,
`.parcel-cache`, `.cache`, `coverage`, `.venv`, `venv`, `__pycache__`,
`.mypy_cache`, `.pytest_cache`, `.tox`, `.gradle`, `.idea`, `.direnv`,
`.terraform`, `graphify-out`, `.gestor`, y de sincronizadores (M4): `.stfolder`,
`.stversions` (Syncthing), `.dropbox.cache` (Dropbox).

**Archivos ignorados** (la carpeta sí se vigila):

| Origen | Patrones |
|---|---|
| Editores | `*.swp`, `*.swo`, `*.swx`, `*~`, `4913`, `.#*`, `#*#`, `*___jb_tmp___`, `*___jb_old___` |
| Sistema | `.DS_Store` |
| Sincronizadores (M4) | `.syncthing.*.tmp`, `~syncthing~*.tmp`, `.dropbox`, `.dropbox.attr`, `.sync_*.db*`, `._sync_*.db*`, `.owncloudsync.log`, `.nextcloudsync.log` |
| LibreOffice (M4) | `.~lock.*#`: se crea **al abrir** un documento, o sea que es una lectura |
| La propia app (M4) | `*_BACKUP.md`: lo genera "Backup del proyecto" en la raíz del proyecto (`commands/mod.rs:355-356`) |

**Carpetas sincronizadas (M4):** si la carpeta del proyecto (o un ancestro) tiene
`.stfolder`, o está bajo `~/Dropbox` o `~/Nextcloud`, el proyecto se marca como
**sincronizado**. Se mide igual, pero la app avisa que los archivos que llegan de
otro equipo cuentan como trabajo acá: inotify no dice qué proceso escribió.

**Carpetas rechazadas (R7)**, evaluadas sobre la ruta **canonizada** (un symlink a
`/` no pasa, M11):

| Motivo (`unwatched_projects`) | Regla |
|---|---|
| `carpeta_amplia` | `/`, `/home` o `$HOME`, o menos de 3 componentes. Ojo: también rechaza `/srv/x` y `/opt/x`, algo aceptable para este uso y documentado en el aviso |
| `no_existe` | La ruta no existe o no es directorio |
| `sin_permiso` | No se puede leer |
| `en_red` | Sistema de archivos de red o remoto: `nfs`, `nfs4`, `cifs`, `smb3`, `9p`, `virtiofs`, `ceph`, `glusterfs`, `fuse.sshfs`, `fuse.rclone`, `fuse.gvfsd-fuse`, `davfs` |
| `demasiadas_carpetas` | Supera `max_dirs_per_project` (DX2) |
| `limite_inotify` | El kernel no dio más vigilancias |

**Sistemas de archivos (M11):** el tipo se lee de `/proc/self/mountinfo` (el
`fstype` del punto de montaje más largo que contiene a la ruta), no solo de
`statfs`, porque `statfs` devuelve el mismo número mágico para todo FUSE. Se
rechaza lo remoto porque **un recorrido o un `stat` sobre un montaje de red caído
puede bloquear el hilo sin plazo**, no porque inotify no funcione: inotify ve los
cambios hechos desde esta máquina en cualquier sistema de archivos. Por eso
**FUSE no se rechaza en bloque**: en la base real, 14 proyectos viven en
`/mnt/miparticion`, que es `fuseblk` (NTFS por `ntfs-3g` sobre `/dev/nvme0n1p5`),
y se miden bien. Los FUSE no listados se vigilan con un aviso.

Hoy, en la base real (75 proyectos activos): `carpeta_amplia` afecta a Tienda (30),
Plataforma de Bilbao Bandoneon (31) y SoloSoluciones (43); `no_existe` a 13, 19,
20, 21, 29, 56 y 80 (19, 20 y 21 apuntan a `/home/2025`, que no existe).

**Configuración del demonio (P12):** lector **propio, de solo lectura**. Usa un
struct con únicamente la sección `tracking`, todo con `#[serde(default)]`, e
ignora el resto. No escribe, no crea y no renombra jamás el archivo. Si no puede
leerlo, usa los valores por defecto y lo anota en `last_error`.

Hace `stat` del archivo **cada 5 s** y lo vuelve a parsear solo si cambió su
`mtime` o su tamaño. Así el kill switch (B5) y un cambio de inactividad (M14)
llegan en ≤ 5 s, sin depender de `systemctl`. La config se guarda con temporal +
`rename` (`config/manager.rs:140-153`), así que nunca se lee a medio escribir.

```json
"tracking": {
  "daemon_enabled": true,
  "idle_minutes": 3,
  "min_credit_seconds": 60,
  "max_dirs_per_project": 5000
}
```

| Campo | Rango | Fuera de rango |
|---|---|---|
| `daemon_enabled` | `true`/`false` | Default `true` |
| `idle_minutes` | 1–60 | Se acota y se anota en `last_error` (M14) |
| `min_credit_seconds` | 0–300 | Se acota; 0 = comportamiento v2 (DX1) |
| `max_dirs_per_project` | 100–200.000 | Se acota (DX2) |

**Tiempos para tests:** la variable de entorno `GESTOR_TRACKERD_TEST_TIMINGS`
(por ejemplo `idle=5s,tick=500ms,flush=500ms,reload=1s,heartbeat=1s,config=200ms,rescan=1s`)
reemplaza los tiempos. Con la variable definida, el demonio se niega a usar la base
real del usuario (B8, 3.4).
### 3.6 Base compartida entre dos procesos

| Cambio | Detalle |
|---|---|
| **WAL** | Lo activa **solo la app**, en `Database::new`, con `pragma_update_and_check`, y verifica el resultado. Acepta `"wal"`, y también `"memory"` para las bases `:memory:` de los tests. Cambiar de modo necesita la base para sí sola: si después de `busy_timeout` sigue sin ser `wal`, `Database::new` **falla** con un error visible ("No se pudo preparar la base de datos: otro programa la tiene abierta. Cerralo y volvé a abrir la app.") en vez de seguir en `delete` en silencio (D8). Queda persistente en el archivo |
| El demonio y WAL | No cambia el modo. Si al abrir lee algo distinto de `wal`, espera igual que con un `user_version` atrasado (3.2) |
| `busy_timeout = 5000` | Se mantiene como **requisito** en las dos conexiones (`db/schema.rs:21`). Sin él, cualquier escritura del otro proceso es un `SQLITE_BUSY` inmediato |
| `synchronous = NORMAL` | En las dos conexiones. Es seguro con WAL |
| **Las 6 transacciones** pasan a `transaction_with_behavior(Immediate)` (P13) | El riesgo real son dos: `restore_project` y `purge_project` **leen antes de escribir**, y con WAL una transacción diferida así falla con `SQLITE_BUSY_SNAPSHOT` si el otro proceso escribe en el medio; `busy_timeout` no la salva. Las otras cuatro empiezan escribiendo y solo esperarían. Igual pasan todas, por una regla simple: **toda transacción que escribe es `IMMEDIATE`**, así nadie agrega después una lectura al principio de una diferida (D9). `empty_trash` mete adentro el `SELECT` de ids que hoy hace afuera (`db/trash.rs:205-211`), y `restore_project` y `purge_project` meten adentro la guarda `SELECT deleted_at` que hoy validan antes de abrir la transacción (`db/trash.rs:56-66` y `:133-143`): si no, otro proceso puede restaurar o purgar entre la guarda y la escritura |
| `end_tracking_session` idempotente (P2) | `WHERE id = ? AND ended_at IS NULL`. Si ese `UPDATE` no afecta ninguna fila, **no** se ejecuta el segundo, el que suma al contador (`db/tracking.rs:38-43`): se hace rollback y se devuelve `Ok(false)` (B3). Rige hasta que el contador desaparece (3.2) |
| `get_project` y `update_project` sin `try_lock` (P16) | Pasan a `self.conn()`, que espera y se recupera del envenenamiento como el resto (D7). La "cicatriz de deadlock" del comentario (`db/projects.rs:111-112`) no tiene sustento hoy: `get_project` pasa su guard a `get_project_links_internal` (`:142`), `update_project` lo suelta antes de llamar a `get_project` (`:244-249`) y `get_project_with_children` llama a `get_project` sin tener el lock (`db/groups.rs:81`). La espera queda acotada por lo que tarda la transacción del otro lado, que en el demonio son milisegundos; ningún método sostiene el Mutex más allá de su propia consulta |
| Checkpoint | El demonio corre `wal_checkpoint(TRUNCATE)` cada hora |
| Backups (`VACUUM INTO`) | Siguen igual. La copia se pasa a `journal_mode=DELETE` **antes** de verificarla, para que no aparezcan `-wal`/`-shm` en la carpeta de backups |
| **Permisos** (P19, D11) | `paths::data_dir()` crea la carpeta con `0700` y corrige una existente. `Database::new` deja `projects.db` en `0600` **antes** de activar WAL: SQLite crea `-wal` y `-shm` con los permisos del archivo de la base. Cada backup nuevo queda en `0600`. El unit lleva `UMask=0077` (3.7) |

**Restore (P6):** se reescribe con la **API de backup de SQLite** (feature
`backup` de rusqlite), que escribe **dentro** de la base viva en vez de reemplazar
el archivo. Secuencia:

1. **Antes de confirmar**, la UI avisa qué se pierde: todo lo registrado después de la fecha del backup, incluidas las sesiones del demonio y la de "Trabajar" en curso.
2. Verificar el backup elegido, como hoy (`backup/mod.rs:430-445`).
3. `systemctl --user stop gestor-trackerd`, recordando si estaba activo (3.8). El `stop` cierra las sesiones del demonio en la base vieja.
4. Tomar `trackerd.lock` (`paths::trackerd_lock_file()`, 3.4) con `flock` exclusivo y no bloqueante, y **retenerlo** hasta el paso 7. Si está tomado (un demonio lanzado a mano o fuera de systemd), se aborta: "El tracker sigue corriendo. Paralo con `systemctl --user stop gestor-trackerd` y volvé a intentar." Retenerlo impide además que un demonio arranque en el medio: si lo intenta, sale con 75 y no escribe.
5. Vaciar la `ActiveSession` **sin escribir**: tomar su lock y dejarla en `None`. Parar la sesión antes del restore no sirve, porque el cierre se escribe en la base que el restore pisa. Vaciarla evita que un cierre posterior (Parar, o cerrar la ventana en los 800 ms previos a la salida, `commands/mod.rs:101-104`) escriba en la base restaurada con un id que ahí no existe o es otra sesión.
6. Con el Mutex de `Database` tomado (orden `ActiveSession → Database`, 3.3):
   - Verificar el `page_size`: con la base viva en WAL, la API de backup falla si no coincide, así que se informa con un error claro antes de empezar.
   - `Backup::new(&origen, &mut conn)` y `step(-1)`: una sola pasada copia todas las páginas en **una** transacción de escritura de la base viva. Si falla, no queda a medias.
   - Verificar `journal_mode = wal` y volver a fijarlo si hiciera falta. La API de backup **conserva** el modo WAL del destino aunque el origen esté en `delete` (SQLite reescribe los bytes de versión de la cabecera al terminar, https://sqlite.org/backup.html), así que el re-fijado es **defensivo**: no debería cambiar nada, pero cuesta una línea y cubre un cambio futuro de SQLite.
   - Correr `migrar` (3.2) sobre lo restaurado, porque un backup viejo no tiene `last_activity_at` ni `tracker_status`, y `integrity_check`. La app migra **antes** de soltar el lock: el demonio nunca ve un `user_version` viejo.
7. Soltar el Mutex y el lock.
8. Si el demonio estaba activo y `daemon_enabled` sigue en `true`: `systemctl --user start gestor-trackerd` **antes** de salir (B7), así R6 no depende de reabrir la app. La base ya está migrada, así que arranca normal; si el paso 6 falló a mitad de la migración, el demonio espera (3.2) en vez de escribir sobre un esquema viejo.
9. Salir de la app con `app.exit(0)`, como hoy: el frontend tiene en memoria ids y datos que ya no existen.

**Dónde vive cada paso (A3).** El núcleo va en la lib, en `db/restore.rs` (3.1): `tomar_lock(lock_path) -> Result<LockRestore>` es el paso 4, y `restaurar_desde(&Database, origen, &LockRestore)` es el paso 6; pedir el guard del lock como argumento hace imposible llamarla sin él, y soltarlo (paso 7) es su `Drop`. `backup::restore_backup` (app) hace 1–3, 5, 8 y 9 alrededor. E15 llama a las dos funciones de la lib desde `tests/`.

La API de backup corre sobre la **conexión de la app**, bajo su Mutex, y no
sobre una conexión aparte: así no hay ningún comando de la app a mitad de camino
mientras se copia. Los que estén en cola corren después contra la base
restaurada, por eso la UI bloquea la interacción con el aviso de restauración
hasta la salida.

**Backups del home (D12).** Una herramienta que copia archivos (rsync, Déjà Dup,
Syncthing) y toma `projects.db` sin su `-wal` puede llevarse una base a la que le
faltan las últimas transacciones, o una inconsistente si copia en medio de un
checkpoint. Se documenta en `ARQUITECTURA.md` (commit 27) y en Ajustes → Backups (commit 52): excluir
`~/.local/share/gestor-proyectos/projects.db*` de esas herramientas e incluir, en
cambio, la carpeta de backups de la app, que son copias `VACUUM INTO` completas y
en modo `delete`. El checkpoint por hora achica la ventana, pero no la cierra.

**Volver a una versión anterior:** el piso de downgrade por WAL, `user_version` y
la pérdida de la sección `tracking` (D10) están en 7.2; el runbook, en 7.1.

El comentario de `db/schema.rs:11-20`, que rechazaba WAL, se reescribe con este
razonamiento.
### 3.7 Servicio de systemd

Lo escribe la app (3.8) en `<UNIT_DIR>/gestor-trackerd.service`. `UNIT_DIR` es
`$XDG_CONFIG_HOME/systemd/user` (por defecto `~/.config/systemd/user`) y llega como
parámetro del generador y del trait de systemctl, nunca fijo en el código (O5).
Las rutas se escriben **resueltas y absolutas** (no `%h`):

```ini
# Generado por Gestor de Proyectos. No editar: la app lo reescribe si difiere.
[Unit]
Description=Gestor de Proyectos: medición de tiempo por actividad en archivos
StartLimitIntervalSec=0

[Service]
Type=notify
ExecStart=/home/<usuario>/.local/bin/gestor-trackerd --data-dir /home/<usuario>/.local/share/gestor-proyectos --config-dir /home/<usuario>/.config/gestor-proyectos
Restart=on-failure
RestartSec=5
RestartSteps=6
RestartMaxDelaySec=300
TimeoutStartSec=30
TimeoutStopSec=15
WatchdogSec=60

# Recursos
UMask=0077
Nice=10
IOSchedulingClass=idle
MemoryHigh=192M
MemoryMax=384M
TasksMax=16
LogRateLimitIntervalSec=30s
LogRateLimitBurst=200

# Aislamiento (requiere user namespaces sin privilegios, ver abajo)
NoNewPrivileges=yes
PrivateUsers=yes
ProtectSystem=strict
ReadWritePaths=/home/<usuario>/.local/share/gestor-proyectos
PrivateTmp=yes
PrivateNetwork=yes
ProtectKernelTunables=yes
ProtectControlGroups=yes
RestrictAddressFamilies=AF_UNIX
RestrictNamespaces=yes
RestrictRealtime=yes
LockPersonality=yes
MemoryDenyWriteExecute=yes
SystemCallArchitectures=native

[Install]
WantedBy=default.target
```

**Ciclo de vida del proceso:**

| Decisión | Por qué |
|---|---|
| `ExecStart` = `<BIN_DIR>/gestor-trackerd`, y la app **solo** instala si su propio `current_exe()` está en `BIN_DIR` (3.8, B9) | En desarrollo `current_exe()` está en `src-tauri/target/`: un unit apuntando ahí ejecutaría un binario de debug que el próximo `cargo clean` borra. Con paquetes del sistema (`/usr/bin`, si el bundler llegara a incluir el segundo binario, B4) tampoco se instala: la app informa "instalación no soportada" en los dos casos |
| `--data-dir` y `--config-dir` explícitos (B8) | El administrador de systemd del usuario **no** hereda el entorno de la sesión gráfica: si la app corre con `XDG_DATA_HOME` y el administrador no, el demonio abriría **otra** base. Pasando las rutas que resolvió la app, los dos procesos usan la misma base y el mismo `trackerd.lock` |
| Rutas validadas (B9) | El generador **rechaza** rutas con caracteres fuera de `[A-Za-z0-9._/+@-]` (espacio, `%`, `\`, comillas, salto de línea): en un unit el espacio separa argumentos y `%` es un especificador. Un `$HOME` con esos caracteres no instala y lo dice en Ajustes. Más simple y verificable que escapar |
| `Type=notify` (`sd-notify` 0.5): `READY`, `WATCHDOG`, `STOPPING`, `EXTEND_TIMEOUT_USEC`, `STATUS` | `READY` llega con la base abierta, antes del recorrido (M13). `EXTEND_TIMEOUT_USEC` cubre la espera de `user_version` (3.4) |
| `WatchdogSec=60`, alimentado solo por el hilo principal | Si el bucle se cuelga, systemd lo reinicia (3.4) |
| `Restart=on-failure` | El kill switch sale con 0 **después de `READY=1`** y no se reinicia (B5, A1: una salida 0 antes de `READY` sería `Result=protocol` y sí se reiniciaría, 3.4). Un lock tomado sale con 75 y sí se reintenta |
| `StartLimitIntervalSec=0` + `RestartSteps`/`RestartMaxDelaySec` (M15) | Con `StartLimitBurst` el demonio quedaba en `failed` tras 5 fallos y, con la app cerrada, **días sin medir**. Ahora nunca se rinde: reintenta a los 5 s, luego con espera exponencial hasta un intento cada 5 min. `RestartSteps` es de systemd **254**; con uno más viejo (el generador recibe `systemctl --version` por el trait) escribe `StartLimitIntervalSec=300` + `StartLimitBurst=5` y la app hace `reset-failed` al abrirse (3.8). Acá corre systemd 261 |
| `TimeoutStartSec=30`, `TimeoutStopSec=15` | Arranque rápido por diseño (M13); SIGTERM solo cierra sesiones en memoria |
| `default.target`, **sin lingering** | Corre mientras haya sesión iniciada y arranca solo al iniciarla (R6) |
| Logs a stderr, que van a journald | `journalctl --user -u gestor-trackerd`. Reutiliza `logging.rs` |
| Nombre `gestor-trackerd` | Distinto del de la app; igual `pkill -f gestor-proyectos` lo alcanzaría por el `--data-dir …/gestor-proyectos` del `ExecStart`, por eso el commit 43 acota el patrón de `start-app.sh` (P10) |

**Logs (O2):**
- **Nunca** una ruta de archivo por evento. Ni en `debug`: con `log_level=debug`
  se loguean contadores, no rutas.
- Cada 10 min, una línea `info` agregada: eventos recibidos, contados, ignorados
  (por motivo), sesiones abiertas y vigilancias.
- Apertura y cierre de sesión en `info`, con `project_id` y duración.
- `warn`/`error` con límite de tasa propio (10 por minuto por tipo; el resto se
  cuenta y sale en la línea agregada). `LogRateLimit*` de journald es la red de
  seguridad.

**Recursos:**

| Directiva | Funciona en `--user` | Nota |
|---|---|---|
| `UMask=0077` | Sí | `-wal`, `-shm` y `trackerd.lock` nacen `0600` (D11) |
| `Nice=10` | Sí | Subir el nice no requiere privilegios |
| `IOSchedulingClass=idle` | Sí | Sin privilegios desde Linux 2.6.25. Solo tiene efecto con un planificador que respete prioridades (BFQ); acá el NVMe usa `none`, así que no cambia nada, pero tampoco molesta |
| `MemoryHigh`/`MemoryMax`, `TasksMax` | Sí, si el controlador está delegado | `user@.service` delega `pids memory cpu` (`/usr/lib/systemd/system/user@.service:30`); verificado en `cgroup.controllers` del usuario. `io` **no** está delegado: por eso no se usa `IOWeight=` |

`MemoryMax` deja margen sobre el objetivo de < 50 MB de RSS porque las marcas de
inotify (~1 KB por vigilancia) pueden contarse en el cgroup: con las ~52.400
carpetas medidas (2.3) son decenas de MB que no aparecen en `ps`. "Listo" mide
`memory.current` del cgroup además del RSS (O3).

**Aislamiento: qué funciona en `systemd --user` (O1).** Verificado contra
`systemd.exec(5)`:

- Las opciones que necesitan un espacio de nombres de montaje (`ProtectSystem=`,
  `ReadWritePaths=`, `PrivateTmp=`, `PrivateNetwork=`, `ProtectHome=`) **no
  funcionan solas** en un administrador de usuario: el kernel solo deja crear ese
  espacio a procesos privilegiados. Funcionan **con `PrivateUsers=yes`**, que crea
  antes un espacio de nombres de usuario. La página dice que en `--user` lo activa
  implícitamente, pero se escribe explícito para que el unit diga lo que hace.
- Requisito: user namespaces sin privilegios habilitados. Acá
  `kernel.unprivileged_userns_clone = 1` y `user.max_user_namespaces = 127762`.
  Si un sistema no los tiene, el arranque falla al armar los espacios de nombres
  (la página no documenta un modo degradado), antes de ejecutar el binario.
  **Criterio de detección (O1):** `systemctl --user show -p Result,ExecMainStatus
  gestor-trackerd` devuelve `Result=exit-code` y `ExecMainStatus=226`
  (`EXIT_NAMESPACE` de `systemd.exec(5)`, "Exit Codes"). Solo con esa
  combinación la app regenera el unit **sin** el bloque de aislamiento y lo avisa
  en Ajustes; cualquier otro fallo se muestra tal cual y no se toca el unit. No se
  baja nada sin avisar. Test con systemctl falso en el commit 44.
- `NoNewPrivileges`, `RestrictAddressFamilies`, `RestrictNamespaces`,
  `RestrictRealtime`, `LockPersonality`, `MemoryDenyWriteExecute` y
  `SystemCallArchitectures` usan seccomp o `prctl`, que no necesitan privilegios
  con `NoNewPrivileges`.

**Qué deja leer y escribir:**

| Necesidad | Con este unit | Por qué funciona |
|---|---|---|
| Leer las carpetas de los proyectos (recorrido + inotify) | Sí | `ProtectSystem=strict` deja **todo** en solo lectura (incluido `/home`), pero leer sigue permitido. inotify trabaja sobre inodos, no sobre montajes: ve las escrituras que hacen el editor o el compilador desde fuera del espacio de nombres |
| Proyectos en discos montados después (`/mnt`, `/run/media`) | Sí | La propagación de montajes del servicio es esclava: los montajes nuevos del sistema aparecen adentro |
| Escribir la base, `-wal`, `-shm` y `trackerd.lock` | Sí, **solo** en `ReadWritePaths` | Todo está en `<data_dir>`. La carpeta **tiene que existir** antes del arranque o falla al armar el espacio de nombres; la app la crea (`backup/mod.rs:398` ya hace `create_dir_all`) antes de habilitar el unit. `$XDG_RUNTIME_DIR` queda en solo lectura: otra razón para el lock en `<data_dir>` (3.4) |
| Temporales de SQLite | Sí | `PrivateTmp=yes` da un `/tmp` y `/var/tmp` propios y escribibles. Sin él, `ProtectSystem=strict` los deja en solo lectura y un `VACUUM` o un orden grande fallaría |
| Leer la config | Sí | Solo lectura alcanza (P12) |
| `sd_notify` | Sí | El socket de notificación es de ruta (no abstracto), así que `PrivateNetwork` no lo corta; el kernel no aplica solo lectura a `connect()` sobre un socket (solo a archivos, carpetas y enlaces). Lo confirma E16 |
| journald | Sí | stderr ya es un socket conectado al arrancar |
| Red | No | El demonio no la usa. `PrivateNetwork=yes` + `RestrictAddressFamilies=AF_UNIX` |

**No se usa `ProtectHome=`:** con `yes` o `tmpfs` el demonio no vería las carpetas
de los proyectos; con `read-only` no agrega nada a `ProtectSystem=strict`.

**Verificación:** el texto del unit tiene test unitario (commit 44). Que el
aislamiento realmente deja medir lo prueba **E16** (T2): `systemd-run --user` con
**las mismas propiedades** que genera el código salvo `WatchdogSec=5` (se arman desde la misma función),
`XDG_*` en carpetas temporales y un proyecto en `$HOME`: `READY`, un guardado
crea la sesión, `stop` la cierra. Paso manual: `systemd-analyze security --user
gestor-trackerd` y anotar el puntaje en el PR.
### 3.8 Ciclo de vida desde la app (R6)

Hoy `main.rs` no tiene `.setup()`: abre la base, lee la config y hace el
auto-backup en `main()` antes del plugin de instancia única (`main.rs:29-53`
contra `:60`, P11). Pasa a correr **después** de los plugins (D15), con este
orden, que sale de cómo arranca Tauri 2 (verificado en `crates/tauri/src/app.rs`,
Tauri 2.8.5 en `Cargo.lock`):

- `Builder::build()` inicializa los plugins (`initialize_plugins`), así que la
  segunda instancia ya salió por el plugin de instancia única al volver de `build()`.
- `App::run` crea las ventanas de `tauri.conf.json` (la `main`, con `create` por
  defecto en `true`) **antes** de llamar al closure de `.setup()`. Un `invoke` del
  frontend puede llegar antes de que el closure termine, y un `State<Database>`
  todavía no registrado falla.

Por eso el commit 4 abre la base, lee la config y hace `app.manage(db)`,
`app.manage(config)` y `app.manage(ActiveSession)` **de forma síncrona entre
`build()` y `run()`**, sin nada lento en el medio. El trabajo lento (auto-backup,
recuperación de huérfanas, ciclo del servicio) va en un hilo lanzado desde
`.setup()`, sin demorar la ventana. Alternativa si `manage` entre `build()` y
`run()` diera problemas: `"create": false` en la ventana y crearla al final del
`.setup()`. Todo `systemctl` pasa por un
trait (`ServiceManager`) que recibe `UNIT_DIR` y `BIN_DIR` como parámetros (O5),
así los tests usan un systemctl falso y carpetas temporales.

**Al abrir la app:**

1. **Condiciones.** Solo si es Linux, el build es de **producción**
   (`!cfg!(debug_assertions)`) y no hay `GESTOR_NO_TRACKERD=1`.
2. **Kill switch (B5).** Si `tracking.daemon_enabled = false`: si el unit existe y
   está habilitado o activo, `systemctl --user disable --now gestor-trackerd`.
   Termina. El indicador muestra "Tracker apagado (desactivado en Ajustes)".
3. **Ubicación (B9).** Se toma `std::env::current_exe()`:
   - si termina en `" (deleted)"` o no existe (el binario se reemplazó con `mv`
     mientras la app corría), **no se toca nada** en esta apertura;
   - si su carpeta canonizada **no es** `BIN_DIR` (por defecto `~/.local/bin`),
     no se instala nada: "El tracker solo se instala con `install.sh` o
     `release.sh --install`". Cubre desarrollo (`src-tauri/target/`) y paquetes
     del sistema, haya o no un segundo binario en el paquete (B4);
   - si falta `BIN_DIR/gestor-trackerd` o no es ejecutable: "Tracker no instalado".
4. **Rutas.** Se validan `BIN_DIR`, `data_dir` y `config_dir` con la regla de 3.7
   (sin espacios, `%` ni saltos de línea) y se asegura que `data_dir` exista.
5. **Unit por contenido completo (B9).** Se genera el texto y se compara **byte a
   byte** con el archivo existente (no una versión en la primera línea). Si falta
   o difiere: temporal + `rename` en `UNIT_DIR`, `daemon-reload`, y se anota
   "unit cambiado". Después se verifica que
   `systemctl --user show -p FragmentPath gestor-trackerd` sea **ese** archivo;
   si no (otro unit con el mismo nombre tiene prioridad, o el administrador mira
   otro `XDG_CONFIG_HOME`), se muestra en Ajustes y no se sigue (O5).
6. **Habilitar sin pisar al usuario (B5).** Según `is-enabled`:

   | Estado | Acción |
   |---|---|
   | El unit **no existía** antes del paso 5 (primera instalación, o después de `uninstall.sh`) | `enable --now`, solo si `daemon_enabled = true` (con `false` ya terminó en el paso 2) |
   | `enabled` | `reset-failed` si está en `failed`; `try-restart` si el unit cambió; si no, `start` (no hace nada si ya corre) |
   | `disabled` con el unit ya presente y la config en `true` | **No se reactiva.** Lo deshabilitó el usuario con `systemctl --user disable`. Ajustes: "Desactivado desde systemctl. Activalo desde acá si querés medir" |
   | `masked`, `linked` u otro | No se toca; se muestra el estado |

   Un `systemctl --user stop` manual (sin `disable`) **sí** se revierte al abrir
   la app, porque el unit sigue habilitado: el usuario lo detuvo, no lo apagó.
7. **Versión.** Si `tracker_status.version` ≠ versión de la app y el demonio está
   activo: `try-restart`, **una vez** por apertura. Si a los 10 s sigue distinta,
   aviso "El binario del tracker no coincide con la app: reinstalá" (y no se
   reintenta en bucle).
8. **Aislamiento no disponible.** Si el unit está en `failed` con
   `Result=exit-code` y `ExecMainStatus=226` (criterio de 3.7, O1), se regenera
   sin el bloque de aislamiento y se avisa.
9. Cualquier fallo de `systemctl` (sin bus de usuario, sin systemd) se muestra en
   Ajustes, no en un panic.

La recuperación de huérfanas `work_button` también corre en `.setup()`, con las
reglas de cierre de 3.3 (M7). Solo cierra filas cuyo
`COALESCE(last_activity_at, started_at)` tenga **más de 2 minutos** (2 latidos),
así nunca toca la sesión viva de otra instancia.

Cerrar la app **no** toca el demonio.

**Desde Ajustes → Tracking (B5).** Siempre **primero la config, después
systemctl**. Si `systemctl` falla, la config ya cambió y el demonio la ve en ≤ 5 s
(3.5), así que el apagado nunca depende de que systemd responda:

| Acción | Secuencia |
|---|---|
| Apagar | `daemon_enabled = false` → `disable --now`. El demonio, si sigue vivo, cierra sus sesiones y sale 0 |
| Encender | `daemon_enabled = true` → pasos 3–6 → `reset-failed` → `enable --now`. Es intención explícita, así que **sí** revierte un `disable` manual |
| Reiniciar | `restart` |

El toggle y el indicador mínimos se entregan en **F6**, junto con la instalación:
el demonio nunca corre en producción sin una forma de apagarlo desde la app.

**Restore con el demonio (B7).** La secuencia completa está en 3.6 (pasos 1–9):
`stop` recordando si estaba activo, lock retenido, migración **antes** de soltar el
lock, y `start` antes de `app.exit(0)` si estaba activo y sigue habilitado en la
config. Si la app muere entre los pasos 3 y 8, el lock se libera con el proceso y el
demonio vuelve en el próximo inicio de sesión o al abrir la app (paso 6 de arriba),
porque sigue habilitado.

**Desarrollo:** `tauri dev` nunca instala el servicio (paso 1 y 3). El demonio se
corre a mano con
`cargo run --bin gestor-trackerd -- --data-dir /tmp/trackerd-dev --config-dir /tmp/trackerd-dev`;
sin `--data-dir` ni `XDG_DATA_HOME` explícita, en debug se niega a arrancar (B8).
El `flock` en `<data_dir>` evita que convivan dos sobre la misma base.

**Instalación y actualización de binarios** (`release.sh --install`, `install.sh`, O5):

- Variables con default: `PREFIX=${PREFIX:-$HOME/.local}`, `BIN_DIR=$PREFIX/bin`,
  `UNIT_DIR=${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user`,
  `SYSTEMCTL=${SYSTEMCTL:-systemctl}`. Hoy las rutas están fijas
  (`scripts/release.sh:303-304`, `install.sh:21-28`) y el `.desktop` tiene
  `Exec=/home/manjarodesktop/.local/bin/gestor-proyectos` literal (`install.sh:46`):
  pasa a `$BIN_DIR`. Ojo: ese `.desktop` se escribe con un heredoc **con comillas**
  (`<< 'EOF'`, `install.sh:40`), que no expande variables; el commit 48 lo pasa a
  `<< EOF` para que `$BIN_DIR` se expanda. Los tests corren con `PREFIX=$tmp` y `SYSTEMCTL=true`.
1. Los **dos** binarios se copian a un temporal **en `BIN_DIR`** y se hace `mv`
   sobre el destino. Hoy es `cp` (`release.sh:304`, `install.sh:27`): sobre un
   binario en ejecución falla con `ETXTBSY` y con `set -e` corta el script a la mitad.
2. Si el unit existe: `$SYSTEMCTL --user try-restart gestor-trackerd`.

Los scripts **no escriben el unit**: hay un solo generador (la app), y el próximo
arranque de la app lo actualiza por contenido (paso 5).

`uninstall.sh` (O4): `disable --now`, borra el unit, `daemon-reload`,
`reset-failed`, y borra los dos binarios. La base se conserva, como hoy
(`uninstall.sh:37-41`), y con ella la fila de `tracker_status`. No hace falta
borrarla: el estado del indicador sale del unit, de `pid` y del latido (3.9), y el
demonio la pisa entera al arrancar.
### 3.9 La app: lo que ve el usuario

**El tiempo deja de depender de `.gestor/` (B2).** Hoy `TimeTracker.tsx`
(`loadTrackingData`, `:272-289`) llama a `checkTrackingConfig` y **solo** pide
`getTimeStats` si existe `<proyecto>/.gestor/config.json` (`:277-283`, que usa
`has_tracking_config` de `tracking/config.rs:122`). Si no, las dos vistas muestran
el botón "Track" / "El tracking de tiempo no está activado" (`:41-65` en `CompactView`, `:130-150` en `FullView`).
El demonio no crea `.gestor/` (y la ignora, 3.5), así que todo lo que mide sería
**invisible**. Cambio: `TimeTracker` carga `getTimeStats` **siempre** y desaparece
la rama "no activado"; `checkTrackingConfig` e `initTracking` quedan sin llamadores
desde esta vista (su retiro, si corresponde, va aparte). Test Vitest: sin
`.gestor/`, con sesiones en la base, la tarjeta muestra el total.

| Pieza | Comportamiento |
|---|---|
| `get_live_tracking` | Una consulta: sesiones abiertas (las dos fuentes) con `started_at`, `last_activity_at` y `source`, más `tracker_status`. Para la `ActiveSession` de **esta** app agrega `elapsed_seconds` medido con su `Instant` (lo que ya calcula `work_session_status_sync`, `commands/mod.rs:1666`) |
| Contador en la tarjeta, **`work_button`** (M5) | Muestra `elapsed_seconds` del backend, **no** `ahora − started_at`: el `Instant` no avanza en una suspensión, igual que lo que se guarda al parar (`commands/mod.rs:1566`, `:1616`). Entre sondeos suma de a 1 s con un `setInterval` que se **resincroniza** en cada respuesta y nunca adelanta más de 5 s. Con `Date.now()` el contador incluiría la suspensión y después **bajaría** al guardarse |
| Contador en la tarjeta, **`file_watch`** (M9) | Muestra `last_activity_at − started_at`. Avanza **a saltos, con cada cambio guardado**, con un retraso de hasta ~10 s (≤ 5 s del volcado del demonio, 3.4, más ≤ 5 s del sondeo de la app), y se queda quieto mientras no se toca nada. Nunca pasa del último cambio real (R3). Incluye el margen de DX1 (`started_at` ya viene corrido hacia atrás) |
| Texto "en vivo" (M9) | El punto verde y el tooltip dicen lo que pasa de verdad: **"Midiendo por cambios en archivos · se actualiza con cada guardado (con unos segundos de retraso, hasta 10 s)"**. Para `work_button`: "Midiendo con Trabajar". No se promete un segundero continuo para `file_watch` |
| Frecuencia | Cada 5 s, solo con la ventana visible (`visibilitychange`) |
| Badge de total | Se refresca cuando se cierra una sesión (P5) |
| Indicador global | En la cabecera. Estados abajo |
| Ajustes → Tracking | Activar/desactivar (3.8), minutos de inactividad (1–60, default 3) con el valor **efectivo** de `tracker_status.idle_minutes`, margen por guardado (DX1), tope de carpetas por proyecto (DX2), estado, proyectos vigilados y no vigilados con su motivo, y uso de inotify (abajo) |
| Ajustes → Backups | Nota de D12 (3.6): "Si respaldás el home con rsync, Déjà Dup o Syncthing, excluí `projects.db*` e incluí la carpeta de backups de la app." |
| Avisos por proyecto | Tabla de abajo |
| "Trabajar" | Su latido de 60 s escribe `last_activity_at` **y** `duration_seconds` medido con `Instant` (3.2), así que el tramo vivo en la base es `started_at + elapsed` y coincide con el contador. Muestra los errores en español (P7) y suma el botón **Parar** (P9) |
| "Abrir" del Dashboard | Usa el mismo flujo que "Trabajar" (P8) |

**Indicador global:**

| Estado | Condición | Texto |
|---|---|---|
| Activo | `estado = 'activo'`, `pid` no `NULL`, latido < 45 s, `scan_completed_at` presente | "Tracker activo" |
| Preparando | Igual, sin `scan_completed_at` | "Tracker preparando la vigilancia (N carpetas)" |
| Apagado | `daemon_enabled = false`, `estado = 'desactivado'`, `pid = NULL`, o unit `disabled`/`inactive` | "Tracker apagado" + motivo: "desactivado en Ajustes", "desactivado desde systemctl", "detenido" |
| Con problemas | `last_error` reciente (`last_error_at`), `estado` `esperando_migracion` o `esquema_nuevo`, unit en `failed`, o latido vencido con `pid` no `NULL` (se cayó) | "Tracker con problemas" + `last_error` |
| No instalado | Falta el unit o el binario en `BIN_DIR` | "Tracker no instalado: corré `install.sh`" |
| No disponible | Build de desarrollo, app fuera de `BIN_DIR`, sin systemd | "Tracker no disponible en esta instalación" |

**Avisos por proyecto.** Los de carpeta (R7, contención, red, sincronizada) los
calcula la **app** con las mismas funciones de la lib que usa el demonio (3.4,
3.5), así que aparecen aunque el demonio esté apagado. Los de recorrido salen de
`tracker_status.unwatched_projects`:

| Motivo | Texto |
|---|---|
| `carpeta_amplia` | "⚠️ Carpeta no válida para medir tiempo. Editá el proyecto y elegí su carpeta real." |
| `no_existe` / `sin_permiso` | "⚠️ La carpeta del proyecto no existe o no se puede leer. Editá el proyecto y corregí la ruta." |
| `en_red` | "⚠️ La carpeta está en un disco de red: no se mide para no colgar el tracker. Si trabajás ahí, usá Trabajar." |
| `demasiadas_carpetas` (DX2) | "⚠️ No se mide: tiene N carpetas y el tope es M. Subí el tope en Ajustes o ignorá carpetas con `.gestorignore`." |
| `limite_inotify` | "⚠️ El sistema no dio más vigilancias de archivos. Mirá Ajustes → Tracking." |
| Contiene a otro (DX2) | "ℹ️ Esta carpeta contiene a «X». Los cambios dentro de «X» cuentan solo para «X»." (y el inverso en el hijo) |
| Misma carpeta que otro | "ℹ️ «X» usa la misma carpeta: los cambios cuentan para los dos." |
| Sincronizada (M4) | "ℹ️ Carpeta sincronizada: los archivos que llegan de otros equipos también cuentan como trabajo." |

**Uso de inotify (O3)** en Ajustes: "El tracker usa N vigilancias de un límite de
M (`/proc/sys/fs/inotify/max_user_watches`, hoy 524.288). El límite es compartido
con VSCode, JetBrains y Syncthing." N sale de `watched_dirs`. No se intenta contar
lo que usan los otros programas: recorrer `/proc/*/fdinfo` es caro y no hace falta
para decidir.
## 4. Fases y commits

Cada commit compila y pasa `scripts/check.sh` (5.5), que se crea en el commit 2.
Se usa la convención del repo: `checkpoint:` en español y `git add` de archivos
concretos, nunca `git add .`. Hay una rama por fase (`feat/tracker-f0`, …,
`feat/tracker-f5a`, `feat/tracker-f5b`, …) que se mergea a `main` con fast-forward
cuando pasa su validación, igual que el B14.

**Total: 56 commits `checkpoint:`** en 10 fases, más el commit `release:` que genera
`scripts/release.sh` al final de F8. La columna "Hallazgos" remite al Anexo A.

**Regla de orden:** ningún commit usa algo que crea un commit posterior. Las
dependencias que importan están en la columna "Depende de"; la verificación
completa está en 4.10.

**Validación al cerrar cada fase** (antes del merge):
1. `scripts/check.sh` en verde.
2. Desde F5b: `scripts/check.sh --e2e` (E2E tres veces seguidas, 5.2).
3. El paso manual de la fase, si lo tiene (5.3).
4. F1 y F2, además: el diff con datos reales (commit 19) sobre una copia fresca de la base real, y la app abierta contra esa copia (5.4).

### F0 — Puerta de calidad y arreglos heredados

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 1 | `cargo fmt` de todo el crate, sin otro cambio. Hoy `cargo fmt --check` reporta **127** bloques de diferencia (medido sobre `main@c8b55ae`) | `cargo test` sin cambios; `git diff --stat` solo de formato | B6a | — |
| 2 | `scripts/check.sh` (5.5). Si `cargo clippy --all-targets -- -D warnings` no pasa sobre `main`, los avisos se corrigen en este mismo commit, sin cambios de comportamiento | Correr el script en verde; romper el formato de un archivo a propósito → el script falla | B6a | 1 |
| 3 | `end_tracking_session` idempotente: `WHERE id = ? AND ended_at IS NULL`; si afecta 0 filas, rollback y **no** corre el `UPDATE` del contador (`db/tracking.rs:38-43`); devuelve `Ok(false)` | Cerrar dos veces suma **una** vez (contador y `duration_seconds` intactos tras el 2º); se mantiene `test_end_tracking_session_with_unknown_id_credits_no_project` | P2, B3 | — |
| 4 | Base, config, nivel de log y auto-backup pasan a correr **después** del plugin de instancia única (`main.rs:29-53` contra `:60`), con el orden de 3.8: `Builder::build()` → `Database::new` + `app.manage(db)`, `config`, `ActiveSession` **síncronos** → `app.run()`; el auto-backup y todo trabajo lento van en un hilo lanzado desde `.setup()`. Un error de `Database::new` sale como `Err` con mensaje | Paso manual 1 (5.3): lanzar la app dos veces seguidas → una ventana y **un** backup nuevo; un `invoke` inmediato desde la ventana (por ejemplo `get_all_projects` en el primer render) no falla por estado no registrado | P11, D15 | — |
| 5 | `get_project`/`update_project` sin `try_lock` (`db/projects.rs:113,154`): pasan a `self.conn()`; se reescribe el comentario de `:111-112` | Un hilo sostiene el lock 200 ms: `get_project` espera y devuelve `Ok`; con el Mutex envenenado, funciona | P16, D7 | — |
| 6 | "Trabajar" muestra el error en español (`ProjectList.tsx:209-213`) | Vitest: toast si `startWorkSession` falla | P7 | — |
| 7 | Botón **Parar** (`api.ts:507` gana llamador) | Vitest + test del comando | P9 | — |
| 8 | Regla única de duración mínima: `duracion_a_guardar(source, segundos)` para cambio de proyecto, Parar y cierre de ventana (`commands/mod.rs:1569-1579`, `:1615-1621`); el `let _` de `:1578` propaga | Parar a los 5 s → 0 s con `ended_at` real; cambio de proyecto a los 5 s → 0 s; 11 s → 11 s por los tres caminos; un error de base en el cierre de 0 s sube | M8 | 7 |
| 9 | "Abrir" del Dashboard usa el flujo de "Trabajar" (`Dashboard.tsx:55-65`) | Vitest | P8 | 6 |

### F1 — Esquema versionado, recuperación y unión

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 10 | Migraciones versionadas: `migrar()` con `user_version` y `BEGIN IMMEDIATE`; paso 1 = esquema actual; solo se ignora `duplicate column name`; `user_version` futuro → error en español. `Database::abrir_existente()` para el demonio (sin `SQLITE_OPEN_CREATE`, devuelve `EstadoEsquema`) | (a) esquema viejo real (`schema.rs:294`) termina en `ESQUEMA_ACTUAL`; (b) segunda corrida no hace nada; (c) otra conexión con `BEGIN IMMEDIATE` y `busy_timeout` corto → `Err` y nada cambia; (d) `user_version` futuro → `Err`, base intacta; (e) un error que no es "duplicate column" se propaga; (f) `abrir_existente` sobre ruta inexistente no crea el archivo | P14, B1 | — |
| 11 | `time.rs`: formato y parser únicos, `rango_dia<Tz: TimeZone>` (medianoche ambigua e inexistente), `rango_semana` = 7 fechas locales, `chrono-tz` en `[dev-dependencies]`; excepción documentada de `deleted_at` | Ida y vuelta con `CURRENT_TIMESTAMP`; tabla de zonas de 5.1 (Buenos Aires, Santiago, Havana, Madrid), cada una afirmando su precondición; semana = 7 fechas | P3, D4, D5, D13 | — |
| 12 | `last_activity_at` + índice `(project_id, ended_at)` como paso 2 | De `user_version` 1 a 2; esquema viejo real → 2 con columna e índice | P4 | 10 |
| 13 | Tabla `tracker_status` como paso 3, con todas las columnas de 3.2 (incluida `scan_completed_at`) y `Database::leer_tracker_status() -> Option<…>` | Existe tras migrar; sin fila → `None`; columnas esperadas | B6b | 12 |
| 14 | Sesiones con instante explícito: `abrir_sesion`, `latir`, `cerrar_sesion` con `Result<bool>`; `create_/end_tracking_session` quedan como envoltorios | Abrir escribe `last_activity_at = started_at`; cerrar dos veces → `false`; `latir` sobre cerrada o id inexistente → `false` | D14, M7 | 12 |
| 15 | Latido de "Trabajar" cada 60 s (`last_activity_at` **y** `duration_seconds` del `Instant`) + recuperación de huérfanas no `file_watch` en `.setup()` (> 2 min, reglas de 3.2) | Huérfana con latido y suspensión → cierra con la duración, no con la diferencia; sin latido → 0 s; sesión de otra instancia con latido reciente → intacta; las 81 huérfanas reales no cambian el total | P4, M7 | 4, 8, 14 |
| 16 | `tiempo_trabajado(project_id, rango, activa)`: tramos de 3.3, unión y recorte; variante de una consulta para todos los proyectos | Disjuntos, superpuestos, anidados, que se tocan; abiertos de cada tipo; cruce de medianoche; regla de 10 s; `activa` con suspensión cuenta los segundos del `Instant`; `file_watch` abierto hasta `last_activity_at`; `ended_at` muy posterior no amplía el tramo; `activa` con id cerrado se ignora | R4, M5, M6 | 15 |
| 17 | Lectura por unión (9a de la auditoría): `get_time_stats` y `get_project_stats` sobre la unión en hora local, `active_today` nuevo, errores propagados, `State<ActiveSession>` con orden `ActiveSession → Database`. **No** saca el contador | Sin superposición, mismos números salvo las correcciones documentadas; con superposición, sin doble conteo; semana = 7 fechas; `active_today` en hora local (a las 22:00 de Buenos Aires sigue siendo "hoy"); tabla borrada → `Err`, no 0; `start_work_session` y `get_time_stats` en paralelo 1.000 veces sin deadlock | P3, P18, D2, D4, D6 | 11, 16 |
| 18 | `TimeTracker` carga `getTimeStats` **siempre**; desaparecen la rama "no activado" y el botón "Track" | Vitest: sin `.gestor/` y con sesiones, muestra el total; `checkTrackingConfig` no se llama | P15, B2 | 17 |
| 19 | Diff con datos reales: test unitario `#[ignore]` en `db/golden_diff.rs` (`#[cfg(test)]`: la lib todavía no existe, así que no puede ir en `tests/`) que lee `GESTOR_GOLDEN_DB`, copia con la API de backup (activa la feature `backup` de rusqlite, que el commit 22 reutiliza) a un `tempdir` desde una conexión de solo lectura, y emite por proyecto los números viejos (SQL actual copiado en el test) contra los nuevos. Lista aparte las sesiones > 12 h, las huérfanas y los proyectos con carpeta inexistente, **sin tocar nada** | Corre local, nunca en CI. Salida esperada hoy: proyecto 3 (contador 30.348 s contra sesiones 38.448 s), semana 8 → 7 fechas, "hoy" UTC → local, 9 sesiones > 12 h (936.392 s), 5 sesiones de 1–10 s | D3 | 17 |
| 20 | Sacar el contador y `add_project_time` (9b de la auditoría): la lista completa de 3.2. La columna queda en el esquema | `check.sh` en verde; `rg total_time_seconds` solo encuentra `db/schema.rs` y tests de migración; el diff del commit 19 sigue igual | P1, D2 | 17, 19 |

**Validación de F1:** diff del commit 19 sobre una copia fresca de la base real;
cada diferencia queda anotada para el CHANGELOG (commit 56). Paso manual 2 (5.3).

### F2 — Rutas, restore, transacciones, permisos y WAL

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 21 | `paths.rs`: `data_dir`, `db_file`, `trackerd_lock_file`, `config_file`; se borra `backup::live_db_path` | Con `XDG_DATA_HOME`/`XDG_CONFIG_HOME` temporales: `db_file()` igual a la vieja `live_db_path`; `config_file()` no crea nada | P17, D1 | — |
| 22 | Restore con la API de backup: pasos 1, 2, 4–7 y 9 de 3.6 (aviso, `flock` retenido sobre `trackerd.lock`, `ActiveSession` vaciada sin escribir, Mutex, `page_size`, `step(-1)`, `journal_mode`, `migrar`, `integrity_check`). El núcleo va en `db/restore.rs` (`tomar_lock`, `restaurar_desde`, A3), que entra en la lib con el commit 28; `backup::restore_backup` queda como envoltorio. Los pasos de systemd quedan para el commit 47 | Restore con `-wal` presente no corrompe; `integrity_check = ok`; con el `flock` tomado por otro proceso, aborta sin tocar nada; backup de esquema viejo → `ESQUEMA_ACTUAL`; después del restore, Parar no escribe | P6, B7 | 10, 19, 21 |
| 23 | Las 6 transacciones a `IMMEDIATE`; `empty_trash` mete el `SELECT` de ids adentro, y `restore_project`/`purge_project` meten adentro su guarda `SELECT deleted_at` (`db/trash.rs:56-66`, `:133-143`); comentario corregido | Los existentes; dos conexiones: una escribe entre la lectura y la escritura de `restore_project` → sin `SQLITE_BUSY_SNAPSHOT`; otra conexión restaura el proyecto entre la guarda y la escritura de `purge_project` → `purge` no borra un proyecto activo | P13, D9 | — |
| 24 | Permisos: `data_dir` `0700` (también una existente), `projects.db` `0600`, backups `0600` | `#[cfg(unix)]`: modos de carpeta, base y backup | P19, D11 | 21 |
| 25 | WAL + `synchronous=NORMAL`, solo desde la app; si no queda `wal`, error visible; el restore lo vuelve a fijar | DELETE → `wal`; `:memory:` → `memory`; lectura abierta en otra conexión → `Err`, no `delete` en silencio; 1.000 escrituras desde dos conexiones sin `SQLITE_BUSY`; restore deja `wal`; `-wal`/`-shm` en `0600` | D8 | 22, 24 |
| 26 | Copia de `VACUUM INTO` a `journal_mode=DELETE` antes de verificarla | En la carpeta de backups no aparecen `-wal`/`-shm` | — | 25 |
| 27 | Comentario de `db/schema.rs:11-20`, `CLAUDE.md` (WAL) y `ARQUITECTURA.md` (backups del home, nota de downgrade del lado datos) | — | D10, D12 | 25 |

**Validación de F2 (T7):** antes del merge, copiar la base real a un `tempdir`
(commit 19 o `sqlite3 … ".backup"`), abrir la app con `XDG_DATA_HOME` apuntando
ahí (`pnpm run tauri:dev`), usar "Trabajar", Parar, papelera y un restore, y
comparar el diff. La base real no se toca hasta que la fase está en `main`.

### F3 — Biblioteca, binario y config

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 28 | `lib.rs` (`gestor_proyectos_lib`) con `db` (incluido `db::restore` del commit 22), `models`, `tracking`, `config`, `logging`, `time`, `paths`; `main.rs` reexporta para que los `crate::db` sigan resolviendo | Mismos tests y mismos nombres (técnica del B14); `db::restore::restaurar_desde` visible desde `tests/` | A3 | 11, 21, 22 |
| 29 | `[[bin]] gestor-trackerd` + `default-run = "gestor-proyectos"` en el **mismo** commit. El binario arranca, toma el `flock` en `<data_dir>/trackerd.lock`, acepta `--data-dir`/`--config-dir`, aplica las guardas de debug y de `GESTOR_TRACKERD_TEST_TIMINGS`, loguea y sale. **Tarea de verificación:** `pnpm tauri build --bundles deb`, `dpkg -c` y qué queda en `target/release/`; el resultado se anota en 3.1 y en §1 | `cargo build --bins`; `cargo run` sin `--bin` lanza la app; el test toma el `flock` de `<data_dir>/trackerd.lock` y lanza el binario con **otro** `XDG_RUNTIME_DIR` y la misma base → sale con 75 (el binario de este commit sale enseguida, así que el que sostiene el lock tiene que ser el test); debug sin `--data-dir` ni `XDG_DATA_HOME` se niega; timings + base real se niega | B4, B8 | 28 |
| 30 | Sección `tracking` en `AppConfig` (`daemon_enabled`, `idle_minutes`, `min_credit_seconds`, `max_dirs_per_project`) con `#[serde(default)]` y rangos de 3.5, en la lib. Sin UI | Config vieja sin la sección → defaults; guardar y releer conserva los valores; valores fuera de rango → acotados | B6c | 28 |

### F4 — Motor puro

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 31 | Máquina de estados con reloj inyectado (3.4): `frontera`, margen DX1, `reanudado`, salto de reloj > 5 min, `idle_minutes` acotado, cierres coherentes, 0 filas → soltar, `daemon_enabled = false` → "cerrar todo y salir" | Guardado aislado → **60 s**; dos guardados a 2 min → 3 min; con `min_credit_seconds = 0` → no queda fila (la v2); margen recortado por la sesión anterior y por `reanudado`; pausa 2:59 contra 3:00; **evento antes del tick**; **suspensión de 8 h**; reloj −10 min → cierre + `last_error`; `idle=0` → 1, `idle=999` → 60; dos proyectos alternando | R1, R3, DX1, M7, M12, M14, M16, B5 | 30 |
| 32 | Validación de carpetas y atribución **al más específico** por componentes; empate exacto → los dos | `/home` rechazada; `/a/proj` no captura `/a/proj2`; `/a` y `/a/b`: evento en `/a/b/x` solo para `/a/b`, en `/a/c` para `/a`; misma carpeta → los dos | R1, R7, DX2 | 28 |
| 33 | Contención y validación en la lib, para los avisos de la app: contiene a / misma carpeta, `en_red` por `mountinfo`, sincronizada, R7 sobre ruta canonizada | 84 ⊃ 85; 90 = 91; symlink a `/` rechazado; `fuseblk` aceptado; `fuse.sshfs` rechazado (con `mountinfo` falso); `.stfolder` en un ancestro → sincronizada | R7, M4, M11, DX2 | 32 |
| 34 | Filtro de eventos y de nombres ignorados: tipos de 3.4, matchers de `.gitignore` por evento (anidados, `info/exclude`, global, `.gestorignore`) con guarda de raíz, `Remove` con `is_dir` desconocido, ignorados de M4, carpeta ignorada creada no cuenta | Lecturas no cuentan; `.swp` no; `Close(Write)` sí; `*.log` ignorado no; `mkdir target` en la raíz no; `.~lock.x.odt#` no; `foo_BACKUP.md` no; ruta fuera de raíz no entra en pánico | R2, M3, M4 | 28 |
| 35 | Escritor: acciones → base con `abrir_sesion`/`latir`/`cerrar_sesion`; volcado al abrir y como máximo cada 5 s con actividad; recuperación de huérfanas `file_watch`; `MAX(ended_at)` por proyecto para la `frontera`; `false` → soltar; `BUSY` → volcado pendiente; `user_version` atrasado espera, futuro ocioso | Contra base temporal: purga a mitad de sesión (`UPDATE` de 0 filas) no se cae; 100 eventos en 5 s → ≤ 2 escrituras; sin eventos → 0; `user_version` futuro → ninguna escritura y `estado = 'esquema_nuevo'`; `user_version = ESQUEMA_ACTUAL − 1` → ninguna escritura y `estado = 'esperando_migracion'`, y al migrar desde otra conexión → `activo` sin reiniciar; `journal_mode = delete` → espera igual, y al pasar a `wal` → `activo`; huérfana cierra con `duration = ended_at − started_at` | P4, B1, M7, M9, M12 | 10, 13, 14, 31 |

### F5a — Vigilante y config

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 36 | Vigilante `notify` + `ignore` con registro de vigilancias con **conteo de referencias**; directorios nuevos (con recorrido), renombrados, movidos, borrados; recorridos en hilo aparte; altas en lotes | Integración con `tempdir`: `mkdir -p a/b/c && touch a/b/c/x` cuenta; renombrar y escribir adentro cuenta; dos proyectos con la misma carpeta: quitar uno deja al otro midiendo; paraguas + hijo: quitar el paraguas deja al hijo | M2 | 34 |
| 37 | Desborde con rescate diferido; tope `max_dirs_per_project` → `demasiadas_carpetas`; `en_red`; límite de inotify; carpeta que desaparece; recarga de matchers al cambiar `.gitignore` | Vigilante falso: desborde + carpeta creada durante el desborde → vigilada tras la calma; tope+1 carpetas → no medible y sin vigilancias propias; `logs/` agregado al `.gitignore` en caliente deja de contar | M3, M10, M11, DX2, O3 | 33, 36 |
| 38 | Lector de config de solo lectura (struct del commit 30), `stat` cada 5 s y reparse por `mtime`/tamaño; `GESTOR_TRACKERD_TEST_TIMINGS` con `config=` y `rescan=` | Config ilegible → defaults y el archivo **no** se toca (ni `mtime`); `idle_minutes` en caliente → valor efectivo nuevo; `min_credit_seconds = 999` → 300 | P12, M14, DX1 | 30 |

### F5b — Bucle, servicio y E2E

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 39 | Bucle principal: relectura de proyectos por `PRAGMA data_version` (≤ 5 s) y cada 60 s, `tick` cada 15 s, latido de proceso, checkpoint por hora, logs agregados cada 10 min con límite de tasa | Proyecto nuevo o a la papelera sin reiniciar; papelera vista en ≤ 5 s; unitario del limitador de logs: 50 `warn` del mismo tipo en un minuto → 10 líneas y el resto contado en la línea agregada | M12, O2 | 35, 37, 38 |
| 40 | `sd-notify`, SIGTERM/SIGINT por `signal-hook`, orden de arranque de 3.4 (config antes de la base, `READY` antes del recorrido, `scan_completed_at`, `EXTEND_TIMEOUT_USEC`, watchdog solo desde el bucle), kill switch sale 0 con `pid = NULL` y siempre **después** de `READY=1` (A1) | SIGTERM deja todas cerradas y `pid = NULL`; 60.000 carpetas falsas → `READY` en < 1 s; `daemon_enabled = false` en caliente → salida 0; con un `NOTIFY_SOCKET` falso: `false` desde el arranque → `READY=1`, `STOPPING=1` y salida 0, en ese orden; `false` mientras espera una migración → lo mismo; unitario del watchdog: con un `sd_notify` falso, `WATCHDOG=1` sale una vez por vuelta del bucle y nunca desde otro hilo (M13) | B1, B5, M13, A1 | 39 |
| 41 | Harness E2E (`src-tauri/tests/e2e_trackerd.rs`, aislamiento de 5.2; **todos** sus tests llevan `#[ignore]`) + E1, E1b, E4, E8, E9, E14; `scripts/check.sh --e2e` con el control de cantidad de 5.5 (`E2E_ESPERADOS=6`) | Los escenarios, tres veces seguidas; con un test renombrado para que el filtro no lo encuentre, `check.sh --e2e` falla por cantidad | B6d, T3, T5, A2 | 40 |
| 42 | E2E restantes del demonio: E2, E3, E5, E6, E7, E10–E13, E15, E17–E21; `E2E_ESPERADOS` pasa a 21 | Los escenarios, tres veces seguidas | T1, T4, T5, O2, A2 | 28, 41 |

Desde el cierre de F5b, cada fase corre `scripts/check.sh --e2e` antes del merge.

### F6 — Instalación y kill switch

El demonio entra en producción en esta fase, y la fase entera se mergea junta:
`main` nunca tiene la instalación sin el toggle (B5). Además, desde F5b el
demonio ya sale solo con `daemon_enabled = false` en la config.

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 43 | `start-app.sh` deja de matar al demonio: hoy hace `pgrep -f "gestor-proyectos"` (`start-app.sh:19`) y `pkill -f "gestor-proyectos"` (`:21`), que **sí** alcanzan al demonio, porque su `ExecStart` lleva `--data-dir …/share/gestor-proyectos` (3.7). Los dos pasan al patrón acotado de abajo (solo procesos cuyo `argv[0]` es la app). No sirve `-x`: el nombre del proceso se trunca a 15 caracteres y `gestor-proyectos` tiene 16 | Paso manual 16 (5.3): con el demonio corriendo, `./start-app.sh` → `systemctl --user is-active gestor-trackerd` sigue en `active` | P10 | — |
| 44 | Generador del unit de 3.7 (rutas resueltas, `--data-dir`/`--config-dir`, aislamiento, recursos, backoff con alternativa para systemd < 254) + trait `ServiceManager` con `UNIT_DIR`/`BIN_DIR` por parámetro + ciclo de 3.8 en `.setup()`: condiciones (Linux, producción, sin `GESTOR_NO_TRACKERD`), **kill switch** (`daemon_enabled = false` → `disable --now`), ubicación de `current_exe`, comparación por contenido completo, `FragmentPath`, `is-enabled` sin reactivar un `disable` manual, `try-restart` una vez por apertura | Texto del unit (golden); unit idéntico → sin `daemon-reload`; un byte distinto → reescribe + `try-restart`; exe en `target/` o terminado en `(deleted)` → nada; ruta con espacio → error; `disabled` con unit previo → sin `enable`; sin unit previo → `enable --now`; `daemon_enabled = false` → `disable --now`; plataforma ≠ Linux → ninguna llamada; systemd 250 → `StartLimitBurst`; systemctl falso con `Result=exit-code` + `ExecMainStatus=226` → unit regenerado sin aislamiento y aviso; `Result=exit-code` con otro estado → unit intacto (O1) | R5, R6, B5, B9, O1, O5, M15 | 13, 29, 30 |
| 45 | Toggle mínimo "Tracker activado" en Ajustes + indicador mínimo (activo / apagado) en la cabecera; secuencia **config primero, después `systemctl`** | Vitest del toggle (orden config → comando, con mock); unitario del trait: `false` → `disable --now`, `true` → `reset-failed` + `enable --now` | B5 | 44 |
| 46 | E16 y E22: `systemd-run --user` con las propiedades del generador (5.2) | E16 y E22, locales, `#[ignore]` | T2, O1, A1 | 40, 44 |
| 47 | Restore con systemd: pasos 3 y 8 de 3.6 (`stop` recordando si estaba activo; `start` antes de `app.exit(0)` si estaba activo y `daemon_enabled` sigue en `true`) | Con systemctl falso: orden exacto (`stop` → lock → migrar → soltar → `start`); lock tomado → aborta sin tocar la base; unit `disabled` → sin `start` | B7 | 22, 44 |
| 48 | `scripts/release.sh`, `install.sh` y `uninstall.sh`: `PREFIX`/`BIN_DIR`/`UNIT_DIR`/`SYSTEMCTL`; `.desktop` con `$BIN_DIR` (hoy fijo en `install.sh:46`, y el heredoc pasa de `<< 'EOF'` a `<< EOF`, `install.sh:40`, para que la variable se expanda); temporal + `mv` de **los dos** binarios (hoy `cp` en `scripts/release.sh:304` e `install.sh:27`); `try-restart` si el unit existe; `install.sh` compila `gestor-trackerd` con `cargo build --release --bin gestor-trackerd` si falta (hoy solo exige `gestor-proyectos`, `install.sh:14-17`); `uninstall.sh` con `disable --now`, borrado del unit, `daemon-reload`, `reset-failed`, sin tocar la base | `PREFIX=$tmp SYSTEMCTL=true ./install.sh` dos veces seguidas, y el `.desktop` generado tiene `Exec=$tmp/bin/gestor-proyectos` (no la ruta literal); `scripts/release.sh patch --dry-run`; `uninstall.sh` con `PREFIX=$tmp` deja la base | O4, O5, T6 | 44 |

Patrón del commit 43, para `pgrep -f` y `pkill -f` (fuera de la tabla para no
escapar la barra vertical):

```bash
pkill -f '^([^ ]*/)?gestor-proyectos( |$)'
```

**Validación de F6:** pasos manuales 3–6 y 8–18 de 5.3 (instalación
real con `install.sh`, sin `release.sh`).

### F7 — Interfaz completa

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 49 | `get_live_tracking`: sesiones abiertas de las dos fuentes + `tracker_status` + `elapsed_seconds` de la `ActiveSession` de esta app | Unitario | M5 | 13, 17 |
| 50 | Contador en vivo (reglas de 3.9) y refresco del badge al cerrarse una sesión | Vitest con timers falsos: `file_watch` nunca pasa de `last_activity_at`; suspensión simulada no hace bajar el contador de "Trabajar" más de 5 s; el badge se refresca al cerrar | P5, M5, M9 | 49 |
| 51 | Indicador global de 6 estados (3.9), reemplaza al mínimo del commit 45 | Vitest por estado | B5 | 45, 49 |
| 52 | Ajustes → Tracking completo: inactividad, margen por guardado (DX1), tope de carpetas (DX2), motivos de no medibles, uso de inotify; Ajustes → Backups con la nota de D12 (3.9) | Vitest + config vieja sin la sección; inactividad cambiada → muestra el valor efectivo cuando el demonio lo aplica; Vitest: la nota de backups del home aparece en Ajustes → Backups | DX1, DX2, O3, M14, D12 | 45 |
| 53 | Avisos por proyecto de 3.9, calculados con las funciones de la lib del commit 33 | Vitest por motivo; con el demonio apagado, `carpeta_amplia` igual aparece | R7, DX2, M4 | 33, 49 |

**Validación de F7:** paso manual 7 de 5.3 y Vitest.

### F8 — Validación y documentación

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 54 | `scripts/medir-trackerd.sh`: RSS, `%CPU`, `memory.current` del cgroup y `watched_dirs` (6) | Correrlo con el demonio instalado; salida con los cuatro valores | T8, O3 | 48 |
| 55 | `scripts/datos-historicos.sh`: aplica lo decidido en DX3 y DX4 sobre la base real. Por defecto solo muestra lo que haría (las filas y proyectos afectados, con la lista del commit 19); con `--aplicar` exige un "sí" escrito del usuario, hace antes un backup `VACUUM INTO` verificado con `integrity_check`, y aplica en una transacción `IMMEDIATE`. Con DX3 (a), "marcar" no toca la base: solo genera la lista para el CHANGELOG | Contra una copia: sin `--aplicar` la base queda igual byte a byte; con `--aplicar` y una respuesta distinta de "sí" → nada cambia; con "sí" → backup presente y verificado antes del primer cambio | D3, DX3, DX4 | 19 |
| 56 | `ARQUITECTURA.md`, `CLAUDE.md` (sección Time tracking: los dos caminos, el demonio, el piso de downgrade), CHANGELOG con cada diferencia del diff con datos reales (D3), la lista de sesiones > 12 h marcadas (DX3) y el "piso de downgrade". Después, `scripts/release.sh minor` | — | D3, D10 | todos |

### 4.10 Verificación del orden

| Algo que se usa | Lo crea | Lo usan |
|---|---|---|
| `scripts/check.sh` | 2 | Todos desde el 3 |
| `.setup()` | 4 | 15, 44 |
| Regla mínima | 8 | 15, 16 |
| `migrar`, `abrir_existente` | 10 | 12, 13, 22, 35 |
| `db::restore` (`tomar_lock`, `restaurar_desde`) | 22 | 28 (entra en la lib), 42 (E15), 47 |
| `last_activity_at` | 12 | 14, 15, 35 |
| `tracker_status` | 13 | 35, 44, 49 |
| `abrir_sesion`/`latir`/`cerrar_sesion` | 14 | 15, 35 |
| Feature `backup` de rusqlite | 19 | 22 |
| `paths.rs` | 21 | 22, 24, 28, 29 |
| WAL | 25 | 35 en adelante (el demonio espera si no está) |
| Lib | 28 | 29, 32, 34 y todo F4–F5 |
| Sección `tracking` | 30 | 31, 38, 44, 45, 52 |
| Funciones de contención | 33 | 37, 53 |
| Demonio completo | 40 | 41, 42, 46 |
| Control de cantidad de `check.sh --e2e` | 41 | 42 |
| Trait `ServiceManager` | 44 | 45, 46, 47, 48 |
| Lista del diff con datos reales | 19 | 55, 56 |

Ningún "Depende de" apunta a un número mayor.

### 4.11 Estimación

Días efectivos de un desarrollador, con la prueba manual incluida y sin contar las
esperas por decisiones de la sección 9 ni la hora de medición de "listo".

| Fase | Commits | Días | Lo que más pesa |
|---|---|---|---|
| F0 | 9 | 1–2 | Commit 4 (inicialización después de la instancia única, 3.8) |
| F1 | 11 | 4–5 | Migraciones versionadas, unión con tests de tabla, diff con datos reales |
| F2 | 7 | 2,5–3 | Restore con la API de backup y la validación sobre la copia real |
| F3 | 3 | 1 | Verificación del bundle |
| F4 | 5 | 2,5–3 | Motor con DX1 y escritor |
| F5a | 3 | 2–2,5 | Conteo de referencias y rescate tras desborde |
| F5b | 4 | 2–3 | E2E estables (centinelas, sondeo) |
| F6 | 6 | 2–3 | Unit con aislamiento, E16 y E22, scripts |
| F7 | 5 | 1,5–2,5 | Contadores y Ajustes |
| F8 | 3 | 1–1,5 | Script de datos históricos, corrida triple y docs |
| **Total** | **56** | **20–27** | La v3.1 suma ~0,5 día de tests repartidos (A1, A3, B1, O1, O2) y el commit 55 |

---
## 5. Pruebas

### 5.1 Pirámide

| Nivel | Qué cubre | Herramienta | Cuándo |
|---|---|---|---|
| Unitario | Tramos y unión, `time.rs` (zonas con `chrono-tz`), migraciones, motor, filtros, atribución, carpetas, texto del unit, trait de systemctl | `cargo test` | Cada commit |
| Integración | Escritor contra SQLite real; vigilante contra `tempdir`; dos conexiones concurrentes; restore | `cargo test` (`src-tauri/tests/`) | Cada commit |
| Punta a punta | Demonio real como proceso + base temporal: E1–E15, E17–E21 (5.2). **Todos** los tests de `tests/e2e_trackerd.rs` llevan `#[ignore]`, así que `cargo test` solo no los corre; `--e2e` verifica que corrieron los esperados (5.5) | `scripts/check.sh --e2e` | Al cerrar cada fase desde F5b |
| Systemd local | E16 (el unit real con su aislamiento, como unidad transitoria) y E22 (kill switch al arrancar) | `cargo test --test e2e_trackerd -- --ignored e16` y `… e22` | Al cerrar F6 y F8. Nunca en CI |
| Datos reales | Diff viejo contra nuevo sobre una copia de la base real (5.4) | Test `#[ignore]` del commit 19 | Al cerrar F1, F2 y F8 |
| Frontend | Contadores, indicador, ajustes, avisos, errores | `pnpm test` (Vitest) | Cada commit |
| Manual guiado | 5.3 | Persona + checklist | Según la columna "Fase" de 5.3; completo al cerrar F8 |

Hoy hay **171** `#[test]` en el crate (medido). Los tests de integración no pueden
usar `db/test_support.rs` (es `#[cfg(test)]`): usan la API pública de la lib, que
alcanza (`Database::new`, `create_project`, `abrir_sesion`, `latir`,
`cerrar_sesion`, `tiempo_trabajado`, `db::restore::tomar_lock` y `restaurar_desde`).

**Zonas horarias (D13):** ningún test cambia `TZ` del proceso: los tests corren
en paralelo y sería una carrera. `rango_dia` recibe la zona y los casos usan
`chrono-tz` (`[dev-dependencies]`, commit 11):

| Zona | Caso |
|---|---|
| `America/Argentina/Buenos_Aires` | El uso real: sin horario de verano, el día empieza a las 03:00 UTC |
| `America/Santiago` | Medianoche inexistente al empezar el horario de verano (el reloj salta a las 24:00) |
| `America/Havana` | Medianoche inexistente en marzo y ambigua en noviembre (de 01:00 vuelve a 00:00) |
| `Europe/Madrid` | Días de 23 y 25 h sin tocar la medianoche: el rango igual mide lo que corresponde |

Cada caso afirma primero su precondición (que esa medianoche realmente no existe o
es ambigua): si la base de zonas cambia, el test falla por eso y no queda probando
nada en silencio. `active_today` en hora local se prueba en el commit 17 pasando
la zona, no el reloj del sistema.

### 5.2 Punta a punta automatizado

**Aislamiento (O5, B8).** Ningún test automatizado toca la base, la config, el
unit ni los binarios reales:

| Qué | Cómo se aísla |
|---|---|
| Base y lock | `XDG_DATA_HOME` temporal **y** `--data-dir` explícito. El harness, antes de lanzar, canoniza la ruta y **falla** si es `$HOME/.local/share/gestor-proyectos` o no está bajo el `tempdir` (doble guarda: el demonio ya se niega con `GESTOR_TRACKERD_TEST_TIMINGS` definida, 3.4) |
| Config | `XDG_CONFIG_HOME` temporal y `--config-dir` |
| Runtime | `XDG_RUNTIME_DIR` temporal. E14 usa **otro** distinto para el segundo demonio, a propósito |
| Proceso | El harness guarda el `Child` en un struct con `Drop` que manda SIGKILL: un test que falla no deja demonios vivos |
| Unit y systemctl | Los tests del trait (commit 44) usan `UNIT_DIR` y `BIN_DIR` en un `tempdir` y un systemctl falso que registra las llamadas |
| Scripts | `PREFIX=$tmp BIN_DIR=$tmp/bin UNIT_DIR=$tmp/systemd SYSTEMCTL=true` (commit 48) |
| E16 | Unidad **transitoria** (`systemd-run --user`): vive en `$XDG_RUNTIME_DIR/systemd/transient/`. El test afirma que `~/.config/systemd/user/` tiene los mismos archivos antes y después |

Todos los E2E corren con
`GESTOR_TRACKERD_TEST_TIMINGS=idle=3s,tick=250ms,flush=250ms,reload=1s,heartbeat=1s,config=200ms,rescan=1s`.
Ningún test usa esperas fijas: **sondea hasta un plazo** (10 s) y, si no se
cumple, falla imprimiendo las filas de `time_tracking_sessions` y
`tracker_status`.

**Negativos con centinela (T3).** "No pasó nada" no se puede probar esperando: un
demonio todavía recorriendo tampoco registra nada. Los escenarios negativos (E4,
E5, E12, E18 y la parte "hacia afuera" de E7) siguen siempre este orden:
1. Esperar `scan_completed_at` no nulo y `watched_dirs > 0`.
2. Hacer la acción negativa en el proyecto A.
3. Hacer **un** guardado válido en el proyecto centinela B, después.
4. Esperar a que aparezca la sesión de B. Como los eventos se procesan en orden, cuando B ya tiene sesión, el evento de A ya pasó por el filtro.
5. Afirmar que A no tiene ninguna sesión.

**Tolerancias (T5):** duraciones ±1 s (E1, E1b, E8, E9); plazo de sondeo 10 s;
E10 dura 20 s con 4 escritores en vez de 2 min con uno.

| # | Escenario | Se espera | Commit |
|---|---|---|---|
| E1 | `min_credit_seconds = 0`. Escribir cada 500 ms durante 4 s | 1 sesión `file_watch` de 3,5 s ± 1, cerrada en el último cambio | 41 |
| E1b | Igual, con el margen por defecto | Margen = `min(60 s, idle)` = 3 s con los tiempos de test: 6,5 s ± 1, y `started_at` 3 s antes del primer cambio (DX1) | 41 |
| E2 | Dos proyectos alternados | Dos sesiones superpuestas, una por proyecto (R1); el stderr del demonio no contiene la ruta del `tempdir` (O2) | 42 |
| E3 | Sesión `work_button` (vía `abrir_sesion` + `latir`) superpuesta a una `file_watch` del mismo proyecto | `tiempo_trabajado` = unión, no la suma (R4) | 42 |
| E4 | Solo leer (`cat`, `rg`) | Ninguna sesión (centinela) | 41 |
| E5 | Escribir en `node_modules/`, en un `*.log` ignorado por `.gitignore`, y `mkdir target` en la raíz | Ninguna sesión (centinela); el stderr del demonio no contiene la ruta del `tempdir` (O2) | 42 |
| E6 | `mkdir -p a/b/c && echo x > a/b/c/f` | Cuenta | 42 |
| E7 | Renombrar una carpeta y escribir adentro; mover una carpeta **de afuera hacia adentro** y escribir; moverla **hacia afuera** y escribir; con dos proyectos A y B, mover `A/sub` → `B/sub` y escribir en `B/sub` | Las dos primeras cuentan para el proyecto; la tercera no (centinela); la última cuenta **solo** para B, y A suelta las vigilancias de `sub` (registro con conteo: `watched_dirs` de A baja) (T4) | 42 |
| E8 | `kill -9` con sesión abierta y relanzar | La huérfana cierra en su `last_activity_at` ± 1 s, con `duration = ended_at − started_at` | 41 |
| E9 | SIGTERM con sesión abierta | Cerrada en el último cambio; `pid = NULL` | 41 |
| E10 | 4 conexiones escriben en paralelo durante 20 s mientras el demonio mide | Cero `SQLITE_BUSY` en los escritores y en el log del demonio | 42 |
| E11 | Proyecto agregado, a la papelera y purgado en caliente | Se vigila; deja de medir en ≤ 5 s; el demonio no se cae | 42 |
| E12 | Proyecto con carpeta `/home` | No se vigila y aparece en `unwatched_projects` con `carpeta_amplia` (centinela) | 42 |
| E13 | Config ilegible | Defaults, `last_error` con el motivo, y el archivo intacto: mismos bytes y mismo `mtime` | 42 |
| E14 | Segundo demonio sobre la misma base, con **otro** `XDG_RUNTIME_DIR` | Sale con 75 por el `flock` (B8) | 41 |
| E15 | Restore con el demonio vivo, sin systemd (T1): (a) con el demonio corriendo, `db::restore::tomar_lock` (lib, A3) **falla** por el `flock` y la base no cambia; (b) se para el demonio, se restaura un backup de **esquema viejo** (`user_version = 0`, sin `last_activity_at` ni `tracker_status`), se relanza | (a) error en español, base igual byte a byte; (b) `integrity_check = ok`, `user_version = ESQUEMA_ACTUAL`, columnas y tablas nuevas presentes, `journal_mode = wal`, ninguna sesión abierta, y el demonio vuelve a medir | 42 |
| E16 | `systemd-run --user --unit=gestor-trackerd-test-$PID -p Type=notify -p WatchdogSec=5` con las mismas propiedades que arma el generador **salvo `WatchdogSec=5`** (el unit real usa 60), `--setenv` de `XDG_*` temporales y un proyecto bajo `$HOME` (T2, O1) | `is-active = active` tras `READY`; un guardado crea la sesión; `systemctl --user stop` la cierra; `~/.config/systemd/user` sin cambios | 46 |
| E17 | Cambiar `idle_minutes` y `min_credit_seconds` en la config con el demonio vivo (T4) | `tracker_status.idle_minutes` y `min_credit_seconds` cambian en ≤ 5 s sin reiniciar; `idle_minutes = 999` publica 60 (M14) | 42 |
| E18 | Agregar `logs/` al `.gitignore` en caliente y escribir `logs/x.log` (T4) | No cuenta (centinela); sacarlo del `.gitignore` y escribir de nuevo → cuenta | 42 |
| E19 | Paraguas `/t/p` con hijo `/t/p/h` (DX2, M2) | Un guardado en `h` cuenta solo para el hijo; en `/t/p/otra` para el paraguas; mandar el paraguas a la papelera deja al hijo midiendo | 42 |
| E20 | `daemon_enabled = false` en la config con el demonio vivo y una sesión abierta (B5, T4) | Sesión cerrada en su último cambio, salida con código 0 en ≤ 5 s, `estado = 'desactivado'`, `pid = NULL` | 42 |
| E21 | Concurrencia real con los métodos de la app (T4): un hilo hace el latido de `work_button` cada 250 ms, cierra con `end_tracking_session`, manda un proyecto a la papelera y lo purga, mientras el demonio mide otro proyecto | Ningún error, ningún `SQLITE_BUSY`; el total del proyecto superpuesto es la unión; el demonio suelta el purgado sin caerse | 42 |
| E22 | Kill switch al arrancar, con el systemd real (A1): `systemd-run --user --wait` como E16 (con `Restart=on-failure`), con `daemon_enabled = false` en la config temporal antes de lanzar | `--wait` termina en < 15 s con "Finished with result: success" y `status=0`. Con el defecto de A1 la unidad terminaría `protocol`, se reiniciaría y `--wait` no volvería: el plazo vencido es la falla. Una unidad transitoria que termina bien se descarga sola, por eso se lee el resultado de `--wait` y no de `systemctl show`; `~/.config/systemd/user` sin cambios | 46 |

**Lo que no se automatiza, y por qué:**
- **Suspensión real:** la cubren los tests del motor con reloj falso (commit 31: 8 h, `reanudado`) y el paso manual 11.
- **Watchdog con el bucle colgado:** no se puede colgar el bucle desde afuera sin instrumentarlo. Lo cubre el unitario del commit 40 (M13); que systemd reinicie al vencer el plazo se confía a systemd.
- **Reinicio de la PC y sesión gráfica real:** paso manual 10.

### 5.3 Recorrido manual

Instalación **real** con `install.sh`, nunca con `scripts/release.sh patch --install`,
que crea un commit y un tag de verdad (T6). Antes de empezar: backup desde Ajustes.

| # | Fase | Paso | Se espera |
|---|---|---|---|
| 1 | F0 | Lanzar la app dos veces seguidas | Una sola ventana y **un** backup nuevo en la carpeta de backups (P11) |
| 2 | F1, F2 | Con una copia de la base real (`XDG_DATA_HOME=$tmp pnpm run tauri:dev`): mirar tarjetas y Dashboard, "Trabajar" 5 s y Parar, papelera, restore | Totales iguales al diff del commit 19; Parar a los 5 s deja 0 s; tarjetas con tiempo aunque el proyecto no tenga `.gestor/` |
| 3 | F6 | `pnpm run tauri:build`, `./install.sh`, abrir la app una vez | Unit en `~/.config/systemd/user/`, binario en `~/.local/bin/` |
| 4 | F6 | `systemctl --user status gestor-trackerd` | `active (running)`; la app muestra "Tracker activo" |
| 5 | F6 | **Cerrar la app.** Editar dos proyectos durante 10 min, alternando | — |
| 6 | F6 | Abrir la app | Cada proyecto muestra su propio tiempo, sin repartir (R1, R6) |
| 7 | F7 | Con la app abierta, editar | El contador avanza con cada guardado, con ≤ 10 s de retraso (3.9) |
| 8 | F6 | Dejar pasar 3 min sin tocar nada | La sesión cierra en el último guardado (R3) |
| 9 | F6 | "Trabajar" + editar el mismo proyecto | El total no cuenta doble (R4) |
| 10 | F6 | Reiniciar la PC, **no** abrir la app, editar archivos; después abrir la app | El tiempo está (R6) |
| 11 | F6 | Editar, suspender la notebook 10 min, volver y editar | Dos sesiones; la pausa no cuenta, ni con el margen (DX1) |
| 12 | F6 | Ajustes → Desactivar | `systemctl --user is-enabled gestor-trackerd` → `disabled`; `is-active` → `inactive`; `sqlite3 …/projects.db "SELECT COUNT(*) FROM time_tracking_sessions WHERE source='file_watch' AND ended_at IS NULL"` → 0. Cerrar y abrir la app: sigue apagado (B5) |
| 13 | F6 | Ajustes → Activar; después `systemctl --user disable --now gestor-trackerd` a mano; cerrar y abrir la app | La app **no** lo reactiva y dice "desactivado desde systemctl" (3.8) |
| 14 | F6 | Activar de nuevo; `systemctl --user stop gestor-trackerd`; abrir la app | "Tracker apagado" mientras está parado; al abrir la app vuelve a `active` (un `stop` sí se revierte) |
| 15 | F6 | Restaurar un backup | Base íntegra; el demonio está `active` **sin** reabrir la app (B7) |
| 16 | F6 | Con el demonio corriendo, `./start-app.sh` | El demonio sigue `active` (P10) |
| 17 | F6 | Recompilar y `./install.sh` con el demonio corriendo | Sin `ETXTBSY`; `systemctl --user show -p ExecMainPID gestor-trackerd` cambió (hubo `try-restart`). Si la versión cambió, `tracker_status.version` también |
| 18 | F6 | `systemd-analyze security --user gestor-trackerd` | Se anota el puntaje en el merge de F6 |

### 5.4 Datos reales

El test del commit 19 se corre así, sobre una copia y nunca sobre la base viva:

```bash
GESTOR_GOLDEN_DB=~/.local/share/gestor-proyectos/projects.db \
  cargo test --manifest-path src-tauri/Cargo.toml golden_diff -- --ignored --nocapture
```

El test copia la base con la API de backup desde una conexión de solo lectura y
trabaja sobre la copia. La salida tiene cuatro partes: diferencias por proyecto,
sesiones de más de 12 h, huérfanas y proyectos con carpeta inexistente. Cada
diferencia tiene que estar explicada en el CHANGELOG (commit 56): si aparece una
que no se puede explicar, la fase no se mergea.

### 5.5 Calidad automatizada: `scripts/check.sh`

Lo crea el commit 2. El modo `--e2e` se agrega en el commit 41.

```bash
#!/usr/bin/env bash
# Puerta de cada commit. Uso: scripts/check.sh [--e2e]
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

# Cantidad de E2E automatizados que tienen que correr (6 en el commit 41, 21 desde el 42).
E2E_ESPERADOS=21

if [[ "${1:-}" == "--e2e" ]]; then
  for i in 1 2 3; do
    echo "== E2E, corrida $i de 3 =="
    salida=$(cd src-tauri && cargo test --test e2e_trackerd -- --ignored --test-threads=1 \
      --skip e16 --skip e22 2>&1) || { echo "$salida"; exit 1; }
    echo "$salida"
    corridos=$(sed -nE 's/^test result: ok\. ([0-9]+) passed.*/\1/p' <<<"$salida")
    if [[ "${corridos:-0}" -lt "$E2E_ESPERADOS" ]]; then
      echo "E2E: corrieron ${corridos:-0} y se esperaban $E2E_ESPERADOS" >&2
      exit 1
    fi
  done
fi
```

- **Verde vacío (A2):** todos los tests de `tests/e2e_trackerd.rs` llevan `#[ignore]`,
  así que `cargo test` sin `--ignored` los saltea, y un filtro mal escrito o un test
  renombrado daría "0 passed" en verde. El script parsea la línea
  `test result: ok. N passed` y falla si `N < E2E_ESPERADOS`. Cada commit que agrega
  E2E sube la constante.

- `cargo fmt --check` pasa desde el commit 1, que formatea el crate entero (hoy hay 127 bloques sin formatear).
- `--test-threads=1` en los E2E: cada uno lanza su demonio con tiempos cortos, y en paralelo competirían por CPU y volverían inestables los plazos.
- E16 y E22 se excluyen del script: necesitan el bus de usuario de systemd y se corren a mano (5.1).
- `pnpm run lint` sin `--fix`: la puerta no modifica archivos.

**CI (DX5, decisión abierta, 9.2):** `.github/workflows/check.yml` (hoy no existe
`.github/`) que corra `scripts/check.sh` en `ubuntu-latest`, sin `--e2e` ni E16/E22.
Requiere autorización aparte, por ser remoto.

---
## 6. Definición de "listo"

Cada punto se verifica con un comando o un paso concreto; ninguno queda a criterio.

- [ ] **Calidad:** `scripts/check.sh` en verde sobre `main` después del merge de F8.
- [ ] **E2E estables:** `scripts/check.sh --e2e` pasa completo: E1–E15 y E17–E21 (21 tests, verificado por la cuenta de 5.5), **tres corridas seguidas** sin un solo fallo. Un test inestable se arregla, no se reintenta.
- [ ] **E16 y E22** (`cargo test --test e2e_trackerd -- --ignored e16`, ídem `e22`) pasan en esta máquina con el systemd del usuario.
- [ ] **Recorrido manual** 5.3 completo (pasos 1–18) sobre la instalación real hecha con `install.sh`.
- [ ] **Kill switch probado** (paso manual 12): después de desactivar desde Ajustes, `systemctl --user is-enabled gestor-trackerd` devuelve `disabled`, `is-active` devuelve `inactive` y la consulta de sesiones `file_watch` abiertas devuelve **0**. Al reabrir la app sigue apagado.
- [ ] **Diff con datos reales** (5.4) corrido sobre una copia fresca de la base real, con **cada** diferencia explicada en el CHANGELOG. Las decisiones de la sección 9 sobre sesiones > 12 h, huérfanas y carpetas inexistentes están tomadas y aplicadas (o explícitamente "dejar").
- [ ] **Cordura de la unión:** en la salida del diff, ningún proyecto suma en un día local más de 86.400 s (90.000 s en un día de 25 h).
- [ ] **Recursos**, después de **1 h** de demonio corriendo sobre los proyectos reales, con `scripts/medir-trackerd.sh` (commit 54), que hace:
  ```bash
  pid=$(systemctl --user show -p MainPID --value gestor-trackerd)
  ps -o rss=,%cpu= -p "$pid"                                   # RSS en KiB
  systemctl --user show -p MemoryCurrent --value gestor-trackerd   # memory.current del cgroup, incluye marcas de inotify
  a=$(systemctl --user show -p CPUUsageNSec --value gestor-trackerd); sleep 600
  b=$(systemctl --user show -p CPUUsageNSec --value gestor-trackerd)
  awk -v a="$a" -v b="$b" 'BEGIN { printf "CPU en reposo: %.2f %%\n", (b - a) / 6e9 }'   # 600 s → %
  sqlite3 ~/.local/share/gestor-proyectos/projects.db "SELECT watched_dirs, watched_projects FROM tracker_status"
  ```
  Umbrales: **RSS < 50 MB**, **`MemoryCurrent` < 128 MB** (debajo de `MemoryHigh=192M`, O3), **CPU < 1 %** en los 10 min sin tocar archivos. Se anotan los cuatro valores en el merge de F8.
- [ ] **Matriz de trazabilidad** (abajo) sin huecos: cada fila tiene commit y prueba, y la prueba pasa.
- [ ] **Documentación:** `ARQUITECTURA.md` y `CLAUDE.md` describen el tracking nuevo (sección Time tracking, kill switch, piso de downgrade); el CHANGELOG del release `minor` tiene el "piso de downgrade" (7.2).

### 6.1 Matriz de trazabilidad (T12)

Commits con la numeración de la sección 4. "Manual N" es el paso N de 5.3.

**Problemas de partida (2.2):**

| # | Commit | Prueba |
|---|---|---|
| P1 Dos totales | 17, 20 | Tests del 17 (sin superposición, mismos números); `rg total_time_seconds` del 20; E3 |
| P2 Cierre doble suma doble | 3 | "Cerrar dos veces suma una" |
| P3 Hoy/semana en UTC, semana de 8 fechas | 11, 17 | Tests de zonas (11); semana = 7 fechas y `active_today` local (17) |
| P4 Huérfanas | 12, 15, 35 | Tests de huérfanas del 15 y del 35; E8 |
| P5 Badge sin refresco | 50 | Vitest del badge |
| P6 Restore con `rename` | 22, 47 | "Restore con `-wal` presente no corrompe" (22); E15; manual 15 |
| P7 Error de "Trabajar" invisible | 6 | Vitest del toast |
| P8 "Abrir" del Dashboard no mide | 9 | Vitest |
| P9 Sin botón Parar | 7 | Vitest + test del comando |
| P10 `pkill -f` mata al demonio | 43 | Manual 16 |
| P11 Base abierta antes de la instancia única | 4 | Manual 1 |
| P12 `ConfigManager` en el demonio | 38 | "Config ilegible no se toca" (38); E13 |
| P13 Transacciones diferidas | 23 | Dos conexiones en `restore_project` sin `SQLITE_BUSY_SNAPSHOT`; guarda de `purge_project` dentro de la transacción; E10, E21 |
| P14 Migraciones sin versión | 10 | Tests (a)–(f) del 10 |
| P15 Stats solo con `.gestor/` | 18 | Vitest sin `.gestor/`; manual 2 |
| P16 `try_lock` | 5 | Lock sostenido 200 ms; Mutex envenenado |
| P17 Ruta de la base en `backup/` | 21 | Test de rutas con XDG temporales |
| P18 Error → 0 s | 17 | "Tabla borrada → `Err`" |
| P19 Permisos 0644 | 24, 25 | Test de modos (24); `-wal`/`-shm` en `0600` (25) |

**Requisitos (1):**

| # | Commit | Prueba |
|---|---|---|
| R1 Cada proyecto lo suyo | 31, 32 | Motor "dos proyectos alternando"; atribución más específica; E2, E19; manual 5–6 |
| R2 Actividad en archivos, leer no cuenta | 34, 36 | Filtros del 34; E4, E5, E6, E7, E18; manual 7 |
| R3 Corte de 3 min en el último cambio | 31 | Motor "2:59 contra 3:00", "evento antes del tick"; E1, E1b, E17; manual 8 |
| R4 Unión con "Trabajar" | 16, 17 | Tests de tabla del 16; E3, E21; manual 9 |
| R5 Solo Linux | 44 | "Plataforma ≠ Linux → ninguna llamada a systemctl" |
| R6 Corre con la app cerrada, arranca solo, apagable | 40, 44, 45, 46, 47 | Tests del trait (44, 45); E16, E22; manual 3–6, 10, 12–15 |
| R7 Carpetas no medibles | 32, 33, 37, 53 | `/home` y symlink a `/` rechazados; `en_red`; tope+1 carpetas; E12; Vitest de avisos (53) |

**Bloqueantes de la auditoría (Anexo A, v3):**

| # | Commit | Prueba |
|---|---|---|
| B1 Migraciones y demonio que no migra | 10, 35, 40 | Tests (a)–(f) del 10; `user_version` futuro → ninguna escritura, y `ESQUEMA_ACTUAL − 1` → `esperando_migracion` y `activo` al migrar desde otra conexión (35); `false` mientras espera → `READY`, `STOPPING`, salida 0 (40); E15 |
| B2 Tiempo invisible sin `.gestor/` | 18 | Vitest del 18 |
| B3 Contador sumado con 0 filas | 3 | "Cerrar dos veces suma una" |
| B4 `default-run` y bundle | 29 | `cargo run` sin `--bin` lanza la app; tarea `dpkg -c` anotada en 3.1 |
| B5 Kill switch | 31, 40, 44, 45, 46, 51 | Motor "cerrar todo y salir"; `NOTIFY_SOCKET` falso: `READY` antes de salir (40); E20, E22; trait `false` → `disable --now`; Vitest del toggle; manual 12–13 |
| B6 Orden roto | 1, 2, 13, 30, 41 | 4.10 sin dependencias hacia adelante; `check.sh` desde el commit 2 |
| B7 Restore deja el demonio parado | 22, 47 | Orden de llamadas con systemctl falso (47); E15; manual 15 |
| B8 Lock y entorno | 29 | Segundo demonio con otro `XDG_RUNTIME_DIR` → 75; guardas de debug y de timings; E14 |
| B9 Unit desde `target/`, comparación por versión | 44 | Golden del unit; exe en `target/` → nada; un byte distinto → reescribe |

---
## 7. Riesgos y mitigaciones

| # | Riesgo | Prob. | Impacto | Mitigación |
|---|---|---|---|---|
| 1 | Herramientas que escriben solas (formateadores, watchers, `.eslintcache`, `*.tsbuildinfo`) inflan el tiempo | Media | Alto | Lista de ignorados, **`.gitignore` aplicado a cada evento** (M3), `.gestorignore`, corte de 3 min. E5 y el recorrido manual |
| 2 | Sincronizadores (Syncthing, Dropbox, Nextcloud) escriben archivos de otro equipo y cuentan como trabajo | Media | Medio | Temporales y carpetas de control ignorados (M4); proyecto marcado "sincronizado" con aviso. No se puede distinguir quién escribió: inotify no da el proceso |
| 3 | El margen de 60 s (DX1) suma tiempo que no fue | Alta | Bajo | Acotado por la sesión anterior y por la última suspensión (3.4). Configurable a 0. Es la regla aceptada: nunca más que el margen |
| 4 | Lecturas masivas (indexadores) llenan la cola de inotify y gastan CPU aunque se filtren | Media | Medio | Se filtran temprano. El objetivo de < 1% de CPU se mide en "listo". Con `notify` 9 estable, filtrar en el kernel (`EventKindMask::CORE`) |
| 5 | Un desborde pierde `Create` de carpetas y lo nuevo queda sin vigilar | Media | Medio | Rescate diferido y limitado (M10, 3.5) |
| 6 | `git pull` o `checkout` acreditan unos segundos | Alta | Bajo | Aceptado y documentado: `.git/` no se vigila |
| 7 | Límite de inotify compartido con VSCode, JetBrains o Syncthing; memoria del kernel (~1 KB por vigilancia) | Media | Medio | Tope por proyecto (DX2), uso visible en Ajustes, `memory.current` medido en "listo" (O3). Si se llena: `last_error` y proyecto `limite_inotify` |
| 8 | Proyecto paraguas (`~/2025`) duplica vigilancias o le saca las suyas a los hijos | Media | Alto | Registro con conteo de referencias (M2), atribución al más específico (DX2) |
| 9 | Proyecto grande legítimo queda "no medible" por el tope | **Alta** | Medio | Moodle Potenciarg (55) tiene ~27.100 carpetas: con el default 5.000 no se mide. El aviso dice cómo subir el tope o usar `.gestorignore`. DX2 confirmada: 5.000 y `.gestorignore` para el 55 |
| 10 | Montaje de red caído cuelga un recorrido o un `stat` | Baja | Alto | `en_red` no se vigila (M11). FUSE local (`fuseblk`, 14 proyectos en `/mnt/miparticion`) sí |
| 11 | Bloqueos entre app y demonio | Media | Alto | WAL + transacciones `IMMEDIATE` (3.6) + E10. El demonio no reintenta en caliente: deja el volcado pendiente |
| 12 | Restore con el demonio vivo | Baja | Crítico | API de backup + `stop` + la app toma `trackerd.lock` + migra antes de relanzar (3.8, B7) |
| 13 | Dos demonios con entornos distintos escriben la misma base | Baja | Alto | Lock en `<data_dir>/trackerd.lock` y rutas explícitas en el unit (B8) |
| 14 | Un binario de debug o un test escribe la base real | Media | Alto | `tauri dev` no instala; el unit solo apunta a `BIN_DIR`; en debug el demonio exige `--data-dir`; con `GESTOR_TRACKERD_TEST_TIMINGS` se niega a usar la base real (B8, B9) |
| 15 | Números históricos cambian con la unión | Media | Medio | Diferencias explicadas en el CHANGELOG con el diff "golden" sobre una copia de la base real (D3) |
| 16 | Versiones distintas de app y demonio; base más nueva que el demonio | Media | Medio | El demonio nunca migra ni escribe la config; con `user_version` mayor queda ocioso sin escribir (B1); `try-restart` una vez por apertura |
| 17 | El demonio corre en producción sin forma de apagarlo | Baja | Alto | Toggle e indicador mínimos en F6; el apagado por config llega en ≤ 5 s aunque `systemctl` falle (B5) |
| 18 | La app reactiva un demonio que el usuario deshabilitó a mano | Media | Medio | Si el unit ya existía y está `disabled`, no se toca (3.8 paso 6) |
| 19 | Reinicios agotados con la app cerrada → días sin medir | Baja | Alto | `StartLimitIntervalSec=0` + espera exponencial hasta 5 min (M15); con systemd < 254, `reset-failed` al abrir la app |
| 20 | El aislamiento del unit impide leer proyectos o escribir la base | Baja | Alto | Solo se usa lo que funciona en `--user` con `PrivateUsers=yes`; sin `ProtectHome`; E16 lo prueba; sin user namespaces, unit sin aislamiento y aviso (3.7) |
| 21 | El arranque en frío pasa `TimeoutStartSec` | Baja | Medio | `READY` antes del recorrido, recorrido en hilo aparte, watchdog atado al bucle (M13) |
| 22 | El reloj retrocede o salta | Baja | Medio | `max(t, ultimo_cambio)`; salto > 5 min cierra y anota en `last_error` (M16) |
| 23 | journald se llena con rutas de archivos (privacidad y ruido) | Media | Bajo | Nunca rutas por evento; contadores agregados y límite de tasa (O2) |
| 24 | El usuario no se entera de que el tracker está apagado o roto | Baja | Medio | Indicador siempre visible con 6 estados (3.9) |
| 25 | Carpetas compartidas cuentan para dos proyectos | Alta | Bajo | Es intencional (3.4) y está avisado en la tarjeta |
| 26 | Downgrade de la app rompe la base o la config | Baja | Crítico | Runbook de abajo; piso de downgrade = primera versión con F2 |

### 7.1 Runbook: apagar el tracker y volver atrás

Para cuando el demonio mide mal o molesta y hay que sacarlo **sin perder la base**.
Todo es local; ningún paso toca la red.

1. **Apagarlo.** Desde Ajustes → Tracking → Desactivar. Si la app no abre:
   ```bash
   systemctl --user disable --now gestor-trackerd   # cierra sus sesiones (SIGTERM)
   systemctl --user is-enabled gestor-trackerd       # → disabled
   ```
   Con la app abierta después, **no** se reactiva (3.8 paso 6).
2. **Backup antes de tocar datos.** Ajustes → Backup, o con todo cerrado:
   ```bash
   sqlite3 ~/.local/share/gestor-proyectos/projects.db \
     "VACUUM INTO '$HOME/gestor-antes-de-rollback.db'"
   ```
3. **Decidir qué hacer con lo medido.** Las versiones anteriores suman
   `duration_seconds` de **todas** las sesiones, así que las `file_watch` que se
   superponen con "Trabajar" se contarían doble. Dos opciones:

   | Opción | Comando | Costo |
   |---|---|---|
   | Conservarlas | Nada | Totales inflados donde hubo superposición |
   | Borrarlas | `sqlite3 …/projects.db "DELETE FROM time_tracking_sessions WHERE source = 'file_watch'"` | Se pierde lo que midió el demonio (queda en el backup del paso 2) |

4. **Sacar el servicio:** `./uninstall.sh` (borra unit y binarios, conserva la
   base), o a mano: `rm ~/.config/systemd/user/gestor-trackerd.service
   ~/.local/bin/gestor-trackerd && systemctl --user daemon-reload`.
5. **Instalar la versión anterior**, respetando el piso de abajo.

### 7.2 Downgrade

- **Piso: la primera versión que incluye F2** (se anota en el CHANGELOG como
  "piso de downgrade"). Las anteriores restauran con `fs::rename`
  (`backup/mod.rs:427-499`): con la base en WAL, un `-wal` sobrante se aplica
  sobre el archivo restaurado y lo **corrompe** (P6).
- **Bajar por debajo del piso** solo con la base de vuelta en modo `DELETE`, con
  la app y el demonio cerrados:
  ```bash
  systemctl --user disable --now gestor-trackerd
  sqlite3 ~/.local/share/gestor-proyectos/projects.db "PRAGMA journal_mode=DELETE"   # → delete
  eza ~/.local/share/gestor-proyectos/  # no debe quedar projects.db-wal ni -shm
  ```
  Ojo: la app nueva vuelve a poner WAL al abrir; esto es solo para instalar la vieja.
- **Columnas y tablas nuevas** (`last_activity_at`, `tracker_status`) quedan en la
  base: las versiones viejas no las leen y no molestan. `user_version` tampoco: las
  migraciones viejas lo ignoran (`let _ = conn.execute("ALTER …")`).
- **Config:** una versión vieja **descarta** la sección `tracking` la primera vez
  que guarda (D10): `AppConfig` no tiene `deny_unknown_fields`
  (`config/schema.rs:7`), así que la ignora al leer, y `save_to_file` reescribe el
  struct entero (`config/manager.rs:140-153`). Al volver a la versión nueva,
  `daemon_enabled` vale el default `true` y hay dos casos (3.8, paso 6):
  - **sin unit** (se sacó con `uninstall.sh`, paso 4 del runbook): la app lo
    instala y lo arranca;
  - **con el unit presente y `disabled`**: la app no lo reactiva; se enciende desde
    Ajustes.

  En los dos casos conviene revisar Ajustes → Tracking, sobre todo si estaba
  desactivado a propósito.
- **Demonio sin app:** si se baja solo la app y queda un `gestor-trackerd` nuevo,
  ve un `user_version` igual o menor y sigue midiendo contra una app que no sabe
  mostrarlo. Por eso el paso 4 del runbook va **antes** de instalar la versión vieja.
## 8. Qué NO hacer

**Medición**
- **No** usar `RecursiveMode::Recursive` ni `notify-debouncer-mini`.
- **No** acumular contadores: todo total sale de los tramos.
- **No** escribir fechas en otro formato que el de `time.rs`.
- **No** usar `Instant` para la inactividad del demonio, ni `Date.now()` para el contador de "Trabajar" (M5).
- **No** acreditar tiempo por eventos perdidos ni re-recorrer todo en plena ráfaga tras un desborde: el rescate espera calma y solo agrega lo que falta (M10).
- **No** acreditar más que `min_credit_seconds` por un guardado, ni cruzar la sesión anterior o una suspensión con el margen (DX1).
- **No** llamar a `unwatch()` directo: siempre por el registro con conteo de referencias (M2).
- **No** llamar a `matched_path_or_any_parents` con una ruta fuera de la raíz del matcher: entra en pánico.
- **No** rechazar FUSE en bloque: `fuseblk` es un disco local (M11).

**Base y config**
- **No** migrar desde el demonio. Con `user_version` mayor al conocido, no escribir nada (B1).
- **No** usar `ConfigManager` en el demonio ni escribir la config desde él.
- **No** copiar ni reemplazar el `.db` con `fs::copy` o `rename` en modo WAL.
- **No** hacer downgrade por debajo de la primera versión con F2 sin volver antes a `journal_mode=DELETE` (7.2).

**Servicio**
- **No** poner el lock en `$XDG_RUNTIME_DIR` ni depender del entorno heredado: lock en `<data_dir>` y rutas explícitas en `ExecStart` (B8).
- **No** instalar ni reescribir el unit si `current_exe()` no está en `BIN_DIR`, ni dejar que `tauri dev` instale el servicio (B9).
- **No** comparar el unit por una versión en un comentario: se compara el contenido completo (B9).
- **No** escribir en el unit rutas con espacios, `%` o saltos de línea (B9).
- **No** reactivar un unit que el usuario deshabilitó con `systemctl`; solo el botón de Ajustes lo vuelve a encender (B5).
- **No** cambiar el estado de systemd antes de guardar la config: primero config, después `systemctl` (B5).
- **No** usar `ProtectHome=` ni opciones de espacio de nombres sin `PrivateUsers=yes` en el unit de usuario (O1).
- **No** mandar `WATCHDOG=1` desde un hilo que no sea el del bucle (M13).
- **No** usar `cp` sobre un binario en ejecución; temporal en la misma carpeta + `mv`.
- **No** activar lingering por defecto.
- **No** comunicar app y demonio por sockets.
- **No** loguear rutas de archivos por evento (O2).
## 9. Decisiones

### 9.1 Decididas

Las cuatro figuraban abiertas en la v2 aunque el cuerpo ya las daba por hechas
(T10). Llevan prefijo **Q** para no confundirse con los hallazgos D1–D15 del Anexo A.

| # | Pregunta | Decisión | Dónde |
|---|---|---|---|
| Q1 | Minutos de inactividad | **3**, configurable de 1 a 60, acotado por el demonio | R3; 3.4 (M14); 3.5; commits 30, 31, 52 |
| Q2 | Proyectos con carpeta `/home` (30, 31, 43) | No se vigilan (`carpeta_amplia`); la app los marca y el usuario los corrige. Qué hacer con estos tres en particular sigue en 9.2 (DX4) | R7; 3.5; 3.9; commits 32, 53 |
| Q3 | Un guardado aislado | **Reemplazada por DX1.** La v2 decía "no se guarda (0 s)"; queda como el caso `min_credit_seconds = 0` | 3.4 |
| Q4 | "Abrir" del Dashboard | Igual que "Trabajar" | 3.9; commit 9 |

### 9.2 Confirmadas por el usuario (2026-10-18)

El usuario aprobó la **propuesta** de cada fila. Para DX3 y DX4 eso no autoriza a
reescribir datos: el script del commit 55 pide un "sí" explícito caso por caso. Para
DX5, el commit de CI entra (y el total pasa de 56 a 57) recién cuando se autorice
el push a GitHub, porque la CI corre en remoto.

Orden en que se aplican: DX1 y DX2 entran con F4 (motor y atribución); DX3 y DX4,
con el commit 55, antes de cerrar F8.

| # | Pregunta | Opciones | Propuesta |
|---|---|---|---|
| DX1 | **Margen por guardado:** ¿cuánto acredita un guardado antes del primer cambio de una sesión? (definición en 3.4) | (a) **60 s**, con las cotas de 3.4; (b) 0 s, como la v2: un guardado suelto no cuenta y dos guardados a 2 min valen 2 min; (c) otro valor entre 0 y 300 | **(a)**, configurable en Ajustes. Es la única excepción a "nunca inventar tiempo" y está acotada |
| DX2 | **Tope de carpetas por proyecto** (`max_dirs_per_project`). Con **5.000**, Moodle Potenciarg (**55**, ~27.100 carpetas respetando `.gitignore`, 3.5) queda **no medible** | (a) 5.000 global, y el 55 se arregla con un `.gestorignore` que excluya el código de Moodle que no se toca; (b) 30.000: el 55 entra, y un paraguas como `~/2025` probablemente también, usando más del límite de inotify compartido (~52.400 de 524.288 con todos los proyectos, 2.3); (c) 5.000 por defecto más un tope **por proyecto** editable en la tarjeta: cuesta +1–2 commits, una columna nueva en `projects` y un paso de migración; (d) sin tope, solo el aviso de `limite_inotify` | **(a)**: default único de 5.000, y para el 55 se recomienda `.gestorignore`. (c) queda como opción, con su costo declarado, si el `.gestorignore` no alcanza. (d) no: un paraguas solo podría agotar el límite de VSCode y JetBrains |
| DX3 | **Datos históricos:** las sesiones de más de 12 h (el 69 % del tiempo registrado) y las huérfanas abiertas (cifras en 2.1) | Para las > 12 h: (a) **marcar** = listarlas en el CHANGELOG, **sin columna nueva**, y seguir contándolas; (b) **recortar** cada una a un tope (por ejemplo 8 h); (c) **dejarlas** como están. Para las huérfanas: (a) cerrarlas en 0 s (lo hace el commit 15: hoy ya suman 0, el total no cambia); (b) borrarlas | > 12 h: **(a) marcar**, y que el usuario decida caso por caso con la lista del diff (commit 19). Huérfanas: **(a)**, ya incluido en el plan. Lo que reescriba filas, (b) o el borrado, lo aplica el script del commit 55, con backup previo y **solo** con un "sí" explícito del usuario |
| DX4 | **Proyectos con carpeta que no se puede medir:** 7 con carpeta inexistente y 3 con carpeta amplia (lista en 3.5) | (a) dejarlos marcados con el aviso y que el usuario los edite cuando quiera; (b) corregir las rutas ahora (por ejemplo, `/home/2025/...` → `~/2025/...` si es un error de tipeo); (c) mandarlos a la papelera | **(a)**, y revisar juntos si 19–21 son `~/2025` mal escrito: es probable, pero no se toca sin confirmar. (b) y (c), si se eligen, van por el script del commit 55 con las mismas garantías |
| DX5 | **CI en GitHub Actions** | (a) `.github/workflows/check.yml` que corra `scripts/check.sh` sin `--e2e` en `ubuntu-latest`, agregado como commit aparte al final de F2 (los E2E, E16 y E22 quedan locales); (b) no tener CI | **(a)**, con autorización aparte por ser remoto. Si se aprueba, entra como commit 28, se renumera desde ahí y el total pasa de 56 a 57 en todo el documento |

---
## 10. Fuentes

**Vigilancia de archivos**
- `notify` 8.2.0 (`src/inotify.rs`: vigilancia por directorio, mismo descriptor al vigilar dos veces la misma carpeta, `Q_OVERFLOW → Rescan`, `ENOSPC → MaxFilesWatch`): https://docs.rs/notify/8.2.0
- `ignore` 0.4 (`WalkBuilder`, `gitignore::Gitignore::matched_path_or_any_parents`, que entra en pánico con rutas fuera de la raíz): https://docs.rs/ignore/latest/ignore/struct.WalkBuilder.html · https://docs.rs/ignore/latest/ignore/gitignore/struct.Gitignore.html
- inotify(7) (cola, `IN_Q_OVERFLOW`, `max_user_watches`): https://man7.org/linux/man-pages/man7/inotify.7.html
- Costo de una vigilancia de inotify: https://lkml.iu.edu/hypermail/linux/kernel/2010.3/08749.html
- fanotify sin privilegios (por qué no sirve): https://man7.org/linux/man-pages/man7/fanotify.7.html
- `/proc/self/mountinfo` (tipo de sistema de archivos por punto de montaje): https://man7.org/linux/man-pages/man5/proc_pid_mountinfo.5.html
- `clock_gettime(2)` (`CLOCK_MONOTONIC` contra `CLOCK_BOOTTIME`, detección de suspensión): https://man7.org/linux/man-pages/man2/clock_gettime.2.html

**SQLite y rusqlite**
- SQLite: https://sqlite.org/wal.html · https://sqlite.org/backup.html · https://sqlite.org/howtocorrupt.html · https://sqlite.org/lang_vacuum.html#vacuuminto
- `PRAGMA user_version` y `PRAGMA data_version`: https://sqlite.org/pragma.html#pragma_user_version · https://sqlite.org/pragma.html#pragma_data_version
- `BEGIN IMMEDIATE` y `SQLITE_BUSY_SNAPSHOT`: https://sqlite.org/lang_transaction.html · https://sqlite.org/rescode.html#busy_snapshot
- Permisos de `-wal`/`-shm` (heredan los del archivo de la base): https://sqlite.org/tempfiles.html
- rusqlite 0.32 (`pragma_update_and_check`, `TransactionBehavior::Immediate`, feature `backup`, `OpenFlags`): https://docs.rs/rusqlite/0.32.1

**Fechas**
- `chrono` 0.4 (`LocalResult`, `.earliest()`): https://docs.rs/chrono/0.4
- `chrono-tz` (zonas para los tests, solo `[dev-dependencies]`): https://docs.rs/chrono-tz/latest/chrono_tz/
- Cambios de horario a medianoche (Santiago, Havana): https://www.iana.org/time-zones

**systemd**
- `systemd.service(5)` (`Type=notify`, `Restart=`, `RestartSteps=`, `RestartMaxDelaySec=`, `WatchdogSec=`): https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html
- `systemd.unit(5)` (`StartLimitIntervalSec=`, especificadores `%`, separación de argumentos): https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html
- `systemd.exec(5)` (`PrivateUsers=`, `ProtectSystem=`, `ReadWritePaths=`, `PrivateTmp=`, `UMask=`, `IOSchedulingClass=`, opciones de espacio de nombres en el administrador de usuario): https://www.freedesktop.org/software/systemd/man/latest/systemd.exec.html
- `systemd.resource-control(5)` (`MemoryHigh=`, `MemoryMax=`, `TasksMax=`, delegación): https://www.freedesktop.org/software/systemd/man/latest/systemd.resource-control.html
- `sd_notify(3)` (`READY`, `WATCHDOG`, `STOPPING`, `EXTEND_TIMEOUT_USEC`, `STATUS`): https://www.freedesktop.org/software/systemd/man/latest/sd_notify.html
- `systemd-run(1)` (unidades transitorias, para E16): https://www.freedesktop.org/software/systemd/man/latest/systemd-run.html
- `systemd-analyze security`: https://www.freedesktop.org/software/systemd/man/latest/systemd-analyze.html
- `sd-notify` 0.5: https://docs.rs/sd-notify/0.5.0
- `signal-hook`: https://docs.rs/signal-hook/latest/signal_hook/

**Herramientas y empaquetado**
- `pkill(1)` (`-f` compara la línea de comandos completa; el nombre del proceso se trunca a 15 caracteres): https://man7.org/linux/man-pages/man1/pkill.1.html
- Cargo `default-run`: https://doc.rust-lang.org/cargo/reference/manifest.html#the-default-run-field
- Bundler de Tauri 2 (qué binarios entran en el `.deb`, a verificar en el commit 29): https://v2.tauri.app/distribute/debian/

**Repo** (relevado sobre `main@c8b55ae`): `scripts/release.sh:2` (`set -euo pipefail`), `:302-304` (`cargo build --release` + `cp`); `install.sh:6` (`set -e`), `:14-17` (exige el binario ya compilado), `:27` (`cp`), `:46` (`Exec=` fijo); `uninstall.sh:37-41` (conserva la base); `start-app.sh:21` (`pkill -f "gestor-proyectos"`); `src-tauri/Cargo.toml:19` (rusqlite solo con `bundled`), `:31-32` (`[dev-dependencies]` con `tempfile`); `src-tauri/tauri.conf.json:7` (`beforeBuildCommand`), `:57-59` (`bundle`); no existe `.github/`.

---
## 11. Historial de revisión

- **v2:** la v1 pasó una revisión adversarial con 25 hallazgos (8 críticos), todos incorporados.
- **v3:** la v2 pasó una auditoría con 59 hallazgos (9 bloqueantes), todos con fila y ubicación.
- **v3.1:** la v3 se verificó contra el código y la documentación de systemd y Tauri 2; 3 hallazgos altos (kill switch antes de `READY`, E2E en verde vacío, restore inaccesible desde los tests), 16 medios y 10 bajos corregidos, más un recorte de redundancias.

El detalle de cada ronda, con dónde quedó cada hallazgo, está en el **Anexo A**.

---
## Anexo A — Historial de revisión

### v2 (2026-10-18): revisión adversarial de la v1

Se incorporan los 25 hallazgos. Los números de commit de esta tabla son los de la
v2; la correspondencia con la v3 está en la columna "Hoy en v3".

| Sev. | Hallazgo | Dónde se resolvió (v2) | Hoy en v3 |
|---|---|---|---|
| Crítico | La unión contaba huérfanas viejas "hasta ahora" | 3.3 (tabla de tramos); F1 ordena la recuperación antes de la unión | 3.3; commits 15 → 16 |
| Crítico | "Mismos números que antes" era falso (regla de 10 s, suspensión) | 3.3: tramo = `started_at + duration_seconds` para filas no `file_watch` | 3.2 (regla única, M8); commits 8, 16 |
| Crítico | El corte dependía del `tick` y una suspensión extendía la sesión | 3.4: corte también en `cambio`; reloj de pared con protección contra retrocesos | 3.4 (+ `reanudado`); commit 31 |
| Crítico | La recuperación al arrancar corría también en la segunda instancia | 3.8: va en `.setup()` y solo toca latidos de más de 2 min (P11) | Commits 4 (D15) y 15 |
| Crítico | El demonio con `ConfigManager` podía borrar la config | 3.5: lector propio de solo lectura (P12) | Commit 38 |
| Crítico | WAL antes del restore nuevo dejaba un commit que corrompía | F2: restore primero (commit 10) | Commits 22 → 25 |
| Crítico | Verificar `"wal"` rompía los tests `:memory:` | 3.6: se acepta `"memory"` | Commit 25 |
| Crítico | El restore ignoraba la conexión viva, los esquemas viejos y los ids | 3.6: secuencia con salida de la app y migración al volver a abrir | 3.6 (migra **antes** de relanzar, B7); commits 22, 47 |
| Alto | Solo las transacciones de tracking pasaban a `IMMEDIATE` | 3.6: las 6 (P13) | Commit 23 (justificación corregida, D9) |
| Alto | Las actualizaciones fallaban (`ETXTBSY`, sin reinicio, límite de reintentos) | 3.8: temporal + `mv`, `try-restart`, `reset-failed` | Commit 48; M15 en commit 44 |
| Alto | `ExecStart` asumía `~/.local/bin` | 3.7: ruta hermana de `current_exe`; alcance declarado | Reemplazado por B9: solo instala desde `BIN_DIR`; commit 44 |
| Alto | Formato de fechas sin definir | 3.3: `time.rs` único, consulta de rango y `.earliest()` | Commit 11 (medianoche inexistente corregida, D13) |
| Alto | Carpetas nuevas, renombradas, movidas o borradas | 3.5 + E6/E7 | Commit 36; E6, E7 |
| Alto | El desborde re-recorría 52.000 directorios | 3.5: solo se registra | 3.5: rescate diferido (M10); commit 37 |
| Alto | E2E inexpresables o inestables | 3.5 `GESTOR_TRACKERD_TEST_TIMINGS`; 5.2 sondeo con plazo | 5.2 (+ centinelas, T3); commits 41, 42 |
| Medio | `add_project_time` también escribía el contador | 3.2 + commit 9 | Commit 20 (D2) |
| Medio | El segundero del cliente pasaba del último cambio | 3.9: `file_watch` muestra `last_activity_at − started_at` | Commit 50 |
| Medio | Filas de 0 s y estadísticas sin definir | 3.2 y 3.3 | 3.2 (DX1), 3.3; commits 16, 17 |
| Medio | Papelera y purga a mitad de sesión | 3.4 (errores no fatales) y 3.5 (papelera) | 3.4 (0 filas, M12); commits 35, 39; E11 |
| Medio | `VACUUM INTO` bajo WAL dejaba `-wal`/`-shm` | 3.6 + commit 13 | Commit 26 |
| Medio | Atribución por prefijo de texto | 3.4: `Path::starts_with` por componentes | Commit 32 (más específico, DX2) |
| Medio | Orden de arranque del demonio sin definir | 3.4 | 3.4 (READY antes del recorrido, M13); commit 40 |
| Bajo | Reexportaciones y nombre de la lib | 3.1 + commit 15 | Commit 28 |
| Bajo | Detección de producción y errores de `systemctl` | 3.8 | Commit 44 |
| Bajo | F0 y F1 reescribían lo mismo | "Hoy" pasa a F1 (commit 9) | Commit 17 |
| Bajo | `active_today` en UTC | F1 (commit 9) y `tracker_status.idle_minutes` | Commit 17 (hora local, con tiempo trabajado) |

### v3 (2026-10-18): auditoría de la v2

59 hallazgos (`HALLAZGOS.md` de la auditoría): 2 decisiones por defecto, 9
bloqueantes, 16 de medición, 15 de datos, 5 de operación y 12 de pruebas y entrega.
Todos tienen fila; los que se resolvieron distinto de lo pedido lo dicen.

**Decisiones tomadas por defecto (confirmadas el 2026-10-18, 9.2):**

| ID | Dónde quedó | Resumen |
|---|---|---|
| DX1 | 3 (Principios, 4); 3.2 "Guardados espaciados" (referencia); 3.4 margen y `frontera` (definición única); 3.5 rango; 3.9; 9.2; commits 30, 31, 35, 38, 52; E1b | 60 s antes del primer cambio, acotado por la sesión anterior, la suspensión y el corte; 0 = v2. Reemplaza la Q3 vieja (9.1) |
| DX2 | 3.4 atribución al más específico; 3.5 tope `max_dirs_per_project`; 3.9 avisos; 9.2 con el dato del proyecto 55; commits 32, 33, 37, 52, 53; E19 | Paraguas vigilados con tope (default 5.000) y conteo de referencias. Con 5.000 el 55 (~27.100 carpetas) no se mide: opciones en 9.2 |

**Bloqueantes:**

| ID | Dónde quedó | Resumen |
|---|---|---|
| B1 | 2.2 P14; 3 (Principios, 5); 3.2 "Esquema versionado" (tabla única de estados); 3.4 arranque; commits 10, 35, 40; E15 | `user_version` + `BEGIN IMMEDIATE`; solo se ignora `duplicate column name`; el demonio no migra: espera si está atrasada o sin WAL, ocioso si es futura. La v3 no probaba la espera: la v3.1 agrega en el commit 35 "`ESQUEMA_ACTUAL − 1` → `esperando_migracion`, y `activo` al migrar desde otra conexión" |
| B2 | 2.2 P15; 3.9; commit 18 (F1, no F7) | `TimeTracker` carga stats siempre |
| B3 | 2.2 P2; 3.6; commit 3 | 0 filas en el primer `UPDATE` → rollback, sin sumar al contador |
| B4 | 3.1 `default-run` y empaquetado; 3.7; 3.8; §1 "Fuera de alcance"; commit 29 | `default-run` en el mismo commit del `[[bin]]`; tarea `dpkg -c`; el diseño no depende del resultado |
| B5 | 3.4 kill switch (sale 0); 3.8 (config primero, no reactiva un `disable` manual); 3.9; 7.1 runbook; 7.2; 8; F6 entera junta; commits 31, 40, 44, 45, 51; E20; manual 12–13 | Nunca en producción sin forma de apagarlo; toggle mínimo en F6 |
| B6 | 4 (orden y 4.10); 5.5; commits 1 y 2 (a), 13 (b), 30 (c), 41 y 42 (d) | `check.sh` y `cargo fmt` en F0; `tracker_status` en F1; config `tracking` en F3; E2E desde F5b. Total recontado: 55 en la v3, 56 en la v3.1 (commit 55 nuevo, DX3/DX4) |
| B7 | 3.6 restore pasos 1–9; 3.8; commits 22, 47; E15; manual 15 | La app migra lo restaurado con el lock tomado y hace `start` antes de `app.exit(0)` |
| B8 | 3.1 `trackerd_lock_file`; 3.4 guardas; 3.7 rutas explícitas; 5.2 aislamiento; commit 29; E14 | Lock en `<data_dir>/trackerd.lock`; debug exige `--data-dir`; timings + base real → se niega |
| B9 | 3.7; 3.8 pasos 3–5; 8; commit 44 | Unit comparado por contenido completo; instala solo desde `BIN_DIR`; rutas con caracteres especiales rechazadas |

**Medición:**

| ID | Dónde quedó | Resumen |
|---|---|---|
| M1 | = DX1 | Ver DX1 |
| M2 | 3.5 registro con conteo; 8; commit 36; E19 | `watch`/`unwatch` solo en transiciones 0↔1 |
| M3 | 3.5 filtro por evento; 7 riesgo 1; commits 34, 37; E5, E18 | Matchers de `.gitignore` en cada evento, con recarga en caliente |
| M4 | 3.5 ignorados y carpetas sincronizadas; 3.9; commits 33, 34, 53 | Syncthing, Dropbox, Nextcloud, `.~lock.*#`, `*_BACKUP.md`; proyectos sincronizados avisados |
| M5 | 3.3 fila `activa`; 3.9 contador; 8; commits 16, 49, 50 | Contador de "Trabajar" con `elapsed_seconds` del `Instant`, nunca `Date.now()` |
| M6 | 3.3 firma con `activa`, tramos sin `tracker_status`; commits 16, 17 | Orden de locks `ActiveSession → Database` |
| M7 | 3.2 "Cómo se cierra cada sesión"; 3.4 cierres coherentes; commits 14, 15, 31, 35; E8 | `ended_at` y `duration_seconds` coherentes; `last_activity_at` al abrir |
| M8 | 2.1; 3.2 regla única; commit 8 | Una sola `duracion_a_guardar` para todos los cierres |
| M9 | 3.4 latido de sesión; 3.9 texto honesto; commits 35, 50 | Máx. una escritura cada 5 s con actividad, ninguna sin actividad |
| M10 | 3.5 desborde; 7 riesgo 5; 8; commit 37 | Rescate diferido tras 30 s de calma, máx. uno cada 10 min |
| M11 | 3.5 `mountinfo` y `en_red`; 8; commits 33, 37 | Resuelto distinto: solo se rechaza lo remoto; `fuseblk` local (14 proyectos) se mide |
| M12 | 3.4 0 filas → soltar; 3.5 `data_version`; commits 31, 35, 39; E11, E21 | Papelera vista en ≤ 5 s |
| M13 | 3.4 arranque y watchdog; 3.7; 8; commit 40 | `READY` antes del recorrido; watchdog solo desde el bucle; unitario del watchdog (5.2) |
| M14 | 3.4 inactividad efectiva; 3.5 rangos; commits 31, 38, 52; E17 | `idle_minutes` acotado 1–60 y publicado en caliente |
| M15 | 3.7 `StartLimitIntervalSec=0` + `RestartSteps`; 3.8; commit 44 | Nunca se rinde; alternativa para systemd < 254 con `reset-failed` al abrir |
| M16 | 3.4 reloj; commit 31 | Retroceso > 5 min cierra y anota `last_error`; solo UTC |

**Datos y base:**

| ID | Dónde quedó | Resumen |
|---|---|---|
| D1 | 2.2 P17; 3.1 `paths.rs`; commit 21 | Rutas compartidas; se borra `backup::live_db_path` |
| D2 | 3.2 lista con archivo:línea; commits 17 (9a) y 20 (9b) | Lectura por unión primero, sacar el contador después |
| D3 | 2.1 "La base real hoy"; 5.4; 6; 9.2 DX3; commits 19, 55, 56 | Diff con datos reales sobre copia; > 12 h listadas sin borrar; cada diferencia en el CHANGELOG |
| D4 | 2.2 P3; 3.3 semana; commits 11, 17 | 7 fechas locales incluido hoy |
| D5 | 3.3 excepción `deleted_at`; commit 11 | Excluido de `time.rs` y documentado |
| D6 | 2.2 P18; 3.3; commit 17 | Errores propagados en vez de `.unwrap_or(0)` |
| D7 | 2.2 P16; 3.6; commit 5 | `self.conn()` bloqueante |
| D8 | 2.3; 3.6 WAL y `busy_timeout`; commit 25 | Si no queda en `wal`, error visible |
| D9 | 2.2 P13 corregido; 3.6; commit 23 | Justificación corregida; `IMMEDIATE` en las 6 como regla |
| D10 | 7.2 (3.6 la referencia); commits 27, 56 | La versión vieja descarta `tracking` al guardar: documentado |
| D11 | 2.2 P19; 3.1; 3.6; 3.7 `UMask=0077`; commits 24, 25, 44 | `0700`/`0600` para carpeta, base, `-wal`/`-shm` y backups |
| D12 | 3.6 "Backups del home"; 3.9 Ajustes → Backups; commits 27 (`ARQUITECTURA.md`) y 52 (texto en Ajustes, con Vitest) | Excluir `projects.db*` de rsync/Déjà Dup; respaldar la carpeta de backups. La v3 citaba Ajustes sin commit que lo hiciera |
| D13 | 3.3 tests de zona; 5.1; commit 11 | `rango_dia` genérico, `chrono-tz` dev-dep, sin tocar `TZ` |
| D14 | 3.2 métodos con instante explícito; commit 14 | `abrir_sesion`, `latir`, `cerrar_sesion` con `bool` |
| D15 | 2.2 P11; 3.8; commit 4 (F0); manual 1 | Base, config y auto-backup en `.setup()` |

**Operación y seguridad:**

| ID | Dónde quedó | Resumen |
|---|---|---|
| O1 | 3.7 aislamiento con `PrivateUsers=yes` y tabla de qué deja leer/escribir; 3.8 paso 8; 8; commits 44, 46; E16; manual 18 | Resuelto: sin `ProtectHome`; fallback sin user namespaces con criterio de detección concreto (`Result=exit-code` + `ExecMainStatus=226`) y test con systemctl falso en el commit 44 |
| O2 | 3.7 logs; 8; commit 39 | Sin rutas por evento; línea agregada cada 10 min; límite de tasa |
| O3 | 3.5 tope; 3.7 `MemoryMax`; 3.9 uso de inotify; 6 `MemoryCurrent`; commits 37, 52, 54 | Uso visible en Ajustes y medido en "listo" |
| O4 | 3.8 `uninstall.sh`; commit 48 | Resuelto distinto: **no** se borra `tracker_status` (la base se conserva a propósito y la fila vieja es inofensiva) |
| O5 | 3.7 `UNIT_DIR`; 3.8 variables de los scripts; 5.2 aislamiento; commits 44, 48 | `PREFIX`/`BIN_DIR`/`UNIT_DIR`/`SYSTEMCTL`; `.desktop` sin ruta fija; `FragmentPath` verificado |

**Pruebas y entrega:**

| ID | Dónde quedó | Resumen |
|---|---|---|
| T1 | 3.6 "Dónde vive cada paso"; 5.2 E15; commits 22, 28, 42 | Restore con el demonio vivo sin systemd: aborta con el lock; luego restore de esquema viejo y relanzamiento. E15 llama a `db::restore` de la lib: la v3 lo hacía llamar a `restore_backup_to`, que es privada de `backup/` y no se puede usar desde `tests/` |
| T2 | 5.2 E16; commit 46 | `systemd-run --user` transitorio con las propiedades del generador |
| T3 | 5.2 "Negativos con centinela"; commit 41 | Esperar `scan_completed_at`, negativo, centinela, afirmar |
| T4 | 5.2 E7, E17–E21; 5.3 paso 16; 6.1 (P3 en commit 17) | Todos los escenarios pedidos tienen test o paso manual. La v3 omitía "mover `A/sub` → `B/sub`": la v3.1 lo suma a E7 |
| T5 | 5.2 tolerancias; "Lo que no se automatiza" | ±1 s; E10 de 20 s con 4 escritores; watchdog cubierto con unitario y documentado |
| T6 | 5.3 (instalación con `install.sh`, pasos 3 y 17); commit 48 | Nada del recorrido crea commits ni tags |
| T7 | 4 F5a/F5b; validación de F2 sobre copia de la base real | F5 partida; F2 probada sobre copia antes del merge |
| T8 | 6 completo | Comandos exactos de recursos, kill switch, E1–E15 y E17–E21 ×3 (con cuenta), E16, E22, diff, matriz |
| T9 | 4.11; encabezado | Días efectivos por fase (20–27 en la v3.1) |
| T10 | 9.1 y 9.2 | Q1–Q4 decididas (Q3 reemplazada por DX1); DX1–DX5 abiertas con ubicación. En la v3 se llamaban D1–D5 y chocaban con los hallazgos D1–D15 |
| T11 | Anexo A, v2 | La fila "Bajo" mezclada se separó en dos |
| T12 | 6.1 | Matriz P1–P19, R1–R7 y B1–B9 → commit → prueba |

### v3.1 (2026-10-18): verificación

La v3 se verificó contra el código de `main@c8b55ae`, el de systemd
(`src/core/service.c`) y el de Tauri 2 (`crates/tauri/src/app.rs`). Una línea por
corrección:

**Altos**

| ID | Qué se corrigió |
|---|---|
| A1 | Kill switch al arrancar: salir con 0 antes de `READY=1` deja la unidad `failed` (`Result=protocol`) y, con `Restart=on-failure` + `StartLimitIntervalSec=0`, en bucle. Ahora la config se lee antes de la base y el demonio manda `READY=1` → `STOPPING=1` → salida 0 (3.4, 3.7); tests en el commit 40 y E22 nuevo (commit 46) |
| A2 | E2E en verde vacío: todos los tests de `tests/e2e_trackerd.rs` llevan `#[ignore]` y `check.sh --e2e` falla si corren menos de `E2E_ESPERADOS` (5.5, commits 41 y 42) |
| A3 | E15 no podía llamar a `restore_backup_to` (privada, `backup/mod.rs:427`): el núcleo pasa a `db::restore::{tomar_lock, restaurar_desde}` en el commit 22 y entra en la lib en el 28; `backup/` queda como envoltorio (3.1, 3.6, 4.10, E15) |

**Medios y parciales**

| ID | Qué se corrigió |
|---|---|
| B1 | Test "`ESQUEMA_ACTUAL − 1` → `esperando_migracion`, y `activo` al migrar desde otra conexión" en el commit 35; matriz 6.1 y fila B1 del Anexo A corregidas |
| T4 | E7 suma "mover `A/sub` → `B/sub` y escribir → cuenta solo B; A suelta sus vigilancias"; fila T4 corregida |
| O1 | Criterio de detección sin user namespaces: `Result=exit-code` + `ExecMainStatus=226` (3.7, 3.8 paso 8); test con systemctl falso en el commit 44; fila O1 pasa a "Resuelto" |
| O2 | Unitario del limitador de logs en el commit 39; E2 y E5 afirman que el stderr no contiene la ruta del `tempdir` |
| D3/DX3 | "Marcar" = lista en el CHANGELOG, sin columna (9.2); commit 55 nuevo (`scripts/datos-historicos.sh`, backup previo y solo con "sí" explícito); 5.4 dice "cuatro partes"; total 55 → 56 |
| D12 | El texto de Ajustes → Backups entra en el commit 52, con Vitest (3.6, 3.9) |
| M13 | El commit 40 lista en su columna de pruebas el unitario "`WATCHDOG=1` solo desde el bucle" |
| Orden de arranque | La config se lee antes de abrir la base (paso 4 de 3.4) y el kill switch se respeta mientras espera; `journal_mode ≠ wal` también hace esperar, con test en el commit 35 |
| 7.2 contra 3.8 | Al volver a la versión nueva hay dos ramas: sin unit (tras `uninstall.sh`) se instala; con el unit `disabled`, no se reactiva |
| `install.sh:40` | Verificado: el `.desktop` usa `<< 'EOF'`; el commit 48 lo pasa a `<< EOF` para expandir `$BIN_DIR`, con test |
| `trash.rs` | Verificado: `restore_project` (`:56-66`) y `purge_project` (`:133-143`) validan la guarda fuera de la transacción; el commit 23 la mete adentro de la `IMMEDIATE` (2.2 P13, 3.6) |
| Retraso del contador | ≤ 5 s de volcado + ≤ 5 s de sondeo ≈ ≤ 10 s, en 3.4, 3.9 (fila y tooltip) y paso manual 7 |
| Commit 4 | Verificado en Tauri 2.8.5: `build()` inicializa los plugins y `run()` crea las ventanas **antes** del closure de `.setup()`. `app.manage(db)` va síncrono entre `build()` y `run()`, y el trabajo lento en un hilo (3.8) |
| DX2 | Un solo default: 5.000, con `.gestorignore` para el 55; el tope por proyecto queda como opción con costo declarado (+1–2 commits, columna, migración) |
| Plazos | El encabezado se alinea con 9.2: F0–F3 se pueden empezar; DX1/DX2 antes de mergear F4; DX3/DX4 antes de F8 |
| IDs | Las decisiones de 9.1 pasan a Q1–Q4 y la de CI a DX5; los hallazgos siguen siendo D1–D15 |

**Bajos**

| Ítem | Qué se corrigió |
|---|---|
| E16 | "Las mismas propiedades salvo `WatchdogSec=5`" (3.7 y 5.2) |
| 7.2 | `stop` → `disable --now` antes de bajar a `journal_mode=DELETE` |
| Referencias | "3.0 principio N" → "3 (Principios, N)"; "runbook en 7/8" → 7.1/7.2 |
| Carpetas | Una sola cifra: ~52.400 (2.3), también en 3.7 y 9.2 |
| Commit 29 | El test sostiene el `flock` y el binario debe salir con 75 |
| `start-app.sh` | Verificado: también `pgrep -f "gestor-proyectos"` (`:19`) alcanza al demonio; el commit 43 acota los dos |
| Regex | El patrón de `pkill` sale de la tabla a un bloque de código, sin `\|` |
| Dependencias | Commits 32 y 34: "—" → 28 |
| Restore | La API de backup conserva el modo WAL del destino; el re-fijado queda como defensivo y así se dice (3.6) |
| Anexo A | Filas B1, T4, D12, O1 y T1 corregidas (decían cosas que la v3 no cumplía) |

**Recorte de redundancias**

| Tema | Dónde queda |
|---|---|
| Columnas de `tracker_status` | Solo en 3.2; 3.4 referencia |
| Estados de `user_version` | Tabla única en 3.2 (con `journal_mode`); 3.4 referencia |
| Margen DX1 | Definición única en 3.4; 3.2 y 9.2 referencian |
| Restore | Completo en 3.6; 3.8 lo resume en tres líneas |
| Tests de zona horaria | Regla en 3.3; tabla y precondiciones en 5.1 |
| Downgrade | Solo en 7.2 (con el detalle de D10); 3.6 referencia |
| Números de la base real | En 2.1; §1 y DX3 referencian |
| Lock | Justificación en 3.4; el resto referencia |
| Historial | §11 queda como resumen; el detalle pasa a este anexo |
