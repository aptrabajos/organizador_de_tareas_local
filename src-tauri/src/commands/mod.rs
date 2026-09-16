mod guards;

use guards::assert_registered_project_path;

use crate::models::project::{
    CreateProjectDTO, CreateLinkDTO, Project, ProjectLink, UpdateProjectDTO, UpdateLinkDTO, ProjectWithChildren,
    DashboardData
};
use crate::config::{AppConfig, ConfigManager, DetectedPrograms};
use crate::db::Database;
use std::sync::Mutex;

// ==================== ACTIVE SESSION STATE ====================

/// Estado global de la sesión de tracking activa
pub struct ActiveSessionState {
    pub session_id: Option<i64>,
    pub project_id: Option<i64>,
    pub project_name: Option<String>,
    /// `local_path` del proyecto. Antes NO existía y `get_work_session_status` rellenaba
    /// su campo `project_path` con `project_name` —lo más parecido que había a mano—,
    /// o sea devolvía un nombre donde el contrato promete una ruta.
    pub project_path: Option<String>,
    pub started_at: Option<std::time::Instant>,
}

impl Default for ActiveSessionState {
    fn default() -> Self {
        Self {
            session_id: None,
            project_id: None,
            project_name: None,
            project_path: None,
            started_at: None,
        }
    }
}

/// Wrapper thread-safe para el estado de sesión activa
pub struct ActiveSession(pub Mutex<ActiveSessionState>);

impl Default for ActiveSession {
    fn default() -> Self {
        Self(Mutex::new(ActiveSessionState::default()))
    }
}

#[tauri::command]
pub async fn get_dashboard_data(db: State<'_, Database>) -> Result<DashboardData, String> {
    println!("📊 [DASHBOARD] Obteniendo datos para el dashboard");
    let recent_projects = db.get_recent_projects().map_err(|e| e.to_string())?;
    let pending_todos = db.get_all_pending_todos().map_err(|e| e.to_string())?;
    let recent_journal_entries = db.get_recent_journal_entries().map_err(|e| e.to_string())?;

    Ok(DashboardData {
        recent_projects,
        pending_todos,
        recent_journal_entries,
    })
}

use crate::platform::{get_platform, ProgramDetector};
use std::process::Command;
use std::path::PathBuf;
use tauri::State;
use chrono::Local;

// ==================== BACKUP DE LA BASE DE DATOS ====================

/// Crea un backup manual de la base de datos, lo verifica y aplica retención.
#[tauri::command]
pub async fn backup_database(
    db: State<'_, Database>,
    config: State<'_, ConfigManager>,
) -> Result<crate::backup::BackupResult, String> {
    println!("💾 [BACKUP] Iniciando backup manual de la base de datos");
    crate::backup::run_backup(&db, &config)
}

/// Lista los backups existentes en el directorio destino.
#[tauri::command]
pub async fn list_backups(
    config: State<'_, ConfigManager>,
) -> Result<Vec<crate::backup::BackupEntry>, String> {
    crate::backup::list_backups(&config)
}

/// Restaura la DB viva a partir de un backup elegido por el usuario. Operación
/// IRREVERSIBLE: reemplaza `projects.db` completo. `restore_backup` (módulo
/// `backup`) verifica integridad antes y después de copiar, y solo hace el
/// rename atómico final si todo salió bien; si algo falla, la DB actual queda
/// intacta.
#[tauri::command]
pub async fn restore_backup(
    app: tauri::AppHandle,
    backup_path: String,
) -> Result<crate::backup::RestoreResult, String> {
    println!("♻️ [BACKUP] Iniciando restauración desde: {}", backup_path);
    let result = crate::backup::restore_backup(&backup_path)?;
    println!("✅ [BACKUP] Restauración completa desde: {}", backup_path);

    // La conexión SQLite viva (State<Database>) sigue con el file descriptor abierto
    // sobre el archivo ANTERIOR: un rename no la mueve a leer el nuevo archivo. Si la
    // app siguiera corriendo, seguiría leyendo/escribiendo los datos VIEJOS y cualquier
    // escritura posterior se perdería al cerrar (el inodo viejo queda sin ningún path
    // que lo referencie). Para evitar ese estado inconsistente, cerramos la app: el
    // usuario debe reabrirla para que la nueva conexión lea el archivo restaurado. El
    // pequeño delay le da tiempo al frontend a mostrar el aviso antes de que el
    // proceso termine.
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(800));
        app.exit(0);
    });

    Ok(result)
}

#[tauri::command]
pub async fn create_project(
    db: State<'_, Database>,
    project: CreateProjectDTO,
) -> Result<Project, String> {
    // Prefijo en español: este wrapper transporta la validación de dominio
    // "El grupo padre seleccionado no existe o está en la papelera." (db/mod.rs), que
    // el usuario SÍ puede corregir. Con el prefijo en inglés llegaba a la UI como un
    // híbrido "Error creating project: El grupo padre...".
    db.create_project(project)
        .map_err(|e| format!("No se pudo crear el proyecto: {}", e))
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
pub async fn restore_project(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.restore_project(id)
        .map_err(|e| format!("Error restoring project: {}", e))
}

#[tauri::command]
pub async fn list_trash(db: State<'_, Database>) -> Result<Vec<crate::db::TrashItem>, String> {
    db.list_trash()
        .map_err(|e| format!("Error listing trash: {}", e))
}

#[tauri::command]
pub async fn purge_project(
    db: State<'_, Database>,
    config: State<'_, ConfigManager>,
    id: i64,
) -> Result<(), String> {
    // Red de seguridad antes del DELETE irreversible: backup fire-and-forget
    // reusando la infraestructura ya verificada de backup/mod.rs (VACUUM INTO +
    // integrity_check + retención). Si el backup falla (disco lleno, sin permisos,
    // etc.) NO bloqueamos el purge -solo se loguea-: preferimos un purge sin backup
    // fresco a un purge que deja de funcionar por completo.
    if let Err(e) = crate::backup::run_backup(&db, &config) {
        eprintln!(
            "⚠️ [PURGE] No se pudo crear el backup de seguridad antes de purgar: {}",
            e
        );
    }

    db.purge_project(id)
        .map_err(|e| format!("Error purging project: {}", e))
}

#[tauri::command]
pub async fn empty_trash(
    db: State<'_, Database>,
    config: State<'_, ConfigManager>,
) -> Result<(), String> {
    // Misma red de seguridad que purge_project: empty_trash es igual de irreversible
    // (borra TODA la papelera de una vez) y hasta ahora no tenía ningún backup previo.
    if let Err(e) = crate::backup::run_backup(&db, &config) {
        eprintln!(
            "⚠️ [EMPTY_TRASH] No se pudo crear el backup de seguridad antes de vaciar la papelera: {}",
            e
        );
    }

    db.empty_trash()
        .map_err(|e| format!("Error emptying trash: {}", e))
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

/// Sanea un nombre de proyecto para usarlo como componente de nombre de archivo o
/// de carpeta, evitando path traversal. Reemplaza separadores de path ('/' y '\')
/// y colapsa secuencias '..' para que el resultado nunca contenga un separador ni
/// una referencia a directorio padre.
///
/// ÚNICA fuente de verdad del saneado: la usan `create_project_backup`,
/// `sync_project_to_backup` y `export_project_to_pdf`. Antes cada una tenía su
/// propio criterio y la del PDF era la laxa (sólo espacios y '/'), así que un
/// proyecto llamado `../../etc/passwd` generaba una ruta de PDF con traversal.
///
/// Los espacios se PRESERVAN a propósito: no son un riesgo de traversal (las rutas
/// se pasan a `File::create`/`rsync` como argumento, nunca por shell) y convertirlos
/// a '_' renombraría las carpetas de backup ya existentes en destino, dejándolas
/// huérfanas y forzando un resync completo.
fn sanitize_backup_filename_component(name: &str) -> String {
    name.trim().replace(['/', '\\'], "_").replace("..", "_")
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

    // Crear nombre del archivo. Sanear project.name contra path traversal: reemplazar
    // espacios NO alcanza, hay que neutralizar separadores de path y '..' o un nombre
    // de proyecto malicioso podría escapar la carpeta destino elegida por el usuario.
    let filename = format!("{}_BACKUP.md", sanitize_backup_filename_component(&project.name));
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

/// Sincroniza los ARCHIVOS de un proyecto a la carpeta de backup configurada (no la DB).
/// Saneado: destino configurable (nunca /mnt/sda1 hardcodeado), `--update` (no borra el
/// destino), exclusiones por argumento (no ensucia el repo con .rsyncignore).
#[tauri::command]
pub async fn sync_project_to_backup(
    config: State<'_, ConfigManager>,
    source_path: String,
    project_name: String,
) -> Result<String, String> {
    println!("🔄 [RSYNC] Sincronizando archivos de '{}'", project_name);

    // Verificar que rsync esté instalado (solo Unix; en Windows `which` no existe)
    let rsync_ok = std::process::Command::new("which")
        .arg("rsync")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !rsync_ok {
        return Err("rsync no está instalado en el sistema".to_string());
    }

    // Validar el origen: debe existir y ser un directorio
    if source_path.trim().is_empty() || !std::path::Path::new(&source_path).is_dir() {
        return Err(format!("El directorio de origen no existe: {}", source_path));
    }

    // Sanear project_name contra path traversal: join() con una ruta absoluta o con
    // componentes '..' ESCAPARÍA la carpeta de backup. Lo reducimos a un único nombre
    // de carpeta seguro (sin separadores ni '..') con el saneado compartido.
    let safe_name = sanitize_backup_filename_component(&project_name);
    if safe_name.is_empty() {
        return Err("Nombre de proyecto no válido para el backup".to_string());
    }

    // Destino CONFIGURABLE: {carpeta de backup}/{proyecto}. Reusa la misma resolución
    // que el backup de la DB (config.backup.default_path → default de plataforma).
    let cfg = config.get_config()?;
    let backup_path = crate::backup::resolve_backup_dir(&cfg)?.join(&safe_name);
    std::fs::create_dir_all(&backup_path)
        .map_err(|e| format!("Error creando directorio de destino: {}", e))?;
    let backup_str = backup_path
        .to_str()
        .ok_or("La ruta de backup contiene caracteres no válidos")?;

    // Exclusiones por argumento (NO se escribe ningún .rsyncignore en el repo del usuario).
    // Se excluyen artefactos de build/caches, NO .git (la historia es parte del backup).
    let excludes = [
        "node_modules/", "dist/", "build/", "out/", ".next/", ".nuxt/", ".cache/",
        ".parcel-cache/", "coverage/", ".nyc_output/", "logs/", "*.log", "*.tmp",
        "*.bak", ".DS_Store", "Thumbs.db", "*.swp",
    ];
    let mut args: Vec<String> = vec!["-a".into(), "--update".into()];
    for ex in excludes {
        args.push(format!("--exclude={}", ex));
    }
    args.push(format!("{}/", source_path)); // origen con / final
    args.push(backup_str.to_string());

    let output = std::process::Command::new("rsync")
        .args(&args)
        .output()
        .map_err(|e| format!("Error ejecutando rsync: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Rsync falló: {}", stderr));
    }

    println!("✅ [RSYNC] Sincronización completada en {}", backup_str);
    Ok(format!("Proyecto sincronizado en: {}", backup_str))
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
    // Prefijo en español: transporta la validación del límite de 5 MB, que el usuario
    // puede corregir eligiendo otro archivo.
    db.add_attachment(attachment)
        .map_err(|e| format!("No se pudo adjuntar el archivo: {}", e))
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

#[tauri::command]
pub async fn update_project_order(
    db: State<'_, Database>,
    project_id: i64,
    new_order: i64,
) -> Result<(), String> {
    println!("↕️ [ORDER] Actualizando orden del proyecto ID: {} a {}", project_id, new_order);
    db.update_project_order(project_id, new_order)
        .map_err(|e| format!("Error updating project order: {}", e))
}

// Git Commands
//
// Los 11 comandos git empiezan SIEMPRE con `assert_registered_project_path`: sin ese
// guard la app corre git sobre cualquier repositorio del disco, gestionado o no (ver
// `commands::guards`). El `db: State<'_, Database>` lo inyecta Tauri, NO viaja en el
// payload de `invoke`, así que la firma del lado del cliente no cambia.
//
// IDIOMA DE LOS MENSAJES DE ERROR (ver CLAUDE.md, "Mensajes de error"):
// - "La carpeta del proyecto no es un repositorio git." → ESPAÑOL: el usuario puede
//   corregirlo (inicializar el repo, elegir otro proyecto).
// - "Failed to execute git command" → INGLÉS a propósito: es un fallo al spawnear el
//   proceso. No hay nada que el usuario pueda hacer; es una línea de log.
#[tauri::command]
pub async fn get_git_branch(db: State<'_, Database>, path: String) -> Result<String, String> {
    assert_registered_project_path(&db, &path)?;

    let output = Command::new("git")
        .args(["-C", &path, "rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|e| format!("Failed to execute git command: {}", e))?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        Ok(branch)
    } else {
        Err("La carpeta del proyecto no es un repositorio git.".to_string())
    }
}

#[tauri::command]
pub async fn get_git_status(db: State<'_, Database>, path: String) -> Result<String, String> {
    assert_registered_project_path(&db, &path)?;

    let output = Command::new("git")
        .args(["-C", &path, "status", "--porcelain"])
        .output()
        .map_err(|e| format!("Failed to execute git command: {}", e))?;

    if output.status.success() {
        let status = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(status)
    } else {
        Err("La carpeta del proyecto no es un repositorio git.".to_string())
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
pub async fn get_recent_commits(
    db: State<'_, Database>,
    path: String,
    limit: usize,
) -> Result<Vec<GitCommit>, String> {
    assert_registered_project_path(&db, &path)?;

    let limit_str = limit.to_string();
    let output = Command::new("git")
        .args([
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
        Err("La carpeta del proyecto no es un repositorio git, o todavía no tiene commits.".to_string())
    }
}

// ==================== COMANDOS GIT MEJORADOS ====================

/// Estructura para conteo de archivos modificados
#[derive(serde::Serialize)]
pub struct GitFileCount {
    pub modified: usize,
    pub staged: usize,
    pub untracked: usize,
}

/// Obtener conteo de archivos modificados, staged y untracked
#[tauri::command]
pub async fn get_git_file_count(
    db: State<'_, Database>,
    path: String,
) -> Result<GitFileCount, String> {
    assert_registered_project_path(&db, &path)?;

    let output = Command::new("git")
        .args(["-C", &path, "status", "--porcelain"])
        .output()
        .map_err(|e| format!("Failed to execute git command: {}", e))?;

    if !output.status.success() {
        return Err("La carpeta del proyecto no es un repositorio git.".to_string());
    }

    let status = String::from_utf8_lossy(&output.stdout);
    let mut modified = 0;
    let mut staged = 0;
    let mut untracked = 0;

    for line in status.lines() {
        if line.len() < 3 {
            continue;
        }

        let status_code = &line[0..2];

        // Primer carácter: staged (index)
        // Segundo carácter: working tree
        match status_code.chars().next().unwrap_or(' ') {
            'M' | 'A' | 'D' | 'R' | 'C' => staged += 1,
            _ => {}
        }

        match status_code.chars().nth(1).unwrap_or(' ') {
            'M' | 'D' => modified += 1,
            _ => {}
        }

        if status_code.starts_with("??") {
            untracked += 1;
        }
    }

    Ok(GitFileCount {
        modified,
        staged,
        untracked,
    })
}

/// Obtener lista de archivos modificados
#[tauri::command]
pub async fn get_git_modified_files(
    db: State<'_, Database>,
    path: String,
) -> Result<Vec<String>, String> {
    assert_registered_project_path(&db, &path)?;

    let output = Command::new("git")
        .args(["-C", &path, "status", "--porcelain"])
        .output()
        .map_err(|e| format!("Failed to execute git command: {}", e))?;

    if !output.status.success() {
        return Err("La carpeta del proyecto no es un repositorio git.".to_string());
    }

    let status = String::from_utf8_lossy(&output.stdout);
    let files: Vec<String> = status
        .lines()
        .filter_map(|line| {
            if line.len() >= 3 {
                Some(line[3..].trim().to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(files)
}

/// Stage archivos (git add)
#[tauri::command]
pub async fn git_add(
    db: State<'_, Database>,
    path: String,
    files: Vec<String>,
) -> Result<String, String> {
    assert_registered_project_path(&db, &path)?;

    println!("📝 [GIT] Staging {} archivos", files.len());

    let mut args = vec!["-C", &path, "add"];
    let file_refs: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
    args.extend(file_refs);

    let output = Command::new("git")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute git add: {}", e))?;

    if output.status.success() {
        println!("✅ [GIT] Archivos staged exitosamente");
        Ok("Archivos staged exitosamente".to_string())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(format!("No se pudieron preparar los archivos para el commit: {}", error))
    }
}

/// Crear commit (git commit)
#[tauri::command]
pub async fn git_commit(
    db: State<'_, Database>,
    path: String,
    message: String,
) -> Result<String, String> {
    assert_registered_project_path(&db, &path)?;

    println!("💾 [GIT] Creando commit: {}", message);

    let output = Command::new("git")
        .args(["-C", &path, "commit", "-m", &message])
        .output()
        .map_err(|e| format!("Failed to execute git commit: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("✅ [GIT] Commit creado exitosamente");
        Ok(stdout.to_string())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(format!("No se pudo crear el commit: {}", error))
    }
}

/// Push a remote (git push)
#[tauri::command]
pub async fn git_push(db: State<'_, Database>, path: String) -> Result<String, String> {
    assert_registered_project_path(&db, &path)?;

    println!("🚀 [GIT] Pushing to remote");

    let output = Command::new("git")
        .args(["-C", &path, "push"])
        .output()
        .map_err(|e| format!("Failed to execute git push: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("✅ [GIT] Push exitoso");
        Ok(format!("{}{}", stdout, stderr))
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(format!("No se pudo hacer push al remoto: {}", error))
    }
}

/// Pull from remote (git pull)
#[tauri::command]
pub async fn git_pull(db: State<'_, Database>, path: String) -> Result<String, String> {
    assert_registered_project_path(&db, &path)?;

    println!("⬇️ [GIT] Pulling from remote");

    let output = Command::new("git")
        .args(["-C", &path, "pull"])
        .output()
        .map_err(|e| format!("Failed to execute git pull: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("✅ [GIT] Pull exitoso");
        Ok(stdout.to_string())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(format!("No se pudo hacer pull del remoto: {}", error))
    }
}

/// Obtener URL del remote origin
#[tauri::command]
pub async fn get_git_remote_url(
    db: State<'_, Database>,
    path: String,
) -> Result<Option<String>, String> {
    assert_registered_project_path(&db, &path)?;

    let output = Command::new("git")
        .args(["-C", &path, "remote", "get-url", "origin"])
        .output()
        .map_err(|e| format!("Failed to execute git command: {}", e))?;

    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if url.is_empty() {
            Ok(None)
        } else {
            Ok(Some(url))
        }
    } else {
        Ok(None)
    }
}

/// Obtener commits ahead/behind respecto al remote
#[tauri::command]
pub async fn get_git_ahead_behind(
    db: State<'_, Database>,
    path: String,
) -> Result<(u32, u32), String> {
    // El guard importa especialmente acá: este comando hace `fetch`, o sea tráfico de
    // red contra el remoto del repositorio que le pasen.
    assert_registered_project_path(&db, &path)?;

    // Primero hacer fetch para tener info actualizada
    let _ = Command::new("git")
        .args(["-C", &path, "fetch", "--quiet"])
        .output();

    let output = Command::new("git")
        .args([
            "-C",
            &path,
            "rev-list",
            "--left-right",
            "--count",
            "HEAD...@{upstream}",
        ])
        .output()
        .map_err(|e| format!("Failed to execute git command: {}", e))?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = result.split_whitespace().collect();

        if parts.len() == 2 {
            let ahead = parts[0].parse::<u32>().unwrap_or(0);
            let behind = parts[1].parse::<u32>().unwrap_or(0);
            Ok((ahead, behind))
        } else {
            Ok((0, 0))
        }
    } else {
        // No hay upstream configurado o no es un repo git
        Ok((0, 0))
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

#[tauri::command]
pub async fn select_backup_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    println!("📁 [DIALOG] Abriendo diálogo de selección de carpeta");

    // Usar tauri-plugin-dialog para Tauri 2.x
    let result = tauri_plugin_dialog::DialogExt::dialog(&app)
        .file()
        .set_title("Seleccionar carpeta de backups")
        .blocking_pick_folder();

    match result {
        Some(path) => {
            let path_str = path.to_string();
            println!("✅ [DIALOG] Carpeta seleccionada: {}", path_str);
            Ok(Some(path_str))
        }
        None => {
            println!("⚠️ [DIALOG] Usuario canceló la selección");
            Ok(None)
        }
    }
}

/// Selector de carpeta genérico
#[tauri::command]
pub async fn select_folder(
    app: tauri::AppHandle,
    title: Option<String>,
) -> Result<Option<String>, String> {
    let dialog_title = title.unwrap_or_else(|| "Seleccionar carpeta".to_string());
    println!("📁 [DIALOG] Abriendo selector de carpeta: {}", dialog_title);

    let result = tauri_plugin_dialog::DialogExt::dialog(&app)
        .file()
        .set_title(&dialog_title)
        .blocking_pick_folder();

    match result {
        Some(path) => {
            let path_str = path.to_string();
            println!("✅ [DIALOG] Carpeta seleccionada: {}", path_str);
            Ok(Some(path_str))
        }
        None => {
            println!("⚠️ [DIALOG] Usuario canceló la selección");
            Ok(None)
        }
    }
}

/// Selector de archivo único
#[tauri::command]
pub async fn select_file(
    app: tauri::AppHandle,
    title: Option<String>,
    filters: Option<Vec<(String, Vec<String>)>>,
) -> Result<Option<String>, String> {
    let dialog_title = title.unwrap_or_else(|| "Seleccionar archivo".to_string());
    println!("📄 [DIALOG] Abriendo selector de archivo: {}", dialog_title);

    let mut dialog = tauri_plugin_dialog::DialogExt::dialog(&app)
        .file()
        .set_title(&dialog_title);

    // Agregar filtros si se proporcionan
    if let Some(filter_list) = filters {
        for (name, extensions) in filter_list {
            let ext_refs: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();
            dialog = dialog.add_filter(&name, &ext_refs);
        }
    }

    let result = dialog.blocking_pick_file();

    match result {
        Some(path) => {
            let path_str = path.to_string();
            println!("✅ [DIALOG] Archivo seleccionado: {}", path_str);
            Ok(Some(path_str))
        }
        None => {
            println!("⚠️ [DIALOG] Usuario canceló la selección");
            Ok(None)
        }
    }
}

/// Selector de múltiples archivos
#[tauri::command]
pub async fn select_files(
    app: tauri::AppHandle,
    title: Option<String>,
    filters: Option<Vec<(String, Vec<String>)>>,
) -> Result<Vec<String>, String> {
    let dialog_title = title.unwrap_or_else(|| "Seleccionar archivos".to_string());
    println!(
        "📄 [DIALOG] Abriendo selector de múltiples archivos: {}",
        dialog_title
    );

    let mut dialog = tauri_plugin_dialog::DialogExt::dialog(&app)
        .file()
        .set_title(&dialog_title);

    // Agregar filtros si se proporcionan
    if let Some(filter_list) = filters {
        for (name, extensions) in filter_list {
            let ext_refs: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();
            dialog = dialog.add_filter(&name, &ext_refs);
        }
    }

    let result = dialog.blocking_pick_files();

    match result {
        Some(paths) => {
            let path_strings: Vec<String> =
                paths.iter().map(|p| p.to_string()).collect();
            println!("✅ [DIALOG] {} archivos seleccionados", path_strings.len());
            Ok(path_strings)
        }
        None => {
            println!("⚠️ [DIALOG] Usuario canceló la selección");
            Ok(Vec::new())
        }
    }
}

/// Diálogo para guardar archivo
#[tauri::command]
pub async fn save_file_dialog(
    app: tauri::AppHandle,
    title: Option<String>,
    default_name: Option<String>,
    filters: Option<Vec<(String, Vec<String>)>>,
) -> Result<Option<String>, String> {
    let dialog_title = title.unwrap_or_else(|| "Guardar archivo".to_string());
    println!("💾 [DIALOG] Abriendo diálogo guardar: {}", dialog_title);

    let mut dialog = tauri_plugin_dialog::DialogExt::dialog(&app)
        .file()
        .set_title(&dialog_title);

    // Nombre por defecto
    if let Some(name) = default_name {
        dialog = dialog.set_file_name(&name);
    }

    // Agregar filtros si se proporcionan
    if let Some(filter_list) = filters {
        for (name, extensions) in filter_list {
            let ext_refs: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();
            dialog = dialog.add_filter(&name, &ext_refs);
        }
    }

    let result = dialog.blocking_save_file();

    match result {
        Some(path) => {
            let path_str = path.to_string();
            println!("✅ [DIALOG] Ubicación de guardado: {}", path_str);
            Ok(Some(path_str))
        }
        None => {
            println!("⚠️ [DIALOG] Usuario canceló el guardado");
            Ok(None)
        }
    }
}

// ==================== COMANDOS PARA SHORTCUTS ====================

/// Obtener la configuración de atajos de teclado
/// Nota: El registro real de shortcuts se maneja desde el frontend con el plugin
#[tauri::command]
pub async fn get_shortcuts_config(
    config_manager: State<'_, ConfigManager>,
) -> Result<crate::config::schema::ShortcutsConfig, String> {
    println!("⌨️ [SHORTCUTS] Obteniendo configuración de atajos");
    let config = config_manager.get_config()?;
    Ok(config.shortcuts)
}

/// Actualizar la configuración de atajos de teclado
#[tauri::command]
pub async fn update_shortcuts_config(
    config_manager: State<'_, ConfigManager>,
    shortcuts_config: crate::config::schema::ShortcutsConfig,
) -> Result<(), String> {
    println!("⌨️ [SHORTCUTS] Actualizando configuración de atajos");
    let mut config = config_manager.get_config()?;
    config.shortcuts = shortcuts_config;
    config_manager.update_config(config)?;
    println!("✅ [SHORTCUTS] Configuración actualizada exitosamente");
    Ok(())
}

// ==================== COMANDOS DE GRUPOS DE PROYECTOS (v0.4.0) ====================

/// Obtener solo proyectos raíz (grupos principales)
#[tauri::command]
pub async fn get_root_projects(db: State<'_, Database>) -> Result<Vec<Project>, String> {
    println!("📁 [GROUPS] Obteniendo proyectos raíz");
    db.get_root_projects()
        .map_err(|e| format!("Error getting root projects: {}", e))
}

/// Obtener subproyectos de un grupo
#[tauri::command]
pub async fn get_subprojects(db: State<'_, Database>, parent_id: i64) -> Result<Vec<Project>, String> {
    println!("📁 [GROUPS] Obteniendo subproyectos del grupo ID: {}", parent_id);
    db.get_subprojects(parent_id)
        .map_err(|e| format!("Error getting subprojects: {}", e))
}

/// Obtener proyecto con sus hijos
#[tauri::command]
pub async fn get_project_with_children(
    db: State<'_, Database>,
    id: i64,
) -> Result<ProjectWithChildren, String> {
    println!("📁 [GROUPS] Obteniendo proyecto con hijos ID: {}", id);
    db.get_project_with_children(id)
        .map_err(|e| format!("Error getting project with children: {}", e))
}

/// Contar subproyectos de un grupo
#[tauri::command]
pub async fn count_subprojects(db: State<'_, Database>, parent_id: i64) -> Result<i64, String> {
    db.count_subprojects(parent_id)
        .map_err(|e| format!("Error counting subprojects: {}", e))
}

/// Asignar proyecto a un grupo (o quitarlo si parent_id es null)
#[tauri::command]
pub async fn assign_project_to_group(
    db: State<'_, Database>,
    child_id: i64,
    parent_id: Option<i64>,
) -> Result<(), String> {
    println!("📁 [GROUPS] Asignando proyecto {} al grupo {:?}", child_id, parent_id);
    // El método DB ya devuelve un mensaje en español accionable (self/ciclo/inexistente);
    // se propaga directo para no enmascararlo con un prefijo genérico.
    db.assign_project_to_group(child_id, parent_id)
}

/// Exportar proyecto a PDF
#[tauri::command]
pub async fn export_project_to_pdf(
    db: State<'_, Database>,
    project_id: i64,
) -> Result<String, String> {
    println!("📄 [PDF] Exportando proyecto ID: {}", project_id);

    // Obtener proyecto de la base de datos
    let project = db.get_project(project_id)
        .map_err(|e| format!("Error getting project: {}", e))?;

    // Exportar en la carpeta del proyecto (local_path)
    let export_dir = PathBuf::from(&project.local_path);

    // Verificar que el directorio existe
    if !export_dir.exists() {
        return Err(format!("El directorio del proyecto no existe: {}", project.local_path));
    }

    // Nombre del archivo con timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    // Saneado compartido: antes acá sólo se tocaban espacios y '/', así que un nombre
    // con '\' o '..' (ej. `../../etc/passwd`) escapaba la carpeta del proyecto al
    // hacer join(). Ahora usa el MISMO criterio que los backups.
    let safe_project_name = sanitize_backup_filename_component(&project.name);
    let filename = format!("{}_{}.pdf", safe_project_name, timestamp);
    let output_path = export_dir.join(&filename);

    println!("📄 [PDF] Generando PDF en: {:?}", output_path);

    // Exportar a PDF
    crate::pdf_export::export_project_to_pdf(&db, &project, output_path.to_str().unwrap())?;

    println!("✅ [PDF] PDF generado exitosamente");

    Ok(output_path.to_str().unwrap().to_string())
}

// ==================== TIME TRACKING COMMANDS ====================

use crate::tracking::aggregator::TimeStats;
use crate::tracking::config::{GestorConfig, init_gestor_config, has_tracking_config, find_gestor_config};

/// Inicializar tracking en un proyecto (crear carpeta .gestor/)
#[tauri::command]
pub async fn init_tracking(
    db: State<'_, Database>,
    project_id: i64,
) -> Result<String, String> {
    println!("🕒 [TRACKING] Inicializando tracking para proyecto ID: {}", project_id);

    // Obtener información del proyecto
    let project = db.get_project(project_id)
        .map_err(|e| format!("Error getting project: {}", e))?;

    let project_path = std::path::Path::new(&project.local_path);

    // Verificar que el directorio existe
    if !project_path.exists() {
        return Err(format!("El directorio del proyecto no existe: {}", project.local_path));
    }

    // Inicializar la carpeta .gestor/
    let _config = init_gestor_config(project_path, project_id, project.name.clone())
        .map_err(|e| format!("Error inicializando tracking: {}", e))?;

    println!("✅ [TRACKING] Tracking inicializado para: {}", project.name);

    Ok(format!("Tracking inicializado en {}", project.local_path))
}

/// Obtener sesiones de tracking de un proyecto
#[tauri::command]
pub async fn get_tracking_sessions(
    db: State<'_, Database>,
    project_id: i64,
    limit: Option<i64>,
) -> Result<Vec<crate::tracking::aggregator::TimeTrackingSession>, String> {
    let limit = limit.unwrap_or(20);

    db.get_tracking_sessions(project_id, limit)
        .map_err(|e| format!("Error getting tracking sessions: {}", e))
}

/// Obtener estado actual de tracking (qué proyecto está siendo tracked)
#[tauri::command]
pub async fn get_tracking_status() -> Result<TrackingStatusResponse, String> {
    // Por ahora retornamos un estado vacío
    // Cuando el socket esté integrado, esto leerá del SessionManager
    Ok(TrackingStatusResponse {
        is_tracking: false,
        project_id: None,
        project_path: None,
        elapsed_seconds: 0,
    })
}

#[derive(serde::Serialize)]
pub struct TrackingStatusResponse {
    pub is_tracking: bool,
    pub project_id: Option<i64>,
    pub project_path: Option<String>,
    pub elapsed_seconds: u64,
}

/// Iniciar tracking manualmente para un proyecto
#[tauri::command]
pub async fn start_tracking(
    db: State<'_, Database>,
    project_id: i64,
) -> Result<i64, String> {
    println!("▶️ [TRACKING] Iniciando tracking manual para proyecto ID: {}", project_id);

    // Crear una nueva sesión de tracking
    let session_id = db.create_tracking_session(project_id, "manual")
        .map_err(|e| format!("Error starting tracking session: {}", e))?;

    println!("✅ [TRACKING] Sesión {} iniciada", session_id);

    Ok(session_id)
}

/// Detener tracking actual
#[tauri::command]
pub async fn stop_tracking(
    db: State<'_, Database>,
    session_id: i64,
    duration_seconds: i64,
) -> Result<(), String> {
    println!("⏹️ [TRACKING] Deteniendo sesión {} con {} segundos", session_id, duration_seconds);

    db.end_tracking_session(session_id, duration_seconds)
        .map_err(|e| format!("Error stopping tracking session: {}", e))?;

    println!("✅ [TRACKING] Sesión {} terminada", session_id);

    Ok(())
}

/// Verificar si un path tiene configuración de tracking (.gestor/)
#[tauri::command]
pub async fn check_tracking_config(path: String) -> Result<bool, String> {
    let path = std::path::Path::new(&path);
    Ok(has_tracking_config(path))
}

/// Buscar carpeta .gestor/ hacia arriba desde un path
#[tauri::command]
pub async fn find_tracking_project(path: String) -> Result<Option<GestorConfig>, String> {
    let start_path = std::path::Path::new(&path);

    if let Some(project_path) = find_gestor_config(start_path) {
        let config = GestorConfig::load(&project_path)
            .map_err(|e| format!("Error loading tracking config: {}", e))?;
        Ok(Some(config))
    } else {
        Ok(None)
    }
}

/// Obtener estadísticas de tiempo para un proyecto
#[tauri::command]
pub async fn get_time_stats(
    db: State<'_, Database>,
    project_id: i64,
) -> Result<TimeStats, String> {
    db.get_time_stats(project_id)
        .map_err(|e| format!("Error getting time stats: {}", e))
}

/// Respuesta del comando start_work_session
#[derive(serde::Serialize)]
pub struct WorkSessionResponse {
    pub session_id: i64,
    pub project_id: i64,
    pub project_name: String,
    pub previous_session_stopped: bool,
    pub tracking_initialized: bool,
}

/// Iniciar sesión de trabajo - comando principal para "Trabajar"
/// Este comando:
/// 1. Para cualquier sesión activa anterior
/// 2. Inicializa tracking si no existe (.gestor/)
/// 3. Inicia nueva sesión de tiempo
#[tauri::command]
pub async fn start_work_session(
    db: State<'_, Database>,
    active_session: State<'_, ActiveSession>,
    project_id: i64,
) -> Result<WorkSessionResponse, String> {
    println!("🚀 [WORK] Iniciando sesión de trabajo para proyecto ID: {}", project_id);

    // Obtener información del proyecto. No depende del lock de sesión: puede resolverse
    // antes de tomarlo.
    let project = db.get_project(project_id)
        .map_err(|e| format!("Error getting project: {}", e))?;

    let project_path = std::path::Path::new(&project.local_path);

    // Verificar/inicializar tracking si no existe. Es idempotente por proyecto (no por
    // sesión activa), así que tampoco necesita el lock de sesión.
    let mut tracking_initialized = false;
    if project_path.exists() && !has_tracking_config(project_path) {
        println!("📁 [WORK] Inicializando tracking automáticamente para: {}", project.name);
        let _config = init_gestor_config(project_path, project_id, project.name.clone())
            .map_err(|e| format!("Error initializing tracking: {}", e))?;
        tracking_initialized = true;
    }

    let (session_id, previous_stopped) =
        start_new_session_sync(&db, &active_session, project_id, &project.name, &project.local_path)?;

    println!("✅ [WORK] Sesión {} iniciada para: {}", session_id, project.name);

    Ok(WorkSessionResponse {
        session_id,
        project_id,
        project_name: project.name,
        previous_session_stopped: previous_stopped,
        tracking_initialized,
    })
}

/// Sección crítica de `start_work_session`: parar la sesión anterior (si existe) +
/// crear la fila de la nueva sesión en la DB + publicar el nuevo estado en memoria,
/// todo bajo UN ÚNICO lock sostenido de punta a punta.
///
/// Auditoría (CRÍTICO): antes el lock se soltaba entre "parar la anterior" y "escribir
/// el nuevo estado", y `create_tracking_session` corría en esa ventana sin ningún lock:
/// dos invocaciones casi simultáneas de `start_work_session` podían cada una crear su
/// propia fila en la DB y pisarse mutuamente el estado final en memoria, huerfanando
/// una de las dos sesiones (nunca recibía su `ended_at`/`duration`). Extraída como
/// función libre (no `#[tauri::command]`) para poder testearla directamente con
/// instancias reales de `Database`/`ActiveSession`, sin necesitar un `AppHandle`.
/// Devuelve `(session_id, previous_session_stopped)`.
fn start_new_session_sync(
    db: &Database,
    active_session: &ActiveSession,
    project_id: i64,
    project_name: &str,
    project_path: &str,
) -> Result<(i64, bool), String> {
    let mut session_state = active_session.0.lock()
        .map_err(|_| "Error locking session state")?;

    let mut previous_stopped = false;

    if let (Some(prev_session_id), Some(started_at)) = (session_state.session_id, session_state.started_at) {
        // Calcular duración de la sesión anterior
        let duration = started_at.elapsed().as_secs() as i64;

        // Solo guardar si duró más de 10 segundos (evitar sesiones accidentales)
        if duration > 10 {
            println!("⏹️ [WORK] Parando sesión anterior {} ({}s)", prev_session_id, duration);
            db.end_tracking_session(prev_session_id, duration)
                .map_err(|e| format!("Error stopping previous session: {}", e))?;
            previous_stopped = true;
        } else {
            // Eliminar sesión muy corta
            println!("🗑️ [WORK] Descartando sesión muy corta {} ({}s)", prev_session_id, duration);
            // Marcar como 0 segundos para que no cuente
            let _ = db.end_tracking_session(prev_session_id, 0);
        }
    }

    // Limpiar estado previo (todavía bajo el mismo lock)
    session_state.session_id = None;
    session_state.project_id = None;
    session_state.project_name = None;
    session_state.project_path = None;
    session_state.started_at = None;

    // Crear nueva sesión de tracking (todavía bajo el mismo lock)
    let session_id = db.create_tracking_session(project_id, "work_button")
        .map_err(|e| format!("Error creating tracking session: {}", e))?;

    // Publicar el nuevo estado global (todavía bajo el mismo lock)
    session_state.session_id = Some(session_id);
    session_state.project_id = Some(project_id);
    session_state.project_name = Some(project_name.to_string());
    session_state.project_path = Some(project_path.to_string());
    session_state.started_at = Some(std::time::Instant::now());

    Ok((session_id, previous_stopped))
}

/// Lógica compartida para parar la sesión de tracking activa (si existe) y persistir
/// su duración final. La usan tanto el comando `stop_work_session` (parada manual desde
/// la UI) como el handler `on_window_event(CloseRequested)` en `main.rs` (parada
/// best-effort al cerrar la ventana). Es síncrona a propósito: ambos call sites pueden
/// invocarla sin `.await` (el handler de cierre de ventana no es async).
pub fn stop_active_session_sync(
    db: &Database,
    active_session: &ActiveSession,
) -> Result<Option<i64>, String> {
    let mut session_state = active_session.0.lock()
        .map_err(|_| "Error locking session state")?;

    if let (Some(session_id), Some(started_at)) = (session_state.session_id, session_state.started_at) {
        let duration = started_at.elapsed().as_secs() as i64;

        println!("⏹️ [WORK] Parando sesión {} con {} segundos", session_id, duration);

        db.end_tracking_session(session_id, duration)
            .map_err(|e| format!("Error stopping session: {}", e))?;

        // Limpiar estado
        session_state.session_id = None;
        session_state.project_id = None;
        session_state.project_name = None;
        session_state.project_path = None;
        session_state.started_at = None;

        Ok(Some(duration))
    } else {
        println!("ℹ️ [WORK] No hay sesión activa para parar");
        Ok(None)
    }
}

/// Parar la sesión de trabajo actual manualmente
#[tauri::command]
pub async fn stop_work_session(
    db: State<'_, Database>,
    active_session: State<'_, ActiveSession>,
) -> Result<Option<i64>, String> {
    println!("⏹️ [WORK] Parando sesión de trabajo actual");
    stop_active_session_sync(&db, &active_session)
}

/// Armar el estado de la sesión de trabajo a partir del estado global.
///
/// Extraída como función libre (mismo criterio que `start_new_session_sync` y
/// `stop_active_session_sync`) para poder testearla con un `ActiveSession` real, sin
/// necesitar un `AppHandle`.
fn work_session_status_sync(
    active_session: &ActiveSession,
) -> Result<TrackingStatusResponse, String> {
    let session_state = active_session.0.lock()
        .map_err(|_| "Error locking session state")?;

    // `project_path` sale del path REAL capturado al arrancar la sesión, no del nombre
    // del proyecto (ver `ActiveSessionState::project_path`).
    if let (Some(project_id), Some(started_at), Some(ref project_path)) =
        (session_state.project_id, session_state.started_at, &session_state.project_path) {
        Ok(TrackingStatusResponse {
            is_tracking: true,
            project_id: Some(project_id),
            project_path: Some(project_path.clone()),
            elapsed_seconds: started_at.elapsed().as_secs(),
        })
    } else {
        Ok(TrackingStatusResponse {
            is_tracking: false,
            project_id: None,
            project_path: None,
            elapsed_seconds: 0,
        })
    }
}

/// Obtener estado de la sesión de trabajo actual (mejorado)
#[tauri::command]
pub async fn get_work_session_status(
    active_session: State<'_, ActiveSession>,
) -> Result<TrackingStatusResponse, String> {
    work_session_status_sync(&active_session)
}

#[cfg(test)]
mod backup_filename_sanitization_tests {
    use super::sanitize_backup_filename_component;
    use std::path::{Component, Path};

    /// Auditoría (ALTO): un project.name con separadores de path o '..' no debe
    /// sobrevivir en el nombre de archivo del backup, o escapa la carpeta destino
    /// elegida por el usuario (path traversal).
    #[test]
    fn strips_forward_slashes() {
        let safe = sanitize_backup_filename_component("../../etc/passwd");
        assert!(!safe.contains('/'), "no debe contener '/': {safe}");
        assert!(!safe.contains(".."), "no debe contener '..': {safe}");
    }

    #[test]
    fn strips_backslashes_windows_style() {
        let safe = sanitize_backup_filename_component("..\\..\\Windows\\System32\\evil");
        assert!(!safe.contains('\\'), "no debe contener '\\': {safe}");
        assert!(!safe.contains(".."), "no debe contener '..': {safe}");
    }

    #[test]
    fn strips_parent_dir_references_without_slashes() {
        let safe = sanitize_backup_filename_component("proyecto....secreto");
        assert!(!safe.contains(".."), "no debe contener '..': {safe}");
    }

    #[test]
    fn mixed_traversal_payload_produces_sane_filename() {
        let safe = sanitize_backup_filename_component("a/../b\\..\\c/../../d");
        assert!(!safe.contains('/'), "no debe contener '/': {safe}");
        assert!(!safe.contains('\\'), "no debe contener '\\': {safe}");
        assert!(!safe.contains(".."), "no debe contener '..': {safe}");
    }

    #[test]
    fn normal_names_are_left_intact_modulo_trim() {
        assert_eq!(sanitize_backup_filename_component("Mi Proyecto"), "Mi Proyecto");
        assert_eq!(sanitize_backup_filename_component("  Mi Proyecto  "), "Mi Proyecto");
    }

    /// Decisión explícita al unificar los tres saneados: el criterio del PDF convertía
    /// los espacios a '_', el de los backups no. Gana "preservar": el espacio no es un
    /// vector de traversal y renombrar rompería las carpetas de backup ya sincronizadas.
    #[test]
    fn spaces_are_preserved_by_design() {
        assert_eq!(
            sanitize_backup_filename_component("Proyecto Con Espacios"),
            "Proyecto Con Espacios"
        );
    }

    /// La garantía que realmente importa: el resultado es UN SOLO componente de ruta,
    /// así que `dir.join(resultado)` nunca puede salirse de `dir`.
    #[test]
    fn traversal_payload_collapses_to_single_path_component() {
        for malicious in ["../../etc/passwd", "..\\..\\Windows\\System32", "/etc/shadow"] {
            let safe = sanitize_backup_filename_component(malicious);
            let as_path = Path::new(&safe);
            assert_eq!(
                as_path.components().count(),
                1,
                "'{malicious}' debería quedar en un único componente, quedó '{safe}'"
            );
            assert!(
                !as_path
                    .components()
                    .any(|c| matches!(c, Component::ParentDir | Component::RootDir)),
                "'{malicious}' no debe dejar componentes '..' ni raíz, quedó '{safe}'"
            );
        }
    }

    /// Regresión del agujero real: `export_project_to_pdf` saneaba sólo espacios y '/',
    /// así que un proyecto llamado `../../etc/passwd` producía una ruta de PDF fuera de
    /// la carpeta del proyecto. El nombre construido debe quedar SIEMPRE dentro.
    #[test]
    fn pdf_filename_from_traversal_name_stays_inside_export_dir() {
        let export_dir = Path::new("/home/user/proyectos/demo");
        let filename = format!(
            "{}_{}.pdf",
            sanitize_backup_filename_component("../../etc/passwd"),
            "20250101_120000"
        );
        let output_path = export_dir.join(&filename);

        assert_eq!(
            output_path.parent(),
            Some(export_dir),
            "el PDF se escribió fuera de la carpeta del proyecto: {output_path:?}"
        );
        assert!(!filename.contains(".."), "filename con '..': {filename}");
        assert!(
            !filename.contains('/') && !filename.contains('\\'),
            "filename con separador de ruta: {filename}"
        );
    }
}

#[cfg(test)]
mod work_session_tests {
    use super::{
        start_new_session_sync, stop_active_session_sync, work_session_status_sync, ActiveSession,
    };
    use crate::db::Database;
    use crate::models::project::CreateProjectDTO;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn test_db() -> Database {
        Database::new(PathBuf::from(":memory:")).expect("Failed to create in-memory database")
    }

    fn seed_project(db: &Database, name: &str) -> i64 {
        seed_project_with_path(db, name, "/tmp/does-not-matter")
    }

    fn seed_project_with_path(db: &Database, name: &str, local_path: &str) -> i64 {
        let project = db
            .create_project(CreateProjectDTO {
                name: name.to_string(),
                description: "desc".to_string(),
                local_path: local_path.to_string(),
                documentation_url: None,
                ai_documentation_url: None,
                drive_link: None,
                notes: None,
                image_data: None,
                parent_id: None,
                group_color: None,
                group_icon: None,
            })
            .expect("Failed to seed project");
        project.id
    }

    /// B2: `get_work_session_status` devolvía `project_path: Some(project_name.clone())`,
    /// o sea el NOMBRE del proyecto donde el contrato promete una RUTA. No era el typo de
    /// una variable disponible: `ActiveSessionState` nunca había capturado el path.
    ///
    /// El fixture usa un nombre y un path deliberadamente distintos y sin ninguna
    /// subcadena en común: si fueran parecidos, el test pasaría con el bug puesto.
    #[test]
    fn work_session_status_returns_the_real_path_not_the_project_name() {
        const NAME: &str = "Mi Proyecto Alfa";
        const PATH: &str = "/srv/repos/zeta-backend";

        let db = test_db();
        let active_session = ActiveSession::default();
        let project_id = seed_project_with_path(&db, NAME, PATH);

        start_new_session_sync(&db, &active_session, project_id, NAME, PATH)
            .expect("start_work_session no debería fallar");

        let status = work_session_status_sync(&active_session).expect("status no debería fallar");

        assert!(status.is_tracking);
        assert_eq!(status.project_id, Some(project_id));
        assert_eq!(
            status.project_path.as_deref(),
            Some(PATH),
            "project_path debe ser el local_path real del proyecto"
        );
        assert_ne!(
            status.project_path.as_deref(),
            Some(NAME),
            "project_path NO debe ser el nombre del proyecto (regresión B2)"
        );
    }

    /// Sin sesión activa el status no inventa un path.
    #[test]
    fn work_session_status_is_empty_without_an_active_session() {
        let active_session = ActiveSession::default();

        let status = work_session_status_sync(&active_session).expect("status no debería fallar");

        assert!(!status.is_tracking);
        assert_eq!(status.project_id, None);
        assert_eq!(status.project_path, None);
        assert_eq!(status.elapsed_seconds, 0);
    }

    /// Parar la sesión limpia también el path: no debe sobrevivir al cierre.
    #[test]
    fn stopping_a_session_clears_the_project_path() {
        const NAME: &str = "Mi Proyecto Alfa";
        const PATH: &str = "/srv/repos/zeta-backend";

        let db = test_db();
        let active_session = ActiveSession::default();
        let project_id = seed_project_with_path(&db, NAME, PATH);

        start_new_session_sync(&db, &active_session, project_id, NAME, PATH).unwrap();
        stop_active_session_sync(&db, &active_session).expect("no se pudo parar la sesión");

        let status = work_session_status_sync(&active_session).unwrap();
        assert!(!status.is_tracking);
        assert_eq!(
            status.project_path, None,
            "el path no debe sobrevivir al cierre de la sesión"
        );
    }

    /// Auditoría (CRÍTICO): dos llamadas a `start_work_session` en rápida sucesión no
    /// deben perder ninguna sesión. Antes del fix, la ventana sin lock entre "parar la
    /// sesión anterior" y "escribir el nuevo estado" permitía que la segunda llamada
    /// creara su propia fila en la DB sin enterarse de la primera, huerfanando una fila
    /// con `ended_at` NULL para siempre. Con el fix (un único lock sostenido), la
    /// segunda llamada siempre ve el estado que dejó la primera y la cierra como
    /// "sesión anterior", así que cada fila queda con `ended_at` seteado salvo la
    /// última (la activa).
    #[test]
    fn two_rapid_start_calls_do_not_lose_any_session() {
        let db = test_db();
        let active_session = ActiveSession::default();
        let project_id = seed_project(&db, "Proyecto de prueba");

        let (session_id_1, previous_stopped_1) =
            start_new_session_sync(&db, &active_session, project_id, "Proyecto de prueba", "/tmp/does-not-matter")
                .expect("primer start_work_session no debería fallar");
        assert!(!previous_stopped_1, "no había sesión previa que parar");

        let (session_id_2, _previous_stopped_2) =
            start_new_session_sync(&db, &active_session, project_id, "Proyecto de prueba", "/tmp/does-not-matter")
                .expect("segundo start_work_session no debería fallar");

        assert_ne!(session_id_1, session_id_2, "cada llamada debe crear su propia fila");

        let sessions = db
            .get_tracking_sessions(project_id, 10)
            .expect("no se pudieron leer las sesiones");
        assert_eq!(sessions.len(), 2, "las dos sesiones deben existir en la DB");

        // La sesión 1 (la vieja) tiene que haber quedado cerrada (ended_at set),
        // sin importar que haya durado <10s (se descarta con duration=0, pero
        // `ended_at` SIEMPRE se setea). La sesión 2 (la activa) queda abierta.
        let session_1 = sessions.iter().find(|s| s.id == Some(session_id_1)).unwrap();
        let session_2 = sessions.iter().find(|s| s.id == Some(session_id_2)).unwrap();

        assert!(
            session_1.ended_at.is_some(),
            "la sesión anterior no debe quedar huérfana con ended_at NULL para siempre"
        );
        assert!(
            session_2.ended_at.is_none(),
            "la sesión activa todavía no debería estar cerrada"
        );

        // El estado en memoria debe apuntar exclusivamente a la última sesión creada.
        let state = active_session.0.lock().unwrap();
        assert_eq!(state.session_id, Some(session_id_2));
    }

    /// Variante bajo concurrencia real (hilos de SO + barrera) del test anterior: aun
    /// disparando ambas llamadas lo más simultáneamente posible contra el mismo
    /// `ActiveSession`/`Database`, el lock único serializa las dos secciones críticas
    /// completas -sin la ventana intermedia que existía antes del fix- así que ninguna
    /// fila queda huérfana.
    #[test]
    fn concurrent_start_calls_serialize_without_orphaning_a_session() {
        let db = Arc::new(test_db());
        let active_session = Arc::new(ActiveSession::default());
        let project_id = seed_project(&db, "Proyecto concurrente");

        let barrier = Arc::new(std::sync::Barrier::new(2));

        let handles: Vec<_> = (0..2)
            .map(|_| {
                let db = Arc::clone(&db);
                let active_session = Arc::clone(&active_session);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    start_new_session_sync(&db, &active_session, project_id, "Proyecto concurrente", "/tmp/does-not-matter")
                        .expect("start_work_session no debería fallar bajo concurrencia")
                })
            })
            .collect();

        let results: Vec<(i64, bool)> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let session_ids: Vec<i64> = results.iter().map(|(id, _)| *id).collect();
        assert_ne!(session_ids[0], session_ids[1], "cada hilo debe crear su propia fila");

        let sessions = db
            .get_tracking_sessions(project_id, 10)
            .expect("no se pudieron leer las sesiones");
        assert_eq!(sessions.len(), 2);

        let open_sessions = sessions.iter().filter(|s| s.ended_at.is_none()).count();
        assert_eq!(
            open_sessions, 1,
            "debe quedar exactamente UNA sesión abierta (la última en tomar el lock); \
             cualquier otro número indica una fila huérfana o pisada"
        );
    }

    /// El handler `on_window_event(CloseRequested)` de `main.rs` llama exactamente a
    /// `stop_active_session_sync` para cerrar cualquier sesión activa al salir de la
    /// app. No es testeable a nivel de ventana real sin un `AppHandle` de Tauri vivo,
    /// así que este test cubre la lógica que ese handler ejecuta: que efectivamente
    /// persiste `ended_at`/`duration_seconds` en la DB. Verificado manualmente además:
    /// abrir un proyecto (arranca sesión), cerrar la ventana de la app, reabrir y
    /// confirmar en la DB que la fila de `time_tracking_sessions` quedó con
    /// `ended_at`/`duration_seconds` completos (no NULL).
    #[test]
    fn stop_active_session_sync_persists_the_open_session_on_close() {
        let db = test_db();
        let active_session = ActiveSession::default();
        let project_id = seed_project(&db, "Proyecto a cerrar");

        let (session_id, _) =
            start_new_session_sync(&db, &active_session, project_id, "Proyecto a cerrar", "/tmp/does-not-matter")
                .expect("start_work_session no debería fallar");

        let stopped = stop_active_session_sync(&db, &active_session)
            .expect("stop_active_session_sync no debería fallar");
        assert_eq!(stopped, Some(0));

        let sessions = db
            .get_tracking_sessions(project_id, 10)
            .expect("no se pudieron leer las sesiones");
        let session = sessions.iter().find(|s| s.id == Some(session_id)).unwrap();
        assert!(
            session.ended_at.is_some(),
            "cerrar la sesión activa (equivalente a CloseRequested) debe setear ended_at"
        );
        assert_eq!(session.duration_seconds, Some(0));

        // El estado en memoria queda limpio: no hay sesión activa colgada.
        let state = active_session.0.lock().unwrap();
        assert!(state.session_id.is_none());
    }

    /// Parar sin sesión activa (p.ej. cerrar la app sin haber tocado "Trabajar") no
    /// debe fallar ni escribir nada.
    #[test]
    fn stop_active_session_sync_is_a_noop_without_an_active_session() {
        let db = test_db();
        let active_session = ActiveSession::default();

        let stopped = stop_active_session_sync(&db, &active_session)
            .expect("no debería fallar aunque no haya sesión activa");
        assert_eq!(stopped, None);
    }
}
