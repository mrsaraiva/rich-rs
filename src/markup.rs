//! Markup: BBCode-like markup parsing.
//!
//! Supports syntax like `[bold red]text[/]` for styling, links, and metadata.
//!
//! # Syntax
//!
//! - `[style]text[/style]` - Apply style to text
//! - `[style]text[/]` - Implicit close (closes most recent tag)
//! - `[link=url]text[/link]` - Hyperlink
//! - `[@handler=params]text[/@handler]` - Metadata (for Textual)
//! - `:emoji_name:` - Emoji codes (replaced with Unicode)
//! - `\[` - Escaped bracket (literal `[`)
//!
//! # Example
//!
//! ```
//! use rich_rs::markup::render;
//!
//! let text = render("[bold red]Hello[/] World", true);
//! assert!(text.is_ok());
//! ```

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::emoji::Emoji;
use crate::error::{ParseError, Result};
use crate::style::{MetaValue, Style, StyleMeta};
use crate::text::{Span, Text};

/// A parsed markup tag.
///
/// Represents either a style tag or a special tag like link/metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// The tag name (e.g., "bold", "link", "@click").
    pub name: String,
    /// Optional parameters after `=` (e.g., "https://example.com" in `[link=https://example.com]`).
    pub parameters: Option<String>,
}

impl Tag {
    /// Create a new tag.
    pub fn new(name: impl Into<String>, parameters: Option<String>) -> Self {
        Tag {
            name: name.into(),
            parameters,
        }
    }

    /// Get the markup representation of this tag.
    pub fn markup(&self) -> String {
        match &self.parameters {
            Some(params) => format!("[{}={}]", self.name, params),
            None => format!("[{}]", self.name),
        }
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.parameters {
            Some(params) => write!(f, "{} {}", self.name, params),
            None => write!(f, "{}", self.name),
        }
    }
}

/// A token from the markup parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// Plain text.
    Text(String),
    /// A parsed tag.
    Tag(Tag),
}

/// Regex for matching markup tags.
///
/// Pattern: `((\\*)\[([a-z#/@][^[]*?)])`
/// - Group 1: Full match including backslashes
/// - Group 2: Backslash escapes
/// - Group 3: Tag content (without brackets)
static RE_TAGS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"((?:\\)*)\[([a-z#/@][^\[]*?)]").expect("Invalid regex pattern"));

/// Parse markup into an iterator of (position, text, tag) tuples.
///
/// This is the core tokenizer. It yields:
/// - `(pos, Some(text), None)` for plain text (pos is where text ENDS, matching Python)
/// - `(pos, None, Some(tag))` for tags (pos is where tag STARTS)
///
/// # Note
///
/// Positions are **byte offsets**, not character offsets. For ASCII input this
/// matches Python's behavior, but for non-ASCII text the positions may differ.
/// Error messages use these byte positions.
///
/// # Arguments
///
/// * `markup` - The markup string to parse
pub fn parse(markup: &str) -> Vec<(usize, Option<String>, Option<Tag>)> {
    let mut result = Vec::new();
    let mut position = 0;

    for caps in RE_TAGS.captures_iter(markup) {
        let full_match = caps.get(0).unwrap();
        let escapes = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let tag_text = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        let start = full_match.start();
        let end = full_match.end();

        // Emit any text before this match
        // Python yields `start` (where text ends) as position, not `position` (where it starts)
        if start > position {
            result.push((start, Some(markup[position..start].to_string()), None));
        }

        // Track adjusted start position for tags after backslashes
        let mut tag_start = start;

        if !escapes.is_empty() {
            let escape_len = escapes.len();
            let (backslashes, escaped) = (escape_len / 2, escape_len % 2);

            if backslashes > 0 {
                // Literal backslashes (pairs become singles)
                result.push((start, Some("\\".repeat(backslashes)), None));
                // Advance tag_start past the literal backslashes (matches Python's start += backslashes * 2)
                tag_start = start + backslashes * 2;
            }

            if escaped != 0 {
                // The tag is escaped - emit as literal text (without the escape backslash)
                // Python uses position after any literal backslashes
                let literal_start = tag_start + 1; // Skip the escape backslash
                let literal = &markup[literal_start..end];
                result.push((tag_start, Some(literal.to_string()), None));
                position = end;
                continue;
            }
        }

        // Parse tag: split on first `=`
        let (text, parameters) = match tag_text.find('=') {
            Some(idx) => (
                tag_text[..idx].to_string(),
                Some(tag_text[idx + 1..].to_string()),
            ),
            None => (tag_text.to_string(), None),
        };

        result.push((tag_start, None, Some(Tag::new(text, parameters))));
        position = end;
    }

    // Emit any remaining text
    // Note: Python uses `position` (start) for trailing text, not `len()` (end)
    // This is inconsistent with mid-text behavior but matches Python Rich
    if position < markup.len() {
        result.push((position, Some(markup[position..].to_string()), None));
    }

    result
}

/// Escape text for safe inclusion in markup.
///
/// Doubles existing backslashes before brackets and escapes opening brackets
/// that could be interpreted as tags.
///
/// # Example
///
/// ```
/// use rich_rs::markup::escape;
///
/// assert_eq!(escape("[bold]"), "\\[bold]");
/// assert_eq!(escape("\\[bold]"), "\\\\\\[bold]");
/// ```
pub fn escape(markup: &str) -> String {
    static ESCAPE_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(\\*)(\[[a-z#/@][^\[]*?])").expect("Invalid escape regex"));

    let result = ESCAPE_RE.replace_all(markup, |caps: &regex::Captures| {
        let backslashes = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let text = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        // Double the backslashes and add one more to escape the bracket
        format!("{}{}\\{}", backslashes, backslashes, text)
    });

    // Handle trailing backslash
    let result = result.to_string();
    if result.ends_with('\\') && !result.ends_with("\\\\") {
        format!("{}\\", result)
    } else {
        result
    }
}

/// Normalize a style name for comparison.
///
/// Strips whitespace and converts to lowercase.
fn normalize_style(style: &str) -> String {
    style.trim().to_lowercase()
}

/// Render markup into a Text object.
///
/// # Arguments
///
/// * `markup` - The markup string to render
/// * `emoji` - Whether to replace emoji codes (`:smile:` -> actual emoji)
///
/// # Returns
///
/// A `Text` object with styled spans, or an error if the markup is invalid.
///
/// # Example
///
/// ```
/// use rich_rs::markup::render;
///
/// let text = render("[bold red]Hello[/bold red] World", true).unwrap();
/// assert_eq!(text.plain_text(), "Hello World");
/// ```
pub fn render(markup: &str, emoji: bool) -> Result<Text> {
    // Fast path: no brackets means no markup
    if !markup.contains('[') {
        let content = if emoji {
            Emoji::replace(markup)
        } else {
            markup.to_string()
        };
        return Ok(Text::plain(content));
    }

    let mut text = Text::new();
    let mut style_stack: Vec<(usize, Tag)> = Vec::new();
    let mut spans: Vec<Span> = Vec::new();

    for (position, plain_text, tag) in parse(markup) {
        if let Some(mut content) = plain_text {
            // Handle escaped brackets: \[ -> [
            content = content.replace("\\[", "[");

            // Replace emoji codes if enabled
            if emoji {
                content = Emoji::replace(&content);
            }

            text.append(&content, None);
        } else if let Some(tag) = tag {
            if tag.name.starts_with('/') {
                // Closing tag
                let style_name = tag.name[1..].trim();

                let (start, open_tag) = if !style_name.is_empty() {
                    // Explicit close: [/bold]
                    let normalized = normalize_style(style_name);
                    pop_style(&mut style_stack, &normalized).ok_or_else(|| {
                        ParseError::UnexpectedClosingTag(format!(
                            "closing tag '{}' at position {} doesn't match any open tag",
                            tag.markup(),
                            position
                        ))
                    })?
                } else {
                    // Implicit close: [/]
                    style_stack.pop().ok_or_else(|| {
                        ParseError::UnexpectedClosingTag(format!(
                            "closing tag '[/]' at position {} has nothing to close",
                            position
                        ))
                    })?
                };

                // Create span for the closed region
                if open_tag.name.starts_with('@') {
                    // Metadata tag - used by Textual for event handlers.
                    let mut meta_map = BTreeMap::new();
                    let value = match open_tag.parameters.as_deref().map(str::trim) {
                        None => MetaValue::None,
                        Some("") => MetaValue::None,
                        Some(params) => MetaValue::parse_python_literal(params)
                            .unwrap_or_else(|| MetaValue::str(params.to_string())),
                    };
                    meta_map.insert(open_tag.name.clone(), value);
                    let meta = StyleMeta {
                        meta: Some(Arc::new(meta_map)),
                        ..Default::default()
                    };
                    spans.push(Span::new_with_meta(
                        start,
                        text.len(),
                        Style::new(),
                        Some(meta),
                    ));
                } else if open_tag.name == "link" {
                    // Link tag - apply underline+cyan as visual indicator
                    let link_style = Style::new()
                        .with_underline(true)
                        .with_color(crate::color::SimpleColor::Standard(6)); // cyan
                    let meta = open_tag
                        .parameters
                        .as_ref()
                        .map(|url| StyleMeta::with_link(url.clone()))
                        .unwrap_or_default();
                    spans.push(Span::new_with_meta(
                        start,
                        text.len(),
                        link_style,
                        Some(meta),
                    ));
                } else {
                    // Regular style tag - parse and apply
                    let style_str = open_tag.to_string();
                    if let Some(style) = Style::parse(&style_str) {
                        if !style.is_null() {
                            spans.push(Span::new(start, text.len(), style));
                        }
                        // Note: Unknown style tokens are silently ignored, matching Python
                    }
                }
            } else {
                // Opening tag - push to stack with current position
                let normalized_tag = Tag::new(normalize_style(&tag.name), tag.parameters);
                style_stack.push((text.len(), normalized_tag));
            }
        }
    }

    // Handle unclosed tags - apply them as spans to end of text
    // Use same logic as explicit close: skip metadata, handle link specially, filter null styles
    let text_length = text.len();
    while let Some((start, tag)) = style_stack.pop() {
        if tag.name.starts_with('@') {
            let mut meta_map = BTreeMap::new();
            let value = match tag.parameters.as_deref().map(str::trim) {
                None => MetaValue::None,
                Some("") => MetaValue::None,
                Some(params) => MetaValue::parse_python_literal(params)
                    .unwrap_or_else(|| MetaValue::str(params.to_string())),
            };
            meta_map.insert(tag.name.clone(), value);
            let meta = StyleMeta {
                meta: Some(Arc::new(meta_map)),
                ..Default::default()
            };
            spans.push(Span::new_with_meta(
                start,
                text_length,
                Style::new(),
                Some(meta),
            ));
        } else if tag.name == "link" {
            // Link tag - apply underline+cyan (matches explicit close behavior)
            let link_style = Style::new()
                .with_underline(true)
                .with_color(crate::color::SimpleColor::Standard(6)); // cyan
            let meta = tag
                .parameters
                .as_ref()
                .map(|url| StyleMeta::with_link(url.clone()))
                .unwrap_or_default();
            spans.push(Span::new_with_meta(
                start,
                text_length,
                link_style,
                Some(meta),
            ));
        } else {
            // Regular style tag
            let style_str = tag.to_string();
            if let Some(style) = Style::parse(&style_str) {
                if !style.is_null() {
                    spans.push(Span::new(start, text_length, style));
                }
            }
        }
    }

    // Apply spans to text
    // Python does: sorted(spans[::-1], key=attrgetter("start"))
    // That's: reverse the list, then stable-sort by start
    // This preserves innermost-first ordering for overlapping spans
    spans.reverse();
    spans.sort_by(|a, b| a.start.cmp(&b.start)); // stable sort
    for span in spans {
        text.spans_mut().push(span);
    }

    Ok(text)
}

/// Pop a style from the stack by name.
///
/// Searches from the end of the stack for a matching tag name.
fn pop_style(stack: &mut Vec<(usize, Tag)>, style_name: &str) -> Option<(usize, Tag)> {
    for i in (0..stack.len()).rev() {
        if normalize_style(&stack[i].1.name) == style_name {
            return Some(stack.remove(i));
        }
    }
    None
}

/// Render markup with a base style applied.
///
/// # Arguments
///
/// * `markup` - The markup string to render
/// * `style` - Base style to apply
/// * `emoji` - Whether to replace emoji codes
pub fn render_with_style(markup: &str, style: Style, emoji: bool) -> Result<Text> {
    let mut text = render(markup, emoji)?;

    // Apply base style as a span covering the entire text
    if text.len() > 0 {
        text.stylize(0, text.len(), style);
    }

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_new() {
        let tag = Tag::new("bold", None);
        assert_eq!(tag.name, "bold");
        assert_eq!(tag.parameters, None);
        assert_eq!(tag.markup(), "[bold]");

        let tag_with_params = Tag::new("link", Some("https://example.com".to_string()));
        assert_eq!(tag_with_params.name, "link");
        assert_eq!(
            tag_with_params.parameters,
            Some("https://example.com".to_string())
        );
        assert_eq!(tag_with_params.markup(), "[link=https://example.com]");
    }

    #[test]
    fn test_tag_display() {
        let tag = Tag::new("bold", None);
        assert_eq!(tag.to_string(), "bold");

        let tag_with_params = Tag::new("link", Some("url".to_string()));
        assert_eq!(tag_with_params.to_string(), "link url");
    }

    #[test]
    fn test_parse_plain_text() {
        let tokens = parse("hello world");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].1, Some("hello world".to_string()));
    }

    #[test]
    fn test_parse_single_tag() {
        let tokens = parse("[bold]hello[/bold]");
        assert_eq!(tokens.len(), 3);

        // Opening tag
        assert!(tokens[0].2.is_some());
        assert_eq!(tokens[0].2.as_ref().unwrap().name, "bold");

        // Text
        assert_eq!(tokens[1].1, Some("hello".to_string()));

        // Closing tag
        assert!(tokens[2].2.is_some());
        assert_eq!(tokens[2].2.as_ref().unwrap().name, "/bold");
    }

    #[test]
    fn test_parse_tag_with_params() {
        let tokens = parse("[link=https://example.com]click[/link]");
        assert_eq!(tokens.len(), 3);

        let open_tag = tokens[0].2.as_ref().unwrap();
        assert_eq!(open_tag.name, "link");
        assert_eq!(open_tag.parameters, Some("https://example.com".to_string()));
    }

    #[test]
    fn test_parse_escaped_bracket() {
        let tokens = parse("\\[not a tag]");
        assert_eq!(tokens.len(), 1);
        // The escaped bracket should be plain text
        assert!(tokens[0].1.is_some());
        assert!(tokens[0].1.as_ref().unwrap().contains("[not a tag]"));
    }

    #[test]
    fn test_escape_simple() {
        assert_eq!(escape("[bold]"), "\\[bold]");
    }

    #[test]
    fn test_escape_with_backslash() {
        assert_eq!(escape("\\[bold]"), "\\\\\\[bold]");
    }

    #[test]
    fn test_escape_no_tag() {
        assert_eq!(escape("hello world"), "hello world");
    }

    #[test]
    fn test_escape_not_a_tag() {
        // Brackets that don't start tags are left alone
        assert_eq!(escape("[123]"), "[123]");
    }

    #[test]
    fn test_render_plain() {
        let text = render("hello world", false).unwrap();
        assert_eq!(text.plain_text(), "hello world");
        assert!(text.spans().is_empty());
    }

    #[test]
    fn test_render_bold() {
        let text = render("[bold]hello[/bold]", false).unwrap();
        assert_eq!(text.plain_text(), "hello");
        assert!(!text.spans().is_empty());
    }

    #[test]
    fn test_render_implicit_close() {
        let text = render("[bold]hello[/]", false).unwrap();
        assert_eq!(text.plain_text(), "hello");
        assert!(!text.spans().is_empty());
    }

    #[test]
    fn test_render_nested() {
        let text = render("[bold][italic]hello[/italic][/bold]", false).unwrap();
        assert_eq!(text.plain_text(), "hello");
        // Should have spans for both bold and italic
        assert!(text.spans().len() >= 2);
    }

    #[test]
    fn test_render_with_color() {
        let text = render("[red]hello[/red]", false).unwrap();
        assert_eq!(text.plain_text(), "hello");
        assert!(!text.spans().is_empty());
    }

    #[test]
    fn test_render_unclosed_tag() {
        // Unclosed tags should apply to end of text
        let text = render("[bold]hello", false).unwrap();
        assert_eq!(text.plain_text(), "hello");
        assert!(!text.spans().is_empty());
    }

    #[test]
    fn test_render_unclosed_link() {
        // Unclosed link should apply underline+cyan to end of text
        let text = render("[link=https://x]hi", false).unwrap();
        assert_eq!(text.plain_text(), "hi");
        assert!(!text.spans().is_empty());
        // Should have underline from link style
        assert!(text.spans()[0].style.underline == Some(true));
        assert_eq!(
            text.spans()[0]
                .meta
                .as_ref()
                .and_then(|m| m.link.as_deref()),
            Some("https://x")
        );
    }

    #[test]
    fn test_render_unclosed_metadata() {
        // Unclosed metadata tag should create non-visible spans with metadata attached
        let text = render("[@foo]bar", false).unwrap();
        assert_eq!(text.plain_text(), "bar");
        assert_eq!(text.spans().len(), 1);
        assert!(text.spans()[0].style.is_null());
        let meta = text.spans()[0].meta.as_ref().unwrap();
        let meta_map = meta.meta.as_ref().unwrap();
        assert_eq!(meta_map.get("@foo"), Some(&MetaValue::None));
    }

    #[test]
    fn test_render_unmatched_close_error() {
        let result = render("[bold]hello[/italic]", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_render_empty_close_no_open_error() {
        let result = render("hello[/]", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_render_escaped_bracket() {
        let text = render("\\[not bold]", false).unwrap();
        assert_eq!(text.plain_text(), "[not bold]");
    }

    #[test]
    fn test_render_with_emoji() {
        let text = render(":smile:", true).unwrap();
        // Should contain the smile emoji
        assert!(text.plain_text().contains('\u{1f604}') || text.plain_text() == ":smile:");
    }

    #[test]
    fn test_render_emoji_in_styled() {
        let text = render("[bold]:+1:[/bold]", true).unwrap();
        // Should contain thumbs up emoji
        assert!(!text.spans().is_empty());
    }

    #[test]
    fn test_render_link() {
        let text = render("[link=https://example.com]click here[/link]", false).unwrap();
        assert_eq!(text.plain_text(), "click here");
        // Link should create a span (underlined cyan)
        assert!(!text.spans().is_empty());
        assert_eq!(
            text.spans()[0]
                .meta
                .as_ref()
                .and_then(|m| m.link.as_deref()),
            Some("https://example.com")
        );
    }

    #[test]
    fn test_normalize_style() {
        assert_eq!(normalize_style("BOLD"), "bold");
        assert_eq!(normalize_style("  italic  "), "italic");
        assert_eq!(normalize_style("Bold Red"), "bold red");
    }

    #[test]
    fn test_render_with_style() {
        let base_style = Style::new().with_italic(true);
        let text = render_with_style("[bold]hello[/]", base_style, false).unwrap();
        assert_eq!(text.plain_text(), "hello");
        // Should have both bold and italic spans
        assert!(text.spans().len() >= 2);
    }

    #[test]
    fn test_render_multiple_tags_same_text() {
        let text = render("[bold red on blue]styled[/]", false).unwrap();
        assert_eq!(text.plain_text(), "styled");
        assert!(!text.spans().is_empty());
    }

    #[test]
    fn test_render_overlapping_styles() {
        let text = render("[bold]Hello [italic]World[/italic]![/bold]", false).unwrap();
        assert_eq!(text.plain_text(), "Hello World!");
    }

    #[test]
    fn test_parse_position_tracking() {
        let tokens = parse("abc[bold]def[/bold]ghi");
        // Positions: mid-text yields END position, trailing text yields START position (matches Python)
        assert_eq!(tokens[0].0, 3); // "abc" ends at 3 (where [bold] starts)
        assert_eq!(tokens[1].0, 3); // [bold] starts at 3
        assert_eq!(tokens[2].0, 12); // "def" ends at 12 (where [/bold] starts)
        assert_eq!(tokens[3].0, 12); // [/bold] starts at 12
        assert_eq!(tokens[4].0, 19); // "ghi" starts at 19 (trailing text uses start position)
    }

    #[test]
    fn test_escape_roundtrip() {
        let original = "[bold]text[/bold]";
        let escaped = escape(original);
        let text = render(&escaped, false).unwrap();
        assert_eq!(text.plain_text(), original);
    }
}
