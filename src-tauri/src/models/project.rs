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
}
