//! FileProxy: Redirect writes to a Console.
//!
//! Port of Python Rich's `rich/file_proxy.py`.
//!
//! FileProxy wraps a writer (e.g., stdout) and redirects writes to a Console,
//! using AnsiDecoder to parse ANSI sequences from input. It implements line
//! buffering - accumulating input until a newline, then printing via Console.

use std::io::{self, Stdout, Write};

use crate::ansi::AnsiDecoder;
use crate::text::Text;
use crate::{Console, ConsoleOptions};

/// Wraps a writer (e.g., stdout) and redirects writes to a Console.
///
/// FileProxy buffers input until a newline is encountered, then decodes
/// ANSI escape sequences and prints the result via the Console.
///
/// # Type Parameters
///
/// * `C` - The writer type for the Console (e.g., `Stdout` or `Vec<u8>`).
/// * `W` - The inner writer type to wrap.
///
/// # Example
///
/// ```no_run
/// use rich_rs::{Console, ConsoleOptions};
/// use rich_rs::file_proxy::FileProxy;
/// use std::io::Write;
///
/// let console = Console::new();
/// let mut proxy = FileProxy::new(console, std::io::stdout());
///
/// // Writes are buffered until newline
/// write!(proxy, "Hello, ").unwrap();
/// writeln!(proxy, "World!").unwrap();  // Prints "Hello, World!" via Console
/// ```
pub struct FileProxy<C: Write, W: Write> {
    /// The Console to redirect output to.
    console: Console<C>,
    /// The inner writer (for passthrough operations like fileno).
    inner: W,
    /// Line buffer - accumulates text until newline.
    buffer: String,
    /// ANSI decoder for parsing escape sequences.
    decoder: AnsiDecoder,
}

impl<W: Write> FileProxy<Stdout, W> {
    /// Create a new FileProxy with a stdout Console.
    ///
    /// # Arguments
    ///
    /// * `console` - The Console to redirect output to.
    /// * `inner` - The inner writer to wrap.
    pub fn new(console: Console<Stdout>, inner: W) -> Self {
        Self {
            console,
            inner,
            buffer: String::new(),
            decoder: AnsiDecoder::new(),
        }
    }

    /// Create a new FileProxy with custom console options.
    pub fn with_options(options: ConsoleOptions, inner: W) -> Self {
        Self {
            console: Console::with_options(options),
            inner,
            buffer: String::new(),
            decoder: AnsiDecoder::new(),
        }
    }
}

impl<C: Write, W: Write> FileProxy<C, W> {
    /// Create a new FileProxy with a generic Console.
    ///
    /// # Arguments
    ///
    /// * `console` - The Console to redirect output to.
    /// * `inner` - The inner writer to wrap.
    pub fn with_console(console: Console<C>, inner: W) -> Self {
        Self {
            console,
            inner,
            buffer: String::new(),
            decoder: AnsiDecoder::new(),
        }
    }

    /// Get a reference to the inner writer.
    pub fn inner(&self) -> &W {
        &self.inner
    }

    /// Get a mutable reference to the inner writer.
    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Get a reference to the console.
    pub fn console(&self) -> &Console<C> {
        &self.console
    }

    /// Get a mutable reference to the console.
    pub fn console_mut(&mut self) -> &mut Console<C> {
        &mut self.console
    }

    /// Consume the FileProxy and return the inner writer.
    pub fn into_inner(self) -> W {
        self.inner
    }

    /// Process buffered content and print complete lines.
    fn process_text(&mut self, text: &str) -> io::Result<()> {
        let mut remaining = text;
        let mut lines: Vec<String> = Vec::new();

        while !remaining.is_empty() {
            if let Some(newline_pos) = remaining.find('\n') {
                // Found a newline - complete the current line
                let line_part = &remaining[..newline_pos];
                let complete_line = if self.buffer.is_empty() {
                    line_part.to_string()
                } else {
                    let mut line = std::mem::take(&mut self.buffer);
                    line.push_str(line_part);
                    line
                };
                lines.push(complete_line);
                remaining = &remaining[newline_pos + 1..];
            } else {
                // No newline - buffer the remaining text
                self.buffer.push_str(remaining);
                break;
            }
        }

        // Print complete lines via Console
        if !lines.is_empty() {
            // Decode ANSI sequences and join with newlines
            let decoded_texts: Vec<Text> = lines
                .iter()
                .map(|line| self.decoder.decode_line(line))
                .collect();

            // Join texts with newlines
            let mut output = Text::new();
            for (i, text) in decoded_texts.into_iter().enumerate() {
                if i > 0 {
                    output.append("\n".to_string(), None);
                }
                output.append_text(&text);
            }

            self.console
                .print(&output, None, None, None, false, "\n")
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }

        Ok(())
    }
}

impl<C: Write, W: Write> Write for FileProxy<C, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Convert bytes to string (lossy for non-UTF8)
        let text = String::from_utf8_lossy(buf);
        self.process_text(&text)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // Flush any remaining buffered content
        if !self.buffer.is_empty() {
            let buffered = std::mem::take(&mut self.buffer);
            let decoded = self.decoder.decode_line(&buffered);
            self.console
                .print(&decoded, None, None, None, false, "\n")
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_file_proxy_basic_write() {
        let console = Console::capture();
        let inner = Vec::<u8>::new();
        let mut proxy = FileProxy::with_console(console, inner);

        writeln!(proxy, "Hello, World!").unwrap();
        proxy.flush().unwrap();

        // The output goes to console, not inner
        let console_output = proxy.console().get_captured();
        assert!(console_output.contains("Hello"));
        assert!(console_output.contains("World"));
    }

    #[test]
    fn test_file_proxy_line_buffering() {
        let console = Console::capture();
        let inner = Vec::<u8>::new();
        let mut proxy = FileProxy::with_console(console, inner);

        // Write without newline - should buffer
        write!(proxy, "Hello, ").unwrap();
        assert!(proxy.console().get_captured().is_empty());

        // Write with newline - should flush buffer
        writeln!(proxy, "World!").unwrap();
        let output = proxy.console().get_captured();
        assert!(output.contains("Hello"));
        assert!(output.contains("World"));
    }

    #[test]
    fn test_file_proxy_ansi_decoding() {
        let console = Console::capture();
        let inner = Vec::<u8>::new();
        let mut proxy = FileProxy::with_console(console, inner);

        // Write text with ANSI bold
        writeln!(proxy, "\x1b[1mBold\x1b[0m Normal").unwrap();

        let output = proxy.console().get_captured();
        assert!(output.contains("Bold"));
        assert!(output.contains("Normal"));
    }

    #[test]
    fn test_file_proxy_multiple_lines() {
        let console = Console::capture();
        let inner = Vec::<u8>::new();
        let mut proxy = FileProxy::with_console(console, inner);

        writeln!(proxy, "Line 1").unwrap();
        writeln!(proxy, "Line 2").unwrap();
        writeln!(proxy, "Line 3").unwrap();

        let output = proxy.console().get_captured();
        assert!(output.contains("Line 1"));
        assert!(output.contains("Line 2"));
        assert!(output.contains("Line 3"));
    }

    #[test]
    fn test_file_proxy_flush_partial_line() {
        let console = Console::capture();
        let inner = Vec::<u8>::new();
        let mut proxy = FileProxy::with_console(console, inner);

        // Write without newline
        write!(proxy, "Partial").unwrap();
        assert!(proxy.console().get_captured().is_empty());

        // Explicit flush should print the partial line
        proxy.flush().unwrap();
        let output = proxy.console().get_captured();
        assert!(output.contains("Partial"));
    }

    #[test]
    fn test_file_proxy_inner_access() {
        let console = Console::capture();
        let inner = Vec::<u8>::new();
        let mut proxy = FileProxy::with_console(console, inner);

        // Inner should be accessible
        assert!(proxy.inner().is_empty());
        proxy.inner_mut().push(42);
        assert_eq!(proxy.inner().len(), 1);

        let inner = proxy.into_inner();
        assert_eq!(inner, vec![42]);
    }
}
