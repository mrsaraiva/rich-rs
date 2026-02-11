//! Spinners example - Display all available spinners.
//!
//! This is a port of Python Rich's `examples/spinners.py`.
//!
//! Run with: `cargo run --example spinners`

use std::io::Stdout;
use std::thread::sleep;
use std::time::{Duration, Instant};

use rich_rs::{
    Columns, Console, ConsoleOptions, Live, LiveOptions, Measurement, Panel, Renderable, Segments,
    Spinner, Style, Text,
};

// List of spinner names (from rich_rs::spinner_data.inc.rs, which is pub(crate))
// These are the same spinners available in Python Rich
const SPINNER_NAMES: &[&str] = &[
    "aesthetic",
    "arc",
    "arrow",
    "arrow2",
    "arrow3",
    "balloon",
    "balloon2",
    "betaWave",
    "bounce",
    "bouncingBall",
    "bouncingBar",
    "boxBounce",
    "boxBounce2",
    "christmas",
    "circle",
    "circleHalves",
    "circleQuarters",
    "clock",
    "dots",
    "dots10",
    "dots11",
    "dots12",
    "dots2",
    "dots3",
    "dots4",
    "dots5",
    "dots6",
    "dots7",
    "dots8",
    "dots8Bit",
    "dots9",
    "dqpb",
    "earth",
    "fingerDance",
    "fistBump",
    "flip",
    "grenade",
    "growHorizontal",
    "growVertical",
    "hamburger",
    "hearts",
    "layer",
    "line",
    "line2",
    "material",
    "mindblown",
    "monkey",
    "moon",
    "noise",
    "orangeBluePulse",
    "orangePulse",
    "pipe",
    "point",
    "pong",
    "runner",
    "shark",
    "simpleDots",
    "simpleDotsScrolling",
    "smiley",
    "speaker",
    "squareCorners",
    "squish",
    "star",
    "star2",
    "toggle",
    "toggle10",
    "toggle11",
    "toggle12",
    "toggle13",
    "toggle2",
    "toggle3",
    "toggle4",
    "toggle5",
    "toggle6",
    "toggle7",
    "toggle8",
    "toggle9",
    "weather",
];

/// A renderable that displays a single spinner with its name.
struct SpinnerDisplay {
    spinner: Spinner,
    name: String,
    start: Instant,
}

impl SpinnerDisplay {
    fn new(name: &str) -> Result<Self, String> {
        Ok(Self {
            spinner: Spinner::new(name)?,
            name: name.to_string(),
            start: Instant::now(),
        })
    }
}

impl Renderable for SpinnerDisplay {
    fn render(&self, _console: &Console<Stdout>, _options: &ConsoleOptions) -> Segments {
        let elapsed = self.start.elapsed().as_secs_f64();
        let frame = self.spinner.render_at(elapsed, Some(0.0), None);

        // Create text: "spinner_frame 'spinner_name'"
        let mut result = Segments::new();
        result.extend(frame.render(_console, _options));
        result.push(rich_rs::Segment::new(" "));

        let name_text = Text::styled(
            &format!("'{}'", self.name),
            Style::new().with_color(rich_rs::SimpleColor::Standard(2)),
        );
        result.extend(name_text.render(_console, _options));

        result
    }

    fn measure(&self, _console: &Console<Stdout>, _options: &ConsoleOptions) -> Measurement {
        // Approximate width: spinner (2) + space (1) + quotes (2) + name length
        let width = 3 + 2 + self.name.len();
        Measurement::new(width, width)
    }
}

/// A renderable that displays all spinners in columns.
struct AllSpinners {
    names: Vec<String>,
}

impl AllSpinners {
    fn new() -> Self {
        let mut names: Vec<String> = SPINNER_NAMES
            .iter()
            .filter(|name| Spinner::new(name).is_ok())
            .map(|s| s.to_string())
            .collect();

        // Sort by name for display
        names.sort();

        Self { names }
    }
}

impl Renderable for AllSpinners {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let spinner_displays: Vec<Box<dyn Renderable + Send + Sync>> = self
            .names
            .iter()
            .filter_map(|name| {
                SpinnerDisplay::new(name)
                    .ok()
                    .map(|d| Box::new(d) as Box<dyn Renderable + Send + Sync>)
            })
            .collect();

        let columns = Columns::new(spinner_displays)
            .with_column_first(true)
            .with_expand(true);

        columns.render(console, options)
    }

    fn measure(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        let spinner_displays: Vec<Box<dyn Renderable + Send + Sync>> = self
            .names
            .iter()
            .filter_map(|name| {
                SpinnerDisplay::new(name)
                    .ok()
                    .map(|d| Box::new(d) as Box<dyn Renderable + Send + Sync>)
            })
            .collect();

        let columns = Columns::new(spinner_displays)
            .with_column_first(true)
            .with_expand(true);

        columns.measure(console, options)
    }
}

fn main() -> std::io::Result<()> {
    let all_spinners = AllSpinners::new();

    let panel = Panel::new(Box::new(all_spinners))
        .with_title("[b]Spinners")
        .with_border_style(Style::new().with_color(rich_rs::SimpleColor::Standard(4))); // blue

    let live_options = LiveOptions {
        refresh_per_second: 20.0,
        ..Default::default()
    };

    let mut live = Live::with_options(Box::new(panel), live_options);
    live.start(true)?;

    // Run for a while to show the spinners animating
    // In the Python example, it runs forever with `while True`
    // We'll run for 10 seconds to demonstrate (200 * 50ms = 10s)
    for _ in 0..200 {
        sleep(Duration::from_millis(50));

        // Recreate the panel with fresh spinners to get updated frames
        let all_spinners = AllSpinners::new();
        let panel = Panel::new(Box::new(all_spinners))
            .with_title("[b]Spinners")
            .with_border_style(Style::new().with_color(rich_rs::SimpleColor::Standard(4)));

        live.update(Box::new(panel), true)?;
    }

    live.stop()?;
    Ok(())
}
