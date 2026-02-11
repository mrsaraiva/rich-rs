//! Control: terminal control codes as a renderable.
//!
//! This is a small subset of Python Rich's `Control` used by Live / Progress.

use crate::segment::{ControlType, Segment, Segments};
use crate::{Console, ConsoleOptions, Measurement, Renderable};

#[derive(Debug, Clone, Default)]
pub struct Control {
    controls: Vec<ControlType>,
}

impl Control {
    pub fn new() -> Self {
        Self {
            controls: Vec::new(),
        }
    }

    pub fn home() -> Self {
        Self {
            controls: vec![ControlType::Home],
        }
    }

    pub fn carriage_return() -> Self {
        Self {
            controls: vec![ControlType::CarriageReturn],
        }
    }

    pub fn erase_in_line(mode: u8) -> Self {
        Self {
            controls: vec![ControlType::EraseInLine(mode)],
        }
    }

    pub fn cursor_up(n: u16) -> Self {
        Self {
            controls: vec![ControlType::CursorUp(n)],
        }
    }

    pub fn move_to(x: u16, y: u16) -> Self {
        Self {
            controls: vec![ControlType::MoveTo { x, y }],
        }
    }

    /// Create a Bell control.
    pub fn bell() -> Self {
        Self {
            controls: vec![ControlType::Bell],
        }
    }

    /// Create a Clear screen control.
    pub fn clear() -> Self {
        Self {
            controls: vec![ControlType::Clear],
        }
    }

    /// Show or hide the cursor.
    pub fn show_cursor(show: bool) -> Self {
        Self {
            controls: vec![if show {
                ControlType::ShowCursor
            } else {
                ControlType::HideCursor
            }],
        }
    }

    /// Enable or disable the alternate screen buffer.
    pub fn alt_screen(enable: bool) -> Self {
        if enable {
            Self {
                controls: vec![ControlType::EnableAltScreen, ControlType::Home],
            }
        } else {
            Self {
                controls: vec![ControlType::DisableAltScreen],
            }
        }
    }

    /// Set the terminal window title.
    pub fn title(_title: impl Into<String>) -> Self {
        Self {
            controls: vec![ControlType::SetTitle],
        }
    }

    pub fn extend(&mut self, controls: impl IntoIterator<Item = ControlType>) {
        self.controls.extend(controls);
    }

    pub fn push(&mut self, control: ControlType) {
        self.controls.push(control);
    }

    pub fn is_empty(&self) -> bool {
        self.controls.is_empty()
    }

    pub fn into_segments(self) -> Segments {
        Segments::from_iter(self.controls.into_iter().map(Segment::control))
    }
}

/// Strip control codes from text.
///
/// Removes Bell (7), Backspace (8), Vertical Tab (11), Form Feed (12),
/// and Carriage Return (13).
pub fn strip_control_codes(text: &str) -> String {
    text.chars()
        .filter(|&c| !matches!(c, '\x07' | '\x08' | '\x0B' | '\x0C' | '\r'))
        .collect()
}

/// Escape control codes so they display as visible text.
///
/// Replaces Bell with \a, Backspace with \b, Vertical Tab with \v,
/// Form Feed with \f, and Carriage Return with \r.
pub fn escape_control_codes(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\x07' => result.push_str("\\a"),
            '\x08' => result.push_str("\\b"),
            '\x0B' => result.push_str("\\v"),
            '\x0C' => result.push_str("\\f"),
            '\r' => result.push_str("\\r"),
            _ => result.push(c),
        }
    }
    result
}

impl Renderable for Control {
    fn render(&self, _console: &Console, _options: &ConsoleOptions) -> Segments {
        Segments::from_iter(self.controls.iter().cloned().map(Segment::control))
    }

    fn measure(&self, _console: &Console, _options: &ConsoleOptions) -> Measurement {
        Measurement::new(0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bell() {
        let ctrl = Control::bell();
        assert!(!ctrl.is_empty());
    }

    #[test]
    fn test_clear() {
        let ctrl = Control::clear();
        assert!(!ctrl.is_empty());
    }

    #[test]
    fn test_show_cursor() {
        let show = Control::show_cursor(true);
        assert!(!show.is_empty());
        let hide = Control::show_cursor(false);
        assert!(!hide.is_empty());
    }

    #[test]
    fn test_alt_screen() {
        let enable = Control::alt_screen(true);
        assert!(!enable.is_empty());
        let disable = Control::alt_screen(false);
        assert!(!disable.is_empty());
    }

    #[test]
    fn test_title() {
        let ctrl = Control::title("Hello");
        assert!(!ctrl.is_empty());
    }

    #[test]
    fn test_strip_control_codes() {
        assert_eq!(strip_control_codes("hello"), "hello");
        assert_eq!(strip_control_codes("he\x07llo"), "hello");
        assert_eq!(strip_control_codes("he\x08llo"), "hello");
        assert_eq!(strip_control_codes("he\x0Bllo"), "hello");
        assert_eq!(strip_control_codes("he\x0Cllo"), "hello");
        assert_eq!(strip_control_codes("he\rllo"), "hello");
        assert_eq!(strip_control_codes("\x07\x08\x0B\x0C\rhello"), "hello");
    }

    #[test]
    fn test_escape_control_codes() {
        assert_eq!(escape_control_codes("hello"), "hello");
        assert_eq!(escape_control_codes("he\x07llo"), "he\\allo");
        assert_eq!(escape_control_codes("he\x08llo"), "he\\bllo");
        assert_eq!(escape_control_codes("he\x0Bllo"), "he\\vllo");
        assert_eq!(escape_control_codes("he\x0Cllo"), "he\\fllo");
        assert_eq!(escape_control_codes("he\rllo"), "he\\rllo");
    }
}
