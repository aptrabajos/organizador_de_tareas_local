use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result, TransactionBehavior};

use super::trabajar::{cortar_por_papelera_en, MedidoPorLaApp, SesionCortada};
use super::Database;

/// Item de la papelera. Struct dedicado (no toca el modelo Project) que SÍ lleva la
/// fecha de borrado real y el conteo de subproyectos también en papelera.
#[derive(serde::Serialize)]
pub struct TrashItem {
    pub id: i64,
    pub name: String,
    pub deleted_at: String,
    pub subproject_count: i64,
}

impl Database {
    /// Soft-delete: mueve el proyecto a la papelera marcando deleted_at.
    /// Cascada: marca también los hijos directos AÚN activos con el MISMO
    /// timestamp (eso identifica el batch para restaurarlo en bloque).
    ///
    /// Un proyecto en la papelera no se mide (excepción a D-R8-1, 3.10.9 del diseño
    /// del tracker): en la **misma** transacción `IMMEDIATE` corta con
    /// `proyecto_a_papelera` las sesiones de Trabajar vivas de todos los proyectos que
    /// marca, en `fin`. `medida` es la sesión que mide la app, si mide alguna: su tramo
    /// se cierra con el `Instant`. Devuelve las sesiones cortadas, para el toast.
    pub fn delete_project(
        &self,
        id: i64,
        fin: DateTime<Utc>,
        medida: Option<MedidoPorLaApp>,
    ) -> Result<Vec<SesionCortada>> {
        let mut conn = self.conn();
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Soft-delete del proyecto + TODOS sus descendientes activos (cualquier
        // profundidad, no solo hijos directos) con el MISMO timestamp: así no quedan
        // nietos activos colgando de un padre borrado (invisibles), e identifica el
        // batch para restaurar/purgar en bloque. `AND deleted_at IS NULL` preserva
        // los que ya estaban en papelera de antes.
        let marcados: Vec<i64> = tx
            .prepare(
                "WITH RECURSIVE descendants(did) AS (
                     SELECT ?1
                     UNION ALL
                     SELECT p.id FROM projects p JOIN descendants ON p.parent_id = descendants.did
                 )
                 SELECT id FROM projects
                 WHERE deleted_at IS NULL AND id IN (SELECT did FROM descendants)",
            )?
            .query_map(params![id], |row| row.get(0))?
            .collect::<Result<_>>()?;
        let cortadas = cortar_por_papelera_en(&tx, &marcados, fin, medida)?;
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
        Ok(cortadas)
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
        tx.execute(
            "DELETE FROM project_links WHERE project_id = ?1",
            params![id],
        )?;
        tx.execute(
            "DELETE FROM project_attachments WHERE project_id = ?1",
            params![id],
        )?;
        tx.execute(
            "DELETE FROM project_journal WHERE project_id = ?1",
            params![id],
        )?;
        tx.execute(
            "DELETE FROM project_todos WHERE project_id = ?1",
            params![id],
        )?;
        tx.execute(
            "DELETE FROM project_activity WHERE project_id = ?1",
            params![id],
        )?;
        tx.execute(
            "DELETE FROM time_tracking_sessions WHERE project_id = ?1",
            params![id],
        )?;
        tx.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Vaciar la papelera: elimina DEFINITIVAMENTE todos los proyectos con
    /// deleted_at IS NOT NULL y sus datos, en una sola transacción.
    pub fn empty_trash(&self) -> Result<()> {
        let conn = self.conn();

        let ids: Vec<i64> = {
            let mut stmt = conn.prepare("SELECT id FROM projects WHERE deleted_at IS NOT NULL")?;
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
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;

    /// EL test de regresión que importa: el filtro NO rompió la restauración.
    /// `restore_project` no compone `get_project` —hace su propio SELECT y exige que el
    /// proyecto SÍ esté en papelera—, así que el camino de vuelta sigue intacto.
    #[test]
    fn test_restore_project_still_works_after_trash_filter() {
        let db = test_db();
        let created = db.create_project(test_project_dto()).unwrap();
        db.delete_project(created.id, chrono::Utc::now(), None)
            .unwrap();

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
        db.delete_project(created.id, chrono::Utc::now(), None)
            .unwrap();
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
        db.delete_project(created.id, chrono::Utc::now(), None)
            .unwrap();
        // No aparece en la lista principal ni en raíz...
        assert!(db
            .get_all_projects()
            .unwrap()
            .iter()
            .all(|p| p.id != created.id));
        assert!(db
            .get_root_projects()
            .unwrap()
            .iter()
            .all(|p| p.id != created.id));
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
        db.delete_project(created.id, chrono::Utc::now(), None)
            .unwrap();
        db.restore_project(created.id).unwrap();
        // Vuelve a la lista y sale de la papelera.
        assert!(db
            .get_all_projects()
            .unwrap()
            .iter()
            .any(|p| p.id == created.id));
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
        db.assign_project_to_group(child.id, Some(group.id))
            .unwrap();

        db.delete_project(group.id, chrono::Utc::now(), None)
            .unwrap();
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

        db.delete_project(a.id, chrono::Utc::now(), None).unwrap();
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
        db.delete_project(created.id, chrono::Utc::now(), None)
            .unwrap();
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

        db.delete_project(created.id, chrono::Utc::now(), None)
            .unwrap();
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

        db.delete_project(created.id, chrono::Utc::now(), None)
            .unwrap();
        db.empty_trash().unwrap();

        assert_eq!(
            child_row_counts(&db, created.id),
            (0, 0, 0, 0, 0, 0),
            "empty_trash debe dejar las 6 tablas hijas vacías para el proyecto purgado"
        );
        assert!(db.get_project(created.id).is_err());
        assert!(db.list_trash().unwrap().is_empty());
    }
}
