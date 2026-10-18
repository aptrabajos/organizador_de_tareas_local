use rusqlite::Result;

use super::common::{row_to_project, PROJECT_COLUMNS};
use crate::models::project::Project;

use super::Database;

impl Database {
    // ==================== DASHBOARD METHODS ====================

    pub fn get_recent_projects(&self) -> Result<Vec<Project>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            &format!("SELECT {}
             FROM projects
             WHERE last_opened_at IS NOT NULL AND deleted_at IS NULL
             ORDER BY last_opened_at DESC
             LIMIT 5", PROJECT_COLUMNS)
        )?;

        // `links` queda en None, igual que antes: el dashboard no los muestra.
        let projects_iter = stmt.query_map([], row_to_project)?;

        let mut projects = Vec::new();
        for project in projects_iter {
            projects.push(project?);
        }
        Ok(projects)
    }

    pub fn get_all_pending_todos(&self) -> Result<Vec<crate::models::project::DashboardTodo>> {
        use crate::models::project::{DashboardTodo, ProjectTodo};
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT
                pt.id, pt.project_id, pt.content, pt.is_completed, pt.created_at, pt.completed_at,
                p.name as project_name
             FROM project_todos pt
             JOIN projects p ON pt.project_id = p.id
             WHERE pt.is_completed = 0 AND p.deleted_at IS NULL
             ORDER BY pt.created_at DESC"
        )?;

        let todos_iter = stmt.query_map([], |row| {
            Ok(DashboardTodo {
                todo: ProjectTodo {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    content: row.get(2)?,
                    is_completed: row.get(3)?,
                    created_at: row.get(4)?,
                    completed_at: row.get(5)?,
                },
                project_name: row.get(6)?,
            })
        })?;

        let mut todos = Vec::new();
        for todo in todos_iter {
            todos.push(todo?);
        }
        Ok(todos)
    }

    pub fn get_recent_journal_entries(&self) -> Result<Vec<crate::models::project::DashboardJournalEntry>> {
        use crate::models::project::{DashboardJournalEntry, JournalEntry};
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT
                pj.id, pj.project_id, pj.content, pj.tags, pj.created_at, pj.updated_at,
                p.name as project_name
             FROM project_journal pj
             JOIN projects p ON pj.project_id = p.id
             WHERE p.deleted_at IS NULL
             ORDER BY pj.created_at DESC
             LIMIT 5"
        )?;

        let entries_iter = stmt.query_map([], |row| {
            Ok(DashboardJournalEntry {
                entry: JournalEntry {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    content: row.get(2)?,
                    tags: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                },
                project_name: row.get(6)?,
            })
        })?;

        let mut entries = Vec::new();
        for entry in entries_iter {
            entries.push(entry?);
        }
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use crate::models::project::{CreateTodoDTO, UpdateTodoDTO};

    // --- Dashboard ---

    #[test]
    fn test_get_recent_projects() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        // Before tracking, no recent projects
        let recent = db.get_recent_projects().unwrap();
        assert!(recent.is_empty());
        // Track open
        db.track_project_open(p.id).unwrap();
        let recent2 = db.get_recent_projects().unwrap();
        assert_eq!(recent2.len(), 1);
        assert_eq!(recent2[0].name, "Test Project");
    }

    #[test]
    fn test_get_all_pending_todos() {
        let db = test_db();
        let p = db.create_project(test_project_dto()).unwrap();
        db.create_todo(CreateTodoDTO {
            project_id: p.id,
            content: "Pending task".to_string(),
        })
        .unwrap();
        let completed = db
            .create_todo(CreateTodoDTO {
                project_id: p.id,
                content: "Done task".to_string(),
            })
            .unwrap();
        db.update_todo(
            completed.id,
            UpdateTodoDTO {
                content: None,
                is_completed: Some(true),
            },
        )
        .unwrap();
        let pending = db.get_all_pending_todos().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].todo.content, "Pending task");
        assert_eq!(pending[0].project_name, "Test Project");
    }
}
