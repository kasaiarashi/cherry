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

    /// Add symbol completions matching prefix with smart ranking
    fn add_symbol_completions(&self, completions: &mut Vec<CompletionItem>, prefix: &str) {
        let prefix_lower = prefix.to_lowercase();
        let mut ranked_completions: Vec<(i32, CompletionItem)> = Vec::new();

        // Iterate through ALL symbols (cross-file)
        for symbol_id in self.symbol_table.all_symbols() {
            if let Some(symbol) = self.symbol_table.get_symbol(symbol_id) {
                let name = self.interner.resolve(symbol.name);

                // Calculate match score
                if let Some(score) = self.calculate_match_score(&name, prefix, &prefix_lower) {
                    let mut item = symbol_to_completion(symbol, &name);

                    // Boost UE5 types
                    let ue_boost = if matches!(symbol.kind,
                        SymbolKind::UClass | SymbolKind::UStruct | SymbolKind::UEnum | SymbolKind::UFunction | SymbolKind::UProperty
                    ) { 100 } else { 0 };

                    // Boost types over variables
                    let type_boost = if symbol.kind.is_type() { 50 } else { 0 };

                    let final_score = score + ue_boost + type_boost;

                    // Update sort text based on score (lower = better)
                    item.sort_text = Some(format!("{:05}{}", 100000 - final_score, name));

                    ranked_completions.push((final_score, item));
                }
            }
        }

        // Sort by score (descending) and take top results
        ranked_completions.sort_by(|a, b| b.0.cmp(&a.0));

        // Limit to top 100 completions for performance
        for (_score, item) in ranked_completions.into_iter().take(100) {
            completions.push(item);
        }
    }

    /// Calculate match score for fuzzy matching
    /// Returns None if no match, higher score = better match
    fn calculate_match_score(&self, name: &str, prefix: &str, prefix_lower: &str) -> Option<i32> {
        if prefix.is_empty() {
            return Some(0);
        }

        let name_lower = name.to_lowercase();

        // Exact match - highest score
        if name == prefix {
            return Some(1000);
        }

        // Case-sensitive prefix match - very high score
        if name.starts_with(prefix) {
            return Some(500 + (100 - name.len() as i32));
        }

        // Case-insensitive prefix match - high score
        if name_lower.starts_with(prefix_lower) {
            return Some(300 + (100 - name.len() as i32));
        }

        // Contains match - medium score
        if name_lower.contains(prefix_lower) {
            return Some(100);
        }

        // Fuzzy match (all prefix chars in order) - low score
        if self.fuzzy_match(&name_lower, prefix_lower) {
            return Some(50);
        }

        None
    }

    /// Check if all characters in pattern appear in str in order
    fn fuzzy_match(&self, text: &str, pattern: &str) -> bool {
        let mut text_chars = text.chars();
        for pattern_char in pattern.chars() {
            loop {
                match text_chars.next() {
                    Some(text_char) if text_char == pattern_char => break,
                    Some(_) => continue,
                    None => return false,
                }
            }
        }
        true
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
