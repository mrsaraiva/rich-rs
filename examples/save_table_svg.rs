//! Save Table SVG example
//!
//! Run with: `cargo run --example save_table_svg`
//!
//! Demonstrates how to export a table to SVG format.
//! This is a Rust port of Python Rich's `save_table_svg.py` example.

use rich_rs::{Align, Column, Console, JustifyMethod, Row, Style, Table, Text};

fn main() {
    // Create the Star Wars movies table
    let mut table = Table::new().with_title("Star Wars Movies");

    // Add columns with styles
    table.add_column(
        Column::with_header_str("Released")
            .style(Style::parse("cyan").unwrap_or_default())
            .no_wrap(true),
    );

    table.add_column(
        Column::with_header_str("Title").style(Style::parse("magenta").unwrap_or_default()),
    );

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

    // Create console with recording enabled
    let mut console = Console::new_with_record();

    // Print the table centered (like Python example uses justify="center")
    let centered = Align::center(Box::new(table));
    console
        .print(&centered, None, None, None, false, "\n")
        .unwrap();

    // Save to SVG file
    console
        .save_svg("table.svg", "save_table_svg.rs", None, true, 0.61, None)
        .expect("Failed to save SVG file");

    println!("Table saved to table.svg");

    // Note: Unlike Python, Rust doesn't have a built-in webbrowser module,
    // but you can open the file manually or use the `open` crate if needed.
    // For example: open::that("table.svg").ok();
}
