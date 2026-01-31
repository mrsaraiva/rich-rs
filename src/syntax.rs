//! Syntax: syntax-highlighted code rendering.
//!
//! This module provides syntax highlighting for source code using the `syntect` crate.
//! It supports various programming languages and color themes.
//!
//! # Example
//!
//! ```
//! use rich_rs::Syntax;
//!
//! let code = r#"fn main() {
//!     println!("Hello, World!");
//! }"#;
//!
//! let syntax = Syntax::new(code, "rust")
//!     .with_theme("monokai")
//!     .with_line_numbers(true);
//! ```

use std::collections::HashSet;
use std::io::Stdout;
use std::path::Path;

use once_cell::sync::Lazy;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::cells::cell_len;
use crate::color::SimpleColor as Color;
use crate::console::{Console, ConsoleOptions};
use crate::measure::Measurement;
use crate::padding::PaddingDimensions;
use crate::segment::{Segment, Segments};
use crate::style::Style;
use crate::text::Text;
use crate::Renderable;

// ============================================================================
// Static syntax and theme sets
// ============================================================================

/// Global syntax set loaded once at startup.
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);

/// Global theme set loaded once at startup.
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// Default theme name.
pub const DEFAULT_THEME: &str = "base16-ocean.dark";

/// Default padding for line numbers column.
pub const NUMBERS_COLUMN_DEFAULT_PADDING: usize = 2;

// ============================================================================
// ANSI Theme Styles (for terminal-friendly themes)
// ============================================================================

/// ANSI-friendly style mappings for light terminals.
pub mod ansi_light {
    use crate::style::Style;
    use crate::color::SimpleColor as Color;

    pub fn comment() -> Style {
        Style::new().with_dim(true)
    }
    pub fn comment_preproc() -> Style {
        Style::new().with_color(Color::Standard(6)) // cyan
    }
    pub fn keyword() -> Style {
        Style::new().with_color(Color::Standard(4)) // blue
    }
    pub fn keyword_type() -> Style {
        Style::new().with_color(Color::Standard(6)) // cyan
    }
    pub fn operator_word() -> Style {
        Style::new().with_color(Color::Standard(5)) // magenta
    }
    pub fn name_builtin() -> Style {
        Style::new().with_color(Color::Standard(6)) // cyan
    }
    pub fn name_function() -> Style {
        Style::new().with_color(Color::Standard(2)) // green
    }
    pub fn name_namespace() -> Style {
        Style::new().with_color(Color::Standard(6)).with_underline(true) // cyan underlined
    }
    pub fn name_class() -> Style {
        Style::new().with_color(Color::Standard(2)).with_underline(true) // green underlined
    }
    pub fn name_decorator() -> Style {
        Style::new().with_color(Color::Standard(5)).with_bold(true) // magenta bold
    }
    pub fn name_variable() -> Style {
        Style::new().with_color(Color::Standard(1)) // red
    }
    pub fn name_attribute() -> Style {
        Style::new().with_color(Color::Standard(6)) // cyan
    }
    pub fn name_tag() -> Style {
        Style::new().with_color(Color::Standard(12)) // bright blue
    }
    pub fn string() -> Style {
        Style::new().with_color(Color::Standard(3)) // yellow
    }
    pub fn number() -> Style {
        Style::new().with_color(Color::Standard(4)) // blue
    }
    pub fn error() -> Style {
        Style::new().with_color(Color::Standard(1)).with_underline(true) // red underlined
    }
}

/// ANSI-friendly style mappings for dark terminals.
pub mod ansi_dark {
    use crate::style::Style;
    use crate::color::SimpleColor as Color;

    pub fn comment() -> Style {
        Style::new().with_dim(true)
    }
    pub fn comment_preproc() -> Style {
        Style::new().with_color(Color::Standard(14)) // bright cyan
    }
    pub fn keyword() -> Style {
        Style::new().with_color(Color::Standard(12)) // bright blue
    }
    pub fn keyword_type() -> Style {
        Style::new().with_color(Color::Standard(14)) // bright cyan
    }
    pub fn operator_word() -> Style {
        Style::new().with_color(Color::Standard(13)) // bright magenta
    }
    pub fn name_builtin() -> Style {
        Style::new().with_color(Color::Standard(14)) // bright cyan
    }
    pub fn name_function() -> Style {
        Style::new().with_color(Color::Standard(10)) // bright green
    }
    pub fn name_namespace() -> Style {
        Style::new().with_color(Color::Standard(14)).with_underline(true) // bright cyan underlined
    }
    pub fn name_class() -> Style {
        Style::new().with_color(Color::Standard(10)).with_underline(true) // bright green underlined
    }
    pub fn name_decorator() -> Style {
        Style::new().with_color(Color::Standard(13)).with_bold(true) // bright magenta bold
    }
    pub fn name_variable() -> Style {
        Style::new().with_color(Color::Standard(9)) // bright red
    }
    pub fn name_attribute() -> Style {
        Style::new().with_color(Color::Standard(14)) // bright cyan
    }
    pub fn name_tag() -> Style {
        Style::new().with_color(Color::Standard(12)) // bright blue
    }
    pub fn string() -> Style {
        Style::new().with_color(Color::Standard(3)) // yellow
    }
    pub fn number() -> Style {
        Style::new().with_color(Color::Standard(12)) // bright blue
    }
    pub fn error() -> Style {
        Style::new().with_color(Color::Standard(1)).with_underline(true) // red underlined
    }
}

// ============================================================================
// SyntaxTheme trait
// ============================================================================

/// Trait for syntax themes.
///
/// Abstracts the theme system to support both syntect themes and ANSI themes.
pub trait SyntaxTheme: Send + Sync {
    /// Get the style for a token (foreground, background).
    fn get_style(&self, style: &SyntectStyle) -> Style;

    /// Get the background style for the theme.
    fn get_background_style(&self) -> Style;

    /// Get the underlying syntect Theme, if available.
    fn syntect_theme(&self) -> Option<&Theme>;
}

/// A syntax theme backed by syntect's Theme.
pub struct SyntectTheme {
    theme: Theme,
    background_style: Style,
}

impl SyntectTheme {
    /// Create a new syntect-based theme.
    pub fn new(theme: Theme) -> Self {
        let bg_color = theme.settings.background.map(|c| {
            Color::Rgb {
                r: c.r,
                g: c.g,
                b: c.b,
            }
        });
        let background_style = match bg_color {
            Some(c) => Style::new().with_bgcolor(c),
            None => Style::new(),
        };
        Self {
            theme,
            background_style,
        }
    }

    /// Load a theme by name.
    pub fn from_name(name: &str) -> Option<Self> {
        THEME_SET.themes.get(name).map(|t| Self::new(t.clone()))
    }
}

impl SyntaxTheme for SyntectTheme {
    fn get_style(&self, style: &SyntectStyle) -> Style {
        let fg = style.foreground;
        let bg = style.background;

        let mut result = Style::new();

        // Foreground color
        result = result.with_color(Color::Rgb {
            r: fg.r,
            g: fg.g,
            b: fg.b,
        });

        // Background color (only if different from theme background)
        if let Some(theme_bg) = self.theme.settings.background {
            if bg.r != theme_bg.r || bg.g != theme_bg.g || bg.b != theme_bg.b {
                result = result.with_bgcolor(Color::Rgb {
                    r: bg.r,
                    g: bg.g,
                    b: bg.b,
                });
            }
        }

        // Font style
        let font_style = style.font_style;
        if font_style.contains(syntect::highlighting::FontStyle::BOLD) {
            result = result.with_bold(true);
        }
        if font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
            result = result.with_italic(true);
        }
        if font_style.contains(syntect::highlighting::FontStyle::UNDERLINE) {
            result = result.with_underline(true);
        }

        result
    }

    fn get_background_style(&self) -> Style {
        self.background_style
    }

    fn syntect_theme(&self) -> Option<&Theme> {
        Some(&self.theme)
    }
}

/// An ANSI-compatible theme that uses standard terminal colors.
pub struct AnsiTheme {
    dark: bool,
}

impl AnsiTheme {
    /// Create a new ANSI theme for dark terminals.
    pub fn dark() -> Self {
        Self { dark: true }
    }

    /// Create a new ANSI theme for light terminals.
    pub fn light() -> Self {
        Self { dark: false }
    }
}

impl SyntaxTheme for AnsiTheme {
    fn get_style(&self, style: &SyntectStyle) -> Style {
        // For ANSI themes, we map the color to the nearest standard color
        let fg = style.foreground;

        // Simple heuristic: map based on the dominant component
        let (r, g, b) = (fg.r as u16, fg.g as u16, fg.b as u16);

        // Calculate luminance-like value
        let brightness = (r + g + b) / 3;

        // Choose a standard color based on the RGB values
        let color = if brightness < 50 {
            // Very dark - use default or black
            Color::Standard(0) // black
        } else if r > 200 && g < 100 && b < 100 {
            // Red-ish
            if self.dark {
                Color::Standard(9)
            } else {
                Color::Standard(1)
            }
        } else if g > 200 && r < 100 && b < 100 {
            // Green-ish
            if self.dark {
                Color::Standard(10)
            } else {
                Color::Standard(2)
            }
        } else if b > 200 && r < 100 && g < 100 {
            // Blue-ish
            if self.dark {
                Color::Standard(12)
            } else {
                Color::Standard(4)
            }
        } else if r > 200 && g > 200 && b < 100 {
            // Yellow-ish
            Color::Standard(3)
        } else if r > 200 && b > 200 && g < 100 {
            // Magenta-ish
            if self.dark {
                Color::Standard(13)
            } else {
                Color::Standard(5)
            }
        } else if g > 200 && b > 200 && r < 100 {
            // Cyan-ish
            if self.dark {
                Color::Standard(14)
            } else {
                Color::Standard(6)
            }
        } else if brightness > 200 {
            // Bright - use white
            Color::Standard(7)
        } else {
            // Default - use the theme's choice
            Color::Rgb {
                r: fg.r,
                g: fg.g,
                b: fg.b,
            }
        };

        let mut result = Style::new().with_color(color);

        let font_style = style.font_style;
        if font_style.contains(syntect::highlighting::FontStyle::BOLD) {
            result = result.with_bold(true);
        }
        if font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
            result = result.with_italic(true);
        }
        if font_style.contains(syntect::highlighting::FontStyle::UNDERLINE) {
            result = result.with_underline(true);
        }

        result
    }

    fn get_background_style(&self) -> Style {
        Style::new()
    }

    fn syntect_theme(&self) -> Option<&Theme> {
        None
    }
}

// ============================================================================
// Syntax struct
// ============================================================================

/// A renderable for syntax-highlighted code.
///
/// `Syntax` renders source code with syntax highlighting, optional line numbers,
/// and various display options.
///
/// # Example
///
/// ```
/// use rich_rs::Syntax;
///
/// let code = "fn main() { println!(\"Hello!\"); }";
/// let syntax = Syntax::new(code, "rust")
///     .with_line_numbers(true)
///     .with_theme("monokai");
/// ```
pub struct Syntax {
    /// The source code to highlight.
    code: String,
    /// The language/lexer name.
    lexer: String,
    /// The theme to use.
    theme: Box<dyn SyntaxTheme>,
    /// Whether to dedent the code.
    dedent: bool,
    /// Whether to show line numbers.
    line_numbers: bool,
    /// Starting line number.
    start_line: usize,
    /// Optional line range to display (start, end).
    line_range: Option<(Option<usize>, Option<usize>)>,
    /// Lines to highlight.
    highlight_lines: HashSet<usize>,
    /// Fixed width for the code area.
    code_width: Option<usize>,
    /// Tab size for expansion.
    tab_size: usize,
    /// Whether to word wrap long lines.
    /// NOTE: Not yet implemented - stored for future use.
    #[allow(dead_code)]
    word_wrap: bool,
    /// Optional background color override.
    background_color: Option<Color>,
    /// Whether to show indent guides.
    /// NOTE: Not yet implemented - stored for future use.
    #[allow(dead_code)]
    indent_guides: bool,
    /// Padding around the syntax block.
    padding: (usize, usize, usize, usize),
}

impl std::fmt::Debug for Syntax {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Syntax")
            .field("code_len", &self.code.len())
            .field("lexer", &self.lexer)
            .field("dedent", &self.dedent)
            .field("line_numbers", &self.line_numbers)
            .field("start_line", &self.start_line)
            .field("line_range", &self.line_range)
            .field("highlight_lines", &self.highlight_lines)
            .field("code_width", &self.code_width)
            .field("tab_size", &self.tab_size)
            .field("word_wrap", &self.word_wrap)
            .field("indent_guides", &self.indent_guides)
            .field("padding", &self.padding)
            .finish_non_exhaustive()
    }
}

impl Syntax {
    /// Create a new Syntax object for the given code and language.
    ///
    /// # Arguments
    ///
    /// * `code` - The source code to highlight.
    /// * `lexer` - The language name (e.g., "rust", "python", "javascript").
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::Syntax;
    ///
    /// let syntax = Syntax::new("print('hello')", "python");
    /// ```
    pub fn new(code: impl Into<String>, lexer: impl Into<String>) -> Self {
        let theme = Self::get_theme(DEFAULT_THEME);
        Self {
            code: code.into(),
            lexer: lexer.into(),
            theme,
            dedent: false,
            line_numbers: false,
            start_line: 1,
            line_range: None,
            highlight_lines: HashSet::new(),
            code_width: None,
            tab_size: 4,
            word_wrap: false,
            background_color: None,
            indent_guides: false,
            padding: (0, 0, 0, 0),
        }
    }

    /// Create a Syntax object from a file path.
    ///
    /// The language is auto-detected from the file extension.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the source file.
    ///
    /// # Returns
    ///
    /// `Ok(Syntax)` if the file was read successfully, `Err` otherwise.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let syntax = Syntax::from_path("src/main.rs")?;
    /// ```
    pub fn from_path(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref();
        let code = std::fs::read_to_string(path)?;
        let lexer = Self::guess_lexer(path, Some(&code));
        Ok(Self::new(code, lexer))
    }

    /// Guess the language/lexer for a file path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to examine.
    /// * `code` - Optional code content for better detection.
    ///
    /// # Returns
    ///
    /// The best-guess language name.
    pub fn guess_lexer(path: impl AsRef<Path>, code: Option<&str>) -> String {
        let path = path.as_ref();

        // Try to find syntax by extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if let Some(syntax) = SYNTAX_SET.find_syntax_by_extension(ext) {
                return syntax.name.to_lowercase();
            }
        }

        // Try by filename
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            // Check for specific filenames
            match filename.to_lowercase().as_str() {
                "makefile" | "gnumakefile" => return "makefile".to_string(),
                "dockerfile" => return "dockerfile".to_string(),
                "cmakelists.txt" => return "cmake".to_string(),
                _ => {}
            }
        }

        // Try to detect from content
        if let Some(code) = code {
            // Simple heuristics
            if code.starts_with("#!/usr/bin/env python") || code.starts_with("#!/usr/bin/python")
            {
                return "python".to_string();
            }
            if code.starts_with("#!/bin/bash") || code.starts_with("#!/usr/bin/env bash") {
                return "bash".to_string();
            }
            if code.starts_with("#!/usr/bin/env node") {
                return "javascript".to_string();
            }
            if code.starts_with("#!/usr/bin/env ruby") {
                return "ruby".to_string();
            }

            // Try syntect's first-line detection
            if let Some(syntax) = SYNTAX_SET.find_syntax_by_first_line(code) {
                return syntax.name.to_lowercase();
            }
        }

        // Default to plain text
        "text".to_string()
    }

    /// Get a theme by name.
    ///
    /// # Arguments
    ///
    /// * `name` - Theme name (e.g., "monokai", "dracula", "github-dark").
    ///
    /// # Returns
    ///
    /// A boxed SyntaxTheme. Falls back to Monokai if the theme is not found.
    pub fn get_theme(name: &str) -> Box<dyn SyntaxTheme> {
        // Check for ANSI themes
        match name.to_lowercase().as_str() {
            "ansi_dark" | "ansi-dark" => return Box::new(AnsiTheme::dark()),
            "ansi_light" | "ansi-light" => return Box::new(AnsiTheme::light()),
            _ => {}
        }

        // Try to find syntect theme
        let theme_name = match name.to_lowercase().as_str() {
            "monokai" => "base16-mocha.dark",  // closest to Monokai in default themes
            "dracula" => "base16-eighties.dark", // closest to Dracula in default themes
            "one-dark" | "onedark" => "base16-ocean.dark",
            "one-light" | "onelight" => "base16-ocean.light",
            "github-dark" => "base16-ocean.dark",
            "github-light" => "base16-ocean.light",
            "solarized-dark" => "Solarized (dark)",
            "solarized-light" => "Solarized (light)",
            _ => name,
        };

        SyntectTheme::from_name(theme_name)
            .map(|t| Box::new(t) as Box<dyn SyntaxTheme>)
            .unwrap_or_else(|| {
                // Fall back to default theme
                SyntectTheme::from_name(DEFAULT_THEME)
                    .map(|t| Box::new(t) as Box<dyn SyntaxTheme>)
                    .unwrap_or_else(|| Box::new(AnsiTheme::dark()))
            })
    }

    /// List available theme names.
    pub fn available_themes() -> Vec<&'static str> {
        let mut themes: Vec<&str> = THEME_SET.themes.keys().map(|s| s.as_str()).collect();
        themes.extend(["ansi_dark", "ansi_light"]);
        themes.sort();
        themes
    }

    /// List available language/lexer names.
    pub fn available_languages() -> Vec<String> {
        SYNTAX_SET
            .syntaxes()
            .iter()
            .map(|s| s.name.to_lowercase())
            .collect()
    }

    // ========================================================================
    // Builder methods
    // ========================================================================

    /// Set the theme by name.
    pub fn with_theme(mut self, theme: impl AsRef<str>) -> Self {
        self.theme = Self::get_theme(theme.as_ref());
        self
    }

    /// Set a custom theme.
    pub fn with_custom_theme(mut self, theme: Box<dyn SyntaxTheme>) -> Self {
        self.theme = theme;
        self
    }

    /// Enable or disable code dedenting.
    pub fn with_dedent(mut self, dedent: bool) -> Self {
        self.dedent = dedent;
        self
    }

    /// Enable or disable line numbers.
    pub fn with_line_numbers(mut self, line_numbers: bool) -> Self {
        self.line_numbers = line_numbers;
        self
    }

    /// Set the starting line number.
    pub fn with_start_line(mut self, start_line: usize) -> Self {
        self.start_line = start_line;
        self
    }

    /// Set the line range to display.
    ///
    /// # Arguments
    ///
    /// * `start` - Optional start line (1-based, inclusive).
    /// * `end` - Optional end line (1-based, inclusive).
    pub fn with_line_range(mut self, start: Option<usize>, end: Option<usize>) -> Self {
        self.line_range = Some((start, end));
        self
    }

    /// Set lines to highlight.
    pub fn with_highlight_lines(mut self, lines: impl IntoIterator<Item = usize>) -> Self {
        self.highlight_lines = lines.into_iter().collect();
        self
    }

    /// Set a fixed code width.
    pub fn with_code_width(mut self, width: usize) -> Self {
        self.code_width = Some(width);
        self
    }

    /// Set the tab size.
    pub fn with_tab_size(mut self, tab_size: usize) -> Self {
        self.tab_size = tab_size;
        self
    }

    /// Enable or disable word wrapping.
    ///
    /// NOTE: Not yet implemented - option is stored for future use.
    pub fn with_word_wrap(mut self, word_wrap: bool) -> Self {
        self.word_wrap = word_wrap;
        self
    }

    /// Set a background color override.
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Enable or disable indent guides.
    ///
    /// NOTE: Not yet implemented - option is stored for future use.
    pub fn with_indent_guides(mut self, indent_guides: bool) -> Self {
        self.indent_guides = indent_guides;
        self
    }

    /// Set padding around the syntax block.
    pub fn with_padding(mut self, padding: impl Into<PaddingDimensions>) -> Self {
        self.padding = padding.into().unpack();
        self
    }

    // ========================================================================
    // Getters
    // ========================================================================

    /// Get the source code.
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Get the lexer/language name.
    pub fn lexer(&self) -> &str {
        &self.lexer
    }

    /// Check if line numbers are enabled.
    pub fn line_numbers(&self) -> bool {
        self.line_numbers
    }

    /// Get the tab size.
    pub fn tab_size(&self) -> usize {
        self.tab_size
    }

    // ========================================================================
    // Highlighting
    // ========================================================================

    /// Highlight the code and return a Text object.
    ///
    /// This converts syntect-highlighted code into a rich-rs Text object
    /// with styled spans.
    pub fn highlight(&self) -> Text {
        let (ends_on_nl, processed_code) = self.process_code();

        // Find the syntax
        let syntax = SYNTAX_SET
            .find_syntax_by_token(&self.lexer)
            .or_else(|| SYNTAX_SET.find_syntax_by_extension(&self.lexer))
            .or_else(|| SYNTAX_SET.find_syntax_by_name(&self.lexer))
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

        let base_style = self.get_base_style();
        let mut text = Text::new();
        text.set_base_style(Some(base_style));

        // Highlight the code
        if let Some(syntect_theme) = self.theme.syntect_theme() {
            let mut highlighter = HighlightLines::new(syntax, syntect_theme);

            for line in LinesWithEndings::from(&processed_code) {
                match highlighter.highlight_line(line, &SYNTAX_SET) {
                    Ok(ranges) => {
                        for (style, token) in ranges {
                            let rich_style = self.theme.get_style(&style);
                            text.append(token, Some(rich_style));
                        }
                    }
                    Err(_) => {
                        // Fall back to unstyled text
                        text.append(line, None);
                    }
                }
            }
        } else {
            // For ANSI themes without syntect theme, use plain highlighting
            let mut highlighter = HighlightLines::new(
                syntax,
                &THEME_SET.themes[DEFAULT_THEME],
            );

            for line in LinesWithEndings::from(&processed_code) {
                match highlighter.highlight_line(line, &SYNTAX_SET) {
                    Ok(ranges) => {
                        for (style, token) in ranges {
                            let rich_style = self.theme.get_style(&style);
                            text.append(token, Some(rich_style));
                        }
                    }
                    Err(_) => {
                        text.append(line, None);
                    }
                }
            }
        }

        // Remove trailing newline if the original didn't have one
        if !ends_on_nl && text.plain_text().ends_with('\n') {
            let plain = text.plain_text();
            let new_plain = plain.trim_end_matches('\n');
            if plain != new_plain {
                // Reconstruct text without trailing newline
                let mut new_text = Text::new();
                new_text.set_base_style(text.base_style());
                new_text.append(new_plain, None);
                // Copy spans but adjust for new length
                for span in text.spans() {
                    if span.start < new_plain.chars().count() {
                        new_text.stylize(
                            span.start,
                            span.end.min(new_plain.chars().count()),
                            span.style,
                        );
                    }
                }
                return new_text;
            }
        }

        text
    }

    /// Get the base style for the syntax block.
    fn get_base_style(&self) -> Style {
        let mut style = self.theme.get_background_style();
        if let Some(bg) = self.background_color {
            style = style.with_bgcolor(bg);
        }
        style
    }

    /// Process the code (dedent, normalize newlines, expand tabs).
    fn process_code(&self) -> (bool, String) {
        let ends_on_nl = self.code.ends_with('\n');
        let mut processed = if ends_on_nl {
            self.code.clone()
        } else {
            format!("{}\n", self.code)
        };

        // Dedent if requested
        if self.dedent {
            processed = Self::dedent_code(&processed);
        }

        // Expand tabs
        processed = Self::expand_tabs(&processed, self.tab_size);

        (ends_on_nl, processed)
    }

    /// Dedent code by removing common leading whitespace.
    fn dedent_code(code: &str) -> String {
        let lines: Vec<&str> = code.lines().collect();

        // Find minimum indentation (ignoring empty lines)
        let min_indent = lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.len() - line.trim_start().len())
            .min()
            .unwrap_or(0);

        if min_indent == 0 {
            return code.to_string();
        }

        // Remove min_indent from each line
        lines
            .iter()
            .map(|line| {
                if line.len() >= min_indent {
                    &line[min_indent..]
                } else {
                    line.trim_start()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + if code.ends_with('\n') { "\n" } else { "" }
    }

    /// Expand tabs to spaces.
    fn expand_tabs(code: &str, tab_size: usize) -> String {
        if !code.contains('\t') {
            return code.to_string();
        }

        let mut result = String::new();
        let mut column = 0;

        for c in code.chars() {
            match c {
                '\t' => {
                    let spaces = tab_size - (column % tab_size);
                    for _ in 0..spaces {
                        result.push(' ');
                    }
                    column += spaces;
                }
                '\n' => {
                    result.push(c);
                    column = 0;
                }
                _ => {
                    result.push(c);
                    column += 1;
                }
            }
        }

        result
    }

    /// Get the width of the line numbers column.
    fn numbers_column_width(&self) -> usize {
        if !self.line_numbers {
            return 0;
        }
        let line_count = self.code.lines().count();
        let max_line_no = self.start_line + line_count.saturating_sub(1);
        let digits = max_line_no.to_string().len();
        digits + NUMBERS_COLUMN_DEFAULT_PADDING
    }
}

// SAFETY: Syntax is Send + Sync because:
// - All fields are Send + Sync or owned types
// - theme: Box<dyn SyntaxTheme> where SyntaxTheme: Send + Sync
unsafe impl Send for Syntax {}
unsafe impl Sync for Syntax {}

impl Renderable for Syntax {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let mut result = Segments::new();

        let (pad_top, pad_right, pad_bottom, pad_left) = self.padding;
        let horizontal_padding = pad_left + pad_right;

        // Calculate available width
        let numbers_width = self.numbers_column_width();
        let code_width = if let Some(w) = self.code_width {
            w
        } else if self.line_numbers {
            options
                .max_width
                .saturating_sub(numbers_width + 1 + horizontal_padding)
        } else {
            options.max_width.saturating_sub(horizontal_padding)
        };

        // Get highlighted text
        let text = self.highlight();

        // Split into lines
        let lines: Vec<Text> = text.split("\n", false, true);

        // Apply line range filter
        let (start_idx, end_idx) = if let Some((start, end)) = self.line_range {
            let start = start.map(|s| s.saturating_sub(1)).unwrap_or(0);
            let end = end.unwrap_or(lines.len());
            (start, end)
        } else {
            (0, lines.len())
        };

        let filtered_lines: Vec<&Text> = lines
            .iter()
            .skip(start_idx)
            .take(end_idx.saturating_sub(start_idx))
            .collect();

        let base_style = self.get_base_style();
        let new_line = Segment::line();

        // Line number styling
        let number_style = if base_style.bgcolor.is_some() {
            Style::new().with_dim(true)
        } else {
            Style::new().with_dim(true)
        };

        let highlight_number_style = Style::new().with_bold(true);

        // Add top padding
        if pad_top > 0 {
            let blank = " ".repeat(options.max_width);
            for _ in 0..pad_top {
                result.push(Segment::styled(blank.clone(), base_style));
                result.push(new_line.clone());
            }
        }

        // Render each line
        for (idx, line) in filtered_lines.iter().enumerate() {
            let line_no = self.start_line + start_idx + idx;
            let is_highlighted = self.highlight_lines.contains(&line_no);

            // Left padding
            if pad_left > 0 {
                result.push(Segment::styled(" ".repeat(pad_left), base_style));
            }

            // Line number
            if self.line_numbers {
                let pointer = if is_highlighted { "> " } else { "  " };
                let line_num_str =
                    format!("{:>width$} ", line_no, width = numbers_width - 2);

                if is_highlighted {
                    result.push(Segment::styled(
                        pointer.to_string(),
                        Style::new().with_color(Color::Standard(1)), // red
                    ));
                    result.push(Segment::styled(line_num_str, highlight_number_style));
                } else {
                    result.push(Segment::styled(pointer.to_string(), number_style));
                    result.push(Segment::styled(line_num_str, number_style));
                }
            }

            // Render line content
            let line_segments = line.render(console, &options.update_width(code_width));
            for seg in line_segments {
                result.push(seg);
            }

            // Pad to code width if needed
            let line_len = line.cell_len();
            if line_len < code_width {
                result.push(Segment::styled(
                    " ".repeat(code_width - line_len),
                    base_style,
                ));
            }

            // Right padding
            if pad_right > 0 {
                result.push(Segment::styled(" ".repeat(pad_right), base_style));
            }

            result.push(new_line.clone());
        }

        // Add bottom padding
        if pad_bottom > 0 {
            let blank = " ".repeat(options.max_width);
            for _ in 0..pad_bottom {
                result.push(Segment::styled(blank.clone(), base_style));
                result.push(new_line.clone());
            }
        }

        result
    }

    fn measure(&self, _console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        let (_, pad_right, _, pad_left) = self.padding;
        let horizontal_padding = pad_left + pad_right;

        let numbers_width = self.numbers_column_width();

        if let Some(code_width) = self.code_width {
            let width = code_width + numbers_width + horizontal_padding;
            if self.line_numbers {
                return Measurement::new(numbers_width, width + 1);
            }
            return Measurement::new(numbers_width, width);
        }

        // Calculate from code
        let lines: Vec<&str> = self.code.lines().collect();
        let max_line_width = lines.iter().map(|l| cell_len(l)).max().unwrap_or(0);

        let width = max_line_width + numbers_width + horizontal_padding;
        let width = if self.line_numbers { width + 1 } else { width };

        Measurement::new(numbers_width.max(1), width.min(options.max_width))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_new() {
        let syntax = Syntax::new("fn main() {}", "rust");
        assert_eq!(syntax.code(), "fn main() {}");
        assert_eq!(syntax.lexer(), "rust");
    }

    #[test]
    fn test_syntax_with_line_numbers() {
        let syntax = Syntax::new("fn main() {}", "rust").with_line_numbers(true);
        assert!(syntax.line_numbers());
    }

    #[test]
    fn test_syntax_with_theme() {
        let syntax = Syntax::new("fn main() {}", "rust").with_theme("monokai");
        // Theme should be set (we can't easily inspect it, but it shouldn't panic)
        assert_eq!(syntax.code(), "fn main() {}");
    }

    #[test]
    fn test_syntax_with_tab_size() {
        let syntax = Syntax::new("fn main() {}", "rust").with_tab_size(2);
        assert_eq!(syntax.tab_size(), 2);
    }

    #[test]
    fn test_syntax_highlight() {
        let syntax = Syntax::new("fn main() {}", "rust");
        let text = syntax.highlight();
        // The highlighted text should contain the code
        assert!(text.plain_text().contains("fn"));
        assert!(text.plain_text().contains("main"));
    }

    #[test]
    fn test_syntax_highlight_python() {
        let code = r#"def hello():
    print("Hello, World!")
"#;
        let syntax = Syntax::new(code, "python");
        let text = syntax.highlight();
        assert!(text.plain_text().contains("def"));
        assert!(text.plain_text().contains("hello"));
    }

    #[test]
    fn test_syntax_dedent() {
        let code = "    fn main() {\n        println!(\"hello\");\n    }";
        let syntax = Syntax::new(code, "rust").with_dedent(true);
        let text = syntax.highlight();
        // After dedenting, the first line should start with "fn"
        assert!(text.plain_text().starts_with("fn"));
    }

    #[test]
    fn test_syntax_expand_tabs() {
        let code = "fn main() {\n\tprintln!(\"hello\");\n}";
        let syntax = Syntax::new(code, "rust").with_tab_size(4);
        let text = syntax.highlight();
        // Tabs should be expanded to spaces
        assert!(!text.plain_text().contains('\t'));
    }

    #[test]
    fn test_guess_lexer_by_extension() {
        assert_eq!(Syntax::guess_lexer("test.rs", None), "rust");
        assert_eq!(Syntax::guess_lexer("test.py", None), "python");
        assert_eq!(Syntax::guess_lexer("test.js", None), "javascript");
    }

    #[test]
    fn test_available_themes() {
        let themes = Syntax::available_themes();
        assert!(!themes.is_empty());
        assert!(themes.contains(&"ansi_dark"));
        assert!(themes.contains(&"ansi_light"));
    }

    #[test]
    fn test_available_languages() {
        let languages = Syntax::available_languages();
        assert!(!languages.is_empty());
    }

    #[test]
    fn test_numbers_column_width() {
        let code = "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10";
        let syntax = Syntax::new(code, "text").with_line_numbers(true);
        // 10 lines = 2 digits + 2 padding = 4
        assert_eq!(syntax.numbers_column_width(), 4);
    }

    #[test]
    fn test_syntax_render() {
        let syntax = Syntax::new("fn main() {}", "rust");
        let console = Console::new();
        let options = ConsoleOptions::default();

        let segments = syntax.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(output.contains("fn"));
        assert!(output.contains("main"));
    }

    #[test]
    fn test_syntax_render_with_line_numbers() {
        let syntax = Syntax::new("line1\nline2", "text").with_line_numbers(true);
        let console = Console::new();
        let options = ConsoleOptions::default();

        let segments = syntax.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should contain line numbers
        assert!(output.contains('1'));
        assert!(output.contains('2'));
    }

    #[test]
    fn test_syntax_measure() {
        let syntax = Syntax::new("hello", "text");
        let console = Console::new();
        let options = ConsoleOptions::default();

        let measurement = syntax.measure(&console, &options);
        assert!(measurement.maximum >= 5); // At least the length of "hello"
    }

    #[test]
    fn test_syntax_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Syntax>();
        assert_sync::<Syntax>();
    }

    #[test]
    fn test_dedent_code() {
        let code = "    line1\n    line2\n    line3\n";
        let dedented = Syntax::dedent_code(code);
        assert_eq!(dedented, "line1\nline2\nline3\n");
    }

    #[test]
    fn test_dedent_code_mixed_indent() {
        let code = "    line1\n        line2\n    line3\n";
        let dedented = Syntax::dedent_code(code);
        assert_eq!(dedented, "line1\n    line2\nline3\n");
    }

    #[test]
    fn test_expand_tabs() {
        let code = "a\tb\tc";
        let expanded = Syntax::expand_tabs(code, 4);
        assert_eq!(expanded, "a   b   c");
    }

    #[test]
    fn test_expand_tabs_preserves_newlines() {
        let code = "a\tb\nc\td";
        let expanded = Syntax::expand_tabs(code, 4);
        assert_eq!(expanded, "a   b\nc   d");
    }

    #[test]
    fn test_ansi_theme() {
        let theme = AnsiTheme::dark();
        let style = SyntectStyle {
            foreground: syntect::highlighting::Color {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
            background: syntect::highlighting::Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            font_style: syntect::highlighting::FontStyle::empty(),
        };
        let rich_style = theme.get_style(&style);
        // Should have a color set
        assert!(rich_style.color.is_some());
    }

    #[test]
    fn test_syntect_theme() {
        let theme = SyntectTheme::from_name(DEFAULT_THEME);
        assert!(theme.is_some(), "Default theme '{}' should exist", DEFAULT_THEME);
        let theme = theme.unwrap();
        assert!(theme.syntect_theme().is_some());
    }

    #[test]
    fn test_line_range() {
        let code = "line1\nline2\nline3\nline4\nline5";
        let syntax = Syntax::new(code, "text").with_line_range(Some(2), Some(4));
        let console = Console::new();
        let options = ConsoleOptions::default();

        let segments = syntax.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Should contain lines 2-4
        assert!(output.contains("line2"));
        assert!(output.contains("line3"));
        assert!(output.contains("line4"));
        // Should not contain line1 or line5
        assert!(!output.contains("line1"));
        assert!(!output.contains("line5"));
    }

    #[test]
    fn test_highlight_lines() {
        let code = "line1\nline2\nline3";
        let syntax = Syntax::new(code, "text")
            .with_line_numbers(true)
            .with_highlight_lines([2]);
        let console = Console::new();
        let options = ConsoleOptions::default();

        let segments = syntax.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // Line 2 should be highlighted with ">"
        assert!(output.contains('>'));
    }
}
