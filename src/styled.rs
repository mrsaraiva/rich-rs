//! Styled: apply a style to any renderable.
//!
//! Port of Python Rich's `styled.py` (subset).
//!
//! `Styled` wraps another renderable and applies a base `Style` to every segment.
//! Inner segment styles take precedence (i.e. the base style fills in missing
//! attributes, but doesn't override explicit ones).

use std::io::Stdout;

use crate::console::ConsoleOptions;
use crate::measure::Measurement;
use crate::segment::{Segment, Segments};
use crate::style::Style;
use crate::{Console, Renderable};

pub struct Styled {
    renderable: Box<dyn Renderable + Send + Sync>,
    style: Style,
}

impl std::fmt::Debug for Styled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Styled")
            .field("style", &self.style)
            .finish_non_exhaustive()
    }
}

impl Styled {
    pub fn new(renderable: Box<dyn Renderable + Send + Sync>, style: Style) -> Self {
        Self { renderable, style }
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn style(&self) -> Style {
        self.style
    }
}

impl Renderable for Styled {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let segs = self.renderable.render(console, options);
        if self.style.is_null() {
            return segs;
        }
        Segment::apply_style_to_segments(segs, Some(self.style), None)
    }

    fn measure(&self, console: &Console, options: &ConsoleOptions) -> Measurement {
        self.renderable.measure(console, options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::SimpleColor;
    use crate::text::Text;

    #[test]
    fn test_styled_applies_base_style_without_overriding_inner() {
        let base = Style::new().with_bgcolor(SimpleColor::Standard(4));
        let inner_style = Style::new().with_color(SimpleColor::Standard(1));
        let text = Text::styled("X", inner_style);

        let styled = Styled::new(Box::new(text), base);
        let console = Console::new();
        let options = ConsoleOptions::default();
        let segs = styled.render(&console, &options);

        let s = segs.iter().find(|s| s.text.as_ref() == "X").unwrap();
        let style = s.style.unwrap();
        assert_eq!(style.color, Some(SimpleColor::Standard(1)));
        assert_eq!(style.bgcolor, Some(SimpleColor::Standard(4)));
    }

    #[test]
    fn test_styled_null_style_is_noop() {
        let text = Text::plain("Hi");
        let styled = Styled::new(Box::new(text.clone()), Style::new());
        let console = Console::new();
        let options = ConsoleOptions::default();
        let a: Vec<_> = styled.render(&console, &options).into_iter().collect();
        let b: Vec<_> = text.render(&console, &options).into_iter().collect();
        assert_eq!(a, b);
    }
}
