use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureHistory {
    pub id: Uuid,
    pub feature_id: Uuid,
    pub session_id: Option<Uuid>,
    pub summary: String,
    pub files_changed: Vec<String>,
    pub author: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateHistoryInput {
    pub feature_id: Uuid,
    pub session_id: Option<Uuid>,
    pub summary: String,
    pub files_changed: Vec<String>,
    pub author: String,
}
