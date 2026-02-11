//! Spinner: simple terminal spinner animations (used by Progress).
//!
//! Port of Python Rich's `spinner.py` / `_spinners.py` (full catalog).
//!
//! The spinner frame data is generated from the locally installed Python Rich
//! (`rich/_spinners.py`) via `tools/generate_spinner_data.py` and committed as
//! `src/spinner_data.inc.rs`.

use std::io::Stdout;
use std::time::Instant;

use crate::console::{Console, ConsoleOptions};
use crate::measure::Measurement;
use crate::segment::Segments;
use crate::style::Style;
use crate::text::{Text, TextPart};
use crate::Renderable;

#[derive(Debug, Clone)]
pub struct SpinnerDef {
    pub frames: &'static [&'static str],
    /// Interval between frames in milliseconds.
    pub interval_ms: u64,
}

include!("spinner_data.inc.rs");

#[derive(Debug, Clone)]
pub struct Spinner {
    name: String,
    frames: Vec<&'static str>,
    interval_ms: u64,
    style: Option<Style>,
    speed: f64,
    start: Instant,
    text: Option<Text>,
}

impl Spinner {
    pub fn new(name: &str) -> Result<Self, String> {
        let def = get_spinner_def(name).ok_or_else(|| format!("no spinner called {name:?}"))?;
        Ok(Self {
            name: name.to_string(),
            frames: def.frames.to_vec(),
            interval_ms: def.interval_ms,
            style: None,
            speed: 1.0,
            start: Instant::now(),
            text: None,
        })
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    pub fn with_speed(mut self, speed: f64) -> Self {
        self.speed = speed;
        self
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(Text::plain(text.into()));
        self
    }

    /// Update the spinner's text, style, or speed after creation.
    pub fn update(
        &mut self,
        text: Option<String>,
        style: Option<Style>,
        speed: Option<f64>,
    ) {
        if let Some(t) = text {
            self.text = Some(Text::plain(t));
        }
        if let Some(s) = style {
            self.style = Some(s);
        }
        if let Some(sp) = speed {
            self.speed = sp;
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn interval_ms(&self) -> u64 {
        self.interval_ms
    }

    /// Render a spinner frame for a monotonic time.
    ///
    /// `time_seconds` is typically a time offset from a shared epoch, so multiple
    /// spinners can stay in sync (matches Rich's usage via `task.get_time()`).
    pub fn render_at(
        &self,
        time_seconds: f64,
        start_time_seconds: Option<f64>,
        style: Option<Style>,
    ) -> Text {
        let start = start_time_seconds.unwrap_or(time_seconds);
        let interval = (self.interval_ms as f64) / 1000.0;
        let frame_no = ((time_seconds - start) * self.speed) / interval;
        let frame = self.frames[(frame_no as usize) % self.frames.len()];
        match style.or(self.style) {
            Some(s) => Text::styled(frame, s),
            None => Text::plain(frame),
        }
    }

    /// Render using this spinner's internal start time (for standalone use).
    pub fn render_frame(&self) -> Text {
        let elapsed = self.start.elapsed().as_secs_f64();
        self.render_at(elapsed, Some(0.0), self.style)
    }

    /// Access the text displayed alongside the spinner.
    pub fn text(&self) -> Option<&Text> {
        self.text.as_ref()
    }
}

impl Renderable for Spinner {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let frame_text = self.render_frame();
        if let Some(ref text) = self.text {
            // Compose: spinner frame + " " + text
            let assembled = Text::assemble([
                TextPart::Text(frame_text),
                TextPart::Plain(" ".to_string()),
                TextPart::Text(text.clone()),
            ]);
            assembled.render(console, options)
        } else {
            frame_text.render(console, options)
        }
    }

    fn measure(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        let frame_text = self.render_frame();
        if let Some(ref text) = self.text {
            let assembled = Text::assemble([
                TextPart::Text(frame_text),
                TextPart::Plain(" ".to_string()),
                TextPart::Text(text.clone()),
            ]);
            assembled.measure(console, options)
        } else {
            frame_text.measure(console, options)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_catalog_contains_rich_default() {
        let spinner = Spinner::new("dots").unwrap();
        assert_eq!(spinner.frame_count(), 10);
        assert_eq!(spinner.interval_ms(), 80);
    }

    #[test]
    fn test_spinner_catalog_count_matches_rich() {
        // Generated from rich/_spinners.py at generation time.
        assert_eq!(spinner_names().len(), 73);
    }

    #[test]
    fn test_spinner_unknown_errors() {
        assert!(Spinner::new("not-a-spinner").is_err());
    }

    #[test]
    fn test_spinner_with_text() {
        let spinner = Spinner::new("dots").unwrap().with_text("Loading...");
        assert!(spinner.text().is_some());
        assert_eq!(spinner.text().unwrap().plain_text(), "Loading...");
    }

    #[test]
    fn test_spinner_update() {
        let mut spinner = Spinner::new("dots").unwrap();
        assert!(spinner.text().is_none());

        spinner.update(Some("Working".to_string()), None, Some(2.0));
        assert!(spinner.text().is_some());
        assert_eq!(spinner.text().unwrap().plain_text(), "Working");
    }

    #[test]
    fn test_spinner_render_frame() {
        let spinner = Spinner::new("dots").unwrap();
        let frame = spinner.render_frame();
        assert!(!frame.plain_text().is_empty());
    }

    #[test]
    fn test_spinner_renderable() {
        let spinner = Spinner::new("dots").unwrap().with_text("Test");
        let console = Console::new();
        let options = ConsoleOptions::default();
        let segments = Renderable::render(&spinner, &console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        assert!(output.contains("Test"));
    }
}
