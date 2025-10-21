# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

---

## Project Overview

Gestor de Proyectos - **Aplicación de escritorio nativa** para Linux (Manjaro) que permite gestionar proyectos locales de manera visual y eficiente.

### ⚠️ IMPORTANTE: NO es una aplicación web

- Esta es una **aplicación de escritorio nativa** construida con Tauri
- **NO acceder** a `http://localhost:1420` desde el navegador
- El puerto 1420 es usado internamente por Vite (para servir UI a Tauri)
- La aplicación se abre como una **ventana nativa del sistema**
- Los logs del backend Rust aparecen en la **terminal**, no en el navegador

**Stack Frontend:**

- SolidJS: `^1.9.3`
- TypeScript: `^5.7.3`
- TailwindCSS: `^3.4.17`
- Vite: `^6.0.5`
- Vitest: `^2.1.9`

**Stack Backend:**

- Rust with Tauri: `^2.1.0`
- SQLite (rusqlite): `^0.32`
- Serde for JSON serialization

**Package Manager:**

- **IMPORTANT: Este proyecto usa `pnpm`, NO npm**

---

## Development Commands

**IMPORTANTE: Usar `pnpm` en lugar de `npm`**

### Linting & Formatting

```bash
pnpm run lint           # Run ESLint
pnpm run lint:fix       # Run ESLint with auto-fix
pnpm run format         # Format code with Prettier
```

### Testing

```bash
pnpm test               # Run tests once
pnpm run test:watch     # Run tests in watch mode
```

### Development

```bash
# IMPORTANTE: Asegurarse que no hay instancias corriendo primero
pkill -f "gestor-proyectos" 2>/dev/null

# Iniciar aplicación (se abre ventana nativa)
pnpm run tauri:dev

# Ver logs en tiempo real (en archivo separado)
pnpm run tauri:dev > /tmp/gestor-app.log 2>&1 &
tail -f /tmp/gestor-app.log

# Build para producción
pnpm run tauri:build
```

**Problema común: Puerto 1420 ocupado**

```bash
# Solución: Limpiar procesos y puerto
pkill -f "gestor-proyectos"
lsof -ti:1420 | xargs kill -9 2>/dev/null
pnpm run tauri:dev
```

### Production Build & Installation

**Compilar para producción:**

```bash
pnpm run tauri:build
```

Esto genera:

- Binario: `src-tauri/target/release/gestor-proyectos` (~16MB)
- Paquete DEB: `src-tauri/target/release/bundle/deb/Gestor de Proyectos_0.1.0_amd64.deb` (~5.5MB)

**Instalación en Manjaro/Arch:**

```bash
# Copiar binario al PATH del usuario
mkdir -p ~/.local/bin
cp src-tauri/target/release/gestor-proyectos ~/.local/bin/
chmod +x ~/.local/bin/gestor-proyectos

# Crear entrada en el menú de aplicaciones
cat > ~/.local/share/applications/gestor-proyectos.desktop << 'EOF'
[Desktop Entry]
Version=1.0
Type=Application
Name=Gestor de Proyectos
Comment=Gestor visual de proyectos locales para desarrollo
Exec=/home/manjarodesktop/.local/bin/gestor-proyectos
Icon=folder-development
Terminal=false
Categories=Development;Utility;
Keywords=projects;gestor;desarrollo;
StartupNotify=true
EOF

# Hacer ejecutable y actualizar base de datos
chmod +x ~/.local/share/applications/gestor-proyectos.desktop
update-desktop-database ~/.local/share/applications
```

**Ejecutar la aplicación:**

```bash
# Desde terminal
gestor-proyectos

# O buscar "Gestor de Proyectos" en el menú de aplicaciones
```

---

## MCP Server Configuration

The project includes MCP servers configured in [.mcp.json](.mcp.json):

- **taskmaster-ai**: Task management and workflow automation
- **playwright**: Browser automation and testing
- **context7**: Documentation lookup and context management
- **lint-fix**: Automated linting and formatting
- **run-tests**: Test execution
- **git-checkpoint**: Automatic git commits after tool use
- **final-checkpoint**: Final validation and commit

---

## Quality Control Workflow

Before completing any code task:

1. **Run ESLint** on modified files (`pnpm run lint:fix`)
2. **Format code** with Prettier (`pnpm run format`)
3. **Run tests** to ensure nothing breaks (`pnpm test`)
4. **Fix any errors** before marking task complete

Use the configured MCP servers for automated workflows.

---

## Git Conventions

- **Commit messages in Spanish**: `checkpoint: <descripción de la tarea>`
- Use descriptive messages that explain what was accomplished
- Example: `checkpoint: configuración inicial de ESLint y Prettier`

---

## Project Structure

Currently minimal structure with configuration files:

- `.eslintrc.cjs` - ESLint configuration
- `.prettierrc.json` - Prettier configuration
- `.mcp.json` - MCP server definitions
- `package.json` - Dependencies and scripts

**Note**: As the project grows, update this section with actual architecture details (e.g., `src/`, `components/`, `lib/`, etc.)

---

## Adding Dependencies

When adding new dependencies:

1. Propose the package and version before installing
2. Verify compatibility with existing tooling
3. Update this CLAUDE.md if it becomes a core dependency
4. Run tests after installation to ensure no conflicts

---

## Changelog

### 2025-10-20 - v0.3.0 - Keyboard Shortcuts & Enhanced Git Integration

**Sistema de Atajos de Teclado:**

- Plugin `tauri-plugin-global-shortcut` v2.0 integrado
- 6 shortcuts globales: Ctrl+N (nuevo proyecto), Ctrl+F (buscar), Ctrl+Comma (config), Ctrl+Shift+A (analytics), Ctrl+R (recargar), Escape (cerrar modal)
- Contexto `ShortcutsContext.tsx` con handlers reactivos
- UI en Settings → Tab "⌨️ Atajos" con toggles individuales
- Detección automática OS (Ctrl para Linux/Win, Cmd para macOS)
- **CRÍTICO**: Permisos `global-shortcut:allow-*` en tauri.conf.json (requerido Tauri 2.x)

**Git Integration Mejorado:**

- 7 comandos Tauri: file_count, modified_files, add, commit, push, pull, remote_url, ahead_behind
- `EnhancedGitInfo.tsx`: badges (rama, archivos, ahead/behind), botones (Stage All, Commit, Push, Pull, Repo)
- `GitCommitModal.tsx`: crear commits desde UI con push automático opcional
- Timeline últimos 5 commits, toast notifications

**Archivos creados:** `ShortcutsContext.tsx`, `EnhancedGitInfo.tsx`, `GitCommitModal.tsx`, `types/git.ts`

**Troubleshooting:** Shortcuts no funcionaban - faltaban permisos en tauri.conf.json. Console.log no aparece en terminal (solo println! de Rust). Sin DevTools en apps nativas.

**Resultados:** ✅ Shortcuts funcionando ✅ Git features funcionando ✅ 0 errores ESLint

---

### 2025-10-12 - Mejoras de Calidad de Código

**Correcciones de ESLint y TypeScript:**

- Eliminadas variables no usadas en catch blocks (`_err` en App.tsx)
- Reemplazado uso de `any` por tipos específicos (`ProjectFormData` en ProjectFormTabs.tsx)
- Corregido uso de `any` por tipos indexados (`CreateLinkDTO['link_type']` en ProjectLinks.tsx)

**Mejoras de Reactividad en SolidJS:**

- Corregido acceso a props reactivos fuera de contexto en ProjectForm.tsx
- Implementada captura de valores iniciales con IIFE para evitar warnings de reactividad
- Movido acceso a `props.projectId` a contexto reactivo (onMount) en ProjectLinks.tsx

**Resultados:**

- ESLint: 0 errores, 0 warnings (antes: 6 warnings)
- Tests: 38 tests pasando en 5 archivos
- Código completamente tipado sin uso de `any`
- Aplicación probada y funcionando correctamente

### 2025-10-12 - Build de Producción e Instalación

**Compilación:**

- Build exitoso con Tauri 2.1.0
- Binario optimizado: 16MB (release mode)
- Paquete DEB generado: 5.5MB

**Instalación en Manjaro:**

- Binario instalado en `~/.local/bin/gestor-proyectos`
- Entrada .desktop creada en menú de aplicaciones
- Aplicación disponible desde terminal y menú gráfico
- PATH configurado correctamente

**Ubicaciones:**

- Binario: `~/.local/bin/gestor-proyectos`
- Desktop Entry: `~/.local/share/applications/gestor-proyectos.desktop`
- Documentación de instalación agregada a CLAUDE.md

### 2025-10-12 - Sistema de Analytics y Estadísticas

**Implementación Completa del Sistema de Analytics:**

**Backend (Rust + SQLite):**

- Nuevos campos en tabla `projects`:
  - `last_opened_at` (DATETIME) - Timestamp de última apertura
  - `opened_count` (INTEGER) - Contador de veces que se abrió el proyecto
  - `total_time_seconds` (INTEGER) - Tiempo total trabajado en segundos

- Nueva tabla `project_activity`:
  - Registra todas las actividades (opened, edited, backup, etc.)
  - Campos: id, project_id, activity_type, description, duration_seconds, created_at
  - Relación con tabla projects mediante FOREIGN KEY con CASCADE DELETE

**Comandos Tauri Agregados:**

1. `track_project_open(project_id: i64)` - Registra apertura de proyecto
   - Actualiza `last_opened_at` y `opened_count`
   - Crea registro en tabla `project_activity`

2. `add_project_time(project_id: i64, seconds: i64)` - Agrega tiempo trabajado
   - Incrementa `total_time_seconds`

3. `get_project_stats() -> ProjectStats` - Obtiene estadísticas globales
   - Total de proyectos
   - Proyectos activos hoy
   - Tiempo total trabajado (en horas)
   - Proyecto más activo
   - Timeline de últimas 20 actividades

4. `get_project_activities(project_id: i64, limit: i64)` - Obtiene timeline por proyecto

**Frontend (SolidJS + TypeScript):**

- Nuevo componente `Analytics.tsx`:
  - Dashboard con 4 tarjetas de estadísticas
  - Timeline de actividad reciente con íconos dinámicos
  - Botón de actualización manual
  - Formateo de fechas y duraciones
  - Estados de carga y error
  - Soporte para tema oscuro

- Integración en `App.tsx`:
  - Botón "📊 Estadísticas" en header
  - Alternado entre vista de proyectos y analytics
  - Estado reactivo con SolidJS

- Tracking automático en `ProjectList.tsx`:
  - Función `handleOpenTerminal` que registra apertura antes de abrir terminal
  - Logs de confirmación: "📊 Tracking registrado para proyecto: [nombre]"
  - Manejo de errores sin bloquear funcionalidad

**API Layer (services/api.ts):**

- 4 nuevas funciones async que llaman a comandos Tauri
- Tipado completo con interfaces TypeScript
- Manejo de promesas con async/await

**Tipos TypeScript (types/project.ts):**

- `ProjectActivity` interface
- `ProjectStats` interface
- Campos opcionales agregados a `Project` interface

**Archivos Modificados:**

- `src-tauri/src/db/mod.rs` - 4 métodos nuevos, migrations con ALTER TABLE
- `src-tauri/src/models/project.rs` - 3 structs nuevos
- `src-tauri/src/commands/mod.rs` - 4 comandos Tauri
- `src-tauri/src/main.rs` - Registro de nuevos comandos
- `src/components/Analytics.tsx` - Componente nuevo (171 líneas)
- `src/components/ProjectList.tsx` - Tracking automático
- `src/services/api.ts` - 4 funciones API
- `src/types/project.ts` - Interfaces de analytics
- `src/App.tsx` - Integración del botón y vista

**Características Implementadas:**

✅ Tracking automático al abrir proyectos con botón "🚀 Trabajar"
✅ Dashboard de estadísticas en tiempo real
✅ Timeline de actividad con íconos contextuales
✅ Migración de BD sin afectar datos existentes (ALTER TABLE con graceful failure)
✅ UI responsive con Tailwind CSS
✅ Soporte completo para dark mode
✅ Manejo de errores robusto
✅ 0 errores de ESLint, 1 warning esperado en Rust (dead_code en CreateActivityDTO)

**Próximas Funcionalidades Pendientes:**

- Adjuntar archivos pequeños a proyectos
- Distribución por tags en analytics

### 2025-10-13 - Markdown Editor con Preview y Checklists

**Implementación Completa de Editor Rico para Notas:**

**Frontend (SolidJS + TypeScript):**

- Nuevo componente `MarkdownEditor.tsx`:
  - Tabs "✏️ Editar" y "👁️ Vista Previa"
  - Textarea con sintaxis monoespaciada para edición
  - Preview en tiempo real del markdown renderizado
  - Soporte completo para GitHub Flavored Markdown (GFM)
  - Sanitización de HTML con DOMPurify (seguridad XSS)

- Renderizado en ProjectList.tsx:
  - Markdown renderizado en cards de proyectos
  - Scroll automático para notas largas (max-height: 10rem)
  - Estilos tipográficos con @tailwindcss/typography
  - Checkboxes funcionales de solo lectura

**Dependencias Agregadas:**

- `marked@16.4.0` - Parser de Markdown a HTML
- `dompurify@3.2.7` - Sanitización de HTML
- `@tailwindcss/typography@0.5.19` - Estilos para contenido markdown

**Sintaxis Markdown Soportada:**

````markdown
# Títulos (H1-H6)

**Negrita** _Cursiva_ `código inline`

- [ ] Checklist sin marcar
- [x] Checklist marcada
- Listas sin orden

1. Listas ordenadas
   [Links](https://example.com)
   > Citas en bloque
   > \```
   > Bloques de código
   > \```
````

**Archivos Modificados:**

- `src/components/MarkdownEditor.tsx` - Componente nuevo (104 líneas)
- `src/components/ProjectForm.tsx` - Integración del editor en campo notas
- `src/components/ProjectList.tsx` - Renderizado de markdown en cards
- `tailwind.config.js` - Plugin de typography
- `eslint.config.js` - Ignorar tailwind.config.js
- `package.json` - 3 nuevas dependencias

**Características Implementadas:**

✅ Editor con tabs Edit/Preview
✅ Sintaxis markdown completa (GFM)
✅ Checklists funcionales con checkboxes
✅ Sanitización de HTML contra XSS
✅ Preview en tiempo real
✅ Scroll automático para notas largas
✅ Estilos tipográficos profesionales
✅ Soporte dark mode completo
✅ 0 errores de ESLint, 38 tests pasando

### 2025-10-16 - Project Journal (Diario de Proyecto)

**Implementación Completa del Sistema de Diario/Bitácora por Proyecto:**

**Backend (Rust + SQLite):**

- Nueva tabla `project_journal`:
  - `id` (INTEGER PRIMARY KEY) - ID único de la entrada
  - `project_id` (INTEGER) - Relación con proyectos
  - `content` (TEXT) - Contenido en Markdown
  - `tags` (TEXT) - Tags opcionales en formato JSON array
  - `created_at` (DATETIME) - Timestamp de creación
  - `updated_at` (DATETIME) - Timestamp de última actualización
  - Relación con tabla projects mediante FOREIGN KEY con CASCADE DELETE

**Comandos Tauri Agregados:**

1. `create_journal_entry(entry: CreateJournalEntryDTO) -> JournalEntry`
   - Crea una nueva entrada en el diario del proyecto
   - Soporta contenido Markdown y tags opcionales

2. `get_journal_entries(project_id: i64) -> Vec<JournalEntry>`
   - Obtiene todas las entradas del diario ordenadas por fecha (más reciente primero)

3. `update_journal_entry(id: i64, updates: UpdateJournalEntryDTO) -> JournalEntry`
   - Actualiza contenido y/o tags de una entrada existente
   - Actualiza automáticamente el timestamp `updated_at`

4. `delete_journal_entry(id: i64)`
   - Elimina una entrada del diario con confirmación

**Frontend (SolidJS + TypeScript):**

- Nuevo componente `ProjectJournal.tsx`:
  - Modal full-screen con diseño responsivo
  - Formulario rápido para crear nuevas entradas
  - Lista cronológica de entradas (últimas primero)
  - Edición inline con modo edit/view
  - Renderizado de Markdown con preview
  - Sistema de tags con visualización por colores
  - Soporte completo para dark mode
  - Estados de carga y manejo de errores

- Integración en `ProjectList.tsx`:
  - Botón "📓 Diario" junto a "🚀 Trabajar"
  - Modal controlado con estado reactivo
  - Color distintivo (amber-600) para fácil identificación

**Características del Diario:**

✅ **Entrada rápida**: Textarea simple con soporte Markdown
✅ **Tags flexibles**: Sistema de etiquetado separado por comas (#bug, #tip, #idea)
✅ **Vista cronológica**: Ordenamiento automático por fecha descendente
✅ **Edición inline**: Click en ✏️ para editar cualquier entrada
✅ **Markdown rendering**: Preview en tiempo real con sintaxis GFM
✅ **Timestamps**: Muestra fecha de creación y "(editado)" si fue modificado
✅ **Sanitización**: HTML seguro con DOMPurify
✅ **Responsivo**: Diseño adaptable a diferentes tamaños de pantalla
✅ **Dark mode**: Soporte completo para tema oscuro

**Casos de Uso:**

```markdown
# Ejemplos de entradas típicas:

📅 16 Oct 2025 - 14:30
Implementé el cache con WeakMap para evitar memory leaks.
Mejora de performance del 40%.
Tags: #performance #tip

📅 15 Oct 2025 - 18:45
Bug encontrado en el loader de imágenes.
TODO: Revisar mañana el hook useEffect.
Tags: #bug #pendiente

📅 14 Oct 2025 - 10:15
Reunión con cliente - nuevos requerimientos:

- [ ] Agregar filtros por fecha
- [ ] Export a PDF
- [x] Mejorar búsqueda
      Tags: #meeting #features
```

**API Layer (services/api.ts):**

- 4 nuevas funciones async que llaman a comandos Tauri
- Tipado completo con interfaces TypeScript
- Manejo de promesas con async/await

**Tipos TypeScript (types/project.ts):**

- `JournalEntry` interface
- `CreateJournalEntryDTO` interface
- `UpdateJournalEntryDTO` interface

**Archivos Modificados:**

- `src-tauri/src/db/mod.rs` - 4 métodos CRUD para journal, creación de tabla
- `src-tauri/src/models/project.rs` - 3 structs nuevos para journal
- `src-tauri/src/commands/mod.rs` - 4 comandos Tauri para journal
- `src-tauri/src/main.rs` - Registro de nuevos comandos
- `src/components/ProjectJournal.tsx` - Componente nuevo (285 líneas)
- `src/components/ProjectList.tsx` - Integración del botón y modal
- `src/services/api.ts` - 4 funciones API para journal
- `src/types/project.ts` - Interfaces de journal

**Resultados:**

✅ 0 errores de ESLint
✅ 1 warning en Rust (preexistente: dead_code en CreateActivityDTO)
✅ Compilación exitosa en 12.7 segundos
✅ Hot Module Reload funcionando correctamente
✅ UI completamente funcional con dark mode
✅ Migración de BD automática sin pérdida de datos

### 2025-10-18 - FASE 1: Quick Start & Context - Sistema Completo de Organización

**Implementación completa de características para mejorar flujo de trabajo y organización de proyectos**

#### 1. TodoList Component (TDD)

**Backend (ya implementado previamente):**

- Tabla `project_todos` con campos: id, project_id, content, is_completed, created_at, completed_at
- CRUD completo: create_todo, get_project_todos, update_todo, delete_todo
- Relación con proyectos mediante FOREIGN KEY con CASCADE DELETE

**Frontend (SolidJS + TypeScript):**

- Nuevo componente `TodoList.tsx` (219 líneas)
- Tests comprehensivos `TodoList.test.tsx` (230 líneas) - 11 tests
- Funcionalidades:
  - Crear TODOs con input y botón "Agregar"
  - Marcar/desmarcar como completado con checkbox
  - Eliminar TODOs con botón 🗑️
  - Separación visual entre pendientes y completados
  - Estados de carga y error
  - Empty states con mensajes informativos

**Integración:**

- Botón ✅ (verde-600) en cada tarjeta de proyecto
- Modal con header personalizado "✅ Lista de Tareas"
- Botón de cierre (×) en esquina superior derecha

#### 2. ProjectContext Component (TDD)

**Frontend (SolidJS + TypeScript):**

- Nuevo componente `ProjectContext.tsx` (323 líneas)
- Tests comprehensivos `ProjectContext.test.tsx` (268 líneas) - 13 tests
- Vista consolidada con 5 secciones:
  1. **Información del Proyecto**: Nombre, descripción, path, tags, estado
  2. **📓 Diario Reciente**: Últimas 5 entradas con timestamp
  3. **✅ Tareas Pendientes**: Solo TODOs sin completar
  4. **🔗 Enlaces**: Links externos del proyecto (docs, repos, etc.)
  5. **📎 Archivos Adjuntos**: Lista de attachments con preview de imágenes

**Características de Archivos Adjuntos:**

- Íconos dinámicos por tipo: 🖼️ imágenes, �� videos, 🎵 audio, 📄 PDFs, 📦 comprimidos, 📎 otros
- Información: nombre del archivo, tamaño en KB, fecha de creación
- Vista previa de imágenes (thumbnails 48x48px)
- Estados: con archivos (lista) / sin archivos (mensaje)
- Responsive con soporte dark mode completo

**Integración:**

- Botón 📋 (cyan-600) en cada tarjeta de proyecto
- Modal grande (max-w-4xl) con scroll automático
- Carga de datos en paralelo (Promise.all) para mejor performance

#### 3. Sistema de Filtros

**Implementación:**

- Barra de filtros superior con diseño responsive
- Componentes:
  - **Dropdown de Estado**: Todos, Activo, Pausado, Completado, Archivado
  - **Checkbox "📌 Solo favoritos"**: Filtra proyectos pinned
  - **Botón "Limpiar filtros"**: Aparece solo cuando hay filtros activos
  - **Contador**: "X de Y proyectos" actualizado dinámicamente

**Lógica:**

- Función reactiva `filteredProjects()` que combina filtros
- Filtros aplicables simultáneamente (estado + favoritos)
- Empty state cuando no hay resultados
- Mantiene estado de filtros durante actualizaciones

#### 4. Gestión de Estado de Proyectos

**UI:**

- Dropdown en header de cada tarjeta con opciones:
  - 🟢 Activo
  - 🟡 Pausado
  - ✅ Completado
  - 📦 Archivado
- Cambio inmediato con toast de confirmación
- Actualización reactiva sin reload de página

**Backend:**

- Comando `update_project_status(project_id, status)` ya implementado
- Actualiza campo `status` y `status_changed_at` en BD

#### 5. Sistema de Favoritos/Pin

**UI:**

- Botón de pin en header de cada tarjeta
- Estados visuales: 📌 (marcado) / 📍 (sin marcar)
- Toggle con un solo click
- Toast de confirmación

**Backend:**

- Comando `toggle_pin_project(project_id)` retorna nuevo estado
- Proyectos pinned aparecen primero (ORDER BY is_pinned DESC)

#### 6. Mejora de Experiencia UX - Actualización Reactiva

**Eliminación de Recargas Bruscas:**

- Reemplazado `window.location.reload()` por actualización reactiva
- Sistema de callbacks: `onProjectsChanged={() => store.loadProjects()}`
- Mantiene scroll position y estado de UI

**Animaciones CSS Suaves:**

- `@keyframes fadeIn`: Entrada suave (0.4s)
- `@keyframes pulseGreen`: Pulso verde al actualizar (0.6s)
- Clase `.project-card-updated` combina ambas animaciones

**Resultados de Calidad:**

✅ **70/70 tests pasando** (24 tests nuevos)
✅ **0 errores de ESLint**
✅ **0 warnings de linting**
✅ TDD completo para componentes críticos
✅ Hot Module Replacement funcionando

**Beneficios para el Usuario:**

🚀 **Organización**: Filtros, estados y favoritos
📋 **Contexto rápido**: Vista consolidada de toda la información
✅ **Productividad**: TODOs integrados
🎨 **UX mejorada**: Transiciones suaves, sin recargas
⚡ **Performance**: Carga paralela, actualización reactiva
🌙 **Accesibilidad**: Dark mode completo

### 2025-10-19 - FASE 2: Sistema Multiplataforma - Configuración y Abstracción de Plataforma

**Implementación completa de sistema de configuración y soporte multiplataforma (Linux + Windows)**

#### Arquitectura del Sistema

**Backend Rust - Sistema de Configuración (`src-tauri/src/config/`)**

1. **schema.rs** - Definición de tipos (170+ líneas)
   - `AppConfig`: Configuración completa de la aplicación
   - `PlatformConfig`: Configuración de programas por OS
   - `ProgramConfig`: Terminal, Browser, FileManager, TextEditor
   - `ProgramMode`: Auto | Default | Custom | Script
   - `BackupConfig`, `UiConfig`, `AdvancedConfig`
   - `DetectedProgram` y `DetectedPrograms` para detección automática

2. **defaults.rs** - Valores predeterminados por OS
   - Configuración diferenciada para Linux y Windows
   - Variables de entorno según plataforma (HOME/USERPROFILE)
   - Versión 0.2.0 del sistema de configuración

3. **manager.rs** - Gestión de archivo de configuración (140+ líneas)
   - Creación automática en primera ejecución
   - Ubicación: `~/.config/gestor-proyectos/config.json` (Linux)
   - Ubicación: `%APPDATA%/gestor-proyectos/config.json` (Windows)
   - Métodos: `get_config()`, `update_config()`, `reset_config()`
   - Sistema de migración de versiones (preparado para futuro)

**Backend Rust - Abstracción de Plataforma (`src-tauri/src/platform/`)**

1. **mod.rs** - Trait `PlatformOperations`
   - Métodos abstractos: `open_terminal`, `open_url`, `open_file_manager`, `open_text_editor`
   - `execute_script` con reemplazo de variables `{path}`, `{url}`, etc.
   - `replace_variables` con implementación por defecto
   - Factory function `get_platform()` con conditional compilation

2. **detection.rs** - Detección automática de programas (210+ líneas)
   - `ProgramDetector::detect_all()` - Detecta todos los programas disponibles
   - `detect_terminals()`, `detect_browsers()`, `detect_file_managers()`, `detect_text_editors()`
   - Usa comandos `which` (Linux) y `where` (Windows)
   - Retorna lista ordenada por disponibilidad (default primero)

3. **linux.rs** - Implementación Linux (200+ líneas)
   - **Terminales soportados**: konsole, gnome-terminal, alacritty, kitty, xfce4-terminal, tilix, xterm
   - **Navegadores**: firefox, chromium, google-chrome, brave, opera, vivaldi, microsoft-edge
   - **File Managers**: nautilus, dolphin, thunar, nemo, pcmanfm, caja
   - **Editores**: code, subl, gedit, kate, vim, nano, emacs
   - Sistema de fallback automático
   - Scripts ejecutados con bash

4. **windows.rs** - Implementación Windows (200+ líneas)
   - **Terminales**: Windows Terminal (wt), PowerShell, CMD, Git Bash
   - **Navegadores**: Edge, Chrome, Firefox, Brave, Opera
   - **File Manager**: Windows Explorer
   - **Editores**: notepad, notepad++, VSCode, Sublime Text
   - Scripts ejecutados con PowerShell

**Comandos Tauri Agregados (src-tauri/src/commands/mod.rs)**

Comandos migrados a usar platform abstraction:

- `open_terminal(config_manager, path)` - Ahora usa configuración del usuario
- `open_url(config_manager, url)` - Respeta navegador configurado

6 nuevos comandos agregados:

1. `get_config() -> AppConfig` - Obtener configuración actual
2. `update_config(config: AppConfig)` - Guardar configuración
3. `reset_config() -> AppConfig` - Resetear a valores por defecto
4. `detect_programs() -> DetectedPrograms` - Detectar programas instalados
5. `open_file_manager(config_manager, path)` - Abrir explorador de archivos
6. `open_text_editor(config_manager, path)` - Abrir editor de texto

**Frontend TypeScript - UI de Configuración**

1. **Tipos TypeScript** (`src/types/config.ts` - 90 líneas)
   - `AppConfig` interface con todas las sub-configuraciones
   - `ProgramConfig` para cada tipo de programa
   - `DetectedPrograms` y `DetectedProgram` para resultados de detección
   - `ConfigState` para manejo de estado en componentes SolidJS
   - Types: `ProgramMode`, `ThemeMode`, `LogLevel`

2. **API Layer** (`src/services/api.ts`)
   - 6 nuevas funciones async:
     - `getConfig()`, `updateConfig(config)`, `resetConfig()`
     - `detectPrograms()`, `openFileManager(path)`, `openTextEditor(path)`

3. **Componente Settings.tsx** (530+ líneas)
   - Modal completo con 4 tabs: 🖥️ Programas, 💾 Backups, 🎨 Interfaz, 🔧 Avanzado
   - **Tab Programas (100% implementado)**:
     - Configuración de Terminal, Navegador, File Manager, Editor de Texto
     - 4 modos de operación por programa:
       - **Auto**: Detección automática + lista de programas encontrados
       - **Default**: Usar predeterminado del sistema
       - **Custom**: Ruta y argumentos personalizados con variables `{path}`, `{url}`
       - **Script**: Script personalizado bash (Linux) / PowerShell (Windows)
     - Botones: Guardar, Cancelar, Resetear
     - Mensajes de éxito/error con auto-dismiss
   - Estados de carga y manejo de errores robusto
   - Soporte completo dark mode

4. **Integración en App.tsx**
   - Botón "⚙️ Configuración" en header (color gray-600)
   - Modal Settings controlado con estado reactivo
   - Gestión de estado `showSettings`

#### Características Implementadas

✅ **Configuración Automática**: Archivo JSON creado en primera ejecución
✅ **Detección de Programas**: 40+ programas soportados (20+ Linux, 20+ Windows)
✅ **Modos Flexibles**: Auto, Default, Custom, Script por cada tipo de programa
✅ **Variables en Scripts**: Reemplazo de `{path}`, `{url}` en argumentos y scripts
✅ **UI Completa**: Modal de configuración con tabs y preview de programas detectados
✅ **Platform Abstraction**: Trait-based design con conditional compilation
✅ **Migración de Comandos**: open_terminal y open_url usan nuevo sistema
✅ **Persistencia**: Cambios guardados en ~/.config/gestor-proyectos/config.json

#### Programas Soportados

**Linux (20+ programas):**

- Terminales: konsole, gnome-terminal, alacritty, kitty, xfce4-terminal, tilix, xterm
- Navegadores: firefox, chromium, google-chrome, brave, opera, vivaldi, edge
- File Managers: nautilus, dolphin, thunar, nemo, pcmanfm, caja
- Editores: vscode, sublime, gedit, kate, vim, nano, emacs

**Windows (20+ programas):**

- Terminales: Windows Terminal, PowerShell, CMD, Git Bash
- Navegadores: Edge, Chrome, Firefox, Brave, Opera
- File Manager: Explorer
- Editores: notepad, notepad++, VSCode, Sublime Text

#### Archivos Creados/Modificados

**Backend Rust (8 archivos nuevos):**

- `src-tauri/src/config/mod.rs`
- `src-tauri/src/config/schema.rs`
- `src-tauri/src/config/defaults.rs`
- `src-tauri/src/config/manager.rs`
- `src-tauri/src/platform/mod.rs`
- `src-tauri/src/platform/detection.rs`
- `src-tauri/src/platform/linux.rs`
- `src-tauri/src/platform/windows.rs`

**Frontend TypeScript (2 archivos nuevos, 2 modificados):**

- `src/types/config.ts` (nuevo)
- `src/components/Settings.tsx` (nuevo)
- `src/services/api.ts` (modificado - 6 funciones agregadas)
- `src/App.tsx` (modificado - integración de Settings)

**Archivos de configuración modificados:**

- `src-tauri/src/main.rs` - Registro de módulos config y platform
- `src-tauri/src/commands/mod.rs` - 6 comandos nuevos + 2 migrados

#### Resultados de Compilación

✅ **Compilación exitosa** - 0 errores
✅ **4 warnings** - Solo código no usado (migrations, métodos preparados para futuro)
✅ **Hot Module Reload** - Funcionando correctamente
✅ **Configuración persistente** - JSON guardado y cargado correctamente
✅ **Detección funcional** - Programas detectados en sistema Manjaro

#### Próximas Mejoras (v0.3.0)

- ~~Tabs Backups, UI, Advanced con configuración adicional~~ ✅ **Completado en v0.2.1**
- ~~WelcomeScreen para primera ejecución~~ ✅ **Completado en v0.2.1**
- ~~Build para Windows con MSI installer~~ ✅ **Documentado en v0.2.1** (BUILD_WINDOWS.md)
- Auto-updater integration (tauri-plugin-updater)
- Tests unitarios para config y platform

### 2025-10-19 - v0.2.1 - Configuración Completa y Onboarding

**Implementación de Settings Tabs Restantes y WelcomeScreen:**

#### Tab Backups (Settings)

**Características implementadas:**

- Toggle para `auto_backup_enabled` - Habilitar backups automáticos
- Input numérico para `auto_backup_interval` (días entre backups)
- Toggle para `cleanup_old_backups` - Limpieza automática de backups antiguos
- Input numérico para `retention_days` (días a conservar backups)
- Toggle switches con Tailwind peer classes para UI moderna
- Validación de inputs (min: 1, max: 30 días para intervalo)
- Visibilidad condicional con `<Show>` de SolidJS

**Código clave:**

```tsx
{
  /* Backup automático habilitado */
}
<div class="flex items-center justify-between">
  <label class="relative inline-flex cursor-pointer items-center">
    <input
      type="checkbox"
      class="peer sr-only"
      checked={config()?.backup.auto_backup_enabled || false}
      onChange={(e) => {
        const cfg = config();
        if (cfg) {
          setConfig({
            ...cfg,
            backup: {
              ...cfg.backup,
              auto_backup_enabled: e.currentTarget.checked,
            },
          });
        }
      }}
    />
    <div class="peer h-6 w-11 rounded-full bg-gray-200 after:absolute after:start-[2px] after:top-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-gray-300 after:bg-white after:transition-all after:content-[''] peer-checked:bg-blue-600 peer-checked:after:translate-x-full peer-checked:after:border-white peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:border-gray-600 dark:bg-gray-700 dark:peer-focus:ring-blue-800 rtl:peer-checked:after:-translate-x-full" />
  </label>
</div>;
```

#### Tab Interfaz (UI Settings)

**Características implementadas:**

- Dropdown `theme` con opciones: light, dark, auto (sistema)
- Dropdown `language` con opciones: Español, English
- Toggle `confirm_delete` - Confirmación antes de eliminar proyectos
- Toggle `show_welcome` - Mostrar pantalla de bienvenida en primer inicio
- Nota informativa sobre reinicio requerido para cambio de idioma
- Íconos contextuales (☀️ Claro, 🌙 Oscuro, 🔄 Automático)

**Código clave:**

```tsx
<select
  class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
  value={config()?.ui.theme || 'auto'}
  onChange={(e) => {
    const cfg = config();
    if (cfg) {
      setConfig({
        ...cfg,
        ui: {
          ...cfg.ui,
          theme: e.currentTarget.value as 'light' | 'dark' | 'auto',
        },
      });
    }
  }}
>
  <option value="light">☀️ Claro</option>
  <option value="dark">🌙 Oscuro</option>
  <option value="auto">🔄 Automático (sistema)</option>
</select>
```

#### Tab Avanzado (Advanced Settings)

**Características implementadas:**

- Dropdown `log_level` con opciones: trace, debug, info, warn, error
- Toggle `enable_analytics` - Habilitar seguimiento de uso (local)
- Toggle `enable_auto_update` - Actualizaciones automáticas
- Nota informativa sobre auto-update disponible en v0.3.0
- Descripciones detalladas para cada nivel de log

**Código clave:**

```tsx
<select
  class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
  value={config()?.advanced.log_level || 'info'}
  onChange={(e) => {
    const cfg = config();
    if (cfg) {
      setConfig({
        ...cfg,
        advanced: {
          ...cfg.advanced,
          log_level: e.currentTarget.value as
            | 'trace'
            | 'debug'
            | 'info'
            | 'warn'
            | 'error',
        },
      });
    }
  }}
>
  <option value="error">Error - Solo errores críticos</option>
  <option value="warn">Warn - Advertencias y errores</option>
  <option value="info">Info - Información general</option>
  <option value="debug">Debug - Modo depuración</option>
  <option value="trace">Trace - Todo (muy detallado)</option>
</select>
```

#### WelcomeScreen Component

**Características del Wizard de Bienvenida:**

- **Multi-step wizard** con 3 pasos de onboarding
- **Step 1 - ¡Bienvenido!**: Introducción a la aplicación con características principales
  - 📁 Gestión completa de proyectos locales
  - 🔗 Enlaces y recursos organizados
  - 📊 Analytics y estadísticas de uso
  - 📓 Diario y TODOs por proyecto
  - ⚙️ Configuración multiplataforma

- **Step 2 - Características Principales**: Funcionalidades destacadas
  - 🚀 Abrir terminal en el proyecto con un click
  - 📝 Editor Markdown con preview en tiempo real
  - 📎 Adjuntar archivos importantes
  - 🎨 Dark mode y temas personalizables
  - 🔍 Búsqueda y filtros avanzados
  - ⭐ Sistema de favoritos

- **Step 3 - Primeros Pasos**: Tutorial rápido
  - 1️⃣ Crea tu primer proyecto con el botón "+ Nuevo Proyecto"
  - 2️⃣ Configura tus programas favoritos en "⚙️ Configuración"
  - 3️⃣ Usa "🚀 Trabajar" para abrir el terminal del proyecto
  - 4️⃣ Agrega enlaces, notas y TODOs según necesites
  - 5️⃣ Explora Analytics para ver tu progreso

**Navegación y UX:**

- Navegación con botones "← Anterior" y "Siguiente →"
- Botón final "¡Empezar! 🚀" que cierra el wizard
- Indicadores de paso (dots) clickeables para navegación rápida
- Animación de dot activo (w-8 vs w-2)
- Botón X para cerrar en cualquier momento
- Modal con overlay semi-transparente
- Scroll automático para contenido largo

**Persistencia de estado:**

- Al cerrar (X o "¡Empezar!"), actualiza config: `show_welcome: false`
- Llamada async a `updateConfig()` con try/catch
- Integrado en App.tsx con `onMount()` que verifica `config.ui.show_welcome`

**Código del componente** (152 líneas):

```tsx
export default function WelcomeScreen(props: { onClose: () => void }) {
  const [currentStep, setCurrentStep] = createSignal(0);

  const steps = [
    { title: '¡Bienvenido a Gestor de Proyectos!', icon: '👋', description: '...', features: [...] },
    { title: 'Características Principales', icon: '✨', description: '...', features: [...] },
    { title: 'Primeros Pasos', icon: '🎯', description: '...', features: [...] },
  ];

  const handleClose = async () => {
    try {
      const config = await getConfig();
      await updateConfig({
        ...config,
        ui: { ...config.ui, show_welcome: false },
      });
    } catch (err) {
      console.error('Error actualizando config:', err);
    }
    props.onClose();
  };

  return (
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50 p-4">
      {/* Multi-step wizard UI */}
    </div>
  );
}
```

#### Integración en App.tsx

```tsx
import WelcomeScreen from './components/WelcomeScreen';

const [showWelcome, setShowWelcome] = createSignal(false);

onMount(async () => {
  store.loadProjects();
  try {
    const config = await getConfig();
    if (config.ui.show_welcome) {
      setShowWelcome(true);
    }
  } catch (err) {
    console.error('Error cargando config:', err);
  }
});

// JSX:
<Show when={showWelcome()}>
  <WelcomeScreen onClose={() => setShowWelcome(false)} />
</Show>;
```

#### Build para Windows - Documentación

**Archivo creado: BUILD_WINDOWS.md** (200+ líneas)

Documentación completa para cross-compilation desde Linux a Windows:

**Requisitos:**

- Rust target: `x86_64-pc-windows-gnu`
- Compilador MinGW: `mingw-w64-gcc`
- Configuración en tauri.conf.json

**Comandos principales:**

```bash
# Instalar target de Windows
rustup target add x86_64-pc-windows-gnu

# Instalar MinGW en Manjaro
sudo pacman -S mingw-w64-gcc

# Cross-compile para Windows
cargo tauri build --target x86_64-pc-windows-gnu
```

**Ubicaciones de artefactos Windows:**

- Binario: `src-tauri/target/x86_64-pc-windows-gnu/release/gestor-proyectos.exe`
- MSI Installer: `src-tauri/target/x86_64-pc-windows-gnu/release/bundle/msi/`
- NSIS Setup: `src-tauri/target/x86_64-pc-windows-gnu/release/bundle/nsis/`

**Troubleshooting incluido:**

- Problemas de linker
- Dependencias faltantes
- Errores de WebView2
- Configuración de certificados

#### Configuración tauri.conf.json

**Cambios en versión 0.2.1:**

```json
{
  "productName": "Gestor de Proyectos",
  "version": "0.2.1",
  "bundle": {
    "active": true,
    "targets": "all", // Cambiado de array a "all"
    "windows": {
      "certificateThumbprint": null,
      "digestAlgorithm": "sha256",
      "timestampUrl": ""
    }
  }
}
```

#### Correcciones de ESLint

**Errores corregidos:**

1. **Unused import**: Eliminado `createEffect` de Settings.tsx
2. **no-undef**: Cambiado `setTimeout` → `window.setTimeout` (2 ocurrencias)
3. **solid/prefer-for**: Convertido `array.map()` → `<For>` en WelcomeScreen
4. **solid/reactivity**: Convertido step indicators a `<Index>` component

**Resultado final:**

- ✅ 0 errores de ESLint
- ⚠️ 3 warnings aceptables (any type, props reactivity en onClick handlers)

#### Archivos Modificados/Creados

**Nuevos archivos:**

- `src/components/WelcomeScreen.tsx` (152 líneas) - Wizard de onboarding
- `BUILD_WINDOWS.md` (200+ líneas) - Documentación cross-compilation

**Archivos modificados:**

- `src/components/Settings.tsx` (200+ líneas agregadas) - 3 tabs implementados
- `src/App.tsx` (modificado) - Integración WelcomeScreen con onMount
- `src-tauri/tauri.conf.json` (modificado) - Version 0.2.1, targets: "all", windows config
- `package.json` (modificado) - Version 0.2.1, descripción actualizada

#### Compilación y Artefactos

**Build exitoso:**

```bash
pnpm run tauri:build
```

**Artefactos generados (Linux):**

- ✅ Binario: `gestor-proyectos` (17 MB)
- ✅ Paquete DEB: `Gestor de Proyectos_0.2.1_amd64.deb` (5.8 MB)
- ✅ Paquete RPM: `Gestor de Proyectos-0.2.1-1.x86_64.rpm` (5.8 MB)
- ⚠️ AppImage: Error en linuxdeploy (no crítico)

**Warnings de compilación (esperados):**

- `migrate_if_needed` - Método preparado para futuras migraciones
- `CreateActivityDTO` fields - Struct preparado para tracking manual
- `PlatformOperations` methods - Métodos preparados para backups/paths
- `program_exists` - Función utilitaria preparada

**Instalación:**

```bash
cp src-tauri/target/release/gestor-proyectos ~/.local/bin/
chmod +x ~/.local/bin/gestor-proyectos
```

#### Selector de Carpeta para Backups

**Funcionalidad agregada tras pruebas de usuario:**

El usuario solicitó poder elegir la carpeta de destino para los backups. Se implementó:

**Backend (Rust):**

- Comando Tauri `select_backup_folder()` que abre diálogo nativo del sistema
- Usa `tauri_plugin_dialog::DialogExt` (compatible con Tauri 2.x)
- Método `.blocking_pick_folder()` para selección de carpetas
- Retorna `Option<String>` con la ruta o `None` si se cancela

**Frontend (SolidJS + TypeScript):**

- Nueva sección en Settings → Tab Backups: "📁 Carpeta de Backups"
- Input de solo lectura mostrando ruta actual (`backup.default_path`)
- Botón "📁 Seleccionar" que invoca el diálogo del sistema
- Al seleccionar, actualiza la configuración automáticamente
- Mensaje de confirmación temporal (3 segundos): "Carpeta seleccionada: /ruta"
- Estilo responsive con Tailwind CSS y soporte dark mode

**Código clave (commands/mod.rs):**

```rust
#[tauri::command]
pub async fn select_backup_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    println!("📁 [DIALOG] Abriendo diálogo de selección de carpeta");

    let result = tauri_plugin_dialog::DialogExt::dialog(&app)
        .file()
        .set_title("Seleccionar carpeta de backups")
        .blocking_pick_folder();

    match result {
        Some(path) => {
            let path_str = path.to_string();
            println!("✅ [DIALOG] Carpeta seleccionada: {}", path_str);
            Ok(Some(path_str))
        }
        None => {
            println!("⚠️ [DIALOG] Usuario canceló la selección");
            Ok(None)
        }
    }
}
```

**Archivos modificados:**

- `src-tauri/src/commands/mod.rs` - Comando `select_backup_folder`
- `src-tauri/src/main.rs` - Registro del comando
- `src/services/api.ts` - Función `selectBackupFolder()`
- `src/components/Settings.tsx` - Sección de selección de carpeta

**UX implementada:**

1. Usuario abre Settings → Tab Backups
2. Primera sección muestra input con ruta actual o "(No configurada)"
3. Click en "📁 Seleccionar" → Se abre diálogo nativo del OS
4. Usuario navega y selecciona carpeta
5. Ruta aparece en el input
6. Mensaje verde de confirmación por 3 segundos
7. Click en "💾 Guardar Configuración" → Persiste en config.json

#### Características Implementadas v0.2.1

✅ **Tab Backups completo** - auto_backup, intervalo, cleanup, retention, **selector de carpeta**
✅ **Tab Interfaz completo** - theme, language, confirm_delete, show_welcome
✅ **Tab Avanzado completo** - log_level, analytics, auto_update
✅ **WelcomeScreen wizard** - 3 pasos, navegación, persistencia
✅ **Documentación Windows** - BUILD_WINDOWS.md con cross-compilation
✅ **Selector de carpeta nativo** - Diálogo del sistema para elegir ruta de backups
✅ **Build 0.2.1** - DEB, RPM, binario instalado
✅ **ESLint limpio** - 0 errores, warnings aceptables
✅ **Dark mode** - Soporte completo en todos los componentes nuevos
✅ **Toggle switches** - UI moderna con Tailwind peer classes
✅ **Configuración persistente** - JSON en ~/.config/gestor-proyectos/

#### Próximas Funcionalidades (v0.3.0)

- Implementar lógica de backups automáticos (scheduler en Rust)
- Auto-updater con tauri-plugin-updater
- Internacionalización (i18n) con archivos de traducción
- Tests E2E con Playwright para flujo de onboarding
- Build nativo para Windows (MSI installer funcional)
- Logs con niveles configurables (integrate log_level)
