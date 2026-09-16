# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

---

## Project Overview

**Gestor de Proyectos** - Aplicación de escritorio nativa (Tauri 2.x) para gestionar proyectos locales.

> **Versión:** ver `package.json` (única fuente de verdad). `src-tauri/Cargo.toml` y
> `src-tauri/tauri.conf.json` deben acompañar ese número. No repetirlo en la
> documentación: siempre se desincroniza.

### Importante: Es una aplicación de escritorio, NO web

- Construida con **Tauri 2.1** - NO acceder a `localhost:1420` desde el navegador
- El puerto 1420 es interno de Vite (sirve UI a la ventana Tauri)
- Los logs del backend Rust aparecen en la **terminal**, no en el navegador

---

## Tech Stack

| Capa                | Tecnología                                             |
| ------------------- | ------------------------------------------------------ |
| **Frontend**        | SolidJS 1.9, TypeScript 5.7, TailwindCSS 3.4, Vite 6.0 |
| **Backend**         | Rust + Tauri 2.1, SQLite (rusqlite 0.32), Serde        |
| **Testing**         | Vitest 2.1 (unit), Playwright 1.57 (E2E)               |
| **Package Manager** | **pnpm** (NO usar npm)                                 |

---

## Development Commands

```bash
# Desarrollo
pnpm run tauri:dev          # Inicia app nativa (NO abrir en navegador)

# Testing
pnpm test                   # Tests unitarios (Vitest)
pnpm run test:watch         # Tests en modo watch
pnpm run test:e2e           # Tests E2E (Playwright)

# Calidad de código
pnpm run lint               # ESLint
pnpm run lint:fix           # ESLint con auto-fix
pnpm run format             # Prettier

# Build
pnpm run tauri:build        # Build de producción
```

### Problema común: Puerto 1420 ocupado

```bash
pkill -f "gestor-proyectos"
lsof -ti:1420 | xargs kill -9 2>/dev/null
pnpm run tauri:dev
```

---

## Architecture

### Frontend (src/)

```
src/
├── App.tsx                 # Componente raíz con providers (Theme, Shortcuts)
├── stores/projectStore.ts  # Estado global reactivo (SolidJS store)
├── services/api.ts         # Capa de comunicación con Tauri (invoke)
├── components/             # Componentes UI
│   ├── ProjectList.tsx     # Lista principal con drag & drop
│   ├── ProjectForm.tsx     # Formulario CRUD
│   ├── TreeView.tsx        # Vista jerárquica de grupos
│   ├── Dashboard.tsx       # Panel de estadísticas
│   ├── Settings.tsx        # Modal de configuración (4 tabs)
│   └── ...
├── contexts/               # Context providers
│   ├── ThemeContext.tsx    # Dark/light mode
│   └── ShortcutsContext.tsx # Atajos de teclado globales
└── types/                  # Interfaces TypeScript
    ├── project.ts          # Project, ProjectLink, JournalEntry, etc.
    └── config.ts           # AppConfig, ProgramConfig, etc.
```

**Patrón de comunicación Frontend ↔ Backend:**

```typescript
// src/services/api.ts - todas las llamadas a Tauri
import { invoke } from '@tauri-apps/api/core';
export async function getProject(id: number): Promise<Project> {
  return await invoke('get_project', { id });
}
```

### Backend Rust (src-tauri/src/)

```
src-tauri/src/
├── main.rs                 # Entry point, registro de comandos Tauri
├── db/mod.rs               # Capa de base de datos SQLite (CRUD, migraciones)
├── commands/mod.rs         # Handlers de comandos Tauri (#[tauri::command])
├── models/project.rs       # Structs Rust (Project, ProjectLink, etc.)
├── config/                 # Sistema de configuración
│   ├── schema.rs           # Tipos (AppConfig, ProgramConfig, etc.)
│   ├── defaults.rs         # Valores por defecto por OS
│   └── manager.rs          # Lectura/escritura de config.json
├── platform/               # Abstracción multiplataforma
│   ├── mod.rs              # Trait PlatformOperations
│   ├── linux.rs            # Implementación Linux
│   ├── windows.rs          # Implementación Windows
│   └── detection.rs        # Detección de programas instalados
├── tracking/               # Sistema de time tracking automático
│   ├── mod.rs              # API pública del módulo de tracking
│   ├── config.rs           # Configuración del tracking
│   ├── session.rs          # Modelo de sesión de trabajo (start/stop)
│   ├── socket.rs           # IPC vía socket para tracking en background
│   └── aggregator.rs       # Agregación de tiempo por proyecto
└── pdf_export/mod.rs       # Generación de PDFs (printpdf)
```

**Time tracking:** El módulo `tracking/` registra el tiempo de trabajo por proyecto de
forma automática. Usa un socket para comunicar el estado de la sesión y un agregador
que consolida los tiempos. Se inyecta como `State<'_, ActiveSession>` en los comandos
(`start_work_session`, `stop_work_session`, `get_work_session_status`, etc.).

**Patrón de comandos Tauri:**

```rust
// src-tauri/src/commands/mod.rs
#[tauri::command]
pub async fn get_project(db: State<'_, Database>, id: i64) -> Result<Project, String> {
    db.get_project(id).map_err(|e| e.to_string())
}
```

**Volumen y dominios:** Hay ~77 comandos registrados en `main.rs` vía
`tauri::generate_handler![]`, agrupados por dominio: CRUD de proyectos, links,
attachments, journal, todos, grupos jerárquicos, dashboard, config, time tracking
y operaciones de sistema (abrir terminal/editor/explorador).

**Integración Git:** El backend expone una familia de comandos Git que operan sobre
el directorio del proyecto: `get_git_branch`, `get_git_status`, `get_recent_commits`,
`git_add`, `git_commit`, `git_push`, `git_pull`. Los tipos asociados viven en
`src/types/git.ts`.

### Base de Datos SQLite

**Ubicación:** `~/.local/share/gestor-proyectos/projects.db`

**Tablas principales:**

- `projects` - Proyectos (con soporte jerárquico via `parent_id`)
- `project_links` - Enlaces externos por proyecto
- `project_journal` - Diario/bitácora por proyecto
- `project_todos` - Lista de tareas por proyecto
- `project_activity` - Tracking de actividad
- `attachments` - Archivos adjuntos

**Migraciones:** Se ejecutan automáticamente en `db/mod.rs` usando `ALTER TABLE` con graceful failure.

---

## Key Patterns

### SolidJS Reactivity

- Usar `createSignal` para estado local, `createStore` para estado complejo
- Envolver props callbacks en funciones: `onClick={() => props.onClose()}`
- Usar `<For>` en lugar de `.map()` para listas
- Usar `<Show>` para renderizado condicional

### Tauri State Management

- `Database` y `ConfigManager` se inyectan como `State<>` en comandos
- Los comandos retornan `Result<T, String>` para manejo de errores

### Testing

- Tests unitarios en archivos `*.test.tsx` junto a componentes
- Mocks de Tauri API en `src/test-setup.ts`
- Ejecutar test específico: `pnpm test -- TodoList`

---

## Release

```bash
./scripts/release.sh patch              # 0.4.3 → 0.4.4 (bug fixes)
./scripts/release.sh minor              # 0.4.3 → 0.5.0 (nuevas features)
./scripts/release.sh major              # 0.4.3 → 1.0.0 (breaking changes)
./scripts/release.sh patch --install    # + compila e instala binario
./scripts/release.sh patch --dry-run    # muestra cambios sin aplicar
```

El script actualiza la versión en `package.json`, `Cargo.toml` y `tauri.conf.json`, genera entrada en CHANGELOG.md desde los commits, crea commit (`release: vX.Y.Z`) y tag (`vX.Y.Z`).

---

## Git Conventions

- **Commits en español**: `checkpoint: <descripción de la tarea>`
- Ejemplo: `checkpoint: implementa exportación a PDF`
- **Releases**: `release: v{VERSION}` (generado por `scripts/release.sh`)

---

## Memoria Persistente (Engram)

- **Nombre canónico del proyecto: `gestor_proyecto`** (coincide con el directorio).
  Pasar SIEMPRE `project: "gestor_proyecto"` explícito en `mem_save` / `mem_search` /
  `mem_session_summary`.
- El proyecto se llamaba antes "organizador de tareas local". El historial de engram
  previo al 2026-06-27 vive bajo `organizador_de_tareas_local`; para recuperarlo hay
  que buscar con ese nombre explícito.
- Engram (esta versión) no tiene merge de proyectos, por eso la consistencia se
  garantiza pasando el nombre a mano.

---

## Quality Checklist

Antes de completar una tarea:

1. `pnpm run lint:fix` - Corregir errores ESLint
2. `pnpm run format` - Formatear código
3. `pnpm test` - Verificar tests
4. Compilación limpia sin errores de TypeScript/Rust

---

## Config Locations

| Archivo           | Ubicación                                     |
| ----------------- | --------------------------------------------- |
| Config usuario    | `~/.config/gestor-proyectos/config.json`      |
| Base de datos     | `~/.local/share/gestor-proyectos/projects.db` |
| Binario instalado | `~/.local/bin/gestor-proyectos`               |

---

## Changelog

El historial detallado de cambios se encuentra en [CHANGELOG.md](./CHANGELOG.md).
