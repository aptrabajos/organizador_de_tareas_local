use log::{debug, error};
use rusqlite::{params, Connection, Result, Row};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::models::project::{CreateProjectDTO, CreateLinkDTO, Project, ProjectLink, UpdateProjectDTO, UpdateLinkDTO, ProjectAttachment, CreateAttachmentDTO, JournalEntry, CreateJournalEntryDTO, UpdateJournalEntryDTO, ProjectTodo, CreateTodoDTO, UpdateTodoDTO, ProjectWithChildren};

/// Item de la papelera. Struct dedicado (no toca el modelo Project) que SÍ lleva la
/// fecha de borrado real y el conteo de subproyectos también en papelera.
#[derive(serde::Serialize)]
pub struct TrashItem {
    pub id: i64,
    pub name: String,
    pub deleted_at: String,
    pub subproject_count: i64,
}

/// Las 23 columnas de `projects`, EN ORDEN.
///
/// Esta lista aparecía textualmente en siete métodos (`create_project`,
/// `get_all_projects`, `get_project`, `search_projects`, `get_recent_projects`,
/// `get_root_projects`, `get_subprojects`). Agregar una columna obligaba a tocar
/// los siete en sincronía, y olvidarse de uno no rompe la compilación: rompe en
/// runtime, en el índice posicional de [`row_to_project`], y solo en el camino
/// que se olvidó.
///
/// El orden ES el contrato: [`row_to_project`] lee por posición.
const PROJECT_COLUMNS: &str = "id, name, description, local_path, documentation_url, ai_documentation_url, drive_link, notes, image_data,
                    created_at, updated_at, last_opened_at, opened_count, total_time_seconds,
                    status, status_changed_at, is_pinned, pinned_order, display_order,
                    parent_id, group_color, group_icon, is_group_expanded";

/// Construye un `Project` desde una fila que trae [`PROJECT_COLUMNS`] en ese orden.
///
/// `links` queda en `None` a propósito: los enlaces viven en otra tabla y quien
/// los necesite los completa después con [`Database::get_links_for_projects`].
/// Mezclarlos acá era justamente lo que producía el N+1.
fn row_to_project(row: &Row) -> Result<Project> {
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
        links: None,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        last_opened_at: row.get(11)?,
        opened_count: row.get(12)?,
        total_time_seconds: row.get(13)?,
        status: row.get(14)?,
        status_changed_at: row.get(15)?,
        is_pinned: row.get(16)?,
        pinned_order: row.get(17)?,
        display_order: row.get(18)?,
        parent_id: row.get(19)?,
        group_color: row.get(20)?,
        group_icon: row.get(21)?,
        is_group_expanded: row.get(22)?,
    })
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Toma el lock de la conexión, recuperándose del envenenamiento.
    ///
    /// Había 45 `self.conn()` en el código de producción de este
    /// archivo, y el problema no era el `unwrap()` en abstracto: `Mutex::lock()`
    /// devuelve `Err` SOLO si el mutex quedó envenenado, o sea si otro hilo entró
    /// en panic mientras lo tenía tomado. Con `unwrap()`, ese primer panic
    /// convertía en panic TODAS las operaciones de base posteriores: un fallo
    /// aislado en un camino cualquiera dejaba la app muerta hasta el reinicio.
    ///
    /// La política es recuperar, no propagar. El envenenamiento es una marca que
    /// pone Rust, no una corrupción de SQLite: la conexión sigue siendo
    /// perfectamente válida, así que `into_inner()` devuelve el guard y la app
    /// sigue viva. Propagar un error tipado sería más "honesto" pero convertiría
    /// el panic ajeno en un error visible al usuario en cada operación siguiente,
    /// que es el mismo desastre con mejores modales.
    ///
    /// El `error!` no es opcional: si esto dispara, hubo un panic en algún lado y
    /// alguien tiene que poder encontrarlo.
    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|poisoned| {
            error!(
                "❌ [DB] El mutex de la conexión estaba envenenado (hubo un panic con el lock tomado). Se recupera y se sigue operando."
            );
            poisoned.into_inner()
        })
    }

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

        // Agregar campo para orden personalizado de las cards
        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN display_order INTEGER DEFAULT 0",
            [],
        );

        // Agregar campos para sistema de grupos de proyectos (v0.4.0)
        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN parent_id INTEGER",
            [],
        );

        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN group_color TEXT",
            [],
        );

        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN group_icon TEXT",
            [],
        );

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

    /// Crea un backup transaccionalmente consistente de la base de datos.
    ///
    /// Usa `VACUUM INTO` a través del mismo Mutex que serializa las escrituras,
    /// por lo que la copia queda en un punto consistente sin escrituras a medias.
    /// `dest` debe ser una ruta de archivo que NO exista: `VACUUM INTO` falla si
    /// el archivo destino ya existe.
    pub fn backup_to(&self, dest: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute("VACUUM INTO ?1", params![dest])?;
        Ok(())
    }

    /// Verifica una COPIA de la base de datos (nunca la DB viva).
    ///
    /// Abre el archivo en modo READ_ONLY y corre `PRAGMA integrity_check` más un
    /// conteo de proyectos. Devuelve `(integridad_ok, cantidad_de_proyectos)`.
    pub fn verify_db_file(path: &str) -> Result<(bool, i64)> {
        use rusqlite::OpenFlags;
        let c = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let res: String = c.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        let count: i64 = c.query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))?;
        Ok((res == "ok", count))
    }

    /// Normaliza un texto opcional para columnas nullable: vacío (tras trim) -> NULL.
    /// Mantiene la DB consistente (NULL en vez de cadena vacía) y permite vaciar campos.
    fn empty_to_null(s: String) -> Option<String> {
        if s.trim().is_empty() {
            None
        } else {
            Some(s)
        }
    }

    pub fn create_project(&self, project: CreateProjectDTO) -> Result<Project> {
        let conn = self.conn();

        // Validar parent_id ANTES del INSERT (misma regla que assign_project_to_group,
        // la única barrera hoy validada contra grupos padre inexistentes/borrados): sin
        // esto, create_project podía insertar un parent_id que no existe o que está en
        // papelera, dejando el proyecto nuevo como huérfano invisible en la jerarquía.
        // No hace falta chequeo anti-ciclos acá: el proyecto todavía no tiene id propio.
        if let Some(parent_id) = project.parent_id {
            let exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM projects WHERE id = ?1 AND deleted_at IS NULL",
                params![parent_id],
                |row| row.get(0),
            )?;
            if exists == 0 {
                return Err(rusqlite::Error::InvalidParameterName(
                    "El grupo padre seleccionado no existe o está en la papelera.".to_string(),
                ));
            }
        }

        conn.execute(
            "INSERT INTO projects (name, description, local_path, documentation_url, ai_documentation_url, drive_link, notes, image_data, parent_id, group_color, group_icon)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                project.name,
                project.description,
                project.local_path,
                project.documentation_url.and_then(Self::empty_to_null),
                project.ai_documentation_url.and_then(Self::empty_to_null),
                project.drive_link.and_then(Self::empty_to_null),
                project.notes.and_then(Self::empty_to_null),
                project.image_data.and_then(Self::empty_to_null),
                project.parent_id,
                project.group_color.and_then(Self::empty_to_null),
                project.group_icon.and_then(Self::empty_to_null)
            ],
        )?;

        let id = conn.last_insert_rowid();

        let project = conn.query_row(
            &format!("SELECT {}
             FROM projects WHERE id = ?1", PROJECT_COLUMNS),
            params![id],
            row_to_project,
        )?;

        Ok(project)
    }

    pub fn get_all_projects(&self) -> Result<Vec<Project>> {
        let conn = self.conn();

        let mut stmt = conn.prepare(
            &format!("SELECT {}
             FROM projects
             WHERE deleted_at IS NULL
             ORDER BY display_order ASC, is_pinned DESC, pinned_order ASC, updated_at DESC", PROJECT_COLUMNS)
        )?;

        let projects_without_links = stmt
            .query_map([], row_to_project)?
            .collect::<Result<Vec<_>>>()?;

        // Los enlaces de TODA la lista en una sola consulta. Antes era una
        // consulta por proyecto, y encima con `unwrap_or_else(|_| Vec::new())`:
        // un fallo al cargar enlaces se tragaba en silencio y el proyecto
        // aparecía sin enlaces sin que nadie se enterara. Ahora el error sube.
        let ids: Vec<i64> = projects_without_links.iter().map(|p| p.id).collect();
        let mut links_by_project = self.get_links_for_projects(&ids, &conn)?;

        let projects: Vec<Project> = projects_without_links
            .into_iter()
            .map(|mut project| {
                project.links =
                    Some(links_by_project.remove(&project.id).unwrap_or_default());
                project
            })
            .collect();

        Ok(projects)
    }

    /// Leer UN proyecto activo por id.
    ///
    /// `AND deleted_at IS NULL`: era la única lectura del archivo que NO filtraba la
    /// papelera (`get_all_projects`, `search_projects`, `get_root_projects` y
    /// `get_subprojects` sí lo hacen), así que un proyecto borrado seguía apareciendo
    /// en la cabecera de grupo y en el modal de contexto vía `refreshCurrentGroup()`.
    /// `get_project_with_children` compone esta función y heredaba el defecto.
    ///
    /// Un proyecto en papelera devuelve `QueryReturnedNoRows`. NO hay variante
    /// `_including_trashed` porque nadie la necesita: `restore_project` y
    /// `purge_project` hacen su propio `SELECT deleted_at ... WHERE id = ?1` y exigen
    /// que el proyecto SÍ esté en papelera.
    pub fn get_project(&self, id: i64) -> Result<Project> {
        // Intentar obtener la conexión con timeout.
        // NO tocar este try_lock: es la posible cicatriz de un deadlock por reentrancia
        // del Mutex y nadie documentó el motivo original (§6.6 del plan de auditoría).
        let conn = match self.conn.try_lock() {
            Ok(conn) => conn,
            Err(_) => {
                // Este log SÍ se queda, y en nivel `error`: sólo dispara en el camino de
                // deadlock, no en cada invocación. Es diagnóstico real, no ruido, y es
                // justo lo que querés seguir viendo aunque el nivel esté en `error`.
                error!("❌ [DB] No se pudo obtener lock de conexión - posible deadlock");
                return Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                    None
                ));
            }
        };

        let mut project = conn.query_row(
            &format!("SELECT {}
             FROM projects WHERE id = ?1 AND deleted_at IS NULL", PROJECT_COLUMNS),
            params![id],
            row_to_project,
        )?;

        // Los enlaces se cargan DESPUÉS de cerrar la fila, no adentro del closure de
        // `query_row`. Es un solo proyecto, así que acá nunca hubo N+1; lo que había
        // era `unwrap_or_else(|_| Vec::new())`, que se tragaba en silencio un fallo
        // al cargar enlaces y devolvía el proyecto como si no tuviera ninguno. Ahora
        // el error sube.
        project.links = Some(self.get_project_links_internal(id, &conn)?);

        Ok(project)
    }

    pub fn update_project(&self, id: i64, updates: UpdateProjectDTO) -> Result<Project> {
        debug!("🗄️ [DB] Iniciando update_project en base de datos para ID: {}", id);
        
        // Intentar obtener la conexión con timeout
        let conn = match self.conn.try_lock() {
            Ok(conn) => conn,
            Err(_) => {
                error!("❌ [DB] No se pudo obtener lock de conexión en update_project - posible deadlock");
                return Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                    None
                ));
            }
        };

        debug!("🔒 [DB] Conexión obtenida exitosamente para update_project");

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
        // Columnas nullable: vacío -> NULL (permite vaciar el campo y mantiene la DB consistente)
        if let Some(documentation_url) = updates.documentation_url {
            query_parts.push("documentation_url = ?");
            params.push(Box::new(Self::empty_to_null(documentation_url)));
        }
        if let Some(ai_documentation_url) = updates.ai_documentation_url {
            query_parts.push("ai_documentation_url = ?");
            params.push(Box::new(Self::empty_to_null(ai_documentation_url)));
        }
        if let Some(drive_link) = updates.drive_link {
            query_parts.push("drive_link = ?");
            params.push(Box::new(Self::empty_to_null(drive_link)));
        }
        if let Some(notes) = updates.notes {
            query_parts.push("notes = ?");
            params.push(Box::new(Self::empty_to_null(notes)));
        }
        if let Some(image_data) = updates.image_data {
            query_parts.push("image_data = ?");
            params.push(Box::new(Self::empty_to_null(image_data)));
        }
        // Group fields (v0.4.0). parent_id NO se toca aquí a propósito: TODO cambio de grupo
        // pasa por assign_project_to_group, la ÚNICA barrera validada contra ciclos. Si
        // updates.parent_id llega con valor, se ignora (ruta cerrada).
        if let Some(group_color) = updates.group_color {
            query_parts.push("group_color = ?");
            params.push(Box::new(Self::empty_to_null(group_color)));
        }
        if let Some(group_icon) = updates.group_icon {
            query_parts.push("group_icon = ?");
            params.push(Box::new(Self::empty_to_null(group_icon)));
        }

        query_parts.push("updated_at = CURRENT_TIMESTAMP");
        params.push(Box::new(id));

        let query = format!(
            "UPDATE projects SET {} WHERE id = ?",
            query_parts.join(", ")
        );

        // NO se loguea la query ni el conteo de parámetros. Acá había dos `println!` que
        // volcaban el SQL completo del UPDATE y la cantidad de valores en CADA
        // actualización. Se BORRARON en vez de bajarlos a `debug!`: una query con datos
        // del usuario no debería poder aparecer en un log, ni siquiera en modo debug.
        // Para diagnosticar alcanza con el id afectado y las filas modificadas.

        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        
        match conn.execute(&query, params_ref.as_slice()) {
            Ok(rows_affected) => {
                debug!("🗄️ [DB] UPDATE ejecutado exitosamente, filas afectadas: {}", rows_affected);
            }
            Err(e) => {
                debug!("🗄️ [DB] ERROR en UPDATE: {}", e);
                return Err(e);
            }
        }

        // Liberar la conexión antes de llamar a get_project
        debug!("🔓 [DB] Liberando conexión después del UPDATE");
        drop(conn); // Liberar explícitamente la conexión
        
        debug!("🔍 [DB] Obteniendo proyecto actualizado con ID: {}", id);
        let result = self.get_project(id);
        match &result {
            Ok(project) => {
                debug!("✅ [DB] Proyecto obtenido exitosamente: '{}'", project.name);
            }
            Err(e) => {
                error!("❌ [DB] Error al obtener proyecto: {}", e);
            }
        }
        result
    }

    /// Soft-delete: mueve el proyecto a la papelera marcando deleted_at.
    /// Cascada: marca también los hijos directos AÚN activos con el MISMO
    /// timestamp (eso identifica el batch para restaurarlo en bloque).
    pub fn delete_project(&self, id: i64) -> Result<()> {
        let conn = self.conn();
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let tx = conn.unchecked_transaction()?;
        // Soft-delete del proyecto + TODOS sus descendientes activos (cualquier
        // profundidad, no solo hijos directos) con el MISMO timestamp: así no quedan
        // nietos activos colgando de un padre borrado (invisibles), e identifica el
        // batch para restaurar/purgar en bloque. `AND deleted_at IS NULL` preserva
        // los que ya estaban en papelera de antes.
        tx.execute(
            "UPDATE projects SET deleted_at = ?1
             WHERE deleted_at IS NULL AND id IN (
                 WITH RECURSIVE descendants(did) AS (
                     SELECT ?2
                     UNION ALL
                     SELECT p.id FROM projects p JOIN descendants ON p.parent_id = descendants.did
                 )
                 SELECT did FROM descendants
             )",
            params![now, id],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Restaurar un proyecto de la papelera (deleted_at -> NULL).
    /// - Si tiene padre y el padre sigue en papelera, lo re-parentea a raíz para
    ///   no dejarlo huérfano colgado de un grupo borrado.
    /// - Si el id ES un grupo, restaura también los hijos del MISMO batch (mismo
    ///   timestamp de borrado), preservando la jerarquía original.
    pub fn restore_project(&self, id: i64) -> Result<()> {
        let conn = self.conn();

        // Validar que el proyecto exista Y esté efectivamente en la papelera ANTES de
        // cualquier UPDATE (mismo guard que ya tiene purge_project). Sin esto, para un
        // id inexistente o ya activo el UPDATE de abajo afecta 0 filas, no hay error, y
        // la función devolvía Ok(()) como si hubiera restaurado algo.
        let own_deleted_at: Option<String> = conn
            .query_row(
                "SELECT deleted_at FROM projects WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok()
            .flatten();
        if own_deleted_at.is_none() {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        let tx = conn.unchecked_transaction()?;

        // ¿El padre está disponible (existe Y activo)? Si NO —porque sigue en papelera
        // O porque fue purgado (ya no existe)— re-parenteamos a raíz para no dejar el
        // proyecto colgando de un padre inexistente/borrado (huérfano invisible).
        let parent_id: Option<i64> = tx
            .query_row(
                "SELECT parent_id FROM projects WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        let needs_reparent = match parent_id {
            Some(pid) => {
                let parent_active: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND deleted_at IS NULL)",
                    params![pid],
                    |row| row.get(0),
                )?;
                !parent_active
            }
            None => false,
        };

        if needs_reparent {
            tx.execute(
                "UPDATE projects SET deleted_at = NULL, parent_id = NULL WHERE id = ?1",
                params![id],
            )?;
        } else {
            tx.execute(
                "UPDATE projects SET deleted_at = NULL WHERE id = ?1",
                params![id],
            )?;
        }

        // Restaurar TODO el subárbol del MISMO batch (mismo timestamp de borrado),
        // a cualquier profundidad. El propio id ya quedó en NULL arriba.
        if let Some(deleted_at) = own_deleted_at {
            tx.execute(
                "UPDATE projects SET deleted_at = NULL
                 WHERE deleted_at = ?2 AND id IN (
                     WITH RECURSIVE descendants(did) AS (
                         SELECT ?1
                         UNION ALL
                         SELECT p.id FROM projects p JOIN descendants ON p.parent_id = descendants.did
                     )
                     SELECT did FROM descendants
                 )",
                params![id, deleted_at],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Eliminar DEFINITIVAMENTE un proyecto (solo si está en papelera).
    /// Borra en cascada de todas las tablas hijas dentro de una transacción.
    pub fn purge_project(&self, id: i64) -> Result<()> {
        let conn = self.conn();

        // Validar que el proyecto esté efectivamente en la papelera
        let deleted_at: Option<String> = conn
            .query_row(
                "SELECT deleted_at FROM projects WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok()
            .flatten();
        if deleted_at.is_none() {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        let tx = conn.unchecked_transaction()?;
        // Purgar el grupo + sus descendientes QUE TAMBIÉN ESTÉN EN PAPELERA (no tocar
        // activos): si no, sus subproyectos quedarían como filas zombie apuntando a un
        // padre inexistente. La recursión solo desciende por ramas borradas.
        let ids: Vec<i64> = {
            let mut stmt = tx.prepare(
                "WITH RECURSIVE descendants(did) AS (
                     SELECT ?1
                     UNION ALL
                     SELECT p.id FROM projects p JOIN descendants ON p.parent_id = descendants.did
                     WHERE p.deleted_at IS NOT NULL
                 )
                 SELECT did FROM descendants",
            )?;
            let rows = stmt.query_map(params![id], |row| row.get::<_, i64>(0))?;
            rows.collect::<Result<Vec<_>>>()?
        };
        for pid in ids {
            Self::purge_project_internal(&tx, pid)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Borrado físico de un proyecto y todos sus datos. Recibe la transacción
    /// YA abierta; NO valida el estado de papelera (eso es del caller).
    fn purge_project_internal(tx: &Connection, id: i64) -> Result<()> {
        tx.execute("DELETE FROM project_links WHERE project_id = ?1", params![id])?;
        tx.execute("DELETE FROM project_attachments WHERE project_id = ?1", params![id])?;
        tx.execute("DELETE FROM project_journal WHERE project_id = ?1", params![id])?;
        tx.execute("DELETE FROM project_todos WHERE project_id = ?1", params![id])?;
        tx.execute("DELETE FROM project_activity WHERE project_id = ?1", params![id])?;
        tx.execute("DELETE FROM time_tracking_sessions WHERE project_id = ?1", params![id])?;
        tx.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Vaciar la papelera: elimina DEFINITIVAMENTE todos los proyectos con
    /// deleted_at IS NOT NULL y sus datos, en una sola transacción.
    pub fn empty_trash(&self) -> Result<()> {
        let conn = self.conn();

        let ids: Vec<i64> = {
            let mut stmt = conn.prepare(
                "SELECT id FROM projects WHERE deleted_at IS NOT NULL",
            )?;
            let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
            rows.collect::<Result<Vec<_>>>()?
        };

        let tx = conn.unchecked_transaction()?;
        for id in ids {
            Self::purge_project_internal(&tx, id)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Listar los proyectos en la papelera (deleted_at IS NOT NULL).
    /// Mismo mapeo posicional que get_all_projects.
    pub fn list_trash(&self) -> Result<Vec<TrashItem>> {
        let conn = self.conn();

        // Devolvemos la fecha de borrado REAL (deleted_at) y cuántos subproyectos
        // cayeron también a la papelera, sin tocar el struct Project ni su SELECT.
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.deleted_at,
                    (SELECT COUNT(*) FROM projects c
                     WHERE c.parent_id = t.id AND c.deleted_at IS NOT NULL) AS subproject_count
             FROM projects t
             WHERE t.deleted_at IS NOT NULL
             ORDER BY t.deleted_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(TrashItem {
                id: row.get(0)?,
                name: row.get(1)?,
                deleted_at: row.get(2)?,
                subproject_count: row.get(3)?,
            })
        })?;

        rows.collect()
    }

    /// ¿Hay un proyecto ACTIVO cuyo `local_path` sea exactamente `path`?
    ///
    /// Es la fuente de verdad de AUTORIZACIÓN para los comandos git: la app sólo puede
    /// operar git sobre carpetas de proyectos que ella misma gestiona. El riesgo que
    /// cubre no es inyección de comandos (los argumentos van como argv real, nunca a
    /// un shell), sino que la app corra git contra un repositorio arbitrario del disco.
    ///
    /// El filtro `deleted_at IS NULL` es DELIBERADO y explícito acá: mandar un proyecto
    /// a la papelera tiene que REVOCAR el permiso git sobre su carpeta. `get_project`
    /// ya aplica el mismo criterio (B4), pero esto NO es redundancia: este método no
    /// compone `get_project` —consulta por `local_path`, no por id— así que necesita su
    /// propio filtro. Los dos tienen que decir lo mismo y cada uno lo asserta aparte.
    pub fn is_registered_project_path(&self, path: &str) -> Result<bool> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT 1 FROM projects WHERE local_path = ?1 AND deleted_at IS NULL LIMIT 1",
        )?;
        stmt.exists(params![path])
    }

    /// `local_path` de todos los proyectos ACTIVOS (mismo criterio que
    /// `is_registered_project_path`, en bloque).
    ///
    /// La usa el guard de git para comparar en forma CANÓNICA: el match textual exacto
    /// no alcanza si la ruta guardada y la recibida difieren en forma (barra final,
    /// './' intermedio, symlink) apuntando al mismo directorio real.
    pub fn active_project_paths(&self) -> Result<Vec<String>> {
        let conn = self.conn();
        let mut stmt =
            conn.prepare("SELECT local_path FROM projects WHERE deleted_at IS NULL")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect()
    }

    pub fn search_projects(&self, query: &str) -> Result<Vec<Project>> {
        let conn = self.conn();
        let search_pattern = format!("%{}%", query);

        let mut stmt = conn.prepare(
            &format!("SELECT {}
             FROM projects
             WHERE (name LIKE ?1 OR description LIKE ?1 OR local_path LIKE ?1 OR notes LIKE ?1) AND deleted_at IS NULL
             ORDER BY display_order ASC, is_pinned DESC, pinned_order ASC, updated_at DESC", PROJECT_COLUMNS)
        )?;

        let projects_without_links = stmt
            .query_map(params![search_pattern], row_to_project)?
            .collect::<Result<Vec<_>>>()?;

        // Los enlaces de TODA la lista en una sola consulta. Antes era una
        // consulta por proyecto, y encima con `unwrap_or_else(|_| Vec::new())`:
        // un fallo al cargar enlaces se tragaba en silencio y el proyecto
        // aparecía sin enlaces sin que nadie se enterara. Ahora el error sube.
        let ids: Vec<i64> = projects_without_links.iter().map(|p| p.id).collect();
        let mut links_by_project = self.get_links_for_projects(&ids, &conn)?;

        let projects: Vec<Project> = projects_without_links
            .into_iter()
            .map(|mut project| {
                project.links =
                    Some(links_by_project.remove(&project.id).unwrap_or_default());
                project
            })
            .collect();

        Ok(projects)
    }

    // Métodos para manejar enlaces de proyectos
    pub fn create_link(&self, link: CreateLinkDTO) -> Result<ProjectLink> {
        let conn = self.conn();

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
        let conn = self.conn();
        self.get_project_links_internal(project_id, &conn)
    }

    /// Enlaces de varios proyectos en UNA sola consulta.
    ///
    /// Reemplaza al N+1 que tenían `get_all_projects`, `search_projects`,
    /// `get_root_projects` y `get_subprojects`: los cuatro llamaban a
    /// `get_project_links_internal` una vez POR PROYECTO de la lista. Con la base
    /// local y el Mutex ya tomado el costo por consulta es bajo y con decenas de
    /// proyectos no se nota, pero con cientos sí.
    ///
    /// Un proyecto sin enlaces NO aparece en el mapa; el consumidor usa
    /// `unwrap_or_default()` y obtiene la lista vacía, que es lo que devolvía el
    /// camino viejo.
    ///
    /// El batcheo no es adorno: SQLite tiene un tope de parámetros por statement
    /// (`SQLITE_MAX_VARIABLE_NUMBER`, 32766 en versiones recientes) y pasarlo hace
    /// fallar la query entera. Se usa un tamaño conservador.
    fn get_links_for_projects(
        &self,
        ids: &[i64],
        conn: &Connection,
    ) -> Result<HashMap<i64, Vec<ProjectLink>>> {
        const BATCH_SIZE: usize = 500;

        let mut by_project: HashMap<i64, Vec<ProjectLink>> = HashMap::new();
        if ids.is_empty() {
            return Ok(by_project);
        }

        for chunk in ids.chunks(BATCH_SIZE) {
            let placeholders = vec!["?"; chunk.len()].join(", ");
            let mut stmt = conn.prepare(&format!(
                "SELECT id, project_id, link_type, title, url, created_at FROM project_links
                 WHERE project_id IN ({})
                 ORDER BY created_at DESC",
                placeholders
            ))?;

            let params: Vec<&dyn rusqlite::ToSql> =
                chunk.iter().map(|id| id as &dyn rusqlite::ToSql).collect();

            let links = stmt
                .query_map(params.as_slice(), |row| {
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

            for link in links {
                by_project.entry(link.project_id).or_default().push(link);
            }
        }

        Ok(by_project)
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
        let conn = self.conn();

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
        let conn = self.conn();
        conn.execute("DELETE FROM project_links WHERE id = ?1", params![id])?;
        Ok(())
    }

    // Métodos para tracking y analytics
    pub fn track_project_open(&self, id: i64) -> Result<()> {
        let conn = self.conn();

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
        let conn = self.conn();

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

        let conn = self.conn();

        // Total de proyectos
        let total_projects: i64 = conn.query_row(
            "SELECT COUNT(*) FROM projects WHERE deleted_at IS NULL",
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
            "SELECT COALESCE(SUM(total_time_seconds), 0) FROM projects WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        let total_time_hours = total_seconds as f64 / 3600.0;

        // Proyecto más activo
        let most_active_project: Option<String> = conn.query_row(
            "SELECT name FROM projects
             WHERE deleted_at IS NULL AND opened_count = (SELECT MAX(opened_count) FROM projects WHERE deleted_at IS NULL)
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

        let conn = self.conn();

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
                "El archivo supera el límite de 5 MB. Elegí uno más chico.".to_string(),
            ));
        }

        let conn = self.conn();

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
        let conn = self.conn();

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
        let conn = self.conn();

        conn.execute("DELETE FROM project_attachments WHERE id = ?1", params![id])?;

        Ok(())
    }

    // ==================== MÉTODOS PARA PROJECT JOURNAL ====================

    pub fn create_journal_entry(&self, entry: CreateJournalEntryDTO) -> Result<JournalEntry> {
        let conn = self.conn();

        conn.execute(
            "INSERT INTO project_journal (project_id, content, tags)
             VALUES (?1, ?2, ?3)",
            params![entry.project_id, entry.content, entry.tags.and_then(Self::empty_to_null)],
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
        let conn = self.conn();

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
        let conn = self.conn();

        let mut set_clauses = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(content) = updates.content {
            set_clauses.push("content = ?");
            params.push(Box::new(content));
        }

        if let Some(tags) = updates.tags {
            set_clauses.push("tags = ?");
            params.push(Box::new(Self::empty_to_null(tags)));
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
        let conn = self.conn();

        conn.execute("DELETE FROM project_journal WHERE id = ?1", params![id])?;

        Ok(())
    }

    // ==================== MÉTODOS PARA PROJECT TODOS ====================

    pub fn create_todo(&self, todo: CreateTodoDTO) -> Result<ProjectTodo> {
        let conn = self.conn();

        conn.execute(
            "INSERT INTO project_todos (project_id, content)
             VALUES (?1, ?2)",
            params![todo.project_id, todo.content],
        )?;

        let id = conn.last_insert_rowid();

        let todo = conn.query_row(
            "SELECT id, project_id, content, is_completed, created_at, completed_at
             FROM project_todos WHERE id = ?1",
            params![id],
            |row| {
                Ok(ProjectTodo {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    content: row.get(2)?,
                    is_completed: row.get(3)?,
                    created_at: row.get(4)?,
                    completed_at: row.get(5)?,
                })
            },
        )?;

        Ok(todo)
    }

    pub fn get_project_todos(&self, project_id: i64) -> Result<Vec<ProjectTodo>> {
        let conn = self.conn();

        let mut stmt = conn.prepare(
            "SELECT id, project_id, content, is_completed, created_at, completed_at
             FROM project_todos
             WHERE project_id = ?1
             ORDER BY is_completed ASC, created_at DESC",
        )?;

        let todos = stmt
            .query_map(params![project_id], |row| {
                Ok(ProjectTodo {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    content: row.get(2)?,
                    is_completed: row.get(3)?,
                    created_at: row.get(4)?,
                    completed_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(todos)
    }

    pub fn update_todo(&self, id: i64, updates: UpdateTodoDTO) -> Result<ProjectTodo> {
        let conn = self.conn();

        let mut set_clauses = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(content) = updates.content {
            set_clauses.push("content = ?");
            params.push(Box::new(content));
        }

        if let Some(is_completed) = updates.is_completed {
            set_clauses.push("is_completed = ?");
            params.push(Box::new(is_completed));

            // Si se completó, actualizar completed_at
            if is_completed {
                set_clauses.push("completed_at = CURRENT_TIMESTAMP");
            } else {
                set_clauses.push("completed_at = NULL");
            }
        }

        if set_clauses.is_empty() {
            return Err(rusqlite::Error::InvalidParameterCount(0, 1));
        }

        params.push(Box::new(id));

        let query = format!(
            "UPDATE project_todos SET {} WHERE id = ?",
            set_clauses.join(", ")
        );

        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&query, params_ref.as_slice())?;

        // Obtener el TODO actualizado
        let todo = conn.query_row(
            "SELECT id, project_id, content, is_completed, created_at, completed_at
             FROM project_todos WHERE id = ?1",
            params![id],
            |row| {
                Ok(ProjectTodo {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    content: row.get(2)?,
                    is_completed: row.get(3)?,
                    created_at: row.get(4)?,
                    completed_at: row.get(5)?,
                })
            },
        )?;

        Ok(todo)
    }

    pub fn delete_todo(&self, id: i64) -> Result<()> {
        let conn = self.conn();

        conn.execute("DELETE FROM project_todos WHERE id = ?1", params![id])?;

        Ok(())
    }

    // ==================== MÉTODOS PARA ESTADOS Y FAVORITOS ====================

    pub fn update_project_status(&self, id: i64, status: String) -> Result<()> {
        let conn = self.conn();

        conn.execute(
            "UPDATE projects
             SET status = ?1, status_changed_at = CURRENT_TIMESTAMP
             WHERE id = ?2",
            params![status, id],
        )?;

        Ok(())
    }

    pub fn toggle_pin_project(&self, id: i64) -> Result<bool> {
        let conn = self.conn();

        // Obtener estado actual
        let is_pinned: bool = conn.query_row(
            "SELECT COALESCE(is_pinned, 0) FROM projects WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;

        let new_pinned = !is_pinned;

        // Si se está fijando, asignar un orden
        if new_pinned {
            // Obtener el máximo orden actual
            let max_order: i64 = conn.query_row(
                "SELECT COALESCE(MAX(pinned_order), 0) FROM projects WHERE is_pinned = 1",
                [],
                |row| row.get(0),
            ).unwrap_or(0);

            conn.execute(
                "UPDATE projects
                 SET is_pinned = ?1, pinned_order = ?2
                 WHERE id = ?3",
                params![new_pinned, max_order + 1, id],
            )?;
        } else {
            conn.execute(
                "UPDATE projects
                 SET is_pinned = ?1, pinned_order = 0
                 WHERE id = ?2",
                params![new_pinned, id],
            )?;
        }

        Ok(new_pinned)
    }

    /// Reordenar los proyectos fijados. TODO o NADA.
    ///
    /// Antes era un `UPDATE` por proyecto suelto dentro del `for`: si uno fallaba a
    /// mitad, los anteriores ya habían quedado escritos y el orden terminaba con
    /// `pinned_order` duplicados o con huecos. Ahora va en una transacción.
    ///
    /// Un id inexistente en la lista ABORTA el reordenamiento entero. Es deliberado:
    /// una lista con un id fantasma significa que el frontend está desincronizado con
    /// la DB, y un reordenamiento a medias es peor que ninguno —deja un orden que el
    /// usuario no pidió y que nadie puede explicar—. Fallar fuerte hace que el front
    /// recargue y el usuario vuelva a arrastrar sobre datos reales.
    pub fn reorder_pinned_projects(&self, project_ids: Vec<i64>) -> Result<()> {
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;

        for (index, project_id) in project_ids.iter().enumerate() {
            let rows = tx.execute(
                "UPDATE projects
                 SET pinned_order = ?1
                 WHERE id = ?2",
                params![index as i64 + 1, project_id],
            )?;

            if rows == 0 {
                // Sale sin commit: el Drop de `tx` hace rollback de lo ya escrito.
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn update_project_order(&self, id: i64, new_order: i64) -> Result<()> {
        let conn = self.conn();

        conn.execute(
            "UPDATE projects
             SET display_order = ?1
             WHERE id = ?2",
            params![new_order, id],
        )?;

        Ok(())
    }

    // ==================== DASHBOARD METHODS ====================

    pub fn get_recent_projects(&self) -> Result<Vec<Project>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            &format!("SELECT {}
             FROM projects
             WHERE last_opened_at IS NOT NULL AND deleted_at IS NULL
             ORDER BY last_opened_at DESC
             LIMIT 5", PROJECT_COLUMNS)
        )?;

        // `links` queda en None, igual que antes: el dashboard no los muestra.
        let projects_iter = stmt.query_map([], row_to_project)?;

        let mut projects = Vec::new();
        for project in projects_iter {
            projects.push(project?);
        }
        Ok(projects)
    }

    pub fn get_all_pending_todos(&self) -> Result<Vec<crate::models::project::DashboardTodo>> {
        use crate::models::project::{DashboardTodo, ProjectTodo};
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT
                pt.id, pt.project_id, pt.content, pt.is_completed, pt.created_at, pt.completed_at,
                p.name as project_name
             FROM project_todos pt
             JOIN projects p ON pt.project_id = p.id
             WHERE pt.is_completed = 0 AND p.deleted_at IS NULL
             ORDER BY pt.created_at DESC"
        )?;

        let todos_iter = stmt.query_map([], |row| {
            Ok(DashboardTodo {
                todo: ProjectTodo {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    content: row.get(2)?,
                    is_completed: row.get(3)?,
                    created_at: row.get(4)?,
                    completed_at: row.get(5)?,
                },
                project_name: row.get(6)?,
            })
        })?;

        let mut todos = Vec::new();
        for todo in todos_iter {
            todos.push(todo?);
        }
        Ok(todos)
    }

    pub fn get_recent_journal_entries(&self) -> Result<Vec<crate::models::project::DashboardJournalEntry>> {
        use crate::models::project::{DashboardJournalEntry, JournalEntry};
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT
                pj.id, pj.project_id, pj.content, pj.tags, pj.created_at, pj.updated_at,
                p.name as project_name
             FROM project_journal pj
             JOIN projects p ON pj.project_id = p.id
             WHERE p.deleted_at IS NULL
             ORDER BY pj.created_at DESC
             LIMIT 5"
        )?;

        let entries_iter = stmt.query_map([], |row| {
            Ok(DashboardJournalEntry {
                entry: JournalEntry {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    content: row.get(2)?,
                    tags: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                },
                project_name: row.get(6)?,
            })
        })?;

        let mut entries = Vec::new();
        for entry in entries_iter {
            entries.push(entry?);
        }
        Ok(entries)
    }

    // ==================== MÉTODOS DE GRUPOS DE PROYECTOS (v0.4.0) ====================

    /// Obtener solo proyectos raíz (sin parent_id, grupos principales)
    pub fn get_root_projects(&self) -> Result<Vec<Project>> {
        let conn = self.conn();

        let mut stmt = conn.prepare(
            &format!("SELECT {}
             FROM projects
             WHERE parent_id IS NULL AND deleted_at IS NULL
             ORDER BY display_order ASC, is_pinned DESC, pinned_order ASC, updated_at DESC", PROJECT_COLUMNS)
        )?;

        let projects_without_links = stmt
            .query_map([], row_to_project)?
            .collect::<Result<Vec<_>>>()?;

        // Los enlaces de TODA la lista en una sola consulta. Antes era una
        // consulta por proyecto, y encima con `unwrap_or_else(|_| Vec::new())`:
        // un fallo al cargar enlaces se tragaba en silencio y el proyecto
        // aparecía sin enlaces sin que nadie se enterara. Ahora el error sube.
        let ids: Vec<i64> = projects_without_links.iter().map(|p| p.id).collect();
        let mut links_by_project = self.get_links_for_projects(&ids, &conn)?;

        let projects: Vec<Project> = projects_without_links
            .into_iter()
            .map(|mut project| {
                project.links =
                    Some(links_by_project.remove(&project.id).unwrap_or_default());
                project
            })
            .collect();

        Ok(projects)
    }

    /// Obtener subproyectos de un grupo (parent_id = id)
    pub fn get_subprojects(&self, parent_id: i64) -> Result<Vec<Project>> {
        let conn = self.conn();

        let mut stmt = conn.prepare(
            &format!("SELECT {}
             FROM projects
             WHERE parent_id = ?1 AND deleted_at IS NULL
             ORDER BY display_order ASC, name ASC", PROJECT_COLUMNS)
        )?;

        let projects_without_links = stmt
            .query_map([parent_id], row_to_project)?
            .collect::<Result<Vec<_>>>()?;

        // Los enlaces de TODA la lista en una sola consulta. Antes era una
        // consulta por proyecto, y encima con `unwrap_or_else(|_| Vec::new())`:
        // un fallo al cargar enlaces se tragaba en silencio y el proyecto
        // aparecía sin enlaces sin que nadie se enterara. Ahora el error sube.
        let ids: Vec<i64> = projects_without_links.iter().map(|p| p.id).collect();
        let mut links_by_project = self.get_links_for_projects(&ids, &conn)?;

        let projects: Vec<Project> = projects_without_links
            .into_iter()
            .map(|mut project| {
                project.links =
                    Some(links_by_project.remove(&project.id).unwrap_or_default());
                project
            })
            .collect();

        Ok(projects)
    }

    /// Obtener proyecto con sus hijos
    pub fn get_project_with_children(&self, id: i64) -> Result<ProjectWithChildren> {
        let project = self.get_project(id)?;
        let children = self.get_subprojects(id)?;
        let subproject_count = children.len() as i64;

        Ok(ProjectWithChildren {
            project,
            children,
            subproject_count,
        })
    }

    /// Contar subproyectos de un grupo
    pub fn count_subprojects(&self, parent_id: i64) -> Result<i64> {
        let conn = self.conn();

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM projects WHERE parent_id = ?1 AND deleted_at IS NULL",
            params![parent_id],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    /// Recorre la cadena de ancestros de `start` (subiendo por parent_id) y devuelve
    /// true si encuentra `needle`. Recibe la conexión YA bloqueada: NO re-bloquea el
    /// Mutex (no es reentrante -> deadlock). `max_depth` evita un loop infinito si la
    /// DB ya tuviera un ciclo por datos corruptos previos.
    fn ancestor_chain_contains(
        conn: &Connection,
        start: i64,
        needle: i64,
        max_depth: usize,
    ) -> std::result::Result<bool, String> {
        let mut current = start;
        let mut depth = 0;
        loop {
            if current == needle {
                return Ok(true);
            }
            let next: Option<i64> = match conn.query_row(
                "SELECT parent_id FROM projects WHERE id = ?1",
                params![current],
                |row| row.get::<_, Option<i64>>(0),
            ) {
                Ok(parent) => parent,
                Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
                Err(e) => return Err(e.to_string()),
            };
            match next {
                None => return Ok(false),
                Some(n) => current = n,
            }
            depth += 1;
            if depth > max_depth {
                return Err("Estructura de grupos inconsistente detectada.".to_string());
            }
        }
    }

    /// Asignar proyecto a un grupo (o quitarlo del grupo actual).
    /// Barrera AUTORITATIVA contra ciclos: rechaza self-parent, padre inexistente, y
    /// cualquier asignación que cerraría un ciclo en la jerarquía. El frontend filtra
    /// por UX, pero esta es la garantía real (no confiar en el cliente).
    pub fn assign_project_to_group(
        &self,
        child_id: i64,
        new_parent_id: Option<i64>,
    ) -> std::result::Result<(), String> {
        let conn = self.conn();

        if let Some(parent_id) = new_parent_id {
            // 1) Un proyecto no puede ser su propio grupo padre
            if parent_id == child_id {
                return Err("No podés asignar un proyecto como su propio grupo padre.".to_string());
            }
            // 2) El grupo padre debe existir (no hay FOREIGN KEY que lo garantice)
            let exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM projects WHERE id = ?1 AND deleted_at IS NULL",
                    params![parent_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            if exists == 0 {
                return Err("El grupo padre seleccionado no existe.".to_string());
            }
            // 3) No crear un ciclo: child no debe ser ancestro del nuevo padre
            if Self::ancestor_chain_contains(&conn, parent_id, child_id, 1000)? {
                return Err(
                    "No podés mover este grupo dentro de uno de sus subproyectos (crearía un ciclo)."
                        .to_string(),
                );
            }
        }

        conn.execute(
            "UPDATE projects SET parent_id = ?1 WHERE id = ?2",
            params![new_parent_id, child_id],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    // ==================== TIME TRACKING METHODS ====================

    /// Crear una nueva sesión de tracking
    pub fn create_tracking_session(&self, project_id: i64, source: &str) -> Result<i64> {
        let conn = self.conn();

        conn.execute(
            "INSERT INTO time_tracking_sessions (project_id, started_at, source)
             VALUES (?1, CURRENT_TIMESTAMP, ?2)",
            params![project_id, source],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Terminar una sesión de tracking: cerrarla y acreditar su duración al proyecto.
    ///
    /// Los dos UPDATE van en UNA transacción. Antes eran dos `execute` sueltos bajo el
    /// mismo lock: si el segundo fallaba, la sesión quedaba cerrada y el tiempo
    /// trabajado se perdía en silencio, sin error y sin forma de recuperarlo.
    pub fn end_tracking_session(&self, session_id: i64, duration_seconds: i64) -> Result<()> {
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;

        tx.execute(
            "UPDATE time_tracking_sessions
             SET ended_at = CURRENT_TIMESTAMP, duration_seconds = ?1
             WHERE id = ?2",
            params![duration_seconds, session_id],
        )?;

        // También actualizar el tiempo total del proyecto
        tx.execute(
            "UPDATE projects
             SET total_time_seconds = COALESCE(total_time_seconds, 0) + ?1
             WHERE id = (SELECT project_id FROM time_tracking_sessions WHERE id = ?2)",
            params![duration_seconds, session_id],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Obtener sesiones de tracking de un proyecto
    pub fn get_tracking_sessions(&self, project_id: i64, limit: i64) -> Result<Vec<crate::tracking::aggregator::TimeTrackingSession>> {
        use crate::tracking::aggregator::TimeTrackingSession;

        let conn = self.conn();

        let mut stmt = conn.prepare(
            "SELECT id, project_id, started_at, ended_at, duration_seconds, source
             FROM time_tracking_sessions
             WHERE project_id = ?1
             ORDER BY started_at DESC
             LIMIT ?2"
        )?;

        let sessions = stmt.query_map(params![project_id, limit], |row| {
            Ok(TimeTrackingSession {
                id: row.get(0)?,
                project_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                duration_seconds: row.get(4)?,
                source: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

        Ok(sessions)
    }

    /// Obtener estadísticas de tiempo para un proyecto
    pub fn get_time_stats(&self, project_id: i64) -> Result<crate::tracking::aggregator::TimeStats> {
        use crate::tracking::aggregator::TimeStats;

        let conn = self.conn();

        // Tiempo total y número de sesiones
        let (total_seconds, session_count): (i64, i64) = conn.query_row(
            "SELECT COALESCE(SUM(duration_seconds), 0), COUNT(*)
             FROM time_tracking_sessions
             WHERE project_id = ?1 AND duration_seconds IS NOT NULL",
            params![project_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap_or((0, 0));

        // Promedio de duración
        let avg_session_seconds = if session_count > 0 {
            total_seconds / session_count
        } else {
            0
        };

        // Sesión más larga
        let longest_session_seconds: i64 = conn.query_row(
            "SELECT COALESCE(MAX(duration_seconds), 0)
             FROM time_tracking_sessions
             WHERE project_id = ?1",
            params![project_id],
            |row| row.get(0),
        ).unwrap_or(0);

        // Tiempo hoy
        let today_seconds: i64 = conn.query_row(
            "SELECT COALESCE(SUM(duration_seconds), 0)
             FROM time_tracking_sessions
             WHERE project_id = ?1 AND DATE(started_at) = DATE('now')",
            params![project_id],
            |row| row.get(0),
        ).unwrap_or(0);

        // Tiempo esta semana (últimos 7 días)
        let week_seconds: i64 = conn.query_row(
            "SELECT COALESCE(SUM(duration_seconds), 0)
             FROM time_tracking_sessions
             WHERE project_id = ?1 AND started_at >= DATE('now', '-7 days')",
            params![project_id],
            |row| row.get(0),
        ).unwrap_or(0);

        Ok(TimeStats {
            total_seconds,
            session_count,
            avg_session_seconds,
            longest_session_seconds,
            today_seconds,
            week_seconds,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::*;
    use std::path::PathBuf;

    fn test_db() -> Database {
        Database::new(PathBuf::from(":memory:")).expect("Failed to create in-memory database")
    }

    fn seed_project_at(db: &Database, name: &str, local_path: &str) -> i64 {
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

    fn seed_link(db: &Database, project_id: i64, title: &str) {
        db.create_link(CreateLinkDTO {
            project_id,
            link_type: "docs".to_string(),
            title: title.to_string(),
            url: format!("https://example.com/{}", title),
        })
        .expect("no se pudo sembrar el enlace");
    }

    fn link_titles(project: &Project) -> Vec<String> {
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

    // ============ B15: el envenenamiento del mutex no mata la app ============

    #[test]
    fn la_base_sigue_operando_despues_de_un_panic_con_el_lock_tomado() {
        let db = test_db();
        seed_project_at(&db, "Antes del panic", "/tmp/antes");

        // Envenenar el mutex a mano: un hilo toma el lock y entra en panic. Es
        // exactamente lo que pasa cuando cualquier camino de la app panickea con
        // la conexión tomada, y es la ÚNICA forma de que `Mutex::lock()` devuelva
        // Err.
        let panicked = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let _guard = db.conn.lock().expect("el mutex arranca sano");
                    panic!("panic deliberado con el lock de la conexión tomado");
                })
                .join()
        });
        assert!(panicked.is_err(), "el hilo tenía que entrar en panic");

        // Con `lock().unwrap()` esta línea era un panic, y con ella TODA operación
        // de base posterior: un fallo aislado en un camino cualquiera dejaba la
        // app muerta hasta el reinicio. Este test fallaba con el código anterior.
        let projects = db
            .get_all_projects()
            .expect("la base tiene que seguir operando después del envenenamiento");
        assert_eq!(projects.len(), 1);

        // Y no es solo leer: escribir también.
        seed_project_at(&db, "Después del panic", "/tmp/despues");
        assert_eq!(db.get_all_projects().unwrap().len(), 2);
    }

    // ==================== B16: los enlaces se cargan en lote ====================

    #[test]
    fn los_tres_listados_devuelven_los_enlaces_completos_de_un_proyecto() {
        let db = test_db();
        let id = seed_project_at(&db, "Con enlaces", "/tmp/con-enlaces");
        seed_link(&db, id, "alfa");
        seed_link(&db, id, "beta");
        seed_link(&db, id, "gamma");

        let esperado = vec![
            "alfa".to_string(),
            "beta".to_string(),
            "gamma".to_string(),
        ];

        // Los tres caminos de lectura comparten el mismo batch: si el agrupado por
        // project_id se rompe, se rompe en los tres.
        let todos = db.get_all_projects().unwrap();
        assert_eq!(link_titles(&todos[0]), esperado, "get_all_projects");

        let raices = db.get_root_projects().unwrap();
        assert_eq!(link_titles(&raices[0]), esperado, "get_root_projects");

        let encontrados = db.search_projects("Con enlaces").unwrap();
        assert_eq!(link_titles(&encontrados[0]), esperado, "search_projects");
    }

    #[test]
    fn un_proyecto_sin_enlaces_trae_lista_vacia_y_no_none() {
        let db = test_db();
        seed_project_at(&db, "Sin enlaces", "/tmp/sin-enlaces");

        let todos = db.get_all_projects().unwrap();
        // La distinción importa: el frontend hace `project.links.length` y un `None`
        // que serializa a `null` lo rompe. El batch omite del mapa a los proyectos
        // sin enlaces, así que este es el caso que verifica el `unwrap_or_default`.
        let links = todos[0]
            .links
            .as_ref()
            .expect("links debe ser Some(vec![]), no None");
        assert!(links.is_empty(), "un proyecto sin enlaces no debe traer ninguno");
    }

    #[test]
    fn con_dos_proyectos_los_enlaces_no_se_le_atribuyen_al_equivocado() {
        let db = test_db();
        let con = seed_project_at(&db, "Con enlaces", "/tmp/con");
        let sin = seed_project_at(&db, "Sin enlaces", "/tmp/sin");
        seed_link(&db, con, "alfa");
        seed_link(&db, con, "beta");

        let todos = db.get_all_projects().unwrap();
        let del_con = todos.iter().find(|p| p.id == con).unwrap();
        let del_sin = todos.iter().find(|p| p.id == sin).unwrap();

        // Este es el test que detecta un error de agrupación en el HashMap: con una
        // sola consulta para toda la lista, un `entry()` mal armado le cuelga los dos
        // enlaces al proyecto equivocado, o se los cuelga a los dos.
        assert_eq!(link_titles(del_con), vec!["alfa".to_string(), "beta".to_string()]);
        assert_eq!(link_titles(del_sin), Vec::<String>::new());
    }

    #[test]
    fn los_subproyectos_tambien_traen_sus_enlaces() {
        let db = test_db();
        let padre = seed_project_at(&db, "Padre", "/tmp/padre");
        let hijo = db
            .create_project(CreateProjectDTO {
                name: "Hijo".to_string(),
                description: "desc".to_string(),
                local_path: "/tmp/hijo".to_string(),
                documentation_url: None,
                ai_documentation_url: None,
                drive_link: None,
                notes: None,
                image_data: None,
                parent_id: Some(padre),
                group_color: None,
                group_icon: None,
            })
            .unwrap();
        seed_link(&db, hijo.id, "manual");

        let hijos = db.get_subprojects(padre).unwrap();
        assert_eq!(link_titles(&hijos[0]), vec!["manual".to_string()]);
    }

    // ==================== B4: get_project filtra la papelera ====================

    #[test]
    fn test_get_project_returns_active_project() {
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();

        let fetched = db.get_project(created.id).expect("un proyecto activo debe leerse");
        assert_eq!(fetched.id, created.id);
    }

    #[test]
    fn test_get_project_rejects_trashed_project() {
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();
        db.delete_project(created.id).unwrap();

        let err = db
            .get_project(created.id)
            .expect_err("un proyecto en papelera no debe leerse");
        assert!(
            matches!(err, rusqlite::Error::QueryReturnedNoRows),
            "debe ser QueryReturnedNoRows, fue: {err:?}"
        );
    }

    /// `get_project_with_children` compone `get_project`, así que hereda el filtro.
    #[test]
    fn test_get_project_with_children_rejects_trashed_parent() {
        let db = test_db();
        let parent = db.create_project(test_project_dto()).unwrap();
        db.delete_project(parent.id).unwrap();

        assert!(
            db.get_project_with_children(parent.id).is_err(),
            "un grupo en papelera no debe leerse con get_project_with_children"
        );
    }

    /// EL test de regresión que importa: el filtro NO rompió la restauración.
    /// `restore_project` no compone `get_project` —hace su propio SELECT y exige que el
    /// proyecto SÍ esté en papelera—, así que el camino de vuelta sigue intacto.
    #[test]
    fn test_restore_project_still_works_after_trash_filter() {
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();
        db.delete_project(created.id).unwrap();

        // Invisible mientras está en papelera...
        assert!(db.get_project(created.id).is_err());

        // ...pero restaurable, y después visible de nuevo.
        db.restore_project(created.id)
            .expect("restore_project debe seguir funcionando sobre un proyecto en papelera");
        assert!(
            db.get_project(created.id).is_ok(),
            "tras restaurar, get_project debe volver a devolverlo"
        );
    }

    /// `purge_project` tampoco compone `get_project`: sigue exigiendo papelera.
    #[test]
    fn test_purge_project_still_works_after_trash_filter() {
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();
        db.delete_project(created.id).unwrap();
        assert!(db.get_project(created.id).is_err());

        db.purge_project(created.id)
            .expect("purge_project debe seguir funcionando sobre un proyecto en papelera");
        assert!(db.list_trash().unwrap().is_empty());
    }

    #[test]
    fn test_is_registered_project_path_true_for_registered() {
        let db = test_db();
        seed_project_at(&db, "Alpha", "/home/user/alpha");

        assert!(db.is_registered_project_path("/home/user/alpha").unwrap());
    }

    #[test]
    fn test_is_registered_project_path_false_for_unregistered() {
        let db = test_db();
        seed_project_at(&db, "Alpha", "/home/user/alpha");

        assert!(!db.is_registered_project_path("/home/user/ajeno").unwrap());
        assert!(!db.is_registered_project_path("").unwrap());
    }

    /// EL test que importa: mandar un proyecto a la papelera REVOCA el permiso git
    /// sobre su carpeta. Si `deleted_at IS NULL` se cayera de la query, esto pasa a
    /// verde en falso y un proyecto borrado seguiría habilitando git.
    #[test]
    fn test_is_registered_project_path_false_for_trashed_project() {
        let db = test_db();
        let id = seed_project_at(&db, "Alpha", "/home/user/alpha");
        assert!(db.is_registered_project_path("/home/user/alpha").unwrap());

        db.delete_project(id).expect("no se pudo borrar");

        assert!(
            !db.is_registered_project_path("/home/user/alpha").unwrap(),
            "un proyecto en papelera no debe seguir registrado para git"
        );

        // Y restaurarlo vuelve a habilitarlo.
        db.restore_project(id).expect("no se pudo restaurar");
        assert!(db.is_registered_project_path("/home/user/alpha").unwrap());
    }

    /// `is_registered_project_path` compara el string TAL CUAL: la barra final y el
    /// './' intermedio son un miss acá a propósito. Normalizarlos es responsabilidad
    /// del guard (`commands::guards`), que canonicaliza ambos lados contra el disco.
    #[test]
    fn test_is_registered_project_path_is_an_exact_string_match() {
        let db = test_db();
        seed_project_at(&db, "Alpha", "/home/user/alpha");

        assert!(!db.is_registered_project_path("/home/user/alpha/").unwrap());
        assert!(!db.is_registered_project_path("/home/user/./alpha").unwrap());
        assert!(!db.is_registered_project_path("/home/user/alpha/sub").unwrap());
    }

    #[test]
    fn test_active_project_paths_excludes_trashed() {
        let db = test_db();
        seed_project_at(&db, "Alpha", "/home/user/alpha");
        let beta = seed_project_at(&db, "Beta", "/home/user/beta");

        let mut paths = db.active_project_paths().unwrap();
        paths.sort();
        assert_eq!(paths, vec!["/home/user/alpha", "/home/user/beta"]);

        db.delete_project(beta).expect("no se pudo borrar");

        assert_eq!(db.active_project_paths().unwrap(), vec!["/home/user/alpha"]);
    }

    /// Siembra UNA fila en cada una de las 6 tablas hijas de `project_id`. Usado para
    /// verificar que purge_project/empty_trash realmente vacían las 6, no solo las
    /// que el test anterior (`test_purge_only_on_trashed_and_removes_for_real`) nunca
    /// llegaba a poblar.
    fn seed_all_child_tables(db: &Database, project_id: i64) {
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
    fn child_row_counts(db: &Database, project_id: i64) -> (i64, i64, i64, i64, i64, i64) {
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

    fn test_project_dto() -> CreateProjectDTO {
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

    // ==================== Paso 1: Inicialización y CRUD de Proyectos ====================

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
    fn test_create_project() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        assert!(project.id > 0);
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.status.as_deref(), Some("activo"));
    }

    #[test]
    fn test_create_project_with_all_fields() {
        let db = test_db();
        let dto = CreateProjectDTO {
            name: "Full Project".to_string(),
            description: "Full desc".to_string(),
            local_path: "/tmp/full".to_string(),
            documentation_url: Some("https://docs.example.com".to_string()),
            ai_documentation_url: Some("https://ai.example.com".to_string()),
            drive_link: Some("https://drive.google.com/xxx".to_string()),
            notes: Some("Some notes".to_string()),
            image_data: None,
            parent_id: None,
            group_color: Some("#FF0000".to_string()),
            group_icon: Some("rocket".to_string()),
        };
        let project = db.create_project(dto).unwrap();
        assert_eq!(project.documentation_url.as_deref(), Some("https://docs.example.com"));
        assert_eq!(project.drive_link.as_deref(), Some("https://drive.google.com/xxx"));
        assert_eq!(project.notes.as_deref(), Some("Some notes"));
        assert_eq!(project.group_color.as_deref(), Some("#FF0000"));
        assert_eq!(project.group_icon.as_deref(), Some("rocket"));
    }

    #[test]
    fn test_get_project() {
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();
        let fetched = db.get_project(created.id).unwrap();
        assert_eq!(fetched.name, "Test Project");
        assert!(fetched.links.is_some());
        assert_eq!(fetched.links.unwrap().len(), 0);
    }

    #[test]
    fn test_get_project_not_found() {
        let db = test_db();
        let result = db.get_project(9999);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_all_projects_empty() {
        let db = test_db();
        let projects = db.get_all_projects().unwrap();
        assert!(projects.is_empty());
    }

    #[test]
    fn test_get_all_projects_returns_all() {
        let db = test_db();
        for i in 0..3 {
            let mut dto = test_project_dto();
            dto.name = format!("Project {}", i);
            db.create_project(dto).unwrap();
        }
        let projects = db.get_all_projects().unwrap();
        assert_eq!(projects.len(), 3);
    }

    #[test]
    fn test_update_project_name() {
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();
        let updated = db
            .update_project(
                created.id,
                UpdateProjectDTO {
                    name: Some("Renamed".to_string()),
                    description: None,
                    local_path: None,
                    documentation_url: None,
                    ai_documentation_url: None,
                    drive_link: None,
                    notes: None,
                    image_data: None,
                    parent_id: None,
                    group_color: None,
                    group_icon: None,
                },
            )
            .unwrap();
        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.description, "A test project");
    }

    #[test]
    fn test_update_project_multiple_fields() {
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();
        let updated = db
            .update_project(
                created.id,
                UpdateProjectDTO {
                    name: Some("New Name".to_string()),
                    description: Some("New desc".to_string()),
                    local_path: None,
                    documentation_url: None,
                    ai_documentation_url: None,
                    drive_link: None,
                    notes: Some("New notes".to_string()),
                    image_data: None,
                    parent_id: None,
                    group_color: None,
                    group_icon: None,
                },
            )
            .unwrap();
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.description, "New desc");
        assert_eq!(updated.notes.as_deref(), Some("New notes"));
    }

    #[test]
    fn test_update_project_clears_optional_field_to_null() {
        // Regresión: antes era IMPOSIBLE vaciar un campo opcional. El frontend mandaba
        // `|| undefined` (omitido) y el backend saltaba el campo. Ahora el frontend manda
        // "" (Some("")) y empty_to_null lo normaliza a NULL.
        let db = test_db();
        let created = db
            .create_project(CreateProjectDTO {
                documentation_url: Some("https://docs.viejo.com".to_string()),
                notes: Some("notas viejas".to_string()),
                ..test_project_dto()
            })
            .unwrap();
        assert_eq!(
            created.documentation_url.as_deref(),
            Some("https://docs.viejo.com")
        );

        // El usuario borra el campo en la UI -> llega Some("")
        let updated = db
            .update_project(
                created.id,
                UpdateProjectDTO {
                    name: None,
                    description: None,
                    local_path: None,
                    documentation_url: Some("".to_string()),
                    ai_documentation_url: None,
                    drive_link: None,
                    notes: Some("   ".to_string()), // solo espacios también limpia
                    image_data: None,
                    parent_id: None,
                    group_color: None,
                    group_icon: None,
                },
            )
            .unwrap();

        // El campo quedó vaciado a NULL (None), no a cadena vacía
        assert_eq!(updated.documentation_url, None);
        assert_eq!(updated.notes, None);
    }

    #[test]
    fn test_create_project_normalizes_empty_to_null() {
        let db = test_db();
        let created = db
            .create_project(CreateProjectDTO {
                documentation_url: Some("".to_string()),
                ..test_project_dto()
            })
            .unwrap();
        // Crear con "" guarda NULL, no "" -> DB consistente con update
        assert_eq!(created.documentation_url, None);
    }

    #[test]
    fn test_backup_to_creates_verifiable_copy() {
        // El backup (VACUUM INTO) debe producir una copia válida y verificable.
        let db = test_db();
        db.create_project(test_project_dto()).unwrap();
        let dest = std::env::temp_dir().join("gp-test-backup-verifiable.db");
        let _ = std::fs::remove_file(&dest); // VACUUM INTO falla si el destino existe
        let dest_str = dest.to_str().unwrap();

        db.backup_to(dest_str).unwrap();
        assert!(dest.exists(), "el archivo de backup debe existir");

        let (ok, count) = Database::verify_db_file(dest_str).unwrap();
        assert!(ok, "integrity_check de la copia debe ser ok");
        assert_eq!(count, 1, "la copia debe contener el proyecto creado");

        let _ = std::fs::remove_file(&dest);
    }

    #[test]
    fn test_verify_db_file_rejects_non_db() {
        // Un archivo que NO es SQLite no debe pasar la verificación (debe dar Err),
        // así run_backup nunca deja un "backup" falso en disco.
        let dest = std::env::temp_dir().join("gp-test-notdb.txt");
        std::fs::write(&dest, b"esto no es una base de datos sqlite").unwrap();
        let res = Database::verify_db_file(dest.to_str().unwrap());
        assert!(res.is_err(), "un archivo no-SQLite no debe verificar OK");
        let _ = std::fs::remove_file(&dest);
    }

    #[test]
    fn test_delete_project_is_soft() {
        // delete_project ahora es SOFT: desaparece de las listas pero queda en la papelera.
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();
        db.delete_project(created.id).unwrap();
        // No aparece en la lista principal ni en raíz...
        assert!(db.get_all_projects().unwrap().iter().all(|p| p.id != created.id));
        assert!(db.get_root_projects().unwrap().iter().all(|p| p.id != created.id));
        // ...pero SÍ está en la papelera...
        assert!(db.list_trash().unwrap().iter().any(|p| p.id == created.id));
        // ...y get_project YA NO lo devuelve (B4).
        //
        // El comentario anterior acá decía "el lookup por id sigue funcionando (lo
        // necesita restaurar)". Era FALSO: `restore_project` no compone `get_project`,
        // hace su propio `SELECT deleted_at FROM projects WHERE id = ?1`. Ver
        // `test_restore_project_still_works_after_trash_filter`.
        assert!(
            db.get_project(created.id).is_err(),
            "un proyecto en papelera no debe ser legible con get_project"
        );
    }

    #[test]
    fn test_restore_project() {
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();
        db.delete_project(created.id).unwrap();
        db.restore_project(created.id).unwrap();
        // Vuelve a la lista y sale de la papelera.
        assert!(db.get_all_projects().unwrap().iter().any(|p| p.id == created.id));
        assert!(db.list_trash().unwrap().is_empty());
    }

    #[test]
    fn test_restore_project_rejects_nonexistent_id() {
        // Antes: un id inexistente hacía que el UPDATE afectara 0 filas y la función
        // devolviera Ok(()) como si hubiera restaurado algo.
        let db = test_db();
        let result = db.restore_project(9999);
        assert!(
            result.is_err(),
            "restaurar un id inexistente debe devolver Err, no un Ok(()) silencioso"
        );
    }

    #[test]
    fn test_restore_project_rejects_already_active_project() {
        // Un proyecto que nunca se borró (deleted_at IS NULL) no está "en papelera":
        // restaurarlo debe rechazarse igual que purge_project rechaza purgar un activo.
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();
        let result = db.restore_project(created.id);
        assert!(
            result.is_err(),
            "restaurar un proyecto ya activo debe devolver Err, no un Ok(()) silencioso"
        );
    }

    #[test]
    fn test_delete_group_cascades_to_children() {
        // Borrar un grupo manda sus hijos a la papelera (NO quedan huérfanos).
        let db = test_db();
        let group = db.create_project(test_project_dto()).unwrap();
        let mut child_dto = test_project_dto();
        child_dto.name = "Hijo".to_string();
        let child = db.create_project(child_dto).unwrap();
        db.assign_project_to_group(child.id, Some(group.id)).unwrap();

        db.delete_project(group.id).unwrap();
        // Grupo + hijo en la papelera, ninguno visible (sin huérfanos colgados).
        assert_eq!(db.list_trash().unwrap().len(), 2);
        assert!(db.get_root_projects().unwrap().is_empty());
        assert!(db.get_subprojects(group.id).unwrap().is_empty());
    }

    #[test]
    fn test_delete_cascades_recursively_to_grandchildren() {
        // Cascada N-niveles: borrar el abuelo manda padre Y nieto a la papelera.
        // Antes (cascada 1 nivel) el nieto quedaba activo e invisible (huérfano).
        let db = test_db();
        let a = db.create_project(test_project_dto()).unwrap(); // abuelo (raíz)
        let mut b_dto = test_project_dto();
        b_dto.name = "B".to_string();
        let b = db.create_project(b_dto).unwrap(); // padre
        let mut c_dto = test_project_dto();
        c_dto.name = "C".to_string();
        let c = db.create_project(c_dto).unwrap(); // nieto
        db.assign_project_to_group(b.id, Some(a.id)).unwrap();
        db.assign_project_to_group(c.id, Some(b.id)).unwrap();

        db.delete_project(a.id).unwrap();
        // Los 3 en la papelera, ninguno activo/visible.
        assert_eq!(db.list_trash().unwrap().len(), 3);
        assert!(db.get_all_projects().unwrap().is_empty());

        // Y restaurar el abuelo trae de vuelta todo el subárbol del batch.
        db.restore_project(a.id).unwrap();
        assert!(db.list_trash().unwrap().is_empty());
        assert_eq!(db.get_all_projects().unwrap().len(), 3);
    }

    #[test]
    fn test_purge_only_on_trashed_and_removes_for_real() {
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();
        // Purgar un proyecto ACTIVO debe rechazarse (solo se purga lo que está en papelera).
        assert!(db.purge_project(created.id).is_err());
        // Tras mandarlo a la papelera, el purge sí lo borra DE VERDAD.
        db.delete_project(created.id).unwrap();
        db.purge_project(created.id).unwrap();
        assert!(db.get_project(created.id).is_err());
        assert!(db.list_trash().unwrap().is_empty());
    }

    #[test]
    fn test_purge_project_cascades_all_six_child_tables() {
        // El test anterior (arriba) nunca sembraba las 6 tablas hijas, así que nunca
        // verificaba que el DELETE en cascada de purge_project_internal funcionara de
        // verdad contra las 6. Este sí.
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();
        seed_all_child_tables(&db, created.id);
        assert_eq!(
            child_row_counts(&db, created.id),
            (1, 1, 1, 1, 1, 1),
            "sanity: las 6 tablas hijas deben tener su fila sembrada antes de purgar"
        );

        db.delete_project(created.id).unwrap();
        db.purge_project(created.id).unwrap();

        assert_eq!(
            child_row_counts(&db, created.id),
            (0, 0, 0, 0, 0, 0),
            "purge_project debe dejar las 6 tablas hijas vacías para el proyecto purgado"
        );
    }

    #[test]
    fn test_empty_trash_cascades_all_six_child_tables() {
        // empty_trash no tenía UN SOLO test antes de este work-unit.
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();
        seed_all_child_tables(&db, created.id);

        db.delete_project(created.id).unwrap();
        db.empty_trash().unwrap();

        assert_eq!(
            child_row_counts(&db, created.id),
            (0, 0, 0, 0, 0, 0),
            "empty_trash debe dejar las 6 tablas hijas vacías para el proyecto purgado"
        );
        assert!(db.get_project(created.id).is_err());
        assert!(db.list_trash().unwrap().is_empty());
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

    #[test]
    fn test_create_project_rejects_nonexistent_parent_id() {
        let db = test_db();
        let mut dto = test_project_dto();
        dto.parent_id = Some(9999);
        let result = db.create_project(dto);
        assert!(
            result.is_err(),
            "crear un proyecto con un parent_id inexistente debe rechazarse"
        );
    }

    #[test]
    fn test_create_project_rejects_trashed_parent_id() {
        let db = test_db();
        let parent = db.create_project(test_project_dto()).unwrap();
        db.delete_project(parent.id).unwrap(); // el padre ahora está en papelera

        let mut dto = test_project_dto();
        dto.name = "Hijo".to_string();
        dto.parent_id = Some(parent.id);
        let result = db.create_project(dto);
        assert!(
            result.is_err(),
            "crear un proyecto con un parent_id en papelera debe rechazarse"
        );
    }

    #[test]
    fn test_create_project_accepts_active_parent_id() {
        // Caso feliz: no romper el flujo normal de crear un subproyecto dentro de un
        // grupo activo.
        let db = test_db();
        let parent = db.create_project(test_project_dto()).unwrap();

        let mut dto = test_project_dto();
        dto.name = "Hijo".to_string();
        dto.parent_id = Some(parent.id);
        let child = db.create_project(dto).unwrap();
        assert_eq!(child.parent_id, Some(parent.id));
    }

    #[test]
    fn test_search_projects_by_name() {
        let db = test_db();
        let mut dto = test_project_dto();
        dto.name = "Alpha Beta".to_string();
        db.create_project(dto).unwrap();
        let mut dto2 = test_project_dto();
        dto2.name = "Gamma".to_string();
        db.create_project(dto2).unwrap();
        let results = db.search_projects("Alpha").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Alpha Beta");
    }

    #[test]
    fn test_search_projects_by_notes() {
        let db = test_db();
        let mut dto = test_project_dto();
        dto.notes = Some("keyword_unique_xyz".to_string());
        db.create_project(dto).unwrap();
        let results = db.search_projects("keyword_unique_xyz").unwrap();
        assert_eq!(results.len(), 1);
    }

    // ==================== Paso 2: CRUD de Links ====================

    #[test]
    fn test_create_link() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let link = db
            .create_link(CreateLinkDTO {
                project_id: project.id,
                link_type: "github".to_string(),
                title: "Repo".to_string(),
                url: "https://github.com/test".to_string(),
            })
            .unwrap();
        assert!(link.id > 0);
        assert_eq!(link.project_id, project.id);
        assert_eq!(link.link_type, "github");
        assert_eq!(link.title, "Repo");
        assert_eq!(link.url, "https://github.com/test");
    }

    #[test]
    fn test_get_project_links() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        db.create_link(CreateLinkDTO {
            project_id: project.id,
            link_type: "docs".to_string(),
            title: "Docs".to_string(),
            url: "https://docs.example.com".to_string(),
        })
        .unwrap();
        db.create_link(CreateLinkDTO {
            project_id: project.id,
            link_type: "github".to_string(),
            title: "Repo".to_string(),
            url: "https://github.com/test".to_string(),
        })
        .unwrap();
        let links = db.get_project_links(project.id).unwrap();
        assert_eq!(links.len(), 2);
    }

    #[test]
    fn test_get_project_links_empty() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let links = db.get_project_links(project.id).unwrap();
        assert!(links.is_empty());
    }

    #[test]
    fn test_update_link() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let link = db
            .create_link(CreateLinkDTO {
                project_id: project.id,
                link_type: "github".to_string(),
                title: "Old Title".to_string(),
                url: "https://github.com/old".to_string(),
            })
            .unwrap();
        let updated = db
            .update_link(
                link.id,
                UpdateLinkDTO {
                    link_type: None,
                    title: Some("New Title".to_string()),
                    url: None,
                },
            )
            .unwrap();
        assert_eq!(updated.title, "New Title");
        assert_eq!(updated.url, "https://github.com/old");
    }

    #[test]
    fn test_update_link_empty_fails() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let link = db
            .create_link(CreateLinkDTO {
                project_id: project.id,
                link_type: "github".to_string(),
                title: "Title".to_string(),
                url: "https://github.com/test".to_string(),
            })
            .unwrap();
        let result = db.update_link(
            link.id,
            UpdateLinkDTO {
                link_type: None,
                title: None,
                url: None,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_link() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let link = db
            .create_link(CreateLinkDTO {
                project_id: project.id,
                link_type: "github".to_string(),
                title: "Title".to_string(),
                url: "https://github.com/test".to_string(),
            })
            .unwrap();
        db.delete_link(link.id).unwrap();
        let links = db.get_project_links(project.id).unwrap();
        assert!(links.is_empty());
    }

    #[test]
    fn test_links_included_in_get_all_projects() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        db.create_link(CreateLinkDTO {
            project_id: project.id,
            link_type: "github".to_string(),
            title: "Repo".to_string(),
            url: "https://github.com/test".to_string(),
        })
        .unwrap();
        let projects = db.get_all_projects().unwrap();
        assert_eq!(projects.len(), 1);
        let links = projects[0].links.as_ref().unwrap();
        assert_eq!(links.len(), 1);
    }

    // ==================== Paso 3: Journal, Todos, Attachments ====================

    // --- Journal ---

    #[test]
    fn test_create_journal_entry() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let entry = db
            .create_journal_entry(CreateJournalEntryDTO {
                project_id: project.id,
                content: "First entry".to_string(),
                tags: Some("[\"bug\",\"tip\"]".to_string()),
            })
            .unwrap();
        assert!(entry.id > 0);
        assert_eq!(entry.content, "First entry");
        assert_eq!(entry.tags.as_deref(), Some("[\"bug\",\"tip\"]"));
    }

    #[test]
    fn test_get_journal_entries() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        db.create_journal_entry(CreateJournalEntryDTO {
            project_id: project.id,
            content: "Entry 1".to_string(),
            tags: None,
        })
        .unwrap();
        db.create_journal_entry(CreateJournalEntryDTO {
            project_id: project.id,
            content: "Entry 2".to_string(),
            tags: None,
        })
        .unwrap();
        let entries = db.get_journal_entries(project.id).unwrap();
        assert_eq!(entries.len(), 2);
        // Both entries present (order may vary when created in same second)
        let contents: Vec<&str> = entries.iter().map(|e| e.content.as_str()).collect();
        assert!(contents.contains(&"Entry 1"));
        assert!(contents.contains(&"Entry 2"));
    }

    #[test]
    fn test_update_journal_entry() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let entry = db
            .create_journal_entry(CreateJournalEntryDTO {
                project_id: project.id,
                content: "Original".to_string(),
                tags: None,
            })
            .unwrap();
        let updated = db
            .update_journal_entry(
                entry.id,
                UpdateJournalEntryDTO {
                    content: Some("Updated".to_string()),
                    tags: None,
                },
            )
            .unwrap();
        assert_eq!(updated.content, "Updated");
    }

    #[test]
    fn test_delete_journal_entry() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let entry = db
            .create_journal_entry(CreateJournalEntryDTO {
                project_id: project.id,
                content: "To delete".to_string(),
                tags: None,
            })
            .unwrap();
        db.delete_journal_entry(entry.id).unwrap();
        let entries = db.get_journal_entries(project.id).unwrap();
        assert!(entries.is_empty());
    }

    // --- Todos ---

    #[test]
    fn test_create_todo() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let todo = db
            .create_todo(CreateTodoDTO {
                project_id: project.id,
                content: "Do something".to_string(),
            })
            .unwrap();
        assert!(todo.id > 0);
        assert!(!todo.is_completed);
        assert!(todo.completed_at.is_none());
    }

    #[test]
    fn test_complete_todo() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let todo = db
            .create_todo(CreateTodoDTO {
                project_id: project.id,
                content: "Task".to_string(),
            })
            .unwrap();
        let completed = db
            .update_todo(
                todo.id,
                UpdateTodoDTO {
                    content: None,
                    is_completed: Some(true),
                },
            )
            .unwrap();
        assert!(completed.is_completed);
        assert!(completed.completed_at.is_some());
    }

    #[test]
    fn test_uncomplete_todo() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let todo = db
            .create_todo(CreateTodoDTO {
                project_id: project.id,
                content: "Task".to_string(),
            })
            .unwrap();
        db.update_todo(
            todo.id,
            UpdateTodoDTO {
                content: None,
                is_completed: Some(true),
            },
        )
        .unwrap();
        let uncompleted = db
            .update_todo(
                todo.id,
                UpdateTodoDTO {
                    content: None,
                    is_completed: Some(false),
                },
            )
            .unwrap();
        assert!(!uncompleted.is_completed);
        assert!(uncompleted.completed_at.is_none());
    }

    #[test]
    fn test_get_project_todos_ordering() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let t1 = db
            .create_todo(CreateTodoDTO {
                project_id: project.id,
                content: "Task A".to_string(),
            })
            .unwrap();
        db.create_todo(CreateTodoDTO {
            project_id: project.id,
            content: "Task B".to_string(),
        })
        .unwrap();
        // Complete Task A
        db.update_todo(
            t1.id,
            UpdateTodoDTO {
                content: None,
                is_completed: Some(true),
            },
        )
        .unwrap();
        let todos = db.get_project_todos(project.id).unwrap();
        assert_eq!(todos.len(), 2);
        // Incomplete first
        assert!(!todos[0].is_completed);
        assert!(todos[1].is_completed);
    }

    #[test]
    fn test_delete_todo() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let todo = db
            .create_todo(CreateTodoDTO {
                project_id: project.id,
                content: "To delete".to_string(),
            })
            .unwrap();
        db.delete_todo(todo.id).unwrap();
        let todos = db.get_project_todos(project.id).unwrap();
        assert!(todos.is_empty());
    }

    // --- Attachments ---

    #[test]
    fn test_add_attachment() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let att = db
            .add_attachment(CreateAttachmentDTO {
                project_id: project.id,
                filename: "test.txt".to_string(),
                file_data: "aGVsbG8=".to_string(),
                file_size: 5,
                mime_type: "text/plain".to_string(),
            })
            .unwrap();
        assert!(att.id > 0);
        assert_eq!(att.filename, "test.txt");
        assert_eq!(att.file_size, 5);
    }

    #[test]
    fn test_attachment_size_limit() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let result = db.add_attachment(CreateAttachmentDTO {
            project_id: project.id,
            filename: "big.bin".to_string(),
            file_data: "data".to_string(),
            file_size: 6 * 1024 * 1024, // 6MB > 5MB limit
            mime_type: "application/octet-stream".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_get_attachments() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        db.add_attachment(CreateAttachmentDTO {
            project_id: project.id,
            filename: "a.txt".to_string(),
            file_data: "YQ==".to_string(),
            file_size: 1,
            mime_type: "text/plain".to_string(),
        })
        .unwrap();
        db.add_attachment(CreateAttachmentDTO {
            project_id: project.id,
            filename: "b.txt".to_string(),
            file_data: "Yg==".to_string(),
            file_size: 1,
            mime_type: "text/plain".to_string(),
        })
        .unwrap();
        let attachments = db.get_attachments(project.id).unwrap();
        assert_eq!(attachments.len(), 2);
    }

    #[test]
    fn test_delete_attachment() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let att = db
            .add_attachment(CreateAttachmentDTO {
                project_id: project.id,
                filename: "del.txt".to_string(),
                file_data: "ZA==".to_string(),
                file_size: 1,
                mime_type: "text/plain".to_string(),
            })
            .unwrap();
        db.delete_attachment(att.id).unwrap();
        let attachments = db.get_attachments(project.id).unwrap();
        assert!(attachments.is_empty());
    }

    #[test]
    fn test_attachment_at_exact_limit() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let result = db.add_attachment(CreateAttachmentDTO {
            project_id: project.id,
            filename: "exact.bin".to_string(),
            file_data: "data".to_string(),
            file_size: 5 * 1024 * 1024, // Exactly 5MB
            mime_type: "application/octet-stream".to_string(),
        });
        assert!(result.is_ok());
    }

    // ==================== Paso 4: Analytics, Status, Groups, Dashboard, Time Tracking ====================

    // --- Analytics ---

    #[test]
    fn test_track_project_open() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        db.track_project_open(project.id).unwrap();
        let fetched = db.get_project(project.id).unwrap();
        assert!(fetched.last_opened_at.is_some());
        assert_eq!(fetched.opened_count, Some(1));
        // Open again
        db.track_project_open(project.id).unwrap();
        let fetched2 = db.get_project(project.id).unwrap();
        assert_eq!(fetched2.opened_count, Some(2));
    }

    #[test]
    fn test_add_project_time() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        db.add_project_time(project.id, 60).unwrap();
        db.add_project_time(project.id, 120).unwrap();
        let fetched = db.get_project(project.id).unwrap();
        assert_eq!(fetched.total_time_seconds, Some(180));
    }

    #[test]
    fn test_get_project_stats() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        db.track_project_open(p.id).unwrap();
        let stats = db.get_project_stats().unwrap();
        assert_eq!(stats.total_projects, 1);
        assert!(!stats.recent_activities.is_empty());
    }

    // --- Status/Pin ---

    #[test]
    fn test_update_project_status() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        db.update_project_status(p.id, "pausado".to_string()).unwrap();
        let fetched = db.get_project(p.id).unwrap();
        assert_eq!(fetched.status.as_deref(), Some("pausado"));
    }

    #[test]
    fn test_toggle_pin_project() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        let pinned = db.toggle_pin_project(p.id).unwrap();
        assert!(pinned);
        let unpinned = db.toggle_pin_project(p.id).unwrap();
        assert!(!unpinned);
    }

    #[test]
    fn test_pin_assigns_incrementing_order() {
        let db = test_db();
        let p1 = db.create_project(test_project_dto()).unwrap();
        let mut dto2 = test_project_dto();
        dto2.name = "Project 2".to_string();
        let p2 = db.create_project(dto2).unwrap();
        db.toggle_pin_project(p1.id).unwrap();
        db.toggle_pin_project(p2.id).unwrap();
        let f1 = db.get_project(p1.id).unwrap();
        let f2 = db.get_project(p2.id).unwrap();
        assert_eq!(f1.pinned_order, Some(1));
        assert_eq!(f2.pinned_order, Some(2));
    }

    #[test]
    fn test_reorder_pinned_projects() {
        let db = test_db();
        let mut ids = vec![];
        for i in 0..3 {
            let mut dto = test_project_dto();
            dto.name = format!("P{}", i);
            let p = db.create_project(dto).unwrap();
            db.toggle_pin_project(p.id).unwrap();
            ids.push(p.id);
        }
        // Reorder: [3, 1, 2]
        db.reorder_pinned_projects(vec![ids[2], ids[0], ids[1]]).unwrap();
        let f0 = db.get_project(ids[2]).unwrap();
        let f1 = db.get_project(ids[0]).unwrap();
        let f2 = db.get_project(ids[1]).unwrap();
        assert_eq!(f0.pinned_order, Some(1));
        assert_eq!(f1.pinned_order, Some(2));
        assert_eq!(f2.pinned_order, Some(3));
    }

    /// B8: un id fantasma en la lista ABORTA todo el reordenamiento y no deja ni un
    /// `pinned_order` escrito. Sin la transacción, los ids anteriores al fantasma ya
    /// habrían quedado renumerados y el orden terminaba inconsistente.
    #[test]
    fn test_reorder_pinned_projects_aborts_on_unknown_id_without_partial_write() {
        let db = test_db();
        let mut ids = vec![];
        for i in 0..3 {
            let mut dto = test_project_dto();
            dto.name = format!("P{}", i);
            let p = db.create_project(dto).unwrap();
            db.toggle_pin_project(p.id).unwrap();
            ids.push(p.id);
        }
        let orden_previo: Vec<Option<i64>> = ids
            .iter()
            .map(|id| db.get_project(*id).unwrap().pinned_order)
            .collect();

        // El fantasma va ÚLTIMO a propósito: los dos primeros UPDATE ya corrieron y
        // tienen que revertirse. Si el test pusiera el fantasma primero, pasaría en
        // verde incluso sin transacción y no probaría nada.
        let err = db
            .reorder_pinned_projects(vec![ids[2], ids[0], 999_999])
            .expect_err("un id inexistente debe abortar el reordenamiento");
        assert!(
            matches!(err, rusqlite::Error::QueryReturnedNoRows),
            "debe ser QueryReturnedNoRows, fue: {err:?}"
        );

        let orden_posterior: Vec<Option<i64>> = ids
            .iter()
            .map(|id| db.get_project(*id).unwrap().pinned_order)
            .collect();
        assert_eq!(
            orden_previo, orden_posterior,
            "el rollback debe dejar el pinned_order exactamente como estaba"
        );
    }

    // --- Groups ---

    #[test]
    fn test_get_root_projects() {
        let db = test_db();
        let parent = db.create_project(test_project_dto()).unwrap();
        let mut child_dto = test_project_dto();
        child_dto.name = "Child".to_string();
        child_dto.parent_id = Some(parent.id);
        db.create_project(child_dto).unwrap();
        let roots = db.get_root_projects().unwrap();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].name, "Test Project");
    }

    #[test]
    fn test_get_subprojects() {
        let db = test_db();
        let parent = db.create_project(test_project_dto()).unwrap();
        for i in 0..2 {
            let mut dto = test_project_dto();
            dto.name = format!("Child {}", i);
            dto.parent_id = Some(parent.id);
            db.create_project(dto).unwrap();
        }
        let children = db.get_subprojects(parent.id).unwrap();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_get_project_with_children() {
        let db = test_db();
        let parent = db.create_project(test_project_dto()).unwrap();
        let mut dto = test_project_dto();
        dto.name = "Child 1".to_string();
        dto.parent_id = Some(parent.id);
        db.create_project(dto).unwrap();
        let result = db.get_project_with_children(parent.id).unwrap();
        assert_eq!(result.subproject_count, 1);
        assert_eq!(result.children.len(), 1);
    }

    #[test]
    fn test_assign_project_to_group() {
        let db = test_db();
        let group = db.create_project(test_project_dto()).unwrap();
        let mut child_dto = test_project_dto();
        child_dto.name = "Standalone".to_string();
        let child = db.create_project(child_dto).unwrap();
        // Move into group
        db.assign_project_to_group(child.id, Some(group.id)).unwrap();
        let subs = db.get_subprojects(group.id).unwrap();
        assert_eq!(subs.len(), 1);
        // Move back to root
        db.assign_project_to_group(child.id, None).unwrap();
        let subs2 = db.get_subprojects(group.id).unwrap();
        assert!(subs2.is_empty());
    }

    #[test]
    fn test_assign_rejects_self_parent() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        let err = db.assign_project_to_group(p.id, Some(p.id)).unwrap_err();
        assert!(err.contains("propio grupo padre"), "msg inesperado: {}", err);
        // No se escribió: sigue siendo raíz
        assert_eq!(db.get_project(p.id).unwrap().parent_id, None);
    }

    #[test]
    fn test_assign_rejects_nonexistent_parent() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        let err = db.assign_project_to_group(p.id, Some(99999)).unwrap_err();
        assert!(err.contains("no existe"), "msg inesperado: {}", err);
    }

    #[test]
    fn test_assign_rejects_cycle() {
        // a -> b (b es hijo de a). Intentar a.parent = b cerraría un ciclo.
        let db = test_db();
        let a = db.create_project(test_project_dto()).unwrap();
        let mut b_dto = test_project_dto();
        b_dto.name = "B".to_string();
        let b = db.create_project(b_dto).unwrap();
        db.assign_project_to_group(b.id, Some(a.id)).unwrap(); // b hijo de a (válido)

        // Ahora mover a dentro de b debe rechazarse (a es ancestro de b)
        let err = db.assign_project_to_group(a.id, Some(b.id)).unwrap_err();
        assert!(err.contains("ciclo"), "msg inesperado: {}", err);
        // a sigue siendo raíz, no se corrompió la jerarquía
        assert_eq!(db.get_project(a.id).unwrap().parent_id, None);
    }

    #[test]
    fn test_update_project_does_not_change_parent_id() {
        // Regresión: parent_id ya NO viaja por update_project (ruta cerrada).
        let db = test_db();
        let group = db.create_project(test_project_dto()).unwrap();
        let mut child_dto = test_project_dto();
        child_dto.name = "Child".to_string();
        let child = db.create_project(child_dto).unwrap();
        db.assign_project_to_group(child.id, Some(group.id)).unwrap();

        // Intentar cambiar parent_id por update_project debe ser IGNORADO
        db.update_project(
            child.id,
            UpdateProjectDTO {
                name: Some("Renamed".to_string()),
                description: None,
                local_path: None,
                documentation_url: None,
                ai_documentation_url: None,
                drive_link: None,
                notes: None,
                image_data: None,
                parent_id: None, // aunque fuera Some, update_project lo ignora
                group_color: None,
                group_icon: None,
            },
        )
        .unwrap();

        // El grupo se mantiene (lo gobierna assign_project_to_group, no update)
        assert_eq!(db.get_project(child.id).unwrap().parent_id, Some(group.id));
        assert_eq!(db.get_project(child.id).unwrap().name, "Renamed");
    }

    // --- Dashboard ---

    #[test]
    fn test_get_recent_projects() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        // Before tracking, no recent projects
        let recent = db.get_recent_projects().unwrap();
        assert!(recent.is_empty());
        // Track open
        db.track_project_open(p.id).unwrap();
        let recent2 = db.get_recent_projects().unwrap();
        assert_eq!(recent2.len(), 1);
        assert_eq!(recent2[0].name, "Test Project");
    }

    #[test]
    fn test_get_all_pending_todos() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        db.create_todo(CreateTodoDTO {
            project_id: p.id,
            content: "Pending task".to_string(),
        })
        .unwrap();
        let completed = db
            .create_todo(CreateTodoDTO {
                project_id: p.id,
                content: "Done task".to_string(),
            })
            .unwrap();
        db.update_todo(
            completed.id,
            UpdateTodoDTO {
                content: None,
                is_completed: Some(true),
            },
        )
        .unwrap();
        let pending = db.get_all_pending_todos().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].todo.content, "Pending task");
        assert_eq!(pending[0].project_name, "Test Project");
    }

    // --- Time Tracking DB ---

    #[test]
    fn test_create_and_end_tracking_session() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        let session_id = db.create_tracking_session(p.id, "test").unwrap();
        assert!(session_id > 0);
        db.end_tracking_session(session_id, 300).unwrap();
        let sessions = db.get_tracking_sessions(p.id, 10).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].duration_seconds, Some(300));
        // Verify total_time updated on project
        let fetched = db.get_project(p.id).unwrap();
        assert_eq!(fetched.total_time_seconds, Some(300));
    }

    /// B8: cerrar una sesión inexistente no debe acreditarle tiempo a NINGÚN proyecto.
    /// El segundo UPDATE resuelve su `WHERE id = (SELECT project_id ...)` a NULL y no
    /// matchea nada; este test lo fija para que un refactor del subquery no empiece a
    /// repartir tiempo fantasma.
    #[test]
    fn test_end_tracking_session_with_unknown_id_credits_no_project() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        let session_id = db.create_tracking_session(p.id, "test").unwrap();
        db.end_tracking_session(session_id, 300).unwrap();
        assert_eq!(db.get_project(p.id).unwrap().total_time_seconds, Some(300));

        // Sesión que no existe: no explota y no toca el total.
        db.end_tracking_session(999_999, 5_000)
            .expect("cerrar una sesión inexistente no debe ser un error duro");

        assert_eq!(
            db.get_project(p.id).unwrap().total_time_seconds,
            Some(300),
            "una sesión inexistente no debe sumar tiempo a ningún proyecto"
        );
        assert_eq!(db.get_tracking_sessions(p.id, 10).unwrap().len(), 1);
    }

    #[test]
    fn test_get_time_stats() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        let s1 = db.create_tracking_session(p.id, "test").unwrap();
        db.end_tracking_session(s1, 600).unwrap();
        let s2 = db.create_tracking_session(p.id, "test").unwrap();
        db.end_tracking_session(s2, 300).unwrap();
        let stats = db.get_time_stats(p.id).unwrap();
        assert_eq!(stats.total_seconds, 900);
        assert_eq!(stats.session_count, 2);
        assert_eq!(stats.avg_session_seconds, 450);
        assert_eq!(stats.longest_session_seconds, 600);
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
