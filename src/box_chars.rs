//! Box drawing characters for borders and panels.

/// A set of box drawing characters for creating borders.
#[derive(Debug, Clone, Copy)]
pub struct BoxChars {
    /// Top-left corner.
    pub top_left: char,
    /// Top-right corner.
    pub top_right: char,
    /// Bottom-left corner.
    pub bottom_left: char,
    /// Bottom-right corner.
    pub bottom_right: char,
    /// Horizontal line.
    pub horizontal: char,
    /// Vertical line.
    pub vertical: char,
    /// Left T-junction.
    pub vertical_right: char,
    /// Right T-junction.
    pub vertical_left: char,
    /// Top T-junction.
    pub horizontal_down: char,
    /// Bottom T-junction.
    pub horizontal_up: char,
    /// Cross junction.
    pub cross: char,
}

impl BoxChars {
    /// Get a string for the top edge of a box.
    pub fn top_edge(&self, width: usize) -> String {
        format!(
            "{}{}{}",
            self.top_left,
            self.horizontal.to_string().repeat(width),
            self.top_right
        )
    }

    /// Get a string for the bottom edge of a box.
    pub fn bottom_edge(&self, width: usize) -> String {
        format!(
            "{}{}{}",
            self.bottom_left,
            self.horizontal.to_string().repeat(width),
            self.bottom_right
        )
    }
}

/// ASCII box characters.
pub const ASCII: BoxChars = BoxChars {
    top_left: '+',
    top_right: '+',
    bottom_left: '+',
    bottom_right: '+',
    horizontal: '-',
    vertical: '|',
    vertical_right: '+',
    vertical_left: '+',
    horizontal_down: '+',
    horizontal_up: '+',
    cross: '+',
};

/// Simple Unicode box (thin lines).
pub const SQUARE: BoxChars = BoxChars {
    top_left: '┌',
    top_right: '┐',
    bottom_left: '└',
    bottom_right: '┘',
    horizontal: '─',
    vertical: '│',
    vertical_right: '├',
    vertical_left: '┤',
    horizontal_down: '┬',
    horizontal_up: '┴',
    cross: '┼',
};

/// Rounded corners box.
pub const ROUNDED: BoxChars = BoxChars {
    top_left: '╭',
    top_right: '╮',
    bottom_left: '╰',
    bottom_right: '╯',
    horizontal: '─',
    vertical: '│',
    vertical_right: '├',
    vertical_left: '┤',
    horizontal_down: '┬',
    horizontal_up: '┴',
    cross: '┼',
};

/// Heavy (thick) box characters.
pub const HEAVY: BoxChars = BoxChars {
    top_left: '┏',
    top_right: '┓',
    bottom_left: '┗',
    bottom_right: '┛',
    horizontal: '━',
    vertical: '┃',
    vertical_right: '┣',
    vertical_left: '┫',
    horizontal_down: '┳',
    horizontal_up: '┻',
    cross: '╋',
};

/// Double-line box characters.
pub const DOUBLE: BoxChars = BoxChars {
    top_left: '╔',
    top_right: '╗',
    bottom_left: '╚',
    bottom_right: '╝',
    horizontal: '═',
    vertical: '║',
    vertical_right: '╠',
    vertical_left: '╣',
    horizontal_down: '╦',
    horizontal_up: '╩',
    cross: '╬',
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_edges() {
        assert_eq!(ROUNDED.top_edge(3), "╭───╮");
        assert_eq!(ROUNDED.bottom_edge(3), "╰───╯");
    }

    #[test]
    fn test_ascii_box() {
        assert_eq!(ASCII.top_edge(3), "+---+");
    }
}
