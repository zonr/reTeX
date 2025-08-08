#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    /// Byte offset in the source file.
    ///
    /// Uses u32 for memory efficiency as SourceLocation will be attached to every Token
    /// and AST node and diagnostics. This limits us to files smaller than 4GiB but we
    /// don't expect to encounter TeX files that large.
    ///
    /// This design follows Clang's SourceLocation, which uses a single integer to
    /// represent source positions efficiently.
    pub offset: u32,
}

impl SourceLocation {
    pub fn new(offset: u32) -> Self {
        Self { offset }
    }

    pub fn invalid() -> Self {
        Self::new(u32::MAX)
    }

    pub fn is_valid(self) -> bool {
        self.offset != u32::MAX
    }

    pub fn offset(self) -> u32 {
        self.offset
    }
}

impl Default for SourceLocation {
    fn default() -> Self {
        Self::invalid()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    pub start: SourceLocation,
    pub end: SourceLocation,
}

impl SourceRange {
    pub fn new(start: SourceLocation, end: SourceLocation) -> Self {
        Self { start, end }
    }

    pub fn invalid() -> Self {
        Self::new(SourceLocation::invalid(), SourceLocation::invalid())
    }

    pub fn is_valid(self) -> bool {
        self.start.is_valid() && self.end.is_valid()
    }

    pub fn length(self) -> u32 {
        if self.is_valid() {
            self.end.offset.saturating_sub(self.start.offset)
        } else {
            0
        }
    }
}

impl Default for SourceRange {
    fn default() -> Self {
        Self::invalid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_location_new() {
        let loc = SourceLocation::new(100);
        assert_eq!(loc.offset, 100);
        assert_eq!(loc.offset(), 100);
    }

    #[test]
    fn test_source_location_invalid() {
        let loc = SourceLocation::invalid();
        assert_eq!(loc.offset, u32::MAX);
        assert!(!loc.is_valid());
    }

    #[test]
    fn test_source_location_is_valid() {
        let valid_loc = SourceLocation::new(0);
        assert!(valid_loc.is_valid());

        let valid_loc2 = SourceLocation::new(1000);
        assert!(valid_loc2.is_valid());

        let invalid_loc = SourceLocation::invalid();
        assert!(!invalid_loc.is_valid());
    }

    #[test]
    fn test_source_location_default() {
        let loc = SourceLocation::default();
        assert!(!loc.is_valid());
        assert_eq!(loc, SourceLocation::invalid());
    }

    #[test]
    fn test_source_location_equality() {
        let loc1 = SourceLocation::new(50);
        let loc2 = SourceLocation::new(50);
        let loc3 = SourceLocation::new(100);

        assert_eq!(loc1, loc2);
        assert_ne!(loc1, loc3);
    }

    #[test]
    fn test_source_range_new() {
        let start = SourceLocation::new(10);
        let end = SourceLocation::new(20);
        let range = SourceRange::new(start, end);

        assert_eq!(range.start, start);
        assert_eq!(range.end, end);
    }

    #[test]
    fn test_source_range_invalid() {
        let range = SourceRange::invalid();
        assert!(!range.start.is_valid());
        assert!(!range.end.is_valid());
        assert!(!range.is_valid());
    }

    #[test]
    fn test_source_range_is_valid() {
        let valid_start = SourceLocation::new(10);
        let valid_end = SourceLocation::new(20);
        let valid_range = SourceRange::new(valid_start, valid_end);
        assert!(valid_range.is_valid());

        let invalid_start = SourceLocation::invalid();
        let invalid_range1 = SourceRange::new(invalid_start, valid_end);
        assert!(!invalid_range1.is_valid());

        let invalid_end = SourceLocation::invalid();
        let invalid_range2 = SourceRange::new(valid_start, invalid_end);
        assert!(!invalid_range2.is_valid());

        let fully_invalid_range = SourceRange::invalid();
        assert!(!fully_invalid_range.is_valid());
    }

    #[test]
    fn test_source_range_length() {
        let start = SourceLocation::new(10);
        let end = SourceLocation::new(25);
        let range = SourceRange::new(start, end);
        assert_eq!(range.length(), 15);

        // Test same start and end
        let same_range = SourceRange::new(start, start);
        assert_eq!(same_range.length(), 0);

        // Test invalid range
        let invalid_range = SourceRange::invalid();
        assert_eq!(invalid_range.length(), 0);

        // Test with partial invalid range
        let partial_invalid = SourceRange::new(SourceLocation::invalid(), end);
        assert_eq!(partial_invalid.length(), 0);
    }

    #[test]
    fn test_source_range_length_saturating_sub() {
        // Test case where end is before start (should not happen in practice but let's test saturating_sub)
        let start = SourceLocation::new(20);
        let end = SourceLocation::new(10);
        let range = SourceRange::new(start, end);
        assert_eq!(range.length(), 0); // saturating_sub should give 0
    }

    #[test]
    fn test_source_range_default() {
        let range = SourceRange::default();
        assert!(!range.is_valid());
        assert_eq!(range, SourceRange::invalid());
        assert_eq!(range.length(), 0);
    }

    #[test]
    fn test_source_range_equality() {
        let start1 = SourceLocation::new(10);
        let end1 = SourceLocation::new(20);
        let range1 = SourceRange::new(start1, end1);

        let start2 = SourceLocation::new(10);
        let end2 = SourceLocation::new(20);
        let range2 = SourceRange::new(start2, end2);

        let range3 = SourceRange::new(start1, SourceLocation::new(25));

        assert_eq!(range1, range2);
        assert_ne!(range1, range3);
    }

    #[test]
    fn test_source_location_clone_copy() {
        let loc = SourceLocation::new(42);
        let cloned = loc.clone();
        let copied = loc;

        assert_eq!(loc, cloned);
        assert_eq!(loc, copied);
    }

    #[test]
    fn test_source_range_clone_copy() {
        let range = SourceRange::new(SourceLocation::new(10), SourceLocation::new(20));
        let cloned = range.clone();
        let copied = range;

        assert_eq!(range, cloned);
        assert_eq!(range, copied);
    }
}