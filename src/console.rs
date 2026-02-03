//! Console: the main API for rendering to the terminal.
//!
//! The Console is the central orchestrator for all Rich output. It handles:
//! - Terminal capabilities detection
//! - Rendering renderables to segments
//! - Writing styled output to the terminal
//! - Alternate screen mode
//! - Output capture for testing

use std::collections::HashMap;
use std::env;
use std::io::{self, Stdout, Write};
use std::sync::Arc;

use crossterm::terminal::{self, ClearType};
use crossterm::{cursor, execute, terminal as ct};

use crate::Renderable;
use crate::color::ColorSystem;
use crate::emoji::Emoji;
use crate::highlighter::Highlighter;
use crate::segment::{ControlType, Segment, Segments};
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
    /// Current theme name (e.g., "default", "dracula").
    /// Renderables can use this to match their theme to the console.
    pub theme_name: String,
    /// Whether markup parsing is enabled by default.
    pub markup_enabled: bool,
    /// Whether emoji replacement is enabled by default.
    pub emoji_enabled: bool,
    /// Whether highlighting is enabled by default.
    pub highlight_enabled: bool,
    /// Tab size for tab expansion.
    pub tab_size: usize,
    /// Disable terminal automatic line wrap while printing.
    ///
    /// This prevents "soft wrap" artifacts when output fills exactly the last
    /// column of the terminal (common on Windows Terminal and xterm-like VTs).
    ///
    /// When enabled and writing to a real terminal, the Console will emit
    /// `ESC[?7l` before printing and `ESC[?7h` after.
    pub disable_line_wrap: bool,
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
            theme_name: "default".to_string(),
            markup_enabled: true,
            emoji_enabled: true,
            highlight_enabled: true,
            tab_size: 8,
            disable_line_wrap: false,
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
            max_width: width.max(1),
            max_height: height,
            height: None,
            is_terminal,
            // Avoid soft-wrap artifacts by temporarily disabling automatic line wrap.
            // This allows layouts to use the full terminal width while still preventing
            // terminals from inserting an extra wrapped line when writing in the last column.
            disable_line_wrap: true,
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
    /// Current theme name for renderables to use.
    theme_name: String,
    /// Whether the alt screen is currently active.
    is_alt_screen: bool,
    /// Whether to suppress all output (quiet mode).
    quiet: bool,
    /// Tab size for tab expansion.
    tab_size: usize,
    /// Live display manager (Live/Progress).
    live: LiveManager,
    /// Stable hyperlink id registry (per-console).
    link_ids: HashMap<Arc<str>, Arc<str>>,
    /// Next id counter for generated hyperlinks.
    next_link_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiveVerticalOverflow {
    Crop,
    Ellipsis,
    Visible,
}

impl From<crate::live::VerticalOverflowMethod> for LiveVerticalOverflow {
    fn from(v: crate::live::VerticalOverflowMethod) -> Self {
        match v {
            crate::live::VerticalOverflowMethod::Crop => Self::Crop,
            crate::live::VerticalOverflowMethod::Ellipsis => Self::Ellipsis,
            crate::live::VerticalOverflowMethod::Visible => Self::Visible,
        }
    }
}

struct LiveEntry {
    renderable: Box<dyn crate::Renderable + Send + Sync>,
    vertical_overflow: LiveVerticalOverflow,
}

#[derive(Default)]
struct LiveManager {
    next_id: usize,
    stack: Vec<usize>,
    entries: HashMap<usize, LiveEntry>,
    shape: Option<(usize, usize)>,
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
            theme_name: "default".to_string(),
            is_alt_screen: false,
            quiet: false,
            tab_size: 8,
            live: LiveManager::default(),
            link_ids: HashMap::new(),
            next_link_id: 1,
        }
    }

    /// Set the console theme by name.
    ///
    /// This sets the base theme for all renderables. Renderables like `Pretty` and
    /// `Syntax` will automatically use this theme unless they have an explicit theme set.
    ///
    /// Available themes: "default", "dracula", "gruvbox-dark", "nord"
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::Console;
    ///
    /// let console = Console::new().with_theme("dracula");
    /// ```
    pub fn with_theme(mut self, name: &str) -> Self {
        if let Some(theme) = Theme::from_name(name) {
            self.theme_stack = ThemeStack::new(theme.clone());
            self.options.theme_stack = ThemeStack::new(theme);
            self.theme_name = name.to_string();
            self.options.theme_name = name.to_string();
        }
        self
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
            theme_name: options.theme_name.clone(),
            tab_size: options.tab_size,
            legacy_windows: options.legacy_windows,
            // Non-state fields
            force_terminal: None,
            is_alt_screen: false,
            quiet: false,
            // Store the options
            options,
            live: LiveManager::default(),
            link_ids: HashMap::new(),
            next_link_id: 1,
        }
    }

    /// Detect color system from environment variables.
    fn detect_color_system_static(is_terminal: bool) -> Option<ColorSystem> {
        // Check NO_COLOR environment variable (takes precedence)
        if env::var("NO_COLOR").is_ok() {
            return None;
        }

        // Check FORCE_COLOR to enable colors even when not a terminal
        let force_color = env::var("FORCE_COLOR").is_ok();

        if !is_terminal && !force_color {
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
    fn link_id_for_url(&mut self, url: &Arc<str>) -> Arc<str> {
        if let Some(existing) = self.link_ids.get(url) {
            return existing.clone();
        }
        let id: Arc<str> = Arc::from(format!("richrs-{}", self.next_link_id));
        self.next_link_id = self.next_link_id.saturating_add(1);
        self.link_ids.insert(url.clone(), id.clone());
        id
    }

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
            theme_name: options.theme_name.clone(),
            tab_size: options.tab_size,
            legacy_windows: options.legacy_windows,
            // Non-state fields
            force_terminal: None,
            is_alt_screen: false,
            quiet: false,
            // Store the options
            options,
            live: LiveManager::default(),
            link_ids: HashMap::new(),
            next_link_id: 1,
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
        self.theme_name = self.options.theme_name.clone();
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

    /// Check if the terminal is considered "dumb" (no cursor control).
    pub fn is_dumb_terminal(&self) -> bool {
        match env::var("TERM") {
            Ok(term) => {
                let t = term.to_lowercase();
                t == "dumb" || t == "unknown"
            }
            Err(_) => false,
        }
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

    /// Get the current theme name.
    ///
    /// Returns the name of the base theme (e.g., "default", "dracula").
    pub fn theme_name(&self) -> &str {
        &self.theme_name
    }

    /// Set the theme by name.
    ///
    /// This replaces the base theme. Any pushed themes remain on the stack.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::Console;
    ///
    /// let mut console = Console::new();
    /// console.set_theme("dracula");
    /// assert_eq!(console.theme_name(), "dracula");
    /// ```
    pub fn set_theme(&mut self, name: &str) {
        if let Some(theme) = Theme::from_name(name) {
            // Create a new theme stack with the new base theme
            self.theme_stack = ThemeStack::new(theme.clone());
            self.options.theme_stack = ThemeStack::new(theme);
            self.theme_name = name.to_string();
            self.options.theme_name = name.to_string();
        }
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
        if self.is_terminal() && !self.is_dumb_terminal() && self.has_live() {
            return self.print(&Text::plain(text), None, None, None, false, "\n");
        }
        writeln!(self.writer, "{}", text)?;
        self.writer.flush()
    }

    /// Print styled text with a newline.
    pub fn print_styled(&mut self, text: &str, style: Style) -> io::Result<()> {
        if self.quiet {
            return Ok(());
        }
        if self.is_terminal() && !self.is_dumb_terminal() && self.has_live() {
            return self.print(&Text::styled(text, style), None, None, None, false, "\n");
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
    ///
    /// Uses streaming output that avoids resetting styles between segments,
    /// which prevents visual artifacts like black hairlines between colored lines.
    pub fn print_segments(&mut self, segments: &Segments) -> io::Result<()> {
        if self.quiet {
            return Ok(());
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct StyleState {
            fg: crate::color::SimpleColor,
            bg: crate::color::SimpleColor,
            bold: bool,
            dim: bool,
            italic: bool,
            underline: bool,
            blink: bool,
            reverse: bool,
            strike: bool,
        }

        impl StyleState {
            const DEFAULT: Self = Self {
                fg: crate::color::SimpleColor::Default,
                bg: crate::color::SimpleColor::Default,
                bold: false,
                dim: false,
                italic: false,
                underline: false,
                blink: false,
                reverse: false,
                strike: false,
            };

            fn from_style(style: Option<Style>) -> Self {
                let style = style.unwrap_or_default();
                Self {
                    fg: style.color.unwrap_or(crate::color::SimpleColor::Default),
                    bg: style.bgcolor.unwrap_or(crate::color::SimpleColor::Default),
                    bold: style.bold.unwrap_or(false),
                    dim: style.dim.unwrap_or(false),
                    italic: style.italic.unwrap_or(false),
                    underline: style.underline.unwrap_or(false),
                    blink: style.blink.unwrap_or(false),
                    reverse: style.reverse.unwrap_or(false),
                    strike: style.strike.unwrap_or(false),
                }
            }

            fn sgr_diff(self, target: Self, color_system: ColorSystem) -> String {
                if self == target {
                    return String::new();
                }

                let mut sgr: Vec<String> = Vec::new();

                // Reset codes first (explicitly turning attributes off).
                // Note: SGR 22 resets both bold AND dim.
                let needs_22 = (self.bold && !target.bold) || (self.dim && !target.dim);
                if needs_22 {
                    sgr.push("22".to_string());
                }
                if self.italic && !target.italic {
                    sgr.push("23".to_string());
                }
                if self.underline && !target.underline {
                    sgr.push("24".to_string());
                }
                if self.blink && !target.blink {
                    sgr.push("25".to_string());
                }
                if self.reverse && !target.reverse {
                    sgr.push("27".to_string());
                }
                if self.strike && !target.strike {
                    sgr.push("29".to_string());
                }

                // Colors: treat unspecified colors as Default (39/49) to avoid "bleed"
                // between segments when using streaming output.
                if self.fg != target.fg {
                    let fg = target.fg.downgrade(color_system);
                    sgr.extend(fg.get_ansi_codes(true));
                }
                if self.bg != target.bg {
                    let bg = target.bg.downgrade(color_system);
                    sgr.extend(bg.get_ansi_codes(false));
                }

                // Enable codes last.
                if target.bold && (!self.bold || needs_22) {
                    sgr.push("1".to_string());
                }
                if target.dim && (!self.dim || needs_22) {
                    sgr.push("2".to_string());
                }
                if target.italic && !self.italic {
                    sgr.push("3".to_string());
                }
                if target.underline && !self.underline {
                    sgr.push("4".to_string());
                }
                if target.blink && !self.blink {
                    sgr.push("5".to_string());
                }
                if target.reverse && !self.reverse {
                    sgr.push("7".to_string());
                }
                if target.strike && !self.strike {
                    sgr.push("9".to_string());
                }

                sgr.join(";")
            }
        }

        let mut current = StyleState::DEFAULT;
        let mut used_sgr = false;
        let hyperlinks_enabled = self.is_terminal() && !self.is_dumb_terminal();
        let mut current_link: Option<(Arc<str>, Option<Arc<str>>)> = None;
        let mut hyperlink_manual = false;

        for segment in segments.iter() {
            if let Some(control) = &segment.control {
                // Emit terminal controls regardless of style state.
                // Control sequences generally do not alter SGR state.
                match control {
                    ControlType::Bell => write!(self.writer, "\x07")?,
                    ControlType::CarriageReturn => write!(self.writer, "\r")?,
                    ControlType::Home => write!(self.writer, "\x1b[H")?,
                    ControlType::Clear => write!(self.writer, "\x1b[2J\x1b[H")?,
                    ControlType::ShowCursor => write!(self.writer, "\x1b[?25h")?,
                    ControlType::HideCursor => write!(self.writer, "\x1b[?25l")?,
                    ControlType::EnableAltScreen => write!(self.writer, "\x1b[?1049h")?,
                    ControlType::DisableAltScreen => write!(self.writer, "\x1b[?1049l")?,
                    ControlType::SetTitle => {
                        // Not representable without a payload; ignore.
                    }
                    ControlType::CursorUp(n) => write!(self.writer, "\x1b[{}A", n)?,
                    ControlType::CursorDown(n) => write!(self.writer, "\x1b[{}B", n)?,
                    ControlType::CursorForward(n) => write!(self.writer, "\x1b[{}C", n)?,
                    ControlType::CursorBackward(n) => write!(self.writer, "\x1b[{}D", n)?,
                    ControlType::EraseInLine(mode) => write!(self.writer, "\x1b[{}K", mode)?,
                    ControlType::HyperlinkStart { url, id } => {
                        if hyperlinks_enabled {
                            if let Some(id) = id.as_deref() {
                                write!(self.writer, "\x1b]8;id={};{}\x1b\\", id, url)?;
                            } else {
                                write!(self.writer, "\x1b]8;;{}\x1b\\", url)?;
                            }
                            current_link = Some((url.clone(), id.clone()));
                            hyperlink_manual = true;
                        }
                    }
                    ControlType::HyperlinkEnd => {
                        if hyperlinks_enabled {
                            write!(self.writer, "\x1b]8;;\x1b\\")?;
                            current_link = None;
                            hyperlink_manual = false;
                        }
                    }
                    ControlType::MoveTo { x, y } => {
                        // CSI row;col H (1-based)
                        write!(
                            self.writer,
                            "\x1b[{};{}H",
                            (*y as usize) + 1,
                            (*x as usize) + 1
                        )?
                    }
                }
                continue;
            }

            if hyperlinks_enabled && !hyperlink_manual {
                let mut desired_link: Option<(Arc<str>, Option<Arc<str>>)> = None;
                if let Some(meta) = segment.meta.as_ref() {
                    if let Some(url) = meta.link.as_ref() {
                        let url = url.clone();
                        let id = meta
                            .link_id
                            .clone()
                            .or_else(|| Some(self.link_id_for_url(&url)));
                        desired_link = Some((url, id));
                    }
                }

                if desired_link != current_link {
                    // Close any previous link.
                    if current_link.is_some() {
                        write!(self.writer, "\x1b]8;;\x1b\\")?;
                    }
                    // Open the new link.
                    if let Some((url, id)) = &desired_link {
                        if let Some(id) = id.as_deref() {
                            write!(self.writer, "\x1b]8;id={};{}\x1b\\", id, url)?;
                        } else {
                            write!(self.writer, "\x1b]8;;{}\x1b\\", url)?;
                        }
                    }
                    current_link = desired_link;
                }
            }

            if let Some(color_system) = self.color_system {
                let target = StyleState::from_style(segment.style);
                let diff = current.sgr_diff(target, color_system);
                if !diff.is_empty() {
                    write!(self.writer, "\x1b[{}m", diff)?;
                    used_sgr = true;
                }
                write!(self.writer, "{}", segment.text)?;
                current = target;
            } else {
                write!(self.writer, "{}", segment.text)?;
            }
        }

        // Close any active hyperlink so it doesn't leak past the renderable.
        if hyperlinks_enabled && current_link.is_some() {
            write!(self.writer, "\x1b]8;;\x1b\\")?;
        }

        // Reset at the end so terminal state doesn't leak past the renderable.
        if self.color_system.is_some() && used_sgr && current != StyleState::DEFAULT {
            write!(self.writer, "\x1b[0m")?;
        }

        self.writer.flush()
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
        let mut segments = if let Some(s) = style {
            Segment::apply_style_to_segments(segments, Some(s), None)
        } else {
            segments
        };

        let live_active = self.is_terminal() && !self.is_dumb_terminal() && self.has_live();
        let mut end_to_write = end;
        if live_active {
            // When Live is active, the trailing newline belongs to the *printed* content,
            // and the live render must be re-drawn after it (Rich behavior).
            if !end.is_empty() {
                segments.push(Segment::new(end.to_string()));
            }
            end_to_write = "";

            let mut wrapped = Segments::new();
            for seg in self.live_position_cursor().iter() {
                wrapped.push(seg.clone());
            }
            for seg in segments.into_iter() {
                wrapped.push(seg);
            }
            let live_segments = self.render_live_segments(&temp_console, &options);
            for seg in live_segments.into_iter() {
                wrapped.push(seg);
            }
            segments = wrapped;
        }

        let should_disable_wrap = self.options.disable_line_wrap && atty::is(atty::Stream::Stdout);
        if should_disable_wrap {
            // Disable automatic line wrap (DECAWM) so output can use full width
            // without terminals inserting an extra wrapped line.
            write!(self.writer, "\x1b[?7l")?;
        }

        let result = (|| {
            // Print segments
            self.print_segments(&segments)?;

            // Print end string
            if !end_to_write.is_empty() {
                write!(self.writer, "{}", end_to_write)?;
            }

            self.writer.flush()
        })();

        if should_disable_wrap {
            // Always attempt to restore wrap mode.
            let _ = write!(self.writer, "\x1b[?7h");
        }

        result
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

        if self.is_terminal() && !self.is_dumb_terminal() && self.has_live() {
            for _ in 0..count {
                self.print(&Text::plain(""), None, None, None, false, "\n")?;
            }
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
        self.set_alt_screen(true)
    }

    /// Leave alternate screen mode.
    pub fn leave_alt_screen(&mut self) -> io::Result<bool> {
        if !self.is_terminal() || !self.is_alt_screen {
            return Ok(false);
        }
        self.set_alt_screen(false)
    }

    /// Check if alternate screen mode is active.
    pub fn is_alt_screen(&self) -> bool {
        self.is_alt_screen
    }

    /// Enable or disable alternate screen mode (Rich parity).
    ///
    /// When enabling, Rich emits `ENABLE_ALT_SCREEN` followed by `HOME`.
    /// When disabling, Rich emits `DISABLE_ALT_SCREEN`.
    pub fn set_alt_screen(&mut self, enable: bool) -> io::Result<bool> {
        if !self.is_terminal() || self.legacy_windows {
            return Ok(false);
        }
        if enable == self.is_alt_screen {
            return Ok(false);
        }

        let mut segs = Segments::new();
        if enable {
            segs.push(Segment::control(ControlType::EnableAltScreen));
            segs.push(Segment::control(ControlType::Home));
            self.is_alt_screen = true;
        } else {
            segs.push(Segment::control(ControlType::DisableAltScreen));
            self.is_alt_screen = false;
        }
        self.print_segments(&segs)?;
        Ok(true)
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
    // Live display integration (used by Live / Progress)
    // ========================================================================

    pub fn live_start(
        &mut self,
        renderable: Box<dyn crate::Renderable + Send + Sync>,
        vertical_overflow: crate::live::VerticalOverflowMethod,
    ) -> (usize, bool) {
        let is_root = self.live.stack.is_empty();
        let id = self.live.next_id;
        self.live.next_id += 1;

        self.live.entries.insert(
            id,
            LiveEntry {
                renderable,
                vertical_overflow: vertical_overflow.into(),
            },
        );
        self.live.stack.push(id);
        (id, is_root)
    }

    pub fn live_update(&mut self, id: usize, renderable: Box<dyn crate::Renderable + Send + Sync>) {
        if let Some(entry) = self.live.entries.get_mut(&id) {
            entry.renderable = renderable;
        }
    }

    pub fn live_set_vertical_overflow(
        &mut self,
        id: usize,
        vertical_overflow: crate::live::VerticalOverflowMethod,
    ) {
        if let Some(entry) = self.live.entries.get_mut(&id) {
            entry.vertical_overflow = vertical_overflow.into();
        }
    }

    pub fn live_stop(&mut self, id: usize) -> Option<Box<dyn crate::Renderable + Send + Sync>> {
        self.live.stack.retain(|&x| x != id);
        let entry = self.live.entries.remove(&id);
        if self.live.stack.is_empty() {
            self.live.shape = None;
        }
        entry.map(|e| e.renderable)
    }

    pub fn live_clear(&mut self) {
        self.live.stack.clear();
        self.live.entries.clear();
        self.live.shape = None;
    }

    fn has_live(&self) -> bool {
        !self.live.stack.is_empty()
    }

    fn live_root(&self) -> Option<&LiveEntry> {
        let id = *self.live.stack.first()?;
        self.live.entries.get(&id)
    }

    pub(crate) fn live_position_cursor(&self) -> Segments {
        let Some((_, height)) = self.live.shape else {
            return Segments::new();
        };
        if height == 0 {
            return Segments::new();
        }
        let mut controls = Vec::new();
        controls.push(Segment::control(ControlType::CarriageReturn));
        controls.push(Segment::control(ControlType::EraseInLine(2)));
        for _ in 0..height.saturating_sub(1) {
            controls.push(Segment::control(ControlType::CursorUp(1)));
            controls.push(Segment::control(ControlType::CarriageReturn));
            controls.push(Segment::control(ControlType::EraseInLine(2)));
        }
        Segments::from_iter(controls)
    }

    pub(crate) fn live_restore_cursor(&self) -> Segments {
        let Some((_, height)) = self.live.shape else {
            return Segments::new();
        };
        if height == 0 {
            return Segments::new();
        }
        let mut controls = Vec::new();
        controls.push(Segment::control(ControlType::CarriageReturn));
        for _ in 0..height {
            controls.push(Segment::control(ControlType::CursorUp(1)));
            controls.push(Segment::control(ControlType::CarriageReturn));
            controls.push(Segment::control(ControlType::EraseInLine(2)));
        }
        Segments::from_iter(controls)
    }

    fn render_live_segments(
        &mut self,
        temp_console: &Console<Stdout>,
        options: &ConsoleOptions,
    ) -> Segments {
        let root = match self.live_root() {
            Some(root) => root,
            None => return Segments::new(),
        };

        let mut lines: Vec<Vec<Segment>> = Vec::new();
        for id in self.live.stack.iter() {
            if let Some(entry) = self.live.entries.get(id) {
                let mut rendered = temp_console.render_lines(
                    entry.renderable.as_ref(),
                    Some(options),
                    None,
                    false,
                    false,
                );
                lines.append(&mut rendered);
            }
        }

        let max_height = options.size.1;
        if max_height > 0 && lines.len() > max_height {
            match root.vertical_overflow {
                LiveVerticalOverflow::Visible => {}
                LiveVerticalOverflow::Crop => {
                    lines.truncate(max_height);
                }
                LiveVerticalOverflow::Ellipsis => {
                    lines.truncate(max_height.saturating_sub(1));
                    let style = options.get_style("live.ellipsis").unwrap_or_default();
                    let ellipsis = Text::styled("...", style).center(options.max_width);
                    let ellipsis_lines =
                        temp_console.render_lines(&ellipsis, Some(options), None, false, false);
                    if let Some(first) = ellipsis_lines.into_iter().next() {
                        lines.push(first);
                    }
                }
            }
        }

        let shape = Segment::get_shape(&lines);
        self.live.shape = Some(shape);

        let mut out = Segments::new();
        let new_line = Segment::line();
        for (i, line) in lines.into_iter().enumerate() {
            for seg in line {
                out.push(seg);
            }
            if i + 1 < shape.1 {
                out.push(new_line.clone());
            }
        }
        out
    }

    // ========================================================================
    // Input Methods
    // ========================================================================

    /// Read a line of input from the user.
    ///
    /// This method displays a prompt and reads a line of input from stdin.
    /// If `password` is true, input is masked (not echoed to the terminal).
    ///
    /// # Arguments
    ///
    /// * `prompt` - The text to display as a prompt.
    /// * `password` - If true, input will be masked for password entry.
    ///
    /// # Returns
    ///
    /// The user's input as a string (without trailing newline).
    ///
    /// # Errors
    ///
    /// Returns an error if reading from stdin fails or if the input stream
    /// reaches EOF unexpectedly.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rich_rs::{Console, Text};
    ///
    /// let mut console = Console::new();
    /// let prompt = Text::plain("Enter your name: ");
    /// let name = console.input(&prompt, false)?;
    /// println!("Hello, {}!", name);
    /// ```
    pub fn input(&mut self, prompt: &Text, password: bool) -> io::Result<String> {
        use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
        use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

        // Print the prompt
        self.print(prompt, None, None, None, false, "")?;
        self.writer.flush()?;

        // For password input, use raw mode to capture without echo
        if password && self.is_terminal() && !self.is_dumb_terminal() {
            enable_raw_mode()?;

            let result = (|| -> io::Result<String> {
                let mut input = String::new();

                loop {
                    if let Event::Key(KeyEvent {
                        code, modifiers, ..
                    }) = event::read()?
                    {
                        match code {
                            KeyCode::Enter => {
                                // Print newline after password entry
                                write!(self.writer, "\r\n")?;
                                self.writer.flush()?;
                                return Ok(input);
                            }
                            KeyCode::Backspace => {
                                input.pop();
                            }
                            KeyCode::Char(c) => {
                                // Check for Ctrl+C
                                if c == 'c' && modifiers.contains(KeyModifiers::CONTROL) {
                                    write!(self.writer, "\r\n")?;
                                    self.writer.flush()?;
                                    return Err(io::Error::new(
                                        io::ErrorKind::Interrupted,
                                        "Input cancelled",
                                    ));
                                }
                                // Check for Ctrl+D (EOF)
                                if c == 'd' && modifiers.contains(KeyModifiers::CONTROL) {
                                    write!(self.writer, "\r\n")?;
                                    self.writer.flush()?;
                                    return Err(io::Error::new(
                                        io::ErrorKind::UnexpectedEof,
                                        "EOF",
                                    ));
                                }
                                input.push(c);
                            }
                            KeyCode::Esc => {
                                // ESC cancels input
                                write!(self.writer, "\r\n")?;
                                self.writer.flush()?;
                                return Err(io::Error::new(
                                    io::ErrorKind::Interrupted,
                                    "Input cancelled",
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            })();

            // Always restore terminal mode
            let _ = disable_raw_mode();
            result
        } else {
            // Normal input: read from stdin
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            // Remove trailing newline
            if input.ends_with('\n') {
                input.pop();
                if input.ends_with('\r') {
                    input.pop();
                }
            }
            Ok(input)
        }
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

    /// Update rendered lines at an offset on the alternate screen.
    ///
    /// This is the Rust equivalent of Rich's `Console.update_screen_lines`.
    pub fn update_screen_lines(
        &mut self,
        lines: &[Vec<Segment>],
        x: u16,
        y: u16,
    ) -> io::Result<()> {
        if !self.is_alt_screen() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Alt screen must be enabled to call update_screen_lines",
            ));
        }

        let mut segments = Segments::new();
        for (offset, line) in lines.iter().enumerate() {
            segments.push(Segment::control(ControlType::MoveTo {
                x,
                y: y.saturating_add(offset as u16),
            }));
            segments.extend(line.iter().cloned());
        }
        self.print_segments(&segments)?;
        Ok(())
    }
}

// ============================================================================
// Pager Context
// ============================================================================

/// Options for the pager context.
#[derive(Debug, Clone, Default)]
pub struct PagerOptions {
    /// Whether to preserve ANSI styles in pager output.
    pub styles: bool,
}

impl PagerOptions {
    /// Create new pager options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable styles in pager output.
    pub fn with_styles(mut self, styles: bool) -> Self {
        self.styles = styles;
        self
    }
}

/// A guard that captures console output and sends it to a pager on drop.
///
/// This struct is returned by `Console::pager()` and implements a context-manager
/// pattern similar to Python's `with console.pager():`.
///
/// # Example
///
/// ```no_run
/// use rich_rs::Console;
///
/// let mut console = Console::new();
/// {
///     let mut pager = console.pager(None);
///     pager.print_text("Long content...").unwrap();
///     // Content is sent to pager when `pager` is dropped
/// }
/// ```
pub struct PagerContext {
    /// Captured output buffer.
    buffer: Vec<u8>,
    /// Pager options.
    options: PagerOptions,
    /// Console options for rendering.
    console_options: ConsoleOptions,
}

impl PagerContext {
    /// Create a new pager context.
    fn new(console_options: ConsoleOptions, options: Option<PagerOptions>) -> Self {
        let options = options.unwrap_or_default();
        // If styles are disabled, turn off color system so no ANSI escapes are emitted
        let mut console_options = console_options;
        if !options.styles {
            console_options.is_terminal = false;
            console_options.color_system = None;
        }
        Self {
            buffer: Vec::new(),
            options,
            console_options,
        }
    }

    /// Print plain text.
    pub fn print_text(&mut self, text: &str) -> io::Result<()> {
        writeln!(self.buffer, "{}", text)
    }

    /// Print a renderable.
    pub fn print<R: crate::Renderable + ?Sized>(
        &mut self,
        renderable: &R,
        style: Option<Style>,
        justify: Option<JustifyMethod>,
        overflow: Option<OverflowMethod>,
        no_wrap: bool,
        end: &str,
    ) -> io::Result<()> {
        // Create a capture console to render
        let mut console = Console::with_writer(Vec::new(), self.console_options.clone());

        // Render to the buffer
        console.print(renderable, style, justify, overflow, no_wrap, end)?;

        // Append to our buffer
        self.buffer.extend_from_slice(console.get_captured_bytes());
        Ok(())
    }

    /// Get the current buffer contents.
    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Get the buffer as a string.
    pub fn get_buffer_string(&self) -> String {
        String::from_utf8_lossy(&self.buffer).to_string()
    }

    /// Manually send content to the pager.
    pub fn show(&self) -> io::Result<()> {
        use crate::pager::{Pager, SystemPager};
        let pager = SystemPager::with_styles(self.options.styles);
        let content = self.get_buffer_string();
        pager.show(&content)
    }
}

impl Drop for PagerContext {
    fn drop(&mut self) {
        if !self.buffer.is_empty() {
            let _ = self.show();
        }
    }
}

impl Console<Stdout> {
    /// Create a pager context that captures output and sends it to a pager.
    ///
    /// Similar to Python Rich's `with console.pager():` context manager.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional pager options. Use `PagerOptions::new().with_styles(true)`
    ///   to preserve ANSI escape sequences in the pager.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rich_rs::{Console, PagerOptions};
    ///
    /// let console = Console::new();
    /// {
    ///     let mut pager = console.pager(Some(PagerOptions::new().with_styles(true)));
    ///     pager.print_text("Long content that should be paged...").unwrap();
    ///     // Content is sent to pager when `pager` is dropped
    /// }
    /// ```
    pub fn pager(&self, options: Option<PagerOptions>) -> PagerContext {
        PagerContext::new(self.options.clone(), options)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StyleMeta;

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

    #[test]
    fn test_live_wrap_emits_cursor_controls_after_first_render() {
        let mut console = Console::with_writer(
            Vec::new(),
            ConsoleOptions {
                is_terminal: true,
                ..Default::default()
            },
        );
        console.set_force_terminal(Some(true));
        if console.is_dumb_terminal() {
            // Live wrapping is disabled in dumb terminals.
            return;
        }

        let (_id, _is_root) = console.live_start(
            Box::new(Text::plain("LIVE")),
            crate::live::VerticalOverflowMethod::Ellipsis,
        );

        // First render establishes the live shape but does not emit cursor positioning.
        console
            .print(&Text::plain("A"), None, None, None, false, "\n")
            .unwrap();
        console.clear_captured();

        // Second render should reposition (erase line) before drawing.
        console
            .print(&Text::plain("B"), None, None, None, false, "\n")
            .unwrap();
        let out = console.get_captured();
        assert!(out.contains("\x1b[2K"));
    }

    #[test]
    fn test_set_alt_screen_emits_enable_and_home() {
        let mut console = Console::with_writer(
            Vec::new(),
            ConsoleOptions {
                is_terminal: true,
                ..Default::default()
            },
        );
        console.set_force_terminal(Some(true));
        if console.is_dumb_terminal() {
            // Alt screen isn't meaningful on dumb terminals.
            return;
        }

        console.set_alt_screen(true).unwrap();
        let out = console.get_captured();
        assert!(out.contains("\x1b[?1049h"));
        assert!(out.contains("\x1b[H"));
    }

    #[test]
    fn test_print_segments_does_not_emit_osc8_when_not_terminal() {
        let mut console = Console::capture();
        let mut segments = Segments::new();
        segments.push(Segment::new_with_meta(
            "X",
            StyleMeta::with_link("https://example.com"),
        ));
        console.print_segments(&segments).unwrap();
        let out = console.get_captured();
        assert!(out.contains("X"));
        assert!(!out.contains("\x1b]8;"));
    }

    #[test]
    fn test_print_segments_emits_osc8_for_segment_meta_link() {
        let mut console = Console::with_writer(
            Vec::new(),
            ConsoleOptions {
                is_terminal: true,
                ..Default::default()
            },
        );
        console.set_force_terminal(Some(true));
        if console.is_dumb_terminal() {
            // OSC8 is not meaningful on dumb terminals.
            return;
        }

        let mut segments = Segments::new();
        segments.push(Segment::new_with_meta(
            "X",
            StyleMeta::with_link("https://example.com"),
        ));
        console.print_segments(&segments).unwrap();
        let out = console.get_captured();
        assert!(out.contains("\x1b]8;"));
        assert!(out.contains("https://example.com"));
        assert!(out.contains("\x1b]8;;\x1b\\"));
    }

    #[test]
    fn test_print_segments_osc8_link_id_stable_per_console() {
        let mut console = Console::with_writer(
            Vec::new(),
            ConsoleOptions {
                is_terminal: true,
                ..Default::default()
            },
        );
        console.set_force_terminal(Some(true));
        if console.is_dumb_terminal() {
            return;
        }

        let mut segments = Segments::new();
        segments.push(Segment::new_with_meta(
            "X",
            StyleMeta::with_link("https://example.com"),
        ));

        console.print_segments(&segments).unwrap();
        let out1 = console.get_captured();
        assert!(out1.contains("id=richrs-1;https://example.com"));

        console.clear_captured();
        console.print_segments(&segments).unwrap();
        let out2 = console.get_captured();
        assert!(out2.contains("id=richrs-1;https://example.com"));
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

    // ==================== Pager context tests ====================

    #[test]
    fn test_pager_options_default() {
        let opts = PagerOptions::default();
        assert!(!opts.styles);
    }

    #[test]
    fn test_pager_options_with_styles() {
        let opts = PagerOptions::new().with_styles(true);
        assert!(opts.styles);
    }

    #[test]
    fn test_pager_context_captures_text() {
        let console = Console::new();
        let mut pager = console.pager(None);

        pager.print_text("Hello").unwrap();
        pager.print_text("World").unwrap();

        let buffer = pager.get_buffer_string();
        assert!(buffer.contains("Hello"));
        assert!(buffer.contains("World"));

        // Prevent the pager from actually running during the test
        pager.buffer.clear();
    }

    #[test]
    fn test_pager_context_captures_renderable() {
        let console = Console::new();
        let mut pager = console.pager(None);

        let text = Text::plain("Rendered text");
        pager.print(&text, None, None, None, false, "\n").unwrap();

        let buffer = pager.get_buffer_string();
        assert!(buffer.contains("Rendered text"));

        // Prevent the pager from actually running during the test
        pager.buffer.clear();
    }
}
