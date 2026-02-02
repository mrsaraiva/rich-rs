//! Highlighter: apply styling to text based on patterns.
//!
//! Highlighters analyze text and apply styles based on regex patterns.
//! Each named capture group in a pattern corresponds to a style name.

use regex::Regex;

use crate::text::Text;
use crate::theme::Theme;

/// Build a byte-offset to char-offset lookup table for O(1) conversion.
///
/// Returns a Vec where `map[byte_idx]` gives the character index at that byte position.
/// Only valid for byte indices that fall on UTF-8 character boundaries.
fn byte_to_char_map(s: &str) -> Vec<usize> {
    let mut map = vec![0; s.len() + 1];
    let mut char_idx = 0;
    for (byte_idx, _) in s.char_indices() {
        map[byte_idx] = char_idx;
        char_idx += 1;
    }
    map[s.len()] = char_idx;
    map
}

/// Trait for highlighters that apply styling to text based on patterns.
///
/// Highlighters modify text in place, applying styles to matched regions.
pub trait Highlighter: Send + Sync {
    /// Apply highlighting in place to text.
    fn highlight(&self, text: &mut Text);
}

/// A highlighter that does nothing. Used to disable highlighting.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullHighlighter;

impl Highlighter for NullHighlighter {
    fn highlight(&self, _text: &mut Text) {
        // No-op
    }
}

/// Base for regex-based highlighters.
///
/// This highlighter applies styles based on named capture groups in regex patterns.
/// The style name is constructed by combining `base_style` with the capture group name.
///
/// For example, with `base_style = "repr."` and a pattern containing `(?P<number>...)`,
/// matched text will receive the style `"repr.number"`.
#[derive(Debug, Clone)]
pub struct RegexHighlighter {
    /// Compiled regex patterns with named capture groups for styling.
    pub highlights: Vec<Regex>,
    /// Style prefix for named groups (e.g., "repr." -> "repr.number").
    pub base_style: String,
    /// Theme for style lookups.
    theme: Theme,
}

impl RegexHighlighter {
    /// Create a new regex highlighter from patterns.
    ///
    /// # Arguments
    ///
    /// * `patterns` - Regex patterns with named capture groups
    /// * `base_style` - Style prefix to prepend to capture group names
    ///
    /// # Panics
    ///
    /// Panics if any pattern fails to compile.
    pub fn new(patterns: &[&str], base_style: &str) -> Self {
        let highlights = patterns
            .iter()
            .map(|p| Regex::new(p).expect("Invalid regex pattern"))
            .collect();

        RegexHighlighter {
            highlights,
            base_style: base_style.to_string(),
            theme: Theme::new(),
        }
    }

    /// Try to create a new regex highlighter from patterns, returning an error if compilation fails.
    ///
    /// # Arguments
    ///
    /// * `patterns` - Regex patterns with named capture groups
    /// * `base_style` - Style prefix to prepend to capture group names
    ///
    /// # Returns
    ///
    /// `Ok(RegexHighlighter)` if all patterns compile, `Err` with the first error otherwise.
    pub fn try_new(patterns: &[&str], base_style: &str) -> Result<Self, regex::Error> {
        let highlights: Result<Vec<_>, _> = patterns.iter().map(|p| Regex::new(p)).collect();

        Ok(RegexHighlighter {
            highlights: highlights?,
            base_style: base_style.to_string(),
            theme: Theme::new(),
        })
    }

    /// Set the theme for this highlighter.
    ///
    /// The theme determines the colors and styles applied to matched patterns.
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

impl Highlighter for RegexHighlighter {
    fn highlight(&self, text: &mut Text) {
        let plain = text.plain_text().to_string();

        // Precompute byte-to-char mapping for O(n) total complexity instead of O(n²)
        let byte_to_char = byte_to_char_map(&plain);

        for regex in &self.highlights {
            for captures in regex.captures_iter(&plain) {
                // Iterate over named capture groups
                for name in regex.capture_names().flatten() {
                    if let Some(m) = captures.name(name) {
                        // O(1) conversion using precomputed map
                        let start_char = byte_to_char[m.start()];
                        let end_char = byte_to_char[m.end()];

                        // Look up style from theme by base_style + capture group name
                        let style_name = format!("{}{}", self.base_style, name);
                        if let Some(style) = self.theme.get_style(&style_name) {
                            text.stylize(start_char, end_char, style);
                        }
                    }
                }
            }
        }
    }
}

/// Combine multiple regex patterns into a single pattern with OR.
///
/// # Example
///
/// ```
/// use rich_rs::highlighter::combine_regex;
///
/// let combined = combine_regex(&["foo", "bar", "baz"]);
/// assert_eq!(combined, "foo|bar|baz");
/// ```
pub fn combine_regex(patterns: &[&str]) -> String {
    patterns.join("|")
}

/// Create a highlighter for `__repr__` output.
///
/// Highlights numbers, strings, booleans, None, IP addresses, UUIDs, paths, URLs, etc.
///
/// Note: Some patterns from Python's ReprHighlighter have been simplified because
/// Rust's regex crate doesn't support look-around assertions or conditional patterns.
pub fn repr_highlighter() -> RegexHighlighter {
    repr_highlighter_with_theme(Theme::new())
}

/// Create a highlighter for `__repr__` output with a custom theme.
///
/// # Arguments
///
/// * `theme` - The theme to use for styling
pub fn repr_highlighter_with_theme(theme: Theme) -> RegexHighlighter {
    // Patterns adapted from Python's ReprHighlighter
    // Note: Look-around assertions removed for Rust regex compatibility
    let patterns = [
        r"(?P<tag_start><)(?P<tag_name>[-\w.:|]*)(?P<tag_contents>[\w\W]*?)(?P<tag_end>>)",
        r#"(?P<attrib_name>[\w_]{1,50})=(?P<attrib_value>"?[\w_]+"?)?"#,
        // Character class with ] first to include it literally
        r"(?P<brace>[\[\]{}()])",
        // Combined pattern for various literals
        &combine_regex(&[
            r"(?P<ipv4>[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3})",
            r"(?P<ipv6>([A-Fa-f0-9]{1,4}::?){1,7}[A-Fa-f0-9]{1,4})",
            r"(?P<eui64>(?:[0-9A-Fa-f]{1,2}-){7}[0-9A-Fa-f]{1,2}|(?:[0-9A-Fa-f]{1,2}:){7}[0-9A-Fa-f]{1,2}|(?:[0-9A-Fa-f]{4}\.){3}[0-9A-Fa-f]{4})",
            r"(?P<eui48>(?:[0-9A-Fa-f]{1,2}-){5}[0-9A-Fa-f]{1,2}|(?:[0-9A-Fa-f]{1,2}:){5}[0-9A-Fa-f]{1,2}|(?:[0-9A-Fa-f]{4}\.){2}[0-9A-Fa-f]{4})",
            r"(?P<uuid>[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{12})",
            r"(?P<call>[\w.]*?)\(",
            r"\b(?P<bool_true>True)\b|\b(?P<bool_false>False)\b|\b(?P<none>None)\b",
            r"(?P<ellipsis>\.\.\.)",
            // Simplified: removed look-behind assertion
            r"(?P<number_complex>(?:\-?[0-9]+\.?[0-9]*(?:e[-+]?\d+?)?)(?:[-+](?:[0-9]+\.?[0-9]*(?:e[-+]?\d+)?))?j)",
            r"(?P<number>\b\-?[0-9]+\.?[0-9]*(e[-+]?\d+?)?\b|0x[0-9a-fA-F]*)",
            r"(?P<path>\B(/[-\w._+]+)*\/)(?P<filename>[-\w._+]*)?",
            // Simplified: removed look-behind assertions for string matching
            r#"(?P<str>b?'''.*?'''|b?'[^']*'|b?""".*?"""|b?"[^"]*")"#,
            r"(?P<url>(file|https|http|ws|wss)://[-0-9a-zA-Z$_+!`(),.?/;:&=%#~@]*)",
        ]),
    ];

    // Build with owned strings to avoid lifetime issues
    let owned_patterns: Vec<String> = patterns.iter().map(|s| s.to_string()).collect();
    let pattern_refs: Vec<&str> = owned_patterns.iter().map(|s| s.as_str()).collect();

    RegexHighlighter::new(&pattern_refs, "repr.").with_theme(theme)
}

/// Create a highlighter for JSON.
///
/// Highlights braces, strings, booleans, null, and numbers.
///
/// Note: Simplified from Python version due to Rust regex limitations
/// (no look-around assertions).
pub fn json_highlighter() -> RegexHighlighter {
    // Simplified string pattern without look-behind
    let json_str = r#"(?P<str>"[^"\\]*(?:\\.[^"\\]*)*")"#;

    let pattern = combine_regex(&[
        r"(?P<brace>[\[\]{}()])",
        r"\b(?P<bool_true>true)\b|\b(?P<bool_false>false)\b|\b(?P<null>null)\b",
        r"(?P<number>\b\-?[0-9]+\.?[0-9]*(e[\-\+]?\d+?)?\b|0x[0-9a-fA-F]*)",
        json_str,
    ]);

    RegexHighlighter::new(&[&pattern], "json.")
}

/// Create a highlighter for file system paths.
///
/// Highlights path components: directory separators, directories, and filenames.
pub fn path_highlighter() -> RegexHighlighter {
    let patterns = [
        // Highlight path components (Unix-style)
        r"(?P<path>(?:/[-\w._+]+)+/)(?P<filename>[-\w._+]*)",
        // Windows-style paths
        r"(?P<path>(?:[A-Za-z]:)?(?:\\[-\w._+]+)+\\)(?P<filename>[-\w._+]*)",
    ];
    RegexHighlighter::new(&patterns, "traceback.")
}

/// Create a highlighter for ISO8601 date/time strings.
///
/// Highlights various ISO8601 date and time formats.
///
/// # Limitations
///
/// - Negative years (e.g., `-0001-01-01`) are not matched due to word boundary constraints
/// - Patterns simplified from Python version; conditional patterns like `(?(hyphen)-)`
///   are not supported in Rust's regex crate
pub fn iso8601_highlighter() -> RegexHighlighter {
    // Note: Anchors removed to allow inline matching within larger text.
    // Use word boundaries (\b) where appropriate for standalone matching.
    let patterns = [
        // Calendar month (e.g. 2008-08)
        r"\b(?P<year>[0-9]{4})-(?P<month>1[0-2]|0[1-9])\b",
        // Calendar date w/o hyphens (e.g. 20080830)
        r"\b(?P<date>(?P<year>[0-9]{4})(?P<month>1[0-2]|0[1-9])(?P<day>3[01]|0[1-9]|[12][0-9]))\b",
        // Ordinal date (e.g. 2008-243)
        r"\b(?P<date>(?P<year>[0-9]{4})-?(?P<day>36[0-6]|3[0-5][0-9]|[12][0-9]{2}|0[1-9][0-9]|00[1-9]))\b",
        // Week of the year (e.g., 2008-W35)
        r"\b(?P<date>(?P<year>[0-9]{4})-?W(?P<week>5[0-3]|[1-4][0-9]|0[1-9]))\b",
        // Week date (e.g., 2008-W35-6)
        r"\b(?P<date>(?P<year>[0-9]{4})-?W(?P<week>5[0-3]|[1-4][0-9]|0[1-9])-?(?P<day>[1-7]))\b",
        // Hours and minutes (e.g., 17:21)
        r"\b(?P<time>(?P<hour>2[0-3]|[01][0-9]):?(?P<minute>[0-5][0-9]))\b",
        // Hours, minutes, and seconds w/o colons (e.g., 172159)
        r"\b(?P<time>(?P<hour>2[0-3]|[01][0-9])(?P<minute>[0-5][0-9])(?P<second>[0-5][0-9]))\b",
        // Time zone designator (e.g., Z, +07 or +07:00)
        r"(?P<timezone>Z|[+-](?:2[0-3]|[01][0-9])(?::?(?:[0-5][0-9]))?)\b",
        // Hours, minutes, seconds with time zone
        r"\b(?P<time>(?P<hour>2[0-3]|[01][0-9])(?P<minute>[0-5][0-9])(?P<second>[0-5][0-9]))(?P<timezone>Z|[+-](?:2[0-3]|[01][0-9])(?::?(?:[0-5][0-9]))?)",
        // Calendar date with hyphens and time with colons (e.g., 2008-08-30 17:21:59)
        r"\b(?P<date>(?P<year>[0-9]{4})-(?P<month>1[0-2]|0[1-9])-(?P<day>3[01]|0[1-9]|[12][0-9]))[T ](?P<time>(?P<hour>2[0-3]|[01][0-9]):(?P<minute>[0-5][0-9]):(?P<second>[0-5][0-9]))",
        // Calendar date without hyphens and time without colons (e.g., 20080830 172159)
        r"\b(?P<date>(?P<year>[0-9]{4})(?P<month>1[0-2]|0[1-9])(?P<day>3[01]|0[1-9]|[12][0-9]))[T ](?P<time>(?P<hour>2[0-3]|[01][0-9])(?P<minute>[0-5][0-9])(?P<second>[0-5][0-9]))",
        // XML Schema date
        r"\b(?P<date>(?P<year>-?(?:[1-9][0-9]*)?[0-9]{4})-(?P<month>1[0-2]|0[1-9])-(?P<day>3[01]|0[1-9]|[12][0-9]))(?P<timezone>Z|[+-](?:2[0-3]|[01][0-9]):[0-5][0-9])?",
        // XML Schema time
        r"\b(?P<time>(?P<hour>2[0-3]|[01][0-9]):(?P<minute>[0-5][0-9]):(?P<second>[0-5][0-9])(?P<frac>\.[0-9]+)?)(?P<timezone>Z|[+-](?:2[0-3]|[01][0-9]):[0-5][0-9])?",
        // XML Schema dateTime
        r"\b(?P<date>(?P<year>-?(?:[1-9][0-9]*)?[0-9]{4})-(?P<month>1[0-2]|0[1-9])-(?P<day>3[01]|0[1-9]|[12][0-9]))T(?P<time>(?P<hour>2[0-3]|[01][0-9]):(?P<minute>[0-5][0-9]):(?P<second>[0-5][0-9])(?P<ms>\.[0-9]+)?)(?P<timezone>Z|[+-](?:2[0-3]|[01][0-9]):[0-5][0-9])?",
    ];

    RegexHighlighter::new(&patterns, "iso8601.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_highlighter_no_modification() {
        let highlighter = NullHighlighter;
        let mut text = Text::plain("hello world");
        let original_text = text.plain_text().to_string();
        let original_spans_len = text.spans().len();

        highlighter.highlight(&mut text);

        assert_eq!(text.plain_text(), original_text);
        assert_eq!(text.spans().len(), original_spans_len);
    }

    #[test]
    fn test_combine_regex() {
        assert_eq!(combine_regex(&["a", "b", "c"]), "a|b|c");
        assert_eq!(combine_regex(&["foo"]), "foo");
        assert_eq!(combine_regex(&[]), "");
    }

    #[test]
    fn test_combine_regex_with_groups() {
        let combined = combine_regex(&[r"(?P<num>\d+)", r"(?P<word>\w+)"]);
        assert_eq!(combined, r"(?P<num>\d+)|(?P<word>\w+)");
    }

    #[test]
    fn test_regex_highlighter_new_compiles() {
        // Should not panic
        let _highlighter = RegexHighlighter::new(&[r"\d+", r"\w+"], "test.");
    }

    #[test]
    fn test_regex_highlighter_try_new_valid() {
        let result = RegexHighlighter::try_new(&[r"\d+", r"\w+"], "test.");
        assert!(result.is_ok());
    }

    #[test]
    fn test_regex_highlighter_try_new_invalid() {
        let result = RegexHighlighter::try_new(&[r"[invalid"], "test.");
        assert!(result.is_err());
    }

    #[test]
    #[should_panic(expected = "Invalid regex pattern")]
    fn test_regex_highlighter_new_invalid_panics() {
        let _highlighter = RegexHighlighter::new(&[r"[invalid"], "test.");
    }

    #[test]
    fn test_repr_highlighter_creates() {
        // Should not panic
        let highlighter = repr_highlighter();
        assert!(!highlighter.highlights.is_empty());
        assert_eq!(highlighter.base_style, "repr.");
    }

    #[test]
    fn test_json_highlighter_creates() {
        // Should not panic
        let highlighter = json_highlighter();
        assert!(!highlighter.highlights.is_empty());
        assert_eq!(highlighter.base_style, "json.");
    }

    #[test]
    fn test_iso8601_highlighter_creates() {
        // Should not panic
        let highlighter = iso8601_highlighter();
        assert!(!highlighter.highlights.is_empty());
        assert_eq!(highlighter.base_style, "iso8601.");
    }

    #[test]
    fn test_path_highlighter_creates() {
        // Should not panic
        let highlighter = path_highlighter();
        assert!(!highlighter.highlights.is_empty());
        assert_eq!(highlighter.base_style, "traceback.");
    }

    #[test]
    fn test_regex_highlighter_basic_highlight() {
        // Use "repr." base style which has "repr.number" defined in the theme
        let highlighter = RegexHighlighter::new(&[r"(?P<number>\d+)"], "repr.");
        let mut text = Text::plain("value is 42");

        highlighter.highlight(&mut text);

        // Should have added a span for the number (repr.number exists in theme)
        assert!(!text.spans().is_empty());
        // The span should cover "42" (characters 9-11)
        let span = &text.spans()[0];
        assert_eq!(span.start, 9);
        assert_eq!(span.end, 11);
    }

    #[test]
    fn test_highlighter_trait_object() {
        // Verify the trait is object-safe
        let highlighters: Vec<Box<dyn Highlighter>> = vec![
            Box::new(NullHighlighter),
            Box::new(RegexHighlighter::new(&[r"\d+"], "")),
        ];

        let mut text = Text::plain("test 123");
        for h in &highlighters {
            h.highlight(&mut text);
        }
    }
}
