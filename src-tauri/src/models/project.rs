use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub local_path: String,
    pub documentation_url: Option<String>,
    pub ai_documentation_url: Option<String>,
    pub drive_link: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectDTO {
    pub name: String,
    pub description: String,
    pub local_path: String,
    pub documentation_url: Option<String>,
    pub ai_documentation_url: Option<String>,
    pub drive_link: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectDTO {
    pub name: Option<String>,
    pub description: Option<String>,
    pub local_path: Option<String>,
    pub documentation_url: Option<String>,
    pub ai_documentation_url: Option<String>,
    pub drive_link: Option<String>,
}
