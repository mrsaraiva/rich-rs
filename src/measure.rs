//! Measurement: width requirements for renderables.

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
