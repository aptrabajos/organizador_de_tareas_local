use super::PlatformOperations;
use crate::config::{AppConfig, ProgramMode};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// Implementación de operaciones de plataforma para Windows
pub struct WindowsPlatform;

impl WindowsPlatform {
    pub fn new() -> Self {
        Self
    }

    /// Intentar abrir Windows Terminal
    fn try_windows_terminal(&self, path: &str) -> Result<(), String> {
        Command::new("wt")
            .arg("-d")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Error al abrir Windows Terminal: {}", e))?;
        Ok(())
    }

    /// Intentar abrir PowerShell
    fn try_powershell(&self, path: &str) -> Result<(), String> {
        Command::new("powershell")
            .arg("-NoExit")
            .arg("-Command")
            .arg(&format!("Set-Location '{}'", path))
            .spawn()
            .map_err(|e| format!("Error al abrir PowerShell: {}", e))?;
        Ok(())
    }

    /// Intentar abrir CMD
    fn try_cmd(&self, path: &str) -> Result<(), String> {
        Command::new("cmd")
            .arg("/K")
            .arg(&format!("cd /d \"{}\"", path))
            .spawn()
            .map_err(|e| format!("Error al abrir CMD: {}", e))?;
        Ok(())
    }

    /// Fallback: intentar terminales en orden de preferencia
    fn try_terminal_fallback(&self, path: &str) -> Result<(), String> {
        // Prioridad: Windows Terminal > PowerShell > CMD
        self.try_windows_terminal(path)
            .or_else(|_| self.try_powershell(path))
            .or_else(|_| self.try_cmd(path))
            .map_err(|_| "No se encontró ningún terminal instalado".to_string())
    }
}

impl PlatformOperations for WindowsPlatform {
    fn open_terminal(&self, path: &str, config: &AppConfig) -> Result<(), String> {
        match &config.platform.terminal.mode {
            ProgramMode::Auto => {
                // Intentar con terminales comunes en orden de preferencia
                self.try_terminal_fallback(path)
            }
            ProgramMode::Default => {
                // Usar CMD como predeterminado de Windows
                self.try_cmd(path)
            }
            ProgramMode::Custom => {
                let program = config
                    .platform
                    .terminal
                    .custom_path
                    .as_ref()
                    .ok_or("Ruta de terminal personalizado no configurada")?;

                let vars: HashMap<String, String> =
                    [("path".to_string(), path.to_string())]
                        .iter()
                        .cloned()
                        .collect();

                let args: Vec<String> = config
                    .platform
                    .terminal
                    .custom_args
                    .iter()
                    .map(|arg| self.replace_variables(arg, &vars))
                    .collect();

                Command::new(program)
                    .args(&args)
                    .spawn()
                    .map_err(|e| format!("Error al ejecutar terminal personalizado: {}", e))?;

                Ok(())
            }
            ProgramMode::Script => {
                let script = config
                    .platform
                    .terminal
                    .custom_script
                    .as_ref()
                    .ok_or("Script de terminal no configurado")?;

                let vars: HashMap<String, String> =
                    [("path".to_string(), path.to_string())]
                        .iter()
                        .cloned()
                        .collect();

                self.execute_script(script, vars)
            }
        }
    }

    fn open_url(&self, url: &str, config: &AppConfig) -> Result<(), String> {
        match &config.platform.browser.mode {
            ProgramMode::Auto | ProgramMode::Default => {
                // Usar 'start' de Windows (abre con el navegador predeterminado)
                Command::new("cmd")
                    .args(["/c", "start", url])
                    .spawn()
                    .map_err(|e| format!("Error al abrir URL: {}", e))?;
                Ok(())
            }
            ProgramMode::Custom => {
                let program = config
                    .platform
                    .browser
                    .custom_path
                    .as_ref()
                    .ok_or("Ruta de navegador personalizado no configurada")?;

                Command::new(program)
                    .arg(url)
                    .spawn()
                    .map_err(|e| format!("Error al abrir navegador personalizado: {}", e))?;

                Ok(())
            }
            ProgramMode::Script => {
                let script = config
                    .platform
                    .browser
                    .custom_script
                    .as_ref()
                    .ok_or("Script de navegador no configurado")?;

                let vars: HashMap<String, String> =
                    [("url".to_string(), url.to_string())]
                        .iter()
                        .cloned()
                        .collect();

                self.execute_script(script, vars)
            }
        }
    }

    fn open_file_manager(&self, path: &str, _config: &AppConfig) -> Result<(), String> {
        // Usar Windows Explorer
        Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Error al abrir explorador de archivos: {}", e))?;
        Ok(())
    }

    fn open_text_editor(&self, path: &str, config: &AppConfig) -> Result<(), String> {
        match &config.platform.text_editor.mode {
            ProgramMode::Auto | ProgramMode::Default => {
                // Usar notepad como predeterminado
                Command::new("notepad")
                    .arg(path)
                    .spawn()
                    .map_err(|e| format!("Error al abrir editor: {}", e))?;
                Ok(())
            }
            ProgramMode::Custom => {
                let program = config
                    .platform
                    .text_editor
                    .custom_path
                    .as_ref()
                    .ok_or("Ruta de editor personalizado no configurada")?;

                Command::new(program)
                    .arg(path)
                    .spawn()
                    .map_err(|e| format!("Error al abrir editor personalizado: {}", e))?;

                Ok(())
            }
            ProgramMode::Script => {
                let script = config
                    .platform
                    .text_editor
                    .custom_script
                    .as_ref()
                    .ok_or("Script de editor no configurado")?;

                let vars: HashMap<String, String> =
                    [("path".to_string(), path.to_string())]
                        .iter()
                        .cloned()
                        .collect();

                self.execute_script(script, vars)
            }
        }
    }

    fn get_config_dir(&self) -> Result<PathBuf, String> {
        let appdata = dirs::config_dir()
            .ok_or("No se pudo obtener directorio APPDATA")?;
        Ok(appdata.join("gestor-proyectos"))
    }

    fn get_data_dir(&self) -> Result<PathBuf, String> {
        let local_appdata = dirs::data_local_dir()
            .ok_or("No se pudo obtener directorio LOCALAPPDATA")?;
        Ok(local_appdata.join("gestor-proyectos"))
    }

    fn get_default_backup_path(&self) -> Result<PathBuf, String> {
        let documents = dirs::document_dir()
            .ok_or("No se pudo obtener directorio de Documentos")?;
        Ok(documents.join("Backups").join("gestor-proyectos"))
    }

    fn execute_script(
        &self,
        script: &str,
        vars: HashMap<String, String>,
    ) -> Result<(), String> {
        // Reemplazar variables en el script
        let script_with_vars = self.replace_variables(script, &vars);

        // Ejecutar con PowerShell
        Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(&script_with_vars)
            .spawn()
            .map_err(|e| format!("Error al ejecutar script: {}", e))?;

        Ok(())
    }
}
