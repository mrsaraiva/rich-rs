//! Console: the main API for rendering to the terminal.
//!
//! The Console is the central orchestrator for all Rich output. It handles:
//! - Terminal capabilities detection
//! - Rendering renderables to segments
//! - Writing styled output to the terminal
//! - Alternate screen mode
//! - Output capture for testing

use std::env;
use std::io::{self, Stdout, Write};

use crossterm::terminal::{self, ClearType};
use crossterm::{cursor, execute, terminal as ct};

use crate::Renderable;
use crate::color::ColorSystem;
use crate::emoji::Emoji;
use crate::highlighter::Highlighter;
use crate::segment::{Segment, Segments};
use crate::style::Style;
use crate::text::Text;
use crate::theme::{Theme, ThemeStack};

// ============================================================================
// JustifyMethod
// ============================================================================

/// Text justification method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JustifyMethod {
    /// Use default justification (typically left).
    #[default]
    Default,
    /// Left-aligned text.
    Left,
    /// Center-aligned text.
    Center,
    /// Right-aligned text.
    Right,
    /// Full justification (stretch to fill width).
    Full,
}

impl JustifyMethod {
    /// Parse a justify method from a string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "default" => Some(JustifyMethod::Default),
            "left" => Some(JustifyMethod::Left),
            "center" => Some(JustifyMethod::Center),
            "right" => Some(JustifyMethod::Right),
            "full" => Some(JustifyMethod::Full),
            _ => None,
        }
    }
}

// ============================================================================
// OverflowMethod
// ============================================================================

/// Text overflow handling method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverflowMethod {
    /// Fold text at word boundaries.
    #[default]
    Fold,
    /// Crop text at the edge.
    Crop,
    /// Add ellipsis when text is cropped.
    Ellipsis,
    /// Ignore overflow (let text extend beyond bounds).
    Ignore,
}

impl OverflowMethod {
    /// Parse an overflow method from a string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "fold" => Some(OverflowMethod::Fold),
            "crop" => Some(OverflowMethod::Crop),
            "ellipsis" => Some(OverflowMethod::Ellipsis),
            "ignore" => Some(OverflowMethod::Ignore),
            _ => None,
        }
    }
}

// ============================================================================
// ConsoleOptions
// ============================================================================

/// Options passed through the rendering pipeline.
///
/// This struct carries rendering context that flows through the entire
/// render pipeline, allowing renderables to adapt to the output context.
///
/// # Console State
///
/// In addition to rendering options, this struct carries console configuration
/// that renderables may need to access (theme styles, feature flags, etc.).
/// This allows nested renderables to access console configuration without
/// needing a direct reference to the Console.
#[derive(Debug, Clone)]
pub struct ConsoleOptions {
    /// Terminal dimensions as (width, height).
    pub size: (usize, usize),
    /// Minimum width for rendering.
    pub min_width: usize,
    /// Maximum width for rendering.
    pub max_width: usize,
    /// Maximum height constraint.
    pub max_height: usize,
    /// Optional height constraint for specific renderables.
    pub height: Option<usize>,
    /// Whether output is to a terminal (vs file/pipe).
    pub is_terminal: bool,
    /// Character encoding.
    pub encoding: String,
    /// Whether to use legacy Windows console.
    pub legacy_windows: bool,
    /// Text justification override.
    pub justify: Option<JustifyMethod>,
    /// Text overflow handling override.
    pub overflow: Option<OverflowMethod>,
    /// Disable text wrapping.
    pub no_wrap: bool,
    /// Highlight override for render_str.
    pub highlight: Option<bool>,
    /// Markup parsing enabled.
    pub markup: Option<bool>,

    // =========================================================================
    // Console state passed through to renderables
    // =========================================================================
    /// Theme stack for style lookups. Cloned from the console.
    pub theme_stack: ThemeStack,
    /// Whether markup parsing is enabled by default.
    pub markup_enabled: bool,
    /// Whether emoji replacement is enabled by default.
    pub emoji_enabled: bool,
    /// Whether highlighting is enabled by default.
    pub highlight_enabled: bool,
    /// Tab size for tab expansion.
    pub tab_size: usize,
    /// Detected color system (None = no colors).
    pub color_system: Option<ColorSystem>,
}

impl Default for ConsoleOptions {
    fn default() -> Self {
        ConsoleOptions {
            size: (80, 24),
            min_width: 1,
            max_width: 80,
            max_height: 24,
            height: None,
            is_terminal: true,
            encoding: "utf-8".to_string(),
            legacy_windows: false,
            justify: None,
            overflow: None,
            no_wrap: false,
            highlight: None,
            markup: None,
            // Console state defaults
            theme_stack: ThemeStack::default(),
            markup_enabled: true,
            emoji_enabled: true,
            highlight_enabled: true,
            tab_size: 8,
            color_system: Some(ColorSystem::EightBit),
        }
    }
}

impl ConsoleOptions {
    /// Create options from the current terminal.
    pub fn from_terminal() -> Self {
        let (width, height) = terminal::size().unwrap_or((80, 24));
        let width = width as usize;
        let height = height as usize;
        let is_terminal = atty::is(atty::Stream::Stdout);
        let color_system = Console::<Stdout>::detect_color_system_static(is_terminal);
        ConsoleOptions {
            size: (width, height),
            min_width: 1,
            max_width: width,
            max_height: height,
            height: None,
            is_terminal,
            color_system,
            ..Default::default()
        }
    }

    /// Get a style from the theme stack by name.
    pub fn get_style(&self, name: &str) -> Option<Style> {
        self.theme_stack.get_style(name)
    }

    /// Check if renderables should use ASCII only.
    pub fn ascii_only(&self) -> bool {
        !self.encoding.to_lowercase().starts_with("utf")
    }

    /// Create a copy of the options.
    pub fn copy(&self) -> Self {
        self.clone()
    }

    /// Update values and return a new copy.
    ///
    /// Only non-None values in the update parameters will override
    /// the existing values.
    pub fn update(
        &self,
        width: Option<usize>,
        min_width: Option<usize>,
        max_width: Option<usize>,
        justify: Option<Option<JustifyMethod>>,
        overflow: Option<Option<OverflowMethod>>,
        no_wrap: Option<bool>,
        highlight: Option<Option<bool>>,
        markup: Option<Option<bool>>,
        height: Option<Option<usize>>,
    ) -> Self {
        let mut options = self.clone();

        if let Some(w) = width {
            options.min_width = w.max(0);
            options.max_width = w.max(0);
        }
        if let Some(w) = min_width {
            options.min_width = w;
        }
        if let Some(w) = max_width {
            options.max_width = w;
        }
        if let Some(j) = justify {
            options.justify = j;
        }
        if let Some(o) = overflow {
            options.overflow = o;
        }
        if let Some(nw) = no_wrap {
            options.no_wrap = nw;
        }
        if let Some(h) = highlight {
            options.highlight = h;
        }
        if let Some(m) = markup {
            options.markup = m;
        }
        if let Some(h) = height {
            if let Some(h) = h {
                options.max_height = h;
            }
            options.height = h;
        }

        options
    }

    /// Update just the width, return a copy.
    pub fn update_width(&self, width: usize) -> Self {
        let mut options = self.clone();
        options.min_width = width.max(0);
        options.max_width = width.max(0);
        options
    }

    /// Update the height and return a copy.
    pub fn update_height(&self, height: usize) -> Self {
        let mut options = self.clone();
        options.max_height = height;
        options.height = Some(height);
        options
    }

    /// Update both width and height, return a copy.
    pub fn update_dimensions(&self, width: usize, height: usize) -> Self {
        let mut options = self.clone();
        options.min_width = width.max(0);
        options.max_width = width.max(0);
        options.max_height = height;
        options.height = Some(height);
        options
    }

    /// Reset height to None, return a copy.
    pub fn reset_height(&self) -> Self {
        let mut options = self.clone();
        options.height = None;
        options
    }
}

// ============================================================================
// Console (Generic over Writer)
// ============================================================================

/// The main console for rendering output.
///
/// Console is generic over the writer type, allowing it to write to any
/// type that implements `Write`. The default is `Stdout`, but you can use
/// `Vec<u8>` for testing or any other writer.
///
/// # Example
///
/// ```
/// use rich_rs::Console;
///
/// let mut console = Console::new();
/// console.print_text("Hello, World!").unwrap();
/// ```
///
/// # Testing with capture
///
/// ```
/// use rich_rs::Console;
///
/// let mut console = Console::capture();
/// console.print_text("Hello").unwrap();
/// assert!(console.get_captured().contains("Hello"));
/// ```
pub struct Console<W: Write = Stdout> {
    /// Output writer.
    writer: W,
    /// Console options.
    options: ConsoleOptions,
    /// Detected color system.
    color_system: Option<ColorSystem>,
    /// Whether terminal mode is forced.
    force_terminal: Option<bool>,
    /// Whether to use legacy Windows console.
    legacy_windows: bool,
    /// Whether markup parsing is enabled by default.
    markup_enabled: bool,
    /// Whether emoji replacement is enabled by default.
    emoji_enabled: bool,
    /// Whether highlighting is enabled by default.
    highlight_enabled: bool,
    /// Theme stack for styled output.
    theme_stack: ThemeStack,
    /// Whether the alt screen is currently active.
    is_alt_screen: bool,
    /// Whether to suppress all output (quiet mode).
    quiet: bool,
    /// Tab size for tab expansion.
    tab_size: usize,
}

impl Console<Stdout> {
    /// Create a new console writing to stdout.
    pub fn new() -> Self {
        let options = ConsoleOptions::from_terminal();
        let color_system = Self::detect_color_system_static(options.is_terminal);

        Console {
            writer: io::stdout(),
            options,
            color_system,
            force_terminal: None,
            legacy_windows: cfg!(windows) && env::var("WT_SESSION").is_err(),
            markup_enabled: true,
            emoji_enabled: true,
            highlight_enabled: true,
            theme_stack: ThemeStack::new(Theme::default()),
            is_alt_screen: false,
            quiet: false,
            tab_size: 8,
        }
    }

    /// Create a console with specific options.
    ///
    /// Console state fields (theme_stack, markup_enabled, etc.) are initialized
    /// from the provided options, ensuring that nested renderables see the
    /// correct state when a temp Console is created from options.
    pub fn with_options(options: ConsoleOptions) -> Self {
        Console {
            writer: io::stdout(),
            // Initialize Console fields from ConsoleOptions state
            color_system: options.color_system,
            markup_enabled: options.markup_enabled,
            emoji_enabled: options.emoji_enabled,
            highlight_enabled: options.highlight_enabled,
            theme_stack: options.theme_stack.clone(),
            tab_size: options.tab_size,
            legacy_windows: options.legacy_windows,
            // Non-state fields
            force_terminal: None,
            is_alt_screen: false,
            quiet: false,
            // Store the options
            options,
        }
    }

    /// Detect color system from environment variables.
    fn detect_color_system_static(is_terminal: bool) -> Option<ColorSystem> {
        // Check NO_COLOR environment variable
        if env::var("NO_COLOR").is_ok() {
            return None;
        }

        if !is_terminal {
            return None;
        }

        // Check COLORTERM for truecolor support
        if let Ok(colorterm) = env::var("COLORTERM") {
            let ct = colorterm.to_lowercase();
            if ct == "truecolor" || ct == "24bit" {
                return Some(ColorSystem::TrueColor);
            }
        }

        // Check TERM for color capabilities
        if let Ok(term) = env::var("TERM") {
            let term_lower = term.to_lowercase();

            // Truecolor terminals
            if term_lower.contains("truecolor")
                || term_lower.contains("24bit")
                || term_lower.contains("direct")
            {
                return Some(ColorSystem::TrueColor);
            }

            // 256 color terminals
            if term_lower.contains("256color")
                || term_lower.contains("kitty")
                || term_lower.contains("alacritty")
            {
                return Some(ColorSystem::EightBit);
            }

            // 16 color terminals
            if term_lower.contains("16color") || term_lower == "xterm" || term_lower == "linux" {
                return Some(ColorSystem::Standard);
            }

            // Dumb terminal
            if term_lower == "dumb" || term_lower == "unknown" {
                return None;
            }
        }

        // Default to 256 colors if terminal is detected
        Some(ColorSystem::EightBit)
    }
}

impl Default for Console<Stdout> {
    fn default() -> Self {
        Console::new()
    }
}

impl Console<Vec<u8>> {
    /// Create a console that captures output to a buffer.
    ///
    /// Use this for testing to capture console output.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::Console;
    ///
    /// let mut console = Console::capture();
    /// console.print_text("Hello").unwrap();
    /// let output = console.get_captured();
    /// assert!(output.contains("Hello"));
    /// ```
    pub fn capture() -> Self {
        Console::with_writer(
            Vec::new(),
            ConsoleOptions {
                is_terminal: false,
                ..Default::default()
            },
        )
    }

    /// Create a capture console with specific options.
    pub fn capture_with_options(options: ConsoleOptions) -> Self {
        Console::with_writer(Vec::new(), options)
    }

    /// Get captured output as a string.
    pub fn get_captured(&self) -> String {
        String::from_utf8_lossy(&self.writer).to_string()
    }

    /// Get captured output as bytes.
    pub fn get_captured_bytes(&self) -> &[u8] {
        &self.writer
    }

    /// Clear the capture buffer.
    pub fn clear_captured(&mut self) {
        self.writer.clear();
    }
}

impl<W: Write> Console<W> {
    /// Create a console with a custom writer.
    ///
    /// Console state fields are initialized from the provided options,
    /// ensuring that nested renderables see the correct state.
    pub fn with_writer(writer: W, options: ConsoleOptions) -> Self {
        Console {
            writer,
            // Initialize Console fields from ConsoleOptions state
            color_system: options.color_system,
            markup_enabled: options.markup_enabled,
            emoji_enabled: options.emoji_enabled,
            highlight_enabled: options.highlight_enabled,
            theme_stack: options.theme_stack.clone(),
            tab_size: options.tab_size,
            legacy_windows: options.legacy_windows,
            // Non-state fields
            force_terminal: None,
            is_alt_screen: false,
            quiet: false,
            // Store the options
            options,
        }
    }

    // ========================================================================
    // Configuration
    // ========================================================================

    /// Get the console options.
    pub fn options(&self) -> &ConsoleOptions {
        &self.options
    }

    /// Get mutable access to console options.
    ///
    /// # Warning
    ///
    /// Modifying state fields (`markup_enabled`, `emoji_enabled`, `highlight_enabled`,
    /// `tab_size`, `color_system`, `theme_stack`) via `options_mut()` will NOT update
    /// the corresponding `Console` fields. This can cause inconsistent behavior.
    ///
    /// Use the specific setters (`set_markup_enabled()`, `set_tab_size()`, etc.) to
    /// modify these fields, which keep both in sync. Or call `sync_from_options()`
    /// after modifying options directly.
    ///
    /// This method is safe for modifying non-state fields like `max_width`, `justify`, etc.
    pub fn options_mut(&mut self) -> &mut ConsoleOptions {
        &mut self.options
    }

    /// Sync Console fields from options.
    ///
    /// Call this after modifying state fields via `options_mut()` to ensure
    /// Console fields stay in sync with options.
    pub fn sync_from_options(&mut self) {
        self.markup_enabled = self.options.markup_enabled;
        self.emoji_enabled = self.options.emoji_enabled;
        self.highlight_enabled = self.options.highlight_enabled;
        self.tab_size = self.options.tab_size;
        self.color_system = self.options.color_system;
        self.theme_stack = self.options.theme_stack.clone();
        self.legacy_windows = self.options.legacy_windows;
    }

    /// Get a copy of options with current console state.
    ///
    /// Since Console setters now keep `self.options` in sync, this just clones
    /// the options. It ensures caller-provided options will have correct state
    /// if they were derived from `console.options()`.
    ///
    /// # Note
    ///
    /// If a caller creates `ConsoleOptions` from scratch (not derived from
    /// `console.options()`), they should ensure state fields (theme_stack,
    /// markup_enabled, etc.) are set appropriately. The console state is
    /// passed through `ConsoleOptions`, not through the `Console` reference.
    pub fn options_with_state(&self) -> ConsoleOptions {
        // Since setters keep self.options in sync, just return a clone
        self.options.clone()
    }

    /// Get the terminal width.
    pub fn width(&self) -> usize {
        self.options.max_width
    }

    /// Get the terminal height.
    pub fn height(&self) -> usize {
        self.options.max_height
    }

    /// Get the terminal size as (width, height).
    pub fn size(&self) -> (usize, usize) {
        self.options.size
    }

    /// Set the terminal size.
    pub fn set_size(&mut self, width: usize, height: usize) {
        self.options.size = (width, height);
        self.options.max_width = width;
        self.options.max_height = height;
    }

    /// Check if the console is writing to a terminal.
    pub fn is_terminal(&self) -> bool {
        self.force_terminal.unwrap_or(self.options.is_terminal)
    }

    /// Force terminal mode on or off.
    pub fn set_force_terminal(&mut self, force: Option<bool>) {
        self.force_terminal = force;
    }

    /// Get the color system.
    pub fn color_system(&self) -> Option<ColorSystem> {
        self.color_system
    }

    /// Set the color system.
    pub fn set_color_system(&mut self, system: Option<ColorSystem>) {
        self.color_system = system;
        self.options.color_system = system;
    }

    /// Check if markup is enabled by default.
    pub fn is_markup_enabled(&self) -> bool {
        self.markup_enabled
    }

    /// Enable or disable markup parsing by default.
    pub fn set_markup_enabled(&mut self, enabled: bool) {
        self.markup_enabled = enabled;
        self.options.markup_enabled = enabled;
    }

    /// Check if emoji replacement is enabled by default.
    pub fn is_emoji_enabled(&self) -> bool {
        self.emoji_enabled
    }

    /// Enable or disable emoji replacement by default.
    pub fn set_emoji_enabled(&mut self, enabled: bool) {
        self.emoji_enabled = enabled;
        self.options.emoji_enabled = enabled;
    }

    /// Check if highlighting is enabled by default.
    pub fn is_highlight_enabled(&self) -> bool {
        self.highlight_enabled
    }

    /// Enable or disable highlighting by default.
    pub fn set_highlight_enabled(&mut self, enabled: bool) {
        self.highlight_enabled = enabled;
        self.options.highlight_enabled = enabled;
    }

    /// Get the tab size.
    pub fn tab_size(&self) -> usize {
        self.tab_size
    }

    /// Set the tab size.
    pub fn set_tab_size(&mut self, size: usize) {
        self.tab_size = size;
        self.options.tab_size = size;
    }

    /// Check if quiet mode is enabled.
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    /// Enable or disable quiet mode (suppress all output).
    pub fn set_quiet(&mut self, quiet: bool) {
        self.quiet = quiet;
    }

    /// Get a reference to the theme stack.
    pub fn theme_stack(&self) -> &ThemeStack {
        &self.theme_stack
    }

    /// Get a mutable reference to the theme stack.
    ///
    /// # Warning
    ///
    /// Modifying the theme stack directly will NOT update `self.options.theme_stack`.
    /// This can cause nested renderables (which read from options) to see stale theme data.
    ///
    /// Prefer using `push_theme()` and `pop_theme()` which keep both stacks in sync.
    /// If you need direct access, call `sync_theme_to_options()` after modifications.
    pub fn theme_stack_mut(&mut self) -> &mut ThemeStack {
        &mut self.theme_stack
    }

    /// Sync the options theme stack from the Console theme stack.
    ///
    /// Call this after modifying the theme stack via `theme_stack_mut()` to ensure
    /// nested renderables see the updated theme.
    pub fn sync_theme_to_options(&mut self) {
        self.options.theme_stack = self.theme_stack.clone();
    }

    /// Push a new theme onto the stack.
    ///
    /// If `inherit` is true, the new theme inherits styles from the current theme.
    pub fn push_theme(&mut self, theme: Theme) {
        self.theme_stack.push_theme(theme.clone());
        self.options.theme_stack.push_theme(theme);
    }

    /// Pop the top theme from the stack.
    ///
    /// Returns an error if trying to pop the base theme.
    pub fn pop_theme(&mut self) -> Result<(), crate::theme::ThemeError> {
        self.theme_stack.pop_theme()?;
        self.options.theme_stack.pop_theme()
    }

    // ========================================================================
    // Core Render Methods
    // ========================================================================

    // Note: The Renderable trait takes &Console (defaults to Console<Stdout>) plus
    // &ConsoleOptions. Console state (theme_stack, markup_enabled, etc.) is now
    // passed through ConsoleOptions, so renderables can access it without needing
    // a direct reference to the caller's Console<W>.

    /// Render to a grid of lines (for layout).
    ///
    /// # Arguments
    ///
    /// * `renderable` - The object to render.
    /// * `options` - Optional custom options, or None to use console defaults.
    ///   If provided, ensure these options include the console state fields
    ///   (theme_stack, markup_enabled, etc.) by deriving from `console.options()`.
    /// * `style` - Optional style to apply to all segments.
    /// * `pad` - Whether to pad lines to the full width.
    /// * `new_lines` - Whether to include newline segments at the end of lines.
    pub fn render_lines<R: Renderable + ?Sized>(
        &self,
        renderable: &R,
        options: Option<&ConsoleOptions>,
        style: Option<Style>,
        pad: bool,
        new_lines: bool,
    ) -> Vec<Vec<Segment>> {
        // Use provided options or console's options (which include state)
        let render_options = options.cloned().unwrap_or_else(|| self.options.clone());

        // Create a temp Console<Stdout> for the rendering call.
        // Console::with_options() initializes console fields from options,
        // so nested renderables will see the correct state.
        let temp_console = Console::<Stdout>::with_options(render_options.clone());
        let segments = renderable.render(&temp_console, &render_options);

        // Apply style if provided
        let segments = if let Some(s) = style {
            Segment::apply_style_to_segments(segments, Some(s), None)
        } else {
            segments
        };

        // Split and crop lines
        let width = render_options.max_width;
        Segment::split_and_crop_lines(segments, width, style, pad, new_lines)
    }

    /// Render a string to Text with optional markup/emoji/highlight.
    ///
    /// This method converts a string to a Text object, applying:
    /// - Markup parsing (if enabled)
    /// - Emoji replacement (if enabled)
    /// - Syntax highlighting (if highlighter provided)
    ///
    /// # Arguments
    ///
    /// * `text` - The text to render.
    /// * `markup` - Whether to parse markup, or None to use console default.
    /// * `emoji` - Whether to replace emoji codes, or None to use console default.
    /// * `highlight` - Whether to apply highlighting, or None to use console default.
    /// * `highlighter` - Optional highlighter to apply.
    pub fn render_str(
        &self,
        text: &str,
        markup: Option<bool>,
        emoji: Option<bool>,
        highlight: Option<bool>,
        highlighter: Option<&dyn Highlighter>,
    ) -> Text {
        let markup_enabled = markup.unwrap_or(self.markup_enabled);
        let emoji_enabled = emoji.unwrap_or(self.emoji_enabled);
        let highlight_enabled = highlight.unwrap_or(self.highlight_enabled);

        // Start with the input text, possibly with emoji replaced
        let processed_text = if emoji_enabled {
            Emoji::replace(text)
        } else {
            text.to_string()
        };

        // Parse markup if enabled
        let mut result = if markup_enabled {
            Text::from_markup(&processed_text, false)
                .unwrap_or_else(|_| Text::plain(&processed_text))
        } else {
            Text::plain(&processed_text)
        };

        // Apply highlighter if provided and highlighting is enabled
        if let (true, Some(hl)) = (highlight_enabled, highlighter) {
            hl.highlight(&mut result);
        }

        result
    }

    // ========================================================================
    // Output Methods
    // ========================================================================

    /// Write raw bytes to the output.
    pub fn write_raw(&mut self, data: &[u8]) -> io::Result<()> {
        if self.quiet {
            return Ok(());
        }
        self.writer.write_all(data)?;
        self.writer.flush()
    }

    /// Write a string directly to the output.
    pub fn write_str(&mut self, s: &str) -> io::Result<()> {
        self.write_raw(s.as_bytes())
    }

    /// Print plain text with a newline.
    pub fn print_text(&mut self, text: &str) -> io::Result<()> {
        if self.quiet {
            return Ok(());
        }
        writeln!(self.writer, "{}", text)?;
        self.writer.flush()
    }

    /// Print styled text with a newline.
    pub fn print_styled(&mut self, text: &str, style: Style) -> io::Result<()> {
        if self.quiet {
            return Ok(());
        }
        // Only apply ANSI styling if color system is available
        if let Some(color_system) = self.color_system {
            let styled = style.render(text, color_system);
            writeln!(self.writer, "{}", styled)?;
        } else {
            writeln!(self.writer, "{}", text)?;
        }
        self.writer.flush()
    }

    /// Print a segment.
    pub fn print_segment(&mut self, segment: &Segment) -> io::Result<()> {
        if self.quiet {
            return Ok(());
        }

        if let Some(style) = segment.style {
            if let Some(color_system) = self.color_system {
                let styled = style.render(&segment.text, color_system);
                write!(self.writer, "{}", styled)?;
            } else {
                write!(self.writer, "{}", segment.text)?;
            }
        } else {
            write!(self.writer, "{}", segment.text)?;
        }
        self.writer.flush()
    }

    /// Print multiple segments.
    pub fn print_segments(&mut self, segments: &Segments) -> io::Result<()> {
        if self.quiet {
            return Ok(());
        }

        for segment in segments.iter() {
            self.print_segment(segment)?;
        }
        Ok(())
    }

    /// Print a renderable object.
    ///
    /// This is the main method for printing content to the console.
    /// It renders the object to segments and writes them to the output.
    ///
    /// # Arguments
    ///
    /// * `renderable` - The object to render and print.
    /// * `style` - Optional style to apply to all output.
    /// * `justify` - Optional justify override.
    /// * `overflow` - Optional overflow override.
    /// * `no_wrap` - Whether to disable word wrapping.
    /// * `end` - String to print at the end (default "\n").
    pub fn print<R: Renderable + ?Sized>(
        &mut self,
        renderable: &R,
        style: Option<Style>,
        justify: Option<JustifyMethod>,
        overflow: Option<OverflowMethod>,
        no_wrap: bool,
        end: &str,
    ) -> io::Result<()> {
        if self.quiet {
            return Ok(());
        }

        // Create options with overrides - self.options already contains console state
        let options = self.options.update(
            None,
            None,
            None,
            Some(justify),
            Some(overflow),
            Some(no_wrap),
            None,
            None,
            None,
        );

        // Create a temp Console<Stdout> for the rendering call.
        // Console::with_options() initializes console fields from options.
        let temp_console = Console::<Stdout>::with_options(options.clone());

        // Render to segments
        let segments = renderable.render(&temp_console, &options);

        // Apply style if provided
        let segments = if let Some(s) = style {
            Segment::apply_style_to_segments(segments, Some(s), None)
        } else {
            segments
        };

        // Print segments
        self.print_segments(&segments)?;

        // Print end string
        if !end.is_empty() {
            write!(self.writer, "{}", end)?;
        }

        self.writer.flush()
    }

    /// Render a line (horizontal rule).
    pub fn rule(&mut self, title: Option<&str>) -> io::Result<()> {
        if self.quiet {
            return Ok(());
        }

        let width = self.width();
        match title {
            Some(t) => {
                // Use cell_len for correct width calculation with wide characters
                let title_width = crate::cells::cell_len(t);
                let padding = (width.saturating_sub(title_width + 2)) / 2;
                let line: String = "─".repeat(padding);
                writeln!(self.writer, "{} {} {}", line, t, line)?;
            }
            None => {
                let line: String = "─".repeat(width);
                writeln!(self.writer, "{}", line)?;
            }
        }
        self.writer.flush()
    }

    /// Print new line(s).
    pub fn line(&mut self, count: usize) -> io::Result<()> {
        if self.quiet {
            return Ok(());
        }

        for _ in 0..count {
            writeln!(self.writer)?;
        }
        self.writer.flush()
    }

    // ========================================================================
    // Terminal Control
    // ========================================================================

    /// Clear the screen.
    pub fn clear(&mut self) -> io::Result<()> {
        if !self.is_terminal() {
            return Ok(());
        }
        execute!(self.writer, ct::Clear(ClearType::All))?;
        execute!(self.writer, cursor::MoveTo(0, 0))?;
        self.writer.flush()
    }

    /// Clear the current line.
    pub fn clear_line(&mut self) -> io::Result<()> {
        if !self.is_terminal() {
            return Ok(());
        }
        execute!(self.writer, ct::Clear(ClearType::CurrentLine))?;
        self.writer.flush()
    }

    /// Move the cursor to a specific position.
    pub fn move_cursor(&mut self, x: u16, y: u16) -> io::Result<()> {
        if !self.is_terminal() {
            return Ok(());
        }
        execute!(self.writer, cursor::MoveTo(x, y))?;
        self.writer.flush()
    }

    /// Show or hide the cursor.
    pub fn show_cursor(&mut self, show: bool) -> io::Result<bool> {
        if !self.is_terminal() {
            return Ok(false);
        }
        if show {
            execute!(self.writer, cursor::Show)?;
        } else {
            execute!(self.writer, cursor::Hide)?;
        }
        self.writer.flush()?;
        Ok(true)
    }

    /// Enter alternate screen mode.
    ///
    /// The alternate screen is a separate screen buffer that can be used
    /// for full-screen applications. Call `leave_alt_screen` when done.
    pub fn enter_alt_screen(&mut self) -> io::Result<bool> {
        if !self.is_terminal() || self.legacy_windows {
            return Ok(false);
        }
        execute!(self.writer, ct::EnterAlternateScreen)?;
        self.is_alt_screen = true;
        self.writer.flush()?;
        Ok(true)
    }

    /// Leave alternate screen mode.
    pub fn leave_alt_screen(&mut self) -> io::Result<bool> {
        if !self.is_terminal() || !self.is_alt_screen {
            return Ok(false);
        }
        execute!(self.writer, ct::LeaveAlternateScreen)?;
        self.is_alt_screen = false;
        self.writer.flush()?;
        Ok(true)
    }

    /// Check if alternate screen mode is active.
    pub fn is_alt_screen(&self) -> bool {
        self.is_alt_screen
    }

    /// Set the window title.
    pub fn set_window_title(&mut self, title: &str) -> io::Result<bool> {
        if !self.is_terminal() {
            return Ok(false);
        }
        execute!(self.writer, ct::SetTitle(title))?;
        self.writer.flush()?;
        Ok(true)
    }

    /// Ring the terminal bell.
    pub fn bell(&mut self) -> io::Result<()> {
        write!(self.writer, "\x07")?;
        self.writer.flush()
    }

    // ========================================================================
    // Measurement
    // ========================================================================

    /// Measure a renderable object.
    ///
    /// Returns the minimum and maximum width required to render the object.
    ///
    /// # Arguments
    ///
    /// * `renderable` - The object to measure.
    /// * `options` - Optional custom options, or None to use console defaults.
    ///   If provided, ensure these options include the console state fields
    ///   by deriving from `console.options()`.
    pub fn measure<R: Renderable + ?Sized>(
        &self,
        renderable: &R,
        options: Option<&ConsoleOptions>,
    ) -> crate::measure::Measurement {
        // Use provided options or console's options (which include state)
        let measure_opts = options.cloned().unwrap_or_else(|| self.options.clone());

        // Create a temp Console<Stdout> for the measure call.
        // Console::with_options() initializes console fields from options.
        let temp_console = Console::<Stdout>::with_options(measure_opts.clone());
        renderable.measure(&temp_console, &measure_opts)
    }
}

// Implement render and render_with_options specifically for Console<Stdout>
// since the Renderable trait requires &Console<Stdout>
impl Console<Stdout> {
    /// Render a Renderable to Segments.
    pub fn render<R: Renderable + ?Sized>(&self, renderable: &R) -> Segments {
        renderable.render(self, &self.options)
    }

    /// Render a Renderable with custom options.
    pub fn render_with_options<R: Renderable + ?Sized>(
        &self,
        renderable: &R,
        options: &ConsoleOptions,
    ) -> Segments {
        renderable.render(self, options)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== JustifyMethod tests ====================

    #[test]
    fn test_justify_method_parse() {
        assert_eq!(JustifyMethod::parse("left"), Some(JustifyMethod::Left));
        assert_eq!(JustifyMethod::parse("CENTER"), Some(JustifyMethod::Center));
        assert_eq!(JustifyMethod::parse("Right"), Some(JustifyMethod::Right));
        assert_eq!(JustifyMethod::parse("full"), Some(JustifyMethod::Full));
        assert_eq!(
            JustifyMethod::parse("default"),
            Some(JustifyMethod::Default)
        );
        assert_eq!(JustifyMethod::parse("invalid"), None);
    }

    // ==================== OverflowMethod tests ====================

    #[test]
    fn test_overflow_method_parse() {
        assert_eq!(OverflowMethod::parse("fold"), Some(OverflowMethod::Fold));
        assert_eq!(OverflowMethod::parse("CROP"), Some(OverflowMethod::Crop));
        assert_eq!(
            OverflowMethod::parse("Ellipsis"),
            Some(OverflowMethod::Ellipsis)
        );
        assert_eq!(
            OverflowMethod::parse("ignore"),
            Some(OverflowMethod::Ignore)
        );
        assert_eq!(OverflowMethod::parse("invalid"), None);
    }

    // ==================== ConsoleOptions tests ====================

    #[test]
    fn test_console_options_default() {
        let options = ConsoleOptions::default();
        assert_eq!(options.size, (80, 24));
        assert_eq!(options.min_width, 1);
        assert_eq!(options.max_width, 80);
        assert_eq!(options.max_height, 24);
        assert!(options.is_terminal);
    }

    #[test]
    fn test_console_options_ascii_only() {
        let options = ConsoleOptions {
            encoding: "utf-8".to_string(),
            ..Default::default()
        };
        assert!(!options.ascii_only());

        let options = ConsoleOptions {
            encoding: "ascii".to_string(),
            ..Default::default()
        };
        assert!(options.ascii_only());

        let options = ConsoleOptions {
            encoding: "latin-1".to_string(),
            ..Default::default()
        };
        assert!(options.ascii_only());
    }

    #[test]
    fn test_console_options_update_width() {
        let options = ConsoleOptions::default();
        let updated = options.update_width(120);
        assert_eq!(updated.min_width, 120);
        assert_eq!(updated.max_width, 120);
    }

    #[test]
    fn test_console_options_update_height() {
        let options = ConsoleOptions::default();
        let updated = options.update_height(40);
        assert_eq!(updated.max_height, 40);
        assert_eq!(updated.height, Some(40));
    }

    #[test]
    fn test_console_options_update_dimensions() {
        let options = ConsoleOptions::default();
        let updated = options.update_dimensions(100, 50);
        assert_eq!(updated.min_width, 100);
        assert_eq!(updated.max_width, 100);
        assert_eq!(updated.max_height, 50);
        assert_eq!(updated.height, Some(50));
    }

    #[test]
    fn test_console_options_reset_height() {
        let options = ConsoleOptions {
            height: Some(40),
            ..Default::default()
        };
        let reset = options.reset_height();
        assert_eq!(reset.height, None);
    }

    // ==================== Console capture tests ====================

    #[test]
    fn test_console_capture() {
        let mut console = Console::capture();
        console.print_text("Hello, World!").unwrap();
        let output = console.get_captured();
        assert!(output.contains("Hello, World!"));
    }

    #[test]
    fn test_console_capture_styled() {
        let mut console = Console::capture();
        let style = Style::new().with_bold(true);
        console.print_styled("Bold text", style).unwrap();
        let output = console.get_captured();
        assert!(output.contains("Bold text"));
    }

    #[test]
    fn test_console_capture_clear() {
        let mut console = Console::capture();
        console.print_text("First").unwrap();
        console.clear_captured();
        console.print_text("Second").unwrap();
        let output = console.get_captured();
        assert!(!output.contains("First"));
        assert!(output.contains("Second"));
    }

    #[test]
    fn test_console_capture_bytes() {
        let mut console = Console::capture();
        console.print_text("Test").unwrap();
        let bytes = console.get_captured_bytes();
        assert!(!bytes.is_empty());
    }

    // ==================== Console configuration tests ====================

    #[test]
    fn test_console_width() {
        let console = Console::capture_with_options(ConsoleOptions {
            max_width: 120,
            ..Default::default()
        });
        assert_eq!(console.width(), 120);
    }

    #[test]
    fn test_console_set_size() {
        let mut console = Console::capture();
        console.set_size(100, 50);
        assert_eq!(console.width(), 100);
        assert_eq!(console.height(), 50);
        assert_eq!(console.size(), (100, 50));
    }

    #[test]
    fn test_console_force_terminal() {
        let mut console = Console::capture();
        assert!(!console.is_terminal()); // Capture is not terminal by default

        console.set_force_terminal(Some(true));
        assert!(console.is_terminal());

        console.set_force_terminal(Some(false));
        assert!(!console.is_terminal());
    }

    #[test]
    fn test_console_quiet_mode() {
        let mut console = Console::capture();
        console.set_quiet(true);
        console.print_text("This should not appear").unwrap();
        assert!(console.get_captured().is_empty());
    }

    #[test]
    fn test_console_markup_emoji_highlight() {
        let mut console = Console::capture();

        assert!(console.is_markup_enabled());
        console.set_markup_enabled(false);
        assert!(!console.is_markup_enabled());

        assert!(console.is_emoji_enabled());
        console.set_emoji_enabled(false);
        assert!(!console.is_emoji_enabled());

        assert!(console.is_highlight_enabled());
        console.set_highlight_enabled(false);
        assert!(!console.is_highlight_enabled());
    }

    #[test]
    fn test_console_tab_size() {
        let mut console = Console::capture();
        assert_eq!(console.tab_size(), 8);
        console.set_tab_size(4);
        assert_eq!(console.tab_size(), 4);
    }

    // ==================== Console render tests ====================

    #[test]
    fn test_console_render_text() {
        // Use Console<Stdout> directly for render methods
        let console = Console::with_options(ConsoleOptions::default());
        let text = Text::plain("Hello, World!");
        let segments = console.render(&text);
        assert!(!segments.is_empty());

        let combined: String = segments.iter().map(|s| s.text.to_string()).collect();
        assert_eq!(combined, "Hello, World!");
    }

    #[test]
    fn test_console_render_str() {
        let console = Console::capture();
        let text = console.render_str("Hello", None, None, None, None);
        assert_eq!(text.plain_text(), "Hello");
    }

    #[test]
    fn test_console_render_str_with_emoji() {
        let console = Console::capture();
        let text = console.render_str(":smile:", None, Some(true), None, None);
        // Should contain the emoji or the original text if emoji not found
        assert!(!text.plain_text().is_empty());
    }

    // ==================== Console print tests ====================

    #[test]
    fn test_console_print_renderable() {
        let mut console = Console::capture();
        let text = Text::plain("Hello");
        console.print(&text, None, None, None, false, "\n").unwrap();
        let output = console.get_captured();
        assert!(output.contains("Hello"));
    }

    #[test]
    fn test_console_print_with_style() {
        let mut console = Console::capture();
        let text = Text::plain("Styled");
        let style = Style::new().with_bold(true);
        console
            .print(&text, Some(style), None, None, false, "\n")
            .unwrap();
        let output = console.get_captured();
        assert!(output.contains("Styled"));
    }

    #[test]
    fn test_console_rule() {
        let mut console = Console::capture();
        console.rule(None).unwrap();
        let output = console.get_captured();
        assert!(output.contains("─"));
    }

    #[test]
    fn test_console_rule_with_title() {
        let mut console = Console::capture();
        console.rule(Some("Title")).unwrap();
        let output = console.get_captured();
        assert!(output.contains("Title"));
        assert!(output.contains("─"));
    }

    #[test]
    fn test_console_line() {
        let mut console = Console::capture();
        console.line(3).unwrap();
        let output = console.get_captured();
        assert_eq!(output.matches('\n').count(), 3);
    }

    // ==================== Console measure tests ====================

    #[test]
    fn test_console_measure() {
        let console = Console::capture();
        let text = Text::plain("Hello World");
        let measurement = console.measure(&text, None);
        assert!(measurement.minimum > 0);
        assert!(measurement.maximum >= measurement.minimum);
    }

    // ==================== Console alt screen tests (unit only) ====================

    #[test]
    fn test_console_alt_screen_tracking() {
        let console = Console::capture();
        assert!(!console.is_alt_screen());
        // Can't actually test enter/leave without a real terminal
    }

    // ==================== Theme tests ====================

    #[test]
    fn test_console_theme_stack() {
        let mut console = Console::capture();
        let theme = Theme::default();
        console.push_theme(theme.clone());
        assert!(console.pop_theme().is_ok());
    }

    // ==================== Color system detection tests ====================

    #[test]
    fn test_color_system_detection_no_terminal() {
        let result = Console::<Stdout>::detect_color_system_static(false);
        assert!(result.is_none());
    }

    // ==================== State propagation tests ====================

    #[test]
    fn test_console_setters_sync_to_options() {
        let mut console = Console::capture();

        // Test set_markup_enabled
        console.set_markup_enabled(false);
        assert!(!console.is_markup_enabled());
        assert!(!console.options().markup_enabled);

        // Test set_emoji_enabled
        console.set_emoji_enabled(false);
        assert!(!console.is_emoji_enabled());
        assert!(!console.options().emoji_enabled);

        // Test set_highlight_enabled
        console.set_highlight_enabled(false);
        assert!(!console.is_highlight_enabled());
        assert!(!console.options().highlight_enabled);

        // Test set_tab_size
        console.set_tab_size(4);
        assert_eq!(console.tab_size(), 4);
        assert_eq!(console.options().tab_size, 4);

        // Test set_color_system
        console.set_color_system(Some(ColorSystem::TrueColor));
        assert_eq!(console.color_system(), Some(ColorSystem::TrueColor));
        assert_eq!(console.options().color_system, Some(ColorSystem::TrueColor));
    }

    #[test]
    fn test_console_options_with_state() {
        let mut console = Console::capture();

        // Modify console state
        console.set_markup_enabled(false);
        console.set_tab_size(2);

        // Get options with state
        let opts = console.options_with_state();
        assert!(!opts.markup_enabled);
        assert_eq!(opts.tab_size, 2);
    }

    #[test]
    fn test_with_options_initializes_from_options() {
        // Create options with custom state
        let mut options = ConsoleOptions::default();
        options.markup_enabled = false;
        options.emoji_enabled = false;
        options.tab_size = 4;
        options.color_system = Some(ColorSystem::Standard);

        // Create console from options
        let console = Console::with_options(options);

        // Console fields should match options
        assert!(!console.is_markup_enabled());
        assert!(!console.is_emoji_enabled());
        assert_eq!(console.tab_size(), 4);
        assert_eq!(console.color_system(), Some(ColorSystem::Standard));
    }

    #[test]
    fn test_sync_from_options() {
        let mut console = Console::capture();

        // Modify options directly (not recommended, but supported)
        console.options_mut().markup_enabled = false;
        console.options_mut().tab_size = 2;

        // Console fields are now out of sync
        assert!(console.is_markup_enabled()); // Still true!
        assert_eq!(console.tab_size(), 8); // Still 8!

        // Sync from options
        console.sync_from_options();

        // Now console fields match options
        assert!(!console.is_markup_enabled());
        assert_eq!(console.tab_size(), 2);
    }

    #[test]
    fn test_sync_theme_to_options() {
        let mut console = Console::capture();

        // Modify theme stack directly (not recommended, but supported)
        let mut custom_theme = Theme::empty();
        custom_theme.add_style("direct.style", Style::new().with_italic(true));
        console.theme_stack_mut().push_theme(custom_theme);

        // Options theme stack is now out of sync
        assert!(console.theme_stack().get_style("direct.style").is_some());
        assert!(
            console
                .options()
                .theme_stack
                .get_style("direct.style")
                .is_none()
        ); // Out of sync!

        // Sync theme to options
        console.sync_theme_to_options();

        // Now options theme stack matches
        assert!(
            console
                .options()
                .theme_stack
                .get_style("direct.style")
                .is_some()
        );
    }

    #[test]
    fn test_nested_renderable_gets_state() {
        // This test verifies that when Padding (which calls render_lines internally)
        // renders, the inner renderable gets the correct console state.

        use crate::padding::Padding;

        let mut console = Console::capture();
        console.set_markup_enabled(false);

        // Create a simple padding around text
        let text = Text::plain("Hello");
        let padded = Padding::new(Box::new(text), 1);

        // Render - if state doesn't propagate, this would crash or produce wrong output
        let options = console.options().clone();
        let segments = padded.render(&Console::with_options(options.clone()), &options);

        // Verify rendering worked (basic check)
        assert!(!segments.is_empty());
    }

    #[test]
    fn test_theme_push_syncs_to_options() {
        let mut console = Console::capture();

        // Get initial depth
        let initial_depth = console.theme_stack().depth();
        let initial_opts_depth = console.options().theme_stack.depth();
        assert_eq!(initial_depth, initial_opts_depth);

        // Push a theme
        let mut custom_theme = Theme::empty();
        custom_theme.add_style("test.style", Style::new().with_bold(true));
        console.push_theme(custom_theme);

        // Both should have increased depth
        assert_eq!(console.theme_stack().depth(), initial_depth + 1);
        assert_eq!(console.options().theme_stack.depth(), initial_depth + 1);

        // Both should see the new style
        assert!(console.theme_stack().get_style("test.style").is_some());
        assert!(
            console
                .options()
                .theme_stack
                .get_style("test.style")
                .is_some()
        );

        // Pop the theme
        console.pop_theme().unwrap();

        // Both should be back to original depth
        assert_eq!(console.theme_stack().depth(), initial_depth);
        assert_eq!(console.options().theme_stack.depth(), initial_depth);
    }

    // ==================== Send + Sync assertions ====================

    #[test]
    fn test_console_options_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<ConsoleOptions>();
        assert_sync::<ConsoleOptions>();
    }
}
