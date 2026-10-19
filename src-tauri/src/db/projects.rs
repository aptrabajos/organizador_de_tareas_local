use log::{debug, error};
use rusqlite::{params, Result};

use super::common::{row_to_project, PROJECT_COLUMNS};
use crate::models::project::{CreateProjectDTO, Project, UpdateProjectDTO};

use super::Database;

impl Database {
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
            &format!(
                "SELECT {}
             FROM projects WHERE id = ?1",
                PROJECT_COLUMNS
            ),
            params![id],
            row_to_project,
        )?;

        Ok(project)
    }

    pub fn get_all_projects(&self) -> Result<Vec<Project>> {
        let conn = self.conn();

        let mut stmt = conn.prepare(&format!(
            "SELECT {}
             FROM projects
             WHERE deleted_at IS NULL
             ORDER BY display_order ASC, is_pinned DESC, pinned_order ASC, updated_at DESC",
            PROJECT_COLUMNS
        ))?;

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
                project.links = Some(links_by_project.remove(&project.id).unwrap_or_default());
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
        // `conn()` espera el lock y se recupera del envenenamiento, como el resto de
        // la capa (P16). Antes era un `try_lock` que devolvía `SQLITE_BUSY` al instante
        // si el Mutex estaba ocupado, justificado como "posible cicatriz de deadlock por
        // reentrancia" sin que nadie documentara el caso. No hay reentrancia: el guard
        // se le pasa a `get_project_links_internal`, `update_project` lo suelta antes
        // de llamar acá y `get_project_with_children` llama sin tenerlo. La espera
        // queda acotada por la consulta de quien tenga el lock.
        let conn = self.conn();

        let mut project = conn.query_row(
            &format!(
                "SELECT {}
             FROM projects WHERE id = ?1 AND deleted_at IS NULL",
                PROJECT_COLUMNS
            ),
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
        debug!(
            "🗄️ [DB] Iniciando update_project en base de datos para ID: {}",
            id
        );

        // Mismo criterio que `get_project`: esperar el lock en vez de fallar.
        let conn = self.conn();

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
                debug!(
                    "🗄️ [DB] UPDATE ejecutado exitosamente, filas afectadas: {}",
                    rows_affected
                );
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
        let mut stmt = conn.prepare("SELECT local_path FROM projects WHERE deleted_at IS NULL")?;
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
                project.links = Some(links_by_project.remove(&project.id).unwrap_or_default());
                project
            })
            .collect();

        Ok(projects)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use crate::models::project::{CreateProjectDTO, UpdateProjectDTO};

    // ============ P16: get_project/update_project esperan el lock ============

    fn solo_nombre(name: &str) -> UpdateProjectDTO {
        UpdateProjectDTO {
            name: Some(name.to_string()),
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
        }
    }

    /// Con `try_lock`, un lock tomado por otro hilo hacía fallar `get_project` al
    /// instante con `SQLITE_BUSY` ("Trabajar" lo llama primero). Ahora espera.
    #[test]
    fn test_get_project_waits_for_held_lock() {
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();
        let (tomado_tx, tomado_rx) = std::sync::mpsc::channel();

        std::thread::scope(|scope| {
            scope.spawn(|| {
                let _guard = db.conn.lock().unwrap();
                tomado_tx.send(()).unwrap();
                std::thread::sleep(std::time::Duration::from_millis(200));
            });
            // Recién cuando el otro hilo tiene el lock.
            tomado_rx.recv().unwrap();

            let inicio = std::time::Instant::now();
            let fetched = db
                .get_project(created.id)
                .expect("get_project debe esperar el lock, no fallar");
            assert_eq!(fetched.id, created.id);
            assert!(
                inicio.elapsed() >= std::time::Duration::from_millis(100),
                "get_project tenía que esperar a que se soltara el lock"
            );

            let updated = db
                .update_project(created.id, solo_nombre("Renombrado"))
                .expect("update_project debe esperar el lock, no fallar");
            assert_eq!(updated.name, "Renombrado");
        });
    }

    /// `try_lock` tampoco se recuperaba del envenenamiento; `conn()` sí (B15).
    #[test]
    fn test_get_project_and_update_project_survive_poisoned_mutex() {
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();

        let panicked = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let _guard = db.conn.lock().expect("el mutex arranca sano");
                    panic!("panic deliberado con el lock de la conexión tomado");
                })
                .join()
        });
        assert!(panicked.is_err(), "el hilo tenía que entrar en panic");

        db.get_project(created.id)
            .expect("get_project tiene que funcionar con el mutex envenenado");
        let updated = db
            .update_project(created.id, solo_nombre("Tras el panic"))
            .expect("update_project tiene que funcionar con el mutex envenenado");
        assert_eq!(updated.name, "Tras el panic");
    }

    // ==================== B4: get_project filtra la papelera ====================

    #[test]
    fn test_get_project_returns_active_project() {
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();

        let fetched = db
            .get_project(created.id)
            .expect("un proyecto activo debe leerse");
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
        assert!(!db
            .is_registered_project_path("/home/user/alpha/sub")
            .unwrap());
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

    // ==================== Paso 1: Inicialización y CRUD de Proyectos ====================

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
        assert_eq!(
            project.documentation_url.as_deref(),
            Some("https://docs.example.com")
        );
        assert_eq!(
            project.drive_link.as_deref(),
            Some("https://drive.google.com/xxx")
        );
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
}
