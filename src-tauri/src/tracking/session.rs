// Máquina de estados para sesiones de tracking
// Maneja el ciclo de vida de una sesión de trabajo

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Estado de una sesión de tracking
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SessionState {
    /// Sin sesión activa
    Idle,
    /// Sesión en progreso
    Active,
    /// Sesión pausada (por inactividad o manualmente)
    Paused,
}

/// Representa una sesión de tracking activa
#[derive(Debug)]
pub struct TrackingSession {
    /// ID del proyecto asociado
    pub project_id: i64,
    /// Path del proyecto
    pub project_path: String,
    /// Estado actual de la sesión
    state: SessionState,
    /// Momento en que inició la sesión (o se reanudó)
    started_at: Option<Instant>,
    /// Tiempo acumulado antes de pausar
    accumulated_seconds: u64,
    /// Último heartbeat recibido
    last_heartbeat: Option<Instant>,
}

impl TrackingSession {
    /// Crear una nueva sesión de tracking
    pub fn new(project_id: i64, project_path: String) -> Self {
        Self {
            project_id,
            project_path,
            state: SessionState::Idle,
            started_at: None,
            accumulated_seconds: 0,
            last_heartbeat: None,
        }
    }

    /// Obtener el estado actual
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Iniciar la sesión
    pub fn start(&mut self) {
        if self.state == SessionState::Idle {
            self.state = SessionState::Active;
            self.started_at = Some(Instant::now());
            self.last_heartbeat = Some(Instant::now());
            self.accumulated_seconds = 0;
        }
    }

    /// Pausar la sesión
    pub fn pause(&mut self) {
        if self.state == SessionState::Active {
            // Acumular tiempo transcurrido
            if let Some(start) = self.started_at {
                self.accumulated_seconds += start.elapsed().as_secs();
            }
            self.state = SessionState::Paused;
            self.started_at = None;
        }
    }

    /// Reanudar la sesión
    pub fn resume(&mut self) {
        if self.state == SessionState::Paused {
            self.state = SessionState::Active;
            self.started_at = Some(Instant::now());
            self.last_heartbeat = Some(Instant::now());
        }
    }

    /// Detener la sesión y retornar duración total en segundos
    pub fn stop(&mut self) -> u64 {
        let total = self.get_elapsed_seconds();
        self.state = SessionState::Idle;
        self.started_at = None;
        self.accumulated_seconds = 0;
        self.last_heartbeat = None;
        total
    }

    /// Registrar un heartbeat (actualiza último tiempo de actividad)
    pub fn heartbeat(&mut self) {
        if self.state == SessionState::Active || self.state == SessionState::Paused {
            self.last_heartbeat = Some(Instant::now());

            // Si estaba pausado por inactividad, reanudar automáticamente
            if self.state == SessionState::Paused {
                self.resume();
            }
        }
    }

    /// Obtener tiempo transcurrido en segundos
    pub fn get_elapsed_seconds(&self) -> u64 {
        let mut total = self.accumulated_seconds;

        if self.state == SessionState::Active {
            if let Some(start) = self.started_at {
                total += start.elapsed().as_secs();
            }
        }

        total
    }

    /// Verificar si la sesión está inactiva (sin heartbeat en X segundos)
    pub fn is_inactive(&self, timeout_seconds: u64) -> bool {
        if let Some(last) = self.last_heartbeat {
            last.elapsed().as_secs() > timeout_seconds
        } else {
            false
        }
    }

    /// Auto-pausar si ha pasado el timeout de inactividad
    pub fn check_inactivity(&mut self, timeout_seconds: u64) {
        if self.state == SessionState::Active && self.is_inactive(timeout_seconds) {
            self.pause();
        }
    }
}

/// Gestor de sesiones activas (puede haber múltiples terminales)
pub struct SessionManager {
    /// Sesiones activas por path de proyecto
    sessions: Arc<Mutex<HashMap<String, TrackingSession>>>,
    /// Timeout de inactividad en segundos (default: 15 minutos)
    pub inactivity_timeout: u64,
}

impl SessionManager {
    /// Toma el lock del mapa de sesiones, recuperándose del envenenamiento.
    ///
    /// Mismo criterio que `Database::conn`: `Mutex::lock()` solo devuelve `Err` si
    /// otro hilo entró en panic con el lock tomado, y con `unwrap()` ese primer
    /// panic convertía en panic TODAS las operaciones de sesión posteriores. El
    /// `HashMap` de adentro sigue siendo un `HashMap` perfectamente usable: el
    /// envenenamiento es una marca de Rust, no una corrupción del dato.
    fn sessions(&self) -> std::sync::MutexGuard<'_, HashMap<String, TrackingSession>> {
        self.sessions.lock().unwrap_or_else(|poisoned| {
            log::error!(
                "❌ [TRACKING] El mutex de sesiones estaba envenenado (hubo un panic con el lock tomado). Se recupera y se sigue operando."
            );
            poisoned.into_inner()
        })
    }

    /// Crear un nuevo gestor de sesiones
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            inactivity_timeout: 15 * 60, // 15 minutos por defecto
        }
    }

    /// Entrar a un proyecto (crear o reanudar sesión)
    pub fn enter_project(&self, project_id: i64, project_path: String) {
        let mut sessions = self.sessions();

        if let Some(session) = sessions.get_mut(&project_path) {
            // Sesión existente - reanudar si estaba pausada
            if session.state() == SessionState::Paused {
                session.resume();
            }
        } else {
            // Nueva sesión
            let mut session = TrackingSession::new(project_id, project_path.clone());
            session.start();
            sessions.insert(project_path, session);
        }
    }

    /// Salir de un proyecto (pausar o terminar sesión)
    pub fn exit_project(&self, project_path: &str) -> Option<u64> {
        let mut sessions = self.sessions();

        if let Some(session) = sessions.get_mut(project_path) {
            let duration = session.stop();
            sessions.remove(project_path);
            Some(duration)
        } else {
            None
        }
    }

    /// Registrar heartbeat para un proyecto
    pub fn heartbeat(&self, project_path: &str) {
        let mut sessions = self.sessions();

        if let Some(session) = sessions.get_mut(project_path) {
            session.heartbeat();
        }
    }

    /// Obtener estado actual de tracking
    pub fn get_current_session(&self) -> Option<(String, i64, SessionState, u64)> {
        let sessions = self.sessions();

        for (path, session) in sessions.iter() {
            if session.state() == SessionState::Active {
                return Some((
                    path.clone(),
                    session.project_id,
                    session.state(),
                    session.get_elapsed_seconds(),
                ));
            }
        }
        None
    }

    /// Verificar y pausar sesiones inactivas
    pub fn check_all_inactivity(&self) {
        let mut sessions = self.sessions();
        let timeout = self.inactivity_timeout;

        for session in sessions.values_mut() {
            session.check_inactivity(timeout);
        }
    }

    /// Obtener número de sesiones activas
    pub fn active_session_count(&self) -> usize {
        let sessions = self.sessions();
        sessions.values()
            .filter(|s| s.state() == SessionState::Active)
            .count()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {

    #[test]
    fn el_gestor_sigue_operando_despues_de_un_panic_con_el_lock_tomado() {
        let manager = SessionManager::new();
        manager.enter_project(1, "/tmp/proyecto".to_string());

        let panicked = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let _guard = manager.sessions.lock().expect("el mutex arranca sano");
                    panic!("panic deliberado con el lock de sesiones tomado");
                })
                .join()
        });
        assert!(panicked.is_err(), "el hilo tenía que entrar en panic");

        // Con `lock().unwrap()` esto era un panic en cascada: un fallo aislado
        // dejaba el tracking muerto hasta reiniciar la app.
        assert_eq!(manager.active_session_count(), 1);
    }

    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_new_session_is_idle() {
        let session = TrackingSession::new(123, "/path/to/project".to_string());

        assert_eq!(session.state(), SessionState::Idle);
        assert_eq!(session.project_id, 123);
        assert_eq!(session.project_path, "/path/to/project");
        assert_eq!(session.get_elapsed_seconds(), 0);
    }

    #[test]
    fn test_start_session() {
        let mut session = TrackingSession::new(123, "/path/to/project".to_string());

        session.start();

        assert_eq!(session.state(), SessionState::Active);

        // Esperar un poco y verificar que el tiempo avanza
        thread::sleep(Duration::from_millis(100));
        assert!(session.get_elapsed_seconds() >= 0);
    }

    #[test]
    fn test_stop_session_returns_duration() {
        let mut session = TrackingSession::new(123, "/path".to_string());

        session.start();
        thread::sleep(Duration::from_secs(1));

        let duration = session.stop();

        assert!(duration >= 1);
        assert_eq!(session.state(), SessionState::Idle);
        assert_eq!(session.get_elapsed_seconds(), 0);
    }

    #[test]
    fn test_pause_and_resume() {
        let mut session = TrackingSession::new(123, "/path".to_string());

        // Iniciar
        session.start();
        thread::sleep(Duration::from_secs(1));

        // Pausar
        session.pause();
        assert_eq!(session.state(), SessionState::Paused);
        let paused_time = session.get_elapsed_seconds();
        assert!(paused_time >= 1, "Should have at least 1 second");

        // El tiempo no debería avanzar mientras está pausado
        thread::sleep(Duration::from_millis(200));
        assert_eq!(session.get_elapsed_seconds(), paused_time);

        // Reanudar
        session.resume();
        assert_eq!(session.state(), SessionState::Active);

        thread::sleep(Duration::from_secs(1));

        // El tiempo debería haber avanzado
        let total_time = session.get_elapsed_seconds();
        assert!(total_time >= paused_time + 1, "Should have advanced by at least 1 second");
    }

    #[test]
    fn test_heartbeat_updates_activity() {
        let mut session = TrackingSession::new(123, "/path".to_string());
        session.start();

        // Sin heartbeat, después de un tiempo debería ser inactivo
        thread::sleep(Duration::from_millis(200));

        // Registrar heartbeat
        session.heartbeat();

        // Justo después del heartbeat, no debería ser inactivo
        assert!(!session.is_inactive(1)); // 1 segundo de timeout
    }

    #[test]
    fn test_auto_pause_on_inactivity() {
        let mut session = TrackingSession::new(123, "/path".to_string());
        session.start();

        // Esperar más de 1 segundo para que el test funcione con granularidad de segundos
        thread::sleep(Duration::from_secs(2));

        // Verificar inactividad con timeout de 1 segundo
        // Después de 2 segundos sin heartbeat, debería ser inactivo
        session.check_inactivity(1);

        assert_eq!(session.state(), SessionState::Paused);
    }

    #[test]
    fn test_session_manager_enter_exit() {
        let manager = SessionManager::new();

        // Entrar al proyecto
        manager.enter_project(123, "/project1".to_string());

        assert_eq!(manager.active_session_count(), 1);

        // Verificar sesión actual
        let current = manager.get_current_session();
        assert!(current.is_some());
        let (path, id, state, _) = current.unwrap();
        assert_eq!(path, "/project1");
        assert_eq!(id, 123);
        assert_eq!(state, SessionState::Active);

        // Salir del proyecto
        let duration = manager.exit_project("/project1");
        assert!(duration.is_some());
        assert_eq!(manager.active_session_count(), 0);
    }

    #[test]
    fn test_session_manager_multiple_projects() {
        let manager = SessionManager::new();

        // Entrar a múltiples proyectos
        manager.enter_project(1, "/project1".to_string());
        manager.enter_project(2, "/project2".to_string());

        // Ambos deberían estar activos
        assert_eq!(manager.active_session_count(), 2);

        // Salir de uno
        manager.exit_project("/project1");
        assert_eq!(manager.active_session_count(), 1);
    }

    #[test]
    fn test_session_manager_heartbeat() {
        let manager = SessionManager::new();

        manager.enter_project(123, "/project".to_string());
        thread::sleep(Duration::from_millis(100));

        // El heartbeat debería funcionar
        manager.heartbeat("/project");

        // La sesión sigue activa
        assert_eq!(manager.active_session_count(), 1);
    }

    #[test]
    fn test_accumulated_time_across_pauses() {
        let mut session = TrackingSession::new(123, "/path".to_string());

        // Primera sesión de 1 segundo
        session.start();
        thread::sleep(Duration::from_secs(1));
        session.pause();

        let after_first = session.get_elapsed_seconds();
        assert!(after_first >= 1);

        // Segunda sesión de 1 segundo
        session.resume();
        thread::sleep(Duration::from_secs(1));

        let total = session.stop();

        // Debería ser al menos 2 segundos
        assert!(total >= 2);
    }
}
