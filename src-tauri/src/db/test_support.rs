//! Helpers compartidos por los `mod tests` de la capa SQLite.

use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

use crate::models::project::*;

use super::Database;

pub(super) fn test_db() -> Database {
    Database::new(PathBuf::from(":memory:")).expect("Failed to create in-memory database")
}

/// Siembra en `db_path` un esquema viejo real (pre-v0.4.0, `user_version = 0`) con un
/// proyecto y un enlace, y cierra la conexión antes de volver.
///
/// No tiene ninguna de las columnas que las `ALTER` agregan después (incluida
/// `ai_documentation_url`), ni `updated_at` en `project_links`, ni las tablas que
/// vinieron más tarde: es el camino de upgrade más largo que puede atravesar un
/// usuario real.
pub(super) fn sembrar_esquema_viejo(db_path: &Path) {
    let old_conn = Connection::open(db_path).expect("no se pudo crear el schema viejo");
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
} // old_conn se dropea acá, liberando el archivo antes de que lo reabra el llamador

/// `PRAGMA user_version` de una conexión.
pub(super) fn user_version(conn: &Connection) -> i64 {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap()
}

/// Nombres de columna de `tabla`, en orden.
pub(super) fn columnas(conn: &Connection, tabla: &str) -> Vec<String> {
    conn.prepare(&format!("PRAGMA table_info({})", tabla))
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
}

/// Nombres de todos los objetos del esquema (tablas, índices, triggers), ordenados:
/// sirve para afirmar que una migración fallida no dejó nada a medias.
pub(super) fn objetos_del_esquema(conn: &Connection) -> Vec<String> {
    conn.prepare("SELECT type || ' ' || name FROM sqlite_master ORDER BY type, name")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
}

pub(super) fn seed_project_at(db: &Database, name: &str, local_path: &str) -> i64 {
    db.create_project(CreateProjectDTO {
        name: name.to_string(),
        description: "desc".to_string(),
        local_path: local_path.to_string(),
        documentation_url: None,
        ai_documentation_url: None,
        drive_link: None,
        notes: None,
        image_data: None,
        parent_id: None,
        group_color: None,
        group_icon: None,
    })
    .expect("no se pudo sembrar el proyecto")
    .id
}

pub(super) fn seed_link(db: &Database, project_id: i64, title: &str) {
    db.create_link(CreateLinkDTO {
        project_id,
        link_type: "docs".to_string(),
        title: title.to_string(),
        url: format!("https://example.com/{}", title),
    })
    .expect("no se pudo sembrar el enlace");
}

pub(super) fn link_titles(project: &Project) -> Vec<String> {
    let mut titles: Vec<String> = project
        .links
        .as_ref()
        .expect("links nunca debe ser None en un listado")
        .iter()
        .map(|link| link.title.clone())
        .collect();
    titles.sort();
    titles
}

/// Siembra UNA fila en cada una de las 6 tablas hijas de `project_id`. Usado para
/// verificar que purge_project/empty_trash realmente vacían las 6, no solo las
/// que el test anterior (`test_purge_only_on_trashed_and_removes_for_real`) nunca
/// llegaba a poblar.
pub(super) fn seed_all_child_tables(db: &Database, project_id: i64) {
    db.create_link(CreateLinkDTO {
        project_id,
        link_type: "repo".to_string(),
        title: "Repo".to_string(),
        url: "https://example.com".to_string(),
    })
    .unwrap();
    db.add_attachment(CreateAttachmentDTO {
        project_id,
        filename: "a.txt".to_string(),
        file_data: "ZGF0YQ==".to_string(),
        file_size: 4,
        mime_type: "text/plain".to_string(),
    })
    .unwrap();
    db.create_journal_entry(CreateJournalEntryDTO {
        project_id,
        content: "entrada de prueba".to_string(),
        tags: None,
    })
    .unwrap();
    db.create_todo(CreateTodoDTO {
        project_id,
        content: "todo de prueba".to_string(),
    })
    .unwrap();
    db.track_project_open(project_id).unwrap(); // siembra project_activity
    db.create_tracking_session(project_id, "test").unwrap(); // siembra time_tracking_sessions
}

/// Cuenta filas de las 6 tablas hijas para `project_id`, en el mismo orden que
/// `seed_all_child_tables`: (links, attachments, journal, todos, activity, sessions).
pub(super) fn child_row_counts(db: &Database, project_id: i64) -> (i64, i64, i64, i64, i64, i64) {
    let conn = db.conn.lock().unwrap();
    let count = |table: &str| -> i64 {
        conn.query_row(
            &format!("SELECT COUNT(*) FROM {} WHERE project_id = ?1", table),
            params![project_id],
            |row| row.get(0),
        )
        .unwrap()
    };
    (
        count("project_links"),
        count("project_attachments"),
        count("project_journal"),
        count("project_todos"),
        count("project_activity"),
        count("time_tracking_sessions"),
    )
}

pub(super) fn test_project_dto() -> CreateProjectDTO {
    CreateProjectDTO {
        name: "Test Project".to_string(),
        description: "A test project".to_string(),
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
