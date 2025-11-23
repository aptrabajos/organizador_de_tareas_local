# 🪟 Instalación de Herramientas para Compilar en Windows

Guía paso a paso para instalar todo lo necesario para compilar **Gestor de Proyectos** en Windows 10/11.

---

## 📋 Requisitos Previos

### 1. Node.js (ya instalado ✅)

Tienes Node.js v24.11.1 instalado. Perfecto.

### 2. Instalar pnpm

```powershell
# Opción A: Via npm (ya tienes npm con Node.js)
npm install -g pnpm

# Verificar instalación
pnpm --version
```

### 3. Instalar Rust y Cargo

**Descargar e instalar desde el sitio oficial:**

1. Ve a: https://rustup.rs/
2. Descarga `rustup-init.exe`
3. Ejecuta el instalador
4. Selecciona la instalación por defecto (opción 1)
5. Espera a que descargue e instale Rust

**Verificar instalación:**

```powershell
# Cerrar y abrir PowerShell (para recargar PATH)
rustc --version
cargo --version
rustup --version
```

**Resultado esperado:**
```
rustc 1.75.0 (o superior)
cargo 1.75.0 (o superior)
rustup 1.26.0 (o superior)
```

### 4. Instalar Visual Studio Build Tools

Tauri requiere las herramientas de compilación de C++ de Visual Studio.

**Opción A: Visual Studio Build Tools (recomendado, ~7 GB):**

1. Descarga: https://visualstudio.microsoft.com/es/downloads/
2. Busca "Build Tools para Visual Studio 2022"
3. Ejecuta el instalador
4. Selecciona:
   - ✅ "Desarrollo para el escritorio con C++"
   - ✅ "Windows 10 SDK" (o 11 SDK)
   - ✅ "MSVC v143 - VS 2022 C++ x64/x86 build tools"
5. Instala (demora ~30 minutos)

**Opción B: Visual Studio Community (completo, ~10 GB):**

Si prefieres el IDE completo con todas las herramientas.

### 5. Instalar WebView2 (probablemente ya lo tienes)

Windows 10/11 modernos ya incluyen WebView2. Verificar:

1. Abre: `C:\Program Files (x86)\Microsoft\EdgeWebView\Application`
2. Si existe, ya lo tienes ✅

Si no lo tienes:
- Descarga: https://developer.microsoft.com/en-us/microsoft-edge/webview2/

---

## 🔨 Compilación del Proyecto

### Paso 1: Instalar dependencias del proyecto

```powershell
cd "c:\Users\Windows10VM\Desktop\Desarrollos\organizador_de_tareas_local"

# Instalar dependencias Node
pnpm install
```

### Paso 2: Build de Frontend

```powershell
# Compilar el frontend (SolidJS)
pnpm run build
```

### Paso 3: Build completo (Frontend + Backend)

```powershell
# Build completo para Windows
pnpm run tauri build
```

**Esto generará:**
- Binario EXE: `src-tauri\target\release\gestor-proyectos.exe`
- Instalador MSI: `src-tauri\target\release\bundle\msi\Gestor de Proyectos_0.3.0_x64_en-US.msi`
- Instalador NSIS: `src-tauri\target\release\bundle\nsis\Gestor de Proyectos_0.3.0_x64-setup.exe`

---

## 🎯 Opcional: Instaladores con Firma Digital

Para distribución profesional, puedes firmar los instaladores:

### Obtener Certificado de Firma de Código

1. Comprar certificado de una CA (ej: DigiCert, Sectigo)
2. O usar certificado auto-firmado para testing

### Configurar en tauri.conf.json

```json
{
  "bundle": {
    "windows": {
      "certificateThumbprint": "TU_THUMBPRINT_AQUI",
      "digestAlgorithm": "sha256",
      "timestampUrl": "http://timestamp.digicert.com"
    }
  }
}
```

---

## 📦 Instaladores Generados

### MSI Installer (Windows Installer)

**Ubicación:** `src-tauri\target\release\bundle\msi\Gestor de Proyectos_0.3.0_x64_en-US.msi`

**Características:**
- Instalación estándar de Windows
- Aparece en "Programas y Características"
- Desinstalación limpia
- Tamaño: ~6-8 MB

**Instalación:**
- Doble click en el MSI
- Siguiente, Siguiente, Instalar
- Ubicación por defecto: `C:\Program Files\Gestor de Proyectos\`

### NSIS Installer (Nullsoft Scriptable Install System)

**Ubicación:** `src-tauri\target\release\bundle\nsis\Gestor de Proyectos_0.3.0_x64-setup.exe`

**Características:**
- Instalador moderno y personalizable
- Más opciones de configuración
- Tamaño: ~6-8 MB

**Instalación:**
- Ejecutar el setup.exe
- Seleccionar idioma (Español/English)
- Elegir carpeta de instalación
- Crear acceso directo en escritorio/menú inicio

---

## ✅ Verificación Post-Instalación

### Probar Binario EXE

```powershell
# Ejecutar directamente el binario
.\src-tauri\target\release\gestor-proyectos.exe
```

### Probar Instalador MSI

1. Doble click en el MSI
2. Instalar
3. Buscar "Gestor de Proyectos" en el menú inicio
4. Ejecutar la aplicación

### Verificar Configuración

1. Primera ejecución debería crear: `%APPDATA%\gestor-proyectos\config.json`
2. Abrir Settings → Programas
3. Verificar que detecta:
   - Windows Terminal / PowerShell / CMD
   - Edge / Chrome / Firefox
   - Windows Explorer
   - Notepad / VSCode

---

## 🐛 Solución de Problemas

### Error: "npm ERR! peer dep missing"

```powershell
# Limpiar cache de pnpm
pnpm store prune

# Reinstalar dependencias
rm -rf node_modules
pnpm install
```

### Error: "linking with `link.exe` failed"

**Causa:** Visual Studio Build Tools no instalado o no en PATH

**Solución:**
1. Instalar Visual Studio Build Tools (ver arriba)
2. Reiniciar PowerShell
3. Verificar: `where link.exe` debe mostrar la ruta

### Error: "WebView2 not found"

```powershell
# Descargar e instalar WebView2 Runtime
# https://developer.microsoft.com/en-us/microsoft-edge/webview2/
```

### Error: "failed to bundle project"

```powershell
# Limpiar builds anteriores
cargo clean --manifest-path src-tauri\Cargo.toml

# Rebuild
pnpm run tauri build
```

### Compilación muy lenta

**Primera compilación:** 10-15 minutos (normal, compila todas las dependencias)
**Compilaciones subsecuentes:** 2-5 minutos

**Optimización:**
```powershell
# Compilar solo el binario (sin instaladores)
cargo build --release --manifest-path src-tauri\Cargo.toml
```

---

## 📊 Tamaños Esperados

| Archivo | Tamaño Aproximado |
|---------|-------------------|
| `gestor-proyectos.exe` | ~15-18 MB |
| MSI Installer | ~6-8 MB |
| NSIS Setup | ~6-8 MB |
| Carpeta `target/release` completa | ~2-3 GB |

---

## 🚀 Distribución

### GitHub Releases

```powershell
# 1. Tag de versión
git tag v0.3.0
git push origin v0.3.0

# 2. Crear release en GitHub
# 3. Subir artefactos:
#    - Gestor de Proyectos_0.3.0_x64_en-US.msi
#    - Gestor de Proyectos_0.3.0_x64-setup.exe
```

### Compartir con usuarios finales

**Opción A: MSI (recomendado para usuarios corporativos)**
- Más familiar para administradores de sistemas
- Instalación silenciosa: `msiexec /i "Gestor de Proyectos_0.3.0_x64_en-US.msi" /quiet`

**Opción B: NSIS Setup (recomendado para usuarios finales)**
- Interfaz más moderna
- Más opciones de personalización
- Detección de idioma automática

**Opción C: Portable (EXE standalone)**
- Copiar `gestor-proyectos.exe` a cualquier carpeta
- No requiere instalación
- Requiere WebView2 instalado en el sistema

---

## 🔗 Enlaces Útiles

- **Rust para Windows:** https://www.rust-lang.org/tools/install
- **Tauri Prerequisites:** https://tauri.app/v1/guides/getting-started/prerequisites#windows
- **Visual Studio Build Tools:** https://visualstudio.microsoft.com/downloads/
- **WebView2:** https://developer.microsoft.com/en-us/microsoft-edge/webview2/
- **NSIS:** https://nsis.sourceforge.io/
- **WiX Toolset:** https://wixtoolset.org/

---

## ⏱️ Tiempo Estimado

| Tarea | Tiempo |
|-------|--------|
| Instalar Rust | 5-10 min |
| Instalar Visual Studio Build Tools | 30-60 min |
| Instalar pnpm | 2 min |
| `pnpm install` | 5-10 min |
| Primera compilación (`pnpm run tauri build`) | 15-20 min |
| **TOTAL** | **~1-2 horas** |

Compilaciones subsecuentes: **2-5 minutos**

---

## 📝 Checklist de Instalación

```
[ ] Node.js instalado (v18+)
[ ] pnpm instalado
[ ] Rust instalado (rustc, cargo, rustup)
[ ] Visual Studio Build Tools instalado
[ ] WebView2 instalado
[ ] Dependencias del proyecto instaladas (pnpm install)
[ ] Build exitoso (pnpm run tauri build)
[ ] MSI generado en src-tauri/target/release/bundle/msi/
[ ] NSIS generado en src-tauri/target/release/bundle/nsis/
[ ] Aplicación probada y funcional
```

---

**Estado:** 📝 Guía completa - Listo para instalar y compilar
**Versión del proyecto:** 0.3.0
**Fecha:** 2025-11-22
