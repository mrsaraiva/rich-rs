//! Segment: the atomic unit of terminal output.
//!
//! Everything in Rich ultimately becomes a sequence of Segments.

use crate::style::Style;

/// Control codes that can be embedded in output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlType {
    /// Ring the terminal bell.
    Bell,
    /// Carriage return.
    CarriageReturn,
    /// Move cursor to home position.
    Home,
    /// Clear the screen.
    Clear,
    /// Show the cursor.
    ShowCursor,
    /// Hide the cursor.
    HideCursor,
    /// Enable alternate screen buffer.
    EnableAltScreen,
    /// Disable alternate screen buffer.
    DisableAltScreen,
    /// Set window title.
    SetTitle,
}

/// A segment of text with optional style and control codes.
///
/// This is the fundamental unit of output in Rich. All renderables
/// produce sequences of Segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// The text content.
    pub text: String,
    /// Optional style to apply.
    pub style: Option<Style>,
    /// Optional control code (if set, text is typically empty).
    pub control: Option<ControlType>,
}

impl Segment {
    /// Create a new text segment.
    pub fn new(text: impl Into<String>) -> Self {
        Segment {
            text: text.into(),
            style: None,
            control: None,
        }
    }

    /// Create a new styled segment.
    pub fn styled(text: impl Into<String>, style: Style) -> Self {
        Segment {
            text: text.into(),
            style: Some(style),
            control: None,
        }
    }

    /// Create a control segment.
    pub fn control(control: ControlType) -> Self {
        Segment {
            text: String::new(),
            style: None,
            control: Some(control),
        }
    }

    /// Create a newline segment.
    pub fn line() -> Self {
        Segment::new("\n")
    }

    /// Check if this segment is a control segment.
    pub fn is_control(&self) -> bool {
        self.control.is_some()
    }

    /// Get the cell width of this segment's text.
    pub fn cell_len(&self) -> usize {
        crate::cells::cell_len(&self.text)
    }

    /// Apply a style to this segment, combining with any existing style.
    pub fn apply_style(&self, style: &Style) -> Self {
        Segment {
            text: self.text.clone(),
            style: Some(match &self.style {
                Some(existing) => existing.combine(style),
                None => *style,
            }),
            control: self.control,
        }
    }
}

impl Default for Segment {
    fn default() -> Self {
        Segment::new("")
    }
}

impl From<&str> for Segment {
    fn from(s: &str) -> Self {
        Segment::new(s)
    }
}

impl From<String> for Segment {
    fn from(s: String) -> Self {
        Segment::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_segment() {
        let seg = Segment::new("hello");
        assert_eq!(seg.text, "hello");
        assert!(seg.style.is_none());
        assert!(seg.control.is_none());
    }

    #[test]
    fn test_cell_len() {
        let seg = Segment::new("hello");
        assert_eq!(seg.cell_len(), 5);
    }
}
