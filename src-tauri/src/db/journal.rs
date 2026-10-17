use rusqlite::{params, Result};

use crate::models::project::{CreateJournalEntryDTO, JournalEntry, UpdateJournalEntryDTO};

use super::Database;

impl Database {
    // ==================== MÉTODOS PARA PROJECT JOURNAL ====================

    pub fn create_journal_entry(&self, entry: CreateJournalEntryDTO) -> Result<JournalEntry> {
        let conn = self.conn();

        conn.execute(
            "INSERT INTO project_journal (project_id, content, tags)
             VALUES (?1, ?2, ?3)",
            params![entry.project_id, entry.content, entry.tags.and_then(Self::empty_to_null)],
        )?;

        let id = conn.last_insert_rowid();

        let journal_entry = conn.query_row(
            "SELECT id, project_id, content, tags, created_at, updated_at
             FROM project_journal WHERE id = ?1",
            params![id],
            |row| {
                Ok(JournalEntry {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    content: row.get(2)?,
                    tags: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )?;

        Ok(journal_entry)
    }

    pub fn get_journal_entries(&self, project_id: i64) -> Result<Vec<JournalEntry>> {
        let conn = self.conn();

        let mut stmt = conn.prepare(
            "SELECT id, project_id, content, tags, created_at, updated_at
             FROM project_journal
             WHERE project_id = ?1
             ORDER BY created_at DESC",
        )?;

        let entries = stmt
            .query_map(params![project_id], |row| {
                Ok(JournalEntry {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    content: row.get(2)?,
                    tags: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(entries)
    }

    pub fn update_journal_entry(&self, id: i64, updates: UpdateJournalEntryDTO) -> Result<JournalEntry> {
        let conn = self.conn();

        let mut set_clauses = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(content) = updates.content {
            set_clauses.push("content = ?");
            params.push(Box::new(content));
        }

        if let Some(tags) = updates.tags {
            set_clauses.push("tags = ?");
            params.push(Box::new(Self::empty_to_null(tags)));
        }

        if set_clauses.is_empty() {
            return Err(rusqlite::Error::InvalidParameterCount(0, 1));
        }

        set_clauses.push("updated_at = CURRENT_TIMESTAMP");
        params.push(Box::new(id));

        let query = format!(
            "UPDATE project_journal SET {} WHERE id = ?",
            set_clauses.join(", ")
        );

        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&query, params_ref.as_slice())?;

        // Obtener la entrada actualizada
        let entry = conn.query_row(
            "SELECT id, project_id, content, tags, created_at, updated_at
             FROM project_journal WHERE id = ?1",
            params![id],
            |row| {
                Ok(JournalEntry {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    content: row.get(2)?,
                    tags: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )?;

        Ok(entry)
    }

    pub fn delete_journal_entry(&self, id: i64) -> Result<()> {
        let conn = self.conn();

        conn.execute("DELETE FROM project_journal WHERE id = ?1", params![id])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use crate::models::project::{CreateJournalEntryDTO, UpdateJournalEntryDTO};

    // --- Journal ---

    #[test]
    fn test_create_journal_entry() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let entry = db
            .create_journal_entry(CreateJournalEntryDTO {
                project_id: project.id,
                content: "First entry".to_string(),
                tags: Some("[\"bug\",\"tip\"]".to_string()),
            })
            .unwrap();
        assert!(entry.id > 0);
        assert_eq!(entry.content, "First entry");
        assert_eq!(entry.tags.as_deref(), Some("[\"bug\",\"tip\"]"));
    }

    #[test]
    fn test_get_journal_entries() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        db.create_journal_entry(CreateJournalEntryDTO {
            project_id: project.id,
            content: "Entry 1".to_string(),
            tags: None,
        })
        .unwrap();
        db.create_journal_entry(CreateJournalEntryDTO {
            project_id: project.id,
            content: "Entry 2".to_string(),
            tags: None,
        })
        .unwrap();
        let entries = db.get_journal_entries(project.id).unwrap();
        assert_eq!(entries.len(), 2);
        // Both entries present (order may vary when created in same second)
        let contents: Vec<&str> = entries.iter().map(|e| e.content.as_str()).collect();
        assert!(contents.contains(&"Entry 1"));
        assert!(contents.contains(&"Entry 2"));
    }

    #[test]
    fn test_update_journal_entry() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let entry = db
            .create_journal_entry(CreateJournalEntryDTO {
                project_id: project.id,
                content: "Original".to_string(),
                tags: None,
            })
            .unwrap();
        let updated = db
            .update_journal_entry(
                entry.id,
                UpdateJournalEntryDTO {
                    content: Some("Updated".to_string()),
                    tags: None,
                },
            )
            .unwrap();
        assert_eq!(updated.content, "Updated");
    }

    #[test]
    fn test_delete_journal_entry() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let entry = db
            .create_journal_entry(CreateJournalEntryDTO {
                project_id: project.id,
                content: "To delete".to_string(),
                tags: None,
            })
            .unwrap();
        db.delete_journal_entry(entry.id).unwrap();
        let entries = db.get_journal_entries(project.id).unwrap();
        assert!(entries.is_empty());
    }
}
