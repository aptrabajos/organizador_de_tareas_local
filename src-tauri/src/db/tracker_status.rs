use rusqlite::{OptionalExtension, Result};

use super::Database;

/// La fila única (`id = 1`) de `tracker_status`: el estado que publica el demonio.
///
/// Solo la escribe el demonio; la app la lee para mostrar el estado, nunca para
/// calcular tiempo. Los nombres siguen a las columnas (3.2 del diseño del tracker).
#[allow(dead_code)] // `estado_trabajar` lee la fila y el estado; el resto, la UI de F7
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackerStatus {
    /// `NULL` cuando el demonio salió limpio.
    pub pid: Option<i64>,
    pub version: Option<String>,
    pub started_at: Option<String>,
    /// El `ESQUEMA_ACTUAL` que conoce el binario del demonio.
    pub schema_version: Option<i64>,
    /// `activo`, `desactivado`, `esperando_migracion` o `esquema_nuevo`.
    pub estado: String,
    pub last_heartbeat: Option<String>,
    pub idle_minutes: Option<i64>,
    pub min_credit_seconds: Option<i64>,
    pub watched_projects: Option<i64>,
    pub watched_dirs: Option<i64>,
    /// Fin del recorrido inicial; `NULL` mientras recorre.
    pub scan_completed_at: Option<String>,
    /// JSON: `[{"id": 30, "motivo": "carpeta_amplia"}, …]`.
    pub unwatched_projects: Option<String>,
    pub last_error: Option<String>,
    pub last_error_at: Option<String>,
    /// Paso 4: la pausa efectiva de Trabajar, en minutos.
    pub trabajar_idle_minutes: Option<i64>,
    /// Paso 4: `kde`, `gnome`, `prueba` o `no_disponible`.
    pub presencia: Option<String>,
    pub presencia_motivo: Option<String>,
    pub presencia_at: Option<String>,
}

impl Database {
    /// Lee el estado que publicó el demonio. `None` significa que el demonio nunca
    /// corrió contra esta base (la tabla existe desde el paso 3, pero sin fila).
    pub fn leer_tracker_status(&self) -> Result<Option<TrackerStatus>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT pid, version, started_at, schema_version, estado, last_heartbeat,
                    idle_minutes, min_credit_seconds, watched_projects, watched_dirs,
                    scan_completed_at, unwatched_projects, last_error, last_error_at,
                    trabajar_idle_minutes, presencia, presencia_motivo, presencia_at
             FROM tracker_status
             WHERE id = 1",
            [],
            |row| {
                Ok(TrackerStatus {
                    pid: row.get(0)?,
                    version: row.get(1)?,
                    started_at: row.get(2)?,
                    schema_version: row.get(3)?,
                    estado: row.get(4)?,
                    last_heartbeat: row.get(5)?,
                    idle_minutes: row.get(6)?,
                    min_credit_seconds: row.get(7)?,
                    watched_projects: row.get(8)?,
                    watched_dirs: row.get(9)?,
                    scan_completed_at: row.get(10)?,
                    unwatched_projects: row.get(11)?,
                    last_error: row.get(12)?,
                    last_error_at: row.get(13)?,
                    trabajar_idle_minutes: row.get(14)?,
                    presencia: row.get(15)?,
                    presencia_motivo: row.get(16)?,
                    presencia_at: row.get(17)?,
                })
            },
        )
        .optional()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;

    #[test]
    fn tracker_status_existe_despues_de_migrar() {
        let db = test_db();
        assert!(objetos_del_esquema(&db.conn()).contains(&"table tracker_status".to_string()));
    }

    #[test]
    fn tracker_status_tiene_las_columnas_esperadas() {
        let db = test_db();
        assert_eq!(
            columnas(&db.conn(), "tracker_status"),
            vec![
                "id",
                "pid",
                "version",
                "started_at",
                "schema_version",
                "estado",
                "last_heartbeat",
                "idle_minutes",
                "min_credit_seconds",
                "watched_projects",
                "watched_dirs",
                "scan_completed_at",
                "unwatched_projects",
                "last_error",
                "last_error_at",
                "trabajar_idle_minutes",
                "presencia",
                "presencia_motivo",
                "presencia_at",
            ]
        );
    }

    /// Sin fila, el demonio nunca corrió: no es un error.
    #[test]
    fn leer_tracker_status_sin_fila_devuelve_none() {
        let db = test_db();
        assert_eq!(db.leer_tracker_status().unwrap(), None);
    }

    #[test]
    fn leer_tracker_status_devuelve_la_fila_del_demonio() {
        let db = test_db();
        db.execute_batch_para_tests(
            "INSERT INTO tracker_status (id, pid, version, started_at, schema_version, estado,
                                         last_heartbeat, idle_minutes, min_credit_seconds,
                                         watched_projects, watched_dirs, unwatched_projects)
             VALUES (1, 4242, '0.7.0', '2026-10-20 09:00:00', 3, 'activo',
                     '2026-10-20 09:00:15', 3, 60, 12, 340, '[{\"id\": 30, \"motivo\": \"carpeta_amplia\"}]')",
        );

        let estado = db
            .leer_tracker_status()
            .unwrap()
            .expect("con fila tiene que devolver Some");
        assert_eq!(estado.pid, Some(4242));
        assert_eq!(estado.version.as_deref(), Some("0.7.0"));
        assert_eq!(estado.estado, "activo");
        assert_eq!(
            estado.last_heartbeat.as_deref(),
            Some("2026-10-20 09:00:15")
        );
        assert_eq!(estado.watched_dirs, Some(340));
        assert_eq!(estado.scan_completed_at, None, "todavía recorre");
        assert_eq!(estado.last_error, None);
        assert_eq!(estado.presencia, None, "el paso 4 no inventa la presencia");
    }

    /// Una sola fila (`id = 1`) y solo los estados que define el diseño.
    #[test]
    fn tracker_status_rechaza_otra_fila_y_estados_desconocidos() {
        let db = test_db();
        let conn = db.conn();
        assert!(conn
            .execute(
                "INSERT INTO tracker_status (id, estado) VALUES (2, 'activo')",
                []
            )
            .is_err());
        assert!(conn
            .execute(
                "INSERT INTO tracker_status (id, estado) VALUES (1, 'cualquiera')",
                []
            )
            .is_err());
        assert!(conn
            .execute(
                "INSERT INTO tracker_status (id, estado) VALUES (1, 'esperando_migracion')",
                []
            )
            .is_ok());
    }

    /// Paso 4: la presencia solo toma los valores que publica el demonio (3.10.4).
    #[test]
    fn tracker_status_rechaza_una_presencia_desconocida() {
        let db = test_db();
        let conn = db.conn();
        conn.execute(
            "INSERT INTO tracker_status (id, estado) VALUES (1, 'activo')",
            [],
        )
        .unwrap();
        assert!(conn
            .execute("UPDATE tracker_status SET presencia = 'xfce'", [])
            .is_err());
        for valor in ["kde", "gnome", "prueba", "no_disponible"] {
            conn.execute("UPDATE tracker_status SET presencia = ?1", [valor])
                .unwrap();
        }
    }
}
