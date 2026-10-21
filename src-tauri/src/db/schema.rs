use rusqlite::{ffi, Connection, OpenFlags, Result, Transaction, TransactionBehavior};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::Database;

/// Versión del esquema que conoce este binario. Se guarda en `PRAGMA user_version`;
/// una base anterior a las migraciones versionadas está en `0`.
///
/// Cada paso de [`PASOS`] lleva la base de la versión `n - 1` a la `n`. Para agregar
/// uno: escribir `paso_n`, sumarlo al final de `PASOS` y subir esta constante.
pub(crate) const ESQUEMA_ACTUAL: i64 = 1;

/// Los pasos en orden: `PASOS[0]` es el paso 1, y así.
const PASOS: [fn(&Transaction) -> Result<()>; ESQUEMA_ACTUAL as usize] = [paso_1];

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

        drop(conn);
        drop(db);
        // `dir` (TempDir) borra el directorio temporal solo al salir de scope.
    }

    // --- P14/B1: migraciones versionadas ---

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
