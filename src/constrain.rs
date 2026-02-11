//! Constrain: clamp a renderable to a maximum width.
//!
//! Port of Python Rich's `constrain.py` (subset).
//!
//! Constrain doesn't crop output directly; it renders the child with a reduced
//! `ConsoleOptions.max_width`, which causes wrapping / layout to behave as if the
//! available width were smaller.

use std::io::Stdout;

use crate::console::ConsoleOptions;
use crate::measure::Measurement;
use crate::segment::Segments;
use crate::{Console, Renderable};

pub struct Constrain {
    renderable: Box<dyn Renderable + Send + Sync>,
    width: Option<usize>,
}

impl std::fmt::Debug for Constrain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Constrain")
            .field("width", &self.width)
            .finish_non_exhaustive()
    }
}

impl Constrain {
    /// Create a new Constrain with an explicit width.
    pub fn new(renderable: Box<dyn Renderable + Send + Sync>, width: Option<usize>) -> Self {
        Self { renderable, width }
    }

    /// Create a new Constrain with the default width of 80 (matches Python Rich).
    pub fn with_default_width(renderable: Box<dyn Renderable + Send + Sync>) -> Self {
        Self {
            renderable,
            width: Some(80),
        }
    }

    pub fn with_width(mut self, width: Option<usize>) -> Self {
        self.width = width;
        self
    }

    pub fn width(&self) -> Option<usize> {
        self.width
    }
}

impl Renderable for Constrain {
    fn render(&self, _console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let Some(width) = self.width else {
            let temp_console = Console::<Stdout>::with_options(options.clone());
            return self.renderable.render(&temp_console, options);
        };

        let child_width = width.min(options.max_width);
        let child_options = options.update_width(child_width);
        let temp_console = Console::<Stdout>::with_options(child_options.clone());
        self.renderable.render(&temp_console, &child_options)
    }

    fn measure(&self, _console: &Console, options: &ConsoleOptions) -> Measurement {
        let options = if let Some(width) = self.width {
            options.update_width(width)
        } else {
            options.clone()
        };
        let temp_console = Console::<Stdout>::with_options(options.clone());
        let measurement = self.renderable.measure(&temp_console, &options);
        // Match Rich: measurements are constrained by the updated width.
        measurement.with_maximum(options.max_width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segment::Segment;
    use crate::text::Text;

    #[test]
    fn test_constrain_none_yields_child() {
        let text = Text::plain("hello");
        let constrain = Constrain::new(Box::new(text.clone()), None);
        let options = ConsoleOptions::default();
        let console = Console::new();
        let a: Vec<_> = constrain.render(&console, &options).into_iter().collect();
        let b: Vec<_> = text.render(&console, &options).into_iter().collect();
        assert_eq!(a, b);
    }

    #[test]
    fn test_constrain_wraps_child_to_width() {
        let text = Text::plain("hello world");
        let constrain = Constrain::new(Box::new(text), Some(5));
        let options = ConsoleOptions {
            max_width: 80,
            ..Default::default()
        };
        let console = Console::new();

        let segs = constrain.render(&console, &options);
        let lines = Segment::split_lines(segs);
        let (w, h) = Segment::get_shape(&lines);
        assert!(h >= 2);
        assert!(w <= 5);
    }

    #[test]
    fn test_constrain_default_width() {
        let text = Text::plain("hello world");
        let constrain = Constrain::with_default_width(Box::new(text));
        assert_eq!(constrain.width(), Some(80));
    }

    #[test]
    fn test_constrain_measure_respects_width() {
        let text = Text::plain("hello world");
        let constrain = Constrain::new(Box::new(text), Some(5));
        let options = ConsoleOptions {
            max_width: 80,
            ..Default::default()
        };
        let console = Console::new();
        let m = constrain.measure(&console, &options);
        assert!(m.maximum <= 5);
    }
}
