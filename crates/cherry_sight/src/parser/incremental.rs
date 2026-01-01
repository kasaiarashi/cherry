// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Incremental parsing support using tree-sitter

use tree_sitter::{Parser, Tree, InputEdit, Point};
use std::ops::Range;

/// Incremental parser that caches the previous tree for efficient re-parsing
pub struct IncrementalParser {
    parser: Parser,
    cached_tree: Option<Tree>,
}

impl IncrementalParser {
    /// Create a new incremental parser
    pub fn new() -> anyhow::Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_cpp::LANGUAGE.into())?;

        Ok(Self {
            parser,
            cached_tree: None,
        })
    }

    /// Parse source code, using the cached tree if available
    pub fn parse(&mut self, source: &str) -> Option<Tree> {
        let tree = self.parser.parse(source, self.cached_tree.as_ref())?;
        self.cached_tree = Some(tree.clone());
        Some(tree)
    }

    /// Apply an edit and re-parse incrementally
    pub fn edit(&mut self, source: &str, edit: Edit) -> Option<Tree> {
        if let Some(ref mut tree) = self.cached_tree {
            tree.edit(&edit.to_input_edit());
        }

        self.parse(source)
    }

    /// Get the cached tree
    pub fn cached_tree(&self) -> Option<&Tree> {
        self.cached_tree.as_ref()
    }

    /// Clear the cached tree
    pub fn clear_cache(&mut self) {
        self.cached_tree = None;
    }
}

impl Default for IncrementalParser {
    fn default() -> Self {
        Self::new().expect("Failed to create incremental parser")
    }
}

/// Edit operation for incremental parsing
#[derive(Debug, Clone)]
pub struct Edit {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_position: Position,
    pub old_end_position: Position,
    pub new_end_position: Position,
}

impl Edit {
    /// Create a simple edit from byte ranges
    pub fn simple(range: Range<usize>, new_text: &str) -> Self {
        Self {
            start_byte: range.start,
            old_end_byte: range.end,
            new_end_byte: range.start + new_text.len(),
            start_position: Position::zero(),
            old_end_position: Position::zero(),
            new_end_position: Position::zero(),
        }
    }

    /// Convert to tree-sitter InputEdit
    fn to_input_edit(&self) -> InputEdit {
        InputEdit {
            start_byte: self.start_byte,
            old_end_byte: self.old_end_byte,
            new_end_byte: self.new_end_byte,
            start_position: self.start_position.to_point(),
            old_end_position: self.old_end_position.to_point(),
            new_end_position: self.new_end_position.to_point(),
        }
    }
}

/// Position in source code (line and column)
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub row: usize,
    pub column: usize,
}

impl Position {
    pub fn zero() -> Self {
        Self { row: 0, column: 0 }
    }

    fn to_point(&self) -> Point {
        Point {
            row: self.row,
            column: self.column,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incremental_parser_basic() {
        let mut parser = IncrementalParser::new().unwrap();

        let source1 = "int main() { return 0; }";
        let tree1 = parser.parse(source1).unwrap();

        assert!(!tree1.root_node().has_error());
    }

    #[test]
    fn test_incremental_parser_edit() {
        let mut parser = IncrementalParser::new().unwrap();

        let source1 = "int main() { return 0; }";
        let _tree1 = parser.parse(source1).unwrap();

        // Change "int" to "void"
        let edit = Edit::simple(0..3, "void");
        let source2 = "void main() { return 0; }";
        let tree2 = parser.edit(source2, edit).unwrap();

        assert!(!tree2.root_node().has_error());
    }

    #[test]
    fn test_incremental_parser_cache() {
        let mut parser = IncrementalParser::new().unwrap();

        assert!(parser.cached_tree().is_none());

        let source = "int main() {}";
        parser.parse(source);

        assert!(parser.cached_tree().is_some());

        parser.clear_cache();
        assert!(parser.cached_tree().is_none());
    }
}
