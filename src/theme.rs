//! Theme system for named styles.
//!
//! Themes provide a mapping from style names (like "repr.number" or "markdown.h1")
//! to `Style` objects. They can be stacked to create scoped style overrides.
//!
//! # Example
//!
//! ```
//! use rich_rs::{Theme, Style};
//!
//! let mut theme = Theme::new();
//! theme.add_style("error", Style::parse("bold red").unwrap());
//! assert!(theme.get_style("error").is_some());
//! ```
//!
//! # Named Themes
//!
//! Several themes are embedded and can be loaded by name:
//!
//! ```
//! use rich_rs::Theme;
//!
//! let theme = Theme::from_name("dracula").unwrap();
//! assert!(theme.get_style("repr.number").is_some());
//! ```

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Cursor};
use std::path::Path;

use once_cell::sync::Lazy;

use crate::color::SimpleColor as Color;
use crate::style::Style;

// ============================================================================
// Embedded Themes (generated from Pygments)
// ============================================================================

/// Dracula theme data (dark theme with purple accents)
const DRACULA_THEME_DATA: &str = include_str!("themes/dracula.theme");

/// Gruvbox Dark theme data (retro groove dark theme)
const GRUVBOX_DARK_THEME_DATA: &str = include_str!("themes/gruvbox-dark.theme");

/// Nord theme data (arctic, north-bluish color palette)
const NORD_THEME_DATA: &str = include_str!("themes/nord.theme");

/// Errors that can occur when working with themes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeError {
    /// Unable to pop the base theme from the stack.
    PopBaseTheme,
    /// IO error when reading theme file.
    IoError(String),
    /// Invalid theme file format.
    InvalidFormat(String),
}

impl std::fmt::Display for ThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeError::PopBaseTheme => write!(f, "Unable to pop base theme"),
            ThemeError::IoError(msg) => write!(f, "IO error: {}", msg),
            ThemeError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
        }
    }
}

impl std::error::Error for ThemeError {}

impl From<io::Error> for ThemeError {
    fn from(err: io::Error) -> Self {
        ThemeError::IoError(err.to_string())
    }
}

/// A container for style information.
///
/// Themes map style names (like "repr.number") to `Style` objects.
/// They can optionally inherit from the default styles.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Named styles in this theme.
    styles: HashMap<String, Style>,
    /// Whether this theme inherits default styles.
    inherit: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}

impl Theme {
    /// Create an empty theme that inherits default styles.
    pub fn new() -> Self {
        Theme {
            styles: default_styles(),
            inherit: true,
        }
    }

    /// Create an empty theme without default styles.
    pub fn empty() -> Self {
        Theme {
            styles: HashMap::new(),
            inherit: false,
        }
    }

    /// Create a theme with custom styles, optionally inheriting defaults.
    pub fn with_styles(styles: HashMap<String, Style>, inherit: bool) -> Self {
        let mut theme_styles = if inherit {
            default_styles()
        } else {
            HashMap::new()
        };
        theme_styles.extend(styles);
        Theme {
            styles: theme_styles,
            inherit,
        }
    }

    /// Read a theme from an INI-like config file.
    ///
    /// # Format
    ///
    /// ```ini
    /// [styles]
    /// repr.number = bold cyan
    /// repr.string = green
    /// ```
    pub fn read<P: AsRef<Path>>(path: P, inherit: bool) -> Result<Self, ThemeError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::from_reader(reader, inherit)
    }

    /// Read a theme from a file (convenience wrapper for `read`).
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ThemeError> {
        Self::read(path, true)
    }

    /// Parse a theme from a reader.
    pub fn from_reader<R: BufRead>(reader: R, inherit: bool) -> Result<Self, ThemeError> {
        let mut styles = HashMap::new();
        let mut in_styles_section = false;

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            // Check for section header
            if line.starts_with('[') && line.ends_with(']') {
                let section = &line[1..line.len() - 1].trim().to_lowercase();
                in_styles_section = section == "styles";
                continue;
            }

            // Parse key = value in styles section
            if in_styles_section && let Some((name, style_str)) = line.split_once('=') {
                let name = name.trim().to_string();
                let style_str = style_str.trim();
                if let Some(style) = Style::parse(style_str) {
                    styles.insert(name, style);
                }
            }
        }

        Ok(Self::with_styles(styles, inherit))
    }

    /// Get a style by name.
    pub fn get_style(&self, name: &str) -> Option<Style> {
        self.styles.get(name).copied()
    }

    /// Add or update a style.
    pub fn add_style(&mut self, name: impl Into<String>, style: Style) {
        self.styles.insert(name.into(), style);
    }

    /// Remove a style by name.
    pub fn remove_style(&mut self, name: &str) -> Option<Style> {
        self.styles.remove(name)
    }

    /// Check if a style exists.
    pub fn has_style(&self, name: &str) -> bool {
        self.styles.contains_key(name)
    }

    /// Get all style names.
    pub fn style_names(&self) -> impl Iterator<Item = &str> {
        self.styles.keys().map(String::as_str)
    }

    /// Get the number of styles.
    pub fn len(&self) -> usize {
        self.styles.len()
    }

    /// Check if the theme is empty.
    pub fn is_empty(&self) -> bool {
        self.styles.is_empty()
    }

    /// Whether this theme inherits default styles.
    pub fn inherits(&self) -> bool {
        self.inherit
    }

    /// Generate INI config file contents for this theme.
    pub fn to_config(&self) -> String {
        let mut lines = vec!["[styles]".to_string()];
        let mut names: Vec<_> = self.styles.keys().collect();
        names.sort();

        for name in names {
            if let Some(style) = self.styles.get(name) {
                lines.push(format!("{} = {}", name, style_to_string(style)));
            }
        }

        lines.join("\n")
    }

    /// Load a theme by name.
    ///
    /// Available themes:
    /// - `"default"` - The default rich-rs theme
    /// - `"dracula"` - Dark theme with purple accents
    /// - `"gruvbox-dark"` - Retro groove dark theme
    /// - `"nord"` - Arctic, north-bluish color palette
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::Theme;
    ///
    /// let theme = Theme::from_name("dracula").unwrap();
    /// assert!(theme.get_style("repr.number").is_some());
    /// ```
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "default" => Some(Self::new()),
            "dracula" => {
                let reader = Cursor::new(DRACULA_THEME_DATA);
                Self::from_reader(reader, true).ok()
            }
            "gruvbox-dark" | "gruvbox" => {
                let reader = Cursor::new(GRUVBOX_DARK_THEME_DATA);
                Self::from_reader(reader, true).ok()
            }
            "nord" => {
                let reader = Cursor::new(NORD_THEME_DATA);
                Self::from_reader(reader, true).ok()
            }
            _ => None,
        }
    }

    /// List available theme names.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::Theme;
    ///
    /// let themes = Theme::available_themes();
    /// assert!(themes.contains(&"dracula"));
    /// ```
    pub fn available_themes() -> Vec<&'static str> {
        vec!["default", "dracula", "gruvbox-dark", "nord"]
    }
}

/// A stack of themes for scoped style overrides.
///
/// The stack allows pushing themes that temporarily override styles,
/// then popping them to restore previous styles. Style lookups search
/// from top to bottom.
#[derive(Debug, Clone)]
pub struct ThemeStack {
    /// Merged style dictionaries at each level.
    entries: Vec<HashMap<String, Style>>,
}

impl ThemeStack {
    /// Create a new theme stack with a base theme.
    pub fn new(theme: Theme) -> Self {
        ThemeStack {
            entries: vec![theme.styles],
        }
    }

    /// Push a theme onto the stack.
    ///
    /// If `inherit` is true, styles from the current top are merged
    /// with the new theme's styles.
    pub fn push(&mut self, theme: Theme, inherit: bool) {
        let styles = if inherit && !self.entries.is_empty() {
            let mut merged = self.entries.last().unwrap().clone();
            merged.extend(theme.styles);
            merged
        } else {
            theme.styles
        };
        self.entries.push(styles);
    }

    /// Alias for `push` with inherit=true.
    pub fn push_theme(&mut self, theme: Theme) {
        self.push(theme, true);
    }

    /// Pop the top theme from the stack.
    ///
    /// Returns an error if attempting to pop the base theme.
    pub fn pop(&mut self) -> Result<(), ThemeError> {
        if self.entries.len() == 1 {
            return Err(ThemeError::PopBaseTheme);
        }
        self.entries.pop();
        Ok(())
    }

    /// Alias for `pop`.
    pub fn pop_theme(&mut self) -> Result<(), ThemeError> {
        self.pop()
    }

    /// Get a style by name from the top of the stack.
    pub fn get_style(&self, name: &str) -> Option<Style> {
        self.entries
            .last()
            .and_then(|styles| styles.get(name).copied())
    }

    /// Get the number of themes on the stack.
    pub fn depth(&self) -> usize {
        self.entries.len()
    }
}

impl Default for ThemeStack {
    fn default() -> Self {
        Self::new(Theme::new())
    }
}

// ============================================================================
// Default Styles
// ============================================================================

/// Convert a Style to a string representation (for config file output).
fn style_to_string(style: &Style) -> String {
    let mut parts = Vec::new();

    // Attributes
    if style.bold == Some(true) {
        parts.push("bold".to_string());
    } else if style.bold == Some(false) {
        parts.push("not bold".to_string());
    }

    if style.dim == Some(true) {
        parts.push("dim".to_string());
    } else if style.dim == Some(false) {
        parts.push("not dim".to_string());
    }

    if style.italic == Some(true) {
        parts.push("italic".to_string());
    } else if style.italic == Some(false) {
        parts.push("not italic".to_string());
    }

    if style.underline == Some(true) {
        parts.push("underline".to_string());
    } else if style.underline == Some(false) {
        parts.push("not underline".to_string());
    }

    if style.blink == Some(true) {
        parts.push("blink".to_string());
    } else if style.blink == Some(false) {
        parts.push("not blink".to_string());
    }

    if style.reverse == Some(true) {
        parts.push("reverse".to_string());
    } else if style.reverse == Some(false) {
        parts.push("not reverse".to_string());
    }

    if style.strike == Some(true) {
        parts.push("strike".to_string());
    } else if style.strike == Some(false) {
        parts.push("not strike".to_string());
    }

    // Foreground color
    if let Some(color) = style.color {
        parts.push(color_to_string(color));
    }

    // Background color
    if let Some(bgcolor) = style.bgcolor {
        parts.push(format!("on {}", color_to_string(bgcolor)));
    }

    if parts.is_empty() {
        "none".to_string()
    } else {
        parts.join(" ")
    }
}

/// Convert a Color to a string representation.
fn color_to_string(color: Color) -> String {
    match color {
        Color::Default => "default".to_string(),
        Color::Standard(n) => {
            // Map standard colors to names
            match n {
                0 => "black".to_string(),
                1 => "red".to_string(),
                2 => "green".to_string(),
                3 => "yellow".to_string(),
                4 => "blue".to_string(),
                5 => "magenta".to_string(),
                6 => "cyan".to_string(),
                7 => "white".to_string(),
                8 => "bright_black".to_string(),
                9 => "bright_red".to_string(),
                10 => "bright_green".to_string(),
                11 => "bright_yellow".to_string(),
                12 => "bright_blue".to_string(),
                13 => "bright_magenta".to_string(),
                14 => "bright_cyan".to_string(),
                15 => "bright_white".to_string(),
                _ => format!("color({})", n),
            }
        }
        Color::EightBit(n) => format!("color({})", n),
        Color::Rgb { r, g, b } => format!("rgb({},{},{})", r, g, b),
    }
}

/// Create the default styles map.
///
/// These correspond to Python Rich's DEFAULT_STYLES from default_styles.py.
pub fn default_styles() -> HashMap<String, Style> {
    let mut styles = HashMap::new();

    // Helper to insert a style
    macro_rules! add {
        ($name:expr, $style:expr) => {
            styles.insert($name.to_string(), $style);
        };
    }

    // Basic styles
    add!("none", Style::new());
    add!(
        "reset",
        Style {
            color: Some(Color::Default),
            bgcolor: Some(Color::Default),
            bold: Some(false),
            dim: Some(false),
            italic: Some(false),
            underline: Some(false),
            blink: Some(false),
            reverse: Some(false),
            strike: Some(false),
        }
    );
    add!("dim", Style::new().with_dim(true));
    add!(
        "bright",
        Style {
            dim: Some(false),
            ..Default::default()
        }
    );
    add!("bold", Style::new().with_bold(true));
    add!("strong", Style::new().with_bold(true));
    add!(
        "code",
        Style {
            reverse: Some(true),
            bold: Some(true),
            ..Default::default()
        }
    );
    add!("italic", Style::new().with_italic(true));
    add!("emphasize", Style::new().with_italic(true));
    add!("underline", Style::new().with_underline(true));
    add!(
        "blink",
        Style {
            blink: Some(true),
            ..Default::default()
        }
    );
    add!(
        "reverse",
        Style {
            reverse: Some(true),
            ..Default::default()
        }
    );
    add!("strike", Style::new().with_strike(true));

    // Color styles
    add!("black", Style::color(Color::Standard(0)));
    add!("red", Style::color(Color::Standard(1)));
    add!("green", Style::color(Color::Standard(2)));
    add!("yellow", Style::color(Color::Standard(3)));
    add!("blue", Style::color(Color::Standard(4)));
    add!("magenta", Style::color(Color::Standard(5)));
    add!("cyan", Style::color(Color::Standard(6)));
    add!("white", Style::color(Color::Standard(7)));

    // Inspect styles
    add!(
        "inspect.attr",
        Style::color(Color::Standard(3)).with_italic(true)
    );
    add!(
        "inspect.attr.dunder",
        Style::color(Color::Standard(3))
            .with_italic(true)
            .with_dim(true)
    );
    add!(
        "inspect.callable",
        Style::color(Color::Standard(1)).with_bold(true)
    );
    add!(
        "inspect.async_def",
        Style::color(Color::Standard(14)).with_italic(true)
    );
    add!(
        "inspect.def",
        Style::color(Color::Standard(14)).with_italic(true)
    );
    add!(
        "inspect.class",
        Style::color(Color::Standard(14)).with_italic(true)
    );
    add!(
        "inspect.error",
        Style::color(Color::Standard(1)).with_bold(true)
    );
    add!("inspect.equals", Style::new());
    add!("inspect.help", Style::color(Color::Standard(6)));
    add!("inspect.doc", Style::new().with_dim(true));
    add!("inspect.value.border", Style::color(Color::Standard(2)));

    // Live styles
    add!(
        "live.ellipsis",
        Style::color(Color::Standard(1)).with_bold(true)
    );

    // Layout styles
    add!(
        "layout.tree.row",
        Style::color(Color::Standard(1)).with_dim(false)
    );
    add!(
        "layout.tree.column",
        Style::color(Color::Standard(4)).with_dim(false)
    );

    // Logging styles
    add!(
        "logging.keyword",
        Style::color(Color::Standard(3)).with_bold(true)
    );
    add!("logging.level.notset", Style::new().with_dim(true));
    add!("logging.level.debug", Style::color(Color::Standard(2)));
    add!("logging.level.info", Style::color(Color::Standard(4)));
    add!("logging.level.warning", Style::color(Color::Standard(3)));
    add!(
        "logging.level.error",
        Style::color(Color::Standard(1)).with_bold(true)
    );
    add!(
        "logging.level.critical",
        Style {
            color: Some(Color::Standard(1)),
            bold: Some(true),
            reverse: Some(true),
            ..Default::default()
        }
    );

    // Log styles
    add!("log.level", Style::new());
    add!("log.time", Style::color(Color::Standard(6)).with_dim(true));
    add!("log.message", Style::new());
    add!("log.path", Style::new().with_dim(true));

    // Repr styles
    add!("repr.ellipsis", Style::color(Color::Standard(3)));
    add!(
        "repr.indent",
        Style::color(Color::Standard(2)).with_dim(true)
    );
    add!(
        "repr.error",
        Style::color(Color::Standard(1)).with_bold(true)
    );
    add!(
        "repr.str",
        Style::color(Color::Standard(2))
            .with_italic(false)
            .with_bold(false)
    );
    add!("repr.brace", Style::new().with_bold(true));
    add!("repr.comma", Style::new().with_bold(true));
    add!(
        "repr.ipv4",
        Style::color(Color::Standard(10)).with_bold(true)
    );
    add!(
        "repr.ipv6",
        Style::color(Color::Standard(10)).with_bold(true)
    );
    add!(
        "repr.eui48",
        Style::color(Color::Standard(10)).with_bold(true)
    );
    add!(
        "repr.eui64",
        Style::color(Color::Standard(10)).with_bold(true)
    );
    add!("repr.tag_start", Style::new().with_bold(true));
    add!(
        "repr.tag_name",
        Style::color(Color::Standard(13)).with_bold(true)
    );
    add!("repr.tag_contents", Style::color(Color::Default));
    add!("repr.tag_end", Style::new().with_bold(true));
    add!(
        "repr.attrib_name",
        Style::color(Color::Standard(3)).with_italic(false)
    );
    add!("repr.attrib_equal", Style::new().with_bold(true));
    add!(
        "repr.attrib_value",
        Style::color(Color::Standard(5)).with_italic(false)
    );
    add!(
        "repr.number",
        Style::color(Color::Standard(6))
            .with_bold(true)
            .with_italic(false)
    );
    add!(
        "repr.number_complex",
        Style::color(Color::Standard(6))
            .with_bold(true)
            .with_italic(false)
    );
    add!(
        "repr.bool_true",
        Style::color(Color::Standard(10)).with_italic(true)
    );
    add!(
        "repr.bool_false",
        Style::color(Color::Standard(9)).with_italic(true)
    );
    add!(
        "repr.none",
        Style::color(Color::Standard(5)).with_italic(true)
    );
    add!(
        "repr.url",
        Style::color(Color::Standard(12))
            .with_underline(true)
            .with_italic(false)
            .with_bold(false)
    );
    add!(
        "repr.uuid",
        Style::color(Color::Standard(11)).with_bold(false)
    );
    add!(
        "repr.call",
        Style::color(Color::Standard(5)).with_bold(true)
    );
    add!("repr.path", Style::color(Color::Standard(5)));
    add!("repr.filename", Style::color(Color::Standard(13)));

    // Rule styles
    add!("rule.line", Style::color(Color::Standard(10)));
    add!("rule.text", Style::new());

    // JSON styles
    add!("json.brace", Style::new().with_bold(true));
    add!(
        "json.bool_true",
        Style::color(Color::Standard(10)).with_italic(true)
    );
    add!(
        "json.bool_false",
        Style::color(Color::Standard(9)).with_italic(true)
    );
    add!(
        "json.null",
        Style::color(Color::Standard(5)).with_italic(true)
    );
    add!(
        "json.number",
        Style::color(Color::Standard(6))
            .with_bold(true)
            .with_italic(false)
    );
    add!(
        "json.str",
        Style::color(Color::Standard(2))
            .with_italic(false)
            .with_bold(false)
    );
    add!("json.key", Style::color(Color::Standard(4)).with_bold(true));

    // Prompt styles
    add!("prompt", Style::new());
    add!(
        "prompt.choices",
        Style::color(Color::Standard(5)).with_bold(true)
    );
    add!(
        "prompt.default",
        Style::color(Color::Standard(6)).with_bold(true)
    );
    add!("prompt.invalid", Style::color(Color::Standard(1)));
    add!("prompt.invalid.choice", Style::color(Color::Standard(1)));

    // Pretty styles
    add!("pretty", Style::new());

    // Scope styles (for local variable display in tracebacks)
    add!("scope.border", Style::color(Color::Standard(4)).with_dim(true));
    add!(
        "scope.key",
        Style::color(Color::Standard(6)).with_bold(true)
    );
    add!(
        "scope.key.special",
        Style::color(Color::Standard(3))
            .with_italic(true)
            .with_dim(true)
    );
    add!("scope.equals", Style::new());

    // Table styles
    add!("table.header", Style::new().with_bold(true));
    add!("table.footer", Style::new().with_bold(true));
    add!("table.cell", Style::new());
    add!("table.title", Style::new().with_italic(true));
    add!(
        "table.caption",
        Style::new().with_italic(true).with_dim(true)
    );

    // Traceback styles
    add!("traceback.border", Style::color(Color::Standard(1)));
    add!(
        "traceback.border.syntax_error",
        Style::color(Color::Standard(9))
    );
    add!(
        "traceback.title",
        Style::color(Color::Standard(1)).with_bold(true)
    );
    add!("traceback.text", Style::color(Color::Standard(1)));
    add!(
        "traceback.exc_type",
        Style::color(Color::Standard(9)).with_bold(true)
    );
    add!("traceback.exc_value", Style::new());
    add!(
        "traceback.error",
        Style::color(Color::Standard(1)).with_bold(true)
    );
    add!(
        "traceback.error_range",
        Style::color(Color::Standard(1))
            .with_bold(true)
            .with_underline(true)
    );
    add!("traceback.path", Style::color(Color::Standard(8)));
    add!("traceback.filename", Style::color(Color::Standard(13)));
    add!("traceback.lineno", Style::color(Color::Standard(13)));
    add!(
        "traceback.offset",
        Style::color(Color::Standard(9)).with_bold(true)
    );
    add!(
        "traceback.note",
        Style::color(Color::Standard(2)).with_bold(true)
    );
    add!("traceback.group.border", Style::color(Color::Standard(5)));

    // Bar/progress styles
    add!("bar.back", Style::color(Color::EightBit(59))); // grey23
    add!(
        "bar.complete",
        Style::color(Color::Rgb {
            r: 249,
            g: 38,
            b: 114
        })
    );
    add!(
        "bar.finished",
        Style::color(Color::Rgb {
            r: 114,
            g: 156,
            b: 31
        })
    );
    add!(
        "bar.pulse",
        Style::color(Color::Rgb {
            r: 249,
            g: 38,
            b: 114
        })
    );

    add!("progress.description", Style::new());
    add!("progress.filesize", Style::color(Color::Standard(2)));
    add!("progress.filesize.total", Style::color(Color::Standard(2)));
    add!("progress.download", Style::color(Color::Standard(2)));
    add!("progress.elapsed", Style::color(Color::Standard(3)));
    add!("progress.percentage", Style::color(Color::Standard(5)));
    add!("progress.remaining", Style::color(Color::Standard(6)));
    add!("progress.data.speed", Style::color(Color::Standard(1)));
    add!("progress.spinner", Style::color(Color::Standard(2)));

    add!("status.spinner", Style::color(Color::Standard(2)));

    // Tree styles
    add!("tree", Style::new());
    add!("tree.line", Style::new());

    // Markdown styles
    add!("markdown.paragraph", Style::new());
    add!("markdown.text", Style::new());
    add!("markdown.em", Style::new().with_italic(true));
    add!("markdown.emph", Style::new().with_italic(true));
    add!("markdown.strong", Style::new().with_bold(true));
    add!(
        "markdown.code",
        Style::color(Color::Standard(6))
            .with_bold(true)
            .with_bgcolor(Color::Standard(0))
    );
    add!(
        "markdown.code_block",
        Style::color(Color::Standard(6)).with_bgcolor(Color::Standard(0))
    );
    add!("markdown.block_quote", Style::color(Color::Standard(5)));
    add!("markdown.list", Style::color(Color::Standard(6)));
    add!("markdown.item", Style::new());
    add!(
        "markdown.item.bullet",
        Style::color(Color::Standard(3)).with_bold(true)
    );
    add!(
        "markdown.item.number",
        Style::color(Color::Standard(3)).with_bold(true)
    );
    add!("markdown.hr", Style::color(Color::Standard(3)));
    add!("markdown.h1.border", Style::new());
    add!("markdown.h1", Style::new().with_bold(true));
    add!(
        "markdown.h2",
        Style::new().with_bold(true).with_underline(true)
    );
    add!("markdown.h3", Style::new().with_bold(true));
    add!("markdown.h4", Style::new().with_bold(true).with_dim(true));
    add!("markdown.h5", Style::new().with_underline(true));
    add!("markdown.h6", Style::new().with_italic(true));
    add!("markdown.h7", Style::new().with_italic(true).with_dim(true));
    add!("markdown.link", Style::color(Color::Standard(12)));
    add!(
        "markdown.link_url",
        Style::color(Color::Standard(4)).with_underline(true)
    );
    add!("markdown.s", Style::new().with_strike(true));

    // ISO8601 styles
    add!("iso8601.date", Style::color(Color::Standard(4)));
    add!("iso8601.time", Style::color(Color::Standard(5)));
    add!("iso8601.timezone", Style::color(Color::Standard(3)));

    styles
}

static DEFAULT_STYLES_MAP: Lazy<HashMap<String, Style>> = Lazy::new(default_styles);

/// Get a style from the built-in default styles by name.
///
/// This supports Rich's convention of style names such as `"progress.percentage"`.
pub fn get_default_style(name: &str) -> Option<Style> {
    DEFAULT_STYLES_MAP.get(name).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_new() {
        let theme = Theme::new();
        // Should have default styles
        assert!(theme.has_style("repr.number"));
        assert!(theme.has_style("markdown.h1"));
        assert!(theme.len() > 50);
    }

    #[test]
    fn test_theme_empty() {
        let theme = Theme::empty();
        assert!(theme.is_empty());
        assert!(!theme.has_style("repr.number"));
    }

    #[test]
    fn test_theme_get_style() {
        let theme = Theme::new();
        let style = theme.get_style("repr.number");
        assert!(style.is_some());
        let style = style.unwrap();
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.color, Some(Color::Standard(6)));
    }

    #[test]
    fn test_theme_add_style() {
        let mut theme = Theme::empty();
        theme.add_style("custom", Style::new().with_bold(true));
        assert!(theme.has_style("custom"));
        assert_eq!(theme.get_style("custom").unwrap().bold, Some(true));
    }

    #[test]
    fn test_theme_remove_style() {
        let mut theme = Theme::new();
        assert!(theme.has_style("repr.number"));
        theme.remove_style("repr.number");
        assert!(!theme.has_style("repr.number"));
    }

    #[test]
    fn test_theme_with_styles() {
        let mut custom = HashMap::new();
        custom.insert("my.style".to_string(), Style::new().with_italic(true));

        let theme = Theme::with_styles(custom, true);
        // Should have both default and custom
        assert!(theme.has_style("repr.number"));
        assert!(theme.has_style("my.style"));
    }

    #[test]
    fn test_theme_with_styles_no_inherit() {
        let mut custom = HashMap::new();
        custom.insert("my.style".to_string(), Style::new().with_italic(true));

        let theme = Theme::with_styles(custom, false);
        // Should only have custom
        assert!(!theme.has_style("repr.number"));
        assert!(theme.has_style("my.style"));
    }

    #[test]
    fn test_theme_to_config() {
        let mut theme = Theme::empty();
        theme.add_style("test.bold", Style::new().with_bold(true));
        theme.add_style("test.color", Style::color(Color::Standard(1)));

        let config = theme.to_config();
        assert!(config.starts_with("[styles]"));
        assert!(config.contains("test.bold = bold"));
        assert!(config.contains("test.color = red"));
    }

    #[test]
    fn test_theme_from_reader() {
        let config = r#"
[styles]
custom.style = bold red
another = italic cyan
"#;
        let reader = std::io::Cursor::new(config);
        let theme = Theme::from_reader(reader, false).unwrap();

        assert!(theme.has_style("custom.style"));
        let style = theme.get_style("custom.style").unwrap();
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.color, Some(Color::Standard(1)));

        let another = theme.get_style("another").unwrap();
        assert_eq!(another.italic, Some(true));
        assert_eq!(another.color, Some(Color::Standard(6)));
    }

    #[test]
    fn test_theme_from_reader_with_comments() {
        let config = r#"
# Comment line
[styles]
; Another comment
test = bold
"#;
        let reader = std::io::Cursor::new(config);
        let theme = Theme::from_reader(reader, false).unwrap();
        assert!(theme.has_style("test"));
    }

    #[test]
    fn test_theme_stack_new() {
        let stack = ThemeStack::new(Theme::new());
        assert_eq!(stack.depth(), 1);
        assert!(stack.get_style("repr.number").is_some());
    }

    #[test]
    fn test_theme_stack_push_pop() {
        let mut stack = ThemeStack::new(Theme::new());

        // Create custom theme
        let mut custom = Theme::empty();
        custom.add_style("repr.number", Style::new().with_italic(true));

        // Push custom theme
        stack.push(custom, true);
        assert_eq!(stack.depth(), 2);

        // Custom style should override
        let style = stack.get_style("repr.number").unwrap();
        assert_eq!(style.italic, Some(true));

        // Pop should restore original
        stack.pop().unwrap();
        assert_eq!(stack.depth(), 1);

        let style = stack.get_style("repr.number").unwrap();
        assert_eq!(style.bold, Some(true));
        // Original style has italic = Some(false) (explicitly not italic)
        assert_eq!(style.italic, Some(false));
    }

    #[test]
    fn test_theme_stack_push_inherit() {
        let mut stack = ThemeStack::new(Theme::new());

        let mut custom = Theme::empty();
        custom.add_style("custom.style", Style::new().with_bold(true));

        // Push with inherit=true should keep existing styles
        stack.push(custom, true);
        assert!(stack.get_style("repr.number").is_some());
        assert!(stack.get_style("custom.style").is_some());
    }

    #[test]
    fn test_theme_stack_push_no_inherit() {
        let mut stack = ThemeStack::new(Theme::new());

        let mut custom = Theme::empty();
        custom.add_style("custom.style", Style::new().with_bold(true));

        // Push with inherit=false should only have new styles
        stack.push(custom, false);
        assert!(stack.get_style("repr.number").is_none());
        assert!(stack.get_style("custom.style").is_some());
    }

    #[test]
    fn test_theme_stack_pop_base_error() {
        let mut stack = ThemeStack::new(Theme::new());
        let result = stack.pop();
        assert!(matches!(result, Err(ThemeError::PopBaseTheme)));
    }

    #[test]
    fn test_default_styles_count() {
        let styles = default_styles();
        // Should have roughly the same number as Python's DEFAULT_STYLES
        assert!(
            styles.len() >= 100,
            "Expected at least 100 default styles, got {}",
            styles.len()
        );
    }

    #[test]
    fn test_default_styles_has_expected() {
        let styles = default_styles();

        // Check some key styles exist
        let expected = [
            "none",
            "reset",
            "bold",
            "italic",
            "repr.number",
            "repr.str",
            "repr.bool_true",
            "markdown.h1",
            "markdown.code",
            "log.level",
            "log.time",
            "json.brace",
            "json.key",
            "table.header",
            "table.cell",
            "traceback.error",
            "traceback.title",
            "progress.spinner",
            "progress.percentage",
        ];

        for name in expected {
            assert!(styles.contains_key(name), "Missing default style: {}", name);
        }
    }

    #[test]
    fn test_style_to_string_roundtrip() {
        let style = Style::new().with_bold(true).with_color(Color::Standard(1));

        let s = style_to_string(&style);
        assert!(s.contains("bold"));
        assert!(s.contains("red"));

        let parsed = Style::parse(&s).unwrap();
        assert_eq!(parsed.bold, Some(true));
        assert_eq!(parsed.color, Some(Color::Standard(1)));
    }

    #[test]
    fn test_theme_from_name() {
        // Default theme
        let default = Theme::from_name("default");
        assert!(default.is_some());
        assert!(default.unwrap().has_style("repr.number"));

        // Dracula theme
        let dracula = Theme::from_name("dracula");
        assert!(dracula.is_some());
        let dracula = dracula.unwrap();
        assert!(dracula.has_style("repr.number"));
        // Should have custom colors from the theme file
        let style = dracula.get_style("repr.number").unwrap();
        assert!(style.color.is_some());

        // Gruvbox dark theme
        let gruvbox = Theme::from_name("gruvbox-dark");
        assert!(gruvbox.is_some());

        // Nord theme
        let nord = Theme::from_name("nord");
        assert!(nord.is_some());

        // Unknown theme returns None
        assert!(Theme::from_name("nonexistent").is_none());
    }

    #[test]
    fn test_theme_available_themes() {
        let themes = Theme::available_themes();
        assert!(themes.contains(&"default"));
        assert!(themes.contains(&"dracula"));
        assert!(themes.contains(&"gruvbox-dark"));
        assert!(themes.contains(&"nord"));
    }
}
