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
    ///
    /// Nota de seguridad: en vez de armar `Set-Location '{path}'` como string
    /// para `-Command` (vulnerable a inyección si `path` contiene comillas o
    /// metacaracteres de PowerShell), se fija el directorio de trabajo del
    /// proceso vía `current_dir` (argv real / API de creación de proceso),
    /// sin necesidad de interpolar `path` en ningún string ejecutable.
    fn try_powershell(&self, path: &str) -> Result<(), String> {
        Command::new("powershell")
            .current_dir(path)
            .arg("-NoExit")
            .spawn()
            .map_err(|e| format!("Error al abrir PowerShell: {}", e))?;
        Ok(())
    }

    /// Intentar abrir CMD
    ///
    /// Nota de seguridad: en vez de armar `cd /d "{path}"` como string para
    /// `/K` (vulnerable a inyección si `path` contiene comillas o
    /// metacaracteres de cmd.exe como `&`, `|`, `%`), se fija el directorio
    /// de trabajo del proceso vía `current_dir`, sin interpolar `path` en
    /// ningún string ejecutable.
    fn try_cmd(&self, path: &str) -> Result<(), String> {
        Command::new("cmd")
            .current_dir(path)
            .arg("/K")
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

    /// Reemplazar variables en un string que será interpretado por
    /// PowerShell (`powershell -Command "..."`). Cada valor sustituido se
    /// escapa como literal de PowerShell (comillas simples, duplicando las
    /// comillas simples embebidas), de forma que datos de origen no
    /// confiable (p. ej. el path de un proyecto elegido por el usuario) no
    /// puedan inyectar comandos adicionales vía `;`, `&`, backticks,
    /// `$(...)`, comillas, etc.
    fn replace_variables_powershell_escaped(
        &self,
        text: &str,
        vars: &HashMap<String, String>,
    ) -> String {
        let mut result = text.to_string();
        for (key, value) in vars {
            result = result.replace(&format!("{{{}}}", key), &escape_powershell(value));
        }
        result
    }
}

/// Escapa un valor como literal de PowerShell envolviéndolo en comillas
/// simples. Dentro de un string de comillas simples, PowerShell no expande
/// variables (`$foo`), subexpresiones (`$(...)`) ni backticks; la única
/// regla de escape necesaria es duplicar cada comilla simple embebida.
fn escape_powershell(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
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
        // Reemplazar variables escapando cada valor como literal de
        // PowerShell. El script en sí lo escribe el usuario (modo "Custom
        // Script") y se ejecuta intencionalmente con `powershell -Command`,
        // pero los VALORES sustituidos (p. ej. el path del proyecto) son
        // texto libre que no debe poder inyectar comandos adicionales.
        let script_with_vars = self.replace_variables_powershell_escaped(script, &vars);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_powershell_wraps_plain_value_in_single_quotes() {
        assert_eq!(
            escape_powershell("C:\\proyectos\\foo"),
            "'C:\\proyectos\\foo'"
        );
    }

    #[test]
    fn escape_powershell_neutralizes_embedded_quotes_and_metacharacters() {
        // Un path malicioso que intenta cerrar el string y encadenar un
        // comando adicional vía `;`.
        let malicious = "foo'; Remove-Item -Recurse -Force C:\\ #";
        let escaped = escape_powershell(malicious);
        // La comilla simple embebida se duplica en vez de cerrar el string,
        // así que todo el contenido queda como un único literal inerte.
        assert_eq!(escaped, "'foo''; Remove-Item -Recurse -Force C:\\ #'");
    }

    #[test]
    fn replace_variables_powershell_escaped_substitutes_escaped_value() {
        let platform = WindowsPlatform::new();
        let malicious = "foo'; Remove-Item -Recurse -Force C:\\ #";
        let vars: HashMap<String, String> = [("path".to_string(), malicious.to_string())]
            .into_iter()
            .collect();

        let script = platform.replace_variables_powershell_escaped("Set-Location {path}", &vars);

        assert_eq!(
            script,
            "Set-Location 'foo''; Remove-Item -Recurse -Force C:\\ #'"
        );
    }
}
