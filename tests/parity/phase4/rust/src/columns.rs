//! Columns module parity tests.

use rich_rs::columns::Columns;
use rich_rs::text::Text;
use rich_rs::{Console, ConsoleOptions, Renderable};

/// Helper to render columns to plain text
fn render_columns(columns: &Columns, width: usize) -> String {
    let console = Console::with_options(ConsoleOptions {
        max_width: width,
        ..Default::default()
    });
    let options = console.options().clone();
    let segments = columns.render(&console, &options);
    segments.iter().map(|s| s.text.to_string()).collect()
}

pub fn run() {
    println!("=== Simple columns ===");

    let items: Vec<Box<dyn Renderable + Send + Sync>> = vec![
        Box::new(Text::plain("apple")),
        Box::new(Text::plain("banana")),
        Box::new(Text::plain("cherry")),
        Box::new(Text::plain("date")),
        Box::new(Text::plain("elderberry")),
        Box::new(Text::plain("fig")),
    ];
    let columns = Columns::new(items);
    let output = render_columns(&columns, 40);
    let lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();
    println!("Columns(6 items) at width=40: lines={}", lines.len());
    let all_present = ["apple", "banana", "cherry", "date", "elderberry", "fig"]
        .iter()
        .all(|item| output.contains(item));
    println!("  all items present: {}", all_present);

    println!("\n=== Columns with expand ===");

    let items: Vec<Box<dyn Renderable + Send + Sync>> = vec![
        Box::new(Text::plain("A")),
        Box::new(Text::plain("B")),
        Box::new(Text::plain("C")),
        Box::new(Text::plain("D")),
    ];
    let columns = Columns::new(items).with_expand(true);
    let output = render_columns(&columns, 40);
    let lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();
    if !lines.is_empty() {
        let max_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        println!(
            "Columns(expand=True) at width=40: max_line_len={}",
            max_len
        );
    }

    println!("\n=== Columns with equal ===");

    let items: Vec<Box<dyn Renderable + Send + Sync>> = vec![
        Box::new(Text::plain("Short")),
        Box::new(Text::plain("Much Longer Text")),
        Box::new(Text::plain("X")),
    ];
    let columns = Columns::new(items).with_equal(true);
    let output = render_columns(&columns, 60);
    let lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();
    println!("Columns(equal=True) lines={}", lines.len());
    let all_present = ["Short", "Much Longer Text", "X"]
        .iter()
        .all(|item| output.contains(item));
    println!("  all items present: {}", all_present);

    println!("\n=== Columns with column_first ===");

    // Use width=8 to force multiple rows with 7 items
    let items_normal: Vec<Box<dyn Renderable + Send + Sync>> = (1..=7)
        .map(|i| Box::new(Text::plain(&i.to_string())) as Box<dyn Renderable + Send + Sync>)
        .collect();
    let columns_normal = Columns::new(items_normal);

    let items_cf: Vec<Box<dyn Renderable + Send + Sync>> = (1..=7)
        .map(|i| Box::new(Text::plain(&i.to_string())) as Box<dyn Renderable + Send + Sync>)
        .collect();
    let columns_cf = Columns::new(items_cf).with_column_first(true);

    let output_normal = render_columns(&columns_normal, 8);
    let output_cf = render_columns(&columns_cf, 8);
    let same = output_normal == output_cf;
    println!("Columns(column_first=True) differs from normal: {}", !same);
    // Show actual layout
    println!("  normal row 0: {:?}", output_normal.lines().next().unwrap_or(""));
    println!("  cf row 0: {:?}", output_cf.lines().next().unwrap_or(""));

    println!("\n=== Columns with right_to_left ===");

    let items_ltr: Vec<Box<dyn Renderable + Send + Sync>> = vec![
        Box::new(Text::plain("A")),
        Box::new(Text::plain("B")),
        Box::new(Text::plain("C")),
    ];
    let columns_ltr = Columns::new(items_ltr);

    let items_rtl: Vec<Box<dyn Renderable + Send + Sync>> = vec![
        Box::new(Text::plain("A")),
        Box::new(Text::plain("B")),
        Box::new(Text::plain("C")),
    ];
    let columns_rtl = Columns::new(items_rtl).with_right_to_left(true);

    let output_ltr = render_columns(&columns_ltr, 30);
    let output_rtl = render_columns(&columns_rtl, 30);
    let same = output_ltr == output_rtl;
    println!(
        "Columns(right_to_left=True) differs from normal: {}",
        !same
    );

    println!("\n=== Narrow width columns ===");

    let items: Vec<Box<dyn Renderable + Send + Sync>> = vec![
        Box::new(Text::plain("Hello World")),
        Box::new(Text::plain("Goodbye World")),
    ];
    let columns = Columns::new(items);
    let output = render_columns(&columns, 15);
    let lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();
    println!("Columns at narrow width=15: lines={}", lines.len());
    println!("  items stacked vertically: {}", lines.len() >= 2);
}
