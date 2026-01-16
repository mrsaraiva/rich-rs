//! Text: rich text with styled spans.

use crate::cells::cell_len;
use crate::style::Style;

/// A span of styled text within a Text object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// Start index (character offset).
    pub start: usize,
    /// End index (exclusive).
    pub end: usize,
    /// Style to apply.
    pub style: Style,
}

impl Span {
    /// Create a new span.
    pub fn new(start: usize, end: usize, style: Style) -> Self {
        Span { start, end, style }
    }
}

/// Rich text with styled spans.
///
/// Text is the primary way to work with styled content in Rich.
#[derive(Debug, Clone, Default)]
pub struct Text {
    /// The plain text content.
    text: String,
    /// Styled spans applied to the text.
    spans: Vec<Span>,
}

impl Text {
    /// Create new empty text.
    pub fn new() -> Self {
        Text::default()
    }

    /// Create text from a plain string.
    pub fn plain(text: impl Into<String>) -> Self {
        Text {
            text: text.into(),
            spans: Vec::new(),
        }
    }

    /// Create text with a style applied to the entire content.
    pub fn styled(text: impl Into<String>, style: Style) -> Self {
        let text = text.into();
        let len = text.chars().count();
        Text {
            text,
            spans: vec![Span::new(0, len, style)],
        }
    }

    /// Get the plain text content.
    pub fn plain_text(&self) -> &str {
        &self.text
    }

    /// Get the cell width of the text.
    pub fn cell_len(&self) -> usize {
        cell_len(&self.text)
    }

    /// Get the character count.
    pub fn len(&self) -> usize {
        self.text.chars().count()
    }

    /// Check if the text is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Append text with an optional style.
    pub fn append(&mut self, text: impl Into<String>, style: Option<Style>) {
        let text = text.into();
        let start = self.len();
        let end = start + text.chars().count();

        self.text.push_str(&text);

        if let Some(s) = style {
            self.spans.push(Span::new(start, end, s));
        }
    }

    /// Apply a style to a range of the text.
    pub fn stylize(&mut self, start: usize, end: usize, style: Style) {
        self.spans.push(Span::new(start, end, style));
    }

    /// Get the spans.
    pub fn spans(&self) -> &[Span] {
        &self.spans
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Text::plain(s)
    }
}

impl From<String> for Text {
    fn from(s: String) -> Self {
        Text::plain(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    #[test]
    fn test_text_plain() {
        let text = Text::plain("hello");
        assert_eq!(text.plain_text(), "hello");
        assert_eq!(text.len(), 5);
    }

    #[test]
    fn test_text_styled() {
        let style = Style::new().with_bold(true);
        let text = Text::styled("hello", style);
        assert_eq!(text.spans().len(), 1);
        assert_eq!(text.spans()[0].start, 0);
        assert_eq!(text.spans()[0].end, 5);
    }

    #[test]
    fn test_text_append() {
        let mut text = Text::new();
        text.append("hello ", None);
        text.append("world", Some(Style::new().with_bold(true)));
        assert_eq!(text.plain_text(), "hello world");
        assert_eq!(text.spans().len(), 1);
    }
}
