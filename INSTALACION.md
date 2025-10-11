# Guía de Compilación e Instalación

## 📦 Compilación de la Aplicación

### **Requisitos Previos**
- Node.js y pnpm instalados
- Rust y Cargo instalados
- Dependencias del sistema (GTK, webkit2gtk)

### **Compilar en Modo Desarrollo**
```bash
pnpm tauri dev
```

### **Compilar para Producción**
```bash
pnpm tauri build
```

**Resultado:**
- ✅ Ejecutable optimizado: `src-tauri/target/release/gestor-proyectos`
- ✅ Paquete DEB: `src-tauri/target/release/bundle/deb/`
- ✅ Tamaño: ~16 MB
- ✅ Rendimiento: Optimizado para producción

---

## 🚀 Instalación en Manjaro/Arch Linux

### **Opción 1: Instalación Local (Recomendado)**

```bash
# 1. Compilar la aplicación
pnpm tauri build

# 2. Ejecutar el instalador
./install.sh
```

**¿Qué hace el instalador?**
- ✅ Copia el ejecutable a `~/.local/bin/gestor-proyectos`
- ✅ Crea entrada en el menú de aplicaciones
- ✅ Instala el ícono de la aplicación
- ✅ Configura el archivo `.desktop`

**Cómo usar:**
- Busca "Gestor de Proyectos" en tu menú de aplicaciones
- O ejecuta `gestor-proyectos` desde la terminal

---

### **Opción 2: Paquete DEB (Debian/Ubuntu)**

```bash
# Instalar el paquete DEB generado
sudo dpkg -i src-tauri/target/release/bundle/deb/Gestor_de_Proyectos_0.1.0_amd64.deb

# Si hay dependencias faltantes
sudo apt-get install -f
```

---

### **Opción 3: Ejecutable Portable**

```bash
# Simplemente ejecutar el binario directamente
./src-tauri/target/release/gestor-proyectos
```

---

## 🗑️ Desinstalación

### **Si instalaste con install.sh:**
```bash
./uninstall.sh
```

### **Si instalaste el paquete DEB:**
```bash
sudo dpkg -r gestor-de-proyectos
```

### **Eliminar datos de la aplicación:**
```bash
rm -rf ~/.local/share/gestor-proyectos/
```

---

## 📁 Estructura de Archivos Instalados

```
~/.local/
├── bin/
│   └── gestor-proyectos                    # Ejecutable
├── share/
│   ├── applications/
│   │   └── gestor-proyectos.desktop        # Entrada del menú
│   ├── icons/hicolor/256x256/apps/
│   │   └── gestor-proyectos.png            # Ícono de la app
│   └── gestor-proyectos/
│       └── proyectos.db                    # Base de datos (creada al usar la app)
```

---

## 🔧 Solución de Problemas

### **La aplicación no aparece en el menú**
```bash
# Actualizar caché de aplicaciones
update-desktop-database ~/.local/share/applications
```

### **Error: comando no encontrado**
Agrega `~/.local/bin` a tu PATH:
```bash
# En ~/.bashrc o ~/.zshrc
export PATH="$HOME/.local/bin:$PATH"
```

### **Error de permisos al ejecutar**
```bash
chmod +x ~/.local/bin/gestor-proyectos
```

### **Recompilar después de cambios**
```bash
# 1. Limpiar builds anteriores
rm -rf src-tauri/target/release

# 2. Recompilar
pnpm tauri build

# 3. Reinstalar
./install.sh
```

---

## 📦 Distribución de la Aplicación

### **Compartir con otros usuarios**

**1. Compartir el ejecutable:**
```bash
# Copiar solo el ejecutable (16 MB)
cp src-tauri/target/release/gestor-proyectos ~/compartir/
```

**2. Compartir el paquete DEB:**
```bash
# Copiar el .deb (para usuarios de Debian/Ubuntu)
cp src-tauri/target/release/bundle/deb/Gestor_de_Proyectos_0.1.0_amd64.deb ~/compartir/
```

**3. Compartir el proyecto completo:**
```bash
# Clonar desde GitHub
git clone git@github.com:aptrabajos/organizador_de_tareas_local.git
cd organizador_de_tareas_local
pnpm install
pnpm tauri build
./install.sh
```

---

## 🎯 Mejores Prácticas

### **Desarrollo:**
- Usar `pnpm tauri dev` para desarrollo con hot reload
- Usar `./start-app.sh` para inicio limpio sin conflictos de puerto

### **Producción:**
- Compilar con `pnpm tauri build` para optimización máxima
- Instalar con `./install.sh` para integración completa con el sistema
- Crear backup de `proyectos.db` regularmente

### **Actualización:**
```bash
# 1. Actualizar código
git pull origin main

# 2. Actualizar dependencias
pnpm install

# 3. Recompilar e instalar
pnpm tauri build && ./install.sh
```

---

## 📊 Información Técnica

- **Framework UI:** SolidJS + Tailwind CSS
- **Backend:** Rust + Tauri 2.x
- **Base de datos:** SQLite (rusqlite)
- **Bundle size:** ~16 MB
- **Plataforma:** Linux (GTK3/WebKit2)
- **Arquitectura:** x86_64

---

## 🚀 ¡Listo!

Ahora tu aplicación está instalada y disponible en tu sistema Manjaro.
Busca "Gestor de Proyectos" en tu menú de aplicaciones o ejecuta `gestor-proyectos` desde la terminal.

**¡Disfruta tu gestor de proyectos!** 🎉

