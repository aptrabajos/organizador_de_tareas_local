# 📂 Gestor de Proyectos

Aplicación de escritorio nativa para Linux (Manjaro) que permite gestionar proyectos locales de manera visual y eficiente.

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
```

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

```bash
# Instalar dependencias
npm install

# Modo desarrollo (hot reload)
npm run tauri dev

# Tests Rust
cd src-tauri && cargo test

# Tests Frontend
npm test

# Lint y formato
npm run lint
npm run format
```

## 📦 Build y Distribución

```bash
# Build optimizado para producción
npm run tauri build

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

Desarrollado con ❤️ usando Tauri + SolidJS
