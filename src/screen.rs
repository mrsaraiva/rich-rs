//! Screen: A renderable that fills the terminal screen.
//!
//! Port of Python Rich's `rich/screen.py`.

use std::sync::Arc;

use crate::console::Console;
use crate::console::ConsoleOptions;
use crate::group::Group;
use crate::loop_helpers::loop_last;
use crate::segment::{Segment, Segments};
use crate::style::Style;
use crate::Renderable;

/// A renderable that fills the terminal screen and crops excess.
///
/// Screen renders its content to fill the full terminal dimensions,
/// cropping any excess and padding if needed.
///
/// # Example
///
/// ```
/// use rich_rs::{Console, Text};
/// use rich_rs::screen::Screen;
///
/// let text = Text::plain("Hello, World!");
/// let screen = Screen::new(text);
/// ```
pub struct Screen {
    /// The content to render.
    renderable: Arc<dyn Renderable>,
    /// Optional background style.
    style: Option<Style>,
    /// If true, use application mode newlines (\n\r instead of \n).
    application_mode: bool,
}

impl Screen {
    /// Create a new Screen with a renderable.
    pub fn new(renderable: impl Renderable + 'static) -> Self {
        Self {
            renderable: Arc::new(renderable),
            style: None,
            application_mode: false,
        }
    }

    /// Create a new Screen from multiple renderables.
    ///
    /// Python Rich's `Screen(*renderables)` wraps the inputs in `Group`.
    /// This is the closest Rust equivalent.
    pub fn new_many<I, R>(renderables: I) -> Self
    where
        I: IntoIterator<Item = R>,
        R: Renderable + 'static,
    {
        Self::new(Group::new(renderables))
    }

    /// Create a new Screen from an Arc<dyn Renderable>.
    ///
    /// This avoids an extra Arc allocation when you already have one.
    pub fn from_arc(renderable: Arc<dyn Renderable>) -> Self {
        Self {
            renderable,
            style: None,
            application_mode: false,
        }
    }

    /// Set the background style.
    pub fn with_style(mut self, style: impl Into<Style>) -> Self {
        self.style = Some(style.into());
        self
    }

    /// Enable application mode (uses \n\r newlines).
    ///
    /// Application mode is typically used when the terminal is in alternate
    /// screen mode, where `\n\r` provides correct cursor positioning.
    pub fn with_application_mode(mut self, mode: bool) -> Self {
        self.application_mode = mode;
        self
    }
}

impl Renderable for Screen {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
        let (width, height) = options.size;

        // Create options with full screen dimensions
        let mut render_options = options.clone();
        render_options.size = (width, height);
        render_options.min_width = width.max(1);
        render_options.max_width = width.max(1);
        render_options.max_height = height;
        render_options.height = Some(height);

        // Render to lines with padding
        let lines = console.render_lines(
            &*self.renderable,
            Some(&render_options),
            self.style,
            true,  // pad
            false, // new_lines - we'll add them manually
        );

        // Set shape to exact screen size
        let lines = Segment::set_shape(&lines, width, Some(height), self.style, false);

        // Build output with appropriate newlines
        let new_line = if self.application_mode {
            Segment::new("\n\r")
        } else {
            Segment::line()
        };

        let mut out = Segments::new();
        for (is_last, line) in loop_last(lines.into_iter()) {
            for seg in line {
                out.push(seg);
            }
            if !is_last {
                out.push(new_line.clone());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Text;

    #[test]
    fn test_screen_new_many() {
        let console = Console::new();
        let options = ConsoleOptions {
            size: (10, 3),
            max_width: 10,
            max_height: 3,
            ..Default::default()
        };

        let screen = Screen::new_many([Text::plain("A"), Text::plain("B")]);
        let output: String = screen
            .render(&console, &options)
            .iter()
            .map(|s| s.text.to_string())
            .collect();

        // Should contain both lines in order, separated by newline.
        assert!(output.contains("A"));
        assert!(output.contains("B"));
        assert!(output.contains('\n'));
    }

    #[test]
    fn test_screen_basic() {
        let console = Console::new();
        let options = ConsoleOptions {
            size: (10, 3),
            max_width: 10,
            max_height: 3,
            ..Default::default()
        };

        let text = Text::plain("Hello");
        let screen = Screen::new(text);
        let segments = screen.render(&console, &options);

        // Should produce 3 lines of 10 characters each
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Count newlines - should have 2 (between 3 lines)
        let newline_count = output.matches('\n').count();
        assert_eq!(newline_count, 2);

        // Check that lines are padded to width
        let lines: Vec<&str> = output.split('\n').collect();
        assert_eq!(lines.len(), 3);
        for line in &lines {
            // Each line should be 10 cells wide
            assert_eq!(crate::cell_len(line), 10);
        }
    }

    #[test]
    fn test_screen_with_style() {
        use crate::SimpleColor;

        let console = Console::new();
        let options = ConsoleOptions {
            size: (10, 2),
            max_width: 10,
            max_height: 2,
            ..Default::default()
        };

        // Use Standard ANSI red (color 1)
        let style = Style::new().with_bgcolor(SimpleColor::Standard(1));
        let text = Text::plain("Hi");
        let screen = Screen::new(text).with_style(style);
        let segments = screen.render(&console, &options);

        // Check that style is applied to padding segments
        let styled_segments: Vec<_> = segments
            .iter()
            .filter(|s| s.style.is_some() && !s.text.is_empty() && s.text.as_ref() != "\n")
            .collect();

        assert!(!styled_segments.is_empty());
    }

    #[test]
    fn test_screen_application_mode() {
        let console = Console::new();
        let options = ConsoleOptions {
            size: (5, 2),
            max_width: 5,
            max_height: 2,
            ..Default::default()
        };

        let text = Text::plain("a\nb");
        let screen = Screen::new(text).with_application_mode(true);
        let segments = screen.render(&console, &options);

        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should use \n\r instead of \n
        assert!(output.contains("\n\r"));
        // Should not have plain \n (except as part of \n\r)
        let plain_newlines = output
            .chars()
            .zip(output.chars().skip(1).chain(std::iter::once('\0')))
            .filter(|&(c, next)| c == '\n' && next != '\r')
            .count();
        assert_eq!(plain_newlines, 0);
    }

    #[test]
    fn test_screen_crops_excess() {
        let console = Console::new();
        let options = ConsoleOptions {
            size: (5, 2),
            max_width: 5,
            max_height: 2,
            ..Default::default()
        };

        // Content that exceeds screen size
        let text = Text::plain("Line 1\nLine 2\nLine 3\nLine 4");
        let screen = Screen::new(text);
        let segments = screen.render(&console, &options);

        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should only have 2 lines (cropped to height)
        let lines: Vec<&str> = output.split('\n').collect();
        assert_eq!(lines.len(), 2);

        // Each line should be 5 cells (cropped to width)
        for line in &lines {
            assert_eq!(crate::cell_len(line), 5);
        }
    }

    #[test]
    fn test_screen_from_arc() {
        let console = Console::new();
        let options = ConsoleOptions {
            size: (10, 2),
            max_width: 10,
            max_height: 2,
            ..Default::default()
        };

        let text: Arc<dyn Renderable> = Arc::new(Text::plain("Hello"));
        let screen = Screen::from_arc(text);
        let segments = screen.render(&console, &options);

        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        assert!(output.contains("Hello"));
    }
}
