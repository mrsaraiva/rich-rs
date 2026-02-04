//! Table module parity tests.

use rich_rs::table::Table;
use rich_rs::{Console, ConsoleOptions};

/// Helper to render table to plain text
fn render_table(table: &Table, width: usize) -> String {
    let console = Console::with_options(ConsoleOptions {
        max_width: width,
        is_terminal: true,
        color_system: None,
        ..Default::default()
    });
    let options = console.options().clone();
    let lines = console.render_lines(table, Some(&options), None, false, false);
    let mut out = String::new();
    for line in lines {
        for segment in line {
            out.push_str(&segment.text);
        }
        out.push('\n');
    }
    out
}

fn bool_py(value: bool) -> &'static str {
    if value {
        "True"
    } else {
        "False"
    }
}

pub fn run() {
    println!("=== Simple table ===");

    let mut table = Table::new();
    table.add_column_str("Name");
    table.add_column_str("Age");
    table.add_row_strs(&["Alice", "30"]);
    table.add_row_strs(&["Bob", "25"]);
    let output = render_table(&table, 40);
    let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
    println!("Simple table lines={}", lines.len());
    let has_header = lines.iter().any(|l| l.contains("Name") && l.contains("Age"));
    println!("  has header row: {}", bool_py(has_header));
    let has_data = lines.iter().any(|l| l.contains("Alice"));
    println!("  has data row: {}", bool_py(has_data));

    println!("\n=== Table.grid ===");

    let mut grid = Table::grid();
    grid.add_column_str("");
    grid.add_column_str("");
    grid.add_row_strs(&["A", "B"]);
    grid.add_row_strs(&["C", "D"]);
    let output = render_table(&grid, 40);
    let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
    println!("Table.grid() lines={}", lines.len());
    let has_border = lines.iter().any(|l| l.contains("│") || l.contains("─"));
    println!("  has borders: {}", bool_py(has_border));

    println!("\n=== Table with title ===");

    let mut table = Table::new().with_title("My Table");
    table.add_column_str("Col1");
    table.add_row_strs(&["Data"]);
    let output = render_table(&table, 40);
    let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
    let has_title = lines.iter().any(|l| l.contains("My Table"));
    println!("Table(title='My Table') has_title={}", bool_py(has_title));

    println!("\n=== Table with caption ===");

    let mut table = Table::new().with_caption("Table caption");
    table.add_column_str("Col1");
    table.add_row_strs(&["Data"]);
    let output = render_table(&table, 40);
    let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
    let has_caption = lines.iter().any(|l| l.contains("Table caption"));
    println!(
        "Table(caption='Table caption') has_caption={}",
        bool_py(has_caption)
    );

    println!("\n=== Table column count ===");

    let mut table = Table::new();
    table.add_column_str("A");
    table.add_column_str("B");
    table.add_column_str("C");
    table.add_row_strs(&["1", "2", "3"]);
    let output = render_table(&table, 60);
    let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
    for line in &lines {
        if line.contains("1") && line.contains("2") && line.contains("3") {
            let sep_count = line.matches("│").count();
            println!("  data row has {} separators", sep_count);
            break;
        }
    }

    println!("\n=== Table with expand ===");

    let mut table = Table::new().with_expand(true);
    table.add_column_str("Col");
    table.add_row_strs(&["X"]);
    let output = render_table(&table, 50);
    let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
    if !lines.is_empty() {
        let max_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        println!("Table(expand=True) at width=50: max_line_len={}", max_len);
    }
}
