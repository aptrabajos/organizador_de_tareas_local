//! Inicialización y control del logging de la aplicación.
//!
//! Antes de esto el backend no tenía logger: 138 `println!`/`eprintln!` escribían
//! directo a stdout/stderr, sin niveles y sin filtro en release. El campo
//! `advanced.log_level` existía en el schema, se persistía y tenía una perilla en
//! Settings, pero NO tenía ningún consumidor: no controlaba absolutamente nada.
//!
//! El truco central está en cómo se combinan las dos capas de filtrado del crate
//! `log`:
//!
//! 1. `log::max_level()` — la puerta global. Los macros (`debug!`, `info!`, ...)
//!    la consultan ANTES de formatear los argumentos, así que un nivel apagado no
//!    cuesta ni el formateo.
//! 2. El filtro propio del backend (acá, `env_logger`).
//!
//! `env_logger` se construye deliberadamente con `LevelFilter::Trace`, o sea con
//! su capa ABIERTA del todo, y el nivel efectivo se gobierna exclusivamente con
//! `log::set_max_level`. Esa asimetría es a propósito: `set_max_level` se puede
//! mover en runtime en las DOS direcciones, mientras que el filtro con el que se
//! construyó el backend queda fijo para siempre. Si se hiciera al revés —
//! construir `env_logger` con el nivel de la config — bajar el nivel funcionaría,
//! pero SUBIRLO no: la capa de `env_logger` seguiría descartando los records, y
//! la perilla quedaría a medio funcionar.

use crate::config::schema::LogLevel;
use log::LevelFilter;

/// Instala el logger del proceso.
///
/// Arranca en `Info` a propósito, no en el nivel configurado: esto corre ANTES de
/// que la configuración esté leída (leerla ya loguea), y quedarse ciego durante el
/// arranque es peor que mostrar una o dos líneas de más a quien puso `error`. El
/// nivel real se aplica con [`apply_level`] apenas la config está disponible.
///
/// Es idempotente: `try_init` devuelve `Err` si ya hay un logger instalado —el
/// caso normal en los tests— y ahí no hay nada que hacer.
pub fn init() {
    let _ = env_logger::Builder::new()
        .filter_level(LevelFilter::Trace)
        .format_timestamp_secs()
        .try_init();
    apply_level(LogLevel::Info);
}

/// Aplica el nivel de `advanced.log_level`.
///
/// Se llama al arrancar, con la config ya leída, y de nuevo en cada
/// `update_config`: por eso cambiar la perilla en Settings tiene efecto en el
/// acto, sin reiniciar la app.
pub fn apply_level(level: LogLevel) {
    log::set_max_level(level.to_level_filter());
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{Level, Log, Metadata, Record};
    use std::sync::{Mutex, Once, OnceLock};

    /// Logger de captura: se queda con el nivel y el mensaje de cada record que
    /// LOGRA pasar la puerta global. Es lo que permite testear el filtrado de
    /// verdad, en vez de testear que `to_level_filter` devuelve lo que devuelve.
    struct CapturingLogger;

    static CAPTURED: OnceLock<Mutex<Vec<(Level, String)>>> = OnceLock::new();
    static INSTALL: Once = Once::new();
    /// `log::max_level()` es estado global del proceso y los tests de Rust corren
    /// en paralelo: sin este mutex, un test le pisaría el nivel a otro.
    static SERIALIZE: Mutex<()> = Mutex::new(());

    fn captured() -> &'static Mutex<Vec<(Level, String)>> {
        CAPTURED.get_or_init(|| Mutex::new(Vec::new()))
    }

    impl Log for CapturingLogger {
        fn enabled(&self, _: &Metadata) -> bool {
            true
        }
        fn log(&self, record: &Record) {
            captured()
                .lock()
                .unwrap()
                .push((record.level(), record.args().to_string()));
        }
        fn flush(&self) {}
    }

    /// Instala el logger de captura una sola vez y limpia lo acumulado.
    fn setup() {
        INSTALL.call_once(|| {
            log::set_logger(&CapturingLogger).expect("no habia logger instalado");
        });
        captured().lock().unwrap().clear();
    }

    /// Emite un record de cada nivel y devuelve los que sobrevivieron al filtro.
    fn emit_all_levels() -> Vec<Level> {
        log::error!("error");
        log::warn!("warn");
        log::info!("info");
        log::debug!("debug");
        captured()
            .lock()
            .unwrap()
            .iter()
            .map(|(level, _)| *level)
            .collect()
    }

    #[test]
    fn el_nivel_configurado_se_respeta_de_verdad() {
        let _guard = SERIALIZE.lock().unwrap_or_else(|e| e.into_inner());
        setup();

        // Debug deja pasar los cuatro.
        apply_level(LogLevel::Debug);
        assert_eq!(
            emit_all_levels(),
            vec![Level::Error, Level::Warn, Level::Info, Level::Debug]
        );

        // Info descarta debug.
        captured().lock().unwrap().clear();
        apply_level(LogLevel::Info);
        assert_eq!(
            emit_all_levels(),
            vec![Level::Error, Level::Warn, Level::Info]
        );

        // Warn descarta info y debug.
        captured().lock().unwrap().clear();
        apply_level(LogLevel::Warn);
        assert_eq!(emit_all_levels(), vec![Level::Error, Level::Warn]);

        // Error, el nivel más restrictivo, deja pasar SOLO los errores. Este es el
        // caso que demuestra que la perilla sirve: el ruido de `debug!` de los
        // caminos calientes (get_project, update_project) desaparece.
        captured().lock().unwrap().clear();
        apply_level(LogLevel::Error);
        assert_eq!(emit_all_levels(), vec![Level::Error]);
    }

    #[test]
    fn el_nivel_se_puede_subir_y_no_solo_bajar() {
        let _guard = SERIALIZE.lock().unwrap_or_else(|e| e.into_inner());
        setup();

        // Esta es la regresión que importa: si el backend se construyera con el
        // nivel de la config en vez de con Trace, pasar de Error a Debug NO
        // recuperaría los records, y cambiar la perilla hacia arriba en Settings
        // no haría nada.
        apply_level(LogLevel::Error);
        assert_eq!(emit_all_levels(), vec![Level::Error]);

        captured().lock().unwrap().clear();
        apply_level(LogLevel::Debug);
        assert_eq!(
            emit_all_levels(),
            vec![Level::Error, Level::Warn, Level::Info, Level::Debug]
        );
    }

    #[test]
    fn el_mapeo_de_niveles_no_se_corre_de_lugar() {
        assert_eq!(LogLevel::Error.to_level_filter(), LevelFilter::Error);
        assert_eq!(LogLevel::Warn.to_level_filter(), LevelFilter::Warn);
        assert_eq!(LogLevel::Info.to_level_filter(), LevelFilter::Info);
        assert_eq!(LogLevel::Debug.to_level_filter(), LevelFilter::Debug);
    }
}
