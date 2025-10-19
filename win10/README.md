# Gestor de Proyectos v0.2.1 - Windows Installation Guide

## 📦 Distribución para Windows 10/11

**Gestor de Proyectos** es una aplicación de escritorio nativa para gestionar proyectos de desarrollo de software, construida con **Tauri** + **SolidJS**.

---

## 🎯 Características v0.2.1

### ✨ Funcionalidades Principales

- **Gestión completa de proyectos**: Crear, editar, eliminar y organizar proyectos
- **Favoritos**: Sistema de proyectos destacados con reordenamiento
- **Enlaces y recursos**: Organiza URLs, documentación y recursos por proyecto
- **Diario de proyecto**: Bitácora con soporte Markdown y tags
- **TODOs**: Lista de tareas por proyecto con estado (pendiente/completado)
- **Adjuntos**: Guarda archivos pequeños relacionados con cada proyecto
- **Analytics**: Estadísticas de uso, tiempo trabajado y actividad
- **Git Integration**: Visualiza branch, status y commits recientes
- **Editor Markdown**: Preview en tiempo real con GitHub Flavored Markdown
- **Búsqueda avanzada**: Filtra por nombre, descripción, tags, estado

### ⚙️ Configuración Multiplataforma

- **Tab Programas**: Configura terminal, navegador, explorador de archivos y editor
  - Detección automática de programas instalados
  - Modos: Auto, Default, Custom, Script

- **Tab Backups**:
  - Selector de carpeta de destino con diálogo nativo
  - Backup automático configurable (intervalo en días)
  - Limpieza automática de backups antiguos
  - Retención configurable

- **Tab Interfaz**:
  - Tema: Light / Dark / Auto (según sistema)
  - Idioma: Español / English
  - Confirmación antes de eliminar
  - Pantalla de bienvenida para nuevos usuarios

- **Tab Avanzado**:
  - Nivel de logs: Trace / Debug / Info / Warn / Error
  - Analytics local (opcional)
  - Auto-update (preparado para v0.3.0)

### 🎨 Interfaz de Usuario

- **WelcomeScreen**: Wizard de onboarding de 3 pasos para nuevos usuarios
- **Dark Mode completo**: Soporte para tema oscuro en todos los componentes
- **Toggle switches modernos**: UI moderna con Tailwind CSS
- **Responsive**: Adaptable a diferentes resoluciones

---

## 📋 Requisitos del Sistema

### Mínimos
- **OS**: Windows 10 (versión 1809 o superior) / Windows 11
- **RAM**: 512 MB
- **Disco**: 50 MB para la aplicación + espacio para proyectos
- **WebView2**: Requerido (instalación automática si no está presente)

### Recomendados
- **OS**: Windows 11
- **RAM**: 2 GB o más
- **Disco**: 200 MB para aplicación y backups

---

## 🚀 Opciones de Instalación

### Opción 1: Build desde Código Fuente (Recomendado para Desarrollo)

**Requisitos previos:**
- [Node.js](https://nodejs.org/) v18 o superior
- [Rust](https://www.rust-lang.org/tools/install) (rustc 1.70+)
- [pnpm](https://pnpm.io/installation) (package manager)
- [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (generalmente ya instalado en Windows 10/11)

**Pasos:**

1. **Instalar Rust:**
   ```powershell
   # Descargar e instalar desde: https://www.rust-lang.org/tools/install
   # O usar winget:
   winget install Rustlang.Rustup
   ```

2. **Instalar Node.js y pnpm:**
   ```powershell
   # Descargar desde: https://nodejs.org/
   # O usar winget:
   winget install OpenJS.NodeJS.LTS

   # Instalar pnpm globalmente:
   npm install -g pnpm
   ```

3. **Clonar o copiar el código fuente:**
   ```powershell
   # Si tienes git:
   git clone <url-del-repositorio>
   cd gestor_proyecto

   # O copiar la carpeta "source" desde este paquete
   cd source
   ```

4. **Instalar dependencias:**
   ```powershell
   pnpm install
   ```

5. **Compilar la aplicación:**
   ```powershell
   pnpm run tauri:build
   ```

6. **Ubicación del instalador generado:**
   ```
   src-tauri\target\release\bundle\msi\Gestor de Proyectos_0.2.1_x64_en-US.msi
   ```

7. **Instalar:**
   - Doble click en el archivo `.msi`
   - Seguir el asistente de instalación
   - La aplicación se agregará al menú de inicio

### Opción 2: Usar el Instalador MSI (Si está incluido)

Si este paquete incluye un archivo `.msi` pre-compilado:

1. Localiza el archivo `Gestor de Proyectos_0.2.1_x64_en-US.msi`
2. Doble click para iniciar el instalador
3. Sigue las instrucciones del asistente
4. Una vez instalado, busca "Gestor de Proyectos" en el menú de inicio

### Opción 3: Ejecutable Portable (Si está incluido)

Si este paquete incluye un `.exe` standalone:

1. Copia el archivo `gestor-proyectos.exe` a una carpeta de tu elección
2. Doble click para ejecutar
3. La aplicación creará su configuración en:
   ```
   C:\Users\<TuUsuario>\AppData\Local\gestor-proyectos\
   ```

---

## ⚙️ Configuración Post-Instalación

### Primera Ejecución

Al abrir la aplicación por primera vez:

1. **WelcomeScreen** se mostrará automáticamente
2. Navega por los 3 pasos del tutorial
3. Click en "¡Empezar! 🚀"

### Configurar Programas

1. Abre **⚙️ Configuración** (botón superior derecho)
2. En el tab **🔧 Programas**:
   - Click en **🔍 Detectar Programas** para autodetección
   - O configura manualmente:
     - **Terminal**: cmd.exe, PowerShell, Windows Terminal
     - **Navegador**: Chrome, Edge, Firefox
     - **Explorador**: File Explorer (default)
     - **Editor**: VSCode, Notepad++, Sublime

### Configurar Backups

1. En el tab **💾 Backups**:
   - **Click en "📁 Seleccionar"** para elegir carpeta de destino
   - Ejemplo: `D:\Backups\Proyectos\`
   - Activa **Backup Automático** si lo deseas
   - Configura **Intervalo** (ej: 7 días)
   - Activa **Limpiar Backups Antiguos**
   - Configura **Días de Retención** (ej: 30 días)

2. Click en **💾 Guardar Configuración**

### Elegir Tema

1. En el tab **🎨 Interfaz**:
   - **Tema**: Light / Dark / Auto (sigue el tema de Windows)
   - **Idioma**: Español / English

---

## 📖 Uso Básico

### Crear un Proyecto

1. Click en **"+ Nuevo Proyecto"**
2. Completa los campos:
   - **Nombre**: Nombre del proyecto
   - **Descripción**: Breve descripción
   - **Ruta**: Ubicación en disco del proyecto
   - **Tags**: Etiquetas separadas por comas
   - **Estado**: En desarrollo / Pausado / Completado
   - **Notas**: Markdown con preview en tiempo real

### Trabajar en un Proyecto

1. En la card del proyecto, click en **🚀 Trabajar**
2. Se abrirá una terminal en la carpeta del proyecto
3. El sistema registrará automáticamente la actividad (analytics)

### Ver Analytics

1. Click en **📊 Estadísticas** (header)
2. Visualiza:
   - Total de proyectos
   - Proyectos activos hoy
   - Tiempo total trabajado
   - Proyecto más activo
   - Timeline de actividad reciente

---

## 📁 Estructura de Archivos

```
C:\Users\<TuUsuario>\AppData\Local\gestor-proyectos\
├── config.json          # Configuración de la aplicación
└── projects.db          # Base de datos SQLite con todos los datos
```

**⚠️ IMPORTANTE**: Haz backup manual de esta carpeta regularmente o usa la función de backups automáticos.

---

## 🔧 Solución de Problemas

### La aplicación no inicia

**Problema**: Doble click en el exe/acceso directo no hace nada.

**Solución**:
1. Verifica que **WebView2** esté instalado:
   - Descarga desde: https://developer.microsoft.com/en-us/microsoft-edge/webview2/
2. Ejecuta como administrador (click derecho → Ejecutar como administrador)
3. Revisa el **Visor de eventos de Windows** (Event Viewer) en busca de errores

### Error "VCRUNTIME140.dll no encontrado"

**Solución**:
- Instala **Visual C++ Redistributable**:
  - https://aka.ms/vs/17/release/vc_redist.x64.exe

### La terminal no se abre al hacer click en "🚀 Trabajar"

**Solución**:
1. Abre **Configuración → Programas**
2. En **Terminal**, selecciona:
   - **Modo**: Default o Custom
   - **Custom Path**: `C:\Windows\System32\cmd.exe` o ruta a tu terminal preferida
3. Guarda la configuración

### El selector de carpeta de backups no abre

**Solución**:
- Esto puede ocurrir si hay permisos restringidos
- Ejecuta la app como administrador
- O elige una carpeta dentro de tu perfil de usuario (ej: `C:\Users\<TuNombre>\Backups`)

### La aplicación está en inglés

**Solución**:
1. Abre **Configuración → Interfaz**
2. Cambia **Idioma** a "Español"
3. Reinicia la aplicación

---

## 🆕 Próximas Funcionalidades (v0.3.0)

- ⏰ Scheduler de backups automáticos en Rust
- 🔄 Auto-updater funcional con tauri-plugin-updater
- 🌐 Internacionalización completa (i18n)
- 🧪 Tests E2E con Playwright
- 📊 Más estadísticas y gráficos en Analytics
- 🔍 Búsqueda global en diarios y TODOs

---

## 📝 Notas de Versión v0.2.1

### Nuevas Funcionalidades

- **Selector de carpeta nativo** para backups (diálogo de Windows)
- **WelcomeScreen** con wizard de 3 pasos
- **4 tabs de configuración** completos (Programas, Backups, Interfaz, Avanzado)
- **Dark mode** en todos los componentes
- **Toggle switches modernos** con Tailwind CSS

### Mejoras

- Configuración persistente mejorada
- Detección automática de programas optimizada
- UI/UX refinada en toda la aplicación

### Correcciones

- 0 errores de ESLint
- Warnings de Rust solo en código preparado para futuras features

---

## 📞 Soporte y Contribuciones

- **Reporte de bugs**: Crea un issue en el repositorio
- **Sugerencias**: Propón nuevas features en discussions
- **Contribuir**: Pull requests son bienvenidos

---

## 📄 Licencia

Este proyecto está bajo la licencia especificada en el repositorio.

---

## 🙏 Agradecimientos

Construido con:
- [Tauri](https://tauri.app/) - Framework para apps de escritorio
- [SolidJS](https://www.solidjs.com/) - Framework reactivo
- [Rust](https://www.rust-lang.org/) - Backend de alto rendimiento
- [SQLite](https://www.sqlite.org/) - Base de datos embebida
- [Tailwind CSS](https://tailwindcss.com/) - Framework de estilos

---

**¡Disfruta usando Gestor de Proyectos! 🚀**
