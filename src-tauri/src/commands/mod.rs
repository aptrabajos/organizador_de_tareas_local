use crate::db::Database;
use crate::models::project::{CreateProjectDTO, CreateLinkDTO, Project, ProjectLink, UpdateProjectDTO, UpdateLinkDTO};
use std::process::Command;
use std::path::PathBuf;
use tauri::State;
use chrono::Local;

#[tauri::command]
pub async fn create_project(
    db: State<'_, Database>,
    project: CreateProjectDTO,
) -> Result<Project, String> {
    db.create_project(project)
        .map_err(|e| format!("Error creating project: {}", e))
}

#[tauri::command]
pub async fn get_all_projects(db: State<'_, Database>) -> Result<Vec<Project>, String> {
    db.get_all_projects()
        .map_err(|e| format!("Error getting projects: {}", e))
}

#[tauri::command]
pub async fn get_project(db: State<'_, Database>, id: i64) -> Result<Project, String> {
    db.get_project(id)
        .map_err(|e| format!("Error getting project: {}", e))
}

#[tauri::command]
pub async fn update_project(
    db: State<'_, Database>,
    id: i64,
    updates: UpdateProjectDTO,
) -> Result<Project, String> {
    println!("🔧 [UPDATE] Iniciando actualización del proyecto ID: {}", id);
    println!("📝 [UPDATE] Datos recibidos: {:?}", updates);
    
    let result = db.update_project(id, updates)
        .map_err(|e| {
            println!("❌ [UPDATE] Error en base de datos: {}", e);
            format!("Error updating project: {}", e)
        });
    
    match &result {
        Ok(project) => {
            println!("✅ [UPDATE] Proyecto actualizado exitosamente: '{}'", project.name);
        }
        Err(error) => {
            println!("❌ [UPDATE] Error al actualizar proyecto: {}", error);
        }
    }
    
    result
}

#[tauri::command]
pub async fn delete_project(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_project(id)
        .map_err(|e| format!("Error deleting project: {}", e))
}

#[tauri::command]
pub async fn search_projects(db: State<'_, Database>, query: String) -> Result<Vec<Project>, String> {
    db.search_projects(&query)
        .map_err(|e| format!("Error searching projects: {}", e))
}

#[tauri::command]
pub async fn open_terminal(path: String) -> Result<(), String> {
    // Detectar terminal disponible en el sistema
    let terminals = vec![
        "konsole",
        "gnome-terminal",
        "alacritty",
        "kitty",
        "xfce4-terminal",
        "tilix",
        "xterm",
    ];

    for terminal in terminals {
        let result = match terminal {
            "konsole" => Command::new(terminal)
                .arg("--workdir")
                .arg(&path)
                .spawn(),
            "gnome-terminal" => Command::new(terminal)
                .arg("--working-directory")
                .arg(&path)
                .spawn(),
            "alacritty" | "kitty" => Command::new(terminal)
                .arg("--working-directory")
                .arg(&path)
                .spawn(),
            "xfce4-terminal" => Command::new(terminal)
                .arg("--working-directory")
                .arg(&path)
                .spawn(),
            "tilix" => Command::new(terminal)
                .arg("-w")
                .arg(&path)
                .spawn(),
            "xterm" => Command::new("sh")
                .arg("-c")
                .arg(format!("cd {} && xterm", path))
                .spawn(),
            _ => continue,
        };

        if result.is_ok() {
            return Ok(());
        }
    }

    Err("No se encontró ningún terminal instalado".to_string())
}

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

#[derive(serde::Serialize)]
pub struct BackupData {
    content: String,
    path: String,
    filename: String,
}

#[tauri::command]
pub async fn create_project_backup(
    db: State<'_, Database>,
    project_id: i64,
) -> Result<BackupData, String> {
    println!("🔵 [BACKUP] Iniciando backup del proyecto ID: {}", project_id);

    // Obtener datos del proyecto
    let project = db
        .get_project(project_id)
        .map_err(|e| {
            println!("❌ [BACKUP] Error obteniendo proyecto: {}", e);
            format!("Error obteniendo proyecto: {}", e)
        })?;
    
    println!("✅ [BACKUP] Proyecto encontrado: '{}' ({})", project.name, project.local_path);

    // Crear contenido del markdown
    let now = Local::now();
    let markdown_content = format!(
        r#"# {} - Información del Proyecto

**Generado:** {}

---

## 📋 Información General

- **Nombre:** {}
- **Descripción:** {}
- **Ruta Local:** `{}`

---

## 🔗 Enlaces y Recursos

### Documentación
{}

### Documentación IA
{}

### Google Drive
{}

---

## ⏱️ Timestamps

- **Creado:** {}
- **Última actualización:** {}

---

*Backup generado automáticamente por Gestor de Proyectos*
"#,
        project.name,
        now.format("%Y-%m-%d %H:%M:%S"),
        project.name,
        project.description,
        project.local_path,
        project
            .documentation_url
            .as_ref()
            .map(|url| format!("🔗 [{}]({})", url, url))
            .unwrap_or_else(|| "❌ No configurada".to_string()),
        project
            .ai_documentation_url
            .as_ref()
            .map(|url| format!("🔗 [{}]({})", url, url))
            .unwrap_or_else(|| "❌ No configurada".to_string()),
        project
            .drive_link
            .as_ref()
            .map(|url| format!("🔗 [{}]({})", url, url))
            .unwrap_or_else(|| "❌ No configurado".to_string()),
        project.created_at,
        project.updated_at
    );

    // Crear nombre del archivo
    let filename = format!("{}_BACKUP.md", project.name.replace(" ", "_"));
    let backup_path = PathBuf::from(&project.local_path).join(&filename);

    let result_path = backup_path
        .to_str()
        .ok_or("Error convirtiendo ruta")?
        .to_string();

    println!("📄 [BACKUP] Nombre de archivo: {}", filename);
    println!("📁 [BACKUP] Ruta sugerida: {}", result_path);
    println!("📊 [BACKUP] Tamaño del contenido: {} bytes", markdown_content.len());
    println!("✅ [BACKUP] Datos de backup generados exitosamente");

    Ok(BackupData {
        content: markdown_content,
        path: result_path,
        filename,
    })
}

#[tauri::command]
pub async fn write_file_to_path(
    file_path: String,
    content: String,
) -> Result<String, String> {
    println!("📝 [WRITE] Escribiendo archivo: {}", file_path);
    
    use std::fs::write;
    use std::path::Path;
    
    // Crear directorio padre si no existe
    if let Some(parent) = Path::new(&file_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Error creando directorio: {}", e))?;
    }
    
    // Escribir archivo
    write(&file_path, content)
        .map_err(|e| format!("Error escribiendo archivo: {}", e))?;
    
    println!("✅ [WRITE] Archivo escrito exitosamente: {}", file_path);
    
    Ok(format!("Archivo escrito en: {}", file_path))
}

#[tauri::command]
pub async fn sync_project_to_backup(
    source_path: String,
    project_name: String,
) -> Result<String, String> {
    println!("🔄 [RSYNC] Iniciando sincronización:");
    println!("   📂 Origen: {}", source_path);
    
    let backup_path = format!("/mnt/sda1/{}", project_name);
    println!("   📁 Destino: {}", backup_path);
    
    // Crear directorio de destino si no existe
    std::fs::create_dir_all(&backup_path)
        .map_err(|e| format!("Error creando directorio de destino: {}", e))?;
    
    // Crear .rsyncignore básico si no existe en el proyecto origen
    let rsyncignore_path = format!("{}/.rsyncignore", source_path);
    if !std::path::Path::new(&rsyncignore_path).exists() {
        println!("📝 [RSYNC] Creando .rsyncignore básico para proyecto nuevo");
        let basic_rsyncignore = r#"# Archivos y directorios a ignorar en la sincronización rsync

# Dependencias de Node.js
node_modules/
npm-debug.log*
yarn-debug.log*
yarn-error.log*
pnpm-debug.log*

# Archivos de build y distribución
dist/
build/
out/
.next/
.nuxt/
.vuepress/dist/

# Archivos temporales
.tmp/
temp/
*.tmp
*.temp

# Logs
*.log
logs/

# Archivos de sistema
.DS_Store
Thumbs.db
*.swp
*.swo
*~

# Archivos de IDE/Editor
.vscode/settings.json
.idea/
*.sublime-*

# Archivos de Git
.git/
.gitignore

# Archivos de cache
.cache/
.parcel-cache/

# Archivos de testing
coverage/
.nyc_output/

# Archivos de backup
*.backup
*.bak
"#;
        
        std::fs::write(&rsyncignore_path, basic_rsyncignore)
            .map_err(|e| format!("Error creando .rsyncignore: {}", e))?;
        println!("✅ [RSYNC] .rsyncignore creado exitosamente");
    }
    
           // Ejecutar rsync con archivo de exclusión
           let output = std::process::Command::new("rsync")
               .args([
                   "-av",           // Archivo, verboso
                   "--delete",      // Eliminar archivos que no están en origen
                   "--progress",    // Mostrar progreso
                   "--exclude-from=.rsyncignore", // Usar archivo de exclusión
                   &format!("{}/", source_path), // Origen con / al final
                   &backup_path,    // Destino
               ])
               .current_dir(&source_path) // Establecer directorio de trabajo para .rsyncignore
               .output()
               .map_err(|e| format!("Error ejecutando rsync: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Rsync falló: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("✅ [RSYNC] Sincronización completada exitosamente");
    println!("📊 [RSYNC] Output: {}", stdout);
    
    Ok(format!("Proyecto sincronizado: {} -> {}", source_path, backup_path))
}

#[tauri::command]
pub async fn sync_project(
    source_path: String,
    destination_path: String,
) -> Result<String, String> {
    println!(
        "Sincronizando {} -> {}",
        source_path, destination_path
    );

    // Verificar que rsync esté disponible
    let rsync_check = Command::new("which").arg("rsync").output();

    if rsync_check.is_err() || !rsync_check.unwrap().status.success() {
        return Err("rsync no está instalado en el sistema".to_string());
    }

    // Ejecutar rsync
    let output = Command::new("rsync")
        .arg("-av") // archive + verbose
        .arg("--update") // solo copiar archivos más nuevos
        .arg("--progress")
        .arg(format!("{}/", source_path)) // trailing slash importante
        .arg(&destination_path)
        .output()
        .map_err(|e| format!("Error ejecutando rsync: {}", e))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("rsync falló: {}", error));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("rsync output:\n{}", stdout);

    Ok(format!(
        "Sincronización completada exitosamente a: {}",
        destination_path
    ))
}

// Comandos para manejar enlaces de proyectos
#[tauri::command]
pub async fn create_project_link(
    db: State<'_, Database>,
    link: CreateLinkDTO,
) -> Result<ProjectLink, String> {
    println!("🔗 [LINK] Creando enlace: {} - {}", link.title, link.url);
    db.create_link(link)
        .map_err(|e| format!("Error creating link: {}", e))
}

#[tauri::command]
pub async fn get_project_links(
    db: State<'_, Database>,
    project_id: i64,
) -> Result<Vec<ProjectLink>, String> {
    println!("🔗 [LINK] Obteniendo enlaces para proyecto ID: {}", project_id);
    db.get_project_links(project_id)
        .map_err(|e| format!("Error getting links: {}", e))
}

#[tauri::command]
pub async fn update_project_link(
    db: State<'_, Database>,
    id: i64,
    link: UpdateLinkDTO,
) -> Result<ProjectLink, String> {
    println!("🔗 [LINK] Actualizando enlace ID: {}", id);
    db.update_link(id, link)
        .map_err(|e| format!("Error updating link: {}", e))
}

#[tauri::command]
pub async fn delete_project_link(
    db: State<'_, Database>,
    id: i64,
) -> Result<(), String> {
    println!("🔗 [LINK] Eliminando enlace ID: {}", id);
    db.delete_link(id)
        .map_err(|e| format!("Error deleting link: {}", e))
}

// Comandos para Analytics y Tracking
#[tauri::command]
pub async fn track_project_open(
    db: State<'_, Database>,
    project_id: i64,
) -> Result<(), String> {
    println!("📊 [ANALYTICS] Registrando apertura del proyecto ID: {}", project_id);
    db.track_project_open(project_id)
        .map_err(|e| format!("Error tracking project open: {}", e))
}

#[tauri::command]
pub async fn add_project_time(
    db: State<'_, Database>,
    project_id: i64,
    seconds: i64,
) -> Result<(), String> {
    println!("⏱️ [ANALYTICS] Agregando {} segundos al proyecto ID: {}", seconds, project_id);
    db.add_project_time(project_id, seconds)
        .map_err(|e| format!("Error adding project time: {}", e))
}

#[tauri::command]
pub async fn get_project_stats(
    db: State<'_, Database>,
) -> Result<crate::models::project::ProjectStats, String> {
    println!("📈 [ANALYTICS] Obteniendo estadísticas globales");
    db.get_project_stats()
        .map_err(|e| format!("Error getting project stats: {}", e))
}

#[tauri::command]
pub async fn get_project_activities(
    db: State<'_, Database>,
    project_id: i64,
    limit: i64,
) -> Result<Vec<crate::models::project::ProjectActivity>, String> {
    println!("📋 [ANALYTICS] Obteniendo actividades del proyecto ID: {}", project_id);
    db.get_project_activities(project_id, limit)
        .map_err(|e| format!("Error getting project activities: {}", e))
}

// ==================== COMANDOS PARA ARCHIVOS ADJUNTOS ====================

#[tauri::command]
pub async fn add_attachment(
    db: State<'_, Database>,
    attachment: crate::models::project::CreateAttachmentDTO,
) -> Result<crate::models::project::ProjectAttachment, String> {
    println!("📎 [ATTACHMENT] Agregando archivo: {} ({} bytes)", attachment.filename, attachment.file_size);
    db.add_attachment(attachment)
        .map_err(|e| format!("Error adding attachment: {}", e))
}

#[tauri::command]
pub async fn get_attachments(
    db: State<'_, Database>,
    project_id: i64,
) -> Result<Vec<crate::models::project::ProjectAttachment>, String> {
    println!("📎 [ATTACHMENT] Obteniendo archivos del proyecto ID: {}", project_id);
    db.get_attachments(project_id)
        .map_err(|e| format!("Error getting attachments: {}", e))
}

#[tauri::command]
pub async fn delete_attachment(
    db: State<'_, Database>,
    id: i64,
) -> Result<(), String> {
    println!("🗑️ [ATTACHMENT] Eliminando archivo ID: {}", id);
    db.delete_attachment(id)
        .map_err(|e| format!("Error deleting attachment: {}", e))
}

