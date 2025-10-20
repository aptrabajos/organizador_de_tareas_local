use super::schema::*;
use std::collections::HashMap;

const CONFIG_VERSION: &str = "0.2.0";

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION.to_string(),
            platform: PlatformConfig::default(),
            backup: BackupConfig::default(),
            ui: UiConfig::default(),
            advanced: AdvancedConfig::default(),
        }
    }
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            terminal: ProgramConfig::default(),
            browser: ProgramConfig::default(),
            file_manager: ProgramConfig::default(),
            text_editor: ProgramConfig::default(),
            environment: HashMap::new(),
        }
    }
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            default_path: None, // Se detectará automáticamente
            auto_backup_enabled: false,
            auto_backup_interval: 7, // 7 días
            cleanup_old_backups: false,
            retention_days: 30,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Auto,
            language: "es".to_string(),
            confirm_delete: true,
            show_welcome: true,
        }
    }
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            log_level: LogLevel::Info,
            enable_analytics: true,
            database_path: None,
            enable_auto_update: true,
        }
    }
}

/// Obtener configuración predeterminada específica por sistema operativo
pub fn get_os_defaults() -> AppConfig {
    let mut config = AppConfig::default();

    // Configuración específica para Windows
    #[cfg(target_os = "windows")]
    {
        // Windows: agregar variables de entorno comunes
        config.platform.environment.insert(
            "USERPROFILE".to_string(),
            std::env::var("USERPROFILE").unwrap_or_default(),
        );
    }

    // Configuración específica para Linux
    #[cfg(target_os = "linux")]
    {
        // Linux: agregar variables de entorno comunes
        config.platform.environment.insert(
            "HOME".to_string(),
            std::env::var("HOME").unwrap_or_default(),
        );
    }

    config
}
