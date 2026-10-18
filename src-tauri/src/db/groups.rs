use rusqlite::{params, Connection, Result};

use super::common::{row_to_project, PROJECT_COLUMNS};
use crate::models::project::{Project, ProjectWithChildren};

use super::Database;

impl Database {
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
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use crate::models::project::UpdateProjectDTO;

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
}
