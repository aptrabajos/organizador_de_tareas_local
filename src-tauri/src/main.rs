// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backup;
mod commands;
mod config;
mod db;
mod logging;
mod models;
mod pdf_export;
mod platform;
mod tracking;

use commands::ActiveSession;
use config::ConfigManager;
use db::Database;
use log::{error, info, warn};
use tauri::Manager;

fn main() {
    // El logger va PRIMERO: todo lo que sigue (resolver la DB, leer la config,
    // el auto-backup) ya loguea, y sin logger instalado esos records se tiran.
    // Arranca en Info; el nivel real de `advanced.log_level` se aplica apenas la
    // config esté leída.
    logging::init();

    if let Err(e) = run() {
        // Un fallo de arranque (la base no abre, la config no se puede crear) sale
        // con el motivo en el log y código 1, no con un panic sin contexto.
        error!("❌ [ARRANQUE] {}", e);
        std::process::exit(1);
    }
}

/// Arranca la app en el orden que impone Tauri 2 (P11/D15, 3.8 del diseño del
/// tracker). Verificado en `tauri-2.8.5/src/app.rs`:
///
/// 1. `Builder::build()` corre `initialize_plugins` al final. Ahí el plugin de
///    instancia única intenta tomar su nombre en D-Bus y, si ya hay otra instancia,
///    le reenvía los argumentos y hace `std::process::exit(0)`. O sea: la segunda
///    instancia sale DENTRO de `build()`, sin haber tocado la base.
/// 2. Entre `build()` y `run()` se abre la base, se lee la config y se registra el
///    estado con `app.manage(...)`, todo síncrono y sin nada lento.
/// 3. `App::run` crea las ventanas de `tauri.conf.json` en el `Ready` del event
///    loop y RECIÉN DESPUÉS llama al closure de `.setup()`. Por eso el estado no
///    puede registrarse en `.setup()`: un `invoke` del primer render podría llegar
///    antes y fallar con "state not managed".
///
/// El trabajo lento (auto-backup) va en un hilo lanzado desde `.setup()`.
fn run() -> Result<(), String> {
    let app = tauri::Builder::default()
        // Auditoría (ALTO): sin guard de instancia única, dos copias de la app corrían
        // en paralelo contra el mismo archivo SQLite, duplicando tiempo trackeado en
        // silencio. Debe ir primero: si ya hay una instancia corriendo, este plugin
        // aborta el arranque de la nueva (dentro de `build()`, ver `run`) y en su
        // lugar reenfoca la ventana existente.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.webview_windows().values().next() {
                let _ = window.set_focus();
                let _ = window.unminimize();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        // Lo único que corre en `.setup()` es lo que puede esperar: la ventana ya
        // existe y el estado ya está registrado (ver `run`).
        .setup(|app| {
            spawn_auto_backup(app.handle().clone());
            Ok(())
        })
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
                        info!(
                            "⏹️ [WORK] Sesión activa cerrada al salir de la app ({}s)",
                            duration
                        );
                    }
                    Ok(None) => {}
                    Err(e) => {
                        warn!(
                            "⚠️ [WORK] No se pudo cerrar la sesión activa al salir: {}",
                            e
                        );
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
        .build(tauri::generate_context!())
        .map_err(|e| format!("No se pudo construir la aplicación Tauri: {}", e))?;

    // Si llegamos acá, esta es la única instancia: el plugin ya habría salido
    // dentro de `build()`. Recién ahora se abre (y migra) la base.
    init_state(&app)?;

    app.run(|_, _| {});
    Ok(())
}

/// Abre la base, lee la config y registra el estado de los comandos. Corre entre
/// `build()` y `run()`, antes de que exista ninguna ventana que pueda invocar.
fn init_state(app: &tauri::App) -> Result<(), String> {
    // Ruta de la DB viva: misma resolución que usa `backup::restore_backup` para
    // reemplazarla, así ambas quedan garantizadas en sync (una sola fuente de verdad).
    let db_path = backup::live_db_path()?;
    let db = Database::new(db_path.clone()).map_err(|e| {
        format!(
            "No se pudo abrir la base de datos {}: {}",
            db_path.display(),
            e
        )
    })?;
    let config_manager = ConfigManager::new()
        .map_err(|e| format!("No se pudo inicializar la configuración: {}", e))?;

    // Acá la perilla `advanced.log_level` de Settings empieza a gobernar de verdad:
    // hasta ahora el campo se validaba, se persistía y no tenía ni un consumidor.
    // Si la config no se pudo leer se sigue con el Info del arranque: quedarse sin
    // logs por no poder leer una preferencia sería el peor de los dos mundos.
    match config_manager.get_config() {
        Ok(cfg) => logging::apply_level(cfg.advanced.log_level),
        Err(e) => warn!(
            "⚠️ [LOG] No se pudo leer advanced.log_level, se mantiene 'info': {}",
            e
        ),
    }

    app.manage(db);
    app.manage(config_manager);
    app.manage(ActiveSession::default());
    Ok(())
}

/// Auto-backup al arrancar: si está activado y pasó el intervalo desde el último,
/// crea un backup verificado. Va en un hilo para no demorar la ventana (un destino
/// lento o de red podía trabar el arranque). Si falla NO aborta: se loguea y la app
/// sigue. Mientras el `VACUUM INTO` tiene tomada la conexión, los comandos que usan
/// la base esperan su turno en el mutex, igual que con cualquier otro comando.
fn spawn_auto_backup(handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        let db = handle.state::<Database>();
        let config_manager = handle.state::<ConfigManager>();
        if let Err(e) = backup::maybe_auto_backup(&db, &config_manager) {
            warn!(
                "⚠️ [AUTO-BACKUP] No se pudo crear el backup automático: {}",
                e
            );
        }
    });
}
