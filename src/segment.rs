//! Segment: the atomic unit of terminal output.
//!
//! Everything in Rich ultimately becomes a sequence of Segments.

use smallvec::SmallVec;
use std::borrow::Cow;

use crate::cells::{cell_len, char_width, set_cell_size};
use crate::style::Style;
use std::sync::Arc;

/// Control codes that can be embedded in output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlType {
    /// Ring the terminal bell.
    Bell,
    /// Carriage return.
    CarriageReturn,
    /// Move cursor to home position.
    Home,
    /// Clear the screen.
    Clear,
    /// Show the cursor.
    ShowCursor,
    /// Hide the cursor.
    HideCursor,
    /// Enable alternate screen buffer.
    EnableAltScreen,
    /// Disable alternate screen buffer.
    DisableAltScreen,
    /// Set window title.
    SetTitle,
    /// Move cursor up N lines.
    CursorUp(u16),
    /// Move cursor down N lines.
    CursorDown(u16),
    /// Move cursor forward N columns.
    CursorForward(u16),
    /// Move cursor backward N columns.
    CursorBackward(u16),
    /// Erase in line (0=cursor to end, 1=start to cursor, 2=entire line).
    EraseInLine(u8),
    /// Start an OSC 8 hyperlink.
    HyperlinkStart { url: Arc<str>, id: Option<Arc<str>> },
    /// End an OSC 8 hyperlink.
    HyperlinkEnd,
}

/// A segment of text with optional style and control codes.
///
/// This is the fundamental unit of output in Rich. All renderables
/// produce sequences of Segments.
///
/// Uses `Cow<'static, str>` for text to allow both owned and static strings
/// without lifetime complexity in the API. This is a deliberate tradeoff
/// favoring API simplicity over zero-copy for borrowed non-static input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// The text content.
    pub text: Cow<'static, str>,
    /// Optional style to apply.
    pub style: Option<Style>,
    /// Optional control code (if set, text is typically empty).
    pub control: Option<ControlType>,
}

impl Segment {
    /// Create a new text segment.
    pub fn new(text: impl Into<Cow<'static, str>>) -> Self {
        Segment {
            text: text.into(),
            style: None,
            control: None,
        }
    }

    /// Create a new styled segment.
    pub fn styled(text: impl Into<Cow<'static, str>>, style: Style) -> Self {
        Segment {
            text: text.into(),
            style: Some(style),
            control: None,
        }
    }

    /// Create a control segment.
    pub fn control(control: ControlType) -> Self {
        Segment {
            text: Cow::Borrowed(""),
            style: None,
            control: Some(control),
        }
    }

    /// Create a newline segment.
    pub fn line() -> Self {
        Segment::new("\n")
    }

    /// Check if this segment is a control segment.
    pub fn is_control(&self) -> bool {
        self.control.is_some()
    }

    /// Get the cell width of this segment's text.
    pub fn cell_len(&self) -> usize {
        crate::cells::cell_len(&self.text)
    }

    /// Apply a style to this segment, combining with any existing style.
    pub fn apply_style(&self, style: &Style) -> Self {
        Segment {
            text: self.text.clone(),
            style: Some(match &self.style {
                Some(existing) => existing.combine(style),
                None => *style,
            }),
            control: self.control.clone(),
        }
    }

    /// Split segment into two segments at the specified cell position.
    ///
    /// If the cut point falls in the middle of a 2-cell wide character then it is replaced
    /// by two spaces, to preserve the display width of the parent segment.
    ///
    /// # Arguments
    ///
    /// * `cut` - Cell offset within the segment to cut at.
    ///
    /// # Returns
    ///
    /// A tuple of two segments: (before, after).
    ///
    /// # Example
    ///
    /// ```
    /// use rich_rs::Segment;
    ///
    /// let seg = Segment::new("hello");
    /// let (before, after) = seg.split_cells(3);
    /// assert_eq!(&*before.text, "hel");
    /// assert_eq!(&*after.text, "lo");
    /// ```
    pub fn split_cells(&self, cut: usize) -> (Segment, Segment) {
        let text = &self.text;
        let style = self.style;
        let control = self.control.clone();

        // Control segments have no visual width
        if control.is_some() {
            return (
                self.clone(),
                Segment::new_with_style_control("", style, control),
            );
        }

        let segment_cell_len = cell_len(text);

        // If cut is at or beyond the end, return original and empty
        if cut >= segment_cell_len {
            return (
                self.clone(),
                Segment::new_with_style_control("", style, control),
            );
        }

        // If cut is at the start, return empty and original
        if cut == 0 {
            return (
                Segment::new_with_style_control("", style, control),
                self.clone(),
            );
        }

        // Fast path: check if all characters are single-width ASCII
        if text.is_ascii() {
            // ASCII characters are all single-cell width
            let before = &text[..cut];
            let after = &text[cut..];
            return (
                Segment::new_with_style_control(before.to_string(), style, control.clone()),
                Segment::new_with_style_control(after.to_string(), style, control),
            );
        }

        // Slow path: iterate through characters tracking cell position
        let mut current_cell_pos = 0;

        for (byte_idx, c) in text.char_indices() {
            let c_width = char_width(c);

            if current_cell_pos == cut {
                // Exact cut point
                let before = &text[..byte_idx];
                let after = &text[byte_idx..];
                return (
                    Segment::new_with_style_control(before.to_string(), style, control.clone()),
                    Segment::new_with_style_control(after.to_string(), style, control),
                );
            }

            if current_cell_pos + c_width > cut {
                // Cut falls in the middle of a double-width character
                // Replace with spaces to preserve total width
                let before = &text[..byte_idx];
                let after_start_byte = byte_idx + c.len_utf8();
                let after = &text[after_start_byte..];

                // We need to add spaces: one at the end of `before`, one at the start of `after`
                let before_with_space = format!("{} ", before);
                let after_with_space = format!(" {}", after);

                return (
                    Segment::new_with_style_control(before_with_space, style, control.clone()),
                    Segment::new_with_style_control(after_with_space, style, control),
                );
            }

            current_cell_pos += c_width;
        }

        // Shouldn't reach here, but return original and empty as fallback
        (
            self.clone(),
            Segment::new_with_style_control("", style, control),
        )
    }

    /// Internal helper to create a segment with optional style and control.
    fn new_with_style_control(
        text: impl Into<Cow<'static, str>>,
        style: Option<Style>,
        control: Option<ControlType>,
    ) -> Self {
        Segment {
            text: text.into(),
            style,
            control,
        }
    }

    // ========================================================================
    // Associated functions (class methods in Python)
    // ========================================================================

    /// Split a sequence of segments into lines on newline characters.
    ///
    /// # Arguments
    ///
    /// * `segments` - Segments potentially containing newlines.
    ///
    /// # Returns
    ///
    /// A vector of lines, where each line is a vector of segments.
    pub fn split_lines(segments: impl IntoIterator<Item = Segment>) -> Vec<Vec<Segment>> {
        let mut lines: Vec<Vec<Segment>> = Vec::new();
        let mut current_line: Vec<Segment> = Vec::new();

        for segment in segments {
            if segment.text.contains('\n') && segment.control.is_none() {
                let text = segment.text.to_string();
                let style = segment.style;
                let mut remaining = text.as_str();

                while !remaining.is_empty() {
                    if let Some(newline_pos) = remaining.find('\n') {
                        let before = &remaining[..newline_pos];
                        if !before.is_empty() {
                            current_line.push(Segment::new_with_style_control(
                                before.to_string(),
                                style,
                                None,
                            ));
                        }
                        lines.push(std::mem::take(&mut current_line));
                        remaining = &remaining[newline_pos + 1..];
                    } else {
                        // No more newlines
                        if !remaining.is_empty() {
                            current_line.push(Segment::new_with_style_control(
                                remaining.to_string(),
                                style,
                                None,
                            ));
                        }
                        break;
                    }
                }
            } else {
                current_line.push(segment);
            }
        }

        // Don't forget the last line if it's non-empty
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }

    /// Split segments into lines and crop/pad each line to a specific length.
    ///
    /// # Arguments
    ///
    /// * `segments` - An iterable of segments to process.
    /// * `length` - Desired line length in cells.
    /// * `style` - Style to use for padding.
    /// * `pad` - Whether to pad lines shorter than `length`.
    /// * `include_new_lines` - Whether to append newline segments to each line.
    ///
    /// # Returns
    ///
    /// A vector of lines, each cropped/padded to the desired length.
    pub fn split_and_crop_lines(
        segments: impl IntoIterator<Item = Segment>,
        length: usize,
        style: Option<Style>,
        pad: bool,
        include_new_lines: bool,
    ) -> Vec<Vec<Segment>> {
        let mut lines: Vec<Vec<Segment>> = Vec::new();
        let mut current_line: Vec<Segment> = Vec::new();
        let new_line_segment = Segment::line();

        for segment in segments {
            if segment.text.contains('\n') && segment.control.is_none() {
                let text = segment.text.to_string();
                let segment_style = segment.style;
                let mut remaining = text.as_str();

                while !remaining.is_empty() {
                    if let Some(newline_pos) = remaining.find('\n') {
                        let before = &remaining[..newline_pos];
                        if !before.is_empty() {
                            current_line.push(Segment::new_with_style_control(
                                before.to_string(),
                                segment_style,
                                None,
                            ));
                        }
                        let mut cropped =
                            Self::adjust_line_length(&current_line, length, style, pad);
                        if include_new_lines {
                            cropped.push(new_line_segment.clone());
                        }
                        lines.push(cropped);
                        current_line.clear();
                        remaining = &remaining[newline_pos + 1..];
                    } else {
                        if !remaining.is_empty() {
                            current_line.push(Segment::new_with_style_control(
                                remaining.to_string(),
                                segment_style,
                                None,
                            ));
                        }
                        break;
                    }
                }
            } else {
                current_line.push(segment);
            }
        }

        // Handle the last line
        if !current_line.is_empty() {
            lines.push(Self::adjust_line_length(&current_line, length, style, pad));
        }

        lines
    }

    /// Adjust a line to a given width by cropping or padding.
    ///
    /// # Arguments
    ///
    /// * `line` - A slice of segments representing a single line.
    /// * `length` - Desired width in cells.
    /// * `style` - Style to use for padding.
    /// * `pad` - Whether to pad lines shorter than `length`.
    ///
    /// # Returns
    ///
    /// A new vector of segments with the desired length.
    pub fn adjust_line_length(
        line: &[Segment],
        length: usize,
        style: Option<Style>,
        pad: bool,
    ) -> Vec<Segment> {
        let line_length = Self::get_line_length(line);

        if line_length < length {
            // Line is shorter than desired
            if pad {
                let mut new_line = line.to_vec();
                let padding = " ".repeat(length - line_length);
                let end_style = line
                    .iter()
                    .rev()
                    .find_map(|seg| {
                        if seg.control.is_some() {
                            return None;
                        }
                        seg.style
                    });
                // Padding should extend *background* colors to avoid hairlines, but should not
                // inherit decoration attributes like underline/bold/dim from the preceding text.
                let padding_style = match (style, end_style) {
                    (Some(mut base), Some(end)) => {
                        if base.bgcolor.is_none() {
                            if let Some(bg) = end.bgcolor {
                                base.bgcolor = Some(bg);
                            }
                        }
                        Some(base)
                    }
                    (Some(base), None) => Some(base),
                    (None, Some(end)) => end.bgcolor.map(|bg| Style::new().with_bgcolor(bg)),
                    (None, None) => None,
                };
                new_line.push(Segment::new_with_style_control(padding, padding_style, None));
                new_line
            } else {
                line.to_vec()
            }
        } else if line_length > length {
            // Line is longer than desired - crop it
            let mut new_line = Vec::new();
            let mut current_length = 0;

            for segment in line {
                let segment_length = segment.cell_len();

                if segment.control.is_some() {
                    // Control segments don't contribute to visual length
                    new_line.push(segment.clone());
                    continue;
                }

                if current_length + segment_length <= length {
                    // Segment fits entirely
                    new_line.push(segment.clone());
                    current_length += segment_length;
                } else {
                    // Segment needs to be cropped
                    let remaining_space = length - current_length;
                    if remaining_space > 0 {
                        let cropped_text = set_cell_size(&segment.text, remaining_space);
                        new_line.push(Segment::new_with_style_control(
                            cropped_text,
                            segment.style,
                            None,
                        ));
                    }
                    break;
                }
            }

            new_line
        } else {
            // Line is exactly the right length
            line.to_vec()
        }
    }

    /// Get the last non-control style in a line.
    ///
    /// This is useful for determining the "end of line" style when padding with spaces,
    /// so background colors extend to the full width.
    pub fn get_last_style(line: &[Segment]) -> Option<Style> {
        line.iter()
            .rev()
            .find_map(|seg| if seg.control.is_some() { None } else { seg.style })
    }

    /// Simplify segments by merging adjacent segments with the same style.
    ///
    /// # Arguments
    ///
    /// * `segments` - An iterable of segments to simplify.
    ///
    /// # Returns
    ///
    /// A `Segments` collection with adjacent same-style segments merged.
    pub fn simplify(segments: impl IntoIterator<Item = Segment>) -> Segments {
        let mut result = Segments::new();
        let mut iter = segments.into_iter();

        let Some(mut last_segment) = iter.next() else {
            return result;
        };

        for segment in iter {
            // Only merge non-control segments with same style
            if last_segment.style == segment.style
                && last_segment.control.is_none()
                && segment.control.is_none()
            {
                // Merge text
                let merged_text = format!("{}{}", last_segment.text, segment.text);
                last_segment =
                    Segment::new_with_style_control(merged_text, last_segment.style, None);
            } else {
                result.push(last_segment);
                last_segment = segment;
            }
        }

        result.push(last_segment);
        result
    }

    /// Divide segments at multiple cell positions.
    ///
    /// # Arguments
    ///
    /// * `segments` - Segments to divide.
    /// * `cuts` - Cell positions where to divide (must be sorted in ascending order).
    ///
    /// # Returns
    ///
    /// A vector of segment vectors, one for each division. Always includes a trailing
    /// partition containing any remaining content after the last cut.
    ///
    /// # Panics (debug mode only)
    ///
    /// Debug-asserts that cuts are sorted in ascending order.
    pub fn divide(
        segments: impl IntoIterator<Item = Segment>,
        cuts: &[usize],
    ) -> Vec<Vec<Segment>> {
        // Precondition: cuts must be sorted ascending
        debug_assert!(
            cuts.windows(2).all(|w| w[0] <= w[1]),
            "cuts must be sorted in ascending order"
        );

        if cuts.is_empty() {
            return Vec::new();
        }

        let mut result: Vec<Vec<Segment>> = Vec::new();
        let mut split_segments: Vec<Segment> = Vec::new();
        let mut cut_iter = cuts.iter().copied();

        // Handle leading zeros
        let mut current_cut;
        loop {
            match cut_iter.next() {
                None => return result,
                Some(0) => result.push(Vec::new()),
                Some(c) => {
                    current_cut = c;
                    break;
                }
            }
        }

        let mut pos: usize = 0;
        let mut cuts_exhausted = false;

        for segment in segments {
            // Control segments don't contribute to position
            if segment.control.is_some() {
                split_segments.push(segment);
                continue;
            }

            // If cuts are exhausted, just accumulate remaining segments
            if cuts_exhausted {
                split_segments.push(segment);
                continue;
            }

            let mut current_segment = segment;

            loop {
                let text = &current_segment.text;
                if text.is_empty() {
                    break;
                }

                let seg_len = cell_len(text);
                let end_pos = pos + seg_len;

                if end_pos < current_cut {
                    // Entire segment fits before cut
                    split_segments.push(current_segment);
                    pos = end_pos;
                    break;
                } else if end_pos == current_cut {
                    // Segment ends exactly at cut
                    split_segments.push(current_segment);
                    result.push(std::mem::take(&mut split_segments));
                    pos = end_pos;

                    // Move to next cut
                    match cut_iter.next() {
                        None => {
                            // No more cuts - set flag and continue accumulating
                            cuts_exhausted = true;
                        }
                        Some(next_cut) => current_cut = next_cut,
                    }
                    break;
                } else {
                    // Segment crosses the cut boundary - split it
                    let split_point = current_cut - pos;
                    let (before, after) = current_segment.split_cells(split_point);

                    if !before.text.is_empty() {
                        split_segments.push(before);
                    }
                    result.push(std::mem::take(&mut split_segments));
                    pos = current_cut;

                    // Continue processing with the remaining part
                    current_segment = after;

                    // Move to next cut
                    match cut_iter.next() {
                        None => {
                            // No more cuts - add remaining segment and set flag
                            if !current_segment.text.is_empty() {
                                split_segments.push(current_segment);
                            }
                            cuts_exhausted = true;
                            break;
                        }
                        Some(next_cut) => current_cut = next_cut,
                    }
                    // Continue the inner loop to process remaining segment
                }
            }
        }

        // Always yield the trailing partition (matches Python Rich's `yield segments_copy()`)
        result.push(split_segments);
        result
    }

    /// Apply style to all segments.
    ///
    /// Returns segments where the style is replaced by `style + segment.style + post_style`.
    ///
    /// # Arguments
    ///
    /// * `segments` - Segments to process.
    /// * `style` - Base style to apply first.
    /// * `post_style` - Style to apply after segment's own style.
    ///
    /// # Returns
    ///
    /// A new `Segments` collection with styles applied.
    pub fn apply_style_to_segments(
        segments: impl IntoIterator<Item = Segment>,
        style: Option<Style>,
        post_style: Option<Style>,
    ) -> Segments {
        let mut result = Segments::new();

        for segment in segments {
            if segment.control.is_some() {
                // Don't apply style to control segments
                result.push(segment);
                continue;
            }

            let mut new_style = segment.style;

            // Apply base style first
            if let Some(base) = style {
                new_style = Some(match new_style {
                    Some(existing) => base.combine(&existing),
                    None => base,
                });
            }

            // Apply post style
            if let Some(post) = post_style {
                new_style = Some(match new_style {
                    Some(existing) => existing.combine(&post),
                    None => post,
                });
            }

            result.push(Segment {
                text: segment.text,
                style: new_style,
                control: None,
            });
        }

        result
    }

    /// Filter segments by control status.
    ///
    /// # Arguments
    ///
    /// * `segments` - Segments to filter.
    /// * `is_control` - If true, keep only control segments; if false, keep only non-control segments.
    ///
    /// # Returns
    ///
    /// A new `Segments` collection with only matching segments.
    pub fn filter_control(
        segments: impl IntoIterator<Item = Segment>,
        is_control: bool,
    ) -> Segments {
        segments
            .into_iter()
            .filter(|s| s.is_control() == is_control)
            .collect()
    }

    /// Remove all styles from segments.
    ///
    /// # Arguments
    ///
    /// * `segments` - Segments to process.
    ///
    /// # Returns
    ///
    /// A new `Segments` collection with all styles removed.
    pub fn strip_styles(segments: impl IntoIterator<Item = Segment>) -> Segments {
        segments
            .into_iter()
            .map(|s| Segment {
                text: s.text,
                style: None,
                control: s.control,
            })
            .collect()
    }

    /// Get the total cell width of a line of segments.
    ///
    /// # Arguments
    ///
    /// * `line` - A slice of segments representing a single line (no newlines).
    ///
    /// # Returns
    ///
    /// The total cell width of all non-control segments.
    pub fn get_line_length(line: &[Segment]) -> usize {
        line.iter()
            .filter(|s| s.control.is_none())
            .map(|s| cell_len(&s.text))
            .sum()
    }

    /// Get the shape (enclosing rectangle) of a list of lines.
    ///
    /// # Arguments
    ///
    /// * `lines` - A list of lines (no newline characters).
    ///
    /// # Returns
    ///
    /// A tuple of (width, height) representing the enclosing rectangle.
    pub fn get_shape(lines: &[Vec<Segment>]) -> (usize, usize) {
        if lines.is_empty() {
            return (0, 0);
        }

        let max_width = lines
            .iter()
            .map(|line| Self::get_line_length(line))
            .max()
            .unwrap_or(0);
        (max_width, lines.len())
    }

    /// Set the shape of a list of lines to a specific rectangle.
    ///
    /// # Arguments
    ///
    /// * `lines` - A list of lines.
    /// * `width` - Desired width.
    /// * `height` - Desired height (if None, uses current height).
    /// * `style` - Style for padding.
    /// * `new_lines` - Whether padded lines should include newline characters.
    ///
    /// # Returns
    ///
    /// A new list of lines with the specified shape.
    pub fn set_shape(
        lines: &[Vec<Segment>],
        width: usize,
        height: Option<usize>,
        style: Option<Style>,
        new_lines: bool,
    ) -> Vec<Vec<Segment>> {
        let target_height = height.unwrap_or(lines.len());

        // Create blank line for padding
        let blank_text = if new_lines {
            format!("{}\n", " ".repeat(width))
        } else {
            " ".repeat(width)
        };
        let blank = vec![Segment::new_with_style_control(blank_text, style, None)];

        let mut result: Vec<Vec<Segment>> = lines
            .iter()
            .take(target_height)
            .map(|line| Self::adjust_line_length(line, width, style, true))
            .collect();

        // Add blank lines if needed
        while result.len() < target_height {
            result.push(blank.clone());
        }

        result
    }
}

impl Default for Segment {
    fn default() -> Self {
        Segment::new("")
    }
}

impl From<&'static str> for Segment {
    fn from(s: &'static str) -> Self {
        Segment::new(s)
    }
}

impl From<String> for Segment {
    fn from(s: String) -> Self {
        Segment::new(s)
    }
}

/// A collection of segments, backed by SmallVec for efficiency.
///
/// This newtype abstracts over the underlying storage, allowing future
/// optimization (e.g., streaming) without breaking the API.
#[derive(Debug, Clone, Default)]
pub struct Segments(SmallVec<[Segment; 8]>);

impl Segments {
    /// Create an empty Segments collection.
    pub fn new() -> Self {
        Segments(SmallVec::new())
    }

    /// Create a Segments collection with a single segment.
    pub fn one(segment: Segment) -> Self {
        let mut sv = SmallVec::new();
        sv.push(segment);
        Segments(sv)
    }

    /// Add a segment to the collection.
    pub fn push(&mut self, segment: Segment) {
        self.0.push(segment);
    }

    /// Extend with segments from an iterator.
    pub fn extend(&mut self, iter: impl IntoIterator<Item = Segment>) {
        self.0.extend(iter);
    }

    /// Get the number of segments.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate over segments.
    pub fn iter(&self) -> impl Iterator<Item = &Segment> {
        self.0.iter()
    }

    /// Get the total cell width of all segments.
    pub fn cell_len(&self) -> usize {
        self.0.iter().map(|s| s.cell_len()).sum()
    }

    /// Convert to a Vec (consumes self).
    pub fn into_vec(self) -> Vec<Segment> {
        self.0.into_vec()
    }
}

impl From<Segment> for Segments {
    fn from(segment: Segment) -> Self {
        Segments::one(segment)
    }
}

impl From<Vec<Segment>> for Segments {
    fn from(vec: Vec<Segment>) -> Self {
        Segments(SmallVec::from_vec(vec))
    }
}

impl FromIterator<Segment> for Segments {
    fn from_iter<I: IntoIterator<Item = Segment>>(iter: I) -> Self {
        Segments(iter.into_iter().collect())
    }
}

impl IntoIterator for Segments {
    type Item = Segment;
    type IntoIter = smallvec::IntoIter<[Segment; 8]>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Segments {
    type Item = &'a Segment;
    type IntoIter = std::slice::Iter<'a, Segment>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_segment() {
        let seg = Segment::new("hello");
        assert_eq!(&*seg.text, "hello");
        assert!(seg.style.is_none());
        assert!(seg.control.is_none());
    }

    #[test]
    fn test_cell_len() {
        let seg = Segment::new("hello");
        assert_eq!(seg.cell_len(), 5);
    }

    #[test]
    fn test_segments_collection() {
        let mut segs = Segments::new();
        segs.push(Segment::new("hello"));
        segs.push(Segment::new(" "));
        segs.push(Segment::new("world"));
        assert_eq!(segs.len(), 3);
        assert_eq!(segs.cell_len(), 11);
    }

    #[test]
    fn test_segments_from_iter() {
        let segs: Segments = vec![Segment::new("a"), Segment::new("b"), Segment::new("c")]
            .into_iter()
            .collect();
        assert_eq!(segs.len(), 3);
    }

    // ==================== split_cells tests ====================

    #[test]
    fn test_split_cells_ascii() {
        let seg = Segment::new("hello");
        let (before, after) = seg.split_cells(3);
        assert_eq!(&*before.text, "hel");
        assert_eq!(&*after.text, "lo");
    }

    #[test]
    fn test_split_cells_at_start() {
        let seg = Segment::new("hello");
        let (before, after) = seg.split_cells(0);
        assert_eq!(&*before.text, "");
        assert_eq!(&*after.text, "hello");
    }

    #[test]
    fn test_split_cells_at_end() {
        let seg = Segment::new("hello");
        let (before, after) = seg.split_cells(5);
        assert_eq!(&*before.text, "hello");
        assert_eq!(&*after.text, "");
    }

    #[test]
    fn test_split_cells_beyond_end() {
        let seg = Segment::new("hello");
        let (before, after) = seg.split_cells(10);
        assert_eq!(&*before.text, "hello");
        assert_eq!(&*after.text, "");
    }

    #[test]
    fn test_split_cells_cjk_exact() {
        // CJK characters are 2 cells wide
        let seg = Segment::new("你好");
        let (before, after) = seg.split_cells(2);
        assert_eq!(&*before.text, "你");
        assert_eq!(&*after.text, "好");
    }

    #[test]
    fn test_split_cells_cjk_middle() {
        // Splitting in the middle of a double-width char should replace with spaces
        let seg = Segment::new("你好");
        let (before, after) = seg.split_cells(1);
        assert_eq!(&*before.text, " "); // Space replaces first half of 你
        assert_eq!(&*after.text, " 好"); // Space + remaining chars
    }

    #[test]
    fn test_split_cells_cjk_middle_complex() {
        // "你好世界" = 8 cells total
        let seg = Segment::new("你好世界");
        let (before, after) = seg.split_cells(3);
        // Cut at 3 is in the middle of 好 (which spans cells 2-3)
        assert_eq!(&*before.text, "你 "); // 你 + space
        assert_eq!(&*after.text, " 世界"); // space + 世界
    }

    #[test]
    fn test_split_cells_mixed_content() {
        // "a你b" = 1 + 2 + 1 = 4 cells
        let seg = Segment::new("a你b");
        let (before, after) = seg.split_cells(3);
        assert_eq!(&*before.text, "a你");
        assert_eq!(&*after.text, "b");
    }

    #[test]
    fn test_split_cells_preserves_style() {
        let style = Style::new().with_bold(true);
        let seg = Segment::styled("hello", style);
        let (before, after) = seg.split_cells(2);
        assert_eq!(before.style, Some(style));
        assert_eq!(after.style, Some(style));
    }

    #[test]
    fn test_split_cells_control_segment() {
        let seg = Segment::control(ControlType::Bell);
        let (before, after) = seg.split_cells(5);
        assert!(before.control.is_some());
        assert_eq!(&*after.text, "");
    }

    #[test]
    fn test_split_cells_emoji() {
        // Emoji are typically 2 cells wide
        let seg = Segment::new("😀hello");
        let (before, after) = seg.split_cells(2);
        assert_eq!(&*before.text, "😀");
        assert_eq!(&*after.text, "hello");
    }

    // ==================== split_lines tests ====================

    #[test]
    fn test_split_lines_no_newlines() {
        let segments = vec![Segment::new("hello"), Segment::new(" world")];
        let lines = Segment::split_lines(segments);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 2);
    }

    #[test]
    fn test_split_lines_single_newline() {
        let segments = vec![Segment::new("hello\nworld")];
        let lines = Segment::split_lines(segments);
        assert_eq!(lines.len(), 2);
        assert_eq!(&*lines[0][0].text, "hello");
        assert_eq!(&*lines[1][0].text, "world");
    }

    #[test]
    fn test_split_lines_multiple_newlines() {
        let segments = vec![Segment::new("a\nb\nc")];
        let lines = Segment::split_lines(segments);
        assert_eq!(lines.len(), 3);
        assert_eq!(&*lines[0][0].text, "a");
        assert_eq!(&*lines[1][0].text, "b");
        assert_eq!(&*lines[2][0].text, "c");
    }

    #[test]
    fn test_split_lines_trailing_newline() {
        let segments = vec![Segment::new("hello\n")];
        let lines = Segment::split_lines(segments);
        assert_eq!(lines.len(), 1);
        assert_eq!(&*lines[0][0].text, "hello");
    }

    #[test]
    fn test_split_lines_preserves_style() {
        let style = Style::new().with_bold(true);
        let segments = vec![Segment::styled("hello\nworld", style)];
        let lines = Segment::split_lines(segments);
        assert_eq!(lines[0][0].style, Some(style));
        assert_eq!(lines[1][0].style, Some(style));
    }

    #[test]
    fn test_split_lines_control_segment_unaffected() {
        let segments = vec![
            Segment::new("hello"),
            Segment::control(ControlType::Bell),
            Segment::new("world"),
        ];
        let lines = Segment::split_lines(segments);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 3);
    }

    // ==================== split_and_crop_lines tests ====================

    #[test]
    fn test_split_and_crop_lines_basic() {
        let segments = vec![Segment::new("hello world")];
        let lines = Segment::split_and_crop_lines(segments, 5, None, true, false);
        assert_eq!(lines.len(), 1);
        // Line should be cropped to 5 cells
        let total_len: usize = lines[0].iter().map(|s| cell_len(&s.text)).sum();
        assert_eq!(total_len, 5);
    }

    #[test]
    fn test_split_and_crop_lines_with_newlines() {
        let segments = vec![Segment::new("hello\nworld")];
        let lines = Segment::split_and_crop_lines(segments, 10, None, true, false);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_split_and_crop_lines_include_newlines() {
        let segments = vec![Segment::new("hello\nworld")];
        let lines = Segment::split_and_crop_lines(segments, 10, None, true, true);
        assert_eq!(lines.len(), 2);
        // First line should have a newline segment at the end
        let last_seg = lines[0].last().unwrap();
        assert_eq!(&*last_seg.text, "\n");
    }

    #[test]
    fn test_split_and_crop_lines_padding() {
        let segments = vec![Segment::new("hi")];
        let lines = Segment::split_and_crop_lines(segments, 5, None, true, false);
        let total_len: usize = lines[0].iter().map(|s| cell_len(&s.text)).sum();
        assert_eq!(total_len, 5); // Should be padded to 5
    }

    #[test]
    fn test_split_and_crop_lines_no_padding() {
        let segments = vec![Segment::new("hi")];
        let lines = Segment::split_and_crop_lines(segments, 5, None, false, false);
        let total_len: usize = lines[0].iter().map(|s| cell_len(&s.text)).sum();
        assert_eq!(total_len, 2); // Should not be padded
    }

    // ==================== adjust_line_length tests ====================

    #[test]
    fn test_adjust_line_length_exact() {
        let line = vec![Segment::new("hello")];
        let result = Segment::adjust_line_length(&line, 5, None, true);
        assert_eq!(Segment::get_line_length(&result), 5);
    }

    #[test]
    fn test_adjust_line_length_pad() {
        let line = vec![Segment::new("hi")];
        let result = Segment::adjust_line_length(&line, 5, None, true);
        assert_eq!(Segment::get_line_length(&result), 5);
        assert_eq!(result.len(), 2); // Original + padding
    }

    #[test]
    fn test_adjust_line_length_pad_inherits_end_style() {
        let end_style =
            Style::new().with_bgcolor(crate::SimpleColor::Rgb { r: 1, g: 2, b: 3 });
        let line = vec![Segment::styled("x", end_style)];
        let result = Segment::adjust_line_length(&line, 3, None, true);
        assert_eq!(Segment::get_line_length(&result), 3);
        let padding = result.last().unwrap();
        assert_eq!(&*padding.text, "  ");
        assert_eq!(padding.style.unwrap().bgcolor, end_style.bgcolor);
    }

    #[test]
    fn test_adjust_line_length_pad_combines_base_and_end_style() {
        let base = Style::new().with_bold(true);
        let end_style =
            Style::new().with_bgcolor(crate::SimpleColor::Rgb { r: 4, g: 5, b: 6 });
        let line = vec![Segment::styled("x", end_style)];
        let result = Segment::adjust_line_length(&line, 3, Some(base), true);
        let padding = result.last().unwrap().style.unwrap();
        assert_eq!(padding.bold, Some(true));
        assert_eq!(padding.bgcolor, end_style.bgcolor);
    }

    #[test]
    fn test_adjust_line_length_no_pad() {
        let line = vec![Segment::new("hi")];
        let result = Segment::adjust_line_length(&line, 5, None, false);
        assert_eq!(Segment::get_line_length(&result), 2);
    }

    #[test]
    fn test_adjust_line_length_crop() {
        let line = vec![Segment::new("hello world")];
        let result = Segment::adjust_line_length(&line, 5, None, true);
        assert_eq!(Segment::get_line_length(&result), 5);
    }

    #[test]
    fn test_adjust_line_length_crop_multiple_segments() {
        let line = vec![Segment::new("hello"), Segment::new(" world")];
        let result = Segment::adjust_line_length(&line, 7, None, true);
        assert_eq!(Segment::get_line_length(&result), 7);
    }

    #[test]
    fn test_adjust_line_length_preserves_control() {
        let line = vec![Segment::control(ControlType::Bell), Segment::new("hello")];
        let result = Segment::adjust_line_length(&line, 3, None, true);
        assert!(result[0].control.is_some());
    }

    #[test]
    fn test_adjust_line_length_crop_cjk() {
        // CJK characters are 2 cells wide
        let line = vec![Segment::new("你好世界")]; // 8 cells
        let result = Segment::adjust_line_length(&line, 5, None, true);
        assert_eq!(Segment::get_line_length(&result), 5);
    }

    // ==================== simplify tests ====================

    #[test]
    fn test_simplify_empty() {
        let segments: Vec<Segment> = vec![];
        let result = Segment::simplify(segments);
        assert!(result.is_empty());
    }

    #[test]
    fn test_simplify_single() {
        let segments = vec![Segment::new("hello")];
        let result = Segment::simplify(segments);
        assert_eq!(result.len(), 1);
        assert_eq!(&*result.iter().next().unwrap().text, "hello");
    }

    #[test]
    fn test_simplify_same_style() {
        let segments = vec![Segment::new("hello"), Segment::new(" world")];
        let result = Segment::simplify(segments);
        assert_eq!(result.len(), 1);
        assert_eq!(&*result.iter().next().unwrap().text, "hello world");
    }

    #[test]
    fn test_simplify_different_styles() {
        let style1 = Style::new().with_bold(true);
        let style2 = Style::new().with_italic(true);
        let segments = vec![
            Segment::styled("hello", style1),
            Segment::styled(" world", style2),
        ];
        let result = Segment::simplify(segments);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_simplify_control_not_merged() {
        let segments = vec![
            Segment::new("hello"),
            Segment::control(ControlType::Bell),
            Segment::new(" world"),
        ];
        let result = Segment::simplify(segments);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_simplify_mixed() {
        let style = Style::new().with_bold(true);
        let segments = vec![
            Segment::new("a"),
            Segment::new("b"),
            Segment::styled("c", style),
            Segment::styled("d", style),
            Segment::new("e"),
        ];
        let result = Segment::simplify(segments);
        assert_eq!(result.len(), 3);
        let texts: Vec<&str> = result.iter().map(|s| &*s.text).collect();
        assert_eq!(texts, vec!["ab", "cd", "e"]);
    }

    // ==================== divide tests ====================

    #[test]
    fn test_divide_empty_cuts() {
        let segments = vec![Segment::new("hello")];
        let result = Segment::divide(segments, &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_divide_single_cut() {
        let segments = vec![Segment::new("hello world")];
        let result = Segment::divide(segments, &[5]);
        // With trailing partition: [0..5) = "hello", [5..) = " world"
        assert_eq!(result.len(), 2);
        let first_text: String = result[0].iter().map(|s| s.text.to_string()).collect();
        let second_text: String = result[1].iter().map(|s| s.text.to_string()).collect();
        assert_eq!(first_text, "hello");
        assert_eq!(second_text, " world");
    }

    #[test]
    fn test_divide_multiple_cuts() {
        let segments = vec![Segment::new("hello world!")];
        // Cuts at cell positions 5 and 11 divide the string into portions
        // [0..5) = "hello", [5..11) = " world", [11..) = "!" (trailing partition)
        let result = Segment::divide(segments, &[5, 11]);
        assert_eq!(result.len(), 3);
        let first: String = result[0].iter().map(|s| s.text.to_string()).collect();
        let second: String = result[1].iter().map(|s| s.text.to_string()).collect();
        let third: String = result[2].iter().map(|s| s.text.to_string()).collect();
        assert_eq!(first, "hello");
        assert_eq!(second, " world");
        assert_eq!(third, "!");
    }

    #[test]
    fn test_divide_includes_remainder() {
        let segments = vec![Segment::new("hello world!")];
        // With cuts at 5 and 12, we get "hello", " world!", and empty trailing partition
        let result = Segment::divide(segments, &[5, 12]);
        assert_eq!(result.len(), 3);
        let first: String = result[0].iter().map(|s| s.text.to_string()).collect();
        let second: String = result[1].iter().map(|s| s.text.to_string()).collect();
        let third: String = result[2].iter().map(|s| s.text.to_string()).collect();
        assert_eq!(first, "hello");
        assert_eq!(second, " world!");
        assert_eq!(third, ""); // Empty trailing partition when content ends exactly at cut
    }

    #[test]
    fn test_divide_zero_cut() {
        let segments = vec![Segment::new("hello")];
        let result = Segment::divide(segments, &[0, 3]);
        // [0..0) = empty, [0..3) = "hel", [3..) = "lo" (trailing)
        assert_eq!(result.len(), 3);
        assert!(result[0].is_empty()); // Zero cut yields empty
        let second: String = result[1].iter().map(|s| s.text.to_string()).collect();
        let third: String = result[2].iter().map(|s| s.text.to_string()).collect();
        assert_eq!(second, "hel");
        assert_eq!(third, "lo");
    }

    #[test]
    fn test_divide_cjk() {
        // "你好世界" = 8 cells
        let segments = vec![Segment::new("你好世界")];
        let result = Segment::divide(segments, &[4]);
        // [0..4) = "你好", [4..) = "世界" (trailing partition)
        assert_eq!(result.len(), 2);
        let first: String = result[0].iter().map(|s| s.text.to_string()).collect();
        let second: String = result[1].iter().map(|s| s.text.to_string()).collect();
        assert_eq!(first, "你好");
        assert_eq!(second, "世界");
    }

    #[test]
    fn test_divide_trailing_content_after_last_cut() {
        // This is the key test for the bug fix: content after last cut should be included
        let segments = vec![Segment::new("abc123xyz")];
        let result = Segment::divide(segments, &[3, 6]);
        // [0..3) = "abc", [3..6) = "123", [6..) = "xyz" (trailing)
        assert_eq!(result.len(), 3);
        let first: String = result[0].iter().map(|s| s.text.to_string()).collect();
        let second: String = result[1].iter().map(|s| s.text.to_string()).collect();
        let third: String = result[2].iter().map(|s| s.text.to_string()).collect();
        assert_eq!(first, "abc");
        assert_eq!(second, "123");
        assert_eq!(third, "xyz");
    }

    #[test]
    fn test_divide_empty_trailing_partition_when_content_ends_at_cut() {
        // When content ends exactly at the last cut, we still get an empty trailing partition
        let segments = vec![Segment::new("hello")];
        let result = Segment::divide(segments, &[5]);
        // [0..5) = "hello", [5..) = "" (empty trailing)
        assert_eq!(result.len(), 2);
        let first: String = result[0].iter().map(|s| s.text.to_string()).collect();
        let second: String = result[1].iter().map(|s| s.text.to_string()).collect();
        assert_eq!(first, "hello");
        assert_eq!(second, "");
    }

    #[test]
    fn test_divide_multiple_segments_with_trailing() {
        // Test with multiple input segments
        let segments = vec![
            Segment::new("hello"),
            Segment::new(" "),
            Segment::new("world"),
        ];
        let result = Segment::divide(segments, &[6]);
        // [0..6) = "hello ", [6..) = "world"
        assert_eq!(result.len(), 2);
        let first: String = result[0].iter().map(|s| s.text.to_string()).collect();
        let second: String = result[1].iter().map(|s| s.text.to_string()).collect();
        assert_eq!(first, "hello ");
        assert_eq!(second, "world");
    }

    // ==================== apply_style_to_segments tests ====================

    #[test]
    fn test_apply_style_base() {
        let base = Style::new().with_bold(true);
        let segments = vec![Segment::new("hello")];
        let result = Segment::apply_style_to_segments(segments, Some(base), None);
        assert_eq!(result.iter().next().unwrap().style, Some(base));
    }

    #[test]
    fn test_apply_style_post() {
        let post = Style::new().with_italic(true);
        let segments = vec![Segment::new("hello")];
        let result = Segment::apply_style_to_segments(segments, None, Some(post));
        assert_eq!(result.iter().next().unwrap().style, Some(post));
    }

    #[test]
    fn test_apply_style_both() {
        let base = Style::new().with_bold(true);
        let post = Style::new().with_italic(true);
        let segments = vec![Segment::new("hello")];
        let result = Segment::apply_style_to_segments(segments, Some(base), Some(post));
        let style = result.iter().next().unwrap().style.unwrap();
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.italic, Some(true));
    }

    #[test]
    fn test_apply_style_combines_with_existing() {
        let base = Style::new().with_bold(true);
        let existing = Style::new().with_italic(true);
        let segments = vec![Segment::styled("hello", existing)];
        let result = Segment::apply_style_to_segments(segments, Some(base), None);
        let style = result.iter().next().unwrap().style.unwrap();
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.italic, Some(true));
    }

    #[test]
    fn test_apply_style_control_unchanged() {
        let base = Style::new().with_bold(true);
        let segments = vec![Segment::control(ControlType::Bell)];
        let result = Segment::apply_style_to_segments(segments, Some(base), None);
        let seg = result.iter().next().unwrap();
        assert!(seg.control.is_some());
    }

    // ==================== filter_control tests ====================

    #[test]
    fn test_filter_control_keep_control() {
        let segments = vec![
            Segment::new("hello"),
            Segment::control(ControlType::Bell),
            Segment::new("world"),
        ];
        let result = Segment::filter_control(segments, true);
        assert_eq!(result.len(), 1);
        assert!(result.iter().next().unwrap().control.is_some());
    }

    #[test]
    fn test_filter_control_keep_non_control() {
        let segments = vec![
            Segment::new("hello"),
            Segment::control(ControlType::Bell),
            Segment::new("world"),
        ];
        let result = Segment::filter_control(segments, false);
        assert_eq!(result.len(), 2);
        for seg in result.iter() {
            assert!(seg.control.is_none());
        }
    }

    // ==================== strip_styles tests ====================

    #[test]
    fn test_strip_styles() {
        let style = Style::new().with_bold(true);
        let segments = vec![
            Segment::styled("hello", style),
            Segment::styled("world", style),
        ];
        let result = Segment::strip_styles(segments);
        for seg in result.iter() {
            assert!(seg.style.is_none());
        }
    }

    #[test]
    fn test_strip_styles_preserves_control() {
        let segments = vec![Segment::control(ControlType::Bell)];
        let result = Segment::strip_styles(segments);
        assert!(result.iter().next().unwrap().control.is_some());
    }

    // ==================== get_line_length tests ====================

    #[test]
    fn test_get_line_length_simple() {
        let line = vec![Segment::new("hello")];
        assert_eq!(Segment::get_line_length(&line), 5);
    }

    #[test]
    fn test_get_line_length_multiple() {
        let line = vec![Segment::new("hello"), Segment::new(" world")];
        assert_eq!(Segment::get_line_length(&line), 11);
    }

    #[test]
    fn test_get_line_length_ignores_control() {
        let line = vec![
            Segment::new("hello"),
            Segment::control(ControlType::Bell),
            Segment::new("world"),
        ];
        assert_eq!(Segment::get_line_length(&line), 10);
    }

    #[test]
    fn test_get_line_length_cjk() {
        let line = vec![Segment::new("你好")];
        assert_eq!(Segment::get_line_length(&line), 4);
    }

    // ==================== get_shape tests ====================

    #[test]
    fn test_get_shape_empty() {
        let lines: Vec<Vec<Segment>> = vec![];
        assert_eq!(Segment::get_shape(&lines), (0, 0));
    }

    #[test]
    fn test_get_shape_single_line() {
        let lines = vec![vec![Segment::new("hello")]];
        assert_eq!(Segment::get_shape(&lines), (5, 1));
    }

    #[test]
    fn test_get_shape_multiple_lines() {
        let lines = vec![
            vec![Segment::new("hello")],
            vec![Segment::new("world!")],
            vec![Segment::new("hi")],
        ];
        assert_eq!(Segment::get_shape(&lines), (6, 3));
    }

    // ==================== set_shape tests ====================

    #[test]
    fn test_set_shape_pad_width() {
        let lines = vec![vec![Segment::new("hi")]];
        let result = Segment::set_shape(&lines, 5, None, None, false);
        assert_eq!(result.len(), 1);
        assert_eq!(Segment::get_line_length(&result[0]), 5);
    }

    #[test]
    fn test_set_shape_add_height() {
        let lines = vec![vec![Segment::new("hello")]];
        let result = Segment::set_shape(&lines, 5, Some(3), None, false);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_set_shape_crop_height() {
        let lines = vec![
            vec![Segment::new("a")],
            vec![Segment::new("b")],
            vec![Segment::new("c")],
        ];
        let result = Segment::set_shape(&lines, 5, Some(2), None, false);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_set_shape_with_newlines() {
        let lines = vec![vec![Segment::new("hi")]];
        let result = Segment::set_shape(&lines, 5, Some(2), None, true);
        assert_eq!(result.len(), 2);
        // Blank lines should contain newline
        let blank_text = result[1]
            .iter()
            .map(|s| s.text.to_string())
            .collect::<String>();
        assert!(blank_text.ends_with('\n'));
    }

    #[test]
    fn test_set_shape_with_style() {
        let style = Style::new().with_bold(true);
        let lines: Vec<Vec<Segment>> = vec![];
        let result = Segment::set_shape(&lines, 5, Some(1), Some(style), false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0][0].style, Some(style));
    }

    // ==================== Send + Sync compile-time assertions ====================

    /// Compile-time assertion that Segment is Send + Sync.
    /// This test ensures that if a future field breaks these traits, the build will fail.
    #[test]
    fn test_segment_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Segment>();
        assert_sync::<Segment>();
    }

    /// Compile-time assertion that Segments is Send + Sync.
    #[test]
    fn test_segments_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Segments>();
        assert_sync::<Segments>();
    }
}
