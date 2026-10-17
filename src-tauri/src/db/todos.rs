use rusqlite::{params, Result};

use crate::models::project::{CreateTodoDTO, ProjectTodo, UpdateTodoDTO};

use super::Database;

impl Database {
    // ==================== MÉTODOS PARA PROJECT TODOS ====================

    pub fn create_todo(&self, todo: CreateTodoDTO) -> Result<ProjectTodo> {
        let conn = self.conn();

        conn.execute(
            "INSERT INTO project_todos (project_id, content)
             VALUES (?1, ?2)",
            params![todo.project_id, todo.content],
        )?;

        let id = conn.last_insert_rowid();

        let todo = conn.query_row(
            "SELECT id, project_id, content, is_completed, created_at, completed_at
             FROM project_todos WHERE id = ?1",
            params![id],
            |row| {
                Ok(ProjectTodo {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    content: row.get(2)?,
                    is_completed: row.get(3)?,
                    created_at: row.get(4)?,
                    completed_at: row.get(5)?,
                })
            },
        )?;

        Ok(todo)
    }

    pub fn get_project_todos(&self, project_id: i64) -> Result<Vec<ProjectTodo>> {
        let conn = self.conn();

        let mut stmt = conn.prepare(
            "SELECT id, project_id, content, is_completed, created_at, completed_at
             FROM project_todos
             WHERE project_id = ?1
             ORDER BY is_completed ASC, created_at DESC",
        )?;

        let todos = stmt
            .query_map(params![project_id], |row| {
                Ok(ProjectTodo {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    content: row.get(2)?,
                    is_completed: row.get(3)?,
                    created_at: row.get(4)?,
                    completed_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(todos)
    }

    pub fn update_todo(&self, id: i64, updates: UpdateTodoDTO) -> Result<ProjectTodo> {
        let conn = self.conn();

        let mut set_clauses = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(content) = updates.content {
            set_clauses.push("content = ?");
            params.push(Box::new(content));
        }

        if let Some(is_completed) = updates.is_completed {
            set_clauses.push("is_completed = ?");
            params.push(Box::new(is_completed));

            // Si se completó, actualizar completed_at
            if is_completed {
                set_clauses.push("completed_at = CURRENT_TIMESTAMP");
            } else {
                set_clauses.push("completed_at = NULL");
            }
        }

        if set_clauses.is_empty() {
            return Err(rusqlite::Error::InvalidParameterCount(0, 1));
        }

        params.push(Box::new(id));

        let query = format!(
            "UPDATE project_todos SET {} WHERE id = ?",
            set_clauses.join(", ")
        );

        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&query, params_ref.as_slice())?;

        // Obtener el TODO actualizado
        let todo = conn.query_row(
            "SELECT id, project_id, content, is_completed, created_at, completed_at
             FROM project_todos WHERE id = ?1",
            params![id],
            |row| {
                Ok(ProjectTodo {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    content: row.get(2)?,
                    is_completed: row.get(3)?,
                    created_at: row.get(4)?,
                    completed_at: row.get(5)?,
                })
            },
        )?;

        Ok(todo)
    }

    pub fn delete_todo(&self, id: i64) -> Result<()> {
        let conn = self.conn();

        conn.execute("DELETE FROM project_todos WHERE id = ?1", params![id])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use crate::models::project::{CreateTodoDTO, UpdateTodoDTO};

    // --- Todos ---

    #[test]
    fn test_create_todo() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let todo = db
            .create_todo(CreateTodoDTO {
                project_id: project.id,
                content: "Do something".to_string(),
            })
            .unwrap();
        assert!(todo.id > 0);
        assert!(!todo.is_completed);
        assert!(todo.completed_at.is_none());
    }

    #[test]
    fn test_complete_todo() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let todo = db
            .create_todo(CreateTodoDTO {
                project_id: project.id,
                content: "Task".to_string(),
            })
            .unwrap();
        let completed = db
            .update_todo(
                todo.id,
                UpdateTodoDTO {
                    content: None,
                    is_completed: Some(true),
                },
            )
            .unwrap();
        assert!(completed.is_completed);
        assert!(completed.completed_at.is_some());
    }

    #[test]
    fn test_uncomplete_todo() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let todo = db
            .create_todo(CreateTodoDTO {
                project_id: project.id,
                content: "Task".to_string(),
            })
            .unwrap();
        db.update_todo(
            todo.id,
            UpdateTodoDTO {
                content: None,
                is_completed: Some(true),
            },
        )
        .unwrap();
        let uncompleted = db
            .update_todo(
                todo.id,
                UpdateTodoDTO {
                    content: None,
                    is_completed: Some(false),
                },
            )
            .unwrap();
        assert!(!uncompleted.is_completed);
        assert!(uncompleted.completed_at.is_none());
    }

    #[test]
    fn test_get_project_todos_ordering() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let t1 = db
            .create_todo(CreateTodoDTO {
                project_id: project.id,
                content: "Task A".to_string(),
            })
            .unwrap();
        db.create_todo(CreateTodoDTO {
            project_id: project.id,
            content: "Task B".to_string(),
        })
        .unwrap();
        // Complete Task A
        db.update_todo(
            t1.id,
            UpdateTodoDTO {
                content: None,
                is_completed: Some(true),
            },
        )
        .unwrap();
        let todos = db.get_project_todos(project.id).unwrap();
        assert_eq!(todos.len(), 2);
        // Incomplete first
        assert!(!todos[0].is_completed);
        assert!(todos[1].is_completed);
    }

    #[test]
    fn test_delete_todo() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let todo = db
            .create_todo(CreateTodoDTO {
                project_id: project.id,
                content: "To delete".to_string(),
            })
            .unwrap();
        db.delete_todo(todo.id).unwrap();
        let todos = db.get_project_todos(project.id).unwrap();
        assert!(todos.is_empty());
    }
}
