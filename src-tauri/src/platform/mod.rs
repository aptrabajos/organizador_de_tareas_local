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
    #[allow(dead_code)] // Preparado para futuras funcionalidades
    fn get_config_dir(&self) -> Result<PathBuf, String>;

    /// Obtener directorio de datos de la aplicación
    #[allow(dead_code)] // Preparado para futuras funcionalidades
    fn get_data_dir(&self) -> Result<PathBuf, String>;

    /// Obtener ruta predeterminada para backups
    #[allow(dead_code)] // Preparado para sistema de backups automáticos
    fn get_default_backup_path(&self) -> Result<PathBuf, String>;

    /// Ejecutar script personalizado con variables
    fn execute_script(
        &self,
        script: &str,
        vars: HashMap<String, String>,
    ) -> Result<(), String>;

    /// Reemplazar variables en un string destinado a un argumento de proceso
    /// real (p. ej. `Command::arg`/`Command::args`). El valor sustituido NO
    /// se escapa porque se entrega literal al proceso hijo sin pasar por
    /// ningún shell, así que no hay metacaracteres que interpretar.
    ///
    /// NUNCA usar el resultado de este método como un string que luego se
    /// pasa a `bash -c`, `sh -c`, `powershell -Command`, etc. Para eso usar
    /// [`replace_variables_shell_escaped`](Self::replace_variables_shell_escaped).
    fn replace_variables(&self, text: &str, vars: &HashMap<String, String>) -> String {
        let mut result = text.to_string();
        for (key, value) in vars {
            result = result.replace(&format!("{{{}}}", key), value);
        }
        result
    }

    /// Reemplazar variables en un string que será interpretado por un shell
    /// POSIX (`bash -c "..."`, `sh -c "..."`). Cada valor sustituido se
    /// escapa como literal de shell (comillas simples), de forma que datos
    /// de origen no confiable (p. ej. el path de un proyecto elegido por el
    /// usuario) no puedan inyectar comandos adicionales vía metacaracteres
    /// (`;`, `&`, `|`, `` ` ``, `$()`, comillas, etc.).
    fn replace_variables_shell_escaped(
        &self,
        text: &str,
        vars: &HashMap<String, String>,
    ) -> String {
        let mut result = text.to_string();
        for (key, value) in vars {
            result = result.replace(&format!("{{{}}}", key), &shell_escape_posix(value));
        }
        result
    }
}

/// Escapa un valor como literal de shell POSIX envolviéndolo en comillas
/// simples. Cualquier comilla simple embebida se cierra la comilla, se
/// escapa (`'\''`) y se reabre, de forma que el resultado siempre se
/// interpreta como un único token literal sin importar su contenido.
pub fn shell_escape_posix(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_escape_posix_wraps_plain_value_in_single_quotes() {
        assert_eq!(
            shell_escape_posix("/home/user/proyecto"),
            "'/home/user/proyecto'"
        );
    }

    #[test]
    fn shell_escape_posix_neutralizes_embedded_single_quotes_and_metacharacters() {
        let malicious = "foo'; rm -rf ~ #";
        let escaped = shell_escape_posix(malicious);
        assert_eq!(escaped, "'foo'\\''; rm -rf ~ #'");
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
