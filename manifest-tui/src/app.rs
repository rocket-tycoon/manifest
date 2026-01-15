//! Application state for the Manifest TUI.

use crate::spawn;

/// A selectable item in the TUI.
pub struct Item {
    pub label: String,
    pub prompt: String,
}

/// Application state.
pub struct App {
    pub items: Vec<Item>,
    pub selected: usize,
    pub should_quit: bool,
    pub last_error: Option<String>,
    pub last_spawn: Option<String>,
    /// Pane IDs of spawned Claude instances.
    pub spawned_panes: Vec<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            items: vec![
                Item {
                    label: "Test: Get project context".into(),
                    prompt: "Use the manifest MCP server to call get_project_context for the current directory. Show me what project this is and any instructions.".into(),
                },
                Item {
                    label: "Test: List features".into(),
                    prompt: "Use the manifest MCP server to call list_features. Show me what features exist in this project.".into(),
                },
                Item {
                    label: "Test: Create feature".into(),
                    prompt: "Use the manifest MCP server to: 1) call get_project_context to get the project_id, 2) create a test feature titled 'TUI Hello World' with details 'Test feature created from manifest-tui'. Show me the result.".into(),
                },
                Item {
                    label: "Hello world".into(),
                    prompt: "Hello from Manifest TUI!".into(),
                },
            ],
            selected: 0,
            should_quit: false,
            last_error: None,
            last_spawn: None,
            spawned_panes: Vec::new(),
        }
    }

    /// Handle Enter key - spawn the selected item.
    /// Only allows one pane at a time.
    pub fn spawn_selected(&mut self) {
        // Kill existing pane first (single-spawn mode)
        self.kill_spawned_panes();

        if let Some(item) = self.items.get(self.selected) {
            match spawn::spawn_claude_in_tmux(&item.prompt) {
                Ok(pane_id) => {
                    self.spawned_panes.push(pane_id);
                    self.last_error = None;
                    self.last_spawn = Some(item.label.clone());
                }
                Err(e) => {
                    self.last_error = Some(e);
                    self.last_spawn = None;
                }
            }
        }
    }

    /// Kill all spawned panes.
    pub fn kill_spawned_panes(&mut self) {
        for pane_id in &self.spawned_panes {
            let _ = spawn::kill_pane(pane_id);
        }
        self.spawned_panes.clear();
    }

    /// Move selection up.
    pub fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if self.selected < self.items.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    /// Quit the application and kill all spawned panes.
    pub fn quit(&mut self) {
        self.kill_spawned_panes();
        self.should_quit = true;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
