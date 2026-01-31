//! Rule: a horizontal line renderable.
//!
//! A rule is a horizontal line that can optionally have a title.
//! The title can be aligned left, center, or right.
//!
//! # Example
//!
//! ```
//! use rich_rs::rule::{Rule, AlignMethod};
//!
//! // Simple rule without title
//! let rule = Rule::new();
//!
//! // Rule with centered title
//! let rule = Rule::new().with_title("Section Header");
//!
//! // Rule with left-aligned title
//! let rule = Rule::new()
//!     .with_title("Left Title")
//!     .with_align(AlignMethod::Left);
//! ```

use crate::Renderable;
use crate::cells::{cell_len, set_cell_size};
use crate::console::{Console, ConsoleOptions, OverflowMethod};
use crate::measure::Measurement;
use crate::segment::Segments;
use crate::style::Style;
use crate::text::Text;

// ============================================================================
// AlignMethod
// ============================================================================

/// Text alignment method for Rule titles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignMethod {
    /// Left-aligned title.
    Left,
    /// Center-aligned title (default).
    #[default]
    Center,
    /// Right-aligned title.
    Right,
}

impl AlignMethod {
    /// Parse an alignment method from a string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "left" => Some(AlignMethod::Left),
            "center" => Some(AlignMethod::Center),
            "right" => Some(AlignMethod::Right),
            _ => None,
        }
    }
}

// ============================================================================
// Rule
// ============================================================================

/// A horizontal rule (line) that can optionally have a title.
///
/// # Example
///
/// ```
/// use rich_rs::rule::{Rule, AlignMethod};
/// use rich_rs::Style;
///
/// // Simple horizontal line
/// let rule = Rule::new();
///
/// // Rule with a centered title
/// let titled_rule = Rule::new().with_title("My Section");
///
/// // Customized rule
/// let custom_rule = Rule::new()
///     .with_title("Header")
///     .with_characters("=")
///     .with_align(AlignMethod::Left)
///     .with_style(Style::new().with_bold(true));
/// ```
#[derive(Debug, Clone)]
pub struct Rule {
    /// Optional title text.
    title: Option<Text>,
    /// Characters used to draw the line (default: "─").
    characters: String,
    /// Style for the rule line.
    style: Style,
    /// String to append at the end (default: "\n").
    end: String,
    /// Title alignment (default: Center).
    align: AlignMethod,
}

impl Default for Rule {
    fn default() -> Self {
        Rule::new()
    }
}

impl Rule {
    /// Create a new rule with default settings.
    ///
    /// Default settings:
    /// - No title
    /// - "─" (box drawing horizontal) as the line character
    /// - Default style (from theme "rule.line")
    /// - Newline at the end
    /// - Center alignment
    pub fn new() -> Self {
        Rule {
            title: None,
            characters: "─".to_string(),
            style: Style::new(),
            end: "\n".to_string(),
            align: AlignMethod::Center,
        }
    }

    /// Set the title from a string.
    ///
    /// The title will be rendered in the middle of the rule line.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(Text::plain(title.into()));
        self
    }

    /// Set the title from a Text object.
    ///
    /// This allows for styled titles.
    pub fn with_title_text(mut self, title: Text) -> Self {
        self.title = Some(title);
        self
    }

    /// Set the characters used to draw the line.
    ///
    /// # Panics
    ///
    /// Panics if the characters have a cell width less than 1.
    pub fn with_characters(mut self, characters: impl Into<String>) -> Self {
        let chars = characters.into();
        assert!(
            cell_len(&chars) >= 1,
            "'characters' argument must have a cell width of at least 1"
        );
        self.characters = chars;
        self
    }

    /// Set the style for the rule line.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the end string (default is "\n").
    pub fn with_end(mut self, end: impl Into<String>) -> Self {
        self.end = end.into();
        self
    }

    /// Set the title alignment.
    pub fn with_align(mut self, align: AlignMethod) -> Self {
        self.align = align;
        self
    }

    /// Generate a line of characters without a title.
    fn rule_line(&self, characters: &str, chars_len: usize, width: usize) -> Text {
        // Create enough characters to fill the width, then truncate
        let repeat_count = (width / chars_len) + 1;
        let line_chars = characters.repeat(repeat_count);
        let rule_text = Text::styled(line_chars, self.style);

        // Truncate to exact width (using character count, not cell width)
        let chars: Vec<char> = rule_text.plain_text().chars().collect();
        let mut current_width = 0;
        let mut char_count = 0;
        for c in &chars {
            let cw = crate::cells::char_width(*c);
            if current_width + cw > width {
                break;
            }
            current_width += cw;
            char_count += 1;
        }

        // Rebuild the text with the truncated content
        let truncated: String = chars[..char_count].iter().collect();
        let mut result = Text::styled(truncated, self.style);

        // Pad to exact width if needed
        if current_width < width {
            let padding = " ".repeat(width - current_width);
            result.append(&padding, Some(self.style));
        }

        result
    }
}

impl Renderable for Rule {
    fn render(&self, _console: &Console, options: &ConsoleOptions) -> Segments {
        let width = options.max_width;

        // Handle ASCII-only mode
        let characters = if options.ascii_only() && !self.characters.is_ascii() {
            "-".to_string()
        } else {
            self.characters.clone()
        };

        let chars_len = cell_len(&characters);

        // If no title, just render the line
        if self.title.is_none() {
            let mut rule_text = self.rule_line(&characters, chars_len, width);
            // Apply the end string
            if !self.end.is_empty() {
                rule_text.append(&self.end, None);
            }
            return rule_text.render(_console, options);
        }

        // We have a title - need to build the rule with title
        let title = self.title.as_ref().unwrap();

        // Prepare title text: replace newlines with spaces and expand tabs
        // We preserve the base style from the original title
        let plain = title.plain_text().replace('\n', " ");
        let mut title_text = if let Some(style) = title.base_style() {
            Text::styled(&plain, style)
        } else {
            Text::plain(&plain)
        };

        // Copy over spans from original title
        // Since we just replaced newlines with spaces, character offsets are preserved
        for span in title.spans() {
            title_text.stylize(span.start, span.end, span.style);
        }

        // Expand tabs
        title_text = title_text.expand_tabs(8);

        // Calculate required space for title
        // Center alignment needs 2 chars on each side, left/right needs 2 chars on one side
        let required_space = if self.align == AlignMethod::Center {
            4
        } else {
            2
        };

        let truncate_width = width.saturating_sub(required_space);
        if truncate_width == 0 {
            // No room for title, just render the line
            let mut rule_text = self.rule_line(&characters, chars_len, width);
            if !self.end.is_empty() {
                rule_text.append(&self.end, None);
            }
            return rule_text.render(_console, options);
        }

        // Truncate title if needed
        let title_text = title_text.truncate(truncate_width, OverflowMethod::Ellipsis, false);

        // Build the rule text based on alignment
        let mut rule_text = Text::new();

        match self.align {
            AlignMethod::Center => {
                // ───── Title ─────
                let title_cell_len = cell_len(title_text.plain_text());
                let side_width = (width.saturating_sub(title_cell_len)) / 2;

                // Left side: characters filling side_width - 1 (leave space for " ")
                let left_chars = characters.repeat((side_width / chars_len) + 1);
                let left_truncated = set_cell_size(&left_chars, side_width.saturating_sub(1));
                rule_text.append(&left_truncated, Some(self.style));
                rule_text.append(" ", Some(self.style));

                // Title
                rule_text.append_text(&title_text);

                // Right side: fill remaining space
                let right_length =
                    width.saturating_sub(cell_len(&left_truncated) + 1 + title_cell_len);
                rule_text.append(" ", Some(self.style));
                let right_chars = characters.repeat((right_length / chars_len) + 1);
                let right_truncated =
                    set_cell_size(&right_chars, right_length.saturating_sub(1).max(0));
                rule_text.append(&right_truncated, Some(self.style));
            }
            AlignMethod::Left => {
                // Title ─────────────
                rule_text.append_text(&title_text);
                rule_text.append(" ", Some(self.style)); // Separator uses rule style

                let remaining = width.saturating_sub(rule_text.cell_len());
                let fill_chars = characters.repeat((remaining / chars_len) + 1);
                let fill_truncated = set_cell_size(&fill_chars, remaining);
                rule_text.append(&fill_truncated, Some(self.style));
            }
            AlignMethod::Right => {
                // ───────────── Title
                let title_cell_len = cell_len(title_text.plain_text());
                let fill_length = width.saturating_sub(title_cell_len + 1);
                let fill_chars = characters.repeat((fill_length / chars_len) + 1);
                let fill_truncated = set_cell_size(&fill_chars, fill_length);
                rule_text.append(&fill_truncated, Some(self.style));
                rule_text.append(" ", Some(self.style)); // Separator uses rule style
                rule_text.append_text(&title_text);
            }
        }

        // Ensure exact width
        let final_plain = set_cell_size(rule_text.plain_text(), width);
        let mut final_text = Text::plain(&final_plain);

        // Re-apply styles from rule_text
        for span in rule_text.spans() {
            // Clamp span to new text length
            let new_len = final_text.len();
            if span.start < new_len {
                final_text.stylize(span.start, span.end.min(new_len), span.style);
            }
        }

        // Apply base style if present
        if let Some(base) = rule_text.base_style() {
            final_text.set_base_style(Some(base));
        }

        // Append end string
        if !self.end.is_empty() {
            final_text.append(&self.end, None);
        }

        final_text.render(_console, options)
    }

    fn measure(&self, _console: &Console, _options: &ConsoleOptions) -> Measurement {
        // Rule always uses exactly 1 cell minimum and maximum width
        // (it expands to fill available space)
        Measurement::new(1, 1)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== AlignMethod tests ====================

    #[test]
    fn test_align_method_parse() {
        assert_eq!(AlignMethod::parse("left"), Some(AlignMethod::Left));
        assert_eq!(AlignMethod::parse("LEFT"), Some(AlignMethod::Left));
        assert_eq!(AlignMethod::parse("center"), Some(AlignMethod::Center));
        assert_eq!(AlignMethod::parse("CENTER"), Some(AlignMethod::Center));
        assert_eq!(AlignMethod::parse("right"), Some(AlignMethod::Right));
        assert_eq!(AlignMethod::parse("RIGHT"), Some(AlignMethod::Right));
        assert_eq!(AlignMethod::parse("invalid"), None);
    }

    #[test]
    fn test_align_method_default() {
        assert_eq!(AlignMethod::default(), AlignMethod::Center);
    }

    // ==================== Rule construction tests ====================

    #[test]
    fn test_rule_new() {
        let rule = Rule::new();
        assert!(rule.title.is_none());
        assert_eq!(rule.characters, "─");
        assert_eq!(rule.end, "\n");
        assert_eq!(rule.align, AlignMethod::Center);
    }

    #[test]
    fn test_rule_with_title() {
        let rule = Rule::new().with_title("Test");
        assert!(rule.title.is_some());
        assert_eq!(rule.title.as_ref().unwrap().plain_text(), "Test");
    }

    #[test]
    fn test_rule_with_title_text() {
        let text = Text::styled("Styled Title", Style::new().with_bold(true));
        let rule = Rule::new().with_title_text(text);
        assert!(rule.title.is_some());
        assert_eq!(rule.title.as_ref().unwrap().plain_text(), "Styled Title");
    }

    #[test]
    fn test_rule_with_characters() {
        let rule = Rule::new().with_characters("=");
        assert_eq!(rule.characters, "=");
    }

    #[test]
    #[should_panic(expected = "'characters' argument must have a cell width of at least 1")]
    fn test_rule_with_empty_characters() {
        Rule::new().with_characters("");
    }

    #[test]
    fn test_rule_with_style() {
        let style = Style::new().with_bold(true);
        let rule = Rule::new().with_style(style);
        assert_eq!(rule.style.bold, Some(true));
    }

    #[test]
    fn test_rule_with_end() {
        let rule = Rule::new().with_end("");
        assert_eq!(rule.end, "");
    }

    #[test]
    fn test_rule_with_align() {
        let rule = Rule::new().with_align(AlignMethod::Left);
        assert_eq!(rule.align, AlignMethod::Left);
    }

    // ==================== Rule render tests ====================

    #[test]
    fn test_rule_render_no_title() {
        let rule = Rule::new().with_end("");
        let console = Console::new();
        let options = ConsoleOptions {
            max_width: 20,
            ..Default::default()
        };

        let segments = rule.render(&console, &options);
        let text: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should be exactly 20 cells of "─"
        assert_eq!(cell_len(&text), 20);
        assert!(text.contains("─"));
    }

    #[test]
    fn test_rule_render_with_title_center() {
        let rule = Rule::new().with_title("Test").with_end("");
        let console = Console::new();
        let options = ConsoleOptions {
            max_width: 20,
            ..Default::default()
        };

        let segments = rule.render(&console, &options);
        let text: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should contain the title
        assert!(text.contains("Test"));
        // Should be exactly 20 cells
        assert_eq!(cell_len(&text), 20);
    }

    #[test]
    fn test_rule_render_with_title_left() {
        let rule = Rule::new()
            .with_title("Left")
            .with_align(AlignMethod::Left)
            .with_end("");
        let console = Console::new();
        let options = ConsoleOptions {
            max_width: 20,
            ..Default::default()
        };

        let segments = rule.render(&console, &options);
        let text: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Title should be at the left
        assert!(text.starts_with("Left"));
        assert_eq!(cell_len(&text), 20);
    }

    #[test]
    fn test_rule_render_with_title_right() {
        let rule = Rule::new()
            .with_title("Right")
            .with_align(AlignMethod::Right)
            .with_end("");
        let console = Console::new();
        let options = ConsoleOptions {
            max_width: 20,
            ..Default::default()
        };

        let segments = rule.render(&console, &options);
        let text: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Title should be at the right
        assert!(text.trim_end().ends_with("Right"));
        assert_eq!(cell_len(&text), 20);
    }

    #[test]
    fn test_rule_ascii_only() {
        let rule = Rule::new().with_end("");
        let console = Console::new();
        let options = ConsoleOptions {
            max_width: 10,
            encoding: "ascii".to_string(),
            ..Default::default()
        };

        let segments = rule.render(&console, &options);
        let text: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should use "-" instead of "─" in ASCII mode
        assert!(text.chars().all(|c| c == '-' || c == ' '));
        assert!(!text.contains("─"));
    }

    #[test]
    fn test_rule_long_title_truncation() {
        let rule = Rule::new()
            .with_title("This is a very long title that needs truncation")
            .with_end("");
        let console = Console::new();
        let options = ConsoleOptions {
            max_width: 20,
            ..Default::default()
        };

        let segments = rule.render(&console, &options);
        let text: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should truncate with ellipsis
        assert!(text.contains("…") || cell_len(&text) == 20);
        assert_eq!(cell_len(&text), 20);
    }

    #[test]
    fn test_rule_multi_char_pattern() {
        let rule = Rule::new().with_characters("+-").with_end("");
        let console = Console::new();
        let options = ConsoleOptions {
            max_width: 10,
            ..Default::default()
        };

        let segments = rule.render(&console, &options);
        let text: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should contain the pattern
        assert!(text.contains('+') || text.contains('-'));
        assert_eq!(cell_len(&text), 10);
    }

    #[test]
    fn test_rule_measure() {
        let rule = Rule::new().with_title("Test");
        let console = Console::new();
        let options = ConsoleOptions::default();

        let measurement = rule.measure(&console, &options);
        assert_eq!(measurement.minimum, 1);
        assert_eq!(measurement.maximum, 1);
    }

    #[test]
    fn test_rule_with_end_newline() {
        let rule = Rule::new();
        let console = Console::new();
        let options = ConsoleOptions {
            max_width: 10,
            ..Default::default()
        };

        let segments = rule.render(&console, &options);
        let text: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should end with newline
        assert!(text.ends_with('\n'));
    }

    // ==================== Edge case tests ====================

    #[test]
    fn test_rule_very_narrow_width() {
        let rule = Rule::new().with_title("Test").with_end("");
        let console = Console::new();
        let options = ConsoleOptions {
            max_width: 4,
            ..Default::default()
        };

        let segments = rule.render(&console, &options);
        let text: String = segments.iter().map(|s| s.text.to_string()).collect();

        // With only 4 cells, no room for title (requires 4 for padding)
        // Should render just the line
        assert_eq!(cell_len(&text), 4);
    }

    #[test]
    fn test_rule_unicode_characters() {
        let rule = Rule::new().with_characters("═").with_end("");
        let console = Console::new();
        let options = ConsoleOptions {
            max_width: 10,
            ..Default::default()
        };

        let segments = rule.render(&console, &options);
        let text: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(text.contains("═"));
        assert_eq!(cell_len(&text), 10);
    }

    #[test]
    fn test_rule_cjk_title() {
        let rule = Rule::new().with_title("你好").with_end("");
        let console = Console::new();
        let options = ConsoleOptions {
            max_width: 20,
            ..Default::default()
        };

        let segments = rule.render(&console, &options);
        let text: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(text.contains("你好"));
        assert_eq!(cell_len(&text), 20);
    }
}
