use rusqlite::{params, Result};

use super::Database;

impl Database {
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
        use crate::models::project::{ProjectActivity, ProjectStats};

        let conn = self.conn();

        // Total de proyectos
        let total_projects: i64 = conn.query_row(
            "SELECT COUNT(*) FROM projects WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )?;

        // Proyectos activos hoy
        let active_today: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT project_id) FROM project_activity
             WHERE DATE(created_at) = DATE('now')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

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
             LIMIT 20",
        )?;

        let activities = stmt
            .query_map([], |row| {
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

    pub fn get_project_activities(
        &self,
        project_id: i64,
        limit: i64,
    ) -> Result<Vec<crate::models::project::ProjectActivity>> {
        use crate::models::project::ProjectActivity;

        let conn = self.conn();

        let mut stmt = conn.prepare(
            "SELECT id, project_id, activity_type, description, duration_seconds, created_at
             FROM project_activity
             WHERE project_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )?;

        let activities = stmt
            .query_map(params![project_id, limit], |row| {
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
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;

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
}
