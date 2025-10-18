use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::models::project::{CreateProjectDTO, CreateLinkDTO, Project, ProjectLink, UpdateProjectDTO, UpdateLinkDTO, ProjectAttachment, CreateAttachmentDTO, JournalEntry, CreateJournalEntryDTO, UpdateJournalEntryDTO, ProjectTodo, CreateTodoDTO, UpdateTodoDTO};

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

        Ok(Database {
            conn: Mutex::new(conn),
        })
    }

    pub fn create_project(&self, project: CreateProjectDTO) -> Result<Project> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO projects (name, description, local_path, documentation_url, ai_documentation_url, drive_link, notes, image_data)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                project.name,
                project.description,
                project.local_path,
                project.documentation_url,
                project.ai_documentation_url,
                project.drive_link,
                project.notes,
                project.image_data
            ],
        )?;

        let id = conn.last_insert_rowid();

        let project = conn.query_row(
            "SELECT id, name, description, local_path, documentation_url, ai_documentation_url, drive_link, notes, image_data,
                    created_at, updated_at, last_opened_at, opened_count, total_time_seconds,
                    status, status_changed_at, is_pinned, pinned_order FROM projects WHERE id = ?1",
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
                    notes: row.get(7)?,
                    image_data: row.get(8)?,
                    links: None, // Los enlaces se cargan por separado
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    last_opened_at: row.get(11)?,
                    opened_count: row.get(12)?,
                    total_time_seconds: row.get(13)?,
                    status: row.get(14)?,
                    status_changed_at: row.get(15)?,
                    is_pinned: row.get(16)?,
                    pinned_order: row.get(17)?,
                })
            },
        )?;

        Ok(project)
    }

    pub fn get_all_projects(&self) -> Result<Vec<Project>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, description, local_path, documentation_url, ai_documentation_url, drive_link, notes, image_data,
                    created_at, updated_at, last_opened_at, opened_count, total_time_seconds,
                    status, status_changed_at, is_pinned, pinned_order
             FROM projects
             ORDER BY is_pinned DESC, pinned_order ASC, updated_at DESC"
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
                row.get::<_, Option<String>>(7)?,  // notes
                row.get::<_, Option<String>>(8)?,  // image_data
                row.get::<_, String>(9)?,  // created_at
                row.get::<_, String>(10)?,  // updated_at
                row.get::<_, Option<String>>(11)?,  // last_opened_at
                row.get::<_, Option<i64>>(12)?,  // opened_count
                row.get::<_, Option<i64>>(13)?,  // total_time_seconds
                row.get::<_, Option<String>>(14)?,  // status
                row.get::<_, Option<String>>(15)?,  // status_changed_at
                row.get::<_, Option<bool>>(16)?,  // is_pinned
                row.get::<_, Option<i64>>(17)?,  // pinned_order
            ))
        })?
        .collect::<Result<Vec<_>>>()?;

        // Para cada proyecto, obtener sus enlaces
        for (id, name, description, local_path, documentation_url, ai_documentation_url, drive_link, notes, image_data, created_at, updated_at, last_opened_at, opened_count, total_time_seconds, status, status_changed_at, is_pinned, pinned_order) in project_rows {
            let links = self.get_project_links_internal(id, &conn).unwrap_or_else(|_| Vec::new());

            projects.push(Project {
                id,
                name,
                description,
                local_path,
                documentation_url,
                ai_documentation_url,
                drive_link,
                notes,
                image_data,
                links: Some(links),
                created_at,
                updated_at,
                last_opened_at,
                opened_count,
                total_time_seconds,
                status,
                status_changed_at,
                is_pinned,
                pinned_order,
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
            "SELECT id, name, description, local_path, documentation_url, ai_documentation_url, drive_link, notes, image_data,
                    created_at, updated_at, last_opened_at, opened_count, total_time_seconds,
                    status, status_changed_at, is_pinned, pinned_order FROM projects WHERE id = ?1",
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
                    notes: row.get(7)?,
                    image_data: row.get(8)?,
                    links: Some(links),
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    last_opened_at: row.get(11)?,
                    opened_count: row.get(12)?,
                    total_time_seconds: row.get(13)?,
                    status: row.get(14)?,
                    status_changed_at: row.get(15)?,
                    is_pinned: row.get(16)?,
                    pinned_order: row.get(17)?,
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
        if let Some(notes) = updates.notes {
            query_parts.push("notes = ?");
            params.push(Box::new(notes));
        }
        if let Some(image_data) = updates.image_data {
            query_parts.push("image_data = ?");
            params.push(Box::new(image_data));
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
            "SELECT id, name, description, local_path, documentation_url, ai_documentation_url, drive_link, notes, image_data,
                    created_at, updated_at, last_opened_at, opened_count, total_time_seconds,
                    status, status_changed_at, is_pinned, pinned_order FROM projects
             WHERE name LIKE ?1 OR description LIKE ?1 OR local_path LIKE ?1 OR notes LIKE ?1
             ORDER BY is_pinned DESC, pinned_order ASC, updated_at DESC"
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
                row.get::<_, Option<String>>(7)?,  // notes
                row.get::<_, Option<String>>(8)?,  // image_data
                row.get::<_, String>(9)?,  // created_at
                row.get::<_, String>(10)?,  // updated_at
                row.get::<_, Option<String>>(11)?,  // last_opened_at
                row.get::<_, Option<i64>>(12)?,  // opened_count
                row.get::<_, Option<i64>>(13)?,  // total_time_seconds
                row.get::<_, Option<String>>(14)?,  // status
                row.get::<_, Option<String>>(15)?,  // status_changed_at
                row.get::<_, Option<bool>>(16)?,  // is_pinned
                row.get::<_, Option<i64>>(17)?,  // pinned_order
            ))
        })?
        .collect::<Result<Vec<_>>>()?;

        // Para cada proyecto, obtener sus enlaces
        for (id, name, description, local_path, documentation_url, ai_documentation_url, drive_link, notes, image_data, created_at, updated_at, last_opened_at, opened_count, total_time_seconds, status, status_changed_at, is_pinned, pinned_order) in project_rows {
            let links = self.get_project_links_internal(id, &conn).unwrap_or_else(|_| Vec::new());

            projects.push(Project {
                id,
                name,
                description,
                local_path,
                documentation_url,
                ai_documentation_url,
                drive_link,
                notes,
                image_data,
                links: Some(links),
                created_at,
                updated_at,
                last_opened_at,
                opened_count,
                total_time_seconds,
                status,
                status_changed_at,
                is_pinned,
                pinned_order,
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

    // Métodos para tracking y analytics
    pub fn track_project_open(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE projects
             SET last_opened_at = CURRENT_TIMESTAMP,
                 opened_count = COALESCE(opened_count, 0) + 1
             WHERE id = ?1",
            params![id],
        )?;

        // Crear registro de actividad
        conn.execute(
            "INSERT INTO project_activity (project_id, activity_type, description)
             VALUES (?1, 'opened', 'Proyecto abierto')",
            params![id],
        )?;

        Ok(())
    }

    pub fn add_project_time(&self, id: i64, seconds: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE projects
             SET total_time_seconds = COALESCE(total_time_seconds, 0) + ?2
             WHERE id = ?1",
            params![id, seconds],
        )?;

        Ok(())
    }

    pub fn get_project_stats(&self) -> Result<crate::models::project::ProjectStats> {
        use crate::models::project::{ProjectStats, ProjectActivity};

        let conn = self.conn.lock().unwrap();

        // Total de proyectos
        let total_projects: i64 = conn.query_row(
            "SELECT COUNT(*) FROM projects",
            [],
            |row| row.get(0),
        )?;

        // Proyectos activos hoy
        let active_today: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT project_id) FROM project_activity
             WHERE DATE(created_at) = DATE('now')",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        // Tiempo total en horas
        let total_seconds: i64 = conn.query_row(
            "SELECT COALESCE(SUM(total_time_seconds), 0) FROM projects",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        let total_time_hours = total_seconds as f64 / 3600.0;

        // Proyecto más activo
        let most_active_project: Option<String> = conn.query_row(
            "SELECT name FROM projects
             WHERE opened_count = (SELECT MAX(opened_count) FROM projects)
             LIMIT 1",
            [],
            |row| row.get(0),
        ).ok();

        // Actividades recientes (últimas 20)
        let mut stmt = conn.prepare(
            "SELECT id, project_id, activity_type, description, duration_seconds, created_at
             FROM project_activity
             ORDER BY created_at DESC
             LIMIT 20"
        )?;

        let activities = stmt.query_map([], |row| {
            Ok(ProjectActivity {
                id: row.get(0)?,
                project_id: row.get(1)?,
                activity_type: row.get(2)?,
                description: row.get(3)?,
                duration_seconds: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

        Ok(ProjectStats {
            total_projects,
            active_today,
            total_time_hours,
            most_active_project,
            recent_activities: activities,
        })
    }

    pub fn get_project_activities(&self, project_id: i64, limit: i64) -> Result<Vec<crate::models::project::ProjectActivity>> {
        use crate::models::project::ProjectActivity;

        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, project_id, activity_type, description, duration_seconds, created_at
             FROM project_activity
             WHERE project_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2"
        )?;

        let activities = stmt.query_map(params![project_id, limit], |row| {
            Ok(ProjectActivity {
                id: row.get(0)?,
                project_id: row.get(1)?,
                activity_type: row.get(2)?,
                description: row.get(3)?,
                duration_seconds: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

        Ok(activities)
    }

    // ==================== MÉTODOS PARA ARCHIVOS ADJUNTOS ====================

    pub fn add_attachment(&self, attachment: CreateAttachmentDTO) -> Result<ProjectAttachment> {
        const MAX_FILE_SIZE: i64 = 5 * 1024 * 1024; // 5MB en bytes

        // Validar tamaño
        if attachment.file_size > MAX_FILE_SIZE {
            return Err(rusqlite::Error::InvalidParameterName(
                "File size exceeds 5MB limit".to_string(),
            ));
        }

        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO project_attachments (project_id, filename, file_data, file_size, mime_type)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                attachment.project_id,
                attachment.filename,
                attachment.file_data,
                attachment.file_size,
                attachment.mime_type
            ],
        )?;

        let id = conn.last_insert_rowid();

        let attachment = conn.query_row(
            "SELECT id, project_id, filename, file_data, file_size, mime_type, created_at
             FROM project_attachments WHERE id = ?1",
            params![id],
            |row| {
                Ok(ProjectAttachment {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    filename: row.get(2)?,
                    file_data: row.get(3)?,
                    file_size: row.get(4)?,
                    mime_type: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        )?;

        Ok(attachment)
    }

    pub fn get_attachments(&self, project_id: i64) -> Result<Vec<ProjectAttachment>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, project_id, filename, file_data, file_size, mime_type, created_at
             FROM project_attachments WHERE project_id = ?1 ORDER BY created_at DESC",
        )?;

        let attachments = stmt
            .query_map(params![project_id], |row| {
                Ok(ProjectAttachment {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    filename: row.get(2)?,
                    file_data: row.get(3)?,
                    file_size: row.get(4)?,
                    mime_type: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(attachments)
    }

    pub fn delete_attachment(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute("DELETE FROM project_attachments WHERE id = ?1", params![id])?;

        Ok(())
    }

    // ==================== MÉTODOS PARA PROJECT JOURNAL ====================

    pub fn create_journal_entry(&self, entry: CreateJournalEntryDTO) -> Result<JournalEntry> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO project_journal (project_id, content, tags)
             VALUES (?1, ?2, ?3)",
            params![entry.project_id, entry.content, entry.tags],
        )?;

        let id = conn.last_insert_rowid();

        let journal_entry = conn.query_row(
            "SELECT id, project_id, content, tags, created_at, updated_at
             FROM project_journal WHERE id = ?1",
            params![id],
            |row| {
                Ok(JournalEntry {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    content: row.get(2)?,
                    tags: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )?;

        Ok(journal_entry)
    }

    pub fn get_journal_entries(&self, project_id: i64) -> Result<Vec<JournalEntry>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, project_id, content, tags, created_at, updated_at
             FROM project_journal
             WHERE project_id = ?1
             ORDER BY created_at DESC",
        )?;

        let entries = stmt
            .query_map(params![project_id], |row| {
                Ok(JournalEntry {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    content: row.get(2)?,
                    tags: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(entries)
    }

    pub fn update_journal_entry(&self, id: i64, updates: UpdateJournalEntryDTO) -> Result<JournalEntry> {
        let conn = self.conn.lock().unwrap();

        let mut set_clauses = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(content) = updates.content {
            set_clauses.push("content = ?");
            params.push(Box::new(content));
        }

        if let Some(tags) = updates.tags {
            set_clauses.push("tags = ?");
            params.push(Box::new(tags));
        }

        if set_clauses.is_empty() {
            return Err(rusqlite::Error::InvalidParameterCount(0, 1));
        }

        set_clauses.push("updated_at = CURRENT_TIMESTAMP");
        params.push(Box::new(id));

        let query = format!(
            "UPDATE project_journal SET {} WHERE id = ?",
            set_clauses.join(", ")
        );

        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&query, params_ref.as_slice())?;

        // Obtener la entrada actualizada
        let entry = conn.query_row(
            "SELECT id, project_id, content, tags, created_at, updated_at
             FROM project_journal WHERE id = ?1",
            params![id],
            |row| {
                Ok(JournalEntry {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    content: row.get(2)?,
                    tags: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )?;

        Ok(entry)
    }

    pub fn delete_journal_entry(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute("DELETE FROM project_journal WHERE id = ?1", params![id])?;

        Ok(())
    }
}
