//! Justify example
//!
//! Run with: `cargo run --example justify`
//!
//! This demonstrates text justification options in rich-rs.

use rich_rs::{Console, ConsoleOptions, JustifyMethod, Style, Text};

fn main() {
    // Create a console with fixed width of 20
    let options = ConsoleOptions {
        max_width: 20,
        ..ConsoleOptions::from_terminal()
    };
    let mut console = Console::with_options(options);

    // Parse the style "bold white on blue"
    let style = Style::parse("bold white on blue").unwrap_or_default();

    // Print "Rich" with the style (default justification)
    let text = Text::styled("Rich", style);
    console.print(&text, None, None, None, false, "\n").unwrap();

    // Print with left justification
    let text = Text::styled("Rich", style);
    console
        .print(&text, None, Some(JustifyMethod::Left), None, false, "\n")
        .unwrap();

    // Print with center justification
    let text = Text::styled("Rich", style);
    console
        .print(&text, None, Some(JustifyMethod::Center), None, false, "\n")
        .unwrap();

    // Print with right justification
    let text = Text::styled("Rich", style);
    console
        .print(&text, None, Some(JustifyMethod::Right), None, false, "\n")
        .unwrap();
}
