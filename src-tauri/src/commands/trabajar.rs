//! Trabajar, Parar y lo que lee la tarjeta (commit 17, R8-3; 3.10.6, 3.10.7, 3.10.9,
//! 3.10.11 y 3.10.14 del diseño del tracker).
//!
//! La **base** es la fuente de verdad de las sesiones (DT2): `trabajar_sesiones` dice
//! qué sesiones existen y en qué estado están. `ActiveSession` guarda solo la única
//! sesión que mide la app con su propio reloj (`MedicionApp`), porque sin demonio la
//! app mide **un** proyecto a la vez.
//!
//! Orden de locks: `ActiveSession → Database`, como en el resto de la app.
//!
//! Las funciones `*_sync` reciben el reloj, el entorno y la forma de abrir la
//! terminal: así los tests fijan el tiempo (un `Instant` de 13 h), simulan Windows o
//! un `gestor-trackerd` en un `BIN_DIR` temporal, y no abren terminales.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use chrono::{DateTime, Utc};
use log::{debug, info, warn};
use tauri::State;

use crate::config::ConfigManager;
use crate::db::{
    Database, Dueno, InicioTrabajar, LatidoApp, MedicionTrabajar, MedidoPorLaApp, MotivoSesion,
    PedidoTrabajar, ResultadoParar, SesionCortada, SesionViva,
};
use crate::platform::{get_platform, MarcaSesion};
use crate::restore_lock;
use crate::tracking::config::{has_tracking_config, init_gestor_config};

/// Variable de entorno que declara que en esta instalación no hay tracker (la misma
/// de las condiciones de instalación de §3.8).
const VAR_SIN_TRACKER: &str = "GESTOR_NO_TRACKERD";

/// El binario del demonio dentro de `BIN_DIR`.
const BINARIO_DEL_TRACKER: &str = "gestor-trackerd";

/// El demonio se espera caído recién después de esta cantidad de sondeos seguidos sin
/// latido (≥ 10 s), para que la alarma no parpadee en un `restart` (3.10.14).
const SONDEOS_PARA_LA_ALARMA: u32 = 2;

/// La sesión que mide la app con su `Instant` (3.10.7): `dueno = app`, en
/// `viva_activa`, con su tramo `work_button` abierto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MedicionApp {
    pub project_id: i64,
    pub token: String,
    pub tramo_id: i64,
    pub inicio: Instant,
}

impl MedicionApp {
    /// Lo medido hasta `ahora`, para cerrar el tramo con el `Instant`.
    fn medido(&self, ahora: Instant) -> MedidoPorLaApp<'_> {
        MedidoPorLaApp {
            token: &self.token,
            tramo_id: self.tramo_id,
            segundos: segundos_desde(self.inicio, ahora),
        }
    }
}

fn segundos_desde(inicio: Instant, ahora: Instant) -> i64 {
    i64::try_from(ahora.saturating_duration_since(inicio).as_secs()).unwrap_or(i64::MAX)
}

/// Lo único que la app guarda en memoria: la sesión que mide ella, si mide alguna.
#[derive(Debug, Default)]
pub struct ActiveSession(pub Mutex<Option<MedicionApp>>);

impl ActiveSession {
    fn tomar(&self) -> Result<MutexGuard<'_, Option<MedicionApp>>, String> {
        self.0
            .lock()
            .map_err(|_| "Error locking session state".to_string())
    }
}

/// El instante de una acción: la pared que se guarda en la base y el monotónico con
/// el que mide la app.
#[derive(Debug, Clone, Copy)]
pub struct Reloj {
    pub pared: DateTime<Utc>,
    pub mono: Instant,
}

impl Reloj {
    pub fn ahora() -> Self {
        Self {
            pared: Utc::now(),
            mono: Instant::now(),
        }
    }
}

/// Dónde corre la app, para `tracker_soportado()` y `restore.lock`.
#[derive(Debug, Clone)]
pub struct Entorno {
    /// Carpeta de la base, donde vive `restore.lock`.
    pub data_dir: PathBuf,
    /// `BIN_DIR` (por defecto `~/.local/bin`): donde se instala `gestor-trackerd`.
    pub bin_dir: PathBuf,
    pub linux: bool,
    /// `GESTOR_NO_TRACKERD` definida en el entorno.
    pub sin_tracker_por_variable: bool,
}

impl Entorno {
    pub fn del_sistema() -> Result<Self, String> {
        let db_file = crate::backup::live_db_path()?;
        let data_dir = db_file
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| format!("Invalid database path: {}", db_file.display()))?;
        let bin_dir = dirs::home_dir()
            .map(|home| home.join(".local").join("bin"))
            .unwrap_or_default();
        Ok(Self {
            data_dir,
            bin_dir,
            linux: cfg!(target_os = "linux"),
            sin_tracker_por_variable: std::env::var_os(VAR_SIN_TRACKER).is_some(),
        })
    }
}

/// ¿Puede haber un tracker en esta instalación? Linux, sin `GESTOR_NO_TRACKERD` y con
/// `BIN_DIR/gestor-trackerd` presente y ejecutable (3.10.7, commit 17). En F1–F5
/// ningún script instala ese binario, así que en una instalación real da falso.
pub fn tracker_soportado(entorno: &Entorno) -> bool {
    entorno.linux
        && !entorno.sin_tracker_por_variable
        && es_ejecutable(&entorno.bin_dir.join(BINARIO_DEL_TRACKER))
}

/// Nadie va a adoptar nunca una sesión en espera (U4, 3.10.7). Crece por commits: la
/// cláusula `daemon_enabled` entra en el 35. Con adopción imposible, cerrar la app,
/// Trabajar en otro proyecto, una caída o el tope de 12 h **terminan** la sesión de la
/// app como antes de R8, en vez de dejarla esperando al tracker.
pub fn adopcion_imposible(entorno: &Entorno) -> bool {
    !tracker_soportado(entorno)
}

#[cfg(unix)]
fn es_ejecutable(ruta: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(ruta)
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn es_ejecutable(ruta: &Path) -> bool {
    ruta.is_file()
}

/// Token nuevo: 32 hex de 16 bytes del generador del sistema. No es el id de la fila:
/// los ids se reutilizan después de un restore (3.10.1).
fn token_nuevo() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| format!("Failed to generate session token: {e}"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Los primeros 8 caracteres del token: lo único que se loguea (3.10.3).
fn corto(token: &str) -> &str {
    token.get(..8).unwrap_or(token)
}

fn en_papelera(nombre: &str) -> String {
    format!("{nombre} está en la papelera: restauralo para trabajar en él.")
}

const PROYECTO_INEXISTENTE: &str =
    "El proyecto ya no existe. Recargá la lista de proyectos y probá de nuevo.";

/// Respuesta de `trabajar`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RespuestaTrabajar {
    pub token: String,
    pub project_id: i64,
    pub project_name: String,
    /// `app` o `demonio`: quién mide la sesión.
    pub dueno: String,
    /// La app soltó la sesión que medía en otro proyecto y le guardó tiempo (antes
    /// `previous_session_stopped`): la tarjeta de ese proyecto tiene que recargarse.
    pub sesion_anterior: bool,
    /// Se creó la carpeta `.gestor/` del proyecto.
    pub tracking_initialized: bool,
    /// Mide la app sin el tracker que se esperaba: toast de 3.10.2 (U6). Con la
    /// adopción imposible no hay toast (lo dicen la tarjeta y la cabecera).
    pub aviso_sin_tracker: bool,
}

/// Crea la carpeta `.gestor/` del proyecto si falta (como Trabajar antes de R8).
/// Devuelve `true` si la creó.
fn inicializar_gestor(project_id: i64, nombre: &str, ruta: &str) -> Result<bool, String> {
    let carpeta = Path::new(ruta);
    if !carpeta.exists() || has_tracking_config(carpeta) {
        return Ok(false);
    }
    debug!(
        "📁 [WORK] Inicializando tracking automáticamente para: {}",
        nombre
    );
    init_gestor_config(carpeta, project_id, nombre.to_string())
        .map_err(|e| format!("Error initializing tracking: {}", e))?;
    Ok(true)
}

/// Trabajar en `project_id` (3.10.6, 3.10.7): crea o reutiliza la sesión del proyecto
/// en la base y abre la terminal con su marca (`abrir`).
///
/// - Rechaza un proyecto en la papelera y, durante un restore, `restore.lock`.
/// - Reutiliza el token vivo del proyecto (D-R8-4) y anota el clic.
/// - El dueño es el demonio si está vivo; si no, la app, que mide un proyecto a la
///   vez: la sesión que medía en otro proyecto cierra su tramo con
///   `cambio_de_proyecto` y queda esperando al tracker (o termina, si la adopción es
///   imposible).
/// - Si la terminal no abre, una sesión recién creada se borra.
/// - Si el token reutilizado se cerró entre la transacción y la terminal (carrera con
///   un cierre, 3.10.2), se crea uno nuevo y se relanza la terminal con esa marca.
pub(crate) fn trabajar_sync(
    db: &Database,
    active_session: &ActiveSession,
    entorno: &Entorno,
    reloj: Reloj,
    project_id: i64,
    mut abrir: impl FnMut(&str, &MarcaSesion) -> Result<(), String>,
) -> Result<RespuestaTrabajar, String> {
    let (nombre, ruta, en_la_papelera) = db
        .proyecto_para_trabajar(project_id)
        .map_err(|e| format!("Error getting project: {}", e))?
        .ok_or_else(|| PROYECTO_INEXISTENTE.to_string())?;
    if en_la_papelera {
        return Err(en_papelera(&nombre));
    }
    let tracking_initialized = inicializar_gestor(project_id, &nombre, &ruta)?;

    let mut medicion = active_session.tomar()?;
    let imposible = adopcion_imposible(entorno);
    let mut sesion_anterior = false;
    let mut puede_reintentar = true;

    loop {
        let inicio = {
            // Compartido y solo mientras dura la transacción (3.10.7).
            let _restore = restore_lock::tomar_compartido(&entorno.data_dir)?;
            let token = token_nuevo()?;
            db.iniciar_trabajar(&PedidoTrabajar {
                project_id,
                ahora: reloj.pared,
                token_nuevo: &token,
                medida_por_la_app: medicion.as_ref().map(|m| m.medido(reloj.mono)),
                adopcion_imposible: imposible,
            })
            .map_err(|e| format!("Error starting work session: {}", e))?
        };

        let (token, dueno, nueva) = match inicio {
            InicioTrabajar::NoExiste => return Err(PROYECTO_INEXISTENTE.to_string()),
            InicioTrabajar::EnPapelera => return Err(en_papelera(&nombre)),
            InicioTrabajar::Iniciada {
                token,
                dueno,
                nueva,
                medicion: medicion_nueva,
                guardado_anterior,
            } => {
                sesion_anterior |= guardado_anterior.is_some_and(|segundos| segundos > 0);
                match medicion_nueva {
                    MedicionTrabajar::Sigue => {}
                    MedicionTrabajar::Nueva(tramo_id) => {
                        *medicion = Some(MedicionApp {
                            project_id,
                            token: token.clone(),
                            tramo_id,
                            inicio: reloj.mono,
                        });
                    }
                    MedicionTrabajar::Ninguna => *medicion = None,
                }
                (token, dueno, nueva)
            }
        };

        let marca = MarcaSesion::desde_token(&token)
            .ok_or_else(|| "Invalid session token in the database".to_string())?;
        if let Err(e) = abrir(&ruta, &marca) {
            if nueva {
                db.descartar_sesion_nueva(&token)
                    .map_err(|e| format!("Error discarding work session: {}", e))?;
                if medicion.as_ref().is_some_and(|m| m.token == token) {
                    *medicion = None;
                }
            }
            return Err(e);
        }

        if !nueva
            && puede_reintentar
            && db
                .sesion_terminada(&token)
                .map_err(|e| format!("Error reading work session: {}", e))?
        {
            warn!(
                "⚠️ [WORK] La sesión {} se cerró mientras se abría la terminal: se crea otra",
                corto(&token)
            );
            if medicion.as_ref().is_some_and(|m| m.token == token) {
                *medicion = None;
            }
            puede_reintentar = false;
            continue;
        }

        info!(
            "✅ [WORK] Trabajar en {}: sesión {} ({}, {})",
            nombre,
            corto(&token),
            dueno.como_texto(),
            if nueva { "nueva" } else { "reutilizada" }
        );
        return Ok(RespuestaTrabajar {
            token,
            project_id,
            project_name: nombre,
            dueno: dueno.como_texto().to_string(),
            sesion_anterior,
            tracking_initialized,
            aviso_sin_tracker: dueno == Dueno::App && !imposible,
        });
    }
}

/// Parar (3.10.9): termina la sesión `token` del proyecto (la que muestra la tarjeta)
/// o, sin token, su sesión viva. Si la medía la app, su tramo se cierra con el
/// `Instant` (regla de los 10 s y tope de 12 h). Un error de base sube y la app sigue
/// midiendo.
pub(crate) fn parar_sync(
    db: &Database,
    active_session: &ActiveSession,
    reloj: Reloj,
    project_id: i64,
    token: Option<&str>,
) -> Result<ResultadoParar, String> {
    let mut medicion = active_session.tomar()?;
    let parada = db
        .parar_trabajar(
            project_id,
            token,
            reloj.pared,
            MotivoSesion::PararApp,
            medicion.as_ref().map(|m| m.medido(reloj.mono)),
        )
        .map_err(|e| format!("Error stopping session: {}", e))?;

    if parada.token.is_some() && medicion.as_ref().map(|m| &m.token) == parada.token.as_ref() {
        *medicion = None;
    }
    Ok(parada.resultado)
}

/// Cerrar la ventana (D-R8-1, 3.10.11): ninguna sesión termina por eso. La que mide
/// la app cierra su tramo con el `Instant` (`cierre_app`) y pasa a esperar al tracker;
/// con la adopción imposible termina con `cierre_app`, como antes de R8. Las del
/// demonio no se tocan. Devuelve los segundos guardados, o `None` si la app no medía
/// nada.
pub(crate) fn al_cerrar_ventana_sync(
    db: &Database,
    active_session: &ActiveSession,
    entorno: &Entorno,
    reloj: Reloj,
) -> Result<Option<i64>, String> {
    let mut medicion = active_session.tomar()?;
    let Some(medida) = medicion.as_ref() else {
        return Ok(None);
    };
    let guardado = db
        .soltar_medicion_app(
            &medida.medido(reloj.mono),
            reloj.pared,
            MotivoSesion::CierreApp,
            adopcion_imposible(entorno),
        )
        .map_err(|e| format!("Error stopping session: {}", e))?;
    *medicion = None;
    Ok(Some(guardado.unwrap_or(0)))
}

/// `al_cerrar_ventana_sync` con el entorno y el reloj reales, para el
/// `CloseRequested` de `main.rs`.
pub fn al_cerrar_ventana(
    db: &Database,
    active_session: &ActiveSession,
) -> Result<Option<i64>, String> {
    let entorno = Entorno::del_sistema()?;
    al_cerrar_ventana_sync(db, active_session, &entorno, Reloj::ahora())
}

/// Mandar a la papelera (3.10.9, commit 18): el soft-delete del proyecto y sus
/// descendientes corta sus sesiones vivas en la misma transacción. Si la app medía una,
/// su tramo se cierra antes con el `Instant` (orden `ActiveSession → Database`) y la
/// app deja de medirla.
pub(crate) fn mandar_a_papelera_sync(
    db: &Database,
    active_session: &ActiveSession,
    reloj: Reloj,
    project_id: i64,
) -> Result<Vec<SesionCortada>, String> {
    let mut medicion = active_session.tomar()?;
    let cortadas = db
        .delete_project(
            project_id,
            reloj.pared,
            medicion.as_ref().map(|m| m.medido(reloj.mono)),
        )
        .map_err(|e| format!("Error deleting project: {}", e))?;

    if let Some(medida) = medicion.as_ref() {
        if cortadas.iter().any(|cortada| cortada.token == medida.token) {
            *medicion = None;
        }
    }
    for cortada in &cortadas {
        info!(
            "🗑️ [WORK] Sesión {} cortada: su proyecto fue a la papelera",
            corto(&cortada.token)
        );
    }
    Ok(cortadas)
}

/// Cada cuánto late la sesión que mide la app (§3.2): dos latidos son los 2 min que
/// espera la recuperación de huérfanas antes de tocar una sesión.
pub const LATIDO_TRABAJAR: std::time::Duration = std::time::Duration::from_secs(60);

/// Latido de la única `MedicionApp` (commit 19): `last_activity_at` y lo medido con el
/// `Instant`. Al llegar al tope de 12 h cierra el tramo con `tope_sin_tracker` y la app
/// deja de medir; lo mismo si el tramo ya estaba cerrado. Sin medición, no hace nada.
pub(crate) fn latir_trabajar_sync(
    db: &Database,
    active_session: &ActiveSession,
    entorno: &Entorno,
    reloj: Reloj,
) -> Result<Option<LatidoApp>, String> {
    let mut medicion = active_session.tomar()?;
    let Some(medida) = medicion.as_ref() else {
        return Ok(None);
    };
    let latido = db
        .latir_medicion_app(
            &medida.medido(reloj.mono),
            reloj.pared,
            adopcion_imposible(entorno),
        )
        .map_err(|e| format!("Error writing work session heartbeat: {}", e))?;

    match latido {
        LatidoApp::Latio => {}
        LatidoApp::Tope => {
            info!(
                "⏹️ [WORK] La sesión {} llegó a 12 h sin el tracker y se cortó",
                corto(&medida.token)
            );
            *medicion = None;
        }
        LatidoApp::Cerrado => *medicion = None,
    }
    Ok(Some(latido))
}

/// Latido de Trabajar en un hilo propio, cada 60 s, mientras viva la app. Lanzado
/// desde `.setup()`: el estado ya está registrado. Un error se loguea y el hilo sigue.
pub fn lanzar_latido(handle: tauri::AppHandle) {
    use tauri::Manager;

    std::thread::Builder::new()
        .name("latido-trabajar".to_string())
        .spawn(move || loop {
            std::thread::sleep(LATIDO_TRABAJAR);
            let db = handle.state::<Database>();
            let active_session = handle.state::<ActiveSession>();
            let latido = Entorno::del_sistema().and_then(|entorno| {
                latir_trabajar_sync(&db, &active_session, &entorno, Reloj::ahora())
            });
            if let Err(e) = latido {
                warn!("⚠️ [WORK] No se pudo escribir el latido de Trabajar: {}", e);
            }
        })
        .map(|_| ())
        .unwrap_or_else(|e| warn!("⚠️ [WORK] No se pudo lanzar el latido de Trabajar: {}", e));
}

/// Recuperación de huérfanas `work_button` al arrancar (§3.2, 3.10.11, commit 19): las
/// sesiones que la app medía cuando se cortó sin cerrar la ventana.
pub(crate) fn recuperar_huerfanas_sync(
    db: &Database,
    entorno: &Entorno,
    reloj: Reloj,
) -> Result<usize, String> {
    db.recuperar_huerfanas_app(reloj.pared, adopcion_imposible(entorno))
        .map_err(|e| format!("Error recovering orphan sessions: {}", e))
}

/// `recuperar_huerfanas_sync` con el entorno y el reloj reales, para `.setup()`.
pub fn recuperar_huerfanas(db: &Database) -> Result<usize, String> {
    let entorno = Entorno::del_sistema()?;
    recuperar_huerfanas_sync(db, &entorno, Reloj::ahora())
}

/// Lo que decide el aviso de cabecera sin tracker (U6, 3.10.14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntradaAviso<'a> {
    pub adopcion_imposible: bool,
    /// Hay fila en `tracker_status`: el demonio corrió alguna vez contra esta base.
    pub tracker_corrio: bool,
    /// `tracker_status.estado`, si hay fila.
    pub estado_tracker: Option<&'a str>,
    /// Sondeos seguidos, contando este, con `demonio_vivo()` falso.
    pub sondeos_sin_demonio: u32,
    /// Hay una sesión `dueno = app` viva.
    pub hay_sesion_app: bool,
}

/// Qué muestra la cabecera. Los textos los pone la UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct AvisoCabecera {
    /// (a) "El tracker no está corriendo: no se mide el tiempo automático; Trabajar
    /// mide sin pausa (hasta 12 h)".
    pub alarma: bool,
    /// (b) "Trabajar mide sin vigilancia: no se pausa por inactividad y se corta a las
    /// 12 h".
    pub linea_trabajar: bool,
}

/// La regla única del aviso de cabecera (3.10.14):
///
/// - (a) **Alarma** solo si el tracker se espera (adopción posible), corrió alguna vez
///   (fila en `tracker_status`), no responde en dos sondeos seguidos y no está
///   `esperando_migracion` (transitorio: lo resuelve la app al migrar).
/// - (b) **Línea de Trabajar** mientras viva una sesión `dueno = app`, en cualquier
///   contexto. Si también vale (a), se ven las dos.
/// - (c) y (d): sin sesión de la app y sin tracker esperado, nada.
pub fn aviso_cabecera(entrada: &EntradaAviso) -> AvisoCabecera {
    AvisoCabecera {
        alarma: !entrada.adopcion_imposible
            && entrada.tracker_corrio
            && entrada.sondeos_sin_demonio >= SONDEOS_PARA_LA_ALARMA
            && entrada.estado_tracker != Some("esperando_migracion"),
        linea_trabajar: entrada.hay_sesion_app,
    }
}

/// Una sesión viva como la ve la tarjeta: la fila de la base más lo que mide la app.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SesionDeLaTarjeta {
    #[serde(flatten)]
    pub sesion: SesionViva,
    /// Segundos del `Instant` si es la sesión que mide la app.
    pub elapsed_seconds: Option<i64>,
}

/// Respuesta de `estado_trabajar` (3.10.7).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct EstadoTrabajar {
    pub sesiones: Vec<SesionDeLaTarjeta>,
    pub demonio_vivo: bool,
    pub adopcion_imposible: bool,
    /// Hay fila en `tracker_status`.
    pub tracker_corrio: bool,
    /// Sondeos seguidos sin demonio, contando este: la cabecera lo devuelve en el
    /// sondeo siguiente.
    pub sondeos_sin_demonio: u32,
    pub aviso: AvisoCabecera,
}

/// Las sesiones vivas de la base y el estado del tracker (3.10.7). La cabecera pasa
/// los sondeos seguidos sin demonio que le devolvió el sondeo anterior; la tarjeta no
/// pasa nada (cuenta como el primero).
pub(crate) fn estado_trabajar_sync(
    db: &Database,
    active_session: &ActiveSession,
    entorno: &Entorno,
    reloj: Reloj,
    sondeos_previos: Option<u32>,
) -> Result<EstadoTrabajar, String> {
    // Se copia lo que mide la app y se suelta el lock antes de tocar la base.
    let medida = active_session
        .tomar()?
        .as_ref()
        .map(|m| (m.token.clone(), m.inicio));

    let leer = |e: rusqlite::Error| format!("Error reading work sessions: {}", e);
    let sesiones: Vec<SesionDeLaTarjeta> = db
        .sesiones_vivas()
        .map_err(leer)?
        .into_iter()
        .map(|sesion| {
            let elapsed_seconds = medida
                .as_ref()
                .filter(|(token, _)| *token == sesion.token)
                .map(|(_, inicio)| segundos_desde(*inicio, reloj.mono));
            SesionDeLaTarjeta {
                sesion,
                elapsed_seconds,
            }
        })
        .collect();
    let demonio_vivo = db.demonio_vivo(reloj.pared).map_err(leer)?;
    let tracker_status = db.leer_tracker_status().map_err(leer)?;
    let imposible = adopcion_imposible(entorno);
    let sondeos_sin_demonio = if demonio_vivo {
        0
    } else {
        sondeos_previos.unwrap_or(0).saturating_add(1)
    };

    let aviso = aviso_cabecera(&EntradaAviso {
        adopcion_imposible: imposible,
        tracker_corrio: tracker_status.is_some(),
        estado_tracker: tracker_status.as_ref().map(|t| t.estado.as_str()),
        sondeos_sin_demonio,
        hay_sesion_app: sesiones
            .iter()
            .any(|s| s.sesion.dueno == Dueno::App.como_texto()),
    });

    Ok(EstadoTrabajar {
        sesiones,
        demonio_vivo,
        adopcion_imposible: imposible,
        tracker_corrio: tracker_status.is_some(),
        sondeos_sin_demonio,
        aviso,
    })
}

/// Trabajar: crea o reutiliza la sesión del proyecto y abre la terminal con su marca.
#[tauri::command]
pub async fn trabajar(
    db: State<'_, Database>,
    config_manager: State<'_, ConfigManager>,
    active_session: State<'_, ActiveSession>,
    project_id: i64,
) -> Result<RespuestaTrabajar, String> {
    debug!("🚀 [WORK] Trabajar en el proyecto ID: {}", project_id);
    let entorno = Entorno::del_sistema()?;
    let config = config_manager.get_config()?;
    let plataforma = get_platform();
    trabajar_sync(
        &db,
        &active_session,
        &entorno,
        Reloj::ahora(),
        project_id,
        |ruta, marca| plataforma.open_terminal(ruta, &config, Some(marca)),
    )
}

/// Parar la sesión de un proyecto: `token` es el que muestra la tarjeta.
#[tauri::command]
pub async fn parar(
    db: State<'_, Database>,
    active_session: State<'_, ActiveSession>,
    project_id: i64,
    token: Option<String>,
) -> Result<ResultadoParar, String> {
    debug!("⏹️ [WORK] Parar en el proyecto ID: {}", project_id);
    parar_sync(
        &db,
        &active_session,
        Reloj::ahora(),
        project_id,
        token.as_deref(),
    )
}

/// Las sesiones vivas y el estado del tracker, para la tarjeta y la cabecera.
#[tauri::command]
pub async fn estado_trabajar(
    db: State<'_, Database>,
    active_session: State<'_, ActiveSession>,
    sondeos_sin_demonio: Option<u32>,
) -> Result<EstadoTrabajar, String> {
    let entorno = Entorno::del_sistema()?;
    estado_trabajar_sync(
        &db,
        &active_session,
        &entorno,
        Reloj::ahora(),
        sondeos_sin_demonio,
    )
}

#[cfg(test)]
mod tests {
    //! Reescriben `work_session_tests` (§5.6 del diseño del tracker) contra `trabajar`,
    //! `parar`, `al_cerrar_ventana` y `estado_trabajar`, y suman los del commit 17.

    use super::*;
    use crate::commands::duracion_a_guardar;
    use crate::models::project::CreateProjectDTO;
    use rusqlite::params;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;

    /// Base en memoria, `ActiveSession` vacía y un entorno en un `tempdir`: con o sin
    /// un `gestor-trackerd` falso en su `BIN_DIR`.
    struct Prueba {
        db: Database,
        active: ActiveSession,
        entorno: Entorno,
        /// El `Instant` de referencia: cada acción del test lo corre hacia adelante.
        t0: Instant,
        dir: TempDir,
    }

    impl Prueba {
        fn nueva(con_binario_del_tracker: bool) -> Self {
            let dir = TempDir::new().unwrap();
            let bin_dir = dir.path().join("bin");
            let data_dir = dir.path().join("data");
            std::fs::create_dir_all(&bin_dir).unwrap();
            std::fs::create_dir_all(&data_dir).unwrap();
            if con_binario_del_tracker {
                let binario = bin_dir.join(BINARIO_DEL_TRACKER);
                std::fs::write(&binario, "#!/bin/sh\n").unwrap();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&binario, std::fs::Permissions::from_mode(0o755))
                        .unwrap();
                }
            }
            Self {
                db: Database::new(PathBuf::from(":memory:")).unwrap(),
                active: ActiveSession::default(),
                entorno: Entorno {
                    data_dir,
                    bin_dir,
                    linux: true,
                    sin_tracker_por_variable: false,
                },
                t0: Instant::now(),
                dir,
            }
        }

        /// Sin el binario del tracker: la adopción es imposible, como en todo F1–F5.
        fn sin_tracker() -> Self {
            Self::nueva(false)
        }

        /// Con un `gestor-trackerd` falso en `BIN_DIR`: la adopción es posible.
        fn con_tracker() -> Self {
            Self::nueva(true)
        }

        fn carpeta(&self, nombre: &str) -> String {
            self.dir.path().join(nombre).to_string_lossy().into_owned()
        }

        fn proyecto(&self, nombre: &str) -> i64 {
            self.proyecto_en(nombre, None)
        }

        fn proyecto_en(&self, nombre: &str, padre: Option<i64>) -> i64 {
            self.db
                .create_project(CreateProjectDTO {
                    name: nombre.to_string(),
                    description: "desc".to_string(),
                    // Dentro del `tempdir` y sin crear: Trabajar no le escribe `.gestor/`.
                    local_path: self.carpeta(nombre),
                    documentation_url: None,
                    ai_documentation_url: None,
                    drive_link: None,
                    notes: None,
                    image_data: None,
                    parent_id: padre,
                    group_color: None,
                    group_icon: None,
                })
                .unwrap()
                .id
        }

        /// El reloj `segundos` después de `t0`, con la pared a partir de las 09:00.
        fn reloj(&self, segundos: u64) -> Reloj {
            Reloj {
                pared: crate::time::parsear("2026-10-20 09:00:00").unwrap()
                    + chrono::Duration::seconds(segundos as i64),
                mono: self.t0 + Duration::from_secs(segundos),
            }
        }

        fn trabajar(&self, project_id: i64, segundos: u64) -> Result<RespuestaTrabajar, String> {
            trabajar_sync(
                &self.db,
                &self.active,
                &self.entorno,
                self.reloj(segundos),
                project_id,
                |_, _| Ok(()),
            )
        }

        fn parar(
            &self,
            project_id: i64,
            token: Option<&str>,
            segundos: u64,
        ) -> Result<ResultadoParar, String> {
            parar_sync(
                &self.db,
                &self.active,
                self.reloj(segundos),
                project_id,
                token,
            )
        }

        fn papelera(&self, project_id: i64, segundos: u64) -> Result<Vec<SesionCortada>, String> {
            mandar_a_papelera_sync(&self.db, &self.active, self.reloj(segundos), project_id)
        }

        fn latir(&self, segundos: u64) -> Option<LatidoApp> {
            latir_trabajar_sync(&self.db, &self.active, &self.entorno, self.reloj(segundos))
                .unwrap()
        }

        fn recuperar(&self, segundos: u64) -> usize {
            recuperar_huerfanas_sync(&self.db, &self.entorno, self.reloj(segundos)).unwrap()
        }

        /// La app se cortó sin cerrar la ventana: la memoria se pierde, la base queda.
        fn cortar_la_app(&self) {
            *self.active.0.lock().unwrap() = None;
        }

        /// Siembra un tramo abierto y devuelve su id.
        fn sembrar_tramo(
            &self,
            project_id: i64,
            source: &str,
            started_at: &str,
            last_activity_at: Option<&str>,
            duration_seconds: Option<i64>,
        ) -> i64 {
            self.db.execute_batch_para_tests(&format!(
                "INSERT INTO time_tracking_sessions
                     (project_id, started_at, last_activity_at, duration_seconds, source)
                 VALUES ({project_id}, '{started_at}', {}, {}, '{source}');",
                last_activity_at.map_or("NULL".to_string(), |t| format!("'{t}'")),
                duration_seconds.map_or("NULL".to_string(), |d| d.to_string()),
            ));
            self.contar("SELECT MAX(id) FROM time_tracking_sessions")
        }

        fn cerrar_ventana(&self, segundos: u64) -> Result<Option<i64>, String> {
            al_cerrar_ventana_sync(&self.db, &self.active, &self.entorno, self.reloj(segundos))
        }

        fn estado(&self, segundos: u64, sondeos: Option<u32>) -> EstadoTrabajar {
            estado_trabajar_sync(
                &self.db,
                &self.active,
                &self.entorno,
                self.reloj(segundos),
                sondeos,
            )
            .unwrap()
        }

        fn medicion(&self) -> Option<MedicionApp> {
            self.active.0.lock().unwrap().clone()
        }

        /// (dueno, estado, motivo_cierre) de una sesión.
        fn sesion(&self, token: &str) -> (String, String, Option<String>) {
            self.db.fila_para_tests(
                "SELECT dueno, estado, motivo_cierre FROM trabajar_sesiones WHERE token = ?1",
                params![token],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
        }

        /// (ended_at, duration_seconds, motivo_cierre) de un tramo.
        fn tramo(&self, id: i64) -> (Option<String>, Option<i64>, Option<String>) {
            self.db.fila_para_tests(
                "SELECT ended_at, duration_seconds, motivo_cierre
                 FROM time_tracking_sessions WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
        }

        fn contar(&self, sql: &str) -> i64 {
            self.db.fila_para_tests(sql, [], |row| row.get(0))
        }

        /// Hace fallar cualquier cierre de tramo (UPDATE) desde la base.
        fn romper_cierres(&self) {
            self.db.execute_batch_para_tests(
                "CREATE TRIGGER cierre_roto BEFORE UPDATE ON time_tracking_sessions
                 BEGIN SELECT RAISE(ABORT, 'cierre roto'); END;",
            );
        }
    }

    // ============ Un proyecto a la vez con la app (3.10.6, 3.10.7) ============

    /// A y B con la app y el tracker posible: Trabajar en B cierra el tramo de A con
    /// `cambio_de_proyecto` y A queda esperando al tracker (D-R8-1). Mientras se mide,
    /// A está en `viva_activa` y hay una sola `MedicionApp`.
    #[test]
    fn a_y_b_con_la_app_dejan_a_esperando_al_demonio_y_una_sola_medicion() {
        let p = Prueba::con_tracker();
        let a = p.proyecto("A");
        let b = p.proyecto("B");

        let en_a = p.trabajar(a, 0).unwrap();
        assert_eq!(en_a.dueno, "app");
        assert_eq!(
            p.sesion(&en_a.token),
            ("app".into(), "viva_activa".into(), None)
        );
        let tramo_a = p.medicion().unwrap().tramo_id;

        let en_b = p.trabajar(b, 600).unwrap();

        assert!(en_b.sesion_anterior, "A guardó sus 10 min");
        assert_ne!(en_a.token, en_b.token);
        assert_eq!(
            p.sesion(&en_a.token),
            ("demonio".into(), "esperando_demonio".into(), None),
            "A no termina: espera al tracker"
        );
        assert_eq!(
            p.tramo(tramo_a),
            (
                Some("2026-10-20 09:10:00".into()),
                Some(600),
                Some("cambio_de_proyecto".into())
            )
        );
        let medicion = p.medicion().unwrap();
        assert_eq!((medicion.project_id, medicion.token), (b, en_b.token));
        assert_eq!(
            p.contar("SELECT COUNT(*) FROM trabajar_sesiones WHERE dueno = 'app' AND estado <> 'cerrada'"),
            1,
            "invariante 11: a lo sumo una sesión viva de la app"
        );
    }

    /// D-R8-4 (antes `two_rapid_start_calls_do_not_lose_any_session`): Trabajar dos
    /// veces en el mismo proyecto reutiliza el token, anota el clic y sigue con el
    /// mismo tramo y el mismo `Instant`.
    #[test]
    fn trabajar_dos_veces_en_el_mismo_proyecto_reutiliza_el_token() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");

        let primera = p.trabajar(a, 0).unwrap();
        let medicion = p.medicion().unwrap();
        let segunda = p.trabajar(a, 90).unwrap();

        assert_eq!(primera.token, segunda.token);
        assert!(!segunda.sesion_anterior);
        assert_eq!(p.medicion(), Some(medicion), "sigue la misma medición");
        let clic: String = p.db.fila_para_tests(
            "SELECT ultimo_clic_at FROM trabajar_sesiones WHERE token = ?1",
            params![primera.token],
            |row| row.get(0),
        );
        assert_eq!(clic, "2026-10-20 09:01:30");
        assert_eq!(
            p.contar("SELECT COUNT(*) FROM time_tracking_sessions WHERE ended_at IS NULL"),
            1
        );
        assert_eq!(p.contar("SELECT COUNT(*) FROM trabajar_sesiones"), 1);
    }

    /// Antes `concurrent_start_calls_serialize_without_orphaning_a_session`: dos
    /// Trabajar a la vez en el mismo proyecto dan el mismo token, una sola sesión y un
    /// solo tramo abierto.
    #[test]
    fn trabajar_a_la_vez_en_el_mismo_proyecto_da_un_solo_token() {
        let p = Arc::new(Prueba::sin_tracker());
        let a = p.proyecto("A");
        let barrera = Arc::new(std::sync::Barrier::new(2));

        let hilos: Vec<_> = (0..2)
            .map(|_| {
                let (p, barrera) = (Arc::clone(&p), Arc::clone(&barrera));
                std::thread::spawn(move || {
                    barrera.wait();
                    p.trabajar(a, 0).unwrap().token
                })
            })
            .collect();
        let tokens: Vec<String> = hilos.into_iter().map(|h| h.join().unwrap()).collect();

        assert_eq!(tokens[0], tokens[1]);
        assert_eq!(p.contar("SELECT COUNT(*) FROM trabajar_sesiones"), 1);
        assert_eq!(
            p.contar("SELECT COUNT(*) FROM time_tracking_sessions WHERE ended_at IS NULL"),
            1
        );
    }

    /// Una sesión del demonio sin demonio vivo: Trabajar la pasa a la app (3.10.7).
    /// El tramo que quedó abierto cierra en su última actividad con `tracker_caido`.
    #[test]
    fn trabajar_sobre_una_sesion_del_demonio_sin_demonio_la_toma_la_app() {
        let p = Prueba::con_tracker();
        let a = p.proyecto("A");
        let token = "0123456789abcdef0123456789abcdef";
        p.db.execute_batch_para_tests(&format!(
            "INSERT INTO trabajar_sesiones
                 (token, project_id, dueno, origen, estado, creada_at, ultimo_clic_at, adoptada_at)
             VALUES ('{token}', {a}, 'demonio', 'boton', 'viva_activa',
                     '2026-10-20 08:00:00', '2026-10-20 08:00:00', '2026-10-20 08:00:00');
             INSERT INTO time_tracking_sessions
                 (project_id, started_at, last_activity_at, duration_seconds, source, terminal_token)
             VALUES ({a}, '2026-10-20 08:00:00', '2026-10-20 08:20:00', 1200, 'terminal', '{token}');"
        ));

        let respuesta = p.trabajar(a, 0).unwrap();

        assert_eq!(respuesta.token, token);
        assert_eq!(respuesta.dueno, "app");
        assert_eq!(p.sesion(token), ("app".into(), "viva_activa".into(), None));
        let (fin, duracion, motivo): (String, i64, String) = p.db.fila_para_tests(
            "SELECT ended_at, duration_seconds, motivo_cierre FROM time_tracking_sessions
             WHERE source = 'terminal'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        );
        assert_eq!(
            (fin.as_str(), duracion, motivo.as_str()),
            ("2026-10-20 08:20:00", 1200, "tracker_caido")
        );
        assert!(p.medicion().is_some(), "la app mide el tramo nuevo");
    }

    // ============ Parar (3.10.9) ============

    #[test]
    fn parar_cierra_despues_no_hay_nada_y_con_el_token_cerrado_es_ya_cerrada() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        let token = p.trabajar(a, 0).unwrap().token;

        assert_eq!(
            p.parar(a, Some(&token), 75).unwrap(),
            ResultadoParar::Cerrada { segundos: 75 }
        );
        assert!(p.medicion().is_none());
        assert_eq!(
            p.sesion(&token),
            ("app".into(), "cerrada".into(), Some("parar_app".into()))
        );

        assert_eq!(p.parar(a, None, 80).unwrap(), ResultadoParar::Ninguna);
        assert_eq!(
            p.parar(a, Some(&token), 80).unwrap(),
            ResultadoParar::YaCerrada {
                motivo: "parar_app".into(),
                segundos: 75
            }
        );
    }

    /// Recorte a 12 h (U6) con un `Instant` de 13 h inyectado.
    #[test]
    fn parar_con_un_instant_de_13_h_guarda_43200() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        p.trabajar(a, 0).unwrap();
        let tramo = p.medicion().unwrap().tramo_id;

        assert_eq!(
            p.parar(a, None, 13 * 3600).unwrap(),
            ResultadoParar::Cerrada { segundos: 43_200 }
        );
        assert_eq!(p.tramo(tramo).1, Some(43_200));
    }

    // ============ Cerrar la ventana (3.10.11, U4) ============

    /// Antes `stop_active_session_sync_persists_the_open_session_on_close`: cerrar la
    /// ventana con el tracker posible cierra el tramo con `cierre_app` y deja el token
    /// esperando al demonio.
    #[test]
    fn cerrar_la_ventana_con_el_tracker_instalado_deja_el_token_esperando() {
        let p = Prueba::con_tracker();
        let a = p.proyecto("A");
        let token = p.trabajar(a, 0).unwrap().token;
        let tramo = p.medicion().unwrap().tramo_id;

        assert_eq!(p.cerrar_ventana(30).unwrap(), Some(30));

        assert_eq!(
            p.tramo(tramo),
            (
                Some("2026-10-20 09:00:30".into()),
                Some(30),
                Some("cierre_app".into())
            )
        );
        assert_eq!(
            p.sesion(&token),
            ("demonio".into(), "esperando_demonio".into(), None)
        );
        assert!(p.medicion().is_none());
    }

    /// F1–F5: sin el binario del tracker la adopción es imposible y la sesión termina
    /// como antes de R8.
    #[test]
    fn sin_el_binario_cerrar_la_ventana_termina_con_cierre_app() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        let token = p.trabajar(a, 0).unwrap().token;

        p.cerrar_ventana(30).unwrap();

        assert_eq!(
            p.sesion(&token),
            ("app".into(), "cerrada".into(), Some("cierre_app".into()))
        );
    }

    #[test]
    fn sin_el_binario_trabajar_en_otro_proyecto_termina_con_cambio_de_proyecto() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        let b = p.proyecto("B");
        let token_a = p.trabajar(a, 0).unwrap().token;
        let tramo_a = p.medicion().unwrap().tramo_id;

        p.trabajar(b, 30).unwrap();

        assert_eq!(
            p.sesion(&token_a),
            (
                "app".into(),
                "cerrada".into(),
                Some("cambio_de_proyecto".into())
            )
        );
        assert_eq!(p.tramo(tramo_a).2.as_deref(), Some("cambio_de_proyecto"));
    }

    #[test]
    fn con_windows_simulado_cerrar_la_ventana_termina_con_cierre_app() {
        let mut p = Prueba::con_tracker();
        p.entorno.linux = false;
        let a = p.proyecto("A");
        let token = p.trabajar(a, 0).unwrap().token;

        p.cerrar_ventana(30).unwrap();

        assert_eq!(p.sesion(&token).1, "cerrada");
        assert_eq!(p.sesion(&token).2.as_deref(), Some("cierre_app"));
    }

    /// Antes `stop_active_session_sync_is_a_noop_without_an_active_session`.
    #[test]
    fn cerrar_la_ventana_sin_medicion_no_hace_nada() {
        let p = Prueba::sin_tracker();
        assert_eq!(p.cerrar_ventana(0).unwrap(), None);
    }

    // ============ Terminal, restore.lock y carreras ============

    #[test]
    fn si_la_terminal_no_abre_la_sesion_nueva_se_borra() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");

        let err = trabajar_sync(&p.db, &p.active, &p.entorno, p.reloj(0), a, |_, _| {
            Err("No se encontró ninguna terminal.".to_string())
        })
        .unwrap_err();

        assert_eq!(err, "No se encontró ninguna terminal.");
        assert_eq!(p.contar("SELECT COUNT(*) FROM trabajar_sesiones"), 0);
        assert_eq!(p.contar("SELECT COUNT(*) FROM time_tracking_sessions"), 0);
        assert!(p.medicion().is_none());
    }

    /// La terminal nace con la marca del token de la sesión.
    #[test]
    fn la_terminal_se_abre_en_la_carpeta_con_la_marca_del_token() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        let mut abiertas = Vec::new();

        let respuesta = trabajar_sync(
            &p.db,
            &p.active,
            &p.entorno,
            p.reloj(0),
            a,
            |ruta, marca| {
                abiertas.push((ruta.to_string(), marca.token().to_string()));
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(abiertas, vec![(p.carpeta("A"), respuesta.token)]);
    }

    /// Con el restore en curso (`restore.lock` exclusivo), Trabajar rechaza sin crear
    /// nada (DT15).
    #[test]
    fn con_el_restore_en_curso_trabajar_rechaza_sin_crear_nada() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        let restore = std::fs::File::create(p.entorno.data_dir.join(restore_lock::NOMBRE)).unwrap();
        restore.try_lock().unwrap();

        let err = p.trabajar(a, 0).unwrap_err();

        assert_eq!(err, restore_lock::MENSAJE_RESTAURACION_EN_CURSO);
        assert_eq!(p.contar("SELECT COUNT(*) FROM trabajar_sesiones"), 0);
    }

    /// 3.10.2: si el token reutilizado se cerró entre la transacción y la terminal,
    /// Trabajar crea otro y relanza la terminal con esa marca.
    #[test]
    fn si_el_token_reutilizado_se_cerro_al_abrir_la_terminal_se_crea_otro() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        let primero = p.trabajar(a, 0).unwrap().token;
        let mut marcas = Vec::new();

        let respuesta = trabajar_sync(&p.db, &p.active, &p.entorno, p.reloj(60), a, |_, marca| {
            if marcas.is_empty() {
                // Otro proceso (la CLI, el demonio) cierra la sesión justo ahora.
                p.db.execute_batch_para_tests(&format!(
                    "UPDATE time_tracking_sessions SET ended_at = '2026-10-20 09:01:00',
                         duration_seconds = 60, motivo_cierre = 'parar_cli'
                     WHERE terminal_token = '{primero}' AND ended_at IS NULL;
                     UPDATE trabajar_sesiones SET estado = 'cerrada',
                         terminada_at = '2026-10-20 09:01:00', motivo_cierre = 'parar_cli'
                     WHERE token = '{primero}';"
                ));
            }
            marcas.push(marca.token().to_string());
            Ok(())
        })
        .unwrap();

        assert_ne!(respuesta.token, primero);
        assert_eq!(marcas, vec![primero, respuesta.token.clone()]);
        assert_eq!(p.sesion(&respuesta.token).1, "viva_activa");
        assert_eq!(p.medicion().unwrap().token, respuesta.token);
    }

    // ============ Papelera (3.10.9, commit 18) ============

    /// Un padre con una sesión del demonio (tramo `terminal` abierto) y un hijo con la
    /// sesión que mide la app. Devuelve (padre, hijo, otro, token del padre, token del
    /// hijo, tramo del hijo).
    fn padre_e_hijo_con_sesiones(p: &Prueba) -> (i64, i64, i64, String, String, i64) {
        let padre = p.proyecto("Padre");
        let hijo = p.proyecto_en("Hijo", Some(padre));
        let otro = p.proyecto("Otro");
        let token_padre = "0123456789abcdef0123456789abcdef".to_string();
        p.db.execute_batch_para_tests(&format!(
            "INSERT INTO trabajar_sesiones
                 (token, project_id, dueno, origen, estado, creada_at, ultimo_clic_at, adoptada_at)
             VALUES ('{token_padre}', {padre}, 'demonio', 'boton', 'viva_activa',
                     '2026-10-20 08:00:00', '2026-10-20 08:00:00', '2026-10-20 08:00:00');
             INSERT INTO time_tracking_sessions
                 (project_id, started_at, last_activity_at, duration_seconds, source, terminal_token)
             VALUES ({padre}, '2026-10-20 08:00:00', '2026-10-20 08:30:00', 1800, 'terminal', '{token_padre}');"
        ));
        let token_hijo = p.trabajar(hijo, 0).unwrap().token;
        let tramo_hijo = p.medicion().unwrap().tramo_id;
        (padre, hijo, otro, token_padre, token_hijo, tramo_hijo)
    }

    /// Papelera de un padre con sesiones en el padre y en el hijo: las dos terminan con
    /// `proyecto_a_papelera`. La del hijo, que medía la app, cierra con el `Instant`.
    #[test]
    fn la_papelera_de_un_padre_corta_las_sesiones_del_padre_y_del_hijo() {
        let p = Prueba::sin_tracker();
        let (padre, hijo, otro, token_padre, token_hijo, tramo_hijo) =
            padre_e_hijo_con_sesiones(&p);
        let token_otro = {
            // Una sesión del demonio en otro proyecto, que no se toca.
            p.db.execute_batch_para_tests(&format!(
                "INSERT INTO trabajar_sesiones
                     (token, project_id, dueno, origen, estado, creada_at, ultimo_clic_at)
                 VALUES ('fedcba9876543210fedcba9876543210', {otro}, 'demonio', 'boton',
                         'viva_pausada', '2026-10-20 08:00:00', '2026-10-20 08:00:00');"
            ));
            "fedcba9876543210fedcba9876543210"
        };

        let mut cortadas = p.papelera(padre, 600).unwrap();
        cortadas.sort_by_key(|c| c.project_id);

        let papelera = Some("proyecto_a_papelera".to_string());
        assert_eq!(p.sesion(&token_padre).1, "cerrada");
        assert_eq!(p.sesion(&token_padre).2, papelera);
        assert_eq!(p.sesion(&token_hijo).1, "cerrada");
        assert_eq!(p.sesion(&token_hijo).2, papelera);
        assert_eq!(
            p.tramo(tramo_hijo),
            (
                Some("2026-10-20 09:10:00".into()),
                Some(600),
                papelera.clone()
            ),
            "el tramo de la app cierra con el Instant"
        );
        assert!(p.medicion().is_none(), "la app deja de medir");
        assert_eq!(
            cortadas
                .iter()
                .map(|c| (c.project_id, c.nombre.as_str(), c.segundos))
                .collect::<Vec<_>>(),
            vec![(padre, "Padre", 1800), (hijo, "Hijo", 600)]
        );
        assert_eq!(p.sesion(token_otro).1, "viva_pausada");
        assert_eq!(
            p.contar(
                "SELECT COUNT(*) FROM trabajar_sesiones s JOIN projects p ON p.id = s.project_id
                 WHERE p.deleted_at IS NOT NULL AND s.estado <> 'cerrada'"
            ),
            0,
            "invariante 12: ninguna sesión viva en la papelera"
        );
    }

    /// El corte va en la transacción del soft-delete: si el `UPDATE` falla, no queda
    /// ninguna sesión cortada y la app sigue midiendo.
    #[test]
    fn si_el_update_de_la_papelera_falla_no_queda_ninguna_sesion_cortada() {
        let p = Prueba::sin_tracker();
        let (padre, _, _, token_padre, token_hijo, tramo_hijo) = padre_e_hijo_con_sesiones(&p);
        let medicion = p.medicion();
        p.db.execute_batch_para_tests(
            "CREATE TRIGGER papelera_rota BEFORE UPDATE OF deleted_at ON projects
             BEGIN SELECT RAISE(ABORT, 'papelera rota'); END;",
        );

        let err = p.papelera(padre, 600).unwrap_err();

        assert!(err.contains("papelera rota"), "mensaje: {err}");
        assert_eq!(p.sesion(&token_padre).1, "viva_activa");
        assert_eq!(p.sesion(&token_hijo).1, "viva_activa");
        assert_eq!(
            p.tramo(tramo_hijo).0,
            None,
            "el tramo de la app sigue abierto"
        );
        assert_eq!(p.medicion(), medicion);
        assert_eq!(
            p.contar("SELECT COUNT(*) FROM projects WHERE deleted_at IS NOT NULL"),
            0
        );
    }

    #[test]
    fn trabajar_sobre_un_proyecto_en_la_papelera_es_un_error_en_espanol() {
        let p = Prueba::sin_tracker();
        let padre = p.proyecto("Padre");
        let hijo = p.proyecto_en("Hijo", Some(padre));
        assert!(p.papelera(padre, 0).unwrap().is_empty());

        let err = p.trabajar(hijo, 60).unwrap_err();

        assert_eq!(
            err,
            "Hijo está en la papelera: restauralo para trabajar en él."
        );
        assert_eq!(p.contar("SELECT COUNT(*) FROM trabajar_sesiones"), 0);
    }

    // ============ Latido y huérfanas (commit 19) ============

    /// El latido escribe la última actividad y lo medido con el `Instant`.
    #[test]
    fn el_latido_escribe_la_ultima_actividad_y_lo_medido() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        p.trabajar(a, 0).unwrap();
        let tramo = p.medicion().unwrap().tramo_id;

        assert_eq!(p.latir(60), Some(LatidoApp::Latio));

        let (ultima, medido): (String, i64) = p.db.fila_para_tests(
            "SELECT last_activity_at, duration_seconds FROM time_tracking_sessions WHERE id = ?1",
            params![tramo],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );
        assert_eq!((ultima.as_str(), medido), ("2026-10-20 09:01:00", 60));
        assert_eq!(p.tramo(tramo).0, None, "el tramo sigue abierto");
        assert!(p.medicion().is_some());
    }

    #[test]
    fn sin_medicion_el_latido_no_hace_nada() {
        let p = Prueba::sin_tracker();
        assert_eq!(p.latir(60), None);
    }

    /// Si el tramo ya no está abierto (Parar desde la CLI, un restore), la app suelta su
    /// medición.
    #[test]
    fn el_latido_de_un_tramo_cerrado_suelta_la_medicion() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        let token = p.trabajar(a, 0).unwrap().token;
        p.db.execute_batch_para_tests(&format!(
            "UPDATE time_tracking_sessions SET ended_at = '2026-10-20 09:00:30',
                 duration_seconds = 30, motivo_cierre = 'parar_cli'
             WHERE terminal_token = '{token}';"
        ));

        assert_eq!(p.latir(60), Some(LatidoApp::Cerrado));
        assert!(p.medicion().is_none());
    }

    /// U6: el latido a las 12 h + 1 s cierra el tramo en `started_at + 12 h` con
    /// 43200 s y `tope_sin_tracker`, y el token queda esperando al tracker.
    #[test]
    fn el_latido_a_las_12_h_corta_con_tope_y_deja_el_token_esperando() {
        let p = Prueba::con_tracker();
        let a = p.proyecto("A");
        let token = p.trabajar(a, 0).unwrap().token;
        let tramo = p.medicion().unwrap().tramo_id;
        assert_eq!(p.latir(43_140), Some(LatidoApp::Latio));

        assert_eq!(p.latir(43_201), Some(LatidoApp::Tope));

        assert_eq!(
            p.tramo(tramo),
            (
                Some("2026-10-20 21:00:00".into()),
                Some(43_200),
                Some("tope_sin_tracker".into())
            )
        );
        assert_eq!(
            p.sesion(&token),
            ("demonio".into(), "esperando_demonio".into(), None)
        );
        assert!(p.medicion().is_none(), "la app deja de medir");
    }

    #[test]
    fn con_windows_simulado_el_latido_a_las_12_h_termina_con_tope_sin_tracker() {
        let mut p = Prueba::con_tracker();
        p.entorno.linux = false;
        let a = p.proyecto("A");
        let token = p.trabajar(a, 0).unwrap().token;

        assert_eq!(p.latir(43_201), Some(LatidoApp::Tope));

        assert_eq!(
            p.sesion(&token),
            (
                "app".into(),
                "cerrada".into(),
                Some("tope_sin_tracker".into())
            )
        );
    }

    /// Con una suspensión entre latidos, la huérfana cierra con lo medido por el
    /// `Instant` (50 min), no con `last_activity_at − started_at` (2 h).
    #[test]
    fn una_huerfana_con_latido_y_suspension_cierra_con_la_duracion_no_con_la_diferencia() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        let id = p.sembrar_tramo(
            a,
            "work_button",
            "2026-10-20 06:00:00",
            Some("2026-10-20 08:00:00"),
            Some(3000),
        );

        assert_eq!(p.recuperar(0), 1);

        assert_eq!(
            p.tramo(id),
            (
                Some("2026-10-20 08:00:00".into()),
                Some(3000),
                Some("huerfana".into())
            )
        );
    }

    #[test]
    fn una_huerfana_sin_latido_cierra_en_su_inicio_con_0_s() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        let sin_ultima = p.sembrar_tramo(a, "work_button", "2026-10-20 06:00:00", None, None);
        let abierta = p.sembrar_tramo(
            a,
            "work_button",
            "2026-10-20 07:00:00",
            Some("2026-10-20 07:00:00"),
            None,
        );

        assert_eq!(p.recuperar(0), 2);

        assert_eq!(
            p.tramo(sin_ultima),
            (
                Some("2026-10-20 06:00:00".into()),
                Some(0),
                Some("huerfana".into())
            )
        );
        assert_eq!(p.tramo(abierta).1, Some(0));
    }

    /// Un latido de hace 2 min o menos es de una sesión viva (otra instancia): no se
    /// toca.
    #[test]
    fn la_sesion_de_otra_instancia_con_latido_reciente_queda_intacta() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        let hace_1_min = p.sembrar_tramo(
            a,
            "work_button",
            "2026-10-20 08:00:00",
            Some("2026-10-20 08:59:00"),
            Some(3540),
        );
        let hace_2_min = p.sembrar_tramo(
            a,
            "work_button",
            "2026-10-20 08:00:00",
            Some("2026-10-20 08:58:00"),
            Some(3480),
        );

        assert_eq!(p.recuperar(0), 0);

        assert_eq!(p.tramo(hace_1_min), (None, Some(3540), None));
        assert_eq!(p.tramo(hace_2_min), (None, Some(3480), None));
    }

    /// Las 81 huérfanas reales no tienen latido ni duración: cierran con 0 s y
    /// `huerfana`, el total no cambia y no se toca ninguna sesión de Trabajar.
    #[test]
    fn las_81_huerfanas_sin_latido_no_cambian_el_total() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        p.db.execute_batch_para_tests(&format!(
            "INSERT INTO time_tracking_sessions
                 (project_id, started_at, ended_at, duration_seconds, source)
             VALUES ({a}, '2026-10-01 10:00:00', '2026-10-01 11:00:00', 3600, 'work_button');"
        ));
        for dia in 0..81 {
            p.sembrar_tramo(
                a,
                "work_button",
                &format!("2026-07-{:02} 10:00:00", dia % 28 + 1),
                None,
                None,
            );
        }
        let otro = p.proyecto("Otro");
        let token = p.trabajar(otro, 0).unwrap().token;
        let antes = p.db.get_time_stats(a).unwrap().total_seconds;

        assert_eq!(p.recuperar(0), 81);

        assert_eq!(p.db.get_time_stats(a).unwrap().total_seconds, antes);
        assert_eq!(antes, 3600);
        assert_eq!(
            p.contar(
                "SELECT COUNT(*) FROM time_tracking_sessions
                 WHERE project_id IN (SELECT id FROM projects WHERE name = 'A')
                   AND motivo_cierre = 'huerfana' AND duration_seconds = 0"
            ),
            81
        );
        assert_eq!(
            p.sesion(&token).1,
            "viva_activa",
            "la sesión viva de Trabajar no se toca"
        );
    }

    /// Una huérfana de 13 h medidas se recorta a 12 h, con `tope_sin_tracker`.
    #[test]
    fn una_huerfana_de_13_h_se_recorta_a_43200() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        let id = p.sembrar_tramo(
            a,
            "work_button",
            "2026-10-19 19:00:00",
            Some("2026-10-20 08:00:00"),
            Some(46_800),
        );

        p.recuperar(0);

        assert_eq!(
            p.tramo(id),
            (
                Some("2026-10-20 08:00:00".into()),
                Some(43_200),
                Some("tope_sin_tracker".into())
            )
        );
    }

    /// La app se cortó midiendo: con el tracker posible, la huérfana cierra su tramo y
    /// el token pasa a `esperando_demonio`, como si se hubiera cerrado la ventana.
    #[test]
    fn una_huerfana_con_token_pasa_a_esperando_demonio() {
        let p = Prueba::con_tracker();
        let a = p.proyecto("A");
        let token = p.trabajar(a, 0).unwrap().token;
        let tramo = p.medicion().unwrap().tramo_id;
        p.latir(60);
        p.cortar_la_app();

        assert_eq!(p.recuperar(600), 1);

        assert_eq!(
            p.tramo(tramo),
            (
                Some("2026-10-20 09:01:00".into()),
                Some(60),
                Some("huerfana".into())
            )
        );
        assert_eq!(
            p.sesion(&token),
            ("demonio".into(), "esperando_demonio".into(), None)
        );
    }

    #[test]
    fn con_windows_simulado_una_huerfana_con_token_termina_con_huerfana() {
        let mut p = Prueba::con_tracker();
        p.entorno.linux = false;
        let a = p.proyecto("A");
        let token = p.trabajar(a, 0).unwrap().token;
        p.latir(60);
        p.cortar_la_app();

        p.recuperar(600);

        assert_eq!(
            p.sesion(&token),
            ("app".into(), "cerrada".into(), Some("huerfana".into()))
        );
    }

    /// Una huérfana sin token solo cierra su tramo; una fila `terminal` nunca se toca
    /// acá (la recupera el demonio).
    #[test]
    fn una_huerfana_sin_token_solo_cierra_el_tramo_y_una_terminal_no_se_toca() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        let sin_token = p.sembrar_tramo(
            a,
            "work_button",
            "2026-10-20 07:00:00",
            Some("2026-10-20 07:30:00"),
            Some(1800),
        );
        let terminal = p.sembrar_tramo(
            a,
            "terminal",
            "2026-10-20 07:00:00",
            Some("2026-10-20 07:30:00"),
            Some(1800),
        );

        assert_eq!(p.recuperar(0), 1);

        assert_eq!(p.tramo(sin_token).2.as_deref(), Some("huerfana"));
        assert_eq!(p.tramo(terminal), (None, Some(1800), None));
        assert_eq!(p.contar("SELECT COUNT(*) FROM trabajar_sesiones"), 0);
    }

    // ============ Aviso sin tracker (U6) ============

    #[test]
    fn trabajar_con_la_app_avisa_sin_tracker_solo_si_la_adopcion_es_posible() {
        let con = Prueba::con_tracker();
        let a = con.proyecto("A");
        assert!(con.trabajar(a, 0).unwrap().aviso_sin_tracker);

        let sin = Prueba::sin_tracker();
        let a = sin.proyecto("A");
        assert!(!sin.trabajar(a, 0).unwrap().aviso_sin_tracker);
    }

    /// Los cuatro casos de 3.10.14, con dos sondeos contra uno.
    #[test]
    fn aviso_cabecera_por_caso() {
        let esperado = EntradaAviso {
            adopcion_imposible: false,
            tracker_corrio: true,
            estado_tracker: Some("activo"),
            sondeos_sin_demonio: 2,
            hay_sesion_app: false,
        };
        let aviso = |entrada: EntradaAviso| aviso_cabecera(&entrada);
        let nada = AvisoCabecera {
            alarma: false,
            linea_trabajar: false,
        };

        // (a) Alarma: se espera el tracker, corrió y no responde en dos sondeos.
        assert!(aviso(esperado).alarma);
        assert_eq!(
            aviso(EntradaAviso {
                sondeos_sin_demonio: 1,
                ..esperado
            }),
            nada,
            "un solo sondeo (un restart) no alarma"
        );
        assert_eq!(
            aviso(EntradaAviso {
                sondeos_sin_demonio: 0,
                ..esperado
            }),
            nada
        );
        assert_eq!(
            aviso(EntradaAviso {
                estado_tracker: Some("esperando_migracion"),
                ..esperado
            }),
            nada
        );
        assert_eq!(
            aviso(EntradaAviso {
                tracker_corrio: false,
                estado_tracker: None,
                ..esperado
            }),
            nada,
            "sin fila en tracker_status: nunca corrió"
        );
        // (b) Línea de Trabajar con una sesión de la app, también junto a la alarma.
        assert_eq!(
            aviso(EntradaAviso {
                hay_sesion_app: true,
                ..esperado
            }),
            AvisoCabecera {
                alarma: true,
                linea_trabajar: true
            }
        );
        // (c)/(d) Adopción imposible (F1–F5, Windows, desarrollo): sin alarma, y la
        // línea solo con una sesión de la app.
        let imposible = EntradaAviso {
            adopcion_imposible: true,
            ..esperado
        };
        assert_eq!(aviso(imposible), nada);
        assert_eq!(
            aviso(EntradaAviso {
                hay_sesion_app: true,
                ..imposible
            }),
            AvisoCabecera {
                alarma: false,
                linea_trabajar: true
            }
        );
    }

    // ============ estado_trabajar (3.10.7) ============

    /// Antes `work_session_status_returns_the_real_path_not_the_project_name`,
    /// `work_session_status_is_empty_without_an_active_session` y
    /// `stopping_a_session_clears_the_project_path`: la tarjeta lee la base.
    #[test]
    fn estado_trabajar_lee_las_sesiones_vivas_de_la_base() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");

        let vacio = p.estado(0, None);
        assert!(vacio.sesiones.is_empty());
        assert!(!vacio.demonio_vivo);
        assert!(vacio.adopcion_imposible);
        assert!(!vacio.tracker_corrio);
        assert_eq!(
            vacio.aviso,
            AvisoCabecera {
                alarma: false,
                linea_trabajar: false
            }
        );

        let token = p.trabajar(a, 0).unwrap().token;
        let estado = p.estado(42, None);
        assert_eq!(estado.sesiones.len(), 1);
        let sesion = &estado.sesiones[0];
        assert_eq!(sesion.sesion.token, token);
        assert_eq!(sesion.sesion.project_id, a);
        assert_eq!(sesion.sesion.estado, "viva_activa");
        assert_eq!(sesion.sesion.dueno, "app");
        assert_eq!(sesion.sesion.origen, "boton");
        assert_eq!(
            sesion
                .sesion
                .tramo_abierto
                .as_ref()
                .map(|t| t.started_at.as_str()),
            Some("2026-10-20 09:00:00")
        );
        assert_eq!(sesion.elapsed_seconds, Some(42));
        assert!(estado.aviso.linea_trabajar, "la app mide sin vigilancia");

        p.parar(a, Some(&token), 60).unwrap();
        assert!(p.estado(60, None).sesiones.is_empty());
    }

    /// La alarma necesita fila en `tracker_status`, el tracker esperado y dos sondeos
    /// seguidos sin demonio: el sondeo devuelve la cuenta para el siguiente.
    #[test]
    fn estado_trabajar_cuenta_los_sondeos_sin_demonio() {
        let p = Prueba::con_tracker();
        p.db.execute_batch_para_tests(
            "INSERT INTO tracker_status (id, estado, pid, last_heartbeat)
             VALUES (1, 'activo', NULL, '2026-10-20 08:00:00');",
        );

        let primero = p.estado(0, None);
        assert!(primero.tracker_corrio);
        assert_eq!(primero.sondeos_sin_demonio, 1);
        assert!(!primero.aviso.alarma);

        let segundo = p.estado(5, Some(primero.sondeos_sin_demonio));
        assert_eq!(segundo.sondeos_sin_demonio, 2);
        assert!(segundo.aviso.alarma);
    }

    // ============ M8: regla única de duración mínima ============
    //
    // La duración se inyecta con el reloj: ningún test duerme.

    #[test]
    fn duracion_a_guardar_aplica_los_10_s_salvo_a_file_watch() {
        assert_eq!(duracion_a_guardar("work_button", 0), 0);
        assert_eq!(duracion_a_guardar("work_button", 5), 0);
        assert_eq!(duracion_a_guardar("work_button", 10), 0);
        assert_eq!(duracion_a_guardar("work_button", 11), 11);
        // Fuentes viejas: misma regla que "Trabajar".
        assert_eq!(duracion_a_guardar("shell_hook", 7), 0);
        assert_eq!(duracion_a_guardar("shell_hook", 3600), 3600);
        // `file_watch` no tiene regla de 10 s (la cubre el margen por guardado).
        assert_eq!(duracion_a_guardar("file_watch", 5), 5);
        // `terminal` tampoco (DT5): el tramo de 0 s se borra al cerrarlo.
        assert_eq!(duracion_a_guardar("terminal", 5), 5);
        assert_eq!(duracion_a_guardar("terminal", 0), 0);
    }

    #[test]
    fn parar_a_los_5_s_guarda_0_con_ended_at_real() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        p.trabajar(a, 0).unwrap();
        let tramo = p.medicion().unwrap().tramo_id;

        assert_eq!(
            p.parar(a, None, 5).unwrap(),
            ResultadoParar::Cerrada { segundos: 0 }
        );
        assert_eq!(
            p.tramo(tramo),
            (
                Some("2026-10-20 09:00:05".into()),
                Some(0),
                Some("parar_app".into())
            ),
            "la fila se cierra con su ended_at"
        );
        assert_eq!(p.db.get_time_stats(a).unwrap().total_seconds, 0);
    }

    /// Con el tracker posible, A además queda esperando al demonio (§5.6).
    #[test]
    fn cambio_de_proyecto_a_los_5_s_guarda_0() {
        let p = Prueba::con_tracker();
        let a = p.proyecto("A");
        let b = p.proyecto("B");
        let token_a = p.trabajar(a, 0).unwrap().token;
        let tramo_a = p.medicion().unwrap().tramo_id;

        let en_b = p.trabajar(b, 5).unwrap();

        assert!(
            !en_b.sesion_anterior,
            "una sesión de 0 s no cuenta como guardada"
        );
        assert_eq!(p.tramo(tramo_a).1, Some(0));
        assert!(p.tramo(tramo_a).0.is_some());
        assert_eq!(p.sesion(&token_a).1, "esperando_demonio");
    }

    #[test]
    fn once_segundos_se_guardan_tal_cual_al_cambiar_de_proyecto() {
        let p = Prueba::con_tracker();
        let a = p.proyecto("A");
        let b = p.proyecto("B");
        let token_a = p.trabajar(a, 0).unwrap().token;
        let tramo_a = p.medicion().unwrap().tramo_id;

        assert!(p.trabajar(b, 11).unwrap().sesion_anterior);

        assert_eq!(p.tramo(tramo_a).1, Some(11));
        assert_eq!(p.sesion(&token_a).1, "esperando_demonio");
    }

    #[test]
    fn once_segundos_se_guardan_tal_cual_al_parar() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        p.trabajar(a, 0).unwrap();
        let tramo = p.medicion().unwrap().tramo_id;

        assert_eq!(
            p.parar(a, None, 11).unwrap(),
            ResultadoParar::Cerrada { segundos: 11 }
        );
        assert_eq!(p.tramo(tramo).1, Some(11));
    }

    /// El cierre de la ventana va por `al_cerrar_ventana`, no por Parar: se prueba
    /// aparte. Con el tracker posible, el token queda esperando (§5.6).
    #[test]
    fn once_segundos_se_guardan_tal_cual_al_cerrar_la_ventana() {
        let p = Prueba::con_tracker();
        let a = p.proyecto("A");
        let token = p.trabajar(a, 0).unwrap().token;

        assert_eq!(p.cerrar_ventana(11).unwrap(), Some(11));

        assert_eq!(p.db.get_time_stats(a).unwrap().total_seconds, 11);
        assert_eq!(p.sesion(&token).1, "esperando_demonio");
    }

    #[test]
    fn un_error_de_base_al_cerrar_con_0_s_en_el_cambio_de_proyecto_sube() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        let b = p.proyecto("B");
        p.trabajar(a, 0).unwrap();
        let medicion = p.medicion();
        p.romper_cierres();

        let err = p
            .trabajar(b, 5)
            .expect_err("el error del cierre de 0 s no debe descartarse");

        assert!(err.contains("cierre roto"), "mensaje: {err}");
        // La app sigue midiendo A y no se creó nada para B.
        assert_eq!(p.medicion(), medicion);
        assert_eq!(
            p.contar(&format!(
                "SELECT COUNT(*) FROM trabajar_sesiones WHERE project_id = {b}"
            )),
            0
        );
        assert!(p.db.get_tracking_sessions(b, 10).unwrap().is_empty());
    }

    #[test]
    fn un_error_de_base_al_parar_con_0_s_sube_y_la_sesion_sigue_activa() {
        let p = Prueba::sin_tracker();
        let a = p.proyecto("A");
        let token = p.trabajar(a, 0).unwrap().token;
        let medicion = p.medicion();
        p.romper_cierres();

        let err = p
            .parar(a, Some(&token), 5)
            .expect_err("el error del cierre de 0 s no debe descartarse");

        assert!(err.contains("cierre roto"), "mensaje: {err}");
        assert_eq!(p.medicion(), medicion);
        assert_eq!(p.sesion(&token).1, "viva_activa");
    }
}
