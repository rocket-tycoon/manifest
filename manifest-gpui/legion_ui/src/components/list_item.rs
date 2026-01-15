//! ListItem component for tree views and lists
//!
//! A flexible list item with support for indentation, selection states,
//! disclosure toggles, and icon slots.

// Some builder methods and variants are defined for API completeness but not yet used
#![allow(dead_code)]

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::theme::Theme;

/// Spacing options for list items
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum ListItemSpacing {
    /// Dense spacing (24px height)
    Dense,
    /// Default spacing (28px height)
    #[default]
    Default,
    /// Comfortable spacing (32px height)
    Comfortable,
}

impl ListItemSpacing {
    fn height(self) -> Pixels {
        match self {
            Self::Dense => px(24.0),
            Self::Default => px(28.0),
            Self::Comfortable => px(32.0),
        }
    }
}

/// A list item component for tree views
pub struct ListItem {
    id: ElementId,
    indent_level: usize,
    indent_step_size: Pixels,
    spacing: ListItemSpacing,
    selected: bool,
    disabled: bool,
    toggle: Option<bool>,
    start_slot: Option<AnyElement>,
    children: Vec<AnyElement>,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    on_toggle: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl ListItem {
    /// Create a new list item
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            indent_level: 0,
            indent_step_size: px(12.0),
            spacing: ListItemSpacing::Default,
            selected: false,
            disabled: false,
            toggle: None,
            start_slot: None,
            children: Vec::new(),
            on_click: None,
            on_toggle: None,
        }
    }

    /// Set the indentation level (0 = root, 1 = first level, etc.)
    pub fn indent_level(mut self, level: usize) -> Self {
        self.indent_level = level;
        self
    }

    /// Set the indentation step size (default: 12px)
    pub fn indent_step_size(mut self, size: Pixels) -> Self {
        self.indent_step_size = size;
        self
    }

    /// Set the spacing/height
    pub fn spacing(mut self, spacing: ListItemSpacing) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set whether this item is selected
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Set whether this item is disabled
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the toggle state (Some = show disclosure, None = no disclosure)
    pub fn toggle(mut self, is_open: Option<bool>) -> Self {
        self.toggle = is_open;
        self
    }

    /// Set the start slot content (typically an icon)
    pub fn start_slot(mut self, element: impl IntoElement) -> Self {
        self.start_slot = Some(element.into_any_element());
        self
    }

    /// Add a child element (typically the label)
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    /// Set the click handler
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Set the toggle handler (for disclosure)
    pub fn on_toggle(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for ListItem {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let indent = px(self.indent_level as f32 * 12.0); // 12px per level
        let height = self.spacing.height();

        // Determine background color based on state
        let bg_color = if self.selected {
            Theme::ghost_element_selected()
        } else {
            Theme::transparent()
        };

        let hover_bg = if self.selected {
            Theme::ghost_element_selected()
        } else {
            Theme::ghost_element_hover()
        };

        let text_color = if self.disabled {
            Theme::text_disabled()
        } else {
            Theme::text()
        };

        // Render the disclosure toggle inline
        let toggle_state = self.toggle;
        let on_toggle = self.on_toggle;

        div()
            .id(self.id.clone())
            .h(height)
            .w_full()
            .pl(indent + px(4.0)) // Base padding + indent
            .pr(px(8.0))
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .bg(bg_color)
            .text_color(text_color)
            .rounded_sm()
            .when(!self.disabled, |el| {
                el.cursor_pointer()
                    .hover(|style| style.bg(hover_bg))
                    .active(|style| style.bg(Theme::ghost_element_active()))
            })
            .when_some(self.on_click, |el, handler| {
                el.on_click(move |event, window, cx| handler(event, window, cx))
            })
            // Disclosure toggle (rendered inline)
            .when_some(toggle_state, |el, is_open| {
                let icon = if is_open { "▼" } else { "▶" };
                el.child(
                    div()
                        .id("toggle")
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(16.0))
                        .h(px(16.0))
                        .text_size(px(10.0))
                        .text_color(Theme::text_muted())
                        .cursor_pointer()
                        .hover(|style| style.text_color(Theme::text()))
                        .when_some(on_toggle, |el, handler| {
                            el.on_click(move |event, window, cx| handler(event, window, cx))
                        })
                        .child(icon),
                )
            })
            // Placeholder for disclosure width when no toggle
            .when(toggle_state.is_none(), |el| el.child(div().w(px(16.0))))
            // Start slot (icon)
            .when_some(self.start_slot, |el, slot| {
                el.child(div().flex().items_center().justify_center().child(slot))
            })
            // Children (label)
            .children(self.children)
    }
}

impl IntoElement for ListItem {
    type Element = Component<Self>;

    fn into_element(self) -> Self::Element {
        Component::new(self)
    }
}
