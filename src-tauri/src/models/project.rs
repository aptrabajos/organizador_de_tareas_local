use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectLink {
    pub id: i64,
    pub project_id: i64,
    pub link_type: String,
    pub title: String,
    pub url: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub local_path: String,
    pub documentation_url: Option<String>,
    pub ai_documentation_url: Option<String>,
    pub drive_link: Option<String>,
    pub notes: Option<String>,
    pub image_data: Option<String>,
    pub links: Option<Vec<ProjectLink>>,
    pub created_at: String,
    pub updated_at: String,
    // Analytics fields
    pub last_opened_at: Option<String>,
    pub opened_count: Option<i64>,
    pub total_time_seconds: Option<i64>,
    // Quick Start & Context fields
    pub status: Option<String>, // activo, pausado, completado, archivado
    pub status_changed_at: Option<String>,
    pub is_pinned: Option<bool>,
    pub pinned_order: Option<i64>,
    pub display_order: Option<i64>, // Orden personalizado para drag & drop
    // Group/Hierarchy fields (v0.4.0)
    pub parent_id: Option<i64>, // NULL = grupo raíz, INT = subproyecto
    pub group_color: Option<String>, // Color hex para identificación visual
    pub group_icon: Option<String>, // Emoji o nombre de ícono
    pub is_group_expanded: Option<bool>, // Estado UI: expandido/colapsado
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectActivity {
    pub id: i64,
    pub project_id: i64,
    pub activity_type: String,
    pub description: Option<String>,
    pub duration_seconds: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // DTO preparado para registro manual de actividades
pub struct CreateActivityDTO {
    pub project_id: i64,
    pub activity_type: String,
    pub description: Option<String>,
    pub duration_seconds: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectStats {
    pub total_projects: i64,
    pub active_today: i64,
    pub total_time_hours: f64,
    pub most_active_project: Option<String>,
    pub recent_activities: Vec<ProjectActivity>,
}

// Proyecto con sus hijos (subproyectos) para navegación de grupos
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectWithChildren {
    pub project: Project,
    pub children: Vec<Project>,
    pub subproject_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectDTO {
    pub name: String,
    pub description: String,
    pub local_path: String,
    pub documentation_url: Option<String>,
    pub ai_documentation_url: Option<String>,
    pub drive_link: Option<String>,
    pub notes: Option<String>,
    pub image_data: Option<String>,
    // Group fields (v0.4.0)
    pub parent_id: Option<i64>,
    pub group_color: Option<String>,
    pub group_icon: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateLinkDTO {
    pub project_id: i64,
    pub link_type: String,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLinkDTO {
    pub link_type: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectDTO {
    pub name: Option<String>,
    pub description: Option<String>,
    pub local_path: Option<String>,
    pub documentation_url: Option<String>,
    pub ai_documentation_url: Option<String>,
    pub drive_link: Option<String>,
    pub notes: Option<String>,
    pub image_data: Option<String>,
    // Group fields (v0.4.0)
    pub parent_id: Option<i64>,
    pub group_color: Option<String>,
    pub group_icon: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectAttachment {
    pub id: i64,
    pub project_id: i64,
    pub filename: String,
    pub file_data: String,
    pub file_size: i64,
    pub mime_type: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAttachmentDTO {
    pub project_id: i64,
    pub filename: String,
    pub file_data: String,
    pub file_size: i64,
    pub mime_type: String,
}

// ==================== PROJECT JOURNAL ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: i64,
    pub project_id: i64,
    pub content: String,
    pub tags: Option<String>, // JSON array: ["bug", "tip"]
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateJournalEntryDTO {
    pub project_id: i64,
    pub content: String,
    pub tags: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateJournalEntryDTO {
    pub content: Option<String>,
    pub tags: Option<String>,
}

// ==================== DASHBOARD DATA ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardTodo {
    #[serde(flatten)]
    pub todo: ProjectTodo,
    pub project_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardJournalEntry {
    #[serde(flatten)]
    pub entry: JournalEntry,
    pub project_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardData {
    pub recent_projects: Vec<Project>,
    pub pending_todos: Vec<DashboardTodo>,
    pub recent_journal_entries: Vec<DashboardJournalEntry>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTodo {
    pub id: i64,
    pub project_id: i64,
    pub content: String,
    pub is_completed: bool,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTodoDTO {
    pub project_id: i64,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTodoDTO {
    pub content: Option<String>,
    pub is_completed: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_serialization_roundtrip() {
        let project = Project {
            id: 1,
            name: "Test".to_string(),
            description: "Desc".to_string(),
            local_path: "/tmp/test".to_string(),
            documentation_url: Some("https://docs.example.com".to_string()),
            ai_documentation_url: None,
            drive_link: None,
            notes: Some("Notes here".to_string()),
            image_data: None,
            links: Some(vec![ProjectLink {
                id: 10,
                project_id: 1,
                link_type: "github".to_string(),
                title: "Repo".to_string(),
                url: "https://github.com/test".to_string(),
                created_at: "2025-01-01".to_string(),
            }]),
            created_at: "2025-01-01".to_string(),
            updated_at: "2025-01-02".to_string(),
            last_opened_at: Some("2025-01-02".to_string()),
            opened_count: Some(5),
            total_time_seconds: Some(3600),
            status: Some("activo".to_string()),
            status_changed_at: None,
            is_pinned: Some(true),
            pinned_order: Some(1),
            display_order: Some(0),
            parent_id: None,
            group_color: Some("#FF0000".to_string()),
            group_icon: Some("rocket".to_string()),
            is_group_expanded: Some(true),
        };
        let json = serde_json::to_string(&project).unwrap();
        let deserialized: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, 1);
        assert_eq!(deserialized.name, "Test");
        assert_eq!(deserialized.links.as_ref().unwrap().len(), 1);
        assert_eq!(deserialized.group_color.as_deref(), Some("#FF0000"));
    }

    #[test]
    fn test_dashboard_todo_flatten() {
        let dt = DashboardTodo {
            todo: ProjectTodo {
                id: 1,
                project_id: 2,
                content: "Task".to_string(),
                is_completed: false,
                created_at: "2025-01-01".to_string(),
                completed_at: None,
            },
            project_name: "My Project".to_string(),
        };
        let json = serde_json::to_string(&dt).unwrap();
        // flatten means "id", "content" are at top level, not nested
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["content"], "Task");
        assert_eq!(v["project_name"], "My Project");
        assert!(v.get("todo").is_none()); // flattened, no nested "todo" key
    }

    #[test]
    fn test_dashboard_journal_entry_flatten() {
        let dje = DashboardJournalEntry {
            entry: JournalEntry {
                id: 1,
                project_id: 2,
                content: "Entry content".to_string(),
                tags: Some("[\"tag1\"]".to_string()),
                created_at: "2025-01-01".to_string(),
                updated_at: "2025-01-01".to_string(),
            },
            project_name: "Project X".to_string(),
        };
        let json = serde_json::to_string(&dje).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["content"], "Entry content");
        assert_eq!(v["project_name"], "Project X");
        assert!(v.get("entry").is_none()); // flattened
    }

    #[test]
    fn test_create_project_dto_deserialization() {
        let json = r#"{
            "name": "New Project",
            "description": "Desc",
            "local_path": "/tmp/new",
            "documentation_url": null,
            "ai_documentation_url": null,
            "drive_link": null,
            "notes": null,
            "image_data": null,
            "parent_id": null,
            "group_color": null,
            "group_icon": null
        }"#;
        let dto: CreateProjectDTO = serde_json::from_str(json).unwrap();
        assert_eq!(dto.name, "New Project");
        assert!(dto.parent_id.is_none());
    }
}
