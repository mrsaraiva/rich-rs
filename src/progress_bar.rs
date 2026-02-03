//! ProgressBar: a progress bar renderable (used by Progress).
//!
//! Port of Python Rich's `progress_bar.py` (subset).

use std::f64::consts::PI;

use crate::color::{blend_rgb, Color, ColorSystem, ColorTriplet, SimpleColor};
use crate::console::ConsoleOptions;
use crate::measure::Measurement;
use crate::segment::{Segment, Segments};
use crate::style::Style;
use crate::Renderable;
use crate::{Console};

const PULSE_SIZE: usize = 20;

#[derive(Debug, Clone)]
pub struct ProgressBar {
    pub total: Option<f64>,
    pub completed: f64,
    pub width: Option<usize>,
    pub pulse: bool,
    pub style: String,
    pub complete_style: String,
    pub finished_style: String,
    pub pulse_style: String,
    pub animation_time: Option<f64>,
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self {
            total: Some(100.0),
            completed: 0.0,
            width: None,
            pulse: false,
            style: "bar.back".to_string(),
            complete_style: "bar.complete".to_string(),
            finished_style: "bar.finished".to_string(),
            pulse_style: "bar.pulse".to_string(),
            animation_time: None,
        }
    }
}

impl ProgressBar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, completed: f64, total: Option<f64>) {
        self.completed = completed;
        if let Some(t) = total {
            self.total = Some(t);
        }
    }

    fn resolve_style(options: &ConsoleOptions, name: &str) -> Style {
        options.get_style(name).unwrap_or_default()
    }

    fn truecolor_from_style(style: Style, foreground: bool) -> ColorTriplet {
        let simple = if foreground {
            style.color.unwrap_or(SimpleColor::Default)
        } else {
            style.bgcolor.unwrap_or(SimpleColor::Default)
        };
        Color::from(simple).get_truecolor(foreground)
    }

    fn get_pulse_segments(
        options: &ConsoleOptions,
        fore_style: Style,
        back_style: Style,
        ascii: bool,
    ) -> Vec<Segment> {
        let bar = if ascii { "-" } else { "━" };

        let color_system = options.color_system.unwrap_or(ColorSystem::Standard);
        let no_color = options.color_system.is_none();
        if !matches!(
            color_system,
            ColorSystem::Standard | ColorSystem::EightBit | ColorSystem::TrueColor
        ) || no_color
        {
            let mut segments: Vec<Segment> = Vec::new();
            segments.extend(std::iter::repeat(Segment::styled(bar, fore_style)).take(PULSE_SIZE / 2));
            let back_char = if no_color { " " } else { bar };
            segments.extend(std::iter::repeat(Segment::styled(back_char, back_style)).take(PULSE_SIZE - (PULSE_SIZE / 2)));
            return segments;
        }

        // Truecolor / indexed: compute a cosine blend across the pulse length.
        let fore_color = Self::truecolor_from_style(fore_style, true);
        let back_color = Self::truecolor_from_style(back_style, true);
        let mut segments: Vec<Segment> = Vec::with_capacity(PULSE_SIZE);
        for index in 0..PULSE_SIZE {
            let position = index as f64 / (PULSE_SIZE - 1).max(1) as f64;
            let fade = (1.0 - (position * 2.0 - 1.0).abs()).powf(1.0);
            let cross_fade = (fade * PI).cos() * -0.5 + 0.5;
            let blended = blend_rgb(fore_color, back_color, cross_fade);
            let style = Style::new().with_color(SimpleColor::Rgb {
                r: blended.red,
                g: blended.green,
                b: blended.blue,
            });
            segments.push(Segment::styled(bar, style));
        }
        segments
    }

    fn render_pulse(&self, options: &ConsoleOptions, width: usize, ascii: bool) -> Segments {
        let fore_style = Self::resolve_style(options, &self.pulse_style);
        let back_style = Self::resolve_style(options, &self.style);
        let pulse_segments = Self::get_pulse_segments(options, fore_style, back_style, ascii);
        let segment_count = pulse_segments.len().max(1);

        let current_time = self.animation_time.unwrap_or(0.0);
        let mut segments: Vec<Segment> = Vec::new();
        let repeats = (width / segment_count) + 2;
        for _ in 0..repeats {
            segments.extend(pulse_segments.iter().cloned());
        }
        let offset = ((-current_time * 15.0) as isize).rem_euclid(segment_count as isize) as usize;
        let slice = &segments[offset..offset + width];
        Segments::from_iter(slice.iter().cloned())
    }
}

impl Renderable for ProgressBar {
    fn render(&self, _console: &Console, options: &ConsoleOptions) -> Segments {
        let width = (self.width.unwrap_or(options.max_width)).min(options.max_width).max(1);
        let ascii = options.legacy_windows || !options.encoding.to_lowercase().starts_with("utf");
        let should_pulse = self.pulse || self.total.is_none();
        if should_pulse {
            return self.render_pulse(options, width, ascii);
        }

        let total = self.total.unwrap_or(0.0);
        let completed = self.completed.clamp(0.0, total.max(0.0));

        let bar = if ascii { "-" } else { "━" };
        let half_bar_right = if ascii { " " } else { "╸" };
        let half_bar_left = if ascii { " " } else { "╺" };

        let complete_halves = if total > 0.0 {
            ((width as f64) * 2.0 * completed / total).floor() as usize
        } else {
            width * 2
        };
        let bar_count = complete_halves / 2;
        let half_bar_count = complete_halves % 2;

        let style = Self::resolve_style(options, &self.style);
        let is_finished = self.total.is_none() || self.completed >= total;
        let complete_style = Self::resolve_style(
            options,
            if is_finished {
                &self.finished_style
            } else {
                &self.complete_style
            },
        );

        let mut out = Segments::new();
        if bar_count > 0 {
            out.push(Segment::styled(bar.repeat(bar_count), complete_style));
        }
        if half_bar_count > 0 {
            out.push(Segment::styled(half_bar_right.to_string(), complete_style));
        }

        // Match Rich: when colors are disabled, fill the remainder with unstyled spaces
        // (so the bar doesn't "bleed" style into the padded region).
        if options.color_system.is_none() {
            let remaining = width.saturating_sub(bar_count + half_bar_count);
            if remaining > 0 {
                out.push(Segment::new(" ".repeat(remaining)));
            }
            return out;
        }

        // In Rich, remaining bars are only drawn if colors are enabled.
        if options.color_system.is_some() {
            let mut remaining_bars = width.saturating_sub(bar_count + half_bar_count);
            if remaining_bars > 0 && half_bar_count == 0 && bar_count > 0 {
                out.push(Segment::styled(half_bar_left.to_string(), style));
                remaining_bars = remaining_bars.saturating_sub(1);
            }
            if remaining_bars > 0 {
                out.push(Segment::styled(bar.repeat(remaining_bars), style));
            }
        }

        out
    }

    fn measure(&self, _console: &Console, options: &ConsoleOptions) -> Measurement {
        if let Some(w) = self.width {
            Measurement::new(w, w)
        } else {
            Measurement::new(4, options.max_width)
        }
    }
}
