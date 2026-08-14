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
            // Cargar configuración existente. `load_from_file` es infalible: si el
            // archivo está corrupto (JSON inválido, enum desconocido, etc.) lo
            // respalda y cae a los defaults del OS en vez de propagar un error que
            // terminaría en panic en main.rs antes de que exista la ventana.
            Ok(Self::load_from_file(path))
        } else {
            // Crear configuración nueva con defaults del OS
            println!("🎉 Primera ejecución - Creando configuración predeterminada");
            let config = get_os_defaults();
            Self::save_to_file(path, &config)?;
            Ok(config)
        }
    }

    /// Cargar configuración desde archivo. Nunca falla: ante cualquier problema
    /// (archivo ilegible, JSON corrupto/truncado, enum inválido, etc.) respalda el
    /// archivo original y devuelve los defaults del OS.
    fn load_from_file(path: &PathBuf) -> AppConfig {
        match Self::try_load_from_file(path) {
            Ok(config) => {
                println!("✅ Configuración cargada desde: {}", path.display());
                config
            }
            Err(e) => {
                eprintln!(
                    "⚠️ [CONFIG] No se pudo cargar la configuración desde {}: {}. Se usarán valores predeterminados.",
                    path.display(),
                    e
                );
                Self::backup_corrupt_file(path);
                get_os_defaults()
            }
        }
    }

    /// Intento de carga que sí propaga el error, para que `load_from_file` decida
    /// cómo recuperarse.
    fn try_load_from_file(path: &PathBuf) -> Result<AppConfig, String> {
        let contents = fs::read_to_string(path)
            .map_err(|e| format!("Error al leer archivo de configuración: {}", e))?;

        serde_json::from_str(&contents)
            .map_err(|e| format!("Error al parsear configuración: {}", e))
    }

    /// Respaldar un archivo de configuración corrupto renombrándolo con sufijo
    /// `.corrupto-<timestamp>` en el mismo directorio, para no perder el contenido
    /// original y dejar evidencia de qué pasó.
    fn backup_corrupt_file(path: &PathBuf) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut backup_name = path.clone().into_os_string();
        backup_name.push(format!(".corrupto-{}", timestamp));
        let backup_path = PathBuf::from(backup_name);

        match fs::rename(path, &backup_path) {
            Ok(_) => eprintln!(
                "📦 [CONFIG] Archivo corrupto respaldado en: {}",
                backup_path.display()
            ),
            Err(e) => eprintln!(
                "⚠️ [CONFIG] No se pudo respaldar el archivo corrupto ({}): {}",
                path.display(),
                e
            ),
        }
    }

    /// Guardar configuración en archivo de forma atómica: se escribe primero a un
    /// archivo temporal en el mismo directorio y luego se reemplaza el archivo final
    /// con un rename (atómico en Linux/Windows/macOS). Así una escritura interrumpida
    /// (crash, corte de luz) nunca deja config.json truncado o a medio escribir.
    fn save_to_file(path: &PathBuf, config: &AppConfig) -> Result<(), String> {
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Error al serializar configuración: {}", e))?;

        let mut tmp_name = path.clone().into_os_string();
        tmp_name.push(".tmp");
        let tmp_path = PathBuf::from(tmp_name);

        fs::write(&tmp_path, json).map_err(|e| {
            format!("Error al escribir archivo temporal de configuración: {}", e)
        })?;

        fs::rename(&tmp_path, path)
            .map_err(|e| format!("Error al reemplazar archivo de configuración: {}", e))?;

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
        // Ruta de terminal personalizada: si no existe, NO se bloquea el guardado.
        // Bloquear todo el update_config por este campo rompía guardar cosas no
        // relacionadas (ej. la carpeta de backup). El error real se maneja al abrir
        // la terminal; acá solo se advierte.
        if config.platform.terminal.mode == ProgramMode::Custom {
            if let Some(ref path) = config.platform.terminal.custom_path {
                if !PathBuf::from(path).exists() {
                    eprintln!("⚠️ [CONFIG] Ruta de terminal personalizada no existe: {}", path);
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
    #[allow(dead_code)] // Preparado para futuras migraciones de esquema de configuración
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Busca en el directorio del archivo original un sibling cuyo nombre empiece
    /// con `<nombre original>.corrupto-`, generado por `backup_corrupt_file`.
    fn find_corrupt_backup(path: &PathBuf) -> Option<PathBuf> {
        let dir = path.parent()?;
        let file_name = path.file_name()?.to_string_lossy().to_string();
        let prefix = format!("{}.corrupto-", file_name);

        fs::read_dir(dir).ok()?.filter_map(|e| e.ok()).find_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&prefix) {
                Some(entry.path())
            } else {
                None
            }
        })
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().expect("no se pudo crear tempdir");
        let path = dir.path().join("config.json");

        let mut config = get_os_defaults();
        config.ui.language = "en".to_string();
        config.ui.theme = ThemeMode::Dark;

        ConfigManager::save_to_file(&path, &config).expect("save_to_file falló");
        assert!(path.exists(), "el archivo final debe existir tras guardar");

        // No debe quedar archivo temporal huérfano.
        let tmp_path = {
            let mut s = path.clone().into_os_string();
            s.push(".tmp");
            PathBuf::from(s)
        };
        assert!(!tmp_path.exists(), "no debe quedar un .tmp tras un save exitoso");

        let loaded = ConfigManager::load_from_file(&path);
        assert_eq!(loaded.version, config.version);
        assert_eq!(loaded.ui.language, "en");
        assert_eq!(loaded.ui.theme, ThemeMode::Dark);
    }

    #[test]
    fn test_load_from_corrupted_json_falls_back_to_defaults_without_panicking() {
        let dir = tempfile::tempdir().expect("no se pudo crear tempdir");
        let path = dir.path().join("config.json");

        // JSON truncado/inválido (ej. simulando un crash a mitad de escritura).
        fs::write(&path, r#"{ "version": "0.3.0", "ui": { "theme":"#)
            .expect("no se pudo escribir config corrupto");

        let loaded = ConfigManager::load_from_file(&path);
        let defaults = get_os_defaults();

        assert_eq!(loaded.version, defaults.version);
        assert_eq!(loaded.ui.theme, defaults.ui.theme);

        // El archivo corrupto se respalda y ya no queda en la ruta original.
        assert!(!path.exists(), "el archivo corrupto debe haberse movido al backup");
        assert!(
            find_corrupt_backup(&path).is_some(),
            "debe existir un backup .corrupto-<timestamp> del archivo inválido"
        );
    }

    #[test]
    fn test_load_from_invalid_enum_value_falls_back_to_defaults_without_panicking() {
        let dir = tempfile::tempdir().expect("no se pudo crear tempdir");
        let path = dir.path().join("config.json");

        // JSON sintácticamente válido pero con un valor de enum que no existe.
        let json = r#"{
            "version": "0.3.0",
            "ui": { "theme": "blue", "language": "es", "confirm_delete": true, "show_welcome": true }
        }"#;
        fs::write(&path, json).expect("no se pudo escribir config con enum inválido");

        let loaded = ConfigManager::load_from_file(&path);
        let defaults = get_os_defaults();

        assert_eq!(loaded.ui.theme, defaults.ui.theme);
        assert!(!path.exists(), "el archivo inválido debe haberse movido al backup");
        assert!(
            find_corrupt_backup(&path).is_some(),
            "debe existir un backup .corrupto-<timestamp> del archivo con enum inválido"
        );
    }

    #[test]
    fn test_config_manager_new_does_not_panic_on_corrupted_config() {
        let dir = tempfile::tempdir().expect("no se pudo crear tempdir");
        let path = dir.path().join("config.json");
        fs::write(&path, "esto no es json").expect("no se pudo escribir config corrupto");

        // Ejercita el mismo camino que `load_or_create`, que es lo que usa
        // `ConfigManager::new()` internamente: no debe propagar un error de parseo.
        let result = ConfigManager::load_or_create(&path);
        assert!(result.is_ok(), "load_or_create no debe fallar ante un config corrupto");
    }
}
