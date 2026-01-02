// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! C++ parser using tree-sitter-cpp

use crate::ast::*;
use crate::util::{FileId, Interner, InternedString, Span};
use super::text_range::TextRangeExt;
use super::error_recovery::collect_errors;
use tree_sitter::{Parser, Tree, Node};
use anyhow::Result;
use parking_lot::RwLock;
use std::sync::Arc;

/// C++ parser that converts tree-sitter trees to our AST
pub struct CppParser {
    parser: Parser,
    interner: Arc<RwLock<Interner>>,
}

impl CppParser {
    /// Create a new C++ parser with a shared interner
    pub fn new(interner: Arc<RwLock<Interner>>) -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_cpp::LANGUAGE.into())?;

        Ok(Self {
            parser,
            interner,
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

        log::info!("Parsing file {:?} with root node kind: {}", file_id, root.kind());

        let mut declarations = Vec::new();
        let mut cursor = root.walk();
        let mut node_kinds = std::collections::HashMap::new();

        for child in root.children(&mut cursor) {
            let kind = child.kind();
            *node_kinds.entry(kind).or_insert(0) += 1;

            if let Some(decl) = self.parse_declaration(child, source, file_id) {
                declarations.push(decl);
            } else {
                log::debug!("Skipped node kind: '{}' at byte range {}..{}", kind, child.start_byte(), child.end_byte());
            }
        }

        log::info!("Node kinds found: {:?}", node_kinds);
        log::info!("Extracted {} declarations from {} total nodes", declarations.len(), node_kinds.values().sum::<i32>());

        Ok(TranslationUnit {
            file_id,
            declarations,
            errors,
        })
    }

    /// Extract documentation comment before a node
    fn extract_doc_comment(&self, node: Node, source: &str) -> Option<String> {
        // Look for comment nodes immediately before this node
        let mut current = node;
        let mut comments = Vec::new();

        // Walk backwards from the node to find preceding comments
        while let Some(prev) = current.prev_sibling() {
            match prev.kind() {
                "comment" => {
                    let comment_text = &source[prev.byte_range()];
                    // Parse different comment styles
                    let cleaned = if comment_text.starts_with("///") {
                        // Doxygen-style single-line
                        comment_text.trim_start_matches("///").trim().to_string()
                    } else if comment_text.starts_with("//!") {
                        // Rust-style doc comment
                        comment_text.trim_start_matches("//!").trim().to_string()
                    } else if comment_text.starts_with("/**") && comment_text.ends_with("*/") {
                        // Multi-line doc comment
                        let inner = comment_text.trim_start_matches("/**").trim_end_matches("*/");
                        // Clean up each line
                        inner.lines()
                            .map(|line| line.trim().trim_start_matches('*').trim())
                            .collect::<Vec<_>>()
                            .join("\n")
                    } else if comment_text.starts_with("//") {
                        // Regular comment might still be documentation
                        comment_text.trim_start_matches("//").trim().to_string()
                    } else {
                        continue;
                    };
                    comments.push(cleaned);
                    current = prev;
                }
                // Skip whitespace
                kind if kind.contains("whitespace") || kind == "\n" => {
                    current = prev;
                }
                // Stop at any other node
                _ => break,
            }
        }

        if comments.is_empty() {
            None
        } else {
            comments.reverse();
            Some(comments.join("\n"))
        }
    }

    /// Parse a declaration node
    fn parse_declaration(&mut self, node: Node, source: &str, file_id: FileId) -> Option<Declaration> {
        match node.kind() {
            "function_definition" => {
                // Check if this is actually a misidentified class with export macro
                // Tree-sitter sometimes sees "class EXPORT_MACRO ClassName { ... }" as function_definition
                // Possible structures:
                // 1. Simple: class_specifier("class EXPORT"), identifier("ClassName"), compound_statement
                // 2. With base: class_specifier("class EXPORT"), ERROR("ClassName : public"), identifier("BaseClass"), compound_statement
                let mut cursor = node.walk();
                let mut class_spec = None;
                let mut struct_spec = None;
                let mut error_node = None;
                let mut class_name_identifier = None;
                let mut body = None;

                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "class_specifier" => class_spec = Some(child),
                        "struct_specifier" => struct_spec = Some(child),
                        "ERROR" => error_node = Some(child),
                        "identifier" => class_name_identifier = Some(child),
                        "compound_statement" => body = Some(child),
                        _ => {}
                    }
                }

                if let Some(class_node) = class_spec {
                    log::info!("Found class_specifier inside function_definition (export macro confusion)");

                    // If there's an ERROR node, the class name is likely in there (before " : public")
                    if let Some(err_node) = error_node {
                        let err_text = &source[err_node.byte_range()];
                        log::info!("Found ERROR node: '{}'", err_text);

                        // Extract class name from error text (before " : " or " :")
                        if let Some(class_name) = err_text.split(':').next().map(|s| s.trim()) {
                            if !class_name.is_empty() {
                                log::info!("Extracted class name from ERROR node: '{}'", class_name);
                                return self.parse_class_with_export_macro_from_text(
                                    class_node, class_name, body, source, file_id, false
                                ).map(Declaration::Class);
                            }
                        }
                    }

                    // Otherwise, use the identifier sibling
                    if let Some(name_node) = class_name_identifier {
                        let class_name = &source[name_node.byte_range()];
                        log::info!("Real class name from identifier: '{}'", class_name);
                        return self.parse_class_with_export_macro_from_text(
                            class_node, class_name, body, source, file_id, false
                        ).map(Declaration::Class);
                    }

                    // Fallback: use the old method if no identifier found
                    return self.parse_class(class_node, source, file_id, false).map(Declaration::Class);
                }

                if let Some(struct_node) = struct_spec {
                    log::info!("Found struct_specifier inside function_definition (export macro confusion)");

                    // If there's an ERROR node, the struct name is likely in there
                    if let Some(err_node) = error_node {
                        let err_text = &source[err_node.byte_range()];
                        log::info!("Found ERROR node: '{}'", err_text);

                        if let Some(struct_name) = err_text.split(':').next().map(|s| s.trim()) {
                            if !struct_name.is_empty() {
                                log::info!("Extracted struct name from ERROR node: '{}'", struct_name);
                                return self.parse_class_with_export_macro_from_text(
                                    struct_node, struct_name, body, source, file_id, true
                                ).map(Declaration::Struct);
                            }
                        }
                    }

                    // Otherwise, use the identifier sibling
                    if let Some(name_node) = class_name_identifier {
                        let struct_name = &source[name_node.byte_range()];
                        log::info!("Real struct name from identifier: '{}'", struct_name);
                        return self.parse_class_with_export_macro_from_text(
                            struct_node, struct_name, body, source, file_id, true
                        ).map(Declaration::Struct);
                    }

                    // Fallback: use the old method if no identifier found
                    return self.parse_class(struct_node, source, file_id, true).map(Declaration::Struct);
                }

                // Actually a function
                self.parse_function(node, source, file_id).map(Declaration::Function)
            }
            "class_specifier" => self.parse_class(node, source, file_id, false).map(Declaration::Class),
            "struct_specifier" => self.parse_class(node, source, file_id, true).map(Declaration::Struct),
            "enum_specifier" => self.parse_enum(node, source, file_id).map(Declaration::Enum),
            "namespace_definition" => self.parse_namespace(node, source, file_id).map(Declaration::Namespace),

            // "declaration" nodes might contain class/struct with export macros (e.g., class MY_API MyClass)
            "declaration" => {
                // First check if it contains a class_specifier or struct_specifier
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "class_specifier" => {
                            log::info!("Found class_specifier inside declaration node (likely has export macro)");
                            return self.parse_class(child, source, file_id, false).map(Declaration::Class);
                        }
                        "struct_specifier" => {
                            log::info!("Found struct_specifier inside declaration node (likely has export macro)");
                            return self.parse_class(child, source, file_id, true).map(Declaration::Struct);
                        }
                        _ => {}
                    }
                }
                // Fall back to variable declaration
                self.parse_variable_declaration(node, source, file_id).map(Declaration::Variable)
            }

            // Handle ERROR nodes - try to find class_specifier or struct_specifier inside
            "ERROR" => {
                log::info!("Attempting to recover from ERROR node at {}..{}", node.start_byte(), node.end_byte());
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if let Some(decl) = self.parse_declaration(child, source, file_id) {
                        return Some(decl);
                    }
                }
                None
            }

            // Handle expression_statement - might be UE5 macros or contain class definitions
            "expression_statement" => {
                // Log what's in the expression statement
                let text = &source[node.byte_range()];
                let preview = if text.len() > 100 { &text[..100] } else { text };
                log::info!("expression_statement at {}..{}: {}", node.start_byte(), node.end_byte(), preview);

                // Check if it contains a class_specifier or struct_specifier
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "class_specifier" => {
                            log::info!("Found class_specifier inside expression_statement!");
                            return self.parse_class(child, source, file_id, false).map(Declaration::Class);
                        }
                        "struct_specifier" => {
                            log::info!("Found struct_specifier inside expression_statement!");
                            return self.parse_class(child, source, file_id, true).map(Declaration::Struct);
                        }
                        _ => {}
                    }
                }

                // Check if next sibling is a class_specifier
                if let Some(next) = node.next_sibling() {
                    log::info!("Next sibling after expression_statement: kind='{}' at {}..{}",
                        next.kind(), next.start_byte(), next.end_byte());

                    match next.kind() {
                        "class_specifier" => {
                            log::info!("Next node IS a class_specifier, but will be parsed in next iteration");
                        }
                        "struct_specifier" => {
                            log::info!("Next node IS a struct_specifier, but will be parsed in next iteration");
                        }
                        _ => {
                            log::info!("Next node is NOT a class/struct specifier");
                        }
                    }
                }
                None
            }

            _ => None,
        }
    }

    /// Parse a function definition
    fn parse_function(&mut self, node: Node, source: &str, file_id: FileId) -> Option<FunctionDecl> {
        let span = node.to_span(file_id);

        // Extract documentation comment
        let doc_comment = self.extract_doc_comment(node, source);

        let mut cursor = node.walk();
        let mut return_type = Type::Void;
        let mut name = None;
        let mut name_span = span;
        let mut parameters = Vec::new();
        let mut is_const = false;
        let mut is_static = false;
        let mut is_virtual = false;
        let mut is_override = false;
        let mut is_final = false;
        let mut is_inline = false;

        // Parse function components from tree-sitter nodes
        for child in node.children(&mut cursor) {
            match child.kind() {
                // Storage class specifiers
                "storage_class_specifier" => {
                    let specifier = &source[child.byte_range()];
                    if specifier == "static" {
                        is_static = true;
                    } else if specifier == "inline" {
                        is_inline = true;
                    }
                }
                // Virtual specifier
                "virtual_specifier" | "virtual" => {
                    is_virtual = true;
                }
                // Type specifier (return type)
                "type_qualifier" | "primitive_type" | "type_identifier" | "qualified_identifier" | "template_type" => {
                    return_type = self.parse_type_from_node(child, source);
                }
                // Function declarator contains the name and parameters
                "function_declarator" => {
                    if let Some((func_name, func_name_span, params)) = self.parse_function_declarator(child, source, file_id) {
                        name = Some(func_name);
                        name_span = func_name_span;
                        parameters = params;
                    }
                }
                // Type qualifiers after parameters (const, override, final)
                "type_qualifier" if child.start_byte() > node.start_byte() + (node.byte_range().len() / 2) => {
                    let qualifier = &source[child.byte_range()];
                    if qualifier == "const" {
                        is_const = true;
                    }
                }
                // Virtual function specifiers
                "virtual_function_specifier" => {
                    let specifier = &source[child.byte_range()];
                    if specifier == "override" {
                        is_override = true;
                    } else if specifier == "final" {
                        is_final = true;
                    }
                }
                _ => {}
            }
        }

        let name = name?;

        Some(FunctionDecl {
            name,
            span: name_span,  // Use the precise name span, not the entire function
            return_type,
            parameters,
            is_const,
            is_static,
            is_virtual,
            is_override,
            is_final,
            is_inline,
            body: Some(FunctionBody {
                span,
                statements: Vec::new(), // Statement parsing not yet implemented
            }),
            doc_comment,
        })
    }

    /// Parse a function declarator to extract name and parameters
    fn parse_function_declarator(&mut self, node: Node, source: &str, file_id: FileId) -> Option<(InternedString, Span, Vec<crate::ast::Parameter>)> {
        let mut cursor = node.walk();
        let mut name = None;
        let mut name_span = None;
        let mut parameters = Vec::new();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" | "field_identifier" | "destructor_name" => {
                    let text = &source[child.byte_range()];
                    name = Some(self.interner.write().intern(text));
                    name_span = Some(child.to_span(file_id));
                }
                "qualified_identifier" => {
                    // For qualified names like ClassName::MethodName, get the last part
                    if let Some(id) = self.find_last_identifier(child, source) {
                        name = Some(id);
                        name_span = Some(child.to_span(file_id));
                    }
                }
                "parameter_list" => {
                    parameters = self.parse_parameter_list(child, source, file_id);
                }
                "function_declarator" => {
                    // Nested declarator (e.g., function pointer)
                    if let Some((n, ns, _)) = self.parse_function_declarator(child, source, file_id) {
                        name = Some(n);
                        name_span = Some(ns);
                    }
                }
                _ => {}
            }
        }

        Some((name?, name_span?, parameters))
    }

    /// Parse a parameter list
    fn parse_parameter_list(&mut self, node: Node, source: &str, file_id: FileId) -> Vec<crate::ast::Parameter> {
        let mut parameters = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "parameter_declaration" || child.kind() == "optional_parameter_declaration" {
                if let Some(param) = self.parse_parameter(child, source, file_id) {
                    parameters.push(param);
                }
            }
        }

        parameters
    }

    /// Parse a single parameter
    fn parse_parameter(&mut self, node: Node, source: &str, file_id: FileId) -> Option<crate::ast::Parameter> {
        let span = node.to_span(file_id);
        let mut cursor = node.walk();
        let mut param_type = Type::Auto;
        let mut name = None;

        for child in node.children(&mut cursor) {
            match child.kind() {
                "type_qualifier" | "primitive_type" | "type_identifier" | "qualified_identifier" => {
                    param_type = self.parse_type_from_node(child, source);
                }
                "identifier" => {
                    let text = &source[child.byte_range()];
                    name = Some(self.interner.write().intern(text));
                }
                "declarator" | "reference_declarator" | "pointer_declarator" => {
                    // Handle pointer/reference declarators
                    if let Some(id) = self.find_identifier(child, source) {
                        name = Some(id);
                    }
                }
                _ => {}
            }
        }

        Some(crate::ast::Parameter {
            name,
            ty: param_type,
            span,
            default_value: None,
        })
    }

    /// Parse a type from a tree-sitter node
    fn parse_type_from_node(&mut self, node: Node, source: &str) -> Type {
        match node.kind() {
            "primitive_type" => {
                let type_name = &source[node.byte_range()];
                match type_name {
                    "void" => Type::Void,
                    "bool" => Type::Primitive(PrimitiveType::Bool),
                    "char" => Type::Primitive(PrimitiveType::Char),
                    "int" => Type::Primitive(PrimitiveType::Int),
                    "short" => Type::Primitive(PrimitiveType::Short),
                    "long" => Type::Primitive(PrimitiveType::Long),
                    "float" => Type::Primitive(PrimitiveType::Float),
                    "double" => Type::Primitive(PrimitiveType::Double),
                    _ => Type::Auto,
                }
            }
            "type_identifier" | "qualified_identifier" => {
                let type_name = &source[node.byte_range()];
                let interned_name = self.interner.write().intern(type_name);
                Type::Named(TypePath {
                    segments: vec![interned_name],
                    is_global: false,
                })
            }
            "pointer_declarator" => {
                // For pointer types, recursively get the base type
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() != "*" {
                        let base_type = self.parse_type_from_node(child, source);
                        return Type::Pointer(Box::new(base_type), PointerKind::Raw);
                    }
                }
                Type::Auto
            }
            "reference_declarator" => {
                // For reference types
                let mut cursor = node.walk();
                let mut is_rvalue = false;
                for child in node.children(&mut cursor) {
                    if child.kind() == "&&" {
                        is_rvalue = true;
                    } else if child.kind() != "&" {
                        let base_type = self.parse_type_from_node(child, source);
                        let ref_kind = if is_rvalue { ReferenceKind::RValue } else { ReferenceKind::LValue };
                        return Type::Reference(Box::new(base_type), ref_kind);
                    }
                }
                Type::Auto
            }
            _ => Type::Auto,
        }
    }

    /// Find the last identifier in a qualified name
    fn find_last_identifier(&mut self, node: Node, source: &str) -> Option<InternedString> {
        let mut cursor = node.walk();
        let mut last_id = None;

        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "type_identifier" {
                let text = &source[child.byte_range()];
                last_id = Some(self.interner.write().intern(text));
            }
        }

        last_id
    }

    /// Parse a class or struct
    fn parse_class(&mut self, node: Node, source: &str, file_id: FileId, is_struct: bool) -> Option<ClassDecl> {
        let span = node.to_span(file_id);

        // Extract documentation comment
        let doc_comment = self.extract_doc_comment(node, source);

        let mut cursor = node.walk();
        let mut name = None;
        let mut bases = Vec::new();
        let mut members = Vec::new();

        // Debug: log all children to understand tree structure
        #[cfg(test)]
        {
            let mut debug_cursor = node.walk();
            eprintln!("\nclass_specifier node (bytes {}..{}):", node.start_byte(), node.end_byte());
            eprintln!("Full text: '{}'", &source[node.byte_range()]);
            eprintln!("Children:");
            for child in node.children(&mut debug_cursor) {
                let text = &source[child.byte_range()];
                let preview = if text.len() > 80 { &text[..80] } else { text };
                eprintln!("  kind='{}' bytes={}..{} text='{}'", child.kind(), child.start_byte(), child.end_byte(), preview);
            }
        }

        for child in node.children(&mut cursor) {
            match child.kind() {
                "type_identifier" => {
                    let text = &source[child.byte_range()];
                    // Keep updating name to use the LAST type_identifier
                    // This handles export macros like "class EXPORT_API ClassName"
                    // where the first identifier is the macro and the last is the actual class name
                    log::debug!("Found type_identifier in class: '{}'", text);
                    name = Some(self.interner.write().intern(text));
                }
                "base_class_clause" => {
                    bases = self.parse_base_classes(child, source, file_id);
                }
                "field_declaration_list" => {
                    // Parse class body
                    members = self.parse_class_members(child, source, file_id, is_struct);
                }
                _ => {}
            }
        }

        let name = name?;

        Some(ClassDecl {
            name,
            span,
            access: if is_struct { AccessSpecifier::Public } else { AccessSpecifier::Private },
            bases,
            members,
            is_struct,
            template_params: None,
            doc_comment,
        })
    }

    /// Parse a class that was misidentified as a function_definition due to export macros
    /// Example: "class EXPORT_API ClassName : Base { ... }" is parsed as function_definition with:
    /// - class_specifier: "class EXPORT_API"
    /// - ERROR or identifier: contains "ClassName" (the real class name!)
    /// - compound_statement: "{ ... }"
    fn parse_class_with_export_macro_from_text(
        &mut self,
        class_spec_node: Node,
        class_name: &str,
        body_node: Option<Node>,
        source: &str,
        file_id: FileId,
        is_struct: bool,
    ) -> Option<ClassDecl> {
        // Use the provided class name
        let name = self.interner.write().intern(class_name);

        // Extract doc comment from the class_specifier node
        let doc_comment = self.extract_doc_comment(class_spec_node, source);

        // Parse base classes from the class_specifier or from the function_definition parent
        let mut bases = Vec::new();
        let mut cursor = class_spec_node.walk();
        for child in class_spec_node.children(&mut cursor) {
            if child.kind() == "base_class_clause" {
                bases = self.parse_base_classes(child, source, file_id);
            }
        }

        // Parse class members from the compound_statement (body)
        let members = if let Some(body) = body_node {
            self.parse_class_members(body, source, file_id, is_struct)
        } else {
            Vec::new()
        };

        // Calculate span: from class_specifier start to body end (or class_spec end if no body)
        let start = class_spec_node.start_byte();
        let end = body_node
            .map(|b| b.end_byte())
            .unwrap_or_else(|| class_spec_node.end_byte() + class_name.len());
        let span = Span::new(file_id, start as u32, end as u32);

        Some(ClassDecl {
            name,
            span,
            access: if is_struct { AccessSpecifier::Public } else { AccessSpecifier::Private },
            bases,
            members,
            is_struct,
            template_params: None,
            doc_comment,
        })
    }

    /// Parse base classes
    fn parse_base_classes(&mut self, node: Node, source: &str, file_id: FileId) -> Vec<crate::ast::BaseClass> {
        let mut bases = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "base_class_specifier" || child.kind() == "type_identifier" || child.kind() == "qualified_identifier" {
                let mut access = AccessSpecifier::Public;
                let mut is_virtual = false;
                let mut type_path = None;

                let mut spec_cursor = child.walk();
                for spec_child in child.children(&mut spec_cursor) {
                    match spec_child.kind() {
                        "access_specifier" => {
                            let access_str = &source[spec_child.byte_range()];
                            access = match access_str {
                                "public" => AccessSpecifier::Public,
                                "protected" => AccessSpecifier::Protected,
                                "private" => AccessSpecifier::Private,
                                _ => AccessSpecifier::Public,
                            };
                        }
                        "virtual" => {
                            is_virtual = true;
                        }
                        "type_identifier" | "qualified_identifier" => {
                            let type_name = &source[spec_child.byte_range()];
                            let interned = self.interner.write().intern(type_name);
                            type_path = Some(crate::ast::TypePath {
                                segments: vec![interned],
                                is_global: false,
                            });
                        }
                        _ => {}
                    }
                }

                if let Some(tp) = type_path {
                    bases.push(crate::ast::BaseClass {
                        type_path: tp,
                        access,
                        is_virtual,
                    });
                }
            }
        }

        bases
    }

    /// Parse class members
    fn parse_class_members(&mut self, node: Node, source: &str, file_id: FileId, is_struct: bool) -> Vec<crate::ast::ClassMember> {
        let mut members = Vec::new();
        let mut cursor = node.walk();
        let mut current_access = if is_struct { AccessSpecifier::Public } else { AccessSpecifier::Private };

        for child in node.children(&mut cursor) {
            match child.kind() {
                "access_specifier" => {
                    let access_str = &source[child.byte_range()];
                    current_access = match access_str.trim_end_matches(':') {
                        "public" => AccessSpecifier::Public,
                        "protected" => AccessSpecifier::Protected,
                        "private" => AccessSpecifier::Private,
                        _ => current_access,
                    };
                }
                "function_definition" => {
                    if let Some(func) = self.parse_function(child, source, file_id) {
                        members.push(crate::ast::ClassMember::Method(func));
                    }
                }
                "field_declaration" => {
                    if let Some(field) = self.parse_field(child, source, file_id, current_access) {
                        members.push(crate::ast::ClassMember::Field(field));
                    }
                }
                _ => {}
            }
        }

        members
    }

    /// Parse a field declaration
    fn parse_field(&mut self, node: Node, source: &str, file_id: FileId, access: AccessSpecifier) -> Option<crate::ast::FieldDecl> {
        let span = node.to_span(file_id);
        let mut cursor = node.walk();
        let mut field_type = Type::Auto;
        let mut name = None;
        let mut is_static = false;
        let mut is_mutable = false;

        for child in node.children(&mut cursor) {
            match child.kind() {
                "storage_class_specifier" => {
                    let specifier = &source[child.byte_range()];
                    if specifier == "static" {
                        is_static = true;
                    } else if specifier == "mutable" {
                        is_mutable = true;
                    }
                }
                "type_qualifier" | "primitive_type" | "type_identifier" | "qualified_identifier" => {
                    field_type = self.parse_type_from_node(child, source);
                }
                "field_declarator" | "declarator" => {
                    if let Some(id) = self.find_identifier(child, source) {
                        name = Some(id);
                    }
                }
                _ => {}
            }
        }

        name?;

        Some(crate::ast::FieldDecl {
            name: name.unwrap(),
            ty: field_type,
            span,
            access,
            is_static,
            is_mutable,
            initializer: None,
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
                name = Some(self.interner.write().intern(text));
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
                return Some(self.interner.write().intern(text));
            }

            // Recursively search in child nodes
            if let Some(id) = self.find_identifier(child, source) {
                return Some(id);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::RwLock;
    use std::sync::Arc;

    #[test]
    fn test_parse_simple_function() {
        let interner = Arc::new(RwLock::new(Interner::new()));
        let mut parser = CppParser::new(interner.clone()).unwrap();
        let source = "int main() { return 0; }";
        let file_id = FileId::new(1);

        let result = parser.parse(source, file_id);
        assert!(result.is_ok());

        let unit = result.unwrap();
        assert_eq!(unit.declarations.len(), 1);

        match &unit.declarations[0] {
            Declaration::Function(func) => {
                let name = interner.read().resolve(func.name);
                assert_eq!(name, "main");
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn test_parse_class() {
        let interner = Arc::new(RwLock::new(Interner::new()));
        let mut parser = CppParser::new(interner.clone()).unwrap();
        let source = "class MyClass {};";
        let file_id = FileId::new(1);

        let result = parser.parse(source, file_id);
        assert!(result.is_ok());

        let unit = result.unwrap();
        assert_eq!(unit.declarations.len(), 1);

        match &unit.declarations[0] {
            Declaration::Class(class) => {
                let name = interner.read().resolve(class.name);
                assert_eq!(name, "MyClass");
                assert!(!class.is_struct);
            }
            _ => panic!("Expected class declaration"),
        }
    }

    #[test]
    fn test_parse_namespace() {
        let interner = Arc::new(RwLock::new(Interner::new()));
        let mut parser = CppParser::new(interner.clone()).unwrap();
        let source = "namespace MyNamespace { void foo(); }";
        let file_id = FileId::new(1);

        let result = parser.parse(source, file_id);
        assert!(result.is_ok());

        let unit = result.unwrap();
        assert_eq!(unit.declarations.len(), 1);

        match &unit.declarations[0] {
            Declaration::Namespace(ns) => {
                assert!(ns.name.is_some());
                let name = interner.read().resolve(ns.name.unwrap());
                assert_eq!(name, "MyNamespace");
            }
            _ => panic!("Expected namespace declaration"),
        }
    }

    #[test]
    fn test_parse_with_errors() {
        let interner = Arc::new(RwLock::new(Interner::new()));
        let mut parser = CppParser::new(interner).unwrap();
        let source = "int main() { return 0 }"; // Missing semicolon
        let file_id = FileId::new(1);

        let result = parser.parse(source, file_id);
        assert!(result.is_ok());

        let unit = result.unwrap();
        assert!(unit.errors.len() > 0, "Should have parse errors");
    }

    #[test]
    fn test_parse_class_with_export_macro() {
        let interner = Arc::new(RwLock::new(Interner::new()));
        let mut parser = CppParser::new(interner.clone()).unwrap();

        // Simulate UE5 class with export macro like "class API_EXPORT MyClass"
        let source = "class MY_API MyClass { public: void foo(); };";
        let file_id = FileId::new(1);

        // First, let's see what tree-sitter creates
        let tree = parser.parser.parse(source, None).unwrap();
        let root = tree.root_node();

        eprintln!("\n=== FULL PARSE TREE ===");
        eprintln!("Source: '{}'", source);
        eprintln!("Root node: kind='{}' bytes={}..{}", root.kind(), root.start_byte(), root.end_byte());

        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            eprintln!("  Top-level: kind='{}' bytes={}..{} text='{}'",
                child.kind(), child.start_byte(), child.end_byte(), &source[child.byte_range()]);

            let mut child_cursor = child.walk();
            for grandchild in child.children(&mut child_cursor) {
                eprintln!("    Child: kind='{}' bytes={}..{} text='{}'",
                    grandchild.kind(), grandchild.start_byte(), grandchild.end_byte(), &source[grandchild.byte_range()]);
            }
        }
        eprintln!("======================\n");

        let result = parser.parse(source, file_id);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let unit = result.unwrap();

        eprintln!("Parsed {} declarations", unit.declarations.len());
        for (i, decl) in unit.declarations.iter().enumerate() {
            match decl {
                Declaration::Class(c) => eprintln!("  [{}] Class: {}", i, interner.read().resolve(c.name)),
                Declaration::Function(f) => eprintln!("  [{}] Function: {}", i, interner.read().resolve(f.name)),
                _ => eprintln!("  [{}] Other", i),
            }
        }

        assert_eq!(unit.declarations.len(), 1, "Expected 1 declaration, got {}", unit.declarations.len());

        match &unit.declarations[0] {
            Declaration::Class(class) => {
                let name = interner.read().resolve(class.name);
                assert_eq!(name, "MyClass", "Expected class name 'MyClass', got '{}'", name);
            }
            other => panic!("Expected class declaration, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_ue5_class_with_uclass_macro() {
        let interner = Arc::new(RwLock::new(Interner::new()));
        let mut parser = CppParser::new(interner.clone()).unwrap();

        // Simplified version of MissionManagerSystem.h
        let source = r#"
class UObjectPoolManager;
class UDestructionManager;
class UMissionData;

UCLASS(ClassGroup=(OpenWorldFramework), Category="OpenWorldFramework")
class OWF_MISSIONSYSTEM_API UMissionManagerSystem : public UGameInstanceSubsystem
{
    GENERATED_BODY()

public:
    virtual void Initialize(FSubsystemCollectionBase& Collection) override;
    void Initialise();
};
"#;
        let file_id = FileId::new(1);

        // Debug: see what tree-sitter creates for the main class
        let tree = parser.parser.parse(source, None).unwrap();
        let root = tree.root_node();

        eprintln!("\n=== CHECKING UMissionManagerSystem DECLARATION ===");
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "function_definition" && source[child.byte_range()].contains("UMissionManagerSystem") {
                eprintln!("Found function_definition containing UMissionManagerSystem:");
                eprintln!("  bytes={}..{}", child.start_byte(), child.end_byte());
                eprintln!("  text='{}'", &source[child.byte_range()]);

                let mut child_cursor = child.walk();
                for grandchild in child.children(&mut child_cursor) {
                    let text = &source[grandchild.byte_range()];
                    let preview = if text.len() > 60 { &text[..60] } else { text };
                    eprintln!("    kind='{}' bytes={}..{} text='{}'",
                        grandchild.kind(), grandchild.start_byte(), grandchild.end_byte(), preview);
                }
            }
        }
        eprintln!("=====================================================\n");

        let result = parser.parse(source, file_id);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let unit = result.unwrap();

        println!("Parsed {} declarations:", unit.declarations.len());
        for (i, decl) in unit.declarations.iter().enumerate() {
            match decl {
                Declaration::Class(c) => {
                    let name = interner.read().resolve(c.name);
                    println!("  [{}] Class: {}", i, name);
                }
                Declaration::Function(f) => {
                    let name = interner.read().resolve(f.name);
                    println!("  [{}] Function: {}", i, name);
                }
                _ => println!("  [{}] Other: {:?}", i, decl),
            }
        }

        // Should have at least 4 class declarations:
        // 1. UObjectPoolManager (forward decl)
        // 2. UDestructionManager (forward decl)
        // 3. UMissionData (forward decl)
        // 4. UMissionManagerSystem (full class)
        assert!(unit.declarations.len() >= 4, "Expected at least 4 declarations, got {}", unit.declarations.len());

        // Find the UMissionManagerSystem class
        let main_class = unit.declarations.iter().find_map(|decl| {
            if let Declaration::Class(class) = decl {
                let name = interner.read().resolve(class.name);
                if name == "UMissionManagerSystem" {
                    Some(class)
                } else {
                    None
                }
            } else {
                None
            }
        });

        assert!(main_class.is_some(), "Could not find UMissionManagerSystem class");
        let main_class = main_class.unwrap();

        let class_name = interner.read().resolve(main_class.name);
        assert_eq!(class_name, "UMissionManagerSystem",
            "Expected class name 'UMissionManagerSystem', got '{}'", class_name);
    }
}
