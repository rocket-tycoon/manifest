use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationNote {
    pub id: Uuid,
    pub feature_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub content: String,
    pub files_changed: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateImplementationNoteInput {
    pub content: String,
    #[serde(default)]
    pub files_changed: Vec<String>,
}
