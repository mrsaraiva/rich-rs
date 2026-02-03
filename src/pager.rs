//! Pager: Display content using a system pager.
//!
//! Port of Python Rich's `rich/pager.py`.
//!
//! Provides a `Pager` trait and `SystemPager` implementation that uses
//! the system pager (`less`, `more`, or `PAGER` env var) to display content.

use std::env;
use std::io::{self, Write};
use std::process::{Command, Stdio};

/// Trait for pager implementations.
///
/// A pager displays content that may be too large for the terminal,
/// allowing the user to scroll through it.
pub trait Pager: Send + Sync {
    /// Show content in the pager.
    ///
    /// # Arguments
    ///
    /// * `content` - The content to display.
    ///
    /// # Returns
    ///
    /// An IO result indicating success or failure.
    fn show(&self, content: &str) -> io::Result<()>;
}

/// Uses the pager installed on the system.
///
/// Tries the following pagers in order:
/// 1. `PAGER` environment variable
/// 2. `less`
/// 3. `more`
/// 4. Falls back to printing directly to stdout
///
/// # Example
///
/// ```no_run
/// use rich_rs::pager::{Pager, SystemPager};
///
/// let pager = SystemPager::new();
/// pager.show("Long content here...").unwrap();
/// ```
#[derive(Debug, Clone, Default)]
pub struct SystemPager {
    /// Optional styles flag (for compatibility with Python Rich).
    /// When true, ANSI styles are preserved in the pager output.
    pub styles: bool,
}

impl SystemPager {
    /// Create a new SystemPager.
    pub fn new() -> Self {
        Self { styles: false }
    }

    /// Create a new SystemPager with styles enabled.
    ///
    /// When styles is true, the pager is invoked with flags to preserve
    /// ANSI escape sequences (e.g., `less -R`).
    pub fn with_styles(styles: bool) -> Self {
        Self { styles }
    }

    /// Get the pager command to use.
    ///
    /// Returns the pager command from `PAGER` env var, or falls back to
    /// `less` or `more`.
    fn get_pager_command(&self) -> Option<(String, Vec<String>)> {
        // Check PAGER environment variable first
        if let Ok(pager) = env::var("PAGER") {
            if !pager.is_empty() {
                // Parse the pager command (may include args)
                let parts: Vec<&str> = pager.split_whitespace().collect();
                if let Some((cmd, args)) = parts.split_first() {
                    let mut args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
                    // Add -R flag for styles if using less and styles enabled
                    if self.styles && cmd.ends_with("less") && !args.iter().any(|a| a.contains('R'))
                    {
                        args.push("-R".to_string());
                    }
                    return Some((cmd.to_string(), args));
                }
            }
        }

        // Try less first (with -R flag for ANSI color support when styles enabled)
        if Self::command_exists("less") {
            let args = if self.styles {
                vec!["-R".to_string()]
            } else {
                vec![]
            };
            return Some(("less".to_string(), args));
        }

        // Try more as fallback
        if Self::command_exists("more") {
            return Some(("more".to_string(), vec![]));
        }

        None
    }

    /// Check if a command exists on the system.
    fn command_exists(cmd: &str) -> bool {
        #[cfg(unix)]
        {
            Command::new("which")
                .arg(cmd)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }

        #[cfg(windows)]
        {
            Command::new("where")
                .arg(cmd)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
    }

    /// Run the pager with the given content.
    fn run_pager(&self, cmd: &str, args: &[String], content: &str) -> io::Result<()> {
        let mut child = Command::new(cmd).args(args).stdin(Stdio::piped()).spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(content.as_bytes())?;
            stdin.flush()?;
        }

        child.wait()?;
        Ok(())
    }
}

impl Pager for SystemPager {
    fn show(&self, content: &str) -> io::Result<()> {
        if let Some((cmd, args)) = self.get_pager_command() {
            self.run_pager(&cmd, &args, content)
        } else {
            // Fallback: just print to stdout
            print!("{}", content);
            io::stdout().flush()
        }
    }
}

/// A pager that does nothing - just prints content directly.
///
/// Useful for testing or when paging is not desired.
#[derive(Debug, Clone, Default)]
pub struct NullPager;

impl NullPager {
    /// Create a new NullPager.
    pub fn new() -> Self {
        Self
    }
}

impl Pager for NullPager {
    fn show(&self, content: &str) -> io::Result<()> {
        print!("{}", content);
        io::stdout().flush()
    }
}

/// A pager that captures content to a buffer.
///
/// Useful for testing.
#[derive(Debug, Default)]
pub struct BufferPager {
    /// The captured content.
    content: std::sync::Mutex<String>,
}

impl BufferPager {
    /// Create a new BufferPager.
    pub fn new() -> Self {
        Self {
            content: std::sync::Mutex::new(String::new()),
        }
    }

    /// Get the captured content.
    pub fn get_content(&self) -> String {
        self.content.lock().unwrap().clone()
    }

    /// Clear the captured content.
    pub fn clear(&self) {
        self.content.lock().unwrap().clear();
    }
}

impl Clone for BufferPager {
    fn clone(&self) -> Self {
        Self {
            content: std::sync::Mutex::new(self.content.lock().unwrap().clone()),
        }
    }
}

impl Pager for BufferPager {
    fn show(&self, content: &str) -> io::Result<()> {
        self.content.lock().unwrap().push_str(content);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_pager_new() {
        let pager = SystemPager::new();
        assert!(!pager.styles);
    }

    #[test]
    fn test_system_pager_with_styles() {
        let pager = SystemPager::with_styles(true);
        assert!(pager.styles);
    }

    #[test]
    fn test_null_pager() {
        let pager = NullPager::new();
        // NullPager just prints to stdout, so we can't easily capture
        // Just verify it doesn't panic
        assert!(pager.show("test").is_ok());
    }

    #[test]
    fn test_buffer_pager() {
        let pager = BufferPager::new();
        pager.show("Hello").unwrap();
        pager.show(" World").unwrap();
        assert_eq!(pager.get_content(), "Hello World");

        pager.clear();
        assert_eq!(pager.get_content(), "");
    }

    #[test]
    fn test_command_exists() {
        // 'ls' should exist on Unix, 'dir' on Windows
        #[cfg(unix)]
        assert!(SystemPager::command_exists("ls"));

        #[cfg(windows)]
        assert!(SystemPager::command_exists("cmd"));

        // This command definitely shouldn't exist
        assert!(!SystemPager::command_exists(
            "definitely_not_a_real_command_12345"
        ));
    }
}
