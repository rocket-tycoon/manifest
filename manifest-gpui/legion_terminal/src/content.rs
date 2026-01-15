//! Terminal content snapshot for rendering.

use std::ops::Deref;

use alacritty_terminal::index::{Column, Line, Point as AlacPoint};
use alacritty_terminal::term::cell::Cell;
use alacritty_terminal::term::{RenderableCursor, TermMode};
use alacritty_terminal::vte::ansi::CursorShape;

use crate::bounds::TerminalBounds;

/// A cell with its position in the terminal grid.
#[derive(Clone, Debug)]
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
///
/// This is a simplified version that omits selection and hyperlink
/// highlighting for the initial implementation.
#[derive(Clone)]
pub struct TerminalContent {
    /// All visible cells in the terminal grid.
    pub cells: Vec<IndexedCell>,
    /// Terminal mode flags (cursor visibility, app cursor mode, etc.)
    pub mode: TermMode,
    /// Current scroll offset from the bottom.
    pub display_offset: usize,
    /// Cursor position and shape.
    pub cursor: RenderableCursor,
    /// Character under the cursor.
    pub cursor_char: char,
    /// Terminal dimensions.
    pub terminal_bounds: TerminalBounds,
}

impl Default for TerminalContent {
    fn default() -> Self {
        TerminalContent {
            cells: Vec::new(),
            mode: TermMode::empty(),
            display_offset: 0,
            cursor: RenderableCursor {
                shape: CursorShape::Block,
                point: AlacPoint::new(Line(0), Column(0)),
            },
            cursor_char: ' ',
            terminal_bounds: TerminalBounds::default(),
        }
    }
}
