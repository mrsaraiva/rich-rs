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
pub mod error;
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
pub use error::ParseError;
pub use measure::{measure_renderables, Measurement};
pub use segment::{ControlType, Segment, Segments};
pub use style::{Style, StyleMeta};
pub use text::Text;

/// A type that can be rendered to the console.
///
/// All renderables must be `Send + Sync` to support `Live` and `Progress` features.
/// The `measure` method has a default implementation that renders and measures
/// the result; override it for better performance when possible.
pub trait Renderable: Send + Sync {
    /// Render this object to a sequence of segments.
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments;

    /// Measure the minimum and maximum width requirements.
    ///
    /// Default implementation renders and measures the result.
    /// Override for better performance.
    fn measure(&self, console: &Console, options: &ConsoleOptions) -> Measurement {
        Measurement::from_segments(&self.render(console, options))
    }
}

/// A type that can be converted to a Renderable.
///
/// Uses an associated type to avoid heap allocation.
pub trait RichCast {
    /// The renderable type this converts to.
    type Output: Renderable;

    /// Convert to a renderable type.
    fn rich(&self) -> Self::Output;
}

// Implement Renderable for common types
impl Renderable for str {
    fn render(&self, _console: &Console, _options: &ConsoleOptions) -> Segments {
        // Convert to owned String since Segment requires 'static
        Segments::from(Segment::new(self.to_owned()))
    }
}

impl Renderable for String {
    fn render(&self, _console: &Console, _options: &ConsoleOptions) -> Segments {
        Segments::from(Segment::new(self.clone()))
    }
}
