// Tipos de datos de las sesiones de tiempo.
//
// Son el contrato compartido entre `Database` (que lee y escribe la tabla
// `time_tracking_sessions`) y los comandos Tauri que los devuelven al frontend.
//
// Acá vivía `TimeAggregator`, una fachada sobre esos mismos métodos de
// `Database` que nadie construía en ningún lado. Se eliminó en el B17.

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
            source: "work_button".to_string(),
        };

        assert_eq!(session.project_id, 123);
        assert_eq!(session.duration_seconds, Some(3600));
    }
}
