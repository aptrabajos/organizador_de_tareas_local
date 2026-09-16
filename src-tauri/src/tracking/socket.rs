// Servidor Unix Socket para recibir eventos de shell hooks
// Escucha en /tmp/gestor-proyectos.sock

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(unix)]
use std::io::{BufRead, BufReader, Write};

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread::{self, JoinHandle};

/// Mensajes que el shell hook puede enviar
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum TrackingMessage {
    /// Entrando a un directorio de proyecto
    Enter { path: String },
    /// Saliendo de un directorio de proyecto
    Exit { path: String },
    /// Heartbeat para indicar actividad
    Heartbeat { path: String },
    /// Solicitar estado actual
    Status,
}

/// Respuestas del servidor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingResponse {
    pub success: bool,
    pub message: Option<String>,
    pub project_id: Option<i64>,
    pub elapsed_seconds: Option<u64>,
}

impl TrackingResponse {
    pub fn ok() -> Self {
        Self {
            success: true,
            message: None,
            project_id: None,
            elapsed_seconds: None,
        }
    }

    pub fn ok_with_message(msg: &str) -> Self {
        Self {
            success: true,
            message: Some(msg.to_string()),
            project_id: None,
            elapsed_seconds: None,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            success: false,
            message: Some(msg.to_string()),
            project_id: None,
            elapsed_seconds: None,
        }
    }

    pub fn with_status(project_id: i64, elapsed: u64) -> Self {
        Self {
            success: true,
            message: None,
            project_id: Some(project_id),
            elapsed_seconds: Some(elapsed),
        }
    }
}

/// Path por defecto del socket
pub fn default_socket_path() -> PathBuf {
    PathBuf::from("/tmp/gestor-proyectos.sock")
}

/// Servidor de socket para recibir eventos de tracking
pub struct SocketServer {
    socket_path: PathBuf,
    running: Arc<AtomicBool>,
    #[cfg(unix)]
    handle: Option<JoinHandle<()>>,
}

impl SocketServer {
    /// Crear un nuevo servidor de socket
    pub fn new() -> Self {
        Self {
            socket_path: default_socket_path(),
            running: Arc::new(AtomicBool::new(false)),
            #[cfg(unix)]
            handle: None,
        }
    }

    /// Crear servidor con path personalizado (útil para tests)
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            socket_path: path,
            running: Arc::new(AtomicBool::new(false)),
            #[cfg(unix)]
            handle: None,
        }
    }

    /// Obtener el path del socket
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }

    /// Verificar si el servidor está corriendo
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Iniciar el servidor (solo Unix)
    #[cfg(unix)]
    pub fn start<F>(&mut self, handler: F) -> Result<(), SocketError>
    where
        F: Fn(TrackingMessage) -> TrackingResponse + Send + Sync + 'static,
    {
        if self.is_running() {
            return Err(SocketError::AlreadyRunning);
        }

        // Eliminar socket anterior si existe
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)
                .map_err(|e| SocketError::IoError(e.to_string()))?;
        }

        let listener = UnixListener::bind(&self.socket_path)
            .map_err(|e| SocketError::BindError(e.to_string()))?;

        // Configurar timeout para poder detener el servidor
        listener.set_nonblocking(true)
            .map_err(|e| SocketError::IoError(e.to_string()))?;

        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();
        let handler = Arc::new(handler);

        let handle = thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let handler = handler.clone();
                        thread::spawn(move || {
                            if let Err(e) = handle_client(stream, &*handler) {
                                // Ruta completa en vez de `use log::warn`: este bloque
                                // está detrás de `cfg(unix)` y un import a nivel de
                                // archivo quedaría sin usar en el resto de plataformas.
                                log::warn!("Error atendiendo a un cliente del socket: {}", e);
                            }
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No hay conexiones pendientes, esperar un poco
                        thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(e) => {
                        log::warn!("Error aceptando una conexión en el socket: {}", e);
                    }
                }
            }
        });

        self.handle = Some(handle);
        Ok(())
    }

    /// Iniciar el servidor (no-Unix - stub)
    #[cfg(not(unix))]
    pub fn start<F>(&mut self, _handler: F) -> Result<(), SocketError>
    where
        F: Fn(TrackingMessage) -> TrackingResponse + Send + Sync + 'static,
    {
        Err(SocketError::NotSupported)
    }

    /// Detener el servidor
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);

        #[cfg(unix)]
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }

        // Limpiar socket
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }
}

impl Default for SocketServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SocketServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Manejar conexión de cliente
#[cfg(unix)]
fn handle_client<F>(mut stream: UnixStream, handler: &F) -> Result<(), SocketError>
where
    F: Fn(TrackingMessage) -> TrackingResponse,
{
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| SocketError::IoError(e.to_string()))?);
    let mut line = String::new();

    reader.read_line(&mut line)
        .map_err(|e| SocketError::IoError(e.to_string()))?;

    let message: TrackingMessage = serde_json::from_str(line.trim())
        .map_err(|e| SocketError::ParseError(e.to_string()))?;

    let response = handler(message);

    let response_json = serde_json::to_string(&response)
        .map_err(|e| SocketError::SerializeError(e.to_string()))?;

    writeln!(stream, "{}", response_json)
        .map_err(|e| SocketError::IoError(e.to_string()))?;

    Ok(())
}

/// Errores del servidor de socket
#[derive(Debug, Clone, PartialEq)]
pub enum SocketError {
    AlreadyRunning,
    BindError(String),
    IoError(String),
    ParseError(String),
    SerializeError(String),
    NotSupported,
}

impl std::fmt::Display for SocketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SocketError::AlreadyRunning => write!(f, "Server already running"),
            SocketError::BindError(msg) => write!(f, "Bind error: {}", msg),
            SocketError::IoError(msg) => write!(f, "IO error: {}", msg),
            SocketError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            SocketError::SerializeError(msg) => write!(f, "Serialize error: {}", msg),
            SocketError::NotSupported => write!(f, "Unix sockets not supported on this platform"),
        }
    }
}

impl std::error::Error for SocketError {}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_enter_message() {
        let json = r#"{"type":"Enter","path":"/home/user/project"}"#;
        let msg: TrackingMessage = serde_json::from_str(json).expect("Should parse");

        match msg {
            TrackingMessage::Enter { path } => {
                assert_eq!(path, "/home/user/project");
            }
            _ => panic!("Expected Enter message"),
        }
    }

    #[test]
    fn test_parse_exit_message() {
        let json = r#"{"type":"Exit","path":"/home/user/project"}"#;
        let msg: TrackingMessage = serde_json::from_str(json).expect("Should parse");

        match msg {
            TrackingMessage::Exit { path } => {
                assert_eq!(path, "/home/user/project");
            }
            _ => panic!("Expected Exit message"),
        }
    }

    #[test]
    fn test_parse_heartbeat_message() {
        let json = r#"{"type":"Heartbeat","path":"/home/user/project"}"#;
        let msg: TrackingMessage = serde_json::from_str(json).expect("Should parse");

        match msg {
            TrackingMessage::Heartbeat { path } => {
                assert_eq!(path, "/home/user/project");
            }
            _ => panic!("Expected Heartbeat message"),
        }
    }

    #[test]
    fn test_parse_status_message() {
        let json = r#"{"type":"Status"}"#;
        let msg: TrackingMessage = serde_json::from_str(json).expect("Should parse");

        assert_eq!(msg, TrackingMessage::Status);
    }

    #[test]
    fn test_socket_path_creation() {
        let server = SocketServer::new();
        assert_eq!(server.socket_path(), &PathBuf::from("/tmp/gestor-proyectos.sock"));

        let custom = SocketServer::with_path(PathBuf::from("/custom/path.sock"));
        assert_eq!(custom.socket_path(), &PathBuf::from("/custom/path.sock"));
    }

    #[test]
    fn test_response_serialization() {
        let response = TrackingResponse::ok_with_message("Test");
        let json = serde_json::to_string(&response).expect("Should serialize");

        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"message\":\"Test\""));
    }

    #[test]
    fn test_response_with_status() {
        let response = TrackingResponse::with_status(123, 3600);
        let json = serde_json::to_string(&response).expect("Should serialize");

        assert!(json.contains("\"project_id\":123"));
        assert!(json.contains("\"elapsed_seconds\":3600"));
    }

    #[test]
    fn test_message_roundtrip() {
        let original = TrackingMessage::Enter { path: "/test/path".to_string() };
        let json = serde_json::to_string(&original).expect("Should serialize");
        let parsed: TrackingMessage = serde_json::from_str(&json).expect("Should parse");

        assert_eq!(original, parsed);
    }

    #[cfg(unix)]
    #[test]
    fn test_socket_server_lifecycle() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Should create temp dir");
        let socket_path = temp_dir.path().join("test.sock");

        let mut server = SocketServer::with_path(socket_path.clone());

        // Servidor no debería estar corriendo
        assert!(!server.is_running());

        // Iniciar servidor
        server.start(|msg| {
            match msg {
                TrackingMessage::Status => TrackingResponse::ok_with_message("Running"),
                _ => TrackingResponse::ok(),
            }
        }).expect("Should start");

        assert!(server.is_running());

        // Detener servidor
        server.stop();
        assert!(!server.is_running());
        assert!(!socket_path.exists());
    }
}
