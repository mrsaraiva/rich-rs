//! Group example
//!
//! Run with: `cargo run --example group`
//!
//! This demonstrates the Group renderable in rich-rs.

use rich_rs::{Console, Group, Panel, SimpleColor, Style, Text};

fn main() {
    let mut console = Console::new();

    // Create two panels with different styles
    let panel1 = Panel::new(Box::new(Text::plain("Hello")))
        .with_style(Style::new().with_bgcolor(SimpleColor::Standard(4))); // on blue

    let panel2 = Panel::new(Box::new(Text::plain("World")))
        .with_style(Style::new().with_bgcolor(SimpleColor::Standard(1))); // on red

    // Group the two panels together
    let panel_group = Group::new([panel1, panel2]);

    // Wrap the group in another panel
    let outer_panel = Panel::new(Box::new(panel_group));

    // Print the nested panels
    console
        .print(&outer_panel, None, None, None, false, "\n")
        .unwrap();
}
