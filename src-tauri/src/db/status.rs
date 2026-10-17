use rusqlite::{params, Result};

use super::Database;

impl Database {
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
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;

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
}
