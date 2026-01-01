// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Error recovery and error collection from tree-sitter parse trees

use crate::ast::ParseError;
use crate::util::FileId;
use tree_sitter::{Node, Tree};
use super::text_range::TextRangeExt;

/// Collect all parse errors from a tree-sitter tree
pub fn collect_errors(tree: &Tree, file_id: FileId) -> Vec<ParseError> {
    let mut errors = Vec::new();
    collect_errors_recursive(tree.root_node(), file_id, &mut errors);
    errors
}

fn collect_errors_recursive(node: Node, file_id: FileId, errors: &mut Vec<ParseError>) {
    if node.is_error() {
        errors.push(ParseError {
            span: node.to_span(file_id),
            message: format!("Syntax error at {:?}", node.kind()),
        });
    } else if node.is_missing() {
        errors.push(ParseError {
            span: node.to_span(file_id),
            message: format!("Missing {}", node.kind()),
        });
    }

    // Recursively check children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_errors_recursive(child, file_id, errors);
    }
}

/// Check if a tree has any errors
pub fn has_errors(tree: &Tree) -> bool {
    has_errors_recursive(tree.root_node())
}

fn has_errors_recursive(node: Node) -> bool {
    if node.is_error() || node.is_missing() {
        return true;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if has_errors_recursive(child) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Tree {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_cpp::LANGUAGE.into()).unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_collect_errors_valid() {
        let tree = parse("int main() { return 0; }");
        let file_id = FileId::new(1);
        let errors = collect_errors(&tree, file_id);

        assert_eq!(errors.len(), 0);
        assert!(!has_errors(&tree));
    }

    #[test]
    fn test_collect_errors_invalid() {
        // Missing semicolon and brace
        let tree = parse("int main() { return 0 }");
        let file_id = FileId::new(1);
        let errors = collect_errors(&tree, file_id);

        assert!(errors.len() > 0);
        assert!(has_errors(&tree));
    }

    #[test]
    fn test_collect_errors_missing() {
        // Incomplete function
        let tree = parse("void foo(");
        let file_id = FileId::new(1);
        let errors = collect_errors(&tree, file_id);

        assert!(errors.len() > 0);
        assert!(has_errors(&tree));
    }
}
