//! Keystroke to ANSI escape sequence conversion.
//!
//! This module converts GPUI keystrokes into the appropriate ANSI escape sequences
//! that terminals expect. Based on Zed editor's terminal implementation.

use std::borrow::Cow;

use alacritty_terminal::term::TermMode;
use gpui::Keystroke;

#[derive(Debug, PartialEq, Eq)]
enum Modifiers {
    None,
    Alt,
    Ctrl,
    Shift,
    CtrlShift,
    Other,
}

impl Modifiers {
    fn new(ks: &Keystroke) -> Self {
        match (
            ks.modifiers.alt,
            ks.modifiers.control,
            ks.modifiers.shift,
            ks.modifiers.platform,
        ) {
            (false, false, false, false) => Modifiers::None,
            (true, false, false, false) => Modifiers::Alt,
            (false, true, false, false) => Modifiers::Ctrl,
            (false, false, true, false) => Modifiers::Shift,
            (false, true, true, false) => Modifiers::CtrlShift,
            _ => Modifiers::Other,
        }
    }

    fn any(&self) -> bool {
        !matches!(self, Modifiers::None)
    }
}

/// Convert a GPUI keystroke to an ANSI escape sequence.
pub fn to_esc_str(
    keystroke: &Keystroke,
    mode: &TermMode,
    option_as_meta: bool,
) -> Option<Cow<'static, str>> {
    let modifiers = Modifiers::new(keystroke);

    // Manual bindings including modifiers
    let manual_esc_str: Option<&'static str> = match (keystroke.key.as_ref(), &modifiers) {
        // Basic special keys
        ("tab", Modifiers::None) => Some("\x09"),
        ("escape", Modifiers::None) => Some("\x1b"),
        ("enter", Modifiers::None) => Some("\x0d"),
        ("enter", Modifiers::Shift) => Some("\x0a"),
        ("enter", Modifiers::Alt) => Some("\x1b\x0d"),
        ("backspace", Modifiers::None) => Some("\x7f"),
        // Interesting escape codes
        ("tab", Modifiers::Shift) => Some("\x1b[Z"),
        ("backspace", Modifiers::Ctrl) => Some("\x08"),
        ("backspace", Modifiers::Alt) => Some("\x1b\x7f"),
        ("backspace", Modifiers::Shift) => Some("\x7f"),
        ("space", Modifiers::Ctrl) => Some("\x00"),
        // Home/End with shift in alt screen
        ("home", Modifiers::Shift) if mode.contains(TermMode::ALT_SCREEN) => Some("\x1b[1;2H"),
        ("end", Modifiers::Shift) if mode.contains(TermMode::ALT_SCREEN) => Some("\x1b[1;2F"),
        ("pageup", Modifiers::Shift) if mode.contains(TermMode::ALT_SCREEN) => Some("\x1b[5;2~"),
        ("pagedown", Modifiers::Shift) if mode.contains(TermMode::ALT_SCREEN) => Some("\x1b[6;2~"),
        // Navigation keys with app cursor mode
        ("home", Modifiers::None) if mode.contains(TermMode::APP_CURSOR) => Some("\x1bOH"),
        ("home", Modifiers::None) if !mode.contains(TermMode::APP_CURSOR) => Some("\x1b[H"),
        ("end", Modifiers::None) if mode.contains(TermMode::APP_CURSOR) => Some("\x1bOF"),
        ("end", Modifiers::None) if !mode.contains(TermMode::APP_CURSOR) => Some("\x1b[F"),
        ("up", Modifiers::None) if mode.contains(TermMode::APP_CURSOR) => Some("\x1bOA"),
        ("up", Modifiers::None) if !mode.contains(TermMode::APP_CURSOR) => Some("\x1b[A"),
        ("down", Modifiers::None) if mode.contains(TermMode::APP_CURSOR) => Some("\x1bOB"),
        ("down", Modifiers::None) if !mode.contains(TermMode::APP_CURSOR) => Some("\x1b[B"),
        ("right", Modifiers::None) if mode.contains(TermMode::APP_CURSOR) => Some("\x1bOC"),
        ("right", Modifiers::None) if !mode.contains(TermMode::APP_CURSOR) => Some("\x1b[C"),
        ("left", Modifiers::None) if mode.contains(TermMode::APP_CURSOR) => Some("\x1bOD"),
        ("left", Modifiers::None) if !mode.contains(TermMode::APP_CURSOR) => Some("\x1b[D"),
        ("back", Modifiers::None) => Some("\x7f"),
        ("insert", Modifiers::None) => Some("\x1b[2~"),
        ("delete", Modifiers::None) => Some("\x1b[3~"),
        ("pageup", Modifiers::None) => Some("\x1b[5~"),
        ("pagedown", Modifiers::None) => Some("\x1b[6~"),
        // Function keys
        ("f1", Modifiers::None) => Some("\x1bOP"),
        ("f2", Modifiers::None) => Some("\x1bOQ"),
        ("f3", Modifiers::None) => Some("\x1bOR"),
        ("f4", Modifiers::None) => Some("\x1bOS"),
        ("f5", Modifiers::None) => Some("\x1b[15~"),
        ("f6", Modifiers::None) => Some("\x1b[17~"),
        ("f7", Modifiers::None) => Some("\x1b[18~"),
        ("f8", Modifiers::None) => Some("\x1b[19~"),
        ("f9", Modifiers::None) => Some("\x1b[20~"),
        ("f10", Modifiers::None) => Some("\x1b[21~"),
        ("f11", Modifiers::None) => Some("\x1b[23~"),
        ("f12", Modifiers::None) => Some("\x1b[24~"),
        // Ctrl+letter mappings (caret notation)
        ("a", Modifiers::Ctrl) => Some("\x01"),
        ("A", Modifiers::CtrlShift) => Some("\x01"),
        ("b", Modifiers::Ctrl) => Some("\x02"),
        ("B", Modifiers::CtrlShift) => Some("\x02"),
        ("c", Modifiers::Ctrl) => Some("\x03"),
        ("C", Modifiers::CtrlShift) => Some("\x03"),
        ("d", Modifiers::Ctrl) => Some("\x04"),
        ("D", Modifiers::CtrlShift) => Some("\x04"),
        ("e", Modifiers::Ctrl) => Some("\x05"),
        ("E", Modifiers::CtrlShift) => Some("\x05"),
        ("f", Modifiers::Ctrl) => Some("\x06"),
        ("F", Modifiers::CtrlShift) => Some("\x06"),
        ("g", Modifiers::Ctrl) => Some("\x07"),
        ("G", Modifiers::CtrlShift) => Some("\x07"),
        ("h", Modifiers::Ctrl) => Some("\x08"),
        ("H", Modifiers::CtrlShift) => Some("\x08"),
        ("i", Modifiers::Ctrl) => Some("\x09"),
        ("I", Modifiers::CtrlShift) => Some("\x09"),
        ("j", Modifiers::Ctrl) => Some("\x0a"),
        ("J", Modifiers::CtrlShift) => Some("\x0a"),
        ("k", Modifiers::Ctrl) => Some("\x0b"),
        ("K", Modifiers::CtrlShift) => Some("\x0b"),
        ("l", Modifiers::Ctrl) => Some("\x0c"),
        ("L", Modifiers::CtrlShift) => Some("\x0c"),
        ("m", Modifiers::Ctrl) => Some("\x0d"),
        ("M", Modifiers::CtrlShift) => Some("\x0d"),
        ("n", Modifiers::Ctrl) => Some("\x0e"),
        ("N", Modifiers::CtrlShift) => Some("\x0e"),
        ("o", Modifiers::Ctrl) => Some("\x0f"),
        ("O", Modifiers::CtrlShift) => Some("\x0f"),
        ("p", Modifiers::Ctrl) => Some("\x10"),
        ("P", Modifiers::CtrlShift) => Some("\x10"),
        ("q", Modifiers::Ctrl) => Some("\x11"),
        ("Q", Modifiers::CtrlShift) => Some("\x11"),
        ("r", Modifiers::Ctrl) => Some("\x12"),
        ("R", Modifiers::CtrlShift) => Some("\x12"),
        ("s", Modifiers::Ctrl) => Some("\x13"),
        ("S", Modifiers::CtrlShift) => Some("\x13"),
        ("t", Modifiers::Ctrl) => Some("\x14"),
        ("T", Modifiers::CtrlShift) => Some("\x14"),
        ("u", Modifiers::Ctrl) => Some("\x15"),
        ("U", Modifiers::CtrlShift) => Some("\x15"),
        ("v", Modifiers::Ctrl) => Some("\x16"),
        ("V", Modifiers::CtrlShift) => Some("\x16"),
        ("w", Modifiers::Ctrl) => Some("\x17"),
        ("W", Modifiers::CtrlShift) => Some("\x17"),
        ("x", Modifiers::Ctrl) => Some("\x18"),
        ("X", Modifiers::CtrlShift) => Some("\x18"),
        ("y", Modifiers::Ctrl) => Some("\x19"),
        ("Y", Modifiers::CtrlShift) => Some("\x19"),
        ("z", Modifiers::Ctrl) => Some("\x1a"),
        ("Z", Modifiers::CtrlShift) => Some("\x1a"),
        ("@", Modifiers::Ctrl) => Some("\x00"),
        ("[", Modifiers::Ctrl) => Some("\x1b"),
        ("\\", Modifiers::Ctrl) => Some("\x1c"),
        ("]", Modifiers::Ctrl) => Some("\x1d"),
        ("^", Modifiers::Ctrl) => Some("\x1e"),
        ("_", Modifiers::Ctrl) => Some("\x1f"),
        ("?", Modifiers::Ctrl) => Some("\x7f"),
        _ => None,
    };

    if let Some(esc_str) = manual_esc_str {
        return Some(Cow::Borrowed(esc_str));
    }

    // Automated bindings applying modifiers
    if modifiers.any() {
        let modifier_code = modifier_code(keystroke);
        let modified_esc_str = match keystroke.key.as_ref() {
            "up" => Some(format!("\x1b[1;{}A", modifier_code)),
            "down" => Some(format!("\x1b[1;{}B", modifier_code)),
            "right" => Some(format!("\x1b[1;{}C", modifier_code)),
            "left" => Some(format!("\x1b[1;{}D", modifier_code)),
            "f1" => Some(format!("\x1b[1;{}P", modifier_code)),
            "f2" => Some(format!("\x1b[1;{}Q", modifier_code)),
            "f3" => Some(format!("\x1b[1;{}R", modifier_code)),
            "f4" => Some(format!("\x1b[1;{}S", modifier_code)),
            "f5" => Some(format!("\x1b[15;{}~", modifier_code)),
            "f6" => Some(format!("\x1b[17;{}~", modifier_code)),
            "f7" => Some(format!("\x1b[18;{}~", modifier_code)),
            "f8" => Some(format!("\x1b[19;{}~", modifier_code)),
            "f9" => Some(format!("\x1b[20;{}~", modifier_code)),
            "f10" => Some(format!("\x1b[21;{}~", modifier_code)),
            "f11" => Some(format!("\x1b[23;{}~", modifier_code)),
            "f12" => Some(format!("\x1b[24;{}~", modifier_code)),
            _ if modifier_code == 2 => None,
            "insert" => Some(format!("\x1b[2;{}~", modifier_code)),
            "pageup" => Some(format!("\x1b[5;{}~", modifier_code)),
            "pagedown" => Some(format!("\x1b[6;{}~", modifier_code)),
            "end" => Some(format!("\x1b[1;{}F", modifier_code)),
            "home" => Some(format!("\x1b[1;{}H", modifier_code)),
            _ => None,
        };
        if let Some(esc_str) = modified_esc_str {
            return Some(Cow::Owned(esc_str));
        }
    }

    // Alt+letter sends ESC prefix
    if !cfg!(target_os = "macos") || option_as_meta {
        let is_alt_lowercase_ascii = modifiers == Modifiers::Alt && keystroke.key.is_ascii();
        let is_alt_uppercase_ascii =
            keystroke.modifiers.alt && keystroke.modifiers.shift && keystroke.key.is_ascii();
        if is_alt_lowercase_ascii || is_alt_uppercase_ascii {
            let key = if is_alt_uppercase_ascii {
                &keystroke.key.to_ascii_uppercase()
            } else {
                &keystroke.key
            };
            return Some(Cow::Owned(format!("\x1b{}", key)));
        }
    }

    None
}

/// Calculate modifier code for escape sequences.
///   Code     Modifiers
/// ---------+---------------------------
///    2     | Shift
///    3     | Alt
///    4     | Shift + Alt
///    5     | Control
///    6     | Shift + Control
///    7     | Alt + Control
///    8     | Shift + Alt + Control
fn modifier_code(keystroke: &Keystroke) -> u32 {
    let mut modifier_code = 0;
    if keystroke.modifiers.shift {
        modifier_code |= 1;
    }
    if keystroke.modifiers.alt {
        modifier_code |= 1 << 1;
    }
    if keystroke.modifiers.control {
        modifier_code |= 1 << 2;
    }
    modifier_code + 1
}
