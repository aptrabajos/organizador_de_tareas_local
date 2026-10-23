//! Sesiones de Trabajar atadas a la terminal (R8-1, 3.10 del diseño del tracker).
//!
//! "Sesión" es un **token** (una fila de `trabajar_sesiones`) y "tramo" es una fila de
//! `time_tracking_sessions` con ese `terminal_token`. La base es la fuente de verdad
//! de las sesiones (DT2): la app, el demonio y la CLI escriben acá, cada uno con su
//! conexión.
//!
//! Todos los métodos corren en una transacción `IMMEDIATE` y son **condicionales**:
//! solo tocan una sesión que no esté `cerrada` y un tramo con `ended_at IS NULL`. Si
//! dos procesos cierran la misma sesión a la vez (Parar contra "terminal cerrada"),
//! gana uno y el otro recibe `false` o `None` y suelta el token (M12). La
//! transacción `IMMEDIATE` toma el lock de escritura antes de leer, así que la
//! comprobación y la escritura ven la misma base.
//!
//! Sin llamadores hasta Trabajar y Parar (commit 17) y el escritor del demonio
//! (commit 44): el `allow(dead_code)` de cada método lo dice.

use chrono::{DateTime, Duration, Utc};
use rusqlite::types::Type;
use rusqlite::{params, Connection, OptionalExtension, Result, Transaction, TransactionBehavior};

use super::tracking::cerrar_sesion_en;
use super::Database;
use crate::commands::duracion_a_guardar;
use crate::time;

/// Fuente de los tramos que mide el demonio para un token (`source = 'terminal'`).
/// Los tramos de un token `dueno = app` siguen siendo `work_button`.
pub(crate) const FUENTE_TERMINAL: &str = "terminal";

/// Tope de un tramo medido por la app sin el demonio (U6). La constante de la app
/// (`TOPE_SIN_TRACKER`) llega con el commit 17; acá la usa solo el cierre de un tramo
/// `work_button` sin el `Instant` de la app, con la regla de huérfanas (§3.2).
const TOPE_SIN_TRACKER_S: i64 = 43_200;

/// El demonio está vivo si su último latido tiene menos que esto (3.10.7).
const LATIDO_VIGENTE_S: i64 = 45;

/// Largo de un minuto de evidencia en el formato de la base: `"2026-10-20 14:37"`.
const LARGO_MINUTO: usize = 16;

/// Por qué terminó una sesión (3.10.15). Es la lista del `CHECK` de
/// `trabajar_sesiones.motivo_cierre`.
#[allow(dead_code)] // Sin llamadores hasta Trabajar y Parar (commit 17)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotivoSesion {
    TerminalCerrada,
    PararApp,
    PararCli,
    TrackerApagado,
    TrackerCaido,
    ProyectoAPapelera,
    Restauracion,
    CambioDeProyecto,
    CierreApp,
    Huerfana,
    TopeSinTracker,
}

impl MotivoSesion {
    pub fn como_texto(self) -> &'static str {
        match self {
            MotivoSesion::TerminalCerrada => "terminal_cerrada",
            MotivoSesion::PararApp => "parar_app",
            MotivoSesion::PararCli => "parar_cli",
            MotivoSesion::TrackerApagado => "tracker_apagado",
            MotivoSesion::TrackerCaido => "tracker_caido",
            MotivoSesion::ProyectoAPapelera => "proyecto_a_papelera",
            MotivoSesion::Restauracion => "restauracion",
            MotivoSesion::CambioDeProyecto => "cambio_de_proyecto",
            MotivoSesion::CierreApp => "cierre_app",
            MotivoSesion::Huerfana => "huerfana",
            MotivoSesion::TopeSinTracker => "tope_sin_tracker",
        }
    }

    /// La "fórmula del motivo" de 3.10.15 sale de la propia sesión (`last_seen_at`,
    /// o `creada_at` si nunca se vio) y no del instante que pasa el llamador.
    fn termina_en_la_ultima_vista(self) -> bool {
        matches!(
            self,
            MotivoSesion::TerminalCerrada
                | MotivoSesion::TrackerApagado
                | MotivoSesion::TrackerCaido
                | MotivoSesion::Restauracion
        )
    }

    /// `motivo_salida` de las anclas que siguen abiertas cuando la sesión termina
    /// (3.10.1). Las de shells muertas ya las cerró `cerrar_reclamos_muertos` con
    /// `shell_cerrada`, que el demonio corre en cada escaneo antes de cerrar nada.
    fn motivo_salida_de_anclas(self) -> &'static str {
        match self {
            MotivoSesion::PararApp | MotivoSesion::PararCli => "parar",
            MotivoSesion::CambioDeProyecto => "cambio_de_proyecto",
            _ => "sesion_terminada",
        }
    }
}

/// Por qué se cortó un tramo (3.10.15): los motivos de sesión, más los cortes que no
/// terminan la sesión. Es la lista del `CHECK` de `time_tracking_sessions.motivo_cierre`.
#[allow(dead_code)] // Los cortes que no terminan la sesión llegan con el escritor (commit 44)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotivoTramo {
    /// El tramo se corta porque termina la sesión: los dos llevan el mismo valor.
    Sesion(MotivoSesion),
    Inactividad,
    Suspension,
    Reloj,
    TrackerDetenido,
    /// Solo `file_watch`: la carpeta dejó de vigilarse.
    SinVigilancia,
}

impl MotivoTramo {
    pub fn como_texto(self) -> &'static str {
        match self {
            MotivoTramo::Sesion(motivo) => motivo.como_texto(),
            MotivoTramo::Inactividad => "inactividad",
            MotivoTramo::Suspension => "suspension",
            MotivoTramo::Reloj => "reloj",
            MotivoTramo::TrackerDetenido => "tracker_detenido",
            MotivoTramo::SinVigilancia => "sin_vigilancia",
        }
    }
}

/// Un minuto de evidencia (DT12, U1): qué se contó y qué se ignoró en ese minuto.
#[allow(dead_code)] // Sin llamadores hasta el escritor del demonio (commit 44)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Evidencia<'a> {
    /// Cualquier instante del minuto: se guarda truncado a `'%Y-%m-%d %H:%M'`.
    pub minuto: DateTime<Utc>,
    /// Máscara con los bits de abajo.
    pub fuentes: i64,
    /// Ruta **relativa a la carpeta del proyecto** del primer cambio del minuto.
    pub archivo_rel: Option<&'a str>,
    /// Segundos (0–59) del primer y del último cambio sin presencia (bit 32).
    pub ausente: Option<(i64, i64)>,
}

#[allow(dead_code)] // Sin llamadores hasta el escritor del demonio (commit 44)
impl Evidencia<'_> {
    pub const TIPEO: i64 = 1;
    pub const ARCHIVO_CON_PRESENCIA: i64 = 2;
    pub const REMOTO: i64 = 4;
    pub const ARCHIVO_SIN_DATO_DE_PRESENCIA: i64 = 8;
    pub const TIPEO_FUERA_DE_LA_CARPETA: i64 = 16;
    pub const ARCHIVO_SIN_PRESENCIA: i64 = 32;
}

/// UPSERT de un minuto de evidencia: OR de los bits, el primer archivo y el mínimo y
/// el máximo de los segundos sin presencia. `MIN`/`MAX` de varios argumentos dan
/// `NULL` si alguno lo es, de ahí el `COALESCE` (3.10.1).
fn sql_upsert_evidencia(tabla: &str, clave: &str) -> String {
    format!(
        "INSERT INTO {tabla} ({clave}, minuto, fuentes, archivo_rel, ausente_desde_s, ausente_hasta_s)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT({clave}, minuto) DO UPDATE SET
             fuentes = fuentes | excluded.fuentes,
             archivo_rel = COALESCE(archivo_rel, excluded.archivo_rel),
             ausente_desde_s = COALESCE(MIN(ausente_desde_s, excluded.ausente_desde_s),
                                        ausente_desde_s, excluded.ausente_desde_s),
             ausente_hasta_s = COALESCE(MAX(ausente_hasta_s, excluded.ausente_hasta_s),
                                        ausente_hasta_s, excluded.ausente_hasta_s)"
    )
}

fn registrar_evidencia_en(
    tx: &Transaction,
    tabla: &str,
    clave: &str,
    valor_clave: &dyn rusqlite::ToSql,
    evidencia: &Evidencia,
) -> Result<()> {
    let mut minuto = time::formatear(evidencia.minuto);
    minuto.truncate(LARGO_MINUTO);
    tx.execute(
        &sql_upsert_evidencia(tabla, clave),
        params![
            valor_clave,
            minuto,
            evidencia.fuentes,
            evidencia.archivo_rel,
            evidencia.ausente.map(|(desde, _)| desde),
            evidencia.ausente.map(|(_, hasta)| hasta),
        ],
    )?;
    Ok(())
}

/// Lee una fecha de la base como error de rusqlite si no tiene el formato único.
fn leer_instante(texto: &str) -> Result<DateTime<Utc>> {
    time::parsear(texto)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(e)))
}

/// `(id, source, started_at, last_activity_at, duration_seconds)` de un tramo abierto.
type TramoAbierto = (i64, Option<String>, String, Option<String>, Option<i64>);

/// Cierra **en la última actividad** (2a) el tramo abierto del token, si lo hay.
///
/// - `terminal`: `ended_at = last_activity_at` y `duration = ended − started`. Un
///   tramo de 0 s se borra (DT5).
/// - Cualquier otra fuente (un `work_button` que la app no cerró con su `Instant`):
///   la regla de huérfanas de §3.2, `MIN(COALESCE(duration, 0), ended − started,
///   12 h)`, con la regla de los 10 s.
fn cerrar_tramo_en_ua(tx: &Transaction, token: &str, motivo: MotivoTramo) -> Result<()> {
    let abierto: Option<TramoAbierto> = tx
        .query_row(
            "SELECT id, source, started_at, last_activity_at, duration_seconds
             FROM time_tracking_sessions
             WHERE terminal_token = ?1 AND ended_at IS NULL",
            params![token],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .optional()?;
    let Some((id, source, started_at, last_activity_at, medido)) = abierto else {
        return Ok(());
    };

    let inicio = leer_instante(&started_at)?;
    let fin = match last_activity_at {
        Some(texto) => leer_instante(&texto)?,
        None => inicio,
    };
    let bruto = (fin - inicio).num_seconds().max(0);
    let source = source.unwrap_or_default();
    let duracion = if source == FUENTE_TERMINAL {
        bruto
    } else {
        duracion_a_guardar(
            &source,
            medido.unwrap_or(0).min(bruto).min(TOPE_SIN_TRACKER_S),
        )
    };

    cerrar_sesion_en(tx, id, fin, duracion, Some(motivo))?;
    Ok(())
}

/// Termina una sesión viva: cierra su tramo abierto en la última actividad, la pasa
/// a `cerrada` con `terminada_at` por la **regla única** de 3.10.15 y cierra sus
/// anclas abiertas, todo en la transacción del llamador. `false` si ya estaba
/// cerrada o no existe.
///
/// Regla única: `terminada_at = MAX(<fórmula del motivo>, MAX(ended_at) de sus
/// tramos)`. Así nunca queda antes del fin de un tramo, aunque `last_seen_at` falte o
/// sea viejo (invariante 9 de §6.2).
fn terminar_token_en(
    tx: &Transaction,
    token: &str,
    fin: DateTime<Utc>,
    motivo: MotivoSesion,
) -> Result<bool> {
    let viva: Option<(String, Option<String>)> = tx
        .query_row(
            "SELECT creada_at, last_seen_at FROM trabajar_sesiones
             WHERE token = ?1 AND estado <> 'cerrada'",
            params![token],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((creada_at, last_seen_at)) = viva else {
        return Ok(false);
    };

    cerrar_tramo_en_ua(tx, token, MotivoTramo::Sesion(motivo))?;

    let formula = if motivo.termina_en_la_ultima_vista() {
        leer_instante(last_seen_at.as_deref().unwrap_or(&creada_at))?
    } else {
        fin
    };
    let fin_del_ultimo_tramo: Option<String> = tx.query_row(
        "SELECT MAX(ended_at) FROM time_tracking_sessions WHERE terminal_token = ?1",
        params![token],
        |row| row.get(0),
    )?;
    let terminada_at = match fin_del_ultimo_tramo {
        Some(texto) => formula.max(leer_instante(&texto)?),
        None => formula,
    };
    let terminada_at = time::formatear(terminada_at);

    tx.execute(
        "UPDATE trabajar_sesiones
         SET estado = 'cerrada', terminada_at = ?2, motivo_cierre = ?3
         WHERE token = ?1 AND estado <> 'cerrada'",
        params![token, terminada_at, motivo.como_texto()],
    )?;
    tx.execute(
        "UPDATE trabajar_anclas SET hasta = ?2, motivo_salida = ?3
         WHERE token = ?1 AND hasta IS NULL",
        params![token, terminada_at, motivo.motivo_salida_de_anclas()],
    )?;

    Ok(true)
}

impl Database {
    /// Corre `f` en una transacción `IMMEDIATE`: el lock de escritura se toma antes de
    /// la primera lectura, así que lo que `f` comprueba sigue valiendo cuando escribe.
    fn en_transaccion_inmediata<T>(&self, f: impl FnOnce(&Transaction) -> Result<T>) -> Result<T> {
        let mut conn = self.conn();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let resultado = f(&tx)?;
        tx.commit()?;
        Ok(resultado)
    }

    /// Abre un tramo de `source` para el token en `inicio`, con
    /// `last_activity_at = inicio`. Comprueba en la misma transacción que el token
    /// siga vivo: si un Parar llegó en el medio, devuelve `None` y no deja un tramo
    /// huérfano. Un tramo `terminal` nace con `duration_seconds = 0`, para que valga
    /// desde el principio `last_activity_at − started_at = duration` (DT1).
    ///
    /// Un segundo tramo abierto del mismo token es un error del índice único
    /// `idx_sessions_tramo_abierto`: serían dos escritores.
    #[allow(dead_code)] // Sin llamadores hasta Trabajar (commit 17) y el escritor del demonio (commit 44)
    pub fn abrir_tramo(
        &self,
        token: &str,
        source: &str,
        inicio: DateTime<Utc>,
    ) -> Result<Option<i64>> {
        self.en_transaccion_inmediata(|tx| {
            let abiertos = tx.execute(
                "INSERT INTO time_tracking_sessions
                     (project_id, started_at, last_activity_at, duration_seconds, source, terminal_token)
                 SELECT project_id, ?2, ?2, CASE WHEN ?3 = 'terminal' THEN 0 END, ?3, token
                 FROM trabajar_sesiones
                 WHERE token = ?1 AND estado <> 'cerrada'",
                params![token, time::formatear(inicio), source],
            )?;
            Ok((abiertos > 0).then(|| tx.last_insert_rowid()))
        })
    }

    /// Termina las sesiones vivas de esos proyectos con `motivo` (Parar, papelera,
    /// kill switch, restauración…). `fin` es el instante de la fórmula del motivo
    /// para los que no la sacan de la propia sesión (el clic, el comando, el momento
    /// de mandar a la papelera). Devuelve `true` si terminó alguna.
    ///
    /// Un tramo `work_button` que la app sigue midiendo lo cierra antes la app, con su
    /// `Instant`; si quedó abierto, se cierra con la regla de huérfanas.
    #[allow(dead_code)] // Sin llamadores hasta Parar (commit 17) y la papelera (commit 18)
    pub fn terminar_sesiones_de(
        &self,
        project_ids: &[i64],
        fin: DateTime<Utc>,
        motivo: MotivoSesion,
    ) -> Result<bool> {
        self.en_transaccion_inmediata(|tx| {
            let mut tokens = Vec::new();
            {
                let mut stmt = tx.prepare(
                    "SELECT token FROM trabajar_sesiones
                     WHERE project_id = ?1 AND estado <> 'cerrada'",
                )?;
                for project_id in project_ids {
                    for token in
                        stmt.query_map(params![project_id], |row| row.get::<_, String>(0))?
                    {
                        tokens.push(token?);
                    }
                }
            }

            let mut alguna = false;
            for token in &tokens {
                alguna |= terminar_token_en(tx, token, fin, motivo)?;
            }
            Ok(alguna)
        })
    }

    /// Traspaso `app → demonio` (3.10.7): la sesión queda `esperando_demonio`, sin
    /// `adoptada_at`, para que el demonio la adopte. `dueno` nunca cambia con un tramo
    /// abierto (DT14): si el token tiene uno, o ya está cerrado, devuelve `false` y no
    /// toca nada.
    #[allow(dead_code)] // Sin llamadores hasta Trabajar en otro proyecto y el cierre de la app (commit 17)
    pub fn pasar_a_esperando_demonio(&self, token: &str) -> Result<bool> {
        self.en_transaccion_inmediata(|tx| {
            let pasadas = tx.execute(
                "UPDATE trabajar_sesiones
                 SET dueno = 'demonio', estado = 'esperando_demonio', adoptada_at = NULL
                 WHERE token = ?1 AND estado <> 'cerrada'
                   AND NOT EXISTS (SELECT 1 FROM time_tracking_sessions
                                   WHERE terminal_token = ?1 AND ended_at IS NULL)",
                params![token],
            )?;
            Ok(pasadas > 0)
        })
    }

    /// El demonio termina la sesión con `terminal_cerrada` porque el escaneo que
    /// empezó en `escaneo` no vio ninguna terminal ni ancla viva (3.10.2).
    ///
    /// Cierre **condicional, sin carrera**: no cierra si, después de ese escaneo,
    /// Trabajar reutilizó el token (`ultimo_clic_at > escaneo`) o la CLI ancló una
    /// shell nueva (`desde > escaneo`); esa terminal nueva el escaneo no la podía
    /// ver. Tampoco cierra una sesión `dueno = app`, que el demonio no mide. En esos
    /// casos devuelve `false` y el demonio re-escanea en el sondeo siguiente.
    #[allow(dead_code)] // Sin llamadores hasta el escritor del demonio (commit 44)
    pub fn cerrar_por_terminal(&self, token: &str, escaneo: DateTime<Utc>) -> Result<bool> {
        self.en_transaccion_inmediata(|tx| {
            let escaneo_texto = time::formatear(escaneo);
            let se_puede: bool = tx.query_row(
                "SELECT EXISTS (
                     SELECT 1 FROM trabajar_sesiones s
                     WHERE s.token = ?1 AND s.estado <> 'cerrada' AND s.dueno = 'demonio'
                       AND s.ultimo_clic_at <= ?2
                       AND NOT EXISTS (SELECT 1 FROM trabajar_anclas a
                                       WHERE a.token = s.token AND a.hasta IS NULL
                                         AND a.desde > ?2))",
                params![token, escaneo_texto],
                |row| row.get(0),
            )?;
            if !se_puede {
                return Ok(false);
            }
            terminar_token_en(tx, token, escaneo, MotivoSesion::TerminalCerrada)
        })
    }

    /// Cierra con `shell_cerrada` los reclamos vigentes (anclas y vetos) de las shells
    /// `(pid, starttime)` que el llamador vio muertas. Un reclamo cuenta solo mientras
    /// su shell vive: los números de pty se reutilizan enseguida (3.10.1). Devuelve
    /// cuántos cerró.
    #[allow(dead_code)] // Sin llamadores hasta el escaneo del demonio (commit 44) y la CLI (commit 61)
    pub fn cerrar_reclamos_muertos(
        &self,
        muertos: &[(i64, i64)],
        hasta: DateTime<Utc>,
    ) -> Result<usize> {
        self.en_transaccion_inmediata(|tx| {
            let hasta = time::formatear(hasta);
            let mut stmt = tx.prepare(
                "UPDATE trabajar_anclas SET hasta = ?3, motivo_salida = 'shell_cerrada'
                 WHERE pid = ?1 AND starttime = ?2 AND hasta IS NULL",
            )?;
            let mut cerrados = 0;
            for (pid, starttime) in muertos {
                cerrados += stmt.execute(params![pid, starttime, hasta])?;
            }
            Ok(cerrados)
        })
    }

    /// Suma un minuto de evidencia del token (`trabajar_evidencia`).
    #[allow(dead_code)] // Sin llamadores hasta el escritor del demonio (commit 44)
    pub fn registrar_evidencia(&self, token: &str, evidencia: &Evidencia) -> Result<()> {
        self.en_transaccion_inmediata(|tx| {
            registrar_evidencia_en(tx, "trabajar_evidencia", "token", &token, evidencia)
        })
    }

    /// Suma un minuto de evidencia de archivos del proyecto (`archivos_evidencia`,
    /// U1): solo los bits 8 (sin dato de presencia) y 32 (sin presencia).
    #[allow(dead_code)] // Sin llamadores hasta el bucle del demonio (commit 48)
    pub fn registrar_evidencia_archivos(
        &self,
        project_id: i64,
        evidencia: &Evidencia,
    ) -> Result<()> {
        self.en_transaccion_inmediata(|tx| {
            registrar_evidencia_en(
                tx,
                "archivos_evidencia",
                "project_id",
                &project_id,
                evidencia,
            )
        })
    }
}

/// ¿Está vivo el demonio? La **única** definición (3.10.7). Verdadero si:
///
/// - `tracker_status` tiene fila, con `estado = 'activo'` y `pid` no `NULL`;
/// - `ahora − last_heartbeat < 45 s`;
/// - en Linux, `/proc/<pid>/comm` existe y es `gestor-trackerd`. Sin el primero,
///   después de un `kill -9` el demonio parecería vivo 45 s; sin el nombre, un pid
///   reutilizado en esos 45 s también.
///
/// No lee la config: con el tracker apagado, el demonio deja `pid = NULL` al salir.
#[allow(dead_code)] // Sin llamadores hasta Trabajar (commit 17)
pub(crate) fn demonio_vivo(conn: &Connection, ahora: DateTime<Utc>) -> Result<bool> {
    let fila: Option<(String, Option<i64>, Option<String>)> = conn
        .query_row(
            "SELECT estado, pid, last_heartbeat FROM tracker_status WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let Some((estado, Some(pid), Some(latido))) = fila else {
        return Ok(false);
    };
    if estado != "activo" {
        return Ok(false);
    }
    if ahora - leer_instante(&latido)? >= Duration::seconds(LATIDO_VIGENTE_S) {
        return Ok(false);
    }
    Ok(es_el_demonio(pid))
}

#[cfg(target_os = "linux")]
fn es_el_demonio(pid: i64) -> bool {
    /// `comm` del demonio: 15 caracteres, justo el límite del kernel.
    const COMM_DEL_DEMONIO: &str = "gestor-trackerd";

    pid > 0
        && std::fs::read_to_string(format!("/proc/{pid}/comm"))
            .map(|comm| comm.trim_end_matches('\n') == COMM_DEL_DEMONIO)
            .unwrap_or(false)
}

/// Fuera de Linux no hay `/proc` ni demonio (3.10.13): alcanzan la fila y el latido.
#[cfg(not(target_os = "linux"))]
fn es_el_demonio(_pid: i64) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Duration, Utc};
    use rusqlite::{params, OptionalExtension};
    use std::sync::{Arc, Barrier};
    use tempfile::TempDir;

    use super::super::test_support::*;
    use super::*;
    use crate::db::Database;

    fn instante(texto: &str) -> DateTime<Utc> {
        time::parsear(texto).unwrap()
    }

    /// Un token válido (32 hex) distinto por número.
    fn token(n: u32) -> String {
        format!("{n:032x}")
    }

    /// Siembra una sesión con `ultimo_clic_at = creada_at`, como la deja Trabajar.
    fn sembrar_sesion(db: &Database, project_id: i64, token: &str, dueno: &str, estado: &str) {
        db.conn()
            .execute(
                "INSERT INTO trabajar_sesiones
                     (token, project_id, dueno, origen, estado, creada_at, ultimo_clic_at)
                 VALUES (?1, ?2, ?3, 'boton', ?4, '2026-10-20 09:00:00', '2026-10-20 09:00:00')",
                params![token, project_id, dueno, estado],
            )
            .unwrap();
    }

    /// Un tramo `terminal` de 09:00 hasta `ultima_actividad`, abierto.
    fn tramo_terminal(db: &Database, token: &str, ultima_actividad: &str) -> i64 {
        let id = db
            .abrir_tramo(token, FUENTE_TERMINAL, instante("2026-10-20 09:00:00"))
            .unwrap()
            .expect("el token está vivo");
        let duracion = (instante(ultima_actividad) - instante("2026-10-20 09:00:00")).num_seconds();
        assert!(db
            .latir(id, instante(ultima_actividad), Some(duracion))
            .unwrap());
        id
    }

    /// (estado, terminada_at, motivo_cierre) de una sesión.
    fn sesion(db: &Database, token: &str) -> (String, Option<String>, Option<String>) {
        db.conn()
            .query_row(
                "SELECT estado, terminada_at, motivo_cierre FROM trabajar_sesiones WHERE token = ?1",
                params![token],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap()
    }

    /// (ended_at, duration_seconds, motivo_cierre) de un tramo, o `None` si se borró.
    fn tramo(db: &Database, id: i64) -> Option<(Option<String>, Option<i64>, Option<String>)> {
        db.conn()
            .query_row(
                "SELECT ended_at, duration_seconds, motivo_cierre
                 FROM time_tracking_sessions WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .unwrap()
    }

    fn base_en_archivo(dir: &TempDir) -> (Database, Database) {
        let ruta = dir.path().join("projects.db");
        let una = Database::new(ruta.clone()).unwrap();
        let otra = Database::new(ruta).unwrap();
        (una, otra)
    }

    // --- Restricciones del esquema (paso 4) ---

    #[test]
    fn dos_sesiones_vivas_en_un_proyecto_son_un_error() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        sembrar_sesion(&db, p, &token(1), "demonio", "viva_activa");

        let conn = db.conn();
        let err = conn
            .execute(
                "INSERT INTO trabajar_sesiones
                     (token, project_id, dueno, origen, estado, creada_at, ultimo_clic_at)
                 VALUES (?1, ?2, 'app', 'boton', 'esperando_demonio', '2026-10-20 10:00:00', '2026-10-20 10:00:00')",
                params![token(2), p],
            )
            .expect_err("a lo sumo una sesión viva por proyecto");
        assert!(err.to_string().contains("UNIQUE"), "{err}");
        drop(conn);

        // Con la primera cerrada, la nueva entra.
        assert!(db
            .terminar_sesiones_de(
                &[p],
                instante("2026-10-20 09:30:00"),
                MotivoSesion::PararApp
            )
            .unwrap());
        sembrar_sesion(&db, p, &token(2), "app", "viva_activa");
    }

    #[test]
    fn el_token_tiene_que_ser_de_32_hex_en_minuscula() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        let conn = db.conn();
        for malo in ["abc", &"A".repeat(32), &format!("{}g", "0".repeat(31))] {
            assert!(
                conn.execute(
                    "INSERT INTO trabajar_sesiones
                         (token, project_id, dueno, origen, estado, creada_at, ultimo_clic_at)
                     VALUES (?1, ?2, 'app', 'boton', 'viva_activa', '2026-10-20 09:00:00', '2026-10-20 09:00:00')",
                    params![malo, p],
                )
                .is_err(),
                "el token {malo:?} no es válido"
            );
        }
    }

    #[test]
    fn una_sesion_cerrada_sin_motivo_o_viva_con_motivo_es_un_error_de_check() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        sembrar_sesion(&db, p, &token(1), "demonio", "viva_pausada");
        let conn = db.conn();

        let err = conn
            .execute(
                "UPDATE trabajar_sesiones SET estado = 'cerrada', terminada_at = '2026-10-20 10:00:00'",
                [],
            )
            .expect_err("cerrada sin motivo");
        assert!(err.to_string().contains("CHECK"), "{err}");
        assert!(conn
            .execute(
                "UPDATE trabajar_sesiones SET estado = 'cerrada', motivo_cierre = 'parar_app'",
                [],
            )
            .is_err());
        assert!(conn
            .execute(
                "UPDATE trabajar_sesiones SET motivo_cierre = 'parar_app'",
                []
            )
            .is_err());
        assert!(conn
            .execute("UPDATE trabajar_sesiones SET estado = 'pausada'", [])
            .is_err());
    }

    /// El `CHECK` de tramo de sesión con motivo cubre el `UPDATE` y el `INSERT`, y
    /// acepta las filas viejas: sin token, cerradas sin motivo.
    #[test]
    fn un_tramo_de_sesion_cerrado_sin_motivo_es_un_error_de_check() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        sembrar_sesion(&db, p, &token(1), "app", "viva_activa");
        let id = db
            .abrir_tramo(&token(1), "work_button", instante("2026-10-20 09:00:00"))
            .unwrap()
            .unwrap();
        let conn = db.conn();

        let err = conn
            .execute(
                "UPDATE time_tracking_sessions
                 SET ended_at = '2026-10-20 10:00:00', duration_seconds = 3600 WHERE id = ?1",
                params![id],
            )
            .expect_err("UPDATE sin motivo");
        assert!(err.to_string().contains("CHECK"), "{err}");

        assert!(conn
            .execute(
                "INSERT INTO time_tracking_sessions
                     (project_id, started_at, ended_at, duration_seconds, source, terminal_token)
                 VALUES (?1, '2026-10-20 07:00:00', '2026-10-20 08:00:00', 3600, 'work_button', ?2)",
                params![p, token(1)],
            )
            .is_err());
        assert!(conn
            .execute(
                "UPDATE time_tracking_sessions SET motivo_cierre = 'otro' WHERE id = ?1",
                params![id],
            )
            .is_err());

        // Las filas viejas, sin token, se aceptan cerradas sin motivo.
        conn.execute(
            "INSERT INTO time_tracking_sessions (project_id, started_at, ended_at, duration_seconds, source)
             VALUES (?1, '2026-10-19 07:00:00', '2026-10-19 08:00:00', 3600, 'work_button')",
            params![p],
        )
        .unwrap();
    }

    #[test]
    fn dos_tramos_abiertos_de_un_token_son_un_error_de_indice() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        sembrar_sesion(&db, p, &token(1), "demonio", "viva_activa");
        db.abrir_tramo(&token(1), FUENTE_TERMINAL, instante("2026-10-20 09:00:00"))
            .unwrap()
            .unwrap();

        let err = db
            .abrir_tramo(&token(1), FUENTE_TERMINAL, instante("2026-10-20 09:05:00"))
            .expect_err("serían dos escritores");
        assert!(err.to_string().contains("UNIQUE"), "{err}");

        // Varias filas abiertas SIN token (file_watch, filas viejas) siguen valiendo.
        db.abrir_sesion(p, "file_watch", instante("2026-10-20 09:00:00"))
            .unwrap();
        db.abrir_sesion(p, "file_watch", instante("2026-10-20 09:01:00"))
            .unwrap();
    }

    #[test]
    fn un_reclamo_vigente_por_shell_y_vetos_sin_token() {
        let db = test_db();
        let conn = db.conn();
        let insertar = |token: Option<&str>| {
            conn.execute(
                "INSERT INTO trabajar_anclas (token, pid, starttime, tty_nr, desde)
                 VALUES (?1, 4242, 99, 34817, '2026-10-20 09:00:00')",
                params![token],
            )
        };
        insertar(None).expect("un veto no tiene token");
        assert!(
            insertar(None).is_err(),
            "la misma shell con dos reclamos vigentes"
        );

        conn.execute(
            "UPDATE trabajar_anclas SET hasta = '2026-10-20 09:10:00', motivo_salida = 'sin_medir'",
            [],
        )
        .unwrap();
        insertar(None).expect("cerrado el anterior, entra otro");

        assert!(
            conn.execute(
                "UPDATE trabajar_anclas SET hasta = '2026-10-20 09:20:00' WHERE hasta IS NULL",
                []
            )
            .is_err(),
            "hasta sin motivo_salida"
        );
        assert!(conn
            .execute(
                "INSERT INTO trabajar_anclas (pid, starttime, tty_nr, desde) VALUES (1, 1, 0, '2026-10-20 09:00:00')",
                [],
            )
            .is_err(), "tty_nr = 0 no es un pty");
    }

    #[test]
    fn borrar_el_proyecto_borra_en_cascada_sesiones_tramos_anclas_y_evidencia() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        let otro = seed_project_at(&db, "Otro", "/tmp/otro");
        sembrar_sesion(&db, p, &token(1), "demonio", "viva_activa");
        sembrar_sesion(&db, otro, &token(2), "demonio", "viva_activa");
        tramo_terminal(&db, &token(1), "2026-10-20 09:10:00");
        db.execute_batch_para_tests(&format!(
            "INSERT INTO trabajar_anclas (token, pid, starttime, tty_nr, desde)
             VALUES ('{}', 10, 20, 34817, '2026-10-20 09:00:00');",
            token(1)
        ));
        let evidencia = Evidencia {
            minuto: instante("2026-10-20 09:05:00"),
            fuentes: Evidencia::TIPEO,
            archivo_rel: None,
            ausente: None,
        };
        db.registrar_evidencia(&token(1), &evidencia).unwrap();
        db.registrar_evidencia(&token(2), &evidencia).unwrap();
        db.registrar_evidencia_archivos(
            p,
            &Evidencia {
                fuentes: Evidencia::ARCHIVO_SIN_DATO_DE_PRESENCIA,
                ..evidencia
            },
        )
        .unwrap();

        db.delete_project(p).unwrap();
        db.purge_project(p).unwrap();

        let conn = db.conn();
        let cuantas = |sql: &str| -> i64 { conn.query_row(sql, [], |row| row.get(0)).unwrap() };
        assert_eq!(cuantas("SELECT COUNT(*) FROM trabajar_sesiones"), 1);
        assert_eq!(cuantas("SELECT COUNT(*) FROM trabajar_anclas"), 0);
        assert_eq!(cuantas("SELECT COUNT(*) FROM trabajar_evidencia"), 1);
        assert_eq!(cuantas("SELECT COUNT(*) FROM archivos_evidencia"), 0);
        assert_eq!(
            cuantas("SELECT COUNT(*) FROM time_tracking_sessions WHERE terminal_token IS NOT NULL"),
            0
        );
        drop(conn);

        // También el DELETE directo del proyecto, sin el borrado manual de los tramos.
        db.execute_batch_para_tests(&format!("DELETE FROM projects WHERE id = {otro};"));
        assert_eq!(
            db.conn()
                .query_row("SELECT COUNT(*) FROM trabajar_sesiones", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    // --- Métodos condicionales ---

    #[test]
    fn abrir_tramo_sobre_un_token_cerrado_o_inexistente_devuelve_none() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        sembrar_sesion(&db, p, &token(1), "demonio", "viva_pausada");
        db.terminar_sesiones_de(
            &[p],
            instante("2026-10-20 09:30:00"),
            MotivoSesion::PararApp,
        )
        .unwrap();

        assert_eq!(
            db.abrir_tramo(&token(1), FUENTE_TERMINAL, instante("2026-10-20 09:40:00"))
                .unwrap(),
            None
        );
        assert_eq!(
            db.abrir_tramo(&token(9), FUENTE_TERMINAL, instante("2026-10-20 09:40:00"))
                .unwrap(),
            None
        );
        let tramos: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM time_tracking_sessions", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(tramos, 0, "no deja un tramo huérfano");
    }

    /// Un tramo `terminal` nace con `duration = 0`, `last_activity_at = started_at`, el
    /// proyecto del token y el token.
    #[test]
    fn abrir_tramo_terminal_nace_con_la_invariante_de_dt1() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        sembrar_sesion(&db, p, &token(1), "demonio", "esperando_terminal");

        let id = db
            .abrir_tramo(&token(1), FUENTE_TERMINAL, instante("2026-10-20 09:00:07"))
            .unwrap()
            .unwrap();

        let fila: (i64, String, String, i64, String, String) = db
            .conn()
            .query_row(
                "SELECT project_id, started_at, last_activity_at, duration_seconds, source, terminal_token
                 FROM time_tracking_sessions WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            )
            .unwrap();
        assert_eq!(
            fila,
            (
                p,
                "2026-10-20 09:00:07".to_string(),
                "2026-10-20 09:00:07".to_string(),
                0,
                "terminal".to_string(),
                token(1)
            )
        );
    }

    /// Terminar cierra el tramo en la última actividad (2a) con el motivo de la
    /// sesión, y las anclas abiertas con `parar`. Terminar otra vez: `false`, nada
    /// cambia.
    #[test]
    fn terminar_dos_veces_devuelve_false_y_no_toca_nada() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        sembrar_sesion(&db, p, &token(1), "demonio", "viva_activa");
        let id = tramo_terminal(&db, &token(1), "2026-10-20 09:20:00");
        db.execute_batch_para_tests(&format!(
            "INSERT INTO trabajar_anclas (token, pid, starttime, tty_nr, desde)
             VALUES ('{}', 10, 20, 34817, '2026-10-20 09:00:00');",
            token(1)
        ));

        assert!(db
            .terminar_sesiones_de(
                &[p],
                instante("2026-10-20 09:45:00"),
                MotivoSesion::PararApp
            )
            .unwrap());

        assert_eq!(
            tramo(&db, id),
            Some((
                Some("2026-10-20 09:20:00".to_string()),
                Some(1200),
                Some("parar_app".to_string())
            )),
            "cierra en la última actividad: la cola no se cobra"
        );
        let cerrada = sesion(&db, &token(1));
        assert_eq!(
            cerrada,
            (
                "cerrada".to_string(),
                Some("2026-10-20 09:45:00".to_string()),
                Some("parar_app".to_string())
            )
        );
        let ancla: (Option<String>, Option<String>) = db
            .conn()
            .query_row(
                "SELECT hasta, motivo_salida FROM trabajar_anclas",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            ancla,
            (
                Some("2026-10-20 09:45:00".to_string()),
                Some("parar".to_string())
            )
        );

        assert!(!db
            .terminar_sesiones_de(
                &[p],
                instante("2026-10-20 10:00:00"),
                MotivoSesion::TrackerApagado
            )
            .unwrap());
        assert_eq!(sesion(&db, &token(1)), cerrada);
        assert!(!db
            .terminar_sesiones_de(&[], instante("2026-10-20 10:00:00"), MotivoSesion::PararApp)
            .unwrap());
    }

    /// La papelera de un padre termina las sesiones del padre y de los hijos que le
    /// pasan, y no las de otros proyectos.
    #[test]
    fn terminar_sesiones_de_varios_proyectos_deja_las_de_los_demas() {
        let db = test_db();
        let padre = seed_project_at(&db, "Padre", "/tmp/padre");
        let hijo = seed_project_at(&db, "Hijo", "/tmp/hijo");
        let otro = seed_project_at(&db, "Otro", "/tmp/otro");
        sembrar_sesion(&db, padre, &token(1), "demonio", "viva_pausada");
        sembrar_sesion(&db, hijo, &token(2), "demonio", "esperando_terminal");
        sembrar_sesion(&db, otro, &token(3), "demonio", "viva_pausada");

        assert!(db
            .terminar_sesiones_de(
                &[padre, hijo],
                instante("2026-10-20 10:00:00"),
                MotivoSesion::ProyectoAPapelera
            )
            .unwrap());

        assert_eq!(
            sesion(&db, &token(1)).2.as_deref(),
            Some("proyecto_a_papelera")
        );
        assert_eq!(
            sesion(&db, &token(2)).2.as_deref(),
            Some("proyecto_a_papelera")
        );
        assert_eq!(sesion(&db, &token(3)).0, "viva_pausada");
    }

    /// Regla única de 3.10.15: una sesión que nunca se vio (`last_seen_at` nulo,
    /// midió solo por archivos) termina con `tracker_apagado` en
    /// `COALESCE(last_seen_at, creada_at)`, que queda ANTES de sus tramos: gana el
    /// `MAX(ended_at)`.
    #[test]
    fn terminada_at_nunca_es_anterior_al_fin_de_un_tramo() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        sembrar_sesion(&db, p, &token(1), "demonio", "terminal_no_detectada");
        let primero = tramo_terminal(&db, &token(1), "2026-10-20 09:15:00");
        db.cerrar_sesion(
            primero,
            instante("2026-10-20 09:15:00"),
            900,
            MotivoTramo::Inactividad,
        )
        .unwrap();
        db.conn()
            .execute(
                "INSERT INTO time_tracking_sessions
                     (project_id, started_at, last_activity_at, duration_seconds, source, terminal_token)
                 VALUES (?1, '2026-10-20 11:00:00', '2026-10-20 11:30:00', 1800, 'terminal', ?2)",
                params![p, token(1)],
            )
            .unwrap();

        assert!(db
            .terminar_sesiones_de(
                &[p],
                instante("2026-10-20 12:00:00"),
                MotivoSesion::TrackerApagado
            )
            .unwrap());

        let (_, terminada_at, motivo) = sesion(&db, &token(1));
        assert_eq!(terminada_at.as_deref(), Some("2026-10-20 11:30:00"));
        assert_eq!(motivo.as_deref(), Some("tracker_apagado"));
        let posteriores: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM trabajar_sesiones s JOIN time_tracking_sessions t
                     ON t.terminal_token = s.token
                 WHERE s.estado = 'cerrada' AND t.ended_at > s.terminada_at",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(posteriores, 0, "invariante 9 de §6.2");
    }

    /// DT5: un tramo `terminal` que cierra en su inicio (0 s) se borra.
    #[test]
    fn un_tramo_terminal_de_0_s_se_borra() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        sembrar_sesion(&db, p, &token(1), "demonio", "esperando_terminal");
        let id = db
            .abrir_tramo(&token(1), FUENTE_TERMINAL, instante("2026-10-20 09:00:00"))
            .unwrap()
            .unwrap();

        assert!(db
            .terminar_sesiones_de(
                &[p],
                instante("2026-10-20 09:00:30"),
                MotivoSesion::PararApp
            )
            .unwrap());
        assert_eq!(tramo(&db, id), None, "el tramo de 0 s se borra");
        assert_eq!(sesion(&db, &token(1)).0, "cerrada");

        // Con `cerrar_sesion` directo, igual; un `work_button` de 0 s se conserva.
        let otro = db
            .abrir_sesion(p, "terminal", instante("2026-10-20 10:00:00"))
            .unwrap();
        assert!(db
            .cerrar_sesion(
                otro,
                instante("2026-10-20 10:00:00"),
                0,
                MotivoTramo::Inactividad
            )
            .unwrap());
        assert_eq!(tramo(&db, otro), None);
        let boton = db
            .abrir_sesion(p, "work_button", instante("2026-10-20 10:00:00"))
            .unwrap();
        assert!(db
            .cerrar_sesion(
                boton,
                instante("2026-10-20 10:00:05"),
                0,
                MotivoTramo::Inactividad
            )
            .unwrap());
        assert!(tramo(&db, boton).is_some());
    }

    /// Un `work_button` de la app que quedó abierto se cierra con la regla de huérfanas:
    /// en su último latido, con lo medido, y con los 10 s.
    #[test]
    fn terminar_cierra_un_work_button_abierto_con_la_regla_de_huerfanas() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        sembrar_sesion(&db, p, &token(1), "app", "viva_activa");
        let id = db
            .abrir_tramo(&token(1), "work_button", instante("2026-10-20 09:00:00"))
            .unwrap()
            .unwrap();
        // Latido a las 10:00 con 50 min medidos (hubo una suspensión).
        db.latir(id, instante("2026-10-20 10:00:00"), Some(3000))
            .unwrap();

        db.terminar_sesiones_de(
            &[p],
            instante("2026-10-20 10:30:00"),
            MotivoSesion::Restauracion,
        )
        .unwrap();
        assert_eq!(
            tramo(&db, id),
            Some((
                Some("2026-10-20 10:00:00".to_string()),
                Some(3000),
                Some("restauracion".to_string())
            ))
        );
    }

    #[test]
    fn pasar_a_esperando_demonio_solo_sin_tramo_abierto_y_viva() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        sembrar_sesion(&db, p, &token(1), "app", "viva_activa");
        let id = db
            .abrir_tramo(&token(1), "work_button", instante("2026-10-20 09:00:00"))
            .unwrap()
            .unwrap();

        assert!(
            !db.pasar_a_esperando_demonio(&token(1)).unwrap(),
            "dueno no cambia con un tramo abierto (DT14)"
        );
        assert_eq!(sesion(&db, &token(1)).0, "viva_activa");

        db.cerrar_sesion(
            id,
            instante("2026-10-20 10:00:00"),
            3600,
            MotivoTramo::Sesion(MotivoSesion::CierreApp),
        )
        .unwrap();
        assert!(db.pasar_a_esperando_demonio(&token(1)).unwrap());
        let (dueno, estado, adoptada_at): (String, String, Option<String>) = db
            .conn()
            .query_row(
                "SELECT dueno, estado, adoptada_at FROM trabajar_sesiones",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            (dueno.as_str(), estado.as_str(), adoptada_at),
            ("demonio", "esperando_demonio", None)
        );

        db.terminar_sesiones_de(
            &[p],
            instante("2026-10-20 11:00:00"),
            MotivoSesion::PararApp,
        )
        .unwrap();
        assert!(!db.pasar_a_esperando_demonio(&token(1)).unwrap());
        assert!(!db.pasar_a_esperando_demonio(&token(9)).unwrap());
    }

    /// Sin carrera (3.10.2): un clic o un ancla posteriores al escaneo que no vio
    /// nada impiden el cierre; sin ellos, cierra con `terminal_cerrada` en
    /// `last_seen_at`.
    #[test]
    fn cerrar_por_terminal_no_cierra_con_un_clic_o_un_ancla_posteriores_al_escaneo() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        sembrar_sesion(&db, p, &token(1), "demonio", "viva_activa");
        let id = tramo_terminal(&db, &token(1), "2026-10-20 09:20:00");
        db.execute_batch_para_tests(
            "UPDATE trabajar_sesiones SET last_seen_at = '2026-10-20 09:24:55',
                                          ultimo_clic_at = '2026-10-20 09:25:01';",
        );
        let escaneo = instante("2026-10-20 09:25:00");

        assert!(
            !db.cerrar_por_terminal(&token(1), escaneo).unwrap(),
            "clic posterior"
        );

        db.execute_batch_para_tests(&format!(
            "UPDATE trabajar_sesiones SET ultimo_clic_at = '2026-10-20 09:00:00';
             INSERT INTO trabajar_anclas (token, pid, starttime, tty_nr, desde)
             VALUES ('{}', 10, 20, 34817, '2026-10-20 09:25:02');",
            token(1)
        ));
        assert!(
            !db.cerrar_por_terminal(&token(1), escaneo).unwrap(),
            "ancla posterior"
        );
        assert_eq!(sesion(&db, &token(1)).0, "viva_activa");
        assert_eq!(tramo(&db, id).unwrap().0, None, "el tramo sigue abierto");

        // Un escaneo posterior al ancla (que la vio muerta) sí cierra.
        db.cerrar_reclamos_muertos(&[(10, 20)], instante("2026-10-20 09:25:10"))
            .unwrap();
        assert!(db
            .cerrar_por_terminal(&token(1), instante("2026-10-20 09:25:10"))
            .unwrap());
        assert_eq!(
            sesion(&db, &token(1)),
            (
                "cerrada".to_string(),
                Some("2026-10-20 09:24:55".to_string()),
                Some("terminal_cerrada".to_string())
            )
        );
        assert_eq!(
            tramo(&db, id).unwrap().2.as_deref(),
            Some("terminal_cerrada")
        );
        assert!(!db
            .cerrar_por_terminal(&token(1), instante("2026-10-20 09:30:00"))
            .unwrap());
    }

    /// El demonio no cierra por terminal una sesión que mide la app.
    #[test]
    fn cerrar_por_terminal_no_toca_una_sesion_de_la_app() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        sembrar_sesion(&db, p, &token(1), "app", "esperando_demonio");
        assert!(!db
            .cerrar_por_terminal(&token(1), instante("2026-10-20 10:00:00"))
            .unwrap());
        assert_eq!(sesion(&db, &token(1)).0, "esperando_demonio");
    }

    /// Parar desde la app y "terminal cerrada" desde el demonio, a la vez y con dos
    /// conexiones: gana uno solo, y el otro recibe `false` sin error.
    #[test]
    fn parar_y_terminal_cerrada_a_la_vez_desde_dos_conexiones_dan_un_solo_true() {
        let dir = TempDir::new().unwrap();
        let (app, demonio) = base_en_archivo(&dir);
        let p = seed_project_at(&app, "P", "/tmp/p");
        let (app, demonio) = (Arc::new(app), Arc::new(demonio));

        for vuelta in 0..20 {
            let t = token(vuelta + 1);
            sembrar_sesion(&app, p, &t, "demonio", "viva_activa");
            tramo_terminal(&app, &t, "2026-10-20 09:20:00");

            let largada = Arc::new(Barrier::new(2));
            let parar = {
                let (app, largada) = (Arc::clone(&app), Arc::clone(&largada));
                std::thread::spawn(move || {
                    largada.wait();
                    app.terminar_sesiones_de(
                        &[p],
                        instante("2026-10-20 09:30:00"),
                        MotivoSesion::PararApp,
                    )
                })
            };
            let cerrar = {
                let (demonio, largada, t) = (Arc::clone(&demonio), Arc::clone(&largada), t.clone());
                std::thread::spawn(move || {
                    largada.wait();
                    demonio.cerrar_por_terminal(&t, instante("2026-10-20 09:30:00"))
                })
            };
            let resultados = [
                parar
                    .join()
                    .unwrap()
                    .expect("Parar no puede fallar por la carrera"),
                cerrar
                    .join()
                    .unwrap()
                    .expect("el cierre no puede fallar por la carrera"),
            ];

            assert_eq!(
                resultados.iter().filter(|cerro| **cerro).count(),
                1,
                "vuelta {vuelta}: {resultados:?}"
            );
            assert_eq!(sesion(&app, &t).0, "cerrada");
        }
    }

    #[test]
    fn cerrar_reclamos_muertos_cierra_solo_los_vigentes_de_esas_shells() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        sembrar_sesion(&db, p, &token(1), "demonio", "viva_activa");
        db.execute_batch_para_tests(&format!(
            "INSERT INTO trabajar_anclas (token, pid, starttime, tty_nr, desde)
             VALUES ('{t}', 10, 20, 34817, '2026-10-20 09:00:00'),
                    (NULL, 11, 21, 34818, '2026-10-20 09:00:00'),
                    ('{t}', 12, 22, 34819, '2026-10-20 09:00:00'),
                    ('{t}', 10, 19, 34817, '2026-10-20 08:00:00');
             UPDATE trabajar_anclas SET hasta = '2026-10-20 08:30:00', motivo_salida = 'sin_medir'
             WHERE starttime = 19;",
            t = token(1)
        ));

        // La shell 10 con otro starttime es otra shell (pid reutilizado): no se toca.
        let cerrados = db
            .cerrar_reclamos_muertos(
                &[(10, 20), (11, 21), (99, 99)],
                instante("2026-10-20 09:30:00"),
            )
            .unwrap();
        assert_eq!(cerrados, 2, "el ancla y el veto");

        let filas: Vec<(i64, Option<String>, Option<String>)> = db
            .conn()
            .prepare("SELECT pid, hasta, motivo_salida FROM trabajar_anclas ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        let cerrada = |m: &str| (Some("2026-10-20 09:30:00".to_string()), Some(m.to_string()));
        assert_eq!(
            (filas[0].1.clone(), filas[0].2.clone()),
            cerrada("shell_cerrada")
        );
        assert_eq!(
            (filas[1].1.clone(), filas[1].2.clone()),
            cerrada("shell_cerrada")
        );
        assert_eq!((filas[2].1.clone(), filas[2].2.clone()), (None, None));
        assert_eq!(filas[3].2.as_deref(), Some("sin_medir"));
    }

    // --- Evidencia (DT12, U1) ---

    /// (fuentes, archivo_rel, ausente_desde_s, ausente_hasta_s) del único minuto.
    fn minuto_de(db: &Database, tabla: &str) -> (i64, Option<String>, Option<i64>, Option<i64>) {
        db.conn()
            .query_row(
                &format!(
                    "SELECT fuentes, archivo_rel, ausente_desde_s, ausente_hasta_s FROM {tabla}"
                ),
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap()
    }

    /// El UPSERT suma bits con OR, conserva el primer archivo y lleva el mínimo y el
    /// máximo de los segundos sin presencia, en las cuatro combinaciones de nulos.
    #[test]
    fn el_upsert_de_evidencia_hace_or_de_bits_y_min_max_de_los_segundos() {
        type Segundos = Option<(i64, i64)>;
        let casos: [(Segundos, Segundos, Segundos); 4] = [
            (None, None, None),
            (None, Some((30, 40)), Some((30, 40))),
            (Some((10, 20)), None, Some((10, 20))),
            (Some((10, 50)), Some((5, 30)), Some((5, 50))),
        ];
        for (antes, despues, esperado) in casos {
            let db = test_db();
            let p = seed_project_at(&db, "P", "/tmp/p");
            sembrar_sesion(&db, p, &token(1), "demonio", "viva_activa");
            let fuentes = |ausente: Option<(i64, i64)>, base: i64| {
                base | ausente.map_or(0, |_| Evidencia::ARCHIVO_SIN_PRESENCIA)
            };
            let minuto = instante("2026-10-20 14:37:12");

            db.registrar_evidencia(
                &token(1),
                &Evidencia {
                    minuto,
                    fuentes: fuentes(antes, Evidencia::TIPEO),
                    archivo_rel: Some("src/main.rs"),
                    ausente: antes,
                },
            )
            .unwrap();
            db.registrar_evidencia(
                &token(1),
                &Evidencia {
                    minuto: minuto + Duration::seconds(30),
                    fuentes: fuentes(despues, Evidencia::REMOTO),
                    archivo_rel: Some("otro.rs"),
                    ausente: despues,
                },
            )
            .unwrap();

            let (bits, archivo, desde, hasta) = minuto_de(&db, "trabajar_evidencia");
            assert_eq!(
                bits,
                Evidencia::TIPEO | Evidencia::REMOTO | fuentes(esperado, 0),
                "{antes:?} + {despues:?}"
            );
            assert_eq!(archivo.as_deref(), Some("src/main.rs"), "el primer archivo");
            assert_eq!(desde.zip(hasta), esperado, "{antes:?} + {despues:?}");
            let minuto: String = db
                .conn()
                .query_row("SELECT minuto FROM trabajar_evidencia", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(minuto, "2026-10-20 14:37");
        }
    }

    #[test]
    fn el_bit_32_sin_segundos_o_los_segundos_sin_el_bit_son_un_error_de_check() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        sembrar_sesion(&db, p, &token(1), "demonio", "viva_activa");
        let base = Evidencia {
            minuto: instante("2026-10-20 14:37:00"),
            fuentes: Evidencia::ARCHIVO_SIN_PRESENCIA,
            archivo_rel: None,
            ausente: None,
        };

        let err = db.registrar_evidencia(&token(1), &base).unwrap_err();
        assert!(err.to_string().contains("CHECK"), "{err}");
        assert!(db
            .registrar_evidencia(
                &token(1),
                &Evidencia {
                    fuentes: Evidencia::TIPEO,
                    ausente: Some((1, 2)),
                    ..base
                }
            )
            .is_err());
        assert!(
            db.registrar_evidencia(
                &token(1),
                &Evidencia {
                    ausente: Some((50, 60)),
                    ..base
                }
            )
            .is_err(),
            "segundos fuera de 0–59"
        );
        assert!(
            db.registrar_evidencia(
                &token(1),
                &Evidencia {
                    ausente: Some((40, 10)),
                    ..base
                }
            )
            .is_err(),
            "el primero después del último"
        );
        db.registrar_evidencia(
            &token(1),
            &Evidencia {
                ausente: Some((10, 40)),
                ..base
            },
        )
        .unwrap();
    }

    #[test]
    fn archivos_evidencia_solo_acepta_los_bits_8_y_32() {
        let db = test_db();
        let p = seed_project_at(&db, "P", "/tmp/p");
        let base = Evidencia {
            minuto: instante("2026-10-20 14:37:00"),
            fuentes: Evidencia::ARCHIVO_SIN_DATO_DE_PRESENCIA,
            archivo_rel: Some("a.txt"),
            ausente: None,
        };
        for bits in [
            0,
            Evidencia::TIPEO,
            Evidencia::ARCHIVO_CON_PRESENCIA,
            8 | 16,
            64,
        ] {
            let err = db
                .registrar_evidencia_archivos(
                    p,
                    &Evidencia {
                        fuentes: bits,
                        ..base
                    },
                )
                .expect_err("solo 8 y 32");
            assert!(err.to_string().contains("CHECK"), "bits {bits}: {err}");
        }

        db.registrar_evidencia_archivos(p, &base).unwrap();
        db.registrar_evidencia_archivos(
            p,
            &Evidencia {
                fuentes: Evidencia::ARCHIVO_SIN_PRESENCIA,
                archivo_rel: Some("b.txt"),
                ausente: Some((12, 12)),
                ..base
            },
        )
        .unwrap();
        assert_eq!(
            minuto_de(&db, "archivos_evidencia"),
            (40, Some("a.txt".to_string()), Some(12), Some(12))
        );
    }

    // --- demonio_vivo (3.10.7) ---

    fn sembrar_tracker_status(db: &Database, estado: &str, pid: Option<i64>, latido: &str) {
        db.conn()
            .execute(
                "INSERT OR REPLACE INTO tracker_status (id, estado, pid, last_heartbeat)
                 VALUES (1, ?1, ?2, ?3)",
                params![estado, pid, latido],
            )
            .unwrap();
    }

    fn vivo(db: &Database, ahora: &str) -> bool {
        demonio_vivo(&db.conn(), instante(ahora)).unwrap()
    }

    /// Un proceso hijo cuyo `comm` es `gestor-trackerd`: un `sh` que se renombra
    /// escribiendo su propio `/proc/<pid>/comm` y espera.
    #[cfg(target_os = "linux")]
    struct DemonioFalso(std::process::Child);

    #[cfg(target_os = "linux")]
    impl DemonioFalso {
        fn lanzar() -> Self {
            // Envuelto ya: si no se renombra, el panic lo mata y lo espera en `drop`.
            let demonio = DemonioFalso(
                std::process::Command::new("sh")
                    .args(["-c", "printf gestor-trackerd > /proc/$$/comm; sleep 30; :"])
                    .spawn()
                    .unwrap(),
            );
            let comm = format!("/proc/{}/comm", demonio.pid());
            for _ in 0..200 {
                if std::fs::read_to_string(&comm).is_ok_and(|c| c == "gestor-trackerd\n") {
                    return demonio;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            panic!("el proceso de prueba no se renombró");
        }

        fn pid(&self) -> i64 {
            i64::from(self.0.id())
        }
    }

    #[cfg(target_os = "linux")]
    impl Drop for DemonioFalso {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn demonio_vivo_exige_fila_activa_pid_latido_reciente_y_el_comm_del_demonio() {
        let db = test_db();
        let demonio = DemonioFalso::lanzar();
        let latido = "2026-10-20 09:00:00";

        assert!(!vivo(&db, "2026-10-20 09:00:10"), "sin fila: nunca corrió");

        sembrar_tracker_status(&db, "activo", Some(demonio.pid()), latido);
        assert!(vivo(&db, "2026-10-20 09:00:44"), "latido de hace 44 s");
        assert!(!vivo(&db, "2026-10-20 09:00:46"), "latido de hace 46 s");

        sembrar_tracker_status(&db, "desactivado", Some(demonio.pid()), latido);
        assert!(!vivo(&db, "2026-10-20 09:00:10"), "desactivado");

        sembrar_tracker_status(&db, "activo", None, latido);
        assert!(!vivo(&db, "2026-10-20 09:00:10"), "pid NULL: salió limpio");

        sembrar_tracker_status(&db, "activo", Some(i64::from(i32::MAX)), latido);
        assert!(
            !vivo(&db, "2026-10-20 09:00:10"),
            "pid inexistente (kill -9)"
        );

        sembrar_tracker_status(&db, "activo", Some(i64::from(std::process::id())), latido);
        assert!(!vivo(&db, "2026-10-20 09:00:10"), "pid vivo con otro comm");
    }
}
