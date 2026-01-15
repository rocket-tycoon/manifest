//! Manifest GPUI - Hello World with Terminal
//!
//! Minimal GPUI app that spawns Claude in a terminal.

use gpui::*;
use legion_terminal::Terminal;
use legion_ui::terminal::TerminalView;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting Manifest GPUI");

    Application::new().run(|cx: &mut App| {
        // Bind quit shortcut
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.on_action(|_: &Quit, cx| cx.quit());

        // Create main window
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1000.0), px(700.0)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Manifest".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| ManifestApp::new(window, cx)),
        )
        .expect("Failed to open window");

        cx.activate(true);
    });
}

actions!(manifest, [Quit]);

struct ManifestApp {
    terminal_view: Option<Entity<TerminalView>>,
}

impl ManifestApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Spawn Claude in background and set up terminal when ready
        let task = cx.background_executor().spawn(async move {
            Terminal::create_pty_with_shell(
                None, // working directory
                0,    // window_id
                Some("claude".to_string()),
                vec!["Hello from Manifest GPUI!".to_string()],
            )
        });

        cx.spawn_in(window, async move |this, cx| {
            match task.await {
                Ok(builder) => {
                    this.update(cx, |app, cx| {
                        let terminal = cx.new(|cx| builder.build(cx));
                        let view = cx.new(|cx| TerminalView::from_terminal_entity(terminal, cx));
                        app.terminal_view = Some(view);
                        cx.notify();
                    })?;
                }
                Err(e) => {
                    tracing::error!("Failed to create terminal: {}", e);
                }
            }
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);

        Self {
            terminal_view: None,
        }
    }
}

impl Render for ManifestApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .text_color(rgb(0xd4d4d4))
            .child(
                div()
                    .size_full()
                    .p_2()
                    .child(if let Some(ref view) = self.terminal_view {
                        view.clone().into_any_element()
                    } else {
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child("Starting Claude...")
                            .into_any_element()
                    }),
            )
    }
}
