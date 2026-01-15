use gpui::{
    point, px, relative, size, App, Bounds, Element, ElementId, ElementInputHandler,
    Entity, Focusable, Font, FontStyle, FontWeight, GlobalElementId, Hitbox, LayoutId, Pixels,
    ShapedLine, SharedString, Style, TextRun, Window, fill,
};

use crate::editor::FeatureEditor;

/// Colors for the text editor (matching Pigs in Space theme).
mod colors {
    use gpui::Hsla;

    pub fn background() -> Hsla {
        Hsla { h: 210.0 / 360.0, s: 0.13, l: 0.15, a: 1.0 }
    }

    pub fn text() -> Hsla {
        Hsla { h: 210.0 / 360.0, s: 0.45, l: 0.84, a: 1.0 }
    }

    pub fn selection() -> Hsla {
        Hsla { h: 210.0 / 360.0, s: 0.5, l: 0.4, a: 0.4 }
    }

    pub fn cursor() -> Hsla {
        Hsla { h: 220.0 / 360.0, s: 1.0, l: 0.75, a: 1.0 }
    }
}

/// Stored layout information for mouse position calculations.
/// Uses f32 values for calculations since Pixels fields are private.
#[derive(Clone, Copy, Debug)]
pub struct TextLayoutInfo {
    pub content_origin_x: f32,
    pub content_origin_y: f32,
    pub line_height: f32,
    pub char_width: f32,
    pub first_visible_line: usize,
}

/// Custom GPUI element for rendering multi-line text with cursor and selection.
pub struct MultiLineTextElement {
    pub editor: Entity<FeatureEditor>,
}

/// Prepaint state storing computed layout information.
pub struct MultiLineTextPrepaintState {
    pub lines: Vec<ShapedLine>,
    pub line_height_px: Pixels,
    pub line_height_f32: f32,
    pub char_width_f32: f32,
    pub cursor_bounds: Option<Bounds<Pixels>>,
    pub selection_bounds: Vec<Bounds<Pixels>>,
    pub hitbox: Hitbox,
    pub content_bounds: Bounds<Pixels>,
    pub content_origin_x: f32,
    pub content_origin_y: f32,
    pub first_visible_line: usize,
}

impl gpui::IntoElement for MultiLineTextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for MultiLineTextElement {
    type RequestLayoutState = ();
    type PrepaintState = MultiLineTextPrepaintState;

    fn id(&self) -> Option<ElementId> {
        // Use a stable element ID. Changing IDs can confuse GPUI's layout engine.
        // The wrapper divs have stable IDs, and the content is refreshed via cx.notify().
        Some(ElementId::Name("multiline-text-editor".into()))
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let editor = self.editor.read(cx);

        // Use the same font as the terminal: Bitstream Vera Sans Mono 14px
        let font_size = px(14.0);
        let line_height_px = font_size * 1.2; // Match terminal line height

        // f32 values for calculations
        let base_font_size = 14.0_f32;
        let line_height_f32 = base_font_size * 1.2;

        // Create the monospace font matching terminal
        let editor_font = Font {
            family: "Bitstream Vera Sans Mono".into(),
            features: Default::default(),
            fallbacks: None,
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
        };

        // Calculate character width (for monospace)
        let char_width_px = {
            let run = TextRun {
                len: 1,
                font: editor_font.clone(),
                color: colors::text(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let shaped = window.text_system().shape_line("M".into(), font_size, &[run], None);
            shaped.width
        };
        // Get actual char width from shaped text
        let char_width_f32 = char_width_px / px(1.0);

        // Content area with padding (more padding for visual offset from pane edge)
        let padding = px(16.0);
        let padding_f32 = 16.0_f32;
        let content_bounds = Bounds {
            origin: point(bounds.origin.x + padding, bounds.origin.y + padding),
            size: size(bounds.size.width - padding * 2.0, bounds.size.height - padding * 2.0),
        };

        // Use actual content bounds height for visible line calculations.
        // Previously this used a hardcoded 600.0 estimate which caused rendering bugs
        // when the actual pane height differed from the estimate.
        let content_height_f32 = content_bounds.size.height / px(1.0);

        // Get active tab data
        let (lines_data, cursor, selection_bounds_opt, scroll_offset) = if let Some(tab) = editor.active_tab() {
            (
                tab.lines.clone(),
                tab.cursor,
                tab.selection_bounds(),
                tab.scroll_offset,
            )
        } else {
            (vec![String::new()], Default::default(), None, 0.0)
        };

        // Calculate visible lines based on scroll offset
        let total_lines = lines_data.len();
        let first_visible = (scroll_offset / line_height_f32).floor() as usize;
        // Use actual content height (with fallback for very small bounds during initial layout)
        let effective_height = content_height_f32.max(line_height_f32 * 5.0);
        let visible_count = (effective_height / line_height_f32).ceil() as usize + 1;
        let last_visible = (first_visible + visible_count).min(total_lines);

        // Shape visible lines
        let mut shaped_lines = Vec::with_capacity(visible_count);
        for line_idx in first_visible..last_visible {
            let line_text = lines_data.get(line_idx).cloned().unwrap_or_default();
            let display_text: SharedString = if line_text.is_empty() {
                " ".into()
            } else {
                SharedString::from(line_text)
            };

            let run = TextRun {
                len: display_text.len(),
                font: editor_font.clone(),
                color: colors::text(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let shaped = window.text_system().shape_line(display_text, font_size, &[run], None);
            shaped_lines.push(shaped);
        }

        // Calculate selection bounds
        let mut selection_rects = Vec::new();
        if let Some((start, end)) = selection_bounds_opt {
            for line_idx in first_visible..last_visible {
                if line_idx < start.line || line_idx > end.line {
                    continue;
                }

                let line_len = lines_data.get(line_idx).map(|s| s.len()).unwrap_or(0);
                let line_y = content_bounds.origin.y + px((line_idx - first_visible) as f32 * line_height_f32);

                let sel_start = if line_idx == start.line { start.column } else { 0 };
                let sel_end = if line_idx == end.line { end.column } else { line_len };

                if sel_start < sel_end || (line_idx > start.line && line_idx < end.line) {
                    let start_x = content_bounds.origin.x + px(sel_start as f32 * char_width_f32);
                    let end_x = if sel_end > sel_start {
                        content_bounds.origin.x + px(sel_end as f32 * char_width_f32)
                    } else {
                        content_bounds.origin.x + char_width_px
                    };

                    selection_rects.push(Bounds::from_corners(
                        point(start_x, line_y),
                        point(end_x, line_y + line_height_px),
                    ));
                }
            }
        }

        // Calculate cursor bounds
        let cursor_bounds = if cursor.line >= first_visible && cursor.line < last_visible {
            let cursor_line_offset = cursor.line - first_visible;
            let cursor_x = content_bounds.origin.x + px(cursor.column as f32 * char_width_f32);
            let cursor_y = content_bounds.origin.y + px(cursor_line_offset as f32 * line_height_f32);

            Some(Bounds::new(
                point(cursor_x, cursor_y),
                size(px(2.0), line_height_px),
            ))
        } else {
            None
        };

        // Create hitbox for mouse events
        let hitbox = window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal);

        MultiLineTextPrepaintState {
            lines: shaped_lines,
            line_height_px,
            line_height_f32,
            char_width_f32,
            cursor_bounds,
            selection_bounds: selection_rects,
            hitbox,
            content_bounds,
            content_origin_x: padding_f32,
            content_origin_y: padding_f32,
            first_visible_line: first_visible,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        // Paint background
        window.paint_quad(fill(bounds, colors::background()));

        // Paint selections
        for sel_bounds in &prepaint.selection_bounds {
            window.paint_quad(fill(*sel_bounds, colors::selection()));
        }

        // Paint text lines
        for (idx, line) in prepaint.lines.iter().enumerate() {
            let y = prepaint.content_bounds.origin.y + px(idx as f32 * prepaint.line_height_f32);
            line.paint(
                point(prepaint.content_bounds.origin.x, y),
                prepaint.line_height_px,
                gpui::TextAlign::Left,
                None,
                window,
                cx,
            )
            .ok();
        }

        // Paint cursor if focused
        let focus_handle = self.editor.read(cx).focus_handle(cx);
        if focus_handle.is_focused(window) {
            if let Some(cursor_bounds) = prepaint.cursor_bounds {
                window.paint_quad(fill(cursor_bounds, colors::cursor()));
            }
        }

        // Register input handler for IME support
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.editor.clone()),
            cx,
        );

        // Store layout info for mouse position calculations
        // Use actual screen coordinates from content_bounds
        let content_origin_x = prepaint.content_bounds.origin.x / px(1.0);
        let content_origin_y = prepaint.content_bounds.origin.y / px(1.0);
        let content_height = prepaint.content_bounds.size.height / px(1.0);

        let layout_info = TextLayoutInfo {
            content_origin_x,
            content_origin_y,
            line_height: prepaint.line_height_f32,
            char_width: prepaint.char_width_f32,
            first_visible_line: prepaint.first_visible_line,
        };
        let visible_height = content_height.max(prepaint.line_height_f32 * 10.0); // Use actual content height

        self.editor.update(cx, |editor, _cx| {
            editor.set_layout_info(layout_info);
            editor.set_visible_height(visible_height);
        });
    }
}
