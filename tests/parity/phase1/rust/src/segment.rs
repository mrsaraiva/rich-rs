use rich_rs::{Segment, Style};

fn format_style(style: &Option<Style>) -> String {
    match style {
        Some(s) => {
            // Build a style string similar to Python's output
            let mut parts = Vec::new();
            if s.bold == Some(true) {
                parts.push("bold");
            }
            if s.italic == Some(true) {
                parts.push("italic");
            }
            if s.underline == Some(true) {
                parts.push("underline");
            }
            if s.strike == Some(true) {
                parts.push("strike");
            }
            if parts.is_empty() {
                "none".to_string()
            } else {
                parts.join(" ")
            }
        }
        None => "None".to_string(),
    }
}

fn format_list(items: &[&str]) -> String {
    let formatted: Vec<String> = items.iter().map(|s| format!("'{}'", s)).collect();
    format!("[{}]", formatted.join(", "))
}

fn format_list_of_lists(items: &[Vec<&str>]) -> String {
    let inner: Vec<String> = items.iter().map(|list| format_list(list)).collect();
    format!("[{}]", inner.join(", "))
}

fn format_tuple_list(items: &[(&str, String)]) -> String {
    let formatted: Vec<String> = items.iter().map(|(t, s)| format!("('{}', '{}')", t, s)).collect();
    format!("[{}]", formatted.join(", "))
}

fn format_tuple_int_list(items: &[(&str, usize)]) -> String {
    let formatted: Vec<String> = items.iter().map(|(t, n)| format!("('{}', {})", t, n)).collect();
    format!("[{}]", formatted.join(", "))
}

pub fn run() {
    println!("=== Segment Creation ===");

    let seg = Segment::new("hello");
    println!("Segment(\"hello\") -> text=\"{}\", style={}", seg.text, format_style(&seg.style));

    let seg = Segment::styled("hello", Style::parse("bold").unwrap());
    println!("Segment(\"hello\", bold) -> text=\"{}\", style={}", seg.text, format_style(&seg.style));

    println!("\n=== cell_length ===");

    let seg = Segment::new("hello");
    println!("Segment(\"hello\").cell_length -> {}", seg.cell_len());

    let seg = Segment::new("你好");
    println!("Segment(\"你好\").cell_length -> {}", seg.cell_len());

    let seg = Segment::new("hello你好");
    println!("Segment(\"hello你好\").cell_length -> {}", seg.cell_len());

    println!("\n=== split_cells ===");

    let seg = Segment::new("hello");
    let (left, right) = seg.split_cells(3);
    println!("Segment(\"hello\").split_cells(3) -> (\"{}\", \"{}\")", left.text, right.text);

    let seg = Segment::new("hello");
    let (left, right) = seg.split_cells(0);
    println!("Segment(\"hello\").split_cells(0) -> (\"{}\", \"{}\")", left.text, right.text);

    let seg = Segment::new("hello");
    let (left, right) = seg.split_cells(10);
    println!("Segment(\"hello\").split_cells(10) -> (\"{}\", \"{}\")", left.text, right.text);

    let seg = Segment::new("你好世界");
    let (left, right) = seg.split_cells(4);
    println!("Segment(\"你好世界\").split_cells(4) -> (\"{}\", \"{}\")", left.text, right.text);

    let seg = Segment::new("你好世界");
    let (left, right) = seg.split_cells(3);
    println!("Segment(\"你好世界\").split_cells(3) -> (\"{}\", \"{}\")", left.text, right.text);

    println!("\n=== split_lines ===");

    let segments = vec![Segment::new("a\nb\nc".to_string())];
    let lines = Segment::split_lines(segments);
    let result: Vec<Vec<&str>> = lines.iter().map(|line| line.iter().map(|s| s.text.as_ref()).collect()).collect();
    println!("split_lines([Segment(\"a\\nb\\nc\")]) -> {}", format_list_of_lists(&result));

    let segments = vec![
        Segment::new("hello"),
        Segment::new("\n"),
        Segment::new("world"),
    ];
    let lines = Segment::split_lines(segments);
    let result: Vec<Vec<&str>> = lines.iter().map(|line| line.iter().map(|s| s.text.as_ref()).collect()).collect();
    println!("split_lines([Segment(\"hello\"), Segment(\"\\n\"), Segment(\"world\")]) -> {}", format_list_of_lists(&result));

    println!("\n=== simplify ===");

    let bold = Style::parse("bold").unwrap();
    let italic = Style::parse("italic").unwrap();

    let segments = vec![
        Segment::styled("a", bold),
        Segment::styled("b", bold),
        Segment::styled("c", italic),
    ];
    let simplified = Segment::simplify(segments);
    let result: Vec<(&str, String)> = simplified.iter().map(|s| (s.text.as_ref(), format_style(&s.style))).collect();
    println!("simplify([(\"a\", bold), (\"b\", bold), (\"c\", italic)]) -> {}", format_tuple_list(&result));

    let segments = vec![
        Segment::new("a"),
        Segment::new("b"),
        Segment::new("c"),
    ];
    let simplified = Segment::simplify(segments);
    let result: Vec<&str> = simplified.iter().map(|s| s.text.as_ref()).collect();
    println!("simplify([(\"a\"), (\"b\"), (\"c\")]) -> {}", format_list(&result));

    println!("\n=== adjust_line_length ===");

    let line = vec![Segment::new("hello")];
    let adjusted = Segment::adjust_line_length(&line, 10, None, true);
    let result: Vec<(&str, usize)> = adjusted.iter().map(|s| (s.text.as_ref(), s.text.len())).collect();
    println!("adjust_line_length([Segment(\"hello\")], 10) -> {}", format_tuple_int_list(&result));

    let line = vec![Segment::new("hello world".to_string())];
    let adjusted = Segment::adjust_line_length(&line, 5, None, true);
    let result: Vec<&str> = adjusted.iter().map(|s| s.text.as_ref()).collect();
    println!("adjust_line_length([Segment(\"hello world\")], 5) -> {}", format_list(&result));

    let line = vec![Segment::new("hello")];
    let adjusted = Segment::adjust_line_length(&line, 5, None, false);
    let result: Vec<&str> = adjusted.iter().map(|s| s.text.as_ref()).collect();
    println!("adjust_line_length([Segment(\"hello\")], 5, pad=False) -> {}", format_list(&result));
}
