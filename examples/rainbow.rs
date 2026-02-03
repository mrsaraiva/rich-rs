//! Rainbow Highlighter Example
//!
//! Demonstrates implementing a custom Highlighter that applies random colors
//! to each character, creating a rainbow effect.
//!
//! This is a port of Python Rich's rainbow.py example:
//! ```python
//! from random import randint
//! from rich import print
//! from rich.highlighter import Highlighter
//!
//! class RainbowHighlighter(Highlighter):
//!     def highlight(self, text):
//!         for index in range(len(text)):
//!             text.stylize(f"color({randint(16, 255)})", index, index + 1)
//!
//! rainbow = RainbowHighlighter()
//! print(rainbow("I must not fear. Fear is the mind-killer."))
//! ```
//!
//! Run with: `cargo run --example rainbow`

use rand::Rng as _;

use rich_rs::highlighter::Highlighter;
use rich_rs::{Console, SimpleColor, Style, Text};

/// A highlighter that applies a random color to each character.
struct RainbowHighlighter;

impl Highlighter for RainbowHighlighter {
    fn highlight(&self, text: &mut Text) {
        let mut rng = rand::rng();
        let len = text.len();

        for index in 0..len {
            // Random color from 16-255 (256-color palette, excluding basic colors)
            let color_code = rng.random_range(16..=255);
            let style = Style::new().with_color(SimpleColor::EightBit(color_code));
            text.stylize(index, index + 1, style);
        }
    }
}

fn main() {
    let highlighter = RainbowHighlighter;

    // Create text and apply rainbow highlighting
    let mut text = Text::plain("I must not fear. Fear is the mind-killer.");
    highlighter.highlight(&mut text);

    // Print to console
    let mut console = Console::new();
    let _ = console.print(&text, None, None, None, false, "\n");
}
