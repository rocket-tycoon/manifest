//! Terminal events.

use crate::bounds::TerminalBounds;

/// Events emitted by the terminal for the UI to handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Terminal title changed (from shell escape sequence).
    TitleChanged,
    /// Terminal requests close.
    CloseTerminal,
    /// Bell character received.
    Bell,
    /// Terminal content changed, needs redraw.
    Wakeup,
}

/// Internal events for terminal state management.
#[derive(Clone, Debug)]
pub enum InternalEvent {
    /// Resize terminal to new bounds.
    Resize(TerminalBounds),
    /// Clear terminal screen.
    Clear,
    /// Scroll by lines or pages.
    Scroll(alacritty_terminal::grid::Scroll),
}
