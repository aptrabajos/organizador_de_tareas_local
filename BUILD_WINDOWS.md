# 🪟 Build para Windows - Gestor de Proyectos

Guía completa para compilar el proyecto para Windows desde Linux (Manjaro).

---

## 📋 Requisitos Previos

### 1. Instalar Dependencias para Cross-Compilation

```bash
# Actualizar sistema
sudo pacman -Syu

# Instalar toolchain de Windows
sudo pacman -S mingw-w64-gcc

# Instalar target de Rust para Windows
rustup target add x86_64-pc-windows-gnu
```

### 2. Instalar WiX Toolset (para MSI installer)

WiX solo funciona en Windows, así que para generar MSI necesitas:
- **Opción A**: Compilar en una máquina Windows
- **Opción B**: Usar Wine para ejecutar WiX en Linux
- **Opción C**: Solo generar exe (sin MSI)

```bash
# Instalar Wine (para opción B)
sudo pacman -S wine winetricks
```

---

## 🔨 Compilación

### Opción 1: Compilar solo Binario Windows (.exe)

```bash
# Limpiar builds anteriores
cargo clean --manifest-path src-tauri/Cargo.toml

# Compilar para Windows
cargo build --release --target x86_64-pc-windows-gnu --manifest-path src-tauri/Cargo.toml

# El binario estará en:
# src-tauri/target/x86_64-pc-windows-gnu/release/gestor-proyectos.exe
```

### Opción 2: Build completo con Tauri CLI

```bash
# Configurar target en Cargo.toml
# Agregar en src-tauri/.cargo/config.toml:
[build]
target = "x86_64-pc-windows-gnu"

# Compilar
pnpm run tauri build -- --target x86_64-pc-windows-gnu
```

---

## 🎯 Configuración de tauri.conf.json

El archivo ya está configurado para Windows con:

```json
{
  "bundle": {
    "targets": "all",
    "windows": {
      "wix": {
        "language": "es-ES"
      },
      "nsis": {
        "displayLanguageSelector": true,
        "languages": ["es-ES", "en-US"]
      }
    }
  }
}
```

**Targets disponibles**:
- `wix`: MSI installer (requiere WiX Toolset)
- `nsis`: NSIS installer (más fácil de compilar)
- Solo especificar el ejecutable

---

## 🐛 Solución de Problemas Comunes

### Error: "linker `x86_64-w64-mingw32-gcc` not found"

```bash
# Verificar que mingw esté instalado
pacman -Q mingw-w64-gcc

# Si no está, instalar:
sudo pacman -S mingw-w64-gcc
```

### Error: "could not find native library `sqlite3`"

```bash
# SQLite para Windows se incluye automáticamente con rusqlite
# Verificar en Cargo.toml que rusqlite tenga feature "bundled"
```

### Error: WiX no disponible

```bash
# Opción 1: Compilar sin MSI
pnpm run tauri build -- --target x86_64-pc-windows-gnu --bundles exe

# Opción 2: Usar NSIS en lugar de WiX
# Modificar tauri.conf.json para solo usar NSIS
```

---

## 📦 Archivos Generados

Después de compilar exitosamente, encontrarás:

```
src-tauri/target/x86_64-pc-windows-gnu/release/
├── gestor-proyectos.exe           # Binario ejecutable
└── bundle/
    ├── msi/                        # MSI installer (si WiX funciona)
    │   └── gestor-proyectos_0.2.1_x64.msi
    └── nsis/                       # NSIS installer (alternativa)
        └── gestor-proyectos_0.2.1_x64-setup.exe
```

---

## ✅ Verificación

### Probar en Windows

1. Copiar el `.exe` o installer a una máquina Windows
2. **Primera ejecución**: Verificar que se cree `%APPDATA%/gestor-proyectos/config.json`
3. **Probar detección**: Abrir Settings → Programas → debería detectar:
   - Windows Terminal, PowerShell, CMD
   - Edge, Chrome, Firefox
   - Windows Explorer
   - Notepad, VSCode

4. **Probar funcionalidad básica**:
   - Crear proyecto
   - Abrir terminal (debería usar Windows Terminal si está instalado)
   - Abrir URL (debería usar navegador predeterminado)
   - Settings → guardar configuración

---

## 🚀 Distribución

### Opción A: GitHub Releases

```bash
# Tag de versión
git tag v0.2.1
git push origin v0.2.1

# Subir a releases:
# - gestor-proyectos_0.2.1_x64.msi (Windows)
# - Gestor_de_Proyectos_0.2.1_amd64.deb (Linux)
```

### Opción B: Auto-updater (futuro v0.3.0)

Integrar `tauri-plugin-updater` para actualizaciones automáticas desde GitHub releases.

---

## 📝 Notas Importantes

### Diferencias Linux vs Windows

**Rutas de config**:
- Linux: `~/.config/gestor-proyectos/`
- Windows: `%APPDATA%\gestor-proyectos\`

**Comandos de shell**:
- Linux: bash
- Windows: PowerShell

**Programas detectados**:
- Ver `src-tauri/src/platform/windows.rs` para lista completa
- Ver `src-tauri/src/platform/linux.rs` para lista Linux

### Limitaciones de Cross-Compilation

- **MSI**: Requiere WiX que solo funciona bien en Windows
- **Firma de código**: No se puede firmar desde Linux
- **Testing**: Requiere máquina Windows real para probar completamente

### Recomendación

Para builds de producción de Windows, idealmente:
1. Usar GitHub Actions con runner Windows
2. O compilar en máquina Windows nativa
3. O usar VM Windows con VirtualBox/VMware

---

## 🔗 Referencias

- [Tauri Prerequisites Windows](https://tauri.app/v1/guides/getting-started/prerequisites#windows)
- [Cross-Compilation Guide](https://tauri.app/v1/guides/building/cross-platform)
- [WiX Toolset](https://wixtoolset.org/)
- [NSIS](https://nsis.sourceforge.io/)

---

## ⚡ Quick Start

```bash
# TL;DR: Compilar rápidamente para Windows

# 1. Instalar target
rustup target add x86_64-pc-windows-gnu

# 2. Compilar
pnpm run tauri build -- --target x86_64-pc-windows-gnu

# 3. Buscar ejecutable en:
ls -lh src-tauri/target/x86_64-pc-windows-gnu/release/gestor-proyectos.exe
```

---

**Estado**: ✅ Configuración lista - Listo para compilar
**Versión**: 0.2.1
**Última actualización**: 2025-10-19
