//! Color conversion utilities for terminal rendering.

use gpui::{Hsla, Rgba, rgba};

/// Terminal color palette - hardcoded for MVP.
pub struct TerminalColors;

impl TerminalColors {
    pub fn background() -> Rgba {
        rgba(0x1e1e1eff)
    }

    pub fn foreground() -> Rgba {
        rgba(0xd4d4d4ff)
    }

    pub fn cursor() -> Rgba {
        rgba(0xffffffff)
    }

    pub fn selection() -> Rgba {
        rgba(0x264f7880)
    }

    // Standard ANSI colors (0-7)
    pub fn ansi_black() -> Rgba {
        rgba(0x000000ff)
    }

    pub fn ansi_red() -> Rgba {
        rgba(0xcd3131ff)
    }

    pub fn ansi_green() -> Rgba {
        rgba(0x0dbc79ff)
    }

    pub fn ansi_yellow() -> Rgba {
        rgba(0xe5e510ff)
    }

    pub fn ansi_blue() -> Rgba {
        rgba(0x2472c8ff)
    }

    pub fn ansi_magenta() -> Rgba {
        rgba(0xbc3fbcff)
    }

    pub fn ansi_cyan() -> Rgba {
        rgba(0x11a8cdff)
    }

    pub fn ansi_white() -> Rgba {
        rgba(0xe5e5e5ff)
    }

    // Bright ANSI colors (8-15)
    pub fn ansi_bright_black() -> Rgba {
        rgba(0x666666ff)
    }

    pub fn ansi_bright_red() -> Rgba {
        rgba(0xf14c4cff)
    }

    pub fn ansi_bright_green() -> Rgba {
        rgba(0x23d18bff)
    }

    pub fn ansi_bright_yellow() -> Rgba {
        rgba(0xf5f543ff)
    }

    pub fn ansi_bright_blue() -> Rgba {
        rgba(0x3b8eeaff)
    }

    pub fn ansi_bright_magenta() -> Rgba {
        rgba(0xd670d6ff)
    }

    pub fn ansi_bright_cyan() -> Rgba {
        rgba(0x29b8dbff)
    }

    pub fn ansi_bright_white() -> Rgba {
        rgba(0xe5e5e5ff)
    }
}

/// Get an ANSI color by index (0-15).
pub fn get_ansi_color(index: u8) -> Rgba {
    match index {
        0 => TerminalColors::ansi_black(),
        1 => TerminalColors::ansi_red(),
        2 => TerminalColors::ansi_green(),
        3 => TerminalColors::ansi_yellow(),
        4 => TerminalColors::ansi_blue(),
        5 => TerminalColors::ansi_magenta(),
        6 => TerminalColors::ansi_cyan(),
        7 => TerminalColors::ansi_white(),
        8 => TerminalColors::ansi_bright_black(),
        9 => TerminalColors::ansi_bright_red(),
        10 => TerminalColors::ansi_bright_green(),
        11 => TerminalColors::ansi_bright_yellow(),
        12 => TerminalColors::ansi_bright_blue(),
        13 => TerminalColors::ansi_bright_magenta(),
        14 => TerminalColors::ansi_bright_cyan(),
        15 => TerminalColors::ansi_bright_white(),
        _ => TerminalColors::foreground(),
    }
}

/// Convert an indexed color (0-255) to RGBA.
pub fn get_indexed_color(index: u8) -> Rgba {
    if index < 16 {
        return get_ansi_color(index);
    }

    // Colors 16-231: 6x6x6 color cube
    if index < 232 {
        let index = index - 16;
        let r = (index / 36) % 6;
        let g = (index / 6) % 6;
        let b = index % 6;

        let r = if r > 0 { r * 40 + 55 } else { 0 };
        let g = if g > 0 { g * 40 + 55 } else { 0 };
        let b = if b > 0 { b * 40 + 55 } else { 0 };

        return rgba(((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | 0xff);
    }

    // Colors 232-255: grayscale
    let gray = (index - 232) * 10 + 8;
    rgba(((gray as u32) << 24) | ((gray as u32) << 16) | ((gray as u32) << 8) | 0xff)
}

/// Convert an alacritty color to GPUI Hsla.
pub fn convert_color(color: &alacritty_terminal::vte::ansi::Color) -> Hsla {
    use alacritty_terminal::vte::ansi::{Color, NamedColor};

    match color {
        Color::Named(named) => {
            let rgba = match named {
                NamedColor::Black => TerminalColors::ansi_black(),
                NamedColor::Red => TerminalColors::ansi_red(),
                NamedColor::Green => TerminalColors::ansi_green(),
                NamedColor::Yellow => TerminalColors::ansi_yellow(),
                NamedColor::Blue => TerminalColors::ansi_blue(),
                NamedColor::Magenta => TerminalColors::ansi_magenta(),
                NamedColor::Cyan => TerminalColors::ansi_cyan(),
                NamedColor::White => TerminalColors::ansi_white(),
                NamedColor::BrightBlack => TerminalColors::ansi_bright_black(),
                NamedColor::BrightRed => TerminalColors::ansi_bright_red(),
                NamedColor::BrightGreen => TerminalColors::ansi_bright_green(),
                NamedColor::BrightYellow => TerminalColors::ansi_bright_yellow(),
                NamedColor::BrightBlue => TerminalColors::ansi_bright_blue(),
                NamedColor::BrightMagenta => TerminalColors::ansi_bright_magenta(),
                NamedColor::BrightCyan => TerminalColors::ansi_bright_cyan(),
                NamedColor::BrightWhite => TerminalColors::ansi_bright_white(),
                NamedColor::Foreground => TerminalColors::foreground(),
                NamedColor::Background => TerminalColors::background(),
                NamedColor::Cursor => TerminalColors::cursor(),
                _ => TerminalColors::foreground(),
            };
            rgba.into()
        }
        Color::Spec(rgb) => {
            Hsla::from(Rgba {
                r: rgb.r as f32 / 255.0,
                g: rgb.g as f32 / 255.0,
                b: rgb.b as f32 / 255.0,
                a: 1.0,
            })
        }
        Color::Indexed(index) => {
            get_indexed_color(*index).into()
        }
    }
}
