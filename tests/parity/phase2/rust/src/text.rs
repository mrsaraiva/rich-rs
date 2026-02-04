use rich_rs::{Span, Style, Text, TextPart};

fn format_list(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|s| format!("'{}'", s)).collect();
    format!("[{}]", quoted.join(", "))
}

fn bool_lower(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

pub fn run() {
    println!("=== Span Methods ===");

    let style = Style::parse("bold").unwrap();
    let span = Span::new(5, 15, style);
    let (left, right) = span.split(10);
    if let Some(right) = right {
        println!(
            "Span(5,15).split(10) -> ({},{}), ({},{})",
            left.start, left.end, right.start, right.end
        );
    } else {
        println!(
            "Span(5,15).split(10) -> ({},{}), None",
            left.start, left.end
        );
    }

    let style = Style::parse("bold").unwrap();
    let span = Span::new(5, 15, style);
    let (left, right) = span.split(3);
    println!(
        "Span(5,15).split(3) -> ({},{}), {}",
        left.start,
        left.end,
        match right {
            Some(right) => format!("({},{})", right.start, right.end),
            None => "None".to_string(),
        }
    );

    let style = Style::parse("bold").unwrap();
    let span = Span::new(5, 15, style);
    let (left, right) = span.split(20);
    println!(
        "Span(5,15).split(20) -> ({},{}), {}",
        left.start,
        left.end,
        match right {
            Some(right) => format!("({},{})", right.start, right.end),
            None => "None".to_string(),
        }
    );

    let style = Style::parse("bold").unwrap();
    let span = Span::new(5, 10, style);
    let moved = span.move_by(3);
    println!("Span(5,10).move(3) -> ({},{})", moved.start, moved.end);

    let style = Style::parse("bold").unwrap();
    let span = Span::new(5, 10, style);
    let moved = span.move_by(-2);
    println!(
        "Span(5,10).move(-2) -> ({},{})",
        moved.start, moved.end
    );

    let style = Style::parse("bold").unwrap();
    let span = Span::new(5, 15, style);
    let cropped = span.right_crop(10);
    println!(
        "Span(5,15).right_crop(10) -> ({},{})",
        cropped.start, cropped.end
    );

    let style = Style::parse("bold").unwrap();
    let span = Span::new(5, 15, style);
    let cropped = span.right_crop(20);
    println!(
        "Span(5,15).right_crop(20) -> ({},{})",
        cropped.start, cropped.end
    );

    let style = Style::parse("bold").unwrap();
    let span = Span::new(5, 10, style);
    let extended = span.extend(5);
    println!(
        "Span(5,10).extend(5) -> ({},{})",
        extended.start, extended.end
    );

    let style = Style::parse("bold").unwrap();
    let span = Span::new(5, 10, style);
    let extended = span.extend(0);
    println!(
        "Span(5,10).extend(0) -> ({},{})",
        extended.start, extended.end
    );

    println!("\n=== Text.from_markup() ===");

    let text = Text::from_markup("[bold]Hello[/bold] World", false).unwrap();
    println!(
        "from_markup(\"[bold]Hello[/] World\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    let text = Text::from_markup("[red]Red[/red] and [blue]Blue[/blue]", false).unwrap();
    println!(
        "from_markup(\"[red]Red[/] and [blue]Blue[/]\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    let text = Text::from_markup("No markup here", false).unwrap();
    println!(
        "from_markup(\"No markup here\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    let text = Text::from_markup(":smile: emoji", true).unwrap();
    let has_emoji = text.plain_text().contains('😄');
    println!(
        "from_markup(\":smile: emoji\") -> has_emoji={}",
        bool_lower(has_emoji)
    );

    println!("\n=== Text.assemble() ===");

    let bold = Style::parse("bold").unwrap();
    let text = Text::assemble([
        TextPart::from("Hello "),
        TextPart::from(("World", bold)),
    ]);
    println!(
        "assemble(\"Hello \", (\"World\", \"bold\")) -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    let red = Style::parse("red").unwrap();
    let blue = Style::parse("blue").unwrap();
    let text = Text::assemble([
        TextPart::from(("Red", red)),
        TextPart::from(" and "),
        TextPart::from(("Blue", blue)),
    ]);
    println!(
        "assemble((\"Red\", \"red\"), \" and \", (\"Blue\", \"blue\")) -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    let italic = Style::parse("italic").unwrap();
    let styled = Text::styled("Styled", italic);
    let text = Text::assemble([
        TextPart::from("Prefix "),
        TextPart::from(styled),
        TextPart::from(" Suffix"),
    ]);
    println!(
        "assemble(\"Prefix \", Text(\"Styled\", style=\"italic\"), \" Suffix\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    println!("\n=== Text.stylize() ===");

    let mut text = Text::plain("Hello World");
    let bold = Style::parse("bold").unwrap();
    text.stylize_range(bold, 0, Some(5));
    println!(
        "stylize(\"bold\", 0, 5) -> spans={}, first=({},{})",
        text.spans().len(),
        text.spans()[0].start,
        text.spans()[0].end
    );

    let mut text = Text::plain("Hello World");
    let bold = Style::parse("bold").unwrap();
    text.stylize_range(bold, -5, None);
    println!(
        "stylize(\"bold\", -5) -> spans={}, first=({},{})",
        text.spans().len(),
        text.spans()[0].start,
        text.spans()[0].end
    );

    let mut text = Text::plain("Hello World");
    let bold = Style::parse("bold").unwrap();
    text.stylize_range(bold, 0, Some(-6));
    println!(
        "stylize(\"bold\", 0, -6) -> spans={}, first=({},{})",
        text.spans().len(),
        text.spans()[0].start,
        text.spans()[0].end
    );

    println!("\n=== Text.stylize_before() ===");

    let mut text = Text::plain("Hello World");
    let bold = Style::parse("bold").unwrap();
    text.stylize_range(bold, 0, None);
    let italic = Style::parse("italic").unwrap();
    text.stylize_before(italic, 0, None);
    let spans_order: Vec<String> = text
        .spans()
        .iter()
        .map(|s| {
            if s.style.italic == Some(true) {
                "italic".to_string()
            } else {
                "bold".to_string()
            }
        })
        .collect();
    println!(
        "stylize(\"bold\") then stylize_before(\"italic\") -> order={}",
        format_list(&spans_order)
    );

    println!("\n=== Text.highlight_regex() ===");

    let mut text = Text::plain("Hello World Hello");
    let bold = Style::parse("bold").unwrap();
    let count = text.highlight_regex(r"Hello", bold);
    println!(
        "highlight_regex(\"Hello\") -> count={}, spans={}",
        count,
        text.spans().len()
    );

    let mut text = Text::plain("test123test456");
    let red = Style::parse("red").unwrap();
    let count = text.highlight_regex(r"\d+", red);
    println!(
        "highlight_regex(r\"\\d+\") -> count={}, spans={}",
        count,
        text.spans().len()
    );

    let mut text = Text::plain("No matches here");
    let red = Style::parse("red").unwrap();
    let count = text.highlight_regex(r"\d+", red);
    println!(
        "highlight_regex(r\"\\d+\") on \"No matches here\" -> count={}",
        count
    );

    println!("\n=== Text.highlight_words() ===");

    let mut text = Text::plain("The quick brown fox");
    let bold = Style::parse("bold").unwrap();
    let count = text.highlight_words(&["quick", "fox"], bold, true);
    println!(
        "highlight_words([\"quick\", \"fox\"]) -> count={}, spans={}",
        count,
        text.spans().len()
    );

    let mut text = Text::plain("Hello HELLO hello");
    let bold = Style::parse("bold").unwrap();
    let count = text.highlight_words(&["hello"], bold, false);
    println!(
        "highlight_words([\"hello\"], case_sensitive=False) -> count={}",
        count
    );

    let mut text = Text::plain("Hello HELLO hello");
    let bold = Style::parse("bold").unwrap();
    let count = text.highlight_words(&["hello"], bold, true);
    println!(
        "highlight_words([\"hello\"], case_sensitive=True) -> count={}",
        count
    );

    println!("\n=== Text.divide() ===");

    let text = Text::plain("Hello World!");
    let divided = text.divide(vec![5, 6]);
    let plains: Vec<String> = divided.iter().map(|t| t.plain_text().to_string()).collect();
    println!("divide([5, 6]) -> {}", format_list(&plains));

    let text = Text::plain("ABCDEFGHIJ");
    let divided = text.divide(vec![2, 5, 8]);
    let plains: Vec<String> = divided.iter().map(|t| t.plain_text().to_string()).collect();
    println!("divide([2, 5, 8]) -> {}", format_list(&plains));

    let mut text = Text::plain("Hello World");
    let bold = Style::parse("bold").unwrap();
    text.stylize_range(bold, 0, Some(5));
    let divided = text.divide(vec![5]);
    let spans_counts: Vec<usize> = divided.iter().map(|t| t.spans().len()).collect();
    println!(
        "divide([5]) with span(0,5) -> span_counts={:?}",
        spans_counts
    );

    let mut text = Text::plain("Hello World");
    let bold = Style::parse("bold").unwrap();
    text.stylize_range(bold, 3, Some(8));
    let divided = text.divide(vec![5]);
    let spans_counts: Vec<usize> = divided.iter().map(|t| t.spans().len()).collect();
    println!(
        "divide([5]) with span(3,8) crossing -> span_counts={:?}",
        spans_counts
    );
}
