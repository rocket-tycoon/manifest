use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::task::CreateTaskInput;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub feature_id: Uuid,
    pub goal: String,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Completed,
    Failed,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionInput {
    pub feature_id: Uuid,
    pub goal: String,
    pub tasks: Vec<CreateTaskInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub session: Session,
    pub tasks: Vec<super::Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatusResponse {
    pub session: Session,
    pub feature: SessionFeatureSummary,
    pub tasks: Vec<super::Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFeatureSummary {
    pub id: Uuid,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteSessionInput {
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCompletionResult {
    pub session: Session,
    pub history_entry: super::FeatureHistory,
}
