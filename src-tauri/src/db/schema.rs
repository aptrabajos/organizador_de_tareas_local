use rusqlite::{ffi, Connection, OpenFlags, Result, Transaction, TransactionBehavior};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::Database;

/// Versión del esquema que conoce este binario. Se guarda en `PRAGMA user_version`;
/// una base anterior a las migraciones versionadas está en `0`.
///
/// Cada paso de [`PASOS`] lleva la base de la versión `n - 1` a la `n`. Para agregar
/// uno: escribir `paso_n`, sumarlo al final de `PASOS` y subir esta constante.
pub(crate) const ESQUEMA_ACTUAL: i64 = 5;

/// Los pasos en orden: `PASOS[0]` es el paso 1, y así.
const PASOS: [fn(&Transaction) -> Result<()>; ESQUEMA_ACTUAL as usize] =
    [paso_1, paso_2, paso_3, paso_4, paso_5];

/// Mensaje para el usuario cuando la base la escribió una versión más nueva de la app.
const MENSAJE_ESQUEMA_NUEVO: &str = "La base de datos es de una versión más nueva de Gestor de Proyectos. Actualizá la app antes de abrirla.";

/// Qué encontró `abrir_existente` en `user_version`, comparado con [`ESQUEMA_ACTUAL`].
///
/// El demonio no migra (solo lo hace la app): con `Atrasada` espera a que la app
/// actualice la base y con `MasNueva` queda ocioso.
#[allow(dead_code)] // Sin llamadores hasta que exista el demonio (F3)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoEsquema {
    AlDia,
    Atrasada(i64),
    MasNueva(i64),
}

impl Database {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let mut conn = Connection::open(db_path)?;
        migrar(&mut conn)?;

        Ok(Database {
            conn: Mutex::new(conn),
        })
    }

    /// Abre una base que ya existe, **sin crearla y sin migrarla**: es la apertura del
    /// demonio. Sin `SQLITE_OPEN_CREATE`, una ruta inexistente es un error y no deja un
    /// archivo vacío que la app después tomaría por una base nueva.
    ///
    /// Devuelve la base junto con el estado de su esquema; decidir qué hacer con una
    /// base atrasada o más nueva es del llamador.
    #[allow(dead_code)] // Sin llamadores hasta que exista el demonio (F3)
    pub fn abrir_existente(db_path: &Path) -> Result<(Self, EstadoEsquema)> {
        let conn = Connection::open_with_flags(
            db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        preparar_conexion(&conn)?;

        let version = leer_user_version(&conn)?;
        let estado = match version.cmp(&ESQUEMA_ACTUAL) {
            std::cmp::Ordering::Equal => EstadoEsquema::AlDia,
            std::cmp::Ordering::Less => EstadoEsquema::Atrasada(version),
            std::cmp::Ordering::Greater => EstadoEsquema::MasNueva(version),
        };

        Ok((
            Database {
                conn: Mutex::new(conn),
            },
            estado,
        ))
    }
}

/// Migraciones versionadas (P14). Solo la corre la app, desde `Database::new`.
///
/// 1. Los `PRAGMA` de conexión, fuera de toda transacción.
/// 2. `BEGIN IMMEDIATE`: si otro proceso tiene la base tomada más allá del
///    `busy_timeout`, el error sube y no se toca nada.
/// 3. Con `user_version` mayor que [`ESQUEMA_ACTUAL`], rollback y error en español.
/// 4. Los pasos que faltan, en orden. Cualquier error los deshace a todos.
/// 5. `user_version = ESQUEMA_ACTUAL` y `COMMIT`. `user_version` vive en la página 1
///    de la base, así que entra en la misma transacción: o queda todo o nada.
pub(crate) fn migrar(conn: &mut Connection) -> Result<()> {
    preparar_conexion(conn)?;
    aplicar_pasos(conn)
}

/// `PRAGMA` de cada conexión, la de la app y la del demonio.
fn preparar_conexion(conn: &Connection) -> Result<()> {
    // Auditoría (ALTO): sin busy_timeout, cualquier contención de escritura entre
    // conexiones (p.ej. el backup por VACUUM INTO corriendo en paralelo a un write,
    // o -antes de agregar el guard de instancia única- dos copias de la app contra
    // el mismo archivo) tiraba SQLITE_BUSY crudo en vez de esperar. 5s es margen de
    // sobra para una DB local de este tamaño sin bloquear la UI de forma perceptible.
    // Nota: journal_mode = WAL se evaluó y se descartó por ahora: `backup_to` usa
    // `VACUUM INTO` bajo el mismo Mutex que serializa todas las escrituras (ver más
    // abajo), así que no hay concurrencia real que aprovechar, y WAL agrega archivos
    // -wal/-shm cuyo ciclo de vida complicaría el copiado/restore de backups sin
    // beneficio medible acá. Queda documentado por si en el futuro se justifica.
    conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

    // Activar el enforcement de FOREIGN KEY (SQLite lo trae OFF por conexión salvo
    // que se pida explícitamente). Las 6 tablas hijas ya declaran
    // `ON DELETE CASCADE`, pero esas cláusulas eran inertes sin este PRAGMA: la
    // corrección de purge/empty_trash dependía 100% de que `purge_project_internal`
    // se mantuviera sincronizado a mano con el schema. Con el PRAGMA activo queda
    // como red de seguridad real a nivel motor. `purge_project_internal` ya borra
    // manualmente en el orden correcto (hijos antes que el padre), así que el
    // CASCADE queda redundante-pero-inofensivo ahí: nunca llega a dispararse porque
    // para cuando se borra la fila de `projects` sus hijos ya no existen.
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    Ok(())
}

/// Pasos 2 a 5 de [`migrar`], sin tocar los `PRAGMA`: los tests de contención fijan
/// un `busy_timeout` corto antes de llamarla, para no esperar los 5 s de la app.
fn aplicar_pasos(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let version = leer_user_version(&tx)?;
    if version > ESQUEMA_ACTUAL {
        // Soltar `tx` sin commit es un rollback: la base queda como estaba.
        return Err(error_de_esquema(MENSAJE_ESQUEMA_NUEVO));
    }
    if version == ESQUEMA_ACTUAL {
        // Al día: nada que escribir. Se suelta sin commit para no tocar la base.
        return Ok(());
    }

    for paso in &PASOS[version.max(0) as usize..] {
        paso(&tx)?;
    }

    tx.pragma_update(None, "user_version", ESQUEMA_ACTUAL)?;
    tx.commit()
}

fn leer_user_version(conn: &Connection) -> Result<i64> {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
}

/// Error con un mensaje propio, para que llegue tal cual a quien muestra el error
/// (`main.rs` lo antepone con "No se pudo abrir la base de datos …").
fn error_de_esquema(mensaje: &str) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(
        ffi::Error::new(ffi::SQLITE_ERROR),
        Some(mensaje.to_string()),
    )
}

/// `ALTER TABLE … ADD COLUMN` que tolera **solo** que la columna ya exista.
///
/// Antes de las migraciones versionadas, las `ALTER` corrían en cada arranque con
/// `let _`, que tragaba también cualquier otro error (tabla inexistente, disco
/// lleno, base bloqueada) y dejaba el esquema a medias sin avisar. Ahora una columna
/// repetida se ignora (pasa en el paso 1 sobre una base real, que ya las tiene) y
/// todo lo demás sube.
fn agregar_columna(tx: &Transaction, sql: &str) -> Result<()> {
    match tx.execute(sql, []) {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(_, Some(ref mensaje)))
            if mensaje.starts_with("duplicate column name") =>
        {
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Paso 1: la línea de base. Es lo que hacía `Database::new` antes de versionar el
/// esquema, así que sobre una base existente no cambia nada.
fn paso_1(conn: &Transaction) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS projects (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                local_path TEXT NOT NULL,
                documentation_url TEXT,
                ai_documentation_url TEXT,
                drive_link TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        [],
    )?;

    // Migraciones: agregar columnas si no existen
    agregar_columna(
        conn,
        "ALTER TABLE projects ADD COLUMN ai_documentation_url TEXT",
    )?;

    agregar_columna(conn, "ALTER TABLE projects ADD COLUMN notes TEXT")?;

    agregar_columna(conn, "ALTER TABLE projects ADD COLUMN image_data TEXT")?;

    // Agregar campos de tracking para analytics
    agregar_columna(
        conn,
        "ALTER TABLE projects ADD COLUMN last_opened_at DATETIME",
    )?;

    agregar_columna(
        conn,
        "ALTER TABLE projects ADD COLUMN opened_count INTEGER DEFAULT 0",
    )?;

    agregar_columna(
        conn,
        "ALTER TABLE projects ADD COLUMN total_time_seconds INTEGER DEFAULT 0",
    )?;

    // Crear tabla de actividad para timeline
    conn.execute(
        "CREATE TABLE IF NOT EXISTS project_activity (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER NOT NULL,
                activity_type TEXT NOT NULL,
                description TEXT,
                duration_seconds INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
            )",
        [],
    )?;

    // Crear tabla de enlaces de proyectos
    conn.execute(
        "CREATE TABLE IF NOT EXISTS project_links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER NOT NULL,
                link_type TEXT NOT NULL,
                title TEXT NOT NULL,
                url TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
            )",
        [],
    )?;

    // Crear tabla de archivos adjuntos
    conn.execute(
        "CREATE TABLE IF NOT EXISTS project_attachments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER NOT NULL,
                filename TEXT NOT NULL,
                file_data TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                mime_type TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
            )",
        [],
    )?;

    // Crear tabla de journal entries (diario de proyecto)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS project_journal (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER NOT NULL,
                content TEXT NOT NULL,
                tags TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
            )",
        [],
    )?;

    // Agregar campos de Quick Start & Context a projects
    agregar_columna(
        conn,
        "ALTER TABLE projects ADD COLUMN status TEXT DEFAULT 'activo'",
    )?;

    agregar_columna(
        conn,
        "ALTER TABLE projects ADD COLUMN status_changed_at DATETIME",
    )?;

    agregar_columna(
        conn,
        "ALTER TABLE projects ADD COLUMN is_pinned BOOLEAN DEFAULT 0",
    )?;

    agregar_columna(
        conn,
        "ALTER TABLE projects ADD COLUMN pinned_order INTEGER DEFAULT 0",
    )?;

    // Agregar campo para orden personalizado de las cards
    agregar_columna(
        conn,
        "ALTER TABLE projects ADD COLUMN display_order INTEGER DEFAULT 0",
    )?;

    // Agregar campos para sistema de grupos de proyectos (v0.4.0)
    agregar_columna(conn, "ALTER TABLE projects ADD COLUMN parent_id INTEGER")?;

    agregar_columna(conn, "ALTER TABLE projects ADD COLUMN group_color TEXT")?;

    agregar_columna(conn, "ALTER TABLE projects ADD COLUMN group_icon TEXT")?;

    agregar_columna(
        conn,
        "ALTER TABLE projects ADD COLUMN is_group_expanded BOOLEAN DEFAULT 1",
    )?;

    // Crear índice en parent_id para queries eficientes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_projects_parent_id ON projects(parent_id)",
        [],
    )?;

    // Papelera: soft-delete. NULL = activo, timestamp = en papelera.
    agregar_columna(conn, "ALTER TABLE projects ADD COLUMN deleted_at TEXT")?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_projects_deleted_at ON projects(deleted_at) WHERE deleted_at IS NOT NULL", [])?;

    // Migración: agregar updated_at a project_links
    agregar_columna(
        conn,
        "ALTER TABLE project_links ADD COLUMN updated_at DATETIME",
    )?;

    // Crear tabla de TODOs por proyecto
    conn.execute(
        "CREATE TABLE IF NOT EXISTS project_todos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER NOT NULL,
                content TEXT NOT NULL,
                is_completed BOOLEAN DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                completed_at DATETIME,
                FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
            )",
        [],
    )?;

    // Crear tabla de sesiones de time tracking
    conn.execute(
        "CREATE TABLE IF NOT EXISTS time_tracking_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER NOT NULL,
                started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                ended_at DATETIME,
                duration_seconds INTEGER,
                source TEXT DEFAULT 'shell_hook',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
            )",
        [],
    )?;

    // Crear índices para time tracking
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_project ON time_tracking_sessions(project_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_started ON time_tracking_sessions(started_at)",
        [],
    )?;

    Ok(())
}

/// Paso 2 (P4): el latido de las sesiones y el índice para encontrar las abiertas.
///
/// `last_activity_at` se escribe al abrir (igual a `started_at`) y en cada latido:
/// es la hora real con la que se recupera una sesión huérfana. El índice
/// `(project_id, ended_at)` sirve a las consultas de "sesiones abiertas de un
/// proyecto" (`ended_at IS NULL`).
fn paso_2(conn: &Transaction) -> Result<()> {
    agregar_columna(
        conn,
        "ALTER TABLE time_tracking_sessions ADD COLUMN last_activity_at DATETIME",
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_project_ended
         ON time_tracking_sessions(project_id, ended_at)",
        [],
    )?;

    Ok(())
}

/// Paso 3 (B6b): `tracker_status`, una sola fila (`id = 1`) que escribe el demonio y
/// lee la app. Sin fila significa que el demonio nunca corrió.
///
/// Columnas (3.2 del diseño del tracker): identidad del proceso (`pid` en `NULL` al
/// salir limpio), el esquema que conoce, el estado, el latido (cada 15 s), los
/// valores efectivos de la config, los contadores de vigilancia, el fin del
/// recorrido inicial y el último error.
fn paso_3(conn: &Transaction) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tracker_status (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            pid INTEGER,
            version TEXT,
            started_at DATETIME,
            schema_version INTEGER,
            estado TEXT NOT NULL CHECK (estado IN ('activo', 'desactivado', 'esperando_migracion', 'esquema_nuevo')),
            last_heartbeat DATETIME,
            idle_minutes INTEGER,
            min_credit_seconds INTEGER,
            watched_projects INTEGER,
            watched_dirs INTEGER,
            scan_completed_at DATETIME,
            unwatched_projects TEXT,
            last_error TEXT,
            last_error_at DATETIME
        )",
        [],
    )?;

    Ok(())
}

/// Paso 4 (R8-1): las sesiones de Trabajar atadas a la terminal (3.10.1 del diseño del
/// tracker).
///
/// - `trabajar_sesiones`: una fila por token, con su dueño actual, su estado y, al
///   cerrarse, `terminada_at` y el motivo. Los dos `CHECK` de fila impiden una sesión
///   cerrada sin motivo o una viva con motivo. No hay `ultima_actividad_at`: la única
///   fuente es el tramo (`last_activity_at`, o el `ended_at` del último).
/// - `trabajar_anclas`: las shells que reclama la CLI (anclas) y los vetos de pty
///   (`token` en `NULL`).
/// - `trabajar_evidencia` y `archivos_evidencia`: un registro por minuto con
///   actividad, con los bits de lo contado y lo ignorado (DT12, U1).
/// - `terminal_token` y `motivo_cierre` en `time_tracking_sessions`. El `CHECK` de
///   `motivo_cierre` exige motivo a todo tramo cerrado **de una sesión**; las filas
///   viejas, sin token, quedan aceptadas aunque estén cerradas sin motivo. Va como
///   `CHECK` y no como trigger porque tiene que cubrir también el `INSERT`.
/// - Las columnas de presencia y la pausa de Trabajar en `tracker_status`.
/// - Tres índices únicos parciales: una sesión viva por proyecto, un tramo abierto
///   por token y un reclamo vigente por shell.
///
/// `source = 'terminal'` (los tramos que mide el demonio) no cambia el esquema:
/// `source` nunca tuvo `CHECK`.
fn paso_4(conn: &Transaction) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS trabajar_sesiones (
            token TEXT PRIMARY KEY CHECK (length(token) = 32 AND token NOT GLOB '*[^0-9a-f]*'),
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            dueno TEXT NOT NULL CHECK (dueno IN ('demonio', 'app')),
            origen TEXT NOT NULL CHECK (origen IN ('boton', 'cli')),
            estado TEXT NOT NULL CHECK (estado IN ('esperando_terminal', 'esperando_demonio',
                'viva_activa', 'viva_pausada', 'terminal_no_detectada', 'cerrada')),
            creada_at DATETIME NOT NULL,
            ultimo_clic_at DATETIME NOT NULL,
            adoptada_at DATETIME,
            last_seen_at DATETIME,
            terminada_at DATETIME,
            motivo_cierre TEXT CHECK (motivo_cierre IS NULL OR motivo_cierre IN (
                'terminal_cerrada', 'parar_app', 'parar_cli', 'tracker_apagado',
                'tracker_caido', 'proyecto_a_papelera', 'restauracion',
                'cambio_de_proyecto', 'cierre_app', 'huerfana', 'tope_sin_tracker')),
            CHECK ((estado = 'cerrada') = (terminada_at IS NOT NULL)),
            CHECK ((estado = 'cerrada') = (motivo_cierre IS NOT NULL))
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS trabajar_anclas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            token TEXT REFERENCES trabajar_sesiones(token) ON DELETE CASCADE,
            pid INTEGER NOT NULL,
            starttime INTEGER NOT NULL,
            tty_nr INTEGER NOT NULL CHECK (tty_nr <> 0),
            multiplexor TEXT CHECK (multiplexor IS NULL OR multiplexor IN ('tmux', 'screen')),
            desde DATETIME NOT NULL,
            hasta DATETIME,
            motivo_salida TEXT CHECK (motivo_salida IS NULL OR motivo_salida IN (
                'shell_cerrada', 'cambio_de_proyecto', 'parar', 'sin_medir', 'sesion_terminada')),
            CHECK ((hasta IS NULL) = (motivo_salida IS NULL))
        )",
        [],
    )?;

    // Las dos tablas de evidencia comparten las columnas de los segundos sin
    // presencia: con el bit 32 van los dos, sin él ninguno, y el primero nunca es
    // posterior al último.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS trabajar_evidencia (
            token TEXT NOT NULL REFERENCES trabajar_sesiones(token) ON DELETE CASCADE,
            minuto TEXT NOT NULL,
            fuentes INTEGER NOT NULL,
            archivo_rel TEXT,
            ausente_desde_s INTEGER CHECK (ausente_desde_s BETWEEN 0 AND 59),
            ausente_hasta_s INTEGER CHECK (ausente_hasta_s BETWEEN 0 AND 59),
            CHECK ((ausente_desde_s IS NULL) = ((fuentes & 32) = 0)),
            CHECK ((ausente_hasta_s IS NULL) = ((fuentes & 32) = 0)),
            CHECK (ausente_desde_s <= ausente_hasta_s),
            PRIMARY KEY (token, minuto)
        ) WITHOUT ROWID",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS archivos_evidencia (
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            minuto TEXT NOT NULL,
            fuentes INTEGER NOT NULL CHECK (fuentes <> 0 AND (fuentes & ~40) = 0),
            archivo_rel TEXT,
            ausente_desde_s INTEGER CHECK (ausente_desde_s BETWEEN 0 AND 59),
            ausente_hasta_s INTEGER CHECK (ausente_hasta_s BETWEEN 0 AND 59),
            CHECK ((ausente_desde_s IS NULL) = ((fuentes & 32) = 0)),
            CHECK ((ausente_hasta_s IS NULL) = ((fuentes & 32) = 0)),
            CHECK (ausente_desde_s <= ausente_hasta_s),
            PRIMARY KEY (project_id, minuto)
        ) WITHOUT ROWID",
        [],
    )?;

    agregar_columna(
        conn,
        "ALTER TABLE time_tracking_sessions
         ADD COLUMN terminal_token TEXT REFERENCES trabajar_sesiones(token)",
    )?;
    agregar_columna(
        conn,
        "ALTER TABLE time_tracking_sessions
         ADD COLUMN motivo_cierre TEXT
         CHECK (motivo_cierre IS NULL OR motivo_cierre IN (
             'terminal_cerrada', 'parar_app', 'parar_cli', 'tracker_apagado',
             'tracker_caido', 'proyecto_a_papelera', 'restauracion',
             'cambio_de_proyecto', 'cierre_app', 'huerfana', 'tope_sin_tracker',
             'inactividad', 'suspension', 'reloj', 'tracker_detenido', 'sin_vigilancia'))
         CHECK (ended_at IS NULL OR terminal_token IS NULL OR motivo_cierre IS NOT NULL)",
    )?;

    agregar_columna(
        conn,
        "ALTER TABLE tracker_status ADD COLUMN trabajar_idle_minutes INTEGER",
    )?;
    agregar_columna(
        conn,
        "ALTER TABLE tracker_status ADD COLUMN presencia TEXT
         CHECK (presencia IS NULL OR presencia IN ('kde', 'gnome', 'prueba', 'no_disponible'))",
    )?;
    agregar_columna(
        conn,
        "ALTER TABLE tracker_status ADD COLUMN presencia_motivo TEXT",
    )?;
    agregar_columna(
        conn,
        "ALTER TABLE tracker_status ADD COLUMN presencia_at DATETIME",
    )?;

    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_trabajar_sesiones_viva
         ON trabajar_sesiones(project_id) WHERE estado <> 'cerrada'",
        [],
    )?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_tramo_abierto
         ON time_tracking_sessions(terminal_token) WHERE ended_at IS NULL",
        [],
    )?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_trabajar_anclas_vigente
         ON trabajar_anclas(pid, starttime) WHERE hasta IS NULL",
        [],
    )?;

    Ok(())
}

/// Paso 5 (R8-A1): los ajustes manuales de tiempo (3.10.17 del diseño del tracker).
///
/// `ajustes_tiempo` es **solo de agregado** (DT13, decisión 5a): un ajuste no se edita
/// ni se borra, se revierte con otro (`revierte_id`, `UNIQUE`: no se revierte dos
/// veces). Lo garantiza la base, no el código: un trigger rechaza todo `UPDATE` y otro
/// todo `DELETE` de un ajuste cuyo proyecto sigue vivo. La purga de un proyecto (desde
/// la papelera o directa) sí borra sus ajustes en cascada: durante la cascada la fila
/// del proyecto ya no existe, así que el `EXISTS` del trigger da falso.
///
/// `desde`/`hasta` son opcionales y van juntos: a qué rato o día corresponde el ajuste.
/// `segundos` lleva signo y no pasa de 24 h.
fn paso_5(conn: &Transaction) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ajustes_tiempo (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            segundos INTEGER NOT NULL CHECK (segundos <> 0 AND abs(segundos) <= 86400),
            desde DATETIME,
            hasta DATETIME,
            nota TEXT NOT NULL CHECK (length(trim(nota)) > 0),
            creado_en DATETIME NOT NULL,
            origen TEXT NOT NULL CHECK (origen IN ('app', 'cli')),
            revierte_id INTEGER UNIQUE REFERENCES ajustes_tiempo(id),
            CHECK ((desde IS NULL) = (hasta IS NULL)),
            CHECK (hasta IS NULL OR hasta > desde)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS ajustes_tiempo_sin_update BEFORE UPDATE ON ajustes_tiempo
         BEGIN SELECT RAISE(ABORT, 'ajustes_tiempo es solo de agregado'); END",
        [],
    )?;
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS ajustes_tiempo_sin_delete BEFORE DELETE ON ajustes_tiempo
         WHEN EXISTS (SELECT 1 FROM projects WHERE id = OLD.project_id AND deleted_at IS NULL)
         BEGIN SELECT RAISE(ABORT, 'ajustes_tiempo es solo de agregado'); END",
        [],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::super::test_support::*;
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_database_initialization() {
        let db = test_db();
        let conn = db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(tables.contains(&"projects".to_string()));
        assert!(tables.contains(&"project_links".to_string()));
        assert!(tables.contains(&"project_journal".to_string()));
        assert!(tables.contains(&"project_todos".to_string()));
        assert!(tables.contains(&"project_activity".to_string()));
        assert!(tables.contains(&"project_attachments".to_string()));
        assert!(tables.contains(&"time_tracking_sessions".to_string()));
    }

    #[test]
    fn test_foreign_keys_pragma_is_enabled() {
        // Las 6 tablas hijas declaran ON DELETE CASCADE, pero esas cláusulas son
        // inertes si SQLite no tiene el enforcement de FK prendido en la conexión.
        let db = test_db();
        let conn = db.conn.lock().unwrap();
        let fk_on: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            fk_on, 1,
            "PRAGMA foreign_keys debe estar activado en cada conexión abierta por Database::new"
        );
    }

    // --- Migraciones contra un schema real pre-existente ---

    /// Auditoría (ALTO): las 21 `ALTER TABLE` de `Database::new()` descartan sus
    /// errores en silencio (`let _ = conn.execute(...)`). Todos los demás tests parten
    /// de `test_db()`, que abre un `:memory:` con el `CREATE TABLE IF NOT EXISTS` YA
    /// actualizado, así que las ALTER nunca corrían de verdad contra un schema viejo:
    /// el camino de upgrade que atraviesa cada usuario real en cada release nunca se
    /// ejercitaba en tests.
    ///
    /// Este test siembra a mano, en un archivo temporal, un schema deliberadamente
    /// viejo (pre-v0.4.0: sin ninguna de las columnas que las ALTER agregan después
    /// -incluida `ai_documentation_url`-, y sin `updated_at` en `project_links`), con
    /// datos reales adentro, y después corre `Database::new()` sobre ESE MISMO
    /// archivo -el flujo real de upgrade-, verificando que las columnas nuevas
    /// existan y que los datos viejos sigan ahí y sean legibles.
    ///
    /// Desde las migraciones versionadas (P14) las `ALTER` ya no tragan sus errores
    /// (solo `duplicate column name`) y la base termina en `ESQUEMA_ACTUAL`.
    #[test]
    fn test_migrations_apply_to_real_old_schema_and_preserve_data() {
        let dir = TempDir::new().expect("no se pudo crear directorio temporal");
        let db_path = dir.path().join("old_schema.db");

        // 1. Sembrar un schema viejo real (pre-v0.4.0) con datos, y cerrar la conexión.
        sembrar_esquema_viejo(&db_path);

        // 2. Correr las migraciones reales -el mismo Database::new() que usa la app- sobre
        // ese mismo archivo.
        let db = Database::new(db_path.clone())
            .expect("Database::new debería migrar el schema viejo sin errores");

        let conn = db.conn.lock().unwrap();

        // 3a. Las columnas que las ALTER TABLE agregan ahora existen en `projects`.
        let projects_columns: Vec<String> = conn
            .prepare("PRAGMA table_info(projects)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        for expected in [
            "ai_documentation_url",
            "notes",
            "image_data",
            "last_opened_at",
            "opened_count",
            "total_time_seconds",
            "status",
            "status_changed_at",
            "is_pinned",
            "pinned_order",
            "display_order",
            "parent_id",
            "group_color",
            "group_icon",
            "is_group_expanded",
            "deleted_at",
        ] {
            assert!(
                projects_columns.iter().any(|c| c == expected),
                "falta columna migrada '{}' en projects: {:?}",
                expected,
                projects_columns
            );
        }

        // 3b. La columna que la ALTER agrega a `project_links` también existe.
        let links_columns: Vec<String> = conn
            .prepare("PRAGMA table_info(project_links)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(
            links_columns.iter().any(|c| c == "updated_at"),
            "falta columna migrada 'updated_at' en project_links: {:?}",
            links_columns
        );

        // 3c. Las tablas nuevas (CREATE TABLE IF NOT EXISTS) también se crean sobre el
        // schema viejo.
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        for expected in [
            "project_activity",
            "project_attachments",
            "project_journal",
            "project_todos",
            "time_tracking_sessions",
        ] {
            assert!(
                tables.contains(&expected.to_string()),
                "falta tabla '{}' tras migrar: {:?}",
                expected,
                tables
            );
        }

        // 4. Los datos sembrados en el schema viejo siguen presentes y legibles.
        let (name, local_path): (String, String) = conn
            .query_row(
                "SELECT name, local_path FROM projects WHERE name = 'Proyecto Viejo'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("el proyecto sembrado en el schema viejo debería seguir presente");
        assert_eq!(name, "Proyecto Viejo");
        assert_eq!(local_path, "/tmp/viejo");

        let link_title: String = conn
            .query_row(
                "SELECT title FROM project_links WHERE url = 'https://old.example.com'",
                [],
                |row| row.get(0),
            )
            .expect("el link sembrado en el schema viejo debería seguir presente");
        assert_eq!(link_title, "Link Viejo");

        // 5. P14 (a): la base vieja (`user_version = 0`) termina en la versión actual.
        assert_eq!(user_version(&conn), ESQUEMA_ACTUAL);

        // 6. Paso 2 (P4): el latido y el índice de sesiones abiertas.
        assert!(columnas(&conn, "time_tracking_sessions").contains(&"last_activity_at".to_string()));
        assert!(
            objetos_del_esquema(&conn).contains(&"index idx_sessions_project_ended".to_string())
        );

        // 7. Paso 3 (B6b): la tabla del estado del demonio, vacía.
        assert!(objetos_del_esquema(&conn).contains(&"table tracker_status".to_string()));

        // 8. Paso 4 (R8-1): las sesiones de Trabajar, sobre el esquema viejo real.
        afirmar_paso_4(&conn);

        // 9. Paso 5 (R8-A1): los ajustes de tiempo, con sus dos triggers.
        afirmar_paso_5(&conn);

        drop(conn);
        drop(db);
        // `dir` (TempDir) borra el directorio temporal solo al salir de scope.
    }

    // --- P14/B1: migraciones versionadas ---

    /// Deja en `db_path` una base migrada hasta `version` (no hasta la actual), como
    /// la dejó una versión anterior de la app.
    fn base_en_version(db_path: &std::path::Path, version: i64) -> Connection {
        let mut conn = Connection::open(db_path).unwrap();
        preparar_conexion(&conn).unwrap();
        let tx = conn.transaction().unwrap();
        for paso in &PASOS[..version as usize] {
            paso(&tx).unwrap();
        }
        tx.pragma_update(None, "user_version", version).unwrap();
        tx.commit().unwrap();
        conn
    }

    /// Paso 2: una base en la versión 1 gana `last_activity_at` (NULL en las filas
    /// viejas: nunca latieron) y el índice `(project_id, ended_at)`, sin perder datos.
    #[test]
    fn migrar_de_la_version_1_a_la_2_agrega_el_latido_y_el_indice() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("v1.db");
        {
            let conn = base_en_version(&db_path, 1);
            assert!(!columnas(&conn, "time_tracking_sessions")
                .contains(&"last_activity_at".to_string()));
            conn.execute_batch(
                "INSERT INTO projects (id, name, description, local_path) VALUES (1, 'P', 'd', '/tmp/p');
                 INSERT INTO time_tracking_sessions (project_id, started_at, ended_at, duration_seconds, source)
                 VALUES (1, '2026-10-01 10:00:00', '2026-10-01 11:00:00', 3600, 'work_button');",
            )
            .unwrap();
        }

        let db = Database::new(db_path).unwrap();
        let conn = db.conn();
        assert_eq!(user_version(&conn), ESQUEMA_ACTUAL);
        assert!(columnas(&conn, "time_tracking_sessions").contains(&"last_activity_at".to_string()));
        assert!(
            objetos_del_esquema(&conn).contains(&"index idx_sessions_project_ended".to_string())
        );

        let (duracion, latido): (i64, Option<String>) = conn
            .query_row(
                "SELECT duration_seconds, last_activity_at FROM time_tracking_sessions",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(duracion, 3600);
        assert_eq!(latido, None);
    }

    /// Paso 3: una base en la versión 2 gana `tracker_status`, sin fila (el demonio
    /// nunca corrió contra ella).
    #[test]
    fn migrar_de_la_version_2_a_la_3_agrega_tracker_status() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("v2.db");
        {
            let conn = base_en_version(&db_path, 2);
            assert!(!objetos_del_esquema(&conn).contains(&"table tracker_status".to_string()));
        }

        let db = Database::new(db_path).unwrap();
        assert_eq!(user_version(&db.conn()), ESQUEMA_ACTUAL);
        assert!(objetos_del_esquema(&db.conn()).contains(&"table tracker_status".to_string()));
        assert_eq!(db.leer_tracker_status().unwrap(), None);
    }

    /// Lo que deja el paso 4: las cuatro tablas, las columnas nuevas y los tres índices
    /// únicos parciales.
    fn afirmar_paso_4(conn: &Connection) {
        let objetos = objetos_del_esquema(conn);
        for esperado in [
            "table trabajar_sesiones",
            "table trabajar_anclas",
            "table trabajar_evidencia",
            "table archivos_evidencia",
            "index idx_trabajar_sesiones_viva",
            "index idx_sessions_tramo_abierto",
            "index idx_trabajar_anclas_vigente",
        ] {
            assert!(
                objetos.contains(&esperado.to_string()),
                "falta {esperado}: {objetos:?}"
            );
        }
        for indice in [
            "idx_trabajar_sesiones_viva",
            "idx_sessions_tramo_abierto",
            "idx_trabajar_anclas_vigente",
        ] {
            let (unico, parcial): (bool, bool) = conn
                .query_row(
                    "SELECT \"unique\", partial FROM pragma_index_list(
                         (SELECT tbl_name FROM sqlite_master WHERE name = ?1))
                     WHERE name = ?1",
                    [indice],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert!(unico && parcial, "{indice} tiene que ser único y parcial");
        }

        assert_eq!(
            columnas(conn, "trabajar_sesiones"),
            vec![
                "token",
                "project_id",
                "dueno",
                "origen",
                "estado",
                "creada_at",
                "ultimo_clic_at",
                "adoptada_at",
                "last_seen_at",
                "terminada_at",
                "motivo_cierre",
            ],
            "sin ultima_actividad_at: la única fuente es el tramo"
        );
        assert_eq!(
            columnas(conn, "trabajar_anclas"),
            vec![
                "id",
                "token",
                "pid",
                "starttime",
                "tty_nr",
                "multiplexor",
                "desde",
                "hasta",
                "motivo_salida",
            ]
        );
        for tabla in ["trabajar_evidencia", "archivos_evidencia"] {
            let columnas = columnas(conn, tabla);
            assert_eq!(
                columnas[1..],
                [
                    "minuto",
                    "fuentes",
                    "archivo_rel",
                    "ausente_desde_s",
                    "ausente_hasta_s"
                ],
                "{tabla}"
            );
        }
        let sesiones = columnas(conn, "time_tracking_sessions");
        assert!(sesiones.contains(&"terminal_token".to_string()));
        assert!(sesiones.contains(&"motivo_cierre".to_string()));
        let estado = columnas(conn, "tracker_status");
        for columna in [
            "trabajar_idle_minutes",
            "presencia",
            "presencia_motivo",
            "presencia_at",
        ] {
            assert!(estado.contains(&columna.to_string()), "falta {columna}");
        }
    }

    /// Paso 4: una base en la versión 3, con tramos viejos cerrados sin motivo, una
    /// huérfana abierta y el estado del demonio, gana las sesiones de Trabajar sin
    /// perder nada. El `CHECK` de tramo con motivo acepta las filas viejas porque no
    /// tienen token.
    #[test]
    fn migrar_de_la_version_3_a_la_4_agrega_las_sesiones_de_trabajar() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("v3.db");
        {
            let conn = base_en_version(&db_path, 3);
            assert!(!objetos_del_esquema(&conn).contains(&"table trabajar_sesiones".to_string()));
            conn.execute_batch(
                "INSERT INTO projects (id, name, description, local_path) VALUES (1, 'P', 'd', '/tmp/p');
                 INSERT INTO time_tracking_sessions (project_id, started_at, ended_at, duration_seconds, source)
                 VALUES (1, '2026-10-01 10:00:00', '2026-10-01 11:00:00', 3600, 'work_button'),
                        (1, '2026-10-02 10:00:00', '2026-10-02 10:00:05', 5, 'shell_hook');
                 INSERT INTO time_tracking_sessions (project_id, started_at, source)
                 VALUES (1, '2026-10-03 10:00:00', 'work_button');
                 INSERT INTO tracker_status (id, pid, estado, last_heartbeat)
                 VALUES (1, 4242, 'activo', '2026-10-03 10:00:00');",
            )
            .unwrap();
        }

        let db = Database::new(db_path).unwrap();
        let conn = db.conn();
        assert_eq!(user_version(&conn), ESQUEMA_ACTUAL);
        afirmar_paso_4(&conn);

        let filas: Vec<(Option<i64>, Option<String>, Option<String>)> = conn
            .prepare(
                "SELECT duration_seconds, terminal_token, motivo_cierre
                 FROM time_tracking_sessions ORDER BY id",
            )
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            filas,
            vec![
                (Some(3600), None, None),
                (Some(5), None, None),
                (None, None, None)
            ]
        );

        let (pid, presencia): (i64, Option<String>) = conn
            .query_row("SELECT pid, presencia FROM tracker_status", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!((pid, presencia), (4242, None));
        drop(conn);

        // Y la huérfana vieja, sin token, se puede seguir cerrando sin motivo.
        assert!(db.end_tracking_session(3, 0).unwrap());
    }

    /// Lo que deja el paso 5: la tabla de ajustes con sus columnas y los dos triggers
    /// que la hacen solo de agregado (invariante 16 de 6.2).
    fn afirmar_paso_5(conn: &Connection) {
        let objetos = objetos_del_esquema(conn);
        for esperado in [
            "table ajustes_tiempo",
            "trigger ajustes_tiempo_sin_update",
            "trigger ajustes_tiempo_sin_delete",
        ] {
            assert!(
                objetos.contains(&esperado.to_string()),
                "falta {esperado}: {objetos:?}"
            );
        }
        assert_eq!(
            columnas(conn, "ajustes_tiempo"),
            vec![
                "id",
                "project_id",
                "segundos",
                "desde",
                "hasta",
                "nota",
                "creado_en",
                "origen",
                "revierte_id",
            ]
        );
    }

    /// Paso 5: una base en la versión 4, con proyectos, tramos y una sesión de Trabajar
    /// cerrada, gana los ajustes sin perder nada. La tabla nace vacía y ya rechaza un
    /// `UPDATE`.
    #[test]
    fn migrar_de_la_version_4_a_la_5_agrega_los_ajustes() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("v4.db");
        let token = "0123456789abcdef0123456789abcdef";
        {
            let conn = base_en_version(&db_path, 4);
            assert!(!objetos_del_esquema(&conn).contains(&"table ajustes_tiempo".to_string()));
            conn.execute_batch(&format!(
                "INSERT INTO projects (id, name, description, local_path) VALUES (1, 'P', 'd', '/tmp/p');
                 INSERT INTO trabajar_sesiones
                     (token, project_id, dueno, origen, estado, creada_at, ultimo_clic_at,
                      terminada_at, motivo_cierre)
                 VALUES ('{token}', 1, 'app', 'boton', 'cerrada', '2026-10-01 10:00:00',
                         '2026-10-01 10:00:00', '2026-10-01 11:00:00', 'parar_app');
                 INSERT INTO time_tracking_sessions
                     (project_id, started_at, ended_at, duration_seconds, source,
                      terminal_token, motivo_cierre)
                 VALUES (1, '2026-10-01 10:00:00', '2026-10-01 11:00:00', 3600, 'work_button',
                         '{token}', 'parar_app'),
                        (1, '2026-10-02 10:00:00', '2026-10-02 10:30:00', 1800, 'work_button',
                         NULL, NULL);"
            ))
            .unwrap();
        }

        let db = Database::new(db_path).unwrap();
        let conn = db.conn();
        assert_eq!(user_version(&conn), 5);
        assert_eq!(ESQUEMA_ACTUAL, 5);
        afirmar_paso_4(&conn);
        afirmar_paso_5(&conn);

        let tramos: Vec<(i64, Option<String>)> = conn
            .prepare(
                "SELECT duration_seconds, motivo_cierre FROM time_tracking_sessions ORDER BY id",
            )
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            tramos,
            vec![(3600, Some("parar_app".to_string())), (1800, None)]
        );
        let estado: String = conn
            .query_row("SELECT estado FROM trabajar_sesiones", [], |row| row.get(0))
            .unwrap();
        assert_eq!(estado, "cerrada");
        let ajustes: i64 = conn
            .query_row("SELECT COUNT(*) FROM ajustes_tiempo", [], |row| row.get(0))
            .unwrap();
        assert_eq!(ajustes, 0);

        conn.execute(
            "INSERT INTO ajustes_tiempo (project_id, segundos, nota, creado_en, origen)
             VALUES (1, 600, 'Reunión', '2026-10-03 12:00:00', 'app')",
            [],
        )
        .unwrap();
        let err = conn
            .execute("UPDATE ajustes_tiempo SET nota = 'x'", [])
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("ajustes_tiempo es solo de agregado"),
            "{err}"
        );
    }

    /// (b) Una base al día no se toca: ni un paso, ni el `user_version`, ni un commit.
    /// `data_version` lo ve otra conexión, y cambia con cualquier commit que escriba
    /// el archivo.
    #[test]
    fn migrar_una_base_al_dia_no_escribe_nada() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("al_dia.db");
        drop(Database::new(db_path.clone()).unwrap());

        let testigo = Connection::open(&db_path).unwrap();
        let data_version = |c: &Connection| -> i64 {
            c.query_row("PRAGMA data_version", [], |row| row.get(0))
                .unwrap()
        };
        let antes = (data_version(&testigo), objetos_del_esquema(&testigo));

        drop(Database::new(db_path.clone()).expect("la segunda corrida no debe fallar"));

        let despues = (data_version(&testigo), objetos_del_esquema(&testigo));
        assert_eq!(antes, despues, "la segunda corrida no debe escribir nada");
        assert_eq!(user_version(&testigo), ESQUEMA_ACTUAL);
    }

    /// (c) Si otra conexión tiene la base tomada para escribir (`BEGIN IMMEDIATE`) más
    /// allá del `busy_timeout`, la migración falla y no deja nada a medias.
    ///
    /// Llama a `aplicar_pasos` con un `busy_timeout` corto en vez de a `migrar`, que
    /// fija los 5 s de la app: el camino de la transacción es el mismo.
    #[test]
    fn migrar_con_la_base_tomada_por_otra_conexion_falla_sin_cambiar_nada() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("tomada.db");
        sembrar_esquema_viejo(&db_path);

        let otra = Connection::open(&db_path).unwrap();
        otra.execute_batch("BEGIN IMMEDIATE;").unwrap();

        let mut conn = Connection::open(&db_path).unwrap();
        conn.busy_timeout(std::time::Duration::from_millis(50))
            .unwrap();
        let antes = objetos_del_esquema(&conn);

        let err =
            aplicar_pasos(&mut conn).expect_err("con la base tomada la migración debe fallar");
        assert_eq!(
            err.sqlite_error_code(),
            Some(rusqlite::ErrorCode::DatabaseBusy),
            "el error debe ser el de base ocupada: {err}"
        );

        otra.execute_batch("ROLLBACK;").unwrap();
        assert_eq!(objetos_del_esquema(&conn), antes);
        assert_eq!(user_version(&conn), 0);
        assert!(!columnas(&conn, "projects").contains(&"notes".to_string()));
    }

    /// (d) Una base que escribió una versión más nueva de la app no se abre (el
    /// esquema podría tener cosas que esta versión rompería) y queda intacta.
    #[test]
    fn migrar_una_base_de_una_version_mas_nueva_falla_en_espanol_y_no_la_toca() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("futura.db");
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(&format!(
                "CREATE TABLE del_futuro (id INTEGER PRIMARY KEY);
                 PRAGMA user_version = {};",
                ESQUEMA_ACTUAL + 1
            ))
            .unwrap();
        }

        let err = match Database::new(db_path.clone()) {
            Ok(_) => panic!("una base más nueva no se debe abrir"),
            Err(e) => e,
        };
        assert_eq!(
            err.to_string(),
            "La base de datos es de una versión más nueva de Gestor de Proyectos. Actualizá la app antes de abrirla."
        );

        let conn = Connection::open(&db_path).unwrap();
        assert_eq!(user_version(&conn), ESQUEMA_ACTUAL + 1);
        assert_eq!(objetos_del_esquema(&conn), vec!["table del_futuro"]);
    }

    /// (e) Solo se tolera `duplicate column name`. Cualquier otro error de una `ALTER`
    /// sube y deshace todo el paso: antes, el `let _` lo tragaba y la app arrancaba
    /// con el esquema a medias.
    ///
    /// Para provocarlo, `projects` es una vista: el `CREATE TABLE IF NOT EXISTS` la
    /// saltea y la primera `ALTER TABLE projects` falla con "Cannot add a column to a
    /// view".
    #[test]
    fn migrar_propaga_un_error_que_no_es_columna_repetida() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("rota.db");
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch("CREATE VIEW projects AS SELECT 1 AS id;")
                .unwrap();
        }

        let err = match Database::new(db_path.clone()) {
            Ok(_) => panic!("la ALTER sobre una vista tiene que fallar"),
            Err(e) => e,
        };
        assert!(
            !err.to_string().contains("duplicate column name"),
            "tiene que subir el error real: {err}"
        );

        let conn = Connection::open(&db_path).unwrap();
        assert_eq!(user_version(&conn), 0);
        assert_eq!(objetos_del_esquema(&conn), vec!["view projects"]);
    }

    /// Una columna repetida, en cambio, se ignora: es lo que pasa en el paso 1 sobre
    /// la base real, que ya tiene todas las columnas.
    #[test]
    fn agregar_columna_ignora_solo_la_columna_repetida() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t (a TEXT);").unwrap();
        let tx = conn.transaction().unwrap();

        agregar_columna(&tx, "ALTER TABLE t ADD COLUMN b TEXT").unwrap();
        agregar_columna(&tx, "ALTER TABLE t ADD COLUMN b TEXT")
            .expect("la columna repetida se ignora");
        assert!(agregar_columna(&tx, "ALTER TABLE no_existe ADD COLUMN b TEXT").is_err());
    }

    /// (f) El demonio abre sin `SQLITE_OPEN_CREATE`: una ruta inexistente es un error
    /// y no deja un archivo vacío que la app después tomaría por una base nueva.
    #[test]
    fn abrir_existente_sobre_una_ruta_inexistente_no_crea_el_archivo() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("no_existe.db");

        assert!(Database::abrir_existente(&db_path).is_err());
        assert!(!db_path.exists(), "abrir_existente no debe crear la base");
    }

    /// `abrir_existente` informa el estado del esquema y no migra.
    #[test]
    fn abrir_existente_informa_el_estado_del_esquema_sin_migrar() {
        let dir = TempDir::new().unwrap();

        let al_dia = dir.path().join("al_dia.db");
        drop(Database::new(al_dia.clone()).unwrap());
        let (_, estado) = Database::abrir_existente(&al_dia).unwrap();
        assert_eq!(estado, EstadoEsquema::AlDia);

        let vieja = dir.path().join("vieja.db");
        sembrar_esquema_viejo(&vieja);
        let (db, estado) = Database::abrir_existente(&vieja).unwrap();
        assert_eq!(estado, EstadoEsquema::Atrasada(0));
        assert_eq!(user_version(&db.conn()), 0, "abrir_existente no migra");
        assert!(!columnas(&db.conn(), "projects").contains(&"notes".to_string()));

        let futura = dir.path().join("futura.db");
        Connection::open(&futura)
            .unwrap()
            .pragma_update(None, "user_version", ESQUEMA_ACTUAL + 1)
            .unwrap();
        let (_, estado) = Database::abrir_existente(&futura).unwrap();
        assert_eq!(estado, EstadoEsquema::MasNueva(ESQUEMA_ACTUAL + 1));
    }
}
