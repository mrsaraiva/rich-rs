//! Layout example: demonstrates rich.Layout for terminal area splitting.
//!
//! This is a port of Python Rich's examples/layout.py
//!
//! Run with:
//!   cargo run --example layout

use std::io::Stdout;
use std::thread::sleep;
use std::time::Duration;

use rich_rs::{
    Align, Console, ConsoleOptions, Layout, Live, LiveOptions, Measurement, Renderable, Segments,
    Style, Text, VerticalAlignMethod,
};

// ============================================================================
// Clock: A renderable that shows the current time
// ============================================================================

/// Renders the time in the center of the screen.
/// This is the Rust equivalent of the Python Clock class with __rich__ method.
struct Clock;

impl Renderable for Clock {
    fn render(&self, _console: &Console<Stdout>, _options: &ConsoleOptions) -> Segments {
        // Get current time using std::time
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        // Format a simple time string (seconds since epoch formatted as time)
        let total_secs = now.as_secs();
        let secs = total_secs % 60;
        let mins = (total_secs / 60) % 60;
        let hours = (total_secs / 3600) % 24;

        // Create a simple time display
        let time_str = format!("{:02}:{:02}:{:02} UTC", hours, mins, secs);
        let text = Text::styled(&time_str, Style::parse("bold magenta").unwrap_or_default());
        text.render(_console, _options)
    }

    fn measure(&self, _console: &Console<Stdout>, _options: &ConsoleOptions) -> Measurement {
        // "HH:MM:SS UTC" is 12 characters
        Measurement::new(12, 12)
    }
}

fn main() -> std::io::Result<()> {
    // Create the main layout
    let layout = Layout::new();

    // Split into header (size 1), main (ratio 1), and footer (size 10)
    let header = Layout::new().with_name("header").with_size(1);
    let main = Layout::new().with_name("main").with_ratio(1);
    let footer = Layout::new().with_name("footer").with_size(10);

    layout.split_column(vec![header.clone(), main.clone(), footer]);

    // Split main into side and body (ratio 2)
    let side = Layout::new().with_name("side");
    let body = Layout::new().with_name("body").with_ratio(2);

    main.split_row(vec![side.clone(), body.clone()]);

    // Split side into two unnamed layouts
    let side_top = Layout::new();
    let side_bottom = Layout::new();
    side.split_column(vec![side_top, side_bottom]);

    // Update body with centered content
    let body_text = Text::from_markup(
        "This is a demonstration of [bold cyan]rich.Layout[/]\n\nHit Ctrl+C to exit",
        false,
    )
    .unwrap_or_else(|_| {
        Text::plain("This is a demonstration of rich.Layout\n\nHit Ctrl+C to exit")
    });

    let body_content =
        Align::center(Box::new(body_text)).with_vertical(VerticalAlignMethod::Middle);

    if let Some(body_layout) = layout.get("body") {
        body_layout.update(body_content);
    }

    // Update header with Clock
    if let Some(header_layout) = layout.get("header") {
        header_layout.update(Clock);
    }

    // Create Live display with screen mode
    let options = LiveOptions {
        screen: true,
        refresh_per_second: 1.0,
        ..Default::default()
    };

    let mut live = Live::with_options(Box::new(layout.clone()), options);
    live.start(true)?;

    // Run until Ctrl+C
    loop {
        // Update the layout with Clock (which reads current time on each render)
        // The Live display will auto-refresh, but we also update manually
        if let Some(header_layout) = layout.get("header") {
            header_layout.update(Clock);
        }
        live.update(Box::new(layout.clone()), true)?;
        sleep(Duration::from_secs(1));
    }
}
