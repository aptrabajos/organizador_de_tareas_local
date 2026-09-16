# 🪟 Gestor de Proyectos para Windows

**Versión:** 0.3.0
**Plataforma:** Windows 10/11
**Licencia:** MIT

---

## 📖 Descripción

**Gestor de Proyectos** es una aplicación de escritorio nativa para Windows que te permite organizar y gestionar tus proyectos de desarrollo de manera visual y eficiente.

### ✨ Características Principales

- 📁 **Gestión completa de proyectos** - Crea, edita y organiza tus proyectos
- 🚀 **Integración con terminal** - Abre la terminal en tu proyecto con un click
- 📓 **Diario de proyecto** - Registra notas y bitácoras con Markdown
- ✅ **Sistema de TODOs** - Lista de tareas por proyecto
- 📊 **Analytics** - Estadísticas de uso y tiempo trabajado
- 🔗 **Enlaces organizados** - Repositorios, documentación, staging, producción
- 📎 **Archivos adjuntos** - Adjunta capturas, diagramas o archivos importantes
- 🔍 **Búsqueda avanzada** - Encuentra proyectos rápidamente
- ⭐ **Favoritos** - Marca proyectos importantes con pin
- 🎨 **Dark mode** - Tema claro/oscuro/automático
- ⌨️ **Atajos de teclado** - Trabaja más rápido con shortcuts globales
- 🔧 **Git Integration** - Stage, commit, push/pull desde la UI
- 🌐 **Multiplataforma** - También disponible para Linux

---

## 💾 Descarga e Instalación

### Requisitos del Sistema

- **Sistema Operativo:** Windows 10 (1809+) o Windows 11
- **RAM:** 4 GB mínimo (8 GB recomendado)
- **Espacio:** 100 MB
- **WebView2:** Incluido en Windows 11 (descarga automática en Win 10)

### Métodos de Instalación

#### 📦 Instalador MSI (Recomendado)

**Ideal para:** Instalación tradicional de Windows

1. Descarga `Gestor de Proyectos_0.3.0_x64_es-ES.msi`
2. Doble click en el archivo
3. Sigue el asistente de instalación
4. Busca "Gestor de Proyectos" en el menú inicio

**Características:**
- Instalación estándar de Windows
- Aparece en "Programas y Características"
- Desinstalación limpia
- Instalación silenciosa disponible: `msiexec /i "installer.msi" /quiet`

#### 🎯 Instalador NSIS (Moderno)

**Ideal para:** Usuarios finales que prefieren interfaz moderna

1. Descarga `Gestor de Proyectos_0.3.0_x64-setup.exe`
2. Ejecuta el instalador
3. Selecciona idioma (Español/English)
4. Elige opciones de instalación
5. Acceso directo en escritorio/menú inicio

**Características:**
- Interfaz moderna y personalizable
- Multi-idioma
- Opciones de instalación flexibles

#### 📦 Portable (Sin instalación)

**Ideal para:** Uso en USB o sin permisos de administrador

1. Descarga `gestor-proyectos.exe`
2. Copia a cualquier carpeta
3. Ejecuta directamente

**Características:**
- No requiere instalación
- Portable en USB
- Requiere WebView2 instalado en el sistema

---

## 🚀 Inicio Rápido

### Primera Ejecución

1. **Abre la aplicación** desde el menú inicio o escritorio
2. Verás el **wizard de bienvenida** (3 pasos)
3. Conoce las características principales
4. Click en "¡Empezar! 🚀"

### Crear tu Primer Proyecto

1. Click en **"+ Nuevo Proyecto"**
2. Completa los campos:
   - **Nombre:** Nombre del proyecto
   - **Descripción:** Breve descripción
   - **Ruta local:** Click en 📁 para seleccionar carpeta del proyecto
3. (Opcional) Agrega notas en Markdown
4. Click en **"Crear Proyecto"**

### Trabajar con el Proyecto

- **🚀 Trabajar:** Abre la terminal en la carpeta del proyecto
- **📓 Diario:** Escribe notas y bitácora del proyecto
- **✅ TODOs:** Gestiona tareas pendientes
- **📋 Contexto:** Vista consolidada de todo el proyecto
- **✏️ Editar:** Modifica información del proyecto
- **🗑️ Eliminar:** Elimina el proyecto (con confirmación)

---

## ⌨️ Atajos de Teclado

| Atajo | Acción |
|-------|--------|
| `Ctrl+N` | Nuevo proyecto |
| `Ctrl+F` | Buscar proyectos |
| `Ctrl+Comma` | Abrir configuración |
| `Ctrl+Shift+A` | Mostrar/ocultar analytics |
| `Ctrl+R` | Recargar lista |
| `Escape` | Cerrar modal |

---

## ⚙️ Configuración

### Personalizar Programas

**Settings → Programas**

Configura qué programas usar:
- **Terminal:** Windows Terminal, PowerShell, CMD, Git Bash
- **Navegador:** Edge, Chrome, Firefox, Brave, Opera
- **Explorador de archivos:** Windows Explorer
- **Editor de texto:** Notepad, VSCode, Sublime Text

**4 modos disponibles:**
- **Auto:** Detección automática
- **Default:** Predeterminado del sistema
- **Custom:** Ruta personalizada
- **Script:** Script PowerShell personalizado

### Tema Oscuro/Claro

- Click en ☀️/🌙 en el header para alternar
- O en Settings → Interfaz → Tema

**Opciones:**
- ☀️ Claro
- 🌙 Oscuro
- 🔄 Automático (sigue el tema de Windows)

---

## 📊 Características Avanzadas

### Git Integration

Cada proyecto con repositorio Git muestra:
- **Rama actual**
- **Archivos modificados**
- **Commits ahead/behind**

**Acciones rápidas:**
- 📁 Stage All
- 💾 Crear Commit
- ⬆️ Push
- ⬇️ Pull
- 🔗 Abrir Repo en navegador

### Analytics

**Settings → Estadísticas** o `Ctrl+Shift+A`

Visualiza:
- Total de proyectos
- Proyectos activos hoy
- Tiempo total trabajado
- Proyecto más activo
- Timeline de actividad reciente

### Markdown Editor

Las notas y el diario soportan **GitHub Flavored Markdown**:

```markdown
# Títulos

**Negrita** *Cursiva* `código`

- [ ] Checklist sin marcar
- [x] Checklist marcada

> Citas

\```
Bloques de código
\```
```

### Filtros Avanzados

- **Por estado:** Activo, Pausado, Completado, Archivado
- **Favoritos:** Solo proyectos marcados con pin
- **Búsqueda:** En nombre y descripción
- **Combinables:** Aplica múltiples filtros

---

## 🗂️ Ubicación de Archivos

### Datos del Usuario

```
%APPDATA%\gestor-proyectos\
├── projects.db          # Base de datos SQLite
└── config.json          # Configuración
```

**Acceso rápido:**
- `Windows+R` → `%APPDATA%\gestor-proyectos`

### Archivos del Programa

**Instalación:**
- `C:\Program Files\Gestor de Proyectos\`

**Portable:**
- Donde copiaste el EXE

---

## 🔒 Privacidad y Seguridad

- ✅ **100% Local:** Todos los datos se guardan en tu PC
- ✅ **Sin telemetría:** No enviamos datos a servidores
- ✅ **Open Source:** Código fuente disponible en GitHub
- ✅ **Sin conexión requerida:** Funciona completamente offline
- ✅ **Tus datos, tu control:** Puedes hacer backup de `%APPDATA%\gestor-proyectos\`

---

## 🆘 Soporte y Ayuda

### Documentación

- **Manual completo:** `MANUAL_USUARIO_WINDOWS.md` (incluido en instalación)
- **Guía de desarrollo:** `CLAUDE.md`
- **Arquitectura:** `ARQUITECTURA.md`

### Reportar Problemas

**GitHub Issues:** https://github.com/tu-usuario/gestor-proyectos/issues

**Incluye:**
- Versión de la app (0.3.0)
- Versión de Windows (10/11)
- Pasos para reproducir el error
- Captura de pantalla (si aplica)

### Preguntas Frecuentes

**¿Es gratis?**
Sí, completamente gratis y open source (licencia MIT).

**¿Necesito internet?**
No, funciona 100% offline.

**¿Cuántos proyectos puedo crear?**
Ilimitados (solo limitado por espacio en disco).

**¿Se sincroniza en la nube?**
No en v0.3.0. Función planeada para v0.4.0.

**¿Puedo exportar mis proyectos?**
Función de import/export planeada para v0.4.0.

**¿Los archivos adjuntos tienen límite?**
Sí, 5 MB por archivo.

---

## 🗑️ Desinstalar

### Desde Windows Settings

1. `Windows+I` → **Apps** → **Aplicaciones instaladas**
2. Busca "Gestor de Proyectos"
3. Click en **Desinstalar**

### Eliminar Datos Completamente

Si también quieres eliminar tus proyectos:

1. `Windows+R` → `%APPDATA%`
2. Elimina la carpeta `gestor-proyectos`

---

## 🛠️ Compilar desde el Código Fuente

Si eres desarrollador y quieres compilar el proyecto:

**Ver:** `INSTRUCCIONES_BUILD_WINDOWS.md`

**Requisitos:**
- Node.js 18+
- pnpm
- Rust + Cargo
- Visual Studio Build Tools

**Comando:**
```powershell
.\build-windows.ps1
```

---

## 📄 Licencia

MIT License - Copyright © 2025 Gestor de Proyectos

Ver archivo `LICENSE` para más detalles.

---

## 🙏 Créditos

**Tecnologías utilizadas:**
- [Tauri](https://tauri.app/) - Framework de aplicaciones nativas
- [SolidJS](https://www.solidjs.com/) - Framework reactivo para UI
- [Rust](https://www.rust-lang.org/) - Backend de alto rendimiento
- [SQLite](https://www.sqlite.org/) - Base de datos embebida
- [TailwindCSS](https://tailwindcss.com/) - Framework CSS
- [TypeScript](https://www.typescriptlang.org/) - Tipado estático

---

## 🎉 ¡Gracias por usar Gestor de Proyectos!

Si te gusta la aplicación:
- ⭐ Dale una estrella en GitHub
- 🐛 Reporta bugs para mejorarla
- 💡 Sugiere nuevas características
- 📢 Compártela con otros desarrolladores

---

**Versión:** 0.3.0
**Última actualización:** Noviembre 2025
**Sitio web:** https://github.com/tu-usuario/gestor-proyectos
**Licencia:** MIT
