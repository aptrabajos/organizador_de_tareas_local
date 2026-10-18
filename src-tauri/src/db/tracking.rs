use rusqlite::{params, Result};

use super::Database;

impl Database {
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
    use super::super::test_support::*;

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
}
