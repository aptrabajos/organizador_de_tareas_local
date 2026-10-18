use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

use super::Database;

impl Database {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let conn = Connection::open(db_path)?;

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
        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN ai_documentation_url TEXT",
            [],
        );

        let _ = conn.execute("ALTER TABLE projects ADD COLUMN notes TEXT", []);

        let _ = conn.execute("ALTER TABLE projects ADD COLUMN image_data TEXT", []);

        // Agregar campos de tracking para analytics
        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN last_opened_at DATETIME",
            [],
        );

        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN opened_count INTEGER DEFAULT 0",
            [],
        );

        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN total_time_seconds INTEGER DEFAULT 0",
            [],
        );

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
        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN status TEXT DEFAULT 'activo'",
            [],
        );

        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN status_changed_at DATETIME",
            [],
        );

        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN is_pinned BOOLEAN DEFAULT 0",
            [],
        );

        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN pinned_order INTEGER DEFAULT 0",
            [],
        );

        // Agregar campo para orden personalizado de las cards
        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN display_order INTEGER DEFAULT 0",
            [],
        );

        // Agregar campos para sistema de grupos de proyectos (v0.4.0)
        let _ = conn.execute("ALTER TABLE projects ADD COLUMN parent_id INTEGER", []);

        let _ = conn.execute("ALTER TABLE projects ADD COLUMN group_color TEXT", []);

        let _ = conn.execute("ALTER TABLE projects ADD COLUMN group_icon TEXT", []);

        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN is_group_expanded BOOLEAN DEFAULT 1",
            [],
        );

        // Crear índice en parent_id para queries eficientes
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_projects_parent_id ON projects(parent_id)",
            [],
        );

        // Papelera: soft-delete. NULL = activo, timestamp = en papelera.
        let _ = conn.execute("ALTER TABLE projects ADD COLUMN deleted_at TEXT", []);
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_projects_deleted_at ON projects(deleted_at) WHERE deleted_at IS NOT NULL",
            [],
        );

        // Migración: agregar updated_at a project_links
        let _ = conn.execute(
            "ALTER TABLE project_links ADD COLUMN updated_at DATETIME",
            [],
        );

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
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_project ON time_tracking_sessions(project_id)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_started ON time_tracking_sessions(started_at)",
            [],
        );

        Ok(Database {
            conn: Mutex::new(conn),
        })
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::super::test_support::*;
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
    #[test]
    fn test_migrations_apply_to_real_old_schema_and_preserve_data() {
        use tempfile::TempDir;

        let dir = TempDir::new().expect("no se pudo crear directorio temporal");
        let db_path = dir.path().join("old_schema.db");

        // 1. Sembrar un schema viejo real (pre-v0.4.0) con datos, y cerrar la conexión.
        {
            let old_conn = Connection::open(&db_path).expect("no se pudo crear el schema viejo");
            old_conn
                .execute(
                    "CREATE TABLE projects (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        name TEXT NOT NULL,
                        description TEXT NOT NULL,
                        local_path TEXT NOT NULL,
                        documentation_url TEXT,
                        drive_link TEXT,
                        created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                        updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                    )",
                    [],
                )
                .unwrap();
            old_conn
                .execute(
                    "CREATE TABLE project_links (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        project_id INTEGER NOT NULL,
                        link_type TEXT NOT NULL,
                        title TEXT NOT NULL,
                        url TEXT NOT NULL,
                        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                    )",
                    [],
                )
                .unwrap();

            old_conn
                .execute(
                    "INSERT INTO projects (name, description, local_path, documentation_url, drive_link)
                     VALUES ('Proyecto Viejo', 'Sembrado en schema pre-v0.4.0', '/tmp/viejo', NULL, NULL)",
                    [],
                )
                .unwrap();
            let project_id = old_conn.last_insert_rowid();
            old_conn
                .execute(
                    "INSERT INTO project_links (project_id, link_type, title, url)
                     VALUES (?1, 'repo', 'Link Viejo', 'https://old.example.com')",
                    params![project_id],
                )
                .unwrap();
        } // old_conn se dropea acá, liberando el archivo antes de que Database::new lo reabra

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

        drop(conn);
        drop(db);
        // `dir` (TempDir) borra el directorio temporal solo al salir de scope.
    }
}
