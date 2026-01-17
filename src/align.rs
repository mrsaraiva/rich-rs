//! Align: horizontal and vertical alignment wrapper for renderables.
//!
//! Align adds space around content to position it within a given width/height.
//!
//! # Example
//!
//! ```
//! use rich_rs::{Align, Text};
//!
//! // Center-align text
//! let text = Text::plain("Hello");
//! let aligned = Align::center(Box::new(text));
//!
//! // Right-align with custom width
//! let text = Text::plain("Right");
//! let aligned = Align::right(Box::new(text)).with_width(40);
//! ```

use std::io::Stdout;

use crate::console::ConsoleOptions;
use crate::measure::Measurement;
use crate::rule::AlignMethod;
use crate::segment::{Segment, Segments};
use crate::style::Style;
use crate::{Console, Renderable};

// ============================================================================
// VerticalAlignMethod
// ============================================================================

/// Vertical alignment method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlignMethod {
    /// Align to the top (default).
    #[default]
    Top,
    /// Align to the middle (vertically centered).
    Middle,
    /// Align to the bottom.
    Bottom,
}

impl VerticalAlignMethod {
    /// Parse a vertical alignment method from a string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "top" => Some(VerticalAlignMethod::Top),
            "middle" => Some(VerticalAlignMethod::Middle),
            "bottom" => Some(VerticalAlignMethod::Bottom),
            _ => None,
        }
    }
}

// ============================================================================
// Align
// ============================================================================

/// Align a renderable by adding spaces.
///
/// Align wraps a renderable and positions it within the available space
/// by adding padding. Supports both horizontal (left, center, right) and
/// vertical (top, middle, bottom) alignment.
///
/// # Example
///
/// ```
/// use rich_rs::{Align, Text, Style};
/// use rich_rs::align::VerticalAlignMethod;
///
/// // Simple center alignment
/// let text = Text::plain("Centered");
/// let aligned = Align::center(Box::new(text));
///
/// // Right-aligned with background style
/// let text = Text::plain("Right");
/// let aligned = Align::right(Box::new(text))
///     .with_style(Style::new().with_bold(true));
///
/// // Full alignment with vertical centering
/// let text = Text::plain("Middle");
/// let aligned = Align::center(Box::new(text))
///     .with_vertical(VerticalAlignMethod::Middle)
///     .with_height(10);
/// ```
pub struct Align {
    /// The wrapped renderable.
    renderable: Box<dyn Renderable + Send + Sync>,
    /// Horizontal alignment method.
    align: AlignMethod,
    /// Optional vertical alignment.
    vertical: Option<VerticalAlignMethod>,
    /// Style for padding spaces.
    style: Style,
    /// Whether to pad the right side (default: true).
    pad: bool,
    /// Optional fixed width constraint.
    width: Option<usize>,
    /// Optional fixed height constraint.
    height: Option<usize>,
}

impl std::fmt::Debug for Align {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Align")
            .field("align", &self.align)
            .field("vertical", &self.vertical)
            .field("style", &self.style)
            .field("pad", &self.pad)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl Align {
    /// Create a new Align wrapper with the specified alignment.
    ///
    /// # Arguments
    ///
    /// * `renderable` - The content to align.
    /// * `align` - Horizontal alignment method.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::{Align, Text, AlignMethod};
    ///
    /// let text = Text::plain("Hello");
    /// let aligned = Align::new(Box::new(text), AlignMethod::Center);
    /// ```
    pub fn new(renderable: Box<dyn Renderable + Send + Sync>, align: AlignMethod) -> Self {
        Align {
            renderable,
            align,
            vertical: None,
            style: Style::default(),
            pad: true,
            width: None,
            height: None,
        }
    }

    /// Create a left-aligned wrapper.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::{Align, Text};
    ///
    /// let text = Text::plain("Left-aligned");
    /// let aligned = Align::left(Box::new(text));
    /// ```
    pub fn left(renderable: Box<dyn Renderable + Send + Sync>) -> Self {
        Self::new(renderable, AlignMethod::Left)
    }

    /// Create a center-aligned wrapper.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::{Align, Text};
    ///
    /// let text = Text::plain("Centered");
    /// let aligned = Align::center(Box::new(text));
    /// ```
    pub fn center(renderable: Box<dyn Renderable + Send + Sync>) -> Self {
        Self::new(renderable, AlignMethod::Center)
    }

    /// Create a right-aligned wrapper.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::{Align, Text};
    ///
    /// let text = Text::plain("Right-aligned");
    /// let aligned = Align::right(Box::new(text));
    /// ```
    pub fn right(renderable: Box<dyn Renderable + Send + Sync>) -> Self {
        Self::new(renderable, AlignMethod::Right)
    }

    /// Set the style for padding spaces.
    ///
    /// The style is applied to the padding characters (spaces) used for alignment.
    /// This is useful for setting a background color on the aligned area.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the vertical alignment.
    ///
    /// Vertical alignment requires a height to be set (either via `with_height()`
    /// or from `ConsoleOptions::height`).
    pub fn with_vertical(mut self, vertical: VerticalAlignMethod) -> Self {
        self.vertical = Some(vertical);
        self
    }

    /// Set whether to pad the right side.
    ///
    /// When `true` (default), padding is added to the right to fill the width.
    /// When `false`, only left padding is added for center/right alignment.
    pub fn with_pad(mut self, pad: bool) -> Self {
        self.pad = pad;
        self
    }

    /// Set a fixed width constraint.
    ///
    /// The content will be constrained to this width. If not set, uses
    /// `ConsoleOptions::max_width`.
    pub fn with_width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    /// Set a fixed height constraint.
    ///
    /// Required for vertical alignment. If not set but vertical alignment is
    /// specified, falls back to `ConsoleOptions::height`.
    pub fn with_height(mut self, height: usize) -> Self {
        self.height = Some(height);
        self
    }

    /// Get the horizontal alignment.
    pub fn align(&self) -> AlignMethod {
        self.align
    }

    /// Get the vertical alignment.
    pub fn vertical(&self) -> Option<VerticalAlignMethod> {
        self.vertical
    }

    /// Get the style.
    pub fn style(&self) -> Style {
        self.style
    }

    /// Get whether padding is enabled.
    pub fn pad(&self) -> bool {
        self.pad
    }

    /// Get the width constraint.
    pub fn width(&self) -> Option<usize> {
        self.width
    }

    /// Get the height constraint.
    pub fn height(&self) -> Option<usize> {
        self.height
    }
}

// SAFETY: Align is Send + Sync because:
// - renderable: Box<dyn Renderable + Send + Sync> is explicitly Send + Sync
// - All other fields (AlignMethod, Option<VerticalAlignMethod>, Style, bool, Option<usize>)
//   are all Send + Sync
// The unsafe impl is technically redundant but makes the guarantees explicit.
unsafe impl Send for Align {}
unsafe impl Sync for Align {}

impl Renderable for Align {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let mut result = Segments::new();

        // Determine the available width
        let available_width = self.width.unwrap_or(options.max_width);

        // Measure the inner content to find its maximum width
        let inner_measurement = self.renderable.measure(console, options);
        let content_width = inner_measurement.maximum.min(available_width);

        // Create render options for inner content
        // Clear height constraint so inner content isn't constrained by Align's height
        let mut render_options = options.update_width(content_width);
        render_options.height = None;

        // Render inner content to lines
        let lines = console.render_lines(
            self.renderable.as_ref(),
            Some(&render_options),
            None,  // Don't apply style to content
            true,  // pad=true to normalize line widths
            false, // new_lines=false (we add them ourselves)
        );

        // Get the shape of the rendered content
        let (rendered_width, rendered_height) = Segment::get_shape(&lines);

        // Normalize lines to have consistent width
        let lines = Segment::set_shape(&lines, rendered_width, Some(rendered_height), None, false);

        let new_line = Segment::line();
        let excess_space = available_width.saturating_sub(rendered_width);

        // Generate horizontally aligned segments
        let generate_segments = |result: &mut Segments| {
            if excess_space == 0 {
                // Exact fit - no alignment needed
                for line in &lines {
                    for segment in line {
                        result.push(segment.clone());
                    }
                    result.push(new_line.clone());
                }
            } else {
                match self.align {
                    AlignMethod::Left => {
                        // Pad on the right
                        let pad_segment = if self.pad {
                            Some(Segment::styled(" ".repeat(excess_space), self.style))
                        } else {
                            None
                        };
                        for line in &lines {
                            for segment in line {
                                result.push(segment.clone());
                            }
                            if let Some(ref pad) = pad_segment {
                                result.push(pad.clone());
                            }
                            result.push(new_line.clone());
                        }
                    }
                    AlignMethod::Center => {
                        // Pad left and right
                        let left_padding = excess_space / 2;
                        let right_padding = excess_space - left_padding;

                        let left_segment = if left_padding > 0 {
                            Some(Segment::styled(" ".repeat(left_padding), self.style))
                        } else {
                            None
                        };
                        let right_segment = if self.pad && right_padding > 0 {
                            Some(Segment::styled(" ".repeat(right_padding), self.style))
                        } else {
                            None
                        };

                        for line in &lines {
                            if let Some(ref left) = left_segment {
                                result.push(left.clone());
                            }
                            for segment in line {
                                result.push(segment.clone());
                            }
                            if let Some(ref right) = right_segment {
                                result.push(right.clone());
                            }
                            result.push(new_line.clone());
                        }
                    }
                    AlignMethod::Right => {
                        // Pad on the left
                        let left_segment = Segment::styled(" ".repeat(excess_space), self.style);

                        for line in &lines {
                            result.push(left_segment.clone());
                            for segment in line {
                                result.push(segment.clone());
                            }
                            result.push(new_line.clone());
                        }
                    }
                }
            }
        };

        // Handle vertical alignment
        let vertical_height = self.height.or(options.height);

        if let (Some(v_align), Some(v_height)) = (self.vertical, vertical_height) {
            if v_height > rendered_height {
                // Create blank line for vertical padding
                let blank_width = if self.pad { available_width } else { 0 };
                let blank_line = if blank_width > 0 {
                    Segment::styled(format!("{}\n", " ".repeat(blank_width)), self.style)
                } else {
                    Segment::new("\n")
                };

                let blank_lines = |result: &mut Segments, count: usize| {
                    for _ in 0..count {
                        result.push(blank_line.clone());
                    }
                };

                match v_align {
                    VerticalAlignMethod::Top => {
                        generate_segments(&mut result);
                        let bottom_space = v_height.saturating_sub(rendered_height);
                        blank_lines(&mut result, bottom_space);
                    }
                    VerticalAlignMethod::Middle => {
                        let top_space = (v_height.saturating_sub(rendered_height)) / 2;
                        let bottom_space = v_height.saturating_sub(top_space).saturating_sub(rendered_height);
                        blank_lines(&mut result, top_space);
                        generate_segments(&mut result);
                        blank_lines(&mut result, bottom_space);
                    }
                    VerticalAlignMethod::Bottom => {
                        let top_space = v_height.saturating_sub(rendered_height);
                        blank_lines(&mut result, top_space);
                        generate_segments(&mut result);
                    }
                }
            } else {
                // Content fills or exceeds available height
                generate_segments(&mut result);
            }
        } else {
            // No vertical alignment
            generate_segments(&mut result);
        }

        // Apply style to all segments if set
        if self.style != Style::default() {
            Segment::apply_style_to_segments(result, Some(self.style), None)
        } else {
            result
        }
    }

    fn measure(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        // Align doesn't change the measurement of the inner content
        self.renderable.measure(console, options)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cells::cell_len;
    use crate::text::Text;

    // ==================== VerticalAlignMethod tests ====================

    #[test]
    fn test_vertical_align_method_parse() {
        assert_eq!(
            VerticalAlignMethod::parse("top"),
            Some(VerticalAlignMethod::Top)
        );
        assert_eq!(
            VerticalAlignMethod::parse("TOP"),
            Some(VerticalAlignMethod::Top)
        );
        assert_eq!(
            VerticalAlignMethod::parse("middle"),
            Some(VerticalAlignMethod::Middle)
        );
        assert_eq!(
            VerticalAlignMethod::parse("MIDDLE"),
            Some(VerticalAlignMethod::Middle)
        );
        assert_eq!(
            VerticalAlignMethod::parse("bottom"),
            Some(VerticalAlignMethod::Bottom)
        );
        assert_eq!(
            VerticalAlignMethod::parse("BOTTOM"),
            Some(VerticalAlignMethod::Bottom)
        );
        assert_eq!(VerticalAlignMethod::parse("invalid"), None);
    }

    #[test]
    fn test_vertical_align_method_default() {
        assert_eq!(VerticalAlignMethod::default(), VerticalAlignMethod::Top);
    }

    // ==================== Align construction tests ====================

    #[test]
    fn test_align_new() {
        let text = Text::plain("Hello");
        let align = Align::new(Box::new(text), AlignMethod::Center);
        assert_eq!(align.align(), AlignMethod::Center);
        assert_eq!(align.vertical(), None);
        assert!(align.pad());
        assert_eq!(align.width(), None);
        assert_eq!(align.height(), None);
    }

    #[test]
    fn test_align_left() {
        let text = Text::plain("Hello");
        let align = Align::left(Box::new(text));
        assert_eq!(align.align(), AlignMethod::Left);
    }

    #[test]
    fn test_align_center() {
        let text = Text::plain("Hello");
        let align = Align::center(Box::new(text));
        assert_eq!(align.align(), AlignMethod::Center);
    }

    #[test]
    fn test_align_right() {
        let text = Text::plain("Hello");
        let align = Align::right(Box::new(text));
        assert_eq!(align.align(), AlignMethod::Right);
    }

    #[test]
    fn test_align_with_style() {
        let text = Text::plain("Hello");
        let style = Style::new().with_bold(true);
        let align = Align::center(Box::new(text)).with_style(style);
        assert_eq!(align.style().bold, Some(true));
    }

    #[test]
    fn test_align_with_vertical() {
        let text = Text::plain("Hello");
        let align = Align::center(Box::new(text)).with_vertical(VerticalAlignMethod::Middle);
        assert_eq!(align.vertical(), Some(VerticalAlignMethod::Middle));
    }

    #[test]
    fn test_align_with_pad() {
        let text = Text::plain("Hello");
        let align = Align::center(Box::new(text)).with_pad(false);
        assert!(!align.pad());
    }

    #[test]
    fn test_align_with_width() {
        let text = Text::plain("Hello");
        let align = Align::center(Box::new(text)).with_width(40);
        assert_eq!(align.width(), Some(40));
    }

    #[test]
    fn test_align_with_height() {
        let text = Text::plain("Hello");
        let align = Align::center(Box::new(text)).with_height(10);
        assert_eq!(align.height(), Some(10));
    }

    // ==================== Align render tests ====================

    #[test]
    fn test_align_render_left() {
        let text = Text::plain("Hello");
        let align = Align::left(Box::new(text));
        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = align.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // "Hello" should be followed by padding (15 spaces) + newline
        assert!(output.starts_with("Hello"));
        assert!(output.ends_with('\n'));
        // Total width should be 20 (content + padding)
        let line = output.lines().next().unwrap();
        assert_eq!(cell_len(line), 20);
    }

    #[test]
    fn test_align_render_center() {
        let text = Text::plain("Hello"); // 5 chars
        let align = Align::center(Box::new(text));
        let console = Console::with_options(ConsoleOptions {
            max_width: 15,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = align.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let line = output.lines().next().unwrap();

        // With width 15 and content 5, we have 10 excess
        // Left padding: 5, right padding: 5
        assert!(line.starts_with("     ")); // 5 spaces
        assert!(line.contains("Hello"));
        assert_eq!(cell_len(line), 15);
    }

    #[test]
    fn test_align_render_right() {
        let text = Text::plain("Hello"); // 5 chars
        let align = Align::right(Box::new(text));
        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = align.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let line = output.lines().next().unwrap();

        // With width 20 and content 5, left padding should be 15
        assert!(line.starts_with("               ")); // 15 spaces
        assert!(line.ends_with("Hello"));
        assert_eq!(cell_len(line), 20);
    }

    #[test]
    fn test_align_render_no_pad() {
        let text = Text::plain("Hello");
        let align = Align::center(Box::new(text)).with_pad(false);
        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = align.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let line = output.lines().next().unwrap();

        // Center with no right padding: left padding only
        // 20 - 5 = 15 excess, left = 7, no right padding
        assert!(line.contains("Hello"));
        // Without right padding, line should be shorter than max_width
        assert!(cell_len(line) < 20);
    }

    #[test]
    fn test_align_render_with_width() {
        let text = Text::plain("Hello");
        let align = Align::center(Box::new(text)).with_width(10);
        let console = Console::with_options(ConsoleOptions {
            max_width: 50, // Larger than specified width
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = align.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let line = output.lines().next().unwrap();

        // Should use the specified width of 10, not max_width of 50
        assert_eq!(cell_len(line), 10);
    }

    #[test]
    fn test_align_render_exact_fit() {
        let text = Text::plain("Hello"); // 5 chars
        let align = Align::center(Box::new(text));
        let console = Console::with_options(ConsoleOptions {
            max_width: 5, // Exact fit
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = align.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let line = output.lines().next().unwrap();

        // No padding when content exactly fits
        assert_eq!(line, "Hello");
    }

    // ==================== Vertical alignment tests ====================

    #[test]
    fn test_align_render_vertical_top() {
        let text = Text::plain("X");
        let align = Align::center(Box::new(text))
            .with_vertical(VerticalAlignMethod::Top)
            .with_height(3)
            .with_width(5);
        let console = Console::new();
        let options = ConsoleOptions {
            max_width: 5,
            ..Default::default()
        };

        let segments = align.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let lines: Vec<&str> = output.lines().collect();

        // Should have 3 lines total: content at top, 2 blank lines below
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("X")); // Content at top
    }

    #[test]
    fn test_align_render_vertical_middle() {
        let text = Text::plain("X");
        let align = Align::center(Box::new(text))
            .with_vertical(VerticalAlignMethod::Middle)
            .with_height(5)
            .with_width(3);
        let console = Console::new();
        let options = ConsoleOptions {
            max_width: 3,
            ..Default::default()
        };

        let segments = align.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let lines: Vec<&str> = output.lines().collect();

        // Should have 5 lines: 2 blank, content, 2 blank (or similar)
        assert_eq!(lines.len(), 5);
        // Middle line(s) should contain content
        assert!(lines[2].contains("X")); // (5-1)/2 = 2
    }

    #[test]
    fn test_align_render_vertical_bottom() {
        let text = Text::plain("X");
        let align = Align::center(Box::new(text))
            .with_vertical(VerticalAlignMethod::Bottom)
            .with_height(3)
            .with_width(5);
        let console = Console::new();
        let options = ConsoleOptions {
            max_width: 5,
            ..Default::default()
        };

        let segments = align.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let lines: Vec<&str> = output.lines().collect();

        // Should have 3 lines: 2 blank lines above, content at bottom
        assert_eq!(lines.len(), 3);
        assert!(lines[2].contains("X")); // Content at bottom
    }

    // ==================== Measure tests ====================

    #[test]
    fn test_align_measure() {
        let text = Text::plain("Hello World"); // min=5 (World), max=11
        let align = Align::center(Box::new(text));
        let console = Console::new();
        let options = ConsoleOptions::default();

        let measurement = align.measure(&console, &options);
        // Align passes through the inner measurement
        assert_eq!(measurement.minimum, 5);
        assert_eq!(measurement.maximum, 11);
    }

    // ==================== Send + Sync tests ====================

    #[test]
    fn test_align_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Align>();
        assert_sync::<Align>();
    }

    #[test]
    fn test_vertical_align_method_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<VerticalAlignMethod>();
        assert_sync::<VerticalAlignMethod>();
    }

    // ==================== Debug tests ====================

    #[test]
    fn test_align_debug() {
        let text = Text::plain("Hello");
        let align = Align::center(Box::new(text))
            .with_vertical(VerticalAlignMethod::Middle)
            .with_height(10);
        let debug_str = format!("{:?}", align);
        assert!(debug_str.contains("Align"));
        assert!(debug_str.contains("Center"));
        assert!(debug_str.contains("Middle"));
    }

    // ==================== CJK and Unicode tests ====================

    #[test]
    fn test_align_cjk_content() {
        let text = Text::plain("你好"); // 4 cells
        let align = Align::center(Box::new(text));
        let console = Console::with_options(ConsoleOptions {
            max_width: 10,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = align.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let line = output.lines().next().unwrap();

        assert!(line.contains("你好"));
        assert_eq!(cell_len(line), 10);
    }

    #[test]
    fn test_align_emoji_content() {
        let text = Text::plain("Hi!"); // 4 cells with emoji (2) + "Hi" (2) - actually "Hi!" is 3
        let align = Align::right(Box::new(text));
        let console = Console::with_options(ConsoleOptions {
            max_width: 10,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = align.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let line = output.lines().next().unwrap();

        assert!(line.ends_with("Hi!"));
        assert_eq!(cell_len(line), 10);
    }
}
