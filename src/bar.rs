//! Bar: a horizontal bar renderable.
//!
//! Renders a solid block bar with smooth edges using Unicode block characters.
//! The bar can represent a portion of a range, useful for progress indicators,
//! charts, and graphical displays.
//!
//! # Example
//!
//! ```ignore
//! use rich_rs::Bar;
//! use rich_rs::SimpleColor;
//!
//! // Create a bar that fills from 25% to 75% of a 40-cell width
//! let bar = Bar::new(1.0, 0.25, 0.75)
//!     .with_width(40)
//!     .with_color(SimpleColor::Standard(1)); // Red
//! ```

use crate::color::SimpleColor;
use crate::measure::Measurement;
use crate::segment::{Segment, Segments};
use crate::style::Style;
use crate::{Console, ConsoleOptions, Renderable};

// Block elements for smooth rendering
// There are left-aligned characters for 1/8 to 7/8, but
// the right-aligned characters exist only for 1/8 and 4/8.
const BEGIN_BLOCK_ELEMENTS: [char; 8] = ['█', '█', '█', '▐', '▐', '▐', '▕', '▕'];
const END_BLOCK_ELEMENTS: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];
const FULL_BLOCK: char = '█';

/// A horizontal bar renderable using Unicode block characters.
///
/// The bar is rendered within the range [0, size], with filled portion
/// between [begin, end]. Uses Unicode block characters for smooth edges:
/// - Full block: █ (U+2588)
/// - Partial blocks for smooth edges
///
/// # Fields
///
/// * `size` - The total size/range of the bar (typically 1.0 or 100.0).
/// * `begin` - Starting position of the filled portion (0 to size).
/// * `end` - Ending position of the filled portion (0 to size).
/// * `width` - Optional width in cells. If None, uses max_width from options.
/// * `style` - Style containing foreground and background colors.
#[derive(Debug, Clone)]
pub struct Bar {
    /// The total size (value for end of the bar).
    pub size: f64,
    /// Begin point (between 0 and size, inclusive).
    begin: f64,
    /// End point (between 0 and size, inclusive).
    end: f64,
    /// Width of the bar, or None for maximum width.
    pub width: Option<usize>,
    /// Style for the bar (color and bgcolor).
    pub style: Style,
}

impl Bar {
    /// Create a new bar.
    ///
    /// # Arguments
    ///
    /// * `size` - The total size/range of the bar.
    /// * `begin` - Starting position of the filled portion (clamped to 0).
    /// * `end` - Ending position of the filled portion (clamped to size).
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rich_rs::Bar;
    ///
    /// // Create a bar that fills 50% to 100%
    /// let bar = Bar::new(1.0, 0.5, 1.0);
    /// ```
    pub fn new(size: f64, begin: f64, end: f64) -> Self {
        Bar {
            size,
            begin: begin.max(0.0),
            end: end.min(size),
            width: None,
            style: Style::default(),
        }
    }

    /// Set the width of the bar in cells.
    pub fn with_width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the foreground color of the bar.
    pub fn with_color(mut self, color: SimpleColor) -> Self {
        self.style = self.style.with_color(color);
        self
    }

    /// Set the background color of the bar.
    pub fn with_bgcolor(mut self, bgcolor: SimpleColor) -> Self {
        self.style = self.style.with_bgcolor(bgcolor);
        self
    }

    /// Set the style of the bar directly.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Renderable for Bar {
    fn render(&self, _console: &Console, options: &ConsoleOptions) -> Segments {
        let width = match self.width {
            Some(w) => w.min(options.max_width),
            None => options.max_width,
        };

        let mut segments = Segments::new();

        // If begin >= end or size is zero/negative, render empty bar (just spaces)
        if self.begin >= self.end || self.size <= 0.0 {
            let text = " ".repeat(width);
            segments.push(Segment::styled(text, self.style));
            segments.push(Segment::line());
            return segments;
        }

        // Calculate prefix (empty space before the bar)
        let prefix_complete_eights = (width as f64 * 8.0 * self.begin / self.size) as usize;
        let prefix_bar_count = prefix_complete_eights / 8;
        let prefix_eights_count = prefix_complete_eights % 8;

        // Calculate body (the filled portion)
        let body_complete_eights = (width as f64 * 8.0 * self.end / self.size) as usize;
        let body_bar_count = body_complete_eights / 8;
        let body_eights_count = body_complete_eights % 8;

        // Build prefix: spaces plus optional partial block at the beginning
        let mut prefix = " ".repeat(prefix_bar_count);
        if prefix_eights_count > 0 {
            prefix.push(BEGIN_BLOCK_ELEMENTS[prefix_eights_count]);
        }

        // Build body: full blocks plus optional partial block at the end
        let mut body = FULL_BLOCK.to_string().repeat(body_bar_count);
        if body_eights_count > 0 {
            body.push(END_BLOCK_ELEMENTS[body_eights_count]);
        }

        // Calculate suffix (empty space after the bar)
        let suffix = " ".repeat(width.saturating_sub(body.chars().count()));

        // Combine: prefix + (body with prefix stripped) + suffix
        // The body[prefix.len()..] skips the overlapping portion
        let prefix_len = prefix.chars().count();
        let body_remainder: String = body.chars().skip(prefix_len).collect();
        let bar_text = format!("{}{}{}", prefix, body_remainder, suffix);

        segments.push(Segment::styled(bar_text, self.style));
        segments.push(Segment::line());

        segments
    }

    fn measure(&self, _console: &Console, options: &ConsoleOptions) -> Measurement {
        match self.width {
            Some(w) => Measurement::exact(w),
            None => Measurement::new(4, options.max_width),
        }
    }
}

impl std::fmt::Display for Bar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bar({}, {}, {})", self.size, self.begin, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar_new() {
        let bar = Bar::new(1.0, 0.25, 0.75);
        assert_eq!(bar.size, 1.0);
        assert_eq!(bar.begin, 0.25);
        assert_eq!(bar.end, 0.75);
        assert!(bar.width.is_none());
    }

    #[test]
    fn test_bar_clamps_begin() {
        let bar = Bar::new(1.0, -0.5, 0.5);
        assert_eq!(bar.begin, 0.0);
    }

    #[test]
    fn test_bar_clamps_end() {
        let bar = Bar::new(1.0, 0.5, 1.5);
        assert_eq!(bar.end, 1.0);
    }

    #[test]
    fn test_bar_with_width() {
        let bar = Bar::new(1.0, 0.0, 1.0).with_width(40);
        assert_eq!(bar.width, Some(40));
    }

    #[test]
    fn test_bar_with_color() {
        let bar = Bar::new(1.0, 0.0, 1.0).with_color(SimpleColor::Standard(1));
        assert_eq!(bar.style.color, Some(SimpleColor::Standard(1)));
    }

    #[test]
    fn test_bar_display() {
        let bar = Bar::new(2.0, 0.5, 1.5);
        assert_eq!(format!("{}", bar), "Bar(2, 0.5, 1.5)");
    }

    #[test]
    fn test_bar_render_full() {
        let console = Console::new();
        let mut options = ConsoleOptions::default();
        options.max_width = 10;
        let bar = Bar::new(1.0, 0.0, 1.0).with_width(10);
        let segments = bar.render(&console, &options);

        // Should have 2 segments: the bar content and a newline
        assert_eq!(segments.len(), 2);

        // First segment should be full blocks
        let first = segments.iter().next().unwrap();
        assert!(first.text.contains('█'));
    }

    #[test]
    fn test_bar_render_empty() {
        let console = Console::new();
        let mut options = ConsoleOptions::default();
        options.max_width = 10;
        let bar = Bar::new(1.0, 0.5, 0.5).with_width(10); // begin == end
        let segments = bar.render(&console, &options);

        // Should render empty bar (spaces)
        let first = segments.iter().next().unwrap();
        assert_eq!(first.text.trim(), ""); // All spaces
    }

    #[test]
    fn test_bar_render_partial() {
        let console = Console::new();
        let mut options = ConsoleOptions::default();
        options.max_width = 20;
        let bar = Bar::new(1.0, 0.25, 0.75).with_width(20);
        let segments = bar.render(&console, &options);

        // Should have content
        let first = segments.iter().next().unwrap();
        assert!(!first.text.is_empty());
    }

    #[test]
    fn test_bar_measure_with_width() {
        let console = Console::new();
        let mut options = ConsoleOptions::default();
        options.max_width = 100;
        let bar = Bar::new(1.0, 0.0, 1.0).with_width(40);
        let measurement = bar.measure(&console, &options);

        assert_eq!(measurement.minimum, 40);
        assert_eq!(measurement.maximum, 40);
    }

    #[test]
    fn test_bar_measure_without_width() {
        let console = Console::new();
        let mut options = ConsoleOptions::default();
        options.max_width = 80;
        let bar = Bar::new(1.0, 0.0, 1.0);
        let measurement = bar.measure(&console, &options);

        assert_eq!(measurement.minimum, 4);
        assert_eq!(measurement.maximum, 80);
    }

    #[test]
    fn test_bar_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Bar>();
        assert_sync::<Bar>();
    }
}
