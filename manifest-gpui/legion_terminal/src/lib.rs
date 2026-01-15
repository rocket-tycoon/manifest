//! Terminal emulator for Legion.
//!
//! Wraps alacritty_terminal to provide PTY-based terminal emulation
//! integrated with GPUI.

pub mod bounds;
pub mod content;
pub mod event;
pub mod mappings;
mod terminal;

pub use bounds::TerminalBounds;
pub use content::{IndexedCell, TerminalContent};
pub use event::{Event, InternalEvent};
pub use terminal::{Terminal, TerminalBuilder};
