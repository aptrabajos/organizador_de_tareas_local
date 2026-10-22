use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};

use super::Database;

/// Un instante como se guarda en las sesiones: UTC con el formato de
/// `CURRENT_TIMESTAMP` (`%Y-%m-%d %H:%M:%S`). Los filtros por rango comparan texto,
/// así que mezclar formatos los rompería sin aviso.
///
/// Es local a este módulo hasta que exista el formateador único de `time.rs`
/// (commit 11 del diseño del tracker), que lo reemplaza.
fn instante_sql(instante: DateTime<Utc>) -> String {
    instante.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// El `UPDATE` de [`Database::cerrar_sesion`], sobre la conexión o transacción que
/// le pasen: `end_tracking_session` lo necesita dentro de la suya.
fn cerrar_sesion_en(
    conn: &Connection,
    session_id: i64,
    ended_at: DateTime<Utc>,
    duration_seconds: i64,
) -> Result<bool> {
    let cerradas = conn.execute(
        "UPDATE time_tracking_sessions
         SET ended_at = ?1, duration_seconds = ?2
         WHERE id = ?3 AND ended_at IS NULL",
        params![instante_sql(ended_at), duration_seconds, session_id],
    )?;
    Ok(cerradas > 0)
}

impl Database {
    // ==================== TIME TRACKING METHODS ====================

    // --- Sesiones con instante explícito (D14) ---
    //
    // Quien abre, late o cierra una sesión dice CUÁNDO: nadie más escribe
    // `CURRENT_TIMESTAMP` en las sesiones. El demonio cierra en el último cambio real,
    // no en "ahora", y los tests fijan el reloj.

    /// Abre una sesión en `started_at`. Escribe también `last_activity_at` con el
    /// mismo instante, para que una sesión huérfana nunca quede sin latido (M7).
    pub fn abrir_sesion(
        &self,
        project_id: i64,
        source: &str,
        started_at: DateTime<Utc>,
    ) -> Result<i64> {
        let conn = self.conn();
        let instante = instante_sql(started_at);

        conn.execute(
            "INSERT INTO time_tracking_sessions (project_id, started_at, last_activity_at, source)
             VALUES (?1, ?2, ?2, ?3)",
            params![project_id, instante, source],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Latido de una sesión abierta: mueve `last_activity_at` y, si viene, deja en
    /// `duration_seconds` lo medido hasta ahora (el latido de Trabajar, que mide con
    /// `Instant`). Con `None` la duración no se toca (el latido de `file_watch`).
    ///
    /// Devuelve `false` si no tocó ninguna fila: la sesión ya estaba cerrada o no
    /// existe (por ejemplo, el proyecto se purgó). Quien late la suelta.
    #[allow(dead_code)] // Sin llamadores hasta el latido de Trabajar (commit 19) y el demonio
    pub fn latir(
        &self,
        session_id: i64,
        last_activity_at: DateTime<Utc>,
        duration_seconds: Option<i64>,
    ) -> Result<bool> {
        let conn = self.conn();

        let latidas = conn.execute(
            "UPDATE time_tracking_sessions
             SET last_activity_at = ?1, duration_seconds = COALESCE(?2, duration_seconds)
             WHERE id = ?3 AND ended_at IS NULL",
            params![instante_sql(last_activity_at), duration_seconds, session_id],
        )?;

        Ok(latidas > 0)
    }

    /// Cierra una sesión abierta: `ended_at` y `duration_seconds` juntos, en un solo
    /// `UPDATE` (M7). Devuelve `false` si ya estaba cerrada o no existe.
    ///
    /// No toca `projects.total_time_seconds`: el contador está deprecado y solo lo
    /// sigue sumando `end_tracking_session` hasta que desaparezca.
    #[allow(dead_code)] // Sin llamadores hasta la recuperación de huérfanas (commit 19) y el demonio
    pub fn cerrar_sesion(
        &self,
        session_id: i64,
        ended_at: DateTime<Utc>,
        duration_seconds: i64,
    ) -> Result<bool> {
        cerrar_sesion_en(&self.conn(), session_id, ended_at, duration_seconds)
    }

    /// Crear una nueva sesión de tracking que empieza ahora. Envoltorio de
    /// [`Database::abrir_sesion`] para los llamadores que todavía no pasan el instante.
    pub fn create_tracking_session(&self, project_id: i64, source: &str) -> Result<i64> {
        self.abrir_sesion(project_id, source, Utc::now())
    }

    /// Terminar una sesión de tracking: cerrarla y acreditar su duración al proyecto.
    ///
    /// Los dos UPDATE van en UNA transacción. Antes eran dos `execute` sueltos bajo el
    /// mismo lock: si el segundo fallaba, la sesión quedaba cerrada y el tiempo
    /// trabajado se perdía en silencio, sin error y sin forma de recuperarlo.
    ///
    /// Idempotente (P2/B3): sólo cierra una sesión que siga abierta. Si el primer
    /// UPDATE no afecta ninguna fila (ya estaba cerrada o el id no existe), el del
    /// contador NO corre: filtrar sólo el primero no alcanzaba, porque el segundo
    /// sumaba igual y cerrar dos veces acreditaba dos veces. Devuelve `true` si cerró
    /// la sesión y `false` si no había nada que cerrar.
    ///
    /// Envoltorio de [`Database::cerrar_sesion`] con "ahora", más el contador.
    pub fn end_tracking_session(&self, session_id: i64, duration_seconds: i64) -> Result<bool> {
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;

        if !cerrar_sesion_en(&tx, session_id, Utc::now(), duration_seconds)? {
            // Nada que cerrar: soltar `tx` sin commit es un rollback.
            return Ok(false);
        }

        // También actualizar el tiempo total del proyecto
        tx.execute(
            "UPDATE projects
             SET total_time_seconds = COALESCE(total_time_seconds, 0) + ?1
             WHERE id = (SELECT project_id FROM time_tracking_sessions WHERE id = ?2)",
            params![duration_seconds, session_id],
        )?;

        tx.commit()?;
        Ok(true)
    }

    /// Obtener sesiones de tracking de un proyecto
    pub fn get_tracking_sessions(
        &self,
        project_id: i64,
        limit: i64,
    ) -> Result<Vec<crate::tracking::aggregator::TimeTrackingSession>> {
        use crate::tracking::aggregator::TimeTrackingSession;

        let conn = self.conn();

        let mut stmt = conn.prepare(
            "SELECT id, project_id, started_at, ended_at, duration_seconds, source
             FROM time_tracking_sessions
             WHERE project_id = ?1
             ORDER BY started_at DESC
             LIMIT ?2",
        )?;

        let sessions = stmt
            .query_map(params![project_id, limit], |row| {
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
    pub fn get_time_stats(
        &self,
        project_id: i64,
    ) -> Result<crate::tracking::aggregator::TimeStats> {
        use crate::tracking::aggregator::TimeStats;

        let conn = self.conn();

        // Tiempo total y número de sesiones
        let (total_seconds, session_count): (i64, i64) = conn
            .query_row(
                "SELECT COALESCE(SUM(duration_seconds), 0), COUNT(*)
             FROM time_tracking_sessions
             WHERE project_id = ?1 AND duration_seconds IS NOT NULL",
                params![project_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or((0, 0));

        // Promedio de duración
        let avg_session_seconds = if session_count > 0 {
            total_seconds / session_count
        } else {
            0
        };

        // Sesión más larga
        let longest_session_seconds: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(duration_seconds), 0)
             FROM time_tracking_sessions
             WHERE project_id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Tiempo hoy
        let today_seconds: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(duration_seconds), 0)
             FROM time_tracking_sessions
             WHERE project_id = ?1 AND DATE(started_at) = DATE('now')",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Tiempo esta semana (últimos 7 días)
        let week_seconds: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(duration_seconds), 0)
             FROM time_tracking_sessions
             WHERE project_id = ?1 AND started_at >= DATE('now', '-7 days')",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

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
    use chrono::{DateTime, Utc};
    use rusqlite::params;

    use super::super::test_support::*;
    use crate::db::Database;

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
        let cerro = db
            .end_tracking_session(999_999, 5_000)
            .expect("cerrar una sesión inexistente no debe ser un error duro");
        assert!(!cerro, "una sesión inexistente no se cierra");

        assert_eq!(
            db.get_project(p.id).unwrap().total_time_seconds,
            Some(300),
            "una sesión inexistente no debe sumar tiempo a ningún proyecto"
        );
        assert_eq!(db.get_tracking_sessions(p.id, 10).unwrap().len(), 1);
    }

    /// P2/B3: cerrar dos veces la misma sesión suma UNA vez. El segundo cierre (una
    /// carrera entre dos caminos que cierran) devuelve `false` y no toca ni el
    /// contador del proyecto ni la duración ni el `ended_at` de la sesión.
    #[test]
    fn test_end_tracking_session_twice_credits_once() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        let session_id = db.create_tracking_session(p.id, "test").unwrap();

        assert!(db.end_tracking_session(session_id, 300).unwrap());
        let antes = db.get_tracking_sessions(p.id, 10).unwrap();

        assert!(
            !db.end_tracking_session(session_id, 900).unwrap(),
            "el segundo cierre no debe cerrar nada"
        );

        assert_eq!(db.get_project(p.id).unwrap().total_time_seconds, Some(300));
        let despues = db.get_tracking_sessions(p.id, 10).unwrap();
        assert_eq!(despues.len(), 1);
        assert_eq!(despues[0].duration_seconds, Some(300));
        assert_eq!(despues[0].ended_at, antes[0].ended_at);
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

    // --- D14/M7: sesiones con instante explícito ---

    fn instante(texto: &str) -> DateTime<Utc> {
        chrono::NaiveDateTime::parse_from_str(texto, "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc()
    }

    /// (started_at, last_activity_at, ended_at, duration_seconds) de una sesión.
    fn fila_sesion(
        db: &Database,
        session_id: i64,
    ) -> (String, Option<String>, Option<String>, Option<i64>) {
        db.conn()
            .query_row(
                "SELECT started_at, last_activity_at, ended_at, duration_seconds
                 FROM time_tracking_sessions WHERE id = ?1",
                params![session_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap()
    }

    #[test]
    fn abrir_sesion_escribe_el_latido_igual_al_inicio() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();

        let id = db
            .abrir_sesion(p.id, "file_watch", instante("2026-10-20 09:00:00"))
            .unwrap();

        let (started_at, last_activity_at, ended_at, duracion) = fila_sesion(&db, id);
        assert_eq!(started_at, "2026-10-20 09:00:00");
        assert_eq!(last_activity_at.as_deref(), Some("2026-10-20 09:00:00"));
        assert_eq!(ended_at, None);
        assert_eq!(duracion, None);
    }

    /// El envoltorio con "ahora" también escribe el latido, en el formato de
    /// `CURRENT_TIMESTAMP`.
    #[test]
    fn create_tracking_session_escribe_el_latido_igual_al_inicio() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();

        let id = db.create_tracking_session(p.id, "work_button").unwrap();

        let (started_at, last_activity_at, _, _) = fila_sesion(&db, id);
        assert_eq!(last_activity_at.as_deref(), Some(started_at.as_str()));
        let ahora: String = db
            .conn()
            .query_row("SELECT CURRENT_TIMESTAMP", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            started_at.len(),
            ahora.len(),
            "mismo formato que CURRENT_TIMESTAMP"
        );
    }

    #[test]
    fn latir_mueve_el_latido_y_solo_toca_la_duracion_si_viene() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        let id = db
            .abrir_sesion(p.id, "work_button", instante("2026-10-20 09:00:00"))
            .unwrap();

        assert!(db
            .latir(id, instante("2026-10-20 09:01:00"), Some(60))
            .unwrap());
        let (_, latido, _, duracion) = fila_sesion(&db, id);
        assert_eq!(latido.as_deref(), Some("2026-10-20 09:01:00"));
        assert_eq!(duracion, Some(60));

        assert!(db.latir(id, instante("2026-10-20 09:02:00"), None).unwrap());
        let (_, latido, ended_at, duracion) = fila_sesion(&db, id);
        assert_eq!(latido.as_deref(), Some("2026-10-20 09:02:00"));
        assert_eq!(duracion, Some(60), "sin duración, el latido no la toca");
        assert_eq!(ended_at, None, "latir no cierra");
    }

    /// Una sesión cerrada o inexistente (proyecto purgado) no late: `false`, y la
    /// fila cerrada no cambia.
    #[test]
    fn latir_sobre_una_sesion_cerrada_o_inexistente_devuelve_false() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        let id = db
            .abrir_sesion(p.id, "file_watch", instante("2026-10-20 09:00:00"))
            .unwrap();
        assert!(db
            .cerrar_sesion(id, instante("2026-10-20 09:30:00"), 1800)
            .unwrap());
        let cerrada = fila_sesion(&db, id);

        assert!(!db
            .latir(id, instante("2026-10-20 10:00:00"), Some(3600))
            .unwrap());
        assert_eq!(fila_sesion(&db, id), cerrada);

        assert!(!db
            .latir(999_999, instante("2026-10-20 10:00:00"), None)
            .unwrap());
    }

    #[test]
    fn cerrar_sesion_dos_veces_devuelve_false_y_no_toca_la_fila() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        let id = db
            .abrir_sesion(p.id, "file_watch", instante("2026-10-20 09:00:00"))
            .unwrap();

        assert!(db
            .cerrar_sesion(id, instante("2026-10-20 09:30:00"), 1800)
            .unwrap());
        let (_, _, ended_at, duracion) = fila_sesion(&db, id);
        assert_eq!(ended_at.as_deref(), Some("2026-10-20 09:30:00"));
        assert_eq!(duracion, Some(1800));

        assert!(!db
            .cerrar_sesion(id, instante("2026-10-20 11:00:00"), 7200)
            .unwrap());
        let (_, _, ended_at, duracion) = fila_sesion(&db, id);
        assert_eq!(ended_at.as_deref(), Some("2026-10-20 09:30:00"));
        assert_eq!(duracion, Some(1800));

        assert!(!db
            .cerrar_sesion(999_999, instante("2026-10-20 11:00:00"), 7200)
            .unwrap());
    }

    /// `cerrar_sesion` no suma al contador deprecado; eso lo sigue haciendo solo el
    /// envoltorio `end_tracking_session`.
    #[test]
    fn cerrar_sesion_no_suma_al_contador_del_proyecto() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        let id = db
            .abrir_sesion(p.id, "file_watch", instante("2026-10-20 09:00:00"))
            .unwrap();
        db.cerrar_sesion(id, instante("2026-10-20 09:30:00"), 1800)
            .unwrap();
        assert_eq!(db.get_project(p.id).unwrap().total_time_seconds, Some(0));
    }
}
