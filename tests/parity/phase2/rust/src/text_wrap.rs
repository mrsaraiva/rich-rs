use rich_rs::{JustifyMethod, OverflowMethod, Text};

/// Format a list with single quotes like Python
fn format_list(items: &[&str]) -> String {
    let quoted: Vec<String> = items.iter().map(|s| format!("'{}'", s)).collect();
    format!("[{}]", quoted.join(", "))
}

pub fn run() {
    println!("=== Text.expand_tabs() ===");

    let text = Text::plain("Hello\tWorld");
    let expanded = text.expand_tabs(4);
    println!(
        r#"expand_tabs(4) on "Hello\tWorld" -> plain="{}", len={}"#,
        expanded.plain_text(),
        expanded.plain_text().len()
    );

    let text = Text::plain("\t\tIndented");
    let expanded = text.expand_tabs(8);
    println!(
        r#"expand_tabs(8) on "\t\tIndented" -> len={}"#,
        expanded.plain_text().len()
    );

    println!("\n=== Text.rstrip() ===");

    let text = Text::plain("Hello   ");
    let stripped = text.rstrip();
    println!(
        r#"rstrip() on "Hello   " -> plain="{}", len={}"#,
        stripped.plain_text(),
        stripped.plain_text().len()
    );

    let text = Text::plain("Hello");
    let stripped = text.rstrip();
    println!(
        r#"rstrip() on "Hello" -> plain="{}", len={}"#,
        stripped.plain_text(),
        stripped.plain_text().len()
    );

    println!("\n=== Text.rstrip_end() ===");

    let text = Text::plain("Hello World   ");
    let stripped = text.rstrip_end(5);
    println!(
        r#"rstrip_end(5) on "Hello World   " -> len={}"#,
        stripped.plain_text().len()
    );

    println!("\n=== Text.truncate() ===");

    let text = Text::plain("Hello World");
    let truncated = text.truncate(5, OverflowMethod::Ellipsis, false);
    println!(
        r#"truncate(5, ellipsis) on "Hello World" -> plain="{}""#,
        truncated.plain_text()
    );

    let text = Text::plain("Hello World");
    let truncated = text.truncate(5, OverflowMethod::Crop, false);
    println!(
        r#"truncate(5, crop) on "Hello World" -> plain="{}""#,
        truncated.plain_text()
    );

    let text = Text::plain("Hi");
    let truncated = text.truncate(5, OverflowMethod::Crop, true);
    println!(
        r#"truncate(5, crop, pad=True) on "Hi" -> plain="{}", len={}"#,
        truncated.plain_text(),
        truncated.plain_text().len()
    );

    println!("\n=== Text.align() ===");

    // align("right", 10) = pad_left(10)
    let text = Text::plain("Hello");
    let aligned = text.pad_left(10);
    println!(
        r#"align(right, 10) on "Hello" -> len={}, plain="{}""#,
        aligned.plain_text().len(),
        aligned.plain_text()
    );

    // align("left", 10) = pad_right(10)
    let text = Text::plain("Hello");
    let aligned = text.pad_right(10);
    println!(
        r#"align(left, 10) on "Hello" -> len={}, plain="{}""#,
        aligned.plain_text().len(),
        aligned.plain_text()
    );

    // align("center", 11)
    let text = Text::plain("Hello");
    let aligned = text.center(11);
    println!(
        r#"align(center, 11) on "Hello" -> len={}, plain="{}""#,
        aligned.plain_text().len(),
        aligned.plain_text()
    );

    let text = Text::plain("Hi");
    let aligned = text.center(6);
    println!(
        r#"align(center, 6) on "Hi" -> len={}, plain="{}""#,
        aligned.plain_text().len(),
        aligned.plain_text()
    );

    println!("\n=== Text.split() ===");

    let text = Text::plain("Hello World Test");
    let parts = text.split(" ", false, false);
    let plains: Vec<&str> = parts.iter().map(|t| t.plain_text()).collect();
    println!(
        r#"split(" ") on "Hello World Test" -> {}"#,
        format_list(&plains)
    );

    let text = Text::plain("no-separator");
    let parts = text.split(" ", false, false);
    let plains: Vec<&str> = parts.iter().map(|t| t.plain_text()).collect();
    println!(r#"split(" ") on "no-separator" -> {}"#, format_list(&plains));

    println!("\n=== Text.wrap() ===");

    // Basic wrap - width=10, justify=Left, overflow=default, tab_size=8, no_wrap=false
    let text = Text::plain("Hello World How Are You");
    let wrapped = text.wrap(10, Some(JustifyMethod::Left), None, 8, false);
    let line_count = wrapped.len();
    let first_plain = wrapped.first().map(|t| t.plain_text()).unwrap_or("");
    println!(
        r#"wrap(10, left) on "Hello World How Are You" -> lines={}, first="{}""#,
        line_count, first_plain
    );

    // Wrap with justify full
    let text = Text::plain("Hello World Test");
    let wrapped = text.wrap(12, Some(JustifyMethod::Full), None, 8, false);
    let line_count = wrapped.len();
    println!(
        r#"wrap(12, full) on "Hello World Test" -> lines={}"#,
        line_count
    );

    // Wrap with center
    let text = Text::plain("Hi Test");
    let wrapped = text.wrap(10, Some(JustifyMethod::Center), None, 8, false);
    let first_len = wrapped.first().map(|t| t.plain_text().len()).unwrap_or(0);
    println!(
        r#"wrap(10, center) on "Hi Test" -> first_len={}"#,
        first_len
    );

    // Wrap with fold
    let text = Text::plain("Supercalifragilistic");
    let wrapped = text.wrap(8, Some(JustifyMethod::Left), Some(OverflowMethod::Fold), 8, false);
    let line_count = wrapped.len();
    println!(
        r#"wrap(8, overflow=fold) on "Supercalifragilistic" -> lines={}"#,
        line_count
    );
}
