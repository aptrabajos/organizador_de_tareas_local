# 🎯 INSTRUCCIONES RÁPIDAS - Build para Windows

**Objetivo:** Compilar Gestor de Proyectos v0.3.0 para Windows con instaladores MSI y NSIS.

---

## ✅ Checklist de Prerrequisitos

Antes de compilar, necesitas instalar:

### 1. Node.js
- **Estado:** ✅ Ya instalado (v24.11.1)
- **Acción:** Ninguna

### 2. pnpm
- **Estado:** ❌ NO instalado
- **Acción:** Ejecutar en PowerShell:
  ```powershell
  npm install -g pnpm
  ```

### 3. Rust
- **Estado:** ❌ NO instalado
- **Acción:**
  1. Descarga: https://rustup.rs/
  2. Ejecuta `rustup-init.exe`
  3. Selecciona instalación por defecto (opción 1)
  4. **Cierra y abre PowerShell** después de instalar

### 4. Visual Studio Build Tools
- **Estado:** ⚠️ Desconocido
- **Acción:**
  1. Descarga: https://visualstudio.microsoft.com/es/downloads/
  2. Busca "Build Tools para Visual Studio 2022"
  3. Instala con opciones:
     - ✅ "Desarrollo para el escritorio con C++"
     - ✅ "Windows 10 SDK"
     - ✅ "MSVC v143"
  4. Demora ~30 minutos

### 5. WebView2
- **Estado:** ✅ Probablemente ya instalado (Windows 10/11)
- **Acción:** Ninguna (se descarga automáticamente si falta)

---

## 🚀 Proceso de Compilación (3 Pasos)

### Paso 1: Instalar Herramientas (solo primera vez)

**Tiempo estimado:** 45-60 minutos

```powershell
# 1. Instalar pnpm
npm install -g pnpm

# 2. Instalar Rust (descargar e instalar rustup-init.exe)
# URL: https://rustup.rs/

# 3. Verificar instalaciones (DESPUÉS de cerrar y abrir PowerShell)
node --version      # Debe mostrar v24.11.1
pnpm --version      # Debe mostrar 9.x.x
rustc --version     # Debe mostrar 1.75.0 o superior
cargo --version     # Debe mostrar 1.75.0 o superior
```

### Paso 2: Instalar Dependencias del Proyecto

**Tiempo estimado:** 5-10 minutos

```powershell
# Navegar al proyecto
cd "c:\Users\Windows10VM\Desktop\Desarrollos\organizador_de_tareas_local"

# Instalar dependencias Node
pnpm install
```

### Paso 3: Compilar con Script Automático

**Tiempo estimado:** 15-20 minutos (primera vez), 2-5 minutos (subsecuentes)

```powershell
# Ejecutar script de build
.\build-windows.ps1
```

**El script te preguntará:**

1. **¿Limpiar builds anteriores?** → Recomendado: `s` (primera vez) o `n` (builds rápidos)

2. **Tipo de build:**
   - **Opción 1** (recomendada): Build completo con instaladores MSI y NSIS
   - **Opción 2**: Solo binario EXE (más rápido, para testing)
   - **Opción 3**: Solo frontend (para desarrollo)

3. **¿Copiar a carpeta distribución?** → `s` para fácil acceso a los instaladores

---

## 📦 Resultados Esperados

Después de compilar exitosamente:

### Opción 1: Build Completo

**Artefactos generados:**

```
📁 src-tauri\target\release\
   ├── gestor-proyectos.exe                 (~15-18 MB)
   └── bundle\
       ├── msi\
       │   └── Gestor de Proyectos_0.3.0_x64_es-ES.msi      (~6-8 MB)
       └── nsis\
           └── Gestor de Proyectos_0.3.0_x64-setup.exe      (~6-8 MB)
```

**Si elegiste copiar a distribución:**

```
📁 distribucion\
   ├── gestor-proyectos.exe
   ├── Gestor de Proyectos_0.3.0_x64_es-ES.msi
   └── Gestor de Proyectos_0.3.0_x64-setup.exe
```

### Opción 2: Solo Binario

```
📁 src-tauri\target\release\
   └── gestor-proyectos.exe                 (~15-18 MB)
```

---

## 🧪 Probar la Compilación

### Método 1: Ejecutar el EXE

```powershell
.\src-tauri\target\release\gestor-proyectos.exe
```

**Debe:**
- ✅ Abrir la ventana de la aplicación
- ✅ Crear `%APPDATA%\gestor-proyectos\config.json` en primera ejecución
- ✅ Funcionar todas las características

### Método 2: Instalar el MSI

1. Doble click en `Gestor de Proyectos_0.3.0_x64_es-ES.msi`
2. Seguir el wizard de instalación
3. Buscar "Gestor de Proyectos" en el menú inicio
4. Ejecutar

### Método 3: Instalar con NSIS

1. Ejecutar `Gestor de Proyectos_0.3.0_x64-setup.exe`
2. Seleccionar idioma (Español/English)
3. Seguir instalación
4. Ejecutar desde escritorio o menú inicio

---

## 🐛 Solución de Problemas Rápidos

### Error: "pnpm: command not found"

```powershell
# Instalar pnpm
npm install -g pnpm

# Cerrar y abrir PowerShell
# Verificar
pnpm --version
```

### Error: "rustc: command not found"

**Causa:** Rust no instalado o PowerShell no recargado

**Solución:**
1. Instalar Rust: https://rustup.rs/
2. **IMPORTANTE:** Cerrar y abrir PowerShell
3. Verificar: `rustc --version`

### Error: "linking with `link.exe` failed"

**Causa:** Visual Studio Build Tools no instalado

**Solución:**
1. Instalar Visual Studio Build Tools (ver Paso 1)
2. Reiniciar PowerShell
3. Verificar: `where link.exe` (debe mostrar ruta)

### Error: "failed to bundle project"

**Solución:**
```powershell
# Limpiar builds anteriores
cargo clean --manifest-path src-tauri\Cargo.toml

# Rebuild
.\build-windows.ps1
```

### Compilación muy lenta

**Normal:**
- Primera compilación: 10-20 minutos
- Compilaciones siguientes: 2-5 minutos

**Optimización:**
```powershell
# Solo compilar binario (sin instaladores)
cargo build --release --manifest-path src-tauri\Cargo.toml
```

---

## 📋 Checklist Final

Antes de distribuir, verifica:

```
[ ] Instaladores generados correctamente (MSI y NSIS)
[ ] Binario EXE funciona standalone
[ ] Instalador MSI instala y desinstala limpiamente
[ ] Instalador NSIS funciona y muestra idioma español
[ ] Aplicación abre sin errores
[ ] Settings → Programas detecta programas de Windows
[ ] Crear/editar/eliminar proyectos funciona
[ ] Git integration funciona (si hay repo git)
[ ] Atajos de teclado funcionan (Ctrl+N, Ctrl+F, etc.)
[ ] Dark mode funciona
[ ] Todas las características principales funcionan
```

---

## 🚀 Distribución

### Opción A: GitHub Releases

```powershell
# 1. Crear tag
git tag v0.3.0
git push origin v0.3.0

# 2. En GitHub:
# - Ir a Releases → New Release
# - Tag: v0.3.0
# - Título: "Gestor de Proyectos v0.3.0 - Windows"
# - Subir archivos:
#   * Gestor de Proyectos_0.3.0_x64_es-ES.msi
#   * Gestor de Proyectos_0.3.0_x64-setup.exe
#   * (Opcional) gestor-proyectos.exe (portable)
```

### Opción B: Compartir Directamente

**Para usuarios finales:**
- **Recomendado MSI:** Más familiar para usuarios corporativos
- **Recomendado NSIS:** Interfaz moderna, multi-idioma
- **Portable EXE:** Para usuarios que no quieren instalar

---

## 📞 ¿Necesitas Ayuda?

### Problemas con la instalación de herramientas
→ Ver: `INSTALACION_WINDOWS.md`

### Problemas con la compilación
→ Revisar logs del script `build-windows.ps1`

### Preguntas sobre el proyecto
→ Ver: `CLAUDE.md` y `ARQUITECTURA.md`

### Manual de usuario final
→ Ver: `MANUAL_USUARIO_WINDOWS.md`

---

## ⏱️ Resumen de Tiempos

| Tarea | Primera Vez | Subsecuente |
|-------|-------------|-------------|
| Instalar herramientas | 45-60 min | - |
| Instalar dependencias | 5-10 min | - |
| Compilación completa | 15-20 min | 2-5 min |
| **TOTAL** | **~1-1.5 horas** | **2-5 min** |

---

## ✨ ¡Listo para Compilar!

**Ejecuta:**

```powershell
.\build-windows.ps1
```

**Y sigue las instrucciones del script.**

---

**Versión:** 0.3.0
**Fecha:** 2025-11-22
**Status:** ✅ Listo para compilar
