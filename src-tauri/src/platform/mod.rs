pub mod detection;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

pub use detection::ProgramDetector;

use crate::config::AppConfig;
use std::collections::HashMap;
use std::path::PathBuf;

/// Trait para operaciones específicas de plataforma
pub trait PlatformOperations {
    /// Abrir terminal en la ruta especificada
    fn open_terminal(&self, path: &str, config: &AppConfig) -> Result<(), String>;

    /// Abrir URL en el navegador
    fn open_url(&self, url: &str, config: &AppConfig) -> Result<(), String>;

    /// Abrir explorador de archivos en la ruta especificada
    fn open_file_manager(&self, path: &str, config: &AppConfig) -> Result<(), String>;

    /// Abrir editor de texto con el archivo especificado
    fn open_text_editor(&self, path: &str, config: &AppConfig) -> Result<(), String>;

    /// Obtener directorio de configuración de la aplicación
    fn get_config_dir(&self) -> Result<PathBuf, String>;

    /// Obtener directorio de datos de la aplicación
    fn get_data_dir(&self) -> Result<PathBuf, String>;

    /// Obtener ruta predeterminada para backups
    fn get_default_backup_path(&self) -> Result<PathBuf, String>;

    /// Ejecutar script personalizado con variables
    fn execute_script(
        &self,
        script: &str,
        vars: HashMap<String, String>,
    ) -> Result<(), String>;

    /// Reemplazar variables en string
    fn replace_variables(&self, text: &str, vars: &HashMap<String, String>) -> String {
        let mut result = text.to_string();
        for (key, value) in vars {
            result = result.replace(&format!("{{{}}}", key), value);
        }
        result
    }
}

/// Obtener instancia de PlatformOperations según el OS actual
pub fn get_platform() -> Box<dyn PlatformOperations> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsPlatform::new())
    }

    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxPlatform::new())
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        panic!("Sistema operativo no soportado")
    }
}
