# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

---

## Project Overview

Gestor de Proyectos - Aplicación de escritorio nativa para Linux (Manjaro) que permite gestionar proyectos locales de manera visual y eficiente.

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
pnpm run tauri:dev      # Run app in development mode
pnpm run tauri:build    # Build production app
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
