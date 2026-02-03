//! Rich-rs: Rich text and beautiful formatting for the terminal
//!
//! A Rust port of Python's [Rich](https://github.com/Textualize/rich) library.
//!
//! # Example
//!
//! ```
//! use rich_rs::{Console, Text};
//!
//! let mut console = Console::new();
//! // Print plain text
//! console.print_text("Hello, World!").unwrap();
//!
//! // Print styled text using Text and render
//! let text = Text::from_markup("Hello, [bold red]World[/]!", false).unwrap();
//! console.print(&text, None, None, None, false, "\n").unwrap();
//! ```

// Core modules
mod cells;
mod color;
mod filesize;
mod emoji;
mod ansi;
pub mod styled;
pub mod error;
mod measure;
mod segment;
mod control;
mod style;
mod theme;

// Higher-level modules
mod console;
pub mod file_proxy;
pub mod pager;
pub mod highlighter;
pub mod markup;
pub mod text;
pub mod wrap;

// Box drawing characters
pub mod r#box;

// Simple renderables
pub mod align;
pub mod columns;
pub mod padding;
pub mod panel;
pub mod pretty;
pub mod rule;
pub mod scope;
pub mod syntax;
pub mod table;
pub mod traceback;
pub mod tree;
pub mod markdown;
pub mod live;
pub mod live_render;
pub mod spinner;
pub mod progress_bar;
pub mod progress;
pub mod constrain;
pub mod loop_helpers;
pub mod prompt;
pub mod region;
pub mod screen;

// Builtin renderables
mod renderables;

// Re-exports for public API
pub use cells::{cell_len, chop_cells, set_cell_size};
pub use color::{
    ANSI_COLOR_NAMES, Color, ColorSystem, ColorTriplet, ColorType, EIGHT_BIT_PALETTE, Palette,
    STANDARD_PALETTE, SimpleColor, WINDOWS_PALETTE, blend_rgb, parse_rgb_hex,
};
pub use console::{Console, ConsoleOptions, JustifyMethod, OverflowMethod, PagerContext, PagerOptions};
pub use error::{ParseError, Result as ParseResult};
pub use measure::{Measurement, measure_renderables};
pub use segment::{ControlType, Segment, Segments};
pub use control::Control;
pub use style::{NULL_STYLE, Style, StyleMeta};
pub use text::{Span, Text, TextPart};
pub use theme::{Theme, ThemeError, ThemeStack, default_styles};
pub use wrap::divide_line;

// Emoji re-exports
pub use emoji::{EMOJI, Emoji, EmojiVariant};
pub use ansi::AnsiDecoder;
pub use filesize::{decimal as filesize_decimal, pick_unit_and_suffix};
pub use loop_helpers::{loop_first, loop_first_last, loop_last};

// Highlighter re-exports
pub use highlighter::{
    Highlighter, NullHighlighter, RegexHighlighter, combine_regex, iso8601_highlighter,
    json_highlighter, repr_highlighter,
};

// Simple renderable re-exports
pub use align::{Align, VerticalAlignMethod};
pub use columns::Columns;
pub use padding::{Padding, PaddingDimensions};
pub use panel::Panel;
pub use rule::{AlignMethod, Rule};
pub use styled::Styled;
pub use constrain::Constrain;
pub use table::{Column, Row, Table};
pub use tree::{ASCII_GUIDES, TREE_GUIDES, Tree, TreeGuides};

// Syntax highlighting re-exports
pub use syntax::{AnsiTheme, Syntax, SyntaxTheme, SyntectTheme, DEFAULT_THEME};

// Pretty printing re-exports
pub use pretty::{Pretty, pprint, pretty_repr};

// Scope re-exports
pub use scope::{ScopeRenderable, render_scope};

// Traceback re-exports
pub use traceback::{Frame, Stack, SyntaxErrorInfo, Trace, Traceback, TracebackBuilder};
pub use live::{Live, LiveOptions, VerticalOverflowMethod};
pub use live_render::LiveRender;
pub use region::Region;
pub use screen::Screen;
pub use progress_bar::ProgressBar;
pub use spinner::Spinner;
pub use progress::{
    BarColumn, DownloadColumn, FileSizeColumn, MofNCompleteColumn, Progress, ProgressColumn,
    ProgressTask, SpinnerColumn, TaskID, TaskProgressColumn, TextColumn, TimeElapsedColumn,
    TimeRemainingColumn, TrackConfig, TransferSpeedColumn, TotalFileSizeColumn,
};

// Prompt re-exports
pub use prompt::{Confirm, FloatPrompt, IntPrompt, InvalidResponse, Prompt, PromptError, Result as PromptResult};

// Pager re-exports
pub use pager::{BufferPager, NullPager, Pager, SystemPager};
pub use file_proxy::FileProxy;

/// A type that can be rendered to the console.
///
/// All renderables must be `Send + Sync` to support `Live` and `Progress` features.
/// The `measure` method has a default implementation that renders and measures
/// the result; override it for better performance when possible.
pub trait Renderable: Send + Sync {
    /// Render this object to a sequence of segments.
    fn render(&self, console: &Console, options: &ConsoleOptions) -> Segments;

    /// Measure the minimum and maximum width requirements.
    ///
    /// Default implementation renders and measures the result.
    /// Override for better performance.
    fn measure(&self, console: &Console, options: &ConsoleOptions) -> Measurement {
        Measurement::from_segments(&self.render(console, options))
    }
}

/// A type that can be converted to a Renderable.
///
/// Uses an associated type to avoid heap allocation.
pub trait RichCast {
    /// The renderable type this converts to.
    type Output: Renderable;

    /// Convert to a renderable type.
    fn rich(&self) -> Self::Output;
}

// Implement Renderable for common types
impl Renderable for str {
    fn render(&self, _console: &Console, _options: &ConsoleOptions) -> Segments {
        // Convert to owned String since Segment requires 'static
        Segments::from(Segment::new(self.to_owned()))
    }
}

impl Renderable for String {
    fn render(&self, _console: &Console, _options: &ConsoleOptions) -> Segments {
        Segments::from(Segment::new(self.clone()))
    }
}
