//! Claude Code spawning logic.
//!
//! High-level API for spawning Claude Code instances in tmux panes.

use crate::tmux;
use std::process::Command;

pub use tmux::in_tmux;

/// Get the claude binary path.
fn get_claude_path() -> Result<String, String> {
    // Primary: use `which` to find claude in PATH (works with homebrew, etc.)
    let output = Command::new("which")
        .arg("claude")
        .output()
        .map_err(|e| format!("Failed to find claude: {}", e))?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(path);
        }
    }

    // Fallback: check ~/.claude/local/claude (local dev installs)
    if let Some(home) = std::env::var_os("HOME") {
        let local_claude = std::path::PathBuf::from(&home).join(".claude/local/claude");
        if local_claude.exists() {
            return Ok(local_claude.to_string_lossy().to_string());
        }
    }

    Err("claude not found in PATH".into())
}

/// Spawn Claude Code in a new tmux pane to the right.
///
/// The new pane takes 75% of the width, leaving the TUI in a narrow sidebar.
/// Returns the pane ID (e.g., "%5") on success.
pub fn spawn_claude_in_tmux(prompt: &str) -> Result<String, String> {
    if !in_tmux() {
        return Err("Not running in tmux. Start with: tmux".into());
    }

    let claude_path = get_claude_path()?;

    // Escape the prompt for shell
    let escaped_prompt = prompt.replace('\\', "\\\\").replace('"', "\\\"");

    let command = format!(r#"{} "{}""#, claude_path, escaped_prompt);

    // Split horizontally (side by side), new pane gets 75%, stay in current pane
    tmux::split_pane(true, 75, Some(&command), true)
}

/// Kill a tmux pane by ID (gracefully by default).
pub fn kill_pane(pane_id: &str) -> Result<(), String> {
    tmux::kill_pane(pane_id, true)
}

/// Information about a spawned Claude instance.
#[derive(Debug, Clone)]
pub struct SpawnedClaude {
    pub pane_id: String,
    pub label: String,
}

impl SpawnedClaude {
    /// Check if Claude is still running in this pane.
    pub fn is_alive(&self) -> bool {
        tmux::get_pane_command(&self.pane_id).is_some()
    }

    /// Get the current command running in the pane.
    pub fn current_command(&self) -> Option<String> {
        tmux::get_pane_command(&self.pane_id)
    }

    /// Kill this Claude instance.
    pub fn kill(&self) -> Result<(), String> {
        kill_pane(&self.pane_id)
    }
}
