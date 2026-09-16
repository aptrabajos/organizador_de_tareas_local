//! Guards de autorización para los comandos Tauri.
//!
//! Primer submódulo de `commands/` y existe por un motivo concreto, no por prolijidad:
//! los 11 comandos git reciben un `path` del frontend y se lo pasan a
//! `git -C <path>`. El riesgo NO es inyección de comandos (los argumentos viajan como
//! argv real, nunca por un shell), es de AUTORIZACIÓN: sin este guard la app opera git
//! —incluyendo `add`, `commit`, `push`, `pull` y el `fetch` de `get_git_ahead_behind`,
//! que genera tráfico de red— sobre cualquier repositorio del disco, gestionado o no.

use crate::db::Database;

/// Exige que `path` sea la RAÍZ de un proyecto registrado y activo (no en papelera).
///
/// Compara en forma canónica de los DOS lados (`std::fs::canonicalize`) para que una
/// barra final, un `./` intermedio o un symlink no produzcan un falso negativo que
/// bloquee a un usuario legítimo.
///
/// MATCH EXACTO DE LA RAÍZ, no de subdirectorios: hoy todos los consumidores
/// (`EnhancedGitInfo`, `GitInfo`, `GitCommitModal`) reciben `props.projectPath`, que
/// siempre sale de `project.local_path`, o sea la raíz exacta. Es el criterio más
/// estricto y alcanza. Si en el futuro hiciera falta operar en subdirectorios, la
/// relajación es `path == registrado || path.starts_with(registrado + separador)`
/// (el separador es imprescindible: sin él `/home/u/proyecto-ajeno` pasaría el
/// chequeo de `/home/u/proyecto`).
pub fn assert_registered_project_path(db: &Database, path: &str) -> Result<(), String> {
    // Canonicalizar el path recibido. Si falla (no existe, no es accesible) es RECHAZO,
    // nunca un pase: un path que no se puede resolver no se puede autorizar.
    let canonical_input = std::fs::canonicalize(path).map_err(|e| {
        format!(
            "No se puede operar git sobre '{}': la carpeta no existe o no es accesible ({}).",
            path, e
        )
    })?;

    // Camino rápido: coincidencia textual exacta con un proyecto activo. Es el caso
    // normal, porque el frontend manda `project.local_path` tal cual está guardado.
    // Ya sabemos que la carpeta existe: el canonicalize de arriba pasó.
    if db
        .is_registered_project_path(path)
        .map_err(|e| format!("Error consultando proyectos registrados: {}", e))?
    {
        return Ok(());
    }

    // Camino lento: comparar en forma canónica contra cada proyecto activo, por si la
    // ruta guardada y la recibida apuntan al mismo directorio real con distinta forma.
    let registered = db
        .active_project_paths()
        .map_err(|e| format!("Error consultando proyectos registrados: {}", e))?;

    let autorizado = registered.iter().any(|candidate| {
        // CASO BORDE: si el path REGISTRADO no canonicaliza (el usuario borró la carpeta
        // del disco pero el proyecto sigue en la DB), se SALTEA en vez de comparar el
        // string crudo. Es fail-closed y no pierde nada: el input ya canonicalizó bien,
        // así que existe; un registro que no existe en disco no puede ser ese mismo
        // directorio real. Y si input y registrado fueran el mismo string, el camino
        // rápido de arriba ya habría devuelto Ok.
        std::fs::canonicalize(candidate)
            .map(|canonical_candidate| canonical_candidate == canonical_input)
            .unwrap_or(false)
    });

    if autorizado {
        Ok(())
    } else {
        Err(format!(
            "Operación git no permitida sobre '{}': no corresponde a ningún proyecto \
             registrado. Agregá esa carpeta como proyecto en el gestor (o restauralo \
             desde la papelera si lo borraste) y volvé a intentar.",
            path
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::assert_registered_project_path;
    use crate::db::Database;
    use crate::models::project::CreateProjectDTO;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn test_db() -> Database {
        Database::new(PathBuf::from(":memory:")).expect("no se pudo crear la DB en memoria")
    }

    fn seed_project_at(db: &Database, local_path: &str) -> i64 {
        db.create_project(CreateProjectDTO {
            name: "Proyecto".to_string(),
            description: "desc".to_string(),
            local_path: local_path.to_string(),
            documentation_url: None,
            ai_documentation_url: None,
            drive_link: None,
            notes: None,
            image_data: None,
            parent_id: None,
            group_color: None,
            group_icon: None,
        })
        .expect("no se pudo sembrar el proyecto")
        .id
    }

    #[test]
    fn registered_and_existing_path_is_allowed() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap().to_string();
        let db = test_db();
        seed_project_at(&db, &path);

        assert!(assert_registered_project_path(&db, &path).is_ok());
    }

    #[test]
    fn unregistered_path_is_rejected_with_spanish_message() {
        let dir = TempDir::new().unwrap();
        let ajeno = TempDir::new().unwrap();
        let db = test_db();
        seed_project_at(&db, dir.path().to_str().unwrap());

        let err = assert_registered_project_path(&db, ajeno.path().to_str().unwrap())
            .expect_err("una carpeta ajena no debe autorizarse");
        assert!(
            err.contains("no corresponde a ningún proyecto registrado"),
            "el mensaje debe ser en español y accionable: {err}"
        );
    }

    /// Un proyecto en papelera NO habilita git: borrar revoca el permiso.
    #[test]
    fn trashed_project_path_is_rejected() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap().to_string();
        let db = test_db();
        let id = seed_project_at(&db, &path);

        assert!(assert_registered_project_path(&db, &path).is_ok());

        db.delete_project(id).expect("no se pudo borrar");
        assert!(
            assert_registered_project_path(&db, &path).is_err(),
            "un proyecto en papelera no debe autorizar operaciones git"
        );
    }

    /// `canonicalize` del input falla ⇒ rechazo, nunca un pase. Ojo: la ruta ESTÁ
    /// registrada en la DB, así que el camino rápido la dejaría pasar si el chequeo de
    /// existencia no fuera lo PRIMERO que corre.
    #[test]
    fn registered_path_missing_from_disk_is_rejected() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("subcarpeta-que-no-existe");
        let path_str = path.to_str().unwrap().to_string();
        let db = test_db();
        seed_project_at(&db, &path_str);

        let err = assert_registered_project_path(&db, &path_str)
            .expect_err("un path inexistente no debe autorizarse aunque esté registrado");
        assert!(
            err.contains("no existe o no es accesible"),
            "debe explicar que la carpeta no está en disco: {err}"
        );
    }

    /// Caso borde elegido: el REGISTRADO no existe en disco y el input sí. Se saltea
    /// el registro roto (fail-closed) en vez de caer a comparar strings crudos.
    #[test]
    fn registered_path_gone_from_disk_authorizes_nothing_else() {
        let existente = TempDir::new().unwrap();
        let fantasma = TempDir::new().unwrap();
        let fantasma_path = fantasma.path().to_str().unwrap().to_string();
        let db = test_db();
        seed_project_at(&db, &fantasma_path);
        drop(fantasma); // la carpeta registrada desaparece del disco

        assert!(
            assert_registered_project_path(&db, existente.path().to_str().unwrap()).is_err(),
            "un registro sin carpeta en disco no puede autorizar otra carpeta"
        );
    }

    /// Barra final y './' intermedio apuntan al mismo directorio real: no deben dar
    /// falso negativo contra la ruta canónica guardada.
    #[test]
    fn trailing_slash_and_dot_segment_resolve_to_the_same_project() {
        let dir = TempDir::new().unwrap();
        let canonical = std::fs::canonicalize(dir.path()).unwrap();
        let db = test_db();
        seed_project_at(&db, canonical.to_str().unwrap());

        let con_barra = format!("{}/", canonical.to_str().unwrap());
        assert!(
            assert_registered_project_path(&db, &con_barra).is_ok(),
            "la barra final no debe bloquear a un proyecto legítimo"
        );

        let con_punto = canonical.join(".");
        assert!(
            assert_registered_project_path(&db, con_punto.to_str().unwrap()).is_ok(),
            "un './' intermedio no debe bloquear a un proyecto legítimo"
        );
    }
}
