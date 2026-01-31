//! Text wrapping utilities.
//!
//! Provides functions for word wrapping text to fit within a given cell width.

use crate::cells::{cell_len, chop_cells};

/// A word match from the text: (start_index, end_index, word).
///
/// A "word" in this context includes the actual word and any whitespace to the right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordMatch<'a> {
    /// Start byte offset in the original text.
    pub start: usize,
    /// End byte offset in the original text.
    pub end: usize,
    /// The matched word (may include trailing whitespace).
    pub word: &'a str,
}

/// Iterator over words in text.
///
/// Yields each word as a tuple containing (start_index, end_index, word).
/// A "word" in this context may include the actual word and any whitespace to the right.
/// This matches the Python regex `\s*\S+\s*`.
pub struct Words<'a> {
    text: &'a str,
    position: usize,
}

impl<'a> Words<'a> {
    /// Create a new word iterator over the given text.
    pub fn new(text: &'a str) -> Self {
        Self { text, position: 0 }
    }
}

impl<'a> Iterator for Words<'a> {
    type Item = WordMatch<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.text.len() {
            return None;
        }

        let remaining = &self.text[self.position..];

        // Match pattern: \s*\S+\s*
        // Work with characters to properly handle Unicode whitespace (NBSP, em-space, etc.)
        let mut char_indices = remaining.char_indices().peekable();

        // 1. Skip leading whitespace
        let mut byte_offset = 0;
        while let Some(&(idx, c)) = char_indices.peek() {
            if c.is_whitespace() {
                byte_offset = idx + c.len_utf8();
                char_indices.next();
            } else {
                break;
            }
        }

        // If we've consumed all remaining text (only whitespace left), no more words
        if char_indices.peek().is_none() {
            self.position = self.text.len();
            return None;
        }

        // 2. Consume non-whitespace characters (the actual word)
        while let Some(&(idx, c)) = char_indices.peek() {
            if !c.is_whitespace() {
                byte_offset = idx + c.len_utf8();
                char_indices.next();
            } else {
                break;
            }
        }

        // 3. Consume trailing whitespace
        while let Some(&(idx, c)) = char_indices.peek() {
            if c.is_whitespace() {
                byte_offset = idx + c.len_utf8();
                char_indices.next();
            } else {
                break;
            }
        }

        let start = self.position;
        let end = self.position + byte_offset;
        let word = &self.text[start..end];

        self.position = end;

        Some(WordMatch { start, end, word })
    }
}

/// Create an iterator over words in text.
///
/// Yields each word as a `WordMatch` containing (start, end, word).
/// A "word" includes leading whitespace, the word itself, and trailing whitespace,
/// matching the Python regex pattern `\s*\S+\s*`.
///
/// # Example
///
/// ```
/// use rich_rs::wrap::words;
///
/// let text = "hello world";
/// let word_list: Vec<_> = words(text).collect();
/// assert_eq!(word_list.len(), 2);
/// assert_eq!(word_list[0].word, "hello ");
/// assert_eq!(word_list[1].word, "world");
/// ```
pub fn words(text: &str) -> Words<'_> {
    Words::new(text)
}

/// Find optimal positions to divide a line of text for wrapping.
///
/// Returns byte offsets (positions in the original string) where the text should be split.
///
/// # Arguments
///
/// * `text` - The text to examine.
/// * `width` - The available cell width.
/// * `fold` - If true, words longer than `width` will be folded (hard-wrapped) onto new lines.
///   If false, long words will overflow.
///
/// # Example
///
/// ```
/// use rich_rs::divide_line;
///
/// // Basic word wrap
/// let breaks = divide_line("hello world test", 6, false);
/// assert_eq!(breaks, vec![6, 12]);  // Break before "world" and "test"
///
/// // With folding for long words
/// let breaks = divide_line("abcdefghij", 4, true);
/// assert_eq!(breaks, vec![4, 8]);  // Fold at positions 4 and 8
/// ```
pub fn divide_line(text: &str, width: usize, fold: bool) -> Vec<usize> {
    if width == 0 || text.is_empty() {
        return Vec::new();
    }

    let mut break_positions: Vec<usize> = Vec::new();
    let mut cell_offset = 0;

    for word_match in words(text) {
        let mut start = word_match.start;
        let word = word_match.word;

        // Calculate word length without trailing whitespace
        let word_length = cell_len(word.trim_end());
        let remaining_space = width.saturating_sub(cell_offset);
        let word_fits_remaining_space = remaining_space >= word_length;

        if word_fits_remaining_space {
            // Simplest case - the word fits within the remaining width for this line.
            cell_offset += cell_len(word);
        } else {
            // Not enough space remaining for this word on the current line.
            if word_length > width {
                // The word doesn't fit on any line, so we can't simply
                // place it on the next line...
                if fold {
                    // Fold the word across multiple lines.
                    let folded_word = chop_cells(word, width);
                    let num_lines = folded_word.len();

                    for (idx, line) in folded_word.into_iter().enumerate() {
                        let is_last = idx == num_lines - 1;

                        if start > 0 {
                            break_positions.push(start);
                        }

                        if is_last {
                            cell_offset = cell_len(&line);
                        } else {
                            start += line.len();
                        }
                    }
                } else {
                    // Folding isn't allowed, so just move to next line.
                    if start > 0 {
                        break_positions.push(start);
                    }
                    cell_offset = cell_len(word);
                }
            } else if cell_offset > 0 && start > 0 {
                // The word doesn't fit within the remaining space on the current
                // line, but it *can* fit on to the next (empty) line.
                break_positions.push(start);
                cell_offset = cell_len(word);
            }
        }
    }

    break_positions
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== words() tests ====================

    #[test]
    fn test_words_simple() {
        let result: Vec<_> = words("hello world").collect();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].word, "hello ");
        assert_eq!(result[0].start, 0);
        assert_eq!(result[0].end, 6);
        assert_eq!(result[1].word, "world");
        assert_eq!(result[1].start, 6);
        assert_eq!(result[1].end, 11);
    }

    #[test]
    fn test_words_multiple_spaces() {
        let result: Vec<_> = words("hello   world").collect();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].word, "hello   ");
        assert_eq!(result[1].word, "world");
    }

    #[test]
    fn test_words_leading_space() {
        let result: Vec<_> = words("  hello world").collect();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].word, "  hello ");
        assert_eq!(result[1].word, "world");
    }

    #[test]
    fn test_words_trailing_space() {
        let result: Vec<_> = words("hello world  ").collect();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].word, "hello ");
        assert_eq!(result[1].word, "world  ");
    }

    #[test]
    fn test_words_single_word() {
        let result: Vec<_> = words("hello").collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].word, "hello");
    }

    #[test]
    fn test_words_empty() {
        let result: Vec<_> = words("").collect();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_words_only_whitespace() {
        let result: Vec<_> = words("   ").collect();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_words_unicode_whitespace() {
        // NBSP (U+00A0) and em-space (U+2003) should be treated as whitespace
        let nbsp = '\u{00A0}'; // Non-breaking space
        let em_space = '\u{2003}'; // Em space

        let text = format!("hello{}world{}test", nbsp, em_space);
        let result: Vec<_> = words(&text).collect();
        assert_eq!(result.len(), 3);
        assert!(result[0].word.starts_with("hello"));
        assert!(result[1].word.starts_with("world"));
        assert_eq!(result[2].word, "test");
    }

    #[test]
    fn test_divide_line_unicode_whitespace() {
        // Words separated by NBSP should still break correctly
        let nbsp = '\u{00A0}';
        let text = format!("hello{}world", nbsp);
        let breaks = divide_line(&text, 6, false);
        // "hello\u{00A0}" = 6 cells, "world" needs new line
        assert_eq!(breaks.len(), 1);
    }

    // ==================== divide_line() tests ====================

    #[test]
    fn test_divide_line_basic() {
        // "hello world" with width 6: "hello " fits (6 cells), "world" needs new line
        let breaks = divide_line("hello world", 6, false);
        assert_eq!(breaks, vec![6]); // Break before "world"
    }

    #[test]
    fn test_divide_line_multiple_breaks() {
        // "one two three" with width 4
        let breaks = divide_line("one two three", 4, false);
        assert_eq!(breaks, vec![4, 8]); // Break before "two" and "three"
    }

    #[test]
    fn test_divide_line_exact_fit() {
        // Words fit exactly
        let breaks = divide_line("ab cd", 5, false);
        assert_eq!(breaks, Vec::<usize>::new()); // No breaks needed
    }

    #[test]
    fn test_divide_line_empty() {
        let breaks = divide_line("", 10, false);
        assert_eq!(breaks, Vec::<usize>::new());
    }

    #[test]
    fn test_divide_line_zero_width() {
        let breaks = divide_line("hello world", 0, false);
        assert_eq!(breaks, Vec::<usize>::new());
    }

    #[test]
    fn test_divide_line_single_word_fits() {
        let breaks = divide_line("hello", 10, false);
        assert_eq!(breaks, Vec::<usize>::new());
    }

    #[test]
    fn test_divide_line_long_word_no_fold() {
        // Long word without folding - just starts on new line
        let breaks = divide_line("ab abcdefghij", 5, false);
        // "ab " fits (3 cells), "abcdefghij" is too long but starts at position 3
        assert_eq!(breaks, vec![3]);
    }

    #[test]
    fn test_divide_line_long_word_with_fold() {
        // Long word with folding - breaks within the word
        let breaks = divide_line("abcdefghij", 4, true);
        // "abcd" (4), "efgh" (4), "ij" (2)
        assert_eq!(breaks, vec![4, 8]);
    }

    #[test]
    fn test_divide_line_long_word_with_fold_and_prefix() {
        // Word that needs folding after some text
        let breaks = divide_line("ab abcdefghij", 4, true);
        // "ab " (3), then "abcdefghij" needs folding
        // First fold: position 3 (start of long word)
        // Then: "abcd" (4), break at 7, "efgh" (4), break at 11, "ij" (2)
        assert_eq!(breaks, vec![3, 7, 11]);
    }

    #[test]
    fn test_divide_line_cjk() {
        // CJK characters are 2 cells wide
        let breaks = divide_line("你好 世界", 5, false);
        // "你好 " = 5 cells (2+2+1), "世界" = 4 cells
        assert_eq!(breaks, vec![7]); // Break before "世界" (UTF-8: 你好 = 6 bytes + space = 7)
    }

    #[test]
    fn test_divide_line_cjk_fold() {
        // CJK with folding
        let breaks = divide_line("你好世界", 3, true);
        // Width 3 can only fit one CJK char (2 cells)
        // "你" (2), "好" (2), "世" (2), "界" (2)
        // Each char is 3 bytes in UTF-8
        assert_eq!(breaks, vec![3, 6, 9]);
    }

    #[test]
    fn test_divide_line_mixed_cjk_ascii() {
        // "a你b好" has no spaces, so it's one "word" - no break with fold=false
        let breaks = divide_line("a你b好", 3, false);
        assert_eq!(breaks, Vec::<usize>::new());

        // With fold=true, it should break
        let breaks = divide_line("a你b好", 3, true);
        // "a你" = 3 cells (1+2), "b好" = 3 cells (1+2)
        // "a你" is 4 bytes (a=1, 你=3)
        assert_eq!(breaks, vec![4]);
    }

    #[test]
    fn test_divide_line_preserves_leading_word() {
        // First word should never have a break before it
        let breaks = divide_line("hello", 3, true);
        // "hel" (3), "lo" (2)
        assert_eq!(breaks, vec![3]);
    }

    #[test]
    fn test_divide_line_single_char_width() {
        let breaks = divide_line("ab cd", 1, true);
        // Each character needs its own line
        assert_eq!(breaks, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_divide_line_emoji() {
        // Most emoji are 2 cells wide
        let breaks = divide_line("😀 😀", 3, false);
        // "😀 " = 3 cells (2+1), "😀" = 2 cells
        // 😀 is 4 bytes in UTF-8
        assert_eq!(breaks, vec![5]); // Break before second emoji
    }

    #[test]
    fn test_divide_line_no_break_at_start() {
        // Should never insert a break at position 0
        let breaks = divide_line("abcdefghij", 4, true);
        assert!(!breaks.contains(&0));
    }

    #[test]
    fn test_divide_line_whitespace_handling() {
        // Multiple spaces between words
        // "ab  " is 4 cells exactly, "cd" = 2 cells
        // But after the word "ab  " we're at position 4, so "cd" starts fresh
        let breaks = divide_line("ab  cd", 4, false);
        // The word "ab  " (with trailing spaces) is captured as one word
        // Then "cd" as another - but cell_offset after "ab  " is 4, which
        // doesn't leave room for "cd" (2 cells) since 4 + 2 > 4
        assert_eq!(breaks, vec![4]);
    }
}
