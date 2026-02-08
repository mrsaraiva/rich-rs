//! Panel: a bordered box around content.
//!
//! Panel wraps any renderable in a border with optional title and subtitle.
//!
//! # Example
//!
//! ```
//! use rich_rs::{Panel, Text};
//!
//! let text = Text::plain("Hello, World!");
//! let panel = Panel::new(Box::new(text))
//!     .with_title("My Panel")
//!     .with_box(rich_rs::r#box::ROUNDED);
//! ```

use std::io::Stdout;

use crate::r#box::{Box as RichBox, ROUNDED};
use crate::console::ConsoleOptions;
use crate::measure::{Measurement, measure_renderables};
use crate::padding::PaddingDimensions;
use crate::rule::AlignMethod;
use crate::segment::{Segment, Segments};
use crate::style::Style;
use crate::text::Text;
use crate::{Console, Renderable};

// ============================================================================
// Panel
// ============================================================================

/// A bordered box around content.
///
/// Panel draws a border around any renderable, with optional title and subtitle.
///
/// # Example
///
/// ```
/// use rich_rs::{Panel, Text, Style};
/// use rich_rs::r#box::DOUBLE;
///
/// let text = Text::plain("Hello, World!");
/// let panel = Panel::new(Box::new(text))
///     .with_title("Title")
///     .with_subtitle("Subtitle")
///     .with_box(DOUBLE)
///     .with_border_style(Style::new().with_bold(true));
/// ```
pub struct Panel {
    /// The content to wrap.
    renderable: Box<dyn Renderable + Send + Sync>,
    /// Border style (box drawing characters).
    box_type: RichBox,
    /// Optional title displayed in the header.
    title: Option<Text>,
    /// Optional subtitle displayed in the footer.
    subtitle: Option<Text>,
    /// Title alignment.
    title_align: AlignMethod,
    /// Subtitle alignment.
    subtitle_align: AlignMethod,
    /// Use ASCII-safe box characters (None = use console default).
    safe_box: Option<bool>,
    /// Expand to fill available width.
    expand: bool,
    /// Style for the panel (border and content background).
    style: Style,
    /// Style for the border only.
    border_style: Style,
    /// Fixed width (None = auto).
    width: Option<usize>,
    /// Fixed height (None = auto).
    height: Option<usize>,
    /// Inner padding (top, right, bottom, left).
    padding: (usize, usize, usize, usize),
    /// Enable highlighting of content.
    highlight: bool,
}

impl std::fmt::Debug for Panel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Panel")
            .field("box_type", &self.box_type)
            .field("title", &self.title)
            .field("subtitle", &self.subtitle)
            .field("title_align", &self.title_align)
            .field("subtitle_align", &self.subtitle_align)
            .field("safe_box", &self.safe_box)
            .field("expand", &self.expand)
            .field("style", &self.style)
            .field("border_style", &self.border_style)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("padding", &self.padding)
            .field("highlight", &self.highlight)
            .finish_non_exhaustive()
    }
}

impl Panel {
    /// Create a new panel with default settings (expands to fill width).
    ///
    /// # Arguments
    ///
    /// * `renderable` - The content to wrap in the panel.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::{Panel, Text};
    ///
    /// let text = Text::plain("Hello");
    /// let panel = Panel::new(Box::new(text));
    /// ```
    pub fn new(renderable: Box<dyn Renderable + Send + Sync>) -> Self {
        Panel {
            renderable,
            box_type: ROUNDED,
            title: None,
            subtitle: None,
            title_align: AlignMethod::Center,
            subtitle_align: AlignMethod::Center,
            safe_box: None,
            expand: true,
            style: Style::default(),
            border_style: Style::default(),
            width: None,
            height: None,
            padding: (0, 1, 0, 1), // Default: 1 space left/right padding
            highlight: false,
        }
    }

    /// Create a panel that fits its content (expand=false).
    ///
    /// # Arguments
    ///
    /// * `renderable` - The content to wrap in the panel.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::{Panel, Text};
    ///
    /// let text = Text::plain("Hello");
    /// let panel = Panel::fit(Box::new(text));
    /// ```
    pub fn fit(renderable: Box<dyn Renderable + Send + Sync>) -> Self {
        let mut panel = Self::new(renderable);
        panel.expand = false;
        panel
    }

    /// Set the box style for the border.
    pub fn with_box(mut self, box_type: RichBox) -> Self {
        self.box_type = box_type;
        self
    }

    /// Set the title displayed in the panel header.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        let title_str = title.into();
        // Try to parse as markup, fall back to plain
        let title_text =
            Text::from_markup(&title_str, false).unwrap_or_else(|_| Text::plain(&title_str));
        self.title = Some(title_text);
        self
    }

    /// Set the title using a Text object directly.
    pub fn with_title_text(mut self, title: Text) -> Self {
        self.title = Some(title);
        self
    }

    /// Set the subtitle displayed in the panel footer.
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        let subtitle_str = subtitle.into();
        let subtitle_text =
            Text::from_markup(&subtitle_str, false).unwrap_or_else(|_| Text::plain(&subtitle_str));
        self.subtitle = Some(subtitle_text);
        self
    }

    /// Set the subtitle using a Text object directly.
    pub fn with_subtitle_text(mut self, subtitle: Text) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    /// Set the title alignment.
    pub fn with_title_align(mut self, align: AlignMethod) -> Self {
        self.title_align = align;
        self
    }

    /// Set the subtitle alignment.
    pub fn with_subtitle_align(mut self, align: AlignMethod) -> Self {
        self.subtitle_align = align;
        self
    }

    /// Set whether to use ASCII-safe box characters.
    pub fn with_safe_box(mut self, safe: bool) -> Self {
        self.safe_box = Some(safe);
        self
    }

    /// Set whether to expand to fill available width.
    pub fn with_expand(mut self, expand: bool) -> Self {
        self.expand = expand;
        self
    }

    /// Set the panel style (applied to border and content background).
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the border style (color/attributes for border characters).
    pub fn with_border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    /// Set a fixed width for the panel.
    pub fn with_width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    /// Set a fixed height for the panel.
    pub fn with_height(mut self, height: usize) -> Self {
        self.height = Some(height);
        self
    }

    /// Set the inner padding.
    ///
    /// Accepts CSS-style padding: 1 value (all), 2 values (vertical, horizontal),
    /// or 4 values (top, right, bottom, left).
    pub fn with_padding(mut self, padding: impl Into<PaddingDimensions>) -> Self {
        self.padding = padding.into().unpack();
        self
    }

    /// Set whether to enable highlighting of content.
    pub fn with_highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    // ========================================================================
    // Getters
    // ========================================================================

    /// Get the box type.
    pub fn box_type(&self) -> RichBox {
        self.box_type
    }

    /// Get the title.
    pub fn title(&self) -> Option<&Text> {
        self.title.as_ref()
    }

    /// Get the subtitle.
    pub fn subtitle(&self) -> Option<&Text> {
        self.subtitle.as_ref()
    }

    /// Get whether expand is enabled.
    pub fn expand(&self) -> bool {
        self.expand
    }

    /// Get the style.
    pub fn style(&self) -> Style {
        self.style
    }

    /// Get the border style.
    pub fn border_style(&self) -> Style {
        self.border_style
    }

    /// Get the padding.
    pub fn padding(&self) -> (usize, usize, usize, usize) {
        self.padding
    }

    // ========================================================================
    // Internal helpers
    // ========================================================================

    /// Prepare title text for rendering (strip newlines, add padding).
    fn prepare_title(&self) -> Option<Text> {
        self.title.as_ref().map(|title| {
            let mut t = title.copy();
            // Replace newlines with spaces
            let plain = t.plain_text().replace('\n', " ");
            t = Text::plain(&plain);
            // Copy spans from original, adjusting for any length changes
            for span in title.spans() {
                if span.start < plain.len() && span.end > span.start {
                    t.stylize(span.start, span.end.min(plain.len()), span.style);
                }
            }
            if let Some(base) = title.base_style() {
                t.set_base_style(Some(base));
            }
            // Add 1 space padding on each side
            let mut padded = Text::plain(" ");
            padded.append_text(&t);
            padded.append(" ", None);
            padded
        })
    }

    /// Prepare subtitle text for rendering.
    fn prepare_subtitle(&self) -> Option<Text> {
        self.subtitle.as_ref().map(|subtitle| {
            let mut t = subtitle.copy();
            let plain = t.plain_text().replace('\n', " ");
            t = Text::plain(&plain);
            for span in subtitle.spans() {
                if span.start < plain.len() && span.end > span.start {
                    t.stylize(span.start, span.end.min(plain.len()), span.style);
                }
            }
            if let Some(base) = subtitle.base_style() {
                t.set_base_style(Some(base));
            }
            let mut padded = Text::plain(" ");
            padded.append_text(&t);
            padded.append(" ", None);
            padded
        })
    }

    /// Align title/subtitle text within available width.
    fn align_text(
        &self,
        text: &Text,
        width: usize,
        align: AlignMethod,
        fill_char: char,
        style: Style,
    ) -> Text {
        let mut t = text.copy();

        // Truncate if too long
        if t.cell_len() > width {
            t = t.truncate(width, crate::console::OverflowMethod::Crop, false);
        }

        let excess_space = width.saturating_sub(t.cell_len());
        if excess_space == 0 {
            return t;
        }

        // Apply border style as base before alignment
        t.stylize_before(style, 0, None);

        match align {
            AlignMethod::Left => {
                // Pad right with fill char
                let fill: String = std::iter::repeat(fill_char).take(excess_space).collect();
                let mut result = t;
                result.append(&fill, Some(style));
                result
            }
            AlignMethod::Center => {
                let left_pad = excess_space / 2;
                let right_pad = excess_space - left_pad;
                let left_fill: String = std::iter::repeat(fill_char).take(left_pad).collect();
                let right_fill: String = std::iter::repeat(fill_char).take(right_pad).collect();

                let mut result = Text::styled(&left_fill, style);
                result.append_text(&t);
                result.append(&right_fill, Some(style));
                result
            }
            AlignMethod::Right => {
                // Pad left with fill char
                let fill: String = std::iter::repeat(fill_char).take(excess_space).collect();
                let mut result = Text::styled(&fill, style);
                result.append_text(&t);
                result
            }
        }
    }
}


impl Renderable for Panel {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let mut result = Segments::new();

        // Unpack padding
        let (pad_top, pad_right, pad_bottom, pad_left) = self.padding;

        // Reference to inner renderable
        let renderable_ref: &dyn Renderable = self.renderable.as_ref();

        // Get styles
        let style = self.style;
        let border_style = style.combine(&self.border_style);

        // Determine panel width
        let max_width = match self.width {
            Some(w) => w.min(options.max_width),
            None => options.max_width,
        };

        // Determine safe_box setting
        let safe_box = self.safe_box.unwrap_or(options.legacy_windows);
        let box_chars = self.box_type.substitute(safe_box, options.ascii_only());

        // Prepare title and subtitle
        let title_text = self.prepare_title();
        let subtitle_text = self.prepare_subtitle();

        // Calculate child width (inner area between borders, including padding)
        let child_width = if self.expand {
            // When expanding, use full width minus borders
            max_width.saturating_sub(2)
        } else {
            // When fitting, measure content and add padding
            let render_opts =
                options.update_width(max_width.saturating_sub(2 + pad_left + pad_right));
            let content_width = console.measure(renderable_ref, Some(&render_opts)).maximum;
            // child_width = content + padding (fits snugly)
            content_width + pad_left + pad_right
        };

        // Adjust child width for title/subtitle if present
        // Title/subtitle fits in (width - 4) space, so child_width must be >= text_len + 2
        let mut child_width = child_width.min(max_width.saturating_sub(2));
        if let Some(ref t) = title_text {
            child_width = child_width.max(t.cell_len() + 2);
        }
        if let Some(ref s) = subtitle_text {
            child_width = child_width.max(s.cell_len() + 2);
        }
        let child_width = child_width.min(max_width.saturating_sub(2));

        // Calculate final panel width
        let width = child_width + 2;

        // Calculate child height
        let child_height = self.height.or(options.height).map(|h| h.saturating_sub(2));

        // Create child render options with padding applied
        let child_options = {
            let mut opts = options.update_width(child_width.saturating_sub(pad_left + pad_right));
            if let Some(h) = child_height {
                opts = opts.update_height(h.saturating_sub(pad_top + pad_bottom));
            }
            if self.highlight {
                opts.highlight = Some(true);
            }
            opts
        };

        // Render content lines
        let content_lines = if pad_top > 0 || pad_right > 0 || pad_bottom > 0 || pad_left > 0 {
            // Render inner content first
            let inner_lines = console.render_lines(
                renderable_ref,
                Some(&child_options),
                Some(style),
                true,
                false,
            );

            // Apply padding manually
            let mut padded_lines: Vec<Vec<Segment>> = Vec::new();
            let blank_line_width = child_width;
            let blank_segment = Segment::styled(" ".repeat(blank_line_width), style);

            // Top padding
            for _ in 0..pad_top {
                padded_lines.push(vec![blank_segment.clone()]);
            }

            // Content lines with left/right padding
            let left_pad = Segment::styled(" ".repeat(pad_left), style);
            let right_pad = Segment::styled(" ".repeat(pad_right), style);

            for line in inner_lines {
                let mut padded_line = Vec::new();
                if pad_left > 0 {
                    padded_line.push(left_pad.clone());
                }
                for seg in line {
                    padded_line.push(seg);
                }
                if pad_right > 0 {
                    padded_line.push(right_pad.clone());
                }
                // Adjust line to child_width
                padded_line =
                    Segment::adjust_line_length(&padded_line, child_width, Some(style), true);
                padded_lines.push(padded_line);
            }

            // Bottom padding
            for _ in 0..pad_bottom {
                padded_lines.push(vec![blank_segment.clone()]);
            }

            padded_lines
        } else {
            console.render_lines(
                renderable_ref,
                Some(&options.update_width(child_width)),
                Some(style),
                true,
                false,
            )
        };

        // Ensure lines match child_width
        let content_lines: Vec<Vec<Segment>> = content_lines
            .into_iter()
            .map(|line| Segment::adjust_line_length(&line, child_width, Some(style), true))
            .collect();

        // Handle height constraint
        let content_lines = if let Some(h) = child_height {
            Segment::set_shape(&content_lines, child_width, Some(h), Some(style), false)
        } else {
            content_lines
        };

        let new_line = Segment::line();

        // Render top border
        if title_text.is_none() || width <= 4 {
            // Simple top border
            let top = box_chars.get_top(&[child_width]);
            result.push(Segment::styled(top, border_style));
        } else {
            // Top border with title
            let title = title_text.as_ref().unwrap();
            let aligned_title = self.align_text(
                title,
                width.saturating_sub(4),
                self.title_align,
                box_chars.top,
                border_style,
            );

            // Left corner + one border char
            result.push(Segment::styled(
                format!("{}{}", box_chars.top_left, box_chars.top),
                border_style,
            ));

            // Render aligned title
            let title_segments =
                aligned_title.render(console, &options.update_width(width.saturating_sub(4)));
            for seg in title_segments {
                result.push(seg);
            }

            // One border char + right corner
            result.push(Segment::styled(
                format!("{}{}", box_chars.top, box_chars.top_right),
                border_style,
            ));
        }
        result.push(new_line.clone());

        // Render content lines with left/right borders
        let line_start = Segment::styled(box_chars.mid_left.to_string(), border_style);
        let line_end = Segment::styled(box_chars.mid_right.to_string(), border_style);

        for line in &content_lines {
            result.push(line_start.clone());
            for seg in line {
                result.push(seg.clone());
            }
            result.push(line_end.clone());
            result.push(new_line.clone());
        }

        // Render bottom border (subtitle_text was prepared earlier)
        if subtitle_text.is_none() || width <= 4 {
            // Simple bottom border
            let bottom = box_chars.get_bottom(&[child_width]);
            result.push(Segment::styled(bottom, border_style));
        } else {
            // Bottom border with subtitle
            let subtitle = subtitle_text.as_ref().unwrap();
            let mut styled_subtitle = subtitle.copy();
            styled_subtitle.stylize_before(border_style, 0, None);

            let aligned_subtitle = self.align_text(
                &styled_subtitle,
                width.saturating_sub(4),
                self.subtitle_align,
                box_chars.bottom,
                border_style,
            );

            // Left corner + one border char
            result.push(Segment::styled(
                format!("{}{}", box_chars.bottom_left, box_chars.bottom),
                border_style,
            ));

            // Render aligned subtitle
            let subtitle_segments =
                aligned_subtitle.render(console, &options.update_width(width.saturating_sub(4)));
            for seg in subtitle_segments {
                result.push(seg);
            }

            // One border char + right corner
            result.push(Segment::styled(
                format!("{}{}", box_chars.bottom, box_chars.bottom_right),
                border_style,
            ));
        }
        result.push(new_line);

        result
    }

    fn measure(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        let title = self.prepare_title();
        let subtitle = self.prepare_subtitle();
        let (_, pad_right, _, pad_left) = self.padding;
        let horizontal_padding = pad_left + pad_right;

        // Build list of renderables to measure (include both title and subtitle)
        let inner_ref: &dyn Renderable = self.renderable.as_ref();
        let title_ref: Option<&dyn Renderable> = title.as_ref().map(|t| t as &dyn Renderable);
        let subtitle_ref: Option<&dyn Renderable> = subtitle.as_ref().map(|t| t as &dyn Renderable);

        let mut renderables: Vec<&dyn Renderable> = vec![inner_ref];
        if let Some(t) = title_ref {
            renderables.push(t);
        }
        if let Some(s) = subtitle_ref {
            renderables.push(s);
        }

        if let Some(w) = self.width {
            return Measurement::exact(w).with_maximum(options.max_width);
        }

        // Measure the inner content within the space available after borders and padding.
        // Use both min and max so parent layouts (e.g., Table) can shrink panels without
        // thinking they require the full available width.
        let measure_options =
            options.update_width(options.max_width.saturating_sub(horizontal_padding + 2));
        let inner_measurement = measure_renderables(console, &measure_options, &renderables);

        let min_width = inner_measurement.minimum + horizontal_padding + 2;
        let max_width = if self.expand {
            options.max_width
        } else {
            inner_measurement.maximum + horizontal_padding + 2
        };

        Measurement::new(min_width, max_width).with_maximum(options.max_width)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r#box::{ASCII, DOUBLE, HEAVY, SQUARE};
    use crate::cells::cell_len;

    // ==================== Construction tests ====================

    #[test]
    fn test_panel_new() {
        let text = Text::plain("Hello");
        let panel = Panel::new(Box::new(text));
        assert!(panel.expand());
        assert_eq!(panel.box_type(), ROUNDED);
    }

    #[test]
    fn test_panel_fit() {
        let text = Text::plain("Hello");
        let panel = Panel::fit(Box::new(text));
        assert!(!panel.expand());
    }

    #[test]
    fn test_panel_with_title() {
        let text = Text::plain("Hello");
        let panel = Panel::new(Box::new(text)).with_title("Title");
        assert!(panel.title().is_some());
    }

    #[test]
    fn test_panel_with_subtitle() {
        let text = Text::plain("Hello");
        let panel = Panel::new(Box::new(text)).with_subtitle("Subtitle");
        assert!(panel.subtitle().is_some());
    }

    #[test]
    fn test_panel_with_box() {
        let text = Text::plain("Hello");
        let panel = Panel::new(Box::new(text)).with_box(DOUBLE);
        assert_eq!(panel.box_type(), DOUBLE);
    }

    #[test]
    fn test_panel_with_padding() {
        let text = Text::plain("Hello");
        let panel = Panel::new(Box::new(text)).with_padding((1, 2, 3, 4));
        assert_eq!(panel.padding(), (1, 2, 3, 4));
    }

    #[test]
    fn test_panel_with_style() {
        let text = Text::plain("Hello");
        let style = Style::new().with_bold(true);
        let panel = Panel::new(Box::new(text)).with_style(style);
        assert_eq!(panel.style().bold, Some(true));
    }

    #[test]
    fn test_panel_with_border_style() {
        let text = Text::plain("Hello");
        let style = Style::new().with_italic(true);
        let panel = Panel::new(Box::new(text)).with_border_style(style);
        assert_eq!(panel.border_style().italic, Some(true));
    }

    // ==================== Render tests ====================

    #[test]
    fn test_panel_render_basic() {
        let text = Text::plain("Hello");
        let panel = Panel::fit(Box::new(text)).with_padding(0);
        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = panel.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should have top border, content, bottom border
        assert!(output.contains('╭')); // ROUNDED top-left
        assert!(output.contains('╮')); // ROUNDED top-right
        assert!(output.contains("Hello"));
        assert!(output.contains('╰')); // ROUNDED bottom-left
        assert!(output.contains('╯')); // ROUNDED bottom-right
    }

    #[test]
    fn test_panel_render_with_title() {
        let text = Text::plain("Hello");
        let panel = Panel::fit(Box::new(text))
            .with_title("Title")
            .with_padding(0);
        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = panel.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(output.contains("Title"));
    }

    #[test]
    fn test_panel_render_with_subtitle() {
        let text = Text::plain("Hello");
        let panel = Panel::fit(Box::new(text))
            .with_subtitle("Sub")
            .with_padding(0);
        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = panel.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(output.contains("Sub"));
    }

    #[test]
    fn test_panel_render_ascii_box() {
        let text = Text::plain("Hi");
        let panel = Panel::fit(Box::new(text)).with_box(ASCII).with_padding(0);
        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = panel.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(output.contains('+')); // ASCII corners
        assert!(output.contains('-')); // ASCII horizontal
    }

    #[test]
    fn test_panel_render_expand() {
        let text = Text::plain("Hi");
        let panel = Panel::new(Box::new(text)).with_padding(0);
        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = panel.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let lines: Vec<&str> = output.lines().collect();

        // With expand=true, panel should fill the width
        // First line (top border) should be 20 chars
        assert_eq!(cell_len(lines[0]), 20);
    }

    #[test]
    fn test_panel_render_with_padding() {
        let text = Text::plain("X");
        let panel = Panel::fit(Box::new(text)).with_padding((1, 2, 1, 2));
        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = panel.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let lines: Vec<&str> = output.lines().collect();

        // With top/bottom padding of 1, we should have more lines
        // top border + 1 blank + content + 1 blank + bottom border = 5 lines
        assert!(lines.len() >= 5);
    }

    #[test]
    fn test_panel_render_title_alignment_left() {
        let text = Text::plain("X");
        let panel = Panel::fit(Box::new(text))
            .with_title("T")
            .with_title_align(AlignMethod::Left)
            .with_padding(0)
            .with_width(10);
        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = panel.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let first_line = output.lines().next().unwrap();

        // Title should be near the left (after corner + border)
        assert!(first_line.starts_with("╭─"));
    }

    #[test]
    fn test_panel_render_title_alignment_right() {
        let text = Text::plain("X");
        let panel = Panel::fit(Box::new(text))
            .with_title("T")
            .with_title_align(AlignMethod::Right)
            .with_padding(0)
            .with_width(10);
        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = panel.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let first_line = output.lines().next().unwrap();

        // Title should be near the right (before corner)
        assert!(first_line.ends_with("─╮"));
    }

    // ==================== Measure tests ====================

    #[test]
    fn test_panel_measure_basic() {
        let text = Text::plain("Hello");
        let panel = Panel::fit(Box::new(text)).with_padding(0);
        let console = Console::new();
        let options = ConsoleOptions::default();

        let measurement = panel.measure(&console, &options);
        // "Hello" is 5 chars + 2 for borders = 7
        assert_eq!(measurement.minimum, 7);
        assert_eq!(measurement.maximum, 7);
    }

    #[test]
    fn test_panel_measure_with_padding() {
        let text = Text::plain("Hi");
        let panel = Panel::fit(Box::new(text)).with_padding((0, 2, 0, 2));
        let console = Console::new();
        let options = ConsoleOptions::default();

        let measurement = panel.measure(&console, &options);
        // "Hi" is 2 chars + 4 padding + 2 borders = 8
        assert_eq!(measurement.minimum, 8);
    }

    #[test]
    fn test_panel_measure_with_fixed_width() {
        let text = Text::plain("Hello");
        let panel = Panel::new(Box::new(text)).with_width(20);
        let console = Console::new();
        let options = ConsoleOptions::default();

        let measurement = panel.measure(&console, &options);
        assert_eq!(measurement.minimum, 20);
        assert_eq!(measurement.maximum, 20);
    }

    // ==================== Send + Sync tests ====================

    #[test]
    fn test_panel_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Panel>();
        assert_sync::<Panel>();
    }

    // ==================== Debug tests ====================

    #[test]
    fn test_panel_debug() {
        let text = Text::plain("Hello");
        let panel = Panel::new(Box::new(text))
            .with_title("Title")
            .with_box(HEAVY);
        let debug_str = format!("{:?}", panel);
        assert!(debug_str.contains("Panel"));
        assert!(debug_str.contains("title"));
    }

    // ==================== Edge case tests ====================

    #[test]
    fn test_panel_empty_content() {
        let text = Text::plain("");
        let panel = Panel::fit(Box::new(text)).with_padding(0);
        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = panel.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should still have borders
        assert!(output.contains('╭'));
        assert!(output.contains('╰'));
    }

    #[test]
    fn test_panel_multiline_content() {
        let text = Text::plain("Line1\nLine2");
        let panel = Panel::fit(Box::new(text)).with_padding(0);
        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = panel.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let lines: Vec<&str> = output.lines().collect();

        // Top border + 2 content lines + bottom border = 4 lines
        assert!(lines.len() >= 4);
    }

    #[test]
    fn test_panel_cjk_content() {
        let text = Text::plain("你好");
        let panel = Panel::fit(Box::new(text)).with_padding(0);
        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = panel.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(output.contains("你好"));
    }

    #[test]
    fn test_panel_various_boxes() {
        let boxes = [ROUNDED, SQUARE, HEAVY, DOUBLE, ASCII];

        for box_type in boxes {
            let text = Text::plain("Test");
            let panel = Panel::fit(Box::new(text))
                .with_box(box_type)
                .with_padding(0);
            let console = Console::with_options(ConsoleOptions {
                max_width: 20,
                ..Default::default()
            });
            let options = console.options().clone();

            let segments = panel.render(&console, &options);
            assert!(!segments.is_empty());
        }
    }
}
