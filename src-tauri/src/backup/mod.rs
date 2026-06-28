//! Módulo de backup REAL de la base de datos.
//!
//! Crea copias transaccionalmente consistentes de `projects.db` usando
//! `VACUUM INTO` (ver `Database::backup_to`), las verifica con un
//! `PRAGMA integrity_check` sobre la copia y, opcionalmente, aplica una política
//! de retención que elimina backups antiguos.
//!
//! Reglas de seguridad:
//! - La carpeta destino se resuelve desde la config o, en su defecto, desde la
//!   plataforma actual. NUNCA se hardcodean rutas.
//! - La retención SOLO borra archivos que matchean el patrón `projects-*.db` y
//!   NUNCA borra el backup más reciente.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;

use crate::config::{AppConfig, ConfigManager};
use crate::db::Database;
use crate::platform::get_platform;

/// Prefijo y sufijo del nombre de archivo de los backups.
const BACKUP_PREFIX: &str = "projects-";
const BACKUP_SUFFIX: &str = ".db";

/// Resultado de un backup recién creado.
#[derive(serde::Serialize)]
pub struct BackupResult {
    pub file_path: String,
    pub size_bytes: u64,
    pub created_at: String,
    pub integrity_ok: bool,
    pub project_count: i64,
}

/// Entrada de la lista de backups existentes.
#[derive(serde::Serialize)]
pub struct BackupEntry {
    pub file_path: String,
    pub filename: String,
    pub size_bytes: u64,
    pub created_at: String,
}

/// Resuelve el directorio destino de los backups.
///
/// Prioridad:
/// 1. `config.backup.default_path` si está presente y no vacío.
/// 2. Ruta predeterminada de la plataforma actual (`get_default_backup_path`).
///
/// Crea el directorio si no existe y verifica que sea escribible con una prueba
/// real (crear y borrar un archivo temporal).
pub fn resolve_backup_dir(config: &AppConfig) -> Result<PathBuf, String> {
    // 1. Ruta configurada por el usuario
    let dir = match &config.backup.default_path {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        // 2. Ruta predeterminada de la plataforma
        _ => get_platform().get_default_backup_path()?,
    };

    // Crear el directorio si no existe
    fs::create_dir_all(&dir)
        .map_err(|e| format!("No se pudo crear el directorio de backups '{}': {}", dir.display(), e))?;

    // Prueba de escritura real: crear y borrar un archivo temporal
    let test_file = dir.join(".write_test");
    fs::write(&test_file, b"test")
        .map_err(|e| format!("El directorio de backups '{}' no es escribible: {}", dir.display(), e))?;
    let _ = fs::remove_file(&test_file);

    Ok(dir)
}

/// Ejecuta un backup manual completo: copia, verificación, persistencia del
/// timestamp y (opcional) retención.
pub fn run_backup(db: &Database, config_mgr: &ConfigManager) -> Result<BackupResult, String> {
    let mut cfg = config_mgr.get_config()?;
    let dir = resolve_backup_dir(&cfg)?;

    // Nombre con timestamp legible para evitar colisiones y facilitar el orden
    let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let filename = format!("{}{}{}", BACKUP_PREFIX, timestamp, BACKUP_SUFFIX);
    let dest = dir.join(&filename);

    // VACUUM INTO falla si el destino existe; ante colisión improbable, abortamos claro
    if dest.exists() {
        return Err(format!(
            "Ya existe un backup con el nombre '{}'. Esperá un segundo y reintentá.",
            filename
        ));
    }

    let dest_str = dest
        .to_str()
        .ok_or_else(|| "La ruta del backup contiene caracteres no válidos".to_string())?;

    // 1. Crear la copia consistente. Si VACUUM INTO falla a mitad (disco lleno / I/O),
    // borrar el archivo parcial para que NUNCA aparezca como un backup válido en la lista.
    db.backup_to(dest_str).map_err(|e| {
        let _ = fs::remove_file(&dest);
        format!("Error al crear el backup: {}", e)
    })?;

    // 2. Verificar la COPIA (no la DB viva)
    let (integrity_ok, project_count) = Database::verify_db_file(dest_str)
        .map_err(|e| format!("Error al verificar el backup: {}", e))?;

    // integrity_check es el bloqueante: si falla, el backup no sirve -> borrar
    if !integrity_ok {
        let _ = fs::remove_file(&dest);
        return Err("El backup no pasó la verificación de integridad".to_string());
    }

    // 3. Tamaño del archivo generado
    let size_bytes = fs::metadata(&dest)
        .map_err(|e| format!("No se pudo leer el tamaño del backup: {}", e))?
        .len();

    // Timestamp legible para mostrar/persistir
    let created_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 4. Persistir el último backup en la configuración (best-effort).
    // El backup YA está creado y verificado: si persistir el metadato falla
    // (p.ej. validate_config rechaza un custom_path de terminal inexistente),
    // NO debemos invalidar un backup correcto. Solo se loguea.
    cfg.backup.last_backup = Some(created_at.clone());
    if let Err(e) = config_mgr.update_config(cfg.clone()) {
        eprintln!("⚠️ [BACKUP] No se pudo persistir last_backup: {}", e);
    }

    // 5. Retención opcional de backups antiguos
    if cfg.backup.cleanup_old_backups {
        // La limpieza no debe hacer fallar un backup exitoso: solo se loguea
        if let Err(e) = apply_retention(&dir, cfg.backup.retention_days) {
            eprintln!("⚠️ [BACKUP] No se pudo aplicar la retención: {}", e);
        }
    }

    Ok(BackupResult {
        file_path: dest_str.to_string(),
        size_bytes,
        created_at,
        integrity_ok,
        project_count,
    })
}

/// Lista los backups existentes en el directorio destino, del más reciente al
/// más antiguo (ordenados por nombre, que incluye el timestamp).
pub fn list_backups(config_mgr: &ConfigManager) -> Result<Vec<BackupEntry>, String> {
    let cfg = config_mgr.get_config()?;
    let dir = resolve_backup_dir(&cfg)?;

    let mut entries: Vec<BackupEntry> = Vec::new();

    let read_dir = fs::read_dir(&dir)
        .map_err(|e| format!("No se pudo leer el directorio de backups: {}", e))?;

    for entry in read_dir.flatten() {
        let path = entry.path();
        if !is_backup_file(&path) {
            continue;
        }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let size_bytes = metadata.len();
        let created_at = format_mtime(&metadata);

        entries.push(BackupEntry {
            file_path: path.to_string_lossy().to_string(),
            filename,
            size_bytes,
            created_at,
        });
    }

    // Más reciente primero: el nombre incluye el timestamp, así que orden desc por nombre
    entries.sort_by(|a, b| b.filename.cmp(&a.filename));

    Ok(entries)
}

/// Aplica la política de retención: borra backups más viejos que `retention_days`.
///
/// Guards de seguridad:
/// - SOLO borra archivos que matchean `projects-*.db`.
/// - NUNCA borra el backup más reciente (aunque supere la retención).
fn apply_retention(dir: &Path, retention_days: u32) -> Result<(), String> {
    let mut backups: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| format!("No se pudo leer el directorio de backups: {}", e))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_backup_file(p))
        .collect();

    if backups.len() <= 1 {
        // Nunca borramos el único/último backup
        return Ok(());
    }

    // Ordenar por nombre desc (más reciente primero) para preservar el primero
    backups.sort_by(|a, b| {
        let na = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let nb = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
        nb.cmp(na)
    });

    let max_age = chrono::Duration::days(retention_days as i64);
    let now = Local::now();

    // Saltear el índice 0 (más reciente) -> nunca se borra
    for path in backups.iter().skip(1) {
        let metadata = match fs::metadata(path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let modified = match metadata.modified() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let modified: chrono::DateTime<Local> = modified.into();
        if now.signed_duration_since(modified) > max_age {
            if let Err(e) = fs::remove_file(path) {
                eprintln!(
                    "⚠️ [BACKUP] No se pudo borrar el backup antiguo '{}': {}",
                    path.display(),
                    e
                );
            }
        }
    }

    Ok(())
}

/// Determina si un path corresponde a un archivo de backup (`projects-*.db`).
fn is_backup_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name.starts_with(BACKUP_PREFIX) && name.ends_with(BACKUP_SUFFIX),
        None => false,
    }
}

/// Formatea la fecha de modificación de un archivo a un string legible.
fn format_mtime(metadata: &fs::Metadata) -> String {
    match metadata.modified() {
        Ok(t) => {
            let dt: chrono::DateTime<Local> = t.into();
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        }
        Err(_) => String::new(),
    }
}
