//! ScreenBuffer: a 2D grid of styled cells plus a diff algorithm.
//!
//! Rich itself doesn't expose a public "cell buffer" in the same way Textual does, but a
//! screen buffer + diff is a foundational building block for future TUIs.
//!
//! This module provides:
//! - `Cell` and `ScreenBuffer` (width × height grid)
//! - Conversion from rendered lines / segments into a `ScreenBuffer`
//! - A `diff_to_segments` method that produces terminal controls + styled text segments
//!   to update one buffer into another (cursor-safe, no newlines).

use crate::cells::char_width;
use crate::segment::{ControlType, Segment, Segments};
use crate::style::Style;
use crate::{Console, ConsoleOptions, Renderable};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// Text to print at this cell (may be empty for wide continuations).
    pub text: String,
    /// Style for this cell.
    pub style: Option<Style>,
    /// True if this cell is the trailing continuation of a wide glyph.
    pub continuation: bool,
}

impl Cell {
    pub fn blank(style: Option<Style>) -> Self {
        Self {
            text: " ".to_string(),
            style,
            continuation: false,
        }
    }

    pub fn continuation(style: Option<Style>) -> Self {
        Self {
            text: String::new(),
            style,
            continuation: true,
        }
    }

    pub fn width(&self) -> usize {
        if self.continuation {
            0
        } else {
            crate::cell_len(&self.text)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenBuffer {
    pub width: usize,
    pub height: usize,
    default_style: Option<Style>,
    cells: Vec<Cell>,
}

impl ScreenBuffer {
    pub fn new(width: usize, height: usize, style: Option<Style>) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        Self {
            width,
            height,
            default_style: style,
            cells: vec![Cell::blank(style); width * height],
        }
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn get(&self, x: usize, y: usize) -> &Cell {
        &self.cells[self.idx(x, y)]
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> &mut Cell {
        let idx = self.idx(x, y);
        &mut self.cells[idx]
    }

    pub fn as_plain_lines(&self) -> Vec<String> {
        let mut lines = Vec::with_capacity(self.height);
        for y in 0..self.height {
            let mut line = String::new();
            for x in 0..self.width {
                let cell = self.get(x, y);
                if cell.continuation {
                    continue;
                }
                if cell.text.is_empty() {
                    line.push(' ');
                } else {
                    line.push_str(&cell.text);
                }
            }
            lines.push(crate::cells::set_cell_size(&line, self.width));
        }
        lines
    }

    /// Render a renderable to a ScreenBuffer.
    ///
    /// This uses `Console::render_lines` and then converts the rendered lines to cells.
    pub fn from_renderable(
        console: &Console,
        options: &ConsoleOptions,
        renderable: &dyn Renderable,
        style: Option<Style>,
    ) -> Self {
        let (width, height) = options.size;
        let lines = console.render_lines(renderable, Some(options), style, true, false);
        let lines = Segment::set_shape(&lines, width, Some(height), style, false);
        Self::from_lines(&lines, width, height, style)
    }

    /// Build a ScreenBuffer from pre-rendered lines.
    ///
    /// The caller is expected to provide lines already padded/cropped to `width` × `height`.
    pub fn from_lines(
        lines: &[Vec<Segment>],
        width: usize,
        height: usize,
        default_style: Option<Style>,
    ) -> Self {
        let mut buffer = ScreenBuffer::new(width, height, default_style);

        for (y, line) in lines.iter().take(height).enumerate() {
            buffer.write_line(y, line);
        }

        buffer
    }

    fn clear_line(&mut self, y: usize) {
        for x in 0..self.width {
            *self.get_mut(x, y) = Cell::blank(self.default_style);
        }
    }

    fn write_line(&mut self, y: usize, line: &[Segment]) {
        if y >= self.height {
            return;
        }
        self.clear_line(y);

        let mut x: usize = 0;
        let mut last_non_zero: Option<(usize, usize)> = None; // (x, width)

        for seg in line {
            if seg.control.is_some() {
                continue;
            }
            let style = seg.style;
            for ch in seg.text.chars() {
                let w = char_width(ch);

                if w == 0 {
                    // Combine with previous cell, if any.
                    if let Some((prev_x, prev_w)) = last_non_zero {
                        let cell = self.get_mut(prev_x, y);
                        cell.text.push(ch);
                        // Keep style from the segment currently being processed to match Rich behavior
                        // for combining marks following styled text.
                        cell.style = style;
                        // If previous glyph was wide, combining marks should still attach to the start.
                        last_non_zero = Some((prev_x, prev_w));
                    }
                    continue;
                }

                if x >= self.width {
                    return;
                }

                if w == 2 && x + 1 >= self.width {
                    // Can't place a wide glyph in the last column; fall back to a space.
                    *self.get_mut(x, y) = Cell::blank(style);
                    x += 1;
                    last_non_zero = Some((x.saturating_sub(1), 1));
                    continue;
                }

                *self.get_mut(x, y) = Cell {
                    text: ch.to_string(),
                    style,
                    continuation: false,
                };
                last_non_zero = Some((x, w));

                if w == 2 {
                    *self.get_mut(x + 1, y) = Cell::continuation(style);
                    x += 2;
                } else {
                    x += 1;
                }
            }
        }
    }

    fn cell_span_width(&self, x: usize, y: usize) -> usize {
        let cell = self.get(x, y);
        if cell.continuation {
            0
        } else {
            let w = cell.width();
            if w == 0 {
                1
            } else {
                w
            }
        }
    }

    /// Compute an update sequence that transforms `previous` into `self`.
    ///
    /// The returned segments:
    /// - Start with `Home` (cursor to 0,0)
    /// - Use cursor controls (no `\n`) for positioning
    /// - Emit styled text for changed spans
    pub fn diff_to_segments(&self, previous: &ScreenBuffer) -> Segments {
        assert_eq!(self.width, previous.width, "buffer widths differ");
        assert_eq!(self.height, previous.height, "buffer heights differ");

        let mut out = Segments::new();
        out.push(Segment::control(ControlType::Home));

        let mut cursor_x: usize = 0;
        let mut cursor_y: usize = 0;

        for y in 0..self.height {
            let mut x: usize = 0;

            while x < self.width {
                let curr = self.get(x, y);
                let prev = previous.get(x, y);

                // Never start updates on continuation cells.
                if curr.continuation || prev.continuation {
                    x += 1;
                    continue;
                }

                if curr == prev {
                    x += 1;
                    continue;
                }

                let mut span = self.cell_span_width(x, y).max(previous.cell_span_width(x, y)).max(1);
                span = span.min(self.width.saturating_sub(x));

                // Extend span over subsequent differing cells.
                let mut end_x = x + span;
                while end_x < self.width {
                    let c = self.get(end_x, y);
                    let p = previous.get(end_x, y);
                    if c.continuation || p.continuation {
                        end_x += 1;
                        continue;
                    }
                    if c == p {
                        break;
                    }
                    let extra = self
                        .cell_span_width(end_x, y)
                        .max(previous.cell_span_width(end_x, y))
                        .max(1);
                    end_x = (end_x + extra).min(self.width);
                }

                // Move cursor to (x, y)
                if y != cursor_y {
                    if y > cursor_y {
                        out.push(Segment::control(ControlType::CursorDown((y - cursor_y) as u16)));
                    } else {
                        out.push(Segment::control(ControlType::CursorUp((cursor_y - y) as u16)));
                    }
                    cursor_y = y;
                    cursor_x = 0;
                    out.push(Segment::control(ControlType::CarriageReturn));
                }

                if x != cursor_x {
                    // Normalize to start-of-line then move forward.
                    out.push(Segment::control(ControlType::CarriageReturn));
                    cursor_x = 0;
                    if x > 0 {
                        out.push(Segment::control(ControlType::CursorForward(x as u16)));
                        cursor_x = x;
                    }
                }

                // Emit the updated span as styled segments.
                let mut run_x = x;
                while run_x < end_x {
                    let cell = self.get(run_x, y);
                    if cell.continuation {
                        run_x += 1;
                        continue;
                    }
                    let w = self.cell_span_width(run_x, y).max(1);
                    let text = if cell.text.is_empty() { " ".to_string() } else { cell.text.clone() };
                    let mut seg = Segment::new(text);
                    seg.style = cell.style;
                    out.push(seg);
                    cursor_x += w;
                    run_x += w;
                }

                x = end_x;
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Text;

    fn apply_segments(mut buffer: ScreenBuffer, segments: &Segments) -> ScreenBuffer {
        let mut x: usize = 0;
        let mut y: usize = 0;
        let mut last_non_zero: Option<(usize, usize)> = None; // (x, width)

        let width = buffer.width;
        let height = buffer.height;

        for seg in segments.iter() {
            if let Some(ctrl) = &seg.control {
                match ctrl {
                    ControlType::Home => {
                        x = 0;
                        y = 0;
                    }
                    ControlType::CarriageReturn => x = 0,
                    ControlType::CursorUp(n) => y = y.saturating_sub(*n as usize),
                    ControlType::CursorDown(n) => y = (y + *n as usize).min(height.saturating_sub(1)),
                    ControlType::CursorForward(n) => x = (x + *n as usize).min(width.saturating_sub(1)),
                    ControlType::CursorBackward(n) => x = x.saturating_sub(*n as usize),
                    _ => {}
                }
                continue;
            }

            for ch in seg.text.chars() {
                let w = char_width(ch);
                if x >= width || y >= height {
                    break;
                }

                if w == 0 {
                    if let Some((prev_x, prev_w)) = last_non_zero {
                        let cell = buffer.get_mut(prev_x, y);
                        cell.text.push(ch);
                        cell.style = seg.style;
                        last_non_zero = Some((prev_x, prev_w));
                    }
                    continue;
                }

                if w == 2 && x + 1 >= width {
                    break;
                }
                *buffer.get_mut(x, y) = Cell {
                    text: ch.to_string(),
                    style: seg.style,
                    continuation: false,
                };
                last_non_zero = Some((x, w));
                if w == 2 {
                    *buffer.get_mut(x + 1, y) = Cell::continuation(seg.style);
                    x += 2;
                } else {
                    x += 1;
                }
            }
        }

        buffer
    }

    #[test]
    fn test_screen_buffer_from_renderable_plain() {
        let console = Console::new();
        let mut options = console.options().clone();
        options.size = (5, 2);
        options.max_width = 5;
        options.max_height = 2;

        let buf = ScreenBuffer::from_renderable(&console, &options, &Text::plain("hi"), None);
        assert_eq!(buf.as_plain_lines()[0], "hi   ");
        assert_eq!(buf.as_plain_lines()[1], "     ");
    }

    #[test]
    fn test_screen_buffer_diff_applies() {
        let console = Console::new();
        let mut options = console.options().clone();
        options.size = (10, 3);
        options.max_width = 10;
        options.max_height = 3;

        let prev = ScreenBuffer::from_renderable(&console, &options, &Text::plain("A"), None);
        let next = ScreenBuffer::from_renderable(&console, &options, &Text::plain("B"), None);

        let diff = next.diff_to_segments(&prev);
        let applied = apply_segments(prev.clone(), &diff);
        assert_eq!(applied, next);
    }

    #[test]
    fn test_screen_buffer_diff_handles_wide_char() {
        let console = Console::new();
        let mut options = console.options().clone();
        options.size = (6, 1);
        options.max_width = 6;
        options.max_height = 1;

        // Wide CJK character (2 cells)
        let prev = ScreenBuffer::from_renderable(&console, &options, &Text::plain("你"), None);
        let next = ScreenBuffer::from_renderable(&console, &options, &Text::plain("a"), None);

        let diff = next.diff_to_segments(&prev);
        let applied = apply_segments(prev.clone(), &diff);
        assert_eq!(applied, next);
    }

    #[test]
    fn test_screen_buffer_diff_uses_no_newlines() {
        let console = Console::new();
        let mut options = console.options().clone();
        options.size = (10, 2);
        options.max_width = 10;
        options.max_height = 2;

        let prev = ScreenBuffer::from_renderable(&console, &options, &Text::plain("A"), None);
        let next = ScreenBuffer::from_renderable(&console, &options, &Text::plain("B"), None);
        let diff = next.diff_to_segments(&prev);

        assert!(diff.iter().all(|s| !s.text.contains('\n')));
    }
}
