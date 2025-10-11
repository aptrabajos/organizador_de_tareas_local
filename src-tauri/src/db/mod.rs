use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::models::{CreateProjectDTO, Project, UpdateProjectDTO};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let conn = Connection::open(db_path)?;

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

        // Migración: agregar columna ai_documentation_url si no existe
        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN ai_documentation_url TEXT",
            [],
        );

        Ok(Database {
            conn: Mutex::new(conn),
        })
    }

    pub fn create_project(&self, project: CreateProjectDTO) -> Result<Project> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO projects (name, description, local_path, documentation_url, ai_documentation_url, drive_link)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                project.name,
                project.description,
                project.local_path,
                project.documentation_url,
                project.ai_documentation_url,
                project.drive_link
            ],
        )?;

        let id = conn.last_insert_rowid();

        let project = conn.query_row(
            "SELECT id, name, description, local_path, documentation_url, ai_documentation_url, drive_link,
                    created_at, updated_at FROM projects WHERE id = ?1",
            params![id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    local_path: row.get(3)?,
                    documentation_url: row.get(4)?,
                    ai_documentation_url: row.get(5)?,
                    drive_link: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            },
        )?;

        Ok(project)
    }

    pub fn get_all_projects(&self) -> Result<Vec<Project>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, description, local_path, documentation_url, ai_documentation_url, drive_link,
                    created_at, updated_at FROM projects ORDER BY updated_at DESC"
        )?;

        let projects = stmt
            .query_map([], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    local_path: row.get(3)?,
                    documentation_url: row.get(4)?,
                    ai_documentation_url: row.get(5)?,
                    drive_link: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(projects)
    }

    pub fn get_project(&self, id: i64) -> Result<Project> {
        println!("🔍 [DB] get_project iniciado para ID: {}", id);
        
        // Intentar obtener la conexión con timeout
        let conn = match self.conn.try_lock() {
            Ok(conn) => conn,
            Err(_) => {
                println!("❌ [DB] No se pudo obtener lock de conexión - posible deadlock");
                return Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                    None
                ));
            }
        };

        println!("🔒 [DB] Conexión obtenida exitosamente");

        let result = conn.query_row(
            "SELECT id, name, description, local_path, documentation_url, ai_documentation_url, drive_link,
                    created_at, updated_at FROM projects WHERE id = ?1",
            params![id],
            |row| {
                println!("📊 [DB] Leyendo fila de base de datos...");
                let project = Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    local_path: row.get(3)?,
                    documentation_url: row.get(4)?,
                    ai_documentation_url: row.get(5)?,
                    drive_link: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                };
                println!("✅ [DB] Proyecto leído de BD: '{}'", project.name);
                Ok(project)
            },
        );

        match &result {
            Ok(proj) => {
                println!("✅ [DB] get_project exitoso: '{}'", proj.name);
            }
            Err(e) => {
                println!("❌ [DB] Error en get_project: {}", e);
            }
        }

        result
    }

    pub fn update_project(&self, id: i64, updates: UpdateProjectDTO) -> Result<Project> {
        println!("🗄️ [DB] Iniciando update_project en base de datos para ID: {}", id);
        
        // Intentar obtener la conexión con timeout
        let conn = match self.conn.try_lock() {
            Ok(conn) => conn,
            Err(_) => {
                println!("❌ [DB] No se pudo obtener lock de conexión en update_project - posible deadlock");
                return Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                    None
                ));
            }
        };

        println!("🔒 [DB] Conexión obtenida exitosamente para update_project");

        let mut query_parts = vec![];
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(name) = updates.name {
            query_parts.push("name = ?");
            params.push(Box::new(name));
        }
        if let Some(description) = updates.description {
            query_parts.push("description = ?");
            params.push(Box::new(description));
        }
        if let Some(local_path) = updates.local_path {
            query_parts.push("local_path = ?");
            params.push(Box::new(local_path));
        }
        if let Some(documentation_url) = updates.documentation_url {
            query_parts.push("documentation_url = ?");
            params.push(Box::new(documentation_url));
        }
        if let Some(ai_documentation_url) = updates.ai_documentation_url {
            query_parts.push("ai_documentation_url = ?");
            params.push(Box::new(ai_documentation_url));
        }
        if let Some(drive_link) = updates.drive_link {
            query_parts.push("drive_link = ?");
            params.push(Box::new(drive_link));
        }

        query_parts.push("updated_at = CURRENT_TIMESTAMP");
        params.push(Box::new(id));

        let query = format!(
            "UPDATE projects SET {} WHERE id = ?",
            query_parts.join(", ")
        );

        println!("🗄️ [DB] Query SQL: {}", query);
        println!("🗄️ [DB] Número de parámetros: {}", params.len());

        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        
        match conn.execute(&query, params_ref.as_slice()) {
            Ok(rows_affected) => {
                println!("🗄️ [DB] UPDATE ejecutado exitosamente, filas afectadas: {}", rows_affected);
            }
            Err(e) => {
                println!("🗄️ [DB] ERROR en UPDATE: {}", e);
                return Err(e);
            }
        }

        // Liberar la conexión antes de llamar a get_project
        println!("🔓 [DB] Liberando conexión después del UPDATE");
        drop(conn); // Liberar explícitamente la conexión
        
        println!("🔍 [DB] Obteniendo proyecto actualizado con ID: {}", id);
        let result = self.get_project(id);
        match &result {
            Ok(project) => {
                println!("✅ [DB] Proyecto obtenido exitosamente: '{}'", project.name);
            }
            Err(e) => {
                println!("❌ [DB] Error al obtener proyecto: {}", e);
            }
        }
        result
    }

    pub fn delete_project(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn search_projects(&self, query: &str) -> Result<Vec<Project>> {
        let conn = self.conn.lock().unwrap();
        let search_pattern = format!("%{}%", query);

        let mut stmt = conn.prepare(
            "SELECT id, name, description, local_path, documentation_url, ai_documentation_url, drive_link,
                    created_at, updated_at FROM projects
             WHERE name LIKE ?1 OR description LIKE ?1 OR local_path LIKE ?1
             ORDER BY updated_at DESC"
        )?;

        let projects = stmt
            .query_map(params![search_pattern], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    local_path: row.get(3)?,
                    documentation_url: row.get(4)?,
                    ai_documentation_url: row.get(5)?,
                    drive_link: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(projects)
    }
}
