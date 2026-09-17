# 📂 Gestor de Proyectos

**Aplicación de escritorio nativa** para Linux (Manjaro) que permite gestionar proyectos locales de manera visual y eficiente.

## ⚠️ IMPORTANTE

Esta es una **aplicación de escritorio nativa**, **NO una aplicación web**:

- [ ] ✅ Se ejecuta con: `pnpm run tauri:dev`
- [ ] ✅ Se abre automáticamente una **ventana nativa**
- [ ] ❌ **NO acceder** a `http://localhost:1420` desde el navegador
- [ ] 📝 Ver logs en la **terminal** donde ejecutaste el comando

**Para más detalles de desarrollo, ver:** [`docs/GUIA-DESARROLLO.md`](./docs/GUIA-DESARROLLO.md)

## ✨ Características

- 🚀 **Abrir terminal en proyecto**: Click en "Trabajar" abre terminal en la ubicación del proyecto
- 📁 **Gestión completa**: Crear, editar, eliminar y buscar proyectos
- 📝 **Metadata rica**: Nombre, descripción, rutas locales, documentación, enlaces Drive
- 🔍 **Búsqueda rápida**: Filtrado en tiempo real
- 💾 **Base de datos local**: SQLite embebido, sin configuración
- ⚡ **Ultra rápido**: Consumo mínimo de recursos gracias a Tauri

## 🛠️ Stack Tecnológico

### Frontend

- **SolidJS** - Framework reactivo ultrarrápido (sin Virtual DOM)
- **TypeScript** - Type safety
- **TailwindCSS** - Styling moderno
- **Vite** - Build tool y dev server

### Backend

- **Rust** - Lenguaje de sistemas de alto rendimiento
- **Tauri 2.1** - Framework para apps de escritorio nativas
- **SQLite** (rusqlite) - Base de datos embebida
- **Serde** - Serialización JSON

## 📦 Prerequisitos

### Sistema (Manjaro Linux)

```bash
# Instalar Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Dependencias del sistema para Tauri
sudo pacman -S --needed webkit2gtk base-devel curl wget openssl \
  appmenu-gtk-module gtk3 libappindicator-gtk3 librsvg libvips

# Node.js (si no está instalado)
sudo pacman -S nodejs npm

# pnpm: es el UNICO gestor de paquetes de este repo (no usar npm ni yarn)
npm install -g pnpm
```

> **Gestor de paquetes:** este repo usa **pnpm** exclusivamente. El único
> lockfile versionado es `pnpm-lock.yaml`. `package-lock.json` está ignorado.

### Emuladores de terminal soportados

La app detecta automáticamente el terminal instalado:

- konsole (KDE)
- gnome-terminal (GNOME)
- alacritty
- kitty
- xfce4-terminal
- tilix
- xterm (fallback)

## 🚀 Desarrollo

### Iniciar Aplicación (Modo Desarrollo)

**Método 1: Script Automático (Recomendado)**

```bash
# Script que verifica instancias, libera puertos e inicia la app
./start-app.sh
```

**Método 2: Manual**

```bash
# Instalar dependencias (primera vez)
pnpm install

# Iniciar aplicación (se abre ventana nativa)
pnpm run tauri:dev

# Los logs aparecen en esta misma terminal
# La ventana de la app se abre automáticamente en ~5-10 segundos
```

**⚠️ Si el puerto 1420 está ocupado:**

```bash
# Detener todas las instancias
pkill -f "gestor-proyectos"
lsof -ti:1420 | xargs kill -9 2>/dev/null

# Iniciar de nuevo
pnpm run tauri:dev
```

### Testing

```bash
# Tests Frontend
pnpm test

# Tests Rust (Backend)
cd src-tauri && cargo test

# Lint y formato
pnpm run lint
pnpm run format
```

**📖 Para guía detallada de desarrollo, ver:** [`docs/GUIA-DESARROLLO.md`](./docs/GUIA-DESARROLLO.md)

## 📦 Build y Distribución

```bash
# Build optimizado para producción
pnpm run tauri:build

# Genera automáticamente:
# - .deb (Debian/Ubuntu/Manjaro)
# - .AppImage (universal Linux)
# - .rpm (Fedora/openSUSE)
```

Los binarios estarán en `src-tauri/target/release/bundle/`

## 🎯 Uso

1. **Crear proyecto**: Click en "➕ Nuevo Proyecto"
2. **Seleccionar carpetas**: Usa el selector de archivos para elegir ubicación
3. **Trabajar**: Click en "🚀 Trabajar" abre terminal en la ruta del proyecto
4. **Gestionar**: Editar, eliminar o abrir documentación

## 💻 CLI `gestor` (desde la terminal)

Además de la app gráfica hay una **CLI externa** que lee la misma base SQLite:
navegá tus proyectos sin abrir la ventana. Te da un menú interactivo con `fzf` y
comandos directos que te **llevan** a la carpeta del proyecto (`cd` real en tu shell).

```bash
# Menú interactivo: elegís y te deja parado en la carpeta
gestor

# Ir derecho a un proyecto (TAB completa los nombres existentes)
gestor abrir CRM_Multas        # nombre exacto: va derecho
gestor abrir multas            # varios matches: abre el menú ya acotado

# Listar
gestor list                    # nombre + ruta, uno por línea
gestor list --grupos           # agrupado por grupo padre
gestor list --con-papelera     # incluye los borrados

# Dar de alta un proyecto
gestor crear                   # asistente paso a paso, campo por campo
                               # si la carpeta no existe, pregunta si crearla
gestor crear "Mi Proyecto" --ruta ~/proyectos/mi_proyecto   # directo

# Ayuda
gestor ayuda
```

> **Cómo está montada:** el binario vive en `~/.local/bin/gestor` y hay una
> **función de shell** en `~/.bashrc` / `~/.zshrc` que es la que hace el `cd` real
> (un script hijo no puede cambiar el directorio del shell padre).
> Requisitos: `sqlite3` (obligatorio) y `fzf` (opcional, mejora el menú).
>
> **📖 Documentación completa:** [`docs/cli.md`](./docs/cli.md)

## 📁 Estructura del Proyecto

```
gestor_proyecto/
├── src/                    # Frontend SolidJS
│   ├── components/         # Componentes UI
│   ├── stores/            # Estado global
│   ├── services/          # Servicios (Tauri API)
│   └── types/             # Tipos TypeScript
├── src-tauri/             # Backend Rust
│   ├── src/
│   │   ├── main.rs        # Entry point
│   │   ├── db/            # SQLite operations
│   │   ├── models/        # Structs de datos
│   │   ├── commands/      # Comandos Tauri
│   │   └── utils/         # Utilidades
│   └── Cargo.toml         # Dependencias Rust
├── .mcp.json              # MCP servers config
└── CLAUDE.md              # Guía para Claude Code
```

## 🔧 Configuración

### Base de datos

Se crea automáticamente en: `~/.local/share/gestor-proyectos/projects.db`

### Personalización

Edita `src-tauri/tauri.conf.json` para cambiar configuración de la app.

## 📝 Licencia

MIT

## 🤝 Contribuir

Las contribuciones son bienvenidas. Por favor:

1. Fork el proyecto
2. Crea una rama para tu feature (`git checkout -b feature/AmazingFeature`)
3. Commit tus cambios (`git commit -m 'Add: AmazingFeature'`)
4. Push a la rama (`git push origin feature/AmazingFeature`)
5. Abre un Pull Request

## 📸 Screenshots

_(Agregar screenshots cuando la UI esté lista)_

---

## 📚 Documentación Adicional

**📇 Índice completo de toda la documentación:** [`docs/INDEX.md`](./docs/INDEX.md)

- [`ARQUITECTURA.md`](./ARQUITECTURA.md) - **Arquitectura completa del proyecto** (¿Qué es Vite? ¿Cómo funciona todo? ¿Qué hace cada módulo Rust?)
- [`docs/GUIA-DESARROLLO.md`](./docs/GUIA-DESARROLLO.md) - Guía completa de desarrollo
- [`docs/cli.md`](./docs/cli.md) - La CLI `gestor`: menú `fzf`, `abrir`, `crear` y completado con TAB
- [`docs/ESTADO-FUNCIONALIDADES.md`](./docs/ESTADO-FUNCIONALIDADES.md) - Qué está implementado y qué falta
- [`TROUBLESHOOTING.md`](./TROUBLESHOOTING.md) - Problemas conocidos y soluciones
- [`CLAUDE.md`](./CLAUDE.md) - Guía para desarrollo con Claude
- [`docs/sesiones/`](./docs/sesiones/) - Bitácoras de sesiones de trabajo (histórico)
- [`docs/historico/`](./docs/historico/) - Documentación congelada del ciclo de Windows v0.1.0–v0.3.0
- [`start-app.sh`](./start-app.sh) - Script helper para iniciar la app

---

Desarrollado con ❤️ usando Tauri + SolidJS
