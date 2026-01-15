//! Manifest Terminal Application
//!
//! A standalone GPUI application with a single terminal window.

use gpui::{
    actions, App, Application, Bounds, Context, Entity, Focusable, KeyBinding, Render,
    TitlebarOptions, Window, WindowBounds, WindowOptions, div, point, prelude::*, px, size,
};
use terminal_view::TerminalView;
use terminal::mappings::colors::TerminalColors;

actions!(app, [Quit]);

/// Root application view that contains the terminal.
struct ManifestApp {
    terminal_view: Entity<TerminalView>,
}

impl ManifestApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let terminal_view = cx.new(|cx| TerminalView::new(window, cx));

        // Focus the terminal on startup
        let focus_handle = terminal_view.focus_handle(cx);
        focus_handle.focus(window, cx);

        ManifestApp { terminal_view }
    }
}

impl Render for ManifestApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let bg_color: gpui::Hsla = TerminalColors::background().into();

        div()
            .id("manifest-app")
            .size_full()
            .bg(bg_color)
            .child(self.terminal_view.clone())
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        // Register global actions
        cx.on_action(|_: &Quit, cx| cx.quit());

        // Bind keys
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
        ]);

        // Open the main window
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(100.0), px(100.0)),
                size: size(px(800.0), px(600.0)),
            })),
            titlebar: Some(TitlebarOptions {
                title: Some("Manifest Terminal".into()),
                appears_transparent: false,
                ..Default::default()
            }),
            focus: true,
            show: true,
            kind: gpui::WindowKind::Normal,
            is_movable: true,
            app_id: Some("com.manifest.terminal".into()),
            ..Default::default()
        };

        cx.open_window(window_options, |window, cx| {
            cx.new(|cx| ManifestApp::new(window, cx))
        }).ok();

        cx.activate(true);
    });
}
