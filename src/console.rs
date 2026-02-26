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
use std::fs::OpenOptions;
use std::io::{self, Stdout, Write};
use std::sync::{Arc, Mutex, OnceLock};

use crossterm::terminal::{self, ClearType};
use crossterm::{cursor, execute, terminal as ct};

use crate::Renderable;
use crate::cells::cell_len;
use crate::color::{ColorSystem, ColorTriplet, SimpleColor};
use crate::emoji::Emoji;
use crate::export_format::{CONSOLE_HTML_FORMAT, CONSOLE_SVG_FORMAT};
use crate::highlighter::Highlighter;
use crate::screen_buffer::ScreenBuffer;
use crate::segment::{ControlType, Segment, Segments};
use crate::style::Style;
use crate::table::{Column, Row, Table};
use crate::terminal_theme::{DEFAULT_TERMINAL_THEME, SVG_EXPORT_THEME, TerminalTheme};
use crate::text::Text;
use crate::theme::{Theme, ThemeStack};
use crate::traceback::Traceback;

use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowsRenderMode {
    Segment,
    Streaming,
}

fn parse_windows_render_mode(value: Option<&str>) -> WindowsRenderMode {
    match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Some("streaming") => WindowsRenderMode::Streaming,
        Some("segment") => WindowsRenderMode::Segment,
        _ => WindowsRenderMode::Streaming,
    }
}

fn detect_windows_render_mode() -> WindowsRenderMode {
    parse_windows_render_mode(env::var("RICH_RS_WINDOWS_RENDER_MODE").ok().as_deref())
}

fn parse_bool_env(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn detect_legacy_windows_default() -> bool {
    if let Ok(value) = env::var("RICH_RS_LEGACY_WINDOWS")
        && let Some(parsed) = parse_bool_env(&value)
    {
        return parsed;
    }
    #[cfg(windows)]
    {
        // Align with Rich Python's capability-first gating:
        // legacy mode is only required when VT is unavailable.
        return !crossterm::ansi_support::supports_ansi();
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn debug_segments_log(line: &str) {
    static PATH: OnceLock<Option<String>> = OnceLock::new();
    let path = PATH.get_or_init(|| env::var("RICH_RS_DEBUG_SEGMENTS_FILE").ok());
    let Some(path) = path.as_ref() else {
        return;
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

fn debug_ansi_log(line: &str) {
    static PATH: OnceLock<Option<String>> = OnceLock::new();
    let path = PATH.get_or_init(|| env::var("RICH_RS_DEBUG_ANSI_FILE").ok());
    let Some(path) = path.as_ref() else {
        return;
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

fn debug_segments_match_text(text: &str) -> bool {
    static FILTERS: OnceLock<Vec<String>> = OnceLock::new();
    let filters = FILTERS.get_or_init(|| {
        env::var("RICH_RS_DEBUG_SEGMENTS_FILTER")
            .ok()
            .map(|value| {
                value
                    .split(',')
                    .map(|part| part.trim().to_ascii_lowercase())
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });
    if filters.is_empty() {
        return true;
    }
    let lowered = text.to_ascii_lowercase();
    filters.iter().any(|filter| lowered.contains(filter))
}

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
    /// Whether recording is enabled.
    record: bool,
    /// Buffer for recorded segments (protected by mutex for thread safety).
    record_buffer: Arc<Mutex<Vec<Segment>>>,
    /// Render hooks stack.
    render_hooks: Vec<Box<dyn Fn(&Segments) -> Segments + Send + Sync>>,
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
    buffer: Option<ScreenBuffer>,
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
            legacy_windows: cfg!(windows) && detect_legacy_windows_default(),
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
            record: false,
            record_buffer: Arc::new(Mutex::new(Vec::new())),
            render_hooks: Vec::new(),
        }
    }

    /// Create a new console with recording enabled.
    ///
    /// When recording is enabled, all segments written via `print()` are
    /// captured in an internal buffer that can be exported as SVG/HTML.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::Console;
    ///
    /// let mut console = Console::new_with_record();
    /// console.print_text("Hello, World!").unwrap();
    /// let svg = console.export_svg("Example", None, true, None, 0.61, None);
    /// ```
    pub fn new_with_record() -> Self {
        let mut console = Self::new();
        console.record = true;
        console
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
            record: false,
            record_buffer: Arc::new(Mutex::new(Vec::new())),
            render_hooks: Vec::new(),
        }
    }

    /// Detect color system from environment variables.
    fn detect_color_system_static(is_terminal: bool) -> Option<ColorSystem> {
        // Explicit override wins.
        if let Ok(value) = env::var("RICH_RS_COLOR_SYSTEM") {
            match value.to_ascii_lowercase().as_str() {
                "none" | "off" | "0" => return None,
                "16" | "standard" => return Some(ColorSystem::Standard),
                "256" | "eightbit" | "8bit" => return Some(ColorSystem::EightBit),
                "truecolor" | "24bit" | "rgb" => return Some(ColorSystem::TrueColor),
                "auto" => {}
                _ => {}
            }
        }

        // NO_COLOR disables color unconditionally.
        if env::var("NO_COLOR").is_ok() {
            return None;
        }

        let force_color = env::var("FORCE_COLOR").is_ok();
        if !is_terminal && !force_color {
            return None;
        }

        #[cfg(windows)]
        if is_terminal && !crossterm::ansi_support::supports_ansi() {
            // Legacy Windows console path: keep colors conservative.
            return Some(ColorSystem::Standard);
        }

        if let Ok(colorterm) = env::var("COLORTERM") {
            let ct = colorterm.to_ascii_lowercase();
            if ct == "truecolor" || ct == "24bit" || ct == "yes" || ct == "true" {
                return Some(ColorSystem::TrueColor);
            }
        }

        if let Ok(term) = env::var("TERM") {
            let term_lower = term.to_ascii_lowercase();
            if term_lower.contains("truecolor")
                || term_lower.contains("24bit")
                || term_lower.contains("direct")
            {
                return Some(ColorSystem::TrueColor);
            }
            if term_lower.contains("256color") {
                return Some(ColorSystem::EightBit);
            }
            if term_lower == "dumb" || term_lower == "unknown" {
                return None;
            }
        }

        // Interactive default: modern assumption.
        if is_terminal {
            #[cfg(windows)]
            {
                return Some(ColorSystem::TrueColor);
            }
            #[cfg(not(windows))]
            {
                return Some(ColorSystem::TrueColor);
            }
        }
        if force_color {
            return Some(ColorSystem::EightBit);
        }
        None
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
            record: false,
            record_buffer: Arc::new(Mutex::new(Vec::new())),
            render_hooks: Vec::new(),
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

    /// Get the configured output encoding.
    pub fn encoding(&self) -> &str {
        &self.options.encoding
    }

    /// Set the output encoding.
    pub fn set_encoding(&mut self, encoding: impl Into<String>) {
        self.options.encoding = encoding.into();
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
        let segments = self.apply_render_hooks(segments);

        // Split and crop lines
        let width = render_options.max_width;
        Segment::split_and_crop_lines(segments, width, style, pad, new_lines)
    }

    fn apply_render_hooks(&self, mut segments: Segments) -> Segments {
        for hook in &self.render_hooks {
            segments = hook(&segments);
        }
        segments
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
        // Use the full print() path if recording or live mode is active
        if self.record || (self.is_terminal() && !self.is_dumb_terminal() && self.has_live()) {
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
        // Use the full print() path if recording or live mode is active
        if self.record || (self.is_terminal() && !self.is_dumb_terminal() && self.has_live()) {
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

    /// Print a traceback.
    ///
    /// This renders the given `Traceback` to the console with appropriate
    /// styling. It's the Rust equivalent of Python Rich's `console.print_exception()`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rich_rs::{Console, traceback::{Traceback, Trace, Stack, Frame}};
    ///
    /// let frame = Frame::new("main.rs", 42, "main");
    /// let stack = Stack::new("Error", "Something went wrong").with_frame(frame);
    /// let trace = Trace::new(vec![stack]);
    /// let tb = Traceback::new(trace);
    ///
    /// let mut console = Console::new();
    /// console.print_traceback(&tb).unwrap();
    /// ```
    pub fn print_traceback(&mut self, traceback: &Traceback) -> io::Result<()> {
        self.print(traceback, None, None, None, false, "\n")
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
        if cfg!(windows) && detect_windows_render_mode() == WindowsRenderMode::Segment {
            return self.print_segments_segment_mode(segments);
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
                debug_segments_log(&format!("[control][streaming] {:?}", control));
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
                if debug_segments_match_text(&segment.text) {
                    debug_segments_log(&format!(
                        "[segment][streaming] text={:?} style={:?} color_system={:?}",
                        segment.text, segment.style, self.color_system
                    ));
                }
                let target = StyleState::from_style(segment.style);
                let diff = current.sgr_diff(target, color_system);
                if !diff.is_empty() {
                    write!(self.writer, "\x1b[{}m", diff)?;
                    if debug_segments_match_text(&segment.text) {
                        debug_ansi_log(&format!(
                            "[ansi][streaming] text={:?} sgr=\\x1b[{}m target={:?}",
                            segment.text, diff, target
                        ));
                    }
                    used_sgr = true;
                }
                write!(self.writer, "{}", segment.text)?;
                current = target;
            } else {
                if debug_segments_match_text(&segment.text) {
                    debug_segments_log(&format!(
                        "[segment][streaming] text={:?} style={:?} color_system=None",
                        segment.text, segment.style
                    ));
                }
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
            debug_ansi_log("[ansi][streaming] tail-reset=\\x1b[0m");
        }

        self.writer.flush()
    }

    fn print_segments_segment_mode(&mut self, segments: &Segments) -> io::Result<()> {
        let hyperlinks_enabled = self.is_terminal() && !self.is_dumb_terminal();
        let mut current_link: Option<(Arc<str>, Option<Arc<str>>)> = None;
        let mut hyperlink_manual = false;

        for segment in segments.iter() {
            if let Some(control) = &segment.control {
                debug_segments_log(&format!("[control][segment] {:?}", control));
                match control {
                    ControlType::Bell => write!(self.writer, "\x07")?,
                    ControlType::CarriageReturn => write!(self.writer, "\r")?,
                    ControlType::Home => write!(self.writer, "\x1b[H")?,
                    ControlType::Clear => write!(self.writer, "\x1b[2J\x1b[H")?,
                    ControlType::ShowCursor => write!(self.writer, "\x1b[?25h")?,
                    ControlType::HideCursor => write!(self.writer, "\x1b[?25l")?,
                    ControlType::EnableAltScreen => write!(self.writer, "\x1b[?1049h")?,
                    ControlType::DisableAltScreen => write!(self.writer, "\x1b[?1049l")?,
                    ControlType::SetTitle => {}
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
                    ControlType::MoveTo { x, y } => write!(
                        self.writer,
                        "\x1b[{};{}H",
                        (*y as usize) + 1,
                        (*x as usize) + 1
                    )?,
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
                    if current_link.is_some() {
                        write!(self.writer, "\x1b]8;;\x1b\\")?;
                    }
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

            if let Some(style) = segment.style {
                if debug_segments_match_text(&segment.text) {
                    debug_segments_log(&format!(
                        "[segment][segment] text={:?} style={:?} color_system={:?}",
                        segment.text, style, self.color_system
                    ));
                }
                if let Some(color_system) = self.color_system {
                    let styled = style.render(&segment.text, color_system);
                    if debug_segments_match_text(&segment.text) {
                        let sgr = styled
                            .strip_prefix("\x1b[")
                            .and_then(|rest| rest.split_once('m').map(|(a, _)| a))
                            .unwrap_or("<none>");
                        debug_ansi_log(&format!(
                            "[ansi][segment] text={:?} sgr=\\x1b[{}m style={:?} color_system={:?}",
                            segment.text, sgr, style, self.color_system
                        ));
                    }
                    write!(self.writer, "{}", styled)?;
                } else {
                    write!(self.writer, "{}", segment.text)?;
                }
            } else {
                if debug_segments_match_text(&segment.text) {
                    debug_segments_log(&format!(
                        "[segment][segment] text={:?} style=None color_system={:?}",
                        segment.text, self.color_system
                    ));
                }
                write!(self.writer, "{}", segment.text)?;
            }
        }

        if hyperlinks_enabled && current_link.is_some() {
            write!(self.writer, "\x1b]8;;\x1b\\")?;
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
        segments = self.apply_render_hooks(segments);

        let live_active = self.is_terminal() && !self.is_dumb_terminal() && self.has_live();
        let mut end_to_write = end;
        if live_active {
            // Cursor repositioning must be based on the *previous* live shape
            // (the currently visible frame), not the newly rendered shape.
            let previous_live_shape = self.live.shape;
            // When Live is active, the trailing newline belongs to the *printed* content,
            // and the live render must be re-drawn after it (Rich behavior).
            if !end.is_empty() {
                segments.push(Segment::new(end.to_string()));
            }
            end_to_write = "";

            let (live_segments, full_redraw) = self.render_live_segments(&options);
            let mut wrapped = Segments::new();
            let cursor_controls = if full_redraw {
                self.live_position_cursor_for_shape(previous_live_shape, true)
            } else {
                self.live_position_cursor_for_shape(previous_live_shape, false)
            };
            for seg in cursor_controls.iter() {
                wrapped.push(seg.clone());
            }
            for seg in segments.into_iter() {
                wrapped.push(seg);
            }
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

        // Record segments if recording is enabled
        if self.record {
            if let Ok(mut buffer) = self.record_buffer.lock() {
                for seg in segments.iter() {
                    buffer.push(seg.clone());
                }
                if !end_to_write.is_empty() {
                    buffer.push(Segment::new(end_to_write.to_string()));
                }
            }
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

    /// Log a renderable with timestamp prefix.
    ///
    /// Similar to `print()`, but adds a timestamp prefix in `[HH:MM:SS]` format.
    /// Optionally displays the source file and line number.
    ///
    /// # Arguments
    ///
    /// * `renderable` - The object to render and log.
    /// * `file` - Optional source file name (use `file!()` macro at call site).
    /// * `line` - Optional line number (use `line!()` macro at call site).
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rich_rs::{Console, Text};
    ///
    /// let mut console = Console::new();
    /// // Using the log! macro (recommended)
    /// rich_rs::log!(console, &Text::plain("Server starting..."));
    ///
    /// // Or directly with file/line
    /// console.log(&Text::plain("Message"), Some(file!()), Some(line!())).unwrap();
    /// ```
    pub fn log<R: Renderable + ?Sized>(
        &mut self,
        renderable: &R,
        file: Option<&str>,
        line: Option<u32>,
    ) -> io::Result<()> {
        if self.quiet {
            return Ok(());
        }

        // Get current time
        let now = SystemTime::now();
        let duration = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();
        let hours = (secs / 3600) % 24;
        let minutes = (secs / 60) % 60;
        let seconds = secs % 60;

        // Create timestamp text with dim style
        let timestamp = format!("[{:02}:{:02}:{:02}]", hours, minutes, seconds);
        let time_style = self
            .theme_stack
            .get_style("log.time")
            .unwrap_or_else(|| Style::new().with_dim(true));
        let time_text = Text::styled(&timestamp, time_style);

        // Create a grid table for layout: [time] [message] [path:line]
        let mut grid = Table::grid().with_padding(0, 1).with_expand(true);

        // Time column
        grid.add_column(Column::new().style(time_style).no_wrap(true));

        // Message column (ratio=1, expands to fill)
        let message_style = self
            .theme_stack
            .get_style("log.message")
            .unwrap_or_default();
        grid.add_column(Column::new().style(message_style).ratio(1));

        // Path column (optional)
        let has_path = file.is_some();
        if has_path {
            let path_style = self
                .theme_stack
                .get_style("log.path")
                .unwrap_or_else(|| Style::new().with_dim(true));
            grid.add_column(Column::new().style(path_style).no_wrap(true));
        }

        // Build the row
        let mut cells: Vec<Box<dyn Renderable + Send + Sync>> = vec![Box::new(time_text)];

        // Wrap the renderable in a capturing approach
        // We need to render the user's content and wrap it
        let options = self.options.clone();
        let temp_console = Console::<Stdout>::with_options(options.clone());
        let segments = renderable.render(&temp_console, &options);

        // Convert segments to Text for the cell
        let mut message_text = Text::plain("");
        for seg in segments.iter() {
            if seg.control.is_none() {
                message_text.append(&*seg.text, seg.style);
            }
        }
        cells.push(Box::new(message_text));

        // Add path:line if provided
        if let Some(f) = file {
            // Extract just the filename from the path
            let filename = f.rsplit(['/', '\\']).next().unwrap_or(f);
            let path_text = if let Some(l) = line {
                Text::plain(format!("{}:{}", filename, l))
            } else {
                Text::plain(filename)
            };
            cells.push(Box::new(path_text));
        }

        grid.add_row(Row::new(cells));

        // Print the grid
        self.print(&grid, None, None, None, false, "\n")
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

    /// Enter alternate screen mode with a context guard.
    ///
    /// This returns a [`crate::ScreenContext`] that automatically leaves alternate screen
    /// mode when dropped, providing RAII semantics for full-screen applications.
    ///
    /// # Arguments
    ///
    /// * `hide_cursor` - Whether to hide the cursor while in alternate screen mode.
    /// * `style` - Optional background style for the screen.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rich_rs::{Console, Text};
    ///
    /// let mut console = Console::new();
    /// let mut screen = console.screen(true, None)?;
    /// screen.update(Text::plain("Hello!"))?;
    /// // Screen is automatically exited when `screen` is dropped
    /// ```
    pub fn screen(
        &mut self,
        hide_cursor: bool,
        style: Option<Style>,
    ) -> io::Result<crate::screen_context::ScreenContext<'_, W>> {
        crate::screen_context::ScreenContext::new(self, hide_cursor, style)
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
            self.live.buffer = None;
        }
        entry.map(|e| e.renderable)
    }

    pub fn live_clear(&mut self) {
        self.live.stack.clear();
        self.live.entries.clear();
        self.live.shape = None;
        self.live.buffer = None;
    }

    fn has_live(&self) -> bool {
        !self.live.stack.is_empty()
    }

    fn live_root(&self) -> Option<&LiveEntry> {
        let id = *self.live.stack.first()?;
        self.live.entries.get(&id)
    }

    fn live_position_cursor_for_shape(
        &self,
        shape: Option<(usize, usize)>,
        erase: bool,
    ) -> Segments {
        let Some((_, height)) = shape else {
            return Segments::new();
        };
        if height == 0 {
            return Segments::new();
        }
        let mut controls = Vec::new();
        controls.push(Segment::control(ControlType::CarriageReturn));
        if erase {
            controls.push(Segment::control(ControlType::EraseInLine(2)));
        }
        for _ in 0..height.saturating_sub(1) {
            controls.push(Segment::control(ControlType::CursorUp(1)));
            if erase {
                controls.push(Segment::control(ControlType::CarriageReturn));
                controls.push(Segment::control(ControlType::EraseInLine(2)));
            }
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

    fn render_live_segments(&mut self, options: &ConsoleOptions) -> (Segments, bool) {
        let root = match self.live_root() {
            Some(root) => root,
            None => return (Segments::new(), false),
        };

        let mut lines: Vec<Vec<Segment>> = Vec::new();
        for id in self.live.stack.iter() {
            if let Some(entry) = self.live.entries.get(id) {
                let mut rendered =
                    self.render_lines(entry.renderable.as_ref(), Some(options), None, false, false);
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
                        self.render_lines(&ellipsis, Some(options), None, false, false);
                    if let Some(first) = ellipsis_lines.into_iter().next() {
                        lines.push(first);
                    }
                }
            }
        }

        let shape = Segment::get_shape(&lines);
        self.live.shape = Some(shape);

        let width = options.max_width.max(1);
        let height = shape.1.max(1);
        let current_buffer = ScreenBuffer::from_lines(&lines, width, height, None);

        let use_diff = self.live.buffer.as_ref().is_some_and(|previous| {
            previous.width == current_buffer.width && previous.height == current_buffer.height
        });

        if use_diff {
            let previous = self.live.buffer.as_ref().expect("checked above");
            let diff = current_buffer.diff_to_segments_from_origin(previous);
            self.live.buffer = Some(current_buffer);
            return (diff, false);
        }

        self.live.buffer = Some(current_buffer);

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
        (out, true)
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

    // ========================================================================
    // New parity methods
    // ========================================================================

    /// Low-level output that bypasses the full rendering pipeline.
    ///
    /// Unlike `print()`, this won't pretty print, wrap text, or apply markup,
    /// but will optionally apply a basic style and highlighting.
    pub fn out(
        &mut self,
        text: &str,
        style: Option<Style>,
        _highlight: Option<bool>,
    ) -> io::Result<()> {
        if self.quiet {
            return Ok(());
        }
        self.print(
            &Text::plain(text),
            style,
            None,
            Some(OverflowMethod::Ignore),
            true, // no_wrap
            "\n",
        )
    }

    /// Export recorded output as plain text.
    ///
    /// Requires `record=true` to be set. If `styles` is false, ANSI codes are
    /// stripped from the output (only plain text is returned).
    pub fn export_text(&self, clear: bool, styles: bool) -> String {
        let mut buffer = self.record_buffer.lock().unwrap();
        let text = if styles {
            buffer
                .iter()
                .filter(|s| s.control.is_none())
                .map(|s| {
                    if let Some(style) = s.style {
                        if let Some(color_system) = self.color_system {
                            style.render(&s.text, color_system)
                        } else {
                            s.text.to_string()
                        }
                    } else {
                        s.text.to_string()
                    }
                })
                .collect::<String>()
        } else {
            buffer
                .iter()
                .filter(|s| s.control.is_none())
                .map(|s| s.text.to_string())
                .collect::<String>()
        };
        if clear {
            buffer.clear();
        }
        text
    }

    /// Save export_text output to a file.
    pub fn save_text(&self, path: &str, clear: bool, styles: bool) -> io::Result<()> {
        let text = self.export_text(clear, styles);
        std::fs::write(path, text)
    }

    /// Push a render hook that intercepts/transforms rendered segments before output.
    pub fn push_render_hook(&mut self, hook: Box<dyn Fn(&Segments) -> Segments + Send + Sync>) {
        self.render_hooks.push(hook);
    }

    /// Remove the last render hook.
    pub fn pop_render_hook(&mut self) {
        self.render_hooks.pop();
    }

    /// Create and return a Status spinner.
    ///
    /// This is a convenience method that creates a `Status` with the console's
    /// default settings.
    pub fn status(
        &self,
        status: &str,
        spinner: Option<&str>,
        spinner_style: Option<Style>,
        speed: Option<f64>,
        refresh_per_second: Option<f64>,
    ) -> crate::status::Status {
        crate::status::Status::with_options(
            status,
            spinner.unwrap_or("dots"),
            spinner_style,
            speed.unwrap_or(1.0),
            refresh_per_second.unwrap_or(12.5),
        )
    }

    /// Pretty-print JSON.
    ///
    /// Parse, format, highlight, and print JSON content.
    pub fn print_json(
        &mut self,
        json: &str,
        indent: usize,
        highlight: bool,
        sort_keys: bool,
    ) -> io::Result<()> {
        let json_renderable = crate::json::Json::new(json, indent, highlight, sort_keys);
        self.print(&json_renderable, None, None, None, true, "\n")
    }
}

// Implement render and render_with_options specifically for Console<Stdout>
// since the Renderable trait requires &Console<Stdout>
impl Console<Stdout> {
    /// Render a Renderable to Segments.
    pub fn render<R: Renderable + ?Sized>(&self, renderable: &R) -> Segments {
        self.apply_render_hooks(renderable.render(self, &self.options))
    }

    /// Render a Renderable with custom options.
    pub fn render_with_options<R: Renderable + ?Sized>(
        &self,
        renderable: &R,
        options: &ConsoleOptions,
    ) -> Segments {
        self.apply_render_hooks(renderable.render(self, options))
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

    // ========================================================================
    // Recording and Export Methods
    // ========================================================================

    /// Check if recording is enabled.
    pub fn is_recording(&self) -> bool {
        self.record
    }

    /// Enable or disable recording.
    ///
    /// When recording is enabled, all segments written via `print()` are
    /// captured in an internal buffer that can be exported as SVG/HTML.
    pub fn set_record(&mut self, record: bool) {
        self.record = record;
    }

    /// Clear the record buffer.
    pub fn clear_record_buffer(&mut self) {
        if let Ok(mut buffer) = self.record_buffer.lock() {
            buffer.clear();
        }
    }

    /// Get the current record buffer contents.
    ///
    /// Returns a clone of the recorded segments.
    pub fn get_record_buffer(&self) -> Vec<Segment> {
        self.record_buffer
            .lock()
            .map(|buf| buf.clone())
            .unwrap_or_default()
    }

    /// Export console contents as SVG.
    ///
    /// Generates an SVG image from the recorded console output. Requires
    /// `record=true` to have been set (via `new_with_record()` or `set_record(true)`).
    ///
    /// # Arguments
    ///
    /// * `title` - The title shown in the terminal window chrome.
    /// * `theme` - Optional terminal theme for colors. Defaults to `SVG_EXPORT_THEME`.
    /// * `clear` - Whether to clear the record buffer after exporting.
    /// * `code_format` - Optional custom SVG template. Defaults to `CONSOLE_SVG_FORMAT`.
    /// * `font_aspect_ratio` - Width/height ratio of the font. Defaults to 0.61 (Fira Code).
    /// * `unique_id` - Optional unique ID for CSS classes. Auto-generated if not provided.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::Console;
    ///
    /// let mut console = Console::new_with_record();
    /// console.print_text("Hello, World!").unwrap();
    /// let svg = console.export_svg("Example", None, true, None, 0.61, None);
    /// assert!(svg.contains("Hello"));
    /// ```
    pub fn export_svg(
        &mut self,
        title: &str,
        theme: Option<&TerminalTheme>,
        clear: bool,
        code_format: Option<&str>,
        font_aspect_ratio: f64,
        unique_id: Option<&str>,
    ) -> String {
        let theme = theme.unwrap_or(&*SVG_EXPORT_THEME);
        let code_format = code_format.unwrap_or(CONSOLE_SVG_FORMAT);

        // CSS rules cache - uses string key instead of Style (which doesn't implement Hash)
        let mut classes: HashMap<String, usize> = HashMap::new();
        let mut style_no = 1usize;

        let width = self.width();
        let char_height = 20.0;
        let char_width = char_height * font_aspect_ratio;
        let line_height = char_height * 1.22;

        let margin_top = 1.0;
        let margin_right = 1.0;
        let margin_bottom = 1.0;
        let margin_left = 1.0;

        let padding_top = 40.0;
        let padding_right = 8.0;
        let padding_bottom = 8.0;
        let padding_left = 8.0;

        let padding_width = padding_left + padding_right;
        let padding_height = padding_top + padding_bottom;
        let margin_width = margin_left + margin_right;
        let margin_height = margin_top + margin_bottom;

        let mut text_backgrounds: Vec<String> = Vec::new();
        let mut text_group: Vec<String> = Vec::new();

        // Get segments from record buffer
        let segments: Vec<Segment> = {
            let mut buffer = self.record_buffer.lock().unwrap();
            let segments: Vec<Segment> = buffer
                .iter()
                .filter(|s| s.control.is_none())
                .cloned()
                .collect();
            if clear {
                buffer.clear();
            }
            segments
        };

        // Generate unique ID if not provided
        let unique_id = unique_id.map(|s| s.to_string()).unwrap_or_else(|| {
            let content: String = segments
                .iter()
                .map(|s| format!("{:?}", s))
                .collect::<Vec<_>>()
                .join("");
            let hash = adler32(&format!("{}{}", content, title));
            format!("terminal-{}", hash)
        });

        // Split segments into lines
        let lines =
            Segment::split_and_crop_lines(Segments::from_iter(segments), width, None, false, false);

        let mut y = 0usize;
        for line in &lines {
            let mut x = 0usize;

            for segment in line {
                let style = segment.style.unwrap_or_default();
                let rules = get_svg_style_for_segment(&style, theme);

                if !classes.contains_key(&rules) {
                    classes.insert(rules.clone(), style_no);
                    style_no += 1;
                }
                let class_name = format!("r{}", classes[&rules]);

                // Check for background
                let has_background = if style.reverse.unwrap_or(false) {
                    true
                } else {
                    style.bgcolor.is_some() && !is_default_color(style.bgcolor)
                };

                let background = if style.reverse.unwrap_or(false) {
                    style
                        .color
                        .map(|c| resolve_color_for_svg(c, theme, true))
                        .unwrap_or(theme.foreground_color)
                } else {
                    style
                        .bgcolor
                        .map(|c| resolve_color_for_svg(c, theme, false))
                        .unwrap_or(theme.background_color)
                };

                let text_length = cell_len(&segment.text);

                if has_background {
                    text_backgrounds.push(make_tag(
                        "rect",
                        None,
                        &[
                            ("fill", &background.hex()),
                            ("x", &format_number(x as f64 * char_width)),
                            ("y", &format_number(y as f64 * line_height + 1.5)),
                            ("width", &format_number(char_width * text_length as f64)),
                            ("height", &format_number(line_height + 0.25)),
                            ("shape-rendering", "crispEdges"),
                        ],
                    ));
                }

                // Only add text if it's not all spaces
                if !segment.text.chars().all(|c| c == ' ') {
                    text_group.push(make_tag(
                        "text",
                        Some(&escape_text(&segment.text)),
                        &[
                            ("class", &format!("{}-{}", unique_id, class_name)),
                            ("x", &format_number(x as f64 * char_width)),
                            ("y", &format_number(y as f64 * line_height + char_height)),
                            (
                                "textLength",
                                &format_number(char_width * text_length as f64),
                            ),
                            ("clip-path", &format!("url(#{}-line-{})", unique_id, y)),
                        ],
                    ));
                }

                x += text_length;
            }
            y += 1;
        }

        // Generate clip paths for lines
        let line_offsets: Vec<f64> = (0..y)
            .map(|line_no| line_no as f64 * line_height + 1.5)
            .collect();

        let lines_svg: String = line_offsets
            .iter()
            .enumerate()
            .map(|(line_no, offset)| {
                format!(
                    r#"<clipPath id="{}-line-{}">
    {}
            </clipPath>"#,
                    unique_id,
                    line_no,
                    make_tag(
                        "rect",
                        None,
                        &[
                            ("x", "0"),
                            ("y", &format_number(*offset)),
                            ("width", &format_number(char_width * width as f64)),
                            ("height", &format_number(line_height + 0.25)),
                        ],
                    )
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Generate CSS styles
        let styles: String = classes
            .iter()
            .map(|(css, rule_no)| format!(".{}-r{} {{ {} }}", unique_id, rule_no, css))
            .collect::<Vec<_>>()
            .join("\n");

        let backgrounds = text_backgrounds.join("");
        let matrix = text_group.join("\n");

        let terminal_width = (width as f64 * char_width + padding_width).ceil();
        let terminal_height = (y as f64 + 1.0) * line_height + padding_height;

        // Generate terminal chrome
        let mut chrome = make_tag(
            "rect",
            None,
            &[
                ("fill", &theme.background_color.hex()),
                ("stroke", "rgba(255,255,255,0.35)"),
                ("stroke-width", "1"),
                ("x", &format_number(margin_left)),
                ("y", &format_number(margin_top)),
                ("width", &format_number(terminal_width)),
                ("height", &format_number(terminal_height)),
                ("rx", "8"),
            ],
        );

        // Add title if provided
        if !title.is_empty() {
            chrome.push_str(&make_tag(
                "text",
                Some(&escape_text(title)),
                &[
                    ("class", &format!("{}-title", unique_id)),
                    ("fill", &theme.foreground_color.hex()),
                    ("text-anchor", "middle"),
                    ("x", &format_number(terminal_width / 2.0)),
                    ("y", &format_number(margin_top + char_height + 6.0)),
                ],
            ));
        }

        // Add window buttons
        chrome.push_str(
            r##"
            <g transform="translate(26,22)">
            <circle cx="0" cy="0" r="7" fill="#ff5f57"/>
            <circle cx="22" cy="0" r="7" fill="#febc2e"/>
            <circle cx="44" cy="0" r="7" fill="#28c840"/>
            </g>
        "##,
        );

        // Generate final SVG
        code_format
            .replace("{unique_id}", &unique_id)
            .replace("{char_width}", &format_number(char_width))
            .replace("{char_height}", &format_number(char_height))
            .replace("{line_height}", &format_number(line_height))
            .replace(
                "{terminal_width}",
                &format_number(char_width * width as f64 - 1.0),
            )
            .replace(
                "{terminal_height}",
                &format_number((y as f64 + 1.0) * line_height - 1.0),
            )
            .replace("{width}", &format_number(terminal_width + margin_width))
            .replace("{height}", &format_number(terminal_height + margin_height))
            .replace("{terminal_x}", &format_number(margin_left + padding_left))
            .replace("{terminal_y}", &format_number(margin_top + padding_top))
            .replace("{styles}", &styles)
            .replace("{chrome}", &chrome)
            .replace("{backgrounds}", &backgrounds)
            .replace("{matrix}", &matrix)
            .replace("{lines}", &lines_svg)
    }

    /// Export console contents as HTML.
    ///
    /// Generates an HTML document from the recorded console output. Requires
    /// `record=true` to have been set (via `new_with_record()` or `set_record(true)`).
    ///
    /// This is modeled after Python Rich's `Console.export_html()`. Hyperlinks are
    /// emitted as `<a href="...">` when `StyleMeta.link` is present on segments.
    ///
    /// # Arguments
    ///
    /// * `theme` - Optional terminal theme for colors. Defaults to `DEFAULT_TERMINAL_THEME`.
    /// * `clear` - Whether to clear the record buffer after exporting.
    /// * `code_format` - Optional custom HTML template. Defaults to `CONSOLE_HTML_FORMAT`.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::Console;
    ///
    /// let mut console = Console::new_with_record();
    /// console.print_text("Hello, World!").unwrap();
    /// let html = console.export_html(None, true, None);
    /// assert!(html.contains("<!DOCTYPE html>"));
    /// assert!(html.contains("Hello, World!"));
    /// ```
    pub fn export_html(
        &mut self,
        theme: Option<&TerminalTheme>,
        clear: bool,
        code_format: Option<&str>,
    ) -> String {
        let theme = theme.unwrap_or(&*DEFAULT_TERMINAL_THEME);
        let code_format = code_format.unwrap_or(CONSOLE_HTML_FORMAT);

        // CSS rules cache - uses string key instead of Style (which doesn't implement Hash).
        let mut classes: HashMap<String, usize> = HashMap::new();
        let mut style_no = 1usize;

        // Get segments from record buffer.
        let segments: Vec<Segment> = {
            let mut buffer = self.record_buffer.lock().unwrap();
            let segments: Vec<Segment> = buffer
                .iter()
                .filter(|s| s.control.is_none())
                .cloned()
                .collect();
            if clear {
                buffer.clear();
            }
            segments
        };

        // First pass: collect CSS classes needed.
        for segment in &segments {
            let style = segment.style.unwrap_or_default();
            let rules = get_html_style_for_segment(&style, theme);
            if !rules.is_empty() && !classes.contains_key(&rules) {
                classes.insert(rules, style_no);
                style_no += 1;
            }
        }

        // Generate CSS styles.
        let stylesheet: String = classes
            .iter()
            .map(|(css, rule_no)| format!(".r{} {{ {} }}", rule_no, css))
            .collect::<Vec<_>>()
            .join("\n");

        // Render HTML-encoded console content.
        #[derive(Debug, Clone, PartialEq, Eq)]
        enum OpenTag {
            Span {
                class_no: usize,
            },
            Link {
                class_no: Option<usize>,
                href: Arc<str>,
            },
        }

        let mut code = String::new();
        let mut open: Option<OpenTag> = None;

        let close_open = |code: &mut String, open: &mut Option<OpenTag>| {
            if let Some(tag) = open.take() {
                match tag {
                    OpenTag::Span { .. } => code.push_str("</span>"),
                    OpenTag::Link { .. } => code.push_str("</a>"),
                }
            }
        };

        let open_tag = |code: &mut String, tag: &OpenTag| match tag {
            OpenTag::Span { class_no } => {
                code.push_str(&format!("<span class=\"r{}\">", class_no));
            }
            OpenTag::Link { class_no, href } => {
                let href_escaped = escape_html_attr(href);
                if let Some(class_no) = class_no {
                    code.push_str(&format!(
                        "<a class=\"r{}\" href=\"{}\">",
                        class_no, href_escaped
                    ));
                } else {
                    code.push_str(&format!("<a href=\"{}\">", href_escaped));
                }
            }
        };

        for segment in &segments {
            let text = segment.text.as_ref();
            if text.is_empty() {
                continue;
            }

            let style = segment.style.unwrap_or_default();
            let rules = get_html_style_for_segment(&style, theme);
            let class_no = if rules.is_empty() {
                None
            } else {
                Some(classes[&rules])
            };

            let href = segment.meta.as_ref().and_then(|m| m.link.as_ref()).cloned();

            let desired: Option<OpenTag> = if let Some(href) = href {
                Some(OpenTag::Link { class_no, href })
            } else if let Some(class_no) = class_no {
                Some(OpenTag::Span { class_no })
            } else {
                None
            };

            if desired != open {
                close_open(&mut code, &mut open);
                if let Some(tag) = &desired {
                    open_tag(&mut code, tag);
                }
                open = desired;
            }

            // HTML-escape text (do not replace spaces; <pre> preserves them).
            code.push_str(&escape_html_text(text));
        }

        close_open(&mut code, &mut open);

        // Generate final HTML.
        code_format
            .replace("{stylesheet}", &stylesheet)
            .replace("{foreground}", &theme.foreground_color.hex())
            .replace("{background}", &theme.background_color.hex())
            .replace("{code}", &code)
    }

    /// Save console contents to an HTML file.
    ///
    /// This is a convenience method that calls `export_html()` and writes the result to a file.
    pub fn save_html(
        &mut self,
        path: &str,
        theme: Option<&TerminalTheme>,
        clear: bool,
        code_format: Option<&str>,
    ) -> io::Result<()> {
        let html = self.export_html(theme, clear, code_format);
        std::fs::write(path, html)
    }

    /// Save console contents to an SVG file.
    ///
    /// This is a convenience method that calls `export_svg()` and writes
    /// the result to a file.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path to write to.
    /// * `title` - The title shown in the terminal window chrome.
    /// * `theme` - Optional terminal theme for colors.
    /// * `clear` - Whether to clear the record buffer after exporting.
    /// * `font_aspect_ratio` - Width/height ratio of the font. Defaults to 0.61.
    /// * `unique_id` - Optional unique ID for CSS classes.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rich_rs::Console;
    ///
    /// let mut console = Console::new_with_record();
    /// console.print_text("Hello, World!").unwrap();
    /// console.save_svg("output.svg", "Example", None, true, 0.61, None).unwrap();
    /// ```
    pub fn save_svg(
        &mut self,
        path: &str,
        title: &str,
        theme: Option<&TerminalTheme>,
        clear: bool,
        font_aspect_ratio: f64,
        unique_id: Option<&str>,
    ) -> io::Result<()> {
        let svg = self.export_svg(title, theme, clear, None, font_aspect_ratio, unique_id);
        std::fs::write(path, svg)
    }
}

// ============================================================================
// SVG Export Helper Functions
// ============================================================================

/// Get CSS style rules for a segment style.
pub(crate) fn get_svg_style_for_segment(style: &Style, theme: &TerminalTheme) -> String {
    let mut css_rules = Vec::new();

    // Get foreground color
    let fg_color = style
        .color
        .map(|c| resolve_color_for_svg(c, theme, true))
        .unwrap_or(theme.foreground_color);

    // Get background color
    let bg_color = style
        .bgcolor
        .map(|c| resolve_color_for_svg(c, theme, false))
        .unwrap_or(theme.background_color);

    // Handle reverse
    let (fg_color, bg_color) = if style.reverse.unwrap_or(false) {
        (bg_color, fg_color)
    } else {
        (fg_color, bg_color)
    };

    // Handle dim
    let fg_color = if style.dim.unwrap_or(false) {
        blend_rgb_for_svg(fg_color, bg_color, 0.4)
    } else {
        fg_color
    };

    css_rules.push(format!("fill: {}", fg_color.hex()));

    if style.bold.unwrap_or(false) {
        css_rules.push("font-weight: bold".to_string());
    }
    if style.italic.unwrap_or(false) {
        css_rules.push("font-style: italic".to_string());
    }
    if style.underline.unwrap_or(false) {
        css_rules.push("text-decoration: underline".to_string());
    }
    if style.strike.unwrap_or(false) {
        css_rules.push("text-decoration: line-through".to_string());
    }

    css_rules.join(";")
}

/// Get CSS style rules for a segment style in HTML export.
fn get_html_style_for_segment(style: &Style, theme: &TerminalTheme) -> String {
    let mut css_rules: Vec<String> = Vec::new();

    // Get foreground color
    let fg_color = style
        .color
        .map(|c| resolve_color_for_svg(c, theme, true))
        .unwrap_or(theme.foreground_color);

    // Get background color
    let bg_color = style
        .bgcolor
        .map(|c| resolve_color_for_svg(c, theme, false))
        .unwrap_or(theme.background_color);

    // Handle reverse
    let (fg_color, bg_color) = if style.reverse.unwrap_or(false) {
        (bg_color, fg_color)
    } else {
        (fg_color, bg_color)
    };

    // Handle dim (match Python Rich export_html: blend 50% towards background)
    let fg_color = if style.dim.unwrap_or(false) {
        blend_rgb_for_svg(fg_color, bg_color, 0.5)
    } else {
        fg_color
    };

    // Foreground color
    if style.color.is_some() || style.reverse.unwrap_or(false) || style.dim.unwrap_or(false) {
        css_rules.push(format!("color: {}", fg_color.hex()));
        css_rules.push(format!("text-decoration-color: {}", fg_color.hex()));
    }

    // Background color only when explicitly set (or reverse forces it)
    let has_background = if style.reverse.unwrap_or(false) {
        true
    } else {
        style.bgcolor.is_some() && !is_default_color(style.bgcolor)
    };
    if has_background {
        css_rules.push(format!("background-color: {}", bg_color.hex()));
    }

    // Attributes
    if style.bold.unwrap_or(false) {
        css_rules.push("font-weight: bold".to_string());
    }
    if style.italic.unwrap_or(false) {
        css_rules.push("font-style: italic".to_string());
    }

    let mut decorations = Vec::new();
    if style.underline.unwrap_or(false) {
        decorations.push("underline");
    }
    if style.strike.unwrap_or(false) {
        decorations.push("line-through");
    }
    if !decorations.is_empty() {
        css_rules.push(format!("text-decoration: {}", decorations.join(" ")));
    }

    css_rules.join("; ")
}

/// Resolve a SimpleColor to a ColorTriplet using the terminal theme.
pub(crate) fn resolve_color_for_svg(
    color: SimpleColor,
    theme: &TerminalTheme,
    is_foreground: bool,
) -> ColorTriplet {
    match color {
        SimpleColor::Default => {
            if is_foreground {
                theme.foreground_color
            } else {
                theme.background_color
            }
        }
        SimpleColor::Standard(index) => theme.get_ansi_color(index as usize),
        SimpleColor::EightBit(index) => {
            // For 8-bit colors, use the 256-color palette lookup
            if let Some(triplet) = crate::color::EIGHT_BIT_PALETTE.get(index as usize) {
                triplet
            } else {
                theme.foreground_color
            }
        }
        SimpleColor::Rgb { r, g, b } => ColorTriplet::new(r, g, b),
    }
}

/// Check if a color is the default color.
pub(crate) fn is_default_color(color: Option<SimpleColor>) -> bool {
    matches!(color, None | Some(SimpleColor::Default))
}

/// Blend two colors for dim effect.
pub(crate) fn blend_rgb_for_svg(
    color: ColorTriplet,
    background: ColorTriplet,
    factor: f64,
) -> ColorTriplet {
    let r = (color.red as f64 + (background.red as f64 - color.red as f64) * factor) as u8;
    let g = (color.green as f64 + (background.green as f64 - color.green as f64) * factor) as u8;
    let b = (color.blue as f64 + (background.blue as f64 - color.blue as f64) * factor) as u8;
    ColorTriplet::new(r, g, b)
}

/// Simple Adler-32 checksum for generating unique IDs.
pub(crate) fn adler32(data: &str) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    const MOD_ADLER: u32 = 65521;

    for byte in data.bytes() {
        a = (a + byte as u32) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }

    (b << 16) | a
}

/// Escape text for SVG/HTML.
pub(crate) fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace(' ', "&#160;")
}

/// Escape text content for HTML (does not replace spaces; `<pre>` preserves them).
fn escape_html_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Escape a string for use in an HTML attribute value (double-quoted).
fn escape_html_attr(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Format a number for SVG attributes (removes trailing zeros).
pub(crate) fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{:.2}", value)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// Make an SVG tag with attributes.
pub(crate) fn make_tag(name: &str, content: Option<&str>, attribs: &[(&str, &str)]) -> String {
    let attribs_str: String = attribs
        .iter()
        .map(|(k, v)| format!("{}=\"{}\"", k, v))
        .collect::<Vec<_>>()
        .join(" ");

    if let Some(content) = content {
        format!("<{} {}>{}</{}>", name, attribs_str, content, name)
    } else {
        format!("<{} {}/>", name, attribs_str)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Control;
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
        assert_eq!(options.encoding, "utf-8");
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
    fn test_render_hook_runs_in_print_pipeline() {
        let mut console = Console::capture();
        console.push_render_hook(Box::new(|segments: &Segments| {
            Segments::from_iter(segments.iter().map(|seg| {
                if seg.control.is_some() {
                    seg.clone()
                } else {
                    Segment::new(seg.text.to_string().to_uppercase())
                }
            }))
        }));

        console
            .print(&Text::plain("hooked"), None, None, None, false, "\n")
            .unwrap();
        let output = console.get_captured();
        assert!(output.contains("HOOKED"));
    }

    #[test]
    fn test_render_hook_runs_for_live_renderables() {
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
        console.push_render_hook(Box::new(|segments: &Segments| {
            let mut out = Segments::new();
            for seg in segments.iter() {
                out.push(seg.clone());
            }
            out.push(Segment::new("!"));
            out
        }));

        let (_id, _is_root) = console.live_start(
            Box::new(Text::plain("LIVE")),
            crate::live::VerticalOverflowMethod::Ellipsis,
        );
        console
            .print(&Control::new(), None, None, None, false, "")
            .unwrap();

        let output = console.get_captured();
        assert!(
            output.contains("LIVE!"),
            "expected hooked live output in captured text, got: {:?}",
            output
        );
    }

    #[test]
    fn test_console_status_honors_refresh_per_second() {
        let console = Console::capture();
        let status = console.status("Working...", None, None, None, Some(9.0));
        assert_eq!(status.refresh_per_second(), 9.0);
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

        // Second render takes the screen-buffer diff path (same shape), so it
        // repositions the cursor without a full erase.  Verify that cursor
        // repositioning is emitted (\r for carriage return).
        console
            .print(&Text::plain("B"), None, None, None, false, "\n")
            .unwrap();
        let out = console.get_captured();
        assert!(
            out.contains("\r"),
            "expected cursor repositioning (\\r) in second live render, got: {:?}",
            out,
        );
    }

    #[test]
    fn test_live_full_redraw_repositions_from_previous_shape() {
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

        let (id, _is_root) = console.live_start(
            Box::new(Text::plain("one line")),
            crate::live::VerticalOverflowMethod::Ellipsis,
        );

        // Establish an initial 1-line live frame.
        console
            .print(&Control::new(), None, None, None, false, "")
            .unwrap();
        console.clear_captured();

        // Grow the live render to 2 lines. Full redraw should position cursor
        // using the previous (1-line) frame, so no cursor-up should be emitted.
        console.live_update(id, Box::new(Text::plain("line 1\nline 2")));
        console
            .print(&Control::new(), None, None, None, false, "")
            .unwrap();

        let out = console.get_captured();
        assert!(
            !out.contains("\x1b[1A"),
            "did not expect cursor-up for previous 1-line frame, got: {:?}",
            out,
        );
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

    #[test]
    fn test_print_text_from_ansi_emits_osc8_lifecycle_from_style_meta() {
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

        let text = Text::from_ansi("\x1b]8;id=src42;https://example.com\x07Link\x1b]8;;\x07 done");
        console.print(&text, None, None, None, false, "").unwrap();
        let out = console.get_captured();

        let open = "\x1b]8;id=src42;https://example.com\x1b\\";
        let close = "\x1b]8;;\x1b\\";
        assert!(out.contains(open));
        assert!(out.contains(close));

        let open_pos = out.find(open).unwrap();
        let link_text_pos = out.find("Link").unwrap();
        let close_pos = out.find(close).unwrap();
        let plain_pos = out.rfind(" done").unwrap();
        assert!(open_pos < link_text_pos);
        assert!(link_text_pos < close_pos);
        assert!(close_pos < plain_pos);
    }

    #[test]
    fn test_print_segments_closes_hyperlink_before_tail_reset() {
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
        segments.push(Segment::styled_with_meta(
            "X",
            Style::new().with_bold(true),
            StyleMeta::with_link("https://example.com"),
        ));

        console.print_segments(&segments).unwrap();
        let out = console.get_captured();
        let close_pos = out.find("\x1b]8;;\x1b\\").unwrap();
        let reset_pos = out.rfind("\x1b[0m").unwrap();
        assert!(close_pos < reset_pos);
    }

    #[test]
    fn test_parse_windows_render_mode_defaults_to_streaming() {
        assert_eq!(
            parse_windows_render_mode(None),
            WindowsRenderMode::Streaming
        );
        assert_eq!(
            parse_windows_render_mode(Some("invalid")),
            WindowsRenderMode::Streaming
        );
    }

    #[test]
    fn test_parse_windows_render_mode_values() {
        assert_eq!(
            parse_windows_render_mode(Some("segment")),
            WindowsRenderMode::Segment
        );
        assert_eq!(
            parse_windows_render_mode(Some("streaming")),
            WindowsRenderMode::Streaming
        );
        assert_eq!(
            parse_windows_render_mode(Some("  StReAmInG  ")),
            WindowsRenderMode::Streaming
        );
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

    #[test]
    fn test_console_encoding() {
        let mut console = Console::capture();
        assert_eq!(console.encoding(), "utf-8");

        console.set_encoding("latin-1");
        assert_eq!(console.encoding(), "latin-1");
        assert_eq!(console.options().encoding, "latin-1");
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

        // Test set_encoding
        console.set_encoding("cp1252");
        assert_eq!(console.encoding(), "cp1252");
        assert_eq!(console.options().encoding, "cp1252");

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
        options.encoding = "ascii".to_string();
        options.color_system = Some(ColorSystem::Standard);

        // Create console from options
        let console = Console::with_options(options);

        // Console fields should match options
        assert!(!console.is_markup_enabled());
        assert!(!console.is_emoji_enabled());
        assert_eq!(console.tab_size(), 4);
        assert_eq!(console.encoding(), "ascii");
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

    // ==================== Recording and SVG export tests ====================

    #[test]
    fn test_console_new_with_record() {
        let console = Console::new_with_record();
        assert!(console.is_recording());
    }

    #[test]
    fn test_console_set_record() {
        let mut console = Console::new();
        assert!(!console.is_recording());

        console.set_record(true);
        assert!(console.is_recording());

        console.set_record(false);
        assert!(!console.is_recording());
    }

    #[test]
    fn test_console_record_buffer() {
        let mut console = Console::new_with_record();
        console.options_mut().is_terminal = false;
        console.options_mut().max_width = 80;

        // Print something
        console.print_text("Hello").unwrap();

        // Check that something was recorded
        let buffer = console.get_record_buffer();
        assert!(!buffer.is_empty());

        // Find the segment containing "Hello"
        let has_hello = buffer.iter().any(|s| s.text.contains("Hello"));
        assert!(has_hello, "Record buffer should contain 'Hello'");

        // Clear the buffer
        console.clear_record_buffer();
        let buffer = console.get_record_buffer();
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_export_svg_basic() {
        let mut console = Console::new_with_record();
        console.options_mut().is_terminal = false;
        console.options_mut().max_width = 40;

        console.print_text("Hello, World!").unwrap();

        let svg = console.export_svg("Test", None, true, None, 0.61, None);

        // Check SVG structure
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Hello"));
        assert!(svg.contains("rich-terminal"));

        // Buffer should be cleared
        let buffer = console.get_record_buffer();
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_export_svg_with_custom_title() {
        let mut console = Console::new_with_record();
        console.options_mut().is_terminal = false;
        console.options_mut().max_width = 40;

        console.print_text("Test").unwrap();

        let svg = console.export_svg("My Custom Title", None, true, None, 0.61, None);

        assert!(svg.contains("My&#160;Custom&#160;Title"));
    }

    #[test]
    fn test_export_svg_with_unique_id() {
        let mut console = Console::new_with_record();
        console.options_mut().is_terminal = false;
        console.options_mut().max_width = 40;

        console.print_text("Test").unwrap();

        let svg = console.export_svg("Test", None, true, None, 0.61, Some("my-unique-id"));

        assert!(svg.contains("my-unique-id"));
    }

    #[test]
    fn test_export_svg_escape_text() {
        let mut console = Console::new_with_record();
        console.options_mut().is_terminal = false;
        console.options_mut().max_width = 80;

        console.print_text("<script>alert('XSS')</script>").unwrap();

        let svg = console.export_svg("Test", None, true, None, 0.61, None);

        // Should be escaped
        assert!(svg.contains("&lt;"));
        assert!(svg.contains("&gt;"));
        assert!(!svg.contains("<script>"));
    }

    #[test]
    fn test_export_svg_no_clear() {
        let mut console = Console::new_with_record();
        console.options_mut().is_terminal = false;
        console.options_mut().max_width = 40;

        console.print_text("Hello").unwrap();

        // Export without clearing
        let _svg = console.export_svg("Test", None, false, None, 0.61, None);

        // Buffer should still contain data
        let buffer = console.get_record_buffer();
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_export_html_basic() {
        let mut console = Console::new_with_record();
        console.options_mut().is_terminal = false;
        console.options_mut().max_width = 40;

        console.print_text("Hello, World!").unwrap();

        let html = console.export_html(None, true, None);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Hello, World!"));

        // Buffer should be cleared
        let buffer = console.get_record_buffer();
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_export_html_link_emits_anchor_tag() {
        let mut console = Console::new_with_record();
        console.options_mut().is_terminal = false;

        let text =
            Text::from_markup("[link=https://textualize.io]Textualize.io[/link]", false).unwrap();
        console.print(&text, None, None, None, false, "\n").unwrap();

        let html = console.export_html(None, true, None);
        assert!(html.contains("<a"));
        assert!(html.contains("href=\"https://textualize.io\""));
        assert!(html.contains("Textualize.io"));
    }

    #[test]
    fn test_export_html_escapes_text() {
        let mut console = Console::new_with_record();
        console.options_mut().is_terminal = false;

        console.print_text("<script>alert('XSS')</script>").unwrap();
        let html = console.export_html(None, true, None);
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
    }
}
