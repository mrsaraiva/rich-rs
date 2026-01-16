use rich_rs::{cell_len, chop_cells, set_cell_size};

fn format_list(items: &[String]) -> String {
    let formatted: Vec<String> = items.iter().map(|s| format!("'{}'", s)).collect();
    format!("[{}]", formatted.join(", "))
}

pub fn run() {
    println!("=== cell_len ===");
    println!("cell_len(\"hello\") -> {}", cell_len("hello"));
    println!("cell_len(\"\") -> {}", cell_len(""));
    println!("cell_len(\"你好\") -> {}", cell_len("你好"));
    println!("cell_len(\"hello你好\") -> {}", cell_len("hello你好"));
    println!("cell_len(\"😀\") -> {}", cell_len("😀"));

    println!("\n=== set_cell_size ===");
    println!("set_cell_size(\"hello\", 5) -> \"{}\"", set_cell_size("hello", 5));
    println!("set_cell_size(\"hello\", 10) -> \"{}\"", set_cell_size("hello", 10));
    println!("set_cell_size(\"hello world\", 5) -> \"{}\"", set_cell_size("hello world", 5));
    println!("set_cell_size(\"你好世界\", 4) -> \"{}\"", set_cell_size("你好世界", 4));
    println!("set_cell_size(\"你好世界\", 5) -> \"{}\"", set_cell_size("你好世界", 5));
    println!("set_cell_size(\"hello\", 0) -> \"{}\"", set_cell_size("hello", 0));

    println!("\n=== chop_cells ===");
    println!("chop_cells(\"hello\", 3) -> {}", format_list(&chop_cells("hello", 3)));
    println!("chop_cells(\"abcdef\", 2) -> {}", format_list(&chop_cells("abcdef", 2)));
    println!("chop_cells(\"你好世界\", 4) -> {}", format_list(&chop_cells("你好世界", 4)));
    println!("chop_cells(\"你好世界\", 5) -> {}", format_list(&chop_cells("你好世界", 5)));
    println!("chop_cells(\"a你b好\", 3) -> {}", format_list(&chop_cells("a你b好", 3)));
}
