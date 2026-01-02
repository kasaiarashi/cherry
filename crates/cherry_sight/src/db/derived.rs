// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Derived queries for parsing and analysis

use crate::ast::{ParseError, TranslationUnit};
use crate::db::SourceFile;
use crate::parser::CppParser;
use crate::util::Interner;
use parking_lot::RwLock;
use std::sync::Arc;

/// Parse a source file into a translation unit
pub fn parse(source: &SourceFile) -> Arc<TranslationUnit> {
    let content = &source.content;
    let file_id = source.file_id;

    // Parse the source code with a temporary interner
    // Note: For LSP usage, use the shared interner via LspHandlers instead
    let interner = Arc::new(RwLock::new(Interner::new()));
    let mut parser = CppParser::new(interner).expect("Failed to create parser");
    let result = parser.parse(content, file_id).expect("Failed to parse");

    Arc::new(result)
}

/// Get syntax errors for a file
pub fn syntax_errors(source: &SourceFile) -> Arc<Vec<ParseError>> {
    let unit = parse(source);
    Arc::new(unit.errors.clone())
}

/// Line index for converting between byte offsets and line/column positions
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// Byte offset of the start of each line
    pub line_starts: Vec<u32>,
}

impl LineIndex {
    /// Create a line index from source content
    pub fn new(content: &str) -> Self {
        let mut line_starts = vec![0];

        for (i, byte) in content.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push((i + 1) as u32);
            }
        }

        Self { line_starts }
    }

    /// Convert byte offset to line number (0-indexed)
    pub fn line(&self, offset: u32) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(line) => line,
            Err(next_line) => next_line.saturating_sub(1),
        }
    }

    /// Convert byte offset to line and column (0-indexed)
    pub fn line_col(&self, offset: u32) -> (usize, usize) {
        let line = self.line(offset);
        let line_start = self.line_starts[line];
        let col = (offset - line_start) as usize;
        (line, col)
    }

    /// Get the byte offset of a line start
    pub fn line_start(&self, line: usize) -> Option<u32> {
        self.line_starts.get(line).copied()
    }
}

/// Get line index for a file
pub fn line_index(source: &SourceFile) -> Arc<LineIndex> {
    let content = &source.content;
    Arc::new(LineIndex::new(content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_index_basic() {
        let content = "line 1\nline 2\nline 3";
        let index = LineIndex::new(content);

        assert_eq!(index.line(0), 0); // First character of line 1
        assert_eq!(index.line(7), 1); // First character of line 2
        assert_eq!(index.line(14), 2); // First character of line 3

        assert_eq!(index.line_col(0), (0, 0));
        assert_eq!(index.line_col(3), (0, 3)); // 'e' in "line"
        assert_eq!(index.line_col(7), (1, 0)); // 'l' in second "line"
    }

    #[test]
    fn test_line_index_empty() {
        let content = "";
        let index = LineIndex::new(content);

        assert_eq!(index.line(0), 0);
        assert_eq!(index.line_starts.len(), 1);
    }

    #[test]
    fn test_line_index_single_line() {
        let content = "single line";
        let index = LineIndex::new(content);

        assert_eq!(index.line(0), 0);
        assert_eq!(index.line(5), 0);
        assert_eq!(index.line_starts.len(), 1);
    }

    #[test]
    fn test_line_start() {
        let content = "line 1\nline 2\nline 3";
        let index = LineIndex::new(content);

        assert_eq!(index.line_start(0), Some(0));
        assert_eq!(index.line_start(1), Some(7));
        assert_eq!(index.line_start(2), Some(14));
        assert_eq!(index.line_start(3), None);
    }
}
