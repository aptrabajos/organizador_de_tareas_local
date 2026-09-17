use rusqlite::{Result, Row};

use crate::models::project::Project;

use super::Database;

/// Las 23 columnas de `projects`, EN ORDEN.
///
/// Esta lista aparecía textualmente en siete métodos (`create_project`,
/// `get_all_projects`, `get_project`, `search_projects`, `get_recent_projects`,
/// `get_root_projects`, `get_subprojects`). Agregar una columna obligaba a tocar
/// los siete en sincronía, y olvidarse de uno no rompe la compilación: rompe en
/// runtime, en el índice posicional de [`row_to_project`], y solo en el camino
/// que se olvidó.
///
/// El orden ES el contrato: [`row_to_project`] lee por posición.
pub(super) const PROJECT_COLUMNS: &str = "id, name, description, local_path, documentation_url, ai_documentation_url, drive_link, notes, image_data,
                    created_at, updated_at, last_opened_at, opened_count, total_time_seconds,
                    status, status_changed_at, is_pinned, pinned_order, display_order,
                    parent_id, group_color, group_icon, is_group_expanded";

/// Construye un `Project` desde una fila que trae [`PROJECT_COLUMNS`] en ese orden.
///
/// `links` queda en `None` a propósito: los enlaces viven en otra tabla y quien
/// los necesite los completa después con [`Database::get_links_for_projects`].
/// Mezclarlos acá era justamente lo que producía el N+1.
pub(super) fn row_to_project(row: &Row) -> Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        local_path: row.get(3)?,
        documentation_url: row.get(4)?,
        ai_documentation_url: row.get(5)?,
        drive_link: row.get(6)?,
        notes: row.get(7)?,
        image_data: row.get(8)?,
        links: None,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        last_opened_at: row.get(11)?,
        opened_count: row.get(12)?,
        total_time_seconds: row.get(13)?,
        status: row.get(14)?,
        status_changed_at: row.get(15)?,
        is_pinned: row.get(16)?,
        pinned_order: row.get(17)?,
        display_order: row.get(18)?,
        parent_id: row.get(19)?,
        group_color: row.get(20)?,
        group_icon: row.get(21)?,
        is_group_expanded: row.get(22)?,
    })
}

impl Database {
    /// Normaliza un texto opcional para columnas nullable: vacío (tras trim) -> NULL.
    /// Mantiene la DB consistente (NULL en vez de cadena vacía) y permite vaciar campos.
    pub(super) fn empty_to_null(s: String) -> Option<String> {
        if s.trim().is_empty() {
            None
        } else {
            Some(s)
        }
    }
}
