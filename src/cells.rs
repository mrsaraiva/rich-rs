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

/// Set the length of a string to fit within given number of cells.
///
/// If the text is shorter than `width`, it is padded with spaces.
/// If the text is longer, it is truncated. If truncation occurs at a
/// double-width character boundary, a space is added to reach the exact width.
///
/// # Example
///
/// ```
/// use rich_rs::set_cell_size;
///
/// assert_eq!(set_cell_size("hello", 10), "hello     ");
/// assert_eq!(set_cell_size("hello world", 5), "hello");
/// assert_eq!(set_cell_size("你好世界", 5), "你好 ");  // Truncated at double-width boundary
/// ```
pub fn set_cell_size(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let text_width = cell_len(text);

    if text_width == width {
        return text.to_string();
    }

    if text_width < width {
        // Pad with spaces
        let mut result = text.to_string();
        result.push_str(&" ".repeat(width - text_width));
        return result;
    }

    // Truncate: iterate through characters until we reach or exceed the width
    let mut result = String::new();
    let mut current_width = 0;

    for c in text.chars() {
        let w = char_width(c);
        if current_width + w > width {
            // Character doesn't fit; if we're one short and it's a double-width char, add space
            if current_width < width {
                result.push(' ');
            }
            break;
        }
        result.push(c);
        current_width += w;
    }

    result
}

/// Split text into individual character graphemes.
///
/// This is a simplified version that splits by characters. For proper
/// Unicode grapheme cluster splitting, `unicode-segmentation` would be needed.
///
/// # Example
///
/// ```
/// use rich_rs::split_graphemes;
///
/// assert_eq!(split_graphemes("hello"), vec!["h", "e", "l", "l", "o"]);
/// assert_eq!(split_graphemes("你好"), vec!["你", "好"]);
/// ```
pub fn split_graphemes(text: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut idx = 0;
    for c in text.chars() {
        let start = idx;
        idx += c.len_utf8();
        result.push(&text[start..idx]);
    }
    result
}

/// Split text into lines such that each line fits within the available (cell) width.
///
/// Each resulting line has a cell width less than or equal to `width`, except when
/// a single character's width exceeds `width` (in which case it is placed on its own
/// line and may overflow).
///
/// If `width` is 0, each character is placed on its own line.
///
/// # Example
///
/// ```
/// use rich_rs::chop_cells;
///
/// assert_eq!(chop_cells("hello", 3), vec!["hel", "lo"]);
/// assert_eq!(chop_cells("你好世界", 5), vec!["你好", "世界"]);
/// ```
pub fn chop_cells(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        // Each character becomes its own line (or empty if text is empty)
        if text.is_empty() {
            return vec![String::new()];
        }
        return text.chars().map(|c| c.to_string()).collect();
    }

    let mut lines: Vec<String> = vec![String::new()];
    let mut total_width = 0;

    for c in text.chars() {
        let cell_width = char_width(c);
        let char_doesnt_fit = total_width + cell_width > width;

        if char_doesnt_fit {
            // Reuse current line if empty (avoids leading empty line bug),
            // otherwise start a new line with this character
            if lines.last().unwrap().is_empty() {
                lines.last_mut().unwrap().push(c);
            } else {
                lines.push(c.to_string());
            }
            total_width = cell_width;
        } else {
            // Add to current line
            lines.last_mut().unwrap().push(c);
            total_width += cell_width;
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== cell_len tests ====================

    #[test]
    fn test_cell_len_ascii() {
        assert_eq!(cell_len("hello"), 5);
        assert_eq!(cell_len(""), 0);
        assert_eq!(cell_len(" "), 1);
        assert_eq!(cell_len("hello world"), 11);
    }

    #[test]
    fn test_cell_len_cjk() {
        assert_eq!(cell_len("你好"), 4);
        assert_eq!(cell_len("日本語"), 6);
        assert_eq!(cell_len("中文测试"), 8);
    }

    #[test]
    fn test_cell_len_mixed() {
        assert_eq!(cell_len("hello你好"), 9); // 5 + 4
        assert_eq!(cell_len("a中b"), 4); // 1 + 2 + 1
    }

    #[test]
    fn test_cell_len_emoji() {
        // Most emoji are 2 cells wide
        assert_eq!(cell_len("😀"), 2);
        assert_eq!(cell_len("🎉"), 2);
    }

    // ==================== char_width tests ====================

    #[test]
    fn test_char_width() {
        assert_eq!(char_width('a'), 1);
        assert_eq!(char_width('中'), 2);
        assert_eq!(char_width('😀'), 2);
        assert_eq!(char_width(' '), 1);
    }

    // ==================== set_cell_size tests ====================

    #[test]
    fn test_set_cell_size_exact() {
        // Text already exactly the right size
        assert_eq!(set_cell_size("hello", 5), "hello");
        assert_eq!(set_cell_size("你好", 4), "你好");
    }

    #[test]
    fn test_set_cell_size_pad() {
        // Text shorter than width - pad with spaces
        assert_eq!(set_cell_size("hello", 10), "hello     ");
        assert_eq!(set_cell_size("hi", 5), "hi   ");
        assert_eq!(set_cell_size("", 3), "   ");
        assert_eq!(set_cell_size("你好", 6), "你好  ");
    }

    #[test]
    fn test_set_cell_size_truncate_ascii() {
        // ASCII truncation
        assert_eq!(set_cell_size("hello world", 5), "hello");
        assert_eq!(set_cell_size("hello", 3), "hel");
        assert_eq!(set_cell_size("abcdef", 1), "a");
    }

    #[test]
    fn test_set_cell_size_truncate_cjk() {
        // CJK truncation - exact boundary
        assert_eq!(set_cell_size("你好世界", 4), "你好");
        assert_eq!(set_cell_size("你好世界", 8), "你好世界");
    }

    #[test]
    fn test_set_cell_size_truncate_double_width_boundary() {
        // Truncation at double-width character boundary - add space
        assert_eq!(set_cell_size("你好世界", 5), "你好 "); // Can't fit 世 (2 cells), add 1 space
        assert_eq!(set_cell_size("你好世界", 3), "你 "); // Can't fit 好 (2 cells), add 1 space
        assert_eq!(set_cell_size("你好世界", 1), " "); // Can't fit 你 (2 cells), add 1 space
    }

    #[test]
    fn test_set_cell_size_mixed() {
        // Mixed ASCII and CJK
        assert_eq!(set_cell_size("a你b好c", 4), "a你b");
        assert_eq!(set_cell_size("a你b好c", 3), "a你");
        assert_eq!(set_cell_size("a你b好c", 2), "a ");
    }

    #[test]
    fn test_set_cell_size_zero_width() {
        assert_eq!(set_cell_size("hello", 0), "");
        assert_eq!(set_cell_size("你好", 0), "");
        assert_eq!(set_cell_size("", 0), "");
    }

    #[test]
    fn test_set_cell_size_empty() {
        assert_eq!(set_cell_size("", 5), "     ");
        assert_eq!(set_cell_size("", 0), "");
    }

    #[test]
    fn test_set_cell_size_emoji() {
        assert_eq!(set_cell_size("😀😀", 4), "😀😀");
        assert_eq!(set_cell_size("😀😀", 3), "😀 "); // Can't fit second emoji
        assert_eq!(set_cell_size("😀😀", 2), "😀");
        assert_eq!(set_cell_size("😀😀", 1), " "); // Can't fit any emoji
    }

    // ==================== chop_cells tests ====================

    #[test]
    fn test_chop_cells_ascii() {
        assert_eq!(chop_cells("hello", 3), vec!["hel", "lo"]);
        assert_eq!(chop_cells("hello", 5), vec!["hello"]);
        assert_eq!(chop_cells("hello", 10), vec!["hello"]);
        assert_eq!(chop_cells("abcdef", 2), vec!["ab", "cd", "ef"]);
    }

    #[test]
    fn test_chop_cells_cjk() {
        assert_eq!(chop_cells("你好世界", 4), vec!["你好", "世界"]);
        assert_eq!(chop_cells("你好世界", 8), vec!["你好世界"]);
        assert_eq!(chop_cells("你好世界", 2), vec!["你", "好", "世", "界"]);
    }

    #[test]
    fn test_chop_cells_cjk_odd_width() {
        // With odd width, double-width chars that don't fit start new line
        assert_eq!(chop_cells("你好世界", 5), vec!["你好", "世界"]);
        assert_eq!(chop_cells("你好世界", 3), vec!["你", "好", "世", "界"]);
    }

    #[test]
    fn test_chop_cells_mixed() {
        assert_eq!(chop_cells("a你b好", 3), vec!["a你", "b好"]);
        assert_eq!(chop_cells("a你b好", 4), vec!["a你b", "好"]);
    }

    #[test]
    fn test_chop_cells_empty() {
        assert_eq!(chop_cells("", 5), vec![""]);
        assert_eq!(chop_cells("", 0), vec![""]);
    }

    #[test]
    fn test_chop_cells_zero_width() {
        // Each character becomes its own line
        assert_eq!(chop_cells("abc", 0), vec!["a", "b", "c"]);
        assert_eq!(chop_cells("你好", 0), vec!["你", "好"]);
    }

    #[test]
    fn test_chop_cells_emoji() {
        assert_eq!(chop_cells("😀😀😀", 4), vec!["😀😀", "😀"]);
        assert_eq!(chop_cells("😀😀", 2), vec!["😀", "😀"]);
        assert_eq!(chop_cells("a😀b", 3), vec!["a😀", "b"]);
    }

    // ==================== split_graphemes tests ====================

    #[test]
    fn test_split_graphemes_ascii() {
        assert_eq!(split_graphemes("hello"), vec!["h", "e", "l", "l", "o"]);
    }

    #[test]
    fn test_split_graphemes_cjk() {
        assert_eq!(split_graphemes("你好"), vec!["你", "好"]);
    }

    #[test]
    fn test_split_graphemes_mixed() {
        assert_eq!(split_graphemes("a你b"), vec!["a", "你", "b"]);
    }

    #[test]
    fn test_split_graphemes_empty() {
        let empty: Vec<&str> = vec![];
        assert_eq!(split_graphemes(""), empty);
    }

    #[test]
    fn test_split_graphemes_emoji() {
        assert_eq!(split_graphemes("😀hi"), vec!["😀", "h", "i"]);
    }

    #[test]
    fn test_chop_cells_single_char_per_line() {
        assert_eq!(chop_cells("abc", 1), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_chop_cells_wide_char_exceeds_width() {
        // When a wide character doesn't fit in width, it should be placed on its own line
        // without creating a leading empty line
        assert_eq!(chop_cells("你", 1), vec!["你"]);
        assert_eq!(chop_cells("😀", 1), vec!["😀"]);
        assert_eq!(chop_cells("你好", 1), vec!["你", "好"]);
        // Mixed: ASCII fits, then CJK overflows
        assert_eq!(chop_cells("a你", 1), vec!["a", "你"]);
    }
}

#[test]
fn test_half_block_width() {
    // Half-block character used in ColorBox
    assert_eq!(cell_len("▄"), 1, "half-block should be 1 cell");
    assert_eq!(cell_len("▄▄▄"), 3, "three half-blocks should be 3 cells");
}
