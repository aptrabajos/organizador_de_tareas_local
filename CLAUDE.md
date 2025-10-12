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
