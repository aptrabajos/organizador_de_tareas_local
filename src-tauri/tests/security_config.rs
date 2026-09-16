//! Gate de regresión sobre la configuración de seguridad de `tauri.conf.json`.
//!
//! Por qué este test existe y por qué NO es un test e2e:
//!
//! La CSP la inyecta Tauri desde el protocolo `tauri://`, que sirve los assets
//! embebidos. En `tauri dev` sobre escritorio ese protocolo nunca se usa: el WebView
//! navega directo al `devUrl` de Vite, porque `PROXY_DEV_SERVER` vale
//! `cfg!(all(dev, mobile))`. O sea que en desarrollo sobre Linux y Windows la CSP
//! directamente no se emite, y un test de Playwright contra `localhost:1420` abre un
//! Chromium común contra Vite: nunca ve la política.
//!
//! Un test así pasa siempre, incluso con una CSP que rompe la aplicación en release.
//! Eso es peor que no tener gate, porque da confianza falsa.
//!
//! Verificar la CSP en runtime exige un binario empaquetado manejado por WebDriver
//! (`tauri build --debug` + `tauri-driver`), que es infraestructura que este repo hoy
//! no tiene. Mientras tanto, lo que SÍ se puede garantizar de forma automática es que
//! nadie reintroduzca la configuración insegura de la que partimos: `csp: null`,
//! `script-src` permisivo o los scopes de filesystem abiertos. Eso es lo que hace
//! este archivo.

use serde_json::Value;

fn config() -> Value {
    let raw = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tauri.conf.json"))
        .expect("no se pudo leer tauri.conf.json");
    serde_json::from_str(&raw).expect("tauri.conf.json no es JSON válido")
}

fn csp() -> String {
    config()["app"]["security"]["csp"]
        .as_str()
        .expect("app.security.csp debe ser un string: 'null' desactiva la CSP por completo")
        .to_string()
}

fn main_capability() -> Value {
    config()["app"]["security"]["capabilities"][0].clone()
}

fn permissions() -> Vec<Value> {
    main_capability()["permissions"]
        .as_array()
        .expect("la capability debe declarar permissions")
        .clone()
}

/// El estado del que partimos era `"csp": null`, que hace que Tauri no inyecte
/// ninguna política. `index.html` tampoco tiene el meta equivalente.
#[test]
fn csp_is_declared_and_not_null() {
    let policy = csp();
    assert!(
        !policy.trim().is_empty(),
        "la CSP no puede estar vacía: sin política el WebView queda sin barrera secundaria \
         frente a los tres puntos de innerHTML de la app"
    );
}

/// La concesión de `'unsafe-inline'` se aceptó SOLO en `style-src` (solid-toast inyecta
/// un `<style>` real). En `script-src` no se otorga, y esa es la directiva que importa.
#[test]
fn script_src_never_allows_inline_or_eval() {
    let policy = csp();
    let script_src = policy
        .split(';')
        .map(str::trim)
        .find(|d| d.starts_with("script-src"))
        .expect("la CSP debe declarar script-src explícitamente, no heredarlo de default-src");

    for prohibido in ["'unsafe-inline'", "'unsafe-eval'", "*"] {
        assert!(
            !script_src.contains(prohibido),
            "script-src no puede contener {prohibido}: quedó '{script_src}'"
        );
    }
}

/// Directivas que cierran vectores que la aplicación no usa. Son gratis y su ausencia
/// sólo se nota cuando ya es tarde.
#[test]
fn csp_closes_unused_vectors() {
    let policy = csp();
    for esperado in [
        "object-src 'none'",
        "base-uri 'self'",
        "frame-ancestors 'none'",
    ] {
        assert!(
            policy.contains(esperado),
            "falta la directiva '{esperado}' en la CSP"
        );
    }
}

/// El estado del que partimos otorgaba `/home/**` —el home de TODOS los usuarios de la
/// máquina, no el del usuario— en tres bloques de scope distintos.
#[test]
fn filesystem_scope_never_grants_every_users_home() {
    let serializado = serde_json::to_string(&permissions()).unwrap();
    assert!(
        !serializado.contains("/home/**"),
        "ningún scope puede otorgar /home/**: es el home de todos los usuarios de la máquina"
    );
}

/// `fs:default` arrastraba el set `fs:deny-default`, que deniega el data folder del
/// WebView. Al recortar la capability ese set se perdió sin querer, y en Windows
/// $APPLOCALDATA cae bajo $HOME/**: el WebView podía escribir ahí.
#[test]
fn filesystem_keeps_the_webview_data_folder_denied() {
    let permisos = permissions();
    let tiene_deny_default = permisos
        .iter()
        .any(|p| p.as_str() == Some("fs:deny-default"));

    assert!(
        tiene_deny_default,
        "falta fs:deny-default. Es un set de sólo denegación (no habilita ningún comando) \
         que protege $APPLOCALDATA. Sin él, en Windows el WebView puede escribir dentro \
         del data folder de WebView2, porque $APPLOCALDATA cae bajo el allow de $HOME/**."
    );
}

/// El WebView usa un único comando del plugin fs: `writeTextFile`, para el markdown de
/// backup. No lee, no lista, no chequea existencia. Cualquier permiso de lectura que
/// reaparezca acá es superficie que nadie pidió.
#[test]
fn webview_has_no_filesystem_read_permission() {
    let serializado = serde_json::to_string(&permissions()).unwrap();
    for prohibido in [
        "fs:allow-read",
        "fs:default",
        "fs:allow-exists",
        "fs:allow-read-text-file",
        "fs:allow-read-dir",
    ] {
        assert!(
            !serializado.contains(prohibido),
            "reapareció el permiso de lectura '{prohibido}': el WebView no lee del filesystem"
        );
    }
}

/// `plugin-shell` no se importa en ningún archivo del frontend. El permiso estaba
/// otorgado igual.
#[test]
fn shell_open_is_not_granted() {
    let serializado = serde_json::to_string(&permissions()).unwrap();
    assert!(
        !serializado.contains("shell:"),
        "no se otorga ningún permiso de shell: el frontend no importa plugin-shell, \
         y abrir terminal y URLs lo hacen comandos propios de Rust"
    );
}

/// `devtools: true` estaba declarado en la ventana. Aunque en release sea un no-op
/// (la llamada `with_devtools` ni se compila sin la feature `devtools`), declararlo
/// comunica una intención equivocada.
#[test]
fn window_does_not_force_devtools_on() {
    let ventana = config()["app"]["windows"][0].clone();
    assert!(
        ventana.get("devtools").is_none(),
        "la ventana no debe fijar 'devtools': el default de Tauri ya lo habilita en \
         debug y lo deshabilita en release"
    );
}
