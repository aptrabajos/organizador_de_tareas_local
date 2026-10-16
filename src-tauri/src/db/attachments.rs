use rusqlite::{params, Result};

use crate::models::project::{CreateAttachmentDTO, ProjectAttachment};

use super::Database;

impl Database {
    // ==================== MÉTODOS PARA ARCHIVOS ADJUNTOS ====================

    pub fn add_attachment(&self, attachment: CreateAttachmentDTO) -> Result<ProjectAttachment> {
        const MAX_FILE_SIZE: i64 = 5 * 1024 * 1024; // 5MB en bytes

        // Validar tamaño
        if attachment.file_size > MAX_FILE_SIZE {
            return Err(rusqlite::Error::InvalidParameterName(
                "El archivo supera el límite de 5 MB. Elegí uno más chico.".to_string(),
            ));
        }

        let conn = self.conn();

        conn.execute(
            "INSERT INTO project_attachments (project_id, filename, file_data, file_size, mime_type)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                attachment.project_id,
                attachment.filename,
                attachment.file_data,
                attachment.file_size,
                attachment.mime_type
            ],
        )?;

        let id = conn.last_insert_rowid();

        let attachment = conn.query_row(
            "SELECT id, project_id, filename, file_data, file_size, mime_type, created_at
             FROM project_attachments WHERE id = ?1",
            params![id],
            |row| {
                Ok(ProjectAttachment {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    filename: row.get(2)?,
                    file_data: row.get(3)?,
                    file_size: row.get(4)?,
                    mime_type: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        )?;

        Ok(attachment)
    }

    pub fn get_attachments(&self, project_id: i64) -> Result<Vec<ProjectAttachment>> {
        let conn = self.conn();

        let mut stmt = conn.prepare(
            "SELECT id, project_id, filename, file_data, file_size, mime_type, created_at
             FROM project_attachments WHERE project_id = ?1 ORDER BY created_at DESC",
        )?;

        let attachments = stmt
            .query_map(params![project_id], |row| {
                Ok(ProjectAttachment {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    filename: row.get(2)?,
                    file_data: row.get(3)?,
                    file_size: row.get(4)?,
                    mime_type: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(attachments)
    }

    pub fn delete_attachment(&self, id: i64) -> Result<()> {
        let conn = self.conn();

        conn.execute("DELETE FROM project_attachments WHERE id = ?1", params![id])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use crate::models::project::CreateAttachmentDTO;

    // --- Attachments ---

    #[test]
    fn test_add_attachment() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let att = db
            .add_attachment(CreateAttachmentDTO {
                project_id: project.id,
                filename: "test.txt".to_string(),
                file_data: "aGVsbG8=".to_string(),
                file_size: 5,
                mime_type: "text/plain".to_string(),
            })
            .unwrap();
        assert!(att.id > 0);
        assert_eq!(att.filename, "test.txt");
        assert_eq!(att.file_size, 5);
    }

    #[test]
    fn test_attachment_size_limit() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let result = db.add_attachment(CreateAttachmentDTO {
            project_id: project.id,
            filename: "big.bin".to_string(),
            file_data: "data".to_string(),
            file_size: 6 * 1024 * 1024, // 6MB > 5MB limit
            mime_type: "application/octet-stream".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_get_attachments() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        db.add_attachment(CreateAttachmentDTO {
            project_id: project.id,
            filename: "a.txt".to_string(),
            file_data: "YQ==".to_string(),
            file_size: 1,
            mime_type: "text/plain".to_string(),
        })
        .unwrap();
        db.add_attachment(CreateAttachmentDTO {
            project_id: project.id,
            filename: "b.txt".to_string(),
            file_data: "Yg==".to_string(),
            file_size: 1,
            mime_type: "text/plain".to_string(),
        })
        .unwrap();
        let attachments = db.get_attachments(project.id).unwrap();
        assert_eq!(attachments.len(), 2);
    }

    #[test]
    fn test_delete_attachment() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let att = db
            .add_attachment(CreateAttachmentDTO {
                project_id: project.id,
                filename: "del.txt".to_string(),
                file_data: "ZA==".to_string(),
                file_size: 1,
                mime_type: "text/plain".to_string(),
            })
            .unwrap();
        db.delete_attachment(att.id).unwrap();
        let attachments = db.get_attachments(project.id).unwrap();
        assert!(attachments.is_empty());
    }

    #[test]
    fn test_attachment_at_exact_limit() {
        let db = test_db();
        let project = db.create_project(test_project_dto()).unwrap();
        let result = db.add_attachment(CreateAttachmentDTO {
            project_id: project.id,
            filename: "exact.bin".to_string(),
            file_data: "data".to_string(),
            file_size: 5 * 1024 * 1024, // Exactly 5MB
            mime_type: "application/octet-stream".to_string(),
        });
        assert!(result.is_ok());
    }
}
