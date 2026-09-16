# 📖 Manual de Usuario - Gestor de Proyectos para Windows

**Versión:** 0.3.0
**Plataforma:** Windows 10/11
**Fecha:** Noviembre 2025

---

## 📥 Instalación

### Requisitos del Sistema

- **Sistema Operativo:** Windows 10 (1809 o superior) o Windows 11
- **RAM:** 4 GB mínimo (8 GB recomendado)
- **Espacio en disco:** 100 MB para la aplicación
- **WebView2:** Incluido en Windows 11, descarga automática en Windows 10

### Métodos de Instalación

#### Opción 1: Instalador MSI (Recomendado)

1. **Descarga** el archivo `Gestor de Proyectos_0.3.0_x64_es-ES.msi`
2. **Doble click** en el archivo MSI
3. Sigue el asistente de instalación:
   - Click en **"Siguiente"**
   - Acepta los términos de licencia
   - Selecciona la carpeta de instalación (por defecto: `C:\Program Files\Gestor de Proyectos`)
   - Click en **"Instalar"**
4. Espera a que finalice la instalación
5. Click en **"Finalizar"**

**Ubicación:**
- Programa: `C:\Program Files\Gestor de Proyectos\gestor-proyectos.exe`
- Acceso directo en Menú Inicio
- Datos de usuario: `%APPDATA%\gestor-proyectos\`

#### Opción 2: Instalador NSIS

1. **Descarga** el archivo `Gestor de Proyectos_0.3.0_x64-setup.exe`
2. **Ejecuta** el instalador
3. Selecciona el **idioma** (Español o English)
4. Sigue el asistente:
   - Lee y acepta la licencia
   - Elige la carpeta de instalación
   - Selecciona si quieres crear acceso directo en escritorio
   - Click en **"Instalar"**
5. Click en **"Finalizar"** y marca "Ejecutar Gestor de Proyectos"

#### Opción 3: Portable (EXE sin instalación)

1. **Descarga** el archivo `gestor-proyectos.exe`
2. Copia el EXE a cualquier carpeta de tu preferencia
3. **Doble click** para ejecutar
4. La configuración se guardará en `%APPDATA%\gestor-proyectos\`

**Ventajas:**
- No requiere instalación
- Puedes llevarlo en USB
- No modifica el sistema

**Requisitos:**
- WebView2 debe estar instalado en el sistema

---

## 🚀 Primer Uso

### Pantalla de Bienvenida

Al abrir por primera vez, verás un **wizard de 3 pasos**:

#### Paso 1: Bienvenida
Introducción a las características principales:
- 📁 Gestión completa de proyectos
- 🔗 Enlaces y recursos organizados
- 📊 Analytics y estadísticas
- 📓 Diario y TODOs por proyecto
- ⚙️ Configuración multiplataforma

#### Paso 2: Características
Funcionalidades destacadas:
- 🚀 Abrir terminal en el proyecto
- 📝 Editor Markdown con preview
- 📎 Adjuntar archivos importantes
- 🎨 Dark mode y temas
- 🔍 Búsqueda y filtros
- ⭐ Sistema de favoritos

#### Paso 3: Primeros Pasos
Tutorial rápido para empezar

**Puedes cerrar** este wizard en cualquier momento (botón X arriba a la derecha).
Para volver a verlo: **Settings → Interfaz → Mostrar pantalla de bienvenida**

---

## 📂 Gestión de Proyectos

### Crear un Proyecto

1. Click en **"+ Nuevo Proyecto"** (arriba a la derecha)
2. Completa el formulario:

**Tab "Detalles":**
- **Nombre:** Nombre descriptivo del proyecto
- **Descripción:** Breve resumen del proyecto
- **Ruta local:** Carpeta del proyecto en tu PC
  - Click en **"📁 Seleccionar"** para elegir la carpeta
- **URL de documentación:** (Opcional) Link a la documentación
- **URL de IA/documentación:** (Opcional) Link a docs de IA
- **Link de Drive:** (Opcional) Link a Google Drive/OneDrive
- **Notas:** Editor Markdown con tabs Editar/Preview
  - Soporta títulos, listas, checkboxes, código, etc.
- **Imagen:** (Opcional) Captura de pantalla o logo del proyecto

**Tab "Enlaces":**
- Agregar enlaces externos organizados por tipo:
  - 🗂️ Repositorio (GitHub, GitLab, etc.)
  - 📚 Documentación
  - 🧪 Staging
  - 🚀 Producción
  - 🎨 Diseño
  - 🔌 API
  - 🔗 Otro

3. Click en **"Crear Proyecto"**
4. El proyecto aparecerá en la lista principal

### Editar un Proyecto

1. Click en **"✏️ Editar"** en la tarjeta del proyecto
2. Modifica los campos que desees
3. Click en **"💾 Guardar Cambios"**

### Eliminar un Proyecto

1. Click en **"🗑️ Eliminar"** en la tarjeta del proyecto
2. Confirma la eliminación en el diálogo
3. El proyecto y **todos sus datos relacionados** se eliminarán (journal, TODOs, attachments, etc.)

---

## 🔍 Búsqueda y Filtros

### Barra de Búsqueda

- **Ubicación:** Arriba en el centro
- **Función:** Busca por nombre o descripción de proyecto
- **Atajo:** `Ctrl+F` coloca el foco en la barra de búsqueda
- **Tiempo real:** Los resultados se filtran mientras escribes

### Filtros

**Barra de filtros** debajo de la búsqueda:

1. **Filtro por Estado:**
   - Dropdown con opciones: Todos, Activo, Pausado, Completado, Archivado
   - Cambia el estado desde la tarjeta del proyecto (dropdown en header)

2. **Solo Favoritos:**
   - Checkbox "📌 Solo favoritos"
   - Muestra solo proyectos marcados con pin
   - Para marcar favorito: Click en 📍 en la tarjeta → se convierte en 📌

3. **Contador:**
   - Muestra "X de Y proyectos"
   - Se actualiza dinámicamente según filtros

4. **Limpiar Filtros:**
   - Botón que aparece cuando hay filtros activos
   - Restaura vista completa

---

## 🚀 Trabajar con Proyectos

### Abrir Terminal en el Proyecto

1. Click en **"🚀 Trabajar"** en la tarjeta del proyecto
2. Se abrirá tu terminal configurada en la carpeta del proyecto
3. Por defecto detecta:
   - Windows Terminal (si está instalado)
   - PowerShell
   - CMD

**Configurar terminal:** Settings → Programas → Terminal

### Diario del Proyecto (Journal)

1. Click en **"📓 Diario"** en la tarjeta del proyecto
2. Se abre un modal con:
   - Formulario rápido para crear entradas
   - Lista cronológica de entradas (más recientes primero)

**Crear entrada:**
- Escribe en el campo de texto (soporta Markdown)
- (Opcional) Agrega tags separados por comas: `#bug, #tip, #idea`
- Click en **"Guardar Entrada"**

**Editar entrada:**
- Click en ✏️ junto a la entrada
- Modifica contenido o tags
- Click en ✅ para guardar

**Eliminar entrada:**
- Click en 🗑️ junto a la entrada
- Confirma eliminación

### Lista de Tareas (TODOs)

1. Click en **"✅ Lista de Tareas"** en la tarjeta del proyecto
2. Se abre un modal con:
   - Input para agregar nuevas tareas
   - Lista de tareas pendientes
   - Lista de tareas completadas

**Crear TODO:**
- Escribe en el campo "Nueva tarea..."
- Click en **"Agregar"** o presiona Enter

**Marcar como completada:**
- Click en el checkbox ☐ → se marca como ✅

**Eliminar TODO:**
- Click en 🗑️ junto a la tarea

### Vista Consolidada (Contexto del Proyecto)

1. Click en **"📋 Contexto"** en la tarjeta del proyecto
2. Se abre un modal grande con **5 secciones:**

#### 1. Información del Proyecto
- Nombre, descripción, ruta local
- Tags, estado, última actualización

#### 2. 📓 Diario Reciente
- Últimas 5 entradas del journal
- Con timestamp y tags

#### 3. ✅ Tareas Pendientes
- Solo TODOs sin completar
- Click para ir al gestor completo

#### 4. 🔗 Enlaces
- Todos los enlaces externos
- Click para abrir en navegador

#### 5. 📎 Archivos Adjuntos
- Lista de archivos adjuntos
- Preview de imágenes (thumbnails)
- Información: nombre, tamaño, fecha

**Útil para:** Tener una vista rápida de todo lo relacionado al proyecto.

### Archivos Adjuntos

(Funcionalidad disponible pero UI por implementar en v0.4.0)

---

## 📊 Analytics y Estadísticas

### Acceder al Dashboard

1. Click en **"📊 Estadísticas"** en el header
2. O presiona `Ctrl+Shift+A`

### Métricas Disponibles

**Tarjetas de estadísticas:**
1. **Total de Proyectos:** Cantidad total en la BD
2. **Activos Hoy:** Proyectos abiertos hoy
3. **Tiempo Total:** Horas trabajadas acumuladas
4. **Más Activo:** Proyecto con más actividad

**Timeline de Actividad:**
- Últimas 20 actividades en orden cronológico
- Iconos contextuales por tipo:
  - 🚀 Proyecto abierto
  - ✏️ Proyecto editado
  - 💾 Backup creado
  - Etc.

### Tracking Automático

- Al hacer click en **"🚀 Trabajar"**, se registra automáticamente:
  - Incrementa contador de "veces abierto"
  - Actualiza "última vez abierto"
  - Crea entrada en timeline

---

## 🔗 Integración con Git

### Ver Información de Git

Cada tarjeta de proyecto muestra automáticamente:
- **Rama actual:** Badge con nombre de la rama
- **Archivos modificados:** Contador de cambios
- **Commits:** Badge con ahead/behind respecto a origin

### Acciones Rápidas de Git

**Botones disponibles:**
1. **📁 Stage All:** Agrega todos los cambios (`git add .`)
2. **💾 Commit:** Abre modal para crear commit
3. **⬆️ Push:** Sube commits a origin (`git push`)
4. **⬇️ Pull:** Descarga cambios de origin (`git pull`)
5. **🔗 Abrir Repo:** Abre la URL del repositorio en navegador

### Crear Commit desde la UI

1. Click en **"💾 Commit"**
2. En el modal:
   - Escribe el **mensaje del commit**
   - (Opcional) Marca "Push automático después de commit"
3. Click en **"Crear Commit"**
4. Se ejecuta `git commit` y opcionalmente `git push`
5. Toast de confirmación muestra el resultado

**Nota:** Requiere que el proyecto tenga un repositorio Git inicializado.

---

## ⌨️ Atajos de Teclado

### Atajos Globales (funcionan en toda la aplicación)

| Atajo | Acción |
|-------|--------|
| `Ctrl+N` | Crear nuevo proyecto |
| `Ctrl+F` | Focus en barra de búsqueda |
| `Ctrl+Comma` | Abrir configuración |
| `Ctrl+Shift+A` | Toggle Analytics (mostrar/ocultar) |
| `Ctrl+R` | Recargar lista de proyectos |
| `Escape` | Cerrar modal activo |

### Configurar Atajos

1. Settings → Tab **"⌨️ Atajos"**
2. Toggles para habilitar/deshabilitar cada atajo
3. Los cambios se guardan automáticamente

**Nota:** En macOS usa `Cmd` en lugar de `Ctrl`.

---

## ⚙️ Configuración

### Acceder a Settings

- Click en **"⚙️ Configuración"** en el header
- O presiona `Ctrl+Comma`

### Tab "🖥️ Programas"

Configura qué programas usar para abrir terminal, navegador, explorador de archivos y editor de texto.

**Para cada programa:**

1. **Modo Auto (recomendado):**
   - Detección automática
   - Click en **"🔍 Detectar Programas"** para ver lista de programas encontrados
   - Selecciona uno de la lista

2. **Modo Default:**
   - Usa el predeterminado del sistema
   - Ej: Terminal predeterminada, navegador predeterminado

3. **Modo Custom:**
   - Ruta personalizada al ejecutable
   - Argumentos opcionales (usa variables `{path}`, `{url}`)
   - Ejemplo: `C:\Program Files\Alacritty\alacritty.exe --working-directory {path}`

4. **Modo Script:**
   - Script PowerShell personalizado
   - Variables disponibles: `{path}`, `{url}`, `{file}`
   - Ejemplo:
     ```powershell
     Start-Process -FilePath "wt.exe" -ArgumentList "-d", "{path}"
     ```

**Programas detectables en Windows:**
- **Terminales:** Windows Terminal, PowerShell, CMD, Git Bash
- **Navegadores:** Edge, Chrome, Firefox, Brave, Opera
- **Explorador:** Windows Explorer
- **Editores:** Notepad, Notepad++, VSCode, Sublime Text

### Tab "💾 Backups"

Configuración de backups automáticos.

**Opciones:**
1. **Habilitar backup automático:** Toggle on/off
2. **Intervalo de backup:** Días entre backups (1-30)
3. **Limpieza automática:** Toggle para eliminar backups antiguos
4. **Días de retención:** Cuántos días conservar backups viejos
5. **Carpeta de backups:**
   - Muestra la ruta actual
   - Click en **"📁 Seleccionar"** para cambiar ubicación

**Nota:** La función de backups automáticos se implementará en v0.4.0. Actualmente solo guarda la configuración.

### Tab "🎨 Interfaz"

Personalización de la UI.

**Opciones:**
1. **Tema:**
   - ☀️ Claro
   - 🌙 Oscuro
   - 🔄 Automático (sigue el tema del sistema)

2. **Idioma:**
   - Español
   - English
   - **Nota:** Requiere reiniciar la aplicación

3. **Confirmación al eliminar:**
   - Toggle para pedir confirmación antes de eliminar proyectos

4. **Mostrar pantalla de bienvenida:**
   - Toggle para mostrar el wizard en el próximo inicio

### Tab "🔧 Avanzado"

Configuraciones técnicas.

**Opciones:**
1. **Nivel de log:**
   - Error: Solo errores críticos
   - Warn: Advertencias y errores
   - Info: Información general (recomendado)
   - Debug: Modo depuración
   - Trace: Todo (muy detallado)

2. **Habilitar analytics:**
   - Toggle para tracking de uso (local, no se envía a servidores)

3. **Actualizaciones automáticas:**
   - Toggle para auto-update
   - **Nota:** Disponible en v0.4.0

### Guardar Configuración

- Click en **"💾 Guardar Configuración"** al finalizar
- Mensaje de éxito por 3 segundos
- Los cambios se aplican inmediatamente

### Resetear Configuración

- Click en **"🔄 Resetear a Valores por Defecto"**
- Confirma la acción
- Restaura configuración de fábrica

**Ubicación del archivo de configuración:**
`%APPDATA%\gestor-proyectos\config.json`

---

## 🎨 Tema Oscuro / Claro

### Cambiar Tema Manualmente

- Click en el botón ☀️/🌙 en el header (arriba a la derecha)
- Alterna entre claro y oscuro

### Configurar Tema Permanente

1. Settings → Tab "Interfaz"
2. Selecciona tema:
   - **Claro:** Siempre claro
   - **Oscuro:** Siempre oscuro
   - **Automático:** Sigue el tema del sistema Windows
3. Guardar configuración

---

## 🗂️ Ubicaciones de Archivos

### Archivos de Datos

- **Base de datos:** `%APPDATA%\gestor-proyectos\projects.db`
- **Configuración:** `%APPDATA%\gestor-proyectos\config.json`

**Para acceder rápidamente:**
1. Presiona `Windows+R`
2. Escribe: `%APPDATA%\gestor-proyectos`
3. Enter

### Archivos del Programa

- **Instalación MSI/NSIS:** `C:\Program Files\Gestor de Proyectos\`
- **Portable:** Donde copiaste el EXE

---

## 🆘 Solución de Problemas

### La aplicación no abre

**Causa posible:** WebView2 no instalado

**Solución:**
1. Descarga WebView2 Runtime: https://developer.microsoft.com/en-us/microsoft-edge/webview2/
2. Instala el runtime
3. Reinicia la aplicación

### "No se puede abrir la terminal"

**Causa:** Terminal no configurada o no encontrada

**Solución:**
1. Settings → Programas → Terminal
2. Selecciona **Modo Auto**
3. Click en **"🔍 Detectar Programas"**
4. Selecciona una terminal de la lista
5. Guardar

### "Error al abrir enlace"

**Causa:** Navegador no configurado

**Solución:**
1. Settings → Programas → Navegador
2. Selecciona **Modo Default** (usa el navegador predeterminado de Windows)
3. Guardar

### La búsqueda no encuentra proyectos

**Causa:** Filtros activos o búsqueda muy específica

**Solución:**
1. Click en **"Limpiar filtros"**
2. Borra el texto de búsqueda
3. Verifica que tengas proyectos creados

### No se guardan los cambios

**Causa:** Error en la base de datos

**Solución:**
1. Cierra la aplicación completamente
2. Ve a `%APPDATA%\gestor-proyectos\`
3. Haz backup de `projects.db`
4. Reinicia la aplicación

### Atajos de teclado no funcionan

**Solución:**
1. Settings → Atajos
2. Verifica que los toggles estén habilitados
3. Reinicia la aplicación
4. Si persiste, puede haber conflicto con otra aplicación

---

## 🔄 Actualizar la Aplicación

### Método Manual (v0.3.0)

1. Descarga la nueva versión
2. Desinstala la versión anterior:
   - Settings de Windows → Apps → Gestor de Proyectos → Desinstalar
3. Instala la nueva versión
4. Tus datos se conservan (están en `%APPDATA%`)

### Método Automático (v0.4.0+)

- La aplicación notificará cuando haya actualizaciones
- Click en "Actualizar ahora"
- Descarga e instala automáticamente

---

## 🗑️ Desinstalar

### Instalación MSI/NSIS

1. **Settings de Windows** (Windows+I)
2. **Apps** → **Aplicaciones instaladas**
3. Busca **"Gestor de Proyectos"**
4. Click en los tres puntos → **Desinstalar**
5. Confirma

### Eliminar Datos Completamente

Si también quieres eliminar tus proyectos y configuración:

1. Presiona `Windows+R`
2. Escribe: `%APPDATA%`
3. Elimina la carpeta `gestor-proyectos`

---

## 📞 Soporte

### Reportar Bugs

Si encuentras algún error:

1. Ve a: https://github.com/tu-usuario/gestor-proyectos/issues
2. Click en **"New Issue"**
3. Describe el problema:
   - Qué estabas haciendo
   - Qué esperabas que pasara
   - Qué pasó realmente
   - Captura de pantalla (si aplica)
   - Versión de Windows

### Solicitar Funcionalidades

Para sugerir nuevas características:

1. GitHub Issues → **"New Issue"**
2. Etiqueta: `enhancement`
3. Describe la funcionalidad deseada

### Preguntas Frecuentes

**¿Es gratis?**
Sí, totalmente gratis y open source (licencia MIT).

**¿Se envían mis datos a internet?**
No. Todo se almacena localmente en tu PC. No hay servidores externos.

**¿Puedo usar la app sin internet?**
Sí, funciona 100% offline.

**¿Cuántos proyectos puedo crear?**
Ilimitados (limitado solo por espacio en disco).

**¿Puedo exportar mis proyectos?**
Función de import/export planeada para v0.4.0.

---

## 📄 Licencia

Este software está licenciado bajo la licencia MIT.
Ver archivo `LICENSE` para más detalles.

---

## 🎉 ¡Disfruta Gestor de Proyectos!

Gracias por usar nuestra aplicación. Si te gusta, compártela con otros desarrolladores.

**Versión:** 0.3.0
**Última actualización:** Noviembre 2025
