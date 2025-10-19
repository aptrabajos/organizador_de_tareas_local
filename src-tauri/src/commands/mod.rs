use crate::config::{AppConfig, ConfigManager, DetectedPrograms};
use crate::db::Database;
use crate::models::project::{CreateProjectDTO, CreateLinkDTO, Project, ProjectLink, UpdateProjectDTO, UpdateLinkDTO};
use crate::platform::{get_platform, ProgramDetector};
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
pub async fn open_terminal(
    config_manager: State<'_, ConfigManager>,
    path: String,
) -> Result<(), String> {
    println!("🚀 [TERMINAL] Abriendo terminal en: {}", path);
    let config = config_manager.get_config()?;
    let platform = get_platform();
    platform.open_terminal(&path, &config)
}

#[tauri::command]
pub async fn open_url(
    config_manager: State<'_, ConfigManager>,
    url: String,
) -> Result<(), String> {
    println!("🌐 [URL] Abriendo URL: {}", url);
    let config = config_manager.get_config()?;
    let platform = get_platform();
    platform.open_url(&url, &config)
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

// ==================== COMANDOS PARA PROJECT JOURNAL ====================

#[tauri::command]
pub async fn create_journal_entry(
    db: State<'_, Database>,
    entry: crate::models::project::CreateJournalEntryDTO,
) -> Result<crate::models::project::JournalEntry, String> {
    println!("📓 [JOURNAL] Creando entrada de diario para proyecto ID: {}", entry.project_id);
    db.create_journal_entry(entry)
        .map_err(|e| format!("Error creating journal entry: {}", e))
}

#[tauri::command]
pub async fn get_journal_entries(
    db: State<'_, Database>,
    project_id: i64,
) -> Result<Vec<crate::models::project::JournalEntry>, String> {
    println!("📓 [JOURNAL] Obteniendo entradas de diario para proyecto ID: {}", project_id);
    db.get_journal_entries(project_id)
        .map_err(|e| format!("Error getting journal entries: {}", e))
}

#[tauri::command]
pub async fn update_journal_entry(
    db: State<'_, Database>,
    id: i64,
    updates: crate::models::project::UpdateJournalEntryDTO,
) -> Result<crate::models::project::JournalEntry, String> {
    println!("📓 [JOURNAL] Actualizando entrada de diario ID: {}", id);
    db.update_journal_entry(id, updates)
        .map_err(|e| format!("Error updating journal entry: {}", e))
}

#[tauri::command]
pub async fn delete_journal_entry(
    db: State<'_, Database>,
    id: i64,
) -> Result<(), String> {
    println!("📓 [JOURNAL] Eliminando entrada de diario ID: {}", id);
    db.delete_journal_entry(id)
        .map_err(|e| format!("Error deleting journal entry: {}", e))
}

// ==================== COMANDOS PARA PROJECT TODOS ====================

#[tauri::command]
pub async fn create_todo(
    db: State<'_, Database>,
    todo: crate::models::project::CreateTodoDTO,
) -> Result<crate::models::project::ProjectTodo, String> {
    println!("✅ [TODO] Creando TODO para proyecto ID: {}", todo.project_id);
    db.create_todo(todo)
        .map_err(|e| format!("Error creating todo: {}", e))
}

#[tauri::command]
pub async fn get_project_todos(
    db: State<'_, Database>,
    project_id: i64,
) -> Result<Vec<crate::models::project::ProjectTodo>, String> {
    println!("✅ [TODO] Obteniendo TODOs para proyecto ID: {}", project_id);
    db.get_project_todos(project_id)
        .map_err(|e| format!("Error getting todos: {}", e))
}

#[tauri::command]
pub async fn update_todo(
    db: State<'_, Database>,
    id: i64,
    updates: crate::models::project::UpdateTodoDTO,
) -> Result<crate::models::project::ProjectTodo, String> {
    println!("✅ [TODO] Actualizando TODO ID: {}", id);
    db.update_todo(id, updates)
        .map_err(|e| format!("Error updating todo: {}", e))
}

#[tauri::command]
pub async fn delete_todo(
    db: State<'_, Database>,
    id: i64,
) -> Result<(), String> {
    println!("✅ [TODO] Eliminando TODO ID: {}", id);
    db.delete_todo(id)
        .map_err(|e| format!("Error deleting todo: {}", e))
}

// ==================== COMANDOS PARA ESTADOS Y FAVORITOS ====================

#[tauri::command]
pub async fn update_project_status(
    db: State<'_, Database>,
    project_id: i64,
    status: String,
) -> Result<(), String> {
    println!("🔄 [STATUS] Actualizando estado del proyecto ID: {} a '{}'", project_id, status);
    db.update_project_status(project_id, status)
        .map_err(|e| format!("Error updating project status: {}", e))
}

#[tauri::command]
pub async fn toggle_pin_project(
    db: State<'_, Database>,
    project_id: i64,
) -> Result<bool, String> {
    println!("⭐ [PIN] Toggling favorito para proyecto ID: {}", project_id);
    db.toggle_pin_project(project_id)
        .map_err(|e| format!("Error toggling pin: {}", e))
}

#[tauri::command]
pub async fn reorder_pinned_projects(
    db: State<'_, Database>,
    project_ids: Vec<i64>,
) -> Result<(), String> {
    println!("↕️ [PIN] Reordenando proyectos favoritos: {:?}", project_ids);
    db.reorder_pinned_projects(project_ids)
        .map_err(|e| format!("Error reordering pinned projects: {}", e))
}

// Git Commands
#[tauri::command]
pub async fn get_git_branch(path: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(&["-C", &path, "rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|e| format!("Failed to execute git command: {}", e))?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        Ok(branch)
    } else {
        Err("Not a git repository".to_string())
    }
}

#[tauri::command]
pub async fn get_git_status(path: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(&["-C", &path, "status", "--porcelain"])
        .output()
        .map_err(|e| format!("Failed to execute git command: {}", e))?;

    if output.status.success() {
        let status = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(status)
    } else {
        Err("Not a git repository".to_string())
    }
}

#[derive(serde::Serialize)]
pub struct GitCommit {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

#[tauri::command]
pub async fn get_recent_commits(path: String, limit: usize) -> Result<Vec<GitCommit>, String> {
    let limit_str = limit.to_string();
    let output = Command::new("git")
        .args(&[
            "-C",
            &path,
            "log",
            &format!("-{}", limit_str),
            "--pretty=format:%H|%an|%ar|%s",
        ])
        .output()
        .map_err(|e| format!("Failed to execute git command: {}", e))?;

    if output.status.success() {
        let commits_str = String::from_utf8_lossy(&output.stdout);
        let commits: Vec<GitCommit> = commits_str
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() == 4 {
                    Some(GitCommit {
                        hash: parts[0][..7].to_string(), // Short hash
                        author: parts[1].to_string(),
                        date: parts[2].to_string(),
                        message: parts[3].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();
        Ok(commits)
    } else {
        Err("Not a git repository or no commits".to_string())
    }
}

// ==================== COMANDOS PARA CONFIGURACIÓN ====================

#[tauri::command]
pub async fn get_config(
    config_manager: State<'_, ConfigManager>,
) -> Result<AppConfig, String> {
    println!("⚙️ [CONFIG] Obteniendo configuración");
    config_manager.get_config()
}

#[tauri::command]
pub async fn update_config(
    config_manager: State<'_, ConfigManager>,
    config: AppConfig,
) -> Result<(), String> {
    println!("💾 [CONFIG] Actualizando configuración");
    config_manager.update_config(config)
}

#[tauri::command]
pub async fn reset_config(
    config_manager: State<'_, ConfigManager>,
) -> Result<AppConfig, String> {
    println!("🔄 [CONFIG] Reseteando configuración a valores por defecto");
    config_manager.reset_config()
}

#[tauri::command]
pub async fn detect_programs() -> Result<DetectedPrograms, String> {
    println!("🔍 [DETECTION] Detectando programas instalados");
    Ok(ProgramDetector::detect_all())
}

#[tauri::command]
pub async fn open_file_manager(
    config_manager: State<'_, ConfigManager>,
    path: String,
) -> Result<(), String> {
    println!("📁 [FILE_MANAGER] Abriendo gestor de archivos en: {}", path);
    let config = config_manager.get_config()?;
    let platform = get_platform();
    platform.open_file_manager(&path, &config)
}

#[tauri::command]
pub async fn open_text_editor(
    config_manager: State<'_, ConfigManager>,
    path: String,
) -> Result<(), String> {
    println!("📝 [TEXT_EDITOR] Abriendo editor de texto: {}", path);
    let config = config_manager.get_config()?;
    let platform = get_platform();
    platform.open_text_editor(&path, &config)
}

