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
│  │  pub async fn create_backup(...)                  │  │
│  │  pub async fn sync_project(...)                   │  │
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
│   │   └── models/               # Structs de datos
│   │       ├── mod.rs
│   │       └── project.rs
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

