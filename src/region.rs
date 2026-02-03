//! Region: Rectangle math for screen layouts.
//!
//! This module provides a `Region` type for representing rectangular areas
//! on the screen. It's used for layout calculations, clipping, and hit testing.

/// Defines a rectangular region of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Region {
    /// Create a new region.
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Create a region at origin.
    pub fn from_size(width: u32, height: u32) -> Self {
        Self::new(0, 0, width, height)
    }

    /// Check if this region contains a point.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x
            && y >= self.y
            && x < self.x + self.width as i32
            && y < self.y + self.height as i32
    }

    /// Check if this region contains another region.
    pub fn contains_region(&self, other: &Region) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.x + other.width as i32 <= self.x + self.width as i32
            && other.y + other.height as i32 <= self.y + self.height as i32
    }

    /// Get intersection of two regions. Returns None if no overlap.
    pub fn intersection(&self, other: &Region) -> Option<Region> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width as i32).min(other.x + other.width as i32);
        let y2 = (self.y + self.height as i32).min(other.y + other.height as i32);

        if x2 > x1 && y2 > y1 {
            Some(Region::new(x1, y1, (x2 - x1) as u32, (y2 - y1) as u32))
        } else {
            None
        }
    }

    /// Get bounding box union of two regions.
    pub fn union(&self, other: &Region) -> Region {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.width as i32).max(other.x + other.width as i32);
        let y2 = (self.y + self.height as i32).max(other.y + other.height as i32);

        Region::new(x1, y1, (x2 - x1) as u32, (y2 - y1) as u32)
    }

    /// Crop a region to fit within bounds.
    pub fn crop(&self, bounds: &Region) -> Option<Region> {
        self.intersection(bounds)
    }

    /// Offset the region by dx, dy.
    pub fn offset(&self, dx: i32, dy: i32) -> Region {
        Region::new(self.x + dx, self.y + dy, self.width, self.height)
    }

    /// Get the area.
    pub fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Check if region is empty (zero width or height).
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Basic Construction ===

    #[test]
    fn test_new() {
        let region = Region::new(10, 20, 30, 40);
        assert_eq!(region.x, 10);
        assert_eq!(region.y, 20);
        assert_eq!(region.width, 30);
        assert_eq!(region.height, 40);
    }

    #[test]
    fn test_from_size() {
        let region = Region::from_size(100, 50);
        assert_eq!(region.x, 0);
        assert_eq!(region.y, 0);
        assert_eq!(region.width, 100);
        assert_eq!(region.height, 50);
    }

    #[test]
    fn test_default() {
        let region = Region::default();
        assert_eq!(region, Region::new(0, 0, 0, 0));
    }

    #[test]
    fn test_negative_coordinates() {
        let region = Region::new(-10, -20, 30, 40);
        assert_eq!(region.x, -10);
        assert_eq!(region.y, -20);
    }

    // === Contains Point ===

    #[test]
    fn test_contains_point_inside() {
        let region = Region::new(10, 10, 20, 20);
        assert!(region.contains(15, 15));
        assert!(region.contains(10, 10)); // top-left corner
        assert!(region.contains(29, 29)); // just inside bottom-right
    }

    #[test]
    fn test_contains_point_outside() {
        let region = Region::new(10, 10, 20, 20);
        assert!(!region.contains(9, 15)); // left of region
        assert!(!region.contains(15, 9)); // above region
        assert!(!region.contains(30, 15)); // right of region (exclusive)
        assert!(!region.contains(15, 30)); // below region (exclusive)
    }

    #[test]
    fn test_contains_point_on_edge() {
        let region = Region::new(0, 0, 10, 10);
        // Top-left is inclusive
        assert!(region.contains(0, 0));
        // Bottom-right is exclusive
        assert!(!region.contains(10, 10));
        assert!(!region.contains(10, 0));
        assert!(!region.contains(0, 10));
    }

    #[test]
    fn test_contains_point_negative_coords() {
        let region = Region::new(-10, -10, 20, 20);
        assert!(region.contains(-5, -5));
        assert!(region.contains(0, 0));
        assert!(region.contains(9, 9));
        assert!(!region.contains(-11, 0));
        assert!(!region.contains(10, 0));
    }

    // === Contains Region ===

    #[test]
    fn test_contains_region_fully_inside() {
        let outer = Region::new(0, 0, 100, 100);
        let inner = Region::new(10, 10, 20, 20);
        assert!(outer.contains_region(&inner));
    }

    #[test]
    fn test_contains_region_same_size() {
        let region = Region::new(10, 10, 50, 50);
        assert!(region.contains_region(&region));
    }

    #[test]
    fn test_contains_region_partially_outside() {
        let outer = Region::new(0, 0, 100, 100);
        let partial = Region::new(90, 90, 20, 20); // extends past bounds
        assert!(!outer.contains_region(&partial));
    }

    #[test]
    fn test_contains_region_completely_outside() {
        let outer = Region::new(0, 0, 50, 50);
        let outside = Region::new(100, 100, 20, 20);
        assert!(!outer.contains_region(&outside));
    }

    #[test]
    fn test_contains_region_at_edge() {
        let outer = Region::new(0, 0, 100, 100);
        // Region that touches the exact boundary
        let at_edge = Region::new(80, 80, 20, 20);
        assert!(outer.contains_region(&at_edge));
        // Region that extends 1 pixel past
        let past_edge = Region::new(80, 80, 21, 20);
        assert!(!outer.contains_region(&past_edge));
    }

    // === Intersection ===

    #[test]
    fn test_intersection_overlapping() {
        let a = Region::new(0, 0, 20, 20);
        let b = Region::new(10, 10, 20, 20);
        let intersection = a.intersection(&b);
        assert_eq!(intersection, Some(Region::new(10, 10, 10, 10)));
    }

    #[test]
    fn test_intersection_no_overlap() {
        let a = Region::new(0, 0, 10, 10);
        let b = Region::new(20, 20, 10, 10);
        assert_eq!(a.intersection(&b), None);
    }

    #[test]
    fn test_intersection_touching_edges() {
        // Regions that share an edge but don't overlap
        let a = Region::new(0, 0, 10, 10);
        let b = Region::new(10, 0, 10, 10);
        assert_eq!(a.intersection(&b), None);
    }

    #[test]
    fn test_intersection_one_contains_other() {
        let outer = Region::new(0, 0, 100, 100);
        let inner = Region::new(20, 20, 30, 30);
        let intersection = outer.intersection(&inner);
        assert_eq!(intersection, Some(inner));
    }

    #[test]
    fn test_intersection_same_region() {
        let region = Region::new(10, 10, 50, 50);
        assert_eq!(region.intersection(&region), Some(region));
    }

    #[test]
    fn test_intersection_partial_overlap_horizontal() {
        let a = Region::new(0, 0, 30, 10);
        let b = Region::new(20, 0, 30, 10);
        assert_eq!(a.intersection(&b), Some(Region::new(20, 0, 10, 10)));
    }

    #[test]
    fn test_intersection_partial_overlap_vertical() {
        let a = Region::new(0, 0, 10, 30);
        let b = Region::new(0, 20, 10, 30);
        assert_eq!(a.intersection(&b), Some(Region::new(0, 20, 10, 10)));
    }

    #[test]
    fn test_intersection_with_negative_coords() {
        let a = Region::new(-20, -20, 30, 30);
        let b = Region::new(-10, -10, 30, 30);
        assert_eq!(a.intersection(&b), Some(Region::new(-10, -10, 20, 20)));
    }

    // === Union ===

    #[test]
    fn test_union_overlapping() {
        let a = Region::new(0, 0, 20, 20);
        let b = Region::new(10, 10, 20, 20);
        assert_eq!(a.union(&b), Region::new(0, 0, 30, 30));
    }

    #[test]
    fn test_union_disjoint() {
        let a = Region::new(0, 0, 10, 10);
        let b = Region::new(30, 30, 10, 10);
        assert_eq!(a.union(&b), Region::new(0, 0, 40, 40));
    }

    #[test]
    fn test_union_same_region() {
        let region = Region::new(10, 10, 50, 50);
        assert_eq!(region.union(&region), region);
    }

    #[test]
    fn test_union_one_contains_other() {
        let outer = Region::new(0, 0, 100, 100);
        let inner = Region::new(20, 20, 30, 30);
        assert_eq!(outer.union(&inner), outer);
    }

    #[test]
    fn test_union_with_negative_coords() {
        let a = Region::new(-20, -20, 10, 10);
        let b = Region::new(10, 10, 10, 10);
        assert_eq!(a.union(&b), Region::new(-20, -20, 40, 40));
    }

    // === Crop ===

    #[test]
    fn test_crop_within_bounds() {
        let region = Region::new(10, 10, 20, 20);
        let bounds = Region::new(0, 0, 100, 100);
        assert_eq!(region.crop(&bounds), Some(region));
    }

    #[test]
    fn test_crop_partially_outside() {
        let region = Region::new(90, 90, 20, 20);
        let bounds = Region::new(0, 0, 100, 100);
        assert_eq!(region.crop(&bounds), Some(Region::new(90, 90, 10, 10)));
    }

    #[test]
    fn test_crop_completely_outside() {
        let region = Region::new(200, 200, 20, 20);
        let bounds = Region::new(0, 0, 100, 100);
        assert_eq!(region.crop(&bounds), None);
    }

    // === Offset ===

    #[test]
    fn test_offset_positive() {
        let region = Region::new(10, 10, 20, 20);
        assert_eq!(region.offset(5, 10), Region::new(15, 20, 20, 20));
    }

    #[test]
    fn test_offset_negative() {
        let region = Region::new(10, 10, 20, 20);
        assert_eq!(region.offset(-5, -10), Region::new(5, 0, 20, 20));
    }

    #[test]
    fn test_offset_zero() {
        let region = Region::new(10, 10, 20, 20);
        assert_eq!(region.offset(0, 0), region);
    }

    #[test]
    fn test_offset_to_negative() {
        let region = Region::new(10, 10, 20, 20);
        assert_eq!(region.offset(-20, -20), Region::new(-10, -10, 20, 20));
    }

    // === Area ===

    #[test]
    fn test_area() {
        let region = Region::new(0, 0, 10, 20);
        assert_eq!(region.area(), 200);
    }

    #[test]
    fn test_area_zero() {
        let region = Region::new(0, 0, 0, 10);
        assert_eq!(region.area(), 0);
    }

    #[test]
    fn test_area_large() {
        // Test that area doesn't overflow with large dimensions
        let region = Region::new(0, 0, u32::MAX, 2);
        assert_eq!(region.area(), u32::MAX as u64 * 2);
    }

    // === Is Empty ===

    #[test]
    fn test_is_empty_zero_width() {
        let region = Region::new(10, 10, 0, 20);
        assert!(region.is_empty());
    }

    #[test]
    fn test_is_empty_zero_height() {
        let region = Region::new(10, 10, 20, 0);
        assert!(region.is_empty());
    }

    #[test]
    fn test_is_empty_both_zero() {
        let region = Region::new(10, 10, 0, 0);
        assert!(region.is_empty());
    }

    #[test]
    fn test_is_not_empty() {
        let region = Region::new(10, 10, 1, 1);
        assert!(!region.is_empty());
    }

    // === Edge Cases ===

    #[test]
    fn test_empty_region_contains() {
        let empty = Region::new(10, 10, 0, 0);
        assert!(!empty.contains(10, 10));
    }

    #[test]
    fn test_empty_region_intersection() {
        let empty = Region::new(10, 10, 0, 0);
        let normal = Region::new(0, 0, 100, 100);
        assert_eq!(empty.intersection(&normal), None);
    }

    #[test]
    fn test_empty_region_union() {
        let empty = Region::new(10, 10, 0, 0);
        let normal = Region::new(0, 0, 20, 20);
        // Union with empty region still expands to include the empty region's point
        assert_eq!(empty.union(&normal), Region::new(0, 0, 20, 20));
    }

    #[test]
    fn test_single_cell_region() {
        let cell = Region::new(5, 5, 1, 1);
        assert!(cell.contains(5, 5));
        assert!(!cell.contains(6, 5));
        assert!(!cell.contains(5, 6));
        assert_eq!(cell.area(), 1);
        assert!(!cell.is_empty());
    }
}
