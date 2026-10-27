use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result, TransactionBehavior};

use super::ajustes::{con_ajustes, suma_de_ajustes};
use super::tiempo::{segundos_en, union_del_proyecto, SesionActiva};
use super::trabajar::{MotivoTramo, FUENTE_TERMINAL};
use super::Database;
use crate::time::{self, RangoUtc};

/// El cierre de [`Database::cerrar_sesion`], sobre la conexión o transacción que le
/// pasen: `end_tracking_session` y los cierres de `db/trabajar.rs` lo necesitan
/// dentro de la suya.
///
/// Un tramo `terminal` de 0 s se **borra** en vez de cerrarse (DT5): no usa la regla
/// de los 10 s y no aporta nada. `motivo` es `None` solo para el envoltorio con
/// "ahora", que cierra filas sin token como antes del paso 4; el `CHECK` de la columna
/// rechaza un tramo de sesión cerrado sin motivo.
pub(super) fn cerrar_sesion_en(
    conn: &Connection,
    session_id: i64,
    ended_at: DateTime<Utc>,
    duration_seconds: i64,
    motivo: Option<MotivoTramo>,
) -> Result<bool> {
    if duration_seconds == 0 {
        let borradas = conn.execute(
            "DELETE FROM time_tracking_sessions
             WHERE id = ?1 AND ended_at IS NULL AND source = ?2",
            params![session_id, FUENTE_TERMINAL],
        )?;
        if borradas > 0 {
            return Ok(true);
        }
    }

    let cerradas = conn.execute(
        "UPDATE time_tracking_sessions
         SET ended_at = ?1, duration_seconds = ?2, motivo_cierre = ?3
         WHERE id = ?4 AND ended_at IS NULL",
        params![
            time::formatear(ended_at),
            duration_seconds,
            motivo.map(MotivoTramo::como_texto),
            session_id
        ],
    )?;
    Ok(cerradas > 0)
}

/// El latido de [`Database::latir`], sobre la conexión o transacción que le pasen: el
/// latido de Trabajar lo necesita dentro de la suya, junto al control del tope.
pub(super) fn latir_en(
    conn: &Connection,
    session_id: i64,
    last_activity_at: DateTime<Utc>,
    duration_seconds: Option<i64>,
) -> Result<bool> {
    let latidas = conn.execute(
        "UPDATE time_tracking_sessions
         SET last_activity_at = ?1, duration_seconds = COALESCE(?2, duration_seconds)
         WHERE id = ?3 AND ended_at IS NULL",
        params![
            time::formatear(last_activity_at),
            duration_seconds,
            session_id
        ],
    )?;

    Ok(latidas > 0)
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
        let instante = time::formatear(started_at);

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
    #[allow(dead_code)] // El latido de Trabajar usa `latir_en` en su transacción; sin llamadores hasta el demonio
    pub fn latir(
        &self,
        session_id: i64,
        last_activity_at: DateTime<Utc>,
        duration_seconds: Option<i64>,
    ) -> Result<bool> {
        latir_en(&self.conn(), session_id, last_activity_at, duration_seconds)
    }

    /// Cierra una sesión abierta: `ended_at`, `duration_seconds` y el motivo juntos, en
    /// un solo `UPDATE` (M7). Desde el paso 4 todo cierre lleva motivo (3.10.15): la
    /// firma lo exige. Devuelve `false` si ya estaba cerrada o no existe. Un tramo
    /// `terminal` de 0 s se borra (DT5) y cuenta como cerrado.
    #[allow(dead_code)] // Los cierres de la app usan `cerrar_sesion_en`; sin llamadores hasta el demonio
    pub fn cerrar_sesion(
        &self,
        session_id: i64,
        ended_at: DateTime<Utc>,
        duration_seconds: i64,
        motivo: MotivoTramo,
    ) -> Result<bool> {
        cerrar_sesion_en(
            &self.conn(),
            session_id,
            ended_at,
            duration_seconds,
            Some(motivo),
        )
    }

    /// Crear una nueva sesión de tracking que empieza ahora. Envoltorio de
    /// [`Database::abrir_sesion`] para los llamadores que todavía no pasan el instante.
    #[allow(dead_code)] // Desde el commit 17 Trabajar abre sus tramos con el token; quedan los tests
    pub fn create_tracking_session(&self, project_id: i64, source: &str) -> Result<i64> {
        self.abrir_sesion(project_id, source, Utc::now())
    }

    /// Terminar una sesión de tracking ahora.
    ///
    /// Idempotente (P2/B3): sólo cierra una sesión que siga abierta. Devuelve `true` si
    /// cerró la sesión y `false` si no había nada que cerrar (ya estaba cerrada o el id
    /// no existe).
    ///
    /// Envoltorio de [`Database::cerrar_sesion`] con "ahora". Cierra sin motivo, así
    /// que solo sirve para filas sin token: desde el commit 17 Trabajar cierra sus
    /// tramos con motivo y este envoltorio queda sin llamadores fuera de los tests.
    /// Desde el commit 25 no suma a ningún contador: el total sale de la unión de
    /// tramos (P1, D2).
    ///
    /// La transacción es `IMMEDIATE` (R8-1): toma el lock de escritura antes de leer.
    /// En una diferida que lee antes de escribir, la conexión puede quedar con el lock
    /// de lectura esperando el de escritura de otra, y SQLite le devuelve
    /// `SQLITE_BUSY` sin esperar el `busy_timeout`. Desde el commit 17 la llaman
    /// cierres que compiten con otra conexión (el demonio, la CLI).
    #[allow(dead_code)] // Sin llamadores desde el commit 17: Trabajar cierra con motivo
    pub fn end_tracking_session(&self, session_id: i64, duration_seconds: i64) -> Result<bool> {
        let mut conn = self.conn();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if !cerrar_sesion_en(&tx, session_id, Utc::now(), duration_seconds, None)? {
            // Nada que cerrar: soltar `tx` sin commit es un rollback.
            return Ok(false);
        }

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

    /// Estadísticas de tiempo de un proyecto (3.3, 9a de la auditoría).
    ///
    /// - `total_seconds`, `today_seconds` y `week_seconds`: `tiempo_total` de su rango,
    ///   la **unión** de los tramos (`tiempo_trabajado`) recortada al rango, con la
    ///   `activa`, más los ajustes de ese rango, recortado en 0 (3.10.8, commit 22).
    ///   `hoy` y `semana` son días locales ya convertidos a UTC por el llamador
    ///   (`time::rango_dia` y `time::rango_semana`): la semana son 7 fechas, incluido
    ///   hoy.
    /// - `session_count`, `avg_session_seconds` y `longest_session_seconds`: sobre las
    ///   sesiones **individuales** cerradas con duración > 0, sin unir y sin ajustes.
    ///   Describen sesiones medidas, no el total.
    ///
    /// Un error de base se propaga (D6, P18): antes se mostraba como 0 s.
    pub fn get_time_stats(
        &self,
        project_id: i64,
        hoy: RangoUtc,
        semana: RangoUtc,
        activa: Option<SesionActiva>,
    ) -> Result<crate::tracking::aggregator::TimeStats> {
        use crate::tracking::aggregator::TimeStats;

        let conn = self.conn();

        // Una sola consulta: la unión de siempre sirve para los tres rangos.
        let unidos = union_del_proyecto(&conn, project_id, &time::rango_total(), activa)?;
        let total_de = |rango: &RangoUtc| -> Result<i64> {
            Ok(con_ajustes(
                segundos_en(&unidos, rango),
                suma_de_ajustes(&conn, project_id, rango)?,
            ))
        };

        let (session_count, suma, longest_session_seconds): (i64, i64, i64) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(duration_seconds), 0), COALESCE(MAX(duration_seconds), 0)
             FROM time_tracking_sessions
             WHERE project_id = ?1 AND ended_at IS NOT NULL AND duration_seconds > 0",
            params![project_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let avg_session_seconds = if session_count > 0 {
            suma / session_count
        } else {
            0
        };

        Ok(TimeStats {
            total_seconds: total_de(&time::rango_total())?,
            session_count,
            avg_session_seconds,
            longest_session_seconds,
            today_seconds: total_de(&hoy)?,
            week_seconds: total_de(&semana)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use rusqlite::params;

    use super::super::test_support::*;
    use super::super::tiempo::SesionActiva;
    use super::super::trabajar::MotivoTramo;
    use crate::db::Database;
    use crate::time::{self, RangoUtc};
    use chrono_tz::America::Argentina::Buenos_Aires;

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
    }

    /// B8: cerrar una sesión inexistente no debe acreditarle tiempo a NINGÚN proyecto
    /// ni tocar las sesiones que existen.
    #[test]
    fn test_end_tracking_session_with_unknown_id_credits_no_project() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        let session_id = db.create_tracking_session(p.id, "test").unwrap();
        db.end_tracking_session(session_id, 300).unwrap();

        // Sesión que no existe: no explota y no toca el total.
        let cerro = db
            .end_tracking_session(999_999, 5_000)
            .expect("cerrar una sesión inexistente no debe ser un error duro");
        assert!(!cerro, "una sesión inexistente no se cierra");

        let sesiones = db.get_tracking_sessions(p.id, 10).unwrap();
        assert_eq!(sesiones.len(), 1);
        assert_eq!(
            sesiones[0].duration_seconds,
            Some(300),
            "una sesión inexistente no debe sumar tiempo a ningún proyecto"
        );
    }

    /// P2/B3: cerrar dos veces la misma sesión suma UNA vez. El segundo cierre (una
    /// carrera entre dos caminos que cierran) devuelve `false` y no toca ni la duración
    /// ni el `ended_at` de la sesión.
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

        let despues = db.get_tracking_sessions(p.id, 10).unwrap();
        assert_eq!(despues.len(), 1);
        assert_eq!(despues[0].duration_seconds, Some(300));
        assert_eq!(despues[0].ended_at, antes[0].ended_at);
    }

    // --- 9a: estadísticas sobre la unión en hora local (commit 21) ---

    /// Rangos de "hoy" y "semana" de Buenos Aires para el 2026-10-20.
    fn rangos_del_20() -> (RangoUtc, RangoUtc) {
        let hoy = chrono::NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        (
            time::rango_dia(&Buenos_Aires, hoy),
            time::rango_semana(&Buenos_Aires, hoy),
        )
    }

    fn stats(db: &Database, project_id: i64) -> crate::tracking::aggregator::TimeStats {
        let (hoy, semana) = rangos_del_20();
        db.get_time_stats(project_id, hoy, semana, None).unwrap()
    }

    fn sesion_cerrada(db: &Database, project_id: i64, source: &str, desde: &str, segundos: i64) {
        let inicio = instante(desde);
        db.conn()
            .execute(
                "INSERT INTO time_tracking_sessions
                     (project_id, source, started_at, ended_at, duration_seconds)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    project_id,
                    source,
                    time::formatear(inicio),
                    time::formatear(inicio + chrono::Duration::seconds(segundos)),
                    segundos
                ],
            )
            .unwrap();
    }

    /// Sin superposición, los números son los del SQL de antes (copiado acá). Las
    /// correcciones documentadas no aparecen en estos datos: sesiones abiertas, de 0 s
    /// o superpuestas, y "hoy" y "semana" en hora local.
    #[test]
    fn sin_superposicion_da_los_mismos_numeros_que_el_sql_de_antes() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap().id;
        sesion_cerrada(&db, p, "work_button", "2026-10-01 10:00:00", 600);
        sesion_cerrada(&db, p, "file_watch", "2026-10-01 12:00:00", 300);
        sesion_cerrada(&db, p, "terminal", "2026-10-02 09:00:00", 5400);

        let (total, cantidad): (i64, i64) = db
            .conn()
            .query_row(
                "SELECT COALESCE(SUM(duration_seconds), 0), COUNT(*)
                 FROM time_tracking_sessions
                 WHERE project_id = ?1 AND duration_seconds IS NOT NULL",
                params![p],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let mas_larga: i64 = db
            .conn()
            .query_row(
                "SELECT COALESCE(MAX(duration_seconds), 0)
                 FROM time_tracking_sessions WHERE project_id = ?1",
                params![p],
                |row| row.get(0),
            )
            .unwrap();

        let nuevas = stats(&db, p);
        assert_eq!(nuevas.total_seconds, total);
        assert_eq!(nuevas.total_seconds, 6300);
        assert_eq!(nuevas.session_count, cantidad);
        assert_eq!(nuevas.avg_session_seconds, total / cantidad);
        assert_eq!(nuevas.longest_session_seconds, mas_larga);
    }

    /// Con superposición, el total no cuenta dos veces el mismo instante. Las sesiones
    /// individuales siguen describiendo sesiones: no se unen.
    #[test]
    fn con_superposicion_no_hay_doble_conteo() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap().id;
        sesion_cerrada(&db, p, "work_button", "2026-10-20 13:00:00", 7200);
        sesion_cerrada(&db, p, "file_watch", "2026-10-20 14:00:00", 7200);

        let s = stats(&db, p);
        assert_eq!(s.total_seconds, 3 * 3600, "no 4 h");
        assert_eq!(s.today_seconds, 3 * 3600);
        assert_eq!(s.session_count, 2);
        assert_eq!(s.avg_session_seconds, 7200);
        assert_eq!(s.longest_session_seconds, 7200);
    }

    /// Las sesiones de 0 s y las abiertas no cuentan como sesiones (corrección
    /// documentada: antes una cerrada de 0 s sumaba al conteo y bajaba el promedio).
    #[test]
    fn session_count_solo_cuenta_cerradas_con_duracion() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap().id;
        sesion_cerrada(&db, p, "work_button", "2026-10-20 13:00:00", 600);
        sesion_cerrada(&db, p, "work_button", "2026-10-20 14:00:00", 0);
        db.abrir_sesion(p, "work_button", instante("2026-10-20 15:00:00"))
            .unwrap();

        let s = stats(&db, p);
        assert_eq!(s.session_count, 1);
        assert_eq!(s.avg_session_seconds, 600);
        assert_eq!(s.total_seconds, 600);
    }

    /// La semana son 7 fechas locales incluido hoy: hace 6 días entra, hace 7 no (antes
    /// entraba: eran 8 fechas, P3).
    #[test]
    fn semana_son_7_fechas_locales() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap().id;
        // 2026-10-14 12:00 en Buenos Aires: hace 6 días.
        sesion_cerrada(&db, p, "terminal", "2026-10-14 15:00:00", 600);
        // 2026-10-13 12:00 en Buenos Aires: hace 7 días.
        sesion_cerrada(&db, p, "terminal", "2026-10-13 15:00:00", 300);

        let s = stats(&db, p);
        assert_eq!(s.week_seconds, 600);
        assert_eq!(s.today_seconds, 0);
        assert_eq!(s.total_seconds, 900);
    }

    /// "Hoy" es el día local: a las 22:00 de Buenos Aires (01:00 UTC del día
    /// siguiente) lo trabajado a la tarde sigue siendo de hoy.
    #[test]
    fn hoy_es_el_dia_local() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap().id;
        // 21:00 a 21:30 de Buenos Aires = 00:00 a 00:30 UTC del 21.
        sesion_cerrada(&db, p, "terminal", "2026-10-21 00:00:00", 1800);
        // 23:30 de Buenos Aires del 19 = 02:30 UTC del 20: es de ayer.
        sesion_cerrada(&db, p, "terminal", "2026-10-20 02:30:00", 600);

        assert_eq!(stats(&db, p).today_seconds, 1800);
    }

    /// La `activa` entra en los tres totales con los segundos del `Instant`.
    #[test]
    fn la_activa_entra_en_los_totales() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap().id;
        let viva = db
            .abrir_sesion(p, "work_button", instante("2026-10-20 15:00:00"))
            .unwrap();
        let (hoy, semana) = rangos_del_20();
        let activa = Some(SesionActiva {
            session_id: viva,
            segundos: 900,
        });

        let s = db.get_time_stats(p, hoy, semana, activa).unwrap();
        assert_eq!(
            (s.total_seconds, s.today_seconds, s.week_seconds),
            (900, 900, 900)
        );
        assert_eq!(s.session_count, 0, "abierta: todavía no es una sesión");
    }

    /// Un error de base sube en vez de mostrarse como 0 s (P18, D6).
    #[test]
    fn tabla_borrada_es_un_error_y_no_0() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap().id;
        db.conn()
            .execute_batch("DROP TABLE time_tracking_sessions")
            .unwrap();
        let (hoy, semana) = rangos_del_20();
        assert!(db.get_time_stats(p, hoy, semana, None).is_err());
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
            .cerrar_sesion(
                id,
                instante("2026-10-20 09:30:00"),
                1800,
                MotivoTramo::Inactividad
            )
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
            .cerrar_sesion(
                id,
                instante("2026-10-20 09:30:00"),
                1800,
                MotivoTramo::Inactividad
            )
            .unwrap());
        let (_, _, ended_at, duracion) = fila_sesion(&db, id);
        assert_eq!(ended_at.as_deref(), Some("2026-10-20 09:30:00"));
        assert_eq!(duracion, Some(1800));

        assert!(!db
            .cerrar_sesion(
                id,
                instante("2026-10-20 11:00:00"),
                7200,
                MotivoTramo::Inactividad
            )
            .unwrap());
        let (_, _, ended_at, duracion) = fila_sesion(&db, id);
        assert_eq!(ended_at.as_deref(), Some("2026-10-20 09:30:00"));
        assert_eq!(duracion, Some(1800));

        assert!(!db
            .cerrar_sesion(
                999_999,
                instante("2026-10-20 11:00:00"),
                7200,
                MotivoTramo::Inactividad
            )
            .unwrap());
    }

    /// R8-1: `end_tracking_session` en `IMMEDIATE`. Dos conexiones cierran la misma
    /// sesión a la vez (Parar contra el cierre de otra conexión): una sola cierra,
    /// ninguna recibe `SQLITE_BUSY` y cada sesión queda con la duración del cierre que
    /// ganó. Fija el contrato
    /// para cuando el cierre lea antes de escribir (commit 17): con una transacción
    /// diferida, ese patrón puede devolver `SQLITE_BUSY` sin esperar el
    /// `busy_timeout`.
    #[test]
    fn end_tracking_session_con_dos_conexiones_cierra_una_vez_y_sin_busy() {
        use std::sync::{Arc, Barrier};

        let dir = tempfile::TempDir::new().unwrap();
        let ruta = dir.path().join("projects.db");
        let una = Arc::new(Database::new(ruta.clone()).unwrap());
        let otra = Arc::new(Database::new(ruta).unwrap());
        let p = una.create_project(test_project_dto()).unwrap();

        for vuelta in 0..30 {
            let id = una.create_tracking_session(p.id, "work_button").unwrap();
            let largada = Arc::new(Barrier::new(2));
            let hilos: Vec<_> = [Arc::clone(&una), Arc::clone(&otra)]
                .into_iter()
                .map(|db| {
                    let largada = Arc::clone(&largada);
                    std::thread::spawn(move || {
                        largada.wait();
                        db.end_tracking_session(id, 100)
                    })
                })
                .collect();
            let cerraron: Vec<bool> = hilos
                .into_iter()
                .map(|h| h.join().unwrap().expect("sin SQLITE_BUSY"))
                .collect();
            assert_eq!(
                cerraron.iter().filter(|c| **c).count(),
                1,
                "vuelta {vuelta}: {cerraron:?}"
            );
        }

        let sesiones = una.get_tracking_sessions(p.id, 100).unwrap();
        assert_eq!(sesiones.len(), 30);
        assert!(sesiones.iter().all(|s| s.duration_seconds == Some(100)));
    }

    /// `cerrar_sesion` guarda el motivo del tramo (3.10.15).
    #[test]
    fn cerrar_sesion_guarda_el_motivo() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        let id = db
            .abrir_sesion(p.id, "file_watch", instante("2026-10-20 09:00:00"))
            .unwrap();
        db.cerrar_sesion(
            id,
            instante("2026-10-20 09:30:00"),
            1800,
            MotivoTramo::SinVigilancia,
        )
        .unwrap();
        let motivo: Option<String> = db
            .conn()
            .query_row(
                "SELECT motivo_cierre FROM time_tracking_sessions WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(motivo.as_deref(), Some("sin_vigilancia"));
    }
}
