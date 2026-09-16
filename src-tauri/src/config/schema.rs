use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuración completa de la aplicación
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)] // forward-compat: campos faltantes en config.json viejo usan Default en vez de fallar
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
    /// Configuración de atajos de teclado
    pub shortcuts: ShortcutsConfig,
}

/// Configuración de plataforma (programas y comportamientos del OS)
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)] // forward-compat: tolera campos faltantes
pub struct PlatformConfig {
    /// Sistema operativo a utilizar (Auto detecta automáticamente)
    #[serde(default)]
    pub os_override: OsOverride,
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
#[serde(default)] // forward-compat: tolera campos faltantes
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
#[serde(default)] // forward-compat: tolera campos faltantes
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
    /// Timestamp del último backup realizado (formato ISO local, ej. "2026-06-27 14:30:05")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backup: Option<String>,
}

/// Configuración de interfaz de usuario
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)] // forward-compat: tolera campos faltantes
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
#[serde(default)] // forward-compat: tolera campos faltantes
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

impl LogLevel {
    /// Traduce el nivel de la config al filtro del crate `log`.
    ///
    /// Es la bisagra que hace que `advanced.log_level` DEJE de ser decorativa: la
    /// perilla existía en Settings y se persistía, pero no tenía ningún consumidor
    /// —no había logger que configurar—, así que no controlaba nada.
    ///
    /// Un `LevelFilter` admite un record si `record.level() <= filtro`, o sea que
    /// `Warn` deja pasar `Error` y `Warn`, y descarta `Info` y `Debug`.
    pub fn to_level_filter(&self) -> log::LevelFilter {
        match self {
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Debug,
        }
    }
}

/// Sistema operativo a utilizar
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OsOverride {
    /// Detectar automáticamente el sistema operativo
    Auto,
    /// Forzar comportamiento de Linux
    Linux,
    /// Forzar comportamiento de Windows
    Windows,
}

impl Default for OsOverride {
    fn default() -> Self {
        Self::Auto
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_default_roundtrip() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.version, config.version);
        assert_eq!(deserialized.ui.language, "es");
        assert!(deserialized.ui.confirm_delete);
    }

    #[test]
    fn test_forward_compat_old_config_missing_fields() {
        // Simula un config.json viejo (v0.2.0) SIN el campo `shortcuts` (añadido en 0.3.0)
        // y con `ui` parcial (sin show_welcome). Antes del fix esto rompía la
        // deserialización -> Err -> panic en el arranque. Con #[serde(default)] debe
        // cargar y rellenar los campos faltantes con sus defaults.
        let old_json = r#"{
            "version": "0.2.0",
            "platform": {
                "terminal": { "mode": "auto" },
                "browser": { "mode": "auto" },
                "file_manager": { "mode": "auto" },
                "text_editor": { "mode": "auto" },
                "environment": {}
            },
            "backup": {
                "auto_backup_enabled": false,
                "auto_backup_interval": 7,
                "cleanup_old_backups": false,
                "retention_days": 30
            },
            "ui": {
                "theme": "auto",
                "language": "es",
                "confirm_delete": true
            },
            "advanced": {
                "log_level": "info",
                "enable_analytics": true,
                "enable_auto_update": true
            }
        }"#;

        let config: AppConfig =
            serde_json::from_str(old_json).expect("config viejo debe deserializar sin fallar");

        // El campo ausente `shortcuts` se rellena con el default (6 atajos)
        assert!(config.shortcuts.enabled);
        assert_eq!(config.shortcuts.shortcuts.len(), 6);
        // El campo nested ausente `ui.show_welcome` toma el default (true)
        assert!(config.ui.show_welcome);
        // Los valores presentes se respetan
        assert_eq!(config.version, "0.2.0");
        assert_eq!(config.ui.language, "es");
    }

    #[test]
    fn test_program_mode_serde_rename() {
        let auto_json = serde_json::to_string(&ProgramMode::Auto).unwrap();
        assert_eq!(auto_json, "\"auto\"");
        let custom_json = serde_json::to_string(&ProgramMode::Custom).unwrap();
        assert_eq!(custom_json, "\"custom\"");
        let default_json = serde_json::to_string(&ProgramMode::Default).unwrap();
        assert_eq!(default_json, "\"default\"");
        let script_json = serde_json::to_string(&ProgramMode::Script).unwrap();
        assert_eq!(script_json, "\"script\"");
        // Roundtrip
        let parsed: ProgramMode = serde_json::from_str("\"auto\"").unwrap();
        assert_eq!(parsed, ProgramMode::Auto);
    }

    #[test]
    fn test_shortcuts_config_default_keys() {
        let config = ShortcutsConfig::default();
        assert!(config.enabled);
        let keys: Vec<&String> = config.shortcuts.keys().collect();
        assert_eq!(config.shortcuts.len(), 6);
        assert!(config.shortcuts.contains_key("new_project"));
        assert!(config.shortcuts.contains_key("search"));
        assert!(config.shortcuts.contains_key("settings"));
        assert!(config.shortcuts.contains_key("about"));
        assert!(config.shortcuts.contains_key("refresh"));
        assert!(config.shortcuts.contains_key("close_modal"));
    }
}

/// Configuración de atajos de teclado
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)] // forward-compat: tolera campos faltantes
pub struct ShortcutsConfig {
    /// Habilitar atajos de teclado globales
    pub enabled: bool,
    /// Mapa de acciones a combinaciones de teclas
    pub shortcuts: HashMap<String, ShortcutBinding>,
}

/// Configuración de un atajo de teclado individual
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShortcutBinding {
    /// Combinación de teclas (ej: "Ctrl+N", "Cmd+T")
    pub key: String,
    /// Si este atajo específico está habilitado
    pub enabled: bool,
    /// Descripción de la acción
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
