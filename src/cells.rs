//! Cell width calculation for terminal display.
//!
//! Wraps `unicode-width` to calculate how many terminal cells a string occupies.
//! CJK characters take 2 cells, most other characters take 1, some take 0.

use unicode_width::UnicodeWidthStr;

/// Calculate the display width of a string in terminal cells.
///
/// # Example
///
/// ```
/// use rich_rs::cell_len;
///
/// assert_eq!(cell_len("hello"), 5);
/// assert_eq!(cell_len("你好"), 4);  // CJK characters are 2 cells wide
/// ```
pub fn cell_len(text: &str) -> usize {
    text.width()
}

/// Calculate the display width of a single character.
pub fn char_width(c: char) -> usize {
    unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
}

/// Truncate a string to fit within a given cell width.
///
/// Returns the truncated string and the actual width used.
pub fn set_cell_size(text: &str, width: usize) -> (String, usize) {
    let mut result = String::new();
    let mut current_width = 0;

    for c in text.chars() {
        let w = char_width(c);
        if current_width + w > width {
            break;
        }
        result.push(c);
        current_width += w;
    }

    (result, current_width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_len_ascii() {
        assert_eq!(cell_len("hello"), 5);
        assert_eq!(cell_len(""), 0);
    }

    #[test]
    fn test_cell_len_cjk() {
        assert_eq!(cell_len("你好"), 4);
        assert_eq!(cell_len("日本語"), 6);
    }

    #[test]
    fn test_set_cell_size() {
        let (truncated, width) = set_cell_size("hello", 3);
        assert_eq!(truncated, "hel");
        assert_eq!(width, 3);
    }
}
