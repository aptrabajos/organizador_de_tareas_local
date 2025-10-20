# 🚀 Ejecutable Portable - gestor-proyectos.exe

## 📦 Archivo Incluido

- **gestor-proyectos.exe** (25 MB)
- Compilado con cross-compilation desde Linux usando mingw-w64
- No requiere instalación
- Compatible con Windows 10/11 (64-bit)

---

## ✅ Uso del Ejecutable Portable

### Opción 1: Ejecutar Directamente

1. **Doble click** en `gestor-proyectos.exe`
2. La aplicación se abrirá inmediatamente
3. Los datos se guardarán en:
   ```
   C:\Users\<TuUsuario>\AppData\Local\gestor-proyectos\
   ```

### Opción 2: Crear Carpeta Portable

1. **Crea una carpeta** en tu ubicación preferida:
   ```
   D:\Aplicaciones\GestorProyectos\
   ```

2. **Copia** `gestor-proyectos.exe` a esa carpeta

3. **Ejecuta** el .exe desde ahí

4. **(Opcional)** Crea un acceso directo:
   - Click derecho en gestor-proyectos.exe
   - "Enviar a" → "Escritorio (crear acceso directo)"

---

## ⚠️ Requisitos

### WebView2 Runtime

Windows 10/11 generalmente ya lo tiene instalado. Si no:

1. **Descarga** desde:
   https://developer.microsoft.com/en-us/microsoft-edge/webview2/

2. **Instala** el "Evergreen Standalone Installer"

3. **Reinicia** el .exe

### SmartScreen de Windows

Al ejecutar por primera vez, Windows puede mostrar:

```
"Windows protegió tu PC"
```

**Solución:**
1. Click en **"Más información"**
2. Click en **"Ejecutar de todas formas"**

Esto ocurre porque el .exe no está firmado digitalmente.

---

## 📁 Ubicación de Datos

Cuando ejecutas el portable, los datos se guardan en:

```
C:\Users\<TuUsuario>\AppData\Local\gestor-proyectos\
├── config.json      # Configuración de la app
└── projects.db      # Base de datos SQLite
```

**Para hacer backup:**
- Copia toda la carpeta `C:\Users\<TuUsuario>\AppData\Local\gestor-proyectos\`
- O usa la función de backups automáticos de la app

---

## 🔧 Configuración Inicial

Al abrir la app:

1. **WelcomeScreen** aparecerá (wizard de 3 pasos)
2. Click "Siguiente →" para ver las características
3. Click "¡Empezar! 🚀" al final

Luego:

1. **Click en "⚙️ Configuración"** (arriba a la derecha)
2. **Tab "🔧 Programas"**:
   - Terminal: `C:\Windows\System32\cmd.exe` o PowerShell
   - Navegador: Chrome, Edge, Firefox (detectará automáticamente)
   - Editor: VSCode, Notepad++ (si los tienes)
3. **Tab "💾 Backups"**:
   - Click "📁 Seleccionar" para elegir carpeta de backups
   - Ejemplo: `D:\Backups\Proyectos\`
4. **Tab "🎨 Interfaz"**:
   - Tema: Light / Dark / Auto
   - Idioma: Español / English

---

## 🎯 Ventajas del Ejecutable Portable

✅ **No requiere instalación** - Solo doble click
✅ **No modifica el registro** de Windows
✅ **Fácil de mover** - Copia el .exe a un USB y úsalo en otra PC
✅ **Fácil de desinstalar** - Elimina el .exe y la carpeta de datos
✅ **No requiere permisos** de administrador

---

## 🆚 ¿Portable vs Instalador MSI?

| Característica | Ejecutable Portable | Instalador MSI |
|----------------|---------------------|----------------|
| Requiere instalación | ❌ No | ✅ Sí |
| Tamaño | 25 MB | ~6 MB + instalación |
| Integración con Windows | Básica | Completa |
| Acceso directo en Menú Inicio | ❌ Manual | ✅ Automático |
| Actualizaciones | ❌ Manual | ✅ Automáticas (v0.3.0) |
| Desinstalación | ❌ Borrar .exe | ✅ Panel de Control |

**Recomendación:**
- **Usa Portable** si quieres probarlo rápido o usarlo ocasionalmente
- **Usa MSI** si quieres instalación permanente e integración completa

---

## 🐛 Problemas Comunes

### La app no abre

**Causa 1**: Falta WebView2
- **Solución**: Instala WebView2 Runtime (ver arriba)

**Causa 2**: Antivirus bloqueándolo
- **Solución**: Agrega excepción en tu antivirus para gestor-proyectos.exe

**Causa 3**: Windows SmartScreen
- **Solución**: "Más información" → "Ejecutar de todas formas"

### Error "VCRUNTIME140.dll no encontrado"

**Solución:**
```
https://aka.ms/vs/17/release/vc_redist.x64.exe
```
Instala Visual C++ Redistributable y reinicia.

### La terminal no se abre

**Solución:**
1. Configuración → Programas → Terminal
2. Custom Path: `C:\Windows\System32\cmd.exe`
3. Guarda y prueba de nuevo

---

## 📊 Comparación de Tamaños

- **Ejecutable portable**: 25 MB (único archivo)
- **Código fuente**: 2.1 MB (sin node_modules)
- **Datos de usuario**: ~5-50 MB (depende del uso)

---

## 🔄 Actualizar a Nueva Versión

Cuando salga una nueva versión (ej: v0.3.0):

1. **Haz backup** de tus datos:
   - Copia `C:\Users\<TuUsuario>\AppData\Local\gestor-proyectos\`

2. **Descarga** el nuevo .exe

3. **Reemplaza** el viejo .exe con el nuevo

4. **Ejecuta** - Tus datos se mantendrán

---

## ✨ Características v0.2.1

- ✅ Gestión completa de proyectos
- ✅ Favoritos y búsqueda avanzada
- ✅ Enlaces, diarios, TODOs
- ✅ Analytics y estadísticas
- ✅ Git integration
- ✅ Editor Markdown con preview
- ✅ Backups automáticos configurables
- ✅ Dark mode completo
- ✅ WelcomeScreen para nuevos usuarios

---

## 📚 Más Información

- **README.md**: Guía completa
- **QUICK_START.md**: Inicio rápido
- **Compilar desde fuente**: `source/README_SOURCE.md`

---

**¡Disfruta usando Gestor de Proyectos! 🎉**
