//! LiveRender: A renderable that may be updated and tracks its shape.
//!
//! Port of Python Rich's `rich/live_render.py`.

use std::sync::{Arc, Mutex};

use crate::control::Control;
use crate::live::VerticalOverflowMethod;
use crate::loop_helpers::loop_last;
use crate::segment::{ControlType, Segment, Segments};
use crate::style::Style;
use crate::text::Text;
use crate::{Console, ConsoleOptions, Measurement, Renderable};

/// Creates a renderable that may be updated.
///
/// LiveRender wraps another renderable, tracks the shape of the last render,
/// and provides methods to position/restore the cursor for live updates.
pub struct LiveRender {
    /// The wrapped renderable.
    renderable: Arc<Mutex<Arc<dyn Renderable>>>,
    /// Style to apply.
    style: Option<Style>,
    /// How to handle vertical overflow.
    vertical_overflow: VerticalOverflowMethod,
    /// Shape of last render (width, height).
    shape: Arc<Mutex<Option<(usize, usize)>>>,
}

impl LiveRender {
    /// Create a new LiveRender.
    pub fn new(renderable: impl Renderable + 'static) -> Self {
        Self {
            renderable: Arc::new(Mutex::new(Arc::new(renderable))),
            style: None,
            vertical_overflow: VerticalOverflowMethod::Ellipsis,
            shape: Arc::new(Mutex::new(None)),
        }
    }

    /// Set the style.
    pub fn with_style(mut self, style: impl Into<Style>) -> Self {
        self.style = Some(style.into());
        self
    }

    /// Set the vertical overflow method.
    pub fn with_vertical_overflow(mut self, method: VerticalOverflowMethod) -> Self {
        self.vertical_overflow = method;
        self
    }

    /// Get the height of the last render.
    pub fn last_render_height(&self) -> usize {
        self.shape.lock().unwrap().map(|(_, h)| h).unwrap_or(0)
    }

    /// Set a new renderable.
    pub fn set_renderable(&self, renderable: impl Renderable + 'static) {
        *self.renderable.lock().unwrap() = Arc::new(renderable);
    }

    /// Get control codes to move cursor to beginning of live render.
    ///
    /// This positions the cursor at the start of the previously rendered content
    /// so that a new render can overwrite it.
    pub fn position_cursor(&self) -> Control {
        if let Some((_, height)) = *self.shape.lock().unwrap() {
            // CR + Erase line, then (cursor up + erase line) * (height-1)
            let mut control = Control::new();
            control.push(ControlType::CarriageReturn);
            control.push(ControlType::EraseInLine(2));
            for _ in 1..height {
                control.push(ControlType::CursorUp(1));
                control.push(ControlType::EraseInLine(2));
            }
            control
        } else {
            Control::new()
        }
    }

    /// Get control codes to clear the render and restore cursor.
    ///
    /// This moves the cursor to where it was before the live render started
    /// and clears all the lines that were used.
    pub fn restore_cursor(&self) -> Control {
        if let Some((_, height)) = *self.shape.lock().unwrap() {
            // CR + (cursor up + erase line) * height
            let mut control = Control::new();
            control.push(ControlType::CarriageReturn);
            for _ in 0..height {
                control.push(ControlType::CursorUp(1));
                control.push(ControlType::EraseInLine(2));
            }
            control
        } else {
            Control::new()
        }
    }
}

impl Renderable for LiveRender {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
        let renderable = self.renderable.lock().unwrap().clone();
        let style = self.style;

        let mut lines = console.render_lines(&*renderable, Some(options), style, false, false);
        let mut shape = Segment::get_shape(&lines);

        let (_, height) = shape;
        let max_height = options.size.1;

        if height > max_height {
            match self.vertical_overflow {
                VerticalOverflowMethod::Crop => {
                    lines.truncate(max_height);
                    shape = Segment::get_shape(&lines);
                }
                VerticalOverflowMethod::Ellipsis => {
                    lines.truncate(max_height.saturating_sub(1));
                    // Get the "live.ellipsis" style from options, or use default
                    let ellipsis_style = options.get_style("live.ellipsis").unwrap_or_default();
                    let ellipsis = Text::styled("...", ellipsis_style).center(options.max_width);
                    let ellipsis_lines =
                        console.render_lines(&ellipsis, Some(options), None, false, false);
                    if let Some(first) = ellipsis_lines.into_iter().next() {
                        lines.push(first);
                    }
                    shape = Segment::get_shape(&lines);
                }
                VerticalOverflowMethod::Visible => {
                    // Don't crop - keep all lines
                }
            }
        }

        *self.shape.lock().unwrap() = Some(shape);

        // Build output
        let new_line = Segment::line();
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

    fn measure(&self, console: &Console, options: &ConsoleOptions) -> Measurement {
        let renderable = self.renderable.lock().unwrap().clone();
        renderable.measure(console, options)
    }
}

// LiveRender uses Arc<Mutex<...>> internally for thread-safety,
// so it's automatically Send + Sync

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Console;

    #[test]
    fn test_live_render_basic() {
        let text = Text::plain("Hello World");
        let live_render = LiveRender::new(text);

        let console = Console::new();
        let options = console.options();
        let segments = live_render.render(&console, options);

        // Should have rendered something
        assert!(!segments.is_empty());

        // Shape should be tracked
        assert!(live_render.last_render_height() > 0);
    }

    #[test]
    fn test_live_render_last_render_height() {
        let text = Text::plain("Line 1\nLine 2\nLine 3");
        let live_render = LiveRender::new(text);

        // Before render, height should be 0
        assert_eq!(live_render.last_render_height(), 0);

        let console = Console::new();
        let options = console.options();
        let _ = live_render.render(&console, options);

        // After render, height should be 3
        assert_eq!(live_render.last_render_height(), 3);
    }

    #[test]
    fn test_live_render_position_cursor_no_render() {
        let text = Text::plain("Hello");
        let live_render = LiveRender::new(text);

        // Before any render, position_cursor should return empty control
        let control = live_render.position_cursor();
        assert!(control.is_empty());
    }

    #[test]
    fn test_live_render_position_cursor_after_render() {
        let text = Text::plain("Line 1\nLine 2\nLine 3");
        let live_render = LiveRender::new(text);

        let console = Console::new();
        let options = console.options();
        let _ = live_render.render(&console, options);

        // After render with 3 lines, position_cursor should return control codes
        let control = live_render.position_cursor();
        assert!(!control.is_empty());

        // Verify the control codes through rendering
        let segments = control.render(&console, options);
        // Should have CR, EraseInLine, (CursorUp, EraseInLine) * 2
        // Total: 1 + 1 + 2*2 = 6 control segments
        assert_eq!(segments.len(), 6);
    }

    #[test]
    fn test_live_render_restore_cursor_no_render() {
        let text = Text::plain("Hello");
        let live_render = LiveRender::new(text);

        // Before any render, restore_cursor should return empty control
        let control = live_render.restore_cursor();
        assert!(control.is_empty());
    }

    #[test]
    fn test_live_render_restore_cursor_after_render() {
        let text = Text::plain("Line 1\nLine 2\nLine 3");
        let live_render = LiveRender::new(text);

        let console = Console::new();
        let options = console.options();
        let _ = live_render.render(&console, options);

        // After render with 3 lines, restore_cursor should return control codes
        let control = live_render.restore_cursor();
        assert!(!control.is_empty());

        // Verify the control codes through rendering
        let segments = control.render(&console, options);
        // Should have CR, (CursorUp, EraseInLine) * 3
        // Total: 1 + 3*2 = 7 control segments
        assert_eq!(segments.len(), 7);
    }

    #[test]
    fn test_live_render_set_renderable() {
        let text1 = Text::plain("Hello");
        let live_render = LiveRender::new(text1);

        let console = Console::new();
        let options = console.options();
        let _ = live_render.render(&console, options);
        assert_eq!(live_render.last_render_height(), 1);

        // Update renderable
        let text2 = Text::plain("Line 1\nLine 2");
        live_render.set_renderable(text2);

        let _ = live_render.render(&console, options);
        assert_eq!(live_render.last_render_height(), 2);
    }

    #[test]
    fn test_live_render_vertical_overflow_crop() {
        // Create a multi-line text that exceeds terminal height
        let lines: Vec<&str> = (0..100).map(|_| "Line").collect();
        let text = Text::plain(lines.join("\n"));
        let live_render =
            LiveRender::new(text).with_vertical_overflow(VerticalOverflowMethod::Crop);

        let console = Console::new();
        let mut options = console.options().clone();
        options.size = (80, 10); // 10 lines max

        let _ = live_render.render(&console, &options);

        // Height should be cropped to max_height
        assert_eq!(live_render.last_render_height(), 10);
    }

    #[test]
    fn test_live_render_vertical_overflow_ellipsis() {
        // Create a multi-line text that exceeds terminal height
        let lines: Vec<&str> = (0..100).map(|_| "Line").collect();
        let text = Text::plain(lines.join("\n"));
        let live_render =
            LiveRender::new(text).with_vertical_overflow(VerticalOverflowMethod::Ellipsis);

        let console = Console::new();
        let mut options = console.options().clone();
        options.size = (80, 10); // 10 lines max

        let _ = live_render.render(&console, &options);

        // Height should be max_height (9 lines + 1 ellipsis line)
        assert_eq!(live_render.last_render_height(), 10);
    }

    #[test]
    fn test_live_render_vertical_overflow_visible() {
        // Create a multi-line text that exceeds terminal height
        let lines: Vec<&str> = (0..100).map(|_| "Line").collect();
        let text = Text::plain(lines.join("\n"));
        let live_render =
            LiveRender::new(text).with_vertical_overflow(VerticalOverflowMethod::Visible);

        let console = Console::new();
        let mut options = console.options().clone();
        options.size = (80, 10); // 10 lines max

        let _ = live_render.render(&console, &options);

        // Height should NOT be cropped
        assert_eq!(live_render.last_render_height(), 100);
    }

    #[test]
    fn test_live_render_with_style() {
        let text = Text::plain("Hello");
        let style = Style::new().with_bold(true);
        let live_render = LiveRender::new(text).with_style(style);

        let console = Console::new();
        let options = console.options();
        let segments = live_render.render(&console, options);

        // Segments should have been rendered (style is applied internally)
        assert!(!segments.is_empty());
    }

    #[test]
    fn test_live_render_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<LiveRender>();
        assert_sync::<LiveRender>();
    }
}
