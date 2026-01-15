//! ResizeHandle component for draggable panel dividers
//!
//! Provides a visual drag handle between panels that allows resizing.
//! Follows Zed's patterns from pane_group.rs and dock.rs.

use gpui::*;

use crate::theme::Theme;

/// Axis for resize operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeAxis {
    /// Horizontal resize (drag left-right) - for sidebar width
    Horizontal,
    /// Vertical resize (drag up-down) - for terminal height
    Vertical,
}

/// Marker struct for drag operations - used to identify resize drags
#[derive(Debug, Clone)]
pub struct ResizeDrag {
    pub axis: ResizeAxis,
}

impl Render for ResizeDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Drag ghost - invisible during drag
        div()
    }
}

/// Handle size constants
const HANDLE_SIZE: f32 = 6.0;

/// A draggable resize handle component
pub struct ResizeHandle {
    id: ElementId,
    axis: ResizeAxis,
}

impl ResizeHandle {
    /// Create a new resize handle for the given axis
    pub fn new(id: impl Into<ElementId>, axis: ResizeAxis) -> Self {
        Self {
            id: id.into(),
            axis,
        }
    }
}

impl RenderOnce for ResizeHandle {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let axis = self.axis;
        let base = div()
            .id(self.id)
            .flex_shrink_0()
            .bg(Theme::panel_divider())
            .hover(|s| s.bg(Theme::panel_divider_hover()))
            .on_drag(ResizeDrag { axis }, |drag, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| drag.clone())
            })
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            });

        match axis {
            ResizeAxis::Horizontal => base
                .w(px(HANDLE_SIZE))
                .h_full()
                .cursor(CursorStyle::ResizeColumn),
            ResizeAxis::Vertical => base
                .w_full()
                .h(px(HANDLE_SIZE))
                .cursor(CursorStyle::ResizeRow),
        }
    }
}

impl IntoElement for ResizeHandle {
    type Element = Component<Self>;

    fn into_element(self) -> Self::Element {
        Component::new(self)
    }
}
