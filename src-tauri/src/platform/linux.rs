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
        // Intentar konsole
        if Command::new("konsole")
            .args(["--workdir", path])
            .spawn()
            .is_ok()
        {
            return Ok(());
        }

        // Intentar gnome-terminal
        if Command::new("gnome-terminal")
            .args(["--working-directory", path])
            .spawn()
            .is_ok()
        {
            return Ok(());
        }

        // Intentar alacritty
        if Command::new("alacritty")
            .args(["--working-directory", path])
            .spawn()
            .is_ok()
        {
            return Ok(());
        }

        // Intentar kitty
        if Command::new("kitty")
            .args(["--directory", path])
            .spawn()
            .is_ok()
        {
            return Ok(());
        }

        // Intentar xfce4-terminal
        if Command::new("xfce4-terminal")
            .args(["--working-directory", path])
            .spawn()
            .is_ok()
        {
            return Ok(());
        }

        // Intentar tilix
        if Command::new("tilix")
            .args(["-w", path])
            .spawn()
            .is_ok()
        {
            return Ok(());
        }

        // Intentar xterm como último recurso.
        // Nota de seguridad: se usa `current_dir` para fijar el directorio de
        // trabajo (vía argv real, sin pasar por un shell) en vez de armar un
        // string `cd '{path}' && exec $SHELL` para `xterm -e`, que era
        // vulnerable a inyección de comandos si `path` contenía comillas o
        // metacaracteres de shell.
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        if Command::new("xterm")
            .current_dir(path)
            .arg("-e")
            .arg(&shell)
            .spawn()
            .is_ok()
        {
            return Ok(());
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
                if Command::new("x-terminal-emulator")
                    .arg("--working-directory")
                    .arg(path)
                    .spawn()
                    .is_ok()
                {
                    Ok(())
                } else {
                    // Fallback si x-terminal-emulator no existe
                    self.try_terminal_fallback(path)
                }
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
        // Reemplazar variables escapando cada valor como literal de shell
        // POSIX. El script en sí lo escribe el usuario (modo "Custom Script")
        // y se ejecuta intencionalmente con `bash -c`, pero los VALORES
        // sustituidos (p. ej. el path del proyecto) son texto libre que no
        // debe poder inyectar comandos adicionales.
        let script_with_vars = self.replace_variables_shell_escaped(script, &vars);

        // Ejecutar con bash
        Command::new("bash")
            .arg("-c")
            .arg(&script_with_vars)
            .spawn()
            .map_err(|e| format!("Error al ejecutar script: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_script_escapes_malicious_path_as_single_argument() {
        let platform = LinuxPlatform::new();
        let malicious = "foo'; touch /tmp/gestor_proyectos_pwned_marker; echo '";
        let vars: HashMap<String, String> = [("path".to_string(), malicious.to_string())]
            .into_iter()
            .collect();

        // Mismo patrón que execute_script: sustituir variables escapadas y
        // pasar el resultado a `bash -c`.
        let script = platform.replace_variables_shell_escaped("printf '%s' {path}", &vars);

        let output = Command::new("bash")
            .arg("-c")
            .arg(&script)
            .output()
            .expect("bash debería poder ejecutarse en el entorno de test");

        assert!(
            output.status.success(),
            "el comando generado no debería fallar: {:?}",
            output
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            malicious,
            "el path malicioso debe llegar intacto como UN solo argumento, sin ejecutar el `touch` embebido"
        );
        assert!(
            !std::path::Path::new("/tmp/gestor_proyectos_pwned_marker").exists(),
            "el comando embebido en el path NO debía ejecutarse"
        );
    }

    #[test]
    fn execute_script_plain_path_still_substitutes_correctly() {
        let platform = LinuxPlatform::new();
        let path = "/home/user/mi proyecto";
        let vars: HashMap<String, String> = [("path".to_string(), path.to_string())]
            .into_iter()
            .collect();

        let script = platform.replace_variables_shell_escaped("printf '%s' {path}", &vars);

        let output = Command::new("bash")
            .arg("-c")
            .arg(&script)
            .output()
            .expect("bash debería poder ejecutarse en el entorno de test");

        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout), path);
    }
}
