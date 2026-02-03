//! Overflow example
//!
//! Run with: `cargo run --example overflow`
//!
//! This demonstrates text overflow handling methods in rich-rs.

use rich_rs::{Console, ConsoleOptions, OverflowMethod, Rule, Style, Text};

fn main() {
    // Create a console with fixed width of 14
    let options = ConsoleOptions {
        max_width: 14,
        ..ConsoleOptions::from_terminal()
    };
    let mut console = Console::with_options(options);

    let supercali = "supercalifragilisticexpialidocious";
    let style = Style::parse("bold blue").unwrap_or_default();

    // Test each overflow method
    let overflow_methods = [
        ("fold", OverflowMethod::Fold),
        ("crop", OverflowMethod::Crop),
        ("ellipsis", OverflowMethod::Ellipsis),
    ];

    for (name, overflow) in overflow_methods {
        // Print a rule with the overflow method name
        let rule = Rule::new().with_title(name);
        console.print(&rule, None, None, None, false, "").unwrap();

        // Print the long word with the specified overflow method
        let text = Text::styled(supercali, style);
        console
            .print(&text, None, None, Some(overflow), false, "\n")
            .unwrap();

        // Print a blank line
        console.line(1).unwrap();
    }
}
