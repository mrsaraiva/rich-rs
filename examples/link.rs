//! Link example
//!
//! Run with: `cargo run --example link`
//!
//! This demonstrates hyperlinks in markup in rich-rs.

use rich_rs::{Console, Text};

fn main() {
    let mut console = Console::new();

    // Print introductory message
    let intro =
        Text::plain("If your terminal supports links, the following text should be clickable:");
    console
        .print(&intro, None, None, None, false, "\n")
        .unwrap();

    // Print styled text with a hyperlink using markup
    // [link=URL]text[/] creates a clickable hyperlink
    let linked_text = Text::from_markup(
        "[link=https://www.willmcgugan.com][i]Visit [red]my[/red][/i] [yellow]Blog[/]",
        false,
    )
    .unwrap();

    console
        .print(&linked_text, None, None, None, false, "\n")
        .unwrap();
}
