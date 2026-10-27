//! Ajustes manuales de tiempo, solo de agregado (3.10.17 del diseño del tracker, DT13,
//! decisión 5a; commit 22).
//!
//! Las mediciones nunca se tocan: una corrección es un ajuste aparte, con signo y con
//! nota obligatoria, y un ajuste no se edita ni se borra, se revierte con otro. La base
//! lo garantiza con los triggers del paso 5 (`db/schema.rs`); acá van las reglas que la
//! base no puede expresar (la reversión exacta, el rato que no suma más de lo que dura)
//! y los mensajes en español para lo que el usuario puede corregir.
//!
//! Los totales que ve el usuario son `tiempo_total = max(0, unión de tramos + Σ
//! ajustes del rango)` (3.10.8). Un ajuste cae entero en el rango que contiene su
//! **fecha efectiva**, `COALESCE(desde, creado_en)`, sin prorratear. `tiempo_trabajado`
//! sigue siendo la unión pura: la usan el desglose y la simultaneidad, donde los ajustes
//! no entran.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use rusqlite::{ffi, params, Connection, OptionalExtension, Result, Row, TransactionBehavior};

use super::tiempo::{segundos_en, union_del_proyecto, SesionActiva};
use super::Database;
use crate::time::{self, RangoUtc};

/// Tope de un ajuste, en valor absoluto: 24 h (el `CHECK` de la columna).
const TOPE_AJUSTE_SEGUNDOS: i64 = 86_400;

const MENSAJE_SIN_NOTA: &str =
    "Escribí una nota: es lo que vas a mostrar si te preguntan por este ajuste.";
const MENSAJE_CERO: &str =
    "Un ajuste de 0 segundos no cambia nada. Poné cuánto querés sumar o restar.";
const MENSAJE_TOPE: &str = "Un ajuste no puede pasar de 24 h. Si necesitás más, cargá uno por día.";
const MENSAJE_HORARIO_INCOMPLETO: &str = "Si el ajuste tiene horario, poné el desde y el hasta.";
const MENSAJE_HORARIO_AL_REVES: &str =
    "El horario del ajuste tiene que terminar después de empezar. Revisá el desde y el hasta.";
const MENSAJE_REVERTIR_REVERSION: &str =
    "Ese ajuste ya es una reversión y no se revierte. Si hace falta, cargá un ajuste nuevo.";

/// Quién cargó el ajuste.
#[allow(dead_code)] // `Cli` lo usa el subcomando `ajuste` (commit 63)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrigenAjuste {
    App,
    Cli,
}

impl OrigenAjuste {
    pub fn como_texto(self) -> &'static str {
        match self {
            OrigenAjuste::App => "app",
            OrigenAjuste::Cli => "cli",
        }
    }
}

/// Un ajuste por cargar. `desde`/`hasta` van juntos o no van: a qué rato o día
/// corresponde. Sin ellos, el ajuste cuenta el día en que se cargó (`creado_en`).
#[derive(Debug, Clone)]
pub struct NuevoAjuste<'a> {
    pub project_id: i64,
    /// Con signo: + suma, − resta.
    pub segundos: i64,
    pub desde: Option<DateTime<Utc>>,
    pub hasta: Option<DateTime<Utc>>,
    pub nota: &'a str,
    pub origen: OrigenAjuste,
    pub creado_en: DateTime<Utc>,
}

/// Un ajuste guardado, como lo lista el desglose (3.10.17).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Ajuste {
    pub id: i64,
    pub project_id: i64,
    pub segundos: i64,
    pub desde: Option<String>,
    pub hasta: Option<String>,
    pub nota: String,
    pub creado_en: String,
    pub origen: String,
    pub revierte_id: Option<i64>,
}

const COLUMNAS_AJUSTE: &str = "a.id, a.project_id, a.segundos, a.desde, a.hasta, a.nota, a.creado_en, a.origen, a.revierte_id";

fn leer_ajuste(row: &Row) -> Result<Ajuste> {
    Ok(Ajuste {
        id: row.get(0)?,
        project_id: row.get(1)?,
        segundos: row.get(2)?,
        desde: row.get(3)?,
        hasta: row.get(4)?,
        nota: row.get(5)?,
        creado_en: row.get(6)?,
        origen: row.get(7)?,
        revierte_id: row.get(8)?,
    })
}

/// Error con un mensaje para el usuario, que llega tal cual a quien lo muestra.
fn error_de_validacion(mensaje: impl Into<String>) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(ffi::Error::new(ffi::SQLITE_ERROR), Some(mensaje.into()))
}

/// "1 h 20 min", "45 min", "30 s": para los mensajes de validación.
fn duracion_legible(segundos: i64) -> String {
    let (horas, minutos, resto) = (segundos / 3600, segundos % 3600 / 60, segundos % 60);
    match (horas, minutos, resto) {
        (0, 0, s) => format!("{s} s"),
        (0, m, 0) => format!("{m} min"),
        (h, 0, 0) => format!("{h} h"),
        (h, m, 0) => format!("{h} h {m} min"),
        _ => format!("{} min {} s", segundos / 60, resto),
    }
}

fn validar_nota(nota: &str) -> Result<()> {
    if nota.trim().is_empty() {
        return Err(error_de_validacion(MENSAJE_SIN_NOTA));
    }
    Ok(())
}

/// Las reglas de un ajuste nuevo que el usuario puede corregir, en español. Las mismas
/// que los `CHECK` de la tabla, más la del rato: con horario y signo +, no se suma más
/// de lo que dura el rato (`segundos ≤ hasta − desde`).
fn validar(nuevo: &NuevoAjuste) -> Result<()> {
    validar_nota(nuevo.nota)?;
    if nuevo.segundos == 0 {
        return Err(error_de_validacion(MENSAJE_CERO));
    }
    if nuevo.segundos.abs() > TOPE_AJUSTE_SEGUNDOS {
        return Err(error_de_validacion(MENSAJE_TOPE));
    }
    match (nuevo.desde, nuevo.hasta) {
        (None, None) => Ok(()),
        (Some(desde), Some(hasta)) => {
            // En segundos enteros, como se guardan: el `CHECK` compara el texto ya
            // truncado.
            let intervalo = hasta.timestamp() - desde.timestamp();
            if intervalo <= 0 {
                return Err(error_de_validacion(MENSAJE_HORARIO_AL_REVES));
            }
            if nuevo.segundos > intervalo {
                return Err(error_de_validacion(format!(
                    "Ese rato dura {} y el ajuste suma {}: no puede sumar más que el rato. Achicá la cantidad o ampliá el horario.",
                    duracion_legible(intervalo),
                    duracion_legible(nuevo.segundos)
                )));
            }
            Ok(())
        }
        _ => Err(error_de_validacion(MENSAJE_HORARIO_INCOMPLETO)),
    }
}

/// Fecha efectiva de un ajuste dentro de `[:desde, :hasta)`.
const FILTRO_FECHA_EFECTIVA: &str =
    "COALESCE(a.desde, a.creado_en) >= :desde AND COALESCE(a.desde, a.creado_en) < :hasta";

/// Σ de los ajustes de `project_id` cuya fecha efectiva cae en el rango.
pub(super) fn suma_de_ajustes(conn: &Connection, project_id: i64, rango: &RangoUtc) -> Result<i64> {
    conn.query_row(
        &format!(
            "SELECT COALESCE(SUM(a.segundos), 0) FROM ajustes_tiempo a
             WHERE a.project_id = :project_id AND {FILTRO_FECHA_EFECTIVA}"
        ),
        rusqlite::named_params! {
            ":project_id": project_id,
            ":desde": rango.desde_texto(),
            ":hasta": rango.hasta_texto(),
        },
        |row| row.get(0),
    )
}

/// [`suma_de_ajustes`] de cada proyecto fuera de la papelera con ajustes en el rango,
/// con una sola consulta (para `get_project_stats`).
pub(super) fn sumas_de_ajustes_por_proyecto(
    conn: &Connection,
    rango: &RangoUtc,
) -> Result<HashMap<i64, i64>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT a.project_id, SUM(a.segundos) FROM ajustes_tiempo a
         JOIN projects p ON p.id = a.project_id
         WHERE p.deleted_at IS NULL AND {FILTRO_FECHA_EFECTIVA}
         GROUP BY a.project_id"
    ))?;
    let filas = stmt.query_map(
        rusqlite::named_params! {
            ":desde": rango.desde_texto(),
            ":hasta": rango.hasta_texto(),
        },
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    filas.collect()
}

/// El total que ve el usuario: lo medido más los ajustes, recortado en 0 cuando los
/// ajustes restan más de lo medido (3.10.17).
pub(super) fn con_ajustes(trabajado: i64, ajustes: i64) -> i64 {
    (trabajado + ajustes).max(0)
}

impl Database {
    /// Carga un ajuste (3.10.17) y devuelve su id. Valida antes de escribir, con
    /// mensajes en español; la escritura va en una transacción `IMMEDIATE`.
    #[allow(dead_code)] // Sin llamadores hasta el subcomando (commit 63) y el formulario (commit 70)
    pub fn agregar_ajuste(&self, nuevo: &NuevoAjuste) -> Result<i64> {
        validar(nuevo)?;
        let mut conn = self.conn();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO ajustes_tiempo (project_id, segundos, desde, hasta, nota, creado_en, origen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                nuevo.project_id,
                nuevo.segundos,
                nuevo.desde.map(time::formatear),
                nuevo.hasta.map(time::formatear),
                nuevo.nota,
                time::formatear(nuevo.creado_en),
                nuevo.origen.como_texto()
            ],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(id)
    }

    /// Revierte el ajuste `id` con otro: copia `project_id`, `desde` y `hasta`, y pone
    /// `segundos = −original`. El original no puede ser a su vez una reversión. Revertir
    /// dos veces el mismo es un error del `UNIQUE` de `revierte_id`: lo frena la base.
    #[allow(dead_code)] // Sin llamadores hasta el subcomando (commit 63) y el formulario (commit 70)
    pub fn revertir_ajuste(
        &self,
        id: i64,
        nota: &str,
        origen: OrigenAjuste,
        creado_en: DateTime<Utc>,
    ) -> Result<i64> {
        validar_nota(nota)?;
        let mut conn = self.conn();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let original: Option<Ajuste> = tx
            .query_row(
                &format!("SELECT {COLUMNAS_AJUSTE} FROM ajustes_tiempo a WHERE a.id = ?1"),
                params![id],
                leer_ajuste,
            )
            .optional()?;
        let Some(original) = original else {
            return Err(error_de_validacion(format!(
                "No existe el ajuste {id}. Revisá el número en la lista de ajustes."
            )));
        };
        if original.revierte_id.is_some() {
            return Err(error_de_validacion(MENSAJE_REVERTIR_REVERSION));
        }
        tx.execute(
            "INSERT INTO ajustes_tiempo
                 (project_id, segundos, desde, hasta, nota, creado_en, origen, revierte_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                original.project_id,
                -original.segundos,
                original.desde,
                original.hasta,
                nota,
                time::formatear(creado_en),
                origen.como_texto(),
                id
            ],
        )?;
        let nuevo = tx.last_insert_rowid();
        tx.commit()?;
        Ok(nuevo)
    }

    /// Los ajustes de `project_id` cuya fecha efectiva cae en el rango, en orden de
    /// fecha efectiva y de carga.
    #[allow(dead_code)] // Sin llamadores hasta el subcomando (commit 63) y el desglose (commit 70)
    pub fn listar_ajustes(&self, project_id: i64, rango: RangoUtc) -> Result<Vec<Ajuste>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLUMNAS_AJUSTE}
             FROM ajustes_tiempo a
             WHERE a.project_id = :project_id AND {FILTRO_FECHA_EFECTIVA}
             ORDER BY COALESCE(a.desde, a.creado_en), a.id"
        ))?;
        let filas = stmt.query_map(
            rusqlite::named_params! {
                ":project_id": project_id,
                ":desde": rango.desde_texto(),
                ":hasta": rango.hasta_texto(),
            },
            leer_ajuste,
        )?;
        filas.collect()
    }

    /// `max(0, tiempo_trabajado + Σ ajustes del rango)` (3.10.8): el total que ve el
    /// usuario. La unión y los ajustes se leen bajo el mismo lock.
    #[allow(dead_code)] // Las estadísticas lo arman bajo un solo lock para sus tres rangos; lo llaman el desglose y el demonio
    pub fn tiempo_total(
        &self,
        project_id: i64,
        rango: RangoUtc,
        activa: Option<SesionActiva>,
    ) -> Result<i64> {
        let conn = self.conn();
        let unidos = union_del_proyecto(&conn, project_id, &rango, activa)?;
        Ok(con_ajustes(
            segundos_en(&unidos, &rango),
            suma_de_ajustes(&conn, project_id, &rango)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::*;
    use crate::time::{parsear, rango_dia};
    use chrono::NaiveDate;
    use chrono_tz::America::Argentina::Buenos_Aires;

    const H: i64 = 3600;

    fn instante(texto: &str) -> DateTime<Utc> {
        parsear(texto).unwrap()
    }

    /// Un día entero en UTC: el 2026-10-20.
    fn dia() -> RangoUtc {
        RangoUtc {
            desde: instante("2026-10-20 00:00:00"),
            hasta: instante("2026-10-21 00:00:00"),
        }
    }

    fn ajuste(project_id: i64, segundos: i64) -> NuevoAjuste<'static> {
        NuevoAjuste {
            project_id,
            segundos,
            desde: None,
            hasta: None,
            nota: "Reunión con el cliente",
            origen: OrigenAjuste::App,
            creado_en: instante("2026-10-20 15:00:00"),
        }
    }

    fn rato<'a>(nuevo: NuevoAjuste<'a>, desde: &str, hasta: &str) -> NuevoAjuste<'a> {
        NuevoAjuste {
            desde: Some(instante(desde)),
            hasta: Some(instante(hasta)),
            ..nuevo
        }
    }

    fn tramo(db: &Database, project_id: i64, desde: &str, hasta: &str) {
        let segundos = instante(hasta).timestamp() - instante(desde).timestamp();
        db.conn()
            .execute(
                "INSERT INTO time_tracking_sessions
                     (project_id, source, started_at, ended_at, duration_seconds)
                 VALUES (?1, 'terminal', ?2, ?3, ?4)",
                params![project_id, desde, hasta, segundos],
            )
            .unwrap();
    }

    fn cantidad_de_ajustes(db: &Database) -> i64 {
        db.conn()
            .query_row("SELECT COUNT(*) FROM ajustes_tiempo", [], |row| row.get(0))
            .unwrap()
    }

    fn mensaje(resultado: Result<i64>) -> String {
        resultado.expect_err("tenía que ser un error").to_string()
    }

    // --- Solo de agregado: triggers y cascada ---

    /// En un proyecto vivo, el trigger rechaza cualquier `UPDATE` y cualquier `DELETE`.
    #[test]
    fn el_trigger_bloquea_update_y_delete_en_un_proyecto_vivo() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        let id = db.agregar_ajuste(&ajuste(p, 600)).unwrap();

        for sql in [
            "UPDATE ajustes_tiempo SET nota = 'x'",
            "UPDATE ajustes_tiempo SET segundos = 1",
            "DELETE FROM ajustes_tiempo",
        ] {
            let err = db.conn().execute(sql, []).unwrap_err();
            assert!(
                err.to_string()
                    .contains("ajustes_tiempo es solo de agregado"),
                "{sql}: {err}"
            );
        }
        let nota: String = db
            .conn()
            .query_row(
                "SELECT nota FROM ajustes_tiempo WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(nota, "Reunión con el cliente");
        assert_eq!(cantidad_de_ajustes(&db), 1);
    }

    /// Siembra un proyecto con un ajuste y su reversión (que apunta al otro), y lo manda
    /// a la papelera.
    fn proyecto_con_ajustes_en_la_papelera(db: &Database, nombre: &str) -> i64 {
        let p = seed_project_at(db, nombre, &format!("/tmp/{nombre}"));
        let id = db.agregar_ajuste(&ajuste(p, 600)).unwrap();
        db.revertir_ajuste(id, "Era de otro proyecto", OrigenAjuste::App, Utc::now())
            .unwrap();
        db.delete_project(p, Utc::now(), None).unwrap();
        p
    }

    /// Purgar un proyecto, de a uno o vaciando la papelera, borra sus ajustes en
    /// cascada, incluida una reversión que apunta a otro ajuste del mismo proyecto. Los
    /// de otro proyecto quedan.
    #[test]
    fn purgar_borra_los_ajustes_en_cascada() {
        let db = test_db();
        let vivo = seed_project_at(&db, "Vivo", "/tmp/vivo");
        db.agregar_ajuste(&ajuste(vivo, 300)).unwrap();

        let de_a_uno = proyecto_con_ajustes_en_la_papelera(&db, "De a uno");
        assert_eq!(cantidad_de_ajustes(&db), 3);
        db.purge_project(de_a_uno).unwrap();
        assert_eq!(cantidad_de_ajustes(&db), 1);

        proyecto_con_ajustes_en_la_papelera(&db, "Papelera A");
        proyecto_con_ajustes_en_la_papelera(&db, "Papelera B");
        assert_eq!(cantidad_de_ajustes(&db), 5);
        db.empty_trash().unwrap();
        assert_eq!(
            cantidad_de_ajustes(&db),
            1,
            "solo queda el del proyecto vivo"
        );
    }

    /// Borrar la fila de un proyecto vivo directamente (sin pasar por la papelera)
    /// también cascadea: durante la cascada la fila del proyecto ya no existe.
    #[test]
    fn borrar_un_proyecto_vivo_directo_borra_los_ajustes_en_cascada() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        let id = db.agregar_ajuste(&ajuste(p, 600)).unwrap();
        db.revertir_ajuste(id, "Me equivoqué", OrigenAjuste::Cli, Utc::now())
            .unwrap();

        db.conn()
            .execute("DELETE FROM projects WHERE id = ?1", params![p])
            .unwrap();
        assert_eq!(cantidad_de_ajustes(&db), 0);
    }

    // --- Reversión ---

    /// La reversión copia proyecto y horario y niega los segundos (invariante 15).
    #[test]
    fn revertir_copia_el_horario_y_niega_los_segundos() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        let original = db
            .agregar_ajuste(&rato(
                ajuste(p, 1800),
                "2026-10-20 15:00:00",
                "2026-10-20 16:00:00",
            ))
            .unwrap();

        let reversion = db
            .revertir_ajuste(
                original,
                "Ya estaba medido",
                OrigenAjuste::Cli,
                instante("2026-10-22 10:00:00"),
            )
            .unwrap();

        let ajustes = db.listar_ajustes(p, dia()).unwrap();
        assert_eq!(ajustes.len(), 2, "la reversión cae en el día del original");
        let r = ajustes.iter().find(|a| a.id == reversion).unwrap();
        assert_eq!(r.segundos, -1800);
        assert_eq!(r.desde.as_deref(), Some("2026-10-20 15:00:00"));
        assert_eq!(r.hasta.as_deref(), Some("2026-10-20 16:00:00"));
        assert_eq!(r.revierte_id, Some(original));
        assert_eq!(r.origen, "cli");
        assert_eq!(r.creado_en, "2026-10-22 10:00:00");
        assert_eq!(db.tiempo_total(p, dia(), None).unwrap(), 0);
    }

    /// Revertir dos veces el mismo ajuste lo frena el `UNIQUE` de `revierte_id`.
    #[test]
    fn revertir_dos_veces_es_un_error_de_unique() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        let id = db.agregar_ajuste(&ajuste(p, 600)).unwrap();
        db.revertir_ajuste(id, "Primera", OrigenAjuste::App, Utc::now())
            .unwrap();

        let err = db
            .revertir_ajuste(id, "Segunda", OrigenAjuste::App, Utc::now())
            .unwrap_err();
        assert_eq!(
            err.sqlite_error_code(),
            Some(rusqlite::ErrorCode::ConstraintViolation),
            "{err}"
        );
        assert!(err.to_string().contains("UNIQUE"), "{err}");
        assert_eq!(cantidad_de_ajustes(&db), 2);
    }

    /// Una reversión no se revierte, y un id que no existe tampoco: errores en español.
    #[test]
    fn revertir_una_reversion_o_un_id_inexistente_es_un_error_en_espanol() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        let id = db.agregar_ajuste(&ajuste(p, 600)).unwrap();
        let reversion = db
            .revertir_ajuste(id, "Primera", OrigenAjuste::App, Utc::now())
            .unwrap();

        assert_eq!(
            mensaje(db.revertir_ajuste(reversion, "Otra", OrigenAjuste::App, Utc::now())),
            MENSAJE_REVERTIR_REVERSION
        );
        assert_eq!(
            mensaje(db.revertir_ajuste(999, "Otra", OrigenAjuste::App, Utc::now())),
            "No existe el ajuste 999. Revisá el número en la lista de ajustes."
        );
        assert_eq!(
            mensaje(db.revertir_ajuste(id, "   ", OrigenAjuste::App, Utc::now())),
            MENSAJE_SIN_NOTA
        );
        assert_eq!(cantidad_de_ajustes(&db), 2);
    }

    // --- Validaciones en español ---

    #[test]
    fn las_validaciones_dan_errores_en_espanol_y_no_escriben() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        let casos: Vec<(&str, NuevoAjuste, String)> = vec![
            (
                "nota vacía",
                NuevoAjuste {
                    nota: "",
                    ..ajuste(p, 600)
                },
                MENSAJE_SIN_NOTA.to_string(),
            ),
            (
                "nota de espacios",
                NuevoAjuste {
                    nota: " \t\n ",
                    ..ajuste(p, 600)
                },
                MENSAJE_SIN_NOTA.to_string(),
            ),
            ("0 segundos", ajuste(p, 0), MENSAJE_CERO.to_string()),
            (
                "más de 24 h",
                ajuste(p, -(TOPE_AJUSTE_SEGUNDOS + 1)),
                MENSAJE_TOPE.to_string(),
            ),
            (
                "desde sin hasta",
                NuevoAjuste {
                    desde: Some(instante("2026-10-20 15:00:00")),
                    ..ajuste(p, 600)
                },
                MENSAJE_HORARIO_INCOMPLETO.to_string(),
            ),
            (
                "hasta sin desde",
                NuevoAjuste {
                    hasta: Some(instante("2026-10-20 15:00:00")),
                    ..ajuste(p, 600)
                },
                MENSAJE_HORARIO_INCOMPLETO.to_string(),
            ),
            (
                "hasta antes que desde",
                rato(ajuste(p, 600), "2026-10-20 16:00:00", "2026-10-20 15:00:00"),
                MENSAJE_HORARIO_AL_REVES.to_string(),
            ),
            (
                "rato con más segundos que el intervalo",
                rato(ajuste(p, H + 1), "2026-10-20 15:00:00", "2026-10-20 16:00:00"),
                "Ese rato dura 1 h y el ajuste suma 60 min 1 s: no puede sumar más que el rato. Achicá la cantidad o ampliá el horario."
                    .to_string(),
            ),
        ];
        for (nombre, nuevo, esperado) in casos {
            assert_eq!(mensaje(db.agregar_ajuste(&nuevo)), esperado, "{nombre}");
        }
        assert_eq!(cantidad_de_ajustes(&db), 0);

        // El límite exacto vale, y un rato que resta no tiene tope por el intervalo.
        db.agregar_ajuste(&rato(
            ajuste(p, H),
            "2026-10-20 15:00:00",
            "2026-10-20 16:00:00",
        ))
        .unwrap();
        db.agregar_ajuste(&rato(
            ajuste(p, -2 * H),
            "2026-10-20 15:00:00",
            "2026-10-20 16:00:00",
        ))
        .unwrap();
        db.agregar_ajuste(&ajuste(p, TOPE_AJUSTE_SEGUNDOS)).unwrap();
        assert_eq!(cantidad_de_ajustes(&db), 3);
    }

    /// Los `CHECK` de la tabla frenan lo que no pase por `agregar_ajuste`.
    #[test]
    fn los_check_de_la_tabla_frenan_un_insert_directo_invalido() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        for (segundos, desde, hasta, nota, origen) in [
            (0, None, None, "nota", "app"),
            (86_401, None, None, "nota", "app"),
            (60, Some("2026-10-20 15:00:00"), None, "nota", "app"),
            (
                60,
                Some("2026-10-20 15:00:00"),
                Some("2026-10-20 15:00:00"),
                "nota",
                "app",
            ),
            (60, None, None, "  ", "app"),
            (60, None, None, "nota", "web"),
        ] {
            let err = db
                .conn()
                .execute(
                    "INSERT INTO ajustes_tiempo (project_id, segundos, desde, hasta, nota, creado_en, origen)
                     VALUES (?1, ?2, ?3, ?4, ?5, '2026-10-20 15:00:00', ?6)",
                    params![p, segundos, desde, hasta, nota, origen],
                )
                .unwrap_err();
            assert_eq!(
                err.sqlite_error_code(),
                Some(rusqlite::ErrorCode::ConstraintViolation),
                "{segundos} {desde:?} {hasta:?} {nota:?} {origen}: {err}"
            );
        }
        assert_eq!(cantidad_de_ajustes(&db), 0);
    }

    #[test]
    fn duracion_legible_de_los_mensajes() {
        for (segundos, esperado) in [
            (30, "30 s"),
            (45 * 60, "45 min"),
            (2 * H, "2 h"),
            (H + 20 * 60, "1 h 20 min"),
            (90, "1 min 30 s"),
        ] {
            assert_eq!(duracion_legible(segundos), esperado);
        }
    }

    // --- tiempo_total ---

    /// Un ajuste suma o resta en el rango que contiene su fecha efectiva, y en ningún
    /// otro: con −30 min, el total baja 30 min.
    #[test]
    fn tiempo_total_suma_los_ajustes_dentro_del_rango_y_no_los_de_afuera() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        tramo(&db, p, "2026-10-20 13:00:00", "2026-10-20 15:00:00");
        assert_eq!(db.tiempo_total(p, dia(), None).unwrap(), 2 * H);

        db.agregar_ajuste(&ajuste(p, -30 * 60)).unwrap();
        // Fuera del rango: con horario el día anterior, y sin horario cargado al otro día.
        db.agregar_ajuste(&rato(
            ajuste(p, 600),
            "2026-10-19 10:00:00",
            "2026-10-19 11:00:00",
        ))
        .unwrap();
        db.agregar_ajuste(&NuevoAjuste {
            creado_en: instante("2026-10-21 00:00:00"),
            ..ajuste(p, 900)
        })
        .unwrap();

        assert_eq!(db.tiempo_total(p, dia(), None).unwrap(), 2 * H - 30 * 60);
        assert_eq!(
            db.tiempo_trabajado(p, dia(), None).unwrap(),
            2 * H,
            "la unión pura no ve los ajustes"
        );
        assert_eq!(
            db.tiempo_total(p, time::rango_total(), None).unwrap(),
            2 * H - 30 * 60 + 600 + 900
        );
        assert_eq!(db.listar_ajustes(p, dia()).unwrap().len(), 1);
    }

    /// Un ajuste que resta más de lo medido deja el total en 0, no negativo.
    #[test]
    fn tiempo_total_se_recorta_en_0() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        tramo(&db, p, "2026-10-20 13:00:00", "2026-10-20 13:20:00");
        db.agregar_ajuste(&ajuste(p, -H)).unwrap();
        assert_eq!(db.tiempo_total(p, dia(), None).unwrap(), 0);

        let sin_tramos = seed_project_at(&db, "Sin tramos", "/tmp/sin");
        db.agregar_ajuste(&ajuste(sin_tramos, -600)).unwrap();
        assert_eq!(db.tiempo_total(sin_tramos, dia(), None).unwrap(), 0);
        db.agregar_ajuste(&ajuste(sin_tramos, 1800)).unwrap();
        assert_eq!(db.tiempo_total(sin_tramos, dia(), None).unwrap(), 1200);
    }

    /// Sin `desde`, el ajuste cuenta el día local en que se cargó: a las 22:00 de Buenos
    /// Aires (01:00 UTC del 21) sigue siendo el 20. Con horario, el de su `desde`.
    #[test]
    fn sin_desde_cuenta_el_dia_local_de_creado_en() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        db.agregar_ajuste(&NuevoAjuste {
            creado_en: instante("2026-10-21 01:00:00"),
            ..ajuste(p, 600)
        })
        .unwrap();
        // Cargado el 22, pero para el rato del 20 a las 23:00 locales (02:00 UTC del 21).
        db.agregar_ajuste(&NuevoAjuste {
            creado_en: instante("2026-10-22 12:00:00"),
            ..rato(ajuste(p, 300), "2026-10-21 02:00:00", "2026-10-21 02:30:00")
        })
        .unwrap();

        let fecha = |d| NaiveDate::from_ymd_opt(2026, 10, d).unwrap();
        let el_20 = rango_dia(&Buenos_Aires, fecha(20));
        let el_21 = rango_dia(&Buenos_Aires, fecha(21));
        assert_eq!(db.tiempo_total(p, el_20, None).unwrap(), 900);
        assert_eq!(db.tiempo_total(p, el_21, None).unwrap(), 0);
        assert_eq!(dia().desde_texto(), "2026-10-20 00:00:00");
        assert_eq!(
            db.tiempo_total(p, dia(), None).unwrap(),
            0,
            "en UTC los dos caen el 21"
        );
    }

    /// `get_time_stats` y `get_project_stats` muestran `tiempo_total`: los ajustes entran
    /// en el total, en hoy y en la semana, pero no en las sesiones. La papelera no entra.
    #[test]
    fn las_estadisticas_usan_tiempo_total() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        let solo_ajustes = seed_project_at(&db, "Solo ajustes", "/tmp/solo");
        let negativo = seed_project_at(&db, "Negativo", "/tmp/negativo");
        let borrado = seed_project_at(&db, "Borrado", "/tmp/borrado");
        // 15:00 a 17:00 UTC del 20 = 12:00 a 14:00 de Buenos Aires.
        tramo(&db, p, "2026-10-20 15:00:00", "2026-10-20 17:00:00");
        db.agregar_ajuste(&ajuste(p, -30 * 60)).unwrap();
        db.agregar_ajuste(&NuevoAjuste {
            creado_en: instante("2026-10-10 15:00:00"),
            ..ajuste(p, 600)
        })
        .unwrap();
        db.agregar_ajuste(&ajuste(solo_ajustes, 1800)).unwrap();
        tramo(&db, negativo, "2026-10-20 15:00:00", "2026-10-20 15:10:00");
        db.agregar_ajuste(&ajuste(negativo, -H)).unwrap();
        db.agregar_ajuste(&ajuste(borrado, 7200)).unwrap();
        db.delete_project(borrado, Utc::now(), None).unwrap();

        let hoy = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        let s = db
            .get_time_stats(
                p,
                rango_dia(&Buenos_Aires, hoy),
                time::rango_semana(&Buenos_Aires, hoy),
                None,
            )
            .unwrap();
        assert_eq!(s.total_seconds, 2 * H - 1800 + 600);
        assert_eq!(s.today_seconds, 2 * H - 1800);
        assert_eq!(
            s.week_seconds,
            2 * H - 1800,
            "el del 10 queda fuera de la semana"
        );
        assert_eq!(
            (
                s.session_count,
                s.avg_session_seconds,
                s.longest_session_seconds
            ),
            (1, 2 * H, 2 * H),
            "las sesiones no ven los ajustes"
        );

        let stats = db
            .get_project_stats(rango_dia(&Buenos_Aires, hoy), None)
            .unwrap();
        // P: 2 h − 30 min + 10 min; solo ajustes: 30 min; negativo: recortado en 0.
        assert_eq!(
            stats.total_time_hours,
            (2 * H - 1800 + 600 + 1800) as f64 / 3600.0
        );
    }
}
