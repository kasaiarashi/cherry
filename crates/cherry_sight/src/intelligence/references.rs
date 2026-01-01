// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Find all references to a symbol

use crate::index::symbol::{Symbol, SymbolId};
use crate::index::symbol_table::SymbolTable;
use crate::util::{FileId, Span};
use std::collections::HashSet;

/// Result of a reference search
#[derive(Debug, Clone)]
pub struct ReferenceResult {
    pub symbol_id: SymbolId,
    pub span: Span,
    pub file_id: FileId,
    pub kind: ReferenceKind,
}

/// Kind of reference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKind {
    /// Declaration/definition of the symbol
    Definition,
    /// Read reference
    Read,
    /// Write reference
    Write,
    /// Call reference (for functions)
    Call,
}

/// Finds all references to a symbol
pub struct ReferenceFinder<'a> {
    symbol_table: &'a SymbolTable,
}

impl<'a> ReferenceFinder<'a> {
    pub fn new(symbol_table: &'a SymbolTable) -> Self {
        Self { symbol_table }
    }

    /// Find all references to a symbol
    pub fn find_references(&self, symbol_id: SymbolId, include_declaration: bool) -> Vec<ReferenceResult> {
        let mut results = Vec::new();

        let symbol = match self.symbol_table.get_symbol(symbol_id) {
            Some(s) => s,
            None => return results,
        };

        // Add the definition itself if requested
        if include_declaration {
            results.push(ReferenceResult {
                symbol_id,
                span: symbol.span,
                file_id: symbol.file_id,
                kind: ReferenceKind::Definition,
            });
        }

        // For now, we would need additional tracking in the AST to find actual references
        // This is a simplified implementation that shows the structure
        // In a real implementation, we would:
        // 1. Parse all files and track identifier usage
        // 2. Match identifiers to symbols through name resolution
        // 3. Track read/write context

        // Find references by searching for symbols with the same name
        // This is a placeholder - real implementation needs AST analysis
        self.find_references_by_name(symbol, &mut results);

        results
    }

    /// Find references to all overrides of a virtual method
    pub fn find_override_references(&self, method_id: SymbolId) -> Vec<ReferenceResult> {
        let mut results = Vec::new();

        // Find all methods in the override chain
        let mut methods_to_search = HashSet::new();
        methods_to_search.insert(method_id);

        // Add all overrides
        let overrides = self.symbol_table.find_overrides(method_id);
        for &override_id in &overrides {
            methods_to_search.insert(override_id);
        }

        // Add the base method
        if let Some(base_id) = self.symbol_table.find_overridden(method_id) {
            methods_to_search.insert(base_id);
        }

        // Find references to each method
        for &method_id in &methods_to_search {
            let method_refs = self.find_references(method_id, true);
            results.extend(method_refs);
        }

        results
    }

    /// Find references by searching for symbols with matching names (simplified)
    fn find_references_by_name(&self, _symbol: &Symbol, _results: &mut Vec<ReferenceResult>) {
        // In a real implementation, this would:
        // 1. Get all files that might reference this symbol
        // 2. Parse each file and find identifier nodes
        // 3. Resolve each identifier to see if it refers to our symbol
        // 4. Classify each reference as Read/Write/Call based on AST context

        // For now, this is a placeholder showing the structure
        // We would need to store reference information during parsing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::symbol::SymbolKind;
    use crate::util::{Interner, Span};

    #[test]
    fn test_find_references_basic() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let name = interner.intern("myVar");

        let id = table.next_id();
        let symbol = Symbol::new(id, SymbolKind::Variable, name, Span::new(file_id, 10, 15), file_id);
        table.add_symbol(symbol);

        let finder = ReferenceFinder::new(&table);
        let results = finder.find_references(id, true);

        // Should at least include the definition
        assert!(results.len() >= 1);
        assert_eq!(results[0].kind, ReferenceKind::Definition);
    }

    #[test]
    fn test_find_override_references() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let file_id = FileId::new(1);
        let method_name = interner.intern("Update");

        // Create base method
        let base_id = table.next_id();
        let mut base_method = Symbol::new(
            base_id,
            SymbolKind::Method,
            method_name,
            Span::new(file_id, 10, 20),
            file_id,
        );

        // Create override method
        let override_id = table.next_id();
        let mut override_method = Symbol::new(
            override_id,
            SymbolKind::Method,
            method_name,
            Span::new(file_id, 50, 60),
            file_id,
        );
        override_method.overrides = Some(base_id);
        base_method.add_overridden_by(override_id);

        table.add_symbol(base_method);
        table.add_symbol(override_method);

        let finder = ReferenceFinder::new(&table);
        let results = finder.find_override_references(base_id);

        // Should find both base and override
        assert!(results.len() >= 2);
    }
}
