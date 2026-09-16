# 🏗️ Arquitectura del Proyecto - Gestor de Proyectos

## 📌 Resumen Rápido

**¿Qué es este proyecto?**
Una **aplicación de escritorio nativa** para Linux construida con:
- **Backend:** Rust + Tauri + SQLite
- **Frontend:** SolidJS + TypeScript + TailwindCSS
- **Build Tool:** Vite

---

## 🎯 ¿Para qué está instalado Vite?

### ✅ Vite SÍ se usa - Es FUNDAMENTAL

**Vite NO es parte del runtime de la aplicación** (no se incluye en el ejecutable final).

**Vite es la herramienta que:**

1. **Compila el código del frontend** (Desarrollo y Producción)
   ```
   src/App.tsx (TypeScript + SolidJS)
         ↓ Vite compila
   dist/assets/index.js (JavaScript vanilla optimizado)
   ```

2. **Procesa los estilos**
   ```
   src/index.css + TailwindCSS
         ↓ Vite + PostCSS
   dist/assets/index.css (CSS optimizado)
   ```

3. **Sirve la UI en desarrollo** (Hot Module Replacement)
   ```
   Puerto 1420 (solo interno)
         ↓
   Tauri carga la UI desde aquí
         ↓
   Ventana nativa muestra la UI
   ```

4. **Optimiza para producción**
   - Minificación
   - Tree-shaking (elimina código no usado)
   - Code splitting
   - Compresión

---

## 🏗️ Arquitectura Completa

```
┌─────────────────────────────────────────────────────────┐
│           APLICACIÓN NATIVA - GESTOR DE PROYECTOS       │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│  VENTANA NATIVA DEL SISTEMA OPERATIVO (Linux/Windows)   │
│  ┌───────────────────────────────────────────────────┐  │
│  │   WebView (webkit2gtk en Linux)                   │  │
│  │  ┌─────────────────────────────────────────────┐  │  │
│  │  │  FRONTEND (UI de la aplicación)            │  │  │
│  │  │                                             │  │  │
│  │  │  - Código: SolidJS + TypeScript            │  │  │
│  │  │  - Estilos: TailwindCSS                    │  │  │
│  │  │  - Componentes: App.tsx, ProjectList, etc  │  │  │
│  │  │                                             │  │  │
│  │  │  Compilado por: Vite ⚡                     │  │  │
│  │  │  Resultado: HTML + JS + CSS estáticos      │  │  │
│  │  └─────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────┘  │
└──────────────────┬──────────────────────────────────────┘
                   │
                   │ Tauri API Bridge
                   │ (Comunicación bidireccional)
                   │
┌──────────────────▼──────────────────────────────────────┐
│              BACKEND RUST (Core de la App)              │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Comandos Tauri (src-tauri/src/commands/mod.rs)  │  │
│  │                                                   │  │
│  │  #[tauri::command]                                │  │
│  │  pub async fn create_project(...)                 │  │
│  │  pub async fn get_all_projects(...)               │  │
│  │  pub async fn open_terminal(...)                  │  │
│  │  pub async fn create_project_backup(...)          │  │
│  │  pub async fn sync_project_to_backup(...)         │  │
│  └───────────────────────────────────────────────────┘  │
│                                                          │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Base de Datos (src-tauri/src/db/mod.rs)         │  │
│  │                                                   │  │
│  │  - SQLite embebido (rusqlite)                    │  │
│  │  - CRUD operations                                │  │
│  │  - Búsquedas                                      │  │
│  └───────────────────────────────────────────────────┘  │
│                                                          │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Operaciones del Sistema                          │  │
│  │                                                   │  │
│  │  - Abrir terminales (konsole, gnome-terminal)    │  │
│  │  - Abrir URLs en navegador (xdg-open)            │  │
│  │  - File system (rsync, backups)                  │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│              ALMACENAMIENTO LOCAL                        │
│  ~/.local/share/gestor-proyectos/projects.db           │
└─────────────────────────────────────────────────────────┘
```

---

## 🔄 Flujo Completo

### Modo Desarrollo (`pnpm run tauri:dev`)

```
1. package.json: "tauri:dev" → ejecuta tauri dev
         ↓
2. tauri.conf.json: "beforeDevCommand" → pnpm run dev
         ↓
3. package.json: "dev" → ejecuta vite
         ↓
4. Vite inicia servidor en puerto 1420
   - Compila TypeScript a JavaScript (en memoria)
   - Compila SolidJS a JavaScript vanilla
   - Procesa TailwindCSS
   - Hot Module Replacement activo
         ↓
5. Tauri compila el backend Rust
   - cargo run compila src-tauri/
   - Genera target/debug/gestor-proyectos
         ↓
6. Tauri carga UI desde http://localhost:1420
   - Lee tauri.conf.json: "devUrl"
   - WebView carga la UI desde Vite
         ↓
7. Se abre la ventana nativa con la aplicación
```

**Ventajas en desarrollo:**
- ⚡ Hot reload: cambios instantáneos en UI
- 🔄 Recompilación automática de Rust
- 📝 Logs en terminal

### Modo Producción (`pnpm run tauri:build`)

```
1. Vite compila frontend para producción
   - Minificación y optimización
   - Tree-shaking (elimina código no usado)
   - Genera dist/ con HTML+JS+CSS
         ↓
2. Tauri compila backend Rust (modo release)
   - Optimizaciones de Rust activas
   - Genera target/release/gestor-proyectos
         ↓
3. Tauri empaqueta todo junto
   - Embediza carpeta dist/ en el ejecutable
   - Crea paquetes (.deb, .AppImage, .rpm)
         ↓
4. Ejecutable final (~5-10 MB)
   - Backend Rust compilado
   - Frontend (HTML+JS+CSS) embebido
   - NO incluye Node.js, NO incluye Chromium
   - Solo usa WebView del sistema
```

**Ventajas en producción:**
- 📦 Ejecutable pequeño (~5-10 MB vs ~150 MB de Electron)
- 🚀 Inicio rápido
- 💾 Bajo consumo de RAM (~20-50 MB vs ~100-300 MB)
- ✅ Un solo archivo para distribuir

---

## 🔌 Comunicación Frontend ↔ Backend

### Desde el Frontend (TypeScript)

```typescript
// src/services/api.ts
import { invoke } from '@tauri-apps/api/core';

export async function createProject(project: CreateProjectDTO): Promise<Project> {
  return await invoke('create_project', { project });
}
```

### En el Backend (Rust)

```rust
// src-tauri/src/commands/mod.rs
#[tauri::command]
pub async fn create_project(
    db: State<'_, Database>,
    project: CreateProjectDTO,
) -> Result<Project, String> {
    db.create_project(project)
        .map_err(|e| format!("Error creating project: {}", e))
}
```

**El puente lo hace Tauri automáticamente:**
- Serializa/deserializa JSON
- Maneja errores
- Comunica entre el WebView y el proceso Rust

---

## 📦 Stack Tecnológico Detallado

### Frontend

| Tecnología | Versión | Rol |
|------------|---------|-----|
| **SolidJS** | 1.9.3 | Framework UI reactivo (sin Virtual DOM) |
| **TypeScript** | 5.7.3 | Lenguaje con tipado estático |
| **TailwindCSS** | 3.4.17 | Framework de utilidades CSS |
| **Vite** | 6.0.5 | Build tool y dev server |
| **solid-toast** | 0.5.0 | Notificaciones |

### Backend

| Tecnología | Versión | Rol |
|------------|---------|-----|
| **Rust** | 2021 edition | Lenguaje de sistemas |
| **Tauri** | 2.1.0 | Framework para apps nativas |
| **rusqlite** | 0.32 | Driver SQLite |
| **serde** | 1.x | Serialización JSON |
| **chrono** | 0.4 | Manejo de fechas |

### Herramientas

| Tecnología | Rol |
|------------|-----|
| **pnpm** | Package manager de Node.js |
| **cargo** | Package manager de Rust |
| **Vitest** | Framework de tests |
| **ESLint** | Linter para TypeScript |
| **Prettier** | Formateador de código |

---

## 🆚 Comparación: Tauri vs Electron

### Electron (ej: VSCode, Slack, Discord)

```
┌─────────────────────────────────────────┐
│  ELECTRON APP                           │
├─────────────────────────────────────────┤
│  Chromium completo embebido (~100 MB)   │  ← ❌ Muy pesado
│  Node.js runtime (~30 MB)               │  ← ❌ Extra
│  Tu código (React/Vue + JS)             │
├─────────────────────────────────────────┤
│  Tamaño final: 150-200 MB              │
│  RAM: 100-300 MB                        │
│  Inicio: Lento                          │
└─────────────────────────────────────────┘
```

### Tauri (Tu proyecto)

```
┌─────────────────────────────────────────┐
│  TAURI APP                              │
├─────────────────────────────────────────┤
│  WebView del sistema (0 MB, ya existe) │  ← ✅ Ligero
│  Backend Rust compilado (~5 MB)        │  ← ✅ Rápido
│  Frontend compilado (~48 KB)           │  ← ✅ Optimizado
├─────────────────────────────────────────┤
│  Tamaño final: 5-10 MB                 │
│  RAM: 20-50 MB                          │
│  Inicio: Rápido                         │
└─────────────────────────────────────────┘
```

**Ventajas de Tauri:**
- ✅ 10x más pequeño
- ✅ 5x menos RAM
- ✅ Inicio más rápido
- ✅ Backend en Rust (más seguro y rápido)
- ✅ Usa recursos del sistema (WebView ya instalado)

**"Desventajas" de Tauri:**
- ⚠️ WebView puede variar entre sistemas (pero webkit2gtk es estándar en Linux)
- ⚠️ Menos ecosistema que Electron (pero está creciendo rápido)

---

## 📂 Estructura de Archivos

```
gestor_proyecto/
├── src/                          ← FRONTEND (SolidJS)
│   ├── App.tsx                   # Componente principal
│   ├── main.tsx                  # Entry point
│   ├── index.css                 # Estilos globales
│   ├── components/               # Componentes UI
│   │   ├── ProjectForm.tsx
│   │   ├── ProjectList.tsx
│   │   └── SearchBar.tsx
│   ├── stores/                   # Estado global
│   │   └── projectStore.ts
│   ├── services/                 # Comunicación con backend
│   │   └── api.ts
│   └── types/                    # Tipos TypeScript
│       └── project.ts
│
├── src-tauri/                    ← BACKEND (Rust)
│   ├── src/
│   │   ├── main.rs               # Entry point
│   │   ├── commands/             # Comandos Tauri
│   │   │   └── mod.rs
│   │   ├── db/                   # SQLite operations
│   │   │   └── mod.rs
│   │   ├── models/               # Structs de datos
│   │   │   ├── mod.rs
│   │   │   └── project.rs
│   │   ├── backup/               # Backup/restore de la BD
│   │   ├── config/               # Configuración de la app (config.json)
│   │   │   ├── schema.rs         # AppConfig y sub-structs
│   │   │   ├── defaults.rs       # Defaults por OS
│   │   │   └── manager.rs        # ConfigManager (carga/guardado)
│   │   ├── platform/             # Abstracción por sistema operativo
│   │   │   ├── detection.rs      # ProgramDetector
│   │   │   ├── linux.rs          # LinuxPlatform
│   │   │   └── windows.rs        # WindowsPlatform
│   │   ├── pdf_export/           # Exportación de proyectos a PDF
│   │   │   └── mod.rs
│   │   └── tracking/             # Time tracking (PARCIALMENTE CABLEADO)
│   │       ├── config.rs         # Carpeta .gestor/ por proyecto
│   │       ├── session.rs        # TrackingSession / SessionManager
│   │       ├── socket.rs         # SocketServer (Unix socket)
│   │       └── aggregator.rs     # TimeAggregator (persistencia)
│   ├── Cargo.toml                # Dependencias Rust
│   └── tauri.conf.json           # Configuración Tauri
│
├── dist/                         ← OUTPUT DE VITE (generado)
│   ├── index.html
│   └── assets/
│       ├── index-[hash].js       # JavaScript compilado
│       └── index-[hash].css      # CSS compilado
│
├── vite.config.ts                ← CONFIGURACIÓN DE VITE
├── tailwind.config.js            ← Configuración TailwindCSS
├── package.json                  # Dependencias Node.js
└── tsconfig.json                 # Configuración TypeScript
```

---

## 🧩 Módulos del backend Rust

Cuatro módulos que no se mencionaban en este documento y que conviene entender
antes de tocar el backend: `config/`, `platform/`, `pdf_export/` y `tracking/`.

Los tres primeros están cableados en producción. **El cuarto no lo está del
todo** — leé la advertencia grande más abajo antes de asumir nada.

### `config/` — configuración persistente de la aplicación

Maneja el `config.json` del usuario: qué terminal abrir, qué navegador, temas,
atajos, política de backups.

| Archivo | Responsabilidad |
| ------- | --------------- |
| `schema.rs` | Define `AppConfig` y sus sub-structs: `PlatformConfig`, `BackupConfig`, `UiConfig`, `AdvancedConfig`, `ShortcutsConfig`. Cada programa (terminal, navegador, file manager, editor) se describe con un `ProgramConfig` cuyo `ProgramMode` puede ser `Auto`, `Default`, `Custom` o `Script`. |
| `defaults.rs` | Implementa `Default` para todo el árbol y expone `get_os_defaults()`, que arma una configuración inicial según el sistema operativo. `CONFIG_VERSION` está en `"0.3.0"`. |
| `manager.rs` | `ConfigManager`: resuelve la ruta del archivo (`~/.config/gestor-proyectos/config.json` en Linux, `%APPDATA%/gestor-proyectos/` en Windows), lo crea en la primera ejecución, lo carga y lo guarda. |

Dos decisiones de diseño que importan:

- **Forward-compatibility.** Todos los structs llevan `#[serde(default)]`. Un
  `config.json` viejo al que le faltan campos nuevos **no rompe**: los campos
  ausentes toman su `Default`.
- **Carga infalible.** `load_from_file` nunca propaga error. Si el JSON está
  corrupto, hace un respaldo del archivo y cae a los defaults del OS. La razón
  es concreta: `main.rs` hace `ConfigManager::new().expect(...)`, así que un
  error ahí sería un panic **antes de que exista la ventana**, dejando al
  usuario con una app que no abre y sin ningún mensaje.

`ConfigManager` se instancia en `main.rs` y se registra con `.manage()`, o sea
que está disponible como `State<ConfigManager>` en cualquier comando Tauri.

### `platform/` — abstracción de sistema operativo

Define el trait `PlatformOperations` con las operaciones que dependen del OS:
`open_terminal`, `open_url`, `open_file_manager`, `open_text_editor`,
`execute_script`, y los directorios de config/datos/backup.

`get_platform()` devuelve la implementación correcta según `target_os`:
`LinuxPlatform` (`linux.rs`) o `WindowsPlatform` (`windows.rs`). Cualquier otro
OS hace `panic!` en tiempo de ejecución: solo se soportan Linux y Windows.

`detection.rs` expone `ProgramDetector`, que sondea qué programas hay realmente
instalados (`detect_terminals`, `detect_browsers`, `detect_file_managers`,
`detect_text_editors`) y devuelve un `DetectedPrograms`. Eso es lo que alimenta
el modo `Auto` de `ProgramConfig`.

**Ojo con la sustitución de variables — hay dos métodos y no son
intercambiables:**

- `replace_variables(text, vars)` — para valores que van como argumento real de
  proceso (`Command::arg`). No escapa nada, porque el valor se entrega literal
  al proceso hijo sin pasar por ningún shell.
- `replace_variables_shell_escaped(text, vars)` — **obligatorio** cuando el
  string resultante lo va a interpretar un shell (`bash -c`, `sh -c`,
  `powershell -Command`). Escapa cada valor con `shell_escape_posix()`, que lo
  envuelve en comillas simples y neutraliza las comillas embebidas.

Usar el primero donde va el segundo es una inyección de comandos: el path de un
proyecto lo elige el usuario y puede contener `;`, `&&`, `$()`, backticks.

### `pdf_export/` — exportación de un proyecto a PDF

Un solo archivo, `mod.rs`, con la función `export_project_to_pdf(db, project,
output_path)`. Usa el crate `printpdf` 0.7 y genera un A4 con fuentes built-in
(Helvetica / Helvetica-Bold), sin dependencias externas de renderizado ni
headless browser.

Arma una portada con nombre, descripción, estado y fecha de exportación, y
después vuelca los datos del proyecto. Las constantes de tipografía y color
(`FONT_SIZE_*`, `COLOR_*`) están arriba del archivo.

### `tracking/` — time tracking automático por shell hooks

> ### ⚠️ LEER ESTO ANTES DE TOCAR `tracking/`
>
> **`SocketServer`, `SessionManager` y `TimeAggregator` están implementados y
> tienen tests, pero NO se instancian en producción.** Sus únicas
> instanciaciones en todo el repo están dentro de sus propios `#[cfg(test)]`
> (`socket.rs:317`, `socket.rs:320`, `socket.rs:359`, `session.rs:342`,
> `session.rs:365`, `session.rs:381`). Ningún comando Tauri, ni `main.rs`, los
> construye.
>
> **No hay ningún socket escuchando en `/tmp/gestor-proyectos.sock` cuando la
> app corre.** El `SocketServer` nunca arranca porque nadie lo arranca.
>
> **`get_tracking_status` es un stub.** Devuelve siempre
> `is_tracking: false, project_id: None, project_path: None, elapsed_seconds: 0`
> — valores fijos, no consulta nada. El comentario en
> `commands/mod.rs:1385` lo dice: *"Cuando el socket esté integrado, esto leerá
> del SessionManager"*. Todavía no está integrado.
>
> Si estás debuggeando "el tracking por socket no anda": no está roto, no está
> conectado.

Lo que el módulo **sí** tiene escrito:

| Archivo | Qué contiene |
| ------- | ------------ |
| `config.rs` | Carpeta `.gestor/` por proyecto, con un `config.json` adentro — el mismo patrón que `.git/`. `GestorConfig` guarda `project_id`, `project_name`, `created_at`, `tracking_enabled`. `find_gestor_config()` busca hacia arriba desde un path. **Esta parte sí se usa en producción** vía los comandos `check_tracking_config` y `find_tracking_project`. |
| `session.rs` | Máquina de estados `SessionState` (`Idle` → `Active` → `Paused`), `TrackingSession` con tiempo acumulado y heartbeat, y `SessionManager` que mantiene un `HashMap` de sesiones por path con auto-pausa por inactividad (default: 15 minutos). Solo se ejercita desde sus tests. |
| `socket.rs` | `SocketServer` sobre Unix socket en `/tmp/gestor-proyectos.sock`, con el protocolo `TrackingMessage` (`Enter`, `Exit`, `Heartbeat`, `Status`) y `TrackingResponse`. Solo se ejercita desde sus tests. |
| `aggregator.rs` | `TimeAggregator`, que persiste sesiones en SQLite (`start_session` / `end_session`) y define `TimeTrackingSession` y `TimeStats`. Solo se ejercita desde sus tests: los comandos van directo a `Database`, sin pasar por el agregador. |

#### Qué del tracking SÍ funciona en producción

El tracking que realmente anda es **manual y por comando Tauri**, y no toca ni
el socket ni el `SessionManager`:

- `start_tracking` / `stop_tracking` — crean y cierran filas en SQLite llamando
  directo a `db.create_tracking_session` / `db.end_tracking_session`.
- `get_time_stats` — lee estadísticas desde `db.get_time_stats`.
- `check_tracking_config` / `find_tracking_project` — usan `tracking::config`
  para detectar la carpeta `.gestor/`.
- `start_work_session` / `stop_work_session` / `get_work_session_status` — el
  camino de "sesión de trabajo" introducido en v0.5.1, que mantiene el estado en
  un `ActiveSession` registrado con `.manage()` en `main.rs`. **Este es el que
  hay que mirar si querés saber si hay una sesión activa**, no
  `get_tracking_status`.

En resumen: hay dos caminos de tracking en el repo. Uno funciona
(`work_session` + comandos manuales sobre la BD) y el otro está escrito,
testeado y desconectado (`socket` + `SessionManager` + `TimeAggregator`).

---

## 🔍 ¿Cuándo se usa cada herramienta?

### Durante Desarrollo

```
Escribes código:     TypeScript, SolidJS, TailwindCSS
       ↓
Vite compila:        → JavaScript + CSS (en memoria)
       ↓
Vite sirve:          → Puerto 1420 (con hot reload)
       ↓
Tauri carga:         → WebView carga desde localhost:1420
       ↓
Rust compila:        → Backend en target/debug/
       ↓
Todo junto:          → Ventana nativa funcionando
```

### Durante Build

```
Escribes código:     TypeScript, SolidJS, TailwindCSS
       ↓
Vite compila:        → dist/ (HTML+JS+CSS optimizado)
       ↓
Rust compila:        → target/release/ (optimizado)
       ↓
Tauri empaqueta:     → Embediza dist/ en el ejecutable
       ↓
Resultado:           → gestor-proyectos.deb/AppImage/rpm
```

---

## 🎓 Conceptos Clave

### 1. Vite NO es parte del runtime

**En desarrollo:**
- Vite corre como servidor (puerto 1420)
- Tauri carga UI desde ese servidor
- Hot reload funciona

**En producción:**
- Vite NO está presente
- Solo está el output compilado (dist/)
- Todo carga de archivos locales

### 2. WebView vs Chromium embebido

**Tauri usa WebView del sistema:**
- webkit2gtk en Linux
- Edge WebView2 en Windows
- WKWebView en macOS

**Ventajas:**
- No necesita empaquetar navegador completo
- Usa recursos ya existentes en el sistema
- Actualizaciones automáticas (cuando el sistema actualiza el WebView)

### 3. Rust compila a código nativo

**Backend Rust:**
```rust
pub async fn open_terminal(path: String) -> Result<(), String>
```

**Se compila a:**
- Código máquina nativo (x86_64, ARM, etc.)
- Sin runtime (a diferencia de Node.js)
- Sin garbage collector
- Extremadamente rápido

---

## 📊 Tamaños Reales (Tu Proyecto)

### Frontend compilado (dist/)

```
48 KB total
  ├── 20 KB - index.js  (SolidJS + tu código + Tauri API)
  ├── 13 KB - index.css (TailwindCSS compilado)
  └── 404 bytes - index.html
```

### Backend compilado (debug)

```
248 MB - target/debug/gestor-proyectos
  (Incluye símbolos de debug + info extra)
```

### Backend compilado (release)

```
~5-10 MB - target/release/gestor-proyectos
  (Optimizado, sin debug info)
```

### Ejecutable final empaquetado

```
~10-15 MB - gestor-proyectos.deb
  (Incluye backend + frontend + assets + metadatos)
```

---

## 🚀 Ventajas de esta Arquitectura

1. **Performance**
   - Rust es extremadamente rápido
   - SolidJS es más rápido que React
   - Vite es el build tool más rápido

2. **Tamaño**
   - Ejecutable pequeño (~10 MB)
   - No incluye runtime pesado
   - Usa recursos del sistema

3. **Seguridad**
   - Rust es memory-safe por diseño
   - Tauri tiene sistema de permisos granular
   - No hay Node.js expuesto

4. **Developer Experience**
   - Hot reload instantáneo (Vite)
   - Type safety (TypeScript + Rust)
   - Recompilación automática

5. **Distribución**
   - Un solo archivo para distribuir
   - Múltiples formatos (.deb, .AppImage, .rpm)
   - No requiere instalación de runtimes

---

## ❓ Preguntas Frecuentes

### ¿Por qué no usar Electron?

Electron es más pesado:
- Incluye Chromium completo (~100 MB)
- Incluye Node.js (~30 MB)
- Consume más RAM
- Inicio más lento

Tauri es más moderno y eficiente.

### ¿Puedo reemplazar Vite con Webpack?

Técnicamente sí, pero Vite es:
- Más rápido (10-100x en hot reload)
- Más simple de configurar
- Diseñado para desarrollo moderno
- Recomendado por el equipo de Tauri

### ¿El puerto 1420 debe estar abierto en producción?

**NO.** El puerto 1420 **solo se usa en desarrollo**.

En producción, Tauri carga los archivos desde:
```
tauri://localhost/
```
(Protocolo personalizado, no HTTP real)

### ¿Puedo acceder a la app desde el navegador?

**NO.** Esta es una aplicación nativa, no una app web.

El puerto 1420 es **solo interno** para que Tauri cargue la UI en desarrollo.

---

## 📚 Referencias

- [Tauri Docs](https://tauri.app/v1/guides/)
- [Vite Docs](https://vitejs.dev/)
- [SolidJS Docs](https://www.solidjs.com/)
- [Rust Book](https://doc.rust-lang.org/book/)

---

**Última actualización:** 2025-10-10

