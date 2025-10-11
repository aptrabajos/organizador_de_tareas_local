use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::models::project::{CreateProjectDTO, CreateLinkDTO, Project, ProjectLink, UpdateProjectDTO, UpdateLinkDTO};

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

        // Migraciones: agregar columnas si no existen
        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN ai_documentation_url TEXT",
            [],
        );

        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN notes TEXT",
            [],
        );

        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN image_data TEXT",
            [],
        );

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
                    links: None, // Los enlaces se cargan por separado
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

        let mut projects = Vec::new();
        let project_rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,  // id
                row.get::<_, String>(1)?,  // name
                row.get::<_, String>(2)?,  // description
                row.get::<_, String>(3)?,  // local_path
                row.get::<_, Option<String>>(4)?,  // documentation_url
                row.get::<_, Option<String>>(5)?,  // ai_documentation_url
                row.get::<_, Option<String>>(6)?,  // drive_link
                row.get::<_, String>(7)?,  // created_at
                row.get::<_, String>(8)?,  // updated_at
            ))
        })?
        .collect::<Result<Vec<_>>>()?;

        // Para cada proyecto, obtener sus enlaces
        for (id, name, description, local_path, documentation_url, ai_documentation_url, drive_link, created_at, updated_at) in project_rows {
            let links = self.get_project_links_internal(id, &conn).unwrap_or_else(|_| Vec::new());
            
            projects.push(Project {
                id,
                name,
                description,
                local_path,
                documentation_url,
                ai_documentation_url,
                drive_link,
                links: Some(links),
                created_at,
                updated_at,
            });
        }

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
                let project_id = row.get::<_, i64>(0)?;
                
                // Obtener enlaces del proyecto
                let links = self.get_project_links_internal(project_id, &conn).unwrap_or_else(|_| Vec::new());
                
                let project = Project {
                    id: project_id,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    local_path: row.get(3)?,
                    documentation_url: row.get(4)?,
                    ai_documentation_url: row.get(5)?,
                    drive_link: row.get(6)?,
                    links: Some(links),
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

        let mut projects = Vec::new();
        let project_rows = stmt.query_map(params![search_pattern], |row| {
            Ok((
                row.get::<_, i64>(0)?,  // id
                row.get::<_, String>(1)?,  // name
                row.get::<_, String>(2)?,  // description
                row.get::<_, String>(3)?,  // local_path
                row.get::<_, Option<String>>(4)?,  // documentation_url
                row.get::<_, Option<String>>(5)?,  // ai_documentation_url
                row.get::<_, Option<String>>(6)?,  // drive_link
                row.get::<_, String>(7)?,  // created_at
                row.get::<_, String>(8)?,  // updated_at
            ))
        })?
        .collect::<Result<Vec<_>>>()?;

        // Para cada proyecto, obtener sus enlaces
        for (id, name, description, local_path, documentation_url, ai_documentation_url, drive_link, created_at, updated_at) in project_rows {
            let links = self.get_project_links_internal(id, &conn).unwrap_or_else(|_| Vec::new());
            
            projects.push(Project {
                id,
                name,
                description,
                local_path,
                documentation_url,
                ai_documentation_url,
                drive_link,
                links: Some(links),
                created_at,
                updated_at,
            });
        }

        Ok(projects)
    }

    // Métodos para manejar enlaces de proyectos
    pub fn create_link(&self, link: CreateLinkDTO) -> Result<ProjectLink> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO project_links (project_id, link_type, title, url)
             VALUES (?1, ?2, ?3, ?4)",
            params![link.project_id, link.link_type, link.title, link.url],
        )?;

        let id = conn.last_insert_rowid();
        
        let mut stmt = conn.prepare(
            "SELECT id, project_id, link_type, title, url, created_at FROM project_links WHERE id = ?1"
        )?;
        
        let link = stmt.query_row(params![id], |row| {
            Ok(ProjectLink {
                id: row.get(0)?,
                project_id: row.get(1)?,
                link_type: row.get(2)?,
                title: row.get(3)?,
                url: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        Ok(link)
    }

    pub fn get_project_links(&self, project_id: i64) -> Result<Vec<ProjectLink>> {
        let conn = self.conn.lock().unwrap();
        self.get_project_links_internal(project_id, &conn)
    }

    fn get_project_links_internal(&self, project_id: i64, conn: &Connection) -> Result<Vec<ProjectLink>> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, link_type, title, url, created_at FROM project_links 
             WHERE project_id = ?1 ORDER BY created_at DESC"
        )?;

        let links = stmt
            .query_map(params![project_id], |row| {
                Ok(ProjectLink {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    link_type: row.get(2)?,
                    title: row.get(3)?,
                    url: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(links)
    }

    pub fn update_link(&self, id: i64, link: UpdateLinkDTO) -> Result<ProjectLink> {
        let conn = self.conn.lock().unwrap();

        // Construir la query dinámicamente basada en los campos proporcionados
        let mut set_clauses = Vec::new();
        let mut param_values: Vec<String> = Vec::new();

        if let Some(link_type) = link.link_type {
            set_clauses.push("link_type = ?");
            param_values.push(link_type);
        }

        if let Some(title) = link.title {
            set_clauses.push("title = ?");
            param_values.push(title);
        }

        if let Some(url) = link.url {
            set_clauses.push("url = ?");
            param_values.push(url);
        }

        if set_clauses.is_empty() {
            return Err(rusqlite::Error::InvalidParameterCount(0, 1));
        }

        set_clauses.push("updated_at = CURRENT_TIMESTAMP");

        let query = format!(
            "UPDATE project_links SET {} WHERE id = ?",
            set_clauses.join(", ")
        );

        // Crear parámetros con el ID al final
        let mut params: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        params.push(&id);

        conn.execute(&query, &*params)?;

        // Obtener el enlace actualizado
        let mut stmt = conn.prepare(
            "SELECT id, project_id, link_type, title, url, created_at FROM project_links WHERE id = ?1"
        )?;
        
        let updated_link = stmt.query_row(params![id], |row| {
            Ok(ProjectLink {
                id: row.get(0)?,
                project_id: row.get(1)?,
                link_type: row.get(2)?,
                title: row.get(3)?,
                url: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        Ok(updated_link)
    }

    pub fn delete_link(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM project_links WHERE id = ?1", params![id])?;
        Ok(())
    }
}
