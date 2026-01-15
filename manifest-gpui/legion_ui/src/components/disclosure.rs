//! Disclosure component for expand/collapse toggles
//!
//! A chevron icon that toggles between expanded (▼) and collapsed (▶) states.
//! Currently unused (ListItem handles toggles inline) but kept for future use.

#![allow(dead_code)]

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::theme::Theme;

/// A disclosure toggle (chevron) for expandable tree items
pub struct Disclosure {
    id: ElementId,
    is_open: bool,
    on_toggle: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl Disclosure {
    /// Create a new disclosure toggle
    pub fn new(id: impl Into<ElementId>, is_open: bool) -> Self {
        Self {
            id: id.into(),
            is_open,
            on_toggle: None,
        }
    }

    /// Set the toggle handler
    pub fn on_toggle(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Disclosure {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let icon = if self.is_open { "▼" } else { "▶" };

        div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_center()
            .w(px(16.0))
            .h(px(16.0))
            .text_size(px(10.0))
            .text_color(Theme::text_muted())
            .cursor_pointer()
            .hover(|style| style.text_color(Theme::text()))
            .when_some(self.on_toggle, |el, handler| {
                el.on_click(move |event, window, cx| handler(event, window, cx))
            })
            .child(icon)
    }
}
