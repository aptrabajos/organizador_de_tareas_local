> **Congelado en v0.3.0.** No refleja el proceso actual de build. Se conserva como registro histórico.

# ✅ CHECKLIST - Build para Windows

**Proyecto:** Gestor de Proyectos v0.3.0
**Fecha de inicio:** 2025-11-22

---

## 📋 FASE 1: Instalación de Herramientas

### Node.js
- [x] Node.js instalado
- [x] Versión verificada: v24.11.1
- [x] npm funcional

**Status:** ✅ **COMPLETADO**

---

### pnpm
- [ ] Ejecutar: `npm install -g pnpm`
- [ ] Verificar: `pnpm --version`
- [ ] Debe mostrar: `9.x.x` o superior

**Comando:**
```powershell
npm install -g pnpm
pnpm --version
```

**Status:** ⏳ **PENDIENTE**

---

### Rust
- [ ] Descargar desde: https://rustup.rs/
- [ ] Ejecutar `rustup-init.exe`
- [ ] Seleccionar opción 1 (instalación por defecto)
- [ ] Esperar a que termine (~5-10 min)
- [ ] **CERRAR Y ABRIR POWERSHELL**
- [ ] Verificar: `rustc --version`
- [ ] Verificar: `cargo --version`
- [ ] Ambos deben mostrar versión 1.75.0+

**Comandos de verificación:**
```powershell
# DESPUÉS de cerrar y abrir PowerShell
rustc --version
cargo --version
rustup --version
```

**Status:** ⏳ **PENDIENTE**

---

### Visual Studio Build Tools
- [ ] Ir a: https://visualstudio.microsoft.com/downloads/
- [ ] Descargar "Build Tools para Visual Studio 2022"
- [ ] Ejecutar instalador
- [ ] Seleccionar workload: "Desarrollo para el escritorio con C++"
- [ ] Verificar componentes marcados:
  - [ ] Windows 10 SDK (o Windows 11 SDK)
  - [ ] MSVC v143 - VS 2022 C++ x64/x86 build tools
- [ ] Click en "Instalar"
- [ ] Esperar instalación (~30-60 min)
- [ ] Reiniciar PowerShell después de instalar
- [ ] Verificar: `where link.exe` (debe mostrar ruta)

**Comando de verificación:**
```powershell
where link.exe
# Debe mostrar algo como:
# C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.x\bin\Hostx64\x64\link.exe
```

**Status:** ⏳ **PENDIENTE**

---

### WebView2 Runtime
- [x] Incluido en Windows 11
- [ ] Si Windows 10, verificar en: `C:\Program Files (x86)\Microsoft\EdgeWebView\Application`
- [ ] Si no existe, descargar de: https://developer.microsoft.com/en-us/microsoft-edge/webview2/

**Status:** ✅ **PROBABLEMENTE YA INSTALADO**

---

## 📋 FASE 2: Preparación del Proyecto

### Dependencias del Proyecto
- [ ] Abrir PowerShell en la carpeta del proyecto
- [ ] Ejecutar: `pnpm install`
- [ ] Esperar a que termine (~5-10 min)
- [ ] Verificar que se creó la carpeta `node_modules`
- [ ] No debe haber errores en la instalación

**Comandos:**
```powershell
cd "c:\Users\Windows10VM\Desktop\Desarrollos\organizador_de_tareas_local"
pnpm install
```

**Status:** ⏳ **PENDIENTE**

---

## 📋 FASE 3: Compilación

### Primera Compilación
- [ ] Ejecutar: `.\build-windows.ps1`
- [ ] Seleccionar "s" para limpiar builds anteriores
- [ ] Seleccionar opción "1" (Build completo)
- [ ] Esperar compilación (~15-20 min primera vez)
- [ ] Sin errores en la compilación
- [ ] Seleccionar "s" para copiar a carpeta distribución

**Comando:**
```powershell
.\build-windows.ps1
```

**Opciones del script:**
```
¿Limpiar builds anteriores? → s
Tipo de build [1-3] → 1
¿Copiar a distribución? → s
```

**Status:** ⏳ **PENDIENTE**

---

### Verificar Artefactos Generados

**Binario EXE:**
- [ ] Archivo existe: `src-tauri\target\release\gestor-proyectos.exe`
- [ ] Tamaño: ~15-18 MB

**Instalador MSI:**
- [ ] Archivo existe: `src-tauri\target\release\bundle\msi\Gestor de Proyectos_0.3.0_x64_es-ES.msi`
- [ ] Tamaño: ~6-8 MB

**Instalador NSIS:**
- [ ] Archivo existe: `src-tauri\target\release\bundle\nsis\Gestor de Proyectos_0.3.0_x64-setup.exe`
- [ ] Tamaño: ~6-8 MB

**Carpeta de Distribución:**
- [ ] Carpeta `distribucion\` creada
- [ ] Contiene los 3 artefactos

**Status:** ⏳ **PENDIENTE**

---

## 📋 FASE 4: Pruebas

### Probar Binario EXE
- [ ] Ejecutar: `.\src-tauri\target\release\gestor-proyectos.exe`
- [ ] Se abre la ventana de la aplicación
- [ ] Aparece el wizard de bienvenida (3 pasos)
- [ ] Se puede navegar el wizard
- [ ] Se puede cerrar el wizard
- [ ] No hay errores en consola

**Status:** ⏳ **PENDIENTE**

---

### Funcionalidad Básica
- [ ] Crear un proyecto de prueba
- [ ] Editar el proyecto
- [ ] Ver el proyecto en la lista
- [ ] Usar la búsqueda
- [ ] Aplicar filtros
- [ ] Marcar como favorito (pin)
- [ ] Cambiar estado del proyecto
- [ ] Eliminar el proyecto

**Status:** ⏳ **PENDIENTE**

---

### Settings y Configuración
- [ ] Abrir Settings (Ctrl+Comma o botón ⚙️)
- [ ] Tab "Programas" → Click en "Detectar Programas"
- [ ] Se muestran programas de Windows detectados
- [ ] Seleccionar una terminal (ej: PowerShell)
- [ ] Seleccionar un navegador (ej: Edge)
- [ ] Guardar configuración
- [ ] Mensaje de éxito aparece

**Status:** ⏳ **PENDIENTE**

---

### Características Avanzadas
- [ ] Crear entrada en el diario (Journal)
- [ ] Crear TODO
- [ ] Marcar TODO como completado
- [ ] Ver Analytics (Ctrl+Shift+A)
- [ ] Cambiar a dark mode (botón ☀️/🌙)
- [ ] Vista de contexto del proyecto (botón 📋)

**Status:** ⏳ **PENDIENTE**

---

### Atajos de Teclado
- [ ] `Ctrl+N` → Abre formulario de nuevo proyecto
- [ ] `Ctrl+F` → Focus en búsqueda
- [ ] `Ctrl+Comma` → Abre Settings
- [ ] `Ctrl+Shift+A` → Toggle Analytics
- [ ] `Ctrl+R` → Recarga proyectos
- [ ] `Escape` → Cierra modal activo

**Status:** ⏳ **PENDIENTE**

---

### Probar Instalador MSI
- [ ] Doble click en `Gestor de Proyectos_0.3.0_x64_es-ES.msi`
- [ ] Wizard de instalación en español
- [ ] Instalación sin errores
- [ ] Programa aparece en menú inicio
- [ ] Ejecutar desde menú inicio
- [ ] Aplicación funciona correctamente
- [ ] Desinstalar desde "Programas y Características"
- [ ] Desinstalación limpia sin errores

**Status:** ⏳ **PENDIENTE**

---

### Probar Instalador NSIS
- [ ] Ejecutar `Gestor de Proyectos_0.3.0_x64-setup.exe`
- [ ] Selector de idioma aparece
- [ ] Seleccionar Español
- [ ] Wizard de instalación funciona
- [ ] Opciones de instalación disponibles
- [ ] Instalación sin errores
- [ ] Acceso directo en escritorio (si se eligió)
- [ ] Ejecutar desde acceso directo
- [ ] Aplicación funciona correctamente

**Status:** ⏳ **PENDIENTE**

---

## 📋 FASE 5: Distribución

### Preparar Paquete de Distribución
- [ ] Crear carpeta: `GestorProyectos-v0.3.0-Windows`
- [ ] Copiar: `Gestor de Proyectos_0.3.0_x64_es-ES.msi`
- [ ] Copiar: `Gestor de Proyectos_0.3.0_x64-setup.exe`
- [ ] Copiar: `gestor-proyectos.exe` (portable)
- [ ] Copiar: `README_WINDOWS.md`
- [ ] Copiar: `MANUAL_USUARIO_WINDOWS.md`
- [ ] Copiar: `LICENSE`
- [ ] Comprimir en ZIP

**Status:** ⏳ **PENDIENTE**

---

### GitHub Release (Opcional)
- [ ] Crear tag: `git tag v0.3.0`
- [ ] Push tag: `git push origin v0.3.0`
- [ ] Ir a GitHub → Releases → New Release
- [ ] Tag version: v0.3.0
- [ ] Título: "Gestor de Proyectos v0.3.0 - Windows"
- [ ] Descripción con changelog
- [ ] Subir instalador MSI
- [ ] Subir instalador NSIS
- [ ] Subir EXE portable
- [ ] Subir README_WINDOWS.md
- [ ] Publicar release

**Status:** ⏳ **PENDIENTE**

---

## 📋 FASE 6: Documentación Final

### Archivos de Documentación
- [x] `LEEME_PRIMERO.md` creado
- [x] `INSTALACION_WINDOWS.md` creado
- [x] `INSTRUCCIONES_BUILD_WINDOWS.md` creado
- [x] `MANUAL_USUARIO_WINDOWS.md` creado
- [x] `README_WINDOWS.md` creado
- [x] `RESUMEN_BUILD_WINDOWS.md` creado
- [x] `LICENSE` creado
- [x] `build-windows.ps1` creado

**Status:** ✅ **COMPLETADO**

---

## 📊 Resumen de Progreso

### Herramientas
- [x] Node.js ✅
- [ ] pnpm ⏳
- [ ] Rust ⏳
- [ ] VS Build Tools ⏳
- [x] WebView2 ✅ (probablemente)

**Progreso:** 2/5 (40%)

---

### Compilación
- [ ] Dependencias instaladas
- [ ] Primera compilación exitosa
- [ ] Artefactos generados
- [ ] Artefactos verificados

**Progreso:** 0/4 (0%)

---

### Pruebas
- [ ] Binario EXE probado
- [ ] Funcionalidad básica verificada
- [ ] Settings funcionando
- [ ] Características avanzadas funcionando
- [ ] Atajos de teclado funcionando
- [ ] MSI probado
- [ ] NSIS probado

**Progreso:** 0/7 (0%)

---

### Distribución
- [ ] Paquete preparado
- [ ] GitHub Release (opcional)

**Progreso:** 0/2 (0%)

---

## 🎯 Próxima Acción Inmediata

**Según este checklist, necesitas:**

1. ✅ **Instalar Rust** (https://rustup.rs/)
2. ✅ **Instalar pnpm** (`npm install -g pnpm`)
3. ✅ **Instalar VS Build Tools** (https://visualstudio.microsoft.com/downloads/)

**Después:**
```powershell
cd "c:\Users\Windows10VM\Desktop\Desarrollos\organizador_de_tareas_local"
pnpm install
.\build-windows.ps1
```

---

## ⏱️ Tiempo Estimado Restante

| Tarea | Tiempo |
|-------|--------|
| Instalar Rust | 5-10 min |
| Instalar pnpm | 2 min |
| Instalar VS Build Tools | 30-60 min |
| Instalar dependencias | 5-10 min |
| Primera compilación | 15-20 min |
| Pruebas | 15-30 min |
| **TOTAL** | **~1.5-2.5 horas** |

---

## 📝 Notas

### Instalaciones Completadas
- Node.js v24.11.1 ✅
- (Agregar aquí conforme completes)

### Problemas Encontrados
- (Registra aquí cualquier problema que encuentres)

### Soluciones Aplicadas
- (Registra aquí las soluciones que funcionaron)

---

## ✨ ¡Ánimo!

**Estás muy cerca de tener tu aplicación compilada y lista para distribuir!**

Solo necesitas instalar las herramientas y seguir el proceso.
Todo está automatizado con el script `build-windows.ps1`.

---

**Última actualización:** 2025-11-22
**Versión:** 0.3.0
**Status general:** 🟡 **EN PROGRESO**
