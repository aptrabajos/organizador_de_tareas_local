# 🚀 Inicio Rápido - Gestor de Proyectos v0.2.1 (Windows)

## 📦 Instalación en 3 Pasos

### Opción A: Con Instalador MSI (Recomendado)

1. **Ejecutar instalador:**
   ```
   Doble click en: Gestor de Proyectos_0.2.1_x64_en-US.msi
   ```

2. **Seguir el asistente**
   - Click en "Next" → "Next" → "Install"

3. **Abrir la aplicación:**
   - Busca "Gestor de Proyectos" en el Menú de Inicio

### Opción B: Con Script PowerShell

1. **Abrir PowerShell como Administrador:**
   - Windows + X → "Windows PowerShell (Admin)"

2. **Navegar a la carpeta:**
   ```powershell
   cd "C:\ruta\a\win10"
   ```

3. **Ejecutar el instalador:**
   ```powershell
   .\scripts\install.ps1
   ```

### Opción C: Compilar desde Código Fuente

1. **Instalar requisitos:**
   - Node.js: https://nodejs.org/
   - Rust: https://www.rust-lang.org/tools/install
   - pnpm: `npm install -g pnpm`

2. **Copiar carpeta `source`** a tu PC

3. **Compilar:**
   ```powershell
   cd source
   pnpm install
   pnpm run tauri:build
   ```

4. **Instalar el MSI generado:**
   ```
   src-tauri\target\release\bundle\msi\Gestor de Proyectos_0.2.1_x64_en-US.msi
   ```

---

## ⚡ Primer Uso

### 1. WelcomeScreen

Al abrir por primera vez:
- Wizard de 3 pasos te guiará
- Click "Siguiente →" para avanzar
- Click "¡Empezar! 🚀" al final

### 2. Configurar Programas

1. Click en **⚙️ Configuración** (arriba a la derecha)
2. Tab **🔧 Programas**
3. Click en **🔍 Detectar Programas**
4. Ajusta si es necesario:
   - **Terminal**: PowerShell, cmd, Windows Terminal
   - **Navegador**: Chrome, Edge, Firefox
   - **Editor**: VSCode, Notepad++
5. Click en **💾 Guardar Configuración**

### 3. Configurar Backups

1. Tab **💾 Backups**
2. Click en **📁 Seleccionar** para elegir carpeta (ej: `D:\Backups\Proyectos`)
3. (Opcional) Activa **Backup Automático**
4. Configura **Intervalo** y **Retención**
5. **💾 Guardar Configuración**

### 4. Crear tu Primer Proyecto

1. Click en **+ Nuevo Proyecto**
2. Completa:
   - **Nombre**: Mi Primer Proyecto
   - **Ruta**: C:\Users\TuNombre\Proyectos\mi-proyecto
   - **Descripción**: Un proyecto de ejemplo
   - **Tags**: ejemplo, test
3. Click en **💾 Crear**

### 5. Trabajar en el Proyecto

1. En la card del proyecto, click en **🚀 Trabajar**
2. Se abrirá PowerShell/cmd en la carpeta del proyecto
3. El sistema registrará automáticamente tu actividad

---

## 🎯 Funciones Clave

| Funcionalidad | Ubicación |
|--------------|-----------|
| **Crear proyecto** | Botón "+ Nuevo Proyecto" |
| **Editar proyecto** | Click en "✏️ Editar" en la card |
| **Eliminar proyecto** | Click en "🗑️" en la card |
| **Favoritos** | Click en "⭐" para marcar/desmarcar |
| **Abrir terminal** | Click en "🚀 Trabajar" |
| **Ver estadísticas** | Botón "📊 Estadísticas" (header) |
| **Configuración** | Botón "⚙️ Configuración" (header) |
| **Buscar** | Barra de búsqueda en el header |
| **Filtros** | Dropdowns de Estado y Tags |

---

## 🔧 Atajos de Teclado (Próximamente en v0.3.0)

Por ahora, todo se opera con mouse/click.

---

## 🐛 Problemas Comunes

### La app no abre

**Solución:**
1. Verifica WebView2:
   - Descarga: https://developer.microsoft.com/en-us/microsoft-edge/webview2/
2. Ejecuta como Administrador

### Terminal no se abre

**Solución:**
1. Configuración → Programas → Terminal
2. Selecciona "Custom"
3. Ruta: `C:\Windows\System32\cmd.exe`
4. Guarda

### Selector de carpeta no funciona

**Solución:**
- Ejecuta la app como Administrador
- O elige carpeta en tu perfil: `C:\Users\TuNombre\Backups`

---

## 📚 Documentación Completa

Ver: **README.md** en esta carpeta

---

## 🆘 Soporte

- Issues: Repositorio GitHub
- Docs: Ver README.md y BUILD_WINDOWS.md

---

**¡Listo para gestionar tus proyectos! 🎉**
