//! Highlighter example
//!
//! Run with: `cargo run --example highlighter`
//!
//! This demonstrates a simple text highlighter that applies styles based on
//! regex patterns. It's a port of Python Rich's `examples/highlighter.py`.

use rich_rs::{Console, Highlighter, RegexHighlighter, Style, Text, Theme};

fn main() {
    // Create a custom highlighter for email addresses
    // The named capture group (?P<email>...) will be styled with "example.email"
    // (base_style + capture group name)
    let email_highlighter =
        RegexHighlighter::new(&[r"(?P<email>[\w-]+@([\w-]+\.)+[\w-]+)"], "example.");

    // Create a theme with the custom style for emails
    let mut theme = Theme::new();
    theme.add_style(
        "example.email",
        Style::parse("bold magenta").unwrap_or_default(),
    );

    // Create the highlighter with our custom theme
    let email_highlighter = email_highlighter.with_theme(theme.clone());

    // Create the console and push the theme for other styles
    let mut console = Console::new();
    console.push_theme(theme);

    // Create text and apply the highlighter
    let mut text = Text::plain("Send funds to money@example.org");
    email_highlighter.highlight(&mut text);

    // Print the highlighted text
    console.print(&text, None, None, None, false, "\n").unwrap();

    // Demonstrate highlighting multiple emails
    let mut text2 = Text::plain("Contact support@rich-rs.dev or sales@rich-rs.dev for help");
    email_highlighter.highlight(&mut text2);
    console
        .print(&text2, None, None, None, false, "\n")
        .unwrap();

    // Show that non-email text remains unstyled
    let mut text3 = Text::plain("No emails here, just plain text");
    email_highlighter.highlight(&mut text3);
    console
        .print(&text3, None, None, None, false, "\n")
        .unwrap();
}
