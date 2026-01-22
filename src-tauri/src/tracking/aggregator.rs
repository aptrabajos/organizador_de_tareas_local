// Agregador de tiempo - persiste sesiones de tracking en la BD
// Conecta el SessionManager con la base de datos

use crate::db::Database;
use std::sync::Arc;

/// Modelo de sesión de tracking para persistencia
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeTrackingSession {
    pub id: Option<i64>,
    pub project_id: i64,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_seconds: Option<i64>,
    pub source: String,
}

/// Estadísticas de tiempo para un proyecto
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeStats {
    /// Tiempo total en segundos
    pub total_seconds: i64,
    /// Número de sesiones
    pub session_count: i64,
    /// Duración promedio de sesión en segundos
    pub avg_session_seconds: i64,
    /// Sesión más larga en segundos
    pub longest_session_seconds: i64,
    /// Tiempo hoy en segundos
    pub today_seconds: i64,
    /// Tiempo esta semana en segundos
    pub week_seconds: i64,
}

impl Default for TimeStats {
    fn default() -> Self {
        Self {
            total_seconds: 0,
            session_count: 0,
            avg_session_seconds: 0,
            longest_session_seconds: 0,
            today_seconds: 0,
            week_seconds: 0,
        }
    }
}

/// Agregador de tiempo - conecta tracking con persistencia
pub struct TimeAggregator {
    db: Arc<Database>,
}

impl TimeAggregator {
    /// Crear un nuevo agregador
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Iniciar una nueva sesión de tracking (crear registro en BD)
    pub fn start_session(&self, project_id: i64, source: &str) -> Result<i64, AggregatorError> {
        self.db.create_tracking_session(project_id, source)
            .map_err(|e| AggregatorError::DatabaseError(e.to_string()))
    }

    /// Terminar una sesión de tracking
    pub fn end_session(&self, session_id: i64, duration_seconds: i64) -> Result<(), AggregatorError> {
        self.db.end_tracking_session(session_id, duration_seconds)
            .map_err(|e| AggregatorError::DatabaseError(e.to_string()))
    }

    /// Obtener sesiones de un proyecto
    pub fn get_sessions(&self, project_id: i64, limit: i64) -> Result<Vec<TimeTrackingSession>, AggregatorError> {
        self.db.get_tracking_sessions(project_id, limit)
            .map_err(|e| AggregatorError::DatabaseError(e.to_string()))
    }

    /// Obtener estadísticas de tiempo para un proyecto
    pub fn get_stats(&self, project_id: i64) -> Result<TimeStats, AggregatorError> {
        self.db.get_time_stats(project_id)
            .map_err(|e| AggregatorError::DatabaseError(e.to_string()))
    }

    /// Agregar tiempo a un proyecto (método simple sin sesión)
    pub fn add_time(&self, project_id: i64, seconds: i64) -> Result<(), AggregatorError> {
        // Crear sesión completa de una vez
        let session_id = self.start_session(project_id, "manual")?;
        self.end_session(session_id, seconds)?;
        Ok(())
    }
}

/// Errores del agregador
#[derive(Debug, Clone)]
pub enum AggregatorError {
    DatabaseError(String),
    SessionNotFound(i64),
}

impl std::fmt::Display for AggregatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AggregatorError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            AggregatorError::SessionNotFound(id) => write!(f, "Session not found: {}", id),
        }
    }
}

impl std::error::Error for AggregatorError {}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_stats_default() {
        let stats = TimeStats::default();

        assert_eq!(stats.total_seconds, 0);
        assert_eq!(stats.session_count, 0);
        assert_eq!(stats.avg_session_seconds, 0);
        assert_eq!(stats.longest_session_seconds, 0);
        assert_eq!(stats.today_seconds, 0);
        assert_eq!(stats.week_seconds, 0);
    }

    #[test]
    fn test_time_stats_serialization() {
        let stats = TimeStats {
            total_seconds: 3600,
            session_count: 5,
            avg_session_seconds: 720,
            longest_session_seconds: 1800,
            today_seconds: 600,
            week_seconds: 2400,
        };

        let json = serde_json::to_string(&stats).expect("Should serialize");

        assert!(json.contains("\"total_seconds\":3600"));
        assert!(json.contains("\"session_count\":5"));
    }

    #[test]
    fn test_tracking_session_creation() {
        let session = TimeTrackingSession {
            id: Some(1),
            project_id: 123,
            started_at: "2025-01-22T10:00:00Z".to_string(),
            ended_at: Some("2025-01-22T11:00:00Z".to_string()),
            duration_seconds: Some(3600),
            source: "shell_hook".to_string(),
        };

        assert_eq!(session.project_id, 123);
        assert_eq!(session.duration_seconds, Some(3600));
    }

    #[test]
    fn test_aggregator_error_display() {
        let db_error = AggregatorError::DatabaseError("Connection failed".to_string());
        assert_eq!(format!("{}", db_error), "Database error: Connection failed");

        let not_found = AggregatorError::SessionNotFound(42);
        assert_eq!(format!("{}", not_found), "Session not found: 42");
    }
}
