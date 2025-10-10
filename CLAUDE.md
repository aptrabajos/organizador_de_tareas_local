# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

---

## Project Overview

This is a TypeScript/JavaScript template repository with configured tooling for linting, formatting, and testing. It serves as a foundation for new projects with quality control infrastructure already in place.

**Stack:**
- TypeScript: `^5.7.3` (via `@types/node`)
- ESLint: `^9.37.0` with TypeScript plugin
- Prettier: `^3.6.2`
- Vitest: `^2.1.9`

---

## Development Commands

### Linting & Formatting
```bash
npm run lint           # Run ESLint
npm run lint:fix       # Run ESLint with auto-fix
npm run format         # Format code with Prettier
```

### Testing
```bash
npm test               # Run tests once
npm run test:watch     # Run tests in watch mode
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

1. **Run ESLint** on modified files (`npm run lint:fix`)
2. **Format code** with Prettier (`npm run format`)
3. **Run tests** to ensure nothing breaks (`npm test`)
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
