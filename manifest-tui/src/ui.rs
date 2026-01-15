//! UI rendering with Ratatui.
//!
//! Design: Minimal black and white aesthetic. No colored borders.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;

/// Render the application UI.
pub fn render(frame: &mut Frame, app: &App) {
    // Main layout: content area + status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Main content
            Constraint::Length(1), // Status bar (single line, no border)
        ])
        .split(frame.area());

    render_main(frame, app, chunks[0]);
    render_status(frame, app, chunks[1]);
}

/// Render the main content area.
fn render_main(frame: &mut Frame, app: &App, area: Rect) {
    // Build list items
    let items: Vec<ListItem> = app
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == app.selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            ListItem::new(format!("  {}", item.label)).style(style)
        })
        .collect();

    let list = List::new(items).block(Block::default().title(" manifest ").borders(Borders::ALL));

    frame.render_widget(list, area);
}

/// Render the status bar.
fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let status_text = if let Some(ref err) = app.last_error {
        Line::from(vec![
            Span::raw("error: "),
            Span::styled(err.as_str(), Style::default().add_modifier(Modifier::DIM)),
        ])
    } else if let Some(ref spawned) = app.last_spawn {
        Line::from(vec![Span::raw("spawned: "), Span::raw(spawned.as_str())])
    } else {
        Line::from(vec![
            Span::styled("enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" spawn  "),
            Span::styled("esc", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" quit"),
        ])
    };

    let status = Paragraph::new(status_text);
    frame.render_widget(status, area);
}
