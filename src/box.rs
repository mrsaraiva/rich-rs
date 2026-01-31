//! Box drawing characters for borders, panels, and tables.
//!
//! This module provides a comprehensive set of box-drawing character definitions
//! matching Python Rich's `box.py`. Each `Box` defines 32 characters arranged in
//! 8 rows that control how borders and dividers are rendered:
//!
//! ```text
//! ┌─┬┐ top
//! │ ││ head
//! ├─┼┤ head_row
//! │ ││ mid
//! ├─┼┤ row
//! ├─┼┤ foot_row
//! │ ││ foot
//! └─┴┘ bottom
//! ```

/// Level of a row separator in a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowLevel {
    /// Header row separator (between header and body).
    Head,
    /// Regular row separator.
    Row,
    /// Footer row separator (between body and footer).
    Foot,
    /// Mid-section (vertical lines only, used for content rows).
    Mid,
}

/// A complete set of box-drawing characters for creating borders and tables.
///
/// The structure contains 32 characters arranged in 8 logical rows:
/// - `top`: Top border (corners and dividers)
/// - `head`: Header content row (vertical lines)
/// - `head_row`: Header separator row
/// - `mid`: Mid content row (vertical lines)
/// - `row`: Body row separator
/// - `foot_row`: Footer separator row
/// - `foot`: Footer content row (vertical lines)
/// - `bottom`: Bottom border (corners and dividers)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Box {
    // Top row: ┌─┬┐
    /// Top-left corner character.
    pub top_left: char,
    /// Top horizontal line character.
    pub top: char,
    /// Top divider (T-junction pointing down).
    pub top_divider: char,
    /// Top-right corner character.
    pub top_right: char,

    // Head row: │ ││
    /// Left edge of header row.
    pub head_left: char,
    /// Vertical divider in header.
    pub head_vertical: char,
    /// Right edge of header row.
    pub head_right: char,

    // Head separator row: ├─┼┤
    /// Left edge of head row separator.
    pub head_row_left: char,
    /// Horizontal line in head row separator.
    pub head_row_horizontal: char,
    /// Cross junction in head row separator.
    pub head_row_cross: char,
    /// Right edge of head row separator.
    pub head_row_right: char,

    // Mid row: │ ││
    /// Left edge of mid content row.
    pub mid_left: char,
    /// Vertical divider in mid section.
    pub mid_vertical: char,
    /// Right edge of mid content row.
    pub mid_right: char,

    // Row separator: ├─┼┤
    /// Left edge of row separator.
    pub row_left: char,
    /// Horizontal line in row separator.
    pub row_horizontal: char,
    /// Cross junction in row separator.
    pub row_cross: char,
    /// Right edge of row separator.
    pub row_right: char,

    // Foot separator row: ├─┼┤
    /// Left edge of foot row separator.
    pub foot_row_left: char,
    /// Horizontal line in foot row separator.
    pub foot_row_horizontal: char,
    /// Cross junction in foot row separator.
    pub foot_row_cross: char,
    /// Right edge of foot row separator.
    pub foot_row_right: char,

    // Foot row: │ ││
    /// Left edge of footer row.
    pub foot_left: char,
    /// Vertical divider in footer.
    pub foot_vertical: char,
    /// Right edge of footer row.
    pub foot_right: char,

    // Bottom row: └─┴┘
    /// Bottom-left corner character.
    pub bottom_left: char,
    /// Bottom horizontal line character.
    pub bottom: char,
    /// Bottom divider (T-junction pointing up).
    pub bottom_divider: char,
    /// Bottom-right corner character.
    pub bottom_right: char,

    /// Whether this box uses ASCII characters only.
    pub ascii: bool,
}

/// ASCII box using + - | characters.
pub const ASCII: Box = Box {
    top_left: '+',
    top: '-',
    top_divider: '-',
    top_right: '+',
    head_left: '|',
    head_vertical: '|',
    head_right: '|',
    head_row_left: '|',
    head_row_horizontal: '-',
    head_row_cross: '+',
    head_row_right: '|',
    mid_left: '|',
    mid_vertical: '|',
    mid_right: '|',
    row_left: '|',
    row_horizontal: '-',
    row_cross: '+',
    row_right: '|',
    foot_row_left: '|',
    foot_row_horizontal: '-',
    foot_row_cross: '+',
    foot_row_right: '|',
    foot_left: '|',
    foot_vertical: '|',
    foot_right: '|',
    bottom_left: '+',
    bottom: '-',
    bottom_divider: '-',
    bottom_right: '+',
    ascii: true,
};

/// ASCII box variant 2 with + for all junctions.
pub const ASCII2: Box = Box {
    top_left: '+',
    top: '-',
    top_divider: '+',
    top_right: '+',
    head_left: '|',
    head_vertical: '|',
    head_right: '|',
    head_row_left: '+',
    head_row_horizontal: '-',
    head_row_cross: '+',
    head_row_right: '+',
    mid_left: '|',
    mid_vertical: '|',
    mid_right: '|',
    row_left: '+',
    row_horizontal: '-',
    row_cross: '+',
    row_right: '+',
    foot_row_left: '+',
    foot_row_horizontal: '-',
    foot_row_cross: '+',
    foot_row_right: '+',
    foot_left: '|',
    foot_vertical: '|',
    foot_right: '|',
    bottom_left: '+',
    bottom: '-',
    bottom_divider: '+',
    bottom_right: '+',
    ascii: true,
};

/// ASCII box with double-line header separator (= instead of -).
pub const ASCII_DOUBLE_HEAD: Box = Box {
    top_left: '+',
    top: '-',
    top_divider: '+',
    top_right: '+',
    head_left: '|',
    head_vertical: '|',
    head_right: '|',
    head_row_left: '+',
    head_row_horizontal: '=',
    head_row_cross: '+',
    head_row_right: '+',
    mid_left: '|',
    mid_vertical: '|',
    mid_right: '|',
    row_left: '+',
    row_horizontal: '-',
    row_cross: '+',
    row_right: '+',
    foot_row_left: '+',
    foot_row_horizontal: '-',
    foot_row_cross: '+',
    foot_row_right: '+',
    foot_left: '|',
    foot_vertical: '|',
    foot_right: '|',
    bottom_left: '+',
    bottom: '-',
    bottom_divider: '+',
    bottom_right: '+',
    ascii: true,
};

/// Square box with thin Unicode lines.
pub const SQUARE: Box = Box {
    top_left: '┌',
    top: '─',
    top_divider: '┬',
    top_right: '┐',
    head_left: '│',
    head_vertical: '│',
    head_right: '│',
    head_row_left: '├',
    head_row_horizontal: '─',
    head_row_cross: '┼',
    head_row_right: '┤',
    mid_left: '│',
    mid_vertical: '│',
    mid_right: '│',
    row_left: '├',
    row_horizontal: '─',
    row_cross: '┼',
    row_right: '┤',
    foot_row_left: '├',
    foot_row_horizontal: '─',
    foot_row_cross: '┼',
    foot_row_right: '┤',
    foot_left: '│',
    foot_vertical: '│',
    foot_right: '│',
    bottom_left: '└',
    bottom: '─',
    bottom_divider: '┴',
    bottom_right: '┘',
    ascii: false,
};

/// Square box with double-line header separator.
pub const SQUARE_DOUBLE_HEAD: Box = Box {
    top_left: '┌',
    top: '─',
    top_divider: '┬',
    top_right: '┐',
    head_left: '│',
    head_vertical: '│',
    head_right: '│',
    head_row_left: '╞',
    head_row_horizontal: '═',
    head_row_cross: '╪',
    head_row_right: '╡',
    mid_left: '│',
    mid_vertical: '│',
    mid_right: '│',
    row_left: '├',
    row_horizontal: '─',
    row_cross: '┼',
    row_right: '┤',
    foot_row_left: '├',
    foot_row_horizontal: '─',
    foot_row_cross: '┼',
    foot_row_right: '┤',
    foot_left: '│',
    foot_vertical: '│',
    foot_right: '│',
    bottom_left: '└',
    bottom: '─',
    bottom_divider: '┴',
    bottom_right: '┘',
    ascii: false,
};

/// Minimal box with sparse borders.
pub const MINIMAL: Box = Box {
    top_left: ' ',
    top: ' ',
    top_divider: '╷',
    top_right: ' ',
    head_left: ' ',
    head_vertical: '│',
    head_right: ' ',
    head_row_left: '╶',
    head_row_horizontal: '─',
    head_row_cross: '┼',
    head_row_right: '╴',
    mid_left: ' ',
    mid_vertical: '│',
    mid_right: ' ',
    row_left: '╶',
    row_horizontal: '─',
    row_cross: '┼',
    row_right: '╴',
    foot_row_left: '╶',
    foot_row_horizontal: '─',
    foot_row_cross: '┼',
    foot_row_right: '╴',
    foot_left: ' ',
    foot_vertical: '│',
    foot_right: ' ',
    bottom_left: ' ',
    bottom: ' ',
    bottom_divider: '╵',
    bottom_right: ' ',
    ascii: false,
};

/// Minimal box with heavy header separator.
pub const MINIMAL_HEAVY_HEAD: Box = Box {
    top_left: ' ',
    top: ' ',
    top_divider: '╷',
    top_right: ' ',
    head_left: ' ',
    head_vertical: '│',
    head_right: ' ',
    head_row_left: '╺',
    head_row_horizontal: '━',
    head_row_cross: '┿',
    head_row_right: '╸',
    mid_left: ' ',
    mid_vertical: '│',
    mid_right: ' ',
    row_left: '╶',
    row_horizontal: '─',
    row_cross: '┼',
    row_right: '╴',
    foot_row_left: '╶',
    foot_row_horizontal: '─',
    foot_row_cross: '┼',
    foot_row_right: '╴',
    foot_left: ' ',
    foot_vertical: '│',
    foot_right: ' ',
    bottom_left: ' ',
    bottom: ' ',
    bottom_divider: '╵',
    bottom_right: ' ',
    ascii: false,
};

/// Minimal box with double-line header separator.
pub const MINIMAL_DOUBLE_HEAD: Box = Box {
    top_left: ' ',
    top: ' ',
    top_divider: '╷',
    top_right: ' ',
    head_left: ' ',
    head_vertical: '│',
    head_right: ' ',
    head_row_left: ' ',
    head_row_horizontal: '═',
    head_row_cross: '╪',
    head_row_right: ' ',
    mid_left: ' ',
    mid_vertical: '│',
    mid_right: ' ',
    row_left: ' ',
    row_horizontal: '─',
    row_cross: '┼',
    row_right: ' ',
    foot_row_left: ' ',
    foot_row_horizontal: '─',
    foot_row_cross: '┼',
    foot_row_right: ' ',
    foot_left: ' ',
    foot_vertical: '│',
    foot_right: ' ',
    bottom_left: ' ',
    bottom: ' ',
    bottom_divider: '╵',
    bottom_right: ' ',
    ascii: false,
};

/// Simple box with only header and footer separators.
pub const SIMPLE: Box = Box {
    top_left: ' ',
    top: ' ',
    top_divider: ' ',
    top_right: ' ',
    head_left: ' ',
    head_vertical: ' ',
    head_right: ' ',
    head_row_left: ' ',
    head_row_horizontal: '─',
    head_row_cross: '─',
    head_row_right: ' ',
    mid_left: ' ',
    mid_vertical: ' ',
    mid_right: ' ',
    row_left: ' ',
    row_horizontal: ' ',
    row_cross: ' ',
    row_right: ' ',
    foot_row_left: ' ',
    foot_row_horizontal: '─',
    foot_row_cross: '─',
    foot_row_right: ' ',
    foot_left: ' ',
    foot_vertical: ' ',
    foot_right: ' ',
    bottom_left: ' ',
    bottom: ' ',
    bottom_divider: ' ',
    bottom_right: ' ',
    ascii: false,
};

/// Simple box with only header separator.
pub const SIMPLE_HEAD: Box = Box {
    top_left: ' ',
    top: ' ',
    top_divider: ' ',
    top_right: ' ',
    head_left: ' ',
    head_vertical: ' ',
    head_right: ' ',
    head_row_left: ' ',
    head_row_horizontal: '─',
    head_row_cross: '─',
    head_row_right: ' ',
    mid_left: ' ',
    mid_vertical: ' ',
    mid_right: ' ',
    row_left: ' ',
    row_horizontal: ' ',
    row_cross: ' ',
    row_right: ' ',
    foot_row_left: ' ',
    foot_row_horizontal: ' ',
    foot_row_cross: ' ',
    foot_row_right: ' ',
    foot_left: ' ',
    foot_vertical: ' ',
    foot_right: ' ',
    bottom_left: ' ',
    bottom: ' ',
    bottom_divider: ' ',
    bottom_right: ' ',
    ascii: false,
};

/// Simple box with heavy (thick) separators.
pub const SIMPLE_HEAVY: Box = Box {
    top_left: ' ',
    top: ' ',
    top_divider: ' ',
    top_right: ' ',
    head_left: ' ',
    head_vertical: ' ',
    head_right: ' ',
    head_row_left: ' ',
    head_row_horizontal: '━',
    head_row_cross: '━',
    head_row_right: ' ',
    mid_left: ' ',
    mid_vertical: ' ',
    mid_right: ' ',
    row_left: ' ',
    row_horizontal: ' ',
    row_cross: ' ',
    row_right: ' ',
    foot_row_left: ' ',
    foot_row_horizontal: '━',
    foot_row_cross: '━',
    foot_row_right: ' ',
    foot_left: ' ',
    foot_vertical: ' ',
    foot_right: ' ',
    bottom_left: ' ',
    bottom: ' ',
    bottom_divider: ' ',
    bottom_right: ' ',
    ascii: false,
};

/// Horizontals-only box with lines at top, head, row, foot, and bottom.
pub const HORIZONTALS: Box = Box {
    top_left: ' ',
    top: '─',
    top_divider: '─',
    top_right: ' ',
    head_left: ' ',
    head_vertical: ' ',
    head_right: ' ',
    head_row_left: ' ',
    head_row_horizontal: '─',
    head_row_cross: '─',
    head_row_right: ' ',
    mid_left: ' ',
    mid_vertical: ' ',
    mid_right: ' ',
    row_left: ' ',
    row_horizontal: '─',
    row_cross: '─',
    row_right: ' ',
    foot_row_left: ' ',
    foot_row_horizontal: '─',
    foot_row_cross: '─',
    foot_row_right: ' ',
    foot_left: ' ',
    foot_vertical: ' ',
    foot_right: ' ',
    bottom_left: ' ',
    bottom: '─',
    bottom_divider: '─',
    bottom_right: ' ',
    ascii: false,
};

/// Rounded box with curved corners.
pub const ROUNDED: Box = Box {
    top_left: '╭',
    top: '─',
    top_divider: '┬',
    top_right: '╮',
    head_left: '│',
    head_vertical: '│',
    head_right: '│',
    head_row_left: '├',
    head_row_horizontal: '─',
    head_row_cross: '┼',
    head_row_right: '┤',
    mid_left: '│',
    mid_vertical: '│',
    mid_right: '│',
    row_left: '├',
    row_horizontal: '─',
    row_cross: '┼',
    row_right: '┤',
    foot_row_left: '├',
    foot_row_horizontal: '─',
    foot_row_cross: '┼',
    foot_row_right: '┤',
    foot_left: '│',
    foot_vertical: '│',
    foot_right: '│',
    bottom_left: '╰',
    bottom: '─',
    bottom_divider: '┴',
    bottom_right: '╯',
    ascii: false,
};

/// Heavy box with thick lines throughout.
pub const HEAVY: Box = Box {
    top_left: '┏',
    top: '━',
    top_divider: '┳',
    top_right: '┓',
    head_left: '┃',
    head_vertical: '┃',
    head_right: '┃',
    head_row_left: '┣',
    head_row_horizontal: '━',
    head_row_cross: '╋',
    head_row_right: '┫',
    mid_left: '┃',
    mid_vertical: '┃',
    mid_right: '┃',
    row_left: '┣',
    row_horizontal: '━',
    row_cross: '╋',
    row_right: '┫',
    foot_row_left: '┣',
    foot_row_horizontal: '━',
    foot_row_cross: '╋',
    foot_row_right: '┫',
    foot_left: '┃',
    foot_vertical: '┃',
    foot_right: '┃',
    bottom_left: '┗',
    bottom: '━',
    bottom_divider: '┻',
    bottom_right: '┛',
    ascii: false,
};

/// Heavy edge box (thick outer, thin inner lines).
pub const HEAVY_EDGE: Box = Box {
    top_left: '┏',
    top: '━',
    top_divider: '┯',
    top_right: '┓',
    head_left: '┃',
    head_vertical: '│',
    head_right: '┃',
    head_row_left: '┠',
    head_row_horizontal: '─',
    head_row_cross: '┼',
    head_row_right: '┨',
    mid_left: '┃',
    mid_vertical: '│',
    mid_right: '┃',
    row_left: '┠',
    row_horizontal: '─',
    row_cross: '┼',
    row_right: '┨',
    foot_row_left: '┠',
    foot_row_horizontal: '─',
    foot_row_cross: '┼',
    foot_row_right: '┨',
    foot_left: '┃',
    foot_vertical: '│',
    foot_right: '┃',
    bottom_left: '┗',
    bottom: '━',
    bottom_divider: '┷',
    bottom_right: '┛',
    ascii: false,
};

/// Heavy head box (thick header, thin body).
pub const HEAVY_HEAD: Box = Box {
    top_left: '┏',
    top: '━',
    top_divider: '┳',
    top_right: '┓',
    head_left: '┃',
    head_vertical: '┃',
    head_right: '┃',
    head_row_left: '┡',
    head_row_horizontal: '━',
    head_row_cross: '╇',
    head_row_right: '┩',
    mid_left: '│',
    mid_vertical: '│',
    mid_right: '│',
    row_left: '├',
    row_horizontal: '─',
    row_cross: '┼',
    row_right: '┤',
    foot_row_left: '├',
    foot_row_horizontal: '─',
    foot_row_cross: '┼',
    foot_row_right: '┤',
    foot_left: '│',
    foot_vertical: '│',
    foot_right: '│',
    bottom_left: '└',
    bottom: '─',
    bottom_divider: '┴',
    bottom_right: '┘',
    ascii: false,
};

/// Double-line box throughout.
pub const DOUBLE: Box = Box {
    top_left: '╔',
    top: '═',
    top_divider: '╦',
    top_right: '╗',
    head_left: '║',
    head_vertical: '║',
    head_right: '║',
    head_row_left: '╠',
    head_row_horizontal: '═',
    head_row_cross: '╬',
    head_row_right: '╣',
    mid_left: '║',
    mid_vertical: '║',
    mid_right: '║',
    row_left: '╠',
    row_horizontal: '═',
    row_cross: '╬',
    row_right: '╣',
    foot_row_left: '╠',
    foot_row_horizontal: '═',
    foot_row_cross: '╬',
    foot_row_right: '╣',
    foot_left: '║',
    foot_vertical: '║',
    foot_right: '║',
    bottom_left: '╚',
    bottom: '═',
    bottom_divider: '╩',
    bottom_right: '╝',
    ascii: false,
};

/// Double edge box (double outer, single inner lines).
pub const DOUBLE_EDGE: Box = Box {
    top_left: '╔',
    top: '═',
    top_divider: '╤',
    top_right: '╗',
    head_left: '║',
    head_vertical: '│',
    head_right: '║',
    head_row_left: '╟',
    head_row_horizontal: '─',
    head_row_cross: '┼',
    head_row_right: '╢',
    mid_left: '║',
    mid_vertical: '│',
    mid_right: '║',
    row_left: '╟',
    row_horizontal: '─',
    row_cross: '┼',
    row_right: '╢',
    foot_row_left: '╟',
    foot_row_horizontal: '─',
    foot_row_cross: '┼',
    foot_row_right: '╢',
    foot_left: '║',
    foot_vertical: '│',
    foot_right: '║',
    bottom_left: '╚',
    bottom: '═',
    bottom_divider: '╧',
    bottom_right: '╝',
    ascii: false,
};

/// Markdown-compatible table format.
pub const MARKDOWN: Box = Box {
    top_left: ' ',
    top: ' ',
    top_divider: ' ',
    top_right: ' ',
    head_left: '|',
    head_vertical: '|',
    head_right: '|',
    head_row_left: '|',
    head_row_horizontal: '-',
    head_row_cross: '|',
    head_row_right: '|',
    mid_left: '|',
    mid_vertical: '|',
    mid_right: '|',
    row_left: '|',
    row_horizontal: '-',
    row_cross: '|',
    row_right: '|',
    foot_row_left: '|',
    foot_row_horizontal: '-',
    foot_row_cross: '|',
    foot_row_right: '|',
    foot_left: '|',
    foot_vertical: '|',
    foot_right: '|',
    bottom_left: ' ',
    bottom: ' ',
    bottom_divider: ' ',
    bottom_right: ' ',
    ascii: true,
};

impl Box {
    /// Substitute this box for another if it won't render due to platform issues.
    ///
    /// # Arguments
    ///
    /// * `legacy_windows` - If true, substitute boxes that don't render well with
    ///   legacy Windows console (raster fonts).
    /// * `ascii_only` - If true, substitute non-ASCII boxes with ASCII equivalent.
    ///
    /// # Returns
    ///
    /// A compatible `Box`. For known box constants that need substitution, returns
    /// the appropriate fallback. For custom boxes, returns `self` unchanged unless
    /// `ascii_only` is true and `self.ascii` is false, in which case returns `ASCII`.
    pub fn substitute(&self, legacy_windows: bool, ascii_only: bool) -> Box {
        let mut result = *self;

        if legacy_windows {
            // Only substitute known box constants that have rendering issues
            // Group boxes by their substitution target
            result =
                if *self == ROUNDED || *self == HEAVY || *self == HEAVY_EDGE || *self == HEAVY_HEAD
                {
                    SQUARE
                } else if *self == MINIMAL_HEAVY_HEAD {
                    MINIMAL
                } else if *self == SIMPLE_HEAVY {
                    SIMPLE
                } else {
                    result
                };
        }

        if ascii_only && !result.ascii {
            return ASCII;
        }

        result
    }

    /// If this box uses special characters for the header borders, return the
    /// equivalent box without special header characters.
    ///
    /// # Returns
    ///
    /// The equivalent plain-headed `Box`, or `self` if already plain.
    /// For custom boxes, returns `self` unchanged.
    pub fn get_plain_headed_box(&self) -> Box {
        // Group boxes by their substitution target
        if *self == HEAVY_HEAD || *self == SQUARE_DOUBLE_HEAD {
            SQUARE
        } else if *self == MINIMAL_DOUBLE_HEAD || *self == MINIMAL_HEAVY_HEAD {
            MINIMAL
        } else if *self == ASCII_DOUBLE_HEAD {
            ASCII2
        } else {
            *self
        }
    }

    /// Generate the top border of a box.
    ///
    /// # Arguments
    ///
    /// * `widths` - Slice of column widths.
    ///
    /// # Returns
    ///
    /// A string representing the top border.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::r#box::SQUARE;
    ///
    /// let top = SQUARE.get_top(&[5, 10, 5]);
    /// assert_eq!(top, "┌─────┬──────────┬─────┐");
    /// ```
    pub fn get_top(&self, widths: &[usize]) -> String {
        let mut parts = String::new();
        parts.push(self.top_left);

        for (i, &width) in widths.iter().enumerate() {
            for _ in 0..width {
                parts.push(self.top);
            }
            if i < widths.len() - 1 {
                parts.push(self.top_divider);
            }
        }

        parts.push(self.top_right);
        parts
    }

    /// Generate a row separator line.
    ///
    /// # Arguments
    ///
    /// * `widths` - Slice of column widths.
    /// * `level` - The type of row separator (Head, Row, Foot, or Mid).
    /// * `edge` - Whether to include edge characters (left and right borders).
    ///
    /// # Returns
    ///
    /// A string representing the row separator.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::r#box::{SQUARE, RowLevel};
    ///
    /// let row = SQUARE.get_row(&[5, 10], RowLevel::Head, true);
    /// assert_eq!(row, "├─────┼──────────┤");
    /// ```
    pub fn get_row(&self, widths: &[usize], level: RowLevel, edge: bool) -> String {
        let (left, horizontal, cross, right) = match level {
            RowLevel::Head => (
                self.head_row_left,
                self.head_row_horizontal,
                self.head_row_cross,
                self.head_row_right,
            ),
            RowLevel::Row => (
                self.row_left,
                self.row_horizontal,
                self.row_cross,
                self.row_right,
            ),
            RowLevel::Foot => (
                self.foot_row_left,
                self.foot_row_horizontal,
                self.foot_row_cross,
                self.foot_row_right,
            ),
            RowLevel::Mid => (self.mid_left, ' ', self.mid_vertical, self.mid_right),
        };

        let mut parts = String::new();

        if edge {
            parts.push(left);
        }

        for (i, &width) in widths.iter().enumerate() {
            for _ in 0..width {
                parts.push(horizontal);
            }
            if i < widths.len() - 1 {
                parts.push(cross);
            }
        }

        if edge {
            parts.push(right);
        }

        parts
    }

    /// Generate the bottom border of a box.
    ///
    /// # Arguments
    ///
    /// * `widths` - Slice of column widths.
    ///
    /// # Returns
    ///
    /// A string representing the bottom border.
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::r#box::SQUARE;
    ///
    /// let bottom = SQUARE.get_bottom(&[5, 10, 5]);
    /// assert_eq!(bottom, "└─────┴──────────┴─────┘");
    /// ```
    pub fn get_bottom(&self, widths: &[usize]) -> String {
        let mut parts = String::new();
        parts.push(self.bottom_left);

        for (i, &width) in widths.iter().enumerate() {
            for _ in 0..width {
                parts.push(self.bottom);
            }
            if i < widths.len() - 1 {
                parts.push(self.bottom_divider);
            }
        }

        parts.push(self.bottom_right);
        parts
    }

    /// Get a string for the top edge of a simple box (single column).
    ///
    /// This is a convenience method for backward compatibility with the
    /// original `BoxChars` implementation.
    ///
    /// # Arguments
    ///
    /// * `width` - The width of the box interior.
    ///
    /// # Returns
    ///
    /// A string representing the top edge.
    pub fn top_edge(&self, width: usize) -> String {
        self.get_top(&[width])
    }

    /// Get a string for the bottom edge of a simple box (single column).
    ///
    /// This is a convenience method for backward compatibility with the
    /// original `BoxChars` implementation.
    ///
    /// # Arguments
    ///
    /// * `width` - The width of the box interior.
    ///
    /// # Returns
    ///
    /// A string representing the bottom edge.
    pub fn bottom_edge(&self, width: usize) -> String {
        self.get_bottom(&[width])
    }
}

// ============================================================================
// Backward Compatibility
// ============================================================================

/// Deprecated alias for `Box`.
///
/// This type alias is provided for backward compatibility with code that
/// used the original `BoxChars` struct.
#[deprecated(since = "0.2.0", note = "Use `Box` instead")]
pub type BoxChars = Box;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_top_edge() {
        assert_eq!(ROUNDED.top_edge(3), "╭───╮");
        assert_eq!(ROUNDED.bottom_edge(3), "╰───╯");
    }

    #[test]
    fn test_ascii_box() {
        assert_eq!(ASCII.top_edge(3), "+---+");
        assert_eq!(ASCII.bottom_edge(3), "+---+");
    }

    #[test]
    fn test_square_box() {
        assert_eq!(SQUARE.top_edge(5), "┌─────┐");
        assert_eq!(SQUARE.bottom_edge(5), "└─────┘");
    }

    #[test]
    fn test_heavy_box() {
        assert_eq!(HEAVY.top_edge(4), "┏━━━━┓");
        assert_eq!(HEAVY.bottom_edge(4), "┗━━━━┛");
    }

    #[test]
    fn test_double_box() {
        assert_eq!(DOUBLE.top_edge(3), "╔═══╗");
        assert_eq!(DOUBLE.bottom_edge(3), "╚═══╝");
    }

    #[test]
    fn test_get_top_multiple_columns() {
        assert_eq!(SQUARE.get_top(&[3, 5, 2]), "┌───┬─────┬──┐");
        // ASCII uses '-' for both top and top_divider, so no visible column separators
        assert_eq!(ASCII.get_top(&[3, 5, 2]), "+------------+");
        // ASCII2 uses '+' for top_divider, showing column separators
        assert_eq!(ASCII2.get_top(&[3, 5, 2]), "+---+-----+--+");
        assert_eq!(HEAVY.get_top(&[2, 2]), "┏━━┳━━┓");
    }

    #[test]
    fn test_get_bottom_multiple_columns() {
        assert_eq!(SQUARE.get_bottom(&[3, 5, 2]), "└───┴─────┴──┘");
        // ASCII uses '-' for both bottom and bottom_divider
        assert_eq!(ASCII.get_bottom(&[3, 5, 2]), "+------------+");
        // ASCII2 uses '+' for bottom_divider
        assert_eq!(ASCII2.get_bottom(&[3, 5, 2]), "+---+-----+--+");
        assert_eq!(DOUBLE.get_bottom(&[4, 4]), "╚════╩════╝");
    }

    #[test]
    fn test_get_row_head() {
        assert_eq!(SQUARE.get_row(&[3, 5], RowLevel::Head, true), "├───┼─────┤");
        assert_eq!(HEAVY.get_row(&[3, 5], RowLevel::Head, true), "┣━━━╋━━━━━┫");
    }

    #[test]
    fn test_get_row_regular() {
        assert_eq!(SQUARE.get_row(&[4, 4], RowLevel::Row, true), "├────┼────┤");
        assert_eq!(ASCII.get_row(&[3, 3], RowLevel::Row, true), "|---+---|");
    }

    #[test]
    fn test_get_row_foot() {
        assert_eq!(SQUARE.get_row(&[3, 3], RowLevel::Foot, true), "├───┼───┤");
    }

    #[test]
    fn test_get_row_mid() {
        assert_eq!(SQUARE.get_row(&[3, 3], RowLevel::Mid, true), "│   │   │");
        assert_eq!(HEAVY.get_row(&[2, 2], RowLevel::Mid, true), "┃  ┃  ┃");
    }

    #[test]
    fn test_get_row_no_edge() {
        assert_eq!(SQUARE.get_row(&[3, 3], RowLevel::Row, false), "───┼───");
        assert_eq!(ASCII.get_row(&[2, 2], RowLevel::Row, false), "--+--");
    }

    #[test]
    fn test_substitute_legacy_windows() {
        // ROUNDED should become SQUARE on legacy Windows
        let result = ROUNDED.substitute(true, false);
        assert_eq!(result, SQUARE);

        // HEAVY should become SQUARE
        let result = HEAVY.substitute(true, false);
        assert_eq!(result, SQUARE);

        // MINIMAL_HEAVY_HEAD should become MINIMAL
        let result = MINIMAL_HEAVY_HEAD.substitute(true, false);
        assert_eq!(result, MINIMAL);

        // SIMPLE_HEAVY should become SIMPLE
        let result = SIMPLE_HEAVY.substitute(true, false);
        assert_eq!(result, SIMPLE);

        // SQUARE stays SQUARE
        let result = SQUARE.substitute(true, false);
        assert_eq!(result, SQUARE);
    }

    #[test]
    fn test_substitute_ascii_only() {
        // Non-ASCII boxes should become ASCII
        let result = SQUARE.substitute(false, true);
        assert_eq!(result, ASCII);

        let result = ROUNDED.substitute(false, true);
        assert_eq!(result, ASCII);

        // ASCII boxes stay ASCII
        let result = ASCII.substitute(false, true);
        assert_eq!(result, ASCII);

        let result = ASCII2.substitute(false, true);
        assert_eq!(result, ASCII2);

        let result = MARKDOWN.substitute(false, true);
        assert_eq!(result, MARKDOWN);
    }

    #[test]
    fn test_substitute_both_flags() {
        // Legacy Windows + ASCII only: ROUNDED -> SQUARE -> ASCII
        let result = ROUNDED.substitute(true, true);
        assert_eq!(result, ASCII);
    }

    #[test]
    fn test_substitute_custom_box() {
        // Custom boxes should be returned unchanged when not needing substitution
        let custom = Box {
            top_left: '*',
            top: '*',
            top_divider: '*',
            top_right: '*',
            head_left: '*',
            head_vertical: '*',
            head_right: '*',
            head_row_left: '*',
            head_row_horizontal: '*',
            head_row_cross: '*',
            head_row_right: '*',
            mid_left: '*',
            mid_vertical: '*',
            mid_right: '*',
            row_left: '*',
            row_horizontal: '*',
            row_cross: '*',
            row_right: '*',
            foot_row_left: '*',
            foot_row_horizontal: '*',
            foot_row_cross: '*',
            foot_row_right: '*',
            foot_left: '*',
            foot_vertical: '*',
            foot_right: '*',
            bottom_left: '*',
            bottom: '*',
            bottom_divider: '*',
            bottom_right: '*',
            ascii: true,
        };

        // Custom box should be unchanged
        let result = custom.substitute(false, false);
        assert_eq!(result, custom);

        // Custom ASCII box stays custom even with legacy_windows
        let result = custom.substitute(true, false);
        assert_eq!(result, custom);

        // Custom ASCII box stays custom with ascii_only (since it is ASCII)
        let result = custom.substitute(false, true);
        assert_eq!(result, custom);

        // Non-ASCII custom box becomes ASCII when ascii_only=true
        let custom_unicode = Box {
            ascii: false,
            ..custom
        };
        let result = custom_unicode.substitute(false, true);
        assert_eq!(result, ASCII);
    }

    #[test]
    fn test_get_plain_headed_box() {
        assert_eq!(HEAVY_HEAD.get_plain_headed_box(), SQUARE);
        assert_eq!(SQUARE_DOUBLE_HEAD.get_plain_headed_box(), SQUARE);
        assert_eq!(MINIMAL_DOUBLE_HEAD.get_plain_headed_box(), MINIMAL);
        assert_eq!(MINIMAL_HEAVY_HEAD.get_plain_headed_box(), MINIMAL);
        assert_eq!(ASCII_DOUBLE_HEAD.get_plain_headed_box(), ASCII2);

        // Plain boxes return themselves
        assert_eq!(SQUARE.get_plain_headed_box(), SQUARE);
        assert_eq!(ASCII.get_plain_headed_box(), ASCII);
        assert_eq!(ROUNDED.get_plain_headed_box(), ROUNDED);
    }

    #[test]
    fn test_get_plain_headed_box_custom() {
        // Custom boxes should be returned unchanged
        let custom = Box {
            top_left: '#',
            top: '#',
            top_divider: '#',
            top_right: '#',
            head_left: '#',
            head_vertical: '#',
            head_right: '#',
            head_row_left: '#',
            head_row_horizontal: '#',
            head_row_cross: '#',
            head_row_right: '#',
            mid_left: '#',
            mid_vertical: '#',
            mid_right: '#',
            row_left: '#',
            row_horizontal: '#',
            row_cross: '#',
            row_right: '#',
            foot_row_left: '#',
            foot_row_horizontal: '#',
            foot_row_cross: '#',
            foot_row_right: '#',
            foot_left: '#',
            foot_vertical: '#',
            foot_right: '#',
            bottom_left: '#',
            bottom: '#',
            bottom_divider: '#',
            bottom_right: '#',
            ascii: true,
        };
        assert_eq!(custom.get_plain_headed_box(), custom);
    }

    #[test]
    fn test_all_boxes_defined() {
        // Verify all 19 boxes are defined and have the expected ascii flag
        assert!(ASCII.ascii);
        assert!(ASCII2.ascii);
        assert!(ASCII_DOUBLE_HEAD.ascii);
        assert!(!SQUARE.ascii);
        assert!(!SQUARE_DOUBLE_HEAD.ascii);
        assert!(!MINIMAL.ascii);
        assert!(!MINIMAL_HEAVY_HEAD.ascii);
        assert!(!MINIMAL_DOUBLE_HEAD.ascii);
        assert!(!SIMPLE.ascii);
        assert!(!SIMPLE_HEAD.ascii);
        assert!(!SIMPLE_HEAVY.ascii);
        assert!(!HORIZONTALS.ascii);
        assert!(!ROUNDED.ascii);
        assert!(!HEAVY.ascii);
        assert!(!HEAVY_EDGE.ascii);
        assert!(!HEAVY_HEAD.ascii);
        assert!(!DOUBLE.ascii);
        assert!(!DOUBLE_EDGE.ascii);
        assert!(MARKDOWN.ascii);
    }

    #[test]
    fn test_box_equality() {
        // Each box should equal itself
        assert_eq!(SQUARE, SQUARE);
        assert_eq!(ROUNDED, ROUNDED);

        // Different boxes should not be equal
        assert_ne!(SQUARE, ROUNDED);
        assert_ne!(ASCII, MARKDOWN);
    }

    #[test]
    fn test_single_column() {
        // Single column should work without dividers
        assert_eq!(SQUARE.get_top(&[5]), "┌─────┐");
        assert_eq!(SQUARE.get_bottom(&[5]), "└─────┘");
        assert_eq!(SQUARE.get_row(&[5], RowLevel::Row, true), "├─────┤");
    }

    #[test]
    fn test_empty_widths() {
        // Empty widths should produce minimal output
        assert_eq!(SQUARE.get_top(&[]), "┌┐");
        assert_eq!(SQUARE.get_bottom(&[]), "└┘");
        assert_eq!(SQUARE.get_row(&[], RowLevel::Row, true), "├┤");
    }

    #[test]
    fn test_zero_width_column() {
        // Zero-width columns are valid
        assert_eq!(SQUARE.get_top(&[0, 3, 0]), "┌┬───┬┐");
    }

    #[test]
    fn test_markdown_box() {
        // Markdown has space for top/bottom (no visible borders)
        // top_left=' ', top=' ', top_right=' '
        // For width 5: ' ' + 5*' ' + ' ' = 7 spaces
        assert_eq!(MARKDOWN.get_top(&[5]), "       ");
        assert_eq!(
            MARKDOWN.get_row(&[5, 5], RowLevel::Head, true),
            "|-----|-----|"
        );
        assert_eq!(MARKDOWN.get_bottom(&[5]), "       ");
        // With two columns of width 5: ' ' + '     ' + ' ' + '     ' + ' ' = 13 chars
        assert_eq!(MARKDOWN.get_top(&[5, 5]), "             ");
    }

    #[test]
    fn test_horizontals_box() {
        // HORIZONTALS has lines at top and bottom, no corner chars
        // top_left=' ', top='─', top_divider='─', top_right=' '
        assert_eq!(HORIZONTALS.get_top(&[5]), " ───── ");
        assert_eq!(HORIZONTALS.get_bottom(&[5]), " ───── ");
    }

    #[test]
    fn test_minimal_box() {
        // MINIMAL has mostly spaces, dividers only appear between columns
        // top_left=' ', top=' ', top_divider='╷', top_right=' '
        // For width 3: ' ' + 3*' ' + ' ' = 5 spaces (no divider for single column)
        assert_eq!(MINIMAL.get_top(&[3]), "     ");
        assert_eq!(MINIMAL.get_bottom(&[3]), "     ");
        // With two columns: ' ' + '   ' + '╷' + '   ' + ' '
        assert_eq!(MINIMAL.get_top(&[3, 3]), "    ╷    ");
    }

    #[test]
    fn test_simple_box() {
        // SIMPLE has mostly spaces
        assert_eq!(SIMPLE.get_top(&[5]), "       ");
        assert_eq!(SIMPLE.get_row(&[5], RowLevel::Head, true), " ───── ");
    }

    #[test]
    fn test_double_headed_boxes() {
        // Verify the header row uses different characters
        assert_eq!(
            SQUARE_DOUBLE_HEAD.get_row(&[3], RowLevel::Head, true),
            "╞═══╡"
        );
        assert_eq!(
            ASCII_DOUBLE_HEAD.get_row(&[3], RowLevel::Head, true),
            "+===+"
        );
    }

    #[test]
    fn test_heavy_variations() {
        // HEAVY_HEAD has thick header, thin body
        assert_eq!(HEAVY_HEAD.get_top(&[3]), "┏━━━┓");
        assert_eq!(HEAVY_HEAD.get_row(&[3], RowLevel::Row, true), "├───┤");

        // HEAVY_EDGE has thick edges, thin interior
        assert_eq!(HEAVY_EDGE.get_top(&[3, 3]), "┏━━━┯━━━┓");
        assert_eq!(
            HEAVY_EDGE.get_row(&[3, 3], RowLevel::Row, true),
            "┠───┼───┨"
        );
    }
}
