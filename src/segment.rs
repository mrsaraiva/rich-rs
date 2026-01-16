//! Segment: the atomic unit of terminal output.
//!
//! Everything in Rich ultimately becomes a sequence of Segments.

use smallvec::SmallVec;
use std::borrow::Cow;

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
    /// Move cursor up N lines.
    CursorUp(u16),
    /// Move cursor down N lines.
    CursorDown(u16),
    /// Move cursor forward N columns.
    CursorForward(u16),
    /// Move cursor backward N columns.
    CursorBackward(u16),
    /// Erase in line (0=cursor to end, 1=start to cursor, 2=entire line).
    EraseInLine(u8),
}

/// A segment of text with optional style and control codes.
///
/// This is the fundamental unit of output in Rich. All renderables
/// produce sequences of Segments.
///
/// Uses `Cow<'static, str>` for text to allow both owned and static strings
/// without lifetime complexity in the API. This is a deliberate tradeoff
/// favoring API simplicity over zero-copy for borrowed non-static input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// The text content.
    pub text: Cow<'static, str>,
    /// Optional style to apply.
    pub style: Option<Style>,
    /// Optional control code (if set, text is typically empty).
    pub control: Option<ControlType>,
}

impl Segment {
    /// Create a new text segment.
    pub fn new(text: impl Into<Cow<'static, str>>) -> Self {
        Segment {
            text: text.into(),
            style: None,
            control: None,
        }
    }

    /// Create a new styled segment.
    pub fn styled(text: impl Into<Cow<'static, str>>, style: Style) -> Self {
        Segment {
            text: text.into(),
            style: Some(style),
            control: None,
        }
    }

    /// Create a control segment.
    pub fn control(control: ControlType) -> Self {
        Segment {
            text: Cow::Borrowed(""),
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

impl From<&'static str> for Segment {
    fn from(s: &'static str) -> Self {
        Segment::new(s)
    }
}

impl From<String> for Segment {
    fn from(s: String) -> Self {
        Segment::new(s)
    }
}

// Implement Send + Sync for Segment (Cow<'static, str> is Send + Sync)
unsafe impl Send for Segment {}
unsafe impl Sync for Segment {}

/// A collection of segments, backed by SmallVec for efficiency.
///
/// This newtype abstracts over the underlying storage, allowing future
/// optimization (e.g., streaming) without breaking the API.
#[derive(Debug, Clone, Default)]
pub struct Segments(SmallVec<[Segment; 8]>);

impl Segments {
    /// Create an empty Segments collection.
    pub fn new() -> Self {
        Segments(SmallVec::new())
    }

    /// Create a Segments collection with a single segment.
    pub fn one(segment: Segment) -> Self {
        let mut sv = SmallVec::new();
        sv.push(segment);
        Segments(sv)
    }

    /// Add a segment to the collection.
    pub fn push(&mut self, segment: Segment) {
        self.0.push(segment);
    }

    /// Extend with segments from an iterator.
    pub fn extend(&mut self, iter: impl IntoIterator<Item = Segment>) {
        self.0.extend(iter);
    }

    /// Get the number of segments.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate over segments.
    pub fn iter(&self) -> impl Iterator<Item = &Segment> {
        self.0.iter()
    }

    /// Get the total cell width of all segments.
    pub fn cell_len(&self) -> usize {
        self.0.iter().map(|s| s.cell_len()).sum()
    }

    /// Convert to a Vec (consumes self).
    pub fn into_vec(self) -> Vec<Segment> {
        self.0.into_vec()
    }
}

impl From<Segment> for Segments {
    fn from(segment: Segment) -> Self {
        Segments::one(segment)
    }
}

impl From<Vec<Segment>> for Segments {
    fn from(vec: Vec<Segment>) -> Self {
        Segments(SmallVec::from_vec(vec))
    }
}

impl FromIterator<Segment> for Segments {
    fn from_iter<I: IntoIterator<Item = Segment>>(iter: I) -> Self {
        Segments(iter.into_iter().collect())
    }
}

impl IntoIterator for Segments {
    type Item = Segment;
    type IntoIter = smallvec::IntoIter<[Segment; 8]>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Segments {
    type Item = &'a Segment;
    type IntoIter = std::slice::Iter<'a, Segment>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

// Segments is Send + Sync because Segment is
unsafe impl Send for Segments {}
unsafe impl Sync for Segments {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_segment() {
        let seg = Segment::new("hello");
        assert_eq!(&*seg.text, "hello");
        assert!(seg.style.is_none());
        assert!(seg.control.is_none());
    }

    #[test]
    fn test_cell_len() {
        let seg = Segment::new("hello");
        assert_eq!(seg.cell_len(), 5);
    }

    #[test]
    fn test_segments_collection() {
        let mut segs = Segments::new();
        segs.push(Segment::new("hello"));
        segs.push(Segment::new(" "));
        segs.push(Segment::new("world"));
        assert_eq!(segs.len(), 3);
        assert_eq!(segs.cell_len(), 11);
    }

    #[test]
    fn test_segments_from_iter() {
        let segs: Segments = vec![
            Segment::new("a"),
            Segment::new("b"),
            Segment::new("c"),
        ]
        .into_iter()
        .collect();
        assert_eq!(segs.len(), 3);
    }
}
