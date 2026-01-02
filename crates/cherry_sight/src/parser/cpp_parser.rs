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
        let mut includes = Vec::new();
        let mut cursor = root.walk();
        let mut node_kinds = std::collections::HashMap::new();

        for child in root.children(&mut cursor) {
            let kind = child.kind();
            *node_kinds.entry(kind).or_insert(0) += 1;

            eprintln!("Top-level node: kind='{}' at {}..{}", kind, child.start_byte(), child.end_byte());

            // Parse include directives
            if kind == "preproc_include" {
                if let Some(include) = self.parse_include(child, source, file_id) {
                    includes.push(include);
                }
            } else if let Some(decl) = self.parse_declaration(child, source, file_id) {
                declarations.push(decl);
            } else {
                log::debug!("Skipped node kind: '{}' at byte range {}..{}", kind, child.start_byte(), child.end_byte());
            }
        }

        log::info!("Node kinds found: {:?}", node_kinds);
        log::info!("Extracted {} includes and {} declarations from {} total nodes",
                   includes.len(), declarations.len(), node_kinds.values().sum::<i32>());

        Ok(TranslationUnit {
            file_id,
            includes,
            declarations,
            errors,
        })
    }

    /// Parse an include directive
    fn parse_include(&mut self, node: Node, source: &str, file_id: FileId) -> Option<crate::ast::IncludeDirective> {
        // preproc_include has children: "#include" and either string_literal or system_lib_string
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "string_literal" => {
                    // #include "header.h"
                    let text = &source[child.byte_range()];
                    // Remove quotes
                    let path = text.trim_matches('"').to_string();
                    return Some(crate::ast::IncludeDirective {
                        path,
                        is_system: false,
                        span: Span::new(file_id, node.start_byte() as u32, node.end_byte() as u32),
                    });
                }
                "system_lib_string" => {
                    // #include <header>
                    let text = &source[child.byte_range()];
                    // Remove angle brackets
                    let path = text.trim_start_matches('<').trim_end_matches('>').to_string();
                    return Some(crate::ast::IncludeDirective {
                        path,
                        is_system: true,
                        span: Span::new(file_id, node.start_byte() as u32, node.end_byte() as u32),
                    });
                }
                _ => {}
            }
        }
        None
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

                eprintln!("function_definition node at {}..{} - checking if it's a class:", node.start_byte(), node.end_byte());
                for child in node.children(&mut cursor) {
                    let text = &source[child.byte_range()];
                    let preview = if text.len() > 40 { &text[..40] } else { text };
                    eprintln!("  child: kind='{}' text='{}'", child.kind(), preview);

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
                    eprintln!("  -> Found class_specifier, parsing as class");
                    log::info!("Found class_specifier inside function_definition (export macro confusion)");

                    // Collect all children to handle complex cases (multiple base classes, etc.)
                    let mut all_error_nodes = Vec::new();
                    let mut all_identifiers = Vec::new();
                    let mut init_decl = None;

                    let mut child_cursor = node.walk();
                    for child in node.children(&mut child_cursor) {
                        match child.kind() {
                            "ERROR" => all_error_nodes.push(child),
                            "identifier" => all_identifiers.push(child),
                            "init_declarator" => init_decl = Some(child),
                            _ => {}
                        }
                    }

                    // Determine class name and base classes
                    let (class_name, bases) = if !all_error_nodes.is_empty() {
                        let first_error = &source[all_error_nodes[0].byte_range()];
                        eprintln!("  -> First ERROR: '{}'", first_error);

                        if first_error.trim().starts_with(':') {
                            // Pattern 1: ERROR = ": public Base[, Base2...]", identifier = ClassName
                            // Reconstruct base class list from all siblings
                            let base_start = all_error_nodes[0].start_byte();
                            let base_end = if let Some(ref init) = init_decl {
                                init.start_byte()
                            } else if let Some(last) = all_identifiers.last() {
                                last.end_byte()
                            } else {
                                all_error_nodes.last().unwrap().end_byte()
                            };
                            let (bases, base_text_debug) = if base_end as usize > base_start as usize {
                                let base_text = &source[base_start as usize..base_end as usize];
                                (self.parse_bases_from_text(base_text, file_id, base_start as u32), base_text.to_string())
                            } else {
                                (Vec::new(), String::new())
                            };

                            let name = all_identifiers.first()
                                .map(|n| &source[n.byte_range()])
                                .unwrap_or("UnknownClass");

                            eprintln!("  -> Pattern 1: class='{}', {} bases from text: '{}'", name, bases.len(), base_text_debug.trim());
                            (name.to_string(), bases)
                        } else {
                            // Pattern 2: ERROR = "ClassName : public[...]", followed by base classes
                            let class_name = first_error.split(':').next()
                                .map(|s| s.trim())
                                .unwrap_or("UnknownClass");

                            // Reconstruct base class text from first ERROR onwards
                            let base_start = if let Some(colon_pos) = first_error.find(':') {
                                all_error_nodes[0].start_byte() as usize + colon_pos
                            } else {
                                all_error_nodes[0].end_byte() as usize
                            };

                            let base_end = if let Some(ref init) = init_decl {
                                init.end_byte() as usize
                            } else if let Some(last) = all_identifiers.last() {
                                last.end_byte() as usize
                            } else if all_error_nodes.len() > 1 {
                                all_error_nodes.last().unwrap().end_byte() as usize
                            } else {
                                all_error_nodes[0].end_byte() as usize
                            };

                            if base_end > base_start {
                                let base_text = &source[base_start..base_end];
                                let bases = self.parse_bases_from_text(base_text, file_id, base_start as u32);
                                eprintln!("  -> Pattern 2: class='{}', {} bases from text: '{}'", class_name, bases.len(), base_text.trim());
                                (class_name.to_string(), bases)
                            } else {
                                eprintln!("  -> Pattern 2: class='{}', no bases", class_name);
                                (class_name.to_string(), Vec::new())
                            }
                        }
                    } else if !all_identifiers.is_empty() {
                        let name = &source[all_identifiers[0].byte_range()];
                        eprintln!("  -> No ERROR, using first identifier: '{}'", name);
                        (name.to_string(), Vec::new())
                    } else {
                        eprintln!("  -> No ERROR or identifiers!");
                        ("UnknownClass".to_string(), Vec::new())
                    };

                    // Parse with base classes
                    return self.parse_class_with_export_macro_and_bases(
                        class_node, &class_name, bases, body, source, file_id, false
                    ).map(Declaration::Class);
                }

                if let Some(struct_node) = struct_spec {
                    eprintln!("  -> Found struct_specifier, parsing as struct");
                    log::info!("Found struct_specifier inside function_definition (export macro confusion)");

                    // Determine struct name and base classes from ERROR node and identifier
                    let (struct_name, bases) = if let Some(err_node) = error_node {
                        let err_text = &source[err_node.byte_range()];
                        eprintln!("  -> ERROR node contains: '{}'", err_text);
                        log::info!("Found ERROR node: '{}'", err_text);

                        if err_text.trim().starts_with(':') {
                            // Pattern 1: ERROR = ": public Base", identifier = StructName
                            let bases = self.parse_bases_from_text(err_text, file_id, err_node.start_byte() as u32);
                            let name = if let Some(name_node) = class_name_identifier {
                                &source[name_node.byte_range()]
                            } else {
                                "UnknownStruct"
                            };
                            eprintln!("  -> Pattern 1: struct='{}', bases from ERROR", name);
                            (name.to_string(), bases)
                        } else {
                            // Pattern 2: ERROR = "StructName : public", identifier = BaseName
                            let struct_name = err_text.split(':').next()
                                .map(|s| s.trim())
                                .unwrap_or("UnknownStruct");

                            let mut bases = Vec::new();
                            if let Some(colon_pos) = err_text.find(':') {
                                let after_colon = &err_text[colon_pos..];
                                if let Some(name_node) = class_name_identifier {
                                    let base_name = &source[name_node.byte_range()];
                                    let full_base = format!("{} {}", after_colon.trim(), base_name);
                                    eprintln!("  -> Pattern 2: Parsing base from: '{}'", full_base);
                                    // Note: full_base is synthetic, so we use name_node's position for approximate spans
                                    bases = self.parse_bases_from_text(&full_base, file_id, name_node.start_byte() as u32);
                                }
                            }
                            eprintln!("  -> Pattern 2: struct='{}', {} bases", struct_name, bases.len());
                            (struct_name.to_string(), bases)
                        }
                    } else if let Some(name_node) = class_name_identifier {
                        let name = &source[name_node.byte_range()];
                        eprintln!("  -> No ERROR node, using identifier: '{}'", name);
                        (name.to_string(), Vec::new())
                    } else {
                        eprintln!("  -> No ERROR or identifier!");
                        ("UnknownStruct".to_string(), Vec::new())
                    };

                    // Parse with base classes
                    return self.parse_class_with_export_macro_and_bases(
                        struct_node, &struct_name, bases, body, source, file_id, true
                    ).map(Declaration::Struct);
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
                // Check if this is a class/struct declaration with export macro
                // Pattern: declaration { class_specifier("class EXPORT"), init_declarator("ClassName : Base { }") }
                // Or with multiple bases: class_specifier, ERROR nodes, identifiers, init_declarator
                let mut cursor = node.walk();
                let mut class_spec = None;
                let mut struct_spec = None;
                let mut init_declarator = None;
                let mut has_error_or_ident = false;

                eprintln!("parse_declaration: 'declaration' node at {}..{}", node.start_byte(), node.end_byte());
                for child in node.children(&mut cursor) {
                    eprintln!("  child: kind='{}' at {}..{}", child.kind(), child.start_byte(), child.end_byte());
                    match child.kind() {
                        "class_specifier" => class_spec = Some(child),
                        "struct_specifier" => struct_spec = Some(child),
                        "init_declarator" => init_declarator = Some(child),
                        "ERROR" | "identifier" => has_error_or_ident = true,
                        _ => {}
                    }
                }

                // If we have class_specifier + init_declarator (simple case, single base or no bases)
                if let (Some(class_node), Some(init_node)) = (class_spec, init_declarator) {
                    if !has_error_or_ident {
                        eprintln!("  -> Taking simple init_declarator path for class");
                        log::info!("Found class with export macro (declaration + init_declarator pattern)");
                        return self.parse_class_from_init_declarator(init_node, source, file_id, false)
                            .map(Declaration::Class);
                    } else {
                        eprintln!("  -> Has ERROR/identifiers, parsing with multiple base class logic");
                        // Multiple base classes - collect all relevant nodes
                        let mut all_error_nodes = Vec::new();
                        let mut all_identifiers = Vec::new();
                        let mut body_node = None;
                        let mut last_base_name_end = None;

                        let mut child_cursor = node.walk();
                        for child in node.children(&mut child_cursor) {
                            match child.kind() {
                                "ERROR" => all_error_nodes.push(child),
                                "identifier" => all_identifiers.push(child),
                                "init_declarator" => {
                                    // Extract identifier and body from init_declarator
                                    let mut init_cursor = child.walk();
                                    for init_child in child.children(&mut init_cursor) {
                                        match init_child.kind() {
                                            "identifier" => {
                                                // This is the last base class name
                                                all_identifiers.push(init_child);
                                                last_base_name_end = Some(init_child.end_byte() as usize);
                                            }
                                            "initializer_list" | "compound_statement" => {
                                                body_node = Some(init_child);
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }

                        // Extract class name and bases using the same logic as function_definition handler
                        let (class_name, bases) = if !all_error_nodes.is_empty() {
                            let first_error = &source[all_error_nodes[0].byte_range()];
                            let class_name = first_error.split(':').next()
                                .map(|s| s.trim())
                                .unwrap_or("UnknownClass");

                            // Reconstruct base class text
                            let base_start = if let Some(colon_pos) = first_error.find(':') {
                                all_error_nodes[0].start_byte() as usize + colon_pos
                            } else {
                                all_error_nodes[0].end_byte() as usize
                            };

                            // Use the last base name end position if we have it, otherwise use init_declarator start
                            let base_end = if let Some(end) = last_base_name_end {
                                end
                            } else if let Some(ref init) = init_declarator {
                                init.start_byte() as usize  // Just before the init_declarator
                            } else if let Some(last) = all_identifiers.last() {
                                last.end_byte() as usize
                            } else {
                                all_error_nodes[0].end_byte() as usize
                            };

                            let bases = if base_end > base_start {
                                let base_text = &source[base_start..base_end];
                                self.parse_bases_from_text(base_text, file_id, base_start as u32)
                            } else {
                                Vec::new()
                            };

                            (class_name.to_string(), bases)
                        } else {
                            ("UnknownClass".to_string(), Vec::new())
                        };

                        return self.parse_class_with_export_macro_and_bases(
                            class_node, &class_name, bases, body_node, source, file_id, false
                        ).map(Declaration::Class);
                    }
                }

                // If we have struct_specifier + init_declarator
                if let (Some(_struct_node), Some(init_node)) = (struct_spec, init_declarator) {
                    if !has_error_or_ident {
                        log::info!("Found struct with export macro (declaration + init_declarator pattern)");
                        return self.parse_class_from_init_declarator(init_node, source, file_id, true)
                            .map(Declaration::Struct);
                    }
                }

                // If only class_specifier without init_declarator (old pattern)
                if let Some(class_node) = class_spec {
                    log::info!("Found class_specifier inside declaration node (likely has export macro)");
                    return self.parse_class(class_node, source, file_id, false).map(Declaration::Class);
                }

                if let Some(struct_node) = struct_spec {
                    log::info!("Found struct_specifier inside declaration node (likely has export macro)");
                    return self.parse_class(struct_node, source, file_id, true).map(Declaration::Struct);
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
                // For pointer return types, tree-sitter wraps function_declarator in pointer_declarator
                "pointer_declarator" | "reference_declarator" if name.is_none() => {
                    // Look for function_declarator inside
                    let mut decl_cursor = child.walk();
                    for decl_child in child.children(&mut decl_cursor) {
                        if decl_child.kind() == "function_declarator" {
                            if let Some((func_name, func_name_span, params)) = self.parse_function_declarator(decl_child, source, file_id) {
                                name = Some(func_name);
                                name_span = func_name_span;
                                parameters = params;

                                // Update return type to be a pointer/reference
                                if child.kind() == "pointer_declarator" {
                                    return_type = Type::Pointer(Box::new(return_type), crate::ast::PointerKind::Raw);
                                } else {
                                    return_type = Type::Reference(Box::new(return_type), crate::ast::ReferenceKind::LValue);
                                }
                                break;
                            }
                        }
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
    /// Parse class with export macro and pre-parsed base classes
    fn parse_class_with_export_macro_and_bases(
        &mut self,
        class_spec_node: Node,
        class_name: &str,
        bases: Vec<crate::ast::BaseClass>,
        body_node: Option<Node>,
        source: &str,
        file_id: FileId,
        is_struct: bool,
    ) -> Option<ClassDecl> {
        // Use the provided class name
        let name = self.interner.write().intern(class_name);

        // Extract doc comment from the class_specifier node
        let doc_comment = self.extract_doc_comment(class_spec_node, source);

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

        eprintln!("  -> Created class '{}' with {} bases and {} members", class_name, bases.len(), members.len());

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

    /// Parse class from init_declarator node (for export macro pattern)
    /// init_declarator contains: "ClassName : public Base { body }"
    fn parse_class_from_init_declarator(
        &mut self,
        node: Node,
        source: &str,
        file_id: FileId,
        is_struct: bool,
    ) -> Option<ClassDecl> {
        // Extract text and parse manually
        let text = &source[node.byte_range()];

        // Find class name (before ':' or '{')
        let name_end = text.find(':').or_else(|| text.find('{')).unwrap_or(text.len());
        let class_name = text[..name_end].trim();
        let name = self.interner.write().intern(class_name);

        // Find base classes and body by parsing child nodes
        let mut bases = Vec::new();
        let mut body_node = None;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            #[cfg(test)]
            {
                eprintln!("init_declarator child: kind='{}' text='{}'", child.kind(), &source[child.byte_range()]);
            }

            match child.kind() {
                "base_class_clause" => {
                    bases = self.parse_base_classes(child, source, file_id);
                }
                "ERROR" => {
                    // ERROR node may contain base class info (": public Base")
                    let error_text = &source[child.byte_range()];
                    if error_text.trim().starts_with(':') {
                        // Parse base classes from the ERROR text
                        bases = self.parse_bases_from_text(error_text, file_id, child.start_byte() as u32);
                    }
                }
                "field_declaration_list" | "compound_statement" | "initializer_list" => {
                    body_node = Some(child);
                }
                _ => {}
            }
        }

        // Parse members from body
        let members = if let Some(body) = body_node {
            self.parse_class_members(body, source, file_id, is_struct)
        } else {
            Vec::new()
        };

        // Calculate span
        let span = node.to_span(file_id);

        Some(ClassDecl {
            name,
            span,
            access: if is_struct { AccessSpecifier::Public } else { AccessSpecifier::Private },
            bases,
            members,
            is_struct,
            template_params: None,
            doc_comment: None,
        })
    }

    /// Parse base classes from text (for ERROR nodes)
    fn parse_bases_from_text(&mut self, text: &str, file_id: FileId, text_offset: u32) -> Vec<crate::ast::BaseClass> {
        let mut bases = Vec::new();

        // Remove leading ':' and split by ','
        let text = text.trim();
        let trimmed_start = text.len() - text.trim_start_matches(':').trim().len();
        let text = text.trim_start_matches(':').trim();

        let mut current_offset = text_offset + trimmed_start as u32;

        for part in text.split(',') {
            let part = part.trim();
            if part.is_empty() {
                // Skip empty parts but advance offset past the comma
                current_offset += 1;
                continue;
            }

            // Find where this part starts in the original text
            let part_offset = current_offset;

            // Parse "public Base" or "protected Base" or just "Base"
            let mut access = AccessSpecifier::Public;
            let mut base_name = part;
            let mut base_name_offset = part_offset;

            if part.starts_with("public ") {
                base_name = &part[7..];
                base_name_offset = part_offset + 7;
            } else if part.starts_with("protected ") {
                access = AccessSpecifier::Protected;
                base_name = &part[10..];
                base_name_offset = part_offset + 10;
            } else if part.starts_with("private ") {
                access = AccessSpecifier::Private;
                base_name = &part[8..];
                base_name_offset = part_offset + 8;
            }

            base_name = base_name.trim();
            // Adjust offset if there was whitespace trimmed
            let trim_offset = (part.len() - part.trim_start().len()) as u32;
            base_name_offset += trim_offset;

            if !base_name.is_empty() {
                let interned = self.interner.write().intern(base_name);
                let span = Span::new(
                    file_id,
                    base_name_offset,
                    base_name_offset + base_name.len() as u32,
                );
                bases.push(crate::ast::BaseClass {
                    type_path: crate::ast::TypePath {
                        segments: vec![interned],
                        is_global: false,
                    },
                    access,
                    is_virtual: false,
                    span,
                });
            }

            // Advance offset past this part and the comma
            current_offset += part.len() as u32 + 1;
        }

        bases
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
                let mut type_span = None;

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
                            // Capture the span of the type identifier
                            type_span = Some(Span::new(
                                file_id,
                                spec_child.start_byte() as u32,
                                spec_child.end_byte() as u32,
                            ));
                        }
                        _ => {}
                    }
                }

                if let (Some(tp), Some(span)) = (type_path, type_span) {
                    bases.push(crate::ast::BaseClass {
                        type_path: tp,
                        access,
                        is_virtual,
                        span,
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

        #[cfg(test)]
        {
            eprintln!("parse_class_members: node kind='{}' bytes={}..{}", node.kind(), node.start_byte(), node.end_byte());
            let mut debug_cursor = node.walk();
            for child in node.children(&mut debug_cursor) {
                let text = &source[child.byte_range()];
                let preview = if text.len() > 60 { &text[..60] } else { text };
                eprintln!("  child: kind='{}' text='{}'", child.kind(), preview);
            }
        }

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
                // Tree-sitter sometimes creates labeled_statement for access specifiers like "public:"
                "labeled_statement" => {
                    let text = &source[child.byte_range()];
                    // Check if this is an access specifier (public:, protected:, private:)
                    if text.trim_start().starts_with("public:") {
                        current_access = AccessSpecifier::Public;
                    } else if text.trim_start().starts_with("protected:") {
                        current_access = AccessSpecifier::Protected;
                    } else if text.trim_start().starts_with("private:") {
                        current_access = AccessSpecifier::Private;
                    }

                    #[cfg(test)]
                    {
                        eprintln!("  labeled_statement children:");
                        let mut debug_cursor = child.walk();
                        for lc in child.children(&mut debug_cursor) {
                            let ltext = &source[lc.byte_range()];
                            let lpreview = if ltext.len() > 40 { &ltext[..40] } else { ltext };
                            eprintln!("    kind='{}' text='{}'", lc.kind(), lpreview);
                        }
                    }

                    // Parse members inside the labeled statement
                    let mut labeled_cursor = child.walk();
                    for labeled_child in child.children(&mut labeled_cursor) {
                        match labeled_child.kind() {
                            "function_definition" | "declaration" => {
                                // Try to parse as function first
                                if let Some(func) = self.parse_function(labeled_child, source, file_id) {
                                    members.push(crate::ast::ClassMember::Method(func));
                                } else if let Some(field) = self.parse_field(labeled_child, source, file_id, current_access) {
                                    members.push(crate::ast::ClassMember::Field(field));
                                }
                            }
                            "field_declaration" => {
                                if let Some(field) = self.parse_field(labeled_child, source, file_id, current_access) {
                                    members.push(crate::ast::ClassMember::Field(field));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                "function_definition" => {
                    if let Some(func) = self.parse_function(child, source, file_id) {
                        members.push(crate::ast::ClassMember::Method(func));
                    }
                }
                // Tree-sitter may parse method declarations as "declaration" nodes
                "declaration" => {
                    #[cfg(test)]
                    {
                        let text = &source[child.byte_range()];
                        let preview = if text.len() > 60 { &text[..60] } else { text };
                        eprintln!("  Processing declaration: '{}'", preview);
                    }

                    // Try to parse as function first
                    if let Some(func) = self.parse_function(child, source, file_id) {
                        let func_name = self.interner.read().resolve(func.name);

                        // Skip UE5 macros that look like function calls (UPROPERTY, UFUNCTION, etc.)
                        if func_name.starts_with("UPROPERTY") || func_name.starts_with("UFUNCTION") ||
                           func_name.starts_with("UCLASS") || func_name.starts_with("USTRUCT") ||
                           func_name.starts_with("UENUM") || func_name == "GENERATED_BODY" {
                            #[cfg(test)]
                            {
                                eprintln!("    -> Skipping UE5 macro: {}", func_name);
                            }
                            // Skip this - it's a macro, not a real method
                        } else {
                            #[cfg(test)]
                            {
                                eprintln!("    -> Parsed as method: {}", func_name);
                            }
                            members.push(crate::ast::ClassMember::Method(func));
                        }
                    } else if let Some(field) = self.parse_field(child, source, file_id, current_access) {
                        #[cfg(test)]
                        {
                            let fname = self.interner.read().resolve(field.name);
                            eprintln!("    -> Parsed as field: {}", fname);
                        }
                        members.push(crate::ast::ClassMember::Field(field));
                    } else {
                        #[cfg(test)]
                        {
                            eprintln!("    -> Could not parse as function or field");
                        }
                    }
                }
                "field_declaration" => {
                    #[cfg(test)]
                    {
                        let text = &source[child.byte_range()];
                        let preview = if text.len() > 60 { &text[..60] } else { text };
                        eprintln!("  Processing field_declaration: '{}'", preview);
                    }

                    // Check if this is actually a UE5 macro call (UPROPERTY, etc.)
                    let text = &source[child.byte_range()];
                    if text.trim().starts_with("UPROPERTY") || text.trim().starts_with("UFUNCTION") {
                        #[cfg(test)]
                        {
                            eprintln!("    -> Skipping UE5 macro field_declaration");
                        }
                        // Skip macro calls
                        continue;
                    }

                    if let Some(field) = self.parse_field(child, source, file_id, current_access) {
                        #[cfg(test)]
                        {
                            let fname = self.interner.read().resolve(field.name);
                            eprintln!("    -> Successfully parsed field: {}", fname);
                        }
                        members.push(crate::ast::ClassMember::Field(field));
                    } else {
                        #[cfg(test)]
                        {
                            eprintln!("    -> parse_field returned None");
                        }
                    }
                }
                _ => {}
            }
        }

        members
    }

    /// Parse a field declaration
    fn parse_field(&mut self, node: Node, source: &str, file_id: FileId, access: AccessSpecifier) -> Option<crate::ast::FieldDecl> {
        #[cfg(test)]
        {
            eprintln!("parse_field called on node kind='{}' text='{}'", node.kind(), &source[node.byte_range()]);
        }

        let span = node.to_span(file_id);
        let mut cursor = node.walk();
        let mut field_type = Type::Auto;
        let mut name = None;
        let mut is_static = false;
        let mut is_mutable = false;

        #[cfg(test)]
        {
            eprintln!("  parse_field children:");
            let mut debug_cursor = node.walk();
            for child in node.children(&mut debug_cursor) {
                let text = &source[child.byte_range()];
                let preview = if text.len() > 30 { &text[..30] } else { text };
                eprintln!("    kind='{}' text='{}'", child.kind(), preview);
            }
        }

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
                "type_qualifier" | "primitive_type" | "type_identifier" | "qualified_identifier" | "template_type" => {
                    field_type = self.parse_type_from_node(child, source);
                }
                "field_declarator" | "declarator" => {
                    if let Some(id) = self.find_identifier(child, source) {
                        name = Some(id);
                    }
                }
                // For fields with initializers like "int x = 42", tree-sitter creates init_declarator
                "init_declarator" if name.is_none() => {
                    // Find identifier inside init_declarator
                    if let Some(id) = self.find_identifier(child, source) {
                        name = Some(id);
                        #[cfg(test)]
                        eprintln!("    -> Extracted name from init_declarator: {}", self.interner.read().resolve(id));
                    }
                }
                // For field names, tree-sitter uses "field_identifier"
                "field_identifier" if name.is_none() => {
                    let field_name = &source[child.byte_range()];
                    name = Some(self.interner.write().intern(field_name));
                }
                // For simple field declarations like "int myVar;", tree-sitter gives us a plain identifier
                "identifier" if name.is_none() => {
                    let field_name = &source[child.byte_range()];
                    name = Some(self.interner.write().intern(field_name));
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
    fn test_base_class_with_export_macro() {
        let interner = Arc::new(RwLock::new(Interner::new()));
        let mut parser = CppParser::new(interner.clone()).unwrap();

        let source = "class OWF_API ATask_Sample : public ATask { };";
        let file_id = FileId::new(1);

        let tree = parser.parser.parse(source, None).unwrap();
        let root = tree.root_node();

        eprintln!("\n=== BASE CLASS TREE STRUCTURE ===");
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            eprintln!("Top: kind='{}' text='{}'", child.kind(), &source[child.byte_range()]);
            let mut c2 = child.walk();
            for c2child in child.children(&mut c2) {
                eprintln!("  kind='{}' text='{}'", c2child.kind(), &source[c2child.byte_range()]);
                if c2child.kind() == "ERROR" || c2child.kind() == "base_class_clause" {
                    let mut c3 = c2child.walk();
                    for c3child in c2child.children(&mut c3) {
                        eprintln!("    kind='{}' text='{}'", c3child.kind(), &source[c3child.byte_range()]);
                    }
                }
            }
        }

        let result = parser.parse(source, file_id);
        assert!(result.is_ok());
        let ast = result.unwrap();

        match &ast.declarations[0] {
            crate::ast::Declaration::Class(cls) => {
                let name = interner.read().resolve(cls.name);
                eprintln!("\nParsed class: {}", name);
                eprintln!("Base classes: {}", cls.bases.len());
                for base in &cls.bases {
                    let base_name = base.type_path.segments.iter()
                        .map(|&s| interner.read().resolve(s))
                        .collect::<Vec<_>>()
                        .join("::");
                    eprintln!("  Base: {}", base_name);
                }
                assert_eq!(name, "ATask_Sample");
                assert_eq!(cls.bases.len(), 1, "Should have 1 base class");
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn test_multiple_base_classes() {
        let interner = Arc::new(RwLock::new(Interner::new()));
        let mut parser = CppParser::new(interner.clone()).unwrap();

        // Test class with multiple base classes (including interfaces)
        let source = r#"UCLASS()
class OWF_MISSIONSYSTEM_API ATask : public AActor, public IMissionTask, public IDebugSettingsInterface
{
    GENERATED_BODY()
public:
    void Execute();
};"#;
        let file_id = FileId::new(1);

        // First check tree structure
        let tree = parser.parser.parse(source, None).unwrap();
        let root = tree.root_node();

        eprintln!("\n=== MULTIPLE BASE CLASSES TREE ===");
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            let text = &source[child.byte_range()];
            let preview = if text.len() > 60 { &text[..60] } else { text };
            eprintln!("Top: kind='{}' text='{}'", child.kind(), preview);

            let mut child_cursor = child.walk();
            for grandchild in child.children(&mut child_cursor) {
                let gc_text = &source[grandchild.byte_range()];
                let gc_preview = if gc_text.len() > 60 { &gc_text[..60] } else { gc_text };
                eprintln!("  kind='{}' text='{}'", grandchild.kind(), gc_preview);
            }
        }

        let result = parser.parse(source, file_id);
        assert!(result.is_ok());

        let ast = result.unwrap();
        assert_eq!(ast.declarations.len(), 1);

        match &ast.declarations[0] {
            crate::ast::Declaration::Class(cls) => {
                let name = interner.read().resolve(cls.name);
                assert_eq!(name, "ATask");

                // Should have 3 base classes
                eprintln!("\nClass: {} with {} bases", name, cls.bases.len());
                for base in &cls.bases {
                    let base_name = base.type_path.segments.iter()
                        .map(|&s| interner.read().resolve(s))
                        .collect::<Vec<_>>()
                        .join("::");
                    eprintln!("  Base: {}", base_name);
                }

                assert_eq!(cls.bases.len(), 3, "Should have 3 base classes");

                // Check base class names
                let base_names: Vec<String> = cls.bases.iter()
                    .map(|b| interner.read().resolve(b.type_path.segments[0]).to_string())
                    .collect();

                eprintln!("\nBase names collected:");
                for (i, name) in base_names.iter().enumerate() {
                    eprintln!("  [{}]: '{}'", i, name);
                }

                assert!(base_names.contains(&"AActor".to_string()), "Missing AActor");
                assert!(base_names.contains(&"IMissionTask".to_string()), "Missing IMissionTask");
                assert!(base_names.contains(&"IDebugSettingsInterface".to_string()), "Missing IDebugSettingsInterface");
            }
            _ => panic!("Expected class declaration"),
        }
    }

    #[test]
    fn test_full_task_sample_header() {
        let interner = Arc::new(RwLock::new(Interner::new()));
        let mut parser = CppParser::new(interner.clone()).unwrap();

        // Full Task_Sample.h with multiple UCLASS definitions
        let source = r#"
#pragma once
#include "CoreMinimal.h"
#include "Task.h"
#include "Task_Sample.generated.h"

UCLASS()
class OWF_MISSIONSYSTEM_API ATask_Sample : public ATask
{
    GENERATED_BODY()

public:
    ATask_Sample();

    UPROPERTY(BlueprintReadWrite)
    TObjectPtr<class USampleTaskConfig> TaskConfig;

    UPROPERTY(BlueprintReadOnly)
    TObjectPtr<class USampleTaskDebugSettings> DebugSettings;

    UPROPERTY()
    FName SomeName;

    UPROPERTY(BlueprintReadWrite)
    FName SomeOtherName;

    UPROPERTY(BlueprintReadWrite)
    int SomeIntProperty = 42;

    virtual void Initialise_Implementation(UMissionTaskConfig* Config) override;
    virtual void Execute_Implementation() override;

    virtual void ConfigureDebugging_Implementation(UDebugSettings* DebugSettings) override;
    virtual UDebugSettings* GetDebugSettings_Implementation() override;
};

UCLASS(BlueprintType, EditInlineNew)
class OWF_MISSIONSYSTEM_API USampleTaskDebugSettings : public UDebugSettings
{
    GENERATED_BODY()
public:
    UPROPERTY(EditAnywhere, BlueprintReadWrite)
    bool bPrintLogs = true;
};

UCLASS(BlueprintType, EditInlineNew)
class OWF_MISSIONSYSTEM_API USampleTaskConfig : public UMissionTaskConfig
{
    GENERATED_BODY()
public:
    USampleTaskConfig()
    {
        TaskName = FText::FromString("Sample Task");
    }

    UPROPERTY(EditAnywhere, BlueprintReadWrite)
    int SomeProperty = 42;
};
"#;
        let file_id = FileId::new(1);

        // Parse the file
        let result = parser.parse(source, file_id);
        assert!(result.is_ok());
        let ast = result.unwrap();

        eprintln!("\n=== AST ANALYSIS ===");
        eprintln!("Total declarations: {}", ast.declarations.len());

        let mut class_count = 0;
        for decl in &ast.declarations {
            match decl {
                crate::ast::Declaration::Class(_) |
                crate::ast::Declaration::Struct(_) |
                crate::ast::Declaration::UClass(_) |
                crate::ast::Declaration::UStruct(_) => {
                    class_count += 1;
                }
                _ => {}
            }
        }
        eprintln!("Class declarations found: {}", class_count);

        for (i, decl) in ast.declarations.iter().enumerate() {
            match decl {
                crate::ast::Declaration::Class(cls) => {
                    let name = interner.read().resolve(cls.name);
                    eprintln!("\n[{}] Class: {} with {} members and {} bases",
                        i, name, cls.members.len(), cls.bases.len());

                    // Show base classes
                    for base in &cls.bases {
                        let base_name = base.type_path.segments.iter()
                            .map(|&s| interner.read().resolve(s))
                            .collect::<Vec<_>>()
                            .join("::");
                        eprintln!("  Base: {} (access: {:?})", base_name, base.access);
                    }

                    // Show members
                    for (j, member) in cls.members.iter().enumerate() {
                        match member {
                            crate::ast::ClassMember::Field(field) => {
                                let fname = interner.read().resolve(field.name);
                                eprintln!("  [{}] Field: {}", j, fname);
                            }
                            crate::ast::ClassMember::Method(method) => {
                                let mname = interner.read().resolve(method.name);
                                eprintln!("  [{}] Method: {}", j, mname);
                            }
                            crate::ast::ClassMember::Constructor(_) => {
                                eprintln!("  [{}] Constructor", j);
                            }
                            _ => {
                                eprintln!("  [{}] Other member", j);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Build symbol table
        use crate::index::{AstSymbolBuilder, SymbolTable};
        let symbol_table = Arc::new(RwLock::new(SymbolTable::new()));
        let mut builder = AstSymbolBuilder::new(symbol_table.clone(), interner.clone());
        builder.build_from_ast(&ast);

        // Get all symbols
        let table = symbol_table.read();
        let symbols = table.symbols_in_file(file_id);

        eprintln!("\n=== SYMBOL TABLE ===");
        eprintln!("Total symbols: {}", symbols.len());

        for &sym_id in &symbols {
            if let Some(symbol) = table.get_symbol(sym_id) {
                let name = interner.read().resolve(symbol.name);
                eprintln!("Symbol #{}: {} (kind: {:?}) span: {}..{} (size: {})",
                    sym_id.0, name, symbol.kind, symbol.span.start, symbol.span.end, symbol.span.len());
            }
        }

        // Find the offset of "TaskConfig" identifier (around position of 'T' in TaskConfig)
        let taskconfig_pos = source.find("TaskConfig").expect("Should find TaskConfig");
        eprintln!("\n=== Finding symbol at offset {} (TaskConfig identifier) ===", taskconfig_pos);

        // Find all symbols that contain this offset
        let mut candidates = Vec::new();
        for &sym_id in &symbols {
            if let Some(symbol) = table.get_symbol(sym_id) {
                if symbol.span.contains(taskconfig_pos as u32) {
                    let name = interner.read().resolve(symbol.name);
                    candidates.push((sym_id, name.to_string(), symbol.kind, symbol.span.len()));
                }
            }
        }

        eprintln!("Candidates containing offset {}:", taskconfig_pos);
        for (id, name, kind, size) in &candidates {
            eprintln!("  #{}: {} ({:?}) size={}", id.0, name, kind, size);
        }

        // The smallest span should be the field, not the class
        if let Some((best_id, best_name, best_kind, best_size)) = candidates.iter().min_by_key(|(_, _, _, size)| size) {
            eprintln!("\nBest match (smallest span): #{} {} ({:?}) size={}", best_id.0, best_name, best_kind, best_size);
            assert_eq!(best_name, "TaskConfig", "Should find field symbol, not class!");
        } else {
            panic!("No symbols found at TaskConfig position!");
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

        // Verify class members are parsed
        eprintln!("\nClass members ({}):", main_class.members.len());
        for (i, member) in main_class.members.iter().enumerate() {
            match member {
                crate::ast::ClassMember::Method(m) => {
                    let method_name = interner.read().resolve(m.name);
                    eprintln!("  [{}] Method: {}", i, method_name);
                }
                crate::ast::ClassMember::Field(f) => {
                    let field_name = interner.read().resolve(f.name);
                    eprintln!("  [{}] Field: {}", i, field_name);
                }
                _ => eprintln!("  [{}] Other", i),
            }
        }

        // Should have at least Initialize and Initialise methods
        let has_initialize = main_class.members.iter().any(|m| {
            if let crate::ast::ClassMember::Method(method) = m {
                interner.read().resolve(method.name) == "Initialize"
            } else {
                false
            }
        });

        let has_initialise = main_class.members.iter().any(|m| {
            if let crate::ast::ClassMember::Method(method) = m {
                interner.read().resolve(method.name) == "Initialise"
            } else {
                false
            }
        });

        assert!(has_initialize, "Should have parsed Initialize method");
        assert!(has_initialise, "Should have parsed Initialise method");
    }

    #[test]
    fn test_parse_ue5_class_with_fields() {
        let interner = Arc::new(RwLock::new(Interner::new()));
        let mut parser = CppParser::new(interner.clone()).unwrap();

        // Test class with fields and methods
        let source = r#"
class MY_API MyClass
{
public:
    int MyVariable;
    FString MyString;

    void MyMethod(int Param1, FString Param2);
};
"#;
        let file_id = FileId::new(1);

        let result = parser.parse(source, file_id);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let unit = result.unwrap();
        assert_eq!(unit.declarations.len(), 1);

        match &unit.declarations[0] {
            Declaration::Class(class) => {
                let class_name = interner.read().resolve(class.name);
                assert_eq!(class_name, "MyClass");

                eprintln!("\nClass '{}' has {} members:", class_name, class.members.len());
                for (i, member) in class.members.iter().enumerate() {
                    match member {
                        crate::ast::ClassMember::Method(m) => {
                            let method_name = interner.read().resolve(m.name);
                            eprintln!("  [{}] Method: {} with {} params", i, method_name, m.parameters.len());
                            for (j, param) in m.parameters.iter().enumerate() {
                                if let Some(param_name) = param.name {
                                    let pname = interner.read().resolve(param_name);
                                    eprintln!("      Param[{}]: {}", j, pname);
                                } else {
                                    eprintln!("      Param[{}]: (unnamed)", j);
                                }
                            }
                        }
                        crate::ast::ClassMember::Field(f) => {
                            let field_name = interner.read().resolve(f.name);
                            eprintln!("  [{}] Field: {}", i, field_name);
                        }
                        _ => eprintln!("  [{}] Other", i),
                    }
                }

                // Should have 2 fields
                let field_count = class.members.iter().filter(|m| matches!(m, crate::ast::ClassMember::Field(_))).count();
                assert_eq!(field_count, 2, "Expected 2 fields, got {}", field_count);

                // Should have 1 method
                let method_count = class.members.iter().filter(|m| matches!(m, crate::ast::ClassMember::Method(_))).count();
                assert_eq!(method_count, 1, "Expected 1 method, got {}", method_count);

                // Check MyMethod has parameters with names
                let my_method = class.members.iter().find_map(|m| {
                    if let crate::ast::ClassMember::Method(method) = m {
                        if interner.read().resolve(method.name) == "MyMethod" {
                            Some(method)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });

                assert!(my_method.is_some(), "Should have MyMethod");
                let my_method = my_method.unwrap();
                assert_eq!(my_method.parameters.len(), 2, "MyMethod should have 2 parameters");

                // Check parameter names
                let param1_name = my_method.parameters[0].name
                    .map(|n| interner.read().resolve(n).to_string());
                let param2_name = my_method.parameters[1].name
                    .map(|n| interner.read().resolve(n).to_string());

                assert_eq!(param1_name, Some("Param1".to_string()), "First param should be named 'Param1'");
                assert_eq!(param2_name, Some("Param2".to_string()), "Second param should be named 'Param2'");
            }
            _ => panic!("Expected class declaration"),
        }
    }
}
