//! Bar example - Draws a filled circle using bars with red gradient.
//!
//! This is a port of Python Rich's bars.py example.
//!
//! Run with: `cargo run --example bars`
//!
//! Note: Requires `pub mod bar;` to be added to src/lib.rs

use rich_rs::bar::Bar;
use rich_rs::{Align, Console, SimpleColor};

const SIZE: usize = 40;

fn main() -> std::io::Result<()> {
    let mut console = Console::new();

    for row in 0..SIZE {
        // Map row to y coordinate in [-1, 1]
        let y = (row as f64 / (SIZE - 1) as f64) * 2.0 - 1.0;

        // Calculate x using circle equation: x^2 + y^2 = 1
        // x = sqrt(1 - y^2)
        let x = (1.0 - y * y).sqrt();

        // Create red gradient based on y position
        // y goes from -1 to 1, so (1 + y) / 2 goes from 0 to 1
        let red_intensity = ((1.0 + y) * 127.5) as u8;
        let color = SimpleColor::Rgb {
            r: red_intensity,
            g: 0,
            b: 0,
        };

        // Create bar with size=2 (range is [0, 2])
        // begin = 1 - x (left edge of circle)
        // end = 1 + x (right edge of circle)
        let bar = Bar::new(2.0, 1.0 - x, 1.0 + x)
            .with_width(SIZE * 2)
            .with_color(color);

        // Center-align the bar
        let aligned = Align::center(Box::new(bar));

        // Print without extra newline (bar already includes one)
        console.print(&aligned, None, None, None, false, "")?;
    }

    Ok(())
}
