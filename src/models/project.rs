use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDirectory {
    pub id: Uuid,
    pub project_id: Uuid,
    pub path: String,
    pub git_remote: Option<String>,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectInput {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProjectInput {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddDirectoryInput {
    pub path: String,
    pub git_remote: Option<String>,
    #[serde(default)]
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDirectoryInput {
    pub path: Option<String>,
    pub git_remote: Option<String>,
    pub is_primary: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectWithDirectories {
    #[serde(flatten)]
    pub project: Project,
    pub directories: Vec<ProjectDirectory>,
}
