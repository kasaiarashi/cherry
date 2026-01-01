// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Text range utilities for parser

use crate::util::{Span, TextRange, FileId};
use tree_sitter::Node;

/// Extension trait for tree-sitter nodes
pub trait TextRangeExt {
    /// Convert tree-sitter node to our Span type
    fn to_span(&self, file_id: FileId) -> Span;

    /// Convert tree-sitter node to our TextRange type
    fn to_text_range(&self) -> TextRange;
}

impl TextRangeExt for Node<'_> {
    fn to_span(&self, file_id: FileId) -> Span {
        Span::new(
            file_id,
            self.start_byte() as u32,
            self.end_byte() as u32,
        )
    }

    fn to_text_range(&self) -> TextRange {
        TextRange::new(
            self.start_byte() as u32,
            self.end_byte() as u32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_range_conversion() {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_cpp::LANGUAGE.into()).unwrap();

        let source = "int main() {}";
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        let file_id = FileId::new(1);
        let span = root.to_span(file_id);

        assert_eq!(span.file_id, file_id);
        assert_eq!(span.start, 0);
        assert_eq!(span.end, source.len() as u32);
    }
}
