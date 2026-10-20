# Diseño: `gestor-trackerd`, el demonio de medición de tiempo

> **Estado:** aprobado (v3.2). F0 está hecha (commits 1–9, rama `feat/tracker-f0`
> en `83c48d4`); `83c48d4` (refresco de la tarjeta) queda **fuera** de la
> numeración. De F1 en adelante no hay nada implementado: el commit 10 (migraciones
> versionadas) **no** está en el código (`rg user_version src-tauri/src` no
> encuentra nada). Las decisiones DX1–DX5 de 9.2 están confirmadas desde el
> 2026-10-18; las de R8 (D-R8-1..4, R8-CLI, 1a–5a y U1–U7), desde el 2026-10-19 y el
> 2026-10-20 (§1.1 y 9.3). DX3/DX4 siguen pidiendo un "sí" caso por caso antes de
> reescribir filas (commit 72). DX5 (CI) se agrega cuando se autorice el push a GitHub.
> **Fecha:** 2026-10-20 · **Base:** `main@c8b55ae` (B14 ya mergeado) para las
> secciones 2–9 heredadas de la v3.1; `feat/tracker-f0@83c48d4` para las líneas de
> código que cita R8 (3.10, 5.6), que corren distinto.
> **Alcance:** solo Linux (systemd). Instalación soportada: `install.sh` /
> `scripts/release.sh --install`. **Ajustes manuales del tiempo: sí**, solo de
> agregado y con nota (3.10.17); editar o borrar tramos medidos, no. El concepto de
> **cliente** queda fuera (§1).
> **Plan:** 10 fases (F0–F8, con F5 partida en F5a y F5b), **73 commits** `checkpoint:`
> (74 con DX5) más el `release:` que genera el script. Estimación: **41,5–52,5 días
> efectivos** (40,5–50,5 desde F1, porque F0 está hecha; 4.11).
> **Numeración:** todo número de commit de este documento es de la v3.2 (sección 4).
> Las equivalencias con la v3.1 están en el Anexo A, v3.2.
> **Revisión:** la v1 pasó una revisión adversarial con 25 hallazgos (8 críticos),
> incorporados en la v2. La v2 pasó una auditoría con **59 hallazgos** (9
> bloqueantes), incorporados en la v3; la v3.1 aplica la verificación de la v3
> contra el código. La v3.2 integra el requisito **R8** (la sesión de "Trabajar"
> atada a la terminal) y **R8-CLI**, después de cuatro auditorías del plan R8 y de
> las decisiones del usuario. Resumen en la sección 11 y detalle en el Anexo A.
>
> **Historial:** v3.1 (2026-10-18) → **v3.2 (2026-10-20):** requisito R8 (sesión
> atada a la terminal) y R8-CLI; decisiones D-R8-1..4, 1a–5a y U1–U7 del
> 2026-10-20; 17 commits nuevos (56 → 73); alcance: los ajustes manuales auditados
> entran, el cliente queda fuera.

Un programa aparte, `gestor-trackerd`, corre en segundo plano como servicio de
systemd del usuario. Mira los archivos de cada proyecto y registra **en vivo** el
tiempo en que realmente se trabajó en cada uno, esté la app abierta o cerrada. La
app pasa a ser la que **muestra** ese tiempo: contadores por proyecto que avanzan
con cada guardado y un indicador del estado del demonio. El botón "Trabajar" sigue
existiendo, y su tiempo se combina con el del demonio sin contar doble. El demonio
se puede apagar desde la app desde el primer día en que se instala.

Desde la v3.2, "Trabajar" abre una terminal **marcada** y la sesión dura lo que esa
terminal (o las que se abran desde ella) esté abierta, con la app abierta o cerrada:
termina al cerrar la última terminal o con Parar. Se pausa sola tras 10 min sin
actividad y se cobra hasta la última actividad. La CLI `gestor` mide igual desde la
terminal donde se corre. Cada minuto cobrado se puede explicar en un **desglose**, y
el tiempo se corrige con **ajustes** que nunca borran lo medido (3.10).

---
## Camino rápido

| Fase | Commits | Qué entrega | Por qué va ahí |
|---|---|---|---|
| **F0** | 1–9 (hechos) | Puerta de calidad (`cargo fmt` + `scripts/check.sh`) y arreglos que el demonio hereda: cierre idempotente, base abierta **después** de la instancia única, sin `try_lock`, errores visibles, botón Parar, regla única de duración mínima, Dashboard igual a "Trabajar" | Son bugs de hoy que con dos procesos se vuelven graves; sin `check.sh` ningún commit posterior tiene puerta |
| **F1** | 10–25 | Esquema versionado (`user_version`, pasos 1–5), `last_activity_at`, `tracker_status`, **sesiones de Trabajar en la base** (`trabajar_sesiones`, anclas, evidencia), terminal marcada sin zombis, Trabajar y Parar que leen la base, papelera que corta sesiones, recuperación de huérfanas con tope de 12 h, **unión de tramos** en hora local con simultaneidad, **ajustes manuales**, tiempo visible sin `.gestor/`, diff con datos reales, fin del contador | Sin esto, demonio + "Trabajar" cuentan doble, ninguna columna nueva entra sin versión, y la sesión no sobrevive a cerrar la app |
| **F2** | 26–32 | Rutas compartidas, restore seguro (con `restore.lock` y corte de sesiones vivas), transacciones `IMMEDIATE`, permisos `0600`, después WAL | Sin esto, app y demonio se bloquean o el restore corrompe la base. Se valida sobre una **copia de la base real** |
| **F3** | 33–35 | `lib.rs`, binario `gestor-trackerd` con lock y guardas, sección `tracking` de la config | El demonio y la CLI reutilizan `db/` sin duplicarlo; la config existe antes de que alguien la lea |
| **F4** | 36–44 | Motor del demonio: lógica pura, reloj inyectado, atribución, filtros, escritor; lector de `/proc`, presencia del escritorio, **motor y escritor de las sesiones de terminal** | Lo difícil se prueba primero, sin disco real |
| **F5a** | 45–47 | Vigilante (`notify` + `ignore`, conteo de referencias, desborde, tope) y lector de config | La parte que toca el disco, todavía sin bucle |
| **F5b** | 48–52 | Bucle (con terminales y compuerta de presencia), `sd-notify`, señales, kill switch del lado del demonio, y los **E2E** (también los de terminal) | El demonio corre a mano de punta a punta; desde acá cada fase pasa los E2E |
| **F6** | 53–63 | Instalación desde la app y los scripts, **toggle e indicador mínimos**, E16 y E22 con `systemd-run`, restore con systemd, Trabajar elige el demonio, **CLI versionada que mide** y ajustes desde la CLI | "Se levanta la primera vez y queda corriendo", y nunca sin forma de apagarlo |
| **F7** | 64–70 | Contadores en vivo, indicador de 6 estados, Ajustes completos, avisos por carpeta, indicador por estado de la sesión, **desglose** y UI de ajustes | Lo que el usuario ve |
| **F8** | 71–73 | Medición de recursos, script de datos históricos (DX3/DX4), corrida triple de E2E, matriz por terminal y recorrido manual, documentación y release `minor` | Definición de "listo" (sección 6) |

Al terminar cada fase, `main` compila, `scripts/check.sh` pasa, la app se puede
usar como antes (o mejor) y la fase se mergea sola con fast-forward. F0–F2 ya
mejoran el tracking actual antes de que exista el demonio. Hasta F6 el demonio no
se instala en ningún lado: se corre a mano, con una base temporal. Mientras tanto,
Trabajar mide con la app (sesiones `dueno = app`, 3.10.13) y, como hasta F6 la
adopción por un demonio es imposible (3.10.7), cerrar la app o Trabajar en otro
proyecto terminan la sesión como hoy: en `main` no aparecen sesiones "Esperando al
tracker" ni avisos de un tracker que todavía no existe.

---
## 1. Requisitos

| # | Requisito | Origen |
|---|---|---|
| R1 | Medición **en vivo y real** por proyecto. Con varios proyectos activos a la vez, cada uno suma **sus propios tramos**, sin repartir. Si un proyecto contiene a otro, el cambio cuenta para el **más específico**; si dos tienen la misma carpeta, para los dos (3.4) | "no no no, quiero medición en vivo, real!!!" |
| R2 | Se mide **actividad en archivos**: guardar, crear, renombrar o borrar dentro de la carpeta del proyecto. Leer no cuenta, ni lo que ignora el `.gitignore`. Cada sesión acredita un margen de hasta **60 s** antes de su primer cambio (DX1, confirmada, 9). Desde la v3.2, un cambio cuenta solo **con el usuario presente** en el escritorio (1a y U1, 3.10.4) | Definición acordada; margen propuesto en la v3; presencia decidida el 2026-10-20 |
| R3 | Corte por inactividad: **3 minutos** sin cambios cierran la sesión (configurable de 1 a 60). La sesión termina en el **último cambio real** | Propuesta aceptada |
| R4 | "Trabajar" **convive** con el demonio. Si se superponen, se cuenta la **unión** de los tramos | Aprobado |
| R5 | **Solo Linux** | Aprobado |
| R6 | El demonio **sigue corriendo con la app cerrada**. Se instala y arranca la primera vez que se abre la app instalada, y desde ahí arranca solo en cada inicio de sesión. Se puede apagar desde Ajustes, y la app no lo vuelve a encender si el usuario lo deshabilitó con `systemctl` (3.8) | "la app cerrada el demonio tiene qué seguir corriendo" |
| R7 | Carpetas que no se pueden medir bien **no se vigilan**: demasiado amplias (`/`, `/home`, `$HOME`), inexistentes, sin permiso, en un disco de red, o con más carpetas que el tope (DX2, confirmada, 9). La app las marca, dice por qué y el usuario las corrige | Propuesta aceptada; motivos ampliados en la v3 (3.5) |
| R8 | Al tocar Trabajar, el demonio sabe qué terminal se abrió y cierra la sesión cuando esa terminal se cierra. Funciona con la app cerrada (3.10) | Pedido del usuario (2026-10-19) |
| R8-CLI | Elegir un proyecto en `gestor` (menú `fzf` o `gestor abrir X`) = Trabajar en la terminal actual. Mide siempre por defecto (`--sin-medir` opcional). `list`, `ayuda` y `crear` no miden. `gestor parar` corta (3.10.16) | Pedido del usuario (2026-10-19, D-R8-5) |

### 1.1 Decisiones del usuario sobre R8

**Qué cambia para el usuario:**

- Tocás **Trabajar** y se abre una terminal. La sesión dura lo que esa terminal (o las que abras desde ella) esté abierta, con la app abierta o cerrada. Cerrar la app **no** la corta.
- La sesión termina cuando cerrás **la última** terminal, o cuando tocás **Parar** (o corrés `gestor parar`).
- Podés trabajar en A y en B a la vez, cada uno con su sesión y su Parar (con el tracker corriendo).
- Si pasan **10 minutos** sin actividad, la sesión se **pausa** y deja de sumar. Vuelve sola cuando retomás. Actividad es escribir en la terminal **dentro de la carpeta del proyecto** (o en un `ssh`), o que cambie un archivo del proyecto **mientras estás en la compu** (hubo teclado o mouse en **cualquier** ventana del escritorio en los últimos 10 min y la pantalla no está bloqueada).
- La misma regla de presencia vale para el **tiempo automático por archivos** (`file_watch`, sin Trabajar). Lo que cambia en disco mientras no estás **no suma**, pero tampoco desaparece: el desglose lo muestra como "solo archivos, sin presencia" con su horario y su duración, y con **"Cargar como ajuste"** lo pasás a tiempo cobrable con una nota.
- Si no hay forma de saber si estás (sin sesión gráfica, D-Bus caído), los cambios de archivos **cuentan** y quedan marcados "sin dato de presencia".
- **Se cobra hasta la última actividad.** Lo que pasa entre tu última tecla o guardado y el cierre de la terminal (o Parar) no se cobra. Las pausas cortas, de menos de 10 min, sí cuentan.
- En la CLI, `gestor` (el menú) y `gestor abrir X` **son Trabajar** en la terminal donde los corrés. `gestor parar` corta. `--sin-medir` hace el `cd` sin medir.
- Cada minuto cobrado se puede explicar. El **desglose** muestra los tramos con su inicio, su fin y **por qué** terminaron, qué sostuvo cada minuto (tipeo, archivo, remoto), lo que **no** se cobró y por qué, y si un tramo fue **simultáneo** con otro proyecto ("simultáneo con B: 1 h 50"). No se reparte nada solo.
- Podés **corregir** con **ajustes** de tiempo (sumar o restar, con nota obligatoria). Un ajuste no se edita ni se borra: se revierte con otro ajuste.
- Si tu terminal no deja detectar el cierre, la tarjeta te avisa, la sesión se **pausa** (no se cierra con 0 s) y se corta con Parar.
- Sin el tracker corriendo, Trabajar mide con la app, **un proyecto a la vez**, hasta Parar y **como mucho 12 h**: sin tracker no hay pausa por inactividad ni se descuenta la cola. Mientras la app mide, la cabecera dice "Trabajar mide sin vigilancia: no se pausa por inactividad y se corta a las 12 h", y el desglose marca esos tramos "sin vigilancia". Si además el tracker **debería** estar corriendo (corrió alguna vez o está instalado, no lo apagaste y no es Windows ni desarrollo) y no responde en dos sondeos seguidos, la cabecera muestra una alarma persistente, "El tracker no está corriendo: no se mide el tiempo automático; Trabajar mide sin pausa (hasta 12 h)", y Trabajar avisa con un toast al empezar. Con el tracker apagado a propósito, o antes de que exista (F1–F5), no hay alarma.
- Al cerrar la app con una sesión abierta: si el tracker anda, la sesión sigue. Si está caído o no instalado, queda "Esperando al tracker" y se retoma cuando vuelva (ese rato no se mide). Si lo apagaste en Ajustes (o estás en Windows), termina como hoy; hasta F6, cuando el tracker todavía no se instala en ningún lado, también. En la CLI, sin tracker, la sesión queda esperando y se mide desde que arranque. Con el tracker apagado hay una asimetría a propósito: el botón Trabajar sigue midiendo con la app (U4), pero `gestor abrir X` hace el `cd` sin medir y lo avisa (U5), porque la CLI no tiene una app que mida por ella.

**Decisiones (firmes):**

| ID | Decisión |
|---|---|
| D-R8-1 | Una sesión de Trabajar termina **solo** al cerrar su(s) terminal(es) o con Parar (botón o `gestor parar`). Cerrar la app **no** la termina. El demonio sigue |
| D-R8-2 | Las sesiones de proyectos distintos conviven |
| D-R8-3 | Pausa tras **10 min** sin actividad (el `file_watch` automático sigue en 3 min). Actividad = cambios en la carpeta del proyecto **o** tipeo en la terminal. Se reanuda sola |
| D-R8-4 | Trabajar dos veces en el mismo proyecto = otra terminal en la **misma** sesión; termina cuando se cierra la última |
| D-R8-5 (R8-CLI) | Elegir un proyecto en `gestor` (menú `fzf` o `gestor abrir X`) = Trabajar en la terminal actual. Mide siempre por defecto (`--sin-medir` opcional). `list`, `ayuda` y `crear` no miden. `gestor parar` |
| 1a PRESENCIA | Los cambios de archivos cuentan como actividad solo si hubo teclado o mouse en el escritorio en los últimos 10 min (idle de la sesión gráfica; pantalla bloqueada = ausente). El tipeo en el pty cuenta siempre. Sin dato de presencia, un fallback explícito y registrado en el desglose (U2: cuentan, marcados) |
| 2a COLA | La sesión factura hasta la última actividad. El tramo final, después de la última actividad, **no** se factura. Las pausas internas menores a 10 min sí. Reemplaza la "cola de hasta 10 min" de la primera versión de R8 |
| 3a CWD | El tipeo cuenta para el proyecto solo si el proceso en primer plano del pty está dentro de la carpeta. Si es `ssh` (u otra sesión remota), cuenta, marcado "remoto: ssh" |
| 4a SIMULTÁNEO | Tramos facturables superpuestos entre dos proyectos se muestran ("simultáneo con B: 1 h 50") en el desglose de cada uno. Sin reparto automático |
| 5a AJUSTES | Tabla append-only `ajustes_tiempo`. Nada se borra; una corrección es otro ajuste. Totales = sesiones + ajustes |
| U1–U7 (2026-10-20) | Siete dudas resueltas (detalle en 9.3): presencia de **cualquier** ventana también para `file_watch`, con lo no cobrado visible como `solo_archivos` y convertible en ajuste (U1); sin dato de presencia los archivos cuentan con `presencia_desconocida` (U2); el paraguas se sostiene con los hijos (U3); cierre de la app según haya tracker (U4); `gestor abrir` con el tracker apagado no mide (U5); sin demonio, tope de 12 h + `sin_vigilancia` + aviso: alarma de cabecera solo si se espera el tracker y no corre, línea "Trabajar mide sin vigilancia" mientras mide la app y toast al empezar Trabajar (U6, 3.10.14); una sola etapa (U7). Los identificadores se explican en 9.4 |

### 1.2 Excepciones declaradas

**A D-R8-1** (cortes que no son "cerrar la terminal" ni Parar):

| Corte | Motivo | Por qué |
|---|---|---|
| Kill switch (apagar el tracker en Ajustes) | `tracker_apagado` | Sin demonio nadie mide esas sesiones, y la app no las adopta en silencio como `dueno = app`: para seguir midiendo, se toca Trabajar |
| Mandar el proyecto (o un ancestro) a la papelera | `proyecto_a_papelera` | Un proyecto en la papelera no se mide |
| Restaurar un backup | `restauracion` | El restore reemplaza la base; el diálogo lista las sesiones que se cortan |
| `gestor abrir Y` (o `--sin-medir` hacia otro proyecto) en la última terminal de X | `cambio_de_proyecto` | Es "cerrar la terminal" para X: esa terminal pasa a otro proyecto |
| Adopción imposible (3.10.7: Windows, tracker apagado en Ajustes, build de desarrollo o, en F1–F5, sin el binario del tracker): cerrar la app, Trabajar en otro proyecto, app caída | `cierre_app`, `cambio_de_proyecto`, `huerfana` | Sin demonio posible nadie puede seguir la sesión (U4) |
| Tope de 12 h de un tramo medido por la app con adopción imposible | `tope_sin_tracker` | U6: sin tracker no hay pausa, y el tope acota un olvido |

**A D-R8-3 y 2a sin tracker (`dueno = app`)** (U6, decidida):

| Caso | Qué pasa | Por qué |
|---|---|---|
| Sesión que mide la app (sin demonio vivo: F1–F5, tracker caído, no instalado o apagado en Ajustes, Windows, desarrollo) | No se pausa a los 10 min y el tramo `work_button` mide hasta Parar, Trabajar en otro proyecto o cerrar la app: **se cobra la cola**, con un **tope de 12 h** por tramo (`tope_sin_tracker`) | Sin demonio no hay señal de actividad (ni `atime` ni presencia). El desglose marca esos tramos **`sin_vigilancia`** ("medido hasta Parar, sin detección de inactividad"), la cabecera dice "Trabajar mide sin vigilancia" mientras la sesión vive y, si se espera el tracker, muestra además la alarma y el toast de 3.10.14. Riesgo aceptado 59 |

Las excepciones al principio 4 ("nunca inventar tiempo") están en §3, Principios.

**Fuera de alcance:**
- Windows y macOS. (En Windows la app debería compilar con la firma nueva de `open_terminal` —se verifica con `scripts/check.sh --windows`, §6— y mide como hoy: la adopción por el demonio es imposible, 3.10.7.)
- Medir sin sesión iniciada (lingering).
- Editar o borrar tramos medidos. **Cambio de alcance de la v3.2:** la v3.1 dejaba
  fuera "Editar sesiones a mano"; ahora los **ajustes manuales** entran, solo de
  agregado, con nota obligatoria y auditables (3.10.17). Las mediciones nunca se tocan.
- El concepto de **cliente**: la unidad de cobro es el proyecto; agrupar proyectos
  por cliente, tarifas o facturas no entra en esta versión.
- Reportes nuevos (el desglose de 3.10.14 explica el tiempo de un proyecto; no es un reporte).
- Paquetes `.deb`/`.rpm`/AppImage: la app **no instala** el demonio si no corre
  desde `BIN_DIR` (`~/.local/bin`), y lo informa (3.8). **Pendiente de verificar**
  si el bundler de Tauri 2 mete en el paquete el segundo binario (tarea del commit
  34, B4). Si lo mete, esta línea se corrige, pero el comportamiento no cambia: un
  `gestor-trackerd` en `/usr/bin` no se instala solo.
- Corregir datos históricos **sin consentimiento**. Las sesiones de más de 12 h,
  las huérfanas y los proyectos con carpeta inexistente (cifras en 2.1) se listan
  en el diff con datos reales (commit 24) y se deciden en 9.2 (DX3, DX4). El código
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
  en 3.2, "Regla única de duración mínima"). Resuelto en F0 (commit 8, `a3f5e76`).
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

Resueltos en F0 (rama `feat/tracker-f0`; la tabla describe `main@c8b55ae`): P2 (commit
3, `c0d0a6f`), P7 (6, `9017485`), P8 (9, `0d2fb1f`), P9 (7, `895955f`), P11 (4,
`fa35369`) y P16 (5, `c9f5e74`).

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
                    │   ├─ Presencia (D-Bus, 1 hilo, 3.10.4)         │
 terminales ───────►│   ├─ Lector de /proc y /dev/pts (3.10.3)       │
 (/proc, pty)       │   ├─ Motor (lógica pura, reloj inyectado)      │
                    │   │     abrir / extender / cerrar              │
                    │   ├─ Motor de terminales (3.10.2)              │
                    │   └─ Escritor (rusqlite, IMMEDIATE, no migra)  │
                    │         sesiones · tracker_status cada 15 s    │
                    └───────────────────────┬────────────────────────┘
                                            ▼
            ~/.local/share/gestor-proyectos/projects.db  (WAL, 0600, user_version)
            ~/.local/share/gestor-proyectos/trackerd.lock (flock)
            ~/.local/share/gestor-proyectos/restore.lock  (flock, 3.10.7)
                                            ▲
                    ┌───────────────────────┴────────────────────────┐
                    │  Gestor de Proyectos (Tauri)                   │
                    │   migra · lee cada 5 s · "Trabajar" · servicio │
                    └────────────────────────────────────────────────┘
                    CLI `gestor` ─► gestor-trackerd sesion|ajuste
                    (lib, sin migrar, misma base; 3.10.16)
```

**Principios:**
1. **La base es el único canal** entre app, demonio y CLI: no hay sockets ni IPC. La CLI escribe con la lib, por el subcomando `gestor-trackerd sesion|ajuste` (3.10.16).
2. **El motor es puro.** Recibe eventos con su instante y devuelve acciones, así que se testea con un reloj falso.
3. **Se guardan tramos y el total se calcula.** No hay contadores acumulados.
4. **Nunca inventar tiempo.** Ante la duda (huérfanas, eventos perdidos, suspensión) se cuenta de menos, nunca de más. Las excepciones son explícitas y acotadas, y el desglose (3.10.14) las marca:
   1. **DX1:** el margen de `min_credit_seconds` antes del primer cambio de una sesión `file_watch` (3.4). Además, dentro de una pausa de Trabajar, un guardado que reabre `file_watch` suma sus 60 s previos a la unión. El desglose lo dice ("incluye 60 s antes del primer guardado").
   2. **Fallback de presencia** (U2). Sin dato de presencia (sin sesión gráfica, D-Bus caído, escritorio sin API), los cambios de archivos cuentan sin verificar que el usuario esté, en las sesiones de Trabajar y en `file_watch`. Cada minuto así queda marcado `presencia_desconocida` (bit 8) y Ajustes avisa (3.10.4).
   3. **Tramos `dueno = app`** (U6; sin demonio vivo). Miden hasta Parar, Trabajar en otro proyecto o cerrar la app: sin señal de actividad, la cola posterior a la última actividad **se cobra**, con un **tope de 12 h** por tramo. El desglose los marca `sin_vigilancia` y la app lo avisa con la regla de 3.10.14 (3.10.13, riesgo 59).

   Con 2a, la cola de las sesiones del demonio **no** es una excepción: no se cobra. Lo contrario tampoco: los cambios de archivos **sin presencia** (U1) no inventan nada, porque no suman; quedan como tramos `solo_archivos` y solo cuentan si el usuario los carga como ajuste, con nota (3.10.4).
5. **Solo la app migra.** El esquema tiene versión (`PRAGMA user_version`). El demonio nunca altera el esquema: si la base está atrasada, espera; si es de una versión más nueva que la que conoce, no escribe sesiones (3.2). El subcomando de la CLI tampoco migra: con la base atrasada no mide (3.10.16).
### 3.1 Estructura del crate

**Decisión: biblioteca y segundo binario dentro del mismo paquete `src-tauri`.**

```
src-tauri/
├── Cargo.toml          # default-run = "gestor-proyectos" + [lib] gestor_proyectos_lib + [[bin]] gestor-trackerd
└── src/
    ├── lib.rs          # pub mod db; models; tracking; trackerd; time; paths; config; logging;
    │                   #   db/ suma trabajar.rs, ajustes.rs, tiempo.rs y desglose.rs (3.10)
    ├── main.rs         # la app: use gestor_proyectos_lib::{db, models, …}
    ├── bin/gestor-trackerd.rs   # punto de entrada fino del demonio
    ├── paths.rs        # NUEVO: rutas de datos, base, lock y config (P17)
    ├── time.rs         # NUEVO: formato único de fechas y rangos locales (3.3)
    └── trackerd/       # NUEVO: motor, vigilante, escritor, servicio,
                        #   procesos, presencia y terminales (3.10.3, 3.10.4, 3.10.2)
```

**Lo que suma R8 a la lib** (commit 33, al crearla): `db/trabajar.rs` (sesiones,
anclas y evidencia, 3.10.1), `db/ajustes.rs` (3.10.17), `db/tiempo.rs` (unión y
`simultaneidad`, 3.10.8), `demonio_vivo()` (3.10.7) y el helper de `restore.lock`; después
`trackerd::procesos` (40), `trackerd::presencia` (41), `trackerd::terminales` (42) y
`db/desglose.rs` (69). El binario `gestor-trackerd` suma los subcomandos `sesion`
(61) y `ajuste` (63), que usa la CLI: enlazan la lib, no Tauri, así que arrancan en
milisegundos y no toman `trackerd.lock` (3.10.16).

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
test de integración en `tests/` no la puede llamar. El commit 27 lo escribe como
`db::restore::tomar_lock` + `db::restore::restaurar_desde` (lock, `page_size`,
`Backup::step(-1)`, `migrar`, `integrity_check`, 3.6); entra en la lib con el resto de
`db/` en el commit 33, y `backup::restore_backup` queda como envoltorio de la app
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
| 4 | Sesiones de Trabajar: `trabajar_sesiones`, `trabajar_anclas`, `trabajar_evidencia`, `archivos_evidencia`; `terminal_token` y `motivo_cierre` en `time_tracking_sessions`; columnas nuevas de `tracker_status`; índices parciales (3.10.1) | Esquema paso 4 (15) |
| 5 | `ajustes_tiempo` con sus triggers (3.10.17) | Ajustes solo de agregado (22) |

`ESQUEMA_ACTUAL = 5` al terminar F1.

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
| `source` | Valores `work_button`, `file_watch` y, desde el paso 4, `terminal` (tramos que mide el demonio para una sesión de Trabajar, 3.10.1). `shell_hook` y `manual` quedan solo en filas viejas (hoy 2 y 1) | Saber quién la creó |
| `terminal_token`, `motivo_cierre` | **Nuevas** (paso 4). El tramo pertenece a esa sesión de Trabajar, y por qué se cerró. Definición y `CHECK` en 3.10.1 | Explicar cada tramo (3.10.15) |
| `last_activity_at DATETIME` | **Nueva** (paso 2). Se escribe **al abrir** (= `started_at`) en todas las fuentes | Latido de la sesión, y hora real para recuperar huérfanas |
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
| `work_button` al cambiar de proyecto, con Parar o al cerrar la ventana | Ahora (pared) | Lo medido con `Instant`, con la regla mínima de abajo y el tope de 12 h de un tramo con token (U6, 3.10.13) |
| `terminal` (todo cierre: pausa, terminal cerrada, Parar, kill switch, SIGTERM) | La última actividad (2a, 3.10.4) | `ended_at − started_at`. Si da 0, la fila se borra |
| Huérfana `file_watch` (el demonio, al arrancar) | `COALESCE(last_activity_at, started_at)` | `ended_at − started_at`. Si da 0, la fila se borra |
| Huérfana `work_button` (la app, en `.setup()`, solo si `COALESCE(last_activity_at, started_at)` tiene más de 2 min) | `COALESCE(last_activity_at, started_at)` | `MIN(COALESCE(duration_seconds, 0), ended_at − started_at, 43200)`, con la regla mínima. Motivo `huerfana` (o `tope_sin_tracker` si pasó las 12 h); si tiene token, el token pasa a `esperando_demonio` (3.10.11). Una fila `terminal` nunca se toca acá: la recupera el demonio (3.10.10) |

Desde el paso 4, todo cierre lleva además `motivo_cierre` (lista en 3.10.15).

Las 81 huérfanas reales no tienen `last_activity_at` ni `duration_seconds`:
cierran en su `started_at` con 0 s. Hoy ya suman 0, así que el total no cambia.

**Métodos de `Database` con instante explícito (D14).** Nadie más escribe
`CURRENT_TIMESTAMP` en las sesiones:

- `abrir_sesion(project_id, source, started_at) -> Result<i64>`: escribe `started_at` y `last_activity_at`.
- `latir(id, last_activity_at, duration_seconds: Option<i64>) -> Result<bool>`: `WHERE id = ? AND ended_at IS NULL`. Devuelve `false` si no tocó ninguna fila (sesión cerrada o proyecto purgado, 3.4).
- `cerrar_sesion(id, ended_at, duration_seconds, motivo) -> Result<bool>`: `WHERE id = ? AND ended_at IS NULL`. Devuelve `false` si ya estaba cerrada. El `motivo` entra en el commit 15, antes de que el método tenga llamadores.

`create_tracking_session` y `end_tracking_session` quedan como envoltorios con
"ahora" para no tocar los llamadores de golpe.

**Regla única de duración mínima (M8), hecha en F0** (commit 8, `a3f5e76`). Antes
de F0 la regla de los 10 s se aplicaba solo al cambiar de proyecto (`main@c8b55ae`,
`commands/mod.rs:1569-1579`); Parar y cerrar la ventana guardaban cualquier duración,
y el cierre con 0 s descartaba su error (`let _`). Desde el commit 8 hay una sola
función, `duracion_a_guardar(source, segundos)` (`commands/mod.rs:1704` en
`83c48d4`), que usan **todos** los cierres; R8 le suma el caso `terminal` (commit 15):

- `work_button` y fuentes viejas: 10 s o menos se guardan como 0. La fila se conserva, con su `ended_at`, pero no cuenta como sesión (3.3). El error del cierre se propaga.
- `file_watch`: sin regla de 10 s; la cubre el margen de abajo.
- `terminal`: sin regla de 10 s (`duracion_a_guardar('terminal', s) = s`); un tramo de 0 s se borra (DT5, 3.10.1).
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
| `trabajar_idle_minutes`, `presencia`, `presencia_motivo`, `presencia_at` | **Paso 4.** Pausa efectiva de Trabajar y estado de la presencia del escritorio (`kde`, `gnome`, `prueba` o `no_disponible`), 3.10.1 y 3.10.4 |

La UI no depende de `tracker_status` para **calcular** tiempo, solo para
mostrar el estado (3.3).

**`projects.total_time_seconds`:** queda **deprecada**. La columna se mantiene
en el esquema para no romper bases viejas ni backups, pero el código deja de
leerla y de escribirla en dos commits (D2):

- **Lectura por unión:** `get_time_stats` y `get_project_stats.total_time_hours` (`db/activity.rs:63-69`) pasan a `tiempo_trabajado` (3.3). Nada se borra todavía.
- **Sacar el contador:** el segundo `UPDATE` de `end_tracking_session` (`db/tracking.rs:37-43`; `~49-54` en `83c48d4`); `add_project_time` (`db/activity.rs:28-39`), su comando (`commands/mod.rs:501-508`; `:533-542` en `83c48d4`), su registro (`main.rs:113`; `:119` en `83c48d4`) y su función de API sin llamadores (`src/services/api.ts:161-166`); el campo en `PROJECT_COLUMNS` y en el mapeo (`db/common.rs:18,43`), en el modelo (`models/project.rs:30`) y en el tipo (`src/types/project.ts:33`); los tests que lo afirman (`db/tracking.rs:170-172,185,191-195`, `db/activity.rs:164-172`) y el fixture (`models/project.rs:250`); y `ARQUITECTURA.md:442`.
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
| Cerrada, `terminal` | `[started_at, ended_at]` (igual a `started_at + duration_seconds`, 3.10.1) |
| Abierta, `terminal` | `[started_at, last_activity_at]` |
| Abierta, otra fuente | `[started_at, started_at + COALESCE(duration_seconds, 0)]`: lo medido hasta el último latido (3.2), o 0 s si nunca latió |

"Otra fuente" incluye los tramos `work_button` de una sesión de Trabajar, con o sin
`terminal_token`. La `activa` es, desde el commit 17, la copia de la única
`MedicionApp` de `ActiveSession(Mutex<Option<MedicionApp>>)` (3.10.7): las sesiones del
demonio se leen de la base y no la necesitan (3.10.8).

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
(`start_new_session_sync`, `commands/mod.rs:1638-1684` en `83c48d4`). El demonio llama con `activa = None`.

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
| `total_seconds`, `today_seconds`, `week_seconds` | `tiempo_trabajado` sobre su rango (unión), con la `activa`. Desde el commit 22, `tiempo_total = max(0, tiempo_trabajado + Σ ajustes del rango)` (3.10.8, 3.10.17) |
| `session_count` | Sesiones cerradas con duración > 0 |
| `avg_session_seconds`, `longest_session_seconds` | Sobre las sesiones **individuales** cerradas con duración > 0, sin unir: describen sesiones, no el total |

Un error de base **se propaga** (D6, P18): se reemplazan los `.unwrap_or(0)` de
`db/tracking.rs:101,119,130,141` y `db/activity.rs:61,68`. El mensaje es de
sistema y queda en inglés, según el criterio de `CLAUDE.md`.

`get_project_stats` también pasa a la unión y a la hora local:
`total_time_hours` es la suma de `tiempo_trabajado` de los proyectos fuera de la
papelera (de `tiempo_total` desde el commit 22), y `active_today` cuenta los proyectos con actividad en
`project_activity` **o** con tiempo trabajado en el día local. Lo segundo es
nuevo: con el demonio, un proyecto editado con la app cerrada también estuvo
activo.

> **Por qué en Rust y no en SQL:** fusionar intervalos en SQLite exige consultas
> recursivas difíciles de leer y de testear. En Rust son unas 20 líneas con tests
> de tabla. Hoy hay 193 sesiones; aun con 50.000 el cálculo es instantáneo.

**Simultaneidad (4a).** `simultaneidad(project_id, rango)` intersecta la unión de un
proyecto con la de cada otro; se calcula, no se guarda, y los ajustes no entran
(3.10.8).

**Tests de zona horaria (D13):** nunca se cambia `TZ` del proceso; cada caso pasa
su zona a `rango_dia` con `chrono-tz`. Casos y precondiciones en 5.1.
### 3.4 El motor del demonio

**Reloj:** las marcas de tiempo son de **pared** (UTC del sistema), porque son las
que se guardan. No se usa `Instant` para la inactividad: en Linux es
`CLOCK_MONOTONIC`, no avanza durante una suspensión y ocultaría el corte. El
motor recibe el reloj inyectado, así que todo lo de esta sección se prueba con un
reloj falso (commit 36).

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
  acotar el margen de DX1: nunca se acredita tiempo de antes del despertar. La
  clasificación la hace `clasificar_salto` de la lib, la misma que usa el motor de
  terminales con su propio umbral (3.10.5).

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
| SIGTERM / SIGINT | — | **Cerrar** todas en su `ultimo_cambio`. Antes, el demonio relee la config: con `false` es el kill switch; con `true`, las sesiones de Trabajar no terminan (DT7, 3.10.10) |

La regla de `cambio` con hueco ≥ inactividad es la que corta bien en dos casos: un
evento que llega antes del próximo `tick`, y un evento que llega justo después de
una suspensión de horas. El tramo nunca cruza la pausa.

**Compuerta de presencia (U1, commit 48).** Un `cambio` filtrado llega a este motor
solo si el usuario está presente en el escritorio (o no hay dato de presencia, U2:
cuenta con el bit 8). Sin presencia **no abre ni extiende** ningún tramo automático, y
el minuto queda en `archivos_evidencia` con el bit 32 y sus segundos (3.10.4). Desde
el commit 15 todo cierre de `file_watch` lleva `motivo_cierre` (`inactividad`,
`suspension`, `reloj`, `tracker_detenido`, `tracker_caido`, `tracker_apagado`,
`proyecto_a_papelera`, `sin_vigilancia`; 3.10.15).

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
el demonio cierra todas las sesiones (las de Trabajar terminan con `tracker_apagado`,
3.10.12), pone `estado = 'desactivado'` y
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
   borra si queda en 0 s. Después, **sesiones de Trabajar**: lee el `pid` que dejó la
   instancia anterior (salida limpia o sucia) antes de pisar `tracker_status`, y
   aplica el paso 6 extendido de 3.10.10 (readopción, cierres con `tracker_caido` o
   `terminal_cerrada`, gracia).
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
| Presencia (3.10.4) | Lee la inactividad y el bloqueo del escritorio por D-Bus cada 5 s y manda la lectura al principal por canal | Tocar la base; hacer esperar al principal (un bus colgado no frena el bucle ni el watchdog) |

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
`.stversions` (Syncthing), `.dropbox.cache` (Dropbox). Desde la v3.2 (commit 39,
3.10.4), también `.vite`, `logs`, `.angular`, `.expo` y `.dart_tool` por nombre, y
`storage/framework` y `bootstrap/cache` (Laravel) **por ruta relativa a la raíz** del
proyecto, porque `framework` y `cache` son nombres genéricos.

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
  "max_dirs_per_project": 5000,
  "trabajar_idle_minutes": 10
}
```

| Campo | Rango | Fuera de rango |
|---|---|---|
| `daemon_enabled` | `true`/`false` | Default `true` |
| `idle_minutes` | 1–60 | Se acota y se anota en `last_error` (M14) |
| `min_credit_seconds` | 0–300 | Se acota; 0 = comportamiento v2 (DX1) |
| `max_dirs_per_project` | 100–200.000 | Se acota (DX2) |
| `trabajar_idle_minutes` | 1–120 | Se acota. Pausa de las sesiones de Trabajar (D-R8-3, 3.10.2); el valor efectivo se publica en `tracker_status.trabajar_idle_minutes` |

**Tiempos para tests:** la variable de entorno `GESTOR_TRACKERD_TEST_TIMINGS`
(por ejemplo `idle=5s,tick=500ms,flush=500ms,reload=1s,heartbeat=1s,config=200ms,rescan=1s`)
reemplaza los tiempos. R8 suma `poll=` (sondeo de terminales), `grace=` (gracia) y
`pause=` (pausa de Trabajar), y el parser rechaza claves desconocidas (3.10.2). Con la
variable definida, el demonio se niega a usar la base real del usuario (B8, 3.4).
`GESTOR_TRACKERD_TEST_PRESENCIA=<archivo>` fija la presencia en los tests y solo se
acepta junto con `GESTOR_TRACKERD_TEST_TIMINGS` (3.10.4).
### 3.6 Base compartida entre procesos

Desde la v3.2 escriben **tres** procesos: la app, el demonio y el subcomando de la CLI
(`gestor-trackerd sesion|ajuste`, 3.10.16). El subcomando sigue las mismas reglas que
el demonio: `busy_timeout`, transacciones `IMMEDIATE`, sin migrar, y la base resuelta
con `--data-dir` o `GESTOR_DB`, como la app.

| Cambio | Detalle |
|---|---|
| **WAL** | Lo activa **solo la app**, en `Database::new`, con `pragma_update_and_check`, y verifica el resultado. Acepta `"wal"`, y también `"memory"` para las bases `:memory:` de los tests. Cambiar de modo necesita la base para sí sola: si después de `busy_timeout` sigue sin ser `wal`, `Database::new` **falla** con un error visible ("No se pudo preparar la base de datos: otro programa la tiene abierta. Cerralo y volvé a abrir la app.") en vez de seguir en `delete` en silencio (D8). Queda persistente en el archivo |
| El demonio y WAL | No cambia el modo. Si al abrir lee algo distinto de `wal`, espera igual que con un `user_version` atrasado (3.2) |
| `busy_timeout = 5000` | Se mantiene como **requisito** en las dos conexiones (`db/schema.rs:21`). Sin él, cualquier escritura del otro proceso es un `SQLITE_BUSY` inmediato |
| `synchronous = NORMAL` | En las dos conexiones. Es seguro con WAL |
| **Las 6 transacciones** pasan a `transaction_with_behavior(Immediate)` (P13) | El riesgo real son dos: `restore_project` y `purge_project` **leen antes de escribir**, y con WAL una transacción diferida así falla con `SQLITE_BUSY_SNAPSHOT` si el otro proceso escribe en el medio; `busy_timeout` no la salva. Las otras cuatro empiezan escribiendo y solo esperarían. Igual pasan todas, por una regla simple: **toda transacción que escribe es `IMMEDIATE`**, así nadie agrega después una lectura al principio de una diferida (D9). Dos llegan antes que el resto: `end_tracking_session` en el commit 15 y `delete_project` en el 18, porque desde ahí compiten con otras conexiones; el commit 28 convierte las **cuatro** que quedan (`trash.rs:68/145/211`, `status.rs:75`, líneas de `feat/tracker-f0`, 3.10.7). `empty_trash` mete adentro el `SELECT` de ids que hoy hace afuera (`db/trash.rs:205-211`), y `restore_project` y `purge_project` meten adentro la guarda `SELECT deleted_at` que hoy validan antes de abrir la transacción (`db/trash.rs:56-66` y `:133-143`): si no, otro proceso puede restaurar o purgar entre la guarda y la escritura |
| `end_tracking_session` idempotente (P2). **Hecho en F0** (commit 3, `c0d0a6f`) | `WHERE id = ? AND ended_at IS NULL`. Si ese `UPDATE` no afecta ninguna fila, **no** se ejecuta el segundo, el que suma al contador (`db/tracking.rs:38-43`): se hace rollback y se devuelve `Ok(false)` (B3). Rige hasta que el contador desaparece (3.2) |
| `get_project` y `update_project` sin `try_lock` (P16). **Hecho en F0** (commit 5, `c9f5e74`) | Pasan a `self.conn()`, que espera y se recupera del envenenamiento como el resto (D7). La "cicatriz de deadlock" del comentario (`db/projects.rs:111-112`) no tiene sustento hoy: `get_project` pasa su guard a `get_project_links_internal` (`:142`), `update_project` lo suelta antes de llamar a `get_project` (`:244-249`) y `get_project_with_children` llama a `get_project` sin tener el lock (`db/groups.rs:81`). La espera queda acotada por lo que tarda la transacción del otro lado, que en el demonio son milisegundos; ningún método sostiene el Mutex más allá de su propia consulta |
| Checkpoint | El demonio corre `wal_checkpoint(TRUNCATE)` cada hora |
| Backups (`VACUUM INTO`) | Siguen igual. La copia se pasa a `journal_mode=DELETE` **antes** de verificarla, para que no aparezcan `-wal`/`-shm` en la carpeta de backups |
| **Permisos** (P19, D11) | `paths::data_dir()` crea la carpeta con `0700` y corrige una existente. `Database::new` deja `projects.db` en `0600` **antes** de activar WAL: SQLite crea `-wal` y `-shm` con los permisos del archivo de la base. Cada backup nuevo queda en `0600`. El unit lleva `UMask=0077` (3.7) |

**Restore (P6):** se reescribe con la **API de backup de SQLite** (feature
`backup` de rusqlite), que escribe **dentro** de la base viva en vez de reemplazar
el archivo. Secuencia:

1. **Antes de confirmar**, la UI avisa qué se pierde: todo lo registrado después de la fecha del backup, incluidas las sesiones del demonio y la de "Trabajar" en curso. Desde el commit 27 lista además las sesiones de Trabajar vivas que se van a cortar ("Se van a cortar: A (desde 14:02), B (desde 15:30)…", 3.10.9). Al confirmar, la app toma `restore.lock` **exclusivo** y lo retiene hasta el paso 8: mientras tanto ni Trabajar ni la CLI crean sesiones (3.10.7).
2. Verificar el backup elegido, como hoy (`backup/mod.rs:430-445`).
3. `systemctl --user stop gestor-trackerd`, recordando si estaba activo (3.8). El `stop` cierra los tramos del demonio en la base vieja; las sesiones de Trabajar quedan en `viva_pausada` (DT7), y las que sigan vivas en la base restaurada las corta el paso 6 con `restauracion`.
4. Tomar `trackerd.lock` (`paths::trackerd_lock_file()`, 3.4) con `flock` exclusivo y no bloqueante, y **retenerlo** hasta el paso 7. Si está tomado (un demonio lanzado a mano o fuera de systemd), se aborta: "El tracker sigue corriendo. Paralo con `systemctl --user stop gestor-trackerd` y volvé a intentar." Retenerlo impide además que un demonio arranque en el medio: si lo intenta, sale con 75 y no escribe.
5. Vaciar la `ActiveSession` **sin escribir**: tomar su lock y dejarla en `None`. Parar la sesión antes del restore no sirve, porque el cierre se escribe en la base que el restore pisa. Vaciarla evita que un cierre posterior (Parar, o cerrar la ventana en los 800 ms previos a la salida, `commands/mod.rs:101-104`) escriba en la base restaurada con un id que ahí no existe o es otra sesión.
6. Con el Mutex de `Database` tomado (orden `ActiveSession → Database`, 3.3):
   - Verificar el `page_size`: con la base viva en WAL, la API de backup falla si no coincide, así que se informa con un error claro antes de empezar.
   - `Backup::new(&origen, &mut conn)` y `step(-1)`: una sola pasada copia todas las páginas en **una** transacción de escritura de la base viva. Si falla, no queda a medias.
   - Verificar `journal_mode = wal` y volver a fijarlo si hiciera falta. La API de backup **conserva** el modo WAL del destino aunque el origen esté en `delete` (SQLite reescribe los bytes de versión de la cabecera al terminar, https://sqlite.org/backup.html), así que el re-fijado es **defensivo**: no debería cambiar nada, pero cuesta una línea y cubre un cambio futuro de SQLite.
   - Correr `migrar` (3.2) sobre lo restaurado, porque un backup viejo no tiene `last_activity_at` ni `tracker_status`, y `integrity_check`. La app migra **antes** de soltar el lock: el demonio nunca ve un `user_version` viejo.
   - Cerrar con motivo `restauracion` toda sesión de Trabajar que la base **restaurada** tenga viva (el backup se hizo con sesiones abiertas): el tramo en su `last_activity_at` y el token por la regla única de `terminada_at` (3.10.9, 3.10.15).
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
checkpoint. Se documenta en `ARQUITECTURA.md` (commit 32) y en Ajustes → Backups (commit 67): excluir
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
| Nombre `gestor-trackerd` | Distinto del de la app; igual `pkill -f gestor-proyectos` lo alcanzaría por el `--data-dir …/gestor-proyectos` del `ExecStart`, por eso el commit 53 acota el patrón de `start-app.sh` (P10) |

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
  baja nada sin avisar. Test con systemctl falso en el commit 54.
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

**Opciones prohibidas por R8:** el unit **no** lleva `ProtectProc=`, `ProcSubset=`,
`PrivatePIDs=` ni `PrivateDevices=`. El demonio necesita leer `stat`, `environ` y
`cwd` de procesos de fuera del unit y el `atime` de `/dev/pts/N` (3.10.3). El
generador lo comenta, el golden del commit 54 lo verifica y E16 lo prueba bajo el
aislamiento real. La presencia por D-Bus (bus de sesión y `LockedHint` de logind por el
del sistema) funciona con este aislamiento: verificado con `systemd-run --user` (3.10).

**No se usa `ProtectHome=`:** con `yes` o `tmpfs` el demonio no vería las carpetas
de los proyectos; con `read-only` no agrega nada a `ProtectSystem=strict`.

**Verificación:** el texto del unit tiene test unitario (commit 54). Que el
aislamiento realmente deja medir lo prueba **E16** (T2): `systemd-run --user` con
**las mismas propiedades** que genera el código salvo `WatchdogSec=5` (se arman desde la misma función),
`XDG_*` en carpetas temporales y un proyecto en `$HOME`: `READY`, un guardado
crea la sesión, `stop` la cierra. Paso manual: `systemd-analyze security --user
gestor-trackerd` y anotar el puntaje en el PR.
### 3.8 Ciclo de vida desde la app (R6)

**Hecho en F0** (commit 4, `fa35369`). En `main@c8b55ae`, `main.rs` no tenía
`.setup()`: abría la base, leía la config y hacía el auto-backup en `main()` antes
del plugin de instancia única (`main.rs:29-53` contra `:60`, P11). Desde el commit 4
corre **después** de los plugins (D15), con este orden, que sale de cómo arranca Tauri 2 (verificado en `crates/tauri/src/app.rs`,
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
   está habilitado o activo, `systemctl --user disable --now gestor-trackerd`, y
   después `cerrar_por_kill_switch()` cierra con `tracker_apagado` las sesiones de
   Trabajar del demonio que hayan quedado vivas (3.10.12). Termina. El indicador muestra "Tracker apagado (desactivado en Ajustes)".
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
reglas de cierre de 3.2 (M7). Solo cierra filas cuyo
`COALESCE(last_activity_at, started_at)` tenga **más de 2 minutos** (2 latidos),
así nunca toca la sesión viva de otra instancia.

Desde el commit 19 esa recuperación toca **solo** filas `source = 'work_button'`,
aplica el tope de 12 h y, si la fila tiene token, deja la sesión en
`esperando_demonio` en vez de terminarla (3.10.11).

Cerrar la app **no** toca el demonio. Tampoco termina las sesiones de Trabajar:
`CloseRequested` llama a `al_cerrar_ventana` (3.10.11, D-R8-1).

**Desde Ajustes → Tracking (B5).** Siempre **primero la config, después
systemctl**. Si `systemctl` falla, la config ya cambió y el demonio la ve en ≤ 5 s
(3.5), así que el apagado nunca depende de que systemd responda:

| Acción | Secuencia |
|---|---|
| Apagar | `daemon_enabled = false` → `disable --now`. El demonio, si sigue vivo, cierra sus sesiones y sale 0. Después, `cerrar_por_kill_switch()` cierra lo que quede, idempotente, y muestra el toast de 3.10.12 |
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
  (`<< 'EOF'`, `install.sh:40`), que no expande variables; el commit 59 lo pasa a
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
demonio la pisa entera al arrancar. Desde el commit 60 (R8-C1) también borra la
CLI `gestor` y, solo con `--rc`, sus bloques de los rc (3.10.16).
### 3.9 La app: lo que ve el usuario

**El tiempo deja de depender de `.gestor/` (B2).** Hoy `TimeTracker.tsx`
(`loadTrackingData`, `:272-289`) llama a `checkTrackingConfig` y **solo** pide
`getTimeStats` si existe `<proyecto>/.gestor/config.json` (`:277-283`, que usa
`has_tracking_config` de `tracking/config.rs:122`; `:125` en `83c48d4`). Si no, las dos vistas muestran
el botón "Track" / "El tracking de tiempo no está activado" (`:41-65` en `CompactView`, `:130-150` en `FullView`).
El demonio no crea `.gestor/` (y la ignora, 3.5), así que todo lo que mide sería
**invisible**. Cambio: `TimeTracker` carga `getTimeStats` **siempre** y desaparece
la rama "no activado"; `checkTrackingConfig` e `initTracking` quedan sin llamadores
desde esta vista (su retiro, si corresponde, va aparte). Test Vitest: sin
`.gestor/`, con sesiones en la base, la tarjeta muestra el total.

| Pieza | Comportamiento |
|---|---|
| `get_live_tracking` | Una consulta: sesiones abiertas (todas las fuentes) con `started_at`, `last_activity_at` y `source`, más `tracker_status`. Para la `ActiveSession` de **esta** app agrega `elapsed_seconds` medido con su `Instant` (lo que ya calcula `work_session_status_sync`, `commands/mod.rs:1666`). Con R8 suma las sesiones de Trabajar vivas (`token`, `estado`, `dueno`, `origen`, `creada_at`, `last_seen_at`, tramo abierto, cantidad de terminales), `demonio_vivo` y `tracker_status.presencia` (commit 64, 3.10.7) |
| Contador en la tarjeta, **`work_button`** (`dueno = app`, M5) | Muestra `elapsed_seconds` del backend, **no** `ahora − started_at`: el `Instant` no avanza en una suspensión, igual que lo que se guarda al parar (`start_new_session_sync` y `stop_active_session_sync`). Entre sondeos suma de a 1 s con un `setInterval` que se **resincroniza** en cada respuesta y nunca adelanta más de 5 s. Con `Date.now()` el contador incluiría la suspensión y después **bajaría** al guardarse |
| Contador en la tarjeta, **`file_watch`** (M9) | Muestra `last_activity_at − started_at`. Avanza **a saltos, con cada cambio guardado**, con un retraso de hasta ~10 s (≤ 5 s del volcado del demonio, 3.4, más ≤ 5 s del sondeo de la app), y se queda quieto mientras no se toca nada. Nunca pasa del último cambio real (R3). Incluye el margen de DX1 (`started_at` ya viene corrido hacia atrás) |
| Contador de una sesión de Trabajar **`dueno = demonio`** | La suma de las `duration` de los tramos del token, leída de la base, sin interpolar. Avanza a saltos, con hasta ~10 s de retraso, y **nunca baja ni salta al cerrar** (2a, 3.10.14) |
| Texto "en vivo" (M9) | El punto verde y el tooltip dicen lo que pasa de verdad: **"Midiendo por cambios en archivos · se actualiza con cada guardado (con unos segundos de retraso, hasta 10 s)"**. Para `work_button`: "Midiendo con Trabajar". No se promete un segundero continuo para `file_watch` |
| Frecuencia | Cada 5 s, solo con la ventana visible (`visibilitychange`) |
| Badge de total | Se refresca cuando se cierra una sesión (P5) |
| Indicador global | En la cabecera. Estados abajo |
| Ajustes → Tracking | Activar/desactivar (3.8), minutos de inactividad (1–60, default 3) con el valor **efectivo** de `tracker_status.idle_minutes`, margen por guardado (DX1), tope de carpetas por proyecto (DX2), estado, proyectos vigilados y no vigilados con su motivo, y uso de inotify (abajo). Con R8: "Pausa de Trabajar por inactividad" (1–120 min, default 10, con el valor efectivo) y "Presencia del escritorio: disponible (KDE)" o "no disponible: <motivo>…" (3.10.14) |
| Ajustes → Backups | Nota de D12 (3.6): "Si respaldás el home con rsync, Déjà Dup o Syncthing, excluí `projects.db*` e incluí la carpeta de backups de la app." |
| Avisos por proyecto | Tabla de abajo |
| "Trabajar" | Su latido de 60 s escribe `last_activity_at` **y** `duration_seconds` medido con `Instant` (3.2), así que el tramo vivo en la base es `started_at + elapsed` y coincide con el contador. Muestra los errores en español (P7) y suma el botón **Parar** (P9). Desde el commit 17 abre la terminal con la marca de la sesión, lee el estado de la base y Parar devuelve `Cerrada`, `YaCerrada` o `Ninguna` (3.10.7, 3.10.9) |
| Sesiones de Trabajar | Indicador de la tarjeta por estado (con "tracker caído"), toasts por transición, desglose del tiempo con ajustes (3.10.14, 3.10.17) |
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

**Aviso sin tracker (U6).** La regla completa está en 3.10.14. En corto: la alarma
"El tracker no está corriendo: no se mide el tiempo automático; Trabajar mide sin
pausa (hasta 12 h)" aparece solo si se espera el tracker (`!adopcion_imposible()`, y
corrió alguna vez o está instalado) y `demonio_vivo()` da falso dos sondeos seguidos;
con una sesión `dueno = app` viva se ve la línea "Trabajar mide sin vigilancia"; con el
tracker apagado a propósito o en desarrollo, sin sesión, no hay nada más que el
indicador de arriba. Antes del commit 66 lo muestra la cabecera del commit 17; desde el
66, el indicador global.

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

### 3.10 Sesiones de Trabajar atadas a la terminal (R8 y R8-CLI)

Este capítulo es la única definición de R8: el modelo de datos de las sesiones de
Trabajar, sus estados, cómo el demonio detecta las terminales, la inactividad, la
presencia, el dueño de cada sesión, Parar, el kill switch, la CLI y los ajustes
manuales. Las secciones 3.2–3.9 dicen qué cambia en cada pieza heredada y remiten
acá. Las decisiones del usuario están en §1.1 y sus excepciones en §1.2.

**Vocabulario.** "Sesión" en la UI quiere decir **token** (una fila de
`trabajar_sesiones`); "tramo" quiere decir **fila** de `time_tracking_sessions`. "UA"
es la **última actividad** válida de la sesión. Las líneas de código citadas en este
capítulo son de `feat/tracker-f0@83c48d4`.

| Subsección | Tema |
|---|---|
| 3.10.1 | Modelo de datos (pasos 4 y 5 del esquema) |
| 3.10.2 | Máquina de estados de una sesión |
| 3.10.3 | Detección de procesos y de la marca |
| 3.10.4 | Inactividad: pty, archivos y presencia |
| 3.10.5 | Suspensión y saltos de reloj |
| 3.10.6 | Convivencia de proyectos y varias terminales |
| 3.10.7 | Quién es el dueño: la base como fuente de verdad |
| 3.10.8 | Integración con la unión de tramos (3.3) |
| 3.10.9 | Parar, papelera y restore |
| 3.10.10 | Arranque y cierre del demonio |
| 3.10.11 | Arranque y cierre de la app |
| 3.10.12 | Kill switch |
| 3.10.13 | Sin demonio |
| 3.10.14 | UI: tarjeta, toasts y desglose |
| 3.10.15 | Explicabilidad: motivos de cierre |
| 3.10.16 | R8-CLI: medir desde `gestor` |
| 3.10.17 | Ajustes manuales |

**Decisiones técnicas** (justificación en cada subsección):

| ID | Decisión | Dónde |
|---|---|---|
| DT1 | Una pausa, una suspensión o un salto de reloj **parten la fila**. Cada tramo `terminal` cumple `ended_at − started_at = duration_seconds` y `last_activity_at = ended_at` | 3.10.1, 3.10.5 |
| DT2 | La **base** es la fuente de verdad de las sesiones (`trabajar_sesiones`). `ActiveSession(Mutex<Option<MedicionApp>>)` guarda solo la **única** sesión que mide la app | 3.10.7 |
| DT3 (corregida por 2a) | Un tramo abierto llega hasta la **última actividad**, y **todo** cierre (terminal, Parar, kill switch, SIGTERM) queda ahí. La cola no se cobra. El contador nunca baja ni salta al cerrar | 3.10.4 |
| DT4 | Del pty cuenta solo `atime` (entrada), nunca `mtime` (salida). Desde una pausa, reanudar por tipeo exige **dos** cambios de `atime` en 60 s | 3.10.4 |
| DT5 | Los tramos `source = 'terminal'` no usan la regla de los 10 s; uno de 0 s se borra | 3.10.1 |
| DT6 (corregida) | `terminal_no_detectada` **pausa**, no termina: mide por archivos con presencia (y por el tipeo de un ancla de multiplexor, si la tiene) y sale con Parar o si aparece la terminal | 3.10.2 |
| DT7 | SIGTERM con el tracker encendido cierra el tramo (`tracker_detenido`) sin terminar la sesión; el demonio relee la config y, si está apagado, termina con `tracker_apagado` | 3.10.10, 3.10.12 |
| DT8 | CLI: la identidad es un **ancla** `(pid, starttime)` de la shell. Un pty pertenece a un solo token; `gestor parar` y `--sin-medir` lo **vetan**. Un ancla dentro de tmux/screen no sostiene la sesión | 3.10.16 |
| DT9 | La CLI escribe por el subcomando `gestor-trackerd sesion …` (lib). Sin demonio crea la sesión en `esperando_demonio` | 3.10.16 |
| DT10 | Presencia del escritorio por D-Bus (KDE en ms, GNOME Mutter, `LockedHint` de logind) en un hilo propio | 3.10.4 |
| DT11 | Atribución del tipeo por el `cwd` del primer plano (`tpgid`), con remotos marcados y panel activo de tmux | 3.10.3, 3.10.4 |
| DT12 | Evidencia por minuto (`trabajar_evidencia` por token y `archivos_evidencia` por proyecto para `file_watch`), con bits de lo contado y lo ignorado y el segundo exacto del primer y último cambio sin presencia | 3.10.1 |
| DT13 | Ajustes solo de agregado, con triggers que impiden `UPDATE` y `DELETE` | 3.10.17 |
| DT14 | `dueno` es el dueño **actual** de la sesión y nunca cambia con un tramo abierto: cada tramo tiene un solo medidor, que dice su `source` (`terminal` = demonio, `work_button` = app). Traspaso `app → demonio` (en espera) en los puntos naturales; `demonio → app` solo con un Trabajar explícito. Nunca hay adopción silenciosa | 3.10.7 |
| DT15 | Restaurar corta las sesiones vivas con aviso; `restore.lock` impide crear sesiones durante el restore | 3.10.7, 3.10.9 |
| DT16 (U1) | La presencia también filtra `file_watch`. Los cambios sin presencia no suman: se registran por minuto y el desglose los agrupa en tramos derivados `solo_archivos`, convertibles en ajuste | 3.10.4, 3.10.14 |
| DT17 (U6) | Sin demonio, el tramo `work_button` tiene un **tope de 12 h** (`TOPE_SIN_TRACKER`), se marca `sin_vigilancia` (derivado del `source`) y la app lo avisa: alarma de cabecera solo si se espera el tracker y no corre, línea "Trabajar mide sin vigilancia" mientras mide la app y toast al empezar Trabajar | 3.10.13, 3.10.14 |


> **Verificado en esta máquina** (KDE Plasma 6.7.4, X11, systemd 261):
> `org.freedesktop.ScreenSaver.GetSessionIdleTime` responde por el bus de sesión y
> devuelve **milisegundos** (cinco lecturas a 1 s: 1670, 2674, 3677, 4681, 5684),
> aunque la especificación hable de segundos. El objeto también expone la interfaz
> `org.kde.screensaver`. `LockedHint` de logind se lee en la sesión `Display` del
> usuario (`/org/freedesktop/login1/user/self` → sesión `4`). Las dos lecturas
> funcionan desde `systemd-run --user` con `PrivateUsers=yes`, `ProtectSystem=strict`,
> `PrivateNetwork=yes` y `RestrictAddressFamilies=AF_UNIX` (el aislamiento del unit
> de la §3.7). `kidletime 6.29` está instalado, pero es una biblioteca C++ y no
> se usa: el nombre `org.freedesktop.ScreenSaver` lo sirve **ksmserver** (`busctl
> --user status org.freedesktop.ScreenSaver` → `Comm=ksmserver`), y a ese objeto se le
> habla por D-Bus. `zbus 5.11` ya está en el
> `Cargo.lock` (lo trae `tauri-plugin-single-instance`), así que el árbol de
> dependencias no crece. `/proc/<pid>/stat` da `tpgid` en el campo 8 y
> `/proc/<tpgid>/cwd` se lee sin problema (ejemplo real: pty 34817, primer plano
> `claude`, con un cwd legible; ese cwd no era este repo, así que el ejemplo prueba la
> lectura, no la atribución).

#### 3.10.1 Modelo de datos (pasos 4 y 5 del esquema, commits 15 y 22)

"Sesión" en la UI quiere decir **token**; "tramo" quiere decir **fila** de
`time_tracking_sessions`. El paso 4 (commit 15, R8-1) trae todo lo de la medición por terminal;
el paso 5 (commit 22, R8-A1) trae los ajustes manuales (3.10.17). `ESQUEMA_ACTUAL = 5`.

**Tabla nueva `trabajar_sesiones`** (paso 4), una fila por token:

| Columna | Tipo | Contenido |
|---|---|---|
| `token` | `TEXT PRIMARY KEY CHECK (length(token) = 32 AND token NOT GLOB '*[^0-9a-f]*')` | 32 hex de `getrandom`. **No** es el id: los ids se reutilizan después de un restore |
| `project_id` | `INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE` | Igual que `time_tracking_sessions` (`db/schema.rs:206-217`) |
| `dueno` | `TEXT NOT NULL CHECK (dueno IN ('demonio','app'))` | Dueño **actual**. Cambia solo por los dos traspasos de 3.10.7 y **nunca** con un tramo abierto. Quién midió cada tramo lo dice el `source` del tramo, no esta columna |
| `origen` | `TEXT NOT NULL CHECK (origen IN ('boton','cli'))` | `boton` = Trabajar o "Abrir" del Dashboard |
| `estado` | `TEXT NOT NULL CHECK (estado IN ('esperando_terminal','esperando_demonio','viva_activa','viva_pausada','terminal_no_detectada','cerrada'))` | 3.10.2 |
| `creada_at` | `DATETIME NOT NULL` | Instante del clic o de `gestor abrir` que creó el token |
| `ultimo_clic_at` | `DATETIME NOT NULL` | Instante del **último** Trabajar, "Abrir" del Dashboard o `gestor abrir` sobre el token, también cuando se une a una sesión viva (D-R8-4). Al crear, `= creada_at`. Lo escriben `trabajar` y `sesion iniciar` en la misma transacción `IMMEDIATE` que crea o reutiliza el token. El demonio lo relee por `data_version` y lo trata como actividad válida en ese instante (3.10.2, 3.10.4). Es la única forma de que un clic llegue al demonio |
| `adoptada_at` | `DATETIME NULL` | Cuando el demonio la tomó por primera vez. `NULL` en `dueno = app` y en `esperando_demonio`. Es frontera solo al adoptar un `esperando_demonio` (3.10.2) |
| `last_seen_at` | `DATETIME NULL` | Último sondeo en que se vio viva alguna terminal o ancla. **Solo diagnóstico y `terminada_at`**: con 2a no define ningún `ended_at` de tramo |
| `terminada_at` | `DATETIME NULL` | Al pasar a `cerrada` |
| `motivo_cierre` | `TEXT NULL CHECK (motivo_cierre IS NULL OR motivo_cierre IN (…lista de sesión de 3.10.15…))` | Por qué terminó la sesión |

Restricciones de fila: `CHECK ((estado = 'cerrada') = (terminada_at IS NOT NULL))` y
`CHECK ((estado = 'cerrada') = (motivo_cierre IS NOT NULL))`. Así no puede existir
una sesión cerrada sin motivo, ni una viva con motivo.

**Sale `ultima_actividad_at`** (hallazgo de consistencia: estaba duplicada con
`time_tracking_sessions.last_activity_at`). La única fuente es el tramo:

- con un tramo abierto, la última actividad del token es su `last_activity_at`;
- sin tramo abierto (pausada), es el `ended_at` de su último tramo, porque todo
  tramo de terminal cierra **en** la última actividad (2a, 3.10.4).

Al arrancar, el demonio la reconstruye con
`SELECT MAX(COALESCE(last_activity_at, ended_at)) FROM time_tracking_sessions WHERE terminal_token = ?`.

**Tabla nueva `trabajar_anclas`** (paso 4, solo la CLI, 3.10.16). Guarda las shells que
ya existían cuando se eligió el proyecto (**anclas**) y los **vetos** de pty. Definición
completa:

| Columna | Tipo | Contenido |
|---|---|---|
| `id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | — |
| `token` | `TEXT NULL REFERENCES trabajar_sesiones(token) ON DELETE CASCADE` | `NULL` = **veto**: el pty queda reclamado para nadie (después de `--sin-medir` o de `gestor parar`), y los procesos con una marca vieja no sostienen la sesión anterior |
| `pid` | `INTEGER NOT NULL` | Shell anclada (campo 1 de `/proc/<pid>/stat`) |
| `starttime` | `INTEGER NOT NULL` | Campo 22 de `stat`. Con `pid`, la identidad: un pid reutilizado tiene otro `starttime` |
| `tty_nr` | `INTEGER NOT NULL CHECK (tty_nr <> 0)` | Campo 7 de `stat`: el pty que se reclama |
| `multiplexor` | `TEXT NULL CHECK (multiplexor IS NULL OR multiplexor IN ('tmux','screen'))` | La shell está dentro de un panel de tmux o screen. **No sostiene** la sesión; solo aporta tipeo (3.10.2, 3.10.3, 3.10.16) |
| `desde` | `DATETIME NOT NULL` | Cuando se registró el reclamo |
| `hasta` | `DATETIME NULL` | `NULL` mientras el reclamo está registrado como vigente |
| `motivo_salida` | `TEXT NULL CHECK (motivo_salida IS NULL OR motivo_salida IN ('shell_cerrada','cambio_de_proyecto','parar','sin_medir','sesion_terminada'))` | Por qué se cerró el reclamo. `sesion_terminada`: la sesión terminó por otra vía (Parar desde la app, kill switch, papelera, restore) con la shell viva |

`CHECK ((hasta IS NULL) = (motivo_salida IS NULL))`. Índice único parcial
`(pid, starttime) WHERE hasta IS NULL` (abajo).

**Vigencia real de un reclamo.** Un reclamo (ancla o veto) cuenta solo si
`hasta IS NULL` **y** su `(pid, starttime)` está vivo (`/proc/<pid>` existe, mismo
`starttime`, estado distinto de `Z`). Los números de pty se reutilizan enseguida: sin
esta condición, un veto viejo en `/dev/pts/3` anularía una konsole nueva de Trabajar
que reciba ese pty. Quién pone `hasta` a los muertos:

- el demonio, en cada escaneo con sesiones vivas, sobre **todos** los reclamos con `hasta IS NULL` (también los vetos, que no tienen token): `motivo_salida = shell_cerrada`;
- el subcomando de la CLI, antes de escribir un reclamo nuevo, con la misma regla;
- al terminar una sesión (cualquier motivo), sus anclas todavía abiertas, en la misma transacción: `shell_cerrada` si la shell murió, `parar` o `cambio_de_proyecto` si ese fue el motivo, y `sesion_terminada` en el resto.

**Tabla nueva `trabajar_evidencia`** (paso 4, DT12): un registro por minuto con
actividad, para poder responder "¿por qué se cobró 14:37?" (auditoría de
facturación, "lo que falta").

| Columna | Contenido |
|---|---|
| `token` | `TEXT NOT NULL REFERENCES trabajar_sesiones(token) ON DELETE CASCADE` |
| `minuto` | `TEXT NOT NULL`, `'%Y-%m-%d %H:%M'` en UTC (el formateador de `time.rs` truncado) |
| `fuentes` | `INTEGER NOT NULL`, máscara de bits: **1** tipeo contado · **2** archivo contado con presencia · **4** remoto (`ssh`/`mosh`) · **8** archivo contado sin dato de presencia · **16** tipeo ignorado por estar fuera de la carpeta · **32** archivo ignorado por ausencia del usuario |
| `archivo_rel` | `TEXT NULL`: ruta **relativa a la carpeta del proyecto** del primer cambio del minuto. Nunca rutas absolutas ni nada fuera de la carpeta |
| `ausente_desde_s`, `ausente_hasta_s` | `INTEGER NULL CHECK (… BETWEEN 0 AND 59)`, con `CHECK ((ausente_desde_s IS NULL) = (fuentes & 32 = 0))`: segundo, dentro del minuto, del **primer** y del **último** cambio de archivo sin presencia (bit 32). Con esto el tramo derivado `solo_archivos` (3.10.14) tiene desde y hasta exactos al segundo, y "Cargar como ajuste" los precarga sin redondear a minutos (U1) |

`PRIMARY KEY (token, minuto)`, `WITHOUT ROWID`. Se escribe en la misma transacción
`IMMEDIATE` del volcado del tramo (≤ 1 cada 5 s), con
`INSERT … ON CONFLICT(token, minuto) DO UPDATE SET fuentes = fuentes | excluded.fuentes, archivo_rel = COALESCE(archivo_rel, excluded.archivo_rel), ausente_desde_s = COALESCE(MIN(ausente_desde_s, excluded.ausente_desde_s), ausente_desde_s, excluded.ausente_desde_s), ausente_hasta_s = COALESCE(MAX(ausente_hasta_s, excluded.ausente_hasta_s), ausente_hasta_s, excluded.ausente_hasta_s)`
(el `MIN`/`MAX` de varios argumentos de SQLite da `NULL` si alguno lo es, de ahí el
`COALESCE`; el test del UPSERT cubre las cuatro combinaciones de nulos).

**Tabla nueva `archivos_evidencia`** (paso 4, U1): la misma idea para el tiempo
automático, que no tiene token. Un registro por proyecto y minuto en el que hubo un
cambio de archivo **no cobrado por presencia** (bit 32) o **cobrado sin dato de
presencia** (bit 8, U2). Los minutos cobrados con presencia no se guardan acá: ya
están en el tramo `file_watch`.

| Columna | Contenido |
|---|---|
| `project_id` | `INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE` |
| `minuto` | Igual que en `trabajar_evidencia` |
| `fuentes` | `INTEGER NOT NULL CHECK (fuentes <> 0 AND (fuentes & ~40) = 0)`: solo bits **8** y **32** |
| `archivo_rel` | Igual que en `trabajar_evidencia` |
| `ausente_desde_s`, `ausente_hasta_s` | Igual que en `trabajar_evidencia` |

`PRIMARY KEY (project_id, minuto)`, `WITHOUT ROWID`, mismo UPSERT. La escribe el bucle
del demonio (48) con el escritor de `file_watch` (43), en la transacción del volcado o,
sin tramo abierto, con el latido de 60 s (como la evidencia de un token pausado). Un
cambio sin presencia en la carpeta de un proyecto con Trabajar vivo queda en las dos
tablas; el desglose las une por proyecto y minuto, así que no cuenta doble.
También se registran los minutos **ignorados** (bits 16 y 32) de un token pausado,
porque son justo los que explican una pausa. Un token pausado no tiene tramo que
volcar: su evidencia se acumula en memoria y se escribe con la persistencia de
`last_seen_at` (cada 60 s en `viva_pausada` y `terminal_no_detectada`, y al recibir
SIGTERM, 3.10.10), en la misma transacción. Un corte sucio pierde, como mucho, 60 s de
evidencia ignorada; nunca evidencia de un minuto cobrado, que viaja con su volcado. Volumen: una fila por minuto con
actividad. Con 8 h diarias son 480 filas y ~120.000 por año, un tamaño despreciable.
Las marcas del desglose (3.10.14) se **derivan** de esta tabla y no se guardan en el
tramo: hay una sola fuente.

**Columnas nuevas en `time_tracking_sessions`** (paso 4):

| Columna | Para qué |
|---|---|
| `terminal_token TEXT NULL REFERENCES trabajar_sesiones(token)` | El tramo es de esa sesión. `NULL` en `file_watch` y en las filas viejas |
| `motivo_cierre TEXT NULL CHECK (motivo_cierre IS NULL OR motivo_cierre IN (…lista de tramo de 3.10.15…)) CHECK (ended_at IS NULL OR terminal_token IS NULL OR motivo_cierre IS NOT NULL)` | Por qué se cerró el tramo. `NULL` solo en filas abiertas y en las anteriores al paso 4. El segundo `CHECK` exige motivo a todo tramo cerrado de una sesión (abajo) |

**`source = 'terminal'`** (valor nuevo; hallazgo de consistencia "falta el `source`"):
son los tramos que mide el **demonio** para un token. Los tramos de un token
`dueno = app` siguen siendo `work_button`, con su `terminal_token`. Consecuencias:

- `duracion_a_guardar('terminal', s) = s`: los tramos `terminal` no usan la regla de los 10 s (DT5). Se agrega un caso al test `duracion_a_guardar_aplica_los_10_s_salvo_a_file_watch` (`commands/mod.rs:2257`).
- La recuperación de huérfanas de la app (§3.2) **excluye** `source = 'terminal'`: las recupera el demonio (3.10.2, "arranque").
- Un tramo de 0 s se borra, igual que en `file_watch`.

**Tramo de sesión con motivo: `CHECK`, no trigger.** La regla "todo tramo cerrado con
`terminal_token` tiene motivo" va como `CHECK` de la columna `motivo_cierre` (tabla de
arriba), agregada con `ADD COLUMN`. La base real tenía **197** filas en
`time_tracking_sessions` el 2026-10-19, cuatro más que las 193 de §2.1 (medidas el
2026-10-18): 194 `work_button`, 2 `shell_hook` y 1 `manual`, con **116
cerradas** sin motivo y **81 abiertas** (las huérfanas). El `CHECK` las acepta a todas
porque tienen `terminal_token IS NULL`, y rechaza tanto un `UPDATE` como un `INSERT`
nuevos inválidos (un trigger `BEFORE UPDATE OF ended_at` no habría cubierto el
`INSERT`). Verificado con sqlite 3.53.4 sobre una copia de la base real (`.backup` a una
carpeta temporal): migración aceptada, `UPDATE` y `INSERT` sin motivo → `CHECK constraint
failed`. Para el resto de las fuentes lo garantiza la firma
`cerrar_sesion(id, fin, dur, motivo)` (commit 15, R8-1).

**Las 81 huérfanas reales y la recuperación del 19.** Se probó sobre la misma copia:
con la regla de §3.2 (cierre en `COALESCE(last_activity_at, started_at)` y
`MIN(COALESCE(duration_seconds, 0), …)`), las 81 cierran con 0 s y motivo `huerfana`,
el total de segundos no cambia (1.352.723 s antes y después) y no se toca ninguna fila
de `trabajar_sesiones` (no tienen token). El commit 24 lo suma como test `#[ignore]`
sobre `GESTOR_GOLDEN_DB`, junto al diff con datos reales: necesita la API de backup que
activa ese commit y la recuperación del 19.

**Columnas nuevas en `tracker_status`** (paso 4): `trabajar_idle_minutes`, más `presencia TEXT` (`kde`, `gnome`, `prueba` o `no_disponible`),
`presencia_motivo TEXT NULL` y `presencia_at DATETIME`. Las publica el demonio (3.10.4) y
las muestra Ajustes (3.10.14).

**Índices** (todos parciales, soportados desde SQLite 3.8):

- `CREATE UNIQUE INDEX … ON trabajar_sesiones(project_id) WHERE estado <> 'cerrada'`: a lo sumo una sesión viva por proyecto (D-R8-2, D-R8-4).
- `CREATE UNIQUE INDEX … ON time_tracking_sessions(terminal_token) WHERE ended_at IS NULL`: a lo sumo **un** tramo abierto por token. Es único, no solo un índice: dos tramos abiertos del mismo token serían doble escritor.
- `CREATE UNIQUE INDEX … ON trabajar_anclas(pid, starttime) WHERE hasta IS NULL`.

**Invariantes de los tramos `terminal` (DT1, auditables con SQL):**

- Cerrado: `ended_at − started_at = duration_seconds` y `last_activity_at = ended_at`.
- Abierto: `last_activity_at − started_at = duration_seconds`.
- Ningún tramo cruza una pausa, una suspensión ni un salto de reloj: el motor lo parte antes (3.10.2, 3.10.5).
- Con 2a, todo cierre es **en la última actividad**, así que el número que se ve mientras el tramo está abierto es exactamente el que queda al cerrarlo.
- **Segundos enteros.** Los eventos de archivo traen subsegundos y las columnas guardan segundos. El escritor formatea primero los instantes (`time.rs`, truncando) y calcula `duration_seconds` **desde los valores ya formateados**. Si no, las invariantes darían ±1 s.

Las consultas SQL de estas invariantes están en §6 (Definición de listo).

**Por qué partir y no un estado "pausada" dentro de la fila:** con una fila que
acumulara solo el tiempo activo, `duration` sería menor que `ended − started`, y la
unión (§3.3) la leería como un tramo **desplazado** que cuenta doble con
`file_watch`. Ejemplo: una terminal de 9 a 18 con una suspensión de 12 a 14 se leería
como el tramo 9–16, y un `file_watch` de 16 a 18 quedaría afuera: 9 h en vez de 7.

**Filas `dueno = app`** (sin demonio): "como hoy" (`ended − started ≥ duration`,
`Instant`, regla de los 10 s). 2a no se les puede aplicar, porque sin demonio no hay
señal de actividad: el tramo mide hasta Parar, hasta Trabajar en otro proyecto o hasta
cerrar la app, con un **tope de 12 h** (`duration_seconds ≤ 43200`, motivo
`tope_sin_tracker`, U6), y el desglose lo marca `sin_vigilancia`. Es una excepción
declarada a D-R8-3 y 2a (§1.2). Mientras la app la mide, la sesión está en `viva_activa` con su
tramo `work_button` abierto (3.10.2). En los dos últimos casos se cierra el **tramo**,
pero la **sesión** no termina: pasa a `esperando_demonio` (traspaso de 3.10.7, D-R8-1).
Solo termina ahí donde nadie la va a poder adoptar (3.10.7, "adopción imposible").

#### 3.10.2 Máquina de estados de una sesión

**Estados** (nombres canónicos). "Viva" es el conjunto
`{esperando_terminal, esperando_demonio, viva_activa, viva_pausada, terminal_no_detectada}`.

| Estado | Qué significa | ¿Tramo abierto? |
|---|---|---|
| `esperando_terminal` | Token `dueno = demonio` creado con el demonio vivo (o ya adoptado) que todavía no vio ninguna terminal ni ancla | No |
| `esperando_demonio` | Token `dueno = demonio` creado sin demonio vivo (CLI sin tracker; corrección aprobada en la revisión de R8) o traspasado desde `dueno = app` (cierre de la app, cambio de proyecto, huérfana o tope; 3.10.7). Nadie mide todavía | No |
| `viva_activa` | `dueno = demonio`: hay terminal o ancla viva y actividad válida hace menos de `pausa`. `dueno = app`: la app la está midiendo (su `MedicionApp`), con su tramo `work_button` abierto; sin pausa ni detección de cierre, con tope de 12 h (3.10.13, U6) | Sí |
| `viva_pausada` | Hay terminal o ancla viva, pero pasaron ≥ `pausa` sin actividad válida, o hubo suspensión, salto de reloj, readopción o un SIGTERM con el tracker encendido | No |
| `terminal_no_detectada` | Venció la gracia sin terminal. Mide por **archivos con presencia** y, si la sesión tiene un ancla de multiplexor (3.10.16), también por el **tipeo de ese panel**. Se **pausa** igual que una viva (DT6 corregida). El tramo puede estar abierto o no | A veces |
| `cerrada` | Terminó. `motivo_cierre` de sesión (3.10.15) | No |

**Tiempos** (de test por `GESTOR_TRACKERD_TEST_TIMINGS`, con claves en el **mismo
idioma** que las de la §5.2 (`idle=`, `tick=`, `flush=`…): `poll=` (sondeo),
`grace=` (gracia) y `pause=` (pausa). El parser rechaza claves desconocidas, auditoría
de pruebas). Por defecto: sondeo 5 s, gracia 30 s y
`pausa = tracking.trabajar_idle_minutes` = 10 min (1–120). La ventana de **presencia**
no es `pausa`: es una constante de 10 min (decisión 1a, literal), 3.10.4.

**Relojes.** Las marcas que se guardan son de pared. La **gracia** se mide con el
reloj monotónico desde que el demonio toma el token (adopción o readopción), así que
una suspensión o un reinicio del demonio no la consumen. La pausa se mide en pared,
porque compara instantes guardados.

**Frontera inicial de un token** (corrige "el clic nunca se usa"):

- Token que nace con el demonio vivo (`esperando_terminal` desde la creación): `creada_at`. El clic que lo creó es actividad válida y el primer tramo empieza ahí (E23).
- Token adoptado desde `esperando_demonio`: `adoptada_at`. Lo anterior no se inventa.
- Token readoptado al arrancar el demonio (3.10.10): el arranque.

**Frontera de un token:** el máximo entre su frontera inicial, el último
`reanudado`/`reloj` y el `ended_at` de su tramo anterior. Ningún tramo empieza antes.

**Actividad válida** (detalle en 3.10.4): tipeo en un pty del token con el primer plano
dentro de la carpeta (o remoto), un cambio de archivo en la carpeta **con presencia**
del usuario, o un **clic** sobre ese token, que el demonio ve como un `ultimo_clic_at`
nuevo (3.10.1). Un clic anterior a la frontera no cuenta. Desde `viva_pausada`, el tipeo
reanuda solo con **dos** cambios de `atime` en 60 s (3.10.4); un archivo o un clic
alcanzan con uno.

**Actividad durante la gracia.** En `esperando_terminal` el motor no tiene tramo, pero
anota la última actividad válida que ve (un archivo con presencia, otro clic). El
tramo que se abre al ver la terminal o al vencer la gracia empieza en
`max(ultimo_clic_at, frontera)` y nace con esa UA, así que un guardado durante la
gracia queda adentro.

**Transiciones.** "Cierra en UA" quiere decir que el tramo abierto se cierra con
`ended_at = última actividad` (2a). No hay excepciones: Parar, el cierre de la
terminal y el kill switch tampoco cobran la cola.

| Desde | Evento | Hacia | Efecto en la base |
|---|---|---|---|
| — | Trabajar o `gestor abrir` **con** demonio vivo, sin sesión viva en el proyecto | `esperando_terminal` | Fila del token con `ultimo_clic_at = creada_at`; `adoptada_at` la pone el demonio en ≤ 1 sondeo (relectura por `data_version`). Frontera inicial = `creada_at` |
| — | `gestor abrir` **sin** demonio vivo | `esperando_demonio` | Fila del token con su ancla. La CLI avisa "se mide desde que arranque el tracker" |
| — | Trabajar sin demonio vivo | `viva_activa` con `dueno = app` | Fila del token y tramo `work_button` abierto en el clic, que mide la app con su `Instant` (3.10.7, 3.10.13). Sin pausa ni detección de cierre (U6). Toast, solo si `adopcion_imposible()` es falso: "El tracker no está corriendo: esta sesión no se pausa por inactividad y se corta sola a las 12 h. Tocá Parar al terminar." (con adopción imposible no hay toast: lo dicen la tarjeta y la línea de cabecera, 3.10.14). De acá sale solo por Parar (`cerrada`), por el traspaso `app → demonio` (`esperando_demonio`), por el tope de 12 h (fila siguiente) o, con adopción imposible, por `cierre_app`/`cambio_de_proyecto`/`huerfana` (`cerrada`) (U4) |
| `viva_activa` con `dueno = app` | El latido de 60 s ve `started_at + 12 h ≤ ahora` (`TOPE_SIN_TRACKER`, U6) | `esperando_demonio` (o `cerrada` con adopción imposible) | Cierra el tramo en `started_at + 12 h` con `duration = 43200` y motivo de tramo `tope_sin_tracker`; en la misma transacción, traspaso `app → demonio` (3.10.7) o, con adopción imposible, la sesión termina con `tope_sin_tracker`. Si la terminal sigue viva cuando vuelva el demonio, la adopta como cualquier `esperando_demonio` (U4) |
| cualquiera viva | Trabajar o `gestor abrir` sobre el mismo proyecto (D-R8-4) | igual | Mismo token; `ultimo_clic_at = ahora`. En `viva_pausada` o `terminal_no_detectada` sin tramo, el demonio abre uno en `max(ultimo_clic_at, frontera)`; en `esperando_demonio` el clic queda antes de la frontera y no cuenta. Si es Trabajar con `dueno = demonio` y `demonio_vivo()` da falso, rige el traspaso `demonio → app` de 3.10.7 (pasa a `viva_activa`, `dueno = app`) |
| `esperando_demonio` | El demonio la adopta y hay terminal o ancla viva | `viva_pausada` | `adoptada_at = ahora` = frontera. No se abre tramo: el primer tipeo o archivo posterior lo abre. **El tiempo previo no se inventa** |
| `esperando_demonio` | El demonio la adopta, no hay nada vivo y el token tiene anclas (CLI) o el demonio ya lo había visto (`last_seen_at` no nulo) | `cerrada` | `tracker_caido` ("el tracker no estaba corriendo cuando se cerró la terminal"), `terminada_at` por la regla única de 3.10.15. No se abre tramo. Esta tabla rige también al arrancar (3.10.10 deriva acá todo `esperando_demonio`) |
| `esperando_demonio` | El demonio la adopta, no hay nada vivo, no tiene anclas y nunca se vio (terminal del botón que medía la app) | `esperando_terminal` | Gracia completa desde la adopción; si vence, `terminal_no_detectada` (pausa, sale con Parar). No se cierra a ciegas: puede ser una terminal de instancia única |
| `esperando_terminal` | Actividad válida en `t` (archivo con presencia, otro clic) | igual | Se anota como UA candidata del tramo que va a abrirse |
| `esperando_terminal` | El sondeo ve la terminal antes de que venza la gracia | `viva_activa` | Abre tramo en `max(ultimo_clic_at, frontera)`, con la UA anotada durante la gracia (o el clic): el clic es actividad |
| `esperando_terminal` | Vence la gracia (monotónica) | `terminal_no_detectada` | Abre tramo en `max(ultimo_clic_at, frontera)`, con la UA anotada. Si después no hay actividad, queda en 0 s y se borra |
| `esperando_terminal`, `esperando_demonio` | Parar (app o CLI) | `cerrada` | `parar_app`/`parar_cli`. No hay tramo, o hay uno de 0 s que se borra |
| `viva_activa` | Actividad válida en `t` | igual | `last_activity_at = t`, `duration = t − started_at`, volcado ≤ 1 cada 5 s, evidencia del minuto |
| `viva_activa` | `ahora − UA ≥ pausa` | `viva_pausada` | Cierra en UA, motivo `inactividad` |
| `viva_pausada` | Actividad válida en `t` (con la regla de los dos `atime`) | `viva_activa` | Abre tramo en `max(t₁, frontera)`, donde `t₁` es la **primera** de las dos señales. Sin margen |
| `viva_activa`, `viva_pausada` | El sondeo ya no ve ninguna terminal ni ancla | `cerrada` | Cierra en UA (si había tramo); `terminal_cerrada`, `terminada_at` por la regla única de 3.10.15. **Cierre condicional sin carrera** (abajo) |
| `terminal_no_detectada` | Archivo con presencia en `t` | igual | Extiende el tramo abierto o abre uno en `max(t, frontera)` |
| `terminal_no_detectada` | Tipeo válido en `t` en el pty de un **ancla de multiplexor** del token (3.10.16), con la regla de los dos `atime` si no hay tramo | igual | Extiende el tramo abierto o abre uno en `max(t₁, frontera)`. El ancla de multiplexor aporta tipeo pero no sostiene: la sesión sigue sin terminal detectada y sale con Parar (E36) |
| `terminal_no_detectada` | `ahora − UA ≥ pausa` con tramo abierto | igual | Cierra en UA, motivo `inactividad`. **La sesión sigue** |
| `terminal_no_detectada` | La terminal aparece tarde | `viva_activa` o `viva_pausada` | Sigue el tramo abierto, si lo hay; si no, queda `viva_pausada` |
| cualquiera viva con tramo | `reanudado` (suspensión, 3.10.5) | `viva_pausada` (o `terminal_no_detectada`) | Cierra en UA, `suspension`; frontera = despertar |
| cualquiera viva con tramo | Salto de reloj sin suspensión (3.10.5) | `viva_pausada` (o `terminal_no_detectada`) | Cierra en UA, `reloj`; frontera = ahora |
| cualquiera viva | Parar (app o `gestor parar`) | `cerrada` | Cierra en UA; `parar_app` o `parar_cli` |
| cualquiera viva | Kill switch (corrección aprobada en la revisión de R8) | `cerrada` | Cierra en UA; `tracker_apagado`. El demonio lo hace al releer la config, **también** al recibir SIGTERM; si no llegó, lo hace la app |
| cualquiera viva | Proyecto (o un ancestro suyo) a la papelera (corrección aprobada en la revisión de R8) | `cerrada` | Cierra en UA; `proyecto_a_papelera`. Lo hace `delete_project` en su misma transacción, para el proyecto y sus descendientes (3.10.9, commit 18 (R8-3b)); el demonio lo repite al releer (idempotente) |
| cualquiera viva | Restore (corrección aprobada en la revisión de R8; variante elegida en 3.10.9: se cortan con aviso) | `cerrada` | Cierra en UA; `restauracion` |
| `viva_activa` | SIGTERM con `daemon_enabled = true` | `viva_pausada` (DT7) | Cierra el tramo en UA con motivo de tramo `tracker_detenido`. La sesión no termina. Queda en `viva_pausada`, que es como la deja el rearranque y respeta "`viva_activa` ⇒ tramo abierto" |
| `terminal_no_detectada` con tramo | SIGTERM con `daemon_enabled = true` | igual, sin tramo | Cierra el tramo en UA con `tracker_detenido` |
| `viva_*`, `terminal_no_detectada` | `gestor abrir Y` (o `--sin-medir` hacia otro proyecto) saca la última ancla o terminal de X (3.10.16) | `cerrada` | Cierra en UA; `cambio_de_proyecto` (Enmienda 2) |
| `viva_*`, `terminal_no_detectada` | `gestor abrir --sin-medir X` (el mismo proyecto) saca la última ancla o terminal de X (3.10.16) | `cerrada` | Cierra en UA; `parar_cli`: el usuario pidió dejar de medir X en esa terminal |

**Cierre por `terminal_cerrada` sin carrera con un clic o una unión.** Puede pasar que
el demonio escanee, no vea terminales y, en la misma ventana, Trabajar reutilice el
token (o la CLI se una) y lance o ancle una terminal nueva con un token que ya quedó
`cerrada`. Para que eso no pase:

- El demonio cierra con `terminal_cerrada` en una transacción `IMMEDIATE` **condicional**: `WHERE estado <> 'cerrada' AND ultimo_clic_at <= :escaneo AND NOT EXISTS (SELECT 1 FROM trabajar_anclas WHERE token = :t AND hasta IS NULL AND desde > :escaneo)`, donde `:escaneo` es el instante en que empezó el escaneo que no vio nada. Si no se cumple, no cierra y re-escanea en el sondeo siguiente.
- Del lado de `trabajar` y de `sesion iniciar`: la reutilización del token y la escritura de `ultimo_clic_at` (o del ancla) van en la misma transacción que comprueba `estado <> 'cerrada'`. Si después del spawn el token figura `cerrada` (perdió la carrera por milisegundos), `trabajar` crea un token nuevo y relanza la terminal con esa marca; la CLI reintenta `iniciar` una vez.
- Test en el commit 44 (R8-6) (dos conexiones, el clic entre el escaneo y el cierre) y E28 extendido.

**Arranque del demonio** (lo detalla 3.10.10; acá van las transiciones). Si
el demonio no salió limpio, al arrancar encuentra `tracker_status.pid` distinto de
`NULL` y ese pid ya no es un `gestor-trackerd` vivo. Puede ser un `kill -9`, un corte
de luz o un unit en `failed`.

| Al arrancar | Terminal viva | Terminal muerta |
|---|---|---|
| Salida **sucia** con tramo abierto | Cierra el tramo en su `last_activity_at` con `tracker_caido` → `viva_pausada` | Cierra el tramo con `tracker_caido` → sesión `cerrada/tracker_caido`, `terminada_at` por la regla única de 3.10.15 |
| Salida sucia sin tramo abierto | `viva_pausada` | `cerrada/tracker_caido` |
| Salida **limpia** (SIGTERM, `pid = NULL`) | `viva_pausada` (readopción, el hueco no suma) | `cerrada/terminal_cerrada` en `last_seen_at` (caso normal: se cerró la sesión del escritorio y murieron las terminales) |
| `esperando_terminal` con la gracia vencida | — | `terminal_no_detectada` (gracia monotónica desde la readopción: se le da otra gracia completa) |
| `esperando_demonio` (con o sin `last_seen_at`, salida limpia o sucia) | Adopción, como en la tabla de arriba | Adopción, como en la tabla de arriba (`tracker_caido` si hay prueba de cierre). Las filas de salida limpia/sucia **no** se le aplican: nadie miraba cuando se cerró la terminal |

Corrige el hallazgo "kill -9 con la terminal muerta termina con
`fin_de_sesion_escritorio`": ahora es `tracker_caido`. Además, `fin_de_sesion_escritorio`
desaparece: en sesión es `terminal_cerrada` o `tracker_caido`, y en tramo,
`tracker_detenido`.

**Tracker caído de forma permanente** (unit `failed` o latido vencido con `pid` no
`NULL`, auditoría de consistencia): la sesión queda **viva en la base**, con el tramo
abierto quieto en su última actividad. La unión cuenta hasta ahí y no inventa nada.
La tarjeta lo dice (3.10.14). Parar desde la app la cierra en UA con `parar_app`, aunque
el demonio no esté.

**Implementación y pruebas.** El motor puro (commit 42, R8-5, lib `trackerd::terminales`) recibe:

- `muestra(reloj: Reloj { pared, mono, boot }, vivos_por_token, ptys_por_token)`
- `cambio(proyectos_que_contienen, t, presencia)`
- `clic(token, ultimo_clic_at)`, cuando la relectura por `data_version` trae un `ultimo_clic_at` nuevo
- `tick(reloj)`
- `parar(token, origen)`
- `config(pausa, gracia)`
- `estado_base(token, …)`

Devuelve las acciones `AbrirTramo`, `VolcarTramo { evidencia }`,
`CerrarTramo { motivo }`, `PasarEstado` y `Terminar { motivo }`. Se prueba con una
tabla de transiciones, reloj y muestras falsos: una fila por transición de esta
tabla, más las de 3.10.4 y 3.10.5. El escritor (commit 44, R8-6) las aplica con los métodos
condicionales del commit 15 (R8-1).

#### 3.10.3 Detección de procesos (commit 40, R8-4, `trackerd::procesos` en la lib)

**Antes de detectar: cómo llega la marca (commit 16, R8-2).** La detección depende de que
la terminal nazca con la marca y de que nada más la herede por accidente.

- **Firma.** `open_terminal(path, config, marca: Option<&MarcaSesion>)` cambia junto en el trait (`platform/mod.rs:18`), en `linux.rs:89` y en `windows.rs:95`. `MarcaSesion` es un newtype que solo se construye desde un token que cumple `^[0-9a-f]{32}$`. Windows la recibe y la ignora.
- **Quién pasa marca.** Solo `trabajar` (commit 17, R8-3). El comando `open_terminal` (`commands/mod.rs:234`), que usan `api.ts:79`, el store y el atajo de `App.tsx:178`, pasa `None`: abrir una terminal sin Trabajar no mide.
- **Con marca:** `Command::env("GESTOR_SESION", T)` **y** el argv `/usr/bin/env GESTOR_SESION=T $SHELL` de la tabla de flags (abajo). La marca es **un** elemento del argv, nunca parte de un string de shell. **Sin marca:** `env_remove("GESTOR_SESION")`. En las dos, `current_dir(P)` en todas las ramas.
- **Herencia accidental.** La primera línea de `main()` es `std::env::remove_var("GESTOR_SESION")`, **antes de `logging::init()`** (`main.rs:25`). Ahí todavía no hay hilos, y en edition 2021 (`src-tauri/Cargo.toml:6`) no hace falta `unsafe`. Corta la herencia cuando la app se lanza desde una terminal marcada, por ejemplo con `pnpm run tauri:dev` desde una konsole de Trabajar. El `env_remove` de cada spawn queda como segunda barrera.
- **Modos de terminal.**
  - `Auto` recorre `try_terminal_fallback`. Es el modo del usuario: `~/.config/gestor-proyectos/config.json` tiene `"mode": "auto"`.
  - `Default` llama a `x-terminal-emulator`, que **en esta máquina no existe** (`command -v` no lo encuentra). El spawn falla y cae al fallback, así que acá `Default` hace lo mismo que `Auto`. Donde existe, lleva solo `Command::env`, como `Custom`.
  - `Script` usa `execute_script` (`linux.rs:266`), que también usan el navegador (`:193`) y el editor (`:245`). Al trait **no** se le agrega un parámetro. En `linux.rs` se separa un helper privado, `comando_script(script, vars) -> Command`: `execute_script` lo lanza tal cual y la rama `Script` de `open_terminal` le agrega la marca. Así la marca nunca llega a un navegador ni a un editor.
- **Zombis.** Hoy todas las ramas hacen `spawn()` y descartan el `Child`. `try_terminal_fallback` lo descarta dentro de la condición (`.spawn().is_ok()`, `linux.rs:16-82`), y en esta máquina se vieron 3 `[konsole] <defunct>` colgando de la app (relevado el 2026-10-19).
  - **Corrección:** el fallback pasa a `match cmd.spawn()`. Con `Ok(child)` llama a `esperar_en_hilo(child)` (un hilo con nombre que hace `child.wait()` y termina) y devuelve `Ok(())`. Con `Err` prueba la terminal siguiente.
  - Todos los spawns de `linux.rs` usan el mismo helper `lanzar(cmd)`, porque `xdg-open` también deja zombis.
  - **Descartado:** `SIGCHLD = SIG_IGN`. La app corre `git` con `Command::output()` (14 veces en `commands/mod.rs`), y con `SIG_IGN` el `waitpid` de esas llamadas fallaría con `ECHILD`.
- **`windows.rs` tiene que compilar.** Cambia la firma del trait, pero `windows.rs` no se compila en Linux (`platform/mod.rs:3-4`). El target `x86_64-pc-windows-gnu` y `x86_64-w64-mingw32-gcc` ya están instalados en esta máquina, pero **las dependencias de Windows no están en la caché de cargo**: `cargo check --offline --locked --target x86_64-pc-windows-gnu` falla al bajar `anstyle-wincon v3.0.11` (`attempting to make an HTTP request, but --offline was specified`, salida 101). Por eso:
  - el chequeo de Windows **no** entra en el `scripts/check.sh` por defecto (que tiene que andar sin red); va como `scripts/check.sh --windows`, que corre `cargo check --locked --target x86_64-pc-windows-gnu` y falla con un mensaje claro si falta la caché;
  - llenar la caché necesita red **una vez** (`cargo fetch --locked --target x86_64-pc-windows-gnu`). Es un paso manual que corre el usuario;
  - el lugar natural del chequeo es la CI (DX5, job de Linux con ese target, o un runner de Windows). Hasta que exista, el commit 16 (R8-2) deja pegada en el PR la salida de `check.sh --windows` corrida a mano;
  - que las dependencias de Tauri compilen para ese target sigue **sin verificar** hasta ese primer `fetch`.

**Tabla de flags** (relevada el 2026-10-19 en las fuentes de cada terminal y, para
konsole 26.08.0 y alacritty 0.17.0, también en esta máquina). `argv_terminal` la arma;
la marca va siempre como **un** elemento `GESTOR_SESION=T` después de `/usr/bin/env`, y
`Command::env` la pone además en el entorno del proceso:

| Terminal | ¿Delega en una instancia existente? | ¿Llega el entorno del `Command` a la shell? | argv |
|---|---|---|---|
| konsole | No por defecto (`UseSingleInstance=false`); sí con esa opción | Sí en multiproceso (verificado: las pestañas nuevas heredan); no en instancia única | `konsole --separate --workdir P -e /usr/bin/env GESTOR_SESION=T $SHELL` |
| gnome-terminal | Sí, siempre (`gnome-terminal-server`) | Sí: el cliente le manda su entorno al server | `gnome-terminal --working-directory=P -- /usr/bin/env GESTOR_SESION=T $SHELL` |
| alacritty | No | Sí | `alacritty --working-directory P -e /usr/bin/env GESTOR_SESION=T $SHELL` |
| kitty | No (salvo `--single-instance`, que no se usa) | Sí | `kitty --directory P /usr/bin/env GESTOR_SESION=T $SHELL` |
| xfce4-terminal | Sí por defecto (servidor D-Bus) | No con el server: solo viaja el argv | `xfce4-terminal --disable-server --working-directory=P -x /usr/bin/env GESTOR_SESION=T $SHELL` |
| tilix | Sí (aplicación única) | No cuando delega | `tilix --new-process -w P`, con `Command::env` (su `-e` recibe un string de shell: no se usa) |
| xterm | No | Sí | `xterm -e /usr/bin/env GESTOR_SESION=T $SHELL`, con `current_dir(P)` |
| Custom y `Default` | Depende del programa | Solo si es un proceso nuevo | `Command::env` y `current_dir`; la limitación se documenta |

`$SHELL` va sin `-l`: konsole lanza una zsh interactiva no-login, y `-l` cambia qué rc
se cargan. Las terminales que no pasan el entorno caen en la gracia y en
`terminal_no_detectada` (3.10.2). El golden del commit 16 fija estos argv.

**Criterio para que un proceso cuente.** Un proceso es de la sesión T si cumple todo esto:

1. **UID propio.** Está en `/proc/[0-9]+` y el UID del directorio es el propio.
   - Con `PrivateUsers=yes`, el UID propio se mapea igual (1000 → 1000) y los ajenos aparecen como `nobody`, así que el filtro sigue andando.
   - Poder **leer** `environ` y `cwd` de procesos de fuera del unit depende de `ptrace_may_access` con el mismo kuid. Es plausible, pero no está probado. Por eso E16 es obligatorio (al final de esta sección).
2. **`stat` se parsea después del último `)`.** El campo 2 (`comm`) puede tener espacios y paréntesis. Verificado: el server de tmux se llama `tmux: server`, y su `stat` empieza con `3147793 (tmux: server) S …`.
   - `comm` es lo que está entre el primer `(` y el **último** `)`. Los campos 3 en adelante salen de partir por espacios lo que sigue.
   - Se usan los campos 3 (estado), 4 (`ppid`), 5 (`pgrp`), 6 (sesión), 7 (`tty_nr`), 8 (`tpgid`) y 22 (`starttime`). Verificados en la zsh 21522 de konsole: `S`, 21510, 21522, 21522, 34817, 460508 y 58123.
   - Con estado `Z` o `X` no cuenta: un zombi no tiene `mm` y su `environ` está vacío. En esta máquina hay zombis `zsh <defunct>` reales.
3. **Tiene tty.** `tty_nr ≠ 0`. Cuando muere el líder de sesión, el kernel les quita la tty de control a todos los procesos de su SID, así que un `nohup`, un `setsid`, `node` o Chrome dejan de contar.
4. **Tiene la marca de un token vivo.** Su `environ` tiene **exactamente** `GESTOR_SESION=<T>`, `T` cumple `^[0-9a-f]{32}$` **y** la base tiene ese token en un estado distinto de `cerrada`. Un token que la base no conoce se ignora, por ejemplo uno de antes de una restauración (3.10.9). El `environ` se lee solo si pasaron 1–3, porque `stat` es más barato.
5. **No desciende del server de un multiplexor.** Tampoco puede ser el propio demonio. Se recorre la cadena de `ppid` con el mapa `pid → (ppid, comm, tty_nr)` que ya se armó con los `stat`, buscando `comm = "tmux: server"`, o `comm = "screen"` con `tty_nr = 0`. Verificado acá:
   - El server de tmux tiene `tty_nr = 0`, y cada panel tiene su propio pty.
   - Con `screen -dmS`, el server es `comm = screen` (su argv dice `SCREEN`) con `tty_nr = 0`, y su hijo tiene pty.
   - El `tty_nr = 0` separa al server del **cliente** de screen, que también se llama `screen` pero conserva la tty de la pestaña. El cliente (`tmux attach`, `screen -r`) **sí** cuenta, así que la sesión dura lo que la pestaña.
   - Queda para la matriz manual el arranque interactivo de `screen` (sin `-dm`).
6. **Su pty no está reclamado por otro token.** Ningún ancla ni veto de otro token reclama ese pty (3.10.16). Rige el reclamo más reciente **vigente**: `hasta IS NULL` **y** su `(pid, starttime)` vivo (3.10.1, "Vigencia real de un reclamo"). Un reclamo cuya shell murió no cuenta, aunque todavía no tenga `hasta`: los números de pty se reutilizan, y un veto viejo no puede anular la konsole nueva que reciba ese pty. Un veto (`token = NULL`) vigente reclama el pty para nadie.

**Anclas (CLI).** El ancla `(pid, starttime)` de T está viva si se cumplen tres cosas: `/proc/<pid>` existe, su `starttime` es el guardado y su estado no es `Z`.

- Un **ancla de multiplexor** (la shell está dentro de tmux o screen, 3.10.16) **no sostiene** la sesión, porque sobrevive al cierre de la ventana. Su pty sí aporta tipeo.
- **La sesión T está viva** si tiene algún proceso que cumpla 1–6, **o** algún ancla viva que no sea de multiplexor.

**Pty de cada proceso.**

- En cada escaneo se lista `/dev/pts` y se arma el mapa `st_rdev → /dev/pts/N`. `tty_nr` se compara directo contra `st_rdev`, porque usan la misma codificación. Verificado: `/dev/pts/1` tiene `rdev = 34817`, que es el `tty_nr` de la zsh 21522.
- Reemplaza a la fórmula `(major − 136) × 256 + minor`, que dependía del rango de majors.
- El mismo `stat` del pty da el `atime` (3.10.4).
- Un `tty_nr` que no está en el mapa (una consola virtual, un devpts raro) mantiene viva la sesión, pero no aporta actividad. Se anota en `last_error` (riesgo 45).

**Primer plano de cada pty (decisión 3a).** El tipeo cuenta para el proyecto solo si el programa que lo recibe está en la carpeta del proyecto. Por eso la muestra trae qué hay en primer plano en cada pty:

- **Qué proceso.** `tpgid` (campo 8) es el grupo en primer plano de la terminal. El proceso es el que tiene `pid = tpgid`. Si el líder ya murió, cualquiera del mapa con `pgrp = tpgid`. Con `tpgid = -1`, el líder de sesión (campo 6).
- **`PrimerPlano::Remoto { programa }`** si el `comm` está en `{ssh, mosh-client, autossh, sshpass, et}` (la misma lista de 3.10.4). El tipeo cuenta y el tramo lleva la marca `remoto_ssh` (3.10.4): la carpeta local de un `ssh` no dice nada del trabajo remoto.
- **`PrimerPlano::Multiplexor`** si el `comm` es un cliente de `tmux` o de `screen`: 3.10.4 busca el panel activo.
- **`PrimerPlano::Local { cwd }`** en cualquier otro caso, con `readlink /proc/<pid>/cwd`. Verificado: el `tpgid` de la zsh 21522 es un `claude`, y su `cwd` se lee.
- **`cwd` ilegible.** Pasa con `EACCES` (`sudo vim`, procesos con `PR_SET_DUMPABLE = 0`). Se usa el `cwd` del líder de sesión, que es la shell del usuario y sí se lee. Si tampoco se puede, queda **`PrimerPlano::Desconocido`**, y ese tipeo no cuenta (3.10.4, bit 16 de la evidencia).

Qué se hace en cada caso (dentro o fuera de la carpeta, paraguas, desconocido) lo
decide 3.10.4. Esta sección solo entrega el dato, por pty:
`PtyMuestra { ruta, atime, primer_plano }`.

**Firma pura.** Los criterios 4 (token vivo en la base) y 6 (reclamos y vetos)
necesitan datos de la base, pero la lib no la abre: los recibe.
`escanear(raiz_proc, raiz_dev, uid, propio_pid, tokens_vivos: &HashSet<Token>, reclamos: &[Reclamo]) -> Escaneo`,
con `Reclamo { token: Option<Token>, pid, starttime, tty_nr, multiplexor, desde }`. El
llamador (demonio, app en el commit 58 (R8-8), subcomando de la CLI) carga las dos cosas en la misma
lectura. `Escaneo` devuelve, además de las muestras por token, la lista de reclamos
cuya shell murió, para que el llamador les ponga `hasta` (3.10.1).

**El `cwd` no se loguea nunca**, ni en `debug`. Es la regla O2 (§3.7): ninguna ruta por evento.

**Errores.**

- `ENOENT`/`ESRCH` (el proceso murió en medio de la lectura) y `EACCES` (setuid, `PR_SET_DUMPABLE = 0`) descartan ese pid en silencio.
- **Nunca** se loguea ni se guarda el `environ`.
- Del token se loguean solo los primeros 8 caracteres.

**Costo.** Un escaneo completo cada 5 s, y **solo si hay sesiones vivas**. Sin sesiones vivas, el costo es cero.

- Medido: entre 529 y 539 PIDs, y 8 ms de reloj para leer todos los `environ`.
- En Rust son unos ~190 `stat` propios, unos ~40 `environ`, un `readdir` de `/dev/pts` y un `readlink` por pty con sesión.
- El 71 (`medir-trackerd.sh`) lo mide.

**Identidad `(pid, starttime)`.** La necesitan las anclas y los vetos: sin ella, un pid reutilizado extendería la sesión. El test "pid reutilizado con otro `starttime` no extiende" se escribe igual (commit 40, R8-4).

**Verificación a mano.** `w` **no** sirve: konsole no escribe en utmp, y acá `w` muestra solo `tty2`. Para ver la actividad de un pty se usa `stat -c %x /dev/pts/N`. "Como `w`" queda solo como explicación.

**E16 obligatorio.**

- Bajo el aislamiento real (`PrivateUsers=yes`, `ProtectSystem=strict`), E16 (commit 56) comprueba que el demonio lee `stat`, `environ`, `cwd` y el `atime` de `/dev/pts/N` de un `script` lanzado **fuera** del unit.
- E16 es manual (`#[ignore]`, 5.5) porque necesita el bus de usuario de systemd.
- Pasa a ser **bloqueante para el merge de F6**, con la salida pegada en el PR.

#### 3.10.4 Detección de inactividad (pty, archivos y presencia)

**Actividad válida de la sesión T en el instante `t`** (posterior a su UA y a su
frontera). Es cualquiera de estas:

1. **Tipeo** (DT4 + DT11): cambió el `atime` de un pty con un proceso o un ancla de T, **y** el proceso en primer plano de ese pty está dentro de la carpeta del proyecto o es una sesión remota. El tipeo cuenta **siempre**, sin mirar la presencia del escritorio: es presencia en sí (1a).
2. **Archivo con presencia** (DT10): un `cambio` en la carpeta del proyecto de T, filtrado con las reglas de `file_watch` (§3.5), **y** el usuario presente en el escritorio en ese instante.
3. **Clic** de Trabajar (o `gestor abrir`) sobre T: un `ultimo_clic_at` nuevo (3.10.1), en ese instante, si no es anterior a la frontera.

El resto no cuenta, pero queda registrado como evidencia ignorada (bits 16 y 32 de
3.10.1): `mtime` del pty, tipeo fuera de la carpeta, archivos con el usuario ausente o
con la pantalla bloqueada.

**Escrituras de herramientas con el usuario presente** (auditoría de facturación, AF-2).
La presencia corta el caso "usuario ausente", pero un dev server que reescribe logs o
cachés **mientras el usuario está en la compu** (por ejemplo, navegando) seguiría
sosteniendo la sesión. Se amplía la lista de **carpetas siempre ignoradas** de
§3.5 (commit 39, el filtro de eventos) con nombres que nunca son código
fuente:

- `.vite` (caché de Vite fuera de `node_modules`), `logs` (cubre `storage/logs` de Laravel y `logs/` de cualquier servidor), `.angular`, `.expo`, `.dart_tool`;
- `public/build` ya queda cubierto por `build`, que está en la lista y se compara por **nombre** de directorio;
- por ruta relativa a la raíz del proyecto (no por nombre, porque `framework` y `cache` son genéricos): `storage/framework` y `bootstrap/cache` (Laravel).

Criterio: se agrega un nombre solo si es una carpeta de salida o de caché de una
herramienta conocida y nunca se edita a mano. Los Laravel nuevos ya traen un
`.gitignore` adentro de `storage/logs` y `storage/framework/*`, que el filtro por
`.gitignore` anidado (M3, §3.5) respeta; la lista cubre los proyectos que lo perdieron.
Lo que quede (por ejemplo, un `db.sqlite3` de Django sin `.gitignore`) se ve en la
evidencia: el detalle por minuto muestra `archivo_rel`, y un archivo que sostiene una
sesión se agrega a `.gestorignore`. Queda como límite en §7 (riesgo 55). El filtro del commit 39 lo
prueba con un unitario, y E5 (commit 51) nace con un `logs/app.log` y un
`.vite/deps/x.js` → ninguna sesión.

##### Tipeo: `atime` del pty (DT4, sin cambios)

El kernel actualiza `atime` cuando un proceso **lee** del pty, y `mtime` cuando
escribe. Contar `mtime` mantendría viva la sesión toda la noche con un dev server o un
`tail -f`. La granularidad es de 8 s: `tty_update_time` actualiza el `atime` solo si
el segundo actual cae en otra ventana alineada de 8 s que el guardado (ignora los 3
bits bajos), y lo guarda en segundos enteros (`stat` sobre `/dev/pts/N` da
`.000000000` ns). O sea, **como mucho un cambio de `atime` por ventana de 8 s**, por
más que se tipee sin parar. La UA por pty puede quedar hasta 8 s antes de la última
tecla. El error es siempre para el lado de contar de menos. Los E2E lo tienen en cuenta
(§5.2, perfiles de tiempos).

**Reanudar exige dos señales** (auditoría de facturación, AF-9): vim, htop o tmux
con `mouse on` leen el pty con eventos de foco o de mouse sin que se tipee. Desde
`viva_pausada`, el tipeo reanuda solo si hay **dos cambios distintos de `atime` en
60 s**; el tramo abre en el primero, así que no se pierde nada. En `viva_activa`
alcanza con uno. Un archivo con presencia o un clic reanudan solos.

##### Atribución por cwd (DT11, decisión 3a)

Por cada pty con `atime` nuevo, el lector de procesos (commit 40, R8-4, `trackerd::procesos`)
resuelve el primer plano:

1. Toma de `/proc/<pid>/stat`, de cualquier proceso de ese pty, el campo 8 `tpgid`. El parseo arranca **después del último `)`** (corrección aprobada en la revisión de R8): `comm` puede tener espacios y paréntesis.
2. Lee el líder del grupo, `/proc/<tpgid>/comm` y `readlink /proc/<tpgid>/cwd`. Si el líder ya no existe (por ejemplo, el primer comando de un pipe que terminó), usa cualquier proceso del escaneo con `pgrp = tpgid`.
3. **Remoto:** si el `comm` del líder está en `{ssh, mosh-client, autossh, sshpass, et}`, el tipeo **cuenta** y el minuto lleva el bit 4 (`remoto_ssh`), sin mirar el cwd. Es la marca "remoto: ssh" de 3a.
4. **Multiplexor:** si el líder es un cliente de `tmux` o de `screen`, su cwd es el del `attach`, no el del panel. Se busca el panel activo: entre los ptys cuyos procesos descienden de un `tmux: server` o `screen` del mismo UID, el que tenga el `atime` más reciente, a no más de 8 s del `atime` del pty exterior. Se aplica 2–3 sobre ese pty. Si no se puede resolver, el tipeo **no cuenta** (principio 4) y queda el bit 16. Se verifica en el paso manual 28.
5. **Dentro de la carpeta:** el cwd canonizado se compara con `Path::starts_with` contra la carpeta canonizada, la misma función de contención de §3.4. Si está dentro, cuenta (bit 1). Si no, no cuenta (bit 16).

**Errores:** si el `cwd` del primer plano no se puede leer (`EACCES` de un proceso root
después de `sudo`, `PR_SET_DUMPABLE = 0`), el lector usa el `cwd` del líder de sesión,
que es la shell del usuario (3.10.3). Si tampoco se puede (`PrimerPlano::Desconocido`, o
`ENOENT` por una carrera), ese tipeo no cuenta y el minuto lleva el bit 16. Nunca se
loguea ni se guarda la ruta del cwd.

**Precisión:** el `cwd` se mira en el sondeo, hasta 5 s después de las teclas. Un
`cd ~/B` seguido de Enter se atribuye a B. El error está acotado por el sondeo y queda
documentado.

**Lo que esto arregla** (auditoría de facturación, AF-6, AF-8 y AF-11): un `cd ~/B` o
una pestaña de konsole usada para otra cosa ya no le cobran a A. `ssh servidor-B`
**sí** le cobra a A, pero queda marcado. Que sea trabajo de A o de B lo decide el
usuario con el desglose y, si hace falta, con un ajuste (3.10.17).

##### Presencia del escritorio (DT10, decisión 1a)

**Regla.** Un cambio de archivo en `t` es actividad válida si y solo si se cumplen
las dos condiciones:

- la pantalla **no** está bloqueada en la última lectura;
- `ultima_presencia ≥ t − VENTANA_PRESENCIA`, donde `ultima_presencia = instante_de_lectura − inactividad_del_escritorio` y `VENTANA_PRESENCIA` es una **constante de 10 min** (decisión 1a, literal). No es `pausa`: aunque el usuario configure una pausa de 60 min, la presencia sigue siendo "teclado o mouse en los últimos 10 min".

El tipeo en el pty no necesita presencia.

**Dónde vive:** en la lib, `trackerd::presencia`, con
`trait FuentePresencia { fn leer(&mut self) -> LecturaPresencia }` y
`enum LecturaPresencia { Ok { inactivo_ms: u64, bloqueada: bool }, NoDisponible { motivo } }`.
Corre en un **hilo propio** del demonio, "presencia", que lee cada 5 s y le manda la
lectura al hilo principal por canal. El hilo principal **nunca** espera a D-Bus, así
que un bus colgado no frena el bucle ni el watchdog de M13. Se agrega una fila a la
tabla de hilos de §3.4.

**Fuentes, en orden** (se elige una al arrancar y se reintenta cada 30 s si falla):

| Fuente | Inactividad | Bloqueo | Estado |
|---|---|---|---|
| KDE: bus de sesión, `org.freedesktop.ScreenSaver` en `/org/freedesktop/ScreenSaver`, si el objeto **también** expone `org.kde.screensaver` | `GetSessionIdleTime`, en **milisegundos** (medido en Plasma 6.7.4) | `LockedHint` de logind; de respaldo, `ScreenSaver.GetActive` | **Verificado acá**, también bajo el aislamiento del unit |
| GNOME: `org.gnome.Mutter.IdleMonitor` en `/org/gnome/Mutter/IdleMonitor/Core`, `GetIdletime` (ms) | ídem | `LockedHint` | No verificable en esta máquina. Queda como implementación con test de unidad contra un bus falso |
| Ninguna (otro escritorio, sin sesión gráfica, bus caído) | — | — | `NoDisponible` → fallback |

`GetSessionIdleTime` **no** se usa fuera de KDE: la especificación dice segundos y
KDE devuelve milisegundos, así que en otro escritorio la unidad sería una adivinanza.
`LockedHint` se lee por el bus del sistema, en la sesión `Display` del usuario
(`/org/freedesktop/login1/user/self`, propiedad `Display`). Así no depende de en qué
sesión de logind quede el proceso del demonio. El cliente es `zbus` 5 en modo
bloqueante, que ya está en el árbol.

**Chequeo de unidades:** una actualización de KDE podría pasar a devolver segundos.
Si dos lecturas seguidas crecen y, estando separadas 5 s, difieren en menos de 10, la
fuente está en segundos. En ese caso se multiplica por 1000 y se anota en
`presencia_motivo`. Tiene test de unidad.

**Fallback explícito (sin dato de presencia)** (decisión U2). Los cambios de archivo
**cuentan**, como antes de R8, y cada minuto así lleva el bit 8: `presencia_desconocida`
en el desglose ("sin dato de presencia del escritorio: los archivos contaron sin
verificar que estuvieras"). Vale para el motor de terminales y para `file_watch` (U1),
que lo anota en `archivos_evidencia`. Una lectura con más de 15 s de antigüedad vale
como `NoDisponible`. `tracker_status.presencia` y `presencia_motivo` lo publican, y
Ajustes lo muestra. Es una **excepción declarada al principio 4** (§3, principio 4, excepción 2).

Por qué contar y no ignorar (lo que eligió el usuario): el caso típico es un reinicio
del bus de unos segundos o un escritorio sin API. Ignorar los archivos ahí haría que
una sesión de trabajo en el IDE (sin tipear en la terminal) se pausara entera. Con la
marca, el usuario lo ve y lo corrige con un ajuste negativo.

**Alcance** (decisión U1): la regla de presencia se aplica al motor de terminales
**y** al tiempo automático `file_watch`. Presencia es teclado o mouse en **cualquier**
ventana del escritorio (el idle de la sesión gráfica), no solo en el IDE o en la
terminal. El hilo y la lectura son los mismos; en `file_watch` es una condición en el
bucle (48): un cambio filtrado llega al motor de `file_watch` (36) solo si
hay presencia (o no hay dato, U2). Si no hay presencia, **no abre ni extiende** ningún
tramo automático, y el minuto queda en `archivos_evidencia` con el bit 32 y los
segundos de 3.10.1.

##### Lo que cambió en disco sin presencia: tramos `solo_archivos` (U1, variante c)

Los cambios sin presencia **no suman al total**, pero se ven y se pueden cobrar a mano:

- **Registro.** Cada uno queda por minuto, con el segundo exacto del primero y del último: en `trabajar_evidencia` si hay una sesión de Trabajar viva en el proyecto, en `archivos_evidencia` siempre que el bucle lo vea (con o sin Trabajar). Así hay registro aunque el proyecto no tenga ninguna sesión abierta.
- **Tramo derivado.** El desglose (3.10.14, `db/desglose.rs`) agrupa los minutos con bit 32 del proyecto (las dos tablas, unidas por minuto) en **tramos `solo_archivos`**: cambios consecutivos a menos de `idle` (3 min, el umbral con el que `file_watch` los habría cobrado; se usa el mismo aunque haya Trabajar, para no unir huecos largos sin evidencia: dos tramos contiguos se cargan con dos clics) forman un tramo de `minuto + ausente_desde_s` del primero a `minuto + ausente_hasta_s` del último, cerrado en el último cambio como cualquier tramo (2a). Se le resta lo que ya cubre la unión cobrada del proyecto, así que nunca se muestra como "no cobrado" algo que sí se cobró. No se guarda: se calcula, como las otras marcas.
- **Qué se ve.** "Solo archivos, sin presencia: 14:40–15:05 (25 min), no se cobró", con la ruta relativa del primer archivo de cada minuto al expandir.
- **Cómo se cobra.** El botón **"Cargar como ajuste"** del tramo abre el formulario de 3.10.17 en modo "un rato", con `desde`, `hasta` y `segundos = hasta − desde` precargados, signo +, y pide la nota obligatoria. Si el tramo cruza la medianoche, carga dos ajustes (uno por día) con la misma nota, porque un ajuste con horario es de un solo día (3.10.17).
- **Un tramo de 0 s** (un solo cambio) se muestra igual ("1 cambio a las 14:40") y no ofrece el botón: no hay rato que cargar.

**Pruebas:** unidad con una `FuentePresencia` falsa en el motor (commit 42, R8-5):

- archivo con `inactivo` = 9:59 contra 10:01;
- bloqueada → no cuenta;
- `NoDisponible` → cuenta con el bit 8;
- lectura vieja de 16 s → bit 8;
- archivo sin presencia → evidencia con bit 32 y `ausente_desde_s`/`ausente_hasta_s` del segundo del cambio.

En el bucle (48): un cambio sin presencia no abre ni extiende un tramo `file_watch` y
deja una fila de `archivos_evidencia` con bit 32; con `NoDisponible`, abre el tramo y
deja bit 8. En el desglose (commit 69, R8-9): tres cambios sin presencia a 1 min y uno a 5 min →
dos tramos `solo_archivos` con desde y hasta al segundo; un tramo que se superpone con
uno cobrado se recorta.

Unidad del cliente D-Bus contra un bus privado de prueba (`dbus-daemon --session
--print-address` en un `tempdir`) con un objeto falso que expone las dos interfaces
de KDE. Se lanza por ruta absoluta, `/usr/bin/dbus-daemon`: en esta máquina
`dbus-daemon` resuelve primero a linuxbrew. Si no está, el test **falla** en Linux, no se saltea. En los
E2E, el harness define `GESTOR_TRACKERD_TEST_PRESENCIA=<archivo>` con
`inactivo_ms=<n> bloqueada=<0|1>` o `no_disponible`. Solo se acepta junto con
`GESTOR_TRACKERD_TEST_TIMINGS`, con la misma guarda de base real que B8. El demonio
real **nunca** se prueba contra el escritorio del desarrollador. El bloqueo real de
pantalla queda en el recorrido manual.

##### Qué muestra un tramo abierto y dónde cierra (DT3 corregida por 2a)

Un tramo abierto va de `started_at` a la UA (`last_activity_at`), y **todo** cierre
lo deja ahí. Lo que pasa entre la última actividad y el cierre de la terminal, Parar o
el kill switch no se factura. Las pausas internas menores a `pausa` sí cuentan, porque
no parten el tramo. El contador **nunca baja** y tampoco sube al cerrar: lo que se ve
es lo que se guarda. Vale para todo tramo `terminal` (`dueno = demonio`); los tramos
`work_button` de una sesión `dueno = app` son la excepción declarada en §1.2 (U6: hasta
Parar, con tope de 12 h y la marca `sin_vigilancia`).

**Proyectos no vigilados** (`carpeta_amplia`, `demasiadas_carpetas`, `en_red`, …):
solo cuenta el tipeo. Si además la terminal no se detecta, la sesión no tiene ninguna
fuente de actividad y queda en 0 s. No se inventa "hasta Parar" (principio 4 y 2a). La
tarjeta lo avisa fuerte (3.10.14) y el usuario lo corrige con un ajuste (3.10.17).

#### 3.10.5 Suspensión y saltos de reloj

En cada sondeo, el motor recibe `Reloj { pared, mono, boot }`
(`CLOCK_REALTIME`, `CLOCK_MONOTONIC` y `CLOCK_BOOTTIME`) y calcula, contra el sondeo
anterior:

- `s = Δboot − Δmono`: tiempo suspendido. `BOOTTIME` avanza durante la suspensión y `MONOTONIC` no; ninguno de los dos se mueve con NTP ni con un cambio manual.
- `r = Δpared − Δboot`: salto de la pared que **no** explica la suspensión (NTP con `makestep`, cambio manual).

**Precedencia** (auditoría de consistencia, "NTP contra `reanudado`"). Una sola
función de la lib, `clasificar_salto(Δ, umbral) -> Salto { Nada, Suspension, Reloj, Ambos }`,
la usan los dos motores, cada uno con su umbral:

| Caso | Motivo del corte | Frontera |
|---|---|---|
| `s > 2 s` (con o sin `r`) | `suspension`. Si al despertar NTP también corrige (`Ambos`, lo común), **gana `suspension`**: el tramo ya se cortó, y el salto queda absorbido | Pared después del despertar |
| `s ≤ 2 s` y `\|r\| > 2 s` | `reloj`. Si `\|r\| > 5 min`, además se anota en `last_error` (M16) | Pared actual |
| Ninguno | Nada | — |

Un paso de NTP ya **no** se confunde con una suspensión, porque sin suspensión `s ≈ 0`.
El umbral de 2 s es holgado: el *slew* de NTP llega como mucho a 500 ppm, unos 2,5 ms
cada 5 s, y las tres lecturas se toman juntas. El motor `file_watch` mantiene su
umbral de 30 s sobre `s` (§3.4).

**Efecto:** el tramo abierto cierra en la UA con el motivo de la tabla, la sesión pasa
a `viva_pausada` (o sigue en `terminal_no_detectada` sin tramo) y la frontera se
mueve. Si la terminal sigue viva, el tramo siguiente abre recién con la próxima
actividad válida, así que nada del hueco queda adentro. Un salto **atrás** usa
`max(t, UA)` para que ninguna duración dé negativa.

**Por qué alcanza con esto:** la `duration` de un tramo es `UA − started_at` en pared,
y ningún tramo contiene un salto. Por eso duración de pared y tiempo observado
coinciden, sin un acumulador aparte.

**Pruebas** (commit 42, R8-5; corrige "la guarda monotónica no se puede probar" de la auditoría
de pruebas):

- pared +30 s, mono +5 s, boot +30 s → `suspension`;
- pared +30 s, mono +5 s, boot +5 s → `reloj`;
- pared +8 h 1 min, mono +5 s, boot +8 h → `suspension` (y no `reloj`);
- pared −10 min → `reloj`, `last_error`, duración ≥ 0;
- pared +1,5 s de jitter → nada;
- suspensión de 8 h con la terminal viva → dos tramos y el hueco afuera.

La suspensión real va en el paso manual 26.

#### 3.10.6 Convivencia de proyectos (D-R8-2) y varias terminales (D-R8-4)

- **Convivencia solo con `dueno = demonio`** (corrección aprobada en la revisión de R8; auditoría de facturación, AF-22). Trabajar en B no toca las sesiones `dueno = demonio` de otros proyectos: cada una se pausa sola por inactividad. Con `dueno = app` (sin demonio vivo), la app mide **un** proyecto a la vez (3.10.7): Trabajar en B **cierra el tramo** de la sesión `dueno = app` de A, igual que hoy (`start_new_session_sync`, `commands/mod.rs:1638`), con motivo de tramo `cambio_de_proyecto` y el toast de `sesion_anterior` (el campo `previous_session_stopped` de hoy se renombra en el commit 17 (R8-3), con sus usos: `workSession.ts:40`, `ProjectList.tsx:237`, `api.test.ts:863-875`, `test-setup.ts:142`, `Dashboard.test.tsx:243` y `ProjectList.work.test.tsx:234`). La **sesión** de A no termina (D-R8-1): pasa a `esperando_demonio` y la adopta el demonio cuando esté. Solo termina, con `cambio_de_proyecto`, si la adopción es imposible (3.10.7, U4), que es el caso de todo F1–F5. Sin pausa no hay convivencia segura: una A olvidada sumaría todo lo que se trabajara en B.
- **Mismo proyecto, otra vez** (D-R8-4): Trabajar sobre un proyecto con una sesión viva (en cualquier estado vivo, incluso `esperando_demonio` o `terminal_no_detectada`) **reutiliza el token** y abre otra terminal con la misma marca. En la misma transacción escribe `ultimo_clic_at = ahora` (3.10.1): así el clic llega al demonio, que lo trata como actividad válida y, en `viva_pausada`, abre un tramo en el clic. Si después no hay más actividad, ese tramo queda en 0 s y se borra. Si el token se cerró en la misma ventana (carrera con `terminal_cerrada`, 3.10.2), Trabajar crea uno nuevo.
- **Varias terminales:** la sesión sigue mientras quede alguna terminal o ancla viva, y cierra en la UA cuando se va la última (`terminal_cerrada`).
- **Un pty pertenece a un solo token** (3.10.16): el tipeo en un pty solo puede contar para el token que lo reclama, así que dos proyectos nunca suman el mismo tipeo. El tiempo de pared simultáneo entre proyectos **no se reparte**: se muestra (4a, 3.10.14).
- `gestor abrir X` en una terminal que ya mide Y cambia la terminal de sesión (3.10.16).
- Una terminal abierta **desde** una terminal marcada hereda la marca y extiende la sesión. Con DT11, si esa terminal se usa en otra carpeta, su tipeo no cuenta.

**Pruebas:** en el commit 17 (R8-3), Rust:

- dos `dueno = app`: Trabajar en B cierra el tramo de A con `cambio_de_proyecto` y A queda `esperando_demonio` (siguen valiendo los tests actuales `cambio_de_proyecto_a_los_5_s_guarda_0`, `once_segundos_se_guardan_tal_cual_al_cambiar_de_proyecto` y `un_error_de_base_al_cerrar_con_0_s_en_el_cambio_de_proyecto_sube`);
- Trabajar en A dos veces → mismo token y `ultimo_clic_at` actualizado. Acá cambian de semántica `two_rapid_start_calls_do_not_lose_any_session` (`commands/mod.rs:2049`) y `concurrent_start_calls_serialize_without_orphaning_a_session` (`:2115`), que son el **mismo** proyecto dos veces (§5.6).

En el commit 58 (R8-8), con el demonio vivo falso: A y B `dueno = demonio` conviven, y el tramo de una
`dueno = app` vieja de A **sí** se cierra al abrir B (A pasa a `esperando_demonio` y el demonio la adopta).

#### 3.10.7 Quién es el dueño: la base como fuente de verdad (DT2)

**Decisión: la base.** `trabajar_sesiones` dice qué sesiones existen y en qué estado
están. `ActiveSession` deja de ser la fuente de verdad y guarda **solo** lo que la app
mide con su propio reloj:

```rust
pub struct ActiveSession(pub Mutex<Option<MedicionApp>>);
pub struct MedicionApp { project_id: i64, token: String, tramo_id: i64, inicio: Instant }
```

Es una sola entrada, no un mapa, porque la app mide **un** proyecto a la vez (ver
"Dueño `app`" más abajo).

| Opción | A favor | En contra | Veredicto |
|---|---|---|---|
| **Base como verdad + `ActiveSession` solo para lo que mide la app** | El demonio, la CLI y la app recién reabierta ven lo mismo. Al reabrir la app aparece Parar en las sesiones vivas, cosa que hoy no pasa (`get_work_session_status` solo mira la memoria). El índice único hace de lock entre procesos | Para saber el estado hay que consultar la base. Es una consulta cada 5 s, que ya existe (§3.9) | **Elegida** |
| `ActiveSession` como mapa de todas las sesiones | Se lee en memoria | La memoria no ve lo que hacen el demonio ni la CLI, y la tarjeta se desincroniza de la base | Descartada |

**Dueño único:**

| `dueno` | Quién mide y escribe `duration` | Quién puede cerrar |
|---|---|---|
| `demonio` | Solo el demonio, que la adopta en ≤ 1 sondeo | El demonio (terminal, inactividad, suspensión), la app y la CLI (Parar, papelera, kill switch, restauración). Todos los cierres son condicionales e idempotentes |
| `app` | Solo la app, con el `Instant` y el latido de 60 s del commit 19, como hoy | La app (Parar, papelera) y la CLI (`gestor parar`, 3.10.16) |

`dueno` es el dueño **actual** (3.10.1): cambia solo por los dos traspasos de abajo, y
los dos ocurren sin tramo abierto. Por eso un tramo nunca cambia de medidor, y su
`source` dice quién lo midió (`terminal` = demonio, `work_button` = app). Una sesión
puede tener tramos de las dos fuentes (por ejemplo, la app la midió, se cerró la
ventana y la siguió el demonio). La invariante 10 de §6 controla solo los tramos
**abiertos**.

**Una sola definición de "demonio vivo".** La función es
`demonio_vivo(conn, ahora) -> bool`. Nace en el commit 15 (R8-1), en el crate de la app, y sube a la lib
con el commit 33. Es verdadera si se cumple todo esto:

- `tracker_status` tiene fila, con `estado = 'activo'` y `pid` no `NULL`;
- `ahora − last_heartbeat < 45 s`;
- en Linux, `/proc/<pid>` existe **y** `/proc/<pid>/comm` es `gestor-trackerd` (15 caracteres: entra justo en el límite de `comm`). Sin la primera, después de un `kill -9` el demonio parecería vivo durante 45 s; sin la segunda, un pid reutilizado en esos 45 s también.

La usan `trabajar`, la tarjeta, la reconciliación de la app (3.10.11), el kill switch
(3.10.12) y el subcomando de la CLI (3.10.16). No lee la config: con `daemon_enabled = false`,
el demonio pone `pid = NULL` al salir (§3.4), y eso alcanza.

**`estado_trabajar()`** (comando Tauri, el commit 17 (R8-3)). Devuelve las sesiones vivas de la base,
cada una con `token`, `project_id`, `estado`, `dueno`, `origen`, `creada_at`,
`last_seen_at`, el tramo abierto si lo hay (`started_at`, `last_activity_at`,
`duration_seconds`), los segundos del token, `elapsed_seconds` de la `MedicionApp` si
es la que mide la app, y `demonio_vivo`. Lo usan la tarjeta y la reconciliación de la
app. En el 64 (`get_live_tracking`) se integra con el resto del estado en vivo
(3.10.14).

**Cómo se elige el dueño.** `trabajar` elige `demonio` si `demonio_vivo()` da verdadero,
y `app` si no. Nunca se traspasa una sesión con un tramo abierto. Hay solo dos
traspasos, y en ninguno quedan dos procesos midiendo a la vez:

| Traspaso | Cuándo | Cómo |
|---|---|---|
| `app` → `demonio` | La app deja de medir una sesión que no terminó: se cierra la ventana, se toca Trabajar en otro proyecto, la app se cayó (huérfana, 3.10.11) o el tramo llegó al tope de 12 h (U6). **No** se traspasa en caliente porque el demonio haya vuelto: la app sigue midiendo hasta uno de esos puntos | La app cierra su tramo con el `Instant` (recortado a 12 h), como hoy, con motivo de tramo `cierre_app`, `cambio_de_proyecto`, `huerfana` o `tope_sin_tracker` según el caso. En la misma transacción pone `dueno = demonio` y `estado = esperando_demonio`. El demonio la adopta al arrancar o en el sondeo siguiente (3.10.10). Si la adopción es imposible (ver abajo), la sesión **termina** con ese mismo motivo |
| `demonio` → `app` | **Solo** cuando el usuario toca Trabajar en esa sesión y `demonio_vivo()` da falso | Si quedó un tramo abierto (el demonio murió con `kill -9`), se cierra en su `last_activity_at` con motivo `tracker_caido`. Después, en la misma transacción, pasa a `dueno = app` y `estado = viva_activa`, con un tramo `work_button` nuevo desde el clic |

**Nunca hay adopción silenciosa como `dueno = app`.** Abrir la app, el sondeo de la
tarjeta o el kill switch **no** le pasan a la app una sesión del demonio.

**Dueño `app`: un proyecto a la vez.** Con `dueno = app` no hay pausa por inactividad ni
detección de cierre (3.10.13). Si dos sesiones de la app convivieran, un proyecto olvidado
sumaría todo el día mientras se trabaja en otro: es la "regresión de F1 a F5" de la
auditoría de facturación (AF-22). Por eso:

- Trabajar en B, con A medido por la app, cierra el tramo de A como hoy (`start_new_session_sync`, `commands/mod.rs:1638`) y pasa A a `esperando_demonio`. A **no** termina (D-R8-1): su tarjeta dice "Esperando al tracker" y tiene Parar.
- Si Trabajar elige `demonio` para B mientras la app mide A, A también pasa a `esperando_demonio`. Con el demonio vivo, la app no mide nada.
- La convivencia de proyectos (D-R8-2) es plena solo con `dueno = demonio`.
- **Excepción: adopción imposible** (decisión U4). Ahí nadie va a adoptar nunca una sesión en espera: una `esperando_demonio` quedaría colgada para siempre, o la cerraría el kill switch con un motivo falso. Rige lo de hoy: Trabajar en B **termina** la sesión de la app con `cambio_de_proyecto`, cerrar la ventana con `cierre_app`, la recuperación de huérfanas con `huerfana` (Enmienda 1) y el tope de 12 h con `tope_sin_tracker` (U6). Es una excepción declarada a D-R8-1 (§1.2). Lo que decidió el usuario, por caso: tracker andando → la sesión sigue (la tiene el demonio); tracker caído o sin instalar → `esperando_demonio`, se retoma cuando vuelve y ese rato no se mide; tracker apagado en Ajustes o Windows → termina como hoy.
- **Cuándo es imposible, y en qué commit entra cada condición** (corrige una dependencia invertida: `tracking.daemon_enabled` no existe hasta el 35). Una sola función, `adopcion_imposible(config) -> bool`, que crece por commits:
  - **commit 17 (R8-3):** `!tracker_soportado()`. `tracker_soportado()` = Linux **y** sin `GESTOR_NO_TRACKERD` en el entorno (la misma variable que las condiciones de instalación de §3.8) **y** `BIN_DIR/gestor-trackerd` existe y es ejecutable (`BIN_DIR` por defecto `~/.local/bin`, inyectable en los tests). En F1–F5 ningún script instala ese binario (`release.sh --install` e `install.sh` copian solo la app hasta el commit 59), así que en `main` `adopcion_imposible()` es **verdadero** en toda instalación: Trabajar mide con la app, y cerrar la app, Trabajar en otro proyecto, una caída o el tope de 12 h terminan la sesión como hoy (`cierre_app`, `cambio_de_proyecto`, `huerfana`, `tope_sin_tracker`). No aparecen sesiones "Esperando al tracker" ni alarmas de un tracker que no existe (3.10.14).
  - **35 (sección `tracking` de `AppConfig`):** suma `|| !config.tracking.daemon_enabled`, con su test ("tracker apagado → `cerrada/cierre_app`"). Antes del 35 el usuario no tiene cómo apagar el tracker, así que la cláusula no hace falta.
  - **54 (generador del unit y condiciones de 3.8):** `tracker_soportado()` pasa a las condiciones con las que la app decide instalar el demonio: Linux, producción, sin `GESTOR_NO_TRACKERD` y `current_exe()` dentro de `BIN_DIR`. **Deja de exigir** el binario: desde acá un `gestor-trackerd` que falta es "Tracker no instalado" (3.8, paso 3) y la sesión espera (U4). Es el commit en que `adopcion_imposible()` deja de ser verdadero en una instalación de producción; llega a `main` con el merge de F6, junto con el 59, que instala los dos binarios. En desarrollo (`src-tauri/target/`) la adopción sigue siendo imposible y las sesiones terminan como hoy, en vez de acumularse "Esperando al tracker" (riesgo 49). La pregunta U4 incluía este caso entre los "sin tracker posible" y el usuario aceptó la regla; si prefiriera tratar el binario de desarrollo como "no instalado" (en espera), alcanza con sacar esta cláusula.

**Cierres sin carrera.**

- Todos los métodos nuevos de `db/trabajar.rs` (commit 15, R8-1) corren en `transaction_with_behavior(TransactionBehavior::Immediate)`, con condiciones `estado <> 'cerrada'` y `ended_at IS NULL`. Ese método pide `&mut Connection` (rusqlite 0.32.1, `transaction.rs:428`), que se obtiene del `MutexGuard` de `self.conn()` (`db/mod.rs:50`).
- Si Parar y el demonio cierran a la vez, gana uno. El otro recibe `false` o `None` y suelta el token (M12).
- Antes de abrir un tramo, el demonio comprueba que el token siga vivo **dentro de la misma transacción**. Así, un Parar que llega en el medio no deja un tramo huérfano.
- `end_tracking_session` (`db/tracking.rs:32`) usa hoy `unchecked_transaction()` (`:34`), que es **diferida**. El commit 15 (R8-1) la pasa a `Immediate`, porque desde el commit 17 (R8-3) la llaman cierres que compiten con otra conexión. Hoy hay **6** `unchecked_transaction` (`tracking.rs:34`, `trash.rs:22/68/145/211`, `status.rs:75`). El commit 15 (R8-1) convierte la de `tracking.rs:34` y el commit 18 (R8-3b) la de `delete_project` (`trash.rs:22`), que exige la misma transacción `IMMEDIATE`. El commit 28 (F2) llega después y queda con **cuatro** (`trash.rs:68/145/211`, `status.rs:75`).

**Locks.** Se mantiene el orden `ActiveSession → Database` de §3.3. Las
funciones que calculan tiempo copian la `MedicionApp` y sueltan el lock antes de tocar
la base.

**Restauración en curso: nadie crea sesiones.**

- `trabajar` y el subcomando de la CLI toman un `flock` **compartido** y no bloqueante sobre `<data_dir>/restore.lock` mientras dura su transacción.
- El restore (§3.6) lo toma **exclusivo** al confirmar el diálogo (antes del paso 3, que para el demonio) y lo suelta después del paso 8. No desde el paso 1: mientras el diálogo está abierto, la CLI devolvería 3 sin que haya restore en curso. Una sesión creada entre el diálogo y la confirmación se pierde igual que lo medido después del backup (la base se reemplaza).
- Si está tomado, se rechaza con: "Hay una restauración en curso: esperá a que la app se reinicie."
- `flock` choca también entre dos descriptores del mismo proceso, así que la app usa el mismo mecanismo que la CLI.

**Por qué no alcanzan los mecanismos que ya existen:**

- **`demonio_vivo()`:** el restore para el demonio en el paso 3, y sin demonio la CLI igual crea sesiones (3.10.13).
- **`trackerd.lock`:** lo retiene el demonio todo el tiempo, y la CLI no debe tomarlo (3.10.16).

#### 3.10.8 Integración con la unión de tramos (§3.3)

**Tramos de `terminal`.** La tabla de tramos de 3.3 tiene filas explícitas para
`source = 'terminal'` (sin depender de la invariante para que dé lo mismo, auditoría
de consistencia). Los tramos `work_button` de una sesión, con o sin
`terminal_token`, siguen la fila "Cerrada, otra fuente", y la `activa`, la suya.

**`activa` sigue siendo `Option<SesionActiva>`.** Como la convivencia aplica solo a
`dueno = demonio` (3.10.6), la app mide a lo sumo **una** sesión con `Instant`: es la
`MedicionApp` de `ActiveSession(Mutex<Option<MedicionApp>>)` (3.10.7), y `SesionActiva` es
la copia que se arma de ella antes de soltar el lock. Las de
`dueno = demonio` se leen de la base y no necesitan `activa`. Esto achica los cambios
de los commits 20 y 21: no hace falta pasar a un slice.

**Totales con ajustes** (3.10.17): `tiempo_trabajado` sigue siendo la unión pura, porque
la usan el desglose y la simultaneidad. Los totales que ve el usuario pasan a
`tiempo_total(project_id, rango, activa) = max(0, tiempo_trabajado + Σ ajustes del rango)`.
Lo usan `total_seconds`, `today_seconds` y `week_seconds` de `get_time_stats` y
`total_time_hours` de `get_project_stats`. `session_count`, `avg_session_seconds` y
`longest_session_seconds` no cambian: describen sesiones medidas. Los tramos derivados
`solo_archivos` (3.10.4, U1) **no** entran en ningún total ni en la simultaneidad: suman
solo si el usuario los carga como ajuste.

**Proyecto paraguas** (decisión U3: **sí**; auditoría de consistencia; D-R8-3 literal:
"cambios en la carpeta del proyecto"). El motor de terminales recibe
`cambio(proyectos_que_contienen, t, …)`: **todos** los proyectos cuya carpeta contiene
la ruta, no solo el más específico. La función de contención es la de §3.4 en la
lib (commit 38): la usa el bucle (48) para armar esa lista, y el motor (42) la recibe
hecha.
Una sesión de Trabajar del paraguas `~/2025` **sí** se sostiene con un guardado en
`~/2025/telwinet/casum`, y el tipeo con el cwd en `casum` también cuenta para ella
(está dentro). La atribución de `file_watch` **no cambia**: el guardado sigue creando
tiempo automático solo para `casum` (R1). Si las dos cosas pasan a la vez, el desglose
de cada uno marca "simultáneo con" el otro (4a): se ve y no se reparte. Límite: si el
paraguas no se vigila (por ejemplo, `demasiadas_carpetas`), solo llegan los cambios de
las carpetas que se vigilan por un hijo. Queda como riesgo 41.

**Margen DX1 dentro de una pausa** (auditoría de consistencia y de facturación). Los
tramos `terminal` no tienen margen. Pero `file_watch` sigue corriendo en paralelo:
si un guardado reabre `file_watch` mientras la sesión de terminal está pausada, la
unión suma los 60 s previos al guardado. Se acepta (es la excepción DX1 del
principio 4, §3), y el desglose lo dice en el tramo `file_watch`: "incluye 60 s
antes del primer guardado".

**Simultaneidad (4a).** Se calcula desde los tramos y no se guarda. En la lib,
`db/tiempo.rs`:
`simultaneidad(project_id, rango) -> Vec<(otro_project_id, segundos)>` =
`|unión(P) ∩ unión(Q)|` para cada Q con tramos en el rango, más la variante por tramo
para el desglose. Una sola consulta trae los tramos de todos los proyectos del rango
(la variante de §3.3 para `get_project_stats`) y la intersección de dos listas
ordenadas es lineal. Entran todas las fuentes (`terminal`, `work_button`,
`file_watch`); los ajustes no entran.

**Pruebas** (commit 20, para la unión y la simultaneidad; commit 42 (R8-5), para el
paraguas; commit 22 (R8-A1), que va después del 21, para `tiempo_total` y su uso en las
estadísticas):

- terminal de 9 a 18, suspensión de 12 a 14 y `file_watch` de 16 a 18 → 7 h;
- pausa de 10 min con un `file_watch` de 3 min adentro → unión sin doble conteo y con los 60 s del margen;
- A de 10 a 12 y B de 11 a 13 → simultáneo 1 h en cada uno, y totales de 2 h y 2 h (sin reparto);
- paraguas con un guardado en el hijo (motor del 42, con la lista de contenedores dada) → la sesión del paraguas sigue `viva_activa`; que el `file_watch` sea solo del hijo ya lo prueban el 37 y E19;
- `tiempo_total` con un ajuste de −30 min → total − 30 min, y con un ajuste mayor que lo medido → 0.

#### 3.10.9 Parar

**La función.** `parar(project_id, token: Option<String>, origen) -> ResultadoParar`,
con `enum ResultadoParar { Cerrada { segundos }, YaCerrada { motivo, segundos }, Ninguna }`.

- Reemplaza a `stop_work_session` (`commands/mod.rs:1771`).
- Corrige el defecto de `cerrar_sesion_de_trabajo` (`:1715`), que descarta el `bool` de `end_tracking_session` (`:1728`) y dice "guardado" aunque la fila ya estuviera cerrada.
- `token` es el que muestra la tarjeta. Si ese token ya está `cerrada`, devuelve `YaCerrada` con su motivo y su total. Sin token y sin sesión viva, devuelve `Ninguna`.
- `segundos` es la suma de los tramos del token, para el toast.

**Qué cierra.**

- **Cualquier estado vivo:** `esperando_terminal`, `esperando_demonio`, `viva_activa`, `viva_pausada` o `terminal_no_detectada`. El token pasa a `cerrada`, con `terminada_at` en el clic (que nunca es anterior al fin de un tramo: regla única de 3.10.15) y motivo `parar_app` (botón) o `parar_cli` (`gestor parar`). Sus anclas abiertas se cierran con `motivo_salida = parar` (3.10.1).
- **El tramo abierto, si lo hay:**
  - `dueno = demonio`: se cierra con la regla de fin de 3.10.4 (decisión 2a). Se factura hasta la última actividad, y la cola posterior no.
  - `dueno = app`: se cierra con el `Instant`, como hoy, aplicando la regla de duración de 3.10.1.
- **Sin tramo abierto** (`viva_pausada`, `esperando_*`): no se abre ni se cierra ningún tramo.
- El demonio se entera en el sondeo siguiente (el volcado devuelve `false`, o ve el token `cerrada`) y lo suelta.

**Toasts.**

| Resultado | Toast |
|---|---|
| `Cerrada` | "Sesión de X guardada: 1 h 12 min." |
| `YaCerrada` | "La sesión de X ya había terminado: <texto del motivo, 3.10.15> (1 h 12 min)." |
| `Ninguna` | Ninguno: solo se refresca la tarjeta. Sale el "No había una sesión activa para parar" de hoy (`ProjectList.tsx:253`) |

**La terminal no se cierra.** Sus procesos siguen con el token viejo, que ya está
`cerrada`, así que no cuentan (criterio 4 de 3.10.3). Para seguir midiendo ahí, se toca
Trabajar (abre otra terminal) o se corre `gestor abrir X` en esa misma terminal (3.10.16).
El toast lo dice (riesgo 40).

**`gestor parar [X]`.** Ver 3.10.16. Además de terminar la sesión, veta el pty de la shell
donde se corrió.

**Otros cortes que inicia la app:**

| Acción | Qué pasa con las sesiones vivas | Motivo |
|---|---|---|
| Mandar un proyecto a la papelera (`delete_project`, `commands/mod.rs:167` → `db/trash.rs:19`) | Se terminan las sesiones vivas del proyecto **y las de todos sus descendientes**, porque el soft-delete los marca a todos con el mismo `deleted_at` (`trash.rs:28-37`). Va en la **misma** transacción `IMMEDIATE` que ese `UPDATE`. Si la app medía uno de esos proyectos, antes cierra su tramo con el `Instant` (orden `ActiveSession → Database`). Toast: "Se cortó la sesión de X (1 h 12 min) porque lo mandaste a la papelera." | `proyecto_a_papelera` |
| Sacar el proyecto de la papelera | Nada: las sesiones ya estaban cerradas. Para medir, se toca Trabajar | — |
| Purgar | El `ON DELETE CASCADE` borra las sesiones y sus tramos (3.10.1) | — |
| Restaurar un backup | Ver abajo | `restauracion` |
| `trabajar` o `sesion iniciar` sobre un proyecto en la papelera | Se rechaza: "X está en la papelera: restauralo para trabajar en él." | — |

**Restaurar un backup con sesiones vivas: las sesiones se cortan con aviso.**
Restaurar ya es destructivo, y §3.6 (paso 1) ya avisa que se pierde "la de
Trabajar en curso". Se extiende a todas:

1. **Paso 1 de §3.6:** el diálogo de confirmación lista las sesiones vivas: "Se van a cortar: A (desde 14:02), B (desde 15:30). Lo medido después del backup se pierde."
2. **Paso 5:** se vacía `ActiveSession` sin escribir, como en §3.6.
3. **Paso 6, después de `migrar` y con el lock tomado:** se cierra toda sesión de la base **restaurada** que no esté `cerrada` (pasa si el backup se hizo con sesiones vivas). Su tramo abierto se cierra en su `last_activity_at` (si es un `work_button` de una sesión `dueno = app`, con la duración del último latido, como una huérfana de §3.2), y el token con `terminada_at` por la regla única de 3.10.15, las dos cosas con motivo `restauracion`. Si no se cerraran, el demonio las readoptaría con un hueco, entre el backup y la restauración, que nadie sabría explicar.
4. **Las terminales siguen abiertas,** con tokens que la base restaurada no conoce o que ya figuran cerrados. No cuentan (criterio 4 de 3.10.3). Para seguir midiendo, se toca Trabajar.

Se descartó rechazar el restore mientras haya sesiones vivas. Obligaría a cerrar
terminales, o a tocar Parar en una base que el restore va a pisar, y el resultado sería
el mismo.

#### 3.10.10 Arranque y cierre del demonio

**Arranque, paso 6 extendido (§3.4).** Antes de pisar `tracker_status`, el demonio
lee el `pid` que dejó la instancia anterior:

- `pid = NULL`: la instancia anterior salió limpia.
- Otro valor: fue un **corte sucio** (`kill -9`, un corte de luz, o un `SIGKILL` después de `TimeoutStopSec`).

El paso corre después de cerrar las huérfanas `file_watch` y antes de `READY=1`:

1. **Carga y escaneo.** Se cargan las sesiones `dueno = demonio` cuyo estado no es `cerrada` (incluidas `esperando_terminal` y `esperando_demonio`), los reclamos con `hasta IS NULL`, y se hace un escaneo completo (3.10.3) con `frontera = arranque`. Los reclamos cuya shell murió se cierran con `shell_cerrada` (3.10.1).
2. **`esperando_demonio`, siempre por la tabla de adopción de 3.10.2**, tenga o no `last_seen_at` y haya sido la salida limpia o sucia. Los pasos 3 a 6 no se le aplican: estaba esperando, así que nadie miraba cuando se cerró la terminal (`tracker_caido` si hay prueba de cierre, `viva_pausada` si hay algo vivo, gracia si nunca se vio).
3. **Tramo abierto.** Solo puede quedar después de un corte sucio, porque SIGTERM los cierra. Se cierra en el `last_activity_at` de su fila de `time_tracking_sessions` (la única fuente de la última actividad, 3.10.1), con motivo `tracker_caido`. No se extiende hasta `last_seen_at`: no se sabe qué pasó después.
4. **Sesión viva** → `viva_pausada`. El tramo siguiente empieza con la primera actividad posterior a la `frontera`, así que el hueco **no suma**.
5. **Sesión no viva que el demonio ya había visto** (`last_seen_at` no nulo) → `cerrada`, con `terminada_at` por la regla única de 3.10.15 (en la práctica, `last_seen_at`).
   - Motivo `terminal_cerrada` si la instancia anterior salió limpia. Es el caso normal con `Linger=no`: se cerró la sesión del escritorio, murieron las terminales y el demonio recibió SIGTERM.
   - Motivo `tracker_caido` si fue un corte sucio.
6. **Sesión no viva, nunca vista y sin anclas** (un `esperando_terminal`) → recibe una gracia completa desde el arranque y, si vence sin terminal, pasa a `terminal_no_detectada` (3.10.2). No se cierra a ciegas: puede ser una terminal de instancia única que nunca se iba a ver.

**SIGTERM.** Llega en estos casos:

- al cerrar la sesión del escritorio o apagar el equipo;
- con `systemctl stop` o `restart`, y con el `try-restart` de una actualización;
- con el `stop` del restore (§3.6, paso 3);
- con el `disable --now` del kill switch.

Antes de cerrar nada, el demonio **relee la config**:

- **`daemon_enabled = false`:** es el kill switch. Termina las sesiones con `tracker_apagado` (3.10.12).
- **`true` (DT7):**
  - Cierra el tramo abierto de cada sesión con la regla de fin de 3.10.4, o sea hasta la última actividad, con motivo de tramo `tracker_detenido` (3.10.15, Enmienda 1).
  - **No** termina el token: lo pasa a `viva_pausada` (si estaba en `viva_activa`), y guarda el `last_seen_at` del último sondeo y la evidencia pendiente.
  - Si fue un `restart` y la terminal sigue viva, el paso 4 la readopta: quedan dos filas y el hueco no suma.
  - Si fue el cierre de la sesión del escritorio, el paso 5 termina el token en el próximo inicio, con `terminal_cerrada`. **Ojo con el orden:** con `Linger=no` y `UserStopDelaySec` en su default de 10 s (comentado en `logind.conf` de esta máquina), en un logout de KDE las konsole mueren unos 10 s antes del SIGTERM, y el demonio, que sondea cada 5 s, probablemente ve la terminal muerta y cierra con `terminal_cerrada` **antes** de recibir SIGTERM. Los dos resultados son correctos (el tramo cierra en la UA en los dos) y el paso manual 27 acepta cualquiera.

Las sesiones `file_watch` siguen como siempre: SIGTERM las cierra. Después el demonio
pone `pid = NULL`, avisa `STOPPING=1` y sale con 0 (§3.4).

**Persistencia de `last_seen_at`.** Se escribe:

- con el volcado del tramo (≤ 1 cada 5 s en `viva_activa`);
- cada 60 s en `viva_pausada` y en `terminal_no_detectada`;
- una vez al recibir SIGTERM.

Un corte sucio pierde, como mucho, 60 s de `last_seen_at`.

**Lo que el demonio no toca.**

- Las sesiones `dueno = app`: las mide la app, y si la app se cayó, las recupera ella (3.10.11).
- Los tokens que no están en la base.

**Kill switch al arrancar.** Con `daemon_enabled = false` antes de abrir la base (§3.4,
A1), el demonio sigue sin tocar la base: `READY`, `STOPPING` y salida 0. Las
sesiones que hayan quedado vivas las cierra la app (3.10.12).

#### 3.10.11 Arranque y cierre de la app

**Cerrar la app (D-R8-1): ninguna sesión termina.** `CloseRequested` (`main.rs:78-98`)
deja de llamar a `stop_active_session_sync` (`commands/mod.rs:1740`) y pasa a llamar a
`al_cerrar_ventana(db, active)`:

| Sesión | Qué hace |
|---|---|
| `dueno = demonio`, en cualquier estado (también `terminal_no_detectada`) | **Nada**: la sigue el demonio |
| La que mide la app (`MedicionApp`) | Cierra el tramo con el `Instant`, como hoy, con motivo de tramo `cierre_app`, y pasa el token a `esperando_demonio` (traspaso `app → demonio`, 3.10.7). Si la terminal sigue abierta cuando el demonio la ve, se sigue midiendo por terminal |
| Adopción imposible (`adopcion_imposible()`, 3.10.7: tracker apagado en Ajustes, Windows, desarrollo o, en F1–F5, sin el binario del tracker) | Como hoy: termina la sesión de la app con `cierre_app` (U4) |

Igual que hoy, es síncrono y corto: un `UPDATE` en una transacción.

**Al abrir la app:**

1. **La tarjeta lee la base** (`estado_trabajar`, 3.10.7). Ve todas las sesiones vivas, con su estado, su dueño y su origen, y las del demonio aparecen con Parar. Si `demonio_vivo()` da falso, las sesiones `dueno = demonio` que no están en `esperando_demonio` muestran el aviso de "tracker caído"; las que esperan, "Esperando al tracker". Precedencia y textos en 3.10.14. La cabecera aplica la regla de aviso de 3.10.14 (U6): alarma solo si se espera el tracker y no responde en dos sondeos seguidos, y la línea "Trabajar mide sin vigilancia" si la app mide una sesión.
2. **Kill switch** (§3.8, paso 2; desde el 55). Con `daemon_enabled = false`, después del `disable --now`, la app cierra lo que quede (3.10.12).
3. **Huérfanas de la app** (commit 19, en `.setup()`).
   - Toca solo las filas `source = 'work_button'`, que son las que mide la app, con las reglas de §3.2 (más de 2 min y duración del latido).
   - El tramo se cierra con motivo de tramo `huerfana`. Si tiene token, el token **no** termina: pasa a `esperando_demonio`, como si se hubiera cerrado la ventana. Con adopción imposible (3.10.7), el token termina con `huerfana` (U4).
   - Tope de 12 h (U6): la duración que se le reconoce es `MIN(<la de §3.2>, 43200)`. Si la huérfana pasó el tope, su motivo de tramo es `tope_sin_tracker` en lugar de `huerfana`.
   - Las 81 huérfanas reales no tienen token: solo se cierra el tramo (0 s, `huerfana`) y no se toca ninguna sesión. Probado sobre una copia de la base (3.10.1).
   - Una fila `source = 'terminal'` no se toca nunca en este paso: es del demonio.
4. **Sesiones del demonio sin demonio** (commit 58, R8-8, solo Linux). Si `demonio_vivo()` da falso, al abrir la app y en cada sondeo de la tarjeta:
   - La app escanea `/proc` con la lib (`procesos::escanear`, 3.10.3) y **solo cierra las sesiones que ya no tienen terminal y que se sabe que la tuvieron** (el demonio las había visto, `last_seen_at` no nulo, o tienen anclas): el tramo abierto en su `last_activity_at` y el token con `terminada_at` por la regla única de 3.10.15, con motivo `tracker_caido`. Las nunca vistas y sin anclas quedan como están, con Parar (misma regla que 3.10.10, paso 6). El escaneo usa la firma pura de 3.10.3, con los tokens vivos y los reclamos leídos de la base.
   - Las que siguen vivas **no se tocan**: no hay adopción silenciosa (3.10.7).
   - Esperar a que `demonio_vivo()` dé falso (latido de más de 45 s) evita pisar al demonio durante un `restart`. Si igual se cruzan, los cierres condicionales hacen que gane uno solo.
5. **El demonio vuelve.** Si en un sondeo `demonio_vivo()` pasa a verdadero mientras la app mide una sesión, la app **no** se la pasa en caliente (3.10.7): la sigue midiendo hasta Parar, Trabajar en otro proyecto o el cierre de la ventana, y ahí la traspasa. La tarjeta agrega: "Volvió el tracker: cuando cierres la app, sigue la terminal de X."

La exclusión "mientras el demonio esté disponible" de la primera versión de R8, que usaba
`ServiceManager`, desaparece. La reemplazan `demonio_vivo()` y la regla de no adoptar.

#### 3.10.12 Kill switch

**El problema.** Ajustes → Apagar escribe la config y enseguida corre `disable --now`
(§3.8). `disable --now` manda SIGTERM en milisegundos, antes de que el demonio
relea la config, cosa que hace cada 5 s. Con la DT7 vieja, SIGTERM cerraba el tramo
pero no el token, y la siguiente apertura de la app adoptaba la sesión como
`dueno = app`. Era un traspaso silencioso, el mismo que este diseño prohíbe.

**La corrección, en tres capas:**

1. **El demonio relee la config al recibir SIGTERM (3.10.10).**
   - Con `daemon_enabled = false`, termina todas las sesiones `dueno = demonio` que estén en un estado vivo. Cierra el tramo abierto con la regla de fin de 3.10.4, y el token con `terminada_at` por la regla única de 3.10.15, con motivo `tracker_apagado`.
   - Después pone `estado = 'desactivado'` y `pid = NULL`, avisa `STOPPING=1` y sale con 0.
   - Hace lo mismo si se entera por la relectura de cada 5 s (por ejemplo, si alguien editó el JSON).
2. **La app cierra lo que quede.** `disable --now` espera a que el servicio pare (hasta `TimeoutStopSec = 15 s`). Después, la app corre `cerrar_por_kill_switch()`: la misma operación, con el mismo motivo, sobre toda sesión `dueno = demonio` que siga viva.
   - Cubre tres casos: el demonio no estaba corriendo, el demonio murió antes de leer la config, o la config se editó con la app cerrada. Para el último, también corre en el paso 2 de la apertura (3.10.11).
   - Es idempotente: si el demonio ya las cerró, no toca nada.
3. **La sesión que mide la app no se toca:** no es del tracker.

**Toast:** "Apagaste el tracker: se cortaron las sesiones de A y B, medidas hasta su
última actividad. Para seguir midiendo, tocá Trabajar: sin el tracker, la app mide un
proyecto a la vez, hasta que toques Parar (como mucho 12 h)."

Con el tracker apagado, la CLI no crea sesiones (3.10.16).

**Pruebas.**

- E20 se extiende con un `script` marcado, y se espera `cerrada/tracker_apagado`.
- Un E2E nuevo cubre la carrera: config en `false` y SIGTERM inmediato, antes de la relectura de 5 s. Se espera `tracker_apagado`, nunca un token vivo.

#### 3.10.13 Sin demonio (antes de F6, kill switch, no instalado, desarrollo)

Vale también con el demonio caído.

- **Trabajar crea sesiones `dueno = app`.** Se miden con el `Instant` y el latido de 60 s, como hoy (§3.2), y **un proyecto a la vez** (3.10.7). Si la app deja de medir una sesión sin que se toque Parar (Trabajar en otro proyecto, cerrar la ventana, que la app se caiga, el tope de 12 h), la sesión no termina: queda en `esperando_demonio` y se retoma cuando vuelva el tracker; ese rato no se mide (U4). La excepción es la adopción imposible (3.10.7), donde termina como hoy. Mientras la mide la app, la sesión está en `viva_activa` (3.10.2), sin pausa ni descuento de la cola. En F1–F5 la adopción es imposible (3.10.7: no hay binario del tracker), así que en `main` estas sesiones terminan como hoy y ninguna queda en espera.
- **Tope, marca y aviso (U6, decidida).** Tres cosas acotan la excepción de §1.2:
  - **Tope de 12 h** por tramo `work_button` con token: constante `TOPE_SIN_TRACKER = 43200 s` (no es configurable). Parar, Trabajar en otro proyecto y cerrar la ventana recortan la duración a 12 h (commit 17, R8-3); el latido de 60 s cierra el tramo al llegar, con `tope_sin_tracker` (19); la recuperación de huérfanas también recorta (19).
  - **Marca `sin_vigilancia`** en el desglose: se **deriva** de `source = 'work_button'` con `terminal_token` no nulo, sin columna nueva. Chip: "sin vigilancia: medido hasta Parar, sin detección de inactividad". No confundir con el motivo de tramo `sin_vigilancia` de `file_watch` (3.10.15): son dominios distintos (una marca del desglose y un valor de `motivo_cierre`), y el texto de cada uno lo aclara.
  - **Aviso** (regla completa en 3.10.14). (1) **Alarma de cabecera**, solo si se espera el tracker y no corre: "El tracker no está corriendo: no se mide el tiempo automático; Trabajar mide sin pausa (hasta 12 h)". (2) **Línea de Trabajar**, mientras viva una sesión `dueno = app`, en cualquier contexto: "Trabajar mide sin vigilancia: no se pausa por inactividad y se corta a las 12 h". (3) **Toast al empezar Trabajar** con `dueno = app`, solo si se espera el tracker (el de 3.10.2). La tarjeta lo repite (3.10.14). Con el tracker apagado a propósito, en desarrollo o en F1–F5, sin sesión, no hay aviso. Entran con el commit 17 (R8-3), que ya tiene `demonio_vivo` en `estado_trabajar()`; desde el 66 la cabecera se integra al indicador global de 3.9.
- **La marca se inyecta igual en la terminal.** Es lo que le permite al demonio, cuando arranque, seguir la sesión por la terminal.
- **No se detectan el cierre de la terminal ni la inactividad.** La tarjeta de la sesión que mide la app y la de las que están en espera usan los textos de 3.10.14 ("Midiendo con Trabajar (sin tracker…)" y "Esperando al tracker: se va a medir desde que arranque").
- **La CLI sí crea la sesión** (corrección aprobada). Crea el token con `dueno = demonio` y `estado = esperando_demonio`, y ancla la shell. El tiempo cuenta desde que el demonio la adopta (3.10.2, tabla de adopción; 3.10.10, paso 2), no antes. Avisa: "El tracker no está corriendo: la sesión de X queda esperando y se mide desde que arranque." Ya no existe el "no se mide" (3.10.16).
- **Una sesión en espera cuya terminal se cerró** se cierra con `tracker_caido` si hay prueba de que se cerró (anclas de la CLI, o el demonio la había visto). La cierra el demonio al arrancar (3.10.10, paso 2, por la tabla de adopción de 3.10.2) o, si el demonio sigue sin estar, la app (3.10.11, paso 4, desde el commit 58 (R8-8)). Las del botón que nadie vio pasan por la gracia y quedan en `terminal_no_detectada` hasta Parar.
- **Descartado:** que la app corra el motor por su cuenta mientras está abierta. Obligaría a traspasar el dueño con tramos abiertos. Lo que mide la app sin demonio se acota con el tope, la marca y el aviso de arriba (U6).
- **Antes del commit 58 (R8-8)** nadie cierra las sesiones en espera cuya terminal ya murió: quedarían en la tarjeta con Parar. En `main` no pasa: en F1–F5 la adopción es imposible y ninguna sesión entra en espera (3.10.7), y F6 (53 a 63) se mergea entera. Solo puede verse dentro de la rama de F6, entre el 54 y el 58 (riesgo 49).

#### 3.10.14 UI: tarjeta, toasts y desglose

**Lectura.** La tarjeta lee el estado de `get_live_tracking` (64): sesiones vivas con
`token`, `estado`, `dueno`, `origen`, `creada_at`, `last_seen_at`, tramo abierto y
cantidad de terminales. Relee cada 5 s con la ventana visible, al ganar el foco y
después de cada acción. Los tests de `83c48d4` (`ProjectList.work.test.tsx`) se
conservan.

**Indicador de la tarjeta, por estado:**

| Estado | Texto |
|---|---|
| `esperando_terminal` | "Esperando la terminal…" |
| `esperando_demonio` | "Esperando al tracker: se va a medir desde que arranque". Si el tracker corrió alguna vez (hay fila en `tracker_status`) y ahora no está vivo, agrega "(el tracker no está corriendo desde las 14:32)". **Nunca** muestra la alarma de "tracker caído": este estado existe justamente cuando no hay demonio |
| `viva_activa`, `dueno = demonio` | "Midiendo · 2 terminales" (+ "· desde la terminal (gestor)" si `origen = cli`) |
| `viva_activa`, `dueno = app` | "Midiendo con Trabajar sin tracker: no se pausa por inactividad, se corta sola a las 12 h. Tocá Parar al terminar; si cerrás la app, la sesión queda esperando al tracker". Donde la adopción es imposible (3.10.7; también todo F1–F5): "Midiendo con Trabajar: sin pausa por inactividad, hasta Parar, hasta cerrar la app o como mucho 12 h" (U4, U6) |
| `viva_pausada` | "En pausa desde 14:32 (10 min sin actividad) · escribí en la terminal o guardá un archivo para seguir" |
| `terminal_no_detectada` | "⚠️ No se puede detectar tu terminal: se mide solo con archivos guardados mientras estés en la compu. Tocá Parar al terminar." (y "· en pausa desde 14:32" si no hay tramo abierto) |
| `terminal_no_detectada` con un ancla de multiplexor | "⚠️ Estás en tmux/screen: no puedo saber cuándo cerrás la terminal. Se mide tu tipeo en ese panel y los archivos guardados. Tocá Parar (o `gestor parar`) al terminar." |
| `terminal_no_detectada` en un proyecto no vigilado | "⚠️ Sin terminal detectada y con la carpeta sin vigilar: no hay cómo medir esta sesión. Usá konsole o cargá un ajuste." |
| **Tracker caído**: `dueno = demonio`, estado vivo **distinto de `esperando_demonio`**, `daemon_enabled = true`, `demonio_vivo()` falso **y** el demonio corrió alguna vez (fila en `tracker_status`) o el servicio está instalado | "⚠️ El tracker no está corriendo desde las 14:32: no se está midiendo. Tocá Trabajar para medirla con la app, o Parar (guarda hasta la última actividad)." Tiene prioridad sobre los textos de `esperando_terminal`, `viva_*` y `terminal_no_detectada` (auditoría de consistencia), no sobre `esperando_demonio` |

**Aviso de cabecera sin tracker (U6).** Va en la cabecera, no en la tarjeta. Lo decide
una sola función pura, `aviso_cabecera(entrada) -> AvisoCabecera` (distinta del campo
`aviso_sin_tracker` que devuelve `trabajar` para el toast), con estos casos:

| Caso | Condición | Qué se ve |
|---|---|---|
| (a) Alarma | El tracker **se espera**: `!adopcion_imposible()` (3.10.7); el demonio **corrió alguna vez** (hay fila en `tracker_status`) **o**, desde el 66, el servicio está instalado; `demonio_vivo()` da falso en **2 sondeos seguidos** (≥ 10 s, para que no parpadee en un `restart`), y el estado no es `esperando_migracion` (es transitorio: lo resuelve la app al migrar) | Línea persistente: "El tracker no está corriendo: no se mide el tiempo automático; Trabajar mide sin pausa (hasta 12 h)". Desaparece en el primer sondeo con `demonio_vivo()` verdadero |
| (b) Línea de Trabajar | Hay una sesión `dueno = app` viva, en cualquier contexto (apagado, desarrollo, F1–F5, caído) | "Trabajar mide sin vigilancia: no se pausa por inactividad y se corta a las 12 h", mientras viva la sesión. Si también vale (a), se ven las dos |
| (c) Tracker apagado a propósito, sin sesión de la app | — | Solo el "Tracker apagado" del indicador global (3.9) |
| (d) "No disponible" (desarrollo) o F1–F5, sin sesión de la app | — | Nada |

El texto anterior, "el tiempo se mide sin vigilancia", daba a entender que se seguía
midiendo; sin demonio el tiempo automático no se mide y solo Trabajar mide, sin pausa.
El aviso es independiente del indicador de cada tarjeta: con el tracker caído, la
tarjeta de una sesión del demonio muestra además "tracker caído" (tabla de arriba). Lo
agrega el commit 17 (R8-3), con (a) atado a la fila de `tracker_status`; el 66 le suma
"o el servicio está instalado" y lo integra al indicador global (3.9).

**Contador.** Con `dueno = demonio` es la suma de las `duration` de los tramos del
token: la base, sin interpolar. Avanza a saltos, con hasta ~10 s de retraso, y
**nunca baja ni salta al cerrar** (2a). Con `dueno = app` es `elapsed_seconds` del
`Instant`, como hoy (M5).

**Toasts por transición** (el sondeo compara estados):

- Trabajar con `dueno = app` y el tracker esperado (`!adopcion_imposible()`) → el toast de 3.10.2 ("El tracker no está corriendo: …", U6). Con adopción imposible, sin toast.
- Tramo cerrado con `tope_sin_tracker` → "La sesión de X llegó a 12 h sin el tracker y se cortó. Si seguías trabajando, tocá Trabajar."
- `cerrada/terminal_cerrada` → "La sesión de X terminó al cerrar la terminal: 1 h 12 min."
- `tracker_apagado` → el texto de 3.10.12.
- `cerrada/tracker_caido` → "El tracker se cortó: la sesión de X se guardó hasta su última actividad (14:32)."
- `restauracion` → lo lista el diálogo del restore (3.10.9).
- `proyecto_a_papelera` → el toast de 3.10.9.
- `viva_pausada` → sin toast; solo el indicador.
- Parar con `YaCerrada` → como en 3.10.9.

**Desglose ("¿de dónde sale este tiempo?")**, por proyecto y por día o rango. Lo arma
el comando nuevo `desglose_tiempo(project_id, rango) -> Desglose`, en la lib
`db/desglose.rs` con su envoltorio Tauri. Muestra:

| Bloque | Contenido |
|---|---|
| Tramos | Hora de inicio y fin, duración, origen ("Trabajar", "Trabajar desde la CLI", "Archivos (automático)", "Trabajar sin tracker (medido hasta Parar)") y motivo de cierre en español (3.10.15) |
| Marcas por tramo cobrado (chips) | `remoto_ssh` → "remoto: ssh 35 min" · `presencia_desconocida` → "sin dato de presencia 12 min" (bit 8, también en tramos `file_watch`, U2) · `simultaneo_con:<id>` → "simultáneo con B: 1 h 50" · `sin_vigilancia` → "sin vigilancia: medido hasta Parar, sin detección de inactividad" (tramos `work_button` con token, U6) · "sin tipeo" (todos los minutos vinieron de archivos con presencia; informativo) · "superpuesto con archivos: no suma doble" · "incluye 60 s antes del primer guardado" (DX1) |
| **Solo archivos, sin presencia** (marca `solo_archivos`, U1) | Tramos derivados de los minutos con bit 32 de `trabajar_evidencia` y `archivos_evidencia` (3.10.4): "Solo archivos, sin presencia: 14:40–15:05 (25 min), no se cobró", con la duración al segundo y el botón **"Cargar como ajuste"**, que precarga desde, hasta y segundos y pide la nota. Aparecen también en proyectos sin ninguna sesión de Trabajar (tiempo automático que no se cobró) |
| Pausas | Entre dos tramos del mismo token: "Pausa 14:32–15:10 (38 min sin actividad): no se cobró", con el botón **"Cargar como ajuste"**, que abre el formulario de 3.10.17 con el horario y la nota a completar (auditoría de facturación, AF-P2) |
| Lo ignorado | Desde la evidencia: "6 min de tipeo fuera de la carpeta del proyecto: no se cobraron" (bit 16). Los archivos sin presencia van en su propio bloque (arriba) |
| Detalle de un minuto | Al expandir un tramo: los minutos con su fuente (tipeo, archivo y su ruta relativa, remoto). Es la respuesta a "¿por qué se cobró 14:37?" |
| Ajustes | Los del rango (3.10.17), con su nota, signo, origen y fecha; los revertidos, tachados con su reversión |
| Totales | "Medido 5 h 40 · ajustes −30 min · **total 5 h 10**" · "simultáneo con B: 1 h 50 en el día" |

**Ajustes → Tracking:**

- "Pausa de Trabajar por inactividad" (1–120 min, default 10), con el valor efectivo de `tracker_status.trabajar_idle_minutes`.
- "Presencia del escritorio: disponible (KDE)", o "no disponible: <motivo>. Los cambios de archivos cuentan sin verificar que estés y quedan marcados en el desglose".

**Pruebas** (Vitest en el commit 69 (R8-9) y el commit 70 (R8-A2)):

- un texto por fila de la tabla del indicador (10 casos, con "tracker caído" ganándole a `viva_activa` y **perdiendo** contra `esperando_demonio`, y sin "tracker caído" cuando el demonio nunca corrió);
- transición a `terminal_cerrada` → toast con la duración, y `viva_pausada` → sin toast;
- contador de una sesión del demonio que no baja entre sondeos ni al cerrar;
- desglose de un día con una pausa, una suspensión, un `ssh` y un simultáneo → tramos, chips y total igual a `tiempo_total`;
- "Cargar como ajuste" prellena el horario, desde una pausa y desde un tramo `solo_archivos` (desde, hasta y segundos al segundo; uno que cruza la medianoche → dos ajustes);
- un tramo `solo_archivos` de un proyecto sin Trabajar (solo `archivos_evidencia`) se muestra y no suma al total;
- chip `sin_vigilancia` en un tramo `work_button` con token, y toast de `tope_sin_tracker`;
- reabrir la app con una sesión del demonio viva → Parar visible;
- aviso de cabecera (commit 17; desde el 66, dentro del indicador global): con `adopcion_imposible` falso, fila en `tracker_status` y `demonio_vivo` falso en dos sondeos seguidos → alarma; en un solo sondeo → nada (`restart`); sin fila → nada; con `adopcion_imposible` verdadero → nada; con una sesión `dueno = app` viva → la línea "Trabajar mide sin vigilancia", también con adopción imposible; la alarma desaparece en el sondeo en que `demonio_vivo` pasa a verdadero. En el 66, además: servicio instalado y sin fila → alarma; Apagado sin sesión → nada.

#### 3.10.15 Explicabilidad: motivos de cierre

Hay **dos** columnas con el mismo nombre y dominios distintos:
`trabajar_sesiones.motivo_cierre` dice por qué terminó la **sesión** y
`time_tracking_sessions.motivo_cierre` dice por qué se cortó el **tramo**. Cuando el
tramo se corta porque termina la sesión, llevan el mismo valor. Todo tramo
`terminal` cierra en la **última actividad** (2a), así que la columna `ended_at` no
cambia por motivo.

**Motivos de sesión** (los 7 canónicos de la revisión de R8, más los 3 que acepta la
Enmienda 1 (Anexo A, v3.2) y `tope_sin_tracker`, que agrega la decisión U6; la Enmienda 2 extiende
`cambio_de_proyecto` a `dueno = demonio` cuando la misma terminal pasa a otro proyecto
con la CLI).

**Regla única de `terminada_at`** (corrige que varias fórmulas quedaran antes del fin de
un tramo y rompieran la invariante 9). Un solo método de `db/trabajar.rs`, que usan
todos los cierres de sesión, calcula, en la misma transacción que cierra el último
tramo:

`terminada_at = MAX(<fórmula del motivo>, MAX(ended_at) de los tramos del token)`

con `MAX(ended_at)` ignorado si el token no tiene tramos. Así `terminada_at` nunca es
anterior al `ended_at` de ningún tramo, aunque `last_seen_at` falte (sesión `dueno = app`
restaurada, `terminal_no_detectada` que midió solo archivos), sea viejo (ciclo
`demonio → app → esperando_demonio`) o quede antes de un guardado con presencia que
cayó entre el último sondeo que vio la terminal y el que la vio muerta.

| Motivo | Quién | Fórmula del motivo (antes del `MAX`) | Texto del desglose |
|---|---|---|---|
| `terminal_cerrada` | Demonio | `last_seen_at` | "Cerraste la terminal" |
| `parar_app` | App | El clic | "Tocaste Parar" |
| `parar_cli` | CLI | El comando | "Corriste `gestor parar`" / "Seguiste en la terminal sin medir (`--sin-medir`)" |
| `tracker_apagado` | Demonio, o la app si el demonio no llegó (3.10.12) | `COALESCE(last_seen_at, creada_at)` | "Apagaste el tracker" |
| `tracker_caido` | Demonio al arrancar después de una salida sucia; demonio al adoptar una sesión en espera cuya terminal ya no está; app sin demonio (3.10.11) | `COALESCE(last_seen_at, creada_at)` | "El tracker se cortó; se contó hasta la última actividad registrada" / "El tracker no estaba corriendo cuando se cerró la terminal" |
| `proyecto_a_papelera` | App (`delete_project`) | El momento de mandarlo a la papelera | "Mandaste el proyecto a la papelera" |
| `restauracion` | App (restore) | `COALESCE(last_seen_at, creada_at)` de la base restaurada (3.10.9) | "Se restauró un backup" |
| `cambio_de_proyecto` | CLI (`gestor abrir Y` o `--sin-medir` hacia otro proyecto saca la última terminal de X, Enmienda 2); app solo con adopción imposible (Trabajar en otro proyecto) | El comando o el clic | "La terminal pasó a otro proyecto" / "Empezaste a trabajar en otro proyecto (sin tracker)" |
| `cierre_app` | App, solo `dueno = app` y adopción imposible (3.10.7, U4) | El cierre | "Cerraste la app (sesión sin tracker)" |
| `huerfana` | App, solo `dueno = app` y adopción imposible (3.10.7, U4) | Regla de §3.2 | "La app se cortó sin cerrar la sesión" |
| `tope_sin_tracker` | App (latido o recuperación de huérfanas), solo `dueno = app` y adopción imposible (U6) | `started_at + 12 h` del tramo | "Llegó a 12 h sin el tracker y se cortó" |

"Adopción imposible" quiere decir `adopcion_imposible()` verdadero (3.10.7: Windows,
`GESTOR_NO_TRACKERD` definida o sin el binario del tracker en `BIN_DIR` desde el commit 17 (R8-3), lo que en F1–F5 vale siempre; tracker apagado desde el 35; desde el 54, las condiciones de instalación de 3.8, desarrollo incluido, en lugar del binario). Con el tracker posible, cerrar la app, Trabajar en otro
proyecto, una caída de la app o el tope de 12 h **no** terminan la sesión
`dueno = app`: cierran su tramo con `cierre_app`, `cambio_de_proyecto`, `huerfana` o
`tope_sin_tracker` como **motivo de tramo**, y la sesión pasa a `esperando_demonio`
(D-R8-1, U4).

**Motivos de tramo:** los de sesión, más los cortes que no terminan la sesión:

| Motivo | `ended_at` | Texto del desglose |
|---|---|---|
| `inactividad` | Última actividad (hasta −8 s por el pty) | "10 min sin actividad: se pausó" |
| `suspension` | Última actividad | "El equipo se suspendió" |
| `reloj` | Última actividad | "Se ajustó el reloj del sistema" |
| `tracker_detenido` | Última actividad | "Se detuvo el tracker (reinicio, actualización o cierre de la sesión del escritorio)" |
| `sin_vigilancia` | Último cambio | Solo `file_watch`: "La carpeta dejó de vigilarse" (cambió de carpeta o dejó de ser medible). No es la marca `sin_vigilancia` del desglose (U6, 3.10.13), que se deriva del `source` |
| `tope_sin_tracker` | `started_at + 12 h` | Solo `work_button` con token (U6): "Llegó a 12 h sin el tracker y se cortó" |

Las filas `file_watch` también llevan motivo desde el commit 15 (R8-1) (`inactividad`, `suspension`,
`reloj`, `tracker_detenido`, `tracker_caido` para la huérfana del arranque,
`tracker_apagado`, `proyecto_a_papelera`, `sin_vigilancia`). Así la Definición de
listo puede exigir "ninguna fila cerrada desde el commit 15 (R8-1) sin motivo".

**Consultas de auditoría** (para el desglose y el recorrido manual):

```sql
-- Los tramos de una sesión y su porqué
SELECT started_at, ended_at, duration_seconds, motivo_cierre
FROM time_tracking_sessions WHERE terminal_token = ? ORDER BY started_at;

-- Qué sostuvo cada minuto
SELECT minuto, fuentes, archivo_rel FROM trabajar_evidencia
WHERE token = ? ORDER BY minuto;
```

#### 3.10.16 R8-CLI: medir desde `gestor`

**Cómo es hoy.** La CLI es `~/.local/bin/gestor`, un script de bash con `fzf` de 756
líneas que **no** está en el repo. `docs/cli.md:7-8` lo dice así: "No es parte de
`src/` ni de `src-tauri/` […] Vive en `~/.local/bin/gestor`".

- Resuelve la base con `GESTOR_DB:-$HOME/.local/share/gestor-proyectos/projects.db` (`:29`).
- Escribe la ruta en `GESTOR_CD_FILE` (`entregar`, `:193-214`).
- Registra el "abierto" con un `UPDATE` directo (`registrar_open`, `:178-191`).
- La función `gestor()` del bloque `~/.zshrc:112-133` (y de `~/.bashrc:141-162`; líneas de las marcas, medidas el 2026-10-20) hace el `cd` en la shell del usuario.
- Los dos archivos tienen dos bloques entre marcas: `# >>> gestor-proyectos cli >>>` y `# >>> gestor-proyectos completion >>>`.

**El problema de identidad.** La shell ya existe. Un `export GESTOR_SESION=T` dentro de
la función **no** aparece en el `/proc/<pid>/environ` de esa shell, que solo refleja el
entorno del `execve` (proc_pid_environ(5)). Lo ven únicamente los hijos que se lancen
después, así que con la marca sola una shell ociosa no contaría.

| Alternativa | A favor | En contra | Veredicto |
|---|---|---|---|
| **A. Ancla: registrar `(pid, starttime)` de la shell y su pty en la base, y además exportar la marca** | Identifica la shell existente sin tocarla. `starttime` evita la reutilización de pids. La inactividad sale del mismo pty. Los hijos que llevan la marca (pestañas nuevas, `script`, `tmux attach`) siguen sumando | Una tabla más (`trabajar_anclas`) y reglas de propiedad del pty | **Elegida** (DT8) |
| B. `exec env GESTOR_SESION=T $SHELL` desde la función | Usa el mismo mecanismo que el botón | Pierde el estado de la shell (variables sin exportar, historial en memoria, jobs). Es intrusivo y sorprende | Descartada |
| C. Solo la marca exportada | No agrega nada | La shell no cuenta: la sesión muere en la gracia o queda en `terminal_no_detectada` | Descartada |
| D. Solo el pty (`/dev/pts/N`) | Simple | Los números de pty se reutilizan apenas se cierra uno: no sirven como identidad | Descartada como identidad (sí se usa para la actividad) |

**El ancla.**

- Es la shell interactiva, identificada por `(pid, starttime)` y su `tty_nr`. El binario los lee de `/proc/<pid>/stat`, parseado después del último `)` (3.10.3). No vienen del script.
- El pid llega en `GESTOR_SHELL_PID=$$`. Medido en bash 5 y zsh 5.9: `$$` es la shell principal dentro de una función, de `( … )`, de un pipe y de una sustitución de comando.

**Propiedad del pty: un pty pertenece a un solo token a la vez.**

- El reclamo vigente más reciente de un pty (un ancla o un veto) decide a quién pertenece. Los procesos con la marca de **otro** token que corren en ese pty no cuentan para ese otro token.
- **Ejemplo:** se lanzó `npm run dev &` en esa terminal mientras medía X, y después se hizo `gestor abrir Y`. El `npm` no sostiene a X, y la actividad del pty va a Y.

**Vetos.** `gestor parar` y `--sin-medir` registran, para la shell donde se corren, un
**veto** `(pty, pid, starttime, token = NULL)`. Rige mientras viva esa shell, o hasta el
próximo `gestor abrir` en ella, y hace que **ningún** proceso marcado de ese pty
cuente. Cierra dos huecos:

- **`--sin-medir` en una terminal abierta con el botón.** Esa terminal no tiene ancla, y la marca está en su `environ`: sin el veto, seguiría contando el proyecto viejo desde otra carpeta.
- **`gestor abrir Y` y después `gestor parar`.** Al cerrarse Y se libera el reclamo del pty, y la shell, que todavía tiene la marca de X en su `environ`, volvería a sostener a X si X sigue viva por otra terminal.

**Semántica (D-R8-5):**

| Comando | Efecto |
|---|---|
| `gestor` (menú) o `gestor abrir X` | Hace el `cd`, igual que hoy, y **además** mide. Si X no tiene sesión viva, crea el token (`origen = cli`, `dueno = demonio`) y ancla esta shell. Si X ya tiene una sesión viva (la del botón u otra terminal), se **une** al mismo token (D-R8-4), suma esta shell como ancla y escribe `ultimo_clic_at = ahora` (el clic llega al demonio, 3.10.1). Si el token se cerró en la misma ventana (carrera con `terminal_cerrada`, 3.10.2), reintenta una vez y crea uno nuevo. Si la sesión de X la mide la app, el ancla queda registrada y la app sigue midiendo, hasta que se la pase al demonio (3.10.7). Si esta shell ya estaba anclada a X, no hace nada: "Ya estás midiendo X en esta terminal." |
| `gestor abrir Y` en una shell anclada a X | La shell **deja de contar para X** (`hasta = ahora`, `motivo_salida = cambio_de_proyecto`) y se ancla a Y. El subcomando escanea con la lib: si a X no le queda ninguna terminal viva, X termina en ese instante con `cambio_de_proyecto` (Enmienda 2); si le quedan otras, sigue |
| `gestor parar` | Termina la sesión **entera** de la terminal actual (todas sus terminales), igual que el botón Parar. La sesión sale, en este orden, del ancla de esta shell, del reclamo del pty o de la `GESTOR_SESION` heredada. Imprime "→ Paraste X (1 h 12 min)", veta el pty, y la función hace `unset GESTOR_SESION`. Si no hay sesión: "No hay ninguna sesión midiendo en esta terminal.", con código 0 |
| `gestor parar X` | Termina la sesión de X, desde cualquier terminal. Veta el pty solo si la sesión de X era la de esta terminal |
| `gestor estado` | Solo lectura: las sesiones vivas, con proyecto, estado, terminales y tiempo de hoy. Si `demonio_vivo()` da falso, lo avisa |
| `--sin-medir` (`gestor --sin-medir`, `gestor abrir --sin-medir X`) | Hace el `cd` sin medir: suelta el ancla de esta shell, si la hay (`motivo_salida = sin_medir`), y veta el pty. Después el subcomando `soltar` **escanea con la lib** (como `gestor abrir Y`), contando ya el veto, y si a la sesión que medía esta terminal no le queda ninguna terminal viva, la termina **en ese instante** en vez de dejar que el demonio la cierre como `terminal_cerrada` con la terminal abierta. Motivo: `cambio_de_proyecto` si el destino es otro proyecto (Enmienda 2) y `parar_cli` si es el mismo (el usuario pidió dejar de medirlo en esa terminal). Imprime "→ Dejaste de medir X (1 h 12 min). Esta terminal sigue sin medir." Vale igual para una terminal del botón, que no tiene ancla pero sí la marca (E34) |
| `gestor list`, `ayuda` y `crear` | No miden. **`crear` tampoco**, aunque después se haga el `cd`: dar de alta un proyecto no es empezar a trabajarlo. Hoy `crear` ni siquiera hace `cd` (`cli.md:133`); para medir, se corre `gestor abrir` después |
| `( gestor abrir X )` en una subshell | `$$` es la shell principal (medido), así que se ancla la principal, pero el `cd` y el `export` se pierden al salir de la subshell. Queda documentado. Por la regla del primer plano (3a, 3.10.3), el tipeo cuenta para X solo si el programa en primer plano está en la carpeta de X |

**Dentro de tmux o screen.**

- **El problema.** Un ancla dentro de un panel de tmux sobrevive al cierre de la konsole (la shell vive en el server), y la sesión nunca terminaría por terminal. La validación `tty_nr ≠ 0` no lo detecta, porque el panel tiene su propio pty.
- **La corrección.** El subcomando recorre los ancestros de la shell. Si hay un `tmux: server` o un `screen` con `tty_nr = 0` (3.10.3), registra el ancla como **de multiplexor** (columna `multiplexor` de `trabajar_anclas`, 3.10.1). Ese ancla no sostiene la sesión: solo aporta tipeo.
- **Estado.** Si la sesión no tiene otra terminal, pasa a `terminal_no_detectada` cuando vence la gracia. Ese estado es una pausa (3.10.2): sale con Parar o `gestor parar`, o cuando aparece una terminal real. **El tipeo en el panel sí cuenta ahí**: `terminal_no_detectada` tiene su transición de tipeo de un ancla de multiplexor (3.10.2), con la misma atribución por cwd (3a) y la regla de los dos `atime` desde una pausa. Sin eso, "aporta tipeo" sería letra muerta.
- **Aviso en la CLI:** "Estás dentro de tmux: no puedo saber cuándo cerrás la terminal. La sesión de X se corta con `gestor parar`."

**Quién escribe (DT9).** La CLI **no** arma SQL para las sesiones. Lo hace un subcomando
del binario del demonio, con la lib:

```
gestor-trackerd sesion iniciar --proyecto <id> --shell-pid <pid> [--data-dir D] [--config-dir C]  # stdout: token
gestor-trackerd sesion parar   [--proyecto <id> | --token <t>] [--shell-pid <pid>] [rutas]        # stdout: segundos
gestor-trackerd sesion soltar  --shell-pid <pid> [rutas]                                          # --sin-medir
gestor-trackerd sesion estado  [--json] [rutas]
```

- **Qué base usa.** El orden de precedencia es `--data-dir`, después `GESTOR_DB` (su carpeta; el archivo tiene que llamarse `projects.db`), y por último el módulo de rutas de la lib (`XDG_DATA_HOME`, como la app).
  - El script le pasa **siempre** `--data-dir "$(dirname "$DB")"`, con el `DB` que ya resolvió en la línea 29. Así los dos procesos miran la misma base, también en los E2E con `GESTOR_DB` temporal.
  - Para la config, el orden es `--config-dir` y después `XDG_CONFIG_HOME`.
  - Aplica la guarda B8: con `GESTOR_TRACKERD_TEST_TIMINGS` definida, se niega a usar la base real.
- **Sin migrar.** Abre la base con `abrir_existente`, sin crearla y **sin migrar** (principio 5). Si `user_version ≠ ESQUEMA_ACTUAL` o `journal_mode ≠ wal`, dice: "Abrí la app una vez para actualizar la base; por ahora no se mide."
- **Cómo escribe.** Cada escritura es una transacción `IMMEDIATE` con los métodos del commit 15 (R8-1), dentro del `flock` compartido de `restore.lock` (3.10.7).
- **Con demonio y sin demonio.** Lo decide `demonio_vivo()` (3.10.7). Con demonio, el token nace en `esperando_terminal` y el demonio lo adopta en ≤ 1 sondeo. Sin demonio, nace en `esperando_demonio` (3.10.13). En los dos casos se escribe el ancla.
- **Tracker apagado** (`daemon_enabled = false` en la config, decisión U5): no crea nada, la función hace el `cd` igual (la terminal queda abierta en la carpeta, sin medir) y avisa en una línea: "Tracker apagado en Ajustes: no se mide." La razón no es que apagar sea "no medir": con el tracker apagado el botón Trabajar **sí** mide, con la app (U4). Es que nadie podría medir esta sesión: sin demonio nadie vigila la terminal, la CLI no tiene una app que mida por ella, y una sesión en espera la cerraría el kill switch con `tracker_apagado` apenas se abriera la app (3.10.12).
- **Demonio caído** (auditoría de facturación, AF-18): con `demonio_vivo()` falso y fila en `tracker_status` (el demonio corrió alguna vez), `iniciar`, `estado` y `parar` avisan por stderr "El tracker no está corriendo desde las HH:MM: revisá `systemctl --user status gestor-trackerd`". Es el único aviso fuera de la app cuando el unit queda `failed` con la app cerrada; costo cero, porque el subcomando ya consulta `demonio_vivo()`.
- **Valida `--shell-pid`.** El pid tiene que ser un ancestro del propio subcomando (por la cadena de `ppid`), del mismo UID, con estado distinto de `Z` y con `tty_nr ≠ 0`. `starttime` y `tty_nr` los lee el binario.
- **Rechaza** un proyecto que no existe o que está en la papelera (3.10.9).
- **`parar` sobre una sesión `dueno = app`.** El subcomando no conoce el `Instant` de la app. Cierra el tramo en su `last_activity_at`, con la duración del último latido: es la regla de huérfanas de §3.2 (con su tope de 12 h), que cuenta de menos (60 s como mucho) y nunca inventa tiempo. La app suelta su `MedicionApp` cuando el siguiente latido devuelve `false`.
- **Qué binario es.** El mismo que instala F6 en `BIN_DIR`, enlazado con la lib (no con Tauri), así que arranca en milisegundos. No lanza un demonio y **no** toma `trackerd.lock`, que retiene el demonio vivo.
- **Códigos de salida.** La CLI hace el `cd` siempre: medir nunca rompe la navegación.

  | Código | Significado |
  |---|---|
  | `0` | Sesión creada, unida o parada. Los avisos van a stderr, en español (esperando al demonio, tmux) |
  | `3` | No se mide por una condición conocida: tracker apagado (U5), restauración en curso, esquema atrasado, proyecto inexistente o en la papelera |
  | `2` | Error de uso o de validación: `--shell-pid` que no es ancestro, o sin tty |
  | `1` | Error inesperado |

**Cómo llega el token a la shell.**

- La función crea un segundo temporal, `GESTOR_SESION_FILE`, junto a `GESTOR_CD_FILE`. Los dos se crean con `mktemp`, en `0600`.
- El script escribe ahí `set <token>` o `unset`.
- Después del `cd`, la función hace `export GESTOR_SESION=<token>` (validado contra `^[0-9a-f]{32}$`) o `unset GESTOR_SESION`.
- La función también pasa `GESTOR_SHELL_PID=$$`.

**Por qué hace falta la función (corrige la justificación de `$PPID`).** La primera versión de R8 decía
que, con `cd "$(command gestor abrir x)"`, el `$PPID` del script "sería una subshell que
muere enseguida". **Es falso.** Medido en bash 5 y zsh 5.9: en la sustitución de un
comando simple, la subshell hace `exec` directo, y el `$PPID` del comando es la shell
principal. Pasa lo mismo en una función, en `( … )` y en un pipe. Solo cambia en una
sustitución compuesta (`$(cmd; true)`), donde `$PPID` es la subshell.

Las razones verdaderas para medir solo desde la función son tres:

1. Solo la función puede hacer `export GESTOR_SESION` en la shell. Sin eso, las pestañas, `script` o `tmux attach` que se abran después no llevan la marca.
2. Solo la función sabe que el proceso que se ancla es la shell interactiva. Invocado directo, el `$PPID` del script puede ser otro script, `xargs`, `watch` o una subshell compuesta: procesos que mueren enseguida y dejan una sesión de segundos.
3. El `unset` de `gestor parar` también lo hace la función.

Invocado directo (sin `GESTOR_SESION_FILE`), el script hace lo de hoy: imprime la ruta.
Además avisa: "Para medir, usá la función `gestor` (corré
`scripts/cli/instalar-cli.sh --rc`)."

**Caminos de "elegir un proyecto" que no miden** (R8-CLI dice "mide siempre por
defecto"; estas son todas las excepciones, y **todas** avisan por stderr en español y
hacen el `cd` igual):

| Camino | Por qué no mide | Código del subcomando |
|---|---|---|
| Script invocado directo, o función `gestor` vieja (sin `GESTOR_SESION_FILE`) | No hay cómo anclar la shell interactiva ni exportar la marca | — (no se llama al subcomando) |
| Falta el binario `gestor-trackerd` en `BIN_DIR` (antes de F6 o instalación rota) | No hay quién escriba la sesión | — |
| `--sin-medir` | Lo pidió el usuario | 0 |
| Tracker apagado en Ajustes (U5) | Sin demonio nadie vigila esta terminal, y la CLI no tiene una app que mida por ella (el botón sí: U4) | 3 |
| Restauración en curso (`restore.lock` exclusivo) | La base se va a reemplazar | 3 |
| Esquema atrasado (`user_version ≠ ESQUEMA_ACTUAL`) o `journal_mode ≠ wal` | El subcomando no migra (principio 5) | 3 |
| Proyecto inexistente o en la papelera | No se mide un proyecto que no está activo | 3 |
| `--shell-pid` que no es ancestro, del mismo UID, vivo y con tty | Validación (riesgo 43) | 2 |
| Error inesperado (base corrupta, I/O) | — | 1 |

Ninguno de estos es el camino por defecto: con la función nueva instalada, el binario
presente y el tracker encendido, elegir un proyecto mide. Sin demonio **sí** mide (queda
`esperando_demonio`, 3.10.13), y dentro de tmux o screen también (ancla de multiplexor).

**Versionar la CLI sin tocar tu `.zshrc` sin permiso.**

- **Archivos en el repo:** `scripts/cli/gestor` (el script), `scripts/cli/gestor.sh` (la función, para bash y zsh), `scripts/cli/gestor-completion.bash`, `scripts/cli/gestor-completion.zsh` y `scripts/cli/instalar-cli.sh`.
- **El commit 60 (R8-C1) los copia byte a byte** de lo instalado: `~/.local/bin/gestor` y los bloques actuales de `~/.zshrc` y `~/.bashrc`. La CLI no cambia hasta el commit 62 (R8-C3). Antes de mergear el commit 60 (R8-C1), se hace un `diff` contra lo instalado.
- **`instalar-cli.sh`** instala el script en `$BIN_DIR` (con un temporal + `mv`), y la función y el completado en `$PREFIX/lib/gestor-proyectos/cli/`. No usa `$PREFIX/share/gestor-proyectos`, porque esa es la carpeta de la base.
  - Las variables tienen default, como en `install.sh`: `PREFIX`, `BIN_DIR` y `HOME`.
  - Así se puede probar en `scripts/check.sh` con `HOME=$tmp PREFIX=$tmp`, sin el binario release que exige `install.sh:14`.
- **Los rc solo se tocan con `--rc`.**
  - Sin la flag, `instalar-cli.sh` no escribe `~/.zshrc` ni `~/.bashrc`: muestra el bloque nuevo, el `diff` contra el actual y cómo aplicarlo.
  - Con `--rc`, primero hace una copia (`~/.zshrc.gestor-<fecha>`). Después reemplaza **solo** lo que está entre las marcas existentes, o agrega el bloque al final si las marcas no están.
  - El bloque nuevo es una sola línea **protegida**, con la ruta ya resuelta: `[ -f "$PREFIX/lib/gestor-proyectos/cli/gestor.sh" ] && source "$PREFIX/lib/gestor-proyectos/cli/gestor.sh"`. Las versiones siguientes de la función se actualizan copiando ese archivo, sin volver a tocar los rc, y si la CLI se desinstala sin `--rc`, cada shell nueva arranca sin error (y sin la función).
- **`install.sh`** llama a `instalar-cli.sh` **sin** `--rc`.
- **`uninstall.sh`** borra el script y la carpeta de `lib/`. Con `--rc`, también saca los bloques de los rc. Sin `--rc` no los toca y avisa: "La función `gestor` sigue en tu `~/.zshrc` y `~/.bashrc`, pero ya no hace nada: corré `./uninstall.sh --rc` para sacarla." Se eligió el `source` protegido más el aviso, y no exigir `--rc`: desinstalar no tiene que tocar los rc sin permiso (la misma regla de `--rc` que la instalación), y con el bloque protegido ninguna shell queda rota.
- **`docs/cli.md`** deja de decir que la CLI vive fuera del repo.
- **Con la función vieja** (sin `GESTOR_SESION_FILE`), el script nuevo no mide y avisa cómo actualizarla (riesgo 42).

La razón para versionarla es que la CLI pasa a depender del subcomando y del esquema.
Una CLI fuera del repo se desincroniza con la próxima migración sin que nadie se
entere. Un paquete aparte no tiene sentido, porque comparte la base y el binario.

#### 3.10.17 Ajustes manuales (DT13, decisión 5a)

Entra en la entrega única (decisión U7): el commit 22 (R8-A1) en F1, el commit 63 (R8-C4) en F6 y el commit 70 (R8-A2) en F7.

**Cambio de alcance** (§1.2): la v3.1 dejaba fuera de alcance "Editar sesiones a
mano". La v3.2 dice "Ajustes manuales: sí, solo de agregado. Editar o borrar tramos
medidos: no". Las mediciones nunca se tocan; se corrigen sumando o restando aparte.

**Tabla `ajustes_tiempo`** (paso 5, commit 22 (R8-A1), en F1 después del 21: necesita
`tiempo_trabajado` (20) y las estadísticas por unión (21)):

| Columna | Tipo | Contenido |
|---|---|---|
| `id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | — |
| `project_id` | `INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE` | — |
| `segundos` | `INTEGER NOT NULL CHECK (segundos <> 0 AND abs(segundos) <= 86400)` | Con signo: + suma, − resta. Hasta 24 h por ajuste |
| `desde`, `hasta` | `DATETIME NULL`, `CHECK ((desde IS NULL) = (hasta IS NULL))`, `CHECK (hasta IS NULL OR hasta > desde)` | Opcionales. A qué rato o día corresponde |
| `nota` | `TEXT NOT NULL CHECK (length(trim(nota)) > 0)` | Obligatoria |
| `creado_en` | `DATETIME NOT NULL` | UTC, con el formateador de `time.rs` |
| `origen` | `TEXT NOT NULL CHECK (origen IN ('app','cli'))` | — |
| `revierte_id` | `INTEGER NULL UNIQUE REFERENCES ajustes_tiempo(id)` | Si este ajuste anula otro. `UNIQUE`: no se anula dos veces |

**Solo de agregado, garantizado por la base** (verificado con sqlite3 3.53 sobre un
esquema equivalente):

```sql
CREATE TRIGGER ajustes_tiempo_sin_update BEFORE UPDATE ON ajustes_tiempo
BEGIN SELECT RAISE(ABORT, 'ajustes_tiempo es solo de agregado'); END;

CREATE TRIGGER ajustes_tiempo_sin_delete BEFORE DELETE ON ajustes_tiempo
WHEN EXISTS (SELECT 1 FROM projects WHERE id = OLD.project_id AND deleted_at IS NULL)
BEGIN SELECT RAISE(ABORT, 'ajustes_tiempo es solo de agregado'); END;
```

El `DELETE` de un ajuste de un proyecto vivo falla, igual que cualquier `UPDATE`. La
purga de un proyecto, desde la papelera o directa, sí borra sus ajustes en cascada:
durante la cascada la fila del proyecto ya no existe, así que el `EXISTS` da falso.
Lo verifiqué, incluida una reversión que apunta a otro ajuste del mismo proyecto.
Restaurar un backup reemplaza la base entera, y eso no es una edición.

**Reglas de escritura** (lib, `db/ajustes.rs`, `agregar_ajuste(...)` y
`revertir_ajuste(id, nota, origen)`, con transacción `IMMEDIATE`):

- Una reversión copia `project_id`, `desde` y `hasta`, pone `segundos = −original` y exige que el original no sea a su vez una reversión.
- Con horario (`desde`/`hasta` dentro del mismo día) y signo +: `segundos ≤ hasta − desde`. Además, el formulario avisa si el rato se superpone con tiempo ya medido ("De 15:00 a 16:00 ya hay 20 min medidos: ¿cargo solo 40 min?") y propone el resto. Los totales **suman** (5a), así que sin este aviso se contaría doble.
- Los mensajes de validación van en español rioplatense, porque el usuario los puede corregir (`CLAUDE.md`, "Mensajes de error"). Ejemplos: "Escribí una nota: es lo que vas a mostrar si te preguntan por este ajuste." y "Un ajuste no puede pasar de 24 h. Si necesitás más, cargá uno por día."

**Cómo entra en los totales y en los rangos:**

- `tiempo_total = max(0, unión de tramos + Σ ajustes)` por proyecto y por rango (3.10.8). Si el resultado se recorta en 0, el desglose lo dice ("los ajustes restan más de lo medido; el total queda en 0").
- Un ajuste cae en el rango que contiene su **fecha efectiva**, `COALESCE(desde, creado_en)`, entera y sin prorratear.
- El formulario ofrece dos modos. "Sumar o restar N min **el día D**" guarda `desde`/`hasta` como los límites locales de ese día. "**Un rato** de HH:MM a HH:MM" guarda ese intervalo. Sin `desde`, el ajuste cuenta el día en que se cargó.
- No entran en la unión ni en la simultaneidad, ni en `session_count`, `avg` o `longest`.

**UI mínima** (commit 70, R8-A2, F7, en el desglose de 3.10.14):

- Botón "Agregar ajuste": signo (sumar/restar), cantidad (h y min), modo (día o rato) y nota obligatoria. "Cargar como ajuste" desde una pausa o desde un tramo `solo_archivos` (U1) lo prellena en modo rato, con desde, hasta y segundos al segundo (partido en dos si cruza la medianoche); el foco queda en la nota.
- Lista de ajustes del rango: fecha, signo y duración, nota, origen (app/CLI) y botón "Revertir", que pide una nota. Nunca hay "editar" ni "borrar".

**CLI** (`origen = cli`; commit propio el commit 63 (R8-C4), después del commit 62 (R8-C3)):

```
gestor ajustar X -30m "Reunión interna, no facturable" [--dia 2026-10-21 | --desde 15:00 --hasta 16:00]
gestor ajustar --revertir <id> "Lo cargué en el proyecto equivocado"
gestor ajustes X [--semana]
```

La CLI llama a `gestor-trackerd ajuste agregar|revertir|listar`, que usa la lib, sin
SQL en el script (DT9). No hace falta que el demonio esté vivo: no hay nada que
medir. Resuelve la base con `--data-dir`/`GESTOR_DB` (corrección aprobada en la revisión de R8) y rechaza mientras
hay un restore en curso (corrección aprobada en la revisión de R8).

**Pruebas:**

- El commit 22 (R8-A1) (Rust): el trigger bloquea `UPDATE` y `DELETE` en un proyecto vivo; la purga directa y desde la papelera borra en cascada; revertir dos veces → error de `UNIQUE`; nota vacía o de espacios → error; `segundos = 0` → error; `desde` sin `hasta` → error; un rato con más segundos que el intervalo → error en español; `tiempo_total` con ajustes dentro y fuera del rango; recorte en 0; un ajuste sin `desde` cuenta el día de `creado_en` (con zona horaria inyectada, D13); migración de 4 a 5 sobre una base con datos.
- El commit 70 (R8-A2) (Vitest): formulario con validaciones; el aviso de superposición propone el resto; "Revertir" pide nota; tachado del revertido; "Cargar como ajuste" desde un tramo `solo_archivos` precarga desde/hasta/segundos y no guarda sin nota.
- E2E E34, extendido: `gestor ajustar X -30m "nota"` → una fila `origen = cli`, y `gestor ajustes X` la lista.

## 4. Fases y commits

Cada commit compila y pasa `scripts/check.sh` (5.5), que se crea en el commit 2.
Se usa la convención del repo: `checkpoint:` en español y `git add` de archivos
concretos, nunca `git add .`. Hay una rama por fase (`feat/tracker-f0`, …,
`feat/tracker-f5a`, `feat/tracker-f5b`, …) que se mergea a `main` con fast-forward
cuando pasa su validación, igual que el B14.

**Total: 73 commits `checkpoint:`** en 10 fases (74 con DX5, 9.2), más el commit
`release:` que genera `scripts/release.sh` al final de F8. F0 (1–9) está hecha. La
columna "Hallazgos" remite al Anexo A y, en los 17 commits de R8 (marcados
**R8-x.**, con su identificador del plan R8), a las decisiones de §1.1 y de 3.10. En
los commits heredados de la v3.1, **R8:** marca lo que R8 les agrega.

**Regla de orden:** ningún commit usa algo que crea un commit posterior. Las
dependencias que importan están en la columna "Depende de"; la verificación
completa está en 4.10.

**Validación al cerrar cada fase** (antes del merge):
1. `scripts/check.sh` en verde.
2. Desde F5b: `scripts/check.sh --e2e` (E2E tres veces seguidas, 5.2).
3. El paso manual de la fase, si lo tiene (5.3).
4. F1 y F2, además: el diff con datos reales (commit 24) sobre una copia fresca de la base real, y la app abierta contra esa copia (5.4).

### F0 — Puerta de calidad y arreglos heredados

Hecha en la rama `feat/tracker-f0`: 1 `476a936` · 2 `f2d48c7` · 3 `c0d0a6f` · 4
`fa35369` · 5 `c9f5e74` · 6 `9017485` · 7 `895955f` · 8 `a3f5e76` · 9 `0d2fb1f`.
`83c48d4` (la tarjeta refresca el total después de Parar) queda fuera de la
numeración. Las líneas de código de esta tabla son las de `main@c8b55ae`, antes de
cada commit.

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 1 | `cargo fmt` de todo el crate, sin otro cambio. Antes del commit, `cargo fmt --check` reportaba **127** bloques de diferencia (medido sobre `main@c8b55ae`); desde `476a936` sale con 0 | `cargo test` sin cambios; `git diff --stat` solo de formato | B6a | — |
| 2 | `scripts/check.sh` (5.5). Si `cargo clippy --all-targets -- -D warnings` no pasa sobre `main`, los avisos se corrigen en este mismo commit, sin cambios de comportamiento | Correr el script en verde; romper el formato de un archivo a propósito → el script falla | B6a | 1 |
| 3 | `end_tracking_session` idempotente: `WHERE id = ? AND ended_at IS NULL`; si afecta 0 filas, rollback y **no** corre el `UPDATE` del contador (`db/tracking.rs:38-43`); devuelve `Ok(false)` | Cerrar dos veces suma **una** vez (contador y `duration_seconds` intactos tras el 2º); se mantiene `test_end_tracking_session_with_unknown_id_credits_no_project` | P2, B3 | — |
| 4 | Base, config, nivel de log y auto-backup pasan a correr **después** del plugin de instancia única (`main.rs:29-53` contra `:60`), con el orden de 3.8: `Builder::build()` → `Database::new` + `app.manage(db)`, `config`, `ActiveSession` **síncronos** → `app.run()`; el auto-backup y todo trabajo lento van en un hilo lanzado desde `.setup()`. Un error de `Database::new` sale como `Err` con mensaje | Paso manual 1 (5.3): lanzar la app dos veces seguidas → una ventana y **un** backup nuevo; un `invoke` inmediato desde la ventana (por ejemplo `get_all_projects` en el primer render) no falla por estado no registrado | P11, D15 | — |
| 5 | `get_project`/`update_project` sin `try_lock` (`db/projects.rs:113,154`): pasan a `self.conn()`; se reescribe el comentario de `:111-112` | Un hilo sostiene el lock 200 ms: `get_project` espera y devuelve `Ok`; con el Mutex envenenado, funciona | P16, D7 | — |
| 6 | "Trabajar" muestra el error en español (`ProjectList.tsx:209-213`) | Vitest: toast si `startWorkSession` falla | P7 | — |
| 7 | Botón **Parar** (`api.ts:507` gana llamador) | Vitest + test del comando | P9 | — |
| 8 | Regla única de duración mínima: `duracion_a_guardar(source, segundos)` para cambio de proyecto, Parar y cierre de ventana (`commands/mod.rs:1569-1579`, `:1615-1621`); el `let _` de `:1578` propaga | Parar a los 5 s → 0 s con `ended_at` real; cambio de proyecto a los 5 s → 0 s; 11 s → 11 s por los tres caminos; un error de base en el cierre de 0 s sube | M8 | 7 |
| 9 | "Abrir" del Dashboard usa el flujo de "Trabajar" (`Dashboard.tsx:55-65`) | Vitest | P8 | 6 |

### F1 — Esquema versionado, sesiones de Trabajar, recuperación, unión y ajustes

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 10 | Migraciones versionadas: `migrar()` con `user_version` y `BEGIN IMMEDIATE`; paso 1 = esquema actual; solo se ignora `duplicate column name`; `user_version` futuro → error en español. `Database::abrir_existente()` para el demonio (sin `SQLITE_OPEN_CREATE`, devuelve `EstadoEsquema`) | (a) esquema viejo real (`schema.rs:294`) termina en `ESQUEMA_ACTUAL`; (b) segunda corrida no hace nada; (c) otra conexión con `BEGIN IMMEDIATE` y `busy_timeout` corto → `Err` y nada cambia; (d) `user_version` futuro → `Err`, base intacta; (e) un error que no es "duplicate column" se propaga; (f) `abrir_existente` sobre ruta inexistente no crea el archivo | P14, B1 | — |
| 11 | `time.rs`: formato y parser únicos, `rango_dia<Tz: TimeZone>` (medianoche ambigua e inexistente), `rango_semana` = 7 fechas locales, `chrono-tz` en `[dev-dependencies]`; excepción documentada de `deleted_at` | Ida y vuelta con `CURRENT_TIMESTAMP`; tabla de zonas de 5.1 (Buenos Aires, Santiago, Havana, Madrid), cada una afirmando su precondición; semana = 7 fechas | P3, D4, D5, D13 | — |
| 12 | `last_activity_at` + índice `(project_id, ended_at)` como paso 2 | De `user_version` 1 a 2; esquema viejo real → 2 con columna e índice | P4 | 10 |
| 13 | Tabla `tracker_status` como paso 3, con todas las columnas de 3.2 (incluida `scan_completed_at`) y `Database::leer_tracker_status() -> Option<…>` | Existe tras migrar; sin fila → `None`; columnas esperadas | B6b | 12 |
| 14 | Sesiones con instante explícito: `abrir_sesion`, `latir`, `cerrar_sesion` con `Result<bool>`; `create_/end_tracking_session` quedan como envoltorios | Abrir escribe `last_activity_at = started_at`; cerrar dos veces → `false`; `latir` sobre cerrada o id inexistente → `false` | D14, M7 | 12 |
| 15 | **R8-1.** `checkpoint: esquema paso 4 para sesiones de Trabajar atadas a la terminal`. `trabajar_sesiones` (con `CHECK` de estado y motivo, `adoptada_at`, `ultimo_clic_at`, sin `ultima_actividad_at`), `trabajar_anclas` completa (3.10.1: `token` nullable, `pid`, `starttime`, `tty_nr`, `multiplexor`, `desde`, `hasta`, `motivo_salida` con `sesion_terminada`), `trabajar_evidencia` (con `ausente_desde_s`/`ausente_hasta_s`), `archivos_evidencia` (U1), `terminal_token` y `motivo_cierre` (con el `CHECK` de tramo de sesión con motivo y `tope_sin_tracker` en las listas, U6) en `time_tracking_sessions`, `source = 'terminal'`, `presencia*` y `trabajar_idle_minutes` en `tracker_status`, los tres índices parciales (el de tramo abierto, **único**); `ESQUEMA_ACTUAL = 4`. `db/trabajar.rs` con todos los métodos condicionales en `transaction_with_behavior(Immediate)`, `terminar_sesiones_de(ids, fin, motivo)` (con la regla única de `terminada_at` de 3.10.15 y el cierre de las anclas abiertas), `pasar_a_esperando_demonio(token)`, `cerrar_por_terminal(token, escaneo)` condicional (3.10.2, sin carrera), `cerrar_reclamos_muertos(...)` y `registrar_evidencia(...)` / `registrar_evidencia_archivos(...)` (UPSERT con OR de bits y MIN/MAX de los segundos). `cerrar_sesion(id, fin, dur, motivo)`. `duracion_a_guardar('terminal', s) = s`. `demonio_vivo(conn, ahora)` (con `comm`). `end_tracking_session` a `IMMEDIATE`. Helper `flock` compartido de `restore.lock` | De 3 a 4 y del esquema viejo real a 4; dos sesiones vivas en un proyecto → error; terminar dos veces → `false`; `abrir_tramo` sobre un token cerrado → `None`; Parar y "terminal cerrada" a la vez desde dos conexiones → un solo `true`; sesión cerrada sin motivo → error de `CHECK`; tramo con token cerrado sin motivo (`UPDATE` e `INSERT`) → error de `CHECK`, y las filas viejas sin token cerradas sin motivo se aceptan; dos tramos abiertos de un token → error de índice; UPSERT de evidencia con OR de bits y los segundos sin presencia (cuatro combinaciones de nulos; bit 32 sin segundos → error de `CHECK`); `archivos_evidencia` con un bit distinto de 8 o 32 → error de `CHECK`; tramo de 0 s → borrado; borrar el proyecto → cascada; `terminada_at` nunca menor que `MAX(ended_at)` (sesión sin `last_seen_at` con tramos de archivos); `cerrar_por_terminal` con un `ultimo_clic_at` o un ancla posteriores al escaneo → no cierra; `demonio_vivo` (sin fila, `desactivado`, `pid NULL`, latido 44/46 s, pid inexistente, pid vivo con otro `comm`); `end_tracking_session` con dos conexiones | R8, DT1, DT5, DT12, DT14, U1, U6 | 10, 13, 14 |
| 16 | **R8-2.** `checkpoint: la terminal de Trabajar nace con la marca de la sesión y sin zombis`. `open_terminal(path, config, marca: Option<&MarcaSesion>)` en el trait y las dos plataformas; `argv_terminal` con la tabla de flags; `comando_script()` privado (la marca nunca llega a navegador ni editor); helper `lanzar(cmd)` con hilo `wait()` para **todos** los spawns de `linux.rs`; `try_terminal_fallback` con `match spawn()`; el comando `open_terminal` pasa `None`; `limpiar_entorno_heredado()` (`remove_var("GESTOR_SESION")`) antes de `logging::init()`; `scripts/check.sh --windows` (`cargo check --locked --target x86_64-pc-windows-gnu`, fuera del check por defecto porque la caché de cargo no tiene las dependencias de Windows y bajarlas necesita red: paso manual `cargo fetch` del usuario o CI, 3.10.3) | Golden de `argv_terminal` (la marca es **un** elemento del argv); terminal falsa en un `tempdir` del `PATH` que vuelca argv y `environ` → la marca está en los dos, y ausente sin marca con el padre marcado; reaper: la terminal falsa escribe su pid y se sondea hasta que `/proc/<pid>` desaparece; `comando_script` de navegador/editor sin marca; `limpiar_entorno_heredado` probado lanzando el binario de test con `GESTOR_SESION`; salida de `check.sh --windows` pegada en el PR | R8 | — |
| 17 | **R8-3.** `checkpoint: Trabajar y Parar leen la sesión de la base y conviven por proyecto`. `trabajar(project_id)` (reutiliza el token vivo, D-R8-4; dueño `app` hasta el commit 58 (R8-8); rechaza proyecto en la papelera y con `restore.lock` tomado); `parar(project_id, token, origen) -> ResultadoParar { Cerrada, YaCerrada{motivo, segundos}, Ninguna }`; `estado_trabajar()`; `ActiveSession(Mutex<Option<MedicionApp>>)`; Trabajar en otro proyecto cierra el tramo de la app y pasa el token a `esperando_demonio`; `al_cerrar_ventana` en `CloseRequested`; `tracker_soportado()` (Linux, sin `GESTOR_NO_TRACKERD` y con `BIN_DIR/gestor-trackerd` presente) y `adopcion_imposible()` **solo** con `!tracker_soportado()` (la cláusula `daemon_enabled` entra en el 35, 3.10.7): en F1–F5 da verdadero y todo termina como hoy; `ultimo_clic_at` al crear o reutilizar; la sesión `dueno = app` vive en `viva_activa`; `TOPE_SIN_TRACKER = 43200` (U6): Parar, Trabajar en otro proyecto y cerrar la ventana recortan a 12 h (el latido que corta en el tope llega con el 19), y Trabajar con `dueno = app` devuelve `aviso_sin_tracker` para el toast solo si `adopcion_imposible()` es falso; `estado_trabajar()` devuelve además `adopcion_imposible` y si el demonio corrió alguna vez (fila en `tracker_status`), y la cabecera aplica `aviso_cabecera` de 3.10.14 (U6: alarma, línea de Trabajar o nada). Salen `start_work_session`, `stop_work_session`, `get_work_session_status`; el campo `previous_session_stopped` pasa a `sesion_anterior`. Frontend: `api.trabajar`, `parar`, `estadoTrabajar`; `trabajar` abre la terminal en el backend (con la marca), así que `src/utils/workSession.ts` (`trabajarEnProyecto`) deja de recibir el callback `abrirTerminal`, y sus dos llamadores lo sueltan: `Dashboard.tsx:52-67` y `ProjectList.tsx:230`; la tarjeta relee al ganar el foco y después de cada acción | Rust: A y B con la app → A `esperando_demonio`, una sola `MedicionApp`, A en `viva_activa` mientras se mide; A dos veces → mismo token y `ultimo_clic_at` nuevo; Parar → `Cerrada`, otra vez → `Ninguna`, con token cerrado → `YaCerrada{motivo}`; cerrar la ventana con un `gestor-trackerd` falso en el `BIN_DIR` temporal → tramo `cierre_app`, token en espera; sin ese binario (F1–F5) → `cerrada/cierre_app`, y Trabajar en otro proyecto → `cerrada/cambio_de_proyecto`; Windows simulado → `cerrada/cierre_app`; spawn que falla con token nuevo → token borrado; Parar con un `Instant` de 13 h (reloj inyectado) → 43200 s; Trabajar con `dueno = app` → `aviso_sin_tracker` con el binario presente, y sin aviso con adopción imposible; tabla de `aviso_cabecera` (los cuatro casos de 3.10.14, dos sondeos contra uno); los tests de `work_session_tests` reescritos o ajustados (§5.6). Vitest: los tres de `ProjectList.work.test.tsx` de `83c48d4` (193, 211 y 226) conservan su intención; los usos de `previous_session_stopped` (`api.test.ts:863-875`, `test-setup.ts:142`, `Dashboard.test.tsx:243`, `ProjectList.work.test.tsx:234`) pasan a `sesion_anterior`; `workSession.ts` sin `abrirTerminal` (hoy no tiene test propio: se cubre por `Dashboard.test.tsx` y `ProjectList.work.test.tsx`); toast de `YaCerrada`; toast "El tracker no está corriendo…" con `aviso_sin_tracker`; cabecera: alarma con fila en `tracker_status` y dos sondeos sin demonio, nada sin fila o con adopción imposible, línea "Trabajar mide sin vigilancia" con una sesión de la app (3.10.14); relectura al ganar el foco | D-R8-1, D-R8-2, D-R8-4, DT2, DT14, U4, U6 | 7, 8, 9, 15, 16 |
| 18 | **R8-3b.** `checkpoint: mandar a la papelera corta las sesiones vivas del proyecto y sus descendientes`. `delete_project` cierra en la misma transacción `IMMEDIATE` (con el `Instant` si la mide la app) y toast | Papelera de un padre con sesiones en padre e hijo → las dos `proyecto_a_papelera`; un fallo del `UPDATE` no deja sesiones cortadas; `trabajar` sobre un proyecto en la papelera → error en español | D-R8-1 (excepción) | 15, 17 |
| 19 | Latido de "Trabajar" cada 60 s (`last_activity_at` **y** `duration_seconds` del `Instant`) + recuperación de huérfanas `work_button` en `.setup()` (> 2 min, reglas de 3.2) **R8:** El latido de 60 s es de la única `MedicionApp` y aplica el **tope de 12 h** (U6, 3.10.13): al llegar cierra el tramo en `started_at + 12 h` con `tope_sin_tracker` y pasa el token a `esperando_demonio` (o lo termina con `tope_sin_tracker` si la adopción es imposible). Huérfanas: solo `source = 'work_button'`; el tramo cierra con motivo `huerfana` (o `tope_sin_tracker` si pasó las 12 h, con la duración recortada a 43200) y el token pasa a `esperando_demonio` (o termina con ese motivo si la adopción es imposible, 3.10.7). Una fila `terminal` nunca se toca. | Huérfana con latido y suspensión → cierra con la duración, no con la diferencia; sin latido → 0 s; sesión de otra instancia con latido reciente → intacta; las 81 huérfanas reales no cambian el total. **R8:** latido con reloj inyectado a 12 h + 1 s → tramo de 43200 s con `tope_sin_tracker` y token en espera; ídem con Windows simulado → `cerrada/tope_sin_tracker`; huérfana de 13 h → 43200 s; huérfana con token → `esperando_demonio`; Windows simulado → `cerrada/huerfana`; huérfana **sin** token → solo el tramo, con `huerfana`. El ensayo sobre una copia de la base real va en el 24, que trae la API de backup (3.10.1) | P4, M7 | 4, 8, 14, 15, 17 |
| 20 | `tiempo_trabajado(project_id, rango, activa)`: tramos de 3.3, unión y recorte; variante de una consulta para todos los proyectos **R8:** `activa` **sigue** siendo `Option<SesionActiva>`. Filas explícitas de `terminal` en la tabla de tramos (3.10.8) y `simultaneidad()`. | Disjuntos, superpuestos, anidados, que se tocan; abiertos de cada tipo; cruce de medianoche; regla de 10 s; `activa` con suspensión cuenta los segundos del `Instant`; `file_watch` abierto hasta `last_activity_at`; `ended_at` muy posterior no amplía el tramo; `activa` con id cerrado se ignora. **R8:** terminal 9–18 con suspensión 12–14 y `file_watch` 16–18 → 7 h; pausa con `file_watch` adentro y los 60 s de DX1; A 10–12 y B 11–13 → 1 h simultánea en cada uno (el paraguas se prueba en el 42) | R4, M5, M6 | 15, 19 |
| 21 | Lectura por unión (9a de la auditoría): `get_time_stats` y `get_project_stats` sobre la unión en hora local, `active_today` nuevo, errores propagados, `State<ActiveSession>` con orden `ActiveSession → Database`. **No** saca el contador **R8:** Copia la `MedicionApp` y suelta el lock (sin `Vec`). | Sin superposición, mismos números salvo las correcciones documentadas; con superposición, sin doble conteo; semana = 7 fechas; `active_today` en hora local (a las 22:00 de Buenos Aires sigue siendo "hoy"); tabla borrada → `Err`, no 0; `start_work_session` y `get_time_stats` en paralelo 1.000 veces sin deadlock. **R8:** El test de concurrencia (1.000 veces) usa `trabajar` en vez de `start_work_session` | P3, P18, D2, D4, D6 | 11, 20 |
| 22 | **R8-A1.** `checkpoint: ajustes de tiempo solo de agregado (paso 5)`. `ajustes_tiempo` + triggers + `db/ajustes.rs` (`agregar_ajuste`, `revertir_ajuste`, `listar_ajustes`) + `tiempo_total` en `get_time_stats` y `get_project_stats`; `ESQUEMA_ACTUAL = 5` | Trigger bloquea `UPDATE` y `DELETE` en proyecto vivo; purga directa y desde la papelera → cascada; revertir dos veces → `UNIQUE`; nota vacía, `segundos = 0`, `desde` sin `hasta`, rato con más segundos que el intervalo → error en español; `tiempo_total` dentro y fuera del rango, recorte en 0; sin `desde` cuenta el día de `creado_en` (zona inyectada); migración de 4 a 5 con datos | 5a, DT13 | 15, 20, 21 |
| 23 | `TimeTracker` carga `getTimeStats` **siempre**; desaparecen la rama "no activado" y el botón "Track" | Vitest: sin `.gestor/` y con sesiones, muestra el total; `checkTrackingConfig` no se llama | P15, B2 | 21 |
| 24 | Diff con datos reales: test unitario `#[ignore]` en `db/golden_diff.rs` (`#[cfg(test)]`: la lib todavía no existe, así que no puede ir en `tests/`) que lee `GESTOR_GOLDEN_DB`, copia con la API de backup (activa la feature `backup` de rusqlite, que el commit 27 reutiliza) a un `tempdir` desde una conexión de solo lectura, y emite por proyecto los números viejos (SQL actual copiado en el test) contra los nuevos. Lista aparte las sesiones > 12 h, las huérfanas y los proyectos con carpeta inexistente, **sin tocar nada** | Corre local, nunca en CI. Salida esperada hoy: proyecto 3 (contador 30.348 s contra sesiones 38.448 s), semana 8 → 7 fechas, "hoy" UTC → local, 9 sesiones > 12 h (936.392 s), 5 sesiones de 1–10 s. **R8:** test `#[ignore]` de huérfanas sobre la misma copia (3.10.1): con la recuperación del 19, las 81 abiertas cierran con 0 s y `huerfana`, ninguna fila de `trabajar_sesiones` cambia y el total de segundos queda igual (ensayado a mano el 2026-10-19: 1.352.723 s antes y después) | D3 | 19, 21 |
| 25 | Sacar el contador y `add_project_time` (9b de la auditoría): la lista completa de 3.2. La columna queda en el esquema | `check.sh` en verde; `rg total_time_seconds` solo encuentra `db/schema.rs` y tests de migración; el diff del commit 24 sigue igual | P1, D2 | 21, 24 |

**Validación de F1:** diff del commit 24 sobre una copia fresca de la base real;
cada diferencia queda anotada para el CHANGELOG (commit 73). Paso manual 2 (5.3).
Además, verificar a mano que `pnpm run tauri:dev` lanzado desde una konsole de
Trabajar no propaga la marca `GESTOR_SESION` (3.10.3).

### F2 — Rutas, restore, transacciones, permisos y WAL

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 26 | `paths.rs`: `data_dir`, `db_file`, `trackerd_lock_file`, `config_file`; se borra `backup::live_db_path` | Con `XDG_DATA_HOME`/`XDG_CONFIG_HOME` temporales: `db_file()` igual a la vieja `live_db_path`; `config_file()` no crea nada | P17, D1 | — |
| 27 | Restore con la API de backup: pasos 1, 2, 4–7 y 9 de 3.6 (aviso, `flock` retenido sobre `trackerd.lock`, `ActiveSession` vaciada sin escribir, Mutex, `page_size`, `step(-1)`, `journal_mode`, `migrar`, `integrity_check`). El núcleo va en `db/restore.rs` (`tomar_lock`, `restaurar_desde`, A3), que entra en la lib con el commit 33; `backup::restore_backup` queda como envoltorio. Los pasos de systemd quedan para el commit 57 **R8:** `restore.lock` **exclusivo** desde la confirmación del diálogo (antes del paso 3) hasta el paso 8; el diálogo del paso 1 lista las sesiones vivas; paso 5 vacía `ActiveSession` sin escribir; paso 6 cierra con `restauracion` las sesiones vivas de la base **restaurada** (tramo en su `last_activity_at`, token por la regla única de 3.10.15). | Restore con `-wal` presente no corrompe; `integrity_check = ok`; con el `flock` tomado por otro proceso, aborta sin tocar nada; backup de esquema viejo → `ESQUEMA_ACTUAL`; después del restore, Parar no escribe. **R8:** backup con un token vivo → `cerrada/restauracion`; backup con una sesión `dueno = app` viva y tramos de archivos → `terminada_at ≥ MAX(ended_at)`; después del restore, Parar → `Ninguna` o `YaCerrada`; con el diálogo abierto (antes de confirmar), `trabajar` no rechaza: el `flock` compartido de `restore.lock` se obtiene | P6, B7 | 10, 15, 17, 24, 26 |
| 28 | Las 6 transacciones a `IMMEDIATE`; `empty_trash` mete el `SELECT` de ids adentro, y `restore_project`/`purge_project` meten adentro su guarda `SELECT deleted_at` (`db/trash.rs:56-66`, `:133-143`); comentario corregido **R8:** Quedan **cuatro** transacciones (`trash.rs:68/145/211`, `status.rs:75`): `end_tracking_session` ya pasó a `IMMEDIATE` en el commit 15 (R8-1) y `delete_project` (`trash.rs:22`) en el commit 18 (R8-3b) | Los existentes; dos conexiones: una escribe entre la lectura y la escritura de `restore_project` → sin `SQLITE_BUSY_SNAPSHOT`; otra conexión restaura el proyecto entre la guarda y la escritura de `purge_project` → `purge` no borra un proyecto activo | P13, D9 | — |
| 29 | Permisos: `data_dir` `0700` (también una existente), `projects.db` `0600`, backups `0600` | `#[cfg(unix)]`: modos de carpeta, base y backup | P19, D11 | 26 |
| 30 | WAL + `synchronous=NORMAL`, solo desde la app; si no queda `wal`, error visible; el restore lo vuelve a fijar | DELETE → `wal`; `:memory:` → `memory`; lectura abierta en otra conexión → `Err`, no `delete` en silencio; 1.000 escrituras desde dos conexiones sin `SQLITE_BUSY`; restore deja `wal`; `-wal`/`-shm` en `0600` | D8 | 27, 29 |
| 31 | Copia de `VACUUM INTO` a `journal_mode=DELETE` antes de verificarla | En la carpeta de backups no aparecen `-wal`/`-shm` | — | 30 |
| 32 | Comentario de `db/schema.rs:11-20`, `CLAUDE.md` (WAL) y `ARQUITECTURA.md` (backups del home, nota de downgrade del lado datos) | — | D10, D12 | 30 |

**Validación de F2 (T7):** antes del merge, copiar la base real a un `tempdir`
(commit 24 o `sqlite3 … ".backup"`), abrir la app con `XDG_DATA_HOME` apuntando
ahí (`pnpm run tauri:dev`), usar "Trabajar", Parar, papelera y un restore, y
comparar el diff. La base real no se toca hasta que la fase está en `main`.

### F3 — Biblioteca, binario y config

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 33 | `lib.rs` (`gestor_proyectos_lib`) con `db` (incluido `db::restore` del commit 27), `models`, `tracking`, `config`, `logging`, `time`, `paths`; `main.rs` reexporta para que los `crate::db` sigan resolviendo **R8:** La lib recibe `db/trabajar.rs`, `db/ajustes.rs`, `db/tiempo.rs` (con `simultaneidad`), `demonio_vivo()` y el helper de `restore.lock` | Mismos tests y mismos nombres (técnica del B14); `db::restore::restaurar_desde` visible desde `tests/` | A3 | 11, 26, 27 |
| 34 | `[[bin]] gestor-trackerd` + `default-run = "gestor-proyectos"` en el **mismo** commit. El binario arranca, toma el `flock` en `<data_dir>/trackerd.lock`, acepta `--data-dir`/`--config-dir`, aplica las guardas de debug y de `GESTOR_TRACKERD_TEST_TIMINGS`, loguea y sale. **Tarea de verificación:** `pnpm tauri build --bundles deb`, `dpkg -c` y qué queda en `target/release/`; el resultado se anota en 3.1 y en §1 | `cargo build --bins`; `cargo run` sin `--bin` lanza la app; el test toma el `flock` de `<data_dir>/trackerd.lock` y lanza el binario con **otro** `XDG_RUNTIME_DIR` y la misma base → sale con 75 (el binario de este commit sale enseguida, así que el que sostiene el lock tiene que ser el test); debug sin `--data-dir` ni `XDG_DATA_HOME` se niega; timings + base real se niega | B4, B8 | 33 |
| 35 | Sección `tracking` en `AppConfig` (`daemon_enabled`, `idle_minutes`, `min_credit_seconds`, `max_dirs_per_project`) con `#[serde(default)]` y rangos de 3.5, en la lib. Sin UI **R8:** `tracking.trabajar_idle_minutes` (default 10, rango 1–120, acotado). **Además**, `adopcion_imposible()` (3.10.7) suma la cláusula `!daemon_enabled`, que no puede existir antes de este commit. | Config vieja sin la sección → defaults; guardar y releer conserva los valores; valores fuera de rango → acotados. **R8:** `daemon_enabled = false` + cerrar la ventana con una sesión de la app → `cerrada/cierre_app`; Trabajar en otro proyecto → `cerrada/cambio_de_proyecto` | B6c | 17, 33 |

### F4 — Motor puro y sesiones de terminal

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 36 | Máquina de estados con reloj inyectado (3.4): `frontera`, margen DX1, `reanudado`, salto de reloj > 5 min, `idle_minutes` acotado, cierres coherentes, 0 filas → soltar, `daemon_enabled = false` → "cerrar todo y salir" **R8:** cada acción de cierre lleva su `motivo_cierre` de tramo (`inactividad`, `suspension`, `reloj`, `tracker_detenido`, `tracker_apagado`, `proyecto_a_papelera`, `sin_vigilancia`; 3.10.15) | Guardado aislado → **60 s**; dos guardados a 2 min → 3 min; con `min_credit_seconds = 0` → no queda fila (la v2); margen recortado por la sesión anterior y por `reanudado`; pausa 2:59 contra 3:00; **evento antes del tick**; **suspensión de 8 h**; reloj −10 min → cierre + `last_error`; `idle=0` → 1, `idle=999` → 60; dos proyectos alternando. **R8:** cada cierre con su motivo (tabla) | R1, R3, DX1, M7, M12, M14, M16, B5 | 35 |
| 37 | Validación de carpetas y atribución **al más específico** por componentes; empate exacto → los dos | `/home` rechazada; `/a/proj` no captura `/a/proj2`; `/a` y `/a/b`: evento en `/a/b/x` solo para `/a/b`, en `/a/c` para `/a`; misma carpeta → los dos | R1, R7, DX2 | 33 |
| 38 | Contención y validación en la lib, para los avisos de la app: contiene a / misma carpeta, `en_red` por `mountinfo`, sincronizada, R7 sobre ruta canonizada | 84 ⊃ 85; 90 = 91; symlink a `/` rechazado; `fuseblk` aceptado; `fuse.sshfs` rechazado (con `mountinfo` falso); `.stfolder` en un ancestro → sincronizada | R7, M4, M11, DX2 | 37 |
| 39 | Filtro de eventos y de nombres ignorados: tipos de 3.4, matchers de `.gitignore` por evento (anidados, `info/exclude`, global, `.gestorignore`) con guarda de raíz, `Remove` con `is_dir` desconocido, ignorados de M4, carpeta ignorada creada no cuenta **R8:** Carpetas siempre ignoradas ampliadas (3.10.4, AF-2): `.vite`, `logs`, `.angular`, `.expo`, `.dart_tool` por nombre; `storage/framework` y `bootstrap/cache` por ruta relativa a la raíz. | Lecturas no cuentan; `.swp` no; `Close(Write)` sí; `*.log` ignorado no; `mkdir target` en la raíz no; `.~lock.x.odt#` no; `foo_BACKUP.md` no; ruta fuera de raíz no entra en pánico. **R8:** unitario del filtro: `logs/app.log`, `.vite/deps/x.js` y `storage/framework/views/x.php` no cuentan; `src/framework/x.ts` sí (la regla de `storage/framework` es por ruta relativa a la raíz) | R2, M3, M4 | 33 |
| 40 | **R8-4.** `checkpoint: lector de /proc para terminales marcadas, anclas y primer plano`. `trackerd::procesos::escanear(raiz_proc, raiz_dev, uid, propio_pid, tokens_vivos, reclamos)` (firma pura, 3.10.3; devuelve también los reclamos muertos): `stat` tras el último `)`, criterios 1–6 (reclamo vigente = `hasta IS NULL` **y** shell viva), `st_rdev → /dev/pts/N`, `PtyMuestra { ruta, atime, primer_plano }` con `PrimerPlano::{Local, Remoto, Multiplexor, Desconocido}`, `cwd` con respaldo al líder de sesión, panel activo de tmux, anclas y vetos, anclas de multiplexor, tokens desconocidos ignorados; `leer_identidad(pid)` | `/proc` falso: `Z`, `tty_nr = 0`, otro token, token inválido o desconocido, descendiente de `tmux: server` o de `screen` con `tty_nr = 0`, `starttime` distinto, pty reclamado por otro token, veto vigente, UID distinto → no cuenta; veto con la shell muerta sobre un pty reutilizado por una konsole nueva marcada → **sí** cuenta, y el veto sale en la lista de muertos; `comm` con espacios y `)`; `tpgid` con líder muerto; `cwd` dentro, fuera y symlink que sale; `ssh`/`mosh-client` → `Remoto`; cwd ilegible → líder; ambos → `Desconocido`. Reales en Linux con `/usr/bin/script` (fallan si falta, no se saltean) | R8, DT8, DT11, 3a | 33 |
| 41 | **R8-P1.** `checkpoint: presencia del escritorio por D-Bus con hilo propio`. `trackerd::presencia`: trait `FuentePresencia`, KDE (ms) con `org.kde.screensaver`, GNOME Mutter, `LockedHint` por `user/self.Display`, chequeo de unidades, `PresenciaArchivo` para E2E (`GESTOR_TRACKERD_TEST_PRESENCIA`, solo con `TEST_TIMINGS`), hilo con canal y reintento cada 30 s. `zbus` directo (misma versión que el lock) | Bus privado (`dbus-daemon --session` en un `tempdir`) con objetos falsos KDE y GNOME; ms contra segundos; bus caído → `NoDisponible`; lectura vieja → desconocida; bus que no responde no frena al principal. Se lanza `/usr/bin/dbus-daemon` por ruta absoluta (en esta máquina el del `PATH` es el de linuxbrew); si falta en Linux, el test **falla** | 1a, DT10, U2 | 33 |
| 42 | **R8-5.** `checkpoint: motor puro de sesiones de terminal con pausa, presencia y cwd`. Estados de 3.10.2, gracia monotónica, pausa, dos `atime` para reanudar, compuerta de presencia, regla de cwd, `clasificar_salto` con `Reloj { pared, mono, boot }`, todo cierre en UA, `esperando_demonio`, `terminal_no_detectada` que pausa (y suma el tipeo de un ancla de multiplexor), papelera, restore, kill switch, `tracker_detenido` (→ `viva_pausada`), contención para el paraguas, frontera inicial (`creada_at` o `adoptada_at`), `clic` desde `ultimo_clic_at`, UA anotada durante la gracia, ventana de presencia constante de 10 min, `VolcarTramo { evidencia }` | Una fila por transición de 3.10.2 con reloj y muestras falsos; gracia 29 contra 31 s; pausa 9:59 contra 10:00; presencia 9:59 contra 10:01 y bloqueada, también con `pausa = 60 min` (la ventana sigue en 10); archivo sin presencia → evidencia con bit 32 y el segundo del cambio (U1); `NoDisponible` → cuenta con bit 8 (U2); primer tramo de un token nacido con el demonio vivo empieza en `creada_at`; clic sobre un token pausado → tramo en el clic; clic sobre un `esperando_demonio` → no cuenta; guardado con presencia durante la gracia → queda en el tramo; tipeo de un ancla de multiplexor en `terminal_no_detectada` → tramo; SIGTERM en `viva_activa` → `viva_pausada`; un `atime` no reanuda y dos en 60 s sí; tipeo fuera de la carpeta → pausa; la tabla de relojes de 3.10.5; `terminal_no_detectada` + 10 min → pausa y sigue viva; dos tokens independientes; paraguas con un guardado en el hijo (lista de contenedores con los dos) → la sesión del paraguas sigue `viva_activa` (U3); invariante en cada acción | D-R8-3, 2a, DT1, DT3, DT4, DT6, DT7, DT16 | 35, 36, 40, 41 |
| 43 | Escritor: acciones → base con `abrir_sesion`/`latir`/`cerrar_sesion`; volcado al abrir y como máximo cada 5 s con actividad; recuperación de huérfanas `file_watch`; `MAX(ended_at)` por proyecto para la `frontera`; `false` → soltar; `BUSY` → volcado pendiente; `user_version` atrasado espera, futuro ocioso **R8:** escribe el `motivo_cierre` de cada cierre con `cerrar_sesion(…, motivo)` del 15, y `tracker_caido` en la huérfana del arranque | Contra base temporal: purga a mitad de sesión (`UPDATE` de 0 filas) no se cae; 100 eventos en 5 s → ≤ 2 escrituras; sin eventos → 0; `user_version` futuro → ninguna escritura y `estado = 'esquema_nuevo'`; `user_version = ESQUEMA_ACTUAL − 1` → ninguna escritura y `estado = 'esperando_migracion'`, y al migrar desde otra conexión → `activo` sin reiniciar; `journal_mode = delete` → espera igual, y al pasar a `wal` → `activo`; huérfana cierra con `duration = ended_at − started_at`. **R8:** ningún cierre sin motivo; la huérfana, con `tracker_caido` | P4, B1, M7, M9, M12 | 10, 13, 14, 15, 36 |
| 44 | **R8-6.** `checkpoint: escritor y arranque de las sesiones de terminal`. Aplica las acciones con los métodos del commit 15 (R8-1); evidencia por UPSERT en la transacción del volcado (≤ 1 cada 5 s); `last_seen_at` y la evidencia de los tokens pausados cada 60 s en pausa y al recibir SIGTERM; cierre condicional por `terminal_cerrada` (3.10.2); cierre de reclamos muertos; `false`/`None` → soltar; `BUSY` → pendiente; solo `dueno = demonio`; arranque limpio/sucio y pasos 1–6 de 3.10.10 (`esperando_demonio` siempre por la tabla de adopción) | Base temporal: arranque sucio con la terminal viva (→ `viva_pausada`) y muerta (→ `tracker_caido`); limpio con la terminal muerta → `terminal_cerrada` en `last_seen_at`; `esperando_demonio` con ancla muerta → `tracker_caido`; `esperando_demonio` con `last_seen_at` y salida limpia → `tracker_caido` (no `terminal_cerrada`); un clic desde otra conexión entre el escaneo vacío y el cierre → no cierra, y el token sigue vivo; veto de una shell muerta → `hasta` con `shell_cerrada`; evidencia ignorada de un token pausado → escrita a los 60 s; sin anclas y nunca vista → gracia y `terminal_no_detectada`; Parar desde otra conexión entre dos volcados → no se reabre nada; sesión `dueno = app` intacta; 1 h simulada → ≤ 720 transacciones con la evidencia adentro | DT1, DT7, DT12 | 15, 42, 43 |

### F5a — Vigilante y config

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 45 | Vigilante `notify` + `ignore` con registro de vigilancias con **conteo de referencias**; directorios nuevos (con recorrido), renombrados, movidos, borrados; recorridos en hilo aparte; altas en lotes | Integración con `tempdir`: `mkdir -p a/b/c && touch a/b/c/x` cuenta; renombrar y escribir adentro cuenta; dos proyectos con la misma carpeta: quitar uno deja al otro midiendo; paraguas + hijo: quitar el paraguas deja al hijo | M2 | 39 |
| 46 | Desborde con rescate diferido; tope `max_dirs_per_project` → `demasiadas_carpetas`; `en_red`; límite de inotify; carpeta que desaparece; recarga de matchers al cambiar `.gitignore` | Vigilante falso: desborde + carpeta creada durante el desborde → vigilada tras la calma; tope+1 carpetas → no medible y sin vigilancias propias; `logs/` agregado al `.gitignore` en caliente deja de contar | M3, M10, M11, DX2, O3 | 38, 45 |
| 47 | Lector de config de solo lectura (struct del commit 35), `stat` cada 5 s y reparse por `mtime`/tamaño; `GESTOR_TRACKERD_TEST_TIMINGS` con `config=` y `rescan=` | Config ilegible → defaults y el archivo **no** se toca (ni `mtime`); `idle_minutes` en caliente → valor efectivo nuevo; `min_credit_seconds = 999` → 300 | P12, M14, DX1 | 35 |

### F5b — Bucle, servicio y E2E

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 48 | Bucle principal: relectura de proyectos por `PRAGMA data_version` (≤ 5 s) y cada 60 s, `tick` cada 15 s, latido de proceso, checkpoint por hora, logs agregados cada 10 min con límite de tasa **R8:** El bucle: arranca el hilo **presencia**; **compuerta de presencia para `file_watch`** (U1): un cambio filtrado llega al motor de `file_watch` solo con presencia o sin dato (U2, bit 8); sin presencia no abre ni extiende tramo y se anota en `archivos_evidencia` (bit 32, con los segundos), que el escritor (43) vuelca con el método del commit 15 (R8-1) en el volcado o con el latido de 60 s; escanea `/proc` cada 5 s **solo con sesiones vivas**, con los tokens vivos y los reclamos cargados de la base (firma pura de 3.10.3) y cierra los reclamos muertos; manda cada `cambio` filtrado al motor de terminales con **todos** los proyectos que contienen la ruta y la última lectura de presencia; `reanudado`/`reloj` con `clasificar_salto` a los dos motores; relee `trabajar_sesiones` por `data_version` (incluido `ultimo_clic_at` → `clic`). Las claves de test `poll=`, `grace=` y `pause=` entran acá. | Proyecto nuevo o a la papelera sin reiniciar; papelera vista en ≤ 5 s; unitario del limitador de logs: 50 `warn` del mismo tipo en un minuto → 10 líneas y el resto contado en la línea agregada. **R8:** una sesión creada desde otra conexión se adopta en ≤ 1 sondeo; un Trabajar sobre un token pausado desde otra conexión abre tramo en el clic; con presencia falsa ausente, un guardado no crea tramo `file_watch` y deja `archivos_evidencia` con bit 32 y sus segundos; con `NoDisponible`, crea el tramo y deja bit 8 | M12, O2 | 15, 41, 43, 44, 46, 47 |
| 49 | `sd-notify`, SIGTERM/SIGINT por `signal-hook`, orden de arranque de 3.4 (config antes de la base, `READY` antes del recorrido, `scan_completed_at`, `EXTEND_TIMEOUT_USEC`, watchdog solo desde el bucle), kill switch sale 0 con `pid = NULL` y siempre **después** de `READY=1` (A1) **R8:** Antes de pisar `tracker_status`, lee el `pid` anterior (limpio o sucio). Paso 6 extendido (3.10.10). Al recibir SIGTERM **relee la config**: `false` → `tracker_apagado` (termina tokens); `true` → tramos con `tracker_detenido`, tokens vivos en `viva_pausada`, `last_seen_at` y evidencia pendiente guardados. Publica `trabajar_idle_minutes` y `presencia*` | SIGTERM deja todas cerradas y `pid = NULL`; 60.000 carpetas falsas → `READY` en < 1 s; `daemon_enabled = false` en caliente → salida 0; con un `NOTIFY_SOCKET` falso: `false` desde el arranque → `READY=1`, `STOPPING=1` y salida 0, en ese orden; `false` mientras espera una migración → lo mismo; unitario del watchdog: con un `sd_notify` falso, `WATCHDOG=1` sale una vez por vuelta del bucle y nunca desde otro hilo (M13) | B1, B5, M13, A1 | 48 |
| 50 | Harness E2E (`src-tauri/tests/e2e_trackerd.rs`, aislamiento de 5.2; **todos** sus tests llevan `#[ignore]`) + E1, E1b, E4, E8, E9, E14; `scripts/check.sh --e2e` con el control de cantidad de 5.5 (`E2E_ESPERADOS=6`) **R8:** El harness define **siempre** `GESTOR_TRACKERD_TEST_PRESENCIA` (por defecto `inactivo_ms=0 bloqueada=0`), también para E1–E22 de la v3.1: desde el 48, `file_watch` depende de la presencia (U1), y sin la variable el demonio de prueba leería el escritorio del desarrollador. Los escenarios de la v3.1 no cambian de resultado | Los escenarios, tres veces seguidas; con un test renombrado para que el filtro no lo encuentre, `check.sh --e2e` falla por cantidad | B6d, T3, T5, A2 | 41, 49 |
| 51 | E2E restantes del demonio: E2, E3, E5, E6, E7, E10–E13, E15, E17–E21; `E2E_ESPERADOS` pasa a 21 **R8:** E5 nace con `logs/app.log` y `.vite/deps/x.js` (carpetas ampliadas del 39) | Los escenarios, tres veces seguidas | T1, T4, T5, O2, A2 | 33, 39, 50 |
| 52 | **R8-7.** `checkpoint: E2E de las sesiones de terminal (E23–E31, E35)`. E2E de §5.2 del lado del demonio; E15(b), E17 y E20 extendidos; `E2E_ESPERADOS` 21 → **31** | Los escenarios, tres veces seguidas | R8, U1, U2 | 44, 48, 49, 50, 51 |

Desde el cierre de F5b, cada fase corre `scripts/check.sh --e2e` antes del merge.

### F6 — Instalación, kill switch y CLI

El demonio entra en producción en esta fase, y la fase entera se mergea junta:
`main` nunca tiene la instalación sin el toggle (B5). Además, desde F5b el
demonio ya sale solo con `daemon_enabled = false` en la config.

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 53 | `start-app.sh` deja de matar al demonio: hoy hace `pgrep -f "gestor-proyectos"` (`start-app.sh:19`) y `pkill -f "gestor-proyectos"` (`:21`), que **sí** alcanzan al demonio, porque su `ExecStart` lleva `--data-dir …/share/gestor-proyectos` (3.7). Los dos pasan al patrón acotado de abajo (solo procesos cuyo `argv[0]` es la app). No sirve `-x`: el nombre del proceso se trunca a 15 caracteres y `gestor-proyectos` tiene 16 | Paso manual 16 (5.3): con el demonio corriendo, `./start-app.sh` → `systemctl --user is-active gestor-trackerd` sigue en `active` | P10 | — |
| 54 | Generador del unit de 3.7 (rutas resueltas, `--data-dir`/`--config-dir`, aislamiento, recursos, backoff con alternativa para systemd < 254) + trait `ServiceManager` con `UNIT_DIR`/`BIN_DIR` por parámetro + ciclo de 3.8 en `.setup()`: condiciones (Linux, producción, sin `GESTOR_NO_TRACKERD`), **kill switch** (`daemon_enabled = false` → `disable --now`), ubicación de `current_exe`, comparación por contenido completo, `FragmentPath`, `is-enabled` sin reactivar un `disable` manual, `try-restart` una vez por apertura **R8:** Golden: el unit **no** tiene `ProtectProc=`, `ProcSubset=`, `PrivatePIDs=` ni `PrivateDevices=`. Comentario en el generador y línea en §6. **Además**, `tracker_soportado()` (3.10.7) pasa a usar las mismas condiciones con las que este commit decide instalar el demonio (Linux, producción, sin `GESTOR_NO_TRACKERD`, `current_exe()` en `BIN_DIR`) y deja de exigir el binario: desde acá `adopcion_imposible()` deja de ser verdadero en producción (3.10.7). | Texto del unit (golden); unit idéntico → sin `daemon-reload`; un byte distinto → reescribe + `try-restart`; exe en `target/` o terminado en `(deleted)` → nada; ruta con espacio → error; `disabled` con unit previo → sin `enable`; sin unit previo → `enable --now`; `daemon_enabled = false` → `disable --now`; plataforma ≠ Linux → ninguna llamada; systemd 250 → `StartLimitBurst`; systemctl falso con `Result=exit-code` + `ExecMainStatus=226` → unit regenerado sin aislamiento y aviso; `Result=exit-code` con otro estado → unit intacto (O1). **R8:** exe en `target/` → adopción imposible → cerrar la ventana termina la sesión de la app con `cierre_app`; app de producción en `BIN_DIR` sin `gestor-trackerd` → la sesión queda `esperando_demonio` (no instalado, U4) | R5, R6, B5, B9, O1, O5, M15 | 13, 17, 34, 35 |
| 55 | Toggle mínimo "Tracker activado" en Ajustes + indicador mínimo (activo / apagado) en la cabecera; secuencia **config primero, después `systemctl`** **R8:** Ajustes → Apagar: después del `disable --now`, `cerrar_por_kill_switch()`; también en el paso 2 de la apertura (§3.8). Toast de 3.10.12 | Vitest del toggle (orden config → comando, con mock); unitario del trait: `false` → `disable --now`, `true` → `reset-failed` + `enable --now` | B5 | 54 |
| 56 | E16 y E22: `systemd-run --user` con las propiedades del generador (5.2) **R8:** E16 lee, bajo el aislamiento real, `stat`, `environ`, `cwd` y el `atime` de `/dev/pts/N` de un `script` lanzado **fuera** del unit, y una presencia por D-Bus contra un bus de prueba. **Bloqueante para el merge de F6**, con la salida pegada en el PR | E16 y E22, locales, `#[ignore]` | T2, O1, A1 | 40, 41, 49, 54 |
| 57 | Restore con systemd: pasos 3 y 8 de 3.6 (`stop` recordando si estaba activo; `start` antes de `app.exit(0)` si estaba activo y `daemon_enabled` sigue en `true`) | Con systemctl falso: orden exacto (`stop` → lock → migrar → soltar → `start`); lock tomado → aborta sin tocar la base; unit `disabled` → sin `start` | B7 | 27, 54 |
| 58 | **R8-8.** `checkpoint: Trabajar elige el demonio cuando está vivo y la app no adopta en silencio`. `trabajar` elige `dueno = demonio` si `demonio_vivo()`; traspaso `demonio → app` solo con Trabajar explícito; reconciliación sin demonio que cierra solo lo que tiene prueba de cierre (`tracker_caido`); indicador "tracker caído" mínimo | Demonio vivo falso → `dueno = demonio`; latido vencido → `app`; cerrar la ventana con una sesión del demonio (también `terminal_no_detectada`) → intacta; sin demonio y terminal viva (`script` real) → no se toca; sin demonio, ancla muerta → `tracker_caido`; Trabajar sobre una sesión del demonio sin demonio → tramo abierto cerrado con `tracker_caido` y `dueno = app` | DT2, DT14 | 17, 19, 40, 55 |
| 59 | `scripts/release.sh`, `install.sh` y `uninstall.sh`: `PREFIX`/`BIN_DIR`/`UNIT_DIR`/`SYSTEMCTL`; `.desktop` con `$BIN_DIR` (hoy fijo en `install.sh:46`, y el heredoc pasa de `<< 'EOF'` a `<< EOF`, `install.sh:40`, para que la variable se expanda); temporal + `mv` de **los dos** binarios (hoy `cp` en `scripts/release.sh:304` e `install.sh:27`); `try-restart` si el unit existe; `install.sh` compila `gestor-trackerd` con `cargo build --release --bin gestor-trackerd` si falta (hoy solo exige `gestor-proyectos`, `install.sh:14-17`); `uninstall.sh` con `disable --now`, borrado del unit, `daemon-reload`, `reset-failed`, sin tocar la base | `PREFIX=$tmp SYSTEMCTL=true ./install.sh` dos veces seguidas, y el `.desktop` generado tiene `Exec=$tmp/bin/gestor-proyectos` (no la ruta literal); `scripts/release.sh patch --dry-run`; `uninstall.sh` con `PREFIX=$tmp` deja la base | O4, O5, T6 | 54 |
| 60 | **R8-C1.** `checkpoint: versiona la CLI gestor sin cambiar su comportamiento`. `scripts/cli/{gestor, gestor.sh, gestor-completion.bash, gestor-completion.zsh, instalar-cli.sh}` copiados byte a byte; `instalar-cli.sh` toca los rc solo con `--rc` (copia previa, bloque de una línea con `source` protegido por `[ -f … ]`); `install.sh` lo llama sin `--rc`; `uninstall.sh`; `docs/cli.md` | `HOME=$tmp PREFIX=$tmp SYSTEMCTL=true scripts/cli/instalar-cli.sh` dos veces → script idéntico y rc sin tocar; con `--rc` dos veces → un solo bloque por archivo, copia previa, resto del rc igual (diff); `uninstall.sh --rc` → sin bloques; `uninstall.sh` sin `--rc` → aviso en stderr, y `ZDOTDIR=$tmp zsh -i -c true` y `bash --rcfile $tmp/.bashrc -i -c true` con el bloque protegido no escriben nada en stderr; `shellcheck` en `check.sh`; `diff` contra lo instalado antes del merge | R8-CLI | 59 |
| 61 | **R8-C2.** `checkpoint: subcomando gestor-trackerd sesion para la CLI`. `iniciar \| parar \| soltar \| estado`, base por `--data-dir` > `GESTOR_DB` > XDG, config por `--config-dir`, guarda B8, sin migrar, `restore.lock` compartido, `demonio_vivo()`, `esperando_demonio` sin demonio, tracker apagado → código 3, validación de `--shell-pid` (ancestro, UID, no `Z`, tty), anclas de multiplexor con aviso, vetos, cierre de reclamos muertos antes de escribir, `ultimo_clic_at` al unirse, `soltar` que escanea y termina la sesión si era la última terminal (`cambio_de_proyecto` o `parar_cli`), aviso de demonio caído por stderr, `parar` sobre `dueno = app` con la regla de huérfanas, códigos 0/1/2/3 | Base por cada vía (y `GESTOR_DB` que no termina en `projects.db` → error); `restore.lock` exclusivo → 3 y nada escrito; `restore.lock` libre mientras el diálogo del restore está abierto (sin confirmar) → `iniciar` no devuelve 3; apagado → 3; sin demonio → `esperando_demonio` con ancla; ancestro `tmux: server` → ancla de multiplexor + aviso; pid que no es ancestro → 2; sin tty → 2; esquema atrasado → 3 y nada escrito; iniciar X dos veces → no-op; iniciar Y → X `cambio_de_proyecto` si era la última; `soltar` → veto, y si era la última terminal → `cambio_de_proyecto` (destino distinto) o `parar_cli` (mismo proyecto), nunca `terminal_cerrada`; veto viejo de una shell muerta en el mismo `tty_nr` → no bloquea el ancla nueva; demonio caído → aviso en stderr y código 0; `parar` de `dueno = app` → regla de huérfanas; `estado --json` estable | R8-CLI, DT8, DT9, U5 | 15, 34, 40, 49, 60 |
| 62 | **R8-C3.** `checkpoint: la CLI mide al elegir un proyecto`. Menú y `abrir` llaman a `sesion iniciar` con `GESTOR_SHELL_PID=$$` y `--data-dir "$(dirname "$DB")"`; `GESTOR_SESION_FILE` + `export`/`unset`; `gestor parar [X]`, `gestor estado`, `--sin-medir`; avisos en español (función vieja, sin binario, sin demonio, apagado, tmux); completado; `docs/cli.md`; E32–E34, E36, E37; `E2E_ESPERADOS` 31 → **36** | E2E de §5.2; `gestor list`, `crear` y `ayuda` → ninguna sesión | R8-CLI | 52, 60, 61 |
| 63 | **R8-C4.** `checkpoint: ajustes de tiempo desde la CLI`. Subcomando `gestor-trackerd ajuste agregar \| revertir \| listar` (no exige demonio, rechaza durante un restore) y `gestor ajustar`, `gestor ajustar --revertir`, `gestor ajustes`; completado; `docs/cli.md` | Integración del subcomando (nota vacía, `segundos = 0`, reversión doble → errores en español, códigos); E34 extendido | R8-CLI, 5a | 22, 61, 62 |

Patrón del commit 53, para `pgrep -f` y `pkill -f` (fuera de la tabla para no
escapar la barra vertical):

```bash
pkill -f '^([^ ]*/)?gestor-proyectos( |$)'
```

**Validación de F6:** pasos manuales 3–6 y 8–18 de 5.3 (instalación
real con `install.sh`, sin `release.sh`). E16 extendido (`cwd`, presencia,
`/dev/pts`) en verde, con la salida pegada en el PR: es **bloqueante** para el merge
(3.10.3). Salida de `scripts/check.sh --windows` pegada en el PR del commit 16 o
corrida en la CI (3.10.3).

### F7 — Interfaz completa

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 64 | `get_live_tracking`: sesiones abiertas de todas las fuentes + `tracker_status` + `elapsed_seconds` de la `ActiveSession` de esta app **R8:** `get_live_tracking` agrega las sesiones vivas (`token`, `estado`, `dueno`, `origen`, `creada_at`, `last_seen_at`, tramo abierto, cantidad de terminales), `elapsed_seconds` de la `MedicionApp`, `demonio_vivo` y `tracker_status.presencia` | Unitario | M5 | 13, 15, 17, 21 |
| 65 | Contador en vivo (reglas de 3.9) y refresco del badge al cerrarse una sesión **R8:** Contador: `dueno = demonio` usa la suma de `duration` de los tramos del token, sin interpolar; `dueno = app`, el `Instant`. | Vitest con timers falsos: `file_watch` nunca pasa de `last_activity_at`; suspensión simulada no hace bajar el contador de "Trabajar" más de 5 s; el badge se refresca al cerrar. **R8:** Vitest: el contador de una sesión del demonio no baja entre sondeos ni salta al cerrar | P5, M5, M9 | 64 |
| 66 | Indicador global de 6 estados (3.9), reemplaza al mínimo del commit 55. **R8:** integra el aviso de cabecera del commit 17 (`aviso_cabecera`, U6, 3.10.14) y le suma "o el servicio está instalado" | Vitest por estado. **R8:** la alarma aparece en No instalado (con el unit instalado alguna vez) y en Con problemas por caída, tras dos sondeos; nunca en Apagado ni en No disponible; en Apagado con una sesión de la app, solo la línea "Trabajar mide sin vigilancia" | B5, U6 | 17, 55, 64 |
| 67 | Ajustes → Tracking completo: inactividad, margen por guardado (DX1), tope de carpetas (DX2), motivos de no medibles, uso de inotify; Ajustes → Backups con la nota de D12 (3.9) **R8:** Campo "Pausa de Trabajar por inactividad" y línea "Presencia del escritorio" en Ajustes → Tracking | Vitest + config vieja sin la sección; inactividad cambiada → muestra el valor efectivo cuando el demonio lo aplica; Vitest: la nota de backups del home aparece en Ajustes → Backups | DX1, DX2, O3, M14, D12 | 55 |
| 68 | Avisos por proyecto de 3.9, calculados con las funciones de la lib del commit 38 | Vitest por motivo; con el demonio apagado, `carpeta_amplia` igual aparece | R7, DX2, M4 | 38, 64 |
| 69 | **R8-9.** `checkpoint: indicador por estado, toasts y desglose del tiempo`. Comando `desglose_tiempo` (lib `db/desglose.rs`); UI: indicador por estado con "tracker caído", toasts por transición (también `tope_sin_tracker`), desglose con chips (`sin_vigilancia` derivado del `source`, U6), pausas, tramos derivados `solo_archivos` desde `trabajar_evidencia` y `archivos_evidencia` (U1), lo ignorado, detalle por minuto, totales con ajustes y simultaneidad; presencia en Ajustes | Rust (`db/desglose.rs`): tres cambios sin presencia a 1 min y uno a 5 min → dos tramos `solo_archivos` con desde/hasta al segundo; un tramo `solo_archivos` superpuesto con uno cobrado se recorta; el mismo minuto en las dos tablas no se duplica; `solo_archivos` no cambia `tiempo_total`. Vitest: 10 textos del indicador ("tracker caído" gana a `viva_activa` y pierde con `esperando_demonio`); `terminal_cerrada` → toast; `viva_pausada` → sin toast; contador que no baja ni salta; desglose de un día con pausa, suspensión, `ssh` y simultáneo → total igual a `tiempo_total`; bloque "Solo archivos, sin presencia" en un proyecto sin Trabajar; chip `sin_vigilancia`; reabrir la app con una sesión del demonio → Parar visible | 4a, U1, U6 | 58, 64, 65, 67 |
| 70 | **R8-A2.** `checkpoint: UI de ajustes manuales en el desglose`. Formulario (día/rato, signo, cantidad, nota), aviso de superposición con lo medido, lista con "Revertir", "Cargar como ajuste" desde una pausa y desde un tramo `solo_archivos` (U1: precarga desde/hasta/segundos, parte en dos si cruza la medianoche, pide la nota) | Vitest: validaciones; el aviso propone el resto; "Revertir" pide nota; tachado del revertido; "Cargar como ajuste" prellena el horario desde una pausa y desde un tramo `solo_archivos` (segundos exactos, no guarda sin nota, dos ajustes si cruza la medianoche) | 5a, U1 | 22, 69 |

**Validación de F7:** paso manual 7 de 5.3 y Vitest.

### F8 — Validación y documentación

| # | Commit | Test | Hallazgos | Depende de |
|---|---|---|---|---|
| 71 | `scripts/medir-trackerd.sh`: RSS, `%CPU`, `memory.current` del cgroup y `watched_dirs` (6) **R8:** `medir-trackerd.sh` mide la CPU también con **3 sesiones vivas** y el hilo de presencia (2 llamadas D-Bus cada 5 s). Umbral < 1 % | Correrlo con el demonio instalado; salida con los cuatro valores | T8, O3 | 59 |
| 72 | `scripts/datos-historicos.sh`: aplica lo decidido en DX3 y DX4 sobre la base real. Por defecto solo muestra lo que haría (las filas y proyectos afectados, con la lista del commit 24); con `--aplicar` exige un "sí" escrito del usuario, hace antes un backup `VACUUM INTO` verificado con `integrity_check`, y aplica en una transacción `IMMEDIATE`. Con DX3 (a), "marcar" no toca la base: solo genera la lista para el CHANGELOG | Contra una copia: sin `--aplicar` la base queda igual byte a byte; con `--aplicar` y una respuesta distinta de "sí" → nada cambia; con "sí" → backup presente y verificado antes del primer cambio | D3, DX3, DX4 | 24 |
| 73 | `ARQUITECTURA.md`, `CLAUDE.md` (sección Time tracking: los dos caminos, el demonio, el piso de downgrade), CHANGELOG con cada diferencia del diff con datos reales (D3), la lista de sesiones > 12 h marcadas (DX3) y el "piso de downgrade". Después, `scripts/release.sh minor` **R8:** `ARQUITECTURA.md`, `CLAUDE.md` (Time tracking: sesión atada a la terminal, motivos, ajustes, CLI; conteo de comandos: hoy `CLAUDE.md` dice 79 y `generate_handler!` registra 80 (medido en `83c48d4`); salen `start_work_session`, `stop_work_session` y `get_work_session_status`, entran `trabajar`, `parar`, `estado_trabajar`, `desglose_tiempo` y los de ajustes, y el commit escribe el número que dé `generate_handler!` en ese momento), `docs/cli.md` (medición, `parar`, `estado`, `--sin-medir`, `ajustar`, `ajustes`) | — | D3, D10 | todos |

**Validación de F8:** matriz por terminal (5.3.1) y recorrido de R8 (pasos 19–42 de
5.3.2) sobre la instalación real, además de la Definición de "listo" (§6).

### 4.10 Verificación del orden

| Algo que se usa | Lo crea | Lo usan |
|---|---|---|
| `scripts/check.sh` | 2 | Todos desde el 3 |
| `.setup()` | 4 | 19, 54 |
| Regla mínima | 8 | 19, 20 |
| `migrar`, `abrir_existente` | 10 | 12, 13, 27, 43 |
| `db::restore` (`tomar_lock`, `restaurar_desde`) | 27 | 33 (entra en la lib), 51 (E15), 57 |
| `last_activity_at` | 12 | 14, 19, 43 |
| `tracker_status` | 13 | 43, 54, 64 |
| `abrir_sesion`/`latir`/`cerrar_sesion` | 14 | 19, 43 |
| Recuperación de huérfanas de la app (con token y tope) | 19 | 24 (test sobre la copia real) |
| Feature `backup` de rusqlite | 24 | 27 |
| `paths.rs` | 26 | 27, 29, 33, 34 |
| WAL | 30 | 43 en adelante (el demonio espera si no está) |
| Lib | 33 | 34, 37, 39, 40, 41 y todo F4–F6 |
| Sección `tracking` | 35 | 36, 47, 54, 55, 67 |
| Funciones de contención | 38 | 46, 68 |
| Demonio completo | 49 | 50, 51, 56 |
| Control de cantidad de `check.sh --e2e` | 50 | 51, 52, 62 |
| Trait `ServiceManager` | 54 | 55, 56, 57, 59 |
| Lista del diff con datos reales | 24 | 72, 73 |
| Paso 4, métodos condicionales, `demonio_vivo`, `restore.lock`, `terminar_sesiones_de` | 15 (R8-1) | 17, 18, 19, 20, 27, 40, 44, 49, 55, 58, 61 |
| `archivos_evidencia`, segundos sin presencia, `registrar_evidencia_archivos` (U1) | 15 (R8-1) | 44, 48, 69 |
| `TOPE_SIN_TRACKER`, `aviso_sin_tracker` (U6) | 17 (R8-3) | 19, 69 |
| Presencia del harness por defecto (U1) | 50 (modificado) | 51, 52, 62 |
| `open_terminal` con marca, reaper, `limpiar_entorno_heredado` | 16 (R8-2) | 17 |
| `trabajar`, `parar`, `estado_trabajar`, `ActiveSession` como `Option` | 17 (R8-3) | 18, 19, 21, 27, 58 |
| Papelera que corta sesiones | 18 (R8-3b) | 52 (E2E), 69 (toast) |
| `tiempo_trabajado`, `simultaneidad` | 20 | 22, 69 |
| Paso 5, `ajustes_tiempo`, `tiempo_total` | 22 (R8-A1) | 63, 69, 70 |
| `trabajar_idle_minutes` | 35 | 42, 49, 67 |
| `adopcion_imposible()`: base `!tracker_soportado()` | 17 (R8-3) | 18, 19, 58 |
| `adopcion_imposible()`: cláusula `!daemon_enabled` | 35 (modificado) | 55, 58 |
| `tracker_soportado()` con las condiciones de instalación | 54 (modificado) | 58 |
| Carpetas siempre ignoradas ampliadas | 39 (modificado) | 48, 51 (E5) |
| `trackerd::procesos` | 40 (R8-4) | 42, 56, 58, 61 |
| `trackerd::presencia` | 41 (R8-P1) | 42, 48, 56 |
| Motor de terminales | 42 (R8-5) | 44 |
| Escritor y arranque de terminales | 44 (R8-6) | 48, 49, 52 |
| Bucle con terminales y presencia | 48 (modificado) | 52 |
| SIGTERM que relee la config | 49 (modificado) | 52, 55 |
| `cerrar_por_kill_switch` | 55 (modificado) | 58 |
| Subcomando `sesion` | 61 (R8-C2) | 62 |
| CLI versionada | 60 (R8-C1) | 62, 63 |
| `get_live_tracking` con sesiones | 64 (modificado) | 65, 69 |
| Desglose | 69 (R8-9) | 70 |
| Aviso de cabecera sin tracker (`aviso_cabecera`, U6) | 17 (R8-3) | 66 |

Dependencias de R8 revisadas contra la numeración de la sección 4:

- 15 (R8-1) ← 10, 13, 14. 16 (R8-2) ← nada. 17 (R8-3) ← 7, 8, 9, 15, 16. 18 (R8-3b) ← 15, 17.
- 22 (R8-A1) ← 15, 20, 21.
- 40 (R8-4) ← 33. 41 (R8-P1) ← 33. 42 (R8-5) ← 35, 36, 40, 41. 44 (R8-6) ← 15, 42, 43.
- 52 (R8-7) ← 44, 48–51.
- 58 (R8-8) ← 17, 19, 40, 55 (ya no depende de `ServiceManager` para excluir filas).
- 60 (R8-C1) ← 59. 61 (R8-C2) ← 15, 34, 40, 49, 60. 62 (R8-C3) ← 52, 60, 61. 63 (R8-C4) ← 22, 61, 62.
- 69 (R8-9) ← 58, 64, 65, 67. 70 (R8-A2) ← 22, 69.
- Modificados: 19 usa 15 y 17 (también `TOPE_SIN_TRACKER`); 20 usa 15; 24 usa 19 (huérfanas sobre la copia); 36 y 43 usan 15 (`motivo_cierre`); 27 usa 15 y 17; 35 usa 17 (`adopcion_imposible`); 48 usa 15 (`archivos_evidencia`), 41 y 44; 50 usa 41 (`GESTOR_TRACKERD_TEST_PRESENCIA`); 54 usa 17; 56 usa 40 y 41; 64 usa 15 y 17.

**Ningún "Depende de" apunta a un número mayor.** En la primera redacción había uno
oculto: el 17 (R8-3) y el 19 usaban `daemon_enabled`, que recién nace en el 35. Se
corrigió partiendo `adopcion_imposible()` por commits (3.10.7): el 17 (R8-3) solo usa
`tracker_soportado()`, y la cláusula de la config entra en el 35. En la corrección final se
movieron cuatro pruebas que usaban algo posterior: el ensayo de huérfanas sobre la copia
real (19 → 24, por la feature `backup`), la variante de `restore.lock` de la CLI
(27 → 61), el paraguas (20 → 42) y las carpetas ampliadas, que en el 39 se prueban con un
unitario porque E5 nace en el 51.

### 4.11 Estimación

Días efectivos de un desarrollador, con la prueba manual incluida y sin contar las
esperas por decisiones de la sección 9 ni la hora de medición de "listo". La columna
"v3.1" es la estimación anterior; "Adicional R8" suma la primera estimación de R8 con
la estimación realista de la auditoría de pruebas (+10 a +12 en vez de +8 a +9,75: le
faltaban la integración, la prueba manual y el endurecimiento contra inestabilidad),
lo nuevo de facturación (+52 a +60 h: presencia, cwd, evidencia, ajustes, desglose) y
lo nuevo de ciclo de vida (+30 a +38 h: papelera, restore, kill switch,
`demonio_vivo`, CLI aislada, Windows), sin contar dos veces lo que se solapa (40 y 56).

| Fase | Commits | v3.1 | Adicional R8 | v3.2 | Lo que más pesa |
|---|---|---|---|---|---|
| F0 | 9 | 1–2 | 0 | 1–2 (hecha) | Commit 4 (inicialización después de la instancia única, 3.8) |
| F1 | 16 | 4–5 | 4,75–5,5 | 8,75–10,5 | Migraciones versionadas, unión con tests de tabla, diff con datos reales. R8: 15 (R8-1) con dos listas de métodos, `ultimo_clic_at`, regla única de `terminada_at` y cierre sin carrera, `archivos_evidencia`; 17 (R8-3) (traspaso, `workSession.ts`, tests reescritos, tope de 12 h y aviso); 19 con el tope en el latido; 22 (R8-A1); test de huérfanas sobre copia |
| F2 | 7 | 2,5–3 | 0,5 | 3–3,5 | Restore con la API de backup y la validación sobre la copia real. R8: Restore con `restore.lock` y `restauracion` |
| F3 | 3 | 1 | 0 | 1 | Verificación del bundle |
| F4 | 9 | 2,5–3 | 5,5–6 | 8–9 | Motor con DX1 y escritor. R8: 41 (R8-P1) (D-Bus), 42 (R8-5) (tabla de transiciones grande, ahora con clic, gracia con UA y tipeo de multiplexor), 40 (R8-4) (primer plano, reclamos vivos), 44 (R8-6) (cierre condicional, reclamos muertos, evidencia en pausa) |
| F5a | 3 | 2–2,5 | 0 | 2–2,5 | Conteo de referencias y rescate tras desborde |
| F5b | 5 | 2–3 | 2,25–2,75 | 4,25–5,75 | E2E estables (centinelas, sondeo). R8: 52 (R8-7): 10 E2E con pty y variantes de presencia; 48 con la compuerta de presencia de `file_watch` y `archivos_evidencia`; harness (50) con presencia por defecto |
| F6 | 11 | 2–3 | 3,75–4,75 | 5,75–7,75 | Unit con aislamiento, E16 y E22, scripts. R8: 61 (R8-C2) (guardas, códigos, `soltar` que escanea, vetos vivos), 62 (R8-C3) en bash y zsh con aislamiento, E36–E37 con el perfil de tipeo, 63 (R8-C4), E16 bloqueante |
| F7 | 7 | 1,5–2,5 | 2,75–3,5 | 4,25–6 | Contadores y Ajustes. R8: Desglose (69, R8-9) con tramos `solo_archivos` y chip `sin_vigilancia`; UI de ajustes (70, R8-A2) con "Cargar como ajuste" desde `solo_archivos` |
| F8 | 3 | 1–1,5 | 2–2,5 | 3–4 | Script de datos históricos, corrida triple y docs. R8: Matriz (~45 min) y recorrido de 24 pasos con esperas de 12–15 min, suspensión y logout; endurecer E2E inestables |
| v3.1: tests repartidos | — | 0,5 | 0 | 0,5 | A1, A3, B1, O1, O2 y el commit 72 |
| **Total** | **73** | **20–27** | **+21,5 a +25,5** | **41,5–52,5** | Quedan **40,5–50,5** desde F1 (F0 está hecha) |

Comparación: la primera versión del plan R8 decía +8 a +9,75 (total 28–36,75). La primera redacción
de la versión 2 del plan decía +20 a +24 (40–51); las correcciones de los verificadores suman **+1 día**
(F1 +0,25, F4 +0,5, F6 +0,25: `ultimo_clic_at`, regla única de `terminada_at`, cierre
sin carrera, reclamos vivos, tipeo de multiplexor, `soltar` que escanea, perfiles de
tiempos en los E2E). Las decisiones del 2026-10-20 suman **+1 día** más (F1 +0,25:
tope de 12 h en el 17 (R8-3) y 19; F5b +0,25: presencia en `file_watch` y `archivos_evidencia`
en el 48 y el harness; F7 +0,5: tramos `solo_archivos`, chip `sin_vigilancia` y "Cargar
como ajuste" precargado). U2, U3, U4, U5 y U7 no cambian la estimación: siguen la
recomendación sobre la que ya estaba escrito el plan R8. La diferencia total contra la primera versión
(+13,5 a +15,75) sale de la auditoría de pruebas (+2 a +2,25), de lo nuevo de A
(presencia, cwd, evidencia, ajustes y desglose: ~+6,5 a +7,5), de lo nuevo de B (~+4 a
+5), de las correcciones (+1) y de las decisiones (+1). Esas partes suman +14,5 a
+16,75: sobra ~1 día, que es el solape entre el 40 (R8-4) y el E16 extendido (56), contado
una sola vez en la tabla (~0,5), y la fila "Integrar la v3.2 al documento y renumerar"
(0,5) del plan, que salió de la tabla porque esa integración es este documento.

---
## 5. Pruebas

### 5.1 Pirámide

| Nivel | Qué cubre | Herramienta | Cuándo |
|---|---|---|---|
| Unitario | Tramos y unión, `time.rs` (zonas con `chrono-tz`), migraciones, motor, filtros, atribución, carpetas, texto del unit, trait de systemctl | `cargo test` | Cada commit |
| Integración | Escritor contra SQLite real; vigilante contra `tempdir`; dos conexiones concurrentes; restore | `cargo test` (`src-tauri/tests/`) | Cada commit |
| Punta a punta | Demonio real como proceso + base temporal: E1–E15, E17–E21 y, con R8, E23–E37 (5.2). **Todos** los tests de `tests/e2e_trackerd.rs` llevan `#[ignore]`, así que `cargo test` solo no los corre; `--e2e` verifica que corrieron los esperados (5.5) | `scripts/check.sh --e2e` | Al cerrar cada fase desde F5b |
| Systemd local | E16 (el unit real con su aislamiento, como unidad transitoria; con R8, también la lectura de `/proc`, `/dev/pts` y la presencia, 3.10.3) y E22 (kill switch al arrancar) | `cargo test --test e2e_trackerd -- --ignored e16` y `… e22` | Al cerrar F6 y F8. Nunca en CI |
| Datos reales | Diff viejo contra nuevo sobre una copia de la base real (5.4) | Test `#[ignore]` del commit 24 | Al cerrar F1, F2 y F8 |
| Frontend | Contadores, indicador, ajustes, avisos, errores | `pnpm test` (Vitest) | Cada commit |
| Manual guiado | 5.3 (pasos 1–18), 5.3.1 (matriz por terminal) y 5.3.2 (pasos 19–42) | Persona + checklist | Según la columna "Fase" de 5.3 (pasos 1–18); 5.3.1 y 5.3.2, al cerrar F8 |

En `main@c8b55ae` había **171** `#[test]` en el crate, y en `feat/tracker-f0@83c48d4` hay **174** (medido). Los tests de integración no pueden
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
nada en silencio. `active_today` en hora local se prueba en el commit 21 pasando
la zona, no el reloj del sistema.

#### Unitarios e integración de R8

| Nivel | Qué | Commit |
|---|---|---|
| Esquema y métodos | `CHECK` (incluido el de tramo con motivo), índices únicos, condicionales, cascada, carreras entre dos conexiones (clic contra `terminal_cerrada`), regla única de `terminada_at`, `demonio_vivo` con `comm` | 15 (R8-1) |
| Lanzador | Golden del argv; terminal falsa que vuelca argv y `environ`; reaper por pid; `limpiar_entorno_heredado` con binario auxiliar | 16 (R8-2) |
| Comandos | Un proyecto a la vez con la app, traspaso a `esperando_demonio`, reutilización del token, `ResultadoParar`, adopción imposible (también sin el binario del tracker, F1–F5), `restore.lock`, recorte a 12 h, `aviso_sin_tracker` y `aviso_cabecera` (U6) | 17 (R8-3) |
| Papelera | Proyecto y descendientes en una transacción | 18 (R8-3b) |
| Huérfanas y tope | Solo `work_button`; token a `esperando_demonio`; latido que corta a las 12 h con `tope_sin_tracker`; huérfana de 13 h recortada (U6) | 19 |
| Huérfanas reales | Las 81 sin token sobre una copia de la base real (`#[ignore]`, con la API de backup) | 24 |
| Presencia en `file_watch` | Sin presencia → sin tramo y `archivos_evidencia` con bit 32 y segundos; `NoDisponible` → tramo con bit 8 (U1, U2) | 48 |
| Adopción imposible con el tracker apagado | `daemon_enabled = false` → `cierre_app`/`cambio_de_proyecto` | 35 |
| Carpetas ignoradas | Unitario del filtro: `logs/`, `.vite/` y `storage/framework` (por ruta) no cuentan; E5 los incluye desde que nace | 39, 51 |
| Unión | Terminal 9–18 con suspensión 12–14 y `file_watch` 16–18 → 7 h; pausa con `file_watch` y DX1; simultaneidad | 20 |
| Ajustes | Triggers, cascada, validaciones en español, `tiempo_total`, fecha efectiva con zona inyectada | 22 (R8-A1) |
| Restore | Backup con token vivo → `restauracion`; Parar después → `Ninguna`/`YaCerrada` | 27 |
| `/proc` falso | Criterios 1–6, anclas, vetos, multiplexor, primer plano, cwd; reales con `/usr/bin/script` | 40 (R8-4) |
| Presencia | Bus D-Bus privado, unidades, caída, lectura vieja, hilo que no bloquea | 41 (R8-P1) |
| Motor | Una fila por transición; relojes `{pared, mono, boot}` (la guarda monotónica ahora **sí** se prueba); presencia; dos `atime`; paraguas sostenido por el hijo (U3) | 42 (R8-5) |
| Escritor | Arranque limpio/sucio, adopción con y sin prueba, cadencia con evidencia | 44 (R8-6) |
| Kill switch | SIGTERM que relee la config; `cerrar_por_kill_switch` idempotente | 49, 55 |
| Subcomandos | `sesion` (rutas, guardas, códigos) y `ajuste` | 61 (R8-C2), 63 (R8-C4) |
| CLI instalada | `instalar-cli.sh` con `HOME=$tmp PREFIX=$tmp`, rc solo con `--rc` | 60 (R8-C1) |
| Desglose (Rust) | Tramos `solo_archivos` derivados de las dos tablas de evidencia, recorte contra lo cobrado, sin efecto en `tiempo_total` | 69 (R8-9) |
| Vitest | Tarjeta, contador, toasts (incluido el aviso sin tracker), desglose (`solo_archivos`, `sin_vigilancia`), ajustes ("Cargar como ajuste" desde `solo_archivos`); `ProjectList.work.test.tsx` de `83c48d4` | 17, 65, 69, 70 |

### 5.2 Punta a punta automatizado

**Aislamiento (O5, B8).** Ningún test automatizado toca la base, la config, el
unit ni los binarios reales:

| Qué | Cómo se aísla |
|---|---|
| Base y lock | `XDG_DATA_HOME` temporal **y** `--data-dir` explícito. El harness, antes de lanzar, canoniza la ruta y **falla** si es `$HOME/.local/share/gestor-proyectos` o no está bajo el `tempdir` (doble guarda: el demonio ya se niega con `GESTOR_TRACKERD_TEST_TIMINGS` definida, 3.4) |
| Config | `XDG_CONFIG_HOME` temporal y `--config-dir` |
| Runtime | `XDG_RUNTIME_DIR` temporal. E14 usa **otro** distinto para el segundo demonio, a propósito |
| Proceso | El harness guarda el `Child` en un struct con `Drop` que manda SIGKILL: un test que falla no deja demonios vivos |
| Unit y systemctl | Los tests del trait (commit 54) usan `UNIT_DIR` y `BIN_DIR` en un `tempdir` y un systemctl falso que registra las llamadas |
| Scripts | `PREFIX=$tmp BIN_DIR=$tmp/bin UNIT_DIR=$tmp/systemd SYSTEMCTL=true` (commit 59) |
| E16 | Unidad **transitoria** (`systemd-run --user`): vive en `$XDG_RUNTIME_DIR/systemd/transient/`. El test afirma que `~/.config/systemd/user/` tiene los mismos archivos antes y después |

Todos los E2E corren con
`GESTOR_TRACKERD_TEST_TIMINGS=idle=3s,tick=250ms,flush=250ms,reload=1s,heartbeat=1s,config=200ms,rescan=1s,poll=250ms,grace=2s,pause=3s`
(las tres últimas claves son de R8; los escenarios con tipeo usan `pause=20s`, ver
abajo) y, desde el commit 50, con `GESTOR_TRACKERD_TEST_PRESENCIA` (por defecto
`inactivo_ms=0 bloqueada=0`): desde el 48 `file_watch` depende de la presencia (U1).
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
| E1 | `min_credit_seconds = 0`. Escribir cada 500 ms durante 4 s | 1 sesión `file_watch` de 3,5 s ± 1, cerrada en el último cambio | 50 |
| E1b | Igual, con el margen por defecto | Margen = `min(60 s, idle)` = 3 s con los tiempos de test: 6,5 s ± 1, y `started_at` 3 s antes del primer cambio (DX1) | 50 |
| E2 | Dos proyectos alternados | Dos sesiones superpuestas, una por proyecto (R1); el stderr del demonio no contiene la ruta del `tempdir` (O2) | 51 |
| E3 | Sesión `work_button` (vía `abrir_sesion` + `latir`) superpuesta a una `file_watch` del mismo proyecto | `tiempo_trabajado` = unión, no la suma (R4) | 51 |
| E4 | Solo leer (`cat`, `rg`) | Ninguna sesión (centinela) | 50 |
| E5 | Escribir en `node_modules/`, en un `*.log` ignorado por `.gitignore`, en `logs/app.log` y en `.vite/deps/x.js` (carpetas ampliadas, 3.10.4), y `mkdir target` en la raíz | Ninguna sesión (centinela); el stderr del demonio no contiene la ruta del `tempdir` (O2) | 51 |
| E6 | `mkdir -p a/b/c && echo x > a/b/c/f` | Cuenta | 51 |
| E7 | Renombrar una carpeta y escribir adentro; mover una carpeta **de afuera hacia adentro** y escribir; moverla **hacia afuera** y escribir; con dos proyectos A y B, mover `A/sub` → `B/sub` y escribir en `B/sub` | Las dos primeras cuentan para el proyecto; la tercera no (centinela); la última cuenta **solo** para B, y A suelta las vigilancias de `sub` (registro con conteo: `watched_dirs` de A baja) (T4) | 51 |
| E8 | `kill -9` con sesión abierta y relanzar | La huérfana cierra en su `last_activity_at` ± 1 s, con `duration = ended_at − started_at` | 50 |
| E9 | SIGTERM con sesión abierta | Cerrada en el último cambio; `pid = NULL` | 50 |
| E10 | 4 conexiones escriben en paralelo durante 20 s mientras el demonio mide | Cero `SQLITE_BUSY` en los escritores y en el log del demonio | 51 |
| E11 | Proyecto agregado, a la papelera y purgado en caliente | Se vigila; deja de medir en ≤ 5 s; el demonio no se cae | 51 |
| E12 | Proyecto con carpeta `/home` | No se vigila y aparece en `unwatched_projects` con `carpeta_amplia` (centinela) | 51 |
| E13 | Config ilegible | Defaults, `last_error` con el motivo, y el archivo intacto: mismos bytes y mismo `mtime` | 51 |
| E14 | Segundo demonio sobre la misma base, con **otro** `XDG_RUNTIME_DIR` | Sale con 75 por el `flock` (B8) | 50 |
| E15 | Restore con el demonio vivo, sin systemd (T1): (a) con el demonio corriendo, `db::restore::tomar_lock` (lib, A3) **falla** por el `flock` y la base no cambia; (b) se para el demonio, se restaura un backup de **esquema viejo** (`user_version = 0`, sin `last_activity_at` ni `tracker_status`), se relanza | (a) error en español, base igual byte a byte; (b) `integrity_check = ok`, `user_version = ESQUEMA_ACTUAL`, columnas y tablas nuevas presentes, `journal_mode = wal`, ninguna sesión abierta, y el demonio vuelve a medir | 51 |
| E16 | `systemd-run --user --unit=gestor-trackerd-test-$PID -p Type=notify -p WatchdogSec=5` con las mismas propiedades que arma el generador **salvo `WatchdogSec=5`** (el unit real usa 60), `--setenv` de `XDG_*` temporales y un proyecto bajo `$HOME` (T2, O1) | `is-active = active` tras `READY`; un guardado crea la sesión; `systemctl --user stop` la cierra; `~/.config/systemd/user` sin cambios | 56 |
| E17 | Cambiar `idle_minutes` y `min_credit_seconds` en la config con el demonio vivo (T4) | `tracker_status.idle_minutes` y `min_credit_seconds` cambian en ≤ 5 s sin reiniciar; `idle_minutes = 999` publica 60 (M14) | 51 |
| E18 | Agregar `logs/` al `.gitignore` en caliente y escribir `logs/x.log` (T4) | No cuenta (centinela); sacarlo del `.gitignore` y escribir de nuevo → cuenta | 51 |
| E19 | Paraguas `/t/p` con hijo `/t/p/h` (DX2, M2) | Un guardado en `h` cuenta solo para el hijo; en `/t/p/otra` para el paraguas; mandar el paraguas a la papelera deja al hijo midiendo | 51 |
| E20 | `daemon_enabled = false` en la config con el demonio vivo y una sesión abierta (B5, T4) | Sesión cerrada en su último cambio, salida con código 0 en ≤ 5 s, `estado = 'desactivado'`, `pid = NULL` | 51 |
| E21 | Concurrencia real con los métodos de la app (T4): un hilo hace el latido de `work_button` cada 250 ms, cierra con `end_tracking_session`, manda un proyecto a la papelera y lo purga, mientras el demonio mide otro proyecto | Ningún error, ningún `SQLITE_BUSY`; el total del proyecto superpuesto es la unión; el demonio suelta el purgado sin caerse | 51 |
| E22 | Kill switch al arrancar, con el systemd real (A1): `systemd-run --user --wait` como E16 (con `Restart=on-failure`), con `daemon_enabled = false` en la config temporal antes de lanzar | `--wait` termina en < 15 s con "Finished with result: success" y `status=0`. Con el defecto de A1 la unidad terminaría `protocol`, se reiniciaría y `--wait` no volvería: el plazo vencido es la falla. Una unidad transitoria que termina bien se descarga sola, por eso se lee el resultado de `--wait` y no de `systemctl show`; `~/.config/systemd/user` sin cambios | 56 |

**Lo que no se automatiza, y por qué:**
- **Suspensión real:** la cubren los tests del motor con reloj falso (commit 36: 8 h, `reanudado`) y los pasos manuales 11 y 26.
- **Watchdog con el bucle colgado:** no se puede colgar el bucle desde afuera sin instrumentarlo. Lo cubre el unitario del commit 49 (M13); que systemd reinicie al vencer el plazo se confía a systemd.
- **Reinicio de la PC y sesión gráfica real:** paso manual 10 (y 27 para el logout con una sesión de Trabajar).

#### E2E de las sesiones de terminal (E23–E37, pty con `script`)

**Dónde.** Todos en `src-tauri/tests/e2e_trackerd.rs`, con `#[ignore]`, para que
`cargo test --test e2e_trackerd` los corra y la cuenta de `check.sh --e2e` los vea.

**Tiempos unificados.** Una sola variable, con las claves de 3.5, y un parser que
rechaza claves desconocidas:

```
GESTOR_TRACKERD_TEST_TIMINGS=idle=3s,tick=250ms,flush=250ms,reload=1s,heartbeat=1s,config=200ms,rescan=1s,poll=250ms,grace=2s,pause=3s
```

**Dos perfiles de tiempos, por la granularidad del `atime`** (3.10.4: como mucho un cambio
por ventana alineada de 8 s). Con `pause=3s`, tipear durante más de 3 s pausaría el
tramo, porque dentro de una misma ventana el `atime` no se mueve; y reanudar exige dos
`atime` en ventanas distintas. Por eso:

| Perfil | `pause=` | Escenarios | Regla para escribir |
|---|---|---|---|
| **Archivos** (el de arriba) | `3s` | E24, E26, E27, E28, E29, E34, E35 | No dependen del tipeo |
| **Tipeo** | `20s` | E23, E25, E30, E31, E32, E33, E36, E37 | Cada escritura que tiene que mover el `atime` va a **≥ 9 s** de la anterior (otra ventana de 8 s); para reanudar desde una pausa, dos tandas a ≥ 9 s y < 60 s. Las afirmaciones sobre `ended_at` y `started_at` aceptan −8 s / +1 s |

Con `pause=20s`, 9 s entre escrituras nunca pausa, y la pausa llega en ≤ 20 + 8 + 1 s
después de la última. Los escenarios con tipeo duran más (unos 30–45 s cada uno); entra
en el +1 día de las correcciones (4.11). La presencia se fija con
`GESTOR_TRACKERD_TEST_PRESENCIA=<archivo>` (`inactivo_ms=<n> bloqueada=<0|1>` o
`no_disponible`); por defecto el harness escribe `inactivo_ms=0 bloqueada=0`, **en
todos** los escenarios, también E1–E22: desde el 48 `file_watch` depende
de la presencia (U1) y sin la variable leería el escritorio real (commit 50).
El demonio nunca se prueba contra el escritorio del desarrollador. Se sondea hasta un
plazo (10 s) y los negativos llevan centinela (T3).

**Aislamiento** (auditoría de pruebas, Alta). Vale para todos y en especial para los
de la CLI:

| Qué | Cómo |
|---|---|
| Base | `GESTOR_DB=$tmp/data/projects.db`, `XDG_DATA_HOME=$tmp/xdg-data` y `--data-dir` explícito. La CLI pasa `--data-dir "$(dirname "$GESTOR_DB")"` al subcomando, así los dos miran la misma base. El harness canoniza y **falla** si la ruta es `$HOME/.local/share/gestor-proyectos` o no está bajo el `tempdir`; el binario además se niega con `TEST_TIMINGS` (B8) |
| Shell y rc | `HOME=$tmp/home`, `ZDOTDIR=$tmp/home`, `XDG_CONFIG_HOME`, `XDG_RUNTIME_DIR` y `XDG_STATE_HOME` en el `tempdir`. `bash --norc --noprofile -i` y `zsh -f -i`; la función se carga con `source scripts/cli/gestor.sh` del repo |
| rc reales | El harness guarda el `sha256` de `~/.zshrc` y `~/.bashrc` reales antes de empezar y lo compara al final: si cambió, falla |
| Binarios | Rutas absolutas `/usr/bin/script`, `/usr/bin/bash`, `/usr/bin/zsh`, `/usr/bin/tmux`, `/usr/bin/dbus-daemon` y `/usr/bin/sqlite3`. En esta máquina `script`, `zsh`, `tmux`, `dbus-daemon`, `sqlite3` y `shellcheck` resuelven primero a linuxbrew (`which -a`). `shellcheck` **solo** está en linuxbrew: `check.sh` lo busca en el `PATH` y falla con un mensaje claro si no está. Si falta alguno de los de `/usr/bin` en Linux, el test **falla**, no se saltea |
| `PATH` | Mínimo: carpeta con los binarios compilados (`gestor-trackerd`), carpeta con `scripts/cli/gestor`, carpeta con un `fzf` falso (imprime la línea que elige el test) y `/usr/bin`. Nunca `~/.local/bin` |
| Marca heredada | El harness hace `env_remove("GESTOR_SESION")` en **cada** spawn y lo afirma: la terminal falsa vuelca su `environ` y el test verifica que no trae la marca del desarrollador |
| Proceso | El `Drop` del harness manda SIGKILL al demonio, a los `script` y al server de tmux (`tmux -L <tmp>`) |

**"Terminal" de prueba:** `/usr/bin/script -qfc "cat >/dev/null" /dev/null` con
`GESTOR_SESION=<token>` en el entorno del `Command`. `cat` **lee** el pty, así que
escribir en el stdin del `script` mueve el `atime`. La sesión se crea con los métodos
de la lib, como lo haría `trabajar` con el demonio vivo.

| # | Escenario | Se espera | Commit |
|---|---|---|---|
| E23 | Sesión del demonio + terminal de prueba (perfil tipeo); escribir 3 veces, a ≥ 9 s una de otra; matar el `script` | `viva_activa` en ≤ 1 sondeo; tramo con `started_at = creada_at` (frontera inicial de un token nacido con el demonio vivo, 3.10.2); al matar → token `cerrada/terminal_cerrada` con `terminada_at` ≈ último sondeo vivo; el tramo cierra en la **última escritura** (−8 s / +1 s), no en el kill; invariante DT1 | 52 |
| E24 | Dentro del `script`: `setsid -f sleep 600` y `nohup sleep 600 &`; matar el `script` | Se cierra igual: los huérfanos quedan sin tty | 52 |
| E25 | Dos `script` con el mismo token; matar uno y después el otro | Sigue viva tras el primero; `terminal_cerrada` tras el segundo; sin dos tramos abiertos | 52 |
| E26 | Sesión sin ningún proceso con su marca | `terminal_no_detectada` a los 2 s, con tramo en `creada_at`; sin archivos durante 3 s → tramo cerrado por `inactividad` (0 s → borrado) y la sesión **sigue** en `terminal_no_detectada`; un guardado con presencia → tramo nuevo; `parar` → `cerrada/parar_app`. Nunca `cerrada` por inactividad | 52 |
| E27 | `kill -9` al demonio con un tramo abierto; relanzar con el `script` vivo; escribir un archivo | Tramo viejo `tracker_caido` en su `last_activity_at` ± 1 s; nada cubre el hueco; tramo nuevo desde el cambio. Variante: `script` muerto mientras el demonio está caído → `cerrada/tracker_caido` | 52 |
| E28 | (a) `parar` desde el test en el mismo instante en que se mata el `script` (20 repeticiones). (b) Matar el único `script` y, en la misma ventana, reutilizar el token (`ultimo_clic_at = ahora`) y lanzar otro `script` con la marca (20 repeticiones) | (a) Un solo cierre por repetición, motivo `parar_app` o `terminal_cerrada`, ningún tramo abierto ni reabierto. (b) Nunca un `script` vivo con un token `cerrada` sin token nuevo: o el token sigue vivo, o el test (como `trabajar`) ve `cerrada` y crea otro que el demonio adopta | 52 |
| E29 | Pausa y reanudación por archivos: un cambio, 5 s de calma, otro cambio, matar. Variantes de presencia: `inactivo_ms=3600000`, `bloqueada=1`, `no_disponible` | Tramos `inactividad` (en el primer cambio ± 1 s), nuevo desde el segundo, cierre por terminal en el último cambio; unión con `file_watch` sin doble conteo. Ausente o bloqueada → el cambio **no** reanuda (bit 32 con sus segundos) y tampoco abre un tramo `file_watch` automático; `archivos_evidencia` tiene ese minuto con bit 32 (U1); `no_disponible` → reanuda con bit 8, y el `file_watch` abre su tramo con bit 8 (U2) | 52 |
| E30 | (a) Perfil tipeo: escribir cada 2 s durante 30 s y parar (el `atime` se mueve una vez cada 8 s, y alcanza para no pausar); `script -qfc "yes > /dev/tty"` sin entrada (drenando su stdout). (b) `script -qfc "bash --norc -i"`: `cd /tmp` y escribir; después `cd` a la carpeta y escribir dos veces, a ≥ 9 s y < 60 s | (a) Sin pausa mientras se escribe; pausa en ≤ 20 + 8 + 1 s con `ended_at` entre la última escritura − 8 s y + 1 s; `yes` se pausa (DT4). (b) Fuera de la carpeta no sostiene (bit 16); adentro, con dos `atime`, reanuda | 52 |
| E31 | SIGTERM con el tracker encendido y el `script` vivo; relanzar; escribir dos tandas a ≥ 9 s y < 60 s | Tramo `tracker_detenido`, token vivo en `viva_pausada`; al relanzar, `viva_pausada`, y con las dos tandas un tramo nuevo desde la primera. Variante: `script` muerto durante la parada → `cerrada/terminal_cerrada` en `last_seen_at`; si el `script` muere **antes** del SIGTERM, se acepta también el cierre por `terminal_cerrada` sin `tracker_detenido` (como en un logout real, 3.10.10) | 52 |
| E35 | Carrera del kill switch: `daemon_enabled = false` y SIGTERM inmediato, antes de la relectura de 5 s, con un `script` marcado vivo | `cerrada/tracker_apagado`, tramo en su UA; nunca un token vivo | 52 |
| E32 | CLI en `script -qfc "/usr/bin/bash --norc --noprofile -i"` y en `zsh -f -i`: `gestor abrir X`; el menú con el `fzf` falso; escribir; `exit` | Token `origen = cli` con ancla igual al pid de la shell; `pwd` es la carpeta de X; `GESTOR_SESION` exportada; `viva_activa`; al `exit` → `terminal_cerrada` con el tramo hasta la última tecla | 62 |
| E33 | Misma shell: `sleep 600 &`, `gestor abrir Y`, `gestor parar` | Ancla de X con `hasta`; X `cerrada/cambio_de_proyecto` (el `sleep` marcado no la sostiene); Y viva en el mismo pty; `parar` → Y `parar_cli` y el pty vetado: con X viva por otra terminal, esta shell no la sostiene | 62 |
| E34 | `gestor parar` sin sesión; `gestor abrir --sin-medir X` en una terminal del botón; `gestor ajustar X -30m "nota"`, `gestor ajustes X`, `gestor ajustar --revertir <id> "nota"` | Mensaje y código 0; `--sin-medir` → `cd`, veto del pty y la marca vieja deja de contar; como era la única terminal de la sesión del botón, la sesión termina **en el acto** con `parar_cli` (mismo proyecto) o `cambio_de_proyecto` (otro), nunca `terminal_cerrada` con la terminal abierta; ajuste `origen = cli` listado; reversión con `segundos` opuesto | 62 (ajustes: 63) |
| E36 | CLI dentro de tmux: `tmux -L <tmp> -f /dev/null new -d` con la función cargada; `gestor abrir X`; esperar la gracia; escribir en el panel dos tandas a ≥ 9 s; matar el server; `gestor parar X` desde otra shell | Ancla de multiplexor + aviso en stderr; `terminal_no_detectada` tras la gracia; el tipeo en el panel abre un tramo (el ancla de multiplexor aporta tipeo, 3.10.2); matar el server no la cierra; `parar` → `cerrada/parar_cli`, tramo en la última tecla | 62 |
| E37 | CLI sin demonio: `gestor abrir X`; esperar; arrancar el demonio; escribir dos tandas a ≥ 9 s y < 60 s | `esperando_demonio` con ancla y aviso; nada medido antes de `adoptada_at`; al arrancar → `viva_pausada`; con las dos tandas, tramo desde la primera (−8 s) | 62 |

El veto viejo sobre un pty reutilizado **no** tiene E2E propio: forzar que el kernel
reutilice un número de pty no es determinista. Lo cubren el `/proc` falso del commit 40 (R8-4) y el
test de integración del commit 61 (R8-C2) (3.10.1, "Vigencia real de un reclamo").

**Extensiones sin cambiar la cuenta** (van en el commit 52 salvo que se diga otra cosa):

- **E15(b), restore con tokens vivos:** el backup tiene un token `dueno = demonio` vivo y hay un `script` de esa marca vivo → después del restore, `cerrada/restauracion` y el `script` no reabre nada.
- **E20, kill switch con una sesión de terminal:** `script` marcado + `daemon_enabled = false` → `cerrada/tracker_apagado`, tramo en su UA.
- **E17:** cambiar `trabajar_idle_minutes` en caliente → `tracker_status` lo publica en ≤ 5 s.
- **E16 (56):** cubre `stat`, `environ`, `cwd`, el `atime` de `/dev/pts/N` y la presencia bajo el aislamiento real (3.10.3).
- **E29 (U1, U2):** suma el lado de `file_watch` (sin presencia no hay tramo automático y queda `archivos_evidencia`; sin dato, tramo con bit 8). No cambia la cuenta.

**`E2E_ESPERADOS`:** 21 → **31** (commit 52, R8-7: E23–E31 y E35) → **36** (commit 62, R8-C3: E32–E34,
E36 y E37). La constante de `scripts/check.sh` sube en cada uno de esos commits.

### 5.3 Recorrido manual

Instalación **real** con `install.sh`, nunca con `scripts/release.sh patch --install`,
que crea un commit y un tag de verdad (T6). Antes de empezar: backup desde Ajustes.

| # | Fase | Paso | Se espera |
|---|---|---|---|
| 1 | F0 | Lanzar la app dos veces seguidas | Una sola ventana y **un** backup nuevo en la carpeta de backups (P11) |
| 2 | F1, F2 | Con una copia de la base real (`XDG_DATA_HOME=$tmp pnpm run tauri:dev`): mirar tarjetas y Dashboard, "Trabajar" 5 s y Parar, papelera, restore | Totales iguales al diff del commit 24; Parar a los 5 s deja 0 s; tarjetas con tiempo aunque el proyecto no tenga `.gestor/` |
| 3 | F6 | `pnpm run tauri:build`, `./install.sh`, abrir la app una vez | Unit en `~/.config/systemd/user/`, binario en `~/.local/bin/` |
| 4 | F6 | `systemctl --user status gestor-trackerd` | `active (running)`; la app muestra "Tracker activo" |
| 5 | F6 | **Cerrar la app.** Editar dos proyectos durante 10 min, alternando | — |
| 6 | F6 | Abrir la app | Cada proyecto muestra su propio tiempo, sin repartir (R1, R6) |
| 7 | F7 | Con la app abierta, editar | El contador avanza con cada guardado, con ≤ 10 s de retraso (3.9) |
| 8 | F6 | Dejar pasar 3 min sin tocar nada | La sesión cierra en el último guardado (R3) |
| 9 | F6 | "Trabajar" + editar el mismo proyecto | El total no cuenta doble (R4) |
| 10 | F6 | Reiniciar la PC, **no** abrir la app, editar archivos; después abrir la app | El tiempo está (R6) |
| 11 | F6 | Editar, suspender la notebook 10 min, volver y editar | Dos sesiones; la pausa no cuenta, ni con el margen (DX1) |
| 12 | F6 | Ajustes → Desactivar | `systemctl --user is-enabled gestor-trackerd` → `disabled`; `is-active` → `inactive`; `sqlite3 …/projects.db "SELECT COUNT(*) FROM time_tracking_sessions WHERE source='file_watch' AND ended_at IS NULL"` → 0; con R8, también `SELECT COUNT(*) FROM trabajar_sesiones WHERE dueno = 'demonio' AND estado <> 'cerrada'` → 0 y, si había una viva, `… WHERE motivo_cierre = 'tracker_apagado'` ≥ 1. Cerrar y abrir la app: sigue apagado (B5) |
| 13 | F6 | Ajustes → Activar; después `systemctl --user disable --now gestor-trackerd` a mano; cerrar y abrir la app | La app **no** lo reactiva y dice "desactivado desde systemctl" (3.8) |
| 14 | F6 | Activar de nuevo; `systemctl --user stop gestor-trackerd`; abrir la app | "Tracker apagado" mientras está parado; al abrir la app vuelve a `active` (un `stop` sí se revierte) |
| 15 | F6 | Restaurar un backup | Base íntegra; el demonio está `active` **sin** reabrir la app (B7) |
| 16 | F6 | Con el demonio corriendo, `./start-app.sh` | El demonio sigue `active` (P10) |
| 17 | F6 | Recompilar y `./install.sh` con el demonio corriendo | Sin `ETXTBSY`; `systemctl --user show -p ExecMainPID gestor-trackerd` cambió (hubo `try-restart`). Si la versión cambió, `tracker_status.version` también |
| 18 | F6 | `systemd-analyze security --user gestor-trackerd` | Se anota el puntaje en el merge de F6 |

#### 5.3.1 Matriz manual por terminal (~45 min)

En esta máquina hay konsole, alacritty, yakuake, tmux y screen. Salvo konsole, cada
fila verifica solo adopción en ≤ 5 s, herencia en una pestaña nueva y cierre de la
última. gnome-terminal, kitty, xterm, xfce4-terminal y tilix quedan cubiertas solo por
el golden del argv y **fuera** del criterio de listo. La actividad se verifica con
`stat -c %x /dev/pts/N` (no con `w`: konsole no escribe en utmp).

| # | Terminal | Se espera | Min |
|---|---|---|---|
| 1 | konsole (default del usuario) | Recorrido 5.3.2 completo; es la única con la prueba de 12 min | (en 5.3.2) |
| 2 | konsole con `UseSingleInstance=true` | Pasa por `--separate` y el argv | 5 |
| 3 | alacritty | Pasa | 5 |
| 4 | tmux y screen (también `screen` interactivo, sin `-dm`) dentro de konsole | La sesión dura lo que la pestaña; el server no la sostiene; el `comm` exacto del server de screen queda anotado; el tipeo en un panel con cwd fuera de la carpeta no cuenta | 10 |
| 5 | yakuake como Custom | `terminal_no_detectada` con el aviso; Parar funciona | 5 |
| 6 | CLI en konsole con bash y con zsh | `gestor abrir X`, tipear, `exit` → `terminal_cerrada`; `gestor parar`; `--sin-medir` | 10 |
| 7 | `ssh` en primer plano en una konsole de A | Chip "remoto: ssh" | 5 |
| 8 | Default (`x-terminal-emulator` no existe acá) | Cae al fallback como Auto | 5 |

#### 5.3.2 Recorrido de las sesiones de Trabajar (konsole primero, pasos 19–42)

Se corre al cerrar F8 (validación de F8, sección 4), sobre la instalación real con `install.sh` y con backup previo desde Ajustes. Sigue la numeración de los pasos 1–18 de arriba. En cada paso se corre
la consulta de auditoría de 3.10.15.

| # | Paso | Se espera |
|---|---|---|
| 19 | Trabajar en A | Se abre konsole; la tarjeta pasa de "Esperando la terminal…" a "Midiendo · 1 terminal" en ≤ 5 s |
| 20 | Ctrl+Shift+T en esa konsole; cerrar la pestaña original | Sigue midiendo |
| 21 | Trabajar en B | Otra konsole; A sigue (D-R8-2); dos Parar |
| 22 | Trabajar en A otra vez | Otra konsole, **mismo** token (una sola sesión viva de A) |
| 23 | **Cerrar la app.** Tipear 5 min en A; no tocar nada 12 min; tipear dos teclas | Tramo `inactividad` que termina en la última tecla (−8 s como mucho); tramo nuevo desde la primera tecla al volver |
| 24 | Dejar A sin tipear 3 min y cerrar todas sus ventanas; reabrir la app | A `cerrada/terminal_cerrada`; su último tramo termina en la última tecla, **no** en el cierre (2a); B sigue con Parar |
| 25 | Parar en B | Toast con la duración; B `cerrada/parar_app`, tramo en la última actividad |
| 26 | Trabajar en A; suspender 10 min con la terminal abierta; volver y tipear | Tramo `suspension` + tramo nuevo desde la tecla |
| 27 | Trabajar en A; cerrar la sesión del escritorio con la terminal abierta y volver a entrar | Al volver, la sesión `cerrada/terminal_cerrada` y su último tramo en la última tecla. Valen los dos órdenes (3.10.10): con tramo `tracker_detenido` (SIGTERM antes de que mueran las konsole) o cerrado directo por `terminal_cerrada` (las konsole mueren ~10 s antes, por `UserStopDelaySec`) |
| 28 | En una konsole de A: `tmux`, trabajar adentro (en la carpeta de A), cerrar la konsole | El tipeo en el panel cuenta; termina al cerrar la konsole |
| 29 | En una konsole de A: `code .` y `npm run dev &`; cerrar la konsole | Termina; VSCode y node no la sostienen |
| 30 | En una konsole de A: `cd /tmp` y tipear 12 min | Pausa; el desglose muestra "tipeo fuera de la carpeta" |
| 31 | Pantalla bloqueada 15 min con Claude Code editando archivos de A (con Trabajar abierto) y de C (sin Trabajar) | Sin tiempo en A ni en C (tampoco automático); el desglose de cada uno muestra un tramo "Solo archivos, sin presencia" con su horario y duración; "Cargar como ajuste" sobre el de A precarga desde/hasta/segundos, pide la nota y, con la nota, el total de A sube exactamente eso (U1) |
| 32 | `ssh` en la konsole de A (sin autorizar ningún destino: dejar `ssh` esperando en el prompt de la contraseña de un host que elija el usuario, o `ssh localhost` si el usuario lo habilita) | Chip "remoto: ssh" |
| 33 | Trabajar en A y en B, alternando cada 5 min durante 20 min | Chip "simultáneo con" en los dos, sin reparto |
| 34 | Cargar −30 min con nota en A desde la app y revertirlo; lo mismo con `gestor ajustar` | Totales = medido + ajustes; el revertido aparece tachado; `COUNT(*)` de `ajustes_tiempo` solo sube |
| 35 | Mandar a la papelera un proyecto con sesión viva | Toast; `cerrada/proyecto_a_papelera` |
| 36 | Restaurar un backup con una sesión viva | El diálogo lista la sesión; `cerrada/restauracion` |
| 37 | Ajustes → desactivar el tracker con A viva; `gestor abrir A` | Toast de 3.10.12; A `cerrada/tracker_apagado`; la CLI hace el `cd`, avisa "Tracker apagado en Ajustes: no se mide." y no crea sesión (U5) |
| 38 | CLI: `gestor abrir A`; tipear; `gestor abrir B`; `gestor estado`; `gestor parar` | A termina con `cambio_de_proyecto`; B se inicia; `estado` lo lista; `parar` → B `cerrada/parar_cli` |
| 39 | Trabajar en A desde la app y `gestor abrir A` en otra konsole; cerrar la de la app | Misma sesión; sigue por la konsole de la CLI |
| 40 | `systemctl --user stop gestor-trackerd`; `gestor abrir A`; tipear; Trabajar en B desde la app; cerrar la app; `systemctl --user start gestor-trackerd`; tipear en A y en la konsole de B | "Esperando al tracker" en A; Trabajar en B muestra el toast "El tracker no está corriendo…" y la tarjeta "sin tracker… 12 h", y a los dos sondeos la cabecera muestra la alarma "no se mide el tiempo automático…" (U6); al cerrar la app, B queda "Esperando al tracker" (U4); nada medido antes de la adopción; tramos desde la primera tecla después; el tramo de B medido por la app lleva el chip `sin_vigilancia` |
| 41 | Matriz 5.3.1 | Lo anotado en cada fila |
| 42 | Cerrar todo | `gestor estado` vacío; consultas de §6 en 0 |

### 5.4 Datos reales

El test del commit 24 se corre así, sobre una copia y nunca sobre la base viva:

```bash
GESTOR_GOLDEN_DB=~/.local/share/gestor-proyectos/projects.db \
  cargo test --manifest-path src-tauri/Cargo.toml golden_diff -- --ignored --nocapture
```

El test copia la base con la API de backup desde una conexión de solo lectura y
trabaja sobre la copia. La salida tiene cuatro partes: diferencias por proyecto,
sesiones de más de 12 h, huérfanas y proyectos con carpeta inexistente. Cada
diferencia tiene que estar explicada en el CHANGELOG (commit 73): si aparece una
que no se puede explicar, la fase no se mergea.

### 5.5 Calidad automatizada: `scripts/check.sh`

Lo crea el commit 2. El modo `--e2e` se agrega en el commit 50, `--windows` en el 16
(3.10.3) y `shellcheck` en el 60.

```bash
#!/usr/bin/env bash
# Puerta de cada commit. Uso: scripts/check.sh [--e2e | --windows]
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ "${1:-}" == "--windows" ]]; then
  # Necesita la caché de cargo con las dependencias de Windows: una vez, con red,
  # `cargo fetch --locked --target x86_64-pc-windows-gnu` (paso manual del usuario).
  (cd src-tauri && cargo check --locked --target x86_64-pc-windows-gnu) || {
    echo "cargo check de Windows falló; si falta la caché, ver 3.10.3" >&2; exit 1; }
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
command -v shellcheck >/dev/null || { echo "Falta shellcheck en el PATH" >&2; exit 1; }
shellcheck -s bash scripts/cli/gestor scripts/cli/gestor.sh scripts/cli/instalar-cli.sh \
  scripts/cli/gestor-completion.bash   # la de zsh queda afuera: shellcheck no soporta zsh

# Cantidad de E2E automatizados que tienen que correr: 6 en el commit 50, 21 desde
# el 51, 31 desde el 52 y 36 desde el 62.
E2E_ESPERADOS=36

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

- `cargo fmt --check` pasa desde el commit 1, que formatea el crate entero (antes de F0 había 127 bloques sin formatear; desde `476a936`, ninguno).
- `--test-threads=1` en los E2E: cada uno lanza su demonio con tiempos cortos, y en paralelo competirían por CPU y volverían inestables los plazos.
- E16 y E22 se excluyen del script: necesitan el bus de usuario de systemd y se corren a mano (5.1).
- `pnpm run lint` sin `--fix`: la puerta no modifica archivos.
- `--windows` **no** entra en la corrida por defecto, que tiene que andar sin red: la caché de cargo no tiene las dependencias de Windows (3.10.3, riesgo 50). Hasta que exista la CI, su salida se pega a mano en el PR del commit 16.
- `shellcheck` está solo en linuxbrew en esta máquina: el script lo busca en el `PATH` y falla con un mensaje claro si no está.

**CI (DX5, decisión abierta, 9.2):** `.github/workflows/check.yml` (hoy no existe
`.github/`) que corra `scripts/check.sh` en `ubuntu-latest`, sin `--e2e` ni E16/E22.
Requiere autorización aparte, por ser remoto. Es también el lugar natural de
`check.sh --windows` (3.10.3).

### 5.6 Tests existentes que se rompen con R8

- **Rust, `commands/mod.rs`, módulo `work_session_tests` (1933–2389, 15 tests):** los **14** que tocan `start_new_session_sync`, `stop_active_session_sync` o `work_session_status_sync` se reescriben contra `trabajar`, `parar`, `al_cerrar_ventana` y `estado_trabajar`; el **15.º**, 2257 (`duracion_a_guardar…`), es una función pura que no toca ninguna de las tres: solo suma el caso `terminal`. Cambian de **semántica** 2049 y 2115 (mismo proyecto dos veces = mismo token) y 2170/2342 (cerrar la ventana deja el token en espera). 2287, 2308 y 2356 conservan la semántica del tramo y afirman además el token en `esperando_demonio`. `main.rs:78-98` cambia a `al_cerrar_ventana`.
- **Frontend:** `api.test.ts:857-905`, `test-setup.ts:137-156`, `ProjectList.work.test.tsx` (61, 92, 113, 150, 159, 171, 193, 211, 226, 247, 263; **193, 211 y 226 son los tres de `83c48d4`: se conserva la intención** —la recarga después de Parar, y en 226 la recarga cuando Trabajar cierra la sesión anterior, que es justo el caso del cambio de proyecto) y `Dashboard.test.tsx:225-270`. El renombre `previous_session_stopped` → `sesion_anterior` toca `api.test.ts:863-875`, `test-setup.ts:142`, `Dashboard.test.tsx:243` y `ProjectList.work.test.tsx:234`. `src/utils/workSession.ts` (sin test propio) pierde el callback `abrirTerminal`, y con él sus llamadores `Dashboard.tsx:52-67` y `ProjectList.tsx:230`. `projectStore.test.ts` no se rompe (`open_terminal` sigue, con `None`).

---
## 6. Definición de "listo"

Cada punto se verifica con un comando o un paso concreto; ninguno queda a criterio.

- [ ] **Calidad:** `scripts/check.sh` en verde sobre `main` después del merge de F8, incluido el `shellcheck` de la CLI (5.5: `scripts/cli/gestor`, `gestor.sh`, `instalar-cli.sh` y el completado de bash; `shellcheck` está solo en linuxbrew en esta máquina; `check.sh` falla con un mensaje claro si no lo encuentra).
- [ ] **Windows compila:** `cargo check --locked --target x86_64-pc-windows-gnu` en verde: en la CI (DX5) o, mientras no exista, con `scripts/check.sh --windows` corrido a mano después de un `cargo fetch` con red que autoriza el usuario, y la salida pegada en el PR. No forma parte del `check.sh` por defecto, que tiene que andar offline (3.10.3, riesgo 50).
- [ ] **E2E estables:** `scripts/check.sh --e2e` pasa completo: E1–E15, E17–E21 y E23–E37 (36 tests, `E2E_ESPERADOS=36`, verificado por la cuenta de 5.5), **tres corridas seguidas** sin un solo fallo. Un test inestable se arregla, no se reintenta.
- [ ] **Rc intactos:** los `sha256` de `~/.zshrc` y `~/.bashrc` reales son iguales antes y después de `check.sh --e2e`.
- [ ] **E16 y E22** (`cargo test --test e2e_trackerd -- --ignored e16`, ídem `e22`) pasan en esta máquina con el systemd del usuario. E16 extendido (`cwd`, presencia, `/dev/pts`) con la salida pegada en el PR de F6.
- [ ] **Vitest** en verde, con los tests de `83c48d4` en `ProjectList.work.test.tsx` conservando su intención.
- [ ] **Recorrido manual** 5.3 completo (pasos 1–42) y matriz por terminal 5.3.1, sobre la instalación real hecha con `install.sh`, anotados.
- [ ] **Kill switch probado** (paso manual 12): después de desactivar desde Ajustes, `systemctl --user is-enabled gestor-trackerd` devuelve `disabled`, `is-active` devuelve `inactive` y la consulta de sesiones `file_watch` abiertas devuelve **0**, igual que `SELECT COUNT(*) FROM trabajar_sesiones WHERE dueno = 'demonio' AND estado <> 'cerrada'`; si había una sesión del demonio viva, `SELECT COUNT(*) FROM trabajar_sesiones WHERE motivo_cierre = 'tracker_apagado'` da ≥ 1. Al reabrir la app sigue apagado.
- [ ] **Diff con datos reales** (5.4) corrido sobre una copia fresca de la base real, con **cada** diferencia explicada en el CHANGELOG. Las decisiones de la sección 9 sobre sesiones > 12 h, huérfanas y carpetas inexistentes están tomadas y aplicadas (o explícitamente "dejar").
- [ ] **Cordura de la unión:** en la salida del diff, ningún proyecto suma en un día local más de 86.400 s (90.000 s en un día de 25 h).
- [ ] **Recursos**, después de **1 h** de demonio corriendo sobre los proyectos reales, con `scripts/medir-trackerd.sh` (commit 71), que hace:
  ```bash
  pid=$(systemctl --user show -p MainPID --value gestor-trackerd)
  ps -o rss=,%cpu= -p "$pid"                                   # RSS en KiB
  systemctl --user show -p MemoryCurrent --value gestor-trackerd   # memory.current del cgroup, incluye marcas de inotify
  a=$(systemctl --user show -p CPUUsageNSec --value gestor-trackerd); sleep 600
  b=$(systemctl --user show -p CPUUsageNSec --value gestor-trackerd)
  awk -v a="$a" -v b="$b" 'BEGIN { printf "CPU en reposo: %.2f %%\n", (b - a) / 6e9 }'   # 600 s → %
  sqlite3 ~/.local/share/gestor-proyectos/projects.db "SELECT watched_dirs, watched_projects FROM tracker_status"
  ```
  Umbrales: **RSS < 50 MB**, **`MemoryCurrent` < 128 MB** (debajo de `MemoryHigh=192M`, O3), **CPU < 1 %** en los 10 min sin tocar archivos. Con R8, la misma medición con **3 sesiones de Trabajar vivas** y el hilo de presencia: CPU < 1 %, RSS < 50 MB. Se anotan los valores en el merge de F8.
- [ ] **Sin tokens en los logs:** `journalctl --user -u gestor-trackerd | rg '[0-9a-f]{32}'` no imprime nada (lo mismo sobre los logs de la app).
- [ ] **Sin sesiones colgadas:** al final, con todo cerrado, `gestor estado` no lista ninguna sesión.
- [ ] **Invariantes SQL** (6.2): las 21 dan lo esperado sobre la base real después del recorrido.
- [ ] **Matriz de trazabilidad** (abajo) sin huecos: cada fila tiene commit y prueba, y la prueba pasa.
- [ ] **Documentación:** `ARQUITECTURA.md` y `CLAUDE.md` describen el tracking nuevo (sección Time tracking, kill switch, piso de downgrade, sesión atada a la terminal, motivos, ajustes, CLI); `docs/cli.md` describe la medición desde la CLI (commit 73); el CHANGELOG del release `minor` tiene el "piso de downgrade" (7.2).

### 6.1 Matriz de trazabilidad (T12)

Commits con la numeración de la sección 4. "Manual N" es el paso N de 5.3.

**Problemas de partida (2.2):**

| # | Commit | Prueba |
|---|---|---|
| P1 Dos totales | 21, 25 | Tests del 21 (sin superposición, mismos números); `rg total_time_seconds` del 25; E3 |
| P2 Cierre doble suma doble | 3 | "Cerrar dos veces suma una" |
| P3 Hoy/semana en UTC, semana de 8 fechas | 11, 21 | Tests de zonas (11); semana = 7 fechas y `active_today` local (21) |
| P4 Huérfanas | 12, 19, 43 | Tests de huérfanas del 19 y del 43; E8 |
| P5 Badge sin refresco | 65 | Vitest del badge |
| P6 Restore con `rename` | 27, 57 | "Restore con `-wal` presente no corrompe" (27); E15; manual 15 |
| P7 Error de "Trabajar" invisible | 6 | Vitest del toast |
| P8 "Abrir" del Dashboard no mide | 9 | Vitest |
| P9 Sin botón Parar | 7 | Vitest + test del comando |
| P10 `pkill -f` mata al demonio | 53 | Manual 16 |
| P11 Base abierta antes de la instancia única | 4 | Manual 1 |
| P12 `ConfigManager` en el demonio | 47 | "Config ilegible no se toca" (47); E13 |
| P13 Transacciones diferidas | 28 | Dos conexiones en `restore_project` sin `SQLITE_BUSY_SNAPSHOT`; guarda de `purge_project` dentro de la transacción; E10, E21 |
| P14 Migraciones sin versión | 10 | Tests (a)–(f) del 10 |
| P15 Stats solo con `.gestor/` | 23 | Vitest sin `.gestor/`; manual 2 |
| P16 `try_lock` | 5 | Lock sostenido 200 ms; Mutex envenenado |
| P17 Ruta de la base en `backup/` | 26 | Test de rutas con XDG temporales |
| P18 Error → 0 s | 21 | "Tabla borrada → `Err`" |
| P19 Permisos 0644 | 29, 30 | Test de modos (29); `-wal`/`-shm` en `0600` (30) |

**Requisitos (1):**

| # | Commit | Prueba |
|---|---|---|
| R1 Cada proyecto lo suyo | 36, 37 | Motor "dos proyectos alternando"; atribución más específica; E2, E19; manual 5–6 |
| R2 Actividad en archivos, leer no cuenta | 39, 45 | Filtros del 39; E4, E5, E6, E7, E18; manual 7 |
| R3 Corte de 3 min en el último cambio | 36 | Motor "2:59 contra 3:00", "evento antes del tick"; E1, E1b, E17; manual 8 |
| R4 Unión con "Trabajar" | 20, 21 | Tests de tabla del 20; E3, E21; manual 9 |
| R5 Solo Linux | 54 | "Plataforma ≠ Linux → ninguna llamada a systemctl" |
| R6 Corre con la app cerrada, arranca solo, apagable | 49, 54, 55, 56, 57 | Tests del trait (54, 55); E16, E22; manual 3–6, 10, 12–15 |
| R7 Carpetas no medibles | 37, 38, 46, 68 | `/home` y symlink a `/` rechazados; `en_red`; tope+1 carpetas; E12; Vitest de avisos (68) |

**Bloqueantes de la auditoría (Anexo A, v3):**

| # | Commit | Prueba |
|---|---|---|
| B1 Migraciones y demonio que no migra | 10, 43, 49 | Tests (a)–(f) del 10; `user_version` futuro → ninguna escritura, y `ESQUEMA_ACTUAL − 1` → `esperando_migracion` y `activo` al migrar desde otra conexión (43); `false` mientras espera → `READY`, `STOPPING`, salida 0 (49); E15 |
| B2 Tiempo invisible sin `.gestor/` | 23 | Vitest del 23 |
| B3 Contador sumado con 0 filas | 3 | "Cerrar dos veces suma una" |
| B4 `default-run` y bundle | 34 | `cargo run` sin `--bin` lanza la app; tarea `dpkg -c` anotada en 3.1 |
| B5 Kill switch | 36, 49, 54, 55, 56, 66 | Motor "cerrar todo y salir"; `NOTIFY_SOCKET` falso: `READY` antes de salir (49); E20, E22; trait `false` → `disable --now`; Vitest del toggle; manual 12–13 |
| B6 Orden roto | 1, 2, 13, 35, 50 | 4.10 sin dependencias hacia adelante; `check.sh` desde el commit 2 |
| B7 Restore deja el demonio parado | 27, 57 | Orden de llamadas con systemctl falso (57); E15; manual 15 |
| B8 Lock y entorno | 34 | Segundo demonio con otro `XDG_RUNTIME_DIR` → 75; guardas de debug y de timings; E14 |
| B9 Unit desde `target/`, comparación por versión | 54 | Golden del unit; exe en `target/` → nada; un byte distinto → reescribe |

**R8 y decisiones del usuario (§1.1).** "Paso N" es el paso N de 5.3.2; "invariante N", la de 6.2:

| # | Commit | Prueba |
|---|---|---|
| R8 Sesión atada a la terminal | 15, 16, 17, 40, 42, 44, 48, 58 | Golden del argv y terminal falsa (16); `/proc` falso (40); tabla de transiciones (42); E23–E26; matriz 5.3.1; pasos 19–24 |
| R8-CLI (D-R8-5) | 60, 61, 62, 63 | Integración del subcomando (61); E32–E34, E36, E37; pasos 37–39 |
| D-R8-1 Cerrar la app no termina | 17, 19, 58 | Cerrar la ventana → token en espera (17); "cerrar la ventana con una sesión del demonio → intacta" (58); paso 23 |
| D-R8-2 Proyectos conviven | 17, 58 | "A y B `dueno = demonio` conviven" (58); paso 21 |
| D-R8-3 Pausa a los 10 min | 35, 42, 48 | Motor "pausa 9:59 contra 10:00" (42); E29, E30; paso 23 |
| D-R8-4 Mismo proyecto, misma sesión | 17, 61 | "A dos veces → mismo token" (17); E25; paso 22 |
| 1a Presencia | 41, 42, 48 | Bus D-Bus privado (41); presencia falsa en el motor (42); E29; paso 31 |
| 2a La cola no se cobra | 42, 44 | Invariantes 1–3; E23, E30; paso 24 |
| 3a Atribución por cwd | 40, 42 | `/proc` falso con cwd dentro y fuera (40); E30 (b); paso 30 |
| 4a Simultáneo | 20, 69 | "A 10–12 y B 11–13 → 1 h simultánea" (20); Vitest del desglose (69); paso 33 |
| 5a Ajustes | 22, 63, 70 | Triggers y validaciones (22); E34; Vitest (70); paso 34; invariantes 15 y 16 |
| U1 Presencia en `file_watch` y `solo_archivos` | 15, 48, 69, 70 | Compuerta del bucle (48); E29; desglose (69); paso 31; invariante 20 |
| U2 Sin dato de presencia, los archivos cuentan | 41, 42, 48 | Bus caído → `NoDisponible` (41); bit 8 en el motor (42) y en el bucle (48); E29 (`no_disponible`) |
| U3 Paraguas sostenido por el hijo | 42 | "Guardado en el hijo → la sesión del paraguas sigue `viva_activa`" (42); riesgo 41 |
| U4 Cierre de la app según el tracker | 17, 35, 54 | Sin binario → `cierre_app`, con binario falso → espera (17); `daemon_enabled = false` → `cierre_app` (35); exe en `target/` → `cierre_app` y producción sin binario → espera (54); paso 40 |
| U5 `gestor abrir` con el tracker apagado no mide | 61 | "Apagado → 3" (61); paso 37 |
| U6 Sin tracker | 17, 19, 66, 69 | Recorte a 12 h (17, 19); `aviso_cabecera` (alarma y línea de Trabajar) y toast (17, 66); chip `sin_vigilancia` (69); paso 40; invariante 21 |
| U7 Una sola etapa | 20, 22, 69, 70 | Simultaneidad (20), ajustes (22, 70) y desglose (69) entran en los 73 commits; invariante 18 (`user_version` = 5) |

### 6.2 Invariantes SQL sobre la base real

Se corren después del recorrido de 5.3.2 (paso 42). Cada consulta tiene que dar
**0**, salvo que se indique otra cosa. Se pueden pegar tal cual en `sqlite3`.

```sql
-- 1. Tramos terminal cerrados: ended − started = duration
SELECT COUNT(*) FROM time_tracking_sessions
WHERE source = 'terminal' AND ended_at IS NOT NULL
  AND CAST(strftime('%s', ended_at) AS INTEGER) - CAST(strftime('%s', started_at) AS INTEGER) <> duration_seconds;

-- 2. Tramos terminal cerrados terminan en su última actividad (2a)
SELECT COUNT(*) FROM time_tracking_sessions
WHERE source = 'terminal' AND ended_at IS NOT NULL AND last_activity_at <> ended_at;

-- 3. Tramo terminal abierto: last_activity − started = duration
SELECT COUNT(*) FROM time_tracking_sessions
WHERE source = 'terminal' AND ended_at IS NULL
  AND CAST(strftime('%s', last_activity_at) AS INTEGER) - CAST(strftime('%s', started_at) AS INTEGER) <> duration_seconds;

-- 4. Ningún cierre nuevo sin motivo
SELECT COUNT(*) FROM time_tracking_sessions
WHERE ended_at IS NOT NULL AND motivo_cierre IS NULL
  AND (terminal_token IS NOT NULL OR source IN ('terminal', 'file_watch'));

-- 5. Sesión cerrada ⇔ motivo y terminada_at presentes
SELECT COUNT(*) FROM trabajar_sesiones
WHERE (estado = 'cerrada') <> (motivo_cierre IS NOT NULL)
   OR (estado = 'cerrada') <> (terminada_at IS NOT NULL);

-- 6. Ningún tramo abierto en una sesión que no lo admite
SELECT COUNT(*) FROM time_tracking_sessions t
JOIN trabajar_sesiones s ON s.token = t.terminal_token
WHERE t.ended_at IS NULL
  AND s.estado IN ('cerrada', 'viva_pausada', 'esperando_terminal', 'esperando_demonio');

-- 7. A lo sumo un tramo abierto por token
SELECT COUNT(*) FROM (SELECT terminal_token FROM time_tracking_sessions
  WHERE terminal_token IS NOT NULL AND ended_at IS NULL
  GROUP BY terminal_token HAVING COUNT(*) > 1);

-- 8. Tramos de un mismo token sin superposición
SELECT COUNT(*) FROM time_tracking_sessions a JOIN time_tracking_sessions b
  ON a.terminal_token = b.terminal_token AND a.id < b.id
WHERE a.started_at < COALESCE(b.ended_at, b.last_activity_at)
  AND b.started_at < COALESCE(a.ended_at, a.last_activity_at);

-- 9. terminada_at nunca antes del fin de un tramo de la sesión
--    (se cumple por construcción: regla única de terminada_at, 3.10.15)
SELECT COUNT(*) FROM trabajar_sesiones s JOIN time_tracking_sessions t
  ON t.terminal_token = s.token
WHERE s.estado = 'cerrada' AND t.ended_at > s.terminada_at;

-- 10. Un tramo abierto lo mide el dueño actual de su sesión
--     (dueno cambia solo sin tramo abierto, 3.10.7; los tramos cerrados de un
--     dueño anterior son válidos, por eso solo se miran los abiertos)
SELECT COUNT(*) FROM time_tracking_sessions t JOIN trabajar_sesiones s
  ON s.token = t.terminal_token
WHERE t.ended_at IS NULL
  AND ((t.source = 'terminal' AND s.dueno <> 'demonio')
    OR (t.source = 'work_button' AND s.dueno <> 'app'));

-- 11. A lo sumo una sesión viva medida por la app (da 0 o 1)
SELECT COUNT(*) FROM trabajar_sesiones WHERE dueno = 'app' AND estado <> 'cerrada';

-- 12. Ninguna sesión viva en un proyecto de la papelera
SELECT COUNT(*) FROM trabajar_sesiones s JOIN projects p ON p.id = s.project_id
WHERE p.deleted_at IS NOT NULL AND s.estado <> 'cerrada';

-- 13. Una shell anclada a un solo reclamo vigente
SELECT COUNT(*) FROM (SELECT pid, starttime FROM trabajar_anclas
  WHERE hasta IS NULL GROUP BY pid, starttime HAVING COUNT(*) > 1);

-- 14. Evidencia sin rutas absolutas ni que salgan de la carpeta
--     (por componente: `a..b.txt` es un nombre legítimo; las dos tablas, U1)
SELECT COUNT(*) FROM (
  SELECT archivo_rel FROM trabajar_evidencia
  UNION ALL SELECT archivo_rel FROM archivos_evidencia)
WHERE archivo_rel LIKE '/%'
   OR archivo_rel = '..' OR archivo_rel LIKE '../%'
   OR archivo_rel LIKE '%/../%' OR archivo_rel LIKE '%/..';

-- 15. Ajustes: reversiones exactas y nunca de otra reversión
SELECT COUNT(*) FROM ajustes_tiempo r JOIN ajustes_tiempo o ON o.id = r.revierte_id
WHERE r.segundos <> -o.segundos OR r.project_id <> o.project_id OR o.revierte_id IS NOT NULL;

-- 16. Ajustes append-only: los dos triggers existen (da 2)
SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger'
  AND name IN ('ajustes_tiempo_sin_update', 'ajustes_tiempo_sin_delete');

-- 17. Al final, ninguna sesión viva
SELECT COUNT(*) FROM trabajar_sesiones WHERE estado <> 'cerrada';

-- 18. Esquema al día (da 5: entrega en una sola etapa, U7)
PRAGMA user_version;

-- 19. Una sesión en viva_activa siempre tiene su tramo abierto
--     (vale con dueno = demonio y con dueno = app; todo cambio de estado
--     y su tramo van en la misma transacción IMMEDIATE, 3.10.7)
SELECT COUNT(*) FROM trabajar_sesiones s
WHERE s.estado = 'viva_activa'
  AND NOT EXISTS (SELECT 1 FROM time_tracking_sessions t
                  WHERE t.terminal_token = s.token AND t.ended_at IS NULL);

-- 20. Lo no cobrado por ausencia queda registrado con sus segundos (U1):
--     archivos_evidencia solo tiene bits 8 y 32, y todo minuto con bit 32
--     (en las dos tablas) tiene ausente_desde_s ≤ ausente_hasta_s
SELECT COUNT(*) FROM archivos_evidencia WHERE fuentes = 0 OR (fuentes & ~40) <> 0;
SELECT COUNT(*) FROM (
  SELECT fuentes, ausente_desde_s, ausente_hasta_s FROM trabajar_evidencia
  UNION ALL SELECT fuentes, ausente_desde_s, ausente_hasta_s FROM archivos_evidencia)
WHERE (fuentes & 32) <> 0
  AND (ausente_desde_s IS NULL OR ausente_hasta_s IS NULL OR ausente_desde_s > ausente_hasta_s);

-- 21. Ningún tramo medido por la app pasa el tope de 12 h (U6)
SELECT COUNT(*) FROM time_tracking_sessions
WHERE source = 'work_button' AND terminal_token IS NOT NULL
  AND COALESCE(duration_seconds, 0) > 43200;
```

Además, `ajustes_tiempo` solo crece: se anotan `COUNT(*)` y `MAX(id)` antes y después
del recorrido y ninguno baja, y un `UPDATE ajustes_tiempo SET nota = 'x'` sobre una
**copia** de la base falla con "ajustes_tiempo es solo de agregado".

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
| 24 | El usuario no se entera de que el tracker está apagado o roto | Baja | Medio | Indicador siempre visible con 6 estados (3.9) y, si se espera el tracker y no corre, la alarma de cabecera "no se mide el tiempo automático" (U6, 3.10.14) |
| 25 | Carpetas compartidas cuentan para dos proyectos | Alta | Bajo | Es intencional (3.4) y está avisado en la tarjeta |
| 26 | Downgrade de la app rompe la base o la config | Baja | Crítico | Runbook de abajo; piso de downgrade = primera versión con F2 |
| 27 | Terminal olvidada abierta toda la noche | Alta | Alto | Pausa de 10 min (D-R8-3); `mtime` no cuenta (DT4); con 2a, lo que se cobra termina en la última actividad |
| 28 | Descendientes que heredan la marca (`node`, Chrome, `code`) | Alta | Alto | `tty_nr ≠ 0` y `Z` excluido (3.10.3); E24; pasos manuales 28 y 29 |
| 29 | tmux o screen: el server sobrevive a la ventana | Media | Medio | Se excluyen los descendientes del server; el cliente cuenta. Con la CLI, el ancla dentro de un panel es de **multiplexor** y no sostiene (E36). El tipeo se atribuye por el panel activo (heurística, riesgo 48) |
| 30 | Terminales de instancia única o servidores que no pasan el entorno | Media | Medio | Flags y doble inyección (commit 16, R8-2); gracia y `terminal_no_detectada`, que **pausa** y mide solo archivos con presencia; aviso y Parar. Matriz 5.3.1 |
| 31 | Pestañas de **otra cosa** abiertas desde la konsole de Trabajar | Media | Bajo | Mitigado por DT11: si el primer plano está fuera de la carpeta, el tipeo no cuenta. Sigue contando una pestaña ajena **dentro** de la carpeta |
| 32 | `exec sudo -i` o `exec su` en la shell de la sesión | Baja | Bajo | El proceso root no es legible: fin prematuro. Documentado. Con `sudo vim` (no `exec`), el cwd se toma de la shell. **Riesgo aceptado (AF-20):** el desglose no lleva una marca "terminal perdida"; la sesión cierra con `terminal_cerrada` en el último sondeo que la vio |
| 33 | Precisión | — | — | Cierre de la terminal ±5 s (solo `terminada_at`); actividad por pty −8 s; suspensión cortada en la UA. Todo error es de contar de menos, salvo las excepciones al principio 4 (§3) |
| 34 | `/proc`, `cwd` o `/dev/pts` invisibles bajo el aislamiento del unit | Baja | Alto | Prohibidas `ProtectProc`, `ProcSubset`, `PrivatePIDs` y `PrivateDevices` (54, §6); E16 bloqueante para F6 |
| 35 | Dos dueños escriben `duration` | Media | Alto | Un solo medidor por tramo: `dueno` (actual) nunca cambia con un tramo abierto (DT14); invariante 10; cierres condicionales `IMMEDIATE`; índice único de tramo abierto; E28 |
| 36 | La app cierra sesiones vivas del demonio durante un `restart` o un backoff | Baja | Medio | Sin reconciliación que adopte; la app cierra solo con `demonio_vivo()` falso **y** prueba de cierre (anclas o `last_seen_at`) |
| 37 | Herencia accidental de la marca (la app lanzada desde una terminal marcada) | Media | Medio | `limpiar_entorno_heredado()` antes de `logging::init()` con test; `env_remove` en cada spawn |
| 38 | Privacidad de `environ` y `cwd` | Baja | Alto | Se leen solo para decidir; nunca se loguean ni se guardan; token truncado a 8 caracteres en el log; evidencia con rutas **relativas** |
| 39 | CPU del escaneo y de la presencia | Baja | Bajo | Escaneo solo con sesiones vivas; presencia en su hilo (2 llamadas D-Bus cada 5 s); medido en 71 |
| 40 | Después de Parar, la terminal sigue abierta y no mide | Media | Bajo | Toast "Para seguir midiendo en esa terminal, corré `gestor abrir X` ahí o tocá Trabajar" |
| 41 | Paraguas: un guardado en un hijo sostiene la sesión de Trabajar del paraguas (literal de D-R8-3) | Media | Bajo | Decidido así (U3). Se muestra "simultáneo con" el hijo |
| 42 | Función `gestor` vieja en el `.zshrc` | Media | Bajo | Sin `GESTOR_SESION_FILE` el script no mide y dice cómo actualizar; los rc solo se tocan con `--rc` |
| 43 | `--shell-pid` falsificado | Baja | Medio | Validación de ancestro, UID, estado y tty; la identidad la lee el binario |
| 44 | CLI sin demonio | Media | Bajo | Crea la sesión en `esperando_demonio`; se mide desde la adopción, sin inventar el hueco (E37) |
| 45 | `atime` del pty en sistemas raros | Baja | Medio | `tty_nr` sin `/dev/pts` correspondiente sostiene pero no aporta actividad; `last_error` |
| 46 | Escritorio sin API de presencia (ni KDE ni GNOME), o bus caído | Media | Medio | Fallback explícito (U2): los archivos cuentan con la marca `presencia_desconocida`, también en `file_watch`; Ajustes lo muestra. Excepción 2 al principio 4 (§3) |
| 47 | Una actualización de KDE devuelve el idle en segundos | Baja | Alto | Chequeo de unidades con test; se anota en `presencia_motivo` |
| 48 | Heurística del panel activo de tmux falla | Media | Bajo | Si no se resuelve, el tipeo **no** cuenta (principio 4); paso manual 28 |
| 49 | Sesiones "Esperando al tracker" que nadie cierra: del botón que el demonio nunca vio, o en la rama de F6 entre el 54 y el 58 | Baja | Bajo | No suman tiempo. Las del botón pasan a `terminal_no_detectada` al adoptarlas y piden Parar. En F1–F5 no existen: sin el binario del tracker la adopción es imposible y todo termina como hoy (3.10.7); en desarrollo tampoco, porque desde el 54 la app desde `target/` cuenta como adopción imposible |
| 50 | `windows.rs` deja de compilar con la firma nueva | Media | Medio | `scripts/check.sh --windows` (`cargo check --locked --target x86_64-pc-windows-gnu`). **No** corre offline: la caché de cargo no tiene `anstyle-wincon v3.0.11` ni el resto de las dependencias de Windows (`cargo check --offline` sale con 101). Llenarla necesita red una vez (`cargo fetch --locked --target …`), como paso manual del usuario; el lugar estable es la CI (DX5). Hasta entonces, la salida se pega a mano en el PR del commit 16 (R8-2) |
| 51 | Un ajuste positivo sobre un rato ya medido cuenta doble (5a suma) | Media | Medio | El formulario avisa la superposición y propone el resto (3.10.17) |
| 52 | Tracker caído de forma permanente con la app cerrada: nadie avisa | Baja | Alto | La tarjeta y `gestor estado` lo muestran; Parar guarda hasta la UA. `OnFailure=` con `notify-send` queda como mejora opcional (no verificado contra `StartLimitIntervalSec=0`) |
| 53 | Tiempo de pared doble entre dos proyectos (4a) | Alta | Medio | Se muestra "simultáneo con", sin reparto: lo decide el usuario |
| 54 | Eventos de foco o mouse (vim, htop, tmux `mouse on`) mueven el `atime` | Media | Bajo | Reanudar desde una pausa exige dos `atime` en 60 s |
| 55 | Herramientas que escriben en la carpeta **con el usuario presente** (dev server que reescribe logs o una base `db.sqlite3`) sostienen la sesión (AF-2) | Media | Medio | Carpetas siempre ignoradas ampliadas (3.10.4, commit 39); `.gitignore` anidado; lo que quede se ve en la evidencia por minuto (`archivo_rel`) y se agrega a `.gestorignore`. **Límite aceptado:** no hay detección automática de "archivo reescrito N veces por hora" |
| 56 | Demonio en `failed` con la app cerrada: nadie avisa en el escritorio (AF-18) | Baja | Alto | **Riesgo aceptado.** La CLI avisa por stderr en `gestor abrir`, `estado` y `parar` (3.10.16), y la tarjeta al abrir la app. `OnFailure=` con `notify-send` sigue como mejora opcional (riesgo 52) |
| 57 | Logout sin linger: las sesiones se cierran con `terminal_cerrada` en el próximo arranque del demonio y nadie lo avisa si la app no estaba abierta (AF-19) | Media | Bajo | **Riesgo aceptado.** El cierre es correcto (tramo en la UA) y queda en el desglose con su motivo. Mejora opcional: toast "Al volver se cerraron N sesiones" la primera vez que la app ve sesiones cerradas desde su último cierre |
| 58 | `exec sudo -i` sin marca "terminal perdida" (AF-20) | Baja | Bajo | **Riesgo aceptado**, documentado en riesgo 32 |
| 59 | Sin demonio, la sesión que mide la app no se pausa y cobra la cola hasta Parar o hasta cerrar la app (AF-23) | Media | Medio | **Riesgo aceptado (U6).** Excepción declarada (§1.2), acotada: tope de 12 h por tramo (`tope_sin_tracker`), chip `sin_vigilancia` en el desglose y aviso: línea "Trabajar mide sin vigilancia" mientras mide la app, alarma de cabecera si se espera el tracker y no corre, y toast al empezar Trabajar (3.10.14). Lo que queda: un olvido con la app abierta cobra **hasta 12 h** de cola, visible y marcado; se corrige con un ajuste negativo (3.10.17) |
| 60 | Un proceso que reescribe archivos con el usuario presente en **otra** ventana (la presencia es de todo el escritorio, U1) sigue sumando tiempo automático | Media | Bajo | Es la regla elegida: presencia = teclado o mouse en cualquier ventana. Lo mitigan las carpetas siempre ignoradas (riesgo 55) y la evidencia por minuto (`archivo_rel`) |
| 61 | Volumen de `archivos_evidencia` con un agente que escribe toda la noche sin presencia | Baja | Bajo | Una fila por proyecto y minuto con cambios (≤ 1440 por día y proyecto, y solo los minutos sin presencia o sin dato); los mismos filtros de carpetas ignoradas que `file_watch` |

Los riesgos 27–61 son de R8 (sesiones de Trabajar atadas a la terminal, 3.10). Los
que dicen **Riesgo aceptado** quedan así por decisión, con su mitigación parcial.

### 7.1 Runbook: apagar el tracker y volver atrás

Para cuando el demonio mide mal o molesta y hay que sacarlo **sin perder la base**.
Todo es local; ningún paso toca la red.

1. **Apagarlo.** Desde Ajustes → Tracking → Desactivar: la app escribe
   `daemon_enabled = false`, corre `disable --now` y cierra con `tracker_apagado` las
   sesiones de Trabajar del demonio que queden (3.8, 3.10.12). Si la app no abre,
   **primero la config y después systemd**, igual que Ajustes:
   ```bash
   # 1) con la app cerrada, poner "daemon_enabled": false en la sección "tracking" de
   #    ~/.config/gestor-proyectos/config.json. Sin esto, el SIGTERM solo cierra los
   #    tramos (tracker_detenido) y deja las sesiones de Trabajar en viva_pausada (DT7)
   systemctl --user disable --now gestor-trackerd   # relee la config: cierra tramos y sesiones (tracker_apagado)
   systemctl --user is-enabled gestor-trackerd       # → disabled
   sqlite3 ~/.local/share/gestor-proyectos/projects.db \
     "SELECT COUNT(*) FROM trabajar_sesiones WHERE dueno = 'demonio' AND estado <> 'cerrada'"   # → 0
   ```
   Si la última consulta no da 0 (el demonio no estaba corriendo), esas sesiones las
   cierra la versión nueva al abrirse (3.10.12); una versión vieja no las ve, solo sus
   tramos (paso 3). Las sesiones que mide la app (`dueno = app`) no son del tracker: se
   cortan con Parar o, con el tracker apagado, al cerrar la app (`cierre_app`, 3.10.7).
   Con la app abierta después, el tracker **no** se reactiva (3.8 paso 6).
2. **Backup antes de tocar datos.** Ajustes → Backup, o con todo cerrado:
   ```bash
   sqlite3 ~/.local/share/gestor-proyectos/projects.db \
     "VACUUM INTO '$HOME/gestor-antes-de-rollback.db'"
   ```
3. **Decidir qué hacer con lo medido.** Las versiones anteriores suman
   `duration_seconds` de **todas** las filas de `time_tracking_sessions`: también los
   tramos `source = 'terminal'` y los `work_button` con `terminal_token`. Donde se
   superponen con `file_watch` se contarían doble. Tres opciones:

   | Opción | Comando | Costo |
   |---|---|---|
   | Conservarlas | Nada | Totales inflados donde hubo superposición |
   | Borrar lo automático | `sqlite3 …/projects.db "DELETE FROM time_tracking_sessions WHERE source = 'file_watch'"` | Se pierde lo que midió el demonio por archivos (queda en el backup del paso 2); los tramos de terminal siguen |
   | Borrar también los tramos de terminal | `sqlite3 …/projects.db "DELETE FROM time_tracking_sessions WHERE source IN ('file_watch', 'terminal')"` | Se pierde todo lo que midió el demonio; queda lo que midió la app |

   Los **ajustes** (`ajustes_tiempo`) una versión vieja no los ve: el total de cada
   proyecto cambia exactamente en la suma de sus ajustes. Si importan, anotarlos antes:
   `SELECT project_id, SUM(segundos) FROM ajustes_tiempo GROUP BY project_id`.

4. **Sacar el servicio:** `./uninstall.sh` (borra unit, binarios y la CLI `gestor`;
   conserva la base). Sin `--rc` deja en los rc el bloque protegido, que ya no hace
   nada, y lo avisa; `./uninstall.sh --rc` lo saca también (3.10.16). Antes de R8 la CLI
   vivía fuera del repo y no se desinstalaba: para conservar `gestor`, copiar antes
   `~/.local/bin/gestor` (sin `gestor-trackerd` hace el `cd` sin medir, 3.10.16). A
   mano: `rm ~/.config/systemd/user/gestor-trackerd.service
   ~/.local/bin/gestor-trackerd && systemctl --user daemon-reload`. `trackerd.lock` y
   `restore.lock` quedan en `~/.local/share/gestor-proyectos/`: son archivos vacíos de
   `flock` que una versión vieja no usa, y se pueden borrar con todo cerrado.
5. **Instalar la versión anterior**, respetando el piso de abajo.

### 7.2 Downgrade

- **Piso: la primera versión que incluye F2** (se anota en el CHANGELOG como
  "piso de downgrade"). Esa versión ya trae el esquema 5 completo (F1 entra entera
  antes que F2), así que desde el piso todas leen la misma base; una versión de F1 sin
  F2 queda **por debajo** del piso. Las anteriores restauran con `fs::rename`
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
- **Columnas y tablas nuevas** quedan en la base: `last_activity_at`,
  `tracker_status` y, de R8, `trabajar_sesiones`, `trabajar_anclas`,
  `trabajar_evidencia`, `archivos_evidencia`, `ajustes_tiempo` (con sus dos triggers) y
  las columnas `terminal_token` y `motivo_cierre` de `time_tracking_sessions`. Las
  versiones viejas no las leen y no molestan. El `CHECK` de `motivo_cierre` solo
  restringe filas con `terminal_token`, y una versión vieja nunca escribe ni cierra filas
  así (solo cierra las que abrió ella), de modo que no choca con él. Conviene
  confirmarlo antes de bajar: con la versión vieja apuntando a una **copia** de la base
  (`XDG_DATA_HOME`), Trabajar y Parar sin errores. `user_version` tampoco molesta: las
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

**Sesiones de Trabajar (R8, 3.10)**
- **No** usar `ProtectProc`, `ProcSubset`, `PrivatePIDs` ni `PrivateDevices` en el unit.
- **No** loguear ni guardar `environ` ni el `cwd`; **no** guardar rutas absolutas en la evidencia.
- **No** contar el `mtime` del pty.
- **No** cerrar con 0 s, ni terminar por inactividad, una sesión cuya terminal no se detectó.
- **No** usar la regla de los 10 s en los tramos `terminal`.
- **No** cobrar la cola entre la última actividad y el cierre en ninguna sesión del demonio. (Salvedad declarada: los tramos `dueno = app` sin demonio la cobran hasta Parar, con tope de 12 h, §1.2 y §3, principio 4, U6.)
- **No** pasar de 12 h en un tramo `work_button` con token.
- **No** sumar al total los cambios de archivos sin presencia, ni en Trabajar ni en `file_watch`; **no** descartarlos sin registrarlos (U1).
- **No** armar SQL de sesiones ni de ajustes en la CLI.
- **No** adoptar como `dueno = app` una sesión del demonio sin un Trabajar explícito.
- **No** usar `SIGCHLD = SIG_IGN` (rompe el `waitpid` de los `git` de la app).
- **No** usar `GetSessionIdleTime` fuera de KDE.
- **No** editar ni borrar filas de `ajustes_tiempo`.
- **No** tocar los rc del usuario sin `--rc`.
## 9. Decisiones

### 9.1 Decididas

Las cuatro figuraban abiertas en la v2 aunque el cuerpo ya las daba por hechas
(T10). Llevan prefijo **Q** para no confundirse con los hallazgos D1–D15 del Anexo A.

| # | Pregunta | Decisión | Dónde |
|---|---|---|---|
| Q1 | Minutos de inactividad | **3**, configurable de 1 a 60, acotado por el demonio | R3; 3.4 (M14); 3.5; commits 35, 36, 67 |
| Q2 | Proyectos con carpeta `/home` (30, 31, 43) | No se vigilan (`carpeta_amplia`); la app los marca y el usuario los corrige. Qué hacer con estos tres en particular sigue en 9.2 (DX4) | R7; 3.5; 3.9; commits 37, 68 |
| Q3 | Un guardado aislado | **Reemplazada por DX1.** La v2 decía "no se guarda (0 s)"; queda como el caso `min_credit_seconds = 0` | 3.4 |
| Q4 | "Abrir" del Dashboard | Igual que "Trabajar" | 3.9; commit 9 |

### 9.2 Confirmadas por el usuario (2026-10-18)

El usuario aprobó la **propuesta** de cada fila. Para DX3 y DX4 eso no autoriza a
reescribir datos: el script del commit 72 pide un "sí" explícito caso por caso. Para
DX5, el commit de CI entra (y el total pasa de 73 a 74) recién cuando se autorice
el push a GitHub, porque la CI corre en remoto.

Orden en que se aplican: DX1 y DX2 entran con F4 (motor y atribución); DX3 y DX4,
con el commit 72, antes de cerrar F8.

| # | Pregunta | Opciones | Propuesta |
|---|---|---|---|
| DX1 | **Margen por guardado:** ¿cuánto acredita un guardado antes del primer cambio de una sesión? (definición en 3.4) | (a) **60 s**, con las cotas de 3.4; (b) 0 s, como la v2: un guardado suelto no cuenta y dos guardados a 2 min valen 2 min; (c) otro valor entre 0 y 300 | **(a)**, configurable en Ajustes. Es la única excepción a "nunca inventar tiempo" y está acotada |
| DX2 | **Tope de carpetas por proyecto** (`max_dirs_per_project`). Con **5.000**, Moodle Potenciarg (**55**, ~27.100 carpetas respetando `.gitignore`, 3.5) queda **no medible** | (a) 5.000 global, y el 55 se arregla con un `.gestorignore` que excluya el código de Moodle que no se toca; (b) 30.000: el 55 entra, y un paraguas como `~/2025` probablemente también, usando más del límite de inotify compartido (~52.400 de 524.288 con todos los proyectos, 2.3); (c) 5.000 por defecto más un tope **por proyecto** editable en la tarjeta: cuesta +1–2 commits, una columna nueva en `projects` y un paso de migración; (d) sin tope, solo el aviso de `limite_inotify` | **(a)**: default único de 5.000, y para el 55 se recomienda `.gestorignore`. (c) queda como opción, con su costo declarado, si el `.gestorignore` no alcanza. (d) no: un paraguas solo podría agotar el límite de VSCode y JetBrains |
| DX3 | **Datos históricos:** las sesiones de más de 12 h (el 69 % del tiempo registrado) y las huérfanas abiertas (cifras en 2.1) | Para las > 12 h: (a) **marcar** = listarlas en el CHANGELOG, **sin columna nueva**, y seguir contándolas; (b) **recortar** cada una a un tope (por ejemplo 8 h); (c) **dejarlas** como están. Para las huérfanas: (a) cerrarlas en 0 s (lo hace el commit 19: hoy ya suman 0, el total no cambia); (b) borrarlas | > 12 h: **(a) marcar**, y que el usuario decida caso por caso con la lista del diff (commit 24). Huérfanas: **(a)**, ya incluido en el plan. Lo que reescriba filas, (b) o el borrado, lo aplica el script del commit 72, con backup previo y **solo** con un "sí" explícito del usuario |
| DX4 | **Proyectos con carpeta que no se puede medir:** 7 con carpeta inexistente y 3 con carpeta amplia (lista en 3.5) | (a) dejarlos marcados con el aviso y que el usuario los edite cuando quiera; (b) corregir las rutas ahora (por ejemplo, `/home/2025/...` → `~/2025/...` si es un error de tipeo); (c) mandarlos a la papelera | **(a)**, y revisar juntos si 19–21 son `~/2025` mal escrito: es probable, pero no se toca sin confirmar. (b) y (c), si se eligen, van por el script del commit 72 con las mismas garantías |
| DX5 | **CI en GitHub Actions** | (a) `.github/workflows/check.yml` que corra `scripts/check.sh` sin `--e2e` en `ubuntu-latest`, agregado como commit aparte al final de F2 (los E2E, E16 y E22 quedan locales); (b) no tener CI | **(a)**, con autorización aparte por ser remoto. Si se aprueba, entra como commit 33 (el primero después de F2), se renumera desde ahí y el total pasa de 73 a 74 en todo el documento |

### 9.3 Decididas para R8 (2026-10-19 y 2026-10-20)

Las decisiones de fondo de R8 (D-R8-1..5 y 1a–5a) están en §1.1. Las siete preguntas que
el plan R8 no podía decidir solo las resolvió el usuario el 2026-10-20 (U1–U7, 9.4) y están
aplicadas en todo el documento:

| # | Pregunta | Decisión | Dónde |
|---|---|---|---|
| U1 | ¿La regla de presencia vale también para el tiempo automático por archivos (`file_watch`)? | **Sí.** Presencia = teclado o mouse en **cualquier** ventana del escritorio (idle de la sesión gráfica), para Trabajar **y** para `file_watch`. Variante (c): los cambios en disco sin presencia **no suman**, pero quedan registrados por minuto (con el segundo exacto) y el desglose los muestra como tramos **`solo_archivos`** ("solo archivos, sin presencia") con su duración; el usuario los convierte en tiempo cobrable con **"Cargar como ajuste"**, que precarga desde, hasta y segundos y pide la nota | 3.4, 3.10.1, 3.10.4, 3.10.8, 3.10.14, 3.10.17; commits 15, 48, 50, 69, 70; E29; paso 31; riesgos 60 y 61; invariantes 14 y 20 |
| U2 | Sin dato de presencia, ¿los archivos cuentan o no? | **Cuentan**, marcados `presencia_desconocida` (bit 8), en Trabajar y en `file_watch`. Es una **excepción declarada al principio 4** | 3.10.4, 3.10.14; §3, principio 4 (excepción 2); riesgo 46 |
| U3 | ¿Un guardado en un subproyecto sostiene la sesión de Trabajar del paraguas? | **Sí**, marcado "simultáneo con" el hijo | 3.10.8; riesgo 41 |
| U4 | ¿Qué pasa al cerrar la app con una sesión abierta? | Tracker andando → la sesión **sigue** (la tiene el demonio). Tracker caído o sin instalar → `esperando_demonio`, se retoma cuando vuelve y ese rato **no** se mide. Tracker apagado en Ajustes (o Windows) → termina como hoy con `cierre_app`, y Trabajar en otro proyecto con `cambio_de_proyecto`. El binario de desarrollo (`target/`) sigue en "adopción imposible", como lo planteaba la pregunta | §1.2, 3.10.2, 3.10.6, 3.10.7, 3.10.11, 3.10.13, 3.10.14, 3.10.15; paso 40 |
| U5 | Con el tracker apagado en Ajustes, ¿`gestor abrir X` mide? | **No.** Hace el `cd` (la terminal queda en la carpeta) sin medir, con un aviso corto: "Tracker apagado en Ajustes: no se mide." Código 3. El botón Trabajar sí mide con la app (U4); la CLI no, porque sin demonio nadie vigila su terminal y no tiene una app que mida por ella | 3.10.16; paso 37 |
| U6 | Sin demonio, ¿se acota lo que mide la app? | **Sí.** La app mide hasta Parar con un **tope de 12 h** por tramo (motivo nuevo `tope_sin_tracker`), el tramo queda marcado **`sin_vigilancia`** en el desglose (derivado del `source`, sin columna) y la app lo **avisa**: alarma de cabecera solo si se espera el tracker y no corre, línea "Trabajar mide sin vigilancia" mientras mide la app y toast al empezar Trabajar (3.10.14). Riesgo aceptado 59 | §1.2, 3.9, 3.10.1, 3.10.2, 3.10.7, 3.10.11, 3.10.13, 3.10.14, 3.10.15; commits 17, 19, 66, 69; §3, principio 4 (excepción 3); riesgo 59; invariante 21 |
| U7 | ¿Una etapa o dos? | **Una sola etapa**: 73 commits (74 con DX5). La variante de dos etapas (una F9 con ajustes y simultaneidad, 74 commits) se descartó | Sección 4; 4.11; 3.10.17; pasos 33 y 34; invariante 18 |

**Detalles fijados al aplicar las decisiones** (aceptados el 2026-10-20):

- **Agrupación de `solo_archivos`:** cambios a menos de 3 min (el `idle` de `file_watch`) forman un tramo, también dentro de una sesión de Trabajar, para no unir huecos largos sin evidencia (3.10.4).
- **Dos `sin_vigilancia`:** la marca del desglose (U6, derivada del `source`) y el motivo de tramo de `file_watch` ("la carpeta dejó de vigilarse") conviven, en dominios distintos y con textos distintos (3.10.13, 3.10.15).
- **Aviso de U6:** la primera aplicación ponía solo el toast al empezar Trabajar; la integración agregó un indicador persistente con cualquier `demonio_vivo()` falso, que salía también con el tracker apagado a propósito, en desarrollo y en todo F1–F5, y decía "el tiempo se mide sin vigilancia" cuando el tiempo automático no se mide. La corrección final lo acota (3.10.14): alarma solo si se espera el tracker y no responde en dos sondeos seguidos, línea "Trabajar mide sin vigilancia" con una sesión de la app, y nada en el resto.
- **`tope_sin_tracker`** es un motivo nuevo, de tramo y de sesión: hace falta para que el `CHECK` exija motivo al cortar en el tope (3.10.15).
- **Desarrollo (`target/`)** sigue como "adopción imposible" (U4); sacar esa cláusula alcanzaría para tratarlo como "no instalado" (3.10.7).

**Nombres que traen las decisiones:** el motivo `tope_sin_tracker` (de tramo y de
sesión, U6), la marca `sin_vigilancia` del desglose (U6) y la tabla
`archivos_evidencia` (U1). La marca `solo_archivos` nombra el tramo derivado de
archivos **sin** presencia; el chip informativo de "tramo cobrado sin tipeo" se llama
"sin tipeo".

### 9.4 Identificadores

Para que ningún ID se lea como otro:

| Prefijo | Qué es | Dónde |
|---|---|---|
| P1–P19 | Problemas de partida | 2.2 |
| R1–R8, R8-CLI | Requisitos | §1 |
| D-R8-1..5, 1a–5a | Decisiones de fondo de R8 (2026-10-19) | §1.1 |
| **U1–U7** | Decisiones del usuario del 2026-10-20 | 9.3 |
| Q1–Q4, DX1–DX5 | Decisiones de la v2 y las confirmadas el 2026-10-18 | 9.1, 9.2 |
| DT1–DT17 | Decisiones técnicas de R8 | 3.10 |
| B1–B9, M1–M16, D1–D15, O1–O5, T1–T12 | Hallazgos de la auditoría de la v2 | Anexo A, v3 |
| A1–A3 y el resto de la sección v3.1 | Correcciones de la verificación de la v3 | Anexo A, v3.1 |
| **AF-n**, AF-P2 | Escenario n de la auditoría de facturación del plan R8 (2026-10-19), y su propuesta P2 (ajustes y pausas reclamables) | 3.10.4, 3.10.6, 3.10.7, 3.10.14, 3.10.16, §7 |

Las decisiones del 2026-10-20 se llamaban D1–D7 hasta la corrección final y chocaban
con los hallazgos D1–D15 (por ejemplo, "D6" era a la vez "errores propagados" y "tope
de 12 h"). Los escenarios de la auditoría de facturación se citaban como F2, F18… y se
confundían con las fases F0–F8. El informe de esa auditoría no está versionado: el
texto copia lo que usa de cada escenario.

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
- Bundler de Tauri 2 (qué binarios entran en el `.deb`, a verificar en el commit 34): https://v2.tauri.app/distribute/debian/

**Procesos, terminales y escritorio (R8)**
- `proc_pid_stat(5)` (campos `tty_nr`, `tpgid`, `starttime`; `comm` entre paréntesis): https://man7.org/linux/man-pages/man5/proc_pid_stat.5.html
- `proc_pid_environ(5)` (refleja solo el entorno del `execve`): https://man7.org/linux/man-pages/man5/proc_pid_environ.5.html
- `script(1)` (la "terminal" de los E2E): https://man7.org/linux/man-pages/man1/script.1.html
- logind por D-Bus (`LockedHint`, sesión `Display`): https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.login1.html
- `zbus` 5 (cliente D-Bus, ya en el `Cargo.lock`): https://docs.rs/zbus/5

**Repo** (relevado sobre `main@c8b55ae`; lo de R8, sobre `feat/tracker-f0@83c48d4`, y además `docs/cli.md:7-8`, `~/.local/bin/gestor` y `~/.zshrc:112-133` y `~/.bashrc:141-162` (bloques entre marcas, 2026-10-20), que no están en el repo): `scripts/release.sh:2` (`set -euo pipefail`), `:302-304` (`cargo build --release` + `cp`); `install.sh:6` (`set -e`), `:14-17` (exige el binario ya compilado), `:27` (`cp`), `:46` (`Exec=` fijo); `uninstall.sh:37-41` (conserva la base); `start-app.sh:21` (`pkill -f "gestor-proyectos"`); `src-tauri/Cargo.toml:19` (rusqlite solo con `bundled`), `:31-32` (`[dev-dependencies]` con `tempfile`); `src-tauri/tauri.conf.json:7` (`beforeBuildCommand`), `:57-59` (`bundle`); no existe `.github/`.

---
## 11. Historial de revisión

- **v2:** la v1 pasó una revisión adversarial con 25 hallazgos (8 críticos), todos incorporados.
- **v3:** la v2 pasó una auditoría con 59 hallazgos (9 bloqueantes), todos con fila y ubicación.
- **v3.1:** la v3 se verificó contra el código y la documentación de systemd y Tauri 2; 3 hallazgos altos (kill switch antes de `READY`, E2E en verde vacío, restore inaccesible desde los tests), 16 medios y 10 bajos corregidos, más un recorte de redundancias.

- **v3.2 (2026-10-20):** integra R8 (la sesión de "Trabajar" atada a la terminal) y R8-CLI, revisados por cuatro auditorías y dos verificaciones, con las decisiones del usuario D-R8-1..5, 1a–5a y U1–U7; 17 commits nuevos (56 → 73) y estimación de 41,5–52,5 días. Los ajustes manuales entran al alcance; el cliente queda fuera.

El detalle de cada ronda, con dónde quedó cada hallazgo, está en el **Anexo A**.

---
## Anexo A — Historial de revisión

En todo el anexo los números de commit están en la numeración v3.2 (la equivalencia
con la v3.1 está en la sección v3.2, al final), salvo donde dice "de la v2".

### v2 (2026-10-18): revisión adversarial de la v1

Se incorporan los 25 hallazgos. La columna "Dónde se resolvió (v2)" cita commits de
la v2 y lo aclara en cada caso; la columna "Hoy" usa la numeración v3.2.

| Sev. | Hallazgo | Dónde se resolvió (v2) | Hoy (v3.2) |
|---|---|---|---|
| Crítico | La unión contaba huérfanas viejas "hasta ahora" | 3.3 (tabla de tramos); F1 ordena la recuperación antes de la unión | 3.3; commits 19 → 20 |
| Crítico | "Mismos números que antes" era falso (regla de 10 s, suspensión) | 3.3: tramo = `started_at + duration_seconds` para filas no `file_watch` | 3.2 (regla única, M8); commits 8, 20 |
| Crítico | El corte dependía del `tick` y una suspensión extendía la sesión | 3.4: corte también en `cambio`; reloj de pared con protección contra retrocesos | 3.4 (+ `reanudado`); commit 36 |
| Crítico | La recuperación al arrancar corría también en la segunda instancia | 3.8: va en `.setup()` y solo toca latidos de más de 2 min (P11) | Commits 4 (D15) y 19 |
| Crítico | El demonio con `ConfigManager` podía borrar la config | 3.5: lector propio de solo lectura (P12) | Commit 47 |
| Crítico | WAL antes del restore nuevo dejaba un commit que corrompía | F2: restore primero (commit 10 de la v2) | Commits 27 → 30 |
| Crítico | Verificar `"wal"` rompía los tests `:memory:` | 3.6: se acepta `"memory"` | Commit 30 |
| Crítico | El restore ignoraba la conexión viva, los esquemas viejos y los ids | 3.6: secuencia con salida de la app y migración al volver a abrir | 3.6 (migra **antes** de relanzar, B7); commits 27, 57 |
| Alto | Solo las transacciones de tracking pasaban a `IMMEDIATE` | 3.6: las 6 (P13) | Commit 28 (justificación corregida, D9) |
| Alto | Las actualizaciones fallaban (`ETXTBSY`, sin reinicio, límite de reintentos) | 3.8: temporal + `mv`, `try-restart`, `reset-failed` | Commit 59; M15 en commit 54 |
| Alto | `ExecStart` asumía `~/.local/bin` | 3.7: ruta hermana de `current_exe`; alcance declarado | Reemplazado por B9: solo instala desde `BIN_DIR`; commit 54 |
| Alto | Formato de fechas sin definir | 3.3: `time.rs` único, consulta de rango y `.earliest()` | Commit 11 (medianoche inexistente corregida, D13) |
| Alto | Carpetas nuevas, renombradas, movidas o borradas | 3.5 + E6/E7 | Commit 45; E6, E7 |
| Alto | El desborde re-recorría 52.000 directorios | 3.5: solo se registra | 3.5: rescate diferido (M10); commit 46 |
| Alto | E2E inexpresables o inestables | 3.5 `GESTOR_TRACKERD_TEST_TIMINGS`; 5.2 sondeo con plazo | 5.2 (+ centinelas, T3); commits 50, 51 |
| Medio | `add_project_time` también escribía el contador | 3.2 + commit 9 de la v2 | Commit 25 (D2) |
| Medio | El segundero del cliente pasaba del último cambio | 3.9: `file_watch` muestra `last_activity_at − started_at` | Commit 65 |
| Medio | Filas de 0 s y estadísticas sin definir | 3.2 y 3.3 | 3.2 (DX1), 3.3; commits 20, 21 |
| Medio | Papelera y purga a mitad de sesión | 3.4 (errores no fatales) y 3.5 (papelera) | 3.4 (0 filas, M12); commits 43, 48; E11 |
| Medio | `VACUUM INTO` bajo WAL dejaba `-wal`/`-shm` | 3.6 + commit 13 de la v2 | Commit 31 |
| Medio | Atribución por prefijo de texto | 3.4: `Path::starts_with` por componentes | Commit 37 (más específico, DX2) |
| Medio | Orden de arranque del demonio sin definir | 3.4 | 3.4 (READY antes del recorrido, M13); commit 49 |
| Bajo | Reexportaciones y nombre de la lib | 3.1 + commit 15 de la v2 | Commit 33 |
| Bajo | Detección de producción y errores de `systemctl` | 3.8 | Commit 54 |
| Bajo | F0 y F1 reescribían lo mismo | "Hoy" pasa a F1 (commit 9 de la v2) | Commit 21 |
| Bajo | `active_today` en UTC | F1 (commit 9 de la v2) y `tracker_status.idle_minutes` | Commit 21 (hora local, con tiempo trabajado) |

### v3 (2026-10-18): auditoría de la v2

59 hallazgos de la auditoría: 2 decisiones por defecto, 9
bloqueantes, 16 de medición, 15 de datos, 5 de operación y 12 de pruebas y entrega.
Todos tienen fila; los que se resolvieron distinto de lo pedido lo dicen.

**Decisiones tomadas por defecto (confirmadas el 2026-10-18, 9.2):**

| ID | Dónde quedó | Resumen |
|---|---|---|
| DX1 | 3 (Principios, 4); 3.2 "Guardados espaciados" (referencia); 3.4 margen y `frontera` (definición única); 3.5 rango; 3.9; 9.2; commits 35, 36, 43, 47, 67; E1b | 60 s antes del primer cambio, acotado por la sesión anterior, la suspensión y el corte; 0 = v2. Reemplaza la Q3 vieja (9.1) |
| DX2 | 3.4 atribución al más específico; 3.5 tope `max_dirs_per_project`; 3.9 avisos; 9.2 con el dato del proyecto 55; commits 37, 38, 46, 67, 68; E19 | Paraguas vigilados con tope (default 5.000) y conteo de referencias. Con 5.000 el 55 (~27.100 carpetas) no se mide: opciones en 9.2 |

**Bloqueantes:**

| ID | Dónde quedó | Resumen |
|---|---|---|
| B1 | 2.2 P14; 3 (Principios, 5); 3.2 "Esquema versionado" (tabla única de estados); 3.4 arranque; commits 10, 43, 49; E15 | `user_version` + `BEGIN IMMEDIATE`; solo se ignora `duplicate column name`; el demonio no migra: espera si está atrasada o sin WAL, ocioso si es futura. La v3 no probaba la espera: la v3.1 agrega en el commit 43 "`ESQUEMA_ACTUAL − 1` → `esperando_migracion`, y `activo` al migrar desde otra conexión" |
| B2 | 2.2 P15; 3.9; commit 23 (F1, no F7) | `TimeTracker` carga stats siempre |
| B3 | 2.2 P2; 3.6; commit 3 | 0 filas en el primer `UPDATE` → rollback, sin sumar al contador |
| B4 | 3.1 `default-run` y empaquetado; 3.7; 3.8; §1 "Fuera de alcance"; commit 34 | `default-run` en el mismo commit del `[[bin]]`; tarea `dpkg -c`; el diseño no depende del resultado |
| B5 | 3.4 kill switch (sale 0); 3.8 (config primero, no reactiva un `disable` manual); 3.9; 7.1 runbook; 7.2; 8; F6 entera junta; commits 36, 49, 54, 55, 66; E20; manual 12–13 | Nunca en producción sin forma de apagarlo; toggle mínimo en F6 |
| B6 | 4 (orden y 4.10); 5.5; commits 1 y 2 (a), 13 (b), 35 (c), 50 y 51 (d) | `check.sh` y `cargo fmt` en F0; `tracker_status` en F1; config `tracking` en F3; E2E desde F5b. Total recontado: 55 en la v3, 56 en la v3.1 (commit 72 nuevo, DX3/DX4) |
| B7 | 3.6 restore pasos 1–9; 3.8; commits 27, 57; E15; manual 15 | La app migra lo restaurado con el lock tomado y hace `start` antes de `app.exit(0)` |
| B8 | 3.1 `trackerd_lock_file`; 3.4 guardas; 3.7 rutas explícitas; 5.2 aislamiento; commit 34; E14 | Lock en `<data_dir>/trackerd.lock`; debug exige `--data-dir`; timings + base real → se niega |
| B9 | 3.7; 3.8 pasos 3–5; 8; commit 54 | Unit comparado por contenido completo; instala solo desde `BIN_DIR`; rutas con caracteres especiales rechazadas |

**Medición:**

| ID | Dónde quedó | Resumen |
|---|---|---|
| M1 | = DX1 | Ver DX1 |
| M2 | 3.5 registro con conteo; 8; commit 45; E19 | `watch`/`unwatch` solo en transiciones 0↔1 |
| M3 | 3.5 filtro por evento; 7 riesgo 1; commits 39, 46; E5, E18 | Matchers de `.gitignore` en cada evento, con recarga en caliente |
| M4 | 3.5 ignorados y carpetas sincronizadas; 3.9; commits 38, 39, 68 | Syncthing, Dropbox, Nextcloud, `.~lock.*#`, `*_BACKUP.md`; proyectos sincronizados avisados |
| M5 | 3.3 fila `activa`; 3.9 contador; 8; commits 20, 64, 65 | Contador de "Trabajar" con `elapsed_seconds` del `Instant`, nunca `Date.now()` |
| M6 | 3.3 firma con `activa`, tramos sin `tracker_status`; commits 20, 21 | Orden de locks `ActiveSession → Database` |
| M7 | 3.2 "Cómo se cierra cada sesión"; 3.4 cierres coherentes; commits 14, 19, 36, 43; E8 | `ended_at` y `duration_seconds` coherentes; `last_activity_at` al abrir |
| M8 | 2.1; 3.2 regla única; commit 8 | Una sola `duracion_a_guardar` para todos los cierres |
| M9 | 3.4 latido de sesión; 3.9 texto honesto; commits 43, 65 | Máx. una escritura cada 5 s con actividad, ninguna sin actividad |
| M10 | 3.5 desborde; 7 riesgo 5; 8; commit 46 | Rescate diferido tras 30 s de calma, máx. uno cada 10 min |
| M11 | 3.5 `mountinfo` y `en_red`; 8; commits 38, 46 | Resuelto distinto: solo se rechaza lo remoto; `fuseblk` local (14 proyectos) se mide |
| M12 | 3.4 0 filas → soltar; 3.5 `data_version`; commits 36, 43, 48; E11, E21 | Papelera vista en ≤ 5 s |
| M13 | 3.4 arranque y watchdog; 3.7; 8; commit 49 | `READY` antes del recorrido; watchdog solo desde el bucle; unitario del watchdog (5.2) |
| M14 | 3.4 inactividad efectiva; 3.5 rangos; commits 36, 47, 67; E17 | `idle_minutes` acotado 1–60 y publicado en caliente |
| M15 | 3.7 `StartLimitIntervalSec=0` + `RestartSteps`; 3.8; commit 54 | Nunca se rinde; alternativa para systemd < 254 con `reset-failed` al abrir |
| M16 | 3.4 reloj; commit 36 | Retroceso > 5 min cierra y anota `last_error`; solo UTC |

**Datos y base:**

| ID | Dónde quedó | Resumen |
|---|---|---|
| D1 | 2.2 P17; 3.1 `paths.rs`; commit 26 | Rutas compartidas; se borra `backup::live_db_path` |
| D2 | 3.2 lista con archivo:línea; commits 21 (9a) y 25 (9b) | Lectura por unión primero, sacar el contador después |
| D3 | 2.1 "La base real hoy"; 5.4; 6; 9.2 DX3; commits 24, 72, 73 | Diff con datos reales sobre copia; > 12 h listadas sin borrar; cada diferencia en el CHANGELOG |
| D4 | 2.2 P3; 3.3 semana; commits 11, 21 | 7 fechas locales incluido hoy |
| D5 | 3.3 excepción `deleted_at`; commit 11 | Excluido de `time.rs` y documentado |
| D6 | 2.2 P18; 3.3; commit 21 | Errores propagados en vez de `.unwrap_or(0)` |
| D7 | 2.2 P16; 3.6; commit 5 | `self.conn()` bloqueante |
| D8 | 2.3; 3.6 WAL y `busy_timeout`; commit 30 | Si no queda en `wal`, error visible |
| D9 | 2.2 P13 corregido; 3.6; commit 28 | Justificación corregida; `IMMEDIATE` en las 6 como regla |
| D10 | 7.2 (3.6 la referencia); commits 32, 73 | La versión vieja descarta `tracking` al guardar: documentado |
| D11 | 2.2 P19; 3.1; 3.6; 3.7 `UMask=0077`; commits 29, 30, 54 | `0700`/`0600` para carpeta, base, `-wal`/`-shm` y backups |
| D12 | 3.6 "Backups del home"; 3.9 Ajustes → Backups; commits 32 (`ARQUITECTURA.md`) y 67 (texto en Ajustes, con Vitest) | Excluir `projects.db*` de rsync/Déjà Dup; respaldar la carpeta de backups. La v3 citaba Ajustes sin commit que lo hiciera |
| D13 | 3.3 tests de zona; 5.1; commit 11 | `rango_dia` genérico, `chrono-tz` dev-dep, sin tocar `TZ` |
| D14 | 3.2 métodos con instante explícito; commit 14 | `abrir_sesion`, `latir`, `cerrar_sesion` con `bool` |
| D15 | 2.2 P11; 3.8; commit 4 (F0); manual 1 | Base, config y auto-backup en `.setup()` |

**Operación y seguridad:**

| ID | Dónde quedó | Resumen |
|---|---|---|
| O1 | 3.7 aislamiento con `PrivateUsers=yes` y tabla de qué deja leer/escribir; 3.8 paso 8; 8; commits 54, 56; E16; manual 18 | Resuelto: sin `ProtectHome`; fallback sin user namespaces con criterio de detección concreto (`Result=exit-code` + `ExecMainStatus=226`) y test con systemctl falso en el commit 54 |
| O2 | 3.7 logs; 8; commit 48 | Sin rutas por evento; línea agregada cada 10 min; límite de tasa |
| O3 | 3.5 tope; 3.7 `MemoryMax`; 3.9 uso de inotify; 6 `MemoryCurrent`; commits 46, 67, 71 | Uso visible en Ajustes y medido en "listo" |
| O4 | 3.8 `uninstall.sh`; commit 59 | Resuelto distinto: **no** se borra `tracker_status` (la base se conserva a propósito y la fila vieja es inofensiva) |
| O5 | 3.7 `UNIT_DIR`; 3.8 variables de los scripts; 5.2 aislamiento; commits 54, 59 | `PREFIX`/`BIN_DIR`/`UNIT_DIR`/`SYSTEMCTL`; `.desktop` sin ruta fija; `FragmentPath` verificado |

**Pruebas y entrega:**

| ID | Dónde quedó | Resumen |
|---|---|---|
| T1 | 3.6 "Dónde vive cada paso"; 5.2 E15; commits 27, 33, 51 | Restore con el demonio vivo sin systemd: aborta con el lock; luego restore de esquema viejo y relanzamiento. E15 llama a `db::restore` de la lib: la v3 lo hacía llamar a `restore_backup_to`, que es privada de `backup/` y no se puede usar desde `tests/` |
| T2 | 5.2 E16; commit 56 | `systemd-run --user` transitorio con las propiedades del generador |
| T3 | 5.2 "Negativos con centinela"; commit 50 | Esperar `scan_completed_at`, negativo, centinela, afirmar |
| T4 | 5.2 E7, E17–E21; 5.3 paso 16; 6.1 (P3 en commit 21) | Todos los escenarios pedidos tienen test o paso manual. La v3 omitía "mover `A/sub` → `B/sub`": la v3.1 lo suma a E7 |
| T5 | 5.2 tolerancias; "Lo que no se automatiza" | ±1 s; E10 de 20 s con 4 escritores; watchdog cubierto con unitario y documentado |
| T6 | 5.3 (instalación con `install.sh`, pasos 3 y 17); commit 59 | Nada del recorrido crea commits ni tags |
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
| A1 | Kill switch al arrancar: salir con 0 antes de `READY=1` deja la unidad `failed` (`Result=protocol`) y, con `Restart=on-failure` + `StartLimitIntervalSec=0`, en bucle. Ahora la config se lee antes de la base y el demonio manda `READY=1` → `STOPPING=1` → salida 0 (3.4, 3.7); tests en el commit 49 y E22 nuevo (commit 56) |
| A2 | E2E en verde vacío: todos los tests de `tests/e2e_trackerd.rs` llevan `#[ignore]` y `check.sh --e2e` falla si corren menos de `E2E_ESPERADOS` (5.5, commits 50 y 51) |
| A3 | E15 no podía llamar a `restore_backup_to` (privada, `backup/mod.rs:427`): el núcleo pasa a `db::restore::{tomar_lock, restaurar_desde}` en el commit 27 y entra en la lib en el 33; `backup/` queda como envoltorio (3.1, 3.6, 4.10, E15) |

**Medios y parciales**

| ID | Qué se corrigió |
|---|---|
| B1 | Test "`ESQUEMA_ACTUAL − 1` → `esperando_migracion`, y `activo` al migrar desde otra conexión" en el commit 43; matriz 6.1 y fila B1 del Anexo A corregidas |
| T4 | E7 suma "mover `A/sub` → `B/sub` y escribir → cuenta solo B; A suelta sus vigilancias"; fila T4 corregida |
| O1 | Criterio de detección sin user namespaces: `Result=exit-code` + `ExecMainStatus=226` (3.7, 3.8 paso 8); test con systemctl falso en el commit 54; fila O1 pasa a "Resuelto" |
| O2 | Unitario del limitador de logs en el commit 48; E2 y E5 afirman que el stderr no contiene la ruta del `tempdir` |
| D3/DX3 | "Marcar" = lista en el CHANGELOG, sin columna (9.2); commit 72 nuevo (`scripts/datos-historicos.sh`, backup previo y solo con "sí" explícito); 5.4 dice "cuatro partes"; total 55 → 56 |
| D12 | El texto de Ajustes → Backups entra en el commit 67, con Vitest (3.6, 3.9) |
| M13 | El commit 49 lista en su columna de pruebas el unitario "`WATCHDOG=1` solo desde el bucle" |
| Orden de arranque | La config se lee antes de abrir la base (paso 4 de 3.4) y el kill switch se respeta mientras espera; `journal_mode ≠ wal` también hace esperar, con test en el commit 43 |
| 7.2 contra 3.8 | Al volver a la versión nueva hay dos ramas: sin unit (tras `uninstall.sh`) se instala; con el unit `disabled`, no se reactiva |
| `install.sh:40` | Verificado: el `.desktop` usa `<< 'EOF'`; el commit 59 lo pasa a `<< EOF` para expandir `$BIN_DIR`, con test |
| `trash.rs` | Verificado: `restore_project` (`:56-66`) y `purge_project` (`:133-143`) validan la guarda fuera de la transacción; el commit 28 la mete adentro de la `IMMEDIATE` (2.2 P13, 3.6) |
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
| Commit 34 | El test sostiene el `flock` y el binario debe salir con 75 |
| `start-app.sh` | Verificado: también `pgrep -f "gestor-proyectos"` (`:19`) alcanza al demonio; el commit 53 acota los dos |
| Regex | El patrón de `pkill` sale de la tabla a un bloque de código, sin `\|` |
| Dependencias | Commits 37 y 39: "—" → 33 |
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

### v3.2 (2026-10-20): R8 y R8-CLI

**Origen.** El usuario pidió el 2026-10-19 que la sesión de "Trabajar" quede atada a
la terminal (R8) y que elegir un proyecto en la CLI `gestor` sea Trabajar (R8-CLI). El
plan R8 pasó por dos versiones: la primera (+9 commits de R8 y +3 de R8-CLI) se revisó
con cuatro auditorías (código y sistema, consistencia, facturación y pruebas); la
segunda incorporó esas auditorías, las decisiones 1a–5a y dos verificaciones
(cobertura y coherencia), y el 2026-10-20 recibió las decisiones U1–U7. La v3.2 es el
resultado integrado en este documento. Los relevamientos previos de R8 (impacto en
el código y terminales) no están versionados: los datos que el diseño toma de ellos
están copiados en el texto (tabla de flags y zombis de konsole en 3.10.3; ejemplo de la
suspensión en 3.10.1 y 3.10.8).

**Decisiones del usuario:** D-R8-1..4 y R8-CLI (D-R8-5) del 2026-10-19; 1a–5a
(presencia, cola, cwd, simultáneo, ajustes); U1–U7 del 2026-10-20 (9.3; hasta la corrección final se llamaban D1–D7, 9.4). Al
integrar, el aviso de U6 quedó doble (indicador persistente y toast); la corrección
final acotó el indicador (3.10.14, 9.3).

**Enmiendas al vocabulario de R8** (el texto las cita así):

| Enmienda | Qué fija |
|---|---|
| Enmienda 1 | Motivos de sesión `cambio_de_proyecto` y `cierre_app` (solo para sesiones `dueno = app`, nunca para `dueno = demonio`) y `huerfana`; motivos de tramo `inactividad`, `suspension`, `reloj`, `tracker_detenido` y `sin_vigilancia`. Se quita `fin_de_sesion_escritorio` |
| Enmienda 2 | `cambio_de_proyecto` también vale con `dueno = demonio` cuando la misma terminal pasa a otro proyecto con `gestor abrir Y` (el ancla deja de ser de X). `cierre_app` sigue siendo solo `dueno = app` |

**Correcciones aprobadas en la revisión de R8** (el texto las cita como "corrección
aprobada en la revisión de R8"):

| Tema | Corrección | Dónde |
|---|---|---|
| Terminal no detectada | `terminal_no_detectada` **pausa**: no termina por inactividad ni por cierre de la app; sale con Parar o si aparece la terminal | 3.10.2 |
| CLI sin demonio | Crea la sesión igual, en `esperando_demonio`; el tiempo cuenta desde la adopción. No hay "fallback sin medir" | 3.10.13, 3.10.16 |
| Kill switch | El demonio relee la config al recibir SIGTERM y cierra con `tracker_apagado`; si no llegó, la app cierra igual. Nunca se adopta en silencio como `dueno = app` | 3.10.12 |
| Convivencia | La convivencia de proyectos vale solo con `dueno = demonio`; la app mide un proyecto a la vez | 3.10.6, 3.10.7 |
| tmux y screen | Un ancla dentro de un multiplexor no sostiene la sesión (pausa en `terminal_no_detectada`, se corta con `gestor parar`), con aviso en la CLI y E36 | 3.10.16 |
| Papelera | Cierra las sesiones vivas con `proyecto_a_papelera` | 3.10.9 |
| Restore | Corta las sesiones vivas con aviso que las lista (`restauracion`) | 3.10.9 |
| `demonio_vivo()` | Una sola función; durante un restore se rechaza Trabajar (`restore.lock`) | 3.10.7 |
| Base de la CLI | El subcomando resuelve la base con `--data-dir` / `GESTOR_DB`, igual que la app | 3.10.16 |
| Transacciones | `end_tracking_session` con `transaction_with_behavior(Immediate)` | 3.10.7 |
| Herencia de la marca | `remove_var("GESTOR_SESION")` antes de `logging::init` | 3.10.3 |
| `$PPID` | Se corrigió la justificación falsa de la función `gestor` | 3.10.16 |
| `/proc/<pid>/stat` | Se parsea después del último `)` | 3.10.3 |
| Zombis de konsole | `try_terminal_fallback` pasa el `Child` a un hilo que hace `wait` | 3.10.3 |

**Qué corrigieron las verificaciones del plan R8 v2** (por tema; el detalle por
hallazgo quedó fuera del repo):

| Tema | Qué quedó |
|---|---|
| Dueño de la sesión | `dueno` es el dueño **actual** y cambia solo sin tramo abierto; la invariante 10 mira solo tramos abiertos (3.10.1, 3.10.7) |
| El clic no llegaba al demonio | Columna `ultimo_clic_at` y frontera inicial `creada_at`/`adoptada_at`/arranque (3.10.1, 3.10.2) |
| Dependencia invertida | `adopcion_imposible()` crece por commits: 17, 35 y 54 (3.10.7, 4.10) |
| `terminada_at` antes del fin de un tramo | Regla única `MAX(<fórmula>, MAX(ended_at))` (3.10.15, invariante 9) |
| Estado de una sesión `dueno = app` | `viva_activa` con tramo abierto (3.10.2, invariante 19) |
| "Tracker caído" contra "Esperando al tracker" | La alarma nunca tapa a `esperando_demonio` (3.10.14) |
| `pause=3s` contra el `atime` de 8 s | Dos perfiles de tiempos en los E2E (5.2) |
| Vetos sobre ptys reutilizados | Un reclamo vale solo con su shell viva (3.10.1, 3.10.3) |
| Carrera entre `terminal_cerrada` y un clic | Cierre condicional al instante del escaneo (3.10.2, E28) |
| Ancla de tmux en `terminal_no_detectada` | Aporta tipeo (3.10.2, E36) |
| Arranque con `esperando_demonio` | Siempre por la tabla de adopción (3.10.10) |
| Caminos que no miden | Tabla completa con códigos de salida (3.10.16) |
| Windows offline | `scripts/check.sh --windows` aparte, con `cargo fetch` manual o CI (3.10.3, 5.5) |
| Escrituras de herramientas con el usuario presente | Carpetas siempre ignoradas ampliadas (3.5, 3.10.4, riesgo 55) |
| Datos reales | 197 filas (116 cerradas, 81 abiertas); `CHECK` en vez de trigger, probado sobre una copia (3.10.1) |

**Equivalencia de commits v3.1 → v3.2** (para leer documentos y PRs viejos):

`1–14 = 1–14 · 15→19 · 16→20 · 17→21 · 18→23 · 19→24 · 20→25 · 21→26 · 22→27 · 23→28 · 24→29 · 25→30 · 26→31 · 27→32 · 28→33 · 29→34 · 30→35 · 31→36 · 32→37 · 33→38 · 34→39 · 35→43 · 36→45 · 37→46 · 38→47 · 39→48 · 40→49 · 41→50 · 42→51 · 43→53 · 44→54 · 45→55 · 46→56 · 47→57 · 48→59 · 49→64 · 50→65 · 51→66 · 52→67 · 53→68 · 54→71 · 55→72 · 56→73`.

Los 17 commits nuevos: 15 (R8-1), 16 (R8-2), 17 (R8-3), 18 (R8-3b), 22 (R8-A1),
40 (R8-4), 41 (R8-P1), 42 (R8-5), 44 (R8-6), 52 (R8-7), 58 (R8-8), 60 (R8-C1),
61 (R8-C2), 62 (R8-C3), 63 (R8-C4), 69 (R8-9) y 70 (R8-A2).

**Qué cambió en cada sección:**

| Sección | Cambio |
|---|---|
| Encabezado, camino rápido | Versión, estado (F0 hecha), 73 commits, 41,5–52,5 días, rangos por fase |
| §1 | R8 y R8-CLI como requisitos; §1.1 decisiones; §1.2 excepciones; alcance (ajustes sí, cliente no) |
| §3 | Diagrama con presencia, lector de `/proc`, motor de terminales, `restore.lock` y CLI; principios 1, 4 (tres excepciones) y 5 |
| 3.1 | Lo que suma R8 a la lib y al binario |
| 3.2 | Pasos 4 y 5; `source = 'terminal'`; cierres de `terminal`; huérfanas solo `work_button` con tope; `cerrar_sesion` con motivo; columnas de `tracker_status` |
| 3.3 | Filas de `terminal`; `tiempo_total`; simultaneidad |
| 3.4 | `clasificar_salto`; compuerta de presencia; motivos de `file_watch`; paso 6 extendido; hilo de presencia; SIGTERM que relee la config |
| 3.5 | Carpetas siempre ignoradas ampliadas; `trabajar_idle_minutes`; claves `poll=`, `grace=`, `pause=`; presencia de prueba |
| 3.6 | Tres escritores; transacciones adelantadas; `restore.lock` y cierre con `restauracion` en el restore |
| 3.7 | Opciones de aislamiento prohibidas |
| 3.8 | `al_cerrar_ventana`; huérfanas a `esperando_demonio`; `cerrar_por_kill_switch()` |
| 3.9 | `get_live_tracking` con sesiones; contador del demonio; Ajustes; aviso de cabecera sin tracker |
| 3.10 | Nuevo: el diseño completo de R8 |
| §4 | Numeración 1–73; filas R8 nuevas y **R8:** en las heredadas; 4.10 y 4.11 |
| §5 | Unitarios de R8, E23–E37, matriz por terminal, pasos 19–42, `check.sh` con `--windows` y `shellcheck`, 5.6 |
| §6 | Puntos nuevos, trazabilidad de R8 y 6.2 con las 21 invariantes |
| §7, §8 | Riesgos 27–61; "Qué NO hacer" de R8 |
| §9, §10, §11 | 9.3; DX5 = commit 33 (74); fuentes de R8; historial |
