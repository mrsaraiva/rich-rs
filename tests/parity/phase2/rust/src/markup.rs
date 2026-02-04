//! Parity test for markup module.

use rich_rs::markup::{escape, render};

fn bool_lower(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

pub fn run() {
    println!("=== Markup escape() ===");

    let result = escape("[bold]");
    println!("escape(\"[bold]\") -> \"{}\"", result);

    let result = escape("\\[bold]");
    println!("escape(\"\\\\[bold]\") -> \"{}\"", result);

    let result = escape("hello world");
    println!("escape(\"hello world\") -> \"{}\"", result);

    let result = escape("[123]");
    println!("escape(\"[123]\") -> \"{}\"", result);

    let result = escape("[red]text[/red]");
    println!("escape(\"[red]text[/red]\") -> \"{}\"", result);

    println!("\n=== Markup render() - Basic ===");

    let text = render("plain text", false).unwrap();
    println!(
        "render(\"plain text\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    let text = render("[bold]hello[/bold]", false).unwrap();
    println!(
        "render(\"[bold]hello[/]\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    let text = render("[italic]world[/italic]", false).unwrap();
    println!(
        "render(\"[italic]world[/]\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    let text = render("[bold][italic]both[/italic][/bold]", false).unwrap();
    println!(
        "render(\"[bold][italic]both[/][/]\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    println!("\n=== Markup render() - Colors ===");

    let text = render("[red]red text[/red]", false).unwrap();
    println!(
        "render(\"[red]red text[/]\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    let text = render("[bold red]styled[/bold red]", false).unwrap();
    println!(
        "render(\"[bold red]styled[/]\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    let text = render("[on blue]bg color[/on blue]", false).unwrap();
    println!(
        "render(\"[on blue]bg color[/]\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    println!("\n=== Markup render() - Implicit close ===");

    let text = render("[bold]hello[/]", false).unwrap();
    println!(
        "render(\"[bold]hello[/]\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    let text = render("[red][bold]nested[/][/]", false).unwrap();
    println!(
        "render(\"[red][bold]nested[/][/]\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    println!("\n=== Markup render() - Escaped brackets ===");

    let text = render("\\[not a tag]", false).unwrap();
    println!(
        "render(\"\\\\[not a tag]\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    let text = render("before \\[escaped] after", false).unwrap();
    println!(
        "render(\"before \\\\[escaped] after\") -> plain=\"{}\"",
        text.plain_text()
    );

    println!("\n=== Markup render() - Links ===");

    let text = render("[link=https://example.com]click[/link]", false).unwrap();
    println!(
        "render(\"[link=url]click[/link]\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    println!("\n=== Markup render() - Emoji ===");

    let text = render(":smile:", true).unwrap();
    let has_emoji = text.plain_text().contains('😄');
    println!(
        "render(\":smile:\") -> has_emoji={}",
        bool_lower(has_emoji)
    );

    let text = render("[bold]:+1:[/bold]", true).unwrap();
    let has_emoji = text.plain_text().contains('👍');
    println!(
        "render(\"[bold]:+1:[/]\") -> has_emoji={}, spans={}",
        bool_lower(has_emoji),
        text.spans().len()
    );

    println!("\n=== Markup render() - Mixed content ===");

    let text = render("Hello [bold]World[/bold]!", false).unwrap();
    println!(
        "render(\"Hello [bold]World[/] !\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );

    let text = render("[red]A[/red] [blue]B[/blue] [green]C[/green]", false).unwrap();
    println!(
        "render(\"[red]A[/] [blue]B[/] [green]C[/]\") -> plain=\"{}\", spans={}",
        text.plain_text(),
        text.spans().len()
    );
}
