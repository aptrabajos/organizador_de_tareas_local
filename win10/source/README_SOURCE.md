# Gestor de Proyectos v0.2.1 - Código Fuente

## 📂 Estructura del Proyecto

```
source/
├── src/                    # Frontend (SolidJS + TypeScript)
│   ├── components/         # Componentes de UI
│   ├── services/          # Lógica de negocio y API calls
│   ├── types/             # Definiciones de TypeScript
│   ├── App.tsx            # Componente principal
│   └── index.tsx          # Entry point
│
├── src-tauri/             # Backend (Rust + Tauri)
│   ├── src/
│   │   ├── commands/      # Comandos Tauri (API bridge)
│   │   ├── config/        # Sistema de configuración
│   │   ├── db/            # Base de datos SQLite
│   │   ├── models/        # Modelos de datos
│   │   ├── platform/      # Abstracción de plataforma (Linux/Windows)
│   │   └── main.rs        # Entry point Rust
│   │
│   ├── Cargo.toml         # Dependencias Rust
│   └── tauri.conf.json    # Configuración Tauri
│
├── package.json           # Dependencias Node.js
├── pnpm-lock.yaml         # Lockfile de pnpm
├── tsconfig.json          # Configuración TypeScript
├── tailwind.config.js     # Configuración Tailwind CSS
├── vite.config.ts         # Configuración Vite (build tool)
├── eslint.config.js       # Configuración ESLint
└── .prettierrc.json       # Configuración Prettier
```

---

## 🔧 Requisitos para Compilar

### Windows

1. **Node.js** v18 o superior
   - Descarga: https://nodejs.org/
   - O con winget: `winget install OpenJS.NodeJS.LTS`

2. **pnpm** (package manager)
   ```powershell
   npm install -g pnpm
   ```

3. **Rust** (rustc 1.70+)
   - Descarga: https://www.rust-lang.org/tools/install
   - O con winget: `winget install Rustlang.Rustup`

4. **WebView2** (generalmente ya instalado en Windows 10/11)
   - Descarga: https://developer.microsoft.com/en-us/microsoft-edge/webview2/

5. **Visual Studio Build Tools** (para linker de Rust)
   - Descarga: https://visualstudio.microsoft.com/downloads/
   - Selecciona: "C++ build tools"

### Linux (Manjaro/Arch)

1. **Node.js** y **pnpm**:
   ```bash
   sudo pacman -S nodejs-lts-iron pnpm
   ```

2. **Rust**:
   ```bash
   sudo pacman -S rust
   ```

3. **Dependencias de Tauri**:
   ```bash
   sudo pacman -S webkit2gtk base-devel curl wget openssl appmenu-gtk-module \
                  gtk3 libappindicator-gtk3 librsvg libvips
   ```

---

## 🚀 Compilación

### 1. Instalar Dependencias

```bash
# En el directorio source/
pnpm install
```

Esto instalará todas las dependencias de Node.js (~200 MB).

### 2. Desarrollo (con hot-reload)

```bash
# Inicia la app en modo desarrollo:
pnpm run tauri:dev
```

- Se abrirá una ventana con la aplicación
- Los cambios en el código se reflejan automáticamente
- Logs aparecen en la terminal

### 3. Build de Producción

```bash
# Compila la versión optimizada:
pnpm run tauri:build
```

**Ubicación de los archivos generados:**

- **Windows:**
  - Ejecutable: `src-tauri\target\release\gestor-proyectos.exe`
  - Instalador MSI: `src-tauri\target\release\bundle\msi\Gestor de Proyectos_0.2.1_x64_en-US.msi`
  - Instalador NSIS: `src-tauri\target\release\bundle\nsis\Gestor de Proyectos_0.2.1_x64-setup.exe`

- **Linux:**
  - Binario: `src-tauri/target/release/gestor-proyectos`
  - Paquete DEB: `src-tauri/target/release/bundle/deb/Gestor de Proyectos_0.2.1_amd64.deb`
  - Paquete RPM: `src-tauri/target/release/bundle/rpm/Gestor de Proyectos-0.2.1-1.x86_64.rpm`

---

## 🧪 Testing

```bash
# Ejecutar tests unitarios:
pnpm test

# Tests en modo watch:
pnpm run test:watch
```

---

## 🎨 Linting y Formato

```bash
# Lint (detectar errores):
pnpm run lint

# Lint con auto-fix:
pnpm run lint:fix

# Formatear código:
pnpm run format
```

---

## 📝 Scripts Disponibles

```json
{
  "tauri:dev": "Desarrollo con hot-reload",
  "tauri:build": "Build de producción",
  "build": "Build del frontend (solo Vite)",
  "lint": "Ejecutar ESLint",
  "lint:fix": "ESLint con auto-fix",
  "format": "Formatear con Prettier",
  "test": "Ejecutar tests con Vitest",
  "test:watch": "Tests en watch mode"
}
```

---

## 🔍 Troubleshooting

### Windows

**Error: "cargo not found"**
- Asegúrate de reiniciar PowerShell después de instalar Rust
- Verifica que Rust esté en el PATH: `cargo --version`

**Error: "link.exe not found"**
- Instala Visual Studio Build Tools con "C++ build tools"
- O instala Visual Studio Community completo

**Error: "WebView2 not found"**
- Descarga e instala: https://developer.microsoft.com/en-us/microsoft-edge/webview2/

### Linux

**Error: "webkit2gtk not found"**
```bash
sudo pacman -S webkit2gtk
```

**Error: "failed to run linuxdeploy"**
- Este error al generar AppImage no es crítico
- Los archivos DEB, RPM y binario se generan correctamente

---

## 📦 Cross-Compilation (Avanzado)

### Compilar para Windows desde Linux

1. **Instalar mingw-w64**:
   ```bash
   sudo pacman -S mingw-w64-gcc
   ```

2. **Agregar target de Windows a Rust**:
   ```bash
   rustup target add x86_64-pc-windows-gnu
   ```

3. **Compilar**:
   ```bash
   cargo tauri build --target x86_64-pc-windows-gnu
   ```

4. **Resultado**:
   - `src-tauri/target/x86_64-pc-windows-gnu/release/gestor-proyectos.exe`

**Nota**: El instalador MSI no se puede generar desde Linux, solo el .exe

---

## 📚 Documentación de Tecnologías

- **Tauri**: https://tauri.app/v2/
- **SolidJS**: https://www.solidjs.com/
- **Rust**: https://doc.rust-lang.org/book/
- **Vite**: https://vite.dev/
- **Tailwind CSS**: https://tailwindcss.com/
- **TypeScript**: https://www.typescriptlang.org/

---

## 🤝 Contribuir

1. Fork el repositorio
2. Crea una rama: `git checkout -b feature/nueva-caracteristica`
3. Haz tus cambios
4. Ejecuta lint y tests: `pnpm run lint:fix && pnpm test`
5. Commit: `git commit -m "feat: descripción de la característica"`
6. Push: `git push origin feature/nueva-caracteristica`
7. Crea un Pull Request

---

## 📄 Licencia

Ver LICENSE en el repositorio principal.

---

**¡Happy Coding! 🚀**
