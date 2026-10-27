use std::collections::HashSet;

use rusqlite::{params, Result};

use super::ajustes::{con_ajustes, sumas_de_ajustes_por_proyecto};
use super::tiempo::{segundos_en, uniones_por_proyecto, SesionActiva};
use super::Database;
use crate::time::{self, RangoUtc};

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

    /// Estadísticas globales del Dashboard (3.3, 9a de la auditoría).
    ///
    /// - `total_time_hours`: la suma del `tiempo_total` de cada proyecto fuera de la
    ///   papelera: su **unión**, con la `activa`, más sus ajustes, recortado en 0 por
    ///   proyecto (3.10.8, commit 22). Una consulta para todos los tramos y otra para
    ///   todos los ajustes.
    /// - `active_today`: los proyectos con actividad en `project_activity` **o** con
    ///   tiempo trabajado en el día local `hoy` (ya convertido a UTC por el llamador).
    ///   Lo segundo es nuevo: con el demonio, un proyecto editado con la app cerrada
    ///   también estuvo activo.
    ///
    /// Un error de base se propaga (D6, P18): antes se mostraba como 0.
    pub fn get_project_stats(
        &self,
        hoy: RangoUtc,
        activa: Option<SesionActiva>,
    ) -> Result<crate::models::project::ProjectStats> {
        use crate::models::project::{ProjectActivity, ProjectStats};

        let conn = self.conn();

        // Total de proyectos
        let total_projects: i64 = conn.query_row(
            "SELECT COUNT(*) FROM projects WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )?;

        // La unión de siempre de cada proyecto sirve para el total y para "hoy".
        let uniones = uniones_por_proyecto(&conn, &time::rango_total(), activa)?;

        // Proyectos activos hoy: con actividad registrada o con tiempo trabajado.
        let mut activos: HashSet<i64> = uniones
            .iter()
            .filter(|(_, unidos)| segundos_en(unidos, &hoy) > 0)
            .map(|(project_id, _)| *project_id)
            .collect();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT project_id FROM project_activity
             WHERE created_at >= ?1 AND created_at < ?2",
        )?;
        for project_id in stmt.query_map(params![hoy.desde_texto(), hoy.hasta_texto()], |row| {
            row.get::<_, i64>(0)
        })? {
            activos.insert(project_id?);
        }
        let active_today = i64::try_from(activos.len()).unwrap_or(i64::MAX);

        // Tiempo total en horas: lo medido más los ajustes, proyecto por proyecto (uno
        // puede tener solo ajustes).
        let ajustes = sumas_de_ajustes_por_proyecto(&conn, &time::rango_total())?;
        let con_tiempo: HashSet<i64> = uniones.keys().chain(ajustes.keys()).copied().collect();
        let total_seconds: i64 = con_tiempo
            .iter()
            .map(|project_id| {
                con_ajustes(
                    uniones
                        .get(project_id)
                        .map_or(0, |unidos| segundos_en(unidos, &time::rango_total())),
                    ajustes.get(project_id).copied().unwrap_or(0),
                )
            })
            .sum();
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
    use super::*;
    use chrono::Utc;
    use chrono_tz::America::Argentina::Buenos_Aires;

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

    fn hoy_en_buenos_aires() -> RangoUtc {
        let hoy = chrono::NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        time::rango_dia(&Buenos_Aires, hoy)
    }

    fn sesion(db: &Database, project_id: i64, desde: &str, hasta: &str, segundos: i64) {
        db.conn()
            .execute(
                "INSERT INTO time_tracking_sessions
                     (project_id, source, started_at, ended_at, duration_seconds)
                 VALUES (?1, 'terminal', ?2, ?3, ?4)",
                params![project_id, desde, hasta, segundos],
            )
            .unwrap();
    }

    #[test]
    fn test_get_project_stats() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        db.track_project_open(p.id).unwrap();
        let stats = db
            .get_project_stats(time::rango_dia(&Utc, Utc::now().date_naive()), None)
            .unwrap();
        assert_eq!(stats.total_projects, 1);
        assert_eq!(stats.active_today, 1, "abierto hoy");
        assert!(!stats.recent_activities.is_empty());
    }

    /// El total es la unión de cada proyecto fuera de la papelera, no el contador:
    /// sin superposición da la suma de las sesiones y con superposición no cuenta dos
    /// veces.
    #[test]
    fn total_time_hours_suma_la_union_sin_la_papelera() {
        let db = test_db();
        let a = seed_project_at(&db, "A", "/tmp/a");
        let b = seed_project_at(&db, "B", "/tmp/b");
        let borrado = seed_project_at(&db, "Borrado", "/tmp/borrado");
        sesion(&db, a, "2026-10-01 10:00:00", "2026-10-01 11:00:00", 3600);
        sesion(&db, a, "2026-10-01 10:30:00", "2026-10-01 11:30:00", 3600);
        sesion(&db, b, "2026-10-01 10:00:00", "2026-10-01 10:30:00", 1800);
        sesion(
            &db,
            borrado,
            "2026-10-01 10:00:00",
            "2026-10-01 20:00:00",
            36000,
        );
        db.conn()
            .execute(
                "UPDATE projects SET deleted_at = '2026-10-02 12:00:00' WHERE id = ?1",
                params![borrado],
            )
            .unwrap();
        // El contador deprecado ya no se lee.
        db.add_project_time(a, 99_999).unwrap();

        let stats = db.get_project_stats(hoy_en_buenos_aires(), None).unwrap();
        assert_eq!(stats.total_time_hours, 2.0);
        assert_eq!(stats.total_projects, 2);
    }

    /// `active_today` en hora local: a las 22:00 de Buenos Aires (01:00 UTC del 21) lo
    /// de la tarde sigue siendo "hoy". Cuenta la actividad registrada y, como novedad,
    /// el tiempo trabajado; un proyecto con las dos cosas cuenta una vez.
    #[test]
    fn active_today_en_hora_local_con_actividad_o_tiempo() {
        let db = test_db();
        let con_actividad = seed_project_at(&db, "Actividad", "/tmp/actividad");
        let con_tiempo = seed_project_at(&db, "Tiempo", "/tmp/tiempo");
        let con_las_dos = seed_project_at(&db, "Las dos", "/tmp/las-dos");
        let de_ayer = seed_project_at(&db, "Ayer", "/tmp/ayer");
        for (project_id, cuando) in [
            (con_actividad, "2026-10-20 18:00:00"),
            (con_las_dos, "2026-10-20 18:00:00"),
            // 23:00 de Buenos Aires del 19.
            (de_ayer, "2026-10-20 02:00:00"),
        ] {
            db.conn()
                .execute(
                    "INSERT INTO project_activity (project_id, activity_type, created_at)
                     VALUES (?1, 'opened', ?2)",
                    params![project_id, cuando],
                )
                .unwrap();
        }
        // 21:00 a 21:30 de Buenos Aires = 00:00 a 00:30 UTC del 21.
        sesion(
            &db,
            con_tiempo,
            "2026-10-21 00:00:00",
            "2026-10-21 00:30:00",
            1800,
        );
        sesion(
            &db,
            con_las_dos,
            "2026-10-20 15:00:00",
            "2026-10-20 16:00:00",
            3600,
        );
        sesion(
            &db,
            de_ayer,
            "2026-10-20 01:00:00",
            "2026-10-20 02:00:00",
            3600,
        );

        let stats = db.get_project_stats(hoy_en_buenos_aires(), None).unwrap();
        assert_eq!(stats.active_today, 3);
    }

    /// Un error de base sube en vez de mostrarse como 0 (P18, D6).
    #[test]
    fn tabla_borrada_es_un_error_y_no_0() {
        let db = test_db();
        seed_project_at(&db, "A", "/tmp/a");
        db.conn()
            .execute_batch("DROP TABLE project_activity")
            .unwrap();
        assert!(db.get_project_stats(hoy_en_buenos_aires(), None).is_err());

        let db = test_db();
        db.conn()
            .execute_batch("DROP TABLE time_tracking_sessions")
            .unwrap();
        assert!(db.get_project_stats(hoy_en_buenos_aires(), None).is_err());
    }
}
