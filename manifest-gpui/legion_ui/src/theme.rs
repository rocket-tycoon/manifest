//! Pigs in Space theme for Legion UI
//!
//! A dark theme inspired by the "Pigs in Space" Zed extension.

use gpui::{rgb, Rgba};

/// Theme colors for the application
pub struct Theme;

impl Theme {
    /// Transparent color
    pub fn transparent() -> Rgba {
        Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }
    }
}

impl Theme {
    // ─────────────────────────────────────────────────────────────────────────
    // Backgrounds
    // ─────────────────────────────────────────────────────────────────────────

    /// Main background color (editor, workspace)
    pub fn background() -> Rgba {
        rgb(0x21262c)
    }

    /// Surface/panel background (sidebar, tab bar, status bar)
    pub fn surface() -> Rgba {
        rgb(0x1d2228)
    }

    /// Active line background in editor
    pub fn active_line() -> Rgba {
        rgb(0x2c3137)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Borders
    // ─────────────────────────────────────────────────────────────────────────

    /// Standard border color
    pub fn border() -> Rgba {
        rgb(0x2d333a)
    }

    /// Border variant (slightly lighter)
    pub fn border_variant() -> Rgba {
        rgb(0x353c42)
    }

    /// Focused border color
    pub fn border_focused() -> Rgba {
        rgb(0x3a424b)
    }

    /// Selected border color (uses accent)
    pub fn border_selected() -> Rgba {
        rgb(0x4c9c9d)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Text
    // ─────────────────────────────────────────────────────────────────────────

    /// Primary text color
    pub fn text() -> Rgba {
        rgb(0xc2d6ea)
    }

    /// Secondary/muted text color
    pub fn text_muted() -> Rgba {
        rgb(0x78859b)
    }

    /// Editor foreground text
    pub fn text_editor() -> Rgba {
        rgb(0xa0b0c1)
    }

    /// Placeholder text color
    pub fn text_placeholder() -> Rgba {
        rgb(0x636e80)
    }

    /// Disabled text color
    pub fn text_disabled() -> Rgba {
        rgb(0x404953)
    }

    /// Accent text color
    pub fn text_accent() -> Rgba {
        rgb(0x4c9c9d)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Elements (buttons, inputs, list items)
    // ─────────────────────────────────────────────────────────────────────────

    /// Element background
    pub fn element() -> Rgba {
        rgb(0x353c42)
    }

    /// Element hover state
    pub fn element_hover() -> Rgba {
        rgb(0x3e464d)
    }

    /// Element active/selected state
    pub fn element_active() -> Rgba {
        rgb(0x373f47)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Ghost Elements (Zed-style transparent overlays for list items)
    // ─────────────────────────────────────────────────────────────────────────

    /// Ghost element hover - subtle highlight for list items on hover
    pub fn ghost_element_hover() -> Rgba {
        Rgba {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.05,
        }
    }

    /// Ghost element selected - highlight for selected list items
    pub fn ghost_element_selected() -> Rgba {
        Rgba {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.08,
        }
    }

    /// Ghost element active - highlight when clicking/pressing
    pub fn ghost_element_active() -> Rgba {
        Rgba {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.12,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Indent Guides (for tree views)
    // ─────────────────────────────────────────────────────────────────────────

    /// Indent guide line color
    pub fn indent_guide() -> Rgba {
        rgb(0x2d333a)
    }

    /// Indent guide hover color
    pub fn indent_guide_hover() -> Rgba {
        rgb(0x3a424b)
    }

    /// Indent guide active color (parent of selected item)
    pub fn indent_guide_active() -> Rgba {
        rgb(0x4c9c9d)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Accent colors
    // ─────────────────────────────────────────────────────────────────────────

    /// Primary accent color (teal)
    pub fn accent() -> Rgba {
        rgb(0x4c9c9d)
    }

    /// Secondary accent color (blue)
    pub fn accent_secondary() -> Rgba {
        rgb(0x82aaff)
    }

    /// Accent hover state
    pub fn accent_hover() -> Rgba {
        rgb(0x5eb3b4)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Status colors
    // ─────────────────────────────────────────────────────────────────────────

    /// Error color (coral red)
    pub fn error() -> Rgba {
        rgb(0xff5370)
    }

    /// Error background
    pub fn error_bg() -> Rgba {
        rgb(0x3a2029)
    }

    /// Warning color (amber)
    pub fn warning() -> Rgba {
        rgb(0xf8be53)
    }

    /// Warning background
    pub fn warning_bg() -> Rgba {
        rgb(0x3d3223)
    }

    /// Success color (lime green)
    pub fn success() -> Rgba {
        rgb(0xc3e88d)
    }

    /// Success background
    pub fn success_bg() -> Rgba {
        rgb(0x1e3a2f)
    }

    /// Info color (blue)
    pub fn info() -> Rgba {
        rgb(0x82aaff)
    }

    /// Info background
    pub fn info_bg() -> Rgba {
        rgb(0x1e2940)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Git/diff colors
    // ─────────────────────────────────────────────────────────────────────────

    /// Created/added (green)
    pub fn created() -> Rgba {
        rgb(0xc3e88d)
    }

    /// Modified (blue)
    pub fn modified() -> Rgba {
        rgb(0x82aaff)
    }

    /// Deleted (red)
    pub fn deleted() -> Rgba {
        rgb(0xff5370)
    }

    /// Renamed (purple)
    pub fn renamed() -> Rgba {
        rgb(0xc792ea)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Syntax-inspired UI colors
    // ─────────────────────────────────────────────────────────────────────────

    /// Function/method color (gold)
    pub fn function() -> Rgba {
        rgb(0xffc66d)
    }

    /// String color (green)
    pub fn string() -> Rgba {
        rgb(0xa5c25c)
    }

    /// Number color (coral)
    pub fn number() -> Rgba {
        rgb(0xe87366)
    }

    /// Keyword color (mauve)
    pub fn keyword() -> Rgba {
        rgb(0xac8497)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Terminal colors
    // ─────────────────────────────────────────────────────────────────────────

    /// Terminal background
    pub fn terminal_bg() -> Rgba {
        rgb(0x21262c)
    }

    /// Terminal foreground
    pub fn terminal_fg() -> Rgba {
        rgb(0xb6c4d2)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Scrollbar
    // ─────────────────────────────────────────────────────────────────────────

    /// Scrollbar thumb
    pub fn scrollbar_thumb() -> Rgba {
        rgb(0x3a424b)
    }

    /// Scrollbar thumb hover
    pub fn scrollbar_thumb_hover() -> Rgba {
        rgb(0x4a545e)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Panel Resize Dividers
    // ─────────────────────────────────────────────────────────────────────────

    /// Panel divider color (subtle)
    pub fn panel_divider() -> Rgba {
        rgb(0x3d4450)
    }

    /// Panel divider hover color (accent teal)
    pub fn panel_divider_hover() -> Rgba {
        rgb(0x4c9c9d)
    }
}
