# 🔧 Troubleshooting - Gestor de Proyectos

Este documento registra problemas encontrados durante el desarrollo y sus soluciones.

---

## 🌐 Problema: Los enlaces (URLs) no abren en el navegador

### Descripción del problema
Los botones de "Documentación", "Docs IA" y "Drive" no abrían las URLs en el navegador del sistema cuando se hacía clic en ellos.

### Causa raíz
El plugin `tauri-plugin-shell` con su función `open()` no funcionaba correctamente en Tauri 2.1 debido a:

1. **Configuración incorrecta de permisos** en `tauri.conf.json`
2. **Sintaxis no válida** intentando usar campo `scope` que no existe en Tauri 2.x
3. **Dependencia del plugin shell** cuando se puede implementar directamente con comandos Rust

### Intentos fallidos

#### ❌ Intento 1: Usar `<a>` con `preventDefault()`
```tsx
<a href={url} onClick={(e) => { e.preventDefault(); openUrl(url); }}>
```
**Problema:** El `preventDefault()` bloqueaba la navegación nativa y la función `openUrl()` no ejecutaba.

#### ❌ Intento 2: Usar `<a>` sin `preventDefault()`
```tsx
<a href={url} onClick={() => openUrl(url)}>
```
**Problema:** El plugin shell seguía sin funcionar.

#### ❌ Intento 3: Agregar fallback con `window.open()`
```tsx
onClick={() => {
  openUrl(url).catch(() => window.open(url, '_blank'));
}}
```
**Problema:** En aplicaciones Tauri, `window.open()` no funciona igual que en navegadores web.

#### ❌ Intento 4: Configurar scope en tauri.conf.json
```json
"plugins": {
  "shell": {
    "open": true,
    "scope": [...]  // ❌ Campo inválido
  }
}
```
**Problema:** Error de deserialización - el campo `scope` no existe para el plugin shell en Tauri 2.x.

### ✅ Solución final: Comando Tauri personalizado

En lugar de depender del plugin shell, se implementó un comando Tauri personalizado que llama directamente a los comandos del sistema operativo.

#### 1. Crear comando Rust (src-tauri/src/commands/mod.rs)
```rust
#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    println!("Intentando abrir URL: {}", url);

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Error al abrir URL: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Error al abrir URL: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(&["/C", "start", &url])
            .spawn()
            .map_err(|e| format!("Error al abrir URL: {}", e))?;
    }

    println!("URL abierta exitosamente");
    Ok(())
}
```

#### 2. Registrar comando (src-tauri/src/main.rs)
```rust
.invoke_handler(tauri::generate_handler![
    // ... otros comandos
    commands::open_url,
])
```

#### 3. Actualizar servicio frontend (src/services/api.ts)
```typescript
export async function openUrl(url: string): Promise<void> {
  await invoke('open_url', { url });
}
```

#### 4. Usar botones en lugar de enlaces (src/components/ProjectList.tsx)
```tsx
<button
  onClick={() => openUrl(project.documentation_url!)}
  type="button"
>
  📖 Documentación
</button>
```

### Lecciones aprendidas

1. **En Tauri, los comandos personalizados son más confiables** que depender de plugins que pueden tener limitaciones o cambios entre versiones.

2. **Los logs con `println!()` en Rust** aparecen en la terminal donde corre `pnpm run tauri:dev`, no en la consola del navegador.

3. **Tauri no es un navegador web** - funcionalidades como `window.open()` o `console.log()` pueden no funcionar como se espera.

4. **Usar comandos nativos del OS** (`xdg-open` en Linux, `open` en macOS, `start` en Windows) es la forma más confiable de abrir URLs.

5. **Los tests deben actualizarse** cuando cambiamos de `<a>` a `<button>` - el `role` cambia de `link` a `button`.

### Verificación de funcionamiento

Para verificar que las URLs funcionan:
1. Ejecutar `pnpm run tauri:dev`
2. Crear un proyecto con URLs de documentación
3. Click en los botones de documentación/Drive
4. Verificar en la terminal los logs:
   ```
   Intentando abrir URL: https://...
   URL abierta exitosamente
   ```
5. El navegador del sistema debe abrirse con la URL

---

## 📝 Actualización de tests

Los tests de `ProjectList.test.tsx` se actualizaron de:
```typescript
screen.getAllByRole('link', { name: /documentación/i })
```

A:
```typescript
screen.getAllByRole('button', { name: /documentación/i })
```

---

**Fecha:** 2025-10-10
**Versión Tauri:** 2.1.0
**Sistema operativo probado:** Manjaro Linux
