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
mod ansi;
mod cells;
mod color;
mod control;
mod emoji;
pub mod error;
pub mod export_format;
mod filesize;
mod measure;
mod segment;
mod style;
pub mod styled;
pub mod terminal_theme;
mod theme;

// Higher-level modules
mod console;
pub mod file_proxy;
pub mod highlighter;
pub mod json;
pub mod markup;
pub mod pager;
pub mod text;
pub mod wrap;

// Box drawing characters
pub mod r#box;

// Simple renderables
pub mod align;
pub mod bar;
pub mod columns;
pub mod constrain;
pub mod group;
pub mod layout;
pub mod live;
pub mod live_render;
pub mod loop_helpers;
pub mod markdown;
pub mod padding;
pub mod panel;
pub mod pretty;
pub mod progress;
pub mod progress_bar;
pub mod prompt;
pub mod region;
pub mod rule;
pub mod scope;
pub mod screen;
pub mod recorder;
pub mod screen_buffer;
pub mod screen_context;
pub mod spinner;
pub mod status;
pub mod syntax;
pub mod table;
pub mod traceback;
pub mod tree;

// Builtin renderables
mod renderables;

// Re-exports for public API
pub use cells::{cell_len, chop_cells, set_cell_size, split_graphemes};
pub use color::{
    ANSI_COLOR_NAMES, Color, ColorSystem, ColorTriplet, ColorType, EIGHT_BIT_PALETTE, Palette,
    STANDARD_PALETTE, SimpleColor, WINDOWS_PALETTE, blend_rgb, parse_rgb_hex,
};
pub use console::{
    Console, ConsoleOptions, JustifyMethod, OverflowMethod, PagerContext, PagerOptions,
};
pub use control::{Control, escape_control_codes, strip_control_codes};
pub use error::{ParseError, Result as ParseResult};
pub use measure::{Measurement, measure_renderables};
pub use segment::{ControlType, Segment, SegmentLines, Segments};
pub use style::{MetaValue, NULL_STYLE, Style, StyleMeta, StyleStack};
pub use text::{Span, Text, TextPart};
pub use theme::{Theme, ThemeError, ThemeStack, default_styles};
pub use wrap::divide_line;

// Emoji re-exports
pub use ansi::AnsiDecoder;
pub use emoji::{EMOJI, Emoji, EmojiVariant};
pub use filesize::{
    decimal as filesize_decimal, decimal_with_params as filesize_decimal_with_params,
    pick_unit_and_suffix,
};
pub use loop_helpers::{loop_first, loop_first_last, loop_last};

// Highlighter re-exports
pub use highlighter::{
    Highlighter, NullHighlighter, RegexHighlighter, combine_regex, iso8601_highlighter,
    json_highlighter, repr_highlighter,
};

// JSON re-export
pub use json::Json;

// Simple renderable re-exports
pub use align::{Align, VerticalAlignMethod};
pub use bar::Bar;
pub use columns::Columns;
pub use constrain::Constrain;
pub use group::{Group, Lines, Renderables};
pub use padding::{Padding, PaddingDimensions};
pub use panel::Panel;
pub use rule::{AlignMethod, Rule};
pub use styled::Styled;
pub use table::{Column, Row, Table};
pub use tree::{ASCII_GUIDES, BOLD_TREE_GUIDES, TREE_GUIDES, Tree, TreeGuides, TreeNodeOptions};

// Syntax highlighting re-exports
pub use syntax::{AnsiTheme, DEFAULT_THEME, Syntax, SyntaxTheme, SyntectTheme};

// Pretty printing re-exports
pub use pretty::{Pretty, pprint, pretty_repr};

// Scope re-exports
pub use scope::{ScopeRenderable, render_scope};

// Traceback re-exports
pub use layout::{Layout, LayoutRender, SplitterKind};
pub use live::{Live, LiveOptions, VerticalOverflowMethod};
pub use live_render::LiveRender;
pub use progress::{
    BarColumn, DownloadColumn, FileSizeColumn, MofNCompleteColumn, Progress, ProgressColumn,
    ProgressReader, ProgressTask, RenderableColumn, SpinnerColumn, TaskID, TaskProgressColumn,
    TextColumn, TimeElapsedColumn, TimeRemainingColumn, TotalFileSizeColumn, TrackConfig,
    TransferSpeedColumn, WrapFileBuilder,
};
pub use progress_bar::ProgressBar;
pub use recorder::FrameRecorder;
pub use region::Region;
pub use screen::Screen;
pub use screen_buffer::{Cell, ScreenBuffer};
pub use screen_context::ScreenContext;
pub use spinner::{Spinner, spinner_names};
pub use status::Status;
pub use traceback::{Frame, Stack, SyntaxErrorInfo, Trace, Traceback, TracebackBuilder};

// Prompt re-exports
pub use prompt::{
    Confirm, FloatPrompt, IntPrompt, InvalidResponse, Prompt, PromptError, Result as PromptResult,
};

// Pager re-exports
pub use file_proxy::FileProxy;
pub use pager::{BufferPager, NullPager, Pager, SystemPager};

// Terminal theme and export re-exports
pub use export_format::{CONSOLE_HTML_FORMAT, CONSOLE_SVG_FORMAT};
pub use terminal_theme::{
    DEFAULT_TERMINAL_THEME, DIMMED_MONOKAI, MONOKAI, NIGHT_OWLISH, SVG_EXPORT_THEME, TerminalTheme,
};

// Markup re-export
pub use markup::escape as escape_markup;

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

/// Log a renderable to the console with automatic timestamp and source location.
///
/// This macro calls `Console::log()` and automatically captures the file and line
/// number using `file!()` and `line!()` macros.
///
/// # Example
///
/// ```ignore
/// use rich_rs::{Console, Text, log};
///
/// let mut console = Console::new();
/// rich_rs::log!(console, &Text::plain("Server starting..."));
/// ```
#[macro_export]
macro_rules! log {
    ($console:expr, $renderable:expr) => {
        $console.log($renderable, Some(file!()), Some(line!()))
    };
}

// ============================================================================
// Global Console
// ============================================================================

use std::sync::{Mutex, OnceLock};

/// Get a reference to the global console singleton.
///
/// The global console is lazily initialized on first access.
/// Use this for convenience when you don't need a custom console.
///
/// # Example
///
/// ```ignore
/// let console = rich_rs::get_console();
/// console.lock().unwrap().print_text("Hello!").unwrap();
/// ```
pub fn get_console() -> &'static Mutex<Console> {
    static CONSOLE: OnceLock<Mutex<Console>> = OnceLock::new();
    CONSOLE.get_or_init(|| Mutex::new(Console::new()))
}

/// Print styled content using the global console.
///
/// Accepts a string that may contain Rich markup, and prints it
/// to the global console.
///
/// # Example
///
/// ```ignore
/// rich_rs::rich_print!("[bold red]Hello[/] World");
/// ```
#[macro_export]
macro_rules! rich_print {
    ($($arg:tt)*) => {{
        let text = format!($($arg)*);
        if let Ok(mut console) = $crate::get_console().lock() {
            let rendered = $crate::Text::from_markup(&text, true)
                .unwrap_or_else(|_| $crate::Text::plain(&text));
            let _ = console.print(&rendered, None, None, None, false, "\n");
        }
    }};
}
