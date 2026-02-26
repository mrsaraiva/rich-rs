//! Columns: Display renderables in neat columns.
//!
//! Columns arranges a collection of renderables in a multi-column grid layout,
//! automatically calculating the optimal number of columns based on content widths.
//!
//! # Example
//!
//! ```
//! use rich_rs::columns::Columns;
//! use rich_rs::text::Text;
//!
//! let items: Vec<Box<dyn rich_rs::Renderable + Send + Sync>> = vec![
//!     Box::new(Text::plain("Item 1")),
//!     Box::new(Text::plain("Item 2")),
//!     Box::new(Text::plain("Item 3")),
//! ];
//! let columns = Columns::new(items);
//! ```

use std::collections::HashMap;
use std::io::Stdout;
use std::sync::Arc;

use crate::align::Align;
use crate::console::ConsoleOptions;
use crate::measure::Measurement;
use crate::padding::PaddingDimensions;
use crate::rule::AlignMethod;
use crate::segment::Segments;
use crate::table::{Column, Table};
use crate::text::Text;
use crate::{Console, Renderable};

// ============================================================================
// Columns
// ============================================================================

/// Display renderables in neat columns.
///
/// Columns automatically arranges a collection of renderables in a grid layout,
/// calculating the optimal number of columns based on available width and content sizes.
///
/// # Example
///
/// ```
/// use rich_rs::columns::Columns;
/// use rich_rs::text::Text;
///
/// let items: Vec<Box<dyn rich_rs::Renderable + Send + Sync>> = vec![
///     Box::new(Text::plain("apple")),
///     Box::new(Text::plain("banana")),
///     Box::new(Text::plain("cherry")),
///     Box::new(Text::plain("date")),
/// ];
/// let columns = Columns::new(items)
///     .with_expand(true)
///     .with_equal(true);
/// ```
pub struct Columns {
    /// The renderables to display in columns.
    renderables: Vec<Arc<dyn Renderable + Send + Sync>>,
    /// Optional fixed width for each column.
    width: Option<usize>,
    /// Padding around cells (top, right, bottom, left).
    padding: (usize, usize, usize, usize),
    /// Expand to fill available width.
    expand: bool,
    /// Arrange into equal-sized columns.
    equal: bool,
    /// Arrange items top-to-bottom instead of left-to-right.
    column_first: bool,
    /// Start columns from right side.
    right_to_left: bool,
    /// Optional alignment for cell contents.
    align: Option<AlignMethod>,
    /// Optional title for the columns.
    title: Option<Text>,
}

impl std::fmt::Debug for Columns {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Columns")
            .field("renderables_count", &self.renderables.len())
            .field("width", &self.width)
            .field("padding", &self.padding)
            .field("expand", &self.expand)
            .field("equal", &self.equal)
            .field("column_first", &self.column_first)
            .field("right_to_left", &self.right_to_left)
            .field("align", &self.align)
            .field("title", &self.title)
            .finish()
    }
}

impl Default for Columns {
    fn default() -> Self {
        Self {
            renderables: Vec::new(),
            width: None,
            padding: (0, 1, 0, 1), // Default: 1 space left/right
            expand: false,
            equal: false,
            column_first: false,
            right_to_left: false,
            align: None,
            title: None,
        }
    }
}

impl Columns {
    /// Create a new Columns layout with the given renderables.
    pub fn new(renderables: Vec<Box<dyn Renderable + Send + Sync>>) -> Self {
        Self {
            renderables: renderables.into_iter().map(Arc::from).collect(),
            ..Default::default()
        }
    }

    /// Create an empty Columns layout.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Add a renderable to the columns.
    pub fn add(&mut self, renderable: Box<dyn Renderable + Send + Sync>) {
        self.renderables.push(Arc::from(renderable));
    }

    /// Add a string as a renderable.
    pub fn add_str(&mut self, text: &str) {
        self.renderables.push(Arc::new(Text::plain(text)));
    }

    /// Set fixed width for each column.
    pub fn with_width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    /// Set padding around cells.
    ///
    /// Accepts CSS-style padding: 1 value (all), 2 values (vertical, horizontal),
    /// or 4 values (top, right, bottom, left).
    pub fn with_padding(mut self, padding: impl Into<PaddingDimensions>) -> Self {
        self.padding = padding.into().unpack();
        self
    }

    /// Set whether to expand to fill available width.
    pub fn with_expand(mut self, expand: bool) -> Self {
        self.expand = expand;
        self
    }

    /// Set whether to use equal-sized columns.
    pub fn with_equal(mut self, equal: bool) -> Self {
        self.equal = equal;
        self
    }

    /// Set whether to arrange items top-to-bottom (column-first) instead of left-to-right.
    pub fn with_column_first(mut self, column_first: bool) -> Self {
        self.column_first = column_first;
        self
    }

    /// Set whether to arrange columns from right to left.
    pub fn with_right_to_left(mut self, right_to_left: bool) -> Self {
        self.right_to_left = right_to_left;
        self
    }

    /// Set alignment for cell contents.
    pub fn with_align(mut self, align: AlignMethod) -> Self {
        self.align = Some(align);
        self
    }

    /// Set a title for the columns.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(Text::plain(&title.into()));
        self
    }

    /// Calculate optimal column count and build the table.
    fn build_table(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Table {
        let (top, right, bottom, left) = self.padding;
        let width_padding = left.max(right);
        let max_width = options.max_width;

        // Measure all renderables
        let mut renderable_widths: Vec<usize> = self
            .renderables
            .iter()
            .map(|r| console.measure(r.as_ref(), Some(options)).maximum)
            .collect();

        // If equal mode, use max width for all
        if self.equal && !renderable_widths.is_empty() {
            let max_w = *renderable_widths.iter().max().unwrap_or(&0);
            renderable_widths = vec![max_w; renderable_widths.len()];
        }

        let item_count = self.renderables.len();
        let mut column_count = item_count.max(1);

        // Create the grid table
        let mut table = Table::grid()
            .with_collapse_padding(true)
            .with_pad_edge(false);

        // Apply padding (Table uses left, right only for cells)
        table = table.with_padding(left, right);
        table = table.with_leading(top.saturating_add(bottom));

        // Apply expand setting
        table = table.with_expand(self.expand);

        if let Some(ref title) = self.title {
            table = table.with_title(&title.plain_text());
        }

        // Determine column count
        if let Some(col_width) = self.width {
            // Fixed width mode: calculate how many columns fit
            column_count = max_width / (col_width + width_padding);
            column_count = column_count.max(1);
            for _ in 0..column_count {
                let mut col = Column::new();
                col.width = Some(col_width);
                table.add_column(col);
            }
        } else {
            // Auto mode: find optimal column count
            while column_count > 1 {
                let mut widths: HashMap<usize, usize> = HashMap::new();
                let mut column_no = 0;
                let mut fits = true;

                for idx in self.iter_indices(column_count) {
                    let w = idx
                        .and_then(|i| renderable_widths.get(i).copied())
                        .unwrap_or(0);
                    let entry = widths.entry(column_no).or_insert(0);
                    *entry = (*entry).max(w);

                    let total_width: usize = widths.values().sum::<usize>()
                        + width_padding * widths.len().saturating_sub(1);

                    if total_width > max_width {
                        column_count = widths.len().saturating_sub(1).max(1);
                        fits = false;
                        break;
                    }

                    column_no = (column_no + 1) % column_count;
                }

                // If we didn't break, we found a valid column count
                if fits {
                    break;
                }
            }

            // Add columns without fixed width
            for _ in 0..column_count {
                table.add_column_str("");
            }
        }

        // Build rows
        let indices: Vec<Option<usize>> = self.iter_indices(column_count).collect();

        // Add padding for incomplete last row
        let padded_len = if item_count % column_count != 0 {
            item_count + (column_count - item_count % column_count)
        } else if item_count == 0 {
            0
        } else {
            item_count
        };

        for start in (0..padded_len).step_by(column_count) {
            let mut row_items: Vec<Box<dyn Renderable + Send + Sync>> = Vec::new();

            for i in start..start + column_count {
                let idx = indices.get(i).copied().flatten();
                let renderable_idx = idx.filter(|&i| i < self.renderables.len());

                if let Some(ridx) = renderable_idx {
                    // Keep the renderable intact so table layout can measure and render it
                    // with the correct per-cell constraints (instead of flattening it to text).
                    let renderable = self.renderables[ridx].clone();

                    // Wrap in alignment if specified
                    let cell: Box<dyn Renderable + Send + Sync> = if let Some(align) = self.align {
                        let width = if self.equal {
                            renderable_widths.first().copied()
                        } else {
                            None
                        };
                        if let Some(w) = width {
                            Box::new(
                                Align::new(Box::new(ArcRenderable::new(renderable)), align)
                                    .with_width(w),
                            )
                        } else {
                            Box::new(Align::new(Box::new(ArcRenderable::new(renderable)), align))
                        }
                    } else if self.equal {
                        // Constrain to equal width
                        let width = renderable_widths.first().copied().unwrap_or(0);
                        Box::new(
                            Align::new(Box::new(ArcRenderable::new(renderable)), AlignMethod::Left)
                                .with_width(width),
                        )
                    } else {
                        Box::new(ArcRenderable::new(renderable))
                    };
                    row_items.push(cell);
                } else {
                    row_items.push(Box::new(Text::plain("")));
                }
            }

            // Reverse for right-to-left
            if self.right_to_left {
                row_items.reverse();
            }

            // Add row to table
            table.add_row(crate::table::Row::new(row_items));
        }

        table
    }

    /// Iterator over indices in display order (row by row, left to right).
    /// Returns the item index that should appear at each display cell.
    fn iter_indices(&self, column_count: usize) -> impl Iterator<Item = Option<usize>> + '_ {
        let item_count = self.renderables.len();
        let column_first = self.column_first;

        let row_count = if column_count > 0 {
            (item_count + column_count - 1) / column_count
        } else {
            0
        };
        let total_cells = row_count * column_count;

        (0..total_cells).map(move |cell_idx| {
            // cell_idx is in display order: row 0 col 0, row 0 col 1, ...
            let row = cell_idx / column_count;
            let col = cell_idx % column_count;

            if column_first && column_count > 0 {
                // Column-first: items fill columns top-to-bottom, then left-to-right
                // Calculate column lengths: first `extra` columns have (base+1) items
                let base_per_col = item_count / column_count;
                let extra = item_count % column_count;

                // This column's length
                let col_length = if col < extra {
                    base_per_col + 1
                } else {
                    base_per_col
                };

                if row >= col_length {
                    None // This cell is empty
                } else {
                    // Calculate item index: sum of items in previous columns + row
                    let items_before = if col <= extra {
                        col * (base_per_col + 1)
                    } else {
                        extra * (base_per_col + 1) + (col - extra) * base_per_col
                    };
                    let idx = items_before + row;
                    if idx < item_count { Some(idx) } else { None }
                }
            } else {
                // Row-first: simple linear mapping
                if cell_idx < item_count {
                    Some(cell_idx)
                } else {
                    None
                }
            }
        })
    }
}

#[derive(Clone)]
struct ArcRenderable {
    inner: Arc<dyn Renderable + Send + Sync>,
}

impl ArcRenderable {
    fn new(inner: Arc<dyn Renderable + Send + Sync>) -> Self {
        Self { inner }
    }
}

impl Renderable for ArcRenderable {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        self.inner.render(console, options)
    }

    fn measure(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        self.inner.measure(console, options)
    }
}

impl Renderable for Columns {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        if self.renderables.is_empty() {
            return Segments::new();
        }

        let table = self.build_table(console, options);
        table.render(console, options)
    }

    fn measure(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        if self.renderables.is_empty() {
            return Measurement::new(0, 0);
        }

        let table = self.build_table(console, options);
        table.measure(console, options)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_console(width: usize) -> Console<Stdout> {
        Console::with_options(ConsoleOptions {
            max_width: width,
            ..Default::default()
        })
    }

    #[test]
    fn test_columns_empty() {
        let columns = Columns::empty();
        let console = make_console(80);
        let options = console.options().clone();
        let segments = columns.render(&console, &options);
        assert!(segments.is_empty());
    }

    #[test]
    fn test_columns_single_item() {
        let items: Vec<Box<dyn Renderable + Send + Sync>> = vec![Box::new(Text::plain("Hello"))];
        let columns = Columns::new(items);
        let console = make_console(80);
        let options = console.options().clone();
        let segments = columns.render(&console, &options);
        let text: String = segments.iter().map(|s| s.text.to_string()).collect();
        assert!(text.contains("Hello"));
    }

    #[test]
    fn test_columns_multiple_items() {
        let items: Vec<Box<dyn Renderable + Send + Sync>> = vec![
            Box::new(Text::plain("A")),
            Box::new(Text::plain("B")),
            Box::new(Text::plain("C")),
            Box::new(Text::plain("D")),
        ];
        let columns = Columns::new(items);
        let console = make_console(80);
        let options = console.options().clone();
        let segments = columns.render(&console, &options);
        let text: String = segments.iter().map(|s| s.text.to_string()).collect();
        assert!(text.contains("A"));
        assert!(text.contains("B"));
        assert!(text.contains("C"));
        assert!(text.contains("D"));
    }

    #[test]
    fn test_columns_add() {
        let mut columns = Columns::empty();
        columns.add(Box::new(Text::plain("Item 1")));
        columns.add_str("Item 2");
        assert_eq!(columns.renderables.len(), 2);
    }

    #[test]
    fn test_columns_with_expand() {
        let items: Vec<Box<dyn Renderable + Send + Sync>> =
            vec![Box::new(Text::plain("A")), Box::new(Text::plain("B"))];
        let columns = Columns::new(items).with_expand(true);
        assert!(columns.expand);
    }

    #[test]
    fn test_columns_with_equal() {
        let items: Vec<Box<dyn Renderable + Send + Sync>> = vec![
            Box::new(Text::plain("Short")),
            Box::new(Text::plain("Much Longer Text")),
        ];
        let columns = Columns::new(items).with_equal(true);
        assert!(columns.equal);
    }

    #[test]
    fn test_columns_with_title() {
        let items: Vec<Box<dyn Renderable + Send + Sync>> = vec![Box::new(Text::plain("A"))];
        let columns = Columns::new(items).with_title("My Title");
        assert!(columns.title.is_some());
    }

    #[test]
    fn test_columns_narrow_width() {
        // With very narrow width, should reduce to 1 column
        let items: Vec<Box<dyn Renderable + Send + Sync>> = vec![
            Box::new(Text::plain("Hello World")),
            Box::new(Text::plain("Goodbye World")),
        ];
        let columns = Columns::new(items);
        let console = make_console(15);
        let options = console.options().clone();
        let segments = columns.render(&console, &options);
        let text: String = segments.iter().map(|s| s.text.to_string()).collect();
        // Both items should be present
        assert!(text.contains("Hello"));
        assert!(text.contains("Goodbye"));
    }
}
