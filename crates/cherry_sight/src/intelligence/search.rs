// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Symbol search functionality

use crate::index::symbol::{Symbol, SymbolId, SymbolKind};
use crate::index::symbol_table::SymbolTable;
use crate::util::{FileId, InternedString, Interner, Span};

/// Result of a symbol search
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub symbol_id: SymbolId,
    pub name: InternedString,
    pub kind: SymbolKind,
    pub span: Span,
    pub file_id: FileId,
    pub score: f32, // Relevance score
}

/// Search filter
#[derive(Debug, Clone, Default)]
pub struct SearchFilter {
    pub kinds: Option<Vec<SymbolKind>>,
    pub file_id: Option<FileId>,
    pub max_results: Option<usize>,
}

/// Symbol searcher
pub struct SymbolSearcher<'a> {
    symbol_table: &'a SymbolTable,
    interner: &'a Interner,
}

impl<'a> SymbolSearcher<'a> {
    pub fn new(symbol_table: &'a SymbolTable, interner: &'a Interner) -> Self {
        Self {
            symbol_table,
            interner,
        }
    }

    /// Search for symbols by name (exact match)
    pub fn search_exact(&self, query: &str, filter: &SearchFilter) -> Vec<SearchResult> {
        let query_str = self.interner.intern(query);
        let symbols = self.symbol_table.find_all(query_str);

        self.filter_and_score(symbols, query, filter, true)
    }

    /// Search for symbols by name (fuzzy match)
    pub fn search_fuzzy(&self, query: &str, filter: &SearchFilter) -> Vec<SearchResult> {
        let mut all_symbols = Vec::new();

        // Iterate through all symbols and find matches
        // In a real implementation, we'd use a more efficient fuzzy matching algorithm
        for (symbol_id, symbol) in self.iter_symbols() {
            let symbol_name = self.interner.resolve(symbol.name);
            if self.fuzzy_match(&symbol_name, query) {
                all_symbols.push(symbol_id);
            }
        }

        self.filter_and_score(all_symbols, query, filter, false)
    }

    /// Search for symbols by prefix
    pub fn search_prefix(&self, prefix: &str, filter: &SearchFilter) -> Vec<SearchResult> {
        let mut matching_symbols = Vec::new();

        for (symbol_id, symbol) in self.iter_symbols() {
            let symbol_name = self.interner.resolve(symbol.name);
            if symbol_name.starts_with(prefix) {
                matching_symbols.push(symbol_id);
            }
        }

        self.filter_and_score(matching_symbols, prefix, filter, false)
    }

    /// Search within a specific scope
    pub fn search_in_scope(
        &self,
        query: &str,
        scope_id: SymbolId,
        filter: &SearchFilter,
    ) -> Vec<SearchResult> {
        let query_str = self.interner.intern(query);
        let symbols = self.symbol_table.find_in_scope(query_str, Some(scope_id));

        self.filter_and_score(symbols, query, filter, true)
    }

    /// Find all symbols of a specific kind
    pub fn find_by_kind(&self, kind: SymbolKind, filter: &SearchFilter) -> Vec<SearchResult> {
        let mut results = Vec::new();

        for (symbol_id, symbol) in self.iter_symbols() {
            if symbol.kind == kind {
                results.push(SearchResult {
                    symbol_id,
                    name: symbol.name,
                    kind: symbol.kind,
                    span: symbol.span,
                    file_id: symbol.file_id,
                    score: 1.0,
                });
            }
        }

        self.apply_filter(results, filter)
    }

    /// Filter and score search results
    fn filter_and_score(
        &self,
        symbols: Vec<SymbolId>,
        query: &str,
        filter: &SearchFilter,
        exact_match: bool,
    ) -> Vec<SearchResult> {
        let mut results = Vec::new();

        for symbol_id in symbols {
            if let Some(symbol) = self.symbol_table.get_symbol(symbol_id) {
                let symbol_name = self.interner.resolve(symbol.name);
                let score = if exact_match {
                    1.0
                } else {
                    self.calculate_score(&symbol_name, query)
                };

                results.push(SearchResult {
                    symbol_id,
                    name: symbol.name,
                    kind: symbol.kind,
                    span: symbol.span,
                    file_id: symbol.file_id,
                    score,
                });
            }
        }

        results = self.apply_filter(results, filter);

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        results
    }

    /// Apply search filter
    fn apply_filter(&self, mut results: Vec<SearchResult>, filter: &SearchFilter) -> Vec<SearchResult> {
        // Filter by kind
        if let Some(ref kinds) = filter.kinds {
            results.retain(|r| kinds.contains(&r.kind));
        }

        // Filter by file
        if let Some(file_id) = filter.file_id {
            results.retain(|r| r.file_id == file_id);
        }

        // Limit results
        if let Some(max) = filter.max_results {
            results.truncate(max);
        }

        results
    }

    /// Simple fuzzy matching
    fn fuzzy_match(&self, text: &str, pattern: &str) -> bool {
        let text_lower = text.to_lowercase();
        let pattern_lower = pattern.to_lowercase();

        let mut pattern_chars = pattern_lower.chars();
        let mut current_pattern_char = pattern_chars.next();

        for text_char in text_lower.chars() {
            if let Some(p_char) = current_pattern_char {
                if text_char == p_char {
                    current_pattern_char = pattern_chars.next();
                }
            }
        }

        current_pattern_char.is_none()
    }

    /// Calculate relevance score
    fn calculate_score(&self, symbol_name: &str, query: &str) -> f32 {
        let lower_symbol = symbol_name.to_lowercase();
        let lower_query = query.to_lowercase();

        // Exact match
        if lower_symbol == lower_query {
            return 1.0;
        }

        // Starts with query
        if lower_symbol.starts_with(&lower_query) {
            return 0.9;
        }

        // Contains query
        if lower_symbol.contains(&lower_query) {
            return 0.7;
        }

        // Fuzzy match
        if self.fuzzy_match(symbol_name, query) {
            return 0.5;
        }

        0.0
    }

    /// Iterator over all symbols
    fn iter_symbols(&self) -> Vec<(SymbolId, &Symbol)> {
        let mut symbols = Vec::new();
        // In a real implementation, we'd have a proper iterator
        // For now, iterate through global symbols and their children
        for &global_id in self.symbol_table.global_symbols() {
            if let Some(symbol) = self.symbol_table.get_symbol(global_id) {
                symbols.push((global_id, symbol));
                self.collect_children(global_id, &mut symbols);
            }
        }
        symbols
    }

    /// Recursively collect all children
    fn collect_children(&self, parent_id: SymbolId, symbols: &mut Vec<(SymbolId, &'a Symbol)>) {
        for &child_id in &self.symbol_table.children(parent_id) {
            if let Some(symbol) = self.symbol_table.get_symbol(child_id) {
                symbols.push((child_id, symbol));
                self.collect_children(child_id, symbols);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::symbol::Symbol;

    #[test]
    fn test_search_exact() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let name = interner.intern("MyClass");

        let id = table.next_id();
        let symbol = Symbol::new(id, SymbolKind::Class, name, Span::new(file_id, 0, 10), file_id);
        table.add_symbol(symbol);

        let searcher = SymbolSearcher::new(&table, &interner);
        let results = searcher.search_exact("MyClass", &SearchFilter::default());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].symbol_id, id);
        assert_eq!(results[0].score, 1.0);
    }

    #[test]
    fn test_search_by_kind() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);

        // Add a class
        let class_name = interner.intern("MyClass");
        let class_id = table.next_id();
        let class = Symbol::new(class_id, SymbolKind::Class, class_name, Span::new(file_id, 0, 10), file_id);
        table.add_symbol(class);

        // Add a function
        let func_name = interner.intern("MyFunc");
        let func_id = table.next_id();
        let func = Symbol::new(func_id, SymbolKind::Function, func_name, Span::new(file_id, 20, 30), file_id);
        table.add_symbol(func);

        let searcher = SymbolSearcher::new(&table, &interner);
        let results = searcher.find_by_kind(SymbolKind::Class, &SearchFilter::default());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].symbol_id, class_id);
    }

    #[test]
    fn test_fuzzy_match() {
        let table = SymbolTable::new();
        let interner = Interner::new();
        let searcher = SymbolSearcher::new(&table, &interner);

        assert!(searcher.fuzzy_match("MyClassName", "MCN"));
        assert!(searcher.fuzzy_match("GameObject", "GO"));
        assert!(!searcher.fuzzy_match("Player", "XYZ"));
    }
}
