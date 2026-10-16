use rusqlite::{params, Connection, Result};

use super::Database;

impl Database {
    /// Crea un backup transaccionalmente consistente de la base de datos.
    ///
    /// Usa `VACUUM INTO` a través del mismo Mutex que serializa las escrituras,
    /// por lo que la copia queda en un punto consistente sin escrituras a medias.
    /// `dest` debe ser una ruta de archivo que NO exista: `VACUUM INTO` falla si
    /// el archivo destino ya existe.
    pub fn backup_to(&self, dest: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute("VACUUM INTO ?1", params![dest])?;
        Ok(())
    }

    /// Verifica una COPIA de la base de datos (nunca la DB viva).
    ///
    /// Abre el archivo en modo READ_ONLY y corre `PRAGMA integrity_check` más un
    /// conteo de proyectos. Devuelve `(integridad_ok, cantidad_de_proyectos)`.
    pub fn verify_db_file(path: &str) -> Result<(bool, i64)> {
        use rusqlite::OpenFlags;
        let c = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let res: String = c.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        let count: i64 = c.query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))?;
        Ok((res == "ok", count))
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use crate::db::Database;

    #[test]
    fn test_backup_to_creates_verifiable_copy() {
        // El backup (VACUUM INTO) debe producir una copia válida y verificable.
        let db = test_db();
        db.create_project(test_project_dto()).unwrap();
        let dest = std::env::temp_dir().join("gp-test-backup-verifiable.db");
        let _ = std::fs::remove_file(&dest); // VACUUM INTO falla si el destino existe
        let dest_str = dest.to_str().unwrap();

        db.backup_to(dest_str).unwrap();
        assert!(dest.exists(), "el archivo de backup debe existir");

        let (ok, count) = Database::verify_db_file(dest_str).unwrap();
        assert!(ok, "integrity_check de la copia debe ser ok");
        assert_eq!(count, 1, "la copia debe contener el proyecto creado");

        let _ = std::fs::remove_file(&dest);
    }

    #[test]
    fn test_verify_db_file_rejects_non_db() {
        // Un archivo que NO es SQLite no debe pasar la verificación (debe dar Err),
        // así run_backup nunca deja un "backup" falso en disco.
        let dest = std::env::temp_dir().join("gp-test-notdb.txt");
        std::fs::write(&dest, b"esto no es una base de datos sqlite").unwrap();
        let res = Database::verify_db_file(dest.to_str().unwrap());
        assert!(res.is_err(), "un archivo no-SQLite no debe verificar OK");
        let _ = std::fs::remove_file(&dest);
    }
}
