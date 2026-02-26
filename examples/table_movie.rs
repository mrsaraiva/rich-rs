//! Table movie example
//!
//! Run with: `cargo run --example table_movie`
//!
//! This is a Rust port of Python Rich's `table_movie.py` example.
//! It demonstrates Live display with a dynamically modified Table.
//!
//! Note: Unlike Python which shares mutable references, Rust requires us to
//! rebuild the renderable hierarchy for each update. We use Arc<Mutex<>> to
//! share the table state between the main thread and the Live display.

use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use rich_rs::r#box::{MINIMAL, SIMPLE, SIMPLE_HEAD, SQUARE};
use rich_rs::{
    Align, Column, Console, ConsoleOptions, JustifyMethod, Live, LiveOptions, Measurement,
    Renderable, Segments, Style, Table, Text,
};

/// Table data: Star Wars box office information.
const TABLE_DATA: &[(&str, &str, &str, &str, &str)] = &[
    (
        "May 25, 1977",
        "Star Wars Ep. [b]IV[/]: [i]A New Hope",
        "$11,000,000",
        "$1,554,475",
        "$775,398,007",
    ),
    (
        "May 21, 1980",
        "Star Wars Ep. [b]V[/]: [i]The Empire Strikes Back",
        "$23,000,000",
        "$4,910,483",
        "$547,969,004",
    ),
    (
        "May 25, 1983",
        "Star Wars Ep. [b]VI[/b]: [i]Return of the Jedi",
        "$32,500,000",
        "$23,019,618",
        "$475,106,177",
    ),
    (
        "May 19, 1999",
        "Star Wars Ep. [b]I[/b]: [i]The phantom Menace",
        "$115,000,000",
        "$64,810,870",
        "$1,027,044,677",
    ),
    (
        "May 16, 2002",
        "Star Wars Ep. [b]II[/b]: [i]Attack of the Clones",
        "$115,000,000",
        "$80,027,814",
        "$656,695,615",
    ),
    (
        "May 19, 2005",
        "Star Wars Ep. [b]III[/b]: [i]Revenge of the Sith",
        "$115,500,000",
        "$380,270,577",
        "$848,998,877",
    ),
];

/// Beat time in milliseconds (Python uses 0.04 seconds = 40ms).
const BEAT_TIME: u64 = 40;

/// Pause for a given number of beats.
fn beat(count: u64) {
    sleep(Duration::from_millis(count * BEAT_TIME));
}

/// A wrapper that holds an Arc<Mutex<Table>> and renders it centered.
/// This allows the table to be mutated while the Live display holds a reference.
struct SharedTable {
    table: Arc<Mutex<Table>>,
}

struct TableProxy {
    table: Arc<Mutex<Table>>,
}

impl Renderable for TableProxy {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
        let table = self.table.lock().unwrap();
        table.render(console, options)
    }

    fn measure(&self, console: &Console, options: &ConsoleOptions) -> Measurement {
        let table = self.table.lock().unwrap();
        table.measure(console, options)
    }
}

impl SharedTable {
    fn new(table: Table) -> Self {
        Self {
            table: Arc::new(Mutex::new(table)),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Table> {
        self.table.lock().unwrap()
    }

    /// Measure the table.
    fn measure_table(&self, console: &Console, options: &ConsoleOptions) -> Measurement {
        let table = self.table.lock().unwrap();
        table.measure(console, options)
    }
}

impl Renderable for SharedTable {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
        let proxy = TableProxy {
            table: Arc::clone(&self.table),
        };
        let centered = Align::center(Box::new(proxy));
        centered.render(console, options)
    }

    fn measure(&self, console: &Console, options: &ConsoleOptions) -> Measurement {
        let proxy = TableProxy {
            table: Arc::clone(&self.table),
        };
        let centered = Align::center(Box::new(proxy));
        centered.measure(console, options)
    }
}

fn main() {
    // Create an empty table with show_footer=false
    let table = Table::new().with_show_footer(false);

    // Wrap in SharedTable for thread-safe access
    let shared_table = Arc::new(SharedTable::new(table));

    // Create console and clear screen
    let mut console = Console::new();
    let _ = console.clear();

    // Create a clone for the Live display
    let display_table = Arc::clone(&shared_table);

    // Create Live display with options
    // We use a simple wrapper that just calls render on SharedTable
    struct TableRenderer {
        table: Arc<SharedTable>,
    }

    impl Renderable for TableRenderer {
        fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
            self.table.render(console, options)
        }

        fn measure(&self, console: &Console, options: &ConsoleOptions) -> Measurement {
            self.table.measure(console, options)
        }
    }

    let renderer = TableRenderer {
        table: display_table,
    };

    let mut live = Live::with_options(
        Box::new(renderer),
        LiveOptions {
            screen: false,
            refresh_per_second: 20.0,
            auto_refresh: false,
            ..Default::default()
        },
    );

    // Start the live display
    live.start(true).unwrap();

    // Helper to refresh the display
    let refresh = || {
        let _ = live.refresh();
    };

    // Add columns one by one
    beat(10);
    {
        let mut table = shared_table.lock();
        table.add_column(Column::with_header_str("Release Date").no_wrap(true));
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        table.add_column(
            Column::with_header_str("Title")
                .with_footer(Box::new(Text::from_markup("[b]Total", false).unwrap())),
        );
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        table.add_column(
            Column::with_header_str("Budget")
                .with_footer(Box::new(
                    Text::from_markup("[u]$412,000,000", false).unwrap(),
                ))
                .no_wrap(true),
        );
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        table.add_column(
            Column::with_header_str("Opening Weekend")
                .with_footer(Box::new(
                    Text::from_markup("[u]$577,703,455", false).unwrap(),
                ))
                .no_wrap(true),
        );
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        table.add_column(
            Column::with_header_str("Box Office")
                .with_footer(Box::new(
                    Text::from_markup("[u]$4,331,212,357", false).unwrap(),
                ))
                .no_wrap(true),
        );
    }
    refresh();

    // Set title
    beat(10);
    {
        let mut table = shared_table.lock();
        table.set_title(Some(Text::plain("Star Wars Box Office")));
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        table.set_title(Some(
            Text::from_markup(
                "[not italic]:popcorn:[/] Star Wars Box Office [not italic]:popcorn:[/]",
                true,
            )
            .unwrap(),
        ));
    }
    refresh();

    // Set caption
    beat(10);
    {
        let mut table = shared_table.lock();
        table.set_caption(Some(Text::plain("Made with Rich-rs")));
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        table.set_caption(Some(
            Text::from_markup("Made with [b]Rich-rs[/b]", false).unwrap(),
        ));
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        table.set_caption(Some(
            Text::from_markup("Made with [b magenta not dim]Rich-rs[/]", false).unwrap(),
        ));
    }
    refresh();

    // Add data rows
    for (date, title, budget, opening, box_office) in TABLE_DATA {
        beat(10);
        let title_text = Text::from_markup(title, false).unwrap_or_else(|_| Text::plain(*title));
        {
            let mut table = shared_table.lock();
            table.add_row(rich_rs::Row::new(vec![
                Box::new(Text::plain(*date)),
                Box::new(title_text),
                Box::new(Text::plain(*budget)),
                Box::new(Text::plain(*opening)),
                Box::new(Text::plain(*box_office)),
            ]));
        }
        refresh();
    }

    // Show footer
    beat(10);
    {
        let mut table = shared_table.lock();
        table.set_show_footer(true);
    }
    refresh();

    // Set column justifications
    beat(10);
    {
        let mut table = shared_table.lock();
        if let Some(col) = table.column_mut(2) {
            col.set_justify(JustifyMethod::Right);
        }
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        if let Some(col) = table.column_mut(3) {
            col.set_justify(JustifyMethod::Right);
        }
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        if let Some(col) = table.column_mut(4) {
            col.set_justify(JustifyMethod::Right);
        }
    }
    refresh();

    // Set header styles
    beat(10);
    {
        let mut table = shared_table.lock();
        if let Some(col) = table.column_mut(2) {
            col.set_header_style(Style::parse("bold red").unwrap_or_default());
        }
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        if let Some(col) = table.column_mut(3) {
            col.set_header_style(Style::parse("bold green").unwrap_or_default());
        }
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        if let Some(col) = table.column_mut(4) {
            col.set_header_style(Style::parse("bold blue").unwrap_or_default());
        }
    }
    refresh();

    // Set column styles
    beat(10);
    {
        let mut table = shared_table.lock();
        if let Some(col) = table.column_mut(2) {
            col.set_style(Style::parse("red").unwrap_or_default());
        }
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        if let Some(col) = table.column_mut(3) {
            col.set_style(Style::parse("green").unwrap_or_default());
        }
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        if let Some(col) = table.column_mut(4) {
            col.set_style(Style::parse("blue").unwrap_or_default());
        }
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        if let Some(col) = table.column_mut(0) {
            col.set_style(Style::parse("cyan").unwrap_or_default());
            col.set_header_style(Style::parse("bold cyan").unwrap_or_default());
        }
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        if let Some(col) = table.column_mut(1) {
            col.set_style(Style::parse("magenta").unwrap_or_default());
            col.set_header_style(Style::parse("bold magenta").unwrap_or_default());
        }
    }
    refresh();

    // Set footer styles
    beat(10);
    {
        let mut table = shared_table.lock();
        if let Some(col) = table.column_mut(2) {
            col.set_footer_style(Style::parse("bright_red").unwrap_or_default());
        }
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        if let Some(col) = table.column_mut(3) {
            col.set_footer_style(Style::parse("bright_green").unwrap_or_default());
        }
    }
    refresh();

    beat(10);
    {
        let mut table = shared_table.lock();
        if let Some(col) = table.column_mut(4) {
            col.set_footer_style(Style::parse("bright_blue").unwrap_or_default());
        }
    }
    refresh();

    // Set row styles for alternating effect
    beat(10);
    {
        let mut table = shared_table.lock();
        table.set_row_styles(vec![Style::default(), Style::new().with_dim(true)]);
    }
    refresh();

    // Set border style
    beat(10);
    {
        let mut table = shared_table.lock();
        table.set_border_style(Style::parse("bright_yellow").unwrap_or_default());
    }
    refresh();

    // Cycle through box styles
    for box_style in [SQUARE, MINIMAL, SIMPLE, SIMPLE_HEAD] {
        beat(10);
        {
            let mut table = shared_table.lock();
            table.set_box(Some(box_style));
        }
        refresh();
    }

    // Toggle pad_edge
    beat(10);
    {
        let mut table = shared_table.lock();
        table.set_pad_edge(false);
    }
    refresh();

    // Get original width for animation
    let original_width = shared_table
        .measure_table(&console, console.options())
        .maximum;
    let console_width = console.options().max_width;

    // Animate width expansion to console width
    let mut width = original_width;
    while width < console_width {
        beat(1);
        {
            let mut table = shared_table.lock();
            table.set_width(Some(width));
        }
        refresh();
        width += 2;
    }

    // Animate width contraction back to original
    let mut width = console_width;
    while width > original_width {
        beat(1);
        {
            let mut table = shared_table.lock();
            table.set_width(Some(width));
        }
        refresh();
        width = width.saturating_sub(2);
    }

    // Animate width contraction to 90
    let mut width = original_width;
    while width > 90 {
        beat(1);
        {
            let mut table = shared_table.lock();
            table.set_width(Some(width));
        }
        refresh();
        width = width.saturating_sub(2);
    }

    // Animate width expansion back to original
    let mut width = 90;
    while width <= original_width {
        beat(1);
        {
            let mut table = shared_table.lock();
            table.set_width(Some(width));
        }
        refresh();
        width += 2;
    }

    // Reset to auto width
    beat(2);
    {
        let mut table = shared_table.lock();
        table.set_width(None);
    }
    refresh();

    // Stop the live display
    live.stop().unwrap();
}
