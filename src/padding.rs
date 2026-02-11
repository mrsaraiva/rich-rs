//! Padding: CSS-style spacing wrapper for renderables.
//!
//! Padding draws space around content, similar to CSS padding.
//!
//! # Example
//!
//! ```ignore
//! use rich_rs::{Padding, Text, Style, SimpleColor};
//!
//! let text = Text::plain("Hello, World!");
//! let padded = Padding::new(Box::new(text), (2, 4))
//!     .with_style(Style::new().with_bgcolor(SimpleColor::Standard(4)));
//! ```

use std::io::Stdout;

use crate::console::ConsoleOptions;
use crate::measure::Measurement;
use crate::segment::{Segment, Segments};
use crate::style::Style;
use crate::{Console, Renderable};

/// CSS-style padding dimensions.
///
/// Supports three forms:
/// - Single value: all sides use the same padding
/// - Two values: (vertical, horizontal) - top/bottom and left/right
/// - Four values: (top, right, bottom, left) - CSS order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddingDimensions {
    /// All sides use the same padding.
    All(usize),
    /// (vertical, horizontal) - top/bottom share one value, left/right share another.
    TwoWay(usize, usize),
    /// (top, right, bottom, left) - CSS order, all specified individually.
    FourWay(usize, usize, usize, usize),
}

impl PaddingDimensions {
    /// Unpack padding dimensions to (top, right, bottom, left).
    ///
    /// This follows CSS padding order.
    pub fn unpack(&self) -> (usize, usize, usize, usize) {
        match *self {
            PaddingDimensions::All(v) => (v, v, v, v),
            PaddingDimensions::TwoWay(vert, horiz) => (vert, horiz, vert, horiz),
            PaddingDimensions::FourWay(top, right, bottom, left) => (top, right, bottom, left),
        }
    }
}

impl From<usize> for PaddingDimensions {
    fn from(v: usize) -> Self {
        PaddingDimensions::All(v)
    }
}

impl From<(usize,)> for PaddingDimensions {
    fn from(v: (usize,)) -> Self {
        PaddingDimensions::All(v.0)
    }
}

impl From<(usize, usize)> for PaddingDimensions {
    fn from(v: (usize, usize)) -> Self {
        PaddingDimensions::TwoWay(v.0, v.1)
    }
}

impl From<(usize, usize, usize, usize)> for PaddingDimensions {
    fn from(v: (usize, usize, usize, usize)) -> Self {
        PaddingDimensions::FourWay(v.0, v.1, v.2, v.3)
    }
}

/// Draw space around content.
///
/// Padding wraps a renderable and adds blank space around it, similar to CSS padding.
///
/// # Example
///
/// ```ignore
/// use rich_rs::{Padding, Text, Style, SimpleColor};
///
/// let text = Text::plain("Hello, World!");
/// // Create padding with 2 lines top/bottom, 4 spaces left/right
/// let padded = Padding::new(Box::new(text), (2, 4))
///     .with_style(Style::new().with_bgcolor(SimpleColor::Standard(4)));
/// ```
pub struct Padding {
    /// The wrapped renderable.
    renderable: Box<dyn Renderable + Send + Sync>,
    /// Padding at the top (number of blank lines).
    top: usize,
    /// Padding on the right (number of spaces).
    right: usize,
    /// Padding at the bottom (number of blank lines).
    bottom: usize,
    /// Padding on the left (number of spaces).
    left: usize,
    /// Style for padding characters.
    style: Style,
    /// Whether to expand to fill available width (default true).
    expand: bool,
}

impl std::fmt::Debug for Padding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Padding")
            .field("top", &self.top)
            .field("right", &self.right)
            .field("bottom", &self.bottom)
            .field("left", &self.left)
            .field("style", &self.style)
            .field("expand", &self.expand)
            .finish_non_exhaustive()
    }
}

impl Padding {
    /// Create a new Padding wrapper.
    ///
    /// # Arguments
    ///
    /// * `renderable` - The content to wrap.
    /// * `pad` - Padding dimensions (CSS-style: 1, 2, or 4 values).
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rich_rs::{Padding, Text};
    ///
    /// let text = Text::plain("Hello");
    /// // Single value: all sides same
    /// let p1 = Padding::new(Box::new(text.clone()), 2);
    /// // Two values: (vertical, horizontal)
    /// let p2 = Padding::new(Box::new(text.clone()), (1, 4));
    /// // Four values: (top, right, bottom, left)
    /// let p3 = Padding::new(Box::new(text), (1, 2, 3, 4));
    /// ```
    pub fn new(
        renderable: Box<dyn Renderable + Send + Sync>,
        pad: impl Into<PaddingDimensions>,
    ) -> Self {
        let (top, right, bottom, left) = pad.into().unpack();
        Padding {
            renderable,
            top,
            right,
            bottom,
            left,
            style: Style::default(),
            expand: true,
        }
    }

    /// Create a Padding that indents content (left padding only).
    ///
    /// This is a convenience method for creating left-only indentation.
    /// The expand flag is set to false.
    ///
    /// # Arguments
    ///
    /// * `renderable` - The content to indent.
    /// * `level` - Number of spaces to indent.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rich_rs::{Padding, Text};
    ///
    /// let text = Text::plain("Indented text");
    /// let indented = Padding::indent(Box::new(text), 4);
    /// ```
    pub fn indent(renderable: Box<dyn Renderable + Send + Sync>, level: usize) -> Self {
        Padding {
            renderable,
            top: 0,
            right: 0,
            bottom: 0,
            left: level,
            style: Style::default(),
            expand: false,
        }
    }

    /// Unpack padding dimensions to (top, right, bottom, left).
    ///
    /// This is the CSS-style unpacking function that can be used
    /// independently of creating a Padding struct.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rich_rs::Padding;
    ///
    /// // Single value
    /// assert_eq!(Padding::unpack(2), (2, 2, 2, 2));
    /// // Two values
    /// assert_eq!(Padding::unpack((1, 4)), (1, 4, 1, 4));
    /// // Four values
    /// assert_eq!(Padding::unpack((1, 2, 3, 4)), (1, 2, 3, 4));
    /// ```
    pub fn unpack(pad: impl Into<PaddingDimensions>) -> (usize, usize, usize, usize) {
        pad.into().unpack()
    }

    /// Set the style for padding characters.
    ///
    /// # Arguments
    ///
    /// * `style` - Style to apply to padding spaces.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set whether to expand to fill available width.
    ///
    /// When true (default), the padding expands to fill `max_width`.
    /// When false, the width is based on the inner content's measurement.
    ///
    /// # Arguments
    ///
    /// * `expand` - Whether to expand to fill width.
    pub fn with_expand(mut self, expand: bool) -> Self {
        self.expand = expand;
        self
    }

    /// Get the top padding.
    pub fn top(&self) -> usize {
        self.top
    }

    /// Get the right padding.
    pub fn right(&self) -> usize {
        self.right
    }

    /// Get the bottom padding.
    pub fn bottom(&self) -> usize {
        self.bottom
    }

    /// Get the left padding.
    pub fn left(&self) -> usize {
        self.left
    }

    /// Get the style.
    pub fn style(&self) -> Style {
        self.style
    }

    /// Get whether expand is enabled.
    pub fn expand(&self) -> bool {
        self.expand
    }
}

impl Renderable for Padding {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let mut result = Segments::new();

        // Determine the width to use
        let width = if self.expand {
            options.max_width
        } else {
            // Measure inner content and add padding
            let inner_measurement = self.renderable.measure(console, options);
            (inner_measurement.maximum + self.left + self.right).min(options.max_width)
        };

        // Calculate inner width (width available for content)
        // If padding exceeds available width, clamp it proportionally
        let total_padding = self.left + self.right;
        let (effective_left, effective_right, inner_width) = if total_padding >= width {
            // No room for content, collapse padding proportionally to original ratio
            if total_padding == 0 {
                (0, 0, width)
            } else {
                let ratio_left = self.left as f64 / total_padding as f64;
                let scaled_left = (width as f64 * ratio_left).round() as usize;
                let scaled_right = width.saturating_sub(scaled_left);
                (scaled_left, scaled_right, 0)
            }
        } else {
            (self.left, self.right, width - total_padding)
        };

        // Create render options for inner content
        let render_options = options.update_width(inner_width);

        // Adjust height if specified
        let render_options = if let Some(h) = render_options.height {
            let new_height = h.saturating_sub(self.top + self.bottom);
            render_options.update_height(new_height)
        } else {
            render_options
        };

        // Render inner content to lines.
        // Python Rich passes the padding style to render_lines, which applies it as a
        // base style to all content. This ensures background colors extend across the
        // full width including content (not just padding spaces).
        let style_arg = if self.style.is_null() {
            None
        } else {
            Some(self.style)
        };
        let lines = console.render_lines(
            self.renderable.as_ref(),
            Some(&render_options),
            style_arg, // Apply padding style to content (matches Python Rich)
            true,      // pad=true
            false,     // new_lines=false (we add them ourselves)
        );

        // Create padding segments (using clamped values)
        let left_padding = if effective_left > 0 {
            Some(Segment::styled(" ".repeat(effective_left), self.style))
        } else {
            None
        };

        let right_padding_and_newline = if effective_right > 0 {
            vec![
                Segment::styled(" ".repeat(effective_right), self.style),
                Segment::line(),
            ]
        } else {
            vec![Segment::line()]
        };

        // Create blank line for top/bottom padding
        let blank_line = Segment::styled(format!("{}\n", " ".repeat(width)), self.style);

        // Add top padding
        for _ in 0..self.top {
            result.push(blank_line.clone());
        }

        // Add content lines with left/right padding
        for line in lines {
            if let Some(ref left) = left_padding {
                result.push(left.clone());
            }
            for segment in line {
                result.push(segment);
            }
            for segment in &right_padding_and_newline {
                result.push(segment.clone());
            }
        }

        // Add bottom padding
        for _ in 0..self.bottom {
            result.push(blank_line.clone());
        }

        result
    }

    fn measure(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        let max_width = options.max_width;
        let extra_width = self.left + self.right;

        // If there's not enough room for content, return max_width
        if max_width < extra_width + 1 {
            return Measurement::new(max_width, max_width);
        }

        // Measure inner content
        let inner_measurement = self.renderable.measure(console, options);

        // Add padding to measurement
        let measurement = Measurement::new(
            inner_measurement.minimum + extra_width,
            inner_measurement.maximum + extra_width,
        );

        // Clamp to max_width
        measurement.with_maximum(max_width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Text;

    // ==================== PaddingDimensions tests ====================

    #[test]
    fn test_unpack_single_value() {
        assert_eq!(Padding::unpack(5), (5, 5, 5, 5));
    }

    #[test]
    fn test_unpack_single_tuple() {
        assert_eq!(Padding::unpack((5,)), (5, 5, 5, 5));
    }

    #[test]
    fn test_unpack_two_values() {
        // (vertical, horizontal)
        assert_eq!(Padding::unpack((2, 4)), (2, 4, 2, 4));
    }

    #[test]
    fn test_unpack_four_values() {
        // (top, right, bottom, left) - CSS order
        assert_eq!(Padding::unpack((1, 2, 3, 4)), (1, 2, 3, 4));
    }

    #[test]
    fn test_unpack_zero() {
        assert_eq!(Padding::unpack(0), (0, 0, 0, 0));
    }

    // ==================== Padding creation tests ====================

    #[test]
    fn test_padding_new() {
        let text = Text::plain("Hello");
        let padding = Padding::new(Box::new(text), (1, 2, 3, 4));
        assert_eq!(padding.top(), 1);
        assert_eq!(padding.right(), 2);
        assert_eq!(padding.bottom(), 3);
        assert_eq!(padding.left(), 4);
        assert!(padding.expand());
    }

    #[test]
    fn test_padding_indent() {
        let text = Text::plain("Hello");
        let padding = Padding::indent(Box::new(text), 4);
        assert_eq!(padding.top(), 0);
        assert_eq!(padding.right(), 0);
        assert_eq!(padding.bottom(), 0);
        assert_eq!(padding.left(), 4);
        assert!(!padding.expand());
    }

    #[test]
    fn test_padding_with_style() {
        let text = Text::plain("Hello");
        let style = Style::new().with_bold(true);
        let padding = Padding::new(Box::new(text), 1).with_style(style);
        assert_eq!(padding.style().bold, Some(true));
    }

    #[test]
    fn test_padding_with_expand() {
        let text = Text::plain("Hello");
        let padding = Padding::new(Box::new(text), 1).with_expand(false);
        assert!(!padding.expand());
    }

    // ==================== Padding render tests ====================

    #[test]
    fn test_padding_render_basic() {
        let text = Text::plain("Hello");
        let padding = Padding::new(Box::new(text), (0, 2, 0, 2));
        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = padding.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should have left padding + "Hello" + right padding + newline
        assert!(output.contains("  Hello")); // 2 spaces left + Hello
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn test_padding_render_with_top_bottom() {
        let text = Text::plain("X");
        let padding = Padding::new(Box::new(text), (1, 0, 1, 0));
        let console = Console::with_options(ConsoleOptions {
            max_width: 10,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = padding.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let lines: Vec<&str> = output.lines().collect();

        // Should have blank line, content line, blank line (3 lines total)
        // But the last line may not have a trailing newline visible in lines()
        assert!(lines.len() >= 2); // At least top blank + content
    }

    #[test]
    fn test_padding_render_expand_true() {
        let text = Text::plain("Hi");
        let padding = Padding::new(Box::new(text), (0, 0, 0, 0)).with_expand(true);
        let console = Console::with_options(ConsoleOptions {
            max_width: 10,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = padding.render(&console, &options);

        // When expand=true and pad=true, the line should be padded to max_width
        let total_text: String = segments
            .iter()
            .filter(|s| !s.text.contains('\n'))
            .map(|s| s.text.to_string())
            .collect();

        // The content should be "Hi" padded to 10 characters
        assert_eq!(crate::cells::cell_len(&total_text), 10);
    }

    #[test]
    fn test_padding_render_expand_false() {
        let text = Text::plain("Hi");
        let padding = Padding::new(Box::new(text), (0, 1, 0, 1)).with_expand(false);
        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = padding.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // With expand=false, width = content_max(2) + left(1) + right(1) = 4
        // So we should have: " Hi " + newline
        assert!(output.contains(" Hi ")); // left + Hi + right
    }

    // ==================== Padding measure tests ====================

    #[test]
    fn test_padding_measure_basic() {
        let text = Text::plain("Hello"); // 5 chars
        let padding = Padding::new(Box::new(text), (0, 2, 0, 2)); // +4 horizontal
        let console = Console::with_options(ConsoleOptions {
            max_width: 80,
            ..Default::default()
        });
        let options = console.options().clone();

        let measurement = padding.measure(&console, &options);
        // "Hello" has min=max=5 (no spaces)
        // With left=2, right=2, we add 4 to both
        assert_eq!(measurement.minimum, 9);
        assert_eq!(measurement.maximum, 9);
    }

    #[test]
    fn test_padding_measure_with_words() {
        let text = Text::plain("Hello World"); // min=5 (longest word), max=11
        let padding = Padding::new(Box::new(text), (0, 1, 0, 1)); // +2 horizontal
        let console = Console::with_options(ConsoleOptions {
            max_width: 80,
            ..Default::default()
        });
        let options = console.options().clone();

        let measurement = padding.measure(&console, &options);
        assert_eq!(measurement.minimum, 7); // 5 + 2
        assert_eq!(measurement.maximum, 13); // 11 + 2
    }

    #[test]
    fn test_padding_measure_clamped() {
        let text = Text::plain("Hello World");
        let padding = Padding::new(Box::new(text), (0, 2, 0, 2)); // +4 horizontal
        let console = Console::with_options(ConsoleOptions {
            max_width: 10,
            ..Default::default()
        });
        let options = console.options().clone();

        let measurement = padding.measure(&console, &options);
        // max should be clamped to max_width
        assert!(measurement.maximum <= 10);
    }

    #[test]
    fn test_padding_measure_insufficient_width() {
        let text = Text::plain("Hi");
        let padding = Padding::new(Box::new(text), (0, 5, 0, 5)); // +10 horizontal
        let console = Console::with_options(ConsoleOptions {
            max_width: 8,
            ..Default::default()
        });
        let options = console.options().clone();

        let measurement = padding.measure(&console, &options);
        // When max_width < extra_width + 1, return max_width for both
        assert_eq!(measurement.minimum, 8);
        assert_eq!(measurement.maximum, 8);
    }

    #[test]
    fn test_padding_style_applies_to_content() {
        use crate::color::SimpleColor;

        let text = Text::plain("Hi");
        let style = Style::new().with_bgcolor(SimpleColor::Standard(4)); // blue bg
        let padding = Padding::new(Box::new(text), (0, 0, 0, 0)).with_style(style);
        let console = Console::with_options(ConsoleOptions {
            max_width: 10,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = padding.render(&console, &options);

        // Content segments should have the padding style applied (blue background)
        let content_seg = segments.iter().find(|s| s.text.contains("Hi"));
        assert!(content_seg.is_some(), "Should find 'Hi' segment");
        let seg = content_seg.unwrap();
        let seg_style = seg.style.unwrap_or_default();
        assert!(
            seg_style.bgcolor.is_some(),
            "Content should have background color from padding style"
        );
    }

    // ==================== Send + Sync tests ====================

    #[test]
    fn test_padding_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Padding>();
        assert_sync::<Padding>();
    }

    #[test]
    fn test_padding_dimensions_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<PaddingDimensions>();
        assert_sync::<PaddingDimensions>();
    }

    // ==================== Debug tests ====================

    #[test]
    fn test_padding_debug() {
        let text = Text::plain("Hello");
        let padding = Padding::new(Box::new(text), (1, 2, 3, 4));
        let debug_str = format!("{:?}", padding);
        assert!(debug_str.contains("Padding"));
        assert!(debug_str.contains("top: 1"));
        assert!(debug_str.contains("right: 2"));
        assert!(debug_str.contains("bottom: 3"));
        assert!(debug_str.contains("left: 4"));
    }
}
