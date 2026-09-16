# Reporte de cierre — plan de fixes v0.6.1

Cierre del plan de arreglo de bugs ejecutado sobre la v0.6.0: qué se tocó, por
qué, qué se decidió y qué quedó abierto. El `CHANGELOG.md` lista los commits;
acá está el porqué.

## Resumen

- **26 de los 27 bugs del plan** cerrados. El restante (B14) no es un pendiente:
  es una decisión tomada de no hacerlo ahora (ver más abajo).
- **38 commits** en la rama `fixes-base`, en cinco fases ordenadas por
  dependencia: seguridad → higiene → correctitud de datos → funcionalidad →
  deuda técnica. Merge a `main` y release **v0.6.1** publicada con tag.

## Qué se corrigió

### Seguridad (B9, B10, B11, B12, B12b, B13)

- **B9 — Ventana sin política de contenido y con devtools abiertas.** Se
  empaquetaba con `csp: null` y `devtools: true`: contenido inyectado en la
  vista podía ejecutar código, y el inspector se abría en la app instalada.
  Ahora hay una CSP explícita y devtools sale de la config de ventana.
- **B10 — Permisos de disco demasiado amplios.** La vista web tenía acceso de
  lectura/escritura a `$HOME`, `/home`, `/tmp` y `/mnt` completos. Se volvió a
  `deny` por defecto y quedaron solo los permisos que la app realmente usa.
- **B11 — Comandos de Git sobre rutas arbitrarias.** Se podía operar Git contra
  cualquier carpeta de la máquina; ahora exige proyecto registrado y activo.
- **B12 / B12b — Dos comandos de escritura sin validación.** `write_file_to_path`
  y `sync_project` escribían donde se les indicara y no los llamaba nadie: se
  eliminaron en vez de blindarlos. El saneado de nombres de archivo quedó
  unificado en un solo lugar.
- **B13 — `Cargo.lock` fuera del control de versiones.** Dos compilaciones del
  mismo commit podían traer dependencias distintas. Se versionó.

### Higiene del repositorio y documentación (B24, B25, B26)

- **B24 — Dos lockfiles conviviendo.** Se sacó `package-lock.json`; la única
  fuente es `pnpm-lock.yaml`.
- **B25 — Documentación a la deriva.** 25 markdowns sueltos en la raíz, varios
  anunciando versiones viejas. Se reorganizó en `docs/` y se desacopló del
  número de versión.
- **B26 — Artefactos binarios versionados.** ~68 MB en el índice, incluido un
  `.exe` de 26 MB. Salieron del índice; siguen en disco, ignorados.

### Correctitud de datos (B21, B7, B27, B4, B8, B2)

- **B4 — Proyectos borrados que reaparecían.** `get_project` no filtraba la
  papelera: un proyecto eliminado seguía siendo accesible por consulta directa.
- **B8 — Operaciones de dos pasos sin transacción.** Cerrar una sesión y
  reordenar proyectos fijados escribían suelto; si fallaba el segundo paso,
  quedaban datos a medias. Ahora van en transacción.
- **B2 — Ruta equivocada en la sesión activa.** `get_work_session_status`
  devolvía el *nombre* del proyecto donde debía ir la *ruta*.
- **B7 — "Error desconocido".** La interfaz tapaba el mensaje real del backend
  con un genérico. Ahora se propaga el concreto, y el Dashboard lo muestra.
- **B27 — Mensajes mitad en inglés, mitad en español.** Se tradujeron los
  mensajes de error que el usuario puede accionar.
- **B21 — Test que validaba un contrato inexistente.** El mock del conteo de
  archivos de Git devolvía campos que el código real no usa: pasaba sin probar.

### Funcionalidad visible que no funcionaba (B1, B6, B5, B3)

- **B5 — "Confirmar antes de eliminar" no hacía nada.** La opción se guardaba y
  nunca se leía: la app confirmaba siempre. Ahora gobierna los borrados
  reversibles, con un test que lo cuida.
- **B6 — Cambiar el tema en Ajustes no cambiaba la interfaz.** Había dos fuentes
  de verdad para el tema. Quedó una sola, y el modo automático funciona.
- **B3 — El PDF perdía contenido.** Al exceder el alto de la página, la
  exportación cortaba y descartaba el resto en lugar de abrir página nueva.
- **B1 — Vista previa de imágenes rota.** El adjunto se veía bien en un panel y
  roto en otro: cada uno armaba la URL a su manera. Se unificó en un helper.

### Deuda técnica (B22, B19, B20, B23, B18, B16, B15)

- **B15 — Un fallo podía dejar la app muerta hasta reiniciarla.** Un error con
  el candado de la base tomado lo envenenaba y toda operación posterior fallaba.
  Se corrigió el patrón.
- **B16 — Consultas N+1 en los listados.** Los enlaces de cada proyecto se
  pedían de a uno; ahora se cargan en lote, y las 23 columnas de proyecto se
  definen en un solo lugar.
- **B18 — Conteo de subproyectos secuencial y duplicado.** Se pedía dos veces y
  de a uno, lo que se notaba al abrir la vista de grupos.
- **B23 — Logging de producción sin control.** 105 llamadas a `console.*` en el
  frontend sin guarda de entorno y 138 `println!` en el backend, uno de ellos
  imprimiendo la consulta SQL. Ahora el frontend pasa por un logger con guarda
  y el backend tiene niveles: la perilla `log_level` de Ajustes, que existía
  pero no estaba conectada, finalmente hace algo.
- **B19, B20, B22 — Código muerto.** Se eliminó `GitInfo.tsx` (sin consumidor),
  los tipos duplicados y tres logs del Dashboard que nunca se ejecutaban.

## Decisiones de producto tomadas

### B17 — Se eliminó el tracking manual por socket

Convivían **dos caminos** para registrar tiempo sobre la misma tabla. Uno
funcionaba (`work_session`, el botón "Trabajar"); el otro era un servidor de
socket en `/tmp` que nunca se arrancaba, solo para Unix, y cuyo
`get_tracking_status` era un stub que respondía siempre "no estoy midiendo" —
peor que no existir, porque quien lo llamara le iba a creer.

**Se eliminó** (~1.300 líneas): `tracking/socket.rs`, `tracking/session.rs`, el
`SessionManager`, el `TimeAggregator`, los comandos `start_tracking` /
`stop_tracking` / `get_tracking_status`, su superficie en el frontend y los tres
scripts de shell hook (`gestor-track.sh`, `.zsh`, `.fish`) que eran clientes de
un servidor que ya no existe.

**Se conservó** el camino `work_session` completo: `start_work_session`,
`stop_work_session`, `get_work_session_status`, la carpeta `.gestor/` por
proyecto y la acreditación de segundos. Es el único camino y está documentado en
`ARQUITECTURA.md`. Consecuencia asumida: si algún día se quiere tracking
automático por shell hooks, se arranca de cero y por el cliente, no por el
servidor.

### B14 — No se fragmentan los monolitos por ahora

Hay cuatro archivos grandes (`db/mod.rs` con 3.536 líneas a la cabeza). Se
decidió **no** partirlos en este ciclo, por una razón operativa: un refactor de
esa escala mezclado con 26 arreglos funcionales hace imposible bisecar una
regresión — el `git bisect` cae en un commit que movió miles de líneas y no se
distingue el refactor del arreglo. En su lugar se hicieron cinco extracciones
con justificación funcional propia (guard de Git, `row_to_project`, atajos de
Ajustes, helper de imagen, paginación del PDF): los archivos se achicaron como
efecto de arreglar bugs.

## Deuda conocida que queda abierta

- **B14**: el refactor grande sigue pendiente. Ahora es el momento razonable
  para encararlo: bugs cerrados, tests verdes, release publicada.
- **`ui.language` y `advanced.database_path`**: existen en Ajustes y todavía no
  gobiernan nada. Quedaron explícitamente fuera del plan.

## Cómo se validó

- Suite completa en verde: **168 tests de backend** y **203 de frontend**.
- Verificación de regresiones bug por bug contra el plan, y build de la app.
- Los tests del módulo eliminado en B17 se dieron de baja con el módulo; los del
  camino `work_session` se mantienen.

### Cuatro puntos que hay que validar a ojo en la app

Los tests no cubren renderizado ni el efecto visual de la configuración. Estos
cuatro cambios tocan lo que se ve:

1. **Ajustes → Atajos.** La lista dejó de estar escrita a mano y se genera en un
   bucle (B20). Verificar que aparecen todos los atajos, en orden y sin
   duplicados.
2. **Vista de grupos y contadores de subproyectos.** Cambió cómo y cuándo se
   piden los conteos (B18). Verificar que cada grupo muestra el número correcto
   y que no queda en cero al entrar.
3. **Perilla `log_level` en Ajustes.** Antes no estaba conectada (B23). Cambiar
   el nivel y confirmar que la salida de logs efectivamente cambia.
4. **Estado de error del Dashboard.** Ahora muestra el mensaje real del backend
   (B7). Forzar un error y verificar que se ve el texto concreto y no un
   genérico.

## Referencias

- Snapshot previo: `a7be761` (tag `pre-fixes`, rama de trabajo `fixes-base`).
- Merge del plan a `main`: `1a0b06e`.
- Release: `3fb931a`, tag `v0.6.1`.
- Detalle commit a commit: `CHANGELOG.md`, entrada v0.6.1. Origen del trabajo:
  `plan-arreglo-bugs.md`, con los 27 bugs verificados uno por uno contra el
  código.
