//! Group: a renderable that renders multiple children sequentially.
//!
//! This is a Rust port of Python Rich's `rich.console.Group`.
//! In Rich, `Group(*renderables)` yields child renderables in sequence.
//!
//! In `rich-rs` we insert newlines between children when required so that
//! adjacent renderables don't accidentally render on the same terminal line.

use std::sync::Arc;

use crate::segment::{Segment, Segments};
use crate::{Console, ConsoleOptions, Measurement, Renderable, Text, measure_renderables};

/// A renderable that renders multiple children sequentially.
#[derive(Clone, Default)]
pub struct Group {
    renderables: Vec<Arc<dyn Renderable>>,
    fit: bool,
}

impl Group {
    /// Create a new Group from an iterator of renderables.
    pub fn new<I, R>(renderables: I) -> Self
    where
        I: IntoIterator<Item = R>,
        R: Renderable + 'static,
    {
        Self::from_arcs(
            renderables
                .into_iter()
                .map(|r| Arc::new(r) as Arc<dyn Renderable>),
        )
    }

    /// Create a Group from an iterator of `Arc<dyn Renderable>`.
    pub fn from_arcs<I>(renderables: I) -> Self
    where
        I: IntoIterator<Item = Arc<dyn Renderable>>,
    {
        Self {
            renderables: renderables.into_iter().collect(),
            fit: true,
        }
    }

    /// Set whether the Group should fit to its contents for measurement.
    ///
    /// When `fit` is false, measurement returns an exact width equal to the available width.
    pub fn with_fit(mut self, fit: bool) -> Self {
        self.fit = fit;
        self
    }

    /// Return the child renderables.
    pub fn renderables(&self) -> &[Arc<dyn Renderable>] {
        &self.renderables
    }
}

fn segments_end_with_newline(segments: &Segments) -> bool {
    // Find the last non-control segment with non-empty text.
    let rev: Vec<_> = segments.iter().collect();
    for seg in rev.into_iter().rev() {
        if seg.control.is_some() {
            continue;
        }
        if seg.text.is_empty() {
            continue;
        }
        let text: &str = seg.text.as_ref();
        return text.ends_with('\n') || text.ends_with("\n\r");
    }
    false
}

impl std::fmt::Debug for Group {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Group")
            .field("len", &self.renderables.len())
            .field("fit", &self.fit)
            .finish()
    }
}

impl Renderable for Group {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
        let mut out = Segments::new();
        let mut first = true;

        for child in &self.renderables {
            let segs = child.render(console, options);

            if !first && !segments_end_with_newline(&out) {
                out.push(Segment::line());
            }
            first = false;

            out.extend(segs.into_iter());
        }

        out
    }

    fn measure(&self, console: &Console, options: &ConsoleOptions) -> Measurement {
        if self.fit {
            let refs: Vec<&dyn Renderable> = self.renderables.iter().map(|r| r.as_ref()).collect();
            measure_renderables(console, options, &refs)
        } else {
            Measurement::exact(options.max_width)
        }
    }
}

// ============================================================================
// Renderables — like Group but without inserting newlines
// ============================================================================

/// A container that renders multiple children sequentially WITHOUT newlines.
///
/// Unlike `Group`, which inserts newlines between children, `Renderables`
/// yields each child's output directly, matching Python Rich's
/// `rich.containers.Renderables`.
#[derive(Clone, Default)]
pub struct Renderables {
    renderables: Vec<Arc<dyn Renderable>>,
}

impl Renderables {
    /// Create a new Renderables from an iterator of renderables.
    pub fn new<I, R>(renderables: I) -> Self
    where
        I: IntoIterator<Item = R>,
        R: Renderable + 'static,
    {
        Self {
            renderables: renderables
                .into_iter()
                .map(|r| Arc::new(r) as Arc<dyn Renderable>)
                .collect(),
        }
    }

    /// Create from an iterator of `Arc<dyn Renderable>`.
    pub fn from_arcs<I>(renderables: I) -> Self
    where
        I: IntoIterator<Item = Arc<dyn Renderable>>,
    {
        Self {
            renderables: renderables.into_iter().collect(),
        }
    }

    /// Create an empty Renderables.
    pub fn empty() -> Self {
        Self {
            renderables: Vec::new(),
        }
    }

    /// Add a renderable.
    pub fn push(&mut self, renderable: impl Renderable + 'static) {
        self.renderables.push(Arc::new(renderable));
    }

    /// Add a pre-wrapped Arc renderable.
    pub fn push_arc(&mut self, renderable: Arc<dyn Renderable>) {
        self.renderables.push(renderable);
    }

    /// Return the child renderables.
    pub fn renderables(&self) -> &[Arc<dyn Renderable>] {
        &self.renderables
    }
}

impl std::fmt::Debug for Renderables {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderables")
            .field("len", &self.renderables.len())
            .finish()
    }
}

impl Renderable for Renderables {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
        let mut out = Segments::new();
        for child in &self.renderables {
            let segs = child.render(console, options);
            out.extend(segs.into_iter());
        }
        out
    }

    fn measure(&self, console: &Console, options: &ConsoleOptions) -> Measurement {
        let refs: Vec<&dyn Renderable> = self.renderables.iter().map(|r| r.as_ref()).collect();
        if refs.is_empty() {
            return Measurement::new(1, 1);
        }
        measure_renderables(console, options, &refs)
    }
}

// ============================================================================
// Lines — a text line container with list-like API
// ============================================================================

/// A container of [`Text`] lines that renders one line per row.
///
/// This type is equivalent to Python Rich's `rich.containers.Lines`.
#[derive(Clone, Default)]
pub struct Lines {
    lines: Vec<Text>,
}

impl Lines {
    /// Create `Lines` from an iterator of [`Text`].
    pub fn new<I>(lines: I) -> Self
    where
        I: IntoIterator<Item = Text>,
    {
        Self {
            lines: lines.into_iter().collect(),
        }
    }

    /// Create an empty `Lines`.
    pub fn empty() -> Self {
        Self { lines: Vec::new() }
    }

    /// Append a line.
    pub fn append(&mut self, line: Text) {
        self.lines.push(line);
    }

    /// Extend with multiple lines.
    pub fn extend<I>(&mut self, lines: I)
    where
        I: IntoIterator<Item = Text>,
    {
        self.lines.extend(lines);
    }

    /// Pop the last line.
    pub fn pop(&mut self) -> Option<Text> {
        self.lines.pop()
    }

    /// Number of lines.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// True if there are no lines.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Borrow contained lines.
    pub fn lines(&self) -> &[Text] {
        &self.lines
    }
}

impl std::fmt::Debug for Lines {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lines")
            .field("len", &self.lines.len())
            .finish()
    }
}

impl Renderable for Lines {
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments {
        let mut out = Segments::new();
        let mut first = true;

        for line in &self.lines {
            let segs = line.render(console, options);

            if !first && !segments_end_with_newline(&out) {
                out.push(Segment::line());
            }
            first = false;

            out.extend(segs.into_iter());
        }

        out
    }

    fn measure(&self, console: &Console, options: &ConsoleOptions) -> Measurement {
        if self.lines.is_empty() {
            return Measurement::new(1, 1);
        }
        let refs: Vec<&dyn Renderable> = self
            .lines
            .iter()
            .map(|line| line as &dyn Renderable)
            .collect();
        measure_renderables(console, options, &refs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_renders_children_with_newlines() {
        let group = Group::new([Text::plain("A"), Text::plain("B")]);
        let console = Console::new();
        let options = console.options();
        let rendered: String = group
            .render(&console, options)
            .iter()
            .map(|s| s.text.to_string())
            .collect();
        assert!(rendered.contains('\n'));
    }

    #[test]
    fn test_group_measure_fit() {
        let group = Group::new([Text::plain("Hello"), Text::plain("World!")]).with_fit(true);
        let console = Console::new();
        let options = console.options();
        let m = group.measure(&console, options);
        assert!(m.maximum >= 6);
    }

    #[test]
    fn test_renderables_no_newlines() {
        let r = Renderables::new([Text::plain("A"), Text::plain("B")]);
        let console = Console::new();
        let options = console.options();
        let rendered: String = r
            .render(&console, options)
            .iter()
            .map(|s| s.text.to_string())
            .collect();
        // Renderables should NOT insert newlines between children
        assert!(rendered.contains("AB") || rendered.contains("A") && rendered.contains("B"));
        // Check there's no inserted newline between A and B
        let pos_a = rendered.find('A').unwrap();
        let pos_b = rendered.find('B').unwrap();
        let between = &rendered[pos_a + 1..pos_b];
        assert!(
            !between.contains('\n'),
            "Should not insert newline between children"
        );
    }

    #[test]
    fn test_renderables_measure() {
        let r = Renderables::new([Text::plain("Hello"), Text::plain("World!")]);
        let console = Console::new();
        let options = console.options();
        let m = r.measure(&console, options);
        assert!(m.maximum >= 6); // "World!" = 6 chars
    }

    #[test]
    fn test_renderables_empty() {
        let r = Renderables::empty();
        let console = Console::new();
        let options = console.options();
        let m = r.measure(&console, options);
        assert_eq!(m.minimum, 1);
        assert_eq!(m.maximum, 1);
    }

    #[test]
    fn test_group_measure_fill() {
        let group = Group::new([Text::plain("Hello")]).with_fit(false);
        let console = Console::new();
        let mut options = console.options().clone();
        options.max_width = 42;
        let m = group.measure(&console, &options);
        assert_eq!(m.minimum, 42);
        assert_eq!(m.maximum, 42);
    }

    #[test]
    fn test_lines_render_inserts_newlines_between_lines() {
        let lines = Lines::new([Text::plain("A"), Text::plain("B")]);
        let console = Console::new();
        let options = console.options();
        let rendered: String = lines
            .render(&console, options)
            .iter()
            .map(|s| s.text.to_string())
            .collect();
        assert_eq!(rendered, "A\nB");
    }

    #[test]
    fn test_lines_render_does_not_double_insert_newlines() {
        let lines = Lines::new([Text::plain("A\n"), Text::plain("B")]);
        let console = Console::new();
        let options = console.options();
        let rendered: String = lines
            .render(&console, options)
            .iter()
            .map(|s| s.text.to_string())
            .collect();
        assert_eq!(rendered, "A\nB");
    }

    #[test]
    fn test_lines_list_api() {
        let mut lines = Lines::empty();
        assert!(lines.is_empty());

        lines.append(Text::plain("A"));
        lines.extend([Text::plain("B"), Text::plain("C")]);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines.lines()[0].plain_text(), "A");
        assert_eq!(lines.lines()[2].plain_text(), "C");

        let popped = lines.pop().unwrap();
        assert_eq!(popped.plain_text(), "C");
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_lines_measure_uses_widest_line() {
        let lines = Lines::new([Text::plain("cat"), Text::plain("hippo")]);
        let console = Console::new();
        let options = console.options();
        let m = lines.measure(&console, options);
        assert_eq!(m.minimum, 5);
        assert_eq!(m.maximum, 5);
    }
}
