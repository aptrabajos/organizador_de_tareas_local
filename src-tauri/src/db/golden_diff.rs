//! Diff con datos reales (5.4 del diseño del tracker, D3; commit 24).
//!
//! Compara, proyecto por proyecto, los números **viejos** (el SQL de `main@c8b55ae`,
//! copiado acá tal cual) con los **nuevos** (la unión de tramos en hora local, con los
//! ajustes), y lista aparte lo que la sección 9 decide caso por caso: las sesiones de
//! más de 12 h, las huérfanas, las sesiones de 1 a 10 s y los proyectos con carpeta
//! inexistente. **No toca nada**: la base de origen se abre de solo lectura y se copia
//! con la API de backup a un `tempdir`; todo lo demás, migración incluida, corre sobre
//! la copia.
//!
//! Es `#[cfg(test)]` y no un test de `tests/` porque la lib todavía no existe (llega
//! con F3). Los dos tests con datos reales llevan `#[ignore]`: corren solo a mano, nunca
//! en `check.sh` ni en CI, y leen la ruta de `GESTOR_GOLDEN_DB`:
//!
//! ```bash
//! GESTOR_GOLDEN_DB=/ruta/a/una/copia/projects.db \
//!   cargo test --manifest-path src-tauri/Cargo.toml golden_diff -- --ignored --nocapture
//! ```
//!
//! Los tests sin `#[ignore]` hacen lo mismo sobre una base sintética, para que el
//! informe no se rompa en silencio entre una corrida real y la siguiente.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use chrono::{TimeZone, Utc};
use rusqlite::{params, Connection, DatabaseName, OpenFlags, Result};
use tempfile::TempDir;

use super::Database;
use crate::time;

/// Variable de entorno con la ruta de la base a comparar.
const VARIABLE_DB: &str = "GESTOR_GOLDEN_DB";

/// Una sesión de más de esto se lista aparte (DX3).
const DOCE_HORAS: i64 = 43_200;

/// Copia `origen` a `destino` con la API de backup, desde una conexión de **solo
/// lectura**: el origen no se escribe ni se le crea un `-journal`.
fn copiar_de_solo_lectura(origen: &Path, destino: &Path) -> Result<()> {
    let conn = Connection::open_with_flags(
        origen,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.backup(DatabaseName::Main, destino, None)
}

/// Los números de un proyecto con el SQL de antes (`main@c8b55ae`, `db/tracking.rs` y
/// `db/activity.rs`), incluido el contador deprecado.
#[derive(Debug, Default, Clone, PartialEq)]
struct Viejos {
    nombre: String,
    en_papelera: bool,
    local_path: String,
    contador: i64,
    total: i64,
    sesiones: i64,
    promedio: i64,
    mas_larga: i64,
    hoy: i64,
    semana: i64,
}

/// El SQL de `get_time_stats` de `main@c8b55ae`, copiado: suma de duraciones, "hoy"
/// en UTC (`DATE('now')`) y la semana de 8 fechas (`DATE('now', '-7 days')`).
fn numeros_viejos(conn: &Connection) -> Result<BTreeMap<i64, Viejos>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, deleted_at IS NOT NULL, local_path, COALESCE(total_time_seconds, 0)
         FROM projects ORDER BY id",
    )?;
    let proyectos: Vec<(i64, String, bool, String, i64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?
        .collect::<Result<_>>()?;

    let mut viejos = BTreeMap::new();
    for (id, nombre, en_papelera, local_path, contador) in proyectos {
        let (total, sesiones): (i64, i64) = conn.query_row(
            "SELECT COALESCE(SUM(duration_seconds), 0), COUNT(*)
             FROM time_tracking_sessions
             WHERE project_id = ?1 AND duration_seconds IS NOT NULL",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let promedio = if sesiones > 0 { total / sesiones } else { 0 };
        let mas_larga: i64 = conn.query_row(
            "SELECT COALESCE(MAX(duration_seconds), 0)
             FROM time_tracking_sessions
             WHERE project_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        let hoy: i64 = conn.query_row(
            "SELECT COALESCE(SUM(duration_seconds), 0)
             FROM time_tracking_sessions
             WHERE project_id = ?1 AND DATE(started_at) = DATE('now')",
            params![id],
            |row| row.get(0),
        )?;
        let semana: i64 = conn.query_row(
            "SELECT COALESCE(SUM(duration_seconds), 0)
             FROM time_tracking_sessions
             WHERE project_id = ?1 AND started_at >= DATE('now', '-7 days')",
            params![id],
            |row| row.get(0),
        )?;
        viejos.insert(
            id,
            Viejos {
                nombre,
                en_papelera,
                local_path,
                contador,
                total,
                sesiones,
                promedio,
                mas_larga,
                hoy,
                semana,
            },
        );
    }
    Ok(viejos)
}

/// El total del Dashboard de antes: la suma del contador fuera de la papelera.
fn dashboard_viejo(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(total_time_seconds), 0) FROM projects WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )
}

/// Filas `(id, project_id, source, started_at, duration_seconds)` de una consulta de
/// sesiones, para las listas aparte.
type FilaSesion = (i64, i64, Option<String>, String, Option<i64>);

fn sesiones_donde(conn: &Connection, condicion: &str) -> Result<Vec<FilaSesion>> {
    conn.prepare(&format!(
        "SELECT id, project_id, source, started_at, duration_seconds
         FROM time_tracking_sessions WHERE {condicion} ORDER BY id"
    ))?
    .query_map([], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
        ))
    })?
    .collect()
}

fn escribir_sesiones(salida: &mut String, titulo: &str, filas: &[FilaSesion]) {
    let suma: i64 = filas.iter().filter_map(|f| f.4).sum();
    let _ = writeln!(salida, "\n== {titulo}: {} (suman {suma} s) ==", filas.len());
    for (id, project_id, source, started_at, duracion) in filas {
        let _ = writeln!(
            salida,
            "  sesión {id} · proyecto {project_id} · {} · desde {started_at} · {}",
            source.as_deref().unwrap_or("-"),
            duracion.map_or("sin duración".to_string(), |d| format!("{d} s"))
        );
    }
}

/// Arma el informe completo sobre una copia de `origen` en `tz`. No escribe `origen`.
fn informe<Tz: TimeZone>(origen: &Path, tz: &Tz) -> Result<String> {
    let dir = TempDir::new().map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let copia = dir.path().join("projects.db");
    copiar_de_solo_lectura(origen, &copia)?;

    // Lo viejo y las listas, sobre la copia sin migrar: el SQL de antes ve la base
    // como la veía la app de antes.
    let (viejos, dashboard_viejo, version_original, largas, huerfanas, cortas) = {
        let conn = Connection::open(&copia)?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        (
            numeros_viejos(&conn)?,
            dashboard_viejo(&conn)?,
            version,
            sesiones_donde(&conn, &format!("duration_seconds > {DOCE_HORAS}"))?,
            sesiones_donde(&conn, "ended_at IS NULL")?,
            sesiones_donde(&conn, "duration_seconds BETWEEN 1 AND 10")?,
        )
    };

    // Lo nuevo: la copia migrada por la app, como la va a abrir.
    let db = Database::new(copia.clone())?;
    let hoy = Utc::now().with_timezone(tz).date_naive();
    let rango_hoy = time::rango_dia(tz, hoy);
    let rango_semana = time::rango_semana(tz, hoy);

    let mut salida = String::new();
    let _ = writeln!(
        salida,
        "Diff con datos reales: {} (user_version {version_original}), hoy local {hoy}",
        origen.display()
    );
    let _ = writeln!(
        salida,
        "\n== Diferencias por proyecto (viejo → nuevo; solo las que cambian) =="
    );
    let mut diferentes = 0;
    for (id, viejo) in &viejos {
        let nuevo = db.get_time_stats(*id, rango_hoy, rango_semana, None)?;
        let pares = [
            ("contador", viejo.contador, nuevo.total_seconds),
            ("total", viejo.total, nuevo.total_seconds),
            ("sesiones", viejo.sesiones, nuevo.session_count),
            ("promedio", viejo.promedio, nuevo.avg_session_seconds),
            ("más larga", viejo.mas_larga, nuevo.longest_session_seconds),
            ("hoy (UTC → local)", viejo.hoy, nuevo.today_seconds),
            ("semana (8 → 7 fechas)", viejo.semana, nuevo.week_seconds),
        ];
        let cambios: Vec<String> = pares
            .iter()
            .filter(|(_, antes, despues)| antes != despues)
            .map(|(campo, antes, despues)| format!("{campo}: {antes} → {despues}"))
            .collect();
        if cambios.is_empty() {
            continue;
        }
        diferentes += 1;
        let _ = writeln!(
            salida,
            "  proyecto {id} «{}»{}: {}",
            viejo.nombre,
            if viejo.en_papelera {
                " (en la papelera)"
            } else {
                ""
            },
            cambios.join(" · ")
        );
    }
    let _ = writeln!(
        salida,
        "  {diferentes} de {} proyectos con alguna diferencia",
        viejos.len()
    );

    let stats = db.get_project_stats(rango_hoy, None)?;
    let _ = writeln!(
        salida,
        "\n== Dashboard ==\n  total: {dashboard_viejo} s (contador) → {:.0} s (unión + ajustes)",
        stats.total_time_hours * 3600.0
    );

    escribir_sesiones(&mut salida, "Sesiones de más de 12 h", &largas);
    escribir_sesiones(&mut salida, "Huérfanas (abiertas)", &huerfanas);
    escribir_sesiones(&mut salida, "Sesiones de 1 a 10 s", &cortas);

    let sin_carpeta: Vec<(&i64, &Viejos)> = viejos
        .iter()
        .filter(|(_, v)| !Path::new(&v.local_path).exists())
        .collect();
    let _ = writeln!(
        salida,
        "\n== Proyectos con carpeta inexistente: {} ==",
        sin_carpeta.len()
    );
    for (id, v) in sin_carpeta {
        let _ = writeln!(
            salida,
            "  proyecto {id} «{}»{} · {}",
            v.nombre,
            if v.en_papelera {
                " (en la papelera)"
            } else {
                ""
            },
            v.local_path
        );
    }

    Ok(salida)
}

/// Lo que cambia la recuperación de huérfanas del commit 19 sobre una copia de
/// `origen`: total de segundos antes y después, cuántas cerró y con qué, y si tocó
/// `trabajar_sesiones`. No escribe `origen`.
#[derive(Debug, PartialEq, Eq)]
struct EnsayoHuerfanas {
    abiertas_antes: i64,
    work_button_abiertas: i64,
    cerradas: usize,
    segundos_antes: i64,
    segundos_despues: i64,
    /// Las `work_button` que estaban abiertas, agrupadas por (duración, motivo).
    cierres: BTreeMap<(Option<i64>, Option<String>), i64>,
    sesiones_de_trabajar_iguales: bool,
}

fn ensayo_huerfanas(origen: &Path) -> Result<EnsayoHuerfanas> {
    let dir = TempDir::new().map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let copia = dir.path().join("projects.db");
    copiar_de_solo_lectura(origen, &copia)?;
    let db = Database::new(copia)?;

    let contar = |sql: &str| -> Result<i64> { db.conn().query_row(sql, [], |row| row.get(0)) };
    let segundos =
        || contar("SELECT COALESCE(SUM(duration_seconds), 0) FROM time_tracking_sessions");
    let sesiones_de_trabajar = || -> Result<Vec<String>> {
        db.conn()
            .prepare(
                "SELECT token || '|' || estado || '|' || dueno || '|' ||
                        COALESCE(terminada_at, '') || '|' || COALESCE(motivo_cierre, '')
                 FROM trabajar_sesiones ORDER BY token",
            )?
            .query_map([], |row| row.get(0))?
            .collect()
    };

    let abiertas_antes =
        contar("SELECT COUNT(*) FROM time_tracking_sessions WHERE ended_at IS NULL")?;
    let ids: Vec<i64> = db
        .conn()
        .prepare(
            "SELECT id FROM time_tracking_sessions
             WHERE ended_at IS NULL AND source = 'work_button'",
        )?
        .query_map([], |row| row.get(0))?
        .collect::<Result<_>>()?;
    let segundos_antes = segundos()?;
    let trabajar_antes = sesiones_de_trabajar()?;

    // Con la adopción imposible, como en F1–F5.
    let cerradas = db.recuperar_huerfanas_app(Utc::now(), true)?;

    let mut cierres = BTreeMap::new();
    for id in &ids {
        let clave: (Option<i64>, Option<String>) = db.conn().query_row(
            "SELECT duration_seconds, motivo_cierre FROM time_tracking_sessions WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        *cierres.entry(clave).or_insert(0) += 1;
    }

    Ok(EnsayoHuerfanas {
        abiertas_antes,
        work_button_abiertas: ids.len() as i64,
        cerradas,
        segundos_antes,
        segundos_despues: segundos()?,
        cierres,
        sesiones_de_trabajar_iguales: trabajar_antes == sesiones_de_trabajar()?,
    })
}

/// La ruta de `GESTOR_GOLDEN_DB`. Sin ella, el test falla con un mensaje claro en vez de
/// buscar una base por su cuenta.
fn base_golden() -> std::path::PathBuf {
    let ruta = std::env::var_os(VARIABLE_DB).unwrap_or_else(|| {
        panic!("Definí {VARIABLE_DB} con la ruta de una copia de projects.db (5.4 del diseño del tracker)")
    });
    let ruta = std::path::PathBuf::from(ruta);
    assert!(
        ruta.is_file(),
        "{VARIABLE_DB} no apunta a un archivo: {}",
        ruta.display()
    );
    ruta
}

/// El contenido de un archivo, para afirmar que nadie lo escribió.
fn bytes(ruta: &Path) -> Vec<u8> {
    std::fs::read(ruta).expect("no se pudo leer el archivo")
}

#[test]
#[ignore = "datos reales: corre a mano con GESTOR_GOLDEN_DB (5.4)"]
fn golden_diff_con_datos_reales() {
    let origen = base_golden();
    let antes = bytes(&origen);
    let salida = informe(&origen, &chrono::Local).expect("no se pudo armar el diff");
    println!("{salida}");
    assert_eq!(
        bytes(&origen),
        antes,
        "el diff no puede tocar la base de origen"
    );
}

#[test]
#[ignore = "datos reales: corre a mano con GESTOR_GOLDEN_DB (3.10.1)"]
fn golden_diff_huerfanas_con_datos_reales() {
    let origen = base_golden();
    let antes = bytes(&origen);
    let ensayo = ensayo_huerfanas(&origen).expect("no se pudo ensayar la recuperación");
    println!("{ensayo:#?}");
    assert_eq!(
        bytes(&origen),
        antes,
        "el ensayo no puede tocar la base de origen"
    );

    assert_eq!(ensayo.cerradas as i64, ensayo.work_button_abiertas);
    assert_eq!(
        ensayo.segundos_antes, ensayo.segundos_despues,
        "la recuperación no cambia el total de segundos"
    );
    assert!(
        ensayo.sesiones_de_trabajar_iguales,
        "trabajar_sesiones no cambia"
    );
    for (duracion, motivo) in ensayo.cierres.keys() {
        assert_eq!(
            duracion,
            &Some(0),
            "las huérfanas sin latido cierran con 0 s"
        );
        assert_eq!(motivo.as_deref(), Some("huerfana"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Una base sintética con un poco de cada cosa que el informe tiene que mostrar:
    /// el contador desfasado, una sesión de más de 12 h, una huérfana, una de 5 s y un
    /// proyecto con la carpeta inexistente. Queda en `user_version` 0 (el esquema
    /// viejo), como la base real antes de F1.
    fn base_sintetica(ruta: &Path) {
        let conn = Connection::open(ruta).unwrap();
        conn.execute_batch(
            "CREATE TABLE projects (
                 id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL,
                 description TEXT NOT NULL, local_path TEXT NOT NULL,
                 total_time_seconds INTEGER DEFAULT 0, deleted_at TEXT);
             CREATE TABLE time_tracking_sessions (
                 id INTEGER PRIMARY KEY AUTOINCREMENT, project_id INTEGER NOT NULL,
                 started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, ended_at DATETIME,
                 duration_seconds INTEGER, source TEXT DEFAULT 'shell_hook',
                 created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                 FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE);
             INSERT INTO projects (id, name, description, local_path, total_time_seconds)
             VALUES (1, 'Uno', 'd', '/tmp', 3600),
                    (3, 'Tres', 'd', '/no/existe/golden-diff', 100);
             INSERT INTO time_tracking_sessions
                 (project_id, started_at, ended_at, duration_seconds, source)
             VALUES (1, '2026-01-10 10:00:00', '2026-01-10 11:00:00', 3600, 'work_button'),
                    (3, '2026-01-11 10:00:00', '2026-01-12 00:00:00', 50000, 'work_button'),
                    (3, '2026-01-13 10:00:00', '2026-01-13 10:00:05', 5, 'work_button');
             INSERT INTO time_tracking_sessions (project_id, started_at, source)
             VALUES (3, '2026-01-14 10:00:00', 'work_button');",
        )
        .unwrap();
    }

    #[test]
    fn el_informe_lista_las_diferencias_y_las_cuatro_partes_sin_tocar_el_origen() {
        let dir = TempDir::new().unwrap();
        let origen = dir.path().join("origen.db");
        base_sintetica(&origen);
        let antes = bytes(&origen);

        let salida = informe(&origen, &Utc).unwrap();

        assert_eq!(bytes(&origen), antes, "el origen no se toca");
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            1,
            "ni un -journal al lado del origen"
        );
        assert!(salida.contains("(user_version 0)"), "{salida}");
        assert!(
            salida.contains("proyecto 3 «Tres»: contador: 100 → 50005"),
            "{salida}"
        );
        assert!(
            !salida.contains("proyecto 1 «Uno»"),
            "sin diferencias no se lista: {salida}"
        );
        assert!(
            salida.contains("1 de 2 proyectos con alguna diferencia"),
            "{salida}"
        );
        assert!(
            salida.contains("total: 3700 s (contador) → 53605 s (unión + ajustes)"),
            "{salida}"
        );
        assert!(
            salida.contains("== Sesiones de más de 12 h: 1 (suman 50000 s) =="),
            "{salida}"
        );
        assert!(salida.contains("== Huérfanas (abiertas): 1"), "{salida}");
        assert!(
            salida.contains("== Sesiones de 1 a 10 s: 1 (suman 5 s) =="),
            "{salida}"
        );
        assert!(
            salida.contains("== Proyectos con carpeta inexistente: 1 ==")
                && salida.contains("/no/existe/golden-diff"),
            "{salida}"
        );
    }

    #[test]
    fn el_ensayo_de_huerfanas_cierra_con_0_s_sin_cambiar_el_total_ni_tocar_el_origen() {
        let dir = TempDir::new().unwrap();
        let origen = dir.path().join("origen.db");
        base_sintetica(&origen);
        let antes = bytes(&origen);

        let ensayo = ensayo_huerfanas(&origen).unwrap();

        assert_eq!(bytes(&origen), antes);
        assert_eq!(
            ensayo,
            EnsayoHuerfanas {
                abiertas_antes: 1,
                work_button_abiertas: 1,
                cerradas: 1,
                segundos_antes: 53_605,
                segundos_despues: 53_605,
                cierres: BTreeMap::from([((Some(0), Some("huerfana".to_string())), 1)]),
                sesiones_de_trabajar_iguales: true,
            }
        );
    }

    #[test]
    fn copiar_de_solo_lectura_falla_sin_crear_el_origen() {
        let dir = TempDir::new().unwrap();
        let origen = dir.path().join("no_existe.db");
        assert!(copiar_de_solo_lectura(&origen, &dir.path().join("copia.db")).is_err());
        assert!(!origen.exists());
    }
}
