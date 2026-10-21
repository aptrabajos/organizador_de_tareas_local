use super::{MarcaSesion, PlatformOperations, VAR_MARCA_SESION};
use crate::config::{AppConfig, ProgramMode};
use log::warn;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;

/// Ruta absoluta de `env`, que pone la marca en el argv de la terminal.
const ENV_ABSOLUTO: &str = "/usr/bin/env";

/// Terminales que prueba el modo `Auto`, en orden de preferencia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Terminal {
    Konsole,
    Gnome,
    Alacritty,
    Kitty,
    Xfce4,
    Tilix,
    Xterm,
}

const TERMINALES: [Terminal; 7] = [
    Terminal::Konsole,
    Terminal::Gnome,
    Terminal::Alacritty,
    Terminal::Kitty,
    Terminal::Xfce4,
    Terminal::Tilix,
    Terminal::Xterm,
];

impl Terminal {
    fn programa(self) -> &'static str {
        match self {
            Terminal::Konsole => "konsole",
            Terminal::Gnome => "gnome-terminal",
            Terminal::Alacritty => "alacritty",
            Terminal::Kitty => "kitty",
            Terminal::Xfce4 => "xfce4-terminal",
            Terminal::Tilix => "tilix",
            Terminal::Xterm => "xterm",
        }
    }
}

/// Arma el argv completo (programa incluido) para abrir `terminal` en `path`.
///
/// Con marca sigue la tabla de flags de 3.10.3 del diseño del tracker: se fuerza un
/// proceso nuevo donde la terminal delegaría en una instancia existente (que no
/// recibiría nuestro entorno) y la shell se lanza como
/// `/usr/bin/env GESTOR_SESION=<token> $SHELL`, con la marca como **un** elemento del
/// argv, nunca dentro de un string de shell. `$SHELL` va sin `-l`, como la lanza
/// konsole. tilix es la excepción: su `-e` recibe un string de shell, así que ahí la
/// marca viaja solo por `Command::env`.
///
/// Sin marca el argv queda como antes: solo el directorio de trabajo.
fn argv_terminal(
    terminal: Terminal,
    path: &str,
    marca: Option<&MarcaSesion>,
    shell: &str,
) -> Vec<String> {
    let programa = terminal.programa().to_string();

    let Some(marca) = marca else {
        let resto: Vec<String> = match terminal {
            Terminal::Konsole => vec!["--workdir".into(), path.into()],
            Terminal::Gnome => vec![format!("--working-directory={}", path)],
            Terminal::Alacritty => vec!["--working-directory".into(), path.into()],
            Terminal::Kitty => vec!["--directory".into(), path.into()],
            Terminal::Xfce4 => vec![format!("--working-directory={}", path)],
            Terminal::Tilix => vec!["-w".into(), path.into()],
            // xterm no tiene flag de directorio: lo fija `current_dir`.
            Terminal::Xterm => vec!["-e".into(), shell.into()],
        };
        return std::iter::once(programa).chain(resto).collect();
    };

    let shell_marcada = [
        ENV_ABSOLUTO.to_string(),
        marca.como_asignacion(),
        shell.to_string(),
    ];
    let (flags, con_shell): (Vec<String>, bool) = match terminal {
        Terminal::Konsole => (
            vec![
                "--separate".into(),
                "--workdir".into(),
                path.into(),
                "-e".into(),
            ],
            true,
        ),
        Terminal::Gnome => (
            vec![format!("--working-directory={}", path), "--".into()],
            true,
        ),
        Terminal::Alacritty => (
            vec!["--working-directory".into(), path.into(), "-e".into()],
            true,
        ),
        Terminal::Kitty => (vec!["--directory".into(), path.into()], true),
        Terminal::Xfce4 => (
            vec![
                "--disable-server".into(),
                format!("--working-directory={}", path),
                "-x".into(),
            ],
            true,
        ),
        Terminal::Tilix => (
            vec!["--new-process".into(), "-w".into(), path.into()],
            false,
        ),
        Terminal::Xterm => (vec!["-e".into()], true),
    };

    let mut argv = vec![programa];
    argv.extend(flags);
    if con_shell {
        argv.extend(shell_marcada);
    }
    argv
}

/// Pone (o quita) la marca en el entorno del hijo y fija su directorio de trabajo.
/// Va en todas las ramas de `open_terminal`.
fn aplicar_marca(cmd: &mut Command, path: &str, marca: Option<&MarcaSesion>) {
    cmd.current_dir(path);
    match marca {
        Some(marca) => cmd.env(VAR_MARCA_SESION, marca.token()),
        None => cmd.env_remove(VAR_MARCA_SESION),
    };
}

/// Lanza `cmd` sin dejar un zombi: el `Child` no se descarta, lo espera un hilo.
///
/// Antes cada rama hacía `spawn()` y tiraba el `Child`, así que al cerrarse la
/// terminal quedaba `[konsole] <defunct>` colgando de la app hasta que la app
/// terminaba; `xdg-open` deja el mismo zombi. Se descartó `SIGCHLD = SIG_IGN`
/// porque rompería el `waitpid` de los `Command::output()` (fallarían con `ECHILD`).
fn lanzar(cmd: &mut Command) -> std::io::Result<()> {
    let hijo = cmd.spawn()?;
    esperar_en_hilo(hijo);
    Ok(())
}

/// Un hilo con nombre que hace `wait()` sobre el hijo y termina.
fn esperar_en_hilo(mut hijo: Child) {
    let pid = hijo.id();
    let lanzado = thread::Builder::new()
        .name(format!("reaper-{}", pid))
        .spawn(move || {
            if let Err(e) = hijo.wait() {
                warn!("Failed to wait for child process {}: {}", pid, e);
            }
        });
    if let Err(e) = lanzado {
        // Sin hilo, el hijo queda como zombi hasta que la app termine: molesto,
        // pero no rompe nada.
        warn!("Failed to spawn reaper thread for process {}: {}", pid, e);
    }
}

/// Implementación de operaciones de plataforma para Linux
pub struct LinuxPlatform {
    /// `PATH` con el que se buscan los programas. `None` en producción: se usa el
    /// del proceso. Los tests lo apuntan a un `tempdir` con terminales falsas, sin
    /// tocar el entorno global del proceso de test.
    path_busqueda: Option<OsString>,
}

impl LinuxPlatform {
    pub fn new() -> Self {
        Self {
            path_busqueda: None,
        }
    }

    /// Crea el `Command` de cualquier spawn de este módulo. Siempre quita la marca
    /// del entorno del hijo (segunda barrera, después de `limpiar_entorno_heredado`
    /// en `main.rs`): solo `aplicar_marca` la vuelve a poner, y solo en la terminal.
    fn comando(&self, programa: impl AsRef<OsStr>) -> Command {
        let mut cmd = Command::new(programa);
        cmd.env_remove(VAR_MARCA_SESION);
        if let Some(path) = &self.path_busqueda {
            cmd.env("PATH", path);
        }
        cmd
    }

    /// El `Command` de un script del usuario (`bash -c`), sin lanzar. Lo usan
    /// `execute_script` (navegador, editor) y la rama `Script` de `open_terminal`,
    /// que es la única que le agrega la marca. Así la marca nunca llega a un
    /// navegador ni a un editor.
    fn comando_script(&self, script: &str, vars: &HashMap<String, String>) -> Command {
        // Reemplazar variables escapando cada valor como literal de shell
        // POSIX. El script en sí lo escribe el usuario (modo "Custom Script")
        // y se ejecuta intencionalmente con `bash -c`, pero los VALORES
        // sustituidos (p. ej. el path del proyecto) son texto libre que no
        // debe poder inyectar comandos adicionales.
        let script_with_vars = self.replace_variables_shell_escaped(script, vars);

        let mut cmd = self.comando("bash");
        cmd.arg("-c").arg(script_with_vars);
        cmd
    }

    /// Intentar abrir con diferentes terminales conocidas, en orden. Cada una se
    /// lanza con `match spawn()`: si arranca, a su `Child` lo espera un hilo (sin
    /// zombis); si falla, se prueba la siguiente.
    ///
    /// Nota de seguridad: el directorio va por `current_dir` y por argv real, sin
    /// pasar por un shell (antes xterm armaba `cd '{path}' && exec $SHELL`, que era
    /// vulnerable a inyección si `path` contenía comillas o metacaracteres).
    fn try_terminal_fallback(&self, path: &str, marca: Option<&MarcaSesion>) -> Result<(), String> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

        for terminal in TERMINALES {
            let argv = argv_terminal(terminal, path, marca, &shell);
            let mut cmd = self.comando(&argv[0]);
            cmd.args(&argv[1..]);
            aplicar_marca(&mut cmd, path, marca);

            match cmd.spawn() {
                Ok(hijo) => {
                    esperar_en_hilo(hijo);
                    return Ok(());
                }
                Err(_) => continue,
            }
        }

        Err("No se encontró ningún terminal instalado".to_string())
    }
}

impl PlatformOperations for LinuxPlatform {
    fn open_terminal(
        &self,
        path: &str,
        config: &AppConfig,
        marca: Option<&MarcaSesion>,
    ) -> Result<(), String> {
        // Todas las ramas fijan `current_dir(path)`: con una carpeta inexistente el
        // spawn fallaría y el modo Auto lo confundiría con "no hay terminal".
        if !Path::new(path).is_dir() {
            return Err(format!(
                "La carpeta del proyecto no existe: {}. Revisá la ruta del proyecto.",
                path
            ));
        }

        match &config.platform.terminal.mode {
            ProgramMode::Auto => {
                // Intentar con terminales comunes
                self.try_terminal_fallback(path, marca)
            }
            ProgramMode::Default => {
                // Usar x-terminal-emulator (enlace simbólico al terminal
                // predeterminado). La marca va solo por `Command::env`, como en
                // Custom: no se sabe qué programa hay detrás.
                let mut cmd = self.comando("x-terminal-emulator");
                cmd.arg("--working-directory").arg(path);
                aplicar_marca(&mut cmd, path, marca);
                match cmd.spawn() {
                    Ok(hijo) => {
                        esperar_en_hilo(hijo);
                        Ok(())
                    }
                    // Fallback si x-terminal-emulator no existe
                    Err(_) => self.try_terminal_fallback(path, marca),
                }
            }
            ProgramMode::Custom => {
                let program = config
                    .platform
                    .terminal
                    .custom_path
                    .as_ref()
                    .ok_or("Ruta de terminal personalizado no configurada")?;

                let vars: HashMap<String, String> = [("path".to_string(), path.to_string())]
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

                // La marca va solo por `Command::env`: llega a la shell únicamente
                // si el programa es un proceso nuevo (limitación documentada en
                // 3.10.3 del diseño del tracker).
                let mut cmd = self.comando(program);
                cmd.args(&args);
                aplicar_marca(&mut cmd, path, marca);
                lanzar(&mut cmd)
                    .map_err(|e| format!("Error al ejecutar terminal personalizado: {}", e))
            }
            ProgramMode::Script => {
                let script = config
                    .platform
                    .terminal
                    .custom_script
                    .as_ref()
                    .ok_or("Script de terminal no configurado")?;

                let vars: HashMap<String, String> = [("path".to_string(), path.to_string())]
                    .iter()
                    .cloned()
                    .collect();

                // Única rama de un script que recibe la marca.
                let mut cmd = self.comando_script(script, &vars);
                aplicar_marca(&mut cmd, path, marca);
                lanzar(&mut cmd).map_err(|e| format!("Error al ejecutar script: {}", e))
            }
        }
    }

    fn open_url(&self, url: &str, config: &AppConfig) -> Result<(), String> {
        match &config.platform.browser.mode {
            ProgramMode::Auto | ProgramMode::Default => {
                // Usar xdg-open (estándar de Linux)
                lanzar(self.comando("xdg-open").arg(url))
                    .map_err(|e| format!("Error al abrir URL: {}", e))
            }
            ProgramMode::Custom => {
                let program = config
                    .platform
                    .browser
                    .custom_path
                    .as_ref()
                    .ok_or("Ruta de navegador personalizado no configurada")?;

                lanzar(self.comando(program).arg(url))
                    .map_err(|e| format!("Error al abrir navegador personalizado: {}", e))
            }
            ProgramMode::Script => {
                let script = config
                    .platform
                    .browser
                    .custom_script
                    .as_ref()
                    .ok_or("Script de navegador no configurado")?;

                let vars: HashMap<String, String> = [("url".to_string(), url.to_string())]
                    .iter()
                    .cloned()
                    .collect();

                self.execute_script(script, vars)
            }
        }
    }

    fn open_file_manager(&self, path: &str, _config: &AppConfig) -> Result<(), String> {
        // Por ahora usar xdg-open, luego se puede personalizar
        lanzar(self.comando("xdg-open").arg(path))
            .map_err(|e| format!("Error al abrir explorador de archivos: {}", e))
    }

    fn open_text_editor(&self, path: &str, config: &AppConfig) -> Result<(), String> {
        match &config.platform.text_editor.mode {
            ProgramMode::Auto | ProgramMode::Default => {
                // Usar xdg-open para abrir con el editor predeterminado
                lanzar(self.comando("xdg-open").arg(path))
                    .map_err(|e| format!("Error al abrir editor: {}", e))
            }
            ProgramMode::Custom => {
                let program = config
                    .platform
                    .text_editor
                    .custom_path
                    .as_ref()
                    .ok_or("Ruta de editor personalizado no configurada")?;

                lanzar(self.comando(program).arg(path))
                    .map_err(|e| format!("Error al abrir editor personalizado: {}", e))
            }
            ProgramMode::Script => {
                let script = config
                    .platform
                    .text_editor
                    .custom_script
                    .as_ref()
                    .ok_or("Script de editor no configurado")?;

                let vars: HashMap<String, String> = [("path".to_string(), path.to_string())]
                    .iter()
                    .cloned()
                    .collect();

                self.execute_script(script, vars)
            }
        }
    }

    fn get_config_dir(&self) -> Result<PathBuf, String> {
        let config_dir =
            dirs::config_dir().ok_or("No se pudo obtener directorio de configuración")?;
        Ok(config_dir.join("gestor-proyectos"))
    }

    fn get_data_dir(&self) -> Result<PathBuf, String> {
        let data_dir = dirs::data_dir().ok_or("No se pudo obtener directorio de datos")?;
        Ok(data_dir.join("gestor-proyectos"))
    }

    fn get_default_backup_path(&self) -> Result<PathBuf, String> {
        let home = dirs::home_dir().ok_or("No se pudo obtener directorio home")?;
        Ok(home.join("Backups").join("gestor-proyectos"))
    }

    fn execute_script(&self, script: &str, vars: HashMap<String, String>) -> Result<(), String> {
        // Se lanza tal cual, sin marca (la quita `comando`): lo usan el navegador y
        // el editor. La terminal con marca arma su propio `comando_script`.
        lanzar(&mut self.comando_script(script, &vars))
            .map_err(|e| format!("Error al ejecutar script: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::correr_test_en_subproceso;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};
    use tempfile::TempDir;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";
    const TOKEN_PADRE: &str = "fedcba9876543210fedcba9876543210";
    const PLAZO: Duration = Duration::from_secs(10);

    fn marca(token: &str) -> MarcaSesion {
        MarcaSesion::desde_token(token).expect("token de test válido")
    }

    fn shell_del_proceso() -> String {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }

    /// Lo que vuelca una terminal falsa al arrancar.
    struct Volcado {
        pid: u32,
        argv: Vec<String>,
        environ: Vec<String>,
        cwd: PathBuf,
    }

    impl Volcado {
        /// Líneas `GESTOR_SESION=…` del `environ` del proceso.
        fn marcas_en_environ(&self) -> Vec<&str> {
            self.environ
                .iter()
                .filter(|l| l.starts_with("GESTOR_SESION="))
                .map(String::as_str)
                .collect()
        }

        /// Elementos del argv que llevan la marca.
        fn marcas_en_argv(&self) -> Vec<&str> {
            self.argv
                .iter()
                .filter(|a| a.contains("GESTOR_SESION"))
                .map(String::as_str)
                .collect()
        }
    }

    /// Banco de pruebas: un `tempdir` con `bin/` (terminales falsas, que es todo el
    /// `PATH` de búsqueda de la plataforma), `salida/` (lo que vuelcan) y
    /// `proyecto/` (la carpeta a abrir). Ninguna terminal real se lanza nunca.
    struct Banco {
        _dir: TempDir,
        bin: PathBuf,
        salida: PathBuf,
        proyecto: PathBuf,
    }

    impl Banco {
        fn nuevo() -> Self {
            let dir = tempfile::tempdir().expect("tempdir");
            let bin = dir.path().join("bin");
            let salida = dir.path().join("salida");
            let proyecto = dir.path().join("mi proyecto");
            for d in [&bin, &salida, &proyecto] {
                fs::create_dir_all(d).expect("crear carpeta del banco");
            }
            // Los scripts del usuario corren con `bash -c`, que también se busca en
            // este `PATH`: el único programa real que hay en `bin/`.
            std::os::unix::fs::symlink("/bin/bash", bin.join("bash")).expect("enlazar bash");
            Self {
                _dir: dir,
                bin,
                salida,
                proyecto,
            }
        }

        fn plataforma(&self) -> LinuxPlatform {
            LinuxPlatform {
                path_busqueda: Some(self.bin.clone().into_os_string()),
            }
        }

        fn proyecto(&self) -> &str {
            self.proyecto.to_str().expect("ruta UTF-8")
        }

        /// Instala en `bin/` un ejecutable `nombre` que vuelca su argv, su `environ`
        /// y su `cwd` en `salida/`, escribe su pid (al final y de forma atómica) y
        /// sale enseguida. Devuelve su ruta absoluta.
        fn instalar_terminal_falsa(&self, nombre: &str) -> PathBuf {
            let ruta = self.bin.join(nombre);
            let base = self.salida.join(nombre);
            let script = format!(
                "#!/bin/sh\n\
                 [ \"$1\" = \"--sonda\" ] && exit 0\n\
                 s='{}'\n\
                 printf '%s\\n' \"$@\" > \"$s.argv\"\n\
                 /usr/bin/tr '\\0' '\\n' < /proc/$$/environ > \"$s.environ\"\n\
                 pwd -P > \"$s.cwd\"\n\
                 echo $$ > \"$s.pid.tmp\"\n\
                 /usr/bin/mv \"$s.pid.tmp\" \"$s.pid\"\n",
                base.display()
            );
            fs::write(&ruta, script).expect("escribir terminal falsa");
            fs::set_permissions(&ruta, fs::Permissions::from_mode(0o755)).expect("chmod");
            esperar_ejecutable(&ruta);
            ruta
        }

        /// Espera a que la terminal falsa `nombre` termine de volcar y lo lee.
        fn volcado(&self, nombre: &str) -> Volcado {
            // El pid se escribe último y con `mv`: si está, lo demás ya se volcó.
            let pid = esperar_archivo(&self.archivo(nombre, "pid"));
            let leer =
                |ext: &str| fs::read_to_string(self.archivo(nombre, ext)).expect("leer volcado");
            Volcado {
                pid: pid.trim().parse().expect("pid numérico"),
                argv: leer("argv").lines().map(str::to_string).collect(),
                environ: leer("environ").lines().map(str::to_string).collect(),
                cwd: PathBuf::from(leer("cwd").trim()),
            }
        }

        fn archivo(&self, nombre: &str, ext: &str) -> PathBuf {
            self.salida.join(format!("{}.{}", nombre, ext))
        }

        fn arranco(&self, nombre: &str) -> bool {
            self.archivo(nombre, "pid").exists()
        }
    }

    /// Recién escrito, un ejecutable puede dar `ETXTBSY` si otro hilo del test hizo
    /// `fork` mientras el archivo estaba abierto para escritura. Se sondea hasta que
    /// arranque limpio, para que el fallback no lo confunda con "no está".
    fn esperar_ejecutable(ruta: &Path) {
        let inicio = Instant::now();
        loop {
            match Command::new(ruta).arg("--sonda").status() {
                Ok(_) => return,
                Err(e) if e.raw_os_error() == Some(26) && inicio.elapsed() < PLAZO => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => panic!("no se pudo sondear {}: {}", ruta.display(), e),
            }
        }
    }

    /// Espera a que exista `ruta` (escrita por un script con `mv` atómico).
    fn esperar_archivo(ruta: &Path) -> String {
        let inicio = Instant::now();
        while !ruta.exists() {
            assert!(inicio.elapsed() < PLAZO, "no apareció {}", ruta.display());
            thread::sleep(Duration::from_millis(20));
        }
        fs::read_to_string(ruta).expect("leer archivo")
    }

    fn config_terminal(modo: ProgramMode) -> AppConfig {
        let mut config = AppConfig::default();
        config.platform.terminal.mode = modo;
        config
    }

    fn sin_prefijo_de_programa(argv: Vec<String>) -> Vec<String> {
        argv.into_iter().skip(1).collect()
    }

    // ---- Golden de argv_terminal ----

    #[test]
    fn argv_terminal_golden_con_marca() {
        let m = marca(TOKEN);
        let p = "/home/u/mi proyecto";
        let sh = "/bin/zsh";
        let t = format!("GESTOR_SESION={}", TOKEN);
        let esperado: [(Terminal, Vec<&str>); 7] = [
            (
                Terminal::Konsole,
                vec![
                    "konsole",
                    "--separate",
                    "--workdir",
                    p,
                    "-e",
                    "/usr/bin/env",
                    &t,
                    sh,
                ],
            ),
            (
                Terminal::Gnome,
                vec![
                    "gnome-terminal",
                    "--working-directory=/home/u/mi proyecto",
                    "--",
                    "/usr/bin/env",
                    &t,
                    sh,
                ],
            ),
            (
                Terminal::Alacritty,
                vec![
                    "alacritty",
                    "--working-directory",
                    p,
                    "-e",
                    "/usr/bin/env",
                    &t,
                    sh,
                ],
            ),
            (
                Terminal::Kitty,
                vec!["kitty", "--directory", p, "/usr/bin/env", &t, sh],
            ),
            (
                Terminal::Xfce4,
                vec![
                    "xfce4-terminal",
                    "--disable-server",
                    "--working-directory=/home/u/mi proyecto",
                    "-x",
                    "/usr/bin/env",
                    &t,
                    sh,
                ],
            ),
            (Terminal::Tilix, vec!["tilix", "--new-process", "-w", p]),
            (Terminal::Xterm, vec!["xterm", "-e", "/usr/bin/env", &t, sh]),
        ];

        for (terminal, argv) in esperado {
            let obtenido = argv_terminal(terminal, p, Some(&m), sh);
            assert_eq!(obtenido, argv, "argv de {:?}", terminal);
            // La marca es UN elemento del argv, salvo en tilix, donde no va.
            let con_marca = obtenido.iter().filter(|a| a.contains(TOKEN)).count();
            let esperadas = usize::from(terminal != Terminal::Tilix);
            assert_eq!(con_marca, esperadas, "marcas en el argv de {:?}", terminal);
            assert!(obtenido.contains(&t) || terminal == Terminal::Tilix);
        }
    }

    #[test]
    fn argv_terminal_golden_sin_marca() {
        let p = "/home/u/mi proyecto";
        let sh = "/bin/zsh";
        let esperado: [(Terminal, Vec<&str>); 7] = [
            (Terminal::Konsole, vec!["konsole", "--workdir", p]),
            (
                Terminal::Gnome,
                vec!["gnome-terminal", "--working-directory=/home/u/mi proyecto"],
            ),
            (
                Terminal::Alacritty,
                vec!["alacritty", "--working-directory", p],
            ),
            (Terminal::Kitty, vec!["kitty", "--directory", p]),
            (
                Terminal::Xfce4,
                vec!["xfce4-terminal", "--working-directory=/home/u/mi proyecto"],
            ),
            (Terminal::Tilix, vec!["tilix", "-w", p]),
            (Terminal::Xterm, vec!["xterm", "-e", sh]),
        ];

        for (terminal, argv) in esperado {
            let obtenido = argv_terminal(terminal, p, None, sh);
            assert_eq!(obtenido, argv, "argv de {:?}", terminal);
            assert!(obtenido.iter().all(|a| !a.contains("GESTOR_SESION")));
        }
    }

    // ---- Terminal falsa en el PATH ----

    #[test]
    fn auto_con_marca_la_pone_en_el_argv_y_en_el_environ() {
        let banco = Banco::nuevo();
        banco.instalar_terminal_falsa("konsole");
        let m = marca(TOKEN);

        banco
            .plataforma()
            .open_terminal(
                banco.proyecto(),
                &config_terminal(ProgramMode::Auto),
                Some(&m),
            )
            .expect("abre la konsole falsa");

        let v = banco.volcado("konsole");
        let esperado = sin_prefijo_de_programa(argv_terminal(
            Terminal::Konsole,
            banco.proyecto(),
            Some(&m),
            &shell_del_proceso(),
        ));
        assert_eq!(v.argv, esperado);
        assert_eq!(v.marcas_en_argv(), vec![m.como_asignacion().as_str()]);
        assert_eq!(v.marcas_en_environ(), vec![m.como_asignacion().as_str()]);
        assert_eq!(v.cwd, fs::canonicalize(&banco.proyecto).unwrap());
    }

    #[test]
    fn auto_sin_marca_no_la_pone_en_ningun_lado() {
        let banco = Banco::nuevo();
        banco.instalar_terminal_falsa("konsole");

        banco
            .plataforma()
            .open_terminal(banco.proyecto(), &config_terminal(ProgramMode::Auto), None)
            .expect("abre la konsole falsa");

        let v = banco.volcado("konsole");
        assert_eq!(
            v.argv,
            vec!["--workdir".to_string(), banco.proyecto().into()]
        );
        assert!(v.marcas_en_argv().is_empty());
        assert!(v.marcas_en_environ().is_empty());
        assert_eq!(v.cwd, fs::canonicalize(&banco.proyecto).unwrap());
    }

    #[test]
    fn auto_saltea_las_terminales_que_no_estan_y_respeta_el_orden() {
        // Solo alacritty: konsole y gnome-terminal fallan en el spawn y se prueba
        // la siguiente.
        let banco = Banco::nuevo();
        banco.instalar_terminal_falsa("alacritty");
        let m = marca(TOKEN);
        banco
            .plataforma()
            .open_terminal(
                banco.proyecto(),
                &config_terminal(ProgramMode::Auto),
                Some(&m),
            )
            .expect("cae en alacritty");
        let v = banco.volcado("alacritty");
        assert_eq!(v.marcas_en_environ(), vec![m.como_asignacion().as_str()]);
        assert_eq!(v.marcas_en_argv(), vec![m.como_asignacion().as_str()]);

        // konsole y alacritty: gana konsole y alacritty nunca arranca.
        let banco = Banco::nuevo();
        banco.instalar_terminal_falsa("konsole");
        banco.instalar_terminal_falsa("alacritty");
        banco
            .plataforma()
            .open_terminal(
                banco.proyecto(),
                &config_terminal(ProgramMode::Auto),
                Some(&m),
            )
            .expect("abre konsole");
        banco.volcado("konsole");
        assert!(!banco.arranco("alacritty"));
    }

    #[test]
    fn auto_sin_ninguna_terminal_devuelve_error() {
        let banco = Banco::nuevo();
        let error = banco
            .plataforma()
            .open_terminal(banco.proyecto(), &config_terminal(ProgramMode::Auto), None)
            .unwrap_err();
        assert_eq!(error, "No se encontró ningún terminal instalado");
    }

    #[test]
    fn carpeta_inexistente_devuelve_error_en_espanol() {
        let banco = Banco::nuevo();
        banco.instalar_terminal_falsa("konsole");
        let falta = banco.proyecto.join("no-existe");
        let error = banco
            .plataforma()
            .open_terminal(
                falta.to_str().unwrap(),
                &config_terminal(ProgramMode::Auto),
                None,
            )
            .unwrap_err();
        assert!(error.starts_with("La carpeta del proyecto no existe"));
        assert!(!banco.arranco("konsole"));
    }

    #[test]
    fn custom_con_marca_la_pone_solo_en_el_environ() {
        let banco = Banco::nuevo();
        let programa = banco.instalar_terminal_falsa("mi-terminal");
        let mut config = config_terminal(ProgramMode::Custom);
        config.platform.terminal.custom_path = Some(programa.to_str().unwrap().to_string());
        config.platform.terminal.custom_args = vec!["--dir".into(), "{path}".into()];
        let m = marca(TOKEN);

        banco
            .plataforma()
            .open_terminal(banco.proyecto(), &config, Some(&m))
            .expect("abre la terminal personalizada");

        let v = banco.volcado("mi-terminal");
        assert_eq!(v.argv, vec!["--dir".to_string(), banco.proyecto().into()]);
        assert_eq!(v.marcas_en_environ(), vec![m.como_asignacion().as_str()]);
        assert_eq!(v.cwd, fs::canonicalize(&banco.proyecto).unwrap());
    }

    #[test]
    fn script_de_terminal_con_marca_la_recibe() {
        let banco = Banco::nuevo();
        let salida = banco.salida.join("script-terminal.env");
        let mut config = config_terminal(ProgramMode::Script);
        config.platform.terminal.custom_script = Some(format!(
            "/usr/bin/env > '{0}.tmp' && /usr/bin/mv '{0}.tmp' '{0}'",
            salida.display()
        ));
        let m = marca(TOKEN);

        banco
            .plataforma()
            .open_terminal(banco.proyecto(), &config, Some(&m))
            .expect("corre el script de terminal");

        let env = esperar_archivo(&salida);
        let marcas: Vec<&str> = env
            .lines()
            .filter(|l| l.starts_with("GESTOR_SESION="))
            .collect();
        assert_eq!(marcas, vec![m.como_asignacion().as_str()]);
    }

    // ---- Reaper ----

    /// `true` si el pid ya no existe. Un zombi conserva `/proc/<pid>` hasta que
    /// alguien hace `wait()`, así que esto solo pasa si el reaper lo esperó.
    fn esperar_que_desaparezca(pid: u32) {
        let ruta = PathBuf::from(format!("/proc/{}", pid));
        let inicio = Instant::now();
        while ruta.exists() {
            assert!(
                inicio.elapsed() < PLAZO,
                "el proceso {} quedó colgando (¿zombi sin reaper?)",
                pid
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn reaper_la_terminal_no_queda_como_zombi() {
        let banco = Banco::nuevo();
        banco.instalar_terminal_falsa("konsole");
        banco
            .plataforma()
            .open_terminal(
                banco.proyecto(),
                &config_terminal(ProgramMode::Auto),
                Some(&marca(TOKEN)),
            )
            .expect("abre la konsole falsa");

        let v = banco.volcado("konsole");
        esperar_que_desaparezca(v.pid);
    }

    #[test]
    fn reaper_execute_script_no_deja_zombis() {
        let banco = Banco::nuevo();
        let salida = banco.salida.join("script.pid");
        let vars: HashMap<String, String> =
            [("f".to_string(), salida.to_str().unwrap().to_string())]
                .into_iter()
                .collect();

        banco
            .plataforma()
            .execute_script("echo $$ > {f}.tmp && /usr/bin/mv {f}.tmp {f}", vars)
            .expect("corre el script");

        let pid: u32 = esperar_archivo(&salida).trim().parse().unwrap();
        esperar_que_desaparezca(pid);
    }

    // ---- La marca nunca llega a navegador ni editor ----

    #[test]
    fn comando_script_de_navegador_y_editor_sale_sin_marca() {
        let plataforma = LinuxPlatform::new();
        let vars: HashMap<String, String> = [("url".to_string(), "https://x".to_string())]
            .into_iter()
            .collect();
        let cmd = plataforma.comando_script("firefox {url}", &vars);

        // El hijo nace con la variable explícitamente quitada, aunque el padre la
        // tuviera.
        let marca_en_cmd: Vec<_> = cmd
            .get_envs()
            .filter(|(k, _)| *k == OsStr::new(VAR_MARCA_SESION))
            .collect();
        assert_eq!(marca_en_cmd, vec![(OsStr::new(VAR_MARCA_SESION), None)]);
    }

    /// Corre en un subproceso que nace con `GESTOR_SESION=TOKEN_PADRE` (la app
    /// lanzada desde una terminal marcada, sin `limpiar_entorno_heredado`): ni la
    /// terminal sin marca, ni el navegador, ni el editor la reciben, y la terminal
    /// con marca recibe la suya y no la del padre.
    #[test]
    #[ignore = "lo lanza padre_marcado_la_marca_no_se_hereda en un subproceso"]
    fn subproceso_padre_marcado() {
        assert_eq!(
            std::env::var(VAR_MARCA_SESION).as_deref(),
            Ok(TOKEN_PADRE),
            "el subproceso tenía que nacer marcado"
        );

        // Terminal sin marca.
        let banco = Banco::nuevo();
        banco.instalar_terminal_falsa("konsole");
        banco
            .plataforma()
            .open_terminal(banco.proyecto(), &config_terminal(ProgramMode::Auto), None)
            .expect("abre la konsole falsa");
        let v = banco.volcado("konsole");
        assert!(v.marcas_en_environ().is_empty(), "{:?}", v.environ);
        assert!(v.marcas_en_argv().is_empty());

        // Terminal con marca: la suya, una sola vez, nunca la del padre.
        let banco = Banco::nuevo();
        banco.instalar_terminal_falsa("konsole");
        let m = marca(TOKEN);
        banco
            .plataforma()
            .open_terminal(
                banco.proyecto(),
                &config_terminal(ProgramMode::Auto),
                Some(&m),
            )
            .expect("abre la konsole falsa");
        let v = banco.volcado("konsole");
        assert_eq!(v.marcas_en_environ(), vec![m.como_asignacion().as_str()]);

        // Navegador y editor por script, y navegador y editor personalizados.
        let banco = Banco::nuevo();
        let plataforma = banco.plataforma();
        let volcar_env = |nombre: &str| {
            let f = banco.salida.join(nombre);
            format!(
                "/usr/bin/env > '{0}.tmp' && /usr/bin/mv '{0}.tmp' '{0}'",
                f.display()
            )
        };
        let mut config = AppConfig::default();
        config.platform.browser.mode = ProgramMode::Script;
        config.platform.browser.custom_script = Some(volcar_env("navegador.env"));
        config.platform.text_editor.mode = ProgramMode::Script;
        config.platform.text_editor.custom_script = Some(volcar_env("editor.env"));
        plataforma
            .open_url("https://example.com", &config)
            .expect("script de navegador");
        plataforma
            .open_text_editor(banco.proyecto(), &config)
            .expect("script de editor");
        for nombre in ["navegador.env", "editor.env"] {
            let env = esperar_archivo(&banco.salida.join(nombre));
            assert!(
                env.lines().all(|l| !l.starts_with("GESTOR_SESION=")),
                "{} recibió la marca",
                nombre
            );
        }

        let navegador = banco.instalar_terminal_falsa("navegador");
        let editor = banco.instalar_terminal_falsa("editor");
        config.platform.browser.mode = ProgramMode::Custom;
        config.platform.browser.custom_path = Some(navegador.to_str().unwrap().to_string());
        config.platform.text_editor.mode = ProgramMode::Custom;
        config.platform.text_editor.custom_path = Some(editor.to_str().unwrap().to_string());
        plataforma
            .open_url("https://example.com", &config)
            .expect("navegador personalizado");
        plataforma
            .open_text_editor(banco.proyecto(), &config)
            .expect("editor personalizado");
        for nombre in ["navegador", "editor"] {
            let v = banco.volcado(nombre);
            assert!(
                v.marcas_en_environ().is_empty(),
                "{} recibió la marca",
                nombre
            );
        }
    }

    #[test]
    fn padre_marcado_la_marca_no_se_hereda() {
        correr_test_en_subproceso(
            "platform::linux::tests::subproceso_padre_marcado",
            TOKEN_PADRE,
        );
    }

    // ---- Escape de scripts (previos a R8-2) ----

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
