//! Group: a renderable that renders multiple children sequentially.
//!
//! This is a Rust port of Python Rich's `rich.console.Group`.
//! In Rich, `Group(*renderables)` yields child renderables in sequence.
//!
//! In `rich-rs` we insert newlines between children when required so that
//! adjacent renderables don't accidentally render on the same terminal line.

use std::sync::Arc;

use crate::segment::{Segment, Segments};
use crate::{Console, ConsoleOptions, Measurement, Renderable, measure_renderables};

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
        Self::from_arcs(renderables.into_iter().map(|r| Arc::new(r) as Arc<dyn Renderable>))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Text;

    #[test]
    fn test_group_renders_children_with_newlines() {
        let group = Group::new([Text::plain("A"), Text::plain("B")]);
        let console = Console::new();
        let options = console.options();
        let rendered: String = group.render(&console, options).iter().map(|s| s.text.to_string()).collect();
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
    fn test_group_measure_fill() {
        let group = Group::new([Text::plain("Hello")]).with_fit(false);
        let console = Console::new();
        let mut options = console.options().clone();
        options.max_width = 42;
        let m = group.measure(&console, &options);
        assert_eq!(m.minimum, 42);
        assert_eq!(m.maximum, 42);
    }
}
