use crate::db::Database;
use crate::models::{CreateProjectDTO, Project, UpdateProjectDTO};
use std::process::Command;
use tauri::State;

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
