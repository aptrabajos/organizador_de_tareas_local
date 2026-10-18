use log::error;
use rusqlite::{params, Connection, Result};
use std::sync::Mutex;

use crate::models::project::{Project, ProjectWithChildren};

mod common;
mod schema;
mod maintenance;
mod attachments;
mod journal;
mod todos;
mod activity;
mod status;
mod tracking;
mod links;
mod projects;

#[cfg(test)]
mod test_support;

use common::{row_to_project, PROJECT_COLUMNS};

/// Item de la papelera. Struct dedicado (no toca el modelo Project) que SÍ lleva la
/// fecha de borrado real y el conteo de subproyectos también en papelera.
#[derive(serde::Serialize)]
pub struct TrashItem {
    pub id: i64,
    pub name: String,
    pub deleted_at: String,
    pub subproject_count: i64,
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
}

#[cfg(test)]
mod tests {
    use crate::models::project::*;
    use super::test_support::*;

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

    // ==================== Paso 3: Journal, Todos, Attachments ====================

    // ==================== Paso 4: Analytics, Status, Groups, Dashboard, Time Tracking ====================

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
}
