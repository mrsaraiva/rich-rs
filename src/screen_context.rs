//! ScreenContext: RAII context for alternate screen mode.
//!
//! This module provides a context guard for alternate screen mode operations.
//! When entering the context, the terminal switches to an alternate screen buffer.
//! When exiting (via drop or explicit exit), the terminal returns to the normal buffer.
//!
//! # Example
//!
//! ```ignore
//! use rich_rs::{Console, Panel, Align, Text};
//! use std::thread::sleep;
//! use std::time::Duration;
//!
//! let mut console = Console::new();
//!
//! // Enter alternate screen mode
//! let mut screen = console.screen(true, None)?;
//!
//! // Update the screen content
//! let text = Align::center(Text::from_markup("[blink]Don't Panic![/blink]", false)?, "middle");
//! screen.update(Panel::new(text))?;
//!
//! sleep(Duration::from_secs(5));
//!
//! // Dropping `screen` automatically exits alternate screen mode
//! ```

use std::io::{self, Stdout, Write};

use crate::Renderable;
use crate::console::Console;
use crate::group::Group;
use crate::screen::Screen;
use crate::style::Style;

/// Context guard for alternate screen mode.
///
/// This struct provides RAII semantics for alternate screen operations.
/// When dropped, it automatically leaves alternate screen mode and restores
/// the cursor if it was hidden.
///
/// Use [`Console::screen()`] to create a `ScreenContext`.
pub struct ScreenContext<'a, W: Write = Stdout> {
    /// Reference to the console.
    console: &'a mut Console<W>,
    /// Whether the cursor should be hidden.
    hide_cursor: bool,
    /// Optional style for the screen background.
    style: Option<Style>,
    /// Whether entering alternate screen actually changed state.
    changed: bool,
}

impl<'a, W: Write> ScreenContext<'a, W> {
    /// Create a new ScreenContext.
    ///
    /// This is called by [`Console::screen()`] and should not be called directly.
    pub(crate) fn new(
        console: &'a mut Console<W>,
        hide_cursor: bool,
        style: Option<Style>,
    ) -> io::Result<Self> {
        // Enter alternate screen mode
        let changed = console.set_alt_screen(true)?;

        // Hide cursor if requested and we actually entered alt screen
        if changed && hide_cursor {
            let _ = console.show_cursor(false);
        }

        Ok(Self {
            console,
            hide_cursor,
            style,
            changed,
        })
    }

    /// Update the screen with new content.
    ///
    /// This renders the given renderable to fill the terminal dimensions.
    ///
    /// # Arguments
    ///
    /// * `renderable` - The content to display on the screen.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut screen = console.screen(true, None)?;
    /// screen.update(Text::plain("Hello, World!"))?;
    /// ```
    pub fn update<R: Renderable + 'static>(&mut self, renderable: R) -> io::Result<()> {
        self.update_with_style(renderable, None)
    }

    /// Update the screen with new content and optional style override.
    ///
    /// # Arguments
    ///
    /// * `renderable` - The content to display on the screen.
    /// * `style` - Optional style override for the screen background.
    pub fn update_with_style<R: Renderable + 'static>(
        &mut self,
        renderable: R,
        style: Option<Style>,
    ) -> io::Result<()> {
        // Create a new Screen with the renderable
        let mut screen = Screen::new(renderable);

        // Apply style from the override or the context's default
        if let Some(s) = style.or(self.style) {
            screen = screen.with_style(s);
        }

        // Enable application mode for alternate screen
        screen = screen.with_application_mode(true);

        // Print the screen (no newline at end since Screen handles its own layout)
        self.console.print(&screen, None, None, None, false, "")
    }

    /// Update the screen with multiple renderables.
    ///
    /// The renderables are grouped together and rendered as a single unit.
    ///
    /// # Arguments
    ///
    /// * `renderables` - Iterator of renderables to display.
    pub fn update_many<I, R>(&mut self, renderables: I) -> io::Result<()>
    where
        I: IntoIterator<Item = R>,
        R: Renderable + 'static,
    {
        self.update_many_with_style(renderables, None)
    }

    /// Update the screen with multiple renderables and optional style.
    ///
    /// # Arguments
    ///
    /// * `renderables` - Iterator of renderables to display.
    /// * `style` - Optional style override for the screen background.
    pub fn update_many_with_style<I, R>(
        &mut self,
        renderables: I,
        style: Option<Style>,
    ) -> io::Result<()>
    where
        I: IntoIterator<Item = R>,
        R: Renderable + 'static,
    {
        let group = Group::new(renderables);
        self.update_with_style(group, style)
    }

    /// Set the default style for the screen.
    ///
    /// This style will be used for all subsequent `update()` calls unless
    /// overridden with `update_with_style()`.
    pub fn set_style(&mut self, style: Option<Style>) {
        self.style = style;
    }

    /// Get the current default style.
    pub fn style(&self) -> Option<Style> {
        self.style
    }

    /// Check if alternate screen mode is active.
    pub fn is_active(&self) -> bool {
        self.changed && self.console.is_alt_screen()
    }

    /// Get a reference to the underlying console.
    pub fn console(&self) -> &Console<W> {
        self.console
    }

    /// Get a mutable reference to the underlying console.
    pub fn console_mut(&mut self) -> &mut Console<W> {
        self.console
    }
}

impl<W: Write> Drop for ScreenContext<'_, W> {
    fn drop(&mut self) {
        if self.changed {
            // Leave alternate screen mode
            let _ = self.console.set_alt_screen(false);

            // Restore cursor if it was hidden
            if self.hide_cursor {
                let _ = self.console.show_cursor(true);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Text;

    #[test]
    fn test_screen_context_creation() {
        // Test with a capture console (non-terminal, so alt screen won't change)
        let mut console = Console::capture();
        let ctx = ScreenContext::new(&mut console, true, None);
        assert!(ctx.is_ok());
    }

    #[test]
    fn test_screen_context_update() {
        let mut console = Console::capture();
        let mut ctx = ScreenContext::new(&mut console, false, None).unwrap();

        // Update should succeed even on capture console
        let result = ctx.update(Text::plain("Hello, World!"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_screen_context_with_style() {
        use crate::SimpleColor;

        let style = Style::new().with_bgcolor(SimpleColor::Standard(1));
        let mut console = Console::capture();
        let ctx = ScreenContext::new(&mut console, true, Some(style));
        assert!(ctx.is_ok());
    }

    #[test]
    fn test_screen_context_update_many() {
        let mut console = Console::capture();
        let mut ctx = ScreenContext::new(&mut console, false, None).unwrap();

        let result = ctx.update_many([Text::plain("Line 1"), Text::plain("Line 2")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_screen_context_set_style() {
        use crate::SimpleColor;

        let mut console = Console::capture();
        let mut ctx = ScreenContext::new(&mut console, false, None).unwrap();

        assert!(ctx.style().is_none());

        let style = Style::new().with_bgcolor(SimpleColor::Standard(2));
        ctx.set_style(Some(style));
        assert!(ctx.style().is_some());
    }

    #[test]
    fn test_screen_context_is_active() {
        // With capture console, alt screen won't actually be entered
        let mut console = Console::capture();
        let ctx = ScreenContext::new(&mut console, false, None).unwrap();

        // Since capture console is not a terminal, changed will be false
        assert!(!ctx.is_active());
    }
}
