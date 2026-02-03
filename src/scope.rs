//! Scope: render local variables as a formatted table.
//!
//! This module provides functionality to render variable scopes (like local
//! variables in a stack frame) as a formatted, styled table inside a panel.
//!
//! # Example
//!
//! ```
//! use std::collections::BTreeMap;
//! use rich_rs::scope::render_scope;
//!
//! let mut scope: BTreeMap<String, String> = BTreeMap::new();
//! scope.insert("foo".to_string(), "42".to_string());
//! scope.insert("bar".to_string(), r#""hello""#.to_string());
//!
//! let renderable = render_scope(&scope, Some("Locals"), true, false, None, None);
//! ```

use std::collections::BTreeMap;
use std::io::Stdout;

use crate::Renderable;
use crate::console::{Console, ConsoleOptions, JustifyMethod};
use crate::highlighter::{Highlighter, repr_highlighter};
use crate::measure::Measurement;
use crate::panel::Panel;
use crate::segment::Segments;
use crate::style::Style;
use crate::table::{Column, Row, Table};
use crate::text::Text;

// ============================================================================
// Styles
// ============================================================================

/// Style for regular variable names.
fn scope_key_style() -> Style {
    Style::new().with_color(crate::color::SimpleColor::Standard(6)) // cyan
}

/// Style for special variable names (dunder variables).
fn scope_key_special_style() -> Style {
    Style::new()
        .with_color(crate::color::SimpleColor::Standard(6))
        .with_dim(true) // cyan + dim
}

/// Style for the equals sign.
fn scope_equals_style() -> Style {
    Style::new().with_bold(true)
}

/// Style for the panel border.
fn scope_border_style() -> Style {
    Style::new().with_color(crate::color::SimpleColor::Standard(6)) // cyan
}

// ============================================================================
// ScopeRenderable
// ============================================================================

/// A renderable scope display.
///
/// Renders variable names and their values in a styled table within a panel.
pub struct ScopeRenderable {
    /// The scope data (variable name -> debug representation).
    scope: BTreeMap<String, String>,
    /// Optional title for the panel.
    title: Option<String>,
    /// Whether to sort keys alphabetically.
    sort_keys: bool,
    /// Whether to show indent guides in values.
    indent_guides: bool,
    /// Maximum length for container values before abbreviating.
    max_length: Option<usize>,
    /// Maximum string length before truncating.
    max_string: Option<usize>,
}

impl std::fmt::Debug for ScopeRenderable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScopeRenderable")
            .field("scope_len", &self.scope.len())
            .field("title", &self.title)
            .field("sort_keys", &self.sort_keys)
            .field("indent_guides", &self.indent_guides)
            .field("max_length", &self.max_length)
            .field("max_string", &self.max_string)
            .finish()
    }
}

impl ScopeRenderable {
    /// Create a new scope renderable.
    pub fn new(scope: BTreeMap<String, String>) -> Self {
        Self {
            scope,
            title: None,
            sort_keys: true,
            indent_guides: false,
            max_length: None,
            max_string: None,
        }
    }

    /// Set the title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set whether to sort keys.
    pub fn with_sort_keys(mut self, sort: bool) -> Self {
        self.sort_keys = sort;
        self
    }

    /// Set whether to show indent guides.
    pub fn with_indent_guides(mut self, guides: bool) -> Self {
        self.indent_guides = guides;
        self
    }

    /// Set maximum container length.
    pub fn with_max_length(mut self, max: Option<usize>) -> Self {
        self.max_length = max;
        self
    }

    /// Set maximum string length.
    pub fn with_max_string(mut self, max: Option<usize>) -> Self {
        self.max_string = max;
        self
    }

    /// Get sorted items (dunder vars first, then alphabetical).
    fn sorted_items(&self) -> Vec<(&String, &String)> {
        let mut items: Vec<_> = self.scope.iter().collect();

        if self.sort_keys {
            items.sort_by(|(a, _), (b, _)| {
                // Dunder variables sort first
                let a_dunder = a.starts_with("__");
                let b_dunder = b.starts_with("__");

                match (a_dunder, b_dunder) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.to_lowercase().cmp(&b.to_lowercase()),
                }
            });
        }

        items
    }
}

// SAFETY: ScopeRenderable is Send + Sync because all fields are Send + Sync.
unsafe impl Send for ScopeRenderable {}
unsafe impl Sync for ScopeRenderable {}

impl Renderable for ScopeRenderable {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        // Create a grid table (no borders, minimal padding)
        let mut table = Table::grid().with_padding(0, 1).with_expand(false);

        // Add columns: key (right-justified) and value
        let key_column = Column::new().justify(JustifyMethod::Right);
        table.add_column(key_column);
        table.add_column(Column::new());

        // Get highlighter for values
        let highlighter = repr_highlighter();

        // Add rows
        let items = self.sorted_items();
        for (key, value) in items {
            // Style the key
            let key_style = if key.starts_with("__") {
                scope_key_special_style()
            } else {
                scope_key_style()
            };

            // Build key text: "name ="
            let mut key_text = Text::styled(key, key_style);
            key_text.append(" =", Some(scope_equals_style()));

            // Highlight the value
            let mut value_text = Text::plain(value);
            highlighter.highlight(&mut value_text);

            // Truncate if needed
            if let Some(max_str) = self.max_string {
                if value_text.plain_text().len() > max_str {
                    value_text = value_text.truncate(
                        max_str,
                        crate::console::OverflowMethod::Ellipsis,
                        false,
                    );
                }
            }

            // Create row
            let cells: Vec<Box<dyn Renderable + Send + Sync>> =
                vec![Box::new(key_text), Box::new(value_text)];
            table.add_row(Row::new(cells));
        }

        // Wrap in a panel
        let mut panel = Panel::fit(Box::new(table))
            .with_border_style(scope_border_style())
            .with_padding((0, 1, 0, 1));

        if let Some(ref title) = self.title {
            panel = panel.with_title(title);
        }

        panel.render(console, options)
    }

    fn measure(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        // Delegate to render-then-measure (default behavior)
        Measurement::from_segments(&self.render(console, options))
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Render a scope (variable mapping) as a formatted panel.
///
/// # Arguments
///
/// * `scope` - A mapping of variable names to their debug representations.
/// * `title` - Optional title for the panel.
/// * `sort_keys` - Whether to sort keys (dunder variables first, then alphabetical).
/// * `indent_guides` - Whether to show indent guides in values (not yet implemented).
/// * `max_length` - Maximum container length before abbreviating (not yet implemented).
/// * `max_string` - Maximum string length before truncating.
///
/// # Returns
///
/// A `ScopeRenderable` that implements `Renderable`.
///
/// # Example
///
/// ```
/// use std::collections::BTreeMap;
/// use rich_rs::scope::render_scope;
///
/// let mut locals: BTreeMap<String, String> = BTreeMap::new();
/// locals.insert("x".to_string(), "42".to_string());
/// locals.insert("name".to_string(), r#""Alice""#.to_string());
///
/// let scope_panel = render_scope(&locals, Some("Locals"), true, false, None, None);
/// ```
pub fn render_scope(
    scope: &BTreeMap<String, String>,
    title: Option<&str>,
    sort_keys: bool,
    indent_guides: bool,
    max_length: Option<usize>,
    max_string: Option<usize>,
) -> ScopeRenderable {
    let mut renderable = ScopeRenderable::new(scope.clone())
        .with_sort_keys(sort_keys)
        .with_indent_guides(indent_guides)
        .with_max_length(max_length)
        .with_max_string(max_string);

    if let Some(t) = title {
        renderable = renderable.with_title(t);
    }

    renderable
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_scope_empty() {
        let scope: BTreeMap<String, String> = BTreeMap::new();
        let renderable = render_scope(&scope, None, true, false, None, None);

        let console = Console::new();
        let options = ConsoleOptions::default();
        let segments = renderable.render(&console, &options);

        // Should render even if empty
        assert!(!segments.is_empty());
    }

    #[test]
    fn test_render_scope_with_data() {
        let mut scope: BTreeMap<String, String> = BTreeMap::new();
        scope.insert("foo".to_string(), "42".to_string());
        scope.insert("bar".to_string(), r#""hello""#.to_string());

        let renderable = render_scope(&scope, Some("Locals"), true, false, None, None);

        let console = Console::new();
        let options = ConsoleOptions::default();
        let segments = renderable.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(output.contains("foo"));
        assert!(output.contains("bar"));
        assert!(output.contains("42"));
        assert!(output.contains("hello"));
        assert!(output.contains("Locals"));
    }

    #[test]
    fn test_render_scope_dunder_first() {
        let mut scope: BTreeMap<String, String> = BTreeMap::new();
        scope.insert("zebra".to_string(), "1".to_string());
        scope.insert("__name__".to_string(), r#""test""#.to_string());
        scope.insert("alpha".to_string(), "2".to_string());

        let renderable = ScopeRenderable::new(scope);
        let items = renderable.sorted_items();
        let keys: Vec<&str> = items.iter().map(|(k, _)| k.as_str()).collect();

        // Dunder should be first
        assert_eq!(keys[0], "__name__");
    }

    #[test]
    fn test_scope_renderable_builder() {
        let scope: BTreeMap<String, String> = BTreeMap::new();
        let renderable = ScopeRenderable::new(scope)
            .with_title("Test")
            .with_sort_keys(false)
            .with_indent_guides(true)
            .with_max_length(Some(10))
            .with_max_string(Some(80));

        assert_eq!(renderable.title, Some("Test".to_string()));
        assert!(!renderable.sort_keys);
        assert!(renderable.indent_guides);
        assert_eq!(renderable.max_length, Some(10));
        assert_eq!(renderable.max_string, Some(80));
    }

    #[test]
    fn test_scope_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<ScopeRenderable>();
        assert_sync::<ScopeRenderable>();
    }

    #[test]
    fn test_scope_debug() {
        let mut scope: BTreeMap<String, String> = BTreeMap::new();
        scope.insert("x".to_string(), "1".to_string());
        let renderable = ScopeRenderable::new(scope);
        let debug_str = format!("{:?}", renderable);
        assert!(debug_str.contains("ScopeRenderable"));
        assert!(debug_str.contains("scope_len"));
    }
}
