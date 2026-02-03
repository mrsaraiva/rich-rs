//! Control: terminal control codes as a renderable.
//!
//! This is a small subset of Python Rich's `Control` used by Live / Progress.

use crate::segment::{ControlType, Segment, Segments};
use crate::{Console, ConsoleOptions, Measurement, Renderable};

#[derive(Debug, Clone, Default)]
pub struct Control {
    controls: Vec<ControlType>,
}

impl Control {
    pub fn new() -> Self {
        Self {
            controls: Vec::new(),
        }
    }

    pub fn home() -> Self {
        Self {
            controls: vec![ControlType::Home],
        }
    }

    pub fn carriage_return() -> Self {
        Self {
            controls: vec![ControlType::CarriageReturn],
        }
    }

    pub fn erase_in_line(mode: u8) -> Self {
        Self {
            controls: vec![ControlType::EraseInLine(mode)],
        }
    }

    pub fn cursor_up(n: u16) -> Self {
        Self {
            controls: vec![ControlType::CursorUp(n)],
        }
    }

    pub fn move_to(x: u16, y: u16) -> Self {
        Self {
            controls: vec![ControlType::MoveTo { x, y }],
        }
    }

    pub fn extend(&mut self, controls: impl IntoIterator<Item = ControlType>) {
        self.controls.extend(controls);
    }

    pub fn push(&mut self, control: ControlType) {
        self.controls.push(control);
    }

    pub fn is_empty(&self) -> bool {
        self.controls.is_empty()
    }

    pub fn into_segments(self) -> Segments {
        Segments::from_iter(self.controls.into_iter().map(Segment::control))
    }
}

impl Renderable for Control {
    fn render(&self, _console: &Console, _options: &ConsoleOptions) -> Segments {
        Segments::from_iter(self.controls.iter().cloned().map(Segment::control))
    }

    fn measure(&self, _console: &Console, _options: &ConsoleOptions) -> Measurement {
        Measurement::new(0, 0)
    }
}
