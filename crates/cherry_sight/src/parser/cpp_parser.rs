// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! C++ parser using tree-sitter-cpp

use crate::ast::*;
use crate::util::{FileId, Interner, InternedString};
use super::text_range::TextRangeExt;
use super::error_recovery::collect_errors;
use tree_sitter::{Parser, Tree, Node};
use anyhow::Result;

/// C++ parser that converts tree-sitter trees to our AST
pub struct CppParser {
    parser: Parser,
    interner: Interner,
}

impl CppParser {
    /// Create a new C++ parser
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_cpp::LANGUAGE.into())?;

        Ok(Self {
            parser,
            interner: Interner::new(),
        })
    }

    /// Parse source code into a translation unit
    pub fn parse(&mut self, source: &str, file_id: FileId) -> Result<TranslationUnit> {
        let tree = self.parser.parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse source"))?;

        self.tree_to_ast(&tree, source, file_id)
    }

    /// Convert a tree-sitter tree to our AST
    pub fn tree_to_ast(&mut self, tree: &Tree, source: &str, file_id: FileId) -> Result<TranslationUnit> {
        let root = tree.root_node();
        let errors = collect_errors(tree, file_id);

        let mut declarations = Vec::new();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            if let Some(decl) = self.parse_declaration(child, source, file_id) {
                declarations.push(decl);
            }
        }

        Ok(TranslationUnit {
            file_id,
            declarations,
            errors,
        })
    }

    /// Parse a declaration node
    fn parse_declaration(&mut self, node: Node, source: &str, file_id: FileId) -> Option<Declaration> {
        match node.kind() {
            "function_definition" => self.parse_function(node, source, file_id).map(Declaration::Function),
            "class_specifier" => self.parse_class(node, source, file_id, false).map(Declaration::Class),
            "struct_specifier" => self.parse_class(node, source, file_id, true).map(Declaration::Struct),
            "enum_specifier" => self.parse_enum(node, source, file_id).map(Declaration::Enum),
            "namespace_definition" => self.parse_namespace(node, source, file_id).map(Declaration::Namespace),
            "declaration" => self.parse_variable_declaration(node, source, file_id).map(Declaration::Variable),
            _ => None,
        }
    }

    /// Parse a function definition
    fn parse_function(&mut self, node: Node, source: &str, file_id: FileId) -> Option<FunctionDecl> {
        let span = node.to_span(file_id);

        // Find function name (simplified)
        let name = self.find_identifier(node, source)?;

        // Parse return type (simplified - just use void for now)
        let return_type = Type::Void;

        // Parse parameters (simplified)
        let parameters = Vec::new();

        Some(FunctionDecl {
            name,
            span,
            return_type,
            parameters,
            is_const: false,
            is_static: false,
            is_virtual: false,
            is_override: false,
            is_final: false,
            is_inline: false,
            body: Some(FunctionBody { span }),
        })
    }

    /// Parse a class or struct
    fn parse_class(&mut self, node: Node, source: &str, file_id: FileId, is_struct: bool) -> Option<ClassDecl> {
        let span = node.to_span(file_id);

        // Find class name
        let name = self.find_identifier(node, source)?;

        Some(ClassDecl {
            name,
            span,
            access: if is_struct { AccessSpecifier::Public } else { AccessSpecifier::Private },
            bases: Vec::new(),
            members: Vec::new(),
            is_struct,
            template_params: None,
        })
    }

    /// Parse an enum
    fn parse_enum(&mut self, node: Node, source: &str, file_id: FileId) -> Option<EnumDecl> {
        let span = node.to_span(file_id);

        // Find enum name
        let name = self.find_identifier(node, source)?;

        Some(EnumDecl {
            name,
            span,
            is_class: false,
            underlying_type: None,
            variants: Vec::new(),
        })
    }

    /// Parse a namespace
    fn parse_namespace(&mut self, node: Node, source: &str, file_id: FileId) -> Option<NamespaceDecl> {
        let span = node.to_span(file_id);

        // Find namespace name (direct child only, not recursive)
        let mut name = None;
        let mut declarations = Vec::new();
        let mut cursor = node.walk();

        // Parse namespace children
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "namespace_identifier" {
                let text = &source[child.byte_range()];
                name = Some(self.interner.intern(text));
            } else if child.kind() == "declaration_list" {
                // Parse namespace body
                let mut body_cursor = child.walk();
                for decl_node in child.children(&mut body_cursor) {
                    if let Some(decl) = self.parse_declaration(decl_node, source, file_id) {
                        declarations.push(decl);
                    }
                }
            }
        }

        Some(NamespaceDecl {
            name,
            span,
            declarations,
        })
    }

    /// Parse a variable declaration
    fn parse_variable_declaration(&mut self, node: Node, source: &str, file_id: FileId) -> Option<VariableDecl> {
        let span = node.to_span(file_id);

        // Find variable name
        let name = self.find_identifier(node, source)?;

        Some(VariableDecl {
            name,
            ty: Type::Auto, // Simplified
            span,
            is_const: false,
            is_static: false,
            initializer: None,
        })
    }

    /// Find an identifier in a node
    fn find_identifier(&mut self, node: Node, source: &str) -> Option<InternedString> {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "type_identifier" {
                let text = &source[child.byte_range()];
                return Some(self.interner.intern(text));
            }

            // Recursively search in child nodes
            if let Some(id) = self.find_identifier(child, source) {
                return Some(id);
            }
        }

        None
    }

    /// Get the string interner
    pub fn interner(&self) -> &Interner {
        &self.interner
    }
}

impl Default for CppParser {
    fn default() -> Self {
        Self::new().expect("Failed to create C++ parser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let mut parser = CppParser::new().unwrap();
        let source = "int main() { return 0; }";
        let file_id = FileId::new(1);

        let result = parser.parse(source, file_id);
        assert!(result.is_ok());

        let unit = result.unwrap();
        assert_eq!(unit.declarations.len(), 1);

        match &unit.declarations[0] {
            Declaration::Function(func) => {
                let name = parser.interner().resolve(func.name);
                assert_eq!(name, "main");
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn test_parse_class() {
        let mut parser = CppParser::new().unwrap();
        let source = "class MyClass {};";
        let file_id = FileId::new(1);

        let result = parser.parse(source, file_id);
        assert!(result.is_ok());

        let unit = result.unwrap();
        assert_eq!(unit.declarations.len(), 1);

        match &unit.declarations[0] {
            Declaration::Class(class) => {
                let name = parser.interner().resolve(class.name);
                assert_eq!(name, "MyClass");
                assert!(!class.is_struct);
            }
            _ => panic!("Expected class declaration"),
        }
    }

    #[test]
    fn test_parse_namespace() {
        let mut parser = CppParser::new().unwrap();
        let source = "namespace MyNamespace { void foo(); }";
        let file_id = FileId::new(1);

        let result = parser.parse(source, file_id);
        assert!(result.is_ok());

        let unit = result.unwrap();
        assert_eq!(unit.declarations.len(), 1);

        match &unit.declarations[0] {
            Declaration::Namespace(ns) => {
                assert!(ns.name.is_some());
                let name = parser.interner().resolve(ns.name.unwrap());
                assert_eq!(name, "MyNamespace");
            }
            _ => panic!("Expected namespace declaration"),
        }
    }

    #[test]
    fn test_parse_with_errors() {
        let mut parser = CppParser::new().unwrap();
        let source = "int main() { return 0 }"; // Missing semicolon
        let file_id = FileId::new(1);

        let result = parser.parse(source, file_id);
        assert!(result.is_ok());

        let unit = result.unwrap();
        assert!(unit.errors.len() > 0, "Should have parse errors");
    }
}
