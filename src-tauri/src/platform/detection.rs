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
            let windows_terminals = vec![
                ("wt", "Windows Terminal"),
                ("WindowsTerminal", "Windows Terminal"),
                ("pwsh", "PowerShell 7"),
                ("powershell", "PowerShell"),
                ("cmd", "Command Prompt"),
                ("bash", "Git Bash"),
                ("ubuntu", "WSL Ubuntu"),
                ("debian", "WSL Debian"),
            ];

            for (cmd, name) in windows_terminals {
                if let Some(path) = Self::find_program_windows(cmd) {
                    // Evitar duplicados (ej: wt y WindowsTerminal son lo mismo)
                    if !terminals.iter().any(|t| t.name == name) {
                        terminals.push(DetectedProgram {
                            name: name.to_string(),
                            path,
                            version: None,
                            is_default: terminals.is_empty(),
                        });
                    }
                }
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
    #[allow(dead_code)] // Utilidad preparada para validación futura
    pub fn program_exists(path: &str) -> bool {
        std::path::Path::new(path).exists()
    }

    /// Buscar programa en ubicaciones comunes de Windows
    #[cfg(target_os = "windows")]
    fn find_in_windows_paths(executable_name: &str) -> Option<String> {
        use std::env;
        use std::path::Path;

        // Lista de ubicaciones comunes en Windows
        let common_paths = vec![
            // Program Files
            format!(
                "C:\\Program Files\\{}\\{}",
                executable_name, executable_name
            ),
            format!("C:\\Program Files\\{}.exe", executable_name),
            format!(
                "C:\\Program Files (x86)\\{}\\{}",
                executable_name, executable_name
            ),
            format!("C:\\Program Files (x86)\\{}.exe", executable_name),
            // AppData Local
            format!(
                "{}\\AppData\\Local\\Programs\\{}\\{}",
                env::var("USERPROFILE").unwrap_or_default(),
                executable_name,
                executable_name
            ),
            format!(
                "{}\\AppData\\Local\\{}\\{}",
                env::var("USERPROFILE").unwrap_or_default(),
                executable_name,
                executable_name
            ),
            // Scoop
            format!(
                "{}\\scoop\\apps\\{}\\current\\{}",
                env::var("USERPROFILE").unwrap_or_default(),
                executable_name,
                executable_name
            ),
            // Chocolatey
            format!("C:\\ProgramData\\chocolatey\\bin\\{}.exe", executable_name),
        ];

        for path_str in common_paths {
            let path = Path::new(&path_str);
            if path.exists() {
                return Some(path_str);
            }
            // Intentar con .exe si no se especificó
            if !path_str.ends_with(".exe") {
                let exe_path = format!("{}.exe", path_str);
                if Path::new(&exe_path).exists() {
                    return Some(exe_path);
                }
            }
        }

        None
    }

    /// Buscar programa primero en PATH, luego en ubicaciones comunes de Windows
    #[cfg(target_os = "windows")]
    fn find_program_windows(name: &str) -> Option<String> {
        // Primero intentar con 'where' (PATH del sistema)
        if let Some(path) = Self::find_program(name) {
            return Some(path);
        }

        // Si no se encontró en PATH, buscar en ubicaciones comunes
        Self::find_in_windows_paths(name)
    }

    /// Detectar programas específicos de Windows con búsqueda mejorada
    #[cfg(target_os = "windows")]
    pub fn detect_windows_specific() -> Vec<DetectedProgram> {
        let mut programs = Vec::new();

        // Terminales adicionales para Windows
        let windows_terminals = vec![
            ("wt", "Windows Terminal"),
            ("WindowsTerminal", "Windows Terminal"),
            ("pwsh", "PowerShell 7"),
            ("bash", "Git Bash"),
            ("ubuntu", "WSL Ubuntu"),
            ("debian", "WSL Debian"),
        ];

        for (cmd, name) in windows_terminals {
            if let Some(path) = Self::find_program_windows(cmd) {
                programs.push(DetectedProgram {
                    name: name.to_string(),
                    path,
                    version: None,
                    is_default: programs.is_empty(),
                });
            }
        }

        // Editores de código comunes en Windows
        let windows_editors = vec![
            ("code", "Visual Studio Code"),
            ("notepad++", "Notepad++"),
            ("sublime_text", "Sublime Text"),
            ("atom", "Atom"),
        ];

        for (cmd, name) in windows_editors {
            if let Some(path) = Self::find_program_windows(cmd) {
                programs.push(DetectedProgram {
                    name: name.to_string(),
                    path,
                    version: None,
                    is_default: false,
                });
            }
        }

        programs
    }
}
