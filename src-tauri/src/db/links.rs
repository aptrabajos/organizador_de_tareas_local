use rusqlite::{params, Connection, Result};
use std::collections::HashMap;

use crate::models::project::{CreateLinkDTO, ProjectLink, UpdateLinkDTO};

use super::Database;

impl Database {
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
    pub(super) fn get_links_for_projects(
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

    pub(super) fn get_project_links_internal(&self, project_id: i64, conn: &Connection) -> Result<Vec<ProjectLink>> {
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
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use crate::models::project::{CreateLinkDTO, CreateProjectDTO, UpdateLinkDTO};

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
}
