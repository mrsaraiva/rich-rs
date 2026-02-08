//! Style: text formatting attributes.
//!
//! Styles are immutable and can be combined using the `+` operator or `combine` method.
//!
//! The core `Style` struct is `Copy` for efficiency. For advanced features like
//! hyperlinks and metadata (used by Textual), use `StyleMeta` separately.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::color::{ColorSystem, SimpleColor as Color};

/// A null style with all attributes set to `None`.
///
/// This is useful as a default or starting point for style combinations.
pub const NULL_STYLE: Style = Style {
    color: None,
    bgcolor: None,
    bold: None,
    dim: None,
    italic: None,
    underline: None,
    blink: None,
    reverse: None,
    strike: None,
};

/// Text style with color and attributes.
///
/// Uses `Option<bool>` for attributes to support three states:
/// - `None`: inherit from parent
/// - `Some(true)`: enable
/// - `Some(false)`: disable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    /// Foreground color.
    pub color: Option<Color>,
    /// Background color.
    pub bgcolor: Option<Color>,
    /// Bold text.
    pub bold: Option<bool>,
    /// Dim/faint text.
    pub dim: Option<bool>,
    /// Italic text.
    pub italic: Option<bool>,
    /// Underlined text.
    pub underline: Option<bool>,
    /// Blinking text.
    pub blink: Option<bool>,
    /// Reverse video (swap fg/bg).
    pub reverse: Option<bool>,
    /// Strikethrough text.
    pub strike: Option<bool>,
}

impl Style {
    /// Create a new empty style.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a style with a foreground color.
    pub fn color(color: Color) -> Self {
        Self {
            color: Some(color),
            ..Default::default()
        }
    }

    /// Create a style with a background color.
    pub fn bgcolor(color: Color) -> Self {
        Self {
            bgcolor: Some(color),
            ..Default::default()
        }
    }

    /// Builder: set foreground color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Builder: set background color.
    pub fn with_bgcolor(mut self, color: Color) -> Self {
        self.bgcolor = Some(color);
        self
    }

    /// Builder: set bold.
    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = Some(bold);
        self
    }

    /// Builder: set dim.
    pub fn with_dim(mut self, dim: bool) -> Self {
        self.dim = Some(dim);
        self
    }

    /// Builder: set italic.
    pub fn with_italic(mut self, italic: bool) -> Self {
        self.italic = Some(italic);
        self
    }

    /// Builder: set underline.
    pub fn with_underline(mut self, underline: bool) -> Self {
        self.underline = Some(underline);
        self
    }

    /// Builder: set reverse video.
    pub fn with_reverse(mut self, reverse: bool) -> Self {
        self.reverse = Some(reverse);
        self
    }

    /// Builder: set strike.
    pub fn with_strike(mut self, strike: bool) -> Self {
        self.strike = Some(strike);
        self
    }

    /// Combine this style with another, with `other` taking precedence.
    ///
    /// Values from `other` override values from `self` only if they are `Some`.
    pub fn combine(&self, other: &Style) -> Self {
        Style {
            color: other.color.or(self.color),
            bgcolor: other.bgcolor.or(self.bgcolor),
            bold: other.bold.or(self.bold),
            dim: other.dim.or(self.dim),
            italic: other.italic.or(self.italic),
            underline: other.underline.or(self.underline),
            blink: other.blink.or(self.blink),
            reverse: other.reverse.or(self.reverse),
            strike: other.strike.or(self.strike),
        }
    }

    /// Parse a style from a string.
    ///
    /// Supports space-separated style definitions like:
    /// - "bold red on blue"
    /// - "italic #ff0000"
    /// - "bold underline"
    /// - "not bold" (negation)
    pub fn parse(s: &str) -> Option<Self> {
        let mut style = Style::new();
        let mut on_background = false;

        let mut words = s.split_whitespace().peekable();

        while let Some(word) = words.next() {
            let word_lower = word.to_lowercase();

            if word_lower == "on" {
                on_background = true;
                continue;
            }

            if on_background {
                if let Some(color) = Color::parse(&word_lower) {
                    style.bgcolor = Some(color);
                    on_background = false;
                    continue;
                }
                // If the token wasn't a valid color, drop the background flag and
                // continue processing it normally.
                on_background = false;
            }

            // Support Rich-style named styles from the default theme, e.g. "progress.percentage".
            // If a token matches a default style name, merge it into the current style.
            if let Some(named) = crate::theme::get_default_style(&word_lower) {
                style = style.combine(&named);
                continue;
            }

            // Handle negation: "not bold", "not italic", etc.
            // Also handles shorthand: "not b", "not i", "not u", "not s"
            if word_lower == "not" {
                if let Some(&next_word) = words.peek() {
                    let next_lower = next_word.to_lowercase();
                    match next_lower.as_str() {
                        "bold" | "b" => {
                            style.bold = Some(false);
                            words.next();
                            continue;
                        }
                        "dim" => {
                            style.dim = Some(false);
                            words.next();
                            continue;
                        }
                        "italic" | "i" => {
                            style.italic = Some(false);
                            words.next();
                            continue;
                        }
                        "underline" | "u" => {
                            style.underline = Some(false);
                            words.next();
                            continue;
                        }
                        "blink" => {
                            style.blink = Some(false);
                            words.next();
                            continue;
                        }
                        "reverse" => {
                            style.reverse = Some(false);
                            words.next();
                            continue;
                        }
                        "strike" | "s" => {
                            style.strike = Some(false);
                            words.next();
                            continue;
                        }
                        _ => {}
                    }
                }
                // "not" without valid attribute - ignore
                continue;
            }

            // Check for attributes (including shorthand: b, i, u, s)
            match word_lower.as_str() {
                "bold" | "b" => style.bold = Some(true),
                "dim" => style.dim = Some(true),
                "italic" | "i" => style.italic = Some(true),
                "underline" | "u" => style.underline = Some(true),
                "blink" => style.blink = Some(true),
                "reverse" => style.reverse = Some(true),
                "strike" | "s" => style.strike = Some(true),
                _ => {
                    // Try to parse as color
                    if let Some(color) = Color::parse(&word_lower) {
                        style.color = Some(color);
                    }
                }
            }
        }

        Some(style)
    }

    /// Check if this is a null style (all attributes are None).
    pub fn is_null(&self) -> bool {
        *self == NULL_STYLE
    }

    /// Render text with this style using ANSI escape codes.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to style.
    /// * `color_system` - The color system to render to.
    ///
    /// # Returns
    ///
    /// A string containing the text wrapped in ANSI escape codes.
    /// If text is empty, returns empty string.
    ///
    /// # Examples
    ///
    /// ```
    /// use rich_rs::{Style, ColorSystem, SimpleColor};
    ///
    /// let style = Style::new().with_bold(true).with_color(SimpleColor::Standard(1));
    /// let rendered = style.render("Hello", ColorSystem::TrueColor);
    /// assert!(rendered.contains("\x1b["));
    /// assert!(rendered.contains("Hello"));
    /// assert!(rendered.ends_with("\x1b[0m"));
    /// ```
    pub fn render(&self, text: &str, color_system: ColorSystem) -> String {
        if text.is_empty() {
            return String::new();
        }

        let attrs = self.make_ansi_codes(color_system);
        if attrs.is_empty() {
            text.to_string()
        } else {
            format!("\x1b[{}m{}\x1b[0m", attrs, text)
        }
    }

    /// Render styled text WITHOUT a trailing reset.
    ///
    /// This is used for streaming output where we want to minimize SGR resets
    /// to avoid visual artifacts (like black hairlines between colored lines).
    /// The caller is responsible for emitting a reset at the end.
    pub fn render_open(&self, text: &str, color_system: ColorSystem) -> String {
        if text.is_empty() {
            return String::new();
        }

        let attrs = self.make_ansi_codes(color_system);
        if attrs.is_empty() {
            text.to_string()
        } else {
            format!("\x1b[{}m{}", attrs, text)
        }
    }

    /// Generate the ANSI SGR codes for this style.
    ///
    /// Returns a semicolon-separated string of SGR parameters.
    fn make_ansi_codes(&self, color_system: ColorSystem) -> String {
        let mut sgr: Vec<String> = Vec::new();

        // SGR reset codes for explicitly disabled attributes (emit "off" before "on"):
        // 22 = bold/dim off (resets both)
        // 23 = italic off
        // 24 = underline off
        // 25 = blink off
        // 27 = reverse off
        // 29 = strike off
        //
        // Note: SGR 22 resets both bold AND dim, so we only emit it once if either is false.
        if self.bold == Some(false) || self.dim == Some(false) {
            sgr.push("22".to_string());
        }
        if self.italic == Some(false) {
            sgr.push("23".to_string());
        }
        if self.underline == Some(false) {
            sgr.push("24".to_string());
        }
        if self.blink == Some(false) {
            sgr.push("25".to_string());
        }
        if self.reverse == Some(false) {
            sgr.push("27".to_string());
        }
        if self.strike == Some(false) {
            sgr.push("29".to_string());
        }

        // SGR codes for enabled attributes:
        // bold=1, dim=2, italic=3, underline=4, blink=5, blink2=6, reverse=7, conceal=8, strike=9
        if self.bold == Some(true) {
            sgr.push("1".to_string());
        }
        if self.dim == Some(true) {
            sgr.push("2".to_string());
        }
        if self.italic == Some(true) {
            sgr.push("3".to_string());
        }
        if self.underline == Some(true) {
            sgr.push("4".to_string());
        }
        if self.blink == Some(true) {
            sgr.push("5".to_string());
        }
        if self.reverse == Some(true) {
            sgr.push("7".to_string());
        }
        if self.strike == Some(true) {
            sgr.push("9".to_string());
        }

        // Foreground color
        if let Some(color) = self.color {
            let downgraded = color.downgrade(color_system);
            sgr.extend(downgraded.get_ansi_codes(true));
        }

        // Background color
        if let Some(bgcolor) = self.bgcolor {
            let downgraded = bgcolor.downgrade(color_system);
            sgr.extend(downgraded.get_ansi_codes(false));
        }

        sgr.join(";")
    }

    /// Get a CSS style string for this style.
    ///
    /// # Returns
    ///
    /// A semicolon-separated CSS string suitable for use in a `style` attribute.
    ///
    /// # Examples
    ///
    /// ```
    /// use rich_rs::{Style, SimpleColor};
    ///
    /// let style = Style::new()
    ///     .with_bold(true)
    ///     .with_color(SimpleColor::Rgb { r: 255, g: 0, b: 0 });
    /// let css = style.get_html_style();
    /// assert!(css.contains("font-weight: bold"));
    /// assert!(css.contains("color:"));
    /// ```
    pub fn get_html_style(&self) -> String {
        let mut css: Vec<String> = Vec::new();

        // Handle reverse by swapping colors conceptually
        let (color, bgcolor) = if self.reverse == Some(true) {
            (self.bgcolor, self.color)
        } else {
            (self.color, self.bgcolor)
        };

        // Foreground color
        if let Some(c) = color {
            let hex = c.get_hex();
            css.push(format!("color: {}", hex));
            css.push(format!("text-decoration-color: {}", hex));
        }

        // Background color
        if let Some(c) = bgcolor {
            let hex = c.get_hex();
            css.push(format!("background-color: {}", hex));
        }

        // Text attributes
        if self.bold == Some(true) {
            css.push("font-weight: bold".to_string());
        }
        if self.italic == Some(true) {
            css.push("font-style: italic".to_string());
        }

        // Collect text-decoration values to avoid clobbering
        let mut decorations = Vec::new();
        if self.underline == Some(true) {
            decorations.push("underline");
        }
        if self.strike == Some(true) {
            decorations.push("line-through");
        }
        if !decorations.is_empty() {
            css.push(format!("text-decoration: {}", decorations.join(" ")));
        }

        css.join("; ")
    }
}

impl std::ops::Add for Style {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        self.combine(&other)
    }
}

/// Metadata for styles, used for hyperlinks and custom data.
///
/// This is kept separate from `Style` to preserve `Style: Copy` for the
/// common case. Only segments with links or metadata need a `StyleMeta`.
///
/// Uses `BTreeMap` instead of `HashMap` for deterministic ordering,
/// which is important for segment simplification and serialization.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StyleMeta {
    /// Hyperlink URL (terminal OSC 8 escape sequence).
    pub link: Option<Arc<str>>,
    /// Link ID for grouping multiple segments with the same link.
    pub link_id: Option<Arc<str>>,
    /// Custom metadata (used by Textual for event handlers).
    pub meta: Option<Arc<BTreeMap<String, MetaValue>>>,
}

impl StyleMeta {
    /// Create a new empty StyleMeta.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a StyleMeta with a hyperlink.
    pub fn with_link(link: impl Into<Arc<str>>) -> Self {
        StyleMeta {
            link: Some(link.into()),
            ..Default::default()
        }
    }

    /// Check if this meta has any content.
    pub fn is_empty(&self) -> bool {
        self.link.is_none() && self.link_id.is_none() && self.meta.is_none()
    }

    /// Combine with another StyleMeta, with `other` taking precedence.
    pub fn combine(&self, other: &StyleMeta) -> Self {
        StyleMeta {
            link: other.link.clone().or_else(|| self.link.clone()),
            link_id: other.link_id.clone().or_else(|| self.link_id.clone()),
            meta: match (&self.meta, &other.meta) {
                (Some(a), Some(b)) => {
                    let mut merged = (**a).clone();
                    merged.extend((**b).clone());
                    Some(Arc::new(merged))
                }
                (None, Some(b)) => Some(b.clone()),
                (Some(a), None) => Some(a.clone()),
                (None, None) => None,
            },
        }
    }
}

/// Structured metadata values (used by Textual handlers and richer annotations).
///
/// This is intentionally deterministic and `Eq` so it can be compared / merged
/// and used in segment simplification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaValue {
    None,
    Bool(bool),
    Int(i64),
    Str(Arc<str>),
    List(Vec<MetaValue>),
    Tuple(Vec<MetaValue>),
    Map(BTreeMap<String, MetaValue>),
}

impl Default for MetaValue {
    fn default() -> Self {
        MetaValue::None
    }
}

impl MetaValue {
    pub fn str(value: impl Into<Arc<str>>) -> Self {
        MetaValue::Str(value.into())
    }

    /// Parse a small, deterministic subset of Python literals (like `ast.literal_eval`).
    ///
    /// Supported:
    /// - `None`, `True`, `False`
    /// - integers (base-10)
    /// - quoted strings `'...'` / `"..."` with basic escapes
    /// - lists `[a, b]`
    /// - tuples `(a, b)` (single element tuples require a trailing comma, like Python)
    /// - dicts `{'k': 1}` (string keys only)
    pub fn parse_python_literal(input: &str) -> Option<Self> {
        struct Parser<'a> {
            s: &'a str,
            i: usize,
        }

        impl<'a> Parser<'a> {
            fn new(s: &'a str) -> Self {
                Self { s, i: 0 }
            }

            fn is_eof(&self) -> bool {
                self.i >= self.s.len()
            }

            fn rest(&self) -> &'a str {
                &self.s[self.i..]
            }

            fn skip_ws(&mut self) {
                while let Some(ch) = self.peek() {
                    if ch.is_whitespace() {
                        self.bump(ch);
                    } else {
                        break;
                    }
                }
            }

            fn peek(&self) -> Option<char> {
                self.rest().chars().next()
            }

            fn bump(&mut self, ch: char) {
                self.i += ch.len_utf8();
            }

            fn eat(&mut self, expected: char) -> bool {
                self.skip_ws();
                if self.peek() == Some(expected) {
                    self.bump(expected);
                    true
                } else {
                    false
                }
            }

            fn parse_value(&mut self) -> Option<MetaValue> {
                self.skip_ws();
                let ch = self.peek()?;

                match ch {
                    '\'' | '"' => self.parse_string().map(|s| MetaValue::Str(Arc::from(s))),
                    '[' => self.parse_list(),
                    '{' => self.parse_map(),
                    '(' => self.parse_parens(),
                    '-' | '0'..='9' => self.parse_int().map(MetaValue::Int),
                    _ => self.parse_ident_or_keyword(),
                }
            }

            fn parse_ident_or_keyword(&mut self) -> Option<MetaValue> {
                self.skip_ws();
                let start = self.i;
                while let Some(ch) = self.peek() {
                    if ch.is_ascii_alphanumeric() || ch == '_' {
                        self.bump(ch);
                    } else {
                        break;
                    }
                }
                if self.i == start {
                    return None;
                }
                let ident = &self.s[start..self.i];
                match ident {
                    "None" => Some(MetaValue::None),
                    "True" => Some(MetaValue::Bool(true)),
                    "False" => Some(MetaValue::Bool(false)),
                    _ => None,
                }
            }

            fn parse_int(&mut self) -> Option<i64> {
                self.skip_ws();
                let start = self.i;
                if self.peek() == Some('-') {
                    self.bump('-');
                }
                let mut saw_digit = false;
                while let Some(ch) = self.peek() {
                    if ch.is_ascii_digit() {
                        saw_digit = true;
                        self.bump(ch);
                    } else {
                        break;
                    }
                }
                if !saw_digit {
                    self.i = start;
                    return None;
                }
                self.s[start..self.i].parse::<i64>().ok()
            }

            fn parse_string(&mut self) -> Option<String> {
                self.skip_ws();
                let quote = self.peek()?;
                if quote != '\'' && quote != '"' {
                    return None;
                }
                self.bump(quote);
                let mut out = String::new();
                while let Some(ch) = self.peek() {
                    self.bump(ch);
                    if ch == quote {
                        return Some(out);
                    }
                    if ch == '\\' {
                        let esc = self.peek()?;
                        self.bump(esc);
                        match esc {
                            'n' => out.push('\n'),
                            'r' => out.push('\r'),
                            't' => out.push('\t'),
                            '\\' => out.push('\\'),
                            '\'' => out.push('\''),
                            '"' => out.push('"'),
                            other => out.push(other),
                        }
                    } else {
                        out.push(ch);
                    }
                }
                None
            }

            fn parse_list(&mut self) -> Option<MetaValue> {
                if !self.eat('[') {
                    return None;
                }
                let mut items = Vec::new();
                loop {
                    self.skip_ws();
                    if self.eat(']') {
                        break;
                    }
                    let value = self.parse_value()?;
                    items.push(value);
                    self.skip_ws();
                    if self.eat(']') {
                        break;
                    }
                    if !self.eat(',') {
                        return None;
                    }
                }
                Some(MetaValue::List(items))
            }

            fn parse_map(&mut self) -> Option<MetaValue> {
                if !self.eat('{') {
                    return None;
                }
                let mut map: BTreeMap<String, MetaValue> = BTreeMap::new();
                loop {
                    self.skip_ws();
                    if self.eat('}') {
                        break;
                    }
                    let key = match self.parse_value()? {
                        MetaValue::Str(s) => s.to_string(),
                        _ => return None,
                    };
                    if !self.eat(':') {
                        return None;
                    }
                    let value = self.parse_value()?;
                    map.insert(key, value);
                    self.skip_ws();
                    if self.eat('}') {
                        break;
                    }
                    if !self.eat(',') {
                        return None;
                    }
                }
                Some(MetaValue::Map(map))
            }

            fn parse_parens(&mut self) -> Option<MetaValue> {
                if !self.eat('(') {
                    return None;
                }
                self.skip_ws();
                if self.eat(')') {
                    return Some(MetaValue::Tuple(Vec::new()));
                }

                let first = self.parse_value()?;
                self.skip_ws();

                // If there's no comma, treat as grouping: `(x)` -> `x`
                if self.eat(')') {
                    return Some(first);
                }
                if !self.eat(',') {
                    return None;
                }

                let mut items = vec![first];
                loop {
                    self.skip_ws();
                    if self.eat(')') {
                        break;
                    }
                    let value = self.parse_value()?;
                    items.push(value);
                    self.skip_ws();
                    if self.eat(')') {
                        break;
                    }
                    if !self.eat(',') {
                        return None;
                    }
                }

                Some(MetaValue::Tuple(items))
            }
        }

        let mut p = Parser::new(input);
        let value = p.parse_value()?;
        p.skip_ws();
        if !p.is_eof() {
            return None;
        }
        Some(value)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_builder() {
        let style = Style::new().with_bold(true).with_color(Color::Standard(1));
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.color, Some(Color::Standard(1)));
    }

    #[test]
    fn test_style_combine() {
        let base = Style::new().with_bold(true);
        let overlay = Style::new().with_italic(true);
        let combined = base.combine(&overlay);
        assert_eq!(combined.bold, Some(true));
        assert_eq!(combined.italic, Some(true));
    }

    #[test]
    fn test_style_parse() {
        let style = Style::parse("bold red").unwrap();
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.color, Some(Color::Standard(1)));
    }

    #[test]
    fn test_style_parse_background() {
        let style = Style::parse("on blue").unwrap();
        assert_eq!(style.color, None);
        assert_eq!(style.bgcolor, Some(Color::Standard(4)));

        let style = Style::parse("bold red on blue").unwrap();
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.color, Some(Color::Standard(1)));
        assert_eq!(style.bgcolor, Some(Color::Standard(4)));
    }

    // --- NULL_STYLE tests ---

    #[test]
    fn test_null_style_is_default() {
        assert_eq!(NULL_STYLE, Style::default());
        assert!(NULL_STYLE.is_null());
    }

    #[test]
    fn test_null_style_all_none() {
        assert_eq!(NULL_STYLE.color, None);
        assert_eq!(NULL_STYLE.bgcolor, None);
        assert_eq!(NULL_STYLE.bold, None);
        assert_eq!(NULL_STYLE.dim, None);
        assert_eq!(NULL_STYLE.italic, None);
        assert_eq!(NULL_STYLE.underline, None);
        assert_eq!(NULL_STYLE.blink, None);
        assert_eq!(NULL_STYLE.reverse, None);
        assert_eq!(NULL_STYLE.strike, None);
    }

    #[test]
    fn test_is_null() {
        assert!(Style::new().is_null());
        assert!(!Style::new().with_bold(true).is_null());
        assert!(!Style::new().with_color(Color::Standard(1)).is_null());
    }

    // --- render() tests ---

    #[test]
    fn test_render_empty_text() {
        let style = Style::new().with_bold(true);
        assert_eq!(style.render("", ColorSystem::TrueColor), "");
    }

    #[test]
    fn test_render_null_style() {
        let style = Style::new();
        // Null style should return text without ANSI codes
        assert_eq!(style.render("Hello", ColorSystem::TrueColor), "Hello");
    }

    #[test]
    fn test_render_bold() {
        let style = Style::new().with_bold(true);
        let rendered = style.render("Hello", ColorSystem::TrueColor);
        assert_eq!(rendered, "\x1b[1mHello\x1b[0m");
    }

    #[test]
    fn test_render_multiple_attributes() {
        let style = Style::new().with_bold(true).with_italic(true);
        let rendered = style.render("Hello", ColorSystem::TrueColor);
        assert_eq!(rendered, "\x1b[1;3mHello\x1b[0m");
    }

    #[test]
    fn test_render_all_attributes() {
        let style = Style {
            color: None,
            bgcolor: None,
            bold: Some(true),
            dim: Some(true),
            italic: Some(true),
            underline: Some(true),
            blink: Some(true),
            reverse: Some(true),
            strike: Some(true),
        };
        let rendered = style.render("X", ColorSystem::TrueColor);
        // SGR codes: bold=1, dim=2, italic=3, underline=4, blink=5, reverse=7, strike=9
        assert_eq!(rendered, "\x1b[1;2;3;4;5;7;9mX\x1b[0m");
    }

    #[test]
    fn test_render_with_standard_color() {
        let style = Style::new().with_color(Color::Standard(1)); // red
        let rendered = style.render("Hi", ColorSystem::TrueColor);
        // Standard color 1 = red, foreground code = 31
        assert_eq!(rendered, "\x1b[31mHi\x1b[0m");
    }

    #[test]
    fn test_render_with_bright_color() {
        let style = Style::new().with_color(Color::Standard(9)); // bright red
        let rendered = style.render("Hi", ColorSystem::TrueColor);
        // Bright red, foreground code = 91
        assert_eq!(rendered, "\x1b[91mHi\x1b[0m");
    }

    #[test]
    fn test_render_with_256_color() {
        let style = Style::new().with_color(Color::EightBit(196));
        let rendered = style.render("Hi", ColorSystem::TrueColor);
        assert_eq!(rendered, "\x1b[38;5;196mHi\x1b[0m");
    }

    #[test]
    fn test_render_with_rgb_color() {
        let style = Style::new().with_color(Color::Rgb {
            r: 255,
            g: 128,
            b: 0,
        });
        let rendered = style.render("Hi", ColorSystem::TrueColor);
        assert_eq!(rendered, "\x1b[38;2;255;128;0mHi\x1b[0m");
    }

    #[test]
    fn test_render_with_bgcolor() {
        let style = Style::new().with_bgcolor(Color::Standard(4)); // blue bg
        let rendered = style.render("Hi", ColorSystem::TrueColor);
        // Blue background code = 44
        assert_eq!(rendered, "\x1b[44mHi\x1b[0m");
    }

    #[test]
    fn test_render_with_fg_and_bg() {
        let style = Style::new()
            .with_color(Color::Standard(1)) // red fg
            .with_bgcolor(Color::Standard(7)); // white bg
        let rendered = style.render("Hi", ColorSystem::TrueColor);
        assert_eq!(rendered, "\x1b[31;47mHi\x1b[0m");
    }

    #[test]
    fn test_render_bold_and_color() {
        let style = Style::new().with_bold(true).with_color(Color::Standard(2)); // green
        let rendered = style.render("OK", ColorSystem::TrueColor);
        assert_eq!(rendered, "\x1b[1;32mOK\x1b[0m");
    }

    #[test]
    fn test_render_color_downgrade_to_256() {
        // RGB color should be downgraded when using 256 color system
        let style = Style::new().with_color(Color::Rgb { r: 255, g: 0, b: 0 });
        let rendered = style.render("X", ColorSystem::EightBit);
        // Should contain 38;5;N format, not 38;2;R;G;B
        assert!(rendered.contains("38;5;"));
        assert!(!rendered.contains("38;2;"));
    }

    #[test]
    fn test_render_color_downgrade_to_standard() {
        // RGB color should be downgraded when using standard color system
        let style = Style::new().with_color(Color::Rgb { r: 255, g: 0, b: 0 });
        let rendered = style.render("X", ColorSystem::Standard);
        // Should contain simple code like 31 or 91, not extended format
        assert!(!rendered.contains("38;5;"));
        assert!(!rendered.contains("38;2;"));
    }

    // --- get_html_style() tests ---

    #[test]
    fn test_html_style_empty() {
        let style = Style::new();
        assert_eq!(style.get_html_style(), "");
    }

    #[test]
    fn test_html_style_bold() {
        let style = Style::new().with_bold(true);
        assert_eq!(style.get_html_style(), "font-weight: bold");
    }

    #[test]
    fn test_html_style_italic() {
        let style = Style::new().with_italic(true);
        assert_eq!(style.get_html_style(), "font-style: italic");
    }

    #[test]
    fn test_html_style_underline() {
        let style = Style::new().with_underline(true);
        assert_eq!(style.get_html_style(), "text-decoration: underline");
    }

    #[test]
    fn test_html_style_strike() {
        let style = Style::new().with_strike(true);
        assert_eq!(style.get_html_style(), "text-decoration: line-through");
    }

    #[test]
    fn test_with_reverse_builder_sets_flag() {
        let style = Style::new().with_reverse(true);
        assert_eq!(style.reverse, Some(true));
    }

    #[test]
    fn test_html_style_color_rgb() {
        let style = Style::new().with_color(Color::Rgb { r: 255, g: 0, b: 0 });
        let css = style.get_html_style();
        assert!(css.contains("color: #ff0000"));
        assert!(css.contains("text-decoration-color: #ff0000"));
    }

    #[test]
    fn test_html_style_bgcolor() {
        let style = Style::new().with_bgcolor(Color::Rgb { r: 0, g: 0, b: 255 });
        let css = style.get_html_style();
        assert!(css.contains("background-color: #0000ff"));
    }

    #[test]
    fn test_html_style_reverse_swaps_colors() {
        let style = Style {
            color: Some(Color::Rgb { r: 255, g: 0, b: 0 }),
            bgcolor: Some(Color::Rgb { r: 0, g: 0, b: 255 }),
            reverse: Some(true),
            ..Default::default()
        };
        let css = style.get_html_style();
        // After reverse, fg should be blue and bg should be red
        assert!(css.contains("color: #0000ff"));
        assert!(css.contains("background-color: #ff0000"));
    }

    #[test]
    fn test_html_style_combined() {
        let style = Style::new()
            .with_bold(true)
            .with_italic(true)
            .with_color(Color::Rgb {
                r: 255,
                g: 128,
                b: 0,
            });
        let css = style.get_html_style();
        assert!(css.contains("font-weight: bold"));
        assert!(css.contains("font-style: italic"));
        assert!(css.contains("color: #ff8000"));
    }

    #[test]
    fn test_html_style_standard_color() {
        // Standard color 1 = red
        let style = Style::new().with_color(Color::Standard(1));
        let css = style.get_html_style();
        // Should look up in palette and return hex
        assert!(css.contains("color: #"));
    }

    #[test]
    fn test_html_style_underline_and_strike_combined() {
        // Bug fix: underline + strike should combine into single text-decoration
        let style = Style::new().with_underline(true).with_strike(true);
        let css = style.get_html_style();
        // Should emit "text-decoration: underline line-through" (single property)
        assert!(css.contains("text-decoration: underline line-through"));
        // Should NOT have two separate text-decoration properties
        assert_eq!(css.matches("text-decoration").count(), 1);
    }

    // --- make_ansi_codes() tests ---

    #[test]
    fn test_make_ansi_codes_empty() {
        let style = Style::new();
        assert_eq!(style.make_ansi_codes(ColorSystem::TrueColor), "");
    }

    #[test]
    fn test_make_ansi_codes_attributes_only() {
        let style = Style::new().with_bold(true).with_dim(true);
        assert_eq!(style.make_ansi_codes(ColorSystem::TrueColor), "1;2");
    }

    #[test]
    fn test_make_ansi_codes_false_attributes_emit_reset() {
        // Explicitly false attributes should emit SGR reset codes before "on" codes
        let style = Style {
            bold: Some(false),
            italic: Some(true),
            ..Default::default()
        };
        // 22 = bold/dim off, 3 = italic on
        assert_eq!(style.make_ansi_codes(ColorSystem::TrueColor), "22;3");
    }

    // --- Bug fix tests ---

    #[test]
    fn test_parse_not_bold() {
        // Bug 1: "not bold" should set bold = Some(false)
        let style = Style::parse("not bold").unwrap();
        assert_eq!(style.bold, Some(false));
    }

    #[test]
    fn test_parse_not_italic() {
        let style = Style::parse("not italic").unwrap();
        assert_eq!(style.italic, Some(false));
    }

    #[test]
    fn test_parse_not_underline() {
        let style = Style::parse("not underline").unwrap();
        assert_eq!(style.underline, Some(false));
    }

    #[test]
    fn test_parse_not_dim() {
        let style = Style::parse("not dim").unwrap();
        assert_eq!(style.dim, Some(false));
    }

    #[test]
    fn test_parse_not_blink() {
        let style = Style::parse("not blink").unwrap();
        assert_eq!(style.blink, Some(false));
    }

    #[test]
    fn test_parse_not_reverse() {
        let style = Style::parse("not reverse").unwrap();
        assert_eq!(style.reverse, Some(false));
    }

    #[test]
    fn test_parse_not_strike() {
        let style = Style::parse("not strike").unwrap();
        assert_eq!(style.strike, Some(false));
    }

    #[test]
    fn test_parse_mixed_attributes_with_negation() {
        // "bold not italic red" should set bold=true, italic=false, color=red
        let style = Style::parse("bold not italic red").unwrap();
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.italic, Some(false));
        assert_eq!(style.color, Some(Color::Standard(1)));
    }

    #[test]
    fn test_make_ansi_codes_bold_false_emits_22() {
        // Bug 2: bold = Some(false) should emit SGR code 22
        let style = Style {
            bold: Some(false),
            ..Default::default()
        };
        assert_eq!(style.make_ansi_codes(ColorSystem::TrueColor), "22");
    }

    #[test]
    fn test_make_ansi_codes_italic_false_emits_23() {
        let style = Style {
            italic: Some(false),
            ..Default::default()
        };
        assert_eq!(style.make_ansi_codes(ColorSystem::TrueColor), "23");
    }

    #[test]
    fn test_make_ansi_codes_underline_false_emits_24() {
        let style = Style {
            underline: Some(false),
            ..Default::default()
        };
        assert_eq!(style.make_ansi_codes(ColorSystem::TrueColor), "24");
    }

    #[test]
    fn test_make_ansi_codes_blink_false_emits_25() {
        let style = Style {
            blink: Some(false),
            ..Default::default()
        };
        assert_eq!(style.make_ansi_codes(ColorSystem::TrueColor), "25");
    }

    #[test]
    fn test_make_ansi_codes_reverse_false_emits_27() {
        let style = Style {
            reverse: Some(false),
            ..Default::default()
        };
        assert_eq!(style.make_ansi_codes(ColorSystem::TrueColor), "27");
    }

    #[test]
    fn test_make_ansi_codes_strike_false_emits_29() {
        let style = Style {
            strike: Some(false),
            ..Default::default()
        };
        assert_eq!(style.make_ansi_codes(ColorSystem::TrueColor), "29");
    }

    #[test]
    fn test_make_ansi_codes_dim_false_emits_22() {
        // dim=false also uses 22 (same as bold off)
        let style = Style {
            dim: Some(false),
            ..Default::default()
        };
        assert_eq!(style.make_ansi_codes(ColorSystem::TrueColor), "22");
    }

    #[test]
    fn test_make_ansi_codes_bold_false_dim_true() {
        // Edge case: bold=false, dim=true should emit 22 (off), then 2 (dim on)
        let style = Style {
            bold: Some(false),
            dim: Some(true),
            ..Default::default()
        };
        assert_eq!(style.make_ansi_codes(ColorSystem::TrueColor), "22;2");
    }

    #[test]
    fn test_render_with_false_attribute() {
        let style = Style {
            bold: Some(false),
            ..Default::default()
        };
        let rendered = style.render("Hi", ColorSystem::TrueColor);
        assert_eq!(rendered, "\x1b[22mHi\x1b[0m");
    }

    // --- Shorthand aliases tests ---

    #[test]
    fn test_parse_shorthand_b_for_bold() {
        let style = Style::parse("b").unwrap();
        assert_eq!(style.bold, Some(true));
    }

    #[test]
    fn test_parse_shorthand_i_for_italic() {
        let style = Style::parse("i").unwrap();
        assert_eq!(style.italic, Some(true));
    }

    #[test]
    fn test_parse_shorthand_u_for_underline() {
        let style = Style::parse("u").unwrap();
        assert_eq!(style.underline, Some(true));
    }

    #[test]
    fn test_parse_shorthand_s_for_strike() {
        let style = Style::parse("s").unwrap();
        assert_eq!(style.strike, Some(true));
    }

    #[test]
    fn test_parse_shorthand_combined() {
        let style = Style::parse("b i red").unwrap();
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.italic, Some(true));
        assert_eq!(style.color, Some(Color::Standard(1)));
    }

    #[test]
    fn test_parse_shorthand_negation() {
        let style = Style::parse("not b").unwrap();
        assert_eq!(style.bold, Some(false));

        let style = Style::parse("not i").unwrap();
        assert_eq!(style.italic, Some(false));
    }
}
