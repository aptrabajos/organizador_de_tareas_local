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
