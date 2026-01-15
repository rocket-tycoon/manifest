//! Tmux integration module.
//!
//! Management patterns:
//! - Session name validation to prevent shell injection
//! - Pane command/PID detection for process monitoring
//! - Graceful shutdown (Ctrl-C then kill)
//! - Session caching for O(1) lookups
//! - Multiple send patterns for different scenarios

use std::collections::HashSet;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Validates session/pane names to prevent shell injection.
/// Only allows alphanumeric, underscore, and hyphen.
fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Check if we're running inside tmux.
pub fn in_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

/// Get the current tmux session name.
pub fn current_session() -> Option<String> {
    let output = Command::new("tmux")
        .args(["display-message", "-p", "#{session_name}"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Session Management
// ─────────────────────────────────────────────────────────────────────────────

/// Cached set of active tmux sessions for O(1) lookup.
/// Replaces N subprocess calls with a single `list-sessions`.
pub struct SessionSet {
    sessions: HashSet<String>,
}

impl SessionSet {
    /// Fetch all active sessions in one call.
    pub fn fetch() -> Result<Self, String> {
        let output = Command::new("tmux")
            .args(["list-sessions", "-F", "#{session_name}"])
            .output()
            .map_err(|e| format!("Failed to list sessions: {}", e))?;

        if !output.status.success() {
            // tmux may return error if no sessions exist
            return Ok(Self {
                sessions: HashSet::new(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let sessions: HashSet<String> = stdout.lines().map(String::from).collect();

        Ok(Self { sessions })
    }

    /// Check if a session exists (O(1)).
    pub fn contains(&self, name: &str) -> bool {
        self.sessions.contains(name)
    }
}

/// Create a new detached tmux session.
pub fn new_session(name: &str, working_dir: Option<&str>) -> Result<(), String> {
    if !is_valid_name(name) {
        return Err(format!("Invalid session name: {}", name));
    }

    let mut args = vec!["new-session", "-d", "-s", name];

    if let Some(dir) = working_dir {
        args.extend(["-c", dir]);
    }

    let status = Command::new("tmux")
        .args(&args)
        .status()
        .map_err(|e| format!("Failed to create session: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("tmux new-session failed for '{}'", name))
    }
}

/// Create a session with an initial command (avoids shell readiness race).
pub fn new_session_with_command(
    name: &str,
    command: &str,
    working_dir: Option<&str>,
) -> Result<(), String> {
    if !is_valid_name(name) {
        return Err(format!("Invalid session name: {}", name));
    }

    let mut args = vec!["new-session", "-d", "-s", name];

    if let Some(dir) = working_dir {
        args.extend(["-c", dir]);
    }

    args.push(command);

    let status = Command::new("tmux")
        .args(&args)
        .status()
        .map_err(|e| format!("Failed to create session: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("tmux new-session failed for '{}'", name))
    }
}

/// Kill a session, optionally gracefully (Ctrl-C first, then kill).
pub fn kill_session(name: &str, graceful: bool) -> Result<(), String> {
    if !is_valid_name(name) {
        return Err(format!("Invalid session name: {}", name));
    }

    if graceful {
        // Send Ctrl-C and wait briefly
        let _ = send_keys(name, "C-c", false);
        thread::sleep(Duration::from_millis(100));
    }

    let status = Command::new("tmux")
        .args(["kill-session", "-t", name])
        .status()
        .map_err(|e| format!("Failed to kill session: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Failed to kill session '{}'", name))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pane Operations
// ─────────────────────────────────────────────────────────────────────────────

/// Information about a tmux pane.
#[derive(Debug, Clone)]
pub struct PaneInfo {
    pub id: String,
    pub pid: Option<u32>,
    pub command: Option<String>,
    pub working_dir: Option<String>,
}

/// Get information about a pane.
pub fn get_pane_info(pane_id: &str) -> Result<PaneInfo, String> {
    let output = Command::new("tmux")
        .args([
            "display-message",
            "-t",
            pane_id,
            "-p",
            "#{pane_id}:#{pane_pid}:#{pane_current_command}:#{pane_current_path}",
        ])
        .output()
        .map_err(|e| format!("Failed to get pane info: {}", e))?;

    if !output.status.success() {
        return Err(format!("Pane {} not found", pane_id));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.trim().splitn(4, ':').collect();

    Ok(PaneInfo {
        id: parts.first().unwrap_or(&"").to_string(),
        pid: parts.get(1).and_then(|s| s.parse().ok()),
        command: parts.get(2).map(|s| s.to_string()),
        working_dir: parts.get(3).map(|s| s.to_string()),
    })
}

/// Get the current command running in a pane.
pub fn get_pane_command(pane_id: &str) -> Option<String> {
    let output = Command::new("tmux")
        .args([
            "display-message",
            "-t",
            pane_id,
            "-p",
            "#{pane_current_command}",
        ])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Get the PID of the main process in a pane.
pub fn get_pane_pid(pane_id: &str) -> Option<u32> {
    let output = Command::new("tmux")
        .args(["display-message", "-t", pane_id, "-p", "#{pane_pid}"])
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8_lossy(&output.stdout).trim().parse().ok()
    } else {
        None
    }
}

/// Split the current pane and return the new pane ID.
pub fn split_pane(
    horizontal: bool,
    percent: u8,
    command: Option<&str>,
    stay_in_current: bool,
) -> Result<String, String> {
    let mut args = vec![
        "split-window".to_string(),
        if horizontal { "-h" } else { "-v" }.to_string(),
        "-p".to_string(),
        percent.to_string(),
        "-P".to_string(),
        "-F".to_string(),
        "#{pane_id}".to_string(),
    ];

    if stay_in_current {
        args.push("-d".to_string());
    }

    if let Some(cmd) = command {
        args.push(cmd.to_string());
    }

    let output = Command::new("tmux")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to split pane: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err("tmux split-window failed".into())
    }
}

/// Kill a pane, optionally gracefully.
pub fn kill_pane(pane_id: &str, graceful: bool) -> Result<(), String> {
    if graceful {
        let _ = send_keys(pane_id, "C-c", false);
        thread::sleep(Duration::from_millis(100));
    }

    let status = Command::new("tmux")
        .args(["kill-pane", "-t", pane_id])
        .status()
        .map_err(|e| format!("Failed to kill pane: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Failed to kill pane {}", pane_id))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Send Keys (Multiple Patterns)
// ─────────────────────────────────────────────────────────────────────────────

/// Basic send keys with optional Enter.
pub fn send_keys(target: &str, keys: &str, with_enter: bool) -> Result<(), String> {
    let mut args = vec!["send-keys", "-t", target, keys];

    if with_enter {
        args.push("Enter");
    }

    let status = Command::new("tmux")
        .args(&args)
        .status()
        .map_err(|e| format!("Failed to send keys: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("tmux send-keys failed".into())
    }
}

/// Send keys in literal mode (no tmux key interpretation).
pub fn send_keys_literal(target: &str, text: &str) -> Result<(), String> {
    let status = Command::new("tmux")
        .args(["send-keys", "-t", target, "-l", text])
        .status()
        .map_err(|e| format!("Failed to send keys: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("tmux send-keys failed".into())
    }
}

/// Clear pending input (Ctrl-U) then send keys.
pub fn send_keys_replace(target: &str, keys: &str, with_enter: bool) -> Result<(), String> {
    // Clear line first
    send_keys(target, "C-u", false)?;
    thread::sleep(Duration::from_millis(50));
    send_keys(target, keys, with_enter)
}

/// Send keys with a delay before sending.
pub fn send_keys_delayed(
    target: &str,
    keys: &str,
    delay_ms: u64,
    with_enter: bool,
) -> Result<(), String> {
    thread::sleep(Duration::from_millis(delay_ms));
    send_keys(target, keys, with_enter)
}

// ─────────────────────────────────────────────────────────────────────────────
// Capture and Readiness
// ─────────────────────────────────────────────────────────────────────────────

/// Capture the visible content of a pane.
pub fn capture_pane(pane_id: &str, lines: Option<i32>) -> Result<String, String> {
    let mut args = vec!["capture-pane", "-t", pane_id, "-p"];

    let lines_str;
    if let Some(n) = lines {
        lines_str = format!("-{}", n);
        args.extend(["-S", &lines_str]);
    }

    let output = Command::new("tmux")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to capture pane: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err("tmux capture-pane failed".into())
    }
}

/// Wait for a command to NOT be running in the pane.
/// Useful for waiting until a shell prompt is ready.
pub fn wait_for_command_exit(
    pane_id: &str,
    excluded_commands: &[&str],
    timeout_ms: u64,
    poll_ms: u64,
) -> Result<(), String> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_millis(timeout_ms);

    while start.elapsed() < timeout {
        if let Some(cmd) = get_pane_command(pane_id) {
            if !excluded_commands.iter().any(|&exc| cmd.contains(exc)) {
                return Ok(());
            }
        }
        thread::sleep(Duration::from_millis(poll_ms));
    }

    Err("Timeout waiting for command to exit".into())
}

// ─────────────────────────────────────────────────────────────────────────────
// Status Bar
// ─────────────────────────────────────────────────────────────────────────────

/// Set the tmux status bar format.
pub fn set_status_format(left: Option<&str>, right: Option<&str>) -> Result<(), String> {
    if let Some(l) = left {
        Command::new("tmux")
            .args(["set-option", "-g", "status-left", l])
            .status()
            .map_err(|e| format!("Failed to set status-left: {}", e))?;
    }

    if let Some(r) = right {
        Command::new("tmux")
            .args(["set-option", "-g", "status-right", r])
            .status()
            .map_err(|e| format!("Failed to set status-right: {}", e))?;
    }

    Ok(())
}

/// Set status bar style (bg and fg colors).
pub fn set_status_style(style: &str) -> Result<(), String> {
    let status = Command::new("tmux")
        .args(["set-option", "-g", "status-style", style])
        .status()
        .map_err(|e| format!("Failed to set status-style: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("Failed to set status style".into())
    }
}

/// Enable mouse support (click to switch panes, resize, scroll).
pub fn enable_mouse() -> Result<(), String> {
    let status = Command::new("tmux")
        .args(["set-option", "-g", "mouse", "on"])
        .status()
        .map_err(|e| format!("Failed to enable mouse: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("Failed to enable mouse".into())
    }
}
