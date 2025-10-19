# 📦 Instrucciones de Distribución para Windows

## 🎯 Propósito de esta Carpeta

Esta carpeta `win10/` contiene todo lo necesario para **distribuir profesionalmente** la aplicación Gestor de Proyectos v0.2.1 a usuarios de Windows 10/11.

---

## 📂 Contenido Preparado

```
win10/
├── README.md              # Guía completa de instalación y uso
├── QUICK_START.md         # Inicio rápido para usuarios finales
├── MANIFEST.txt           # Descripción del paquete
├── DISTRIBUCION.md        # Este archivo
│
├── docs/
│   └── BUILD_WINDOWS.md   # Guía de compilación desde fuente
│
├── scripts/
│   └── install.ps1        # Script automatizado de instalación
│
└── source/                # NOTA: Debes copiar el código fuente aquí
    (Copiar todo el proyecto aquí antes de distribuir)
```

---

## 🚀 Pasos para Preparar la Distribución

### Opción A: Distribución CON Código Fuente (Recomendado para Desarrollo)

1. **Copiar el código fuente:**
   ```bash
   # Desde el directorio raíz del proyecto:
   cp -r src/ src-tauri/ package.json tsconfig.json tailwind.config.js \
         vite.config.ts eslint.config.js .prettierrc.json \
         win10/source/
   ```

2. **Excluir archivos innecesarios:**
   ```bash
   # Eliminar node_modules y carpetas de build:
   rm -rf win10/source/node_modules
   rm -rf win10/source/src-tauri/target
   rm -rf win10/source/dist
   ```

3. **Crear archivo .gitignore en source/:**
   ```
   node_modules/
   target/
   dist/
   .env
   ```

4. **Empaquetar:**
   ```bash
   # Crear archivo ZIP:
   cd ..
   zip -r gestor-proyectos-v0.2.1-windows-source.zip win10/
   ```

### Opción B: Distribución SOLO con Instalador (Más Ligero)

1. **Compilar el instalador MSI primero:**
   ```bash
   # Desde el directorio raíz:
   pnpm run tauri:build
   ```

2. **Copiar el instalador MSI a win10/:**
   ```bash
   cp src-tauri/target/release/bundle/msi/*.msi win10/
   ```

3. **NO incluir la carpeta source/ (opcional)**

4. **Empaquetar:**
   ```bash
   zip -r gestor-proyectos-v0.2.1-windows-installer.zip win10/
   ```

---

## 📋 Checklist Pre-Distribución

### ✅ Archivos Esenciales Incluidos

- [ ] README.md (guía completa)
- [ ] QUICK_START.md (inicio rápido)
- [ ] MANIFEST.txt (descripción del paquete)
- [ ] scripts/install.ps1 (instalador automatizado)
- [ ] docs/BUILD_WINDOWS.md (guía de compilación)

### ✅ Si Incluyes Instalador MSI

- [ ] Instalador MSI compilado y probado
- [ ] Tamaño del MSI ~6 MB
- [ ] Versión correcta en el nombre del archivo

### ✅ Si Incluyes Código Fuente

- [ ] Carpeta source/ con todos los archivos del proyecto
- [ ] node_modules/ eliminado
- [ ] target/ eliminado
- [ ] dist/ eliminado
- [ ] .gitignore creado en source/
- [ ] Archivo LICENSE (si aplica)

### ✅ Verificación Final

- [ ] Todos los archivos .md son legibles
- [ ] Script install.ps1 tiene permisos de ejecución
- [ ] Tamaño total razonable (<50 MB sin source, <500 MB con source)
- [ ] Probado en una máquina Windows limpia

---

## 📤 Métodos de Distribución

### 1. GitHub Releases

```bash
# Crear release en GitHub
git tag v0.2.1
git push origin v0.2.1

# Subir el ZIP como release asset
```

**Archivo a subir:**
- `gestor-proyectos-v0.2.1-windows-installer.zip` (~10 MB)
- O `gestor-proyectos-v0.2.1-windows-source.zip` (~200 MB)

### 2. Google Drive / Dropbox

1. Subir el ZIP a tu almacenamiento en la nube
2. Obtener link de descarga pública
3. Compartir el link + README con los usuarios

### 3. Servidor Propio

```bash
# Subir vía SCP
scp gestor-proyectos-v0.2.1-windows-installer.zip usuario@servidor:/var/www/downloads/

# URL de descarga:
# https://tuservidor.com/downloads/gestor-proyectos-v0.2.1-windows-installer.zip
```

### 4. Email Directo

- Comprimir con máxima compresión: `zip -9`
- Si es >25 MB, usar servicio como WeTransfer

---

## 📝 Mensaje de Distribución Sugerido

Al enviar a usuarios, incluye este mensaje:

```
Hola,

Te comparto Gestor de Proyectos v0.2.1 para Windows.

📦 DESCARGA:
   [Link al ZIP]

📖 INSTALACIÓN:
   1. Descomprime el ZIP
   2. Lee QUICK_START.md para inicio rápido
   3. O lee README.md para guía completa

⚙️ OPCIONES:
   - Instalador MSI: Doble click y seguir asistente
   - Script PowerShell: .\scripts\install.ps1
   - Desde fuente: Ver docs/BUILD_WINDOWS.md

🐛 SOPORTE:
   - Issues: [GitHub Issues URL]
   - Email: [tu email]

¡Disfruta! 🚀
```

---

## 🔐 Consideraciones de Seguridad

### Firmar el Instalador MSI (Opcional pero Recomendado)

Windows SmartScreen puede mostrar advertencias si el instalador no está firmado.

**Para firmar (requiere certificado de código):**

1. Obtener certificado de código de una CA confiable
2. Firmar el MSI:
   ```powershell
   signtool sign /f "certificado.pfx" /p "password" /t "http://timestamp.digicert.com" "Gestor de Proyectos_0.2.1_x64_en-US.msi"
   ```

**Alternativa (sin certificado):**
- Incluir en README.md instrucciones para usuarios sobre advertencias de SmartScreen
- Explicar que es seguro hacer click en "Más información" → "Ejecutar de todas formas"

---

## 📊 Estructura Final Recomendada para Distribución

```
gestor-proyectos-v0.2.1-windows/
│
├── README.md                                  # LEER PRIMERO
├── QUICK_START.md                             # Inicio rápido
├── MANIFEST.txt                               # Contenido del paquete
│
├── Gestor de Proyectos_0.2.1_x64_en-US.msi   # ← Instalador (si incluyes)
│
├── docs/
│   └── BUILD_WINDOWS.md
│
├── scripts/
│   └── install.ps1
│
└── source/                                     # ← Código fuente (opcional)
    ├── src/
    ├── src-tauri/
    ├── package.json
    └── ...
```

---

## 🎯 Próximos Pasos

1. **Decidir qué distribuir:**
   - Solo instalador MSI (más liviano, ~10 MB)
   - Con código fuente (para desarrolladores, ~200 MB)

2. **Preparar archivos:**
   - Si incluyes MSI: Compilar con `pnpm run tauri:build`
   - Si incluyes source: Copiar archivos a `win10/source/`

3. **Empaquetar:**
   ```bash
   zip -r gestor-proyectos-v0.2.1-windows.zip win10/
   ```

4. **Distribuir:**
   - GitHub Releases (recomendado)
   - O método alternativo

5. **Comunicar:**
   - Compartir link + README
   - Ofrecer soporte vía Issues

---

## ✅ ¡Listo para Distribuir!

Esta carpeta `win10/` está **profesionalmente preparada** para ser distribuida a usuarios de Windows.

Solo falta:
1. Compilar el MSI (si aplica)
2. Copiar source (si aplica)
3. Empaquetar en ZIP
4. Compartir!

**¡Buena suerte con la distribución! 🚀**
