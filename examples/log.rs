//! Log example
//!
//! Run with: `cargo run --example log`
//!
//! This demonstrates the Console::log() method, which prints messages with
//! timestamps and optional source file/line information.
//!
//! Equivalent to Python Rich's console.log() function.

use std::thread;
use std::time::Duration;

use rich_rs::{Console, Highlighter, RegexHighlighter, Style, Text, Theme};

fn main() {
    let mut console = Console::new();

    // Simple log messages
    rich_rs::log!(console, &Text::plain("Server starting...")).unwrap();
    rich_rs::log!(console, &Text::plain("Loading configuration...")).unwrap();

    thread::sleep(Duration::from_millis(500));

    // Log with markup
    rich_rs::log!(
        console,
        &Text::from_markup("Serving on [bold cyan]http://127.0.0.1:8000[/]", false).unwrap()
    )
    .unwrap();

    thread::sleep(Duration::from_millis(500));

    // Create a custom highlighter for HTTP requests
    // RegexHighlighter::new(patterns, base_style)
    let request_highlighter = RegexHighlighter::new(
        &[
            r"(?P<protocol>HTTP)\s+(?P<method>GET|POST|PUT|DELETE|PATCH)\s+(?P<path>\S+)\s+(?P<status>\d{3})",
            r"\/(?P<filename>\w+\.\w{2,4})",
        ],
        "req.",
    );

    // Create a theme with custom styles for the highlighter
    let mut theme = Theme::new();
    theme.add_style(
        "req.protocol",
        Style::parse("dim bold green").unwrap_or_default(),
    );
    theme.add_style("req.method", Style::parse("bold cyan").unwrap_or_default());
    theme.add_style("req.path", Style::parse("magenta").unwrap_or_default());
    theme.add_style(
        "req.status",
        Style::parse("bold yellow").unwrap_or_default(),
    );
    theme.add_style(
        "req.filename",
        Style::parse("bright_cyan").unwrap_or_default(),
    );

    // Push the theme
    console.push_theme(theme);

    // Simulate some HTTP requests
    let requests = [
        "HTTP GET /index.html 200",
        "HTTP GET /static/style.css 200",
        "HTTP GET /api/users 200",
        "HTTP POST /api/login 201",
        "HTTP GET /favicon.ico 404",
    ];

    for request in requests {
        thread::sleep(Duration::from_millis(300));

        // Apply the highlighter to the request text
        let mut text = Text::plain(request);
        request_highlighter.highlight(&mut text);

        rich_rs::log!(console, &text).unwrap();
    }

    thread::sleep(Duration::from_millis(500));

    // Log without file/line (using the method directly)
    console
        .log(
            &Text::from_markup("[bold green]Server shutting down gracefully...[/]", false).unwrap(),
            None,
            None,
        )
        .unwrap();

    // Final message with file/line
    console
        .log(
            &Text::from_markup("[dim]Goodbye![/]", false).unwrap(),
            Some(file!()),
            Some(line!()),
        )
        .unwrap();
}
