//! Padding example
//!
//! Run with: `cargo run --example padding`
//!
//! This demonstrates the Padding renderable in rich-rs.

use rich_rs::{Console, Padding, SimpleColor, Style, Text};

fn main() {
    let mut console = Console::new();

    // Create a Text object with "Hello"
    let text = Text::plain("Hello");

    // Create padding with (2, 4) = 2 lines top/bottom, 4 spaces left/right
    // with style "on blue" and expand=false
    let padded = Padding::new(Box::new(text), (2, 4))
        .with_style(Style::new().with_bgcolor(SimpleColor::Standard(4))) // blue background
        .with_expand(false);

    // Print the padded text
    console.print(&padded, None, None, None, false, "").unwrap();
}
