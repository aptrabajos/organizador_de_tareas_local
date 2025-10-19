// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod db;
mod models;
mod platform;

use db::Database;
use config::ConfigManager;

fn main() {
    // Crear directorio de datos de la app
    let data_dir = dirs::data_local_dir()
        .expect("No se pudo obtener el directorio de datos local")
        .join("gestor-proyectos");

    std::fs::create_dir_all(&data_dir).expect("No se pudo crear el directorio de datos");

    let db_path = data_dir.join("projects.db");

    let db = Database::new(db_path).expect("Error al inicializar la base de datos");
    let config_manager = ConfigManager::new().expect("Error al inicializar la configuración");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(db)
        .invoke_handler(tauri::generate_handler![
            commands::create_project,
            commands::get_all_projects,
            commands::get_project,
            commands::update_project,
            commands::delete_project,
            commands::search_projects,
            commands::open_terminal,
            commands::open_url,
            commands::create_project_backup,
            commands::write_file_to_path,
            commands::sync_project_to_backup,
            commands::sync_project,
            commands::create_project_link,
            commands::get_project_links,
            commands::update_project_link,
            commands::delete_project_link,
            commands::track_project_open,
            commands::add_project_time,
            commands::get_project_stats,
            commands::get_project_activities,
            commands::add_attachment,
            commands::get_attachments,
            commands::delete_attachment,
            commands::create_journal_entry,
            commands::get_journal_entries,
            commands::update_journal_entry,
            commands::delete_journal_entry,
            commands::create_todo,
            commands::get_project_todos,
            commands::update_todo,
            commands::delete_todo,
            commands::update_project_status,
            commands::toggle_pin_project,
            commands::reorder_pinned_projects,
            commands::get_git_branch,
            commands::get_git_status,
            commands::get_recent_commits,
        ])
        .run(tauri::generate_context!())
        .expect("Error al ejecutar la aplicación Tauri");
}
