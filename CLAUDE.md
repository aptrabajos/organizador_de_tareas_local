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

- Tabs Backups, UI, Advanced con configuración adicional
- WelcomeScreen para primera ejecución
- Build para Windows con MSI installer
- Auto-updater integration (tauri-plugin-updater)
- Tests unitarios para config y platform

