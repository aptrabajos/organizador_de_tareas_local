// Módulo de Time Tracking para Gestor de Proyectos
// Implementa tracking automático de tiempo por proyecto usando shell hooks

pub mod config;
pub mod session;
pub mod socket;
pub mod aggregator;

// Re-exports públicos
pub use config::{GestorConfig, find_gestor_config, init_gestor_config};
pub use session::{TrackingSession, SessionState, SessionManager};
pub use socket::{TrackingMessage, SocketServer};
pub use aggregator::TimeAggregator;
