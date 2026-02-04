//! Panel module parity tests.

use rich_rs::panel::Panel;
use rich_rs::text::Text;
use rich_rs::{Console, ConsoleOptions};

/// Helper to render panel to plain text
fn render_panel(panel: &Panel, width: usize) -> String {
    let console = Console::with_options(ConsoleOptions {
        max_width: width,
        is_terminal: true,
        color_system: None,
        ..Default::default()
    });
    let options = console.options().clone();
    let lines = console.render_lines(panel, Some(&options), None, false, false);
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
    println!("=== Basic Panel ===");

    let text = Text::plain("Hello, World!");
    let panel = Panel::new(Box::new(text));
    let output = render_panel(&panel, 30);
    let lines: Vec<&str> = output.split('\n').collect();
    println!("Panel('Hello, World!') lines={}", lines.len());
    for (i, line) in lines.iter().enumerate() {
        println!("  line[{}]: len={}", i, line.chars().count());
    }

    println!("\n=== Panel with title ===");

    let text = Text::plain("Content");
    let panel = Panel::new(Box::new(text)).with_title("Title");
    let output = render_panel(&panel, 30);
    let lines: Vec<&str> = output.split('\n').collect();
    println!("Panel('Content', title='Title') lines={}", lines.len());
    let has_title = lines.iter().any(|l| l.contains("Title"));
    println!("  contains 'Title': {}", bool_py(has_title));

    println!("\n=== Panel with subtitle ===");

    let text = Text::plain("Content");
    let panel = Panel::new(Box::new(text)).with_subtitle("Subtitle");
    let output = render_panel(&panel, 30);
    let lines: Vec<&str> = output.split('\n').collect();
    println!(
        "Panel('Content', subtitle='Subtitle') lines={}",
        lines.len()
    );
    let has_subtitle = lines.iter().any(|l| l.contains("Subtitle"));
    println!("  contains 'Subtitle': {}", bool_py(has_subtitle));

    println!("\n=== Panel.fit ===");

    let text = Text::plain("Short");
    let panel = Panel::fit(Box::new(text));
    let output = render_panel(&panel, 80);
    let lines: Vec<&str> = output.split('\n').filter(|l| !l.is_empty()).collect();
    if let Some(first) = lines.first() {
        let width = first.chars().count();
        println!("Panel.fit('Short') width={}", width);
        println!("  fits tightly: {}", bool_py(width < 30));
    }

    println!("\n=== Panel with padding ===");

    let text = Text::plain("Padded");
    let panel = Panel::new(Box::new(text)).with_padding((1, 2, 1, 2));
    let output = render_panel(&panel, 30);
    let lines: Vec<&str> = output.split('\n').collect();
    println!("Panel('Padded', padding=(1,2)) lines={}", lines.len());
    let content_lines: Vec<_> = lines
        .iter()
        .filter(|l| l.contains("Padded") || (!l.trim().is_empty() && !l.contains("─") && l.contains("│")))
        .collect();
    println!("  content area lines: {}", content_lines.len());
}
