//! TerminalView - GPUI view container for the terminal.

use gpui::{
    App, AsyncWindowContext, Context, Entity, EventEmitter, Focusable, FocusHandle, IntoElement,
    KeyDownEvent, Render, WeakEntity, Window, div, prelude::*,
};
use terminal::{Event as TerminalEvent, Terminal, TerminalBuilder, mappings::colors::TerminalColors};

use crate::TerminalElement;

/// Events emitted by the TerminalView.
#[derive(Clone, Debug)]
pub enum Event {
    TitleChanged,
    Closed,
}

/// GPUI view that contains and renders a Terminal.
pub struct TerminalView {
    terminal: Option<Entity<Terminal>>,
    focus_handle: FocusHandle,
}

impl TerminalView {
    /// Create a new terminal view with a PTY connected to the default shell.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Spawn the terminal creation in the background
        let task = cx.background_executor().spawn(async {
            TerminalBuilder::new(None, 0)
        });

        cx.spawn_in(window, async move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
            match task.await {
                Ok(builder) => {
                    this.update_in(cx, |this, _window, cx| {
                        let terminal = cx.new(|cx| builder.build(cx));
                        this.subscribe_to_terminal(&terminal, cx);
                        this.terminal = Some(terminal);
                        cx.notify();
                    }).ok();
                }
                Err(e) => {
                    eprintln!("Failed to create terminal: {}", e);
                }
            }
        }).detach();

        TerminalView {
            terminal: None,
            focus_handle,
        }
    }

    /// Create a terminal view from an existing Terminal entity.
    pub fn from_terminal(terminal: Entity<Terminal>, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let mut view = TerminalView {
            terminal: Some(terminal.clone()),
            focus_handle,
        };
        view.subscribe_to_terminal(&terminal, cx);
        view
    }

    fn subscribe_to_terminal(&mut self, terminal: &Entity<Terminal>, cx: &mut Context<Self>) {
        cx.subscribe(terminal, |_this, _terminal, event: &TerminalEvent, cx| {
            match event {
                TerminalEvent::Wakeup => {
                    cx.notify();
                }
                TerminalEvent::Bell => {
                    // Could play a sound or flash the window
                }
                TerminalEvent::TitleChanged => {
                    cx.emit(Event::TitleChanged);
                }
                TerminalEvent::CloseTerminal => {
                    cx.emit(Event::Closed);
                }
            }
        }).detach();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(terminal) = &self.terminal {
            terminal.update(cx, |terminal, _cx| {
                terminal.try_keystroke(&event.keystroke);
            });
        }
    }
}

impl EventEmitter<Event> for TerminalView {}

impl Focusable for TerminalView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus_handle.is_focused(window);
        let bg_color: gpui::Hsla = TerminalColors::background().into();

        div()
            .id("terminal-view")
            .size_full()
            .bg(bg_color)
            .track_focus(&self.focus_handle)
            .key_context("Terminal")
            .on_key_down(cx.listener(Self::on_key_down))
            .child(if let Some(terminal) = &self.terminal {
                TerminalElement::new(
                    terminal.clone(),
                    self.focus_handle.clone(),
                    focused,
                ).into_any_element()
            } else {
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(gpui::rgb(0xd4d4d4))
                    .child("Starting terminal...")
                    .into_any_element()
            })
    }
}
