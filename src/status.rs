//! Status: a spinner with status text for long-running operations.
//!
//! This is a Rust port of Python Rich's `rich/status.py`.
//!
//! Status provides a simple way to show a spinner animation with status text
//! while a long-running operation is in progress.
//!
//! # Example
//!
//! ```no_run
//! use rich_rs::status::Status;
//! use std::thread::sleep;
//! use std::time::Duration;
//!
//! let mut status = Status::new("[bold green]Working on tasks...");
//! status.start().unwrap();
//!
//! for i in 0..10 {
//!     sleep(Duration::from_secs(1));
//!     println!("Task {} complete", i + 1);
//! }
//!
//! status.stop().unwrap();
//! ```

use std::io::{self, Stdout};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::console::{Console, ConsoleOptions};
use crate::measure::Measurement;
use crate::segment::Segments;
use crate::spinner::Spinner;
use crate::style::Style;
use crate::text::Text;
use crate::{Live, LiveOptions, Renderable};

/// A spinner with status text.
///
/// The status text is displayed to the right of the spinner animation.
/// This is typically used to indicate that a long-running operation is in progress.
struct StatusRenderable {
    spinner: Spinner,
    text: Text,
    start: Instant,
    spinner_style: Option<Style>,
}

impl StatusRenderable {
    fn new(spinner: Spinner, text: Text, spinner_style: Option<Style>) -> Self {
        Self {
            spinner,
            text,
            start: Instant::now(),
            spinner_style,
        }
    }
}

impl Renderable for StatusRenderable {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let elapsed = self.start.elapsed().as_secs_f64();
        let frame = self
            .spinner
            .render_at(elapsed, Some(0.0), self.spinner_style);

        // Assemble: spinner_frame + " " + status_text
        let assembled = Text::assemble([
            crate::TextPart::Text(frame),
            crate::TextPart::Plain(" ".to_string()),
            crate::TextPart::Text(self.text.clone()),
        ]);

        assembled.render(console, options)
    }

    fn measure(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        // Measure the full assembled text
        let frame = self.spinner.render_at(0.0, Some(0.0), self.spinner_style);
        let assembled = Text::assemble([
            crate::TextPart::Text(frame),
            crate::TextPart::Plain(" ".to_string()),
            crate::TextPart::Text(self.text.clone()),
        ]);
        assembled.measure(console, options)
    }
}


/// Internal state for Status that can be updated.
struct StatusState {
    spinner_name: String,
    spinner_style: Option<Style>,
    speed: f64,
    text: Text,
    start: Instant,
}

/// A status indicator with a spinner animation.
///
/// Status wraps a `Live` display with a `Spinner` and text message.
/// It's a convenience wrapper for showing progress on long-running operations.
///
/// # Example
///
/// ```no_run
/// use rich_rs::status::Status;
/// use std::thread::sleep;
/// use std::time::Duration;
///
/// let mut status = Status::new("[bold green]Working on tasks...");
/// status.start().unwrap();
///
/// for i in 0..10 {
///     sleep(Duration::from_secs(1));
///     println!("Task {} complete", i + 1);
/// }
///
/// status.stop().unwrap();
/// ```
pub struct Status {
    live: Live,
    state: Arc<Mutex<StatusState>>,
}

impl Status {
    /// Create a new Status with the given status text.
    ///
    /// The text can include markup like "[bold green]Working...".
    /// Uses the default "dots" spinner.
    pub fn new(status: &str) -> Self {
        Self::with_options(status, "dots", None, 1.0, 12.5)
    }

    /// Create a new Status with custom options.
    ///
    /// # Arguments
    ///
    /// * `status` - The status text (can include markup)
    /// * `spinner_name` - Name of the spinner animation (e.g., "dots", "earth", "bouncingBall")
    /// * `spinner_style` - Optional style for the spinner animation
    /// * `speed` - Speed factor for the animation (1.0 = normal)
    /// * `refresh_per_second` - How often to refresh the display
    pub fn with_options(
        status: &str,
        spinner_name: &str,
        spinner_style: Option<Style>,
        speed: f64,
        refresh_per_second: f64,
    ) -> Self {
        let text = Text::from_markup(status, false).unwrap_or_else(|_| Text::plain(status));
        let spinner = Spinner::new(spinner_name)
            .unwrap_or_else(|_| Spinner::new("dots").expect("dots spinner must exist"))
            .with_speed(speed);

        let start = Instant::now();
        let state = Arc::new(Mutex::new(StatusState {
            spinner_name: spinner_name.to_string(),
            spinner_style,
            speed,
            text: text.clone(),
            start,
        }));

        let renderable = StatusRenderable::new(spinner, text, spinner_style);

        let live_options = LiveOptions {
            refresh_per_second,
            transient: true,
            ..Default::default()
        };

        let live = Live::with_options(Box::new(renderable), live_options);

        Status { live, state }
    }

    /// Set the spinner name.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rich_rs::status::Status;
    ///
    /// let status = Status::new("Working...")
    ///     .spinner("earth");
    /// ```
    pub fn spinner(mut self, name: &str) -> Self {
        {
            let mut state = self.state.lock().expect("state mutex poisoned");
            state.spinner_name = name.to_string();
        }
        self.rebuild_renderable();
        self
    }

    /// Set the spinner style.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rich_rs::status::Status;
    /// use rich_rs::Style;
    ///
    /// let status = Status::new("Working...")
    ///     .spinner_style(Style::new().with_bold(true));
    /// ```
    pub fn spinner_style(mut self, style: Style) -> Self {
        {
            let mut state = self.state.lock().expect("state mutex poisoned");
            state.spinner_style = Some(style);
        }
        self.rebuild_renderable();
        self
    }

    /// Set the animation speed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rich_rs::status::Status;
    ///
    /// let status = Status::new("Working...")
    ///     .speed(2.0); // 2x speed
    /// ```
    pub fn speed(mut self, speed: f64) -> Self {
        {
            let mut state = self.state.lock().expect("state mutex poisoned");
            state.speed = speed;
        }
        self.rebuild_renderable();
        self
    }

    /// Start the status display.
    ///
    /// This shows the spinner and status text, and begins the refresh loop.
    pub fn start(&mut self) -> io::Result<()> {
        self.live.start(true)
    }

    /// Stop the status display.
    ///
    /// This hides the spinner and status text (since transient=true by default).
    pub fn stop(&mut self) -> io::Result<()> {
        self.live.stop()
    }

    /// Update the status text.
    ///
    /// # Arguments
    ///
    /// * `status` - New status text (can include markup)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rich_rs::status::Status;
    ///
    /// let mut status = Status::new("Starting...");
    /// status.start().unwrap();
    /// // ... do some work ...
    /// status.update("[bold blue]Processing...");
    /// // ... do more work ...
    /// status.stop().unwrap();
    /// ```
    pub fn update(&mut self, status: &str) -> io::Result<()> {
        let text = Text::from_markup(status, false).unwrap_or_else(|_| Text::plain(status));
        {
            let mut state = self.state.lock().expect("state mutex poisoned");
            state.text = text;
        }
        self.rebuild_and_update()
    }

    /// Update the status with new text and optionally change spinner settings.
    ///
    /// # Arguments
    ///
    /// * `status` - Optional new status text
    /// * `spinner` - Optional new spinner name
    /// * `spinner_style` - Optional new spinner style
    /// * `speed` - Optional new animation speed
    pub fn update_full(
        &mut self,
        status: Option<&str>,
        spinner: Option<&str>,
        spinner_style: Option<Style>,
        speed: Option<f64>,
    ) -> io::Result<()> {
        {
            let mut state = self.state.lock().expect("state mutex poisoned");
            if let Some(s) = status {
                state.text = Text::from_markup(s, false).unwrap_or_else(|_| Text::plain(s));
            }
            if let Some(name) = spinner {
                state.spinner_name = name.to_string();
                // Reset start time when changing spinner
                state.start = Instant::now();
            }
            if let Some(style) = spinner_style {
                state.spinner_style = Some(style);
            }
            if let Some(sp) = speed {
                state.speed = sp;
            }
        }
        self.rebuild_and_update()
    }

    /// Check if the status display is currently running.
    pub fn is_started(&self) -> bool {
        self.live.is_started()
    }

    /// Access the current status text.
    pub fn status_text(&self) -> Text {
        let state = self.state.lock().expect("state mutex poisoned");
        state.text.clone()
    }

    fn rebuild_renderable(&mut self) {
        let state = self.state.lock().expect("state mutex poisoned");
        let spinner = Spinner::new(&state.spinner_name)
            .unwrap_or_else(|_| Spinner::new("dots").expect("dots spinner must exist"))
            .with_speed(state.speed);
        let renderable = StatusRenderable {
            spinner,
            text: state.text.clone(),
            start: state.start,
            spinner_style: state.spinner_style,
        };
        drop(state);

        // Note: We can't update the live renderable during builder pattern
        // because Live::update requires &self, but we only have &mut self.
        // The builder pattern calls this before start(), so the pending_renderable
        // in Live will be replaced when we call start().
        let _ = self.live.update(Box::new(renderable), false);
    }

    fn rebuild_and_update(&mut self) -> io::Result<()> {
        let state = self.state.lock().expect("state mutex poisoned");
        let spinner = Spinner::new(&state.spinner_name)
            .unwrap_or_else(|_| Spinner::new("dots").expect("dots spinner must exist"))
            .with_speed(state.speed);
        let renderable = StatusRenderable {
            spinner,
            text: state.text.clone(),
            start: state.start,
            spinner_style: state.spinner_style,
        };
        drop(state);

        self.live.update(Box::new(renderable), true)
    }
}

impl Renderable for Status {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let state = self.state.lock().expect("state mutex poisoned");
        let spinner = Spinner::new(&state.spinner_name)
            .unwrap_or_else(|_| Spinner::new("dots").expect("dots spinner must exist"))
            .with_speed(state.speed);
        let elapsed = state.start.elapsed().as_secs_f64();
        let frame = spinner.render_at(elapsed, Some(0.0), state.spinner_style);

        let assembled = Text::assemble([
            crate::TextPart::Text(frame),
            crate::TextPart::Plain(" ".to_string()),
            crate::TextPart::Text(state.text.clone()),
        ]);
        assembled.render(console, options)
    }

    fn measure(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        Measurement::from_segments(&self.render(console, options))
    }
}

impl Drop for Status {
    fn drop(&mut self) {
        // Live's drop already calls stop(), but we call it explicitly for clarity
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_creation() {
        let status = Status::new("Working...");
        assert!(!status.is_started());
    }

    #[test]
    fn test_status_builder() {
        let status = Status::new("Working...").spinner("earth").speed(2.0);
        assert!(!status.is_started());
    }

    #[test]
    fn test_status_with_options() {
        let status = Status::with_options(
            "[bold green]Working...",
            "dots",
            Some(Style::new().with_bold(true)),
            1.5,
            10.0,
        );
        assert!(!status.is_started());
    }
}
