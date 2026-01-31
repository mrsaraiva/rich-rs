//! Table: a renderable table with columns and rows.
//!
//! Table displays data in a grid with optional headers, footers, and borders.
//!
//! # Example
//!
//! ```
//! use rich_rs::Table;
//!
//! let mut table = Table::new();
//! table.add_column_str("Name");
//! table.add_column_str("Age");
//! table.add_row_strs(&["Alice", "30"]);
//! table.add_row_strs(&["Bob", "25"]);
//! ```

use std::io::Stdout;

use crate::align::VerticalAlignMethod;
use crate::console::{ConsoleOptions, JustifyMethod, OverflowMethod};
use crate::measure::Measurement;
use crate::r#box::{Box as RichBox, RowLevel, HEAVY_HEAD};
use crate::rule::AlignMethod;
use crate::segment::{Segment, Segments};
use crate::style::Style;
use crate::text::Text;
use crate::{Console, Renderable};

// ============================================================================
// Column
// ============================================================================

/// A column definition within a table.
///
/// Columns define how data in a particular column should be displayed,
/// including headers, footers, styling, and width constraints.
///
/// # Note
///
/// The `justify`, `vertical`, `overflow`, and `no_wrap` fields are defined for API
/// compatibility but are not yet fully implemented. They will be applied in a future
/// version. Currently, cell content is rendered with default justification and wrapping.
pub struct Column {
    /// Column header content.
    pub header: Option<Box<dyn Renderable + Send + Sync>>,
    /// Column footer content.
    pub footer: Option<Box<dyn Renderable + Send + Sync>>,
    /// Style for the header.
    pub header_style: Style,
    /// Style for the footer.
    pub footer_style: Style,
    /// Style for column cells.
    pub style: Style,
    /// Horizontal alignment for cell content.
    pub justify: JustifyMethod,
    /// Vertical alignment for cell content.
    pub vertical: VerticalAlignMethod,
    /// Overflow handling method.
    pub overflow: OverflowMethod,
    /// Fixed width (if set, overrides auto-width).
    pub width: Option<usize>,
    /// Minimum width constraint.
    pub min_width: Option<usize>,
    /// Maximum width constraint.
    pub max_width: Option<usize>,
    /// Flexible width ratio (for distributing extra space).
    pub ratio: Option<usize>,
    /// Prevent text wrapping in this column.
    pub no_wrap: bool,
    /// Internal index (set when added to table).
    _index: usize,
}

impl std::fmt::Debug for Column {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Column")
            .field("header_style", &self.header_style)
            .field("footer_style", &self.footer_style)
            .field("style", &self.style)
            .field("justify", &self.justify)
            .field("vertical", &self.vertical)
            .field("overflow", &self.overflow)
            .field("width", &self.width)
            .field("min_width", &self.min_width)
            .field("max_width", &self.max_width)
            .field("ratio", &self.ratio)
            .field("no_wrap", &self.no_wrap)
            .finish_non_exhaustive()
    }
}

impl Default for Column {
    fn default() -> Self {
        Column {
            header: None,
            footer: None,
            header_style: Style::default(),
            footer_style: Style::default(),
            style: Style::default(),
            justify: JustifyMethod::Left,
            vertical: VerticalAlignMethod::Top,
            overflow: OverflowMethod::Ellipsis,
            width: None,
            min_width: None,
            max_width: None,
            ratio: None,
            no_wrap: false,
            _index: 0,
        }
    }
}

impl Column {
    /// Create a new column with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a column with a string header.
    pub fn with_header_str(header: &str) -> Self {
        Column {
            header: Some(Box::new(Text::plain(header))),
            ..Default::default()
        }
    }

    /// Create a column with a renderable header.
    pub fn with_header(header: Box<dyn Renderable + Send + Sync>) -> Self {
        Column {
            header: Some(header),
            ..Default::default()
        }
    }

    /// Set the header style.
    pub fn header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }

    /// Set the footer style.
    pub fn footer_style(mut self, style: Style) -> Self {
        self.footer_style = style;
        self
    }

    /// Set the cell style.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the horizontal alignment.
    pub fn justify(mut self, justify: JustifyMethod) -> Self {
        self.justify = justify;
        self
    }

    /// Set the vertical alignment.
    pub fn vertical(mut self, vertical: VerticalAlignMethod) -> Self {
        self.vertical = vertical;
        self
    }

    /// Set a fixed width.
    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the minimum width.
    pub fn min_width(mut self, width: usize) -> Self {
        self.min_width = Some(width);
        self
    }

    /// Set the maximum width.
    pub fn max_width(mut self, width: usize) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Set the ratio for flexible width distribution.
    pub fn ratio(mut self, ratio: usize) -> Self {
        self.ratio = Some(ratio);
        self
    }

    /// Set no_wrap mode.
    pub fn no_wrap(mut self, no_wrap: bool) -> Self {
        self.no_wrap = no_wrap;
        self
    }

    /// Check if this column is flexible (has a ratio set).
    pub fn flexible(&self) -> bool {
        self.ratio.is_some()
    }
}

// ============================================================================
// Row
// ============================================================================

/// A row within a table.
///
/// Rows contain cells and optional style overrides.
pub struct Row {
    /// Cells in this row (one per column).
    pub cells: Vec<Box<dyn Renderable + Send + Sync>>,
    /// Optional style override for the entire row.
    pub style: Option<Style>,
    /// Draw a section separator after this row.
    pub end_section: bool,
}

impl std::fmt::Debug for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Row")
            .field("cells_count", &self.cells.len())
            .field("style", &self.style)
            .field("end_section", &self.end_section)
            .finish()
    }
}

impl Row {
    /// Create a new row with the given cells.
    pub fn new(cells: Vec<Box<dyn Renderable + Send + Sync>>) -> Self {
        Row {
            cells,
            style: None,
            end_section: false,
        }
    }

    /// Create an empty row.
    pub fn empty() -> Self {
        Row {
            cells: Vec::new(),
            style: None,
            end_section: false,
        }
    }

    /// Set the row style.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Set end_section to draw a separator after this row.
    pub fn with_end_section(mut self, end_section: bool) -> Self {
        self.end_section = end_section;
        self
    }
}

// ============================================================================
// Table
// ============================================================================

/// A console-renderable table with rows and columns.
///
/// Table supports headers, footers, various border styles, and flexible
/// column width calculation.
///
/// # Example
///
/// ```
/// use rich_rs::Table;
///
/// let mut table = Table::new();
/// table.add_column_str("Name");
/// table.add_column_str("Score");
/// table.add_row_strs(&["Alice", "100"]);
/// table.add_row_strs(&["Bob", "95"]);
/// ```
pub struct Table {
    /// Column definitions.
    columns: Vec<Column>,
    /// Data rows (cells stored in columns for measurement efficiency).
    rows: Vec<Row>,
    /// Border style (None = no borders).
    box_type: Option<RichBox>,
    /// Use ASCII-safe box characters (None = use console default).
    safe_box: Option<bool>,
    /// Cell padding (left, right).
    padding: (usize, usize),
    /// Collapse padding between adjacent cells.
    collapse_padding: bool,
    /// Pad the edge cells.
    pad_edge: bool,
    /// Expand table to fill available width.
    expand: bool,
    /// Show header row.
    show_header: bool,
    /// Show footer row.
    show_footer: bool,
    /// Show outer edge (border).
    show_edge: bool,
    /// Show lines between all rows.
    show_lines: bool,
    /// Number of blank lines between rows (alternative to show_lines).
    leading: usize,
    /// Base style for the table.
    style: Style,
    /// Alternating row styles.
    row_styles: Vec<Style>,
    /// Style for the header row.
    header_style: Style,
    /// Style for the footer row.
    footer_style: Style,
    /// Style for borders.
    border_style: Style,
    /// Optional title above the table.
    title: Option<Text>,
    /// Optional caption below the table.
    caption: Option<Text>,
    /// Title alignment.
    title_align: AlignMethod,
    /// Caption alignment.
    caption_align: AlignMethod,
    /// Fixed width (None = auto).
    width: Option<usize>,
    /// Minimum width.
    min_width: Option<usize>,
    /// Enable highlighting of cell contents.
    highlight: bool,
}

impl std::fmt::Debug for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Table")
            .field("columns", &self.columns.len())
            .field("rows", &self.rows.len())
            .field("box_type", &self.box_type)
            .field("show_header", &self.show_header)
            .field("show_footer", &self.show_footer)
            .field("show_edge", &self.show_edge)
            .field("expand", &self.expand)
            .field("width", &self.width)
            .finish_non_exhaustive()
    }
}

impl Default for Table {
    fn default() -> Self {
        Table::new()
    }
}

impl Table {
    /// Create a new empty table with default settings.
    ///
    /// The default table uses HEAVY_HEAD border style, shows headers,
    /// and has standard padding.
    pub fn new() -> Self {
        Table {
            columns: Vec::new(),
            rows: Vec::new(),
            box_type: Some(HEAVY_HEAD),
            safe_box: None,
            padding: (1, 1),
            collapse_padding: false,
            pad_edge: true,
            expand: false,
            show_header: true,
            show_footer: false,
            show_edge: true,
            show_lines: false,
            leading: 0,
            style: Style::default(),
            row_styles: Vec::new(),
            header_style: Style::new().with_bold(true),
            footer_style: Style::default(),
            border_style: Style::default(),
            title: None,
            caption: None,
            title_align: AlignMethod::Center,
            caption_align: AlignMethod::Center,
            width: None,
            min_width: None,
            highlight: false,
        }
    }

    /// Create a grid (table with no borders or header).
    ///
    /// A grid is useful for simple layouts without table decoration.
    pub fn grid() -> Self {
        Table {
            box_type: None,
            padding: (0, 0),
            collapse_padding: true,
            pad_edge: false,
            show_header: false,
            show_footer: false,
            show_edge: false,
            ..Table::new()
        }
    }

    // ========================================================================
    // Builder methods
    // ========================================================================

    /// Set the border style.
    pub fn with_box(mut self, box_type: Option<RichBox>) -> Self {
        self.box_type = box_type;
        self
    }

    /// Set whether to use ASCII-safe box characters.
    pub fn with_safe_box(mut self, safe: bool) -> Self {
        self.safe_box = Some(safe);
        self
    }

    /// Set cell padding (left and right).
    pub fn with_padding(mut self, left: usize, right: usize) -> Self {
        self.padding = (left, right);
        self
    }

    /// Set whether to collapse padding between cells.
    pub fn with_collapse_padding(mut self, collapse: bool) -> Self {
        self.collapse_padding = collapse;
        self
    }

    /// Set whether to pad edge cells.
    pub fn with_pad_edge(mut self, pad: bool) -> Self {
        self.pad_edge = pad;
        self
    }

    /// Set whether to expand to fill available width.
    pub fn with_expand(mut self, expand: bool) -> Self {
        self.expand = expand;
        self
    }

    /// Set whether to show the header row.
    pub fn with_show_header(mut self, show: bool) -> Self {
        self.show_header = show;
        self
    }

    /// Set whether to show the footer row.
    pub fn with_show_footer(mut self, show: bool) -> Self {
        self.show_footer = show;
        self
    }

    /// Set whether to show the outer edge.
    pub fn with_show_edge(mut self, show: bool) -> Self {
        self.show_edge = show;
        self
    }

    /// Set whether to show lines between all rows.
    pub fn with_show_lines(mut self, show: bool) -> Self {
        self.show_lines = show;
        self
    }

    /// Set number of blank lines between rows.
    pub fn with_leading(mut self, leading: usize) -> Self {
        self.leading = leading;
        self
    }

    /// Set the base table style.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set alternating row styles.
    pub fn with_row_styles(mut self, styles: Vec<Style>) -> Self {
        self.row_styles = styles;
        self
    }

    /// Set the header style.
    pub fn with_header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }

    /// Set the footer style.
    pub fn with_footer_style(mut self, style: Style) -> Self {
        self.footer_style = style;
        self
    }

    /// Set the border style.
    pub fn with_border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    /// Set the title above the table.
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(Text::plain(title));
        self
    }

    /// Set the title with a Text object.
    pub fn with_title_text(mut self, title: Text) -> Self {
        self.title = Some(title);
        self
    }

    /// Set the caption below the table.
    pub fn with_caption(mut self, caption: &str) -> Self {
        self.caption = Some(Text::plain(caption));
        self
    }

    /// Set the caption with a Text object.
    pub fn with_caption_text(mut self, caption: Text) -> Self {
        self.caption = Some(caption);
        self
    }

    /// Set title alignment.
    pub fn with_title_align(mut self, align: AlignMethod) -> Self {
        self.title_align = align;
        self
    }

    /// Set caption alignment.
    pub fn with_caption_align(mut self, align: AlignMethod) -> Self {
        self.caption_align = align;
        self
    }

    /// Set a fixed width for the table.
    pub fn with_width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the minimum width.
    pub fn with_min_width(mut self, width: usize) -> Self {
        self.min_width = Some(width);
        self
    }

    /// Set whether to highlight cell contents.
    pub fn with_highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    // ========================================================================
    // Mutation methods
    // ========================================================================

    /// Add a column to the table.
    pub fn add_column(&mut self, mut column: Column) {
        column._index = self.columns.len();
        self.columns.push(column);
    }

    /// Add a column with a string header.
    pub fn add_column_str(&mut self, header: &str) {
        self.add_column(Column::with_header_str(header));
    }

    /// Add a column with a renderable header.
    pub fn add_column_renderable(&mut self, header: Box<dyn Renderable + Send + Sync>) {
        self.add_column(Column::with_header(header));
    }

    /// Add a row of cells to the table.
    pub fn add_row(&mut self, row: Row) {
        // Auto-create columns if needed
        while self.columns.len() < row.cells.len() {
            let mut col = Column::default();
            col._index = self.columns.len();
            self.columns.push(col);
        }
        self.rows.push(row);
    }

    /// Add a row of string cells.
    pub fn add_row_strs(&mut self, cells: &[&str]) {
        let cell_boxes: Vec<Box<dyn Renderable + Send + Sync>> = cells
            .iter()
            .map(|s| Box::new(Text::plain(*s)) as Box<dyn Renderable + Send + Sync>)
            .collect();
        self.add_row(Row::new(cell_boxes));
    }

    /// Add a row of renderable cells.
    pub fn add_row_renderables(&mut self, cells: Vec<Box<dyn Renderable + Send + Sync>>) {
        self.add_row(Row::new(cells));
    }

    /// Mark the last row as an end-of-section (draws separator after).
    pub fn add_section(&mut self) {
        if let Some(row) = self.rows.last_mut() {
            row.end_section = true;
        }
    }

    /// Get the number of rows (excluding header/footer).
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Get the number of columns.
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    // ========================================================================
    // Internal helpers
    // ========================================================================

    /// Calculate extra width from borders and dividers.
    fn extra_width(&self) -> usize {
        let mut width = 0;
        if self.box_type.is_some() && self.show_edge {
            width += 2; // Left and right edges
        }
        if self.box_type.is_some() && self.columns.len() > 1 {
            width += self.columns.len() - 1; // Dividers between columns
        }
        width
    }

    /// Get padding width for a column.
    /// Get the total padding width for a column (left + right).
    fn get_padding_width(&self, column_index: usize) -> usize {
        let (left, right) = self.get_padding_for_column(column_index);
        left + right
    }

    /// Get the actual (left, right) padding for a specific column,
    /// accounting for collapse_padding and pad_edge settings.
    fn get_padding_for_column(&self, column_index: usize) -> (usize, usize) {
        let (pad_left, pad_right) = self.padding;
        let num_columns = self.columns.len();
        let is_first = column_index == 0;
        let is_last = column_index == num_columns.saturating_sub(1);

        let mut left = pad_left;
        let mut right = pad_right;

        // Collapse padding between columns (avoid double padding)
        if self.collapse_padding && !is_first {
            left = 0; // Left padding already provided by previous column's right padding
        }

        // Don't pad edges if pad_edge is false
        if !self.pad_edge {
            if is_first {
                left = 0;
            }
            if is_last {
                right = 0;
            }
        }

        (left, right)
    }

    /// Get the row style for a given row index.
    fn get_row_style(&self, index: usize) -> Style {
        let mut style = Style::default();
        if !self.row_styles.is_empty() {
            style = style.combine(&self.row_styles[index % self.row_styles.len()]);
        }
        if let Some(row_style) = self.rows.get(index).and_then(|r| r.style) {
            style = style.combine(&row_style);
        }
        style
    }

    /// Measure a column to determine its min/max width.
    fn measure_column(
        &self,
        console: &Console<Stdout>,
        options: &ConsoleOptions,
        column: &Column,
    ) -> Measurement {
        let max_width = options.max_width;
        if max_width < 1 {
            return Measurement::new(0, 0);
        }

        let padding_width = self.get_padding_width(column._index);

        // Fixed width column
        if let Some(w) = column.width {
            return Measurement::new(w + padding_width, w + padding_width)
                .with_maximum(max_width);
        }

        // Measure all cells in this column
        let mut min_widths: Vec<usize> = Vec::new();
        let mut max_widths: Vec<usize> = Vec::new();

        // Measure header
        if self.show_header {
            if let Some(ref header) = column.header {
                let m = header.measure(console, options);
                min_widths.push(m.minimum);
                max_widths.push(m.maximum);
            }
        }

        // Measure data cells
        for row in &self.rows {
            if let Some(cell) = row.cells.get(column._index) {
                let m = cell.measure(console, options);
                min_widths.push(m.minimum);
                max_widths.push(m.maximum);
            }
        }

        // Measure footer
        if self.show_footer {
            if let Some(ref footer) = column.footer {
                let m = footer.measure(console, options);
                min_widths.push(m.minimum);
                max_widths.push(m.maximum);
            }
        }

        let min_w = min_widths.iter().max().copied().unwrap_or(1) + padding_width;
        let max_w = max_widths.iter().max().copied().unwrap_or(max_width) + padding_width;

        Measurement::new(min_w, max_w)
            .with_maximum(max_width)
            .clamp_bounds(
                column.min_width.map(|w| w + padding_width),
                column.max_width.map(|w| w + padding_width),
            )
    }

    /// Calculate column widths based on content and constraints.
    fn calculate_column_widths(
        &self,
        console: &Console<Stdout>,
        options: &ConsoleOptions,
    ) -> Vec<usize> {
        if self.columns.is_empty() {
            return Vec::new();
        }

        let max_width = options.max_width;
        let extra = self.extra_width();

        // Measure each column
        let measurements: Vec<Measurement> = self
            .columns
            .iter()
            .map(|col| self.measure_column(console, options, col))
            .collect();

        let mut widths: Vec<usize> = measurements.iter().map(|m| m.maximum).collect();

        // Handle flexible columns with ratios
        if self.expand || self.width.is_some() {
            let ratios: Vec<usize> = self
                .columns
                .iter()
                .filter(|c| c.flexible())
                .map(|c| c.ratio.unwrap_or(0))
                .collect();

            if !ratios.is_empty() && ratios.iter().any(|&r| r > 0) {
                let fixed_width: usize = self
                    .columns
                    .iter()
                    .zip(measurements.iter())
                    .filter(|(c, _)| !c.flexible())
                    .map(|(_, m)| m.maximum)
                    .sum();

                let target_width = self.width.unwrap_or(max_width).saturating_sub(extra);
                let flexible_width = target_width.saturating_sub(fixed_width);
                let total_ratio: usize = ratios.iter().sum();

                if total_ratio > 0 {
                    let mut ratio_idx = 0;
                    for (i, col) in self.columns.iter().enumerate() {
                        if col.flexible() {
                            let ratio = col.ratio.unwrap_or(0);
                            let col_width = (flexible_width * ratio) / total_ratio;
                            let padding = self.get_padding_width(i);
                            widths[i] = col_width.max(padding + 1);
                            ratio_idx += 1;
                        }
                    }
                    // Distribute remainder
                    let assigned: usize = self
                        .columns
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| c.flexible())
                        .map(|(i, _)| widths[i])
                        .sum();
                    let remainder = flexible_width.saturating_sub(assigned);
                    if remainder > 0 && ratio_idx > 0 {
                        // Give remainder to the last flexible column
                        for (i, col) in self.columns.iter().enumerate().rev() {
                            if col.flexible() {
                                widths[i] += remainder;
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Calculate total width
        let table_width: usize = widths.iter().sum::<usize>() + extra;

        // If table is too wide, collapse columns proportionally
        if table_width > max_width {
            let excess = table_width - max_width;
            let total_width: usize = widths.iter().sum();
            if total_width > 0 {
                let mut remaining_excess = excess;
                for width in widths.iter_mut() {
                    if *width > 1 {
                        let reduce = (*width * excess) / total_width;
                        let actual_reduce = reduce.min(*width - 1).min(remaining_excess);
                        *width -= actual_reduce;
                        remaining_excess = remaining_excess.saturating_sub(actual_reduce);
                    }
                }
            }
        }

        // If expanding and table is too narrow, distribute extra space
        let current_width: usize = widths.iter().sum::<usize>() + extra;
        let min_target = self.min_width.unwrap_or(0);
        if self.expand || current_width < min_target {
            let target = self.width.unwrap_or(max_width).max(min_target);
            if current_width < target {
                let extra_space = target - current_width;
                let total_width: usize = widths.iter().sum();
                if total_width > 0 {
                    let mut remaining = extra_space;
                    let widths_count = widths.len();
                    for (i, width) in widths.iter_mut().enumerate() {
                        let add = if i == widths_count - 1 {
                            remaining
                        } else {
                            let proportional = (*width * extra_space) / total_width;
                            proportional.min(remaining)
                        };
                        *width += add;
                        remaining = remaining.saturating_sub(add);
                    }
                }
            }
        }

        widths
    }

    /// Render a cell with proper styling and padding.
    fn render_cell(
        &self,
        console: &Console<Stdout>,
        options: &ConsoleOptions,
        cell: &dyn Renderable,
        column: &Column,
        width: usize,
        style: Style,
        _is_header: bool,
        _is_footer: bool,
    ) -> Vec<Vec<Segment>> {
        // Get actual padding for this column (respects collapse_padding and pad_edge)
        let (pad_left, pad_right) = self.get_padding_for_column(column._index);
        let padding = pad_left + pad_right;
        let content_width = width.saturating_sub(padding);

        // Create options for cell rendering
        let cell_options = options.update_width(content_width);

        // Render cell content
        let cell_lines = console.render_lines(
            cell,
            Some(&cell_options),
            Some(style),
            true,
            false,
        );

        // Apply padding to each line
        let left_pad = Segment::styled(" ".repeat(pad_left), style);
        let right_pad = Segment::styled(" ".repeat(pad_right), style);

        cell_lines
            .into_iter()
            .map(|line| {
                let mut padded = Vec::new();
                if pad_left > 0 {
                    padded.push(left_pad.clone());
                }
                for seg in line {
                    padded.push(seg);
                }
                if pad_right > 0 {
                    padded.push(right_pad.clone());
                }
                // Adjust to exact width
                Segment::adjust_line_length(&padded, width, Some(style), true)
            })
            .collect()
    }
}

// SAFETY: Table is Send + Sync because:
// - columns: Vec<Column> where Column contains Box<dyn Renderable + Send + Sync>
// - rows: Vec<Row> where Row contains Vec<Box<dyn Renderable + Send + Sync>>
// - All other fields are Send + Sync
unsafe impl Send for Table {}
unsafe impl Sync for Table {}

impl Renderable for Table {
    fn render(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Segments {
        let mut result = Segments::new();

        // Empty table
        if self.columns.is_empty() {
            result.push(Segment::line());
            return result;
        }

        // Determine box characters
        let safe_box = self.safe_box.unwrap_or(options.legacy_windows);
        let box_chars = self.box_type.map(|b| b.substitute(safe_box, options.ascii_only()));

        // Calculate column widths
        // Note: calculate_column_widths handles extra_width internally, so pass full max_width
        let max_width = self.width.unwrap_or(options.max_width);
        let extra = self.extra_width();
        let col_options = options.update_width(max_width);
        let widths = self.calculate_column_widths(console, &col_options);

        if widths.is_empty() {
            result.push(Segment::line());
            return result;
        }

        let table_width: usize = widths.iter().sum::<usize>() + extra;
        let border_style = self.style.combine(&self.border_style);
        let new_line = Segment::line();

        // Render title
        if let Some(ref title) = self.title {
            let title_lines = console.render_lines(title, Some(options), None, true, false);
            for line in title_lines {
                let line_width = Segment::get_line_length(&line);
                let padding = table_width.saturating_sub(line_width);

                let (left_pad, right_pad) = match self.title_align {
                    AlignMethod::Left => (0, padding),
                    AlignMethod::Center => {
                        let left = padding / 2;
                        (left, padding - left)
                    }
                    AlignMethod::Right => (padding, 0),
                };

                if left_pad > 0 {
                    result.push(Segment::new(" ".repeat(left_pad)));
                }
                for seg in line {
                    result.push(seg);
                }
                if right_pad > 0 {
                    result.push(Segment::new(" ".repeat(right_pad)));
                }
                result.push(new_line.clone());
            }
        }

        // Render top border
        if let Some(ref bx) = box_chars {
            if self.show_edge {
                let top = bx.get_top(&widths);
                result.push(Segment::styled(top, border_style));
                result.push(new_line.clone());
            }
        }

        // Render header row
        if self.show_header {
            let header_row_style = self.header_style;

            // Render each header cell
            let mut header_cells: Vec<Vec<Vec<Segment>>> = Vec::new();
            let mut max_height = 1;

            let empty_text = Text::plain("");
            for (i, column) in self.columns.iter().enumerate() {
                let cell: &dyn Renderable = column
                    .header
                    .as_ref()
                    .map(|b| b.as_ref())
                    .unwrap_or(&empty_text as &dyn Renderable);

                let cell_style = header_row_style.combine(&column.header_style);
                let cell_lines = self.render_cell(
                    console,
                    options,
                    cell,
                    column,
                    widths[i],
                    cell_style,
                    true,
                    false,
                );
                max_height = max_height.max(cell_lines.len());
                header_cells.push(cell_lines);
            }

            // Normalize heights
            for (i, cells) in header_cells.iter_mut().enumerate() {
                while cells.len() < max_height {
                    let blank = Segment::adjust_line_length(&[], widths[i], Some(header_row_style), true);
                    cells.push(blank);
                }
            }

            // Render header lines
            for line_idx in 0..max_height {
                if let Some(ref bx) = box_chars {
                    if self.show_edge {
                        result.push(Segment::styled(bx.head_left.to_string(), border_style));
                    }
                }

                for (col_idx, cells) in header_cells.iter().enumerate() {
                    for seg in &cells[line_idx] {
                        result.push(seg.clone());
                    }
                    if col_idx < header_cells.len() - 1 {
                        if let Some(ref bx) = box_chars {
                            result.push(Segment::styled(bx.head_vertical.to_string(), border_style));
                        }
                    }
                }

                if let Some(ref bx) = box_chars {
                    if self.show_edge {
                        result.push(Segment::styled(bx.head_right.to_string(), border_style));
                    }
                }
                result.push(new_line.clone());
            }

            // Render header separator
            if let Some(ref bx) = box_chars {
                let row_line = bx.get_row(&widths, RowLevel::Head, self.show_edge);
                result.push(Segment::styled(row_line, border_style));
                result.push(new_line.clone());
            }
        }

        // Render data rows
        let empty_cell = Text::plain("");
        for (row_idx, row) in self.rows.iter().enumerate() {
            let row_style = self.get_row_style(row_idx);

            // Render each cell
            let mut row_cells: Vec<Vec<Vec<Segment>>> = Vec::new();
            let mut max_height = 1;

            for (col_idx, column) in self.columns.iter().enumerate() {
                let cell: &dyn Renderable = row
                    .cells
                    .get(col_idx)
                    .map(|b| b.as_ref())
                    .unwrap_or(&empty_cell as &dyn Renderable);

                let cell_style = self.style.combine(&column.style).combine(&row_style);
                let cell_lines = self.render_cell(
                    console,
                    options,
                    cell,
                    column,
                    widths[col_idx],
                    cell_style,
                    false,
                    false,
                );
                max_height = max_height.max(cell_lines.len());
                row_cells.push(cell_lines);
            }

            // Normalize heights
            let combined_style = self.style.combine(&row_style);
            for (i, cells) in row_cells.iter_mut().enumerate() {
                while cells.len() < max_height {
                    let blank = Segment::adjust_line_length(&[], widths[i], Some(combined_style), true);
                    cells.push(blank);
                }
            }

            // Render row lines
            for line_idx in 0..max_height {
                if let Some(ref bx) = box_chars {
                    if self.show_edge {
                        result.push(Segment::styled(bx.mid_left.to_string(), border_style));
                    }
                }

                for (col_idx, cells) in row_cells.iter().enumerate() {
                    for seg in &cells[line_idx] {
                        result.push(seg.clone());
                    }
                    if col_idx < row_cells.len() - 1 {
                        if let Some(ref bx) = box_chars {
                            result.push(Segment::styled(bx.mid_vertical.to_string(), border_style));
                        }
                    }
                }

                if let Some(ref bx) = box_chars {
                    if self.show_edge {
                        result.push(Segment::styled(bx.mid_right.to_string(), border_style));
                    }
                }
                result.push(new_line.clone());
            }

            // Render row separator if needed
            let is_last_row = row_idx == self.rows.len() - 1;
            let needs_separator = !is_last_row && (self.show_lines || row.end_section);

            if let Some(ref bx) = box_chars {
                if needs_separator {
                    let row_line = bx.get_row(&widths, RowLevel::Row, self.show_edge);
                    result.push(Segment::styled(row_line, border_style));
                    result.push(new_line.clone());
                } else if self.leading > 0 && !is_last_row {
                    // Add blank lines for leading
                    for _ in 0..self.leading {
                        let row_line = bx.get_row(&widths, RowLevel::Mid, self.show_edge);
                        result.push(Segment::styled(row_line, border_style));
                        result.push(new_line.clone());
                    }
                }
            }
        }

        // Render footer
        if self.show_footer {
            // Footer separator
            if let Some(ref bx) = box_chars {
                let row_line = bx.get_row(&widths, RowLevel::Foot, self.show_edge);
                result.push(Segment::styled(row_line, border_style));
                result.push(new_line.clone());
            }

            let footer_row_style = self.footer_style;

            let mut footer_cells: Vec<Vec<Vec<Segment>>> = Vec::new();
            let mut max_height = 1;

            let empty_footer = Text::plain("");
            for (i, column) in self.columns.iter().enumerate() {
                let cell: &dyn Renderable = column
                    .footer
                    .as_ref()
                    .map(|b| b.as_ref())
                    .unwrap_or(&empty_footer as &dyn Renderable);

                let cell_style = footer_row_style.combine(&column.footer_style);
                let cell_lines = self.render_cell(
                    console,
                    options,
                    cell,
                    column,
                    widths[i],
                    cell_style,
                    false,
                    true,
                );
                max_height = max_height.max(cell_lines.len());
                footer_cells.push(cell_lines);
            }

            // Normalize heights
            for (i, cells) in footer_cells.iter_mut().enumerate() {
                while cells.len() < max_height {
                    let blank = Segment::adjust_line_length(&[], widths[i], Some(footer_row_style), true);
                    cells.push(blank);
                }
            }

            // Render footer lines
            for line_idx in 0..max_height {
                if let Some(ref bx) = box_chars {
                    if self.show_edge {
                        result.push(Segment::styled(bx.foot_left.to_string(), border_style));
                    }
                }

                for (col_idx, cells) in footer_cells.iter().enumerate() {
                    for seg in &cells[line_idx] {
                        result.push(seg.clone());
                    }
                    if col_idx < footer_cells.len() - 1 {
                        if let Some(ref bx) = box_chars {
                            result.push(Segment::styled(bx.foot_vertical.to_string(), border_style));
                        }
                    }
                }

                if let Some(ref bx) = box_chars {
                    if self.show_edge {
                        result.push(Segment::styled(bx.foot_right.to_string(), border_style));
                    }
                }
                result.push(new_line.clone());
            }
        }

        // Render bottom border
        if let Some(ref bx) = box_chars {
            if self.show_edge {
                let bottom = bx.get_bottom(&widths);
                result.push(Segment::styled(bottom, border_style));
                result.push(new_line.clone());
            }
        }

        // Render caption
        if let Some(ref caption) = self.caption {
            let caption_lines = console.render_lines(caption, Some(options), None, true, false);
            for line in caption_lines {
                let line_width = Segment::get_line_length(&line);
                let padding = table_width.saturating_sub(line_width);

                let (left_pad, right_pad) = match self.caption_align {
                    AlignMethod::Left => (0, padding),
                    AlignMethod::Center => {
                        let left = padding / 2;
                        (left, padding - left)
                    }
                    AlignMethod::Right => (padding, 0),
                };

                if left_pad > 0 {
                    result.push(Segment::new(" ".repeat(left_pad)));
                }
                for seg in line {
                    result.push(seg);
                }
                if right_pad > 0 {
                    result.push(Segment::new(" ".repeat(right_pad)));
                }
                result.push(new_line.clone());
            }
        }

        result
    }

    fn measure(&self, console: &Console<Stdout>, options: &ConsoleOptions) -> Measurement {
        if self.columns.is_empty() {
            return Measurement::new(0, 0);
        }

        let max_width = self.width.unwrap_or(options.max_width);
        if max_width == 0 {
            return Measurement::new(0, 0);
        }

        let extra = self.extra_width();
        // Note: calculate_column_widths handles extra_width internally, so pass full max_width
        let col_options = options.update_width(max_width);
        let widths = self.calculate_column_widths(console, &col_options);

        let total_width: usize = widths.iter().sum::<usize>() + extra;

        // Calculate minimum width
        let min_measurements: Vec<Measurement> = self
            .columns
            .iter()
            .map(|col| self.measure_column(console, options, col))
            .collect();
        let min_width: usize = min_measurements.iter().map(|m| m.minimum).sum::<usize>() + extra;

        let final_max = if self.width.is_some() {
            self.width.unwrap()
        } else {
            total_width
        };

        Measurement::new(min_width, final_max)
            .clamp_bounds(self.min_width, Some(max_width))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cells::cell_len;
    use crate::r#box::{ASCII, DOUBLE, ROUNDED, SQUARE};

    // ==================== Column tests ====================

    #[test]
    fn test_column_default() {
        let col = Column::new();
        assert!(col.header.is_none());
        assert_eq!(col.justify, JustifyMethod::Left);
        assert!(!col.flexible());
    }

    #[test]
    fn test_column_with_header_str() {
        let col = Column::with_header_str("Name");
        assert!(col.header.is_some());
    }

    #[test]
    fn test_column_flexible() {
        let col = Column::new().ratio(1);
        assert!(col.flexible());
    }

    #[test]
    fn test_column_builder() {
        let col = Column::new()
            .justify(JustifyMethod::Right)
            .width(10)
            .min_width(5)
            .max_width(20)
            .no_wrap(true);

        assert_eq!(col.justify, JustifyMethod::Right);
        assert_eq!(col.width, Some(10));
        assert_eq!(col.min_width, Some(5));
        assert_eq!(col.max_width, Some(20));
        assert!(col.no_wrap);
    }

    // ==================== Row tests ====================

    #[test]
    fn test_row_empty() {
        let row = Row::empty();
        assert!(row.cells.is_empty());
        assert!(row.style.is_none());
        assert!(!row.end_section);
    }

    #[test]
    fn test_row_with_style() {
        let style = Style::new().with_bold(true);
        let row = Row::empty().with_style(style);
        assert_eq!(row.style, Some(style));
    }

    #[test]
    fn test_row_with_end_section() {
        let row = Row::empty().with_end_section(true);
        assert!(row.end_section);
    }

    // ==================== Table construction tests ====================

    #[test]
    fn test_table_new() {
        let table = Table::new();
        assert_eq!(table.column_count(), 0);
        assert_eq!(table.row_count(), 0);
        assert!(table.box_type.is_some());
        assert!(table.show_header);
    }

    #[test]
    fn test_table_grid() {
        let table = Table::grid();
        assert!(table.box_type.is_none());
        assert!(!table.show_header);
        assert!(!table.show_edge);
    }

    #[test]
    fn test_table_add_column() {
        let mut table = Table::new();
        table.add_column_str("Name");
        table.add_column_str("Age");
        assert_eq!(table.column_count(), 2);
    }

    #[test]
    fn test_table_add_row() {
        let mut table = Table::new();
        table.add_column_str("Name");
        table.add_column_str("Age");
        table.add_row_strs(&["Alice", "30"]);
        table.add_row_strs(&["Bob", "25"]);
        assert_eq!(table.row_count(), 2);
    }

    #[test]
    fn test_table_auto_add_columns() {
        let mut table = Table::new();
        table.add_row_strs(&["A", "B", "C"]);
        assert_eq!(table.column_count(), 3);
    }

    #[test]
    fn test_table_add_section() {
        let mut table = Table::new();
        table.add_row_strs(&["A", "B"]);
        table.add_section();
        assert!(table.rows[0].end_section);
    }

    // ==================== Table builder tests ====================

    #[test]
    fn test_table_builder() {
        let table = Table::new()
            .with_box(Some(DOUBLE))
            .with_expand(true)
            .with_show_header(false)
            .with_width(50);

        assert_eq!(table.box_type, Some(DOUBLE));
        assert!(table.expand);
        assert!(!table.show_header);
        assert_eq!(table.width, Some(50));
    }

    #[test]
    fn test_table_with_title() {
        let table = Table::new().with_title("My Table");
        assert!(table.title.is_some());
    }

    #[test]
    fn test_table_with_caption() {
        let table = Table::new().with_caption("Data from 2023");
        assert!(table.caption.is_some());
    }

    #[test]
    fn test_table_with_styles() {
        let style = Style::new().with_bold(true);
        let table = Table::new()
            .with_style(style)
            .with_header_style(style)
            .with_border_style(style)
            .with_row_styles(vec![style]);

        assert_eq!(table.style, style);
        assert_eq!(table.header_style, style);
        assert_eq!(table.border_style, style);
        assert_eq!(table.row_styles.len(), 1);
    }

    // ==================== Table render tests ====================

    #[test]
    fn test_table_render_empty() {
        let table = Table::new();
        let console = Console::with_options(ConsoleOptions::default());
        let options = console.options().clone();

        let segments = table.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(output.contains('\n'));
    }

    #[test]
    fn test_table_render_basic() {
        let mut table = Table::new();
        table.add_column_str("Name");
        table.add_column_str("Age");
        table.add_row_strs(&["Alice", "30"]);

        let console = Console::with_options(ConsoleOptions {
            max_width: 40,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = table.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(output.contains("Name"));
        assert!(output.contains("Age"));
        assert!(output.contains("Alice"));
        assert!(output.contains("30"));
    }

    #[test]
    fn test_table_render_grid() {
        let mut table = Table::grid();
        table.add_row_strs(&["A", "B", "C"]);

        let console = Console::with_options(ConsoleOptions {
            max_width: 20,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = table.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        assert!(output.contains('A'));
        assert!(output.contains('B'));
        assert!(output.contains('C'));
        // Grid has no borders
        assert!(!output.contains('┏'));
    }

    #[test]
    fn test_table_render_with_box_styles() {
        let boxes = [ROUNDED, SQUARE, DOUBLE, ASCII];

        for box_style in boxes {
            let mut table = Table::new().with_box(Some(box_style));
            table.add_column_str("X");
            table.add_row_strs(&["Y"]);

            let console = Console::with_options(ConsoleOptions {
                max_width: 20,
                ..Default::default()
            });
            let options = console.options().clone();

            let segments = table.render(&console, &options);
            assert!(!segments.is_empty());
        }
    }

    #[test]
    fn test_table_render_no_edge() {
        let mut table = Table::new().with_show_edge(false);
        table.add_column_str("Name");
        table.add_row_strs(&["Alice"]);

        let console = Console::with_options(ConsoleOptions {
            max_width: 30,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = table.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();

        // No left/right borders
        assert!(!output.contains("┃Name"));
    }

    #[test]
    fn test_table_render_show_lines() {
        let mut table = Table::new().with_show_lines(true);
        table.add_column_str("Name");
        table.add_row_strs(&["Alice"]);
        table.add_row_strs(&["Bob"]);

        let console = Console::with_options(ConsoleOptions {
            max_width: 30,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = table.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let lines: Vec<&str> = output.lines().collect();

        // Should have separators between rows
        assert!(lines.len() > 4); // top + header + sep + alice + sep + bob + bottom
    }

    #[test]
    fn test_table_render_expand() {
        let mut table = Table::new().with_expand(true);
        table.add_column_str("X");
        table.add_row_strs(&["Y"]);

        let console = Console::with_options(ConsoleOptions {
            max_width: 40,
            ..Default::default()
        });
        let options = console.options().clone();

        let segments = table.render(&console, &options);
        let output: String = segments.iter().map(|s| s.text.to_string()).collect();
        let first_line = output.lines().next().unwrap();

        // Without expand, a single column with "X" header would be ~5 chars
        // With expand, it should be significantly wider (at least 30+)
        let width = cell_len(first_line);
        assert!(width >= 30, "Expanded table width {} should be >= 30", width);
    }

    // ==================== Measure tests ====================

    #[test]
    fn test_table_measure_empty() {
        let table = Table::new();
        let console = Console::new();
        let options = ConsoleOptions::default();

        let m = table.measure(&console, &options);
        assert_eq!(m.minimum, 0);
        assert_eq!(m.maximum, 0);
    }

    #[test]
    fn test_table_measure_basic() {
        let mut table = Table::new();
        table.add_column_str("Name");
        table.add_row_strs(&["Alice"]);

        let console = Console::new();
        let options = ConsoleOptions::default();

        let m = table.measure(&console, &options);
        assert!(m.minimum > 0);
        assert!(m.maximum >= m.minimum);
    }

    #[test]
    fn test_table_measure_fixed_width() {
        let mut table = Table::new().with_width(50);
        table.add_column_str("Name");
        table.add_row_strs(&["Alice"]);

        let console = Console::new();
        let options = ConsoleOptions::default();

        let m = table.measure(&console, &options);
        assert_eq!(m.maximum, 50);
    }

    // ==================== Send + Sync tests ====================

    #[test]
    fn test_table_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Table>();
        assert_sync::<Table>();
    }

    #[test]
    fn test_column_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Column>();
        assert_sync::<Column>();
    }

    #[test]
    fn test_row_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Row>();
        assert_sync::<Row>();
    }

    // ==================== Debug tests ====================

    #[test]
    fn test_table_debug() {
        let mut table = Table::new().with_title("Test");
        table.add_column_str("A");
        table.add_row_strs(&["B"]);

        let debug_str = format!("{:?}", table);
        assert!(debug_str.contains("Table"));
        assert!(debug_str.contains("columns"));
        assert!(debug_str.contains("rows"));
    }

    #[test]
    fn test_column_debug() {
        let col = Column::with_header_str("Name");
        let debug_str = format!("{:?}", col);
        assert!(debug_str.contains("Column"));
    }

    #[test]
    fn test_row_debug() {
        let row = Row::empty();
        let debug_str = format!("{:?}", row);
        assert!(debug_str.contains("Row"));
    }
}
