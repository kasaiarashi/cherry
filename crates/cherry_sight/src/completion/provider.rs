// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Context-aware autocompletion provider

use crate::index::symbol::{Symbol, SymbolId, SymbolKind};
use crate::index::symbol_table::SymbolTable;
use crate::util::{FileId, Interner, Position};

/// Completion item
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
    pub sort_text: Option<String>,
    pub filter_text: Option<String>,
}

/// Kind of completion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Class,
    Struct,
    Enum,
    Function,
    Method,
    Variable,
    Field,
    Property,
    Keyword,
    Snippet,
    Module,
    Namespace,
}

/// Completion context
#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub trigger_character: Option<char>,
    pub is_member_access: bool,
    pub is_scope_resolution: bool,
    pub prefix: String,
}

/// Provides context-aware completions
pub struct CompletionProvider<'a> {
    symbol_table: &'a SymbolTable,
    interner: &'a Interner,
}

impl<'a> CompletionProvider<'a> {
    pub fn new(symbol_table: &'a SymbolTable, interner: &'a Interner) -> Self {
        Self {
            symbol_table,
            interner,
        }
    }

    /// Get completions at a position
    pub fn get_completions(
        &self,
        _file_id: FileId,
        _position: Position,
        context: &CompletionContext,
    ) -> Vec<CompletionItem> {
        let mut completions = Vec::new();

        if context.is_member_access {
            // Member access (. or ->)
            // Would need type information to resolve the left-hand side
            // For now, return placeholder
        } else if context.is_scope_resolution {
            // Scope resolution (::)
            // Would need namespace/class resolution
        } else {
            // General completions - keywords, visible symbols
            self.add_keyword_completions(&mut completions);
            self.add_symbol_completions(&mut completions, &context.prefix);
        }

        completions
    }

    /// Add C++ keyword completions
    fn add_keyword_completions(&self, completions: &mut Vec<CompletionItem>) {
        let keywords = [
            "auto", "break", "case", "char", "const", "continue", "default",
            "do", "double", "else", "enum", "extern", "float", "for",
            "goto", "if", "inline", "int", "long", "register", "return",
            "short", "signed", "sizeof", "static", "struct", "switch",
            "typedef", "union", "unsigned", "void", "volatile", "while",
            "class", "namespace", "template", "typename", "public", "private",
            "protected", "virtual", "override", "final",
        ];

        for &keyword in &keywords {
            completions.push(CompletionItem {
                label: keyword.to_string(),
                kind: CompletionKind::Keyword,
                detail: None,
                documentation: None,
                insert_text: None,
                sort_text: Some(format!("z{}", keyword)), // Sort keywords last
                filter_text: None,
            });
        }
    }

    /// Add symbol completions matching prefix
    fn add_symbol_completions(&self, completions: &mut Vec<CompletionItem>, prefix: &str) {
        // Iterate through global symbols
        for &symbol_id in self.symbol_table.global_symbols() {
            if let Some(symbol) = self.symbol_table.get_symbol(symbol_id) {
                let name = self.interner.resolve(symbol.name);
                if name.starts_with(prefix) {
                    completions.push(symbol_to_completion(symbol, &name));
                }

                // Add children
                self.add_children_completions(symbol_id, prefix, completions);
            }
        }
    }

    /// Recursively add children matching prefix
    fn add_children_completions(
        &self,
        parent_id: SymbolId,
        prefix: &str,
        completions: &mut Vec<CompletionItem>,
    ) {
        for &child_id in &self.symbol_table.children(parent_id) {
            if let Some(child) = self.symbol_table.get_symbol(child_id) {
                let name = self.interner.resolve(child.name);
                if name.starts_with(prefix) {
                    completions.push(symbol_to_completion(child, &name));
                }
                self.add_children_completions(child_id, prefix, completions);
            }
        }
    }
}

/// Convert symbol to completion item
fn symbol_to_completion(symbol: &Symbol, name: &str) -> CompletionItem {
    let kind = match symbol.kind {
        SymbolKind::Class | SymbolKind::UClass => CompletionKind::Class,
        SymbolKind::Struct | SymbolKind::UStruct => CompletionKind::Struct,
        SymbolKind::Enum | SymbolKind::UEnum => CompletionKind::Enum,
        SymbolKind::Function | SymbolKind::UFunction => CompletionKind::Function,
        SymbolKind::Method => CompletionKind::Method,
        SymbolKind::Variable => CompletionKind::Variable,
        SymbolKind::Field | SymbolKind::UProperty => CompletionKind::Field,
        SymbolKind::Namespace => CompletionKind::Namespace,
        _ => CompletionKind::Variable,
    };

    CompletionItem {
        label: name.to_string(),
        kind,
        detail: None,
        documentation: symbol.doc_comment.clone(),
        insert_text: None,
        sort_text: Some(format!("a{}", name)), // Sort symbols first
        filter_text: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Span;

    #[test]
    fn test_keyword_completions() {
        let table = SymbolTable::new();
        let interner = Interner::new();
        let provider = CompletionProvider::new(&table, &interner);

        let context = CompletionContext {
            trigger_character: None,
            is_member_access: false,
            is_scope_resolution: false,
            prefix: String::new(),
        };

        let completions = provider.get_completions(FileId::new(1), Position { line: 0, column: 0 }, &context);

        // Should include keywords
        assert!(completions.iter().any(|c| c.label == "class"));
        assert!(completions.iter().any(|c| c.label == "struct"));
    }

    #[test]
    fn test_symbol_completions() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let name = interner.intern("MyClass");
        let id = table.next_id();
        let symbol = Symbol::new(id, SymbolKind::Class, name, Span::new(file_id, 0, 10), file_id);
        table.add_symbol(symbol);

        let provider = CompletionProvider::new(&table, &interner);

        let context = CompletionContext {
            trigger_character: None,
            is_member_access: false,
            is_scope_resolution: false,
            prefix: "My".to_string(),
        };

        let completions = provider.get_completions(file_id, Position { line: 0, column: 0 }, &context);

        assert!(completions.iter().any(|c| c.label == "MyClass"));
    }
}
