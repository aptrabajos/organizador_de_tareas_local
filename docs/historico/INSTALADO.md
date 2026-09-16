# ✅ Gestor de Proyectos - Instalado en Manjaro

## 🎉 La aplicación está instalada y lista para usar

### 📍 Ubicaciones:

- **Binario**: `~/.local/bin/gestor-proyectos`
- **Lanzador**: `~/.local/share/applications/gestor-proyectos.desktop`

---

## 🚀 Cómo abrir la aplicación:

### Opción 1: Desde el menú de aplicaciones (Recomendado)
1. Abre el menú de aplicaciones de Manjaro
2. Busca "**Gestor de Proyectos**"
3. Haz clic para abrir

**Categoría**: Development / Utility

### Opción 2: Desde la terminal
```bash
gestor-proyectos
```

### Opción 3: Con ALT+F2 (KRunner/Lanzador)
1. Presiona `ALT+F2`
2. Escribe: `gestor-proyectos`
3. Presiona Enter

---

## 📊 Base de datos

La aplicación crea automáticamente su base de datos en:
```
~/.local/share/com.gestor.proyectos/gestor_proyectos.db
```

Todos tus proyectos, notas y archivos adjuntos se guardan ahí.

---

## 🔄 Actualizar la aplicación

Cuando recompiles una nueva versión:

```bash
# 1. Recompilar
cd /home/manjarodesktop/2025/configuraciones/gestor_proyecto
pnpm run tauri:build

# 2. Reemplazar binario
cp src-tauri/target/release/gestor-proyectos ~/.local/bin/

# Listo! El lanzador ya apunta al binario actualizado
```

---

## 🗑️ Desinstalar

Si algún día necesitas desinstalar:

```bash
# Eliminar binario
rm ~/.local/bin/gestor-proyectos

# Eliminar lanzador
rm ~/.local/share/applications/gestor-proyectos.desktop

# Actualizar menú
update-desktop-database ~/.local/share/applications/

# (Opcional) Eliminar base de datos y datos
rm -rf ~/.local/share/com.gestor.proyectos/
```

---

## ✨ Características disponibles:

- ✅ Gestión de proyectos
- ✅ Editor Markdown con preview
- ✅ Checklists interactivas
- ✅ **Archivos adjuntos (hasta 5MB)**
- ✅ Enlaces a recursos
- ✅ Analytics y tracking
- ✅ Búsqueda de proyectos
- ✅ Backups y sincronización

---

**Versión instalada**: 0.1.0  
**Fecha de instalación**: 2025-10-13
