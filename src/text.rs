//! Text: rich text with styled spans.
//!
//! Text is the primary way to work with styled content in Rich.
//! It stores plain text with a list of spans that define styled regions.

use regex::Regex;

use crate::Renderable;
use crate::cells::cell_len;
use crate::console::{Console, ConsoleOptions};
use crate::error::Result;
use crate::measure::Measurement;
use crate::segment::{Segment, Segments};
use crate::style::{Style, StyleMeta};

/// A span of styled text within a Text object.
///
/// Spans define a region of text (by character index) and the style to apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// Start index (character offset, inclusive).
    pub start: usize,
    /// End index (character offset, exclusive).
    pub end: usize,
    /// Style to apply.
    pub style: Style,
    /// Optional style metadata (hyperlinks, Textual handlers, etc.).
    pub meta: Option<StyleMeta>,
}

impl Span {
    /// Create a new span.
    pub fn new(start: usize, end: usize, style: Style) -> Self {
        Span {
            start,
            end,
            style,
            meta: None,
        }
    }

    /// Create a new span with optional metadata.
    pub fn new_with_meta(start: usize, end: usize, style: Style, meta: Option<StyleMeta>) -> Self {
        Span {
            start,
            end,
            style,
            meta: meta.and_then(|m| if m.is_empty() { None } else { Some(m) }),
        }
    }

    /// Check if the span has any content (end > start).
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }

    /// Split a span into two at a given offset.
    ///
    /// If the offset is outside the span, returns `(self, None)`.
    ///
    /// # Arguments
    ///
    /// * `offset` - The character offset at which to split.
    ///
    /// # Returns
    ///
    /// A tuple of (first_span, optional_second_span).
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::text::Span;
    /// use rich_rs::Style;
    ///
    /// let span = Span::new(0, 10, Style::new().with_bold(true));
    /// let (first, second) = span.split(5);
    /// assert_eq!(first.start, 0);
    /// assert_eq!(first.end, 5);
    /// assert!(second.is_some());
    /// let second = second.unwrap();
    /// assert_eq!(second.start, 5);
    /// assert_eq!(second.end, 10);
    /// ```
    pub fn split(&self, offset: usize) -> (Span, Option<Span>) {
        if offset < self.start {
            return (self.clone(), None);
        }
        if offset >= self.end {
            return (self.clone(), None);
        }

        let span1 = Span::new_with_meta(
            self.start,
            offset.min(self.end),
            self.style,
            self.meta.clone(),
        );
        let span2 = Span::new_with_meta(span1.end, self.end, self.style, self.meta.clone());
        (span1, Some(span2))
    }

    /// Move the span by a given offset.
    ///
    /// Both start and end are adjusted by adding the offset.
    ///
    /// # Arguments
    ///
    /// * `offset` - The amount to add to start and end (can be negative via wrapping).
    ///
    /// # Returns
    ///
    /// A new Span with adjusted positions.
    pub fn move_by(&self, offset: isize) -> Span {
        let new_start = (self.start as isize + offset).max(0) as usize;
        let new_end = (self.end as isize + offset).max(0) as usize;
        Span::new_with_meta(new_start, new_end, self.style, self.meta.clone())
    }

    /// Crop the span at a given offset.
    ///
    /// If offset is at or beyond the end, returns self unchanged.
    /// Otherwise, returns a span ending at offset.
    ///
    /// # Arguments
    ///
    /// * `offset` - The offset at which to crop.
    ///
    /// # Returns
    ///
    /// A new (possibly smaller) span.
    pub fn right_crop(&self, offset: usize) -> Span {
        if offset >= self.end {
            return self.clone();
        }
        Span::new_with_meta(
            self.start,
            offset.min(self.end),
            self.style,
            self.meta.clone(),
        )
    }

    /// Extend the span by a given number of cells.
    ///
    /// # Arguments
    ///
    /// * `cells` - The number of cells to add to the end.
    ///
    /// # Returns
    ///
    /// A new span with extended end position.
    pub fn extend(&self, cells: usize) -> Span {
        if cells == 0 {
            return self.clone();
        }
        Span::new_with_meta(self.start, self.end + cells, self.style, self.meta.clone())
    }
}

/// A part that can be assembled into Text.
///
/// Used by `Text::assemble()` to accept various input types.
#[derive(Debug, Clone)]
pub enum TextPart {
    /// Plain text without styling.
    Plain(String),
    /// Text with a style.
    Styled(String, Style),
    /// Another Text object.
    Text(Text),
}

impl From<&str> for TextPart {
    fn from(s: &str) -> Self {
        TextPart::Plain(s.to_string())
    }
}

impl From<String> for TextPart {
    fn from(s: String) -> Self {
        TextPart::Plain(s)
    }
}

impl From<Text> for TextPart {
    fn from(t: Text) -> Self {
        TextPart::Text(t)
    }
}

impl From<(&str, Style)> for TextPart {
    fn from((s, style): (&str, Style)) -> Self {
        TextPart::Styled(s.to_string(), style)
    }
}

impl From<(String, Style)> for TextPart {
    fn from((s, style): (String, Style)) -> Self {
        TextPart::Styled(s, style)
    }
}

/// Rich text with styled spans.
///
/// Text is the primary way to work with styled content in Rich.
/// It stores plain text with a list of spans that define styled regions.
///
/// # Example
///
/// ```
/// use rich_rs::Text;
/// use rich_rs::Style;
///
/// let mut text = Text::plain("Hello, World!");
/// text.stylize(0, 5, Style::new().with_bold(true));
/// text.stylize(7, 12, Style::new().with_italic(true));
/// ```
#[derive(Debug, Clone, Default)]
pub struct Text {
    /// The plain text content.
    text: String,
    /// Styled spans applied to the text.
    spans: Vec<Span>,
    /// Base style for the entire text.
    style: Option<Style>,
    /// Base metadata for the entire text.
    meta: Option<StyleMeta>,
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
            style: None,
            meta: None,
        }
    }

    /// Create text with a style applied to the entire content.
    ///
    /// Note: Unlike creating Text::plain and then calling stylize(),
    /// this sets the base style which affects the entire text including
    /// any padding added later. The base style is applied as a background
    /// to all spans during rendering.
    pub fn styled(text: impl Into<String>, style: Style) -> Self {
        let text = text.into();
        Text {
            text,
            spans: Vec::new(),
            style: Some(style),
            meta: None,
        }
    }

    /// Create text with a base style and base metadata applied to the entire content.
    pub fn styled_with_meta(text: impl Into<String>, style: Style, meta: StyleMeta) -> Self {
        let text = text.into();
        Text {
            text,
            spans: Vec::new(),
            style: Some(style),
            meta: if meta.is_empty() { None } else { Some(meta) },
        }
    }

    /// Create text from markup string.
    ///
    /// Parses BBCode-like markup (e.g., `[bold red]text[/]`) into styled Text.
    ///
    /// # Arguments
    ///
    /// * `markup` - The markup string to parse.
    /// * `emoji` - Whether to replace emoji codes (`:smile:` -> actual emoji).
    ///
    /// # Returns
    ///
    /// A `Text` object with styled spans, or an error if the markup is invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::Text;
    ///
    /// let text = Text::from_markup("[bold]Hello[/] World", true).unwrap();
    /// assert_eq!(text.plain_text(), "Hello World");
    /// ```
    pub fn from_markup(markup: &str, emoji: bool) -> Result<Text> {
        crate::markup::render(markup, emoji)
    }

    /// Create a Text object from a string containing ANSI escape codes.
    ///
    /// This is a port of Python Rich's `Text.from_ansi`, backed by `AnsiDecoder`.
    /// The decoder is lenient and will ignore unknown / malformed escape sequences.
    ///
    /// Style state may persist across lines, matching Rich behavior.
    pub fn from_ansi(ansi_text: &str) -> Text {
        let mut decoder = crate::ansi::AnsiDecoder::new();
        // Match Python Rich: `Text.from_ansi` constructs a joiner Text with an explicit (possibly empty)
        // base style. In rich-rs, using `Some(NULL_STYLE)` preserves that API-visible base-style
        // without affecting rendering (null styles are ignored when generating spans).
        let joiner = Text::styled("\n", Style::new());
        joiner.join(decoder.decode(ansi_text))
    }

    /// Assemble text from multiple parts.
    ///
    /// Each part can be:
    /// - A plain string (`&str` or `String`)
    /// - A `Text` object
    /// - A tuple of `(text, Style)`
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::{Text, TextPart, Style};
    ///
    /// let bold = Style::new().with_bold(true);
    /// let text = Text::assemble([
    ///     TextPart::from("Hello, "),
    ///     TextPart::from(("World", bold)),
    ///     TextPart::from("!"),
    /// ]);
    /// assert_eq!(text.plain_text(), "Hello, World!");
    /// ```
    pub fn assemble<I, P>(parts: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<TextPart>,
    {
        let mut result = Text::new();

        for part in parts {
            match part.into() {
                TextPart::Plain(s) => {
                    result.append(&s, None);
                }
                TextPart::Styled(s, style) => {
                    result.append(&s, Some(style));
                }
                TextPart::Text(t) => {
                    result.append_text(&t);
                }
            }
        }

        result
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

    /// Append another Text object, preserving its spans and base style.
    pub fn append_text(&mut self, other: &Text) {
        let offset = self.len();
        let other_len = other.len();
        self.text.push_str(&other.text);

        // If the other text has a base style/meta, add a span for it.
        // This preserves region-specific base attributes when merging Text objects.
        let other_base_style = other.style.unwrap_or_default();
        let other_base_meta = other.meta.clone().unwrap_or_default();
        if !other_base_style.is_null() || !other_base_meta.is_empty() {
            self.spans.push(Span::new_with_meta(
                offset,
                offset + other_len,
                other_base_style,
                Some(other_base_meta),
            ));
        }

        // Copy and offset spans from the other text
        for span in &other.spans {
            self.spans.push(Span::new_with_meta(
                span.start + offset,
                span.end + offset,
                span.style,
                span.meta.clone(),
            ));
        }
    }

    /// Apply a style to a range of the text (legacy API, kept for compatibility).
    ///
    /// Spans are clamped to the text bounds. Out-of-bounds or empty spans are ignored.
    /// For the enhanced version with negative index support, use `stylize_range`.
    pub fn stylize(&mut self, start: usize, end: usize, style: Style) {
        let length = self.len();
        if start >= length || end <= start {
            return;
        }
        let clamped_end = end.min(length);
        self.spans.push(Span::new(start, clamped_end, style));
    }

    /// Apply a style to a range of the text with negative index support.
    ///
    /// Negative indices count from the end of the text (-1 is the last character).
    /// If `end` is `None`, styles to the end of the text.
    ///
    /// # Arguments
    ///
    /// * `style` - The style to apply.
    /// * `start` - Start offset (negative indexing supported). Defaults to 0.
    /// * `end` - End offset (negative indexing supported), or `None` for end of text.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::{Text, Style};
    ///
    /// let mut text = Text::plain("Hello World");
    /// // Style the last 5 characters
    /// text.stylize_range(Style::new().with_bold(true), -5, None);
    /// ```
    pub fn stylize_range(&mut self, style: Style, start: isize, end: Option<isize>) {
        if style.is_null() {
            return;
        }

        let length = self.len() as isize;

        // Handle negative indices
        let start = if start < 0 {
            (length + start).max(0) as usize
        } else {
            start as usize
        };

        let end = match end {
            None => self.len(),
            Some(e) if e < 0 => (length + e).max(0) as usize,
            Some(e) => e as usize,
        };

        // Validate range
        if start >= self.len() || end <= start {
            return;
        }

        self.spans
            .push(Span::new(start, end.min(self.len()), style));
    }

    /// Apply a style to the text, inserting at the beginning of the spans list.
    ///
    /// Styles applied with `stylize_before` have lower priority than existing styles.
    /// This is useful for adding a base style that existing styles can override.
    ///
    /// # Arguments
    ///
    /// * `style` - The style to apply.
    /// * `start` - Start offset (negative indexing supported). Defaults to 0.
    /// * `end` - End offset (negative indexing supported), or `None` for end of text.
    pub fn stylize_before(&mut self, style: Style, start: isize, end: Option<isize>) {
        if style.is_null() {
            return;
        }

        let length = self.len() as isize;

        // Handle negative indices
        let start = if start < 0 {
            (length + start).max(0) as usize
        } else {
            start as usize
        };

        let end = match end {
            None => self.len(),
            Some(e) if e < 0 => (length + e).max(0) as usize,
            Some(e) => e as usize,
        };

        // Validate range
        if start >= self.len() || end <= start {
            return;
        }

        // Insert at the beginning for lower priority
        self.spans
            .insert(0, Span::new(start, end.min(self.len()), style));
    }

    /// Highlight text matching a regular expression.
    ///
    /// # Arguments
    ///
    /// * `pattern` - A regular expression pattern.
    /// * `style` - The style to apply to matches.
    ///
    /// # Returns
    ///
    /// The number of matches found.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::{Text, Style};
    ///
    /// let mut text = Text::plain("foo bar foo baz");
    /// let count = text.highlight_regex(r"foo", Style::new().with_bold(true));
    /// assert_eq!(count, 2);
    /// ```
    pub fn highlight_regex(&mut self, pattern: &str, style: Style) -> usize {
        let re = match Regex::new(pattern) {
            Ok(r) => r,
            Err(_) => return 0,
        };

        let mut count = 0;
        let plain = self.plain_text().to_string();

        for mat in re.find_iter(&plain) {
            // Convert byte offsets to character offsets
            let start_char = plain[..mat.start()].chars().count();
            let end_char = start_char + plain[mat.start()..mat.end()].chars().count();

            if end_char > start_char {
                self.spans.push(Span::new(start_char, end_char, style));
                count += 1;
            }
        }

        count
    }

    /// Highlight occurrences of specific words.
    ///
    /// # Arguments
    ///
    /// * `words` - Words to highlight.
    /// * `style` - The style to apply.
    /// * `case_sensitive` - Whether matching should be case-sensitive.
    ///
    /// # Returns
    ///
    /// The number of words highlighted.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::{Text, Style};
    ///
    /// let mut text = Text::plain("Hello World Hello");
    /// let count = text.highlight_words(&["Hello"], Style::new().with_bold(true), true);
    /// assert_eq!(count, 2);
    /// ```
    pub fn highlight_words(&mut self, words: &[&str], style: Style, case_sensitive: bool) -> usize {
        if words.is_empty() {
            return 0;
        }

        // Build regex pattern from words
        let pattern = words
            .iter()
            .map(|w| regex::escape(w))
            .collect::<Vec<_>>()
            .join("|");

        let pattern = if case_sensitive {
            pattern
        } else {
            format!("(?i){}", pattern)
        };

        let re = match Regex::new(&pattern) {
            Ok(r) => r,
            Err(_) => return 0,
        };

        let mut count = 0;
        let plain = self.plain_text().to_string();

        for mat in re.find_iter(&plain) {
            // Convert byte offsets to character offsets
            let start_char = plain[..mat.start()].chars().count();
            let end_char = start_char + plain[mat.start()..mat.end()].chars().count();

            if end_char > start_char {
                self.spans.push(Span::new(start_char, end_char, style));
                count += 1;
            }
        }

        count
    }

    /// Divide text at multiple offsets.
    ///
    /// This is a critical algorithm for text wrapping. It splits the text at
    /// the given character offsets and correctly distributes spans across
    /// the resulting Text objects.
    ///
    /// # Arguments
    ///
    /// * `offsets` - Character offsets at which to divide the text.
    ///
    /// # Returns
    ///
    /// A vector of Text objects, one for each division.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::Text;
    ///
    /// let text = Text::plain("Hello World!");
    /// let divided = text.divide([5, 6]);
    /// assert_eq!(divided.len(), 3);
    /// assert_eq!(divided[0].plain_text(), "Hello");
    /// assert_eq!(divided[1].plain_text(), " ");
    /// assert_eq!(divided[2].plain_text(), "World!");
    /// ```
    pub fn divide(&self, offsets: impl IntoIterator<Item = usize>) -> Vec<Text> {
        let plain = self.plain_text();
        let text_length = self.len();

        // Collect, sort, clamp, and deduplicate offsets
        let mut offsets: Vec<usize> = offsets
            .into_iter()
            .map(|o| o.min(text_length)) // Clamp to text length
            .collect();
        offsets.sort_unstable();
        offsets.dedup();

        // Filter out 0 and text_length since we add them below
        let offsets: Vec<usize> = offsets
            .into_iter()
            .filter(|&o| o > 0 && o < text_length)
            .collect();

        if offsets.is_empty() {
            return vec![self.clone()];
        }

        // Build line ranges: [0..offset[0]], [offset[0]..offset[1]], ..., [last_offset..len]
        let mut divide_offsets = vec![0];
        divide_offsets.extend(offsets.iter().copied());
        divide_offsets.push(text_length);

        // Create ranges from consecutive offset pairs
        let line_ranges: Vec<(usize, usize)> = divide_offsets
            .windows(2)
            .map(|w| (w[0], w[1]))
            .filter(|(start, end)| start < end) // Skip empty ranges
            .collect();

        if line_ranges.is_empty() {
            return vec![self.clone()];
        }

        // Extract substrings for each range (character-based slicing)
        let chars: Vec<char> = plain.chars().collect();
        let new_lines: Vec<Text> = line_ranges
            .iter()
            .map(|&(start, end)| {
                let clamped_end = end.min(chars.len());
                let clamped_start = start.min(clamped_end);
                let substring: String = chars[clamped_start..clamped_end].iter().collect();
                Text {
                    text: substring,
                    spans: Vec::new(),
                    style: self.style,
                    meta: self.meta.clone(),
                }
            })
            .collect();

        // If no spans, we're done
        if self.spans.is_empty() {
            return new_lines;
        }

        // Distribute spans to the appropriate lines
        let mut result = new_lines;
        let line_count = line_ranges.len();

        for span in &self.spans {
            // Skip invalid or out-of-bounds spans
            if span.start >= span.end || span.start >= text_length {
                continue;
            }
            let span_start = span.start;
            let span_end = span.end.min(text_length);

            // Binary search to find the starting line for this span
            let mut lower_bound = 0;
            let mut upper_bound = line_count;
            let mut start_line_no = (lower_bound + upper_bound) / 2;

            loop {
                if start_line_no >= line_count {
                    break;
                }
                let (line_start, line_end) = line_ranges[start_line_no];
                if span_start < line_start {
                    if start_line_no == 0 {
                        break;
                    }
                    upper_bound = start_line_no - 1;
                } else if span_start > line_end {
                    lower_bound = start_line_no + 1;
                } else {
                    break;
                }
                start_line_no = (lower_bound + upper_bound) / 2;
            }

            // Find the ending line for this span
            let end_line_no = if span_end < line_ranges[start_line_no].1 {
                start_line_no
            } else {
                lower_bound = start_line_no;
                upper_bound = line_count;
                let mut end_line_no = (lower_bound + upper_bound) / 2;

                loop {
                    if end_line_no >= line_count {
                        end_line_no = line_count - 1;
                        break;
                    }
                    let (line_start, line_end) = line_ranges[end_line_no];
                    if span_end < line_start {
                        if end_line_no == 0 {
                            break;
                        }
                        upper_bound = end_line_no - 1;
                    } else if span_end > line_end {
                        lower_bound = end_line_no + 1;
                    } else {
                        break;
                    }
                    end_line_no = (lower_bound + upper_bound) / 2;
                }
                end_line_no
            };

            // Add span to all lines it covers
            for line_no in start_line_no..=end_line_no.min(line_count - 1) {
                let (line_start, line_end) = line_ranges[line_no];
                let new_start = span_start.saturating_sub(line_start);
                let new_end = span_end
                    .saturating_sub(line_start)
                    .min(line_end - line_start);

                if new_end > new_start {
                    result[line_no].spans.push(Span::new_with_meta(
                        new_start,
                        new_end,
                        span.style,
                        span.meta.clone(),
                    ));
                }
            }
        }

        result
    }

    /// Get the spans.
    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    /// Get mutable access to spans.
    pub fn spans_mut(&mut self) -> &mut Vec<Span> {
        &mut self.spans
    }

    /// Get the base style.
    pub fn base_style(&self) -> Option<Style> {
        self.style
    }

    /// Set the base style.
    pub fn set_base_style(&mut self, style: Option<Style>) {
        self.style = style;
    }

    /// Create a copy of this text.
    pub fn copy(&self) -> Text {
        self.clone()
    }

    /// Create a blank copy with same metadata but no content.
    pub fn blank_copy(&self, plain: &str) -> Text {
        Text {
            text: plain.to_string(),
            spans: Vec::new(),
            style: self.style,
            meta: self.meta.clone(),
        }
    }

    /// Join multiple Text objects with this text as separator.
    pub fn join<I>(&self, texts: I) -> Text
    where
        I: IntoIterator<Item = Text>,
    {
        let mut result = self.blank_copy("");
        let mut first = true;

        for text in texts {
            if !first && !self.is_empty() {
                result.append_text(self);
            }
            result.append_text(&text);
            first = false;
        }

        result
    }

    // ========================================================================
    // Padding and alignment methods
    // ========================================================================

    /// Pad text on the right to reach target cell width.
    ///
    /// Returns a new Text with spaces appended to reach the target width.
    /// If the text is already wider than width, returns a clone unchanged.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::Text;
    ///
    /// let text = Text::plain("hello");
    /// let padded = text.pad_right(10);
    /// assert_eq!(padded.plain_text(), "hello     ");
    /// assert_eq!(padded.cell_len(), 10);
    /// ```
    pub fn pad_right(&self, width: usize) -> Text {
        let current_width = self.cell_len();
        if current_width >= width {
            return self.clone();
        }

        let mut result = self.clone();
        let spaces = " ".repeat(width - current_width);
        result.text.push_str(&spaces);
        result
    }

    /// Pad text on the left to reach target cell width.
    ///
    /// Returns a new Text with spaces prepended to reach the target width.
    /// Existing spans are shifted by the padding amount.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::Text;
    ///
    /// let text = Text::plain("hello");
    /// let padded = text.pad_left(10);
    /// assert_eq!(padded.plain_text(), "     hello");
    /// assert_eq!(padded.cell_len(), 10);
    /// ```
    pub fn pad_left(&self, width: usize) -> Text {
        let current_width = self.cell_len();
        if current_width >= width {
            return self.clone();
        }

        let pad_count = width - current_width;
        let spaces = " ".repeat(pad_count);

        // Shift all spans by the padding amount
        let shifted_spans: Vec<Span> = self
            .spans
            .iter()
            .map(|span| {
                Span::new_with_meta(
                    span.start + pad_count,
                    span.end + pad_count,
                    span.style,
                    span.meta.clone(),
                )
            })
            .collect();

        Text {
            text: format!("{}{}", spaces, self.text),
            spans: shifted_spans,
            style: self.style,
            meta: self.meta.clone(),
        }
    }

    /// Center text within a given cell width.
    ///
    /// Returns a new Text padded on both sides to center within the width.
    /// Left padding is (width - cell_len) / 2, right padding fills the rest.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::Text;
    ///
    /// let text = Text::plain("hi");
    /// let centered = text.center(6);
    /// assert_eq!(centered.plain_text(), "  hi  ");
    /// assert_eq!(centered.cell_len(), 6);
    /// ```
    pub fn center(&self, width: usize) -> Text {
        let current_width = self.cell_len();
        if current_width >= width {
            return self.clone();
        }

        let total_pad = width - current_width;
        let left_pad = total_pad / 2;
        let right_pad = total_pad - left_pad;

        let left_spaces = " ".repeat(left_pad);
        let right_spaces = " ".repeat(right_pad);

        // Shift all spans by the left padding amount
        let shifted_spans: Vec<Span> = self
            .spans
            .iter()
            .map(|span| {
                Span::new_with_meta(
                    span.start + left_pad,
                    span.end + left_pad,
                    span.style,
                    span.meta.clone(),
                )
            })
            .collect();

        Text {
            text: format!("{}{}{}", left_spaces, self.text, right_spaces),
            spans: shifted_spans,
            style: self.style,
            meta: self.meta.clone(),
        }
    }

    /// Expand tabs to spaces.
    ///
    /// Returns a new Text with tabs replaced by spaces, aligning to tab stops.
    ///
    /// # Arguments
    ///
    /// * `tab_size` - The tab stop width (default 8).
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::Text;
    ///
    /// let text = Text::plain("a\tb");
    /// let expanded = text.expand_tabs(4);
    /// assert_eq!(expanded.plain_text(), "a   b");
    /// ```
    pub fn expand_tabs(&self, tab_size: usize) -> Text {
        if !self.text.contains('\t') {
            return self.clone();
        }

        let tab_size = if tab_size == 0 { 8 } else { tab_size };

        let mut result_text = String::new();
        let mut result_spans: Vec<Span> = Vec::new();
        let mut cell_position: usize = 0;

        let chars: Vec<char> = self.text.chars().collect();

        for &c in &chars {
            if c == '\t' {
                // Calculate spaces needed to reach next tab stop
                let tab_remainder = cell_position % tab_size;
                let spaces = if tab_remainder == 0 {
                    tab_size
                } else {
                    tab_size - tab_remainder
                };

                result_text.push_str(&" ".repeat(spaces));
                cell_position += spaces;
            } else if c == '\n' {
                result_text.push(c);
                cell_position = 0; // Reset on newline
            } else {
                result_text.push(c);
                cell_position += crate::cells::char_width(c);
            }
        }

        // Rebuild spans with adjusted positions
        // We need to map old char offsets to new char offsets
        let mut old_to_new: Vec<usize> = Vec::with_capacity(chars.len() + 1);
        old_to_new.push(0);

        let mut new_pos: usize = 0;
        cell_position = 0;

        for &c in &chars {
            if c == '\t' {
                let tab_remainder = cell_position % tab_size;
                let spaces = if tab_remainder == 0 {
                    tab_size
                } else {
                    tab_size - tab_remainder
                };
                new_pos += spaces;
                cell_position += spaces;
            } else if c == '\n' {
                new_pos += 1;
                cell_position = 0;
            } else {
                new_pos += 1;
                cell_position += crate::cells::char_width(c);
            }
            old_to_new.push(new_pos);
        }

        for span in &self.spans {
            let new_start = if span.start < old_to_new.len() {
                old_to_new[span.start]
            } else {
                old_to_new.last().copied().unwrap_or(0)
            };
            let new_end = if span.end < old_to_new.len() {
                old_to_new[span.end]
            } else {
                old_to_new.last().copied().unwrap_or(0)
            };

            if new_end > new_start {
                result_spans.push(Span::new_with_meta(
                    new_start,
                    new_end,
                    span.style,
                    span.meta.clone(),
                ));
            }
        }

        Text {
            text: result_text,
            spans: result_spans,
            style: self.style,
            meta: self.meta.clone(),
        }
    }

    /// Add indentation guides to the text.
    ///
    /// This adds visual indentation guides (like vertical lines) to show
    /// the indentation level of each line.
    ///
    /// # Arguments
    ///
    /// * `indent_size` - The number of spaces per indentation level.
    /// * `style` - Optional style for the guide characters.
    ///
    /// # Returns
    ///
    /// A new Text with indentation guides added.
    ///
    /// # Note
    ///
    /// This is a stub implementation that returns the text unchanged.
    /// Full implementation is TODO.
    pub fn with_indent_guides(self, _indent_size: usize, _style: Option<crate::Style>) -> Text {
        // TODO: Implement actual indent guides
        // For now, return the text unchanged
        self
    }

    /// Strip trailing whitespace from the text.
    ///
    /// Returns a new Text with trailing whitespace removed.
    /// Spans are adjusted to fit within the new text bounds.
    pub fn rstrip(&self) -> Text {
        let trimmed = self.text.trim_end();
        let new_len = trimmed.chars().count();

        let adjusted_spans: Vec<Span> = self
            .spans
            .iter()
            .filter_map(|span| {
                if span.start >= new_len {
                    None
                } else {
                    Some(Span::new_with_meta(
                        span.start,
                        span.end.min(new_len),
                        span.style,
                        span.meta.clone(),
                    ))
                }
            })
            .filter(|span| !span.is_empty())
            .collect();

        Text {
            text: trimmed.to_string(),
            spans: adjusted_spans,
            style: self.style,
            meta: self.meta.clone(),
        }
    }

    /// Remove trailing whitespace beyond a certain width.
    ///
    /// Only removes whitespace characters that extend beyond the target size.
    /// This is used after wrapping to clean up trailing spaces on lines.
    ///
    /// # Arguments
    ///
    /// * `size` - The desired cell width target.
    pub fn rstrip_end(&self, size: usize) -> Text {
        let text_width = self.cell_len();
        if text_width <= size {
            return self.clone();
        }

        let excess = text_width - size;

        // Find how much trailing whitespace we have (in cell width)
        let mut trailing_ws_width = 0;
        for c in self.text.chars().rev() {
            if c.is_whitespace() {
                trailing_ws_width += crate::cells::char_width(c);
            } else {
                break;
            }
        }

        if trailing_ws_width == 0 {
            return self.clone();
        }

        // Remove trailing whitespace until we've removed min(trailing_ws_width, excess) cells
        let cells_to_remove = trailing_ws_width.min(excess);

        // Build new text by removing trailing whitespace
        let mut chars: Vec<char> = self.text.chars().collect();
        let mut removed = 0;
        while !chars.is_empty() && removed < cells_to_remove {
            if let Some(&c) = chars.last() {
                if c.is_whitespace() {
                    removed += crate::cells::char_width(c);
                    chars.pop();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let new_text: String = chars.iter().collect();
        let new_len = chars.len();

        let adjusted_spans: Vec<Span> = self
            .spans
            .iter()
            .filter_map(|span| {
                if span.start >= new_len {
                    None
                } else {
                    Some(Span::new_with_meta(
                        span.start,
                        span.end.min(new_len),
                        span.style,
                        span.meta.clone(),
                    ))
                }
            })
            .filter(|span| !span.is_empty())
            .collect();

        Text {
            text: new_text,
            spans: adjusted_spans,
            style: self.style,
            meta: self.meta.clone(),
        }
    }

    /// Truncate text to fit within a cell width.
    ///
    /// # Arguments
    ///
    /// * `max_width` - Maximum cell width.
    /// * `overflow` - How to handle overflow (Fold, Crop, Ellipsis).
    /// * `pad` - If true, pad with spaces if text is shorter than max_width.
    pub fn truncate(
        &self,
        max_width: usize,
        overflow: crate::console::OverflowMethod,
        pad: bool,
    ) -> Text {
        use crate::cells::set_cell_size;
        use crate::console::OverflowMethod;

        if overflow == OverflowMethod::Ignore {
            if pad && self.cell_len() < max_width {
                return self.pad_right(max_width);
            }
            return self.clone();
        }

        let current_width = self.cell_len();

        if current_width <= max_width {
            if pad {
                return self.pad_right(max_width);
            }
            return self.clone();
        }

        // Truncate the text
        let new_plain = if overflow == OverflowMethod::Ellipsis && max_width > 0 {
            let truncated = set_cell_size(&self.text, max_width.saturating_sub(1));
            format!("{}…", truncated.trim_end())
        } else {
            set_cell_size(&self.text, max_width)
        };

        let new_char_len = new_plain.chars().count();

        // Adjust spans
        let adjusted_spans: Vec<Span> = self
            .spans
            .iter()
            .filter_map(|span| {
                if span.start >= new_char_len {
                    None
                } else {
                    Some(Span::new_with_meta(
                        span.start,
                        span.end.min(new_char_len),
                        span.style,
                        span.meta.clone(),
                    ))
                }
            })
            .filter(|span| !span.is_empty())
            .collect();

        Text {
            text: new_plain,
            spans: adjusted_spans,
            style: self.style,
            meta: self.meta.clone(),
        }
    }

    /// Split text on a separator into a list of Text objects.
    ///
    /// # Arguments
    ///
    /// * `separator` - The string to split on.
    /// * `include_separator` - If true, include the separator at the end of each line.
    /// * `allow_blank` - If true, include a blank line if text ends with separator.
    ///
    /// # Returns
    ///
    /// A vector of Text objects, one per split segment.
    pub fn split(&self, separator: &str, include_separator: bool, allow_blank: bool) -> Vec<Text> {
        if separator.is_empty() {
            return vec![self.clone()];
        }

        if !self.text.contains(separator) {
            return vec![self.clone()];
        }

        // Find all separator positions (ranges)
        let chars: Vec<char> = self.text.chars().collect();
        let sep_chars: Vec<char> = separator.chars().collect();
        let sep_len = sep_chars.len();
        let text_len = chars.len();

        // Collect separator ranges (start, end)
        let mut sep_ranges: Vec<(usize, usize)> = Vec::new();
        let mut i = 0;
        while i + sep_len <= text_len {
            if &chars[i..i + sep_len] == sep_chars.as_slice() {
                sep_ranges.push((i, i + sep_len));
                i += sep_len;
            } else {
                i += 1;
            }
        }

        if sep_ranges.is_empty() {
            return vec![self.clone()];
        }

        // Build segments by extracting text between separators
        let mut result: Vec<Text> = Vec::new();
        let mut pos = 0;

        for (sep_start, sep_end) in &sep_ranges {
            // Extract segment before separator
            if include_separator {
                // Include everything from pos up to and including separator
                if *sep_end > pos {
                    let segment_text: String = chars[pos..*sep_end].iter().collect();
                    result.push(self.slice_at_offsets(pos, *sep_end, &segment_text));
                }
            } else {
                // Only include the part before the separator
                let segment_text: String = chars[pos..*sep_start].iter().collect();
                if allow_blank || !segment_text.is_empty() {
                    result.push(self.slice_at_offsets(pos, *sep_start, &segment_text));
                }
            }
            pos = *sep_end;
        }

        // Handle the trailing segment after the last separator
        if pos < text_len {
            let segment_text: String = chars[pos..].iter().collect();
            if allow_blank || !segment_text.is_empty() {
                result.push(self.slice_at_offsets(pos, text_len, &segment_text));
            }
        } else if include_separator {
            // Text ends with separator and include_separator is true
            // Add trailing empty segment only if allow_blank
            if allow_blank {
                result.push(self.blank_copy(""));
            }
        } else {
            // Text ends with separator and include_separator is false
            // Add trailing empty segment only if allow_blank
            if allow_blank {
                result.push(self.blank_copy(""));
            }
        }

        result
    }

    /// Helper to create a slice with adjusted spans.
    fn slice_at_offsets(&self, start: usize, end: usize, text: &str) -> Text {
        let adjusted_spans: Vec<Span> = self
            .spans
            .iter()
            .filter_map(|span| {
                if span.end <= start || span.start >= end {
                    None
                } else {
                    let new_start = span.start.saturating_sub(start);
                    let new_end = span.end.min(end).saturating_sub(start);
                    if new_start < new_end {
                        Some(Span::new_with_meta(
                            new_start,
                            new_end,
                            span.style,
                            span.meta.clone(),
                        ))
                    } else {
                        None
                    }
                }
            })
            .collect();

        Text {
            text: text.to_string(),
            spans: adjusted_spans,
            style: self.style,
            meta: self.meta.clone(),
        }
    }

    // ========================================================================
    // Full justification helper
    // ========================================================================

    /// Justify text to fill width by expanding spaces between words.
    ///
    /// Used for "full" justification. This expands spaces between words
    /// to make the text fill the entire width.
    fn justify_full(&self, width: usize) -> Text {
        let current_width = self.cell_len();
        if current_width >= width {
            return self.clone();
        }

        // Split into words on spaces.
        // Note: Python Rich uses `split(" ")`, which preserves empty tokens for
        // consecutive spaces. Our split currently drops empty segments unless
        // `allow_blank` is true; this is sufficient for the demo content which
        // uses single spaces between words.
        let words = self.split(" ", false, false);
        if words.len() <= 1 {
            // Single word or empty - can't justify, just pad right
            return self.pad_right(width);
        }

        // Calculate total word width and number of gaps
        let words_width: usize = words.iter().map(|w| w.cell_len()).sum();
        let num_gaps = words.len().saturating_sub(1);
        if num_gaps == 0 {
            return self.pad_right(width);
        }

        // Distribute spaces to match Python Rich:
        // start with 1 space per gap, then add extra spaces from right-to-left.
        let mut spaces: Vec<usize> = vec![1; num_gaps];
        let mut num_spaces = num_gaps;
        let mut index = 0usize;
        while words_width + num_spaces < width {
            let pos = num_gaps.saturating_sub(index).saturating_sub(1);
            spaces[pos] += 1;
            num_spaces += 1;
            index = (index + 1) % num_gaps;
        }

        let mut result = Text::new();
        result.style = self.style;

        for (i, word) in words.iter().enumerate() {
            result.append_text(word);

            if i < num_gaps {
                // Add spaces between words
                result.append(" ".repeat(spaces[i]), None);
            }
        }

        result
    }

    // ========================================================================
    // Wrap method
    // ========================================================================

    /// Wrap text to fit within a given width.
    ///
    /// This method word-wraps the text to fit within the specified cell width,
    /// applying justification and handling overflow as specified.
    ///
    /// # Arguments
    ///
    /// * `width` - Maximum width in cells.
    /// * `justify` - Text justification (None for no justification).
    /// * `overflow` - How to handle words longer than width.
    /// * `tab_size` - Tab stop width (default 8).
    /// * `no_wrap` - If true, don't wrap (just return self).
    ///
    /// # Returns
    ///
    /// A vector of Text objects, one per wrapped line.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::{Text, OverflowMethod};
    ///
    /// let text = Text::plain("hello world this is a test");
    /// let lines = text.wrap(10, None, Some(OverflowMethod::Fold), 8, false);
    /// assert!(lines.len() >= 3);
    /// ```
    pub fn wrap(
        &self,
        width: usize,
        justify: Option<crate::console::JustifyMethod>,
        overflow: Option<crate::console::OverflowMethod>,
        tab_size: usize,
        no_wrap: bool,
    ) -> Vec<Text> {
        use crate::console::{JustifyMethod, OverflowMethod};
        use crate::wrap::divide_line;

        let wrap_justify = justify.unwrap_or(JustifyMethod::Default);
        let wrap_overflow = overflow.unwrap_or(OverflowMethod::Fold);

        // If overflow is Ignore, treat as no_wrap
        let no_wrap = no_wrap || wrap_overflow == OverflowMethod::Ignore;

        let mut all_lines: Vec<Text> = Vec::new();

        // Split on existing newlines first
        let source_lines = self.split("\n", false, true);

        for line in source_lines {
            // Expand tabs
            let line = if line.plain_text().contains('\t') {
                line.expand_tabs(tab_size)
            } else {
                line
            };

            let wrapped_lines = if no_wrap {
                vec![line.clone()]
            } else {
                // Get break positions using divide_line
                let fold = wrap_overflow == OverflowMethod::Fold;
                let offsets = divide_line(line.plain_text(), width, fold);

                if offsets.is_empty() {
                    vec![line.clone()]
                } else {
                    // Convert byte offsets to character offsets
                    let char_offsets: Vec<usize> = offsets
                        .iter()
                        .map(|&byte_offset| line.plain_text()[..byte_offset].chars().count())
                        .collect();
                    line.divide(char_offsets)
                }
            };

            // Process each wrapped line
            for wrapped_line in wrapped_lines {
                all_lines.push(wrapped_line);
            }
        }

        // Apply post-processing: rstrip_end, justification, truncation
        let num_lines = all_lines.len();
        for (i, wrapped_line) in all_lines.iter_mut().enumerate() {
            let is_last_line = i == num_lines - 1;

            // Strip trailing whitespace beyond width (only if wrapping)
            if !no_wrap {
                *wrapped_line = wrapped_line.rstrip_end(width);
            }

            // Apply justification
            *wrapped_line = match wrap_justify {
                JustifyMethod::Left => wrapped_line.pad_right(width),
                JustifyMethod::Right => {
                    let stripped = wrapped_line.rstrip();
                    stripped.pad_left(width)
                }
                JustifyMethod::Center => {
                    let stripped = wrapped_line.rstrip();
                    stripped.center(width)
                }
                JustifyMethod::Full => {
                    // Full justification - last line should be left-aligned
                    if is_last_line {
                        wrapped_line.rstrip().pad_right(width)
                    } else {
                        wrapped_line.justify_full(width)
                    }
                }
                JustifyMethod::Default => wrapped_line.clone(),
            };

            // Truncate if needed (but not for no_wrap/ignore)
            if !no_wrap {
                *wrapped_line = wrapped_line.truncate(width, wrap_overflow, false);
            }
        }

        all_lines
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

/// Implement Renderable for Text.
///
/// This converts Text to Segments for rendering to the terminal.
impl Renderable for Text {
    fn render(&self, _console: &Console, options: &ConsoleOptions) -> Segments {
        let text = self.plain_text();
        let width = options.max_width;

        // Even when `no_wrap` is enabled, we still need to run through `wrap()` when
        // justification or overflow is requested, so that padding/truncation can be applied.
        let needs_processing = width > 0
            && (options.justify.is_some()
                || options.overflow.is_some()
                || text.lines().any(|line| cell_len(line) > width));

        if !needs_processing {
            return self.render_unwrapped();
        }

        let lines = self.wrap(
            width,
            options.justify,
            options.overflow,
            options.tab_size,
            options.no_wrap,
        );

        if lines.len() == 1 {
            return lines[0].render_unwrapped();
        }

        // Render each already-wrapped line without re-running wrap/justify/overflow.
        // Re-processing would strip trailing padding and re-center again, which can
        // shift multiline centered text to the right line-by-line (demo parity issue).
        let mut segments = Segments::new();
        for (i, line) in lines.iter().enumerate() {
            segments.extend(line.render_unwrapped());
            if i + 1 < lines.len() {
                segments.push(Segment::line());
            }
        }

        segments
    }

    fn measure(&self, _console: &Console, _options: &ConsoleOptions) -> Measurement {
        let text = self.plain_text();
        let lines: Vec<&str> = text.lines().collect();

        let max_width = lines.iter().map(|line| cell_len(line)).max().unwrap_or(0);

        let words: Vec<&str> = text.split_whitespace().collect();
        let min_width = words
            .iter()
            .map(|word| cell_len(word))
            .max()
            .unwrap_or(max_width);

        Measurement::new(min_width, max_width)
    }
}

impl Text {
    fn render_unwrapped(&self) -> Segments {
        let text = self.plain_text();

        // Fast path: no spans - still apply base style if present
        if self.spans.is_empty() {
            let base_style = self.style.unwrap_or_default();
            let base_meta = self.meta.clone().unwrap_or_default();
            let segment = match (base_style.is_null(), base_meta.is_empty()) {
                (true, true) => Segment::new(text.to_string()),
                (true, false) => Segment::new_with_meta(text.to_string(), base_meta),
                (false, true) => Segment::styled(text.to_string(), base_style),
                (false, false) => {
                    Segment::styled_with_meta(text.to_string(), base_style, base_meta)
                }
            };
            return Segments::from(segment);
        }

        // Build a list of events: (offset, is_end, span_index)
        // span_index 0 is reserved for the base style
        let enumerated_spans: Vec<(usize, &Span)> = self
            .spans
            .iter()
            .enumerate()
            .map(|(i, s)| (i + 1, s))
            .collect();

        // Build style map: index -> style
        let mut style_map: std::collections::HashMap<usize, Style> =
            std::collections::HashMap::new();
        style_map.insert(0, self.style.unwrap_or_default());
        for (index, span) in &enumerated_spans {
            style_map.insert(*index, span.style);
        }

        // Build meta map: index -> meta
        let mut meta_map: std::collections::HashMap<usize, StyleMeta> =
            std::collections::HashMap::new();
        meta_map.insert(0, self.meta.clone().unwrap_or_default());
        for (index, span) in &enumerated_spans {
            meta_map.insert(*index, span.meta.clone().unwrap_or_default());
        }

        // Build events
        let mut events: Vec<(usize, bool, usize)> = Vec::new();
        events.push((0, false, 0)); // Base style starts at 0
        for (index, span) in &enumerated_spans {
            events.push((span.start, false, *index)); // Start event
            events.push((span.end, true, *index)); // End event
        }
        events.push((self.len(), true, 0)); // Base style ends at text end

        // Sort by offset, then by is_end (starts before ends at same position)
        events.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

        // Process events and generate segments
        let mut segments = Segments::new();
        let mut stack: Vec<usize> = Vec::new();

        let chars: Vec<char> = text.chars().collect();

        for i in 0..events.len() - 1 {
            let (offset, leaving, style_id) = events[i];
            let (next_offset, _, _) = events[i + 1];

            if leaving {
                // Remove style from stack
                if let Some(pos) = stack.iter().position(|&x| x == style_id) {
                    stack.remove(pos);
                }
            } else {
                // Add style to stack
                stack.push(style_id);
            }

            // Generate segment for this region
            if next_offset > offset && offset < chars.len() {
                let end = next_offset.min(chars.len());
                let segment_text: String = chars[offset..end].iter().collect();

                // Combine styles from stack (later styles override earlier ones)
                let mut combined_style = Style::new();
                let mut combined_meta = StyleMeta::new();
                let mut sorted_stack = stack.clone();
                sorted_stack.sort();
                for &style_id in &sorted_stack {
                    if let Some(&style) = style_map.get(&style_id) {
                        combined_style = combined_style.combine(&style);
                    }
                    if let Some(meta) = meta_map.get(&style_id) {
                        combined_meta = combined_meta.combine(meta);
                    }
                }

                match (combined_style.is_null(), combined_meta.is_empty()) {
                    (true, true) => segments.push(Segment::new(segment_text)),
                    (true, false) => {
                        segments.push(Segment::new_with_meta(segment_text, combined_meta))
                    }
                    (false, true) => segments.push(Segment::styled(segment_text, combined_style)),
                    (false, false) => segments.push(Segment::styled_with_meta(
                        segment_text,
                        combined_style,
                        combined_meta,
                    )),
                }
            }
        }

        segments
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Span tests ====================

    #[test]
    fn test_span_new() {
        let style = Style::new().with_bold(true);
        let span = Span::new(0, 10, style);
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 10);
        assert_eq!(span.style, style);
    }

    #[test]
    fn test_span_is_empty() {
        let style = Style::new();
        assert!(Span::new(5, 5, style).is_empty());
        assert!(Span::new(10, 5, style).is_empty());
        assert!(!Span::new(0, 5, style).is_empty());
    }

    #[test]
    fn test_span_split_middle() {
        let style = Style::new().with_bold(true);
        let span = Span::new(0, 10, style);
        let (first, second) = span.split(5);

        assert_eq!(first.start, 0);
        assert_eq!(first.end, 5);
        assert!(second.is_some());
        let second = second.unwrap();
        assert_eq!(second.start, 5);
        assert_eq!(second.end, 10);
    }

    #[test]
    fn test_span_split_before_start() {
        let style = Style::new();
        let span = Span::new(5, 10, style);
        let (first, second) = span.split(3);

        assert_eq!(first.start, 5);
        assert_eq!(first.end, 10);
        assert!(second.is_none());
    }

    #[test]
    fn test_span_split_after_end() {
        let style = Style::new();
        let span = Span::new(0, 5, style);
        let (first, second) = span.split(10);

        assert_eq!(first.start, 0);
        assert_eq!(first.end, 5);
        assert!(second.is_none());
    }

    #[test]
    fn test_span_split_at_end() {
        let style = Style::new();
        let span = Span::new(0, 5, style);
        let (first, second) = span.split(5);

        assert_eq!(first.start, 0);
        assert_eq!(first.end, 5);
        assert!(second.is_none());
    }

    #[test]
    fn test_span_move_positive() {
        let style = Style::new();
        let span = Span::new(5, 10, style);
        let moved = span.move_by(3);

        assert_eq!(moved.start, 8);
        assert_eq!(moved.end, 13);
    }

    #[test]
    fn test_span_move_negative() {
        let style = Style::new();
        let span = Span::new(5, 10, style);
        let moved = span.move_by(-3);

        assert_eq!(moved.start, 2);
        assert_eq!(moved.end, 7);
    }

    #[test]
    fn test_span_move_negative_clamp() {
        let style = Style::new();
        let span = Span::new(2, 5, style);
        let moved = span.move_by(-10);

        assert_eq!(moved.start, 0);
        assert_eq!(moved.end, 0);
    }

    #[test]
    fn test_span_right_crop() {
        let style = Style::new();
        let span = Span::new(0, 10, style);

        let cropped = span.right_crop(5);
        assert_eq!(cropped.start, 0);
        assert_eq!(cropped.end, 5);

        let uncropped = span.right_crop(15);
        assert_eq!(uncropped.start, 0);
        assert_eq!(uncropped.end, 10);
    }

    #[test]
    fn test_span_extend() {
        let style = Style::new();
        let span = Span::new(0, 5, style);

        let extended = span.extend(3);
        assert_eq!(extended.start, 0);
        assert_eq!(extended.end, 8);

        let unchanged = span.extend(0);
        assert_eq!(unchanged.start, 0);
        assert_eq!(unchanged.end, 5);
    }

    // ==================== Text basic tests ====================

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
        // Text::styled sets base style, not a span (matching Python behavior)
        assert_eq!(text.spans().len(), 0);
        assert_eq!(text.base_style(), Some(style));
    }

    #[test]
    fn test_text_append() {
        let mut text = Text::new();
        text.append("hello ", None);
        text.append("world", Some(Style::new().with_bold(true)));
        assert_eq!(text.plain_text(), "hello world");
        assert_eq!(text.spans().len(), 1);
    }

    #[test]
    fn test_text_append_text() {
        let mut text = Text::plain("Hello ");
        let other = Text::styled("World", Style::new().with_bold(true));
        text.append_text(&other);

        assert_eq!(text.plain_text(), "Hello World");
        assert_eq!(text.spans().len(), 1);
        assert_eq!(text.spans()[0].start, 6);
        assert_eq!(text.spans()[0].end, 11);
    }

    // ==================== Text::assemble tests ====================

    #[test]
    fn test_text_assemble() {
        let bold = Style::new().with_bold(true);
        let text = Text::assemble([
            TextPart::Plain("Hello, ".to_string()),
            TextPart::Styled("World".to_string(), bold),
            TextPart::Plain("!".to_string()),
        ]);

        assert_eq!(text.plain_text(), "Hello, World!");
        assert_eq!(text.spans().len(), 1);
        assert_eq!(text.spans()[0].start, 7);
        assert_eq!(text.spans()[0].end, 12);
    }

    #[test]
    fn test_text_assemble_with_text() {
        let inner = Text::styled("styled", Style::new().with_italic(true));
        let text = Text::assemble([
            TextPart::Plain("prefix ".to_string()),
            TextPart::Text(inner),
            TextPart::Plain(" suffix".to_string()),
        ]);

        assert_eq!(text.plain_text(), "prefix styled suffix");
    }

    // ==================== stylize_range tests ====================

    #[test]
    fn test_stylize_range_basic() {
        let mut text = Text::plain("Hello World");
        text.stylize_range(Style::new().with_bold(true), 0, Some(5));

        assert_eq!(text.spans().len(), 1);
        assert_eq!(text.spans()[0].start, 0);
        assert_eq!(text.spans()[0].end, 5);
    }

    #[test]
    fn test_stylize_range_negative_start() {
        let mut text = Text::plain("Hello World");
        text.stylize_range(Style::new().with_bold(true), -5, None);

        assert_eq!(text.spans().len(), 1);
        assert_eq!(text.spans()[0].start, 6); // 11 - 5 = 6
        assert_eq!(text.spans()[0].end, 11);
    }

    #[test]
    fn test_stylize_range_negative_end() {
        let mut text = Text::plain("Hello World");
        text.stylize_range(Style::new().with_bold(true), 0, Some(-6));

        assert_eq!(text.spans().len(), 1);
        assert_eq!(text.spans()[0].start, 0);
        assert_eq!(text.spans()[0].end, 5); // 11 - 6 = 5
    }

    #[test]
    fn test_stylize_range_none_end() {
        let mut text = Text::plain("Hello World");
        text.stylize_range(Style::new().with_bold(true), 6, None);

        assert_eq!(text.spans().len(), 1);
        assert_eq!(text.spans()[0].start, 6);
        assert_eq!(text.spans()[0].end, 11);
    }

    #[test]
    fn test_stylize_range_invalid() {
        let mut text = Text::plain("Hello");
        // Start after end
        text.stylize_range(Style::new().with_bold(true), 10, Some(5));
        assert!(text.spans().is_empty());

        // Start >= length
        text.stylize_range(Style::new().with_bold(true), 10, None);
        assert!(text.spans().is_empty());
    }

    // ==================== stylize_before tests ====================

    #[test]
    fn test_stylize_before() {
        let mut text = Text::plain("Hello World");
        text.stylize_range(Style::new().with_bold(true), 0, None);
        text.stylize_before(Style::new().with_italic(true), 0, None);

        // stylize_before should insert at beginning
        assert_eq!(text.spans().len(), 2);
        assert_eq!(text.spans()[0].style.italic, Some(true));
        assert_eq!(text.spans()[1].style.bold, Some(true));
    }

    // ==================== highlight_regex tests ====================

    #[test]
    fn test_highlight_regex_basic() {
        let mut text = Text::plain("foo bar foo baz");
        let count = text.highlight_regex(r"foo", Style::new().with_bold(true));

        assert_eq!(count, 2);
        assert_eq!(text.spans().len(), 2);
    }

    #[test]
    fn test_highlight_regex_no_match() {
        let mut text = Text::plain("hello world");
        let count = text.highlight_regex(r"xyz", Style::new().with_bold(true));

        assert_eq!(count, 0);
        assert!(text.spans().is_empty());
    }

    #[test]
    fn test_highlight_regex_invalid() {
        let mut text = Text::plain("hello world");
        let count = text.highlight_regex(r"[invalid", Style::new().with_bold(true));

        assert_eq!(count, 0);
    }

    // ==================== highlight_words tests ====================

    #[test]
    fn test_highlight_words_basic() {
        let mut text = Text::plain("Hello World Hello");
        let count = text.highlight_words(&["Hello"], Style::new().with_bold(true), true);

        assert_eq!(count, 2);
        assert_eq!(text.spans().len(), 2);
    }

    #[test]
    fn test_highlight_words_case_insensitive() {
        let mut text = Text::plain("Hello HELLO hello");
        let count = text.highlight_words(&["hello"], Style::new().with_bold(true), false);

        assert_eq!(count, 3);
    }

    #[test]
    fn test_highlight_words_case_sensitive() {
        let mut text = Text::plain("Hello HELLO hello");
        let count = text.highlight_words(&["Hello"], Style::new().with_bold(true), true);

        assert_eq!(count, 1);
    }

    #[test]
    fn test_highlight_words_multiple() {
        let mut text = Text::plain("foo bar baz foo");
        let count = text.highlight_words(&["foo", "bar"], Style::new().with_bold(true), true);

        assert_eq!(count, 3); // foo, bar, foo
    }

    #[test]
    fn test_highlight_words_empty() {
        let mut text = Text::plain("hello");
        let count = text.highlight_words(&[], Style::new().with_bold(true), true);

        assert_eq!(count, 0);
    }

    // ==================== divide tests ====================

    #[test]
    fn test_divide_empty_offsets() {
        let text = Text::plain("Hello World");
        let divided = text.divide([]);

        assert_eq!(divided.len(), 1);
        assert_eq!(divided[0].plain_text(), "Hello World");
    }

    #[test]
    fn test_divide_single_offset() {
        let text = Text::plain("Hello World");
        let divided = text.divide([5]);

        assert_eq!(divided.len(), 2);
        assert_eq!(divided[0].plain_text(), "Hello");
        assert_eq!(divided[1].plain_text(), " World");
    }

    #[test]
    fn test_divide_multiple_offsets() {
        let text = Text::plain("Hello World!");
        let divided = text.divide([5, 6]);

        assert_eq!(divided.len(), 3);
        assert_eq!(divided[0].plain_text(), "Hello");
        assert_eq!(divided[1].plain_text(), " ");
        assert_eq!(divided[2].plain_text(), "World!");
    }

    #[test]
    fn test_divide_with_spans() {
        let mut text = Text::plain("Hello World");
        text.stylize(0, 5, Style::new().with_bold(true));

        let divided = text.divide([5]);

        assert_eq!(divided.len(), 2);
        assert_eq!(divided[0].plain_text(), "Hello");
        assert_eq!(divided[0].spans().len(), 1);
        assert_eq!(divided[0].spans()[0].start, 0);
        assert_eq!(divided[0].spans()[0].end, 5);

        assert_eq!(divided[1].plain_text(), " World");
        assert!(divided[1].spans().is_empty());
    }

    #[test]
    fn test_divide_span_crosses_boundary() {
        let mut text = Text::plain("Hello World");
        // Span covers "llo Wo" (3-9)
        text.stylize(3, 9, Style::new().with_bold(true));

        let divided = text.divide([5]);

        // First part: "Hello" with span 3-5
        assert_eq!(divided[0].plain_text(), "Hello");
        assert_eq!(divided[0].spans().len(), 1);
        assert_eq!(divided[0].spans()[0].start, 3);
        assert_eq!(divided[0].spans()[0].end, 5);

        // Second part: " World" with span 0-4 (was 5-9, offset by -5)
        assert_eq!(divided[1].plain_text(), " World");
        assert_eq!(divided[1].spans().len(), 1);
        assert_eq!(divided[1].spans()[0].start, 0);
        assert_eq!(divided[1].spans()[0].end, 4);
    }

    // ==================== Renderable tests ====================

    #[test]
    fn test_text_render_plain() {
        let text = Text::plain("Hello World");
        let console = Console::new();
        let options = ConsoleOptions::default();

        let segments = text.render(&console, &options);
        assert_eq!(segments.len(), 1);
        assert_eq!(&*segments.iter().next().unwrap().text, "Hello World");
    }

    #[test]
    fn test_text_render_styled() {
        let mut text = Text::plain("Hello World");
        text.stylize(0, 5, Style::new().with_bold(true));

        let console = Console::new();
        let options = ConsoleOptions::default();

        let segments = text.render(&console, &options);
        assert!(segments.len() >= 2);
    }

    #[test]
    fn test_text_measure() {
        let text = Text::plain("Hello\nWorld!");
        let console = Console::new();
        let options = ConsoleOptions::default();

        let measurement = text.measure(&console, &options);
        assert_eq!(measurement.maximum, 6); // "World!" is longest
        assert_eq!(measurement.minimum, 6); // "World!" is longest word
    }

    // ==================== from_markup tests ====================

    #[test]
    fn test_from_markup() {
        let text = Text::from_markup("[bold]Hello[/] World", false).unwrap();
        assert_eq!(text.plain_text(), "Hello World");
        assert!(!text.spans().is_empty());
    }

    // ==================== join tests ====================

    #[test]
    fn test_text_join() {
        let separator = Text::plain(", ");
        let texts = vec![Text::plain("a"), Text::plain("b"), Text::plain("c")];

        let joined = separator.join(texts);
        assert_eq!(joined.plain_text(), "a, b, c");
    }

    #[test]
    fn test_text_join_empty_separator() {
        let separator = Text::plain("");
        let texts = vec![Text::plain("a"), Text::plain("b")];

        let joined = separator.join(texts);
        assert_eq!(joined.plain_text(), "ab");
    }

    // ==================== Unicode tests ====================

    #[test]
    fn test_text_unicode_len() {
        let text = Text::plain("你好");
        assert_eq!(text.len(), 2); // 2 characters
        assert_eq!(text.cell_len(), 4); // 4 cells (each CJK char is 2 wide)
    }

    #[test]
    fn test_divide_unicode() {
        let text = Text::plain("你好世界");
        let divided = text.divide([2]);

        assert_eq!(divided.len(), 2);
        assert_eq!(divided[0].plain_text(), "你好");
        assert_eq!(divided[1].plain_text(), "世界");
    }

    // ==================== pad_right tests ====================

    #[test]
    fn test_pad_right_basic() {
        let text = Text::plain("hello");
        let padded = text.pad_right(10);
        assert_eq!(padded.plain_text(), "hello     ");
        assert_eq!(padded.cell_len(), 10);
    }

    #[test]
    fn test_pad_right_already_wide() {
        let text = Text::plain("hello world");
        let padded = text.pad_right(5);
        assert_eq!(padded.plain_text(), "hello world");
    }

    #[test]
    fn test_pad_right_preserves_spans() {
        let mut text = Text::plain("hello");
        text.stylize(0, 5, Style::new().with_bold(true));
        let padded = text.pad_right(10);
        assert_eq!(padded.spans().len(), 1);
        assert_eq!(padded.spans()[0].start, 0);
        assert_eq!(padded.spans()[0].end, 5);
    }

    // ==================== pad_left tests ====================

    #[test]
    fn test_pad_left_basic() {
        let text = Text::plain("hello");
        let padded = text.pad_left(10);
        assert_eq!(padded.plain_text(), "     hello");
        assert_eq!(padded.cell_len(), 10);
    }

    #[test]
    fn test_pad_left_shifts_spans() {
        let mut text = Text::plain("hello");
        text.stylize(0, 5, Style::new().with_bold(true));
        let padded = text.pad_left(10);
        assert_eq!(padded.spans().len(), 1);
        assert_eq!(padded.spans()[0].start, 5); // Shifted by padding
        assert_eq!(padded.spans()[0].end, 10);
    }

    // ==================== center tests ====================

    #[test]
    fn test_center_basic() {
        let text = Text::plain("hi");
        let centered = text.center(6);
        assert_eq!(centered.plain_text(), "  hi  ");
        assert_eq!(centered.cell_len(), 6);
    }

    #[test]
    fn test_center_odd_padding() {
        let text = Text::plain("hi");
        let centered = text.center(7);
        // 5 total padding, 2 left, 3 right
        assert_eq!(centered.plain_text(), "  hi   ");
    }

    #[test]
    fn test_center_shifts_spans() {
        let mut text = Text::plain("hi");
        text.stylize(0, 2, Style::new().with_bold(true));
        let centered = text.center(6);
        assert_eq!(centered.spans().len(), 1);
        assert_eq!(centered.spans()[0].start, 2);
        assert_eq!(centered.spans()[0].end, 4);
    }

    // ==================== expand_tabs tests ====================

    #[test]
    fn test_expand_tabs_basic() {
        let text = Text::plain("a\tb");
        let expanded = text.expand_tabs(4);
        assert_eq!(expanded.plain_text(), "a   b");
    }

    #[test]
    fn test_expand_tabs_multiple() {
        let text = Text::plain("a\tbc\td");
        let expanded = text.expand_tabs(4);
        // "a" at pos 0, tab expands to 3 spaces (to reach pos 4)
        // "bc" at pos 4-5, tab expands to 2 spaces (to reach pos 8)
        // "d" at pos 8
        assert_eq!(expanded.plain_text(), "a   bc  d");
    }

    #[test]
    fn test_expand_tabs_no_tabs() {
        let text = Text::plain("hello");
        let expanded = text.expand_tabs(4);
        assert_eq!(expanded.plain_text(), "hello");
    }

    #[test]
    fn test_expand_tabs_preserves_spans() {
        let mut text = Text::plain("a\tb");
        text.stylize(0, 1, Style::new().with_bold(true)); // Style "a"
        text.stylize(2, 3, Style::new().with_italic(true)); // Style "b"
        let expanded = text.expand_tabs(4);

        // "a" should still be styled at position 0
        // "b" should now be at position 4 (after "a   ")
        assert_eq!(expanded.spans().len(), 2);
        assert_eq!(expanded.spans()[0].start, 0);
        assert_eq!(expanded.spans()[0].end, 1);
        assert_eq!(expanded.spans()[1].start, 4);
        assert_eq!(expanded.spans()[1].end, 5);
    }

    // ==================== rstrip tests ====================

    #[test]
    fn test_rstrip_basic() {
        let text = Text::plain("hello   ");
        let stripped = text.rstrip();
        assert_eq!(stripped.plain_text(), "hello");
    }

    #[test]
    fn test_rstrip_no_whitespace() {
        let text = Text::plain("hello");
        let stripped = text.rstrip();
        assert_eq!(stripped.plain_text(), "hello");
    }

    #[test]
    fn test_rstrip_adjusts_spans() {
        let mut text = Text::plain("hello   ");
        text.stylize(0, 8, Style::new().with_bold(true));
        let stripped = text.rstrip();
        assert_eq!(stripped.spans().len(), 1);
        assert_eq!(stripped.spans()[0].end, 5); // Clamped to new length
    }

    // ==================== rstrip_end tests ====================

    #[test]
    fn test_rstrip_end_basic() {
        let text = Text::plain("hello   ");
        let stripped = text.rstrip_end(5);
        assert_eq!(stripped.plain_text(), "hello");
    }

    #[test]
    fn test_rstrip_end_partial() {
        let text = Text::plain("hello   ");
        let stripped = text.rstrip_end(7);
        // Only removes 1 trailing space (8 - 7 = 1)
        assert_eq!(stripped.plain_text(), "hello  ");
    }

    #[test]
    fn test_rstrip_end_already_short() {
        let text = Text::plain("hello");
        let stripped = text.rstrip_end(10);
        assert_eq!(stripped.plain_text(), "hello");
    }

    // ==================== truncate tests ====================

    #[test]
    fn test_truncate_crop() {
        use crate::console::OverflowMethod;
        let text = Text::plain("hello world");
        let truncated = text.truncate(5, OverflowMethod::Crop, false);
        assert_eq!(truncated.plain_text(), "hello");
    }

    #[test]
    fn test_truncate_ellipsis() {
        use crate::console::OverflowMethod;
        let text = Text::plain("hello world");
        let truncated = text.truncate(6, OverflowMethod::Ellipsis, false);
        assert_eq!(truncated.plain_text(), "hello…");
    }

    #[test]
    fn test_truncate_with_pad() {
        use crate::console::OverflowMethod;
        let text = Text::plain("hi");
        let truncated = text.truncate(5, OverflowMethod::Crop, true);
        assert_eq!(truncated.plain_text(), "hi   ");
    }

    // ==================== split tests ====================

    #[test]
    fn test_split_newlines() {
        let text = Text::plain("hello\nworld");
        let lines = text.split("\n", false, false);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].plain_text(), "hello");
        assert_eq!(lines[1].plain_text(), "world");
    }

    #[test]
    fn test_split_include_separator() {
        let text = Text::plain("hello\nworld");
        let lines = text.split("\n", true, false);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].plain_text(), "hello\n");
        assert_eq!(lines[1].plain_text(), "world");
    }

    #[test]
    fn test_split_allow_blank() {
        let text = Text::plain("hello\n");
        let lines = text.split("\n", false, true);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].plain_text(), "hello");
        assert_eq!(lines[1].plain_text(), "");
    }

    // ==================== wrap tests ====================

    #[test]
    fn test_wrap_basic() {
        let text = Text::plain("hello world test");
        let lines = text.wrap(6, None, None, 8, false);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].plain_text(), "hello ");
        assert_eq!(lines[1].plain_text(), "world ");
        assert_eq!(lines[2].plain_text(), "test");
    }

    #[test]
    fn test_wrap_existing_newlines() {
        let text = Text::plain("hello\nworld");
        let lines = text.wrap(20, None, None, 8, false);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].plain_text(), "hello");
        assert_eq!(lines[1].plain_text(), "world");
    }

    #[test]
    fn test_wrap_left_justify() {
        use crate::console::JustifyMethod;
        let text = Text::plain("hi");
        let lines = text.wrap(5, Some(JustifyMethod::Left), None, 8, false);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].plain_text(), "hi   ");
    }

    #[test]
    fn test_wrap_right_justify() {
        use crate::console::JustifyMethod;
        let text = Text::plain("hi");
        let lines = text.wrap(5, Some(JustifyMethod::Right), None, 8, false);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].plain_text(), "   hi");
    }

    #[test]
    fn test_wrap_center_justify() {
        use crate::console::JustifyMethod;
        let text = Text::plain("hi");
        let lines = text.wrap(6, Some(JustifyMethod::Center), None, 8, false);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].plain_text(), "  hi  ");
    }

    #[test]
    fn test_wrap_fold_long_word() {
        use crate::console::OverflowMethod;
        let text = Text::plain("abcdefghij");
        let lines = text.wrap(4, None, Some(OverflowMethod::Fold), 8, false);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].plain_text(), "abcd");
        assert_eq!(lines[1].plain_text(), "efgh");
        assert_eq!(lines[2].plain_text(), "ij");
    }

    #[test]
    fn test_wrap_no_wrap() {
        let text = Text::plain("hello world");
        let lines = text.wrap(5, None, None, 8, true);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].plain_text(), "hello world");
    }

    #[test]
    fn test_render_no_wrap_still_applies_justify() {
        use crate::Console;
        use crate::console::{ConsoleOptions, JustifyMethod};

        let console = Console::new();
        let options = ConsoleOptions {
            max_width: 6,
            justify: Some(JustifyMethod::Center),
            no_wrap: true,
            ..Default::default()
        };

        let text = Text::plain("hi");
        let segments = text.render(&console, &options);
        let rendered: String = segments.iter().map(|s| s.text.as_ref()).collect();
        assert_eq!(rendered, "  hi  ");
    }

    #[test]
    fn test_justify_full_distributes_extra_spaces_right_to_left() {
        // Words are "a", "b", "c" => 3 chars.
        // Width 8 means we need 5 spaces total between words.
        // Python Rich distributes extra spaces from right-to-left.
        // With 2 gaps, that yields left gap 2 spaces, right gap 3 spaces.
        let text = Text::plain("a b c");
        let justified = text.justify_full(8);
        assert_eq!(justified.plain_text(), "a  b   c");
    }

    #[test]
    fn test_wrap_with_tabs() {
        let text = Text::plain("a\tb");
        let lines = text.wrap(20, None, None, 4, false);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].plain_text(), "a   b");
    }

    #[test]
    fn test_wrap_preserves_spans() {
        let mut text = Text::plain("hello world");
        text.stylize(0, 5, Style::new().with_bold(true));
        let lines = text.wrap(6, None, None, 8, false);

        assert_eq!(lines.len(), 2);
        // First line "hello " should have the bold span
        assert!(!lines[0].spans().is_empty());
        assert_eq!(lines[0].spans()[0].style.bold, Some(true));
    }

    #[test]
    fn test_wrap_cjk() {
        let text = Text::plain("你好世界");
        // Each CJK char is 2 cells, so with width 5, we can fit 2 chars (4 cells)
        let lines = text.wrap(5, None, None, 8, false);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].plain_text(), "你好");
        assert_eq!(lines[1].plain_text(), "世界");
    }

    #[test]
    fn test_wrap_full_justify() {
        use crate::console::JustifyMethod;
        let text = Text::plain("a b c");
        let lines = text.wrap(7, Some(JustifyMethod::Full), None, 8, false);
        assert_eq!(lines.len(), 1);
        // Last line should be left-aligned, not full justified
        assert_eq!(lines[0].plain_text(), "a b c  ");
    }

    #[test]
    fn test_wrap_full_justify_multiline() {
        use crate::console::JustifyMethod;
        let text = Text::plain("a b c d e");
        let lines = text.wrap(5, Some(JustifyMethod::Full), None, 8, false);
        // Should have multiple lines, with full justification on non-last lines
        assert!(lines.len() >= 2);
    }
}
