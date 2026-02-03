//! Screen example - Demonstrates Console.screen() alternate screen mode.
//!
//! This is a port of Python Rich's `examples/screen.py`.
//!
//! Run with: `cargo run --example screen`

use std::thread::sleep;
use std::time::Duration;

use rich_rs::align::VerticalAlignMethod;
use rich_rs::{Align, Console, Panel, Style, Text};

fn main() -> std::io::Result<()> {
    let mut console = Console::new();

    // Parse style "bold white on red"
    let style = Style::parse("bold white on red").unwrap_or_default();

    // Enter alternate screen mode with cursor hidden
    let mut screen = console.screen(true, Some(style))?;

    // Create blinking "Don't Panic!" text, centered both horizontally and vertically
    let text = Text::from_markup("[blink]Don't Panic![/blink]", false)
        .unwrap_or_else(|_| Text::plain("Don't Panic!"));

    let aligned = Align::center(Box::new(text)).with_vertical(VerticalAlignMethod::Middle);

    // Wrap in a panel
    let panel = Panel::new(Box::new(aligned));

    // Display on screen
    screen.update(panel)?;

    // Sleep for 5 seconds like the Python version
    sleep(Duration::from_secs(5));

    // ScreenContext automatically cleans up on drop
    Ok(())
}
