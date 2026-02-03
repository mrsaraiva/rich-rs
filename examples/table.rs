//! Table example
//!
//! Run with: `cargo run --example table`
//!
//! This demonstrates the Table renderable, mirroring the Python Rich table example.

use rich_rs::{Align, Column, Console, JustifyMethod, Row, Style, Table, Text};

fn main() {
    // Create a table with a title
    let mut table = Table::new().with_title("Star Wars Movies");

    // Add columns with styles
    // Released column: cyan, no wrap
    table.add_column(
        Column::with_header_str("Released")
            .style(Style::parse("cyan").unwrap_or_default())
            .no_wrap(true),
    );

    // Title column: magenta
    table.add_column(
        Column::with_header_str("Title").style(Style::parse("magenta").unwrap_or_default()),
    );

    // Box Office column: right-justified, green
    table.add_column(
        Column::with_header_str("Box Office")
            .justify(JustifyMethod::Right)
            .style(Style::parse("green").unwrap_or_default()),
    );

    // Add rows
    table.add_row(Row::new(vec![
        Box::new(Text::plain("Dec 20, 2019")),
        Box::new(Text::plain("Star Wars: The Rise of Skywalker")),
        Box::new(Text::plain("$952,110,690")),
    ]));

    table.add_row(Row::new(vec![
        Box::new(Text::plain("May 25, 2018")),
        Box::new(Text::plain("Solo: A Star Wars Story")),
        Box::new(Text::plain("$393,151,347")),
    ]));

    table.add_row(Row::new(vec![
        Box::new(Text::plain("Dec 15, 2017")),
        Box::new(Text::plain("Star Wars Ep. V111: The Last Jedi")),
        Box::new(Text::plain("$1,332,539,889")),
    ]));

    table.add_row(Row::new(vec![
        Box::new(Text::plain("Dec 16, 2016")),
        Box::new(Text::plain("Rogue One: A Star Wars Story")),
        Box::new(Text::plain("$1,332,439,889")),
    ]));

    // Create console and print the table centered
    let mut console = Console::new();
    let centered = Align::center(Box::new(table));
    console
        .print(&centered, None, None, None, false, "\n")
        .unwrap();
}
