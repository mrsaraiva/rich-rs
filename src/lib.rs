//! Rich-rs: Rich text and beautiful formatting for the terminal
//!
//! A Rust port of Python's [Rich](https://github.com/Textualize/rich) library.
//!
//! # Example
//!
//! ```
//! use rich_rs::Console;
//!
//! let mut console = Console::new();
//! console.print("Hello, [bold red]World[/]!").unwrap();
//! ```

// Core modules
mod cells;
mod color;
mod measure;
mod segment;
mod style;

// Higher-level modules
mod console;
pub mod markup;
mod text;

// Box drawing characters
mod box_chars;

// Builtin renderables
mod renderables;

// Re-exports for public API
pub use cells::cell_len;
pub use color::Color;
pub use console::{Console, ConsoleOptions};
pub use measure::Measurement;
pub use segment::{ControlType, Segment};
pub use style::Style;
pub use text::Text;

/// A type that can be rendered to the console.
pub trait Renderable {
    /// Render this object to a sequence of segments.
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Vec<Segment>;
}

/// A type that can report its width requirements.
pub trait Measurable {
    /// Measure the minimum and maximum width requirements.
    fn measure(&self, console: &Console, options: &ConsoleOptions) -> Measurement;
}

/// A type that can be converted to a Renderable.
pub trait RichCast {
    /// Convert to a renderable type.
    fn rich(&self) -> Box<dyn Renderable>;
}
