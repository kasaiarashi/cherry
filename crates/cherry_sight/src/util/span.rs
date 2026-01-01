// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Source location tracking - file IDs, spans, positions, and text ranges

use std::fmt;
use std::ops::Range;

/// Unique identifier for a source file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub u32);

impl FileId {
    /// Create a new file ID
    pub const fn new(id: u32) -> Self {
        FileId(id)
    }

    /// Get the raw ID value
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FileId({})", self.0)
    }
}

/// A span representing a range in a source file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// The file this span belongs to
    pub file_id: FileId,
    /// Start byte offset
    pub start: u32,
    /// End byte offset (exclusive)
    pub end: u32,
}

impl Span {
    /// Create a new span
    pub const fn new(file_id: FileId, start: u32, end: u32) -> Self {
        Self { file_id, start, end }
    }

    /// Get the length of this span in bytes
    pub const fn len(&self) -> u32 {
        self.end - self.start
    }

    /// Check if this span is empty
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Check if this span contains a byte offset
    pub const fn contains(&self, offset: u32) -> bool {
        self.start <= offset && offset < self.end
    }

    /// Check if this span overlaps with another span in the same file
    pub fn overlaps(&self, other: &Span) -> bool {
        self.file_id == other.file_id
            && self.start < other.end
            && other.start < self.end
    }

    /// Merge two spans, returning the span that covers both
    pub fn merge(&self, other: &Span) -> Option<Span> {
        if self.file_id != other.file_id {
            return None;
        }

        Some(Span {
            file_id: self.file_id,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        })
    }

    /// Convert to a text range
    pub const fn to_range(&self) -> TextRange {
        TextRange {
            start: self.start,
            end: self.end,
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}..{}", self.file_id, self.start, self.end)
    }
}

/// A position in a source file (line and column)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// 0-based line number
    pub line: u32,
    /// 0-based column number (in UTF-8 bytes)
    pub column: u32,
}

impl Position {
    /// Create a new position
    pub const fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }

    /// Origin position (0, 0)
    pub const fn origin() -> Self {
        Self { line: 0, column: 0 }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line + 1, self.column + 1)
    }
}

/// A text range (byte offsets within a file)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextRange {
    /// Start byte offset
    pub start: u32,
    /// End byte offset (exclusive)
    pub end: u32,
}

impl TextRange {
    /// Create a new text range
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Create an empty range at a position
    pub const fn empty(at: u32) -> Self {
        Self { start: at, end: at }
    }

    /// Get the length of this range
    pub const fn len(&self) -> u32 {
        self.end - self.start
    }

    /// Check if this range is empty
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Check if this range contains an offset
    pub const fn contains(&self, offset: u32) -> bool {
        self.start <= offset && offset < self.end
    }

    /// Convert to a standard Range
    pub fn to_std_range(&self) -> Range<usize> {
        self.start as usize..self.end as usize
    }
}

impl From<Range<u32>> for TextRange {
    fn from(range: Range<u32>) -> Self {
        TextRange::new(range.start, range.end)
    }
}

impl From<Range<usize>> for TextRange {
    fn from(range: Range<usize>) -> Self {
        TextRange::new(range.start as u32, range.end as u32)
    }
}

impl fmt::Display for TextRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_basic() {
        let file = FileId::new(1);
        let span = Span::new(file, 10, 20);

        assert_eq!(span.len(), 10);
        assert!(!span.is_empty());
        assert!(span.contains(15));
        assert!(!span.contains(25));
    }

    #[test]
    fn test_span_overlaps() {
        let file = FileId::new(1);
        let span1 = Span::new(file, 10, 20);
        let span2 = Span::new(file, 15, 25);
        let span3 = Span::new(file, 30, 40);

        assert!(span1.overlaps(&span2));
        assert!(!span1.overlaps(&span3));
    }

    #[test]
    fn test_span_merge() {
        let file = FileId::new(1);
        let span1 = Span::new(file, 10, 20);
        let span2 = Span::new(file, 15, 25);

        let merged = span1.merge(&span2).unwrap();
        assert_eq!(merged.start, 10);
        assert_eq!(merged.end, 25);
    }

    #[test]
    fn test_position() {
        let pos = Position::new(5, 10);
        assert_eq!(pos.to_string(), "6:11"); // 1-based display
    }

    #[test]
    fn test_text_range() {
        let range = TextRange::new(10, 20);
        assert_eq!(range.len(), 10);
        assert!(range.contains(15));
        assert!(!range.contains(25));

        let empty = TextRange::empty(10);
        assert!(empty.is_empty());
    }
}
