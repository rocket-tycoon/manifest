//! Vertical scrollbar component for the feature editor.

use gpui::{
    div, px, CursorStyle, ElementId, InteractiveElement, IntoElement,
    ParentElement, Styled,
};

/// Minimum thumb size to ensure it's clickable.
const MINIMUM_THUMB_SIZE: f32 = 25.0;
/// Scrollbar width.
const SCROLLBAR_WIDTH: f32 = 10.0;
/// Padding from edge.
const SCROLLBAR_PADDING: f32 = 2.0;

/// Scrollbar colors (Pigs in Space theme).
mod colors {
    use gpui::Hsla;

    pub fn track() -> Hsla {
        Hsla { h: 210.0 / 360.0, s: 0.10, l: 0.12, a: 0.5 }
    }

    pub fn thumb() -> Hsla {
        Hsla { h: 210.0 / 360.0, s: 0.15, l: 0.35, a: 0.8 }
    }

    pub fn thumb_hover() -> Hsla {
        Hsla { h: 210.0 / 360.0, s: 0.20, l: 0.45, a: 0.9 }
    }

    pub fn thumb_active() -> Hsla {
        Hsla { h: 220.0 / 360.0, s: 0.30, l: 0.55, a: 1.0 }
    }
}

/// State for scrollbar interaction.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ScrollbarState {
    #[default]
    Inactive,
    Hovered,
    Dragging { start_offset: f32 },
}

/// Scrollbar metrics calculated from content.
#[derive(Clone, Copy, Debug)]
pub struct ScrollbarMetrics {
    /// Total content height in pixels.
    pub content_height: f32,
    /// Visible viewport height in pixels.
    pub viewport_height: f32,
    /// Current scroll offset (0 = top).
    pub scroll_offset: f32,
}

impl ScrollbarMetrics {
    /// Calculate whether scrollbar is needed.
    pub fn needs_scrollbar(&self) -> bool {
        self.content_height > self.viewport_height
    }

    /// Calculate maximum scroll offset.
    pub fn max_scroll(&self) -> f32 {
        (self.content_height - self.viewport_height).max(0.0)
    }

    /// Calculate thumb size as a fraction of viewport (0.0-1.0).
    pub fn thumb_fraction(&self) -> f32 {
        if self.content_height <= 0.0 {
            return 1.0;
        }
        (self.viewport_height / self.content_height).clamp(0.0, 1.0)
    }

    /// Calculate thumb position as a fraction (0.0-1.0).
    pub fn thumb_position(&self) -> f32 {
        let max = self.max_scroll();
        if max <= 0.0 {
            return 0.0;
        }
        (self.scroll_offset / max).clamp(0.0, 1.0)
    }

    /// Convert a click position (0.0-1.0) to a scroll offset.
    pub fn position_to_offset(&self, position: f32, thumb_fraction: f32) -> f32 {
        let max = self.max_scroll();
        let usable_range = 1.0 - thumb_fraction;
        if usable_range <= 0.0 {
            return 0.0;
        }
        let scroll_fraction = (position / usable_range).clamp(0.0, 1.0);
        scroll_fraction * max
    }
}

/// Render a vertical scrollbar.
///
/// The scrollbar is purely visual - scrolling is handled by the parent component
/// via scroll wheel events. Future enhancement: add drag-to-scroll interaction.
pub fn render_scrollbar(
    id: impl Into<ElementId>,
    metrics: ScrollbarMetrics,
    state: ScrollbarState,
) -> impl IntoElement {
    if !metrics.needs_scrollbar() {
        return div().w(px(SCROLLBAR_WIDTH)).into_any_element();
    }

    let thumb_fraction = metrics.thumb_fraction().max(MINIMUM_THUMB_SIZE / metrics.viewport_height.max(1.0));
    let thumb_position = metrics.thumb_position();

    // Calculate thumb bounds within track
    let track_height = metrics.viewport_height - SCROLLBAR_PADDING * 2.0;
    let thumb_height = (track_height * thumb_fraction).max(MINIMUM_THUMB_SIZE);
    let usable_track = track_height - thumb_height;
    let thumb_top = SCROLLBAR_PADDING + (usable_track * thumb_position);

    let thumb_color = match state {
        ScrollbarState::Inactive => colors::thumb(),
        ScrollbarState::Hovered => colors::thumb_hover(),
        ScrollbarState::Dragging { .. } => colors::thumb_active(),
    };

    div()
        .id(id)
        .w(px(SCROLLBAR_WIDTH))
        .h_full()
        .bg(colors::track())
        .cursor(CursorStyle::Arrow)
        .relative()
        .child(
            // Thumb
            div()
                .absolute()
                .top(px(thumb_top))
                .left(px(SCROLLBAR_PADDING))
                .w(px(SCROLLBAR_WIDTH - SCROLLBAR_PADDING * 2.0))
                .h(px(thumb_height))
                .rounded(px(3.0))
                .bg(thumb_color)
        )
        .into_any_element()
}
