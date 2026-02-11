//! JSON: pretty-printed JSON renderable.
//!
//! Formats a JSON string with indentation and optional syntax highlighting.
//! Does not depend on serde_json — uses a simple recursive formatter.

use crate::Renderable;
use crate::console::{Console, ConsoleOptions};
use crate::highlighter::{Highlighter, json_highlighter};
use crate::measure::Measurement;
use crate::segment::Segments;
use crate::text::Text;

/// A renderable that pretty-prints JSON with optional syntax highlighting.
pub struct Json {
    /// The highlighted text representation.
    text: Text,
}

impl Json {
    /// Create a new Json renderable from a JSON string.
    ///
    /// The JSON string is re-formatted with the specified indentation and
    /// optionally highlighted with the JSON highlighter.
    ///
    /// # Arguments
    ///
    /// * `data` - A valid JSON string.
    /// * `indent` - Number of spaces for indentation (default 2).
    /// * `highlight` - Whether to apply syntax highlighting.
    /// * `sort_keys` - Whether to sort object keys.
    pub fn new(data: &str, indent: usize, highlight: bool, sort_keys: bool) -> Self {
        let formatted = format_json(data, indent, sort_keys);
        let mut text = Text::plain(&formatted);

        if highlight {
            let hl = json_highlighter();
            hl.highlight(&mut text);
        }

        Json { text }
    }
}

impl Renderable for Json {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
        self.text.render(console, options)
    }

    fn measure(&self, console: &Console, options: &ConsoleOptions) -> Measurement {
        self.text.measure(console, options)
    }
}

// ============================================================================
// Simple JSON formatter (no serde dependency)
// ============================================================================

/// Format a JSON string with indentation and optional key sorting.
///
/// This is a simple recursive formatter that parses and re-emits JSON
/// without requiring serde_json as a dependency.
fn format_json(input: &str, indent: usize, sort_keys: bool) -> String {
    let trimmed = input.trim();
    let chars: Vec<char> = trimmed.chars().collect();
    let mut pos = 0;
    match format_value(&chars, &mut pos, 0, indent, sort_keys) {
        Some(s) => s,
        None => input.to_string(), // Fallback: return as-is if parsing fails
    }
}

fn skip_whitespace(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && chars[*pos].is_ascii_whitespace() {
        *pos += 1;
    }
}

fn format_value(
    chars: &[char],
    pos: &mut usize,
    depth: usize,
    indent: usize,
    sort_keys: bool,
) -> Option<String> {
    skip_whitespace(chars, pos);
    if *pos >= chars.len() {
        return None;
    }

    match chars[*pos] {
        '{' => format_object(chars, pos, depth, indent, sort_keys),
        '[' => format_array(chars, pos, depth, indent, sort_keys),
        '"' => format_string(chars, pos),
        't' | 'f' => format_bool(chars, pos),
        'n' => format_null(chars, pos),
        _ => format_number(chars, pos),
    }
}

fn format_string(chars: &[char], pos: &mut usize) -> Option<String> {
    if *pos >= chars.len() || chars[*pos] != '"' {
        return None;
    }

    let mut result = String::new();
    result.push('"');
    *pos += 1; // skip opening quote

    while *pos < chars.len() {
        let c = chars[*pos];
        *pos += 1;
        match c {
            '\\' => {
                result.push('\\');
                if *pos < chars.len() {
                    result.push(chars[*pos]);
                    *pos += 1;
                }
            }
            '"' => {
                result.push('"');
                return Some(result);
            }
            _ => {
                result.push(c);
            }
        }
    }

    // Unterminated string
    result.push('"');
    Some(result)
}

fn format_number(chars: &[char], pos: &mut usize) -> Option<String> {
    let start = *pos;
    while *pos < chars.len()
        && (chars[*pos].is_ascii_digit()
            || chars[*pos] == '.'
            || chars[*pos] == '-'
            || chars[*pos] == '+'
            || chars[*pos] == 'e'
            || chars[*pos] == 'E')
    {
        *pos += 1;
    }
    if *pos == start {
        return None;
    }
    Some(chars[start..*pos].iter().collect())
}

fn format_bool(chars: &[char], pos: &mut usize) -> Option<String> {
    let remaining: String = chars[*pos..].iter().take(5).collect();
    if remaining.starts_with("true") {
        *pos += 4;
        Some("true".to_string())
    } else if remaining.starts_with("false") {
        *pos += 5;
        Some("false".to_string())
    } else {
        None
    }
}

fn format_null(chars: &[char], pos: &mut usize) -> Option<String> {
    let remaining: String = chars[*pos..].iter().take(4).collect();
    if remaining == "null" {
        *pos += 4;
        Some("null".to_string())
    } else {
        None
    }
}

fn format_object(
    chars: &[char],
    pos: &mut usize,
    depth: usize,
    indent: usize,
    sort_keys: bool,
) -> Option<String> {
    if *pos >= chars.len() || chars[*pos] != '{' {
        return None;
    }
    *pos += 1; // skip '{'

    skip_whitespace(chars, pos);

    if *pos < chars.len() && chars[*pos] == '}' {
        *pos += 1;
        return Some("{}".to_string());
    }

    let mut entries: Vec<(String, String)> = Vec::new();
    let inner_indent = " ".repeat((depth + 1) * indent);
    let outer_indent = " ".repeat(depth * indent);

    loop {
        if *pos >= chars.len() {
            break;
        }

        skip_whitespace(chars, pos);
        let key = format_string(chars, pos)?;

        skip_whitespace(chars, pos);
        if *pos < chars.len() && chars[*pos] == ':' {
            *pos += 1;
        }

        skip_whitespace(chars, pos);
        let value = format_value(chars, pos, depth + 1, indent, sort_keys)?;

        entries.push((key, value));

        skip_whitespace(chars, pos);
        if *pos < chars.len() && chars[*pos] == ',' {
            *pos += 1;
        } else {
            break;
        }
    }

    skip_whitespace(chars, pos);
    if *pos < chars.len() && chars[*pos] == '}' {
        *pos += 1;
    }

    if sort_keys {
        entries.sort_by(|a, b| a.0.cmp(&b.0));
    }

    let parts: Vec<String> = entries
        .iter()
        .map(|(k, v)| format!("{}{}: {}", inner_indent, k, v))
        .collect();

    Some(format!("{{\n{}\n{}}}", parts.join(",\n"), outer_indent))
}

fn format_array(
    chars: &[char],
    pos: &mut usize,
    depth: usize,
    indent: usize,
    sort_keys: bool,
) -> Option<String> {
    if *pos >= chars.len() || chars[*pos] != '[' {
        return None;
    }
    *pos += 1; // skip '['

    skip_whitespace(chars, pos);

    if *pos < chars.len() && chars[*pos] == ']' {
        *pos += 1;
        return Some("[]".to_string());
    }

    let mut items: Vec<String> = Vec::new();
    let inner_indent = " ".repeat((depth + 1) * indent);
    let outer_indent = " ".repeat(depth * indent);

    loop {
        if *pos >= chars.len() {
            break;
        }

        skip_whitespace(chars, pos);
        let value = format_value(chars, pos, depth + 1, indent, sort_keys)?;
        items.push(format!("{}{}", inner_indent, value));

        skip_whitespace(chars, pos);
        if *pos < chars.len() && chars[*pos] == ',' {
            *pos += 1;
        } else {
            break;
        }
    }

    skip_whitespace(chars, pos);
    if *pos < chars.len() && chars[*pos] == ']' {
        *pos += 1;
    }

    Some(format!("[\n{}\n{}]", items.join(",\n"), outer_indent))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_json_object() {
        let json = r#"{"name": "Alice", "age": 30}"#;
        let formatted = format_json(json, 2, false);
        assert!(formatted.contains("\"name\": \"Alice\""));
        assert!(formatted.contains("\"age\": 30"));
        assert!(formatted.contains('\n'));
    }

    #[test]
    fn test_format_json_array() {
        let json = r#"[1, 2, 3]"#;
        let formatted = format_json(json, 2, false);
        assert!(formatted.contains("1"));
        assert!(formatted.contains("2"));
        assert!(formatted.contains("3"));
    }

    #[test]
    fn test_format_json_nested() {
        let json = r#"{"users": [{"name": "Alice"}, {"name": "Bob"}]}"#;
        let formatted = format_json(json, 2, false);
        assert!(formatted.contains("Alice"));
        assert!(formatted.contains("Bob"));
    }

    #[test]
    fn test_format_json_sort_keys() {
        let json = r#"{"b": 2, "a": 1}"#;
        let formatted = format_json(json, 2, true);
        let a_pos = formatted.find("\"a\"").unwrap();
        let b_pos = formatted.find("\"b\"").unwrap();
        assert!(a_pos < b_pos);
    }

    #[test]
    fn test_format_json_empty_object() {
        let json = "{}";
        let formatted = format_json(json, 2, false);
        assert_eq!(formatted, "{}");
    }

    #[test]
    fn test_format_json_empty_array() {
        let json = "[]";
        let formatted = format_json(json, 2, false);
        assert_eq!(formatted, "[]");
    }

    #[test]
    fn test_format_json_booleans_and_null() {
        let json = r#"{"flag": true, "other": false, "val": null}"#;
        let formatted = format_json(json, 2, false);
        assert!(formatted.contains("true"));
        assert!(formatted.contains("false"));
        assert!(formatted.contains("null"));
    }

    #[test]
    fn test_json_renderable() {
        let json = Json::new(r#"{"name": "Alice"}"#, 2, true, false);
        let console = Console::new();
        let options = ConsoleOptions::default();
        let segments = json.render(&console, &options);
        assert!(!segments.is_empty());
    }
}
