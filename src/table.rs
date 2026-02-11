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
use crate::r#box::{Box as RichBox, HEAVY_HEAD, RowLevel};
use crate::console::{ConsoleOptions, JustifyMethod, OverflowMethod};
use crate::measure::Measurement;
use crate::padding::PaddingDimensions;
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

    /// Set the footer content (builder pattern).
    pub fn with_footer(mut self, footer: Box<dyn Renderable + Send + Sync>) -> Self {
        self.footer = Some(footer);
        self
    }

    /// Check if this column is flexible (has a ratio set).
    pub fn flexible(&self) -> bool {
        self.ratio.is_some()
    }

    // Mutation methods for use with Live displays

    /// Set the column justification.
    pub fn set_justify(&mut self, justify: JustifyMethod) {
        self.justify = justify;
    }

    /// Set the column style.
    pub fn set_style(&mut self, style: Style) {
        self.style = style;
    }

    /// Set the header style.
    pub fn set_header_style(&mut self, style: Style) {
        self.header_style = style;
    }

    /// Set the footer style.
    pub fn set_footer_style(&mut self, style: Style) {
        self.footer_style = style;
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
    /// Cell padding (top, right, bottom, left) — CSS order.
    padding: (usize, usize, usize, usize),
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
    /// Style for the title.
    title_style: Option<Style>,
    /// Style for the caption.
    caption_style: Option<Style>,
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
            padding: (0, 1, 0, 1),
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
            title_style: None,
            caption_style: None,
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
            padding: (0, 0, 0, 0),
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

    /// Set cell padding (left and right), keeping top/bottom at 0.
    ///
    /// For full 4-way padding, use `with_padding_dims`.
    pub fn with_padding(mut self, left: usize, right: usize) -> Self {
        self.padding = (0, right, 0, left);
        self
    }

    /// Set cell padding using CSS-order dimensions.
    ///
    /// Accepts `PaddingDimensions` (or anything that converts to it):
    /// - `usize` — all four sides
    /// - `(vert, horiz)` — top/bottom and left/right
    /// - `(top, right, bottom, left)` — CSS order
    pub fn with_padding_dims(mut self, pad: impl Into<PaddingDimensions>) -> Self {
        self.padding = pad.into().unpack();
        self
    }

    /// Set the title style.
    pub fn with_title_style(mut self, style: Style) -> Self {
        self.title_style = Some(style);
        self
    }

    /// Set the caption style.
    pub fn with_caption_style(mut self, style: Style) -> Self {
        self.caption_style = Some(style);
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
    // Mutation methods (for use with Live displays)
    // ========================================================================

    /// Set the table title.
    pub fn set_title(&mut self, title: Option<Text>) {
        self.title = title;
    }

    /// Set the table caption.
    pub fn set_caption(&mut self, caption: Option<Text>) {
        self.caption = caption;
    }

    /// Set whether to show the footer row.
    pub fn set_show_footer(&mut self, show: bool) {
        self.show_footer = show;
    }

    /// Set the border style.
    pub fn set_border_style(&mut self, style: Style) {
        self.border_style = style;
    }

    /// Set the box drawing style.
    pub fn set_box(&mut self, box_type: Option<RichBox>) {
        self.box_type = box_type;
    }

    /// Set alternating row styles.
    pub fn set_row_styles(&mut self, styles: Vec<Style>) {
        self.row_styles = styles;
    }

    /// Set whether to pad the edge cells.
    pub fn set_pad_edge(&mut self, pad: bool) {
        self.pad_edge = pad;
    }

    /// Set the fixed width (None for auto).
    pub fn set_width(&mut self, width: Option<usize>) {
        self.width = width;
    }

    /// Get a mutable reference to a column by index.
    pub fn column_mut(&mut self, idx: usize) -> Option<&mut Column> {
        self.columns.get_mut(idx)
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

    /// Get the padding width used for measuring / sizing columns.
    ///
    /// Mirrors Python Rich's `_get_padding_width`: collapse padding affects the
    /// *computed* padding width, but `pad_edge` does not.
    ///
    /// Rich uses this value when interpreting column width constraints
    /// (e.g. fixed-width columns, min/max bounds), while the actual padding applied
    /// to cells may vary with `pad_edge` (handled in `get_padding_for_column`).
    fn get_measure_padding_width(&self, column_index: usize) -> usize {
        let (_top, pad_right, _bottom, pad_left) = self.padding;
        let mut left = pad_left;
        let right = pad_right;

        if self.collapse_padding && column_index > 0 {
            left = left.saturating_sub(right);
        }

        left + right
    }

    /// Get the actual (left, right) padding for a specific column,
    /// accounting for collapse_padding and pad_edge settings.
    fn get_padding_for_column(&self, column_index: usize) -> (usize, usize) {
        let (_top, pad_right, _bottom, pad_left) = self.padding;
        let num_columns = self.columns.len();
        let is_first = column_index == 0;
        let is_last = column_index == num_columns.saturating_sub(1);

        let mut left = pad_left;
        let mut right = pad_right;

        // Collapse padding between columns (avoid double padding)
        if self.collapse_padding && !is_first {
            // Mirror Python Rich behavior: subtract the right padding rather than forcing to 0.
            // This matters when left != right.
            left = left.saturating_sub(pad_right);
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

        // Python Rich measures padded cell renderables, which means `pad_edge` affects
        // the measured width of flexible columns. For fixed-width columns / clamp
        // bounds Rich uses `_get_padding_width` (which ignores `pad_edge`), so we
        // keep both concepts here:
        // - `content_padding_width`: actual padding applied at render time
        // - `bounds_padding_width`: width used when interpreting column width constraints
        let (pad_left, pad_right) = self.get_padding_for_column(column._index);
        let content_padding_width = pad_left + pad_right;
        let bounds_padding_width = self.get_measure_padding_width(column._index);

        // Fixed width column
        if let Some(w) = column.width {
            return Measurement::new(w + bounds_padding_width, w + bounds_padding_width)
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

        let min_w = min_widths.iter().max().copied().unwrap_or(1) + content_padding_width;
        let max_w = max_widths.iter().max().copied().unwrap_or(max_width) + content_padding_width;

        Measurement::new(min_w, max_w)
            .with_maximum(max_width)
            .clamp_bounds(
                column.min_width.map(|w| w + bounds_padding_width),
                column.max_width.map(|w| w + bounds_padding_width),
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

        // Important: `options.max_width` here is the available width for *columns* only
        // (i.e. it excludes any border / divider extra width). This mirrors Python Rich,
        // which calls `_calculate_column_widths` with `options.update_width(max_width - extra_width)`.
        let max_width = options.max_width;
        let extra_width = self.extra_width();
        let effective_expand = self.expand || self.width.is_some();

        // Measure each column
        let measurements: Vec<Measurement> = self
            .columns
            .iter()
            .map(|col| self.measure_column(console, options, col))
            .collect();

        let mut widths: Vec<usize> = measurements.iter().map(|m| m.maximum.max(1)).collect();

        // Handle flexible columns with ratios
        if effective_expand {
            let ratios: Vec<usize> = self
                .columns
                .iter()
                .filter(|c| c.flexible())
                .map(|c| c.ratio.unwrap_or(0))
                .collect();

            if ratios.iter().any(|&r| r > 0) {
                let fixed_widths: Vec<usize> = self
                    .columns
                    .iter()
                    .zip(measurements.iter())
                    .map(|(column, measurement)| {
                        if column.flexible() {
                            0
                        } else {
                            measurement.maximum
                        }
                    })
                    .collect();

                let flex_minimums: Vec<usize> = self
                    .columns
                    .iter()
                    .filter(|column| column.flexible())
                    .map(|column| {
                        (column.width.unwrap_or(1)) + self.get_measure_padding_width(column._index)
                    })
                    .collect();

                let fixed_total: usize = fixed_widths.iter().sum();
                let flexible_width = max_width.saturating_sub(fixed_total);
                let flex_widths = ratio_distribute(flexible_width, &ratios, Some(&flex_minimums));
                let mut flex_iter = flex_widths.into_iter();
                for (index, column) in self.columns.iter().enumerate() {
                    if column.flexible() {
                        widths[index] = fixed_widths[index] + flex_iter.next().unwrap_or(0);
                    }
                }
            }
        }

        let mut table_width: usize = widths.iter().sum();

        if table_width > max_width {
            widths = collapse_widths(
                widths,
                self.columns
                    .iter()
                    .map(|column| column.width.is_none() && !column.no_wrap)
                    .collect(),
                max_width,
            );

            table_width = widths.iter().sum();

            // Last resort: reduce columns evenly.
            if table_width > max_width {
                let excess_width = table_width - max_width;
                let ratios = vec![1; widths.len()];
                widths = ratio_reduce(excess_width, &ratios, &widths, &widths);
                table_width = widths.iter().sum();
            }

            // Re-measure with constrained widths (critical for parity with Python Rich).
            let constrained: Vec<Measurement> = widths
                .iter()
                .zip(self.columns.iter())
                .map(|(width, column)| {
                    self.measure_column(console, &options.update_width(*width), column)
                })
                .collect();
            widths = constrained.iter().map(|m| m.maximum).collect();
        }

        // If expanding and table is too narrow, distribute extra space.
        // Mirrors Python Rich's logic: distribute proportionally using current widths.
        if (table_width < max_width && effective_expand)
            || (self.min_width.is_some()
                && table_width < self.min_width.unwrap_or(0).saturating_sub(extra_width))
        {
            let min_target = self.min_width.unwrap_or(0).saturating_sub(extra_width);
            let target = if self.min_width.is_some() {
                min_target.min(max_width)
            } else {
                max_width
            };

            if table_width < target {
                let pad_widths = ratio_distribute(target - table_width, &widths, None);
                widths = widths
                    .into_iter()
                    .zip(pad_widths.into_iter())
                    .map(|(w, pad)| w + pad)
                    .collect();
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
        let mut cell_options = options.update_width(content_width);
        if column.justify != JustifyMethod::Left {
            cell_options.justify = Some(column.justify);
        }
        if column.overflow != OverflowMethod::Ellipsis {
            cell_options.overflow = Some(column.overflow);
        }
        if column.no_wrap {
            cell_options.no_wrap = true;
        }

        // Render cell content
        let cell_lines = console.render_lines(cell, Some(&cell_options), Some(style), true, false);

        // Apply padding to each line
        let left_pad = Segment::styled(" ".repeat(pad_left), style);
        let right_pad = Segment::styled(" ".repeat(pad_right), style);

        let mut result: Vec<Vec<Segment>> = Vec::new();
        let (pad_top, _, pad_bottom, _) = self.padding;

        // Add top padding blank lines
        if pad_top > 0 {
            let blank = Segment::adjust_line_length(&[], width, Some(style), true);
            for _ in 0..pad_top {
                result.push(blank.clone());
            }
        }

        // Add content lines with left/right padding
        for line in cell_lines {
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
            result.push(Segment::adjust_line_length(&padded, width, Some(style), true));
        }

        // Add bottom padding blank lines
        if pad_bottom > 0 {
            let blank = Segment::adjust_line_length(&[], width, Some(style), true);
            for _ in 0..pad_bottom {
                result.push(blank.clone());
            }
        }

        result
    }
}


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
        let box_chars = self
            .box_type
            .map(|b| b.substitute(safe_box, options.ascii_only()));

        // Calculate column widths
        let max_width = self.width.unwrap_or(options.max_width);
        let extra = self.extra_width();
        // Match Python Rich: calculate widths against the available *column* width,
        // which excludes the table's extra border / divider width.
        let col_options = options.update_width(max_width.saturating_sub(extra));
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
            let title_lines = console.render_lines(title, Some(options), self.title_style, false, false);
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

            // Match Python Rich's grid title spacing: insert a blank line after the title
            // for borderless tables (e.g. `Table::grid()`).
            if !self.show_edge {
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
                    console, options, cell, column, widths[i], cell_style, true, false,
                );
                max_height = max_height.max(cell_lines.len());
                header_cells.push(cell_lines);
            }

            // Normalize heights - use column header style for padding
            for (i, cells) in header_cells.iter_mut().enumerate() {
                let col_style = header_row_style.combine(&self.columns[i].header_style);
                let hint_style = cells.last().and_then(|line| Segment::get_last_style(line));
                while cells.len() < max_height {
                    // When padding vertically, preserve only the **background** of the last rendered
                    // line to avoid visible hairlines in block backgrounds, but don't let decoration
                    // attributes (underline/dim/etc.) bleed into the padding.
                    let pad_style = hint_style
                        .and_then(|hint| hint.bgcolor.map(|bg| Style::new().with_bgcolor(bg)))
                        .map(|bg_style| col_style.combine(&bg_style))
                        .unwrap_or(col_style);
                    let blank = Segment::adjust_line_length(&[], widths[i], Some(pad_style), true);
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
                            result
                                .push(Segment::styled(bx.head_vertical.to_string(), border_style));
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

            // Normalize heights - use column style for padding and respect vertical alignment
            for (i, cells) in row_cells.iter_mut().enumerate() {
                let col_style = self
                    .style
                    .combine(&self.columns[i].style)
                    .combine(&row_style);
                let hint_style = cells.last().and_then(|line| Segment::get_last_style(line));
                // Preserve only background from the last rendered line (see header comment).
                let pad_style = hint_style
                    .and_then(|hint| hint.bgcolor.map(|bg| Style::new().with_bgcolor(bg)))
                    .map(|bg_style| col_style.combine(&bg_style))
                    .unwrap_or(col_style);
                let blank = Segment::adjust_line_length(&[], widths[i], Some(pad_style), true);

                let lines_needed = max_height.saturating_sub(cells.len());
                if lines_needed > 0 {
                    match self.columns[i].vertical {
                        VerticalAlignMethod::Top => {
                            // Add blanks at end
                            for _ in 0..lines_needed {
                                cells.push(blank.clone());
                            }
                        }
                        VerticalAlignMethod::Middle => {
                            // Add blanks at start and end
                            let top_pad = lines_needed / 2;
                            let bottom_pad = lines_needed - top_pad;
                            let mut new_cells = Vec::with_capacity(max_height);
                            for _ in 0..top_pad {
                                new_cells.push(blank.clone());
                            }
                            new_cells.append(cells);
                            for _ in 0..bottom_pad {
                                new_cells.push(blank.clone());
                            }
                            *cells = new_cells;
                        }
                        VerticalAlignMethod::Bottom => {
                            // Add blanks at start
                            let mut new_cells = Vec::with_capacity(max_height);
                            for _ in 0..lines_needed {
                                new_cells.push(blank.clone());
                            }
                            new_cells.append(cells);
                            *cells = new_cells;
                        }
                    }
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
                    console, options, cell, column, widths[i], cell_style, false, true,
                );
                max_height = max_height.max(cell_lines.len());
                footer_cells.push(cell_lines);
            }

            // Normalize heights - use column footer style for padding
            for (i, cells) in footer_cells.iter_mut().enumerate() {
                let col_style = footer_row_style.combine(&self.columns[i].footer_style);
                let hint_style = cells.last().and_then(|line| Segment::get_last_style(line));
                while cells.len() < max_height {
                    // Preserve only background from the last rendered line (see header comment).
                    let pad_style = hint_style
                        .and_then(|hint| hint.bgcolor.map(|bg| Style::new().with_bgcolor(bg)))
                        .map(|bg_style| col_style.combine(&bg_style))
                        .unwrap_or(col_style);
                    let blank = Segment::adjust_line_length(&[], widths[i], Some(pad_style), true);
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
                            result
                                .push(Segment::styled(bx.foot_vertical.to_string(), border_style));
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
            let mut caption_options = options.clone();
            caption_options.max_width = table_width;
            let caption_lines =
                console.render_lines(caption, Some(&caption_options), self.caption_style, false, false);
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

        // Mirror Python Rich: measure columns against available column width
        // (excluding border / dividers), then re-measure columns under the computed
        // total column width to get final min/max ranges.
        let extra = self.extra_width();
        let col_options = options.update_width(max_width.saturating_sub(extra));
        let col_widths = self.calculate_column_widths(console, &col_options);
        let content_max_width: usize = col_widths.iter().sum();

        let measurements: Vec<Measurement> = self
            .columns
            .iter()
            .map(|col| self.measure_column(console, &options.update_width(content_max_width), col))
            .collect();

        let min_width: usize = measurements.iter().map(|m| m.minimum).sum::<usize>() + extra;
        let max_calc: usize = if self.width.is_some() {
            self.width.unwrap()
        } else {
            measurements.iter().map(|m| m.maximum).sum::<usize>() + extra
        };

        Measurement::new(min_width, max_calc).clamp_bounds(self.min_width, Some(max_width))
    }
}

// ============================================================================
// Width distribution helpers (ported from Python Rich)
// ============================================================================

fn div_ceil(numer: u128, denom: u128) -> usize {
    if denom == 0 {
        return 0;
    }
    ((numer + denom - 1) / denom) as usize
}

/// Banker's rounding (ties to even) for positive rationals.
///
/// Python's `round()` uses bankers rounding, which is important for Rich parity.
fn round_div_bankers(numer: u128, denom: u128) -> usize {
    if denom == 0 {
        return 0;
    }
    let q = numer / denom;
    let r = numer % denom;
    let twice_r = r.saturating_mul(2);

    if twice_r < denom {
        q as usize
    } else if twice_r > denom {
        (q + 1) as usize
    } else {
        // Exactly half-way: round to even.
        if q % 2 == 0 {
            q as usize
        } else {
            (q + 1) as usize
        }
    }
}

/// Equivalent of `rich._ratio.ratio_reduce`.
///
/// `total` is the number of cells to remove from `values`, proportionally to `ratios`,
/// with per-slot caps in `maximums`.
fn ratio_reduce(
    total: usize,
    ratios: &[usize],
    maximums: &[usize],
    values: &[usize],
) -> Vec<usize> {
    let mut adjusted_ratios: Vec<usize> = Vec::with_capacity(ratios.len());
    for (&ratio, &max) in ratios.iter().zip(maximums.iter()) {
        adjusted_ratios.push(if max > 0 { ratio } else { 0 });
    }

    let mut total_ratio: usize = adjusted_ratios.iter().sum();
    if total_ratio == 0 {
        return values.to_vec();
    }

    let mut total_remaining = total;
    let mut result: Vec<usize> = Vec::with_capacity(values.len());

    for ((&ratio, &maximum), &value) in adjusted_ratios
        .iter()
        .zip(maximums.iter())
        .zip(values.iter())
    {
        if ratio > 0 && total_ratio > 0 {
            let numer = (ratio as u128) * (total_remaining as u128);
            let rounded = round_div_bankers(numer, total_ratio as u128);
            let distributed = rounded.min(maximum);
            result.push(value.saturating_sub(distributed));
            total_remaining = total_remaining.saturating_sub(distributed);
            total_ratio = total_ratio.saturating_sub(ratio);
        } else {
            result.push(value);
        }
    }

    result
}

/// Equivalent of `rich._ratio.ratio_distribute` (with optional minimums).
///
/// Distributes `total` into parts based on `ratios`. With `minimums`, each slot
/// will get at least its minimum (and slots with minimum 0 are treated as ratio 0).
fn ratio_distribute(total: usize, ratios: &[usize], minimums: Option<&[usize]>) -> Vec<usize> {
    let mut adjusted_ratios: Vec<usize> = Vec::with_capacity(ratios.len());
    if let Some(minimums) = minimums {
        for (&ratio, &min) in ratios.iter().zip(minimums.iter()) {
            adjusted_ratios.push(if min > 0 { ratio } else { 0 });
        }
    } else {
        adjusted_ratios.extend_from_slice(ratios);
    }

    let mut total_ratio: usize = adjusted_ratios.iter().sum();
    if total_ratio == 0 {
        // Mirrors Rich's expectation: ratio_distribute requires sum(ratios) > 0,
        // but in practice we can just return zeros for robustness.
        return vec![0; ratios.len()];
    }

    let mut total_remaining = total;
    let mut distributed_total: Vec<usize> = Vec::with_capacity(adjusted_ratios.len());

    let mins: Vec<usize> = if let Some(minimums) = minimums {
        minimums.to_vec()
    } else {
        vec![0; adjusted_ratios.len()]
    };

    for (&ratio, &minimum) in adjusted_ratios.iter().zip(mins.iter()) {
        let distributed = if total_ratio > 0 {
            let numer = (ratio as u128) * (total_remaining as u128);
            div_ceil(numer, total_ratio as u128).max(minimum)
        } else {
            total_remaining
        };
        distributed_total.push(distributed);
        total_ratio = total_ratio.saturating_sub(ratio);
        total_remaining = total_remaining.saturating_sub(distributed);
    }

    distributed_total
}

/// Equivalent of `Table._collapse_widths` from Python Rich.
fn collapse_widths(mut widths: Vec<usize>, wrapable: Vec<bool>, max_width: usize) -> Vec<usize> {
    let mut total_width: usize = widths.iter().sum();
    let mut excess_width = total_width.saturating_sub(max_width);

    if wrapable.iter().any(|&w| w) {
        while total_width > 0 && excess_width > 0 {
            let max_column = widths
                .iter()
                .zip(wrapable.iter())
                .filter_map(|(width, allow_wrap)| allow_wrap.then_some(*width))
                .max()
                .unwrap_or(0);

            let second_max_column = widths
                .iter()
                .zip(wrapable.iter())
                .filter_map(|(width, allow_wrap)| {
                    if *allow_wrap && *width != max_column {
                        Some(*width)
                    } else {
                        None
                    }
                })
                .max()
                .unwrap_or(0);

            let column_difference = max_column.saturating_sub(second_max_column);
            let ratios: Vec<usize> = widths
                .iter()
                .zip(wrapable.iter())
                .map(|(width, allow_wrap)| {
                    if *allow_wrap && *width == max_column {
                        1
                    } else {
                        0
                    }
                })
                .collect();

            if ratios.iter().all(|&r| r == 0) || column_difference == 0 {
                break;
            }

            let max_reduce: Vec<usize> = vec![excess_width.min(column_difference); widths.len()];
            widths = ratio_reduce(excess_width, &ratios, &max_reduce, &widths);

            total_width = widths.iter().sum();
            excess_width = total_width.saturating_sub(max_width);
        }
    }

    widths
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r#box::{ASCII, DOUBLE, ROUNDED, SQUARE};
    use crate::cells::cell_len;

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
        assert!(
            width >= 30,
            "Expanded table width {} should be >= 30",
            width
        );
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

    // ==================== Title/caption style tests ====================

    #[test]
    fn test_table_with_title_style() {
        let style = Style::new().with_bold(true);
        let table = Table::new()
            .with_title("My Title")
            .with_title_style(style);
        assert_eq!(table.title_style, Some(style));
    }

    #[test]
    fn test_table_with_caption_style() {
        let style = Style::new().with_italic(true);
        let table = Table::new()
            .with_caption("My Caption")
            .with_caption_style(style);
        assert_eq!(table.caption_style, Some(style));
    }

    #[test]
    fn test_table_four_way_padding() {
        use crate::padding::PaddingDimensions;
        let table = Table::new().with_padding_dims(PaddingDimensions::FourWay(1, 2, 1, 2));
        assert_eq!(table.padding, (1, 2, 1, 2));
    }

    #[test]
    fn test_table_padding_from_usize() {
        let table = Table::new().with_padding_dims(3usize);
        assert_eq!(table.padding, (3, 3, 3, 3));
    }

    #[test]
    fn test_table_padding_compat() {
        // Old 2-arg with_padding sets left/right with 0 top/bottom
        let table = Table::new().with_padding(2, 3);
        assert_eq!(table.padding, (0, 3, 0, 2));
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
