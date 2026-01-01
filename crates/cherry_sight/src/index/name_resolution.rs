// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! Name resolution for qualified and unqualified names

use crate::index::symbol::SymbolId;
use crate::index::symbol_table::SymbolTable;
use crate::index::scope::{Scope, ScopeStack};
use crate::util::{Interner, InternedString};

/// Name resolution context
pub struct NameResolver<'a> {
    pub symbol_table: &'a SymbolTable,
    pub interner: &'a Interner,
    scope_stack: ScopeStack,
}

impl<'a> NameResolver<'a> {
    pub fn new(symbol_table: &'a SymbolTable, interner: &'a Interner) -> Self {
        Self {
            symbol_table,
            interner,
            scope_stack: ScopeStack::new(),
        }
    }

    /// Resolve an unqualified name in the current scope
    pub fn resolve_unqualified(&self, name: InternedString) -> Vec<SymbolId> {
        // First check the scope stack
        let mut results = self.scope_stack.lookup(name);

        // If not found, check global scope in symbol table
        if results.is_empty() {
            results = self.symbol_table.find_in_scope(name, None);
        }

        results
    }

    /// Resolve a qualified name (e.g., "MyNamespace::MyClass")
    pub fn resolve_qualified(&self, path: &[InternedString]) -> Vec<SymbolId> {
        if path.is_empty() {
            return Vec::new();
        }

        // Start with global scope
        let mut current_scope = None;
        let mut candidates = Vec::new();

        // Resolve each component of the path
        for (i, component) in path.iter().enumerate() {
            let symbols = self.symbol_table.find_in_scope(*component, current_scope);

            if symbols.is_empty() {
                return Vec::new();
            }

            // If this is the last component, return all matches
            if i == path.len() - 1 {
                return symbols;
            }

            // Otherwise, use the first match as the new scope
            // (in real implementation, would handle ambiguity)
            current_scope = Some(symbols[0]);
            candidates = symbols;
        }

        candidates
    }

    /// Push a new scope
    pub fn push_scope(&mut self, scope: Scope) {
        self.scope_stack.push(scope);
    }

    /// Pop the current scope
    pub fn pop_scope(&mut self) -> Option<Scope> {
        self.scope_stack.pop()
    }

    /// Get current scope
    pub fn current_scope(&self) -> &Scope {
        self.scope_stack.current()
    }

    /// Get current scope mutably
    pub fn current_scope_mut(&mut self) -> &mut Scope {
        self.scope_stack.current_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::symbol::{Symbol, SymbolKind};
    use crate::index::symbol_table::SymbolTable;
    use crate::util::{FileId, Span, Interner};

    #[test]
    fn test_unqualified_resolution() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let name = interner.intern("MyClass");
        let file_id = FileId::new(1);
        let span = Span::new(file_id, 0, 10);

        let id = table.next_id();
        let symbol = Symbol::new(id, SymbolKind::Class, name, span, file_id);
        table.add_symbol(symbol);

        let resolver = NameResolver::new(&table, &interner);
        let results = resolver.resolve_unqualified(name);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], id);
    }

    #[test]
    fn test_qualified_resolution() {
        let mut table = SymbolTable::new();
        let mut interner = Interner::new();

        let ns_name = interner.intern("MyNamespace");
        let class_name = interner.intern("MyClass");
        let file_id = FileId::new(1);
        let span = Span::new(file_id, 0, 10);

        // Create namespace
        let ns_id = table.next_id();
        let mut ns = Symbol::new(ns_id, SymbolKind::Namespace, ns_name, span, file_id);

        // Create class in namespace
        let class_id = table.next_id();
        let mut class = Symbol::new(class_id, SymbolKind::Class, class_name, span, file_id);
        class.parent = Some(ns_id);
        ns.add_child(class_id);

        table.add_symbol(ns);
        table.add_symbol(class);

        let resolver = NameResolver::new(&table, &interner);
        let path = vec![ns_name, class_name];
        let results = resolver.resolve_qualified(&path);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], class_id);
    }
}
