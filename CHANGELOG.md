# Changelog

Historial detallado de cambios del proyecto Gestor de Proyectos.

## 2026-09-16 - v0.6.1

**Nuevas caracteristicas:**

- "Confirmar antes de eliminar" finalmente gobierna los borrados reversibles (B5)
- unifica el tema en una sola fuente de verdad con modo auto real (B6)

**Correcciones:**

- el Dashboard muestra el error real del backend, y lo muestra (cierre de B7)
- un panic con el lock tomado ya no deja la app muerta hasta el reinicio (B15)
- el PDF repagina en vez de descartar el contenido sobrante (B3)
- traduce al espanol los mensajes de error que el usuario puede accionar (B27)
- get_work_session_status devuelve el path real y no el nombre del proyecto (B2)
- get_project filtra la papelera y las dos operaciones de dos pasos van en transaccion (B4, B8)
- compone el data URL del adjunto en un helper compartido (B1)
- propaga el mensaje real del backend en vez de "Error desconocido" (B7)

**Tests:**

- guardia permanente del CABLEADO de ui.confirm_delete (cierre de B5)
- alinea el mock de get_git_file_count con el contrato real (B21)
- reemplaza el gate e2e falso de CSP por uno que si puede fallar (correccion de B9)

**Otros cambios:**

- merge: plan de fixes 26/27 bugs (fixes-base)
- docs: actualiza ARQUITECTURA.md y CLAUDE.md tras eliminar el socket (B17)
- chore: elimina los scripts de shell hook del socket borrado (B17)
- chore: limpia el frontend del tracking manual eliminado (B17)
- elimina los comandos de tracking manual sin consumidores (B17)
- elimina TimeAggregator y conserva los tipos de sesion (B17)
- elimina el socket de tracking manual y el SessionManager (B17)
- chore: saca del indice los 30,9 MB de binarios de release-v0.3.0 (cierre de B26)
- perf(db): los enlaces se cargan en lote y las 23 columnas viven en un solo lugar (B16)
- perf: el conteo de subproyectos deja de ser secuencial y de pedirse dos veces (B18)
- refactor(frontend): las 104 llamadas a console pasan por un logger con guarda de entorno (B23, parte 2/2)
- refactor(backend): logging con niveles y la perilla log_level finalmente conectada (B23, parte 1/2)
- una sola definicion de cada tipo y los atajos de Settings en un <For> (B20)
- elimina GitInfo.tsx, componente sin ningun consumidor (B19)
- borra los tres logs que nunca se ejecutan en Dashboard (B22)
- chore: regenera los schemas de ACL tras quitar tauri-plugin-shell
- chore: elimina la dependencia muerta de tauri-plugin-shell
- security: restaura fs:deny-default y quita permisos sin consumidor (correccion de B10)
- security: exige proyecto registrado y activo para operar git (B11)
- docs: reorganiza la documentacion y la desacopla del numero de version (B25)
- chore: saca artefactos binarios y el lockfile de npm del indice (B24, B26)
- security: elimina los dos comandos de escritura sin validar y unifica el saneado de nombres (B12, B12b)
- security: declara una CSP explicita y saca devtools de la config de ventana (B9)
- security: reduce la capability del WebView al minimo que la app usa (B10, opcion B)
- versiona src-tauri/Cargo.lock
---

## 2026-08-14 - v0.6.0

**Otros cambios:**

- aplica lint:fix y format post-merge
- agrega @eslint/js faltante y globals de setTimeout/clearTimeout/process para destrabar pnpm run lint
- valida sustitucion de version y arbol sucio en release.sh
- agrega test de migraciones ALTER TABLE contra schema viejo real
- onProjectsChanged usa reloadCurrentView, debounce+guard de busqueda, reorder resiliente y TreeView reactivo
- persiste sesion de tracking al cerrar y arregla race de start_work_session
- sanitiza nombre de archivo del backup markdown contra path traversal
- red de seguridad en purge/empty_trash + fix restore_project y create_project
- guardado atomico de config.json y recuperacion ante corrupcion
- agrega restore_backup real, re-verificacion de integridad en list_backups y tests de run_backup/apply_retention
- cierra inyeccion de comandos en rutas de shell (custom script y terminal)
---

## 2026-07-05 - v0.5.2

**Nuevas caracteristicas:**

- tarjeta del propio grupo al entrar (Trabajar/editar el padre + subproyectos)
---

## 2026-06-29 - v0.5.1

**Nuevas caracteristicas:**

- backup automatico al arrancar (intervalo configurable, sin timers)
---

## 2026-06-29 - v0.5.0

**Nuevas caracteristicas:**

- papelera (soft-delete) con cascada recursiva, restaurar y purga
---

## 2026-06-28 - v0.4.7

**Correcciones:**

- sanea backup de archivos por proyecto (destino configurable, --update, anti-traversal)
---

## 2026-06-28 - v0.4.6

**Nuevas caracteristicas:**

- backup real de la base de datos (VACUUM INTO + verificación + retención)
---

## 2026-06-27 - v0.4.5

**Correcciones:**

- centraliza estado de búsqueda en el store y corrige navegación de vistas
---

## 2026-06-27 - v0.4.4

**Nuevas caracteristicas:**

- sistema de versionado SemVer automático y versión dinámica en UI

**Correcciones:**

- validar ciclos en jerarquía de proyectos (barrera autoritativa backend)
- permitir vaciar campos opcionales y desagrupar proyectos (NULL uniforme)
- config forward-compatible con serde(default) para no romper al actualizar

**Otros cambios:**

- documenta nombre canónico de engram (gestor_proyecto)
- corrige búsqueda global del finder en todas las vistas
- documenta módulo tracking y comandos git en CLAUDE.md
---

---

## 2025-11-05 - v0.4.3 - Componente About y Eliminacion de Analytics

**Reemplazo de Analytics por About/Acerca de:**

- Eliminado componente `Analytics.tsx` (no era util)
- Nuevo componente `About.tsx` con informacion del desarrollador
- Boton "Acerca de" en header (Ctrl+Shift+A)
- Seccion de desarrollador con nombre y email clickeable

---

## 2025-11-04 - v0.4.2 - Mejoras de Calidad de Codigo

- Carpeta win10/ ignorada en `.gitignore` y ESLint
- Eliminados imports no usados (TypeScript)
- Tipos mejorados: `any` reemplazado por tipos especificos
- 7 correcciones de warnings de reactividad SolidJS
- Sugerencias de Clippy aplicadas (Rust)
- Codigo no usado anotado con `#[allow(dead_code)]`

**Resultados:** ESLint 0 errores, Rust 0 warnings, 100% tipado

---

## 2025-10-30 - v0.4.1 - Exportacion a PDF

- Boton PDF en cada tarjeta de proyecto
- Genera PDF con: portada, info general, enlaces, notas
- Se guarda en carpeta del proyecto (`NombreProyecto_YYYYMMDD_HHMMSS.pdf`)
- Backend: modulo `pdf_export/mod.rs` con printpdf 0.7.0

---

## 2025-10-29 - v0.4.0 - Vista de Arbol y Grupos

**TreeView:**
- Panel lateral 320px con jerarquia expand/collapse
- Navegacion grupos -> subproyectos -> edicion
- Soporte para imagenes y emojis de grupos

**Busqueda Mejorada:**
- Incluye todos los proyectos (grupos y subproyectos)
- Badges visuales: "Grupo" (azul), "Subproyecto" (morado)

**Drag & Drop para Grupos:**
- Arrastrar proyectos sobre GroupCards
- Boton para remover de grupos

---

## 2025-10-22 - v0.3.2 - Optimizacion de UI

- Cards mas compactas (padding reducido)
- Grid ampliado: hasta 5 columnas en pantallas grandes
- Header consolidado en 2 filas
- Nuevo componente `ProjectFilters.tsx`

---

## 2025-10-22 - v0.3.1 - Drag & Drop para Reordenar

- Biblioteca `@thisbeyond/solid-dnd` v0.7.5
- Drag handle visible (icono) en cada card
- Orden persistente en SQLite (`display_order`)
- Animaciones suaves con CSS transforms

---

## 2025-10-20 - v0.3.0 - Keyboard Shortcuts & Git Integration

**Shortcuts:**
- Plugin `tauri-plugin-global-shortcut` v2.0
- 6 shortcuts: Ctrl+N, Ctrl+F, Ctrl+Comma, Ctrl+Shift+A, Ctrl+R, Escape
- UI en Settings -> Tab "Atajos"

**Git:**
- 7 comandos Tauri: file_count, modified_files, add, commit, push, pull, ahead_behind
- `EnhancedGitInfo.tsx`: badges y botones de acciones Git
- `GitCommitModal.tsx`: crear commits desde UI

---

## 2025-10-19 - v0.2.1 - Configuracion Completa y Onboarding

**Settings Tabs:**
- Tab Backups: auto_backup, intervalo, cleanup, selector de carpeta
- Tab Interfaz: theme, language, confirm_delete, show_welcome
- Tab Avanzado: log_level, analytics, auto_update

**WelcomeScreen:**
- Wizard de 3 pasos de onboarding
- Persistencia de estado en config

**Build Windows:**
- Documentacion en `BUILD_WINDOWS.md`
- Cross-compilation desde Linux

---

## 2025-10-19 - v0.2.0 - Sistema Multiplataforma

**Sistema de Configuracion (`src-tauri/src/config/`):**
- `schema.rs`: AppConfig, PlatformConfig, ProgramConfig
- `defaults.rs`: valores por OS (Linux/Windows)
- `manager.rs`: lectura/escritura de config.json

**Abstraccion de Plataforma (`src-tauri/src/platform/`):**
- Trait `PlatformOperations`
- Implementaciones: `linux.rs`, `windows.rs`
- Deteccion automatica de 40+ programas

**Settings UI:**
- Modal con 4 tabs
- Configuracion de Terminal, Navegador, File Manager, Editor
- Modos: Auto, Default, Custom, Script

---

## 2025-10-18 - FASE 1: Quick Start & Context

**TodoList Component (TDD):**
- CRUD de tareas por proyecto
- Tests: 11 tests en `TodoList.test.tsx`

**ProjectContext Component (TDD):**
- Vista consolidada con 5 secciones
- Tests: 13 tests en `ProjectContext.test.tsx`

**Sistema de Filtros:**
- Dropdown de estado
- Checkbox "Solo favoritos"
- Contador dinamico

**Favoritos/Pin:**
- Toggle con un click
- Proyectos pinned aparecen primero

---

## 2025-10-16 - Project Journal

- Nueva tabla `project_journal`
- CRUD: create, get, update, delete journal entries
- Componente `ProjectJournal.tsx` con edicion inline
- Soporte Markdown y tags

---

## 2025-10-13 - Markdown Editor

- Componente `MarkdownEditor.tsx`
- Tabs Edit/Preview
- Dependencias: marked, dompurify, @tailwindcss/typography
- Sanitizacion HTML contra XSS

---

## 2025-10-12 - Analytics y Estadisticas

**Backend:**
- Campos: `last_opened_at`, `opened_count`, `total_time_seconds`
- Tabla `project_activity`
- Comandos: track_project_open, add_project_time, get_project_stats

**Frontend:**
- Componente `Analytics.tsx`
- Tracking automatico al abrir proyectos

---

## 2025-10-12 - Build de Produccion

- Tauri 2.1.0
- Binario: 16MB, DEB: 5.5MB
- Instalacion en `~/.local/bin/`
- Desktop entry para menu de aplicaciones
