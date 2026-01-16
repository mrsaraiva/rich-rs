//! Markup: BBCode-like markup parsing.
//!
//! Supports syntax like `[bold red]text[/]` for styling.

use crate::style::Style;
use crate::text::Text;

/// A parsed markup tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tag {
    /// Opening tag with style: `[bold red]`
    Open(Style),
    /// Closing tag: `[/]` or `[/bold]`
    Close,
}

/// Parse markup text into a Text object.
///
/// # Example
///
/// ```
/// use rich_rs::markup::render;
///
/// let text = render("[bold]Hello[/] World");
/// ```
pub fn render(markup: &str) -> Text {
    let mut text = Text::new();
    let mut style_stack: Vec<Style> = Vec::new();
    let mut current_pos = 0;

    let chars: Vec<char> = markup.chars().collect();
    let len = chars.len();

    while current_pos < len {
        if chars[current_pos] == '[' {
            // Look for closing bracket
            if let Some(close_pos) = find_closing_bracket(&chars, current_pos) {
                let tag_content: String = chars[current_pos + 1..close_pos].iter().collect();

                if tag_content.starts_with('/') {
                    // Closing tag
                    style_stack.pop();
                } else if tag_content == "" {
                    // Empty brackets, treat as literal
                    text.append("[", current_style(&style_stack));
                    text.append("]", current_style(&style_stack));
                } else {
                    // Opening tag with style
                    if let Some(style) = Style::parse(&tag_content) {
                        style_stack.push(style);
                    }
                }
                current_pos = close_pos + 1;
                continue;
            }
        }

        // Regular character
        let ch = chars[current_pos].to_string();
        text.append(ch, current_style(&style_stack));
        current_pos += 1;
    }

    text
}

fn find_closing_bracket(chars: &[char], start: usize) -> Option<usize> {
    for i in (start + 1)..chars.len() {
        if chars[i] == ']' {
            return Some(i);
        }
        if chars[i] == '[' {
            // Nested bracket, not valid
            return None;
        }
    }
    None
}

fn current_style(stack: &[Style]) -> Option<Style> {
    if stack.is_empty() {
        None
    } else {
        // Combine all styles in the stack
        let mut combined = Style::new();
        for style in stack {
            combined = combined.combine(style);
        }
        Some(combined)
    }
}

/// Escape text for safe inclusion in markup.
pub fn escape(text: &str) -> String {
    text.replace('[', "\\[").replace(']', "\\]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_plain() {
        let text = render("hello world");
        assert_eq!(text.plain_text(), "hello world");
    }

    #[test]
    fn test_render_bold() {
        let text = render("[bold]hello[/]");
        assert_eq!(text.plain_text(), "hello");
        assert!(!text.spans().is_empty());
    }

    #[test]
    fn test_escape() {
        assert_eq!(escape("hello [world]"), "hello \\[world\\]");
    }
}
