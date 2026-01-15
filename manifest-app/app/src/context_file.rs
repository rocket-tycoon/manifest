//! File-based active feature context sharing.
//!
//! Writes the currently selected feature to `~/.manifest/active_context.json`
//! so the MCP server can read it and provide context to AI agents.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

/// Active feature context written to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveContext {
    pub feature_id: Uuid,
    pub title: String,
    pub updated_at: String,
}

/// Get the path to the active context file.
fn context_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".manifest").join("active_context.json"))
}

/// Write the active feature context to disk.
///
/// Uses atomic write (write to temp file, then rename) to prevent corruption.
pub fn write_context(feature_id: Uuid, title: &str) -> std::io::Result<()> {
    let path = context_file_path().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine home directory",
        )
    })?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let context = ActiveContext {
        feature_id,
        title: title.to_string(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    let json = serde_json::to_string_pretty(&context)?;

    // Atomic write: write to temp file, then rename
    let temp_path = path.with_extension("json.tmp");
    {
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&temp_path, &path)?;

    Ok(())
}

/// Clear the active feature context (no feature selected).
pub fn clear_context() -> std::io::Result<()> {
    let path = context_file_path().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine home directory",
        )
    })?;

    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

/// Read the active feature context from disk.
pub fn read_context() -> std::io::Result<Option<ActiveContext>> {
    let path = context_file_path().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine home directory",
        )
    })?;

    if !path.exists() {
        return Ok(None);
    }

    let json = fs::read_to_string(&path)?;
    let context: ActiveContext = serde_json::from_str(&json)?;
    Ok(Some(context))
}
