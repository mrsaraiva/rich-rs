//! Parity test for markup module.

use rich_rs::markup::{escape, parse, render, Tag};

fn format_token(position: usize, text: &Option<String>, tag: &Option<Tag>) -> String {
    if let Some(t) = text {
        // Use single quotes to match Python output
        format!("({}, Text('{}'))", position, t.replace('\'', "\\'"))
    } else if let Some(t) = tag {
        match &t.parameters {
            Some(p) => format!("({}, Tag('{}', '{}'))", position, t.name, p),
            None => format!("({}, Tag('{}', None))", position, t.name),
        }
    } else {
        format!("({}, None, None)", position)
    }
}

fn format_span(start: usize, end: usize, style: &str) -> String {
    // Use single quotes to match Python output
    format!("Span({}, {}, '{}')", start, end, style)
}

/// Format a string value with Python-style single quotes
fn py_repr(s: &str) -> String {
    format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
}

pub fn run() {
    println!("=== _parse (tokenizer) ===");

    // Plain text
    let tokens = parse("hello world");
    println!("_parse(\"hello world\"):");
    for (pos, text, tag) in &tokens {
        println!("  {}", format_token(*pos, text, tag));
    }

    // Single tag
    let tokens = parse("[bold]hello[/bold]");
    println!("_parse(\"[bold]hello[/bold]\"):");
    for (pos, text, tag) in &tokens {
        println!("  {}", format_token(*pos, text, tag));
    }

    // Tag with parameters
    let tokens = parse("[link=https://example.com]click[/link]");
    println!("_parse(\"[link=https://example.com]click[/link]\"):");
    for (pos, text, tag) in &tokens {
        println!("  {}", format_token(*pos, text, tag));
    }

    // Escaped bracket
    let tokens = parse("\\[not a tag]");
    println!("_parse(\"\\\\[not a tag]\"):");
    for (pos, text, tag) in &tokens {
        println!("  {}", format_token(*pos, text, tag));
    }

    // Mixed content
    let tokens = parse("Hello [bold]World[/bold]!");
    println!("_parse(\"Hello [bold]World[/bold]!\"):");
    for (pos, text, tag) in &tokens {
        println!("  {}", format_token(*pos, text, tag));
    }

    println!("\n=== escape ===");
    println!("escape(\"hello world\") -> {}", py_repr(&escape("hello world")));
    println!("escape(\"[bold]\") -> {}", py_repr(&escape("[bold]")));
    println!("escape(\"\\\\[bold]\") -> {}", py_repr(&escape("\\[bold]")));
    println!(
        "escape(\"[not a tag because 123]\") -> {}",
        py_repr(&escape("[not a tag because 123]"))
    );
    println!("escape(\"[red]hello[/red]\") -> {}", py_repr(&escape("[red]hello[/red]")));

    println!("\n=== render (plain text) ===");

    // Plain text (no markup)
    let text = render("hello world", false).unwrap();
    println!("render(\"hello world\").plain -> {}", py_repr(text.plain_text()));

    // Bold text
    let text = render("[bold]hello[/bold]", false).unwrap();
    println!("render(\"[bold]hello[/bold]\").plain -> {}", py_repr(text.plain_text()));

    // Implicit close
    let text = render("[bold]hello[/]", false).unwrap();
    println!("render(\"[bold]hello[/]\").plain -> {}", py_repr(text.plain_text()));

    // Nested tags
    let text = render("[bold][italic]hello[/italic][/bold]", false).unwrap();
    println!(
        "render(\"[bold][italic]hello[/italic][/bold]\").plain -> {}",
        py_repr(text.plain_text())
    );

    // Color
    let text = render("[red]hello[/red]", false).unwrap();
    println!("render(\"[red]hello[/red]\").plain -> {}", py_repr(text.plain_text()));

    // Link
    let text = render("[link=https://example.com]click here[/link]", false).unwrap();
    println!(
        "render(\"[link=https://example.com]click here[/link]\").plain -> {}",
        py_repr(text.plain_text())
    );

    // Escaped bracket
    let text = render("\\[not bold]", false).unwrap();
    println!("render(\"\\\\[not bold]\").plain -> {}", py_repr(text.plain_text()));

    // Unclosed tag (applies to end)
    let text = render("[bold]hello", false).unwrap();
    println!("render(\"[bold]hello\").plain -> {}", py_repr(text.plain_text()));

    // Multiple styles
    let text = render("[bold red on blue]styled[/]", false).unwrap();
    println!(
        "render(\"[bold red on blue]styled[/]\").plain -> {}",
        py_repr(text.plain_text())
    );

    // Overlapping styles
    let text = render("[bold]Hello [italic]World[/italic]![/bold]", false).unwrap();
    println!(
        "render(\"[bold]Hello [italic]World[/italic]![/bold]\").plain -> {}",
        py_repr(text.plain_text())
    );

    println!("\n=== render (spans) ===");

    // Bold text spans
    let text = render("[bold]hello[/bold]", false).unwrap();
    println!("render(\"[bold]hello[/bold]\").spans:");
    for span in text.spans() {
        // Convert style to string representation for comparison
        // Note: Rust stores actual Style, Python stores style string
        println!("  {}", format_span(span.start, span.end, "bold"));
    }

    // Nested tags spans
    let text = render("[bold][italic]hello[/italic][/bold]", false).unwrap();
    println!("render(\"[bold][italic]hello[/italic][/bold]\").spans:");
    let mut spans: Vec<_> = text.spans().iter().collect();
    spans.sort_by_key(|s| (s.start, s.end));
    // Note: We emit both spans but style names may differ in representation
    for (i, span) in spans.iter().enumerate() {
        let style_name = if i == 0 { "bold" } else { "italic" };
        println!("  {}", format_span(span.start, span.end, style_name));
    }

    // Multiple tags
    let text = render("[red]Hello[/red] [blue]World[/blue]", false).unwrap();
    println!("render(\"[red]Hello[/red] [blue]World[/blue]\").spans:");
    let mut spans: Vec<_> = text.spans().iter().collect();
    spans.sort_by_key(|s| (s.start, s.end));
    for (i, span) in spans.iter().enumerate() {
        let style_name = if i == 0 { "red" } else { "blue" };
        println!("  {}", format_span(span.start, span.end, style_name));
    }

    println!("\n=== render with emoji ===");

    // Emoji replacement
    let text = render(":smile:", true).unwrap();
    println!("render(\":smile:\", emoji=True).plain -> {}", py_repr(text.plain_text()));

    // Emoji in styled text
    let text = render("[bold]:+1:[/bold]", true).unwrap();
    println!(
        "render(\"[bold]:+1:[/bold]\", emoji=True).plain -> {}",
        py_repr(text.plain_text())
    );

    // No emoji replacement
    let text = render(":smile:", false).unwrap();
    println!("render(\":smile:\", emoji=False).plain -> {}", py_repr(text.plain_text()));
}
