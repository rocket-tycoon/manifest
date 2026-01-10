use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Criterion {
    pub id: Uuid,
    pub task_id: Uuid,
    pub description: String,
    pub status: CriterionStatus,
    pub verification: VerificationType,
    pub test_file: Option<String>,
    pub blocked_reason: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CriterionStatus {
    Pending,
    Complete,
    Blocked,
}

impl CriterionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Complete => "complete",
            Self::Blocked => "blocked",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "complete" => Some(Self::Complete),
            "blocked" => Some(Self::Blocked),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationType {
    Manual,
    Test,
}

impl VerificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Test => "test",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "manual" => Some(Self::Manual),
            "test" => Some(Self::Test),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCriterionInput {
    pub description: String,
    #[serde(default)]
    pub verification: Option<VerificationType>,
    pub test_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCriterionInput {
    pub status: Option<CriterionStatus>,
    pub blocked_reason: Option<String>,
}
