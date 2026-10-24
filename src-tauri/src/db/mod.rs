use log::error;
use rusqlite::Connection;
use std::sync::Mutex;

mod activity;
mod attachments;
mod common;
mod dashboard;
mod groups;
mod journal;
mod links;
mod maintenance;
mod projects;
mod schema;
mod status;
mod tiempo;
mod todos;
mod trabajar;
mod tracker_status;
mod tracking;
mod trash;

#[cfg(test)]
mod test_support;

/// Único re-export necesario: el comando `list_trash` de `commands/mod.rs`
/// nombra este tipo por ruta (`crate::db::TrashItem`).
pub use trash::TrashItem;

/// `source` de los tramos que mide el demonio: lo nombra la regla de duración mínima
/// de `commands/mod.rs`.
pub(crate) use trabajar::FUENTE_TERMINAL;

/// La copia de lo que mide la app, para las estadísticas (`commands`, commit 21).
pub(crate) use tiempo::SesionActiva;

/// Lo que usan Trabajar, Parar y la tarjeta (`commands/trabajar.rs`, commit 17).
pub(crate) use trabajar::{
    Dueno, InicioTrabajar, LatidoApp, MedicionTrabajar, MedidoPorLaApp, MotivoSesion,
    PedidoTrabajar, ResultadoParar, SesionCortada, SesionViva,
};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Toma el lock de la conexión, recuperándose del envenenamiento.
    ///
    /// Había 45 `self.conn()` en el código de producción de este
    /// archivo, y el problema no era el `unwrap()` en abstracto: `Mutex::lock()`
    /// devuelve `Err` SOLO si el mutex quedó envenenado, o sea si otro hilo entró
    /// en panic mientras lo tenía tomado. Con `unwrap()`, ese primer panic
    /// convertía en panic TODAS las operaciones de base posteriores: un fallo
    /// aislado en un camino cualquiera dejaba la app muerta hasta el reinicio.
    ///
    /// La política es recuperar, no propagar. El envenenamiento es una marca que
    /// pone Rust, no una corrupción de SQLite: la conexión sigue siendo
    /// perfectamente válida, así que `into_inner()` devuelve el guard y la app
    /// sigue viva. Propagar un error tipado sería más "honesto" pero convertiría
    /// el panic ajeno en un error visible al usuario en cada operación siguiente,
    /// que es el mismo desastre con mejores modales.
    ///
    /// El `error!` no es opcional: si esto dispara, hubo un panic en algún lado y
    /// alguien tiene que poder encontrarlo.
    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|poisoned| {
            error!(
                "❌ [DB] El mutex de la conexión estaba envenenado (hubo un panic con el lock tomado). Se recupera y se sigue operando."
            );
            poisoned.into_inner()
        })
    }
}

/// Acceso crudo a la conexión para los tests de otros módulos (p. ej. `commands`), que
/// necesitan provocar errores de base (un trigger que aborta) sin API pública para eso.
#[cfg(test)]
impl Database {
    pub(crate) fn execute_batch_para_tests(&self, sql: &str) {
        self.conn()
            .execute_batch(sql)
            .expect("no se pudo ejecutar el SQL del test");
    }

    /// Lee una fila con la conexión cruda, para las aserciones de los tests de otros
    /// módulos (el estado de una sesión cerrada, el motivo de un tramo).
    pub(crate) fn fila_para_tests<T>(
        &self,
        sql: &str,
        params: impl rusqlite::Params,
        fila: impl FnOnce(&rusqlite::Row) -> rusqlite::Result<T>,
    ) -> T {
        self.conn()
            .query_row(sql, params, fila)
            .expect("no se pudo leer la fila del test")
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;

    // ============ B15: el envenenamiento del mutex no mata la app ============

    #[test]
    fn la_base_sigue_operando_despues_de_un_panic_con_el_lock_tomado() {
        let db = test_db();
        seed_project_at(&db, "Antes del panic", "/tmp/antes");

        // Envenenar el mutex a mano: un hilo toma el lock y entra en panic. Es
        // exactamente lo que pasa cuando cualquier camino de la app panickea con
        // la conexión tomada, y es la ÚNICA forma de que `Mutex::lock()` devuelva
        // Err.
        let panicked = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let _guard = db.conn.lock().expect("el mutex arranca sano");
                    panic!("panic deliberado con el lock de la conexión tomado");
                })
                .join()
        });
        assert!(panicked.is_err(), "el hilo tenía que entrar en panic");

        // Con `lock().unwrap()` esta línea era un panic, y con ella TODA operación
        // de base posterior: un fallo aislado en un camino cualquiera dejaba la
        // app muerta hasta el reinicio. Este test fallaba con el código anterior.
        let projects = db
            .get_all_projects()
            .expect("la base tiene que seguir operando después del envenenamiento");
        assert_eq!(projects.len(), 1);

        // Y no es solo leer: escribir también.
        seed_project_at(&db, "Después del panic", "/tmp/despues");
        assert_eq!(db.get_all_projects().unwrap().len(), 2);
    }
}
