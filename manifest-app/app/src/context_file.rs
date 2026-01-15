//! File-based active feature context sharing.
//!
//! Writes the currently selected feature to `~/.manifest/active_context.json`
//! so the MCP server can read it and provide context to AI agents.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Active feature context written to disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    write_context_to_path(&path, feature_id, title)
}

/// Clear the active feature context (no feature selected).
pub fn clear_context() -> std::io::Result<()> {
    let path = context_file_path().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine home directory",
        )
    })?;
    clear_context_at_path(&path)
}

/// Read the active feature context from disk.
pub fn read_context() -> std::io::Result<Option<ActiveContext>> {
    let path = context_file_path().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine home directory",
        )
    })?;
    read_context_from_path(&path)
}

// --- Internal functions that accept a path (for testing) ---

fn write_context_to_path(path: &Path, feature_id: Uuid, title: &str) -> std::io::Result<()> {
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

fn clear_context_at_path(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn read_context_from_path(path: &Path) -> std::io::Result<Option<ActiveContext>> {
    if !path.exists() {
        return Ok(None);
    }

    let json = fs::read_to_string(path)?;
    let context: ActiveContext = serde_json::from_str(&json)?;
    Ok(Some(context))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_path(dir: &TempDir) -> PathBuf {
        dir.path().join("active_context.json")
    }

    #[test]
    fn write_and_read_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = test_path(&dir);
        let feature_id = Uuid::new_v4();
        let title = "Test Feature";

        write_context_to_path(&path, feature_id, title).unwrap();

        let context = read_context_from_path(&path).unwrap().unwrap();
        assert_eq!(context.feature_id, feature_id);
        assert_eq!(context.title, title);
        assert!(!context.updated_at.is_empty());
    }

    #[test]
    fn read_returns_none_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let path = test_path(&dir);

        let result = read_context_from_path(&path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn clear_removes_existing_file() {
        let dir = TempDir::new().unwrap();
        let path = test_path(&dir);
        let feature_id = Uuid::new_v4();

        write_context_to_path(&path, feature_id, "Test").unwrap();
        assert!(path.exists());

        clear_context_at_path(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn clear_succeeds_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let path = test_path(&dir);

        // Should not error when file doesn't exist
        clear_context_at_path(&path).unwrap();
    }

    #[test]
    fn write_creates_parent_directories() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("dir").join("context.json");
        let feature_id = Uuid::new_v4();

        write_context_to_path(&path, feature_id, "Nested Test").unwrap();

        assert!(path.exists());
        let context = read_context_from_path(&path).unwrap().unwrap();
        assert_eq!(context.feature_id, feature_id);
    }

    #[test]
    fn write_overwrites_existing_file() {
        let dir = TempDir::new().unwrap();
        let path = test_path(&dir);
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        write_context_to_path(&path, id1, "First").unwrap();
        write_context_to_path(&path, id2, "Second").unwrap();

        let context = read_context_from_path(&path).unwrap().unwrap();
        assert_eq!(context.feature_id, id2);
        assert_eq!(context.title, "Second");
    }

    #[test]
    fn json_format_is_valid() {
        let dir = TempDir::new().unwrap();
        let path = test_path(&dir);
        let feature_id = Uuid::new_v4();

        write_context_to_path(&path, feature_id, "JSON Test").unwrap();

        let json = fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed.get("feature_id").is_some());
        assert!(parsed.get("title").is_some());
        assert!(parsed.get("updated_at").is_some());
    }

    #[test]
    fn updated_at_is_rfc3339_format() {
        let dir = TempDir::new().unwrap();
        let path = test_path(&dir);
        let feature_id = Uuid::new_v4();

        write_context_to_path(&path, feature_id, "Timestamp Test").unwrap();

        let context = read_context_from_path(&path).unwrap().unwrap();
        // RFC3339 parsing should succeed
        chrono::DateTime::parse_from_rfc3339(&context.updated_at)
            .expect("updated_at should be valid RFC3339");
    }

    #[test]
    fn handles_special_characters_in_title() {
        let dir = TempDir::new().unwrap();
        let path = test_path(&dir);
        let feature_id = Uuid::new_v4();
        let title = "Feature with \"quotes\" and 'apostrophes' and\nnewlines";

        write_context_to_path(&path, feature_id, title).unwrap();

        let context = read_context_from_path(&path).unwrap().unwrap();
        assert_eq!(context.title, title);
    }

    #[test]
    fn handles_unicode_in_title() {
        let dir = TempDir::new().unwrap();
        let path = test_path(&dir);
        let feature_id = Uuid::new_v4();
        let title = "Feature \u{1F680} Rocket \u{2728} Sparkles";

        write_context_to_path(&path, feature_id, title).unwrap();

        let context = read_context_from_path(&path).unwrap().unwrap();
        assert_eq!(context.title, title);
    }

    #[test]
    fn read_fails_on_invalid_json() {
        let dir = TempDir::new().unwrap();
        let path = test_path(&dir);

        fs::write(&path, "not valid json").unwrap();

        let result = read_context_from_path(&path);
        assert!(result.is_err());
    }

    #[test]
    fn read_fails_on_missing_fields() {
        let dir = TempDir::new().unwrap();
        let path = test_path(&dir);

        fs::write(&path, r#"{"feature_id": "not-a-uuid"}"#).unwrap();

        let result = read_context_from_path(&path);
        assert!(result.is_err());
    }
}
