use crate::config::{DetectedProgram, DetectedPrograms};
use std::process::Command;

/// Detector de programas instalados en el sistema
pub struct ProgramDetector;

impl ProgramDetector {
    /// Detectar todos los programas disponibles
    pub fn detect_all() -> DetectedPrograms {
        DetectedPrograms {
            terminals: Self::detect_terminals(),
            browsers: Self::detect_browsers(),
            file_managers: Self::detect_file_managers(),
            text_editors: Self::detect_text_editors(),
        }
    }

    /// Detectar terminales instaladas
    pub fn detect_terminals() -> Vec<DetectedProgram> {
        let mut terminals = Vec::new();

        #[cfg(target_os = "windows")]
        {
            // Windows Terminal
            if let Some(wt) = Self::find_program("wt") {
                terminals.push(DetectedProgram {
                    name: "Windows Terminal".to_string(),
                    path: wt,
                    version: None,
                    is_default: true,
                });
            }

            // PowerShell
            if let Some(ps) = Self::find_program("powershell") {
                terminals.push(DetectedProgram {
                    name: "PowerShell".to_string(),
                    path: ps,
                    version: None,
                    is_default: terminals.is_empty(),
                });
            }

            // CMD
            if let Some(cmd) = Self::find_program("cmd") {
                terminals.push(DetectedProgram {
                    name: "Command Prompt".to_string(),
                    path: cmd,
                    version: None,
                    is_default: terminals.is_empty(),
                });
            }

            // Git Bash
            if let Some(bash) = Self::find_program("bash") {
                terminals.push(DetectedProgram {
                    name: "Git Bash".to_string(),
                    path: bash,
                    version: None,
                    is_default: false,
                });
            }
        }

        #[cfg(target_os = "linux")]
        {
            let linux_terminals = vec![
                ("konsole", "Konsole"),
                ("gnome-terminal", "GNOME Terminal"),
                ("alacritty", "Alacritty"),
                ("kitty", "Kitty"),
                ("xfce4-terminal", "XFCE Terminal"),
                ("tilix", "Tilix"),
                ("xterm", "XTerm"),
            ];

            for (cmd, name) in linux_terminals {
                if let Some(path) = Self::find_program(cmd) {
                    terminals.push(DetectedProgram {
                        name: name.to_string(),
                        path,
                        version: None,
                        is_default: terminals.is_empty(),
                    });
                }
            }
        }

        terminals
    }

    /// Detectar navegadores instalados
    pub fn detect_browsers() -> Vec<DetectedProgram> {
        let mut browsers = Vec::new();

        #[cfg(target_os = "windows")]
        {
            let windows_browsers = vec![
                ("chrome", "Google Chrome"),
                ("msedge", "Microsoft Edge"),
                ("firefox", "Mozilla Firefox"),
                ("brave", "Brave"),
            ];

            for (cmd, name) in windows_browsers {
                if let Some(path) = Self::find_program(cmd) {
                    browsers.push(DetectedProgram {
                        name: name.to_string(),
                        path,
                        version: None,
                        is_default: browsers.is_empty(),
                    });
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let linux_browsers = vec![
                ("google-chrome", "Google Chrome"),
                ("firefox", "Mozilla Firefox"),
                ("chromium", "Chromium"),
                ("brave", "Brave"),
            ];

            for (cmd, name) in linux_browsers {
                if let Some(path) = Self::find_program(cmd) {
                    browsers.push(DetectedProgram {
                        name: name.to_string(),
                        path,
                        version: None,
                        is_default: browsers.is_empty(),
                    });
                }
            }
        }

        browsers
    }

    /// Detectar exploradores de archivos instalados
    pub fn detect_file_managers() -> Vec<DetectedProgram> {
        let mut managers = Vec::new();

        #[cfg(target_os = "windows")]
        {
            // Windows Explorer (siempre disponible)
            managers.push(DetectedProgram {
                name: "Windows Explorer".to_string(),
                path: "explorer".to_string(),
                version: None,
                is_default: true,
            });
        }

        #[cfg(target_os = "linux")]
        {
            let linux_managers = vec![
                ("dolphin", "Dolphin"),
                ("nautilus", "Nautilus"),
                ("thunar", "Thunar"),
                ("nemo", "Nemo"),
                ("pcmanfm", "PCManFM"),
            ];

            for (cmd, name) in linux_managers {
                if let Some(path) = Self::find_program(cmd) {
                    managers.push(DetectedProgram {
                        name: name.to_string(),
                        path,
                        version: None,
                        is_default: managers.is_empty(),
                    });
                }
            }
        }

        managers
    }

    /// Detectar editores de texto instalados
    pub fn detect_text_editors() -> Vec<DetectedProgram> {
        let mut editors = Vec::new();

        let common_editors = vec![
            ("code", "Visual Studio Code"),
            ("code-insiders", "VS Code Insiders"),
            ("subl", "Sublime Text"),
            ("atom", "Atom"),
            ("nvim", "Neovim"),
            ("vim", "Vim"),
            ("nano", "Nano"),
        ];

        for (cmd, name) in common_editors {
            if let Some(path) = Self::find_program(cmd) {
                editors.push(DetectedProgram {
                    name: name.to_string(),
                    path,
                    version: None,
                    is_default: editors.is_empty(),
                });
            }
        }

        #[cfg(target_os = "windows")]
        {
            // Notepad (siempre disponible)
            editors.push(DetectedProgram {
                name: "Notepad".to_string(),
                path: "notepad".to_string(),
                version: None,
                is_default: editors.is_empty(),
            });
        }

        editors
    }

    /// Buscar programa en el PATH del sistema
    fn find_program(name: &str) -> Option<String> {
        // Intentar ejecutar 'where' (Windows) o 'which' (Linux/Unix)
        #[cfg(target_os = "windows")]
        let cmd = "where";

        #[cfg(target_os = "linux")]
        let cmd = "which";

        match Command::new(cmd).arg(name).output() {
            Ok(output) if output.status.success() => {
                let path = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .map(|s| s.trim().to_string());
                path
            }
            _ => None,
        }
    }

    /// Verificar si un programa existe en una ruta específica
    pub fn program_exists(path: &str) -> bool {
        std::path::Path::new(path).exists()
    }
}
