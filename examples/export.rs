//! Export example
//!
//! Run with: `cargo run --example export`
//!
//! Demonstrates exporting console output to HTML and plain text files.
//! This is a Rust port of Python Rich's `export.py` example.

use rich_rs::{Align, Column, Console, JustifyMethod, Row, Style, Table, Text};

fn main() {
    // Create console with recording enabled
    let mut console = Console::new_with_record();

    // Helper function to create the Star Wars table
    fn create_table() -> Table {
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

        table
    }

    // Print table to console (this records it)
    let table = create_table();
    console
        .print(&table, None, None, None, false, "\n")
        .unwrap();

    // Export as HTML (clear=false to keep buffer for next export)
    let file1 = "table_export.html";
    let html = console.export_html(None, false, None);
    std::fs::write(file1, &html).expect("Failed to write HTML file");
    println!("Exported console output as HTML to {}", file1);

    // Export as SVG
    let file2 = "table_export.svg";
    let svg = console.export_svg("Star Wars Movies", None, true, None, 0.61, None);
    std::fs::write(file2, &svg).expect("Failed to write SVG file");
    println!("Exported console output as SVG to {}", file2);

    // Print table again (since buffer was cleared by SVG export)
    let table = create_table();
    console
        .print(&table, None, None, None, false, "\n")
        .unwrap();

    // Use save_html convenience method
    let file3 = "table_export2.html";
    console
        .save_html(file3, None, true, None)
        .expect("Failed to save HTML file");
    println!("Exported console output as HTML to {}", file3);

    // Print table again with centering
    let table = create_table();
    let centered = Align::center(Box::new(table));
    console
        .print(&centered, None, None, None, false, "\n")
        .unwrap();

    // Use save_svg convenience method
    let file4 = "table_export2.svg";
    console
        .save_svg(file4, "Star Wars Movies (Centered)", None, true, 0.61, None)
        .expect("Failed to save SVG file");
    println!("Exported console output as SVG to {}", file4);
}
