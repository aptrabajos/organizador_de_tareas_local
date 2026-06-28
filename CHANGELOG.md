# Changelog

Historial detallado de cambios del proyecto Gestor de Proyectos.

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
