use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuración completa de la aplicación
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    /// Versión del schema de configuración
    pub version: String,
    /// Configuración específica de plataforma
    pub platform: PlatformConfig,
    /// Configuración de backups
    pub backup: BackupConfig,
    /// Configuración de interfaz
    pub ui: UiConfig,
    /// Configuración avanzada
    pub advanced: AdvancedConfig,
}

/// Configuración de plataforma (programas y comportamientos del OS)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlatformConfig {
    /// Configuración de terminal
    pub terminal: ProgramConfig,
    /// Configuración de navegador
    pub browser: ProgramConfig,
    /// Configuración de explorador de archivos
    pub file_manager: ProgramConfig,
    /// Configuración de editor de texto
    pub text_editor: ProgramConfig,
    /// Variables de entorno personalizadas
    pub environment: HashMap<String, String>,
}

/// Configuración de un programa específico
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProgramConfig {
    /// Modo de ejecución del programa
    pub mode: ProgramMode,
    /// Ruta personalizada al programa (solo para Custom)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_path: Option<String>,
    /// Argumentos personalizados (solo para Custom)
    #[serde(default)]
    pub custom_args: Vec<String>,
    /// Script personalizado (solo para Script)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_script: Option<String>,
}

/// Modo de ejecución de un programa
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProgramMode {
    /// Detección automática del mejor programa disponible
    Auto,
    /// Usar el programa predeterminado del sistema
    Default,
    /// Usar un programa personalizado con ruta específica
    Custom,
    /// Ejecutar un script personalizado
    Script,
}

/// Configuración de backups
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackupConfig {
    /// Ruta predeterminada para guardar backups
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_path: Option<String>,
    /// Habilitar auto-backup periódico
    pub auto_backup_enabled: bool,
    /// Intervalo de auto-backup en días
    pub auto_backup_interval: u32,
    /// Limpiar backups antiguos automáticamente
    pub cleanup_old_backups: bool,
    /// Días de retención de backups antiguos
    pub retention_days: u32,
}

/// Configuración de interfaz de usuario
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UiConfig {
    /// Tema de la aplicación
    pub theme: ThemeMode,
    /// Idioma de la aplicación
    pub language: String,
    /// Confirmar antes de eliminar proyectos
    pub confirm_delete: bool,
    /// Mostrar pantalla de bienvenida al iniciar
    pub show_welcome: bool,
}

/// Modo de tema
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    /// Automático según sistema
    Auto,
    /// Tema claro
    Light,
    /// Tema oscuro
    Dark,
}

/// Configuración avanzada
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdvancedConfig {
    /// Nivel de logging
    pub log_level: LogLevel,
    /// Habilitar analytics/telemetría
    pub enable_analytics: bool,
    /// Ruta personalizada de base de datos
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_path: Option<String>,
    /// Habilitar auto-actualización
    pub enable_auto_update: bool,
}

/// Nivel de logging
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

/// Programa detectado en el sistema
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetectedProgram {
    /// Nombre del programa
    pub name: String,
    /// Ruta completa al ejecutable
    pub path: String,
    /// Versión (si está disponible)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Si es el programa predeterminado del sistema
    pub is_default: bool,
}

/// Resultado de detección de programas
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetectedPrograms {
    pub terminals: Vec<DetectedProgram>,
    pub browsers: Vec<DetectedProgram>,
    pub file_managers: Vec<DetectedProgram>,
    pub text_editors: Vec<DetectedProgram>,
}

/// Resultado de validación de script
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

impl Default for ProgramConfig {
    fn default() -> Self {
        Self {
            mode: ProgramMode::Auto,
            custom_path: None,
            custom_args: Vec::new(),
            custom_script: None,
        }
    }
}
