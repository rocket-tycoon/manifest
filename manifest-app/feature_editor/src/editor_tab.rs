use std::ops::Range;

use gpui::SharedString;
use uuid::Uuid;

/// Position within multi-line text content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct CursorPosition {
    /// 0-indexed line number.
    pub line: usize,
    /// Byte offset within the line.
    pub column: usize,
}

impl CursorPosition {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    /// Clamp the column to the line's length.
    pub fn clamp_to_line(&self, line_len: usize) -> Self {
        Self {
            line: self.line,
            column: self.column.min(line_len),
        }
    }
}

/// State for a single feature editor tab.
pub struct FeatureEditorTab {
    /// Unique ID for this tab (for GPUI element IDs).
    pub id: usize,
    /// The feature being edited.
    pub feature_id: Uuid,
    /// Feature title (displayed in tab).
    pub title: String,
    /// Original content from server (for dirty detection).
    pub original_content: String,
    /// Content split by lines for efficient multi-line editing.
    pub lines: Vec<String>,
    /// Current cursor position.
    pub cursor: CursorPosition,
    /// Selection anchor (if selecting). None means no active selection.
    pub selection_anchor: Option<CursorPosition>,
    /// Vertical scroll offset in pixels.
    pub scroll_offset: f32,
    /// True if content has been modified since last save.
    pub is_dirty: bool,
    /// Marked text range for IME composition (byte offset within flattened content).
    pub marked_range: Option<Range<usize>>,
}

impl FeatureEditorTab {
    /// Create a new tab for editing a feature.
    pub fn new(id: usize, feature_id: Uuid, title: String, details: Option<String>) -> Self {
        let content = details.unwrap_or_default();
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(String::from).collect()
        };

        Self {
            id,
            feature_id,
            title,
            original_content: content,
            lines,
            cursor: CursorPosition::default(),
            selection_anchor: None,
            scroll_offset: 0.0,
            is_dirty: false,
            marked_range: None,
        }
    }

    /// Get the full content as a single string.
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    /// Get the content as a SharedString for rendering.
    pub fn content_shared(&self) -> SharedString {
        self.content().into()
    }

    /// Set the content and update dirty state.
    pub fn set_content(&mut self, content: &str) {
        self.lines = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(String::from).collect()
        };
        self.update_dirty();
        self.clamp_cursor();
    }

    /// Update the dirty flag based on content comparison.
    fn update_dirty(&mut self) {
        self.is_dirty = self.content() != self.original_content;
    }

    /// Clamp cursor to valid bounds.
    fn clamp_cursor(&mut self) {
        let max_line = self.lines.len().saturating_sub(1);
        self.cursor.line = self.cursor.line.min(max_line);
        if let Some(line) = self.lines.get(self.cursor.line) {
            self.cursor.column = self.cursor.column.min(line.len());
        }
    }

    /// Mark content as saved (reset dirty state).
    pub fn mark_saved(&mut self) {
        self.original_content = self.content();
        self.is_dirty = false;
    }

    /// Get the current line text.
    pub fn current_line(&self) -> &str {
        self.lines.get(self.cursor.line).map(|s| s.as_str()).unwrap_or("")
    }

    /// Get the length of the current line.
    pub fn current_line_len(&self) -> usize {
        self.current_line().len()
    }

    /// Get the total number of lines.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Convert a CursorPosition to a byte offset in the flattened content.
    pub fn position_to_offset(&self, pos: CursorPosition) -> usize {
        let mut offset = 0;
        for (i, line) in self.lines.iter().enumerate() {
            if i == pos.line {
                return offset + pos.column.min(line.len());
            }
            offset += line.len() + 1; // +1 for newline
        }
        offset.saturating_sub(1) // Handle end of content
    }

    /// Convert a byte offset to a CursorPosition.
    pub fn offset_to_position(&self, offset: usize) -> CursorPosition {
        let mut remaining = offset;
        for (line_idx, line) in self.lines.iter().enumerate() {
            let line_len_with_newline = line.len() + 1;
            if remaining < line_len_with_newline || line_idx == self.lines.len() - 1 {
                return CursorPosition {
                    line: line_idx,
                    column: remaining.min(line.len()),
                };
            }
            remaining -= line_len_with_newline;
        }
        CursorPosition::default()
    }

    /// Get the selected range as byte offsets, if any.
    pub fn selected_range(&self) -> Option<Range<usize>> {
        self.selection_anchor.map(|anchor| {
            let start = self.position_to_offset(anchor.min(self.cursor));
            let end = self.position_to_offset(anchor.max(self.cursor));
            start..end
        })
    }

    /// Get the selected range as CursorPositions, ordered (start <= end).
    pub fn selection_bounds(&self) -> Option<(CursorPosition, CursorPosition)> {
        self.selection_anchor.map(|anchor| {
            if anchor <= self.cursor {
                (anchor, self.cursor)
            } else {
                (self.cursor, anchor)
            }
        })
    }

    /// Check if there is an active selection.
    pub fn has_selection(&self) -> bool {
        self.selection_anchor.is_some() && self.selection_anchor != Some(self.cursor)
    }

    /// Clear the selection.
    pub fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    /// Start or extend selection to current cursor.
    pub fn start_selection(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
    }

    /// Insert text at cursor position, replacing selection if any.
    pub fn insert_text(&mut self, text: &str) {
        // Delete selected text first if any
        if self.has_selection() {
            self.delete_selection();
        }

        // Handle multi-line insertion
        let insert_lines: Vec<&str> = text.split('\n').collect();

        if insert_lines.len() == 1 {
            // Single line insert
            if let Some(line) = self.lines.get_mut(self.cursor.line) {
                let col = self.cursor.column.min(line.len());
                line.insert_str(col, text);
                self.cursor.column = col + text.len();
            }
        } else {
            // Multi-line insert
            let current_line = self.lines.get(self.cursor.line).cloned().unwrap_or_default();
            let col = self.cursor.column.min(current_line.len());
            let (before, after) = current_line.split_at(col);

            // First part: before cursor + first insert line
            let first_line = format!("{}{}", before, insert_lines[0]);

            // Last part: last insert line + after cursor
            let last_line = format!("{}{}", insert_lines.last().unwrap_or(&""), after);

            // Build new lines
            let mut new_lines = vec![first_line];
            for insert_line in &insert_lines[1..insert_lines.len() - 1] {
                new_lines.push(insert_line.to_string());
            }
            new_lines.push(last_line.clone());

            // Replace current line with new lines
            self.lines.splice(self.cursor.line..=self.cursor.line, new_lines);

            // Update cursor position
            self.cursor.line += insert_lines.len() - 1;
            self.cursor.column = insert_lines.last().map(|s| s.len()).unwrap_or(0);
        }

        self.clear_selection();
        self.update_dirty();
    }

    /// Delete the selected text.
    pub fn delete_selection(&mut self) {
        if let Some((start, end)) = self.selection_bounds() {
            if start.line == end.line {
                // Same line deletion
                if let Some(line) = self.lines.get_mut(start.line) {
                    let start_col = start.column.min(line.len());
                    let end_col = end.column.min(line.len());
                    line.drain(start_col..end_col);
                }
            } else {
                // Multi-line deletion
                let start_line = self.lines.get(start.line).cloned().unwrap_or_default();
                let end_line = self.lines.get(end.line).cloned().unwrap_or_default();

                let before = &start_line[..start.column.min(start_line.len())];
                let after = &end_line[end.column.min(end_line.len())..];

                let merged = format!("{}{}", before, after);

                // Remove lines and replace with merged
                self.lines.drain(start.line..=end.line);
                self.lines.insert(start.line, merged);
            }

            self.cursor = start;
            self.clear_selection();
            self.update_dirty();
        }
    }

    /// Delete character before cursor (backspace).
    pub fn backspace(&mut self) {
        if self.has_selection() {
            self.delete_selection();
            return;
        }

        if self.cursor.column > 0 {
            // Compute boundary first (immutable borrow)
            let prev_boundary = {
                let line = self.lines.get(self.cursor.line).map(|s| s.as_str()).unwrap_or("");
                self.previous_grapheme_boundary(self.cursor.column, line)
            };
            // Now mutate (mutable borrow in separate scope)
            if let Some(line) = self.lines.get_mut(self.cursor.line) {
                line.drain(prev_boundary..self.cursor.column);
            }
            self.cursor.column = prev_boundary;
        } else if self.cursor.line > 0 {
            // Merge with previous line
            let current_line = self.lines.remove(self.cursor.line);
            self.cursor.line -= 1;
            if let Some(prev_line) = self.lines.get_mut(self.cursor.line) {
                self.cursor.column = prev_line.len();
                prev_line.push_str(&current_line);
            }
        }
        self.update_dirty();
    }

    /// Delete character at cursor (delete key).
    pub fn delete(&mut self) {
        if self.has_selection() {
            self.delete_selection();
            return;
        }

        // Check what action to take (immutable borrow)
        let line_len = self.lines.get(self.cursor.line).map(|s| s.len()).unwrap_or(0);
        let total_lines = self.lines.len();

        if self.cursor.column < line_len {
            // Compute boundary first (immutable borrow)
            let next_boundary = {
                let line = self.lines.get(self.cursor.line).map(|s| s.as_str()).unwrap_or("");
                self.next_grapheme_boundary(self.cursor.column, line)
            };
            // Now mutate (mutable borrow in separate scope)
            if let Some(line) = self.lines.get_mut(self.cursor.line) {
                line.drain(self.cursor.column..next_boundary);
            }
        } else if self.cursor.line < total_lines - 1 {
            // Merge with next line
            let next_line = self.lines.remove(self.cursor.line + 1);
            if let Some(current_line) = self.lines.get_mut(self.cursor.line) {
                current_line.push_str(&next_line);
            }
        }
        self.update_dirty();
    }

    /// Insert a newline at cursor.
    pub fn insert_newline(&mut self) {
        if self.has_selection() {
            self.delete_selection();
        }

        let current_line = self.lines.get(self.cursor.line).cloned().unwrap_or_default();
        let col = self.cursor.column.min(current_line.len());

        let (before, after) = current_line.split_at(col);
        self.lines[self.cursor.line] = before.to_string();
        self.lines.insert(self.cursor.line + 1, after.to_string());

        self.cursor.line += 1;
        self.cursor.column = 0;
        self.update_dirty();
    }

    // --- Cursor movement ---

    /// Move cursor left by one grapheme.
    pub fn move_left(&mut self) {
        self.clear_selection();
        if self.cursor.column > 0 {
            let line = self.current_line();
            self.cursor.column = self.previous_grapheme_boundary(self.cursor.column, line);
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.column = self.current_line_len();
        }
    }

    /// Move cursor right by one grapheme.
    pub fn move_right(&mut self) {
        self.clear_selection();
        let line_len = self.current_line_len();
        if self.cursor.column < line_len {
            let line = self.current_line();
            self.cursor.column = self.next_grapheme_boundary(self.cursor.column, line);
        } else if self.cursor.line < self.lines.len() - 1 {
            self.cursor.line += 1;
            self.cursor.column = 0;
        }
    }

    /// Move cursor up one line.
    pub fn move_up(&mut self) {
        self.clear_selection();
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            // Clamp column to new line length
            self.cursor.column = self.cursor.column.min(self.current_line_len());
        }
    }

    /// Move cursor down one line.
    pub fn move_down(&mut self) {
        self.clear_selection();
        if self.cursor.line < self.lines.len() - 1 {
            self.cursor.line += 1;
            // Clamp column to new line length
            self.cursor.column = self.cursor.column.min(self.current_line_len());
        }
    }

    /// Move cursor to start of line.
    pub fn move_to_line_start(&mut self) {
        self.clear_selection();
        self.cursor.column = 0;
    }

    /// Move cursor to end of line.
    pub fn move_to_line_end(&mut self) {
        self.clear_selection();
        self.cursor.column = self.current_line_len();
    }

    /// Move cursor to start of document.
    pub fn move_to_start(&mut self) {
        self.clear_selection();
        self.cursor = CursorPosition::default();
    }

    /// Move cursor to end of document.
    pub fn move_to_end(&mut self) {
        self.clear_selection();
        self.cursor.line = self.lines.len().saturating_sub(1);
        self.cursor.column = self.current_line_len();
    }

    /// Select all content.
    pub fn select_all(&mut self) {
        self.cursor = CursorPosition::default();
        self.selection_anchor = Some(CursorPosition::default());
        self.cursor.line = self.lines.len().saturating_sub(1);
        self.cursor.column = self.current_line_len();
    }

    // --- Grapheme boundary helpers ---

    fn previous_grapheme_boundary(&self, offset: usize, text: &str) -> usize {
        use unicode_segmentation::UnicodeSegmentation;
        text.grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| if idx < offset { Some(idx) } else { None })
            .unwrap_or(0)
    }

    fn next_grapheme_boundary(&self, offset: usize, text: &str) -> usize {
        use unicode_segmentation::UnicodeSegmentation;
        text.grapheme_indices(true)
            .find_map(|(idx, _)| if idx > offset { Some(idx) } else { None })
            .unwrap_or(text.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_position_ordering() {
        let a = CursorPosition::new(0, 5);
        let b = CursorPosition::new(1, 0);
        assert!(a < b);

        let c = CursorPosition::new(1, 3);
        let d = CursorPosition::new(1, 5);
        assert!(c < d);
    }

    #[test]
    fn test_insert_single_line() {
        let mut tab = FeatureEditorTab::new(0, Uuid::new_v4(), "Test".into(), Some("Hello".into()));
        tab.cursor.column = 5;
        tab.insert_text(" World");
        assert_eq!(tab.content(), "Hello World");
        assert_eq!(tab.cursor.column, 11);
    }

    #[test]
    fn test_insert_newline() {
        let mut tab = FeatureEditorTab::new(0, Uuid::new_v4(), "Test".into(), Some("Hello World".into()));
        tab.cursor.column = 5;
        tab.insert_newline();
        assert_eq!(tab.content(), "Hello\n World");
        assert_eq!(tab.cursor.line, 1);
        assert_eq!(tab.cursor.column, 0);
    }

    #[test]
    fn test_backspace_within_line() {
        let mut tab = FeatureEditorTab::new(0, Uuid::new_v4(), "Test".into(), Some("Hello".into()));
        tab.cursor.column = 5;
        tab.backspace();
        assert_eq!(tab.content(), "Hell");
    }

    #[test]
    fn test_backspace_merge_lines() {
        let mut tab = FeatureEditorTab::new(0, Uuid::new_v4(), "Test".into(), Some("Hello\nWorld".into()));
        tab.cursor.line = 1;
        tab.cursor.column = 0;
        tab.backspace();
        assert_eq!(tab.content(), "HelloWorld");
        assert_eq!(tab.cursor.line, 0);
        assert_eq!(tab.cursor.column, 5);
    }

    #[test]
    fn test_dirty_state() {
        let mut tab = FeatureEditorTab::new(0, Uuid::new_v4(), "Test".into(), Some("Hello".into()));
        assert!(!tab.is_dirty);
        tab.insert_text("!");
        assert!(tab.is_dirty);
        tab.mark_saved();
        assert!(!tab.is_dirty);
    }
}
