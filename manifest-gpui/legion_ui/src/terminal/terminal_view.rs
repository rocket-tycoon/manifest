//! Terminal view - wraps the Terminal entity and handles rendering/input.
//!
//! This module provides two ways to create a terminal view:
//! 1. `new()` + `initialize()` - Creates and manages its own PTY (legacy pattern)
//! 2. `from_terminal()` - Accepts an existing Terminal entity (decoupled pattern)
//!
//! The decoupled pattern is preferred for agent terminals where we need to
//! inject prompts before the view is displayed.

use gpui::prelude::*;
use gpui::{ScrollDelta, ScrollWheelEvent, *};
use legion_terminal::{Event as TerminalEvent, Terminal, TerminalBuilder};
use std::borrow::Cow;
use std::path::PathBuf;

use crate::theme::Theme;

/// Terminal font family. Change this to your preferred monospace font.
/// Common options: "Menlo", "Monaco", "SF Mono", "Bitstream Vera Sans Mono"
const TERMINAL_FONT_FAMILY: &str = "Bitstream Vera Sans Mono";

/// Terminal view - displays terminal output and handles input.
pub struct TerminalView {
    terminal: Option<Entity<Terminal>>,
    working_directory: Option<PathBuf>,
    focus_handle: FocusHandle,
    initializing: bool,
}

impl TerminalView {
    /// Create a new terminal view that will spawn its own PTY.
    ///
    /// Call `initialize()` after construction to spawn the PTY.
    /// For pre-configured terminals (e.g., agent terminals), use `from_terminal()` instead.
    pub fn new(
        working_directory: Option<PathBuf>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        TerminalView {
            terminal: None,
            working_directory,
            focus_handle: cx.focus_handle(),
            initializing: false,
        }
    }

    /// Create a terminal view from an existing Terminal entity.
    ///
    /// This is the preferred constructor for agent terminals where the caller
    /// needs to configure the terminal (e.g., inject prompts) before display.
    ///
    /// # Example
    /// ```ignore
    /// // Create terminal model first
    /// let terminal = cx.new(|cx| builder.build(cx));
    ///
    /// // Inject prompt before creating view
    /// terminal.update(cx, |term, _| {
    ///     term.input(b"claude --help\n".to_vec());
    /// });
    ///
    /// // Create view with pre-configured terminal
    /// let view = cx.new(|cx| TerminalView::from_terminal(terminal, window, cx));
    /// ```
    pub fn from_terminal(
        terminal: Entity<Terminal>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::from_terminal_entity(terminal, cx)
    }

    /// Create a terminal view from an existing Terminal entity without requiring a window.
    ///
    /// This is useful when creating terminal views in async contexts where
    /// a window reference is not available.
    pub fn from_terminal_entity(terminal: Entity<Terminal>, cx: &mut Context<Self>) -> Self {
        // Subscribe to terminal events
        cx.subscribe(
            &terminal,
            |_this, _terminal, event: &TerminalEvent, cx| match event {
                TerminalEvent::Wakeup => {
                    cx.notify();
                }
                TerminalEvent::Bell => {
                    tracing::debug!("Terminal bell");
                }
                TerminalEvent::CloseTerminal => {
                    tracing::info!("Terminal closed");
                }
                TerminalEvent::TitleChanged => {
                    tracing::debug!("Terminal title changed");
                }
            },
        )
        .detach();

        TerminalView {
            terminal: Some(terminal),
            working_directory: None,
            focus_handle: cx.focus_handle(),
            initializing: false,
        }
    }

    /// Initialize the terminal (spawn PTY).
    pub fn initialize(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.initializing || self.terminal.is_some() {
            return;
        }
        self.initializing = true;

        let working_directory = self.working_directory.clone();
        // Use 0 as window_id - it's only used for PTY metadata
        let window_id = 0u64;

        // Spawn terminal creation in background
        let task = cx
            .background_executor()
            .spawn(async move { Terminal::create_pty_sync(working_directory, window_id) });

        cx.spawn(async move |this, cx| {
            match task.await {
                Ok(builder) => {
                    this.update(cx, |view, cx| {
                        view.build_terminal(builder, cx);
                        view.initializing = false;
                    })?;
                }
                Err(e) => {
                    tracing::error!("Failed to create terminal: {}", e);
                    this.update(cx, |view, _cx| {
                        view.initializing = false;
                    })?;
                }
            }
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn build_terminal(&mut self, builder: TerminalBuilder, cx: &mut Context<Self>) {
        let terminal = cx.new(|cx| builder.build(cx));

        // Subscribe to terminal events
        cx.subscribe(
            &terminal,
            |_this, _terminal, event: &TerminalEvent, cx| match event {
                TerminalEvent::Wakeup => {
                    cx.notify();
                }
                TerminalEvent::Bell => {
                    tracing::debug!("Terminal bell");
                }
                TerminalEvent::CloseTerminal => {
                    tracing::info!("Terminal closed");
                }
                TerminalEvent::TitleChanged => {
                    tracing::debug!("Terminal title changed");
                }
            },
        )
        .detach();

        self.terminal = Some(terminal);
        cx.notify();
    }

    /// Handle key down events.
    fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        tracing::debug!("Terminal received key event: {:?}", event.keystroke);

        if let Some(terminal) = &self.terminal {
            // Check for scroll shortcuts first (Shift+PageUp/PageDown)
            let key = event.keystroke.key.as_str();
            let shift = event.keystroke.modifiers.shift;

            if shift && key == "pageup" {
                terminal.update(cx, |term, _cx| {
                    term.scroll_page_up();
                });
                cx.notify();
                return;
            }
            if shift && key == "pagedown" {
                terminal.update(cx, |term, _cx| {
                    term.scroll_page_down();
                });
                cx.notify();
                return;
            }
            if shift && key == "home" {
                terminal.update(cx, |term, _cx| {
                    term.scroll_to_top();
                });
                cx.notify();
                return;
            }
            if shift && key == "end" {
                terminal.update(cx, |term, _cx| {
                    term.scroll_to_bottom();
                });
                cx.notify();
                return;
            }

            // Try special key handling (arrows, ctrl+c, etc.)
            let handled =
                terminal.update(cx, |term, _cx| term.try_keystroke(&event.keystroke, false));

            if handled {
                tracing::debug!("Special keystroke handled: {:?}", event.keystroke);
                cx.notify();
            } else if let Some(key_char) = &event.keystroke.key_char {
                // For regular characters, send them directly to the terminal
                if !key_char.is_empty() {
                    tracing::debug!("Sending key_char to terminal: {:?}", key_char);
                    let bytes = key_char.as_bytes().to_vec();
                    terminal.update(cx, |term, _cx| {
                        term.input(bytes);
                    });
                    cx.notify();
                }
            }
        } else {
            tracing::debug!("No terminal instance available");
        }
    }

    /// Handle scroll wheel events.
    fn on_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(terminal) = &self.terminal {
            // Convert scroll delta to lines (roughly 3 lines per scroll tick)
            let lines = match event.delta {
                ScrollDelta::Lines(delta) => (delta.y * 3.0) as i32,
                ScrollDelta::Pixels(delta) => (f32::from(delta.y) / 20.0) as i32,
            };

            if lines != 0 {
                terminal.update(cx, |term, _cx| {
                    term.scroll_lines(lines);
                });
                cx.notify();
            }
        }
    }

    /// Get the focus handle.
    pub fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }

    /// Focus the terminal.
    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_handle.focus(window);
        cx.notify();
    }

    /// Inject text input into the terminal.
    ///
    /// This is useful for sending initial prompts or commands to agent terminals.
    /// The text is sent directly to the PTY as if typed by the user.
    ///
    /// # Example
    /// ```ignore
    /// view.update(cx, |view, cx| {
    ///     view.inject_input("claude --print 'Implement the feature'\n", cx);
    /// });
    /// ```
    pub fn inject_input(&self, text: impl Into<Cow<'static, str>>, cx: &mut Context<Self>) {
        if let Some(terminal) = &self.terminal {
            let text: Cow<'static, str> = text.into();
            terminal.update(cx, |term, _cx| {
                term.input(text.into_owned().into_bytes());
            });
            cx.notify();
        } else {
            tracing::warn!("Cannot inject input: terminal not initialized");
        }
    }

    /// Get a reference to the underlying terminal entity, if initialized.
    pub fn terminal(&self) -> Option<&Entity<Terminal>> {
        self.terminal.as_ref()
    }

    /// Check if the terminal is initialized and ready.
    pub fn is_ready(&self) -> bool {
        self.terminal.is_some()
    }

    /// Check if the terminal is currently initializing.
    pub fn is_initializing(&self) -> bool {
        self.initializing
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus_handle.is_focused(window);

        // Main terminal container
        div()
            .id("terminal-view")
            .size_full()
            .bg(Theme::surface())
            .border_1()
            .border_color(if focused {
                Theme::accent()
            } else {
                Theme::border()
            })
            .rounded_md()
            .overflow_hidden()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
            .child(self.render_content(cx))
    }
}

impl TerminalView {
    /// Render the terminal content.
    fn render_content(&self, cx: &App) -> impl IntoElement {
        if let Some(terminal) = &self.terminal {
            let content = terminal.read(cx).content();
            let cursor_point = content.cursor.point;

            // Build lines with cursor
            let mut lines: Vec<String> = Vec::new();
            let mut current_line: i32 = -1;
            let mut current_line_text = String::new();
            let mut current_col: usize = 0;

            for cell in &content.cells {
                // New line
                if cell.point.line.0 != current_line {
                    if current_line >= 0 {
                        // Check if cursor is on the previous line past the text
                        if cursor_point.line.0 == current_line
                            && cursor_point.column.0 >= current_col
                        {
                            // Pad to cursor position and add cursor
                            while current_line_text.len() < cursor_point.column.0 {
                                current_line_text.push(' ');
                            }
                            current_line_text.push('█');
                        }
                        lines.push(current_line_text);
                    }
                    current_line = cell.point.line.0;
                    current_line_text = String::new();
                    current_col = 0;
                }

                // Check if cursor is at this position
                if cursor_point.line.0 == current_line
                    && cursor_point.column.0 == cell.point.column.0
                {
                    // Show cursor (inverse the character or show block)
                    if cell.cell.c == ' ' || cell.cell.c == '\0' {
                        current_line_text.push('█');
                    } else {
                        // For non-space, show the char with underline indicator
                        current_line_text.push(cell.cell.c);
                        current_line_text.push('▏'); // thin cursor after char
                    }
                } else {
                    current_line_text.push(cell.cell.c);
                }
                current_col = cell.point.column.0 + 1;
            }

            // Don't forget the last line
            if current_line >= 0 {
                // Check if cursor is on the last line past the text
                if cursor_point.line.0 == current_line && cursor_point.column.0 >= current_col {
                    while current_line_text.len() < cursor_point.column.0 {
                        current_line_text.push(' ');
                    }
                    current_line_text.push('█');
                }
                lines.push(current_line_text);
            }

            let text = if lines.is_empty() {
                "Terminal starting...".to_string()
            } else {
                lines.join("\n")
            };

            div()
                .p_2()
                .font_family(TERMINAL_FONT_FAMILY)
                .text_sm()
                .text_color(Theme::text())
                .child(text)
        } else if self.initializing {
            div()
                .p_4()
                .flex()
                .items_center()
                .justify_center()
                .text_color(Theme::text_muted())
                .child("Starting terminal...")
        } else {
            div()
                .p_4()
                .flex()
                .items_center()
                .justify_center()
                .text_color(Theme::text_muted())
                .child("Terminal not initialized")
        }
    }
}

impl Focusable for TerminalView {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.focus_handle(cx)
    }
}
