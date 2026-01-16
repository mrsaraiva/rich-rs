//! Console: the main API for rendering to the terminal.

use std::io::{self, Write};

use crossterm::terminal;

use crate::segment::Segment;
use crate::style::Style;

/// Terminal color capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorSystem {
    /// No color support.
    None,
    /// Standard 16 colors.
    Standard,
    /// 256 colors.
    EightBit,
    /// 24-bit true color.
    #[default]
    TrueColor,
}

/// Options passed through the rendering pipeline.
#[derive(Debug, Clone)]
pub struct ConsoleOptions {
    /// Maximum width for rendering.
    pub max_width: usize,
    /// Terminal height (if known).
    pub height: Option<usize>,
    /// Color system to use.
    pub color_system: ColorSystem,
    /// Whether output is to a terminal (vs file/pipe).
    pub is_terminal: bool,
    /// Character encoding.
    pub encoding: String,
    /// Whether to use legacy Windows console.
    pub legacy_windows: bool,
}

impl Default for ConsoleOptions {
    fn default() -> Self {
        ConsoleOptions {
            max_width: 80,
            height: None,
            color_system: ColorSystem::TrueColor,
            is_terminal: true,
            encoding: "utf-8".to_string(),
            legacy_windows: false,
        }
    }
}

impl ConsoleOptions {
    /// Create options from the current terminal.
    pub fn from_terminal() -> Self {
        let (width, height) = terminal::size().unwrap_or((80, 24));
        ConsoleOptions {
            max_width: width as usize,
            height: Some(height as usize),
            is_terminal: atty::is(atty::Stream::Stdout),
            ..Default::default()
        }
    }
}

/// The main console for rendering output.
pub struct Console {
    /// Output writer.
    writer: Box<dyn Write>,
    /// Console options.
    options: ConsoleOptions,
    /// Whether to force terminal mode (colors, etc).
    force_terminal: bool,
}

impl Console {
    /// Create a new console writing to stdout.
    pub fn new() -> Self {
        Console {
            writer: Box::new(io::stdout()),
            options: ConsoleOptions::from_terminal(),
            force_terminal: false,
        }
    }

    /// Create a console with specific options.
    pub fn with_options(options: ConsoleOptions) -> Self {
        Console {
            writer: Box::new(io::stdout()),
            options,
            force_terminal: false,
        }
    }

    /// Get the console options.
    pub fn options(&self) -> &ConsoleOptions {
        &self.options
    }

    /// Get the terminal width.
    pub fn width(&self) -> usize {
        self.options.max_width
    }

    /// Get the terminal height (if known).
    pub fn height(&self) -> Option<usize> {
        self.options.height
    }

    /// Print text to the console.
    ///
    /// TODO: Support markup parsing, multiple arguments, etc.
    pub fn print(&mut self, text: &str) -> io::Result<()> {
        writeln!(self.writer, "{}", text)?;
        self.writer.flush()
    }

    /// Print a styled segment.
    pub fn print_segment(&mut self, segment: &Segment) -> io::Result<()> {
        // TODO: Apply style via ANSI codes
        write!(self.writer, "{}", segment.text)?;
        self.writer.flush()
    }

    /// Print multiple segments.
    pub fn print_segments(&mut self, segments: &[Segment]) -> io::Result<()> {
        for segment in segments {
            self.print_segment(segment)?;
        }
        Ok(())
    }

    /// Render a line (horizontal rule).
    pub fn rule(&mut self, title: Option<&str>) -> io::Result<()> {
        let width = self.width();
        match title {
            Some(t) => {
                let padding = (width.saturating_sub(t.len() + 2)) / 2;
                let line: String = "─".repeat(padding);
                writeln!(self.writer, "{} {} {}", line, t, line)?;
            }
            None => {
                let line: String = "─".repeat(width);
                writeln!(self.writer, "{}", line)?;
            }
        }
        self.writer.flush()
    }
}

impl Default for Console {
    fn default() -> Self {
        Console::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_width() {
        let console = Console::with_options(ConsoleOptions {
            max_width: 120,
            ..Default::default()
        });
        assert_eq!(console.width(), 120);
    }
}
