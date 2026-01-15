//! Terminal emulator backed by alacritty_terminal.
//!
//! This creates a PTY and event loop for shell interaction.

use std::borrow::Cow;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use alacritty_terminal::event::{
    Event as AlacTermEvent, EventListener, Notify, OnResize, WindowSize,
};
use alacritty_terminal::event_loop::{EventLoop, Notifier};
use alacritty_terminal::grid::Scroll as AlacScroll;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::tty;
use alacritty_terminal::vte::ansi::{CursorShape, CursorStyle, Handler};
use anyhow::{Context as _, Result};
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use gpui::{App, AsyncApp, Context, EventEmitter, Keystroke, Task, WeakEntity};

use crate::bounds::TerminalBounds;
use crate::content::{IndexedCell, TerminalContent};
use crate::event::{Event, InternalEvent};
use crate::mappings::keys::to_esc_str;

/// Default scrollback history lines.
const DEFAULT_SCROLL_HISTORY_LINES: usize = 10_000;

/// Bridges alacritty terminal events to our channel.
pub struct LegionListener(pub UnboundedSender<AlacTermEvent>);

impl EventListener for LegionListener {
    fn send_event(&self, event: AlacTermEvent) {
        self.0.unbounded_send(event).ok();
    }
}

/// Terminal entity wrapping alacritty_terminal.
pub struct Terminal {
    term: Arc<FairMutex<Term<LegionListener>>>,
    pty_tx: Notifier,
    events: VecDeque<InternalEvent>,
    pub last_content: TerminalContent,
    #[allow(dead_code)]
    event_loop_task: Task<Result<()>>,
}

impl EventEmitter<Event> for Terminal {}

impl Terminal {
    /// Create a new terminal with PTY.
    pub fn new(
        working_directory: Option<PathBuf>,
        window_id: u64,
        cx: &App,
    ) -> Task<Result<TerminalBuilder>> {
        cx.background_executor()
            .spawn(async move { Self::create_pty_sync(working_directory, window_id) })
    }

    /// Create PTY synchronously - can be called from a background thread.
    pub fn create_pty_sync(
        working_directory: Option<PathBuf>,
        window_id: u64,
    ) -> Result<TerminalBuilder> {
        Self::create_pty_with_shell(working_directory, window_id, None, Vec::new())
    }

    /// Create PTY with a custom shell/command.
    ///
    /// This is useful for:
    /// - Agent terminals that run `claude` directly
    /// - Testing with mock commands like `cat`
    ///
    /// # Arguments
    /// * `working_directory` - Directory to start the shell in
    /// * `window_id` - Window ID for PTY metadata
    /// * `shell` - Optional shell program (e.g., "claude", "/bin/bash"). If None, uses system default.
    /// * `args` - Arguments to pass to the shell program
    ///
    /// # Example
    /// ```ignore
    /// // Run claude CLI directly
    /// let builder = Terminal::create_pty_with_shell(
    ///     Some(worktree_path),
    ///     0,
    ///     Some("claude".to_string()),
    ///     vec!["--print".to_string()],
    /// )?;
    /// ```
    pub fn create_pty_with_shell(
        working_directory: Option<PathBuf>,
        window_id: u64,
        shell: Option<String>,
        args: Vec<String>,
    ) -> Result<TerminalBuilder> {
        // Set up PTY options
        let shell_config = shell.map(|program| tty::Shell::new(program, args));

        let pty_options = tty::Options {
            shell: shell_config,
            working_directory,
            hold: false,
            env: std::env::vars().collect(),
        };

        let config = Config {
            scrolling_history: DEFAULT_SCROLL_HISTORY_LINES,
            default_cursor_style: CursorStyle {
                shape: CursorShape::Block,
                blinking: false,
            },
            ..Config::default()
        };

        // Create PTY
        let pty = tty::new(&pty_options, TerminalBounds::default().into(), window_id)
            .context("failed to create PTY")?;

        // Create event channel
        let (events_tx, events_rx) = unbounded();

        // Create terminal state
        let term = Term::new(
            config,
            &TerminalBounds::default(),
            LegionListener(events_tx.clone()),
        );
        let term = Arc::new(FairMutex::new(term));

        // Create and spawn event loop
        let event_loop = EventLoop::new(
            term.clone(),
            LegionListener(events_tx),
            pty,
            pty_options.hold,
            false,
        )
        .context("failed to create event loop")?;

        let pty_tx = Notifier(event_loop.channel());
        let _io_thread = event_loop.spawn();

        Ok(TerminalBuilder {
            term,
            pty_tx,
            events_rx,
        })
    }

    /// Finalize terminal construction with GPUI context.
    pub fn from_builder(builder: TerminalBuilder, cx: &mut Context<Self>) -> Self {
        let term = builder.term;
        let pty_tx = builder.pty_tx;
        let events_rx = builder.events_rx;

        // Spawn event processing task
        let event_loop_task =
            cx.spawn(async move |this, cx| Self::process_events(this, events_rx, cx).await);

        Terminal {
            term,
            pty_tx,
            events: VecDeque::with_capacity(10),
            last_content: TerminalContent::default(),
            event_loop_task,
        }
    }

    /// Process events from the PTY event loop.
    async fn process_events(
        this: WeakEntity<Self>,
        mut events_rx: UnboundedReceiver<AlacTermEvent>,
        cx: &mut AsyncApp,
    ) -> Result<()> {
        while let Some(event) = events_rx.next().await {
            // Process this event
            this.update(cx, |terminal, cx| {
                terminal.process_alac_event(&event, cx);
            })?;

            // Sync and notify
            this.update(cx, |terminal, cx| {
                terminal.sync();
                cx.emit(Event::Wakeup);
                cx.notify();
            })?;
        }

        Ok(())
    }

    /// Process a single alacritty terminal event.
    fn process_alac_event(&mut self, event: &AlacTermEvent, cx: &mut Context<Self>) {
        match event {
            AlacTermEvent::Wakeup => {
                // Terminal content changed
            }
            AlacTermEvent::Bell => {
                cx.emit(Event::Bell);
            }
            AlacTermEvent::Exit => {
                cx.emit(Event::CloseTerminal);
            }
            AlacTermEvent::Title(title) => {
                tracing::debug!("Terminal title changed: {}", title);
                cx.emit(Event::TitleChanged);
            }
            AlacTermEvent::ChildExit(_status) => {
                cx.emit(Event::CloseTerminal);
            }
            _ => {}
        }
    }

    /// Write input bytes to the PTY.
    pub fn input(&mut self, input: impl Into<Cow<'static, [u8]>>) {
        // Scroll to bottom on input
        self.events
            .push_back(InternalEvent::Scroll(AlacScroll::Bottom));
        self.write_to_pty(input);
    }

    /// Write bytes directly to the PTY.
    fn write_to_pty(&mut self, input: impl Into<Cow<'static, [u8]>>) {
        let input: Cow<'static, [u8]> = input.into();
        self.pty_tx.notify(input);
    }

    /// Try to handle a keystroke, returning true if handled.
    pub fn try_keystroke(&mut self, keystroke: &Keystroke, option_as_meta: bool) -> bool {
        let esc = to_esc_str(keystroke, &self.last_content.mode, option_as_meta);
        if let Some(esc) = esc {
            match esc {
                Cow::Borrowed(string) => self.input(string.as_bytes()),
                Cow::Owned(string) => self.input(string.into_bytes()),
            };
            true
        } else {
            false
        }
    }

    /// Resize the terminal.
    pub fn resize(&mut self, bounds: TerminalBounds) {
        self.events.push_back(InternalEvent::Resize(bounds));
    }

    /// Scroll up by the given number of lines.
    pub fn scroll_lines(&mut self, lines: i32) {
        if lines > 0 {
            self.events
                .push_back(InternalEvent::Scroll(AlacScroll::Delta(lines)));
        } else if lines < 0 {
            self.events
                .push_back(InternalEvent::Scroll(AlacScroll::Delta(lines)));
        }
    }

    /// Scroll up by one page.
    pub fn scroll_page_up(&mut self) {
        self.events
            .push_back(InternalEvent::Scroll(AlacScroll::PageUp));
    }

    /// Scroll down by one page.
    pub fn scroll_page_down(&mut self) {
        self.events
            .push_back(InternalEvent::Scroll(AlacScroll::PageDown));
    }

    /// Scroll to the top of the scrollback.
    pub fn scroll_to_top(&mut self) {
        self.events
            .push_back(InternalEvent::Scroll(AlacScroll::Top));
    }

    /// Scroll to the bottom (most recent output).
    pub fn scroll_to_bottom(&mut self) {
        self.events
            .push_back(InternalEvent::Scroll(AlacScroll::Bottom));
    }

    /// Sync terminal state - process internal events and update content.
    pub fn sync(&mut self) {
        let term = self.term.clone();
        let mut terminal = term.lock_unfair();

        // Process internal events
        while let Some(event) = self.events.pop_front() {
            match event {
                InternalEvent::Resize(bounds) => {
                    terminal.resize(bounds);
                    let window_size: WindowSize = bounds.into();
                    self.pty_tx.on_resize(window_size);
                    self.last_content.terminal_bounds = bounds;
                }
                InternalEvent::Clear => {
                    terminal.clear_screen(alacritty_terminal::vte::ansi::ClearMode::All);
                }
                InternalEvent::Scroll(scroll) => {
                    terminal.scroll_display(scroll);
                }
            }
        }

        // Update content snapshot
        self.last_content = Self::make_content(&terminal, &self.last_content);
    }

    /// Create a content snapshot from the terminal state.
    fn make_content(
        term: &Term<LegionListener>,
        last_content: &TerminalContent,
    ) -> TerminalContent {
        let content = term.renderable_content();

        let estimated_size = content.display_iter.size_hint().0;
        let mut cells = Vec::with_capacity(estimated_size);

        cells.extend(content.display_iter.map(|ic| IndexedCell {
            point: ic.point,
            cell: ic.cell.clone(),
        }));

        TerminalContent {
            cells,
            mode: content.mode,
            display_offset: content.display_offset,
            cursor: content.cursor,
            cursor_char: term.grid()[content.cursor.point].c,
            terminal_bounds: last_content.terminal_bounds,
        }
    }

    /// Get the current terminal mode.
    pub fn mode(&self) -> TermMode {
        self.last_content.mode
    }

    /// Get the terminal content for rendering.
    pub fn content(&self) -> &TerminalContent {
        &self.last_content
    }
}

/// Builder for Terminal - separates async PTY creation from entity construction.
pub struct TerminalBuilder {
    term: Arc<FairMutex<Term<LegionListener>>>,
    pty_tx: Notifier,
    events_rx: UnboundedReceiver<AlacTermEvent>,
}

impl TerminalBuilder {
    /// Build the terminal entity.
    pub fn build(self, cx: &mut Context<Terminal>) -> Terminal {
        Terminal::from_builder(self, cx)
    }
}
