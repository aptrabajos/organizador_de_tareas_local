pub mod detection;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

pub use detection::ProgramDetector;

use crate::config::AppConfig;
use std::collections::HashMap;
use std::path::PathBuf;

/// Variable de entorno con la que nace la terminal de Trabajar. El lector de `/proc`
/// del demonio (3.10.3 del diseño del tracker) busca exactamente `GESTOR_SESION=<token>`
/// en el `environ` de cada proceso para atribuirlo a la sesión.
pub const VAR_MARCA_SESION: &str = "GESTOR_SESION";

/// Marca de una sesión de Trabajar: el token que viaja a la terminal como
/// `GESTOR_SESION=<token>`.
///
/// Es un newtype que solo se construye desde un token que cumple `^[0-9a-f]{32}$`,
/// así que el valor que llega al argv o al entorno nunca puede ser texto arbitrario.
/// El token lo genera `trabajar` (`commands/trabajar.rs`), que es el único que pasa
/// una marca.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarcaSesion(String);

impl MarcaSesion {
    /// Construye la marca si el token tiene exactamente 32 dígitos hexadecimales en
    /// minúscula. Cualquier otra cosa devuelve `None`.
    pub fn desde_token(token: &str) -> Option<Self> {
        let valido = token.len() == 32
            && token
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        valido.then(|| Self(token.to_string()))
    }

    /// El token tal cual, para `Command::env`.
    pub fn token(&self) -> &str {
        &self.0
    }

    /// La marca como un único elemento de argv: `GESTOR_SESION=<token>`.
    pub fn como_asignacion(&self) -> String {
        format!("{}={}", VAR_MARCA_SESION, self.0)
    }
}

/// Trait para operaciones específicas de plataforma
pub trait PlatformOperations {
    /// Abrir terminal en la ruta especificada.
    ///
    /// Con `marca`, la terminal nace con `GESTOR_SESION=<token>` (solo la pasa
    /// `trabajar`); sin marca, la variable se quita del entorno del hijo para que no
    /// se herede por accidente. Windows la recibe y la ignora.
    fn open_terminal(
        &self,
        path: &str,
        config: &AppConfig,
        marca: Option<&MarcaSesion>,
    ) -> Result<(), String>;

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
    fn execute_script(&self, script: &str, vars: HashMap<String, String>) -> Result<(), String>;

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

/// Relanza el binario de test para correr solo `nombre_test` (un test `#[ignore]`)
/// con `GESTOR_SESION=<token_padre>` en el entorno, y exige que pase. Sirve para
/// probar la herencia de la marca sin tocar el entorno del proceso de test, que
/// comparten todos los hilos.
#[cfg(all(test, target_os = "linux"))]
pub(crate) fn correr_test_en_subproceso(nombre_test: &str, token_padre: &str) {
    let exe = std::env::current_exe().expect("ruta del binario de test");
    let salida = std::process::Command::new(exe)
        .args([
            "--exact",
            nombre_test,
            "--ignored",
            "--test-threads=1",
            "--nocapture",
        ])
        .env(VAR_MARCA_SESION, token_padre)
        .output()
        .expect("lanzar el binario de test");
    let stdout = String::from_utf8_lossy(&salida.stdout);
    let stderr = String::from_utf8_lossy(&salida.stderr);
    assert!(
        salida.status.success() && stdout.contains("test result: ok. 1 passed"),
        "el subproceso {} falló:\n{}\n{}",
        nombre_test,
        stdout,
        stderr
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marca_sesion_solo_acepta_32_hex_en_minuscula() {
        let token = "0123456789abcdef0123456789abcdef";
        let marca = MarcaSesion::desde_token(token).expect("token válido");
        assert_eq!(marca.token(), token);
        assert_eq!(marca.como_asignacion(), format!("GESTOR_SESION={}", token));

        for invalido in [
            "",
            "0123456789abcdef0123456789abcde",   // 31
            "0123456789abcdef0123456789abcdef0", // 33
            "0123456789ABCDEF0123456789abcdef",  // mayúsculas
            "0123456789abcdef0123456789abcdeg",  // no hex
            "0123456789abcdef 123456789abcdef",  // espacio
            "0123456789abcdef;123456789abcdef",  // metacaracter
        ] {
            assert!(
                MarcaSesion::desde_token(invalido).is_none(),
                "no debía aceptar {:?}",
                invalido
            );
        }
    }

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
