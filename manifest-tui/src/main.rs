//! Manifest TUI - Terminal UI for living feature documentation.
//!
//! A minimal proof-of-concept that demonstrates spawning Claude Code
//! in tmux panes from a Ratatui sidebar.

mod app;
mod spawn;
mod tmux;
mod ui;

use std::io;
use std::os::unix::process::CommandExt;
use std::process::Command;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::App;

fn main() -> io::Result<()> {
    // Auto-launch tmux if not already inside it
    if !spawn::in_tmux() {
        let exe = std::env::current_exe()?;
        let err = Command::new("tmux")
            .args([
                "new-session",
                "-A",
                "-s",
                "manifest",
                &exe.to_string_lossy(),
            ])
            .exec();
        // exec() only returns on error
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to start tmux: {}", err),
        ));
    }

    // Enable tmux mouse support (click to switch panes)
    let _ = tmux::enable_mouse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }

    Ok(())
}

/// Main event loop.
fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if let Event::Key(key) = event::read()? {
            // Only handle key press events (not release)
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Esc => app.quit(),
                KeyCode::Enter => app.spawn_selected(),
                KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
                KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
