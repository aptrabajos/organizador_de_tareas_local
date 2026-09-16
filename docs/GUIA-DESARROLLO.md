# 🚀 Guía de Desarrollo - Gestor de Proyectos

## ⚠️ IMPORTANTE: Esta es una Aplicación de Escritorio Nativa

**Gestor de Proyectos** es una **aplicación de escritorio nativa** construida con Tauri, **NO es una aplicación web**.

### ❌ NO HACER:
- ❌ **NO abrir** `http://localhost:1420` en tu navegador
- ❌ **NO intentar** acceder a la aplicación como si fuera una web
- ❌ El puerto 1420 es solo interno para Vite (usado por Tauri)

### ✅ SÍ HACER:
- ✅ **Ejecutar** `pnpm run tauri:dev` - esto abre una **ventana nativa** automáticamente
- ✅ **Usar** la aplicación desde la ventana que se abre (como cualquier app de escritorio)
- ✅ **Ver logs** en la terminal donde ejecutaste el comando

---

## 🖥️ Iniciar la Aplicación (Modo Desarrollo)

### Paso 1: Asegurarse de que no hay instancias corriendo

```bash
# Detener todas las instancias
pkill -f "gestor-proyectos"
pkill -f "tauri dev"

# Liberar el puerto 1420 (si está ocupado)
lsof -ti:1420 | xargs kill -9 2>/dev/null
```

### Paso 2: Iniciar la aplicación

```bash
# Opción A: Ver logs en la terminal (recomendado)
pnpm run tauri:dev

# Opción B: Logs en archivo de fondo
pnpm run tauri:dev > /tmp/gestor-app.log 2>&1 &
```

### Paso 3: Usar la aplicación

**Después de ~5-10 segundos** (compilación inicial), se abrirá automáticamente una **ventana nativa** con la aplicación.

---

## 📊 Entender los Logs

### Logs de Inicio (Secuencia Normal)

```
> tauri dev
     Running BeforeDevCommand (`pnpm run dev`)
> vite
  VITE v6.3.6  ready in 204 ms
  ➜  Local:   http://localhost:1420/    ← IGNORAR, es solo interno

     Running DevCommand (`cargo run --no-default-features --color always --`)
        Info Watching /home/.../.../src-tauri for changes...
   Compiling gestor-proyectos v0.1.0
    Building [=======================> ] 483/484
    Finished `dev` profile in 5.24s
     Running `target/debug/gestor-proyectos`    ← APP INICIADA ✅
```

### Logs del Backend (Rust)

Todos los `println!()` del código Rust aparecen en la terminal:

```rust
// Ejemplo: src-tauri/src/commands/mod.rs línea 145
println!("Generando datos de backup del proyecto ID: {}", project_id);
```

**Salida en terminal:**
```
Generando datos de backup del proyecto ID: 2
Datos de backup generados para: /home/fdsf_BACKUP.md
```

### Ver Logs en Tiempo Real

```bash
# Si iniciaste con logs en archivo:
tail -f /tmp/gestor-app.log

# Si iniciaste normalmente, los logs ya están en tu terminal
```

---

## 🔍 Monitorear la Aplicación

### Verificar que solo hay UNA instancia corriendo

```bash
ps aux | grep "gestor-proyectos" | grep -v grep
```

**Salida esperada:**
```
manjaro+ 207022  4.9  0.7 75456536 248268 ?  Sl  22:05  0:01 target/debug/gestor-proyectos
```

Debe aparecer **solo 1 línea** (1 proceso).

### Verificar procesos relacionados

```bash
ps aux | grep -E "(vite|pnpm|tauri)" | grep -v grep
```

Verás varios procesos relacionados:
- `pnpm run tauri:dev` (gestor principal)
- `tauri dev` (CLI de Tauri)
- `vite` (servidor de desarrollo)
- `gestor-proyectos` (aplicación nativa)

**Esto es normal** - Tauri usa Vite internamente.

---

## 🛠️ Comandos Útiles

### Detener la aplicación

```bash
# Ctrl+C en la terminal donde corre
# O forzar detención:
pkill -f "gestor-proyectos"
```

### Reiniciar la aplicación

```bash
# Detener
pkill -f "gestor-proyectos"
sleep 2

# Iniciar de nuevo
pnpm run tauri:dev
```

### Ver base de datos

```bash
# Ubicación
ls -lh ~/.local/share/gestor-proyectos/projects.db

# Ver proyectos
sqlite3 ~/.local/share/gestor-proyectos/projects.db \
  "SELECT id, name, local_path FROM projects;"
```

### Limpiar todo (reset completo)

```bash
# Detener app
pkill -f "gestor-proyectos"

# Limpiar puerto
lsof -ti:1420 | xargs kill -9 2>/dev/null

# Borrar base de datos (CUIDADO: elimina todos los proyectos)
rm ~/.local/share/gestor-proyectos/projects.db

# Iniciar limpio
pnpm run tauri:dev
```

---

## 🐛 Solución de Problemas

### Problema: "Port 1420 is already in use"

**Causa:** Hay múltiples instancias corriendo o el puerto no se liberó.

**Solución:**
```bash
pkill -f "gestor-proyectos"
lsof -ti:1420 | xargs kill -9 2>/dev/null
pnpm run tauri:dev
```

### Problema: La ventana no se abre

**Verificar que la app está corriendo:**
```bash
ps aux | grep "gestor-proyectos" | grep -v grep
```

Si no aparece nada, revisar logs para ver errores de compilación.

### Problema: Múltiples ventanas se abren

**Causa:** Iniciaste la aplicación varias veces.

**Solución:**
```bash
# Detener todas las instancias
pkill -f "gestor-proyectos"
sleep 2

# Verificar que no hay nada corriendo
ps aux | grep "gestor-proyectos" | grep -v grep

# Iniciar UNA SOLA vez
pnpm run tauri:dev
```

### Problema: Los cambios en el código no se reflejan

**Para Frontend (TypeScript/SolidJS):**
- Hot reload está activado, los cambios se ven automáticamente
- Si no funciona, guarda el archivo de nuevo

**Para Backend (Rust):**
- La aplicación se recompila automáticamente al detectar cambios
- Espera a que termine la compilación (~5 segundos)
- Verás: `Finished \`dev\` profile in X.XXs`

---

## 📝 Agregar Logs al Código

### En Rust (Backend)

```rust
// src-tauri/src/commands/mod.rs
#[tauri::command]
pub async fn mi_comando() -> Result<(), String> {
    println!("🔍 Ejecutando mi_comando");  // ← Aparece en terminal
    
    // Tu lógica aquí
    
    println!("✅ mi_comando completado");
    Ok(())
}
```

### En TypeScript (Frontend)

```typescript
// Los console.log() NO aparecen en la terminal
// Usa las DevTools de la ventana (F12 o Ctrl+Shift+I)
console.log("Debug desde frontend");
```

**Para ver logs del frontend:**
1. Con la aplicación abierta, presiona `F12` o `Ctrl+Shift+I`
2. Se abrirán las DevTools (igual que en un navegador)
3. Ve a la pestaña "Console"

---

## 🎯 Resumen Rápido

```bash
# 1. Instalar dependencias (solo primera vez)
pnpm install

# 2. Iniciar aplicación
pnpm run tauri:dev

# 3. Esperar a que se abra la ventana nativa (~5-10 seg)
# 4. Usar la aplicación desde la ventana
# 5. Ver logs en la terminal

# Para detener: Ctrl+C
```

---

## 📚 Arquitectura Interna (Para Referencia)

```
┌─────────────────────────────────────────┐
│   Ventana Nativa de la Aplicación       │
│   (WebView con tu UI de SolidJS)        │
└──────────────┬──────────────────────────┘
               │
               │ Tauri API (invoke)
               │
┌──────────────▼──────────────────────────┐
│   Backend Rust                          │
│   - Comandos Tauri                      │
│   - Base de datos SQLite                │
│   - Operaciones del sistema             │
└─────────────────────────────────────────┘
```

**Vite (puerto 1420)** es solo usado **internamente** por Tauri para servir el frontend. **No es para acceso web**.

---

## 🔗 Ver También

- `README.md` - Descripción general del proyecto
- `TROUBLESHOOTING.md` - Problemas conocidos y soluciones
- `CLAUDE.md` - Guía para desarrollo con Claude

---

**Última actualización:** 2025-10-10

