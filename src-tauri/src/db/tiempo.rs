//! Tiempo trabajado como unión de tramos (§3.3 y 3.10.8 del diseño del tracker,
//! commit 20).
//!
//! Cada fila de `time_tracking_sessions` aporta un **tramo** `[desde, hasta)` según
//! su fuente y si está abierta (la tabla de 3.3). Los tramos de un proyecto se
//! recortan al rango pedido, se ordenan y se fusionan: un instante cuenta una sola vez
//! aunque lo cubran Trabajar, la terminal y `file_watch` a la vez (R4). Ningún tramo
//! depende de `tracker_status` (M6): un `file_watch` abierto cuenta hasta su último
//! cambio, esté vivo o muerto el demonio.
//!
//! Fusionar en SQL exigiría consultas recursivas; en Rust es un recorrido lineal sobre
//! la lista ordenada.

// `get_time_stats` y `get_project_stats` pasan a la unión en el commit 21; la
// simultaneidad la muestra el desglose (commit 69). Hasta entonces solo la usan sus
// tests.
#![allow(dead_code)]

use std::collections::HashMap;

use rusqlite::types::Type;
use rusqlite::{Connection, Result, Row};

use super::trabajar::{FUENTE_TERMINAL, TOPE_SIN_TRACKER};
use super::Database;
use crate::commands::duracion_a_guardar;
use crate::time::{self, RangoUtc};

/// Fuente de los tramos que abre el demonio por cambios en archivos.
const FUENTE_ARCHIVOS: &str = "file_watch";

/// La sesión de "Trabajar" viva de esta app, con lo medido por su `Instant` (M5): la
/// copia de la única `MedicionApp` de `ActiveSession`, armada antes de soltar el lock.
/// Las sesiones del demonio se leen de la base y no la necesitan (3.10.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SesionActiva {
    /// El id del tramo `work_button` abierto que mide la app.
    pub session_id: i64,
    /// Segundos del `Instant` en el momento de la consulta.
    pub segundos: i64,
}

/// Un tramo `[desde, hasta)` en segundos Unix.
type Tramo = (i64, i64);

/// Lo que la unión lee de cada fila de `time_tracking_sessions`.
struct Fila {
    id: i64,
    project_id: i64,
    source: Option<String>,
    started_at: String,
    ended_at: Option<String>,
    last_activity_at: Option<String>,
    duration_seconds: Option<i64>,
}

const COLUMNAS: &str =
    "s.id, s.project_id, s.source, s.started_at, s.ended_at, s.last_activity_at, s.duration_seconds";

/// Filtro del rango (3.3, paso 1): `started_at < hasta` y la fila abierta o cerrada
/// después de `desde`. En las cerradas de otra fuente `ended_at ≥ started_at +
/// duration_seconds`, así que trae de más y nunca de menos: el recorte lo hace Rust.
const FILTRO_RANGO: &str = "s.started_at < :hasta AND (s.ended_at IS NULL OR s.ended_at > :desde)";

fn leer_fila(row: &Row) -> Result<Fila> {
    Ok(Fila {
        id: row.get(0)?,
        project_id: row.get(1)?,
        source: row.get(2)?,
        started_at: row.get(3)?,
        ended_at: row.get(4)?,
        last_activity_at: row.get(5)?,
        duration_seconds: row.get(6)?,
    })
}

fn segundos_unix(texto: &str) -> Result<i64> {
    time::parsear(texto)
        .map(|instante| instante.timestamp())
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(e)))
}

/// El tramo de una fila (tabla de 3.3), sin recortar. Nunca termina antes de empezar.
///
/// - Cerrada, `file_watch` o `terminal`: `[started_at, ended_at]`.
/// - Cerrada, otra fuente: `[started_at, started_at + duration_seconds]`. No se usa
///   `ended_at`: la duración del `Instant` excluye la suspensión y respeta la regla de
///   los 10 s.
/// - Abierta y es la `activa`: los segundos del `Instant` con la regla de los 10 s y el
///   tope de 12 h, lo mismo que se va a guardar al cerrar.
/// - Abierta, `file_watch` o `terminal`: hasta `last_activity_at`, el último cambio
///   real volcado.
/// - Abierta, otra fuente: lo medido hasta el último latido, o 0 s si nunca latió.
fn tramo_de(fila: &Fila, activa: Option<SesionActiva>) -> Result<Tramo> {
    let inicio = segundos_unix(&fila.started_at)?;
    let source = fila.source.as_deref().unwrap_or_default();
    let de_ultimo_cambio = source == FUENTE_ARCHIVOS || source == FUENTE_TERMINAL;
    let activa = activa.filter(|a| a.session_id == fila.id);

    let fin = match (&fila.ended_at, activa) {
        (Some(ended_at), _) if de_ultimo_cambio => segundos_unix(ended_at)?,
        (Some(_), _) => inicio + fila.duration_seconds.unwrap_or(0),
        (None, Some(activa)) => {
            inicio + duracion_a_guardar(source, activa.segundos.min(TOPE_SIN_TRACKER))
        }
        (None, None) if de_ultimo_cambio => match &fila.last_activity_at {
            Some(ultimo) => segundos_unix(ultimo)?,
            None => inicio,
        },
        (None, None) => inicio + fila.duration_seconds.unwrap_or(0),
    };
    Ok((inicio, fin.max(inicio)))
}

fn recortar((desde, hasta): Tramo, rango: &RangoUtc) -> Tramo {
    (
        desde.max(rango.desde.timestamp()),
        hasta.min(rango.hasta.timestamp()),
    )
}

/// Ordena y fusiona: los superpuestos, anidados y los que se tocan quedan en uno. Los
/// vacíos se descartan.
fn unir(mut tramos: Vec<Tramo>) -> Vec<Tramo> {
    tramos.retain(|(desde, hasta)| desde < hasta);
    tramos.sort_unstable();
    let mut unidos: Vec<Tramo> = Vec::with_capacity(tramos.len());
    for (desde, hasta) in tramos {
        match unidos.last_mut() {
            Some(ultimo) if desde <= ultimo.1 => ultimo.1 = ultimo.1.max(hasta),
            _ => unidos.push((desde, hasta)),
        }
    }
    unidos
}

fn sumar(unidos: &[Tramo]) -> i64 {
    unidos.iter().map(|(desde, hasta)| hasta - desde).sum()
}

/// Segundos que cubren a la vez dos uniones (listas ordenadas y disjuntas): un
/// recorrido lineal de las dos.
fn interseccion(a: &[Tramo], b: &[Tramo]) -> i64 {
    let (mut i, mut j, mut total) = (0, 0, 0);
    while i < a.len() && j < b.len() {
        let desde = a[i].0.max(b[j].0);
        let hasta = a[i].1.min(b[j].1);
        if desde < hasta {
            total += hasta - desde;
        }
        if a[i].1 < b[j].1 {
            i += 1;
        } else {
            j += 1;
        }
    }
    total
}

/// La unión de los tramos de `project_id` dentro del rango.
pub(super) fn union_del_proyecto(
    conn: &Connection,
    project_id: i64,
    rango: &RangoUtc,
    activa: Option<SesionActiva>,
) -> Result<Vec<Tramo>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNAS} FROM time_tracking_sessions s
         WHERE s.project_id = :project_id AND {FILTRO_RANGO}"
    ))?;
    let filas = stmt.query_map(
        rusqlite::named_params! {
            ":project_id": project_id,
            ":desde": rango.desde_texto(),
            ":hasta": rango.hasta_texto(),
        },
        leer_fila,
    )?;
    let mut tramos = Vec::new();
    for fila in filas {
        tramos.push(recortar(tramo_de(&fila?, activa)?, rango));
    }
    Ok(unir(tramos))
}

/// La unión de cada proyecto fuera de la papelera con tramos en el rango, con **una**
/// consulta para todos (la variante de 3.3 para `get_project_stats`).
pub(super) fn uniones_por_proyecto(
    conn: &Connection,
    rango: &RangoUtc,
    activa: Option<SesionActiva>,
) -> Result<HashMap<i64, Vec<Tramo>>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNAS} FROM time_tracking_sessions s
         JOIN projects p ON p.id = s.project_id
         WHERE p.deleted_at IS NULL AND {FILTRO_RANGO}"
    ))?;
    let filas = stmt.query_map(
        rusqlite::named_params! {
            ":desde": rango.desde_texto(),
            ":hasta": rango.hasta_texto(),
        },
        leer_fila,
    )?;
    let mut tramos: HashMap<i64, Vec<Tramo>> = HashMap::new();
    for fila in filas {
        let fila = fila?;
        let tramo = recortar(tramo_de(&fila, activa)?, rango);
        tramos.entry(fila.project_id).or_default().push(tramo);
    }
    Ok(tramos
        .into_iter()
        .map(|(project_id, tramos)| (project_id, unir(tramos)))
        .collect())
}

impl Database {
    /// Segundos trabajados en `project_id` dentro de `[desde, hasta)`, contando cada
    /// instante una sola vez aunque haya sesiones superpuestas (3.3). `activa` solo se
    /// aplica a la fila con ese id que siga abierta. El demonio llama con `None`.
    pub fn tiempo_trabajado(
        &self,
        project_id: i64,
        rango: RangoUtc,
        activa: Option<SesionActiva>,
    ) -> Result<i64> {
        Ok(sumar(&union_del_proyecto(
            &self.conn(),
            project_id,
            &rango,
            activa,
        )?))
    }

    /// [`Database::tiempo_trabajado`] de todos los proyectos fuera de la papelera, con
    /// una sola consulta. Un proyecto sin tramos en el rango no aparece.
    pub fn tiempo_trabajado_por_proyecto(
        &self,
        rango: RangoUtc,
        activa: Option<SesionActiva>,
    ) -> Result<HashMap<i64, i64>> {
        Ok(uniones_por_proyecto(&self.conn(), &rango, activa)?
            .into_iter()
            .map(|(project_id, unidos)| (project_id, sumar(&unidos)))
            .collect())
    }

    /// Simultaneidad (4a, 3.10.8): para cada otro proyecto con tramos en el rango, los
    /// segundos que su unión comparte con la de `project_id`. Se calcula, no se guarda,
    /// y no se reparte: cada proyecto conserva su total. Los ajustes no entran. Solo
    /// aparecen los proyectos con algún segundo en común.
    pub fn simultaneidad(&self, project_id: i64, rango: RangoUtc) -> Result<Vec<(i64, i64)>> {
        let mut uniones = uniones_por_proyecto(&self.conn(), &rango, None)?;
        let Some(propia) = uniones.remove(&project_id) else {
            return Ok(Vec::new());
        };
        let mut simultaneos: Vec<(i64, i64)> = uniones
            .iter()
            .map(|(otro, unidos)| (*otro, interseccion(&propia, unidos)))
            .filter(|(_, segundos)| *segundos > 0)
            .collect();
        simultaneos.sort_unstable();
        Ok(simultaneos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::*;
    use crate::time::{parsear, rango_dia};
    use chrono::NaiveDate;
    use chrono_tz::America::Argentina::Buenos_Aires;
    use rusqlite::params;

    const H: i64 = 3600;

    /// Un día entero en UTC: el 2026-10-20.
    fn dia() -> RangoUtc {
        RangoUtc {
            desde: parsear("2026-10-20 00:00:00").unwrap(),
            hasta: parsear("2026-10-21 00:00:00").unwrap(),
        }
    }

    fn a_las(hora: &str) -> String {
        format!("2026-10-20 {hora}")
    }

    /// Un tramo cerrado: `ended_at` y `duration_seconds` explícitos.
    fn cerrado(
        db: &Database,
        project_id: i64,
        source: &str,
        desde: &str,
        hasta: &str,
        duracion: i64,
    ) -> i64 {
        db.conn()
            .execute(
                "INSERT INTO time_tracking_sessions
                     (project_id, source, started_at, ended_at, last_activity_at, duration_seconds)
                 VALUES (?1, ?2, ?3, ?4, ?4, ?5)",
                params![project_id, source, a_las(desde), a_las(hasta), duracion],
            )
            .unwrap();
        db.conn().last_insert_rowid()
    }

    /// Un tramo de `terminal` o `file_watch` cerrado, con la duración de pared.
    fn cerrado_de_pared(db: &Database, project_id: i64, source: &str, desde: &str, hasta: &str) {
        let duracion =
            segundos_unix(&a_las(hasta)).unwrap() - segundos_unix(&a_las(desde)).unwrap();
        cerrado(db, project_id, source, desde, hasta, duracion);
    }

    /// Un tramo abierto con su latido y lo medido hasta él.
    fn abierto(
        db: &Database,
        project_id: i64,
        source: &str,
        desde: &str,
        latido: Option<&str>,
        duracion: Option<i64>,
    ) -> i64 {
        db.conn()
            .execute(
                "INSERT INTO time_tracking_sessions
                     (project_id, source, started_at, last_activity_at, duration_seconds)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    project_id,
                    source,
                    a_las(desde),
                    latido.map(a_las),
                    duracion
                ],
            )
            .unwrap();
        db.conn().last_insert_rowid()
    }

    fn proyecto(db: &Database, nombre: &str) -> i64 {
        seed_project_at(db, nombre, &format!("/tmp/{nombre}"))
    }

    fn trabajado(db: &Database, project_id: i64) -> i64 {
        db.tiempo_trabajado(project_id, dia(), None).unwrap()
    }

    // --- Unión pura (tabla) ---

    #[test]
    fn unir_disjuntos_superpuestos_anidados_y_que_se_tocan() {
        let casos: [(&str, Vec<Tramo>, Vec<Tramo>); 6] = [
            (
                "disjuntos",
                vec![(30, 40), (0, 10)],
                vec![(0, 10), (30, 40)],
            ),
            ("superpuestos", vec![(0, 10), (5, 20)], vec![(0, 20)]),
            ("anidados", vec![(0, 30), (10, 20)], vec![(0, 30)]),
            ("que se tocan", vec![(0, 10), (10, 20)], vec![(0, 20)]),
            ("vacíos afuera", vec![(5, 5), (8, 3)], vec![]),
            (
                "cadena desordenada",
                vec![(50, 60), (0, 10), (5, 15), (14, 20), (55, 58)],
                vec![(0, 20), (50, 60)],
            ),
        ];
        for (nombre, tramos, esperado) in casos {
            assert_eq!(unir(tramos), esperado, "{nombre}");
        }
        assert_eq!(sumar(&[(0, 20), (50, 60)]), 30);
    }

    #[test]
    fn interseccion_de_dos_uniones() {
        let casos: [(&str, Vec<Tramo>, Vec<Tramo>, i64); 4] = [
            ("disjuntas", vec![(0, 10)], vec![(10, 20)], 0),
            ("superpuestas", vec![(0, 10)], vec![(5, 20)], 5),
            ("anidada", vec![(0, 100)], vec![(10, 20), (30, 40)], 20),
            (
                "varias",
                vec![(0, 10), (20, 30), (40, 50)],
                vec![(5, 25), (45, 60)],
                15,
            ),
        ];
        for (nombre, a, b, esperado) in casos {
            assert_eq!(interseccion(&a, &b), esperado, "{nombre}");
            assert_eq!(interseccion(&b, &a), esperado, "{nombre}, al revés");
        }
    }

    // --- Tramos cerrados contra la base ---

    #[test]
    fn disjuntos_superpuestos_anidados_y_que_se_tocan_en_la_base() {
        let db = test_db();
        let disjuntos = proyecto(&db, "disjuntos");
        cerrado(&db, disjuntos, "work_button", "09:00:00", "10:00:00", H);
        cerrado(&db, disjuntos, "work_button", "11:00:00", "12:00:00", H);
        assert_eq!(trabajado(&db, disjuntos), 2 * H);

        let superpuestos = proyecto(&db, "superpuestos");
        cerrado(
            &db,
            superpuestos,
            "work_button",
            "09:00:00",
            "11:00:00",
            2 * H,
        );
        cerrado_de_pared(&db, superpuestos, "file_watch", "10:00:00", "12:00:00");
        assert_eq!(trabajado(&db, superpuestos), 3 * H);

        let anidados = proyecto(&db, "anidados");
        cerrado_de_pared(&db, anidados, "terminal", "09:00:00", "12:00:00");
        cerrado_de_pared(&db, anidados, "file_watch", "10:00:00", "11:00:00");
        assert_eq!(trabajado(&db, anidados), 3 * H);

        let se_tocan = proyecto(&db, "se tocan");
        cerrado_de_pared(&db, se_tocan, "terminal", "09:00:00", "10:00:00");
        cerrado_de_pared(&db, se_tocan, "file_watch", "10:00:00", "11:00:00");
        assert_eq!(trabajado(&db, se_tocan), 2 * H);
    }

    /// Una fila cerrada de otra fuente dura lo medido, no `ended_at − started_at`: un
    /// `ended_at` muy posterior (la suspensión dentro de la sesión) no amplía el tramo.
    #[test]
    fn ended_at_muy_posterior_no_amplia_el_tramo() {
        let db = test_db();
        let p = proyecto(&db, "p");
        cerrado(&db, p, "work_button", "09:00:00", "20:00:00", 1800);
        assert_eq!(trabajado(&db, p), 1800);
    }

    /// Regla de los 10 s: un `work_button` cerrado con 0 s no suma aunque `ended_at`
    /// esté 8 s después, y la `activa` con 5 s cuenta 0, lo mismo que se va a guardar.
    #[test]
    fn regla_de_los_10_s() {
        let db = test_db();
        let p = proyecto(&db, "p");
        cerrado(&db, p, "work_button", "09:00:00", "09:00:08", 0);
        assert_eq!(trabajado(&db, p), 0);

        let viva = abierto(&db, p, "work_button", "10:00:00", None, None);
        let activa = |segundos| {
            Some(SesionActiva {
                session_id: viva,
                segundos,
            })
        };
        assert_eq!(db.tiempo_trabajado(p, dia(), activa(5)).unwrap(), 0);
        assert_eq!(db.tiempo_trabajado(p, dia(), activa(11)).unwrap(), 11);
    }

    // --- Tramos abiertos de cada tipo ---

    #[test]
    fn abiertos_de_cada_tipo() {
        let db = test_db();

        // `file_watch`: hasta el último cambio volcado.
        let archivos = proyecto(&db, "archivos");
        abierto(
            &db,
            archivos,
            "file_watch",
            "09:00:00",
            Some("09:20:00"),
            None,
        );
        assert_eq!(trabajado(&db, archivos), 20 * 60);

        // `terminal`: hasta la última actividad.
        let terminal = proyecto(&db, "terminal");
        abierto(
            &db,
            terminal,
            "terminal",
            "09:00:00",
            Some("09:45:00"),
            Some(45 * 60),
        );
        assert_eq!(trabajado(&db, terminal), 45 * 60);

        // Otra fuente, con latido: lo medido hasta él, no la pared del latido.
        let con_latido = proyecto(&db, "con latido");
        abierto(
            &db,
            con_latido,
            "work_button",
            "09:00:00",
            Some("10:00:00"),
            Some(1800),
        );
        assert_eq!(trabajado(&db, con_latido), 1800);

        // Otra fuente, sin latido: 0 s.
        let sin_latido = proyecto(&db, "sin latido");
        abierto(
            &db,
            sin_latido,
            "work_button",
            "09:00:00",
            Some("09:00:00"),
            None,
        );
        assert_eq!(trabajado(&db, sin_latido), 0);
    }

    /// Un `file_watch` abierto llega hasta `last_activity_at`, no hasta "ahora" ni hasta
    /// el fin del rango, y no depende de `tracker_status` (M6).
    #[test]
    fn file_watch_abierto_hasta_last_activity_at() {
        let db = test_db();
        let p = proyecto(&db, "p");
        abierto(&db, p, "file_watch", "14:00:00", Some("14:07:30"), None);
        db.conn()
            .execute_batch(
                "INSERT INTO tracker_status (id, estado, pid, last_heartbeat)
                 VALUES (1, 'desactivado', NULL, '2026-10-20 08:00:00')",
            )
            .unwrap();
        assert_eq!(trabajado(&db, p), 450);
    }

    /// La `activa` cuenta los segundos del `Instant` (M5): tras una suspensión, la pared
    /// avanzó 3 h pero el `Instant` midió 1 h, y el latido guardó solo 10 min.
    #[test]
    fn activa_con_suspension_cuenta_los_segundos_del_instant() {
        let db = test_db();
        let p = proyecto(&db, "p");
        let viva = abierto(
            &db,
            p,
            "work_button",
            "09:00:00",
            Some("12:00:00"),
            Some(600),
        );

        let activa = SesionActiva {
            session_id: viva,
            segundos: H,
        };
        assert_eq!(db.tiempo_trabajado(p, dia(), Some(activa)).unwrap(), H);
        assert_eq!(
            trabajado(&db, p),
            600,
            "sin la activa, lo del último latido"
        );
    }

    /// La `activa` pasa el tope de 12 h: cuenta 12 h, lo que se va a guardar al cerrar.
    #[test]
    fn activa_respeta_el_tope_de_12_h() {
        let db = test_db();
        let p = proyecto(&db, "p");
        let viva = abierto(&db, p, "work_button", "00:00:00", None, None);
        let activa = SesionActiva {
            session_id: viva,
            segundos: 13 * H,
        };
        assert_eq!(
            db.tiempo_trabajado(p, dia(), Some(activa)).unwrap(),
            TOPE_SIN_TRACKER
        );
    }

    /// `activa` con el id de una fila ya cerrada (la cerró otro camino entre la copia y
    /// la consulta) se ignora: manda la fila cerrada.
    #[test]
    fn activa_con_id_cerrado_se_ignora() {
        let db = test_db();
        let p = proyecto(&db, "p");
        let cerrada = cerrado(&db, p, "work_button", "09:00:00", "09:30:00", 1800);
        let activa = SesionActiva {
            session_id: cerrada,
            segundos: 5 * H,
        };
        assert_eq!(db.tiempo_trabajado(p, dia(), Some(activa)).unwrap(), 1800);

        // Y nunca se aplica a la fila abierta de otro id.
        let otra = abierto(&db, p, "work_button", "11:00:00", None, Some(60));
        assert_eq!(db.tiempo_trabajado(p, dia(), Some(activa)).unwrap(), 1860);
        assert_ne!(otra, cerrada);
    }

    // --- Recorte al rango ---

    /// Una sesión que cruza la medianoche local suma a cada día lo suyo. En Buenos Aires
    /// (UTC−3), de 23:00 a 01:00 locales son 02:00 a 04:00 UTC.
    #[test]
    fn cruce_de_medianoche_reparte_entre_los_dos_dias() {
        let db = test_db();
        let p = proyecto(&db, "p");
        db.conn()
            .execute(
                "INSERT INTO time_tracking_sessions
                     (project_id, source, started_at, ended_at, duration_seconds)
                 VALUES (?1, 'terminal', '2026-10-21 02:00:00', '2026-10-21 04:00:00', 7200)",
                params![p],
            )
            .unwrap();
        let fecha = |d| NaiveDate::from_ymd_opt(2026, 10, d).unwrap();

        let martes = rango_dia(&Buenos_Aires, fecha(20));
        let miercoles = rango_dia(&Buenos_Aires, fecha(21));
        assert_eq!(db.tiempo_trabajado(p, martes, None).unwrap(), H);
        assert_eq!(db.tiempo_trabajado(p, miercoles, None).unwrap(), H);
        assert_eq!(
            db.tiempo_trabajado(p, rango_dia(&Buenos_Aires, fecha(22)), None)
                .unwrap(),
            0
        );
    }

    /// Un `work_button` cerrado con `ended_at` dentro del rango pero cuyo tramo medido
    /// termina antes de `desde` no suma: el filtro trae de más y el recorte lo deja en 0.
    #[test]
    fn el_filtro_trae_de_mas_y_el_recorte_lo_corrige() {
        let db = test_db();
        let p = proyecto(&db, "p");
        db.conn()
            .execute(
                "INSERT INTO time_tracking_sessions
                     (project_id, source, started_at, ended_at, duration_seconds)
                 VALUES (?1, 'work_button', '2026-10-19 22:00:00', '2026-10-20 02:00:00', 1800)",
                params![p],
            )
            .unwrap();
        assert_eq!(trabajado(&db, p), 0);
    }

    // --- R8 (3.10.8) ---

    /// Terminal de 9 a 18 con una suspensión de 12 a 14 (dos tramos) y `file_watch` de
    /// 16 a 18 → 7 h.
    #[test]
    fn terminal_con_suspension_y_file_watch_adentro_suman_7_h() {
        let db = test_db();
        let p = proyecto(&db, "p");
        cerrado_de_pared(&db, p, "terminal", "09:00:00", "12:00:00");
        cerrado_de_pared(&db, p, "terminal", "14:00:00", "18:00:00");
        cerrado_de_pared(&db, p, "file_watch", "16:00:00", "18:00:00");
        assert_eq!(trabajado(&db, p), 7 * H);
    }

    /// Pausa de 10 min (10:30 a 10:40) con un `file_watch` de 3 min adentro, que empieza
    /// 60 s antes del primer guardado (DX1): sin doble conteo y con el margen.
    #[test]
    fn pausa_con_file_watch_adentro_suma_el_margen_de_60_s() {
        let db = test_db();
        let p = proyecto(&db, "p");
        cerrado_de_pared(&db, p, "terminal", "10:00:00", "10:30:00");
        cerrado_de_pared(&db, p, "terminal", "10:40:00", "11:00:00");
        // Primer guardado a las 10:34, último a las 10:36: el tramo arranca a las 10:33.
        cerrado_de_pared(&db, p, "file_watch", "10:33:00", "10:36:00");
        assert_eq!(trabajado(&db, p), (30 + 20 + 3) * 60);

        // Un `file_watch` que se superpone con la terminal no suma de nuevo.
        cerrado_de_pared(&db, p, "file_watch", "10:25:00", "10:31:00");
        assert_eq!(trabajado(&db, p), (30 + 1 + 20 + 3) * 60);
    }

    /// A de 10 a 12 y B de 11 a 13 → 1 h simultánea en cada uno, y cada uno conserva
    /// sus 2 h (sin reparto).
    #[test]
    fn a_y_b_superpuestos_son_simultaneos_una_hora_sin_reparto() {
        let db = test_db();
        let a = proyecto(&db, "A");
        let b = proyecto(&db, "B");
        let c = proyecto(&db, "C");
        cerrado_de_pared(&db, a, "terminal", "10:00:00", "12:00:00");
        cerrado_de_pared(&db, b, "terminal", "11:00:00", "13:00:00");
        cerrado_de_pared(&db, c, "terminal", "15:00:00", "16:00:00");

        assert_eq!(db.simultaneidad(a, dia()).unwrap(), vec![(b, H)]);
        assert_eq!(db.simultaneidad(b, dia()).unwrap(), vec![(a, H)]);
        assert!(db.simultaneidad(c, dia()).unwrap().is_empty());
        assert_eq!(trabajado(&db, a), 2 * H);
        assert_eq!(trabajado(&db, b), 2 * H);
    }

    // --- Variante de una consulta ---

    /// La variante para todos los proyectos da lo mismo que la de uno, fuera de la
    /// papelera, y con la `activa` aplicada a su fila.
    #[test]
    fn por_proyecto_coincide_con_la_de_uno_y_excluye_la_papelera() {
        let db = test_db();
        let a = proyecto(&db, "A");
        let b = proyecto(&db, "B");
        let borrado = proyecto(&db, "Borrado");
        let sin_tramos = proyecto(&db, "Sin tramos");
        cerrado(&db, a, "work_button", "09:00:00", "10:00:00", H);
        cerrado_de_pared(&db, a, "file_watch", "09:30:00", "10:30:00");
        let viva = abierto(&db, b, "work_button", "11:00:00", None, None);
        cerrado(&db, borrado, "work_button", "09:00:00", "10:00:00", H);
        db.conn()
            .execute(
                "UPDATE projects SET deleted_at = '2026-10-20 12:00:00' WHERE id = ?1",
                params![borrado],
            )
            .unwrap();
        let activa = Some(SesionActiva {
            session_id: viva,
            segundos: 900,
        });

        let todos = db.tiempo_trabajado_por_proyecto(dia(), activa).unwrap();

        assert_eq!(todos.get(&a), Some(&(H + 1800)));
        assert_eq!(todos.get(&b), Some(&900));
        assert_eq!(todos.get(&borrado), None, "la papelera no entra");
        assert_eq!(todos.get(&sin_tramos), None);
        for p in [a, b] {
            assert_eq!(
                todos.get(&p).copied(),
                Some(db.tiempo_trabajado(p, dia(), activa).unwrap())
            );
        }
    }

    /// Una fecha que no se puede leer es un error, no 0 s.
    #[test]
    fn una_fecha_ilegible_es_un_error() {
        let db = test_db();
        let p = proyecto(&db, "p");
        db.conn()
            .execute(
                "INSERT INTO time_tracking_sessions
                     (project_id, source, started_at, ended_at, duration_seconds)
                 VALUES (?1, 'terminal', '2026-10-20T09:00:00Z', '2026-10-20 10:00:00', 3600)",
                params![p],
            )
            .unwrap();
        assert!(db.tiempo_trabajado(p, dia(), None).is_err());
    }
}
