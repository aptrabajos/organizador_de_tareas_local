// `restore.lock`: mientras dura un restore, nadie crea sesiones de Trabajar (3.10.7
// del diseño del tracker, DT15).
//
// El restore lo toma **exclusivo** desde que se confirma el diálogo hasta que la base
// restaurada quedó migrada (commit 27). `trabajar` y el subcomando de la CLI toman un
// lock **compartido y no bloqueante** mientras dura su transacción: si está tomado,
// rechazan en vez de esperar. Es un `flock` (lo que usa `File::try_lock_shared` en
// Linux), que choca también entre dos descriptores del mismo proceso, así que la app
// y la CLI usan el mismo mecanismo.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;

/// Nombre del archivo, dentro del directorio de datos (junto a `projects.db`).
pub const NOMBRE: &str = "restore.lock";

/// Lo que ve el usuario si toca Trabajar durante un restore: puede esperar.
pub const MENSAJE_RESTAURACION_EN_CURSO: &str =
    "Hay una restauración en curso: esperá a que la app se reinicie.";

/// Lock compartido sobre `restore.lock`. Se suelta al soltar el valor (cerrar el
/// descriptor suelta el `flock`).
#[derive(Debug)]
pub struct LockRestoreCompartido {
    _archivo: File,
}

/// Toma el lock compartido de `<data_dir>/restore.lock`, creando el archivo si falta.
///
/// Si el restore lo tiene exclusivo, devuelve [`MENSAJE_RESTAURACION_EN_CURSO`] sin
/// esperar. Un fallo al abrir o al tomar el lock es de sistema: va en inglés.
pub fn tomar_compartido(data_dir: &Path) -> Result<LockRestoreCompartido, String> {
    let ruta = data_dir.join(NOMBRE);
    let archivo = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&ruta)
        .map_err(|e| format!("Failed to open {}: {}", ruta.display(), e))?;

    match archivo.try_lock_shared() {
        Ok(()) => Ok(LockRestoreCompartido { _archivo: archivo }),
        Err(TryLockError::WouldBlock) => Err(MENSAJE_RESTAURACION_EN_CURSO.to_string()),
        Err(TryLockError::Error(e)) => Err(format!("Failed to lock {}: {}", ruta.display(), e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// El lock exclusivo que tomará el restore (commit 27), con otro descriptor.
    fn tomar_exclusivo(dir: &Path) -> File {
        let archivo = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join(NOMBRE))
            .unwrap();
        archivo
            .try_lock()
            .expect("el lock exclusivo tiene que estar libre");
        archivo
    }

    #[test]
    fn el_lock_compartido_crea_el_archivo_y_admite_varios_a_la_vez() {
        let dir = TempDir::new().unwrap();

        let primero = tomar_compartido(dir.path()).expect("sin restore, se toma");
        let segundo = tomar_compartido(dir.path()).expect("dos Trabajar a la vez no chocan");

        assert!(dir.path().join(NOMBRE).exists());
        drop((primero, segundo));
    }

    /// Con el restore en curso se rechaza en español y sin esperar; cuando el restore
    /// lo suelta, se vuelve a tomar.
    #[test]
    fn con_el_restore_en_curso_se_rechaza_y_al_soltarlo_se_toma() {
        let dir = TempDir::new().unwrap();
        let restore = tomar_exclusivo(dir.path());

        let err = tomar_compartido(dir.path()).expect_err("con el restore en curso no se toma");
        assert_eq!(err, MENSAJE_RESTAURACION_EN_CURSO);

        drop(restore);
        tomar_compartido(dir.path()).expect("sin restore, se vuelve a tomar");
    }

    /// Y al revés: mientras alguien tiene el compartido, el restore no lo toma.
    #[test]
    fn mientras_se_crea_una_sesion_el_restore_no_toma_el_lock() {
        let dir = TempDir::new().unwrap();
        let sesion = tomar_compartido(dir.path()).unwrap();

        let otro = File::open(dir.path().join(NOMBRE)).unwrap();
        assert!(matches!(otro.try_lock(), Err(TryLockError::WouldBlock)));

        drop(sesion);
        otro.try_lock()
            .expect("sin sesiones creándose, el restore lo toma");
    }

    #[test]
    fn un_directorio_inexistente_es_un_error_de_sistema_en_ingles() {
        let dir = TempDir::new().unwrap();
        let err = tomar_compartido(&dir.path().join("no_existe")).unwrap_err();
        assert!(err.starts_with("Failed to open"), "{err}");
    }
}
