//! Measurement: width requirements for renderables.

use crate::segment::Segments;
use crate::{Console, ConsoleOptions, Renderable};

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

    /// Normalize the measurement ensuring minimum <= maximum and both >= 0.
    ///
    /// Since we use `usize`, values are always >= 0, but this ensures
    /// the minimum does not exceed the maximum.
    ///
    /// # Examples
    ///
    /// ```
    /// use rich_rs::Measurement;
    ///
    /// // Inverted measurement gets corrected
    /// let m = Measurement::new(50, 10);
    /// let normalized = m.normalize();
    /// assert_eq!(normalized.minimum, 10);
    /// assert_eq!(normalized.maximum, 10);
    /// ```
    pub fn normalize(&self) -> Self {
        let minimum = self.minimum.min(self.maximum);
        Measurement {
            minimum,
            maximum: self.maximum,
        }
    }

    /// Get a measurement where both widths are <= the given width.
    ///
    /// # Examples
    ///
    /// ```
    /// use rich_rs::Measurement;
    ///
    /// let m = Measurement::new(10, 50);
    /// let constrained = m.with_maximum(30);
    /// assert_eq!(constrained.minimum, 10);
    /// assert_eq!(constrained.maximum, 30);
    ///
    /// // When width is less than minimum, both get clamped
    /// let m = Measurement::new(20, 50);
    /// let constrained = m.with_maximum(15);
    /// assert_eq!(constrained.minimum, 15);
    /// assert_eq!(constrained.maximum, 15);
    /// ```
    pub fn with_maximum(&self, width: usize) -> Self {
        Measurement {
            minimum: self.minimum.min(width),
            maximum: self.maximum.min(width),
        }
    }

    /// Get a measurement where both widths are >= the given width.
    ///
    /// # Examples
    ///
    /// ```
    /// use rich_rs::Measurement;
    ///
    /// let m = Measurement::new(10, 50);
    /// let constrained = m.with_minimum(20);
    /// assert_eq!(constrained.minimum, 20);
    /// assert_eq!(constrained.maximum, 50);
    ///
    /// // When width is greater than maximum, both get raised
    /// let m = Measurement::new(10, 30);
    /// let constrained = m.with_minimum(40);
    /// assert_eq!(constrained.minimum, 40);
    /// assert_eq!(constrained.maximum, 40);
    /// ```
    pub fn with_minimum(&self, width: usize) -> Self {
        Measurement {
            minimum: self.minimum.max(width),
            maximum: self.maximum.max(width),
        }
    }

    /// Clamp the measurement within optional min and max bounds.
    ///
    /// This clamps the measurement itself (both minimum and maximum fields),
    /// not a width value. Use `clamp_width` to clamp a width within measurement bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use rich_rs::Measurement;
    ///
    /// let m = Measurement::new(10, 50);
    ///
    /// // Clamp with both bounds
    /// let clamped = m.clamp_bounds(Some(15), Some(40));
    /// assert_eq!(clamped.minimum, 15);
    /// assert_eq!(clamped.maximum, 40);
    ///
    /// // Clamp with only max bound
    /// let clamped = m.clamp_bounds(None, Some(30));
    /// assert_eq!(clamped.minimum, 10);
    /// assert_eq!(clamped.maximum, 30);
    ///
    /// // Clamp with only min bound
    /// let clamped = m.clamp_bounds(Some(20), None);
    /// assert_eq!(clamped.minimum, 20);
    /// assert_eq!(clamped.maximum, 50);
    /// ```
    pub fn clamp_bounds(&self, min_width: Option<usize>, max_width: Option<usize>) -> Self {
        let mut result = *self;
        if let Some(min_w) = min_width {
            result = result.with_minimum(min_w);
        }
        if let Some(max_w) = max_width {
            result = result.with_maximum(max_w);
        }
        result
    }

    /// Clamp a width value to within the measurement bounds.
    ///
    /// Returns a width that is >= minimum and <= maximum.
    ///
    /// # Panics
    ///
    /// Panics if the measurement invariant is violated (i.e., `minimum > maximum`).
    /// In debug builds, a `debug_assert!` provides a clearer error message.
    /// Use [`normalize`](Self::normalize) to fix invalid measurements before
    /// calling this method.
    ///
    /// # Examples
    ///
    /// ```
    /// use rich_rs::Measurement;
    ///
    /// let m = Measurement::new(10, 50);
    /// assert_eq!(m.clamp_width(5), 10);   // Below minimum
    /// assert_eq!(m.clamp_width(30), 30);  // Within bounds
    /// assert_eq!(m.clamp_width(100), 50); // Above maximum
    /// ```
    #[track_caller]
    pub fn clamp_width(&self, width: usize) -> usize {
        debug_assert!(
            self.minimum <= self.maximum,
            "Measurement invariant violated: minimum ({}) > maximum ({})",
            self.minimum,
            self.maximum
        );
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
    /// The minimum is the longest word, maximum is the widest rendered line.
    pub fn from_segments(segments: &Segments) -> Self {
        let mut max_line_width = 0;
        let mut current_line_width = 0;
        let mut max_word_width = 0;
        let mut current_word_width = 0;

        for segment in segments.iter() {
            for c in segment.text.chars() {
                if c == '\n' {
                    max_line_width = max_line_width.max(current_line_width);
                    max_word_width = max_word_width.max(current_word_width);
                    current_line_width = 0;
                    current_word_width = 0;
                    continue;
                }

                let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                current_line_width += char_width;

                if c.is_whitespace() {
                    max_word_width = max_word_width.max(current_word_width);
                    current_word_width = 0;
                } else {
                    current_word_width += char_width;
                }
            }
        }

        // Account for trailing line / word when input doesn't end with '\n'.
        max_line_width = max_line_width.max(current_line_width);
        max_word_width = max_word_width.max(current_word_width);

        Measurement {
            minimum: max_word_width,
            maximum: max_line_width,
        }
    }
}

/// Get a combined measurement for multiple renderables.
///
/// Returns a measurement that would fit all the given renderables by taking
/// the maximum of all minimums and the maximum of all maximums.
///
/// # Examples
///
/// ```ignore
/// use rich_rs::{Console, ConsoleOptions, measure_renderables};
///
/// let console = Console::new();
/// let options = ConsoleOptions::default();
/// let renderables: Vec<&dyn Renderable> = vec![&"Hello", &"World!"];
/// let measurement = measure_renderables(&console, &options, &renderables);
/// ```
pub fn measure_renderables(
    console: &Console,
    options: &ConsoleOptions,
    renderables: &[&dyn Renderable],
) -> Measurement {
    if renderables.is_empty() {
        return Measurement::new(0, 0);
    }

    let mut max_minimum = 0;
    let mut max_maximum = 0;

    for renderable in renderables {
        let measurement = renderable.measure(console, options);
        max_minimum = max_minimum.max(measurement.minimum);
        max_maximum = max_maximum.max(measurement.maximum);
    }

    Measurement {
        minimum: max_minimum,
        maximum: max_maximum,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segment::Segment;

    #[test]
    fn test_measurement_basic() {
        let m = Measurement::new(10, 50);
        assert_eq!(m.minimum, 10);
        assert_eq!(m.maximum, 50);
    }

    #[test]
    fn test_measurement_exact() {
        let m = Measurement::exact(25);
        assert_eq!(m.minimum, 25);
        assert_eq!(m.maximum, 25);
        assert_eq!(m.span(), 0);
    }

    #[test]
    fn test_span() {
        let m = Measurement::new(10, 50);
        assert_eq!(m.span(), 40);

        // span uses saturating_sub, so inverted returns 0
        let inverted = Measurement::new(50, 10);
        assert_eq!(inverted.span(), 0);
    }

    #[test]
    fn test_normalize() {
        // Normal measurement stays the same
        let m = Measurement::new(10, 50);
        let normalized = m.normalize();
        assert_eq!(normalized.minimum, 10);
        assert_eq!(normalized.maximum, 50);

        // Inverted measurement gets minimum clamped to maximum
        let inverted = Measurement::new(50, 10);
        let normalized = inverted.normalize();
        assert_eq!(normalized.minimum, 10);
        assert_eq!(normalized.maximum, 10);

        // Equal values stay equal
        let equal = Measurement::new(25, 25);
        let normalized = equal.normalize();
        assert_eq!(normalized.minimum, 25);
        assert_eq!(normalized.maximum, 25);
    }

    #[test]
    fn test_with_maximum() {
        let m = Measurement::new(10, 50);

        // Width greater than maximum - no change
        let result = m.with_maximum(100);
        assert_eq!(result.minimum, 10);
        assert_eq!(result.maximum, 50);

        // Width between min and max - only max changes
        let result = m.with_maximum(30);
        assert_eq!(result.minimum, 10);
        assert_eq!(result.maximum, 30);

        // Width less than minimum - both get clamped
        let result = m.with_maximum(5);
        assert_eq!(result.minimum, 5);
        assert_eq!(result.maximum, 5);

        // Width equals minimum
        let result = m.with_maximum(10);
        assert_eq!(result.minimum, 10);
        assert_eq!(result.maximum, 10);
    }

    #[test]
    fn test_with_minimum() {
        let m = Measurement::new(10, 50);

        // Width less than minimum - no change
        let result = m.with_minimum(5);
        assert_eq!(result.minimum, 10);
        assert_eq!(result.maximum, 50);

        // Width between min and max - only min changes
        let result = m.with_minimum(30);
        assert_eq!(result.minimum, 30);
        assert_eq!(result.maximum, 50);

        // Width greater than maximum - both get raised
        let result = m.with_minimum(60);
        assert_eq!(result.minimum, 60);
        assert_eq!(result.maximum, 60);

        // Width equals maximum
        let result = m.with_minimum(50);
        assert_eq!(result.minimum, 50);
        assert_eq!(result.maximum, 50);
    }

    #[test]
    fn test_clamp_bounds() {
        let m = Measurement::new(10, 50);

        // Both bounds
        let clamped = m.clamp_bounds(Some(15), Some(40));
        assert_eq!(clamped.minimum, 15);
        assert_eq!(clamped.maximum, 40);

        // Only min bound
        let clamped = m.clamp_bounds(Some(20), None);
        assert_eq!(clamped.minimum, 20);
        assert_eq!(clamped.maximum, 50);

        // Only max bound
        let clamped = m.clamp_bounds(None, Some(30));
        assert_eq!(clamped.minimum, 10);
        assert_eq!(clamped.maximum, 30);

        // No bounds - no change
        let clamped = m.clamp_bounds(None, None);
        assert_eq!(clamped.minimum, 10);
        assert_eq!(clamped.maximum, 50);

        // Bounds that make min > max get corrected by ordering
        // with_minimum(40) -> (40, 50), then with_maximum(30) -> (30, 30)
        let clamped = m.clamp_bounds(Some(40), Some(30));
        assert_eq!(clamped.minimum, 30);
        assert_eq!(clamped.maximum, 30);
    }

    #[test]
    fn test_clamp_width() {
        let m = Measurement::new(10, 50);
        assert_eq!(m.clamp_width(5), 10); // Below minimum
        assert_eq!(m.clamp_width(10), 10); // At minimum
        assert_eq!(m.clamp_width(30), 30); // Within bounds
        assert_eq!(m.clamp_width(50), 50); // At maximum
        assert_eq!(m.clamp_width(100), 50); // Above maximum
    }

    #[test]
    fn test_union() {
        let m1 = Measurement::new(10, 50);
        let m2 = Measurement::new(15, 40);
        let combined = m1.union(&m2);
        assert_eq!(combined.minimum, 15);
        assert_eq!(combined.maximum, 50);

        let m3 = Measurement::new(5, 60);
        let combined = m1.union(&m3);
        assert_eq!(combined.minimum, 10);
        assert_eq!(combined.maximum, 60);
    }

    #[test]
    fn test_default() {
        let m = Measurement::default();
        assert_eq!(m.minimum, 0);
        assert_eq!(m.maximum, 0);
    }

    #[test]
    fn test_measure_renderables_empty() {
        let console = Console::new();
        let options = ConsoleOptions::default();
        let renderables: Vec<&dyn Renderable> = vec![];
        let measurement = measure_renderables(&console, &options, &renderables);
        assert_eq!(measurement.minimum, 0);
        assert_eq!(measurement.maximum, 0);
    }

    #[test]
    fn test_measure_renderables_single() {
        let console = Console::new();
        let options = ConsoleOptions::default();
        let text = String::from("Hello");
        let renderables: Vec<&dyn Renderable> = vec![&text];
        let measurement = measure_renderables(&console, &options, &renderables);
        // "Hello" has no spaces, so minimum == maximum == 5
        assert_eq!(measurement.minimum, 5);
        assert_eq!(measurement.maximum, 5);
    }

    #[test]
    fn test_measure_renderables_multiple() {
        let console = Console::new();
        let options = ConsoleOptions::default();
        let short = String::from("Hi");
        let long = String::from("Hello World");
        let renderables: Vec<&dyn Renderable> = vec![&short, &long];
        let measurement = measure_renderables(&console, &options, &renderables);
        // short: min=2, max=2
        // long: "Hello World" has min=5 (longest word), max=11
        // Combined: max(2,5)=5, max(2,11)=11
        assert_eq!(measurement.minimum, 5);
        assert_eq!(measurement.maximum, 11);
    }

    #[test]
    fn test_measure_renderables_takes_max_of_measurements() {
        let console = Console::new();
        let options = ConsoleOptions::default();
        let a = String::from("ABCDEFGHIJ"); // min=10, max=10 (no spaces)
        let b = String::from("XY Z"); // min=2 ("XY"), max=4
        let c = String::from("12345 67"); // min=5 ("12345"), max=8
        let renderables: Vec<&dyn Renderable> = vec![&a, &b, &c];
        let measurement = measure_renderables(&console, &options, &renderables);
        // max of minimums: max(10, 2, 5) = 10
        // max of maximums: max(10, 4, 8) = 10
        assert_eq!(measurement.minimum, 10);
        assert_eq!(measurement.maximum, 10);
    }

    #[test]
    fn test_from_segments_multiline_uses_widest_line() {
        let segments: Segments = vec![Segment::new("abcd\nef")].into();
        let measurement = Measurement::from_segments(&segments);

        assert_eq!(measurement.minimum, 4);
        assert_eq!(measurement.maximum, 4);
    }
}
