use super::schema::*;
use super::defaults::get_os_defaults;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

/// Manager de configuración de la aplicación
pub struct ConfigManager {
    config: Mutex<AppConfig>,
    config_path: PathBuf,
}

impl ConfigManager {
    /// Crear nuevo manager de configuración
    pub fn new() -> Result<Self, String> {
        let config_path = Self::get_config_path()?;
        let config = Self::load_or_create(&config_path)?;

        Ok(Self {
            config: Mutex::new(config),
            config_path,
        })
    }

    /// Obtener la ruta del archivo de configuración según el OS
    fn get_config_path() -> Result<PathBuf, String> {
        let config_dir = Self::get_config_dir()?;

        // Crear directorio si no existe
        fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Error al crear directorio de configuración: {}", e))?;

        Ok(config_dir.join("config.json"))
    }

    /// Obtener directorio de configuración según el OS
    pub fn get_config_dir() -> Result<PathBuf, String> {
        #[cfg(target_os = "windows")]
        {
            // Windows: %APPDATA%/gestor-proyectos
            let appdata = dirs::config_dir()
                .ok_or("No se pudo obtener directorio APPDATA")?;
            Ok(appdata.join("gestor-proyectos"))
        }

        #[cfg(target_os = "linux")]
        {
            // Linux: ~/.config/gestor-proyectos
            let config_dir = dirs::config_dir()
                .ok_or("No se pudo obtener directorio de configuración")?;
            Ok(config_dir.join("gestor-proyectos"))
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Err("Sistema operativo no soportado".to_string())
        }
    }

    /// Cargar configuración del archivo o crear una nueva con defaults
    fn load_or_create(path: &PathBuf) -> Result<AppConfig, String> {
        if path.exists() {
            // Cargar configuración existente
            Self::load_from_file(path)
        } else {
            // Crear configuración nueva con defaults del OS
            println!("🎉 Primera ejecución - Creando configuración predeterminada");
            let config = get_os_defaults();
            Self::save_to_file(path, &config)?;
            Ok(config)
        }
    }

    /// Cargar configuración desde archivo
    fn load_from_file(path: &PathBuf) -> Result<AppConfig, String> {
        let contents = fs::read_to_string(path)
            .map_err(|e| format!("Error al leer archivo de configuración: {}", e))?;

        let config: AppConfig = serde_json::from_str(&contents)
            .map_err(|e| format!("Error al parsear configuración: {}", e))?;

        println!("✅ Configuración cargada desde: {}", path.display());
        Ok(config)
    }

    /// Guardar configuración en archivo
    fn save_to_file(path: &PathBuf, config: &AppConfig) -> Result<(), String> {
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Error al serializar configuración: {}", e))?;

        fs::write(path, json)
            .map_err(|e| format!("Error al escribir archivo de configuración: {}", e))?;

        println!("💾 Configuración guardada en: {}", path.display());
        Ok(())
    }

    /// Obtener copia de la configuración actual
    pub fn get_config(&self) -> Result<AppConfig, String> {
        self.config
            .lock()
            .map(|config| config.clone())
            .map_err(|e| format!("Error al obtener configuración: {}", e))
    }

    /// Actualizar configuración completa
    pub fn update_config(&self, new_config: AppConfig) -> Result<(), String> {
        // Validar configuración antes de guardar
        self.validate_config(&new_config)?;

        // Actualizar en memoria
        {
            let mut config = self.config
                .lock()
                .map_err(|e| format!("Error al bloquear configuración: {}", e))?;
            *config = new_config.clone();
        }

        // Guardar en disco
        Self::save_to_file(&self.config_path, &new_config)?;

        Ok(())
    }

    /// Resetear configuración a valores predeterminados
    pub fn reset_config(&self) -> Result<AppConfig, String> {
        let default_config = get_os_defaults();
        self.update_config(default_config.clone())?;
        println!("🔄 Configuración reseteada a valores predeterminados");
        Ok(default_config)
    }

    /// Validar configuración
    fn validate_config(&self, config: &AppConfig) -> Result<(), String> {
        // Validar que los paths personalizados existan
        if config.platform.terminal.mode == ProgramMode::Custom {
            if let Some(ref path) = config.platform.terminal.custom_path {
                if !PathBuf::from(path).exists() {
                    return Err(format!("Ruta de terminal no existe: {}", path));
                }
            }
        }

        // Validar backup path si está configurado
        if let Some(ref path) = config.backup.default_path {
            let backup_path = PathBuf::from(path);
            if !backup_path.exists() {
                // Intentar crear el directorio
                fs::create_dir_all(&backup_path)
                    .map_err(|e| format!("No se pudo crear directorio de backup: {}", e))?;
            }
        }

        Ok(())
    }

    /// Migrar configuración de versión anterior si es necesario
    pub fn migrate_if_needed(&self) -> Result<(), String> {
        let config = self.get_config()?;

        // Por ahora solo tenemos la versión 0.2.0
        // En el futuro aquí iría la lógica de migración
        if config.version != "0.2.0" {
            println!("⚠️ Versión de configuración desconocida: {}", config.version);
            println!("💡 Considera resetear la configuración si hay problemas");
        }

        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().expect("Error al crear ConfigManager")
    }
}
