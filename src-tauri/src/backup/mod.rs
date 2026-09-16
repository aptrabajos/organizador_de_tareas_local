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

use log::warn;
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
    /// Resultado de (re)verificar integridad del archivo en disco. `false` si el
    /// PRAGMA integrity_check falló o si el archivo no pudo leerse como SQLite
    /// (p.ej. truncado por un kill a mitad de un `VACUUM INTO`).
    pub integrity_ok: bool,
}

/// Resultado de una restauración exitosa.
#[derive(serde::Serialize)]
pub struct RestoreResult {
    pub restored_from: String,
    pub project_count: i64,
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

/// Núcleo testeable de la creación de un backup: recibe el directorio destino
/// explícito (en vez de resolverlo desde `ConfigManager`, que en producción SIEMPRE
/// apunta a rutas reales del usuario) para poder testear contra un directorio
/// temporal sin tocar ninguna config ni carpeta real.
///
/// Pasos: VACUUM INTO -> verificar integridad de la COPIA -> tamaño -> manifest
/// de integridad cacheado (ver `write_ok_manifest`). NO persiste `last_backup` ni
/// aplica retención: eso es responsabilidad de `run_backup`.
fn run_backup_to_dir(db: &Database, dir: &Path) -> Result<BackupResult, String> {
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

    // 4. Manifest de integridad: solo se escribe porque integrity_ok == true acá.
    // list_backups() lo usa para no re-correr integrity_check en cada refresh.
    let mtime = fs::metadata(&dest).ok().map(|m| mtime_secs(&m)).unwrap_or(0);
    write_ok_manifest(&dest, size_bytes, mtime);

    // Timestamp legible para mostrar/persistir
    let created_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    Ok(BackupResult {
        file_path: dest_str.to_string(),
        size_bytes,
        created_at,
        integrity_ok,
        project_count,
    })
}

/// Ejecuta un backup manual completo: copia, verificación, persistencia del
/// timestamp y (opcional) retención.
pub fn run_backup(db: &Database, config_mgr: &ConfigManager) -> Result<BackupResult, String> {
    let mut cfg = config_mgr.get_config()?;
    let dir = resolve_backup_dir(&cfg)?;

    let result = run_backup_to_dir(db, &dir)?;
    let created_at = result.created_at.clone();

    // 4. Persistir el último backup en la configuración (best-effort).
    // El backup YA está creado y verificado: si persistir el metadato falla
    // (p.ej. validate_config rechaza un custom_path de terminal inexistente),
    // NO debemos invalidar un backup correcto. Solo se loguea.
    cfg.backup.last_backup = Some(created_at.clone());
    if let Err(e) = config_mgr.update_config(cfg.clone()) {
        warn!("⚠️ [BACKUP] No se pudo persistir last_backup: {}", e);
    }

    // 5. Retención opcional de backups antiguos
    if cfg.backup.cleanup_old_backups {
        // La limpieza no debe hacer fallar un backup exitoso: solo se loguea
        if let Err(e) = apply_retention(&dir, cfg.backup.retention_days) {
            warn!("⚠️ [BACKUP] No se pudo aplicar la retención: {}", e);
        }
    }

    Ok(result)
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

        // Re-verificar integridad (cacheada por mtime+tamaño, ver `verify_backup_cached`):
        // un backup truncado por un kill a mitad de VACUUM INTO tiene el nombre y el
        // tamaño parcial de un backup real, pero NO pasa integrity_check.
        let integrity_ok = verify_backup_cached(&path, size_bytes);

        entries.push(BackupEntry {
            file_path: path.to_string_lossy().to_string(),
            filename,
            size_bytes,
            created_at,
            integrity_ok,
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
            match fs::remove_file(path) {
                Ok(()) => {
                    // El manifest de integridad ya no tiene sentido sin su backup:
                    // best-effort, un huérfano no rompe nada (nunca se lee sin el .db).
                    let _ = fs::remove_file(ok_manifest_path(path));
                }
                Err(e) => warn!(
                    "⚠️ [BACKUP] No se pudo borrar el backup antiguo '{}': {}",
                    path.display(),
                    e
                ),
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

// ==================== CACHÉ DE INTEGRIDAD (manifest .ok) ====================
//
// list_backups() necesita saber si cada backup en disco sigue siendo íntegro sin
// correr un PRAGMA integrity_check completo en cada refresh (costoso para DBs
// grandes). Se cachea el resultado en un archivo sidecar `<nombre>.db.ok` que
// guarda el tamaño+mtime con los que se verificó. Si el archivo del backup
// cambia (tamaño o mtime distintos), el caché se invalida y se re-verifica.
//
// El sidecar NUNCA matchea `is_backup_file` (no termina en ".db"), por lo que
// jamás aparece en list_backups ni es candidato de borrado en apply_retention.

/// Ruta del manifest de integridad (sidecar) de un backup.
fn ok_manifest_path(backup_path: &Path) -> PathBuf {
    let mut name = backup_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    name.push_str(".ok");
    backup_path.with_file_name(name)
}

/// mtime en segundos desde epoch (suficiente resolución para detectar cambios).
fn mtime_secs(metadata: &fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Escribe el manifest de integridad. Best-effort: si falla, el peor caso es que
/// list_backups() vuelva a correr integrity_check real la próxima vez.
fn write_ok_manifest(backup_path: &Path, size_bytes: u64, mtime: i64) {
    let _ = fs::write(ok_manifest_path(backup_path), format!("{}:{}", size_bytes, mtime));
}

/// Lee el manifest de integridad si existe y es parseable.
fn read_ok_manifest(backup_path: &Path) -> Option<(u64, i64)> {
    let contents = fs::read_to_string(ok_manifest_path(backup_path)).ok()?;
    let (size_str, mtime_str) = contents.split_once(':')?;
    Some((size_str.parse().ok()?, mtime_str.parse().ok()?))
}

/// Verifica la integridad de un backup usando el caché cuando es válido; si el
/// caché falta o quedó desactualizado (el archivo cambió desde el último check
/// exitoso), corre `PRAGMA integrity_check` de verdad y actualiza el caché SOLO
/// si pasa. Un backup truncado por un kill a mitad de VACUUM INTO nunca tuvo un
/// manifest válido, así que siempre se re-verifica y da `false`.
fn verify_backup_cached(path: &Path, size_bytes: u64) -> bool {
    let current_mtime = match fs::metadata(path) {
        Ok(m) => mtime_secs(&m),
        Err(_) => return false,
    };

    if let Some((cached_size, cached_mtime)) = read_ok_manifest(path) {
        if cached_size == size_bytes && cached_mtime == current_mtime {
            return true; // No cambió desde el último check exitoso
        }
    }

    let path_str = match path.to_str() {
        Some(s) => s,
        None => return false,
    };

    match Database::verify_db_file(path_str) {
        Ok((true, _)) => {
            write_ok_manifest(path, size_bytes, current_mtime);
            true
        }
        _ => false,
    }
}

// ==================== RESTAURACIÓN DE BACKUPS ====================

/// Ruta absoluta de la DB VIVA (`projects.db`), usando el mismo directorio de
/// datos que `main.rs` para inicializar `Database::new`. Crea el directorio si
/// hiciera falta (idempotente).
pub fn live_db_path() -> Result<PathBuf, String> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| "No se pudo obtener el directorio de datos local".to_string())?
        .join("gestor-proyectos");
    fs::create_dir_all(&data_dir)
        .map_err(|e| format!("No se pudo crear el directorio de datos '{}': {}", data_dir.display(), e))?;
    Ok(data_dir.join("projects.db"))
}

/// Restaura la DB viva a partir de un backup elegido por el usuario.
pub fn restore_backup(backup_path: &str) -> Result<RestoreResult, String> {
    let live_path = live_db_path()?;
    restore_backup_to(backup_path, &live_path)
}

/// Núcleo testeable de la restauración: recibe la ruta de la DB viva explícita
/// (en vez de resolverla con `live_db_path()`, que en producción SIEMPRE apunta
/// al directorio de datos real del usuario) para poder testear el swap completo
/// contra un archivo temporal sin arriesgar la DB real.
///
/// Reglas de seguridad (no negociables, es un reemplazo IRREVERSIBLE de datos
/// reales):
/// 1. El candidato debe matchear el patrón de un backup real (`projects-*.db`).
/// 2. Se verifica integridad del backup elegido ANTES de tocar la DB viva. Si
///    falla, se aborta sin modificar nada.
/// 3. El backup se copia a un archivo temporal en el MISMO directorio que la DB
///    viva (mismo filesystem => el rename final es atómico).
/// 4. Se re-verifica integridad de la COPIA temporal (paranoia: detecta
///    corrupción introducida por el propio `fs::copy`, disco lleno a mitad de
///    copia, etc.). Si falla, se aborta sin tocar la DB viva.
/// 5. Recién ahí se hace `fs::rename` del temporal sobre la ruta viva. Si CUALQUIER
///    paso anterior falla, la DB original queda completamente intacta: el rename
///    es literalmente el último paso.
fn restore_backup_to(backup_path: &str, live_path: &Path) -> Result<RestoreResult, String> {
    let src = Path::new(backup_path);

    // 1. Guard de patrón: evita restaurar un archivo arbitrario pasado por error.
    if !is_backup_file(src) {
        return Err(format!(
            "'{}' no es un archivo de backup válido (se espera 'projects-*.db')",
            backup_path
        ));
    }

    // 2. Verificar integridad del backup elegido ANTES de tocar la DB viva.
    let (integrity_ok, _) = Database::verify_db_file(backup_path)
        .map_err(|e| format!("No se pudo leer el backup elegido: {}", e))?;
    if !integrity_ok {
        return Err(
            "El backup elegido no pasó la verificación de integridad. Restauración cancelada; la base de datos actual NO fue modificada.".to_string(),
        );
    }

    let live_dir = live_path.parent().ok_or_else(|| {
        "No se pudo determinar el directorio de la base de datos viva".to_string()
    })?;
    fs::create_dir_all(live_dir)
        .map_err(|e| format!("No se pudo preparar el directorio de la base de datos: {}", e))?;

    // 3. Copiar a un temporal EN EL MISMO DIRECTORIO que la DB viva.
    let tmp_path = live_dir.join(format!(
        ".restore-tmp-{}.db",
        Local::now().format("%Y%m%d%H%M%S%3f")
    ));
    fs::copy(src, &tmp_path)
        .map_err(|e| format!("No se pudo copiar el backup a un archivo temporal: {}", e))?;

    let tmp_str = match tmp_path.to_str() {
        Some(s) => s,
        None => {
            let _ = fs::remove_file(&tmp_path);
            return Err("La ruta temporal contiene caracteres no válidos".to_string());
        }
    };

    // 4. Re-verificar integridad de LA COPIA temporal antes de tocar la DB viva.
    let (tmp_ok, project_count) = match Database::verify_db_file(tmp_str) {
        Ok(r) => r,
        Err(e) => {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!(
                "No se pudo verificar la copia temporal: {}. La base de datos actual NO fue modificada.",
                e
            ));
        }
    };
    if !tmp_ok {
        let _ = fs::remove_file(&tmp_path);
        return Err("La copia temporal no pasó la verificación de integridad. La base de datos actual NO fue modificada.".to_string());
    }

    // 5. Rename atómico: único paso que efectivamente reemplaza la DB viva, y solo
    // se llega acá si 1-4 fueron exitosos.
    fs::rename(&tmp_path, live_path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!(
            "No se pudo reemplazar la base de datos viva: {}. La base de datos actual NO fue modificada.",
            e
        )
    })?;

    Ok(RestoreResult {
        restored_from: backup_path.to_string(),
        project_count,
    })
}

/// Auto-backup al ARRANCAR la app: si está activado y corresponde por intervalo,
/// crea un backup. Se llama UNA vez en el arranque (sin timers ni hilos vivos). El
/// run_backup es sincrónico (rápido para una DB local). Nunca debe romper el arranque:
/// el caller loguea el Err y sigue.
pub fn maybe_auto_backup(db: &Database, config_mgr: &ConfigManager) -> Result<(), String> {
    let cfg = config_mgr.get_config()?;
    if !cfg.backup.auto_backup_enabled {
        return Ok(()); // desactivado por el usuario
    }
    if !auto_backup_due(cfg.backup.last_backup.as_deref(), cfg.backup.auto_backup_interval) {
        return Ok(()); // todavía no pasó el intervalo
    }
    // Reusa el flujo verificado (VACUUM INTO + verificación + retención + last_backup).
    run_backup(db, config_mgr)?;
    Ok(())
}

/// ¿Corresponde un auto-backup ahora? Envuelve el núcleo testeable con la hora real.
fn auto_backup_due(last_backup: Option<&str>, interval_days: u32) -> bool {
    auto_backup_due_at(last_backup, interval_days, Local::now().naive_local())
}

/// Núcleo testeable: corresponde backup si nunca hubo (None o no parsea) o si pasó el
/// intervalo en días desde el último backup.
fn auto_backup_due_at(
    last_backup: Option<&str>,
    interval_days: u32,
    now: chrono::NaiveDateTime,
) -> bool {
    match last_backup {
        None => true,
        Some(ts) => match chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S") {
            Ok(last) => {
                now.signed_duration_since(last) >= chrono::Duration::days(interval_days as i64)
            }
            // Defensivo: si la fecha guardada no parsea, mejor hacer un backup.
            Err(_) => true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::CreateProjectDTO;
    use chrono::NaiveDateTime;
    use std::fs::OpenOptions;
    use std::time::{Duration, SystemTime};

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    #[test]
    fn due_when_never_backed_up() {
        assert!(auto_backup_due_at(None, 7, dt("2026-06-29 10:00:00")));
    }

    #[test]
    fn due_when_interval_passed() {
        // último hace 8 días, intervalo 7 → corresponde
        assert!(auto_backup_due_at(
            Some("2026-06-21 10:00:00"),
            7,
            dt("2026-06-29 10:00:00")
        ));
    }

    #[test]
    fn not_due_when_recent() {
        // último hace 3 días, intervalo 7 → NO corresponde
        assert!(!auto_backup_due_at(
            Some("2026-06-26 10:00:00"),
            7,
            dt("2026-06-29 10:00:00")
        ));
    }

    #[test]
    fn due_when_timestamp_unparseable() {
        assert!(auto_backup_due_at(Some("no-es-fecha"), 7, dt("2026-06-29 10:00:00")));
    }

    // ==================== Helpers de tests de backup/restore ====================

    fn test_db() -> Database {
        Database::new(PathBuf::from(":memory:")).expect("no se pudo crear la DB de prueba")
    }

    fn test_project_dto(name: &str) -> CreateProjectDTO {
        CreateProjectDTO {
            name: name.to_string(),
            description: "desc de prueba".to_string(),
            local_path: "/tmp/test".to_string(),
            documentation_url: None,
            ai_documentation_url: None,
            drive_link: None,
            notes: None,
            image_data: None,
            parent_id: None,
            group_color: None,
            group_icon: None,
        }
    }

    /// Fuerza el mtime de un archivo a `days_ago` días atrás de "ahora".
    fn set_mtime_days_ago(path: &Path, days_ago: i64) {
        let target = SystemTime::now() - Duration::from_secs((days_ago.max(0) as u64) * 86_400);
        let file = OpenOptions::new()
            .write(true)
            .open(path)
            .expect("no se pudo abrir el archivo para ajustar su mtime");
        file.set_modified(target).expect("no se pudo ajustar el mtime");
    }

    /// Crea un backup real (`projects-<suffix>.db`) con `project_count` proyectos en
    /// `dir`, ajusta su mtime a `days_ago` días atrás, y escribe su manifest de
    /// integridad (igual que haría `run_backup_to_dir` en un backup exitoso).
    fn make_backup_file(dir: &Path, suffix: &str, project_count: usize, days_ago: i64) -> PathBuf {
        let db = test_db();
        for i in 0..project_count {
            db.create_project(test_project_dto(&format!("p{}", i))).unwrap();
        }
        let dest = dir.join(format!("{}{}{}", BACKUP_PREFIX, suffix, BACKUP_SUFFIX));
        db.backup_to(dest.to_str().unwrap()).unwrap();
        set_mtime_days_ago(&dest, days_ago);

        let metadata = fs::metadata(&dest).unwrap();
        write_ok_manifest(&dest, metadata.len(), mtime_secs(&metadata));

        dest
    }

    // ==================== run_backup_to_dir ====================

    #[test]
    fn run_backup_to_dir_produces_verifiable_backup() {
        let db = test_db();
        db.create_project(test_project_dto("uno")).unwrap();
        let tmp = tempfile::tempdir().unwrap();

        let result =
            run_backup_to_dir(&db, tmp.path()).expect("run_backup_to_dir debe funcionar");

        assert!(
            result.integrity_ok,
            "el backup recién creado debe pasar integrity_check"
        );
        assert_eq!(result.project_count, 1);
        assert!(
            Path::new(&result.file_path).exists(),
            "el archivo de backup debe existir en disco"
        );

        let (ok, count) = Database::verify_db_file(&result.file_path).unwrap();
        assert!(ok, "verify_db_file debe confirmar integridad del archivo generado");
        assert_eq!(count, 1);

        // El manifest de integridad debe existir tras un backup exitoso, para que
        // list_backups() no tenga que re-correr integrity_check en cada refresh.
        let manifest = ok_manifest_path(Path::new(&result.file_path));
        assert!(
            manifest.exists(),
            "debe existir el manifest .ok tras un backup exitoso"
        );
    }

    // ==================== apply_retention ====================

    #[test]
    fn apply_retention_never_deletes_the_only_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let only = make_backup_file(tmp.path(), "20260101-000000", 0, 100); // muy viejo

        apply_retention(tmp.path(), 7).unwrap();

        assert!(
            only.exists(),
            "el único backup existente NUNCA debe borrarse, sin importar su edad"
        );
    }

    #[test]
    fn apply_retention_keeps_all_within_retention_period() {
        let tmp = tempfile::tempdir().unwrap();
        let a = make_backup_file(tmp.path(), "20260101-000000", 0, 3);
        let b = make_backup_file(tmp.path(), "20260102-000000", 0, 1);
        let c = make_backup_file(tmp.path(), "20260103-000000", 0, 0);

        apply_retention(tmp.path(), 7).unwrap();

        assert!(a.exists(), "dentro del período de retención, no se borra");
        assert!(b.exists(), "dentro del período de retención, no se borra");
        assert!(c.exists(), "dentro del período de retención, no se borra");
    }

    #[test]
    fn apply_retention_deletes_old_backups_but_protects_newest_and_recent() {
        let tmp = tempfile::tempdir().unwrap();
        // Nombre más viejo (índice != 0 al ordenar desc) y mtime muy viejo -> se borra.
        let oldest = make_backup_file(tmp.path(), "20260101-000000", 0, 100);
        // Nombre intermedio, mtime reciente -> sobrevive por estar dentro de retención.
        let recent_middle = make_backup_file(tmp.path(), "20260105-000000", 0, 3);
        // Nombre más nuevo (índice 0 al ordenar desc) con mtime viejo -> protegido por
        // ser el backup más reciente, aunque supere la retención.
        let newest_but_old_mtime = make_backup_file(tmp.path(), "20260110-000000", 0, 50);

        assert!(
            ok_manifest_path(&oldest).exists(),
            "sanity: el manifest se creó junto al backup"
        );

        apply_retention(tmp.path(), 7).unwrap();

        assert!(!oldest.exists(), "un backup viejo y no-protegido debe borrarse");
        assert!(
            !ok_manifest_path(&oldest).exists(),
            "el manifest .ok huérfano también debe limpiarse al borrar su backup"
        );
        assert!(
            recent_middle.exists(),
            "un backup dentro de la retención NUNCA se borra, aunque no sea el más nuevo"
        );
        assert!(
            newest_but_old_mtime.exists(),
            "el backup más reciente (por nombre) nunca se borra, aunque supere la retención"
        );
    }

    // ==================== restore_backup_to ====================

    #[test]
    fn restore_backup_to_swaps_live_db_when_backup_is_valid() {
        let backups_dir = tempfile::tempdir().unwrap();
        let live_dir = tempfile::tempdir().unwrap();
        let live_path = live_dir.path().join("projects.db");

        // DB "vieja" en la ruta viva, con datos distintos a los del backup.
        let old_db = Database::new(live_path.clone()).unwrap();
        old_db.create_project(test_project_dto("viejo")).unwrap();
        drop(old_db);

        // Backup válido con 2 proyectos.
        let backup_path = make_backup_file(backups_dir.path(), "20260101-000000", 2, 0);

        let result = restore_backup_to(backup_path.to_str().unwrap(), &live_path)
            .expect("la restauración debe funcionar con un backup íntegro");

        assert_eq!(result.project_count, 2);
        let (ok, count) = Database::verify_db_file(live_path.to_str().unwrap()).unwrap();
        assert!(ok);
        assert_eq!(count, 2, "la DB viva debe tener el contenido del backup, no el viejo");

        // No debe quedar ningún temporal de restauración huérfano.
        let leftovers: Vec<_> = fs::read_dir(live_dir.path())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with(".restore-tmp-"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "no debe quedar ningún temporal de restauración tras un swap exitoso"
        );
    }

    #[test]
    fn restore_backup_to_rejects_corrupt_backup_and_leaves_live_db_intact() {
        let backups_dir = tempfile::tempdir().unwrap();
        let live_dir = tempfile::tempdir().unwrap();
        let live_path = live_dir.path().join("projects.db");

        let old_db = Database::new(live_path.clone()).unwrap();
        old_db.create_project(test_project_dto("original")).unwrap();
        drop(old_db);

        // "Backup" corrupto: nombre válido, contenido NO es SQLite.
        let corrupt = backups_dir.path().join("projects-corrupt.db");
        fs::write(&corrupt, b"esto no es una base de datos sqlite").unwrap();

        let res = restore_backup_to(corrupt.to_str().unwrap(), &live_path);
        assert!(res.is_err(), "un backup corrupto nunca debe restaurarse");

        let (ok, count) = Database::verify_db_file(live_path.to_str().unwrap()).unwrap();
        assert!(ok, "la DB viva original debe seguir intacta");
        assert_eq!(count, 1, "la DB viva NO debe haber cambiado ante un backup corrupto");
    }

    #[test]
    fn restore_backup_to_rejects_paths_that_dont_look_like_backups() {
        let live_dir = tempfile::tempdir().unwrap();
        let live_path = live_dir.path().join("projects.db");
        let old_db = Database::new(live_path.clone()).unwrap();
        old_db.create_project(test_project_dto("original")).unwrap();
        drop(old_db);

        let res = restore_backup_to("/tmp/no-es-un-backup.txt", &live_path);
        assert!(res.is_err(), "un path que no matchea 'projects-*.db' debe rechazarse");

        let (ok, count) = Database::verify_db_file(live_path.to_str().unwrap()).unwrap();
        assert!(ok);
        assert_eq!(count, 1, "la DB viva NO debe haber cambiado");
    }
}
