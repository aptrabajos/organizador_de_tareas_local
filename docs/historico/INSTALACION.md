# Instalación - Gestor de Proyectos

## 🚀 Aplicación compilada y lista para usar en Manjaro Linux

### Archivos generados

1. **Binario ejecutable**: `src-tauri/target/release/gestor-proyectos` (17MB)
2. **Paquete .deb**: `src-tauri/target/release/bundle/deb/Gestor de Proyectos_0.1.0_amd64.deb` (5.6MB)

---

## 📦 Opción 1: Instalar usando el paquete .deb

```bash
# Instalar el paquete .deb (recomendado)
sudo pacman -U "src-tauri/target/release/bundle/deb/Gestor de Proyectos_0.1.0_amd64.deb"

# O usar debtap para convertir a paquete de Arch
debtap -u
debtap "src-tauri/target/release/bundle/deb/Gestor de Proyectos_0.1.0_amd64.deb"
sudo pacman -U gestor-de-proyectos-0.1.0-1-x86_64.pkg.tar.zst
```

---

## 🔧 Opción 2: Ejecutar el binario directamente

```bash
# Hacer ejecutable (si no lo está)
chmod +x src-tauri/target/release/gestor-proyectos

# Ejecutar desde la terminal
./src-tauri/target/release/gestor-proyectos

# O copiar a ~/.local/bin para ejecutar desde cualquier lugar
cp src-tauri/target/release/gestor-proyectos ~/.local/bin/
gestor-proyectos
```

---

## 🎯 Opción 3: Crear un lanzador de aplicación

Crear archivo en `~/.local/share/applications/gestor-proyectos.desktop`:

```desktop
[Desktop Entry]
Name=Gestor de Proyectos
Comment=Aplicación para gestionar proyectos locales
Exec=/home/TU_USUARIO/.local/bin/gestor-proyectos
Icon=folder-projects
Terminal=false
Type=Application
Categories=Development;Utility;
```

Reemplaza `TU_USUARIO` con tu nombre de usuario.

---

## ✨ Características incluidas en esta versión

### Funcionalidades principales:
- ✅ Gestión de proyectos con base de datos SQLite
- ✅ Editor Markdown con preview en tiempo real
- ✅ Checklists interactivas en notas
- ✅ **NUEVO: Sistema de archivos adjuntos (hasta 5MB)**
- ✅ Enlaces a recursos externos (repos, docs, staging, etc.)
- ✅ Analytics con tracking de tiempo
- ✅ Búsqueda de proyectos
- ✅ Backups y sincronización
- ✅ Integración con terminal

### Sistema de archivos adjuntos:
- Subir archivos de hasta 5MB
- Almacenamiento en base de datos SQLite
- Descarga con un click
- Iconos según tipo de archivo
- Eliminación con confirmación
- Limpieza automática al borrar proyectos

---

## 🔄 Recompilar desde código fuente

Si necesitas recompilar:

```bash
# Desarrollo
pnpm run tauri:dev

# Producción
pnpm run tauri:build
```

---

## 📊 Base de datos

La base de datos SQLite se crea automáticamente en:
`~/.local/share/com.gestor.proyectos/gestor_proyectos.db`

---

## 🐛 Solución de problemas

### Puerto 1420 ocupado
```bash
lsof -ti:1420 | xargs kill -9
```

### Limpiar procesos anteriores
```bash
pkill -f "gestor-proyectos"
```

### Ver logs en desarrollo
```bash
pnpm run tauri:dev 2>&1 | tee /tmp/gestor-app.log
```

---

**Versión**: 0.1.0  
**Build**: 2025-10-12
