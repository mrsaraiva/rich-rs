//! Align module parity tests.

use rich_rs::align::{Align, VerticalAlignMethod};
use rich_rs::text::Text;
use rich_rs::{Console, ConsoleOptions, Renderable};

/// Helper to render align to plain text
fn render_align(align: &Align, width: usize) -> String {
    let console = Console::with_options(ConsoleOptions {
        max_width: width,
        ..Default::default()
    });
    let options = console.options().clone();
    let segments = align.render(&console, &options);
    segments.iter().map(|s| s.text.to_string()).collect()
}

pub fn run() {
    println!("=== Align left ===");

    let text = Text::plain("Hello");
    let align = Align::left(Box::new(text));
    let output = render_align(&align, 20);
    let line = output.lines().next().unwrap_or("");
    println!(
        "Align.left(\"Hello\", width=20) -> \"{}\" (len={})",
        line,
        line.len()
    );

    let text = Text::plain("Left");
    let align = Align::left(Box::new(text));
    let output = render_align(&align, 15);
    let line = output.lines().next().unwrap_or("");
    println!(
        "Align.left(\"Left\", width=15) -> \"{}\" (len={})",
        line,
        line.len()
    );

    println!("\n=== Align center ===");

    let text = Text::plain("Center");
    let align = Align::center(Box::new(text));
    let output = render_align(&align, 20);
    let line = output.lines().next().unwrap_or("");
    println!(
        "Align.center(\"Center\", width=20) -> \"{}\" (len={})",
        line,
        line.len()
    );

    let text = Text::plain("Hi");
    let align = Align::center(Box::new(text));
    let output = render_align(&align, 10);
    let line = output.lines().next().unwrap_or("");
    println!(
        "Align.center(\"Hi\", width=10) -> \"{}\" (len={})",
        line,
        line.len()
    );

    println!("\n=== Align right ===");

    let text = Text::plain("Right");
    let align = Align::right(Box::new(text));
    let output = render_align(&align, 20);
    let line = output.lines().next().unwrap_or("");
    println!(
        "Align.right(\"Right\", width=20) -> \"{}\" (len={})",
        line,
        line.len()
    );

    let text = Text::plain("X");
    let align = Align::right(Box::new(text));
    let output = render_align(&align, 10);
    let line = output.lines().next().unwrap_or("");
    println!(
        "Align.right(\"X\", width=10) -> \"{}\" (len={})",
        line,
        line.len()
    );

    println!("\n=== Align without right padding ===");

    let text = Text::plain("No Pad");
    let align = Align::center(Box::new(text)).with_pad(false);
    let output = render_align(&align, 20);
    let line = output.lines().next().unwrap_or("");
    println!(
        "Align.center(\"No Pad\", pad=false, width=20) -> \"{}\" (len={})",
        line,
        line.len()
    );

    println!("\n=== Align exact fit ===");

    let text = Text::plain("Exact");
    let align = Align::center(Box::new(text));
    let output = render_align(&align, 5);
    let line = output.lines().next().unwrap_or("");
    println!(
        "Align.center(\"Exact\", width=5) -> \"{}\" (len={})",
        line,
        line.len()
    );

    println!("\n=== VerticalAlignMethod parsing ===");

    println!(
        "VerticalAlignMethod::parse(\"top\") -> {:?}",
        VerticalAlignMethod::parse("top")
    );
    println!(
        "VerticalAlignMethod::parse(\"middle\") -> {:?}",
        VerticalAlignMethod::parse("middle")
    );
    println!(
        "VerticalAlignMethod::parse(\"bottom\") -> {:?}",
        VerticalAlignMethod::parse("bottom")
    );
    println!(
        "VerticalAlignMethod::parse(\"invalid\") -> {:?}",
        VerticalAlignMethod::parse("invalid")
    );

    println!("\n=== Align properties ===");

    let text = Text::plain("Test");
    let align = Align::center(Box::new(text))
        .with_width(30)
        .with_height(10)
        .with_vertical(VerticalAlignMethod::Middle);
    println!("Align.center().align() -> {:?}", align.align());
    println!("Align.center().vertical() -> {:?}", align.vertical());
    println!("Align.center().width() -> {:?}", align.width());
    println!("Align.center().height() -> {:?}", align.height());
    println!("Align.center().pad() -> {}", align.pad());
}
