//! Style: text formatting attributes.
//!
//! Styles are immutable and can be combined using the `+` operator or `combine` method.

use crate::color::Color;

/// Text style with color and attributes.
///
/// Uses `Option<bool>` for attributes to support three states:
/// - `None`: inherit from parent
/// - `Some(true)`: enable
/// - `Some(false)`: disable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    /// Foreground color.
    pub color: Option<Color>,
    /// Background color.
    pub bgcolor: Option<Color>,
    /// Bold text.
    pub bold: Option<bool>,
    /// Dim/faint text.
    pub dim: Option<bool>,
    /// Italic text.
    pub italic: Option<bool>,
    /// Underlined text.
    pub underline: Option<bool>,
    /// Blinking text.
    pub blink: Option<bool>,
    /// Reverse video (swap fg/bg).
    pub reverse: Option<bool>,
    /// Strikethrough text.
    pub strike: Option<bool>,
}

impl Style {
    /// Create a new empty style.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a style with a foreground color.
    pub fn color(color: Color) -> Self {
        Self {
            color: Some(color),
            ..Default::default()
        }
    }

    /// Create a style with a background color.
    pub fn bgcolor(color: Color) -> Self {
        Self {
            bgcolor: Some(color),
            ..Default::default()
        }
    }

    /// Builder: set foreground color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Builder: set background color.
    pub fn with_bgcolor(mut self, color: Color) -> Self {
        self.bgcolor = Some(color);
        self
    }

    /// Builder: set bold.
    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = Some(bold);
        self
    }

    /// Builder: set dim.
    pub fn with_dim(mut self, dim: bool) -> Self {
        self.dim = Some(dim);
        self
    }

    /// Builder: set italic.
    pub fn with_italic(mut self, italic: bool) -> Self {
        self.italic = Some(italic);
        self
    }

    /// Builder: set underline.
    pub fn with_underline(mut self, underline: bool) -> Self {
        self.underline = Some(underline);
        self
    }

    /// Builder: set strike.
    pub fn with_strike(mut self, strike: bool) -> Self {
        self.strike = Some(strike);
        self
    }

    /// Combine this style with another, with `other` taking precedence.
    ///
    /// Values from `other` override values from `self` only if they are `Some`.
    pub fn combine(&self, other: &Style) -> Self {
        Style {
            color: other.color.or(self.color),
            bgcolor: other.bgcolor.or(self.bgcolor),
            bold: other.bold.or(self.bold),
            dim: other.dim.or(self.dim),
            italic: other.italic.or(self.italic),
            underline: other.underline.or(self.underline),
            blink: other.blink.or(self.blink),
            reverse: other.reverse.or(self.reverse),
            strike: other.strike.or(self.strike),
        }
    }

    /// Parse a style from a string.
    ///
    /// Supports space-separated style definitions like:
    /// - "bold red on blue"
    /// - "italic #ff0000"
    /// - "bold underline"
    pub fn parse(s: &str) -> Option<Self> {
        let mut style = Style::new();
        let mut on_background = false;

        for word in s.split_whitespace() {
            let word_lower = word.to_lowercase();

            if word_lower == "on" {
                on_background = true;
                continue;
            }

            // Check for attributes
            match word_lower.as_str() {
                "bold" => style.bold = Some(true),
                "dim" => style.dim = Some(true),
                "italic" => style.italic = Some(true),
                "underline" => style.underline = Some(true),
                "blink" => style.blink = Some(true),
                "reverse" => style.reverse = Some(true),
                "strike" => style.strike = Some(true),
                "not bold" => style.bold = Some(false),
                "not dim" => style.dim = Some(false),
                "not italic" => style.italic = Some(false),
                "not underline" => style.underline = Some(false),
                _ => {
                    // Try to parse as color
                    if let Some(color) = Color::parse(&word_lower) {
                        if on_background {
                            style.bgcolor = Some(color);
                            on_background = false;
                        } else {
                            style.color = Some(color);
                        }
                    }
                }
            }
        }

        Some(style)
    }
}

impl std::ops::Add for Style {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        self.combine(&other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_builder() {
        let style = Style::new().with_bold(true).with_color(Color::Standard(1));
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.color, Some(Color::Standard(1)));
    }

    #[test]
    fn test_style_combine() {
        let base = Style::new().with_bold(true);
        let overlay = Style::new().with_italic(true);
        let combined = base.combine(&overlay);
        assert_eq!(combined.bold, Some(true));
        assert_eq!(combined.italic, Some(true));
    }

    #[test]
    fn test_style_parse() {
        let style = Style::parse("bold red").unwrap();
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.color, Some(Color::Standard(1)));
    }
}
