//! Traceback: structured exception/error display.
//!
//! This module provides data structures for representing stack traces and
//! exceptions. It's designed to work with the `Syntax` module for code
//! highlighting and `scope` module for local variable display.
//!
//! # Example
//!
//! ```
//! use rich_rs::traceback::{Frame, Stack, Trace, Traceback};
//!
//! // Create a simple stack trace
//! let frame = Frame::new("main.rs", 42, "main");
//! let stack = Stack::new("RuntimeError", "Something went wrong")
//!     .with_frame(frame);
//! let trace = Trace::new(vec![stack]);
//!
//! let tb = Traceback::new(trace);
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::io::Stdout;
use std::path::Path;

use crate::Renderable;
use crate::r#box::ROUNDED;
use crate::console::{Console, ConsoleOptions};
use crate::highlighter::{Highlighter, RegexHighlighter, path_highlighter, repr_highlighter};
use crate::measure::Measurement;
use crate::scope::render_scope;
use crate::segment::{Segment, Segments};
use crate::style::Style;
use crate::syntax::Syntax;
use crate::text::Text;

// ============================================================================
// Constants
// ============================================================================

/// Default maximum number of frames to display.
pub const DEFAULT_MAX_FRAMES: usize = 100;

/// Default number of extra context lines around the error line.
pub const DEFAULT_EXTRA_LINES: usize = 3;

/// Default maximum length for local variable containers.
pub const LOCALS_MAX_LENGTH: usize = 10;

/// Default maximum length for local variable strings.
pub const LOCALS_MAX_STRING: usize = 80;

// ============================================================================
// Frame
// ============================================================================

/// A single stack frame in a traceback.
///
/// Represents one level in the call stack, including the source location,
/// function name, source line, and optionally local variables.
#[derive(Debug, Clone)]
pub struct Frame {
    /// The source file path.
    pub filename: String,
    /// The line number (1-based).
    pub lineno: usize,
    /// The function or method name.
    pub name: String,
    /// The source code line (may be empty if unavailable).
    pub line: String,
    /// Local variables as debug strings (name -> repr).
    /// Uses `BTreeMap` for deterministic ordering.
    pub locals: Option<BTreeMap<String, String>>,
}

impl Frame {
    /// Create a new frame with the essential fields.
    ///
    /// # Arguments
    ///
    /// * `filename` - The source file path.
    /// * `lineno` - The line number (1-based).
    /// * `name` - The function or method name.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::traceback::Frame;
    ///
    /// let frame = Frame::new("src/main.rs", 42, "main");
    /// ```
    pub fn new(filename: impl Into<String>, lineno: usize, name: impl Into<String>) -> Self {
        Self {
            filename: filename.into(),
            lineno,
            name: name.into(),
            line: String::new(),
            locals: None,
        }
    }

    /// Set the source line for this frame.
    pub fn with_line(mut self, line: impl Into<String>) -> Self {
        self.line = line.into();
        self
    }

    /// Set the local variables for this frame.
    pub fn with_locals(mut self, locals: BTreeMap<String, String>) -> Self {
        self.locals = Some(locals);
        self
    }

    /// Add a single local variable.
    pub fn add_local(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.locals
            .get_or_insert_with(BTreeMap::new)
            .insert(name.into(), value.into());
    }

    /// Check if this frame has local variables.
    pub fn has_locals(&self) -> bool {
        self.locals.as_ref().is_some_and(|l| !l.is_empty())
    }
}

// ============================================================================
// SyntaxErrorInfo
// ============================================================================

/// Information about a syntax error.
///
/// This is used for special handling of syntax errors which have
/// additional location information.
#[derive(Debug, Clone)]
pub struct SyntaxErrorInfo {
    /// Column offset where the error occurred.
    pub offset: usize,
    /// The source file path.
    pub filename: String,
    /// The source line containing the error.
    pub line: String,
    /// The line number (1-based).
    pub lineno: usize,
    /// The error message.
    pub msg: String,
}

impl SyntaxErrorInfo {
    /// Create new syntax error info.
    ///
    /// # Arguments
    ///
    /// * `filename` - The source file path.
    /// * `lineno` - The line number.
    /// * `offset` - The column offset.
    /// * `msg` - The error message.
    pub fn new(
        filename: impl Into<String>,
        lineno: usize,
        offset: usize,
        msg: impl Into<String>,
    ) -> Self {
        Self {
            offset,
            filename: filename.into(),
            line: String::new(),
            lineno,
            msg: msg.into(),
        }
    }

    /// Set the source line.
    pub fn with_line(mut self, line: impl Into<String>) -> Self {
        self.line = line.into();
        self
    }
}

// ============================================================================
// Stack
// ============================================================================

/// A single exception stack (one exception in a chain).
///
/// Contains the exception type, value, optional syntax error info,
/// and the list of frames leading to the exception.
#[derive(Debug, Clone)]
pub struct Stack {
    /// The exception type name (e.g., "ValueError", "RuntimeError").
    pub exc_type: String,
    /// The exception message/value.
    pub exc_value: String,
    /// Syntax error information (for SyntaxError exceptions).
    pub syntax_error: Option<SyntaxErrorInfo>,
    /// Whether this exception was caused by another (chained exception).
    pub is_cause: bool,
    /// The stack frames for this exception.
    pub frames: Vec<Frame>,
}

impl Stack {
    /// Create a new stack with the exception type and value.
    ///
    /// # Arguments
    ///
    /// * `exc_type` - The exception type name.
    /// * `exc_value` - The exception message.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::traceback::Stack;
    ///
    /// let stack = Stack::new("ValueError", "invalid input");
    /// ```
    pub fn new(exc_type: impl Into<String>, exc_value: impl Into<String>) -> Self {
        Self {
            exc_type: exc_type.into(),
            exc_value: exc_value.into(),
            syntax_error: None,
            is_cause: false,
            frames: Vec::new(),
        }
    }

    /// Set the syntax error info.
    pub fn with_syntax_error(mut self, error: SyntaxErrorInfo) -> Self {
        self.syntax_error = Some(error);
        self
    }

    /// Mark this as a caused exception (from exception chaining).
    pub fn with_is_cause(mut self, is_cause: bool) -> Self {
        self.is_cause = is_cause;
        self
    }

    /// Set the frames for this stack.
    pub fn with_frames(mut self, frames: Vec<Frame>) -> Self {
        self.frames = frames;
        self
    }

    /// Add a single frame to the stack.
    pub fn with_frame(mut self, frame: Frame) -> Self {
        self.frames.push(frame);
        self
    }

    /// Add a frame to the stack (mutable version).
    pub fn add_frame(&mut self, frame: Frame) {
        self.frames.push(frame);
    }

    /// Check if this is a syntax error.
    pub fn is_syntax_error(&self) -> bool {
        self.syntax_error.is_some()
    }

    /// Get the number of frames.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}

// ============================================================================
// Trace
// ============================================================================

/// A complete trace with potentially chained exceptions.
///
/// Contains one or more `Stack` objects representing the exception chain.
/// The last stack is typically the most recent exception.
#[derive(Debug, Clone)]
pub struct Trace {
    /// The exception stacks (may be multiple for chained exceptions).
    pub stacks: Vec<Stack>,
}

impl Trace {
    /// Create a new trace with the given stacks.
    ///
    /// # Arguments
    ///
    /// * `stacks` - The exception stacks.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::traceback::{Stack, Trace};
    ///
    /// let stack = Stack::new("Error", "message");
    /// let trace = Trace::new(vec![stack]);
    /// ```
    pub fn new(stacks: Vec<Stack>) -> Self {
        Self { stacks }
    }

    /// Create an empty trace.
    pub fn empty() -> Self {
        Self { stacks: Vec::new() }
    }

    /// Add a stack to the trace.
    pub fn with_stack(mut self, stack: Stack) -> Self {
        self.stacks.push(stack);
        self
    }

    /// Add a stack to the trace (mutable version).
    pub fn add_stack(&mut self, stack: Stack) {
        self.stacks.push(stack);
    }

    /// Get the number of stacks.
    pub fn stack_count(&self) -> usize {
        self.stacks.len()
    }

    /// Check if this trace is empty.
    pub fn is_empty(&self) -> bool {
        self.stacks.is_empty()
    }
}

// ============================================================================
// TracebackBuilder
// ============================================================================

/// Builder for `Traceback` configuration.
#[derive(Debug, Clone)]
pub struct TracebackBuilder {
    trace: Trace,
    width: Option<usize>,
    extra_lines: usize,
    theme: Option<String>,
    word_wrap: bool,
    show_locals: bool,
    locals_max_length: Option<usize>,
    locals_max_string: Option<usize>,
    locals_hide_dunder: bool,
    locals_hide_sunder: bool,
    indent_guides: bool,
    suppress: Vec<String>,
    max_frames: usize,
}

impl TracebackBuilder {
    /// Create a new builder with the given trace.
    pub fn new(trace: Trace) -> Self {
        Self {
            trace,
            width: None,
            extra_lines: DEFAULT_EXTRA_LINES,
            theme: None,
            word_wrap: false,
            show_locals: false,
            locals_max_length: Some(LOCALS_MAX_LENGTH),
            locals_max_string: Some(LOCALS_MAX_STRING),
            locals_hide_dunder: true,
            locals_hide_sunder: false,
            indent_guides: true,
            suppress: Vec::new(),
            max_frames: DEFAULT_MAX_FRAMES,
        }
    }

    /// Set the width for the traceback display.
    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the number of extra context lines around the error.
    pub fn extra_lines(mut self, lines: usize) -> Self {
        self.extra_lines = lines;
        self
    }

    /// Set the syntax highlighting theme name.
    pub fn theme(mut self, theme: impl Into<String>) -> Self {
        self.theme = Some(theme.into());
        self
    }

    /// Enable or disable word wrapping.
    pub fn word_wrap(mut self, wrap: bool) -> Self {
        self.word_wrap = wrap;
        self
    }

    /// Enable or disable showing local variables.
    pub fn show_locals(mut self, show: bool) -> Self {
        self.show_locals = show;
        self
    }

    /// Set the maximum length for local variable containers.
    pub fn locals_max_length(mut self, max: Option<usize>) -> Self {
        self.locals_max_length = max;
        self
    }

    /// Set the maximum string length for local variables.
    pub fn locals_max_string(mut self, max: Option<usize>) -> Self {
        self.locals_max_string = max;
        self
    }

    /// Hide locals prefixed with double underscore.
    pub fn locals_hide_dunder(mut self, hide: bool) -> Self {
        self.locals_hide_dunder = hide;
        self
    }

    /// Hide locals prefixed with single underscore.
    pub fn locals_hide_sunder(mut self, hide: bool) -> Self {
        self.locals_hide_sunder = hide;
        self
    }

    /// Enable or disable indent guides in code.
    pub fn indent_guides(mut self, guides: bool) -> Self {
        self.indent_guides = guides;
        self
    }

    /// Add a module/path to suppress from the traceback.
    pub fn suppress(mut self, path: impl Into<String>) -> Self {
        self.suppress.push(path.into());
        self
    }

    /// Add multiple modules/paths to suppress.
    pub fn suppress_all(mut self, paths: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.suppress.extend(paths.into_iter().map(Into::into));
        self
    }

    /// Set the maximum number of frames to display.
    pub fn max_frames(mut self, max: usize) -> Self {
        self.max_frames = if max > 0 { max.max(4) } else { 0 };
        self
    }

    /// Build the `Traceback`.
    pub fn build(self) -> Traceback {
        Traceback {
            trace: self.trace,
            width: self.width,
            extra_lines: self.extra_lines,
            theme: self.theme,
            word_wrap: self.word_wrap,
            show_locals: self.show_locals,
            locals_max_length: self.locals_max_length,
            locals_max_string: self.locals_max_string,
            locals_hide_dunder: self.locals_hide_dunder,
            locals_hide_sunder: self.locals_hide_sunder,
            indent_guides: self.indent_guides,
            suppress: self.suppress,
            max_frames: self.max_frames,
        }
    }
}

// ============================================================================
// Traceback
// ============================================================================

/// Traceback display configuration and data.
///
/// Holds a `Trace` and configuration options for rendering.
/// Rendering is not yet implemented - this is the struct definition only.
///
/// # Example
///
/// ```
/// use rich_rs::traceback::{Frame, Stack, Trace, Traceback};
///
/// let frame = Frame::new("main.rs", 10, "main")
///     .with_line("    let x = foo();");
///
/// let stack = Stack::new("PanicInfo", "called `Result::unwrap()` on an `Err` value")
///     .with_frame(frame);
///
/// let trace = Trace::new(vec![stack]);
///
/// // Using new() with defaults
/// let tb = Traceback::new(trace.clone());
///
/// // Using builder pattern for customization
/// let tb = Traceback::builder(trace)
///     .width(100)
///     .show_locals(true)
///     .theme("monokai")
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct Traceback {
    /// The trace data.
    pub trace: Trace,
    /// Display width (None = use console width).
    pub width: Option<usize>,
    /// Number of extra context lines around the error line.
    pub extra_lines: usize,
    /// Syntax highlighting theme name.
    pub theme: Option<String>,
    /// Enable word wrapping of long lines.
    pub word_wrap: bool,
    /// Show local variables in each frame.
    pub show_locals: bool,
    /// Maximum length for container locals before abbreviating.
    pub locals_max_length: Option<usize>,
    /// Maximum string length for locals before truncating.
    pub locals_max_string: Option<usize>,
    /// Hide locals prefixed with double underscore.
    pub locals_hide_dunder: bool,
    /// Hide locals prefixed with single underscore.
    pub locals_hide_sunder: bool,
    /// Show indent guides in code.
    pub indent_guides: bool,
    /// Modules/paths to suppress from the traceback.
    pub suppress: Vec<String>,
    /// Maximum number of frames to show (0 = unlimited).
    pub max_frames: usize,
}

impl Traceback {
    /// Create a new traceback with default settings.
    ///
    /// # Arguments
    ///
    /// * `trace` - The trace data to display.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::traceback::{Stack, Trace, Traceback};
    ///
    /// let trace = Trace::new(vec![Stack::new("Error", "message")]);
    /// let tb = Traceback::new(trace);
    /// ```
    pub fn new(trace: Trace) -> Self {
        Self {
            trace,
            width: None,
            extra_lines: DEFAULT_EXTRA_LINES,
            theme: None,
            word_wrap: false,
            show_locals: false,
            locals_max_length: Some(LOCALS_MAX_LENGTH),
            locals_max_string: Some(LOCALS_MAX_STRING),
            locals_hide_dunder: true,
            locals_hide_sunder: false,
            indent_guides: true,
            suppress: Vec::new(),
            max_frames: DEFAULT_MAX_FRAMES,
        }
    }

    /// Create a builder for configuring a traceback.
    ///
    /// # Arguments
    ///
    /// * `trace` - The trace data to display.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::traceback::{Stack, Trace, Traceback};
    ///
    /// let trace = Trace::new(vec![Stack::new("Error", "message")]);
    /// let tb = Traceback::builder(trace)
    ///     .show_locals(true)
    ///     .max_frames(50)
    ///     .build();
    /// ```
    pub fn builder(trace: Trace) -> TracebackBuilder {
        TracebackBuilder::new(trace)
    }

    /// Get the trace data.
    pub fn trace(&self) -> &Trace {
        &self.trace
    }

    /// Check if local variables should be displayed.
    pub fn should_show_locals(&self) -> bool {
        self.show_locals
    }

    /// Filter locals based on hide settings.
    ///
    /// Returns a new `BTreeMap` with hidden variables removed.
    pub fn filter_locals(&self, locals: &BTreeMap<String, String>) -> BTreeMap<String, String> {
        locals
            .iter()
            .filter(|(name, _)| {
                // Hide dunder variables if configured
                if self.locals_hide_dunder && name.starts_with("__") && name.ends_with("__") {
                    return false;
                }
                // Hide sunder variables if configured
                if self.locals_hide_sunder && name.starts_with('_') && !name.starts_with("__") {
                    return false;
                }
                true
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Check if a path should be suppressed.
    pub fn is_suppressed(&self, path: &str) -> bool {
        self.suppress.iter().any(|s| path.contains(s))
    }
}

// SAFETY: Traceback is Send + Sync because all fields are Send + Sync.
unsafe impl Send for Traceback {}
unsafe impl Sync for Traceback {}

// ============================================================================
// Styles
// ============================================================================

/// Style for traceback title.
fn traceback_title_style() -> Style {
    Style::new()
        .with_color(crate::color::SimpleColor::Standard(9)) // bright red
        .with_bold(true)
}

/// Style for traceback border.
fn traceback_border_style() -> Style {
    Style::new().with_color(crate::color::SimpleColor::Standard(1)) // red
}

/// Style for exception type.
fn exc_type_style() -> Style {
    Style::new()
        .with_color(crate::color::SimpleColor::Standard(1)) // red
        .with_bold(true)
}

/// Style for file paths.
fn path_style() -> Style {
    Style::new().with_color(crate::color::SimpleColor::Standard(5)) // magenta
}

/// Style for line numbers.
fn lineno_style() -> Style {
    Style::new().with_color(crate::color::SimpleColor::Standard(6)) // cyan
}

/// Style for function names.
fn function_style() -> Style {
    Style::new().with_color(crate::color::SimpleColor::Standard(2)) // green
}

/// Style for "frames hidden" message.
fn frames_hidden_style() -> Style {
    Style::new()
        .with_color(crate::color::SimpleColor::Standard(3)) // yellow
        .with_italic(true)
}

/// Style for syntax error offset indicator.
fn syntax_error_offset_style() -> Style {
    Style::new()
        .with_color(crate::color::SimpleColor::Standard(1)) // red
        .with_bold(true)
}

// ============================================================================
// Renderable Implementation
// ============================================================================

impl Renderable for Traceback {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let mut result = Segments::new();

        // Get display width
        let width = self
            .width
            .unwrap_or(options.max_width)
            .min(options.max_width);
        let code_width = width.saturating_sub(8); // Leave room for line numbers and borders

        // Render stacks in reverse order (most recent exception last)
        let stacks: Vec<&Stack> = self.trace.stacks.iter().rev().collect();

        for (idx, stack) in stacks.iter().enumerate() {
            let is_last = idx == stacks.len() - 1;

            // Render stack frames if present
            if !stack.frames.is_empty() {
                // Render stack content directly to segments
                let stack_segments = self.render_stack_frames(stack, console, options, code_width);

                // Create a wrapper text from the segments for the panel
                // We'll use a simpler approach: render directly without panel
                // since Panel requires 'static lifetime

                // Create panel title
                let mut title = Text::styled("Traceback ", traceback_title_style());
                title.append(
                    "(most recent call last)",
                    Some(traceback_title_style().with_dim(true)),
                );

                // Render panel border manually
                let border_style = traceback_border_style();
                let box_chars = &ROUNDED;

                // Top border with title
                let title_str = title.plain_text();
                let title_len = crate::cells::cell_len(&title_str);
                let inner_width = width.saturating_sub(2);
                let title_pad = inner_width.saturating_sub(title_len).saturating_sub(2);
                let left_pad = title_pad / 2;
                let right_pad = title_pad - left_pad;

                // Top line: ╭─ Title ─────────────────╮
                result.push(Segment::styled(
                    box_chars.top_left.to_string(),
                    border_style,
                ));
                result.push(Segment::styled(
                    box_chars.top.to_string().repeat(left_pad + 1),
                    border_style,
                ));
                result.extend(title.render(console, options));
                result.push(Segment::styled(
                    box_chars.top.to_string().repeat(right_pad + 1),
                    border_style,
                ));
                result.push(Segment::styled(
                    box_chars.top_right.to_string(),
                    border_style,
                ));
                result.push(Segment::line());

                // Content lines with side borders
                let segment_vec: Vec<Segment> = stack_segments.iter().cloned().collect();
                let content_lines = Segment::split_lines(segment_vec);
                for line_segments in content_lines {
                    result.push(Segment::styled(
                        format!("{} ", box_chars.mid_left),
                        border_style,
                    ));
                    result.extend(line_segments.into_iter().map(|s| Segment::from(s)));
                    // Pad to width
                    let line_width: usize = result
                        .iter()
                        .skip(result.len().saturating_sub(10))
                        .map(|s| crate::cells::cell_len(&s.text))
                        .sum();
                    let padding = inner_width.saturating_sub(line_width.saturating_sub(2));
                    if padding > 0 {
                        result.push(Segment::new(" ".repeat(padding)));
                    }
                    result.push(Segment::styled(
                        format!(" {}", box_chars.mid_right),
                        border_style,
                    ));
                    result.push(Segment::line());
                }

                // Bottom border
                result.push(Segment::styled(
                    box_chars.bottom_left.to_string(),
                    border_style,
                ));
                result.push(Segment::styled(
                    box_chars.bottom.to_string().repeat(inner_width),
                    border_style,
                ));
                result.push(Segment::styled(
                    box_chars.bottom_right.to_string(),
                    border_style,
                ));
                result.push(Segment::line());
            }

            // Render syntax error if present
            if let Some(ref syntax_error) = stack.syntax_error {
                let syntax_error_segments =
                    self.render_syntax_error_content(syntax_error, console, options);
                result.extend(syntax_error_segments);
                result.push(Segment::line());
            }

            // Render exception type and value
            let exc_text = self.render_exception_line(stack);
            let exc_segments = exc_text.render(console, options);
            result.extend(exc_segments);
            result.push(Segment::line());

            // If not the last stack, show chaining message
            if !is_last {
                let chaining_msg = if stack.is_cause {
                    "\nThe above exception was the direct cause of the following exception:\n"
                } else {
                    "\nDuring handling of the above exception, another exception occurred:\n"
                };

                let chaining_text = Text::styled(chaining_msg, Style::new().with_italic(true));
                let chaining_segments = chaining_text.render(console, options);
                result.extend(chaining_segments);
                result.push(Segment::line());
            }
        }

        result
    }

    fn measure(&self, _console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        // Traceback width is configurable, use it or max_width
        let width = self
            .width
            .unwrap_or(options.max_width)
            .min(options.max_width);
        Measurement::new(width, width)
    }
}

impl Traceback {
    /// Render stack frames directly to segments.
    fn render_stack_frames(
        &self,
        stack: &Stack,
        console: &Console<Stdout>,
        options: &ConsoleOptions,
        code_width: usize,
    ) -> Segments {
        let mut result = Segments::new();
        let highlighter = path_highlighter();
        let frames = &stack.frames;

        // Calculate frame exclusion range if needed
        let exclude_range = if self.max_frames > 0 && frames.len() > self.max_frames {
            let half = self.max_frames / 2;
            Some(half..(frames.len() - half))
        } else {
            None
        };

        let mut excluded_shown = false;

        for (idx, frame) in frames.iter().enumerate() {
            // Check if this frame should be excluded
            if let Some(ref range) = exclude_range {
                if range.contains(&idx) {
                    // Show "frames hidden" message once
                    if !excluded_shown {
                        let msg = format!("\n... {} frames hidden ...\n", range.len());
                        let hidden_text = Text::styled(&msg, frames_hidden_style());
                        result.extend(hidden_text.render(console, options));
                        excluded_shown = true;
                    }
                    continue;
                }
            }

            // Check if frame should be suppressed
            let suppressed = self.is_suppressed(&frame.filename);

            // Add blank line between frames (except first)
            if idx > 0 && !frame.filename.starts_with('<') {
                result.push(Segment::line());
            }

            // Render frame location
            let location_text = self.render_frame_location(frame, &highlighter);
            result.extend(location_text.render(console, options));
            result.push(Segment::line());

            // Skip code display for special frames or suppressed frames
            if frame.filename.starts_with('<') || suppressed {
                // Still show locals if available
                if self.show_locals && frame.has_locals() {
                    self.render_locals(&mut result, frame, console, options);
                }
                continue;
            }

            // Try to read and display source code
            if let Some(code) = self.read_source_code(&frame.filename) {
                let lexer = self.guess_lexer(&frame.filename);

                // Calculate line range
                let start_line = frame.lineno.saturating_sub(self.extra_lines);
                let end_line = frame.lineno + self.extra_lines;

                // Create syntax highlighter
                let mut syntax = Syntax::new(&code, lexer)
                    .with_line_numbers(true)
                    .with_line_range(Some(start_line), Some(end_line))
                    .with_highlight_lines(vec![frame.lineno])
                    .with_indent_guides(self.indent_guides)
                    .with_word_wrap(self.word_wrap);

                if let Some(ref theme) = self.theme {
                    syntax = syntax.with_theme(theme);
                }

                if code_width > 0 {
                    syntax = syntax.with_code_width(code_width);
                }

                result.extend(syntax.render(console, options));
            }

            // Show locals if enabled
            if self.show_locals && frame.has_locals() {
                result.push(Segment::line());
                self.render_locals(&mut result, frame, console, options);
            }
        }

        result
    }

    /// Render a frame location line (filename:lineno in function_name).
    fn render_frame_location(&self, frame: &Frame, highlighter: &RegexHighlighter) -> Text {
        let path = Path::new(&frame.filename);

        if path.exists() {
            // File exists: show full path
            let mut text = Text::new();

            // Path with highlighting (dim directory, bold filename)
            let mut path_text = Text::styled(&frame.filename, path_style());
            highlighter.highlight(&mut path_text);
            text.append_text(&path_text);

            // :lineno
            text.append(":", None);
            text.append(&frame.lineno.to_string(), Some(lineno_style()));

            // in function_name
            text.append(" in ", None);
            text.append(&frame.name, Some(function_style()));

            text
        } else {
            // File doesn't exist: show function:lineno format
            let mut text = Text::new();
            text.append("in ", None);
            text.append(&frame.name, Some(function_style()));
            text.append(":", None);
            text.append(&frame.lineno.to_string(), Some(lineno_style()));
            text
        }
    }

    /// Render local variables for a frame.
    fn render_locals(
        &self,
        result: &mut Segments,
        frame: &Frame,
        console: &Console<Stdout>,
        options: &ConsoleOptions,
    ) {
        if let Some(ref locals) = frame.locals {
            // Filter locals based on settings
            let filtered = self.filter_locals(locals);

            if !filtered.is_empty() {
                let scope = render_scope(
                    &filtered,
                    Some("locals"),
                    true, // sort_keys
                    self.indent_guides,
                    self.locals_max_length,
                    self.locals_max_string,
                );
                result.extend(scope.render(console, options));
            }
        }
    }

    /// Render syntax error content directly to segments.
    fn render_syntax_error_content(
        &self,
        error: &SyntaxErrorInfo,
        console: &Console<Stdout>,
        options: &ConsoleOptions,
    ) -> Segments {
        let mut result = Segments::new();

        let highlighter = path_highlighter();
        let repr_hl = repr_highlighter();

        // Show file location if not stdin
        if error.filename != "<stdin>" {
            let path = Path::new(&error.filename);
            if path.exists() {
                let mut location = Text::new();
                location.append(" ", None);

                let mut path_text = Text::styled(&error.filename, path_style());
                highlighter.highlight(&mut path_text);
                location.append_text(&path_text);

                location.append(":", None);
                location.append(&error.lineno.to_string(), Some(lineno_style()));

                result.extend(location.render(console, options));
                result.push(Segment::line());
            }
        }

        // Show the error line
        let mut error_line = Text::plain(error.line.trim_end());
        repr_hl.highlight(&mut error_line);

        // Underline the error position
        let offset = error
            .offset
            .saturating_sub(1)
            .min(error_line.plain_text().len());
        if offset < error_line.plain_text().len() {
            error_line.stylize(
                offset,
                offset + 1,
                Style::new().with_bold(true).with_underline(true),
            );
        }

        result.extend(error_line.render(console, options));

        // Show offset indicator
        let indicator = format!("\n{}▲", " ".repeat(offset));
        let indicator_text = Text::styled(&indicator, syntax_error_offset_style());
        result.extend(indicator_text.render(console, options));

        result
    }

    /// Render the exception type and value line.
    fn render_exception_line(&self, stack: &Stack) -> Text {
        let highlighter = repr_highlighter();

        let mut text = Text::new();

        // Exception type
        text.append(&format!("{}: ", stack.exc_type), Some(exc_type_style()));

        // Exception value (highlighted)
        let mut value_text = Text::plain(&stack.exc_value);
        highlighter.highlight(&mut value_text);
        text.append_text(&value_text);

        text
    }

    /// Read source code from a file.
    fn read_source_code(&self, filename: &str) -> Option<String> {
        let path = Path::new(filename);
        if path.exists() {
            fs::read_to_string(path).ok()
        } else {
            None
        }
    }

    /// Guess the lexer name for syntax highlighting.
    fn guess_lexer(&self, filename: &str) -> &'static str {
        let ext = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match ext {
            "rs" => "rust",
            "py" => "python",
            "js" | "mjs" => "javascript",
            "ts" | "mts" => "typescript",
            "tsx" => "tsx",
            "jsx" => "jsx",
            "c" | "h" => "c",
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => "cpp",
            "go" => "go",
            "java" => "java",
            "rb" => "ruby",
            "sh" | "bash" => "bash",
            "json" => "json",
            "toml" => "toml",
            "yaml" | "yml" => "yaml",
            "md" | "markdown" => "markdown",
            "html" | "htm" => "html",
            "css" => "css",
            "sql" => "sql",
            "xml" => "xml",
            _ => "text",
        }
    }
}

// ============================================================================
// Panic Hook Installation
// ============================================================================

/// Install a rich panic hook that displays panics with syntax highlighting.
///
/// This replaces the default panic hook with one that uses `Traceback` to
/// display the panic information.
///
/// # Example
///
/// ```no_run
/// use rich_rs::traceback::install;
///
/// install();
///
/// // Now panics will be displayed with rich formatting
/// panic!("Something went wrong!");
/// ```
pub fn install() {
    install_with_options(TracebackBuilder::new(Trace::empty()));
}

/// Install a rich panic hook with custom options.
///
/// # Example
///
/// ```no_run
/// use rich_rs::traceback::{install_with_options, Trace, TracebackBuilder};
///
/// install_with_options(
///     TracebackBuilder::new(Trace::empty())
///         .show_locals(true)
///         .width(120)
/// );
/// ```
pub fn install_with_options(builder: TracebackBuilder) {
    use crate::ColorSystem;
    use std::io::{self, Write};

    let config = builder.build();

    std::panic::set_hook(Box::new(move |panic_info| {
        // Create a stdout console for rendering (we'll output to stderr manually)
        let console = Console::new();
        let options = console.options();

        // Get panic location
        let location = panic_info.location();
        let (filename, lineno, _col) = location
            .map(|l| (l.file().to_string(), l.line() as usize, l.column() as usize))
            .unwrap_or_else(|| ("<unknown>".to_string(), 0, 0));

        // Get panic message
        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Box<dyn Any>".to_string()
        };

        // Build a simple trace with one frame
        let frame = Frame::new(&filename, lineno, "panic");
        let stack = Stack::new("panic", &message).with_frame(frame);
        let trace = Trace::new(vec![stack]);

        // Create traceback with user's configuration
        let traceback = Traceback {
            trace,
            width: config.width,
            extra_lines: config.extra_lines,
            theme: config.theme.clone(),
            word_wrap: config.word_wrap,
            show_locals: false, // Can't get locals from panic
            locals_max_length: config.locals_max_length,
            locals_max_string: config.locals_max_string,
            locals_hide_dunder: config.locals_hide_dunder,
            locals_hide_sunder: config.locals_hide_sunder,
            indent_guides: config.indent_guides,
            suppress: config.suppress.clone(),
            max_frames: config.max_frames,
        };

        // Render the traceback
        let segments = traceback.render(&console, &options);

        // Determine color system (assume TrueColor for stderr in most terminals)
        let color_system = ColorSystem::TrueColor;

        // Output to stderr with ANSI styling
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        for segment in segments.iter() {
            if segment.is_control() {
                continue;
            }
            if let Some(style) = segment.style {
                let _ = write!(handle, "{}", style.render(&segment.text, color_system));
            } else {
                let _ = write!(handle, "{}", segment.text);
            }
        }
        let _ = writeln!(handle);
        let _ = handle.flush();
    }));
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Frame tests ====================

    #[test]
    fn test_frame_new() {
        let frame = Frame::new("main.rs", 42, "main");
        assert_eq!(frame.filename, "main.rs");
        assert_eq!(frame.lineno, 42);
        assert_eq!(frame.name, "main");
        assert!(frame.line.is_empty());
        assert!(frame.locals.is_none());
    }

    #[test]
    fn test_frame_with_line() {
        let frame = Frame::new("test.rs", 10, "test").with_line("    let x = 42;");
        assert_eq!(frame.line, "    let x = 42;");
    }

    #[test]
    fn test_frame_with_locals() {
        let mut locals = BTreeMap::new();
        locals.insert("x".to_string(), "42".to_string());

        let frame = Frame::new("test.rs", 10, "test").with_locals(locals);
        assert!(frame.has_locals());
        assert_eq!(frame.locals.unwrap().get("x"), Some(&"42".to_string()));
    }

    #[test]
    fn test_frame_add_local() {
        let mut frame = Frame::new("test.rs", 10, "test");
        frame.add_local("x", "42");
        frame.add_local("y", "100");

        assert!(frame.has_locals());
        let locals = frame.locals.unwrap();
        assert_eq!(locals.len(), 2);
    }

    // ==================== SyntaxErrorInfo tests ====================

    #[test]
    fn test_syntax_error_info_new() {
        let info = SyntaxErrorInfo::new("test.rs", 5, 10, "unexpected token");
        assert_eq!(info.filename, "test.rs");
        assert_eq!(info.lineno, 5);
        assert_eq!(info.offset, 10);
        assert_eq!(info.msg, "unexpected token");
    }

    #[test]
    fn test_syntax_error_info_with_line() {
        let info = SyntaxErrorInfo::new("test.rs", 5, 10, "error").with_line("let x = ;");
        assert_eq!(info.line, "let x = ;");
    }

    // ==================== Stack tests ====================

    #[test]
    fn test_stack_new() {
        let stack = Stack::new("ValueError", "invalid input");
        assert_eq!(stack.exc_type, "ValueError");
        assert_eq!(stack.exc_value, "invalid input");
        assert!(!stack.is_cause);
        assert!(stack.frames.is_empty());
        assert!(!stack.is_syntax_error());
    }

    #[test]
    fn test_stack_with_frame() {
        let frame = Frame::new("test.rs", 10, "test");
        let stack = Stack::new("Error", "msg").with_frame(frame);
        assert_eq!(stack.frame_count(), 1);
    }

    #[test]
    fn test_stack_with_frames() {
        let frames = vec![Frame::new("a.rs", 1, "a"), Frame::new("b.rs", 2, "b")];
        let stack = Stack::new("Error", "msg").with_frames(frames);
        assert_eq!(stack.frame_count(), 2);
    }

    #[test]
    fn test_stack_add_frame() {
        let mut stack = Stack::new("Error", "msg");
        stack.add_frame(Frame::new("a.rs", 1, "a"));
        stack.add_frame(Frame::new("b.rs", 2, "b"));
        assert_eq!(stack.frame_count(), 2);
    }

    #[test]
    fn test_stack_with_syntax_error() {
        let syntax_err = SyntaxErrorInfo::new("test.rs", 5, 10, "error");
        let stack = Stack::new("SyntaxError", "msg").with_syntax_error(syntax_err);
        assert!(stack.is_syntax_error());
    }

    #[test]
    fn test_stack_is_cause() {
        let stack = Stack::new("Error", "caused by").with_is_cause(true);
        assert!(stack.is_cause);
    }

    // ==================== Trace tests ====================

    #[test]
    fn test_trace_new() {
        let stacks = vec![Stack::new("Error", "msg")];
        let trace = Trace::new(stacks);
        assert_eq!(trace.stack_count(), 1);
        assert!(!trace.is_empty());
    }

    #[test]
    fn test_trace_empty() {
        let trace = Trace::empty();
        assert!(trace.is_empty());
        assert_eq!(trace.stack_count(), 0);
    }

    #[test]
    fn test_trace_with_stack() {
        let trace = Trace::empty()
            .with_stack(Stack::new("E1", "m1"))
            .with_stack(Stack::new("E2", "m2"));
        assert_eq!(trace.stack_count(), 2);
    }

    #[test]
    fn test_trace_add_stack() {
        let mut trace = Trace::empty();
        trace.add_stack(Stack::new("E1", "m1"));
        trace.add_stack(Stack::new("E2", "m2"));
        assert_eq!(trace.stack_count(), 2);
    }

    // ==================== Traceback tests ====================

    #[test]
    fn test_traceback_new() {
        let trace = Trace::new(vec![Stack::new("Error", "msg")]);
        let tb = Traceback::new(trace);

        assert!(tb.width.is_none());
        assert_eq!(tb.extra_lines, DEFAULT_EXTRA_LINES);
        assert!(tb.theme.is_none());
        assert!(!tb.word_wrap);
        assert!(!tb.show_locals);
        assert!(tb.locals_hide_dunder);
        assert!(!tb.locals_hide_sunder);
        assert!(tb.indent_guides);
        assert!(tb.suppress.is_empty());
        assert_eq!(tb.max_frames, DEFAULT_MAX_FRAMES);
    }

    #[test]
    fn test_traceback_builder() {
        let trace = Trace::new(vec![Stack::new("Error", "msg")]);
        let tb = Traceback::builder(trace)
            .width(100)
            .extra_lines(5)
            .theme("monokai")
            .word_wrap(true)
            .show_locals(true)
            .locals_max_length(Some(20))
            .locals_max_string(Some(100))
            .locals_hide_dunder(false)
            .locals_hide_sunder(true)
            .indent_guides(false)
            .suppress("/usr/lib")
            .suppress_all(vec!["site-packages"])
            .max_frames(50)
            .build();

        assert_eq!(tb.width, Some(100));
        assert_eq!(tb.extra_lines, 5);
        assert_eq!(tb.theme, Some("monokai".to_string()));
        assert!(tb.word_wrap);
        assert!(tb.show_locals);
        assert_eq!(tb.locals_max_length, Some(20));
        assert_eq!(tb.locals_max_string, Some(100));
        assert!(!tb.locals_hide_dunder);
        assert!(tb.locals_hide_sunder);
        assert!(!tb.indent_guides);
        assert_eq!(tb.suppress.len(), 2);
        assert_eq!(tb.max_frames, 50);
    }

    #[test]
    fn test_traceback_max_frames_minimum() {
        let trace = Trace::empty();
        // If max_frames > 0, it should be at least 4
        let tb = Traceback::builder(trace).max_frames(2).build();
        assert_eq!(tb.max_frames, 4);
    }

    #[test]
    fn test_traceback_max_frames_zero() {
        let trace = Trace::empty();
        // Zero means unlimited
        let tb = Traceback::builder(trace).max_frames(0).build();
        assert_eq!(tb.max_frames, 0);
    }

    #[test]
    fn test_traceback_filter_locals() {
        let trace = Trace::empty();
        let tb = Traceback::builder(trace)
            .locals_hide_dunder(true)
            .locals_hide_sunder(true)
            .build();

        let mut locals = BTreeMap::new();
        locals.insert("x".to_string(), "1".to_string());
        locals.insert("_private".to_string(), "2".to_string());
        locals.insert("__dunder__".to_string(), "3".to_string());
        locals.insert("normal_var".to_string(), "4".to_string());

        let filtered = tb.filter_locals(&locals);
        assert!(filtered.contains_key("x"));
        assert!(filtered.contains_key("normal_var"));
        assert!(!filtered.contains_key("_private")); // sunder hidden
        assert!(!filtered.contains_key("__dunder__")); // dunder hidden
    }

    #[test]
    fn test_traceback_filter_locals_show_all() {
        let trace = Trace::empty();
        let tb = Traceback::builder(trace)
            .locals_hide_dunder(false)
            .locals_hide_sunder(false)
            .build();

        let mut locals = BTreeMap::new();
        locals.insert("x".to_string(), "1".to_string());
        locals.insert("_private".to_string(), "2".to_string());
        locals.insert("__dunder__".to_string(), "3".to_string());

        let filtered = tb.filter_locals(&locals);
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_traceback_is_suppressed() {
        let trace = Trace::empty();
        let tb = Traceback::builder(trace)
            .suppress("/usr/lib/python")
            .suppress("site-packages")
            .build();

        assert!(tb.is_suppressed("/usr/lib/python/foo.py"));
        assert!(tb.is_suppressed("/home/user/.local/lib/site-packages/bar.py"));
        assert!(!tb.is_suppressed("/home/user/project/main.py"));
    }

    #[test]
    fn test_traceback_should_show_locals() {
        let trace = Trace::empty();
        let tb1 = Traceback::new(trace.clone());
        let tb2 = Traceback::builder(trace).show_locals(true).build();

        assert!(!tb1.should_show_locals());
        assert!(tb2.should_show_locals());
    }

    // ==================== Send + Sync tests ====================

    #[test]
    fn test_frame_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Frame>();
        assert_sync::<Frame>();
    }

    #[test]
    fn test_stack_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Stack>();
        assert_sync::<Stack>();
    }

    #[test]
    fn test_trace_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Trace>();
        assert_sync::<Trace>();
    }

    #[test]
    fn test_traceback_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Traceback>();
        assert_sync::<Traceback>();
    }

    // ==================== Rendering tests ====================

    #[test]
    fn test_traceback_render_basic() {
        use crate::{Console, Renderable};

        let frame = Frame::new("test.rs", 42, "test_function").with_line("    let x = 42;");
        let stack = Stack::new("RuntimeError", "Something went wrong").with_frame(frame);
        let trace = Trace::new(vec![stack]);
        let tb = Traceback::new(trace);

        let console = Console::new();
        let options = console.options();
        let segments = tb.render(&console, &options);

        // Should produce some output
        assert!(!segments.is_empty());

        // Should contain the exception type and message
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        assert!(output.contains("RuntimeError"));
        assert!(output.contains("Something went wrong"));
        assert!(output.contains("Traceback"));
    }

    #[test]
    fn test_traceback_render_with_chaining() {
        use crate::{Console, Renderable};

        // Create chained exception (stack order matters for rendering)
        // Stacks are rendered in reverse order, so stack1 appears first in output
        let frame1 = Frame::new("inner.rs", 10, "inner_fn");
        let stack1 = Stack::new("ValueError", "inner error").with_frame(frame1);

        let frame2 = Frame::new("outer.rs", 20, "outer_fn");
        let stack2 = Stack::new("RuntimeError", "outer error")
            .with_frame(frame2)
            .with_is_cause(true); // This stack was caused by the previous one

        let trace = Trace::new(vec![stack1, stack2]);
        let tb = Traceback::new(trace);

        let console = Console::new();
        let options = console.options();
        let segments = tb.render(&console, &options);

        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should contain both exception types
        assert!(output.contains("ValueError"));
        assert!(output.contains("RuntimeError"));
        // Should contain chaining message (between the two stacks)
        assert!(output.contains("above exception") || output.contains("another exception"));
    }

    #[test]
    fn test_traceback_render_empty() {
        use crate::{Console, Renderable};

        let trace = Trace::empty();
        let tb = Traceback::new(trace);

        let console = Console::new();
        let options = console.options();
        let segments = tb.render(&console, &options);

        // Empty trace should produce empty output
        assert!(segments.is_empty());
    }

    #[test]
    fn test_traceback_render_syntax_error() {
        use crate::{Console, Renderable};

        let syntax_err =
            SyntaxErrorInfo::new("test.py", 5, 10, "unexpected token").with_line("def foo(:");

        let stack = Stack::new("SyntaxError", "invalid syntax").with_syntax_error(syntax_err);
        let trace = Trace::new(vec![stack]);
        let tb = Traceback::new(trace);

        let console = Console::new();
        let options = console.options();
        let segments = tb.render(&console, &options);

        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should contain the syntax error info
        assert!(output.contains("SyntaxError"));
        assert!(output.contains("invalid syntax"));
        // Should contain the error indicator
        assert!(output.contains("▲"));
    }
}
