use super::PlatformOperations;
use crate::config::{AppConfig, ProgramMode};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// Implementación de operaciones de plataforma para Linux
pub struct LinuxPlatform;

impl LinuxPlatform {
    pub fn new() -> Self {
        Self
    }

    /// Intentar abrir con diferentes terminales conocidas
    fn try_terminal_fallback(&self, path: &str) -> Result<(), String> {
        let terminals = vec![
            ("konsole", vec!["--workdir", path]),
            ("gnome-terminal", vec!["--working-directory", path]),
            ("alacritty", vec!["--working-directory", path]),
            ("kitty", vec!["--directory", path]),
            ("xfce4-terminal", vec!["--working-directory", path]),
            ("tilix", vec!["-w", path]),
            ("xterm", vec!["-e", &format!("cd '{}' && exec $SHELL", path)]),
        ];

        for (terminal, args) in terminals {
            match Command::new(terminal).args(&args).spawn() {
                Ok(_) => {
                    println!("✅ Terminal abierta: {}", terminal);
                    return Ok(());
                }
                Err(_) => continue,
            }
        }

        Err("No se encontró ningún terminal instalado".to_string())
    }
}

impl PlatformOperations for LinuxPlatform {
    fn open_terminal(&self, path: &str, config: &AppConfig) -> Result<(), String> {
        match &config.platform.terminal.mode {
            ProgramMode::Auto => {
                // Intentar con terminales comunes
                self.try_terminal_fallback(path)
            }
            ProgramMode::Default => {
                // Usar x-terminal-emulator (enlace simbólico al terminal predeterminado)
                Command::new("x-terminal-emulator")
                    .arg("--working-directory")
                    .arg(path)
                    .spawn()
                    .or_else(|_| {
                        // Fallback si x-terminal-emulator no existe
                        self.try_terminal_fallback(path).map(|_| ())
                    })
                    .map_err(|e| format!("Error al abrir terminal: {}", e))?;
                Ok(())
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
                // Usar xdg-open (estándar de Linux)
                Command::new("xdg-open")
                    .arg(url)
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
        // Por ahora usar xdg-open, luego se puede personalizar
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Error al abrir explorador de archivos: {}", e))?;
        Ok(())
    }

    fn open_text_editor(&self, path: &str, config: &AppConfig) -> Result<(), String> {
        match &config.platform.text_editor.mode {
            ProgramMode::Auto | ProgramMode::Default => {
                // Usar xdg-open para abrir con el editor predeterminado
                Command::new("xdg-open")
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
        let config_dir = dirs::config_dir()
            .ok_or("No se pudo obtener directorio de configuración")?;
        Ok(config_dir.join("gestor-proyectos"))
    }

    fn get_data_dir(&self) -> Result<PathBuf, String> {
        let data_dir = dirs::data_dir()
            .ok_or("No se pudo obtener directorio de datos")?;
        Ok(data_dir.join("gestor-proyectos"))
    }

    fn get_default_backup_path(&self) -> Result<PathBuf, String> {
        let home = dirs::home_dir()
            .ok_or("No se pudo obtener directorio home")?;
        Ok(home.join("Backups").join("gestor-proyectos"))
    }

    fn execute_script(
        &self,
        script: &str,
        vars: HashMap<String, String>,
    ) -> Result<(), String> {
        // Reemplazar variables en el script
        let script_with_vars = self.replace_variables(script, &vars);

        // Ejecutar con bash
        Command::new("bash")
            .arg("-c")
            .arg(&script_with_vars)
            .spawn()
            .map_err(|e| format!("Error al ejecutar script: {}", e))?;

        Ok(())
    }
}
