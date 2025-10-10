use crate::db::Database;
use crate::models::{CreateProjectDTO, Project, UpdateProjectDTO};
use std::process::Command;
use std::fs;
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
    db.update_project(id, updates)
        .map_err(|e| format!("Error updating project: {}", e))
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

#[tauri::command]
pub async fn create_project_backup(
    db: State<'_, Database>,
    project_id: i64,
) -> Result<String, String> {
    println!("Creando backup del proyecto ID: {}", project_id);

    // Obtener datos del proyecto
    let project = db
        .get_project(project_id)
        .map_err(|e| format!("Error obteniendo proyecto: {}", e))?;

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
    let backup_path = Path::new(&project.local_path).join(&filename);

    // Escribir archivo
    fs::write(&backup_path, markdown_content)
        .map_err(|e| format!("Error escribiendo archivo: {}", e))?;

    let result_path = backup_path
        .to_str()
        .ok_or("Error convirtiendo ruta")?
        .to_string();

    println!("Backup creado en: {}", result_path);
    Ok(result_path)
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
