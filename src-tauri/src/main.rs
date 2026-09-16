// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backup;
mod commands;
mod config;
mod db;
mod models;
mod platform;
mod pdf_export;
mod tracking;

use db::Database;
use config::ConfigManager;
use commands::ActiveSession;
use tauri::Manager;

fn main() {
    // Ruta de la DB viva: misma resolución que usa `backup::restore_backup` para
    // reemplazarla, así ambas quedan garantizadas en sync (una sola fuente de verdad).
    let db_path = backup::live_db_path().expect("No se pudo resolver la ruta de la base de datos");

    let db = Database::new(db_path).expect("Error al inicializar la base de datos");
    let config_manager = ConfigManager::new().expect("Error al inicializar la configuración");
    let active_session = ActiveSession::default();

    // Auto-backup al arrancar: si está activado y pasó el intervalo desde el último,
    // crea un backup verificado. Corre sincrónico antes de abrir la ventana (rápido
    // para una DB local; un destino lento/de red podría demorar el arranque). Si falla
    // NO aborta: se loguea y la app abre igual.
    if let Err(e) = backup::maybe_auto_backup(&db, &config_manager) {
        eprintln!("⚠️ [AUTO-BACKUP] No se pudo crear el backup automático: {}", e);
    }

    tauri::Builder::default()
        // Auditoría (ALTO): sin guard de instancia única, dos copias de la app corrían
        // en paralelo contra el mismo archivo SQLite, duplicando tiempo trackeado en
        // silencio. Debe ir primero: si ya hay una instancia corriendo, este plugin
        // aborta el arranque de la nueva y en su lugar reenfoca la ventana existente.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.webview_windows().values().next() {
                let _ = window.set_focus();
                let _ = window.unminimize();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(db)
        .manage(config_manager)
        .manage(active_session)
        // Auditoría (CRÍTICO): antes no había ningún on_window_event/CloseRequested
        // registrado, así que `stop_work_session` nunca se llamaba al cerrar la app:
        // la sesión de tracking activa quedaba con ended_at/duration_seconds en NULL
        // para siempre. Esto es best-effort y síncrono (bloquea brevemente el cierre,
        // la app se está yendo de todos modos): usa la misma lógica que el comando
        // manual `stop_work_session` vía `stop_active_session_sync`.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let db = window.state::<Database>();
                let active_session = window.state::<ActiveSession>();
                match commands::stop_active_session_sync(&db, &active_session) {
                    Ok(Some(duration)) => {
                        println!("⏹️ [WORK] Sesión activa cerrada al salir de la app ({}s)", duration);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        eprintln!("⚠️ [WORK] No se pudo cerrar la sesión activa al salir: {}", e);
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_project,
            commands::get_all_projects,
            commands::get_project,
            commands::update_project,
            commands::delete_project,
            commands::restore_project,
            commands::list_trash,
            commands::purge_project,
            commands::empty_trash,
            commands::search_projects,
            commands::open_terminal,
            commands::open_url,
            commands::create_project_backup,
            commands::sync_project_to_backup,
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
            commands::update_project_order,
            commands::get_git_branch,
            commands::get_git_status,
            commands::get_recent_commits,
            commands::get_git_file_count,
            commands::get_git_modified_files,
            commands::git_add,
            commands::git_commit,
            commands::git_push,
            commands::git_pull,
            commands::get_git_remote_url,
            commands::get_git_ahead_behind,
            commands::get_config,
            commands::update_config,
            commands::reset_config,
            commands::detect_programs,
            commands::open_file_manager,
            commands::open_text_editor,
            commands::select_backup_folder,
            commands::select_folder,
            commands::select_file,
            commands::select_files,
            commands::save_file_dialog,
            commands::get_shortcuts_config,
            commands::update_shortcuts_config,
            // Group commands (v0.4.0)
            commands::get_root_projects,
            commands::get_subprojects,
            commands::get_project_with_children,
            commands::count_subprojects,
            commands::assign_project_to_group,
            // PDF Export (v0.4.1)
            commands::export_project_to_pdf,
            // Dashboard (v0.5.0)
            commands::get_dashboard_data,
            // Time Tracking (v0.5.0)
            commands::init_tracking,
            commands::get_tracking_sessions,
            commands::get_tracking_status,
            commands::start_tracking,
            commands::stop_tracking,
            commands::check_tracking_config,
            commands::find_tracking_project,
            commands::get_time_stats,
            // Work Session (v0.5.1) - Tracking automático
            commands::start_work_session,
            commands::stop_work_session,
            commands::get_work_session_status,
            // Backup de la base de datos
            commands::backup_database,
            commands::list_backups,
            commands::restore_backup,
        ])
        .run(tauri::generate_context!())
        .expect("Error al ejecutar la aplicación Tauri");
}
