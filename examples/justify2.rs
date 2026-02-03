//! Justify2 example
//!
//! Run with: `cargo run --example justify2`
//!
//! This demonstrates the justify argument to print, showing how panels
//! can be positioned within the available width.
//!
//! Port of Python Rich's `examples/justify2.py`.

use rich_rs::{Console, ConsoleOptions, JustifyMethod, Panel, Style, Text};

fn main() {
    // Create a console with fixed width of 20
    let options = ConsoleOptions {
        max_width: 20,
        ..ConsoleOptions::from_terminal()
    };
    let mut console = Console::with_options(options);

    // Create a panel with "Rich" as content, styled with red background
    // expand=false means the panel fits its content rather than expanding
    let panel_style = Style::parse("on red").unwrap_or_default();

    // The outer style is "bold white on blue"
    let outer_style = Style::parse("bold white on blue").unwrap_or_default();

    // Print panel with default justification
    let panel = Panel::fit(Box::new(Text::plain("Rich"))).with_style(panel_style);
    console
        .print(&panel, Some(outer_style), None, None, false, "\n")
        .unwrap();

    // Print panel with left justification
    let panel = Panel::fit(Box::new(Text::plain("Rich"))).with_style(panel_style);
    console
        .print(
            &panel,
            Some(outer_style),
            Some(JustifyMethod::Left),
            None,
            false,
            "\n",
        )
        .unwrap();

    // Print panel with center justification
    let panel = Panel::fit(Box::new(Text::plain("Rich"))).with_style(panel_style);
    console
        .print(
            &panel,
            Some(outer_style),
            Some(JustifyMethod::Center),
            None,
            false,
            "\n",
        )
        .unwrap();

    // Print panel with right justification
    let panel = Panel::fit(Box::new(Text::plain("Rich"))).with_style(panel_style);
    console
        .print(
            &panel,
            Some(outer_style),
            Some(JustifyMethod::Right),
            None,
            false,
            "\n",
        )
        .unwrap();
}
