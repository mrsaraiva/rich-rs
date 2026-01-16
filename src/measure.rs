//! Measurement: width requirements for renderables.

use crate::segment::Segments;

/// The minimum and maximum width requirements of a renderable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Measurement {
    /// Minimum width required (content won't fit in less).
    pub minimum: usize,
    /// Maximum width the content would use if given unlimited space.
    pub maximum: usize,
}

impl Measurement {
    /// Create a new measurement.
    pub fn new(minimum: usize, maximum: usize) -> Self {
        Measurement { minimum, maximum }
    }

    /// Create a measurement with both min and max set to the same value.
    pub fn exact(width: usize) -> Self {
        Measurement {
            minimum: width,
            maximum: width,
        }
    }

    /// Get the span (difference between max and min).
    pub fn span(&self) -> usize {
        self.maximum.saturating_sub(self.minimum)
    }

    /// Clamp a width to within the measurement bounds.
    pub fn clamp(&self, width: usize) -> usize {
        width.clamp(self.minimum, self.maximum)
    }

    /// Combine with another measurement, taking the max of mins and maxes.
    pub fn union(&self, other: &Measurement) -> Self {
        Measurement {
            minimum: self.minimum.max(other.minimum),
            maximum: self.maximum.max(other.maximum),
        }
    }

    /// Create a measurement from rendered segments.
    ///
    /// This is the default measurement strategy: render and measure the result.
    /// The minimum is the longest word, maximum is the total width.
    pub fn from_segments(segments: &Segments) -> Self {
        let mut total_width = 0;
        let mut max_word_width = 0;
        let mut current_word_width = 0;

        for segment in segments.iter() {
            for c in segment.text.chars() {
                let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                total_width += char_width;

                if c.is_whitespace() || c == '\n' {
                    max_word_width = max_word_width.max(current_word_width);
                    current_word_width = 0;
                } else {
                    current_word_width += char_width;
                }
            }
        }
        // Don't forget the last word
        max_word_width = max_word_width.max(current_word_width);

        Measurement {
            minimum: max_word_width,
            maximum: total_width,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_measurement() {
        let m = Measurement::new(10, 50);
        assert_eq!(m.span(), 40);
        assert_eq!(m.clamp(5), 10);
        assert_eq!(m.clamp(30), 30);
        assert_eq!(m.clamp(100), 50);
    }
}
