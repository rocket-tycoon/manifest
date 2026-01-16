//! Terminal emulation layer wrapping alacritty_terminal for GPUI applications.

pub mod mappings;
pub mod terminal_hyperlinks;

pub use alacritty_terminal;
use alacritty_terminal::term::search::Match;

use alacritty_terminal::{
    Term,
    event::{Event as AlacTermEvent, EventListener, Notify, WindowSize},
    event_loop::{EventLoop, Notifier},
    grid::Dimensions,
    index::{Column, Line, Point as AlacPoint},
    selection::SelectionRange,
    sync::FairMutex,
    term::{Config, RenderableCursor, TermMode, cell::Cell},
    tty,
    vte::ansi::{CursorShape as AlacCursorShape, CursorStyle},
};
use anyhow::{Context as _, Result};
use futures::StreamExt;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use gpui::{
    Bounds, Context, EventEmitter, Keystroke, Modifiers, MouseButton, Pixels, Point, Size, Task, px,
};
use std::{borrow::Cow, ops::Deref, path::PathBuf, sync::Arc};

use crate::mappings::keys::to_esc_str;
use crate::terminal_hyperlinks::{UrlSearch, find_url_at_point};

// Re-export key types
pub use alacritty_terminal::index::Point as TermPoint;
pub use alacritty_terminal::term::TermMode as Mode;

const DEFAULT_SCROLL_HISTORY_LINES: usize = 10_000;

/// Events emitted by the Terminal entity upward to the view layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    TitleChanged,
    CloseTerminal,
    Bell,
    Wakeup,
    /// Open a URL via Cmd+click
    OpenUrl(String),
}

/// Listener that bridges alacritty events to our async channel.
#[derive(Clone)]
pub struct ManifestListener(pub UnboundedSender<AlacTermEvent>);

impl EventListener for ManifestListener {
    fn send_event(&self, event: AlacTermEvent) {
        self.0.unbounded_send(event).ok();
    }
}

/// Terminal grid dimensions for layout calculations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerminalBounds {
    pub cell_width: Pixels,
    pub line_height: Pixels,
    pub bounds: Bounds<Pixels>,
}

impl TerminalBounds {
    pub fn new(line_height: Pixels, cell_width: Pixels, bounds: Bounds<Pixels>) -> Self {
        TerminalBounds {
            cell_width,
            line_height,
            bounds,
        }
    }

    pub fn num_lines(&self) -> usize {
        (self.bounds.size.height / self.line_height).floor() as usize
    }

    pub fn num_columns(&self) -> usize {
        (self.bounds.size.width / self.cell_width).floor() as usize
    }

    pub fn height(&self) -> Pixels {
        self.bounds.size.height
    }

    pub fn width(&self) -> Pixels {
        self.bounds.size.width
    }

    pub fn cell_width(&self) -> Pixels {
        self.cell_width
    }

    pub fn line_height(&self) -> Pixels {
        self.line_height
    }
}

impl Default for TerminalBounds {
    fn default() -> Self {
        TerminalBounds::new(
            px(14.0),
            px(7.0),
            Bounds {
                origin: Point::default(),
                size: Size {
                    width: px(500.0),
                    height: px(300.0),
                },
            },
        )
    }
}

impl From<TerminalBounds> for WindowSize {
    fn from(val: TerminalBounds) -> Self {
        WindowSize {
            num_lines: val.num_lines() as u16,
            num_cols: val.num_columns() as u16,
            cell_width: f32::from(val.cell_width()) as u16,
            cell_height: f32::from(val.line_height()) as u16,
        }
    }
}

impl Dimensions for TerminalBounds {
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }

    fn screen_lines(&self) -> usize {
        self.num_lines()
    }

    fn columns(&self) -> usize {
        self.num_columns()
    }
}

/// A cell with its grid position, for rendering.
#[derive(Debug, Clone)]
pub struct IndexedCell {
    pub point: AlacPoint,
    pub cell: Cell,
}

impl Deref for IndexedCell {
    type Target = Cell;

    #[inline]
    fn deref(&self) -> &Cell {
        &self.cell
    }
}

/// Snapshot of terminal state for rendering.
#[derive(Clone)]
pub struct TerminalContent {
    pub cells: Vec<IndexedCell>,
    pub mode: TermMode,
    pub display_offset: usize,
    pub selection: Option<SelectionRange>,
    pub cursor: RenderableCursor,
    pub cursor_char: char,
    pub terminal_bounds: TerminalBounds,
    /// Range of cells that are part of a hovered hyperlink (for styling).
    pub hovered_hyperlink: Option<Match>,
}

impl Default for TerminalContent {
    fn default() -> Self {
        TerminalContent {
            cells: Vec::new(),
            mode: TermMode::empty(),
            display_offset: 0,
            selection: None,
            cursor: RenderableCursor {
                shape: AlacCursorShape::Block,
                point: AlacPoint::new(Line(0), Column(0)),
            },
            cursor_char: ' ',
            terminal_bounds: TerminalBounds::default(),
            hovered_hyperlink: None,
        }
    }
}

/// Builder for creating a Terminal with a PTY.
pub struct TerminalBuilder {
    pub term: Arc<FairMutex<Term<ManifestListener>>>,
    pub pty_tx: Notifier,
    pub events_rx: UnboundedReceiver<AlacTermEvent>,
}

impl TerminalBuilder {
    /// Create a new terminal with a PTY connected to the user's default shell.
    pub fn new(working_directory: Option<PathBuf>, window_id: u64) -> Result<Self> {
        Self::new_with_shell(working_directory, window_id, None, vec![])
    }

    /// Create a new terminal with a PTY connected to a specific shell/command.
    pub fn new_with_shell(
        working_directory: Option<PathBuf>,
        window_id: u64,
        shell: Option<String>,
        args: Vec<String>,
    ) -> Result<Self> {
        let shell_config = shell.map(|program| tty::Shell::new(program, args));

        let pty_options = tty::Options {
            shell: shell_config,
            working_directory,
            drain_on_exit: true,
            env: std::env::vars().collect(),
        };

        let config = Config {
            scrolling_history: DEFAULT_SCROLL_HISTORY_LINES,
            default_cursor_style: CursorStyle {
                shape: AlacCursorShape::Block,
                blinking: false,
            },
            ..Config::default()
        };

        let (events_tx, events_rx) = unbounded();
        let listener = ManifestListener(events_tx);

        let pty = tty::new(&pty_options, TerminalBounds::default().into(), window_id)
            .context("Failed to create PTY")?;

        let term = Term::new(config, &TerminalBounds::default(), listener.clone());
        let term = Arc::new(FairMutex::new(term));

        let event_loop = EventLoop::new(
            term.clone(),
            listener,
            pty,
            false, // hold
            false, // ref_test
        )
        .context("Failed to create event loop")?;

        let pty_tx = Notifier(event_loop.channel());

        // Spawn the event loop in a background thread
        event_loop.spawn();

        Ok(TerminalBuilder {
            term,
            pty_tx,
            events_rx,
        })
    }

    /// Build the terminal entity and subscribe to events.
    pub fn build(self, cx: &mut Context<Terminal>) -> Terminal {
        let term = self.term;
        let pty_tx = self.pty_tx;
        let mut events_rx = self.events_rx;

        let event_loop_task = cx.spawn(async move |terminal, cx| {
            while let Some(event) = events_rx.next().await {
                terminal.update(cx, |terminal, cx| {
                    terminal.process_event(event, cx);
                })?;
            }
            anyhow::Ok(())
        });

        Terminal {
            term,
            pty_tx,
            last_content: TerminalContent::default(),
            event_loop_task,
            url_search: UrlSearch::new(),
            mouse_down_url: None,
            hovered_hyperlink: None,
        }
    }
}

/// The terminal entity that wraps alacritty_terminal.
pub struct Terminal {
    term: Arc<FairMutex<Term<ManifestListener>>>,
    pty_tx: Notifier,
    pub last_content: TerminalContent,
    #[allow(dead_code)]
    event_loop_task: Task<Result<(), anyhow::Error>>,
    /// URL search state for hyperlink detection.
    url_search: UrlSearch,
    /// URL that was under the cursor on mouse down (for Cmd+click).
    mouse_down_url: Option<String>,
    /// Currently hovered hyperlink range (when Cmd is held).
    hovered_hyperlink: Option<Match>,
}

impl EventEmitter<Event> for Terminal {}

impl Terminal {
    /// Get the current terminal content for rendering.
    pub fn last_content(&self) -> &TerminalContent {
        &self.last_content
    }

    /// Update the terminal size.
    pub fn set_size(&mut self, bounds: TerminalBounds) {
        let mut term = self.term.lock();
        term.resize(bounds);
        self.pty_tx
            .0
            .send(alacritty_terminal::event_loop::Msg::Resize(bounds.into()))
            .ok();
        self.last_content.terminal_bounds = bounds;
    }

    /// Write input to the PTY.
    pub fn input(&mut self, input: impl Into<Cow<'static, [u8]>>) {
        self.pty_tx.notify(input.into().into_owned());
    }

    /// Try to handle a keystroke, returning true if handled.
    pub fn try_keystroke(&mut self, keystroke: &Keystroke) -> bool {
        let mode = self.last_content.mode;
        if let Some(esc_str) = to_esc_str(keystroke, &mode, false) {
            self.input(esc_str.into_owned().into_bytes());
            true
        } else if let Some(key_char) = &keystroke.key_char {
            // For regular character input
            self.input(key_char.as_bytes().to_vec());
            true
        } else if keystroke.key.len() == 1
            && !keystroke.modifiers.control
            && !keystroke.modifiers.alt
        {
            // Single character key without modifiers
            self.input(keystroke.key.as_bytes().to_vec());
            true
        } else {
            false
        }
    }

    /// Get the terminal mode flags.
    pub fn mode(&self) -> TermMode {
        *self.term.lock().mode()
    }

    /// Process an event from alacritty.
    fn process_event(&mut self, event: AlacTermEvent, cx: &mut Context<Self>) {
        match event {
            AlacTermEvent::Wakeup => {
                self.sync_content();
                cx.emit(Event::Wakeup);
                cx.notify();
            }
            AlacTermEvent::Bell => {
                cx.emit(Event::Bell);
            }
            AlacTermEvent::Title(_) => {
                cx.emit(Event::TitleChanged);
            }
            AlacTermEvent::Exit => {
                cx.emit(Event::CloseTerminal);
            }
            AlacTermEvent::PtyWrite(text) => {
                self.input(text.into_bytes());
            }
            _ => {}
        }
    }

    /// Handle mouse down event. Returns true if the event was consumed (e.g., for hyperlink click).
    pub fn mouse_down(
        &mut self,
        button: MouseButton,
        position: Point<Pixels>,
        modifiers: Modifiers,
    ) -> bool {
        // Only handle Cmd+left click for hyperlinks
        if button != MouseButton::Left || !modifiers.platform {
            self.hovered_hyperlink = None;
            return false;
        }

        // Convert pixel position to grid point
        if let Some(point) = self.pixel_to_grid_point(position) {
            let term = self.term.lock();
            if let Some((url, match_range)) = find_url_at_point(&term, point, &mut self.url_search)
            {
                self.mouse_down_url = Some(url);
                self.hovered_hyperlink = Some(match_range);
                return true;
            }
        }
        self.mouse_down_url = None;
        self.hovered_hyperlink = None;
        false
    }

    /// Handle mouse up event. Returns true if a URL should be opened.
    pub fn mouse_up(
        &mut self,
        button: MouseButton,
        position: Point<Pixels>,
        modifiers: Modifiers,
        cx: &mut Context<Self>,
    ) -> bool {
        // Only handle left click release
        if button != MouseButton::Left {
            return false;
        }

        // Check if we had a URL on mouse down
        let mouse_down_url = self.mouse_down_url.take();
        if let Some(down_url) = mouse_down_url {
            // Verify we're still over the same URL (or any URL with Cmd held)
            if modifiers.platform {
                if let Some(point) = self.pixel_to_grid_point(position) {
                    let term = self.term.lock();
                    if let Some((up_url, _)) = find_url_at_point(&term, point, &mut self.url_search)
                    {
                        if up_url == down_url {
                            // Emit event to open the URL
                            drop(term);
                            cx.emit(Event::OpenUrl(down_url));
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Update hover state when mouse moves with Cmd held.
    pub fn mouse_move(&mut self, position: Point<Pixels>, modifiers: Modifiers) {
        if !modifiers.platform {
            if self.hovered_hyperlink.is_some() {
                self.hovered_hyperlink = None;
            }
            return;
        }

        // Convert pixel position to grid point
        if let Some(point) = self.pixel_to_grid_point(position) {
            let term = self.term.lock();
            if let Some((_url, match_range)) = find_url_at_point(&term, point, &mut self.url_search)
            {
                self.hovered_hyperlink = Some(match_range);
                return;
            }
        }
        self.hovered_hyperlink = None;
    }

    /// Get the currently hovered hyperlink range for rendering.
    pub fn hovered_hyperlink(&self) -> Option<&Match> {
        self.hovered_hyperlink.as_ref()
    }

    /// Clear the hovered hyperlink state.
    pub fn clear_hovered_hyperlink(&mut self) {
        self.hovered_hyperlink = None;
    }

    /// Convert a pixel position (relative to terminal bounds origin) to a grid point.
    fn pixel_to_grid_point(&self, position: Point<Pixels>) -> Option<AlacPoint> {
        let bounds = &self.last_content.terminal_bounds;

        // Check if position is within bounds
        if position.x < Pixels::ZERO || position.y < Pixels::ZERO {
            return None;
        }

        let col = (position.x / bounds.cell_width).floor() as i32;
        let line = (position.y / bounds.line_height).floor() as i32;

        // Clamp to valid range
        let num_cols = bounds.num_columns() as i32;
        let num_lines = bounds.num_lines() as i32;

        if col < 0 || col >= num_cols || line < 0 || line >= num_lines {
            return None;
        }

        // Account for display offset (scrollback)
        let display_offset = self.last_content.display_offset as i32;
        let adjusted_line = line - display_offset;

        Some(AlacPoint::new(Line(adjusted_line), Column(col as usize)))
    }

    /// Sync the content snapshot from the terminal grid.
    fn sync_content(&mut self) {
        let term = self.term.lock();

        let mut cells = Vec::new();
        let content = term.renderable_content();

        for cell in content.display_iter {
            cells.push(IndexedCell {
                point: cell.point,
                cell: cell.cell.clone(),
            });
        }

        self.last_content = TerminalContent {
            cells,
            mode: *term.mode(),
            display_offset: term.grid().display_offset(),
            selection: content.selection.map(|s| s.clone()),
            cursor: content.cursor,
            cursor_char: term.grid()[content.cursor.point].c,
            terminal_bounds: self.last_content.terminal_bounds,
            hovered_hyperlink: self.hovered_hyperlink.clone(),
        };
    }
}
