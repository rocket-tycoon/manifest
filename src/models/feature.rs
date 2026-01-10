use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub id: Uuid,
    pub project_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub story: Option<String>,
    pub details: Option<String>,
    pub state: FeatureState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeatureState {
    Proposed,
    Specified,
    Implemented,
    Deprecated,
}

impl FeatureState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Specified => "specified",
            Self::Implemented => "implemented",
            Self::Deprecated => "deprecated",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "proposed" => Some(Self::Proposed),
            "specified" => Some(Self::Specified),
            "implemented" => Some(Self::Implemented),
            "deprecated" => Some(Self::Deprecated),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFeatureInput {
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub story: Option<String>,
    pub details: Option<String>,
    pub state: Option<FeatureState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFeatureInput {
    pub parent_id: Option<Uuid>,
    pub title: Option<String>,
    pub story: Option<String>,
    pub details: Option<String>,
    pub state: Option<FeatureState>,
}
